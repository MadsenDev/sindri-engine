# API Reference

High-level API reference for Sindri's public engine surface.

## Engine

### Engine

```rust
pub struct Engine { /* ... */ }

impl Engine {
    pub fn new() -> Self;
    pub fn with_title(self, title: impl Into<String>) -> Self;
    pub fn with_size(self, width: u32, height: u32) -> Self;
    pub fn with_vsync(self, vsync: bool) -> Self;
    pub fn with_asset_root(self, root: impl Into<PathBuf>) -> Self;
    pub fn run<G: Game>(self, game: G) -> Result<()>;
}
```

### Game Trait

```rust
pub trait Game {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> { Ok(()) }
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()>;
    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()>;
    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> { Ok(()) }
}
```

### EngineContext

```rust
pub struct EngineContext { /* ... */ }

impl EngineContext {
    pub fn delta_time(&self) -> Duration;
    pub fn delta_seconds(&self) -> f32;
    pub fn elapsed_time(&self) -> Duration;
    pub fn fixed_delta_time(&self) -> Duration;
    pub fn fixed_delta_seconds(&self) -> f32;
    pub fn should_run_fixed_update(&mut self) -> bool;
    pub fn fixed_update_alpha(&self) -> f32;
    pub fn input(&self) -> &InputState;
    pub fn renderer(&mut self) -> &mut Renderer;
    pub fn draw<F>(&mut self, draw_fn: F) -> Result<()>;
    pub fn assets(&mut self) -> &mut AssetManager;
    pub fn audio(&mut self) -> &mut AudioSystem;
    pub fn window(&self) -> &Window;
    pub fn screen_camera(&self) -> Camera2D;
    pub fn mouse_world(&self, camera: &Camera2D) -> Vec2;
    pub fn load_texture(&mut self, path: &str) -> Result<TextureHandle>;
    pub fn load_texture_from_bytes(&mut self, id: &str, bytes: &[u8]) -> Result<TextureHandle>;
    pub fn load_font(&mut self, path: &str) -> Result<FontHandle>;
    pub fn load_font_from_bytes(&mut self, id: &str, bytes: &[u8]) -> Result<FontHandle>;
    pub fn get_texture(&self, id: &str) -> Option<TextureHandle>;
    pub fn builtin_font(&mut self, font: BuiltinFont) -> Result<FontHandle>;
    pub fn request_exit(&mut self);
}
```

## Input

### InputState

```rust
pub struct InputState { /* ... */ }

impl InputState {
    pub fn is_key_down(&self, key: KeyCode) -> bool;
    pub fn is_key_pressed(&self, key: KeyCode) -> bool;
    pub fn is_key_released(&self, key: KeyCode) -> bool;
    pub fn is_mouse_down(&self, button: MouseButton) -> bool;
    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool;
    pub fn is_mouse_released(&self, button: MouseButton) -> bool;
    pub fn mouse_position(&self) -> (f32, f32);
    pub fn mouse_position_vec2(&self) -> Vec2;
}
```

## Rendering

### Renderer

