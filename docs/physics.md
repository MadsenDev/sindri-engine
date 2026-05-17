# Physics System

Sindri includes a 2D physics engine powered by Rapier2D, providing realistic physics simulation for your games.

## Overview

The physics system provides:
- **Rigid body dynamics** - Dynamic, kinematic, and fixed bodies
- **Collision detection** - Automatic collision detection and response
- **Colliders** - Box, circle, and capsule shapes
- **Sensors** - Trigger zones that detect collisions without physical response
- **Physics events** - Collision and trigger callbacks
- **Continuous Collision Detection (CCD)** - Prevents fast-moving objects from tunneling through colliders

## Basic Usage

### Creating a Physics World

```rust
use sindri::{PhysicsWorld, Vec2};

let mut physics = PhysicsWorld::new();

// Set custom gravity (default is Vec2::new(0.0, 9.81))
physics.set_gravity(Vec2::new(0.0, 400.0));
```

### Creating Bodies

```rust
use sindri::{RigidBodyType, EntityId};

let entity = world.spawn();

// Create a dynamic body
physics.create_body(entity, RigidBodyType::Dynamic, Vec2::new(100.0, 100.0), 0.0)?;

// Create a fixed (static) body
physics.create_body(entity, RigidBodyType::Fixed, Vec2::new(0.0, 500.0), 0.0)?;
```

### Adding Colliders

```rust
use sindri::ColliderShape;

// Add a box collider
physics.add_collider_with_material(
    entity,
    ColliderShape::Box { hx: 15.0, hy: 15.0 },  // Half-extents
    Vec2::ZERO,  // Offset from body center
    1.0,  // Density
    0.5,  // Friction
    0.3,  // Restitution (bounciness)
)?;

// Add a circle collider
physics.add_collider_with_material(
    entity,
    ColliderShape::Circle { radius: 10.0 },
    Vec2::ZERO,
    1.0, 0.5, 0.3,
)?;
```

### Stepping the Simulation

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();
    
    // Step physics simulation
    self.physics.step(dt);
    
    Ok(())
}
```

## Body Types

### Dynamic

Dynamic bodies are affected by forces, gravity, and collisions. They move and rotate based on physics.

```rust
physics.create_body(entity, RigidBodyType::Dynamic, pos, 0.0)?;
```

### Kinematic

Kinematic bodies are controlled by velocity. They push dynamic bodies but aren't affected by forces.

```rust
physics.create_body(entity, RigidBodyType::Kinematic, pos, 0.0)?;
```

### Fixed

Fixed bodies are static and never move. They're perfect for ground, walls, and platforms.

```rust
physics.create_body(entity, RigidBodyType::Fixed, pos, 0.0)?;
```

## Collider Shapes

### Box

```rust
ColliderShape::Box { hx: 15.0, hy: 15.0 }  // Half-extents (width/2, height/2)
```

### Circle

```rust
ColliderShape::Circle { radius: 10.0 }
```

### Capsule

```rust
ColliderShape::CapsuleY { half_height: 20.0, radius: 5.0 }  // Vertical capsule
```

## Material Properties

### Density

Controls the mass of the body (mass = density × volume). Higher density = heavier objects.

```rust
physics.add_collider_with_material(entity, shape, offset, 2.0, /* ... */);  // Heavy
physics.add_collider_with_material(entity, shape, offset, 0.5, /* ... */);  // Light
```

### Friction

Controls how much objects resist sliding. Range: 0.0 (no friction) to 1.0+ (high friction).

```rust
physics.add_collider_with_material(entity, shape, offset, 1.0, 0.8, /* ... */);  // High friction
physics.add_collider_with_material(entity, shape, offset, 1.0, 0.1, /* ... */);  // Low friction
```

### Restitution

Controls bounciness. Range: 0.0 (no bounce) to 1.0 (perfectly elastic).

```rust
physics.add_collider_with_material(entity, shape, offset, 1.0, 0.5, 0.9);  // Very bouncy
physics.add_collider_with_material(entity, shape, offset, 1.0, 0.5, 0.0);  // No bounce
```

## Sensors

Sensors detect collisions but don't create physical responses. Perfect for trigger zones, pickups, and checkpoints.

```rust
// Create a sensor (trigger zone)
physics.add_sensor(
    entity,
    ColliderShape::Circle { radius: 50.0 },
    Vec2::ZERO,
)?;
```

## Physics Events

Listen for collision and trigger events:

```rust
use sindri::PhysicsEvent;

