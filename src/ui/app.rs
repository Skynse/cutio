use std::ops::Sub;

use crate::types::playback_state::PlaybackState;
use crate::types::project::Project;
use crate::types::timeline::{self, Timeline};
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

                // Update more frequently for smoother playback
                if dt >= Duration::from_millis(33) {
                    // ~30 FPS update rate
                    *last = Some(now);
                    dt.as_secs_f64()
                } else {
                    if last.is_none() {
                        *last = Some(now);
                    }
                    0.0
                }
            });

            if elapsed > 0.0 {
                // Clamp elapsed time to prevent large jumps
                let elapsed = elapsed.min(0.1); // Max 100ms jump
                self.state.playback_state.playhead += elapsed;

                // Clamp playhead to timeline duration
                let timeline = &self.state.project.timeline;
                let max_time = timeline.duration.max(999.0);
                self.state.playback_state.playhead =
                    self.state.playback_state.playhead.clamp(0.0, max_time);

                // Update video player frame
                let playhead_frame = (self.state.playback_state.playhead * 30.0) as usize;
                self.state.video_player.set_frame(playhead_frame, ctx);

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
            // Only update frame if not playing (to avoid double updates)
            if !self.state.playback_state.is_playing {
                let playhead_frame = (self.state.playback_state.playhead * 30.0) as usize;
                self.state.video_player.set_frame(playhead_frame, ctx);
            }
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
                                (self.state.playback_state.playhead - 1.0).max(0.0);
                            let timeline = &self.state.project.timeline;
                            let max_time = timeline.duration.max(999.0);
                            self.state.playback_state.playhead =
                                self.state.playback_state.playhead.clamp(0.0, max_time);
                        }
                        if ui.button(">>").clicked() {
                            self.state.playback_state.playhead += 1.0;
                            let timeline = &self.state.project.timeline;
                            let max_time = timeline.duration.max(999.0);
                            self.state.playback_state.playhead =
                                self.state.playback_state.playhead.clamp(0.0, max_time);
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
                                    let timeline = &self.state.project.timeline;
                                    let max_time = timeline.duration.max(999.0);
                                    self.state.playback_state.playhead =
                                        new_time.clamp(0.0, max_time);
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
