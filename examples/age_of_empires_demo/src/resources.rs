pub struct Resources {
    pub wood: u32,
    pub gold: u32,
    pub stone: u32,
    pub food: u32,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            wood: 200,
            gold: 200,
            stone: 200,
            food: 200,
        }
    }

    pub fn can_afford_house(&self, cost_wood: u32, cost_stone: u32) -> bool {
        self.wood >= cost_wood && self.stone >= cost_stone
    }

    pub fn can_afford_lumber_mill(&self, cost_wood: u32) -> bool {
        self.wood >= cost_wood
    }

    pub fn can_afford_mine(&self, cost_wood: u32, cost_stone: u32) -> bool {
        self.wood >= cost_wood && self.stone >= cost_stone
    }

    pub fn can_afford_villager(&self, cost_food: u32) -> bool {
        self.food >= cost_food
    }

    pub fn spend_house(&mut self, cost_wood: u32, cost_stone: u32) {
        self.wood -= cost_wood;
        self.stone -= cost_stone;
    }

    pub fn spend_lumber_mill(&mut self, cost_wood: u32) {
        self.wood -= cost_wood;
    }

    pub fn spend_mine(&mut self, cost_wood: u32, cost_stone: u32) {
        self.wood -= cost_wood;
        self.stone -= cost_stone;
    }

    pub fn spend_villager(&mut self, cost_food: u32) {
        self.food -= cost_food;
    }
}

pub struct BuildingCosts {
    pub house_wood: u32,
    pub house_stone: u32,
    pub lumber_mill_wood: u32,
    pub mine_wood: u32,
    pub mine_stone: u32,
    pub villager_food: u32,
}

impl BuildingCosts {
    pub fn new() -> Self {
        Self {
            house_wood: 50,
            house_stone: 30,
            lumber_mill_wood: 100,
            mine_wood: 75,
            mine_stone: 50,
            villager_food: 50,
        }
    }
}
