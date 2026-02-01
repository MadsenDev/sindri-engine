// forge2d/src/physics.rs
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::math::Vec2;
use crate::world::EntityId;

// Rapier is private implementation detail: do NOT re-export it.
use rapier2d::prelude::*;

/// Engine-facing rigid body type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RigidBodyType {
    Dynamic,
    Kinematic,
    Fixed,
}

/// Engine-facing collider shape.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ColliderShape {
    Box { hx: f32, hy: f32 },
    Circle { radius: f32 },
    CapsuleY { half_height: f32, radius: f32 },
}

/// Engine-facing collision event. Uses EntityId only.
#[derive(Clone, Copy, Debug)]
pub enum PhysicsEvent {
    CollisionEnter { a: EntityId, b: EntityId },
    CollisionExit { a: EntityId, b: EntityId },
    TriggerEnter { a: EntityId, b: EntityId },
    TriggerExit { a: EntityId, b: EntityId },
}

/// Optional callback for physics events.
pub type PhysicsEventCallback = Box<dyn Fn(PhysicsEvent) + Send + Sync>;

pub struct PhysicsWorld {
    // --- rapier internals ---
    pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    rigid_bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    query_pipeline: QueryPipeline,

    // Event channels
    event_recv_collision: crossbeam_channel::Receiver<CollisionEvent>,
    event_recv_contact_force: crossbeam_channel::Receiver<ContactForceEvent>,
    event_handler: ChannelEventCollector,

    // --- mappings (engine <-> rapier) ---
    entity_to_body: HashMap<EntityId, RigidBodyHandle>,
    body_to_entity: HashMap<RigidBodyHandle, EntityId>,

    gravity: Vec2,

