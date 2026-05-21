use axum::{
    extract::{Path, Query, State},
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

use sindri::component::Transform;
use sindri::scene::Scene;

pub type SharedScene = Arc<RwLock<Scene>>;
pub type SharedScenePath = Arc<RwLock<PathBuf>>;
pub type SharedKeys = Arc<RwLock<HashSet<String>>>;
pub type SharedPlayback = Arc<Mutex<PlaybackState>>;
pub type SharedErrors = Arc<Mutex<Vec<String>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackMode {
    Stopped,
    Playing,
    Paused,
}

impl PlaybackMode {
    pub fn as_str(self) -> &'static str {
        match self {
            PlaybackMode::Stopped => "stopped",
            PlaybackMode::Playing => "playing",
            PlaybackMode::Paused => "paused",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub mode: PlaybackMode,
    pub edit_scene: Option<Scene>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            mode: PlaybackMode::Stopped,
            edit_scene: None,
        }
    }
}

pub type SharedGizmos = Arc<std::sync::atomic::AtomicBool>;

#[derive(Clone)]
pub struct AppState {
    pub scene: SharedScene,
    pub scene_path: SharedScenePath,
    pub project_root: PathBuf,
    pub screenshot_fn: Arc<dyn Fn(&Scene) -> Option<Vec<u8>> + Send + Sync>,
    pub scripts_root: std::path::PathBuf,
    pub model: String,
    pub keys: SharedKeys,
    pub playback: SharedPlayback,
    pub errors: SharedErrors,
    pub gizmos: SharedGizmos,
    /// Broadcast channel for live JPEG frames from headless rendering.
    pub frame_tx: Option<tokio::sync::broadcast::Sender<Vec<u8>>>,
}

pub async fn stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_stream(socket, state))
}

async fn handle_stream(mut socket: WebSocket, state: AppState) {
    let Some(ref frame_tx) = state.frame_tx else {
        let _ = socket.close().await;
        return;
    };
    let mut rx = frame_tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(frame) => {
                if socket.send(Message::Binary(frame.into())).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => break,
        }
    }
}

fn has_active_camera(scene: &Scene) -> bool {
    scene.entities.values().any(|entity| {
        entity.components.iter().any(|component| {
            matches!(
                component,
                sindri::component::Component::Camera(camera) if camera.active
            )
        })
    })
}

fn deactivate_other_cameras(scene: &mut Scene, active_entity: u64, active_component_idx: usize) {
    for (entity_id, entity) in scene.entities.iter_mut() {
        for (component_idx, component) in entity.components.iter_mut().enumerate() {
            if *entity_id == active_entity && component_idx == active_component_idx {
                continue;
            }
            if let sindri::component::Component::Camera(camera) = component {
                camera.active = false;
            }
        }
    }
}

pub fn normalize_scene_cameras(scene: &mut Scene) {
    let mut camera_refs: Vec<(u64, usize, bool)> =
        scene
            .entities
            .iter()
            .flat_map(|(entity_id, entity)| {
                entity.components.iter().enumerate().filter_map(
                    move |(component_idx, component)| {
                        if let sindri::component::Component::Camera(camera) = component {
                            Some((*entity_id, component_idx, camera.active))
                        } else {
                            None
                        }
                    },
                )
            })
            .collect();
    camera_refs.sort_by_key(|(entity_id, component_idx, _)| (*entity_id, *component_idx));

    let active = camera_refs
        .iter()
        .find(|(_, _, active)| *active)
        .or_else(|| camera_refs.first())
        .copied();
    let Some((active_entity, active_component_idx, _)) = active else {
        return;
    };

    for (entity_id, entity) in scene.entities.iter_mut() {
        for (component_idx, component) in entity.components.iter_mut().enumerate() {
            if let sindri::component::Component::Camera(camera) = component {
                camera.active =
                    *entity_id == active_entity && component_idx == active_component_idx;
            }
        }
    }
}

// GET /assets/*path  — serve files from project_root
pub async fn get_asset(
    State(state): State<AppState>,
    Path(rel): Path<String>,
) -> impl IntoResponse {
    let full = state.project_root.join(&rel);
    match std::fs::read(&full) {
        Ok(bytes) => {
            let mime = match full.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "png"  => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                "gif"  => "image/gif",
                "bmp"  => "image/bmp",
                _      => "application/octet-stream",
            };
            (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, mime)], bytes).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

