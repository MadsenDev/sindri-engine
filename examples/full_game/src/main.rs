use std::time::Duration;

use anyhow::Result;
use sindri::{
    ActionId, AxisBinding, BuiltinFont, Button, Camera2D, Engine, EngineContext, FontHandle,
    HudLayer, HudRect, HudText, InputMap, KeyCode, MouseButton, Sprite, State, StateMachine,
    StateMachineLike, TextAlign, Vec2,
};

// Optional embedded font: if you have a TTF/OTF file, you can include it here.
// For now this example runs without text if the user doesn't provide one.
// const FONT_BYTES: &[u8] = include_bytes!("../../assets/DejaVuSans.ttf");

struct Collectible {
    sprite: Sprite,
    rotation: f32,
    rotation_speed: f32,
}

/// Main menu state.
struct MenuState {
    time: f32,
    selected_index: usize,
    font_title: Option<FontHandle>,
    font_ui: Option<FontHandle>,
    hud: HudLayer,
    menu_items: Vec<&'static str>,
}

impl MenuState {
    fn new() -> Self {
        Self {
            time: 0.0,
            selected_index: 0,
            font_title: None,
            font_ui: None,
            hud: HudLayer::new(),
            menu_items: vec!["Start Game", "Exit"],
        }
    }
}

impl State for MenuState {
    fn on_enter(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Load fonts for the menu
        self.font_title = ctx.builtin_font(BuiltinFont::Title).ok();
        self.font_ui = ctx.builtin_font(BuiltinFont::Ui).ok();
        self.selected_index = 0;
        self.time = 0.0;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        self.time += dt;

        // Check input without holding a borrow
        let up_pressed = ctx.input().is_key_pressed(KeyCode::ArrowUp)
            || ctx.input().is_key_pressed(KeyCode::KeyW);
        let down_pressed = ctx.input().is_key_pressed(KeyCode::ArrowDown)
            || ctx.input().is_key_pressed(KeyCode::KeyS);
        let select_pressed = ctx.input().is_key_pressed(KeyCode::Enter)
            || ctx.input().is_key_pressed(KeyCode::Space);
        let escape_pressed = ctx.input().is_key_pressed(KeyCode::Escape);

        // Navigate menu with arrow keys or WASD
        if up_pressed {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.menu_items.len() - 1;
            }
        }

        if down_pressed {
            self.selected_index = (self.selected_index + 1) % self.menu_items.len();
        }

        // Select menu item
        if select_pressed {
            match self.selected_index {
                0 => {
                    // Start Game - replace menu with gameplay
                    let font = ctx.builtin_font(BuiltinFont::Ui).ok();
                    sm.replace(Box::new(GameplayState::new(font)));
                }
                1 => {
                    // Exit
                    ctx.request_exit();
                }
                _ => {}
            }
        }

        // ESC always exits
        if escape_pressed {
            ctx.request_exit();
        }

