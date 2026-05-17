use crate::entities::{CarriedResource, EntityType, GameEntity, UnitAction};
use sindri::{AStarPathfinder, GridNode, PathfindingGrid, Vec2};

pub struct PathfindingSystem {
    pub grid: PathfindingGrid,
    pub cell_size: f32,
}

impl PathfindingSystem {
    pub fn new(world_size: Vec2, cell_size: f32) -> Self {
        let grid_width = (world_size.x / cell_size) as usize;
        let grid_height = (world_size.y / cell_size) as usize;

        Self {
            grid: PathfindingGrid::new(grid_width, grid_height, cell_size),
            cell_size,
        }
    }

    pub fn mark_obstacle(&mut self, position: Vec2, size: Vec2) {
        // Add padding to ensure units stay outside buildings
        let padding = 10.0; // Extra space around buildings
        let half_size = size * 0.5 + Vec2::new(padding, padding);
        let min_pos = position - half_size;
        let max_pos = position + half_size;

        let min_node = self.grid.world_to_grid(min_pos);
        let max_node = self.grid.world_to_grid(max_pos);

        for x in min_node.x..=max_node.x {
            for y in min_node.y..=max_node.y {
                let node = GridNode::new(x, y);
                if self.grid.is_valid(&node) {
                    self.grid.set_walkable(node, false);
                }
            }
        }
    }

    pub fn is_position_blocked(&self, position: Vec2) -> bool {
        let node = self.grid.world_to_grid(position);
        if self.grid.is_valid(&node) {
            !self.grid.is_walkable(&node)
        } else {
            true // Out of bounds is considered blocked
        }
    }

    pub fn find_path(&self, start: Vec2, goal: Vec2) -> Option<Vec<Vec2>> {
        AStarPathfinder::find_path(&self.grid, start, goal)
    }
}

pub struct MovementSystem;

impl MovementSystem {
    pub fn update_unit_movement(entity: &mut GameEntity, pathfinding: &PathfindingSystem, dt: f32) {
        // Follow path if available
        if !entity.path.is_empty() && entity.path_index < entity.path.len() {
            let next_waypoint = entity.path[entity.path_index];
            let dir = next_waypoint - entity.sprite.transform.position;
            let distance = dir.length();

            if distance > 5.0 {
                let move_dist = entity.speed * dt;
                let new_pos = if move_dist < distance {
                    entity.sprite.transform.position + dir.normalized() * move_dist
                } else {
                    next_waypoint
                };

                // Check if new position is blocked
                if !pathfinding.is_position_blocked(new_pos) {
                    entity.sprite.transform.position = new_pos;
                    if move_dist >= distance {
                        entity.path_index += 1;
                    }
                } else {
                    // Hit an obstacle, try to find a new path
                    entity.path.clear();
                    entity.path_index = 0;
                }
            } else {
                entity.path_index += 1;
            }

            // Check if reached end of path
            if entity.path_index >= entity.path.len() {
                entity.path.clear();
                entity.path_index = 0;
                entity.target_position = None;
                // Only set to Idle if not in a persistent action state
                match entity.action {
                    UnitAction::Gathering(_)
                    | UnitAction::Delivering(_)
                    | UnitAction::Building(_) => {
                        // Keep the action - these states manage themselves
                    }
                    _ => {
                        entity.action = UnitAction::Idle;
                    }
                }
            }
        } else if let Some(target) = entity.target_position {
            // Direct movement (fallback) - but check for obstacles
            let dir = target - entity.sprite.transform.position;
            let distance = dir.length();

            if distance > 5.0 {
                let move_dist = entity.speed * dt;
                let new_pos = if move_dist < distance {
                    entity.sprite.transform.position + dir.normalized() * move_dist
                } else {
                    target
                };

                // Check if new position is blocked
                if !pathfinding.is_position_blocked(new_pos) {
                    entity.sprite.transform.position = new_pos;
                    if move_dist >= distance {
                        entity.target_position = None;
                        // Only set to Idle if not in a persistent action state
                        match entity.action {
                            UnitAction::Gathering(_)
                            | UnitAction::Delivering(_)
                            | UnitAction::Building(_) => {
                                // Keep the action - these states manage themselves
                            }
                            _ => {
                                entity.action = UnitAction::Idle;
                            }
                        }
                    }
                } else {
                    // Blocked - clear target and try to find a path instead
                    entity.target_position = None;
                }
            } else {
                entity.target_position = None;
                // Only set to Idle if not in a persistent action state
                match entity.action {
                    UnitAction::Gathering(_)
                    | UnitAction::Delivering(_)
                    | UnitAction::Building(_) => {
                        // Keep the action - these states manage themselves
                    }
                    _ => {
                        entity.action = UnitAction::Idle;
                    }
                }
            }
        }
    }
}

