pub mod chat;
pub mod client;
pub mod proposal;
pub mod spritesheet;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContextFlags {
    pub include_scene: bool,
    pub include_script: bool,
    pub include_viewport: bool,
    pub include_errors: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenScriptContext {
    pub path: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct AiResponse {
    pub text: String,
    pub actions: Vec<serde_json::Value>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiStreamEvent {
    pub request_id: String,
    pub kind: String,
    pub text: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderStatus {
    pub provider: String,
    pub configured: bool,
    pub default_model: String,
}
