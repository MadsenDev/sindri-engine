use anyhow::Result;
use std::path::Path;

use sindri::{
    hud::{HudLayer, HudText, TextAlign},
    physics::PhysicsWorld,
    render::{FontHandle, TextureHandle},
    script::{ScriptComponent, ScriptParams, ScriptRuntime, ScriptTag},
    BuiltinFont, Camera2D, EngineContext, Game, KeyCode, SpriteComponent, Transform, Vec2, World,
};

use crate::components::{AsteroidMarker, AsteroidSize, BulletMarker, PlayerMarker, PolygonShape};

const SCREEN_W: f32 = 1280.0;
const SCREEN_H: f32 = 720.0;

pub struct ScriptedAsteroids {
    runtime: ScriptRuntime,
    world: World,
    physics: PhysicsWorld,
    camera: Camera2D,

    white_texture: Option<TextureHandle>,
    font: Option<FontHandle>,
    hud: HudLayer,

    player: Option<sindri::EntityId>,

    score: u32,
    lives: u32,
    game_over: bool,
    shoot_cooldown: f32,
    spawn_timer: f32,
    frame_count: u32,

    initialized: bool,
}

impl ScriptedAsteroids {
    pub fn new() -> Result<Self> {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(Vec2::ZERO);

        Ok(Self {
            runtime: ScriptRuntime::new()?.with_hot_reload(true),
            world: World::new(),
            physics,
            camera: Camera2D::default(),
            white_texture: None,
            font: None,
            hud: HudLayer::new(),
            player: None,
            score: 0,
            lives: 3,
            game_over: false,
            shoot_cooldown: 0.0,
            spawn_timer: 0.0,
            frame_count: 0,
            initialized: false,
        })
    }

    fn script_path(name: &str) -> String {
        format!("{}/scripts/{}", env!("CARGO_MANIFEST_DIR"), name)
    }

    fn create_textures(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let white = [255u8, 255u8, 255u8, 255u8];
        self.white_texture = Some(ctx.renderer().load_texture_from_rgba(&white, 1, 1)?);
        Ok(())
    }

