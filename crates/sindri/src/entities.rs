//! Built-in entity components for common game objects.
//!
//! These components can be attached to entities to create standard game objects
//! like sprites, physics bodies, audio sources, etc.

use crate::math::{Transform2D, Vec2};
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
