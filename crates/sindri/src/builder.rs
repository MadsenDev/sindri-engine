use rapier2d::prelude::ColliderHandle;

use crate::entities::{Name, PhysicsBody, SpriteComponent, Transform};
use crate::math::Vec2;
use crate::physics::{ColliderShape, PhysicsWorld, RigidBodyType};
use crate::render::TextureHandle;
use crate::script::{ScriptComponent, ScriptParams, ScriptTag};
use crate::world::{EntityId, World};

/// Fluent helper for spawning an entity with common components and physics setup.
///
/// # Example
///
/// ```ignore
/// let player = EntityBuilder::new(&mut world, &mut physics)
///     .at(Vec2::new(200.0, 300.0))
///     .dynamic()
///     .box_collider(14.0, 18.0)
///     .friction(0.3)
///     .lock_rotation()
///     .sprite(player_texture)
///     .script("scripts/player.lua", ScriptParams::default().insert("speed", 200.0f32))
///     .tag("player")
///     .build();
/// ```
pub struct EntityBuilder<'a> {
    world: &'a mut World,
    physics: &'a mut PhysicsWorld,
    entity: EntityId,
    position: Vec2,
    body_type: Option<RigidBodyType>,
    last_collider: Option<ColliderHandle>,
}

impl<'a> EntityBuilder<'a> {
    pub fn new(world: &'a mut World, physics: &'a mut PhysicsWorld) -> Self {
        let entity = world.spawn();
        Self {
            world,
            physics,
            entity,
            position: Vec2::ZERO,
            body_type: None,
            last_collider: None,
        }
    }

    pub fn at(mut self, position: Vec2) -> Self {
        self.position = position;
        self.world.insert(self.entity, Transform::new(position));
        if self.body_type.is_some() {
            self.physics.set_body_position(self.entity, position);
        }
        self
    }

    pub fn with_name(self, name: &str) -> Self {
        self.world.insert(self.entity, Name::new(name));
        self
    }

    pub fn dynamic(self) -> Self {
        self.with_body(RigidBodyType::Dynamic)
    }

    pub fn kinematic(self) -> Self {
        self.with_body(RigidBodyType::Kinematic)
    }

    pub fn fixed(self) -> Self {
        self.with_body(RigidBodyType::Fixed)
    }

    pub fn box_collider(mut self, hx: f32, hy: f32) -> Self {
        self.add_collider(ColliderShape::Box { hx, hy });
        self
    }

    pub fn circle_collider(mut self, radius: f32) -> Self {
        self.add_collider(ColliderShape::Circle { radius });
        self
    }

    pub fn capsule_collider(mut self, half_height: f32, radius: f32) -> Self {
        self.add_collider(ColliderShape::CapsuleY {
            half_height,
            radius,
        });
        self
    }

    pub fn friction(self, f: f32) -> Self {
        if let Some(collider) = self.last_collider {
            self.physics.set_collider_friction(collider, f);
        }
        self
    }

    pub fn restitution(self, r: f32) -> Self {
        if let Some(collider) = self.last_collider {
            self.physics.set_collider_restitution(collider, r);
        }
        self
    }

    pub fn sensor(self) -> Self {
        if let Some(collider) = self.last_collider {
            self.physics.set_collider_sensor(collider, true);
        }
        self
    }

    pub fn lock_rotation(self) -> Self {
        self.physics.lock_rotations(self.entity, true);
        self
    }

    pub fn sprite(self, texture: TextureHandle) -> Self {
        let mut sprite = SpriteComponent::new(texture);
        sprite.sprite.transform.position = self.position;
        self.world.insert(self.entity, sprite);
        self
    }

    pub fn script(self, path: &str, params: ScriptParams) -> Self {
        if let Some(component) = self.world.get_mut::<ScriptComponent>(self.entity) {
            component.scripts.push(crate::script::ScriptAttachment {
                path: path.to_string(),
                params,
            });
        } else {
            self.world.insert(
                self.entity,
                ScriptComponent::default().with_script(path.to_string(), params),
            );
        }
        self
    }

    pub fn tag(self, tag: &str) -> Self {
        self.world.insert(self.entity, ScriptTag(tag.to_string()));
        self
    }

    pub fn build(self) -> EntityId {
        self.entity
    }

    fn with_body(mut self, body_type: RigidBodyType) -> Self {
        if self
            .physics
            .create_body(self.entity, body_type, self.position, 0.0)
            .is_ok()
        {
            self.world.insert(self.entity, PhysicsBody::new(body_type));
            self.body_type = Some(body_type);
            self.last_collider = None;
        }
        self
    }

    fn ensure_body(&mut self) {
        if self.body_type.is_none() {
            let _ = self
                .physics
                .create_body(self.entity, RigidBodyType::Fixed, self.position, 0.0);
            self.world
                .insert(self.entity, PhysicsBody::new(RigidBodyType::Fixed));
            self.body_type = Some(RigidBodyType::Fixed);
        }
    }

    fn add_collider(&mut self, shape: ColliderShape) {
        self.ensure_body();
        if let Ok(handle) = self.physics.add_collider_with_material_handle(
            self.entity,
            shape,
            Vec2::ZERO,
            1.0,
            0.5,
            0.0,
        ) {
            self.last_collider = Some(handle);
            if let Some(body) = self.world.get_mut::<PhysicsBody>(self.entity) {
                body.collider_shape = Some(shape);
            }
        }
    }
}
