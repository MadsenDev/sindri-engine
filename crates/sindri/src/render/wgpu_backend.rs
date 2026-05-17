use std::{collections::HashMap, fs};

use anyhow::{anyhow, Result};
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use wgpu::{
    vertex_attr_array, AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
    BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites,
    CommandEncoder, CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Extent3d,
    FilterMode, FragmentState, Instance, LoadOp, MapMode, MultisampleState, Operations, Origin3d,
    PipelineLayoutDescriptor, PollType, PresentMode, PrimitiveState, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderModuleDescriptor, ShaderSource,
    SurfaceConfiguration, TexelCopyBufferInfo, TexelCopyBufferLayout, TexelCopyTextureInfo,
    Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexState,
};
use winit::{dpi::PhysicalSize, window::Window};

use crate::{
    math::{Camera2D, Transform2D, Vec2},
    render::light::PointLight,
    render::particles::ParticleSystem,
    render::sprite::{Sprite, TextureHandle},
    render::text::{FontHandle, TextRenderer},
};
use glam::{Mat4, Vec3};
use glyphon::{
    Attrs, Buffer as GlyphonBuffer, Cache, Color, Family, Metrics, Shaping, TextArea, TextAtlas,
    TextRenderer as GlyphonTextRenderer, Viewport,
};

/// Queued sprite draw command (batched rendering)
struct SpriteDrawCommand {
    uniform_offset: u64,
    texture_handle: TextureHandle, // Store texture handle, look up bind group when flushing
}

/// Wrapper around wgpu surface/device setup and simple frame management.
pub struct Renderer {
    backend: WgpuBackend,
}

impl Renderer {
    pub fn new(window: &Window, vsync: bool) -> Result<Self> {
        let backend = WgpuBackend::new(window, vsync)?;
        Ok(Self { backend })
    }

    pub fn new_offscreen(width: u32, height: u32) -> Result<Self> {
        let backend = WgpuBackend::new_offscreen(width, height)?;
        Ok(Self { backend })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.backend.resize(new_size);
    }

    pub fn begin_frame(&mut self) -> Result<Frame> {
        self.backend.begin_frame()
    }

    pub fn clear(&mut self, frame: &mut Frame, color: [f32; 4]) -> Result<()> {
        self.backend.clear(frame, color)
    }

    pub fn draw_sprite(
        &mut self,
        frame: &mut Frame,
        sprite: &Sprite,
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend.draw_sprite(frame, sprite, camera)
    }

    /// Draw a section of a texture (useful for spritesheets).
    /// * `uv_rect`: Normalized UV coordinates [x, y, w, h].
    ///   - x, y: Top-left corner (0.0 to 1.0)
    ///   - w, h: Width and Height (0.0 to 1.0)
    ///   - If None, renders the full texture.
    pub fn draw_texture_region(
        &mut self,
        frame: &mut Frame,
        texture: TextureHandle,
        uv_rect: Option<[f32; 4]>,
        transform: &crate::math::Transform2D,
        tint: [f32; 4],
        is_occluder: bool,
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend.draw_texture_region(
            frame,
            texture,
            uv_rect,
            transform,
            tint,
            is_occluder,
            camera,
        )
    }

    /// Draw a tilemap efficiently (batched rendering).
    pub fn draw_tilemap(
        &mut self,
        frame: &mut Frame,
        tilemap: &crate::render::Tilemap,
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend.draw_tilemap(frame, tilemap, camera)
    }

    pub fn end_frame(&mut self, frame: Frame) -> Result<()> {
        self.backend.end_frame(frame)
    }

    pub fn render_offscreen_rgba<F>(
        &mut self,
        width: u32,
        height: u32,
        draw_fn: F,
    ) -> Result<Vec<u8>>
    where
        F: FnOnce(&mut Renderer, &mut Frame) -> Result<()>,
    {
        let (current_w, current_h) = self.surface_size();
        if current_w != width || current_h != height {
            self.resize(PhysicalSize::new(width, height));
        }
        let mut frame = self.begin_frame()?;
        draw_fn(self, &mut frame)?;
        self.backend.end_frame_readback(frame)
    }

    pub fn load_texture_from_file(&mut self, path: &str) -> Result<TextureHandle> {
        self.backend.load_texture_from_file(path)
    }

    pub fn load_texture_from_bytes(&mut self, bytes: &[u8]) -> Result<TextureHandle> {
        self.backend.load_texture_from_bytes(bytes)
    }

    /// Load a texture from raw RGBA8 data (no PNG decoding).
    ///
    /// This is useful for procedurally generated textures or tests.
    /// `data` must be `width * height * 4` bytes in RGBA8 format.
    pub fn load_texture_from_rgba(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<TextureHandle> {
        // Default to false (sprite texture) for backward compatibility
        self.backend
            .load_texture_from_rgba(data, width, height, false)
    }

    pub fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)> {
        self.backend.texture_size(handle)
    }

    pub fn surface_size(&self) -> (u32, u32) {
        self.backend.surface_size()
    }

    /// Load a font from bytes (TTF/OTF format).
    pub fn load_font_from_bytes(&mut self, bytes: &[u8]) -> Result<FontHandle> {
        self.backend.load_font_from_bytes(bytes)
    }

    /// Rasterize all glyphs needed for a text string.
    /// This is optional when using the default text backend.
    pub fn rasterize_text_glyphs(&mut self, text: &str, font: FontHandle, size: f32) -> Result<()> {
        self.backend.ensure_glyphs_rasterized(text, font, size)
    }

    /// Draw text at the specified position in world coordinates.
    ///
    /// # Arguments
    /// * `frame` - The current frame being rendered
    /// * `text` - The text string to render
    /// * `font` - The font handle to use
    /// * `size` - Font size in pixels
    /// * `position` - World position (bottom-left of first character)
    /// * `color` - RGBA color tint
    /// * `camera` - Camera for view projection
    ///
    /// # Note
    /// Glyph caching is handled internally, so pre-rasterization is optional.
    pub fn draw_text(
        &mut self,
        frame: &mut Frame,
        text: &str,
        font: FontHandle,
        size: f32,
        position: Vec2,
        color: [f32; 4],
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend
            .draw_text(frame, text, font, size, position, color, camera)
    }

    /// Measure the width of text without drawing it.
    /// This is useful for accurate text alignment in HUD elements.
    pub fn measure_text_width(&mut self, text: &str, font: FontHandle, size: f32) -> Result<f32> {
        self.backend.measure_text_width(text, font, size)
    }

    /// Draw a filled polygon from a list of points.
    /// Points should be in world coordinates and will be transformed by the camera.
    pub fn draw_polygon(
        &mut self,
        frame: &mut Frame,
        points: &[Vec2],
        color: [f32; 4],
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend
            .draw_polygon(frame, points, color, camera, true)
    }

    /// Draw a filled polygon that does not occlude light.
    pub fn draw_polygon_no_occlusion(
        &mut self,
        frame: &mut Frame,
        points: &[Vec2],
        color: [f32; 4],
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend
            .draw_polygon(frame, points, color, camera, false)
    }

    /// Draw a filled circle.
    /// Center and radius are in world coordinates.
    pub fn draw_circle(
        &mut self,
        frame: &mut Frame,
        center: Vec2,
        radius: f32,
        color: [f32; 4],
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend
            .draw_circle(frame, center, radius, color, camera)
    }

    /// Draw a point light (emits light in all directions from a position).
    /// Lights are rendered with additive blending after sprites.
    pub fn draw_point_light(
        &mut self,
        frame: &mut Frame,
        light: &PointLight,
        camera: &Camera2D,
    ) -> Result<()> {
        self.backend.draw_point_light(frame, light, camera)
    }

