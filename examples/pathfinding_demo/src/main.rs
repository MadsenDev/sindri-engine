use anyhow::Result;
use sindri::{
    camera::CameraFollow,
    hud::{HudLayer, HudText, TextAlign},
    math::{Camera2D, Vec2},
    pathfinding::{AStarPathfinder, GridNode, PathfindingGrid},
    render::{Renderer, Sprite, TextureHandle},
    Engine, Game, KeyCode,
};
use std::collections::HashSet;

struct PathfindingDemo {
    camera: Camera2D,
    world: sindri::World,

    textures: TextureHandles,
    grid: PathfindingGrid,

    // Pathfinding state
    start_pos: Option<Vec2>,
    goal_pos: Option<Vec2>,
    current_path: Vec<Vec2>,
    path_nodes: Vec<GridNode>,

    // Obstacles (for visualization)
    obstacles: HashSet<GridNode>,

    // Agent position
    agent_pos: Vec2,
    agent_target: Option<Vec2>,
    agent_path: Vec<Vec2>,
    agent_path_index: usize,

    camera_follow: CameraFollow,
    initialized: bool,
    hud: HudLayer,
    font: Option<sindri::FontHandle>,
}

struct TextureHandles {
    grid_cell: Option<TextureHandle>,
    obstacle: Option<TextureHandle>,
    start: Option<TextureHandle>,
    goal: Option<TextureHandle>,
    path: Option<TextureHandle>,
    agent: Option<TextureHandle>,
}

impl PathfindingDemo {
    fn new() -> Self {
        // Create a 40x30 grid with 32px cells
        let grid = PathfindingGrid::new(40, 30, 32.0);

        Self {
            camera: Camera2D::new(Vec2::new(640.0, 480.0)),
            world: sindri::World::new(),
            textures: TextureHandles {
                grid_cell: None,
                obstacle: None,
                start: None,
                goal: None,
                path: None,
                agent: None,
            },
            grid,
            start_pos: None,
            goal_pos: None,
            current_path: Vec::new(),
            path_nodes: Vec::new(),
            obstacles: HashSet::new(),
            agent_pos: Vec2::new(200.0, 200.0),
            agent_target: None,
            agent_path: Vec::new(),
            agent_path_index: 0,
            camera_follow: CameraFollow::new()
                .follow_position(Vec2::new(640.0, 480.0))
                .with_dead_zone(200.0, 150.0)
                .with_smoothing(0.15),
            initialized: false,
            hud: HudLayer::new(),
            font: None,
        }
    }

    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        // Grid cell (light gray, semi-transparent)
        let cell_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [200u8, 200, 200, 100])
            .collect();
        self.textures.grid_cell = Some(renderer.load_texture_from_rgba(&cell_data, 32, 32)?);

        // Obstacle (dark red)
        let obstacle_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [150u8, 50, 50, 255])
            .collect();
        self.textures.obstacle = Some(renderer.load_texture_from_rgba(&obstacle_data, 32, 32)?);

        // Start marker (green)
        let start_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [50u8, 200, 50, 255])
            .collect();
        self.textures.start = Some(renderer.load_texture_from_rgba(&start_data, 32, 32)?);

        // Goal marker (blue)
        let goal_data: Vec<u8> = (0..(4 * 32 * 32))
            .flat_map(|_| [50u8, 50, 200, 255])
            .collect();
        self.textures.goal = Some(renderer.load_texture_from_rgba(&goal_data, 32, 32)?);

        // Path node (yellow)
        let path_data: Vec<u8> = (0..(4 * 24 * 24))
            .flat_map(|_| [255u8, 255, 100, 200])
            .collect();
        self.textures.path = Some(renderer.load_texture_from_rgba(&path_data, 24, 24)?);

        // Agent (cyan circle-like)
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
                    agent_data[idx] = 100;
                    agent_data[idx + 1] = 255;
                    agent_data[idx + 2] = 255;
                    agent_data[idx + 3] = 255;
                }
            }
        }
        self.textures.agent = Some(renderer.load_texture_from_rgba(
            &agent_data,
            agent_size as u32,
            agent_size as u32,
        )?);

        Ok(())
    }

    fn setup_obstacles(&mut self) {
        // Create some obstacles
        // Walls
        self.grid.set_area_walkable(10, 5, 8, 1, false);
        self.grid.set_area_walkable(15, 10, 1, 6, false);
        self.grid.set_area_walkable(20, 8, 5, 1, false);
        self.grid.set_area_walkable(25, 15, 1, 8, false);
        self.grid.set_area_walkable(5, 20, 6, 1, false);
        self.grid.set_area_walkable(30, 5, 1, 10, false);

        // Store obstacle nodes for rendering
        for y in 0..self.grid.height() as i32 {
            for x in 0..self.grid.width() as i32 {
                let node = GridNode::new(x, y);
                if !self.grid.is_walkable(&node) {
                    self.obstacles.insert(node);
                }
            }
        }
    }
}

