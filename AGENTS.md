# Sindri Engine — Agent Instructions

This document is the authoritative specification for all work on the Sindri engine and editor. Read it entirely before writing a single line of code. When in doubt, read it again.

---

## The single most important rule

**Do not rewrite, refactor, or "improve" existing engine code unless a task explicitly tells you to.** The engine is mature and working. Your job is to add to it, not fix it.

---

## What this repo is

Sindri (formerly Forge2D) is a production-ready 2D game engine in Rust built on `wgpu` and `winit`. It has a complete entity/component system, physics, scripting, rendering, audio, tilemaps, particles, lighting, pathfinding, animation, scene serialization, and an undo/redo command system. None of these need to be built.

The work remaining is:
1. Engine quality-of-life improvements (tracked in `IMPROVEMENTS.md` — complete those tasks in order)
2. Two new Rust crates: `sindri-server` and `sindri-ai`
3. A brand new Tauri editor (delete the old `editor/` entirely first)

---

## What already exists — never rebuild these

### `crates/sindri/src/`

| File | What it contains |
|------|-----------------|
| `world.rs` | `World`, `EntityId`, typed component storage, `spawn`, `despawn`, `query`, `insert`, `get`, `get_mut`, `remove`, `restore_entity` |
| `entities.rs` | All built-in components: `Transform`, `SpriteComponent`, `PhysicsBody`, `AudioSource`, `CameraComponent`, `TilemapComponent`, `Player`, `Enemy`, `Collectible`, `Hazard`, `Checkpoint`, `Trigger`, `MovingPlatform` |
| `hierarchy.rs` | Parent/child entity relationships, world position/rotation/scale computation |
| `physics.rs` | Full Rapier2D integration: Dynamic/Kinematic/Fixed bodies, Box/Circle/Capsule colliders, sensors/triggers, CCD, raycasting, point queries, all force/impulse/velocity APIs, collision events via channels |
| `scene.rs` | Scene serialization/deserialization. `Scene`, `SerializablePhysics`, `ComponentSerializable` trait, save/load to `.sindri` files. Physics world has `extract_serializable` and `restore_from_serializable` |
| `script.rs` | Full Lua scripting via mlua. `ScriptRuntime`, `ScriptComponent`, hot reload, all lifecycle hooks, typed facets for all components, command buffer pattern |
| `commands.rs` | Full undo/redo: `Command` trait, `CommandHistory`, `CreateEntity`, `DeleteEntity`, `SetTransform`, `ReparentEntity`, `AddComponent`, `RemoveComponent` |
| `component_metadata.rs` | Runtime component reflection: `ComponentMetadataRegistry`, `ComponentMetadataHandler`, `FieldDescriptor`, `TransformMetadataHandler` |
| `state.rs` | `State` trait, `StateMachine`, push/pop/replace with deferred transitions |
| `camera.rs` | Camera follow with dead zones, smooth lerp, bounds clamping |
| `input.rs` | Frame-accurate input: `InputState`, `InputMap`, `ActionId`, `AxisBinding` |
| `audio.rs` | Sound effects and music via rodio. Preload, play, loop, volume, graceful fallback |
| `assets.rs` | Texture and font caching by path/key |
| `fonts.rs` | Three built-in fonts: Inter (UI), VT323 (Mono), Bangers (Title) via `include_bytes!` |
| `grid.rs` | Typed grid, coordinate conversion, 4/8-directional neighbors |
| `pathfinding.rs` | A* pathfinding, `PathfindingGrid`, `AStarPathfinder` |
| `hud.rs` | Screen-space UI: `HudLayer`, `HudText`, `HudSprite`, `HudRect`, `HudPanel`, `HudLayout` |
| `math.rs` | `Vec2`, `Transform2D`, `Camera2D` (zoom, rotation, shake, smooth zoom, bounds, screen↔world) |
| `engine.rs` | Main loop, `Engine`, `EngineContext`, `Game` trait, fixed timestep, vsync |

### `crates/sindri/src/render/`

