use crate::math::Vec2;

/// A point light that emits light in all directions from a position.
#[derive(Clone, Copy, Debug)]
pub struct PointLight {
    /// Position of the light in world coordinates
    pub position: Vec2,
    /// Color of the light (RGB, values typically 0.0-1.0, can exceed 1.0 for HDR)
    pub color: [f32; 3],
    /// Intensity/brightness of the light (0.0 = off, 1.0 = normal, >1.0 = brighter)
    pub intensity: f32,
    /// Radius of the light (how far it reaches)
    pub radius: f32,
    /// Falloff curve (1.0 = linear, 2.0 = quadratic, higher = sharper falloff)
    pub falloff: f32,
    /// Direction for spotlight (if None, emits in all directions)
    pub direction: Option<Vec2>,
    /// Spotlight angle in radians (cone half-angle, only used if direction is Some)
    pub angle: f32,
}

impl PointLight {
    /// Create a new point light (emits in all directions).
    pub fn new(position: Vec2, color: [f32; 3], intensity: f32, radius: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            radius,
            falloff: 2.0, // Default to quadratic falloff
            direction: None,
            angle: std::f32::consts::PI / 4.0, // 45 degrees default
        }
    }

    /// Create a new spotlight (emits in a specific direction).
    pub fn new_spotlight(
        position: Vec2,
        direction: Vec2,
        color: [f32; 3],
        intensity: f32,
        radius: f32,
        angle: f32,
    ) -> Self {
        Self {
            position,
            color,
            intensity,
            radius,
            falloff: 2.0,
            direction: Some(direction.normalized()),
            angle,
        }
    }

    /// Set the falloff curve (1.0 = linear, 2.0 = quadratic).
    pub fn with_falloff(mut self, falloff: f32) -> Self {
        self.falloff = falloff;
        self
    }
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            radius: 100.0,
            falloff: 2.0,
            direction: None,
            angle: std::f32::consts::PI / 4.0,
        }
    }
}

/// A directional light (like sunlight) that emits light in a specific direction.
#[derive(Clone, Copy, Debug)]
pub struct DirectionalLight {
    /// Direction the light is coming from (normalized vector)
    pub direction: Vec2,
    /// Color of the light (RGB)
    pub color: [f32; 3],
    /// Intensity/brightness of the light
    pub intensity: f32,
}

impl DirectionalLight {
    /// Create a new directional light.
    pub fn new(direction: Vec2, color: [f32; 3], intensity: f32) -> Self {
        Self {
            direction: direction.normalized(),
            color,
            intensity,
        }
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        Self {
            direction: Vec2::new(0.0, -1.0), // Default: light from above
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
        }
    }
}
