use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::Result;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{assets::AssetManager, audio::AudioSystem, input::InputState, render::Renderer};

/// Configuration values for the engine window and runtime behavior.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
    pub asset_root: Option<PathBuf>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            title: "Forge2D Game".into(),
            width: 1280,
            height: 720,
            vsync: true,
            asset_root: None,
        }
    }
}

/// Main entrypoint for running a Forge2D game.
pub struct Engine {
    config: EngineConfig,
}

impl Engine {
    /// Create a new engine instance with default configuration.
    pub fn new() -> Self {
        Self {
            config: EngineConfig::default(),
        }
    }

    /// Override the window title.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = title.into();
        self
    }

    /// Override the initial window size in logical pixels.
    #[must_use]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.config.width = width;
        self.config.height = height;
        self
    }

    /// Enable or disable vertical sync.
    #[must_use]
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.config.vsync = vsync;
        self
    }

    /// Set a base path for asset loading.
    #[must_use]
    pub fn with_asset_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.config.asset_root = Some(root.into());
        self
    }

    /// Run the provided game until the window is closed or the game requests exit.
    pub fn run<G: Game + 'static>(self, mut game: G) -> Result<()> {
        let config = self.config;

        let event_loop = EventLoop::new()?;
        let mut window_attributes = Window::default_attributes();
        window_attributes.title = config.title.clone();
        window_attributes.inner_size = Some(LogicalSize::new(config.width, config.height).into());
        let window = event_loop.create_window(window_attributes)?;

        let mut ctx = EngineContext::new(window, &config)?;
        game.init(&mut ctx)?;

        let mut last_frame = Instant::now();
        event_loop.run(move |event, elwt| {
            match event {
                Event::WindowEvent { event, .. } => {
                    ctx.handle_window_event(&event);

                    match event {
                        WindowEvent::CloseRequested => {
                            elwt.exit();
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if is_escape_pressed(&event) {
                                elwt.exit();
                            }
                        }
                        WindowEvent::Resized(new_size) => {
                            ctx.resize_renderer(new_size);
                        }
                        WindowEvent::ScaleFactorChanged { .. } => {
                            // Note: The actual resize will come through Resized event
                        }
                        WindowEvent::RedrawRequested => {
                            if let Err(err) = game.draw(&mut ctx) {
                                eprintln!("Encountered error during draw: {err:?}");
                                elwt.exit();
                                return;
                            }

                            if ctx.exit_requested {
                                elwt.exit();
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    let now = Instant::now();
                    ctx.update_time(now - last_frame);
                    last_frame = now;
                    ctx.begin_frame();

                    while ctx.should_run_fixed_update() {
                        if let Err(err) = game.fixed_update(&mut ctx) {
                            eprintln!("Encountered error during fixed update: {err:?}");
                            elwt.exit();
                            return;
                        }
                    }

                    if let Err(err) = game.update(&mut ctx) {
                        eprintln!("Encountered error during update: {err:?}");
                        elwt.exit();
                        return;
                    }

                    if ctx.exit_requested {
                        elwt.exit();
                        return;
                    }

                    ctx.window().request_redraw();
                }
                _ => {}
            }
        })?;

        Ok(())
    }
}

fn is_escape_pressed(event: &KeyEvent) -> bool {
    event.state == ElementState::Pressed
        && matches!(
            event.physical_key,
            PhysicalKey::Code(KeyCode::Escape)
        )
}

/// Shared context provided to game code each frame.
pub struct EngineContext {
    // Renderer must be dropped before window to keep the wgpu surface valid.
    renderer: Renderer,
    window: winit::window::Window,
    asset_root: Option<PathBuf>,
    delta_time: Duration,
    elapsed_time: Duration,
    fixed_delta_time: Duration,
    fixed_time_accumulator: Duration,
    exit_requested: bool,
    input: InputState,
    assets: AssetManager,
    audio: AudioSystem,
}

impl EngineContext {
    fn new(window: winit::window::Window, config: &EngineConfig) -> Result<Self> {
        let renderer = Renderer::new(&window, config.vsync)?;
        // Audio initialization is graceful - engine continues even if audio fails
        let audio = AudioSystem::new()?;

        Ok(Self {
            renderer,
            window,
            asset_root: config.asset_root.clone(),
            delta_time: Duration::ZERO,
            elapsed_time: Duration::ZERO,
            fixed_delta_time: Duration::from_secs_f64(1.0 / 60.0), // 60 FPS fixed timestep
            fixed_time_accumulator: Duration::ZERO,
            exit_requested: false,
            input: InputState::new(),
            assets: AssetManager::new(),
            audio,
        })
    }

    fn begin_frame(&mut self) {
        self.input.begin_frame();
    }

    fn update_time(&mut self, delta: Duration) {
        let max_delta = Duration::from_millis(250);
        let clamped = if delta > max_delta { max_delta } else { delta };
        self.delta_time = clamped;
        self.elapsed_time += clamped;
        // Accumulate time for fixed timestep
        self.fixed_time_accumulator += clamped;
    }

    fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => self.input.handle_key(event),
            WindowEvent::MouseInput { state, button, .. } => {
                self.input.handle_mouse_button(*button, *state)
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.handle_cursor_moved(position.x, position.y)
            }
            _ => {}
        }
    }

    fn resize_renderer(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.resize(new_size);
    }

    /// Duration between the current and previous frames.
    pub fn delta_time(&self) -> Duration {
        self.delta_time
    }

    /// Duration between the current and previous frames, in seconds.
    pub fn delta_seconds(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }

    /// Total time elapsed since the engine started running.
    pub fn elapsed_time(&self) -> Duration {
        self.elapsed_time
    }

    /// Fixed timestep duration (typically 1/60 second for 60 FPS).
    pub fn fixed_delta_time(&self) -> Duration {
        self.fixed_delta_time
    }

    /// Fixed timestep duration, in seconds.
    pub fn fixed_delta_seconds(&self) -> f32 {
        self.fixed_delta_time.as_secs_f32()
    }

    /// Check if a fixed timestep update should run and consume accumulated time.
    ///
    /// Returns `true` if enough time has accumulated for a fixed update.
    /// Call this in a loop until it returns `false` to handle multiple fixed updates per frame.
    /// The engine now drives fixed updates automatically via `Game::fixed_update()`,
    /// but this remains available for custom loops or tooling.
    ///
    /// Example:
    /// ```rust,no_run
    /// # use forge2d::EngineContext;
    /// # fn example(mut ctx: &mut EngineContext) {
    /// while ctx.should_run_fixed_update() {
    ///     // Run physics, collision, etc. with fixed timestep
    ///     let fixed_dt = ctx.fixed_delta_time();
    ///     // physics_system.update(fixed_dt);
    /// }
    /// # }
    /// ```
    pub fn should_run_fixed_update(&mut self) -> bool {
        if self.fixed_time_accumulator >= self.fixed_delta_time {
            self.fixed_time_accumulator -= self.fixed_delta_time;
            true
        } else {
            false
        }
    }

    /// Get the interpolation factor for rendering between fixed timestep updates.
    ///
    /// Returns a value between 0.0 and 1.0 indicating how far through the current
    /// fixed timestep interval we are. Useful for smooth interpolation in rendering.
    pub fn fixed_update_alpha(&self) -> f32 {
        if self.fixed_delta_time.as_secs_f32() > 0.0 {
            (self.fixed_time_accumulator.as_secs_f32() / self.fixed_delta_time.as_secs_f32()).min(1.0)
        } else {
            0.0
        }
    }

    /// Access the underlying winit window.
    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    /// Create a camera centered on the current window size.
    ///
    /// This sets the camera position so the top-left of world space is (0, 0)
    /// when your world coordinates are screen-sized.
    pub fn screen_camera(&self) -> crate::math::Camera2D {
        let (width, height) = self.renderer.surface_size();
        crate::math::Camera2D::new(crate::math::Vec2::new(
            width as f32 * 0.5,
            height as f32 * 0.5,
        ))
    }

    /// Access the current input state.
    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// Request that the engine exit after the current frame.
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    /// Access the renderer for drawing operations.
    pub fn renderer(&mut self) -> &mut Renderer {
        &mut self.renderer
    }

    /// Run a render pass with an auto-managed frame.
    pub fn draw<F>(&mut self, draw_fn: F) -> Result<()>
    where
        F: FnOnce(&mut Renderer, &mut crate::render::Frame) -> Result<()>,
    {
        let mut frame = self.renderer.begin_frame()?;
        draw_fn(&mut self.renderer, &mut frame)?;
        self.renderer.end_frame(frame)?;
        Ok(())
    }

    /// Access the asset manager for loading and caching assets.
    pub fn assets(&mut self) -> &mut AssetManager {
        &mut self.assets
    }

    /// Run a scoped asset operation to keep borrows short and predictable.
    pub fn with_assets<R>(&mut self, f: impl FnOnce(&mut AssetManager) -> R) -> R {
        f(&mut self.assets)
    }

    /// Load a texture using the asset manager (convenience method).
    ///
    /// This is equivalent to `ctx.assets().load_texture(ctx.renderer(), path)`
    /// but avoids borrowing issues.
    pub fn load_texture(&mut self, path: &str) -> Result<crate::render::TextureHandle> {
        let resolved = self.resolve_asset_path(path);
        let path = resolved.to_string_lossy().to_string();
        self.assets.load_texture(&mut self.renderer, &path)
    }

    /// Load a texture from bytes using the asset manager (convenience method).
    pub fn load_texture_from_bytes(
        &mut self,
        key: &str,
        bytes: &[u8],
    ) -> Result<crate::render::TextureHandle> {
        self.assets
            .load_texture_from_bytes(&mut self.renderer, key, bytes)
    }

    /// Load a font from bytes using the asset manager (convenience method).
    ///
    /// Fonts are cached by the provided key. Loading the same key again
    /// returns the cached `FontHandle` without re-loading the font data.
    pub fn load_font_from_bytes(
        &mut self,
        key: &str,
        bytes: &[u8],
    ) -> Result<crate::render::FontHandle> {
        self.assets
            .load_font_from_bytes(&mut self.renderer, key, bytes)
    }

    /// Load a font from a file path using the asset manager (convenience method).
    pub fn load_font(&mut self, path: &str) -> Result<crate::render::FontHandle> {
        let resolved = self.resolve_asset_path(path);
        let path = resolved.to_string_lossy().to_string();
        self.assets.load_font_from_file(&mut self.renderer, &path)
    }

    /// Get a cached texture handle by key, if it exists.
    pub fn get_texture(&self, key: &str) -> Option<crate::render::TextureHandle> {
        self.assets.get_texture(key)
    }

    /// Get a cached font handle by key, if it exists.
    pub fn get_font(&self, key: &str) -> Option<crate::render::FontHandle> {
        self.assets.get_font(key)
    }

    /// Load a built-in engine font via the asset system.
    ///
    /// This uses the `BuiltinFont` enum and `AssetManager` under the hood.
    /// Until you configure actual font files in `forge2d::fonts`, this will
    /// return an error which you can gracefully ignore.
    pub fn builtin_font(
        &mut self,
        which: crate::fonts::BuiltinFont,
    ) -> Result<crate::render::FontHandle> {
        which.load(&mut self.assets, &mut self.renderer)
    }

    /// Get mouse position in world coordinates using the current camera.
    ///
    /// This converts screen-space mouse coordinates to world-space coordinates
    /// using the provided camera's view projection.
    pub fn mouse_world(&self, camera: &crate::math::Camera2D) -> crate::math::Vec2 {
        let mouse_screen = self.input.mouse_position_vec2();
        let (screen_w, screen_h) = self.renderer.surface_size();
        camera.screen_to_world(mouse_screen, screen_w, screen_h)
    }

    /// Access the audio system for playing sounds and music.
    pub fn audio(&mut self) -> &mut AudioSystem {
        &mut self.audio
    }

    fn resolve_asset_path(&self, path: &str) -> PathBuf {
        let path = Path::new(path);
        if let Some(root) = &self.asset_root {
            if path.is_relative() {
                return root.join(path);
            }
        }
        path.to_path_buf()
    }
}

