use anyhow::Result;
use sindri::{
    BuiltinFont, Camera2D, EmissionConfig, EngineContext, FontHandle, Game, HudLayer, HudText,
    KeyCode, ParticleEmitter, ParticleSystem, Sprite, TextureHandle, Vec2,
};

use crate::entities::{Asteroid, AsteroidSize, Bullet, Player};

pub struct AsteroidsGame {
    // Camera
    camera: Camera2D,

    // Screen bounds
    screen_width: f32,
    screen_height: f32,

    // No longer need textures - using vector shapes!

    // Font
    font: Option<FontHandle>,

    // Game entities
    player: Option<Player>,
    bullets: Vec<Bullet>,
    asteroids: Vec<Asteroid>,

    // Track which emitter index belongs to which bullet
    // This is needed because emitters can be removed when inactive, breaking index matching
    bullet_emitter_indices: Vec<usize>, // Maps bullet index to emitter index

    // Game state
    score: u32,
    lives: u32,
    game_over: bool,

    // Timing
    shoot_cooldown: f32,
    asteroid_spawn_timer: f32,

    // Particle system for bullet trails
    particle_system: ParticleSystem,
    white_texture: Option<TextureHandle>,

    // HUD
    hud: HudLayer,
}

impl AsteroidsGame {
    pub fn new() -> Self {
        Self {
            camera: Camera2D::default(),
            screen_width: 1280.0,
            screen_height: 720.0,
            font: None,
            player: None,
            bullets: Vec::new(),
            asteroids: Vec::new(),
            bullet_emitter_indices: Vec::new(),
            score: 0,
            lives: 3,
            game_over: false,
            shoot_cooldown: 0.0,
            asteroid_spawn_timer: 0.0,
            particle_system: ParticleSystem::new(),
            white_texture: None,
            hud: HudLayer::new(),
        }
    }

    fn spawn_player(&mut self, renderer: &mut sindri::Renderer) {
        // Create a dummy sprite for the player (we'll draw it as a polygon)
        let dummy_data = vec![255u8, 255, 255, 255];
        let dummy_texture = renderer.load_texture_from_rgba(&dummy_data, 1, 1).unwrap();
        let mut sprite = Sprite::new(dummy_texture);
        sprite.transform.position = Vec2::new(self.screen_width * 0.5, self.screen_height * 0.5);
        // Set size to match the ship triangle size (about 40x40 pixels)
        sprite.set_size_px(Vec2::new(40.0, 40.0), Vec2::new(1.0, 1.0));
        sprite.tint = [1.0, 1.0, 1.0, 1.0];
        self.player = Some(Player::new(sprite));
    }

    fn spawn_asteroid(
        &mut self,
        renderer: &mut sindri::Renderer,
        size: AsteroidSize,
        position: Option<Vec2>,
    ) {
        let pos = position.unwrap_or_else(|| {
            // Spawn at edge of screen
            let side = fastrand::u8(0..4);
            match side {
                0 => Vec2::new(0.0, fastrand::f32() * self.screen_height), // Left
                1 => Vec2::new(self.screen_width, fastrand::f32() * self.screen_height), // Right
                2 => Vec2::new(fastrand::f32() * self.screen_width, 0.0),  // Top
                _ => Vec2::new(fastrand::f32() * self.screen_width, self.screen_height), // Bottom
            }
        });

        // Create a dummy sprite (we'll draw it as a polygon)
        let dummy_data = vec![200u8, 200, 200, 255];
        let dummy_texture = renderer.load_texture_from_rgba(&dummy_data, 1, 1).unwrap();
        let mut sprite = Sprite::new(dummy_texture);
        sprite.transform.position = pos;
        // Set size based on asteroid size
        let asteroid_size = match size {
            AsteroidSize::Large => 80.0,
            AsteroidSize::Medium => 50.0,
            AsteroidSize::Small => 30.0,
        };
        sprite.set_size_px(Vec2::new(asteroid_size, asteroid_size), Vec2::new(1.0, 1.0));
        sprite.tint = [0.8, 0.8, 0.8, 1.0];

        let asteroid = Asteroid::new(sprite, pos, size);
        self.asteroids.push(asteroid);
    }

