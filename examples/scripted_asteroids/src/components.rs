use sindri::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct PlayerMarker;

#[derive(Clone, Copy, Debug)]
pub struct BulletMarker;

#[derive(Clone, Copy, Debug)]
pub struct AsteroidMarker {
    pub size: AsteroidSize,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AsteroidSize {
    Large,
    Medium,
    Small,
}

impl AsteroidSize {
    pub fn radius(self) -> f32 {
        match self {
            Self::Large => 44.0,
            Self::Medium => 26.0,
            Self::Small => 14.0,
        }
    }

    pub fn score(self) -> u32 {
        match self {
            Self::Large => 20,
            Self::Medium => 50,
            Self::Small => 100,
        }
    }

    pub fn child(self) -> Option<Self> {
        match self {
            Self::Large => Some(Self::Medium),
            Self::Medium => Some(Self::Small),
            Self::Small => None,
        }
    }

    pub fn speed_range(self) -> (f32, f32) {
        match self {
            Self::Large => (42.0, 78.0),
            Self::Medium => (70.0, 115.0),
            Self::Small => (105.0, 165.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PolygonShape {
    pub offsets: Vec<Vec2>,
}

impl PolygonShape {
    pub fn asteroid(position: Vec2, radius: f32) -> Self {
        const POINTS: usize = 10;

        let seed = ((position.x * 13.37) as i32 as u32)
            ^ ((position.y * 91.17) as i32 as u32)
            ^ radius as u32;

        let mut offsets = Vec::with_capacity(POINTS);

        for i in 0..POINTS {
            let angle = (i as f32 / POINTS as f32) * std::f32::consts::TAU;
            let n = pseudo_random(seed.wrapping_add(i as u32));
            let r = radius * (0.78 + n * 0.24);
            offsets.push(Vec2::new(angle.cos() * r, angle.sin() * r));
        }

        Self { offsets }
    }

    pub fn transformed(&self, position: Vec2, rotation: f32) -> Vec<Vec2> {
        let cos = rotation.cos();
        let sin = rotation.sin();

        self.offsets
            .iter()
            .map(|p| {
                let rotated = Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
                position + rotated
            })
            .collect()
    }
}

fn pseudo_random(mut x: u32) -> f32 {
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    (x as f32 / u32::MAX as f32).clamp(0.0, 1.0)
}
