use eframe::egui;
use image::{ImageBuffer, Rgba};
use std::path::PathBuf;

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
    /// This is a stub: you should implement actual frame extraction with ffmpeg-next.
    pub fn decode_and_upload_frame(&mut self, ctx: &egui::Context) {
        // TODO: Use ffmpeg-next to decode the frame at self.current_frame from self.path.
        // For now, just create a placeholder image.
        let width = 320;
        let height = 180;
        let mut img = ImageBuffer::<Rgba<u8>, Vec<u8>>::new(width, height);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let r = (x as u8).wrapping_add(self.current_frame as u8 * 3);
            let g = (y as u8).wrapping_add(self.current_frame as u8 * 7);
            let b = 128u8;
            *pixel = Rgba([r, g, b, 255]);
        }
        let color_img = egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            bytemuck::cast_slice(img.as_raw()),
        );
        self.texture =
            Some(ctx.load_texture("video_frame", color_img, egui::TextureOptions::default()));
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