physics.on_event(|event| {
    match event {
        PhysicsEvent::CollisionEnter { a, b } => {
            println!("Collision between entity {} and {}", a.to_u32(), b.to_u32());
        }
        PhysicsEvent::CollisionExit { a, b } => {
            println!("Collision ended between entity {} and {}", a.to_u32(), b.to_u32());
        }
        PhysicsEvent::TriggerEnter { a, b } => {
            println!("Entity {} entered trigger {}", b.to_u32(), a.to_u32());
        }
        PhysicsEvent::TriggerExit { a, b } => {
            println!("Entity {} exited trigger {}", b.to_u32(), a.to_u32());
        }
    }
});
```

## Manipulating Bodies

### Getting Position and Rotation

```rust
if let Some(pos) = physics.body_position(entity) {
    println!("Position: {:?}", pos);
}

if let Some(rot) = physics.body_rotation(entity) {
    println!("Rotation: {} radians", rot);
}
```

### Setting Velocity

```rust
// Set linear velocity directly
physics.set_linear_velocity(entity, Vec2::new(100.0, 0.0));

// Get current velocity
if let Some(vel) = physics.linear_velocity(entity) {
    println!("Velocity: {:?}", vel);
}
```

### Applying Forces

```rust
// Apply an impulse (instant force)
physics.apply_impulse(entity, Vec2::new(0.0, -400.0));  // Jump

// Apply a continuous force
physics.apply_force(entity, Vec2::new(100.0, 0.0));  // Push right
```

### Locking Rotation

For platformer characters, you often want to prevent rotation:

```rust
physics.lock_rotations(entity, true);   // Lock rotation
physics.lock_rotations(entity, false);  // Unlock rotation
```

### Damping

Control how quickly objects slow down:

```rust
// Linear damping (air resistance)
physics.set_linear_damping(entity, 0.5);  // Higher = more resistance
```

## Continuous Collision Detection (CCD)

CCD is automatically enabled for dynamic bodies to prevent fast-moving objects from tunneling through thin colliders. This is especially important for:
- Bullets
- Fast-moving projectiles
- Objects with high velocity

CCD is enabled by default for all dynamic bodies.

## Example: Platformer Character

```rust
use sindri::{PhysicsWorld, RigidBodyType, ColliderShape, Vec2};

fn create_player(physics: &mut PhysicsWorld, entity: EntityId) -> Result<()> {
    // Create dynamic body
    physics.create_body(entity, RigidBodyType::Dynamic, Vec2::new(100.0, 100.0), 0.0)?;
    
    // Add capsule collider (good for characters)
    physics.add_collider_with_material(
        entity,
        ColliderShape::CapsuleY { half_height: 20.0, radius: 8.0 },
        Vec2::ZERO,
        1.0,   // Density
        0.5,   // Friction
        0.0,   // No bounce
    )?;
    
    // Lock rotation (prevent character from tipping over)
    physics.lock_rotations(entity, true);
    
    // Set damping for air resistance
    physics.set_linear_damping(entity, 0.1);
    
    Ok(())
}

fn jump(physics: &mut PhysicsWorld, entity: EntityId) {
    // Apply upward impulse
    physics.apply_impulse(entity, Vec2::new(0.0, -400.0));
}

fn move_horizontal(physics: &mut PhysicsWorld, entity: EntityId, direction: f32) {
    // Get current velocity
    if let Some(mut vel) = physics.linear_velocity(entity) {
        vel.x = direction * 300.0;  // Set horizontal velocity
        physics.set_linear_velocity(entity, vel);
    }
}
```

## Example: Ground Platform

```rust
fn create_ground(physics: &mut PhysicsWorld, entity: EntityId) -> Result<()> {
    // Create fixed body
    physics.create_body(
        entity,
        RigidBodyType::Fixed,
        Vec2::new(640.0, 700.0),  // Position
        0.0,
    )?;
    
    // Add wide box collider
    physics.add_collider_with_material(
        entity,
        ColliderShape::Box { hx: 640.0, hy: 15.0 },  // Wide platform
        Vec2::ZERO,
        0.0,   // Density doesn't matter for fixed bodies
        0.8,   // High friction
        0.0,   // No bounce
    )?;
    
    Ok(())
}
```

## Performance Notes

- Physics simulation runs in `step()`, which should be called every frame
- For deterministic physics, use fixed timestep (see [Fixed Timestep](fixed-timestep.md))
- CCD has a small performance cost but is necessary for fast-moving objects
- Too many active bodies can impact performance - consider pooling or despawning off-screen objects

## Integration with Scene System

Physics state can be saved and loaded using the scene serialization system:

```rust
use sindri::{create_scene, restore_scene_physics};

// Save physics state
let scene = create_scene(&world, &physics)?;

// Load physics state
restore_scene_physics(&mut physics, &scene.physics)?;
```

See [Scene Serialization](scene.md) for more details.

