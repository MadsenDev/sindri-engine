# Sindri Engine Reference

This is the complete factual reference for the Sindri engine. Use it when writing, editing, or analysing code that targets this engine.

---

## Architecture overview

Sindri is a Rust 2D game engine built on wgpu (GPU rendering) + winit (windowing) + rapier2d (physics) + mlua (Lua scripting). The editor communicates with a running engine instance via an HTTP bridge on port 7878.

Two ECS systems exist and must not be confused:

| System | Purpose | Entity ID type |
|--------|---------|----------------|
| `sindri::world::World` | Runtime ECS — stores typed components during gameplay | `EntityId(u32)` |
| `sindri::scene::Scene` | Editor/serialization ECS — the scene graph the editor reads and writes | `u64` |

The editor only ever interacts with the `scene::Scene` layer. The engine runtime uses `world::World`. Scene files (`.sindri`) are plain JSON.

---

## Editor scene model (`sindri::scene` + `sindri::entity` + `sindri::component`)

### Entity

```rust
pub struct Entity {
    pub id: u64,
    pub name: String,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
    pub components: Vec<Component>,
    pub active: bool,
}
```

### Component enum

All components are variants of this enum. Serialised with `"type"` discriminant.

```rust
pub enum Component {
    Transform(Transform),
    Sprite(Sprite),
    Collider(Collider),
    Script(Script),
    Camera(Camera),
    AudioSource(AudioSource),
}
```

#### Transform
```rust
pub struct Transform {
    pub x: f32,        // world position X
    pub y: f32,        // world position Y
    pub scale_x: f32,  // default 1.0
    pub scale_y: f32,  // default 1.0
    pub rotation: f32, // radians, 0.0 = no rotation
}
```
Every visible entity needs a Transform. Positive Y is down in screen space.

#### Sprite
```rust
pub struct Sprite {
    pub texture_path: String, // relative path from project root, e.g. "assets/player.png"
    pub width: f32,
    pub height: f32,
    pub flip_x: bool,
    pub flip_y: bool,
    pub color: [f32; 4], // RGBA tint, [1,1,1,1] = no tint
}
```

#### Collider
```rust
pub struct Collider {
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,    // offset from entity position
    pub offset_y: f32,
    pub is_trigger: bool, // true = sensor (no physics response, only overlap events)
}
```
Colliders are always axis-aligned boxes in the editor model.

#### Script
```rust
pub struct Script {
    pub path: String, // relative path to .lua script, e.g. "scripts/player.lua"
}
```
Scripts are Lua files. See scripting section for the full API.

#### Camera
```rust
pub struct Camera {
    pub zoom: f32,                   // 1.0 = default, 2.0 = zoomed in 2×
    pub follow_entity: Option<u64>,  // entity ID to follow, null = static camera
}
```
Only one camera should be active at a time. The entity also needs a Transform.

#### AudioSource
```rust
pub struct AudioSource {
    pub path: String,         // e.g. "assets/sfx/jump.ogg"
    pub volume: f32,          // 0.0–1.0
    pub looping: bool,
    pub play_on_start: bool,
}
```

### Scene
```rust
pub struct Scene {
    pub name: String,
    pub entities: HashMap<u64, Entity>,
    pub next_id: u64,
}
```

Key methods:
- `scene.spawn("name") -> u64` — creates entity, returns ID
- `scene.set_parent(child, parent)` — wires parent/child relationship
- `scene.add_component(id, component)` — appends component to entity
- `scene.remove_entity(id)` — removes entity, orphans children (does NOT cascade delete)
- `scene.save(path)` / `scene.load(path)` — `.sindri` files

---

## HTTP bridge API (sindri-server, port 7878)

The editor talks to the running engine exclusively through these routes. All bodies are JSON.

| Method | Path | Notes |
|--------|------|-------|
| GET | `/health` | `{"status":"ok","model":"qwen2.5-vl:7b"}` |
| GET | `/scene` | Full scene JSON |
| PUT | `/scene` | Replace entire scene |
| GET | `/scene/entity/:id` | Single entity |
| PATCH | `/scene/entity/:id/transform` | Body: `{x?, y?, scale_x?, scale_y?, rotation?}` |
| POST | `/scene/entity` | Body: `{name, parent_id?}` → `{id}` |
| DELETE | `/scene/entity/:id` | Removes entity, orphans children |
| GET | `/screenshot` | `{"image": "<base64 PNG>"}` |
| GET | `/scripts` | Array of script file paths |
| GET | `/script?path=…` | Script file contents as text |
| PUT | `/script?path=…` | Body: `{content}` |

Port is `127.0.0.1:7878` by default, overridable via `SINDRI_PORT` env var.

---

## Runtime ECS (`sindri::world::World`)

Used during gameplay — NOT the editor scene. EntityId is `EntityId(u32)`.

```rust
world.spawn() -> EntityId
world.despawn(entity)
world.insert(entity, component: T)
world.remove::<T>(entity) -> Option<T>
world.get::<T>(entity) -> Option<&T>
world.get_mut::<T>(entity) -> Option<&mut T>
world.query::<T>() -> Vec<(EntityId, &T)>
world.is_alive(entity) -> bool
world.entities() -> Vec<EntityId>
```

Runtime component types (in `sindri::entities`):
- `Transform { position: Vec2, rotation: f32, scale: Vec2 }`
- `SpriteComponent { texture: TextureHandle, width, height, flip_x, flip_y, color, visible }`
- `CameraComponent { camera: Camera2D, active: bool }`
- `PhysicsBody` — marker that entity has a rapier2d rigid body
- `AudioSource { handle: SoundHandle, volume, looping }`
- `ScriptComponent` — entity has an attached Lua script
- `ScriptTag(String)` — string tag queryable from scripts
- `Player`, `Enemy`, `Collectible`, `Checkpoint`, `Hazard`, `Trigger`, `MovingPlatform` — gameplay marker components

