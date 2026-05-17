use anyhow::Result;
use sindri::{
    BuiltinFont, Camera2D, Engine, EngineContext, FontHandle, Game, KeyCode, MouseButton, Sprite,
    Vec2,
};

// Simple white square texture (32x32)
const WHITE_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20, 0x08, 0x06, 0x00, 0x00, 0x00, 0x73, 0x7a, 0x7a,
    0xf4, 0x00, 0x00, 0x00, 0x2f, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0xed, 0xce, 0x31, 0x01, 0x00,
    0x00, 0x08, 0xc3, 0xb0, 0x81, 0x7f, 0xcf, 0x43, 0x06, 0x4f, 0x6a, 0xa0, 0x99, 0xb6, 0xcd, 0x63,
    0xfb, 0x39, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x48, 0x92, 0x03, 0x4d, 0x88, 0x04, 0x3c, 0x4a, 0xbd, 0x9d, 0x15, 0x00, 0x00, 0x00, 0x00,
    0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

struct CameraDemo {
    camera: Camera2D,
    font: Option<FontHandle>,

    // Demo objects
    grid_sprites: Vec<Sprite>,
    test_objects: Vec<Sprite>,

    // Camera state
    rotation_speed: f32,
    zoom_target: f32,
    shake_cooldown: f32,

    // World bounds
    world_min: Vec2,
    world_max: Vec2,

    // Mouse tracking for zoom-to-point
    last_mouse_world: Vec2,
}

impl CameraDemo {
    fn new() -> Self {
        // Create world bounds
        let world_min = Vec2::new(-500.0, -500.0);
        let world_max = Vec2::new(500.0, 500.0);

        // Initialize camera with bounds
        let camera = Camera2D::new(Vec2::ZERO).with_bounds(world_min, world_max);

        Self {
            camera,
            font: None,
            grid_sprites: Vec::new(),
            test_objects: Vec::new(),
            rotation_speed: 0.0,
            zoom_target: 1.0,
            shake_cooldown: 0.0,
            world_min,
            world_max,
            last_mouse_world: Vec2::ZERO,
        }
    }

    fn create_grid(&mut self, renderer: &mut sindri::Renderer) -> Result<()> {
        self.grid_sprites.clear();

        // Create a grid of sprites
        let grid_size = 20;
        let spacing = 50.0;
        let start = -((grid_size / 2) as f32) * spacing;

        for x in 0..grid_size {
            for y in 0..grid_size {
                let pos = Vec2::new(start + x as f32 * spacing, start + y as f32 * spacing);

                let mut sprite = Sprite::new(renderer.load_texture_from_bytes(WHITE_PNG)?);
                sprite.transform.position = pos;
                sprite.set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
                sprite.tint = [
                    (x as f32 / grid_size as f32) * 0.5 + 0.3,
                    (y as f32 / grid_size as f32) * 0.5 + 0.3,
                    0.8,
                    1.0,
                ];

                self.grid_sprites.push(sprite);
            }
        }

        Ok(())
    }

    fn create_test_objects(&mut self, renderer: &mut sindri::Renderer) -> Result<()> {
        self.test_objects.clear();

        // Create some test objects at different positions
        let positions = vec![
            Vec2::new(-200.0, -200.0),
            Vec2::new(200.0, -200.0),
            Vec2::new(200.0, 200.0),
            Vec2::new(-200.0, 200.0),
            Vec2::new(0.0, 0.0),
        ];

        for (i, pos) in positions.iter().enumerate() {
            let mut sprite = Sprite::new(renderer.load_texture_from_bytes(WHITE_PNG)?);
            sprite.transform.position = *pos;
            let size = 64.0 + i as f32 * 16.0;
            sprite.set_size_px(Vec2::new(size, size), Vec2::new(size, size));

            // Different colors
            let colors = [
                [1.0, 0.0, 0.0, 1.0], // Red
                [0.0, 1.0, 0.0, 1.0], // Green
                [0.0, 0.0, 1.0, 1.0], // Blue
                [1.0, 1.0, 0.0, 1.0], // Yellow
                [1.0, 0.0, 1.0, 1.0], // Magenta
            ];
            sprite.tint = colors[i % colors.len()];

            self.test_objects.push(sprite);
        }

        Ok(())
    }
}

impl Game for CameraDemo {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Load font
        self.font = ctx.builtin_font(BuiltinFont::Ui).ok();

        // Create grid and test objects
        {
            let renderer = ctx.renderer();
            self.create_grid(renderer)?;
            self.create_test_objects(renderer)?;
        }

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        let (screen_w, screen_h) = {
            let renderer = ctx.renderer();
            renderer.surface_size()
        };
        let input = ctx.input();

        // Update camera (handles smooth zoom, shake decay, bounds clamping)
        self.camera.update(dt);

