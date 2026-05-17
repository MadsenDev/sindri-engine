# Sindri scripting overview

Sindri now ships with a lightweight, Unity-like scripting layer powered by [Rhai](https://rhai.rs/). Scripts are treated as components that attach behavior to entities without exposing internal engine state.

## Key concepts
- **ScriptComponent**: a component that holds an ordered list of script attachments (file path plus optional `ScriptParams`). Attach it to an entity to run one or more scripts in insertion order.
- **ScriptRuntime**: the orchestrator that loads/compiles Rhai modules, instantiates scripts per entity, drives lifecycle callbacks, and applies deferred world mutations through a script-only command buffer.
- **Self**: the only object visible to scripts. It exposes component-scoped facets for reading/writing the owner entity, plus read-only views of input/time and bounded world helpers.

## Lifecycle callbacks
Scripts can implement any subset of these functions; missing callbacks are skipped automatically.

```rhai
fn on_create(self) { /* called once when attached */ }
fn on_start(self) { /* called once after creation */ }
fn on_update(self, dt) { /* per-frame */ }
fn on_fixed_update(self, fixed_dt) { /* fixed timestep */ }
fn on_draw(self) { /* optional debug-only drawing hook */ }
fn on_destroy(self) { /* before removal */ }

// Physics events from the engine
fn on_collision_enter(self, other_entity)
fn on_collision_exit(self, other_entity)
fn on_trigger_enter(self, other_entity)
fn on_trigger_exit(self, other_entity)
```

## Safe API surface (`Self` + facets)
- Entity info: `self.entity()`
- Timing: `self.time().delta()`, `self.time().fixed_delta()`
- Transform accessors (if the entity has a Transform): `self.transform().position()`, `self.transform().rotation()`, `self.transform().set_position(vec2(x,y))`, `self.transform().set_rotation(radians)`, `self.transform().set_scale(vec2(x,y))` (facet calls return `()` when missing)
- Physics helpers (if the entity has a physics body): `self.physics().velocity()`, `self.physics().set_velocity(vec2)`, `self.physics().apply_impulse(vec2)` (facet calls return `()` when missing)
- Sprite helpers (if the entity has a Sprite): `self.sprite().set_visible(bool)`, `self.sprite().set_tint([r,g,b,a])`
- Input: `self.input.is_key_down/pressed/released("W"|"A"|"S"|"D"|"Space"|arrow names)`; `self.input.mouse_pos_screen()` (always available)
- World helpers: `self.world().find_by_tag(tag: &str) -> Option<EntityId>`, `self.world().despawn(entity_id)`
- Spawning: `self.world().spawn_dynamic(position, velocity)`, `self.world().spawn_empty(position?, tag?)`
- Optional convenience aliases: `self.position()`, `self.set_position(...)`, `self.apply_impulse(...)`

All writes are deferred through the internal command buffer and applied after script execution, which keeps the engine authoritative for rendering and physics.

## Script logging
Rhai scripts emit output through the runtime's print/debug hooks. Sindri registers default handlers so `print()` and `debug()` show up in the engine console:

```rhai
print("hello world");            // prints: [RHAI] hello world
debug("velocity=" + v.x);        // prints: [RHAI DEBUG] velocity=3.0 @ <unnamed script>:1:1
```

Notes:
- Messages are prefixed with `[RHAI]`/`[RHAI DEBUG]` to keep script logs distinct from engine output.
- `print` only accepts strings; format numbers or vectors before logging them.

## Minimal usage example
```rust
// Build an entity with scripts
let params = ScriptParams::default().insert("speed", 6.0);
world.insert(entity, ScriptComponent::default().with_script("examples/scripts/player_movement.rhai", params));

// Drive the runtime from your game loop
runtime.update(&mut world, &mut physics, ctx.input(), ctx.delta_time())?;
while ctx.should_run_fixed_update() {
    runtime.fixed_update(&mut world, &mut physics, ctx.input(), ctx.fixed_delta_time().as_secs_f32())?;
}
let events = physics.drain_events();
runtime.handle_physics_events(&events, &mut world, &mut physics, ctx.input())?;
```
