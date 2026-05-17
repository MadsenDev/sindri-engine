1. Overall vision

I’d aim for this first:

> A 2D game framework crate (sindri?) that lets you write:

fn main() {
    let game = MyGame::new();
    sindri::Engine::new()
        .with_title("My Cool Game")
        .run(game)
        .unwrap();
}

And MyGame implements a simple Game trait with init, update, and draw.



Not a full engine yet. Just a clean, reusable framework.

Tech stack

Window & events: winit

Rendering: wgpu (modern, cross-platform, 2D via textured quads)

Time: std::time::Instant

Audio: rodio (later phase)

Serialization: serde + ron or serde_json (later phase)

Logging: log + env_logger (optional but nice)



---

2. Folder / crate layout

Monorepo style:

my_engine_project/
  Cargo.toml                # workspace
  sindri/                  # the engine/framework crate
    Cargo.toml
    src/
      lib.rs
      engine.rs
      time.rs
      input.rs
      render/
        mod.rs
        backend_wgpu.rs
        sprite.rs
        camera.rs
      audio.rs
      assets.rs
      math.rs
  examples/
    basic_game/
      Cargo.toml
      src/main.rs

sindri is your public library.

examples/basic_game is a tiny game that proves the engine works.



---

3. Phase 1 – Core engine API + game loop skeleton

Goal

Get a window opening and a loop running with a clean public API, even if nothing renders yet.

What you build

The public entrypoint: Engine and Game trait.

The internal game loop (using winit events).

A basic context passed to the game each frame.


Key types & functions

Engine configuration & entrypoint

// sindri/src/lib.rs
pub mod engine;
pub use engine::{Engine, EngineConfig, Game, EngineContext};

// sindri/src/engine.rs
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
    // ...later: fullscreen, etc.
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Sindri Game".into(),
            width: 1280,
            height: 720,
            vsync: true,
        }
    }
}

pub struct Engine {
    config: EngineConfig,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            config: EngineConfig::default(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    // add with_size, with_vsync, etc.

    pub fn run<G: Game + 'static>(self, mut game: G) -> anyhow::Result<()> {
        // create window
        // init renderer
        // init audio later
        // create event loop & game loop
        // call game.init(&mut ctx)
        // then run loop calling game.update / game.draw
        Ok(())
    }
}

Game trait & context

pub trait Game {
    fn init(&mut self, ctx: &mut EngineContext) -> anyhow::Result<()> {
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext);
    fn draw(&mut self, ctx: &mut EngineContext);
}

pub struct EngineContext<'a> {
    pub time: &'a mut crate::time::Time,
    pub input: &'a crate::input::InputState,
    // later: pub renderer: &'a mut Renderer,
    // later: pub audio: &'a mut AudioSystem,
}

Time module skeleton

// sindri/src/time.rs
use std::time::Instant;

pub struct Time {
    pub delta_seconds: f32,
    pub total_time: f32,
    last_instant: Instant,
}

impl Time {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            delta_seconds: 0.0,
            total_time: 0.0,
            last_instant: now,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_instant);
        self.delta_seconds = dt.as_secs_f32();
        self.total_time += self.delta_seconds;
        self.last_instant = now;
    }
}

What a game looks like now

struct MyGame;

impl sindri::Game for MyGame {
    fn init(&mut self, _ctx: &mut sindri::EngineContext) -> anyhow::Result<()> {
        println!("Game init!");
        Ok(())
    }

    fn update(&mut self, ctx: &mut sindri::EngineContext) {
        println!("dt = {}", ctx.time.delta_seconds);
    }

    fn draw(&mut self, _ctx: &mut sindri::EngineContext) {
        // nothing yet
    }
}

This phase = window + loop + dt logging.


---

4. Phase 2 – Input system

Goal

Give the game a clean way to ask:

“Is key X pressed this frame?”

“Did key Y go down this frame?”

“What’s the mouse position?”


What you build

input.rs module:

Tracks current & previous frame key states.

Stores mouse position/buttons.


Integrate it with winit event handling in the engine loop.


Key types & functions

// sindri/src/input.rs
use winit::event::{ElementState, KeyboardInput, MouseButton, VirtualKeyCode};

pub struct InputState {
    keys_down: [bool; 256],
    keys_pressed: [bool; 256],  // pressed this frame
    keys_released: [bool; 256], // released this frame