// GET /health
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let mode = state
        .playback
        .lock()
        .map(|playback| playback.mode)
        .unwrap_or(PlaybackMode::Stopped);
    Json(serde_json::json!({
        "status": "ok",
        "model": state.model,
        "paused": mode != PlaybackMode::Playing,
        "playback": mode.as_str(),
        "error_count": state.errors.lock().map(|errors| errors.len()).unwrap_or(0),
    }))
}

// GET /errors
pub async fn get_errors(State(state): State<AppState>) -> impl IntoResponse {
    let errors = state
        .errors
        .lock()
        .map(|errors| errors.clone())
        .unwrap_or_default();
    Json(serde_json::json!({ "errors": errors })).into_response()
}

#[derive(Deserialize)]
pub struct ControlBody {
    pub paused: Option<bool>,
    pub action: Option<String>,
}

// POST /control
pub async fn post_control(
    State(state): State<AppState>,
    Json(body): Json<ControlBody>,
) -> impl IntoResponse {
    let action = body.action.as_deref();
    match action {
        Some("play") => {
            let snapshot = {
                let scene = state.scene.read().await;
                scene.clone()
            };
            if let Ok(mut playback) = state.playback.lock() {
                if playback.mode == PlaybackMode::Stopped {
                    playback.edit_scene = Some(snapshot);
                }
                playback.mode = PlaybackMode::Playing;
            }
            if let Ok(mut errors) = state.errors.lock() {
                errors.clear();
            }
        }
        Some("pause") => {
            if let Ok(mut playback) = state.playback.lock() {
                if playback.mode == PlaybackMode::Playing {
                    playback.mode = PlaybackMode::Paused;
                }
            }
        }
        Some("stop") => {
            let restore = if let Ok(mut playback) = state.playback.lock() {
                playback.mode = PlaybackMode::Stopped;
                playback.edit_scene.take()
            } else {
                None
            };
            if let Some(scene) = restore {
                let mut current = state.scene.write().await;
                *current = scene;
            }
            if let Ok(mut keys) = state.keys.try_write() {
                keys.clear();
            }
        }
        Some(_) => return StatusCode::BAD_REQUEST,
        None => {
            let Some(paused) = body.paused else {
                return StatusCode::BAD_REQUEST;
            };
            let snapshot = if paused {
                None
            } else {
                let scene = state.scene.read().await;
                Some(scene.clone())
            };
            if let Ok(mut playback) = state.playback.lock() {
                if paused {
                    if playback.mode == PlaybackMode::Playing {
                        playback.mode = PlaybackMode::Paused;
                    }
                } else {
                    if playback.mode == PlaybackMode::Stopped {
                        playback.edit_scene = snapshot;
                    }
                    playback.mode = PlaybackMode::Playing;
                }
            }
        }
    }
    StatusCode::OK
}

#[derive(Deserialize)]
pub struct GizmosBody {
    pub enabled: bool,
}

