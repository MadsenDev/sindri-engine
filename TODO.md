# Sindri Editor TODO

This document tracks the architectural decisions and implementation tasks needed to make Sindri "editor-ready" without requiring major rewrites later.

## Architecture Decisions

### ✅ Completed
- [x] Scene serialization with versioning
- [x] Component serialization system
- [x] Stable coordinate system

### 🔒 Lock In Now (Before Editor UI)

#### 1. Engine Architecture: Headless Core + Runtime
- [ ] **Split engine into conceptual layers** (even if single crate initially)
  - [ ] `sindri_core`: world, components, scene graph, serialization, asset IDs, commands/undo, reflection
  - [ ] `sindri_runtime`: window, input, game loop integration
  - [ ] `sindri_editor`: UI app + viewport + inspectors + asset browser
  - [ ] Document the separation clearly in code structure

#### 2. Editor UI Tech: Tauri ✅
- [x] **Decision made**: Using Tauri (Rust backend + React frontend)
- [x] Set up Tauri project structure
- [x] Configure Vite + React + Tailwind
- [x] Set up IPC communication layer
- [x] Basic UI layout (entity list, toolbar, inspector panels)
- [x] IPC commands for entity operations

#### 3. Command System (Undo/Redo) ✅
- [x] **Design command trait/interface**
- [x] **Implement core commands:**
  - [x] `CreateEntity` - Create new entity
  - [x] `DeleteEntity` - Delete entity (with undo)
  - [x] `SetTransform` - Modify transform component
  - [x] `AddComponent<T>` - Add component to entity
  - [x] `RemoveComponent<T>` - Remove component from entity
  - [ ] `SetComponentField` - Modify individual component fields (use metadata system)
  - [x] `ReparentEntity` - Change entity hierarchy (via hierarchy module)
- [x] **Command bus/history:**
  - [x] Command history stack
  - [x] Undo/redo functionality
  - [ ] Command batching (for multi-select edits)
- [ ] **Integration:**
  - [ ] Hook commands into World operations (optional - can be done at editor level)
  - [ ] Make scene operations use commands (optional)

#### 4. Component Reflection/Metadata ✅
- [x] **Design component metadata system:**
  - [x] `ComponentMetadataHandler` trait
  - [x] `FieldDescriptor` struct
  - [x] `ComponentMetadataRegistry` for registration
- [x] **Manual metadata registration:**
  - [x] Metadata for `Transform` (position, rotation, scale)
  - [ ] Metadata for `SpriteComponent` (texture, tint, visible)
  - [ ] Metadata for `PhysicsBody` (body_type, collider_shape)
  - [ ] Metadata for other built-in entity components
- [x] **Field editing support:**
  - [x] Get field value by name
  - [x] Set field value by name
  - [x] Basic validation

#### 5. Edit Mode vs Play Mode
- [ ] **Design mode system:**
  - [ ] `EditMode` - Transforms authoritative, physics preview-only
  - [ ] `PlayMode` - Runtime systems run, physics owns transforms
- [ ] **Scene snapshotting:**
  - [ ] Snapshot scene before play
  - [ ] Restore scene after play stops
  - [ ] Use existing scene serialization
- [ ] **Mode transitions:**
  - [ ] Enter play mode (snapshot, start systems)
  - [ ] Exit play mode (stop systems, restore snapshot)
  - [ ] Handle physics state during transitions

#### 6. Entity Hierarchy ✅
- [x] **Add parent/child relationships:**
  - [x] `parent: Option<EntityId>` field in Transform component
- [x] **Hierarchy operations:**
  - [x] `set_parent(entity, parent)`
  - [x] `get_parent(entity) -> Option<EntityId>`
  - [x] `get_children(entity) -> Vec<EntityId>`
  - [x] `get_root(entity) -> EntityId`
  - [x] `reparent(entity, new_parent)`
- [x] **Transform inheritance:**
  - [x] `get_world_position(entity)` - Computes world position accounting for parents
  - [x] `get_world_rotation(entity)` - Computes world rotation accounting for parents
  - [x] `get_world_scale(entity)` - Computes world scale accounting for parents
- [ ] **Serialization:**
  - [ ] Save/load hierarchy in scene format (parent field is already in Transform)
  - [ ] Handle orphaned entities (validation)

## Milestone 1: "Viewport + Select + Move"

The first editor milestone - a thin editor that proves the pipeline.

