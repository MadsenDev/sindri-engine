# Math Utilities

Sindri provides essential 2D math types for game development.

## Vec2

A 2D vector for positions, velocities, directions, etc.

### Creating Vectors

```rust
use sindri::Vec2;

let position = Vec2::new(100.0, 200.0);
let velocity = Vec2::new(50.0, -30.0);
let zero = Vec2::ZERO;
let one = Vec2::ONE;
```

### Vector Operations

```rust
// Addition/subtraction
let new_pos = position + velocity * dt;
let direction = target - position;

// Scalar operations
let scaled = vec * 2.0;
let divided = vec / 2.0;

// Negation
let opposite = -vec;
```

### Vector Methods

```rust
// Length
let length = vec.length();
let length_sq = vec.length_squared();  // Faster (no sqrt)

// Normalization
let normalized = vec.normalized();  // Unit vector
if vec.length_squared() > 0.0 {
    let dir = vec.normalized();
}

// Distance
let distance = pos1.distance(pos2);
let distance_sq = pos1.distance_squared(pos2);  // Faster

// Dot product
let dot = vec1.dot(vec2);

// Interpolation
let interpolated = start.lerp(end, 0.5);  // 50% between start and end

// Angle
let angle_vec = Vec2::from_angle(std::f32::consts::PI / 4.0);  // 45 degrees

// Component-wise operations
let abs = vec.abs();  // Absolute value of each component
let min = vec1.min(vec2);  // Component-wise minimum
let max = vec1.max(vec2);  // Component-wise maximum
```

### Common Directions

```rust
let up = Vec2::new(0.0, -1.0);      // Screen coordinates (Y down)
let down = Vec2::new(0.0, 1.0);
let left = Vec2::new(-1.0, 0.0);
let right = Vec2::new(1.0, 0.0);
```

### Vec2 Fields

```rust
vec.x  // X component
vec.y  // Y component
```

## Transform2D

Represents position, scale, and rotation of a sprite or entity.

### Creating Transforms

```rust
use sindri::Transform2D;

let transform = Transform2D {
    position: Vec2::new(100.0, 200.0),  // Center of sprite
    scale: Vec2::new(1.0, 1.0),          // Multiplier (1.0 = native size)
    rotation: 0.0,                       // Radians
};
```

### Transform Properties

- **`position: Vec2`** - Position (center of sprite)
- **`scale: Vec2`** - Scale multiplier (1.0 = native size)
- **`rotation: f32`** - Rotation in radians

### Converting to Matrix

```rust
let base_size = Vec2::new(32.0, 32.0);  // Original texture size
let matrix = transform.to_matrix(base_size);
```

## Camera2D

A 2D camera for view projection and coordinate conversion.

### Creating a Camera

```rust
use sindri::Camera2D;

// Create at position
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

// Screen to world coordinates
let world_pos = camera.screen_to_world(
    Vec2::new(screen_x, screen_y),
    screen_w,
    screen_h,
);

// World to screen coordinates
let screen_pos = camera.world_to_screen(
    Vec2::new(world_x, world_y),
    screen_w,
    screen_h,
);
```

### View Projection Matrix

```rust
let matrix = camera.view_projection(screen_w, screen_h);
```

## Common Math Patterns

### Movement

```rust
// Move towards target
let direction = (target - position).normalized();
position += direction * speed * dt;

// Move with velocity
position += velocity * dt;
```

### Distance Checks

```rust
// Check if within range
if position.distance_squared(target) < range * range {
    // Within range (using squared distance is faster)
}
```

### Interpolation

```rust
// Smooth movement
position = position.lerp(target, speed * dt);

// Fade in/out
alpha = alpha.lerp(target_alpha, fade_speed * dt);
```

### Rotation

```rust
// Rotate towards direction
let angle = direction.y.atan2(direction.x);
transform.rotation = angle;

// Rotate over time
transform.rotation += rotation_speed * dt;
```

### Bounds Checking

```rust
// Clamp to bounds
position.x = position.x.clamp(min_x, max_x);
position.y = position.y.clamp(min_y, max_y);

// Wrap around
position.x = position.x.rem_euclid(world_width);
position.y = position.y.rem_euclid(world_height);
```

### Collision Detection

```rust
// Circle collision
let distance = pos1.distance(pos2);
if distance < radius1 + radius2 {
    // Collision
}

// AABB collision (axis-aligned bounding box)
let dx = (pos1.x - pos2.x).abs();
let dy = (pos1.y - pos2.y).abs();
if dx < (size1.x + size2.x) * 0.5 && dy < (size1.y + size2.y) * 0.5 {
    // Collision
}
```

