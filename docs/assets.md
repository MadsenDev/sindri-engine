# Asset Management

Forge2D provides an `AssetManager` for caching loaded assets, preventing duplicate loads and improving performance.

## Using AssetManager

The `AssetManager` is accessible through `EngineContext`:

```rust
// Load texture (cached by path/ID)
let texture = ctx.load_texture("assets/sprite.png")?;

// Load from bytes (cached by ID)
let texture2 = ctx.load_texture_from_bytes("my_texture", png_bytes)?;

// Get cached texture
if let Some(texture) = ctx.assets().get_texture("my_texture") {
    // Use texture
}
```

## Texture Loading

### From File

```rust
let texture = ctx.load_texture("assets/sprite.png")?;
```

The texture is cached by the file path. Loading the same path again returns the cached handle.

### From Bytes

```rust
let texture = ctx.load_texture_from_bytes("my_texture", png_bytes)?;
```

The texture is cached by the ID string you provide. Use the same ID to retrieve the cached texture.

### Direct Renderer Access

You can also load textures directly from the renderer (not cached):

```rust
let texture = ctx.renderer().load_texture_from_file("assets/sprite.png")?;
let texture2 = ctx.renderer().load_texture_from_bytes(png_bytes)?;
```

## Asset Caching

### How Caching Works

- Textures are cached by their path or ID
- Loading the same asset multiple times returns the same handle
- This prevents duplicate loads and saves memory

### Example

```rust
// First load - loads from disk
let texture1 = ctx.load_texture("assets/sprite.png")?;

// Second load - returns cached handle
let texture2 = ctx.load_texture("assets/sprite.png")?;

assert_eq!(texture1, texture2);  // Same handle = cached!
```

## Embedded Assets

You can embed assets in your binary using `include_bytes!`:

```rust
const SPRITE_PNG: &[u8] = include_bytes!("assets/sprite.png");

fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let texture = ctx.load_texture_from_bytes("sprite", SPRITE_PNG)?;
    Ok(())
}
```

## Getting Cached Assets

```rust
// Check if texture exists
if let Some(texture) = ctx.assets().get_texture("my_texture") {
    // Use texture
}

// Direct access (returns Option)
let texture = ctx.assets().get_texture("my_texture");
```

## AssetManager API

### Texture Methods

- **`get_texture(id: &str) -> Option<TextureHandle>`** - Get cached texture by ID
- **`load_texture(renderer: &mut Renderer, path: &str) -> Result<TextureHandle>`** - Load texture from file (cached)
- **`load_texture_from_bytes(renderer: &mut Renderer, id: &str, bytes: &[u8]) -> Result<TextureHandle>`** - Load texture from bytes (cached)

### Font Methods

Fonts are also managed through `AssetManager` and cached by string keys:

```rust
use forge2d::FontHandle;

const FONT_BYTES: &[u8] = include_bytes!("assets/font.ttf");

fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
    // Load font via asset manager (cached by key)
    let font: FontHandle = ctx.load_font_from_bytes("ui_font", FONT_BYTES)?;

    // Later, you can retrieve it by key:
    if let Some(cached_font) = ctx.get_font("ui_font") {
        assert_eq!(font, cached_font);
    }

    Ok(())
}
```

Underlying behavior:

- `AssetManager::load_font_from_bytes(renderer, key, bytes)` – loads and caches the font
- `AssetManager::load_font_from_file(renderer, path)` – loads and caches the font by path
- `AssetManager::get_font(key)` – retrieves a cached `FontHandle` if available

### Convenience Methods (via EngineContext)

- **`ctx.load_texture(path: &str) -> Result<TextureHandle>`** - Load texture (cached)
- **`ctx.load_texture_from_bytes(id: &str, bytes: &[u8]) -> Result<TextureHandle>`** - Load texture from bytes (cached)
- **`ctx.load_font(path: &str) -> Result<FontHandle>`** - Load font from file (cached)
- **`ctx.load_font_from_bytes(id: &str, bytes: &[u8]) -> Result<FontHandle>`** - Load font from bytes (cached)
- **`ctx.get_texture(id: &str) -> Option<TextureHandle>`** - Get cached texture by ID
- **`ctx.get_font(id: &str) -> Option<FontHandle>`** - Get cached font by ID
- **`ctx.assets() -> &mut AssetManager`** - Access asset manager directly

### Asset Roots

If you set `Engine::with_asset_root("assets")`, relative paths passed to
`ctx.load_texture()` and `ctx.load_font()` will be resolved against that root.

## Best Practices

1. **Use AssetManager** - Always use `ctx.load_texture()` instead of direct renderer access for automatic caching
2. **Consistent IDs** - Use consistent ID strings for embedded assets
3. **Load in init()** - Load all assets in `init()` to avoid frame-time loading
4. **Embed Small Assets** - Consider embedding small assets (like UI sprites) in your binary

## Future Extensions

The `AssetManager` is designed to be extended for other asset types:
- Fonts
- Audio files
- Shaders
- Data files
