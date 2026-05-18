# Examples

Code examples and common patterns for Sindri.

## Basic Game Structure

```rust
use anyhow::Result;
use sindri::{Engine, EngineContext, Game, KeyCode, Vec2};

struct MyGame {
    // Your game state
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Initialize game
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Update game logic
        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Render game
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("My Game")
        .with_size(1280, 720)
        .run(MyGame {})
}
```

## Common Patterns

### Player Movement

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();
    let input = ctx.input();
    
    let mut move_dir = Vec2::ZERO;
    
    if input.is_key_down(KeyCode::KeyW) {
        move_dir.y -= 1.0;
    }
    if input.is_key_down(KeyCode::KeyS) {
        move_dir.y += 1.0;
    }
    if input.is_key_down(KeyCode::KeyA) {
        move_dir.x -= 1.0;
    }
    if input.is_key_down(KeyCode::KeyD) {
        move_dir.x += 1.0;
    }
    
    if move_dir.length_squared() > 0.0 {
        move_dir = move_dir.normalized();
        self.player.position += move_dir * self.speed * dt;
    }
    
    Ok(())
}
```

### Camera Following (Manual)

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let (screen_w, screen_h) = ctx.renderer().surface_size();
    
    // Target camera position (center on player)
    let target_pos = Vec2::new(
        self.player.position.x - (screen_w as f32 * 0.5),
        self.player.position.y - (screen_h as f32 * 0.5),
    );
    
    // Smooth camera following
    let camera_speed = 5.0;
    let dt = ctx.delta_time().as_secs_f32();
    self.camera.position = self.camera.position.lerp(target_pos, camera_speed * dt);
    
    Ok(())
}
```

### Camera Following (CameraFollow System)

```rust
use sindri::{CameraFollow, update_camera_follow};

fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();
    
    // Configure camera follow with dead-zone
    self.camera_follow = CameraFollow::new()
        .follow_entity(self.player_entity)
        .with_dead_zone(200.0, 150.0)  // Dead zone size
        .with_smoothing(0.15);          // Smooth following
    
    // Update camera
    update_camera_follow(&mut self.camera, &self.camera_follow, &self.physics, dt);
    
    Ok(())
}
```

### Click to Spawn

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    if ctx.input().is_mouse_pressed(MouseButton::Left) {
        let mouse_world = ctx.mouse_world(&self.camera);
        self.spawn_entity_at(mouse_world);
    }
    Ok(())
}
```

## Highlighted Examples

- `hello_sindri` — tiny onboarding example for booting the engine, drawing a sprite, and reading input.
- `platformer` — Rust-first gameplay baseline with physics, camera follow, collisions, particles, and game loop structure.
- `scripted_asteroids` — flagship Lua scripting example with `ScriptRuntime`, `ScriptComponent`, `ScriptParams`, hot reload, and world-driven gameplay.
- `editor_scene` — scene/editor workflow example that loads a `.f2scene`, attaches Lua scripts, and renders entities from `World`.
- `rendering` — curated rendering showcase for particles, lighting, camera effects, animation-like motion, and large object counts.

### Collision Detection

```rust
fn check_collisions(&mut self) {
    for i in 0..self.entities.len() {
        for j in (i + 1)..self.entities.len() {
            let pos1 = self.entities[i].position;
            let pos2 = self.entities[j].position;
            let radius1 = self.entities[i].radius;
            let radius2 = self.entities[j].radius;
            
            let distance = pos1.distance(pos2);
            if distance < radius1 + radius2 {
                // Collision!
                self.handle_collision(i, j);
            }
        }
    }
}
```

### Sprite Rotation

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let dt = ctx.delta_time().as_secs_f32();
    
    for sprite in &mut self.sprites {
        sprite.transform.rotation += self.rotation_speed * dt;
    }
    
    Ok(())
}
```

### Score Display

```rust
fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
    let renderer = ctx.renderer();
    let mut frame = renderer.begin_frame()?;
    renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;
    
    // Draw game sprites...
    
    // Draw score text
    if let Some(font) = self.font {
        let score_text = format!("Score: {}", self.score);
        
        // Re-rasterize if score changed
        renderer.rasterize_text_glyphs(&score_text, font, 24.0)?;
        
        // Position in top-left (screen space)
        let (screen_w, screen_h) = renderer.surface_size();
        let text_pos = Vec2::new(
            self.camera.position.x - (screen_w as f32 * 0.5) + 20.0,
            self.camera.position.y + (screen_h as f32 * 0.5) - 40.0,
        );
        
        renderer.draw_text(
            &mut frame,
            &score_text,
            font,
            24.0,
            text_pos,
            [1.0, 1.0, 1.0, 1.0],
            &self.camera,
        )?;
    }
    
    renderer.end_frame(frame)?;
    Ok(())
}
```

### Bounds Clamping

```rust
fn clamp_to_bounds(&mut self, entity: &mut Entity, bounds: Vec2) {
    let half_size = entity.size * 0.5;
    
    entity.position.x = entity.position.x.clamp(
        half_size.x,
        bounds.x - half_size.x,
    );
    entity.position.y = entity.position.y.clamp(
        half_size.y,
        bounds.y - half_size.y,
    );
}
```

