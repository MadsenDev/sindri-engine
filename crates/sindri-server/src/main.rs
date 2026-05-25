use sindri::component::Component;
use sindri::math::{Camera2D, Transform2D, Vec2};
use sindri::render::{Renderer, TextureHandle};
use sindri::scene::Scene;
use sindri_server::input_map_config::{ControllerState, InputMapConfig, SharedControllerState};
use sindri_server::routes::{PlaybackMode, PlaybackState, SharedErrors, SharedGizmos, SharedPlayback};
use sindri_server::{serve, AppState, SharedScene};
mod lua_runtime;
mod project_settings;
use lua_runtime::{AnimState, LuaRuntime};
use project_settings::ProjectSettings;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::Window,
};

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn push_error(errors: &SharedErrors, message: impl Into<String>) {
    let message = message.into();
    eprintln!("{message}");
    if let Ok(mut errors) = errors.lock() {
        errors.push(message);
        if errors.len() > 50 {
            let excess = errors.len() - 50;
            errors.drain(0..excess);
        }
    }
}

fn default_scene() -> Scene {
    let mut scene = Scene::new("main");
    let camera = scene.spawn("Main Camera");
    scene.add_component(
        camera,
        sindri::component::Component::Transform(sindri::component::Transform::default()),
    );
    scene.add_component(
        camera,
        sindri::component::Component::Camera(sindri::component::Camera {
            active: true,
            zoom: 1.0,
            follow_entity: None,
            offset_x: 0.0,
            offset_y: 0.0,
            bounds_min_x: None,
            bounds_min_y: None,
            bounds_max_x: None,
            bounds_max_y: None,
            smoothing: 1.0,
            dead_zone_width: 0.0,
            dead_zone_height: 0.0,
            pixel_perfect: true,
            runtime_target_zoom: None,
            runtime_zoom_speed: 0.0,
            runtime_shake_intensity: 0.0,
            runtime_shake_timer: 0.0,
            runtime_shake_seed: 0.0,
        }),
    );
    scene
}

fn encode_png(rgba: &[u8]) -> Vec<u8> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    use image::{ColorType, ImageEncoder};

    let mut buf = Vec::new();
    PngEncoder::new_with_quality(&mut buf, CompressionType::Fast, FilterType::NoFilter)
        .write_image(rgba, WIDTH, HEIGHT, ColorType::Rgba8.into())
        .expect("png encode failed");
    buf
}

fn component_transform(entity: &sindri::entity::Entity) -> Option<&sindri::component::Transform> {
    entity.components.iter().find_map(|component| {
        if let Component::Transform(transform) = component {
            Some(transform)
        } else {
            None
        }
    })
}

/// Resolve an entity's world-space position by walking up the parent chain.
/// Each child's (x, y) is a local offset from its parent's world position.
fn resolve_world_pos(scene: &Scene, entity_id: u64) -> Vec2 {
    fn inner(scene: &Scene, entity_id: u64, depth: u8) -> Vec2 {
        if depth > 16 { return Vec2::ZERO; } // cycle guard
        let Some(entity) = scene.entities.get(&entity_id) else { return Vec2::ZERO };
        let Some(t) = component_transform(entity) else { return Vec2::ZERO };
        let local = Vec2::new(t.x, t.y);
        match entity.parent {
            Some(pid) => inner(scene, pid, depth + 1) + local,
            None => local,
        }
    }
    inner(scene, entity_id, 0)
}

#[derive(Default)]
struct CameraRuntime {
    positions: HashMap<u64, Vec2>,
}

struct RenderState {
    // path -> (handle, width, height)
    texture_cache: HashMap<String, Option<(TextureHandle, u32, u32)>>,
    anim_timers: HashMap<u64, (u32, f32)>, // entity_id -> (frame_offset, timer)
    project_root: std::path::PathBuf,
    last_frame_time: std::time::Instant,
}

impl RenderState {
    fn new(project_root: std::path::PathBuf) -> Self {
        Self {
            texture_cache: HashMap::new(),
            anim_timers: HashMap::new(),
            project_root,
            last_frame_time: std::time::Instant::now(),
        }
    }

    /// Returns (handle, tex_w, tex_h) or None if the file couldn't be loaded.
    fn get_or_load(&mut self, renderer: &mut Renderer, path: &str) -> Option<(TextureHandle, u32, u32)> {
        if let Some(cached) = self.texture_cache.get(path) {
            return *cached;
        }
        let full = self.project_root.join(path);
        let result = (|| -> anyhow::Result<(TextureHandle, u32, u32)> {
            let bytes = std::fs::read(&full)?;
            let img = image::load_from_memory(&bytes)?;
            let (w, h) = (img.width(), img.height());
            let handle = renderer.load_texture_from_file(full.to_str().unwrap_or(""))?;
            Ok((handle, w, h))
        })().ok();
        self.texture_cache.insert(path.to_string(), result);
        result
    }

