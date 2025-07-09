use eframe::egui::{self, StrokeKind};

/// Timeline widget state that persists between frames
#[derive(Default)]
pub struct TimelineState {
    /// Horizontal scroll position in pixels
    pub scroll_x: f32,
    /// Current zoom level (pixels per second)
    pub zoom: f32,
    /// Selected clips
    pub selected_clips: std::collections::HashSet<String>,
    /// Drag state
    pub drag_state: Option<DragState>,
    /// Timeline duration cache
    pub cached_duration: f64,
}

#[derive(Debug, Clone)]
pub enum DragState {
    /// Dragging the playhead
    Playhead { start_pos: egui::Pos2 },
    /// Dragging a clip
    Clip {
        clip_id: String,
        track_idx: usize,
        start_pos: egui::Pos2,
        original_start_time: f64,
    },
    /// Resizing a clip from the left edge
    ResizeLeft {
        clip_id: String,
        track_idx: usize,
        start_pos: egui::Pos2,
        original_start_time: f64,
        original_duration: f64,
    },
    /// Resizing a clip from the right edge
    ResizeRight {
        clip_id: String,
        track_idx: usize,
        start_pos: egui::Pos2,
        original_duration: f64,
    },
    /// Selecting multiple clips
    Selection {
        start_pos: egui::Pos2,
        current_pos: egui::Pos2,
    },
}

#[derive(Debug, Clone)]
pub enum TimelineEvent {
    /// Playhead position changed
    PlayheadMoved(f64),
    /// Clip was moved
    ClipMoved {
        clip_id: String,
        track_idx: usize,
        new_start_time: f64,
    },
    /// Clip was resized
    ClipResized {
        clip_id: String,
        track_idx: usize,
        new_start_time: f64,
        new_duration: f64,
    },
    /// Clip was selected
    ClipSelected {
        clip_id: String,
        track_idx: usize,
        multi_select: bool,
    },
    /// Clip was double-clicked
    ClipDoubleClicked { clip_id: String, track_idx: usize },
    /// Timeline was right-clicked
    RightClicked { time: f64, track_idx: Option<usize> },
}

impl TimelineState {
    pub fn new() -> Self {
        Self {
            scroll_x: 0.0,
            zoom: 100.0, // Default: 100 pixels per second
            selected_clips: std::collections::HashSet::new(),
            drag_state: None,
            cached_duration: 0.0,
        }
    }

    /// Convert time to screen x position
    pub fn time_to_x(&self, time: f64) -> f32 {
        let a = (time as f32 * self.zoom) - self.scroll_x;
        a
    }

    /// Convert screen x position to time
    pub fn x_to_time(&self, x: f32) -> f64 {
        let a = ((x + self.scroll_x) / self.zoom) as f64;
        a
    }

    /// Snap time to grid if enabled
    pub fn snap_time(&self, time: f64, snap_enabled: bool) -> f64 {
        if snap_enabled {
            let snap_interval = 0.1; // Snap to 100ms intervals
            (time / snap_interval).round() * snap_interval
        } else {
            time
        }
    }
}

/// Timeline widget implementation
pub struct TimelineWidget<'a> {
    timeline: &'a mut crate::types::timeline::Timeline,
    state: &'a mut TimelineState,
    playhead: f64,
    snap_enabled: bool,
    show_waveforms: bool,
}

impl<'a> TimelineWidget<'a> {
    pub fn new(
        timeline: &'a mut crate::types::timeline::Timeline,
        state: &'a mut TimelineState,
        playhead: f64,
    ) -> Self {
        Self {
            timeline,
            state,
            playhead,
            snap_enabled: true,
            show_waveforms: false,
        }
    }

    pub fn snap_enabled(mut self, enabled: bool) -> Self {
        self.snap_enabled = enabled;
        self
    }