### Fixed Timestep Physics

```rust
impl Game for MyGame {
    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let fixed_dt = ctx.fixed_delta_time().as_secs_f32();
        
        // Update physics
        self.velocity += self.acceleration * fixed_dt;
        self.position += self.velocity * fixed_dt;
        
        // Apply friction
        self.velocity *= 0.95;
        
        // Check collisions
        self.check_collisions();
        
        Ok(())
    }
    
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Interpolate visual position
        let alpha = ctx.fixed_update_alpha();
        self.visual_position = self.last_position.lerp(self.position, alpha);
        Ok(())
    }
}
```

### Audio on Event

```rust
fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
    if ctx.input().is_key_pressed(KeyCode::Space) {
        // Play jump sound
        if ctx.audio().is_available() {
            let jump_sound = include_bytes!("assets/jump.wav");
            ctx.audio().play_sound_from_bytes(jump_sound)?;
        }
    }
    Ok(())
}
```

### Grid-Based Movement

```rust
use sindri::{Grid, GridCoord, Vec2};

struct GridGame {
    grid: Grid<bool>,  // true = walkable
    player_pos: GridCoord,
}

impl GridGame {
    fn move_player(&mut self, direction: Vec2) {
        let new_coord = GridCoord::new(
            self.player_pos.x + direction.x as i32,
            self.player_pos.y + direction.y as i32,
        );
        
        if let Some(&walkable) = self.grid.get(new_coord) {
            if walkable {
                self.player_pos = new_coord;
            }
        }
    }
}
```

### A* Pathfinding

```rust
use sindri::{AStarPathfinder, PathfindingGrid, Vec2};

fn find_path_to_target(
    grid: &PathfindingGrid,
    start: Vec2,
    goal: Vec2,
) -> Option<Vec<Vec2>> {
    AStarPathfinder::find_path(grid, start, goal)
}
```

### Physics Bodies

```rust
use sindri::{PhysicsWorld, RigidBodyType, ColliderShape, Vec2};

fn spawn_physics_object(
    physics: &mut PhysicsWorld,
    entity: EntityId,
    pos: Vec2,
) -> Result<()> {
    // Create dynamic body
    physics.create_body(entity, RigidBodyType::Dynamic, pos, 0.0)?;
    
    // Add box collider
    physics.add_collider_with_material(
        entity,
        ColliderShape::Box { hx: 15.0, hy: 15.0 },
        Vec2::ZERO,
        1.0,  // density
        0.5,  // friction
        0.3,  // restitution
    )?;
    
    Ok(())
}
```

### Scene Save/Load

```rust
use sindri::{create_scene, restore_scene_physics, Scene};

fn save_game(world: &World, physics: &PhysicsWorld) -> Result<()> {
    let scene = create_scene(world, physics)?;
    let json = serde_json::to_string_pretty(&scene)?;
    std::fs::write("save.json", json)?;
    Ok(())
}

fn load_game(physics: &mut PhysicsWorld) -> Result<()> {
    let json = std::fs::read_to_string("save.json")?;
    let scene: Scene = serde_json::from_str(&json)?;
    restore_scene_physics(physics, &scene.physics)?;
    Ok(())
}
```

### HUD Display

```rust
use sindri::{HudLayer, HudText};

fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
    // ... draw game sprites ...
    
    // Draw HUD
    self.hud.clear();
    if let Some(font) = self.font {
        self.hud.add_text(HudText {
            text: format!("Score: {}", self.score),
            font,
            size: 20.0,
            position: Vec2::new(10.0, 10.0),  // Top-left
            color: [1.0, 1.0, 1.0, 1.0],
        });
    }
    self.hud.draw(renderer, &mut frame)?;
    
    Ok(())
}
```

## Curated Examples

Sindri keeps a small public example suite focused on engine identity rather than one demo per subsystem.

### Hello Sindri (`examples/hello_sindri/`)

Tiny first-run example:
- Engine startup
- Sprite rendering
- Keyboard input
- Simple camera usage

```bash
cargo run -p hello_sindri
```

### Platformer (`examples/platformer/`)

Rust-first gameplay baseline:
- Rapier physics
- Camera follow
- Collision checks
- Collectibles, hazards, reset flow
- Lightweight particles and HUD text

```bash
cargo run -p platformer
```

### Scripted Asteroids (`examples/scripted_asteroids/`)

Primary scripting showcase:
- `ScriptRuntime` wiring
- `ScriptComponent` and `ScriptParams`
- Lua-controlled player, bullet, and asteroid behavior
- Hot reload
- World/component-driven gameplay

```bash
cargo run -p scripted_asteroids
```

### Editor Scene (`examples/editor_scene/`)

Scene and editor workflow reference:
- Loads a `.f2scene`
- Restores entities into `World`
- Attaches Lua scripts from scene data
- Renders scene entities with hot reload enabled

```bash
cargo run -p editor_scene
```

### Rendering (`examples/rendering/`)

Rendering and polish showcase:
- Particles
- Point lighting
- Camera zoom
- Large animated object counts
- Occluders and shape rendering

```bash
cargo run -p rendering
```
