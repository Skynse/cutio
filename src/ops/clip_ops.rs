use crate::types::media::{AudioClip, AudioMetadata, Clip, VideoClip, VideoMetadata};

/// Cuts a clip at the given playhead position, returning two new clips if the cut is valid.
/// Returns None if the playhead is outside the clip's range.
pub fn cut_clip_at<T>(clip: &T, playhead: f64) -> Option<(T, T)>
where
    T: Clip + Clone + ClipSplit,
{
    let clip_start = clip.start_time();
    let clip_end = clip.start_time() + clip.duration();

    if playhead <= clip_start || playhead >= clip_end {
        return None;
    }

    let (mut left, mut right) = clip.split();

    // Left part: from original start to playhead
    left.set_id(format!("{}_left", clip.id()));
    left.set_in_point(clip.in_point());
    left.set_out_point(clip.in_point() + (playhead - clip_start));
    left.set_start_time(clip_start);
    left.set_duration(playhead - clip_start);

    // Right part: from playhead to original end
    right.set_id(format!("{}_right", clip.id()));
    right.set_in_point(clip.in_point() + (playhead - clip_start));
    right.set_out_point(clip.out_point());
    right.set_start_time(playhead);
    right.set_duration(clip_end - playhead);

    Some((left, right))
}

/// Trait to allow setting fields on a Clip for splitting/cutting.
/// This is needed because the base Clip trait only has getters.
pub trait ClipSplit: Clip {
    fn set_id(&mut self, id: String);
    fn set_in_point(&mut self, in_point: f64);
    fn set_out_point(&mut self, out_point: f64);
    fn set_start_time(&mut self, start_time: f64);
    fn set_duration(&mut self, duration: f64);
    fn split(&self) -> (Self, Self)
    where
        Self: Sized;
}

impl ClipSplit for VideoClip {
    fn set_id(&mut self, id: String) {
        self.id = id;
    }
    fn set_in_point(&mut self, in_point: f64) {
        self.in_point = in_point;
    }
    fn set_out_point(&mut self, out_point: f64) {
        self.out_point = out_point;
    }
    fn set_start_time(&mut self, start_time: f64) {
        self.start_time = start_time;
    }
    fn set_duration(&mut self, duration: f64) {
        self.duration = duration;
    }
    fn split(&self) -> (Self, Self) {
        (self.clone(), self.clone())
    }
}

impl ClipSplit for AudioClip {
    fn set_id(&mut self, id: String) {
        self.id = id;
    }
    fn set_in_point(&mut self, in_point: f64) {
        self.in_point = in_point;
    }
    fn set_out_point(&mut self, out_point: f64) {
        self.out_point = out_point;
    }
    fn set_start_time(&mut self, start_time: f64) {
        self.start_time = start_time;
    }
    fn set_duration(&mut self, duration: f64) {
        self.duration = duration;
    }
    fn split(&self) -> (Self, Self) {
        (self.clone(), self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::media::{AudioClip, AudioMetadata, VideoClip, VideoMetadata};

    #[test]
    fn test_cut_video_clip_at_middle() {
        let clip = VideoClip {
            id: "vc1".to_string(),
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
        let playhead = 4.0;
        let (left, right) = cut_clip_at(&clip, playhead).unwrap();
        assert_eq!(left.id, "vc1_left");
        assert_eq!(right.id, "vc1_right");
        assert_eq!(left.in_point, 0.0);
        assert_eq!(left.out_point, 4.0);
        assert_eq!(left.start_time, 0.0);
        assert_eq!(left.duration, 4.0);
        assert_eq!(right.in_point, 4.0);
        assert_eq!(right.out_point, 10.0);
        assert_eq!(right.start_time, 4.0);
        assert_eq!(right.duration, 6.0);
        assert_eq!(left.metadata, clip.metadata);
        assert_eq!(right.metadata, clip.metadata);
    }

    #[test]
    fn test_cut_audio_clip_at_middle() {
        let clip = AudioClip {
            id: "ac1".to_string(),
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
        let playhead = 6.0;
        let (left, right) = cut_clip_at(&clip, playhead).unwrap();
        assert_eq!(left.id, "ac1_left");
        assert_eq!(right.id, "ac1_right");
        assert_eq!(left.in_point, 0.0);
        assert_eq!(left.out_point, 4.0);
        assert_eq!(left.start_time, 2.0);
        assert_eq!(left.duration, 4.0);
        assert_eq!(right.in_point, 4.0);
        assert_eq!(right.out_point, 8.0);
        assert_eq!(right.start_time, 6.0);
        assert_eq!(right.duration, 4.0);
        assert_eq!(left.metadata, clip.metadata);
        assert_eq!(right.metadata, clip.metadata);
    }

    #[test]
    fn test_cut_clip_at_out_of_bounds() {
        let clip = VideoClip {
            id: "vc2".to_string(),
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
        // Playhead before start
        assert!(cut_clip_at(&clip, -1.0).is_none());
        // Playhead at start
        assert!(cut_clip_at(&clip, 0.0).is_none());
        // Playhead at end
        assert!(cut_clip_at(&clip, 10.0).is_none());
        // Playhead after end
        assert!(cut_clip_at(&clip, 12.0).is_none());
    }
}
