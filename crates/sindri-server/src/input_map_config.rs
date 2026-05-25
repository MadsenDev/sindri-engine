use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

fn default_deadzone() -> f32 {
    0.2
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct InputMapConfig {
    pub actions: Vec<ActionEntry>,
}

impl InputMapConfig {
    pub fn load(project_dir: &std::path::Path) -> Self {
        let path = project_dir.join("input_map.json");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, project_dir: &std::path::Path) -> anyhow::Result<()> {
        let path = project_dir.join("input_map.json");
        let s = serde_json::to_string_pretty(self)?;
        std::fs::write(path, s)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ActionEntry {
    pub name: String,
    pub bindings: Vec<BindingEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BindingEntry {
    Key {
        key: String,
    },
    KeyAxis {
        negative: String,
        positive: String,
    },
    GamepadButton {
        button: String,
    },
    GamepadAxis {
        axis: String,
        #[serde(default = "default_deadzone")]
        deadzone: f32,
    },
}

/// Live gamepad state updated by a background polling thread.
#[derive(Default, Clone)]
pub struct ControllerState {
    /// Buttons currently held (e.g. "South", "East", "LeftTrigger")
    pub buttons_held: HashMap<String, bool>,
    /// Buttons pressed this frame
    pub buttons_pressed: HashMap<String, bool>,
    /// Analog axis values [-1, 1] (e.g. "LeftStickX", "LeftStickY")
    pub axes: HashMap<String, f32>,
}

pub type SharedControllerState = Arc<RwLock<ControllerState>>;
