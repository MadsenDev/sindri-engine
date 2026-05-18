# Camera Follow System

Sindri provides a camera follow system for smoothly tracking entities or positions with dead-zone support.

## Overview

The `CameraFollow` system allows you to:
- Follow entities or positions
- Define dead-zones (camera doesn't move if target is within this area)
- Smooth camera movement with configurable speed
- Instant or lerp-based following

## Basic Usage

### Following a Position

```rust
use sindri::{CameraFollow, update_camera_follow, Vec2};

struct MyGame {
    camera: Camera2D,
    camera_follow: CameraFollow,
    player_pos: Vec2,
}

impl Game for MyGame {
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        
        // Update camera follow target
        self.camera_follow = CameraFollow::new()
            .follow_position(self.player_pos)
            .with_dead_zone(200.0, 150.0)  // Dead zone size
            .with_smoothing(0.15);          // Smooth following
        
        // Update camera (requires physics world, but you can pass a dummy)
        // For position-based following, you can update manually:
        let offset = self.player_pos - self.camera.position;
        let half_dead_zone = Vec2::new(200.0 / 2.0, 150.0 / 2.0);
        
        if offset.x.abs() > half_dead_zone.x || offset.y.abs() > half_dead_zone.y {
            let mut desired_pos = self.camera.position;
            if offset.x.abs() > half_dead_zone.x {
                desired_pos.x = self.player_pos.x - offset.x.signum() * half_dead_zone.x;
            }
            if offset.y.abs() > half_dead_zone.y {
                desired_pos.y = self.player_pos.y - offset.y.signum() * half_dead_zone.y;
            }
            self.camera.position = self.camera.position.lerp(desired_pos, 0.15);
        }
        
        Ok(())
    }
}
```

### Following an Entity

```rust
use sindri::{CameraFollow, update_camera_follow, PhysicsWorld};

struct MyGame {
    camera: Camera2D,
    camera_follow: CameraFollow,
    physics: PhysicsWorld,
    player_entity: EntityId,
}

impl Game for MyGame {
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        
        // Configure camera follow
        self.camera_follow = CameraFollow::new()
            .follow_entity(self.player_entity)
            .with_dead_zone(200.0, 150.0)
            .with_smoothing(0.15);
        
        // Update camera using the helper function
        update_camera_follow(&mut self.camera, &self.camera_follow, &self.physics, dt);
        
        Ok(())
    }
}
```

## CameraFollow API

### Creating a CameraFollow

```rust
let follow = CameraFollow::new();
```

### Configuration Methods

```rust
// Set entity to follow
follow.follow_entity(entity_id)

// Set position to follow
follow.follow_position(Vec2::new(100.0, 200.0))

// Set dead zone (camera won't move if target is within this area)
follow.with_dead_zone(width: f32, height: f32)

// Enable smooth following with lerp factor (0.0 = instant, 1.0 = very slow)
follow.with_smoothing(factor: f32)

// Set maximum camera speed (for smooth following)
follow.with_max_speed(speed: f32)
```

### Builder Pattern

All methods return `Self`, allowing method chaining:

```rust
let follow = CameraFollow::new()
    .follow_position(player_pos)
    .with_dead_zone(200.0, 150.0)
    .with_smoothing(0.15)
    .with_max_speed(500.0);
```

## Dead Zone

The dead zone defines an area around the camera center where the target can move without the camera following. This creates a more natural camera feel, similar to many platformer games.

```rust
// Large dead zone - camera moves less frequently
follow.with_dead_zone(300.0, 200.0);

// Small dead zone - camera follows more closely
follow.with_dead_zone(100.0, 100.0);

// No dead zone - camera always follows (set to 0, 0)
follow.with_dead_zone(0.0, 0.0);
```

## Smooth Following

Smooth following uses linear interpolation (lerp) to gradually move the camera toward the target position.

```rust
// Smooth following with 15% lerp per frame
follow.with_smoothing(0.15);

// Instant following (no smoothing)
// Just don't call with_smoothing(), or use smoothing factor of 0.0
```

### Max Speed

When using smooth following, you can limit the maximum camera speed:

```rust
// Camera moves at most 500 units per second
follow.with_max_speed(500.0);
```

## update_camera_follow Function

The `update_camera_follow` function handles all the camera movement logic:

```rust
pub fn update_camera_follow(
    camera: &mut Camera2D,
    follow: &CameraFollow,
    physics: &PhysicsWorld,
    dt: f32,
)
```

**Note:** This function requires a `PhysicsWorld` to get entity positions. For position-based following, you can implement the logic manually (see example above).

## Example: Platformer Camera

```rust
use sindri::{CameraFollow, update_camera_follow};

struct PlatformerGame {
    camera: Camera2D,
    camera_follow: CameraFollow,
    physics: PhysicsWorld,
    player_entity: EntityId,
}

impl Game for PlatformerGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Set up camera follow with dead zone for platformer feel
        self.camera_follow = CameraFollow::new()
            .follow_entity(self.player_entity)
            .with_dead_zone(200.0, 150.0)  // Horizontal dead zone larger than vertical
            .with_smoothing(0.15);          // Smooth camera movement
        
        Ok(())
    }
    
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        
        // Update camera to follow player
        update_camera_follow(&mut self.camera, &self.camera_follow, &self.physics, dt);
        
        Ok(())
    }
}
```

## Camera Position

**Important:** `Camera2D.position` represents the **center** of the camera view, not the top-left corner. This is important when calculating dead zones and following logic.


## Camera Features

### Rotation
The camera supports rotation around its center (Z-axis).

```rust
// Rotate 45 degrees
camera.rotation = 45.0_f32.to_radians();

// Or using builder
let camera = Camera2D::default().with_rotation(std::f32::consts::PI / 4.0);
```

### World Bounds
You can restrict the camera movement to a specific area (e.g., the map size).

```rust
// Clamp camera to 0,0 - 1000,1000
camera.bounds = Some((Vec2::ZERO, Vec2::new(1000.0, 1000.0)));

// Or using builder
let camera = Camera2D::default()
    .with_bounds(Vec2::ZERO, Vec2::new(2000.0, 1000.0));
```

### Camera Shake
Add impact to your game with built-in camera shake. The shake decays automatically over time.

```rust
// Shake with intensity 5.0 for 0.5 seconds
camera.shake(5.0, 0.5);
```

### Smooth Zoom
Zoom smoothly to a target level or specific point.

```rust
// Zoom to 2x over time (speed 5.0)
camera.zoom_to(2.0, 5.0);

// Zoom 2x focused on a specific world point (e.g. mouse cursor)
camera.zoom_to_point(mouse_world_pos, 2.0, 5.0, screen_w, screen_h);
```

### Viewport Queries
Efficiently check if objects are visible on screen before drawing them (culling).

```rust
// Check point visibility
if camera.is_point_visible(enemy.pos, screen_w, screen_h) {
    enemy.draw(renderer);
}

// Check rectangle visibility
if camera.is_rect_visible(rect_min, rect_max, screen_w, screen_h) {
    // Draw large map chunk
}
```

## Integrating Camera Update
**Important**: For smooth zoom and shake to work, you MUST call `camera.update(dt)` every frame.

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();
    
    // Update camera logic
    self.camera.update(dt);
    
    Ok(())
}
```

## Examples

For working examples of camera usage, check `examples/platformer` and `examples/rendering`.