```rust
pub struct Renderer { /* ... */ }

impl Renderer {
    pub fn begin_frame(&mut self) -> Result<Frame>;
    pub fn clear(&mut self, frame: &mut Frame, color: [f32; 4]) -> Result<()>;
    pub fn draw_sprite(&mut self, frame: &mut Frame, sprite: &Sprite, camera: &Camera2D) -> Result<()>;
    pub fn draw_text(&mut self, frame: &mut Frame, text: &str, font: FontHandle, size: f32, position: Vec2, color: [f32; 4], camera: &Camera2D) -> Result<()>;
    pub fn measure_text_width(&mut self, text: &str, font: FontHandle, size: f32) -> Result<f32>;
    pub fn draw_circle(&mut self, frame: &mut Frame, center: Vec2, radius: f32, color: [f32; 4], camera: &Camera2D) -> Result<()>;
    pub fn draw_polygon(&mut self, frame: &mut Frame, points: &[Vec2], color: [f32; 4], camera: &Camera2D) -> Result<()>;
    pub fn draw_polygon_no_occlusion(&mut self, frame: &mut Frame, points: &[Vec2], color: [f32; 4], camera: &Camera2D) -> Result<()>;
    pub fn draw_point_light(&mut self, frame: &mut Frame, light: &PointLight, camera: &Camera2D) -> Result<()>;
    pub fn draw_world(&mut self, frame: &mut Frame, world: &World, camera: &Camera2D) -> Result<()>;
    pub fn draw_physics_debug(&mut self, frame: &mut Frame, world: &World, physics: &PhysicsWorld, camera: &Camera2D) -> Result<()>;
    pub fn load_texture_from_file(&mut self, path: &str) -> Result<TextureHandle>;
    pub fn load_texture_from_bytes(&mut self, bytes: &[u8]) -> Result<TextureHandle>;
    pub fn load_font_from_bytes(&mut self, bytes: &[u8]) -> Result<FontHandle>;
    pub fn rasterize_text_glyphs(&mut self, text: &str, font: FontHandle, size: f32) -> Result<()>;
    pub fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)>;
    pub fn surface_size(&self) -> (u32, u32);
    pub fn end_frame(&mut self, frame: Frame) -> Result<()>;
}
```

### Sprite

```rust
pub struct Sprite {
    pub texture: TextureHandle,
    pub transform: Transform2D,
    pub tint: [f32; 4],
    pub is_occluder: bool,
}

impl Sprite {
    pub fn new(texture: TextureHandle) -> Self;
    pub fn set_size_px(&mut self, size_px: Vec2, texture_px: Vec2);
}
```

### Camera2D

```rust
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    pub rotation: f32,
    pub offset: Vec2,
    pub target_zoom: f32,
    pub zoom_speed: f32,
    pub shake_intensity: f32,
    pub shake_timer: f32,
    pub bounds: Option<(Vec2, Vec2)>,
}

impl Camera2D {
    pub fn new(position: Vec2) -> Self;
    pub fn default() -> Self;
    pub fn with_rotation(self, rotation: f32) -> Self;
    pub fn with_offset(self, offset: Vec2) -> Self;
    pub fn with_bounds(self, min: Vec2, max: Vec2) -> Self;
    pub fn without_bounds(self) -> Self;
    
    pub fn update(&mut self, dt: f32);
    pub fn shake(&mut self, intensity: f32, duration: f32);
    pub fn zoom_to(&mut self, target_zoom: f32, speed: f32);
    pub fn zoom_to_point(&mut self, world_point: Vec2, target_zoom: f32, speed: f32, width: u32, height: u32);
    
    pub fn screen_to_world(&self, screen: Vec2, width: u32, height: u32) -> Vec2;
    pub fn world_to_screen(&self, world: Vec2, width: u32, height: u32) -> Vec2;
    pub fn view_projection(&self, width: u32, height: u32) -> Mat4;
    pub fn viewport_bounds(&self, width: u32, height: u32) -> (Vec2, Vec2);
    pub fn is_point_visible(&self, point: Vec2, width: u32, height: u32) -> bool;
    pub fn is_rect_visible(&self, min: Vec2, max: Vec2, width: u32, height: u32) -> bool;
    pub fn is_circle_visible(&self, center: Vec2, radius: f32, width: u32, height: u32) -> bool;
}
```

## Particle System

### ParticleSystem

```rust
pub struct ParticleSystem { /* ... */ }

impl ParticleSystem {
    pub fn new() -> Self;
    pub fn add_emitter(&mut self, emitter: ParticleEmitter);
    pub fn update(&mut self, dt: f32);
    pub fn emitters(&self) -> &[ParticleEmitter];
    pub fn emitters_mut(&mut self) -> &mut [ParticleEmitter];
    pub fn clear(&mut self);
}
```

### ParticleEmitter

