// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use forge2d::{
    create_scene, register_builtin_metadata, restore_scene_physics, Camera2D, Command,
    CommandHistory, ComponentMetadataRegistry, PhysicsWorld, Renderer, Vec2, World,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};

// Project configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ProjectConfig {
    name: String,
    version: String,
    created_at: String,
    // Future: engine version, settings, etc.
}

// Editor state
struct EditorState {
    world: World,
    physics: PhysicsWorld,
    command_history: CommandHistory,
    metadata_registry: ComponentMetadataRegistry,
    scene_dirty: bool,
    is_playing: bool,
    play_snapshot: Option<forge2d::Scene>, // Snapshot taken before play mode
    play_snapshot_dirty: Option<bool>,
    // Texture registry: maps entity ID -> texture file path (for sprites)
    entity_texture_paths: std::collections::HashMap<u32, String>,
    // Project management
    project_path: Option<PathBuf>,
    project_config: Option<ProjectConfig>,
    offscreen_renderer: Option<Renderer>,
    offscreen_textures: HashMap<String, forge2d::TextureHandle>,
    offscreen_fallback: Option<forge2d::TextureHandle>,
}

impl EditorState {
    fn new() -> Self {
        let mut registry = ComponentMetadataRegistry::new();
        register_builtin_metadata(&mut registry);

        Self {
            world: World::new(),
            physics: PhysicsWorld::new(),
            command_history: CommandHistory::default(),
            metadata_registry: registry,
            scene_dirty: false,
            is_playing: false,
            play_snapshot: None,
            play_snapshot_dirty: None,
            entity_texture_paths: std::collections::HashMap::new(),
            project_path: None,
            project_config: None,
            offscreen_renderer: None,
            offscreen_textures: HashMap::new(),
            offscreen_fallback: None,
        }
    }
}

// Global state (in a real app, you'd use proper state management)
static mut EDITOR_STATE: Option<EditorState> = None;

fn get_state() -> &'static mut EditorState {
    unsafe {
        if EDITOR_STATE.is_none() {
            EDITOR_STATE = Some(EditorState::new());
        }
        EDITOR_STATE.as_mut().unwrap()
    }
}

// Helper to find entity by ID (since EntityId constructor is private)
fn find_entity_by_id(state: &EditorState, entity_id: u32) -> Option<forge2d::EntityId> {
    // Query all entities with Transform (most common case)
    for (eid, _) in state.world.query::<forge2d::entities::Transform>() {
        if eid.to_u32() == entity_id {
            return Some(eid);
        }
    }
    // TODO: Also check entities without Transform
    // For now, we only support entities with Transform
    None
}

// IPC Commands

#[derive(Serialize, Deserialize)]
struct EntityInfo {
    id: u32,
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

        entities.push(forge2d::scene::SerializableEntity {
            id: entity,
            components,
        });
    }

    entities
}

