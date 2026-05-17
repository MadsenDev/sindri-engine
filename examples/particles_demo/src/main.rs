use anyhow::Result;
use sindri::{
    Camera2D, EmissionConfig, Engine, EngineContext, Game, ParticleEmitter, ParticleSystem, Vec2,
};

struct ParticlesDemo {
    camera: Camera2D,
    particle_system: ParticleSystem,
    time: f32,
    white_texture: Option<sindri::TextureHandle>,
}

impl ParticlesDemo {
    fn new() -> Self {
        let mut camera = Camera2D::default();
        camera.position = Vec2::new(0.0, 0.0);
        camera.zoom = 1.0;

        let mut particle_system = ParticleSystem::new();

        // Create a few different particle emitters
        // Fire effect
        let fire_config = EmissionConfig::new(Vec2::new(-200.0, -100.0))
            .with_rate(50.0)
            .with_velocity(Vec2::new(-20.0, 50.0), Vec2::new(20.0, 150.0))
            .with_size(Vec2::new(3.0, 3.0), Vec2::new(8.0, 8.0))
            .with_color([1.0, 0.3, 0.0, 1.0], Some([1.0, 0.8, 0.0, 0.0]))
            .with_lifetime(0.5, 1.5)
            .with_acceleration(Vec2::new(0.0, -50.0))
            .with_size_end_multiplier(0.5);

        let mut fire_emitter = ParticleEmitter::new(fire_config).with_max_particles(200);
        particle_system.add_emitter(fire_emitter);

        // Sparkle effect
        let sparkle_config = EmissionConfig::new(Vec2::new(200.0, 100.0))
            .with_rate(30.0)
            .with_velocity(Vec2::new(-100.0, -100.0), Vec2::new(100.0, 100.0))
            .with_size(Vec2::new(2.0, 2.0), Vec2::new(4.0, 4.0))
            .with_color([1.0, 1.0, 0.5, 1.0], Some([0.5, 0.8, 1.0, 0.0]))
            .with_lifetime(1.0, 2.0)
            .with_acceleration(Vec2::new(0.0, -30.0));

        let mut sparkle_emitter = ParticleEmitter::new(sparkle_config).with_max_particles(150);
        particle_system.add_emitter(sparkle_emitter);

        // Smoke effect
        let smoke_config = EmissionConfig::new(Vec2::new(0.0, -150.0))
            .with_rate(20.0)
            .with_velocity(Vec2::new(-30.0, 20.0), Vec2::new(30.0, 60.0))
            .with_size(Vec2::new(8.0, 8.0), Vec2::new(15.0, 15.0))
            .with_color([0.3, 0.3, 0.3, 0.8], Some([0.1, 0.1, 0.1, 0.0]))
            .with_lifetime(2.0, 4.0)
            .with_acceleration(Vec2::new(0.0, -10.0))
            .with_size_end_multiplier(1.5);

        let mut smoke_emitter = ParticleEmitter::new(smoke_config).with_max_particles(100);
        particle_system.add_emitter(smoke_emitter);

        Self {
            camera,
            particle_system,
            time: 0.0,
            white_texture: None,
        }
    }
}

impl Game for ParticlesDemo {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Create a simple white 1x1 texture for particles
        let white_pixel = [255u8, 255u8, 255u8, 255u8];
        self.white_texture = Some(ctx.renderer().load_texture_from_rgba(&white_pixel, 1, 1)?);
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        self.time += dt;

        // Update particle system
        self.particle_system.update(dt);

        // Animate emitters
        if let Some(emitter) = self.particle_system.emitters_mut().get_mut(0) {
            // Fire emitter - move in a circle
            let x = -200.0 + (self.time * 0.5).cos() * 50.0;
            let y = -100.0 + (self.time * 0.5).sin() * 30.0;
            emitter.set_position(Vec2::new(x, y));
        }

        if let Some(emitter) = self.particle_system.emitters_mut().get_mut(1) {
            // Sparkle emitter - move up and down
            let y = 100.0 + (self.time * 0.8).sin() * 80.0;
            emitter.set_position(Vec2::new(200.0, y));
        }

        // Update camera
        self.camera.update(dt);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;

        // Draw a dark background
        let bg_points = vec![
            Vec2::new(-500.0, -300.0),
            Vec2::new(500.0, -300.0),
            Vec2::new(500.0, 300.0),
            Vec2::new(-500.0, 300.0),
        ];
        renderer.draw_polygon(&mut frame, &bg_points, [0.05, 0.05, 0.1, 1.0], &self.camera)?;

        // Draw particles using the white texture
        renderer.draw_particles(
            &mut frame,
            &self.particle_system,
            &self.camera,
            self.white_texture,
        )?;

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Particles Demo - Sindri")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(ParticlesDemo::new())
}
