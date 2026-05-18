use sindri::component::Component;
use sindri::math::{Camera2D, Vec2};
use sindri::render::Renderer;
use sindri::scene::Scene;
use sindri_server::{serve, AppState, SharedScene};
mod lua_runtime;
use lua_runtime::LuaRuntime;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

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
    use image::{ImageBuffer, Rgba};
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(WIDTH, HEIGHT, rgba.to_vec())
        .expect("invalid dimensions");
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
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

fn render_scene(
    renderer: &mut Renderer,
    scene: &Scene,
    camera_runtime: &mut CameraRuntime,
) -> anyhow::Result<Vec<u8>> {
    let rgba = renderer.render_offscreen_rgba(WIDTH, HEIGHT, |r, frame| {
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

            // Body rect
            let (hw, hh, color) = if let Some(s) = sprite {
                (s.width * 0.5, s.height * 0.5, s.color)
            } else if let Some(body) = physics_body {
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
    })?;

    Ok(encode_png(&rgba))
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
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
    let shared_png: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let shared_keys: sindri_server::routes::SharedKeys =
        Arc::new(RwLock::new(std::collections::HashSet::new()));
    let shared_paused: Arc<std::sync::atomic::AtomicBool> =
        Arc::new(std::sync::atomic::AtomicBool::new(true));

    // HTTP server runs on a background thread with its own tokio runtime.
    {
        let scene_sv = shared_scene.clone();
        let png_sv = shared_png.clone();
        let scene_path_sv = shared_scene_path.clone();
        let keys_sv = shared_keys.clone();
        let paused_sv = shared_paused.clone();
        let scripts_root = scripts_dir.clone();
        let project_root = project_dir.clone();
        let project_label = project_dir.display().to_string();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                // Auto-save every 5 seconds
                let save_scene = scene_sv.clone();
                let save_path = scene_path_sv.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        let s = save_scene.read().await;
                        let path = save_path.read().await.clone();
                        let _ = s.save(&path);
                    }
                });

                let state = AppState {
                    scene: scene_sv,
                    scene_path: scene_path_sv,
                    project_root,
                    screenshot_fn: Arc::new(move || png_sv.lock().unwrap().clone()),
                    scripts_root,
                    model: String::new(),
                    keys: keys_sv,
                    paused: paused_sv,
                };

                println!("sindri engine | project: {project_label}");
                println!("listening on http://127.0.0.1:7878");

                if let Err(e) = serve(state).await {
                    eprintln!("server error: {e}");
                }
            });
        });
    }

    // Offscreen renderer + render loop on main thread.
    let mut renderer = Renderer::new_offscreen(WIDTH, HEIGHT)?;
    println!("renderer ready");

    let mut lua = LuaRuntime::new()?;
    let mut camera_runtime = CameraRuntime::default();
    let mut last_tick = std::time::Instant::now();
    let mut was_paused = true;

    loop {
        let now = std::time::Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f32().min(0.1);
        last_tick = now;

        let paused = shared_paused.load(std::sync::atomic::Ordering::Relaxed);

        // Reset script state when transitioning from paused → playing
        // so on_start fires fresh each time play is pressed
        if was_paused && !paused {
            lua.reset();
        }
        was_paused = paused;

        // Run scripts only while playing
        if !paused {
            let keys = shared_keys.blocking_read().clone();
            let mut scene = shared_scene.blocking_write();
            lua.update(&mut scene, &scripts_dir, dt, &keys);
        }
        {
            let mut scene = shared_scene.blocking_write();
            update_scene_camera_runtime(&mut scene, dt);
        }

        let snapshot = shared_scene.blocking_read().clone();
        match render_scene(&mut renderer, &snapshot, &mut camera_runtime) {
            Ok(png) => *shared_png.lock().unwrap() = Some(png),
            Err(e) => eprintln!("render error: {e}"),
        }

        std::thread::sleep(std::time::Duration::from_millis(67)); // ~15 fps
    }
}
