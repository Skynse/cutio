use crate::types::media::{AudioClip, Clip, VideoClip};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Track {
    Video(VideoTrack),
    Audio(AudioTrack),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTrack {
    pub id: String,
    pub name: String,
    pub clips: Vec<VideoClip>,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    pub id: String,
    pub name: String,
    pub clips: Vec<AudioClip>,
    pub muted: bool,
}

enum TrackType {
    Video,
    Audio,
}
