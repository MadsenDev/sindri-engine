//! Scene serialization system for Sindri.
//!
//! Provides save/load functionality for game worlds and physics state.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::math::Vec2;
use crate::physics::{ColliderShape, PhysicsWorld, RigidBodyType};
use crate::world::{EntityId, World};

/// Serializable representation of a physics body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableBody {
    pub entity: EntityId,
    pub body_type: RigidBodyType,
    pub position: Vec2,
    pub rotation: f32,
    pub linear_velocity: Vec2,
    pub angular_velocity: f32,
}

/// Serializable representation of a collider.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableCollider {
    pub entity: EntityId,
    pub shape: ColliderShape,
    pub offset: Vec2,
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub is_sensor: bool,
}

/// Serializable representation of physics world state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializablePhysics {
    pub gravity: Vec2,
    pub bodies: Vec<SerializableBody>,
    pub colliders: Vec<SerializableCollider>,
}

/// Serializable component data for an entity.
///
/// Components are stored as JSON strings since we can't know all component types at compile time.
/// Users should implement serialization for their specific component types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableComponent {
    /// Type name of the component (for deserialization).
    pub type_name: String,
    /// Serialized component data as JSON.
    pub data: serde_json::Value,
}

/// Serializable entity with its components.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializableEntity {
    pub id: EntityId,
    pub components: Vec<SerializableComponent>,
}

/// Complete scene representation that can be serialized.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    /// Scene version for migration support.
    pub version: u32,
    /// All entities and their components.
    pub entities: Vec<SerializableEntity>,
    /// Physics world state.
    pub physics: SerializablePhysics,
}

impl Scene {
    /// Create a new empty scene.
    pub fn new() -> Self {
        Self {
            version: 1,
            entities: Vec::new(),
            physics: SerializablePhysics {
                gravity: Vec2::new(0.0, 9.81),
                bodies: Vec::new(),
                colliders: Vec::new(),
            },
        }
    }

    /// Serialize this scene to JSON.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Deserialize a scene from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Save this scene to a file.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a scene from a file.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a scene from a world and physics world.
///
/// Note: This only captures physics state. To capture component data,
/// you need to manually serialize components using `World::serialize_component`.
pub fn create_scene(physics: &PhysicsWorld) -> Scene {
    Scene {
        version: 1,
        entities: Vec::new(), // Components need to be serialized manually
        physics: physics.extract_serializable(),
    }
}

/// Restore a scene to a physics world.
///
/// Note: This only restores physics state. To restore component data,
/// you need to manually deserialize components using `World::deserialize_component`.
pub fn restore_scene_physics(physics: &mut PhysicsWorld, scene: &Scene) -> Result<()> {
    physics.restore_from_serializable(&scene.physics)
}

/// Restore a scene to a physics world, preserving specified entities.
///
/// Useful for preserving static objects like ground/platforms when loading.
pub fn restore_scene_physics_preserve(
    physics: &mut PhysicsWorld,
    scene: &Scene,
    preserve_entities: &[EntityId],
) -> Result<()> {
    physics.restore_from_serializable_preserve(&scene.physics, preserve_entities)
}

/// Helper trait for components that can be serialized.
///
/// Users should implement this for their component types to enable scene serialization.
pub trait ComponentSerializable:
    serde::Serialize + serde::de::DeserializeOwned + Send + 'static
{
    /// Get the type name for this component.
    fn type_name() -> &'static str;
}

impl PhysicsWorld {
    /// Extract serializable physics state from the physics world.
    pub fn extract_serializable(&self) -> SerializablePhysics {
        let mut bodies = Vec::new();
        let mut colliders = Vec::new();

        // Extract all bodies
        for entity in self.all_entities_with_bodies() {
            if let (Some(position), Some(rotation), Some(body_type)) = (
                self.body_position(entity),
                self.body_rotation(entity),
                self.body_type(entity),
            ) {
                let linear_velocity = self.linear_velocity(entity).unwrap_or(Vec2::ZERO);
                let angular_velocity = self.angular_velocity(entity).unwrap_or(0.0);

                bodies.push(SerializableBody {
                    entity,
                    body_type,
                    position,
                    rotation,
                    linear_velocity,
                    angular_velocity,
                });

                // Extract colliders for this entity
                for (shape, offset, density, friction, restitution, is_sensor) in
                    self.get_colliders(entity)
                {
                    colliders.push(SerializableCollider {
                        entity,
                        shape,
                        offset,
                        density,
                        friction,
                        restitution,
                        is_sensor,
                    });
                }
            }
        }

        SerializablePhysics {
            gravity: self.gravity(),
            bodies,
            colliders,
        }
    }

    /// Restore physics state from serializable data.
    pub fn restore_from_serializable(&mut self, data: &SerializablePhysics) -> Result<()> {
        self.restore_from_serializable_preserve(data, &[])
    }

