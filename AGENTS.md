# Sindri Engine — Agent Instructions

This document is the authoritative reference for all work on the Sindri engine and editor. Read it before writing code. When in doubt, re-read it.

---

## The single most important rule

**Do not rewrite, refactor, or "improve" existing code unless a task explicitly says to.** The engine and editor are working. Add to them — don't fix what isn't broken.

---

## What this repo is

Sindri is a production Rust 2D game engine with:
- A complete entity/component system, physics, Lua scripting, rendering, audio, tilemaps, particles, lighting, pathfinding, animation, and scene serialization
- A local Axum HTTP server (`sindri-server`) that the editor talks to
- A Tauri 2 + React/TypeScript editor (`editor/`) that is fully built and in active use
- Local Ollama-based AI tooling embedded in the editor

---

## Workspace layout

```
sindri/
├── Cargo.toml
├── AGENTS.md
├── crates/
│   ├── sindri/              engine core (stable — don't restructure)
│   └── sindri-server/       engine HTTP server + Lua runtime
└── editor/                  Tauri 2 + React editor
    ├── src/                 frontend (React/TypeScript)
    └── src-tauri/           Tauri backend (Rust)
```

---

## Editor stack

- Tauri 2, React 18 + TypeScript, Vite
- Monaco Editor for scripts
- No UI component libraries. No Tailwind. Hand-built CSS with design tokens.
- Commands via `invoke()` from `@tauri-apps/api/core`

### Design tokens (actual values in use)

```css
--paper: #0d1117
--paper-2: #161b22
--paper-3: #1e2530
--ink: #e6e1d4
--ink-2: #c4beae
--ink-3: #8a8580
--ink-4: #5a554e
--rule: #1f242c
--rule-2: #2a3038
--accent: #f0c050    /* amber — AI affordances only */
--cyan: #6dbcdb      /* entity/camera accent */
--moss: #9bb070      /* success/ready state */
--font-ui: "Pixelify Sans", monospace
--font-mono: "JetBrains Mono", monospace
```

**Amber is AI-only.** Never use it for non-AI buttons or accents.

### Layout

Three-column fixed layout. Left panel: scene/files/input/history tabs. Center: viewport + script editor. Right: inspector or proposals lane.

---

## Editor scene model

The editor works exclusively with `sindri::scene::Scene` and the JSON component enum. The runtime `World` is separate.

### Component enum (editor/server layer)

```rust
pub enum Component {
    Transform(Transform),
    Sprite(Sprite),
    AnimatedSprite(AnimatedSprite),
    PhysicsBody(PhysicsBody),
    Collider(Collider),
    Script(Script),
    Camera(Camera),
    AudioSource(AudioSource),
    Tilemap(Tilemap),
    NavGrid(NavGrid),
}
```

### Transform

```rust
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub scale_x: f32,   // default 1.0
    pub scale_y: f32,   // default 1.0
    pub rotation: f32,  // radians
    pub z_index: i32,   // draw order
    pub pivot_x: f32,   // 0..1, default 0.5 (center)
    pub pivot_y: f32,   // 0..1, default 0.5 (center)
}
```

### Tilemap

Multi-palette. Tiles encoded as `u32`: upper 16 bits = palette_id (1-indexed), lower 16 bits = tile_idx within palette.

```rust
pub struct Tilemap {
    pub palettes: Vec<TilePalette>,
    pub layers: Vec<TilemapLayer>,
    pub map_cols: u32,
    pub map_rows: u32,
    pub tile_width: f32,
    pub tile_height: f32,
    // ... tint, etc.
}
pub struct TilePalette {
    pub name: String,
    pub texture_path: String,
    pub tileset_cols: u32,
    pub tileset_rows: u32,
    pub margin: u32,
    pub spacing: u32,
    pub solid_tiles: Vec<u16>,              // backward-compat boolean flag → Full collider
    pub tile_colliders: HashMap<u16, TileColliderShape>, // per-tile shape, overrides solid_tiles
    pub disabled_tiles: Vec<u16>,           // hidden from painter (still rendered if already placed)
}

// TileColliderShape variants (serde: tag = "type", rename_all = "snake_case"):
// None, Full, Rect { x, y, w, h } (0..1 tile space),
// SlopeCutTl ("/"-floor), SlopeCutTr ("\"-floor), SlopeCutBl, SlopeCutBr (ceiling variants)
```

### NavGrid

```rust
pub struct NavGrid {
    pub cols: u32,
    pub rows: u32,
    pub cell_size: f32,
    pub walkable: Vec<bool>,
    pub mode: NavGridMode,   // TopDown | Platformer
    // ... platformer params
}
```

---

## HTTP bridge (sindri-server, port 7878)

All editor ↔ engine communication goes through this API.

