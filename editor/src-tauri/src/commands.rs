use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

const ENGINE_BASE: &str = "http://127.0.0.1:7878";

pub struct EngineProcess(pub Mutex<Option<tokio::process::Child>>);

fn find_workspace_root() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().unwrap_or(&exe).to_path_buf();
        for _ in 0..8 {
            if dir.join("Cargo.toml").exists() && dir.join("crates").exists() {
                return Some(dir);
            }
            match dir.parent() {
                Some(p) => dir = p.to_path_buf(),
                None => break,
            }
        }
    }

    std::env::current_dir().ok().and_then(|mut dir| {
        loop {
            if dir.join("Cargo.toml").exists() && dir.join("crates").exists() {
                return Some(dir);
            }
            if !dir.pop() {
                return None;
            }
        }
    })
}

fn engine_binary_in_workspace(root: &Path) -> Option<PathBuf> {
    let debug = root.join("target/debug/sindri-server");
    if debug.exists() {
        return Some(debug);
    }
    let release = root.join("target/release/sindri-server");
    if release.exists() {
        return Some(release);
    }
    None
}

fn newest_mtime(path: &Path) -> Option<SystemTime> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.is_file() {
        return metadata.modified().ok();
    }

    let mut newest = metadata.modified().ok();
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path).ok()? {
            let entry = entry.ok()?;
            if let Some(modified) = newest_mtime(&entry.path()) {
                if newest.map_or(true, |current| modified > current) {
                    newest = Some(modified);
                }
            }
        }
    }
    newest
}

fn engine_binary_is_stale(root: &Path, binary: &Path) -> bool {
    let Some(binary_time) = std::fs::metadata(binary)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
    else {
        return true;
    };

    [
        "Cargo.toml",
        "Cargo.lock",
        "crates/sindri-server/Cargo.toml",
        "crates/sindri-server/src",
        "crates/sindri/Cargo.toml",
        "crates/sindri/src",
    ]
    .iter()
    .map(|path| root.join(path))
    .filter_map(|path| newest_mtime(&path))
    .any(|source_time| source_time > binary_time)
}

async fn build_engine_binary(root: &Path, reason: &str) -> Result<PathBuf, String> {
    eprintln!("[start_engine] {reason}; running cargo build -p sindri-server");
    let output = tokio::process::Command::new("cargo")
        .args(["build", "-p", "sindri-server"])
        .current_dir(root)
        .output()
        .await
        .map_err(|e| format!("Failed to run cargo build -p sindri-server: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to build sindri-server.\n\nstdout:\n{}\n\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    engine_binary_in_workspace(root).ok_or_else(|| {
        format!(
            "cargo build -p sindri-server succeeded, but no binary was found under {}",
            root.join("target").display()
        )
    })
}

fn find_engine_binary() -> PathBuf {
    if let Ok(path) = std::env::var("SINDRI_ENGINE_BIN") {
        return PathBuf::from(path);
    }
    if let Some(root) = find_workspace_root() {
        if let Some(binary) = engine_binary_in_workspace(&root) {
            return binary;
        }
    }
    PathBuf::from("sindri-server")
}

async fn ensure_engine_binary() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("SINDRI_ENGINE_BIN") {
        let binary = PathBuf::from(path);
        if binary.exists() {
            return Ok(binary);
        }
        return Err(format!(
            "SINDRI_ENGINE_BIN points to a missing file: {}",
            binary.display()
        ));
    }

    if let Some(root) = find_workspace_root() {
        if let Some(binary) = engine_binary_in_workspace(&root) {
            if !engine_binary_is_stale(&root, &binary) {
                return Ok(binary);
            }
            return build_engine_binary(&root, "sindri-server is older than its sources").await;
        }

        return build_engine_binary(&root, "sindri-server is missing").await;
    }

    let fallback = PathBuf::from("sindri-server");
    if fallback.exists() {
        return Ok(fallback);
    }

    Err(
        "Engine binary not found and the Sindri workspace root could not be located. Set SINDRI_ENGINE_BIN to a sindri-server binary.".into(),
    )
}

