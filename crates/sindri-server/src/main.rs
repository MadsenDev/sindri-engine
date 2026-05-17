use sindri::component::Component;
use sindri::math::{Camera2D, Vec2};
use sindri::render::Renderer;
use sindri::scene::Scene;
use sindri_server::{serve, AppState, SharedScene};
mod lua_runtime;
use lua_runtime::LuaRuntime;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 720;

fn encode_png(rgba: &[u8]) -> Vec<u8> {
    use image::{ImageBuffer, Rgba};
    let img = ImageBuffer::<Rgba<u8>, _>::from_raw(WIDTH, HEIGHT, rgba.to_vec())
        .expect("invalid dimensions");
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .expect("png encode failed");
    buf
}

fn render_scene(renderer: &mut Renderer, scene: &Scene) -> anyhow::Result<Vec<u8>> {
    let rgba = renderer.render_offscreen_rgba(WIDTH, HEIGHT, |r, frame| {
        r.clear(frame, [0.039, 0.043, 0.051, 1.0])?; // editor bg-0

        // Camera: centered on world origin, y-down
        let camera = Camera2D::new(Vec2::new(0.0, 0.0));

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
    let scene = if scene_path.exists() {
        Scene::load(&scene_path)?
    } else {
        let s = Scene::new("main");
        s.save(&scene_path)?;
        s
    };

    let shared_scene: SharedScene = Arc::new(RwLock::new(scene));
    let shared_png: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
    let shared_keys: sindri_server::routes::SharedKeys =
        Arc::new(RwLock::new(std::collections::HashSet::new()));
    let shared_paused: Arc<std::sync::atomic::AtomicBool> =
        Arc::new(std::sync::atomic::AtomicBool::new(true));

    // HTTP server runs on a background thread with its own tokio runtime.
    {
        let scene_sv = shared_scene.clone();
        let png_sv = shared_png.clone();
        let keys_sv = shared_keys.clone();
        let paused_sv = shared_paused.clone();
        let scripts_root = scripts_dir.clone();
        let save_path = scene_path.clone();
        let project_label = project_dir.display().to_string();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                // Auto-save every 5 seconds
                let save_scene = scene_sv.clone();
                tokio::spawn(async move {
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        let s = save_scene.read().await;
                        let _ = s.save(&save_path);
                    }
                });

                let state = AppState {
                    scene: scene_sv,
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

        let snapshot = shared_scene.blocking_read().clone();
        match render_scene(&mut renderer, &snapshot) {
            Ok(png) => *shared_png.lock().unwrap() = Some(png),
            Err(e) => eprintln!("render error: {e}"),
        }

        std::thread::sleep(std::time::Duration::from_millis(67)); // ~15 fps
    }
}