    // Collected engine-facing events for the frame
    pending_events: Vec<PhysicsEvent>,
    callbacks: Vec<PhysicsEventCallback>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsWorld {
    pub fn new() -> Self {
        let (send_col, recv_col) = crossbeam_channel::unbounded();
        let (send_force, recv_force) = crossbeam_channel::unbounded();
        let event_handler = ChannelEventCollector::new(send_col, send_force);

        Self {
            pipeline: PhysicsPipeline::new(),
            integration_parameters: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            rigid_bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            query_pipeline: QueryPipeline::new(),

            event_recv_collision: recv_col,
            event_recv_contact_force: recv_force,
            event_handler,

            entity_to_body: HashMap::new(),
            body_to_entity: HashMap::new(),

            gravity: Vec2::new(0.0, 9.81),
            pending_events: Vec::new(),
            callbacks: Vec::new(),
        }
    }

    pub fn with_gravity(gravity: Vec2) -> Self {
        let mut w = Self::new();
        w.gravity = gravity;
        w
    }

    /// Clear all physics bodies and colliders, keeping only gravity and configuration.
    /// This is useful for scene loading - completely rebuilds the physics world.
    pub fn clear(&mut self) {
        let gravity = self.gravity;
        *self = Self::with_gravity(gravity);
    }

    pub fn set_gravity(&mut self, gravity: Vec2) {
        self.gravity = gravity;
    }

    pub fn gravity(&self) -> Vec2 {
        self.gravity
    }

    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(PhysicsEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// Create/replace a body for an entity. Returns error if something goes wrong.
    pub fn create_body(
        &mut self,
        entity: EntityId,
        body_type: RigidBodyType,
        position: Vec2,
        rotation: f32,
    ) -> Result<()> {
        // Remove existing body if any (keeps invariant 1 body per entity).
        self.remove_body(entity);

        let rb_type = match body_type {
            RigidBodyType::Dynamic => rapier2d::prelude::RigidBodyType::Dynamic,
            RigidBodyType::Kinematic => rapier2d::prelude::RigidBodyType::KinematicPositionBased,
            RigidBodyType::Fixed => rapier2d::prelude::RigidBodyType::Fixed,
        };

        let mut builder = RigidBodyBuilder::new(rb_type)
            .translation(vector![position.x, position.y])
            .rotation(rotation);

        // Enable CCD for dynamic bodies to prevent tunneling through thin colliders
        if matches!(body_type, RigidBodyType::Dynamic) {
            builder = builder.ccd_enabled(true);
        }

        let body = builder.build();

        let handle = self.rigid_bodies.insert(body);
        self.entity_to_body.insert(entity, handle);
        self.body_to_entity.insert(handle, entity);
        Ok(())
    }

    /// Remove a body (and its colliders) for an entity. Returns whether one existed.
    pub fn remove_body(&mut self, entity: EntityId) -> bool {
        if let Some(handle) = self.entity_to_body.remove(&entity) {
            self.rigid_bodies.remove(
                handle,
                &mut self.island_manager,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            );
            self.body_to_entity.remove(&handle);
            true
        } else {
            false
        }
    }

    /// Add a solid collider with material properties.
    pub fn add_collider_with_material(
        &mut self,
        entity: EntityId,
        shape: ColliderShape,
        offset: Vec2,
        density: f32,
        friction: f32,
        restitution: f32,
    ) -> Result<()> {
        let body = self.body_handle(entity)?;

        let rapier_shape = self.to_rapier_shape(shape);
        let collider = ColliderBuilder::new(rapier_shape)
            .translation(vector![offset.x, offset.y])
            .density(density)
            .friction(friction)
            .restitution(restitution)
            .sensor(false) // Explicitly ensure it's NOT a sensor (ChatGPT's fix)
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();

        self.colliders
            .insert_with_parent(collider, body, &mut self.rigid_bodies);

        Ok(())
    }

    /// Add a sensor (trigger volume).
    pub fn add_sensor(
        &mut self,
        entity: EntityId,
        shape: ColliderShape,
        offset: Vec2,
    ) -> Result<()> {
        let body = self.body_handle(entity)?;

        let rapier_shape = self.to_rapier_shape(shape);
        let collider = ColliderBuilder::new(rapier_shape)
            .translation(vector![offset.x, offset.y])
            .sensor(true)
            // ensure we get collision events for sensors:
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();

        self.colliders
            .insert_with_parent(collider, body, &mut self.rigid_bodies);

        Ok(())
    }

    /// Step simulation by fixed dt (seconds).
    pub fn step(&mut self, dt: f32) {
        self.integration_parameters.dt = dt;

        let gravity = vector![self.gravity.x, self.gravity.y];
        let hooks = &();

        self.pipeline.step(
            &gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            hooks,
            &self.event_handler,
        );

        self.query_pipeline
            .update(&self.island_manager, &self.rigid_bodies, &self.colliders);

        self.collect_events();
    }

    /// Drain physics events collected since last step.
    pub fn drain_events(&mut self) -> Vec<PhysicsEvent> {
        std::mem::take(&mut self.pending_events)
    }

    // ------------------------------
    // Per-entity body queries/actions
    // ------------------------------

    pub fn body_position(&self, entity: EntityId) -> Option<Vec2> {
        let h = *self.entity_to_body.get(&entity)?;
        let b = self.rigid_bodies.get(h)?;
        let t = b.translation();
        Some(Vec2::new(t.x, t.y))
    }

    pub fn body_rotation(&self, entity: EntityId) -> Option<f32> {
        let h = *self.entity_to_body.get(&entity)?;
        let b = self.rigid_bodies.get(h)?;
        Some(b.rotation().angle())
    }

    pub fn set_body_position(&mut self, entity: EntityId, pos: Vec2) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.set_translation(vector![pos.x, pos.y], true);
            }
        }
    }

    pub fn set_body_rotation(&mut self, entity: EntityId, rot: f32) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.set_rotation(rot, true);
            }
        }
    }

    pub fn set_linear_velocity(&mut self, entity: EntityId, vel: Vec2) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.set_linvel(vector![vel.x, vel.y], true);
            }
        }
    }

    pub fn apply_impulse(&mut self, entity: EntityId, impulse: Vec2) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                // If you want “real impulse”, use rapier impulse APIs,
                // but for simple demos, velocity add is fine.
                let v = b.linvel();
                b.set_linvel(vector![v.x + impulse.x, v.y + impulse.y], true);
            }
        }
    }

    pub fn apply_force(&mut self, entity: EntityId, force: Vec2) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.add_force(vector![force.x, force.y], true);
            }
        }
    }

    pub fn apply_force_at_point(&mut self, entity: EntityId, force: Vec2, point: Vec2) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.add_force_at_point(vector![force.x, force.y], point![point.x, point.y], true);
            }
        }
    }

    pub fn set_angular_velocity(&mut self, entity: EntityId, w: f32) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.set_angvel(w, true);
            }
        }
    }

    /// Lock rotations for a body (useful for platformer characters).
    pub fn lock_rotations(&mut self, entity: EntityId, locked: bool) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.lock_rotations(locked, true);
            }
        }
    }

    pub fn set_linear_damping(&mut self, entity: EntityId, d: f32) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.set_linear_damping(d);
            }
        }
    }

    pub fn set_angular_damping(&mut self, entity: EntityId, d: f32) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.set_angular_damping(d);
            }
        }
    }

    /// Wake up a body (make it active in the physics simulation).
    /// The `strong` parameter determines if connected bodies should also be woken.
    pub fn wake_up(&mut self, entity: EntityId, strong: bool) {
        if let Some(h) = self.entity_to_body.get(&entity).copied() {
            if let Some(b) = self.rigid_bodies.get_mut(h) {
                b.wake_up(strong);
            }
        }
    }

    /// Update the query pipeline (call after adding/removing bodies/colliders).
    pub fn update_query_pipeline(&mut self) {
        self.query_pipeline
            .update(&self.island_manager, &self.rigid_bodies, &self.colliders);
    }

    // ------------------------------
    // Queries (engine-facing)
    // ------------------------------

    pub fn cast_ray(
        &self,
        origin: Vec2,
        direction: Vec2,
        max_toi: f32,
    ) -> Option<(EntityId, Vec2, f32)> {
        let ray = Ray::new(
            point![origin.x, origin.y],
            vector![direction.x, direction.y],
        );

        let (col_handle, toi) = self.query_pipeline.cast_ray(
            &self.rigid_bodies,
            &self.colliders,
            &ray,
            max_toi,
            true,
            QueryFilter::default(),
        )?;

        let collider = self.colliders.get(col_handle)?;
        let body = collider.parent()?;
        let entity = *self.body_to_entity.get(&body)?;

        let hit = ray.point_at(toi);
        Some((entity, Vec2::new(hit.x, hit.y), toi))
    }

    pub fn point_query(&self, p: Vec2) -> Option<EntityId> {
        let pt = point![p.x, p.y];
        for (_, c) in self.colliders.iter() {
            if c.shape().contains_point(c.position(), &pt) {
                let body = c.parent()?;
                return self.body_to_entity.get(&body).copied();
            }
        }
        None
    }

    /// Get all entities that have physics bodies.
    pub fn all_entities_with_bodies(&self) -> Vec<EntityId> {
        self.entity_to_body.keys().copied().collect()
    }

    /// Return true if an entity currently has a physics body.
    pub fn has_body(&self, entity: EntityId) -> bool {
        self.entity_to_body.contains_key(&entity)
    }

    /// Get linear velocity for an entity's body.
    pub fn linear_velocity(&self, entity: EntityId) -> Option<Vec2> {
        let h = *self.entity_to_body.get(&entity)?;
        let b = self.rigid_bodies.get(h)?;
        let v = b.linvel();
        Some(Vec2::new(v.x, v.y))
    }

    /// Get angular velocity for an entity's body.
    pub fn angular_velocity(&self, entity: EntityId) -> Option<f32> {
        let h = *self.entity_to_body.get(&entity)?;
        let b = self.rigid_bodies.get(h)?;
        Some(b.angvel())
    }

    /// Get body type for an entity.
    pub fn body_type(&self, entity: EntityId) -> Option<RigidBodyType> {
        let h = *self.entity_to_body.get(&entity)?;
        let b = self.rigid_bodies.get(h)?;
        match b.body_type() {
            rapier2d::prelude::RigidBodyType::Dynamic => Some(RigidBodyType::Dynamic),
            rapier2d::prelude::RigidBodyType::KinematicVelocityBased => {
                Some(RigidBodyType::Kinematic)
            }
            rapier2d::prelude::RigidBodyType::KinematicPositionBased => {
                Some(RigidBodyType::Kinematic)
            }
            rapier2d::prelude::RigidBodyType::Fixed => Some(RigidBodyType::Fixed),
        }
    }

    /// Get all colliders for an entity.
    /// Returns a vector of (shape, offset, density, friction, restitution, is_sensor) tuples.
    pub fn get_colliders(
        &self,
        entity: EntityId,
    ) -> Vec<(ColliderShape, Vec2, f32, f32, f32, bool)> {
        let body_handle = match self.entity_to_body.get(&entity) {
            Some(h) => *h,
            None => return Vec::new(),
        };

        let mut result = Vec::new();
        for (_, collider) in self.colliders.iter() {
            if collider.parent() == Some(body_handle) {
                // Calculate local offset: collider world pos - body world pos
                let collider_world_pos = collider.translation();
                let body_world_pos = self
                    .rigid_bodies
                    .get(body_handle)
                    .map(|b| b.translation())
                    .unwrap_or(collider_world_pos);
                let offset = Vec2::new(
                    collider_world_pos.x - body_world_pos.x,
                    collider_world_pos.y - body_world_pos.y,
                );

                let shape = match collider.shape().as_typed_shape() {
                    rapier2d::prelude::TypedShape::Cuboid(cuboid) => ColliderShape::Box {
                        hx: cuboid.half_extents.x,
                        hy: cuboid.half_extents.y,
                    },
                    rapier2d::prelude::TypedShape::Ball(ball) => ColliderShape::Circle {
                        radius: ball.radius,
                    },
                    rapier2d::prelude::TypedShape::Capsule(capsule) => ColliderShape::CapsuleY {
                        half_height: capsule.half_height(),
                        radius: capsule.radius,
                    },
                    _ => continue, // Skip unsupported shapes
                };

                result.push((
                    shape,
                    offset,
                    collider.density(),
                    collider.friction(),
                    collider.restitution(),
                    collider.is_sensor(),
                ));
            }
        }
        result
    }

    // ------------------------------
    // Private helpers
    // ------------------------------

    fn body_handle(&self, entity: EntityId) -> Result<RigidBodyHandle> {
        self.entity_to_body
            .get(&entity)
            .copied()
            .ok_or_else(|| anyhow!("Entity {:?} has no physics body", entity))
    }

    fn to_rapier_shape(&self, s: ColliderShape) -> SharedShape {
        match s {
            ColliderShape::Box { hx, hy } => SharedShape::cuboid(hx, hy),
            ColliderShape::Circle { radius } => SharedShape::ball(radius),
            ColliderShape::CapsuleY {
                half_height,
                radius,
            } => SharedShape::capsule_y(half_height, radius),
        }
    }

    fn collect_events(&mut self) {
        // Collision events (solid contact)
        while let Ok(ev) = self.event_recv_collision.try_recv() {
            match ev {
                CollisionEvent::Started(c1, c2, _) => {
                    if let Some((a, b, is_trigger)) = self.map_pair(c1, c2) {
                        let e = if is_trigger {
                            PhysicsEvent::TriggerEnter { a, b }
                        } else {
                            PhysicsEvent::CollisionEnter { a, b }
                        };
                        self.push_event(e);
                    }
                }
                CollisionEvent::Stopped(c1, c2, _) => {
                    if let Some((a, b, is_trigger)) = self.map_pair(c1, c2) {
                        let e = if is_trigger {
                            PhysicsEvent::TriggerExit { a, b }
                        } else {
                            PhysicsEvent::CollisionExit { a, b }
                        };
                        self.push_event(e);
                    }
                }
            }
        }

        // Note: Sensor (intersection) events are handled through CollisionEvent
        // with the is_trigger flag set, so no separate intersection handling needed.
    }

    fn map_pair(
        &self,
        c1: ColliderHandle,
        c2: ColliderHandle,
    ) -> Option<(EntityId, EntityId, bool)> {
        let col1 = self.colliders.get(c1)?;
        let col2 = self.colliders.get(c2)?;
        let b1 = col1.parent()?;
        let b2 = col2.parent()?;
        let e1 = *self.body_to_entity.get(&b1)?;
        let e2 = *self.body_to_entity.get(&b2)?;

        // sensor if either collider is a sensor
        let is_trigger = col1.is_sensor() || col2.is_sensor();
        Some((e1, e2, is_trigger))
    }

    fn push_event(&mut self, e: PhysicsEvent) {
        for cb in &self.callbacks {
            cb(e);
        }
        self.pending_events.push(e);
    }
}
