use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use anyhow::{anyhow, Result};
use mlua::{Lua, UserData, UserDataMethods, Value};

use crate::entities::{CameraComponent, SpriteComponent, Transform};
use crate::render::AnimatedSprite;
use crate::input::InputState;
use crate::math::Vec2;
use crate::physics::{ColliderShape, PhysicsEvent, PhysicsWorld, RigidBodyType};
use crate::world::{EntityId, World};

// Implement Lua conversion for Vec2
impl<'lua> mlua::FromLua<'lua> for Vec2 {
    fn from_lua(lua_value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        match lua_value {
            mlua::Value::Table(t) => {
                // Try named fields first (x, y), then fall back to indexed (1, 2)
                let x: f64 = t.get("x").or_else(|_| t.get(1))?;
                let y: f64 = t.get("y").or_else(|_| t.get(2))?;
                Ok(Vec2::new(x as f32, y as f32))
            }
            _ => Err(mlua::Error::FromLuaConversionError {
                from: lua_value.type_name(),
                to: "Vec2",
                message: Some("Expected table with x and y fields".to_string()),
            }),
        }
    }
}

impl<'lua> mlua::IntoLua<'lua> for Vec2 {
    fn into_lua(self, lua: &'lua mlua::Lua) -> mlua::Result<mlua::Value<'lua>> {
        let table = lua.create_table()?;
        table.set("x", self.x)?;
        table.set("y", self.y)?;
        Ok(mlua::Value::Table(table))
    }
}

/// Simple configuration value that can be passed from Rust into a script.
#[derive(Clone, Debug)]
pub enum ScriptValue {
    Number(f32),
    Bool(bool),
    Text(String),
    Vec2(Vec2),
}

impl From<f32> for ScriptValue {
    fn from(value: f32) -> Self {
        Self::Number(value)
    }
}

impl From<bool> for ScriptValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<&str> for ScriptValue {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for ScriptValue {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<Vec2> for ScriptValue {
    fn from(value: Vec2) -> Self {
        Self::Vec2(value)
    }
}

/// Arbitrary parameters that can be consumed by a script on startup.
#[derive(Clone, Debug, Default)]
pub struct ScriptParams {
    values: HashMap<String, ScriptValue>,
}

impl ScriptParams {
    /// Insert a configurable parameter for the script.
    pub fn insert(mut self, key: impl Into<String>, value: impl Into<ScriptValue>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }
}

/// The script component stored on entities. Contains an ordered list of script attachments.
#[derive(Clone, Debug, Default)]
pub struct ScriptComponent {
    pub scripts: Vec<ScriptAttachment>,
}

impl ScriptComponent {
    /// Attach a script module (file path or asset identifier) with optional parameters.
    pub fn with_script(mut self, path: impl Into<String>, params: ScriptParams) -> Self {
        self.scripts.push(ScriptAttachment {
            path: path.into(),
            params,
        });
        self
    }
}

/// Single script entry in a ScriptComponent.
#[derive(Clone, Debug)]
pub struct ScriptAttachment {
    pub path: String,
    pub params: ScriptParams,
}

struct ScriptModule {
    source: String,
    modified: Option<SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ScriptInstanceKey {
    entity: EntityId,
    slot: u32,
}

struct ScriptInstance {
    key: ScriptInstanceKey,
    script_path: String,
    has_started: bool,
    last_loaded: Option<SystemTime>,
    env: Option<mlua::RegistryKey>,
    fns: ScriptFns,
}

#[derive(Default)]
struct ScriptFns {
    on_create: Option<mlua::RegistryKey>,
    on_start: Option<mlua::RegistryKey>,
    on_update: Option<mlua::RegistryKey>,
    on_fixed_update: Option<mlua::RegistryKey>,
    on_post_physics: Option<mlua::RegistryKey>,
    on_draw: Option<mlua::RegistryKey>,
    on_destroy: Option<mlua::RegistryKey>,
    on_collision_enter: Option<mlua::RegistryKey>,
    on_collision_exit: Option<mlua::RegistryKey>,
    on_trigger_enter: Option<mlua::RegistryKey>,
    on_trigger_exit: Option<mlua::RegistryKey>,
}

impl ScriptFns {
    fn clear(&mut self, lua: &Lua) {
        let keys = [
            self.on_create.take(),
            self.on_start.take(),
            self.on_update.take(),
            self.on_fixed_update.take(),
            self.on_post_physics.take(),
            self.on_draw.take(),
            self.on_destroy.take(),
            self.on_collision_enter.take(),
            self.on_collision_exit.take(),
            self.on_trigger_enter.take(),
            self.on_trigger_exit.take(),
        ];
        for key in keys.into_iter().flatten() {
            let _ = lua.remove_registry_value(key);
        }
    }
}

impl ScriptInstance {
    fn clear_registry(&mut self, lua: &Lua) {
        self.fns.clear(lua);
        if let Some(key) = self.env.take() {
            let _ = lua.remove_registry_value(key);
        }
    }
}

#[derive(Default)]
pub struct ScriptCommandBuffer {
    commands: Vec<ScriptCommand>,
    pending_spawns: Vec<SpawnRequest>,
}

#[derive(Clone, Debug)]
pub enum ScriptCommand {
    SetTransform {
        entity: EntityId,
        position: Option<Vec2>,
        rotation: Option<f32>,
        scale: Option<Vec2>,
    },
    SetSpriteVisibility {
        entity: EntityId,
        visible: bool,
    },
    SetSpriteTint {
        entity: EntityId,
        tint: [f32; 4],
    },
    ApplyImpulse {
        entity: EntityId,
        impulse: Vec2,
    },
    SetVelocity {
        entity: EntityId,
        velocity: Vec2,
    },
    UpdateAnimation {
        entity: EntityId,
        dt: f32,
    },
    SetAnimationPlaying {
        entity: EntityId,
        playing: bool,
    },
    ResetAnimation {
        entity: EntityId,
    },
    SetAnimationSpeed {
        entity: EntityId,
        speed: f32,
    },
    SetTilemapTile {
        entity: EntityId,
        x: u32,
        y: u32,
        tile_id: u32,
    },
    FillTilemapRect {
        entity: EntityId,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        tile_id: u32,
    },
    SetCameraActive {
        entity: EntityId,
        active: bool,
    },
    SetCameraZoom {
        entity: EntityId,
        zoom: f32,
    },
    SetCameraOffset {
        entity: EntityId,
        offset: Vec2,
    },
    SetCameraBounds {
        entity: EntityId,
        min: Vec2,
        max: Vec2,
    },
    ClearCameraBounds {
        entity: EntityId,
    },
    ShakeCamera {
        entity: EntityId,
        intensity: f32,
        duration: f32,
    },
    ZoomCamera {
        entity: EntityId,
        target: f32,
        speed: f32,
    },
    Despawn {
        entity: EntityId,
    },
}

#[derive(Clone, Debug)]
pub struct SpawnRequest {
    pub body: SpawnBody,
    pub initial_velocity: Option<Vec2>,
    pub tag: Option<String>,
}

#[derive(Clone, Debug)]
pub enum SpawnBody {
    Empty { position: Option<Vec2> },
    Dynamic { position: Vec2 },
}

impl ScriptCommandBuffer {
    pub fn set_transform(
        &mut self,
        entity: EntityId,
        position: Option<Vec2>,
        rotation: Option<f32>,
        scale: Option<Vec2>,
    ) {
        self.commands.push(ScriptCommand::SetTransform {
            entity,
            position,
            rotation,
            scale,
        });
    }

