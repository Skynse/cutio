use crate::types::track::Track;
use crate::ui::video_player::VideoPlayer;
use eframe::egui;

/// Draws a single timeline track (video or audio) with its clips.
/// - `ui`: The egui UI context.
/// - `track`: The track to render.
/// - `track_idx`: The index of the track in the timeline.
/// - `pixels_per_second`: Horizontal scale (zoom).
/// - `track_height`: Height of the track row.
/// - `on_clip_selected`: Callback invoked with the clip ID when a clip is clicked.
pub fn track_widget(
    ui: &mut egui::Ui,
    track: &Track,
    track_idx: usize,
    pixels_per_second: f32,
    track_height: f32,
    on_clip_selected: impl Fn(&str),
) {
    let clip_height = track_height - 12.0;
    match track {
        Track::Video(video_track) => {
            for clip in &video_track.clips {
                let x = (clip.start_time as f32) * pixels_per_second;
                let w = (clip.duration as f32) * pixels_per_second;
                let rect = egui::Rect::from_min_size(
                    ui.min_rect().min + egui::vec2(x, 6.0),
                    egui::vec2(w, clip_height),
                );
                let response = ui.allocate_rect(rect, egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 4.0, egui::Color32::from_rgb(100, 180, 255));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &clip.id,
                    egui::FontId::proportional(12.0),
                    egui::Color32::BLACK,
                );
                if response.clicked() {
                    on_clip_selected(&clip.id);
                }
            }
        }
        Track::Audio(audio_track) => {
            for clip in &audio_track.clips {
                let x = (clip.start_time as f32) * pixels_per_second;
                let w = (clip.duration as f32) * pixels_per_second;
                let rect = egui::Rect::from_min_size(
                    ui.min_rect().min + egui::vec2(x, 6.0),
                    egui::vec2(w, clip_height),
                );
                let response = ui.allocate_rect(rect, egui::Sense::click());
                ui.painter()
                    .rect_filled(rect, 4.0, egui::Color32::from_rgb(180, 255, 100));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &clip.id,
                    egui::FontId::proportional(12.0),
                    egui::Color32::BLACK,
                );
                if response.clicked() {
                    on_clip_selected(&clip.id);
                }
            }
        }
    }
}
