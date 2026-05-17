# Sindri — Engine Improvements

Complete these tasks in order. One task per Claude Code session.
Check off each task when done. Do not skip ahead.
After each task, run `cargo build --workspace` and fix all errors before stopping.

---

## Task 1 — physics sync helper

**File:** `crates/sindri/src/physics.rs`

Add a public method to `PhysicsWorld`:

```rust
pub fn sync_transforms(&self, world: &mut World) {
    // For every entity that has a physics body, write the body's
    // current position and rotation back to its Transform component.
    // If an entity has a body but no Transform, skip it silently.
}
```

This is the most frequently needed helper in any game using physics. Currently every game writes this loop manually in `fixed_update`. This method eliminates that boilerplate.

Also update `lib.rs` to re-export `sync_transforms` if it is not already accessible via the `PhysicsWorld` import.

**Verify:** Write a doc comment with a usage example. Run `cargo build --workspace`. No regressions in existing examples if any exist.

---

## Task 2 — query_mut

**File:** `crates/sindri/src/world.rs`

Add a method to `World`:

```rust
pub fn query_mut<T: Any + Send>(&mut self) -> Vec<(EntityId, &mut T)>
```

Currently `query<T>` returns immutable refs, which means you cannot mutate components in a single pass — you have to collect entity IDs first, then loop again with `get_mut`. This method eliminates that pattern.

Implementation note: returning `&mut T` from a HashMap while holding a mutable borrow on `self` requires unsafe. The correct approach is to collect raw `*mut T` pointers from the HashMap values, then transmute lifetimes to produce `&mut T` refs tied to the `&mut self` lifetime. Add a clear `// SAFETY:` comment explaining that the references are non-overlapping (each entity has at most one component of type T) and that they do not outlive the borrow.

**Verify:** Add a unit test in `world.rs` that calls `query_mut`, modifies a field on each returned component, and asserts the changes persist. Run `cargo build --workspace`.

---

## Task 3 — complete parse_key in script.rs

**File:** `crates/sindri/src/script.rs`

The `parse_key` function at the bottom of the file currently only handles W/A/S/D, Space, and arrow keys. Extend it to cover:

- All letter keys: A–Z (both uppercase and lowercase string inputs mapping to `KeyCode::KeyA` etc.)
- Number keys: "0"–"9" → `KeyCode::Digit0` through `KeyCode::Digit9`
- `"Escape"` / `"escape"` → `KeyCode::Escape`
- `"Enter"` / `"enter"` → `KeyCode::Enter`
- `"Tab"` / `"tab"` → `KeyCode::Tab`
- `"Backspace"` / `"backspace"` → `KeyCode::Backspace`
- `"ShiftLeft"` / `"shift"` → `KeyCode::ShiftLeft`
- `"ShiftRight"` → `KeyCode::ShiftRight`
- `"ControlLeft"` / `"ctrl"` → `KeyCode::ControlLeft`
- `"AltLeft"` / `"alt"` → `KeyCode::AltLeft`
- `"F1"`–`"F12"` → `KeyCode::F1` through `KeyCode::F12`

Do not change the function signature. Do not break existing key name strings.

**Verify:** Run `cargo build --workspace`.

---

## Task 4 — expose InputMap axes to Lua

**File:** `crates/sindri/src/script.rs`

The `InputFacet` Lua userdata currently only exposes `is_key_down`, `is_key_pressed`, `is_key_released`, `mouse_pos_screen`, `is_mouse_pressed`, `is_mouse_down`.

Add two methods to `InputFacet`:

```
input:axis(negative_key, positive_key) -> f32
```
Returns -1.0 if `negative_key` is held, 1.0 if `positive_key` is held, 0.0 if neither or both. Uses the existing `parse_key` function internally.

```
input:axis_raw(negative_key, positive_key) -> f32
```
Same as `axis` but allows both to be pressed simultaneously (returns the sum, clamped to -1..1).