| File | What it contains |
|------|-----------------|
| `wgpu_backend.rs` | Full wgpu pipeline. Batched sprites (2048/frame), shapes, tilemaps with viewport culling, text via glyphon, particles. Multi-pass: scene → occlusion → lightmap → composite. Has `render_offscreen_rgba` and `end_frame_readback` for pixel readback |
| `sprite.rs` | `Sprite`, `TextureHandle` |
| `animation.rs` | `Animation`, `AnimationFrame`, `AnimatedSprite`, spritesheet grid support |
| `tilemap.rs` | `Tilemap`, `Tile`, tileset UV mapping |
| `particles.rs` | `ParticleSystem`, `ParticleEmitter`, `EmissionConfig` |
| `light.rs` | `PointLight` (omni and spotlight), `DirectionalLight` |
| `light.wgsl` | Shadow-casting light shader with occlusion raymarching |
| `composite.wgsl` | Scene × lightmap compositing |
| `sprite.wgsl` | Sprite shader with UV transform and occlusion output |
| `shape.wgsl` | Polygon shader with occlusion output |
| `text.rs` | `TextRenderer` wrapping glyphon |

### Screenshot capture — already implemented
`wgpu_backend.rs` has `render_offscreen_rgba` and `end_frame_readback`. The screenshot server endpoint must call `render_offscreen_rgba`, encode the resulting RGBA bytes as PNG in memory using the `image` crate, then base64-encode the PNG. Do not write to disk.

---

## Important — do not change these

- **Raw pointers in `ScriptSelf`** (`world: *const World`, `physics: *const PhysicsWorld`) are intentional and sound. Do not replace with `Arc<Mutex<>>`.
- **`apply_impulse` uses velocity addition** — intentional, documented in a comment. Leave it.
- **`script.rs.rhai_backup`** — leave this file. Do not delete it.
- **Scripting language is Lua** — `.lua` files via mlua. Never reference Rhai in new code or text.
- **Scene file extension is `.sindri`** — do not change it.
- **Physics→transform sync is manual** — the engine does not automatically write physics positions back to `Transform`. This is addressed in `IMPROVEMENTS.md` Task 1.

---

## Workspace structure (target)

```
sindri/
├── Cargo.toml
├── AGENTS.md
├── IMPROVEMENTS.md
├── sindri-editor.html          ← visual reference mockup
├── sindri-logo.svg             ← engine logo
├── crates/
│   ├── sindri/                 ← engine core (do not restructure)
│   ├── sindri-server/          ← NEW
│   └── sindri-ai/              ← NEW
└── editor/                     ← NEW (delete old editor/ first)
```

Workspace `Cargo.toml` members: `["crates/sindri", "crates/sindri-server", "crates/sindri-ai"]`

---

## crates/sindri-server

Axum HTTP server embedded in the engine process. The editor communicates with the engine exclusively through this API.

### Cargo.toml

```toml
[dependencies]
axum = "0.7"
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sindri = { path = "../sindri" }
anyhow = { workspace = true }
base64 = "0.22"
image = "0.25"
tower-http = { version = "0.5", features = ["cors"] }
```

### Routes

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | `{"status":"ok"}` |
| `GET` | `/scene` | Full scene JSON |
| `PUT` | `/scene` | Replace scene |
| `POST` | `/scene/open` | Load a project-relative `.sindri` file and make it the active autosave target |
| `GET` | `/scene/entity/:id` | Single entity |
| `PATCH` | `/scene/entity/:id/transform` | Update transform fields |
| `POST` | `/scene/entity` | Create entity |
| `DELETE` | `/scene/entity/:id` | Remove entity |
| `GET` | `/screenshot` | `{"data":"<base64 PNG>"}` |
| `GET` | `/scripts` | List `.lua` paths in project |
| `GET` | `/script?path=…` | Script file contents |
| `PUT` | `/script?path=…` | Write script file |
| `GET` | `/components` | Registered component type names |
| `GET` | `/entity/:id/components` | All component field values |
| `PATCH` | `/entity/:id/component/:type/field/:field` | Set a component field |

Port: `127.0.0.1:7878`, configurable via `SINDRI_PORT` env var.

Shared state: `Arc<RwLock<Scene>>` and active `Arc<RwLock<PathBuf>>` scene path between engine loop and server.

The engine spawns the server in a background Tokio task after `init()`.

---

## crates/sindri-ai

### Cargo.toml

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
base64 = "0.22"
```

Ollama endpoint: `http://localhost:11434/api/chat`
Default model (vision+code): `qwen2.5-vl:7b`
Default model (code only): `qwen2.5-coder:7b`

### Context struct

```rust
pub struct AiContext {
    pub scene_json: String,
    pub open_script: Option<String>,
    pub open_script_path: Option<String>,
    pub screenshot_base64: Option<String>,
    pub error_log: Option<String>,
}
```

