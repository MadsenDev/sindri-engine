use anyhow::Result;
use sindri::{Camera2D, Engine, EngineContext, Game, PointLight, Vec2};

struct LightingDemo {
    camera: Camera2D,
    lights: Vec<PointLight>,
    time: f32,
}

impl LightingDemo {
    fn new() -> Self {
        let mut camera = Camera2D::default();
        camera.position = Vec2::new(0.0, 0.0);
        camera.zoom = 1.0;

        // Create a simple point light first to test
        let lights = vec![PointLight::new(
            Vec2::new(0.0, 0.0), // Center of screen
            [1.0, 1.0, 0.8],     // Warm white/yellow
            2.0,                 // High intensity
            300.0,               // Large radius
        )];

        Self {
            camera,
            lights,
            time: 0.0,
        }
    }
}

impl Game for LightingDemo {
    fn init(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        self.time += dt;

        // Animate spotlight (only one light now)
        if !self.lights.is_empty() {
            self.lights[0].position.x = -100.0 + (self.time * 0.3).sin() * 50.0;
            self.lights[0].position.y = -200.0 + (self.time * 0.2).cos() * 30.0;
        }

        // Update camera
        self.camera.update(dt);

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?; // Dark background with slight ambient

        // Draw some basic shapes to show lighting using polygons
        // Draw a large rectangle as the "ground" using polygon
        let ground_points = vec![
            Vec2::new(-400.0, 200.0),
            Vec2::new(400.0, 200.0),
            Vec2::new(400.0, 250.0),
            Vec2::new(-400.0, 250.0),
        ];
        renderer.draw_polygon_no_occlusion(
            &mut frame,
            &ground_points,
            [0.8, 0.8, 0.9, 1.0],
            &self.camera,
        )?;

        // Draw some circles
        for i in 0..5 {
            let x = (i as f32 - 2.0) * 150.0;
            renderer.draw_circle(
                &mut frame,
                Vec2::new(x, -50.0),
                40.0,
                [0.9, 0.9, 0.95, 1.0],
                &self.camera,
            )?;
        }

        // Draw some rectangles using polygons
        for i in 0..4 {
            let x = (i as f32 - 1.5) * 120.0;
            let rect_points = vec![
                Vec2::new(x - 30.0, -150.0),
                Vec2::new(x + 30.0, -150.0),
                Vec2::new(x + 30.0, -100.0),
                Vec2::new(x - 30.0, -100.0),
            ];
            renderer.draw_polygon(
                &mut frame,
                &rect_points,
                [0.95, 0.9, 0.9, 1.0],
                &self.camera,
            )?;
        }

        // Draw all lights (additive blending)
        for light in &self.lights {
            renderer.draw_point_light(&mut frame, light, &self.camera)?;
        }

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Lighting Demo - Sindri")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(LightingDemo::new())
}