| Method | Path | Notes |
|--------|------|-------|
| GET | `/health` | `{"status":"ok"}` |
| GET | `/scene` | Full scene JSON |
| PUT | `/scene` | Replace scene |
| POST | `/scene/open` | Load `.sindri` file |
| POST | `/scene/save` | Save current scene |
| GET | `/scene/entity/:id` | Single entity |
| PATCH | `/scene/entity/:id/transform` | Patch transform fields |
| POST | `/scene/entity` | Create entity → `{id}` |
| DELETE | `/scene/entity/:id` | Remove entity |
| PATCH | `/scene/entity/:id/name` | Rename entity |
| POST | `/scene/entity/:id/component` | Add component |
| DELETE | `/scene/entity/:id/component` | Remove component (by type) |
| PATCH | `/scene/entity/:id/component/:idx` | Patch component by index |
| DELETE | `/scene/entity/:id/component/:idx` | Remove component by index |
| PATCH | `/scene/entity/:id/staged` | Mark entity as staged |
| POST | `/scene/staged/commit` | Commit staged changes |
| POST | `/scene/staged/revert` | Revert staged changes |
| GET | `/screenshot` | `{"image":"<base64 PNG>"}` |
| GET | `/stream` | WebSocket MJPEG stream |
| POST | `/control` | `{"action":"play"\|"pause"\|"stop"}` |
| POST | `/gizmos` | `{"enabled":bool}` |
| GET | `/debug/paths` | Per-entity pathfinding debug paths |
| POST | `/input/keys` | Send keyboard state from browser |
| GET | `/scripts` | List `.lua` paths |
| GET | `/script?path=…` | Script content |
| PUT | `/script?path=…` | Write script |
| GET | `/errors` | Runtime error log |

Port: `127.0.0.1:7878`, overridable via `SINDRI_PORT`.

---

## Lua scripting (sindri-server runtime)

> **This is NOT the engine's ScriptRuntime.** The server has its own Lua runtime in `crates/sindri-server/src/lua_runtime.rs`. Scripts run in entity-sandboxed environments.

### Lifecycle

```lua
function on_start(self)       end  -- first frame
function on_update(self, dt)  end  -- every frame; dt = seconds
```

### Globals

```lua
key_down(key)        -- bool, key held (raw key name, e.g. "ArrowLeft", "a")
key_pressed(key)     -- bool, pressed this frame only
vec2(x, y)           -- {x, y} table
print(...)           -- forwarded to engine console
load_scene(path)     -- swap scene next frame (e.g. "scenes/level2.sindri")
storage              -- persistent key/value table (see storage section below)
entity_transform(name) -- {x, y, rotation} of named entity, or nil
```

### `input` global — action-based (InputMap)

```lua
input.pressed("action_name")       -- bool, any binding held
input.just_pressed("action_name")  -- bool, first frame only
input.just_released("action_name") -- bool, frame it was released
input.axis("action_name")          -- -1..1 from KeyAxis or GamepadAxis bindings
```

Actions are defined in `input_map.json` at the project root. The editor's "Input" tab manages this file.

### `self` methods

| Method | Returns | Notes |
|--------|---------|-------|
| `self:entity()` | integer | Entity ID |
| `self:transform()` | table or nil | Position/rotation/scale |
| `self:sprite()` | table or nil | Tint/visibility |
| `self:animated_sprite()` | table or nil | Clip control |
| `self:physics()` | table or nil | Velocity/impulse/contacts |
| `self:camera()` | table or nil | Camera control |
| `self:tilemap()` | table or nil | Tile queries/mutations |
| `self:nav_grid()` | table or nil | Pathfinding |
| `self:input()` | table | Per-entity legacy input facet |
| `self:world()` | table | Scene queries/spawn/despawn |
| `self:find_path(target)` | array or nil | Shortcut to nav_grid pathfinding |

### `self:transform()`

```lua
local t = self:transform()
t:position()              -- {x, y}
t:set_position(vec2(x,y))
t:rotation()              -- radians
t:set_rotation(r)
t:set_scale(vec2(sx,sy))
```

### `self:physics()`

```lua
local p = self:physics()
p:velocity()              -- {x, y}
p:set_velocity(vec2(vx,vy))
p:apply_impulse(vec2(ix,iy))
p:contacts()              -- table of entity name strings
```

### `self:nav_grid()`

```lua
local nav = self:nav_grid()
nav:find_path({x,y}, {x,y})      -- returns [{x,y},...] or nil
nav:is_walkable(tx, ty)           -- bool
nav:set_walkable(tx, ty, bool)    -- queued
nav:world_to_tile({x,y})          -- {x, y} tile coord
nav:tile_to_world(tx, ty)         -- {x, y} world coord
nav:build_from_tilemap(entity_id, {blocked_ids...})
nav:fill_all(bool)
```

### `self:world()`

