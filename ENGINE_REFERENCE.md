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

#### Transform
```rust
pub struct Transform {
    pub x: f32,        // world position X
    pub y: f32,        // world position Y
    pub scale_x: f32,  // default 1.0
    pub scale_y: f32,  // default 1.0
    pub rotation: f32, // radians, 0.0 = no rotation
    pub z_index: i32,  // draw order (higher = drawn on top), default 0
    pub pivot_x: f32,  // 0..1, rotation/scale pivot, default 0.5 (center)
    pub pivot_y: f32,  // 0..1, rotation/scale pivot, default 0.5 (center)
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

#### PhysicsBody
```rust
pub struct PhysicsBody {
    pub body_type: BodyType,     // "Dynamic" | "Fixed" | "Kinematic"
    pub lock_rotation: bool,     // prevent angular rotation (use true for platformer players)
    pub linear_damping: f32,     // drag applied to linear velocity
    pub angular_damping: f32,
    pub collision_layer: u8,
    pub collision_mask: u32,     // bitmask of layers this body collides with
}
```
Physics requires **both** a `PhysicsBody` and a `Collider` on the same entity.
- `Dynamic` — fully simulated (players, enemies, projectiles)
- `Fixed` — immovable but collidable (ground, walls, platforms)
- `Kinematic` — script-driven movement that still interacts with dynamic bodies

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

> **This is NOT LÖVE2D, Unity, or Godot.** Use only the API documented below.

Scripts are `.lua` files attached to an entity via a `Script` component. The entity **must also have a Transform component** for position to be readable.

### Lifecycle hooks

```lua
function on_start(self)         end  -- called once on the first frame
function on_update(self, dt)    end  -- called every frame; dt = delta time in seconds
```

### `self` — the entity facade

`self` is an **engine userdata object** (not a plain table). Access components via method calls:

| Method | Returns | Description |
|--------|---------|-------------|
| `self:entity()` | integer | This entity's numeric ID |
| `self:input()` | InputFacet | Per-entity raw keyboard input (legacy) |
| `self:transform()` | TransformFacet \| nil | Position/rotation/scale |
| `self:physics()` | PhysicsFacet \| nil | Physics body |
| `self:sprite()` | SpriteFacet \| nil | Sprite tint/visibility |
| `self:animated_sprite()` | AnimatedSpriteFacet \| nil | Clip control |
| `self:camera()` | CameraFacet \| nil | Camera control |
| `self:tilemap()` | TilemapFacet \| nil | Tile queries/mutations |
| `self:nav_grid()` | NavGridFacet \| nil | Pathfinding |
| `self:world()` | WorldFacet | Scene queries/spawn/despawn |
| `self:find_path(target)` | array \| nil | Shortcut to nav_grid pathfinding |

### `input` global — action-based input (preferred)

Reads named actions from `input_map.json` in the project root. Actions are defined in the editor's Input tab.

```lua
input.pressed("action_name")       -- bool: any binding held
input.just_pressed("action_name")  -- bool: first frame only
input.just_released("action_name") -- bool: frame it was released
input.axis("action_name")          -- -1..1 from KeyAxis or GamepadAxis binding
```

**`input_map.json` format** (edit via the editor's Input tab):
```json
{
  "actions": [
    { "name": "jump", "bindings": [
        { "type": "key", "key": " " },
        { "type": "gamepad_button", "button": "South" }
    ]},
    { "name": "move", "bindings": [
        { "type": "key_axis", "negative": "a", "positive": "d" },
        { "type": "gamepad_axis", "axis": "LeftStickX", "deadzone": 0.2 }
    ]}
  ]
}
```

Binding types: `key`, `key_axis`, `gamepad_button`, `gamepad_axis`.

### Raw input — `self:input()` (legacy)

For cases where named actions aren't needed:

```lua
local input = self:input()
input:is_key_down("A")           -- bool: true while key held
input:is_key_pressed("Space")    -- bool: true only on the frame key was first pressed
input:axis("A", "D")             -- float -1..1: neg key = -1, pos key = +1, both/neither = 0
```

Raw key name strings: Single letters `"a"`–`"z"`. Arrows: `"ArrowLeft"`, `"ArrowRight"`, `"ArrowUp"`, `"ArrowDown"`. Special: `" "` (space), `"Enter"`, `"Escape"`. Also available as globals: `key_down(key)`, `key_pressed(key)`.

### TransformFacet — `self:transform()`

```lua
local t = self:transform()
if t == nil then return end      -- always nil-check
t:position()                     -- Vec2
t:rotation()                     -- float (radians)
t:set_position(vec2(x, y))
t:set_rotation(radians)
t:set_scale(vec2(sx, sy))
```

### PhysicsFacet — `self:physics()`

```lua
local phys = self:physics()
if phys == nil then return end   -- nil if entity has no PhysicsBody component
phys:velocity()                  -- Vec2: current linear velocity
phys:set_velocity(vec2(vx, vy)) -- directly set linear velocity
phys:apply_impulse(vec2(ix,iy)) -- add an instantaneous impulse
phys:contacts()                  -- table of entity name strings currently touching this entity
```

### Global `entity_transform(name)`

```lua
local t = entity_transform("Player")   -- returns {x, y, rotation} or nil if not found
if t then
    local dx = t.x - self.x
    local dy = t.y - self.y
