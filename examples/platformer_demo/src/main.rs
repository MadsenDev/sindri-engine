use anyhow::Result;
use sindri::{
    camera::{update_camera_follow, CameraFollow},
    math::{Camera2D, Vec2},
    physics::{ColliderShape, PhysicsWorld, RigidBodyType},
    render::{Renderer, Sprite, TextureHandle},
    Engine, Game, KeyCode,
};

struct PlatformerDemo {
    camera: Camera2D,
    physics: PhysicsWorld,
    world: sindri::World,

    player_entity: sindri::EntityId,
    textures: TextureHandles,

    // Player state
    is_grounded: bool,
    jump_cooldown: f32,

    // Camera follow system
    camera_follow: CameraFollow,

    // Initialization flag
    initialized: bool,
}

struct TextureHandles {
    player: Option<TextureHandle>,
    platform: Option<TextureHandle>,
    ground: Option<TextureHandle>,
}

impl PlatformerDemo {
    fn new() -> Self {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(Vec2::new(0.0, 600.0)); // Platformer gravity

        Self {
            camera: Camera2D::default(),
            physics,
            world: sindri::World::new(),
            player_entity: unsafe { std::mem::zeroed() }, // Will be set in init
            textures: TextureHandles {
                player: None,
                platform: None,
                ground: None,
            },
            is_grounded: false,
            jump_cooldown: 0.0,
            camera_follow: CameraFollow::new()
                .follow_entity(unsafe { std::mem::zeroed() }) // Will be set in init
                .with_dead_zone(150.0, 100.0) // Dead zone: player can move 150px horizontally, 100px vertically before camera moves
                .with_smoothing(0.1), // Smooth camera movement
            initialized: false,
        }
    }

    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        // Player (green rectangle)
        let player_data: Vec<u8> = (0..(4 * 32 * 48))
            .flat_map(|_| [100u8, 255, 100, 255])
            .collect();
        self.textures.player = Some(renderer.load_texture_from_rgba(&player_data, 32, 48)?);

        // Platform (brown)
        let platform_data: Vec<u8> = (0..(4 * 200 * 20))
            .flat_map(|_| [139u8, 90, 43, 255])
            .collect();
        self.textures.platform = Some(renderer.load_texture_from_rgba(&platform_data, 200, 20)?);

        // Ground (dark gray) - use smaller texture, we'll tile it
        let ground_data: Vec<u8> = (0..(4 * 400 * 40))
            .flat_map(|_| [60u8, 60, 60, 255])
            .collect();
        let ground_handle = renderer.load_texture_from_rgba(&ground_data, 400, 40)?;
        self.textures.ground = Some(ground_handle);

        println!(
            "Textures loaded: player={:?}, platform={:?}, ground={:?}",
            self.textures.player, self.textures.platform, self.textures.ground
        );

        Ok(())
    }

    fn spawn_platform(
        &mut self,
        position: Vec2,
        width: f32,
        height: f32,
    ) -> Result<sindri::EntityId> {
        let entity = self.world.spawn();

        // Create platform body (kinematic or fixed - let's use fixed for simplicity)
        self.physics
            .create_body(entity, RigidBodyType::Fixed, position, 0.0)?;

        // Add platform collider
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::Box {
                hx: width / 2.0,
                hy: height / 2.0,
            },
            Vec2::ZERO,
            0.0, // density (static)
            0.7, // friction
            0.0, // restitution
        )?;

        Ok(entity)
    }
}