/// Trait implemented by user code to hook into the engine lifecycle.
pub trait Game {
    /// Called once after the window is created but before the first frame.
    fn init(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        Ok(())
    }

    /// Update game state using a fixed timestep. Called zero or more times per frame.
    fn fixed_update(&mut self, _ctx: &mut EngineContext) -> Result<()> {
        Ok(())
    }

    /// Update game state. Called once per frame before drawing.
    fn update(&mut self, ctx: &mut EngineContext) -> Result<()>;

    /// Draw the current frame. Called after update when a redraw is requested.
    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()>;
}

/// Adapter to use StateMachine as a Game.
/// This allows StateMachine to be used directly with Engine::run().
impl Game for crate::state::StateMachine {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Call on_enter for the initial state (if any)
        self.init_top_state(ctx)?;
        // Apply any initial state transitions
        self.apply_transitions(ctx)?;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        // Apply pending state transitions first
        self.apply_transitions(ctx)?;

        // Update the top state (if any)
        // This method handles borrow checker issues internally
        self.update_top(ctx)?;

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        ctx.draw(|renderer, frame| {
            // Draw all states from bottom to top (oldest to newest)
            // This allows background states to be visible behind foreground states
            self.draw_all(renderer, frame)
        })?;
        Ok(())
    }
}
