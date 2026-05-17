use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "edit_transform")]
    EditTransform {
        entity_id: u64,
        x: Option<f32>,
        y: Option<f32>,
        scale_x: Option<f32>,
        scale_y: Option<f32>,
        rotation: Option<f32>,
    },
    #[serde(rename = "write_script")]
    WriteScript { path: String, content: String },
    #[serde(rename = "create_entity")]
    CreateEntity {
        name: String,
        parent_id: Option<u64>,
    },
    #[serde(rename = "delete_entity")]
    DeleteEntity { entity_id: u64 },
    #[serde(rename = "suggest_fix")]
    SuggestFix {
        description: String,
        entity_id: Option<u64>,
    },
}
