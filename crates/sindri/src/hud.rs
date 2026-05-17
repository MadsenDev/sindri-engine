use anyhow::Result;

use crate::{
    math::{Camera2D, Vec2},
    render::{FontHandle, Frame, Renderer, Sprite, TextureHandle},
};

/// Text alignment for HUD text elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Text element to be drawn in screen-space HUD coordinates (pixels).
#[derive(Clone)]
pub struct HudText {
    pub text: String,
    pub font: FontHandle,
    pub size: f32,
    pub position: Vec2, // screen-space pixels (0,0 = top-left)
    pub color: [f32; 4],
    pub align: TextAlign, // Text alignment
}

impl Default for HudText {
    fn default() -> Self {
        Self {
            text: String::new(),
            font: FontHandle(0),
            size: 16.0,
            position: Vec2::ZERO,
            color: [1.0, 1.0, 1.0, 1.0],
            align: TextAlign::Left,
        }
    }
}

impl HudText {
    /// Create a new HUD text element with left alignment (default).
    pub fn new(text: String, font: FontHandle, size: f32, position: Vec2, color: [f32; 4]) -> Self {
        Self {
            text,
            font,
            size,
            position,
            color,
            align: TextAlign::Left,
        }
    }

    /// Set text alignment.
    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }
}

/// Sprite element to be drawn in screen-space HUD coordinates (pixels).
pub struct HudSprite {
    pub sprite: Sprite,
    pub position: Vec2, // screen-space pixels (0,0 = top-left)
}

/// Simple rectangle element for panels/bars, drawn using a 1x1 white texture.
pub struct HudRect {
    pub position: Vec2,  // top-left in screen-space pixels
    pub size: Vec2,      // width/height in pixels
    pub color: [f32; 4], // RGBA
}

/// Panel with optional border for more structured UI elements.
pub struct HudPanel {
    pub position: Vec2, // top-left in screen-space pixels
    pub size: Vec2,     // width/height in pixels
    pub background_color: [f32; 4],
    pub border_color: Option<[f32; 4]>,
    pub border_width: f32, // Border width in pixels (0 = no border)
}

impl HudPanel {
    /// Create a new panel without a border.
    pub fn new(position: Vec2, size: Vec2, background_color: [f32; 4]) -> Self {
        Self {
            position,
            size,
            background_color,
            border_color: None,
            border_width: 0.0,
        }
    }

    /// Add a border to the panel.
    pub fn with_border(mut self, border_color: [f32; 4], border_width: f32) -> Self {
        self.border_color = Some(border_color);
        self.border_width = border_width;
        self
    }
}

enum HudElement {
    Text(HudText),
    Sprite(HudSprite),
    Rect(HudRect),
    Panel(HudPanel),
}

/// A layer of HUD elements rendered in screen space on top of the world.
pub struct HudLayer {
    elements: Vec<HudElement>,
    rect_texture: Option<TextureHandle>,
}

