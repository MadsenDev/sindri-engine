use super::sprite::TextureHandle;
use crate::math::Vec2;

/// A single particle in the particle system.
#[derive(Clone, Debug)]
pub struct Particle {
    /// Current position in world coordinates
    pub position: Vec2,
    /// Current velocity (units per second)
    pub velocity: Vec2,
    /// Current color (RGBA)
    pub color: [f32; 4],
    /// Current size (width and height)
    pub size: Vec2,
    /// Initial size (used for size interpolation over lifetime)
    pub initial_size: Vec2,
    /// Remaining lifetime in seconds (0.0 = dead)
    pub lifetime: f32,
    /// Maximum lifetime in seconds
    pub max_lifetime: f32,
    /// Rotation in radians
    pub rotation: f32,
    /// Angular velocity (radians per second)
    pub angular_velocity: f32,
}

impl Particle {
    /// Create a new particle with default values.
    pub fn new() -> Self {
        Self {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            color: [1.0, 1.0, 1.0, 1.0],
            size: Vec2::new(1.0, 1.0),
            initial_size: Vec2::new(1.0, 1.0),
            lifetime: 1.0,
            max_lifetime: 1.0,
            rotation: 0.0,
            angular_velocity: 0.0,
        }
    }

    /// Check if the particle is alive.
    pub fn is_alive(&self) -> bool {
        self.lifetime > 0.0
    }

    /// Get the normalized age (0.0 = just spawned, 1.0 = about to die).
    pub fn age(&self) -> f32 {
        if self.max_lifetime > 0.0 {
            1.0 - (self.lifetime / self.max_lifetime)
        } else {
            0.0
        }
    }
}

impl Default for Particle {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for how particles are spawned from an emitter.
#[derive(Clone, Debug)]
pub struct EmissionConfig {
    /// Number of particles to spawn per second (0 = burst only)
    pub particles_per_second: f32,
    /// Number of particles to spawn in a burst (one-time)
    pub burst_count: usize,
    /// Whether the burst has been emitted
    pub burst_emitted: bool,
    /// Position where particles spawn
    pub position: Vec2,
    /// Position variance (random offset from position)
    pub position_variance: Vec2,
    /// Initial velocity range
    pub velocity_min: Vec2,
    pub velocity_max: Vec2,
    /// Initial size range
    pub size_min: Vec2,
    pub size_max: Vec2,
    /// Initial color
    pub color_start: [f32; 4],
    /// End color (particles interpolate from start to end)
    pub color_end: Option<[f32; 4]>,
    /// Lifetime range in seconds
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    /// Gravity/acceleration applied to particles
    pub acceleration: Vec2,
    /// Angular velocity range (radians per second)
    pub angular_velocity_min: f32,
    pub angular_velocity_max: f32,
    /// Size change over lifetime (multiplier at end of life)
    pub size_end_multiplier: f32,
    /// Whether particles should fade out over lifetime
    pub fade_out: bool,
}

impl EmissionConfig {
    /// Create a new emission config with sensible defaults.
    pub fn new(position: Vec2) -> Self {
        Self {
            particles_per_second: 0.0,
            burst_count: 0,
            burst_emitted: false,
            position,
            position_variance: Vec2::ZERO,
            velocity_min: Vec2::new(-50.0, -50.0),
            velocity_max: Vec2::new(50.0, 50.0),
            size_min: Vec2::new(2.0, 2.0),
            size_max: Vec2::new(4.0, 4.0),
            color_start: [1.0, 1.0, 1.0, 1.0],
            color_end: None,
            lifetime_min: 0.5,
            lifetime_max: 2.0,
            acceleration: Vec2::new(0.0, 0.0),
            angular_velocity_min: 0.0,
            angular_velocity_max: 0.0,
            size_end_multiplier: 1.0,
            fade_out: true,
        }
    }

    /// Set continuous emission rate.
    pub fn with_rate(mut self, particles_per_second: f32) -> Self {
        self.particles_per_second = particles_per_second;
        self
    }

    /// Set burst emission (one-time spawn).
    pub fn with_burst(mut self, count: usize) -> Self {
        self.burst_count = count;
        self.burst_emitted = false;
        self
    }

    /// Set velocity range.
    pub fn with_velocity(mut self, min: Vec2, max: Vec2) -> Self {
        self.velocity_min = min;
        self.velocity_max = max;
        self
    }

