use anyhow::Result;

use crate::assets::AssetManager;
use crate::render::{FontHandle, Renderer};

/// Built-in font identifiers for Sindri.
///
/// These map to engine-provided fonts once you wire them up in this module.
/// Each variant corresponds to a string key in the `AssetManager`.
#[derive(Clone, Copy, Debug)]
pub enum BuiltinFont {
    /// Default UI / HUD font.
    Ui,
    /// Monospace font for debug text / consoles.
    Mono,
    /// Larger display font for titles/headings.
    Title,
}

impl BuiltinFont {
    /// String key used for this font in the `AssetManager`.
    pub fn key(self) -> &'static str {
        match self {
            BuiltinFont::Ui => "builtin_ui",
            BuiltinFont::Mono => "builtin_mono",
            BuiltinFont::Title => "builtin_title",
        }
    }

    /// Load this built-in font via the asset system.
    ///
    /// By default, this is a stub that returns an error until you add actual
    /// font files and wire them up. To enable built-in fonts:
    ///
    /// 1. Add TTF/OTF files under `sindri/fonts/` (e.g. `ui.ttf`, `mono.ttf`, `title.ttf`)
    /// 2. Replace the match arms below with `include_bytes!` calls, e.g.:
    ///
    /// ```ignore
    /// BuiltinFont::Ui => {
    ///     const BYTES: &[u8] = include_bytes!("../fonts/ui.ttf");
    ///     assets.load_font_from_bytes(renderer, self.key(), BYTES)
    /// }
    /// ```
    pub fn load(self, assets: &mut AssetManager, renderer: &mut Renderer) -> Result<FontHandle> {
        match self {
            // UI font: use Inter Regular (good general-purpose UI font)
            BuiltinFont::Ui => {
                const BYTES: &[u8] =
                    include_bytes!("../fonts/Inter-4.1/extras/ttf/Inter-Regular.ttf");
                assets.load_font_from_bytes(renderer, self.key(), BYTES)
            }
            // Monospace / debug font: VT323 (retro monospace)
            BuiltinFont::Mono => {
                const BYTES: &[u8] = include_bytes!("../fonts/VT323/VT323-Regular.ttf");
                assets.load_font_from_bytes(renderer, self.key(), BYTES)
            }
            // Title font: Bangers (comic-style display font)
            BuiltinFont::Title => {
                const BYTES: &[u8] = include_bytes!("../fonts/Bangers/Bangers-Regular.ttf");
                assets.load_font_from_bytes(renderer, self.key(), BYTES)
            }
        }
    }
}