    pub fn set_sprite_visibility(&mut self, entity: EntityId, visible: bool) {
        self.commands
            .push(ScriptCommand::SetSpriteVisibility { entity, visible });
    }

    pub fn set_sprite_tint(&mut self, entity: EntityId, tint: [f32; 4]) {
        self.commands
            .push(ScriptCommand::SetSpriteTint { entity, tint });
    }

    pub fn apply_impulse(&mut self, entity: EntityId, impulse: Vec2) {
        self.commands
            .push(ScriptCommand::ApplyImpulse { entity, impulse });
    }

    pub fn set_velocity(&mut self, entity: EntityId, velocity: Vec2) {
        self.commands
            .push(ScriptCommand::SetVelocity { entity, velocity });
    }

    pub fn update_animation(&mut self, entity: EntityId, dt: f32) {
        self.commands.push(ScriptCommand::UpdateAnimation { entity, dt });
    }

    pub fn set_animation_playing(&mut self, entity: EntityId, playing: bool) {
        self.commands.push(ScriptCommand::SetAnimationPlaying { entity, playing });
    }

    pub fn reset_animation(&mut self, entity: EntityId) {
        self.commands.push(ScriptCommand::ResetAnimation { entity });
    }

    pub fn set_animation_speed(&mut self, entity: EntityId, speed: f32) {
        self.commands.push(ScriptCommand::SetAnimationSpeed { entity, speed });
    }

    pub fn set_tilemap_tile(&mut self, entity: EntityId, x: u32, y: u32, tile_id: u32) {
        self.commands.push(ScriptCommand::SetTilemapTile { entity, x, y, tile_id });
    }

    pub fn fill_tilemap_rect(&mut self, entity: EntityId, x: u32, y: u32, width: u32, height: u32, tile_id: u32) {
        self.commands.push(ScriptCommand::FillTilemapRect { entity, x, y, width, height, tile_id });
    }

    pub fn set_camera_active(&mut self, entity: EntityId, active: bool) {
        self.commands.push(ScriptCommand::SetCameraActive { entity, active });
    }

    pub fn set_camera_zoom(&mut self, entity: EntityId, zoom: f32) {
        self.commands.push(ScriptCommand::SetCameraZoom { entity, zoom });
    }

    pub fn set_camera_offset(&mut self, entity: EntityId, offset: Vec2) {
        self.commands.push(ScriptCommand::SetCameraOffset { entity, offset });
    }

    pub fn set_camera_bounds(&mut self, entity: EntityId, min: Vec2, max: Vec2) {
        self.commands.push(ScriptCommand::SetCameraBounds { entity, min, max });
    }

    pub fn clear_camera_bounds(&mut self, entity: EntityId) {
        self.commands.push(ScriptCommand::ClearCameraBounds { entity });
    }

    pub fn shake_camera(&mut self, entity: EntityId, intensity: f32, duration: f32) {
        self.commands.push(ScriptCommand::ShakeCamera { entity, intensity, duration });
    }

    pub fn zoom_camera(&mut self, entity: EntityId, target: f32, speed: f32) {
        self.commands.push(ScriptCommand::ZoomCamera { entity, target, speed });
    }

    pub fn spawn(&mut self, request: SpawnRequest) {
        self.pending_spawns.push(request);
    }

    pub fn despawn(&mut self, entity: EntityId) {
        self.commands.push(ScriptCommand::Despawn { entity });
    }