This eliminates the manual `if A then -1 elseif D then 1` pattern in virtually every movement script.

**Verify:** Run `cargo build --workspace`.

---

## Task 5 — draw_world convenience method

**File:** `crates/sindri/src/render/wgpu_backend.rs` (via `Renderer`)

Add a method to `Renderer`:

```rust
pub fn draw_world(
    &mut self,
    frame: &mut Frame,
    world: &World,
    camera: &Camera2D,
) -> Result<()>
```

This method iterates all entities with `SpriteComponent` and draws them if `sprite.visible == true`. It also draws `TilemapComponent` entities and `AnimatedSprite` entities. It does not draw physics debug overlays.

Order of drawing: tilemaps first, then sprites and animated sprites. Within each category, draw in entity ID order (ascending) as a stable default. A future z-order task will add sorting.

This eliminates the manual render loop that every game currently writes in `draw()`.

Also add the method to `lib.rs` re-exports if necessary.

**Verify:** Run `cargo build --workspace`.

---

## Task 6 — fix measure_text_width

**File:** `crates/sindri/src/render/wgpu_backend.rs`

The `measure_text_width` method currently returns `Ok(0.0)` with a TODO comment. Implement it properly using glyphon's layout system.

The method signature is:
```rust
pub fn measure_text_width(&mut self, text: &str, font: FontHandle, size: f32) -> Result<f32>
```

Use the same glyphon `GlyphonBuffer` setup as `draw_text` (shape the text, call `shape_until_scroll`) but instead of rendering, read back the layout width from the buffer's layout runs. The width is the sum of all glyph advances in the first line.

This is needed for correct HUD text alignment — `HudText` with `TextAlign::Center` and `TextAlign::Right` currently produce wrong positions because they fall back to the approximate `text.len() * size * 0.6` estimate.

**Verify:** Run `cargo build --workspace`. The fallback approximation in `hud.rs` can remain as a fallback but the primary path should now use real measurement.

---

## Task 7 — colored quad particles

**File:** `crates/sindri/src/render/wgpu_backend.rs`

In `draw_particles` (inside `Renderer`), there is a branch:
```rust
} else {
    // If no texture, we'll render as a colored quad
    // For now, use a 1x1 white texture if available, or skip
    // TODO: Add support for rendering particles as colored quads
    continue;
}
```

Replace the `continue` with a draw call using the circle texture (already used by `draw_circle`). If `circle_texture` is None, call `ensure_circle_texture` to create it. Use the particle's color as the tint, size as the scale, and position as the world position. This makes untextured particles render as soft colored circles.

Also update the `Renderer::draw_particles` public method in the same file to pass through correctly.

**Verify:** Run `cargo build --workspace`.

---

## Task 8 — collision layer/mask support

**File:** `crates/sindri/src/physics.rs`

Add collision filtering support. Rapier supports collision groups natively.

Add an optional parameter to `create_body` and `add_collider_with_material`:

```rust
pub fn set_collision_groups(
    &mut self,
    entity: EntityId,
    memberships: u32,  // bitmask: which groups this entity belongs to
    filter: u32,       // bitmask: which groups this entity collides with
)
```

This should update the collision groups on all colliders attached to the entity's body using Rapier's `InteractionGroups`.

Also add a convenience helper:
```rust
pub fn set_collision_layer(&mut self, entity: EntityId, layer: u8)
// Sets memberships = (1 << layer), filter = u32::MAX (collides with everything)

pub fn set_collision_mask(&mut self, entity: EntityId, layer: u8, mask: u32)
// Sets memberships = (1 << layer), filter = mask
```

Also expose these to Lua in `ScriptSelf`:
```
self:physics():set_layer(layer_number)
self:physics():set_mask(layer_number, mask_bits)
```

**Verify:** Run `cargo build --workspace`.

---

## Task 9 — debug draw

**File:** `crates/sindri/src/render/wgpu_backend.rs` (via `Renderer`)

