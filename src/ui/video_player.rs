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
        if self.current_frame != frame {
            self.current_frame = frame;
            self.decode_and_upload_frame(ctx);
        }
    }

    /// Call this to decode and upload the current frame as an egui texture.
    /// Uses GStreamer to extract the frame.
    pub fn decode_and_upload_frame(&mut self, ctx: &egui::Context) {
        let _ = gst::init(); // Safe to call multiple times

        let path_str = self.path.to_string_lossy();
        let pipeline_str = format!(
            "filesrc location=\"{}\" ! decodebin ! videoconvert ! video/x-raw,format=RGBA ! appsink name=sink",
            path_str
        );

        let pipeline = match gst::parse::launch(&pipeline_str) {
            Ok(p) => p,
            Err(_) => {
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
        let seek_time = (self.current_frame as f64 / fps * 1_000_000_000.0) as u64;
        pipeline.set_state(gst::State::Paused).ok();
        pipeline
            .seek_simple(
                gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                gst::ClockTime::from_nseconds(seek_time),
            )
            .ok();
        pipeline.set_state(gst::State::Playing).ok();

        // Pull the sample from appsink
        let sink = match pipeline.by_name("sink") {
            Some(s) => match s.clone().downcast::<gst_app::AppSink>() {
                Ok(appsink) => appsink,
                Err(_) => {
                    self.texture = None;
                    pipeline.set_state(gst::State::Null).ok();
                    return;
                }
            },
            None => {
                self.texture = None;
                pipeline.set_state(gst::State::Null).ok();
                return;
            }
        };

        let sample_result = sink.pull_sample();
        pipeline.set_state(gst::State::Null).ok();

        if let Ok(sample) = sample_result {
            let buffer = sample.buffer().unwrap();
            let map = buffer.map_readable().unwrap();
            let caps = sample.caps().unwrap();
            let s = caps.structure(0).unwrap();
            let width = s.get::<i32>("width").unwrap() as u32;
            let height = s.get::<i32>("height").unwrap() as u32;
            let img = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, map.as_slice().to_vec())
                .unwrap();
            let color_img = egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                bytemuck::cast_slice(img.as_raw()),
            );
            self.texture =
                Some(ctx.load_texture("video_frame", color_img, egui::TextureOptions::default()));
        } else {
            self.texture = None;
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