    pub fn apply(&mut self, world: &mut World, physics: &mut PhysicsWorld) {
        for request in self.pending_spawns.drain(..) {
            let entity = world.spawn();
            match request.body {
                SpawnBody::Empty { position } => {
                    if let Some(pos) = position {
                        world.insert(entity, Transform::new(pos));
                    }
                }
                SpawnBody::Dynamic { position } => {
                    world.insert(entity, Transform::new(position));
                    let _ = physics.create_body(entity, RigidBodyType::Dynamic, position, 0.0);
                }
            }

            if let Some(initial_velocity) = request.initial_velocity {
                physics.set_linear_velocity(entity, initial_velocity);
            }

            if let Some(tag) = request.tag {
                world.insert(entity, ScriptTag(tag));
            }
        }

        for command in self.commands.drain(..) {
            match command {
                ScriptCommand::SetTransform {
                    entity,
                    position,
                    rotation,
                    scale,
                } => {
                    if let Some(transform) = world.get_mut::<Transform>(entity) {
                        if let Some(p) = position {
                            transform.position = p;
                            physics.set_body_position(entity, p);
                        }
                        if let Some(r) = rotation {
                            transform.rotation = r;
                            physics.set_body_rotation(entity, r);
                        }
                        if let Some(s) = scale {
                            transform.scale = s;
                        }
                    }
                }
                ScriptCommand::SetSpriteVisibility { entity, visible } => {
                    if let Some(sprite) = world.get_mut::<SpriteComponent>(entity) {
                        sprite.visible = visible;
                    }
                }
                ScriptCommand::SetSpriteTint { entity, tint } => {
                    if let Some(sprite) = world.get_mut::<SpriteComponent>(entity) {
                        sprite.sprite.tint = tint;
                    }
                }
                ScriptCommand::ApplyImpulse { entity, impulse } => {
                    physics.apply_impulse(entity, impulse);
                }
                ScriptCommand::SetVelocity { entity, velocity } => {
                    physics.set_linear_velocity(entity, velocity);
                    physics.wake_up(entity, true);
                }
                ScriptCommand::UpdateAnimation { entity, dt } => {
                    if let Some(anim) = world.get_mut::<AnimatedSprite>(entity) {
                        anim.update(dt);
                    }
                }
                ScriptCommand::SetAnimationPlaying { entity, playing } => {
                    if let Some(anim) = world.get_mut::<AnimatedSprite>(entity) {
                        anim.playing = playing;
                    }
                }
                ScriptCommand::ResetAnimation { entity } => {
                    if let Some(anim) = world.get_mut::<AnimatedSprite>(entity) {
                        anim.reset();
                    }
                }
                ScriptCommand::SetAnimationSpeed { entity, speed } => {
                    if let Some(anim) = world.get_mut::<AnimatedSprite>(entity) {
                        anim.speed = speed;
                    }
                }
                ScriptCommand::SetTilemapTile { entity, x, y, tile_id } => {
                    if let Some(tilemap_comp) = world.get_mut::<crate::entities::TilemapComponent>(entity) {
                        tilemap_comp.tilemap.set_tile(x, y, tile_id);
                    }
                }
                ScriptCommand::FillTilemapRect { entity, x, y, width, height, tile_id } => {
                    if let Some(tilemap_comp) = world.get_mut::<crate::entities::TilemapComponent>(entity) {
                        tilemap_comp.tilemap.fill_rect(x, y, width, height, tile_id);
                    }
                }
                ScriptCommand::SetCameraActive { entity, active } => {
                    if let Some(cam) = world.get_mut::<CameraComponent>(entity) {
                        cam.active = active;
                    }
                }
                ScriptCommand::SetCameraZoom { entity, zoom } => {
                    if let Some(cam) = world.get_mut::<CameraComponent>(entity) {
                        cam.camera.zoom = zoom;
                        cam.camera.target_zoom = zoom;
                        cam.camera.zoom_speed = 0.0;
                    }
                }
                ScriptCommand::SetCameraOffset { entity, offset } => {
                    if let Some(cam) = world.get_mut::<CameraComponent>(entity) {
                        cam.camera.offset = offset;
                    }
                }
                ScriptCommand::SetCameraBounds { entity, min, max } => {
                    if let Some(cam) = world.get_mut::<CameraComponent>(entity) {
                        cam.camera.bounds = Some((min, max));
                    }
                }
                ScriptCommand::ClearCameraBounds { entity } => {
                    if let Some(cam) = world.get_mut::<CameraComponent>(entity) {
                        cam.camera.bounds = None;
                    }
                }
                ScriptCommand::ShakeCamera { entity, intensity, duration } => {
                    if let Some(cam) = world.get_mut::<CameraComponent>(entity) {
                        cam.camera.shake(intensity, duration);
                    }
                }
                ScriptCommand::ZoomCamera { entity, target, speed } => {
                    if let Some(cam) = world.get_mut::<CameraComponent>(entity) {
                        cam.camera.zoom_to(target, speed);
                    }
                }
                ScriptCommand::Despawn { entity } => {
                    physics.remove_body(entity);
                    world.despawn(entity);
                }
            }
        }
    }
}

/// Tag component that scripts can query for targeted entity lookups.
#[derive(Clone, Debug)]
pub struct ScriptTag(pub String);

// Lua userdata types
#[derive(Clone)]
pub struct ScriptSelf {
    entity: EntityId,
    world: *const World,
    physics: *const PhysicsWorld,
    input: *const InputState,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
    dt: f32,
    fixed_dt: f32,
}

impl UserData for ScriptSelf {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("entity", |_, this, ()| Ok(this.entity.to_u32() as i64));
        methods.add_method("time", |_, this, ()| {
            Ok(TimeFacet {
                dt: this.dt,
                fixed_dt: this.fixed_dt,
            })
        });
        methods.add_method("input", |_, this, ()| {
            Ok(InputFacet {
                input: this.input,
            })
        });
        methods.add_method("world", |_, this, ()| {
            Ok(WorldFacet {
                world: this.world,
                physics: this.physics,
                commands: Arc::clone(&this.commands),
            })
        });
        methods.add_method("transform", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<Transform>(this.entity).is_some() {
                Ok(Some(TransformFacet {
                    entity: this.entity,
                    world: this.world,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("physics", |_, this, ()| {
            let physics = unsafe { &*this.physics };
            if physics.has_body(this.entity) {
                Ok(Some(PhysicsFacet {
                    entity: this.entity,
                    physics: this.physics,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("sprite", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<SpriteComponent>(this.entity).is_some() {
                Ok(Some(SpriteFacet {
                    entity: this.entity,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("animation", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<AnimatedSprite>(this.entity).is_some() {
                Ok(Some(AnimationFacet {
                    entity: this.entity,
                    world: this.world,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("camera", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<CameraComponent>(this.entity).is_some() {
                Ok(Some(CameraFacet {
                    entity: this.entity,
                    world: this.world,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("tilemap", |_, this, ()| {
            let world = unsafe { &*this.world };
            if world.get::<crate::entities::TilemapComponent>(this.entity).is_some() {
                Ok(Some(TilemapFacet {
                    entity: this.entity,
                    world: this.world,
                    commands: Arc::clone(&this.commands),
                }))
            } else {
                Ok(None)
            }
        });
        methods.add_method("position", |_, this, ()| {
            let world = unsafe { &*this.world };
            match world.get::<Transform>(this.entity) {
                Some(t) => Ok(t.position),
                None => Ok(Vec2::ZERO),
            }
        });
        methods.add_method("set_position", |_, this, pos: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_transform(this.entity, Some(pos), None, None);
            }
            Ok(())
        });
        methods.add_method("apply_impulse", |_, this, impulse: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.apply_impulse(this.entity, impulse);
            }
            Ok(())
        });
    }
}

impl ScriptSelf {
    fn new(
        entity: EntityId,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
        commands: Arc<Mutex<ScriptCommandBuffer>>,
        dt: f32,
        fixed_dt: f32,
    ) -> Self {
        Self {
            entity,
            world,
            physics,
            input,
            commands,
            dt,
            fixed_dt,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TimeFacet {
    dt: f32,
    fixed_dt: f32,
}

impl UserData for TimeFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("delta", |_, this, ()| Ok(this.dt));
        methods.add_method("fixed_delta", |_, this, ()| Ok(this.fixed_dt));
    }
}

#[derive(Clone)]
pub struct InputFacet {
    input: *const InputState,
}

impl UserData for InputFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("is_key_down", |_, this, name: String| {
            Ok(parse_key(&name)
                .map(|k| unsafe { &*this.input }.is_key_down(k))
                .unwrap_or(false))
        });
        methods.add_method("is_key_pressed", |_, this, name: String| {
            Ok(parse_key(&name)
                .map(|k| unsafe { &*this.input }.is_key_pressed(k))
                .unwrap_or(false))
        });
        methods.add_method("is_key_released", |_, this, name: String| {
            Ok(parse_key(&name)
                .map(|k| unsafe { &*this.input }.is_key_released(k))
                .unwrap_or(false))
        });
        methods.add_method("mouse_pos_screen", |_, this, ()| {
            let (x, y) = unsafe { &*this.input }.mouse_screen_pixels();
            Ok(Vec2::new(x, y))
        });
        methods.add_method("is_mouse_pressed", |_, this, button_name: String| {
            use winit::event::MouseButton;
            let button = match button_name.as_str() {
                "Left" | "left" => MouseButton::Left,
                "Right" | "right" => MouseButton::Right,
                "Middle" | "middle" => MouseButton::Middle,
                _ => return Ok(false),
            };
            Ok(unsafe { &*this.input }.is_mouse_pressed(button))
        });
        methods.add_method("is_mouse_down", |_, this, button_name: String| {
            use winit::event::MouseButton;
            let button = match button_name.as_str() {
                "Left" | "left" => MouseButton::Left,
                "Right" | "right" => MouseButton::Right,
                "Middle" | "middle" => MouseButton::Middle,
                _ => return Ok(false),
            };
            Ok(unsafe { &*this.input }.is_mouse_down(button))
        });
    }
}

#[derive(Clone)]
pub struct WorldFacet {
    world: *const World,
    physics: *const PhysicsWorld,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for WorldFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("find_by_tag", |_, this, tag: String| {
            for (entity, t) in unsafe { &*this.world }.query::<ScriptTag>() {
                if t.0 == tag {
                    return Ok(Some(entity.to_u32() as i64));
                }
            }
            Ok(None)
        });
        methods.add_method("find_all_by_tag", |lua, this, tag: String| {
            let table = lua.create_table()?;
            let mut index = 1;
            for (entity, t) in unsafe { &*this.world }.query::<ScriptTag>() {
                if t.0 == tag {
                    table.set(index, entity.to_u32() as i64)?;
                    index += 1;
                }
            }
            Ok(table)
        });
        methods.add_method("despawn", |_, this, entity_raw: i64| {
            if entity_raw < 0 {
                return Err(mlua::Error::RuntimeError("Entity id must be non-negative".to_string()));
            }
            let entity = EntityId(entity_raw as u32);
            if !unsafe { &*this.world }.is_alive(entity) {
                return Ok(());
            }
            if let Ok(mut commands) = this.commands.lock() {
                commands.despawn(entity);
            }
            Ok(())
        });
        methods.add_method("tag_of", |_, this, entity_raw: i64| {
            if entity_raw < 0 {
                return Ok(None);
            }
            let entity = EntityId(entity_raw as u32);
            Ok(unsafe { &*this.world }
                .get::<ScriptTag>(entity)
                .map(|tag| tag.0.clone()))
        });
        methods.add_method("position", |_, this, entity_raw: i64| {
            if entity_raw < 0 {
                return Ok(Vec2::ZERO);
            }
            let entity = EntityId(entity_raw as u32);
            let physics = unsafe { &*this.physics };
            if let Some(pos) = physics.body_position(entity) {
                return Ok(pos);
            }
            let world = unsafe { &*this.world };
            Ok(world
                .get::<Transform>(entity)
                .map(|t| t.position)
                .unwrap_or(Vec2::ZERO))
        });
        methods.add_method("collider_size", |_, this, entity_raw: i64| {
            if entity_raw < 0 {
                return Ok(Vec2::ZERO);
            }
            let entity = EntityId(entity_raw as u32);
            let physics = unsafe { &*this.physics };
            let colliders = physics.get_colliders(entity);
            let Some((shape, _, _, _, _, _)) = colliders.first().cloned() else {
                return Ok(Vec2::ZERO);
            };
            let size = match shape {
                ColliderShape::Box { hx, hy } => Vec2::new(hx * 2.0, hy * 2.0),
                ColliderShape::Circle { radius } => Vec2::new(radius * 2.0, radius * 2.0),
                ColliderShape::CapsuleY { half_height, radius } => {
                    Vec2::new(radius * 2.0, half_height * 2.0 + radius * 2.0)
                }
            };
            Ok(size)
        });
        methods.add_method("spawn_dynamic", |_, this, (position, velocity): (Vec2, Vec2)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.spawn(SpawnRequest {
                    body: SpawnBody::Dynamic { position },
                    initial_velocity: Some(velocity),
                    tag: None,
                });
            }
            Ok(())
        });
        methods.add_method("spawn_empty", |_, this, (position, tag): (Option<Vec2>, Option<String>)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.spawn(SpawnRequest {
                    body: SpawnBody::Empty { position },
                    initial_velocity: None,
                    tag,
                });
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct TransformFacet {
    entity: EntityId,
    world: *const World,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for TransformFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("position", |_, this, ()| {
            match unsafe { &*this.world }.get::<Transform>(this.entity) {
                Some(t) => Ok(t.position),
                None => Ok(Vec2::ZERO),
            }
        });
        methods.add_method("rotation", |_, this, ()| {
            match unsafe { &*this.world }.get::<Transform>(this.entity) {
                Some(t) => Ok(t.rotation),
                None => Ok(0.0),
            }
        });
        methods.add_method("set_position", |_, this, pos: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_transform(this.entity, Some(pos), None, None);
            }
            Ok(())
        });
        methods.add_method("set_rotation", |_, this, rot: f64| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_transform(this.entity, None, Some(rot as f32), None);
            }
            Ok(())
        });
        methods.add_method("set_scale", |_, this, scale: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_transform(this.entity, None, None, Some(scale));
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct PhysicsFacet {
    entity: EntityId,
    physics: *const PhysicsWorld,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for PhysicsFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("velocity", |_, this, ()| {
            let physics = unsafe { &*this.physics };
            Ok(physics.linear_velocity(this.entity).unwrap_or(Vec2::ZERO))
        });
        methods.add_method("set_velocity", |_, this, velocity: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_velocity(this.entity, velocity);
            }
            Ok(())
        });
        methods.add_method("apply_impulse", |_, this, impulse: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.apply_impulse(this.entity, impulse);
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct SpriteFacet {
    entity: EntityId,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for SpriteFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("set_visible", |_, this, visible: bool| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_sprite_visibility(this.entity, visible);
            }
            Ok(())
        });
        methods.add_method("set_tint", |_, this, tint: mlua::Table| {
            let r: f64 = tint.get(1)?;
            let g: f64 = tint.get(2)?;
            let b: f64 = tint.get(3)?;
            let a: f64 = tint.get(4)?;
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_sprite_tint(this.entity, [r as f32, g as f32, b as f32, a as f32]);
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct CameraFacet {
    entity: EntityId,
    world: *const World,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for CameraFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("is_active", |_, this, ()| {
            let world = unsafe { &*this.world };
            Ok(world
                .get::<CameraComponent>(this.entity)
                .map(|cam| cam.active)
                .unwrap_or(false))
        });
        methods.add_method("set_active", |_, this, active: bool| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_camera_active(this.entity, active);
            }
            Ok(())
        });
        methods.add_method("zoom", |_, this, ()| {
            let world = unsafe { &*this.world };
            Ok(world
                .get::<CameraComponent>(this.entity)
                .map(|cam| cam.camera.zoom)
                .unwrap_or(1.0))
        });
        methods.add_method("set_zoom", |_, this, zoom: f64| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_camera_zoom(this.entity, zoom as f32);
            }
            Ok(())
        });
        methods.add_method("zoom_to", |_, this, (target, speed): (f64, f64)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.zoom_camera(this.entity, target as f32, speed as f32);
            }
            Ok(())
        });
        methods.add_method("offset", |_, this, ()| {
            let world = unsafe { &*this.world };
            Ok(world
                .get::<CameraComponent>(this.entity)
                .map(|cam| cam.camera.offset)
                .unwrap_or(Vec2::ZERO))
        });
        methods.add_method("set_offset", |_, this, offset: Vec2| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_camera_offset(this.entity, offset);
            }
            Ok(())
        });
        methods.add_method("set_bounds", |_, this, (min, max): (Vec2, Vec2)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_camera_bounds(this.entity, min, max);
            }
            Ok(())
        });
        methods.add_method("clear_bounds", |_, this, ()| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.clear_camera_bounds(this.entity);
            }
            Ok(())
        });
        methods.add_method("shake", |_, this, (intensity, duration): (f64, f64)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.shake_camera(this.entity, intensity as f32, duration as f32);
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
pub struct AnimationFacet {
    entity: EntityId,
    world: *const World,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for AnimationFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("update", |_, this, dt: f64| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.update_animation(this.entity, dt as f32);
            }
            Ok(())
        });
        methods.add_method("play", |_, this, ()| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_animation_playing(this.entity, true);
            }
            Ok(())
        });
        methods.add_method("pause", |_, this, ()| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_animation_playing(this.entity, false);
            }
            Ok(())
        });
        methods.add_method("reset", |_, this, ()| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.reset_animation(this.entity);
            }
            Ok(())
        });
        methods.add_method("set_speed", |_, this, speed: f64| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_animation_speed(this.entity, speed as f32);
            }
            Ok(())
        });
        methods.add_method("current_frame_index", |_, this, ()| {
            let world = unsafe { &*this.world };
            Ok(world.get::<AnimatedSprite>(this.entity)
                .map(|a| a.current_frame_index as i64)
                .unwrap_or(0))
        });
    }
}

#[derive(Clone)]
pub struct TilemapFacet {
    entity: EntityId,
    world: *const World,
    commands: Arc<Mutex<ScriptCommandBuffer>>,
}

impl UserData for TilemapFacet {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("set_tile", |_, this, (x, y, tile_id): (u32, u32, u32)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.set_tilemap_tile(this.entity, x, y, tile_id);
            }
            Ok(())
        });
        methods.add_method("get_tile", |_, this, (x, y): (u32, u32)| {
            let world = unsafe { &*this.world };
            Ok(world.get::<crate::entities::TilemapComponent>(this.entity)
                .and_then(|t| t.tilemap.get_tile(x, y))
                .map(|t| t.id as i64)
                .unwrap_or(0))
        });
        methods.add_method("fill_rect", |_, this, (x, y, w, h, tile_id): (u32, u32, u32, u32, u32)| {
            if let Ok(mut commands) = this.commands.lock() {
                commands.fill_tilemap_rect(this.entity, x, y, w, h, tile_id);
            }
            Ok(())
        });
        methods.add_method("world_to_tile", |lua, this, world_pos: Vec2| {
            let world = unsafe { &*this.world };
            if let Some(tilemap_comp) = world.get::<crate::entities::TilemapComponent>(this.entity) {
                let (tx, ty) = tilemap_comp.tilemap.world_to_tile(world_pos);
                let table = lua.create_table()?;
                table.set("x", tx)?;
                table.set("y", ty)?;
                Ok(mlua::Value::Table(table))
            } else {
                let table = lua.create_table()?;
                table.set("x", 0)?;
                table.set("y", 0)?;
                Ok(mlua::Value::Table(table))
            }
        });
        methods.add_method("tile_to_world", |_, this, (x, y): (u32, u32)| {
            let world = unsafe { &*this.world };
            Ok(world.get::<crate::entities::TilemapComponent>(this.entity)
                .map(|t| t.tilemap.tile_to_world(x, y))
                .unwrap_or(Vec2::ZERO))
        });
    }
}

