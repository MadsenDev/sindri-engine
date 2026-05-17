// Ultra-simple test: Can we make a moving square in < 50 lines?

use anyhow::Result;
use sindri::{Engine, EngineContext, Game, KeyCode, Sprite, Vec2};

struct SimpleTest {
    square: Option<Sprite>,
    pos: Vec2,
    vel: Vec2,
}

impl Game for SimpleTest {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Create a simple white square texture
        let size = 32;
        let data: Vec<u8> = (0..(4 * size * size))
            .flat_map(|_| [255u8, 255, 255, 255])
            .collect();
        let texture = ctx
            .renderer()
            .load_texture_from_rgba(&data, size as u32, size as u32)?;

        let mut sprite = Sprite::new(texture);
        sprite.transform.position = Vec2::new(400.0, 300.0);
        self.square = Some(sprite);
        self.pos = Vec2::new(400.0, 300.0);
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        let input = ctx.input();

        // Simple movement
        if input.is_key_down(KeyCode::KeyW) {
            self.vel.y -= 200.0 * dt;
        }
        if input.is_key_down(KeyCode::KeyS) {
            self.vel.y += 200.0 * dt;
        }
        if input.is_key_down(KeyCode::KeyA) {
            self.vel.x -= 200.0 * dt;
        }
        if input.is_key_down(KeyCode::KeyD) {
            self.vel.x += 200.0 * dt;
        }

        // Apply velocity with friction
        self.vel *= 0.9;
        self.pos += self.vel * dt;

        if let Some(ref mut square) = self.square {
            square.transform.position = self.pos;
        }

        if input.is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.1, 0.1, 0.2, 1.0])?;
        if let Some(ref square) = self.square {
            renderer.draw_sprite(&mut frame, square, &sindri::Camera2D::default())?;
        }
        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Simple Test")
        .with_size(800, 600)
        .run(SimpleTest {
            square: None,
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
        })
}
