use sindri::Sprite;

// Entity types
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntityType {
    Villager,
    Military,
    TownCenter,
    House,
    LumberMill,
    Mine,
    Tree,
    Gold,
    Stone,
}

// Resource types that can be carried
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CarriedResource {
    Wood,
    Gold,
    Stone,
}

// Unit actions
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnitAction {
    Idle,
    Moving,
    Gathering(usize),  // Entity index of resource being gathered
    Delivering(usize), // Entity index of town center to deliver to
    Building(usize),   // Entity index of building being constructed
}

pub struct GameEntity {
    pub sprite: Sprite,
    pub entity_type: EntityType,
    pub selected: bool,
    pub target_position: Option<sindri::Vec2>,
    pub path: Vec<sindri::Vec2>,
    pub path_index: usize,
    pub speed: f32,
    pub action: UnitAction,

    // Resource gathering
    pub resource_target: Option<usize>, // Entity index
    pub gather_timer: f32,
    pub gather_rate: f32, // Resources per second

    // Inventory (for villagers)
    pub carried_resource: Option<CarriedResource>, // What resource is being carried
    pub carried_amount: u32,                       // How much is being carried
    pub max_carry_capacity: u32,                   // Maximum carrying capacity

    // Building construction
    pub build_progress: f32,         // 0.0 to 1.0
    pub build_target: Option<usize>, // Entity index

    // Resource amounts (for trees, gold, stone)
    pub resource_amount: u32,
}

impl GameEntity {
    pub fn new(sprite: Sprite, entity_type: EntityType, speed: f32, resource_amount: u32) -> Self {
        let max_carry = if entity_type == EntityType::Villager {
            100 // Villagers can carry 100 resources
        } else {
            0
        };

        Self {
            sprite,
            entity_type,
            selected: false,
            target_position: None,
            path: Vec::new(),
            path_index: 0,
            speed,
            action: UnitAction::Idle,
            resource_target: None,
            gather_timer: 0.0,
            gather_rate: 5.0, // 5 resources per second
            carried_resource: None,
            carried_amount: 0,
            max_carry_capacity: max_carry,
            build_progress: 0.0,
            build_target: None,
            resource_amount,
        }
    }

    pub fn get_size(&self) -> sindri::Vec2 {
        match self.entity_type {
            EntityType::Villager => sindri::Vec2::new(24.0, 24.0),
            EntityType::Military => sindri::Vec2::new(28.0, 28.0),
            EntityType::TownCenter => sindri::Vec2::new(80.0, 80.0),
            EntityType::House => sindri::Vec2::new(50.0, 50.0),
            EntityType::LumberMill => sindri::Vec2::new(60.0, 60.0),
            EntityType::Mine => sindri::Vec2::new(60.0, 60.0),
            EntityType::Tree => sindri::Vec2::new(40.0, 60.0),
            EntityType::Gold | EntityType::Stone => sindri::Vec2::new(30.0, 30.0),
        }
    }

    pub fn get_selection_radius(&self) -> f32 {
        match self.entity_type {
            EntityType::Villager | EntityType::Military => 15.0,
            EntityType::TownCenter => 40.0,
            EntityType::House | EntityType::LumberMill | EntityType::Mine => 30.0,
            EntityType::Tree => 20.0,
            EntityType::Gold | EntityType::Stone => 15.0,
        }
    }

    pub fn is_unit(&self) -> bool {
        matches!(
            self.entity_type,
            EntityType::Villager | EntityType::Military
        )
    }

    pub fn is_resource(&self) -> bool {
        matches!(
            self.entity_type,
            EntityType::Tree | EntityType::Gold | EntityType::Stone
        )
    }

    pub fn is_building(&self) -> bool {
        matches!(
            self.entity_type,
            EntityType::TownCenter | EntityType::House | EntityType::LumberMill | EntityType::Mine
        )
    }

    pub fn accepts_resource(&self, resource: CarriedResource) -> bool {
        match (self.entity_type, resource) {
            (EntityType::TownCenter, _) => true, // Town center accepts all resources
            (EntityType::LumberMill, CarriedResource::Wood) => true,
            (EntityType::Mine, CarriedResource::Gold) => true,
            (EntityType::Mine, CarriedResource::Stone) => true,
            _ => false,
        }
    }
}
