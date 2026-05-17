# Sprite Animation System

Sindri provides a system for playing frame-based animations using spritesheets.

## Overview

-   **`Animation`**: A resource containing a sequence of frames (`AnimationFrame`).
-   **`AnimatedSprite`**: A component that handles playback state (current frame, timer, looping).

## Usage

### 1. Create an Animation

You can create an animation from a list of frames manually, or helper methods.

**From a Spritesheet Grid:**
If you have a texture with a grid of frames (e.g., 4 columns, 2 rows):

```rust
let animation = Animation::from_grid(
    texture_handle,
    (4, 2), // 4 columns, 2 rows
    8,      // Total frames to use
    0.1,    // Duration per frame (seconds)
);
```

### 2. Create an AnimatedEntity

Add `AnimatedSprite` to your entity or game state.

```rust
struct MyGame {
    player_anim: AnimatedSprite,
}

impl MyGame {
    fn new() -> Self {
        // ... load animation ...
        let mut sprite = AnimatedSprite::new(animation);
        sprite.transform.scale = Vec2::new(2.0, 2.0);
        
        Self {
            player_anim: sprite,
        }
    }
}
```

### 3. Update Animation

Call `update` in your game loop.

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    self.player_anim.update(ctx.delta_time().as_secs_f32());
    Ok(())
}
```

### 4. Draw

Render the current frame using `draw_texture_region`.

```rust
fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let renderer = ctx.renderer();
    let mut frame = renderer.begin_frame()?;
    
    if let Some(frame_data) = self.player_anim.current_frame() {
        renderer.draw_texture_region(
            &mut frame,
            frame_data.texture,
            frame_data.source_rect,
            &self.player_anim.transform,
            self.player_anim.tint,
            self.player_anim.is_occluder,
            &camera // Camera2D
        )?;
    }
    
    renderer.end_frame(frame)?;
    Ok(())
}
```

## Advanced Control

-   **Looping**: `Animation` has a `looping` field.
-   **Speed**: `AnimatedSprite.speed` controls playback speed (1.0 = normal).
-   **Flipping**: `AnimatedSprite.flip_x` / `flip_y` (TODO: renderer support for flipping is in `transform.scale`).

Note: To flip a sprite, you can simply use negative scale:
```rust
sprite.transform.scale.x = -1.0 * sprite.transform.scale.x.abs();
```
