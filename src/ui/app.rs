use std::ops::Sub;

use crate::types::playback_state::PlaybackState;
use crate::types::project::Project;
use crate::types::timeline::Timeline;
use eframe::egui;

use crate::ui::medialib::medialib_panel;
use crate::ui::timeline_widget::{TimelineState, TimelineWidget};

pub struct AppState {
    pub project: Project,
    pub playback_state: PlaybackState,
    pub video_player: crate::ui::video_player::VideoPlayer,
    pub timeline_state: TimelineState,
}

pub struct CutioApp {
    pub state: AppState,
}

impl CutioApp {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl eframe::App for CutioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Timeline playback: advance playhead by 1.0 per second if playing ---
        // Use a local static for last play time, but avoid mutable static ref
        use std::time::{Duration, Instant};
        thread_local! {
            static LAST_PLAY_TIME: std::cell::RefCell<Option<Instant>> = std::cell::RefCell::new(None);
        }

        if self.state.playback_state.is_playing {
            let now = Instant::now();
            let elapsed = LAST_PLAY_TIME.with(|last_play_time| {
                let mut last = last_play_time.borrow_mut();
                let dt = if let Some(last_instant) = *last {
                    now.duration_since(last_instant)
                } else {
                    Duration::from_secs(0)
                };
                if dt >= Duration::from_secs_f32(1.0) {
                    *last = Some(now);
                    1.0
                } else {
                    if last.is_none() {
                        *last = Some(now);
                    }
                    0.0
                }
            });
            if elapsed > 0.0 {
                self.state.playback_state.playhead += elapsed;
                ctx.request_repaint(); // keep ticking
            } else {
                // Schedule next repaint to keep playback smooth
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }

        // Left: Media Library
        egui::SidePanel::left("media_panel").show(ctx, |ui| {
            medialib_panel(
                ui,
                &mut self.state.project.media_library,
                |_medialib| {
                    // TODO: Implement import logic (e.g., file picker)
                },
                |medialib, idx| {
                    // Clone file name before mutable borrow for removal
                    let file_name = if let Some(item) = medialib.all_items().get(idx) {
                        match item {
                            crate::types::media_library::MediaItem::AudioItem(a) => {
                                a.file_descriptor.file_name.clone()
                            }
                            crate::types::media_library::MediaItem::VideoItem(v) => {
                                v.file_descriptor.file_name.clone()
                            }
                        }
                    } else {
                        return;
                    };
                    medialib.remove_by_filename(&file_name);
                },
            );
        });

        // Right/Top: Video Player
        egui::TopBottomPanel::top("video_player_panel").show(ctx, |ui| {
            // The video player should show the frame at the current playhead
            let playhead_frame = self.state.playback_state.playhead as usize;
            self.state.video_player.set_frame(playhead_frame, ctx);
            self.state.video_player.show(ui, ctx);
        });

        // Bottom: Timeline area with playback controls, timeline, and track view
        egui::TopBottomPanel::bottom("timeline_area_panel")
            .resizable(true)
            .min_height(350.0)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Playback controls
                    ui.horizontal(|ui| {
                        if ui
                            .button(if self.state.playback_state.is_playing {
                                "Pause"
                            } else {
                                "Play"
                            })
                            .clicked()
                        {
                            self.state.playback_state.is_playing =
                                !self.state.playback_state.is_playing;
                        }
                        if ui.button("<<").clicked() {
                            self.state.playback_state.playhead =
                                self.state.playback_state.playhead.sub(1.0);
                        }
                        if ui.button(">>").clicked() {
                            self.state.playback_state.playhead += 1.0;
                        }
                        // Seek bar (scrubber)
                        let total_frames = self.state.video_player.total_frames.max(1);
                        let mut playhead_frame = self.state.playback_state.playhead as usize;
                        let slider = egui::Slider::new(&mut playhead_frame, 0..=total_frames - 1)
                            .text("Seek");
                        if ui.add(slider).changed() {
                            self.state.playback_state.playhead = playhead_frame as f64;
                        }
                    });

                    // Timeline and track view
                    ui.group(|ui| {
                        let timeline = &mut self.state.project.timeline;
                        let timeline_events = TimelineWidget::new(
                            timeline,
                            &mut self.state.timeline_state,
                            self.state.playback_state.playhead,
                        )
                        .show(ui);

                        // Handle timeline events (e.g., playhead moved)
                        for event in timeline_events {
                            match event {
                                crate::ui::timeline_widget::TimelineEvent::PlayheadMoved(
                                    new_time,
                                ) => {
                                    self.state.playback_state.playhead = new_time;
                                }
                                // Handle other events as needed
                                _ => {}
                            }
                        }
                    });
                });
            });

        // Optionally, use CentralPanel for background or other content
        egui::CentralPanel::default().show(ctx, |_ui| {});
    }
}
