# Lua Scripting

Sindri uses Lua 5.4 scripting through `mlua`.

Scripts are attached directly to entities using `ScriptComponent` and are driven by `ScriptRuntime`.

The scripting model is designed around **entity-attached behavior**, similar to Unity-style gameplay scripting:

* Rust owns engine systems, rendering, physics, and orchestration
* Lua owns per-entity gameplay behavior
* Scripts safely mutate the world through a deferred command system
* Scripts support hot reload during runtime

This allows gameplay iteration without recompiling the engine or game code.

---

# Philosophy

Sindri scripting is intended for:

* player movement
* enemy logic
* bullet behavior
* triggers
* camera behavior
* animation logic
* gameplay state machines
* scripted events
* editor-driven behaviors

Rust remains responsible for:

* engine architecture
* rendering systems
* ECS/world ownership
* physics simulation
* asset management
* serialization
* networking
* platform/runtime integration

Scripts should control behavior, not own the engine.

---

# Attaching Scripts

Scripts are attached through `ScriptComponent`.

```rust
use sindri::{ScriptComponent, ScriptParams};

let params = ScriptParams::default()
    .insert("speed", 220.0f32)
    .insert("jump_force", 450.0f32);

world.insert(
    player,
    ScriptComponent::default().with_script(
        "scripts/player.lua",
        params,
    ),
);
```

A single entity may contain multiple script attachments.

---

# Script Parameters

Rust can pass configurable values into scripts through `ScriptParams`.

Lua receives them through the global `params` table.

## Rust

```rust
let params = ScriptParams::default()
    .insert("speed", 180.0f32)
    .insert("gravity", true)
    .insert("spawn", Vec2::new(64.0, 32.0));
```

## Lua

```lua
local speed = params.speed or 180.0
local gravity_enabled = params.gravity
local spawn = params.spawn
```

Supported parameter types:

* `f32`
* `bool`
* `String`
* `Vec2`

---

# Lifecycle

Scripts may define any subset of these callbacks.

Missing callbacks are ignored.

```lua
function on_create(self) end
function on_start(self) end

function on_update(self, dt) end
function on_fixed_update(self, fixed_dt) end
function on_post_physics(self, fixed_dt) end

function on_draw(self) end

function on_destroy(self) end

function on_collision_enter(self, other_entity) end
function on_collision_exit(self, other_entity) end

function on_trigger_enter(self, other_entity) end
function on_trigger_exit(self, other_entity) end
```

---

# Runtime Stages

## `on_create`

Called immediately after the script instance is created.

Use for:

* initial setup
* reading params
* cached state initialization

---

## `on_start`

Called after creation once the entity is fully initialized.

Use for:

* setup requiring other components
* startup gameplay logic

---

## `on_update`

Runs every frame.

Use for:

* player input
* timers
* camera logic
* non-physics gameplay behavior

---

## `on_fixed_update`

Runs during the fixed simulation step.

Use for:

* physics movement
* impulses
* deterministic gameplay

---

## `on_post_physics`

Runs after the physics step completes.

Use for:

* reading resolved collisions
* post-simulation corrections
* camera smoothing

---

## `on_destroy`

Called before the script instance is removed.

Use for:

* cleanup
* despawning related entities
* saving transient state

---

# Hot Reload

`ScriptRuntime` supports hot reload.

```rust
let runtime = ScriptRuntime::new()?
    .with_hot_reload(true);
```

When enabled:

* modified `.lua` files are automatically reloaded
* existing script instances are rebuilt
* `on_destroy` is called on the old instance
* `on_create` and `on_start` run again

This allows gameplay iteration while the game is running.

---

# The `self` Object

Every callback receives a `self` object exposing engine facets and helpers.

```lua
function on_update(self, dt)
  print(self:entity())
end
```

---

# Core Helpers

## Entity ID

```lua
local id = self:entity()
```

---

## Time

```lua
local time = self:time()

local dt = time:delta()
local fixed = time:fixed_delta()
```

---

## Input

```lua
local input = self:input()

if input:is_key_down("W") then
  print("moving")
end
```

### Supported Input Helpers

```lua
input:is_key_down(name)
input:is_key_pressed(name)
input:is_key_released(name)

input:axis("A", "D")
input:axis_raw("A", "D")

input:is_mouse_down("Left")
input:is_mouse_pressed("Left")

input:mouse_pos_screen()
```

---

# Input Example

```lua
function on_update(self, dt)
  local input = self:input()
  local transform = self:transform()

  if transform == nil then
    return
  end

  local move = input:axis("A", "D")

  local pos = transform:position()

  transform:set_position(
    vec2(pos.x + move * 180 * dt, pos.y)
  )
end
```

---

# Transform Facet

Available if the entity has a `Transform`.

```lua
local transform = self:transform()

if transform ~= nil then
  local pos = transform:position()
  local rot = transform:rotation()

  transform:set_position(vec2(100, 50))
  transform:set_rotation(1.57)
  transform:set_scale(vec2(2, 2))
end
```

---

# Physics Facet

Available if the entity owns a physics body.

```lua
local physics = self:physics()

if physics ~= nil then
  physics:set_velocity(vec2(120, 0))
  physics:apply_impulse(vec2(0, -300))
end
```

## Physics Helpers

```lua
physics:velocity()
physics:set_velocity(vec2)
physics:apply_impulse(vec2)
```

---

# Sprite Facet

Available if the entity has `SpriteComponent`.

```lua
local sprite = self:sprite()

if sprite ~= nil then
  sprite:set_visible(true)
  sprite:set_tint({1.0, 0.5, 0.5, 1.0})
end
```

---

# Animation Facet

Available if the entity has `AnimatedSprite`.