    mouse_x: f32,
    mouse_y: f32,
    mouse_down: [bool; 8],
    mouse_pressed: [bool; 8],
    mouse_released: [bool; 8],
}

impl InputState {
    pub fn new() -> Self {
        Self {
            keys_down: [false; 256],
            keys_pressed: [false; 256],
            keys_released: [false; 256],
            mouse_x: 0.0,
            mouse_y: 0.0,
            mouse_down: [false; 8],
            mouse_pressed: [false; 8],
            mouse_released: [false; 8],
        }
    }

    pub fn begin_frame(&mut self) {
        self.keys_pressed.fill(false);
        self.keys_released.fill(false);
        self.mouse_pressed.fill(false);
        self.mouse_released.fill(false);
    }

    pub fn handle_key(&mut self, input: KeyboardInput) {
        if let Some(keycode) = input.virtual_keycode {
            let idx = keycode as usize;
            match input.state {
                ElementState::Pressed => {
                    if !self.keys_down[idx] {
                        self.keys_pressed[idx] = true;
                    }
                    self.keys_down[idx] = true;
                }
                ElementState::Released => {
                    self.keys_down[idx] = false;
                    self.keys_released[idx] = true;
                }
            }
        }
    }

    // similar for handle_mouse_button, handle_cursor_moved ...

    pub fn is_key_down(&self, key: VirtualKeyCode) -> bool {
        self.keys_down[key as usize]
    }

    pub fn is_key_pressed(&self, key: VirtualKeyCode) -> bool {
        self.keys_pressed[key as usize]
    }

    pub fn mouse_position(&self) -> (f32, f32) {
        (self.mouse_x, self.mouse_y)
    }
}

You then wire winit::Event::WindowEvent::KeyboardInput, etc., into InputState in the main loop.

What a game can now do

fn update(&mut self, ctx: &mut EngineContext) {
    use winit::event::VirtualKeyCode as Key;

    if ctx.input.is_key_pressed(Key::Space) {
        println!("Space pressed this frame!");
    }
}


---

5. Phase 3 – Basic 2D rendering with wgpu

Goal

Render a clear color, then a simple rectangle/quad, then texture.

This is the most “heavy” phase technically, but AI can write a lot of boilerplate.

What you build

render::backend_wgpu:

Holds wgpu::Device, Queue, Surface, SwapChain/SurfaceConfiguration, etc.


Simple Renderer API:

begin_frame()

clear(color)

draw_sprite(...)

end_frame()



Key types & functions

// sindri/src/render/mod.rs
pub struct Renderer {
    backend: backend_wgpu::WgpuBackend,
    // later: sprite batches, pipelines, etc.
}

impl Renderer {
    pub async fn new(window: &winit::window::Window) -> anyhow::Result<Self> {
        let backend = backend_wgpu::WgpuBackend::new(window).await?;
        Ok(Self { backend })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.backend.resize(width, height);
    }

    pub fn begin_frame(&mut self) -> anyhow::Result<()> {
        self.backend.begin_frame()
    }

    pub fn clear(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.backend.clear(r, g, b, a);
    }

    pub fn end_frame(&mut self) -> anyhow::Result<()> {
        self.backend.end_frame()
    }

    // later: draw_sprite, draw_text, etc.
}

The WgpuBackend handles swapchain, command encoder, render pass, etc.

EngineContext now also has renderer:

pub struct EngineContext<'a> {
    pub time: &'a mut Time,
    pub input: &'a InputState,
    pub renderer: &'a mut Renderer,
}

What a game can now do

fn draw(&mut self, ctx: &mut EngineContext) {
    ctx.renderer.clear(0.1, 0.1, 0.2, 1.0);
}

At the end of Phase 3, you can clear the screen with a color.


---

6. Phase 4 – Sprites, textures & a basic camera

Goal

Load a texture (PNG).

Define a Sprite type.

Draw sprites at positions using a simple camera.


What you build

render::sprite.rs: Sprite, SpriteBatch, etc.

render::camera.rs: Camera2D.

Simple math helpers: Vec2, Transform2D.


Key types & functions

Math

// sindri/src/math.rs
#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Transform2D {
    pub position: Vec2,
    pub rotation: f32,
    pub scale: Vec2,
}

Sprite

// sindri/src/render/sprite.rs
use crate::math::{Transform2D};
use crate::render::TextureHandle;

pub struct Sprite {
    pub texture: TextureHandle,
    pub transform: Transform2D,
    pub size: (f32, f32),
    // later: color tint, uv rect, etc.
}

