use glam::{Mat4, Vec2 as GlamVec2, Vec3};
use serde::{Deserialize, Serialize};

/// 2D vector type used throughout Sindri.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let len = self.length();
        if len == 0.0 {
            Self::ZERO
        } else {
            Self::new(self.x / len, self.y / len)
        }
    }

    pub fn to_glam(&self) -> GlamVec2 {
        GlamVec2::new(self.x, self.y)
    }

    /// Returns the squared length of the vector (faster than `length()`).
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Computes the dot product of two vectors.
    pub fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Computes the distance between two points.
    pub fn distance(self, rhs: Self) -> f32 {
        (self - rhs).length()
    }

    /// Computes the squared distance between two points (faster than `distance()`).
    pub fn distance_squared(self, rhs: Self) -> f32 {
        (self - rhs).length_squared()
    }

    /// Linearly interpolates between two vectors.
    pub fn lerp(self, rhs: Self, t: f32) -> Self {
        Self::new(self.x + (rhs.x - self.x) * t, self.y + (rhs.y - self.y) * t)
    }

    /// Creates a unit vector pointing in the given direction (angle in radians).
    pub fn from_angle(angle: f32) -> Self {
        Self::new(angle.cos(), angle.sin())
    }

    /// Returns a vector with component-wise absolute values.
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs())
    }

    /// Returns a vector with component-wise minimum values.
    pub fn min(self, rhs: Self) -> Self {
        Self::new(self.x.min(rhs.x), self.y.min(rhs.y))
    }

    /// Returns a vector with component-wise maximum values.
    pub fn max(self, rhs: Self) -> Self {
        Self::new(self.x.max(rhs.x), self.y.max(rhs.y))
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from(value: (f32, f32)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl std::ops::MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl std::ops::Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl std::ops::DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl std::ops::Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

/// Transform describing 2D position, scale, and rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    pub position: Vec2,
    pub scale: Vec2,
    /// Rotation in radians around the Z axis.
    pub rotation: f32,
}

impl Transform2D {
    pub fn new(position: Vec2, scale: Vec2, rotation: f32) -> Self {
        Self {
            position,
            scale,
            rotation,
        }
    }

    pub fn identity() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
        }
    }

    pub fn to_matrix(&self, base_size: Vec2) -> Mat4 {
        let translation = Mat4::from_translation(Vec3::new(self.position.x, self.position.y, 0.0));
        let rotation = Mat4::from_rotation_z(self.rotation);
        let scale = Mat4::from_scale(Vec3::new(
            self.scale.x * base_size.x,
            self.scale.y * base_size.y,
            1.0,
        ));

        translation * rotation * scale
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::identity()
    }
}

/// Camera representing a simple 2D view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera2D {
    pub position: Vec2,
    pub zoom: f32,
    /// Rotation in radians around the Z axis (0.0 = no rotation)
    pub rotation: f32,
    /// Offset from position (useful for look-ahead in platformers)
    pub offset: Vec2,
    /// Target zoom for smooth zoom transitions
    pub target_zoom: f32,
    /// Zoom speed for smooth transitions (units per second)
    pub zoom_speed: f32,
    /// Camera shake intensity (decays over time)
    pub shake_intensity: f32,
    /// Camera shake timer (seconds remaining)
    pub shake_timer: f32,
    /// Camera shake seed (for deterministic shake pattern)
    shake_seed: f32,
    /// World bounds (min, max) - camera will be clamped to these bounds
    pub bounds: Option<(Vec2, Vec2)>,
}