pub struct GatheringSystem;

impl GatheringSystem {
    pub fn update_gathering(
        entities: &mut [GameEntity],
        pathfinding: &PathfindingSystem,
        unit_idx: usize,
        resource_idx: usize,
        dt: f32,
    ) -> Option<usize> {
        // Returns Some(town_center_idx) if inventory is full and needs to deliver
        let unit_pos = entities[unit_idx].sprite.transform.position;

        if resource_idx < entities.len() {
            if entities[resource_idx].resource_amount == 0 {
                // Resource depleted
                entities[unit_idx].action = UnitAction::Idle;
                entities[unit_idx].resource_target = None;
                return None;
            }

            let dist = (unit_pos - entities[resource_idx].sprite.transform.position).length();
            let gather_range = 30.0;

            if dist > gather_range {
                // Move closer to resource
                let start = unit_pos;
                let target = entities[resource_idx].sprite.transform.position;
                if let Some(path) = pathfinding.find_path(start, target) {
                    let unit = &mut entities[unit_idx];
                    unit.path = path;
                    unit.path_index = 0;
                    unit.target_position = Some(target);
                } else {
                    entities[unit_idx].target_position = Some(target);
                    entities[unit_idx].path.clear();
                }
            } else {
                // Gather resources into inventory
                // First, get resource type and amount (immutable borrow)
                let resource_type = match entities[resource_idx].entity_type {
                    EntityType::Tree => CarriedResource::Wood,
                    EntityType::Gold => CarriedResource::Gold,
                    EntityType::Stone => CarriedResource::Stone,
                    _ => return None,
                };
                let resource_amount = entities[resource_idx].resource_amount;

                // Update gather timer
                entities[unit_idx].gather_timer += dt;

                if entities[unit_idx].gather_timer >= 1.0 / entities[unit_idx].gather_rate {
                    entities[unit_idx].gather_timer = 0.0;

                    // Set carried resource type if not set
                    if entities[unit_idx].carried_resource.is_none() {
                        entities[unit_idx].carried_resource = Some(resource_type);
                    }

                    // Only gather if carrying the same resource type
                    if entities[unit_idx].carried_resource == Some(resource_type) {
                        let gather_amount = 10u32;
                        let available = resource_amount.min(gather_amount);
                        let space_left = entities[unit_idx]
                            .max_carry_capacity
                            .saturating_sub(entities[unit_idx].carried_amount);
                        let amount = available.min(space_left);

                        if amount > 0 {
                            // Update resource and unit inventory
                            entities[resource_idx].resource_amount -= amount;
                            entities[unit_idx].carried_amount += amount;

                            // Check if inventory is full
                            if entities[unit_idx].carried_amount
                                >= entities[unit_idx].max_carry_capacity
                            {
                                // Find nearest appropriate drop-off building for this resource type
                                let mut nearest_dropoff: Option<(usize, f32)> = None;
                                for (idx, entity) in entities.iter().enumerate() {
                                    if entity.accepts_resource(resource_type) {
                                        let dist =
                                            (unit_pos - entity.sprite.transform.position).length();
                                        if let Some((_, min_dist)) = nearest_dropoff {
                                            if dist < min_dist {
                                                nearest_dropoff = Some((idx, dist));
                                            }
                                        } else {
                                            nearest_dropoff = Some((idx, dist));
                                        }
                                    }
                                }

                                if let Some((dropoff_idx, _)) = nearest_dropoff {
                                    return Some(dropoff_idx);
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // Resource no longer exists
            entities[unit_idx].action = UnitAction::Idle;
            entities[unit_idx].resource_target = None;
        }

        None
    }
}

pub struct DeliverySystem;

impl DeliverySystem {
    pub fn update_delivery(
        entities: &mut [GameEntity],
        pathfinding: &PathfindingSystem,
        unit_idx: usize,
        dropoff_building_idx: usize,
        _dt: f32,
        resources: &mut crate::resources::Resources,
    ) -> Option<usize> {
        // Returns Some(resource_idx) if delivery complete and should return to gathering
        let unit_pos = entities[unit_idx].sprite.transform.position;

        if dropoff_building_idx < entities.len() {
            let building_pos = entities[dropoff_building_idx].sprite.transform.position;
            let dist = (unit_pos - building_pos).length();
            let delivery_range = 50.0;

            if dist > delivery_range {
                // Move to drop-off building
                let start = unit_pos;
                if let Some(path) = pathfinding.find_path(start, building_pos) {
                    let unit = &mut entities[unit_idx];
                    unit.path = path;
                    unit.path_index = 0;
                    unit.target_position = Some(building_pos);
                } else {
                    entities[unit_idx].target_position = Some(building_pos);
                    entities[unit_idx].path.clear();
                }
            } else {
                // At drop-off building - deliver resources
                // Get all needed info before any mutable borrows
                let resource_target = entities[unit_idx].resource_target;
                let carried_amount = entities[unit_idx].carried_amount;
                let carried_resource = entities[unit_idx].carried_resource;

                // If resource_target is not set, try to extract it from the action
                // (This shouldn't happen, but let's be defensive)
                if resource_target.is_none() {
                    // Can't extract from Delivering action, so we'll go idle after delivery
                }

                // Check if resource still exists and has resources (before mutable borrow)
                let should_return_to_gathering = if let Some(resource_idx) = resource_target {
                    resource_idx < entities.len() && entities[resource_idx].resource_amount > 0
                } else {
                    false
                };

                // Now do mutable operations
                let unit = &mut entities[unit_idx];

                if carried_amount > 0 {
                    // Deliver resources
                    if let Some(resource_type) = carried_resource {
                        match resource_type {
                            CarriedResource::Wood => resources.wood += carried_amount,
                            CarriedResource::Gold => resources.gold += carried_amount,
                            CarriedResource::Stone => resources.stone += carried_amount,
                        }
                    }
                    unit.carried_amount = 0;
                    // Keep carried_resource type so we can search for more of the same type
                    // It will be cleared when we find a new resource or go idle
                }

                // After delivery (or if already delivered), return to gathering if we have a valid resource target
                if should_return_to_gathering {
                    if let Some(resource_idx) = resource_target {
                        // Clear target before returning
                        unit.resource_target = None;
                        return Some(resource_idx);
                    }
                } else {
                    // Resource is depleted, doesn't exist, or resource_target is None
                    // Clear target - game loop will handle state transition
                    if resource_target.is_some() {
                        unit.resource_target = None;
                    }
                }
            }
        }

        None
    }
}

pub struct BuildingSystem;

impl BuildingSystem {
    pub fn update_building(
        entities: &mut [GameEntity],
        pathfinding: &mut PathfindingSystem,
        unit_idx: usize,
        build_idx: usize,
        dt: f32,
    ) {
        if build_idx < entities.len() {
            let building_pos = entities[build_idx].sprite.transform.position;
            let building_type = entities[build_idx].entity_type;

            entities[build_idx].build_progress += dt * 0.5; // 2 seconds to build

            if entities[build_idx].build_progress >= 1.0 {
                entities[build_idx].build_progress = 1.0;
                // Building complete - mark as obstacle
                let size = match building_type {
                    EntityType::House => Vec2::new(50.0, 50.0),
                    EntityType::LumberMill => Vec2::new(60.0, 60.0),
                    EntityType::Mine => Vec2::new(60.0, 60.0),
                    EntityType::TownCenter => Vec2::new(80.0, 80.0),
                    _ => Vec2::new(50.0, 50.0),
                };
                pathfinding.mark_obstacle(building_pos, size);

                // Clear build target from unit
                if unit_idx < entities.len() {
                    entities[unit_idx].action = UnitAction::Idle;
                    entities[unit_idx].build_target = None;
                }
            }
        }
    }
}
