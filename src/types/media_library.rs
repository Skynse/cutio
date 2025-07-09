use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaLibrary {
    items: Vec<MediaItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaItem {
    AudioItem(AudioProp),
    VideoItem(VideoProp),
    // ImageProp(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioProp {
    pub file_descriptor: FileDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProp {
    pub file_descriptor: FileDescriptor,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDescriptor {
    pub file_name: String,
    pub path: String,
    pub size: u64,
    pub mime_type: String,
}

impl FileDescriptor {
    pub fn new(file_name: String, path: String, size: u64, mime_type: String) -> Self {
        FileDescriptor {
            file_name,
            path,
            size,
            mime_type,
        }
    }
}

impl MediaLibrary {
    pub fn new() -> Self {
        MediaLibrary { items: Vec::new() }
    }

    pub fn add_audio(&mut self, prop: AudioProp) {
        self.items.push(MediaItem::AudioItem(prop));
    }

    pub fn add_video(&mut self, prop: VideoProp) {
        self.items.push(MediaItem::VideoItem(prop));
    }

    pub fn all_items(&self) -> &Vec<MediaItem> {
        &self.items
    }

    /// Add a file (audio or video) to the media library, inferring type from extension.
    pub fn add_file(&mut self, path: &std::path::Path) {
        use std::fs;
        use std::process::Command;
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let path_str = path.to_string_lossy().to_string();
        let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let mime_type = match ext.as_str() {
            "mp3" | "wav" | "ogg" | "flac" => "audio".to_string(),
            "mp4" | "mov" | "mkv" | "webm" | "avi" => "video".to_string(),
            _ => "unknown".to_string(),
        };

        let fd = FileDescriptor::new(file_name, path_str.clone(), size, mime_type.clone());
        if mime_type == "audio" {
            self.add_audio(AudioProp {
                file_descriptor: fd,
            });
        } else if mime_type == "video" {
            // Extract thumbnail using GStreamer
            let thumbnail_path = {
                let thumb_path = format!("{}.thumb.jpg", path_str);
                let gst_status = {
                    use gst::prelude::*;
                    use gstreamer as gst;
                    let _ = gst::init(); // Safe to call multiple times

                    let pipeline_str = format!(
                        "filesrc location=\"{}\" ! decodebin ! videoconvert ! videoscale ! video/x-raw,format=RGB ! jpegenc ! multifilesink location=\"{}\" next-file=key-frame",
                        path_str, thumb_path
                    );
                    let pipeline = match gst::parse::launch(&pipeline_str) {
                        Ok(p) => p,
                        Err(_) => return,
                    };
                    let pipeline = pipeline
                        .downcast::<gst::Pipeline>()
                        .expect("Expected a gst::Pipeline");

                    pipeline.set_state(gst::State::Paused).ok();
                    pipeline
                        .seek_simple(
                            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                            gst::ClockTime::from_seconds(1),
                        )
                        .ok();
                    pipeline.set_state(gst::State::Playing).ok();

                    let bus = pipeline.bus().unwrap();
                    let mut success = false;
                    for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
                        use gst::MessageView;
                        match msg.view() {
                            MessageView::Eos(..) => {
                                success = true;
                                break;
                            }
                            MessageView::Error(_) => break,
                            _ => (),
                        }
                    }
                    pipeline.set_state(gst::State::Null).ok();
                    success
                };
                if gst_status && std::path::Path::new(&thumb_path).exists() {
                    Some(thumb_path)
                } else {
                    None
                }
            };
            self.add_video(VideoProp {
                file_descriptor: fd,
                thumbnail_path,
            });
        }
        // Ignore unknown types for now
    }

    pub fn find_by_filename(&self, name: &str) -> Option<&MediaItem> {
        self.items.iter().find(|item| match item {
            MediaItem::AudioItem(a) => a.file_descriptor.file_name == name,
            MediaItem::VideoItem(v) => v.file_descriptor.file_name == name,
        })
    }

    pub fn remove_by_filename(&mut self, name: &str) -> Option<MediaItem> {
        let idx = self.items.iter().position(|item| match item {
            MediaItem::AudioItem(a) => a.file_descriptor.file_name == name,
            MediaItem::VideoItem(v) => v.file_descriptor.file_name == name,
        })?;
        Some(self.items.remove(idx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_find_audio() {
        let fd = FileDescriptor::new(
            "song.wav".to_string(),
            "/audio/song.wav".to_string(),
            1024,
            "audio/wav".to_string(),
        );
        let audio = AudioProp {
            file_descriptor: fd.clone(),
        };
        let mut lib = MediaLibrary::new();
        lib.add_audio(audio);

        let found = lib.find_by_filename("song.wav");
        assert!(matches!(found, Some(MediaItem::AudioItem(_))));
        if let Some(MediaItem::AudioItem(a)) = found {
            assert_eq!(a.file_descriptor.file_name, "song.wav");
            assert_eq!(a.file_descriptor.path, "/audio/song.wav");
        }
    }

    #[test]
    fn test_add_and_find_video() {
        let fd = FileDescriptor::new(
            "movie.mp4".to_string(),
            "/video/movie.mp4".to_string(),
            2048,
            "video/mp4".to_string(),
        );
        let video = VideoProp {
            file_descriptor: fd.clone(),
            thumbnail_path: None,
        };
        let mut lib = MediaLibrary::new();
        lib.add_video(video);

        let found = lib.find_by_filename("movie.mp4");
        assert!(matches!(found, Some(MediaItem::VideoItem(_))));
        if let Some(MediaItem::VideoItem(v)) = found {
            assert_eq!(v.file_descriptor.file_name, "movie.mp4");
            assert_eq!(v.file_descriptor.path, "/video/movie.mp4");
        }
    }

    #[test]
    fn test_remove_by_filename() {
        let fd_audio = FileDescriptor::new(
            "song.wav".to_string(),
            "/audio/song.wav".to_string(),
            1024,
            "audio/wav".to_string(),
        );
        let fd_video = FileDescriptor::new(
            "movie.mp4".to_string(),
            "/video/movie.mp4".to_string(),
            2048,
            "video/mp4".to_string(),
        );
        let audio = AudioProp {
            file_descriptor: fd_audio.clone(),
        };
        let video = VideoProp {
            file_descriptor: fd_video.clone(),
            thumbnail_path: None,
        };
        let mut lib = MediaLibrary::new();
        lib.add_audio(audio);
        lib.add_video(video);

        let removed = lib.remove_by_filename("song.wav");
        assert!(matches!(removed, Some(MediaItem::AudioItem(_))));
        assert!(lib.find_by_filename("song.wav").is_none());
        assert!(lib.find_by_filename("movie.mp4").is_some());
    }

    #[test]
    fn test_all_items() {
        let fd_audio = FileDescriptor::new(
            "song.wav".to_string(),
            "/audio/song.wav".to_string(),
            1024,
            "audio/wav".to_string(),
        );
        let fd_video = FileDescriptor::new(
            "movie.mp4".to_string(),
            "/video/movie.mp4".to_string(),
            2048,
            "video/mp4".to_string(),
        );
        let audio = AudioProp {
            file_descriptor: fd_audio.clone(),
        };
        let video = VideoProp {
            file_descriptor: fd_video.clone(),
            thumbnail_path: None,
        };
        let mut lib = MediaLibrary::new();
        lib.add_audio(audio);
        lib.add_video(video);

        let items = lib.all_items();
        assert_eq!(items.len(), 2);
    }
}