    fn reset_game(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.world = World::new();
        self.physics = PhysicsWorld::new();
        self.physics.set_gravity(Vec2::ZERO);

        self.score = 0;
        self.lives = 3;
        self.game_over = false;
        self.shoot_cooldown = 0.0;
        self.spawn_timer = 0.0;
        self.player = None;

        self.spawn_player()?;

        for _ in 0..5 {
            self.spawn_asteroid(AsteroidSize::Large, None)?;
        }

        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), 0.0)?;

        Ok(())
    }

    fn spawn_player(&mut self) -> Result<()> {
        let texture = self.white_texture.expect("white texture not initialized");

        let entity = self.world.spawn();
        let position = Vec2::new(SCREEN_W * 0.5, SCREEN_H * 0.5);

        self.world.insert(entity, Transform::new(position));
        self.world.insert(entity, PlayerMarker);
        self.world.insert(entity, ScriptTag("player".to_string()));

        let mut sprite = SpriteComponent::new(texture);
        sprite
            .sprite
            .set_size_px(Vec2::new(32.0, 32.0), Vec2::new(1.0, 1.0));
        sprite.sprite.tint = [1.0, 1.0, 1.0, 1.0];
        self.world.insert(entity, sprite);

        let script = Self::script_path("player.lua");
        println!(
            "[scripted_asteroids] attaching player script: {} exists={}",
            script,
            Path::new(&script).exists()
        );

        let params = ScriptParams::default()
            .insert("screen_w", SCREEN_W)
            .insert("screen_h", SCREEN_H)
            .insert("rotation_speed", 5.0)
            .insert("thrust", 260.0)
            .insert("friction", 0.985)
            .insert("max_speed", 340.0);

        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script, params),
        );

        self.player = Some(entity);
        Ok(())
    }

    fn spawn_bullet(&mut self) -> Result<()> {
        let Some(player) = self.player else {
            return Ok(());
        };

        let Some(player_transform) = self.world.get::<Transform>(player).cloned() else {
            return Ok(());
        };

        let texture = self.white_texture.expect("white texture not initialized");
        let forward = Vec2::from_angle(player_transform.rotation);
        let position = player_transform.position + forward * 28.0;

        let entity = self.world.spawn();

        self.world.insert(
            entity,
            Transform::new(position).with_rotation(player_transform.rotation),
        );
        self.world.insert(entity, BulletMarker);
        self.world.insert(entity, ScriptTag("bullet".to_string()));

        let mut sprite = SpriteComponent::new(texture);
        sprite
            .sprite
            .set_size_px(Vec2::new(6.0, 6.0), Vec2::new(1.0, 1.0));
        sprite.sprite.tint = [1.0, 0.92, 0.35, 1.0];
        self.world.insert(entity, sprite);

        let script = Self::script_path("bullet.lua");
        let params = ScriptParams::default()
            .insert("screen_w", SCREEN_W)
            .insert("screen_h", SCREEN_H)
            .insert("speed", 520.0)
            .insert("lifetime", 1.35);

        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script, params),
        );

        Ok(())
    }

    fn spawn_asteroid(&mut self, size: AsteroidSize, position: Option<Vec2>) -> Result<()> {
        let texture = self.white_texture.expect("white texture not initialized");

        let position = position.unwrap_or_else(|| {
            let side = fastrand::u8(0..4);
            match side {
                0 => Vec2::new(0.0, fastrand::f32() * SCREEN_H),
                1 => Vec2::new(SCREEN_W, fastrand::f32() * SCREEN_H),
                2 => Vec2::new(fastrand::f32() * SCREEN_W, 0.0),
                _ => Vec2::new(fastrand::f32() * SCREEN_W, SCREEN_H),
            }
        });

        let entity = self.world.spawn();
        let radius = size.radius();

        self.world.insert(entity, Transform::new(position));
        self.world.insert(entity, AsteroidMarker { size, radius });
        self.world
            .insert(entity, PolygonShape::asteroid(position, radius));
        self.world.insert(entity, ScriptTag("asteroid".to_string()));

        let mut sprite = SpriteComponent::new(texture);
        sprite
            .sprite
            .set_size_px(Vec2::new(radius * 2.0, radius * 2.0), Vec2::new(1.0, 1.0));
        sprite.sprite.tint = [0.65, 0.68, 0.72, 1.0];
        self.world.insert(entity, sprite);

        let (min_speed, max_speed) = size.speed_range();
        let angle = fastrand::f32() * std::f32::consts::TAU;
        let speed = min_speed + fastrand::f32() * (max_speed - min_speed);
        let velocity = Vec2::from_angle(angle) * speed;
        let rotation_speed = match size {
            AsteroidSize::Large => 0.45,
            AsteroidSize::Medium => 0.9,
            AsteroidSize::Small => 1.6,
        } * if fastrand::bool() { 1.0 } else { -1.0 };

        let script = Self::script_path("asteroid.lua");
        let params = ScriptParams::default()
            .insert("screen_w", SCREEN_W)
            .insert("screen_h", SCREEN_H)
            .insert("velocity", velocity)
            .insert("rotation_speed", rotation_speed);

        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script, params),
        );

        Ok(())
    }

    fn sync_sprite_transforms(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<SpriteComponent>()
            .into_iter()
            .map(|(entity, _)| entity)
            .collect();

        for entity in entities {
            let Some(transform) = self.world.get::<Transform>(entity).cloned() else {
                continue;
            };

            if let Some(sprite) = self.world.get_mut::<SpriteComponent>(entity) {
                sprite.sprite.transform.position = transform.position;
                sprite.sprite.transform.rotation = transform.rotation;
                sprite.sprite.transform.scale = transform.scale;
            }
        }
    }

    fn update_gameplay(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_seconds();

        if self.shoot_cooldown > 0.0 {
            self.shoot_cooldown -= dt;
        }

        if ctx.input().is_key_down(KeyCode::Space) && self.shoot_cooldown <= 0.0 {
            self.spawn_bullet()?;
            self.shoot_cooldown = 0.18;
        }

        self.spawn_timer += dt;
        if self.spawn_timer >= 5.0 && self.world.query::<AsteroidMarker>().len() < 12 {
            self.spawn_asteroid(AsteroidSize::Large, None)?;
            self.spawn_timer = 0.0;
        }

        self.handle_collisions()?;

        Ok(())
    }

    fn handle_collisions(&mut self) -> Result<()> {
        let bullets: Vec<_> = self
            .world
            .query::<BulletMarker>()
            .into_iter()
            .filter_map(|(entity, _)| {
                let pos = self.world.get::<Transform>(entity)?.position;
                Some((entity, pos))
            })
            .collect();

        let asteroids: Vec<_> = self
            .world
            .query::<AsteroidMarker>()
            .into_iter()
            .filter_map(|(entity, marker)| {
                let pos = self.world.get::<Transform>(entity)?.position;
                Some((entity, pos, *marker))
            })
            .collect();

        let mut bullets_to_remove = Vec::new();
        let mut asteroids_to_remove = Vec::new();
        let mut asteroid_splits = Vec::new();

        for (bullet_entity, bullet_pos) in &bullets {
            for (asteroid_entity, asteroid_pos, asteroid) in &asteroids {
                if distance_sq(*bullet_pos, *asteroid_pos) < (asteroid.radius + 4.0).powi(2) {
                    bullets_to_remove.push(*bullet_entity);
                    asteroids_to_remove.push(*asteroid_entity);
                    self.score += asteroid.size.score();

                    if let Some(child_size) = asteroid.size.child() {
                        asteroid_splits.push((*asteroid_pos, child_size));
                        asteroid_splits.push((*asteroid_pos, child_size));
                    }

                    break;
                }
            }
        }

        for entity in bullets_to_remove {
            self.world.despawn(entity);
        }

        for entity in asteroids_to_remove {
            self.world.despawn(entity);
        }

        for (position, size) in asteroid_splits {
            let jitter = Vec2::new(fastrand::f32() * 18.0 - 9.0, fastrand::f32() * 18.0 - 9.0);
            self.spawn_asteroid(size, Some(position + jitter))?;
        }

        self.handle_player_collisions();

        Ok(())
    }

    fn handle_player_collisions(&mut self) {
        if self.game_over {
            return;
        }

        let Some(player) = self.player else {
            return;
        };

        let Some(player_pos) = self.world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };

        let asteroids: Vec<_> = self
            .world
            .query::<AsteroidMarker>()
            .into_iter()
            .filter_map(|(entity, marker)| {
                let pos = self.world.get::<Transform>(entity)?.position;
                Some((entity, pos, *marker))
            })
            .collect();

        for (_, asteroid_pos, asteroid) in asteroids {
            if distance_sq(player_pos, asteroid_pos) < (asteroid.radius + 16.0).powi(2) {
                self.lives = self.lives.saturating_sub(1);

                let bullets: Vec<_> = self
                    .world
                    .query::<BulletMarker>()
                    .into_iter()
                    .map(|(entity, _)| entity)
                    .collect();

                for bullet in bullets {
                    self.world.despawn(bullet);
                }

                if self.lives == 0 {
                    self.game_over = true;
                    self.world.despawn(player);
                    self.player = None;
                } else if let Some(transform) = self.world.get_mut::<Transform>(player) {
                    transform.position = Vec2::new(SCREEN_W * 0.5, SCREEN_H * 0.5);
                    transform.rotation = 0.0;
                }

                break;
            }
        }
    }

    fn draw_player(&self, ctx: &mut EngineContext, frame: &mut sindri::Frame) -> Result<()> {
        let Some(player) = self.player else {
            return Ok(());
        };

        let Some(transform) = self.world.get::<Transform>(player) else {
            return Ok(());
        };

        let points = ship_points(transform.position, transform.rotation);
        ctx.renderer()
            .draw_polygon(frame, &points, [0.95, 0.98, 1.0, 1.0], &self.camera)?;

        let thrusting =
            ctx.input().is_key_down(KeyCode::KeyW) || ctx.input().is_key_down(KeyCode::ArrowUp);

        if thrusting && !self.game_over {
            let flame = flame_points(
                transform.position,
                transform.rotation,
                ctx.elapsed_time().as_secs_f32(),
            );
            ctx.renderer()
                .draw_polygon(frame, &flame, [1.0, 0.48, 0.05, 1.0], &self.camera)?;
        }

        Ok(())
    }

    fn draw_asteroids(&self, ctx: &mut EngineContext, frame: &mut sindri::Frame) -> Result<()> {
        for (entity, _) in self.world.query::<AsteroidMarker>() {
            let Some(transform) = self.world.get::<Transform>(entity) else {
                continue;
            };
            let Some(shape) = self.world.get::<PolygonShape>(entity) else {
                continue;
            };

            let points = shape.transformed(transform.position, transform.rotation);
            ctx.renderer()
                .draw_polygon(frame, &points, [0.64, 0.67, 0.72, 1.0], &self.camera)?;
        }

        Ok(())
    }

    fn draw_bullets(&self, ctx: &mut EngineContext, frame: &mut sindri::Frame) -> Result<()> {
        for (entity, _) in self.world.query::<BulletMarker>() {
            let Some(transform) = self.world.get::<Transform>(entity) else {
                continue;
            };

            ctx.renderer().draw_circle(
                frame,
                transform.position,
                3.0,
                [1.0, 0.92, 0.35, 1.0],
                &self.camera,
            )?;
        }

        Ok(())
    }

    fn draw_hud(&mut self, ctx: &mut EngineContext, frame: &mut sindri::Frame) -> Result<()> {
        self.hud.clear();

        if let Some(font) = self.font {
            self.hud.add_text(HudText::new(
                format!("Score: {}", self.score),
                font,
                24.0,
                Vec2::new(20.0, 20.0),
                [0.95, 0.95, 1.0, 1.0],
            ));

            self.hud.add_text(HudText::new(
                format!("Lives: {}", self.lives),
                font,
                24.0,
                Vec2::new(20.0, 50.0),
                [0.95, 0.95, 1.0, 1.0],
            ));

            self.hud.add_text(HudText::new(
                "Lua scripts: player movement · bullets · asteroids · hot reload ON".to_string(),
                font,
                16.0,
                Vec2::new(20.0, 684.0),
                [0.72, 0.76, 0.85, 1.0],
            ));

            if self.game_over {
                self.hud.add_text(
                    HudText::new(
                        "GAME OVER".to_string(),
                        font,
                        54.0,
                        Vec2::new(SCREEN_W * 0.5, SCREEN_H * 0.5 - 40.0),
                        [1.0, 0.2, 0.18, 1.0],
                    )
                    .with_align(TextAlign::Center),
                );

                self.hud.add_text(
                    HudText::new(
                        "Press R to restart".to_string(),
                        font,
                        24.0,
                        Vec2::new(SCREEN_W * 0.5, SCREEN_H * 0.5 + 22.0),
                        [0.95, 0.95, 1.0, 1.0],
                    )
                    .with_align(TextAlign::Center),
                );
            }
        }

        self.hud.draw(ctx.renderer(), frame)?;
        Ok(())
    }
}