    fn tick_anim(&mut self, entity_id: u64, fps: f32, frame_count: u32, dt: f32) -> u32 {
        let (frame, timer) = self.anim_timers.entry(entity_id).or_insert((0, 0.0));
        if fps > 0.0 && frame_count > 0 {
            *timer += dt;
            let frame_dur = 1.0 / fps;
            while *timer >= frame_dur {
                *timer -= frame_dur;
                *frame = (*frame + 1) % frame_count;
            }
        }
        *frame
    }
}

fn camera_bounds(camera: &sindri::component::Camera) -> Option<(Vec2, Vec2)> {
    Some((
        Vec2::new(camera.bounds_min_x?, camera.bounds_min_y?),
        Vec2::new(camera.bounds_max_x?, camera.bounds_max_y?),
    ))
}

fn clamp_to_bounds(position: Vec2, bounds: Option<(Vec2, Vec2)>) -> Vec2 {
    if let Some((min, max)) = bounds {
        Vec2::new(
            position.x.clamp(min.x, max.x),
            position.y.clamp(min.y, max.y),
        )
    } else {
        position
    }
}

fn update_scene_camera_runtime(scene: &mut Scene, dt: f32) {
    for entity in scene.entities.values_mut() {
        for component in &mut entity.components {
            let Component::Camera(camera) = component else {
                continue;
            };

            if let Some(target) = camera.runtime_target_zoom {
                let speed = camera.runtime_zoom_speed.max(0.0);
                if speed <= 0.0 {
                    camera.zoom = target.max(0.01);
                    camera.runtime_target_zoom = None;
                } else {
                    let diff = target - camera.zoom;
                    let max_change = speed * dt;
                    if diff.abs() <= max_change {
                        camera.zoom = target.max(0.01);
                        camera.runtime_target_zoom = None;
                    } else {
                        camera.zoom = (camera.zoom + diff.signum() * max_change).max(0.01);
                    }
                }
            }

            if camera.runtime_shake_timer > 0.0 {
                camera.runtime_shake_timer = (camera.runtime_shake_timer - dt).max(0.0);
                camera.runtime_shake_seed += dt * 60.0;
                if camera.runtime_shake_timer <= 0.0 {
                    camera.runtime_shake_intensity = 0.0;
                    camera.runtime_shake_seed = 0.0;
                }
            }
        }
    }
}

fn scene_camera(scene: &Scene, runtime: &mut CameraRuntime) -> Camera2D {
    let candidates = scene.entities.values().filter_map(|entity| {
        let camera = entity.components.iter().find_map(|component| {
            if let Component::Camera(camera) = component {
                Some(camera)
            } else {
                None
            }
        })?;
        let transform = component_transform(entity)?;
        Some((entity.id, entity.active, camera, transform))
    });

    let selected = candidates
        .clone()
        .filter(|(_, entity_active, camera, _)| *entity_active && camera.active)
        .min_by_key(|(id, _, _, _)| *id)
        .or_else(|| candidates.min_by_key(|(id, _, _, _)| *id));

    let Some((camera_entity_id, _, camera, camera_transform)) = selected else {
        return Camera2D::new(Vec2::new(0.0, 0.0));
    };

    let target_pos = camera
        .follow_entity
        .map(|id| resolve_world_pos(scene, id))
        .unwrap_or_else(|| Vec2::new(camera_transform.x, camera_transform.y));
    let target = Vec2::new(
        target_pos.x + camera.offset_x,
        target_pos.y + camera.offset_y,
    );

    let previous = if camera.follow_entity.is_some() {
        runtime
            .positions
            .get(&camera_entity_id)
            .copied()
            .unwrap_or(target)
    } else {
        target
    };
    let mut desired = target;
    let half_dead_zone = Vec2::new(camera.dead_zone_width * 0.5, camera.dead_zone_height * 0.5);
    if half_dead_zone.x > 0.0 && (target.x - previous.x).abs() <= half_dead_zone.x {
        desired.x = previous.x;
    }
    if half_dead_zone.y > 0.0 && (target.y - previous.y).abs() <= half_dead_zone.y {
        desired.y = previous.y;
    }

    let smoothing = camera.smoothing.clamp(0.0, 1.0);
    let mut position = clamp_to_bounds(previous.lerp(desired, smoothing), camera_bounds(camera));
    if camera.runtime_shake_intensity > 0.0 && camera.runtime_shake_timer > 0.0 {
        position.x += (camera.runtime_shake_seed * 50.0).sin() * camera.runtime_shake_intensity;
        position.y += (camera.runtime_shake_seed * 43.0).cos() * camera.runtime_shake_intensity;
    }
    runtime.positions.insert(camera_entity_id, position);

    let mut camera_2d = Camera2D::new(position).with_rotation(camera_transform.rotation);
    camera_2d.zoom = camera.zoom.max(0.01);
    camera_2d.bounds = camera_bounds(camera);
    camera_2d.pixel_perfect = camera.pixel_perfect;
    camera_2d
}