```rust
pub struct ParticleEmitter { /* ... */ }

impl ParticleEmitter {
    pub fn new(config: EmissionConfig) -> Self;
    pub fn with_max_particles(self, max: usize) -> Self;
    pub fn with_texture(self, texture: Option<TextureHandle>) -> Self;
    pub fn update(&mut self, dt: f32);
    pub fn set_position(&mut self, position: Vec2);
    pub fn position(&self) -> Vec2;
    pub fn stop_emission(&mut self);
    pub fn is_emitting(&self) -> bool;
    pub fn is_active(&self) -> bool;
    pub fn particles(&self) -> &[Particle];
}
```

### EmissionConfig

```rust
pub struct EmissionConfig {
    pub particles_per_second: f32,
    pub burst_count: usize,
    pub position: Vec2,
    /* ... other fields ... */
}

impl EmissionConfig {
    pub fn new(position: Vec2) -> Self;
    pub fn with_rate(self, rate: f32) -> Self;
    pub fn with_burst(self, count: usize) -> Self;
    pub fn with_velocity(self, min: Vec2, max: Vec2) -> Self;
    pub fn with_size(self, min: Vec2, max: Vec2) -> Self;
    pub fn with_color(self, start: [f32; 4], end: Option<[f32; 4]>) -> Self;
    pub fn with_lifetime(self, min: f32, max: f32) -> Self;
    pub fn with_acceleration(self, acc: Vec2) -> Self;
    pub fn with_size_end_multiplier(self, mult: f32) -> Self;
    pub fn with_fade_out(self, fade: bool) -> Self;
}
```

### Particle

```rust
pub struct Particle {
    pub position: Vec2,
    pub velocity: Vec2,
    pub color: [f32; 4],
    pub size: Vec2,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub rotation: f32,
}
```

## Animation

### Animation

```rust
pub struct Animation {
    pub frames: Vec<AnimationFrame>,
    pub looping: bool,
    pub total_duration: f32,
}

impl Animation {
    pub fn new(frames: Vec<AnimationFrame>, looping: bool) -> Self;
    pub fn from_grid(texture: TextureHandle, grid_size: (u32, u32), frame_count: usize, frame_duration: f32) -> Self;
}
```

### AnimationFrame

```rust
pub struct AnimationFrame {
    pub texture: TextureHandle,
    pub source_rect: Option<[f32; 4]>,
    pub duration: f32,
}
```

### AnimatedSprite

```rust
pub struct AnimatedSprite {
    pub animation: Animation,
    pub current_frame_index: usize,
    pub playing: bool,
    pub speed: f32,
    pub transform: Transform2D,
    pub tint: [f32; 4],
    pub is_occluder: bool,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl AnimatedSprite {
    pub fn new(animation: Animation) -> Self;
    pub fn update(&mut self, dt: f32);
    pub fn current_frame(&self) -> Option<&AnimationFrame>;
    pub fn reset(&mut self);
}
```


## Math

### Vec2

```rust
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2;
    pub const ONE: Vec2;
    
    pub fn new(x: f32, y: f32) -> Self;
    pub fn length(&self) -> f32;
    pub fn length_squared(&self) -> f32;
    pub fn normalized(&self) -> Vec2;
    pub fn distance(&self, other: Vec2) -> f32;
    pub fn distance_squared(&self, other: Vec2) -> f32;
    pub fn dot(&self, other: Vec2) -> f32;
    pub fn lerp(&self, other: Vec2, t: f32) -> Vec2;
    pub fn from_angle(angle: f32) -> Vec2;
    pub fn abs(&self) -> Vec2;
    pub fn min(&self, other: Vec2) -> Vec2;
    pub fn max(&self, other: Vec2) -> Vec2;
}

impl Add<Vec2> for Vec2 { /* ... */ }
impl Sub<Vec2> for Vec2 { /* ... */ }
impl Mul<f32> for Vec2 { /* ... */ }
impl Div<f32> for Vec2 { /* ... */ }
impl Neg for Vec2 { /* ... */ }
```

### Transform2D

