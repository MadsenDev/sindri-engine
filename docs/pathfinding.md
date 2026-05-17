# Pathfinding

Sindri includes an A* pathfinding implementation for finding optimal paths on 2D grids.

## Overview

The pathfinding system provides:
- **A* algorithm** - Optimal pathfinding with heuristic search
- **8-directional movement** - Supports diagonal movement
- **Grid-based** - Works with discrete grid cells
- **World coordinate support** - Automatically converts between world and grid coordinates

## Basic Usage

### Creating a Pathfinding Grid

```rust
use sindri::{PathfindingGrid, GridNode};

// Create a 40x30 grid with 32px cells
let mut grid = PathfindingGrid::new(40, 30, 32.0);
```

### Setting Obstacles

```rust
// Mark a single cell as non-walkable
grid.set_walkable(GridNode::new(10, 5), false);

// Mark a rectangular area as non-walkable
grid.set_area_walkable(10, 5, 8, 1, false); // x, y, width, height, walkable
```

### Finding a Path

```rust
use sindri::{AStarPathfinder, Vec2};

let start = Vec2::new(100.0, 100.0);
let goal = Vec2::new(500.0, 400.0);

// Find path (returns world positions)
if let Some(path) = AStarPathfinder::find_path(&grid, start, goal) {
    // path is Vec<Vec2> with world positions
    for waypoint in &path {
        println!("Waypoint: {:?}", waypoint);
    }
} else {
    println!("No path found!");
}
```

### Finding a Path (Grid Coordinates)

```rust
let start_node = GridNode::new(5, 5);
let goal_node = GridNode::new(20, 15);

// Find path (returns grid nodes)
if let Some(path) = AStarPathfinder::find_path_grid(&grid, start_node, goal_node) {
    // path is Vec<GridNode>
    for node in &path {
        println!("Node: ({}, {})", node.x, node.y);
    }
}
```

## Pathfinding Grid

### PathfindingGrid

```rust
pub struct PathfindingGrid {
    // ...
}

impl PathfindingGrid {
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self;
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridNode;
    pub fn grid_to_world(&self, node: GridNode) -> Vec2;
    pub fn is_valid(&self, node: &GridNode) -> bool;
    pub fn is_walkable(&self, node: &GridNode) -> bool;
    pub fn set_walkable(&mut self, node: GridNode, walkable: bool);
    pub fn set_area_walkable(&mut self, x: i32, y: i32, width: i32, height: i32, walkable: bool);
    pub fn get_neighbors(&self, node: &GridNode) -> Vec<GridNode>;
}
```

### GridNode

```rust
pub struct GridNode {
    pub x: i32,
    pub y: i32,
}

impl GridNode {
    pub fn new(x: i32, y: i32) -> Self;
    pub fn distance_to(&self, other: &GridNode) -> f32;
    pub fn manhattan_distance(&self, other: &GridNode) -> i32;
}
```

## A* Algorithm

### AStarPathfinder

```rust
pub struct AStarPathfinder;

impl AStarPathfinder {
    /// Find path from start to goal (world coordinates)
    pub fn find_path(
        grid: &PathfindingGrid,
        start_world: Vec2,
        goal_world: Vec2,
    ) -> Option<Vec<Vec2>>;
    
    /// Find path from start to goal (grid coordinates)
    pub fn find_path_grid(
        grid: &PathfindingGrid,
        start: GridNode,
        goal: GridNode,
    ) -> Option<Vec<GridNode>>;
}
```

## Movement Costs

The A* implementation uses:
- **Cardinal movement** (up, down, left, right): Cost of 10
- **Diagonal movement**: Cost of 14 (approximately √2 × 10)

This creates natural diagonal movement while slightly favoring cardinal directions.

## Example: Agent Following a Path

```rust
use sindri::{AStarPathfinder, PathfindingGrid, Vec2};

struct Agent {
    position: Vec2,
    target: Option<Vec2>,
    path: Vec<Vec2>,
    path_index: usize,
}

impl Agent {
    fn update(&mut self, grid: &PathfindingGrid, dt: f32) {
        if let Some(target) = self.target {
            // Find path if we don't have one
            if self.path.is_empty() {
                if let Some(path) = AStarPathfinder::find_path(grid, self.position, target) {
                    self.path = path;
                    self.path_index = 0;
                }
            }
            
            // Move along path
            if self.path_index < self.path.len() {
                let next_pos = self.path[self.path_index];
                let direction = (next_pos - self.position).normalized();
                let speed = 150.0;
                
                self.position += direction * speed * dt;
                
                // Check if we've reached the next waypoint
                if self.position.distance(next_pos) < 5.0 {
                    self.path_index += 1;
                }
                
                // Check if we've reached the goal
                if self.path_index >= self.path.len() {
                    self.target = None;
                    self.path.clear();
                }
            }
        }
    }
}
```

## Integration with Grid System

The pathfinding system works well with the general-purpose `Grid<T>`:

```rust
use sindri::{Grid, PathfindingGrid, AStarPathfinder};

// Create both grids
let mut game_grid = Grid::new(40, 30, 32.0, true);
let mut pathfinding_grid = PathfindingGrid::new(40, 30, 32.0);

// Sync obstacles
for coord in game_grid.iter_coords() {
    if let Some(&walkable) = game_grid.get(coord) {
        pathfinding_grid.set_walkable(
            GridNode::new(coord.x, coord.y),
            walkable
        );
    }
}
```

## Performance Notes

- A* is efficient for most game scenarios
- For very large grids, consider hierarchical pathfinding
- Cache paths when possible if targets don't change frequently
- Use grid-based pathfinding for discrete movement, world-based for continuous

