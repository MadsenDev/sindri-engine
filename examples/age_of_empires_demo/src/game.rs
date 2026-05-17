use anyhow::Result;
use sindri::{BuiltinFont, Camera2D, EngineContext, Game, KeyCode, MouseButton, Sprite, Vec2};

use crate::entities::{EntityType, GameEntity};
use crate::resources::{BuildingCosts, Resources};
use crate::systems::{
    BuildingSystem, DeliverySystem, GatheringSystem, MovementSystem, PathfindingSystem,
};
use crate::ui::UiSystem;

pub struct AgeOfEmpiresDemo {
    // Camera
    camera: Camera2D,
    camera_speed: f32,

    // World
    world_size: Vec2,
    ground_tiles: Vec<Sprite>,

    // Systems
    pathfinding: PathfindingSystem,

    // Entities
    entities: Vec<GameEntity>,
    selected_entities: Vec<usize>,

    // Resources
    resources: Resources,
    costs: BuildingCosts,

    // Textures
    textures: TextureHandles,

    // UI
    ui: UiSystem,

    // Building placement
    building_ghost: Option<(EntityType, Vec2)>,
}

struct TextureHandles {
    villager: Option<sindri::TextureHandle>,
    military: Option<sindri::TextureHandle>,
    town_center: Option<sindri::TextureHandle>,
    house: Option<sindri::TextureHandle>,
    lumber_mill: Option<sindri::TextureHandle>,
    mine: Option<sindri::TextureHandle>,
    tree: Option<sindri::TextureHandle>,
    gold: Option<sindri::TextureHandle>,
    stone: Option<sindri::TextureHandle>,
    ground: Option<sindri::TextureHandle>,
    construction: Option<sindri::TextureHandle>,
}

impl AgeOfEmpiresDemo {
    pub fn new() -> Self {
        let grid_cell_size = 32.0;
        let world_size = Vec2::new(2000.0, 2000.0);

        Self {
            camera: Camera2D::default(),
            camera_speed: 300.0,
            world_size,
            ground_tiles: Vec::new(),
            pathfinding: PathfindingSystem::new(world_size, grid_cell_size),
            entities: Vec::new(),
            selected_entities: Vec::new(),
            resources: Resources::new(),
            costs: BuildingCosts::new(),
            textures: TextureHandles {
                villager: None,
                military: None,
                town_center: None,
                house: None,
                lumber_mill: None,
                mine: None,
                tree: None,
                gold: None,
                stone: None,
                ground: None,
                construction: None,
            },
            ui: UiSystem::new(),
            building_ghost: None,
        }
    }

    fn create_textures(&mut self, renderer: &mut sindri::Renderer) -> Result<()> {
        fn solid_color(r: u8, g: u8, b: u8, width: u32, height: u32) -> Vec<u8> {
            let mut data = Vec::with_capacity((width * height * 4) as usize);
            for _ in 0..(width * height) {
                data.push(r);
                data.push(g);
                data.push(b);
                data.push(255);
            }
            data
        }

        self.textures.villager =
            Some(renderer.load_texture_from_rgba(&solid_color(64, 128, 255, 24, 24), 24, 24)?);
        self.textures.military =
            Some(renderer.load_texture_from_rgba(&solid_color(255, 64, 64, 28, 28), 28, 28)?);
        self.textures.town_center =
            Some(renderer.load_texture_from_rgba(&solid_color(139, 90, 43, 80, 80), 80, 80)?);
        self.textures.house =
            Some(renderer.load_texture_from_rgba(&solid_color(160, 100, 50, 50, 50), 50, 50)?);
        self.textures.lumber_mill =
            Some(renderer.load_texture_from_rgba(&solid_color(101, 67, 33, 60, 60), 60, 60)?); // Brown
        self.textures.mine =
            Some(renderer.load_texture_from_rgba(&solid_color(105, 105, 105, 60, 60), 60, 60)?); // Dark gray
        self.textures.tree =
            Some(renderer.load_texture_from_rgba(&solid_color(34, 139, 34, 40, 60), 40, 60)?);
        self.textures.gold =
            Some(renderer.load_texture_from_rgba(&solid_color(255, 215, 0, 30, 30), 30, 30)?);
        self.textures.stone =
            Some(renderer.load_texture_from_rgba(&solid_color(128, 128, 128, 30, 30), 30, 30)?);
        self.textures.ground =
            Some(renderer.load_texture_from_rgba(&solid_color(124, 252, 0, 128, 128), 128, 128)?);
        self.textures.construction =
            Some(renderer.load_texture_from_rgba(&solid_color(139, 90, 43, 50, 50), 50, 50)?);

        Ok(())
    }

