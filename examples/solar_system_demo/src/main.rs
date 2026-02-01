use anyhow::Result;
use forge2d::{Camera2D, Engine, EngineContext, Game, KeyCode, MouseButton, Vec2};

const GRAVITY_SCALE: f32 = 5200.0;

#[derive(Clone)]
struct OrbitBody {
    name: &'static str,
    radius: f32,
    size: f32,
    color: [f32; 4],
    phase: f32,
    angular_speed: f32,
    pos: Vec2,
    prev_pos: Vec2,
    vel: Vec2,
    mass: f32,
}

#[derive(Clone)]
struct Moon {
    parent_index: usize,
    body: OrbitBody,
}

#[derive(Clone)]
struct Comet {
    pos: Vec2,
    prev_pos: Vec2,
    vel: Vec2,
    life: f32,
}

struct SolarSystem {
    camera: Camera2D,
    starfield: Vec<Vec2>,
    star_colors: Vec<[f32; 4]>,
    bodies: Vec<OrbitBody>,
    moons: Vec<Moon>,
    asteroids: Vec<OrbitBody>,
    comets: Vec<Comet>,
    center: Vec2,
    screen_size: Vec2,
    zoom: f32,
    zoom_target: f32,
    paused: bool,
    follow_selected: bool,
    selected_body: Option<usize>,
    nbody_mode: bool,
    drag_start_world: Option<Vec2>,
    drag_start_screen: Option<Vec2>,
    drag_end_screen: Vec2,
}

impl SolarSystem {
    fn new() -> Self {
        Self {
            camera: Camera2D::default(),
            starfield: Vec::new(),
            star_colors: Vec::new(),
            bodies: Vec::new(),
            moons: Vec::new(),
            asteroids: Vec::new(),
            comets: Vec::new(),
            center: Vec2::ZERO,
            screen_size: Vec2::ZERO,
            zoom: 1.0,
            zoom_target: 1.0,
            paused: false,
            follow_selected: false,
            selected_body: None,
            nbody_mode: false,
            drag_start_world: None,
            drag_start_screen: None,
            drag_end_screen: Vec2::ZERO,
        }
    }

    fn spawn_starfield(&mut self, width: f32, height: f32) {
        self.starfield.clear();
        self.star_colors.clear();
        for _ in 0..80 {
            self.starfield.push(Vec2::new(
                fastrand::f32() * width,
                fastrand::f32() * height,
            ));
            let tint = 0.5 + fastrand::f32() * 0.5;
            self.star_colors.push([0.6 * tint, 0.7 * tint, 1.0 * tint, 0.7]);
        }
    }