### Viewport Rendering ✅ (Basic Canvas Implementation)
- [x] **Basic viewport:**
  - [x] Create viewport canvas
  - [x] Render entities as boxes (canvas-based, not full engine renderer yet)
  - [x] Handle viewport resize
  - [x] Camera controls (pan with Alt+drag, zoom with mouse wheel)
- [x] **Viewport interaction:**
  - [x] Mouse position in world coordinates
  - [x] Viewport coordinate conversion
  - [x] Click to select entity (CPU hit-test)
- [ ] **Full engine renderer integration:**
  - [ ] Embed Sindri renderer in viewport (requires native window or WebGL)
  - [ ] Render sprites, textures, etc.
  - [ ] ID buffer picking for accurate selection

### Entity Selection ✅
- [x] **Selection system:**
  - [x] Single entity selection
  - [x] Multi-select (Ctrl+click)
  - [ ] Selection highlight rendering (needs viewport)
  - [x] Selection persistence
- [ ] **Picking implementation:**
  - [ ] Option A: ID buffer picking (render entity IDs to offscreen buffer)
  - [ ] Option B: CPU hit-test (raycast against entity bounds)
  - [ ] Click to select entity (in viewport)
  - [ ] Handle overlapping entities

### Transform Gizmo ✅
- [x] **Gizmo rendering:**
  - [x] Draw translate gizmo (arrows for X/Y)
  - [x] Draw rotate gizmo (circle)
  - [x] Draw scale gizmo (box)
  - [x] Gizmo follows selected entity
- [x] **Gizmo interaction:**
  - [x] Mouse hover detection (visual feedback)
  - [x] Drag to translate/rotate/scale
  - [ ] Snap to grid (optional - can add later)
  - [ ] Constrain to axis (Shift+drag - can add later)

### Inspector Panel ✅
- [x] **React inspector UI:**
  - [x] Entity list panel (shows all entities)
  - [x] Inspector panel (shows selected entity components)
  - [x] Component field editors (text inputs, sliders, dropdowns)
- [x] **IPC integration:**
  - [x] Query entity components from Rust
  - [x] Send component field updates to Rust
  - [x] Use component metadata for UI generation
- [x] **Field editing:**
  - [x] Edit Transform fields (position, rotation, scale)
  - [ ] Edit SpriteComponent fields (texture, tint, visible) - Needs metadata
  - [ ] Add/remove components
  - [ ] Real-time updates (changes reflect in viewport)

### Scene Save/Load ✅ (Basic)
- [x] **Scene operations:**
  - [x] Save scene to JSON (use existing serialization)
  - [x] Load scene from JSON
  - [x] New scene (clear world)
  - [x] Scene dirty flag (unsaved changes)
- [ ] **File operations:**
  - [ ] Tauri file dialog integration
  - [ ] Save/load dialogs (currently uses fixed path)
  - [ ] Recent files list
- [ ] **Entity serialization:**
  - [ ] Save/load entities and components (currently only physics)

## Milestone 2: "Full Editor Core"

### Asset Browser
- [ ] **Asset management UI:**
  - [ ] Asset list panel
  - [ ] Texture preview
  - [ ] Asset import (drag & drop or file dialog)
  - [ ] Asset metadata display
- [ ] **Asset operations:**
  - [ ] Import texture from file
  - [ ] Import font from file
  - [ ] Asset ID assignment
  - [ ] Asset deletion

### Entity Operations
- [ ] **Entity creation:**
  - [ ] Create entity button/menu
  - [ ] Entity templates (empty, sprite, physics body, etc.)
  - [ ] Duplicate entity
- [ ] **Entity deletion:**
  - [ ] Delete selected entity
  - [ ] Delete with undo
  - [ ] Handle component cleanup
- [ ] **Entity hierarchy UI:**
  - [ ] Hierarchy tree view
  - [ ] Drag & drop reparenting
  - [ ] Expand/collapse nodes

### Undo/Redo UI
- [ ] **Undo/redo controls:**
  - [ ] Undo button (Ctrl+Z)
  - [ ] Redo button (Ctrl+Y)
  - [ ] Undo/redo menu items
  - [ ] History depth limit
- [ ] **Visual feedback:**
  - [ ] Disable buttons when no history
  - [ ] Show history depth

### Play Mode
- [ ] **Play mode controls:**
  - [ ] Play button (starts play mode)
  - [ ] Stop button (exits play mode)
  - [ ] Pause button (pauses systems)
