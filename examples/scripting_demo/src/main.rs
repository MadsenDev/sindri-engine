use anyhow::Result;
use std::path::Path;

use sindri::{
    hud::{HudLayer, HudText},
    input::InputState,
    math::{Camera2D, Vec2},
    physics::{ColliderShape, PhysicsWorld, RigidBodyType},
    render::{FontHandle, Renderer, TextureHandle},
    script::{ScriptComponent, ScriptParams, ScriptRuntime, ScriptTag},
    Engine, EngineContext, Game, KeyCode, SpriteComponent, Transform, World,
};

struct ScriptingDemo {
    runtime: ScriptRuntime,
    world: World,
    physics: PhysicsWorld,
    camera: Camera2D,
    player: Option<sindri::EntityId>,
    player_texture: Option<TextureHandle>,
    platform_texture: Option<TextureHandle>,
    // Benchmark stats
    benchmark_mode: bool,
    entity_count: usize,
    frame_count: u32,
    last_fps_update: std::time::Instant,
    current_fps: f32,
    script_time_ms: f32,
    // HUD and test tracking
    hud: HudLayer,
    font: Option<FontHandle>,
    test_entity: Option<sindri::EntityId>,
    test_stats: TestStats,
}

#[derive(Default)]
struct TestStats {
    start_called: bool,
    update_count: u32,
    fixed_update_count: u32,
    collision_count: u32,
    trigger_count: u32,
    last_position: Option<Vec2>,
}

impl ScriptingDemo {
    fn new() -> Result<Self> {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(Vec2::new(0.0, 500.0)); // Stronger gravity for better feel

        Ok(Self {
            runtime: ScriptRuntime::new()?,
            world: World::new(),
            physics,
            camera: Camera2D::default(),
            player: None,
            player_texture: None,
            platform_texture: None,
            benchmark_mode: false,
            entity_count: 0,
            frame_count: 0,
            last_fps_update: std::time::Instant::now(),
            current_fps: 0.0,
            script_time_ms: 0.0,
            hud: HudLayer::new(),
            font: None,
            test_entity: None,
            test_stats: TestStats::default(),
        })
    }

