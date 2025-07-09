use crate::ops::clip_ops::cut_clip_at;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeline {
    pub tracks: Vec<Track>,
    pub duration: f64,
    pub frame_rate: f64,
    pub resolution: (u32, u32),
}

impl Timeline {
    /// Returns all active video clips at a specific time.
    pub fn active_video_clips_at(&self, time: f64) -> Vec<&VideoClip> {
        self.tracks
            .iter()
            .filter_map(|track| match track {
                Track::Video(video_track) => Some(video_track),
                _ => None,
            })
            .flat_map(|video_track| {
                video_track
                    .clips
                    .iter()
                    .filter(move |clip| clip.is_active_at(time))
            })
            .collect()
    }
}

/// Splits the first clip found at the given playhead on the specified track.
/// Returns true if a split occurred, false otherwise.
impl Timeline {
    pub fn split_clip_at_playhead(&mut self, track_id: &str, playhead: f64) -> bool {
        for track in &mut self.tracks {
            match track {
                Track::Video(video_track) if video_track.id == track_id => {
                    for i in 0..video_track.clips.len() {
                        let clip = &video_track.clips[i];
                        if playhead > clip.start_time && playhead < clip.start_time + clip.duration
                        {
                            if let Some((left, right)) = cut_clip_at(clip, playhead) {
                                // Replace the original clip with the two new clips
                                video_track.clips.remove(i);
                                video_track.clips.insert(i, right);
                                video_track.clips.insert(i, left);
                                return true;
                            }
                        }
                    }
                }
                Track::Audio(audio_track) if audio_track.id == track_id => {
                    for i in 0..audio_track.clips.len() {
                        let clip = &audio_track.clips[i];
                        if playhead > clip.start_time && playhead < clip.start_time + clip.duration
                        {
                            if let Some((left, right)) = cut_clip_at(clip, playhead) {
                                audio_track.clips.remove(i);
                                audio_track.clips.insert(i, right);
                                audio_track.clips.insert(i, left);
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
}

use crate::types::media::{AudioClip, VideoClip};
use crate::types::track::Track;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActiveClip {
    Video(VideoClip),
    Audio(AudioClip),
}

impl Timeline {
    pub fn new() -> Self {
        Timeline {
            tracks: Vec::new(),
            duration: 0.0,
            frame_rate: 30.0,
            resolution: (1920, 1080),
        }
    }

    /// Returns all clips (audio and video) active at a specific time.
    pub fn active_clips_at(&self, time: f64) -> Vec<ActiveClip> {
        let mut result = Vec::new();
        for track in &self.tracks {
            match track {
                Track::Video(video_track) => {
                    for clip in &video_track.clips {
                        if clip.start_time <= time && time < clip.start_time + clip.duration {
                            result.push(ActiveClip::Video(clip.clone()));
                        }
                    }
                }
                Track::Audio(audio_track) => {
                    for clip in &audio_track.clips {
                        if clip.start_time <= time && time < clip.start_time + clip.duration {
                            result.push(ActiveClip::Audio(clip.clone()));
                        }
                    }
                }
            }
        }
        result
    }

    /// Returns all clips (audio and video) that overlap with a given time range.
    pub fn clips_in_range(&self, start: f64, end: f64) -> Vec<ActiveClip> {
        let mut result = Vec::new();
        for track in &self.tracks {
            match track {
                Track::Video(video_track) => {
                    for clip in &video_track.clips {
                        let clip_start = clip.start_time;
                        let clip_end = clip.start_time + clip.duration;
                        if clip_end > start && clip_start < end {
                            result.push(ActiveClip::Video(clip.clone()));
                        }

                        impl Track {
                            pub fn is_video(&self) -> bool {
                                matches!(self, Track::Video(_))
                            }
                        }

                        impl VideoClip {
                            pub fn is_active_at(&self, time: f64) -> bool {
                                time >= self.start_time && time < self.start_time + self.duration
                            }
                        }
                    }
                }
                Track::Audio(audio_track) => {
                    for clip in &audio_track.clips {
                        let clip_start = clip.start_time;
                        let clip_end = clip.start_time + clip.duration;
                        if clip_end > start && clip_start < end {
                            result.push(ActiveClip::Audio(clip.clone()));
                        }
                    }
                }
            }
        }
        result
    }

    /// Returns all clips on a specific track by track id.
    pub fn clips_on_track(&self, track_id: &str) -> Option<Vec<ActiveClip>> {
        self.tracks
            .iter()
            .find(|t| match t {
                Track::Video(v) => v.id == track_id,
                Track::Audio(a) => a.id == track_id,
            })
            .map(|track| match track {
                Track::Video(v) => v.clips.iter().cloned().map(ActiveClip::Video).collect(),
                Track::Audio(a) => a.clips.iter().cloned().map(ActiveClip::Audio).collect(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::ActiveClip;
    use super::*;
    use crate::types::media::{AudioClip, AudioMetadata, VideoClip, VideoMetadata};
    use crate::types::track::{AudioTrack, Track, VideoTrack};
    #[test]
    fn test_split_clip_at_playhead_video() {
        let video_clip = VideoClip {
            id: "v1".to_string(),
            asset_path: "video.mp4".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: VideoMetadata {
                resolution: (1920, 1080),
                frame_rate: 30.0,
                codec: "h264".to_string(),
            },
        };
        let video_track = VideoTrack {
            id: "vt1".to_string(),
            name: "Video Track 1".to_string(),
            clips: vec![video_clip.clone()],
            muted: false,
        };
        let mut timeline = Timeline {
            tracks: vec![Track::Video(video_track)],
            duration: 10.0,
            frame_rate: 30.0,
            resolution: (1920, 1080),
        };
        let split = timeline.split_clip_at_playhead("vt1", 4.0);
        assert!(split);
        if let Track::Video(ref vt) = timeline.tracks[0] {
            assert_eq!(vt.clips.len(), 2);
            assert_eq!(vt.clips[0].start_time, 0.0);
            assert_eq!(vt.clips[0].duration, 4.0);
            assert_eq!(vt.clips[1].start_time, 4.0);
            assert_eq!(vt.clips[1].duration, 6.0);
            assert_eq!(vt.clips[0].id, "v1_left");
            assert_eq!(vt.clips[1].id, "v1_right");
        } else {
            panic!("Expected video track");
        }
    }

    #[test]
    fn test_split_clip_at_playhead_audio() {
        let audio_clip = AudioClip {
            id: "a1".to_string(),
            asset_path: "audio.wav".to_string(),
            in_point: 0.0,
            out_point: 8.0,
            start_time: 2.0,
            duration: 8.0,
            metadata: AudioMetadata {
                sample_rate: 48000,
                channels: 2,
                codec: "pcm".to_string(),
                bitrate: 1536,
            },
        };
        let audio_track = AudioTrack {
            id: "at1".to_string(),
            name: "Audio Track 1".to_string(),
            clips: vec![audio_clip.clone()],
            muted: false,
        };
        let mut timeline = Timeline {
            tracks: vec![Track::Audio(audio_track)],
            duration: 10.0,
            frame_rate: 30.0,
            resolution: (1920, 1080),
        };
        let split = timeline.split_clip_at_playhead("at1", 6.0);
        assert!(split);
        if let Track::Audio(ref at) = timeline.tracks[0] {
            assert_eq!(at.clips.len(), 2);
            assert_eq!(at.clips[0].start_time, 2.0);
            assert_eq!(at.clips[0].duration, 4.0);
            assert_eq!(at.clips[1].start_time, 6.0);
            assert_eq!(at.clips[1].duration, 4.0);
            assert_eq!(at.clips[0].id, "a1_left");
            assert_eq!(at.clips[1].id, "a1_right");
        } else {
            panic!("Expected audio track");
        }
    }

    #[test]
    fn test_split_clip_at_playhead_no_split() {
        let video_clip = VideoClip {
            id: "v1".to_string(),
            asset_path: "video.mp4".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: VideoMetadata {
                resolution: (1920, 1080),
                frame_rate: 30.0,
                codec: "h264".to_string(),
            },
        };
        let video_track = VideoTrack {
            id: "vt1".to_string(),
            name: "Video Track 1".to_string(),
            clips: vec![video_clip.clone()],
            muted: false,
        };
        let mut timeline = Timeline {
            tracks: vec![Track::Video(video_track)],
            duration: 10.0,
            frame_rate: 30.0,
            resolution: (1920, 1080),
        };
        // Playhead at start (should not split)
        let split = timeline.split_clip_at_playhead("vt1", 0.0);
        assert!(!split);
        // Playhead at end (should not split)
        let split = timeline.split_clip_at_playhead("vt1", 10.0);
        assert!(!split);
        // Playhead not on any clip (should not split)
        let split = timeline.split_clip_at_playhead("vt1", 20.0);
        assert!(!split);
    }

    #[test]
    fn test_create_timeline_with_tracks() {
        let video_clip = VideoClip {
            id: "v1".to_string(),
            asset_path: "video.mp4".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: VideoMetadata {
                resolution: (1920, 1080),
                frame_rate: 30.0,
                codec: "h264".to_string(),
            },
        };

        let audio_clip = AudioClip {
            id: "a1".to_string(),
            asset_path: "audio.wav".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: AudioMetadata {
                sample_rate: 48000,
                channels: 2,
                codec: "pcm".to_string(),
                bitrate: 1536,
            },
        };

        let video_track = VideoTrack {
            id: "vt1".to_string(),
            name: "Video Track 1".to_string(),
            clips: vec![video_clip.clone()],
            muted: false,
        };

        let audio_track = AudioTrack {
            id: "at1".to_string(),
            name: "Audio Track 1".to_string(),
            clips: vec![audio_clip.clone()],
            muted: false,
        };

        let timeline = Timeline {
            tracks: vec![Track::Video(video_track), Track::Audio(audio_track)],
            duration: 10.0,
            frame_rate: 30.0,
            resolution: (1920, 1080),
        };

        assert_eq!(timeline.tracks.len(), 2);
        assert_eq!(timeline.duration, 10.0);
        assert_eq!(timeline.frame_rate, 30.0);
        assert_eq!(timeline.resolution, (1920, 1080));
    }

    #[test]
    fn test_active_clips_at() {
        let video_clip = VideoClip {
            id: "v1".to_string(),
            asset_path: "video.mp4".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: VideoMetadata {
                resolution: (1920, 1080),
                frame_rate: 30.0,
                codec: "h264".to_string(),
            },
        };

        let audio_clip = AudioClip {
            id: "a1".to_string(),
            asset_path: "audio.wav".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: AudioMetadata {
                sample_rate: 48000,
                channels: 2,
                codec: "pcm".to_string(),
                bitrate: 1536,
            },
        };

        let video_track = VideoTrack {
            id: "vt1".to_string(),
            name: "Video Track 1".to_string(),
            clips: vec![video_clip.clone()],
            muted: false,
        };

        let audio_track = AudioTrack {
            id: "at1".to_string(),
            name: "Audio Track 1".to_string(),
            clips: vec![audio_clip.clone()],
            muted: false,
        };

        let timeline = Timeline {
            tracks: vec![Track::Video(video_track), Track::Audio(audio_track)],
            duration: 10.0,
            frame_rate: 30.0,
            resolution: (1920, 1080),
        };

        // Both clips are active at time 5.0
        let active = timeline.active_clips_at(5.0);
        assert_eq!(active.len(), 2);

        // No clips are active at time 11.0
        let active = timeline.active_clips_at(11.0);
        assert_eq!(active.len(), 0);

        // Both clips are active at time 0.0
        let active = timeline.active_clips_at(0.0);
        assert!(active.iter().any(|c| matches!(c, ActiveClip::Video(_))));
        assert!(active.iter().any(|c| matches!(c, ActiveClip::Audio(_))));
    }

    #[test]
    fn test_clips_in_range() {
        let video_clip = VideoClip {
            id: "v1".to_string(),
            asset_path: "video.mp4".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: VideoMetadata {
                resolution: (1920, 1080),
                frame_rate: 30.0,
                codec: "h264".to_string(),
            },
        };

        let audio_clip = AudioClip {
            id: "a1".to_string(),
            asset_path: "audio.wav".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: AudioMetadata {
                sample_rate: 48000,
                channels: 2,
                codec: "pcm".to_string(),
                bitrate: 1536,
            },
        };

        let video_track = VideoTrack {
            id: "vt1".to_string(),
            name: "Video Track 1".to_string(),
            clips: vec![video_clip.clone()],
            muted: false,
        };

        let audio_track = AudioTrack {
            id: "at1".to_string(),
            name: "Audio Track 1".to_string(),
            clips: vec![audio_clip.clone()],
            muted: false,
        };

        let timeline = Timeline {
            tracks: vec![Track::Video(video_track), Track::Audio(audio_track)],
            duration: 10.0,
            frame_rate: 30.0,
            resolution: (1920, 1080),
        };

        // Both clips overlap with range 5.0..15.0
        let in_range = timeline.clips_in_range(5.0, 15.0);
        assert_eq!(in_range.len(), 2);

        // Both clips overlap with range -5.0..1.0
        let in_range = timeline.clips_in_range(-5.0, 1.0);
        assert_eq!(in_range.len(), 2);

        // No clips overlap with range 11.0..20.0
        let in_range = timeline.clips_in_range(11.0, 20.0);
        assert_eq!(in_range.len(), 0);
    }

    #[test]
    fn test_clips_on_track() {
        let video_clip = VideoClip {
            id: "v1".to_string(),
            asset_path: "video.mp4".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: VideoMetadata {
                resolution: (1920, 1080),
                frame_rate: 30.0,
                codec: "h264".to_string(),
            },
        };

        let audio_clip = AudioClip {
            id: "a1".to_string(),
            asset_path: "audio.wav".to_string(),
            in_point: 0.0,
            out_point: 10.0,
            start_time: 0.0,
            duration: 10.0,
            metadata: AudioMetadata {
                sample_rate: 48000,
                channels: 2,
                codec: "pcm".to_string(),
                bitrate: 1536,
            },
        };

        let video_track = VideoTrack {
            id: "vt1".to_string(),
            name: "Video Track 1".to_string(),
            clips: vec![video_clip.clone()],
            muted: false,
        };

        let audio_track = AudioTrack {
            id: "at1".to_string(),
            name: "Audio Track 1".to_string(),
            clips: vec![audio_clip.clone()],
            muted: false,
        };

        let timeline = Timeline {
            tracks: vec![Track::Video(video_track), Track::Audio(audio_track)],
            duration: 10.0,
            frame_rate: 30.0,
            resolution: (1920, 1080),
        };

        let video_clips = timeline.clips_on_track("vt1").unwrap();
        assert_eq!(video_clips.len(), 1);
        match &video_clips[0] {
            ActiveClip::Video(vc) => assert_eq!(vc.id, "v1"),
            _ => panic!("Expected video clip"),
        }

        let audio_clips = timeline.clips_on_track("at1").unwrap();
        assert_eq!(audio_clips.len(), 1);
        match &audio_clips[0] {
            ActiveClip::Audio(ac) => assert_eq!(ac.id, "a1"),
            _ => panic!("Expected audio clip"),
        }

        // Non-existent track
        assert!(timeline.clips_on_track("notrack").is_none());
    }
}