    fn spawn_bodies(&mut self) {
        let sun_mass = 22000.0;
        self.bodies = vec![
            OrbitBody {
                name: "Sol",
                radius: 0.0,
                size: 48.0,
                color: [1.0, 0.82, 0.5, 1.0],
                phase: 0.0,
                angular_speed: 0.0,
                pos: self.center,
                prev_pos: self.center,
                vel: Vec2::ZERO,
                mass: sun_mass,
            },
            OrbitBody {
                name: "Aster",
                radius: 120.0,
                size: 12.0,
                color: [0.7, 0.8, 1.0, 1.0],
                phase: 0.2,
                angular_speed: 0.9,
                pos: self.center,
                prev_pos: self.center,
                vel: Vec2::ZERO,
                mass: 5.0,
            },
            OrbitBody {
                name: "Verdan",
                radius: 200.0,
                size: 18.0,
                color: [0.5, 1.0, 0.7, 1.0],
                phase: 1.1,
                angular_speed: 0.65,
                pos: self.center,
                prev_pos: self.center,
                vel: Vec2::ZERO,
                mass: 8.0,
            },
            OrbitBody {
                name: "Cinder",
                radius: 280.0,
                size: 16.0,
                color: [1.0, 0.55, 0.4, 1.0],
                phase: 2.4,
                angular_speed: 0.5,
                pos: self.center,
                prev_pos: self.center,
                vel: Vec2::ZERO,
                mass: 7.0,
            },
            OrbitBody {
                name: "Brume",
                radius: 380.0,
                size: 24.0,
                color: [0.6, 0.7, 1.0, 1.0],
                phase: 0.6,
                angular_speed: 0.35,
                pos: self.center,
                prev_pos: self.center,
                vel: Vec2::ZERO,
                mass: 14.0,
            },
            OrbitBody {
                name: "Sable",
                radius: 470.0,
                size: 20.0,
                color: [0.9, 0.75, 0.45, 1.0],
                phase: 1.7,
                angular_speed: 0.28,
                pos: self.center,
                prev_pos: self.center,
                vel: Vec2::ZERO,
                mass: 10.0,
            },
        ];

        self.moons = vec![
            Moon {
                parent_index: 2,
                body: OrbitBody {
                    name: "Verdan-I",
                    radius: 26.0,
                    size: 6.0,
                    color: [0.8, 0.9, 1.0, 1.0],
                    phase: 0.3,
                    angular_speed: 2.4,
                    pos: self.center,
                    prev_pos: self.center,
                    vel: Vec2::ZERO,
                    mass: 1.0,
                },
            },
            Moon {
                parent_index: 3,
                body: OrbitBody {
                    name: "Cinder-I",
                    radius: 22.0,
                    size: 5.0,
                    color: [1.0, 0.7, 0.6, 1.0],
                    phase: 1.1,
                    angular_speed: 2.8,
                    pos: self.center,
                    prev_pos: self.center,
                    vel: Vec2::ZERO,
                    mass: 1.0,
                },
            },
            Moon {
                parent_index: 4,
                body: OrbitBody {
                    name: "Brume-I",
                    radius: 34.0,
                    size: 7.0,
                    color: [0.7, 0.8, 1.0, 1.0],
                    phase: 2.0,
                    angular_speed: 2.0,
                    pos: self.center,
                    prev_pos: self.center,
                    vel: Vec2::ZERO,
                    mass: 1.0,
                },
            },
            Moon {
                parent_index: 4,
                body: OrbitBody {
                    name: "Brume-II",
                    radius: 52.0,
                    size: 4.5,
                    color: [0.9, 0.95, 1.0, 0.9],
                    phase: 0.7,
                    angular_speed: 1.6,
                    pos: self.center,
                    prev_pos: self.center,
                    vel: Vec2::ZERO,
                    mass: 1.0,
                },
            },
            Moon {
                parent_index: 5,
                body: OrbitBody {
                    name: "Sable-I",
                    radius: 28.0,
                    size: 6.0,
                    color: [0.9, 0.85, 0.65, 1.0],
                    phase: 1.9,
                    angular_speed: 1.8,
                    pos: self.center,
                    prev_pos: self.center,
                    vel: Vec2::ZERO,
                    mass: 1.0,
                },
            },
        ];

        self.asteroids.clear();
        for _ in 0..36 {
            let radius = 330.0 + fastrand::f32() * 60.0;
            let phase = fastrand::f32() * std::f32::consts::TAU;
            let speed = 0.4 + fastrand::f32() * 0.15;
            let size = 2.5 + fastrand::f32() * 2.5;
            self.asteroids.push(OrbitBody {
                name: "Belt",
                radius,
                size,
                color: [0.6, 0.55, 0.5, 0.8],
                phase,
                angular_speed: speed,
                pos: self.center,
                prev_pos: self.center,
                vel: Vec2::ZERO,
                mass: 0.2,
            });
        }

        self.comets.clear();

        // Initialize velocities for n-body mode (circular around sun).
        let sun_pos = self.center;
        for body in &mut self.bodies {
            if body.radius <= 0.0 {
                continue;
            }
            let dir = Vec2::from_angle(body.phase);
            let pos = sun_pos + dir * body.radius;
            let tangent = Vec2::new(-dir.y, dir.x);
            let speed = (sun_mass / body.radius).sqrt();
            body.pos = pos;
            body.prev_pos = pos;
            body.vel = tangent * speed;
        }
    }
}