impl Game for PlatformerDemo {
    fn init(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        self.camera = ctx.screen_camera();

        // Create textures first
        self.create_textures(&mut *ctx.renderer())?;

        // Verify textures were created
        if self.textures.player.is_none()
            || self.textures.platform.is_none()
            || self.textures.ground.is_none()
        {
            return Err(anyhow::anyhow!("Failed to create textures"));
        }

        let screen_size = ctx.window().inner_size();
        let screen_w = screen_size.width as f32;
        let screen_h = screen_size.height as f32;

        // Spawn player at starting position (this will set player_entity)
        let player_pos = Vec2::new(screen_w / 2.0, screen_h - 200.0);
        let entity = self.world.spawn();
        self.player_entity = entity;

        // Create player body (dynamic, can move and jump)
        self.physics
            .create_body(entity, RigidBodyType::Dynamic, player_pos, 0.0)?;

        // Add player collider (capsule shape for smooth platformer feel)
        self.physics.add_collider_with_material(
            entity,
            ColliderShape::CapsuleY {
                half_height: 20.0,
                radius: 12.0,
            },
            Vec2::ZERO,
            1.0, // density
            0.3, // friction (low for sliding)
            0.0, // restitution (no bounce)
        )?;

        // Set low damping - we handle movement manually for better control
        self.physics.set_linear_damping(entity, 0.0);
        self.physics.set_angular_damping(entity, 0.5);

        // Lock rotation (platformer characters shouldn't rotate)
        self.physics.lock_rotations(entity, true);

        // Set camera to player's initial position
        self.camera.position = player_pos;

        // Set up camera follow to track the player
        self.camera_follow = CameraFollow::new()
            .follow_entity(entity)
            .with_dead_zone(150.0, 100.0) // Dead zone: player can move 150px horizontally, 100px vertically before camera moves
            .with_smoothing(0.1); // Smooth camera movement

        // Create ground
        let ground_y = screen_h - 40.0;
        self.spawn_platform(Vec2::new(screen_w / 2.0, ground_y), 2000.0, 40.0)?;

        // Create platforms at different heights
        let platform_y_base = screen_h - 150.0;
        self.spawn_platform(Vec2::new(200.0, platform_y_base), 150.0, 20.0)?;
        self.spawn_platform(Vec2::new(500.0, platform_y_base - 100.0), 150.0, 20.0)?;
        self.spawn_platform(Vec2::new(800.0, platform_y_base - 200.0), 150.0, 20.0)?;
        self.spawn_platform(Vec2::new(1100.0, platform_y_base - 100.0), 150.0, 20.0)?;
        self.spawn_platform(Vec2::new(1400.0, platform_y_base), 150.0, 20.0)?;

        // Higher platforms
        self.spawn_platform(Vec2::new(350.0, platform_y_base - 300.0), 120.0, 20.0)?;
        self.spawn_platform(Vec2::new(650.0, platform_y_base - 400.0), 120.0, 20.0)?;
        self.spawn_platform(Vec2::new(950.0, platform_y_base - 350.0), 120.0, 20.0)?;

        self.initialized = true;
        Ok(())
    }

