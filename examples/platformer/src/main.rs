use anyhow::{anyhow, Result};
use sindri::{
    camera::{update_camera_follow, CameraFollow},
    math::{Camera2D, Vec2},
    physics::{ColliderShape, PhysicsWorld, RigidBodyType},
    render::{FontHandle, Renderer, Sprite, TextureHandle},
    BuiltinFont, Engine, EngineContext, EntityId, Game, KeyCode, World,
};

const PLAYER_SIZE: Vec2 = Vec2 { x: 32.0, y: 48.0 };
const MOVE_SPEED: f32 = 320.0;
const ACCELERATION: f32 = 2400.0;
const DECELERATION: f32 = 2800.0;
const JUMP_IMPULSE: f32 = -430.0;
const DASH_SPEED: f32 = 620.0;
const DASH_TIME: f32 = 0.12;
const DASH_COOLDOWN: f32 = 0.35;

#[derive(Clone, Copy)]
struct Platform {
    entity: EntityId,
    position: Vec2,
    size: Vec2,
}

#[derive(Clone, Copy)]
struct Spark {
    position: Vec2,
    collected: bool,
}

#[derive(Clone, Copy)]
struct Hazard {
    position: Vec2,
    size: Vec2,
}

#[derive(Clone, Copy)]
struct ExitGate {
    position: Vec2,
    size: Vec2,
}

#[derive(Clone, Copy)]
struct BurstParticle {
    position: Vec2,
    velocity: Vec2,
    life: f32,
    max_life: f32,
    size: f32,
    color: [f32; 4],
}

struct TextureHandles {
    white: Option<TextureHandle>,
}

struct EmberRun {
    world: World,
    physics: PhysicsWorld,
    camera: Camera2D,
    camera_follow: CameraFollow,

    player: Option<EntityId>,
    textures: TextureHandles,
    font: Option<FontHandle>,

    platforms: Vec<Platform>,
    sparks: Vec<Spark>,
    hazards: Vec<Hazard>,
    exit_gate: ExitGate,
    particles: Vec<BurstParticle>,

    score: u32,
    total_sparks: u32,
    deaths: u32,

    is_grounded: bool,
    facing: f32,
    jump_cooldown: f32,
    dash_timer: f32,
    dash_cooldown: f32,
    won: bool,

    initialized: bool,
}

impl EmberRun {
    fn new() -> Self {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(Vec2::new(0.0, 650.0));

        Self {
            world: World::new(),
            physics,
            camera: Camera2D::default(),
            camera_follow: CameraFollow::new(),
            player: None,
            textures: TextureHandles { white: None },
            font: None,
            platforms: Vec::new(),
            sparks: Vec::new(),
            hazards: Vec::new(),
            exit_gate: ExitGate {
                position: Vec2::new(1720.0, 420.0),
                size: Vec2::new(44.0, 96.0),
            },
            particles: Vec::new(),
            score: 0,
            total_sparks: 0,
            deaths: 0,
            is_grounded: false,
            facing: 1.0,
            jump_cooldown: 0.0,
            dash_timer: 0.0,
            dash_cooldown: 0.0,
            won: false,
            initialized: false,
        }
    }

