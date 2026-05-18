//! Tiny Sindri onboarding example.
//!
//! Controls: WASD/arrow keys move, Space changes color.

use anyhow::Result;
use sindri::{Camera2D, Engine, EngineContext, Game, KeyCode, Sprite, Vec2};

struct HelloSindri {
    camera: Camera2D,
    player: Option<Sprite>,
    velocity: Vec2,
    pulse: f32,
}

impl HelloSindri {
    fn new() -> Self {
        Self {
            camera: Camera2D::default(),
            player: None,
            velocity: Vec2::ZERO,
            pulse: 0.0,
        }
    }
}

impl Game for HelloSindri {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.camera = ctx.screen_camera();

        let white_pixel = [255u8, 255, 255, 255];
        let texture = ctx.renderer().load_texture_from_rgba(&white_pixel, 1, 1)?;
        let mut player = Sprite::new(texture);
        player.set_size_px(Vec2::new(48.0, 48.0), Vec2::new(1.0, 1.0));
        player.transform.position = Vec2::new(640.0, 360.0);
        player.tint = [0.2, 0.65, 1.0, 1.0];
        self.player = Some(player);

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        let input = ctx.input();

        let mut direction = Vec2::ZERO;
        if input.is_key_down(KeyCode::KeyA) || input.is_key_down(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyD) || input.is_key_down(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }
        if input.is_key_down(KeyCode::KeyW) || input.is_key_down(KeyCode::ArrowUp) {
            direction.y -= 1.0;
        }
        if input.is_key_down(KeyCode::KeyS) || input.is_key_down(KeyCode::ArrowDown) {
            direction.y += 1.0;
        }

        if direction.length_squared() > 0.0 {
            direction = direction.normalized();
        }

        if let Some(player) = &mut self.player {
            self.velocity = direction * 260.0;
            player.transform.position += self.velocity * dt;
            player.transform.position.x = player.transform.position.x.clamp(32.0, 1248.0);
            player.transform.position.y = player.transform.position.y.clamp(32.0, 688.0);

            self.pulse += dt * 5.0;
            let hot = input.is_key_down(KeyCode::Space);
            player.tint = if hot {
                [1.0, 0.72, 0.22, 1.0]
            } else {
                [0.2, 0.65 + self.pulse.sin() * 0.12, 1.0, 1.0]
            };
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let Some(player) = &self.player else {
            return Ok(());
        };

        ctx.draw(|renderer, frame| {
            renderer.clear(frame, [0.03, 0.035, 0.045, 1.0])?;
            renderer.draw_circle(
                frame,
                player.transform.position,
                72.0,
                [0.08, 0.12, 0.18, 1.0],
                &self.camera,
            )?;
            renderer.draw_sprite(frame, player, &self.camera)
        })
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri · Hello")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(HelloSindri::new())
}
