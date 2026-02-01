use anyhow::Result;
use forge2d::{
    math::{Camera2D, Vec2},
    physics::{ColliderShape, PhysicsEvent, PhysicsWorld, RigidBodyType},
    render::{Renderer, Sprite, TextureHandle},
    scene::{create_scene, restore_scene_physics, Scene},
    Engine, Game, KeyCode,
};
use std::collections::HashSet;

struct PhysicsDemo {
    camera: Camera2D,
    physics: PhysicsWorld,
    world: forge2d::World,

    textures: TextureSet,
    entities: Vec<PhysicsEntity>,

    // for tint feedback
    colliding_entities: HashSet<forge2d::EntityId>,

    last_spawn_time: std::time::Instant,

    // Track static entities separately so they don't get deleted on load
    ground_entity: Option<forge2d::EntityId>,
    sensor_entity: Option<forge2d::EntityId>,

    // Debug tracking
    debug_entity: Option<forge2d::EntityId>,
    debug_frame_count: u32,
}

struct TextureSet {
    ground: Option<TextureHandle>,
    box_normal: Option<TextureHandle>,
    box_bouncy: Option<TextureHandle>,
    box_slippery: Option<TextureHandle>,
    circle: Option<TextureHandle>,
    capsule: Option<TextureHandle>,
    sensor: Option<TextureHandle>,
}

struct PhysicsEntity {
    entity: forge2d::EntityId,
    shape: ShapeType,
    material: MaterialType,
    is_sensor: bool,
}

#[derive(Clone, Copy)]
enum ShapeType {
    Box,
    Circle,
    Capsule,
}

#[derive(Clone, Copy)]
enum MaterialType {
    Normal,
    Bouncy,
    Slippery,
}

impl PhysicsDemo {
    fn new() -> Self {
        let mut physics = PhysicsWorld::new();
        physics.set_gravity(Vec2::new(0.0, 400.0));

        Self {
            camera: Camera2D::default(),
            physics,
            world: forge2d::World::new(),
            textures: TextureSet {
                ground: None,
                box_normal: None,
                box_bouncy: None,
                box_slippery: None,
                circle: None,
                capsule: None,
                sensor: None,
            },
            entities: Vec::new(),
            colliding_entities: HashSet::new(),
            last_spawn_time: std::time::Instant::now(),
            ground_entity: None,
            sensor_entity: None,
            debug_entity: None,
            debug_frame_count: 0,
        }
    }

    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        // Ground (dark gray)
        let ground_data: Vec<u8> = (0..(4 * 600 * 30))
            .flat_map(|_| [80u8, 80, 80, 255])
            .collect();
        self.textures.ground = Some(renderer.load_texture_from_rgba(&ground_data, 600, 30)?);

        // Normal box (red)
        let box_data: Vec<u8> = (0..(4 * 30 * 30))
            .flat_map(|_| [255u8, 80, 80, 255])
            .collect();
        self.textures.box_normal = Some(renderer.load_texture_from_rgba(&box_data, 30, 30)?);

        // Bouncy box (yellow)
        let bouncy_data: Vec<u8> = (0..(4 * 30 * 30))
            .flat_map(|_| [255u8, 255, 100, 255])
            .collect();
        self.textures.box_bouncy = Some(renderer.load_texture_from_rgba(&bouncy_data, 30, 30)?);

        // Slippery box (blue)
        let slippery_data: Vec<u8> = (0..(4 * 30 * 30))
            .flat_map(|_| [100u8, 150, 255, 255])
            .collect();
        self.textures.box_slippery =
            Some(renderer.load_texture_from_rgba(&slippery_data, 30, 30)?);