        // Update shake cooldown
        if self.shake_cooldown > 0.0 {
            self.shake_cooldown -= dt;
        }

        // Camera movement (WASD / Arrow keys)
        let mut move_dir = Vec2::ZERO;
        if input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp) {
            move_dir.y -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyS) || input.is_key_down(KeyCode::ArrowDown) {
            move_dir.y += 1.0;
        }
        if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
            move_dir.x -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
            move_dir.x += 1.0;
        }

        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalized();
            self.camera.position += move_dir * 200.0 * dt;
        }

        // Camera rotation (Q/E keys)
        if input.is_key_down(KeyCode::KeyQ) {
            self.camera.rotation -= 1.0 * dt;
        }
        if input.is_key_down(KeyCode::KeyE) {
            self.camera.rotation += 1.0 * dt;
        }

        // Reset rotation (R key)
        if input.is_key_pressed(KeyCode::KeyR) {
            self.camera.rotation = 0.0;
        }

        // Zoom controls (+ zooms in, - zooms out)
        let mut zoom_change = 0.0;
        if input.is_key_down(KeyCode::Equal) || input.is_key_down(KeyCode::NumpadAdd) {
            zoom_change = 1.0; // Zoom in (increase zoom value)
        }
        if input.is_key_down(KeyCode::Minus) || input.is_key_down(KeyCode::NumpadSubtract) {
            zoom_change = -1.0; // Zoom out (decrease zoom value)
        }

        if zoom_change != 0.0 {
            // Increase zoom = zoom in, decrease zoom = zoom out
            self.zoom_target = (self.camera.zoom + zoom_change * 0.5 * dt).clamp(0.1, 3.0);
            self.camera.zoom_to(self.zoom_target, 2.0);
        }

        // Zoom to point (Mouse wheel at mouse position)
        // Note: mouse_world is calculated in draw() to avoid borrow conflicts

        // Camera shake (Space key)
        if input.is_key_pressed(KeyCode::Space) && self.shake_cooldown <= 0.0 {
            self.camera.shake(10.0, 0.5);
            self.shake_cooldown = 0.1;
        }

        // Camera offset / look-ahead (Shift + WASD)
        // This moves the camera view ahead of the camera position (useful for platformers)
        if input.is_key_down(KeyCode::ShiftLeft) || input.is_key_down(KeyCode::ShiftRight) {
            let mut offset = Vec2::ZERO;
            if input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp) {
                offset.y -= 1.0; // Look up
            }
            if input.is_key_down(KeyCode::KeyS) || input.is_key_down(KeyCode::ArrowDown) {
                offset.y += 1.0; // Look down
            }
            if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
                offset.x -= 1.0; // Look left
            }
            if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
                offset.x += 1.0; // Look right
            }

            if offset.length_squared() > 0.0 {
                offset = offset.normalized() * 150.0; // Larger offset to make it more visible
                self.camera.offset = offset;
            }
        } else {
            // Reset offset when not holding shift
            self.camera.offset = self.camera.offset.lerp(Vec2::ZERO, 5.0 * dt);
        }

        // Reset camera (Backspace)
        if input.is_key_pressed(KeyCode::Backspace) {
            self.camera.position = Vec2::ZERO;
            self.camera.rotation = 0.0;
            self.camera.zoom = 1.0;
            self.camera.target_zoom = 1.0;
            self.camera.offset = Vec2::ZERO;
            self.zoom_target = 1.0;
        }

        // Toggle bounds (B key)
        if input.is_key_pressed(KeyCode::KeyB) {
            if self.camera.bounds.is_some() {
                self.camera = self.camera.without_bounds();
            } else {
                self.camera = self.camera.with_bounds(self.world_min, self.world_max);
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Get all values we need before borrowing renderer
        let (screen_w, screen_h) = {
            let renderer = ctx.renderer();
            renderer.surface_size()
        };
        let (viewport_min, viewport_max) = self.camera.viewport_bounds(screen_w, screen_h);
        let mouse_world = {
            let input = ctx.input();
            let screen_pos = input.mouse_position_vec2();
            self.camera.screen_to_world(screen_pos, screen_w, screen_h)
        };
        self.last_mouse_world = mouse_world;

        // Now borrow renderer for the rest of the function
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        // Clear with dark background
        renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;

        // Draw grid sprites (only visible ones for performance)
        for sprite in &self.grid_sprites {
            // Culling: only draw if visible
            if self
                .camera
                .is_point_visible(sprite.transform.position, screen_w, screen_h)
            {
                if let Err(e) = renderer.draw_sprite(&mut frame, sprite, &self.camera) {
                    eprintln!("Error drawing sprite: {}", e);
                }
            }
        }

        // Draw test objects
        for sprite in &self.test_objects {
            if let Err(e) = renderer.draw_sprite(&mut frame, sprite, &self.camera) {
                eprintln!("Error drawing sprite: {}", e);
            }
        }

        // Draw viewport bounds indicator (rectangle) - using polygon for outline
        let bounds_color = if self.camera.bounds.is_some() {
            [0.0, 1.0, 0.0, 0.3]
        } else {
            [1.0, 0.0, 0.0, 0.3]
        };
        let viewport_points = vec![
            viewport_min,
            Vec2::new(viewport_max.x, viewport_min.y),
            viewport_max,
            Vec2::new(viewport_min.x, viewport_max.y),
        ];
        renderer.draw_polygon(&mut frame, &viewport_points, bounds_color, &self.camera)?;

        // Draw world bounds outline
        if self.camera.bounds.is_some() {
            let world_points = vec![
                self.world_min,
                Vec2::new(self.world_max.x, self.world_min.y),
                self.world_max,
                Vec2::new(self.world_min.x, self.world_max.y),
            ];
            renderer.draw_polygon(
                &mut frame,
                &world_points,
                [0.5, 0.5, 0.5, 0.5],
                &self.camera,
            )?;
        }

        // Draw mouse position indicator (yellow)
        renderer.draw_circle(
            &mut frame,
            mouse_world,
            5.0,
            [1.0, 1.0, 0.0, 1.0],
            &self.camera,
        )?;

        // Draw camera position (cyan) and look-ahead offset (magenta)
        renderer.draw_circle(
            &mut frame,
            self.camera.position,
            8.0,
            [0.0, 1.0, 1.0, 1.0], // Cyan for camera position
            &self.camera,
        )?;

        // Draw effective position (position + offset) if offset is non-zero
        if self.camera.offset.length_squared() > 0.1 {
            let effective_pos = self.camera.position + self.camera.offset;
            renderer.draw_circle(
                &mut frame,
                effective_pos,
                6.0,
                [1.0, 0.0, 1.0, 1.0], // Magenta for look-ahead position
                &self.camera,
            )?;

            // Draw line from camera position to look-ahead (using a thin rectangle)
            let dir = (effective_pos - self.camera.position).normalized();
            let perp = Vec2::new(-dir.y, dir.x) * 2.0; // Perpendicular for line width
            let dist = (effective_pos - self.camera.position).length();
            let line_points = vec![
                self.camera.position + perp,
                self.camera.position - perp,
                effective_pos - perp,
                effective_pos + perp,
            ];
            renderer.draw_polygon(&mut frame, &line_points, [1.0, 0.0, 1.0, 0.5], &self.camera)?;
        }

        // Draw HUD text
        if let Some(font) = self.font {
            let mut y_offset = 20.0;
            let line_height = 25.0;

            // Build all text strings first (to avoid temporary value issues)
            let text_lines = vec![
                "=== Camera Features Demo ===".to_string(),
                "".to_string(),
                "Movement: WASD / Arrow Keys".to_string(),
                "Rotation: Q/E (Reset: R)".to_string(),
                "Zoom: +/- keys".to_string(),
                "Shake: SPACE".to_string(),
                "Look-ahead: Shift + WASD".to_string(),
                "Toggle Bounds: B".to_string(),
                "Reset Camera: Backspace".to_string(),
                "".to_string(),
                format!(
                    "Position: ({:.1}, {:.1})",
                    self.camera.position.x, self.camera.position.y
                ),
                format!("Rotation: {:.1}°", self.camera.rotation.to_degrees()),
                format!("Zoom: {:.2}x (higher = zoomed in)", self.camera.zoom),
                format!(
                    "Offset: ({:.1}, {:.1}) - Look-ahead from position",
                    self.camera.offset.x, self.camera.offset.y
                ),
                format!(
                    "Bounds: {}",
                    if self.camera.bounds.is_some() {
                        "ON"
                    } else {
                        "OFF"
                    }
                ),
                format!("Shake: {:.2}", self.camera.shake_intensity),
                "".to_string(),
                format!(
                    "Viewport: ({:.1}, {:.1}) to ({:.1}, {:.1})",
                    viewport_min.x, viewport_min.y, viewport_max.x, viewport_max.y
                ),
                format!("Mouse World: ({:.1}, {:.1})", mouse_world.x, mouse_world.y),
            ];

            for line in &text_lines {
                if let Err(e) = renderer.draw_text(
                    &mut frame,
                    line,
                    font,
                    16.0,
                    Vec2::new(10.0, y_offset),
                    [1.0, 1.0, 1.0, 1.0],
                    &self.camera,
                ) {
                    eprintln!("Error drawing text: {}", e);
                }
                y_offset += line_height;
            }
        }

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri - Camera Features Demo")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(CameraDemo::new())
}