        Ok(())
    }

    fn draw(&mut self, renderer: &mut sindri::Renderer, frame: &mut sindri::Frame) -> Result<()> {
        // Gradient background (darker blue)
        renderer.clear(frame, [0.05, 0.05, 0.15, 1.0])?;

        let (screen_w, screen_h) = renderer.surface_size();
        let center_x = screen_w as f32 * 0.5;
        let center_y = screen_h as f32 * 0.5;

        self.hud.clear();

        // Draw title
        if let Some(font_title) = self.font_title {
            let title_text = "SINDRI";
            let title_size = 64.0;
            let title_y = center_y - 150.0;
            // Approximate text width for centering: "SINDRI" at 64px is roughly 350px wide
            let title_width_approx = 350.0;
            let title_x = center_x - (title_width_approx * 0.5);

            // Title shadow (offset slightly)
            self.hud.add_text(HudText {
                text: title_text.to_string(),
                font: font_title,
                size: title_size,
                position: Vec2::new(title_x + 3.0, title_y + 3.0),
                color: [0.0, 0.0, 0.0, 0.5],
                align: TextAlign::Left,
            });

            // Title main
            self.hud.add_text(HudText {
                text: title_text.to_string(),
                font: font_title,
                size: title_size,
                position: Vec2::new(title_x, title_y),
                color: [0.9, 0.7, 0.2, 1.0], // Gold color
                align: TextAlign::Left,
            });
        }

        // Draw menu items
        if let Some(font_ui) = self.font_ui {
            let menu_start_y = center_y + 50.0;
            let menu_spacing = 60.0;
            let menu_size = 32.0;
            // Approximate text width for menu items: "Start Game" at 32px is roughly 180px wide
            let menu_item_width_approx = 180.0;
            let menu_x = center_x - (menu_item_width_approx * 0.5);

            for (i, item) in self.menu_items.iter().enumerate() {
                let y = menu_start_y + (i as f32 * menu_spacing);
                let is_selected = i == self.selected_index;

                // Selection indicator (pulsing effect)
                if is_selected {
                    let pulse = (self.time * 3.0).sin() * 0.3 + 0.7;

                    // Arrow indicator
                    self.hud.add_text(HudText {
                        text: ">".to_string(),
                        font: font_ui,
                        size: menu_size * pulse,
                        position: Vec2::new(menu_x - 30.0, y),
                        color: [1.0, 0.8, 0.2, pulse],
                        align: TextAlign::Left,
                    });
                }

                // Menu item text
                let text_color = if is_selected {
                    [1.0, 1.0, 1.0, 1.0] // Bright white when selected
                } else {
                    [0.7, 0.7, 0.7, 1.0] // Gray when not selected
                };

                self.hud.add_text(HudText {
                    text: item.to_string(),
                    font: font_ui,
                    size: menu_size,
                    position: Vec2::new(menu_x, y),
                    color: text_color,
                    align: TextAlign::Left,
                });
            }

            // Instructions at bottom
            let instructions = "Arrow Keys/WASD: Navigate | ENTER/Space: Select | ESC: Exit";
            let instructions_size = 16.0;
            // Approximate width for instructions text
            let instructions_width_approx = 600.0;
            let instructions_x = center_x - (instructions_width_approx * 0.5);
            let instructions_y = screen_h as f32 - 40.0;
            self.hud.add_text(HudText {
                text: instructions.to_string(),
                font: font_ui,
                size: instructions_size,
                position: Vec2::new(instructions_x, instructions_y),
                color: [0.5, 0.5, 0.5, 1.0],
                align: TextAlign::Left,
            });
        }

        // Draw HUD
        self.hud.draw(renderer, frame)?;

        Ok(())
    }
}

/// Gameplay state – mostly reuses logic from `basic_game`.
struct GameplayState {
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

    // HUD layer (screen-space UI)
    hud: HudLayer,
}

impl GameplayState {
    fn new(font: Option<FontHandle>) -> Self {
        let mut input_map = InputMap::new();
        let axis_horizontal = ActionId::new("move_horizontal");
        let axis_vertical = ActionId::new("move_vertical");

        input_map.set_axis(
            axis_horizontal.clone(),
            AxisBinding::new(
                vec![Button::Key(KeyCode::KeyA), Button::Key(KeyCode::ArrowLeft)],
                vec![Button::Key(KeyCode::KeyD), Button::Key(KeyCode::ArrowRight)],
            ),
        );

        input_map.set_axis(
            axis_vertical.clone(),
            AxisBinding::new(
                vec![Button::Key(KeyCode::KeyW), Button::Key(KeyCode::ArrowUp)],
                vec![Button::Key(KeyCode::KeyS), Button::Key(KeyCode::ArrowDown)],
            ),
        );

        Self {
            player: None,
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
            font,
            score_text: String::new(),
            input_map,
            axis_horizontal,
            axis_vertical,
            hud: HudLayer::new(),
        }
    }
}