Camera

// sindri/src/render/camera.rs
use crate::math::Vec2;

pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
}

impl Camera2D {
    pub fn new() -> Self {
        Self {
            position: Vec2 { x: 0.0, y: 0.0 },
            zoom: 1.0,
        }
    }

    pub fn view_projection_matrix(&self, screen_width: f32, screen_height: f32) -> glam::Mat4 {
        // you can use glam crate for math
        // orthographic projection based on camera position + zoom
        unimplemented!()
    }
}

Renderer API for sprites

In Renderer:

pub fn draw_sprite(&mut self, sprite: &Sprite, camera: &Camera2D) {
    // push instance data into vertex buffer,
    // use camera matrix for transform, etc.
}

What a game can do now

struct MyGame {
    player_sprite: Sprite,
    camera: Camera2D,
}

impl Game for MyGame {
    fn init(&mut self, ctx: &mut EngineContext) -> anyhow::Result<()> {
        let tex = ctx.renderer.load_texture("assets/player.png")?;
        self.player_sprite = Sprite {
            texture: tex,
            transform: Transform2D {
                position: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
                scale: Vec2 { x: 1.0, y: 1.0 },
            },
            size: (64.0, 64.0),
        };
        self.camera = Camera2D::new();
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) {
        self.player_sprite.transform.position.x += 50.0 * ctx.time.delta_seconds;
    }

    fn draw(&mut self, ctx: &mut EngineContext) {
        ctx.renderer.clear(0.1, 0.1, 0.2, 1.0);
        ctx.renderer.draw_sprite(&self.player_sprite, &self.camera);
    }
}

Now you’ve got actual 2D visual stuff moving.


---

7. Phase 5 – Assets & audio (quality-of-life)

Assets

Goal: stop hardcoding texture loading in random places.

Build an AssetManager that:

Caches loaded textures (keyed by path or ID).

Maybe supports async or lazy load later.



// sindri/src/assets.rs
use std::collections::HashMap;
use crate::render::{Renderer, TextureHandle};

pub struct AssetManager {
    textures: HashMap<String, TextureHandle>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self { textures: HashMap::new() }
    }

    pub fn load_texture(
        &mut self,
        renderer: &mut Renderer,
        path: &str,
    ) -> anyhow::Result<TextureHandle> {
        if let Some(tex) = self.textures.get(path) {
            return Ok(*tex);
        }

        let tex = renderer.load_texture(path)?;
        self.textures.insert(path.to_string(), tex);
        Ok(tex)
    }
}

Add assets to EngineContext.

Audio

Goal: Be able to play sound effects and background music.

audio.rs with AudioSystem:

Wraps rodio output stream.

Methods like play_sound("assets/jump.wav"), play_music_loop(...).



Add audio to EngineContext.


---

8. Phase 6 – Simple state/scene system

Goal

Instead of one big monolithic Game, support game states like:

MainMenu

InGame

Paused


You don’t need full ECS yet; just a stack or enum-based state machine.

What you build

State trait:

on_enter, on_exit, update, draw


StateMachine inside Engine that delegates to current state.


Or, keep Game but let it manage its own states.

Example (engine-owned):

pub trait State {
    fn on_enter(&mut self, ctx: &mut EngineContext) {}
    fn on_exit(&mut self, ctx: &mut EngineContext) {}
    fn update(&mut self, ctx: &mut EngineContext, state_machine: &mut StateMachine);
    fn draw(&mut self, ctx: &mut EngineContext);
}

pub struct StateMachine {
    // stack or single state; your call
}

Game now provides an initial state to the engine.


---

9. Optional Phase 7 – Intro ECS

Only once you feel the pain of managing lots of objects manually, you start:

Adding an ECS (hecs or similar).

Defining components (Transform, SpriteComponent, Velocity).

Writing systems for:

Movement

Rendering

Input-driven actions



That’s a whole next chapter, but the earlier phases already give you a usable framework for small games.


---

How to use this with AI

You can literally go phase by phase, e.g.:

> “Using winit and wgpu, implement the Engine::run function for this API: [paste Engine, Game, EngineContext code]. It should create a window, setup wgpu, run a loop, call time.update(), input.begin_frame(), forward winit events to input, and call game.update() then game.draw() each frame.”



Then:

> “Here is my current Renderer struct: [paste]. Implement WgpuBackend to support begin_frame, clear, end_frame.”



And so on.
