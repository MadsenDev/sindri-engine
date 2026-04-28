# Fixed Timestep

Forge2D supports fixed timestep updates for deterministic game logic, especially useful for physics and collision detection.

## Why Fixed Timestep?

Variable timestep (frame-based) updates can cause:
- Non-deterministic physics
- Inconsistent collision detection
- Different behavior on different frame rates

Fixed timestep ensures:
- Deterministic updates
- Consistent physics regardless of frame rate
- Reproducible game state

## Using Fixed Timestep

### Implementing fixed_update()

```rust
impl Game for MyGame {
    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called at fixed intervals (default: 60 FPS)
        let fixed_dt = ctx.fixed_delta_time();
        
        // Update physics with fixed timestep
        update_physics(fixed_dt);
        
        // Collision detection
        check_collisions();
        
        Ok(())
    }
    
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Called every frame (variable timestep)
        // Use for input, UI, interpolation, etc.
        Ok(())
    }
}
```

## Canonical Interpolation Recipe

```rust
struct MyGame {
    pos: Vec2,
    prev_pos: Vec2,
}

impl Game for MyGame {
    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.fixed_delta_seconds();
        self.prev_pos = self.pos;
        self.pos += Vec2::new(1.0, 0.0) * dt;
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let alpha = ctx.fixed_update_alpha();
        let render_pos = self.prev_pos.lerp(self.pos, alpha);
        ctx.draw(|_renderer, _frame| {
            // draw sprite at render_pos
            Ok(())
        })?;
        Ok(())
    }
}
```

## Fixed Timestep Methods

### should_run_fixed_update()

```rust
if ctx.should_run_fixed_update() {
    // Run fixed update logic
}
```

Returns `true` when a fixed update should run. Most games should not call this directly because the engine already drives `fixed_update()`. Use it only for custom loops or tooling.

### fixed_delta_time()

```rust
let fixed_dt = ctx.fixed_delta_time();
```

Returns the fixed timestep duration (default: 1/60 seconds = ~16.67ms).

### fixed_update_alpha()

```rust
let alpha = ctx.fixed_update_alpha();
```

Returns the interpolation factor (0.0 to 1.0) between the last fixed update and the next one. Use this to smooth visual rendering.

## Interpolation

For smooth visual rendering, interpolate between fixed update states:

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let alpha = ctx.fixed_update_alpha();
    
    // Interpolate visual position
    let visual_pos = last_fixed_pos.lerp(current_fixed_pos, alpha);
    sprite.transform.position = visual_pos;
    
    Ok(())
}
```

## Common Patterns

### Physics in fixed_update()

```rust
fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let fixed_dt = ctx.fixed_delta_time();
    
    // Update velocity
    velocity += acceleration * fixed_dt.as_secs_f32();
    
    // Update position
    position += velocity * fixed_dt.as_secs_f32();
    
    // Apply constraints
    apply_constraints();
    
    Ok(())
}
```

### Collision Detection in fixed_update()

```rust
fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    // Check collisions at fixed timestep
    for entity in &mut entities {
        for other in &entities {
            if check_collision(entity, other) {
                handle_collision(entity, other);
            }
        }
    }
    Ok(())
}
```

### Input in update()

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    // Input should be checked every frame (variable timestep)
    if ctx.input().is_key_pressed(KeyCode::Space) {
        // Jump
    }
    Ok(())
}
```

## When to Use Fixed Timestep

**Use fixed timestep for:**
- Physics simulation
- Collision detection
- Deterministic game logic
- Network synchronization
- Replay systems

**Use variable timestep (update) for:**
- Input handling
- UI updates
- Visual interpolation
- Non-critical animations

## Default Settings

- **Fixed timestep rate**: 60 FPS (1/60 seconds)
- **Max catch-up**: The engine will run multiple fixed updates per frame if needed to catch up

## Example: Complete Fixed Timestep Game

```rust
struct MyGame {
    // Fixed timestep state
    position: Vec2,
    previous_position: Vec2,
    velocity: Vec2,
}

impl Game for MyGame {
    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let fixed_dt = ctx.fixed_delta_time().as_secs_f32();
        self.previous_position = self.position;

        // Update physics
        self.velocity += acceleration * fixed_dt;
        self.position += self.velocity * fixed_dt;
        
        // Collision detection
        check_collisions(&mut self.position, &mut self.velocity);
        
        Ok(())
    }
    
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Draw using interpolated visual position
        let alpha = ctx.fixed_update_alpha();
        let visual_position = self.previous_position.lerp(self.position, alpha);
        // ... render using visual_position ...
        Ok(())
    }
}
```