    fn reset_runtime_state(&mut self) {
        self.world = World::new();
        self.physics = PhysicsWorld::new();
        self.physics.set_gravity(Vec2::new(0.0, 650.0));

        self.player = None;
        self.platforms.clear();
        self.sparks.clear();
        self.hazards.clear();
        self.particles.clear();

        self.score = 0;
        self.total_sparks = 0;
        self.is_grounded = false;
        self.facing = 1.0;
        self.jump_cooldown = 0.0;
        self.dash_timer = 0.0;
        self.dash_cooldown = 0.0;
        self.won = false;
    }

    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        let white = vec![255u8, 255, 255, 255];
        self.textures.white = Some(renderer.load_texture_from_rgba(&white, 1, 1)?);
        Ok(())
    }

    fn build_level(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.reset_runtime_state();

        let player_start = Vec2::new(120.0, 500.0);
        let player = self.world.spawn();
        self.player = Some(player);

        self.physics
        .create_body(player, RigidBodyType::Dynamic, player_start, 0.0)?;

        self.physics.add_collider_with_material(
            player,
            ColliderShape::CapsuleY {
                half_height: 18.0,
                radius: 11.0,
            },
            Vec2::ZERO,
            1.0,
            0.25,
            0.0,
        )?;

        self.physics.lock_rotations(player, true);
        self.physics.set_linear_damping(player, 0.0);

        self.camera = ctx.screen_camera();
        self.camera.position = player_start;

        self.camera_follow = CameraFollow::new()
        .follow_entity(player)
        .with_dead_zone(180.0, 120.0)
        .with_smoothing(0.12);

        self.spawn_platform(Vec2::new(500.0, 650.0), Vec2::new(1100.0, 40.0))?;
        self.spawn_platform(Vec2::new(260.0, 535.0), Vec2::new(170.0, 22.0))?;
        self.spawn_platform(Vec2::new(520.0, 455.0), Vec2::new(170.0, 22.0))?;
        self.spawn_platform(Vec2::new(820.0, 380.0), Vec2::new(180.0, 22.0))?;
        self.spawn_platform(Vec2::new(1120.0, 475.0), Vec2::new(180.0, 22.0))?;
        self.spawn_platform(Vec2::new(1410.0, 560.0), Vec2::new(220.0, 22.0))?;
        self.spawn_platform(Vec2::new(1720.0, 500.0), Vec2::new(180.0, 22.0))?;

        self.spawn_spark(Vec2::new(260.0, 490.0));
        self.spawn_spark(Vec2::new(520.0, 410.0));
        self.spawn_spark(Vec2::new(820.0, 335.0));
        self.spawn_spark(Vec2::new(1120.0, 430.0));
        self.spawn_spark(Vec2::new(1410.0, 515.0));
        self.spawn_spark(Vec2::new(1720.0, 455.0));

        self.hazards.push(Hazard {
            position: Vec2::new(650.0, 625.0),
                          size: Vec2::new(70.0, 28.0),
        });

        self.hazards.push(Hazard {
            position: Vec2::new(1010.0, 625.0),
                          size: Vec2::new(90.0, 28.0),
        });

        self.hazards.push(Hazard {
            position: Vec2::new(1320.0, 535.0),
                          size: Vec2::new(60.0, 28.0),
        });

        self.exit_gate = ExitGate {
            position: Vec2::new(1795.0, 430.0),
            size: Vec2::new(50.0, 110.0),
        };

        self.total_sparks = self.sparks.len() as u32;
        Ok(())
    }

    fn spawn_platform(&mut self, position: Vec2, size: Vec2) -> Result<EntityId> {
        let entity = self.world.spawn();

        self.physics
        .create_body(entity, RigidBodyType::Fixed, position, 0.0)?;

        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box {
                hx: size.x * 0.5,
                hy: size.y * 0.5,
            },
            Vec2::ZERO,
            0.0,
            0.85,
            0.0,
        )?;

        self.platforms.push(Platform {
            entity,
            position,
            size,
        });

        Ok(entity)
    }

    fn spawn_spark(&mut self, position: Vec2) {
        self.sparks.push(Spark {
            position,
            collected: false,
        });
    }

    fn player_entity(&self) -> Result<EntityId> {
        self.player.ok_or_else(|| anyhow!("player not initialized"))
    }

    fn player_position(&self) -> Option<Vec2> {
        self.player
        .and_then(|entity| self.physics.body_position(entity))
    }

    fn rects_overlap(a_pos: Vec2, a_size: Vec2, b_pos: Vec2, b_size: Vec2) -> bool {
        let a_half = a_size * 0.5;
        let b_half = b_size * 0.5;

        a_pos.x - a_half.x < b_pos.x + b_half.x
        && a_pos.x + a_half.x > b_pos.x - b_half.x
        && a_pos.y - a_half.y < b_pos.y + b_half.y
        && a_pos.y + a_half.y > b_pos.y - b_half.y
    }

    fn update_grounded(&mut self) {
        self.is_grounded = false;

        let Some(player) = self.player else {
            return;
        };

        let Some(pos) = self.physics.body_position(player) else {
            return;
        };

        let Some(vel) = self.physics.linear_velocity(player) else {
            return;
        };

        let player_bottom = pos.y + PLAYER_SIZE.y * 0.5;
        let player_half_w = PLAYER_SIZE.x * 0.5;

        for platform in &self.platforms {
            let top = platform.position.y - platform.size.y * 0.5;
            let left = platform.position.x - platform.size.x * 0.5;
            let right = platform.position.x + platform.size.x * 0.5;

            let horizontally_overlapping =
            pos.x + player_half_w > left && pos.x - player_half_w < right;

            let close_to_top = player_bottom >= top - 6.0 && player_bottom <= top + 14.0;

            if horizontally_overlapping && close_to_top && vel.y >= -20.0 {
                self.is_grounded = true;
                break;
            }
        }
    }

    fn update_player(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let player = self.player_entity()?;
        let input = ctx.input();
        let dt = ctx.delta_seconds();

        if self.jump_cooldown > 0.0 {
            self.jump_cooldown -= dt;
        }

        if self.dash_cooldown > 0.0 {
            self.dash_cooldown -= dt;
        }

        if self.dash_timer > 0.0 {
            self.dash_timer -= dt;
        }

        let mut target_x = 0.0;

        if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
            target_x -= MOVE_SPEED;
            self.facing = -1.0;
        }

        if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
            target_x += MOVE_SPEED;
            self.facing = 1.0;
        }

        if let Some(mut vel) = self.physics.linear_velocity(player) {
            if self.dash_timer > 0.0 {
                vel.x = self.facing * DASH_SPEED;
            } else if target_x != 0.0 {
                let diff = target_x - vel.x;
                vel.x += diff * ACCELERATION * dt;
                vel.x = vel.x.clamp(-MOVE_SPEED, MOVE_SPEED);
            } else if vel.x.abs() > 1.0 {
                let stop = -vel.x.signum() * DECELERATION * dt;
                vel.x += stop;

                if vel.x.abs() < 12.0 {
                    vel.x = 0.0;
                }
            } else {
                vel.x = 0.0;
            }

            self.physics.set_linear_velocity(player, vel);
        }

        let wants_jump = input.is_key_pressed(KeyCode::Space)
        || input.is_key_pressed(KeyCode::KeyW)
        || input.is_key_pressed(KeyCode::ArrowUp);

        if wants_jump && self.is_grounded && self.jump_cooldown <= 0.0 && !self.won {
            self.physics.apply_impulse(player, Vec2::new(0.0, JUMP_IMPULSE));
            self.jump_cooldown = 0.16;
            self.is_grounded = false;

            if let Some(pos) = self.player_position() {
                self.spawn_burst(pos + Vec2::new(0.0, PLAYER_SIZE.y * 0.5), 10, [0.95, 0.55, 0.2, 1.0]);
            }
        }

        let wants_dash = input.is_key_pressed(KeyCode::ShiftLeft);

        if wants_dash && self.dash_cooldown <= 0.0 && !self.won {
            self.dash_timer = DASH_TIME;
            self.dash_cooldown = DASH_COOLDOWN;

            if let Some(pos) = self.player_position() {
                self.spawn_burst(pos, 14, [0.45, 0.65, 1.0, 1.0]);
            }
        }

        Ok(())
    }

    fn update_collectibles_and_hazards(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let Some(player_pos) = self.player_position() else {
            return Ok(());
        };

        let mut collected_positions = Vec::new();

        for spark in &mut self.sparks {
            if spark.collected {
                continue;
            }

            if player_pos.distance(spark.position) < 34.0 {
                spark.collected = true;
                self.score += 1;
                collected_positions.push(spark.position);
            }
        }

        for pos in collected_positions {
            self.spawn_burst(pos, 18, [1.0, 0.72, 0.18, 1.0]);
        }

        for hazard in &self.hazards {
            if Self::rects_overlap(player_pos, PLAYER_SIZE, hazard.position, hazard.size) {
                self.deaths += 1;
                self.spawn_burst(player_pos, 24, [1.0, 0.2, 0.15, 1.0]);
                self.build_level(ctx)?;
                return Ok(());
            }
        }

        if Self::rects_overlap(
            player_pos,
            PLAYER_SIZE,
            self.exit_gate.position,
            self.exit_gate.size,
        ) && self.score == self.total_sparks
        {
            self.won = true;
            if let Some(player) = self.player {
                self.physics.set_linear_velocity(player, Vec2::ZERO);
            }
        }

        if player_pos.y > 900.0 {
            self.deaths += 1;
            self.build_level(ctx)?;
        }

        Ok(())
    }

    fn spawn_burst(&mut self, origin: Vec2, count: usize, color: [f32; 4]) {
        for i in 0..count {
            let t = i as f32 / count.max(1) as f32;
            let angle = t * std::f32::consts::TAU;
            let speed = 90.0 + (i % 5) as f32 * 22.0;

            self.particles.push(BurstParticle {
                position: origin,
                velocity: Vec2::from_angle(angle) * speed + Vec2::new(0.0, -35.0),
                                life: 0.45,
                                max_life: 0.45,
                                size: 4.0 + (i % 3) as f32,
                                color,
            });
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for particle in &mut self.particles {
            particle.life -= dt;
            particle.velocity.y += 360.0 * dt;
            particle.position += particle.velocity * dt;
        }

        self.particles.retain(|particle| particle.life > 0.0);
    }

    fn draw_rect(
        renderer: &mut Renderer,
        frame: &mut sindri::Frame,
        texture: TextureHandle,
        camera: &Camera2D,
        position: Vec2,
        size: Vec2,
        color: [f32; 4],
    ) -> Result<()> {
        let mut sprite = Sprite::new(texture);
        sprite.transform.position = position;
        sprite.set_size_px(size, Vec2::new(1.0, 1.0));
        sprite.tint = color;
        renderer.draw_sprite(frame, &sprite, camera)?;
        Ok(())
    }

    fn draw_player(
        &self,
        renderer: &mut Renderer,
        frame: &mut sindri::Frame,
        white: TextureHandle,
    ) -> Result<()> {
        let Some(player_pos) = self.player_position() else {
            return Ok(());
        };

        let body_color = if self.dash_timer > 0.0 {
            [0.35, 0.55, 1.0, 1.0]
        } else if self.is_grounded {
            [0.25, 0.42, 1.0, 1.0]
        } else {
            [0.45, 0.32, 1.0, 1.0]
        };

        Self::draw_rect(
            renderer,
            frame,
            white,
            &self.camera,
            player_pos,
            PLAYER_SIZE,
            body_color,
        )?;

        let eye_y = player_pos.y - 9.0;
        let eye_x = player_pos.x + self.facing * 7.0;

        Self::draw_rect(
            renderer,
            frame,
            white,
            &self.camera,
            Vec2::new(eye_x, eye_y),
                        Vec2::new(5.0, 5.0),
                        [0.92, 0.96, 1.0, 1.0],
        )?;

        Ok(())
    }

    fn draw_level(
        &self,
        renderer: &mut Renderer,
        frame: &mut sindri::Frame,
        white: TextureHandle,
    ) -> Result<()> {
        for platform in &self.platforms {
            let is_ground = platform.size.x > 1000.0;

            let color = if is_ground {
                [0.18, 0.22, 0.13, 1.0]
            } else {
                [0.24, 0.31, 0.18, 1.0]
            };

            Self::draw_rect(
                renderer,
                frame,
                white,
                &self.camera,
                platform.position,
                platform.size,
                color,
            )?;

            Self::draw_rect(
                renderer,
                frame,
                white,
                &self.camera,
                platform.position + Vec2::new(0.0, -platform.size.y * 0.5 + 2.0),
                            Vec2::new(platform.size.x, 4.0),
                            [0.86, 0.56, 0.18, 1.0],
            )?;
        }

        for hazard in &self.hazards {
            Self::draw_rect(
                renderer,
                frame,
                white,
                &self.camera,
                hazard.position,
                hazard.size,
                [0.85, 0.15, 0.12, 1.0],
            )?;

            let spike_count = (hazard.size.x / 14.0).max(1.0) as usize;
            let left = hazard.position.x - hazard.size.x * 0.5;

            for i in 0..spike_count {
                let x = left + i as f32 * 14.0 + 7.0;
                let base_y = hazard.position.y - hazard.size.y * 0.5;
                let points = [
                    Vec2::new(x - 7.0, base_y),
                    Vec2::new(x + 7.0, base_y),
                    Vec2::new(x, base_y - 22.0),
                ];

                renderer.draw_polygon_no_occlusion(
                    frame,
                    &points,
                    [1.0, 0.25, 0.1, 1.0],
                    &self.camera,
                )?;
            }
        }

        let gate_color = if self.score == self.total_sparks {
            [0.15, 0.85, 0.55, 1.0]
        } else {
            [0.25, 0.28, 0.35, 1.0]
        };

        Self::draw_rect(
            renderer,
            frame,
            white,
            &self.camera,
            self.exit_gate.position,
            self.exit_gate.size,
            gate_color,
        )?;

        Self::draw_rect(
            renderer,
            frame,
            white,
            &self.camera,
            self.exit_gate.position,
            Vec2::new(self.exit_gate.size.x - 18.0, self.exit_gate.size.y - 18.0),
                        [0.05, 0.07, 0.09, 1.0],
        )?;

        for spark in &self.sparks {
            if spark.collected {
                continue;
            }

            renderer.draw_circle(
                frame,
                spark.position,
                11.0,
                [1.0, 0.68, 0.14, 1.0],
                &self.camera,
            )?;

            renderer.draw_circle(
                frame,
                spark.position,
                5.0,
                [1.0, 0.95, 0.55, 1.0],
                &self.camera,
            )?;
        }

        Ok(())
    }

    fn draw_particles(
        &self,
        renderer: &mut Renderer,
        frame: &mut sindri::Frame,
    ) -> Result<()> {
        for particle in &self.particles {
            let alpha = (particle.life / particle.max_life).clamp(0.0, 1.0);
            let mut color = particle.color;
            color[3] = alpha;

            renderer.draw_circle(
                frame,
                particle.position,
                particle.size * alpha.max(0.25),
                                 color,
                                 &self.camera,
            )?;
        }

        Ok(())
    }

    fn draw_hud(
        &self,
        renderer: &mut Renderer,
        frame: &mut sindri::Frame,
        screen_w: u32,
        screen_h: u32,
    ) -> Result<()> {
        let Some(font) = self.font else {
            return Ok(());
        };

        let top_left = Vec2::new(
            self.camera.position.x - screen_w as f32 * 0.5 + 24.0,
            self.camera.position.y - screen_h as f32 * 0.5 + 36.0,
        );

        let status = if self.won {
            "YOU WON · Press R to restart".to_string()
        } else if self.score == self.total_sparks {
            "Gate open · Reach the exit".to_string()
        } else {
            "Collect all sparks, then reach the gate".to_string()
        };

        let score = format!(
            "Sparks: {}/{}   Deaths: {}",
            self.score, self.total_sparks, self.deaths
        );

        renderer.draw_text(
            frame,
            &score,
            font,
            22.0,
            top_left,
            [0.95, 0.95, 0.9, 1.0],
            &self.camera,
        )?;

        renderer.draw_text(
            frame,
            &status,
            font,
            18.0,
            top_left + Vec2::new(0.0, 28.0),
                           [0.95, 0.72, 0.28, 1.0],
                           &self.camera,
        )?;

        renderer.draw_text(
            frame,
            "A/D or arrows: move · Space/W/Up: jump · Shift: dash · R: restart · Esc: quit",
            font,
            15.0,
            top_left + Vec2::new(0.0, 54.0),
                           [0.72, 0.76, 0.82, 1.0],
                           &self.camera,
        )?;

        Ok(())
    }
}

