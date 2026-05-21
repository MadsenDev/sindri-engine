use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    #[serde(default = "default_project_name")]
    pub name: String,
    #[serde(default = "default_resolution_width")]
    pub resolution_width: u32,
    #[serde(default = "default_resolution_height")]
    pub resolution_height: u32,
    #[serde(default = "default_pixel_art_mode")]
    pub pixel_art_mode: bool,
    #[serde(default = "default_gravity_x")]
    pub gravity_x: f32,
    #[serde(default = "default_gravity_y")]
    pub gravity_y: f32,
}

fn default_project_name() -> String { "My Game".into() }
fn default_resolution_width() -> u32 { 1280 }
fn default_resolution_height() -> u32 { 720 }
fn default_pixel_art_mode() -> bool { true }
fn default_gravity_x() -> f32 { 0.0 }
fn default_gravity_y() -> f32 { 980.0 }

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            name: default_project_name(),
            resolution_width: default_resolution_width(),
            resolution_height: default_resolution_height(),
            pixel_art_mode: default_pixel_art_mode(),
            gravity_x: default_gravity_x(),
            gravity_y: default_gravity_y(),
        }
    }
}

impl ProjectSettings {
    pub fn load(project_dir: &Path) -> Self {
        let path = project_dir.join("sindri_project.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, project_dir: &Path) -> anyhow::Result<()> {
        let path = project_dir.join("sindri_project.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