    /// Set size range.
    pub fn with_size(mut self, min: Vec2, max: Vec2) -> Self {
        self.size_min = min;
        self.size_max = max;
        self
    }

    /// Set color (with optional end color for interpolation).
    pub fn with_color(mut self, start: [f32; 4], end: Option<[f32; 4]>) -> Self {
        self.color_start = start;
        self.color_end = end;
        self
    }

    /// Set lifetime range.
    pub fn with_lifetime(mut self, min: f32, max: f32) -> Self {
        self.lifetime_min = min;
        self.lifetime_max = max;
        self
    }

    /// Set gravity/acceleration.
    pub fn with_acceleration(mut self, acceleration: Vec2) -> Self {
        self.acceleration = acceleration;
        self
    }

    /// Set size change over lifetime (multiplier at end of life).
    /// 1.0 = no change, 0.5 = shrink to half, 2.0 = grow to double.
    pub fn with_size_end_multiplier(mut self, multiplier: f32) -> Self {
        self.size_end_multiplier = multiplier;
        self
    }

    /// Set whether particles should fade out over lifetime.
    pub fn with_fade_out(mut self, fade_out: bool) -> Self {
        self.fade_out = fade_out;
        self
    }
}

/// A particle emitter that spawns and manages particles.
pub struct ParticleEmitter {
    config: EmissionConfig,
    particles: Vec<Particle>,
    spawn_timer: f32,
    max_particles: usize,
    texture: Option<TextureHandle>,
}

impl ParticleEmitter {
    /// Create a new particle emitter.
    pub fn new(config: EmissionConfig) -> Self {
        Self {
            config,
            particles: Vec::new(),
            spawn_timer: 0.0,
            max_particles: 1000,
            texture: None,
        }
    }

    /// Set the maximum number of particles this emitter can have alive at once.
    pub fn with_max_particles(mut self, max: usize) -> Self {
        self.max_particles = max;
        self
    }

    /// Set a texture for particles (if None, particles are rendered as colored quads).
    pub fn with_texture(mut self, texture: Option<TextureHandle>) -> Self {
        self.texture = texture;
        self
    }

    /// Get the texture handle for particles.
    pub fn texture(&self) -> Option<TextureHandle> {
        self.texture
    }

    /// Update the emitter and all particles.
    pub fn update(&mut self, dt: f32) {
        // Remove dead particles first (cleanup before update)
        self.particles.retain(|p| p.is_alive());

        // Update existing particles
        for particle in &mut self.particles {
            // Apply acceleration
            particle.velocity += self.config.acceleration * dt;

            // Update position
            particle.position += particle.velocity * dt;

            // Update rotation
            particle.rotation += particle.angular_velocity * dt;

            // Update lifetime
            particle.lifetime -= dt;

            // Clamp lifetime to prevent negative values
            if particle.lifetime < 0.0 {
                particle.lifetime = 0.0;
            }

            // Update color interpolation
            if let Some(color_end) = self.config.color_end {
                let age = particle.age();
                particle.color[0] = self.config.color_start[0] * (1.0 - age) + color_end[0] * age;
                particle.color[1] = self.config.color_start[1] * (1.0 - age) + color_end[1] * age;
                particle.color[2] = self.config.color_start[2] * (1.0 - age) + color_end[2] * age;
                particle.color[3] = self.config.color_start[3] * (1.0 - age) + color_end[3] * age;
            }

            // Update size over lifetime (interpolate from initial_size based on age)
            let size_factor = 1.0 + (self.config.size_end_multiplier - 1.0) * particle.age();
            particle.size = Vec2::new(
                particle.initial_size.x * size_factor,
                particle.initial_size.y * size_factor,
            );

            // Fade out alpha if enabled
            if self.config.fade_out {
                particle.color[3] *= particle.lifetime / particle.max_lifetime;
            }
        }

        // Remove dead particles again after update
        self.particles.retain(|p| p.is_alive());

        // Spawn new particles
        if !self.config.burst_emitted && self.config.burst_count > 0 {
            // Emit burst
            for _ in 0..self.config.burst_count {
                if self.particles.len() < self.max_particles {
                    self.spawn_particle();
                }
            }
            self.config.burst_emitted = true;
        }

        // Continuous emission
        if self.config.particles_per_second > 0.0 {
            self.spawn_timer += dt;
            let spawn_interval = 1.0 / self.config.particles_per_second;

            while self.spawn_timer >= spawn_interval && self.particles.len() < self.max_particles {
                self.spawn_particle();
                self.spawn_timer -= spawn_interval;
            }
        }
    }