/// Central runtime that owns the embedded scripting engine and per-entity instances.
pub struct ScriptRuntime {
    pub(crate) lua: Lua,
    modules: HashMap<String, ScriptModule>,
    instances: BTreeMap<ScriptInstanceKey, ScriptInstance>,
    command_buffer: Arc<Mutex<ScriptCommandBuffer>>,
    hot_reload: bool,
}

impl ScriptRuntime {
    /// Create a new runtime with Lua backend and built-in API bindings.
    pub fn new() -> Result<Self> {
        let lua = Lua::new();
        
        // Register print function
        let print_func = lua.create_function(|_, msg: String| {
            println!("[LUA] {}", msg);
            Ok(())
        })?;
        lua.globals().set("print", print_func)?;

        // Register Vec2 type
        lua.register_userdata_type::<Vec2>(|reg| {
            reg.add_method("x", |_, this, ()| Ok(this.x));
            reg.add_method("y", |_, this, ()| Ok(this.y));
            reg.add_method_mut("set_x", |_, this, x: f64| {
                this.x = x as f32;
                Ok(())
            });
            reg.add_method_mut("set_y", |_, this, y: f64| {
                this.y = y as f32;
                Ok(())
            });
        })?;

        // Register vec2 function
        let vec2_func = lua.create_function(|_, (x, y): (f64, f64)| {
            Ok(Vec2::new(x as f32, y as f32))
        })?;
        lua.globals().set("vec2", vec2_func)?;

        // Register GridCoord type
        lua.register_userdata_type::<crate::grid::GridCoord>(|reg| {
            reg.add_method("x", |_, this, ()| Ok(this.x));
            reg.add_method("y", |_, this, ()| Ok(this.y));
        })?;
        
        // Register GridNode type
        lua.register_userdata_type::<crate::pathfinding::GridNode>(|reg| {
            reg.add_method("x", |_, this, ()| Ok(this.x));
            reg.add_method("y", |_, this, ()| Ok(this.y));
        })?;

        // UserData types are automatically registered when first used
        // No explicit registration needed - the UserData impl provides the methods

        Ok(Self {
            lua,
            modules: HashMap::new(),
            instances: BTreeMap::new(),
            command_buffer: Arc::new(Mutex::new(ScriptCommandBuffer::default())),
            hot_reload: false,
        })
    }