impl Game for PathfindingDemo {
    fn init(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        self.create_textures(&mut *ctx.renderer())?;
        self.setup_obstacles();

        // Load font for instructions
        self.font = Some(ctx.builtin_font(sindri::BuiltinFont::Ui)?);

        // Set initial agent position
        self.agent_pos = Vec2::new(100.0, 100.0);
        // Camera should start at agent position (camera.position is the center of the view)
        self.camera.position = self.agent_pos;

        self.initialized = true;
        Ok(())
    }

    fn update(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        let input = ctx.input();
        let dt = ctx.delta_time().as_secs_f32();

        // Convert mouse position to world coordinates
        let mouse_world = ctx.mouse_world(&self.camera);

        // Left click: command agent to move here (primary interaction)
        if input.is_mouse_pressed(sindri::MouseButton::Left) {
            let grid_pos = self.grid.world_to_grid(mouse_world);
            if self.grid.is_walkable(&grid_pos) {
                // Command agent to move to clicked position
                if let Some(path) =
                    AStarPathfinder::find_path(&self.grid, self.agent_pos, mouse_world)
                {
                    self.agent_target = Some(mouse_world);
                    self.agent_path = path.clone();
                    self.agent_path_index = 0;

                    // Also update visualization path
                    self.start_pos = Some(self.agent_pos);
                    self.goal_pos = Some(mouse_world);
                    self.current_path = path.clone();
                    self.path_nodes = path.iter().map(|p| self.grid.world_to_grid(*p)).collect();
                }
            }
        }

        // Right click: set goal position for visualization only
        if input.is_mouse_pressed(sindri::MouseButton::Right) {
            let grid_pos = self.grid.world_to_grid(mouse_world);
            if self.grid.is_walkable(&grid_pos) {
                self.goal_pos = Some(mouse_world);
                self.current_path.clear();
                self.path_nodes.clear();

                // If we have both start and goal, find path for visualization
                if let Some(start) = self.start_pos {
                    if let Some(path) = AStarPathfinder::find_path(&self.grid, start, mouse_world) {
                        self.current_path = path.clone();
                        self.path_nodes =
                            path.iter().map(|p| self.grid.world_to_grid(*p)).collect();
                    }
                }
            }
        }

        // Space: command agent to move to current goal (if set)
        if input.is_key_pressed(KeyCode::Space) {
            if let Some(goal) = self.goal_pos {
                if let Some(path) = AStarPathfinder::find_path(&self.grid, self.agent_pos, goal) {
                    self.agent_target = Some(goal);
                    self.agent_path = path;
                    self.agent_path_index = 0;
                }
            }
        }

        // Move agent along path
        if self.agent_target.is_some() {
            if !self.agent_path.is_empty() && self.agent_path_index < self.agent_path.len() {
                let next_pos = self.agent_path[self.agent_path_index];
                let direction = (next_pos - self.agent_pos).normalized();
                let speed = 150.0;
                let move_distance = speed * dt;

                let distance_to_next = self.agent_pos.distance(next_pos);
                if distance_to_next < move_distance {
                    self.agent_pos = next_pos;
                    self.agent_path_index += 1;

                    if self.agent_path_index >= self.agent_path.len() {
                        self.agent_target = None;
                        self.agent_path.clear();
                        self.agent_path_index = 0;
                    }
                } else {
                    self.agent_pos = self.agent_pos + direction * move_distance;
                }
            }
        }

        // Update camera to follow agent
        self.camera_follow = CameraFollow::new()
            .follow_position(self.agent_pos)
            .with_dead_zone(200.0, 150.0)
            .with_smoothing(0.15);

        // Create a dummy physics world for camera follow (we don't actually use physics here)
        // Actually, we can't use update_camera_follow without physics, so let's just update directly
        let offset = self.agent_pos - self.camera.position;
        let half_dead_zone = Vec2::new(200.0 / 2.0, 150.0 / 2.0);

        if offset.x.abs() > half_dead_zone.x || offset.y.abs() > half_dead_zone.y {
            let mut desired_pos = self.camera.position;
            if offset.x.abs() > half_dead_zone.x {
                desired_pos.x = self.agent_pos.x - offset.x.signum() * half_dead_zone.x;
            }
            if offset.y.abs() > half_dead_zone.y {
                desired_pos.y = self.agent_pos.y - offset.y.signum() * half_dead_zone.y;
            }
            self.camera.position = self.camera.position.lerp(desired_pos, 0.15);
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut sindri::EngineContext) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        // Clear with dark background
        renderer.clear(&mut frame, [0.1, 0.1, 0.15, 1.0])?;

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

        // Skip drawing grid cells to save sprites - just draw obstacles, path, and markers
        // Draw obstacles (only visible ones)
        if let Some(obstacle_tex) = self.textures.obstacle {
            for node in &self.obstacles {
                // Skip if outside view bounds
                if node.x < min_x || node.x > max_x || node.y < min_y || node.y > max_y {
                    continue;
                }

                let world_pos = self.grid.grid_to_world(*node);
                let cell_size = self.grid.cell_size();
                if world_pos.x + cell_size / 2.0 < view_min.x
                    || world_pos.x - cell_size / 2.0 > view_max.x
                    || world_pos.y + cell_size / 2.0 < view_min.y
                    || world_pos.y - cell_size / 2.0 > view_max.y
                {
                    continue;
                }

                let mut sprite = Sprite::new(obstacle_tex);
                sprite.transform.position = world_pos;
                sprite.set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
                if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                    eprintln!("Error drawing obstacle: {}", e);
                }
            }
        }