```rust
pub struct Transform2D {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
}

impl Transform2D {
    pub fn to_matrix(&self, base_size: Vec2) -> Mat4;
}
```

## Assets

### AssetManager

```rust
pub struct AssetManager { /* ... */ }

impl AssetManager {
    pub fn new() -> Self;
    pub fn get_texture(&self, id: &str) -> Option<TextureHandle>;
    pub fn get_font(&self, id: &str) -> Option<FontHandle>;
    pub fn load_texture(&mut self, renderer: &mut Renderer, path: &str) -> Result<TextureHandle>;
    pub fn load_texture_from_bytes(&mut self, renderer: &mut Renderer, id: &str, bytes: &[u8]) -> Result<TextureHandle>;
    pub fn load_font_from_bytes(&mut self, renderer: &mut Renderer, id: &str, bytes: &[u8]) -> Result<FontHandle>;
    pub fn load_font_from_file(&mut self, renderer: &mut Renderer, path: &str) -> Result<FontHandle>;
}
```

## Audio

### AudioSystem

```rust
pub struct AudioSystem { /* ... */ }

impl AudioSystem {
    pub fn new() -> Self;
    pub fn is_available(&self) -> bool;
    pub fn play_sound<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    pub fn play_sound_from_bytes(&self, bytes: &[u8]) -> Result<()>;
    pub fn load_sound<P: AsRef<Path>>(&mut self, path: P) -> Result<SoundHandle>;
    pub fn load_sound_from_bytes(&mut self, key: &str, bytes: &[u8]) -> Result<SoundHandle>;
    pub fn play_sound_handle(&self, handle: SoundHandle) -> Result<()>;
    pub fn play_music_loop<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    pub fn play_music_loop_from_bytes(&self, bytes: &[u8]) -> Result<()>;
    pub fn stop_music(&self);
    pub fn set_music_volume(&self, volume: f32);
    pub fn is_music_playing(&self) -> bool;
}
```

## Types

### TextureHandle

```rust
pub struct TextureHandle(pub(crate) u32);
```

### FontHandle

```rust
pub struct FontHandle(pub(crate) u32);
```

### SoundHandle

```rust
pub struct SoundHandle(pub(crate) u32);
```

### Frame

```rust
pub struct Frame { /* ... */ }
```

## State Management

### State Trait

```rust
pub trait State {
    fn on_enter(&mut self, ctx: &mut EngineContext) -> Result<()>;
    fn on_exit(&mut self, ctx: &mut EngineContext) -> Result<()>;
    fn update(&mut self, ctx: &mut EngineContext, state_machine: &mut dyn StateMachineLike) -> Result<()>;
    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()>;
}
```

### StateMachine

```rust
pub struct StateMachine { /* ... */ }

impl StateMachine {
    pub fn new() -> Self;
    pub fn with_initial_state(initial: Box<dyn State>) -> Self;
    pub fn push(&mut self, state: Box<dyn State>);
    pub fn pop(&mut self);
    pub fn replace(&mut self, state: Box<dyn State>);
    pub fn is_empty(&self) -> bool;
    pub fn len(&self) -> usize;
    pub fn apply_transitions(&mut self, ctx: &mut EngineContext) -> Result<()>;
    pub fn update_top(&mut self, ctx: &mut EngineContext) -> Result<()>;
    pub fn draw_all(&mut self, ctx: &mut EngineContext) -> Result<()>;
    pub fn states(&self) -> &VecDeque<Box<dyn State>>;
}

impl Game for StateMachine { /* ... */ }
```

### StateMachineLike Trait

```rust
pub trait StateMachineLike {
    fn push(&mut self, state: Box<dyn State>);
    fn pop(&mut self);
    fn replace(&mut self, state: Box<dyn State>);
}
```

## Physics

### PhysicsWorld