impl HudLayer {
    /// Create an empty HUD layer.
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            rect_texture: None,
        }
    }

    /// Remove all HUD elements.
    pub fn clear(&mut self) {
        self.elements.clear();
    }

    /// Add a text element to the HUD.
    pub fn add_text(&mut self, text: HudText) {
        self.elements.push(HudElement::Text(text));
    }

    /// Add a sprite element to the HUD.
    pub fn add_sprite(&mut self, sprite: HudSprite) {
        self.elements.push(HudElement::Sprite(sprite));
    }

    /// Add a rectangle element to the HUD.
    pub fn add_rect(&mut self, rect: HudRect) {
        self.elements.push(HudElement::Rect(rect));
    }

    /// Add a panel element to the HUD (with optional border).
    pub fn add_panel(&mut self, panel: HudPanel) {
        self.elements.push(HudElement::Panel(panel));
    }

    /// Helper: Add a panel with border in one call.
    pub fn add_panel_with_border(
        &mut self,
        position: Vec2,
        size: Vec2,
        background_color: [f32; 4],
        border_color: [f32; 4],
        border_width: f32,
    ) {
        self.add_panel(
            HudPanel::new(position, size, background_color).with_border(border_color, border_width),
        );
    }

    /// Draw all HUD elements in screen space.
    ///
    /// This should typically be called after world rendering, using the same
    /// frame but with a fixed "HUD camera" that maps pixels directly.
    pub fn draw(&mut self, renderer: &mut Renderer, frame: &mut Frame) -> Result<()> {
        // Create HUD camera positioned so world (0,0) maps to screen top-left (0,0)
        // The view_projection centers the camera, so we need to offset by half screen size
        let (screen_w, screen_h) = renderer.surface_size();
        let hud_camera = Camera2D::new(Vec2::new(screen_w as f32 / 2.0, screen_h as f32 / 2.0));

        // Lazily create a 1x1 white texture if we need to draw any rects or panels.
        if self.rect_texture.is_none()
            && self
                .elements
                .iter()
                .any(|e| matches!(e, HudElement::Rect(_) | HudElement::Panel(_)))
        {
            let data = [255u8, 255, 255, 255];
            // Rect texture is not a font, use linear filtering
            let tex = renderer.load_texture_from_rgba(&data, 1, 1)?;
            self.rect_texture = Some(tex);
        }

        for element in &self.elements {
            match element {
                HudElement::Text(ht) => {
                    // Calculate text position based on alignment
                    let text_pos = match ht.align {
                        TextAlign::Left => ht.position,
                        TextAlign::Center => {
                            // Measure actual text width for accurate centering
                            let text_width = renderer
                                .measure_text_width(&ht.text, ht.font, ht.size)
                                .unwrap_or_else(|_| ht.text.len() as f32 * ht.size * 0.6); // Fallback to approximation
                            Vec2::new(ht.position.x - text_width * 0.5, ht.position.y)
                        }
                        TextAlign::Right => {
                            // Measure actual text width for accurate right alignment
                            let text_width = renderer
                                .measure_text_width(&ht.text, ht.font, ht.size)
                                .unwrap_or_else(|_| ht.text.len() as f32 * ht.size * 0.6); // Fallback to approximation
                            Vec2::new(ht.position.x - text_width, ht.position.y)
                        }
                    };

                    renderer.draw_text(
                        frame,
                        &ht.text,
                        ht.font,
                        ht.size,
                        text_pos,
                        ht.color,
                        &hud_camera,
                    )?;
                }
                HudElement::Sprite(hs) => {
                    let mut sprite = hs.sprite.clone();
                    // For HudSprite, the position is treated as top-left
                    // We need to convert to center, but we need the actual rendered size
                    // Since scale is a multiplier, we'd need the base texture size
                    // For now, assume the sprite's scale represents pixel size (common case)
                    // If this doesn't work correctly, users should set position as center
                    sprite.transform.position = hs.position;
                    renderer.draw_sprite(frame, &sprite, &hud_camera)?;
                }
                HudElement::Rect(hr) => {
                    if let Some(tex) = self.rect_texture {
                        let mut sprite = Sprite::new(tex);
                        sprite.tint = hr.color;
                        // Convert top-left to center coordinates
                        sprite.transform.position = Vec2::new(
                            hr.position.x + hr.size.x * 0.5,
                            hr.position.y + hr.size.y * 0.5,
                        );
                        // 1x1 base texture; scale directly to pixel size.
                        sprite.transform.scale = hr.size;
                        renderer.draw_sprite(frame, &sprite, &hud_camera)?;
                    }
                }
                HudElement::Panel(hp) => {
                    if let Some(tex) = self.rect_texture {
                        let bw = hp.border_color.map(|_| hp.border_width).unwrap_or(0.0);

                        // Draw background (shrunk to account for borders)
                        if bw > 0.0 {
                            let bg_size = Vec2::new(hp.size.x - bw * 2.0, hp.size.y - bw * 2.0);
                            let mut bg_sprite = Sprite::new(tex);
                            bg_sprite.tint = hp.background_color;
                            // Convert top-left to center, accounting for border offset
                            bg_sprite.transform.position = Vec2::new(
                                hp.position.x + bw + bg_size.x * 0.5,
                                hp.position.y + bw + bg_size.y * 0.5,
                            );
                            bg_sprite.transform.scale = bg_size;
                            renderer.draw_sprite(frame, &bg_sprite, &hud_camera)?;
                        } else {
                            let mut bg_sprite = Sprite::new(tex);
                            bg_sprite.tint = hp.background_color;
                            // Convert top-left to center
                            bg_sprite.transform.position = Vec2::new(
                                hp.position.x + hp.size.x * 0.5,
                                hp.position.y + hp.size.y * 0.5,
                            );
                            bg_sprite.transform.scale = hp.size;
                            renderer.draw_sprite(frame, &bg_sprite, &hud_camera)?;
                        }

                        // Draw border if specified
                        if let Some(border_color) = hp.border_color {
                            let bw = hp.border_width;
                            if bw > 0.0 {
                                // Top border
                                let mut border = Sprite::new(tex);
                                border.tint = border_color;
                                border.transform.position = Vec2::new(
                                    hp.position.x + hp.size.x * 0.5,
                                    hp.position.y + bw * 0.5,
                                );
                                border.transform.scale = Vec2::new(hp.size.x, bw);
                                renderer.draw_sprite(frame, &border, &hud_camera)?;

                                // Bottom border
                                let mut border = Sprite::new(tex);
                                border.tint = border_color;
                                border.transform.position = Vec2::new(
                                    hp.position.x + hp.size.x * 0.5,
                                    hp.position.y + hp.size.y - bw * 0.5,
                                );
                                border.transform.scale = Vec2::new(hp.size.x, bw);
                                renderer.draw_sprite(frame, &border, &hud_camera)?;

                                // Left border
                                let mut border = Sprite::new(tex);
                                border.tint = border_color;
                                border.transform.position = Vec2::new(
                                    hp.position.x + bw * 0.5,
                                    hp.position.y + hp.size.y * 0.5,
                                );
                                border.transform.scale = Vec2::new(bw, hp.size.y);
                                renderer.draw_sprite(frame, &border, &hud_camera)?;

                                // Right border
                                let mut border = Sprite::new(tex);
                                border.tint = border_color;
                                border.transform.position = Vec2::new(
                                    hp.position.x + hp.size.x - bw * 0.5,
                                    hp.position.y + hp.size.y * 0.5,
                                );
                                border.transform.scale = Vec2::new(bw, hp.size.y);
                                renderer.draw_sprite(frame, &border, &hud_camera)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Layout helper for positioning HUD elements.
pub struct HudLayout {
    pub padding: f32,
    pub spacing: f32,
}

impl HudLayout {
    /// Create a new layout helper with default values.
    pub fn new() -> Self {
        Self {
            padding: 10.0,
            spacing: 5.0,
        }
    }

    /// Set padding (space inside panels).
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Set spacing (space between elements).
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Calculate position for centered text within a panel.
    pub fn center_text_in_panel(
        &self,
        panel_pos: Vec2,
        panel_size: Vec2,
        text_approx_width: f32,
    ) -> Vec2 {
        Vec2::new(
            panel_pos.x + (panel_size.x - text_approx_width) * 0.5,
            panel_pos.y + self.padding,
        )
    }

    /// Calculate position for right-aligned text within a panel.
    pub fn right_align_text_in_panel(
        &self,
        panel_pos: Vec2,
        panel_size: Vec2,
        text_approx_width: f32,
    ) -> Vec2 {
        Vec2::new(
            panel_pos.x + panel_size.x - text_approx_width - self.padding,
            panel_pos.y + self.padding,
        )
    }
}

impl Default for HudLayout {
    fn default() -> Self {
        Self::new()
    }
}
