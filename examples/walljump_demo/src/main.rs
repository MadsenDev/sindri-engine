use anyhow::Result;
use forge2d::{
    Camera2D, Engine, EngineContext, Game, KeyCode, Vec2,
    camera::active_camera,
    physics::{ColliderShape, PhysicsWorld, RigidBodyType},
    render::TextureHandle,
    script::{ScriptComponent, ScriptParams, ScriptRuntime, ScriptTag},
    CameraComponent, SpriteComponent, Transform,
    ParticleEmitter, ParticleSystem, EmissionConfig,
};
use std::sync::{Arc, Mutex};

struct WallJumpDemo {
    runtime: ScriptRuntime,
    camera: Camera2D,
    camera_entity: Option<forge2d::EntityId>,
    physics: PhysicsWorld,
    world: forge2d::World,

    player_entity: forge2d::EntityId,
    player_texture: Option<TextureHandle>,
    block_texture: Option<TextureHandle>,
    particle_texture: Option<TextureHandle>,
    particle_system: ParticleSystem,

    player_half_height: f32,
    respawn_point: Vec2,
    respawn_pending: bool,
    respawn_now: bool,
    fade_alpha: f32,
    fade_dir: f32,

    script_bridge: Arc<Mutex<ScriptBridge>>,
}

#[derive(Clone, Copy)]
enum ScriptEffect {
    Landing,
    WallSlide,
    WallJump,
}

struct ScriptBridge {
    effects: Vec<(ScriptEffect, Vec2)>,
    respawn_requested: bool,
    respawn_point: Option<Vec2>,
    shake_request: Option<(f32, f32)>,
}

impl ScriptBridge {
    fn new() -> Self {
        Self {
            effects: Vec::new(),
            respawn_requested: false,
            respawn_point: None,
            shake_request: None,
        }
    }
}

impl WallJumpDemo {
    fn new() -> Result<Self> {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(Vec2::new(0.0, 900.0));

        let script_bridge = Arc::new(Mutex::new(ScriptBridge::new()));
        Ok(Self {
            runtime: ScriptRuntime::new()?,
            camera: Camera2D::default(),
            camera_entity: None,
            physics,
            world: forge2d::World::new(),
            player_entity: unsafe { std::mem::zeroed() },
            player_texture: None,
            block_texture: None,
            particle_texture: None,
            particle_system: ParticleSystem::new(),
            player_half_height: 28.0,
            respawn_point: Vec2::new(220.0, 320.0),
            respawn_pending: false,
            respawn_now: false,
            fade_alpha: 0.0,
            fade_dir: -1.0,
            script_bridge,
        })
    }

