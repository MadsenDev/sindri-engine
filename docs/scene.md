# Scene Serialization

Sindri provides a scene serialization system for saving and loading game worlds, including physics state.

## Overview

The scene system allows you to:
- Save complete game state to JSON
- Load game state from JSON
- Preserve physics properties (position, velocity, etc.)
- Serialize custom components

## Basic Usage

### Saving a Scene

```rust
use sindri::{create_scene, Scene, World, PhysicsWorld};

fn save_game(world: &World, physics: &PhysicsWorld) -> Result<()> {
    // Create scene from current state
    let scene = create_scene(world, physics)?;
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&scene)?;
    
    // Save to file
    std::fs::write("save.json", json)?;
    
    Ok(())
}
```

### Loading a Scene

```rust
use sindri::{restore_scene_physics, World, PhysicsWorld};

fn load_game(world: &mut World, physics: &mut PhysicsWorld) -> Result<()> {
    // Read from file
    let json = std::fs::read_to_string("save.json")?;
    
    // Deserialize scene
    let scene: Scene = serde_json::from_str(&json)?;
    
    // Restore physics state
    restore_scene_physics(physics, &scene.physics)?;
    
    // Restore entities and components
    // (You'll need to implement component deserialization)
    
    Ok(())
}
```

## Scene Structure

A `Scene` contains:

```rust
pub struct Scene {
    pub version: u32,              // Scene version for migration
    pub entities: Vec<SerializableEntity>,
    pub physics: SerializablePhysics,
}
```

### SerializableEntity

```rust
pub struct SerializableEntity {
    pub id: EntityId,
    pub components: Vec<SerializableComponent>,
}
```

### SerializableComponent

```rust
pub struct SerializableComponent {
    pub type_name: String,  // Component type name
    pub data: serde_json::Value,  // Serialized component data
}
```

### SerializablePhysics

```rust
pub struct SerializablePhysics {
    pub gravity: Vec2,
    pub bodies: Vec<SerializableBody>,
    pub colliders: Vec<SerializableCollider>,
}
```

## Component Serialization

To serialize custom components, implement the `ComponentSerializable` trait:

```rust
use sindri::{ComponentSerializable, SerializableComponent};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct MyComponent {
    health: i32,
    score: u32,
}

impl ComponentSerializable for MyComponent {
    fn serialize(&self) -> SerializableComponent {
        SerializableComponent {
            type_name: "MyComponent".to_string(),
            data: serde_json::to_value(self).unwrap(),
        }
    }
    
    fn deserialize(data: &serde_json::Value) -> Result<Self> {
        Ok(serde_json::from_value(data.clone())?)
    }
}
```

### Serializing Components

```rust
use sindri::World;

fn save_components(world: &World, entity: EntityId) -> Result<Vec<SerializableComponent>> {
    let mut components = Vec::new();
    
    // Serialize each component
    if let Some(transform) = world.get::<Transform>(entity) {
        components.push(transform.serialize());
    }
    
    if let Some(my_comp) = world.get::<MyComponent>(entity) {
        components.push(my_comp.serialize());
    }
    
    Ok(components)
}
```

### Deserializing Components

```rust
fn load_components(world: &mut World, entity: EntityId, components: &[SerializableComponent]) -> Result<()> {
    for comp in components {
        match comp.type_name.as_str() {
            "Transform" => {
                let transform = Transform::deserialize(&comp.data)?;
                world.insert(entity, transform);
            }
            "MyComponent" => {
                let my_comp = MyComponent::deserialize(&comp.data)?;
                world.insert(entity, my_comp);
            }
            _ => {
                eprintln!("Unknown component type: {}", comp.type_name);
            }
        }
    }
    Ok(())
}
```

## Physics Serialization

Physics state is automatically serialized and includes:

- **Gravity** - World gravity vector
- **Bodies** - Position, rotation, velocity, body type
- **Colliders** - Shape, offset, material properties, sensor flag

### SerializableBody

```rust
pub struct SerializableBody {
    pub entity: EntityId,
    pub body_type: RigidBodyType,
    pub position: Vec2,
    pub rotation: f32,
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,
}
```

### SerializableCollider

```rust
pub struct SerializableCollider {
    pub entity: EntityId,
    pub shape: ColliderShape,
    pub offset: Vec2,
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub is_sensor: bool,
}
```

## Complete Example

```rust
use sindri::{create_scene, restore_scene_physics, Scene, World, PhysicsWorld};
use std::fs;

struct Game {
    world: World,
    physics: PhysicsWorld,
}

impl Game {
    fn save(&self, filename: &str) -> Result<()> {
        let scene = create_scene(&self.world, &self.physics)?;
        let json = serde_json::to_string_pretty(&scene)?;
        fs::write(filename, json)?;
        Ok(())
    }
    
    fn load(&mut self, filename: &str) -> Result<()> {
        let json = fs::read_to_string(filename)?;
        let scene: Scene = serde_json::from_str(&json)?;
        
        // Clear and restore physics
        restore_scene_physics(&mut self.physics, &scene.physics)?;
        
        // Restore entities
        for entity_data in &scene.entities {
            let entity = self.world.spawn();
            
            // Restore components
            for comp in &entity_data.components {
                // Deserialize based on type_name
                // (implementation depends on your component types)
            }
        }
        
        Ok(())
    }
}
```

## Important Notes

1. **Entity IDs change** - When loading, new entities are created with new IDs. You may need to maintain an ID mapping.

2. **Physics is cleared** - `restore_scene_physics` clears the physics world before restoring. Make sure to recreate any static bodies (like ground) after loading.

3. **Component types** - You must implement deserialization for all component types you want to save/load.

4. **Versioning** - Use the `version` field to handle migration between different scene formats.

## Scene Version Migration

```rust
fn load_with_migration(scene: &mut Scene) -> Result<()> {
    match scene.version {
        1 => {
            // Handle version 1 format
        }
        2 => {
            // Handle version 2 format
        }
        _ => {
            return Err(anyhow!("Unsupported scene version: {}", scene.version));
        }
    }
    Ok(())
}
```