    fn spawn_comprehensive_test_entity(&mut self) -> Result<sindri::EntityId> {
        let position = Vec2::new(600.0, 200.0);
        let entity = self.world.spawn();
        self.world.insert(entity, Transform::new(position));

        if let Some(texture) = self.player_texture {
            let mut sprite = SpriteComponent::new(texture);
            sprite
                .sprite
                .set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
            sprite.sprite.tint = [1.0, 0.5, 0.0, 1.0]; // Orange
            self.world.insert(entity, sprite);
        }

        self.world.insert(entity, ScriptTag("test_entity".into()));
        let params = ScriptParams::default();

        let script_path = format!(
            "{}/scripts/comprehensive_test.lua",
            env!("CARGO_MANIFEST_DIR")
        );

        println!(
            "[ScriptingDemo] Attaching comprehensive test script: {} (exists: {})",
            script_path,
            Path::new(&script_path).exists()
        );

        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script_path, params),
        );

        // Create physics body
        self.physics
            .create_body(entity, RigidBodyType::Dynamic, position, 0.0)?;
        self.physics.lock_rotations(entity, true);
        self.physics.set_linear_damping(entity, 0.5);
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box { hx: 16.0, hy: 16.0 },
            Vec2::ZERO,
            1.0,
            0.8,
            0.1,
        )?;

        // Create a sensor/trigger zone for testing trigger events
        let trigger_entity = self.world.spawn();
        self.world
            .insert(trigger_entity, Transform::new(Vec2::new(400.0, 300.0)));
        self.physics.create_body(
            trigger_entity,
            RigidBodyType::Fixed,
            Vec2::new(400.0, 300.0),
            0.0,
        )?;
        self.physics.add_sensor(
            trigger_entity,
            ColliderShape::Circle { radius: 50.0 },
            Vec2::ZERO,
        )?;

        Ok(entity)
    }

    fn spawn_benchmark_entity(&mut self, x: f32, y: f32) -> Result<()> {
        let entity = self.world.spawn();
        self.world.insert(entity, Transform::new(Vec2::new(x, y)));

        if let Some(texture) = self.player_texture {
            let mut sprite = SpriteComponent::new(texture);
            sprite
                .sprite
                .set_size_px(Vec2::new(16.0, 16.0), Vec2::new(32.0, 32.0));
            sprite.sprite.tint = [
                0.5 + (x as f32 / 1000.0) * 0.5,
                0.5 + (y as f32 / 1000.0) * 0.5,
                1.0,
                1.0,
            ];
            self.world.insert(entity, sprite);
        }

        let params = ScriptParams::default()
            .insert("speed", 30.0 + (x as f32 % 50.0))
            .insert("radius", 50.0 + (y as f32 % 100.0))
            .insert("center_x", x)
            .insert("center_y", y);

        let script_path = format!(
            "{}/scripts/benchmark_entity.lua",
            env!("CARGO_MANIFEST_DIR")
        );

        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script_path, params),
        );

        self.physics
            .create_body(entity, RigidBodyType::Dynamic, Vec2::new(x, y), 0.0)?;
        self.physics.lock_rotations(entity, true);
        self.physics.set_linear_damping(entity, 2.0); // High damping for smooth movement
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box { hx: 8.0, hy: 8.0 },
            Vec2::ZERO,
            1.0,
            0.5,
            0.1,
        )?;

        self.entity_count += 1;
        Ok(())
    }

    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        self.player_texture = Some(self.solid_texture(renderer, 32, [255, 255, 255, 255])?);
        self.platform_texture = Some(self.solid_texture(renderer, 64, [70, 80, 95, 255])?);
        Ok(())
    }

    fn solid_texture(
        &self,
        renderer: &mut Renderer,
        size: u32,
        color: [u8; 4],
    ) -> Result<TextureHandle> {
        let data: Vec<u8> = (0..(size * size)).flat_map(|_| color).collect();
        renderer.load_texture_from_rgba(&data, size, size)
    }

    fn spawn_player(&mut self) -> Result<()> {
        // Start player higher up so they fall onto the ground platform
        let position = Vec2::new(240.0, 200.0);
        let entity = self.world.spawn();

        self.world.insert(entity, Transform::new(position));

        if let Some(texture) = self.player_texture {
            let mut sprite = SpriteComponent::new(texture);
            sprite
                .sprite
                .set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
            sprite.sprite.tint = [0.25, 0.75, 1.0, 1.0];
            self.world.insert(entity, sprite);
        }

        self.world.insert(entity, ScriptTag("player".into()));
        let params = ScriptParams::default()
            // Give the scripted controller enough speed and jump strength to feel responsive.
            .insert("speed", 200.0)
            .insert("jump", 300.0); // Increased jump to compensate for stronger gravity

        // Use an absolute path so the script loads correctly when running from the
        // example's package directory.
        let script_path = format!(
            "{}/scripts/scripting_demo_player.lua",
            env!("CARGO_MANIFEST_DIR")
        );

        println!(
            "[ScriptingDemo] Attaching script to player: {} (exists: {})",
            script_path,
            Path::new(&script_path).exists()
        );

        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script_path, params),
        );

        self.physics
            .create_body(entity, RigidBodyType::Dynamic, position, 0.0)?;
        self.physics.lock_rotations(entity, true);
        // Lower damping for more responsive movement
        self.physics.set_linear_damping(entity, 0.1);
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box { hx: 16.0, hy: 16.0 },
            Vec2::ZERO,
            1.0,
            0.8,
            0.1,
        )?;

        self.player = Some(entity);
        Ok(())
    }

    fn spawn_platform(&mut self, center: Vec2, size: Vec2) -> Result<()> {
        let entity = self.world.spawn();

        // Calculate scale based on desired size and texture size (64x64)
        let texture_size = Vec2::new(64.0, 64.0);
        let scale = Vec2::new(size.x / texture_size.x, size.y / texture_size.y);

        // Create transform with position and calculated scale
        let mut transform = Transform::new(center);
        transform.scale = scale;
        self.world.insert(entity, transform);

        if let Some(texture) = self.platform_texture {
            let mut sprite = SpriteComponent::new(texture);
            // Set size using the scale we calculated
            sprite
                .sprite
                .set_size_px(Vec2::new(size.x, size.y), texture_size);
            sprite.sprite.tint = [0.3, 0.35, 0.4, 1.0];
            self.world.insert(entity, sprite);
        }

        self.physics
            .create_body(entity, RigidBodyType::Fixed, center, 0.0)?;
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box {
                hx: size.x * 0.5,
                hy: size.y * 0.5,
            },
            Vec2::ZERO,
            1.0,
            1.0,
            0.0,
        )?;

        Ok(())
    }

    fn sync_transforms_from_physics(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<Transform>()
            .into_iter()
            .map(|(e, _)| e)
            .collect();
        for entity in entities {
            if let Some(position) = self.physics.body_position(entity) {
                if let Some(transform) = self.world.get_mut::<Transform>(entity) {
                    transform.position = position;
                }
            }

            if let Some(rotation) = self.physics.body_rotation(entity) {
                if let Some(transform) = self.world.get_mut::<Transform>(entity) {
                    transform.rotation = rotation;
                }
            }
        }
    }

    fn update_sprite_transforms(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<SpriteComponent>()
            .into_iter()
            .map(|(e, _)| e)
            .collect();

        for entity in entities {
            if let (Some(transform), Some(sprite)) = (
                self.world.get::<Transform>(entity).cloned(),
                self.world.get_mut::<SpriteComponent>(entity),
            ) {
                sprite.sprite.transform.position = transform.position;
                sprite.sprite.transform.rotation = transform.rotation;
                sprite.sprite.transform.scale = transform.scale;
            }
        }
    }

    fn update_camera(&mut self) {
        // Temporarily disable camera follow to see if player is moving
        // if let Some(player) = self.player {
        //     if let Some(transform) = self.world.get::<Transform>(player) {
        //         // Camera position represents the center of the view, so align it with the
        //         // player's world position just like the other demos.
        //         self.camera.position = transform.position;
        //     }
        // }
        // Keep camera fixed for debugging
        self.camera.position = Vec2::new(480.0, 270.0); // Center of 960x540 screen
    }
}

