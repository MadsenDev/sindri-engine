use anyhow::Result;
use sindri::{
    entities::{SpriteComponent, TilemapComponent, Transform},
    grid::{Grid, GridCoord},
    hud::{HudLayer, HudText},
    math::{Camera2D, Vec2},
    pathfinding::{GridNode, PathfindingGrid},
    physics::PhysicsWorld,
    render::{Renderer, Sprite, TextureHandle, Tilemap},
    script::{ScriptComponent, ScriptParams, ScriptRuntime},
    Engine, EngineContext, Game, World,
};
use std::collections::HashSet;

struct ScriptedGridDemo {
    runtime: ScriptRuntime,
    world: World,
    physics: PhysicsWorld,
    camera: Camera2D,

    textures: TextureHandles,

    // Grid system
    grid: Grid<bool>, // true = walkable, false = blocked
    obstacles: HashSet<GridCoord>,
    pathfinding_grid: PathfindingGrid,

    // Agent entity
    agent_entity: Option<sindri::EntityId>,

    // Tilemap entity
    tilemap_entity: Option<TilemapEntity>,

    // UI
    hud: HudLayer,
    font: Option<sindri::FontHandle>,
    initialized: bool,
}

struct TextureHandles {
    tileset: Option<TextureHandle>, // For tilemap
    agent: Option<TextureHandle>,
    path: Option<TextureHandle>,
    target: Option<TextureHandle>,
}

struct TilemapEntity {
    entity: sindri::EntityId,
}

impl ScriptedGridDemo {
    fn new() -> Result<Self> {
        // Create a 30x20 grid with 32px cells
        let grid = Grid::new(30, 20, 32.0, true);
        let pathfinding_grid = PathfindingGrid::new(30, 20, 32.0);

        Ok(Self {
            runtime: ScriptRuntime::new()?,
            world: World::new(),
            physics: PhysicsWorld::new(),
            camera: Camera2D::new(Vec2::new(480.0, 320.0)),
            textures: TextureHandles {
                tileset: None,
                agent: None,
                path: None,
                target: None,
            },
            grid,
            obstacles: HashSet::new(),
            pathfinding_grid,
            agent_entity: None,
            tilemap_entity: None,
            hud: HudLayer::new(),
            font: None,
            initialized: false,
        })
    }

    fn create_textures(&mut self, renderer: &mut Renderer) -> Result<()> {
        // Load the real tileset image (960x960, 30x30 tiles of 32x32 each)
        let tileset_path = format!(
            "{}/assets/hyptosis_tile-art-batch-1.png",
            env!("CARGO_MANIFEST_DIR")
        );
        self.textures.tileset = Some(renderer.load_texture_from_file(&tileset_path)?);

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
        self.textures.agent = Some(renderer.load_texture_from_rgba(
            &agent_data,
            agent_size as u32,
            agent_size as u32,
        )?);

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

    fn create_tilemap(&mut self) -> Result<()> {
        if let Some(tileset) = self.textures.tileset {
            // Create tilemap matching our grid (30x20 tiles, 32px each)
            let map_width = 30;
            let map_height = 20;
            let tile_size = Vec2::new(32.0, 32.0);

            // Tileset is 960x960 with 32x32 tiles = 30x30 grid
            let tileset_cols = 30;
            let tileset_rows = 30;

            let mut tilemap = Tilemap::new(
                tileset,
                (tileset_cols, tileset_rows), // 30x30 tileset
                tile_size,
                (map_width, map_height),
                Vec2::ZERO, // Start at origin
            );

            // Fill with a nice floor tile (tile ID 1 = first tile, top-left)
            // You can experiment with different tile IDs from the tileset
            tilemap.fill_rect(0, 0, map_width, map_height, 1);

            // Mark obstacles with a wall tile (try different tile IDs to find a good wall)
            // Tile ID 2 would be the second tile in the first row
            // You might want to use a different tile ID depending on the tileset layout
            for coord in &self.obstacles {
                tilemap.set_tile(coord.x as u32, coord.y as u32, 2);
            }

            // Create entity for tilemap
            let entity = self.world.spawn();
            self.world.insert(entity, TilemapComponent::new(tilemap));
            self.tilemap_entity = Some(TilemapEntity { entity });
        }

        Ok(())
    }

    fn setup_obstacles(&mut self) {
        // Create some obstacles
        let obstacle_coords = vec![
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
                        self.pathfinding_grid
                            .set_walkable(GridNode::new(coord.x, coord.y), false);
                    }
                }
            }
        }
    }

    fn spawn_agent(&mut self) -> Result<()> {
        let entity = self.world.spawn();
        let start_pos = self.grid.grid_to_world(GridCoord::new(5, 5));

        self.world.insert(entity, Transform::new(start_pos));

        if let Some(agent_tex) = self.textures.agent {
            let mut sprite = SpriteComponent::new(agent_tex);
            sprite
                .sprite
                .set_size_px(Vec2::new(28.0, 28.0), Vec2::new(28.0, 28.0));
            self.world.insert(entity, sprite);
        }

        // Attach script
        let script_path = format!("{}/scripts/grid_agent.lua", env!("CARGO_MANIFEST_DIR"));
        let params = ScriptParams::default()
            .insert("cell_size", 32.0)
            .insert("move_duration", 0.3);

        self.world.insert(
            entity,
            ScriptComponent::default().with_script(script_path, params),
        );
        self.agent_entity = Some(entity);

        Ok(())
    }

    fn update_sprite_transforms(&mut self) {
        let entities: Vec<_> = self
            .world
            .query::<SpriteComponent>()
            .into_iter()
            .map(|(e, _)| e)
            .collect();

        for entity in entities {
            if let (Some(transform), Some(sprite)) = (
                self.world.get::<Transform>(entity).cloned(),
                self.world.get_mut::<SpriteComponent>(entity),
            ) {
                sprite.sprite.transform.position = transform.position;
                sprite.sprite.transform.rotation = transform.rotation;
                sprite.sprite.transform.scale = transform.scale;
            }
        }
    }
}