struct ScreenshotCapture {
    renderer: Renderer,
    camera_runtime: CameraRuntime,
    white_texture: TextureHandle,
    render_state: RenderState,
}

impl ScreenshotCapture {
    fn new(project_root: std::path::PathBuf) -> anyhow::Result<Self> {
        let mut renderer = Renderer::new_offscreen(WIDTH, HEIGHT)?;
        let white_texture = renderer.load_texture_from_rgba(&[255, 255, 255, 255], 1, 1)?;
        Ok(Self {
            renderer,
            camera_runtime: CameraRuntime::default(),
            white_texture,
            render_state: RenderState::new(project_root),
        })
    }

    fn capture(&mut self, scene: &Scene, anim_states: &HashMap<u64, AnimState>) -> anyhow::Result<Vec<u8>> {
        render_scene_png(
            &mut self.renderer,
            scene,
            &mut self.camera_runtime,
            self.white_texture,
            &mut self.render_state,
            anim_states,
        )
    }
}

/// Compute UV rect [u, v, w, h] for a cell in a spritesheet/tileset,
/// accounting for a pixel border (margin) around the image and gaps between cells (spacing).
fn spritesheet_uv(col: u32, row: u32, cols: u32, rows: u32, margin: u32, spacing: u32, tex_w: f32, tex_h: f32) -> [f32; 4] {
    if margin == 0 && spacing == 0 {
        return [col as f32 / cols as f32, row as f32 / rows as f32, 1.0 / cols as f32, 1.0 / rows as f32];
    }
    let m = margin as f32;
    let s = spacing as f32;
    let cell_w = (tex_w - 2.0 * m - s * (cols.saturating_sub(1)) as f32) / cols as f32;
    let cell_h = (tex_h - 2.0 * m - s * (rows.saturating_sub(1)) as f32) / rows as f32;
    let x = m + col as f32 * (cell_w + s);
    let y = m + row as f32 * (cell_h + s);
    [x / tex_w, y / tex_h, cell_w / tex_w, cell_h / tex_h]
}

