mod ops;
mod renderer;
mod types;
mod ui;

use crate::types::media::{VideoClip, VideoMetadata};
use crate::types::playback_state::PlaybackState;
use crate::types::project::{Project, ProjectSettings};
use crate::types::timeline::Timeline;
use crate::types::track::{Track, VideoTrack};
use crate::ui::app::{AppState, CutioApp};
use crate::ui::timeline_widget::TimelineState;
use crate::ui::video_player::VideoPlayer;
use gstreamer as gst;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let _ = gst::init();
    // Dummy video clip and track for testing
    let video_clip = VideoClip {
        id: "clip1".to_string(),
        asset_path: r"C:\Users\austi\projects\cutio\testdata\sample.mp4".to_string(),
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
    use std::sync::{Arc, RwLock};
    let timeline_arc = Arc::new(RwLock::new(timeline.clone()));

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

    let timeline_arc = Arc::new(RwLock::new(timeline.clone()));
    let video_player = VideoPlayer::new(
        timeline_arc.clone(),
        640,  // width for preview
        360,  // height for preview
        30.0, // frame rate
        playback_state.clone(),
    );
    let app_state = AppState {
        project,
        playback_state,
        video_player,
        timeline: timeline_arc.clone(),
        timeline_state: TimelineState::new(),
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
