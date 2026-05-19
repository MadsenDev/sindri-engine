use sindri::component::Component;
use sindri::math::{Camera2D, Transform2D, Vec2};
use sindri::render::{Renderer, Sprite, TextureHandle};
use sindri::scene::Scene;
use sindri_server::routes::{PlaybackMode, PlaybackState, SharedErrors, SharedPlayback};
use sindri_server::{serve, AppState, SharedScene};
mod lua_runtime;
use lua_runtime::LuaRuntime;
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
        sindri::component::Component::Transform(sindri::component::Transform {
            x: 0.0,
            y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation: 0.0,
        }),
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

#[derive(Default)]
struct CameraRuntime {
    positions: HashMap<u64, Vec2>,
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

    let target_transform = camera
        .follow_entity
        .and_then(|id| scene.entities.get(&id))
        .and_then(component_transform)
        .unwrap_or(camera_transform);
    let target = Vec2::new(
        target_transform.x + camera.offset_x,
        target_transform.y + camera.offset_y,
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
    camera_2d
}

struct ScreenshotCapture {
    renderer: Renderer,
    camera_runtime: CameraRuntime,
    white_texture: TextureHandle,
}

impl ScreenshotCapture {
    fn new() -> anyhow::Result<Self> {
        let mut renderer = Renderer::new_offscreen(WIDTH, HEIGHT)?;
        let white_texture = renderer.load_texture_from_rgba(&[255, 255, 255, 255], 1, 1)?;
        Ok(Self {
            renderer,
            camera_runtime: CameraRuntime::default(),
            white_texture,
        })
    }

    fn capture(&mut self, scene: &Scene) -> anyhow::Result<Vec<u8>> {
        render_scene_png(
            &mut self.renderer,
            scene,
            &mut self.camera_runtime,
            self.white_texture,
        )
    }
}

fn draw_scene_contents(
    r: &mut Renderer,
    frame: &mut sindri::render::Frame,
    scene: &Scene,
    camera_runtime: &mut CameraRuntime,
    white_texture: TextureHandle,
) -> anyhow::Result<()> {
    r.clear(frame, [0.039, 0.043, 0.051, 1.0])?; // editor bg-0

    let camera = scene_camera(scene, camera_runtime);

    for entity in scene.entities.values() {
        let transform = entity.components.iter().find_map(|c| {
            if let Component::Transform(t) = c {
                Some(t)
            } else {
                None
            }
        });
        let sprite = entity.components.iter().find_map(|c| {
            if let Component::Sprite(s) = c {
                Some(s)
            } else {
                None
            }
        });
        let collider = entity.components.iter().find_map(|c| {
            if let Component::Collider(col) = c {
                Some(col)
            } else {
                None
            }
        });
        let physics_body = entity.components.iter().find_map(|c| {
            if let Component::PhysicsBody(body) = c {
                Some(body)
            } else {
                None
            }
        });

        let Some(t) = transform else { continue };
        if sprite.is_none() && collider.is_none() && physics_body.is_none() {
            continue;
        }

        let pos = Vec2::new(t.x, t.y);

        if let Some(s) = sprite {
            let mut sprite = Sprite::new(white_texture);
            sprite.transform = Transform2D {
                position: pos,
                rotation: t.rotation,
                scale: Vec2::new(s.width * t.scale_x, s.height * t.scale_y),
            };
            sprite.tint = s.color;
            r.draw_sprite(frame, &sprite, &camera)?;
        } else {
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

        // Collider outline
        if let Some(col) = collider {
            let cx = pos.x + col.offset_x;
            let cy = pos.y + col.offset_y;
            let chw = col.width * 0.5;
            let chh = col.height * 0.5;
            let col_rect = [
                Vec2::new(cx - chw, cy - chh),
                Vec2::new(cx + chw, cy - chh),
                Vec2::new(cx + chw, cy + chh),
                Vec2::new(cx - chw, cy + chh),
            ];
            r.draw_polygon(frame, &col_rect, [0.30, 0.90, 0.40, 0.25], &camera)?;
        }
    }

    Ok(())
}

fn render_scene_png(
    renderer: &mut Renderer,
    scene: &Scene,
    camera_runtime: &mut CameraRuntime,
    white_texture: TextureHandle,
) -> anyhow::Result<Vec<u8>> {
    let rgba = renderer.render_offscreen_rgba(WIDTH, HEIGHT, |r, frame| {
        draw_scene_contents(r, frame, scene, camera_runtime, white_texture)
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
        lua.update(&mut scene, scripts_dir, dt, &keys);
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
) -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    let mut window_attributes = Window::default_attributes();
    window_attributes.title = format!("Sindri Play - {}", project_dir.display());
    window_attributes.inner_size = Some(LogicalSize::new(WIDTH, HEIGHT).into());
    let window = event_loop.create_window(window_attributes)?;

    let mut renderer = Renderer::new(&window, true)?;
    let white_texture = renderer.load_texture_from_rgba(&[255, 255, 255, 255], 1, 1)?;
    println!("native play window ready");

    let mut lua = LuaRuntime::new(errors.clone())?;
    let mut camera_runtime = CameraRuntime::default();
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
                match renderer.begin_frame().and_then(|mut frame| {
                    draw_scene_contents(
                        &mut renderer,
                        &mut frame,
                        &snapshot,
                        &mut camera_runtime,
                        white_texture,
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
    scripts_dir: PathBuf,
    shared_scene: SharedScene,
    shared_keys: sindri_server::routes::SharedKeys,
    playback: SharedPlayback,
    errors: SharedErrors,
    frame_tx: tokio::sync::broadcast::Sender<Vec<u8>>,
) -> anyhow::Result<()> {
    let mut renderer = Renderer::new_offscreen(STREAM_W, STREAM_H)?;
    let white_texture = renderer.load_texture_from_rgba(&[255, 255, 255, 255], 1, 1)?;
    let mut camera_runtime = CameraRuntime::default();
    let mut lua = LuaRuntime::new(errors.clone())?;
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
            if let Ok(rgba) = renderer.render_offscreen_rgba(STREAM_W, STREAM_H, |r, frame| {
                draw_scene_contents(r, frame, &snapshot, &mut camera_runtime, white_texture)
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
        let project_label = project_dir.display().to_string();
        let frame_tx_sv = frame_tx_opt.clone();

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
                            match ScreenshotCapture::new() {
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
                            .and_then(|capture| capture.capture(scene).ok())
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
            scripts_dir,
            shared_scene,
            shared_keys,
            shared_playback,
            shared_errors,
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
        )
    }
}
