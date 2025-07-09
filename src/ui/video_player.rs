use eframe::egui;
use std::sync::{Arc, RwLock};

use crate::renderer::time_player_bridge::TimelinePlayerBridge;
use crate::renderer::timeline_renderer::{TimelineRenderer, VideoFrame};
use crate::types::playback_state::PlaybackState;
use crate::types::timeline::Timeline;

/// A video player widget that displays frames rendered from the timeline.
pub struct VideoPlayer {
    pub timeline: Arc<RwLock<Timeline>>,
    pub renderer: TimelineRenderer,
    pub player_bridge: TimelinePlayerBridge<'static>,
    pub texture: Option<egui::TextureHandle>,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
}

impl VideoPlayer {
    pub fn new(
        timeline: Arc<RwLock<Timeline>>,
        width: u32,
        height: u32,
        frame_rate: f64,
        playback_state: PlaybackState,
    ) -> Self {
        let renderer = TimelineRenderer::new(timeline.clone(), width, height, frame_rate);
        // SAFETY: We transmute the lifetime to 'static for the bridge, since the renderer is owned by the struct.
        let renderer_ptr: *mut TimelineRenderer = Box::into_raw(Box::new(renderer));
        let renderer_ref = unsafe { &mut *renderer_ptr };
        let timeline_ref: &'static Timeline =
            unsafe { &*(&*timeline.read().unwrap() as *const Timeline) };
        let player_bridge = TimelinePlayerBridge::new(timeline_ref, renderer_ref, playback_state);

        Self {
            timeline,
            renderer: unsafe { std::ptr::read(renderer_ptr) },
            player_bridge,
            texture: None,
            width,
            height,
            frame_rate,
        }
    }

    /// Set the playhead time and update the frame.
    pub fn set_playhead(&mut self, time: f64, ctx: &egui::Context) {
        self.player_bridge.seek(time);
        self.update_texture(ctx);
    }

    /// Advance playback and update the frame.
    pub fn update_playback(&mut self, is_playing: bool, ctx: &egui::Context) {
        if is_playing {
            self.player_bridge.play();
        } else {
            self.player_bridge.pause();
        }
        self.player_bridge.update();
        self.update_texture(ctx);
    }

    /// Update the egui texture from the current VideoFrame.
    pub fn update_texture(&mut self, ctx: &egui::Context) {
        if let Some(frame) = self.player_bridge.current_frame() {
            let color_img = egui::ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.data,
            );
            self.texture = Some(ctx.load_texture(
                "timeline_video_frame",
                color_img,
                egui::TextureOptions::default(),
            ));
        } else {
            self.texture = None;
        }
    }

    /// Show the video player panel in egui.
    pub fn show(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.vertical(|ui| {
            ui.heading("Video Player");
            if let Some(texture) = &self.texture {
                ui.image(texture);
            } else {
                ui.label("No frame loaded");
            }
        });
    }
}
