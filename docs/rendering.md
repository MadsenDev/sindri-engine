# Rendering

Forge2D provides hardware-accelerated 2D rendering using wgpu.

## Basic Rendering Flow

Every frame, you'll follow this pattern:

```rust
fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let renderer = ctx.renderer();
    let mut frame = renderer.begin_frame()?;
    
    // Clear the screen (RGBA: 0.0-1.0)
    renderer.clear(&mut frame, [0.1, 0.1, 0.2, 1.0])?;
    
    // Draw sprites, text, etc.
    
    renderer.end_frame(frame)?;
    Ok(())
}
```

## Loading Textures

### From File

```rust
let texture = renderer.load_texture_from_file("assets/sprite.png")?;
```

### From Bytes

```rust
let texture = renderer.load_texture_from_bytes(png_bytes)?;
```

### Using AssetManager (Recommended)

```rust
// Load texture (cached, prevents duplicate loads)
let texture = ctx.load_texture("assets/sprite.png")?;
let texture2 = ctx.load_texture_from_bytes("my_texture", png_bytes)?;

// Get cached texture
if let Some(texture) = ctx.assets().get_texture("my_texture") {
    // Use texture
}
```

## Sprite Rendering

### Creating Sprites

```rust
use forge2d::{Sprite, Vec2};

// Create a sprite
let mut sprite = Sprite::new(texture);
sprite.transform.position = Vec2::new(100.0, 200.0);
sprite.transform.scale = Vec2::new(1.0, 1.0);  // Scale multiplier
sprite.transform.rotation = 0.0;  // Radians
sprite.tint = [1.0, 1.0, 1.0, 1.0];  // RGBA tint (white)
```

### Setting Sprite Size

**Important:** `Transform2D.scale` is a **multiplier** relative to the base texture size.

```rust
// Method 1: Set scale directly (multiplier)
sprite.transform.scale = Vec2::new(2.0, 2.0);  // 2x the texture size

// Method 2: Set size in pixels (helper method)
sprite.set_size_px(
    Vec2::new(64.0, 64.0),      // Desired size in pixels
    Vec2::new(32.0, 32.0),      // Original texture size
);
```

### Sprite Position

**Important:** `Transform2D.position` represents the **center** of the sprite.

```rust
// Position is the CENTER of the sprite
sprite.transform.position = Vec2::new(100.0, 200.0);

// When clamping to bounds, account for half the sprite size:
let half_size = sprite_size * 0.5;
sprite.transform.position.x = sprite.transform.position.x.clamp(
    half_size,
    world_bounds.x - half_size,
);
```

### Drawing Sprites

```rust
use forge2d::Camera2D;

let camera = Camera2D::default();
renderer.draw_sprite(&mut frame, &sprite, &camera)?;
```

### Sprite Properties

- **`texture: TextureHandle`** - The texture to render
- **`transform: Transform2D`** - Position (center), scale (multiplier), rotation (radians)
- **`tint: [f32; 4]`** - RGBA color tint (default: `[1.0, 1.0, 1.0, 1.0]`)
- **`is_occluder: bool`** - Whether the sprite casts shadows (default: `true`)

## Camera System

### Creating a Camera

```rust
use forge2d::Camera2D;

// Create camera at position
let mut camera = Camera2D::new(Vec2::new(0.0, 0.0));

// Or use default
let camera = Camera2D::default();
```

### Camera Properties

```rust
camera.position = Vec2::new(100.0, 200.0);  // Camera position
camera.zoom = 1.0;  // Zoom level (1.0 = 1:1 pixel-to-world ratio)
```

### Coordinate Conversion

```rust
let (screen_w, screen_h) = renderer.surface_size();

// Screen to world
let world_pos = camera.screen_to_world(screen_pos, screen_w, screen_h);

// World to screen
let screen_pos = camera.world_to_screen(world_pos, screen_w, screen_h);

// Or use the helper method
let mouse_world = ctx.mouse_world(&camera);
```

### Camera Following

```rust
// Smooth camera following
let target_pos = player.position - Vec2::new(screen_w as f32 * 0.5, screen_h as f32 * 0.5);
let camera_speed = 5.0;
camera.position = camera.position.lerp(target_pos, camera_speed * dt);
```

## Text Rendering

### Loading Fonts

```rust
use forge2d::{EngineContext, FontHandle};

// Load a font (TTF/OTF format) via AssetManager (preferred)
const FONT_BYTES: &[u8] = include_bytes!("assets/font.ttf");

fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let font: FontHandle = ctx.load_font_from_bytes("ui_font", FONT_BYTES)?;
    Ok(())
}
```

