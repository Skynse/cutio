use crate::types::media_library::MediaLibrary;
use crate::types::timeline::Timeline;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Write};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub description: Option<String>,
    pub project_file_path: String,
    pub created_at: String, // Could use chrono::DateTime for real timestamps
    pub last_modified: String,
    pub media_library: MediaLibrary,
    pub timeline: Timeline,
    pub cache_dir: String,
    pub render_output_dir: String,
    pub settings: ProjectSettings,
}

impl Project {
    /// Save the project to a JSON file at the given path.
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())
    }

    /// Load a project from a JSON file at the given path.
    pub fn load_from_file(path: &str) -> std::io::Result<Project> {
        let mut file = File::open(path)?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;
        let project: Project = serde_json::from_str(&json).unwrap();
        Ok(project)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub resolution: (u32, u32),
    pub frame_rate: f64,
    pub color_space: String,
    // Add more as needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_project() {
        let project = Project {
            name: "Test Project".to_string(),
            description: Some("A test project".to_string()),
            project_file_path: "/tmp/test_project.json".to_string(),
            created_at: "2024-06-09T12:00:00Z".to_string(),
            last_modified: "2024-06-09T12:00:00Z".to_string(),
            media_library: MediaLibrary::new(),
            timeline: Timeline::new(),
            cache_dir: "/tmp/cache".to_string(),
            render_output_dir: "/tmp/render".to_string(),
            settings: ProjectSettings {
                resolution: (1920, 1080),
                frame_rate: 30.0,
                color_space: "sRGB".to_string(),
            },
        };
        let path = "/tmp/test_project.json";
        project.save_to_file(path).unwrap();
        let loaded = Project::load_from_file(path).unwrap();
        assert_eq!(project.name, loaded.name);
        assert_eq!(project.settings.resolution, loaded.settings.resolution);
        let _ = std::fs::remove_file(path);
    }
}

impl Project {
    pub fn new(
        name: String,
        project_file_path: String,
        cache_dir: String,
        render_output_dir: String,
        settings: ProjectSettings,
    ) -> Self {
        let now = "2024-06-09T12:00:00Z".to_string(); // Placeholder, use chrono for real
        Project {
            name,
            description: None,
            project_file_path,
            created_at: now.clone(),
            last_modified: now,
            media_library: MediaLibrary::new(),
            timeline: Timeline::new(),
            cache_dir,
            render_output_dir,
            settings,
        }
    }
}
