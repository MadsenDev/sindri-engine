use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};

/// Unique identifier for an entity in the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u32);

impl EntityId {
    /// Get the underlying integer ID (useful for debugging or serialization).
    pub fn to_u32(self) -> u32 {
        self.0
    }
}

/// Simple entity/world container with typed component storage.
///
/// This is a minimal "ECS-like" world:
/// - Entities are identified by `EntityId`
/// - Components are stored in type-based maps keyed by `EntityId`
/// - Components are indexed by their Rust type (`T: 'static`)
///
/// It is intentionally small and focused on:
/// - `spawn` / `despawn`
/// - `add` / `remove` / `get` components
/// - simple iteration over components of a single type
pub struct World {
    next_id: u32,
    alive: HashSet<EntityId>,
    storages: HashMap<TypeId, Box<dyn Any + Send>>,
}

impl World {
    /// Create a new, empty world.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            alive: HashSet::new(),
            storages: HashMap::new(),
        }
    }

    /// Spawn a new entity and return its `EntityId`.
    pub fn spawn(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.alive.insert(id);
        id
    }

    /// Despawn an entity, removing it and all of its components.
    pub fn despawn(&mut self, entity: EntityId) -> bool {
        if !self.alive.remove(&entity) {
            return false;
        }

        // Remove from all storages.
        for storage in self.storages.values_mut() {
            if let Some(map) = storage.downcast_mut::<HashMap<EntityId, Box<dyn Any + Send>>>() {
                map.remove(&entity);
            }
        }

        true
    }

    /// Check if an entity is currently alive.
    pub fn is_alive(&self, entity: EntityId) -> bool {
        self.alive.contains(&entity)
    }

    /// Number of alive entities.
    pub fn len(&self) -> usize {
        self.alive.len()
    }

    /// Return all currently alive entities.
    pub fn entities(&self) -> Vec<EntityId> {
        self.alive.iter().copied().collect()
    }

    /// Returns true if there are no entities in the world.
    pub fn is_empty(&self) -> bool {
        self.alive.is_empty()
    }

    /// Insert a component of type `T` for an entity, overwriting any existing component of that type.
    pub fn insert<T: Any + Send>(&mut self, entity: EntityId, component: T) {
        let type_id = TypeId::of::<T>();

        let storage = self
            .storages
            .entry(type_id)
            .or_insert_with(|| Box::new(HashMap::<EntityId, Box<dyn Any + Send>>::new()));

        let map = storage
            .downcast_mut::<HashMap<EntityId, Box<dyn Any + Send>>>()
            .expect("World storage type mismatch");

        map.insert(entity, Box::new(component));
    }

    /// Remove and return a component of type `T` for an entity, if it exists.
    pub fn remove<T: Any + Send>(&mut self, entity: EntityId) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let storage = self.storages.get_mut(&type_id)?;
        let map = storage
            .downcast_mut::<HashMap<EntityId, Box<dyn Any + Send>>>()
            .expect("World storage type mismatch");

        map.remove(&entity)
            .and_then(|boxed| boxed.downcast::<T>().ok())
            .map(|boxed| *boxed)
    }

    /// Get an immutable reference to a component of type `T` for an entity.
    pub fn get<T: Any + Send>(&self, entity: EntityId) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let storage = self.storages.get(&type_id)?;
        let map = storage
            .downcast_ref::<HashMap<EntityId, Box<dyn Any + Send>>>()
            .expect("World storage type mismatch");

        map.get(&entity).and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Get a mutable reference to a component of type `T` for an entity.
    pub fn get_mut<T: Any + Send>(&mut self, entity: EntityId) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        let storage = self.storages.get_mut(&type_id)?;
        let map = storage
            .downcast_mut::<HashMap<EntityId, Box<dyn Any + Send>>>()
            .expect("World storage type mismatch");

        map.get_mut(&entity)
            .and_then(|boxed| boxed.downcast_mut::<T>())
    }

    /// Iterate over all entities that have a component of type `T`, with mutable access.
    ///
    /// Returns a vector of `(EntityId, &mut T)` pairs tied to the `&mut self` lifetime.
    /// Use this when you need to mutate components in a single pass without
    /// collecting entity IDs first and looping again with [`World::get_mut`].
    ///
    /// # Example
    /// ```ignore
    /// for (entity, vel) in world.query_mut::<Velocity>() {
    ///     vel.x += acceleration * dt;
    /// }
    /// ```
    pub fn query_mut<T: Any + Send>(&mut self) -> Vec<(EntityId, &mut T)> {
        let type_id = TypeId::of::<T>();
        let storage = match self.storages.get_mut(&type_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let map = storage
            .downcast_mut::<HashMap<EntityId, Box<dyn Any + Send>>>()
            .expect("World storage type mismatch");

        // SAFETY: We collect raw pointers from the HashMap and convert them
        // back to `&mut T` references. This is sound because:
        // 1. Each entity has at most one component of type T (HashMap keys are
        //    unique EntityIds), so no two returned references alias the same
        //    memory.
        // 2. The `*mut T` pointers are derived from valid, live `Box<dyn Any>`
        //    values owned by `self`, so they are non-null and properly aligned.
        // 3. The produced `&mut T` lifetimes are bounded by `&mut self`, so
        //    callers cannot hold them past the mutable borrow of `World`.
        map.iter_mut()
            .filter_map(|(&entity, boxed)| {
                boxed.downcast_mut::<T>().map(|comp| {
                    let ptr: *mut T = comp as *mut T;
                    // SAFETY: ptr is non-overlapping with all other entries
                    // and lives as long as &mut self (see block comment above).
                    (entity, unsafe { &mut *ptr })
                })
            })
            .collect()
    }

    /// Iterate over all entities that have a component of type `T`.
    ///
    /// Returns a vector of `(EntityId, &T)` pairs.
    /// For simplicity (and to avoid lifetime gymnastics) this collects
    /// results into an owned `Vec`. For most games this is sufficient.
    pub fn query<T: Any + Send>(&self) -> Vec<(EntityId, &T)> {
        let type_id = TypeId::of::<T>();
        let storage = match self.storages.get(&type_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let map = storage
            .downcast_ref::<HashMap<EntityId, Box<dyn Any + Send>>>()
            .expect("World storage type mismatch");

        map.iter()
            .filter_map(|(&entity, boxed)| boxed.downcast_ref::<T>().map(|comp| (entity, comp)))
            .collect()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    /// Restore an entity with a specific ID (for scene loading/play mode restore).
    /// This marks the entity as alive and ensures it can be queried.
    ///
    /// WARNING: This can cause ID conflicts if the ID is already in use.
    /// Only use this when restoring from a snapshot where you control all IDs.
    pub fn restore_entity(&mut self, entity_id: EntityId) {
        self.alive.insert(entity_id);
        // Update next_id to avoid conflicts with future spawns
        let id_num = entity_id.to_u32();
        if id_num >= self.next_id {
            self.next_id = id_num.wrapping_add(1).max(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Counter {
        value: i32,
    }

    #[test]
    fn query_mut_modifies_in_place() {
        let mut world = World::new();
        let a = world.spawn();
        let b = world.spawn();
        world.insert(a, Counter { value: 1 });
        world.insert(b, Counter { value: 2 });

        // Mutate all counters in a single pass — no second loop needed.
        for (_entity, counter) in world.query_mut::<Counter>() {
            counter.value *= 10;
        }

        assert_eq!(world.get::<Counter>(a).unwrap().value, 10);
        assert_eq!(world.get::<Counter>(b).unwrap().value, 20);
    }
}
