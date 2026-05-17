use crate::entities::{EntityType, GameEntity};
use crate::resources::Resources;
use sindri::{FontHandle, HudLayer, HudText, Vec2};

const TOP_BAR_HEIGHT: f32 = 50.0;
const BOTTOM_BAR_HEIGHT: f32 = 140.0;

pub struct UiSystem {
    pub hud: HudLayer,
    pub font: Option<FontHandle>,
}

impl UiSystem {
    pub fn new() -> Self {
        Self {
            hud: HudLayer::new(),
            font: None,
        }
    }

    pub fn draw_top_hud(
        &mut self,
        resources: &Resources,
        population: usize,
        population_cap: usize,
        screen_width: u32,
    ) {
        if let Some(font) = self.font {
            // Top bar background - darker, more opaque with subtle gradient effect
            self.hud.add_panel_with_border(
                Vec2::new(0.0, 0.0),
                Vec2::new(screen_width as f32, TOP_BAR_HEIGHT),
                [0.08, 0.08, 0.12, 1.0], // Dark blue-gray, fully opaque
                [0.6, 0.6, 0.7, 1.0],    // Bright, visible border
                4.0,                     // Thicker, more prominent border
            );

            // Resources section (left side) - horizontal layout
            let start_x = 15.0;
            let center_y = TOP_BAR_HEIGHT * 0.5;
            let resource_spacing = 140.0;

            // Wood
            self.draw_resource_horizontal(
                start_x,
                center_y,
                "Wood",
                resources.wood,
                [0.9, 0.7, 0.3, 1.0], // Brighter brown
                font,
            );

            // Food
            self.draw_resource_horizontal(
                start_x + resource_spacing,
                center_y,
                "Food",
                resources.food,
                [1.0, 0.4, 0.4, 1.0], // Brighter red
                font,
            );

            // Gold
            self.draw_resource_horizontal(
                start_x + resource_spacing * 2.0,
                center_y,
                "Gold",
                resources.gold,
                [1.0, 0.9, 0.2, 1.0], // Brighter gold
                font,
            );

            // Stone
            self.draw_resource_horizontal(
                start_x + resource_spacing * 3.0,
                center_y,
                "Stone",
                resources.stone,
                [0.85, 0.85, 0.85, 1.0], // Brighter gray
                font,
            );

            // Population (right side of top bar) - in a small panel
            let pop_panel_width = 180.0;
            let pop_panel_x = screen_width as f32 - pop_panel_width - 15.0;
            let pop_panel_y = 8.0;

            self.hud.add_panel_with_border(
                Vec2::new(pop_panel_x, pop_panel_y),
                Vec2::new(pop_panel_width, TOP_BAR_HEIGHT - 16.0),
                [0.12, 0.12, 0.18, 1.0],
                [0.5, 0.5, 0.6, 1.0],
                2.0,
            );

            self.hud.add_text(HudText::new(
                format!("Population: {}/{}", population, population_cap),
                font,
                16.0,
                Vec2::new(pop_panel_x + 10.0, pop_panel_y + 10.0),
                [1.0, 1.0, 1.0, 1.0],
            ));
        }
    }

    fn draw_resource_horizontal(
        &mut self,
        x: f32,
        y: f32,
        label: &str,
        value: u32,
        value_color: [f32; 4],
        font: FontHandle,
    ) {
        // Label (smaller, gray)
        self.hud.add_text(HudText::new(
            label.to_string(),
            font,
            12.0,
            Vec2::new(x, y - 12.0),
            [0.7, 0.7, 0.7, 1.0],
        ));

        // Value (larger, colored, bold)
        self.hud.add_text(HudText::new(
            value.to_string(),
            font,
            22.0,
            Vec2::new(x, y + 2.0),
            value_color,
        ));
    }

