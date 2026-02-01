use anyhow::Result;
use forge2d::{
    Camera2D, Engine, EngineContext, Game, KeyCode, MouseButton, Sprite, TextureHandle, Vec2,
};

const DOT_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x20,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x73, 0x7a, 0x7a, 0xf4, 0x00, 0x00, 0x00,
    0x2f, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0xed, 0xce, 0x31, 0x01, 0x00,
    0x00, 0x08, 0xc3, 0xb0, 0x81, 0x7f, 0xcf, 0x43, 0x06, 0x4f, 0x6a, 0xa0,
    0x99, 0xb6, 0xcd, 0x63, 0xfb, 0x39, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x92, 0x03,
    0x4d, 0x88, 0x04, 0x3c, 0x4a, 0xbd, 0x9d, 0x15, 0x00, 0x00, 0x00, 0x00,
    0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

const DOT_SIZE_PX: Vec2 = Vec2 { x: 32.0, y: 32.0 };

#[derive(Clone)]
struct Firefly {
    pos: Vec2,
    prev_pos: Vec2,
    vel: Vec2,
    color: [f32; 4],
    size: f32,
}

#[derive(Clone)]
struct Star {
    pos: Vec2,
    size: f32,
    alpha: f32,
}

struct FirefliesGame {
    camera: Camera2D,
    world_size: Vec2,
    dot_texture: Option<TextureHandle>,
    fireflies: Vec<Firefly>,
    stars: Vec<Star>,
    attractor: Vec2,
    attractor_strength: f32,
    repel: bool,
    scatter_requested: bool,
}

impl FirefliesGame {
    fn new() -> Self {
        Self {
            camera: Camera2D::default(),
            world_size: Vec2::new(960.0, 540.0),
            dot_texture: None,
            fireflies: Vec::new(),
            stars: Vec::new(),
            attractor: Vec2::ZERO,
            attractor_strength: 500.0,
            repel: false,
            scatter_requested: false,
        }
    }

    fn build_firefly(texture: TextureHandle, size: f32, color: [f32; 4]) -> Sprite {
        let mut sprite = Sprite::new(texture);
        sprite.set_size_px(Vec2::new(size, size), DOT_SIZE_PX);
        sprite.tint = color;
        sprite.is_occluder = false;
        sprite
    }
}

impl Game for FirefliesGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.dot_texture = Some(ctx.load_texture_from_bytes("dot", DOT_PNG)?);

        let (screen_w, screen_h) = ctx.renderer().surface_size();
        self.world_size = Vec2::new(screen_w as f32, screen_h as f32);
        self.camera = ctx.screen_camera();
        self.camera.zoom = 1.0;

        let palette = [
            [1.0, 0.9, 0.6, 0.95],
            [0.6, 1.0, 0.8, 0.9],
            [0.8, 0.7, 1.0, 0.9],
            [0.9, 0.8, 1.0, 0.8],
        ];

        self.fireflies.clear();
        for _ in 0..140 {
            let pos = Vec2::new(
                fastrand::f32() * self.world_size.x,
                fastrand::f32() * self.world_size.y,
            );
            let vel = Vec2::new(
                (fastrand::f32() - 0.5) * 60.0,
                (fastrand::f32() - 0.5) * 60.0,
            );
            let color = palette[fastrand::usize(..palette.len())];
            let size = 6.0 + fastrand::f32() * 10.0;
            self.fireflies.push(Firefly {
                pos,
                prev_pos: pos,
                vel,
                color,
                size,
            });
        }

        self.stars.clear();
        for _ in 0..80 {
            let pos = Vec2::new(
                fastrand::f32() * self.world_size.x,
                fastrand::f32() * self.world_size.y,
            );
            let size = 1.5 + fastrand::f32() * 2.5;
            let alpha = 0.2 + fastrand::f32() * 0.6;
            self.stars.push(Star { pos, size, alpha });
        }

        self.attractor = Vec2::new(self.world_size.x * 0.5, self.world_size.y * 0.5);

        Ok(())
    }

    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.fixed_delta_seconds();
        let strength = if self.repel { -self.attractor_strength } else { self.attractor_strength };
        let scatter = self.scatter_requested;

        for firefly in &mut self.fireflies {
            firefly.prev_pos = firefly.pos;

            let to = self.attractor - firefly.pos;
            let dist = to.length().max(1.0);
            let dir = to.normalized();
            let swirl = Vec2::new(-dir.y, dir.x);
            let accel = dir * (strength / (dist + 40.0)) + swirl * (strength * 0.004);

            if scatter {
                let angle = fastrand::f32() * std::f32::consts::TAU;
                let impulse = Vec2::from_angle(angle) * (80.0 + fastrand::f32() * 120.0);
                firefly.vel += impulse;
            }

            firefly.vel += accel * dt;
            let damping = (1.0 - 0.8 * dt).clamp(0.0, 1.0);
            firefly.vel = firefly.vel * damping;
            firefly.pos += firefly.vel * dt;

            if firefly.pos.x < 0.0 {
                firefly.pos.x += self.world_size.x;
                firefly.prev_pos.x += self.world_size.x;
            } else if firefly.pos.x > self.world_size.x {
                firefly.pos.x -= self.world_size.x;
                firefly.prev_pos.x -= self.world_size.x;
            }

            if firefly.pos.y < 0.0 {
                firefly.pos.y += self.world_size.y;
                firefly.prev_pos.y += self.world_size.y;
            } else if firefly.pos.y > self.world_size.y {
                firefly.pos.y -= self.world_size.y;
                firefly.prev_pos.y -= self.world_size.y;
            }
        }

        self.scatter_requested = false;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let (screen_w, screen_h) = ctx.renderer().surface_size();
        self.world_size = Vec2::new(screen_w as f32, screen_h as f32);

        let input = ctx.input();
        self.repel = input.is_mouse_down(MouseButton::Right);

        if input.is_key_pressed(KeyCode::Space) {
            self.scatter_requested = true;
        }

        if input.is_mouse_down(MouseButton::Left) {
            self.attractor = ctx.mouse_world(&self.camera);
        } else {
            let t = ctx.elapsed_time().as_secs_f32();
            let center = Vec2::new(self.world_size.x * 0.5, self.world_size.y * 0.5);
            self.attractor = Vec2::new(
                center.x + t.cos() * (self.world_size.x * 0.2),
                center.y + (t * 0.7).sin() * (self.world_size.y * 0.15),
            );
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let alpha = ctx.fixed_update_alpha();
        let texture = match self.dot_texture {
            Some(handle) => handle,
            None => return Ok(()),
        };

        ctx.draw(|renderer, frame| {
            renderer.clear(frame, [0.03, 0.05, 0.07, 1.0])?;

            for star in &self.stars {
                let mut sprite = Self::build_firefly(texture, star.size, [0.8, 0.9, 1.0, star.alpha]);
                sprite.transform.position = star.pos;
                renderer.draw_sprite(frame, &sprite, &self.camera)?;
            }

            for firefly in &self.fireflies {
                let mut sprite = Self::build_firefly(texture, firefly.size, firefly.color);
                sprite.transform.position = firefly.prev_pos.lerp(firefly.pos, alpha);
                renderer.draw_sprite(frame, &sprite, &self.camera)?;
            }

            let mut hub = Self::build_firefly(texture, 16.0, [1.0, 0.85, 0.6, 0.7]);
            hub.transform.position = self.attractor;
            renderer.draw_sprite(frame, &hub, &self.camera)?;

            Ok(())
        })
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Fireflies: Signal Drift")
        .with_size(960, 540)
        .run(FirefliesGame::new())
}