impl ScriptingDemo {
    fn log_key_presses(&self, input: &InputState) {
        let keys = [
            (KeyCode::KeyA, "A"),
            (KeyCode::KeyD, "D"),
            (KeyCode::ArrowLeft, "Left"),
            (KeyCode::ArrowRight, "Right"),
            (KeyCode::Space, "Space"),
        ];

        for (key, label) in keys {
            if input.is_key_pressed(key) {
                //println!("[ScriptingDemo] Key pressed: {}", label);
            }
        }
    }
}

impl Game for ScriptingDemo {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.camera = ctx.screen_camera();
        self.create_textures(ctx.renderer())?;

        // Load a default font for HUD (using built-in font if available, or create a simple one)
        // For now, we'll try to load a font - if it fails, HUD just won't show text
        // In a real scenario, you'd include a font file
        self.font = None; // We'll add font loading if needed

        // Check if benchmark mode (press B during init, or set via env var)
        // For now, let's make it toggleable with a key press
        // Default to normal mode

        // Spawn a ground platform first (at the bottom of the screen)
        let screen_h = ctx.window().inner_size().height as f32;
        let screen_w = ctx.window().inner_size().width as f32;
        let ground_y = screen_h - 50.0;
        self.spawn_platform(Vec2::new(480.0, ground_y), Vec2::new(960.0, 50.0))?;

        // Spawn a wall on the right side for wall jumping (visible on screen)
        self.spawn_platform(
            Vec2::new(screen_w - 50.0, screen_h / 2.0),
            Vec2::new(50.0, screen_h - 100.0),
        )?;

        if !self.benchmark_mode {
            self.spawn_player()?;
            self.spawn_platform(Vec2::new(300.0, 420.0), Vec2::new(240.0, 24.0))?;
            self.spawn_platform(Vec2::new(520.0, 520.0), Vec2::new(420.0, 24.0))?;

            // Spawn comprehensive test entity
            self.test_entity = Some(self.spawn_comprehensive_test_entity()?);
        } else {
            // Spawn many benchmark entities
            let grid_size = 20;
            let spacing = 40.0;
            let start_x = 100.0;
            let start_y = 100.0;
            for i in 0..grid_size {
                for j in 0..grid_size {
                    let x = start_x + i as f32 * spacing;
                    let y = start_y + j as f32 * spacing;
                    self.spawn_benchmark_entity(x, y)?;
                }
            }
        }
        self.camera.zoom = 1.25;

