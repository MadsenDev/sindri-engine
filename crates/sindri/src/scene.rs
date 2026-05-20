use crate::component::Component;
use crate::entity::{Entity, EntityId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_gravity_y() -> f32 { 980.0 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scene {
    pub name: String,
    pub entities: HashMap<EntityId, Entity>,
    pub next_id: EntityId,
    #[serde(default = "default_gravity_y")]
    pub gravity_y: f32,
    #[serde(default)]
    pub gravity_x: f32,
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn spawn(&mut self, name: &str) -> EntityId {
        self.spawn_staged(name, false)
    }

    pub fn spawn_staged(&mut self, name: &str, staged: bool) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.insert(
            id,
            Entity {
                id,
                name: name.to_string(),
                parent: None,
                children: vec![],
                components: vec![],
                active: true,
                staged,
            },
        );
        id
    }

    pub fn set_parent(&mut self, child: EntityId, parent: EntityId) {
        if let Some(e) = self.entities.get_mut(&child) {
            e.parent = Some(parent);
        }
        if let Some(p) = self.entities.get_mut(&parent) {
            if !p.children.contains(&child) {
                p.children.push(child);
            }
        }
    }

    pub fn add_component(&mut self, id: EntityId, component: Component) {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.components.push(component);
        }
    }

    pub fn remove_entity(&mut self, id: EntityId) {
        if let Some(entity) = self.entities.remove(&id) {
            if let Some(parent_id) = entity.parent {
                if let Some(parent) = self.entities.get_mut(&parent_id) {
                    parent.children.retain(|&c| c != id);
                }
            }
            for child_id in &entity.children {
                if let Some(child) = self.entities.get_mut(child_id) {
                    child.parent = None;
                }
            }
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        std::fs::write(path, self.to_json()?)?;
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        Self::from_json(&json)
    }
}