```rust
pub struct PhysicsWorld { /* ... */ }

impl PhysicsWorld {
    pub fn new() -> Self;
    pub fn with_gravity(gravity: Vec2) -> Self;
    pub fn set_gravity(&mut self, gravity: Vec2);
    pub fn gravity(&self) -> Vec2;
    pub fn clear(&mut self);
    pub fn create_body(&mut self, entity: EntityId, body_type: RigidBodyType, position: Vec2, rotation: f32) -> Result<()>;
    pub fn remove_body(&mut self, entity: EntityId);
    pub fn add_collider_with_material(&mut self, entity: EntityId, shape: ColliderShape, offset: Vec2, density: f32, friction: f32, restitution: f32) -> Result<()>;
    pub fn add_sensor(&mut self, entity: EntityId, shape: ColliderShape, offset: Vec2) -> Result<()>;
    pub fn step(&mut self, dt: f32);
    pub fn body_position(&self, entity: EntityId) -> Option<Vec2>;
    pub fn body_rotation(&self, entity: EntityId) -> Option<f32>;
    pub fn linear_velocity(&self, entity: EntityId) -> Option<Vec2>;
    pub fn set_linear_velocity(&mut self, entity: EntityId, vel: Vec2);
    pub fn apply_impulse(&mut self, entity: EntityId, impulse: Vec2);
    pub fn apply_force(&mut self, entity: EntityId, force: Vec2);
    pub fn lock_rotations(&mut self, entity: EntityId, locked: bool);
    pub fn set_linear_damping(&mut self, entity: EntityId, d: f32);
    pub fn sync_transforms(&self, world: &mut World);
    pub fn set_collision_groups(&mut self, entity: EntityId, memberships: u32, filter: u32);
    pub fn set_collision_layer(&mut self, entity: EntityId, layer: u8);
    pub fn set_collision_mask(&mut self, entity: EntityId, layer: u8, mask: u32);
    pub fn on_event<F>(&mut self, callback: F) where F: Fn(PhysicsEvent) + Send + Sync + 'static;
}
```

### ColliderShape

```rust
pub enum ColliderShape {
    Box { hx: f32, hy: f32 },
    Circle { radius: f32 },
    CapsuleY { half_height: f32, radius: f32 },
}
```

### RigidBodyType

```rust
pub enum RigidBodyType {
    Dynamic,
    Kinematic,
    Fixed,
}
```

### PhysicsEvent

```rust
pub enum PhysicsEvent {
    CollisionEnter { a: EntityId, b: EntityId },
    CollisionExit { a: EntityId, b: EntityId },
    TriggerEnter { a: EntityId, b: EntityId },
    TriggerExit { a: EntityId, b: EntityId },
}
```

## Grid System

### Grid

```rust
pub struct Grid<T> { /* ... */ }

impl<T: Clone> Grid<T> {
    pub fn new(width: usize, height: usize, cell_size: f32, default: T) -> Self;
    pub fn width(&self) -> usize;
    pub fn height(&self) -> usize;
    pub fn cell_size(&self) -> f32;
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridCoord;
    pub fn grid_to_world(&self, coord: GridCoord) -> Vec2;
    pub fn grid_to_world_top_left(&self, coord: GridCoord) -> Vec2;
    pub fn is_valid(&self, coord: &GridCoord) -> bool;
    pub fn get(&self, coord: GridCoord) -> Option<&T>;
    pub fn get_mut(&mut self, coord: GridCoord) -> Option<&mut T>;
    pub fn set(&mut self, coord: GridCoord, value: T) -> bool;
    pub fn neighbors_4(&self, coord: &GridCoord) -> Vec<GridCoord>;
    pub fn neighbors_8(&self, coord: &GridCoord) -> Vec<GridCoord>;
    pub fn iter_coords(&self) -> impl Iterator<Item = GridCoord>;
    pub fn iter(&self) -> impl Iterator<Item = (GridCoord, &T)>;
}
```

### GridCoord

```rust
pub struct GridCoord {
    pub x: i32,
    pub y: i32,
}

impl GridCoord {
    pub fn new(x: i32, y: i32) -> Self;
    pub fn manhattan_distance(&self, other: &GridCoord) -> i32;
    pub fn distance(&self, other: &GridCoord) -> f32;
}
```

