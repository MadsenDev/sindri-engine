//! Component metadata system for editor reflection.
//!
//! Provides a way to discover component fields at runtime for dynamic UI generation.
//! This is a manual system - components must register their metadata.

use crate::math::Vec2;
use crate::world::{EntityId, World};
use anyhow::Result;
use serde_json::Value;

/// Describes a field in a component for editor UI generation.
#[derive(Clone, Debug)]
pub struct FieldDescriptor {
    /// Field name
    pub name: String,
    /// Type name (e.g., "f32", "Vec2", "String")
    pub type_name: String,
    /// Optional minimum value (for numeric fields)
    pub min: Option<f64>,
    /// Optional maximum value (for numeric fields)
    pub max: Option<f64>,
    /// Optional step value (for numeric fields)
    pub step: Option<f64>,
    /// Optional enum values (for enum fields)
    pub enum_values: Option<Vec<String>>,
}

/// Type-erased component metadata handler.
pub trait ComponentMetadataHandler: Send + Sync {
    /// Get all field descriptors for this component.
    fn fields(&self) -> Vec<FieldDescriptor>;

    /// Get a field value by name from an entity.
    fn get_field(&self, world: &World, entity: EntityId, field_name: &str) -> Option<Value>;

    /// Set a field value by name on an entity.
    fn set_field(
        &self,
        world: &mut World,
        entity: EntityId,
        field_name: &str,
        value: Value,
    ) -> Result<()>;
}

/// Registry for component metadata.
pub struct ComponentMetadataRegistry {
    metadata: std::collections::HashMap<String, Box<dyn ComponentMetadataHandler>>,
}

impl ComponentMetadataRegistry {
    pub fn new() -> Self {
        Self {
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Register metadata for a component type.
    pub fn register(&mut self, type_name: String, handler: Box<dyn ComponentMetadataHandler>) {
        self.metadata.insert(type_name, handler);
    }

    /// Get metadata handler for a component type.
    pub fn get(&self, type_name: &str) -> Option<&dyn ComponentMetadataHandler> {
        self.metadata.get(type_name).map(|m| m.as_ref())
    }

    /// Get all registered component type names.
    pub fn type_names(&self) -> Vec<String> {
        self.metadata.keys().cloned().collect()
    }
}

impl Default for ComponentMetadataRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Implementation for Transform component
pub struct TransformMetadataHandler;

impl ComponentMetadataHandler for TransformMetadataHandler {
    fn fields(&self) -> Vec<FieldDescriptor> {
        vec![
            FieldDescriptor {
                name: "position".to_string(),
                type_name: "Vec2".to_string(),
                min: None,
                max: None,
                step: None,
                enum_values: None,
            },
            FieldDescriptor {
                name: "rotation".to_string(),
                type_name: "f32".to_string(),
                min: None,
                max: Some(6.28318), // 2 * PI
                step: Some(0.01),
                enum_values: None,
            },
            FieldDescriptor {
                name: "scale".to_string(),
                type_name: "Vec2".to_string(),
                min: Some(0.0),
                max: None,
                step: Some(0.1),
                enum_values: None,
            },
        ]
    }

    fn get_field(&self, world: &World, entity: EntityId, field_name: &str) -> Option<Value> {
        let transform = world.get::<crate::entities::Transform>(entity)?;

        match field_name {
            "position" => Some(serde_json::json!({
                "x": transform.position.x,
                "y": transform.position.y,
            })),
            "rotation" => Some(Value::Number(serde_json::Number::from_f64(
                transform.rotation as f64,
            )?)),
            "scale" => Some(serde_json::json!({
                "x": transform.scale.x,
                "y": transform.scale.y,
            })),
            _ => None,
        }
    }

    fn set_field(
        &self,
        world: &mut World,
        entity: EntityId,
        field_name: &str,
        value: Value,
    ) -> Result<()> {
        use anyhow::anyhow;

        let transform = world
            .get_mut::<crate::entities::Transform>(entity)
            .ok_or_else(|| anyhow!("Entity does not have Transform component"))?;

        match field_name {
            "position" => {
                if let Some(obj) = value.as_object() {
                    let x = obj
                        .get("x")
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| anyhow!("Invalid position.x"))?
                        as f32;
                    let y = obj
                        .get("y")
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| anyhow!("Invalid position.y"))?
                        as f32;
                    transform.position = Vec2::new(x, y);
                } else {
                    return Err(anyhow!("Position must be an object with x and y"));
                }
            }
            "rotation" => {
                if let Some(num) = value.as_f64() {
                    transform.rotation = num as f32;
                } else {
                    return Err(anyhow!("Rotation must be a number"));
                }
            }
            "scale" => {
                if let Some(obj) = value.as_object() {
                    let x = obj
                        .get("x")
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| anyhow!("Invalid scale.x"))?
                        as f32;
                    let y = obj
                        .get("y")
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| anyhow!("Invalid scale.y"))?
                        as f32;
                    transform.scale = Vec2::new(x, y);
                } else {
                    return Err(anyhow!("Scale must be an object with x and y"));
                }
            }
            _ => return Err(anyhow!("Unknown field: {}", field_name)),
        }

        Ok(())
    }
}

/// Helper function to register built-in component metadata.
pub fn register_builtin_metadata(registry: &mut ComponentMetadataRegistry) {
    registry.register("Transform".to_string(), Box::new(TransformMetadataHandler));
}