end
```

Returns a snapshot table `{x: float, y: float, rotation: float}` for the named entity based on the scene state at the start of the current frame. Returns `nil` if no entity with that name exists.

### SpriteFacet — `self:sprite()`

```lua
local spr = self:sprite()
if spr == nil then return end
spr:set_tint({r, g, b, a})      -- RGBA floats 0..1
spr:set_visible(bool)
```

### NavGridFacet — `self:nav_grid()`

```lua
local nav = self:nav_grid()
if nav == nil then return end
nav:find_path({x,y}, {x,y})      -- returns [{x,y},...] or nil
nav:is_walkable(tx, ty)           -- bool
nav:set_walkable(tx, ty, bool)    -- queued
nav:world_to_tile({x,y})          -- {x, y} tile coord
nav:tile_to_world(tx, ty)         -- {x, y} world coord
nav:build_from_tilemap(entity_id, {blocked_ids...})
nav:fill_all(bool)
```

Shortcut: `self:find_path(target)` — calls nav_grid pathfinding directly, returns `[{x,y},...]` or nil.

### WorldFacet — `self:world()`

```lua
local w = self:world()
w:find_entity("name")             -- entity id or nil
w:entity_transform("name")        -- {x, y, rotation} or nil
w:spawn(name, x, y)               -- queued
w:despawn(entity_id)              -- queued
```

### Global `vec2`

```lua
vec2(x, y)   -- construct a Vec2 value; x and y are accessible as .x and .y
```

### Script examples

**Move with WASD (transform-based):**
```lua
local speed = 200.0

function on_update(self, dt)
    local input = self:input()
    local t = self:transform()
    if t == nil then return end

    local pos = t:position()
    local h = input:axis("A", "D")
    local v = input:axis("W", "S")
    t:set_position(vec2(pos.x + h * speed * dt, pos.y + v * speed * dt))
end
```

**Platformer player (physics-based — requires PhysicsBody Dynamic + Collider):**
```lua
local speed = 260.0
local jump_impulse = -520.0
local grounded_threshold = 30.0

function on_update(self, dt)
    local input = self:input()
    local phys = self:physics()
    if phys == nil then return end

    local vel = phys:velocity()
    local h = input:axis("A", "D")
    if h == 0.0 then h = input:axis("Left", "Right") end

    local vy = vel.y
    local grounded = math.abs(vel.y) < grounded_threshold
    if grounded and (input:is_key_pressed("W") or input:is_key_pressed("Up") or input:is_key_pressed("Space")) then
        vy = jump_impulse
    end

    phys:set_velocity(vec2(h * speed, vy))
end
```

**Rotate and pulse a sprite:**
```lua
local time = 0.0

function on_update(self, dt)
    time = time + dt
    local t = self:transform()
    if t ~= nil then
        t:set_rotation(time * 0.65)
        local pulse = 1.0 + math.sin(time * 2.5) * 0.08
        t:set_scale(vec2(pulse, pulse))
    end
    local spr = self:sprite()
    if spr ~= nil then
        local glow = 0.72 + math.sin(time * 3.0) * 0.2
        spr:set_tint({1.0, glow, 0.22, 1.0})
    end
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