    fn create_ground_tiles(&mut self) {
        self.ground_tiles.clear();
        let tile_size = 128.0;
        let half_tile = tile_size * 0.5;

        if let Some(ground_tex) = self.textures.ground {
            for x in 0..=(self.world_size.x / tile_size) as i32 {
                for y in 0..=(self.world_size.y / tile_size) as i32 {
                    let mut tile = Sprite::new(ground_tex);
                    tile.set_size_px(Vec2::new(tile_size, tile_size), Vec2::new(128.0, 128.0));
                    tile.transform.position = Vec2::new(
                        x as f32 * tile_size + half_tile,
                        y as f32 * tile_size + half_tile,
                    );
                    self.ground_tiles.push(tile);
                }
            }
        }
    }

    fn spawn_entity(&mut self, entity_type: EntityType, position: Vec2) -> Option<usize> {
        let texture = match entity_type {
            EntityType::Villager => self.textures.villager?,
            EntityType::Military => self.textures.military?,
            EntityType::TownCenter => self.textures.town_center?,
            EntityType::House => self.textures.house?,
            EntityType::LumberMill => self.textures.lumber_mill?,
            EntityType::Mine => self.textures.mine?,
            EntityType::Tree => self.textures.tree?,
            EntityType::Gold => self.textures.gold?,
            EntityType::Stone => self.textures.stone?,
        };

        let (size, speed) = match entity_type {
            EntityType::Villager => (Vec2::new(24.0, 24.0), 80.0),
            EntityType::Military => (Vec2::new(28.0, 28.0), 100.0),
            EntityType::TownCenter => (Vec2::new(80.0, 80.0), 0.0),
            EntityType::House => (Vec2::new(50.0, 50.0), 0.0),
            EntityType::LumberMill => (Vec2::new(60.0, 60.0), 0.0),
            EntityType::Mine => (Vec2::new(60.0, 60.0), 0.0),
            EntityType::Tree => (Vec2::new(40.0, 60.0), 0.0),
            EntityType::Gold => (Vec2::new(30.0, 30.0), 0.0),
            EntityType::Stone => (Vec2::new(30.0, 30.0), 0.0),
        };

        let mut sprite = Sprite::new(texture);
        sprite.set_size_px(size, size);
        sprite.transform.position = position;

        if entity_type == EntityType::Tree {
            sprite.tint = [
                0.8 + (position.x as u32 % 3) as f32 * 0.1,
                1.0,
                0.8 + (position.y as u32 % 3) as f32 * 0.1,
                1.0,
            ];
        }

        // Mark obstacles in pathfinding grid
        if matches!(
            entity_type,
            EntityType::TownCenter
                | EntityType::House
                | EntityType::LumberMill
                | EntityType::Mine
                | EntityType::Tree
                | EntityType::Gold
                | EntityType::Stone
        ) {
            self.pathfinding.mark_obstacle(position, size);
        }

        let resource_amount = match entity_type {
            EntityType::Tree => 100,
            EntityType::Gold => 200,
            EntityType::Stone => 150,
            _ => 0,
        };

        let entity = GameEntity::new(sprite, entity_type, speed, resource_amount);
        self.entities.push(entity);
        Some(self.entities.len() - 1)
    }

    fn select_entity_at(&mut self, world_pos: Vec2) {
        for idx in &self.selected_entities {
            if let Some(entity) = self.entities.get_mut(*idx) {
                entity.selected = false;
            }
        }
        self.selected_entities.clear();

        for (idx, entity) in self.entities.iter().enumerate().rev() {
            let dist = (entity.sprite.transform.position - world_pos).length();
            if dist < entity.get_selection_radius() && entity.is_unit() {
                self.selected_entities.push(idx);
                if let Some(e) = self.entities.get_mut(idx) {
                    e.selected = true;
                }
                break;
            }
        }
    }