impl Game for ScriptedAsteroids {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.camera = Camera2D::new(Vec2::new(SCREEN_W * 0.5, SCREEN_H * 0.5));
        self.create_textures(ctx)?;
        self.font = Some(ctx.builtin_font(BuiltinFont::Ui)?);
        self.reset_game(ctx)?;
        self.initialized = true;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
            return Ok(());
        }

        if ctx.input().is_key_pressed(KeyCode::KeyR) {
            self.reset_game(ctx)?;
            return Ok(());
        }

        if !self.initialized {
            return Ok(());
        }

        let dt = ctx.delta_seconds();

        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), dt)?;

        if !self.game_over {
            self.update_gameplay(ctx)?;
        }

        self.sync_sprite_transforms();

        self.frame_count = self.frame_count.wrapping_add(1);

        Ok(())
    }

    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        let fixed_dt = ctx.fixed_delta_seconds();

        self.runtime
            .fixed_update(&mut self.world, &mut self.physics, ctx.input(), fixed_dt)?;

        self.physics.step(fixed_dt);

        let events = self.physics.drain_events();
        self.runtime.handle_physics_events(
            &events,
            &mut self.world,
            &mut self.physics,
            ctx.input(),
        )?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let mut frame = ctx.renderer().begin_frame()?;
        ctx.renderer()
            .clear(&mut frame, [0.015, 0.018, 0.026, 1.0])?;

        draw_starfield(ctx, &mut frame, self.frame_count, &self.camera)?;

        if !self.game_over {
            self.draw_bullets(ctx, &mut frame)?;
            self.draw_asteroids(ctx, &mut frame)?;
            self.draw_player(ctx, &mut frame)?;
        } else {
            self.draw_asteroids(ctx, &mut frame)?;
        }

        self.draw_hud(ctx, &mut frame)?;

        ctx.renderer().end_frame(frame)?;
        Ok(())
    }
}