    fn get_ship_points(&self, player: &Player) -> Vec<Vec2> {
        // Local ship model: nose points RIGHT (matching Vec2::from_angle(0) = (1, 0))
        // In screen coordinates, this means the ship points right when rotation = 0
        // Classic asteroids ship: long pointed nose, narrow back
        let size = 20.0;
        let points = vec![
            Vec2::new(size, 0.0), // Nose (forward, pointing right) - long point
            Vec2::new(-size * 0.5, -size * 0.6), // Back-left - closer to center and narrower
            Vec2::new(-size * 0.5, size * 0.6), // Back-right - closer to center and narrower
        ];

        // Rotate and translate points using player's rotation
        // No offset needed - polygon matches Vec2::from_angle() coordinate system
        let cos = player.rotation.cos();
        let sin = player.rotation.sin();
        points
            .iter()
            .map(|&p| {
                let rotated = Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
                player.sprite.transform.position + rotated
            })
            .collect()
    }

    fn get_asteroid_points(&self, asteroid: &Asteroid) -> Vec<Vec2> {
        // Use pre-computed shape offsets and apply rotation + position
        let cos = asteroid.rotation.cos();
        let sin = asteroid.rotation.sin();

        asteroid
            .shape_offsets
            .iter()
            .map(|&offset| {
                // Rotate offset by asteroid rotation
                let rotated = Vec2::new(
                    offset.x * cos - offset.y * sin,
                    offset.x * sin + offset.y * cos,
                );
                asteroid.sprite.transform.position + rotated
            })
            .collect()
    }

    fn get_flame_points(&self, player: &Player, time: f32) -> Vec<Vec2> {
        // Flame animation: vary size and position based on time
        let animation_speed = 25.0; // How fast the flame flickers
        let base_size = 12.0;
        let size_variation = 4.0;

        // Animate flame size (oscillates between base_size and base_size + size_variation)
        let flame_size = base_size + (time * animation_speed).sin() * size_variation;

        // Position behind the ship (opposite of forward direction)
        // Start from the back of the ship (where the two back points meet)
        let back_dir = Vec2::from_angle(player.rotation + std::f32::consts::PI);
        let ship_size = 20.0;
        // Position flame further back, starting from the ship's back edge
        let ship_back_offset = ship_size * 0.5; // Half ship size to get to back
        let flame_base_pos = player.sprite.transform.position + back_dir * ship_back_offset;

        let cos = player.rotation.cos();
        let sin = player.rotation.sin();

        // Create flame shape: wider at the base (attached to ship), narrower at the tip
        // Flame points backward (opposite of ship direction)
        let local_points = vec![
            Vec2::new(-flame_size, 0.0),       // Tip of flame (furthest back)
            Vec2::new(0.0, -flame_size * 0.5), // Left base point (wider)
            Vec2::new(0.0, flame_size * 0.5),  // Right base point (wider)
        ];

        // Rotate and translate points
        local_points
            .iter()
            .map(|&p| {
                let rotated = Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
                flame_base_pos + rotated
            })
            .collect()
    }

    fn check_collision_circle_circle(pos1: Vec2, radius1: f32, pos2: Vec2, radius2: f32) -> bool {
        let dist_sq = pos1.distance_squared(pos2);
        let radius_sum = radius1 + radius2;
        dist_sq < radius_sum * radius_sum
    }
}