    fn find_resource_at(&self, world_pos: Vec2) -> Option<usize> {
        for (idx, entity) in self.entities.iter().enumerate() {
            if entity.is_resource() {
                let dist = (entity.sprite.transform.position - world_pos).length();
                let radius = match entity.entity_type {
                    EntityType::Tree => 30.0,
                    EntityType::Gold | EntityType::Stone => 20.0,
                    _ => 0.0,
                };

                if dist < radius && entity.resource_amount > 0 {
                    return Some(idx);
                }
            }
        }
        None
    }

    fn find_dropoff_building_at(&self, world_pos: Vec2) -> Option<usize> {
        for (idx, entity) in self.entities.iter().enumerate() {
            if entity.is_building() {
                let dist = (entity.sprite.transform.position - world_pos).length();
                let radius = match entity.entity_type {
                    EntityType::TownCenter => 40.0,
                    EntityType::House | EntityType::LumberMill | EntityType::Mine => 30.0,
                    _ => 0.0,
                };

                if dist < radius {
                    return Some(idx);
                }
            }
        }
        None
    }

    fn find_nearest_resource(
        &self,
        position: Vec2,
        resource_type: crate::entities::CarriedResource,
        search_radius: f32,
    ) -> Option<usize> {
        let target_entity_type = match resource_type {
            crate::entities::CarriedResource::Wood => EntityType::Tree,
            crate::entities::CarriedResource::Gold => EntityType::Gold,
            crate::entities::CarriedResource::Stone => EntityType::Stone,
        };

        let mut nearest: Option<(usize, f32)> = None;

        for (idx, entity) in self.entities.iter().enumerate() {
            if entity.entity_type == target_entity_type && entity.resource_amount > 0 {
                let dist = (entity.sprite.transform.position - position).length();
                if dist <= search_radius {
                    if let Some((_, min_dist)) = nearest {
                        if dist < min_dist {
                            nearest = Some((idx, dist));
                        }
                    } else {
                        nearest = Some((idx, dist));
                    }
                }
            }
        }

        nearest.map(|(idx, _)| idx)
    }

    fn move_selected_units_to(&mut self, target: Vec2) {
        let selected_indices: Vec<usize> = self.selected_entities.clone();
        let mut unit_starts: Vec<(usize, Vec2)> = Vec::new();

        for &idx in &selected_indices {
            if let Some(entity) = self.entities.get(idx) {
                if entity.speed > 0.0 {
                    unit_starts.push((idx, entity.sprite.transform.position));
                }
            }
        }

        let resource_idx = self.find_resource_at(target);

        for (idx, start) in unit_starts {
            if let Some(entity) = self.entities.get_mut(idx) {
                if let Some(res_idx) = resource_idx {
                    if entity.entity_type == EntityType::Villager {
                        entity.action = crate::entities::UnitAction::Gathering(res_idx);
                        entity.resource_target = Some(res_idx);
                    } else {
                        entity.action = crate::entities::UnitAction::Moving;
                        entity.resource_target = None;
                    }
                } else {
                    entity.action = crate::entities::UnitAction::Moving;
                    entity.resource_target = None;
                }

                if let Some(path) = self.pathfinding.find_path(start, target) {
                    entity.path = path;
                    entity.path_index = 0;
                    entity.target_position = Some(target);
                } else {
                    entity.target_position = Some(target);
                    entity.path.clear();
                }
            }
        }
    }

