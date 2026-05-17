use std::time::Duration;

use anyhow::Result;
use sindri::{
    ActionId, AxisBinding, BuiltinFont, Button, Camera2D, Engine, EngineContext, FontHandle, Game,
    InputMap, KeyCode, MouseButton, Sprite, Vec2,
};

// Embedded texture: neutral white square (32x32). We tint per-sprite.
const RED_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20, 0x08, 0x06, 0x00, 0x00, 0x00, 0x73, 0x7a, 0x7a,
    0xf4, 0x00, 0x00, 0x00, 0x2f, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0xed, 0xce, 0x31, 0x01, 0x00,
    0x00, 0x08, 0xc3, 0xb0, 0x81, 0x7f, 0xcf, 0x43, 0x06, 0x4f, 0x6a, 0xa0, 0x99, 0xb6, 0xcd, 0x63,
    0xfb, 0x39, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x48, 0x92, 0x03, 0x4d, 0x88, 0x04, 0x3c, 0x4a, 0xbd, 0x9d, 0x15, 0x00, 0x00, 0x00, 0x00,
    0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

// Reuse same bytes for simplicity; actual color comes from tint.
const BLUE_PNG: &[u8] = RED_PNG;
const GREEN_PNG: &[u8] = RED_PNG;

struct Collectible {
    sprite: Sprite,
    rotation: f32,
    rotation_speed: f32,
}

struct BasicGame {
    // Player
    player: Option<Sprite>,
    player_speed: f32,

    // Camera
    camera: Camera2D,

    // World bounds and background
    world_bounds: Vec2,
    background_tiles: Vec<Sprite>,
    walls: Vec<Sprite>,

    // Collectibles
    collectibles: Vec<Collectible>,

    // Enemies (bouncing sprites)
    enemies: Vec<Sprite>,
    enemy_velocities: Vec<Vec2>,

    // Mouse click positions (for spawning)
    click_positions: Vec<Vec2>,

    // Game state
    score: u32,
    last_collectible_spawn: Duration,

    // Text rendering
    font: Option<FontHandle>,
    score_text: String,

    // Input mapping
    input_map: InputMap,
    axis_horizontal: ActionId,
    axis_vertical: ActionId,
}