impl State for GameplayState {
    fn on_enter(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Create simple solid-color 32x32 textures via raw RGBA data.
        let renderer = ctx.renderer();

        fn solid_rgba(r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
            let mut data = Vec::with_capacity(32 * 32 * 4);
            for _ in 0..(32 * 32) {
                data.push(r);
                data.push(g);
                data.push(b);
                data.push(a);
            }
            data
        }

        let red_tex = solid_rgba(255, 64, 64, 255);
        let blue_tex = solid_rgba(64, 128, 255, 255);
        let green_tex = solid_rgba(64, 255, 96, 255);

        let red_texture = renderer.load_texture_from_rgba(&red_tex, 32, 32)?;
        let blue_texture = renderer.load_texture_from_rgba(&blue_tex, 32, 32)?;
        let green_texture = renderer.load_texture_from_rgba(&green_tex, 32, 32)?;

        // World bounds.
        self.world_bounds = Vec2::new(1400.0, 900.0);

        const TEX_SIZE: f32 = 32.0;
        let tex_vec = Vec2::new(TEX_SIZE, TEX_SIZE);

        // Player.
        let mut player = Sprite::new(blue_texture);
        player.set_size_px(Vec2::new(32.0, 32.0), tex_vec);
        player.tint = [0.3, 0.5, 1.0, 1.0];
        player.transform.position = Vec2::new(200.0, 200.0);

        let (screen_w, screen_h) = ctx.renderer().surface_size();
        self.camera = Camera2D::new(Vec2::new(
            player.transform.position.x - (screen_w as f32 * 0.5),
            player.transform.position.y - (screen_h as f32 * 0.5),
        ));
        self.player = Some(player);

        // Background grid.
        self.background_tiles.clear();
        let tile_size_px = Vec2::new(128.0, 128.0);
        let tile_half = tile_size_px * 0.5;
        for ix in 0..=(self.world_bounds.x / tile_size_px.x) as i32 {
            for iy in 0..=(self.world_bounds.y / tile_size_px.y) as i32 {
                let mut tile = Sprite::new(red_texture);
                tile.set_size_px(tile_size_px, tex_vec);
                tile.transform.position = Vec2::new(
                    ix as f32 * tile_size_px.x + tile_half.x,
                    iy as f32 * tile_size_px.y + tile_half.y,
                );
                tile.tint = [0.2, 0.22, 0.28, 1.0];
                self.background_tiles.push(tile);
            }
        }

        // World bounds.
        self.walls.clear();
        let wall_thickness_px = 32.0;
        let mut make_wall = |x: f32, y: f32, w_px: f32, h_px: f32| {
            let mut wall = Sprite::new(red_texture);
            wall.set_size_px(Vec2::new(w_px, h_px), tex_vec);
            wall.transform.position = Vec2::new(x + w_px * 0.5, y + h_px * 0.5);
            wall.tint = [0.2, 0.2, 0.2, 1.0];
            self.walls.push(wall);
        };

        make_wall(0.0, 0.0, self.world_bounds.x, wall_thickness_px);
        make_wall(
            0.0,
            self.world_bounds.y - wall_thickness_px,
            self.world_bounds.x,
            wall_thickness_px,
        );
        make_wall(0.0, 0.0, wall_thickness_px, self.world_bounds.y);
        make_wall(
            self.world_bounds.x - wall_thickness_px,
            0.0,
            wall_thickness_px,
            self.world_bounds.y,
        );

        self.camera.zoom = 1.0;

        // Initial collectibles.
        self.collectibles.clear();
        let collectible_texture = green_texture;
        for i in 0..5 {
            let mut sprite = Sprite::new(collectible_texture);
            sprite.set_size_px(Vec2::new(48.0, 48.0), tex_vec);
            sprite.transform.position =
                Vec2::new(300.0 + i as f32 * 150.0, 250.0 + i as f32 * 100.0);
            sprite.tint = [0.3, 1.0, 0.3, 1.0];

            self.collectibles.push(Collectible {
                sprite,
                rotation: i as f32 * 0.5,
                rotation_speed: 1.0 + i as f32 * 0.2,
            });
        }

        // Enemies.
        self.enemies.clear();
        self.enemy_velocities.clear();
        let enemy_texture = red_texture;
        for i in 0..3 {
            let mut sprite = Sprite::new(enemy_texture);
            sprite.set_size_px(Vec2::new(56.0, 56.0), tex_vec);
            sprite.transform.position =
                Vec2::new(800.0 + i as f32 * 200.0, 400.0 + i as f32 * 150.0);
            sprite.tint = [1.0, 0.5, 0.5, 1.0];

            self.enemies.push(sprite);
            self.enemy_velocities
                .push(Vec2::new(50.0 + i as f32 * 20.0, 40.0 + i as f32 * 15.0));
        }

        self.score = 0;
        self.score_text = format!("Score: {}", self.score);
        self.last_collectible_spawn = Duration::ZERO;

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();

        // Exit to menu.
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            // Replace gameplay with menu (since we replaced menu with gameplay)
            sm.replace(Box::new(MenuState::new()));
            return Ok(());
        }

        // Pause.
        if ctx.input().is_key_pressed(KeyCode::KeyP) {
            sm.push(Box::new(PauseState::new()));
            return Ok(());
        }

        // Movement via axes.
        let input = ctx.input();
        let move_dir = Vec2::new(
            self.input_map.axis(input, &self.axis_horizontal),
            self.input_map.axis(input, &self.axis_vertical),
        );

