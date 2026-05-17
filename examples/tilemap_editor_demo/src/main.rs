use anyhow::Result;
use sindri::{
    entities::{TilemapComponent, Transform},
    hud::{HudLayer, HudRect, HudSprite, HudText},
    math::{Camera2D, Transform2D, Vec2},
    render::{Renderer, Sprite, TextureHandle, Tilemap},
    Engine, EngineContext, Game, World,
};

struct TilemapEditor {
    world: World,
    camera: Camera2D,

    tileset: Option<TextureHandle>,
    tilemap_entity: Option<sindri::EntityId>,

    // Editor state
    selected_tile_id: u32,
    brush_size: u32, // 1 = single tile, 2 = 2x2, etc.
    is_painting: bool,
    last_paint_pos: Option<(u32, u32)>,

    // UI
    hud: HudLayer,
    font: Option<sindri::FontHandle>,

    // Map dimensions
    map_width: u32,
    map_height: u32,

    // Tile selector panel
    selector_panel_x: f32,
    selector_panel_width: f32,
    selector_tiles_per_row: u32,
    selector_scale: f32, // Scale for tiles in selector (smaller than map tiles)
}

impl TilemapEditor {
    fn new() -> Result<Self> {
        // Center camera on the map (map is 50x30 tiles, 32px each = 1600x960)
        let map_center = Vec2::new(50.0 * 32.0 / 2.0, 30.0 * 32.0 / 2.0);

        Ok(Self {
            world: World::new(),
            camera: Camera2D::new(map_center),
            tileset: None,
            tilemap_entity: None,
            selected_tile_id: 1,
            brush_size: 1,
            is_painting: false,
            last_paint_pos: None,
            hud: HudLayer::new(),
            font: None,
            map_width: 50,
            map_height: 30,
            selector_panel_x: 0.0,
            selector_panel_width: 200.0,
            selector_tiles_per_row: 10,
            selector_scale: 0.5, // Tiles shown at half size in selector
        })
    }

    fn load_tileset(&mut self, renderer: &mut Renderer) -> Result<()> {
        let tileset_path = format!(
            "{}/assets/hyptosis_tile-art-batch-1.png",
            env!("CARGO_MANIFEST_DIR")
        );
        self.tileset = Some(renderer.load_texture_from_file(&tileset_path)?);
        Ok(())
    }

    fn create_tilemap(&mut self) -> Result<()> {
        if let Some(tileset) = self.tileset {
            let tile_size = Vec2::new(32.0, 32.0);
            let tileset_cols = 30;
            let tileset_rows = 30;

            let mut tilemap = Tilemap::new(
                tileset,
                (tileset_cols, tileset_rows),
                tile_size,
                (self.map_width, self.map_height),
                Vec2::ZERO,
            );

            // Start with a test pattern so user can see something
            // Fill with floor tiles (tile ID 1) so there's something visible
            tilemap.fill_rect(0, 0, self.map_width, self.map_height, 1);

            let entity = self.world.spawn();
            self.world.insert(entity, TilemapComponent::new(tilemap));
            self.tilemap_entity = Some(entity);
        }

        Ok(())
    }

    fn paint_tile(&mut self, x: u32, y: u32) {
        if let Some(entity) = self.tilemap_entity {
            if let Some(tilemap_comp) = self.world.get_mut::<TilemapComponent>(entity) {
                if self.brush_size == 1 {
                    // Single tile - simple case
                    if x < self.map_width && y < self.map_height {
                        tilemap_comp.tilemap.set_tile(x, y, self.selected_tile_id);
                    }
                } else {
                    // Multi-tile brush - center it on the click position
                    let half_brush = (self.brush_size as i32 - 1) / 2;
                    let start_x = (x as i32 - half_brush).max(0) as u32;
                    let start_y = (y as i32 - half_brush).max(0) as u32;

                    for dy in 0..self.brush_size {
                        for dx in 0..self.brush_size {
                            let tx = start_x + dx;
                            let ty = start_y + dy;
                            if tx < self.map_width && ty < self.map_height {
                                tilemap_comp.tilemap.set_tile(tx, ty, self.selected_tile_id);
                            }
                        }
                    }
                }
            }
        }
    }

