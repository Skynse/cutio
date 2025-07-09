use crate::types::timeline::Timeline;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

// GStreamer imports for video decoding
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
// use gstreamer_video as gst_video; // Unused import

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>, // Raw pixel data (e.g., RGBA)
    pub width: u32,
    pub height: u32,
    pub timestamp: f64, // Time in seconds
    pub frame_number: u64,
    // Add more fields as needed (e.g., format, color space)
}

#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub data: Vec<f32>, // Raw audio data (e.g., PCM)
    pub sample_rate: u32,
    pub timestamp: f64, // Time in seconds
    pub frame_number: u64,
    // Add more fields as needed (e.g., format, channel count)
}

pub struct TimelineRenderer {
    pub timeline: Arc<RwLock<Timeline>>,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub frame_cache: HashMap<u64, VideoFrame>, // Frame cache keyed by frame number
                                               // Add more fields as needed (e.g., caches, effect processors)
}

impl TimelineRenderer {
    pub fn new(timeline: Arc<RwLock<Timeline>>, width: u32, height: u32, frame_rate: f64) -> Self {
        Self {
            timeline,
            width,
            height,
            frame_rate,
            frame_cache: HashMap::new(),
        }
    }

    /// Render a video frame at the given time (in seconds), with stub compositing and caching.
    pub fn render_frame(&mut self, time: f64) -> VideoFrame {
        let frame_number = (time * self.frame_rate) as u64;

        // 1. Check cache first
        if let Some(frame) = self.frame_cache.get(&frame_number) {
            return frame.clone();
        }

        // 2. Lock the timeline and find active video clips
        let timeline = self.timeline.read().unwrap();

        // Debug print: show all tracks and their clips
        println!("--- Timeline Debug ---");
        println!("Timeline has {} tracks", timeline.tracks.len());
        for (i, track) in timeline.tracks.iter().enumerate() {
            println!("Track {}: {:?}", i, track);
        }

        // Get all active clips (video and audio) at this time
        let active_clips = timeline.active_clips_at(time);

        // Debug print: show active clips at this time
        println!("Active clips at time {}: {:?}", time, active_clips);

        // 3. Composite the clips (real decoding for first active video clip)
        let mut data = vec![0u8; (self.width * self.height * 4) as usize];

        // Find the first active video clip and decode it
        if let Some(crate::types::timeline::ActiveClip::Video(clip)) = active_clips
            .iter()
            .find(|c| matches!(c, crate::types::timeline::ActiveClip::Video(_)))
        {
            let path = &clip.asset_path;
            let clip_in_point = clip.in_point;
            let clip_start_time = clip.start_time;
            // Calculate the timestamp in the source video
            let local_time = time - clip_start_time + clip_in_point;
            if let Some(frame_data) =
                Self::decode_video_frame(path, local_time, self.width, self.height)
            {
                if frame_data.len() == data.len() {
                    data.copy_from_slice(&frame_data);
                } else {
                    println!(
                        "Decoded frame size mismatch: got {}, expected {}",
                        frame_data.len(),
                        data.len()
                    );
                }
            } else {
                println!("Failed to decode video frame for clip at {}", local_time);
            }
        }

        println!("Compositing {} clips at time {}", active_clips.len(), time);

        let output = VideoFrame {
            data,
            width: self.width,
            height: self.height,
            timestamp: time,
            frame_number,
        };

        // 4. Store in cache
        self.frame_cache.insert(frame_number, output.clone());

        output
    }

    /// Optionally, clear the cache (e.g., when timeline changes)
    pub fn clear_cache(&mut self) {
        self.frame_cache.clear();
    }

