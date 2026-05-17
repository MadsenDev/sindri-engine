//! Entity hierarchy system for parent/child relationships.
//!
//! Provides utilities for managing entity hierarchies and computing world transforms.

use crate::entities::Transform;
use crate::math::Vec2;
use crate::world::{EntityId, World};

/// Get the parent of an entity, if it has one.
pub fn get_parent(world: &World, entity: EntityId) -> Option<EntityId> {
    world.get::<Transform>(entity).and_then(|t| t.parent)
}

/// Set the parent of an entity.
///
/// This will update the entity's Transform component to reference the parent.
/// If the entity doesn't have a Transform, one will be created.
pub fn set_parent(world: &mut World, entity: EntityId, parent: Option<EntityId>) {
    if let Some(transform) = world.get_mut::<Transform>(entity) {
        transform.parent = parent;
    } else if let Some(parent) = parent {
        // Create transform with parent
        let pos = world
            .get::<Transform>(entity)
            .map(|t| t.position)
            .unwrap_or(Vec2::ZERO);
        let mut transform = Transform::new(pos);
        transform.parent = Some(parent);
        world.insert(entity, transform);
    }
}

/// Get all children of an entity.
///
/// This searches all entities with Transform components to find those with this entity as parent.
pub fn get_children(world: &World, entity: EntityId) -> Vec<EntityId> {
    world
        .query::<Transform>()
        .into_iter()
        .filter_map(|(child_id, transform)| {
            if transform.parent == Some(entity) {
                Some(child_id)
            } else {
                None
            }
        })
        .collect()
}

/// Get the root entity in the hierarchy (the entity with no parent).
pub fn get_root(world: &World, entity: EntityId) -> EntityId {
    let mut current = entity;
    loop {
        if let Some(parent) = get_parent(world, current) {
            current = parent;
        } else {
            return current;
        }
    }
}

/// Get the world position of an entity (accounting for parent transforms).
///
/// This recursively applies parent transforms to compute the final world position.
pub fn get_world_position(world: &World, entity: EntityId) -> Vec2 {
    if let Some(transform) = world.get::<Transform>(entity) {
        let local_pos = transform.position;

        if let Some(parent) = transform.parent {
            // Get parent's world position and add local position
            let parent_world = get_world_position(world, parent);
            return parent_world + local_pos;
        }

        local_pos
    } else {
        Vec2::ZERO
    }
}

/// Get the world rotation of an entity (accounting for parent rotation).
pub fn get_world_rotation(world: &World, entity: EntityId) -> f32 {
    if let Some(transform) = world.get::<Transform>(entity) {
        let local_rot = transform.rotation;

        if let Some(parent) = transform.parent {
            // Add parent's world rotation
            let parent_world = get_world_rotation(world, parent);
            return parent_world + local_rot;
        }

        local_rot
    } else {
        0.0
    }
}

/// Get the world scale of an entity (accounting for parent scale).
pub fn get_world_scale(world: &World, entity: EntityId) -> Vec2 {
    if let Some(transform) = world.get::<Transform>(entity) {
        let local_scale = transform.scale;

        if let Some(parent) = transform.parent {
            // Multiply by parent's world scale
            let parent_world = get_world_scale(world, parent);
            return Vec2::new(
                parent_world.x * local_scale.x,
                parent_world.y * local_scale.y,
            );
        }

        local_scale
    } else {
        Vec2::new(1.0, 1.0)
    }
}

/// Reparent an entity to a new parent.
///
/// This updates the entity's Transform to reference the new parent.
/// If `new_parent` is None, the entity becomes a root entity.
pub fn reparent(world: &mut World, entity: EntityId, new_parent: Option<EntityId>) {
    set_parent(world, entity, new_parent);
}
