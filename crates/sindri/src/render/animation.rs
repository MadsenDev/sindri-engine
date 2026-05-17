use super::sprite::TextureHandle;
use crate::math::Transform2D;

/// A single frame of an animation.
#[derive(Clone, Debug)]
pub struct AnimationFrame {
    /// The texture to use for this frame.
    pub texture: TextureHandle,
    /// The source rectangle in the texture (normalized UV coordinates: x, y, width, height).
    /// If None, use the full texture.
    pub source_rect: Option<[f32; 4]>,
    /// How long this frame lasts in seconds.
    pub duration: f32,
}

impl AnimationFrame {
    pub fn new(texture: TextureHandle, duration: f32) -> Self {
        Self {
            texture,
            source_rect: None,
            duration,
        }
    }

    pub fn with_rect(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.source_rect = Some([x, y, w, h]);
        self
    }
}

/// An animation sequence consisting of multiple frames.
#[derive(Clone, Debug)]
pub struct Animation {
    pub frames: Vec<AnimationFrame>,
    pub looping: bool,
    pub total_duration: f32,
}

impl Animation {
    pub fn new(frames: Vec<AnimationFrame>, looping: bool) -> Self {
        let total_duration = frames.iter().map(|f| f.duration).sum();
        Self {
            frames,
            looping,
            total_duration,
        }
    }

    /// Create an animation from a spritesheet grid.
    ///
    /// # Arguments
    /// * `texture` - The spritesheet texture.
    /// * `grid_size` - (columns, rows) in the spritesheet.
    /// * `frame_count` - Total number of frames to use (starting from top-left, row by row).
    /// * `frame_duration` - Duration of each frame in seconds.
    pub fn from_grid(
        texture: TextureHandle,
        grid_size: (u32, u32),
        frame_count: usize,
        frame_duration: f32,
    ) -> Self {
        let (cols, rows) = grid_size;
        let uv_width = 1.0 / cols as f32;
        let uv_height = 1.0 / rows as f32;

        let mut frames = Vec::with_capacity(frame_count);
        for i in 0..frame_count {
            let col = (i as u32) % cols;
            let row = (i as u32) / cols;
            // Rows are typically top-to-bottom, so row 0 is y=0.

            let u = col as f32 * uv_width;
            let v = row as f32 * uv_height;

            frames.push(AnimationFrame {
                texture,
                source_rect: Some([u, v, uv_width, uv_height]),
                duration: frame_duration,
            });
        }

        Self::new(frames, true)
    }
}

/// Handle to a shared Animation resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct AnimationHandle(pub(crate) u32);

/// Component for playing an animation on an entity.
#[derive(Clone, Debug)]
pub struct AnimatedSprite {
    pub animation: Animation, // For now, own the animation data directly to simplify
    pub current_frame_index: usize,
    pub timer: f32,
    pub playing: bool,
    pub speed: f32,
    pub loop_count: usize,

    // Transform properties (similar to Sprite)
    pub transform: Transform2D,
    pub tint: [f32; 4],
    pub is_occluder: bool,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl AnimatedSprite {
    pub fn new(animation: Animation) -> Self {
        Self {
            animation,
            current_frame_index: 0,
            timer: 0.0,
            playing: true,
            speed: 1.0,
            loop_count: 0,
            transform: Transform2D::default(),
            tint: [1.0, 1.0, 1.0, 1.0],
            is_occluder: true,
            flip_x: false,
            flip_y: false,
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.playing || self.animation.frames.is_empty() {
            return;
        }

        self.timer += dt * self.speed;

        let frame = &self.animation.frames[self.current_frame_index];
        if self.timer >= frame.duration {
            self.timer -= frame.duration;
            self.current_frame_index += 1;

            if self.current_frame_index >= self.animation.frames.len() {
                if self.animation.looping {
                    self.current_frame_index = 0;
                    self.loop_count += 1;
                } else {
                    self.current_frame_index = self.animation.frames.len() - 1;
                    self.playing = false;
                }
            }
        }
    }

    pub fn current_frame(&self) -> Option<&AnimationFrame> {
        self.animation.frames.get(self.current_frame_index)
    }

    /// Reset animation to start.
    pub fn reset(&mut self) {
        self.current_frame_index = 0;
        self.timer = 0.0;
        self.playing = true;
    }
}