### System prompt (exact — do not paraphrase or shorten)

```
You are an AI assistant embedded in the Sindri game engine editor.
You have access to the current scene (as JSON), optionally an open Lua script,
and optionally a screenshot of the game viewport.

You may respond in two ways:
1. Plain text explanation or analysis.
2. A JSON action block wrapped in <action>...</action> tags.

Action block schema:
{
  "actions": [
    { "type": "edit_transform", "entity_id": 0, "x": 100.0, "y": 200.0 },
    { "type": "write_script", "path": "scripts/player.lua", "content": "..." },
    { "type": "create_entity", "name": "Coin", "parent_id": null },
    { "type": "delete_entity", "entity_id": 3 },
    { "type": "suggest_fix", "description": "Collider height doesn't match sprite", "entity_id": 0 }
  ]
}

Always respond with a brief plain-text explanation first, then the action block if applicable.
Only include an action block when making or suggesting a concrete change.
Keep explanations concise. Use the scene JSON and screenshot to ground your answers.
```

### Action types

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "edit_transform")]
    EditTransform { entity_id: u64, x: Option<f32>, y: Option<f32>, scale_x: Option<f32>, scale_y: Option<f32>, rotation: Option<f32> },
    #[serde(rename = "write_script")]
    WriteScript { path: String, content: String },
    #[serde(rename = "create_entity")]
    CreateEntity { name: String, parent_id: Option<u64> },
    #[serde(rename = "delete_entity")]
    DeleteEntity { entity_id: u64 },
    #[serde(rename = "suggest_fix")]
    SuggestFix { description: String, entity_id: Option<u64> },
}
```

Applied actions execute immediately via the sindri-server API. `suggest_fix` returns to the editor as a card awaiting user approval.

---

## Editor — Tauri rebuild

### First: delete the old editor

```bash
rm -rf editor/
```

Do not look at or reference anything that was there.

### Stack

- Tauri 2, React 18 + TypeScript, Vite
- Monaco Editor (`@monaco-editor/react`)
- pnpm

No UI component libraries. No Tailwind. All styles are handwritten CSS.

### package.json

```json
{
  "dependencies": {
    "@monaco-editor/react": "^4.6.0",
    "@tauri-apps/api": "^2",
    "react": "^18",
    "react-dom": "^18"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@types/react": "^18",
    "@types/react-dom": "^18",
    "typescript": "^5",
    "vite": "^5"
  }
}
```

---

## Design system

All tokens in `editor/src/styles/tokens.css`, imported globally.

### Color tokens

```css
:root {
  --bg-0: #0a0b0d;
  --bg-1: #0f1114;
  --bg-2: #141619;
  --bg-3: #1a1d21;
  --bg-4: #20242a;
  --border: #252930;
  --border-bright: #2e3440;
  --text-dim: #3d4554;
  --text-muted: #5a6478;
  --text-base: #8a95a8;
  --text-bright: #c4cdd8;
  --text-white: #e8edf2;
  --accent: #e8a838;
  --accent-dim: #b07820;
  --accent-glow: rgba(232, 168, 56, 0.12);
  --ai: #5b8aff;
  --ai-dim: #3a5ecc;
  --ai-glow: rgba(91, 138, 255, 0.10);
  --green: #4ecb8a;
  --red: #e85050;
  --radius: 3px;
}
```

### Amber/blue semantic rule — absolute

**Amber = user. Blue = AI. No exceptions.**

- Selected entities → amber left border + amber glow
- Play button → amber fill
- User chat messages → amber glow border
- AI chat messages → blue glow border
- AI-modified code lines → blue left border decoration in Monaco
- AI status indicator → blue
- "AI observing" badge → blue
- Action card "suggestion" badge → blue
- Action card "applied" badge → green
- Inspector field focused by user → amber border
- Inspector field changed by AI → blue border

### Typography

```css
--font-mono: 'Geist Mono', monospace;
--font-ui: 'Syne', sans-serif;
```

Google Fonts:
```html
<link href="https://fonts.googleapis.com/css2?family=Geist+Mono:wght@300;400;500;600&family=Syne:wght@400;500;600;700;800&display=swap" rel="stylesheet">
```

- `--font-mono` is the default for the entire editor.
- `--font-ui` is used only for: logo, panel headers, AI title, action card titles, button labels.
- Base size: `12px`. Never below `9px`.

### Layout

```css
--panel-w-left: 220px;
--panel-w-right: 300px;
--header-h: 38px;
--footer-h: 24px;
--script-h: 200px;
```

Fixed three-column, no resize in v1:

```
┌──────────────────────────────────────────────┐  header 38px
├──────────┬────────────────────────┬──────────┤
│  LEFT    │      VIEWPORT          │  RIGHT   │
│  220px   │      flex: 1           │  300px   │
│          ├────────────────────────┤          │
│          │   SCRIPT EDITOR 200px  │          │
├──────────┴────────────────────────┴──────────┤  footer 24px
```

Global scrollbars:
```css
::-webkit-scrollbar { width: 3px; height: 3px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border-bright); border-radius: 2px; }
```

---

## Panel specifications

### Header

Background `--bg-1`. Bottom border. Left to right:
1. `sindri-logo.svg` + "SINDRI" Syne 800 14px amber letter-spacing -0.02em. Right margin 24px.
2. Menu: File, Edit, Scene, View, Build. Mono 11px `--text-muted`. Hover `--bg-3`.
3. Separator 1px 18px.
4. Tool buttons: ↖ ✥ ⤢ ↻. 28×26px. Active: amber glow + amber.
5. Separator.
6. Run button: amber fill, `--bg-0` text, Syne 700 11px uppercase, circle prefix.
7. AI status chip right: blue glow, blue border, pulsing blue dot, "qwen2.5-vl · local".

### Left panel

**Scene section (flex-grows):**

Header "SCENE" Syne 600 9px `--text-dim` uppercase 0.12em. "+" right.

Each hierarchy item: `[connectors] [arrow?] [icon] [name] [badge?]`

**Tree connectors — do not use padding/indentation:**

`.h-indent` = 16px wide `position: relative`. Four types using `::before`/`::after`:
- `.line` — full-height vertical line at left 7px, 1px `--border-bright`
- `.tee` — same + 6px horizontal stub at top 50%
- `.elbow` — vertical top→50% only + 6px horizontal stub at 50%
- `.blank` — empty spacer

Non-last child gets `tee`. Last child gets `elbow`. Ancestor columns that continue use `line`. See `sindri-editor.html` for the exact visual result.

Selected: 2px amber `::before` left border, amber glow background.
AI badge: blue glow, blue text, blue border.

**Assets section:** 3-column grid of square thumbs. 18px emoji + 8px label. "New" cell: amber border + amber glow.

### Viewport

Tabs (30px): Scene / Game. Active `--bg-4`. Right: resolution + fps `--text-dim`.

Poll `GET /screenshot` at 15fps, display as `<img>`. 30fps during play.

Overlays:
- Selection info bottom-left (entity name + x/y/w/h). Dark bg, blur.
- "AI observing" top-right (blue dot + text). Only when `viewport` chip active.

Selected entity: 7px amber corner handles.

### Script editor

Tabs (30px) with × close. Active: `--bg-1` three-side border.

Monaco with `sindri-dark` theme:
```typescript
monaco.editor.defineTheme('sindri-dark', {
  base: 'vs-dark', inherit: true,
  rules: [
    { token: 'keyword',  foreground: 'c792ea' },
    { token: 'type',     foreground: 'ffcb6b' },
    { token: 'string',   foreground: 'c3e88d' },
    { token: 'number',   foreground: 'e8a838' },
    { token: 'comment',  foreground: '3d4554' },
    { token: 'function', foreground: '82aaff' },
  ],
  colors: {
    'editor.background':                 '#0a0b0d',
    'editor.foreground':                 '#c4cdd8',
    'editorLineNumber.foreground':       '#3d4554',
    'editorLineNumber.activeForeground': '#5a6478',
    'editor.selectionBackground':        '#e8a83825',
    'editor.lineHighlightBackground':    '#0f1114',
    'editorCursor.foreground':           '#e8a838',
    'editorGutter.background':           '#0a0b0d',
  },
});
```

Language: **Lua**. Not Rust. Not JavaScript.

AI-modified lines decoration:
```typescript
{ range: new monaco.Range(line, 1, line, 1), options: { isWholeLine: true, className: 'ai-line-highlight' } }
// .ai-line-highlight { border-left: 2px solid var(--ai-dim); }
```

### AI panel (300px)

Always visible. Never collapses.

**Header (38px):** "AI Assistant" Syne 700 `--text-bright`. Model pill right: `--bg-3` `--text-muted` `--border-bright`.

**Context bar (28px):** `scene` `script` `viewport` `errors` chips. Toggle. Active: amber glow + amber. Controls what context is sent with each request.

**Inspector:** Top of AI panel. Components of selected entity. One sub-section per component. Rows: 26px, 60px label + value fields (18px `--bg-3` `--border-bright`). Reads `GET /entity/:id/components`. Writes `PATCH /entity/:id/component/:type/field/:field`. Uses `ComponentMetadataRegistry` server-side. User-focused field: amber border. AI-changed field: blue border.

**Chat messages:**
- User (right): amber glow bg, amber border, `border-radius: 8px 8px 8px 2px`
- AI (left): blue glow bg, blue border, `border-radius: 2px 8px 8px 8px`
- Sender label: 9px dim color 0.05em spacing

**Action cards:**
- Header: icon + title (Syne 600) + badge
- Body: diff view (green `+`, red `-`, dim context) or plain description
- Footer: primary action (blue glow) + dismiss (neutral)

**Typing indicator:** Three bouncing blue dots + "analyzing…"

**Chat input:** `--bg-0` section. Wrapper: `--bg-2` `--border-bright` 6px radius. Focus: blue border + blue glow. Textarea mono 11px. Footer: hint left, Send button right (blue fill Syne 700 uppercase). Enter = send, Shift+Enter = newline.

### Footer

`--bg-0` top border. Left: green dot "engine ready" · blue dot "ollama · localhost:11434" · amber dot scene name. Right: entity count + errors + engine version.

---

## Tauri commands

```rust
#[tauri::command] async fn get_scene() -> Result<String, String>
#[tauri::command] async fn patch_transform(entity_id: u64, x: Option<f32>, y: Option<f32>, scale_x: Option<f32>, scale_y: Option<f32>, rotation: Option<f32>) -> Result<(), String>
#[tauri::command] async fn get_screenshot() -> Result<String, String>
#[tauri::command] async fn send_ai_message(message: String, context: AiContextFlags) -> Result<AiResponse, String>
#[tauri::command] async fn apply_action(action: serde_json::Value) -> Result<(), String>
#[tauri::command] async fn get_script(path: String) -> Result<String, String>
#[tauri::command] async fn write_script(path: String, content: String) -> Result<(), String>
#[tauri::command] async fn get_entity_components(entity_id: u64) -> Result<serde_json::Value, String>
#[tauri::command] async fn set_component_field(entity_id: u64, component_type: String, field: String, value: serde_json::Value) -> Result<(), String>
#[tauri::command] async fn open_project(path: String) -> Result<(), String>
```

---

## AI context assembly

```typescript
interface AiRequest {
  message: string;
  scene?: string;       // GET /scene if 'scene' chip active
  script?: string;      // open file if 'script' chip active
  scriptPath?: string;
  screenshot?: string;  // base64 PNG if 'viewport' chip active
  errors?: string;      // if 'errors' chip active
}