// POST /gizmos
pub async fn post_gizmos(
    State(state): State<AppState>,
    Json(body): Json<GizmosBody>,
) -> impl IntoResponse {
    state.gizmos.store(body.enabled, std::sync::atomic::Ordering::Relaxed);
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

// GET /scene/path — return the current scene file path as a JSON string
pub async fn get_scene_path(State(state): State<AppState>) -> impl IntoResponse {
    let path = state.scene_path.read().await;
    Json(path.to_string_lossy().to_string()).into_response()
}

// PUT /scene
pub async fn put_scene(State(state): State<AppState>, body: String) -> impl IntoResponse {
    match Scene::from_json(&body) {
        Ok(mut new_scene) => {
            normalize_scene_cameras(&mut new_scene);
            *state.scene.write().await = new_scene;
            StatusCode::OK.into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

// POST /scene/save
pub async fn save_scene(State(state): State<AppState>) -> impl IntoResponse {
    let scene = state.scene.read().await;
    let path = state.scene_path.read().await.clone();
    match scene.save(&path) {
        Ok(()) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Serialize)]
pub struct WorldSettings {
    pub gravity_x: f32,
    pub gravity_y: f32,
}

pub async fn get_world_settings(State(state): State<AppState>) -> impl IntoResponse {
    let scene = state.scene.read().await;
    Json(WorldSettings { gravity_x: scene.gravity_x, gravity_y: scene.gravity_y }).into_response()
}

#[derive(Deserialize)]
pub struct PatchWorldSettings {
    pub gravity_x: Option<f32>,
    pub gravity_y: Option<f32>,
}

pub async fn patch_world_settings(
    State(state): State<AppState>,
    Json(body): Json<PatchWorldSettings>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    if let Some(v) = body.gravity_x { scene.gravity_x = v; }
    if let Some(v) = body.gravity_y { scene.gravity_y = v; }
    StatusCode::OK.into_response()
}

#[derive(Deserialize)]
pub struct OpenSceneBody {
    pub path: String,
}

fn resolve_project_scene_path(
    project_root: &FsPath,
    relative_path: &str,
) -> Result<PathBuf, String> {
    let requested = FsPath::new(relative_path);
    if requested.is_absolute() {
        return Err("scene path must be project-relative".into());
    }

    let full = project_root.join(requested);
    let canon_root = std::fs::canonicalize(project_root).map_err(|e| e.to_string())?;
    let canon_file = std::fs::canonicalize(&full).map_err(|e| e.to_string())?;
    if !canon_file.starts_with(&canon_root) {
        return Err("scene path escapes project directory".into());
    }

    let ext = canon_file
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if ext != "sindri" {
        return Err("only .sindri scene files can be opened".into());
    }

    Ok(canon_file)
}

// POST /scene/open
pub async fn open_scene(
    State(state): State<AppState>,
    Json(body): Json<OpenSceneBody>,
) -> impl IntoResponse {
    let next_path = match resolve_project_scene_path(&state.project_root, &body.path) {
        Ok(path) => path,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };

    let mut next_scene = match Scene::load(&next_path) {
        Ok(scene) => scene,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    normalize_scene_cameras(&mut next_scene);

    // Persist the current active scene before switching, so opening another
    // scene does not drop recent edits that have not reached the autosave tick.
    let current_path = state.scene_path.read().await.clone();
    let current_scene = state.scene.read().await.clone();
    if let Err(e) = current_scene.save(&current_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    *state.scene.write().await = next_scene;
    *state.scene_path.write().await = next_path;
    StatusCode::OK.into_response()
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
    #[serde(default)]
    pub staged: bool,
}

// POST /scene/entity
pub async fn create_entity(
    State(state): State<AppState>,
    Json(body): Json<CreateEntityBody>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    let id = scene.spawn_staged(&body.name, body.staged);
    if let Some(parent_id) = body.parent_id {
        scene.set_parent(id, parent_id);
    }
    Json(serde_json::json!({ "id": id })).into_response()
}

#[derive(Deserialize)]
pub struct SetStagedBody {
    pub staged: bool,
}

// PATCH /scene/entity/:id/staged
pub async fn set_entity_staged(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(body): Json<SetStagedBody>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    match scene.entities.get_mut(&id) {
        Some(e) => { e.staged = body.staged; StatusCode::OK.into_response() }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

#[derive(Deserialize)]
pub struct BatchStagedBody {
    pub entity_ids: Vec<u64>,
}

// POST /scene/staged/commit  — clear staged flag on given entity IDs
pub async fn commit_staged(
    State(state): State<AppState>,
    Json(body): Json<BatchStagedBody>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    for id in &body.entity_ids {
        if let Some(e) = scene.entities.get_mut(id) {
            e.staged = false;
        }
    }
    StatusCode::OK.into_response()
}

// POST /scene/staged/revert  — delete staged entities from given IDs
pub async fn revert_staged(
    State(state): State<AppState>,
    Json(body): Json<BatchStagedBody>,
) -> impl IntoResponse {
    let mut scene = state.scene.write().await;
    for id in &body.entity_ids {
        scene.remove_entity(*id);
    }
    StatusCode::OK.into_response()
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
    let mut component = match body.component_type.as_str() {
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
        "AnimatedSprite" => Component::AnimatedSprite(AnimatedSprite::default()),
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
            active: true,
            zoom: 1.0,
            follow_entity: None,
            offset_x: 0.0,
            offset_y: 0.0,
            bounds_min_x: None,
            bounds_min_y: None,
            bounds_max_x: None,
            bounds_max_y: None,
            smoothing: 1.0,
            dead_zone_width: 0.0,
            dead_zone_height: 0.0,
            runtime_target_zoom: None,
            runtime_zoom_speed: 0.0,
            runtime_shake_intensity: 0.0,
            runtime_shake_timer: 0.0,
            runtime_shake_seed: 0.0,
        }),
        "AudioSource" => Component::AudioSource(AudioSource {
            path: String::new(),
            volume: 1.0,
            looping: false,
            play_on_start: false,
        }),
        "Tilemap" => Component::Tilemap(sindri::component::Tilemap::default()),
        _ => return (StatusCode::BAD_REQUEST, "unknown component type").into_response(),
    };
    let mut scene = state.scene.write().await;
    if let Component::Camera(camera) = &mut component {
        camera.active = !has_active_camera(&scene);
    }
    match scene.entities.get_mut(&id) {
        Some(e) => {
            let component_idx = e.components.len();
            let activates_camera = matches!(&component, Component::Camera(camera) if camera.active);
            e.components.push(component);
            if activates_camera {
                deactivate_other_cameras(&mut scene, id, component_idx);
            }
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
                    sindri::component::Component::AnimatedSprite(_) => "AnimatedSprite",
                    sindri::component::Component::Tilemap(_) => "Tilemap",
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

fn patch_f32(body: &serde_json::Value, key: &str, target: &mut f32) -> Result<(), String> {
    if let Some(value) = body.get(key) {
        let Some(number) = value.as_f64() else {
            return Err(format!("{key} must be a number"));
        };
        *target = number as f32;
    }
    Ok(())
}

fn patch_optional_f32(
    body: &serde_json::Value,
    key: &str,
    target: &mut Option<f32>,
) -> Result<(), String> {
    let Some(value) = body.get(key) else {
        return Ok(());
    };
    if value.is_null() {
        *target = None;
        return Ok(());
    }
    let Some(number) = value.as_f64() else {
        return Err(format!("{key} must be a number or null"));
    };
    *target = Some(number as f32);
    Ok(())
}

fn patch_bool(body: &serde_json::Value, key: &str, target: &mut bool) -> Result<(), String> {
    if let Some(value) = body.get(key) {
        let Some(boolean) = value.as_bool() else {
            return Err(format!("{key} must be a boolean"));
        };
        *target = boolean;
    }
    Ok(())
}

fn patch_string(body: &serde_json::Value, key: &str, target: &mut String) -> Result<(), String> {
    if let Some(value) = body.get(key) {
        let Some(string) = value.as_str() else {
            return Err(format!("{key} must be a string"));
        };
        *target = string.to_string();
    }
    Ok(())
}

fn patch_u8(body: &serde_json::Value, key: &str, target: &mut u8) -> Result<(), String> {
    if let Some(value) = body.get(key) {
        let Some(number) = value.as_u64() else {
            return Err(format!("{key} must be an unsigned integer"));
        };
        if number > u8::MAX as u64 {
            return Err(format!("{key} must be <= {}", u8::MAX));
        }
        *target = number as u8;
    }
    Ok(())
}

fn patch_u32(body: &serde_json::Value, key: &str, target: &mut u32) -> Result<(), String> {
    if let Some(value) = body.get(key) {
        let Some(number) = value.as_u64() else {
            return Err(format!("{key} must be an unsigned integer"));
        };
        if number > u32::MAX as u64 {
            return Err(format!("{key} must be <= {}", u32::MAX));
        }
        *target = number as u32;
    }
    Ok(())
}

fn patch_color(body: &serde_json::Value, key: &str, target: &mut [f32; 4]) -> Result<(), String> {
    let Some(value) = body.get(key) else {
        return Ok(());
    };
    let Some(values) = value.as_array() else {
        return Err(format!("{key} must be an array"));
    };
    if values.len() != 4 {
        return Err(format!("{key} must contain exactly 4 numbers"));
    }
    let mut color = [0.0; 4];
    for (idx, value) in values.iter().enumerate() {
        let Some(number) = value.as_f64() else {
            return Err(format!("{key}[{idx}] must be a number"));
        };
        color[idx] = number as f32;
    }
    *target = color;
    Ok(())
}

fn patch_follow_entity(
    body: &serde_json::Value,
    key: &str,
    target: &mut Option<u64>,
) -> Result<(), String> {
    let Some(value) = body.get(key) else {
        return Ok(());
    };
    if value.is_null() {
        *target = None;
        return Ok(());
    }
    let Some(number) = value.as_u64() else {
        return Err(format!("{key} must be an unsigned integer or null"));
    };
    *target = Some(number);
    Ok(())
}

// PATCH /scene/entity/:id/component/:idx
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
    let mut patched_camera = false;
    let mut requested_active_camera = false;
    let result = match comp {
        sindri::component::Component::Transform(t) => patch_f32(&body, "x", &mut t.x)
            .and_then(|_| patch_f32(&body, "y", &mut t.y))
            .and_then(|_| patch_f32(&body, "scale_x", &mut t.scale_x))
            .and_then(|_| patch_f32(&body, "scale_y", &mut t.scale_y))
            .and_then(|_| patch_f32(&body, "rotation", &mut t.rotation)),
        sindri::component::Component::Sprite(s) => {
            patch_string(&body, "texture_path", &mut s.texture_path)
                .and_then(|_| patch_f32(&body, "width", &mut s.width))
                .and_then(|_| patch_f32(&body, "height", &mut s.height))
                .and_then(|_| patch_bool(&body, "flip_x", &mut s.flip_x))
                .and_then(|_| patch_bool(&body, "flip_y", &mut s.flip_y))
                .and_then(|_| patch_color(&body, "color", &mut s.color))
        }
        sindri::component::Component::Script(s) => patch_string(&body, "path", &mut s.path),
        sindri::component::Component::PhysicsBody(p) => {
            if let Some(value) = body.get("body_type") {
                let Some(body_type) = value.as_str() else {
                    return (StatusCode::BAD_REQUEST, "body_type must be a string").into_response();
                };
                p.body_type = match body_type {
                    "Dynamic" => sindri::component::BodyType::Dynamic,
                    "Kinematic" => sindri::component::BodyType::Kinematic,
                    "Fixed" => sindri::component::BodyType::Fixed,
                    _ => return (StatusCode::BAD_REQUEST, "unknown body_type").into_response(),
                };
            }
            patch_bool(&body, "lock_rotation", &mut p.lock_rotation)
                .and_then(|_| patch_f32(&body, "linear_damping", &mut p.linear_damping))
                .and_then(|_| patch_f32(&body, "angular_damping", &mut p.angular_damping))
                .and_then(|_| patch_u8(&body, "collision_layer", &mut p.collision_layer))
                .and_then(|_| patch_u32(&body, "collision_mask", &mut p.collision_mask))
        }
        sindri::component::Component::Collider(c) => patch_f32(&body, "width", &mut c.width)
            .and_then(|_| patch_f32(&body, "height", &mut c.height))
            .and_then(|_| patch_f32(&body, "offset_x", &mut c.offset_x))
            .and_then(|_| patch_f32(&body, "offset_y", &mut c.offset_y))
            .and_then(|_| patch_bool(&body, "is_trigger", &mut c.is_trigger)),
        sindri::component::Component::Camera(c) => {
            requested_active_camera = body
                .get("active")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let result = patch_bool(&body, "active", &mut c.active)
                .and_then(|_| patch_f32(&body, "zoom", &mut c.zoom))
                .and_then(|_| patch_follow_entity(&body, "follow_entity", &mut c.follow_entity))
                .and_then(|_| patch_f32(&body, "offset_x", &mut c.offset_x))
                .and_then(|_| patch_f32(&body, "offset_y", &mut c.offset_y))
                .and_then(|_| patch_optional_f32(&body, "bounds_min_x", &mut c.bounds_min_x))
                .and_then(|_| patch_optional_f32(&body, "bounds_min_y", &mut c.bounds_min_y))
                .and_then(|_| patch_optional_f32(&body, "bounds_max_x", &mut c.bounds_max_x))
                .and_then(|_| patch_optional_f32(&body, "bounds_max_y", &mut c.bounds_max_y))
                .and_then(|_| patch_f32(&body, "smoothing", &mut c.smoothing))
                .and_then(|_| patch_f32(&body, "dead_zone_width", &mut c.dead_zone_width))
                .and_then(|_| patch_f32(&body, "dead_zone_height", &mut c.dead_zone_height));
            if result.is_ok() {
                c.zoom = c.zoom.max(0.01);
                c.smoothing = c.smoothing.clamp(0.0, 1.0);
                c.dead_zone_width = c.dead_zone_width.max(0.0);
                c.dead_zone_height = c.dead_zone_height.max(0.0);
                patched_camera = true;
            }
            result
        }
        sindri::component::Component::AnimatedSprite(a) => {
            patch_string(&body, "texture_path", &mut a.texture_path)
                .and_then(|_| patch_u32(&body, "cols", &mut a.cols))
                .and_then(|_| patch_u32(&body, "rows", &mut a.rows))
                .and_then(|_| patch_f32(&body, "width", &mut a.width))
                .and_then(|_| patch_f32(&body, "height", &mut a.height))
                .and_then(|_| patch_bool(&body, "flip_x", &mut a.flip_x))
                .and_then(|_| patch_bool(&body, "flip_y", &mut a.flip_y))
                .and_then(|_| patch_color(&body, "tint", &mut a.tint))
                .and_then(|_| patch_string(&body, "default_clip", &mut a.default_clip))
                .and_then(|_| patch_u32(&body, "margin", &mut a.margin))
                .and_then(|_| patch_u32(&body, "spacing", &mut a.spacing))
                .and_then(|_| {
                    if let Some(clips_val) = body.get("clips") {
                        let clips: Vec<sindri::component::AnimClip> =
                            serde_json::from_value(clips_val.clone())
                                .map_err(|e| format!("invalid clips: {e}"))?;
                        a.clips = clips;
                    }
                    Ok(())
                })
        }
        sindri::component::Component::Tilemap(t) => {
            patch_f32(&body, "tile_width", &mut t.tile_width)
                .and_then(|_| patch_f32(&body, "tile_height", &mut t.tile_height))
                .and_then(|_| patch_color(&body, "tint", &mut t.tint))
                .and_then(|_| {
                    // Resize map if map_cols/map_rows changed, preserving tile data in all layers
                    let new_cols = body.get("map_cols").and_then(|v| v.as_u64()).map(|v| v as u32);
                    let new_rows = body.get("map_rows").and_then(|v| v.as_u64()).map(|v| v as u32);
                    if new_cols.is_some() || new_rows.is_some() {
                        let cols = new_cols.unwrap_or(t.map_cols).max(1);
                        let rows = new_rows.unwrap_or(t.map_rows).max(1);
                        for layer in &mut t.layers {
                            let mut new_tiles = vec![0u32; (cols * rows) as usize];
                            for row in 0..rows.min(t.map_rows) {
                                for col in 0..cols.min(t.map_cols) {
                                    new_tiles[(row * cols + col) as usize] =
                                        layer.tiles[(row * t.map_cols + col) as usize];
                                }
                            }
                            layer.tiles = new_tiles;
                        }
                        t.map_cols = cols;
                        t.map_rows = rows;
                    }
                    // Full layers replacement
                    if let Some(layers_val) = body.get("layers") {
                        let layers: Vec<sindri::component::TileLayer> =
                            serde_json::from_value(layers_val.clone())
                                .map_err(|e| format!("invalid layers: {e}"))?;
                        t.layers = layers;
                    }
                    // Patch a single layer's tiles by index
                    if let (Some(layer_val), Some(tiles_val)) = (body.get("layer_idx"), body.get("tiles")) {
                        let layer_idx = layer_val.as_u64().ok_or_else(|| "layer_idx must be an integer".to_string())? as usize;
                        let tiles: Vec<u32> = serde_json::from_value(tiles_val.clone())
                            .map_err(|e| format!("invalid tiles: {e}"))?;
                        if let Some(layer) = t.layers.get_mut(layer_idx) {
                            layer.tiles = tiles;
                        }
                    }
                    if let Some(palettes_val) = body.get("palettes") {
                        let palettes: Vec<sindri::component::TilePalette> =
                            serde_json::from_value(palettes_val.clone())
                                .map_err(|e| format!("invalid palettes: {e}"))?;
                        t.palettes = palettes;
                    }
                    Ok(())
                })
        }
        sindri::component::Component::AudioSource(a) => patch_string(&body, "path", &mut a.path)
            .and_then(|_| patch_f32(&body, "volume", &mut a.volume))
            .and_then(|_| patch_bool(&body, "looping", &mut a.looping))
            .and_then(|_| patch_bool(&body, "play_on_start", &mut a.play_on_start)),
    };

    match result {
        Ok(()) => {
            if requested_active_camera {
                deactivate_other_cameras(&mut scene, p.id, p.idx);
            } else if patched_camera {
                normalize_scene_cameras(&mut scene);
            }
            StatusCode::OK.into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
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
    let scene = state.scene.read().await;
    match (state.screenshot_fn)(&scene) {
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
    let p = std::path::Path::new(&q.path);
    let relative = p.strip_prefix("scripts").unwrap_or(p);
    let full = state.scripts_root.join(relative);
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
    let p = std::path::Path::new(&q.path);
    let relative = p.strip_prefix("scripts").unwrap_or(p);
    let full = state.scripts_root.join(relative);
    if let Some(parent) = full.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&full, &body.content) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