---

## Lua scripting system

> **This is NOT LÖVE2D, Unity, or Godot.** Do not use `love.*`, `Input.GetKey`, `self:input()`, or any other engine's API. Use only the globals and patterns documented below.

Scripts are `.lua` files attached to an entity via a `Script` component. The entity **must also have a Transform component** for position changes to take effect.

### Lifecycle hooks

```lua
function on_start(self)         end  -- called once on the first frame
function on_update(self, dt)    end  -- called every frame; dt = delta time in seconds
```

### The `self` table

`self` is a plain Lua table passed into every hook. Read and write these fields directly:

| Field | Type | Description |
|-------|------|-------------|
| `self.x` | number | World X position (read/write) |
| `self.y` | number | World Y position (read/write) |
| `self.rotation` | number | Rotation in radians (read/write) |
| `self.scale_x` | number | X scale (read/write) |
| `self.scale_y` | number | Y scale (read/write) |
| `self.entity_id` | number | Entity ID (read-only) |
| `self.elapsed` | number | Total elapsed time in seconds (read-only) |

Changes to `self.x`, `self.y`, etc. are written back to the entity's Transform after each `on_update` call.

### Global functions

```lua
key_down(key)      -- bool: true while the key is held
key_pressed(key)   -- bool: true only on the frame the key was first pressed
print(...)         -- logs to the engine console
```

Key name strings match browser `KeyboardEvent.key` values:
`"ArrowLeft"`, `"ArrowRight"`, `"ArrowUp"`, `"ArrowDown"`, `"a"`–`"z"`, `"A"`–`"Z"`, `"0"`–`"9"`, `" "` (Space), `"Enter"`, `"Escape"`, `"Shift"`, `"Control"`, `"Alt"`

### Script examples

**Move with arrow keys:**
```lua
function on_update(self, dt)
  local speed = 200
  if key_down("ArrowLeft")  then self.x = self.x - speed * dt end
  if key_down("ArrowRight") then self.x = self.x + speed * dt end
  if key_down("ArrowUp")    then self.y = self.y - speed * dt end
  if key_down("ArrowDown")  then self.y = self.y + speed * dt end
end
```

**Rotate over time:**
```lua
function on_update(self, dt)
  self.rotation = self.elapsed  -- 1 radian per second
end
```

**Bounce between two positions:**
```lua
function on_start(self)
  self.start_x = self.x
end

function on_update(self, dt)
  self.x = self.start_x + math.sin(self.elapsed * 2) * 100
end
```

**Spawn-once log:**
```lua
function on_start(self)
  print("Entity " .. self.entity_id .. " started at " .. self.x .. ", " .. self.y)
end
```

---

## Physics system (`sindri::physics::PhysicsWorld`)

Physics is rapier2d under the hood. Bodies are separate from the editor's `Collider` component — they are created from Rust code, not from the editor scene. The editor `Collider` component describes the shape; actual physics setup is in game code.

Body types: `Dynamic`, `Fixed`, `KinematicPositionBased`, `KinematicVelocityBased`

Collider shapes: `Box { hx, hy }` (half-extents), `Circle { radius }`, `CapsuleY { half_height, radius }`

---

## Rendering (`sindri::render::Renderer`)

The renderer draws the world each frame. It manages texture handles, sprites, tilemaps, particles, lighting.

- `TextureHandle` — opaque handle returned by `renderer.load_texture(path)`
- `Sprite { texture, x, y, width, height, ... }` — drawn each frame
- `AnimatedSprite` — sprite with named animation clips + current frame tracking
- `Tilemap` / `Tile` — grid-based tile rendering
- `ParticleSystem` / `ParticleEmitter` — particle effects
- `PointLight` / `DirectionalLight` — simple 2D lighting

Coordinate system: origin top-left, X right, Y down, pixels.

---

## Audio (`sindri::audio::AudioSystem`)

```rust
audio.play(path)                    // fire and forget
audio.play_looping(path) -> SoundHandle
audio.stop(handle)
audio.set_volume(handle, 0.0..=1.0)
```

Formats: WAV, OGG, MP3 (via rodio).

---

## Scene save/load for game code (`sindri::scene_physics`)

The physics snapshot system (separate from the editor scene format) is used by game code to checkpoint/restore physics state:

```rust
let snapshot = create_scene(&physics_world);  // captures physics state
restore_scene_physics(&mut physics_world, &snapshot);  // restores it
snapshot.save_to_file(path)?;
let loaded = Scene::load_from_file(path)?;
```

This is distinct from the editor's `sindri::scene::Scene` — do not confuse them.

---

## AI action schema

When the AI takes actions they must match these types exactly:

```json
{ "type": "edit_transform", "entity_id": 0, "x": 100, "y": 200 }
{ "type": "edit_transform", "entity_id": 0, "scale_x": 2.0, "scale_y": 2.0 }
{ "type": "create_entity", "name": "Coin", "parent_id": null }
{ "type": "delete_entity", "entity_id": 3 }
{ "type": "write_script", "path": "scripts/player.lua", "content": "..." }
{ "type": "suggest_fix", "description": "Collider height doesn't match sprite", "entity_id": 0 }
```

All transform fields in `edit_transform` are optional — only include fields being changed. `entity_id` references the scene's `u64` entity IDs (the `id` field on each entity in the scene JSON).
