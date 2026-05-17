# Built-in Entity Components

Sindri provides a set of built-in entity components for common game objects. These components can be attached to entities to create standard game objects.

## Overview

Built-in components include:
- **Transform** - Position, rotation, and scale
- **SpriteComponent** - Visual representation
- **PhysicsBody** - Physics body marker
- **AudioSource** - Positional audio
- **CameraComponent** - Camera attachment
- **Tag components** - Player, Enemy, Collectible, Hazard, Checkpoint, Trigger, MovingPlatform

## Transform Component

The `Transform` component represents position, rotation, and scale.

```rust
use sindri::{Transform, Vec2};

let transform = Transform::new(Vec2::new(100.0, 200.0))
    .with_rotation(0.5)  // Radians
    .with_scale(Vec2::new(2.0, 2.0));
```

### Properties

- `position: Vec2` - World position
- `rotation: f32` - Rotation in radians
- `scale: Vec2` - Scale multiplier

## SpriteComponent

The `SpriteComponent` provides visual representation for an entity.

```rust
use sindri::{SpriteComponent, TextureHandle};

let sprite = SpriteComponent::new(texture_handle)
    .with_tint(1.0, 0.0, 0.0, 1.0);  // Red tint
```

### Properties

- `texture: TextureHandle` - Texture to render
- `sprite: Sprite` - Internal sprite object
- `visible: bool` - Whether the sprite is visible

## PhysicsBody

The `PhysicsBody` component marks an entity as having a physics body.

```rust
use sindri::{PhysicsBody, RigidBodyType, ColliderShape};

let physics_body = PhysicsBody::new(RigidBodyType::Dynamic)
    .with_collider(ColliderShape::Box { hx: 15.0, hy: 15.0 });
```

**Note:** The actual physics body must be created separately using `PhysicsWorld::create_body()`. This component is just a marker.

## AudioSource

The `AudioSource` component provides positional audio for an entity.

```rust
use sindri::AudioSource;

let audio = AudioSource::new()
    .with_volume(0.8)
    .with_pitch(1.0)
    .with_looping(false);
```

### Properties

- `volume: f32` - Volume (0.0 to 1.0)
- `pitch: f32` - Pitch multiplier
- `looping: bool` - Whether to loop the sound
- `sound_id: Option<u32>` - Reference to loaded sound

## CameraComponent

The `CameraComponent` attaches a camera to an entity.

```rust
use sindri::{CameraComponent, Vec2};

let camera = CameraComponent::new(Vec2::new(0.0, 0.0))
    .with_zoom(1.5);
```

### Properties

- `camera: Camera2D` - The camera object
- `active: bool` - Whether the camera is active

## Tag Components

Tag components are simple marker components for categorizing entities.

### Player

```rust
use sindri::Player;

let player = Player;  // Unit struct
```

### Enemy

```rust
use sindri::Enemy;

let enemy = Enemy;  // Unit struct
```

### Collectible

```rust
use sindri::Collectible;

let collectible = Collectible::new(10);  // Value: 10 points
```

### Hazard

```rust
use sindri::Hazard;

let hazard = Hazard::new(5);  // Damage: 5 HP
```

### Checkpoint

```rust
use sindri::Checkpoint;

let checkpoint = Checkpoint::new(1);  // Checkpoint ID: 1
```

### Trigger

```rust
use sindri::Trigger;

let trigger = Trigger::new(1);  // Trigger ID: 1
// trigger.activated tracks if it's been activated
```

### MovingPlatform

```rust
use sindri::{MovingPlatform, Vec2};

let platform = MovingPlatform::new(
    Vec2::new(100.0, 200.0),  // Start position
    Vec2::new(500.0, 200.0),  // End position
    50.0,                      // Speed
);
```

## Usage Example

```rust
use sindri::{World, Transform, SpriteComponent, Player, Vec2};

struct Game {
    world: World,
    player_entity: EntityId,
}

impl Game {
    fn spawn_player(&mut self, texture: TextureHandle) -> EntityId {
        let entity = self.world.spawn();
        
        // Add components
        self.world.add_component(entity, Transform::new(Vec2::new(100.0, 100.0)));
        self.world.add_component(entity, SpriteComponent::new(texture));
        self.world.add_component(entity, Player);
        
        entity
    }
}
```

## Component Serialization

Components can be serialized for scene saving/loading. See [Scene Serialization](scene.md) for details.

## Best Practices

1. **Always use Transform** - Most entities should have a Transform component for position
2. **Use tag components** - Tag components are lightweight and great for queries
3. **Combine components** - Entities can have multiple components (e.g., Transform + SpriteComponent + Player)
4. **Keep components simple** - Components should represent data, not behavior

