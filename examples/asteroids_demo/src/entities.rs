use sindri::{Sprite, Vec2};

pub struct Player {
    pub sprite: Sprite,
    pub velocity: Vec2,
    pub rotation: f32,
    pub rotation_speed: f32,
    pub thrust_power: f32,
    pub max_speed: f32,
    pub friction: f32,
}

impl Player {
    pub fn new(sprite: Sprite) -> Self {
        Self {
            sprite,
            velocity: Vec2::ZERO,
            rotation: 0.0, // Point right initially (matches Vec2::from_angle(0) = right)
            rotation_speed: 5.0, // radians per second
            thrust_power: 200.0,
            max_speed: 300.0,
            friction: 0.98,
        }
    }
}

pub struct Bullet {
    pub sprite: Sprite,
    pub velocity: Vec2,
    pub lifetime: f32,
}

impl Bullet {
    pub fn new(sprite: Sprite, position: Vec2, direction: Vec2, speed: f32) -> Self {
        let mut bullet = Self {
            sprite,
            velocity: direction.normalized() * speed,
            lifetime: 2.0, // seconds
        };
        bullet.sprite.transform.position = position;
        bullet.sprite.transform.rotation = direction.y.atan2(direction.x);
        bullet
    }
}

pub struct Asteroid {
    pub sprite: Sprite,
    pub velocity: Vec2,
    pub size: AsteroidSize,
    pub rotation: f32,
    pub rotation_speed: f32,
    pub shape_offsets: Vec<Vec2>, // Pre-computed shape offsets (relative to center)
}

#[derive(Clone, Copy, PartialEq)]
pub enum AsteroidSize {
    Large,
    Medium,
    Small,
}

impl Asteroid {
    pub fn new(sprite: Sprite, position: Vec2, size: AsteroidSize) -> Self {
        let (speed, rot_speed) = match size {
            AsteroidSize::Large => (50.0, 0.5),
            AsteroidSize::Medium => (80.0, 1.0),
            AsteroidSize::Small => (120.0, 2.0),
        };

        // Random direction
        let angle = fastrand::f32() * std::f32::consts::TAU;
        let velocity = Vec2::from_angle(angle) * speed;

        // Generate shape offsets deterministically based on position
        let radius = match size {
            AsteroidSize::Large => 40.0,
            AsteroidSize::Medium => 20.0,
            AsteroidSize::Small => 10.0,
        };
        let shape_offsets = Self::generate_shape_offsets(position, radius);

        Self {
            sprite,
            velocity,
            size,
            rotation: 0.0,
            rotation_speed: rot_speed * (if fastrand::bool() { 1.0 } else { -1.0 }),
            shape_offsets,
        }
    }

    fn generate_shape_offsets(position: Vec2, radius: f32) -> Vec<Vec2> {
        const POINTS: usize = 10; // Good balance between detail and simplicity
        let mut offsets = Vec::with_capacity(POINTS);

        // Simple hash function for deterministic "randomness" based on position
        fn hash(x: u64) -> u64 {
            let mut x = x;
            x ^= x >> 33;
            x = x.wrapping_mul(0xff51afd7ed558ccd);
            x ^= x >> 33;
            x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
            x ^= x >> 33;
            x
        }

        let base_seed = ((position.x * 1000.0) as u64) ^ ((position.y * 1000.0) as u64);

        // Generate points in counter-clockwise order around a circle
        // with controlled variation to create interesting but valid shapes
        for i in 0..POINTS {
            let angle = (i as f32 / POINTS as f32) * std::f32::consts::TAU;

            // Generate deterministic "random" value for radius variation
            let seed = hash(base_seed.wrapping_add(i as u64));
            let rand_val = seed as f32 / u64::MAX as f32; // 0.0 to 1.0

            // Create variation: 80% to 100% of radius
            // Keeping it closer to full radius ensures convexity
            // Too much variation creates concave shapes that break fan triangulation
            let r = radius * (0.80 + rand_val * 0.20);

            offsets.push(Vec2::new(r * angle.cos(), r * angle.sin()));
        }

        // Verify and fix convexity: ensure no point is too close to center
        // This prevents the "line to center" issue with fan triangulation
        for i in 0..POINTS {
            let dist = offsets[i].length();
            if dist < radius * 0.7 {
                // Push point outward to maintain convexity
                let angle = (i as f32 / POINTS as f32) * std::f32::consts::TAU;
                offsets[i] = Vec2::new(angle.cos(), angle.sin()) * (radius * 0.8);
            }
        }

        offsets
    }

    pub fn radius(&self) -> f32 {
        match self.size {
            AsteroidSize::Large => 40.0,
            AsteroidSize::Medium => 20.0,
            AsteroidSize::Small => 10.0,
        }
    }
}
