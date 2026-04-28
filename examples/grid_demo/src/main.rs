use anyhow::Result;
use forge2d::{
    grid::{Grid, GridCoord},
    hud::{HudLayer, HudText, TextAlign},
    math::{Camera2D, Vec2},
    pathfinding::{AStarPathfinder, GridNode, PathfindingGrid},
    render::{Renderer, Sprite, TextureHandle},
    Engine, Game,
};
use std::collections::HashSet;

struct GridDemo {
    camera: Camera2D,
    
    textures: TextureHandles,
    
    // Grid system
    grid: Grid<bool>, // true = walkable, false = blocked
    obstacles: HashSet<GridCoord>,
    
    // Agent (player character)
    agent_grid_pos: GridCoord,
    agent_world_pos: Vec2,
    agent_target_grid: Option<GridCoord>,
    agent_path: Vec<GridCoord>,
    agent_path_index: usize,
    move_timer: f32,
    move_duration: f32, // Time to move from one cell to another
    
    // Pathfinding visualization
    pathfinding_grid: PathfindingGrid,
    path_nodes: Vec<GridNode>,
    
    // UI
    hud: HudLayer,
    font: Option<forge2d::FontHandle>,
    initialized: bool,
}

struct TextureHandles {
    floor: Option<TextureHandle>,
    wall: Option<TextureHandle>,
    agent: Option<TextureHandle>,
    path: Option<TextureHandle>,
    target: Option<TextureHandle>,
}

impl GridDemo {
    fn new() -> Self {
        // Create a 30x20 grid with 32px cells
        let grid = Grid::new(30, 20, 32.0, true);
        
        // Create corresponding pathfinding grid for A*
        let pathfinding_grid = PathfindingGrid::new(30, 20, 32.0);
        
        Self {
            camera: Camera2D::new(Vec2::new(480.0, 320.0)),
            textures: TextureHandles {
                floor: None,
                wall: None,
                agent: None,
                path: None,
                target: None,
            },
            grid,
            obstacles: HashSet::new(),
            agent_grid_pos: GridCoord::new(5, 5),
            agent_world_pos: Vec2::ZERO, // Will be set in init
            agent_target_grid: None,
            agent_path: Vec::new(),
            agent_path_index: 0,
            move_timer: 0.0,
            move_duration: 0.3, // 0.3 seconds per cell movement
            pathfinding_grid,
            path_nodes: Vec::new(),
            hud: HudLayer::new(),
            font: None,
            initialized: false,
        }
    }
    
    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        // Floor tile (light gray)
        let floor_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [220u8, 220, 220, 255])
            .collect();
        self.textures.floor = Some(renderer.load_texture_from_rgba(&floor_data, 32, 32)?);
        
        // Wall/obstacle (dark gray)
        let wall_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [80u8, 80, 80, 255])
            .collect();
        self.textures.wall = Some(renderer.load_texture_from_rgba(&wall_data, 32, 32)?);
        
        // Agent (cyan circle)
        let agent_size = 28;
        let mut agent_data = vec![0u8; 4 * agent_size * agent_size];
        let center = agent_size as f32 / 2.0;
        for y in 0..agent_size {
            for x in 0..agent_size {
                let dx = x as f32 - center;
                let dy = y as f32 - center;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < center - 2.0 {
                    let idx = (y * agent_size + x) * 4;
                    agent_data[idx] = 50;
                    agent_data[idx + 1] = 200;
                    agent_data[idx + 2] = 255;
                    agent_data[idx + 3] = 255;
                }
            }
        }
        self.textures.agent = Some(renderer.load_texture_from_rgba(&agent_data, agent_size as u32, agent_size as u32)?);
        
        // Path node (yellow)
        let path_data: Vec<u8> = (0..(4 * 24 * 24))
            .flat_map(|_| [255u8, 255, 100, 200])
            .collect();
        self.textures.path = Some(renderer.load_texture_from_rgba(&path_data, 24, 24)?);
        
        // Target marker (green)
        let target_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [50u8, 200, 50, 255])
            .collect();
        self.textures.target = Some(renderer.load_texture_from_rgba(&target_data, 32, 32)?);
        
        Ok(())
    }
    
    fn setup_obstacles(&mut self) {
        // Create some obstacles
        let obstacle_coords = vec![
            // Horizontal walls
            (10, 5, 8, 1),
            (15, 10, 1, 6),
            (20, 8, 5, 1),
            (25, 15, 1, 8),
            (5, 20, 6, 1),
            (30, 5, 1, 10),
        ];
        
        for (x, y, w, h) in obstacle_coords {
            for dy in 0..h {
                for dx in 0..w {
                    let coord = GridCoord::new(x + dx, y + dy);
                    if self.grid.is_valid(&coord) {
                        self.grid.set(coord, false); // Blocked
                        self.obstacles.insert(coord);
                        // Also update pathfinding grid
                        self.pathfinding_grid.set_walkable(GridNode::new(coord.x, coord.y), false);
                    }
                }
            }
        }
    }
    
    fn sync_pathfinding_grid(&mut self) {
        // Sync obstacles from grid to pathfinding grid
        for coord in &self.obstacles {
            self.pathfinding_grid.set_walkable(GridNode::new(coord.x, coord.y), false);
        }
    }
    
    fn find_path_grid(&self, start: GridCoord, goal: GridCoord) -> Option<Vec<GridCoord>> {
        // Convert GridCoord to GridNode for pathfinding
        let start_node = GridNode::new(start.x, start.y);
        let goal_node = GridNode::new(goal.x, goal.y);
        
        // Use A* pathfinder
        if let Some(path_nodes) = AStarPathfinder::find_path_grid(&self.pathfinding_grid, start_node, goal_node) {
            // Convert back to GridCoord
            Some(path_nodes.iter().map(|n| GridCoord::new(n.x, n.y)).collect())
        } else {
            None
        }
    }
}