    fn erase_tile(&mut self, x: u32, y: u32) {
        if let Some(entity) = self.tilemap_entity {
            if let Some(tilemap_comp) = self.world.get_mut::<TilemapComponent>(entity) {
                if self.brush_size == 1 {
                    // Single tile
                    if x < self.map_width && y < self.map_height {
                        tilemap_comp.tilemap.set_tile(x, y, 0); // 0 = empty
                    }
                } else {
                    // Multi-tile brush
                    let half_brush = (self.brush_size as i32 - 1) / 2;
                    let start_x = (x as i32 - half_brush).max(0) as u32;
                    let start_y = (y as i32 - half_brush).max(0) as u32;

                    for dy in 0..self.brush_size {
                        for dx in 0..self.brush_size {
                            let tx = start_x + dx;
                            let ty = start_y + dy;
                            if tx < self.map_width && ty < self.map_height {
                                tilemap_comp.tilemap.set_tile(tx, ty, 0); // 0 = empty
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Game for TilemapEditor {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.load_tileset(ctx.renderer())?;
        self.create_tilemap()?;
        self.font = Some(ctx.builtin_font(sindri::BuiltinFont::Ui)?);

        // Set initial zoom to show a good portion of the map
        // Reserve space for tile selector on the right
        let (screen_w, screen_h) = ctx.renderer().surface_size();
        self.selector_panel_x = screen_w as f32 - self.selector_panel_width;

        // Zoom to fit map height in available space (minus selector panel)
        let available_width = screen_w as f32 - self.selector_panel_width;
        let map_height_px = self.map_height as f32 * 32.0;
        let map_width_px = self.map_width as f32 * 32.0;

        // Zoom to fit either width or height, whichever is more restrictive
        let zoom_for_height = (screen_h as f32 / map_height_px) * 0.9;
        let zoom_for_width = (available_width / map_width_px) * 0.9;
        let initial_zoom = zoom_for_height.min(zoom_for_width).min(1.0);
        self.camera.zoom = initial_zoom;

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();

        // Get screen size first
        let (screen_w, screen_h) = ctx.renderer().surface_size();

        // Then get input
        let input = ctx.input();

        // Camera panning with arrow keys or WASD
        let pan_speed = 300.0;
        let mut pan = Vec2::ZERO;
        if input.is_key_down(sindri::KeyCode::ArrowLeft) || input.is_key_down(sindri::KeyCode::KeyA)
        {
            pan.x -= pan_speed * dt;
        }
        if input.is_key_down(sindri::KeyCode::ArrowRight)
            || input.is_key_down(sindri::KeyCode::KeyD)
        {
            pan.x += pan_speed * dt;
        }
        if input.is_key_down(sindri::KeyCode::ArrowUp) || input.is_key_down(sindri::KeyCode::KeyW) {
            pan.y -= pan_speed * dt;
        }
        if input.is_key_down(sindri::KeyCode::ArrowDown) || input.is_key_down(sindri::KeyCode::KeyS)
        {
            pan.y += pan_speed * dt;
        }
        self.camera.position = self.camera.position + pan;

        // Camera zoom with +/- or scroll wheel
        if input.is_key_pressed(sindri::KeyCode::Equal)
            || input.is_key_pressed(sindri::KeyCode::NumpadAdd)
        {
            self.camera.zoom = (self.camera.zoom * 1.2).min(4.0);
        }
        if input.is_key_pressed(sindri::KeyCode::Minus)
            || input.is_key_pressed(sindri::KeyCode::NumpadSubtract)
        {
            self.camera.zoom = (self.camera.zoom / 1.2).max(0.25);
        }

        // Brush size with [ and ]
        if input.is_key_pressed(sindri::KeyCode::BracketLeft) {
            self.brush_size = (self.brush_size - 1).max(1);
        }
        if input.is_key_pressed(sindri::KeyCode::BracketRight) {
            self.brush_size = (self.brush_size + 1).min(10);
        }

        // Tile selection with number keys (1-9, 0) - quick select first 10 tiles
        for (key, tile_id) in [
            (sindri::KeyCode::Digit1, 1),
            (sindri::KeyCode::Digit2, 2),
            (sindri::KeyCode::Digit3, 3),
            (sindri::KeyCode::Digit4, 4),
            (sindri::KeyCode::Digit5, 5),
            (sindri::KeyCode::Digit6, 6),
            (sindri::KeyCode::Digit7, 7),
            (sindri::KeyCode::Digit8, 8),
            (sindri::KeyCode::Digit9, 9),
            (sindri::KeyCode::Digit0, 10),
        ] {
            if input.is_key_pressed(key) {
                self.selected_tile_id = tile_id;
            }
        }

        // Mouse input
        let mouse_screen = input.mouse_position_vec2();

        // Check if clicking in tile selector panel
        if mouse_screen.x >= self.selector_panel_x
            && input.is_mouse_pressed(sindri::MouseButton::Left)
        {
            // Clicked in selector panel - select tile
            let panel_local_x = mouse_screen.x - self.selector_panel_x;
            let panel_local_y = mouse_screen.y - 50.0; // Account for top HUD space

            if panel_local_y >= 0.0 {
                let tile_size_selector = 32.0 * self.selector_scale;
                let col = (panel_local_x / tile_size_selector) as u32;
                let row = (panel_local_y / tile_size_selector) as u32;
                let tile_id = row * self.selector_tiles_per_row + col + 1; // +1 because tile IDs start at 1

                if tile_id <= 900 {
                    // Max tiles in 30x30 tileset
                    self.selected_tile_id = tile_id;
                }
            }
        }

        // Mouse painting on map (only if not clicking in selector)
        if mouse_screen.x < self.selector_panel_x {
            let mouse_world = self
                .camera
                .screen_to_world(mouse_screen, screen_w, screen_h);

            if let Some(entity) = self.tilemap_entity {
                if let Some(tilemap_comp) = self.world.get::<TilemapComponent>(entity) {
                    let (tile_x, tile_y) = tilemap_comp.tilemap.world_to_tile(mouse_world);

                    if tile_x >= 0
                        && tile_y >= 0
                        && tile_x < self.map_width as i32
                        && tile_y < self.map_height as i32
                    {
                        let tx = tile_x as u32;
                        let ty = tile_y as u32;

                        // Check if mouse is down and position changed (for continuous painting)
                        let current_pos = Some((tx, ty));
                        let pos_changed = self.last_paint_pos != current_pos;

                        if input.is_mouse_down(sindri::MouseButton::Left) {
                            if !self.is_painting || pos_changed {
                                self.paint_tile(tx, ty);
                                self.is_painting = true;
                                self.last_paint_pos = current_pos;
                            }
                        } else if input.is_mouse_down(sindri::MouseButton::Right) {
                            if !self.is_painting || pos_changed {
                                self.erase_tile(tx, ty);
                                self.is_painting = true;
                                self.last_paint_pos = current_pos;
                            }
                        } else {
                            self.is_painting = false;
                            self.last_paint_pos = None;
                        }
                    }
                }
            }
        }

        self.camera.update(dt);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        renderer.clear(&mut frame, [0.2, 0.2, 0.25, 1.0])?;

        // Draw tilemap
        if let Some(entity) = self.tilemap_entity {
            if let Some(tilemap_comp) = self.world.get::<TilemapComponent>(entity) {
                renderer.draw_tilemap(&mut frame, &tilemap_comp.tilemap, &self.camera)?;
            }
        }

        // Draw grid overlay
        self.draw_grid(renderer, &mut frame, &self.camera)?;

        // Draw tile selector panel
        self.draw_tile_selector(renderer, &mut frame)?;

        // Draw HUD
        self.hud.clear();
        if let Some(font) = self.font {
            let instructions = vec![
                "Tilemap Editor".to_string(),
                format!("Selected Tile: {}", self.selected_tile_id),
                format!("Brush Size: {}x{}", self.brush_size, self.brush_size),
                "".to_string(),
                "Controls:".to_string(),
                "Left Click: Paint tile".to_string(),
                "Right Click: Erase tile".to_string(),
                "Click tiles on right: Select".to_string(),
                "Arrow Keys / WASD: Pan camera".to_string(),
                "+/-: Zoom in/out".to_string(),
                "[ ]: Brush size".to_string(),
                "1-9, 0: Quick select (1-10)".to_string(),
            ];

            for (i, text) in instructions.iter().enumerate() {
                self.hud.add_text(HudText {
                    text: text.clone(),
                    font,
                    size: if i == 0 { 24.0 } else { 18.0 },
                    position: Vec2::new(10.0, 10.0 + (i as f32 * 22.0)),
                    color: if i == 0 {
                        [1.0, 1.0, 0.0, 1.0]
                    } else {
                        [0.9, 0.9, 0.9, 1.0]
                    },
                    ..Default::default()
                });
            }
        }
        self.hud.draw(renderer, &mut frame)?;

        renderer.end_frame(frame)?;
        Ok(())
    }
}

impl TilemapEditor {
    fn draw_grid(
        &self,
        renderer: &mut Renderer,
        frame: &mut sindri::Frame,
        camera: &Camera2D,
    ) -> Result<()> {
        let (screen_w, screen_h) = renderer.surface_size();
        let (viewport_min, viewport_max) = camera.viewport_bounds(screen_w, screen_h);

        let tile_size = 32.0;

        // Draw vertical grid lines
        let start_x = (viewport_min.x / tile_size).floor() * tile_size;
        let end_x = (viewport_max.x / tile_size).ceil() * tile_size;

        for x in (start_x as i32..=end_x as i32).step_by(tile_size as usize) {
            let x_f = x as f32;
            if x_f >= viewport_min.x && x_f <= viewport_max.x {
                let line_width = 1.0; // Make lines thicker
                let line_points = vec![
                    Vec2::new(x_f - line_width, viewport_min.y),
                    Vec2::new(x_f + line_width, viewport_min.y),
                    Vec2::new(x_f + line_width, viewport_max.y),
                    Vec2::new(x_f - line_width, viewport_max.y),
                ];
                renderer.draw_polygon_no_occlusion(
                    frame,
                    &line_points,
                    [0.5, 0.5, 0.5, 0.8],
                    camera,
                )?; // More visible
            }
        }

        // Draw horizontal grid lines
        let start_y = (viewport_min.y / tile_size).floor() * tile_size;
        let end_y = (viewport_max.y / tile_size).ceil() * tile_size;

        for y in (start_y as i32..=end_y as i32).step_by(tile_size as usize) {
            let y_f = y as f32;
            if y_f >= viewport_min.y && y_f <= viewport_max.y {
                let line_width = 1.0; // Make lines thicker
                let line_points = vec![
                    Vec2::new(viewport_min.x, y_f - line_width),
                    Vec2::new(viewport_max.x, y_f - line_width),
                    Vec2::new(viewport_max.x, y_f + line_width),
                    Vec2::new(viewport_min.x, y_f + line_width),
                ];
                renderer.draw_polygon_no_occlusion(
                    frame,
                    &line_points,
                    [0.5, 0.5, 0.5, 0.8],
                    camera,
                )?; // More visible
            }
        }

        Ok(())
    }

    fn draw_tile_selector(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut sindri::Frame,
    ) -> Result<()> {
        if let Some(tileset) = self.tileset {
            let (screen_w, screen_h) = renderer.surface_size();
            let tile_size_selector = 32.0 * self.selector_scale;
            let start_y = 50.0; // Below HUD

            // Draw selector panel background
            self.hud.add_rect(HudRect {
                position: Vec2::new(self.selector_panel_x, 0.0),
                size: Vec2::new(self.selector_panel_width, screen_h as f32),
                color: [0.1, 0.1, 0.15, 0.9],
            });

            // Create HUD camera for screen-space rendering
            let hud_camera = Camera2D::new(Vec2::new(screen_w as f32 / 2.0, screen_h as f32 / 2.0));

            // Draw tiles in selector (show first 100 tiles in a 10x10 grid)
            let tiles_to_show = (self.selector_tiles_per_row * 10).min(100);
            if let Some(entity) = self.tilemap_entity {
                if let Some(tilemap_comp) = self.world.get::<TilemapComponent>(entity) {
                    for tile_id in 1..=tiles_to_show {
                        let tile_index = (tile_id - 1) as u32;
                        let col = tile_index % self.selector_tiles_per_row;
                        let row = tile_index / self.selector_tiles_per_row;

                        let x = self.selector_panel_x + col as f32 * tile_size_selector;
                        let y = start_y + row as f32 * tile_size_selector;

                        // Get UV rect for this tile
                        if let Some(uv_rect) = tilemap_comp.tilemap.tile_uv_rect(tile_id) {
                            // Convert screen position to world position for HUD camera
                            // HUD camera centers at (screen_w/2, screen_h/2), so we need to offset
                            // Screen coordinates: (0,0) is top-left, Y increases downward
                            // World coordinates: camera center is at (screen_w/2, screen_h/2), Y increases upward
                            let world_x = x - screen_w as f32 / 2.0;
                            let world_y = screen_h as f32 / 2.0 - y; // Flip Y: screen Y down = world Y up

                            let transform = Transform2D {
                                position: Vec2::new(
                                    world_x + tile_size_selector / 2.0,
                                    world_y - tile_size_selector / 2.0,
                                ),
                                rotation: 0.0,
                                scale: Vec2::new(tile_size_selector, tile_size_selector),
                            };

                            renderer.draw_texture_region(
                                frame,
                                tileset,
                                Some(uv_rect),
                                &transform,
                                if tile_id == self.selected_tile_id {
                                    [1.2, 1.2, 1.2, 1.0] // Highlight selected
                                } else {
                                    [1.0, 1.0, 1.0, 1.0]
                                },
                                false, // Not occluders in HUD
                                &hud_camera,
                            )?;

                            // Draw selection border using HUD rect
                            if tile_id == self.selected_tile_id {
                                self.hud.add_rect(HudRect {
                                    position: Vec2::new(x, y),
                                    size: Vec2::new(tile_size_selector, tile_size_selector),
                                    color: [1.0, 1.0, 0.0, 0.5], // Yellow border, semi-transparent
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri Tilemap Editor")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(TilemapEditor::new()?)
}
