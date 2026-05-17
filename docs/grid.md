# Grid System

Sindri provides a general-purpose grid system for tile-based games, turn-based strategy games, roguelikes, and other grid-based gameplay.

## Overview

The `Grid<T>` system allows you to store arbitrary data per cell, with utilities for coordinate conversion, neighbor queries, and common grid operations.

## Basic Usage

### Creating a Grid

```rust
use sindri::{Grid, GridCoord};

// Create a 40x30 grid with 32px cells, defaulting to true (walkable)
let mut grid = Grid::new(40, 30, 32.0, true);
```

### Coordinate Conversion

```rust
// Convert world position to grid coordinates
let world_pos = Vec2::new(100.0, 200.0);
let grid_coord = grid.world_to_grid(world_pos);
// grid_coord = GridCoord { x: 3, y: 6 }

// Convert grid coordinates to world position (center of cell)
let world_pos = grid.grid_to_world(grid_coord);
// Returns center of the cell in world space

// Convert to top-left corner of cell
let world_pos = grid.grid_to_world_top_left(grid_coord);
```

### Accessing Cells

```rust
// Get cell value
if let Some(value) = grid.get(GridCoord::new(5, 10)) {
    println!("Cell value: {:?}", value);
}

// Set cell value
grid.set(GridCoord::new(5, 10), false); // Mark as blocked

// Get mutable reference
if let Some(cell) = grid.get_mut(GridCoord::new(5, 10)) {
    *cell = false;
}
```

### Neighbor Queries

```rust
let coord = GridCoord::new(10, 10);

// Get 4-directional neighbors (up, down, left, right)
let neighbors_4 = grid.neighbors_4(&coord);

// Get 8-directional neighbors (includes diagonals)
let neighbors_8 = grid.neighbors_8(&coord);
```

### Iteration

```rust
// Iterate over all coordinates
for coord in grid.iter_coords() {
    // Process each cell
}

// Iterate over cells with their coordinates
for (coord, value) in grid.iter() {
    println!("Cell {:?} = {:?}", coord, value);
}
```

## Grid Coordinates

`GridCoord` represents a position in grid space:

```rust
pub struct GridCoord {
    pub x: i32,
    pub y: i32,
}

impl GridCoord {
    pub fn new(x: i32, y: i32) -> Self;
    pub fn manhattan_distance(&self, other: &GridCoord) -> i32;
    pub fn distance(&self, other: &GridCoord) -> f32;
}
```

## Example: Tile-Based Game

```rust
use sindri::{Grid, GridCoord, Vec2};

struct TileMap {
    grid: Grid<TileType>,
}

enum TileType {
    Floor,
    Wall,
    Door,
}

impl TileMap {
    fn new() -> Self {
        let mut grid = Grid::new(50, 50, 32.0, TileType::Floor);
        
        // Create walls around edges
        for x in 0..50 {
            grid.set(GridCoord::new(x, 0), TileType::Wall);
            grid.set(GridCoord::new(x, 49), TileType::Wall);
        }
        for y in 0..50 {
            grid.set(GridCoord::new(0, y), TileType::Wall);
            grid.set(GridCoord::new(49, y), TileType::Wall);
        }
        
        Self { grid }
    }
    
    fn is_walkable(&self, coord: GridCoord) -> bool {
        matches!(self.grid.get(coord), Some(TileType::Floor) | Some(TileType::Door))
    }
}
```

## Pathfinding Integration

The grid system works seamlessly with the pathfinding system:

```rust
use sindri::{Grid, PathfindingGrid, AStarPathfinder};

// Create a grid for gameplay
let mut game_grid = Grid::new(40, 30, 32.0, true);

// Create a corresponding pathfinding grid
let mut pathfinding_grid = PathfindingGrid::new(40, 30, 32.0);

// Sync obstacles
for coord in game_grid.iter_coords() {
    if let Some(&is_walkable) = game_grid.get(coord) {
        pathfinding_grid.set_walkable(
            GridNode::new(coord.x, coord.y),
            is_walkable
        );
    }
}

// Find path
let start = Vec2::new(100.0, 100.0);
let goal = Vec2::new(500.0, 400.0);
if let Some(path) = AStarPathfinder::find_path(&pathfinding_grid, start, goal) {
    // Path found!
}
```

## GridPathfinding Trait

For custom grid types, implement `GridPathfinding`:

```rust
use sindri::{Grid, GridCoord, GridPathfinding};

impl GridPathfinding for Grid<MyTileType> {
    fn is_walkable(&self, coord: &GridCoord) -> bool {
        self.get(*coord)
            .map(|tile| tile.is_walkable())
            .unwrap_or(false)
    }
}
```

