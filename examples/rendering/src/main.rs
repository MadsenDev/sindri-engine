//! Rendering and polish showcase.
//!
//! Controls: mouse moves the light, Space scatters particles, Q/E zoom.

use anyhow::Result;
use sindri::{
    Camera2D, EmissionConfig, Engine, EngineContext, Game, KeyCode, ParticleEmitter,
    ParticleSystem, PointLight, Sprite, TextureHandle, Vec2,
};

struct Orb {
    position: Vec2,
    orbit_radius: f32,
    orbit_speed: f32,
    phase: f32,
    size: f32,
    tint: [f32; 4],
}

struct RenderingShowcase {
    camera: Camera2D,
    white_texture: Option<TextureHandle>,
    particle_system: ParticleSystem,
    orbs: Vec<Orb>,
    light: PointLight,
    time: f32,
}

impl RenderingShowcase {
    fn new() -> Self {
        let mut particles = ParticleSystem::new();
        particles.add_emitter(
            ParticleEmitter::new(
                EmissionConfig::new(Vec2::new(640.0, 360.0))
                    .with_rate(120.0)
                    .with_velocity(Vec2::new(-80.0, -100.0), Vec2::new(80.0, 80.0))
                    .with_size(Vec2::new(3.0, 3.0), Vec2::new(12.0, 12.0))
                    .with_color([0.25, 0.7, 1.0, 0.95], Some([1.0, 0.6, 0.18, 0.0]))
                    .with_lifetime(0.7, 1.8)
                    .with_acceleration(Vec2::new(0.0, 24.0))
                    .with_size_end_multiplier(0.35),
            )
            .with_max_particles(700),
        );

        let palette = [
            [0.28, 0.65, 1.0, 1.0],
            [1.0, 0.68, 0.22, 1.0],
            [0.46, 0.92, 0.62, 1.0],
            [0.82, 0.52, 1.0, 1.0],
        ];

        let mut orbs = Vec::new();
        for i in 0..90 {
            let phase = i as f32 * 0.43;
            orbs.push(Orb {
                position: Vec2::ZERO,
                orbit_radius: 80.0 + (i % 9) as f32 * 32.0,
                orbit_speed: 0.25 + (i % 7) as f32 * 0.08,
                phase,
                size: 9.0 + (i % 5) as f32 * 3.0,
                tint: palette[i % palette.len()],
            });
        }

        Self {
            camera: Camera2D::default(),
            white_texture: None,
            particle_system: particles,
            orbs,
            light: PointLight::new(Vec2::new(640.0, 360.0), [1.0, 0.82, 0.5], 1.8, 320.0),
            time: 0.0,
        }
    }
}

impl Game for RenderingShowcase {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.camera = ctx.screen_camera();
        self.white_texture = Some(ctx.renderer().load_texture_from_rgba(
            &[255, 255, 255, 255],
            1,
            1,
        )?);
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        self.time += dt;

        if ctx.input().is_key_down(KeyCode::KeyQ) {
            self.camera.zoom = (self.camera.zoom - dt * 0.5).max(0.55);
        }
        if ctx.input().is_key_down(KeyCode::KeyE) {
            self.camera.zoom = (self.camera.zoom + dt * 0.5).min(1.6);
        }
        if ctx.input().is_key_pressed(KeyCode::Space) {
            for emitter in self.particle_system.emitters_mut() {
                emitter.set_position(Vec2::new(
                    180.0 + fastrand::f32() * 920.0,
                    160.0 + fastrand::f32() * 400.0,
                ));
            }
        }

        let mouse = ctx.mouse_world(&self.camera);
        self.light.position = mouse;

        for (i, orb) in self.orbs.iter_mut().enumerate() {
            let angle = self.time * orb.orbit_speed + orb.phase;
            let wobble = (self.time * 0.9 + i as f32).sin() * 18.0;
            orb.position = Vec2::new(
                640.0 + angle.cos() * (orb.orbit_radius + wobble),
                360.0 + angle.sin() * (orb.orbit_radius * 0.58),
            );
        }

        self.particle_system.update(dt);
        self.camera.update(dt);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let Some(texture) = self.white_texture else {
            return Ok(());
        };

        ctx.draw(|renderer, frame| {
            renderer.clear(frame, [0.018, 0.022, 0.032, 1.0])?;

            for y in 0..8 {
                let yy = 96.0 + y as f32 * 76.0;
                renderer.draw_polygon_no_occlusion(
                    frame,
                    &[
                        Vec2::new(80.0, yy),
                        Vec2::new(1200.0, yy),
                        Vec2::new(1200.0, yy + 1.5),
                        Vec2::new(80.0, yy + 1.5),
                    ],
                    [0.08, 0.1, 0.14, 1.0],
                    &self.camera,
                )?;
            }

            for orb in &self.orbs {
                let mut sprite = Sprite::new(texture);
                sprite.set_size_px(Vec2::new(orb.size, orb.size), Vec2::new(1.0, 1.0));
                sprite.transform.position = orb.position;
                sprite.transform.rotation = self.time * 0.7 + orb.phase;
                sprite.tint = orb.tint;
                sprite.is_occluder = true;
                renderer.draw_sprite(frame, &sprite, &self.camera)?;
            }

            renderer.draw_particles(frame, &self.particle_system, &self.camera, Some(texture))?;
            renderer.draw_point_light(frame, &self.light, &self.camera)?;
            Ok(())
        })
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri · Rendering")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(RenderingShowcase::new())
}
