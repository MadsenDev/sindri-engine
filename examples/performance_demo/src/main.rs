use anyhow::Result;
use sindri::{
    hud::{HudLayer, HudText, TextAlign},
    math::{Camera2D, Vec2},
    physics::{ColliderShape, PhysicsWorld, RigidBodyType},
    render::{Renderer, Sprite, TextureHandle},
    Engine, Game, KeyCode, World,
};
use std::time::Instant;

struct PerformanceDemo {
    camera: Camera2D,
    physics: PhysicsWorld,
    world: World,

    // Entities
    entities: Vec<Entity>,
    ground_entity: Option<sindri::EntityId>,

    // Textures
    box_texture: Option<TextureHandle>,
    circle_texture: Option<TextureHandle>,

    // Performance tracking
    frame_count: u64,
    last_fps_update: Instant,
    current_fps: f32,
    physics_time: f32,
    render_time: f32,

    // UI
    hud: HudLayer,
    font: Option<sindri::FontHandle>,
    initialized: bool,

    // Controls
    spawn_timer: f32,
    spawn_interval: f32,
    auto_spawn: bool,
}

struct Entity {
    id: sindri::EntityId,
    color: [f32; 4],
    size: f32,
    is_circle: bool,
}

impl PerformanceDemo {
    fn new() -> Self {
        Self {
            camera: Camera2D::new(Vec2::new(640.0, 360.0)), // Fixed position with ground visible
            physics: PhysicsWorld::new(),
            world: World::new(),
            entities: Vec::new(),
            ground_entity: None,
            box_texture: None,
            circle_texture: None,
            frame_count: 0,
            last_fps_update: Instant::now(),
            current_fps: 0.0,
            physics_time: 0.0,
            render_time: 0.0,
            hud: HudLayer::new(),
            font: None,
            initialized: false,
            spawn_timer: 0.0,
            spawn_interval: 0.1, // Spawn every 0.1 seconds
            auto_spawn: false,
        }
    }

    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        // Box texture (32x32 white square)
        let box_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [255u8, 255, 255, 255])
            .collect();
        self.box_texture = Some(renderer.load_texture_from_rgba(&box_data, 32, 32)?);

        // Circle texture (32x32, we'll draw a circle)
        let mut circle_data = vec![0u8; 4 * 32 * 32];
        let center = 16.0;
        for y in 0..32 {
            for x in 0..32 {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < 15.0 {
                    let idx = (y * 32 + x) * 4;
                    circle_data[idx] = 255;
                    circle_data[idx + 1] = 255;
                    circle_data[idx + 2] = 255;
                    circle_data[idx + 3] = 255;
                }
            }
        }
        self.circle_texture = Some(renderer.load_texture_from_rgba(&circle_data, 32, 32)?);

        Ok(())
    }

    fn spawn_physics_object(&mut self, x: f32, y: f32, shape: ColliderShape) -> Result<()> {
        let entity = self.world.spawn();

        // Random color
        let r = fastrand::f32();
        let g = fastrand::f32();
        let b = fastrand::f32();
        let color = [r, g, b, 1.0];

        // Random size (20-40 pixels)
        let size = fastrand::f32() * 20.0 + 20.0;

        // Create physics body
        self.physics
            .create_body(entity, RigidBodyType::Dynamic, Vec2::new(x, y), 0.0)?;

        // Add collider
        self.physics.add_collider_with_material(
            entity,
            shape,
            Vec2::ZERO,
            1.0, // density
            0.5, // friction
            0.3, // restitution
        )?;

        // Random initial velocity (increased for more dynamic movement)
        let vel_x = (fastrand::f32() - 0.5) * 400.0;
        let vel_y = (fastrand::f32() - 0.5) * 400.0;
        self.physics
            .set_linear_velocity(entity, Vec2::new(vel_x, vel_y));

        let is_circle = matches!(shape, ColliderShape::Circle { .. });
        self.entities.push(Entity {
            id: entity,
            color,
            size,
            is_circle,
        });

        Ok(())
    }

    fn spawn_batch(&mut self, count: usize) -> Result<()> {
        let start_x = 200.0;
        let start_y = 100.0;
        let spacing = 50.0;

        for i in 0..count {
            let x = start_x + (i as f32 % 20.0) * spacing;
            let y = start_y + (i as f32 / 20.0).floor() * spacing;

            // Alternate between box and circle
            let shape = if i % 2 == 0 {
                ColliderShape::Box { hx: 15.0, hy: 15.0 }
            } else {
                ColliderShape::Circle { radius: 15.0 }
            };

            self.spawn_physics_object(x, y, shape)?;
        }

        Ok(())
    }

    fn setup_ground(&mut self) -> Result<()> {
        let ground_entity = self.world.spawn();

        // Create ground platform
        self.physics.create_body(
            ground_entity,
            RigidBodyType::Fixed,
            Vec2::new(640.0, 700.0),
            0.0,
        )?;

        self.physics.add_collider_with_material(
            ground_entity,
            ColliderShape::Box {
                hx: 600.0,
                hy: 30.0,
            },
            Vec2::ZERO,
            0.0,
            0.7,
            0.3,
        )?;

        self.ground_entity = Some(ground_entity);
        Ok(())
    }
}