    /// Restore physics state from serializable data, preserving specified entities.
    pub fn restore_from_serializable_preserve(
        &mut self,
        data: &SerializablePhysics,
        preserve_entities: &[EntityId],
    ) -> Result<()> {
        // Clear existing physics, but preserve specified entities
        let entities: Vec<EntityId> = self.all_entities_with_bodies();
        for entity in entities {
            if !preserve_entities.contains(&entity) {
                self.remove_body(entity);
            }
        }

        // Restore gravity
        self.set_gravity(data.gravity);

        // Restore bodies (without colliders first)
        for body_data in &data.bodies {
            // Skip if this entity should be preserved (already exists)
            if preserve_entities.contains(&body_data.entity) {
                continue;
            }

            // Use saved position directly (no offset needed with fresh physics world)
            let position = body_data.position;

            self.create_body(
                body_data.entity,
                body_data.body_type,
                position,
                body_data.rotation,
            )?;
        }

        // Restore colliders (must be done after bodies exist)
        // First, collect all entity IDs that have bodies (so we can verify colliders have bodies)
        let entities_with_bodies: std::collections::HashSet<EntityId> =
            data.bodies.iter().map(|b| b.entity).collect();

        for collider_data in &data.colliders {
            // Skip if this entity should be preserved
            if preserve_entities.contains(&collider_data.entity) {
                continue;
            }

            // Verify the entity has a body before trying to add collider
            if !entities_with_bodies.contains(&collider_data.entity) {
                // This shouldn't happen, but log it and skip
                eprintln!(
                    "Warning: Collider for entity {:?} has no corresponding body, skipping",
                    collider_data.entity
                );
                continue;
            }

            if collider_data.is_sensor {
                if let Err(e) = self.add_sensor(
                    collider_data.entity,
                    collider_data.shape,
                    collider_data.offset,
                ) {
                    eprintln!(
                        "Failed to restore sensor collider for entity {:?}: {}",
                        collider_data.entity, e
                    );
                    return Err(e);
                }
            } else {
                if let Err(e) = self.add_collider_with_material(
                    collider_data.entity,
                    collider_data.shape,
                    collider_data.offset,
                    collider_data.density,
                    collider_data.friction,
                    collider_data.restitution,
                ) {
                    eprintln!(
                        "Failed to restore collider for entity {:?}: {}",
                        collider_data.entity, e
                    );
                    return Err(e);
                }
            }
        }

        // Now set velocities, damping, and wake up bodies AFTER colliders are added
        // This matches the order used when spawning new objects
        for body_data in &data.bodies {
            // Skip if this entity should be preserved (already exists)
            if preserve_entities.contains(&body_data.entity) {
                continue;
            }

            self.set_linear_velocity(body_data.entity, body_data.linear_velocity);
            self.set_angular_velocity(body_data.entity, body_data.angular_velocity);

            // Set damping to match spawn behavior (spawn sets these for dynamic bodies)
            if matches!(body_data.body_type, RigidBodyType::Dynamic) {
                self.set_linear_damping(body_data.entity, 0.1);
                self.set_angular_damping(body_data.entity, 0.2);
                self.wake_up(body_data.entity, true);
            }
        }

        // Wake up preserved entities to ensure they're active
        for entity in preserve_entities {
            if let Some(body_type) = self.body_type(*entity) {
                if matches!(body_type, RigidBodyType::Dynamic) {
                    self.wake_up(*entity, true);
                }
            }
        }

        // Update query pipeline after all bodies/colliders are restored
        self.update_query_pipeline();

        // Verify colliders were restored correctly
        let restored_body_count = self.all_entities_with_bodies().len();
        let expected_body_count = data.bodies.len() + preserve_entities.len();
        if restored_body_count != expected_body_count {
            eprintln!(
                "Warning: Expected {} bodies after restore, but found {}",
                expected_body_count, restored_body_count
            );
        }

        // Verify each body has colliders
        for body_data in &data.bodies {
            if preserve_entities.contains(&body_data.entity) {
                continue;
            }
            let collider_count = self.get_colliders(body_data.entity).len();
            if collider_count == 0 {
                eprintln!(
                    "Warning: Entity {:?} has no colliders after restore!",
                    body_data.entity
                );
            }
        }

        Ok(())
    }
}

/// Helper functions for serializing/deserializing World components.
impl World {
    /// Serialize a component of type T for an entity.
    ///
    /// Returns None if the entity doesn't have this component or if serialization fails.
    pub fn serialize_component<T: ComponentSerializable>(
        &self,
        entity: EntityId,
    ) -> Option<SerializableComponent> {
        let comp = self.get::<T>(entity)?;
        let data = serde_json::to_value(comp).ok()?;
        Some(SerializableComponent {
            type_name: T::type_name().to_string(),
            data,
        })
    }

    /// Deserialize and insert a component for an entity.
    pub fn deserialize_component<T: ComponentSerializable>(
        &mut self,
        entity: EntityId,
        serialized: &SerializableComponent,
    ) -> Result<()> {
        if serialized.type_name != T::type_name() {
            return Err(anyhow!(
                "Type mismatch: expected {}, got {}",
                T::type_name(),
                serialized.type_name
            ));
        }

        let component: T = serde_json::from_value(serialized.data.clone())?;
        self.insert(entity, component);
        Ok(())
    }
}