impl Game for BasicGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Configure high-level input mapping (actions/axes).
        //
        // Movement is bound to WASD + arrow keys via two axes:
        // - move_horizontal: A/Left = -1, D/Right = +1
        // - move_vertical:   W/Up = -1 (up), S/Down = +1 (down)
        self.input_map = InputMap::new();
        self.axis_horizontal = ActionId::new("move_horizontal");
        self.axis_vertical = ActionId::new("move_vertical");

        self.input_map.set_axis(
            self.axis_horizontal.clone(),
            AxisBinding::new(
                vec![Button::Key(KeyCode::KeyA), Button::Key(KeyCode::ArrowLeft)],
                vec![Button::Key(KeyCode::KeyD), Button::Key(KeyCode::ArrowRight)],
            ),
        );

        self.input_map.set_axis(
            self.axis_vertical.clone(),
            AxisBinding::new(
                vec![Button::Key(KeyCode::KeyW), Button::Key(KeyCode::ArrowUp)],
                vec![Button::Key(KeyCode::KeyS), Button::Key(KeyCode::ArrowDown)],
            ),
        );

        // Load textures using AssetManager (demonstrates caching)
        let red_texture = ctx.load_texture_from_bytes("red_square", RED_PNG)?;
        let blue_texture = ctx.load_texture_from_bytes("blue_square", BLUE_PNG)?;
        let green_texture = ctx.load_texture_from_bytes("green_square", GREEN_PNG)?;

        // Try loading red again - should use cache!
        let _cached_red = ctx.load_texture_from_bytes("red_square", RED_PNG)?;
        assert_eq!(red_texture, _cached_red); // Same handle = cached!

        // World bounds - define the playable area
        self.world_bounds = Vec2::new(1400.0, 900.0);

        // Texture is 32x32 pixels
        const TEX_SIZE: f32 = 32.0;
        let tex_vec = Vec2::new(TEX_SIZE, TEX_SIZE);

        // Initialize player at a known world position (center-based transform)
        // Transform.position represents the CENTER of the sprite (vertices are centered)
        // Transform.scale is a multiplier: 1.0 = native texture size (32x32)
        let mut player = Sprite::new(blue_texture);
        player.set_size_px(Vec2::new(32.0, 32.0), tex_vec); // 32px = scale 1.0
        player.tint = [0.3, 0.5, 1.0, 1.0]; // Blue tint
        player.transform.position = Vec2::new(200.0, 200.0); // World coordinates

        // Initialize camera to center on player
        let (screen_w, screen_h) = ctx.renderer().surface_size();
        self.camera = Camera2D::new(Vec2::new(
            player.transform.position.x - (screen_w as f32 * 0.5),
            player.transform.position.y - (screen_h as f32 * 0.5),
        ));
        self.player = Some(player);

        // Background tiles (grid covering world bounds, centered on world origin)
        self.background_tiles.clear();
        let tile_size_px = Vec2::new(128.0, 128.0); // 128x128 pixel tiles
        let tile_half = tile_size_px * 0.5;
        // Cover world bounds with tiles
        for ix in 0..=(self.world_bounds.x / tile_size_px.x) as i32 {
            for iy in 0..=(self.world_bounds.y / tile_size_px.y) as i32 {
                let mut tile = Sprite::new(red_texture);
                tile.set_size_px(tile_size_px, tex_vec); // 128px = scale 4.0
                                                         // Position is center, so offset by half tile size
                tile.transform.position = Vec2::new(
                    ix as f32 * tile_size_px.x + tile_half.x,
                    iy as f32 * tile_size_px.y + tile_half.y,
                );
                tile.tint = [0.2, 0.22, 0.28, 1.0]; // darker grid
                self.background_tiles.push(tile);
            }
        }

        // World boundary walls (position is CENTER, so account for half-size)
        self.walls.clear();
        let wall_thickness_px = 32.0;
        let mut make_wall = |x: f32, y: f32, w_px: f32, h_px: f32| {
            let mut wall = Sprite::new(red_texture);
            wall.set_size_px(Vec2::new(w_px, h_px), tex_vec);
            // Position is center, so offset by half size
            wall.transform.position = Vec2::new(x + w_px * 0.5, y + h_px * 0.5);
            wall.tint = [0.2, 0.2, 0.2, 1.0];
            self.walls.push(wall);
        };
        // Top, bottom, left, right walls (inside world bounds)
        make_wall(0.0, 0.0, self.world_bounds.x, wall_thickness_px); // Top
        make_wall(
            0.0,
            self.world_bounds.y - wall_thickness_px,
            self.world_bounds.x,
            wall_thickness_px,
        ); // Bottom
        make_wall(0.0, 0.0, wall_thickness_px, self.world_bounds.y); // Left
        make_wall(
            self.world_bounds.x - wall_thickness_px,
            0.0,
            wall_thickness_px,
            self.world_bounds.y,
        ); // Right
           // Zoom adjusted for correct sprite sizes (1.0 = native size)
        self.camera.zoom = 1.0;

        // Spawn initial collectibles (green squares) in world coordinates
        let collectible_texture = green_texture;
        for i in 0..5 {
            let mut sprite = Sprite::new(collectible_texture);
            sprite.set_size_px(Vec2::new(48.0, 48.0), tex_vec); // 48px = scale 1.5
                                                                // Position in world space, not screen space
            sprite.transform.position =
                Vec2::new(300.0 + i as f32 * 150.0, 250.0 + i as f32 * 100.0);
            sprite.tint = [0.3, 1.0, 0.3, 1.0]; // Green tint

            self.collectibles.push(Collectible {
                sprite,
                rotation: i as f32 * 0.5,
                rotation_speed: 1.0 + i as f32 * 0.2,
            });
        }

        // Spawn initial enemies (red squares) in world coordinates
        let enemy_texture = red_texture;
        for i in 0..3 {
            let mut sprite = Sprite::new(enemy_texture);
            sprite.set_size_px(Vec2::new(56.0, 56.0), tex_vec); // 56px = scale 1.75
                                                                // Position in world space
            sprite.transform.position =
                Vec2::new(800.0 + i as f32 * 200.0, 400.0 + i as f32 * 150.0);
            sprite.tint = [1.0, 0.5, 0.5, 1.0]; // Slightly tinted

            self.enemies.push(sprite);
            self.enemy_velocities
                .push(Vec2::new(50.0 + i as f32 * 20.0, 40.0 + i as f32 * 15.0));
        }

        // Check audio availability
        if ctx.audio().is_available() {
            println!("Audio system is available!");
        } else {
            println!("Audio system is not available (this is okay)");
        }

        println!("=== Sindri Demo Game ===");
        println!("WASD/Arrow Keys: Move player");
        println!("Mouse Click: Spawn collectible at cursor");
        println!("ESC: Exit");
        println!("Collect green squares for points!");

        // Try to load a built-in font for text rendering.
        // Until you configure `BuiltinFont::Ui` in `sindri::fonts`, this will
        // likely return an error and text will be skipped.
        self.font = ctx.builtin_font(BuiltinFont::Ui).ok();
        self.score_text = format!("Score: {}", self.score);

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();

        // Exit on ESC
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }

        // Player movement using high-level input axes (WASD / arrow keys).
        let input = ctx.input();
        let move_dir = Vec2::new(
            self.input_map.axis(input, &self.axis_horizontal),
            self.input_map.axis(input, &self.axis_vertical),
        );

        // Normalize movement direction for consistent speed
        if let Some(player) = self.player.as_mut() {
            if move_dir.length_squared() > 0.0 {
                let dir = move_dir.normalized();
                player.transform.position += dir * self.player_speed * dt;
            }
            // Clamp player to world bounds (position is CENTER, so account for half-size)
            // Scale is a multiplier, so actual size = scale * texture_size (32px)
            const PLAYER_SIZE_PX: f32 = 32.0;
            let half_size = PLAYER_SIZE_PX * 0.5;
            player.transform.position.x = player
                .transform
                .position
                .x
                .clamp(half_size, self.world_bounds.x - half_size);
            player.transform.position.y = player
                .transform
                .position
                .y
                .clamp(half_size, self.world_bounds.y - half_size);
        }

        // Mouse click to spawn collectible at world position (camera-aware)
        if ctx.input().is_mouse_pressed(MouseButton::Left) {
            // Convert screen mouse position to world coordinates using camera
            let mouse_world = ctx.mouse_world(&self.camera);
            println!("CLICK world = {:?}", mouse_world);
            self.click_positions.push(mouse_world);

            // Spawn a new collectible at world position
            if let Some(green_texture) = ctx.assets().get_texture("green_square") {
                let mut sprite = Sprite::new(green_texture);
                sprite.set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0)); // 32px = scale 1.0
                sprite.transform.position = mouse_world; // World coordinates
                sprite.tint = [0.3, 1.0, 0.3, 1.0]; // Green tint

                self.collectibles.push(Collectible {
                    sprite,
                    rotation: 0.0,
                    rotation_speed: 2.0,
                });
            }
        }

        // Update camera to follow player (smoothly), keeping player near center
        if let Some(player) = &self.player {
            let (screen_w, screen_h) = ctx.renderer().surface_size();
            let target_pos = Vec2::new(
                player.transform.position.x - (screen_w as f32 * 0.5),
                player.transform.position.y - (screen_h as f32 * 0.5),
            );
            let camera_speed = 5.0;
            self.camera.position = self.camera.position.lerp(target_pos, camera_speed * dt);
        }

        // Update collectibles (rotation animation)
        for collectible in &mut self.collectibles {
            collectible.rotation += collectible.rotation_speed * dt;
            collectible.sprite.transform.rotation = collectible.rotation;
        }

        // Check collisions: player vs collectibles (compute radii from actual sprite sizes)
        // Texture is 32x32, so size_px = scale * 32.0
        const TEX_PX: f32 = 32.0;

        let Some(player) = &self.player else {
            return Ok(());
        };
        let player_pos = player.transform.position;
        let player_size_px = player.transform.scale * TEX_PX;
        let player_radius = player_size_px.length() * 0.5;

        self.collectibles.retain_mut(|collectible| {
            let collectible_pos = collectible.sprite.transform.position;
            let collectible_size_px = collectible.sprite.transform.scale * TEX_PX;
            let collectible_radius = collectible_size_px.length() * 0.5;

            let distance = player_pos.distance(collectible_pos);
            if distance < player_radius + collectible_radius {
                self.score += 10;
                self.score_text = format!("Score: {}", self.score);
                println!("Score: {} (+10)", self.score);
                false // Remove collectible
            } else {
                true // Keep collectible
            }
        });

        // Update enemies (bouncing movement) - position is CENTER
        let bounds = self.world_bounds;
        const ENEMY_SIZE_PX: f32 = 56.0;
        let enemy_half_size = ENEMY_SIZE_PX * 0.5;

        for (enemy, velocity) in self
            .enemies
            .iter_mut()
            .zip(self.enemy_velocities.iter_mut())
        {
            enemy.transform.position += *velocity * dt;

            let pos = &mut enemy.transform.position;

            // Bounce off walls (accounting for center-based position)
            if pos.x < enemy_half_size || pos.x > bounds.x - enemy_half_size {
                velocity.x = -velocity.x;
                pos.x = pos.x.max(enemy_half_size).min(bounds.x - enemy_half_size);
            }
            if pos.y < enemy_half_size || pos.y > bounds.y - enemy_half_size {
                velocity.y = -velocity.y;
                pos.y = pos.y.max(enemy_half_size).min(bounds.y - enemy_half_size);
            }
        }

        // Spawn new collectibles periodically in world coordinates
        if ctx.elapsed_time() - self.last_collectible_spawn > Duration::from_secs(3) {
            self.last_collectible_spawn = ctx.elapsed_time();

            if let Some(green_texture) = ctx.assets().get_texture("green_square") {
                let mut sprite = Sprite::new(green_texture);
                const COLLECTIBLE_SIZE_PX: f32 = 48.0;
                sprite.set_size_px(
                    Vec2::new(COLLECTIBLE_SIZE_PX, COLLECTIBLE_SIZE_PX),
                    Vec2::new(32.0, 32.0),
                );
                // Spawn in world space, avoiding edges (using actual pixel size)
                let half_size = COLLECTIBLE_SIZE_PX * 0.5;
                sprite.transform.position = Vec2::new(
                    half_size + (self.score as f32 * 50.0) % (bounds.x - COLLECTIBLE_SIZE_PX),
                    half_size + (self.score as f32 * 70.0) % (bounds.y - COLLECTIBLE_SIZE_PX),
                );
                sprite.tint = [0.3, 1.0, 0.3, 1.0]; // Green tint

                self.collectibles.push(Collectible {
                    sprite,
                    rotation: 0.0,
                    rotation_speed: 1.5,
                });
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        // Clear with a nice dark blue background
        renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;

        // Draw background tiles
        for tile in &self.background_tiles {
            renderer.draw_sprite(&mut frame, tile, &self.camera)?;
        }

        // Draw walls (world bounds)
        for wall in &self.walls {
            renderer.draw_sprite(&mut frame, wall, &self.camera)?;
        }

        // Draw collectibles
        for collectible in &self.collectibles {
            renderer.draw_sprite(&mut frame, &collectible.sprite, &self.camera)?;
        }

        // Draw enemies
        for enemy in &self.enemies {
            renderer.draw_sprite(&mut frame, enemy, &self.camera)?;
        }

        // Draw player (on top)
        if let Some(player) = &self.player {
            renderer.draw_sprite(&mut frame, player, &self.camera)?;
        }

        // Draw text (if font is loaded)
        if let Some(font) = self.font {
            // Update score text if needed
            let current_score_text = format!("Score: {}", self.score);
            if current_score_text != self.score_text {
                self.score_text = current_score_text.clone();
                // Re-rasterize if text changed
                if let Err(e) = renderer.rasterize_text_glyphs(&self.score_text, font, 24.0) {
                    eprintln!("Failed to rasterize text: {}", e);
                }
            }

            // Draw score in top-left corner (screen space, but we'll use world coords)
            // Position relative to camera for screen-space effect
            let (screen_w, screen_h) = renderer.surface_size();
            let text_pos = Vec2::new(
                self.camera.position.x - (screen_w as f32 * 0.5) + 20.0,
                self.camera.position.y + (screen_h as f32 * 0.5) - 40.0,
            );

            if let Err(e) = renderer.draw_text(
                &mut frame,
                &self.score_text,
                font,
                24.0,
                text_pos,
                [1.0, 1.0, 1.0, 1.0], // White
                &self.camera,
            ) {
                eprintln!("Failed to draw text: {}", e);
            }

            // Draw instructions (bottom-left)
            let instructions = "WASD: Move | Click: Spawn | ESC: Exit";
            let instructions_pos = Vec2::new(
                self.camera.position.x - (screen_w as f32 * 0.5) + 20.0,
                self.camera.position.y - (screen_h as f32 * 0.5) + 20.0,
            );

            // Rasterize instructions text
            if let Err(e) = renderer.rasterize_text_glyphs(instructions, font, 16.0) {
                eprintln!("Failed to rasterize instructions: {}", e);
            } else if let Err(e) = renderer.draw_text(
                &mut frame,
                instructions,
                font,
                16.0,
                instructions_pos,
                [0.8, 0.8, 0.8, 1.0], // Light gray
                &self.camera,
            ) {
                eprintln!("Failed to draw instructions: {}", e);
            }
        }

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri Demo - Collect the Green Squares!")
        .with_size(1024, 768)
        .with_vsync(true)
        .run(BasicGame {
            player: None, // Will be initialized in init()
            player_speed: 200.0,
            camera: Camera2D::default(),
            world_bounds: Vec2::ZERO,
            background_tiles: Vec::new(),
            walls: Vec::new(),
            collectibles: Vec::new(),
            enemies: Vec::new(),
            enemy_velocities: Vec::new(),
            click_positions: Vec::new(),
            score: 0,
            last_collectible_spawn: Duration::ZERO,
            font: None,
            score_text: String::new(),
            input_map: InputMap::new(),
            axis_horizontal: ActionId::new("move_horizontal"),
            axis_vertical: ActionId::new("move_vertical"),
        })
}
