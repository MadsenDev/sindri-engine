# Particle System

Sindri includes a flexible CPU-based particle system for creating visual effects like fire, smoke, explosions, and magic spells.

## Overview

The particle system consists of three main components:
-   **`ParticleSystem`**: Manages a collection of emitters.
-   **`ParticleEmitter`**: Spawns and simulates individual particles based on a configuration.
-   **`EmissionConfig`**: Defines how particles behave (velocity, life, color, size, etc.).

## Basic Usage

### 1. Initialize the System

Create a `ParticleSystem` in your game struct.

```rust
struct MyGame {
    particle_system: ParticleSystem,
}

impl MyGame {
    fn new() -> Self {
        Self {
            particle_system: ParticleSystem::new(),
        }
    }
}
```

### 2. Configure an Emitter

Use `EmissionConfig` to define the look and behavior of your particles.

```rust
use sindri::{EmissionConfig, Vec2};

// Create a fire effect configuration
let fire_config = EmissionConfig::new(Vec2::new(100.0, 100.0))
    .with_rate(50.0)                                       // 50 particles per second
    .with_velocity(Vec2::new(-10.0, -50.0), Vec2::new(10.0, -100.0)) // Upward velocity
    .with_color([1.0, 0.5, 0.0, 1.0], Some([1.0, 0.0, 0.0, 0.0]))    // Orange -> Transparent Red
    .with_lifetime(0.5, 1.0)                               // 0.5s to 1.0s lifetime
    .with_size(Vec2::new(5.0, 5.0), Vec2::new(10.0, 10.0)) // Random initial size
    .with_size_end_multiplier(0.5);                        // Shrink to 50% over lifetime
```

### 3. Add to System

Create an emitter from the config and add it to the system.

```rust
use sindri::ParticleEmitter;

let mut emitter = ParticleEmitter::new(fire_config)
    .with_max_particles(100);

// Optional: Assign a texture (defaults to white square if None)
// emitter = emitter.with_texture(Some(my_texture_handle));

self.particle_system.add_emitter(emitter);
```

### 4. Update and Draw

Update the system in your game loop and draw it using the renderer.

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    self.particle_system.update(ctx.delta_time().as_secs_f32());
    Ok(())
}

fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let renderer = ctx.renderer();
    // ... begin frame ...
    
    // Draw all particles
    // Note: You can pass a fallback texture here if emitters don't have their own
    renderer.draw_particles(&mut frame, &self.particle_system, &camera, None)?;
    
    // ... end frame ...
    Ok(())
}
```

## Configuration Guide

`EmissionConfig` provides a fluent builder API for customization.

### Spawning
-   **`with_rate(f32)`**: Particles per second (continuous emission).
-   **`with_burst(usize)`**: Spawn a specific number of particles immediately (one-shot).

### Movement
-   **`with_velocity(min, max)`**: Random initial velocity range.
-   **`with_acceleration(vec2)`**: Constant acceleration (e.g., gravity `Vec2::new(0.0, 9.8)`).

### Appearance
-   **`with_color(start, end)`**: Start color. `end` is optional; if provided, color interpolates over lifetime.
-   **`with_size(min, max)`**: Random initial size range.
-   **`with_size_end_multiplier(f32)`**: how size changes over lifetime (e.g., `2.0` grows to double size).
-   **`with_fade_out(bool)`**: Whether alpha fades to 0 automatically (default `true`).

### Lifetime
-   **`with_lifetime(min, max)`**: Random lifetime range in seconds.

## Emitter Management

You can control emitters after creation:

```rust
// Move an emitter
emitter.set_position(new_pos);

// Stop emission (particles alive will continue to simulate until death)
emitter.stop_emission();

// Check if active (has particles or is emitting)
if emitter.is_active() { ... }
```

## Examples

### Explosion (Burst)
```rust
let explosion_config = EmissionConfig::new(pos)
    .with_burst(50)  // Spawn 50 particles at once
    .with_velocity(Vec2::new(-100.0, -100.0), Vec2::new(100.0, 100.0)) // Radial spread
    .with_lifetime(0.2, 0.5)
    .with_color([1.0, 1.0, 0.0, 1.0], Some([1.0, 0.0, 0.0, 0.0])); // Yellow -> Red fade

let emitter = ParticleEmitter::new(explosion_config);
system.add_emitter(emitter);
```

### Smoke (Continuous)
```rust
let smoke_config = EmissionConfig::new(pos)
    .with_rate(20.0)
    .with_velocity(Vec2::new(-10.0, -20.0), Vec2::new(10.0, -40.0)) // Slow upward drift
    .with_size(Vec2::new(10.0, 10.0), Vec2::new(20.0, 20.0)) // Large puffs
    .with_size_end_multiplier(2.0) // Expand over time
    .with_color([0.5, 0.5, 0.5, 0.5], Some([0.2, 0.2, 0.2, 0.0])); // Grey transparent

let emitter = ParticleEmitter::new(smoke_config);
system.add_emitter(emitter);
```