    fn update_units(&mut self, dt: f32) {
        let entity_count = self.entities.len();
        for idx in 0..entity_count {
            match self.entities[idx].action {
                crate::entities::UnitAction::Idle => {}
                crate::entities::UnitAction::Moving => {
                    MovementSystem::update_unit_movement(
                        &mut self.entities[idx],
                        &self.pathfinding,
                        dt,
                    );
                }
                crate::entities::UnitAction::Gathering(resource_idx) => {
                    if let Some(town_center_idx) = GatheringSystem::update_gathering(
                        &mut self.entities,
                        &self.pathfinding,
                        idx,
                        resource_idx,
                        dt,
                    ) {
                        // Inventory is full, switch to delivering
                        // ALWAYS set resource_target from the action to ensure it's preserved
                        self.entities[idx].resource_target = Some(resource_idx);
                        self.entities[idx].action =
                            crate::entities::UnitAction::Delivering(town_center_idx);
                        self.entities[idx].target_position =
                            Some(self.entities[town_center_idx].sprite.transform.position);
                        self.entities[idx].path.clear();
                    } else {
                        // Continue moving if needed
                        MovementSystem::update_unit_movement(
                            &mut self.entities[idx],
                            &self.pathfinding,
                            dt,
                        );
                    }
                }
                crate::entities::UnitAction::Delivering(town_center_idx) => {
                    if let Some(resource_idx) = DeliverySystem::update_delivery(
                        &mut self.entities,
                        &self.pathfinding,
                        idx,
                        town_center_idx,
                        dt,
                        &mut self.resources,
                    ) {
                        // Delivery complete, return to gathering
                        self.entities[idx].action =
                            crate::entities::UnitAction::Gathering(resource_idx);
                        self.entities[idx].resource_target = Some(resource_idx);
                        let target = self.entities[resource_idx].sprite.transform.position;
                        if let Some(path) = self
                            .pathfinding
                            .find_path(self.entities[idx].sprite.transform.position, target)
                        {
                            self.entities[idx].path = path;
                            self.entities[idx].path_index = 0;
                            self.entities[idx].target_position = Some(target);
                        } else {
                            self.entities[idx].target_position = Some(target);
                            self.entities[idx].path.clear();
                        }
                    } else {
                        // Check if we're stuck at drop-off building with no resource target
                        let unit_pos = self.entities[idx].sprite.transform.position;
                        if town_center_idx < self.entities.len() {
                            let building_pos =
                                self.entities[town_center_idx].sprite.transform.position;
                            let dist = (unit_pos - building_pos).length();
                            if dist <= 50.0
                                && self.entities[idx].resource_target.is_none()
                                && self.entities[idx].carried_amount == 0
                            {
                                // Original resource is gone - search for nearest resource of the same type
                                if let Some(carried_resource) = self.entities[idx].carried_resource
                                {
                                    // Still have the resource type info, search for nearest
                                    let search_radius = 200.0; // Search within 200 units
                                    if let Some(new_resource_idx) = self.find_nearest_resource(
                                        unit_pos,
                                        carried_resource,
                                        search_radius,
                                    ) {
                                        // Found a new resource - go gather it
                                        self.entities[idx].action =
                                            crate::entities::UnitAction::Gathering(
                                                new_resource_idx,
                                            );
                                        self.entities[idx].resource_target = Some(new_resource_idx);
                                        // Clear carried_resource - gathering system will set it based on the new resource
                                        self.entities[idx].carried_resource = None;
                                        let target = self.entities[new_resource_idx]
                                            .sprite
                                            .transform
                                            .position;
                                        if let Some(path) =
                                            self.pathfinding.find_path(unit_pos, target)
                                        {
                                            self.entities[idx].path = path;
                                            self.entities[idx].path_index = 0;
                                            self.entities[idx].target_position = Some(target);
                                        } else {
                                            self.entities[idx].target_position = Some(target);
                                            self.entities[idx].path.clear();
                                        }
                                    } else {
                                        // No resources found - go idle
                                        self.entities[idx].action =
                                            crate::entities::UnitAction::Idle;
                                        self.entities[idx].carried_resource = None;
                                    }
                                } else {
                                    // No resource type info - go idle
                                    self.entities[idx].action = crate::entities::UnitAction::Idle;
                                }
                            } else {
                                // Continue moving to drop-off building
                                MovementSystem::update_unit_movement(
                                    &mut self.entities[idx],
                                    &self.pathfinding,
                                    dt,
                                );
                            }
                        } else {
                            // Drop-off building doesn't exist - go idle
                            self.entities[idx].action = crate::entities::UnitAction::Idle;
                        }
                    }
                }
                crate::entities::UnitAction::Building(build_idx) => {
                    BuildingSystem::update_building(
                        &mut self.entities,
                        &mut self.pathfinding,
                        idx,
                        build_idx,
                        dt,
                    );
                }
            }
        }
    }

    fn try_spawn_villager(&mut self) {
        if self.resources.can_afford_villager(self.costs.villager_food) {
            for idx in 0..self.entities.len() {
                if self.entities[idx].entity_type == EntityType::TownCenter {
                    let spawn_pos =
                        self.entities[idx].sprite.transform.position + Vec2::new(60.0, 0.0);
                    if self.spawn_entity(EntityType::Villager, spawn_pos).is_some() {
                        self.resources.spend_villager(self.costs.villager_food);
                    }
                    break;
                }
            }
        }
    }