fn restore_entities(state: &mut EditorState, scene: &forge2d::Scene) -> Result<(), String> {
    state.entity_texture_paths = HashMap::new();
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

    for (entity_id, transform) in state.world.query::<forge2d::entities::Transform>() {
        let id = entity_id.to_u32();
        let has_transform = true;
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
        let parent_id = transform.parent.map(|e| e.to_u32());
        let children = forge2d::hierarchy::get_children(&state.world, entity_id)
            .iter()
            .map(|e| e.to_u32())
            .collect();

        entities.push(EntityInfo {
            id,
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
fn entity_delete(entity_id: u32) -> Result<(), String> {
    let state = get_state();
    if state.is_playing {
        return Err("Cannot delete entities in play mode".to_string());
    }

    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    let cmd = forge2d::DeleteEntity::new(entity);
    state
        .command_history
        .execute(Box::new(cmd), &mut state.world)
        .map_err(|e| e.to_string())?;

    state.scene_dirty = true;
    Ok(())
}

#[tauri::command]
fn entity_duplicate(entity_id: u32) -> Result<u32, String> {
    let state = get_state();
    let source_entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    // Create new entity
    let mut cmd = Box::new(forge2d::CreateEntity::new());
    cmd.execute(&mut state.world)
        .map_err(|e| format!("Failed to create entity: {}", e))?;
    let new_entity_id = cmd
        .entity()
        .ok_or_else(|| "Entity ID not available after creation".to_string())?;

    // Copy Transform component if it exists
    if let Some(transform) = state
        .world
        .get::<forge2d::entities::Transform>(source_entity)
    {
        let mut new_transform = transform.clone();
        // Offset position slightly so it's visible
        new_transform.position.x += 50.0;
        new_transform.position.y += 50.0;
        state.world.insert(new_entity_id, new_transform);
    }

    // Copy SpriteComponent if it exists
    if let Some(sprite) = state
        .world
        .get::<forge2d::entities::SpriteComponent>(source_entity)
    {
        state.world.insert(new_entity_id, sprite.clone());
    }

    // Copy PhysicsBody if it exists
    if let Some(physics) = state
        .world
        .get::<forge2d::entities::PhysicsBody>(source_entity)
    {
        state.world.insert(new_entity_id, *physics);
    }

    // Copy CameraComponent if it exists
    if let Some(cam) = state
        .world
        .get::<forge2d::entities::CameraComponent>(source_entity)
    {
        state.world.insert(new_entity_id, cam.clone());
    }

    // Add command to history
    state
        .command_history
        .execute(cmd, &mut state.world)
        .map_err(|e| format!("Failed to add command to history: {}", e))?;

    state.scene_dirty = true;
    Ok(new_entity_id.to_u32())
}

#[tauri::command]
fn entity_create() -> Result<u32, String> {
    let state = get_state();
    if state.is_playing {
        return Err("Cannot create entities in play mode".to_string());
    }
    let state = get_state();

    // Create entity via command
    let mut cmd = Box::new(forge2d::CreateEntity::new());

    // Execute the command first to get the entity ID
    cmd.execute(&mut state.world)
        .map_err(|e| format!("Failed to create entity: {}", e))?;

    let entity_id = cmd
        .entity()
        .ok_or_else(|| "Entity ID not available after creation".to_string())?;

    // Add a Transform component so the entity shows up in the list
    // This should also be done via command, but for now we'll do it directly
    state.world.insert(
        entity_id,
        forge2d::entities::Transform::new(forge2d::Vec2::ZERO),
    );

    // Now add the command to history (it's already executed, so this won't execute again)
    // Actually, the history will execute it again, but CreateEntity is idempotent
    state
        .command_history
        .execute(cmd, &mut state.world)
        .map_err(|e| format!("Failed to add command to history: {}", e))?;

    state.scene_dirty = true;
    Ok(entity_id.to_u32())
}

#[tauri::command]
fn entity_create_preset(preset: String, position: Option<[f32; 2]>) -> Result<u32, String> {
    let state = get_state();
    if state.is_playing {
        return Err("Cannot create entities in play mode".to_string());
    }

    let mut cmd = Box::new(forge2d::CreateEntity::new());
    cmd.execute(&mut state.world)
        .map_err(|e| format!("Failed to create entity: {}", e))?;

    let entity_id = cmd
        .entity()
        .ok_or_else(|| "Entity ID not available after creation".to_string())?;

    let pos = position.unwrap_or([0.0, 0.0]);
    state.world.insert(
        entity_id,
        forge2d::entities::Transform::new(forge2d::Vec2::new(pos[0], pos[1])),
    );

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

    state
        .command_history
        .execute(cmd, &mut state.world)
        .map_err(|e| format!("Failed to add command to history: {}", e))?;

    state.scene_dirty = true;
    Ok(entity_id.to_u32())
}

#[tauri::command]
fn undo() -> Result<(), String> {
    let state = get_state();
    state
        .command_history
        .undo(&mut state.world)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn redo() -> Result<(), String> {
    let state = get_state();
    state
        .command_history
        .redo(&mut state.world)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn can_undo() -> bool {
    let state = get_state();
    state.command_history.can_undo()
}

#[tauri::command]
fn can_redo() -> bool {
    let state = get_state();
    state.command_history.can_redo()
}

// Selection management
static mut SELECTED_ENTITIES: Vec<u32> = Vec::new();

#[tauri::command]
fn selection_get() -> Vec<u32> {
    unsafe { SELECTED_ENTITIES.clone() }
}

#[tauri::command]
fn selection_set(ids: Vec<u32>) {
    unsafe {
        SELECTED_ENTITIES = ids;
    }
}

#[tauri::command]
fn selection_add(id: u32) {
    unsafe {
        if !SELECTED_ENTITIES.contains(&id) {
            SELECTED_ENTITIES.push(id);
        }
    }
}

#[tauri::command]
fn selection_clear() {
    unsafe {
        SELECTED_ENTITIES.clear();
    }
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
    if let Some(entity) = find_entity_by_id(state, entity_id) {
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

#[tauri::command]
fn transform_get_hierarchy(entity_id: u32) -> Option<TransformHierarchyData> {
    let state = get_state();
    let entity = find_entity_by_id(state, entity_id)?;
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
fn camera_get(entity_id: u32) -> Option<CameraData> {
    let state = get_state();
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
fn camera_set(
    entity_id: u32,
    active: bool,
    zoom: f32,
    offset: [f32; 2],
    rotation: f32,
) -> Result<(), String> {
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;
    let cam = state
        .world
        .get_mut::<forge2d::entities::CameraComponent>(entity)
        .ok_or_else(|| "CameraComponent not found".to_string())?;
    cam.active = active;
    cam.camera.zoom = zoom.max(0.01);
    cam.camera.offset = forge2d::Vec2::new(offset[0], offset[1]);
    cam.camera.rotation = rotation;
    state.scene_dirty = true;
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

#[derive(Serialize, Deserialize)]
struct SpriteData {
    texture_handle: u32,
    texture_path: Option<String>,   // Path to texture file
    texture_size: Option<[u32; 2]>, // Width, height
    tint: [f32; 4],
    sprite_scale: [f32; 2], // Scale from sprite.transform
}

#[tauri::command]
fn sprite_get(entity_id: u32) -> Option<SpriteData> {
    let state = get_state();
    let entity = find_entity_by_id(state, entity_id)?;
    let sprite_comp = state
        .world
        .get::<forge2d::entities::SpriteComponent>(entity)?;

    // Get texture path for this entity
    let texture_path = state.entity_texture_paths.get(&entity_id).cloned();

    Some(SpriteData {
        texture_handle: 0, // Not used in editor
        texture_path,
        texture_size: None, // Will be determined from loaded image
        tint: sprite_comp.sprite.tint,
        sprite_scale: [
            sprite_comp.sprite.transform.scale.x,
            sprite_comp.sprite.transform.scale.y,
        ],
    })
}

// Set texture path for an entity (called when sprite is created/updated)
#[tauri::command]
fn sprite_set_texture_path(entity_id: u32, path: String) -> Result<(), String> {
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    // Verify entity has SpriteComponent
    if state
        .world
        .get::<forge2d::entities::SpriteComponent>(entity)
        .is_none()
    {
        return Err("Entity does not have SpriteComponent".to_string());
    }

    if path.trim().is_empty() {
        state.entity_texture_paths.remove(&entity_id);
    } else {
        state.entity_texture_paths.insert(entity_id, path);
    }
    state.scene_dirty = true;
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
    let state = get_state();
    if state.offscreen_renderer.is_none() {
        state.offscreen_renderer = Some(
            Renderer::new_offscreen(width, height).map_err(|e| e.to_string())?,
        );
    }

    let renderer = state
        .offscreen_renderer
        .as_mut()
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

    let mut camera = Camera2D::new(Vec2::new(camera_x, camera_y));
    camera.zoom = zoom.max(0.01);
    camera.rotation = rotation;

    let (world, textures, texture_paths) = (
        &state.world,
        &mut state.offscreen_textures,
        &state.entity_texture_paths,
    );
    let mut sprites: Vec<forge2d::Sprite> = Vec::new();

    for (entity, _transform) in world.query::<forge2d::entities::Transform>() {
        let sprite_comp = match world.get::<forge2d::entities::SpriteComponent>(entity) {
            Some(sprite) => sprite,
            None => continue,
        };
        if !sprite_comp.visible {
            continue;
        }

        let entity_id = entity.to_u32();
        let world_pos = forge2d::get_world_position(world, entity);
        let world_rot = forge2d::get_world_rotation(world, entity);
        let world_scale = forge2d::get_world_scale(world, entity);
        let texture_handle = if let Some(path) = texture_paths.get(&entity_id) {
            if let Some(handle) = textures.get(path) {
                *handle
            } else {
                match renderer.load_texture_from_file(path) {
                    Ok(handle) => {
                        textures.insert(path.clone(), handle);
                        handle
                    }
                    Err(_) => fallback,
                }
            }
        } else {
            fallback
        };

        let mut sprite = sprite_comp.sprite.clone();
        sprite.texture = texture_handle;
        sprite.transform.position = world_pos;
        sprite.transform.rotation = world_rot;
        sprite.transform.scale = Vec2::new(
            world_scale.x * sprite_comp.sprite.transform.scale.x,
            world_scale.y * sprite_comp.sprite.transform.scale.y,
        );
        sprites.push(sprite);
    }

    let rgba = renderer
        .render_offscreen_rgba(width, height, |renderer, frame| {
            renderer.clear(frame, [0.06, 0.07, 0.1, 1.0])?;
            for sprite in &sprites {
                renderer.draw_sprite(frame, sprite, &camera)?;
            }
            Ok(())
        })
        .map_err(|e| e.to_string())?;

    Ok(ViewportFrame {
        width,
        height,
        rgba,
    })
}

#[tauri::command]
fn transform_set(
    entity_id: u32,
    position: [f32; 2],
    rotation: f32,
    scale: [f32; 2],
) -> Result<(), String> {
    println!(
        "Received transform_set: entity_id={}, position=[{}, {}], rotation={}, scale=[{}, {}]",
        entity_id, position[0], position[1], rotation, scale[0], scale[1]
    );
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    let cmd = forge2d::SetTransform::new(
        entity,
        forge2d::Vec2::new(position[0], position[1]),
        rotation,
        forge2d::Vec2::new(scale[0], scale[1]),
    );

    state
        .command_history
        .execute(Box::new(cmd), &mut state.world)
        .map_err(|e| e.to_string())?;

    // Update physics body if it exists (only in edit mode)
    if !state.is_playing {
        if state
            .world
            .get::<forge2d::entities::PhysicsBody>(entity)
            .is_some()
        {
            // Get the updated transform
            if let Some(transform) = state.world.get::<forge2d::entities::Transform>(entity) {
                state.physics.set_body_position(entity, transform.position);
                state.physics.set_body_rotation(entity, transform.rotation);
            }
        }
    }

    state.scene_dirty = true;
    Ok(())
}

// Component metadata
#[derive(Serialize, Deserialize)]
struct ComponentFieldInfo {
    name: String,
    type_name: String,
    value: serde_json::Value,
}

#[tauri::command]
fn component_fields(entity_id: u32, component_type: String) -> Option<Vec<ComponentFieldInfo>> {
    let state = get_state();
    let entity = find_entity_by_id(state, entity_id)?;
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
fn component_set_field(
    entity_id: u32,
    component_type: String,
    field_name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    let handler = state
        .metadata_registry
        .get(&component_type)
        .ok_or_else(|| "Component type not found".to_string())?;

    handler
        .set_field(&mut state.world, entity, &field_name, value)
        .map_err(|e| e.to_string())?;
    if component_type == "PhysicsBody" && !state.is_playing {
        ensure_physics_body(state, entity)?;
    }
    state.scene_dirty = true;
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

#[tauri::command]
fn entity_components(entity_id: u32) -> Vec<String> {
    let state = get_state();
    let entity = match find_entity_by_id(state, entity_id) {
        Some(entity) => entity,
        None => return Vec::new(),
    };
    let mut components = Vec::new();
    if state.world.get::<forge2d::entities::Transform>(entity).is_some() {
        components.push("Transform".to_string());
    }
    if state
        .world
        .get::<forge2d::entities::SpriteComponent>(entity)
        .is_some()
    {
        components.push("SpriteComponent".to_string());
    }
    if state
        .world
        .get::<forge2d::entities::PhysicsBody>(entity)
        .is_some()
    {
        components.push("PhysicsBody".to_string());
    }
    if state
        .world
        .get::<forge2d::entities::CameraComponent>(entity)
        .is_some()
    {
        components.push("CameraComponent".to_string());
    }
    if state
        .world
        .get::<forge2d::script::ScriptTag>(entity)
        .is_some()
    {
        components.push("ScriptTag".to_string());
    }
    components
}

#[tauri::command]
fn component_add(entity_id: u32, component_type: String) -> Result<(), String> {
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

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
                if !state.is_playing {
                    ensure_physics_body(state, entity)?;
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

    state.scene_dirty = true;
    Ok(())
}

#[tauri::command]
fn component_remove(entity_id: u32, component_type: String) -> Result<(), String> {
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

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

    state.scene_dirty = true;
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
    let state = get_state();
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

    // Reset scene
    scene_new()?;

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
    let state = get_state();

    // Check if scene is dirty
    if state.scene_dirty && !force {
        return Err("Scene has unsaved changes. Save before closing project.".to_string());
    }

    state.project_path = None;
    state.project_config = None;
    scene_new()?;

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

    let root = find_entity_by_id(state, entity_id)
        .ok_or_else(|| "Entity not found".to_string())?;

    let ids = collect_subtree_ids(state, root);
    let entities = serialize_entities_subset(state, &ids);
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
    let state = get_state();
    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let prefab: PrefabAsset = serde_json::from_str(&json).map_err(|e| e.to_string())?;

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
                _ => {}
            }
        }
    }

    sync_physics_bodies(state)?;
    state.scene_dirty = true;

    let roots = prefab
        .roots
        .iter()
        .filter_map(|root_id| id_map.get(root_id).map(|id| id.to_u32()))
        .collect::<Vec<u32>>();

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
    let state = get_state();
    let mut scene = create_scene(&state.physics);
    scene.entities = serialize_entities(state);

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
    let state = get_state();

    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let scene: forge2d::Scene = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    // Clear world and physics
    state.world = World::new();
    state.physics = PhysicsWorld::new();
    restore_scene_physics(&mut state.physics, &scene).map_err(|e| e.to_string())?;

    // Clear command history
    state.command_history.clear();
    state.scene_dirty = false;

    restore_entities(state, &scene)?;
    sync_physics_bodies(state)?;

    Ok(())
}

#[tauri::command]
fn scene_new() -> Result<(), String> {
    let state = get_state();
    state.world = World::new();
    state.physics = PhysicsWorld::new();
    state.command_history.clear();
    state.scene_dirty = false;
    state.entity_texture_paths.clear();
    Ok(())
}

#[tauri::command]
fn scene_is_dirty() -> bool {
    let state = get_state();
    state.scene_dirty
}

#[tauri::command]
fn play_start() -> Result<(), String> {
    let state = get_state();
    if state.is_playing {
        return Err("Already in play mode".to_string());
    }

    // Snapshot current scene (physics + entities)
    let mut scene = create_scene(&state.physics);
    scene.entities = serialize_entities(state);
    state.play_snapshot = Some(scene);
    state.play_snapshot_dirty = Some(state.scene_dirty);

    // Enable physics simulation
    state.is_playing = true;

    Ok(())
}

#[tauri::command]
fn play_stop() -> Result<(), String> {
    let state = get_state();
    if !state.is_playing {
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

        restore_entities(state, &snapshot)?;
        sync_physics_bodies(state)?;

        // Clear command history after restore
        state.command_history.clear();
    }

    state.is_playing = false;
    if let Some(was_dirty) = state.play_snapshot_dirty.take() {
        state.scene_dirty = was_dirty;
    }
    Ok(())
}

#[tauri::command]
fn play_is_playing() -> bool {
    let state = get_state();
    state.is_playing
}

#[tauri::command]
fn play_step_physics(dt: f32) -> Result<(), String> {
    let state = get_state();
    if !state.is_playing {
        return Err("Not in play mode".to_string());
    }

    // Step physics simulation
    state.physics.step(dt);

    // Sync physics positions back to Transform components
    // Collect entity IDs first to avoid borrow checker issues
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

    Ok(())
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
            project_list,
            entities_list,
            entity_create,
            entity_create_preset,
            entity_delete,
            entity_duplicate,
            undo,
            redo,
            can_undo,
            can_redo,
            selection_get,
            selection_set,
            selection_add,
            selection_clear,
            transform_get,
            transform_get_hierarchy,
            transform_set,
            camera_get,
            camera_set,
            camera_entities,
            sprite_get,
            sprite_set_texture_path,
            viewport_render,
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
            play_stop,
            play_is_playing,
            play_step_physics,
            scene_is_dirty,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