impl Game for GridDemo {
    fn init(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        self.create_textures(&mut *ctx.renderer())?;
        self.setup_obstacles();
        self.sync_pathfinding_grid();
        
        // Load font
        self.font = Some(ctx.builtin_font(forge2d::BuiltinFont::Ui)?);
        
        // Set initial agent world position
        self.agent_world_pos = self.grid.grid_to_world(self.agent_grid_pos);
        self.camera.position = self.agent_world_pos;
        
        self.initialized = true;
        Ok(())
    }
    
    fn update(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        let input = ctx.input();
        let dt = ctx.delta_time().as_secs_f32();
        
        // Convert mouse position to world coordinates
        let mouse_world = ctx.mouse_world(&self.camera);
        let mouse_grid = self.grid.world_to_grid(mouse_world);
        
        // Left click: command agent to move to clicked cell
        if input.is_mouse_pressed(forge2d::MouseButton::Left) {
            if self.grid.is_valid(&mouse_grid) && self.grid.get(mouse_grid).copied().unwrap_or(false) {
                // Find path from current position to target
                if let Some(path) = self.find_path_grid(self.agent_grid_pos, mouse_grid) {
                    if path.len() > 1 {
                        // Skip first node (current position)
                        self.agent_path = path[1..].to_vec();
                        self.agent_path_index = 0;
                        self.agent_target_grid = Some(mouse_grid);
                        self.move_timer = 0.0;
                        
                        // Update path visualization
                        self.path_nodes = self.agent_path.iter()
                            .map(|c| GridNode::new(c.x, c.y))
                            .collect();
                    }
                }
            }
        }
        
        // Move agent along path (grid-based movement)
        if self.agent_target_grid.is_some() {
            if self.agent_path_index < self.agent_path.len() {
                let next_cell = self.agent_path[self.agent_path_index];
                let next_world = self.grid.grid_to_world(next_cell);
                
                // Interpolate movement between cells
                self.move_timer += dt;
                let t = (self.move_timer / self.move_duration).min(1.0);
                
                // Smooth interpolation
                self.agent_world_pos = self.agent_world_pos.lerp(next_world, t * t * (3.0 - 2.0 * t));
                
                // Check if we've reached the next cell
                if t >= 1.0 {
                    self.agent_grid_pos = next_cell;
                    self.agent_world_pos = next_world; // Snap to exact position
                    self.agent_path_index += 1;
                    self.move_timer = 0.0;
                    
                    // Check if we've reached the target
                    if self.agent_path_index >= self.agent_path.len() {
                        self.agent_target_grid = None;
                        self.agent_path.clear();
                        self.agent_path_index = 0;
                        self.path_nodes.clear();
                    }
                }
            }
        }
        
        // Update camera to follow agent
        let offset = self.agent_world_pos - self.camera.position;
        let half_dead_zone = Vec2::new(200.0 / 2.0, 150.0 / 2.0);
        
        if offset.x.abs() > half_dead_zone.x || offset.y.abs() > half_dead_zone.y {
            let mut desired_pos = self.camera.position;
            if offset.x.abs() > half_dead_zone.x {
                desired_pos.x = self.agent_world_pos.x - offset.x.signum() * half_dead_zone.x;
            }
            if offset.y.abs() > half_dead_zone.y {
                desired_pos.y = self.agent_world_pos.y - offset.y.signum() * half_dead_zone.y;
            }
            self.camera.position = self.camera.position.lerp(desired_pos, 0.15);
        }
        
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut forge2d::EngineContext) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }
        
        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;
        
        // Clear with light gray background (represents floor)
        renderer.clear(&mut frame, [0.85, 0.85, 0.85, 1.0])?;
        
        // Calculate camera view bounds for culling
        let screen_width = 1280.0;
        let screen_height = 720.0;
        let half_width = screen_width / 2.0 / self.camera.zoom;
        let half_height = screen_height / 2.0 / self.camera.zoom;
        let view_min = Vec2::new(
            self.camera.position.x - half_width,
            self.camera.position.y - half_height,
        );
        let view_max = Vec2::new(
            self.camera.position.x + half_width,
            self.camera.position.y + half_height,
        );
        
        // Convert view bounds to grid coordinates
        let grid_min = self.grid.world_to_grid(view_min);
        let grid_max = self.grid.world_to_grid(view_max);
        
        // Clamp to grid bounds
        let min_x = (grid_min.x - 1).max(0);
        let max_x = (grid_max.x + 1).min(self.grid.width() as i32 - 1);
        let min_y = (grid_min.y - 1).max(0);
        let max_y = (grid_max.y + 1).min(self.grid.height() as i32 - 1);
        
        // Skip drawing floor tiles to save sprites - just draw obstacles, path, and markers
        // The background color provides the floor appearance
        
        // Draw obstacles
        if let Some(wall_tex) = self.textures.wall {
            for coord in &self.obstacles {
                if coord.x < min_x || coord.x > max_x || coord.y < min_y || coord.y > max_y {
                    continue;
                }
                
                let world_pos = self.grid.grid_to_world(*coord);
                let cell_size = self.grid.cell_size();
                if world_pos.x + cell_size / 2.0 < view_min.x
                    || world_pos.x - cell_size / 2.0 > view_max.x
                    || world_pos.y + cell_size / 2.0 < view_min.y
                    || world_pos.y - cell_size / 2.0 > view_max.y
                {
                    continue;
                }
                
                let mut sprite = Sprite::new(wall_tex);
                sprite.transform.position = world_pos;
                sprite.set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
                if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                    eprintln!("Error drawing wall: {}", e);
                }
            }
        }
        
        // Draw path nodes
        if let Some(path_tex) = self.textures.path {
            for coord in &self.agent_path {
                if coord.x < min_x || coord.x > max_x || coord.y < min_y || coord.y > max_y {
                    continue;
                }
                
                let world_pos = self.grid.grid_to_world(*coord);
                let cell_size = self.grid.cell_size();
                if world_pos.x + cell_size / 2.0 < view_min.x
                    || world_pos.x - cell_size / 2.0 > view_max.x
                    || world_pos.y + cell_size / 2.0 < view_min.y
                    || world_pos.y - cell_size / 2.0 > view_max.y
                {
                    continue;
                }
                
                let mut sprite = Sprite::new(path_tex);
                sprite.transform.position = world_pos;
                sprite.set_size_px(Vec2::new(24.0, 24.0), Vec2::new(24.0, 24.0));
                if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                    eprintln!("Error drawing path: {}", e);
                }
            }
        }
        
        // Draw target marker
        if let Some(target_tex) = self.textures.target {
            if let Some(target) = self.agent_target_grid {
                let world_pos = self.grid.grid_to_world(target);
                let mut sprite = Sprite::new(target_tex);
                sprite.transform.position = world_pos;
                sprite.set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
                if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                    eprintln!("Error drawing target: {}", e);
                }
            }
        }
        
        // Draw agent
        if let Some(agent_tex) = self.textures.agent {
            let mut sprite = Sprite::new(agent_tex);
            sprite.transform.position = self.agent_world_pos;
            sprite.set_size_px(Vec2::new(28.0, 28.0), Vec2::new(28.0, 28.0));
            if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                eprintln!("Error drawing agent: {}", e);
            }
        }
        
        // Draw HUD instructions
        self.hud.clear();
        if let Some(font) = self.font {
            self.hud.add_text(HudText {
                text: "Grid-Based Movement Demo".to_string(),
                font,
                size: 24.0,
                position: Vec2::new(10.0, 10.0),
                color: [1.0, 1.0, 1.0, 1.0],
                align: TextAlign::Left,
            });
            self.hud.add_text(HudText {
                text: "Left Click: Command agent to move (grid-snapped)".to_string(),
                font,
                size: 18.0,
                position: Vec2::new(10.0, 40.0),
                color: [0.9, 0.9, 0.9, 1.0],
                align: TextAlign::Left,
            });
            self.hud.add_text(HudText {
                text: format!("Agent Grid: ({}, {})", self.agent_grid_pos.x, self.agent_grid_pos.y),
                font,
                size: 16.0,
                position: Vec2::new(10.0, 65.0),
                color: [0.8, 0.8, 0.8, 1.0],
                align: TextAlign::Left,
            });
        }
        self.hud.draw(renderer, &mut frame)?;
        
        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Forge2D Grid-Based Movement Demo")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(GridDemo::new())
}
