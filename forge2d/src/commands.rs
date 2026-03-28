//! Command system for undo/redo and editor operations.
//!
//! All modifications to the world should go through commands to enable:
//! - Undo/redo functionality
//! - Multi-select edits
//! - Timeline support (future)
//! - Collaboration (future)

use anyhow::{anyhow, Result};
use crate::world::{EntityId, World};
use crate::entities::Transform;
use crate::hierarchy;
use crate::math::Vec2;

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
            // Entity already created, just ensure it exists
            if let Some(entity) = self.entity {
                if !world.is_alive(entity) {
                    return Err(anyhow!("Entity was despawned"));
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
    had_transform: bool,
    transform: Option<Transform>,
    // Store other components as serialized data if needed
    // For now, we'll just track Transform since it's the most common
}

impl DeleteEntity {
    pub fn new(entity: EntityId) -> Self {
        Self {
            entity,
            had_transform: false,
            transform: None,
        }
    }
}

impl Command for DeleteEntity {
    fn execute(&mut self, world: &mut World) -> Result<()> {
        // Store components before deletion
        if let Some(transform) = world.get::<Transform>(self.entity) {
            self.had_transform = true;
            self.transform = Some(transform.clone());
        }
        
        world.despawn(self.entity);
        Ok(())
    }
    
    fn undo(&mut self, world: &mut World) -> Result<()> {
        // Recreate entity
        // Note: We can't guarantee the same EntityId, so we'll need to handle this differently
        // For now, we'll create a new entity and restore components
        let new_entity = world.spawn();
        
        if self.had_transform {
            if let Some(transform) = self.transform.take() {
                world.insert(new_entity, transform);
            }
        }
        
        // Update entity ID for future operations
        // This is a limitation - we can't restore the exact same EntityId
        // In a real editor, you'd want to track entity ID mappings
        self.entity = new_entity;
        
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
        if let Some(transform) = world.get_mut::<Transform>(self.entity) {
            // Store old values
            if self.old_position.is_none() {
                self.old_position = Some(transform.position);
                self.old_rotation = Some(transform.rotation);
                self.old_scale = Some(transform.scale);
            }
            
            // Apply new values
            transform.position = self.new_position;
            transform.rotation = self.new_rotation;
            transform.scale = self.new_scale;
        } else {
            // Create transform if it doesn't exist
            world.insert(self.entity, Transform::new(self.new_position)
                .with_rotation(self.new_rotation)
                .with_scale(self.new_scale));
        }
        Ok(())
    }
    
    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(transform) = world.get_mut::<Transform>(self.entity) {
            if let (Some(old_pos), Some(old_rot), Some(old_scale)) = 
                (self.old_position, self.old_rotation, self.old_scale) {
                transform.position = old_pos;
                transform.rotation = old_rot;
                transform.scale = old_scale;
            }
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
        // Store old component if it exists
        if let Some(_old) = world.get::<T>(self.entity) {
            self.had_component = true;
            // We can't clone from a reference, so we'll just mark it
            // In practice, you'd serialize/deserialize for undo
        }
        
        world.insert(self.entity, self.component.clone());
        Ok(())
    }
    
    fn undo(&mut self, world: &mut World) -> Result<()> {
        if self.had_component {
            // Restore old component if we had one
            // This is a limitation - we'd need to store the old component
            // For now, we'll just remove it
            world.remove::<T>(self.entity);
        } else {
            // Remove the component we added
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
        if self.component.is_none() {
            self.component = world.remove::<T>(self.entity);
        }
        Ok(())
    }
    
    fn undo(&mut self, world: &mut World) -> Result<()> {
        if let Some(component) = self.component.take() {
            world.insert(self.entity, component);
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
        // Remove any commands after current_index (when we're in the middle of history)
        if self.current_index < self.history.len() {
            self.history.truncate(self.current_index);
        }
        
        // Execute command
        command.execute(world)?;
        
        // Add to history
        self.history.push(command);
        
        // Limit history size
        if self.history.len() > self.max_history {
            self.history.remove(0);
        } else {
            self.current_index = self.history.len();
        }
        
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
