use mlua::{Lua, Table};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use sindri::component::{BodyType, Component};
use sindri::math::Vec2;
use sindri::physics::{ColliderShape, PhysicsWorld, RigidBodyType as PhysicsBodyType};
use sindri::scene::Scene;
use sindri::world::EntityId;
use sindri_server::routes::SharedErrors;

enum CameraCommand {
    SetActive(bool),
    SetZoom(f32),
    ZoomTo {
        target: f32,
        speed: f32,
    },
    SetOffset {
        x: f32,
        y: f32,
    },
    SetBounds {
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
    },
    ClearBounds,
    Shake {
        intensity: f32,
        duration: f32,
    },
}

enum SceneCommand {
    SetTransformPosition { x: f32, y: f32 },
    SetTransformRotation(f32),
    SetTransformScale { x: f32, y: f32 },
    SetSpriteTint([f32; 4]),
}

fn resolve_script_path(scripts_root: &Path, script_path: &str) -> PathBuf {
    let path = Path::new(script_path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    // Strip "scripts/" prefix so both "player.lua" and "scripts/player.lua" resolve correctly
    let relative = path.strip_prefix("scripts").unwrap_or(path);
    scripts_root.join(relative)
}

fn read_vec2(table: &Table) -> mlua::Result<(f32, f32)> {
    let x = table.get::<_, f64>("x").or_else(|_| table.get(1))?;
    let y = table.get::<_, f64>("y").or_else(|_| table.get(2))?;
    Ok((x as f32, y as f32))
}

fn read_color(table: &Table) -> mlua::Result<[f32; 4]> {
    let r = table.get::<_, f64>("r").or_else(|_| table.get(1))?;
    let g = table.get::<_, f64>("g").or_else(|_| table.get(2))?;
    let b = table.get::<_, f64>("b").or_else(|_| table.get(3))?;
    let a = table
        .get::<_, f64>("a")
        .or_else(|_| table.get(4))
        .unwrap_or(1.0);
    Ok([r as f32, g as f32, b as f32, a as f32])
}

/// Normalize browser/winit key names to the canonical Sindri key names.
/// e.g. "a" → "A", " " → "Space", "ArrowLeft" → "Left"
fn normalize_key(key: &str) -> String {
    match key {
        " " => "Space".to_string(),
        "ArrowLeft" => "Left".to_string(),
        "ArrowRight" => "Right".to_string(),
        "ArrowUp" => "Up".to_string(),
        "ArrowDown" => "Down".to_string(),
        k if k.len() == 1 => k.to_uppercase(),
        k => k.to_string(),
    }
}

fn apply_scene_commands(scene: &mut Scene, entity_id: u64, commands: Vec<SceneCommand>) {
    let Some(entity) = scene.entities.get_mut(&entity_id) else {
        return;
    };

    for command in commands {
        match command {
            SceneCommand::SetTransformPosition { x, y } => {
                if let Some(transform) = entity.components.iter_mut().find_map(|component| {
                    if let Component::Transform(transform) = component {
                        Some(transform)
                    } else {
                        None
                    }
                }) {
                    transform.x = x;
                    transform.y = y;
                }
            }
            SceneCommand::SetTransformRotation(rotation) => {
                if let Some(transform) = entity.components.iter_mut().find_map(|component| {
                    if let Component::Transform(transform) = component {
                        Some(transform)
                    } else {
                        None
                    }
                }) {
                    transform.rotation = rotation;
                }
            }
            SceneCommand::SetTransformScale { x, y } => {
                if let Some(transform) = entity.components.iter_mut().find_map(|component| {
                    if let Component::Transform(transform) = component {
                        Some(transform)
                    } else {
                        None
                    }
                }) {
                    transform.scale_x = x;
                    transform.scale_y = y;
                }
            }
            SceneCommand::SetSpriteTint(color) => {
                if let Some(sprite) = entity.components.iter_mut().find_map(|component| {
                    if let Component::Sprite(sprite) = component {
                        Some(sprite)
                    } else {
                        None
                    }
                }) {
                    sprite.color = color;
                }
            }
        }
    }
}

fn apply_camera_commands(scene: &mut Scene, entity_id: u64, commands: Vec<CameraCommand>) {
    for command in commands {
        match command {
            CameraCommand::SetActive(active) => {
                if active {
                    for (other_id, entity) in scene.entities.iter_mut() {
                        for component in &mut entity.components {
                            if let Component::Camera(camera) = component {
                                camera.active = *other_id == entity_id;
                            }
                        }
                    }
                } else if let Some(entity) = scene.entities.get_mut(&entity_id) {
                    for component in &mut entity.components {
                        if let Component::Camera(camera) = component {
                            camera.active = false;
                        }
                    }
                }
            }
            CameraCommand::SetZoom(zoom) => {
                if let Some(camera) = entity_camera_mut(scene, entity_id) {
                    camera.zoom = zoom.max(0.01);
                    camera.runtime_target_zoom = None;
                    camera.runtime_zoom_speed = 0.0;
                }
            }
            CameraCommand::ZoomTo { target, speed } => {
                if let Some(camera) = entity_camera_mut(scene, entity_id) {
                    camera.runtime_target_zoom = Some(target.max(0.01));
                    camera.runtime_zoom_speed = speed.max(0.0);
                }
            }
            CameraCommand::SetOffset { x, y } => {
                if let Some(camera) = entity_camera_mut(scene, entity_id) {
                    camera.offset_x = x;
                    camera.offset_y = y;
                }
            }
            CameraCommand::SetBounds {
                min_x,
                min_y,
                max_x,
                max_y,
            } => {
                if let Some(camera) = entity_camera_mut(scene, entity_id) {
                    camera.bounds_min_x = Some(min_x);
                    camera.bounds_min_y = Some(min_y);
                    camera.bounds_max_x = Some(max_x);
                    camera.bounds_max_y = Some(max_y);
                }
            }
            CameraCommand::ClearBounds => {
                if let Some(camera) = entity_camera_mut(scene, entity_id) {
                    camera.bounds_min_x = None;
                    camera.bounds_min_y = None;
                    camera.bounds_max_x = None;
                    camera.bounds_max_y = None;
                }
            }
            CameraCommand::Shake {
                intensity,
                duration,
            } => {
                if let Some(camera) = entity_camera_mut(scene, entity_id) {
                    camera.runtime_shake_intensity = intensity.max(camera.runtime_shake_intensity);
                    camera.runtime_shake_timer = duration.max(camera.runtime_shake_timer);
                    camera.runtime_shake_seed = 0.0;
                }
            }
        }
    }
    normalize_camera_activity(scene);
}

fn entity_camera_mut(scene: &mut Scene, entity_id: u64) -> Option<&mut sindri::component::Camera> {
    scene
        .entities
        .get_mut(&entity_id)?
        .components
        .iter_mut()
        .find_map(|component| {
            if let Component::Camera(camera) = component {
                Some(camera)
            } else {
                None
            }
        })
}

fn normalize_camera_activity(scene: &mut Scene) {
    let mut camera_refs: Vec<(u64, usize, bool)> =
        scene
            .entities
            .iter()
            .flat_map(|(entity_id, entity)| {
                entity.components.iter().enumerate().filter_map(
                    move |(component_idx, component)| {
                        if let Component::Camera(camera) = component {
                            Some((*entity_id, component_idx, camera.active))
                        } else {
                            None
                        }
                    },
                )
            })
            .collect();
    camera_refs.sort_by_key(|(entity_id, component_idx, _)| (*entity_id, *component_idx));
    let active = camera_refs
        .iter()
        .find(|(_, _, active)| *active)
        .or_else(|| camera_refs.first())
        .copied();
    let Some((active_entity, active_component_idx, _)) = active else {
        return;
    };

    for (entity_id, entity) in scene.entities.iter_mut() {
        for (component_idx, component) in entity.components.iter_mut().enumerate() {
            if let Component::Camera(camera) = component {
                camera.active =
                    *entity_id == active_entity && component_idx == active_component_idx;
            }
        }
    }
}

pub struct LuaRuntime {
    lua: Lua,
    envs: HashMap<(u64, String), mlua::RegistryKey>,
    started: HashSet<(u64, String)>,
    missing_scripts: HashSet<(u64, String)>,
    pub elapsed: f64,
    // Key state shared into Lua globals each frame
    keys: Arc<RwLock<HashSet<String>>>,
    pressed_keys: Arc<RwLock<HashSet<String>>>,
    prev_keys: HashSet<String>,
    errors: SharedErrors,
    // Velocity cache for script access — keyed by entity id
    pub velocities: HashMap<u64, (f32, f32)>,
    physics: PhysicsWorld,
    physics_initialized: bool,
}

impl LuaRuntime {
    pub fn new(errors: SharedErrors) -> anyhow::Result<Self> {
        let lua = Lua::new();

        let print_fn = lua.create_function(|_, args: mlua::MultiValue| {
            let parts: Vec<String> = args
                .iter()
                .map(|v| match v {
                    mlua::Value::String(s) => s.to_str().unwrap_or("?").to_string(),
                    mlua::Value::Integer(n) => n.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Boolean(b) => b.to_string(),
                    mlua::Value::Nil => "nil".to_string(),
                    _ => "?".to_string(),
                })
                .collect();
            println!("[lua] {}", parts.join("\t"));
            Ok(())
        })?;
        lua.globals().set("print", print_fn)?;

        let keys: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));

        // key_down(key) — true while key is held
        let keys_down = keys.clone();
        let key_down_fn = lua.create_function(move |_, key: String| {
            Ok(keys_down.read().map(|k| k.contains(&key)).unwrap_or(false))
        })?;
        lua.globals().set("key_down", key_down_fn)?;

        // key_pressed(key) — true only on the first frame the key is down
        // Updated each frame via update_key_globals()
        lua.globals().set(
            "key_pressed",
            lua.create_function(|_, _key: String| Ok(false))?,
        )?;

        lua.globals().set(
            "vec2",
            lua.create_function(|lua, (x, y): (f64, f64)| {
                let table = lua.create_table()?;
                table.set("x", x)?;
                table.set("y", y)?;
                table.set(1, x)?;
                table.set(2, y)?;
                Ok(table)
            })?,
        )?;

        let pressed_keys: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));

        Ok(Self {
            lua,
            envs: HashMap::new(),
            started: HashSet::new(),
            missing_scripts: HashSet::new(),
            elapsed: 0.0,
            keys,
            pressed_keys,
            prev_keys: HashSet::new(),
            errors,
            velocities: HashMap::new(),
            physics: PhysicsWorld::new(),
            physics_initialized: false,
        })
    }

    fn report_error(&self, message: impl Into<String>) {
        let message = message.into();
        eprintln!("{message}");
        if let Ok(mut errors) = self.errors.lock() {
            errors.push(message);
            if errors.len() > 50 {
                let excess = errors.len() - 50;
                errors.drain(0..excess);
            }
        }
    }

    fn update_key_globals(&mut self, new_keys: &HashSet<String>) {
        // Normalize raw browser/winit key names to canonical Sindri key names.
        let normalized: HashSet<String> = new_keys.iter().map(|k| normalize_key(k)).collect();
        let prev_normalized: HashSet<String> =
            self.prev_keys.iter().map(|k| normalize_key(k)).collect();

        // Update the shared normalized key set (used by key_down global and input facet)
        if let Ok(mut k) = self.keys.write() {
            *k = normalized.clone();
        }

        // Recompute pressed-this-frame set
        let pressed_this_frame: HashSet<String> =
            normalized.difference(&prev_normalized).cloned().collect();
        if let Ok(mut pk) = self.pressed_keys.write() {
            *pk = pressed_this_frame.clone();
        }

        let kp = self.pressed_keys.clone();
        let _ = self.lua.globals().set(
            "key_pressed",
            self.lua
                .create_function(move |_, key: String| {
                    Ok(kp.read().map(|k| k.contains(&key)).unwrap_or(false))
                })
                .expect("key_pressed fn"),
        );

        self.prev_keys = new_keys.clone();
    }

    /// Load a script into a sandboxed environment for the given (entity, path) pair.
    /// Returns the env table from the registry.
    fn load_env(&mut self, entity_id: u64, path: &str, code: &str) -> anyhow::Result<()> {
        let key = (entity_id, path.to_string());
        if self.envs.contains_key(&key) {
            return Ok(());
        }

        // Create a sandboxed env that falls back to Lua globals for builtins
        let env: Table = self.lua.create_table()?;
        let mt: Table = self.lua.create_table()?;
        mt.set("__index", self.lua.globals())?;
        env.set_metatable(Some(mt));

        match self
            .lua
            .load(code)
            .set_name(path)
            .set_environment(env.clone())
            .exec()
        {
            Ok(_) => {}
            Err(e) => self.report_error(format!("[lua] load error ({path}): {e}")),
        }

        let reg = self.lua.create_registry_value(env)?;
        self.envs.insert(key, reg);
        Ok(())
    }

    fn create_camera_table<'lua>(
        lua: &'lua Lua,
        entity_id: u64,
        camera: &sindri::component::Camera,
        commands: Arc<std::sync::Mutex<Vec<CameraCommand>>>,
    ) -> mlua::Result<Table<'lua>> {
        let table = lua.create_table()?;
        table.set("is_active", {
            let active = camera.active;
            lua.create_function(move |_, _: Table| Ok(active))?
        })?;
        table.set("set_active", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, active): (Table, bool)| {
                if let Ok(mut commands) = commands.lock() {
                    commands.push(CameraCommand::SetActive(active));
                }
                Ok(())
            })?
        })?;
        table.set("zoom", {
            let zoom = camera.zoom;
            lua.create_function(move |_, _: Table| Ok(zoom))?
        })?;
        table.set("set_zoom", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, zoom): (Table, f64)| {
                if let Ok(mut commands) = commands.lock() {
                    commands.push(CameraCommand::SetZoom(zoom as f32));
                }
                Ok(())
            })?
        })?;
        table.set("zoom_to", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, target, speed): (Table, f64, f64)| {
                if let Ok(mut commands) = commands.lock() {
                    commands.push(CameraCommand::ZoomTo {
                        target: target as f32,
                        speed: speed as f32,
                    });
                }
                Ok(())
            })?
        })?;
        table.set("offset", {
            let x = camera.offset_x;
            let y = camera.offset_y;
            lua.create_function(move |lua, _: Table| {
                let offset = lua.create_table()?;
                offset.set("x", x)?;
                offset.set("y", y)?;
                Ok(offset)
            })?
        })?;
        table.set("set_offset", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, offset): (Table, Table)| {
                let x = offset.get::<_, f64>("x").or_else(|_| offset.get(1))?;
                let y = offset.get::<_, f64>("y").or_else(|_| offset.get(2))?;
                if let Ok(mut commands) = commands.lock() {
                    commands.push(CameraCommand::SetOffset {
                        x: x as f32,
                        y: y as f32,
                    });
                }
                Ok(())
            })?
        })?;
        table.set("set_bounds", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, min, max): (Table, Table, Table)| {
                let min_x = min.get::<_, f64>("x").or_else(|_| min.get(1))?;
                let min_y = min.get::<_, f64>("y").or_else(|_| min.get(2))?;
                let max_x = max.get::<_, f64>("x").or_else(|_| max.get(1))?;
                let max_y = max.get::<_, f64>("y").or_else(|_| max.get(2))?;
                if let Ok(mut commands) = commands.lock() {
                    commands.push(CameraCommand::SetBounds {
                        min_x: min_x as f32,
                        min_y: min_y as f32,
                        max_x: max_x as f32,
                        max_y: max_y as f32,
                    });
                }
                Ok(())
            })?
        })?;
        table.set("clear_bounds", {
            let commands = commands.clone();
            lua.create_function(move |_, _: Table| {
                if let Ok(mut commands) = commands.lock() {
                    commands.push(CameraCommand::ClearBounds);
                }
                Ok(())
            })?
        })?;
        table.set("shake", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, intensity, duration): (Table, f64, f64)| {
                if let Ok(mut commands) = commands.lock() {
                    commands.push(CameraCommand::Shake {
                        intensity: intensity as f32,
                        duration: duration as f32,
                    });
                }
                Ok(())
            })?
        })?;
        table.set("entity_id", entity_id)?;
        Ok(table)
    }

    fn create_transform_table<'lua>(
        lua: &'lua Lua,
        transform: &sindri::component::Transform,
        commands: Arc<std::sync::Mutex<Vec<SceneCommand>>>,
    ) -> mlua::Result<Table<'lua>> {
        let table = lua.create_table()?;
        table.set("x", transform.x)?;
        table.set("y", transform.y)?;
        table.set("rotation_value", transform.rotation)?;
        table.set("scale_x", transform.scale_x)?;
        table.set("scale_y", transform.scale_y)?;
        table.set("position", {
            let x = transform.x;
            let y = transform.y;
            lua.create_function(move |lua, _: Table| {
                let position = lua.create_table()?;
                position.set("x", x)?;
                position.set("y", y)?;
                position.set(1, x)?;
                position.set(2, y)?;
                Ok(position)
            })?
        })?;
        table.set("set_position", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, position): (Table, Table)| {
                let (x, y) = read_vec2(&position)?;
                if let Ok(mut commands) = commands.lock() {
                    commands.push(SceneCommand::SetTransformPosition { x, y });
                }
                Ok(())
            })?
        })?;
        table.set("rotation", {
            let rotation = transform.rotation;
            lua.create_function(move |_, _: Table| Ok(rotation))?
        })?;
        table.set("set_rotation", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, rotation): (Table, f64)| {
                if let Ok(mut commands) = commands.lock() {
                    commands.push(SceneCommand::SetTransformRotation(rotation as f32));
                }
                Ok(())
            })?
        })?;
        table.set("scale", {
            let x = transform.scale_x;
            let y = transform.scale_y;
            lua.create_function(move |lua, _: Table| {
                let scale = lua.create_table()?;
                scale.set("x", x)?;
                scale.set("y", y)?;
                scale.set(1, x)?;
                scale.set(2, y)?;
                Ok(scale)
            })?
        })?;
        table.set("set_scale", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, scale): (Table, Table)| {
                let (x, y) = read_vec2(&scale)?;
                if let Ok(mut commands) = commands.lock() {
                    commands.push(SceneCommand::SetTransformScale { x, y });
                }
                Ok(())
            })?
        })?;
        Ok(table)
    }

    fn create_sprite_table<'lua>(
        lua: &'lua Lua,
        sprite: &sindri::component::Sprite,
        commands: Arc<std::sync::Mutex<Vec<SceneCommand>>>,
    ) -> mlua::Result<Table<'lua>> {
        let table = lua.create_table()?;
        table.set("texture_path", sprite.texture_path.clone())?;
        table.set("width", sprite.width)?;
        table.set("height", sprite.height)?;
        table.set("tint", {
            let color = sprite.color;
            lua.create_function(move |lua, _: Table| {
                let tint = lua.create_table()?;
                tint.set("r", color[0])?;
                tint.set("g", color[1])?;
                tint.set("b", color[2])?;
                tint.set("a", color[3])?;
                tint.set(1, color[0])?;
                tint.set(2, color[1])?;
                tint.set(3, color[2])?;
                tint.set(4, color[3])?;
                Ok(tint)
            })?
        })?;
        table.set("set_tint", {
            let commands = commands.clone();
            lua.create_function(move |_, (_this, tint): (Table, Table)| {
                let color = read_color(&tint)?;
                if let Ok(mut commands) = commands.lock() {
                    commands.push(SceneCommand::SetSpriteTint(color));
                }
                Ok(())
            })?
        })?;
        Ok(table)
    }

    /// Called when play starts — clears cached envs and started set so scripts restart cleanly.
    pub fn reset(&mut self) {
        self.envs.clear();
        self.started.clear();
        self.missing_scripts.clear();
        self.elapsed = 0.0;
        self.prev_keys.clear();
        self.velocities.clear();
        self.physics = PhysicsWorld::new();
        self.physics_initialized = false;
        if let Ok(mut k) = self.keys.write() {
            k.clear();
        }
        if let Ok(mut k) = self.pressed_keys.write() {
            k.clear();
        }
    }

    /// Step physics simulation and sync positions back to scene transforms.
    /// First call after reset initialises the Rapier world from the scene.
    pub fn step_physics(&mut self, scene: &mut Scene, gravity_x: f32, gravity_y: f32, dt: f32) {
        if !self.physics_initialized {
            self.physics = PhysicsWorld::with_gravity(Vec2::new(gravity_x, gravity_y));
            self.init_physics_from_scene(scene);
            self.physics_initialized = true;
        }

        self.physics.step(dt);

        // Sync Rapier positions → scene transforms for non-fixed bodies
        for (entity_id, entity) in scene.entities.iter_mut() {
            let eid = EntityId(*entity_id as u32);
            let Some(pos) = self.physics.body_position(eid) else { continue };
            if matches!(self.physics.body_type(eid), Some(PhysicsBodyType::Fixed)) {
                continue;
            }
            for comp in &mut entity.components {
                if let Component::Transform(t) = comp {
                    t.x = pos.x;
                    t.y = pos.y;
                    if let Some(rot) = self.physics.body_rotation(eid) {
                        t.rotation = rot;
                    }
                    break;
                }
            }
            if let Some(vel) = self.physics.linear_velocity(eid) {
                self.velocities.insert(*entity_id, (vel.x, vel.y));
            }
        }
    }

    fn init_physics_from_scene(&mut self, scene: &Scene) {
        for (entity_id, entity) in &scene.entities {
            let Some(transform) = entity.components.iter().find_map(|c| {
                if let Component::Transform(t) = c { Some(t) } else { None }
            }) else { continue };

            let Some(collider) = entity.components.iter().find_map(|c| {
                if let Component::Collider(col) = c { Some(col) } else { None }
            }) else { continue };

            let physics_body = entity.components.iter().find_map(|c| {
                if let Component::PhysicsBody(p) = c { Some(p) } else { None }
            });
            let has_script = entity.components.iter().any(|c| matches!(c, Component::Script(_)));

            // Only register in Rapier if explicitly dynamic, or if it's a static object (no script)
            if physics_body.is_none() && has_script { continue; }

            let body_type = physics_body.map(|p| match p.body_type {
                BodyType::Dynamic => PhysicsBodyType::Dynamic,
                BodyType::Kinematic => PhysicsBodyType::Kinematic,
                BodyType::Fixed => PhysicsBodyType::Fixed,
            }).unwrap_or(PhysicsBodyType::Fixed);

            let eid = EntityId(*entity_id as u32);
            let pos = Vec2::new(transform.x, transform.y);
            if self.physics.create_body(eid, body_type, pos, transform.rotation).is_err() {
                continue;
            }

            if let Some(pb) = physics_body {
                if pb.lock_rotation {
                    self.physics.lock_rotations(eid, true);
                }
                self.physics.set_linear_damping(eid, pb.linear_damping);
                self.physics.set_angular_damping(eid, pb.angular_damping);
                self.physics.set_collision_mask(eid, pb.collision_layer, pb.collision_mask);
            }

            let shape = ColliderShape::Box {
                hx: collider.width * 0.5,
                hy: collider.height * 0.5,
            };
            let offset = Vec2::new(collider.offset_x, collider.offset_y);
            if collider.is_trigger {
                let _ = self.physics.add_sensor(eid, shape, offset);
            } else {
                let _ = self.physics.add_collider_with_material(eid, shape, offset, 1.0, 0.3, 0.0);
            }
        }
    }

    pub fn update(
        &mut self,
        scene: &mut Scene,
        scripts_root: &Path,
        dt: f32,
        keys: &HashSet<String>,
    ) {
        self.update_key_globals(keys);
        self.elapsed += dt as f64;
        let elapsed = self.elapsed;

        // Collect entity ids to avoid borrow issues
        let entity_ids: Vec<u64> = scene.entities.keys().cloned().collect();

        for entity_id in entity_ids {
            let Some(entity) = scene.entities.get(&entity_id) else {
                continue;
            };

            // Collect script paths and current transform
            let script_paths: Vec<String> = entity
                .components
                .iter()
                .filter_map(|c| {
                    if let Component::Script(s) = c {
                        Some(s.path.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if script_paths.is_empty() {
                continue;
            }

            let transform = entity.components.iter().find_map(|c| {
                if let Component::Transform(t) = c {
                    Some(t.clone())
                } else {
                    None
                }
            });
            let camera = entity.components.iter().find_map(|c| {
                if let Component::Camera(camera) = c {
                    Some(camera.clone())
                } else {
                    None
                }
            });
            let sprite = entity.components.iter().find_map(|c| {
                if let Component::Sprite(sprite) = c {
                    Some(sprite.clone())
                } else {
                    None
                }
            });
            let has_physics = entity.components.iter().any(|c| matches!(c, Component::PhysicsBody(_)));

            for script_path in script_paths {
                if script_path.is_empty() {
                    continue;
                }

                let full_path = resolve_script_path(scripts_root, &script_path);
                if !full_path.exists() {
                    let missing_key = (entity_id, script_path.clone());
                    if self.missing_scripts.insert(missing_key) {
                        eprintln!(
                            "[lua] script not found ({script_path}): {}",
                            full_path.display()
                        );
                    }
                    continue;
                }

                let code = match std::fs::read_to_string(&full_path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if let Err(e) = self.load_env(entity_id, &script_path, &code) {
                    self.report_error(format!("[lua] env error: {e}"));
                    continue;
                }

                let key = (entity_id, script_path.clone());
                let env: Table = match self
                    .envs
                    .get(&key)
                    .and_then(|rk| self.lua.registry_value::<Table>(rk).ok())
                {
                    Some(t) => t,
                    None => continue,
                };

                // Build self table with entity state
                let self_tbl: Table = match self.lua.create_table() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let _ = self_tbl.set("entity_id", entity_id);
                let _ = self_tbl.set("elapsed", elapsed);
                if let Some(ref t) = transform {
                    let _ = self_tbl.set("x", t.x as f64);
                    let _ = self_tbl.set("y", t.y as f64);
                    let _ = self_tbl.set("rotation", t.rotation as f64);
                    let _ = self_tbl.set("scale_x", t.scale_x as f64);
                    let _ = self_tbl.set("scale_y", t.scale_y as f64);
                } else {
                    let _ = self_tbl.set("x", 0.0f64);
                    let _ = self_tbl.set("y", 0.0f64);
                    let _ = self_tbl.set("rotation", 0.0f64);
                    let _ = self_tbl.set("scale_x", 1.0f64);
                    let _ = self_tbl.set("scale_y", 1.0f64);
                }
                let camera_commands: Arc<std::sync::Mutex<Vec<CameraCommand>>> =
                    Arc::new(std::sync::Mutex::new(Vec::new()));
                let scene_commands: Arc<std::sync::Mutex<Vec<SceneCommand>>> =
                    Arc::new(std::sync::Mutex::new(Vec::new()));
                if let Some(transform) = transform.clone() {
                    let commands = scene_commands.clone();
                    match self.lua.create_function(move |lua, _: mlua::MultiValue| {
                        Self::create_transform_table(lua, &transform, commands.clone()).map(Some)
                    }) {
                        Ok(transform_fn) => {
                            let _ = self_tbl.set("transform", transform_fn);
                        }
                        Err(e) => self.report_error(format!(
                            "[lua] transform method error ({script_path}): {e}"
                        )),
                    }
                } else {
                    match self.lua.create_function(|_, _: mlua::MultiValue| {
                        Ok::<Option<Table>, mlua::Error>(None)
                    }) {
                        Ok(transform_fn) => {
                            let _ = self_tbl.set("transform", transform_fn);
                        }
                        Err(e) => self.report_error(format!(
                            "[lua] transform method error ({script_path}): {e}"
                        )),
                    }
                }
                if let Some(sprite) = sprite.clone() {
                    let commands = scene_commands.clone();
                    match self.lua.create_function(move |lua, _: mlua::MultiValue| {
                        Self::create_sprite_table(lua, &sprite, commands.clone()).map(Some)
                    }) {
                        Ok(sprite_fn) => {
                            let _ = self_tbl.set("sprite", sprite_fn);
                        }
                        Err(e) => self.report_error(format!(
                            "[lua] sprite method error ({script_path}): {e}"
                        )),
                    }
                } else {
                    match self.lua.create_function(|_, _: mlua::MultiValue| {
                        Ok::<Option<Table>, mlua::Error>(None)
                    }) {
                        Ok(sprite_fn) => {
                            let _ = self_tbl.set("sprite", sprite_fn);
                        }
                        Err(e) => self.report_error(format!(
                            "[lua] sprite method error ({script_path}): {e}"
                        )),
                    }
                }
                // input() facet — wraps key_down/key_pressed with normalized key names
                {
                    let keys_for_input = self.keys.clone();
                    let pressed_for_input = self.pressed_keys.clone();
                    match self.lua.create_function(move |lua, _: mlua::MultiValue| {
                        let tbl = lua.create_table()?;
                        let kd = keys_for_input.clone();
                        tbl.set("is_key_down", lua.create_function(move |_, (_this, key): (Table, String)| {
                            Ok(kd.read().map(|k| k.contains(&key)).unwrap_or(false))
                        })?)?;
                        let kp = pressed_for_input.clone();
                        tbl.set("is_key_pressed", lua.create_function(move |_, (_this, key): (Table, String)| {
                            Ok(kp.read().map(|k| k.contains(&key)).unwrap_or(false))
                        })?)?;
                        let kd2 = keys_for_input.clone();
                        tbl.set("axis", lua.create_function(move |_, (_this, neg, pos): (Table, String, String)| {
                            let held = kd2.read().map(|k| k.clone()).unwrap_or_default();
                            let n = if held.contains(&neg) { -1.0f64 } else { 0.0 };
                            let p = if held.contains(&pos) { 1.0f64 } else { 0.0 };
                            Ok((n + p).clamp(-1.0, 1.0))
                        })?)?;
                        Ok(tbl)
                    }) {
                        Ok(f) => { let _ = self_tbl.set("input", f); }
                        Err(e) => self.report_error(format!("[lua] input method error ({script_path}): {e}")),
                    }
                }

                // physics() facet — velocity-based movement with gravity
                let vel_cell: Arc<std::sync::Mutex<(f32, f32)>> = Arc::new(std::sync::Mutex::new(
                    self.velocities.get(&entity_id).copied().unwrap_or((0.0, 0.0)),
                ));
                {
                    let vel_for_physics = vel_cell.clone();
                    let phys_fn = if has_physics {
                        self.lua.create_function(move |lua, _: mlua::MultiValue| {
                            let tbl = lua.create_table()?;
                            let vc = vel_for_physics.clone();
                            tbl.set("velocity", lua.create_function(move |lua, _: Table| {
                                let (vx, vy) = *vc.lock().unwrap();
                                let v = lua.create_table()?;
                                v.set("x", vx as f64)?;
                                v.set("y", vy as f64)?;
                                Ok(v)
                            })?)?;
                            let vc = vel_for_physics.clone();
                            tbl.set("set_velocity", lua.create_function(move |_, (_this, v): (Table, Table)| {
                                let (vx, vy) = read_vec2(&v)?;
                                *vc.lock().unwrap() = (vx, vy);
                                Ok(())
                            })?)?;
                            let vc = vel_for_physics.clone();
                            tbl.set("apply_impulse", lua.create_function(move |_, (_this, v): (Table, Table)| {
                                let (ix, iy) = read_vec2(&v)?;
                                let mut vel = vc.lock().unwrap();
                                vel.0 += ix;
                                vel.1 += iy;
                                Ok(())
                            })?)?;
                            Ok(Some(tbl))
                        })
                    } else {
                        self.lua.create_function(|_, _: mlua::MultiValue| {
                            Ok::<Option<Table>, mlua::Error>(None)
                        })
                    };
                    match phys_fn {
                        Ok(f) => { let _ = self_tbl.set("physics", f); }
                        Err(e) => self.report_error(format!("[lua] physics method error ({script_path}): {e}")),
                    }
                }

                // camera() facet
                if let Some(camera) = camera.clone() {
                    let commands = camera_commands.clone();
                    match self.lua.create_function(move |lua, _: mlua::MultiValue| {
                        Self::create_camera_table(lua, entity_id, &camera, commands.clone())
                            .map(Some)
                    }) {
                        Ok(camera_fn) => { let _ = self_tbl.set("camera", camera_fn); }
                        Err(e) => self.report_error(format!(
                            "[lua] camera method error ({script_path}): {e}"
                        )),
                    }
                } else {
                    match self.lua.create_function(|_, _: mlua::MultiValue| {
                        Ok::<Option<Table>, mlua::Error>(None)
                    }) {
                        Ok(camera_fn) => { let _ = self_tbl.set("camera", camera_fn); }
                        Err(e) => self.report_error(format!(
                            "[lua] camera method error ({script_path}): {e}"
                        )),
                    }
                }

                // on_start (once per script path per play session)
                if !self.started.contains(&key) {
                    if let Ok(f) = env.get::<_, mlua::Function>("on_start") {
                        if let Err(e) = f.call::<_, ()>(self_tbl.clone()) {
                            self.report_error(format!("[lua] on_start error ({script_path}): {e}"));
                        }
                    }
                    self.started.insert(key.clone());
                }

                // on_update
                if let Ok(f) = env.get::<_, mlua::Function>("on_update") {
                    if let Err(e) = f.call::<_, ()>((self_tbl.clone(), dt as f64)) {
                        self.report_error(format!("[lua] on_update error ({script_path}): {e}"));
                    }
                }

                // Write transform back (read legacy self.x/y direct-field access)
                let read_f64 = |t: &Table, k: &str, default: f32| -> f32 {
                    t.get::<_, f64>(k).map(|v| v as f32).unwrap_or(default)
                };
                let (ox, oy, orot, osx, osy) = transform
                    .as_ref()
                    .map(|t| (t.x, t.y, t.rotation, t.scale_x, t.scale_y))
                    .unwrap_or((0.0, 0.0, 0.0, 1.0, 1.0));

                let mut nx = read_f64(&self_tbl, "x", ox);
                let mut ny = read_f64(&self_tbl, "y", oy);
                let nr = read_f64(&self_tbl, "rotation", orot);
                let nsx = read_f64(&self_tbl, "scale_x", osx);
                let nsy = read_f64(&self_tbl, "scale_y", osy);

                if has_physics {
                    if let Ok(vel) = vel_cell.lock() {
                        let (new_vx, new_vy) = *vel;
                        let eid = EntityId(entity_id as u32);
                        if self.physics_initialized && self.physics.has_body(eid) {
                            // Rapier controls position; sync script velocity changes to it
                            self.physics.set_linear_velocity(eid, Vec2::new(new_vx, new_vy));
                        } else {
                            // Fallback: manual velocity integration (no Rapier body)
                            let (old_vx, old_vy) = self.velocities.get(&entity_id).copied().unwrap_or((0.0, 0.0));
                            let dx = (new_vx - old_vx) * dt;
                            let dy = (new_vy - old_vy) * dt;
                            if dx != 0.0 || dy != 0.0 {
                                nx += dx;
                                ny += dy;
                            }
                        }
                        self.velocities.insert(entity_id, (new_vx, new_vy));
                    }
                }

                if let Some(entity_mut) = scene.entities.get_mut(&entity_id) {
                    if let Some(t) = entity_mut.components.iter_mut().find_map(|c| {
                        if let Component::Transform(t) = c { Some(t) } else { None }
                    }) {
                        let eid = EntityId(entity_id as u32);
                        let rapier_dynamic = self.physics_initialized
                            && self.physics.has_body(eid)
                            && !matches!(self.physics.body_type(eid), Some(PhysicsBodyType::Fixed));

                        if rapier_dynamic {
                            // Rapier already updated x/y via step_physics; only apply scale from script
                            t.scale_x = nsx;
                            t.scale_y = nsy;
                        } else {
                            t.x = nx;
                            t.y = ny;
                            t.rotation = nr;
                            t.scale_x = nsx;
                            t.scale_y = nsy;
                        }
                    }
                }

                let camera_commands = camera_commands
                    .lock()
                    .map(|mut commands| std::mem::take(&mut *commands))
                    .unwrap_or_default();
                if !camera_commands.is_empty() {
                    apply_camera_commands(scene, entity_id, camera_commands);
                }

                let scene_commands = scene_commands
                    .lock()
                    .map(|mut commands| std::mem::take(&mut *commands))
                    .unwrap_or_default();
                if !scene_commands.is_empty() {
                    apply_scene_commands(scene, entity_id, scene_commands);
                }
            }
        }
    }

    /// Drop cached state for a removed entity so scripts restart if it's re-added.
    #[allow(dead_code)]
    pub fn evict(&mut self, entity_id: u64) {
        self.envs.retain(|(id, _), _| *id != entity_id);
        self.started.retain(|(id, _)| *id != entity_id);
        self.missing_scripts.retain(|(id, _)| *id != entity_id);
    }
}