impl Game for ScriptedGridDemo {
    fn init(&mut self, ctx: &mut EngineContext) -> Result<()> {
        self.create_textures(ctx.renderer())?;
        self.setup_obstacles();

        self.font = Some(ctx.builtin_font(sindri::BuiltinFont::Ui)?);

        // Register grid/pathfinding functions for scripts
        let grid_clone = self.grid.clone();
        let pathfinding_clone = self.pathfinding_grid.clone();
        let camera_clone = self.camera.clone();
        let lua = self.runtime.lua_mut();

        // Helper to convert world to grid
        {
            let grid = grid_clone.clone();
            let func = lua.create_function(move |lua: &mlua::Lua, world_pos: Vec2| {
                let coord = grid.world_to_grid(world_pos);
                let table = lua.create_table()?;
                table.set("x", coord.x)?;
                table.set("y", coord.y)?;
                Ok(mlua::Value::Table(table))
            })?;
            lua.globals().set("world_to_grid", func)?;
        }

        // Helper to convert grid to world
        {
            let grid = grid_clone.clone();
            let func = lua.create_function(move |lua: &mlua::Lua, coord_table: mlua::Table| {
                let x: i32 = coord_table.get("x")?;
                let y: i32 = coord_table.get("y")?;
                let coord = sindri::GridCoord::new(x, y);
                Ok(grid.grid_to_world(coord))
            })?;
            lua.globals().set("grid_to_world", func)?;
        }

        // Helper to check if grid coord is walkable
        {
            let grid = grid_clone.clone();
            let func = lua.create_function(move |_lua: &mlua::Lua, coord_table: mlua::Table| {
                let x: i32 = coord_table.get("x")?;
                let y: i32 = coord_table.get("y")?;
                let coord = sindri::GridCoord::new(x, y);
                Ok(grid.get(coord).copied().unwrap_or(false))
            })?;
            lua.globals().set("is_walkable", func)?;
        }

        // Helper to find path
        {
            let pathfinding = pathfinding_clone.clone();
            let func = lua.create_function(
                move |lua: &mlua::Lua, (start_table, goal_table): (mlua::Table, mlua::Table)| {
                    let start_x: i32 = start_table.get("x")?;
                    let start_y: i32 = start_table.get("y")?;
                    let goal_x: i32 = goal_table.get("x")?;
                    let goal_y: i32 = goal_table.get("y")?;

                    let start_node = sindri::GridNode::new(start_x, start_y);
                    let goal_node = sindri::GridNode::new(goal_x, goal_y);

                    if let Some(path_nodes) =
                        sindri::AStarPathfinder::find_path_grid(&pathfinding, start_node, goal_node)
                    {
                        let path_table = lua.create_table()?;
                        for (i, node) in path_nodes.iter().enumerate() {
                            let node_table = lua.create_table()?;
                            node_table.set("x", node.x)?;
                            node_table.set("y", node.y)?;
                            path_table.set((i + 1) as i64, node_table)?;
                        }
                        Ok(Some(mlua::Value::Table(path_table)))
                    } else {
                        Ok(None)
                    }
                },
            )?;
            lua.globals().set("find_path", func)?;
        }

        // Helper to convert screen mouse to world (needs camera)
        {
            let camera = camera_clone.clone();
            let func = lua.create_function(move |_lua: &mlua::Lua, mouse_screen: Vec2| {
                // Fixed screen size for demo
                let screen_w = 1280.0;
                let screen_h = 720.0;
                Ok(camera.screen_to_world(mouse_screen, screen_w as u32, screen_h as u32))
            })?;
            lua.globals().set("mouse_world", func)?;
        }

        // Create tilemap
        self.create_tilemap()?;

        self.spawn_agent()?;

        self.initialized = true;
        Ok(())
    }