        // Circle (green)
        let circle_size = 30;
        let mut circle_data = vec![0u8; 4 * circle_size * circle_size];
        let center = circle_size as f32 / 2.0;
        let radius = center - 2.0;
        for y in 0..circle_size {
            for x in 0..circle_size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                if dx * dx + dy * dy <= radius * radius {
                    let idx = ((y * circle_size + x) * 4) as usize;
                    circle_data[idx] = 100;
                    circle_data[idx + 1] = 255;
                    circle_data[idx + 2] = 100;
                    circle_data[idx + 3] = 255;
                }
            }
        }
        self.textures.circle = Some(renderer.load_texture_from_rgba(
            &circle_data,
            circle_size as u32,
            circle_size as u32,
        )?);

        // Capsule (purple)
        let capsule_data: Vec<u8> = (0..(4 * 40 * 20))
            .flat_map(|_| [200u8, 100, 255, 255])
            .collect();
        self.textures.capsule = Some(renderer.load_texture_from_rgba(&capsule_data, 40, 20)?);

        // Sensor (semi-transparent cyan)
        let sensor_data: Vec<u8> = (0..(4 * 50 * 50))
            .flat_map(|_| [100u8, 255, 255, 128])
            .collect();
        self.textures.sensor = Some(renderer.load_texture_from_rgba(&sensor_data, 50, 50)?);

        Ok(())
    }

    fn spawn_object(
        &mut self,
        position: Vec2,
        shape: ShapeType,
        material: MaterialType,
        is_sensor: bool,
    ) -> Result<()> {
        let entity = self.world.spawn();
        self.physics
            .create_body(entity, RigidBodyType::Dynamic, position, 0.0)?;

        let (shape_obj, _size) = match shape {
            ShapeType::Box => (
                ColliderShape::Box { hx: 15.0, hy: 15.0 },
                Vec2::new(30.0, 30.0),
            ),
            ShapeType::Circle => (
                ColliderShape::Circle { radius: 15.0 },
                Vec2::new(30.0, 30.0),
            ),
            ShapeType::Capsule => (
                ColliderShape::CapsuleY {
                    half_height: 8.0,
                    radius: 10.0,
                },
                Vec2::new(40.0, 20.0),
            ),
        };

        if is_sensor {
            self.physics.add_sensor(entity, shape_obj, Vec2::ZERO)?;
        } else {
            let (friction, restitution) = match material {
                MaterialType::Normal => (0.5, 0.0),
                MaterialType::Bouncy => (0.5, 0.8),
                MaterialType::Slippery => (0.1, 0.0),
            };

            self.physics.add_collider_with_material(
                entity,
                shape_obj,
                Vec2::ZERO,
                1.0,
                friction,
                restitution,
            )?;

            self.physics
                .set_angular_velocity(entity, (rand::random::<f32>() - 0.5) * 5.0);
            self.physics.set_linear_damping(entity, 0.1);
            self.physics.set_angular_damping(entity, 0.2);
        }

        self.entities.push(PhysicsEntity {
            entity,
            shape,
            material,
            is_sensor,
        });

        Ok(())
    }
}