impl Game for EmberRun {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.create_textures(&mut *ctx.renderer())?;
        self.font = Some(ctx.builtin_font(BuiltinFont::Ui)?);
        self.build_level(ctx)?;
        self.initialized = true;
        Ok(())
    }

    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if !self.initialized || self.won {
            return Ok(());
        }

        self.physics.step(ctx.fixed_delta_seconds());

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
            return Ok(());
        }

        if ctx.input().is_key_pressed(KeyCode::KeyR) {
            self.build_level(ctx)?;
            return Ok(());
        }

        let dt = ctx.delta_seconds();

        if !self.won {
            self.update_grounded();
            self.update_player(ctx)?;

            self.update_grounded();
            self.update_collectibles_and_hazards(ctx)?;
        }

        self.update_particles(dt);

        if let Some(player) = self.player {
            update_camera_follow(&mut self.camera, &self.camera_follow, &self.physics, dt);

            if let Some(pos) = self.physics.body_position(player) {
                self.camera.position.x = self.camera.position.x.clamp(240.0, 1700.0);
                self.camera.position.y = self.camera.position.y.clamp(260.0, 620.0);

                if self.won {
                    self.camera.position = self.camera.position.lerp(pos, 0.04);
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        let Some(white) = self.textures.white else {
            return Ok(());
        };

        let (screen_w, screen_h) = ctx.renderer().surface_size();

        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        renderer.clear(&mut frame, [0.035, 0.045, 0.06, 1.0])?;

        Self::draw_rect(
            renderer,
            &mut frame,
            white,
            &self.camera,
            Vec2::new(950.0, 450.0),
                        Vec2::new(2300.0, 900.0),
                        [0.045, 0.055, 0.075, 1.0],
        )?;

        for i in 0..18 {
            let x = -100.0 + i as f32 * 140.0;
            Self::draw_rect(
                renderer,
                &mut frame,
                white,
                &self.camera,
                Vec2::new(x, 260.0 + (i % 3) as f32 * 34.0),
                            Vec2::new(70.0, 10.0),
                            [0.09, 0.10, 0.13, 1.0],
            )?;
        }

        self.draw_level(renderer, &mut frame, white)?;
        self.draw_particles(renderer, &mut frame)?;
        self.draw_player(renderer, &mut frame, white)?;
        self.draw_hud(renderer, &mut frame, screen_w, screen_h)?;

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
    .with_title("Sindri Engine · Ember Run")
    .with_size(1280, 720)
    .with_vsync(true)
    .run(EmberRun::new())
}
