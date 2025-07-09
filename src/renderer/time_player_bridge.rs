use crate::renderer::timeline_renderer::{AudioBuffer, TimelineRenderer, VideoFrame};
use crate::types::playback_state::PlaybackState;
use crate::types::timeline::Timeline;
use std::time::Instant;

pub struct TimelinePlayerBridge<'a> {
    pub timeline: &'a Timeline,
    pub renderer: &'a mut TimelineRenderer,
    pub playback_state: PlaybackState,
    pub last_update: Instant,
    pub video_buffer: Vec<VideoFrame>,
    // pub audio_buffer: Vec<AudioBuffer>, // Uncomment if you have audio
}

impl<'a> TimelinePlayerBridge<'a> {
    pub fn new(
        timeline: &'a Timeline,
        renderer: &'a mut TimelineRenderer,
        playback_state: PlaybackState,
    ) -> Self {
        Self {
            timeline,
            renderer,
            playback_state,
            last_update: Instant::now(),
            video_buffer: Vec::new(),
            // audio_buffer: Vec::new(),
        }
    }

    /// Advance playback and update buffers
    pub fn update(&mut self) {
        let now = Instant::now();
        if self.playback_state.is_playing {
            let elapsed = now.duration_since(self.last_update).as_secs_f64();
            self.playback_state.playhead += elapsed * self.playback_state.playback_rate;
        }
        self.last_update = now;

        // Clamp playhead to timeline duration
        let max_time = self.timeline.duration.max(1.0);
        self.playback_state.playhead = self.playback_state.playhead.clamp(0.0, max_time);

        // Render and buffer the current frame
        let frame = self.renderer.render_frame(self.playback_state.playhead);
        self.video_buffer.clear();
        self.video_buffer.push(frame);
        // Do the same for audio if needed
    }

    pub fn seek(&mut self, time: f64) {
        self.playback_state.playhead = time.clamp(0.0, self.timeline.duration.max(1.0));
        self.update();
    }

    pub fn play(&mut self) {
        self.playback_state.is_playing = true;
        self.last_update = Instant::now();
    }

    pub fn pause(&mut self) {
        self.playback_state.is_playing = false;
    }

    pub fn current_frame(&self) -> Option<&VideoFrame> {
        self.video_buffer.last()
    }

    // Add audio methods, stats, etc. as needed
}