### Pre-rasterizing Glyphs

Pre-rasterizing is optional; the default text backend caches glyphs automatically.

```rust
// Rasterize all glyphs needed for a text string
renderer.rasterize_text_glyphs("Hello World", font, 24.0)?;
```

### Drawing Text

```rust
renderer.draw_text(
    &mut frame,
    "Hello World",
    font,
    24.0,                              // Font size in pixels
    Vec2::new(100.0, 200.0),          // World position (bottom-left of text)
    [1.0, 1.0, 1.0, 1.0],            // RGBA color
    &camera,
)?;
```

### Text Rendering Notes

- Glyphs are cached automatically - pre-rasterize only if you want to warm the cache
- Position is the bottom-left corner of the first character
- Text is rendered as sprites (one sprite per glyph)

## HUD Layer

Forge2D provides a HUD (Heads-Up Display) layer for screen-space UI elements that stay fixed on screen regardless of camera position.

### Creating a HUD

```rust
use forge2d::{HudLayer, HudText};

struct MyGame {
    hud: HudLayer,
    font: Option<FontHandle>,
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.font = Some(ctx.builtin_font(forge2d::BuiltinFont::Ui)?);
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;
        
        // Draw game sprites...
        
        // Draw HUD (screen-space, top-left is 0,0)
        self.hud.clear();
        if let Some(font) = self.font {
            self.hud.add_text(HudText {
                text: "Score: 100".to_string(),
                font,
                size: 20.0,
                position: Vec2::new(10.0, 10.0), // Top-left corner
                color: [1.0, 1.0, 1.0, 1.0],
            });
        }
        self.hud.draw(renderer, &mut frame)?;
        
        renderer.end_frame(frame)?;
        Ok(())
    }
}
```

### HUD Elements

- **`HudText`** - Text rendered in screen-space (position in pixels from top-left)
- **`HudSprite`** - Sprites rendered in screen-space
- **`HudRect`** - Rectangles for panels/bars

### HUD Coordinate System

HUD positions use screen-space coordinates where `(0, 0)` is the top-left corner of the screen. Positions are in pixels.

## Performance Notes

### Batched Rendering

Forge2D automatically batches all sprites into a single render pass per frame for optimal performance. You don't need to do anything special - just call `draw_sprite()` for each sprite.

### Sprite Limit

The renderer supports up to **2048 sprites per frame** (increased from 256). If you need to draw more sprites, consider:
- Using viewport culling to only draw visible sprites
- Using texture atlasing to combine multiple sprites
- Implementing instanced rendering for repeated sprites

### Glyph Caching

Text glyphs are cached per (font, character, size) combination. Re-rasterize only when:
- The text string changes
- You're using a new character
- You're using a new font size

### Texture Caching

Use `AssetManager` (via `ctx.load_texture()`) to prevent duplicate texture loads. The same texture loaded multiple times will return the same handle.

## Common Patterns

### Drawing Multiple Sprites

```rust
for sprite in &sprites {
    renderer.draw_sprite(&mut frame, sprite, &camera)?;
}
```

### Screen-Space UI Text

```rust
// Position text relative to camera for screen-space effect
let (screen_w, screen_h) = renderer.surface_size();
let text_pos = Vec2::new(
    camera.position.x - (screen_w as f32 * 0.5) + 20.0,
    camera.position.y + (screen_h as f32 * 0.5) - 40.0,
);
renderer.draw_text(&mut frame, "Score: 100", font, 24.0, text_pos, [1.0, 1.0, 1.0, 1.0], &camera)?;
```

### Sprite Rotation

```rust
sprite.transform.rotation += rotation_speed * dt;
```

### Sprite Tinting

```rust
// Red tint
sprite.tint = [1.0, 0.0, 0.0, 1.0];

// Semi-transparent
sprite.tint = [1.0, 1.0, 1.0, 0.5];
```

## Vector Shape Rendering

Forge2D provides GPU-accelerated vector shape drawing for lines, circles, and polygons. These are useful for procedural graphics, debug visualization, and games that work well with geometric shapes.

### Drawing Lines

```rust
renderer.draw_line(
    &mut frame,
    Vec2::new(0.0, 0.0),        // Start position
    Vec2::new(100.0, 100.0),    // End position
    2.0,                        // Line width in pixels
    [1.0, 1.0, 1.0, 1.0],      // RGBA color
    &camera,
)?;
```