    /// Decode a single video frame from a file at a given timestamp using GStreamer.
    /// Returns RGBA pixel data if successful.
    fn decode_video_frame(path: &str, timestamp: f64, width: u32, height: u32) -> Option<Vec<u8>> {
        let _ = gst::init(); // Safe to call multiple times

        // Debug: Check file existence and print seek time
        if !std::path::Path::new(path).exists() {
            println!("Video file does not exist: {}", path);
            return None;
        }

        println!(
            "Decoding frame from {} at timestamp {} (width {}, height {})",
            path, timestamp, width, height
        );

        let pipeline_str = format!(
            "filesrc location=\"{}\" ! decodebin ! videoconvert ! videoscale ! video/x-raw,format=RGBA,width={},height={} ! appsink name=sink sync=false",
            path, width, height
        );

        let pipeline = match gst::parse::launch(&pipeline_str) {
            Ok(pipeline) => pipeline.downcast::<gst::Pipeline>().ok()?,
            Err(e) => {
                println!("Failed to create pipeline: {}", e);
                return None;
            }
        };

        let sink = pipeline
            .by_name("sink")?
            .clone()
            .downcast::<gst_app::AppSink>()
            .ok()?;

        // Configure appsink
        sink.set_property("emit-signals", true);
        sink.set_property("max-buffers", 1u32);
        sink.set_property("drop", true);

        // Set pipeline to PAUSED and wait for state change
        if let Err(e) = pipeline.set_state(gst::State::Paused) {
            println!("Failed to set pipeline to PAUSED: {}", e);
            return None;
        }

        // Wait for pipeline to reach PAUSED state
        let (state_change_result, state, pending) =
            pipeline.state(Some(gst::ClockTime::from_seconds(5)));
        match (state_change_result, state, pending) {
            (Ok(gst::StateChangeSuccess::Success), gst::State::Paused, _) => {
                println!("Pipeline reached PAUSED state");
            }
            (result, state, pending) => {
                println!(
                    "Pipeline failed to reach PAUSED state: {:?}, current state: {:?}, pending: {:?}",
                    result, state, pending
                );
                pipeline.set_state(gst::State::Null).ok();
                return None;
            }
        }

        // Perform seek
        let seek_time_ns = (timestamp * 1_000_000_000.0) as u64;
        println!("Seeking to {} ns ({} seconds)", seek_time_ns, timestamp);

        let seek_result = pipeline.seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::ClockTime::from_nseconds(seek_time_ns),
        );

        if let Err(e) = seek_result {
            println!("Seek failed: {}", e);
            pipeline.set_state(gst::State::Null).ok();
            return None;
        }

        // Set to PLAYING and wait for state change
        if let Err(e) = pipeline.set_state(gst::State::Playing) {
            println!("Failed to set pipeline to PLAYING: {}", e);
            pipeline.set_state(gst::State::Null).ok();
            return None;
        }

        // Wait for pipeline to reach PLAYING state
        let (state_change_result, state, pending) =
            pipeline.state(Some(gst::ClockTime::from_seconds(2)));
        match (state_change_result, state, pending) {
            (Ok(gst::StateChangeSuccess::Success), gst::State::Playing, _) => {
                println!("Pipeline reached PLAYING state");
            }
            (result, state, pending) => {
                println!(
                    "Pipeline failed to reach PLAYING state: {:?}, current state: {:?}, pending: {:?}",
                    result, state, pending
                );
                pipeline.set_state(gst::State::Null).ok();
                return None;
            }
        }

        // Try to pull sample with timeout
        let sample = match Self::pull_sample_with_timeout(&sink, Duration::from_secs(5)) {
            Some(sample) => sample,
            None => {
                println!("Failed to pull sample from appsink");
                pipeline.set_state(gst::State::Null).ok();
                return None;
            }
        };

        // Clean up pipeline
        pipeline.set_state(gst::State::Null).ok();

        // Extract buffer data
        let buffer = sample.buffer()?;
        let map = buffer.map_readable().ok()?;
        let data = map.as_slice().to_vec();

