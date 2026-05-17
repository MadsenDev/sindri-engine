use anyhow::Result;
use sindri::{
    render::{AnimatedSprite, Animation},
    Camera2D, Engine, EngineContext, Game, Vec2,
};

struct AnimationDemo {
    camera: Camera2D,
    character: Option<AnimatedSprite>,
    explosion: Option<AnimatedSprite>,
}

impl AnimationDemo {
    fn new() -> Self {
        Self {
            camera: Camera2D::default(),
            character: None,
            explosion: None,
        }
    }
}

impl Game for AnimationDemo {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Load textures
        // We'll generate a dummy spritesheet for now or assume one exists/can be generated
        let character_tex = ctx.renderer().load_texture_from_rgba(
            &vec![255; 64 * 64 * 4],
            64,
            64, // Dummy 64x64 texture (checkboard?)
        )?;

        // Actually, let's create a procedural texture that looks like a grid so we can see animation
        let mut bytes = Vec::with_capacity(128 * 64 * 4);
        for y in 0..64 {
            for x in 0..128 {
                let cell_x = x / 32; // 32px width cells -> 4 cols
                let r = if cell_x == 0 { 255 } else { 0 };
                let g = if cell_x == 1 { 255 } else { 0 };
                let b = if cell_x == 2 { 255 } else { 0 };
                let a = if cell_x == 3 { 255 } else { 255 }; // col 3 is white

                // Add a border
                let is_border = x % 32 < 2 || y < 2 || y > 62;
                if is_border {
                    bytes.extend_from_slice(&[0, 0, 0, 255]);
                } else {
                    bytes.extend_from_slice(&[r, g, b, 255]);
                }
            }
        }
        let tex = ctx.renderer().load_texture_from_rgba(&bytes, 128, 64)?;

        // Create character animation (4 frames: Red, Green, Blue, White)
        // Grid: 4 cols, 2 rows (64px height / 32px height per row = 2 rows? No, 128x64. 32px cells.
        // 128 width / 32 = 4 cols.
        // 64 height / 32 = 2 rows.
        let anim = Animation::from_grid(
            tex,
            (4, 2),
            8,   // 8 frames total
            0.2, // 0.2s duration
        );

        let mut sprite = AnimatedSprite::new(anim);
        sprite.transform.position = Vec2::new(0.0, 0.0);
        sprite.transform.scale = Vec2::new(5.0, 5.0); // Scale up
        self.character = Some(sprite);

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();

        if let Some(char) = &mut self.character {
            char.update(dt);
        }

        self.camera.update(dt);
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.1, 0.1, 0.1, 1.0])?;

        if let Some(char) = &self.character {
            if let Some(frame_data) = char.current_frame() {
                renderer.draw_texture_region(
                    &mut frame,
                    frame_data.texture,
                    frame_data.source_rect,
                    &char.transform,
                    char.tint,
                    char.is_occluder,
                    &self.camera,
                )?;
            }
        }

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Animation Demo")
        .with_size(800, 600)
        .run(AnimationDemo::new())
}
