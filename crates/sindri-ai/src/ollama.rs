use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::actions::Action;
use crate::context::AiContext;

const OLLAMA_BASE: &str = "http://localhost:11434";
const VISION_MODEL: &str = "qwen2.5-vl:7b";
const CODE_MODEL: &str = "qwen2.5-coder:7b";

const ENGINE_REFERENCE: &str = include_str!("../../../ENGINE_REFERENCE.md");

fn system_prompt() -> String {
    format!(
        r#"{engine_reference}

---

You are an AI assistant embedded in the Sindri editor. The reference above is your complete knowledge of this engine — use it precisely when writing scripts, editing scenes, or answering questions.

You have access to the current scene (as JSON), optionally an open script file, and optionally a screenshot of the game viewport.

You may respond in two ways:
1. Plain text explanation or analysis.
2. A JSON action block wrapped in <action>...</action> tags.

Action block schema:
{{
  "actions": [
    {{ "type": "edit_transform", "entity_id": 0, "x": 100, "y": 200 }},
    {{ "type": "write_script", "path": "scripts/player.lua", "content": "..." }},
    {{ "type": "create_entity", "name": "Coin", "parent_id": null }},
    {{ "type": "delete_entity", "entity_id": 3 }},
    {{ "type": "suggest_fix", "description": "Collider height doesn't match sprite height", "entity_id": 0 }}
  ]
}}

Rules:
- Scripts are Lua, not Rust. Use the Lua scripting API from the reference.
- Always respond with a brief plain-text explanation first, then the action block if applicable.
- Only include an action block when making or suggesting a concrete change.
- Keep explanations concise.
- entity_id values come from the scene JSON's "id" fields (u64 integers)."#,
        engine_reference = ENGINE_REFERENCE,
    )
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: Value,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

pub struct AiResponse {
    pub text: String,
    pub actions: Vec<Action>,
}

pub async fn send(
    client: &reqwest::Client,
    ctx: AiContext,
    user_message: &str,
) -> Result<AiResponse> {
    let use_vision = ctx.screenshot_base64.is_some();
    let model = if use_vision { VISION_MODEL } else { CODE_MODEL };

    let mut user_content_parts: Vec<Value> = Vec::new();

    // Build context text
    let mut context_text = String::new();
    context_text.push_str(&format!("Scene:\n```json\n{}\n```\n\n", ctx.scene_json));

    if let (Some(path), Some(script)) = (&ctx.open_script_path, &ctx.open_script) {
        context_text.push_str(&format!(
            "Open script ({}):\n```rust\n{}\n```\n\n",
            path, script
        ));
    }

    if let Some(errors) = &ctx.error_log {
        context_text.push_str(&format!("Errors:\n```\n{}\n```\n\n", errors));
    }

    context_text.push_str(&format!("User message: {}", user_message));

    if use_vision {
        if let Some(b64) = &ctx.screenshot_base64 {
            user_content_parts.push(serde_json::json!({
                "type": "image_url",
                "image_url": { "url": format!("data:image/png;base64,{}", b64) }
            }));
        }
        user_content_parts.push(serde_json::json!({ "type": "text", "text": context_text }));
    }

    let user_content: Value = if use_vision {
        Value::Array(user_content_parts)
    } else {
        Value::String(context_text)
    };

    let req = OllamaRequest {
        model: model.to_string(),
        messages: vec![
            OllamaMessage {
                role: "system".into(),
                content: Value::String(system_prompt()),
            },
            OllamaMessage {
                role: "user".into(),
                content: user_content,
            },
        ],
        stream: false,
    };

    let resp: OllamaResponse = client
        .post(format!("{}/api/chat", OLLAMA_BASE))
        .json(&req)
        .send()
        .await?
        .json()
        .await?;

    parse_response(&resp.message.content)
}

fn parse_response(raw: &str) -> Result<AiResponse> {
    let (text, actions) = if let Some(start) = raw.find("<action>") {
        if let Some(end) = raw.find("</action>") {
            let before = raw[..start].trim().to_string();
            let action_json = &raw[start + 8..end];
            let parsed: serde_json::Value = serde_json::from_str(action_json.trim())?;
            let actions: Vec<Action> = serde_json::from_value(
                parsed
                    .get("actions")
                    .cloned()
                    .unwrap_or(Value::Array(vec![])),
            )?;
            (before, actions)
        } else {
            (raw.to_string(), vec![])
        }
    } else {
        (raw.to_string(), vec![])
    };

    Ok(AiResponse { text, actions })
}
