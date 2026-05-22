use super::{AiResponse, ContextFlags, OpenScriptContext};
use super::client::{default_cloud_model, emit_ai_stream, request_ai_text,
    stream_anthropic_text, stream_ollama_text, stream_openai_text, stream_openrouter_text};
use crate::engine_client::engine_url;

#[tauri::command]
pub async fn send_ai_message_stream(
    app: tauri::AppHandle,
    request_id: String,
    message: String,
    provider: Option<String>,
    model: Option<String>,
    history: Vec<serde_json::Value>,
) -> Result<(), String> {
    let provider = provider.unwrap_or_else(|| "ollama".to_string());
    let model = model.unwrap_or_else(|| default_cloud_model(&provider).to_string());
    let client = reqwest::Client::new();
    let mut messages = vec![serde_json::json!({
        "role": "system",
        "content": "You are Sindri's embedded AI assistant. Keep responses concise and practical."
    })];
    let recent_history = if history.len() > 10 { &history[history.len() - 10..] } else { &history[..] };
    messages.extend_from_slice(recent_history);
    messages.push(serde_json::json!({ "role": "user", "content": message }));

    let result = match provider.as_str() {
        "openai" => stream_openai_text(app.clone(), request_id.clone(), client, model, messages).await,
        "anthropic" => stream_anthropic_text(app.clone(), request_id.clone(), client, model, messages).await,
        "openrouter" => stream_openrouter_text(app.clone(), request_id.clone(), client, model, messages).await,
        _ => stream_ollama_text(app.clone(), request_id.clone(), client, model, messages).await,
    };

    match result {
        Ok(full) => emit_ai_stream(&app, &request_id, "done", full),
        Err(err) => emit_ai_stream(&app, &request_id, "error", err),
    }
    Ok(())
}

