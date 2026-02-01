use std::{collections::HashMap, fs};

use crate::render::{FontHandle, Renderer, TextureHandle};

/// Manages cached assets (textures, fonts, and future: sounds, etc.).
pub struct AssetManager {
    textures: HashMap<String, TextureHandle>,
    fonts: HashMap<String, FontHandle>,
}

impl AssetManager {
    /// Create a new asset manager with no cached assets.
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
            fonts: HashMap::new(),
        }
    }

    /// Load a texture from a file path, caching it if already loaded.
    ///
    /// Returns the texture handle. If the texture was previously loaded,
    /// returns the cached handle without reloading from disk.
    pub fn load_texture(
        &mut self,
        renderer: &mut Renderer,
        path: &str,
    ) -> anyhow::Result<TextureHandle> {
        // Check cache first
        if let Some(handle) = self.textures.get(path) {
            return Ok(*handle);
        }

        // Load and cache
        let handle = renderer.load_texture_from_file(path)?;
        self.textures.insert(path.to_string(), handle);
        Ok(handle)
    }

    /// Load a texture from bytes, caching it by a given key.
    ///
    /// Useful for embedded assets or dynamically generated textures.
    pub fn load_texture_from_bytes(
        &mut self,
        renderer: &mut Renderer,
        key: &str,
        bytes: &[u8],
    ) -> anyhow::Result<TextureHandle> {
        // Check cache first
        if let Some(handle) = self.textures.get(key) {
            return Ok(*handle);
        }

        // Load and cache
        let handle = renderer.load_texture_from_bytes(bytes)?;
        self.textures.insert(key.to_string(), handle);
        Ok(handle)
    }

    /// Load a font from bytes (TTF/OTF), caching it by a given key.
    ///
    /// Fonts are loaded via the renderer's font API and cached as `FontHandle`s.
    pub fn load_font_from_bytes(
        &mut self,
        renderer: &mut Renderer,
        key: &str,
        bytes: &[u8],
    ) -> anyhow::Result<FontHandle> {
        // Check cache first
        if let Some(handle) = self.fonts.get(key) {
            return Ok(*handle);
        }

        // Load and cache
        let handle = renderer.load_font_from_bytes(bytes)?;
        self.fonts.insert(key.to_string(), handle);
        Ok(handle)
    }

    /// Load a font from a file path, caching it by the path.
    pub fn load_font_from_file(
        &mut self,
        renderer: &mut Renderer,
        path: &str,
    ) -> anyhow::Result<FontHandle> {
        if let Some(handle) = self.fonts.get(path) {
            return Ok(*handle);
        }

        let bytes = fs::read(path)?;
        let handle = renderer.load_font_from_bytes(&bytes)?;
        self.fonts.insert(path.to_string(), handle);
        Ok(handle)
    }

    /// Get a cached texture handle by key, if it exists.
    pub fn get_texture(&self, key: &str) -> Option<TextureHandle> {
        self.textures.get(key).copied()
    }

    /// Get a cached font handle by key, if it exists.
    pub fn get_font(&self, key: &str) -> Option<FontHandle> {
        self.fonts.get(key).copied()
    }

    /// Check if a texture is already cached.
    pub fn has_texture(&self, key: &str) -> bool {
        self.textures.contains_key(key)
    }

    /// Check if a font is already cached.
    pub fn has_font(&self, key: &str) -> bool {
        self.fonts.contains_key(key)
    }

    /// Clear all cached textures (they will be reloaded on next access).
    pub fn clear(&mut self) {
        self.textures.clear();
        self.fonts.clear();
    }

    /// Remove a specific texture from the cache.
    pub fn unload_texture(&mut self, key: &str) {
        self.textures.remove(key);
    }

    /// Remove a specific font from the cache.
    pub fn unload_font(&mut self, key: &str) {
        self.fonts.remove(key);
    }
}

impl Default for AssetManager {
    fn default() -> Self {
        Self::new()
    }
}