    fn create_textures(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let player_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [80u8, 220, 140, 255])
            .collect();
        let block_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [45u8, 52, 70, 255])
            .collect();

        let player = ctx.renderer().load_texture_from_rgba(&player_data, 32, 32)?;
        let block = ctx.renderer().load_texture_from_rgba(&block_data, 32, 32)?;
        let white_pixel = [255u8, 255u8, 255u8, 255u8];
        let particle = ctx.renderer().load_texture_from_rgba(&white_pixel, 1, 1)?;

        self.player_texture = Some(player);
        self.block_texture = Some(block);
        self.particle_texture = Some(particle);
        Ok(())
    }

    fn register_script_api(&mut self) -> Result<()> {
        let bridge = Arc::clone(&self.script_bridge);
        self.runtime.register_function(
            "emit_effect",
            move |_, (name, x, y): (String, f64, f64)| {
                let mut bridge = bridge.lock().unwrap();
                let effect = match name.as_str() {
                    "landing" => ScriptEffect::Landing,
                    "wall_slide" => ScriptEffect::WallSlide,
                    "wall_jump" => ScriptEffect::WallJump,
                    _ => return Ok(()),
                };
                bridge.effects.push((effect, Vec2::new(x as f32, y as f32)));
                Ok(())
            },
        )?;

        let bridge = Arc::clone(&self.script_bridge);
        self.runtime
            .register_function("request_respawn", move |_, (): ()| {
                let mut bridge = bridge.lock().unwrap();
                bridge.respawn_requested = true;
                Ok(())
            })?;

        let bridge = Arc::clone(&self.script_bridge);
        self.runtime.register_function(
            "set_respawn_point",
            move |_, (x, y): (f64, f64)| {
                let mut bridge = bridge.lock().unwrap();
                bridge.respawn_point = Some(Vec2::new(x as f32, y as f32));
                Ok(())
            },
        )?;

        let bridge = Arc::clone(&self.script_bridge);
        self.runtime.register_function(
            "shake_camera",
            move |_, (intensity, duration): (f64, f64)| {
                let mut bridge = bridge.lock().unwrap();
                bridge.shake_request = Some((intensity as f32, duration as f32));
                Ok(())
            },
        )?;

        Ok(())
    }

    fn spawn_static_block(
        &mut self,
        texture: TextureHandle,
        position: Vec2,
        size: Vec2,
        tag: &'static str,
    ) -> Result<forge2d::EntityId> {
        let entity = self.world.spawn();
        self.world.insert(entity, Transform::new(position));
        self.world.insert(entity, ScriptTag(tag.to_string()));
        self.physics.create_body(entity, RigidBodyType::Fixed, position, 0.0)?;
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box {
                hx: size.x * 0.5,
                hy: size.y * 0.5,
            },
            Vec2::ZERO,
            0.0,
            0.8,
            0.0,
        )?;

        let mut sprite = SpriteComponent::new(texture);
        sprite
            .sprite
            .set_size_px(size, Vec2::new(32.0, 32.0));
        sprite.sprite.transform.position = position;
        sprite.sprite.tint = [0.2, 0.3, 0.45, 1.0];
        sprite.sprite.is_occluder = false;
        self.world.insert(entity, sprite);

        Ok(entity)
    }

    fn spawn_player(&mut self) -> Result<()> {
        let texture = self.player_texture.expect("player texture missing");
        let entity = self.world.spawn();
        self.player_entity = entity;
        self.world.insert(entity, Transform::new(self.respawn_point));
        self.world.insert(entity, ScriptTag("player".to_string()));
        let script_path = format!(
            "{}/scripts/walljump_player.lua",
            env!("CARGO_MANIFEST_DIR")
        );
        let params = ScriptParams::default()
            .insert("move_speed", 260.0)
            .insert("jump_impulse", 620.0)
            .insert("wall_jump_impulse", 520.0)
            .insert("jump_cut", 0.3)
            .insert("jump_max_hold", 0.18)
            .insert("jump_hold_boost", 0.45)
            .insert("wall_jump_hold_boost", 0.35)
            .insert("hang_time", 0.07)
            .insert("hang_speed", 30.0)
            .insert("fall_speed", 900.0)
            .insert("wall_jump_x", 320.0)
            .insert("wall_slide_speed", 80.0)
            .insert("wall_slide_interval", 0.04)
            .insert("wall_stamina_max", 1.0)
            .insert("coyote_time", 0.12)
            .insert("jump_buffer", 0.15)
            .insert("drop_through_time", 0.18)
            .insert("half_height", self.player_half_height)
            .insert("respawn_y", 740.0);
        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script_path, params),
        );

        let start_pos = self.respawn_point;
        self.physics
            .create_body(entity, RigidBodyType::Dynamic, start_pos, 0.0)?;
        self.physics.lock_rotations(entity, true);
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::CapsuleY {
                half_height: 18.0,
                radius: 10.0,
            },
            Vec2::ZERO,
            1.0,
            0.2,
            0.0,
        )?;

        // Match sprite size to capsule collider so it doesn't drift into the ground.
        let mut sprite = SpriteComponent::new(texture);
        sprite
            .sprite
            .set_size_px(Vec2::new(20.0, 56.0), Vec2::new(32.0, 32.0));
        sprite.sprite.transform.position = start_pos;
        sprite.sprite.tint = [0.3, 1.0, 0.6, 1.0];
        sprite.sprite.is_occluder = false;
        self.world.insert(entity, sprite);
        Ok(())
    }

    fn spawn_camera(&mut self) {
        let entity = self.world.spawn();
        let start_pos = self
            .physics
            .body_position(self.player_entity)
            .unwrap_or(self.respawn_point);
        self.world.insert(entity, Transform::new(start_pos));
        self.world.insert(entity, CameraComponent::new(Vec2::ZERO));
        let script_path = format!(
            "{}/scripts/walljump_camera.lua",
            env!("CARGO_MANIFEST_DIR")
        );
        let params = ScriptParams::default()
            .insert("look_ahead", 0.0)
            .insert("look_ahead_y", 0.0)
            .insert("dead_zone_x", 60.0)
            .insert("dead_zone_y", 30.0)
            .insert("smooth", 0.08)
            .insert("zoom", 1.12);
        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script_path, params),
        );
        self.camera_entity = Some(entity);
    }

    fn spawn_one_way_platform(
        &mut self,
        texture: TextureHandle,
        position: Vec2,
        size: Vec2,
    ) -> Result<forge2d::EntityId> {
        let entity = self.world.spawn();
        self.world.insert(entity, Transform::new(position));
        self.world.insert(entity, ScriptTag("one_way".to_string()));
        self.physics.create_body(entity, RigidBodyType::Fixed, position, 0.0)?;
        self.physics.add_sensor(
            entity,
            ColliderShape::Box {
                hx: size.x * 0.5,
                hy: size.y * 0.5,
            },
            Vec2::ZERO,
        )?;

        let mut sprite = SpriteComponent::new(texture);
        sprite
            .sprite
            .set_size_px(size, Vec2::new(32.0, 32.0));
        sprite.sprite.transform.position = position;
        sprite.sprite.tint = [0.18, 0.26, 0.4, 1.0];
        sprite.sprite.is_occluder = false;
        self.world.insert(entity, sprite);
        Ok(entity)
    }

    fn spawn_checkpoint_block(
        &mut self,
        texture: TextureHandle,
        position: Vec2,
        size: Vec2,
    ) -> Result<forge2d::EntityId> {
        let entity = self.spawn_static_block(texture, position, size, "checkpoint")?;
        if let Some(sprite) = self.world.get_mut::<SpriteComponent>(entity) {
            sprite.sprite.tint = [0.3, 0.9, 0.7, 1.0];
        }
        Ok(entity)
    }

    fn apply_script_bridge(&mut self) {
        let mut bridge = self.script_bridge.lock().unwrap();
        let effects = std::mem::take(&mut bridge.effects);
        let respawn_requested = bridge.respawn_requested;
        let respawn_point = bridge.respawn_point.take();
        let shake_request = bridge.shake_request.take();
        bridge.respawn_requested = false;
        drop(bridge);

        if let Some(point) = respawn_point {
            self.respawn_point = point;
        }

        if respawn_requested && !self.respawn_pending {
            self.respawn_pending = true;
            self.fade_dir = 1.0;
        }

        for (effect, pos) in effects {
            match effect {
                ScriptEffect::Landing => self.spawn_landing_burst(pos),
                ScriptEffect::WallSlide => self.spawn_wall_slide_dust(pos),
                ScriptEffect::WallJump => self.spawn_wall_jump_burst(pos),
            }
        }

        if let Some((intensity, duration)) = shake_request {
            if let Some(entity) = self.camera_entity {
                if let Some(cam) = self.world.get_mut::<CameraComponent>(entity) {
                    cam.camera.shake(intensity, duration);
                }
            }
        }
    }

    fn sync_transforms_from_physics(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<Transform>()
            .iter()
            .map(|(entity, _)| *entity)
            .collect();
        for entity in entities {
            if let Some(pos) = self.physics.body_position(entity) {
                if let Some(transform) = self.world.get_mut::<Transform>(entity) {
                    transform.position = pos;
                    if let Some(rot) = self.physics.body_rotation(entity) {
                        transform.rotation = rot;
                    }
                }
            }
        }
    }

    fn sync_sprite_transforms(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<SpriteComponent>()
            .iter()
            .map(|(entity, _)| *entity)
            .collect();
        for entity in entities {
            let transform = self.world.get::<Transform>(entity).cloned();
            if let (Some(transform), Some(sprite)) = (transform, self.world.get_mut::<SpriteComponent>(entity)) {
                sprite.sprite.transform.position = transform.position;
                sprite.sprite.transform.rotation = transform.rotation;
            }
        }
    }

    fn rebuild_level(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.camera_entity = None;
        self.respawn_pending = false;
        self.respawn_now = false;
        self.fade_alpha = 0.0;
        self.fade_dir = -1.0;
        if let Ok(mut bridge) = self.script_bridge.lock() {
            bridge.effects.clear();
            bridge.respawn_requested = false;
            bridge.respawn_point = None;
            bridge.shake_request = None;
        }

        let block = self.block_texture.expect("block texture missing");

        self.spawn_static_block(
            block,
            Vec2::new(480.0, 520.0),
            Vec2::new(960.0, 40.0),
            "ground",
        )?;

        self.spawn_static_block(
            block,
            Vec2::new(40.0, 300.0),
            Vec2::new(60.0, 520.0),
            "wall",
        )?;
        self.spawn_static_block(
            block,
            Vec2::new(920.0, 300.0),
            Vec2::new(60.0, 520.0),
            "wall",
        )?;

        self.spawn_one_way_platform(
            block,
            Vec2::new(300.0, 380.0),
            Vec2::new(220.0, 28.0),
        )?;
        self.spawn_one_way_platform(
            block,
            Vec2::new(640.0, 320.0),
            Vec2::new(260.0, 28.0),
        )?;
        self.spawn_one_way_platform(
            block,
            Vec2::new(460.0, 240.0),
            Vec2::new(180.0, 24.0),
        )?;

        self.spawn_checkpoint_block(
            block,
            Vec2::new(760.0, 440.0),
            Vec2::new(140.0, 24.0),
        )?;
        self.respawn_point = Vec2::new(220.0, 320.0);
        self.spawn_player()?;
        self.spawn_camera();
        self.camera = ctx.screen_camera();
        Ok(())
    }
}

