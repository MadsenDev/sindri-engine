use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Component {
    Transform(Transform),
    Sprite(Sprite),
    PhysicsBody(PhysicsBody),
    Collider(Collider),
    Script(Script),
    Camera(Camera),
    AudioSource(AudioSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
    pub texture_path: String,
    pub width: f32,
    pub height: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsBody {
    pub body_type: BodyType,
    pub lock_rotation: bool,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub collision_layer: u8,
    pub collision_mask: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BodyType {
    Dynamic,
    Kinematic,
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collider {
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub is_trigger: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    #[serde(default = "default_camera_active")]
    pub active: bool,
    #[serde(default = "default_camera_zoom")]
    pub zoom: f32,
    #[serde(default)]
    pub follow_entity: Option<u64>,
    #[serde(default)]
    pub offset_x: f32,
    #[serde(default)]
    pub offset_y: f32,
    #[serde(default)]
    pub bounds_min_x: Option<f32>,
    #[serde(default)]
    pub bounds_min_y: Option<f32>,
    #[serde(default)]
    pub bounds_max_x: Option<f32>,
    #[serde(default)]
    pub bounds_max_y: Option<f32>,
    #[serde(default = "default_camera_smoothing")]
    pub smoothing: f32,
    #[serde(default)]
    pub dead_zone_width: f32,
    #[serde(default)]
    pub dead_zone_height: f32,
    #[serde(skip)]
    pub runtime_target_zoom: Option<f32>,
    #[serde(skip)]
    pub runtime_zoom_speed: f32,
    #[serde(skip)]
    pub runtime_shake_intensity: f32,
    #[serde(skip)]
    pub runtime_shake_timer: f32,
    #[serde(skip)]
    pub runtime_shake_seed: f32,
}

fn default_camera_active() -> bool {
    true
}

fn default_camera_zoom() -> f32 {
    1.0
}

fn default_camera_smoothing() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSource {
    pub path: String,
    pub volume: f32,
    pub looping: bool,
    pub play_on_start: bool,
}
