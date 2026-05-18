# Sindri Documentation

Welcome to the Sindri documentation! This guide will help you get started with creating 2D games using Sindri.

## Table of Contents

1. [Getting Started](getting-started.md) - Quick start guide and basic concepts
2. [Engine & Game Loop](engine.md) - Engine configuration and the Game trait
3. [Input System](input.md) - Keyboard and mouse input handling
4. [Rendering](rendering.md) - Sprites, textures, cameras, and text rendering
5. [Math Utilities](math.md) - Vec2, Transform2D, and Camera2D
6. [Asset Management](assets.md) - Loading and caching textures and other assets
7. [Audio System](audio.md) - Playing sound effects and music
8. [Fixed Timestep](fixed-timestep.md) - Deterministic game updates
9. [State Management](state-management.md) - State/scene system for menus, gameplay, pause, etc.
10. [World & Entities](world.md) - Simple world/entity layer for centralized data
11. [Physics System](physics.md) - 2D physics with Rapier2D integration
12. [Grid System](grid.md) - General-purpose grid for tile-based games
13. [Pathfinding](pathfinding.md) - A* pathfinding algorithm
14. [Camera Follow](camera.md) - Camera follow system with dead-zones
15. [Built-in Entities](entities.md) - Standard entity components
16. [Scene Serialization](scene.md) - Save and load game scenes
17. [Lua Scripting](scripting.md) - Script components and runtime lifecycle
18. [Animation](animation.md) - Spritesheet animation helpers
19. [Particles](particles.md) - Particle emitters and systems
20. [API Reference](api-reference.md) - Public API overview
21. [Adoption / Parity Matrix](adoption-parity-matrix.md) - Engine/editor/runtime/AI support tracking
22. [Examples](examples.md) - Code examples and tutorials
23. [Roadmap](ROADMAP.md) - Development philosophy and future direction

## Quick Links

- [Installation & Setup](getting-started.md#installation)
- [Your First Game](getting-started.md#your-first-game)
- [Common Patterns](examples.md#common-patterns)
- [Performance Tips](rendering.md#performance-notes)
- [Adoption / Parity Matrix](adoption-parity-matrix.md)

## What is Sindri?

Sindri is a 2D game framework built with Rust, winit, and wgpu. It provides the core systems needed to build 2D games: rendering, physics, pathfinding, and content pipelines. The focus is on **coherence over completeness**—making the existing systems work well together rather than adding every possible feature.

### Key Features

- ✅ Cross-platform window management
- ✅ Frame-accurate input system
- ✅ Hardware-accelerated 2D rendering
- ✅ Efficient batched sprite rendering (up to 2048 sprites per frame)
- ✅ Text rendering with TTF/OTF fonts
- ✅ 2D camera system with follow behavior and dead-zones
- ✅ Asset caching
- ✅ Audio support
- ✅ Fixed timestep for deterministic physics
- ✅ 2D physics engine (Rapier2D integration)
- ✅ Collision layers and masks
- ✅ General-purpose grid system for tile-based games
- ✅ A* pathfinding algorithm
- ✅ Built-in entity components
- ✅ Scene serialization (save/load)
- ✅ HUD layer for screen-space UI
- ✅ Spritesheet animation helpers
- ✅ Particle systems
- ✅ Shadow-casting point/directional lighting
- ✅ Lua scripting with lifecycle hooks and command buffering
- ✅ Development physics debug drawing
- ✅ Tauri editor/server/AI prototype

### What Sindri Intentionally Doesn't Do

These are **design decisions**, not oversights:

- ❌ **No advanced UI framework** - HUD primitives (text, sprites, rects) only. No layout system, widgets, or input routing. For complex UI, use a library like `egui` or build custom.
- ❌ **No asset pipeline** - No texture atlasing, compression, or hot-reload. Load assets at runtime from files/bytes.
- ❌ **No full ECS framework** - Lightweight `World`/`EntityId` system only. No archetypes, parallel iteration, or complex queries. For advanced ECS, integrate `hecs` or `bevy_ecs`.
- ❌ **No export/packaging tools** - No built-in way to package games for distribution. Use `cargo` and platform-specific tools.

### What Sindri Guarantees

These are promises you can rely on:

- ✅ **Deterministic physics** - Fixed timestep ensures physics is reproducible across runs (given same inputs)
- ✅ **Stable serialization** - Scene save/load format is versioned and backward-compatible within major versions
- ✅ **Consistent coordinate system** - Camera position represents center of view; all coordinate conversions are consistent
- ✅ **Thread-safe events** - Physics events use channels and are safe to handle across threads
- ✅ **No hidden allocations in hot paths** - Rendering and physics updates avoid allocations during gameplay

### Current Gaps (Known Limitations)

These are areas that may need attention depending on your game:

- ⚠️ **Advanced UI** - Complex menus/interfaces require custom code or external UI library
- ⚠️ **Editor parity** - The Tauri editor is catching up with the engine core and does not expose every engine feature yet
- ⚠️ **ECS ergonomics** - Component queries are simple; complex systems may need better iteration patterns
- ⚠️ **Asset management** - No texture atlasing or compression; large asset counts may need optimization

### Requirements

- **Rust**: 2021 edition or later
- **GPU**: Graphics drivers installed (wgpu requirement)
- **Platforms**: Windows, macOS, Linux

## Philosophy: Coherence Over Completeness

Sindri prioritizes making existing systems work well together over adding every possible feature. The engine is designed to be **extended** rather than **complete**.

**Recommended approach:**
1. Pick a **reference game** (platformer, top-down, tile-based, etc.)
2. Build it using Sindri's existing systems
3. Add only what that game **forces** you to add
4. If you can build the same game twice faster the second time, the engine is winning

This keeps Sindri focused and prevents it from becoming a "forever project" that supports everything but feels awkward to use.

## Getting Help

- Check the [Examples](examples.md) for code samples
- Review the [API Reference](api-reference.md) for detailed method documentation
- See the `examples/` directory for the curated examples: `hello_sindri`, `platformer`, `scripted_asteroids`, `editor_scene`, and `rendering`
