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

/// A single palette (tileset texture + metadata) used by a Tilemap.
/// Matches the `.tilepallet` file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TilePalette {
    /// Display name (used in the painter palette tabs)
    #[serde(default)]
    pub name: String,
    pub texture_path: String,
    /// Columns in the tileset texture
    pub tileset_cols: u32,
    /// Rows in the tileset texture
    pub tileset_rows: u32,
    /// Pixel border around the tileset edge
    #[serde(default)]
    pub margin: u32,
    /// Pixel gap between tiles
    #[serde(default)]
    pub spacing: u32,
    /// 0-based tile indices within this palette that are solid (for physics/pathfinding)
    #[serde(default)]
    pub solid_tiles: Vec<u16>,
}

impl TilePalette {
    pub fn tile_count(&self) -> u32 {
        self.tileset_cols * self.tileset_rows
    }
}

/// Encode a (palette_id, tile_idx) pair into a u32 cell value.
/// palette_id is 1-indexed (1 = first palette); tile_idx is 0-indexed within the palette.
/// Returns 0 (empty) if palette_id is 0.
pub fn encode_tile(palette_id: u32, tile_idx: u32) -> u32 {
    (palette_id << 16) | (tile_idx & 0xFFFF)
}

/// Decode a u32 cell value into (palette_id, tile_idx).
/// Returns (0, 0) for empty cells.
pub fn decode_tile(v: u32) -> (u32, u32) {
    (v >> 16, v & 0xFFFF)
}

/// A tile-based map component.
///
/// Each cell stores a u32: upper 16 bits = palette_id (1-indexed, 0 = empty),
/// lower 16 bits = tile_idx (0-indexed within that palette's tileset).
/// Use `encode_tile` / `decode_tile` helpers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tilemap {
    /// Ordered list of tile palettes available to this map.
    /// palette_id 1 in a cell refers to palettes[0], palette_id 2 to palettes[1], etc.
    #[serde(default)]
    pub palettes: Vec<TilePalette>,
    /// Width of each tile in world units
    pub tile_width: f32,
    /// Height of each tile in world units
    pub tile_height: f32,
    /// Map width in tiles
    pub map_cols: u32,
    /// Map height in tiles
    pub map_rows: u32,
    /// Flat tile array (row-major). 0 = empty. Use encode_tile/decode_tile.
    pub tiles: Vec<u32>,
    pub tint: [f32; 4],
}

impl Tilemap {
    /// Returns true if the tile at (col, row) is solid, based on its palette's solid_tiles.
    pub fn is_tile_solid(&self, col: u32, row: u32) -> bool {
        let v = self.tiles.get((row * self.map_cols + col) as usize).copied().unwrap_or(0);
        if v == 0 { return false; }
        let (palette_id, tile_idx) = decode_tile(v);
        if palette_id == 0 { return false; }
        if let Some(pal) = self.palettes.get((palette_id - 1) as usize) {
            pal.solid_tiles.contains(&(tile_idx as u16))
        } else {
            false
        }
    }

    /// Greedy rectangle merge over solid tiles.
    /// Returns a list of (cx, cy, half_w, half_h) in local tilemap space (origin at top-left).
    pub fn solid_rects(&self) -> Vec<(f32, f32, f32, f32)> {
        let cols = self.map_cols as usize;
        let rows = self.map_rows as usize;
        let tw = self.tile_width;
        let th = self.tile_height;

        let solid: Vec<bool> = (0..rows).flat_map(|r| {
            (0..cols).map(move |c| self.is_tile_solid(c as u32, r as u32))
        }).collect();

        let mut consumed = vec![false; cols * rows];
        let mut rects = Vec::new();

        for row in 0..rows {
            let mut col = 0;
            while col < cols {
                let idx = row * cols + col;
                if solid[idx] && !consumed[idx] {
                    let run_start = col;
                    while col < cols && solid[row * cols + col] && !consumed[row * cols + col] {
                        col += 1;
                    }
                    let run_end = col;

                    let mut run_height = 1;
                    'down: loop {
                        let next_row = row + run_height;
                        if next_row >= rows { break; }
                        for c in run_start..run_end {
                            if !solid[next_row * cols + c] || consumed[next_row * cols + c] {
                                break 'down;
                            }
                        }
                        run_height += 1;
                    }

                    for r in row..row + run_height {
                        for c in run_start..run_end {
                            consumed[r * cols + c] = true;
                        }
                    }

                    let hw = (run_end - run_start) as f32 * tw * 0.5;
                    let hh = run_height as f32 * th * 0.5;
                    let cx = run_start as f32 * tw + hw;
                    let cy = row as f32 * th + hh;
                    rects.push((cx, cy, hw, hh));
                } else {
                    col += 1;
                }
            }
        }

        rects
    }
}

impl Default for Tilemap {
    fn default() -> Self {
        let map_cols = 20u32;
        let map_rows = 10u32;
        Self {
            palettes: Vec::new(),
            tile_width: 32.0,
            tile_height: 32.0,
            map_cols,
            map_rows,
            tiles: vec![0u32; (map_cols * map_rows) as usize],
            tint: [1.0, 1.0, 1.0, 1.0],
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
