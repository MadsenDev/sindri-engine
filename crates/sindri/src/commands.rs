//! Command system for undo/redo and editor operations.
//!
//! All modifications to the world should go through commands to enable:
//! - Undo/redo functionality
//! - Multi-select edits
//! - Timeline support (future)
//! - Collaboration (future)

use crate::entities::{CameraComponent, PhysicsBody, SpriteComponent, Transform};
use crate::hierarchy;
use crate::math::Vec2;
use crate::script::ScriptTag;
use crate::world::{EntityId, World};
use anyhow::{anyhow, Result};

/// A command that can be executed and undone.
pub trait Command: Send + Sync {
    /// Execute the command, modifying the world.
    fn execute(&mut self, world: &mut World) -> Result<()>;

    /// Undo the command, restoring the world to its previous state.
    fn undo(&mut self, world: &mut World) -> Result<()>;

    /// Get a description of what this command does (for UI).
    fn description(&self) -> &str;
}

/// Command to create a new entity.
#[derive(Clone, Debug)]
pub struct CreateEntity {
    entity: Option<EntityId>,
}

impl CreateEntity {
    pub fn new() -> Self {
        Self { entity: None }
    }
}

impl Command for CreateEntity {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if self.entity.is_none() {
            self.entity = Some(world.spawn());
        } else {
            if let Some(entity) = self.entity {
                if !world.is_alive(entity) {
                    world.restore_entity(entity);
                }
            }
        }
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(entity) = self.entity {
            world.despawn(entity);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Create Entity"
    }
}

impl CreateEntity {
    /// Get the created entity ID (only valid after execute).
    pub fn entity(&self) -> Option<EntityId> {
        self.entity
    }
}

/// Command to delete an entity.
///
/// Stores all components before deletion so they can be restored on undo.
#[derive(Clone, Debug)]
pub struct DeleteEntity {
    entity: EntityId,
    transform: Option<Transform>,
    sprite: Option<SpriteComponent>,
    physics: Option<PhysicsBody>,
    camera: Option<CameraComponent>,
    script_tag: Option<ScriptTag>,
    captured: bool,
}

impl DeleteEntity {
    pub fn new(entity: EntityId) -> Self {
        Self {
            entity,
            transform: None,
            sprite: None,
            physics: None,
            camera: None,
            script_tag: None,
            captured: false,
        }
    }
}

impl Command for DeleteEntity {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if !self.captured {
            self.transform = world.get::<Transform>(self.entity).cloned();
            self.sprite = world.get::<SpriteComponent>(self.entity).cloned();
            self.physics = world.get::<PhysicsBody>(self.entity).copied();
            self.camera = world.get::<CameraComponent>(self.entity).cloned();
            self.script_tag = world.get::<ScriptTag>(self.entity).cloned();
            self.captured = true;
        }

        world.despawn(self.entity);
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        world.restore_entity(self.entity);

        if let Some(transform) = &self.transform {
            world.insert(self.entity, transform.clone());
        }
        if let Some(sprite) = &self.sprite {
            world.insert(self.entity, sprite.clone());
        }
        if let Some(physics) = self.physics {
            world.insert(self.entity, physics);
        }
        if let Some(camera) = &self.camera {
            world.insert(self.entity, camera.clone());
        }
        if let Some(script_tag) = &self.script_tag {
            world.insert(self.entity, script_tag.clone());
        }

        Ok(())
    }

    fn description(&self) -> &str {
        "Delete Entity"
    }
}

/// Command to set transform component values.
#[derive(Clone, Debug)]
pub struct SetTransform {
    entity: EntityId,
    had_transform: bool,
    old_position: Option<Vec2>,
    old_rotation: Option<f32>,
    old_scale: Option<Vec2>,
    new_position: Vec2,
    new_rotation: f32,
    new_scale: Vec2,
}

/// Command to reparent an entity in the hierarchy.
#[derive(Clone, Debug)]
pub struct ReparentEntity {
    entity: EntityId,
    old_parent: Option<EntityId>,
    new_parent: Option<EntityId>,
}

impl ReparentEntity {
    pub fn new(entity: EntityId, new_parent: Option<EntityId>) -> Self {
        Self {
            entity,
            old_parent: None,
            new_parent,
        }
    }
}

impl Command for ReparentEntity {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if self.old_parent.is_none() {
            self.old_parent = hierarchy::get_parent(world, self.entity);
        }
        hierarchy::reparent(world, self.entity, self.new_parent);
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        hierarchy::reparent(world, self.entity, self.old_parent);
        Ok(())
    }

    fn description(&self) -> &str {
        "Reparent Entity"
    }
}

impl SetTransform {
    pub fn new(entity: EntityId, position: Vec2, rotation: f32, scale: Vec2) -> Self {
        Self {
            entity,
            had_transform: false,
            old_position: None,
            old_rotation: None,
            old_scale: None,
            new_position: position,
            new_rotation: rotation,
            new_scale: scale,
        }
    }
}