```lua
local w = self:world()
w:find_entity("name")             -- entity id or nil
w:entity_transform("name")        -- {x, y, rotation} or nil
w:entity_count()                  -- number of entities in scene
w:spawn(name, x, y)               -- queued: blank entity + Transform at (x,y)
w:spawn_prefab(path, x, y)        -- queued: load prefab file at (x,y) e.g. "prefabs/enemy.prefab"
w:despawn(entity_id)              -- queued: remove entity by id
```

Spawn/despawn are applied at end of the current frame. On the next frame the entity
exists and can be found via `w:find_entity()`. Spawned entities are blank (Transform
only) unless spawned from a prefab.

### `load_scene(path)` — scene transitions

```lua
load_scene("scenes/level2.sindri")   -- swap to another scene next frame
```

Saves nothing automatically — write any state to `storage` before switching.
All scripts restart when the new scene loads.

### `storage` — persistent save data

Key/value store written to `save_data.json` in the project root. Survives
play/stop and game restarts. Values can be string, number, or boolean.

```lua
storage.set("coins", 42)
storage.set("player_name", "Ada")
storage.set("unlocked", true)

local coins = storage.get("coins")   -- 42, or nil if not set
storage.delete("coins")
storage.save()    -- explicit flush (set/delete auto-flush, but this forces it)
```

### InputMap config format (`input_map.json`)

```json
{
  "actions": [
    { "name": "move_right", "bindings": [
        { "type": "key", "key": "d" },
        { "type": "gamepad_axis", "axis": "LeftStickX", "deadzone": 0.2 }
    ]},
    { "name": "jump", "bindings": [
        { "type": "key", "key": " " },
        { "type": "gamepad_button", "button": "South" }
    ]},
    { "name": "move", "bindings": [
        { "type": "key_axis", "negative": "a", "positive": "d" }
    ]}
  ]
}
```

Binding types: `key`, `key_axis`, `gamepad_button`, `gamepad_axis`.

---

## Tauri commands (editor/src-tauri/src/)

### Engine lifecycle & HTTP proxy

```
start_engine / stop_engine / get_engine_binary_path
get_scene / put_scene / save_scene / open_scene_file / patch_transform
get_engine_status / get_screenshot / get_runtime_errors
set_engine_paused / set_engine_playback / set_gizmos
get_script / write_script / list_scripts
create_entity / rename_entity / add_component / remove_component / patch_component
```

### Project management

```
create_project / list_project_files / list_project_tree
new_script / new_scene_file / create_folder
move_project_entry / delete_project_file / rename_project_file
read_text_file / read_project_file
get_project_settings / save_project_settings
get_editor_prefs / save_editor_prefs
write_anim_file
create_tile_palette / read_tile_palette
read_input_map / write_input_map
```

### Prefabs

```
list_prefabs / save_as_prefab / instantiate_prefab
update_prefab / sync_from_prefab / unlink_from_prefab
```

### AI

```
send_ai_message / send_ai_message_stream
get_ai_provider_status / save_ai_api_key / clear_ai_api_key / test_ai_provider
generate_proposal / apply_action
commit_staged_change / revert_staged_change / clear_staged_proposal / get_pending_proposal
```

### Suggestions (Ollama)

```
list_ollama_models / ensure_suggestion_model / generate_entity_suggestions
```

---

## AI action schema

```json
{ "type": "edit_transform", "entity_id": 0, "x": 100, "y": 200 }
{ "type": "create_entity", "name": "Coin", "parent_id": null }
{ "type": "delete_entity", "entity_id": 3 }
{ "type": "write_script", "path": "scripts/player.lua", "content": "..." }
{ "type": "add_component", "entity_id": 0, "component": { "type": "Sprite", ... } }
{ "type": "remove_component", "entity_id": 0, "component_type": "Sprite" }
{ "type": "patch_component", "entity_id": 0, "component_idx": 1, "patch": { ... } }
{ "type": "suggest_fix", "description": "...", "entity_id": 0 }
```

---

## File extension conventions

| Extension | Kind |
|-----------|------|
| `.sindri` | Scene file (JSON) |
| `.lua` | Script |
| `.tilepal` | Tile palette sidecar (JSON next to a PNG) |
| `.anim` | Animation definition |
| `input_map.json` | Project input map |
| `project.f2proj` | Project metadata |
| `project_settings.json` | Project settings |

---

## Non-negotiables

1. **Local-first AI.** External API calls only when the user explicitly enables a cloud provider. Ollama localhost is the default.
2. **No UI libraries.** Hand-built CSS only.
3. **No Tailwind.** CSS tokens only.
4. **Amber = AI only.** Never use for non-AI affordances.
5. **Do not restructure engine files** in `crates/sindri/`.
6. **Scripting in Lua.** Never reference Rhai.
7. **Scene extension stays `.sindri`.**
8. **Do not add features beyond what the task asks for.** No speculative abstractions.
