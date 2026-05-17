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
    pub zoom: f32,
    pub follow_entity: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSource {
    pub path: String,
    pub volume: f32,
    pub looping: bool,
    pub play_on_start: bool,
}