        if let Some(player) = self.player.as_mut() {
            if move_dir.length_squared() > 0.0 {
                let dir = move_dir.normalized();
                player.transform.position += dir * self.player_speed * dt;
            }

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

        // Mouse click to spawn collectibles at world position.
        if ctx.input().is_mouse_pressed(MouseButton::Left) {
            let mouse_world = ctx.mouse_world(&self.camera);
            self.click_positions.push(mouse_world);

            if let Some(first_collectible) = self.collectibles.first() {
                let texture = first_collectible.sprite.texture;
                let mut sprite = Sprite::new(texture);
                sprite.set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
                sprite.transform.position = mouse_world;
                sprite.tint = [0.3, 1.0, 0.3, 1.0];

                self.collectibles.push(Collectible {
                    sprite,
                    rotation: 0.0,
                    rotation_speed: 1.5,
                });
            }
        }

        // Rotate collectibles and handle simple collision with player.
        if let Some(player) = self.player.as_ref() {
            let player_pos = player.transform.position;
            let player_radius = 16.0;

            self.collectibles.iter_mut().for_each(|c| {
                c.rotation += c.rotation_speed * dt;
                c.sprite.transform.rotation = c.rotation;
            });

            // Count collectibles before collision check
            let collectibles_before = self.collectibles.len();

            // Remove collectibles that collide with player
            self.collectibles
                .retain(|c| (c.sprite.transform.position - player_pos).length() > player_radius);

            // Count collectibles after collision check
            let collectibles_after = self.collectibles.len();
            let collected_count = collectibles_before - collectibles_after;

            // Update score when collectibles are collected
            if collected_count > 0 {
                self.score += collected_count as u32 * 10; // 10 points per collectible
                self.score_text = format!("Score: {}", self.score);
            }
        }

        // Update enemies (bounce within world bounds).
        for (enemy, vel) in self
            .enemies
            .iter_mut()
            .zip(self.enemy_velocities.iter_mut())
        {
            enemy.transform.position += *vel * dt;

            let half_size = 28.0;
            let pos = &mut enemy.transform.position;

            if pos.x - half_size < 0.0 || pos.x + half_size > self.world_bounds.x {
                vel.x = -vel.x;
            }
            if pos.y - half_size < 0.0 || pos.y + half_size > self.world_bounds.y {
                vel.y = -vel.y;
            }
        }

        // Camera follow.
        if let Some(player) = self.player.as_ref() {
            let (screen_w, screen_h) = ctx.renderer().surface_size();
            let target = Vec2::new(
                player.transform.position.x - (screen_w as f32 * 0.5),
                player.transform.position.y - (screen_h as f32 * 0.5),
            );
            let camera_speed = 5.0;
            self.camera.position = self.camera.position.lerp(target, camera_speed * dt);
        }

        Ok(())
    }

    fn draw(&mut self, renderer: &mut sindri::Renderer, frame: &mut sindri::Frame) -> Result<()> {
        renderer.clear(frame, [0.1, 0.1, 0.15, 1.0])?;

        // Background.
        for tile in &self.background_tiles {
            renderer.draw_sprite(frame, tile, &self.camera)?;
        }

        // Walls.
        for wall in &self.walls {
            renderer.draw_sprite(frame, wall, &self.camera)?;
        }

        // Collectibles.
        for collectible in &self.collectibles {
            renderer.draw_sprite(frame, &collectible.sprite, &self.camera)?;
        }

        // Enemies.
        for enemy in &self.enemies {
            renderer.draw_sprite(frame, enemy, &self.camera)?;
        }

        // Player.
        if let Some(player) = &self.player {
            renderer.draw_sprite(frame, player, &self.camera)?;
        }

        // HUD: score, instructions, and a simple health bar (as a demo).
        self.hud.clear();
        if let Some(font) = self.font {
            let (screen_w, _screen_h) = renderer.surface_size();

            // Score in the top-left corner.
            self.hud.add_text(HudText {
                text: self.score_text.clone(),
                font,
                size: 24.0,
                position: Vec2::new(20.0, 32.0),
                color: [1.0, 1.0, 1.0, 1.0],
                align: TextAlign::Left,
            });

            // Instructions at the bottom-left.
            self.hud.add_text(HudText {
                text: "WASD/Arrows: Move | Mouse: Spawn | P: Pause | ESC: Menu".to_string(),
                font,
                size: 16.0,
                position: Vec2::new(20.0, 20.0 + 32.0 + 24.0),
                color: [0.8, 0.8, 0.8, 1.0],
                align: TextAlign::Left,
            });

            // Example: simple health bar (fake value here).
            let health_frac = 0.75f32; // pretend health is 75%
            let bar_width = 200.0;
            let bar_height = 16.0;
            let x = screen_w as f32 - bar_width - 40.0;
            let y = 32.0;

            // Background bar (dark).
            self.hud.add_rect(HudRect {
                position: Vec2::new(x, y),
                size: Vec2::new(bar_width, bar_height),
                color: [0.1, 0.1, 0.1, 0.8],
            });

            // Foreground bar (green).
            self.hud.add_rect(HudRect {
                position: Vec2::new(x, y),
                size: Vec2::new(bar_width * health_frac, bar_height),
                color: [0.2, 0.8, 0.2, 0.9],
            });
        }

        // Draw HUD on top.
        self.hud.draw(renderer, frame)?;

        Ok(())
    }
}

