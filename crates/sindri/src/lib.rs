pub mod assets;
pub mod audio;
pub mod builder;
pub mod camera;
pub mod commands;
pub mod component;
pub mod component_metadata;
pub mod engine;
pub mod entities;
pub mod entity;
pub mod fonts;
pub mod grid;
pub mod hierarchy;
pub mod hud;
pub mod input;
pub mod math;
pub mod pathfinding;
pub mod physics;
pub mod render;
pub mod scene;
pub mod scene_physics;
pub mod screenshot;
pub mod script;
pub mod state;
pub mod world;

pub use crate::assets::AssetManager;
pub use crate::audio::{AudioSystem, SoundHandle};
pub use crate::builder::EntityBuilder;
pub use crate::camera::{active_camera, update_camera_follow, CameraFollow};
pub use crate::commands::{
    AddComponent, Command, CommandHistory, CreateEntity, DeleteEntity, RemoveComponent,
    ReparentEntity, SetTransform,
};
pub use crate::component_metadata::{
    register_builtin_metadata, ComponentMetadataHandler, ComponentMetadataRegistry,
    FieldDescriptor, TransformMetadataHandler,
};
pub use crate::engine::{Engine, EngineConfig, EngineContext, Game};
pub use crate::entities::{
    AudioSource, CameraComponent, Checkpoint, Collectible, Enemy, Hazard, MovingPlatform, Name,
    PhysicsBody, Player, SpriteComponent, TilemapComponent, Transform, Trigger,
};
pub use crate::fonts::BuiltinFont;
pub use crate::grid::{Grid, GridCoord, GridPathfinding};
pub use crate::hierarchy::{
    get_children, get_parent, get_root, get_world_position, get_world_rotation, get_world_scale,
    reparent, set_parent,
};
pub use crate::hud::{HudLayer, HudLayout, HudPanel, HudRect, HudSprite, HudText, TextAlign};
pub use crate::input::{ActionId, AxisBinding, Button, InputMap, InputState};
pub use crate::math::{Camera2D, Transform2D, Vec2};
pub use crate::pathfinding::{AStarPathfinder, GridNode, PathfindingGrid};
pub use crate::physics::{PhysicsEventCallback, PhysicsWorld};
pub use crate::render::{
    AnimatedSprite, Animation, AnimationFrame, DirectionalLight, EmissionConfig, FontHandle, Frame,
    Particle, ParticleEmitter, ParticleSystem, PointLight, Renderer, Sprite, TextureHandle, Tile,
    Tilemap,
};
pub use crate::scene::Scene;
pub use crate::scene_physics::{
    create_scene, restore_scene_physics, restore_scene_physics_preserve, ComponentSerializable,
    SerializableComponent, SerializablePhysics,
};
pub use crate::script::{
    AnimationFacet, InputFacet, PhysicsFacet, ScriptComponent, ScriptParams, ScriptRuntime,
    ScriptSelf, ScriptTag, ScriptValue, SpriteFacet, TilemapFacet, TimeFacet, TransformFacet,
    WorldFacet,
};
pub use crate::state::{State, StateMachine, StateMachineLike};
pub use crate::world::{EntityId, World};
pub use rapier2d::prelude::RigidBodyHandle;
pub use rapier2d::prelude::{ImpulseJointHandle, ImpulseJointSet, RigidBodyType};
pub use winit::{event::MouseButton, keyboard::KeyCode};