    fn try_build_house(&mut self, position: Vec2) {
        if self
            .resources
            .can_afford_house(self.costs.house_wood, self.costs.house_stone)
        {
            let node = self.pathfinding.grid.world_to_grid(position);
            if self.pathfinding.grid.is_walkable(&node) {
                if let Some(idx) = self.spawn_entity(EntityType::House, position) {
                    self.resources
                        .spend_house(self.costs.house_wood, self.costs.house_stone);
                    self.assign_villager_to_build(idx, position);
                }
            }
        }
    }

    fn try_build_lumber_mill(&mut self, position: Vec2) {
        if self
            .resources
            .can_afford_lumber_mill(self.costs.lumber_mill_wood)
        {
            let node = self.pathfinding.grid.world_to_grid(position);
            if self.pathfinding.grid.is_walkable(&node) {
                if let Some(idx) = self.spawn_entity(EntityType::LumberMill, position) {
                    self.resources
                        .spend_lumber_mill(self.costs.lumber_mill_wood);
                    self.assign_villager_to_build(idx, position);
                }
            }
        }
    }

    fn try_build_mine(&mut self, position: Vec2) {
        if self
            .resources
            .can_afford_mine(self.costs.mine_wood, self.costs.mine_stone)
        {
            let node = self.pathfinding.grid.world_to_grid(position);
            if self.pathfinding.grid.is_walkable(&node) {
                if let Some(idx) = self.spawn_entity(EntityType::Mine, position) {
                    self.resources
                        .spend_mine(self.costs.mine_wood, self.costs.mine_stone);
                    self.assign_villager_to_build(idx, position);
                }
            }
        }
    }

    fn assign_villager_to_build(&mut self, building_idx: usize, position: Vec2) {
        if let Some(building) = self.entities.get_mut(building_idx) {
            building.build_progress = 0.0;
        }

        let mut nearest_villager: Option<(usize, f32)> = None;
        for (idx, entity) in self.entities.iter().enumerate() {
            if entity.entity_type == EntityType::Villager
                && matches!(entity.action, crate::entities::UnitAction::Idle)
            {
                let dist = (entity.sprite.transform.position - position).length();
                if let Some((_, min_dist)) = nearest_villager {
                    if dist < min_dist {
                        nearest_villager = Some((idx, dist));
                    }
                } else {
                    nearest_villager = Some((idx, dist));
                }
            }
        }

        if let Some((villager_idx, _)) = nearest_villager {
            if let Some(villager) = self.entities.get_mut(villager_idx) {
                villager.action = crate::entities::UnitAction::Building(building_idx);
                villager.build_target = Some(building_idx);
                let start = villager.sprite.transform.position;
                if let Some(path) = self.pathfinding.find_path(start, position) {
                    villager.path = path;
                    villager.path_index = 0;
                    villager.target_position = Some(position);
                } else {
                    villager.target_position = Some(position);
                    villager.path.clear();
                }
            }
        }
    }

    fn spawn_initial_entities(&mut self) {
        self.spawn_entity(EntityType::TownCenter, Vec2::new(400.0, 400.0));
        self.spawn_entity(EntityType::House, Vec2::new(500.0, 400.0));
        self.spawn_entity(EntityType::House, Vec2::new(550.0, 450.0));

        for i in 0..5 {
            self.spawn_entity(
                EntityType::Villager,
                Vec2::new(400.0 + i as f32 * 30.0, 350.0),
            );
        }

        for i in 0..3 {
            self.spawn_entity(
                EntityType::Military,
                Vec2::new(400.0 + i as f32 * 35.0, 300.0),
            );
        }

        for i in 0..20 {
            let angle = (i as f32 / 20.0) * std::f32::consts::PI * 2.0;
            let radius = 300.0 + (i as f32 % 5.0) * 50.0;
            self.spawn_entity(
                EntityType::Tree,
                Vec2::new(400.0, 400.0) + Vec2::from_angle(angle) * radius,
            );
        }

        for i in 0..5 {
            let angle = (i as f32 / 5.0) * std::f32::consts::PI * 2.0;
            let radius = 500.0;
            self.spawn_entity(
                EntityType::Gold,
                Vec2::new(400.0, 400.0) + Vec2::from_angle(angle) * radius,
            );
        }

        for i in 0..4 {
            let angle = (i as f32 / 4.0) * std::f32::consts::PI * 2.0;
            let radius = 600.0;
            self.spawn_entity(
                EntityType::Stone,
                Vec2::new(400.0, 400.0) + Vec2::from_angle(angle) * radius,
            );
        }
    }

