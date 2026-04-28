// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use forge2d::{
    create_scene, register_builtin_metadata, restore_scene_physics, Camera2D,
    ComponentMetadataRegistry, PhysicsWorld, Renderer, Vec2, World,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// Project configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ProjectConfig {
    name: String,
    version: String,
    created_at: String,
    // Future: engine version, settings, etc.
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum PlayState {
    Stopped,
    Playing,
    Paused,
}

impl PlayState {
    fn is_running(self) -> bool {
        matches!(self, Self::Playing | Self::Paused)
    }

    fn is_advancing(self) -> bool {
        matches!(self, Self::Playing)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ViewportCacheKey {
    width: u32,
    height: u32,
    camera_x_bits: u32,
    camera_y_bits: u32,
    zoom_bits: u32,
    rotation_bits: u32,
    world_revision: u64,
}

// Editor state
struct EditorState {
    world: World,
    physics: PhysicsWorld,
    metadata_registry: ComponentMetadataRegistry,
    scene_dirty: bool,
    play_state: PlayState,
    play_snapshot: Option<forge2d::Scene>, // Snapshot taken before play mode
    play_snapshot_dirty: Option<bool>,
    // Texture registry: maps entity ID -> texture file path (for sprites)
    entity_texture_paths: std::collections::HashMap<u32, String>,
    // Name registry: maps entity ID -> display name
    entity_names: HashMap<u32, String>,
    // Project management
    project_path: Option<PathBuf>,
    project_config: Option<ProjectConfig>,
    offscreen_renderer: Option<Renderer>,
    offscreen_textures: HashMap<String, forge2d::TextureHandle>,
    offscreen_fallback: Option<forge2d::TextureHandle>,
    world_revision: u64,
    viewport_cache_key: Option<ViewportCacheKey>,
    viewport_cache_rgba: Option<Vec<u8>>,
    undo_stack: Vec<EditorSnapshot>,
    redo_stack: Vec<EditorSnapshot>,
}

#[derive(Clone)]
struct EditorSnapshot {
    scene: forge2d::Scene,
    dirty: bool,
}

impl EditorState {
    fn new() -> Self {
        let mut registry = ComponentMetadataRegistry::new();
        register_builtin_metadata(&mut registry);

        Self {
            world: World::new(),
            physics: PhysicsWorld::new(),
            metadata_registry: registry,
            scene_dirty: false,
            play_state: PlayState::Stopped,
            play_snapshot: None,
            play_snapshot_dirty: None,
            entity_texture_paths: std::collections::HashMap::new(),
            entity_names: HashMap::new(),
            project_path: None,
            project_config: None,
            offscreen_renderer: None,
            offscreen_textures: HashMap::new(),
            offscreen_fallback: None,
            world_revision: 0,
            viewport_cache_key: None,
            viewport_cache_rgba: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}

static EDITOR_STATE: OnceLock<Mutex<EditorState>> = OnceLock::new();
static SELECTED_ENTITIES: OnceLock<Mutex<Vec<u32>>> = OnceLock::new();

fn get_state() -> MutexGuard<'static, EditorState> {
    EDITOR_STATE
        .get_or_init(|| Mutex::new(EditorState::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn selection_store() -> MutexGuard<'static, Vec<u32>> {
    SELECTED_ENTITIES
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn can_edit_scene(state: &EditorState) -> bool {
    !state.play_state.is_running()
}

fn bump_world_revision(state: &mut EditorState) {
    state.world_revision = state.world_revision.wrapping_add(1);
    state.viewport_cache_key = None;
    state.viewport_cache_rgba = None;
}

fn is_descendant(
    state: &EditorState,
    candidate_parent: forge2d::EntityId,
    entity: forge2d::EntityId,
) -> bool {
    let mut current = Some(candidate_parent);
    while let Some(node) = current {
        if node == entity {
            return true;
        }
        current = forge2d::hierarchy::get_parent(&state.world, node);
    }
    false
}

fn find_entity_by_id(state: &EditorState, entity_id: u32) -> Option<forge2d::EntityId> {
    for eid in state.world.entities() {
        if eid.to_u32() == entity_id {
            return Some(eid);
        }
    }
    None
}

fn capture_editor_snapshot(state: &EditorState) -> EditorSnapshot {
    let mut scene = create_scene(&state.physics);
    scene.entities = serialize_entities(state);
    EditorSnapshot {
        scene,
        dirty: state.scene_dirty,
    }
}

fn clear_scene_history(state: &mut EditorState) {
    state.undo_stack.clear();
    state.redo_stack.clear();
}

fn commit_scene_edit(state: &mut EditorState, snapshot: EditorSnapshot) {
    state.undo_stack.push(snapshot);
    if state.undo_stack.len() > 100 {
        state.undo_stack.remove(0);
    }
    state.redo_stack.clear();
    state.scene_dirty = true;
    bump_world_revision(state);
}

fn restore_editor_snapshot(state: &mut EditorState, snapshot: &EditorSnapshot) -> Result<(), String> {
    state.world = World::new();
    state.physics = PhysicsWorld::new();
    restore_scene_physics(&mut state.physics, &snapshot.scene).map_err(|e| e.to_string())?;
    restore_entities(state, &snapshot.scene)?;
    sync_physics_bodies(state)?;
    state.scene_dirty = snapshot.dirty;
    bump_world_revision(state);
    Ok(())
}

// IPC Commands

#[derive(Serialize, Deserialize)]
struct EntityInfo {
    id: u32,
    name: String,
    has_transform: bool,
    has_sprite: bool,
    has_physics: bool,
    has_camera: bool,
    parent_id: Option<u32>,
    children: Vec<u32>,
}

#[derive(Serialize, Deserialize)]
struct FileNode {
    name: String,
    path: String,
    is_dir: bool,
    children: Vec<FileNode>,
}

#[derive(Serialize, Deserialize)]
struct ProjectFileTree {
    scenes: FileNode,
    assets: FileNode,
}

#[derive(Serialize, Deserialize)]
struct ViewportFrame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn build_file_tree(path: &Path, depth: usize) -> Result<FileNode, String> {
    let metadata = fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata for {}: {}", path.display(), e))?;
    let is_dir = metadata.is_dir();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let mut node = FileNode {
        name,
        path: path.to_string_lossy().to_string(),
        is_dir,
        children: Vec::new(),
    };

    if is_dir && depth > 0 {
        let mut entries: Vec<PathBuf> = fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory {}: {}", path.display(), e))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| {
                !p.file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
            })
            .collect();

        entries.sort_by(|a, b| {
            let a_dir = a.is_dir();
            let b_dir = b.is_dir();
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry_path in entries {
            let child = build_file_tree(&entry_path, depth - 1)?;
            node.children.push(child);
        }
    }

    Ok(node)
}

#[derive(Serialize, Deserialize)]
struct TransformSerde {
    position: forge2d::Vec2,
    rotation: f32,
    scale: forge2d::Vec2,
    parent: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct SpriteSerde {
    tint: [f32; 4],
    sprite_scale: forge2d::Vec2,
    visible: bool,
    texture_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct PrefabAsset {
    version: u32,
    roots: Vec<u32>,
    entities: Vec<forge2d::scene::SerializableEntity>,
}

#[derive(Serialize, Deserialize)]
struct CameraSerde {
    zoom: f32,
    offset: forge2d::Vec2,
    rotation: f32,
    active: bool,
}

#[derive(Serialize, Deserialize)]
struct PhysicsBodySerde {
    body_type: forge2d::physics::RigidBodyType,
    collider_shape: Option<forge2d::physics::ColliderShape>,
}

fn collect_entity_ids(state: &EditorState) -> HashSet<forge2d::EntityId> {
    let mut ids = HashSet::new();
    for (entity, _) in state.world.query::<forge2d::entities::Transform>() {
        ids.insert(entity);
    }
    for (entity, _) in state.world.query::<forge2d::entities::SpriteComponent>() {
        ids.insert(entity);
    }
    for (entity, _) in state.world.query::<forge2d::entities::PhysicsBody>() {
        ids.insert(entity);
    }
    for (entity, _) in state.world.query::<forge2d::entities::CameraComponent>() {
        ids.insert(entity);
    }
    for (entity, _) in state.world.query::<forge2d::script::ScriptTag>() {
        ids.insert(entity);
    }
    ids
}

fn collect_subtree_ids(
    state: &EditorState,
    root: forge2d::EntityId,
) -> HashSet<forge2d::EntityId> {
    let mut ids = HashSet::new();
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if !ids.insert(entity) {
            continue;
        }
        let children = forge2d::hierarchy::get_children(&state.world, entity);
        for child in children {
            stack.push(child);
        }
    }
    ids
}

fn serialize_entities_subset(
    state: &EditorState,
    ids: &HashSet<forge2d::EntityId>,
) -> Vec<forge2d::scene::SerializableEntity> {
    let mut entities = Vec::new();
    for &entity in ids {
        let mut components = Vec::new();

        if let Some(transform) = state.world.get::<forge2d::entities::Transform>(entity) {
            let mut parent = transform.parent.map(|p| p.to_u32());
            if let Some(parent_id) = parent {
                if !ids.contains(&forge2d::EntityId(parent_id)) {
                    parent = None;
                }
            }
            let data = TransformSerde {
                position: transform.position,
                rotation: transform.rotation,
                scale: transform.scale,
                parent,
            };
            if let Ok(value) = serde_json::to_value(data) {
                components.push(forge2d::scene::SerializableComponent {
                    type_name: "Transform".to_string(),
                    data: value,
                });
            }
        }

        if let Some(sprite) = state.world.get::<forge2d::entities::SpriteComponent>(entity) {
            let texture_path = state.entity_texture_paths.get(&entity.to_u32()).cloned();
            let data = SpriteSerde {
                tint: sprite.sprite.tint,
                sprite_scale: sprite.sprite.transform.scale,
                visible: sprite.visible,
                texture_path,
            };
            if let Ok(value) = serde_json::to_value(data) {
                components.push(forge2d::scene::SerializableComponent {
                    type_name: "SpriteComponent".to_string(),
                    data: value,
                });
            }
        }

        if let Some(cam) = state.world.get::<forge2d::entities::CameraComponent>(entity) {
            let data = CameraSerde {
                zoom: cam.camera.zoom,
                offset: cam.camera.offset,
                rotation: cam.camera.rotation,
                active: cam.active,
            };
            if let Ok(value) = serde_json::to_value(data) {
                components.push(forge2d::scene::SerializableComponent {
                    type_name: "CameraComponent".to_string(),
                    data: value,
                });
            }
        }

        if let Some(body) = state.world.get::<forge2d::entities::PhysicsBody>(entity) {
            let data = PhysicsBodySerde {
                body_type: body.body_type,
                collider_shape: body.collider_shape,
            };
            if let Ok(value) = serde_json::to_value(data) {
                components.push(forge2d::scene::SerializableComponent {
                    type_name: "PhysicsBody".to_string(),
                    data: value,
                });
            }
        }

        if let Some(tag) = state.world.get::<forge2d::script::ScriptTag>(entity) {
            let data = serde_json::json!({ "tag": tag.0 });
            components.push(forge2d::scene::SerializableComponent {
                type_name: "ScriptTag".to_string(),
                data,
            });
        }

        if let Some(name) = state.entity_names.get(&entity.to_u32()) {
            let data = serde_json::json!({ "name": name });
            components.push(forge2d::scene::SerializableComponent {
                type_name: "EntityName".to_string(),
                data,
            });
        }

        entities.push(forge2d::scene::SerializableEntity {
            id: entity,
            components,
        });
    }
    entities
}

fn serialize_entities(state: &EditorState) -> Vec<forge2d::scene::SerializableEntity> {
    let mut entities = Vec::new();
    let ids = collect_entity_ids(state);

    for entity in ids {
        let mut components = Vec::new();

        if let Some(transform) = state.world.get::<forge2d::entities::Transform>(entity) {
            let data = TransformSerde {
                position: transform.position,
                rotation: transform.rotation,
                scale: transform.scale,
                parent: transform.parent.map(|p| p.to_u32()),
            };
            if let Ok(value) = serde_json::to_value(data) {
                components.push(forge2d::scene::SerializableComponent {
                    type_name: "Transform".to_string(),
                    data: value,
                });
            }
        }

        if let Some(sprite) = state.world.get::<forge2d::entities::SpriteComponent>(entity) {
            let texture_path = state.entity_texture_paths.get(&entity.to_u32()).cloned();
            let data = SpriteSerde {
                tint: sprite.sprite.tint,
                sprite_scale: sprite.sprite.transform.scale,
                visible: sprite.visible,
                texture_path,
            };
            if let Ok(value) = serde_json::to_value(data) {
                components.push(forge2d::scene::SerializableComponent {
                    type_name: "SpriteComponent".to_string(),
                    data: value,
                });
            }
        }

        if let Some(cam) = state.world.get::<forge2d::entities::CameraComponent>(entity) {
            let data = CameraSerde {
                zoom: cam.camera.zoom,
                offset: cam.camera.offset,
                rotation: cam.camera.rotation,
                active: cam.active,
            };
            if let Ok(value) = serde_json::to_value(data) {
                components.push(forge2d::scene::SerializableComponent {
                    type_name: "CameraComponent".to_string(),
                    data: value,
                });
            }
        }

        if let Some(body) = state.world.get::<forge2d::entities::PhysicsBody>(entity) {
            let data = PhysicsBodySerde {
                body_type: body.body_type,
                collider_shape: body.collider_shape,
            };
            if let Ok(value) = serde_json::to_value(data) {
                components.push(forge2d::scene::SerializableComponent {
                    type_name: "PhysicsBody".to_string(),
                    data: value,
                });
            }
        }

        if let Some(tag) = state.world.get::<forge2d::script::ScriptTag>(entity) {
            let data = serde_json::json!({ "tag": tag.0 });
            components.push(forge2d::scene::SerializableComponent {
                type_name: "ScriptTag".to_string(),
                data,
            });
        }

        if let Some(name) = state.entity_names.get(&entity.to_u32()) {
            let data = serde_json::json!({ "name": name });
            components.push(forge2d::scene::SerializableComponent {
                type_name: "EntityName".to_string(),
                data,
            });
        }

        entities.push(forge2d::scene::SerializableEntity {
            id: entity,
            components,
        });
    }

    entities
}

fn restore_entities(state: &mut EditorState, scene: &forge2d::Scene) -> Result<(), String> {
    state.entity_texture_paths = HashMap::new();
    state.entity_names = HashMap::new();
    for entity in &scene.entities {
        state.world.restore_entity(entity.id);
    }

    for entity in &scene.entities {
        for component in &entity.components {
            match component.type_name.as_str() {
                "Transform" => {
                    let data: TransformSerde = serde_json::from_value(component.data.clone())
                        .map_err(|e| e.to_string())?;
                    let mut transform = forge2d::entities::Transform::new(data.position);
                    transform.rotation = data.rotation;
                    transform.scale = data.scale;
                    transform.parent = data.parent.map(forge2d::EntityId);
                    state.world.insert(entity.id, transform);
                }
                "SpriteComponent" => {
                    let data: SpriteSerde = serde_json::from_value(component.data.clone())
                        .map_err(|e| e.to_string())?;
                    let mut sprite = forge2d::entities::SpriteComponent::new(
                        forge2d::render::TextureHandle::new(0),
                    );
                    sprite.sprite.tint = data.tint;
                    sprite.sprite.transform.scale = data.sprite_scale;
                    sprite.visible = data.visible;
                    state.world.insert(entity.id, sprite);
                    if let Some(path) = data.texture_path {
                        state.entity_texture_paths.insert(entity.id.to_u32(), path);
                    }
                }
                "CameraComponent" => {
                    let data: CameraSerde = serde_json::from_value(component.data.clone())
                        .map_err(|e| e.to_string())?;
                    let mut cam =
                        forge2d::entities::CameraComponent::new(forge2d::Vec2::ZERO);
                    cam.camera.zoom = data.zoom;
                    cam.camera.offset = data.offset;
                    cam.camera.rotation = data.rotation;
                    cam.active = data.active;
                    state.world.insert(entity.id, cam);
                }
                "PhysicsBody" => {
                    let data: PhysicsBodySerde = serde_json::from_value(component.data.clone())
                        .map_err(|e| e.to_string())?;
                    let mut body = forge2d::entities::PhysicsBody::new(data.body_type);
                    body.collider_shape = data.collider_shape;
                    state.world.insert(entity.id, body);
                }
                "ScriptTag" => {
                    let tag = component
                        .data
                        .get("tag")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    state.world.insert(entity.id, forge2d::script::ScriptTag(tag));
                }
                "EntityName" => {
                    let name = component
                        .data
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !name.is_empty() {
                        state.entity_names.insert(entity.id.to_u32(), name);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn ensure_physics_body(state: &mut EditorState, entity: forge2d::EntityId) -> Result<(), String> {
    let Some(body) = state.world.get::<forge2d::entities::PhysicsBody>(entity).copied() else {
        return Ok(());
    };
    let transform = state.world.get::<forge2d::entities::Transform>(entity);
    let position = transform.map(|t| t.position).unwrap_or(forge2d::Vec2::ZERO);
    let rotation = transform.map(|t| t.rotation).unwrap_or(0.0);

    if !state.physics.has_body(entity) {
        state
            .physics
            .create_body(entity, body.body_type, position, rotation)
            .map_err(|e| e.to_string())?;
    }

    if state.physics.get_colliders(entity).is_empty() {
        let shape = body.collider_shape.unwrap_or_else(|| {
            let (sx, sy) = transform
                .map(|t| (t.scale.x, t.scale.y))
                .unwrap_or((1.0, 1.0));
            forge2d::physics::ColliderShape::Box {
                hx: 16.0 * sx,
                hy: 16.0 * sy,
            }
        });
        state
            .physics
            .add_collider_with_material(entity, shape, forge2d::Vec2::ZERO, 1.0, 0.5, 0.0)
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn sync_physics_bodies(state: &mut EditorState) -> Result<(), String> {
    let entities: Vec<_> = state
        .world
        .query::<forge2d::entities::PhysicsBody>()
        .iter()
        .map(|(entity, _)| *entity)
        .collect();
    for entity in entities {
        ensure_physics_body(state, entity)?;
    }
    Ok(())
}

#[tauri::command]
fn entities_list() -> Vec<EntityInfo> {
    let state = get_state();
    let mut entities = Vec::new();

    for entity_id in state.world.entities() {
        let id = entity_id.to_u32();
        let transform = state.world.get::<forge2d::entities::Transform>(entity_id);
        let has_transform = transform.is_some();
        let has_sprite = state
            .world
            .get::<forge2d::entities::SpriteComponent>(entity_id)
            .is_some();
        let has_physics = state
            .world
            .get::<forge2d::entities::PhysicsBody>(entity_id)
            .is_some();
        let has_camera = state
            .world
            .get::<forge2d::entities::CameraComponent>(entity_id)
            .is_some();
        let parent_id = transform.and_then(|t| t.parent).map(|e| e.to_u32());
        let children = forge2d::hierarchy::get_children(&state.world, entity_id)
            .iter()
            .map(|e| e.to_u32())
            .collect();

        let name = state
            .entity_names
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("Entity {}", id));
        entities.push(EntityInfo {
            id,
            name,
            has_transform,
            has_sprite,
            has_physics,
            has_camera,
            parent_id,
            children,
        });
    }

    entities
}

#[tauri::command]
fn world_revision_get() -> u64 {
    let state = get_state();
    state.world_revision
}

#[tauri::command]
fn entity_delete(entity_id: u32) -> Result<(), String> {
    let mut state = get_state();
    if !can_edit_scene(&state) {
        return Err("Cannot delete entities in play mode".to_string());
    }

    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);
    state.world.despawn(entity);
    state.physics.remove_body(entity);
    state.entity_texture_paths.remove(&entity_id);
    state.entity_names.remove(&entity_id);
    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

#[tauri::command]
fn entity_duplicate(entity_id: u32) -> Result<u32, String> {
    let mut state = get_state();
    let source_entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);

    let new_entity_id = state.world.spawn();

    let source_transform = state
        .world
        .get::<forge2d::entities::Transform>(source_entity)
        .cloned();
    if let Some(transform) = source_transform {
        let mut new_transform = transform.clone();
        // Offset position slightly so it's visible
        new_transform.position.x += 50.0;
        new_transform.position.y += 50.0;
        state.world.insert(new_entity_id, new_transform);
    }

    // Copy SpriteComponent if it exists
    let source_sprite = state
        .world
        .get::<forge2d::entities::SpriteComponent>(source_entity)
        .cloned();
    if let Some(sprite) = source_sprite {
        state.world.insert(new_entity_id, sprite);
    }

    let source_physics = state
        .world
        .get::<forge2d::entities::PhysicsBody>(source_entity)
        .copied();
    if let Some(physics) = source_physics {
        state.world.insert(new_entity_id, physics);
    }

    let source_camera = state
        .world
        .get::<forge2d::entities::CameraComponent>(source_entity)
        .cloned();
    if let Some(cam) = source_camera {
        state.world.insert(new_entity_id, cam);
    }

    if let Some(texture_path) = state.entity_texture_paths.get(&entity_id).cloned() {
        state
            .entity_texture_paths
            .insert(new_entity_id.to_u32(), texture_path);
    }
    let base_name = state
        .entity_names
        .get(&entity_id)
        .cloned()
        .unwrap_or_else(|| format!("Entity {}", entity_id));
    state.entity_names.insert(new_entity_id.to_u32(), format!("{} (Copy)", base_name));
    if state
        .world
        .get::<forge2d::entities::PhysicsBody>(new_entity_id)
        .is_some()
    {
        ensure_physics_body(&mut state, new_entity_id)?;
    }

    commit_scene_edit(&mut state, snapshot);
    Ok(new_entity_id.to_u32())
}

#[tauri::command]
fn entity_create() -> Result<u32, String> {
    let mut state = get_state();
    if !can_edit_scene(&state) {
        return Err("Cannot create entities in play mode".to_string());
    }
    let snapshot = capture_editor_snapshot(&state);
    let entity_id = state.world.spawn();
    state.world.insert(
        entity_id,
        forge2d::entities::Transform::new(forge2d::Vec2::ZERO),
    );
    state.entity_names.insert(entity_id.to_u32(), format!("Entity {}", entity_id.to_u32()));

    commit_scene_edit(&mut state, snapshot);
    Ok(entity_id.to_u32())
}

#[tauri::command]
fn entity_create_preset(preset: String, position: Option<[f32; 2]>) -> Result<u32, String> {
    let mut state = get_state();
    if !can_edit_scene(&state) {
        return Err("Cannot create entities in play mode".to_string());
    }
    let snapshot = capture_editor_snapshot(&state);
    let entity_id = state.world.spawn();

    let pos = position.unwrap_or([0.0, 0.0]);
    state.world.insert(
        entity_id,
        forge2d::entities::Transform::new(forge2d::Vec2::new(pos[0], pos[1])),
    );

    let default_name = match preset.as_str() {
        "sprite" => "Sprite".to_string(),
        "camera" => "Camera".to_string(),
        "physics" => "Physics Body".to_string(),
        "tilemap" => "Tilemap".to_string(),
        "script" => "Script".to_string(),
        _ => format!("Entity {}", entity_id.to_u32()),
    };
    state.entity_names.insert(entity_id.to_u32(), default_name);

    match preset.as_str() {
        "sprite" => {
            state.world.insert(
                entity_id,
                forge2d::entities::SpriteComponent::new(forge2d::TextureHandle::new(0)),
            );
        }
        "camera" => {
            state.world.insert(
                entity_id,
                forge2d::entities::CameraComponent::new(forge2d::Vec2::new(pos[0], pos[1])),
            );
        }
        "physics" => {
            state.world.insert(
                entity_id,
                forge2d::entities::PhysicsBody::new(forge2d::physics::RigidBodyType::Dynamic),
            );
        }
        "tilemap" => {
            let tilemap = forge2d::Tilemap::new(
                forge2d::TextureHandle::new(0),
                (1, 1),
                forge2d::Vec2::new(32.0, 32.0),
                (10, 10),
                forge2d::Vec2::ZERO,
            );
            state
                .world
                .insert(entity_id, forge2d::entities::TilemapComponent::new(tilemap));
        }
        "script" => {
            state
                .world
                .insert(entity_id, forge2d::script::ScriptTag("".to_string()));
        }
        _ => {}
    }

    if state
        .world
        .get::<forge2d::entities::PhysicsBody>(entity_id)
        .is_some()
    {
        ensure_physics_body(&mut state, entity_id)?;
    }

    commit_scene_edit(&mut state, snapshot);
    Ok(entity_id.to_u32())
}

#[tauri::command]
fn undo() -> Result<(), String> {
    let mut state = get_state();
    let Some(snapshot) = state.undo_stack.pop() else {
        return Err("Nothing to undo".to_string());
    };
    let current = capture_editor_snapshot(&state);
    state.redo_stack.push(current);
    restore_editor_snapshot(&mut state, &snapshot)?;
    Ok(())
}

#[tauri::command]
fn redo() -> Result<(), String> {
    let mut state = get_state();
    let Some(snapshot) = state.redo_stack.pop() else {
        return Err("Nothing to redo".to_string());
    };
    let current = capture_editor_snapshot(&state);
    state.undo_stack.push(current);
    restore_editor_snapshot(&mut state, &snapshot)?;
    Ok(())
}

#[tauri::command]
fn can_undo() -> bool {
    let state = get_state();
    !state.undo_stack.is_empty()
}

#[tauri::command]
fn can_redo() -> bool {
    let state = get_state();
    !state.redo_stack.is_empty()
}

#[tauri::command]
fn selection_get() -> Vec<u32> {
    selection_store().clone()
}

#[tauri::command]
fn selection_set(ids: Vec<u32>) {
    *selection_store() = ids;
}

#[tauri::command]
fn selection_add(id: u32) {
    let mut selection = selection_store();
    if !selection.contains(&id) {
        selection.push(id);
    }
}

#[tauri::command]
fn selection_clear() {
    selection_store().clear();
}

#[derive(Serialize, Deserialize)]
struct ScriptTagData {
    tag: String,
}

#[tauri::command]
fn entity_reparent(entity_id: u32, parent_id: Option<u32>) -> Result<(), String> {
    let mut state = get_state();
    if !can_edit_scene(&state) {
        return Err("Cannot reparent entities in play mode".to_string());
    }

    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let parent = match parent_id {
        Some(id) => Some(
            find_entity_by_id(&state, id).ok_or_else(|| "Parent entity not found".to_string())?,
        ),
        None => None,
    };

    if parent == Some(entity) {
        return Err("Entity cannot be parented to itself".to_string());
    }
    if let Some(parent_entity) = parent {
        if is_descendant(&state, parent_entity, entity) {
            return Err("Cannot create hierarchy cycles".to_string());
        }
    }

    let snapshot = capture_editor_snapshot(&state);
    forge2d::hierarchy::reparent(&mut state.world, entity, parent);
    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

// Transform operations
#[derive(Serialize, Deserialize)]
struct TransformData {
    position: [f32; 2],
    rotation: f32,
    scale: [f32; 2],
}

#[derive(Serialize, Deserialize)]
struct TransformHierarchyData {
    local_position: [f32; 2],
    local_rotation: f32,
    local_scale: [f32; 2],
    world_position: [f32; 2],
    world_rotation: f32,
    world_scale: [f32; 2],
    parent_world_position: [f32; 2],
    parent_world_rotation: f32,
    parent_world_scale: [f32; 2],
}

#[derive(Serialize, Deserialize)]
struct TransformHierarchyEntry {
    entity_id: u32,
    transform: TransformHierarchyData,
}

#[derive(Serialize, Deserialize)]
struct CameraData {
    active: bool,
    zoom: f32,
    offset: [f32; 2],
    rotation: f32,
}

#[derive(Serialize, Deserialize)]
struct CameraInfo {
    entity_id: u32,
    world_position: [f32; 2],
    rotation: f32,
    zoom: f32,
    offset: [f32; 2],
    active: bool,
}

#[tauri::command]
fn transform_get(entity_id: u32) -> Option<TransformData> {
    let state = get_state();
    if let Some(entity) = find_entity_by_id(&state, entity_id) {
        if let Some(transform) = state.world.get::<forge2d::entities::Transform>(entity) {
            return Some(TransformData {
                position: [transform.position.x, transform.position.y],
                rotation: transform.rotation,
                scale: [transform.scale.x, transform.scale.y],
            });
        }
    }
    None
}

fn build_transform_hierarchy_data(
    state: &EditorState,
    entity: forge2d::EntityId,
) -> Option<TransformHierarchyData> {
    let transform = state.world.get::<forge2d::entities::Transform>(entity)?;
    let parent = transform.parent;
    let world_pos = forge2d::get_world_position(&state.world, entity);
    let world_rot = forge2d::get_world_rotation(&state.world, entity);
    let world_scale = forge2d::get_world_scale(&state.world, entity);

    let (parent_world_pos, parent_world_rot, parent_world_scale) = if let Some(parent) = parent {
        (
            forge2d::get_world_position(&state.world, parent),
            forge2d::get_world_rotation(&state.world, parent),
            forge2d::get_world_scale(&state.world, parent),
        )
    } else {
        (Vec2::ZERO, 0.0, Vec2::new(1.0, 1.0))
    };

    Some(TransformHierarchyData {
        local_position: [transform.position.x, transform.position.y],
        local_rotation: transform.rotation,
        local_scale: [transform.scale.x, transform.scale.y],
        world_position: [world_pos.x, world_pos.y],
        world_rotation: world_rot,
        world_scale: [world_scale.x, world_scale.y],
        parent_world_position: [parent_world_pos.x, parent_world_pos.y],
        parent_world_rotation: parent_world_rot,
        parent_world_scale: [parent_world_scale.x, parent_world_scale.y],
    })
}

#[tauri::command]
fn transform_get_hierarchy(entity_id: u32) -> Option<TransformHierarchyData> {
    let state = get_state();
    let entity = find_entity_by_id(&state, entity_id)?;
    build_transform_hierarchy_data(&state, entity)
}

#[tauri::command]
fn transforms_list() -> Vec<TransformHierarchyEntry> {
    let state = get_state();
    state
        .world
        .query::<forge2d::entities::Transform>()
        .into_iter()
        .filter_map(|(entity, _)| {
            build_transform_hierarchy_data(&state, entity).map(|transform| TransformHierarchyEntry {
                entity_id: entity.to_u32(),
                transform,
            })
        })
        .collect()
}

fn camera_get_inner(state: &EditorState, entity_id: u32) -> Option<CameraData> {
    let entity = find_entity_by_id(state, entity_id)?;
    let cam = state.world.get::<forge2d::entities::CameraComponent>(entity)?;
    Some(CameraData {
        active: cam.active,
        zoom: cam.camera.zoom,
        offset: [cam.camera.offset.x, cam.camera.offset.y],
        rotation: cam.camera.rotation,
    })
}

#[tauri::command]
fn camera_get(entity_id: u32) -> Option<CameraData> {
    let state = get_state();
    camera_get_inner(&state, entity_id)
}

#[tauri::command]
fn camera_set(
    entity_id: u32,
    active: bool,
    zoom: f32,
    offset: [f32; 2],
    rotation: f32,
) -> Result<(), String> {
    let mut state = get_state();
    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);
    let cam = state
        .world
        .get_mut::<forge2d::entities::CameraComponent>(entity)
        .ok_or_else(|| "CameraComponent not found".to_string())?;
    cam.active = active;
    cam.camera.zoom = zoom.max(0.01);
    cam.camera.offset = forge2d::Vec2::new(offset[0], offset[1]);
    cam.camera.rotation = rotation;
    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

#[tauri::command]
fn camera_entities() -> Vec<CameraInfo> {
    let state = get_state();
    let mut cameras = Vec::new();
    for (entity, cam) in state.world.query::<forge2d::entities::CameraComponent>() {
        let world_pos = if state
            .world
            .get::<forge2d::entities::Transform>(entity)
            .is_some()
        {
            forge2d::get_world_position(&state.world, entity)
        } else {
            cam.camera.position
        };
        cameras.push(CameraInfo {
            entity_id: entity.to_u32(),
            world_position: [world_pos.x, world_pos.y],
            rotation: cam.camera.rotation,
            zoom: cam.camera.zoom,
            offset: [cam.camera.offset.x, cam.camera.offset.y],
            active: cam.active,
        });
    }
    cameras
}

#[tauri::command]
fn active_camera_info() -> Option<CameraInfo> {
    let mut cameras = camera_entities();
    if cameras.is_empty() {
        return None;
    }
    cameras.sort_by_key(|cam| (!cam.active, cam.entity_id));
    cameras.into_iter().next()
}

#[derive(Serialize, Deserialize)]
struct SpriteData {
    texture_handle: u32,
    texture_path: Option<String>,   // Path to texture file
    texture_size: Option<[u32; 2]>, // Width, height
    tint: [f32; 4],
    sprite_scale: [f32; 2], // Scale from sprite.transform
}

fn sprite_get_inner(state: &EditorState, entity_id: u32) -> Option<SpriteData> {
    let entity = find_entity_by_id(state, entity_id)?;
    let sprite_comp = state.world.get::<forge2d::entities::SpriteComponent>(entity)?;
    let texture_path = state.entity_texture_paths.get(&entity_id).cloned();
    Some(SpriteData {
        texture_handle: 0,
        texture_path,
        texture_size: None,
        tint: sprite_comp.sprite.tint,
        sprite_scale: [
            sprite_comp.sprite.transform.scale.x,
            sprite_comp.sprite.transform.scale.y,
        ],
    })
}

#[tauri::command]
fn sprite_get(entity_id: u32) -> Option<SpriteData> {
    let state = get_state();
    sprite_get_inner(&state, entity_id)
}

// Set texture path for an entity (called when sprite is created/updated)
#[tauri::command]
fn sprite_set_texture_path(entity_id: u32, path: String) -> Result<(), String> {
    let mut state = get_state();
    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    // Verify entity has SpriteComponent
    if state
        .world
        .get::<forge2d::entities::SpriteComponent>(entity)
        .is_none()
    {
        return Err("Entity does not have SpriteComponent".to_string());
    }

    let snapshot = capture_editor_snapshot(&state);
    if path.trim().is_empty() {
        state.entity_texture_paths.remove(&entity_id);
    } else {
        state.entity_texture_paths.insert(entity_id, path);
    }
    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

fn script_tag_get_inner(state: &EditorState, entity_id: u32) -> Option<ScriptTagData> {
    let entity = find_entity_by_id(state, entity_id)?;
    let tag = state.world.get::<forge2d::script::ScriptTag>(entity)?;
    Some(ScriptTagData { tag: tag.0.clone() })
}

#[tauri::command]
fn script_tag_get(entity_id: u32) -> Option<ScriptTagData> {
    let state = get_state();
    script_tag_get_inner(&state, entity_id)
}

#[tauri::command]
fn script_tag_set(entity_id: u32, tag: String) -> Result<(), String> {
    let mut state = get_state();
    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);
    if let Some(script_tag) = state.world.get_mut::<forge2d::script::ScriptTag>(entity) {
        script_tag.0 = tag;
        commit_scene_edit(&mut state, snapshot);
        return Ok(());
    }
    Err("Entity does not have ScriptTag".to_string())
}

#[tauri::command]
fn entity_rename(entity_id: u32, name: String) -> Result<(), String> {
    let mut state = get_state();
    let _ = find_entity_by_id(&state, entity_id)
        .ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);
    state.entity_names.insert(entity_id, name);
    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

#[tauri::command]
fn viewport_render(
    width: u32,
    height: u32,
    camera_x: f32,
    camera_y: f32,
    zoom: f32,
    rotation: f32,
) -> Result<ViewportFrame, String> {
    let mut state = get_state();
    let cache_key = ViewportCacheKey {
        width,
        height,
        camera_x_bits: camera_x.to_bits(),
        camera_y_bits: camera_y.to_bits(),
        zoom_bits: zoom.to_bits(),
        rotation_bits: rotation.to_bits(),
        world_revision: state.world_revision,
    };
    if state.viewport_cache_key == Some(cache_key) {
        if let Some(rgba) = &state.viewport_cache_rgba {
            return Ok(ViewportFrame {
                width,
                height,
                rgba: rgba.clone(),
            });
        }
    }

    if state.offscreen_renderer.is_none() {
        state.offscreen_renderer = Some(
            Renderer::new_offscreen(width, height).map_err(|e| e.to_string())?,
        );
    }

    let mut camera = Camera2D::new(Vec2::new(camera_x, camera_y));
    camera.zoom = zoom.max(0.01);
    camera.rotation = rotation;
    let mut sprites: Vec<(forge2d::Sprite, Option<String>)> = Vec::new();

    for (entity, _transform) in state.world.query::<forge2d::entities::Transform>() {
        let sprite_comp = match state.world.get::<forge2d::entities::SpriteComponent>(entity) {
            Some(sprite) => sprite,
            None => continue,
        };
        if !sprite_comp.visible {
            continue;
        }

        let entity_id = entity.to_u32();
        let world_pos = forge2d::get_world_position(&state.world, entity);
        let world_rot = forge2d::get_world_rotation(&state.world, entity);
        let world_scale = forge2d::get_world_scale(&state.world, entity);
        let mut sprite = sprite_comp.sprite.clone();
        sprite.transform.position = world_pos;
        sprite.transform.rotation = world_rot;
        sprite.transform.scale = Vec2::new(
            world_scale.x * sprite_comp.sprite.transform.scale.x,
            world_scale.y * sprite_comp.sprite.transform.scale.y,
        );
        sprites.push((sprite, state.entity_texture_paths.get(&entity_id).cloned()));
    }

    let mut renderer = state
        .offscreen_renderer
        .take()
        .ok_or_else(|| "Renderer not available".to_string())?;

    let fallback = match state.offscreen_fallback {
        Some(handle) => handle,
        None => {
            let handle = renderer
                .load_texture_from_rgba(&[255, 255, 255, 255], 1, 1)
                .map_err(|e| e.to_string())?;
            state.offscreen_fallback = Some(handle);
            handle
        }
    };

    for (_, texture_path) in &sprites {
        if let Some(path) = texture_path {
            if !state.offscreen_textures.contains_key(path) {
                if let Ok(handle) = renderer.load_texture_from_file(path) {
                    state.offscreen_textures.insert(path.clone(), handle);
                }
            }
        }
    }

    for (sprite, texture_path) in &mut sprites {
        sprite.texture = match texture_path {
            Some(path) => state
                .offscreen_textures
                .get(path)
                .copied()
                .unwrap_or(fallback),
            None => fallback,
        };
    }

    let rgba = renderer
        .render_offscreen_rgba(width, height, |renderer, frame| {
            renderer.clear(frame, [0.06, 0.07, 0.1, 1.0])?;
            for (sprite, _) in &sprites {
                renderer.draw_sprite(frame, sprite, &camera)?;
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;

    state.offscreen_renderer = Some(renderer);

    state.viewport_cache_key = Some(cache_key);
    state.viewport_cache_rgba = Some(rgba.clone());

    Ok(ViewportFrame {
        width,
        height,
        rgba,
    })
}

#[tauri::command]
fn viewport_render_raw(
    width: u32,
    height: u32,
    camera_x: f32,
    camera_y: f32,
    zoom: f32,
    rotation: f32,
) -> Result<tauri::ipc::Response, String> {
    let frame = viewport_render(width, height, camera_x, camera_y, zoom, rotation)?;
    Ok(tauri::ipc::Response::new(frame.rgba))
}

#[tauri::command]
fn transform_set(
    entity_id: u32,
    position: [f32; 2],
    rotation: f32,
    scale: [f32; 2],
) -> Result<(), String> {
    let mut state = get_state();
    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);

    if let Some(transform) = state.world.get_mut::<forge2d::entities::Transform>(entity) {
        transform.position = forge2d::Vec2::new(position[0], position[1]);
        transform.rotation = rotation;
        transform.scale = forge2d::Vec2::new(scale[0], scale[1]);
    } else {
        state.world.insert(
            entity,
            forge2d::entities::Transform::new(forge2d::Vec2::new(position[0], position[1]))
                .with_rotation(rotation)
                .with_scale(forge2d::Vec2::new(scale[0], scale[1])),
        );
    }

    // Update physics body if it exists (only in edit mode)
    if can_edit_scene(&state) {
        if state
            .world
            .get::<forge2d::entities::PhysicsBody>(entity)
            .is_some()
        {
            let transform_data = state
                .world
                .get::<forge2d::entities::Transform>(entity)
                .map(|transform| (transform.position, transform.rotation));
            if let Some((position, rotation)) = transform_data {
                state.physics.set_body_position(entity, position);
                state.physics.set_body_rotation(entity, rotation);
            }
        }
    }

    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

// Component metadata
#[derive(Serialize, Deserialize)]
struct ComponentFieldInfo {
    name: String,
    type_name: String,
    value: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct InspectorSnapshot {
    component_types: Vec<String>,
    attachable_types: Vec<String>,
    attached_components: Vec<String>,
    fields: HashMap<String, Vec<ComponentFieldInfo>>,
    sprite_data: Option<SpriteData>,
    camera_data: Option<CameraData>,
    script_data: Option<ScriptTagData>,
}

#[tauri::command]
fn component_fields(entity_id: u32, component_type: String) -> Option<Vec<ComponentFieldInfo>> {
    let state = get_state();
    let entity = find_entity_by_id(&state, entity_id)?;
    let handler = state.metadata_registry.get(&component_type)?;
    let fields = handler.fields();

    Some(
        fields
            .into_iter()
            .map(|field| {
                let value = handler
                    .get_field(&state.world, entity, &field.name)
                    .unwrap_or(serde_json::Value::Null);

                ComponentFieldInfo {
                    name: field.name,
                    type_name: field.type_name,
                    value,
                }
            })
            .collect(),
    )
}

#[tauri::command]
fn inspector_snapshot(entity_id: u32) -> Option<InspectorSnapshot> {
    let state = get_state();
    let entity = find_entity_by_id(&state, entity_id)?;
    let component_types = state.metadata_registry.type_names();
    let attachable_types = component_attachable_types();
    let attached_components = entity_components_inner(&state, entity_id);
    let mut fields = HashMap::new();

    for component_type in &component_types {
        if let Some(handler) = state.metadata_registry.get(component_type) {
            let component_fields = handler
                .fields()
                .into_iter()
                .map(|field| ComponentFieldInfo {
                    name: field.name.clone(),
                    type_name: field.type_name,
                    value: handler
                        .get_field(&state.world, entity, &field.name)
                        .unwrap_or(serde_json::Value::Null),
                })
                .collect::<Vec<_>>();
            if !component_fields.is_empty() {
                fields.insert(component_type.clone(), component_fields);
            }
        }
    }

    let sprite_data = sprite_get_inner(&state, entity_id);
    let camera_data = camera_get_inner(&state, entity_id);
    let script_data = script_tag_get_inner(&state, entity_id);

    Some(InspectorSnapshot {
        component_types,
        attachable_types,
        attached_components,
        fields,
        sprite_data,
        camera_data,
        script_data,
    })
}

#[tauri::command]
fn component_set_field(
    entity_id: u32,
    component_type: String,
    field_name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let mut state = get_state();
    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);

    {
        let metadata_registry = std::mem::take(&mut state.metadata_registry);
        let handler = metadata_registry
            .get(&component_type)
            .ok_or_else(|| "Component type not found".to_string())?;

        let result = handler
            .set_field(&mut state.world, entity, &field_name, value)
            .map_err(|e| e.to_string());
        state.metadata_registry = metadata_registry;
        result?;
    }
    if component_type == "PhysicsBody" && can_edit_scene(&state) {
        ensure_physics_body(&mut state, entity)?;
    }
    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

#[tauri::command]
fn component_types() -> Vec<String> {
    let state = get_state();
    state.metadata_registry.type_names()
}

#[tauri::command]
fn component_attachable_types() -> Vec<String> {
    vec![
        "Transform".to_string(),
        "SpriteComponent".to_string(),
        "PhysicsBody".to_string(),
        "CameraComponent".to_string(),
        "ScriptTag".to_string(),
    ]
}

fn entity_components_inner(state: &EditorState, entity_id: u32) -> Vec<String> {
    let entity = match find_entity_by_id(state, entity_id) {
        Some(e) => e,
        None => return Vec::new(),
    };
    let mut components = Vec::new();
    if state.world.get::<forge2d::entities::Transform>(entity).is_some() {
        components.push("Transform".to_string());
    }
    if state.world.get::<forge2d::entities::SpriteComponent>(entity).is_some() {
        components.push("SpriteComponent".to_string());
    }
    if state.world.get::<forge2d::entities::PhysicsBody>(entity).is_some() {
        components.push("PhysicsBody".to_string());
    }
    if state.world.get::<forge2d::entities::CameraComponent>(entity).is_some() {
        components.push("CameraComponent".to_string());
    }
    if state.world.get::<forge2d::script::ScriptTag>(entity).is_some() {
        components.push("ScriptTag".to_string());
    }
    components
}

#[tauri::command]
fn entity_components(entity_id: u32) -> Vec<String> {
    let state = get_state();
    entity_components_inner(&state, entity_id)
}

#[tauri::command]
fn component_add(entity_id: u32, component_type: String) -> Result<(), String> {
    let mut state = get_state();
    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);

    match component_type.as_str() {
        "Transform" => {
            if state
                .world
                .get::<forge2d::entities::Transform>(entity)
                .is_none()
            {
                state
                    .world
                    .insert(entity, forge2d::entities::Transform::new(forge2d::Vec2::ZERO));
            }
        }
        "SpriteComponent" => {
            if state
                .world
                .get::<forge2d::entities::SpriteComponent>(entity)
                .is_none()
            {
                let sprite = forge2d::entities::SpriteComponent::new(
                    forge2d::render::TextureHandle::new(0),
                );
                state.world.insert(entity, sprite);
            }
        }
        "PhysicsBody" => {
            if state
                .world
                .get::<forge2d::entities::PhysicsBody>(entity)
                .is_none()
            {
                let body = forge2d::entities::PhysicsBody::new(
                    forge2d::physics::RigidBodyType::Dynamic,
                );
                state.world.insert(entity, body);
                if can_edit_scene(&state) {
                    ensure_physics_body(&mut state, entity)?;
                }
            }
        }
        "CameraComponent" => {
            if state
                .world
                .get::<forge2d::entities::CameraComponent>(entity)
                .is_none()
            {
                let cam =
                    forge2d::entities::CameraComponent::new(forge2d::Vec2::ZERO);
                state.world.insert(entity, cam);
            }
        }
        "ScriptTag" => {
            if state
                .world
                .get::<forge2d::script::ScriptTag>(entity)
                .is_none()
            {
                state
                    .world
                    .insert(entity, forge2d::script::ScriptTag(String::new()));
            }
        }
        _ => return Err("Unknown component type".to_string()),
    }

    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

#[tauri::command]
fn component_remove(entity_id: u32, component_type: String) -> Result<(), String> {
    let mut state = get_state();
    let entity =
        find_entity_by_id(&state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let snapshot = capture_editor_snapshot(&state);

    match component_type.as_str() {
        "Transform" => {
            state.world.remove::<forge2d::entities::Transform>(entity);
        }
        "SpriteComponent" => {
            state
                .world
                .remove::<forge2d::entities::SpriteComponent>(entity);
            state.entity_texture_paths.remove(&entity_id);
        }
        "PhysicsBody" => {
            state
                .world
                .remove::<forge2d::entities::PhysicsBody>(entity);
            state.physics.remove_body(entity);
        }
        "CameraComponent" => {
            state
                .world
                .remove::<forge2d::entities::CameraComponent>(entity);
        }
        "ScriptTag" => {
            state
                .world
                .remove::<forge2d::script::ScriptTag>(entity);
        }
        _ => return Err("Unknown component type".to_string()),
    }

    commit_scene_edit(&mut state, snapshot);
    Ok(())
}

// Project operations
#[derive(Serialize, Deserialize)]
struct ProjectInfo {
    name: String,
    path: String,
    version: String,
}

#[tauri::command]
fn project_create(name: String) -> Result<(), String> {
    // Get Documents folder path
    let documents_path =
        dirs::document_dir().ok_or_else(|| "Could not find Documents folder".to_string())?;

    // Create Forge2D projects folder
    let projects_folder = documents_path.join("Forge2D");
    fs::create_dir_all(&projects_folder)
        .map_err(|e| format!("Failed to create Forge2D projects folder: {}", e))?;

    // Create project folder: Documents/Forge2D/{name}
    let project_path = projects_folder.join(&name);

    // Check if project folder already exists
    if project_path.exists() {
        return Err(format!("Project '{}' already exists", name));
    }

    // Create project directory
    fs::create_dir_all(&project_path)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    // Create subdirectories
    fs::create_dir_all(project_path.join("scenes"))
        .map_err(|e| format!("Failed to create scenes directory: {}", e))?;
    fs::create_dir_all(project_path.join("assets"))
        .map_err(|e| format!("Failed to create assets directory: {}", e))?;
    fs::create_dir_all(project_path.join("assets").join("textures"))
        .map_err(|e| format!("Failed to create textures directory: {}", e))?;

    // Create project config
    let config = ProjectConfig {
        name: name.clone(),
        version: "1.0.0".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let config_path = project_path.join("forge2d_project.json");
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize project config: {}", e))?;
    fs::write(&config_path, config_json)
        .map_err(|e| format!("Failed to write project config: {}", e))?;

    // Load the project
    project_open(project_path.to_string_lossy().to_string())
}

#[tauri::command]
fn project_open(path: String) -> Result<(), String> {
    let mut state = get_state();
    let project_path = PathBuf::from(&path);

    // Verify project directory exists
    if !project_path.exists() {
        return Err("Project directory does not exist".to_string());
    }

    // Load project config
    let config_path = project_path.join("forge2d_project.json");
    let config: ProjectConfig = if config_path.exists() {
        let config_json = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read project config: {}", e))?;
        serde_json::from_str(&config_json)
            .map_err(|e| format!("Failed to parse project config: {}", e))?
    } else {
        // Create default config for old projects
        ProjectConfig {
            name: project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Untitled Project")
                .to_string(),
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    };

    state.project_path = Some(project_path);
    state.project_config = Some(config);
    reset_scene(&mut state);

    Ok(())
}

#[tauri::command]
fn project_get_current() -> Option<ProjectInfo> {
    let state = get_state();
    state.project_path.as_ref().and_then(|path| {
        state.project_config.as_ref().map(|config| ProjectInfo {
            name: config.name.clone(),
            path: path.to_string_lossy().to_string(),
            version: config.version.clone(),
        })
    })
}

#[tauri::command]
fn project_close(force: bool) -> Result<(), String> {
    let mut state = get_state();

    // Check if scene is dirty
    if state.scene_dirty && !force {
        return Err("Scene has unsaved changes. Save before closing project.".to_string());
    }

    state.project_path = None;
    state.project_config = None;
    reset_scene(&mut state);

    Ok(())
}

#[tauri::command]
fn project_files_tree() -> Result<ProjectFileTree, String> {
    let state = get_state();
    let project_path = state
        .project_path
        .as_ref()
        .ok_or_else(|| "No project open".to_string())?;

    let scenes_path = project_path.join("scenes");
    let assets_path = project_path.join("assets");

    if !scenes_path.exists() {
        fs::create_dir_all(&scenes_path)
            .map_err(|e| format!("Failed to create scenes folder: {}", e))?;
    }

    if !assets_path.exists() {
        fs::create_dir_all(&assets_path)
            .map_err(|e| format!("Failed to create assets folder: {}", e))?;
    }

    let scenes = build_file_tree(&scenes_path, 6)?;
    let assets = build_file_tree(&assets_path, 6)?;

    Ok(ProjectFileTree { scenes, assets })
}

fn resolve_prefab_destination(
    project_path: &Path,
    requested_path: &Path,
) -> Result<PathBuf, String> {
    let prefabs_dir = project_path.join("assets").join("prefabs");
    fs::create_dir_all(&prefabs_dir)
        .map_err(|e| format!("Failed to create prefabs directory: {}", e))?;

    let file_name = requested_path
        .file_name()
        .ok_or_else(|| "Invalid prefab file name".to_string())?
        .to_string_lossy()
        .to_string();

    let mut destination = prefabs_dir.join(file_name);
    if destination.extension().is_none() {
        destination.set_extension("prefab.json");
    }
    Ok(destination)
}

#[tauri::command]
fn prefab_save(entity_id: u32, path: String) -> Result<String, String> {
    let state = get_state();
    let project_path = state
        .project_path
        .as_ref()
        .ok_or_else(|| "No project open".to_string())?;

    let root = find_entity_by_id(&state, entity_id)
        .ok_or_else(|| "Entity not found".to_string())?;

    let ids = collect_subtree_ids(&state, root);
    let entities = serialize_entities_subset(&state, &ids);
    let prefab = PrefabAsset {
        version: 1,
        roots: vec![entity_id],
        entities,
    };

    let requested = PathBuf::from(path);
    let destination = resolve_prefab_destination(project_path, &requested)?;
    let json = serde_json::to_string_pretty(&prefab).map_err(|e| e.to_string())?;
    fs::write(&destination, json).map_err(|e| e.to_string())?;

    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
fn prefab_instantiate(path: String) -> Result<Vec<u32>, String> {
    let mut state = get_state();
    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let prefab: PrefabAsset = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let snapshot = capture_editor_snapshot(&state);
    let roots = instantiate_prefab(&mut state, prefab, None)?;
    commit_scene_edit(&mut state, snapshot);
    Ok(roots)
}

fn instantiate_prefab(
    state: &mut EditorState,
    prefab: PrefabAsset,
    position: Option<forge2d::Vec2>,
) -> Result<Vec<u32>, String> {
    let mut id_map: HashMap<u32, forge2d::EntityId> = HashMap::new();
    for entity in &prefab.entities {
        let new_entity = state.world.spawn();
        id_map.insert(entity.id.to_u32(), new_entity);
    }

    for entity in &prefab.entities {
        let new_entity = *id_map
            .get(&entity.id.to_u32())
            .ok_or_else(|| "Prefab entity mapping missing".to_string())?;

        for component in &entity.components {
            match component.type_name.as_str() {
                "Transform" => {
                    let mut data: TransformSerde =
                        serde_json::from_value(component.data.clone())
                            .map_err(|e| e.to_string())?;
                    data.parent = data
                        .parent
                        .and_then(|parent_id| id_map.get(&parent_id).map(|p| p.to_u32()));
                    let mut transform = forge2d::entities::Transform::new(data.position);
                    transform.rotation = data.rotation;
                    transform.scale = data.scale;
                    transform.parent = data.parent.map(forge2d::EntityId);
                    state.world.insert(new_entity, transform);
                }
                "SpriteComponent" => {
                    let data: SpriteSerde = serde_json::from_value(component.data.clone())
                        .map_err(|e| e.to_string())?;
                    let mut sprite = forge2d::entities::SpriteComponent::new(
                        forge2d::render::TextureHandle::new(0),
                    );
                    sprite.sprite.tint = data.tint;
                    sprite.sprite.transform.scale = data.sprite_scale;
                    sprite.visible = data.visible;
                    state.world.insert(new_entity, sprite);
                    if let Some(path) = data.texture_path {
                        state
                            .entity_texture_paths
                            .insert(new_entity.to_u32(), path);
                    }
                }
                "CameraComponent" => {
                    let data: CameraSerde = serde_json::from_value(component.data.clone())
                        .map_err(|e| e.to_string())?;
                    let mut cam =
                        forge2d::entities::CameraComponent::new(forge2d::Vec2::ZERO);
                    cam.camera.zoom = data.zoom;
                    cam.camera.offset = data.offset;
                    cam.camera.rotation = data.rotation;
                    cam.active = data.active;
                    state.world.insert(new_entity, cam);
                }
                "PhysicsBody" => {
                    let data: PhysicsBodySerde = serde_json::from_value(component.data.clone())
                        .map_err(|e| e.to_string())?;
                    let mut body = forge2d::entities::PhysicsBody::new(data.body_type);
                    body.collider_shape = data.collider_shape;
                    state.world.insert(new_entity, body);
                }
                "ScriptTag" => {
                    let tag = component
                        .data
                        .get("tag")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    state.world.insert(new_entity, forge2d::script::ScriptTag(tag));
                }
                "EntityName" => {
                    let name = component
                        .data
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if !name.is_empty() {
                        state.entity_names.insert(new_entity.to_u32(), name);
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(position) = position {
        let roots = prefab
            .roots
            .iter()
            .filter_map(|root_id| id_map.get(root_id).copied())
            .collect::<Vec<_>>();
        if let Some(first_root) = roots.first().copied() {
            let origin = forge2d::get_world_position(&state.world, first_root);
            let delta = position - origin;
            for root in roots {
                if let Some(transform) = state.world.get_mut::<forge2d::entities::Transform>(root) {
                    transform.position += delta;
                }
            }
        }
    }

    sync_physics_bodies(state)?;

    let roots = prefab
        .roots
        .iter()
        .filter_map(|root_id| id_map.get(root_id).map(|id| id.to_u32()))
        .collect::<Vec<u32>>();

    Ok(roots)
}

#[tauri::command]
fn prefab_instantiate_at(path: String, position: [f32; 2]) -> Result<Vec<u32>, String> {
    let mut state = get_state();
    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let prefab: PrefabAsset = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let snapshot = capture_editor_snapshot(&state);
    let roots = instantiate_prefab(
        &mut state,
        prefab,
        Some(forge2d::Vec2::new(position[0], position[1])),
    )?;
    commit_scene_edit(&mut state, snapshot);
    Ok(roots)
}

#[tauri::command]
fn asset_import_texture(path: String) -> Result<String, String> {
    let state = get_state();
    let project_path = state
        .project_path
        .as_ref()
        .ok_or_else(|| "No project open".to_string())?;

    let source = PathBuf::from(&path);
    if !source.exists() {
        return Err("Source file does not exist".to_string());
    }

    let textures_dir = project_path.join("assets").join("textures");
    fs::create_dir_all(&textures_dir)
        .map_err(|e| format!("Failed to create textures directory: {}", e))?;

    if source.starts_with(&textures_dir) {
        return Ok(source.to_string_lossy().to_string());
    }

    let file_name = source
        .file_name()
        .ok_or_else(|| "Invalid source file name".to_string())?
        .to_string_lossy()
        .to_string();

    let file_stem = source
        .file_stem()
        .ok_or_else(|| "Invalid source file name".to_string())?
        .to_string_lossy()
        .to_string();

    let extension = source.extension().map(|ext| ext.to_string_lossy().to_string());

    let mut destination = textures_dir.join(&file_name);
    if destination.exists() {
        let mut counter = 1;
        loop {
            let candidate_name = if let Some(ext) = &extension {
                format!("{}_{}.{}", file_stem, counter, ext)
            } else {
                format!("{}_{}", file_stem, counter)
            };
            let candidate = textures_dir.join(candidate_name);
            if !candidate.exists() {
                destination = candidate;
                break;
            }
            counter += 1;
        }
    }

    fs::copy(&source, &destination)
        .map_err(|e| format!("Failed to copy texture: {}", e))?;

    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
fn project_list() -> Result<Vec<ProjectInfo>, String> {
    // Get Documents folder path
    let documents_path =
        dirs::document_dir().ok_or_else(|| "Could not find Documents folder".to_string())?;

    let projects_folder = documents_path.join("Forge2D");

    // Return empty list if folder doesn't exist
    if !projects_folder.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    // Read all directories in Forge2D folder
    let entries = fs::read_dir(&projects_folder)
        .map_err(|e| format!("Failed to read projects folder: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // Check if it's a directory and has a project config file
        if path.is_dir() {
            let config_path = path.join("forge2d_project.json");
            if config_path.exists() {
                // Try to load project config
                if let Ok(config_json) = fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<ProjectConfig>(&config_json) {
                        projects.push(ProjectInfo {
                            name: config.name,
                            path: path.to_string_lossy().to_string(),
                            version: config.version,
                        });
                    }
                }
            }
        }
    }

    // Sort by name
    projects.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(projects)
}

// Scene operations
#[tauri::command]
fn scene_save(path: Option<String>) -> Result<String, String> {
    let mut state = get_state();
    let mut scene = create_scene(&state.physics);
    scene.entities = serialize_entities(&state);

    let json = serde_json::to_string_pretty(&scene).map_err(|e| e.to_string())?;

    // Determine save path
    let save_path = if let Some(p) = path {
        PathBuf::from(p)
    } else if let Some(project_path) = &state.project_path {
        // Default to scenes/scene.json in project
        project_path.join("scenes").join("scene.json")
    } else {
        return Err("No project open and no path provided".to_string());
    };

    // Ensure directory exists
    if let Some(parent) = save_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    fs::write(&save_path, json).map_err(|e| e.to_string())?;

    state.scene_dirty = false;
    Ok(save_path.to_string_lossy().to_string())
}

#[tauri::command]
fn scene_load(path: String) -> Result<(), String> {
    let mut state = get_state();

    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let scene: forge2d::Scene = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    // Clear world and physics
    state.world = World::new();
    state.physics = PhysicsWorld::new();
    restore_scene_physics(&mut state.physics, &scene).map_err(|e| e.to_string())?;

    clear_scene_history(&mut state);
    state.scene_dirty = false;

    restore_entities(&mut state, &scene)?;
    sync_physics_bodies(&mut state)?;
    bump_world_revision(&mut state);

    Ok(())
}

fn reset_scene(state: &mut EditorState) {
    state.world = World::new();
    state.physics = PhysicsWorld::new();
    clear_scene_history(state);
    state.scene_dirty = false;
    state.entity_texture_paths.clear();
    state.entity_names.clear();
    bump_world_revision(state);
}

#[tauri::command]
fn scene_new() -> Result<(), String> {
    let mut state = get_state();
    reset_scene(&mut state);
    Ok(())
}

#[tauri::command]
fn scene_is_dirty() -> bool {
    let state = get_state();
    state.scene_dirty
}

#[tauri::command]
fn play_start() -> Result<(), String> {
    let mut state = get_state();
    if state.play_state.is_running() {
        return Err("Already in play mode".to_string());
    }

    // Snapshot current scene (physics + entities)
    let mut scene = create_scene(&state.physics);
    scene.entities = serialize_entities(&state);
    state.play_snapshot = Some(scene);
    state.play_snapshot_dirty = Some(state.scene_dirty);

    // Enable physics simulation
    state.play_state = PlayState::Playing;

    Ok(())
}

#[tauri::command]
fn play_pause() -> Result<(), String> {
    let mut state = get_state();
    if state.play_state != PlayState::Playing {
        return Err("Play mode is not running".to_string());
    }
    state.play_state = PlayState::Paused;
    Ok(())
}

#[tauri::command]
fn play_resume() -> Result<(), String> {
    let mut state = get_state();
    if state.play_state != PlayState::Paused {
        return Err("Play mode is not paused".to_string());
    }
    state.play_state = PlayState::Playing;
    Ok(())
}

fn advance_play_state(state: &mut EditorState, dt: f32) -> Result<(), String> {
    if !state.play_state.is_running() {
        return Err("Not in play mode".to_string());
    }

    state.physics.step(dt);

    let entity_ids: Vec<_> = state
        .world
        .query::<forge2d::entities::Transform>()
        .iter()
        .map(|(eid, _)| *eid)
        .collect();

    for entity_id in entity_ids {
        if let Some(transform) = state
            .world
            .get_mut::<forge2d::entities::Transform>(entity_id)
        {
            if let Some(pos) = state.physics.body_position(entity_id) {
                transform.position = pos;
            }
            if let Some(rot) = state.physics.body_rotation(entity_id) {
                transform.rotation = rot;
            }
        }
    }

    bump_world_revision(state);
    Ok(())
}

#[tauri::command]
fn play_stop() -> Result<(), String> {
    let mut state = get_state();
    if !state.play_state.is_running() {
        return Err("Not in play mode".to_string());
    }

    // Restore snapshot
    if let Some(snapshot) = state.play_snapshot.take() {
        // Clear world and physics
        state.world = World::new();
        state.physics = PhysicsWorld::new();

        // Restore physics first
        restore_scene_physics(&mut state.physics, &snapshot)
            .map_err(|e| format!("Failed to restore scene physics: {}", e))?;

        restore_entities(&mut state, &snapshot)?;
        sync_physics_bodies(&mut state)?;
        bump_world_revision(&mut state);
    }

    state.play_state = PlayState::Stopped;
    if let Some(was_dirty) = state.play_snapshot_dirty.take() {
        state.scene_dirty = was_dirty;
    }
    Ok(())
}

#[tauri::command]
fn play_state_get() -> PlayState {
    let state = get_state();
    state.play_state
}

#[tauri::command]
fn play_step_physics(dt: f32) -> Result<(), String> {
    let mut state = get_state();
    if !state.play_state.is_advancing() {
        return Ok(());
    }
    advance_play_state(&mut state, dt)
}

#[tauri::command]
fn play_step_frame(dt: Option<f32>) -> Result<(), String> {
    let mut state = get_state();
    if state.play_state != PlayState::Paused {
        return Err("Play mode must be paused to step".to_string());
    }
    advance_play_state(&mut state, dt.unwrap_or(1.0 / 60.0))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            project_create,
            project_open,
            project_get_current,
            project_close,
            project_files_tree,
            asset_import_texture,
            prefab_save,
            prefab_instantiate,
            prefab_instantiate_at,
            project_list,
            entities_list,
            world_revision_get,
            entity_create,
            entity_create_preset,
            entity_delete,
            entity_duplicate,
            entity_rename,
            undo,
            redo,
            can_undo,
            can_redo,
            selection_get,
            selection_set,
            selection_add,
            selection_clear,
            entity_reparent,
            transform_get,
            transform_get_hierarchy,
            transforms_list,
            transform_set,
            camera_get,
            camera_set,
            camera_entities,
            active_camera_info,
            sprite_get,
            sprite_set_texture_path,
            script_tag_get,
            script_tag_set,
            viewport_render,
            viewport_render_raw,
            inspector_snapshot,
            component_fields,
            component_set_field,
            component_types,
            component_attachable_types,
            entity_components,
            component_add,
            component_remove,
            scene_save,
            scene_load,
            scene_new,
            play_start,
            play_pause,
            play_resume,
            play_stop,
            play_state_get,
            play_step_physics,
            play_step_frame,
            scene_is_dirty,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
