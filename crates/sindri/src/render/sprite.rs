use crate::math::{Transform2D, Vec2};

/// Opaque handle used to reference textures owned by the renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub(crate) u32);

impl TextureHandle {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Simple sprite combining a texture and transform metadata.
#[derive(Clone, Debug)]
pub struct Sprite {
    pub texture: TextureHandle,
    pub transform: Transform2D,
    /// Multiplicative tint applied to the sampled texture color.
    pub tint: [f32; 4],
    /// Whether this sprite casts shadows (occludes light).
    pub is_occluder: bool,
}

impl Sprite {
    pub fn new(texture: TextureHandle) -> Self {
        Self {
            texture,
            transform: Transform2D::default(),
            tint: [1.0, 1.0, 1.0, 1.0],
            is_occluder: true, // Default to casting shadows
        }
    }

    /// Set the sprite size in pixels, given the texture's pixel dimensions.
    ///
    /// This is a convenience method that converts pixel sizes to scale multipliers.
    /// The scale is calculated as `size_px / texture_px`.
    ///
    /// Example:
    /// ```rust,no_run
    /// # use sindri::{Sprite, Vec2};
    /// # // In real code, you'd get TextureHandle from Renderer::load_texture
    /// # // For this example, we'll assume you have a texture handle
    /// # fn example(texture: sindri::TextureHandle) {
    /// # let mut sprite = Sprite::new(texture);
    /// // For a 32x32 texture, set sprite to 48x48 pixels
    /// sprite.set_size_px(Vec2::new(48.0, 48.0), Vec2::new(32.0, 32.0));
    /// // sprite.transform.scale is now (1.5, 1.5)
    /// # }
    /// ```
    pub fn set_size_px(&mut self, size_px: Vec2, texture_px: Vec2) {
        self.transform.scale = Vec2::new(size_px.x / texture_px.x, size_px.y / texture_px.y);
    }
}