        // Draw path nodes (only visible ones)
        if let Some(path_tex) = self.textures.path {
            for node in &self.path_nodes {
                // Skip if outside view bounds
                if node.x < min_x || node.x > max_x || node.y < min_y || node.y > max_y {
                    continue;
                }

                let world_pos = self.grid.grid_to_world(*node);
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
                    eprintln!("Error drawing path node: {}", e);
                }
            }
        }

        // Draw start marker
        if let Some(start_tex) = self.textures.start {
            if let Some(start) = self.start_pos {
                let mut sprite = Sprite::new(start_tex);
                sprite.transform.position = start;
                sprite.set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
                if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                    eprintln!("Error drawing start: {}", e);
                }
            }
        }

        // Draw goal marker
        if let Some(goal_tex) = self.textures.goal {
            if let Some(goal) = self.goal_pos {
                let mut sprite = Sprite::new(goal_tex);
                sprite.transform.position = goal;
                sprite.set_size_px(Vec2::new(32.0, 32.0), Vec2::new(32.0, 32.0));
                if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                    eprintln!("Error drawing goal: {}", e);
                }
            }
        }

        // Draw agent
        if let Some(agent_tex) = self.textures.agent {
            let mut sprite = Sprite::new(agent_tex);
            sprite.transform.position = self.agent_pos;
            sprite.set_size_px(Vec2::new(28.0, 28.0), Vec2::new(28.0, 28.0));
            if let Err(e) = renderer.draw_sprite(&mut frame, &sprite, &self.camera) {
                eprintln!("Error drawing agent: {}", e);
            }
        }

        // Draw HUD instructions
        self.hud.clear();
        if let Some(font) = self.font {
            self.hud.add_text(HudText {
                text: "Left Click: Command agent to move here".to_string(),
                font,
                size: 20.0,
                position: Vec2::new(10.0, 10.0),
                color: [1.0, 1.0, 1.0, 1.0],
                align: TextAlign::Left,
            });
            self.hud.add_text(HudText {
                text: "Right Click: Set goal for path visualization".to_string(),
                font,
                size: 20.0,
                position: Vec2::new(10.0, 35.0),
                color: [1.0, 1.0, 1.0, 1.0],
                align: TextAlign::Left,
            });
            self.hud.add_text(HudText {
                text: "Space: Command agent to move to goal".to_string(),
                font,
                size: 20.0,
                position: Vec2::new(10.0, 60.0),
                color: [1.0, 1.0, 1.0, 1.0],
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
        .with_title("Sindri A* Pathfinding Demo")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(PathfindingDemo::new())
}