    fn remove_depleted_resources(&mut self) {
        // Collect indices of depleted resources (in reverse order to avoid index shifting issues)
        let mut to_remove: Vec<usize> = Vec::new();
        for (idx, entity) in self.entities.iter().enumerate() {
            if entity.is_resource() && entity.resource_amount == 0 {
                to_remove.push(idx);
            }
        }

        // Remove entities in reverse order to maintain correct indices
        for &idx in to_remove.iter().rev() {
            self.entities.remove(idx);

            // Update any entity references that point to entities after the removed one
            for entity in &mut self.entities {
                // Update action indices first (Gathering, Delivering, Building)
                match entity.action {
                    crate::entities::UnitAction::Gathering(resource_idx) => {
                        if resource_idx > idx {
                            entity.action =
                                crate::entities::UnitAction::Gathering(resource_idx - 1);
                        } else if resource_idx == idx {
                            // This entity was gathering the removed resource
                            entity.action = crate::entities::UnitAction::Idle;
                            entity.resource_target = None;
                        }
                    }
                    crate::entities::UnitAction::Delivering(tc_idx) => {
                        if tc_idx > idx {
                            entity.action = crate::entities::UnitAction::Delivering(tc_idx - 1);
                        }
                    }
                    crate::entities::UnitAction::Building(build_idx) => {
                        if build_idx > idx {
                            entity.action = crate::entities::UnitAction::Building(build_idx - 1);
                        } else if build_idx == idx {
                            entity.action = crate::entities::UnitAction::Idle;
                            entity.build_target = None;
                        }
                    }
                    _ => {}
                }

                // Update resource_target indices
                if let Some(resource_idx) = entity.resource_target {
                    if resource_idx > idx {
                        entity.resource_target = Some(resource_idx - 1);
                    } else if resource_idx == idx {
                        // This entity was targeting the removed resource
                        entity.resource_target = None;
                    }
                }

                // Update build_target indices
                if let Some(build_idx) = entity.build_target {
                    if build_idx > idx {
                        entity.build_target = Some(build_idx - 1);
                    } else if build_idx == idx {
                        entity.build_target = None;
                    }
                }
            }

            // Update selected_entities indices
            for selected_idx in &mut self.selected_entities {
                if *selected_idx > idx {
                    *selected_idx -= 1;
                } else if *selected_idx == idx {
                    // This shouldn't happen as resources aren't selectable, but handle it anyway
                    *selected_idx = usize::MAX; // Mark for removal
                }
            }
            self.selected_entities.retain(|&idx| idx != usize::MAX);
        }
    }
}

impl Game for AgeOfEmpiresDemo {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.create_textures(ctx.renderer())?;
        self.create_ground_tiles();
        self.ui.font = ctx.builtin_font(BuiltinFont::Ui).ok();
        self.spawn_initial_entities();

        let (screen_w, screen_h) = ctx.renderer().surface_size();
        self.camera = Camera2D::new(Vec2::new(
            400.0 - (screen_w as f32 * 0.5),
            400.0 - (screen_h as f32 * 0.5),
        ));
        self.camera.zoom = 1.0;

        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();
        let (screen_w, screen_h) = ctx.renderer().surface_size();
        let input = ctx.input();

        let mut camera_move = Vec2::ZERO;
        if input.is_key_down(KeyCode::ArrowLeft) || input.is_key_down(KeyCode::KeyA) {
            camera_move.x -= 1.0;
        }
        if input.is_key_down(KeyCode::ArrowRight) || input.is_key_down(KeyCode::KeyD) {
            camera_move.x += 1.0;
        }
        if input.is_key_down(KeyCode::ArrowUp) || input.is_key_down(KeyCode::KeyW) {
            camera_move.y -= 1.0;
        }
        if input.is_key_down(KeyCode::ArrowDown) || input.is_key_down(KeyCode::KeyS) {
            camera_move.y += 1.0;
        }

        if camera_move.length_squared() > 0.0 {
            camera_move = camera_move.normalized();
            self.camera.position += camera_move * self.camera_speed * dt;
        }

        let mouse_world = ctx.mouse_world(&self.camera);
        let mouse_screen = input.mouse_position_vec2();

