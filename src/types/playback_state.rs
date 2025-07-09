#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub playhead: f64,
    pub is_playing: bool,
    pub loop_start: Option<f64>,
    pub loop_end: Option<f64>,
    pub volume: f64,
    pub playback_rate: f64,
}

impl PlaybackState {
    pub fn new() -> Self {
        Self {
            playhead: 0.0,
            is_playing: false,
            loop_start: None,
            loop_end: None,
            volume: 1.0,
            playback_rate: 1.0,
        }
    }
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::new()
    }
}