### GridPathfinding

```rust
pub trait GridPathfinding {
    fn is_walkable(&self, coord: &GridCoord) -> bool;
}
```

## Pathfinding

### AStarPathfinder

```rust
pub struct AStarPathfinder;

impl AStarPathfinder {
    pub fn find_path(grid: &PathfindingGrid, start_world: Vec2, goal_world: Vec2) -> Option<Vec<Vec2>>;
    pub fn find_path_grid(grid: &PathfindingGrid, start: GridNode, goal: GridNode) -> Option<Vec<GridNode>>;
}
```

### PathfindingGrid

```rust
pub struct PathfindingGrid { /* ... */ }

impl PathfindingGrid {
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self;
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridNode;
    pub fn grid_to_world(&self, node: GridNode) -> Vec2;
    pub fn is_valid(&self, node: &GridNode) -> bool;
    pub fn is_walkable(&self, node: &GridNode) -> bool;
    pub fn set_walkable(&mut self, node: GridNode, walkable: bool);
    pub fn set_area_walkable(&mut self, x: i32, y: i32, width: i32, height: i32, walkable: bool);
    pub fn get_neighbors(&self, node: &GridNode) -> Vec<GridNode>;
}
```

### GridNode

```rust
pub struct GridNode {
    pub x: i32,
    pub y: i32,
}

impl GridNode {
    pub fn new(x: i32, y: i32) -> Self;
    pub fn distance_to(&self, other: &GridNode) -> f32;
    pub fn manhattan_distance(&self, other: &GridNode) -> i32;
}
```

## Camera Follow

### CameraFollow

```rust
pub struct CameraFollow {
    pub target_entity: Option<EntityId>,
    pub target_position: Option<Vec2>,
    pub dead_zone: Vec2,
    pub max_speed: f32,
    pub smooth: bool,
    pub smooth_factor: f32,
}

impl CameraFollow {
    pub fn new() -> Self;
    pub fn follow_entity(self, entity: EntityId) -> Self;
    pub fn follow_position(self, position: Vec2) -> Self;
    pub fn with_dead_zone(self, width: f32, height: f32) -> Self;
    pub fn with_smoothing(self, factor: f32) -> Self;
    pub fn with_max_speed(self, speed: f32) -> Self;
}
```

### update_camera_follow

```rust
pub fn update_camera_follow(
    camera: &mut Camera2D,
    follow: &CameraFollow,
    physics: &PhysicsWorld,
    dt: f32,
);
```

## Built-in Entities

### Transform

```rust
pub struct Transform {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

impl Transform {
    pub fn new(position: Vec2) -> Self;
    pub fn with_rotation(self, rotation: f32) -> Self;
    pub fn with_scale(self, scale: Vec2) -> Self;
}
```

### SpriteComponent

```rust
pub struct SpriteComponent {
    pub texture: TextureHandle,
    pub sprite: Sprite,
    pub visible: bool,
}

impl SpriteComponent {
    pub fn new(texture: TextureHandle) -> Self;
    pub fn with_tint(self, r: f32, g: f32, b: f32, a: f32) -> Self;
}
```

### PhysicsBody

```rust
pub struct PhysicsBody {
    pub body_type: RigidBodyType,
    pub collider_shape: Option<ColliderShape>,
}

impl PhysicsBody {
    pub fn new(body_type: RigidBodyType) -> Self;
    pub fn with_collider(self, shape: ColliderShape) -> Self;
}
```

### Name

```rust
pub struct Name(pub String);
```

### Tag Components

```rust
pub struct Player;
pub struct Enemy;
pub struct Collectible { pub value: i32 }
pub struct Hazard { pub damage: i32 }
pub struct Checkpoint { pub checkpoint_id: u32 }
pub struct Trigger { pub trigger_id: u32, pub activated: bool }
pub struct MovingPlatform { pub start_pos: Vec2, pub end_pos: Vec2, pub speed: f32, /* ... */ }
pub struct AudioSource { pub volume: f32, pub pitch: f32, pub looping: bool, /* ... */ }
pub struct CameraComponent { pub camera: Camera2D, pub active: bool }
```