#[tauri::command]
pub async fn start_engine(
    project_dir: String,
    state: tauri::State<'_, EngineProcess>,
) -> Result<(), String> {
    let binary = ensure_engine_binary().await?;
    eprintln!("[start_engine] binary: {}", binary.display());

    // Take any existing child out before awaiting so the MutexGuard is dropped.
    let old_child = state.0.lock().unwrap().take();
    if let Some(mut child) = old_child {
        let _ = child.kill().await;
    }

    // Kill any orphaned sindri-server from a previous session.
    #[cfg(unix)]
    let _ = tokio::process::Command::new("pkill")
        .args(["-f", "sindri-server"])
        .output()
        .await;
    #[cfg(windows)]
    let _ = tokio::process::Command::new("taskkill")
        .args(["/f", "/im", "sindri-server.exe"])
        .output()
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let child = tokio::process::Command::new(&binary)
        .arg("--project-dir")
        .arg(&project_dir)
        .arg("--headless")
        .spawn()
        .map_err(|e| format!("Failed to spawn engine ({}): {}", binary.display(), e))?;
    *state.0.lock().unwrap() = Some(child);

    // Wait up to 10 seconds for the engine to respond.
    let client = reqwest::Client::new();
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if client
            .get("http://127.0.0.1:7878/health")
            .send()
            .await
            .is_ok()
        {
            return Ok(());
        }
    }

    Err(format!(
        "Engine started ({}) but didn't respond on :7878 within 10s. Check terminal output for errors.",
        binary.display()
    ))
}

#[tauri::command]
pub async fn get_engine_binary_path() -> String {
    find_engine_binary().to_string_lossy().to_string()
}