fn draw_scene_contents(
    r: &mut Renderer,
    frame: &mut sindri::render::Frame,
    scene: &Scene,
    camera_runtime: &mut CameraRuntime,
    white_texture: TextureHandle,
    render_state: &mut RenderState,
    anim_states: &HashMap<u64, AnimState>,
    dt: f32,
    gizmos: bool,
) -> anyhow::Result<()> {
    r.clear(frame, [0.039, 0.043, 0.051, 1.0])?;

    let camera = scene_camera(scene, camera_runtime);

    // Collect draw items sorted by (z_index, entity_id) for deterministic render order.
    // Tilemap layers are separate items: effective z = entity.transform.z_index + layer.z_index.
    enum DrawKind { Sprite, AnimSprite, TilemapLayer(usize), PhysicsOnly }
    struct DrawItem { z: i32, entity_id: u64, kind: DrawKind }

    let mut draw_list: Vec<DrawItem> = Vec::new();
    for entity in scene.entities.values() {
        let Some(t) = entity.components.iter().find_map(|c| {
            if let Component::Transform(t) = c { Some(t) } else { None }
        }) else { continue };

        let has_sprite  = entity.components.iter().any(|c| matches!(c, Component::Sprite(_)));
        let has_anim    = entity.components.iter().any(|c| matches!(c, Component::AnimatedSprite(_)));
        let has_physics = entity.components.iter().any(|c| matches!(c, Component::PhysicsBody(_)));
        let tilemap     = entity.components.iter().find_map(|c| {
            if let Component::Tilemap(tm) = c { Some(tm) } else { None }
        });

        if let Some(tm) = tilemap {
            for (layer_idx, layer) in tm.layers.iter().enumerate() {
                if !layer.visible { continue; }
                draw_list.push(DrawItem {
                    z: t.z_index + layer.z_index,
                    entity_id: entity.id,
                    kind: DrawKind::TilemapLayer(layer_idx),
                });
            }
        } else if has_anim {
            draw_list.push(DrawItem { z: t.z_index, entity_id: entity.id, kind: DrawKind::AnimSprite });
        } else if has_sprite {
            draw_list.push(DrawItem { z: t.z_index, entity_id: entity.id, kind: DrawKind::Sprite });
        } else if has_physics {
            draw_list.push(DrawItem { z: t.z_index, entity_id: entity.id, kind: DrawKind::PhysicsOnly });
        }
    }
    draw_list.sort_by_key(|item| (item.z, item.entity_id));

    for item in &draw_list {
        let Some(entity) = scene.entities.get(&item.entity_id) else { continue };
        let Some(t) = entity.components.iter().find_map(|c| {
            if let Component::Transform(t) = c { Some(t) } else { None }
        }) else { continue };
        let pos = resolve_world_pos(scene, item.entity_id);

        match &item.kind {
            DrawKind::TilemapLayer(layer_idx) => {
                let Some(tm) = entity.components.iter().find_map(|c| {
                    if let Component::Tilemap(tm) = c { Some(tm) } else { None }
                }) else { continue };
                let Some(layer) = tm.layers.get(*layer_idx) else { continue };
                let layer_alpha = layer.opacity.clamp(0.0, 1.0) * tm.tint[3];
                let tint = [tm.tint[0], tm.tint[1], tm.tint[2], layer_alpha];
                for tile_row in 0..tm.map_rows {
                    for tile_col in 0..tm.map_cols {
                        let cell = layer.tiles.get((tile_row * tm.map_cols + tile_col) as usize).copied().unwrap_or(0);
                        if cell == 0 { continue; }
                        let (palette_id, tile_idx) = sindri::component::decode_tile(cell);
                        if palette_id == 0 { continue; }
                        let Some(palette) = tm.palettes.get((palette_id - 1) as usize) else { continue };
                        let (tex, tex_w, tex_h) = match render_state.get_or_load(r, &palette.texture_path) {
                            Some((h, tw, th)) => (h, tw as f32, th as f32),
                            None => (white_texture, 1.0, 1.0),
                        };
                        let cols = palette.tileset_cols.max(1);
                        let rows = palette.tileset_rows.max(1);
                        let ts_col = tile_idx % cols;
                        let ts_row = tile_idx / cols;
                        let uv = spritesheet_uv(ts_col, ts_row, cols, rows, palette.margin, palette.spacing, tex_w, tex_h);
                        let tile_cx = pos.x + (tile_col as f32 + 0.5) * tm.tile_width;
                        let tile_cy = pos.y + (tile_row as f32 + 0.5) * tm.tile_height;
                        let tile_transform = Transform2D {
                            position: Vec2::new(tile_cx, tile_cy),
                            rotation: t.rotation,
                            scale: Vec2::new(tm.tile_width / tex_w, tm.tile_height / tex_h),
                        };
                        r.draw_texture_region(frame, tex, Some(uv), &tile_transform, tint, false, &camera)?;
                    }
                }
            }
            DrawKind::AnimSprite => {
                let Some(s) = entity.components.iter().find_map(|c| {
                    if let Component::AnimatedSprite(s) = c { Some(s) } else { None }
                }) else { continue };
                let (clip_name, runtime_frame, flip_x, flip_y) = if let Some(st) = anim_states.get(&entity.id) {
                    (st.current_clip.as_str(), Some(st.frame as u32), st.flip_x, st.flip_y)
                } else {
                    (s.default_clip.as_str(), None, s.flip_x, s.flip_y)
                };
                let clip = s.clips.iter().find(|c| c.name == clip_name)
                    .or_else(|| s.clips.first());
                let (start_frame, frame_count, fps) = clip
                    .map(|c| (c.start_frame, (c.end_frame - c.start_frame + 1).max(1), c.fps))
                    .unwrap_or((0, 1, 0.0));
                let frame_offset = if let Some(f) = runtime_frame {
                    f % frame_count
                } else {
                    render_state.tick_anim(entity.id, fps, frame_count, dt)
                };
                let abs_frame = start_frame + frame_offset;
                let cols = s.cols.max(1);
                let rows = s.rows.max(1);
                let col = abs_frame % cols;
                let row = abs_frame / cols;
                let (tex, tex_w, tex_h) = match render_state.get_or_load(r, &s.texture_path) {
                    Some((h, tw, th)) => (h, tw as f32, th as f32),
                    None => (white_texture, 1.0, 1.0),
                };
                let uv_rect = spritesheet_uv(col, row, cols, rows, s.margin, s.spacing, tex_w, tex_h);
                let pw = s.width * t.scale_x;
                let ph = s.height * t.scale_y;
                let pdx = (0.5 - t.pivot_x) * pw;
                let pdy = (0.5 - t.pivot_y) * ph;
                let (cos_r, sin_r) = (t.rotation.cos(), t.rotation.sin());
                let draw_pos = Vec2::new(
                    pos.x + pdx * cos_r - pdy * sin_r,
                    pos.y + pdx * sin_r + pdy * cos_r,
                );
                let transform = Transform2D {
                    position: draw_pos,
                    rotation: t.rotation,
                    scale: Vec2::new(
                        pw * (if flip_x { -1.0 } else { 1.0 }) / tex_w,
                        ph * (if flip_y { -1.0 } else { 1.0 }) / tex_h,
                    ),
                };
                r.draw_texture_region(frame, tex, Some(uv_rect), &transform, s.tint, false, &camera)?;
            }
            DrawKind::Sprite => {
                let Some(s) = entity.components.iter().find_map(|c| {
                    if let Component::Sprite(s) = c { Some(s) } else { None }
                }) else { continue };
                let (tex, tex_w, tex_h) = match render_state.get_or_load(r, &s.texture_path) {
                    Some((h, tw, th)) => (h, tw as f32, th as f32),
                    None => (white_texture, 1.0, 1.0),
                };
                let pw = s.width * t.scale_x;
                let ph = s.height * t.scale_y;
                let pdx = (0.5 - t.pivot_x) * pw;
                let pdy = (0.5 - t.pivot_y) * ph;
                let (cos_r, sin_r) = (t.rotation.cos(), t.rotation.sin());
                let draw_pos = Vec2::new(
                    pos.x + pdx * cos_r - pdy * sin_r,
                    pos.y + pdx * sin_r + pdy * cos_r,
                );
                let transform = Transform2D {
                    position: draw_pos,
                    rotation: t.rotation,
                    scale: Vec2::new(pw / tex_w, ph / tex_h),
                };
                r.draw_texture_region(frame, tex, None, &transform, s.color, false, &camera)?;
            }
            DrawKind::PhysicsOnly => {
                let physics_body = entity.components.iter().find_map(|c| {
                    if let Component::PhysicsBody(body) = c { Some(body) } else { None }
                });
                let (hw, hh, color) = if let Some(body) = physics_body {
                    let color = match body.body_type {
                        sindri::component::BodyType::Dynamic => [0.0, 1.0, 0.9, 0.85],
                        sindri::component::BodyType::Kinematic => [1.0, 0.9, 0.0, 0.85],
                        sindri::component::BodyType::Fixed => [0.0, 1.0, 0.3, 0.85],
                    };
                    (t.scale_x * 16.0, t.scale_y * 16.0, color)
                } else {
                    (t.scale_x * 16.0, t.scale_y * 16.0, [0.38, 0.60, 0.93, 0.9])
                };
                let rect = [
                    Vec2::new(pos.x - hw, pos.y - hh),
                    Vec2::new(pos.x + hw, pos.y - hh),
                    Vec2::new(pos.x + hw, pos.y + hh),
                    Vec2::new(pos.x - hw, pos.y + hh),
                ];
                r.draw_polygon(frame, &rect, color, &camera)?;
            }
        }

        // Gizmo overlays drawn right after each entity
        if gizmos {
            let collider = entity.components.iter().find_map(|c| {
                if let Component::Collider(col) = c { Some(col) } else { None }
            });
            if let Some(col) = collider {
                let cx = t.x + col.offset_x;
                let cy = t.y + col.offset_y;
                let hw = col.width * 0.5;
                let hh = col.height * 0.5;
                let rect = [
                    Vec2::new(cx - hw, cy - hh),
                    Vec2::new(cx + hw, cy - hh),
                    Vec2::new(cx + hw, cy + hh),
                    Vec2::new(cx - hw, cy + hh),
                ];
                r.draw_polygon(frame, &rect, [0.60, 0.95, 0.45, 0.30], &camera)?;
            }
        }
    }

    Ok(())
}