    /// Toggle hot reload for script files on disk.
    pub fn with_hot_reload(mut self, enabled: bool) -> Self {
        self.hot_reload = enabled;
        self
    }
    
    /// Register a custom Lua function in the global namespace.
    /// This allows demos/examples to expose custom APIs to scripts.
    pub fn register_function<F, A, R>(&mut self, name: &str, func: F) -> Result<()>
    where
        F: 'static + Send + for<'lua> Fn(&'lua Lua, A) -> mlua::Result<R>,
        A: for<'lua> mlua::FromLuaMulti<'lua>,
        R: for<'lua> mlua::IntoLuaMulti<'lua>,
    {
        let func = self.lua.create_function(func)?;
        self.lua.globals().set(name, func)?;
        Ok(())
    }
    
    /// Get mutable access to the Lua state for advanced use cases.
    /// This allows registering custom functions and types directly.
    pub fn lua_mut(&mut self) -> &mut Lua {
        &mut self.lua
    }

    /// Drive `on_update` for all scripts.
    pub fn update(
        &mut self,
        world: &mut World,
        physics: &mut PhysicsWorld,
        input: &InputState,
        dt: f32,
    ) -> Result<()> {
        self.sync_instances(world, physics, input)?;
        self.run_stage(world, physics, input, dt, 0.0, ScriptStage::Update)?;
        if let Ok(mut buffer) = self.command_buffer.lock() {
            buffer.apply(world, physics);
        }
        Ok(())
    }

    /// Drive `on_fixed_update` for all scripts.
    pub fn fixed_update(
        &mut self,
        world: &mut World,
        physics: &mut PhysicsWorld,
        input: &InputState,
        fixed_dt: f32,
    ) -> Result<()> {
        self.sync_instances(world, physics, input)?;
        self.run_stage(
            world,
            physics,
            input,
            0.0,
            fixed_dt,
            ScriptStage::FixedUpdate,
        )?;
        if let Ok(mut buffer) = self.command_buffer.lock() {
            buffer.apply(world, physics);
        }
        Ok(())
    }

    /// Drive `on_post_physics` for all scripts (run after physics step).
    pub fn post_physics_update(
        &mut self,
        world: &mut World,
        physics: &mut PhysicsWorld,
        input: &InputState,
        fixed_dt: f32,
    ) -> Result<()> {
        self.sync_instances(world, physics, input)?;
        self.run_stage(
            world,
            physics,
            input,
            0.0,
            fixed_dt,
            ScriptStage::PostPhysics,
        )?;
        if let Ok(mut buffer) = self.command_buffer.lock() {
            buffer.apply(world, physics);
        }
        Ok(())
    }

    /// Dispatch physics collision/trigger events into script callbacks.
    pub fn handle_physics_events(
        &mut self,
        events: &[PhysicsEvent],
        world: &mut World,
        physics: &mut PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        for event in events {
            let (entity, other, is_trigger, started) = match event {
                PhysicsEvent::CollisionEnter { a, b } => (*a, *b, false, true),
                PhysicsEvent::CollisionExit { a, b } => (*a, *b, false, false),
                PhysicsEvent::TriggerEnter { a, b } => (*a, *b, true, true),
                PhysicsEvent::TriggerExit { a, b } => (*a, *b, true, false),
            };

            self.run_event(entity, other, is_trigger, started, world, physics, input)?;
            self.run_event(other, entity, is_trigger, started, world, physics, input)?;
        }

        if let Ok(mut buffer) = self.command_buffer.lock() {
            buffer.apply(world, physics);
        }
        Ok(())
    }

    fn run_event(
        &mut self,
        entity: EntityId,
        other: EntityId,
        is_trigger: bool,
        started: bool,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        let key_filter: Vec<_> = self
            .instances
            .keys()
            .filter(|k| k.entity == entity)
            .cloned()
            .collect();

        for key in key_filter {
            if let Some(instance) = self.instances.get(&key) {
                let ctx = ScriptSelf::new(
                    entity,
                    world,
                    physics,
                    input,
                    Arc::clone(&self.command_buffer),
                    0.0,
                    0.0,
                );
                let fn_key = match (is_trigger, started) {
                    (false, true) => &instance.fns.on_collision_enter,
                    (false, false) => &instance.fns.on_collision_exit,
                    (true, true) => &instance.fns.on_trigger_enter,
                    (true, false) => &instance.fns.on_trigger_exit,
                };
                self.call_script_fn(
                    fn_key,
                    (ctx, other.to_u32() as i64),
                    "event",
                    &instance.script_path,
                )?;
            }
        }

        Ok(())
    }

    fn sync_instances(
        &mut self,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        let mut desired = Vec::new();
        let mut pairs = world.query::<ScriptComponent>();
        pairs.sort_by_key(|(entity, _)| entity.to_u32());

        for (entity, scripts) in pairs {
            for (slot, attachment) in scripts.scripts.iter().enumerate() {
                let key = ScriptInstanceKey {
                    entity,
                    slot: slot as u32,
                };
                desired.push(key);

                self.load_module(&attachment.path)?;
                let module_modified = self.modules[&attachment.path].modified;

                let needs_reload = match self.instances.get(&key) {
                    Some(entry) => self.hot_reload && module_modified != entry.last_loaded,
                    None => false,
                };

                if needs_reload {
                    if let Some(mut instance) = self.instances.remove(&key) {
                        self.run_destroy(&mut instance, world, physics, input)?;
                        instance.clear_registry(&self.lua);
                    }
                }

                if !self.instances.contains_key(&key) {
                    let mut instance =
                        self.create_instance(key, attachment, module_modified)?;
                    if !instance.has_started {
                        self.run_create_and_start(&mut instance, world, physics, input)?;
                    }
                    self.instances.insert(key, instance);
                }
            }
        }

        let existing: Vec<_> = self.instances.keys().cloned().collect();
        for key in existing {
            if !desired.contains(&key) {
                if let Some(mut inst) = self.instances.remove(&key) {
                    self.run_destroy(&mut inst, world, physics, input)?;
                    inst.clear_registry(&self.lua);
                }
            }
        }

        Ok(())
    }

    fn run_stage(
        &mut self,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
        dt: f32,
        fixed_dt: f32,
        stage: ScriptStage,
    ) -> Result<()> {
        for instance in self.instances.values() {
            let ctx = ScriptSelf::new(
                instance.key.entity,
                world,
                physics,
                input,
                Arc::clone(&self.command_buffer),
                dt,
                fixed_dt,
            );

            let (fn_key, include_dt) = match stage {
                ScriptStage::Update => (&instance.fns.on_update, true),
                ScriptStage::FixedUpdate => (&instance.fns.on_fixed_update, true),
                ScriptStage::PostPhysics => (&instance.fns.on_post_physics, true),
                ScriptStage::Draw => (&instance.fns.on_draw, false),
            };

            if include_dt {
                self.call_script_fn(
                    fn_key,
                    (ctx, if stage == ScriptStage::Update { dt } else { fixed_dt }),
                    "stage",
                    &instance.script_path,
                )?;
            } else {
                self.call_script_fn(fn_key, (ctx,), "stage", &instance.script_path)?;
            }
        }
        Ok(())
    }

    fn run_create_and_start(
        &mut self,
        instance: &mut ScriptInstance,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        let ctx = ScriptSelf::new(
            instance.key.entity,
            world,
            physics,
            input,
            Arc::clone(&self.command_buffer),
            0.0,
            0.0,
        );

        self.call_script_fn(
            &instance.fns.on_create,
            (ctx.clone(),),
            "on_create",
            &instance.script_path,
        )?;
        self.call_script_fn(
            &instance.fns.on_start,
            (ctx,),
            "on_start",
            &instance.script_path,
        )?;

        instance.has_started = true;
        Ok(())
    }

    fn run_destroy(
        &mut self,
        instance: &mut ScriptInstance,
        world: &World,
        physics: &PhysicsWorld,
        input: &InputState,
    ) -> Result<()> {
        let ctx = ScriptSelf::new(
            instance.key.entity,
            world,
            physics,
            input,
            Arc::clone(&self.command_buffer),
            0.0,
            0.0,
        );

        self.call_script_fn(
            &instance.fns.on_destroy,
            (ctx,),
            "on_destroy",
            &instance.script_path,
        )?;

        Ok(())
    }

    fn load_module(&mut self, path: &str) -> Result<()> {
        if !self.hot_reload && self.modules.contains_key(path) {
            return Ok(());
        }

        let contents = fs::read_to_string(Path::new(path))
            .map_err(|err| anyhow!("Failed to load script {path}: {err}"))?;

        let modified = fs::metadata(path).ok().and_then(|m| m.modified().ok());
        self.modules
            .insert(path.to_string(), ScriptModule { source: contents, modified });
        Ok(())
    }

    fn call_script_fn<'lua, A>(
        &'lua self,
        func_key: &Option<mlua::RegistryKey>,
        args: A,
        label: &str,
        script_path: &str,
    ) -> Result<()>
    where
        A: mlua::IntoLuaMulti<'lua>,
    {
        let Some(key) = func_key else {
            return Ok(());
        };
        let func: mlua::Function = self.lua.registry_value(key)?;
        if let Err(e) = func.call::<_, ()>(args) {
            eprintln!("[Script] Error calling {}: {}", label, e);
            return Err(anyhow!("Lua error in {label} ({script_path}): {e}"));
        }
        Ok(())
    }

