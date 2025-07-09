mod ops;
mod types;
mod ui;

use crate::types::media::{VideoClip, VideoMetadata};
use crate::types::playback_state::PlaybackState;
use crate::types::project::{Project, ProjectSettings};
use crate::types::timeline::Timeline;
use crate::types::track::{Track, VideoTrack};
use crate::ui::app::{AppState, CutioApp};
use crate::ui::video_player::VideoPlayer;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    // Dummy video clip and track for testing
    let video_clip = VideoClip {
        id: "clip1".to_string(),
        asset_path: "test.mp4".to_string(),
        in_point: 0.0,
        out_point: 5.0,
        start_time: 0.0,
        duration: 5.0,
        metadata: VideoMetadata {
            resolution: (1920, 1080),
            frame_rate: 30.0,
            codec: "h264".to_string(),
        },
    };

    let timeline = Timeline {
        tracks: vec![],
        frame_rate: 30.0,
        resolution: (1920, 1080),
        duration: 600.0,
        // frame_rate and resolution are private, so do not set them here
    };

    let project = Project {
        name: "Untitled Project".to_string(),
        description: None,
        project_file_path: "".to_string(),
        created_at: "".to_string(),
        last_modified: "".to_string(),
        media_library: crate::types::media_library::MediaLibrary::new(),
        timeline: timeline.clone(),
        cache_dir: "".to_string(),
        render_output_dir: "".to_string(),
        settings: ProjectSettings {
            resolution: (1920, 1080),
            frame_rate: 30.0,
            color_space: "sRGB".to_string(),
        },
    };

    let playback_state = PlaybackState::new();

    // VideoPlayer::new expects two arguments: PathBuf and AppState
    // But AppState is not yet constructed, so we need to construct it first
    // So we construct AppState without video_player first, then create VideoPlayer, then set it in AppState
    let app_state = AppState {
        project,
        video_player: VideoPlayer::new(PathBuf::from("test.mp4")),
        playback_state,
        timeline_state: crate::ui::timeline_widget::TimelineState::new(),
    };

    let app = CutioApp { state: app_state };

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Cutio NLE",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )?;
    Ok(())
}