impl Game for AsteroidsGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let (w, h) = ctx.renderer().surface_size();
        self.screen_width = w as f32;
        self.screen_height = h as f32;

        // Set camera position so that (0,0) is at top-left of screen
        // Camera position represents the center of the view, so we offset by half screen size
        self.camera.position = Vec2::new(self.screen_width * 0.5, self.screen_height * 0.5);

        // Create white texture for particles
        let white_pixel = [255u8, 255u8, 255u8, 255u8];
        self.white_texture = Some(ctx.renderer().load_texture_from_rgba(&white_pixel, 1, 1)?);

        // Load font (no textures needed - using vector shapes!)
        self.font = Some(ctx.builtin_font(BuiltinFont::Ui)?);

        // Spawn player
        self.spawn_player(ctx.renderer());

        // Spawn initial asteroids
        for _ in 0..4 {
            self.spawn_asteroid(ctx.renderer(), AsteroidSize::Large, None);
        }

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();

        if self.game_over {
            let input = ctx.input();
            if input.is_key_pressed(KeyCode::KeyR) {
                // Restart game
                self.score = 0;
                self.lives = 3;
                self.game_over = false;
                self.bullets.clear();
                self.asteroids.clear();
                self.bullet_emitter_indices.clear();
                self.particle_system.clear();
                self.spawn_player(ctx.renderer());
                for _ in 0..4 {
                    self.spawn_asteroid(ctx.renderer(), AsteroidSize::Large, None);
                }
            }
            // Still update particle system to let particles fade out
            self.particle_system.update(dt);
            return Ok(());
        }

        // Update timers
        if self.shoot_cooldown > 0.0 {
            self.shoot_cooldown -= dt;
        }
        self.asteroid_spawn_timer += dt;

        // Spawn asteroids periodically (get renderer before input to avoid borrow conflicts)
        if self.asteroid_spawn_timer >= 5.0 && self.asteroids.len() < 10 {
            self.spawn_asteroid(ctx.renderer(), AsteroidSize::Large, None);
            self.asteroid_spawn_timer = 0.0;
        }

        let input = ctx.input();

        // Update player
        if let Some(ref mut player) = self.player {
            // Rotation
            if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
                player.rotation -= player.rotation_speed * dt;
            }
            if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
                player.rotation += player.rotation_speed * dt;
            }

            // Thrust
            if input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp) {
                let thrust_dir = Vec2::from_angle(player.rotation);
                player.velocity += thrust_dir * player.thrust_power * dt;
            }

            // Apply friction
            player.velocity *= player.friction;

            // Limit speed
            if player.velocity.length() > player.max_speed {
                player.velocity = player.velocity.normalized() * player.max_speed;
            }

            // Update position
            player.sprite.transform.position += player.velocity * dt;
            player.sprite.transform.rotation = player.rotation;

            // Wrap position (inline to avoid borrow checker issues)
            let screen_w = self.screen_width;
            let screen_h = self.screen_height;
            if player.sprite.transform.position.x < 0.0 {
                player.sprite.transform.position.x = screen_w;
            } else if player.sprite.transform.position.x > screen_w {
                player.sprite.transform.position.x = 0.0;
            }
            if player.sprite.transform.position.y < 0.0 {
                player.sprite.transform.position.y = screen_h;
            } else if player.sprite.transform.position.y > screen_h {
                player.sprite.transform.position.y = 0.0;
            }

            // Shooting
            if (input.is_key_down(KeyCode::Space) || input.is_key_pressed(KeyCode::Space))
                && self.shoot_cooldown <= 0.0
            {
                let bullet_pos = player.sprite.transform.position;
                let bullet_dir = Vec2::from_angle(player.rotation);
                // Create a dummy sprite (we'll draw bullets as circles)
                let dummy_data = vec![255u8, 255, 255, 255];
                let dummy_texture = ctx.renderer().load_texture_from_rgba(&dummy_data, 1, 1)?;
                let mut bullet_sprite = Sprite::new(dummy_texture);
                bullet_sprite.transform.position = bullet_pos + bullet_dir * 25.0; // Spawn in front of ship
                                                                                   // Set size to 6x6 pixels (radius 3.0 * 2)
                bullet_sprite.set_size_px(Vec2::new(6.0, 6.0), Vec2::new(1.0, 1.0));
                bullet_sprite.tint = [1.0, 1.0, 1.0, 1.0];

                let bullet = Bullet::new(
                    bullet_sprite,
                    bullet_pos + bullet_dir * 25.0,
                    bullet_dir,
                    400.0,
                );
                let bullet_spawn_pos = bullet_pos + bullet_dir * 25.0;
                self.bullets.push(bullet);
                self.shoot_cooldown = 0.2; // 5 shots per second

                // Create particle trail for the bullet
                let trail_config = EmissionConfig::new(bullet_spawn_pos)
                    .with_rate(250.0) // Very high rate for dense trail
                    .with_velocity(
                        -bullet_dir * 120.0 + Vec2::new(-25.0, -25.0), // Spread opposite to bullet direction
                        -bullet_dir * 120.0 + Vec2::new(25.0, 25.0),
                    )
                    .with_size(Vec2::new(4.0, 4.0), Vec2::new(8.0, 8.0)) // Even larger particles
                    .with_color([1.0, 1.0, 0.3, 1.0], Some([1.0, 0.5, 0.0, 0.0])) // Very bright yellow to orange
                    .with_lifetime(0.6, 1.2)
                    .with_acceleration(Vec2::new(0.0, 0.0))
                    .with_size_end_multiplier(0.6)
                    .with_fade_out(false); // Don't fade alpha separately, let color handle it

                let mut trail_emitter = ParticleEmitter::new(trail_config)
                    .with_max_particles(100)
                    .with_texture(self.white_texture);
                let emitter_index = self.particle_system.emitters().len();
                self.particle_system.add_emitter(trail_emitter);
                // Track that this bullet has this emitter
                self.bullet_emitter_indices.push(emitter_index);
            }
        }

        // Update bullets
        let screen_w = self.screen_width;
        let screen_h = self.screen_height;
        for bullet in &mut self.bullets {
            bullet.sprite.transform.position += bullet.velocity * dt;
            // Wrap position (inline to avoid borrow checker issues)
            if bullet.sprite.transform.position.x < 0.0 {
                bullet.sprite.transform.position.x = screen_w;
            } else if bullet.sprite.transform.position.x > screen_w {
                bullet.sprite.transform.position.x = 0.0;
            }
            if bullet.sprite.transform.position.y < 0.0 {
                bullet.sprite.transform.position.y = screen_h;
            } else if bullet.sprite.transform.position.y > screen_h {
                bullet.sprite.transform.position.y = 0.0;
            }
            bullet.lifetime -= dt;
        }

        // Before removing bullets, stop their emitters
        let bullets_before = self.bullets.len();

        // Build a set of indices to keep
        let mut keep_indices = Vec::new();
        for (i, bullet) in self.bullets.iter().enumerate() {
            if bullet.lifetime > 0.0 {
                keep_indices.push(i);
            }
        }

        // Stop emitters for bullets that will be removed
        for (i, bullet) in self.bullets.iter().enumerate() {
            if bullet.lifetime <= 0.0 {
                // This bullet will be removed - stop its emitter
                if let Some(&emitter_idx) = self.bullet_emitter_indices.get(i) {
                    if let Some(emitter) = self.particle_system.emitters_mut().get_mut(emitter_idx)
                    {
                        emitter.stop_emission();
                    }
                }
            }
        }

        // Remove bullets and update emitter index mapping
        self.bullets.retain(|b| b.lifetime > 0.0);
        let mut new_emitter_indices = Vec::new();
        for &keep_idx in &keep_indices {
            if let Some(&emitter_idx) = self.bullet_emitter_indices.get(keep_idx) {
                new_emitter_indices.push(emitter_idx);
            }
        }
        self.bullet_emitter_indices = new_emitter_indices;

        // Update particle emitter positions to follow bullets BEFORE updating particle system
        // This ensures emitters are positioned correctly before particles are spawned
        // Use the bullet_emitter_indices mapping to find the correct emitter for each bullet
        for (bullet_idx, bullet) in self.bullets.iter().enumerate() {
            if let Some(&emitter_idx) = self.bullet_emitter_indices.get(bullet_idx) {
                // Skip invalid indices (usize::MAX)
                if emitter_idx != usize::MAX && emitter_idx < self.particle_system.emitters().len()
                {
                    if let Some(emitter) = self.particle_system.emitters_mut().get_mut(emitter_idx)
                    {
                        emitter.set_position(bullet.sprite.transform.position);
                    }
                }
            }
        }

        // Update particle system (this will spawn new particles and remove dead ones)
        let emitter_count_before = self.particle_system.emitters().len();
        self.particle_system.update(dt);
        let emitter_count_after = self.particle_system.emitters().len();

        // If emitters were removed, we can't reliably rebuild the mapping
        // because indices shift. Instead, we'll just clear invalid mappings
        // and let the system handle it gracefully
        if emitter_count_after < emitter_count_before {
            // Remove any mappings that point to invalid emitter indices
            self.bullet_emitter_indices
                .retain(|&idx| idx < emitter_count_after);

            // If we have fewer mappings than bullets, some bullets lost their emitters
            // This shouldn't happen in normal operation, but handle it gracefully
            let bullet_count = self.bullets.len();
            if self.bullet_emitter_indices.len() < bullet_count {
                // Trim bullets to match (shouldn't happen, but be safe)
                // Actually, we should keep bullets and just not have particles for them
                // So we'll just leave it as is - bullets without emitters won't have trails
            }
        }

        // Update asteroids
        for asteroid in &mut self.asteroids {
            asteroid.sprite.transform.position += asteroid.velocity * dt;
            // Wrap position (inline to avoid borrow checker issues)
            if asteroid.sprite.transform.position.x < 0.0 {
                asteroid.sprite.transform.position.x = screen_w;
            } else if asteroid.sprite.transform.position.x > screen_w {
                asteroid.sprite.transform.position.x = 0.0;
            }
            if asteroid.sprite.transform.position.y < 0.0 {
                asteroid.sprite.transform.position.y = screen_h;
            } else if asteroid.sprite.transform.position.y > screen_h {
                asteroid.sprite.transform.position.y = 0.0;
            }
            asteroid.rotation += asteroid.rotation_speed * dt;
            asteroid.sprite.transform.rotation = asteroid.rotation;
        }

        // Bullet-Asteroid collisions
        let mut bullets_to_remove = Vec::new();
        let mut asteroids_to_remove = Vec::new();
        let mut new_asteroids = Vec::new();

        for (bullet_idx, bullet) in self.bullets.iter().enumerate() {
            for (asteroid_idx, asteroid) in self.asteroids.iter().enumerate() {
                if Self::check_collision_circle_circle(
                    bullet.sprite.transform.position,
                    4.0, // bullet radius
                    asteroid.sprite.transform.position,
                    asteroid.radius(),
                ) {
                    bullets_to_remove.push(bullet_idx);
                    asteroids_to_remove.push(asteroid_idx);

                    // Break asteroid into smaller pieces
                    match asteroid.size {
                        AsteroidSize::Large => {
                            // Spawn 2 medium asteroids
                            for _ in 0..2 {
                                let dummy_data = vec![200u8, 200, 200, 255];
                                let dummy_texture = ctx
                                    .renderer()
                                    .load_texture_from_rgba(&dummy_data, 1, 1)
                                    .unwrap();
                                let mut sprite = Sprite::new(dummy_texture);
                                sprite.transform.position = asteroid.sprite.transform.position;
                                sprite.set_size_px(Vec2::new(50.0, 50.0), Vec2::new(1.0, 1.0));
                                sprite.tint = [0.8, 0.8, 0.8, 1.0];
                                let new_ast = Asteroid::new(
                                    sprite,
                                    asteroid.sprite.transform.position,
                                    AsteroidSize::Medium,
                                );
                                new_asteroids.push(new_ast);
                            }
                            self.score += 20;
                        }
                        AsteroidSize::Medium => {
                            // Spawn 2 small asteroids
                            for _ in 0..2 {
                                let dummy_data = vec![200u8, 200, 200, 255];
                                let dummy_texture = ctx
                                    .renderer()
                                    .load_texture_from_rgba(&dummy_data, 1, 1)
                                    .unwrap();
                                let mut sprite = Sprite::new(dummy_texture);
                                sprite.transform.position = asteroid.sprite.transform.position;
                                sprite.set_size_px(Vec2::new(30.0, 30.0), Vec2::new(1.0, 1.0));
                                sprite.tint = [0.8, 0.8, 0.8, 1.0];
                                let new_ast = Asteroid::new(
                                    sprite,
                                    asteroid.sprite.transform.position,
                                    AsteroidSize::Small,
                                );
                                new_asteroids.push(new_ast);
                            }
                            self.score += 50;
                        }
                        AsteroidSize::Small => {
                            // Just destroy it
                            self.score += 100;
                        }
                    }
                    break;
                }
            }
        }

        // Remove bullets and asteroids (in reverse order to maintain indices)
        bullets_to_remove.sort();
        bullets_to_remove.reverse();
        for idx in bullets_to_remove {
            // Stop the emitter for this bullet before removing it
            if let Some(&emitter_idx) = self.bullet_emitter_indices.get(idx) {
                if let Some(emitter) = self.particle_system.emitters_mut().get_mut(emitter_idx) {
                    emitter.stop_emission();
                }
            }
            self.bullets.remove(idx);
            self.bullet_emitter_indices.remove(idx);
        }

        asteroids_to_remove.sort();
        asteroids_to_remove.reverse();
        for idx in asteroids_to_remove {
            self.asteroids.remove(idx);
        }

        // Add new asteroids
        self.asteroids.extend(new_asteroids);

        // Player-Asteroid collisions
        if let Some(ref player) = self.player {
            for asteroid in &self.asteroids {
                if Self::check_collision_circle_circle(
                    player.sprite.transform.position,
                    16.0, // player radius
                    asteroid.sprite.transform.position,
                    asteroid.radius(),
                ) {
                    // Player hit!
                    self.lives -= 1;
                    // Always clear bullets and emitters when player is hit
                    self.bullets.clear();
                    self.bullet_emitter_indices.clear();
                    // Stop all emitters
                    for emitter in self.particle_system.emitters_mut() {
                        emitter.stop_emission();
                    }

                    if self.lives <= 0 {
                        self.game_over = true;
                        // Remove player when game over
                        self.player = None;
                    } else {
                        // Respawn player
                        let renderer = ctx.renderer();
                        self.spawn_player(renderer);
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Get input and time before borrowing renderer
        // Only check for thrusting if not game over and player exists
        let is_thrusting = if !self.game_over {
            if let Some(_) = self.player {
                let input = ctx.input();
                input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp)
            } else {
                false
            }
        } else {
            false
        };
        let time = ctx.elapsed_time().as_secs_f32();

        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        // Clear to black
        renderer.clear(&mut frame, [0.0, 0.0, 0.0, 1.0])?;

        // Draw player as triangle (only if not game over)
        if !self.game_over {
            if let Some(ref player) = self.player {
                // Draw flames behind ship when thrusting
                if is_thrusting {
                    let flame_points = self.get_flame_points(player, time);

                    // Calculate flame base position for inner flame scaling
                    let back_dir = Vec2::from_angle(player.rotation + std::f32::consts::PI);
                    let ship_back_offset = 20.0 * 0.5; // Half ship size
                    let flame_base_pos =
                        player.sprite.transform.position + back_dir * ship_back_offset;

                    // Draw outer flame (orange/red) - larger
                    renderer.draw_polygon(
                        &mut frame,
                        &flame_points,
                        [1.0, 0.4, 0.0, 1.0],
                        &self.camera,
                    )?;

                    // Draw middle flame (orange/yellow) - medium size
                    let middle_flame_points: Vec<Vec2> = flame_points
                        .iter()
                        .map(|&p| (p - flame_base_pos) * 0.75 + flame_base_pos)
                        .collect();
                    renderer.draw_polygon(
                        &mut frame,
                        &middle_flame_points,
                        [1.0, 0.7, 0.1, 1.0],
                        &self.camera,
                    )?;

                    // Draw inner flame (bright yellow/white) - smallest, hottest part
                    let inner_flame_points: Vec<Vec2> = flame_points
                        .iter()
                        .map(|&p| (p - flame_base_pos) * 0.5 + flame_base_pos)
                        .collect();
                    renderer.draw_polygon(
                        &mut frame,
                        &inner_flame_points,
                        [1.0, 1.0, 0.5, 1.0],
                        &self.camera,
                    )?;
                }

                // Draw ship on top of flames
                let ship_points = self.get_ship_points(player);
                renderer.draw_polygon(
                    &mut frame,
                    &ship_points,
                    [1.0, 1.0, 1.0, 1.0],
                    &self.camera,
                )?;
            }

            // Draw particle trails (before bullets so they appear behind)
            renderer.draw_particles(
                &mut frame,
                &self.particle_system,
                &self.camera,
                self.white_texture,
            )?;

            // Draw bullets as circles
            for bullet in &self.bullets {
                renderer.draw_circle(
                    &mut frame,
                    bullet.sprite.transform.position,
                    3.0,
                    [1.0, 1.0, 1.0, 1.0],
                    &self.camera,
                )?;
            }

            // Draw asteroids as polygons
            for asteroid in &self.asteroids {
                let asteroid_points = self.get_asteroid_points(asteroid);
                renderer.draw_polygon(
                    &mut frame,
                    &asteroid_points,
                    [0.8, 0.8, 0.8, 1.0],
                    &self.camera,
                )?;
            }
        }

        // Draw HUD
        if let Some(font) = self.font {
            self.hud.clear();

            // Score
            self.hud.add_text(HudText::new(
                format!("Score: {}", self.score),
                font,
                24.0,
                Vec2::new(20.0, 20.0),
                [1.0, 1.0, 1.0, 1.0],
            ));

            // Lives
            self.hud.add_text(HudText::new(
                format!("Lives: {}", self.lives),
                font,
                24.0,
                Vec2::new(20.0, 50.0),
                [1.0, 1.0, 1.0, 1.0],
            ));

            // Game over
            if self.game_over {
                let (screen_w, screen_h) = renderer.surface_size();
                self.hud.add_text(HudText::new(
                    "GAME OVER".to_string(),
                    font,
                    48.0,
                    Vec2::new(screen_w as f32 * 0.5 - 150.0, screen_h as f32 * 0.5 - 50.0),
                    [1.0, 0.0, 0.0, 1.0],
                ));
                self.hud.add_text(HudText::new(
                    "Press R to Restart".to_string(),
                    font,
                    24.0,
                    Vec2::new(screen_w as f32 * 0.5 - 120.0, screen_h as f32 * 0.5),
                    [1.0, 1.0, 1.0, 1.0],
                ));
            }
        }

        self.hud.draw(renderer, &mut frame)?;

        renderer.end_frame(frame)?;
        Ok(())
    }
}