    fn create_instance(
        &mut self,
        key: ScriptInstanceKey,
        attachment: &ScriptAttachment,
        module_modified: Option<SystemTime>,
    ) -> Result<ScriptInstance> {
        let module = &self.modules[&attachment.path];
        let env = self.lua.create_table()?;
        let globals = self.lua.globals();
        let meta = self.lua.create_table()?;
        meta.set("__index", globals)?;
        env.set_metatable(Some(meta));

        let params_table = self.lua.create_table()?;
        for (k, v) in &attachment.params.values {
            match v {
                ScriptValue::Number(n) => params_table.set(k.as_str(), *n)?,
                ScriptValue::Bool(b) => params_table.set(k.as_str(), *b)?,
                ScriptValue::Text(s) => params_table.set(k.as_str(), s.as_str())?,
                ScriptValue::Vec2(v) => params_table.set(k.as_str(), v.clone())?,
            }
        }
        env.set("params", params_table)?;

        let chunk = self.lua.load(&module.source).set_name(&attachment.path);
        let chunk = chunk.set_environment(env.clone());
        if let Err(e) = chunk.exec() {
            return Err(anyhow!("Failed to execute script {}: {}", attachment.path, e));
        }

        let fns = ScriptFns {
            on_create: self.capture_fn(&env, "on_create")?,
            on_start: self.capture_fn(&env, "on_start")?,
            on_update: self.capture_fn(&env, "on_update")?,
            on_fixed_update: self.capture_fn(&env, "on_fixed_update")?,
            on_post_physics: self.capture_fn(&env, "on_post_physics")?,
            on_draw: self.capture_fn(&env, "on_draw")?,
            on_destroy: self.capture_fn(&env, "on_destroy")?,
            on_collision_enter: self.capture_fn(&env, "on_collision_enter")?,
            on_collision_exit: self.capture_fn(&env, "on_collision_exit")?,
            on_trigger_enter: self.capture_fn(&env, "on_trigger_enter")?,
            on_trigger_exit: self.capture_fn(&env, "on_trigger_exit")?,
        };

        let env_key = self.lua.create_registry_value(env)?;
        Ok(ScriptInstance {
            key,
            script_path: attachment.path.clone(),
            has_started: false,
            last_loaded: module_modified,
            env: Some(env_key),
            fns,
        })
    }

    fn capture_fn(
        &self,
        env: &mlua::Table<'_>,
        name: &str,
    ) -> Result<Option<mlua::RegistryKey>> {
        let value = env.get::<_, Value>(name)?;
        match value {
            Value::Function(func) => Ok(Some(self.lua.create_registry_value(func)?)),
            _ => Ok(None),
        }
    }
}

#[derive(PartialEq)]
enum ScriptStage {
    Update,
    FixedUpdate,
    PostPhysics,
    Draw,
}

fn parse_key(name: &str) -> Option<winit::keyboard::KeyCode> {
    use winit::keyboard::KeyCode;

    match name {
        "W" | "w" => Some(KeyCode::KeyW),
        "A" | "a" => Some(KeyCode::KeyA),
        "S" | "s" => Some(KeyCode::KeyS),
        "D" | "d" => Some(KeyCode::KeyD),
        "Space" | "space" => Some(KeyCode::Space),
        "Left" | "left" => Some(KeyCode::ArrowLeft),
        "Right" | "right" => Some(KeyCode::ArrowRight),
        "Up" | "up" => Some(KeyCode::ArrowUp),
        "Down" | "down" => Some(KeyCode::ArrowDown),
        _ => None,
    }
}
