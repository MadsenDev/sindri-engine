# Examples

Code examples and common patterns for Forge2D.

## Basic Game Structure

```rust
use anyhow::Result;
use forge2d::{Engine, EngineContext, Game, KeyCode, Vec2};

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
use forge2d::{CameraFollow, update_camera_follow};

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

- `fireflies_demo` — fixed-step swarm with interpolation, mouse attract/repel, and `ctx.draw()` usage.
- `solar_system_demo` — orbital system with zoom controls, center dragging, and fixed-step interpolation.
- `walljump_demo` — physics-driven platformer with wall jumps, camera follow, and reset key.

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
use forge2d::{Grid, GridCoord, Vec2};

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
use forge2d::{AStarPathfinder, PathfindingGrid, Vec2};

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
use forge2d::{PhysicsWorld, RigidBodyType, ColliderShape, Vec2};

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
use forge2d::{create_scene, restore_scene_physics, Scene};

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
use forge2d::{HudLayer, HudText};

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

## Available Demos

Forge2D includes several complete example demos:

### Basic Game (`examples/basic_game/`)

Complete working example demonstrating:
- Sprite rendering
- Input handling
- Camera following
- Collision detection
- Text rendering
- Asset management

```bash
cargo run -p basic_game
```

### Physics Demo (`examples/physics_demo/`)

Physics engine demonstration:
- Dynamic, kinematic, and fixed bodies
- Collision detection and events
- Sensors and triggers
- Scene save/load functionality

```bash
cargo run -p physics_demo
```

### Platformer Demo (`examples/platformer_demo/`)

2D platformer example:
- Physics-based character controller
- Jumping mechanics
- Camera follow with dead-zone
- Platform navigation

```bash
cargo run -p platformer_demo
```

### Pathfinding Demo (`examples/pathfinding_demo/`)

A* pathfinding visualization:
- Interactive pathfinding
- Obstacle avoidance
- Path visualization
- Agent movement

```bash
cargo run -p pathfinding_demo
```

### Grid Demo (`examples/grid_demo/`)

Grid-based movement demo:
- Discrete grid movement
- A* pathfinding integration
- Grid-snapped movement
- Smooth interpolation

```bash
cargo run -p grid_demo
```

### Performance Demo (`examples/performance_demo/`)

Performance benchmark:
- Large-scale physics simulation
- Performance metrics (FPS, physics time, render time)
- Stress testing
- Real-time statistics

```bash
cargo run -p performance_demo
```