impl Camera2D {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            zoom: 1.0,
            rotation: 0.0,
            offset: Vec2::ZERO,
            target_zoom: 1.0,
            zoom_speed: 0.0,
            shake_intensity: 0.0,
            shake_timer: 0.0,
            shake_seed: 0.0,
            bounds: None,
        }
    }

    /// Set camera rotation in radians.
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set camera offset (look-ahead).
    pub fn with_offset(mut self, offset: Vec2) -> Self {
        self.offset = offset;
        self
    }

    /// Set world bounds that the camera will be clamped to.
    pub fn with_bounds(mut self, min: Vec2, max: Vec2) -> Self {
        self.bounds = Some((min, max));
        self
    }

    /// Remove world bounds.
    pub fn without_bounds(mut self) -> Self {
        self.bounds = None;
        self
    }

    /// Apply camera shake with given intensity and duration.
    pub fn shake(&mut self, intensity: f32, duration: f32) {
        self.shake_intensity = intensity.max(self.shake_intensity);
        self.shake_timer = duration.max(self.shake_timer);
        // Reset seed when new shake starts
        if self.shake_timer == duration {
            self.shake_seed = 0.0;
        }
    }

    /// Set target zoom and speed for smooth zoom transitions.
    pub fn zoom_to(&mut self, target_zoom: f32, speed: f32) {
        self.target_zoom = target_zoom;
        self.zoom_speed = speed;
    }

    /// Zoom towards a specific world point.
    pub fn zoom_to_point(
        &mut self,
        world_point: Vec2,
        target_zoom: f32,
        speed: f32,
        screen_width: u32,
        screen_height: u32,
    ) {
        // Calculate the offset needed to keep the point under the cursor
        let current_screen = self.world_to_screen(world_point, screen_width, screen_height);
        let half_width = screen_width as f32 / 2.0;
        let half_height = screen_height as f32 / 2.0;
        let screen_offset = Vec2::new(
            current_screen.x - half_width,
            current_screen.y - half_height,
        );

        // Store current zoom and set target
        let old_zoom = self.zoom;
        self.target_zoom = target_zoom;
        self.zoom_speed = speed;

        // Adjust position to compensate for zoom change
        // When zooming in, we need to move the camera to keep the point under the cursor
        let zoom_ratio = target_zoom / old_zoom;
        let world_offset = Vec2::new(
            screen_offset.x / old_zoom * (1.0 - 1.0 / zoom_ratio),
            screen_offset.y / old_zoom * (1.0 - 1.0 / zoom_ratio),
        );
        self.position = self.position + world_offset;
    }

    /// Update camera (call this every frame for smooth zoom and shake).
    pub fn update(&mut self, dt: f32) {
        // Update smooth zoom
        if self.zoom_speed > 0.0 && (self.zoom - self.target_zoom).abs() > 0.01 {
            let diff = self.target_zoom - self.zoom;
            let max_change = self.zoom_speed * dt;
            if diff.abs() <= max_change {
                self.zoom = self.target_zoom;
                self.zoom_speed = 0.0;
            } else {
                self.zoom += diff.signum() * max_change;
            }
        }

        // Update shake
        if self.shake_timer > 0.0 {
            self.shake_timer -= dt;
            self.shake_seed += dt * 60.0; // Increment seed at ~60fps rate
            if self.shake_timer <= 0.0 {
                self.shake_intensity = 0.0;
                self.shake_timer = 0.0;
                self.shake_seed = 0.0;
            }
        }

        // Apply bounds clamping
        if let Some((min, max)) = self.bounds {
            self.position.x = self.position.x.clamp(min.x, max.x);
            self.position.y = self.position.y.clamp(min.y, max.y);
        }
    }

    /// Get the effective camera position (position + offset + shake).
    fn effective_position(&self) -> Vec2 {
        let mut pos = self.position + self.offset;

        // Apply shake
        if self.shake_intensity > 0.0 && self.shake_timer > 0.0 {
            // Use seed for deterministic shake pattern
            let shake_x = (self.shake_seed * 50.0).sin() * self.shake_intensity;
            let shake_y = (self.shake_seed * 43.0).cos() * self.shake_intensity;
            pos = pos + Vec2::new(shake_x, shake_y);
        }

        pos
    }

    /// Get the visible world bounds (viewport rectangle in world coordinates).
    pub fn viewport_bounds(&self, screen_width: u32, screen_height: u32) -> (Vec2, Vec2) {
        let effective_pos = self.effective_position();
        let half_width = (screen_width as f32 / 2.0) / self.zoom;
        let half_height = (screen_height as f32 / 2.0) / self.zoom;

        // Account for rotation
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();

        // Corners of the viewport in local space (before rotation)
        let corners = [
            Vec2::new(-half_width, -half_height),
            Vec2::new(half_width, -half_height),
            Vec2::new(half_width, half_height),
            Vec2::new(-half_width, half_height),
        ];

        // Rotate corners and find min/max
        let rotated_corners: Vec<Vec2> = corners
            .iter()
            .map(|&corner| {
                Vec2::new(
                    corner.x * cos - corner.y * sin,
                    corner.x * sin + corner.y * cos,
                ) + effective_pos
            })
            .collect();

        let min_x = rotated_corners
            .iter()
            .map(|v| v.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = rotated_corners
            .iter()
            .map(|v| v.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = rotated_corners
            .iter()
            .map(|v| v.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = rotated_corners
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max);

        (Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
    }

    /// Check if a point is visible in the camera viewport.
    pub fn is_point_visible(&self, point: Vec2, screen_width: u32, screen_height: u32) -> bool {
        let (min, max) = self.viewport_bounds(screen_width, screen_height);
        point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y
    }

    /// Check if a rectangle is visible in the camera viewport (AABB intersection).
    pub fn is_rect_visible(
        &self,
        rect_min: Vec2,
        rect_max: Vec2,
        screen_width: u32,
        screen_height: u32,
    ) -> bool {
        let (viewport_min, viewport_max) = self.viewport_bounds(screen_width, screen_height);

        // AABB intersection test
        rect_min.x <= viewport_max.x
            && rect_max.x >= viewport_min.x
            && rect_min.y <= viewport_max.y
            && rect_max.y >= viewport_min.y
    }

    /// Check if a circle is visible in the camera viewport.
    pub fn is_circle_visible(
        &self,
        center: Vec2,
        radius: f32,
        screen_width: u32,
        screen_height: u32,
    ) -> bool {
        let (viewport_min, viewport_max) = self.viewport_bounds(screen_width, screen_height);

        // Find closest point on viewport AABB to circle center
        let closest_x = center.x.clamp(viewport_min.x, viewport_max.x);
        let closest_y = center.y.clamp(viewport_min.y, viewport_max.y);
        let closest_point = Vec2::new(closest_x, closest_y);

        // Check if closest point is within circle radius
        center.distance_squared(closest_point) <= radius * radius
    }

    pub fn view_projection(&self, width: u32, height: u32) -> Mat4 {
        let projection = Mat4::orthographic_rh_gl(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);

        // Get effective position (includes offset and shake)
        let effective_pos = self.effective_position();

        let half_width = width as f32 / 2.0;
        let half_height = height as f32 / 2.0;

        // For proper rotation and zoom around screen center:
        // 1. Translate world so camera center is at origin
        // 2. Rotate around origin
        // 3. Scale (zoom) around origin
        // 4. Translate to screen center
        // 5. Project

        // Step 1: Move camera position to origin
        let translate_camera_to_origin =
            Mat4::from_translation(Vec3::new(-effective_pos.x, -effective_pos.y, 0.0));

        // Step 2: Rotate around origin
        let rotation = Mat4::from_rotation_z(self.rotation);

        // Step 3: Scale (zoom) around origin
        let zoom = Mat4::from_scale(Vec3::new(self.zoom, self.zoom, 1.0));

        // Step 4: Translate to screen center
        let translate_to_screen_center =
            Mat4::from_translation(Vec3::new(half_width, half_height, 0.0));

        // Matrix multiplication order (right to left):
        // translate_to_screen_center * zoom * rotation * translate_camera_to_origin
        // This means: first move camera to origin, then rotate, then zoom, then move to screen center
        let view = translate_to_screen_center * zoom * rotation * translate_camera_to_origin;

        projection * view
    }

    /// Converts screen coordinates to world coordinates using this camera.
    /// Note: camera.position represents the center of the view, not the top-left corner.
    pub fn screen_to_world(&self, screen_pos: Vec2, screen_width: u32, screen_height: u32) -> Vec2 {
        let effective_pos = self.effective_position();

        let half_width = screen_width as f32 / 2.0;
        let half_height = screen_height as f32 / 2.0;

        // Step 1: Convert from screen space to camera-local space (relative to screen center)
        let local_x = screen_pos.x - half_width;
        let local_y = screen_pos.y - half_height;

        // Step 2: Apply inverse zoom (divide by zoom)
        let zoomed_x = local_x / self.zoom;
        let zoomed_y = local_y / self.zoom;

        // Step 3: Apply inverse rotation
        let cos = (-self.rotation).cos();
        let sin = (-self.rotation).sin();
        let rotated_x = zoomed_x * cos - zoomed_y * sin;
        let rotated_y = zoomed_x * sin + zoomed_y * cos;

        // Step 4: Translate to world space (add camera position)
        Vec2::new(rotated_x + effective_pos.x, rotated_y + effective_pos.y)
    }

    /// Converts world coordinates to screen coordinates using this camera.
    /// Note: camera.position represents the center of the view, not the top-left corner.
    pub fn world_to_screen(&self, world_pos: Vec2, screen_width: u32, screen_height: u32) -> Vec2 {
        let effective_pos = self.effective_position();

        let half_width = screen_width as f32 / 2.0;
        let half_height = screen_height as f32 / 2.0;

        // Step 1: Convert to camera-local space (relative to camera position)
        let local_x = world_pos.x - effective_pos.x;
        let local_y = world_pos.y - effective_pos.y;

        // Step 2: Apply rotation
        let cos = self.rotation.cos();
        let sin = self.rotation.sin();
        let rotated_x = local_x * cos - local_y * sin;
        let rotated_y = local_x * sin + local_y * cos;

        // Step 3: Apply zoom
        let zoomed_x = rotated_x * self.zoom;
        let zoomed_y = rotated_y * self.zoom;

        // Step 4: Convert to screen space (add screen center)
        Vec2::new(zoomed_x + half_width, zoomed_y + half_height)
    }
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            rotation: 0.0,
            offset: Vec2::ZERO,
            target_zoom: 1.0,
            zoom_speed: 0.0,
            shake_intensity: 0.0,
            shake_timer: 0.0,
            shake_seed: 0.0,
            bounds: None,
        }
    }
}