interface AiResponse {
  text: string;
  actions: Action[];
}
```

---

## Editor startup sequence

1. Check Ollama at `localhost:11434` — warn in footer, do not block
2. Spawn engine sidecar
3. Engine starts sindri-server on `127.0.0.1:7878`
4. Poll `GET /health` until 200
5. Fetch `GET /scene` → populate hierarchy
6. Begin 15fps viewport polling
7. Footer status goes live

---

## Build and run

```bash
cargo build --workspace
cd editor && pnpm install && pnpm tauri dev
```

---

## What to delete

- `editor/` — entire directory
- Any `EDITOR_TODO.md` in the root

Do not delete anything inside `crates/sindri/`.

---

## Non-negotiables

1. Local-first AI by default. External AI API calls are allowed only when the user explicitly opts into a BYOK cloud provider in the editor UI. Ollama on localhost remains the default.
2. No UI component libraries. Hand-built only.
3. No Tailwind. CSS tokens only.
4. Amber = user. Blue = AI. Absolute.
5. Old editor is deleted. Nothing from it is referenced.
6. Do not restructure or rename existing engine files.
7. Scripting is Lua. Never reference Rhai.
8. Scene extension stays `.sindri`.

---

## Visual reference

`sindri-editor.html` is the interactive visual ground truth. Open it in a browser. Every layout decision, color value, spacing, connector line, chat bubble, and action card is defined there. When in doubt, check the mockup.

`sindri-logo.svg` goes in the editor header.
