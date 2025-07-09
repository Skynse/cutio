use crate::types::project::Project;

#[derive(Debug, Clone, PartialEq)]
pub struct PlaybackState {
    pub playhead: f64, // Current time in seconds or frames
    pub is_playing: bool,
    pub loop_start: Option<f64>,
    pub loop_end: Option<f64>,
}

impl PlaybackState {
    pub fn new() -> Self {
        PlaybackState {
            playhead: 0.0,
            is_playing: false,
            loop_start: None,
            loop_end: None,
        }
    }
}

/// ProjectSession groups a Project (persistent data) and PlaybackState (ephemeral UI state).
/// Only the Project should be serialized; PlaybackState is for runtime use only.
#[derive(Debug, Clone)]
pub struct ProjectSession {
    pub project: Project,
    pub playback_state: PlaybackState,
}

impl ProjectSession {
    pub fn new(project: Project) -> Self {
        ProjectSession {
            project,
            playback_state: PlaybackState::new(),
        }
    }
}