#[tauri::command]
pub async fn send_ai_message(
    message: String,
    context_flags: ContextFlags,
    provider: Option<String>,
    model: Option<String>,
    history: Vec<serde_json::Value>,
    open_script: Option<OpenScriptContext>,
) -> Result<AiResponse, String> {
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
            v["errors"].as_array().map(|errors| {
                errors
                    .iter()
                    .filter_map(|value| value.as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
        })
    } else {
        None
    };

    let use_vision = screenshot.is_some();
    let provider = provider.unwrap_or_else(|| "ollama".to_string());
    let model = model.unwrap_or_else(|| {
        if provider == "ollama" {
            if use_vision { "qwen2.5-vl:7b" } else { "qwen2.5-coder:7b" }.to_string()
        } else {
            default_cloud_model(&provider).to_string()
        }
    });

    let engine_ref = include_str!("../../../../ENGINE_REFERENCE.md");
    let system_prompt = format!(
        r#"{engine_ref}

---

You are an AI assistant embedded in the Sindri editor. You have FULL CONTROL over the editor and the running engine.

You have access to the current scene (as JSON), optionally an open script file, and optionally a screenshot of the game viewport.

You may respond in two ways:
1. Plain text explanation or analysis.
2. A JSON action block wrapped in <action>...</action> tags.

Action block schema — use as many actions as needed in one block:
{{
  "actions": [
    // Scene structure
    {{ "type": "create_entity", "name": "Coin", "parent_id": null }},
    {{ "type": "delete_entity", "entity_id": 3 }},
    {{ "type": "rename_entity", "entity_id": 1, "name": "Player" }},

    // Components — use entity_name when you just created the entity and don't know its ID yet
    {{ "type": "add_component", "entity_name": "Player", "component_type": "Transform" }},
    {{ "type": "add_component", "entity_name": "Player", "component_type": "PhysicsBody" }},
    {{ "type": "patch_component", "entity_name": "Player", "component_type": "PhysicsBody", "data": {{ "body_type": "Dynamic", "lock_rotation": true }} }},
    {{ "type": "patch_component", "entity_name": "Player", "component_type": "Sprite", "data": {{ "width": 48, "height": 48, "color": [0.3, 0.6, 1.0, 1.0] }} }},
    {{ "type": "add_component", "entity_name": "Ground", "component_type": "PhysicsBody" }},
    {{ "type": "patch_component", "entity_name": "Ground", "component_type": "PhysicsBody", "data": {{ "body_type": "Fixed" }} }},
    {{ "type": "add_component", "entity_name": "Player", "component_type": "Collider" }},
    {{ "type": "patch_component", "entity_name": "Player", "component_type": "Collider", "data": {{ "width": 32, "height": 48, "is_trigger": false }} }},
    {{ "type": "add_component", "entity_id": 1, "component_type": "Script" }},
    {{ "type": "remove_component", "entity_id": 1, "component_type": "Script" }},
    {{ "type": "patch_component", "entity_id": 1, "component_idx": 0, "data": {{ "path": "scripts/player.lua" }} }},
    {{ "type": "patch_component", "entity_id": 1, "component_type": "Camera", "data": {{ "active": true, "zoom": 1.0, "follow_entity": null, "offset_x": 0.0, "offset_y": 0.0, "smoothing": 1.0, "dead_zone_width": 0.0, "dead_zone_height": 0.0 }} }},
    {{ "type": "patch_component", "entity_id": 1, "component_type": "AudioSource", "data": {{ "path": "audio/jump.ogg", "volume": 0.8, "looping": false, "play_on_start": false }} }},

    // Transform
    {{ "type": "edit_transform", "entity_name": "Player", "x": 400, "y": 260, "scale_x": 1.0, "scale_y": 1.0, "rotation": 0.0 }},
    {{ "type": "edit_transform", "entity_name": "Ground", "x": 400, "y": 340, "scale_x": 5.0, "scale_y": 1.0, "rotation": 0.0 }},

    // Scripts
    {{ "type": "write_script", "path": "scripts/beacon.lua", "content": "function on_update(self, dt)\n  self.rotation = self.rotation + 360 * dt\nend" }},
    // PREFERRED for new script-driven entities: write script + add Script component + set path in one shot
    // IMPORTANT: also add a Transform if the entity doesn't have one
    {{ "type": "attach_script", "entity_name": "Player", "path": "scripts/player.lua", "content": "function on_update(self, dt)\n  local input = self:input()\n  local pb = self:physics()\n  local spd = 200\n  local vx = 0\n  if input:is_key_down('ArrowLeft') then vx = -spd end\n  if input:is_key_down('ArrowRight') then vx = spd end\n  pb:set_velocity(vec2(vx, pb:get_velocity().y))\nend" }},

    // Suggestion (shown as a card for user approval)
    {{ "type": "suggest_fix", "description": "Collider height doesn't match sprite height", "entity_id": 0 }}
  ]
}}

CRITICAL RULE — entity references:
- For entities already in the scene: use `"entity_id"` with the numeric ID from the scene JSON.
- For entities you are creating in the same action block: you do NOT know the ID yet. Use `"entity_name": "EntityName"` instead — the executor resolves it by looking up the name in the scene after the entity is created.
- NEVER guess an entity_id (like 0) for a newly created entity. Always use entity_name for those.

Rules:
- Coordinate system: +X moves right, +Y moves down. "Below" means a larger y value than the reference entity. "Above" means a smaller y value. "Left of" means smaller x; "right of" means larger x.
- For vertical relationships like "ground below player", keep x aligned unless the user asks for horizontal offset. Example: Player at x=400,y=260 and Ground at x=400,y=340.
- When creating a player and a ground/platform, place the player above the ground, not beside it. Use explicit edit_transform actions for both entities.
- Physics needs both PhysicsBody and Collider. Use PhysicsBody body_type "Dynamic" for moving players/enemies, "Fixed" for ground/walls/platforms, and "Kinematic" for scripted moving platforms. Add Collider for collision shape/trigger data.
- For platformer-style players, set PhysicsBody lock_rotation=true.
- Supported scene component types are exactly: Transform, Sprite, AnimatedSprite, Tilemap, PhysicsBody, Collider, Script, Camera, AudioSource. You may add, remove, and patch these scene components.
- PLAYER ENTITY TEMPLATE — when creating a player, ALWAYS include ALL of these actions: (1) create_entity "Player", (2) add_component Transform, (3) add_component Sprite + patch color/size, (4) add_component PhysicsBody + patch body_type Dynamic lock_rotation true, (5) add_component Collider + patch width/height to match sprite, (6) attach_script with movement Lua code. An entity with only create_entity and no components is useless.
- Do NOT create unsupported engine-only components such as ParticleEmitter, PointLight, DirectionalLight, HUD, PathfindingGrid, or gameplay marker components through AI actions. If asked for one of these, explain that editor/AI scene support is not implemented yet and suggest Lua/scripted or Rust-side alternatives.
- `patch_component` data fields: Sprite supports texture_path, width, height, flip_x, flip_y, color [r,g,b,a]; AnimatedSprite supports texture_path, cols, rows, width, height, flip_x, flip_y, tint, margin, spacing, clips (array of clip objects — each clip: {{name, start_frame, end_frame, fps, looping}}; frames are 0-indexed, left-to-right row-by-row across the spritesheet), default_clip; example AnimatedSprite patch: {{"texture_path":"textures/player.png","cols":8,"rows":4,"width":48,"height":48,"default_clip":"idle_down","clips":[{{"name":"idle_down","start_frame":0,"end_frame":3,"fps":8,"looping":true}},{{"name":"walk_down","start_frame":8,"end_frame":15,"fps":10,"looping":true}},{{"name":"walk_right","start_frame":16,"end_frame":23,"fps":10,"looping":true}}]}}; Tilemap supports palettes (array of TilePalette objects: {{name, texture_path, tileset_cols, tileset_rows, margin, spacing, solid_tiles (0-based tile indices within this palette that are solid)}}), tile_width, tile_height, map_cols, map_rows, tiles (flat u32 array: 0=empty, upper 16 bits=palette_id (1-indexed), lower 16 bits=tile_idx (0-indexed)), tint; solid tiles per palette auto-generate static physics colliders on play; Collider supports width, height, offset_x, offset_y, is_trigger; PhysicsBody supports body_type, lock_rotation, linear_damping, angular_damping, collision_layer, collision_mask, gravity_scale; Script supports path; Camera supports active, zoom, follow_entity, offset_x, offset_y, bounds_min_x, bounds_min_y, bounds_max_x, bounds_max_y, smoothing, dead_zone_width, dead_zone_height; AudioSource supports path, volume, looping, play_on_start.
- Scripts are Lua 5.4 with a CUSTOM Sindri engine API. Do NOT use LÖVE2D (`love.*`), Unity, Godot, or any other engine's API.
- `self` is an engine userdata object — NOT a plain table. Access everything through method calls.
- Sindri script API summary: hooks are `on_start(self)` / `on_update(self, dt)`. Key facets: `self:input()` → InputFacet, `self:transform()` → TransformFacet|nil, `self:physics()` → PhysicsFacet|nil, `self:sprite()` → SpriteFacet|nil. Global: `vec2(x,y)`.
- InputFacet: `input:is_key_down("A")`, `input:is_key_pressed("Space")`, `input:axis("A","D")` → -1..1. Key names: single letters `"A"`–`"Z"`, arrows `"Left"` `"Right"` `"Up"` `"Down"`, `"Space"`, `"Enter"`, `"Escape"`.
- TransformFacet: `t:position()` → Vec2, `t:set_position(vec2(x,y))`, `t:set_rotation(r)`, `t:set_scale(vec2(sx,sy))`. Always nil-check: `local t = self:transform(); if t == nil then return end`.
- PhysicsFacet: `p:velocity()` → Vec2, `p:set_velocity(vec2(vx,vy))`, `p:apply_impulse(vec2(ix,iy))`, `p:contacts()` → table of entity name strings currently touching this entity (e.g. `{{"Player","Ground"}}`). Nil if entity has no PhysicsBody component.
- Global `entity_transform(name)` → `{{x, y, rotation}}` or nil. Returns the named entity's transform as a snapshot for this frame. Example: `local t = entity_transform("Drone"); if t then local dx = t.x - self.x end`.
- SpriteFacet: `spr:set_tint({{r,g,b,a}})`, `spr:set_visible(bool)`. Nil if no Sprite component.
- AnimatedSpriteFacet: `local anim = self:animated_sprite()` — nil if no AnimatedSprite component. Methods: `anim:play("clip_name")` (switches clip, no-op if already playing), `anim:set_flip_x(bool)`, `anim:set_flip_y(bool)`, `anim:current_clip()` → string. Use `play` every frame based on state; it is idempotent. Example walk pattern: `local anim = self:animated_sprite(); if anim ~= nil then if h > 0.1 then anim:play("walk_right") elseif h < -0.1 then anim:play("walk_left") else anim:play("idle") end end`
- Correct platformer movement: `local h = input:axis("A","D"); phys:set_velocity(vec2(h * speed, phys:velocity().y))`
- ATTACH/FOLLOW PATTERN — to ride/follow another entity, use `contacts()` to detect the touch, then `entity_transform("Name")` to get its position/rotation: `local contacts = self:physics():contacts(); for _, name in ipairs(contacts) do if name == "Drone" then local dt = entity_transform("Drone"); if dt then self:transform():set_rotation(dt.rotation); self:transform():set_position(vec2(dt.x, dt.y - 32)); end end end`
- WRONG (do not use): `love.keyboard.isDown`, `Input.GetKey`, `key_down()` global, `self.x`/`self.y` field access, `self:move_and_slide()`
- When the user asks you to make an entity do something with scripting, use `attach_script` — it writes the file AND wires up the Script component in one action. Also add a Transform component if the entity doesn't have one.
- CRITICAL: attach_script and write_script MUST include the complete Lua code in "content". Never use "..." as a placeholder — if content is empty or placeholder the action will be rejected and the user will see an error.
- When the user explicitly references an existing script file like `#scripts/beacon.lua`, prefer a `write_script` action that edits that file directly instead of unrelated scene actions.
- `patch_component` with `entity_name` + `component_type` is the preferred form. The `entity_id` + `component_idx` form also works but is less readable.
- Always respond with a brief plain-text explanation first, then the action block.
- Only include an action block when making or suggesting a concrete change.
- Keep explanations concise — one or two sentences.
- entity_id values come from the scene JSON's "id" fields (u64 integers)."#,
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

    let raw_text = request_ai_text(&client, &provider, &model, messages).await?;

    let (text, mut actions) =
        if let (Some(start), Some(end)) = (raw_text.find("<action>"), raw_text.find("</action>")) {
            let before = raw_text[..start].trim().to_string();
            let action_json = &raw_text[start + 8..end];
            let parsed: serde_json::Value =
                serde_json::from_str(action_json.trim()).unwrap_or(serde_json::json!({}));
            let actions = parsed["actions"].as_array().cloned().unwrap_or_default();
            (before, actions)
        } else {
            (raw_text, vec![])
        };

    normalize_script_actions(&message, open_script.as_ref(), &mut actions);
    normalize_spatial_actions(&message, scene.as_deref(), &mut actions);

    Ok(AiResponse { text, actions })
}

fn normalize_script_actions(
    message: &str,
    open_script: Option<&OpenScriptContext>,
    actions: &mut [serde_json::Value],
) {
    let referenced_paths = referenced_script_paths(message);
    if referenced_paths.is_empty() {
        return;
    }

    for action in actions.iter_mut() {
        if action["type"].as_str() != Some("attach_script") {
            continue;
        }

        let Some(path) = action["path"].as_str() else {
            continue;
        };

        if !referenced_paths.iter().any(|referenced| referenced == path) {
            continue;
        }

        let content = action["content"].clone();
        *action = serde_json::json!({
            "type": "write_script",
            "path": path,
            "content": content,
        });
    }

    if let Some(script) = open_script {
        for action in actions.iter_mut() {
            if action["type"].as_str() != Some("write_script") {
                continue;
            }
            if action["path"].is_null() {
                action["path"] = serde_json::json!(script.path);
            }
        }
    }
}

fn referenced_script_paths(message: &str) -> Vec<String> {
    message
        .split_whitespace()
        .filter_map(|token| token.strip_prefix('#'))
        .map(|path| path.trim_matches(|ch: char| ",.!?;:()[]{}<>\"'`".contains(ch)))
        .filter(|path| path.ends_with(".lua"))
        .map(str::to_string)
        .collect()
}

fn normalize_spatial_actions(
    message: &str,
    scene_json: Option<&str>,
    actions: &mut Vec<serde_json::Value>,
) {
    let lower = message.to_lowercase();
    let wants_ground_below_player = lower.contains("ground")
        && lower.contains("player")
        && (lower.contains("below") || lower.contains("under") || lower.contains("beneath"));
    if !wants_ground_below_player {
        return;
    }

    let scene_player_transform = scene_json
        .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
        .and_then(|scene| entity_transform_by_name(&scene, "Player"));
    let scene_player_x = scene_player_transform.map(|(x, _)| x);
    let scene_player_y = scene_player_transform.map(|(_, y)| y);

    let player_x = find_transform_value(actions, "Player", "x")
        .or(scene_player_x)
        .unwrap_or(400.0);
    let player_y = find_transform_value(actions, "Player", "y")
        .or(scene_player_y)
        .unwrap_or(260.0);
    let ground_y = find_transform_value(actions, "Ground", "y")
        .filter(|y| *y > player_y)
        .unwrap_or(player_y + 80.0);

    ensure_edit_transform(actions, "Player", player_x, player_y);
    ensure_edit_transform(actions, "Ground", player_x, ground_y);
}

fn entity_transform_by_name(scene: &serde_json::Value, name: &str) -> Option<(f32, f32)> {
    scene["entities"]
        .as_object()?
        .values()
        .find(|entity| entity["name"].as_str() == Some(name))
        .and_then(|entity| {
            entity["components"]
                .as_array()?
                .iter()
                .find_map(|component| {
                    if component["type"].as_str() == Some("Transform") {
                        Some((
                            component["x"].as_f64()? as f32,
                            component["y"].as_f64()? as f32,
                        ))
                    } else {
                        None
                    }
                })
        })
}

fn find_transform_value(
    actions: &[serde_json::Value],
    entity_name: &str,
    field: &str,
) -> Option<f32> {
    actions.iter().find_map(|action| {
        if action["type"].as_str() == Some("edit_transform")
            && action["entity_name"].as_str() == Some(entity_name)
        {
            action[field].as_f64().map(|value| value as f32)
        } else {
            None
        }
    })
}

fn ensure_edit_transform(actions: &mut Vec<serde_json::Value>, entity_name: &str, x: f32, y: f32) {
    if let Some(action) = actions.iter_mut().find(|action| {
        action["type"].as_str() == Some("edit_transform")
            && action["entity_name"].as_str() == Some(entity_name)
    }) {
        action["x"] = serde_json::json!(x);
        action["y"] = serde_json::json!(y);
        return;
    }

    actions.push(serde_json::json!({
        "type": "edit_transform",
        "entity_name": entity_name,
        "x": x,
        "y": y
    }));
}