    fn update(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        let input = ctx.input();
        let dt = ctx.delta_time().as_secs_f32();

        // Update jump cooldown
        if self.jump_cooldown > 0.0 {
            self.jump_cooldown -= dt;
        }

        // Check if player is grounded (simple check: velocity.y is near zero and position is low)
        if let Some(pos) = self.physics.body_position(self.player_entity) {
            if let Some(vel) = self.physics.linear_velocity(self.player_entity) {
                // Simple grounded check: low vertical velocity and near ground level
                let screen_h = ctx.window().inner_size().height as f32;
                let ground_level = screen_h - 40.0;
                self.is_grounded = vel.y.abs() < 10.0 && (pos.y - ground_level) < 50.0;
            }
        }

        // Player movement - direct velocity control for responsive platformer feel
        let move_speed = 300.0;
        let mut target_velocity_x = 0.0;

        if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
            target_velocity_x = -move_speed;
        }
        if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
            target_velocity_x = move_speed;
        }

        // Get current velocity
        if let Some(mut vel) = self.physics.linear_velocity(self.player_entity) {
            // Apply horizontal movement with quick acceleration and deceleration
            let acceleration = 2000.0; // High acceleration for quick response
            let deceleration = 2500.0; // Even higher deceleration for quick stopping

            if target_velocity_x != 0.0 {
                // Accelerate towards target velocity
                let diff = target_velocity_x - vel.x;
                let force = diff * acceleration * dt;
                vel.x += force;
                // Clamp to max speed
                vel.x = vel.x.clamp(-move_speed, move_speed);
            } else {
                // Quick deceleration when no input
                if vel.x.abs() > 1.0 {
                    let stop_force = -vel.x.signum() * deceleration * dt;
                    vel.x += stop_force;
                    // Stop completely if very slow
                    if vel.x.abs() < 10.0 {
                        vel.x = 0.0;
                    }
                } else {
                    vel.x = 0.0;
                }
            }

            // Set the new velocity directly
            self.physics.set_linear_velocity(self.player_entity, vel);
        }

        // Jumping
        if (input.is_key_pressed(KeyCode::Space)
            || input.is_key_pressed(KeyCode::KeyW)
            || input.is_key_pressed(KeyCode::ArrowUp))
            && self.is_grounded
            && self.jump_cooldown <= 0.0
        {
            let jump_force = Vec2::new(0.0, -400.0); // Negative Y is up - reduced for more reasonable jump height
            self.physics.apply_impulse(self.player_entity, jump_force);
            self.jump_cooldown = 0.2; // Small cooldown to prevent double jumps
            self.is_grounded = false;
        }

        // Fixed-step physics
        while ctx.should_run_fixed_update() {
            let dt = ctx.fixed_delta_time().as_secs_f32();
            self.physics.step(dt);
        }

        // Update camera follow system (handles dead-zone and smoothing)
        update_camera_follow(&mut self.camera, &self.camera_follow, &self.physics, dt);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        // Don't draw until initialization is complete
        if !self.initialized {
            return Ok(());
        }

        let screen_size = ctx.window().inner_size();
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        // Clear with sky blue
        renderer.clear(&mut frame, [0.5, 0.7, 1.0, 1.0])?;

        // Draw ground
        if let Some(tex) = self.textures.ground {
            let ground_y = screen_size.height as f32 - 40.0;
            let mut sprite = Sprite::new(tex);
            sprite.transform.position = Vec2::new(screen_size.width as f32 / 2.0, ground_y);
            sprite.set_size_px(Vec2::new(400.0, 40.0), Vec2::new(400.0, 40.0));
            if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                eprintln!("Error drawing ground: {}", e);
            }
        }

        // Draw platforms
        if let Some(platform_tex) = self.textures.platform {
            for entity in self.physics.all_entities_with_bodies() {
                if entity == self.player_entity {
                    continue; // Skip player
                }

                if let Some(pos) = self.physics.body_position(entity) {
                    if let Some(body_type) = self.physics.body_type(entity) {
                        // Only draw fixed bodies (platforms)
                        if matches!(body_type, RigidBodyType::Fixed) {
                            let mut sprite = Sprite::new(platform_tex);
                            sprite.transform.position = pos;

                            // Get platform size from collider
                            let colliders = self.physics.get_colliders(entity);
                            if let Some((shape, _, _, _, _, _)) = colliders.first() {
                                let size = match shape {
                                    ColliderShape::Box { hx, hy } => Vec2::new(hx * 2.0, hy * 2.0),
                                    _ => Vec2::new(200.0, 20.0),
                                };
                                sprite.set_size_px(size, size);
                                if let Err(e) =
                                    renderer.draw_sprite(&mut frame, &sprite, &self.camera)
                                {
                                    eprintln!("Error drawing platform: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Draw player
        if let Some(tex) = self.textures.player {
            if let Some(pos) = self.physics.body_position(self.player_entity) {
                if let Some(rot) = self.physics.body_rotation(self.player_entity) {
                    let mut sprite = Sprite::new(tex);
                    sprite.transform.position = pos;
                    sprite.transform.rotation = rot;
                    sprite.set_size_px(Vec2::new(32.0, 48.0), Vec2::new(32.0, 48.0));
                    if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                        eprintln!("Error drawing player: {}", e);
                    }
                }
            }
        }

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri Platformer Demo")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(PlatformerDemo::new())
}
