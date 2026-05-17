# Sindri

Sindri is a lightweight 2D game framework built with Rust, winit, and wgpu. It provides a clean, simple API for creating 2D games.

## Quick Start

### Running the Example

From the repository root:

```bash
cargo run -p basic_game
```

This launches a window with a bouncing sprite. Press ESC or wait 10 seconds to exit.

### Creating Your Own Game

1. **Add Sindri to your `Cargo.toml`:**

```toml
[dependencies]
sindri = { path = "../sindri" }  # or use git/crates.io when published
anyhow = "1"
```

2. **Create a game struct and implement the `Game` trait:**

```rust
use anyhow::Result;
use sindri::{Engine, EngineContext, Game, KeyCode, Vec2};

struct MyGame {
    // Your game state here
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called once at startup - load assets, initialize state, etc.
        println!("Game initialized!");
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called every frame - update game logic here
        let dt = ctx.delta_time().as_secs_f32();
        
        // Check input
        if ctx.input().is_key_pressed(KeyCode::Space) {
            println!("Space pressed!");
        }
        
        // Access mouse position
        let (mx, my) = ctx.input().mouse_position();
        
        // Request exit
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }
        
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called every frame - render your game here
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        
        // Clear the screen (RGBA values 0.0-1.0)
        renderer.clear(&mut frame, [0.1, 0.1, 0.2, 1.0])?;
        
        // Draw sprites, shapes, etc.
        // renderer.draw_sprite(&mut frame, &sprite, &camera)?;
        
        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("My Game")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(MyGame {})
}
```

## Core Concepts

### The `Game` Trait

Your game must implement three methods:

- **`init()`** - Called once at startup. Load textures, initialize state, etc.
- **`update()`** - Called every frame before drawing. Handle input, update game logic.
- **`draw()`** - Called every frame. Render your game.

### The `EngineContext`

Provides access to engine systems:

- **`ctx.delta_time()`** - Time since last frame (`Duration`)
- **`ctx.elapsed_time()`** - Total time since engine started (`Duration`)
- **`ctx.input()`** - Access input state (keys, mouse)
- **`ctx.renderer()`** - Access the renderer for drawing
- **`ctx.window()`** - Access the underlying winit window
- **`ctx.request_exit()`** - Request the engine to exit

### Input System

```rust
use sindri::{KeyCode, MouseButton};

// Check if key is currently held down
if ctx.input().is_key_down(KeyCode::KeyW) {
    // Move forward
}

// Check if key was just pressed this frame
if ctx.input().is_key_pressed(KeyCode::Space) {
    // Jump
}

// Check if key was just released this frame
if ctx.input().is_key_released(KeyCode::Escape) {
    // Pause menu
}

// Mouse position
let (x, y) = ctx.input().mouse_position();
let mouse_pos = ctx.input().mouse_position_vec2();  // As Vec2

// Mouse buttons
if ctx.input().is_mouse_pressed(MouseButton::Left) {
    // Clicked!
}
```

### Rendering

```rust
// Begin a frame
let mut frame = renderer.begin_frame()?;

// Clear the screen (RGBA: 0.0-1.0)
renderer.clear(&mut frame, [0.1, 0.1, 0.2, 1.0])?;

// Load a texture
let texture = renderer.load_texture_from_file("assets/sprite.png")?;
// Or from bytes:
let texture = renderer.load_texture_from_bytes(png_bytes)?;

// Create a sprite
let mut sprite = Sprite::new(texture);
sprite.transform.position = Vec2::new(100.0, 200.0);
sprite.transform.scale = Vec2::new(64.0, 64.0);
sprite.tint = [1.0, 1.0, 1.0, 1.0];  // RGBA tint

// Draw the sprite (requires a camera)
let camera = Camera2D::default();
renderer.draw_sprite(&mut frame, &sprite, &camera)?;

// End the frame
renderer.end_frame(frame)?;
```

### Math Types

```rust
use sindri::{Vec2, Transform2D, Camera2D};

// Vec2 - 2D vector
let position = Vec2::new(100.0, 200.0);
let velocity = Vec2::new(50.0, -30.0);
let new_pos = position + velocity * dt;

// Useful Vec2 methods:
let distance = pos1.distance(pos2);
let direction = (target - position).normalized();
let interpolated = start.lerp(end, 0.5);  // 50% between start and end
let angle_vec = Vec2::from_angle(std::f32::consts::PI / 4.0);

// Transform2D - position, scale, rotation
let transform = Transform2D {
    position: Vec2::new(100.0, 200.0),
    scale: Vec2::new(1.0, 1.0),
    rotation: 0.0,  // radians
};

// Camera2D - 2D camera
let mut camera = Camera2D::new(Vec2::new(0.0, 0.0));
camera.zoom = 1.5;
camera.position = player_position;

// Convert between screen and world coordinates
let world_pos = camera.screen_to_world(screen_pos, width, height);
let screen_pos = camera.world_to_screen(world_pos, width, height);
```

### Engine Configuration

```rust
Engine::new()
    .with_title("My Game")           // Window title
    .with_size(1280, 720)            // Window size (logical pixels)
    .with_vsync(true)                 // Enable/disable VSync
    .run(my_game)
```

## Project Layout

- `sindri/`: The engine crate containing the public API
- `examples/basic_game/`: A complete example showing sprite rendering

## Documentation

📚 **Comprehensive documentation is available in the [`docs/`](docs/) directory:**

- [Getting Started](docs/getting-started.md) - Quick start guide
- [Engine & Game Loop](docs/engine.md) - Engine configuration
- [Input System](docs/input.md) - Keyboard and mouse input
- [Rendering](docs/rendering.md) - Sprites, textures, cameras, text
- [Math Utilities](docs/math.md) - Vec2, Transform2D, Camera2D
- [Asset Management](docs/assets.md) - Loading and caching assets
- [Audio System](docs/audio.md) - Sound effects and music
- [Fixed Timestep](docs/fixed-timestep.md) - Deterministic updates
- [API Reference](docs/api-reference.md) - Complete API docs
- [Examples](docs/examples.md) - Code examples and patterns

## Current Features

✅ Window creation and event loop  
✅ Input system (keyboard & mouse)  
✅ 2D rendering with wgpu  
✅ Sprite rendering with textures (batched)  
✅ Text rendering with TTF/OTF fonts  
✅ Camera system  
✅ Math utilities (Vec2, Transform2D)  
✅ Asset manager for texture caching  
✅ Audio system  
✅ Fixed timestep support  

## Coming Soon

- State/scene management
- Optional ECS support

## Requirements

- Rust 2021 edition
- A GPU with graphics drivers installed (wgpu requirement)

## License

MIT OR Apache-2.0