        if input.is_mouse_pressed(MouseButton::Left) {
            // Check if click is in building menu (bottom left)
            let bottom_y = screen_h as f32 - 140.0; // BOTTOM_BAR_HEIGHT
            let building_menu_x = 10.0;
            let building_menu_y = bottom_y + 10.0;
            let building_menu_width = 200.0;
            let building_menu_height = 120.0; // BOTTOM_BAR_HEIGHT - 20.0

            // Building button areas
            let button_x = building_menu_x + 15.0;
            let button_y_start = building_menu_y + 35.0;
            let button_spacing = 50.0;
            let button_width = 120.0;
            let button_height = 35.0;

            let clicked_building_menu = mouse_screen.x >= building_menu_x
                && mouse_screen.x <= building_menu_x + building_menu_width
                && mouse_screen.y >= building_menu_y
                && mouse_screen.y <= building_menu_y + building_menu_height;

            if clicked_building_menu {
                // Check if clicked on House button
                if mouse_screen.x >= button_x
                    && mouse_screen.x <= button_x + button_width
                    && mouse_screen.y >= button_y_start
                    && mouse_screen.y <= button_y_start + button_height
                {
                    if self
                        .resources
                        .can_afford_house(self.costs.house_wood, self.costs.house_stone)
                    {
                        self.building_ghost = Some((EntityType::House, mouse_world));
                    }
                }
                // Check if clicked on Lumber Mill button
                else if mouse_screen.x >= button_x
                    && mouse_screen.x <= button_x + button_width
                    && mouse_screen.y >= button_y_start + button_spacing
                    && mouse_screen.y <= button_y_start + button_spacing + button_height
                {
                    if self
                        .resources
                        .can_afford_lumber_mill(self.costs.lumber_mill_wood)
                    {
                        self.building_ghost = Some((EntityType::LumberMill, mouse_world));
                    }
                }
                // Check if clicked on Mine button
                else if mouse_screen.x >= button_x
                    && mouse_screen.x <= button_x + button_width
                    && mouse_screen.y >= button_y_start + button_spacing * 2.0
                    && mouse_screen.y <= button_y_start + button_spacing * 2.0 + button_height
                {
                    if self
                        .resources
                        .can_afford_mine(self.costs.mine_wood, self.costs.mine_stone)
                    {
                        self.building_ghost = Some((EntityType::Mine, mouse_world));
                    }
                }
                // Don't select entities when clicking UI
            } else if input.is_key_down(KeyCode::ShiftLeft) {
                self.building_ghost = Some((EntityType::House, mouse_world));
            } else {
                self.select_entity_at(mouse_world);
                self.building_ghost = None;
            }
        }

        // Update building ghost position to follow mouse
        if let Some((building_type, _)) = self.building_ghost {
            self.building_ghost = Some((building_type, mouse_world));
        }

        if input.is_mouse_pressed(MouseButton::Right) {
            if let Some((building_type, _)) = self.building_ghost {
                match building_type {
                    EntityType::House => self.try_build_house(mouse_world),
                    EntityType::LumberMill => self.try_build_lumber_mill(mouse_world),
                    EntityType::Mine => self.try_build_mine(mouse_world),
                    _ => {}
                }
                self.building_ghost = None;
            } else {
                // Check if clicking on a drop-off building with selected villagers carrying resources
                if let Some(dropoff_idx) = self.find_dropoff_building_at(mouse_world) {
                    // Check if building accepts resources (before mutable borrows)
                    let dropoff_building = &self.entities[dropoff_idx];
                    let dropoff_pos = dropoff_building.sprite.transform.position;

                    // Collect villagers that can deliver
                    let mut villagers_to_deliver: Vec<usize> = Vec::new();
                    for &villager_idx in &self.selected_entities {
                        if let Some(villager) = self.entities.get(villager_idx) {
                            if villager.entity_type == EntityType::Villager
                                && villager.carried_amount > 0
                                && matches!(villager.action, crate::entities::UnitAction::Idle)
                            {
                                if let Some(carried_resource) = villager.carried_resource {
                                    if dropoff_building.accepts_resource(carried_resource) {
                                        villagers_to_deliver.push(villager_idx);
                                    }
                                }
                            }
                        }
                    }

                    if villagers_to_deliver.is_empty() {
                        // If no villagers could deliver, move units normally
                        self.move_selected_units_to(mouse_world);
                    } else {
                        // Make villagers deliver
                        for villager_idx in villagers_to_deliver {
                            if let Some(villager) = self.entities.get_mut(villager_idx) {
                                villager.action =
                                    crate::entities::UnitAction::Delivering(dropoff_idx);
                                villager.target_position = Some(dropoff_pos);
                                villager.path.clear();
                            }
                        }
                    }
                } else {
                    self.move_selected_units_to(mouse_world);
                }
            }
        }