impl Game for PhysicsDemo {
    fn init(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        self.camera = ctx.screen_camera();
        self.create_textures(&mut *ctx.renderer())?;

        let screen_size = ctx.window().inner_size();
        let screen_w = screen_size.width as f32;
        let screen_h = screen_size.height as f32;

        // Ground
        let ground_entity = self.world.spawn();
        self.ground_entity = Some(ground_entity);
        let ground_y = screen_h - 80.0;
        self.physics.create_body(
            ground_entity,
            RigidBodyType::Fixed,
            Vec2::new(screen_w / 2.0, ground_y),
            0.0,
        )?;
        self.physics.add_collider_with_material(
            ground_entity,
            ColliderShape::Box {
                hx: 300.0,
                hy: 15.0,
            },
            Vec2::ZERO,
            0.0,
            0.7,
            0.2,
        )?;

        // Sensor zone (middle)
        let sensor_entity = self.world.spawn();
        self.sensor_entity = Some(sensor_entity);
        self.physics.create_body(
            sensor_entity,
            RigidBodyType::Fixed,
            Vec2::new(screen_w / 2.0, screen_h / 2.0),
            0.0,
        )?;
        self.physics.add_sensor(
            sensor_entity,
            ColliderShape::Circle { radius: 25.0 },
            Vec2::ZERO,
        )?;
        self.entities.push(PhysicsEntity {
            entity: sensor_entity,
            shape: ShapeType::Circle,
            material: MaterialType::Normal,
            is_sensor: true,
        });

        // Spawn initial objects
        for i in 0..3 {
            self.spawn_object(
                Vec2::new(screen_w * 0.2 + i as f32 * 40.0, 100.0),
                ShapeType::Box,
                MaterialType::Bouncy,
                false,
            )?;
            self.spawn_object(
                Vec2::new(screen_w * 0.5 + i as f32 * 40.0, 150.0),
                ShapeType::Box,
                MaterialType::Normal,
                false,
            )?;
            self.spawn_object(
                Vec2::new(screen_w * 0.8 + i as f32 * 40.0, 200.0),
                ShapeType::Box,
                MaterialType::Slippery,
                false,
            )?;
        }

        for i in 0..2 {
            self.spawn_object(
                Vec2::new(screen_w * 0.3 + i as f32 * 60.0, 250.0),
                ShapeType::Circle,
                MaterialType::Normal,
                false,
            )?;
            self.spawn_object(
                Vec2::new(screen_w * 0.6 + i as f32 * 60.0, 300.0),
                ShapeType::Capsule,
                MaterialType::Bouncy,
                false,
            )?;
        }

        Ok(())
    }