- [ ] **Mode transitions:**
  - [ ] Snapshot scene before play
  - [ ] Start physics/runtime systems
  - [ ] Stop systems on exit
  - [ ] Restore scene snapshot
- [ ] **Visual feedback:**
  - [ ] Different viewport appearance in play mode
  - [ ] Disable editing in play mode

## Milestone 3: "Advanced Features"

### Prefab System
- [ ] **Prefab concept:**
  - [ ] Prefab definition (template entity with components)
  - [ ] Instantiate prefab (create entity from template)
  - [ ] Prefab overrides (modify instantiated entity)
- [ ] **Prefab UI:**
  - [ ] Prefab browser
  - [ ] Create prefab from entity
  - [ ] Instantiate prefab in scene
  - [ ] Prefab editing

### Timeline/Animation (If Needed)
- [ ] **Timeline UI:**
  - [ ] Timeline panel
  - [ ] Keyframe editing
  - [ ] Playback controls
- [ ] **Animation system:**
  - [ ] Keyframe storage
  - [ ] Interpolation
  - [ ] Animation playback

### Debug Tools
- [ ] **Debug visualization:**
  - [ ] Toggle collider wireframes
  - [ ] Toggle physics contacts
  - [ ] Toggle pathfinding grid
  - [ ] Toggle entity bounds
- [ ] **Debug panels:**
  - [ ] Physics stats
  - [ ] Render stats
  - [ ] Entity count
  - [ ] Performance metrics

## Implementation Order (Recommended)

### Phase 1: Foundation (Before UI)
1. Command system (undo/redo backbone)
2. Component metadata/reflection
3. Entity hierarchy (if needed)
4. Edit/Play mode separation

### Phase 2: Tauri Setup
1. Set up Tauri project
2. Configure React + Tailwind
3. Set up IPC layer
4. Basic window layout (panels)

### Phase 3: Milestone 1
1. Viewport rendering
2. Entity selection
3. Transform gizmo
4. Inspector panel
5. Scene save/load

### Phase 4: Milestone 2
1. Asset browser
2. Entity operations
3. Undo/redo UI
4. Play mode

### Phase 5: Milestone 3 (As Needed)
1. Prefab system
2. Timeline/animation
3. Debug tools

## IPC API Design

### Editor → Engine (Commands)
```rust
// Commands
cmd.apply(Command) -> Result<()>
cmd.undo() -> Result<()>
cmd.redo() -> Result<()>

// Scene
scene.load(path: String) -> Result<()>
scene.save(path: String) -> Result<()>
scene.new() -> Result<()>
scene.is_dirty() -> bool

// Assets
assets.import_texture(path: String) -> Result<AssetId>
assets.import_font(path: String) -> Result<AssetId>
assets.list() -> Vec<AssetInfo>

// Entities
entities.create() -> EntityId
entities.delete(id: EntityId) -> Result<()>
entities.list() -> Vec<EntityInfo>
entities.get(id: EntityId) -> Option<EntityInfo>
entities.set_component_field(id: EntityId, component: String, field: String, value: JsonValue) -> Result<()>

// Selection
selection.set(ids: Vec<EntityId>)
selection.get() -> Vec<EntityId>

// Play mode
play.start() -> Result<()>
play.stop() -> Result<()>
play.is_playing() -> bool
```

### Engine → Editor (Events)
```rust
// Selection
selection_changed(ids: Vec<EntityId>)

// Scene
scene_changed() // Dirty flag
scene_loaded(path: String)
scene_saved(path: String)

// Assets
asset_imported(id: AssetId, info: AssetInfo)
asset_deleted(id: AssetId)

// Errors
error(message: String)
log(level: String, message: String)

// Metrics
metrics(fps: f32, physics_time: f32, render_time: f32)
```

## Notes

- **Engine runs inside editor process** (Tauri allows this easily)
- **Single-threaded initially** (Rust + React both on main thread is fine)
- **Scene format is JSON** (already have this)
- **Component metadata is manual** (macro-based or trait-based, not full reflection)
- **Commands are the source of truth** (all edits go through commands)

## Questions to Resolve

- [ ] Should entity hierarchy be required or optional?
- [ ] What's the maximum undo history depth?
- [ ] Should commands be serializable (for save/load)?
- [ ] How to handle component type discovery at runtime?
- [ ] Should viewport support multiple cameras?