fn render_scene_png(
    renderer: &mut Renderer,
    scene: &Scene,
    camera_runtime: &mut CameraRuntime,
    white_texture: TextureHandle,
    render_state: &mut RenderState,
    anim_states: &HashMap<u64, AnimState>,
) -> anyhow::Result<Vec<u8>> {
    let rgba = renderer.render_offscreen_rgba(WIDTH, HEIGHT, |r, frame| {
        draw_scene_contents(r, frame, scene, camera_runtime, white_texture, render_state, anim_states, 0.0, false)
    })?;
    Ok(encode_png(&rgba))
}

fn key_name(event: &KeyEvent) -> Option<String> {
    match &event.logical_key {
        Key::Named(NamedKey::ArrowLeft) => Some("ArrowLeft".into()),
        Key::Named(NamedKey::ArrowRight) => Some("ArrowRight".into()),
        Key::Named(NamedKey::ArrowUp) => Some("ArrowUp".into()),
        Key::Named(NamedKey::ArrowDown) => Some("ArrowDown".into()),
        Key::Named(NamedKey::Space) => Some(" ".into()),
        Key::Named(NamedKey::Enter) => Some("Enter".into()),
        Key::Named(NamedKey::Tab) => Some("Tab".into()),
        Key::Named(NamedKey::Shift) => Some("Shift".into()),
        Key::Named(NamedKey::Control) => Some("Control".into()),
        Key::Named(NamedKey::Alt) => Some("Alt".into()),
        Key::Named(NamedKey::Escape) => Some("Escape".into()),
        Key::Character(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Spawn a background thread that polls gilrs for gamepad events and writes to SharedControllerState.
fn spawn_controller_thread(state: SharedControllerState) {
    std::thread::Builder::new()
        .name("sindri-controller".into())
        .spawn(move || {
            let gilrs = match gilrs::Gilrs::new() {
                Ok(g) => g,
                Err(e) => {
                    eprintln!("[controller] gilrs init failed: {e}");
                    return;
                }
            };
            let mut gilrs = gilrs;
            let mut buttons_held: std::collections::HashMap<String, bool> = Default::default();
            let sleep_dur = std::time::Duration::from_millis(8);
            loop {
                let mut pressed_this_poll: std::collections::HashMap<String, bool> = Default::default();
                let mut axes: std::collections::HashMap<String, f32> = Default::default();

                while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
                    match event {
                        gilrs::EventType::ButtonPressed(btn, _) => {
                            let name = format!("{btn:?}");
                            buttons_held.insert(name.clone(), true);
                            pressed_this_poll.insert(name, true);
                        }
                        gilrs::EventType::ButtonReleased(btn, _) => {
                            buttons_held.remove(&format!("{btn:?}"));
                        }
                        gilrs::EventType::AxisChanged(axis, value, _) => {
                            axes.insert(format!("{axis:?}"), value);
                        }
                        _ => {}
                    }
                }

                // Merge persisted axis values from active gamepads
                for (_id, gamepad) in gilrs.gamepads() {
                    for axis in [
                        gilrs::Axis::LeftStickX,
                        gilrs::Axis::LeftStickY,
                        gilrs::Axis::RightStickX,
                        gilrs::Axis::RightStickY,
                        gilrs::Axis::LeftZ,
                        gilrs::Axis::RightZ,
                    ] {
                        if let Some(data) = gamepad.axis_data(axis) {
                            axes.entry(format!("{axis:?}")).or_insert(data.value());
                        }
                    }
                }

                if let Ok(mut s) = state.write() {
                    s.axes = axes;
                    s.buttons_pressed = pressed_this_poll;
                    s.buttons_held = buttons_held.clone();
                }
                std::thread::sleep(sleep_dur);
            }
        })
        .ok();
}

fn update_runtime(
    lua: &mut LuaRuntime,
    shared_scene: &SharedScene,
    scripts_dir: &std::path::Path,
    shared_keys: &sindri_server::routes::SharedKeys,
    playback: &SharedPlayback,
    last_tick: &mut std::time::Instant,
    was_stopped: &mut bool,
) {
    let now = std::time::Instant::now();
    let dt = now.duration_since(*last_tick).as_secs_f32().min(0.1);
    *last_tick = now;

    let mode = playback
        .lock()
        .map(|playback| playback.mode)
        .unwrap_or(PlaybackMode::Stopped);
    if *was_stopped && mode == PlaybackMode::Playing {
        lua.reset();
    }
    *was_stopped = mode == PlaybackMode::Stopped;

    if mode == PlaybackMode::Playing {
        let keys = shared_keys.blocking_read().clone();
        let mut scene = shared_scene.blocking_write();
        let (gx, gy) = (scene.gravity_x, scene.gravity_y);
        lua.step_physics(&mut scene, gx, gy, dt);
        lua.update(&mut scene, scripts_dir, dt, &keys);
    }

    // Handle load_scene() requested by a script
    if let Some(scene_rel) = lua.take_pending_scene_load() {
        let project_dir = scripts_dir.parent().unwrap_or(scripts_dir);
        let full_path = if std::path::Path::new(&scene_rel).is_absolute() {
            std::path::PathBuf::from(&scene_rel)
        } else {
            project_dir.join(&scene_rel)
        };
        match Scene::load(&full_path) {
            Ok(mut new_scene) => {
                sindri_server::routes::normalize_scene_cameras(&mut new_scene);
                *shared_scene.blocking_write() = new_scene;
                lua.reset();
            }
            Err(e) => eprintln!("[lua] load_scene('{scene_rel}') failed: {e}"),
        }
    }

    {
        let mut scene = shared_scene.blocking_write();
        update_scene_camera_runtime(&mut scene, dt);
    }
}

#[allow(deprecated)]
fn run_preview_window(
    project_dir: &std::path::Path,
    scripts_dir: PathBuf,
    shared_scene: SharedScene,
    shared_keys: sindri_server::routes::SharedKeys,
    playback: SharedPlayback,
    errors: SharedErrors,
    gizmos: SharedGizmos,
    debug_paths: sindri_server::routes::SharedDebugPaths,
    input_map: InputMapConfig,
    controller: SharedControllerState,
) -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut window_attributes = Window::default_attributes();
    window_attributes.title = format!("Sindri Play - {}", project_dir.display());
    window_attributes.inner_size = Some(LogicalSize::new(WIDTH, HEIGHT).into());
    let window = event_loop.create_window(window_attributes)?;

    let project_settings = ProjectSettings::load(project_dir);
    let mut renderer = Renderer::new(&window, true)?;
    renderer.set_pixel_art_mode(project_settings.pixel_art_mode);
    let white_texture = renderer.load_texture_from_rgba(&[255, 255, 255, 255], 1, 1)?;
    println!("native play window ready");

    let mut lua = LuaRuntime::new(errors.clone(), debug_paths, input_map, controller, project_dir.to_path_buf())?;
    let mut camera_runtime = CameraRuntime::default();
    let mut render_state = RenderState::new(project_dir.to_path_buf());
    let mut last_tick = std::time::Instant::now();
    let mut was_stopped = true;

    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => {
                elwt.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(key) = key_name(&event) {
                    if key == "Escape" && event.state == ElementState::Pressed {
                        elwt.exit();
                        return;
                    }
                    if let Ok(mut keys) = shared_keys.try_write() {
                        match event.state {
                            ElementState::Pressed => {
                                keys.insert(key);
                            }
                            ElementState::Released => {
                                keys.remove(&key);
                            }
                        }
                    }
                }
            }
            WindowEvent::Resized(new_size) => {
                renderer.resize(new_size);
            }
            WindowEvent::RedrawRequested => {
                let snapshot = shared_scene.blocking_read().clone();
                let gz = gizmos.load(std::sync::atomic::Ordering::Relaxed);
                let now = std::time::Instant::now();
                let dt = now.duration_since(render_state.last_frame_time).as_secs_f32().min(0.1);
                render_state.last_frame_time = now;
                match renderer.begin_frame().and_then(|mut frame| {
                    draw_scene_contents(
                        &mut renderer,
                        &mut frame,
                        &snapshot,
                        &mut camera_runtime,
                        white_texture,
                        &mut render_state,
                        &lua.anim_states,
                        dt,
                        gz,
                    )?;
                    renderer.end_frame(frame)
                }) {
                    Ok(()) => {}
                    Err(e) => push_error(&errors, format!("preview render error: {e}")),
                }
            }
            _ => {}
        },
        Event::AboutToWait => {
            update_runtime(
                &mut lua,
                &shared_scene,
                &scripts_dir,
                &shared_keys,
                &playback,
                &mut last_tick,
                &mut was_stopped,
            );
            window.request_redraw();
        }
        _ => {}
    })?;

    Ok(())
}