impl Game for SolarSystem {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let (screen_w, screen_h) = ctx.renderer().surface_size();
        self.center = Vec2::new(screen_w as f32 * 0.5, screen_h as f32 * 0.5);
        self.screen_size = Vec2::new(screen_w as f32, screen_h as f32);
        self.camera = ctx.screen_camera();
        self.zoom = 1.0;
        self.zoom_target = 1.0;
        self.camera.zoom = self.zoom;

        self.spawn_starfield(screen_w as f32, screen_h as f32);
        self.spawn_bodies();

        Ok(())
    }

    fn fixed_update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if self.paused {
            return Ok(());
        }

        let dt = ctx.fixed_delta_seconds();
        if self.nbody_mode {
            let sun_pos = self.center;
            let sun_mass = self.bodies.first().map(|b| b.mass).unwrap_or(20000.0);
            let mut accelerations = vec![Vec2::ZERO; self.bodies.len()];

            for (i, body) in self.bodies.iter().enumerate() {
                if body.radius == 0.0 {
                    continue;
                }
                let to_sun = sun_pos - body.pos;
                let dist = to_sun.length().max(20.0);
                let accel = to_sun * (sun_mass / (dist * dist * dist));
                accelerations[i] += accel;
            }

            for i in 1..self.bodies.len() {
                for j in (i + 1)..self.bodies.len() {
                    let a = self.bodies[i].pos;
                    let b = self.bodies[j].pos;
                    let to_b = b - a;
                    let dist = to_b.length().max(30.0);
                    let force_dir = to_b * (1.0 / (dist * dist * dist));
                    accelerations[i] += force_dir * self.bodies[j].mass;
                    accelerations[j] = accelerations[j] - force_dir * self.bodies[i].mass;
                }
            }

            for (i, body) in self.bodies.iter_mut().enumerate() {
                body.prev_pos = body.pos;
                if body.radius == 0.0 {
                    body.pos = self.center;
                    continue;
                }
                body.vel += accelerations[i] * dt;
                body.pos += body.vel * dt;
            }
        } else {
            for body in &mut self.bodies {
                body.prev_pos = body.pos;
                if body.radius == 0.0 {
                    body.pos = self.center;
                    continue;
                }
                body.phase += body.angular_speed * dt;
                let angle = body.phase;
                body.pos = Vec2::new(
                    self.center.x + angle.cos() * body.radius,
                    self.center.y + angle.sin() * body.radius,
                );
            }
        }

        for moon in &mut self.moons {
            moon.body.prev_pos = moon.body.pos;
            let parent = &self.bodies[moon.parent_index];
            moon.body.phase += moon.body.angular_speed * dt;
            let angle = moon.body.phase;
            moon.body.pos = Vec2::new(
                parent.pos.x + angle.cos() * moon.body.radius,
                parent.pos.y + angle.sin() * moon.body.radius,
            );
        }

        for asteroid in &mut self.asteroids {
            asteroid.prev_pos = asteroid.pos;
            asteroid.phase += asteroid.angular_speed * dt;
            let angle = asteroid.phase;
            asteroid.pos = Vec2::new(
                self.center.x + angle.cos() * asteroid.radius,
                self.center.y + angle.sin() * asteroid.radius,
            );
        }

        for comet in &mut self.comets {
            comet.prev_pos = comet.pos;
            let mut accel = Vec2::ZERO;

            for body in &self.bodies {
                let to_body = body.pos - comet.pos;
                let dist = to_body.length().max(25.0);
                accel += to_body * (body.mass * GRAVITY_SCALE / (dist * dist * dist));
            }

            for moon in &self.moons {
                let to_body = moon.body.pos - comet.pos;
                let dist = to_body.length().max(20.0);
                accel += to_body * (moon.body.mass * GRAVITY_SCALE / (dist * dist * dist));
            }

            comet.vel += accel * dt;
            comet.vel = comet.vel * (1.0 - 0.08 * dt);
            comet.pos += comet.vel * dt;
            comet.life -= dt;
        }

        self.comets.retain(|comet| {
            if comet.life <= 0.0 {
                return false;
            }

            for body in &self.bodies {
                let hit_radius = body.size * 0.5 + 4.0;
                if body.pos.distance(comet.pos) <= hit_radius {
                    return false;
                }
            }

            for moon in &self.moons {
                let hit_radius = moon.body.size * 0.5 + 3.0;
                if moon.body.pos.distance(comet.pos) <= hit_radius {
                    return false;
                }
            }

            true
        });
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let (screen_w, screen_h) = ctx.renderer().surface_size();
        self.screen_size = Vec2::new(screen_w as f32, screen_h as f32);
        let input = ctx.input();
        if input.is_key_pressed(KeyCode::KeyP) {
            self.paused = !self.paused;
        }

        if input.is_key_pressed(KeyCode::KeyR) {
            self.spawn_bodies();
        }

        if input.is_key_pressed(KeyCode::KeyN) {
            self.nbody_mode = !self.nbody_mode;
        }

        if input.is_key_pressed(KeyCode::KeyF) {
            self.follow_selected = !self.follow_selected;
        }

        if input.is_key_pressed(KeyCode::KeyC) {
            self.selected_body = None;
            self.follow_selected = false;
        }

        if input.is_mouse_down(MouseButton::Right) {
            self.center = ctx.mouse_world(&self.camera);
        }

        if input.is_mouse_pressed(MouseButton::Left) {
            let start_world = ctx.mouse_world(&self.camera);
            let start_screen = input.mouse_position_vec2();
            self.drag_start_world = Some(start_world);
            self.drag_start_screen = Some(start_screen);
            self.drag_end_screen = start_screen;
        } else if input.is_mouse_down(MouseButton::Left) {
            self.drag_end_screen = input.mouse_position_vec2();
        } else if input.is_mouse_released(MouseButton::Left) {
            if let (Some(start_world), Some(start_screen)) = (
                self.drag_start_world.take(),
                self.drag_start_screen.take(),
            ) {
                let drag_screen = self.drag_end_screen - start_screen;
                if drag_screen.length() > 8.0 {
                    let mut nearest_pos = self.center;
                    let mut nearest_mass = self.bodies.first().map(|b| b.mass).unwrap_or(20000.0);
                    let mut nearest_vel = Vec2::ZERO;
                    let mut nearest_dist = f32::INFINITY;

                    for body in &self.bodies {
                        let d = body.pos.distance(start_world);
                        if d < nearest_dist {
                            nearest_dist = d;
                            nearest_pos = body.pos;
                            nearest_mass = body.mass;
                            nearest_vel = body.vel;
                        }
                    }

                    for moon in &self.moons {
                        let d = moon.body.pos.distance(start_world);
                        if d < nearest_dist {
                            nearest_dist = d;
                            nearest_pos = moon.body.pos;
                            nearest_mass = moon.body.mass;
                            nearest_vel = moon.body.vel;
                        }
                    }

                    let to_body = start_world - nearest_pos;
                    let dist = to_body.length().max(30.0);
                    let tangent = Vec2::new(-to_body.y, to_body.x).normalized();
                    let orbit_speed = (GRAVITY_SCALE * nearest_mass / dist).sqrt();
                    let drag_scale = (drag_screen.length() / 120.0).clamp(0.8, 1.6);
                    let body_screen = self.camera.world_to_screen(
                        nearest_pos,
                        self.screen_size.x as u32,
                        self.screen_size.y as u32,
                    );
                    let to_body_screen = start_screen - body_screen;
                    let cross = to_body_screen.x * drag_screen.y - to_body_screen.y * drag_screen.x;
                    let direction = if cross >= 0.0 { 1.0 } else { -1.0 };
                    let vel = tangent * (orbit_speed * drag_scale * direction)
                        + if self.nbody_mode { nearest_vel } else { Vec2::ZERO };

                    self.comets.push(Comet {
                        pos: start_world,
                        prev_pos: start_world,
                        vel,
                        life: 30.0,
                    });
                } else {
                    let mut picked = None;
                    for (idx, body) in self.bodies.iter().enumerate() {
                        if body.radius == 0.0 {
                            continue;
                        }
                        if body.pos.distance(start_world) <= body.size * 0.6 {
                            picked = Some(idx);
                            break;
                        }
                    }
                    self.selected_body = picked;
                }
            }
        }

        if input.is_key_down(KeyCode::Equal) || input.is_key_down(KeyCode::NumpadAdd) {
            self.zoom_target = (self.zoom_target * 1.01).min(2.8);
        }
        if input.is_key_down(KeyCode::Minus) || input.is_key_down(KeyCode::NumpadSubtract) {
            self.zoom_target = (self.zoom_target * 0.99).max(0.5);
        }

        if let Some(selected) = self.selected_body {
            if !self.nbody_mode {
                if input.is_key_down(KeyCode::BracketLeft) {
                    self.bodies[selected].radius = (self.bodies[selected].radius - 30.0).max(80.0);
                }
                if input.is_key_down(KeyCode::BracketRight) {
                    self.bodies[selected].radius = (self.bodies[selected].radius + 30.0).min(520.0);
                }
                if input.is_key_down(KeyCode::Semicolon) {
                    self.bodies[selected].angular_speed = (self.bodies[selected].angular_speed - 0.15).max(0.05);
                }
                if input.is_key_down(KeyCode::Quote) {
                    self.bodies[selected].angular_speed = (self.bodies[selected].angular_speed + 0.15).min(1.6);
                }
            }
        }

        let zoom_speed = 3.0;
        let diff = self.zoom_target - self.zoom;
        self.zoom += diff * zoom_speed * ctx.delta_seconds();
        self.camera.zoom = self.zoom;

        if self.follow_selected {
            if let Some(index) = self.selected_body {
                let target = self.bodies[index].pos;
                self.camera.position = target;
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let alpha = ctx.fixed_update_alpha();

        ctx.draw(|renderer, frame| {
            renderer.clear(frame, [0.02, 0.02, 0.05, 1.0])?;

            for (pos, color) in self.starfield.iter().zip(self.star_colors.iter()) {
                renderer.draw_circle(frame, *pos, 1.2, *color, &self.camera)?;
            }

            for body in &self.bodies {
                if body.radius <= 0.0 {
                    continue;
                }
                for i in 0..18 {
                    let angle = (i as f32 / 18.0) * std::f32::consts::TAU;
                    let pos = Vec2::new(
                        self.center.x + angle.cos() * body.radius,
                        self.center.y + angle.sin() * body.radius,
                    );
                    renderer.draw_circle(frame, pos, 1.2, [0.2, 0.3, 0.6, 0.35], &self.camera)?;
                }
            }

            for body in &self.bodies {
                let pos = body.prev_pos.lerp(body.pos, alpha);
                renderer.draw_circle(frame, pos, body.size * 0.5, body.color, &self.camera)?;
            }

            for moon in &self.moons {
                let pos = moon.body.prev_pos.lerp(moon.body.pos, alpha);
                renderer.draw_circle(frame, pos, moon.body.size * 0.5, moon.body.color, &self.camera)?;
            }

            for asteroid in &self.asteroids {
                let pos = asteroid.prev_pos.lerp(asteroid.pos, alpha);
                renderer.draw_circle(frame, pos, asteroid.size * 0.5, asteroid.color, &self.camera)?;
            }

            for comet in &self.comets {
                let pos = comet.prev_pos.lerp(comet.pos, alpha);
                renderer.draw_circle(frame, pos, 4.0, [0.8, 0.9, 1.0, 0.9], &self.camera)?;
            }

            if let Some(index) = self.selected_body {
                let body = &self.bodies[index];
                let pos = body.prev_pos.lerp(body.pos, alpha);
                renderer.draw_circle(
                    frame,
                    pos,
                    body.size * 0.6 + 6.0,
                    [1.0, 1.0, 1.0, 0.35],
                    &self.camera,
                )?;
            }

            Ok(())
        })
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Solar Drift")
        .with_size(960, 540)
        .run(SolarSystem::new())
}
