use eframe::egui;
use image::{ImageBuffer, Rgba};
use std::path::PathBuf;

// GStreamer imports
use gst::prelude::*;
use gstreamer as gst;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

/// A simple video player widget that decodes frames using ffmpeg-next and displays them in egui.
/// This is a scaffold: actual frame decoding and playback logic should be expanded for real use.
pub struct VideoPlayer {
    pub path: PathBuf,
    pub current_frame: usize,
    pub total_frames: usize,
    pub texture: Option<egui::TextureHandle>,
}

impl VideoPlayer {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            current_frame: 0,
            total_frames: 0,
            texture: None,
        }
    }

    /// Set the frame to display and update the texture if needed.
    pub fn set_frame(&mut self, frame: usize, ctx: &egui::Context) {
        // Clamp frame to reasonable bounds
        let clamped_frame = frame.min(1_000_000); // Max 1M frames (about 9 hours at 30fps)

        if self.current_frame != clamped_frame {
            self.current_frame = clamped_frame;
            self.decode_and_upload_frame(ctx);
        }
    }

    /// Call this to decode and upload the current frame as an egui texture.
    /// Uses GStreamer to extract the frame.
    pub fn decode_and_upload_frame(&mut self, ctx: &egui::Context) {
        let _ = gst::init(); // Safe to call multiple times

        let path_str = self.path.to_string_lossy();

        // Check if file exists before trying to create pipeline
        if !self.path.exists() {
            eprintln!("Video file does not exist: {}", path_str);
            self.texture = None;
            return;
        }

        let pipeline_str = format!(
            "filesrc location=\"{}\" ! decodebin ! videoconvert ! video/x-raw,format=RGBA ! appsink name=sink",
            path_str
        );

        let pipeline = match gst::parse::launch(&pipeline_str) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Failed to create GStreamer pipeline: {}", e);
                self.texture = None;
                return;
            }
        };
        let pipeline = pipeline
            .downcast::<gst::Pipeline>()
            .expect("Expected a gst::Pipeline");

        // Seek to the desired frame (approximate by time)
        // For simplicity, assume 30fps
        let fps = 30.0;
        let seek_time_seconds = self.current_frame as f64 / fps;

        // Clamp seek time to reasonable bounds (0 to 1 hour max)
        let seek_time_seconds = seek_time_seconds.max(0.0).min(3600.0);
        let seek_time_ns = (seek_time_seconds * 1_000_000_000.0) as u64;

        if let Err(e) = pipeline.set_state(gst::State::Paused) {
            eprintln!("Failed to set pipeline to paused: {}", e);
            self.texture = None;
            return;
        }

        if let Err(e) = pipeline.seek_simple(
            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
            gst::ClockTime::from_nseconds(seek_time_ns),
        ) {
            eprintln!("Failed to seek to frame {}: {}", self.current_frame, e);
            pipeline.set_state(gst::State::Null).ok();
            self.texture = None;
            return;
        }

        if let Err(e) = pipeline.set_state(gst::State::Playing) {
            eprintln!("Failed to set pipeline to playing: {}", e);
            pipeline.set_state(gst::State::Null).ok();
            self.texture = None;
            return;
        }

        // Pull the sample from appsink
        let sink = match pipeline.by_name("sink") {
            Some(s) => match s.clone().downcast::<gst_app::AppSink>() {
                Ok(appsink) => appsink,
                Err(e) => {
                    eprintln!("Failed to downcast to AppSink: {:?}", e);
                    self.texture = None;
                    pipeline.set_state(gst::State::Null).ok();
                    return;
                }
            },
            None => {
                eprintln!("Could not find sink element in pipeline");
                self.texture = None;
                pipeline.set_state(gst::State::Null).ok();
                return;
            }
        };

        // Wait a bit for the pipeline to process
        std::thread::sleep(std::time::Duration::from_millis(100));

        let sample_result = sink.pull_sample();
        pipeline.set_state(gst::State::Null).ok();

        match sample_result {
            Ok(sample) => {
                match (sample.buffer(), sample.caps()) {
                    (Some(buffer), Some(caps)) => {
                        match buffer.map_readable() {
                            Ok(map) => {
                                match caps.structure(0) {
                                    Some(s) => {
                                        match (s.get::<i32>("width"), s.get::<i32>("height")) {
                                            (Ok(width), Ok(height)) => {
                                                let width = width as u32;
                                                let height = height as u32;

                                                // Validate dimensions
                                                if width == 0
                                                    || height == 0
                                                    || width > 8192
                                                    || height > 8192
                                                {
                                                    eprintln!(
                                                        "Invalid video dimensions: {}x{}",
                                                        width, height
                                                    );
                                                    self.texture = None;
                                                    return;
                                                }

                                                match ImageBuffer::<Rgba<u8>, _>::from_raw(
                                                    width,
                                                    height,
                                                    map.as_slice().to_vec(),
                                                ) {
                                                    Some(img) => {
                                                        let color_img = egui::ColorImage::from_rgba_unmultiplied(
                                                            [width as usize, height as usize],
                                                            bytemuck::cast_slice(img.as_raw()),
                                                        );
                                                        self.texture = Some(ctx.load_texture(
                                                            "video_frame",
                                                            color_img,
                                                            egui::TextureOptions::default(),
                                                        ));
                                                    }
                                                    None => {
                                                        eprintln!(
                                                            "Failed to create ImageBuffer from video data"
                                                        );
                                                        self.texture = None;
                                                    }
                                                }
                                            }
                                            _ => {
                                                eprintln!(
                                                    "Failed to get width/height from video caps"
                                                );
                                                self.texture = None;
                                            }
                                        }
                                    }
                                    None => {
                                        eprintln!("Failed to get structure from video caps");
                                        self.texture = None;
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to map buffer: {}", e);
                                self.texture = None;
                            }
                        }
                    }
                    _ => {
                        eprintln!("Failed to get buffer or caps from sample");
                        self.texture = None;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to pull sample from sink: {}", e);
                self.texture = None;
            }
        }
    }

    /// Show the video player panel in egui.
    pub fn show(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.vertical(|ui| {
            ui.heading("Video Player");
            ui.label(format!("Frame: {}", self.current_frame));
            // Display the current frame
            if let Some(texture) = &self.texture {
                ui.image(texture);
            } else {
                ui.label("No frame loaded");
            }
        });
    }
}