impl Command for SetTransform {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if self.old_position.is_none() {
            self.had_transform = world.get::<Transform>(self.entity).is_some();
        }

        if let Some(transform) = world.get_mut::<Transform>(self.entity) {
            if self.old_position.is_none() {
                self.old_position = Some(transform.position);
                self.old_rotation = Some(transform.rotation);
                self.old_scale = Some(transform.scale);
            }

            transform.position = self.new_position;
            transform.rotation = self.new_rotation;
            transform.scale = self.new_scale;
        } else {
            world.insert(
                self.entity,
                Transform::new(self.new_position)
                    .with_rotation(self.new_rotation)
                    .with_scale(self.new_scale),
            );
        }
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if self.had_transform {
            if let Some(transform) = world.get_mut::<Transform>(self.entity) {
                if let (Some(old_pos), Some(old_rot), Some(old_scale)) =
                    (self.old_position, self.old_rotation, self.old_scale)
                {
                    transform.position = old_pos;
                    transform.rotation = old_rot;
                    transform.scale = old_scale;
                }
            }
        } else {
            world.remove::<Transform>(self.entity);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Set Transform"
    }
}

/// Command to add a component to an entity.
///
/// Note: This is a simplified version that only works with Clone types.
/// For a full implementation, you'd need to serialize/deserialize components.
#[derive(Clone, Debug)]
pub struct AddComponent<T: Clone + Send + Sync + 'static> {
    entity: EntityId,
    component: T,
    had_component: bool,
    old_component: Option<T>,
}

impl<T: Clone + Send + Sync + 'static> AddComponent<T> {
    pub fn new(entity: EntityId, component: T) -> Self {
        Self {
            entity,
            component,
            had_component: false,
            old_component: None,
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Command for AddComponent<T> {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        if self.old_component.is_none() {
            self.old_component = world.get::<T>(self.entity).cloned();
            self.had_component = self.old_component.is_some();
        }

        world.insert(self.entity, self.component.clone());
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if self.had_component {
            if let Some(old_component) = &self.old_component {
                world.insert(self.entity, old_component.clone());
            }
        } else {
            world.remove::<T>(self.entity);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Add Component"
    }
}

/// Command to remove a component from an entity.
#[derive(Clone, Debug)]
pub struct RemoveComponent<T: Clone + Send + Sync + 'static> {
    entity: EntityId,
    component: Option<T>,
}

impl<T: Clone + Send + Sync + 'static> RemoveComponent<T> {
    pub fn new(entity: EntityId) -> Self {
        Self {
            entity,
            component: None,
        }
    }
}

impl<T: Clone + Send + Sync + 'static> Command for RemoveComponent<T> {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        let removed = world.remove::<T>(self.entity);
        if self.component.is_none() {
            self.component = removed.clone();
        }
        Ok(())
    }

    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(component) = &self.component {
            world.insert(self.entity, component.clone());
        }
        Ok(())
    }

    fn description(&self) -> &str {
        "Remove Component"
    }
}

/// Command history manager for undo/redo.
pub struct CommandHistory {
    history: Vec<Box<dyn Command>>,
    current_index: usize,
    max_history: usize,
}

impl CommandHistory {
    /// Create a new command history with a maximum depth.
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            current_index: 0,
            max_history,
        }
    }

    /// Execute a command and add it to history.
    pub fn execute(&mut self, mut command: Box<dyn Command>, world: &mut World) -> Result<()> {
        if self.current_index < self.history.len() {
            self.history.truncate(self.current_index);
        }

        command.execute(world)?;
        self.history.push(command);

        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
        self.current_index = self.history.len();

        Ok(())
    }

    /// Undo the last command.
    pub fn undo(&mut self, world: &mut World) -> Result<()> {
        if self.current_index == 0 {
            return Err(anyhow!("Nothing to undo"));
        }

        self.current_index -= 1;
        if let Some(command) = self.history.get_mut(self.current_index) {
            command.undo(world)?;
        }

        Ok(())
    }

    /// Redo the next command.
    pub fn redo(&mut self, world: &mut World) -> Result<()> {
        if self.current_index >= self.history.len() {
            return Err(anyhow!("Nothing to redo"));
        }

        if let Some(command) = self.history.get_mut(self.current_index) {
            command.execute(world)?;
        }

        self.current_index += 1;
        Ok(())
    }

    /// Check if undo is possible.
    pub fn can_undo(&self) -> bool {
        self.current_index > 0
    }

    /// Check if redo is possible.
    pub fn can_redo(&self) -> bool {
        self.current_index < self.history.len()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.history.clear();
        self.current_index = 0;
    }

    /// Get the number of commands in history.
    pub fn len(&self) -> usize {
        self.history.len()
    }
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new(100) // Default to 100 commands
    }
}