const STREAM_W: u32 = 960;
const STREAM_H: u32 = 540;

fn run_headless(
    project_dir: PathBuf,
    scripts_dir: PathBuf,
    shared_scene: SharedScene,
    shared_keys: sindri_server::routes::SharedKeys,
    playback: SharedPlayback,
    errors: SharedErrors,
    gizmos: SharedGizmos,
    debug_paths: sindri_server::routes::SharedDebugPaths,
    input_map: InputMapConfig,
    controller: SharedControllerState,
    frame_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
) -> anyhow::Result<()> {
    let project_settings = ProjectSettings::load(&project_dir);
    let mut renderer = Renderer::new_offscreen(STREAM_W, STREAM_H)?;
    renderer.set_pixel_art_mode(project_settings.pixel_art_mode);
    let white_texture = renderer.load_texture_from_rgba(&[255, 255, 255, 255], 1, 1)?;
    let mut camera_runtime = CameraRuntime::default();
    let mut render_state = RenderState::new(project_dir.clone());
    let mut lua = LuaRuntime::new(errors.clone(), debug_paths, input_map, controller, project_dir)?;
    let mut last_tick = std::time::Instant::now();
    let mut was_stopped = true;
    let frame_interval = std::time::Duration::from_millis(33);

    println!("headless streaming ready (ws://127.0.0.1:7878/stream)");

    loop {
        let frame_start = std::time::Instant::now();

        update_runtime(
            &mut lua,
            &shared_scene,
            &scripts_dir,
            &shared_keys,
            &playback,
            &mut last_tick,
            &mut was_stopped,
        );

        if frame_tx.receiver_count() > 0 {
            let snapshot = shared_scene.blocking_read().clone();
            let gz = gizmos.load(std::sync::atomic::Ordering::Relaxed);
            let now = std::time::Instant::now();
            let dt = now.duration_since(render_state.last_frame_time).as_secs_f32().min(0.1);
            render_state.last_frame_time = now;
            if let Ok(rgba) = renderer.render_offscreen_rgba(STREAM_W, STREAM_H, |r, frame| {
                draw_scene_contents(r, frame, &snapshot, &mut camera_runtime, white_texture, &mut render_state, &lua.anim_states, dt, gz)
            }) {
                let _ = frame_tx.send(rgba);
            }
        }

        let elapsed = frame_start.elapsed();
        if elapsed < frame_interval {
            std::thread::sleep(frame_interval - elapsed);
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let headless = args.contains(&"--headless".to_string());
    let project_dir = args
        .iter()
        .position(|a| a == "--project-dir")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("no cwd"));

    let scenes_dir = project_dir.join("scenes");
    let scripts_dir = project_dir.join("scripts");
    std::fs::create_dir_all(&scenes_dir)?;
    std::fs::create_dir_all(&scripts_dir)?;

    let scene_path = scenes_dir.join("main.sindri");
    let mut scene = if scene_path.exists() {
        Scene::load(&scene_path)?
    } else {
        let s = default_scene();
        s.save(&scene_path)?;
        s
    };
    sindri_server::routes::normalize_scene_cameras(&mut scene);

    let shared_scene: SharedScene = Arc::new(RwLock::new(scene));
    let shared_scene_path = Arc::new(RwLock::new(scene_path));
    let screenshot_capture: Arc<Mutex<Option<ScreenshotCapture>>> = Arc::new(Mutex::new(None));
    let shared_keys: sindri_server::routes::SharedKeys =
        Arc::new(RwLock::new(std::collections::HashSet::new()));
    let shared_playback: SharedPlayback = Arc::new(Mutex::new(PlaybackState::default()));
    let shared_errors: SharedErrors = Arc::new(Mutex::new(Vec::new()));
    let shared_gizmos: SharedGizmos = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shared_debug_paths: sindri_server::routes::SharedDebugPaths =
        Arc::new(std::sync::RwLock::new(std::collections::HashMap::new()));
    let input_map = InputMapConfig::load(&project_dir);
    let shared_controller: SharedControllerState = Arc::new(std::sync::RwLock::new(ControllerState::default()));
    spawn_controller_thread(shared_controller.clone());

    let (frame_tx, _frame_rx) = tokio::sync::broadcast::channel::<Vec<u8>>(4);
    let frame_tx_opt: Option<tokio::sync::broadcast::Sender<Vec<u8>>> = if headless {
        Some(frame_tx.clone())
    } else {
        None
    };

    // HTTP server runs on a background thread with its own tokio runtime.
    {
        let scene_sv = shared_scene.clone();
        let screenshot_capture = screenshot_capture.clone();
        let scene_path_sv = shared_scene_path.clone();
        let keys_sv = shared_keys.clone();
        let playback_sv = shared_playback.clone();
        let errors_sv = shared_errors.clone();
        let state_errors = shared_errors.clone();
        let thread_errors = shared_errors.clone();
        let scripts_root = scripts_dir.clone();
        let project_root = project_dir.clone();
        let project_root_for_capture = project_dir.clone();
        let project_label = project_dir.display().to_string();
        let frame_tx_sv = frame_tx_opt.clone();
        let gizmos_sv = shared_gizmos.clone();
        let debug_paths_sv = shared_debug_paths.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                // Auto-save every 5 seconds
                let save_scene = scene_sv.clone();
                let save_path = scene_path_sv.clone();
                let save_playback = playback_sv.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        let should_save = save_playback
                            .lock()
                            .map(|playback| playback.mode == PlaybackMode::Stopped)
                            .unwrap_or(false);
                        if !should_save {
                            continue;
                        }
                        let s = save_scene.read().await;
                        let path = save_path.read().await.clone();
                        let _ = s.save(&path);
                    }
                });

                let state = AppState {
                    scene: scene_sv,
                    scene_path: scene_path_sv,
                    project_root,
                    screenshot_fn: Arc::new(move |scene| {
                        let mut capture = screenshot_capture.lock().ok()?;
                        if capture.is_none() {
                            match ScreenshotCapture::new(project_root_for_capture.clone()) {
                                Ok(new_capture) => *capture = Some(new_capture),
                                Err(e) => {
                                    push_error(
                                        &errors_sv,
                                        format!("screenshot renderer error: {e}"),
                                    );
                                    return None;
                                }
                            }
                        }
                        match capture
                            .as_mut()
                            .and_then(|capture| capture.capture(scene, &HashMap::new()).ok())
                        {
                            Some(image) => Some(image),
                            None => {
                                push_error(&errors_sv, "screenshot capture failed".to_string());
                                None
                            }
                        }
                    }),
                    scripts_root,
                    model: String::new(),
                    keys: keys_sv,
                    playback: playback_sv,
                    errors: state_errors.clone(),
                    gizmos: gizmos_sv,
                    debug_paths: debug_paths_sv,
                    frame_tx: frame_tx_sv,
                };

                println!("sindri engine | project: {project_label}");
                println!("listening on http://127.0.0.1:7878");

                if let Err(e) = serve(state).await {
                    push_error(&thread_errors, format!("server error: {e}"));
                }
            });
        });
    }

    if headless {
        run_headless(
            project_dir,
            scripts_dir,
            shared_scene,
            shared_keys,
            shared_playback,
            shared_errors,
            shared_gizmos,
            shared_debug_paths,
            input_map,
            shared_controller,
            frame_tx,
        )
    } else {
        run_preview_window(
            &project_dir,
            scripts_dir,
            shared_scene,
            shared_keys,
            shared_playback,
            shared_errors,
            shared_gizmos,
            shared_debug_paths,
            input_map,
            shared_controller,
        )
    }
}
