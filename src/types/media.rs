use serde::{Deserialize, Serialize};

pub trait Clip {
    fn id(&self) -> &str;
    fn asset_path(&self) -> &str;
    fn in_point(&self) -> f64;
    fn out_point(&self) -> f64;
    fn start_time(&self) -> f64;
    fn duration(&self) -> f64;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub resolution: (u32, u32),
    pub frame_rate: f64,
    pub codec: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoClip {
    pub id: String,
    pub asset_path: String,
    pub in_point: f64,
    pub out_point: f64,
    pub start_time: f64,
    pub duration: f64,
    pub metadata: VideoMetadata,
}

impl Clip for VideoClip {
    fn id(&self) -> &str {
        &self.id
    }

    fn asset_path(&self) -> &str {
        &self.asset_path
    }

    fn in_point(&self) -> f64 {
        self.in_point
    }

    fn out_point(&self) -> f64 {
        self.out_point
    }

    fn start_time(&self) -> f64 {
        self.start_time
    }

    fn duration(&self) -> f64 {
        self.duration
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioClip {
    pub id: String,
    pub asset_path: String,
    pub in_point: f64,
    pub out_point: f64,
    pub start_time: f64,
    pub duration: f64,
    pub metadata: AudioMetadata,
}

impl Clip for AudioClip {
    fn id(&self) -> &str {
        &self.id
    }

    fn asset_path(&self) -> &str {
        &self.asset_path
    }

    fn in_point(&self) -> f64 {
        self.in_point
    }

    fn out_point(&self) -> f64 {
        self.out_point
    }

    fn start_time(&self) -> f64 {
        self.start_time
    }

    fn duration(&self) -> f64 {
        self.duration
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub sample_rate: u32,
    pub channels: u32,
    pub codec: String,
    pub bitrate: u32,
}