        println!("Successfully decoded frame buffer size: {}", data.len());
        Some(data)
    }

    /// Pull a sample from appsink with a timeout
    fn pull_sample_with_timeout(sink: &gst_app::AppSink, timeout: Duration) -> Option<gst::Sample> {
        let start_time = std::time::Instant::now();

        loop {
            // Try to pull a sample
            if let Ok(sample) = sink.pull_sample() {
                return Some(sample);
            }

            // Check if we've timed out
            if start_time.elapsed() > timeout {
                println!("Timeout waiting for sample");
                return None;
            }

            // Small sleep to avoid busy waiting
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Alternative decode method using bus messages for better error handling
    fn decode_video_frame_with_bus(
        path: &str,
        timestamp: f64,
        width: u32,
        height: u32,
    ) -> Option<Vec<u8>> {
        if !std::path::Path::new(path).exists() {
            println!("Video file does not exist: {}", path);
            return None;
        }

        println!(
            "Decoding frame from {} at timestamp {} using bus method",
            path, timestamp
        );

        let pipeline_str = format!(
            "filesrc location=\"{}\" ! decodebin ! videoconvert ! videoscale ! video/x-raw,format=RGBA,width={},height={} ! appsink name=sink sync=false",
            path, width, height
        );

        let pipeline = gst::parse::launch(&pipeline_str)
            .ok()?
            .downcast::<gst::Pipeline>()
            .ok()?;

        let bus = pipeline.bus()?;
        let sink = pipeline
            .by_name("sink")?
            .downcast::<gst_app::AppSink>()
            .ok()?;

        // Set to PAUSED
        if let Err(e) = pipeline.set_state(gst::State::Paused) {
            println!("Failed to set pipeline to PAUSED: {}", e);
            return None;
        }

        // Wait for ASYNC_DONE message
        let mut preroll_complete = false;
        for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
            match msg.view() {
                gst::MessageView::AsyncDone(_) => {
                    println!("Pipeline preroll complete");
                    preroll_complete = true;
                    break;
                }
                gst::MessageView::Error(err) => {
                    println!("Pipeline error during preroll: {}", err.error());
                    pipeline.set_state(gst::State::Null).ok();
                    return None;
                }
                gst::MessageView::Warning(warn) => {
                    println!("Pipeline warning: {}", warn.error());
                }
                _ => {}
            }
        }

        if !preroll_complete {
            println!("Pipeline preroll timed out");
            pipeline.set_state(gst::State::Null).ok();
            return None;
        }

        // Seek
        let seek_time_ns = (timestamp * 1_000_000_000.0) as u64;
        println!("Seeking to {} ns ({} seconds)", seek_time_ns, timestamp);

        if pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                gst::ClockTime::from_nseconds(seek_time_ns),
            )
            .is_err()
        {
            println!("Seek failed");
            pipeline.set_state(gst::State::Null).ok();
            return None;
        }

        // Set to PLAYING
        if let Err(e) = pipeline.set_state(gst::State::Playing) {
            println!("Failed to set pipeline to PLAYING: {}", e);
            pipeline.set_state(gst::State::Null).ok();
            return None;
        }

        // Wait a bit for the pipeline to process
        std::thread::sleep(Duration::from_millis(100));

        // Pull sample with timeout
        let sample = match Self::pull_sample_with_timeout(&sink, Duration::from_secs(3)) {
            Some(sample) => sample,
            None => {
                println!("Failed to pull sample using bus method");
                pipeline.set_state(gst::State::Null).ok();
                return None;
            }
        };

        pipeline.set_state(gst::State::Null).ok();

        let buffer = sample.buffer()?;
        let map = buffer.map_readable().ok()?;
        let data = map.as_slice().to_vec();

        println!(
            "Successfully decoded frame using bus method, buffer size: {}",
            data.len()
        );
        Some(data)
    }

    /// Utility method to get video file duration
    fn get_video_duration(path: &str) -> Option<f64> {
        let _ = gst::init();

        if !std::path::Path::new(path).exists() {
            return None;
        }

        let pipeline_str = format!("filesrc location=\"{}\" ! decodebin ! fakesink", path);

        let pipeline = gst::parse::launch(&pipeline_str)
            .ok()?
            .downcast::<gst::Pipeline>()
            .ok()?;

        pipeline.set_state(gst::State::Paused).ok()?;

        // Wait for state change
        let (state_change_result, _state, _pending) =
            pipeline.state(Some(gst::ClockTime::from_seconds(5)));
        match state_change_result {
            Ok(gst::StateChangeSuccess::Success) => {}
            _ => return None,
        }

        let duration = pipeline.query_duration::<gst::ClockTime>()?;
        pipeline.set_state(gst::State::Null).ok();

        Some(duration.seconds() as f64)
    }

    /// Validate that the timestamp is within the video duration
    fn validate_timestamp(path: &str, timestamp: f64) -> bool {
        if let Some(duration) = Self::get_video_duration(path) {
            timestamp >= 0.0 && timestamp <= duration
        } else {
            false
        }
    }

    /// Enhanced decode method with validation and fallback
    fn decode_video_frame_enhanced(
        path: &str,
        timestamp: f64,
        width: u32,
        height: u32,
    ) -> Option<Vec<u8>> {
        println!("Enhanced decode attempt for {} at {}", path, timestamp);

        // Validate timestamp
        if !Self::validate_timestamp(path, timestamp) {
            println!("Invalid timestamp {} for video {}", timestamp, path);
            return None;
        }

        // Try primary method first
        if let Some(data) = Self::decode_video_frame(path, timestamp, width, height) {
            return Some(data);
        }

        println!("Primary decode failed, trying bus method");

        // Fallback to bus method
        if let Some(data) = Self::decode_video_frame_with_bus(path, timestamp, width, height) {
            return Some(data);
        }

        println!("All decode methods failed for {} at {}", path, timestamp);
        None
    }

    // Add audio rendering, effect processing, etc. as needed
}