    /// Draw every visible entity in `world` using the given `camera`.
    ///
    /// Drawing order (stable, ascending entity ID within each pass):
    /// 1. [`TilemapComponent`] entities — drawn first so sprites appear on top.
    /// 2. [`SpriteComponent`] entities where `visible == true`.
    /// 3. [`AnimatedSprite`] entities (current animation frame).
    ///
    /// This eliminates the manual render loop that every game otherwise writes
    /// in its `draw()` callback.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn draw(&mut self, ctx: &mut EngineContext, frame: &mut Frame) -> Result<()> {
    ///     ctx.renderer.clear(frame, [0.05, 0.05, 0.1, 1.0])?;
    ///     ctx.renderer.draw_world(frame, &ctx.world, &ctx.camera)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn draw_world(
        &mut self,
        frame: &mut Frame,
        world: &crate::world::World,
        camera: &Camera2D,
    ) -> Result<()> {
        use crate::entities::{SpriteComponent, TilemapComponent};
        use crate::render::animation::AnimatedSprite;

        // --- Pass 1: tilemaps ---
        let mut tilemaps: Vec<_> = world.query::<TilemapComponent>().into_iter().collect();
        tilemaps.sort_by_key(|(id, _)| id.0);
        for (_, tc) in &tilemaps {
            self.draw_tilemap(frame, &tc.tilemap, camera)?;
        }

        // --- Pass 2: static sprites ---
        let mut sprites: Vec<_> = world
            .query::<SpriteComponent>()
            .into_iter()
            .filter(|(_, sc)| sc.visible)
            .collect();
        sprites.sort_by_key(|(id, _)| id.0);
        for (_, sc) in &sprites {
            self.draw_sprite(frame, &sc.sprite, camera)?;
        }

        // --- Pass 3: animated sprites ---
        let mut animated: Vec<_> = world.query::<AnimatedSprite>().into_iter().collect();
        animated.sort_by_key(|(id, _)| id.0);
        for (_, anim) in &animated {
            if let Some(af) = anim.current_frame() {
                self.draw_texture_region(
                    frame,
                    af.texture,
                    af.source_rect,
                    &anim.transform,
                    anim.tint,
                    anim.is_occluder,
                    camera,
                )?;
            }
        }

        Ok(())
    }

    /// Draw physics collider debug overlays for every entity with a physics body.
    ///
    /// This is intended for development builds and editor diagnostics. Call it
    /// from game/editor debug rendering paths only; it is not called by the
    /// engine automatically.
    pub fn draw_physics_debug(
        &mut self,
        frame: &mut Frame,
        world: &crate::world::World,
        physics: &crate::physics::PhysicsWorld,
        camera: &Camera2D,
    ) -> Result<()> {
        use crate::physics::{ColliderShape, RigidBodyType};

        let _ = world;

        let mut entities = physics.all_entities_with_bodies();
        entities.sort_by_key(|entity| entity.0);

        for entity in entities {
            let Some(body_pos) = physics.body_position(entity) else {
                continue;
            };
            let body_rot = physics.body_rotation(entity).unwrap_or(0.0);
            let body_color = match physics.body_type(entity) {
                Some(RigidBodyType::Dynamic) => [0.0, 1.0, 0.9, 0.8],
                Some(RigidBodyType::Kinematic) => [1.0, 0.9, 0.0, 0.8],
                Some(RigidBodyType::Fixed) => [0.0, 1.0, 0.3, 0.8],
                None => continue,
            };

            for (shape, offset, _, _, _, is_sensor) in physics.get_colliders(entity) {
                let color = if is_sensor {
                    [1.0, 0.0, 0.8, 0.8]
                } else {
                    body_color
                };

                match shape {
                    ColliderShape::Box { hx, hy } => {
                        let corners = [
                            Vec2::new(-hx, -hy),
                            Vec2::new(hx, -hy),
                            Vec2::new(hx, hy),
                            Vec2::new(-hx, hy),
                        ];
                        let points: Vec<_> = corners
                            .into_iter()
                            .map(|point| body_pos + rotate_vec2(offset + point, body_rot))
                            .collect();
                        self.draw_polygon_no_occlusion(frame, &points, color, camera)?;
                    }
                    ColliderShape::Circle { radius } => {
                        let center = body_pos + rotate_vec2(offset, body_rot);
                        self.draw_circle(frame, center, radius, color, camera)?;
                    }
                    ColliderShape::CapsuleY {
                        half_height,
                        radius,
                    } => {
                        let top =
                            body_pos + rotate_vec2(offset + Vec2::new(0.0, half_height), body_rot);
                        let bottom =
                            body_pos + rotate_vec2(offset + Vec2::new(0.0, -half_height), body_rot);

                        self.draw_circle(frame, top, radius, color, camera)?;
                        self.draw_circle(frame, bottom, radius, color, camera)?;

                        if half_height > 0.0 && radius > 0.0 {
                            let corners = [
                                Vec2::new(-radius, -half_height),
                                Vec2::new(radius, -half_height),
                                Vec2::new(radius, half_height),
                                Vec2::new(-radius, half_height),
                            ];
                            let points: Vec<_> = corners
                                .into_iter()
                                .map(|point| body_pos + rotate_vec2(offset + point, body_rot))
                                .collect();
                            self.draw_polygon_no_occlusion(frame, &points, color, camera)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Draw all particles from a particle system.
    /// Particles are rendered as sprites, so they can use textures or be simple colored quads.
    pub fn draw_particles(
        &mut self,
        frame: &mut Frame,
        particle_system: &ParticleSystem,
        camera: &Camera2D,
        default_texture: Option<TextureHandle>,
    ) -> Result<()> {
        self.backend
            .draw_particles(frame, particle_system, camera, default_texture)
    }
}

pub struct Frame {
    surface_texture: Option<wgpu::SurfaceTexture>,
    output_texture: Option<Texture>,
    view: TextureView,
    encoder: Option<CommandEncoder>,
    sprite_draws: Vec<SpriteDrawCommand>, // Queue of sprite draws for batching
    light_draws: Vec<LightDrawCommand>,   // Queue of light draws for batching
    // Render targets for lighting
    scene_texture: Option<Texture>,
    scene_texture_view: Option<TextureView>,
    occlusion_texture: Option<Texture>, // New occlusion target
    occlusion_texture_view: Option<TextureView>,
    light_map_texture: Option<Texture>,
    light_map_texture_view: Option<TextureView>,
    scene_cleared: bool, // Track if scene texture has been cleared this frame
}

impl Drop for Frame {
    fn drop(&mut self) {
        // If frame wasn't properly ended, we still need to present the surface texture
        // to avoid leaking resources. The encoder will be dropped automatically.
        if let Some(surface_texture) = self.surface_texture.take() {
            surface_texture.present();
        }
    }
}

fn rotate_vec2(v: Vec2, angle: f32) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos)
}

struct TextureEntry {
    /// The underlying GPU texture. Must be kept alive for the view/sampler to be valid.
    #[allow(dead_code)]
    texture: Texture,
    view: TextureView,
    sampler: Sampler,
    size: (u32, u32),
}

struct SpritePipeline {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    uniform_buffer: Buffer,
    bind_group_layout: BindGroupLayout,
    #[allow(dead_code)]
    uniform_buffer_size: u64,
    uniform_alignment: u64,
}

// Maximum number of sprites we can draw per frame
// Increased to 2048 sprites (512KB buffer) for better performance with large scenes
const MAX_SPRITES_PER_FRAME: usize = 2048;
const UNIFORM_BUFFER_SIZE: u64 = MAX_SPRITES_PER_FRAME as u64 * 512; // Increased for larger uniform struct

struct WgpuBackend {
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: SurfaceConfiguration,
    present_mode: PresentMode,
    sprite_pipeline: SpritePipeline,
    shape_pipeline: ShapePipeline,
    light_pipeline: LightPipeline,
    composite_pipeline: CompositePipeline,
    circle_texture: Option<TextureHandle>,
    circle_texture_size: u32,
    textures: HashMap<TextureHandle, TextureEntry>,
    light_uniform_write_offset: u64,
    next_texture_id: u32,
    uniform_write_offset: u64, // Current offset for writing uniforms
    bind_group_cache: HashMap<(TextureHandle, u64), wgpu::BindGroup>, // Cache bind groups per (texture, offset)
    text_renderer: TextRenderer,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteUniforms {
    mvp: [[f32; 4]; 4],
    color: [f32; 4],
    uv_offset: [f32; 2],
    uv_scale: [f32; 2],
    is_occluder: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShapeUniforms {
    mvp: [[f32; 4]; 4],
    color: [f32; 4],
    is_occluder: f32, // Added
    _pad: [f32; 3],   // 4 + 12 = 16 bytes alignment
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ShapeVertex {
    position: [f32; 2],
}

struct ShapePipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    uniform_buffer: Buffer,
    #[allow(dead_code)]
    uniform_alignment: u64,
}

#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct LightUniforms {
    position: [f32; 2],
    _pad1: [f32; 2], // Padding to align color to 16 bytes
    color: [f32; 3],
    intensity: f32,
    radius: f32,
    falloff: f32,
    direction: [f32; 2], // Spotlight direction (normalized), or [0,0] for point light
    angle: f32,          // Spotlight angle (cos of half-angle), or 0 for point light
    _pad2: f32,          // Padding to align screen_size to 8 bytes
    screen_size: [f32; 2], // Screen size for shadow mapping
    // No padding needed here: 56 + 8 = 64 bytes, which is 16-byte aligned
    view_proj: [[f32; 4]; 4], // View-projection matrix for shadow mapping
    mvp: [[f32; 4]; 4],
}

struct LightPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    uniform_buffer: Buffer,
    uniform_alignment: u64,
    vertex_buffer: Buffer,
}

struct CompositePipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    vertex_buffer: Buffer,
}

/// Queued light draw command
struct LightDrawCommand {
    uniform_offset: u64,
}

const SPRITE_VERTICES: [SpriteVertex; 6] = [
    SpriteVertex {
        position: [-0.5, -0.5],
        uv: [0.0, 0.0], // Top-left (WebGPU convention)
    },
    SpriteVertex {
        position: [0.5, -0.5],
        uv: [1.0, 0.0], // Top-right
    },
    SpriteVertex {
        position: [0.5, 0.5],
        uv: [1.0, 1.0], // Bottom-right
    },
    SpriteVertex {
        position: [-0.5, -0.5],
        uv: [0.0, 0.0], // Top-left
    },
    SpriteVertex {
        position: [0.5, 0.5],
        uv: [1.0, 1.0], // Bottom-right
    },
    SpriteVertex {
        position: [-0.5, 0.5],
        uv: [0.0, 1.0], // Bottom-left
    },
];

impl WgpuBackend {
    fn new(window: &Window, vsync: bool) -> Result<Self> {
        let instance = Instance::default();
        let surface = instance.create_surface(window)?;
        let surface = unsafe {
            // Safety: the window is owned by EngineContext and outlives the renderer.
            // Renderer is dropped before the window (field order), so the surface
            // never outlives its window.
            std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface)
        };

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))?;

        let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
            label: Some("sindri-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: Default::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        }))?;

        let size = window.inner_size();
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);

        let present_mode = choose_present_mode(&capabilities.present_modes, vsync);
        let alpha_mode = choose_alpha_mode(&capabilities.alpha_modes);

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let sprite_pipeline = create_sprite_pipeline(&device, format);
        let shape_pipeline = create_shape_pipeline(&device, format);
        let light_pipeline = create_light_pipeline(&device, format);
        let composite_pipeline = create_composite_pipeline(&device, format);
        Ok(Self {
            surface: Some(surface),
            device,
            queue,
            surface_config,
            present_mode,
            sprite_pipeline,
            shape_pipeline,
            light_pipeline,
            composite_pipeline,
            circle_texture: None,
            circle_texture_size: 64,
            textures: HashMap::new(),
            next_texture_id: 1,
            uniform_write_offset: 0,
            light_uniform_write_offset: 0,
            bind_group_cache: HashMap::new(),
            text_renderer: TextRenderer::new(),
        })
    }

    fn new_offscreen(width: u32, height: u32) -> Result<Self> {
        let instance = Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))?;

        let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
            label: Some("sindri-offscreen-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: Default::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        }))?;

        let format = TextureFormat::Rgba8UnormSrgb;
        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let sprite_pipeline = create_sprite_pipeline(&device, format);
        let shape_pipeline = create_shape_pipeline(&device, format);
        let light_pipeline = create_light_pipeline(&device, format);
        let composite_pipeline = create_composite_pipeline(&device, format);

        Ok(Self {
            surface: None,
            device,
            queue,
            surface_config,
            present_mode: PresentMode::Fifo,
            sprite_pipeline,
            shape_pipeline,
            light_pipeline,
            composite_pipeline,
            circle_texture: None,
            circle_texture_size: 64,
            textures: HashMap::new(),
            next_texture_id: 1,
            uniform_write_offset: 0,
            light_uniform_write_offset: 0,
            bind_group_cache: HashMap::new(),
            text_renderer: TextRenderer::new(),
        })
    }

    fn ensure_text_components_initialized(&mut self) -> Result<()> {
        // Initialize glyphon components if not already initialized
        if self.text_renderer.text_atlas_mut().is_none() {
            let (_width, _height) = (self.surface_config.width, self.surface_config.height);

            // Initialize GPU Cache first (needed for TextAtlas)
            let gpu_cache = Cache::new(&self.device);

            // Initialize TextAtlas - try the API based on compiler errors
            let mut text_atlas = TextAtlas::new(
                &self.device,
                &self.queue,
                &gpu_cache,
                self.surface_config.format,
            );

            // Initialize GlyphonTextRenderer
            let text_renderer = GlyphonTextRenderer::new(
                &mut text_atlas,
                &self.device,
                wgpu::MultisampleState::default(),
                None,
            );

            // Initialize Viewport - API: new(device, cache)
            let viewport = Viewport::new(&self.device, &gpu_cache);

            *self.text_renderer.text_atlas_mut() = Some(text_atlas);
            *self.text_renderer.text_renderer_mut() = Some(text_renderer);
            *self.text_renderer.viewport_mut() = Some(viewport);
            *self.text_renderer.gpu_cache_mut() = Some(gpu_cache);
        }
        Ok(())
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface_config.present_mode = self.present_mode;
        if let Some(surface) = self.surface.as_ref() {
            surface.configure(&self.device, &self.surface_config);
        }
    }

    fn begin_frame(&mut self) -> Result<Frame> {
        // Reset uniform buffer offset at the start of each frame
        self.uniform_write_offset = 0;
        self.light_uniform_write_offset = 0;
        // Clear bind group cache each frame (they're frame-specific)
        self.bind_group_cache.clear();

        if let Some(surface) = self.surface.as_ref() {
            loop {
                match surface.get_current_texture() {
                    Ok(surface_texture) => {
                        let view = surface_texture
                            .texture
                            .create_view(&TextureViewDescriptor::default());
                        let encoder =
                            self.device
                                .create_command_encoder(&CommandEncoderDescriptor {
                                    label: Some("frame-encoder"),
                                });

                        // Create render target textures for scene and light map
                        let (width, height) =
                            (self.surface_config.width, self.surface_config.height);
                        let format = self.surface_config.format;
                        let scene_texture = self.device.create_texture(&TextureDescriptor {
                            label: Some("scene-texture"),
                            size: Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        });
                        let scene_texture_view =
                            scene_texture.create_view(&TextureViewDescriptor::default());

                        let light_map_texture = self.device.create_texture(&TextureDescriptor {
                            label: Some("light-map-texture"),
                            size: Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format,
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        });
                        let light_map_texture_view =
                            light_map_texture.create_view(&TextureViewDescriptor::default());

                        // Create occlusion texture (R8)
                        let occlusion_texture = self.device.create_texture(&TextureDescriptor {
                            label: Some("occlusion-texture"),
                            size: Extent3d {
                                width,
                                height,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format: TextureFormat::R8Unorm, // Single channel for occlusion mask
                            usage: TextureUsages::RENDER_ATTACHMENT
                                | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        });
                        let occlusion_texture_view =
                            occlusion_texture.create_view(&TextureViewDescriptor::default());

                        return Ok(Frame {
                            surface_texture: Some(surface_texture),
                            output_texture: None,
                            view,
                            encoder: Some(encoder),
                            sprite_draws: Vec::new(),
                            light_draws: Vec::new(),
                            scene_texture: Some(scene_texture),
                            scene_texture_view: Some(scene_texture_view),
                            occlusion_texture: Some(occlusion_texture),
                            occlusion_texture_view: Some(occlusion_texture_view),
                            light_map_texture: Some(light_map_texture),
                            light_map_texture_view: Some(light_map_texture_view),
                            scene_cleared: false,
                        });
                    }
                    Err(e) => match e {
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                            surface.configure(&self.device, &self.surface_config);
                            continue;
                        }
                        wgpu::SurfaceError::Timeout => {
                            continue;
                        }
                        wgpu::SurfaceError::OutOfMemory => {
                            return Err(anyhow!("Surface ran out of memory"));
                        }
                        wgpu::SurfaceError::Other => {
                            return Err(anyhow!("Surface error: Other"));
                        }
                    },
                }
            }
        }

        let (width, height) = (self.surface_config.width, self.surface_config.height);
        let format = self.surface_config.format;
        let output_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("offscreen-output"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = output_texture.create_view(&TextureViewDescriptor::default());
        let encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("offscreen-encoder"),
            });

        let scene_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("scene-texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let scene_texture_view = scene_texture.create_view(&TextureViewDescriptor::default());

        let light_map_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("light-map-texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let light_map_texture_view =
            light_map_texture.create_view(&TextureViewDescriptor::default());

        let occlusion_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("occlusion-texture"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let occlusion_texture_view =
            occlusion_texture.create_view(&TextureViewDescriptor::default());

        Ok(Frame {
            surface_texture: None,
            output_texture: Some(output_texture),
            view,
            encoder: Some(encoder),
            sprite_draws: Vec::new(),
            light_draws: Vec::new(),
            scene_texture: Some(scene_texture),
            scene_texture_view: Some(scene_texture_view),
            occlusion_texture: Some(occlusion_texture),
            occlusion_texture_view: Some(occlusion_texture_view),
            light_map_texture: Some(light_map_texture),
            light_map_texture_view: Some(light_map_texture_view),
            scene_cleared: false,
        })
    }

    fn clear(&mut self, frame: &mut Frame, color: [f32; 4]) -> Result<()> {
        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        {
            let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color {
                            r: color[0] as f64,
                            g: color[1] as f64,
                            b: color[2] as f64,
                            a: color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                multiview_mask: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            drop(_pass);
        }

        Ok(())
    }

    fn draw_sprite(&mut self, frame: &mut Frame, sprite: &Sprite, camera: &Camera2D) -> Result<()> {
        self.draw_texture_region(
            frame,
            sprite.texture,
            None,
            &sprite.transform,
            sprite.tint,
            sprite.is_occluder,
            camera,
        )
    }

    /// Internal method to draw a texture region (or full texture)
    fn draw_texture_region(
        &mut self,
        frame: &mut Frame,
        texture_handle: TextureHandle,
        uv_rect: Option<[f32; 4]>, // x, y, w, h (normalized)
        transform: &Transform2D,
        tint: [f32; 4],
        is_occluder: bool,
        camera: &Camera2D,
    ) -> Result<()> {
        let texture = self
            .textures
            .get(&texture_handle)
            .ok_or_else(|| anyhow!("Unknown texture handle"))?;

        // Check if we've exceeded the maximum sprites per frame
        if self.uniform_write_offset >= UNIFORM_BUFFER_SIZE {
            return Err(anyhow!(
                "Too many sprites drawn in one frame (max: {})",
                MAX_SPRITES_PER_FRAME
            ));
        }

        let base_size = Vec2::new(texture.size.0 as f32, texture.size.1 as f32);
        let model = transform.to_matrix(base_size);
        let vp = camera.view_projection(self.surface_config.width, self.surface_config.height);
        let mvp = vp * model;

        let (uv_offset, uv_scale) = if let Some(rect) = uv_rect {
            ([rect[0], rect[1]], [rect[2], rect[3]])
        } else {
            ([0.0, 0.0], [1.0, 1.0])
        };

        let uniforms = SpriteUniforms {
            mvp: mvp.to_cols_array_2d(),
            color: tint,
            uv_offset,
            uv_scale,
            is_occluder: if is_occluder { 1.0 } else { 0.0 },
            _pad: [0.0; 3],
        };

        // Write uniforms at the current offset (aligned to required alignment)
        let aligned_offset = if self.uniform_write_offset == 0 {
            0
        } else {
            (self.uniform_write_offset + self.sprite_pipeline.uniform_alignment - 1)
                & !(self.sprite_pipeline.uniform_alignment - 1)
        };

        self.queue.write_buffer(
            &self.sprite_pipeline.uniform_buffer,
            aligned_offset,
            bytemuck::bytes_of(&uniforms),
        );

        // Get or create bind group for this texture (cache per texture)
        // We ensure it exists here, then look it up again when flushing
        let cache_key = (texture_handle, 0);
        let uniform_size = std::mem::size_of::<SpriteUniforms>() as u64;
        let _bind_group = self.bind_group_cache.entry(cache_key).or_insert_with(|| {
            self.device.create_bind_group(&BindGroupDescriptor {
                label: Some("sprite-bind-group"),
                layout: &self.sprite_pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.sprite_pipeline.uniform_buffer,
                            offset: 0,
                            size: std::num::NonZeroU64::new(uniform_size),
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&texture.view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&texture.sampler),
                    },
                ],
            })
        });

        // Queue the sprite draw instead of executing immediately
        frame.sprite_draws.push(SpriteDrawCommand {
            uniform_offset: aligned_offset,
            texture_handle: texture_handle,
        });

        // Advance offset for next sprite
        self.uniform_write_offset = aligned_offset + self.sprite_pipeline.uniform_alignment;

        Ok(())
    }

    /// Draw a tilemap efficiently (batched rendering with viewport culling).
    fn draw_tilemap(
        &mut self,
        frame: &mut Frame,
        tilemap: &crate::render::Tilemap,
        camera: &Camera2D,
    ) -> Result<()> {
        use crate::math::Transform2D;
        let (map_width, map_height) = tilemap.map_size;

        // Calculate visible tile bounds using camera viewport
        let (screen_w, screen_h) = (
            self.surface_config.width as f32,
            self.surface_config.height as f32,
        );
        let half_screen = Vec2::new(screen_w * 0.5, screen_h * 0.5);
        let camera_scale = 1.0 / camera.zoom;
        let visible_size = Vec2::new(half_screen.x * camera_scale, half_screen.y * camera_scale);

        // World-space bounds of visible area
        let min_world = camera.position - visible_size;
        let max_world = camera.position + visible_size;

        // Convert to tile coordinates (with padding for safety)
        let (min_tile_x, min_tile_y) = tilemap.world_to_tile(min_world);
        let (max_tile_x, max_tile_y) = tilemap.world_to_tile(max_world);

        // Clamp to map bounds
        let start_x = (min_tile_x - 1).max(0) as u32;
        let end_x = ((max_tile_x + 1).min(map_width as i32 - 1).max(0)) as u32;
        let start_y = (min_tile_y - 1).max(0) as u32;
        let end_y = ((max_tile_y + 1).min(map_height as i32 - 1).max(0)) as u32;

        // Only iterate over visible tiles
        for y in start_y..=end_y.min(map_height - 1) {
            for x in start_x..=end_x.min(map_width - 1) {
                let tile = tilemap.tiles[(y * map_width + x) as usize];
                if tile.is_empty() {
                    continue;
                }

                // Get UV rect for this tile
                if let Some(uv_rect) = tilemap.tile_uv_rect(tile.id) {
                    // Calculate world position (center of tile)
                    let world_pos = tilemap.tile_to_world(x, y);

                    // Create transform for this tile
                    let transform = Transform2D {
                        position: world_pos,
                        rotation: 0.0,
                        scale: tilemap.tile_size,
                    };

                    // Draw the tile using texture region
                    self.draw_texture_region(
                        frame,
                        tilemap.tileset,
                        Some(uv_rect),
                        &transform,
                        tilemap.tint,
                        true, // Tiles are occluders
                        camera,
                    )?;
                }
            }
        }

        Ok(())
    }

    /// Clear and prepare the scene texture (called at start of end_frame)
    fn clear_scene_texture(&mut self, frame: &mut Frame) -> Result<()> {
        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        let scene_view = frame
            .scene_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Scene texture view not available"))?;

        let occlusion_view = frame
            .occlusion_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Occlusion texture view not available"))?;

        // Clear the scene texture (this happens before any drawing)
        let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("clear-scene-pass"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: scene_view,
                    resolve_target: None,
                    ops: Operations {
                        // Clear to transparent so the light shader can use alpha for occlusion.
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
                Some(RenderPassColorAttachment {
                    view: occlusion_view,
                    resolve_target: None,
                    ops: Operations {
                        // Clear occlusion mask (0.0 = no occlusion)
                        load: LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
            ],
            depth_stencil_attachment: None,
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        // Pass is dropped here, clear is recorded

        Ok(())
    }

    /// Flush all queued sprite draws to the scene texture (called by end_frame)
    fn flush_sprites(&mut self, frame: &mut Frame) -> Result<()> {
        if frame.sprite_draws.is_empty() {
            return Ok(());
        }

        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        // Render sprites to scene texture
        let scene_view = frame
            .scene_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Scene texture view not available"))?;

        let occlusion_view = frame
            .occlusion_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Occlusion texture view not available"))?;

        // Create render pass for sprites, rendering to scene texture
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("sprite-pass"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: scene_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load, // Load existing scene content (shapes may have been drawn)
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
                Some(RenderPassColorAttachment {
                    view: occlusion_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load, // Load existing occlusion content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
            ],
            depth_stencil_attachment: None,
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.sprite_pipeline.pipeline);
        pass.set_vertex_buffer(0, self.sprite_pipeline.vertex_buffer.slice(..));

        // Draw all queued sprites
        for draw_cmd in &frame.sprite_draws {
            // Look up bind group for this texture (should be cached)
            let cache_key = (draw_cmd.texture_handle, 0);
            if let Some(bind_group) = self.bind_group_cache.get(&cache_key) {
                pass.set_bind_group(0, bind_group, &[draw_cmd.uniform_offset as u32]);
                pass.draw(0..SPRITE_VERTICES.len() as u32, 0..1);
            } else {
                return Err(anyhow!("Bind group not found for texture handle"));
            }
        }

        // Pass is dropped here, commands are recorded in encoder
        Ok(())
    }

    fn draw_point_light(
        &mut self,
        frame: &mut Frame,
        light: &PointLight,
        camera: &Camera2D,
    ) -> Result<()> {
        // Check if we've exceeded the maximum lights per frame
        const MAX_LIGHTS: usize = 256;
        if self.light_uniform_write_offset
            >= (MAX_LIGHTS as u64 * self.light_pipeline.uniform_alignment)
        {
            return Err(anyhow!(
                "Too many lights drawn in one frame (max: {})",
                MAX_LIGHTS
            ));
        }

        // Calculate MVP matrix for the light quad (scaled to light radius)
        let scale = Mat4::from_scale(Vec3::new(light.radius, light.radius, 1.0));
        let translation =
            Mat4::from_translation(Vec3::new(light.position.x, light.position.y, 0.0));
        let model = translation * scale;
        let vp = camera.view_projection(self.surface_config.width, self.surface_config.height);
        let mvp = vp * model;

        let (direction, angle) = if let Some(dir) = light.direction {
            ([dir.x, dir.y], light.angle.cos())
        } else {
            ([0.0, 0.0], 0.0) // Point light (no direction)
        };

        let uniforms = LightUniforms {
            position: [light.position.x, light.position.y],
            _pad1: [0.0, 0.0], // Padding for 16-byte alignment
            color: light.color,
            intensity: light.intensity,
            radius: light.radius,
            falloff: light.falloff,
            direction,
            angle,
            _pad2: 0.0,
            screen_size: [
                self.surface_config.width as f32,
                self.surface_config.height as f32,
            ],
            view_proj: vp.to_cols_array_2d(),
            mvp: mvp.to_cols_array_2d(),
        };

        // Write uniforms at the current offset (aligned to required alignment)
        let aligned_offset = if self.light_uniform_write_offset == 0 {
            0
        } else {
            (self.light_uniform_write_offset + self.light_pipeline.uniform_alignment - 1)
                & !(self.light_pipeline.uniform_alignment - 1)
        };

        // Manually serialize the struct (can't use Pod due to padding)
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &uniforms as *const LightUniforms as *const u8,
                std::mem::size_of::<LightUniforms>(),
            )
        };
        self.queue
            .write_buffer(&self.light_pipeline.uniform_buffer, aligned_offset, bytes);

        // Queue the light draw
        frame.light_draws.push(LightDrawCommand {
            uniform_offset: aligned_offset,
        });

        // Advance offset for next light
        self.light_uniform_write_offset = aligned_offset + self.light_pipeline.uniform_alignment;

        Ok(())
    }

    fn draw_particles(
        &mut self,
        frame: &mut Frame,
        particle_system: &ParticleSystem,
        camera: &Camera2D,
        default_texture: Option<TextureHandle>,
    ) -> Result<()> {
        for emitter in particle_system.emitters() {
            let texture_handle = if let Some(tex) = emitter.texture().or(default_texture) {
                tex
            } else {
                let (handle, _) = self.ensure_circle_texture()?;
                handle
            };

            // Get texture size once per emitter.
            let texture_entry = self
                .textures
                .get(&texture_handle)
                .ok_or_else(|| anyhow!("Unknown texture handle"))?;
            let texture_size = Vec2::new(texture_entry.size.0 as f32, texture_entry.size.1 as f32);

            for particle in emitter.particles() {
                if !particle.is_alive() {
                    continue;
                }

                // Particle size is in pixels, so we need to convert to scale
                // Scale = desired_size / texture_size
                let scale = Vec2::new(
                    particle.size.x / texture_size.x,
                    particle.size.y / texture_size.y,
                );

                let mut sprite = Sprite::new(texture_handle);
                sprite.transform.position = particle.position;
                sprite.transform.scale = scale;
                sprite.transform.rotation = particle.rotation;
                sprite.tint = particle.color;

                self.draw_sprite(frame, &sprite, camera)?;
            }
        }
        Ok(())
    }

    fn clear_light_map_to_white(&mut self, frame: &mut Frame) -> Result<()> {
        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        let light_map_view = frame
            .light_map_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Light map texture view not available"))?;

        // Clear light map to white (0.75, 0.75, 0.75) so that when composite adds ambient (0.25),
        // we get 0.25 + 0.75 = 1.0, which means no darkening of the scene
        let pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("clear-light-map"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: light_map_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color {
                        r: 0.75,
                        g: 0.75,
                        b: 0.75,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        drop(pass);
        Ok(())
    }

    fn flush_lights(&mut self, frame: &mut Frame) -> Result<()> {
        if frame.light_draws.is_empty() {
            return Ok(());
        }

        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        // Render lights to light map texture (additive blending)
        let light_map_view = frame
            .light_map_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Light map texture view not available"))?;

        // Create render pass for lights, rendering to light map texture
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("light-batch-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: light_map_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }), // Clear light map (black = no light)
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.light_pipeline.pipeline);
        pass.set_vertex_buffer(0, self.light_pipeline.vertex_buffer.slice(..));

        // Create bind group for lights (shared for all lights, using dynamic offset)
        let uniform_size = std::mem::size_of::<LightUniforms>() as u64;
        let occlusion_view = frame
            .occlusion_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Occlusion texture view not available"))?;

        // Create sampler for occlusion texture
        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: Some("light-occlusion-sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("light-bind-group"),
            layout: &self.light_pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.light_pipeline.uniform_buffer,
                        offset: 0,
                        size: std::num::NonZeroU64::new(uniform_size),
                    }),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(occlusion_view),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Draw all queued lights
        for draw_cmd in &frame.light_draws {
            pass.set_bind_group(0, &bind_group, &[draw_cmd.uniform_offset as u32]);
            pass.draw(0..6, 0..1); // 6 vertices for quad
        }

        drop(pass);
        Ok(())
    }

    fn end_frame(&mut self, mut frame: Frame) -> Result<()> {
        // Step 0: Clear scene texture if not already cleared (shapes may have cleared it)
        if !frame.scene_cleared {
            self.clear_scene_texture(&mut frame)?;
            frame.scene_cleared = true;
        }

        // Step 1: Render sprites to scene texture (shapes were already drawn during draw() phase)
        self.flush_sprites(&mut frame)?;

        // Step 2: Render lights to light map texture (additive)
        // If there are no lights, clear light map to white so composite doesn't darken the scene
        if frame.light_draws.is_empty() {
            self.clear_light_map_to_white(&mut frame)?;
        } else {
            self.flush_lights(&mut frame)?;
        }

        // Step 3: Composite scene and light map to final surface
        self.composite_scene_and_lights(&mut frame)?;

        let encoder = frame
            .encoder
            .take()
            .ok_or_else(|| anyhow!("Frame already ended"))?;
        self.queue.submit(Some(encoder.finish()));

        // Clean up render target textures (they'll be recreated next frame)
        drop(frame.scene_texture.take());
        drop(frame.scene_texture.take());
        drop(frame.scene_texture_view.take());
        drop(frame.occlusion_texture.take());
        drop(frame.occlusion_texture_view.take());
        drop(frame.light_map_texture.take());
        drop(frame.light_map_texture_view.take());

        if let Some(surface_texture) = frame.surface_texture.take() {
            surface_texture.present();
        }
        Ok(())
    }

    fn end_frame_readback(&mut self, mut frame: Frame) -> Result<Vec<u8>> {
        // Step 0: Clear scene texture if not already cleared (shapes may have cleared it)
        if !frame.scene_cleared {
            self.clear_scene_texture(&mut frame)?;
            frame.scene_cleared = true;
        }

        self.flush_sprites(&mut frame)?;

        if frame.light_draws.is_empty() {
            self.clear_light_map_to_white(&mut frame)?;
        } else {
            self.flush_lights(&mut frame)?;
        }

        self.composite_scene_and_lights(&mut frame)?;

        let output_texture = frame
            .output_texture
            .as_ref()
            .ok_or_else(|| anyhow!("Offscreen output texture not available"))?;

        let (width, height) = (self.surface_config.width, self.surface_config.height);
        let bytes_per_pixel = 4u32;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
        let buffer_size = padded_bytes_per_row as u64 * height as u64;

        let output_buffer = self.device.create_buffer(&BufferDescriptor {
            label: Some("offscreen-readback"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: output_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let encoder = frame
            .encoder
            .take()
            .ok_or_else(|| anyhow!("Frame already ended"))?;
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        rx.recv()
            .map_err(|_| anyhow!("Failed to read back render buffer"))?
            .map_err(|e| anyhow!("Render buffer map error: {:?}", e))?;
        let data = buffer_slice.get_mapped_range();

        let mut rgba = vec![0u8; (unpadded_bytes_per_row * height) as usize];
        for row in 0..height as usize {
            let src_offset = row * padded_bytes_per_row as usize;
            let dst_offset = row * unpadded_bytes_per_row as usize;
            let src = &data[src_offset..src_offset + unpadded_bytes_per_row as usize];
            rgba[dst_offset..dst_offset + unpadded_bytes_per_row as usize].copy_from_slice(src);
        }

        drop(data);
        output_buffer.unmap();

        drop(frame.scene_texture.take());
        drop(frame.scene_texture_view.take());
        drop(frame.occlusion_texture.take());
        drop(frame.occlusion_texture_view.take());
        drop(frame.light_map_texture.take());
        drop(frame.light_map_texture_view.take());

        Ok(rgba)
    }

    fn composite_scene_and_lights(&mut self, frame: &mut Frame) -> Result<()> {
        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        let scene_view = frame
            .scene_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Scene texture view not available"))?;
        let light_map_view = frame
            .light_map_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Light map texture view not available"))?;

        // Create sampler for textures
        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: Some("composite-sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // Create bind group for composite shader
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("composite-bind-group"),
            layout: &self.composite_pipeline.bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(scene_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(light_map_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Render composite to final surface
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("composite-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.composite_pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, self.composite_pipeline.vertex_buffer.slice(..));
        pass.draw(0..6, 0..1); // Fullscreen quad

        drop(pass);
        Ok(())
    }

    fn load_texture_from_file(&mut self, path: &str) -> Result<TextureHandle> {
        let data = fs::read(path)?;
        self.load_texture_from_bytes(&data)
    }

    fn load_texture_from_bytes(&mut self, bytes: &[u8]) -> Result<TextureHandle> {
        let image = image::load_from_memory(bytes)?.to_rgba8();
        let dimensions = image.dimensions();
        // Regular image textures use linear filtering
        self.load_texture_from_rgba(&image, dimensions.0, dimensions.1, false)
    }

    /// Load a texture from raw RGBA8 data (for glyphs, etc.)
    /// `is_font_texture`: if true, uses Nearest filtering for crisp text rendering
    pub(crate) fn load_texture_from_rgba(
        &mut self,
        data: &[u8],
        width: u32,
        height: u32,
        is_font_texture: bool,
    ) -> Result<TextureHandle> {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = self.device.create_texture(&TextureDescriptor {
            label: Some("texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());

        // Filtering mode selection:
        // - Font textures: Nearest for crisp, pixel-perfect rendering
        // - Regular sprites: Linear for smooth scaling
        let (mag_filter, min_filter) = if is_font_texture {
            (FilterMode::Nearest, FilterMode::Nearest)
        } else {
            (FilterMode::Linear, FilterMode::Linear)
        };

        let sampler = self.device.create_sampler(&SamplerDescriptor {
            label: Some(if is_font_texture {
                "font-sampler"
            } else {
                "sprite-sampler"
            }),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter,
            min_filter,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest, // No mipmaps for fonts
            ..Default::default()
        });

        let handle = TextureHandle(self.next_texture_id);
        self.next_texture_id += 1;
        self.textures.insert(
            handle,
            TextureEntry {
                texture,
                view,
                sampler,
                size: (width, height),
            },
        );

        Ok(handle)
    }

    fn texture_size(&self, handle: TextureHandle) -> Option<(u32, u32)> {
        self.textures.get(&handle).map(|t| t.size)
    }

    fn surface_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    fn load_font_from_bytes(&mut self, bytes: &[u8]) -> Result<FontHandle> {
        self.text_renderer.load_font_from_bytes(bytes)
    }

    /// Ensure all characters in the text are rasterized and cached.
    /// Glyphon handles glyph caching internally, so this is a no-op.
    fn ensure_glyphs_rasterized(
        &mut self,
        _text: &str,
        _font: FontHandle,
        _size: f32,
    ) -> Result<()> {
        // Glyphon handles glyph caching internally, no pre-rasterization needed
        Ok(())
    }

    fn draw_text(
        &mut self,
        frame: &mut Frame,
        text: &str,
        _font: FontHandle,
        size: f32,
        position: Vec2,
        color: [f32; 4],
        camera: &Camera2D,
    ) -> Result<()> {
        // Ensure text components are initialized
        self.ensure_text_components_initialized()?;

        // Get mutable references to text rendering components
        let (text_atlas, text_renderer, viewport, font_system, cache) = self
            .text_renderer
            .get_rendering_refs()
            .ok_or_else(|| anyhow!("Text components not initialized"))?;

        // Shape the text - API: set_text(font_system, text, attrs, shaping, align)
        let mut buffer = GlyphonBuffer::new(font_system, Metrics::new(size, size * 1.2));
        let attrs = Attrs::new().family(Family::Name("sans-serif"));
        buffer.set_text(font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(font_system, false);

        // Convert world position to screen coordinates using camera
        let (screen_w, screen_h) = (self.surface_config.width, self.surface_config.height);
        let screen_pos = camera.world_to_screen(position, screen_w, screen_h);

        // Create text area - add custom_glyphs field
        let text_area = TextArea {
            buffer: &buffer,
            left: screen_pos.x,
            top: screen_pos.y,
            scale: 1.0,
            bounds: glyphon::TextBounds {
                left: 0,
                top: 0,
                right: screen_w as i32,
                bottom: screen_h as i32,
            },
            default_color: Color::rgba(
                (color[0] * 255.0) as u8,
                (color[1] * 255.0) as u8,
                (color[2] * 255.0) as u8,
                (color[3] * 255.0) as u8,
            ),
            custom_glyphs: &[],
        };

        // Prepare text for rendering - prepare is on TextRenderer, not TextAtlas
        // API: text_renderer.prepare(device, queue, font_system, atlas, viewport, text_areas, cache)
        text_renderer.prepare(
            &self.device,
            &self.queue,
            font_system,
            text_atlas,
            viewport,
            [text_area],
            cache,
        )?;

        // Get encoder and scene texture view for rendering
        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        let scene_view = frame
            .scene_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Scene texture view not available"))?;

        // Render text to scene texture
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("text-pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: scene_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load, // Load existing scene content
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Render - API: render(atlas, viewport, pass) - renders whatever was prepared
        text_renderer.render(&text_atlas, viewport, &mut pass)?;

        drop(pass);

        Ok(())
    }

    /// Measure the width of text without drawing it.
    /// This is useful for accurate text alignment in HUD elements.
    fn measure_text_width(&mut self, text: &str, _font: FontHandle, size: f32) -> Result<f32> {
        self.ensure_text_components_initialized()?;

        let font_system = self.text_renderer.font_system_mut();

        let mut buffer = GlyphonBuffer::new(font_system, Metrics::new(size, size * 1.2));
        let attrs = Attrs::new().family(Family::Name("sans-serif"));
        buffer.set_text(font_system, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(font_system, false);

        let width = buffer.layout_runs().map(|r| r.line_w).fold(0f32, f32::max);
        Ok(width)
    }

    fn draw_polygon(
        &mut self,
        frame: &mut Frame,
        points: &[Vec2],
        color: [f32; 4],
        camera: &Camera2D,
        is_occluder: bool,
    ) -> Result<()> {
        if points.len() < 3 {
            return Ok(()); // Need at least 3 points for a triangle
        }

        // Triangulate polygon using ear clipping
        let triangles = self.triangulate_polygon(points);
        if triangles.is_empty() {
            return Ok(());
        }

        // Create vertex buffer for this polygon
        let vertices: Vec<ShapeVertex> = triangles
            .iter()
            .flat_map(|&(i0, i1, i2)| {
                vec![
                    ShapeVertex {
                        position: [points[i0].x, points[i0].y],
                    },
                    ShapeVertex {
                        position: [points[i1].x, points[i1].y],
                    },
                    ShapeVertex {
                        position: [points[i2].x, points[i2].y],
                    },
                ]
            })
            .collect();

        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("shape-vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: BufferUsages::VERTEX,
            });

        // Create MVP matrix
        let vp = camera.view_projection(self.surface_config.width, self.surface_config.height);
        let mvp = vp.to_cols_array_2d();

        let uniforms = ShapeUniforms {
            mvp,
            color,
            is_occluder: if is_occluder { 1.0 } else { 0.0 },
            _pad: [0.0; 3],
        };

        // Write uniforms
        self.queue.write_buffer(
            &self.shape_pipeline.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        // Create bind group
        let bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("shape-bind-group"),
            layout: &self.shape_pipeline.bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &self.shape_pipeline.uniform_buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(std::mem::size_of::<ShapeUniforms>() as u64),
                }),
            }],
        });

        // Draw in a render pass to scene texture
        // Clear scene texture on first shape draw if not already cleared
        let encoder = frame
            .encoder
            .as_mut()
            .ok_or_else(|| anyhow!("Frame already ended"))?;

        let scene_view = frame
            .scene_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Scene texture view not available"))?;

        // Fix: Use correct occlusion view binding
        let occlusion_view = frame
            .occlusion_texture_view
            .as_ref()
            .ok_or_else(|| anyhow!("Occlusion texture view not available"))?;

        // Clear scene texture on first draw if not already cleared
        if !frame.scene_cleared {
            let _clear_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("clear-scene-first"),
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: scene_view,
                        resolve_target: None,
                        ops: Operations {
                            // Keep background transparent so only geometry occludes light rays.
                            load: LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                    Some(RenderPassColorAttachment {
                        view: occlusion_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                ],
                depth_stencil_attachment: None,
                multiview_mask: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            frame.scene_cleared = true;
        }

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("shape-pass"),
            color_attachments: &[
                Some(RenderPassColorAttachment {
                    view: scene_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load, // Load existing scene content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
                Some(RenderPassColorAttachment {
                    view: occlusion_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load, // Load existing occlusion content
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                }),
            ],
            depth_stencil_attachment: None,
            multiview_mask: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.shape_pipeline.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        pass.draw(0..vertices.len() as u32, 0..1);

        drop(pass);

        Ok(())
    }

    fn draw_circle(
        &mut self,
        frame: &mut Frame,
        center: Vec2,
        radius: f32,
        color: [f32; 4],
        camera: &Camera2D,
    ) -> Result<()> {
        if radius <= 0.0 {
            return Ok(());
        }

        let (handle, texture_size) = self.ensure_circle_texture()?;
        let scale = Vec2::new(
            (radius * 2.0) / texture_size as f32,
            (radius * 2.0) / texture_size as f32,
        );

        let transform = Transform2D::new(center, scale, 0.0);
        self.draw_texture_region(frame, handle, None, &transform, color, true, camera)
    }

    fn ensure_circle_texture(&mut self) -> Result<(TextureHandle, u32)> {
        if let Some(handle) = self.circle_texture {
            return Ok((handle, self.circle_texture_size));
        }

        let size = self.circle_texture_size;
        let radius = (size as f32) * 0.5 - 1.0;
        let center = Vec2::new(radius + 1.0, radius + 1.0);
        let mut data = Vec::with_capacity((size * size * 4) as usize);

        for y in 0..size {
            for x in 0..size {
                let pos = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let dist = (pos - center).length();
                let edge = radius - dist;
                let alpha = (edge + 1.0).clamp(0.0, 1.0);
                let a = (alpha * 255.0) as u8;
                data.extend_from_slice(&[255, 255, 255, a]);
            }
        }

        let handle = self.load_texture_from_rgba(&data, size, size, false)?;
        self.circle_texture = Some(handle);
        Ok((handle, size))
    }

    /// Triangulate a polygon using ear clipping algorithm
    fn triangulate_polygon(&self, points: &[Vec2]) -> Vec<(usize, usize, usize)> {
        if points.len() < 3 {
            return Vec::new();
        }

        // For simple convex polygons, use fan triangulation
        // For more complex cases, we'd use ear clipping, but fan works for most game cases
        let mut triangles = Vec::new();
        for i in 1..(points.len() - 1) {
            triangles.push((0, i, i + 1));
        }
        triangles
    }
}

fn create_sprite_pipeline(device: &wgpu::Device, surface_format: TextureFormat) -> SpritePipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("sprite-shader"),
        source: ShaderSource::Wgsl(include_str!("sprite.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("sprite-bind-group-layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true, // Enable dynamic offsets
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<SpriteUniforms>() as u64,
                    ),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("sprite-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sprite-vertices"),
        contents: bytemuck::cast_slice(&SPRITE_VERTICES),
        usage: BufferUsages::VERTEX,
    });

    // Get the required uniform buffer alignment (usually 256 bytes)
    let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
    let uniform_size = std::mem::size_of::<SpriteUniforms>() as u64;
    // Round up to alignment (not used directly, but kept for reference)
    let _aligned_uniform_size = (uniform_size + uniform_alignment - 1) & !(uniform_alignment - 1);

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sprite-uniform-buffer"),
        size: UNIFORM_BUFFER_SIZE,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("sprite-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x2],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[
                Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }),
                // Occlusion target (R8)
                Some(ColorTargetState {
                    format: TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }),
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    SpritePipeline {
        pipeline,
        vertex_buffer,
        uniform_buffer,
        bind_group_layout,
        uniform_buffer_size: UNIFORM_BUFFER_SIZE,
        uniform_alignment,
    }
}

fn create_light_pipeline(device: &wgpu::Device, surface_format: TextureFormat) -> LightPipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("light-shader"),
        source: ShaderSource::Wgsl(include_str!("light.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("light-bind-group-layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<LightUniforms>() as u64,
                    ),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("light-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
    let uniform_size = std::mem::size_of::<LightUniforms>() as u64;
    let aligned_uniform_size = (uniform_size + uniform_alignment - 1) & !(uniform_alignment - 1);

    // Create uniform buffer (large enough for many lights)
    const MAX_LIGHTS: usize = 256;
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("light-uniform-buffer"),
        size: aligned_uniform_size * MAX_LIGHTS as u64,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Create vertex buffer for light quad
    let light_vertices: [ShapeVertex; 6] = [
        ShapeVertex {
            position: [-1.0, -1.0],
        }, // Bottom-left
        ShapeVertex {
            position: [1.0, -1.0],
        }, // Bottom-right
        ShapeVertex {
            position: [-1.0, 1.0],
        }, // Top-left
        ShapeVertex {
            position: [1.0, -1.0],
        }, // Bottom-right
        ShapeVertex {
            position: [1.0, 1.0],
        }, // Top-right
        ShapeVertex {
            position: [-1.0, 1.0],
        }, // Top-left
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("light-vertex-buffer"),
        contents: bytemuck::cast_slice(&light_vertices),
        usage: BufferUsages::VERTEX,
    });

    // Additive blending for light map accumulation
    // Lights accumulate additively in the light map texture
    let blend = wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        },
    };

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("light-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ShapeVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x2],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend: Some(blend),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    LightPipeline {
        pipeline,
        bind_group_layout,
        uniform_buffer,
        uniform_alignment,
        vertex_buffer,
    }
}

fn create_composite_pipeline(
    device: &wgpu::Device,
    surface_format: TextureFormat,
) -> CompositePipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("composite-shader"),
        source: ShaderSource::Wgsl(include_str!("composite.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("composite-bind-group-layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("composite-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    // Fullscreen quad vertices (NDC coordinates: -1 to 1)
    let quad_vertices: [SpriteVertex; 6] = [
        SpriteVertex {
            position: [-1.0, -1.0],
            uv: [0.0, 1.0],
        }, // Bottom-left
        SpriteVertex {
            position: [1.0, -1.0],
            uv: [1.0, 1.0],
        }, // Bottom-right
        SpriteVertex {
            position: [-1.0, 1.0],
            uv: [0.0, 0.0],
        }, // Top-left
        SpriteVertex {
            position: [1.0, -1.0],
            uv: [1.0, 1.0],
        }, // Bottom-right
        SpriteVertex {
            position: [1.0, 1.0],
            uv: [1.0, 0.0],
        }, // Top-right
        SpriteVertex {
            position: [-1.0, 1.0],
            uv: [0.0, 0.0],
        }, // Top-left
    ];

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("composite-vertex-buffer"),
        contents: bytemuck::cast_slice(&quad_vertices),
        usage: BufferUsages::VERTEX,
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("composite-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x2],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    CompositePipeline {
        pipeline,
        bind_group_layout,
        vertex_buffer,
    }
}

fn choose_present_mode(modes: &[PresentMode], vsync: bool) -> PresentMode {
    if vsync {
        modes
            .iter()
            .copied()
            .find(|mode| matches!(mode, PresentMode::Fifo | PresentMode::FifoRelaxed))
            .unwrap_or(PresentMode::Fifo)
    } else {
        modes
            .iter()
            .copied()
            .find(|mode| matches!(mode, PresentMode::Immediate | PresentMode::Mailbox))
            .unwrap_or(PresentMode::Immediate)
    }
}

fn choose_alpha_mode(modes: &[CompositeAlphaMode]) -> CompositeAlphaMode {
    modes
        .iter()
        .copied()
        .find(|mode| matches!(mode, CompositeAlphaMode::Auto))
        .unwrap_or_else(|| modes.first().copied().unwrap_or(CompositeAlphaMode::Opaque))
}

fn create_shape_pipeline(device: &wgpu::Device, surface_format: TextureFormat) -> ShapePipeline {
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("shape-shader"),
        source: ShaderSource::Wgsl(include_str!("shape.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("shape-bind-group-layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: std::num::NonZeroU64::new(
                    std::mem::size_of::<ShapeUniforms>() as u64
                ),
            },
            count: None,
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("shape-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;
    let uniform_size = std::mem::size_of::<ShapeUniforms>() as u64;
    let aligned_uniform_size = (uniform_size + uniform_alignment - 1) & !(uniform_alignment - 1);

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("shape-uniform-buffer"),
        size: aligned_uniform_size,
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("shape-pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<ShapeVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x2],
            }],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[
                Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }),
                // Occlusion target (R8)
                Some(ColorTargetState {
                    format: TextureFormat::R8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }),
            ],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    ShapePipeline {
        pipeline,
        bind_group_layout,
        uniform_buffer,
        uniform_alignment,
    }
}