## Scene Serialization

### Scene

```rust
pub struct Scene {
    pub version: u32,
    pub entities: Vec<SerializableEntity>,
    pub physics: SerializablePhysics,
}

impl Scene {
    pub fn new() -> Self;
}
```

### create_scene

```rust
pub fn create_scene(world: &World, physics: &PhysicsWorld) -> Result<Scene>;
```

### restore_scene_physics

```rust
pub fn restore_scene_physics(physics: &mut PhysicsWorld, data: &SerializablePhysics) -> Result<()>;
```

### ComponentSerializable

```rust
pub trait ComponentSerializable {
    fn serialize(&self) -> SerializableComponent;
    fn deserialize(data: &serde_json::Value) -> Result<Self>;
}
```

## HUD

### HudLayer

```rust
pub struct HudLayer { /* ... */ }

impl HudLayer {
    pub fn new() -> Self;
    pub fn clear(&mut self);
    pub fn add_text(&mut self, text: HudText);
    pub fn add_sprite(&mut self, sprite: HudSprite);
    pub fn add_rect(&mut self, rect: HudRect);
    pub fn draw(&mut self, renderer: &mut Renderer, frame: &mut Frame) -> Result<()>;
}
```

### HudText

```rust
pub struct HudText {
    pub text: String,
    pub font: FontHandle,
    pub size: f32,
    pub position: Vec2,  // Screen-space pixels (0,0 = top-left)
    pub color: [f32; 4],
}
```

### HudSprite

```rust
pub struct HudSprite {
    pub sprite: Sprite,
    pub position: Vec2,  // Screen-space pixels
}
```

### HudRect

```rust
pub struct HudRect {
    pub position: Vec2,  // Top-left in screen-space pixels
    pub size: Vec2,      // Width/height in pixels
    pub color: [f32; 4],
}
```

## World & Entities

### World

```rust
pub struct World { /* ... */ }

impl World {
    pub fn new() -> Self;
    pub fn spawn(&mut self) -> EntityId;
    pub fn despawn(&mut self, entity: EntityId);
    pub fn is_alive(&self, entity: EntityId) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T);
    pub fn get<T: 'static>(&self, entity: EntityId) -> Option<&T>;
    pub fn get_mut<T: 'static>(&mut self, entity: EntityId) -> Option<&mut T>;
    pub fn remove<T: 'static>(&mut self, entity: EntityId) -> Option<T>;
    pub fn query<T: 'static>(&self) -> Vec<(EntityId, &T)>;
    pub fn query_mut<T: 'static>(&mut self) -> Vec<(EntityId, &mut T)>;
    pub fn serialize_component<T: ComponentSerializable>(&self, entity: EntityId) -> Option<SerializableComponent>;
    pub fn deserialize_component<T: ComponentSerializable>(&mut self, entity: EntityId, data: &SerializableComponent) -> Result<()>;
}
```

### EntityBuilder

```rust
pub struct EntityBuilder<'a> { /* ... */ }

impl<'a> EntityBuilder<'a> {
    pub fn new(world: &'a mut World, physics: &'a mut PhysicsWorld) -> Self;
    pub fn at(self, position: Vec2) -> Self;
    pub fn dynamic(self) -> Self;
    pub fn kinematic(self) -> Self;
    pub fn fixed(self) -> Self;
    pub fn box_collider(self, hx: f32, hy: f32) -> Self;
    pub fn circle_collider(self, radius: f32) -> Self;
    pub fn capsule_collider(self, half_height: f32, radius: f32) -> Self;
    pub fn build(self) -> EntityId;
}
```

### EntityId

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(u32);

impl EntityId {
    pub fn to_u32(self) -> u32;
}
```

## Re-exports

Sindri re-exports the following from `winit`:

- `KeyCode` - Keyboard key codes
- `MouseButton` - Mouse button types