impl Game for PerformanceDemo {
    fn init(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        self.create_textures(&mut *ctx.renderer())?;

        // Set appropriate gravity for 2D game (stronger than default)
        self.physics.set_gravity(sindri::Vec2::new(0.0, 400.0));

        self.setup_ground()?;

        // Load font
        self.font = Some(ctx.builtin_font(sindri::BuiltinFont::Ui)?);

        // Spawn initial batch
        self.spawn_batch(100)?;

        self.initialized = true;
        Ok(())
    }

    fn update(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        let input = ctx.input();
        let dt = ctx.delta_time().as_secs_f32();

        // Toggle auto-spawn
        if input.is_key_pressed(KeyCode::Space) {
            self.auto_spawn = !self.auto_spawn;
        }

        // Spawn batch on click
        if input.is_mouse_pressed(sindri::MouseButton::Left) {
            let mouse_world = ctx.mouse_world(&self.camera);
            let shape = if fastrand::usize(..2) == 0 {
                ColliderShape::Box { hx: 15.0, hy: 15.0 }
            } else {
                ColliderShape::Circle { radius: 15.0 }
            };
            self.spawn_physics_object(mouse_world.x, mouse_world.y, shape)?;
        }

        // Spawn large batch with 'S' key
        if input.is_key_pressed(KeyCode::KeyS) {
            self.spawn_batch(100)?;
        }

        // Clear all with 'C' key
        if input.is_key_pressed(KeyCode::KeyC) {
            // Remove all entities except ground
            for entity in &self.entities {
                let _ = self.physics.remove_body(entity.id);
            }
            self.entities.clear();
        }

        // Auto-spawn
        if self.auto_spawn {
            self.spawn_timer += dt;
            if self.spawn_timer >= self.spawn_interval {
                self.spawn_timer = 0.0;
                // Spawn a few objects at random positions
                for _ in 0..5 {
                    let x = fastrand::f32() * 1000.0 + 100.0;
                    let y = fastrand::f32() * 200.0 + 50.0;
                    let shape = if fastrand::usize(..2) == 0 {
                        ColliderShape::Box { hx: 15.0, hy: 15.0 }
                    } else {
                        ColliderShape::Circle { radius: 15.0 }
                    };
                    let _ = self.spawn_physics_object(x, y, shape);
                }
            }
        }

        // Update physics (clamp dt for stability, but use actual frame time)
        let physics_start = Instant::now();
        let clamped_dt = dt.min(1.0 / 30.0); // Cap at 30 FPS minimum for stability
        self.physics.step(clamped_dt);
        self.physics_time = physics_start.elapsed().as_secs_f32() * 1000.0; // Convert to ms

        // Keep camera fixed with ground visible near bottom of screen
        // Ground is at y=700, screen height is 720, so center camera so ground is near bottom
        // Camera position represents center of view, so we want camera.y to be around 360 (half of 720)
        // This puts the ground (y=700) at the bottom of the screen
        self.camera.position = Vec2::new(640.0, 360.0); // Fixed position with ground visible

        // Update FPS counter
        self.frame_count += 1;
        let now = Instant::now();
        if now.duration_since(self.last_fps_update).as_secs_f32() >= 0.5 {
            let elapsed = now.duration_since(self.last_fps_update).as_secs_f32();
            self.current_fps = self.frame_count as f32 / elapsed;
            self.frame_count = 0;
            self.last_fps_update = now;
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        let render_start = Instant::now();
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        // Clear
        renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;

        // Draw ground
        if let Some(ground_entity) = self.ground_entity {
            if let Some(pos) = self.physics.body_position(ground_entity) {
                if let Some(box_tex) = self.box_texture {
                    let mut sprite = Sprite::new(box_tex);
                    sprite.transform.position = pos;
                    sprite.set_size_px(Vec2::new(1200.0, 60.0), Vec2::new(32.0, 32.0));
                    sprite.tint = [0.3, 0.3, 0.3, 1.0];
                    let _ = renderer.draw_sprite(&mut frame, &sprite, &self.camera);
                }
            }
        }

        // Draw all entities
        for entity in &self.entities {
            if let Some(pos) = self.physics.body_position(entity.id) {
                if let Some(rot) = self.physics.body_rotation(entity.id) {
                    // Choose texture based on shape
                    let texture = if entity.is_circle {
                        self.circle_texture
                    } else {
                        self.box_texture
                    };

                    if let Some(tex) = texture {
                        let mut sprite = Sprite::new(tex);
                        sprite.transform.position = pos;
                        sprite.transform.rotation = rot;
                        sprite.set_size_px(
                            Vec2::new(entity.size, entity.size),
                            Vec2::new(32.0, 32.0),
                        );
                        sprite.tint = entity.color;
                        let _ = renderer.draw_sprite(&mut frame, &sprite, &self.camera);
                    }
                }
            }
        }

        // Draw HUD (top-left corner, very close to edge)
        self.hud.clear();
        if let Some(font) = self.font {
            self.hud.add_text(HudText {
                text: "Performance Benchmark Demo".to_string(),
                font,
                size: 22.0,
                position: Vec2::new(5.0, 2.0),
                color: [1.0, 1.0, 1.0, 1.0],
                align: TextAlign::Left,
            });

            self.hud.add_text(HudText {
                text: format!("FPS: {:.1}", self.current_fps),
                font,
                size: 18.0,
                position: Vec2::new(5.0, 26.0),
                color: [0.0, 1.0, 0.0, 1.0],
                align: TextAlign::Left,
            });

            self.hud.add_text(HudText {
                text: format!("Objects: {}", self.entities.len()),
                font,
                size: 16.0,
                position: Vec2::new(5.0, 46.0),
                color: [0.9, 0.9, 0.9, 1.0],
                align: TextAlign::Left,
            });

            self.hud.add_text(HudText {
                text: format!("Physics: {:.2}ms", self.physics_time),
                font,
                size: 16.0,
                position: Vec2::new(5.0, 64.0),
                color: [0.9, 0.9, 0.9, 1.0],
                align: TextAlign::Left,
            });

            self.hud.add_text(HudText {
                text: format!("Render: {:.2}ms", self.render_time),
                font,
                size: 16.0,
                position: Vec2::new(5.0, 82.0),
                color: [0.9, 0.9, 0.9, 1.0],
                align: TextAlign::Left,
            });

            self.hud.add_text(HudText {
                text: "Controls:".to_string(),
                font,
                size: 14.0,
                position: Vec2::new(5.0, 105.0),
                color: [0.7, 0.7, 0.7, 1.0],
                align: TextAlign::Left,
            });

            self.hud.add_text(HudText {
                text: "Left Click: Spawn | S: Spawn 100 | C: Clear | Space: Auto-spawn".to_string(),
                font,
                size: 12.0,
                position: Vec2::new(5.0, 121.0),
                color: [0.6, 0.6, 0.6, 1.0],
                align: TextAlign::Left,
            });
        }
        self.hud.draw(renderer, &mut frame)?;

        renderer.end_frame(frame)?;
        self.render_time = render_start.elapsed().as_secs_f32() * 1000.0; // Convert to ms

        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri Performance Benchmark")
        .with_size(1280, 720)
        .with_vsync(true) // VSync enabled (some systems don't support immediate mode)
        .run(PerformanceDemo::new())
}