    pub fn show_waveforms(mut self, show: bool) -> Self {
        self.show_waveforms = show;
        self
    }
    pub fn show(&mut self, ui: &mut egui::Ui) -> Vec<TimelineEvent> {
        let mut events = Vec::new();

        // For drag-and-drop: store dropped media info
        let mut dropped_media: Option<(crate::types::media_library::MediaItem, f64, usize)> = None;

        // Layout constants
        const TRACK_HEIGHT: f32 = 60.0;
        const CLIP_HEIGHT: f32 = 40.0;
        const RULER_HEIGHT: f32 = 30.0;
        const TRACK_LABEL_WIDTH: f32 = 120.0;
        const RESIZE_HANDLE_WIDTH: f32 = 8.0;

        // --- Add Track Button and Playback Controls Bar ---
        ui.horizontal(|ui| {
            if ui.button("+ Add Track").clicked() {
                // Add a new empty video track for demonstration (customize as needed)
                self.timeline.tracks.push(crate::types::track::Track::Video(
                    crate::types::track::VideoTrack {
                        id: format!("track{}", self.timeline.tracks.len() + 1),
                        name: format!("Video Track {}", self.timeline.tracks.len() + 1),
                        clips: vec![],
                        muted: false,
                    },
                ));
            }
            if ui.button("⏮").clicked() { /* jump to start logic */ }
            if ui.button("⏪").clicked() { /* step back logic */ }
            if ui.button("⏯").clicked() { /* play/pause logic */ }
            if ui.button("⏩").clicked() { /* step forward logic */ }
            ui.label(format!("Speed: {:.1}x", 1.0));
            ui.label(format!("Time: {}", format_time(self.playhead)));
        });
        ui.add_space(4.0);

        // Calculate dimensions
        let timeline_width =
            (self.timeline.duration as f32 * self.state.zoom).max(ui.available_width());
        let min_tracks = 3;
        let timeline_height = (self.timeline.tracks.len().max(min_tracks) as f32) * TRACK_HEIGHT;
        let total_height = RULER_HEIGHT + timeline_height;

        // --- Scrollable Timeline Viewport ---
        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                // Set a large inner area for scrolling
                ui.set_min_size(egui::vec2(timeline_width + TRACK_LABEL_WIDTH, total_height));

                // --- Layout rects ---
                let timeline_rect = egui::Rect::from_min_size(
                    ui.min_rect().min,
                    egui::vec2(timeline_width + TRACK_LABEL_WIDTH, total_height),
                );
                let ruler_rect = egui::Rect::from_min_max(
                    timeline_rect.left_top() + egui::vec2(TRACK_LABEL_WIDTH, 0.0),
                    timeline_rect.left_top() + egui::vec2(timeline_rect.width(), RULER_HEIGHT),
                );
                let tracks_rect = egui::Rect::from_min_max(
                    timeline_rect.left_top() + egui::vec2(TRACK_LABEL_WIDTH, RULER_HEIGHT),
                    timeline_rect.right_bottom(),
                );
                let track_list_rect = egui::Rect::from_min_max(
                    timeline_rect.left_top() + egui::vec2(0.0, RULER_HEIGHT),
                    timeline_rect.left_top()
                        + egui::vec2(TRACK_LABEL_WIDTH, timeline_rect.height()),
                );

                let painter = ui.painter_at(timeline_rect);

                // Draw background
                painter.rect_filled(timeline_rect, 0.0, ui.style().visuals.window_fill);

                // --- Track List (Left) ---
                for (track_idx, track) in self.timeline.tracks.iter_mut().enumerate() {
                    let y = track_list_rect.top() + track_idx as f32 * TRACK_HEIGHT;
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(track_list_rect.left(), y),
                        egui::vec2(track_list_rect.width(), TRACK_HEIGHT),
                    );
                    painter.rect_filled(rect, 0.0, egui::Color32::DARK_GRAY);

                    // Mute/unmute button
                    let (track_name, is_muted) = match track {
                        crate::types::track::Track::Video(video_track) => {
                            (&video_track.name, &mut video_track.muted)
                        }
                        crate::types::track::Track::Audio(audio_track) => {
                            (&audio_track.name, &mut audio_track.muted)
                        }
                    };
                    let mute_label = if *is_muted { "🔇" } else { "🔊" };
                    let button_rect = egui::Rect::from_min_size(
                        rect.left_top() + egui::vec2(4.0, 4.0),
                        egui::vec2(28.0, 28.0),
                    );
                    if ui.put(button_rect, egui::Button::new(mute_label)).clicked() {
                        *is_muted = !*is_muted;
                    }

                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        track_name,
                        egui::FontId::proportional(14.0),
                        egui::Color32::WHITE,
                    );
                }

                // --- Drag-and-drop drop zone for timeline area ---
                let drop_zone_frame = egui::Frame::default().inner_margin(4.0);
                let (_, dropped_payload) = ui
                    .dnd_drop_zone::<crate::types::media_library::MediaItem, ()>(
                        drop_zone_frame,
                        |ui| {
                            // Highlight if hovered
                            if ui.ctx().dragged_id().is_some() {
                                if let Some(hover_pos) = ui.ctx().input(|i| i.pointer.latest_pos())
                                {
                                    if tracks_rect.contains(hover_pos) {
                                        let drop_time = self
                                            .state
                                            .x_to_time(hover_pos.x - tracks_rect.left())
                                            .max(0.0);
                                        let drop_x = self.state.time_to_x(drop_time);
                                        let drop_track_idx = ((hover_pos.y - tracks_rect.top())
                                            / TRACK_HEIGHT)
                                            .floor()
                                            as usize;

                                        // Draw drop indicator line
                                        painter.line_segment(
                                            [
                                                egui::pos2(
                                                    tracks_rect.left() + drop_x,
                                                    tracks_rect.top(),
                                                ),
                                                egui::pos2(
                                                    tracks_rect.left() + drop_x,
                                                    tracks_rect.bottom(),
                                                ),
                                            ],
                                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                                        );

                                        // Optionally, highlight the track
                                        let track_y = tracks_rect.top()
                                            + drop_track_idx as f32 * TRACK_HEIGHT;
                                        let track_rect = egui::Rect::from_min_size(
                                            egui::pos2(tracks_rect.left(), track_y),
                                            egui::vec2(tracks_rect.width(), TRACK_HEIGHT),
                                        );
                                        painter.rect_stroke(
                                            track_rect,
                                            0.0,
                                            egui::Stroke::new(2.0, egui::Color32::YELLOW),
                                            egui::StrokeKind::Outside,
                                        );
                                    }
                                }
                            }
                        },
                    );
                if let Some(media_arc) = dropped_payload {
                    if let Some(media) = std::sync::Arc::into_inner(media_arc) {
                        // Determine drop position
                        let pointer_pos = ui.ctx().input(|i| i.pointer.latest_pos());
                        let (drop_time, drop_track_idx) = if let Some(pos) = pointer_pos {
                            (
                                self.state.x_to_time(pos.x - tracks_rect.left()).max(0.0),
                                {
                                    let idx = ((pos.y - tracks_rect.top()) / TRACK_HEIGHT).floor() as usize;
                                    let clamped_idx = idx.min(self.timeline.tracks.len());
                                    println!("Calculated drop_track_idx: {}, clamped to: {}, tracks.len(): {}", idx, clamped_idx, self.timeline.tracks.len());
                                    clamped_idx
                                },
                            )
                        } else {
                            (0.0, self.timeline.tracks.len())
                        };

                        match media {
                            crate::types::media_library::MediaItem::VideoItem(video) => {
                                // Try to add to an existing video track at drop_track_idx, or create a new one
                                let mut added = false;
                                if !self.timeline.tracks.is_empty() && drop_track_idx < self.timeline.tracks.len() {
                                    if let Some(track) = self.timeline.tracks.get_mut(drop_track_idx) {
                                        if let crate::types::track::Track::Video(video_track) = track {
                                            video_track.clips.push(crate::types::media::VideoClip {
                                                id: format!("clip{}", video_track.clips.len() + 1),
                                                asset_path: video.file_descriptor.path.clone(),
                                                in_point: 0.0,
                                                out_point: 5.0,
                                                start_time: drop_time,
                                                duration: 5.0,
                                                metadata: crate::types::media::VideoMetadata {
                                                    resolution: (1920, 1080),
                                                    frame_rate: 30.0,
                                                    codec: "unknown".to_string(),
                                                },
                                            });
                                            added = true;
                                        }
                                    }
                                }
                                if !added {
                                    // Create a new video track and add the clip
                                    let mut video_track = crate::types::track::VideoTrack {
                                        id: format!("track{}", self.timeline.tracks.len() + 1),
                                        name: format!(
                                            "Video Track {}",
                                            self.timeline.tracks.len() + 1
                                        ),
                                        clips: vec![],
                                        muted: false,
                                    };
                                    video_track.clips.push(crate::types::media::VideoClip {
                                        id: "clip1".to_string(),
                                        asset_path: video.file_descriptor.path.clone(),
                                        in_point: 0.0,
                                        out_point: 5.0,
                                        start_time: drop_time,
                                        duration: 5.0,
                                        metadata: crate::types::media::VideoMetadata {
                                            resolution: (1920, 1080),
                                            frame_rate: 30.0,
                                            codec: "unknown".to_string(),
                                        },
                                    });
                                    self.timeline
                                        .tracks
                                        .push(crate::types::track::Track::Video(video_track));
                                }
                            }
                            crate::types::media_library::MediaItem::AudioItem(audio) => {
                                // Try to add to an existing audio track at drop_track_idx, or create a new one
                                let mut added = false;
                                if !self.timeline.tracks.is_empty() && drop_track_idx < self.timeline.tracks.len() {
                                    if let Some(track) = self.timeline.tracks.get_mut(drop_track_idx) {
                                        if let crate::types::track::Track::Audio(audio_track) = track {
                                            audio_track.clips.push(crate::types::media::AudioClip {
                                                id: format!("clip{}", audio_track.clips.len() + 1),
                                                asset_path: audio.file_descriptor.path.clone(),
                                                in_point: 0.0,
                                                out_point: 5.0,
                                                start_time: drop_time,
                                                duration: 5.0,
                                                metadata: crate::types::media::AudioMetadata {
                                                    sample_rate: 44100,
                                                    channels: 2,
                                                    codec: "unknown".to_string(),
                                                    bitrate: 0,
                                                },
                                            });
                                            added = true;
                                        }
                                    }
                                }
                                if !added {
                                    // Create a new audio track and add the clip
                                    let mut audio_track = crate::types::track::AudioTrack {
                                        id: format!("track{}", self.timeline.tracks.len() + 1),
                                        name: format!(
                                            "Audio Track {}",
                                            self.timeline.tracks.len() + 1
                                        ),
                                        clips: vec![],
                                        muted: false,
                                    };
                                    audio_track.clips.push(crate::types::media::AudioClip {
                                        id: "clip1".to_string(),
                                        asset_path: audio.file_descriptor.path.clone(),
                                        in_point: 0.0,
                                        out_point: 5.0,
                                        start_time: drop_time,
                                        duration: 5.0,
                                        metadata: crate::types::media::AudioMetadata {
                                            sample_rate: 44100,
                                            channels: 2,
                                            codec: "unknown".to_string(),
                                            bitrate: 0,
                                        },
                                    });
                                    self.timeline
                                        .tracks
                                        .push(crate::types::track::Track::Audio(audio_track));
                                }
                            }
                        }
                    }
                }

                // --- Draw time ruler ---
                self.draw_ruler(&painter, ruler_rect, RULER_HEIGHT);

                // --- Make ruler interactive for seeking ---
                let ruler_response = ui.allocate_rect(ruler_rect, egui::Sense::click_and_drag());
                if ruler_response.clicked() || ruler_response.dragged() {
                    if let Some(pointer_pos) = ruler_response.interact_pointer_pos() {
                        let local_x = pointer_pos.x - ruler_rect.left();
                        let max_time = self.timeline.duration.max(999.0);
                        let new_time = self.state.x_to_time(local_x).max(0.0).min(max_time);
                        println!("Ruler interaction: local_x={}, new_time={}", local_x, new_time);
                        events.push(TimelineEvent::PlayheadMoved(new_time));
                    }
                }

                // --- Draw tracks and clips ---
                for (track_idx, track) in self.timeline.tracks.iter().enumerate() {
                    let track_y = tracks_rect.top() + track_idx as f32 * TRACK_HEIGHT;
                    let track_rect = egui::Rect::from_min_size(
                        egui::pos2(tracks_rect.left(), track_y),
                        egui::vec2(tracks_rect.width(), TRACK_HEIGHT),
                    );
                    // Draw track background
                    let track_bg_color = ui.style().visuals.widgets.noninteractive.bg_fill;
                    painter.rect_filled(track_rect, 0.0, track_bg_color);
                    painter.line_segment(
                        [track_rect.left_bottom(), track_rect.right_bottom()],
                        egui::Stroke::new(
                            1.0,
                            ui.style().visuals.widgets.noninteractive.bg_stroke.color,
                        ),
                    );

                    // --- Draw clips directly in the track area, with drag support ---
                    let clips: Vec<_> = match track {
                        crate::types::track::Track::Video(video_track) => video_track
                            .clips
                            .iter()
                            .map(|c| (&c.id, c.start_time, c.duration))
                            .collect(),
                        crate::types::track::Track::Audio(audio_track) => audio_track
                            .clips
                            .iter()
                            .map(|c| (&c.id, c.start_time, c.duration))
                            .collect(),
                    };

                    for (clip_id, start_time, duration) in clips {
                        let clip_x = self.state.time_to_x(start_time);
                        let clip_width = duration as f32 * self.state.zoom;

                        if clip_x + clip_width < 0.0 || clip_x > track_rect.width() {
                            continue;
                        }

                        let clip_rect = egui::Rect::from_min_size(
                            egui::pos2(track_rect.left() + clip_x, track_rect.top() + 10.0),
                            egui::vec2(clip_width, CLIP_HEIGHT),
                        );

                        let is_selected = self.state.selected_clips.contains(clip_id);
                        let base_color = match track {
                            crate::types::track::Track::Video(_) => {
                                egui::Color32::from_rgb(100, 180, 255)
                            }
                            crate::types::track::Track::Audio(_) => {
                                egui::Color32::from_rgb(180, 255, 100)
                            }
                        };
                        let clip_color = if is_selected {
                            egui::Color32::from_rgb(255, 180, 100)
                        } else {
                            base_color
                        };

                        painter.rect_filled(clip_rect, 4.0, clip_color);

                        let border_color = if is_selected {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_black_alpha(50)
                        };
                        painter.rect_stroke(
                            clip_rect,
                            4.0,
                            egui::Stroke::new(1.0, border_color),
                            egui::StrokeKind::Inside,
                        );

                        if clip_width > 40.0 {
                            painter.text(
                                clip_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                clip_id,
                                egui::FontId::proportional(12.0),
                                egui::Color32::BLACK,
                            );
                        }

                        // Drag/click support
                        let clip_response =
                            ui.allocate_rect(clip_rect, egui::Sense::click_and_drag());

                        if clip_response.clicked() {
                            let multi_select = ui.input(|i| i.modifiers.ctrl);
                            events.push(TimelineEvent::ClipSelected {
                                clip_id: clip_id.clone(),
                                track_idx,
                                multi_select,
                            });
                        }
                        if clip_response.double_clicked() {
                            events.push(TimelineEvent::ClipDoubleClicked {
                                clip_id: clip_id.clone(),
                                track_idx,
                            });
                        }
                        if clip_response.drag_started() {
                            self.state.drag_state = Some(DragState::Clip {
                                clip_id: clip_id.clone(),
                                track_idx,
                                start_pos: clip_response
                                    .interact_pointer_pos()
                                    .unwrap_or(clip_rect.center()),
                                original_start_time: start_time,
                            });
                        }
                    }
                }

                // --- Handle drop of media item onto timeline ---
                // (handled above in the drop zone logic)

                // --- Draw playhead ---
                self.draw_playhead(&painter, ruler_rect, &mut events);

                // --- Handle drag operations ---
                self.handle_drag_operations(ui, timeline_rect, &mut events);

                // --- Handle selection box ---
                if let Some(DragState::Selection {
                    start_pos,
                    current_pos,
                }) = &self.state.drag_state
                {
                    let selection_rect = egui::Rect::from_two_pos(*start_pos, *current_pos);
                    painter.rect_stroke(
                        selection_rect,
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::WHITE),
                        egui::StrokeKind::Outside,
                    );
                    painter.rect_filled(selection_rect, 0.0, egui::Color32::from_white_alpha(20));
                }

                // --- Handle right-click context menu ---
                if ui.ctx().input(|i| i.pointer.secondary_down()) {
                    if let Some(click_pos) = ui.ctx().input(|i| i.pointer.press_origin()) {
                        let time = self
                            .state
                            .x_to_time(click_pos.x - timeline_rect.left())
                            .max(0.0);
                        let track_idx = if click_pos.y > timeline_rect.top() + RULER_HEIGHT {
                            let idx = ((click_pos.y - timeline_rect.top() - RULER_HEIGHT) / TRACK_HEIGHT) as usize;
                            let clamped_idx = if self.timeline.tracks.is_empty() {
                                0
                            } else {
                                idx.min(self.timeline.tracks.len().saturating_sub(1))
                            };
                            println!("Right-click: idx={}, clamped_idx={}, tracks.len={}", idx, clamped_idx, self.timeline.tracks.len());
                            Some(clamped_idx)
                        } else {
                            None
                        };
                        events.push(TimelineEvent::RightClicked { time, track_idx });
                    }
                }
            }); // close .show(ui, |ui| { ... })

        events
    }

    fn draw_ruler(&self, painter: &egui::Painter, timeline_rect: egui::Rect, ruler_height: f32) {
        let ruler_rect = egui::Rect::from_min_size(
            timeline_rect.min,
            egui::vec2(timeline_rect.width(), ruler_height),
        );

        // Draw ruler background
        painter.rect_filled(ruler_rect, 0.0, egui::Color32::from_gray(40));

        // Calculate tick intervals based on zoom
        let pixels_per_second = self.state.zoom;
        let (major_interval, minor_interval) = if pixels_per_second > 200.0 {
            (1.0, 0.1) // 1 second major, 0.1 second minor
        } else if pixels_per_second > 50.0 {
            (5.0, 1.0) // 5 second major, 1 second minor
        } else {
            (10.0, 5.0) // 10 second major, 5 second minor
        };

        // Draw time ticks
        let start_time = self.state.x_to_time(0.0);
        let end_time = self.state.x_to_time(timeline_rect.width());

        // Minor ticks
        let minor_start = (start_time / minor_interval).floor() * minor_interval;
        let mut time = minor_start;
        while time <= end_time {
            let x = self.state.time_to_x(time);
            if x >= 0.0 && x <= timeline_rect.width() {
                painter.line_segment(
                    [
                        egui::pos2(timeline_rect.left() + x, ruler_rect.bottom() - 5.0),
                        egui::pos2(timeline_rect.left() + x, ruler_rect.bottom()),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_gray(120)),
                );
            }
            time += minor_interval;
        }

        // Major ticks with labels
        let major_start = (start_time / major_interval).floor() * major_interval;
        let mut time = major_start;
        while time <= end_time {
            let x = self.state.time_to_x(time);
            if x >= 0.0 && x <= timeline_rect.width() {
                // Draw major tick
                painter.line_segment(
                    [
                        egui::pos2(timeline_rect.left() + x, ruler_rect.bottom() - 15.0),
                        egui::pos2(timeline_rect.left() + x, ruler_rect.bottom()),
                    ],
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                );

                // Draw time label
                let time_str = format!("{:.1}s", time);
                painter.text(
                    egui::pos2(timeline_rect.left() + x + 2.0, ruler_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    time_str,
                    egui::FontId::proportional(11.0),
                    egui::Color32::WHITE,
                );
            }
            time += major_interval;
        }
    }

    fn draw_track_clips(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        track: &crate::types::track::Track,
        track_idx: usize,
        track_rect: egui::Rect,
        events: &mut Vec<TimelineEvent>,
    ) {
        const CLIP_HEIGHT: f32 = 40.0;
        const RESIZE_HANDLE_WIDTH: f32 = 8.0;

        let clips: Vec<_> = match track {
            crate::types::track::Track::Video(video_track) => video_track
                .clips
                .iter()
                .map(|c| (&c.id, c.start_time, c.duration))
                .collect(),
            crate::types::track::Track::Audio(audio_track) => audio_track
                .clips
                .iter()
                .map(|c| (&c.id, c.start_time, c.duration))
                .collect(),
        };

        for (clip_id, start_time, duration) in clips {
            let clip_x = self.state.time_to_x(start_time);
            let clip_width = duration as f32 * self.state.zoom;

            // Skip clips that are completely outside the visible area
            if clip_x + clip_width < 0.0 || clip_x > track_rect.width() {
                continue;
            }

            let clip_rect = egui::Rect::from_min_size(
                egui::pos2(track_rect.left() + clip_x, track_rect.top() + 10.0),
                egui::vec2(clip_width, CLIP_HEIGHT),
            );

            // Determine clip color based on track type and selection
            let is_selected = self.state.selected_clips.contains(clip_id);
            let base_color = match track {
                crate::types::track::Track::Video(_) => egui::Color32::from_rgb(100, 180, 255),
                crate::types::track::Track::Audio(_) => egui::Color32::from_rgb(180, 255, 100),
            };
            let clip_color = if is_selected {
                egui::Color32::from_rgb(255, 180, 100) // Orange for selected
            } else {
                base_color
            };

            // Draw clip background
            painter.rect_filled(clip_rect, 4.0, clip_color);

            // Draw clip border
            let border_color = if is_selected {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_black_alpha(50)
            };
            painter.rect_stroke(
                clip_rect,
                4.0,
                egui::Stroke::new(1.0, border_color),
                StrokeKind::Inside,
            );

            // Draw clip label
            if clip_width > 40.0 {
                painter.text(
                    clip_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    clip_id,
                    egui::FontId::proportional(12.0),
                    egui::Color32::BLACK,
                );
            }

            // Draw resize handles if selected
            if is_selected && clip_width > RESIZE_HANDLE_WIDTH * 2.0 {
                let left_handle = egui::Rect::from_min_size(
                    clip_rect.min,
                    egui::vec2(RESIZE_HANDLE_WIDTH, CLIP_HEIGHT),
                );
                let right_handle = egui::Rect::from_min_size(
                    egui::pos2(clip_rect.right() - RESIZE_HANDLE_WIDTH, clip_rect.top()),
                    egui::vec2(RESIZE_HANDLE_WIDTH, CLIP_HEIGHT),
                );

                painter.rect_filled(left_handle, 0.0, egui::Color32::from_white_alpha(100));
                painter.rect_filled(right_handle, 0.0, egui::Color32::from_white_alpha(100));
            }

            // Handle clip interactions
            let clip_response = ui.allocate_rect(clip_rect, egui::Sense::click_and_drag());

            if clip_response.clicked() {
                let multi_select = ui.input(|i| i.modifiers.ctrl);
                events.push(TimelineEvent::ClipSelected {
                    clip_id: clip_id.clone(),
                    track_idx,
                    multi_select,
                });
            }

            if clip_response.double_clicked() {
                events.push(TimelineEvent::ClipDoubleClicked {
                    clip_id: clip_id.clone(),
                    track_idx,
                });
            }

            // Handle drag start
            if clip_response.drag_started() {
                if let Some(start_pos) = ui.input(|i| i.pointer.press_origin()) {
                    // Check if we're near the edges for resizing
                    let relative_x = start_pos.x - clip_rect.left();

                    if relative_x < RESIZE_HANDLE_WIDTH && is_selected {
                        self.state.drag_state = Some(DragState::ResizeLeft {
                            clip_id: clip_id.clone(),
                            track_idx,
                            start_pos,
                            original_start_time: start_time,
                            original_duration: duration,
                        });
                    } else if relative_x > clip_width - RESIZE_HANDLE_WIDTH && is_selected {
                        self.state.drag_state = Some(DragState::ResizeRight {
                            clip_id: clip_id.clone(),
                            track_idx,
                            start_pos,
                            original_duration: duration,
                        });
                    } else {
                        self.state.drag_state = Some(DragState::Clip {
                            clip_id: clip_id.clone(),
                            track_idx,
                            start_pos,
                            original_start_time: start_time,
                        });
                    }
                }
            }
        }
    }

    fn draw_playhead(
        &self,
        painter: &egui::Painter,
        timeline_rect: egui::Rect,
        events: &mut Vec<TimelineEvent>,
    ) {
        let playhead_x = self.state.time_to_x(self.playhead);

        if playhead_x >= 0.0 && playhead_x <= timeline_rect.width() {
            // Draw playhead line
            painter.line_segment(
                [
                    egui::pos2(timeline_rect.left() + playhead_x, timeline_rect.top()),
                    egui::pos2(timeline_rect.left() + playhead_x, timeline_rect.bottom()),
                ],
                egui::Stroke::new(2.0, egui::Color32::RED),
            );

            // Draw playhead handle
            let handle_rect = egui::Rect::from_center_size(
                egui::pos2(
                    timeline_rect.left() + playhead_x,
                    timeline_rect.top() + 15.0,
                ),
                egui::vec2(12.0, 12.0),
            );
            painter.rect_filled(handle_rect, 6.0, egui::Color32::RED);
        }
    }

    fn handle_drag_operations(
        &mut self,
        ui: &mut egui::Ui,
        timeline_rect: egui::Rect,
        events: &mut Vec<TimelineEvent>,
    ) {
        if let Some(ref drag_state) = self.state.drag_state.clone() {
            if ui.input(|i| i.pointer.any_released()) {
                // End drag operation
                match drag_state {
                    DragState::Clip {
                        clip_id,
                        track_idx,
                        start_pos,
                        original_start_time,
                    } => {
                        if let Some(current_pos) = ui.input(|i| i.pointer.latest_pos()) {
                            let delta_x = current_pos.x - start_pos.x;
                            let delta_time = delta_x / self.state.zoom;
                            let new_start_time = self
                                .state
                                .snap_time(
                                    original_start_time + delta_time as f64,
                                    self.snap_enabled,
                                )
                                .max(0.0);

                            events.push(TimelineEvent::ClipMoved {
                                clip_id: clip_id.clone(),
                                track_idx: *track_idx,
                                new_start_time,
                            });
                        }
                    }
                    DragState::ResizeLeft {
                        clip_id,
                        track_idx,
                        start_pos,
                        original_start_time,
                        original_duration,
                    } => {
                        if let Some(current_pos) = ui.input(|i| i.pointer.latest_pos()) {
                            let delta_x = current_pos.x - start_pos.x;
                            let delta_time = delta_x / self.state.zoom;
                            let new_start_time = self
                                .state
                                .snap_time(
                                    original_start_time + delta_time as f64,
                                    self.snap_enabled,
                                )
                                .max(0.0);
                            let new_duration = (original_duration
                                - (new_start_time - original_start_time))
                                .max(0.1);

                            events.push(TimelineEvent::ClipResized {
                                clip_id: clip_id.clone(),
                                track_idx: *track_idx,
                                new_start_time,
                                new_duration,
                            });
                        }
                    }
                    DragState::ResizeRight {
                        clip_id,
                        track_idx,
                        start_pos,
                        original_duration,
                    } => {
                        if let Some(current_pos) = ui.input(|i| i.pointer.latest_pos()) {
                            let delta_x = current_pos.x - start_pos.x;
                            let delta_time = delta_x / self.state.zoom;
                            let new_duration = self
                                .state
                                .snap_time(original_duration + delta_time as f64, self.snap_enabled)
                                .max(0.1);

                            // For resize right, we need to find the original start time
                            // This is a simplified approach - in a real implementation,
                            // you'd track this in the drag state
                            events.push(TimelineEvent::ClipResized {
                                clip_id: clip_id.clone(),
                                track_idx: *track_idx,
                                new_start_time: 0.0, // You'd need to track this
                                new_duration,
                            });
                        }
                    }
                    DragState::Playhead { start_pos } => {
                        if let Some(current_pos) = ui.input(|i| i.pointer.latest_pos()) {
                            let new_time = self
                                .state
                                .x_to_time(current_pos.x - timeline_rect.left())
                                .max(0.0);
                            let snapped_time =
                                self.state.snap_time(new_time, self.snap_enabled).max(0.0);
                            events.push(TimelineEvent::PlayheadMoved(snapped_time));
                        }
                    }
                    _ => {}
                }

                self.state.drag_state = None;
            }
        }

        // Handle playhead dragging
        if ui.input(|i| i.pointer.primary_down()) {
            if let Some(current_pos) = ui.input(|i| i.pointer.latest_pos()) {
                let playhead_x = self.state.time_to_x(self.playhead);
                let playhead_screen_x = timeline_rect.left() + playhead_x;

                // Check if we're clicking near the playhead
                if (current_pos.x - playhead_screen_x).abs() < 10.0
                    && current_pos.y >= timeline_rect.top()
                    && current_pos.y <= timeline_rect.top() + 30.0
                {
                    if self.state.drag_state.is_none() {
                        self.state.drag_state = Some(DragState::Playhead {
                            start_pos: current_pos,
                        });
                    }
                }
            }
        }
    }
}

// Helper function to format time as MM:SS.mmm
pub fn format_time(seconds: f64) -> String {
    let minutes = (seconds / 60.0) as i32;
    let secs = seconds % 60.0;
    format!("{:02}:{:06.3}", minutes, secs)
}