impl Game for WallJumpDemo {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.register_script_api()?;
        self.create_textures(ctx)?;
        self.rebuild_level(ctx)?;
        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), 0.0)?;
        Ok(())
    }

    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.fixed_delta_seconds();
        self.runtime
            .fixed_update(&mut self.world, &mut self.physics, ctx.input(), dt)?;

        self.physics.step(dt);

        let events = self.physics.drain_events();
        self.runtime.handle_physics_events(
            &events,
            &mut self.world,
            &mut self.physics,
            ctx.input(),
        )?;

        self.runtime.post_physics_update(
            &mut self.world,
            &mut self.physics,
            ctx.input(),
            dt,
        )?;

        self.apply_script_bridge();
        self.sync_transforms_from_physics();

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_seconds();
        self.particle_system.update(dt);
        self.fade_alpha = (self.fade_alpha + self.fade_dir * dt * 2.5).clamp(0.0, 1.0);
        if self.respawn_pending && self.fade_alpha >= 1.0 {
            self.respawn_now = true;
        }
        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), dt)?;
        self.apply_script_bridge();

        if let Some(cam) = active_camera(&mut self.world, dt) {
            self.camera = cam;
        }

        if self.respawn_now {
            let pos = self.respawn_point;
            self.physics.set_body_position(self.player_entity, pos);
            self.physics.set_linear_velocity(self.player_entity, Vec2::ZERO);
            if let Some(transform) = self.world.get_mut::<Transform>(self.player_entity) {
                transform.position = pos;
            }
            self.respawn_now = false;
            self.respawn_pending = false;
            self.fade_dir = -1.0;
        }

        if ctx.input().is_key_pressed(KeyCode::KeyR) {
            self.physics.clear();
            self.world = forge2d::World::new();
            self.rebuild_level(ctx)?;
            self.runtime
                .update(&mut self.world, &mut self.physics, ctx.input(), 0.0)?;
        }

        self.sync_sprite_transforms();
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let screen_camera = ctx.screen_camera();
        let screen_size = ctx.window().inner_size();
        let fade_alpha = self.fade_alpha;
        ctx.draw(|renderer, frame| {
            renderer.clear(frame, [0.06, 0.08, 0.12, 1.0])?;

            let sprites: Vec<_> = self.world.query::<SpriteComponent>();
            for (entity, sprite) in &sprites {
                let is_player = self
                    .world
                    .get::<ScriptTag>(*entity)
                    .map(|tag| tag.0 == "player")
                    .unwrap_or(false);
                if sprite.visible && !is_player {
                    renderer.draw_sprite(frame, &sprite.sprite, &self.camera)?;
                }
            }
            for (entity, sprite) in &sprites {
                let is_player = self
                    .world
                    .get::<ScriptTag>(*entity)
                    .map(|tag| tag.0 == "player")
                    .unwrap_or(false);
                if sprite.visible && is_player {
                    renderer.draw_sprite(frame, &sprite.sprite, &self.camera)?;
                }
            }

            if let Some(texture) = self.particle_texture {
                renderer.draw_particles(frame, &self.particle_system, &self.camera, Some(texture))?;
            }

            if fade_alpha > 0.0 {
                let w = screen_size.width as f32;
                let h = screen_size.height as f32;
                let points = [
                    Vec2::new(0.0, 0.0),
                    Vec2::new(w, 0.0),
                    Vec2::new(w, h),
                    Vec2::new(0.0, h),
                ];
                renderer.draw_polygon(
                    frame,
                    &points,
                    [0.02, 0.03, 0.05, fade_alpha],
                    &screen_camera,
                )?;
            }

            Ok(())
        })
    }
}

