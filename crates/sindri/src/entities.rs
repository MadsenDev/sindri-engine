//! Built-in entity components for common game objects.
//!
//! These components can be attached to entities to create standard game objects
//! like sprites, physics bodies, audio sources, etc.

use crate::math::{Transform2D, Vec2};
use crate::pathfinding::{PathfindingGrid, PathfindingMode, PlatformGraph};
use crate::physics::{ColliderShape, RigidBodyType};
use crate::render::{Sprite, TextureHandle, Tilemap};

/// Human-readable entity name for editor/debug tooling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name(pub String);

impl Name {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Transform component - position, rotation, and scale.
/// This is the core component that most entities should have.
#[derive(Clone, Debug)]
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
    /// Parent entity (for hierarchy). None means this is a root entity.
    pub parent: Option<crate::world::EntityId>,
}

impl Transform {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
            parent: None,
        }
    }

    /// Set the parent entity (for hierarchy).
    pub fn with_parent(mut self, parent: crate::world::EntityId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }
}

impl From<Transform2D> for Transform {
    fn from(t: Transform2D) -> Self {
        Self {
            position: t.position,
            rotation: t.rotation,
            scale: t.scale,
            parent: None,
        }
    }
}

impl From<Transform> for Transform2D {
    fn from(t: Transform) -> Self {
        Transform2D {
            position: t.position,
            rotation: t.rotation,
            scale: t.scale,
        }
    }
}

/// Sprite component - visual representation of an entity.
#[derive(Clone, Debug)]
pub struct SpriteComponent {
    pub texture: TextureHandle,
    pub sprite: Sprite,
    pub visible: bool,
}

impl SpriteComponent {
    pub fn new(texture: TextureHandle) -> Self {
        Self {
            texture,
            sprite: Sprite::new(texture),
            visible: true,
        }
    }

    pub fn with_tint(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.sprite.tint = [r, g, b, a];
        self
    }
}

/// Physics body component - marks an entity as having a physics body.
/// The actual physics body is managed by PhysicsWorld, but this component
/// tracks which entities have physics.
#[derive(Clone, Copy, Debug)]
pub struct PhysicsBody {
    pub body_type: RigidBodyType,
    pub collider_shape: Option<ColliderShape>,
}

impl PhysicsBody {
    pub fn new(body_type: RigidBodyType) -> Self {
        Self {
            body_type,
            collider_shape: None,
        }
    }

    pub fn with_collider(mut self, shape: ColliderShape) -> Self {
        self.collider_shape = Some(shape);
        self
    }
}

/// Audio source component - for positional audio.
#[derive(Clone, Debug)]
pub struct AudioSource {
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub sound_id: Option<u32>, // Reference to loaded sound
}

impl AudioSource {
    pub fn new() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            looping: false,
            sound_id: None,
        }
    }

    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn with_pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch.max(0.0);
        self
    }

    pub fn with_looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }
}

impl Default for AudioSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Camera component - attaches a camera to an entity.
#[derive(Clone, Debug)]
pub struct CameraComponent {
    pub camera: crate::math::Camera2D,
    pub active: bool,
}

impl CameraComponent {
    pub fn new(position: Vec2) -> Self {
        Self {
            camera: crate::math::Camera2D::new(position),
            active: true,
        }
    }

    pub fn with_zoom(mut self, zoom: f32) -> Self {
        self.camera.zoom = zoom;
        self
    }
}

/// Tag components for marking entities with specific behaviors

/// Marks an entity as the player.
#[derive(Clone, Copy, Debug, Default)]
pub struct Player;

/// Marks an entity as an enemy.
#[derive(Clone, Copy, Debug, Default)]
pub struct Enemy;

/// Marks an entity as a collectible item.
#[derive(Clone, Copy, Debug, Default)]
pub struct Collectible {
    pub value: i32,
}

impl Collectible {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
}

/// Marks an entity as a hazard (damages player on contact).
#[derive(Clone, Copy, Debug, Default)]
pub struct Hazard {
    pub damage: i32,
}

impl Hazard {
    pub fn new(damage: i32) -> Self {
        Self { damage }
    }
}

/// Marks an entity as a checkpoint.
#[derive(Clone, Copy, Debug, Default)]
pub struct Checkpoint {
    pub checkpoint_id: u32,
}

impl Checkpoint {
    pub fn new(id: u32) -> Self {
        Self { checkpoint_id: id }
    }
}

/// Marks an entity as a trigger zone (activates something when entered).
#[derive(Clone, Copy, Debug)]
pub struct Trigger {
    pub trigger_id: u32,
    pub activated: bool,
}

impl Trigger {
    pub fn new(id: u32) -> Self {
        Self {
            trigger_id: id,
            activated: false,
        }
    }
}

/// Marks an entity as a moving platform.
#[derive(Clone, Debug)]
pub struct MovingPlatform {
    pub start_pos: Vec2,
    pub end_pos: Vec2,
    pub speed: f32,
    pub current_t: f32, // 0.0 to 1.0
    pub direction: f32, // 1.0 or -1.0
}