```lua
local animation = self:animation()

if animation ~= nil then
  animation:play()
  animation:set_speed(2.0)
end
```

## Animation Helpers

```lua
animation:update(dt)
animation:play()
animation:pause()
animation:reset()
animation:set_speed(speed)

animation:current_frame_index()
```

---

# Camera Facet

Available if the entity has `CameraComponent`.

```lua
local camera = self:camera()

if camera ~= nil then
  camera:set_zoom(2.0)
  camera:shake(5.0, 0.3)
end
```

## Camera Helpers

```lua
camera:is_active()

camera:set_active(bool)

camera:zoom()
camera:set_zoom(value)
camera:zoom_to(target, speed)

camera:offset()
camera:set_offset(vec2)

camera:set_bounds(min, max)
camera:clear_bounds()

camera:shake(intensity, duration)
```

---

# Tilemap Facet

Available if the entity has `TilemapComponent`.

```lua
local tilemap = self:tilemap()

if tilemap ~= nil then
  tilemap:set_tile(4, 8, 3)
end
```

## Tilemap Helpers

```lua
tilemap:set_tile(x, y, tile_id)
tilemap:get_tile(x, y)

tilemap:fill_rect(x, y, w, h, tile_id)

tilemap:world_to_tile(world_pos)
tilemap:tile_to_world(x, y)
```

---

# World Helpers

Scripts can safely interact with the world through the world facet.

```lua
local world = self:world()
```

---

## Entity Queries

### Find one tagged entity

```lua
local player = world:find_by_tag("player")
```

### Find all tagged entities

```lua
local enemies = world:find_all_by_tag("enemy")
```

---

## Query Tag

```lua
local tag = world:tag_of(entity_id)
```

---

## Entity Position

```lua
local pos = world:position(entity_id)
```

---

## Collider Size

```lua
local size = world:collider_size(entity_id)
```

---

## Despawn Entity

```lua
world:despawn(entity_id)
```

---

# Spawning

Scripts may request safe deferred spawns.

## Spawn Dynamic Physics Body

```lua
world:spawn_dynamic(
  vec2(100, 100),
  vec2(250, 0)
)
```

## Spawn Empty Entity

```lua
world:spawn_empty(
  vec2(64, 64),
  "enemy"
)
```

Spawn/despawn requests are buffered and safely applied after script execution completes.

---

# Safe World Mutation

Scripts never directly mutate the engine world.

All world changes are buffered through `ScriptCommandBuffer`.

This prevents:

* iterator invalidation
* ECS corruption
* unsafe physics mutation
* mid-update entity deletion

Scripts request changes.

The runtime safely applies them afterward.

This is one of the core architectural guarantees of Sindri scripting.

---

# Collision Callbacks

Scripts may respond to collision and trigger events.

```lua
function on_collision_enter(self, other)
  print("hit", other)
end

function on_trigger_enter(self, other)
  print("entered trigger")
end
```

---

# Vec2

Sindri exposes a built-in `vec2()` constructor.

```lua
local velocity = vec2(120, 40)
```

Returned vectors expose:

```lua
vec:x()
vec:y()

vec:set_x(value)
vec:set_y(value)
```

---

# Full Example: Player Controller

```lua
local speed = params.speed or 240.0

function on_update(self, dt)
  local input = self:input()
  local transform = self:transform()

  if transform == nil then
    return
  end

  local move_x = input:axis("A", "D")
  local move_y = input:axis("W", "S")

  local pos = transform:position()

  transform:set_position(
    vec2(
      pos.x + move_x * speed * dt,
      pos.y + move_y * speed * dt
    )
  )
end
```

---

# Full Example: Bullet Lifetime

```lua
local speed = params.speed or 500.0
local lifetime = params.lifetime or 1.5

local velocity = vec2(0, 0)

function on_start(self)
  local transform = self:transform()

  if transform == nil then
    return
  end

  local rotation = transform:rotation()

  velocity = vec2(
    math.cos(rotation) * speed,
    math.sin(rotation) * speed
  )
end

function on_update(self, dt)
  lifetime = lifetime - dt

  local transform = self:transform()

  if transform == nil then
    return
  end

  local pos = transform:position()

  transform:set_position(
    vec2(
      pos.x + velocity.x * dt,
      pos.y + velocity.y * dt
    )
  )

  if lifetime <= 0.0 then
    self:world():despawn(self:entity())
  end
end
```

---

# Driving The Runtime

The game loop is responsible for driving the scripting runtime.

```rust
runtime.update(
    &mut world,
    &mut physics,
    ctx.input(),
    ctx.delta_seconds(),
)?;

runtime.fixed_update(
    &mut world,
    &mut physics,
    ctx.input(),
    ctx.fixed_delta_seconds(),
)?;

physics.step(ctx.fixed_delta_seconds());

runtime.post_physics_update(
    &mut world,
    &mut physics,
    ctx.input(),
    ctx.fixed_delta_seconds(),
)?;

let events = physics.drain_events();

runtime.handle_physics_events(
    &events,
    &mut world,
    &mut physics,
    ctx.input(),
)?;
```

See the curated scripting examples for complete integration patterns.

---

# Recommended Usage

Good scripting targets:

* gameplay behavior
* enemies
* weapons
* UI logic
* triggers
* camera logic
* cutscenes
* scripted interactions

Less ideal scripting targets:

* renderer internals
* ECS architecture
* asset pipelines
* networking
* heavy simulation systems
* serialization infrastructure

---

# Current Limitations

Current scripting intentionally keeps a constrained API surface.

Scripts do not directly:

* own ECS storage
* mutate arbitrary components
* access renderer internals
* manage assets
* allocate engine resources

This keeps scripts predictable, hot-reload-friendly, and safe to iterate on.

The scripting API surface will expand gradually as workflows stabilize.