    fn update(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        let mouse_world = ctx.mouse_world(&self.camera);

        // Spawn on left click
        {
            let input = ctx.input();
            if input.is_mouse_pressed(forge2d::MouseButton::Left) {
                let now = std::time::Instant::now();
                if now.duration_since(self.last_spawn_time).as_millis() > 200 {
                    self.last_spawn_time = now;

                    let shape = match rand::random::<u8>() % 3 {
                        0 => ShapeType::Box,
                        1 => ShapeType::Circle,
                        _ => ShapeType::Capsule,
                    };

                    let material = match rand::random::<u8>() % 3 {
                        0 => MaterialType::Normal,
                        1 => MaterialType::Bouncy,
                        _ => MaterialType::Slippery,
                    };

                    let _ = self.spawn_object(mouse_world, shape, material, false);
                }
            }
        }

        // Fixed-step physics
        while ctx.should_run_fixed_update() {
            let dt = ctx.fixed_delta_time().as_secs_f32();
            self.physics.step(dt);

            // Intensive debugging for selected entity
            if let Some(debug_entity) = self.debug_entity {
                self.debug_frame_count += 1;
                if self.debug_frame_count % 10 == 0 {
                    // Print every 10 frames
                    if let Some(pos) = self.physics.body_position(debug_entity) {
                        if let Some(vel) = self.physics.linear_velocity(debug_entity) {
                            if let Some(ground) = self.ground_entity {
                                if let Some(ground_pos) = self.physics.body_position(ground) {
                                    let dist_to_ground = pos.y - ground_pos.y;
                                    let colliders = self.physics.get_colliders(debug_entity);
                                    let ground_colliders = self.physics.get_colliders(ground);

                                    println!(
                                        "[Frame {}] Entity {:?}: pos={:?}, vel={:?}, dist_to_ground={:.2}, colliders={}, ground_colliders={}",
                                        self.debug_frame_count,
                                        debug_entity,
                                        pos,
                                        vel,
                                        dist_to_ground,
                                        colliders.len(),
                                        ground_colliders.len(),
                                    );

                                    // Check if we're very close to ground but not colliding
                                    if dist_to_ground < 50.0
                                        && dist_to_ground > -10.0
                                        && vel.y > 0.0
                                    {
                                        eprintln!("  ⚠️  WARNING: Falling through ground! dist={:.2}, vel_y={:.2}", dist_to_ground, vel.y);
                                    }

                                    // Check if we should have collided but didn't
                                    if dist_to_ground < 20.0 && vel.y > 100.0 {
                                        eprintln!("  ⚠️  WARNING: Fast fall near ground but no collision detected!");
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // collision tinting via events
            for ev in self.physics.drain_events() {
                match ev {
                    PhysicsEvent::CollisionEnter { a, b } | PhysicsEvent::TriggerEnter { a, b } => {
                        self.colliding_entities.insert(a);
                        self.colliding_entities.insert(b);
                    }
                    PhysicsEvent::CollisionExit { a, b } | PhysicsEvent::TriggerExit { a, b } => {
                        self.colliding_entities.remove(&a);
                        self.colliding_entities.remove(&b);
                    }
                }
            }
        }

        // Right click impulse
        {
            let input = ctx.input();
            if input.is_mouse_pressed(forge2d::MouseButton::Right) {
                for e in &self.entities {
                    if e.is_sensor {
                        continue;
                    }
                    if let Some(pos) = self.physics.body_position(e.entity) {
                        let dist = pos.distance(mouse_world);
                        if dist < 100.0 {
                            let dir = (pos - mouse_world).normalized();
                            self.physics.apply_impulse(e.entity, dir * 500.0);
                        }
                    }
                }
            }
        }

        // Save/Load scene (S to save, L to load)
        {
            let input = ctx.input();
            if input.is_key_pressed(KeyCode::KeyS) {
                // Only save entities that are in self.entities (exclude ground and sensor)
                let mut scene = create_scene(&self.physics);
                // Filter out static entities from saved scene
                let ground_entity = self.ground_entity;
                let sensor_entity = self.sensor_entity;
                scene
                    .physics
                    .bodies
                    .retain(|b| Some(b.entity) != ground_entity && Some(b.entity) != sensor_entity);
                scene
                    .physics
                    .colliders
                    .retain(|c| Some(c.entity) != ground_entity && Some(c.entity) != sensor_entity);

                match scene.save_to_file(std::path::Path::new("physics_scene.json")) {
                    Ok(_) => println!(
                        "Scene saved to physics_scene.json ({} entities)",
                        scene.physics.bodies.len()
                    ),
                    Err(e) => eprintln!("Failed to save scene: {}", e),
                }
            }
            if input.is_key_pressed(KeyCode::KeyL) {
                match Scene::load_from_file(std::path::Path::new("physics_scene.json")) {
                    Ok(scene) => {
                        // ChatGPT's solution: Completely clear physics world and rebuild everything
                        // This avoids stale broad-phase state in Rapier
                        let screen_size = ctx.window().inner_size();
                        let screen_w = screen_size.width as f32;
                        let screen_h = screen_size.height as f32;

                        // Save gravity before clearing
                        let saved_gravity = self.physics.gravity();

                        // Clear ALL entities from World
                        let all_entities: Vec<_> = self.entities.iter().map(|e| e.entity).collect();
                        for entity in &all_entities {
                            self.world.despawn(*entity);
                        }
                        if let Some(ground) = self.ground_entity {
                            self.world.despawn(ground);
                        }
                        if let Some(sensor) = self.sensor_entity {
                            self.world.despawn(sensor);
                        }
                        self.entities.clear();

                        // Completely clear and rebuild physics world (this is the key fix!)
                        self.physics.clear();
                        self.physics.set_gravity(saved_gravity);

                        // Create new World entities and remap scene data
                        let mut id_mapping: std::collections::HashMap<
                            forge2d::EntityId,
                            forge2d::EntityId,
                        > = std::collections::HashMap::new();
                        let mut remapped_scene = scene.clone();

                        // Collect all unique entity IDs
                        let mut all_entity_ids: std::collections::HashSet<forge2d::EntityId> =
                            std::collections::HashSet::new();
                        for body in &remapped_scene.physics.bodies {
                            all_entity_ids.insert(body.entity);
                        }
                        for collider in &remapped_scene.physics.colliders {
                            all_entity_ids.insert(collider.entity);
                        }

                        // Create new World entities
                        for old_entity in &all_entity_ids {
                            let new_entity = self.world.spawn();
                            id_mapping.insert(*old_entity, new_entity);
                        }

                        // Remap scene data
                        for body in &mut remapped_scene.physics.bodies {
                            if let Some(&new_entity) = id_mapping.get(&body.entity) {
                                body.entity = new_entity;
                            }
                        }
                        for collider in &mut remapped_scene.physics.colliders {
                            if let Some(&new_entity) = id_mapping.get(&collider.entity) {
                                collider.entity = new_entity;
                            }
                        }

                        // Restore physics (no preservation - everything is fresh!)
                        if let Err(e) = restore_scene_physics(&mut self.physics, &remapped_scene) {
                            eprintln!("Failed to restore physics: {}", e);
                        } else {
                            // CRITICAL: Recreate ground and sensor AFTER restore_scene_physics
                            // (restore_scene_physics clears all bodies, so we must recreate them after)

                            // Recreate ground
                            let ground_entity = self.world.spawn();
                            self.ground_entity = Some(ground_entity);
                            let ground_y = screen_h - 80.0;
                            if let Err(e) = self.physics.create_body(
                                ground_entity,
                                RigidBodyType::Fixed,
                                Vec2::new(screen_w / 2.0, ground_y),
                                0.0,
                            ) {
                                eprintln!("Failed to recreate ground: {}", e);
                            } else if let Err(e) = self.physics.add_collider_with_material(
                                ground_entity,
                                ColliderShape::Box {
                                    hx: 300.0,
                                    hy: 15.0,
                                },
                                Vec2::ZERO,
                                0.0,
                                0.7,
                                0.2,
                            ) {
                                eprintln!("Failed to recreate ground collider: {}", e);
                            }

                            // Recreate sensor
                            let sensor_entity = self.world.spawn();
                            self.sensor_entity = Some(sensor_entity);
                            if let Err(e) = self.physics.create_body(
                                sensor_entity,
                                RigidBodyType::Fixed,
                                Vec2::new(screen_w / 2.0, screen_h / 2.0),
                                0.0,
                            ) {
                                eprintln!("Failed to recreate sensor: {}", e);
                            } else if let Err(e) = self.physics.add_sensor(
                                sensor_entity,
                                ColliderShape::Circle { radius: 25.0 },
                                Vec2::ZERO,
                            ) {
                                eprintln!("Failed to recreate sensor collider: {}", e);
                            }
                            self.entities.push(PhysicsEntity {
                                entity: sensor_entity,
                                shape: ShapeType::Circle,
                                material: MaterialType::Normal,
                                is_sensor: true,
                            });

                            // Debug: Check physics state after full restore
                            println!("=== Physics State After Full Restore ===");
                            let body_count = self.physics.all_entities_with_bodies().len();
                            let mut total_colliders = 0;
                            for entity in self.physics.all_entities_with_bodies() {
                                total_colliders += self.physics.get_colliders(entity).len();
                            }
                            println!(
                                "Total bodies: {}, Total colliders: {}",
                                body_count, total_colliders
                            );

                            // Check ground
                            if let Some(ground) = self.ground_entity {
                                let ground_colliders = self.physics.get_colliders(ground);
                                println!(
                                    "Ground entity {:?}: {} colliders",
                                    ground,
                                    ground_colliders.len()
                                );
                                for (
                                    _shape,
                                    _offset,
                                    _density,
                                    _friction,
                                    _restitution,
                                    is_sensor,
                                ) in &ground_colliders
                                {
                                    if *is_sensor {
                                        eprintln!("ERROR: Ground collider is a SENSOR!");
                                    }
                                }
                            }

                            // Check sensor
                            if let Some(sensor) = self.sensor_entity {
                                let sensor_colliders = self.physics.get_colliders(sensor);
                                println!(
                                    "Sensor entity {:?}: {} colliders",
                                    sensor,
                                    sensor_colliders.len()
                                );
                            }
                            println!("=== End Physics State Check ===");
                            // Debug: Check body types and compare to spawn behavior
                            println!("=== Body Type Check After Restore ===");
                            for body_data in &remapped_scene.physics.bodies {
                                if let Some(actual_type) = self.physics.body_type(body_data.entity)
                                {
                                    println!(
                                        "Entity {:?}: saved={:?}, actual={:?}, pos={:?}, rot={:?}",
                                        body_data.entity,
                                        body_data.body_type,
                                        actual_type,
                                        body_data.position,
                                        body_data.rotation
                                    );
                                    if body_data.body_type != actual_type {
                                        eprintln!(
                                            "ERROR: Body type mismatch! Saved {:?} but got {:?}",
                                            body_data.body_type, actual_type
                                        );
                                    }

                                    // Check collider properties
                                    let colliders = self.physics.get_colliders(body_data.entity);
                                    for (
                                        shape,
                                        _offset,
                                        density,
                                        friction,
                                        restitution,
                                        is_sensor,
                                    ) in &colliders
                                    {
                                        println!("  Collider: shape={:?}, sensor={}, density={}, friction={}, restitution={}", 
                                            shape, is_sensor, density, friction, restitution);
                                    }
                                } else {
                                    eprintln!(
                                        "ERROR: Could not get body type for entity {:?}",
                                        body_data.entity
                                    );
                                }
                            }
                            println!("=== End Body Type Check ===");

                            // Spawn a test object to compare (but don't keep it - just for debugging)
                            println!("=== Spawning Test Object for Comparison ===");
                            let test_pos = Vec2::new(screen_w / 2.0, 100.0);
                            let test_entity = self.world.spawn();
                            if let Err(e) = self.physics.create_body(
                                test_entity,
                                RigidBodyType::Dynamic,
                                test_pos,
                                0.0,
                            ) {
                                eprintln!("Failed to create test body: {}", e);
                            } else if let Err(e) = self.physics.add_collider_with_material(
                                test_entity,
                                ColliderShape::Box { hx: 15.0, hy: 15.0 },
                                Vec2::ZERO,
                                1.0,
                                0.5,
                                0.0,
                            ) {
                                eprintln!("Failed to create test collider: {}", e);
                            } else {
                                self.physics.set_linear_damping(test_entity, 0.1);
                                self.physics.set_angular_damping(test_entity, 0.2);

                                let test_colliders = self.physics.get_colliders(test_entity);
                                println!("Test object entity {:?}:", test_entity);
                                for (shape, _offset, density, friction, restitution, is_sensor) in
                                    &test_colliders
                                {
                                    println!("  Collider: shape={:?}, sensor={}, density={}, friction={}, restitution={}", 
                                        shape, is_sensor, density, friction, restitution);
                                }

                                // Remove test object immediately (it was just for comparison)
                                self.physics.remove_body(test_entity);
                                self.world.despawn(test_entity);
                            }
                            println!("=== End Test Object Check ===");

                            // Pick one object near center for intensive debugging
                            let debug_entity = remapped_scene
                                .physics
                                .bodies
                                .iter()
                                .find(|b| {
                                    // Find an object near center (x around 640, y should be above ground ~640)
                                    let center_x = screen_w / 2.0;
                                    let dist_from_center = (b.position.x - center_x).abs();
                                    dist_from_center < 200.0 && b.position.y < 500.0
                                })
                                .map(|b| b.entity);

                            if let Some(debug_entity) = debug_entity {
                                println!("\n=== INTENSIVE DEBUG FOR ENTITY {:?} ===", debug_entity);

                                // Initial state
                                if let Some(pos) = self.physics.body_position(debug_entity) {
                                    println!("Initial position: {:?}", pos);
                                }
                                if let Some(rot) = self.physics.body_rotation(debug_entity) {
                                    println!("Initial rotation: {}", rot);
                                }
                                if let Some(vel) = self.physics.linear_velocity(debug_entity) {
                                    println!("Initial velocity: {:?}", vel);
                                }
                                if let Some(ang_vel) = self.physics.angular_velocity(debug_entity) {
                                    println!("Initial angular velocity: {}", ang_vel);
                                }

                                let colliders = self.physics.get_colliders(debug_entity);
                                println!("Collider count: {}", colliders.len());
                                for (
                                    i,
                                    (shape, offset, density, friction, restitution, is_sensor),
                                ) in colliders.iter().enumerate()
                                {
                                    println!("  Collider {}: shape={:?}, offset={:?}, density={}, friction={}, restitution={}, sensor={}", 
                                        i, shape, offset, density, friction, restitution, is_sensor);
                                }

                                // Ground info
                                if let Some(ground) = self.ground_entity {
                                    if let Some(ground_pos) = self.physics.body_position(ground) {
                                        println!("Ground position: {:?}", ground_pos);
                                        if let Some(debug_pos) =
                                            self.physics.body_position(debug_entity)
                                        {
                                            let dist = (debug_pos.y - ground_pos.y).abs();
                                            println!("Distance from ground: {}", dist);
                                        }

                                        let ground_colliders = self.physics.get_colliders(ground);
                                        println!(
                                            "Ground collider count: {}",
                                            ground_colliders.len()
                                        );
                                        for (
                                            i,
                                            (
                                                shape,
                                                offset,
                                                density,
                                                friction,
                                                restitution,
                                                is_sensor,
                                            ),
                                        ) in ground_colliders.iter().enumerate()
                                        {
                                            println!("  Ground collider {}: shape={:?}, offset={:?}, density={}, friction={}, restitution={}, sensor={}", 
                                                i, shape, offset, density, friction, restitution, is_sensor);
                                        }
                                    }
                                }

                                // Store debug entity for tracking
                                self.debug_entity = Some(debug_entity);
                                println!("=== END INITIAL DEBUG ===\n");
                            }

                            // Recreate entity tracking from colliders (they have shape info)
                            // Build a map of entity -> collider info
                            let mut entity_colliders: std::collections::HashMap<_, _> =
                                std::collections::HashMap::new();
                            for collider in &remapped_scene.physics.colliders {
                                entity_colliders
                                    .entry(collider.entity)
                                    .or_insert_with(Vec::new)
                                    .push(collider);
                            }

                            // Recreate entities from physics bodies and colliders
                            let current_ground = self.ground_entity;
                            let current_sensor = self.sensor_entity;
                            for body in &remapped_scene.physics.bodies {
                                // Skip static entities - they're already tracked
                                if Some(body.entity) == current_ground
                                    || Some(body.entity) == current_sensor
                                {
                                    continue;
                                }

                                // Infer shape from collider
                                let shape = entity_colliders
                                    .get(&body.entity)
                                    .and_then(|colliders| colliders.first())
                                    .map(|c| match c.shape {
                                        ColliderShape::Box { .. } => ShapeType::Box,
                                        ColliderShape::Circle { .. } => ShapeType::Circle,
                                        ColliderShape::CapsuleY { .. } => ShapeType::Capsule,
                                    })
                                    .unwrap_or(ShapeType::Box);

                                // Infer material from friction/restitution
                                let material = entity_colliders
                                    .get(&body.entity)
                                    .and_then(|colliders| colliders.first())
                                    .map(|c| {
                                        if c.restitution > 0.5 {
                                            MaterialType::Bouncy
                                        } else if c.friction < 0.2 {
                                            MaterialType::Slippery
                                        } else {
                                            MaterialType::Normal
                                        }
                                    })
                                    .unwrap_or(MaterialType::Normal);

                                let is_sensor = entity_colliders
                                    .get(&body.entity)
                                    .and_then(|colliders| colliders.first())
                                    .map(|c| c.is_sensor)
                                    .unwrap_or(false);

                                self.entities.push(PhysicsEntity {
                                    entity: body.entity,
                                    shape,
                                    material,
                                    is_sensor,
                                });
                            }
                            println!(
                                "Scene loaded from physics_scene.json ({} entities)",
                                self.entities.len()
                            );

                            // Debug: Check for duplicate entities or entities without visuals
                            let mut entity_set = std::collections::HashSet::new();
                            for e in &self.entities {
                                if entity_set.contains(&e.entity) {
                                    eprintln!(
                                        "WARNING: Duplicate entity {:?} in entities list!",
                                        e.entity
                                    );
                                }
                                entity_set.insert(e.entity);

                                // Check if entity has physics body
                                if self.physics.body_position(e.entity).is_none() {
                                    eprintln!("WARNING: Entity {:?} in entities list has no physics body!", e.entity);
                                }
                            }

                            // Check if all physics bodies have corresponding entities
                            // (exclude ground and sensor - they're handled separately)
                            for entity in self.physics.all_entities_with_bodies() {
                                // Ground and sensor are rendered separately, not through entities list
                                if Some(entity) == self.ground_entity
                                    || Some(entity) == self.sensor_entity
                                {
                                    continue;
                                }

                                if !entity_set.contains(&entity) {
                                    eprintln!("WARNING: Physics body for entity {:?} has no corresponding entity in entities list!", entity);
                                    if let Some(pos) = self.physics.body_position(entity) {
                                        eprintln!("  Position: {:?}", pos);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to load scene: {}", e),
                }
            }
        }

        // WASD forces
        let input = ctx.input();
        let force_dir = Vec2::new(
            if input.is_key_down(KeyCode::KeyD) {
                1.0
            } else if input.is_key_down(KeyCode::KeyA) {
                -1.0
            } else {
                0.0
            },
            if input.is_key_down(KeyCode::KeyS) {
                1.0
            } else if input.is_key_down(KeyCode::KeyW) {
                -1.0
            } else {
                0.0
            },
        );

        if force_dir.length() > 0.0 {
            let force = force_dir.normalized() * 200.0;
            for e in &self.entities {
                if !e.is_sensor {
                    self.physics.apply_force(e.entity, force);
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        let screen_size = ctx.window().inner_size();
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        renderer.clear(&mut frame, [0.05, 0.05, 0.1, 1.0])?;

        // Ground
        if let Some(tex) = self.textures.ground {
            let ground_y = (screen_size.height as f32) - 80.0;
            let mut sprite = Sprite::new(tex);
            sprite.transform.position = Vec2::new(screen_size.width as f32 / 2.0, ground_y);
            sprite.set_size_px(Vec2::new(600.0, 30.0), Vec2::new(600.0, 30.0));
            renderer.draw_sprite(&mut frame, &sprite, &self.camera)?;
        }

        // Entities
        for e in &self.entities {
            let tex = match (e.shape, e.material, e.is_sensor) {
                (_, _, true) => self.textures.sensor,
                (ShapeType::Box, MaterialType::Normal, _) => self.textures.box_normal,
                (ShapeType::Box, MaterialType::Bouncy, _) => self.textures.box_bouncy,
                (ShapeType::Box, MaterialType::Slippery, _) => self.textures.box_slippery,
                (ShapeType::Circle, _, _) => self.textures.circle,
                (ShapeType::Capsule, _, _) => self.textures.capsule,
            };

            if let Some(tex) = tex {
                if let Some(pos) = self.physics.body_position(e.entity) {
                    if let Some(rot) = self.physics.body_rotation(e.entity) {
                        let mut sprite = Sprite::new(tex);
                        sprite.transform.position = pos;
                        sprite.transform.rotation = rot;

                        let size = match e.shape {
                            ShapeType::Box => Vec2::new(30.0, 30.0),
                            ShapeType::Circle => Vec2::new(30.0, 30.0),
                            ShapeType::Capsule => Vec2::new(40.0, 20.0),
                        };
                        sprite.set_size_px(size, size);

                        if self.colliding_entities.contains(&e.entity) {
                            sprite.tint = [1.5, 1.5, 1.5, 1.0];
                        } else if e.is_sensor {
                            sprite.tint = [1.0, 1.0, 1.0, 0.5];
                        } else {
                            sprite.tint = [1.0, 1.0, 1.0, 1.0];
                        }

                        renderer.draw_sprite(&mut frame, &sprite, &self.camera)?;
                    }
                }
            }
        }

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Forge2D Physics Showcase")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(PhysicsDemo::new())
}