impl MovingPlatform {
    pub fn new(start_pos: Vec2, end_pos: Vec2, speed: f32) -> Self {
        Self {
            start_pos,
            end_pos,
            speed,
            current_t: 0.0,
            direction: 1.0,
        }
    }
}

/// Tilemap component - renders a tile-based map.
#[derive(Clone, Debug)]
pub struct TilemapComponent {
    pub tilemap: Tilemap,
}

impl TilemapComponent {
    pub fn new(tilemap: Tilemap) -> Self {
        Self { tilemap }
    }
}

/// Navigation grid for pathfinding.
///
/// Attach this to any entity to make it a pathfinding authority.  Scripts
/// query it via `self:nav_grid()` or `world:find_path(nav_entity, start, goal)`.
#[derive(Clone, Debug)]
pub struct NavGridComponent {
    pub grid: PathfindingGrid,
    pub mode: PathfindingMode,
    /// Pre-built platform graph (only `Some` when `mode == Platformer`).
    pub platform_graph: Option<PlatformGraph>,
    /// When true the platform graph is stale and needs `rebuild_platform_graph()`.
    pub graph_dirty: bool,
}

impl NavGridComponent {
    /// Create a top-down 8-directional nav grid covering `width × height` cells.
    pub fn new_topdown(width: usize, height: usize, cell_size: f32) -> Self {
        Self {
            grid: PathfindingGrid::new(width, height, cell_size),
            mode: PathfindingMode::TopDown8,
            platform_graph: None,
            graph_dirty: false,
        }
    }

    /// Create a top-down 4-directional nav grid.
    pub fn new_topdown4(width: usize, height: usize, cell_size: f32) -> Self {
        Self {
            grid: PathfindingGrid::new(width, height, cell_size),
            mode: PathfindingMode::TopDown4,
            platform_graph: None,
            graph_dirty: false,
        }
    }

    /// Create a platformer nav grid.
    pub fn new_platformer(
        width: usize,
        height: usize,
        cell_size: f32,
        gravity: f32,
        jump_velocity: f32,
        move_speed: f32,
    ) -> Self {
        let mut comp = Self {
            grid: PathfindingGrid::new(width, height, cell_size),
            mode: PathfindingMode::Platformer { gravity, jump_velocity, move_speed },
            platform_graph: None,
            graph_dirty: true,
        };
        comp.rebuild_platform_graph();
        comp
    }

    /// (Re)build the platform graph from the current grid and mode parameters.
    /// No-op for non-platformer modes.
    pub fn rebuild_platform_graph(&mut self) {
        if let PathfindingMode::Platformer { gravity, jump_velocity, move_speed } = self.mode {
            self.platform_graph =
                Some(PlatformGraph::build(&self.grid, gravity, jump_velocity, move_speed));
            self.graph_dirty = false;
        }
    }

    /// Mark the platform graph as needing a rebuild on next path query.
    pub fn mark_dirty(&mut self) {
        if matches!(self.mode, PathfindingMode::Platformer { .. }) {
            self.graph_dirty = true;
        }
    }

    /// Build walkability from a `TilemapComponent`, blocking any tile whose
    /// palette-local index is listed in `blocked_tile_indices`.
    ///
    /// The nav grid is resized to match the tilemap dimensions and aligned
    /// to its world-space origin.
    pub fn build_from_tilemap(
        &mut self,
        tilemap: &crate::render::Tilemap,
        blocked_tile_indices: &[u32],
    ) {
        let (cols, rows) = tilemap.map_size;
        let cell_w = tilemap.tile_size.x;
        let cell_h = tilemap.tile_size.y;
        let cell_size = cell_w.min(cell_h);

        self.grid = PathfindingGrid::with_origin(
            cols as usize,
            rows as usize,
            cell_size,
            tilemap.position,
        );

        for y in 0..rows {
            for x in 0..cols {
                if let Some(tile) = tilemap.get_tile(x, y) {
                    if !tile.is_empty() && blocked_tile_indices.contains(&(tile.id - 1)) {
                        self.grid.set_walkable(
                            crate::pathfinding::GridNode::new(x as i32, y as i32),
                            false,
                        );
                    }
                }
            }
        }
        self.mark_dirty();
    }

    /// Build walkability from physics colliders: cells overlapped by a static
    /// collider are marked blocked.
    pub fn build_from_physics(&mut self, physics: &crate::physics::PhysicsWorld) {
        let cs = self.grid.cell_size();
        let origin = self.grid.origin();

        for y in 0..self.grid.height() as i32 {
            for x in 0..self.grid.width() as i32 {
                let cx = origin.x + (x as f32 + 0.5) * cs;
                let cy = origin.y + (y as f32 + 0.5) * cs;
                let blocked = physics.point_query(Vec2::new(cx, cy)).is_some();
                self.grid.set_walkable(
                    crate::pathfinding::GridNode::new(x, y),
                    !blocked,
                );
            }
        }
        self.mark_dirty();
    }
}
