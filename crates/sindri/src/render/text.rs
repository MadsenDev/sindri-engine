use anyhow::Result;
use glyphon::{
    Cache, FontSystem, SwashCache, TextAtlas, TextRenderer as GlyphonTextRenderer, Viewport,
};
use std::collections::HashMap;

/// A font loaded and ready for text rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontHandle(pub(crate) u32);

/// Text renderer that manages fonts and glyph caching using glyphon.
pub struct TextRenderer {
    font_system: FontSystem,
    cache: SwashCache,
    text_atlas: Option<TextAtlas>,
    text_renderer: Option<GlyphonTextRenderer>,
    viewport: Option<Viewport>,
    gpu_cache: Option<Cache>, // GPU resource cache (different from SwashCache)
    fonts: HashMap<FontHandle, Vec<u8>>, // Store font bytes for glyphon
    next_font_id: u32,
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
            cache: SwashCache::new(),
            text_atlas: None,
            text_renderer: None,
            viewport: None,
            gpu_cache: None,
            fonts: HashMap::new(),
            next_font_id: 1,
        }
    }

    /// Load a font from bytes (TTF/OTF format).
    pub fn load_font_from_bytes(&mut self, bytes: &[u8]) -> Result<FontHandle> {
        // Store font bytes for glyphon
        let font_bytes = bytes.to_vec();
        let handle = FontHandle(self.next_font_id);
        self.next_font_id += 1;
        self.fonts.insert(handle, font_bytes.clone());

        // Add font to font system
        self.font_system.db_mut().load_font_data(font_bytes);

        Ok(handle)
    }

    pub(crate) fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub(crate) fn text_atlas_mut(&mut self) -> &mut Option<TextAtlas> {
        &mut self.text_atlas
    }

    pub(crate) fn text_renderer_mut(&mut self) -> &mut Option<GlyphonTextRenderer> {
        &mut self.text_renderer
    }

    pub(crate) fn viewport_mut(&mut self) -> &mut Option<Viewport> {
        &mut self.viewport
    }

    pub(crate) fn gpu_cache_mut(&mut self) -> &mut Option<Cache> {
        &mut self.gpu_cache
    }

    /// Get all mutable references needed for text rendering
    /// This avoids multiple borrows of self
    pub(crate) fn get_rendering_refs(
        &mut self,
    ) -> Option<(
        &mut TextAtlas,
        &mut GlyphonTextRenderer,
        &mut Viewport,
        &mut FontSystem,
        &mut SwashCache,
    )> {
        let text_atlas = self.text_atlas.as_mut()?;
        let text_renderer = self.text_renderer.as_mut()?;
        let viewport = self.viewport.as_mut()?;
        let font_system = &mut self.font_system;
        let cache = &mut self.cache;
        Some((text_atlas, text_renderer, viewport, font_system, cache))
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}
