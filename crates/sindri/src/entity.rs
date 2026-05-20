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
}