fn distance_sq(a: Vec2, b: Vec2) -> f32 {
    let d = a - b;
    d.x * d.x + d.y * d.y
}

fn ship_points(position: Vec2, rotation: f32) -> Vec<Vec2> {
    let local = [
        Vec2::new(24.0, 0.0),
        Vec2::new(-15.0, -13.0),
        Vec2::new(-8.0, 0.0),
        Vec2::new(-15.0, 13.0),
    ];

    rotate_points(&local, position, rotation)
}

fn flame_points(position: Vec2, rotation: f32, time: f32) -> Vec<Vec2> {
    let flicker = 8.0 + (time * 28.0).sin().abs() * 8.0;

    let local = [
        Vec2::new(-14.0, -7.0),
        Vec2::new(-14.0 - flicker, 0.0),
        Vec2::new(-14.0, 7.0),
    ];

    rotate_points(&local, position, rotation)
}

fn rotate_points(points: &[Vec2], position: Vec2, rotation: f32) -> Vec<Vec2> {
    let cos = rotation.cos();
    let sin = rotation.sin();

    points
        .iter()
        .map(|p| {
            let rotated = Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
            position + rotated
        })
        .collect()
}

fn draw_starfield(
    ctx: &mut EngineContext,
    frame: &mut sindri::Frame,
    frame_count: u32,
    camera: &Camera2D,
) -> Result<()> {
    for i in 0..90 {
        let x = ((i * 137) % 1280) as f32;
        let y = ((i * 263) % 720) as f32;
        let pulse = (((frame_count as f32 * 0.025) + i as f32).sin() * 0.5 + 0.5) * 0.35 + 0.35;

        ctx.renderer().draw_circle(
            frame,
            Vec2::new(x, y),
            if i % 7 == 0 { 1.8 } else { 1.1 },
            [pulse, pulse, pulse + 0.12, 1.0],
            camera,
        )?;
    }

    Ok(())
}
