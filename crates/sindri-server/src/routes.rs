use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::RwLock;

use sindri::component::Transform;
use sindri::scene::Scene;

pub type SharedScene = Arc<RwLock<Scene>>;
pub type SharedKeys = Arc<RwLock<HashSet<String>>>;

#[derive(Clone)]
pub struct AppState {
    pub scene: SharedScene,
    pub screenshot_fn: Arc<dyn Fn() -> Option<Vec<u8>> + Send + Sync>,
    pub scripts_root: std::path::PathBuf,
    pub model: String,
    pub keys: SharedKeys,
    pub paused: Arc<AtomicBool>,
}

// GET /health
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "model": state.model,
        "paused": state.paused.load(Ordering::Relaxed),
    }))
}

#[derive(Deserialize)]
pub struct ControlBody {
    pub paused: bool,
}

// POST /control
pub async fn post_control(
    State(state): State<AppState>,
    Json(body): Json<ControlBody>,
) -> impl IntoResponse {
    state.paused.store(body.paused, Ordering::Relaxed);
    StatusCode::OK
}

// GET /scene
pub async fn get_scene(State(state): State<AppState>) -> impl IntoResponse {
    let scene = state.scene.read().await;
    match scene.to_json() {
        Ok(json) => (StatusCode::OK, json).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// PUT /scene
pub async fn put_scene(State(state): State<AppState>, body: String) -> impl IntoResponse {
    match Scene::from_json(&body) {
        Ok(new_scene) => {
            *state.scene.write().await = new_scene;
            StatusCode::OK.into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

// GET /scene/entity/:id
pub async fn get_entity(State(state): State<AppState>, Path(id): Path<u64>) -> impl IntoResponse {
    let scene = state.scene.read().await;
    match scene.entities.get(&id) {
        Some(entity) => Json(entity).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
pub struct TransformPatch {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub scale_x: Option<f32>,
    pub scale_y: Option<f32>,
    pub rotation: Option<f32>,
}

// PATCH /scene/entity/:id/transform
pub async fn patch_transform(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(patch): Json<TransformPatch>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    let entity = match scene.entities.get_mut(&id) {
        Some(e) => e,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let transform = entity.components.iter_mut().find_map(|c| {
        if let sindri::component::Component::Transform(t) = c {
            Some(t)
        } else {
            None
        }
    });

    if let Some(t) = transform {
        if let Some(x) = patch.x {
            t.x = x;
        }
        if let Some(y) = patch.y {
            t.y = y;
        }
        if let Some(sx) = patch.scale_x {
            t.scale_x = sx;
        }
        if let Some(sy) = patch.scale_y {
            t.scale_y = sy;
        }
        if let Some(r) = patch.rotation {
            t.rotation = r;
        }
    } else {
        entity
            .components
            .push(sindri::component::Component::Transform(Transform {
                x: patch.x.unwrap_or(0.0),
                y: patch.y.unwrap_or(0.0),
                scale_x: patch.scale_x.unwrap_or(1.0),
                scale_y: patch.scale_y.unwrap_or(1.0),
                rotation: patch.rotation.unwrap_or(0.0),
            }));
    }

    StatusCode::OK.into_response()
}

#[derive(Deserialize)]
pub struct CreateEntityBody {
    pub name: String,
    pub parent_id: Option<u64>,
}

// POST /scene/entity
pub async fn create_entity(
    State(state): State<AppState>,
    Json(body): Json<CreateEntityBody>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    let id = scene.spawn(&body.name);
    if let Some(parent_id) = body.parent_id {
        scene.set_parent(id, parent_id);
    }
    Json(serde_json::json!({ "id": id })).into_response()
}

#[derive(Deserialize)]
pub struct RenameEntityBody {
    pub name: String,
}

// PATCH /scene/entity/:id/name
pub async fn rename_entity(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(body): Json<RenameEntityBody>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    match scene.entities.get_mut(&id) {
        Some(e) => {
            e.name = body.name;
            StatusCode::OK.into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
pub struct AddComponentBody {
    pub component_type: String,
}

// POST /scene/entity/:id/component
pub async fn add_component(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(body): Json<AddComponentBody>,
) -> impl IntoResponse {
    use sindri::component::*;
    let component = match body.component_type.as_str() {
        "Transform" => Component::Transform(Transform {
            x: 0.0,
            y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
        }),
        "Sprite" => Component::Sprite(Sprite {
            texture_path: String::new(),
            width: 64.0,
            height: 64.0,
            flip_x: false,
            flip_y: false,
            color: [1.0, 1.0, 1.0, 1.0],
        }),
        "PhysicsBody" => Component::PhysicsBody(PhysicsBody {
            body_type: BodyType::Dynamic,
            lock_rotation: false,
            linear_damping: 0.0,
            angular_damping: 0.0,
            collision_layer: 0,
            collision_mask: u32::MAX,
        }),
        "Collider" => Component::Collider(Collider {
            width: 32.0,
            height: 32.0,
            offset_x: 0.0,
            offset_y: 0.0,
            is_trigger: false,
        }),
        "Script" => Component::Script(Script {
            path: String::new(),
        }),
        "Camera" => Component::Camera(Camera {
            zoom: 1.0,
            follow_entity: None,
        }),
        "AudioSource" => Component::AudioSource(AudioSource {
            path: String::new(),
            volume: 1.0,
            looping: false,
            play_on_start: false,
        }),
        _ => return (StatusCode::BAD_REQUEST, "unknown component type").into_response(),
    };
    let mut scene = state.scene.write().await;
    match scene.entities.get_mut(&id) {
        Some(e) => {
            e.components.push(component);
            StatusCode::OK.into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
pub struct ComponentTypeQuery {
    pub component_type: String,
}

// DELETE /scene/entity/:id/component?component_type=Transform
pub async fn remove_component(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(q): Query<ComponentTypeQuery>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    match scene.entities.get_mut(&id) {
        Some(e) => {
            let before = e.components.len();
            e.components.retain(|c| {
                let type_name = match c {
                    sindri::component::Component::Transform(_) => "Transform",
                    sindri::component::Component::Sprite(_) => "Sprite",
                    sindri::component::Component::PhysicsBody(_) => "PhysicsBody",
                    sindri::component::Component::Collider(_) => "Collider",
                    sindri::component::Component::Script(_) => "Script",
                    sindri::component::Component::Camera(_) => "Camera",
                    sindri::component::Component::AudioSource(_) => "AudioSource",
                };
                type_name != q.component_type
            });
            if e.components.len() < before {
                StatusCode::OK.into_response()
            } else {
                (StatusCode::NOT_FOUND, "component not found").into_response()
            }
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// DELETE /scene/entity/:id/component/:idx
pub async fn remove_component_by_idx(
    State(state): State<AppState>,
    Path(p): Path<ComponentIdxPath>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    let Some(entity) = scene.entities.get_mut(&p.id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if p.idx >= entity.components.len() {
        return (StatusCode::NOT_FOUND, "component index out of range").into_response();
    }
    entity.components.remove(p.idx);
    StatusCode::OK.into_response()
}

#[derive(Deserialize)]
pub struct ComponentIdxPath {
    pub id: u64,
    pub idx: usize,
}

#[derive(Deserialize)]
pub struct PatchScriptBody {
    pub path: String,
}

// PATCH /scene/entity/:id/component/:idx  (currently only updates Script path)
pub async fn patch_component(
    State(state): State<AppState>,
    Path(p): Path<ComponentIdxPath>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    let Some(entity) = scene.entities.get_mut(&p.id) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some(comp) = entity.components.get_mut(p.idx) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match comp {
        sindri::component::Component::Script(s) => {
            if let Some(path) = body["path"].as_str() {
                s.path = path.to_string();
                StatusCode::OK.into_response()
            } else {
                (StatusCode::BAD_REQUEST, "missing path").into_response()
            }
        }
        sindri::component::Component::PhysicsBody(p) => {
            if let Some(body_type) = body["body_type"].as_str() {
                p.body_type = match body_type {
                    "Dynamic" => sindri::component::BodyType::Dynamic,
                    "Kinematic" => sindri::component::BodyType::Kinematic,
                    "Fixed" => sindri::component::BodyType::Fixed,
                    _ => return (StatusCode::BAD_REQUEST, "unknown body_type").into_response(),
                };
            }
            if let Some(lock_rotation) = body["lock_rotation"].as_bool() {
                p.lock_rotation = lock_rotation;
            }
            if let Some(linear_damping) = body["linear_damping"].as_f64() {
                p.linear_damping = linear_damping as f32;
            }
            if let Some(angular_damping) = body["angular_damping"].as_f64() {
                p.angular_damping = angular_damping as f32;
            }
            if let Some(collision_layer) = body["collision_layer"].as_u64() {
                p.collision_layer = collision_layer.min(u8::MAX as u64) as u8;
            }
            if let Some(collision_mask) = body["collision_mask"].as_u64() {
                p.collision_mask = collision_mask.min(u32::MAX as u64) as u32;
            }
            StatusCode::OK.into_response()
        }
        sindri::component::Component::Collider(c) => {
            if let Some(width) = body["width"].as_f64() {
                c.width = width as f32;
            }
            if let Some(height) = body["height"].as_f64() {
                c.height = height as f32;
            }
            if let Some(offset_x) = body["offset_x"].as_f64() {
                c.offset_x = offset_x as f32;
            }
            if let Some(offset_y) = body["offset_y"].as_f64() {
                c.offset_y = offset_y as f32;
            }
            if let Some(is_trigger) = body["is_trigger"].as_bool() {
                c.is_trigger = is_trigger;
            }
            StatusCode::OK.into_response()
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "unsupported component type for patch",
        )
            .into_response(),
    }
}

// DELETE /scene/entity/:id
pub async fn delete_entity(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    if scene.entities.contains_key(&id) {
        scene.remove_entity(id);
        StatusCode::OK.into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

// GET /screenshot
pub async fn get_screenshot(State(state): State<AppState>) -> impl IntoResponse {
    match (state.screenshot_fn)() {
        Some(png_bytes) => {
            use base64::Engine as _;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
            Json(serde_json::json!({ "image": b64 })).into_response()
        }
        None => (StatusCode::SERVICE_UNAVAILABLE, "no frame available").into_response(),
    }
}

#[derive(Deserialize)]
pub struct ScriptPathQuery {
    pub path: String,
}

// GET /scripts
pub async fn list_scripts(State(state): State<AppState>) -> impl IntoResponse {
    let root = &state.scripts_root;
    let mut paths: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry
                .path()
                .extension()
                .map(|e| e == "lua")
                .unwrap_or(false)
            {
                if let Some(name) = entry.path().to_str() {
                    paths.push(name.to_string());
                }
            }
        }
    }
    Json(paths).into_response()
}

// GET /script?path=...
pub async fn get_script(
    State(state): State<AppState>,
    Query(q): Query<ScriptPathQuery>,
) -> impl IntoResponse {
    let full = state.scripts_root.join(&q.path);
    match std::fs::read_to_string(&full) {
        Ok(contents) => (StatusCode::OK, contents).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
pub struct WriteScriptBody {
    pub content: String,
}

#[derive(Deserialize)]
pub struct KeysBody {
    pub keys: Vec<String>,
}

// POST /input/keys  — editor sends currently-held keys each frame
pub async fn post_keys(
    State(state): State<AppState>,
    Json(body): Json<KeysBody>,
) -> impl IntoResponse {
    let mut k = state.keys.write().await;
    *k = body.keys.into_iter().collect();
    StatusCode::OK
}

// PUT /script?path=...
pub async fn put_script(
    State(state): State<AppState>,
    Query(q): Query<ScriptPathQuery>,
    Json(body): Json<WriteScriptBody>,
) -> impl IntoResponse {
    let full = state.scripts_root.join(&q.path);
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&full, &body.content) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
