# Sindri

Sindri is a 2D game engine and editor workspace written in Rust. The engine core uses `wgpu`, `winit`, Rapier2D, `mlua`, `rodio`, and glyphon to provide rendering, physics, Lua scripting, audio, tilemaps, particles, lighting, pathfinding, scene serialization, undo/redo commands, and a lightweight entity/component world.

The repository also contains the new Sindri editor stack: an Axum engine server, a local Ollama AI integration, and a Tauri/React editor prototype.

## Workspace

```text
crates/
  sindri/          Engine core
  sindri-server/   Local engine/editor HTTP server
  sindri-ai/       Ollama client and AI action types
editor/            Tauri 2 + React editor
examples/          Engine demos and reference games
docs/              Engine documentation
```

## Quick Start

Build everything:

```bash
cargo build --workspace
```

Run an example:

```bash
cargo run -p basic_game
```

Run the editor:

```bash
cargo build -p sindri-server
cd editor
pnpm install
pnpm tauri dev
```

The editor talks to `sindri-server` on `127.0.0.1:7878` and uses local Ollama models for AI features. No external AI API is required.

## Minimal Game

```rust
use anyhow::Result;
use sindri::{Camera2D, Engine, EngineContext, Game, KeyCode};

struct MyGame {
    camera: Camera2D,
}

impl Game for MyGame {
    fn init(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if ctx.input().is_key_pressed(KeyCode::Escape) {
            ctx.request_exit();
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        renderer.clear(&mut frame, [0.05, 0.05, 0.08, 1.0])?;
        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("My Sindri Game")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(MyGame {
            camera: Camera2D::default(),
        })
}
```

## Core Features

- Windowing and game loop via `winit`
- Hardware-accelerated 2D rendering via `wgpu`
- Batched sprites, shapes, tilemaps, text, particles, and lighting
- Offscreen rendering and PNG screenshot support for editor workflows
- Camera math, follow behavior, zoom, bounds, and screen/world conversion
- Frame-accurate input and action/axis mapping
- Lightweight `World` / `EntityId` component storage with immutable and mutable queries
- Built-in components for transforms, sprites, physics, audio, cameras, tilemaps, gameplay tags, and moving platforms
- Rapier2D physics with dynamic/kinematic/fixed bodies, sensors, CCD, raycasts, point queries, collision events, layers, and masks
- Lua scripting through `mlua` with lifecycle hooks, input helpers, physics helpers, command buffering, and hot reload
- Scene serialization and physics restore support
- Undo/redo command system
- A* pathfinding and typed grids
- HUD primitives for screen-space UI
- Audio playback through `rodio`
- Fluent `EntityBuilder` for common spawn + physics + sprite + script setup

## Editor And AI

The editor is in active development. It currently provides:

- Scene hierarchy and component inspection
- Transform, sprite, collider, script, camera, audio, and physics-body editor components
- Canvas viewport preview and screenshot polling
- Lua script editing
- Local AI chat powered by Ollama
- AI action execution for entity creation, transforms, scripts, and components

The editor scene model is still catching up with the engine core. It now includes explicit `PhysicsBody` support, but full engine parity is not complete yet.

## Documentation

Start with:

- [Getting Started](docs/getting-started.md)
- [Rendering](docs/rendering.md)
- [Physics](docs/physics.md)
- [Scripting](docs/scripting.md)
- [World & Entities](docs/world.md)
- [Examples](docs/examples.md)
- [API Reference](docs/api-reference.md)

## Repository Status

`IMPROVEMENTS.md` tasks 1-10 are complete. Current work is focused on editor/server/AI parity with the engine core and keeping docs aligned with the Sindri rename.

## Requirements

- Rust 2021 edition or later
- A GPU and graphics drivers supported by `wgpu`
- `pnpm` for the editor
- Ollama for local AI features

## License

MIT OR Apache-2.0