    pub fn draw_bottom_hud(
        &mut self,
        selected_entities: &[usize],
        entities: &[GameEntity],
        resources: &Resources,
        screen_width: u32,
        screen_height: u32,
    ) {
        if let Some(font) = self.font {
            let bottom_y = screen_height as f32 - BOTTOM_BAR_HEIGHT;

            // Bottom bar background - darker, more opaque
            self.hud.add_panel_with_border(
                Vec2::new(0.0, bottom_y),
                Vec2::new(screen_width as f32, BOTTOM_BAR_HEIGHT),
                [0.08, 0.08, 0.12, 1.0], // Dark blue-gray, fully opaque
                [0.6, 0.6, 0.7, 1.0],    // Bright, visible border
                4.0,                     // Thicker, more prominent border
            );

            // Left section: Building menu (like AoE2)
            let building_menu_width = 200.0;
            let building_menu_x = 10.0;
            let building_menu_y = bottom_y + 10.0;
            let building_menu_height = BOTTOM_BAR_HEIGHT - 20.0;

            self.hud.add_panel_with_border(
                Vec2::new(building_menu_x, building_menu_y),
                Vec2::new(building_menu_width, building_menu_height),
                [0.12, 0.12, 0.18, 1.0],
                [0.5, 0.5, 0.6, 1.0],
                2.0,
            );

            // Building menu title
            self.hud.add_text(HudText::new(
                "Buildings".to_string(),
                font,
                16.0,
                Vec2::new(building_menu_x + 10.0, building_menu_y + 10.0),
                [1.0, 1.0, 0.8, 1.0],
            ));

            // Building buttons (simplified - just text for now)
            let button_y_start = building_menu_y + 35.0;
            let button_x = building_menu_x + 15.0;
            let button_spacing = 50.0;

            // House button
            let can_afford_house = resources.wood >= 50 && resources.stone >= 30;
            let house_color = if can_afford_house {
                [0.9, 0.9, 0.9, 1.0]
            } else {
                [0.5, 0.5, 0.5, 1.0]
            };
            self.hud.add_text(HudText::new(
                "House".to_string(),
                font,
                14.0,
                Vec2::new(button_x, button_y_start),
                house_color,
            ));
            self.hud.add_text(HudText::new(
                "50W 30S".to_string(),
                font,
                11.0,
                Vec2::new(button_x, button_y_start + 16.0),
                [0.7, 0.7, 0.7, 1.0],
            ));

            // Lumber Mill button
            let can_afford_lumber_mill = resources.wood >= 100;
            let lumber_mill_color = if can_afford_lumber_mill {
                [0.9, 0.9, 0.9, 1.0]
            } else {
                [0.5, 0.5, 0.5, 1.0]
            };
            self.hud.add_text(HudText::new(
                "Lumber Mill".to_string(),
                font,
                14.0,
                Vec2::new(button_x, button_y_start + button_spacing),
                lumber_mill_color,
            ));
            self.hud.add_text(HudText::new(
                "100W".to_string(),
                font,
                11.0,
                Vec2::new(button_x, button_y_start + button_spacing + 16.0),
                [0.7, 0.7, 0.7, 1.0],
            ));

            // Mine button
            let can_afford_mine = resources.wood >= 75 && resources.stone >= 50;
            let mine_color = if can_afford_mine {
                [0.9, 0.9, 0.9, 1.0]
            } else {
                [0.5, 0.5, 0.5, 1.0]
            };
            self.hud.add_text(HudText::new(
                "Mine".to_string(),
                font,
                14.0,
                Vec2::new(button_x, button_y_start + button_spacing * 2.0),
                mine_color,
            ));
            self.hud.add_text(HudText::new(
                "75W 50S".to_string(),
                font,
                11.0,
                Vec2::new(button_x, button_y_start + button_spacing * 2.0 + 16.0),
                [0.7, 0.7, 0.7, 1.0],
            ));

            // Center section: Selected unit info panel
            let center_panel_width = 280.0;
            let center_panel_x = building_menu_x + building_menu_width + 15.0;
            let center_panel_y = bottom_y + 10.0;
            let center_panel_height = BOTTOM_BAR_HEIGHT - 20.0;

            self.hud.add_panel_with_border(
                Vec2::new(center_panel_x, center_panel_y),
                Vec2::new(center_panel_width, center_panel_height),
                [0.12, 0.12, 0.18, 1.0],
                [0.5, 0.5, 0.6, 1.0],
                2.0,
            );

            let info_x = center_panel_x + 15.0;
            let mut info_y = center_panel_y + 15.0;
            let line_spacing = 22.0;

            if !selected_entities.is_empty() {
                // Title
                self.hud.add_text(HudText::new(
                    "Selected Unit".to_string(),
                    font,
                    18.0,
                    Vec2::new(info_x, info_y),
                    [1.0, 1.0, 0.8, 1.0], // Light yellow for title
                ));
                info_y += line_spacing;

                // Selected unit count
                self.hud.add_text(HudText::new(
                    format!("Count: {} unit(s)", selected_entities.len()),
                    font,
                    16.0,
                    Vec2::new(info_x, info_y),
                    [1.0, 1.0, 1.0, 1.0],
                ));
                info_y += line_spacing;

                // Unit type info
                if let Some(&first_idx) = selected_entities.first() {
                    if let Some(entity) = entities.get(first_idx) {
                        let unit_type_name = match entity.entity_type {
                            EntityType::Villager => "Villager",
                            EntityType::Military => "Military Unit",
                            _ => "Unknown",
                        };

                        self.hud.add_text(HudText::new(
                            format!("Type: {}", unit_type_name),
                            font,
                            16.0,
                            Vec2::new(info_x, info_y),
                            [0.9, 0.9, 0.9, 1.0],
                        ));
                        info_y += line_spacing;

                        // Action status
                        let (action_text, status_color) = match entity.action {
                            crate::entities::UnitAction::Idle => ("Idle", [0.7, 0.7, 0.7, 1.0]),
                            crate::entities::UnitAction::Moving => ("Moving", [0.4, 0.8, 1.0, 1.0]),
                            crate::entities::UnitAction::Gathering(_) => {
                                ("Gathering", [0.8, 1.0, 0.4, 1.0])
                            }
                            crate::entities::UnitAction::Delivering(_) => {
                                ("Delivering", [1.0, 0.6, 0.2, 1.0])
                            }
                            crate::entities::UnitAction::Building(_) => {
                                ("Building", [1.0, 0.8, 0.4, 1.0])
                            }
                        };

                        self.hud.add_text(HudText::new(
                            format!("Status: {}", action_text),
                            font,
                            16.0,
                            Vec2::new(info_x, info_y),
                            status_color,
                        ));
                        info_y += line_spacing;

                        // Show inventory if carrying resources
                        if entity.carried_amount > 0 {
                            let resource_name = match entity.carried_resource {
                                Some(crate::entities::CarriedResource::Wood) => "Wood",
                                Some(crate::entities::CarriedResource::Gold) => "Gold",
                                Some(crate::entities::CarriedResource::Stone) => "Stone",
                                None => "Unknown",
                            };
                            self.hud.add_text(HudText::new(
                                format!("Carrying: {} {}", entity.carried_amount, resource_name),
                                font,
                                14.0,
                                Vec2::new(info_x, info_y),
                                [0.9, 0.9, 0.5, 1.0],
                            ));
                        }
                    }
                }
            } else {
                // No selection message
                self.hud.add_text(HudText::new(
                    "No Selection".to_string(),
                    font,
                    18.0,
                    Vec2::new(info_x, info_y),
                    [0.5, 0.5, 0.5, 1.0],
                ));
                info_y += line_spacing;
                self.hud.add_text(HudText::new(
                    "Click to select units".to_string(),
                    font,
                    14.0,
                    Vec2::new(info_x, info_y),
                    [0.6, 0.6, 0.6, 1.0],
                ));
            }

            // Right section: Minimap
            let minimap_size = 110.0;
            let minimap_x = screen_width as f32 - minimap_size - 15.0;
            let minimap_y = bottom_y + 15.0;

            // Minimap background with border
            self.hud.add_panel_with_border(
                Vec2::new(minimap_x, minimap_y),
                Vec2::new(minimap_size, minimap_size),
                [0.10, 0.10, 0.14, 1.0],
                [0.6, 0.6, 0.7, 1.0],
                3.0,
            );

            // Minimap title
            self.hud.add_text(HudText::new(
                "Minimap".to_string(),
                font,
                14.0,
                Vec2::new(minimap_x + 8.0, minimap_y + 8.0),
                [0.7, 0.7, 0.7, 1.0],
            ));

            // Placeholder text in minimap
            self.hud.add_text(HudText::new(
                "Map View".to_string(),
                font,
                12.0,
                Vec2::new(
                    minimap_x + minimap_size * 0.5 - 35.0,
                    minimap_y + minimap_size * 0.5,
                ),
                [0.4, 0.4, 0.4, 1.0],
            ));
        }
    }

    pub fn draw_hud(
        &mut self,
        resources: &Resources,
        selected_entities: &[usize],
        entities: &[GameEntity],
        screen_width: u32,
        screen_height: u32,
    ) {
        self.hud.clear();

        // Count population (units only)
        let population = entities
            .iter()
            .filter(|e| matches!(e.entity_type, EntityType::Villager | EntityType::Military))
            .count();
        let population_cap = 200; // Max population

        self.draw_top_hud(resources, population, population_cap, screen_width);
        self.draw_bottom_hud(
            selected_entities,
            entities,
            resources,
            screen_width,
            screen_height,
        );
    }
}