        // Ensure scripts are attached and their startup callbacks run before the first frame.
        // This guarantees that `on_start` side effects (logging, tint, etc.) happen even if the
        // first update is delayed by platform event scheduling.
        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), 0.0)?;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();

        // Toggle benchmark mode with B key
        if ctx.input().is_key_pressed(KeyCode::KeyB) {
            self.benchmark_mode = !self.benchmark_mode;
            // Clear and respawn
            self.world = World::new();
            self.physics = PhysicsWorld::new();
            self.physics.set_gravity(Vec2::new(0.0, 500.0));
            self.entity_count = 0;
            self.init(ctx)?;
        }

        // Measure script execution time
        let script_start = std::time::Instant::now();
        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), dt)?;
        self.script_time_ms = script_start.elapsed().as_secs_f32() * 1000.0;

        while ctx.should_run_fixed_update() {
            let fixed_dt = ctx.fixed_delta_time().as_secs_f32();
            self.runtime
                .fixed_update(&mut self.world, &mut self.physics, ctx.input(), fixed_dt)?;

            self.physics.step(fixed_dt);
            let events = self.physics.drain_events();

            // Track test entity collisions/triggers
            if let Some(test_entity) = self.test_entity {
                for event in &events {
                    match event {
                        sindri::physics::PhysicsEvent::CollisionEnter { a, b }
                        | sindri::physics::PhysicsEvent::CollisionExit { a, b } => {
                            if *a == test_entity || *b == test_entity {
                                if matches!(
                                    event,
                                    sindri::physics::PhysicsEvent::CollisionEnter { .. }
                                ) {
                                    self.test_stats.collision_count += 1;
                                }
                            }
                        }
                        sindri::physics::PhysicsEvent::TriggerEnter { a, b }
                        | sindri::physics::PhysicsEvent::TriggerExit { a, b } => {
                            if *a == test_entity || *b == test_entity {
                                if matches!(
                                    event,
                                    sindri::physics::PhysicsEvent::TriggerEnter { .. }
                                ) {
                                    self.test_stats.trigger_count += 1;
                                }
                            }
                        }
                    }
                }
            }

            self.runtime.handle_physics_events(
                &events,
                &mut self.world,
                &mut self.physics,
                ctx.input(),
            )?;
            self.sync_transforms_from_physics();
        }

        self.update_sprite_transforms();
        self.update_camera();

        // Update FPS counter
        self.frame_count += 1;
        let now = std::time::Instant::now();
        if now.duration_since(self.last_fps_update).as_secs_f32() >= 0.5 {
            let elapsed = now.duration_since(self.last_fps_update).as_secs_f32();
            self.current_fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.last_fps_update = now;
        }

        self.log_key_presses(ctx.input());

        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.08, 0.09, 0.12, 1.0])?;

        for (_, sprite) in self.world.query::<SpriteComponent>() {
            if sprite.visible {
                renderer.draw_sprite(&mut frame, &sprite.sprite, &self.camera)?;
            }
        }

        // Draw visual HUD with test stats
        self.hud.clear();

        if !self.benchmark_mode && self.test_entity.is_some() {
            // Draw status panel background
            self.hud.add_rect(sindri::hud::HudRect {
                position: Vec2::new(5.0, 5.0),
                size: Vec2::new(280.0, 200.0),
                color: [0.0, 0.0, 0.0, 0.7], // Semi-transparent black
            });

            // Status indicators as colored squares (visual feedback)
            let start_color = if self.test_stats.start_called {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0, 1.0]
            };
            self.hud.add_rect(sindri::hud::HudRect {
                position: Vec2::new(15.0, 15.0),
                size: Vec2::new(30.0, 30.0),
                color: start_color,
            });

            // Update counter indicator (green if updating)
            let update_color = if self.test_stats.update_count > 0 {
                [0.0, 1.0, 0.0, 1.0]
            } else {
                [0.5, 0.5, 0.5, 1.0]
            };
            self.hud.add_rect(sindri::hud::HudRect {
                position: Vec2::new(15.0, 55.0),
                size: Vec2::new(30.0, 30.0),
                color: update_color,
            });

            // Collision indicator
            let collision_color = if self.test_stats.collision_count > 0 {
                [0.0, 0.5, 1.0, 1.0]
            } else {
                [0.3, 0.3, 0.3, 1.0]
            };
            self.hud.add_rect(sindri::hud::HudRect {
                position: Vec2::new(15.0, 95.0),
                size: Vec2::new(30.0, 30.0),
                color: collision_color,
            });

            // Trigger indicator
            let trigger_color = if self.test_stats.trigger_count > 0 {
                [1.0, 0.5, 0.0, 1.0]
            } else {
                [0.3, 0.3, 0.3, 1.0]
            };
            self.hud.add_rect(sindri::hud::HudRect {
                position: Vec2::new(15.0, 135.0),
                size: Vec2::new(30.0, 30.0),
                color: trigger_color,
            });

            // Position indicator (small square that moves)
            if let Some(pos) = self.test_stats.last_position {
                // Draw a small indicator at the test entity's position (scaled to screen)
                let screen_pos = Vec2::new(60.0 + (pos.x / 10.0) % 200.0, 175.0);
                self.hud.add_rect(sindri::hud::HudRect {
                    position: screen_pos,
                    size: Vec2::new(10.0, 10.0),
                    color: [1.0, 1.0, 0.0, 1.0], // Yellow dot
                });
            }
        } else if self.benchmark_mode {
            // Benchmark mode - draw performance panel
            self.hud.add_rect(sindri::hud::HudRect {
                position: Vec2::new(5.0, 5.0),
                size: Vec2::new(200.0, 100.0),
                color: [0.0, 0.0, 0.0, 0.7],
            });
        }

        self.hud.draw(renderer, &mut frame)?;

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri Scripting Demo")
        .with_size(960, 540)
        .run(ScriptingDemo::new()?)
}
