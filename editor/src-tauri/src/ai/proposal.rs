use serde::{Deserialize, Serialize};

use super::client::{default_cloud_model, request_ai_json_text};
use super::spritesheet::analyze_spritesheet_for_message;
use super::{ContextFlags, OpenScriptContext};
use crate::engine_client::engine_url;

#[derive(Serialize, Deserialize, Clone)]
pub struct ScriptBackup {
    pub path: String,
    pub existed: bool,
    pub content: String,
}

#[derive(Serialize)]
pub struct ScriptNewContent {
    pub path: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ProposalChange {
    pub id: String,
    pub label: String,
    pub detail: String,
    /// Entity IDs that were newly created (staged=true) by this change — deleted on reject.
    pub staged_entity_ids: Vec<u64>,
    /// Entity IDs of pre-existing entities modified by this change — just unstaged on reject.
    pub modified_entity_ids: Vec<u64>,
    /// Script paths written by this change (for cleanup on reject).
    pub new_script_paths: Vec<String>,
    /// Previous script contents so reject can restore instead of deleting user files.
    pub script_backups: Vec<ScriptBackup>,
    /// New script contents for diff display in the proposals panel.
    pub script_new_contents: Vec<ScriptNewContent>,
}

#[derive(Serialize)]
pub struct ProposalResponse {
    pub prompt: String,
    pub summary: String,
    pub changes: Vec<ProposalChange>,
}

// ── Entity resolution helpers ─────────────────────────────────────────────────

async fn fetch_scene(client: &reqwest::Client) -> Result<serde_json::Value, String> {
    client
        .get(engine_url("/scene"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

async fn find_entity_id_by_name(
    client: &reqwest::Client,
    name: &str,
) -> Result<Option<u64>, String> {
    let scene = fetch_scene(client).await?;
    Ok(scene["entities"].as_object().and_then(|entities| {
        entities
            .iter()
            .filter_map(|(id_str, entity)| {
                if entity["name"].as_str() == Some(name) {
                    id_str.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .max()
    }))
}

async fn entity_components(
    client: &reqwest::Client,
    entity_id: u64,
) -> Result<Vec<serde_json::Value>, String> {
    let scene = fetch_scene(client).await?;
    let entity_id_str = entity_id.to_string();
    Ok(scene["entities"][&entity_id_str]["components"]
        .as_array()
        .cloned()
        .unwrap_or_default())
}

fn component_type(component: &serde_json::Value) -> Option<&str> {
    component["type"].as_str()
}

async fn has_component(
    client: &reqwest::Client,
    entity_id: u64,
    wanted_type: &str,
) -> Result<bool, String> {
    Ok(entity_components(client, entity_id)
        .await?
        .iter()
        .any(|component| component_type(component) == Some(wanted_type)))
}

async fn ensure_component(
    client: &reqwest::Client,
    entity_id: u64,
    component_type: &str,
) -> Result<(), String> {
    if has_component(client, entity_id, component_type).await? {
        return Ok(());
    }

    client
        .post(engine_url(&format!(
            "/scene/entity/{}/component",
            entity_id
        )))
        .json(&serde_json::json!({ "component_type": component_type }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn resolve_entity_id(
    action: &serde_json::Value,
    client: &reqwest::Client,
) -> Result<u64, String> {
    if let Some(id) = action["entity_id"].as_u64() {
        return Ok(id);
    }
    if let Some(name) = action["entity_name"].as_str() {
        if let Some(id) = find_entity_id_by_name(client, name).await? {
            return Ok(id);
        }
        return Err(format!("No entity found with name '{}'", name));
    }
    Err("missing entity_id or entity_name".to_string())
}

// ── Script helpers ────────────────────────────────────────────────────────────

pub(crate) async fn read_script_backup(client: &reqwest::Client, path: &str) -> ScriptBackup {
    match client
        .get(engine_url(&format!("/script?path={}", path)))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => ScriptBackup {
            path: path.to_string(),
            existed: true,
            content: resp.text().await.unwrap_or_default(),
        },
        _ => ScriptBackup {
            path: path.to_string(),
            existed: false,
            content: String::new(),
        },
    }
}

fn is_placeholder_content(content: &str) -> bool {
    let t = content.trim();
    t.is_empty()
        || t == "..."
        || t == "-- ..."
        || t == "--..."
        || t.starts_with("-- TODO")
        || (t.len() < 10 && t.chars().all(|c| c == '.' || c == '-' || c == ' '))
}

// ── Fallback heuristics ───────────────────────────────────────────────────────

fn looks_like_edit_request(message: &str) -> bool {
    if message.contains('@') {
        return true;
    }
    let lower = message.to_lowercase();
    [
        "add ",
        "make ",
        "create ",
        "update ",
        "change ",
        "fix ",
        "remove ",
        "delete ",
        "attach ",
        "write ",
        "set ",
        "move ",
        "rename ",
        "jump",
        "script",
        "connect",
        "rotate with",
        "follow",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn extract_json_object(raw_text: &str) -> Option<&str> {
    let raw_after_think = if let Some(end) = raw_text.find("</think>") {
        raw_text[end + 8..].trim()
    } else {
        raw_text.trim()
    };

    let stripped = raw_after_think
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let start = stripped.find('{')?;
    let end = stripped.rfind('}').map(|i| i + 1)?;
    (start < end).then_some(&stripped[start..end])
}

fn looks_like_player_jump_request(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("player") && lower.contains("jump")
}

async fn stage_player_jump_fallback(
    client: &reqwest::Client,
    scene_json: Option<&str>,
    open_script: Option<&OpenScriptContext>,
) -> Result<Option<ProposalChange>, String> {
    let scene =
        match scene_json.and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok()) {
            Some(scene) => scene,
            None => fetch_scene(client).await?,
        };

    let Some(player) = scene["entities"].as_object().and_then(|entities| {
        entities.values().find(|entity| {
            entity["name"]
                .as_str()
                .map(|name| name.eq_ignore_ascii_case("player"))
                .unwrap_or(false)
        })
    }) else {
        return Ok(None);
    };

    let Some(player_id) = player["id"].as_u64() else {
        return Ok(None);
    };

    let script_path = player["components"]
        .as_array()
        .and_then(|components| {
            components.iter().find_map(|component| {
                if component["type"].as_str() == Some("Script") {
                    component["path"].as_str().filter(|path| !path.is_empty())
                } else {
                    None
                }
            })
        })
        .map(str::to_string)
        .or_else(|| open_script.map(|script| script.path.clone()))
        .unwrap_or_else(|| "scripts/player.lua".to_string());

    let backup = read_script_backup(client, &script_path).await;
    let content = player_jump_script();
    client
        .put(engine_url(&format!("/script?path={}", script_path)))
        .json(&serde_json::json!({ "content": content }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    ensure_component(client, player_id, "Script").await?;
    let components = entity_components(client, player_id).await?;
    for (idx, component) in components.iter().enumerate() {
        if component["type"].as_str() == Some("Script") {
            client
                .patch(engine_url(&format!(
                    "/scene/entity/{}/component/{}",
                    player_id, idx
                )))
                .json(&serde_json::json!({ "path": script_path }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            break;
        }
    }

    let _ = client
        .patch(engine_url(&format!("/scene/entity/{}/staged", player_id)))
        .json(&serde_json::json!({ "staged": true }))
        .send()
        .await;

    Ok(Some(ProposalChange {
        id: "jump-script".to_string(),
        label: "Add player jump controls".to_string(),
        detail: "Updates the player Lua script with horizontal movement and Space-to-jump using the Sindri input and physics facets.".to_string(),
        staged_entity_ids: Vec::new(),
        modified_entity_ids: vec![player_id],
        new_script_paths: vec![script_path.clone()],
        script_backups: vec![backup],
        script_new_contents: vec![ScriptNewContent { path: script_path, content: content.to_string() }],
    }))
}

fn player_jump_script() -> &'static str {
    r#"local speed = 220.0
local jump_speed = 420.0
local grounded_y = nil
local was_grounded = false

function on_update(self, dt)
  local input = self:input()
  local transform = self:transform()
  local physics = self:physics()
  if input == nil or transform == nil or physics == nil then
    return
  end

  local pos = transform:position()
  if grounded_y == nil then
    grounded_y = pos.y
  end

  local velocity = physics:velocity()
  local horizontal = input:axis("A", "D")
  local grounded = pos.y >= grounded_y - 2.0 and velocity.y >= -1.0

  physics:set_velocity(vec2(horizontal * speed, velocity.y))

  if grounded and input:is_key_pressed("Space") then
    physics:set_velocity(vec2(horizontal * speed, -jump_speed))
    was_grounded = false
  else
    was_grounded = grounded
  end
end
"#
}

// ── apply_action command ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn apply_action(action: serde_json::Value) -> Result<(), String> {
    let client = reqwest::Client::new();
    let action_type = action["type"].as_str().unwrap_or("");

    match action_type {
        "edit_transform" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let body = serde_json::json!({
                "x": action["x"],
                "y": action["y"],
                "scale_x": action["scale_x"],
                "scale_y": action["scale_y"],
                "rotation": action["rotation"],
            });
            client
                .patch(engine_url(&format!(
                    "/scene/entity/{}/transform",
                    entity_id
                )))
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "create_entity" => {
            let name = action["name"].as_str().ok_or("missing name")?;
            if find_entity_id_by_name(&client, name).await?.is_some() {
                return Ok(());
            }

            let body = serde_json::json!({
                "name": action["name"],
                "parent_id": action["parent_id"],
            });
            client
                .post(engine_url("/scene/entity"))
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "delete_entity" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            client
                .delete(engine_url(&format!("/scene/entity/{}", entity_id)))
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "write_script" => {
            let path = action["path"].as_str().ok_or("missing path")?;
            let content = action["content"].as_str().ok_or(
                "write_script action is missing a 'content' field — the model did not generate script code"
            )?;
            if is_placeholder_content(content) {
                return Err(format!(
                    "write_script for '{path}' has placeholder content (\"...\"). The model did not return real code. Nothing was written."
                ));
            }
            client
                .put(engine_url(&format!("/script?path={}", path)))
                .json(&serde_json::json!({ "content": content }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "rename_entity" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let name = action["name"].as_str().ok_or("missing name")?;
            client
                .patch(engine_url(&format!("/scene/entity/{}/name", entity_id)))
                .json(&serde_json::json!({ "name": name }))
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "add_component" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let component_type = action["component_type"]
                .as_str()
                .ok_or("missing component_type")?;
            ensure_component(&client, entity_id, component_type).await?;
        }
        "remove_component" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            if let Some(idx) = action["component_idx"].as_u64() {
                client
                    .delete(engine_url(&format!(
                        "/scene/entity/{}/component/{}",
                        entity_id, idx
                    )))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
            } else {
                let component_type = action["component_type"]
                    .as_str()
                    .ok_or("missing component_type or component_idx")?;
                client
                    .delete(engine_url(&format!(
                        "/scene/entity/{}/component?component_type={}",
                        entity_id, component_type
                    )))
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }
        "patch_component" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let component_idx = if let Some(idx) = action["component_idx"].as_u64() {
                idx as usize
            } else {
                let component_type = action["component_type"]
                    .as_str()
                    .ok_or("missing component_idx or component_type")?;
                entity_components(&client, entity_id)
                    .await?
                    .iter()
                    .position(|component| component["type"].as_str() == Some(component_type))
                    .ok_or_else(|| format!("entity has no {component_type} component"))?
            };
            client
                .patch(engine_url(&format!(
                    "/scene/entity/{}/component/{}",
                    entity_id, component_idx
                )))
                .json(&action["data"])
                .send()
                .await
                .map_err(|e| e.to_string())?;
        }
        "attach_script" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let path = action["path"].as_str().ok_or("missing path")?;
            let content = action["content"].as_str().ok_or(
                "attach_script action is missing a 'content' field — the model did not generate script code"
            )?;
            if is_placeholder_content(content) {
                return Err(format!(
                    "attach_script for '{path}' has placeholder content (\"...\"). The model did not return real code. Nothing was written."
                ));
            }

            client
                .put(engine_url(&format!("/script?path={}", path)))
                .json(&serde_json::json!({ "content": content }))
                .send()
                .await
                .map_err(|e| e.to_string())?;

            ensure_component(&client, entity_id, "Script").await?;

            let components = entity_components(&client, entity_id).await?;
            {
                for (idx, comp) in components.iter().enumerate() {
                    let script_path = comp["path"].as_str().unwrap_or("");
                    if comp["type"] == "Script" && (script_path.is_empty() || script_path == path) {
                        client
                            .patch(engine_url(&format!(
                                "/scene/entity/{}/component/{}",
                                entity_id, idx
                            )))
                            .json(&serde_json::json!({ "path": path }))
                            .send()
                            .await
                            .map_err(|e| e.to_string())?;
                        break;
                    }
                }
            }
        }
        _ => {}
    }

    Ok(())
}

// ── generate_proposal command ─────────────────────────────────────────────────

#[tauri::command]
pub async fn generate_proposal(
    message: String,
    context_flags: ContextFlags,
    provider: Option<String>,
    model: Option<String>,
    history: Vec<serde_json::Value>,
    open_script: Option<OpenScriptContext>,
) -> Result<ProposalResponse, String> {
    let client = reqwest::Client::new();

    let scene = if context_flags.include_scene {
        match reqwest::get(engine_url("/scene")).await {
            Ok(r) => r.text().await.ok(),
            Err(_) => None,
        }
    } else {
        None
    };

    let screenshot = if context_flags.include_viewport {
        let resp: Option<serde_json::Value> = match reqwest::get(engine_url("/screenshot")).await {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        };
        resp.and_then(|v| v["image"].as_str().map(String::from))
    } else {
        None
    };

    let errors = if context_flags.include_errors {
        let resp: Option<serde_json::Value> = match reqwest::get(engine_url("/errors")).await {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        };
        resp.and_then(|v| {
            v["errors"].as_array().map(|errs| {
                errs.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        })
    } else {
        None
    };

    let is_animation_request = {
        let lower = message.to_lowercase();
        lower.contains("anim")
            || lower.contains("sprite")
            || lower.contains("clip")
            || lower.contains("frame")
            || lower.contains("spritesheet")
    };
    let spritesheet = if is_animation_request {
        analyze_spritesheet_for_message(&message, scene.as_deref()).await
    } else {
        None
    };

    let use_vision = screenshot.is_some();
    let provider = provider.unwrap_or_else(|| "ollama".to_string());
    let model = model.unwrap_or_else(|| {
        if provider == "ollama" {
            if use_vision {
                "qwen2.5-vl:7b"
            } else {
                "qwen2.5-coder:7b"
            }
            .to_string()
        } else {
            default_cloud_model(&provider).to_string()
        }
    });

    let spritesheet_guidance = if let Some(ref ss) = spritesheet {
        let row_desc: Vec<String> = ss
            .frames_per_row
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                let start = i as u32 * ss.cols;
                let end = start + n - 1;
                format!(
                    "  row {i}: {n} frames (start_frame={start}, end_frame={end})",
                    i = i,
                    n = n,
                    start = start,
                    end = end
                )
            })
            .collect();
        format!(
            "\n\nSpritesheet pixel analysis for {path}:\
            \n- Grid: {cols} cols × {rows} rows, each frame {fw}×{fh}px\
            \n- Non-empty frame ranges per row:\n{rows_desc}\
            \nUse these exact values for cols, rows, start_frame, and end_frame in the AnimatedSprite patch.\
            \nName clips based on the texture filename, frame counts, and typical game animation conventions\
            \n(idle, walk, run, attack, hurt, death, etc.). Choose fps: idle 6–8, walk 8–10, run 10–12, attack 10–14, death 6–8.",
            path = ss.tex_path,
            cols = ss.cols,
            rows = ss.rows,
            fw = ss.frame_width,
            fh = ss.frame_height,
            rows_desc = row_desc.join("\n"),
        )
    } else {
        String::new()
    };

    let engine_ref = include_str!("../../../../ENGINE_REFERENCE.md");
    let system_prompt = format!(
        r#"{engine_ref}

---

You are an AI assistant embedded in the Sindri editor. Respond with a SINGLE JSON object — no markdown fences, no prose outside the JSON.{spritesheet_guidance}

Format:
{{
  "summary": "One sentence describing the proposed changes (or your answer if no changes).",
  "changes": [
    {{
      "id": "c1",
      "label": "Short human-readable title (e.g. 'Create Player entity')",
      "detail": "What this specific change does and why.",
      "actions": [
        {{ ... action object ... }},
        {{ ... action object ... }}
      ]
    }}
  ]
}}

If the user is asking a question with no scene changes, use "changes": [].
If the user asks you to add, fix, update, remove, create, or make something, you MUST include at least one concrete action in "changes". Never claim that you changed something unless "changes" contains the action that performs it.

Each change groups ALL actions needed for one logical operation. A "Create Player" change needs multiple actions in its "actions" array — create entity, add every component, set script, etc.

Action types — use as many as needed in one change's "actions" array:
{{ "type": "create_entity", "name": "Coin", "parent_id": null }}
{{ "type": "delete_entity", "entity_id": 3 }}
{{ "type": "rename_entity", "entity_id": 1, "name": "Player" }}
{{ "type": "add_component", "entity_name": "Player", "component_type": "Transform" }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "Transform", "data": {{ "x": 640, "y": 490 }} }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "PhysicsBody", "data": {{ "body_type": "Dynamic", "lock_rotation": true }} }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "Sprite", "data": {{ "width": 28, "height": 40, "color": [0.3, 0.75, 0.4, 1.0] }} }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "Collider", "data": {{ "width": 28, "height": 40 }} }}
{{ "type": "remove_component", "entity_id": 1, "component_type": "Script" }}
{{ "type": "edit_transform", "entity_name": "Player", "x": 400, "y": 260, "scale_x": 1.0, "scale_y": 1.0, "rotation": 0.0 }}
{{ "type": "write_script", "path": "scripts/player.lua", "content": "-- FULL script content here, never use '...'" }}
{{ "type": "attach_script", "entity_name": "Player", "path": "scripts/player.lua", "content": "-- FULL script content here, never use '...'" }}

Rules:
- CRITICAL: write_script and attach_script MUST include the complete Lua code in "content". Never use "..." or leave it empty. If content is missing or placeholder the action will be rejected and the file will not be written.
- Use entity_name for entities created in the same change; use entity_id for existing entities.
- Coordinate system: +X right, +Y down.
- Physics needs both PhysicsBody and Collider. body_type: "Dynamic" (players/enemies), "Fixed" (ground/walls), "Kinematic" (scripted platforms). lock_rotation=true for platformer players.
- Supported component types: Transform, Sprite, AnimatedSprite, Tilemap, PhysicsBody, Collider, Script, Camera, AudioSource.
- AnimatedSprite clip schema: each clip is {{name, start_frame, end_frame, fps, looping}}. Frames are 0-indexed, numbered left-to-right row-by-row across the spritesheet (cols × rows grid). Example: 8-col sheet, row 0 = frames 0–7, row 1 = frames 8–15. Always set default_clip to the idle/rest clip name.
- Scripts use the REAL Sindri Lua API — NOT globals like key_down(). Use: self:input():is_key_down("A"), self:transform():set_position(vec2(x,y)), self:physics():set_velocity(vec2(vx,vy)), self:sprite():set_tint({{r,g,b,a}}). Always nil-check facets before use.
- AnimatedSpriteFacet: `local anim = self:animated_sprite()` — nil if no AnimatedSprite component. `anim:play("clip_name")` switches the active clip (idempotent — safe to call every frame). `anim:set_flip_x(bool)`, `anim:set_flip_y(bool)`, `anim:current_clip()` → string. Walk pattern: `local anim = self:animated_sprite(); if anim ~= nil then if h > 0.1 then anim:play("walk_right") elseif h < -0.1 then anim:play("walk_left") else anim:play("idle") end end`
- Cross-entity queries: `entity_transform("Name")` → `{{x, y, rotation}}` or nil (snapshot from this frame). `self:physics():contacts()` → array of entity name strings currently touching this entity's collider.
- ATTACH/FOLLOW PATTERN — to make entity A ride/follow entity B: in A's script, check `self:physics():contacts()` for B's name, then use `entity_transform("B")` to read B's position/rotation, then `self:transform():set_position(...)` and `self:transform():set_rotation(...)` to snap A onto B with an offset. Example: `local contacts = self:physics():contacts(); for _, name in ipairs(contacts) do if name == "Drone" then local dt_b = entity_transform("Drone"); if dt_b then self:transform():set_rotation(dt_b.rotation); self:transform():set_position(vec2(dt_b.x, dt_b.y - 32)); end end end`
- PLAYER TEMPLATE — "actions" must include: create_entity, add+patch Transform, add+patch Sprite (color/size), add+patch PhysicsBody (Dynamic, lock_rotation true), add+patch Collider (same size as sprite), attach_script with full Lua movement code.
- Return ONLY the JSON object."#,
        engine_ref = engine_ref,
    );

    let mut context_text = String::new();
    if let Some(s) = &scene {
        context_text.push_str(&format!("Scene:\n```json\n{}\n```\n\n", s));
    }
    if let Some(script) = &open_script {
        if context_flags.include_script {
            context_text.push_str(&format!(
                "Open script ({path}):\n```lua\n{content}\n```\n\n",
                path = script.path,
                content = script.content
            ));
        }
    }
    if let Some(errors) = &errors {
        if !errors.is_empty() {
            context_text.push_str(&format!("Errors:\n```\n{}\n```\n\n", errors));
        }
    }
    context_text.push_str(&format!("User message: {}", message));

    let user_content: serde_json::Value = if use_vision {
        serde_json::json!([
            { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{}", screenshot.unwrap_or_default()) } },
            { "type": "text", "text": context_text }
        ])
    } else {
        serde_json::json!(context_text)
    };

    let mut messages: Vec<serde_json::Value> =
        vec![serde_json::json!({ "role": "system", "content": system_prompt })];
    let recent_history = if history.len() > 10 {
        &history[history.len() - 10..]
    } else {
        &history[..]
    };
    messages.extend_from_slice(recent_history);
    messages.push(serde_json::json!({ "role": "user", "content": user_content }));

    let raw_text = request_ai_json_text(&client, &provider, &model, messages).await?;
    if raw_text.trim().is_empty() {
        return Err(format!(
            "{provider} returned an empty proposal response for model `{model}`"
        ));
    }

    let Some(json_slice) = extract_json_object(&raw_text) else {
        if !looks_like_edit_request(&message) {
            return Ok(ProposalResponse {
                prompt: message,
                summary: raw_text.trim().to_string(),
                changes: Vec::new(),
            });
        }
        let preview: String = raw_text.chars().take(500).collect();
        return Err(format!(
            "AI response did not include proposal JSON for that edit request. Response preview: {preview}"
        ));
    };

    let parsed: serde_json::Value = serde_json::from_str(json_slice).map_err(|err| {
        let preview: String = raw_text.chars().take(500).collect();
        format!("AI response was not valid proposal JSON: {err}. Response preview: {preview}")
    })?;

    let mut summary = parsed["summary"].as_str().unwrap_or("").trim().to_string();
    let changes_raw = parsed["changes"].as_array().cloned().unwrap_or_default();

    let mut changes: Vec<ProposalChange> = Vec::new();
    for (i, c) in changes_raw.iter().enumerate() {
        let id = c["id"]
            .as_str()
            .unwrap_or(&format!("c{}", i + 1))
            .trim()
            .to_string();
        let label = c["label"].as_str().unwrap_or("Change").trim().to_string();
        let detail = c["detail"].as_str().unwrap_or("").trim().to_string();

        let actions: Vec<serde_json::Value> = if let Some(arr) = c["actions"].as_array() {
            arr.clone()
        } else if !c["action"].is_null() {
            vec![c["action"].clone()]
        } else {
            vec![]
        };
        if actions.is_empty() {
            continue;
        }

        let mut staged_entity_ids: Vec<u64> = Vec::new();
        let mut modified_entity_ids: Vec<u64> = Vec::new();
        let mut new_script_paths: Vec<String> = Vec::new();
        let mut script_backups: Vec<ScriptBackup> = Vec::new();
        let mut script_new_contents: Vec<ScriptNewContent> = Vec::new();
        let mut applied_any = false;

        for action in &actions {
            let action_type = action["type"].as_str().unwrap_or("");
            if action_type.is_empty() {
                continue;
            }

            if action_type == "create_entity" {
                let name = action["name"].as_str().unwrap_or("Entity");
                if find_entity_id_by_name(&client, name).await?.is_none() {
                    let resp: serde_json::Value = client
                        .post(engine_url("/scene/entity"))
                        .json(&serde_json::json!({
                            "name": name,
                            "parent_id": action["parent_id"],
                            "staged": true,
                        }))
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .json()
                        .await
                        .map_err(|e| e.to_string())?;
                    if let Some(id_val) = resp["id"].as_u64() {
                        staged_entity_ids.push(id_val);
                        applied_any = true;
                    }
                }
            } else {
                let supported_action = matches!(
                    action_type,
                    "edit_transform"
                        | "delete_entity"
                        | "write_script"
                        | "rename_entity"
                        | "add_component"
                        | "remove_component"
                        | "patch_component"
                        | "attach_script"
                );
                if !supported_action {
                    eprintln!("[generate_proposal] skipped unsupported action type: {action_type}");
                    continue;
                }

                if action_type == "write_script" || action_type == "attach_script" {
                    if let Some(path) = action["path"].as_str() {
                        if !new_script_paths.iter().any(|existing| existing == path) {
                            new_script_paths.push(path.to_string());
                        }
                        if !script_backups.iter().any(|backup| backup.path == path) {
                            script_backups.push(read_script_backup(&client, path).await);
                        }
                    }
                }
                apply_action(action.clone())
                    .await
                    .map_err(|err| format!("failed to apply {action_type}: {err}"))?;
                applied_any = true;

                if action_type == "write_script" || action_type == "attach_script" {
                    if let Some(path) = action["path"].as_str() {
                        if !script_new_contents.iter().any(|c| c.path == path) {
                            let new_content = client
                                .get(engine_url(&format!("/script?path={}", path)))
                                .send()
                                .await
                                .ok()
                                .and_then(|r| {
                                    if r.status().is_success() {
                                        Some(r)
                                    } else {
                                        None
                                    }
                                });
                            let content = if let Some(r) = new_content {
                                r.text().await.unwrap_or_default()
                            } else {
                                action["content"].as_str().unwrap_or("").to_string()
                            };
                            script_new_contents.push(ScriptNewContent {
                                path: path.to_string(),
                                content,
                            });
                        }
                    }
                }

                let modifies_entity = matches!(
                    action_type,
                    "add_component"
                        | "remove_component"
                        | "patch_component"
                        | "edit_transform"
                        | "rename_entity"
                        | "attach_script"
                );
                if modifies_entity {
                    if let Ok(entity_id) = resolve_entity_id(action, &client).await {
                        if !staged_entity_ids.contains(&entity_id)
                            && !modified_entity_ids.contains(&entity_id)
                        {
                            modified_entity_ids.push(entity_id);
                            let _ = client
                                .patch(engine_url(&format!("/scene/entity/{}/staged", entity_id)))
                                .json(&serde_json::json!({ "staged": true }))
                                .send()
                                .await;
                        }
                    }
                }
            }
        }

        if !applied_any
            && staged_entity_ids.is_empty()
            && modified_entity_ids.is_empty()
            && new_script_paths.is_empty()
        {
            continue;
        }

        changes.push(ProposalChange {
            id,
            label,
            detail,
            staged_entity_ids,
            modified_entity_ids,
            new_script_paths,
            script_backups,
            script_new_contents,
        });
    }

    if changes.is_empty() && looks_like_player_jump_request(&message) {
        if let Some(fallback) =
            stage_player_jump_fallback(&client, scene.as_deref(), open_script.as_ref()).await?
        {
            summary = "Staged jump controls for the player script.".to_string();
            changes.push(fallback);
        }
    }

    if changes.is_empty() && looks_like_edit_request(&message) {
        return Err(
            "The model did not return any staged actions for that edit request. Nothing was changed. Try again with a more specific target, or switch the Code model in AI settings."
                .to_string(),
        );
    }

    if summary.is_empty() {
        summary = if let Some(change) = changes.first() {
            if !change.label.trim().is_empty() && change.label != "Change" {
                change.label.clone()
            } else if !change.detail.trim().is_empty() {
                change.detail.clone()
            } else {
                format!(
                    "Staged {} proposed change{}.",
                    changes.len(),
                    if changes.len() == 1 { "" } else { "s" }
                )
            }
        } else {
            "No changes proposed.".to_string()
        };
    }

    let proposal_json = serde_json::to_string_pretty(&serde_json::json!({
        "prompt": message,
        "summary": summary,
        "changes": changes.iter().map(|c| serde_json::json!({
            "id": c.id,
            "label": c.label,
            "detail": c.detail,
            "staged_entity_ids": c.staged_entity_ids,
            "modified_entity_ids": c.modified_entity_ids,
            "new_script_paths": c.new_script_paths,
            "script_backups": c.script_backups.iter().map(|b| serde_json::json!({
                "path": b.path,
                "existed": b.existed,
                "content": b.content,
            })).collect::<Vec<_>>(),
            "script_new_contents": c.script_new_contents.iter().map(|s| serde_json::json!({
                "path": s.path,
                "content": s.content,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    }))
    .unwrap_or_default();
    if let Ok(resp) = reqwest::get(engine_url("/scene/path")).await {
        if let Ok(scene_path) = resp.text().await {
            let proposal_path = format!("{}.proposal.json", scene_path.trim_matches('"'));
            let _ = std::fs::write(&proposal_path, &proposal_json);
        }
    }

    Ok(ProposalResponse {
        prompt: message,
        summary,
        changes,
    })
}

// ── Staged change management ──────────────────────────────────────────────────

#[tauri::command]
pub async fn commit_staged_change(
    entity_ids: Vec<u64>,
    modified_entity_ids: Vec<u64>,
    script_paths: Vec<String>,
    script_backups: Vec<ScriptBackup>,
    change_id: Option<String>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let all_ids: Vec<u64> = entity_ids
        .iter()
        .chain(modified_entity_ids.iter())
        .copied()
        .collect();
    if !all_ids.is_empty() {
        client
            .post(engine_url("/scene/staged/commit"))
            .json(&serde_json::json!({ "entity_ids": all_ids }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }
    let _ = script_paths;
    let _ = script_backups;
    remove_change_from_proposal(change_id.as_deref()).await;
    Ok(())
}

#[tauri::command]
pub async fn revert_staged_change(
    entity_ids: Vec<u64>,
    modified_entity_ids: Vec<u64>,
    script_paths: Vec<String>,
    script_backups: Vec<ScriptBackup>,
    change_id: Option<String>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    if !entity_ids.is_empty() {
        client
            .post(engine_url("/scene/staged/revert"))
            .json(&serde_json::json!({ "entity_ids": entity_ids }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }
    if !modified_entity_ids.is_empty() {
        client
            .post(engine_url("/scene/staged/commit"))
            .json(&serde_json::json!({ "entity_ids": modified_entity_ids }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
    }
    if !script_backups.is_empty() {
        let scene_path = async {
            let r = reqwest::get(engine_url("/scene/path")).await?;
            r.text().await
        }
        .await
        .unwrap_or_default();
        let scene_path = scene_path.trim_matches('"').to_string();
        for backup in &script_backups {
            if let Some(project_root) = std::path::Path::new(&scene_path).parent() {
                let full = project_root.join(&backup.path);
                if backup.existed {
                    if let Some(parent) = full.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(full, &backup.content);
                } else {
                    let _ = std::fs::remove_file(full);
                }
            }
        }
    } else if !script_paths.is_empty() {
        let scene_path = async {
            let r = reqwest::get(engine_url("/scene/path")).await?;
            r.text().await
        }
        .await
        .unwrap_or_default();
        let scene_path = scene_path.trim_matches('"').to_string();
        for path in &script_paths {
            if let Some(project_root) = std::path::Path::new(&scene_path).parent() {
                let _ = std::fs::remove_file(project_root.join(path));
            }
        }
    }
    remove_change_from_proposal(change_id.as_deref()).await;
    Ok(())
}

async fn remove_change_from_proposal(change_id: Option<&str>) {
    let Some(change_id) = change_id else { return };
    let scene_path = match reqwest::get(engine_url("/scene/path")).await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return,
    };
    let proposal_path = format!("{}.proposal.json", scene_path.trim_matches('"'));
    let content = match std::fs::read_to_string(&proposal_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Some(changes) = parsed["changes"].as_array_mut() {
        changes.retain(|c| c["id"].as_str() != Some(change_id));
    }
    let remaining = parsed["changes"].as_array().map(|a| a.len()).unwrap_or(0);
    if remaining == 0 {
        let _ = std::fs::remove_file(&proposal_path);
    } else {
        let _ = std::fs::write(
            &proposal_path,
            serde_json::to_string_pretty(&parsed).unwrap_or_default(),
        );
    }
}

#[tauri::command]
pub async fn clear_staged_proposal() -> Result<(), String> {
    if let Ok(resp) = reqwest::get(engine_url("/scene/path")).await {
        if let Ok(scene_path) = resp.text().await {
            let proposal_path = format!("{}.proposal.json", scene_path.trim_matches('"'));
            let _ = std::fs::remove_file(&proposal_path);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_pending_proposal() -> Result<Option<ProposalResponse>, String> {
    let scene_path = match reqwest::get(engine_url("/scene/path")).await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return Ok(None),
    };
    let proposal_path = format!("{}.proposal.json", scene_path.trim_matches('"'));
    let content = match std::fs::read_to_string(&proposal_path) {
        Ok(c) => c,
        Err(_) => return Ok(None),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let scene_json = match reqwest::get(engine_url("/scene")).await {
        Ok(r) => r.text().await.unwrap_or_default(),
        Err(_) => return Ok(None),
    };
    let scene: serde_json::Value = serde_json::from_str(&scene_json).unwrap_or_default();
    let has_staged = scene["entities"]
        .as_object()
        .map(|ents| {
            ents.values()
                .any(|e| e["staged"].as_bool().unwrap_or(false))
        })
        .unwrap_or(false);
    let has_script_changes = parsed["changes"]
        .as_array()
        .map(|changes| {
            changes.iter().any(|change| {
                change["new_script_paths"]
                    .as_array()
                    .map(|paths| !paths.is_empty())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);
    if !has_staged && !has_script_changes {
        let _ = std::fs::remove_file(&proposal_path);
        return Ok(None);
    }
    let prompt = parsed["prompt"].as_str().unwrap_or("").to_string();
    let summary = parsed["summary"].as_str().unwrap_or("").to_string();
    let changes = parsed["changes"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(i, c)| ProposalChange {
            id: c["id"]
                .as_str()
                .unwrap_or(&format!("c{}", i + 1))
                .to_string(),
            label: c["label"].as_str().unwrap_or("Change").to_string(),
            detail: c["detail"].as_str().unwrap_or("").to_string(),
            staged_entity_ids: c["staged_entity_ids"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
                .unwrap_or_default(),
            modified_entity_ids: c["modified_entity_ids"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64()).collect())
                .unwrap_or_default(),
            new_script_paths: c["new_script_paths"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            script_backups: c["script_backups"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| serde_json::from_value::<ScriptBackup>(v.clone()).ok())
                        .collect()
                })
                .unwrap_or_default(),
            script_new_contents: c["script_new_contents"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| {
                            Some(ScriptNewContent {
                                path: v["path"].as_str()?.to_string(),
                                content: v["content"].as_str().unwrap_or("").to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();
    Ok(Some(ProposalResponse {
        prompt,
        summary,
        changes,
    }))
}
