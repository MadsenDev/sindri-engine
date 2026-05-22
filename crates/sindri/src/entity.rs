use crate::component::Component;
use serde::{Deserialize, Serialize};

pub type EntityId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub parent: Option<EntityId>,
    pub children: Vec<EntityId>,
    pub components: Vec<Component>,
    pub active: bool,
    #[serde(default)]
    pub staged: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefab_source: Option<String>,
}

/// Serializable tree node for a `.prefab` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefabNode {
    pub name: String,
    pub components: Vec<Component>,
    #[serde(default)]
    pub children: Vec<PrefabNode>,
}
