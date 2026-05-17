# Lua Scripting

Sindri uses Lua 5.4 scripting through `mlua`. Scripts are attached to entities with `ScriptComponent` and are driven by `ScriptRuntime`.

Scripts are ordinary `.lua` files. The engine keeps world mutation safe by buffering script commands and applying them after script callbacks finish.

## Attaching Scripts

```rust
use sindri::{ScriptComponent, ScriptParams};

let params = ScriptParams::default().insert("speed", 220.0f32);
world.insert(
    player,
    ScriptComponent::default().with_script("scripts/player.lua", params),
);
```

## Lifecycle

Scripts may define any of these callbacks:

```lua
function on_create(self) end
function on_start(self) end
function on_update(self, dt) end
function on_fixed_update(self, fixed_dt) end
function on_post_physics(self, fixed_dt) end
function on_destroy(self) end

function on_collision_enter(self, other_entity) end
function on_collision_exit(self, other_entity) end
function on_trigger_enter(self, other_entity) end
function on_trigger_exit(self, other_entity) end
```

Missing callbacks are skipped.

## Input

Use the input facet from `self:input()`:

```lua
function on_update(self, dt)
  local input = self:input()
  local move = input:axis("A", "D")
  local jump = input:is_key_pressed("Space")

  if move ~= 0 then
    local transform = self:transform()
    if transform ~= nil then
      local pos = transform:position()
      transform:set_position(vec2(pos.x + move * 180 * dt, pos.y))
    end
  end
end
```

Supported key names include letters `A`-`Z`, digits `0`-`9`, arrows, `Space`, `Escape`, `Enter`, `Tab`, `Backspace`, shift/control/alt aliases, and `F1`-`F12`.

## Facets

`self` exposes component-scoped helpers:

- `self:entity()`
- `self:time():delta()`
- `self:time():fixed_delta()`
- `self:input()`
- `self:world()`
- `self:transform()` if the entity has `Transform`
- `self:physics()` if the entity has a physics body
- `self:sprite()` if the entity has `SpriteComponent`
- `self:animation()` if the entity has `AnimatedSprite`
- `self:camera()` if the entity has `CameraComponent`

Physics helpers include:

```lua
local physics = self:physics()
if physics ~= nil then
  physics:set_velocity(vec2(120, 0))
  physics:apply_impulse(vec2(0, -300))
  physics:set_layer(1)
  physics:set_mask(1, 0xffffffff)
end
```

## World Helpers

Scripts can query tagged entities and request spawns/despawns through the command buffer. The exact helper surface is intentionally small so scripts do not take ownership of the engine world.

## Driving The Runtime

```rust
runtime.update(&mut world, &mut physics, ctx.input(), ctx.delta_time().as_secs_f32())?;

while ctx.should_run_fixed_update() {
    runtime.fixed_update(
        &mut world,
        &mut physics,
        ctx.input(),
        ctx.fixed_delta_time().as_secs_f32(),
    )?;
    physics.step(ctx.fixed_delta_time().as_secs_f32());
    runtime.post_physics_update(
        &mut world,
        &mut physics,
        ctx.input(),
        ctx.fixed_delta_time().as_secs_f32(),
    )?;
}

let events = physics.drain_events();
runtime.handle_physics_events(&events, &mut world, &mut physics, ctx.input())?;
```

Check the scripting examples for the exact loop shape used by current demos.