Add a method to `Renderer`:

```rust
pub fn draw_physics_debug(
    &mut self,
    frame: &mut Frame,
    world: &World,
    physics: &PhysicsWorld,
    camera: &Camera2D,
) -> Result<()>
```

For each entity with a physics body, draw its colliders as outlines:
- Dynamic bodies: bright cyan `[0.0, 1.0, 0.9, 0.8]`
- Kinematic bodies: bright yellow `[1.0, 0.9, 0.0, 0.8]`
- Fixed bodies: bright green `[0.0, 1.0, 0.3, 0.8]`
- Sensors/triggers: magenta `[1.0, 0.0, 0.8, 0.8]`

Use `draw_polygon` for box shapes (4 corners) and `draw_circle` for circle shapes. For capsules, draw two circles and a rectangle connecting them.

Use `physics.get_colliders(entity)` to get shape data and `physics.body_position(entity)` for position. Transform collider offset by body position to get world position.

This method is debug-only. Gate it behind a feature flag `debug-physics` or simply document that it should only be called during development. Do not call it in release builds.

**Verify:** Run `cargo build --workspace`.

---

## Task 10 — entity builder

**File:** `crates/sindri/src/world.rs` (or a new `crates/sindri/src/builder.rs`)

Add an `EntityBuilder` that reduces the 5-line spawn+physics setup to a fluent chain:

```rust
pub struct EntityBuilder<'a> {
    world: &'a mut World,
    physics: &'a mut PhysicsWorld,
    entity: EntityId,
}

impl<'a> EntityBuilder<'a> {
    pub fn new(world: &'a mut World, physics: &'a mut PhysicsWorld) -> Self
    pub fn at(self, position: Vec2) -> Self                        // inserts Transform
    pub fn with_name(self, name: &str) -> Self                     // stores on a Name component
    pub fn dynamic(self) -> Self                                   // creates Dynamic body
    pub fn kinematic(self) -> Self                                 // creates Kinematic body
    pub fn fixed(self) -> Self                                     // creates Fixed body
    pub fn box_collider(self, hx: f32, hy: f32) -> Self           // adds box collider
    pub fn circle_collider(self, radius: f32) -> Self              // adds circle collider
    pub fn capsule_collider(self, half_height: f32, radius: f32) -> Self
    pub fn friction(self, f: f32) -> Self                          // sets friction on last collider
    pub fn restitution(self, r: f32) -> Self                       // sets restitution on last collider
    pub fn sensor(self) -> Self                                    // makes last collider a sensor
    pub fn lock_rotation(self) -> Self                             // locks rotation
    pub fn sprite(self, texture: TextureHandle) -> Self            // adds SpriteComponent
    pub fn script(self, path: &str, params: ScriptParams) -> Self  // adds ScriptComponent
    pub fn tag(self, tag: &str) -> Self                            // adds ScriptTag
    pub fn build(self) -> EntityId                                 // returns the entity
}
```

Usage should look like:
```rust
let player = EntityBuilder::new(&mut world, &mut physics)
    .at(Vec2::new(200.0, 300.0))
    .dynamic()
    .box_collider(14.0, 18.0)
    .friction(0.3)
    .lock_rotation()
    .sprite(player_texture)
    .script("scripts/player.lua", ScriptParams::default().insert("speed", 200.0f32))
    .tag("player")
    .build();
```

Add `EntityBuilder` to `lib.rs` re-exports.

**Verify:** Run `cargo build --workspace`. Write a short usage example in the doc comment on `EntityBuilder::new`.

---

## Completed tasks

- Task 1 — physics sync helper
- Task 2 — query_mut
- Task 3 — complete parse_key in script.rs
- Task 4 — expose InputMap axes to Lua
- Task 5 — draw_world convenience method
- Task 6 — fix measure_text_width
- Task 7 — colored quad particles
- Task 8 — collision layer/mask support
- Task 9 — debug draw
- Task 10 — entity builder