### Drawing Circles

```rust
renderer.draw_circle(
    &mut frame,
    Vec2::new(200.0, 200.0),   // Center position
    50.0,                       // Radius in pixels
    [1.0, 0.0, 0.0, 1.0],      // RGBA color (red)
    &camera,
)?;
```

### Drawing Polygons

```rust
// Create a triangle
let triangle_points = vec![
    Vec2::new(0.0, -20.0),     // Top point
    Vec2::new(-20.0, 20.0),    // Bottom left
    Vec2::new(20.0, 20.0),     // Bottom right
];

renderer.draw_polygon(
    &mut frame,
    &triangle_points,
    [1.0, 1.0, 1.0, 1.0],      // RGBA color
    &camera,
)?;
```

### Vector Shape Notes

- **Polygon Triangulation**: Polygons are rendered using Ear Clipping triangulation. This means:
  - **Convex and Concave polygons work correctly**
  - **Point Order**: Points should be provided in counter-clockwise order around the shape

- **Performance**: Vector shapes are GPU-accelerated and batched automatically
- **Coordinate System**: All positions are in world coordinates (affected by camera)
- **Color**: RGBA values range from 0.0 to 1.0

### Example: Drawing a Ship

```rust
fn draw_ship(&self, renderer: &mut Renderer, frame: &mut Frame, 
             position: Vec2, rotation: f32, camera: &Camera2D) -> Result<()> {
    // Create triangle pointing up
    let size = 20.0;
    let mut points = vec![
        Vec2::new(0.0, -size),           // Front point
        Vec2::new(-size * 0.7, size),    // Back left
        Vec2::new(size * 0.7, size),     // Back right
    ];
    
    // Rotate points
    let cos = rotation.cos();
    let sin = rotation.sin();
    for point in &mut points {
        let rotated = Vec2::new(
            point.x * cos - point.y * sin,
            point.x * sin + point.y * cos,
        );
        *point = position + rotated;
    }
    
    renderer.draw_polygon(frame, &points, [1.0, 1.0, 1.0, 1.0], camera)?;
    Ok(())
}
```

### Example: Drawing Asteroids

```rust
fn draw_asteroid(&self, renderer: &mut Renderer, frame: &mut Frame,
                 center: Vec2, radius: f32, camera: &Camera2D) -> Result<()> {
    // Generate points around a circle with variation
    const POINTS: usize = 10;
    let mut points = Vec::with_capacity(POINTS);
    
    for i in 0..POINTS {
        let angle = (i as f32 / POINTS as f32) * std::f32::consts::TAU;
        // Add some variation (keep it convex: 80-100% of radius)
        let r = radius * (0.8 + (i as f32 * 0.1) % 0.2);
        points.push(center + Vec2::new(r * angle.cos(), r * angle.sin()));
    }
    
    renderer.draw_polygon(frame, &points, [0.8, 0.8, 0.8, 1.0], camera)?;
    Ok(())
}
```


## Lighting System

Forge2D includes a 2D dynamic lighting system supporting point lights, soft shadows, and occlusion.

### Adding Lights

Lights are drawn using `draw_point_light`. They are additive, meaning they brighten the scene.

```rust
use forge2d::PointLight;

// Create a light
let light = PointLight::new(
    Vec2::new(100.0, 100.0), // Position
    [1.0, 0.8, 0.6],         // Color (RGB)
    2.0,                     // Intensity
    300.0,                   // Radius in pixels
);

renderer.draw_point_light(&mut frame, &light, &camera)?;
```

### Occlusion

The lighting system distinguishes between **Occluders** (objects that block light and cast shadows) and **Background** (objects that receive light but do not cast shadows).

-   **Sprites**: By default, sprites are occluders. You can disable this by setting `sprite.is_occluder = false`.
-   **Shapes**: Standard shape drawing methods (`draw_polygon`, `draw_circle`, `draw_rect`) create occluders.
-   **Backgrounds**: Use `draw_polygon_no_occlusion` to draw geometry that should be illuminated but allow light to pass through (e.g., ground tiles, background walls).

```rust
// Draw a floor that receives light but doesn't block it
renderer.draw_polygon_no_occlusion(
    &mut frame, 
    &floor_points, 
    [0.5, 0.5, 0.5, 1.0], 
    &camera
)?;

// Draw a wall that blocks light
renderer.draw_polygon(
    &mut frame, 
    &wall_points, 
    [0.8, 0.8, 0.8, 1.0], 
    &camera
)?;
```
