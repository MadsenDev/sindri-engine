use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Component {
    Transform(Transform),
    Sprite(Sprite),
    AnimatedSprite(AnimatedSprite),
    Tilemap(Tilemap),
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

/// A named animation clip: a contiguous range of frames on the spritesheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimClip {
    /// Identifier used from Lua: `anim:play("walk")`
    pub name: String,
    /// First frame index (0-based, row-major order on the spritesheet)
    pub start_frame: u32,
    /// Last frame index (inclusive)
    pub end_frame: u32,
    /// Frames per second
    pub fps: f32,
    pub looping: bool,
}

impl AnimClip {
    pub fn frame_duration(&self) -> f32 {
        if self.fps > 0.0 { 1.0 / self.fps } else { 0.1 }
    }
    pub fn frame_count(&self) -> u32 {
        self.end_frame.saturating_sub(self.start_frame) + 1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimatedSprite {
    pub texture_path: String,
    /// Number of columns in the spritesheet grid
    pub cols: u32,
    /// Number of rows in the spritesheet grid
    pub rows: u32,
    /// Display width in world units
    pub width: f32,
    /// Display height in world units
    pub height: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub tint: [f32; 4],
    /// Named animation clips referencing frame ranges on the spritesheet
    pub clips: Vec<AnimClip>,
    /// Name of the clip to play on start
    pub default_clip: String,
    /// Pixel border around the edge of the spritesheet texture
    #[serde(default)]
    pub margin: u32,
    /// Pixel gap between frames in the spritesheet texture
    #[serde(default)]
    pub spacing: u32,
}

impl Default for AnimatedSprite {
    fn default() -> Self {
        Self {
            texture_path: String::new(),
            cols: 4,
            rows: 1,
            width: 64.0,
            height: 64.0,
            flip_x: false,
            flip_y: false,
            tint: [1.0, 1.0, 1.0, 1.0],
            clips: vec![AnimClip {
                name: "idle".to_string(),
                start_frame: 0,
                end_frame: 3,
                fps: 10.0,
                looping: true,
            }],
            default_clip: "idle".to_string(),
            margin: 0,
            spacing: 0,
        }
    }
}

/// A tile-based map component. Tiles reference a tileset texture by 1-based index (0 = empty).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tilemap {
    pub texture_path: String,
    /// Columns of tiles in the tileset texture
    pub tileset_cols: u32,
    /// Rows of tiles in the tileset texture
    pub tileset_rows: u32,
    /// Width of each tile in world units
    pub tile_width: f32,
    /// Height of each tile in world units
    pub tile_height: f32,
    /// Map width in tiles
    pub map_cols: u32,
    /// Map height in tiles
    pub map_rows: u32,
    /// Flat tile array (row-major). 0 = empty, 1-based index into tileset.
    pub tiles: Vec<u16>,
    pub tint: [f32; 4],
    /// Pixel border around the edge of the tileset texture
    #[serde(default)]
    pub margin: u32,
    /// Pixel gap between tiles in the tileset texture
    #[serde(default)]
    pub spacing: u32,
}

impl Default for Tilemap {
    fn default() -> Self {
        let map_cols = 20u32;
        let map_rows = 10u32;
        Self {
            texture_path: String::new(),
            tileset_cols: 8,
            tileset_rows: 8,
            tile_width: 32.0,
            tile_height: 32.0,
            map_cols,
            map_rows,
            tiles: vec![0; (map_cols * map_rows) as usize],
            tint: [1.0, 1.0, 1.0, 1.0],
            margin: 0,
            spacing: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSource {
    pub path: String,
    pub volume: f32,
    pub looping: bool,
    pub play_on_start: bool,
}