#[tauri::command]
pub async fn stop_engine(state: tauri::State<'_, EngineProcess>) -> Result<(), String> {
    let old_child = state.0.lock().unwrap().take();
    if let Some(mut child) = old_child {
        child.kill().await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn create_project(parent_dir: String, name: String) -> Result<String, String> {
    let project_dir = std::path::Path::new(&parent_dir).join(&name);
    std::fs::create_dir_all(project_dir.join("scenes")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("scripts")).map_err(|e| e.to_string())?;
    let meta = serde_json::json!({ "name": name, "version": "0.1.0" });
    std::fs::write(project_dir.join("project.f2proj"), meta.to_string())
        .map_err(|e| e.to_string())?;
    let scene = serde_json::json!({
        "name": "main",
        "entities": {
            "0": {
                "id": 0,
                "name": "Main Camera",
                "parent": null,
                "children": [],
                "components": [
                    {
                        "type": "Transform",
                        "x": 0.0,
                        "y": 0.0,
                        "scale_x": 1.0,
                        "scale_y": 1.0,
                        "rotation": 0.0
                    },
                    {
                        "type": "Camera",
                        "active": true,
                        "zoom": 1.0,
                        "follow_entity": null,
                        "offset_x": 0.0,
                        "offset_y": 0.0,
                        "bounds_min_x": null,
                        "bounds_min_y": null,
                        "bounds_max_x": null,
                        "bounds_max_y": null,
                        "smoothing": 1.0,
                        "dead_zone_width": 0.0,
                        "dead_zone_height": 0.0
                    }
                ],
                "active": true
            }
        },
        "next_id": 1
    });
    std::fs::write(
        project_dir.join("scenes/main.sindri"),
        serde_json::to_string_pretty(&scene).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(project_dir.to_string_lossy().to_string())
}

fn engine_url(path: &str) -> String {
    format!("{}{}", ENGINE_BASE, path)
}

#[tauri::command]
pub async fn get_scene() -> Result<String, String> {
    reqwest::get(engine_url("/scene"))
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize, Deserialize)]
pub struct EngineStatus {
    pub status: String,
    pub model: String,
    pub paused: bool,
    pub playback: Option<String>,
    pub error_count: Option<usize>,
}

#[tauri::command]
pub async fn get_engine_status() -> Result<EngineStatus, String> {
    reqwest::get(engine_url("/health"))
        .await
        .map_err(|e| e.to_string())?
        .json::<EngineStatus>()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn put_scene(scene_json: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .put(engine_url("/scene"))
        .body(scene_json)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|e| e.to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn save_scene() -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(engine_url("/scene/save"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|e| e.to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn open_scene_file(project_path: String, relative_path: String) -> Result<(), String> {
    let root = Path::new(&project_path);
    let full = root.join(&relative_path);
    let canon_root = std::fs::canonicalize(root).map_err(|e| e.to_string())?;
    let canon_file = std::fs::canonicalize(&full).map_err(|e| e.to_string())?;
    if !canon_file.starts_with(&canon_root) {
        return Err("path escapes project directory".into());
    }
    let ext = canon_file
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if ext != "sindri" {
        return Err("only .sindri scene files can be opened".into());
    }

    let scene_json = std::fs::read_to_string(&canon_file).map_err(|e| e.to_string())?;
    // Validate that this is JSON locally; the server performs full Scene parsing.
    serde_json::from_str::<serde_json::Value>(&scene_json).map_err(|e| e.to_string())?;

    let client = reqwest::Client::new();
    let resp = client
        .post(engine_url("/scene/open"))
        .json(&serde_json::json!({ "path": relative_path }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        if status == reqwest::StatusCode::NOT_FOUND
            || status == reqwest::StatusCode::METHOD_NOT_ALLOWED
        {
            return Err(
                "The running sindri-server does not support scene opening yet. Restart the editor so it can rebuild and relaunch the engine sidecar.".into(),
            );
        }
        return Err(resp.text().await.unwrap_or_else(|e| e.to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn patch_transform(
    entity_id: u64,
    x: Option<f32>,
    y: Option<f32>,
    scale_x: Option<f32>,
    scale_y: Option<f32>,
    rotation: Option<f32>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "x": x, "y": y, "scale_x": scale_x, "scale_y": scale_y, "rotation": rotation });
    client
        .patch(engine_url(&format!(
            "/scene/entity/{}/transform",
            entity_id
        )))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_screenshot() -> Result<String, String> {
    let resp: serde_json::Value = reqwest::get(engine_url("/screenshot"))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp["image"].as_str().unwrap_or("").to_string())
}

#[tauri::command]
pub async fn get_runtime_errors() -> Result<Vec<String>, String> {
    let resp: serde_json::Value = reqwest::get(engine_url("/errors"))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp["errors"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(String::from))
        .collect())
}

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

#[tauri::command]
pub async fn send_ai_message(
    message: String,
    context_flags: ContextFlags,
    model: Option<String>,
    history: Vec<serde_json::Value>, // [{role, content}] pairs from prior turns
    open_script: Option<OpenScriptContext>,
) -> Result<AiResponse, String> {
    let client = reqwest::Client::new();

    // Fetch context based on flags
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
    let model = model.as_deref().unwrap_or(if use_vision {
        "qwen2.5-vl:7b"
    } else {
        "qwen2.5-coder:7b"
    });

    let engine_ref = include_str!("../../../ENGINE_REFERENCE.md");
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
    {{ "type": "attach_script", "entity_name": "Player", "path": "scripts/player.lua", "content": "function on_update(self, dt)\n  ...\nend" }},
    {{ "type": "attach_script", "entity_id": 1, "path": "scripts/player.lua", "content": "..." }},

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
- Supported scene component types are exactly: Transform, Sprite, PhysicsBody, Collider, Script, Camera, AudioSource. You may add, remove, and patch these scene components.
- Do NOT create unsupported engine-only components such as Tilemap, Animation, ParticleEmitter, PointLight, DirectionalLight, HUD, PathfindingGrid, or gameplay marker components through AI actions. If asked for one of these, explain that editor/AI scene support is not implemented yet and suggest Lua/scripted or Rust-side alternatives.
- `patch_component` data fields: Sprite supports texture_path, width, height, flip_x, flip_y, color [r,g,b,a]; Collider supports width, height, offset_x, offset_y, is_trigger; PhysicsBody supports body_type, lock_rotation, linear_damping, angular_damping, collision_layer, collision_mask; Script supports path; Camera supports active, zoom, follow_entity, offset_x, offset_y, bounds_min_x, bounds_min_y, bounds_max_x, bounds_max_y, smoothing, dead_zone_width, dead_zone_height; AudioSource supports path, volume, looping, play_on_start.
- Scripts are Lua 5.4 with a CUSTOM engine API. Do NOT use LÖVE2D (`love.*`), Unity, Godot, or any other engine's API.
- Sindri script API: `on_start(self)` and `on_update(self, dt)` hooks. `self` is a table with `x`, `y`, `rotation`, `scale_x`, `scale_y`, `entity_id`, `elapsed`. Globals: `key_down(key)`, `key_pressed(key)`, `print(...)`. Key names are browser KeyboardEvent.key strings: `"ArrowLeft"`, `"ArrowRight"`, `"ArrowUp"`, `"ArrowDown"`, `" "` (Space), `"a"`–`"z"`, etc.
- Correct movement example: `if key_down("ArrowRight") then self.x = self.x + 200 * dt end`
- WRONG (do not use): `love.keyboard.isDown`, `Input.GetKey`, `Input.GetAxis`, `$self.move_and_slide`, `self:input()`, `self:transform()`
- When the user asks you to make an entity do something with scripting, use `attach_script` — it writes the file AND wires up the Script component in one action. Also add a Transform component if the entity doesn't have one.
- When the user explicitly references an existing script file like `#scripts/beacon.lua`, prefer a `write_script` action that edits that file directly instead of unrelated scene actions.
- `patch_component` requires the component index from the scene JSON's "components" array (0-based).
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

    // Build messages: system + history (last N turns) + current user message
    let mut messages: Vec<serde_json::Value> =
        vec![serde_json::json!({ "role": "system", "content": system_prompt })];
    // Include up to the last 10 history messages (5 exchanges)
    let recent_history = if history.len() > 10 {
        &history[history.len() - 10..]
    } else {
        &history[..]
    };
    messages.extend_from_slice(recent_history);
    messages.push(serde_json::json!({ "role": "user", "content": user_content }));

    let req_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });

    let resp: serde_json::Value = client
        .post("http://localhost:11434/api/chat")
        .json(&req_body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let raw_text = resp["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Parse action block
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

#[derive(Serialize)]
pub struct ProposalChange {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub action: serde_json::Value,
}

#[derive(Serialize)]
pub struct ProposalResponse {
    pub prompt: String,
    pub summary: String,
    pub changes: Vec<ProposalChange>,
}

#[tauri::command]
pub async fn generate_proposal(
    message: String,
    context_flags: ContextFlags,
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
                errs.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("\n")
            })
        })
    } else {
        None
    };

    let use_vision = screenshot.is_some();
    let model = model.as_deref().unwrap_or(if use_vision { "qwen2.5-vl:7b" } else { "qwen2.5-coder:7b" });

    let engine_ref = include_str!("../../../ENGINE_REFERENCE.md");
    let system_prompt = format!(
        r#"{engine_ref}

---

You are an AI assistant embedded in the Sindri editor. Respond with a SINGLE JSON object — no markdown fences, no prose outside the JSON.

Format:
{{
  "summary": "One sentence describing the proposed changes (or your answer if no changes).",
  "changes": [
    {{
      "id": "c1",
      "label": "Short human-readable title (e.g. 'Create Player entity')",
      "detail": "What this specific change does and why.",
      "action": {{ ... single action object ... }}
    }}
  ]
}}

If the user is asking a question with no scene changes, use "changes": [].

Action schema — each change.action is ONE action:
{{ "type": "create_entity", "name": "Coin", "parent_id": null }}
{{ "type": "delete_entity", "entity_id": 3 }}
{{ "type": "rename_entity", "entity_id": 1, "name": "Player" }}
{{ "type": "add_component", "entity_name": "Player", "component_type": "Transform" }}
{{ "type": "patch_component", "entity_name": "Player", "component_type": "PhysicsBody", "data": {{ "body_type": "Dynamic", "lock_rotation": true }} }}
{{ "type": "remove_component", "entity_id": 1, "component_type": "Script" }}
{{ "type": "edit_transform", "entity_name": "Player", "x": 400, "y": 260, "scale_x": 1.0, "scale_y": 1.0, "rotation": 0.0 }}
{{ "type": "write_script", "path": "scripts/player.lua", "content": "..." }}
{{ "type": "attach_script", "entity_name": "Player", "path": "scripts/player.lua", "content": "..." }}

Rules:
- Use entity_name for entities created in the same proposal; use entity_id for existing entities.
- Coordinate system: +X right, +Y down.
- Physics needs both PhysicsBody and Collider components.
- Supported component types: Transform, Sprite, PhysicsBody, Collider, Script, Camera, AudioSource.
- Scripts use Sindri Lua API: on_start(self), on_update(self, dt). Globals: key_down(key), key_pressed(key), print(...).
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
    let recent_history = if history.len() > 10 { &history[history.len() - 10..] } else { &history[..] };
    messages.extend_from_slice(recent_history);
    messages.push(serde_json::json!({ "role": "user", "content": user_content }));

    let req_body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });

    let resp: serde_json::Value = client
        .post("http://localhost:11434/api/chat")
        .json(&req_body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let raw_text = resp["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Strip markdown fences if the model wraps the JSON
    let stripped = raw_text.trim();
    let stripped = stripped.trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();

    let parsed: serde_json::Value = serde_json::from_str(stripped).unwrap_or_else(|_| {
        serde_json::json!({ "summary": raw_text, "changes": [] })
    });

    let summary = parsed["summary"].as_str().unwrap_or("").to_string();
    let changes_raw = parsed["changes"].as_array().cloned().unwrap_or_default();

    let changes = changes_raw
        .iter()
        .enumerate()
        .map(|(i, c)| ProposalChange {
            id: c["id"].as_str().unwrap_or(&format!("c{}", i + 1)).to_string(),
            label: c["label"].as_str().unwrap_or("Change").to_string(),
            detail: c["detail"].as_str().unwrap_or("").to_string(),
            action: c["action"].clone(),
        })
        .collect();

    Ok(ProposalResponse { prompt: message, summary, changes })
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

#[tauri::command]
pub async fn create_entity(
    name: String,
    parent_id: Option<u64>,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "name": name, "parent_id": parent_id });
    let resp: serde_json::Value = client
        .post(engine_url("/scene/entity"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp)
}

/// Resolve an entity ID from an action: use `entity_id` if present,
/// otherwise look up by `entity_name` in the live scene.
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
            let body = serde_json::json!({ "content": action["content"] });
            client
                .put(engine_url(&format!("/script?path={}", path)))
                .json(&body)
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
            // Prefer index if provided, else fall back to type-based lookup via old route
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
        // Composite: write script file + add Script component + set path in one shot
        "attach_script" => {
            let entity_id = resolve_entity_id(&action, &client).await?;
            let path = action["path"].as_str().ok_or("missing path")?;
            let content = action["content"].as_str().unwrap_or("");

            // Write the file
            client
                .put(engine_url(&format!("/script?path={}", path)))
                .json(&serde_json::json!({ "content": content }))
                .send()
                .await
                .map_err(|e| e.to_string())?;

            ensure_component(&client, entity_id, "Script").await?;

            // Find an existing Script component and set the path.
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

#[tauri::command]
pub async fn list_ollama_models() -> Result<Vec<String>, String> {
    let resp: serde_json::Value = reqwest::get("http://localhost:11434/api/tags")
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let models = resp["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(models)
}

const SUGGESTION_MODEL: &str = "qwen2.5:0.5b";

fn is_small_model(name: &str) -> bool {
    let lower = name.to_lowercase();
    ["0.5b", "135m", "360m", ":1b", "-1b", ":1.5b", "-1.5b", ":2b", "-2b"]
        .iter()
        .any(|pat| lower.contains(pat))
}

/// Finds an installed small model, or pulls qwen2.5:0.5b if none exists.
/// Returns the model name to use for suggestions.
#[tauri::command]
pub async fn ensure_suggestion_model() -> Result<String, String> {
    let client = reqwest::Client::new();

    let tags: serde_json::Value = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(models) = tags["models"].as_array() {
        for m in models {
            if let Some(name) = m["name"].as_str() {
                if is_small_model(name) {
                    return Ok(name.to_string());
                }
            }
        }
    }

    // No small model found — pull qwen2.5:0.5b
    let resp = client
        .post("http://localhost:11434/api/pull")
        .json(&serde_json::json!({ "name": SUGGESTION_MODEL, "stream": false }))
        .timeout(std::time::Duration::from_secs(900))
        .send()
        .await
        .map_err(|e| format!("pull request failed: {}", e))?;

    // Ollama may return NDJSON even with stream:false — grab the last line
    let body = resp.text().await.map_err(|e| e.to_string())?;
    let last = body.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("");
    let parsed: serde_json::Value = serde_json::from_str(last)
        .unwrap_or(serde_json::json!({"status": "unknown"}));

    match parsed["status"].as_str() {
        Some("success") | Some("pulling manifest") => Ok(SUGGESTION_MODEL.to_string()),
        _ => Err(format!("pull did not succeed: {}", last)),
    }
}

#[derive(Serialize)]
pub struct EntitySuggestion {
    pub label: String,
    pub prompt: String,
    pub mode: String,
}

fn describe_component(c: &serde_json::Value) -> String {
    match c["type"].as_str().unwrap_or("") {
        "Transform" => format!(
            "Transform(x={}, y={}, scale={}/{})",
            c["x"], c["y"], c["scale_x"], c["scale_y"]
        ),
        "Script" => format!(
            "Script(path={})",
            c["path"].as_str().unwrap_or("none")
        ),
        "Sprite" => format!(
            "Sprite({}x{}, texture={})",
            c["width"], c["height"],
            c["texture_path"].as_str().unwrap_or("none")
        ),
        "PhysicsBody" => format!(
            "PhysicsBody(type={}, lock_rotation={})",
            c["body_type"].as_str().unwrap_or("?"),
            c["lock_rotation"].as_bool().unwrap_or(false)
        ),
        "Collider" => format!(
            "Collider({}x{}, trigger={})",
            c["width"], c["height"],
            c["is_trigger"].as_bool().unwrap_or(false)
        ),
        "Camera" => format!(
            "Camera(zoom={}, follow={:?})",
            c["zoom"],
            c["follow_entity"]
        ),
        "AudioSource" => format!(
            "AudioSource(path={}, loop={})",
            c["path"].as_str().unwrap_or("none"),
            c["looping"].as_bool().unwrap_or(false)
        ),
        other => other.to_string(),
    }
}

/// Asks a small local model for 3 actionable prompts specific to this entity.
#[tauri::command]
pub async fn generate_entity_suggestions(
    entity_name: String,
    components: Vec<serde_json::Value>,
    model: String,
) -> Result<Vec<EntitySuggestion>, String> {
    let client = reqwest::Client::new();
    let component_lines = if components.is_empty() {
        "  (no components)".to_string()
    } else {
        components
            .iter()
            .map(|c| format!("  - {}", describe_component(c)))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let prompt = format!(
        r#"Game entity "{name}":
{components}

Write 3 short, specific action prompts for THIS entity based on its actual setup above.
Reference the real component data (script filename, physics type, etc.) — do not invent generic gameplay ideas.
JSON array only, no other text. mode "send" = complete prompt, "prefill" = needs user input (end with ": ").

[
  {{"label": "Short title", "prompt": "Specific prompt referencing actual data", "mode": "send"}},
  {{"label": "Short title", "prompt": "Prompt needing detail: ", "mode": "prefill"}},
  {{"label": "Short title", "prompt": "Specific prompt", "mode": "send"}}
]"#,
        name = entity_name,
        components = component_lines,
    );

    let req_body = serde_json::json!({
        "model": model,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false,
        "options": { "temperature": 0.3, "num_predict": 256 }
    });

    let resp: serde_json::Value = client
        .post("http://localhost:11434/api/chat")
        .json(&req_body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let raw = resp["message"]["content"].as_str().unwrap_or("[]");
    let stripped = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Find JSON array bounds to handle any leading prose from the model
    let json_start = stripped.find('[').unwrap_or(0);
    let json_end = stripped.rfind(']').map(|i| i + 1).unwrap_or(stripped.len());
    let json_slice = &stripped[json_start..json_end];

    let items: Vec<serde_json::Value> = serde_json::from_str(json_slice).unwrap_or_default();

    let suggestions = items
        .into_iter()
        .filter_map(|item| {
            let label = item["label"].as_str()?.to_string();
            let prompt = item["prompt"].as_str()?.to_string();
            let mode = item["mode"].as_str().unwrap_or("send").to_string();
            Some(EntitySuggestion { label, prompt, mode })
        })
        .take(3)
        .collect();

    Ok(suggestions)
}

#[tauri::command]
pub async fn rename_entity(entity_id: u64, name: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    client
        .patch(engine_url(&format!("/scene/entity/{}/name", entity_id)))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn add_component(entity_id: u64, component_type: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(engine_url(&format!(
            "/scene/entity/{}/component",
            entity_id
        )))
        .json(&serde_json::json!({ "component_type": component_type }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(resp.text().await.unwrap_or_default())
    }
}

#[tauri::command]
pub async fn remove_component(entity_id: u64, component_idx: usize) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .delete(engine_url(&format!(
            "/scene/entity/{}/component/{}",
            entity_id, component_idx
        )))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(resp.text().await.unwrap_or_default())
    }
}

#[tauri::command]
pub async fn patch_component(
    entity_id: u64,
    component_idx: usize,
    data: serde_json::Value,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .patch(engine_url(&format!(
            "/scene/entity/{}/component/{}",
            entity_id, component_idx
        )))
        .json(&data)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(resp.text().await.unwrap_or_default())
    }
}

#[tauri::command]
pub async fn get_script(path: String) -> Result<String, String> {
    reqwest::get(engine_url(&format!("/script?path={}", path)))
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_script(path: String, content: String) -> Result<(), String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({ "content": content });
    client
        .put(engine_url(&format!("/script?path={}", path)))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_engine_paused(paused: bool) -> Result<(), String> {
    let client = reqwest::Client::new();
    client
        .post(engine_url("/control"))
        .json(&serde_json::json!({ "paused": paused }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn set_engine_playback(action: String) -> Result<(), String> {
    match action.as_str() {
        "play" | "pause" | "stop" => {}
        _ => return Err("playback action must be play, pause, or stop".into()),
    }

    let client = reqwest::Client::new();
    let resp = client
        .post(engine_url("/control"))
        .json(&serde_json::json!({ "action": action }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_else(|e| e.to_string()));
    }
    Ok(())
}

#[tauri::command]
pub async fn list_scripts() -> Result<Vec<String>, String> {
    let resp: serde_json::Value = reqwest::get(engine_url("/scripts"))
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let paths = resp
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(paths)
}

#[derive(Serialize)]
pub struct ProjectFile {
    pub path: String, // relative to project root, e.g. "scripts/player.lua"
    pub kind: String, // "script" | "scene" | "image" | "audio" | "other"
    pub name: String, // filename only, e.g. "player.lua"
}

#[tauri::command]
pub async fn list_project_files(project_path: String) -> Result<Vec<ProjectFile>, String> {
    let root = std::path::Path::new(&project_path);
    let mut files: Vec<ProjectFile> = Vec::new();

    let dirs = [
        (root.join("scripts"), "scripts"),
        (root.join("scenes"), "scenes"),
        (root.join("assets"), "assets"),
    ];

    for (dir, prefix) in &dirs {
        if !dir.exists() {
            continue;
        }
        collect_files(dir, dir, prefix, &mut files);
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn collect_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    prefix: &str,
    out: &mut Vec<ProjectFile>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let sub_prefix = format!(
                "{}/{}",
                prefix,
                path.file_name().unwrap_or_default().to_string_lossy()
            );
            collect_files(root, &path, &sub_prefix, out);
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let kind = match ext.as_str() {
            "lua" => "script",
            "sindri" => "scene",
            "png" | "jpg" | "jpeg" | "webp" | "bmp" => "image",
            "ogg" | "wav" | "mp3" | "flac" => "audio",
            _ => "other",
        };
        let relative = format!("{}/{}", prefix, name);
        out.push(ProjectFile {
            path: relative,
            kind: kind.to_string(),
            name,
        });
    }
}

#[tauri::command]
pub async fn new_script(project_path: String, name: String) -> Result<String, String> {
    let name = if name.ends_with(".lua") {
        name
    } else {
        format!("{}.lua", name)
    };
    let path = std::path::Path::new(&project_path)
        .join("scripts")
        .join(&name);
    if path.exists() {
        return Err(format!("{} already exists", name));
    }
    let template = format!("function on_start(self)\nend\n\nfunction on_update(self, dt)\nend\n");
    std::fs::write(&path, template).map_err(|e| e.to_string())?;
    Ok(format!("scripts/{}", name))
}

#[tauri::command]
pub async fn delete_project_file(
    project_path: String,
    relative_path: String,
) -> Result<(), String> {
    // Safety: only allow deleting within the project directory
    let full = std::path::Path::new(&project_path).join(&relative_path);
    let canon_root = std::fs::canonicalize(&project_path).map_err(|e| e.to_string())?;
    let canon_file = std::fs::canonicalize(&full).map_err(|e| e.to_string())?;
    if !canon_file.starts_with(&canon_root) {
        return Err("path escapes project directory".into());
    }
    std::fs::remove_file(&full).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_project_file(
    project_path: String,
    relative_path: String,
    new_name: String,
) -> Result<String, String> {
    let full = std::path::Path::new(&project_path).join(&relative_path);
    let parent = full.parent().ok_or("no parent dir")?;
    let new_full = parent.join(&new_name);
    let canon_root = std::fs::canonicalize(&project_path).map_err(|e| e.to_string())?;
    let canon_new_parent = std::fs::canonicalize(parent).map_err(|e| e.to_string())?;
    if !canon_new_parent.starts_with(&canon_root) {
        return Err("path escapes project directory".into());
    }
    std::fs::rename(&full, &new_full).map_err(|e| e.to_string())?;
    let parent_prefix = relative_path.rsplitn(2, '/').nth(1).unwrap_or("");
    Ok(if parent_prefix.is_empty() {
        new_name
    } else {
        format!("{}/{}", parent_prefix, new_name)
    })
}