    fn update(&mut self, ctx: &mut EngineContext) -> Result<()> {
        let dt = ctx.delta_time().as_secs_f32();

        // Update mouse_world function with current camera and screen size
        let (screen_w, screen_h) = ctx.renderer().surface_size();
        let camera_clone = self.camera.clone();
        {
            let lua = self.runtime.lua_mut();
            let func = lua.create_function(move |_lua: &mlua::Lua, mouse_screen: Vec2| {
                Ok(camera_clone.screen_to_world(mouse_screen, screen_w, screen_h))
            })?;
            lua.globals().set("mouse_world", func)?;
        }

        // Update scripting runtime
        self.runtime
            .update(&mut self.world, &mut self.physics, ctx.input(), dt)?;

        // Sync sprite transforms from Transform components
        self.update_sprite_transforms();

        // Update camera to follow agent
        if let Some(agent) = self.agent_entity {
            if let Some(transform) = self.world.get::<Transform>(agent) {
                let offset = transform.position - self.camera.position;
                let half_dead_zone = Vec2::new(200.0 / 2.0, 150.0 / 2.0);

                if offset.x.abs() > half_dead_zone.x || offset.y.abs() > half_dead_zone.y {
                    let mut desired_pos = self.camera.position;
                    if offset.x.abs() > half_dead_zone.x {
                        desired_pos.x = transform.position.x - offset.x.signum() * half_dead_zone.x;
                    }
                    if offset.y.abs() > half_dead_zone.y {
                        desired_pos.y = transform.position.y - offset.y.signum() * half_dead_zone.y;
                    }
                    self.camera.position = self.camera.position.lerp(desired_pos, 0.15);
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut EngineContext) -> Result<()> {
        if !self.initialized {
            return Ok(());
        }

        let renderer = ctx.renderer();
        let mut frame = renderer.begin_frame()?;

        // Clear with light gray background
        renderer.clear(&mut frame, [0.85, 0.85, 0.85, 1.0])?;

        // Draw tilemap
        if let Some(tilemap_entity) = &self.tilemap_entity {
            if let Some(tilemap_comp) = self.world.get::<TilemapComponent>(tilemap_entity.entity) {
                renderer.draw_tilemap(&mut frame, &tilemap_comp.tilemap, &self.camera)?;
            }
        }

        // Draw agent
        if let Some(agent) = self.agent_entity {
            if let Some(sprite) = self.world.get::<SpriteComponent>(agent) {
                if sprite.visible {
                    renderer.draw_sprite(&mut frame, &sprite.sprite, &self.camera)?;
                }
            }
        }

        // Draw HUD
        self.hud.clear();
        if let Some(font) = self.font {
            self.hud.add_text(HudText {
                text: "Scripted Grid Demo".to_string(),
                font,
                size: 24.0,
                position: Vec2::new(10.0, 10.0),
                color: [1.0, 1.0, 1.0, 1.0],
                ..Default::default()
            });
            self.hud.add_text(HudText {
                text: "Left Click: Command agent to move".to_string(),
                font,
                size: 18.0,
                position: Vec2::new(10.0, 40.0),
                color: [0.9, 0.9, 0.9, 1.0],
                ..Default::default()
            });
        }
        self.hud.draw(renderer, &mut frame)?;

        renderer.end_frame(frame)?;
        Ok(())
    }
}

fn main() -> Result<()> {
    Engine::new()
        .with_title("Sindri Scripted Grid Demo")
        .with_size(1280, 720)
        .with_vsync(true)
        .run(ScriptedGridDemo::new()?)
}