/// Pause state overlays the gameplay.
struct PauseState {
    font_title: Option<FontHandle>,
    font_ui: Option<FontHandle>,
    hud: HudLayer,
}

impl PauseState {
    fn new() -> Self {
        Self {
            font_title: None,
            font_ui: None,
            hud: HudLayer::new(),
        }
    }
}

impl State for PauseState {
    fn on_enter(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Load fonts for the pause menu
        self.font_title = ctx.builtin_font(BuiltinFont::Title).ok();
        self.font_ui = ctx.builtin_font(BuiltinFont::Ui).ok();
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext, sm: &mut dyn StateMachineLike) -> Result<()> {
        let p_pressed = ctx.input().is_key_pressed(KeyCode::KeyP);
        let escape_pressed = ctx.input().is_key_pressed(KeyCode::Escape);

        if p_pressed {
            sm.pop(); // Pop pause -> back to gameplay
        }

        if escape_pressed {
            // Pop pause, then replace gameplay with menu
            sm.pop(); // pause
            sm.replace(Box::new(MenuState::new())); // Replace gameplay with menu
        }

        Ok(())
    }

    fn draw(&mut self, renderer: &mut sindri::Renderer, frame: &mut sindri::Frame) -> Result<()> {
        // Semi-transparent dark overlay (gameplay is still visible underneath)
        renderer.clear(frame, [0.0, 0.0, 0.0, 0.6])?;

        let (screen_w, screen_h) = renderer.surface_size();
        let center_x = screen_w as f32 * 0.5;
        let center_y = screen_h as f32 * 0.5;

        self.hud.clear();

        // Draw "PAUSED" title
        if let Some(font_title) = self.font_title {
            let title_text = "PAUSED";
            let title_size = 72.0;
            let title_y = center_y - 100.0;
            let title_width_approx = 400.0;
            let title_x = center_x - (title_width_approx * 0.5);

            // Title shadow
            self.hud.add_text(HudText {
                text: title_text.to_string(),
                font: font_title,
                size: title_size,
                position: Vec2::new(title_x + 4.0, title_y + 4.0),
                color: [0.0, 0.0, 0.0, 0.7],
                align: TextAlign::Left,
            });

            // Title main
            self.hud.add_text(HudText {
                text: title_text.to_string(),
                font: font_title,
                size: title_size,
                position: Vec2::new(title_x, title_y),
                color: [1.0, 0.9, 0.3, 1.0], // Bright yellow
                align: TextAlign::Left,
            });
        }

        // Draw instructions
        if let Some(font_ui) = self.font_ui {
            let instructions = vec!["P: Resume", "ESC: Back to Menu"];
            let instruction_size = 24.0;
            let instruction_spacing = 40.0;
            let instruction_start_y = center_y + 50.0;
            let instruction_width_approx = 200.0;
            let instruction_x = center_x - (instruction_width_approx * 0.5);

            for (i, instruction) in instructions.iter().enumerate() {
                let y = instruction_start_y + (i as f32 * instruction_spacing);
                self.hud.add_text(HudText {
                    text: instruction.to_string(),
                    font: font_ui,
                    size: instruction_size,
                    position: Vec2::new(instruction_x, y),
                    color: [0.9, 0.9, 0.9, 1.0],
                    align: TextAlign::Left,
                });
            }
        }

        // Draw HUD
        self.hud.draw(renderer, frame)?;

        Ok(())
    }
}

fn main() -> Result<()> {
    let state_machine = StateMachine::with_initial_state(Box::new(MenuState::new()));

    Engine::new()
        .with_title("Sindri Full Game Demo")
        .with_size(1024, 768)
        .with_vsync(true)
        .run(state_machine)
}
