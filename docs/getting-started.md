# Getting Started

This guide will help you create your first Sindri game.

## Installation

Add Sindri to your `Cargo.toml`:

```toml
[dependencies]
sindri = { path = "../sindri" }  # or use git/crates.io when published
anyhow = "1"
```

## Your First Game

Here's a minimal game that opens a window and clears the screen:

```rust
use anyhow::Result;
use sindri::{Engine, EngineContext, Game, KeyCode};

struct MyGame;

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        println!("Game initialized!");
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Exit on ESC
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        
        // Clear with dark blue
        renderer.clear(&mut frame, [0.1, 0.1, 0.2, 1.0])?;
        
        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("My First Game")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(MyGame)
}
```

## Core Concepts

### The Game Trait

Your game must implement the `Game` trait with three methods:

- **`init()`** - Called once at startup. Load assets, initialize state.
- **`update()`** - Called every frame. Handle input, update game logic.
- **`draw()`** - Called every frame. Render your game.

### Engine Configuration

Configure the engine using builder methods:

```rust
Engine::new()
    .with_title("My Game")      // Window title
    .with_size(1280, 720)       // Window size
    .with_vsync(true)            // Enable VSync
    .run(my_game)
```

### EngineContext

The `EngineContext` provides access to all engine systems:

```rust
// Time
let dt = ctx.delta_time();
let elapsed = ctx.elapsed_time();

// Input
let input = ctx.input();
if input.is_key_pressed(KeyCode::Space) { /* ... */ }

// Rendering
let renderer = ctx.renderer();
let mut frame = renderer.begin_frame()?;
// ... draw calls ...
renderer.end_frame(frame)?;

// Assets
let texture = ctx.load_texture("assets/sprite.png")?;

// Audio
if ctx.audio().is_available() {
    ctx.audio().play_sound_from_bytes(sound_bytes)?;
}
```

## Next Steps

- Learn about [Input Handling](input.md)
- Explore [Rendering](rendering.md)
- Check out [Examples](examples.md)
- Read about [What Sindri Does and Doesn't Do](../README.md#what-sindri-intentionally-doesnt-do)
- Understand the [Development Philosophy](../README.md#philosophy-coherence-over-completeness)

## Important Notes

**Sindri is not a complete engine**—it's a framework with core systems that you extend. Before starting, understand:

- ✅ What Sindri **does** provide (rendering, physics, pathfinding, etc.)
- ❌ What Sindri **intentionally doesn't** provide (animation, advanced UI, debug tools, etc.)
- ⚠️ What you'll need to **implement yourself** or use libraries for

See the [README](../README.md) for a complete list of constraints and guarantees.

**Recommended approach:** Pick a reference game type (platformer, top-down, etc.) and build it. Add only what that game forces you to add. This keeps development focused and prevents feature bloat.