        // Cancel building mode with Escape
        if input.is_key_pressed(KeyCode::Escape) {
            self.building_ghost = None;
        }

        if input.is_key_pressed(KeyCode::KeyV) {
            self.try_spawn_villager();
        }

        self.update_units(dt);
        self.remove_depleted_resources();

        // Only exit on Escape if not in building mode (building mode cancellation handled above)
        if input.is_key_pressed(KeyCode::Escape) && self.building_ghost.is_none() {
            ctx.request_exit();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        renderer.clear(&mut frame, [0.5, 0.7, 1.0, 1.0])?;

        for tile in &self.ground_tiles {
            renderer.draw_sprite(&mut frame, tile, &self.camera)?;
        }

        if let Some((building_type, pos)) = self.building_ghost {
            let ghost_size = match building_type {
                EntityType::House => Vec2::new(50.0, 50.0),
                EntityType::LumberMill | EntityType::Mine => Vec2::new(60.0, 60.0),
                _ => Vec2::new(50.0, 50.0),
            };
            if let Some(tex) = self.textures.construction {
                let mut ghost = Sprite::new(tex);
                ghost.set_size_px(ghost_size, ghost_size);
                ghost.transform.position = pos;
                ghost.tint = [1.0, 1.0, 1.0, 0.5];
                renderer.draw_sprite(&mut frame, &ghost, &self.camera)?;
            }
        }

        let mut draw_order: Vec<(usize, &GameEntity)> = self.entities.iter().enumerate().collect();
        draw_order.sort_by(|a, b| {
            let order_a = match a.1.entity_type {
                EntityType::Tree | EntityType::Gold | EntityType::Stone => 0,
                EntityType::TownCenter
                | EntityType::House
                | EntityType::LumberMill
                | EntityType::Mine => 1,
                EntityType::Villager | EntityType::Military => 2,
            };
            let order_b = match b.1.entity_type {
                EntityType::Tree | EntityType::Gold | EntityType::Stone => 0,
                EntityType::TownCenter
                | EntityType::House
                | EntityType::LumberMill
                | EntityType::Mine => 1,
                EntityType::Villager | EntityType::Military => 2,
            };
            order_a.cmp(&order_b)
        });

        for (_, entity) in &draw_order {
            let mut sprite = entity.sprite.clone();

            if entity.selected {
                sprite.tint = [1.5, 1.5, 1.0, 1.0];
            }

            if entity.build_progress > 0.0 && entity.build_progress < 1.0 {
                sprite.tint = [1.0, 1.0, 1.0, 0.5 + entity.build_progress * 0.5];
            }

            // Visual feedback for resource depletion
            if entity.is_resource() && entity.resource_amount > 0 {
                let initial_amount = match entity.entity_type {
                    EntityType::Tree => 100,
                    EntityType::Gold => 200,
                    EntityType::Stone => 150,
                    _ => 1,
                };
                let depletion_ratio = entity.resource_amount as f32 / initial_amount as f32;

                // Fade out as resources deplete (alpha goes from 1.0 to 0.3)
                let alpha = 0.3 + depletion_ratio * 0.7;
                sprite.tint[3] = sprite.tint[3].min(alpha);

                // Shrink slightly as resources deplete (scale goes from 1.0 to 0.7)
                let scale_factor = 0.7 + depletion_ratio * 0.3;
                sprite.transform.scale = sprite.transform.scale * scale_factor;
            }

            renderer.draw_sprite(&mut frame, &sprite, &self.camera)?;

            if entity.selected && entity.is_unit() {
                let mut selection = sprite.clone();
                selection.tint = [0.0, 1.0, 0.0, 0.3];
                selection.transform.scale = selection.transform.scale * 1.5;
                renderer.draw_sprite(&mut frame, &selection, &self.camera)?;
            }
        }

        let (screen_w, screen_h) = renderer.surface_size();
        self.ui.draw_hud(
            &self.resources,
            &self.selected_entities,
            &self.entities,
            screen_w,
            screen_h,
        );
        self.ui.hud.draw(renderer, &mut frame)?;

        renderer.end_frame(frame)?;
        Ok(())
    }
}