    /// Spawn a single particle with random properties based on config.
    fn spawn_particle(&mut self) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        // Simple pseudo-random number generator using particle count and time as seed
        let mut hasher = DefaultHasher::new();
        self.particles.len().hash(&mut hasher);
        let time_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        time_seed.hash(&mut hasher);
        let seed = hasher.finish();
        let mut rng_state = (seed as u32) as f32;

        // Helper to generate random float in range using LCG
        let mut next_rand = || -> f32 {
            rng_state = (rng_state * 1103515245.0 + 12345.0) % 2147483647.0;
            rng_state / 2147483647.0
        };

        let mut rand = |min: f32, max: f32| -> f32 { min + next_rand() * (max - min) };

        let mut particle = Particle::new();

        // Random position
        particle.position = Vec2::new(
            self.config.position.x
                + rand(
                    -self.config.position_variance.x,
                    self.config.position_variance.x,
                ),
            self.config.position.y
                + rand(
                    -self.config.position_variance.y,
                    self.config.position_variance.y,
                ),
        );

        // Random velocity
        particle.velocity = Vec2::new(
            rand(self.config.velocity_min.x, self.config.velocity_max.x),
            rand(self.config.velocity_min.y, self.config.velocity_max.y),
        );

        // Random size (store as both current and initial)
        particle.size = Vec2::new(
            rand(self.config.size_min.x, self.config.size_max.x),
            rand(self.config.size_min.y, self.config.size_max.y),
        );
        particle.initial_size = particle.size;

        // Initial color
        particle.color = self.config.color_start;

        // Random lifetime
        particle.lifetime = rand(self.config.lifetime_min, self.config.lifetime_max);
        particle.max_lifetime = particle.lifetime;

        // Random rotation and angular velocity
        particle.rotation = rand(0.0, std::f32::consts::TAU);
        particle.angular_velocity = rand(
            self.config.angular_velocity_min,
            self.config.angular_velocity_max,
        );

        self.particles.push(particle);
    }

    /// Get all alive particles.
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Check if the emitter is still active (has particles or can spawn more).
    pub fn is_active(&self) -> bool {
        // Keep emitter active if it has particles (even if emission is stopped)
        // This allows stopped emitters to finish their particle lifecycle
        !self.particles.is_empty()
            || !self.config.burst_emitted
            || self.config.particles_per_second > 0.0
    }

    /// Update the emitter's position.
    pub fn set_position(&mut self, position: Vec2) {
        self.config.position = position;
    }

    /// Get the emitter's position.
    pub fn position(&self) -> Vec2 {
        self.config.position
    }

    /// Stop emitting particles (set rate to 0).
    pub fn stop_emission(&mut self) {
        self.config.particles_per_second = 0.0;
    }

    /// Check if the emitter is still emitting particles.
    pub fn is_emitting(&self) -> bool {
        self.config.particles_per_second > 0.0
    }
}

/// A system that manages multiple particle emitters.
pub struct ParticleSystem {
    emitters: Vec<ParticleEmitter>,
}

impl ParticleSystem {
    /// Create a new particle system.
    pub fn new() -> Self {
        Self {
            emitters: Vec::new(),
        }
    }

    /// Add an emitter to the system.
    pub fn add_emitter(&mut self, emitter: ParticleEmitter) {
        self.emitters.push(emitter);
    }

    /// Update all emitters.
    pub fn update(&mut self, dt: f32) {
        for emitter in &mut self.emitters {
            emitter.update(dt);
        }

        // Remove inactive emitters
        self.emitters.retain(|e| e.is_active());
    }

    /// Get all emitters.
    pub fn emitters(&self) -> &[ParticleEmitter] {
        &self.emitters
    }

    /// Get mutable access to all emitters.
    pub fn emitters_mut(&mut self) -> &mut [ParticleEmitter] {
        &mut self.emitters
    }

    /// Clear all emitters.
    pub fn clear(&mut self) {
        self.emitters.clear();
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}
