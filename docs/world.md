# World & Entities

Sindri includes a lightweight **World** and **EntityId** system that gives you a central place
to manage game entities and their components, without committing to a full ECS.

This is ideal for:

- Centralizing entity/component data
- Driving debug views or tools
- Preparing for a future, more advanced ECS if you need it

## Core Types

```rust
use sindri::{World, EntityId};
```

### EntityId

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(u32);
```

- Uniquely identifies an entity in a `World`
- Can be converted to a `u32` via `to_u32()` for debugging/serialization

### World

```rust
pub struct World {
    // internal fields
}
```

The `World`:

- Owns all **entities** and their **components**
- Stores components by type (`T: 'static`) keyed by `EntityId`
- Provides simple APIs for:
  - Spawning / despawning entities
  - Adding / removing / querying components

## Spawning & Despawning

```rust
use sindri::{World, EntityId};

let mut world = World::new();

// Spawn a new entity
let player: EntityId = world.spawn();

assert!(world.is_alive(player));
assert_eq!(world.len(), 1);

// Despawn
world.despawn(player);
assert!(!world.is_alive(player));
assert!(world.is_empty());
```

## Components

Components are plain Rust types (`T: 'static`) stored internally in type-based maps.

### Adding Components

```rust
#[derive(Debug)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Debug)]
struct Velocity {
    vx: f32,
    vy: f32,
}

let mut world = World::new();
let e = world.spawn();

world.insert(e, Position { x: 100.0, y: 200.0 });
world.insert(e, Velocity { vx: 10.0, vy: 0.0 });
```

### Getting & Modifying Components

```rust
// Immutable
if let Some(pos) = world.get::<Position>(e) {
    println!("Entity at: ({}, {})", pos.x, pos.y);
}

// Mutable
if let Some(vel) = world.get_mut::<Velocity>(e) {
    vel.vx += 1.0;
}
```

### Removing Components

```rust
let removed: Option<Position> = world.remove::<Position>(e);
```

## Querying

You can iterate over all entities that have a specific component type:

```rust
for (entity, pos) in world.query::<Position>() {
    println!("Entity {:?} at ({}, {})", entity.to_u32(), pos.x, pos.y);
}
```

This returns a `Vec<(EntityId, &T)>` for simplicity. For many games and tools, this is
perfectly adequate and keeps the API straightforward.

For mutable single-component iteration, use `query_mut::<T>()`:

```rust
for (_entity, velocity) in world.query_mut::<Velocity>() {
    velocity.vx *= 0.95;
}
```

## Integration Pattern

The recommended usage pattern is:

- Store a `World` inside your game/state struct:

```rust
use sindri::World;

struct GameState {
    world: World,
    // other fields...
}
```

- Initialize entities and components in `init()`:

```rust
fn init(&mut self, _ctx: &mut EngineContext) -> Result<()> {
    let player = self.world.spawn();
    self.world.insert(player, Position { x: 100.0, y: 100.0 });
    self.world.insert(player, Velocity { vx: 0.0, vy: 0.0 });
    Ok(())
}
```

- Use the world in `update()` and `draw()`:

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();

    // Simple movement system
    let entities: Vec<(EntityId, Position, Velocity)> = self.world
        .query::<Position>()
        .into_iter()
        .filter_map(|(e, pos)| {
            let vel = self.world.get::<Velocity>(e)?;
            Some((e, *pos, *vel))
        })
        .collect();

    for (e, mut pos, vel) in entities {
        pos.x += vel.vx * dt;
        pos.y += vel.vy * dt;
        self.world.insert(e, pos); // write back
    }

    Ok(())
}
```

This keeps:

- **Engine** responsible for timing, input, rendering, etc.
- **World** responsible for entity/component storage
- Your **game/state** responsible for systems (movement, AI, etc.)

## Component Serialization

The `World` system integrates with the scene serialization system. Components can be serialized and deserialized for save/load functionality.

See [Scene Serialization](scene.md) for details on saving and loading game state.

### Serializing Components

```rust
use sindri::{World, ComponentSerializable, SerializableComponent};

// Components that implement ComponentSerializable can be serialized
if let Some(component) = world.get::<MyComponent>(entity) {
    let serialized = component.serialize();
    // Save to scene...
}
```

### Deserializing Components

```rust
// When loading, deserialize and add components
let component = MyComponent::deserialize(&data)?;
world.insert(entity, component);
```

## EntityBuilder

`EntityBuilder` reduces common spawn + component + physics setup:

```rust
let player = EntityBuilder::new(&mut world, &mut physics)
    .at(Vec2::new(200.0, 300.0))
    .dynamic()
    .box_collider(14.0, 18.0)
    .sprite(player_texture)
    .tag("player")
    .build();
```

## Limitations (by design)

This is intentionally **not** a full ECS:

- No archetypes or advanced layout optimizations
- No parallel iteration
- Queries are single-type only (`query::<T>()`, `query_mut::<T>()`)
- No system scheduling or execution order guarantees
- Component add/remove during iteration is not explicitly handled (be careful)

### Query Ergonomics

The current query system is simple but has limitations:

```rust
// Single-type query (works well)
for (entity, pos) in world.query::<Position>() {
    // ...
}

// Multi-component access (requires manual filtering)
let entities: Vec<(EntityId, Position, Velocity)> = world
    .query::<Position>()
    .into_iter()
    .filter_map(|(e, pos)| {
        let vel = world.get::<Velocity>(e)?;
        Some((e, *pos, *vel))
    })
    .collect();
```

**Borrow checker notes:**
- Queries return owned `Vec` to avoid lifetime issues
- Mutable single-component iteration uses `query_mut()`
- No borrow checker magic—you handle iteration safety

### When to Move to a Full ECS

Consider switching to an ECS crate (like `hecs`, `bevy_ecs`, etc.) if:

- You have thousands of entities and performance becomes an issue
- You need complex queries (multiple component types, filters)
- You want parallel system execution
- You need archetype-based optimizations

Until then, this `World` + `EntityId` layer gives you:

- Centralized entity management
- Clean separation between data (world) and behavior (systems/game code)
- A solid foundation for future tools and editors (entity inspectors, hierarchies, etc.)
- Simple, predictable API without magic