impl WallJumpDemo {
    fn spawn_landing_burst(&mut self, pos: Vec2) {
        let config_pos = Vec2::new(pos.x, pos.y + 20.0);
        let mut config = EmissionConfig::new(config_pos)
            .with_burst(42)
            .with_velocity(Vec2::new(-120.0, -80.0), Vec2::new(120.0, -200.0))
            .with_size(Vec2::new(3.0, 3.0), Vec2::new(8.0, 8.0))
            .with_color([0.9, 0.95, 1.0, 1.0], Some([0.3, 0.5, 0.8, 0.0]))
            .with_lifetime(0.35, 0.85)
            .with_acceleration(Vec2::new(0.0, 260.0))
            .with_size_end_multiplier(0.15);
        config.position_variance = Vec2::new(16.0, 6.0);
        let emitter = ParticleEmitter::new(config).with_max_particles(64);
        self.particle_system.add_emitter(emitter);
    }

    fn spawn_wall_slide_dust(&mut self, pos: Vec2) {
        let mut config = EmissionConfig::new(pos)
            .with_burst(10)
            .with_velocity(Vec2::new(-30.0, -20.0), Vec2::new(30.0, -120.0))
            .with_size(Vec2::new(2.0, 2.0), Vec2::new(5.0, 5.0))
            .with_color([0.7, 0.85, 1.0, 0.9], Some([0.2, 0.4, 0.7, 0.0]))
            .with_lifetime(0.25, 0.6)
            .with_acceleration(Vec2::new(0.0, 220.0))
            .with_size_end_multiplier(0.15);
        config.position_variance = Vec2::new(5.0, 8.0);
        let emitter = ParticleEmitter::new(config).with_max_particles(24);
        self.particle_system.add_emitter(emitter);
    }

    fn spawn_wall_jump_burst(&mut self, pos: Vec2) {
        let mut config = EmissionConfig::new(pos)
            .with_burst(20)
            .with_velocity(Vec2::new(-160.0, -80.0), Vec2::new(160.0, -240.0))
            .with_size(Vec2::new(3.0, 3.0), Vec2::new(6.0, 6.0))
            .with_color([0.9, 0.95, 1.0, 1.0], Some([0.3, 0.5, 0.9, 0.0]))
            .with_lifetime(0.3, 0.7)
            .with_acceleration(Vec2::new(0.0, 240.0))
            .with_size_end_multiplier(0.2);
        config.position_variance = Vec2::new(8.0, 10.0);
        let emitter = ParticleEmitter::new(config).with_max_particles(48);
        self.particle_system.add_emitter(emitter);
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Neon Walljump")
        .with_size(960, 540)
        .run(WallJumpDemo::new()?)
}
