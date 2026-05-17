//! General-purpose grid system for tile-based games.
//!
//! Provides a flexible grid structure that can store arbitrary data per cell,
//! with utilities for coordinate conversion, neighbor queries, and common grid operations.

use crate::math::Vec2;

/// A node in the grid (grid coordinates).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridCoord {
    pub x: i32,
    pub y: i32,
}

impl GridCoord {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Manhattan distance to another coordinate.
    pub fn manhattan_distance(&self, other: &GridCoord) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Euclidean distance to another coordinate.
    pub fn distance(&self, other: &GridCoord) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }
}

/// General-purpose grid that can store arbitrary data per cell.
#[derive(Clone, Debug)]
pub struct Grid<T> {
    width: usize,
    height: usize,
    cell_size: f32,
    cells: Vec<T>, // Row-major: [y * width + x]
}

impl<T: Clone> Grid<T> {
    /// Create a new grid with the given dimensions and cell size.
    ///
    /// # Arguments
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `cell_size` - Size of each cell in world units
    /// * `default` - Default value for all cells
    pub fn new(width: usize, height: usize, cell_size: f32, default: T) -> Self {
        Self {
            width,
            height,
            cell_size,
            cells: vec![default; width * height],
        }
    }

    /// Get the width of the grid in cells.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get the height of the grid in cells.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Get the size of each cell in world units.
    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }

    /// Convert world position to grid coordinates.
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridCoord {
        GridCoord {
            x: (world_pos.x / self.cell_size).floor() as i32,
            y: (world_pos.y / self.cell_size).floor() as i32,
        }
    }

    /// Convert grid coordinates to world position (center of cell).
    pub fn grid_to_world(&self, coord: GridCoord) -> Vec2 {
        Vec2::new(
            (coord.x as f32 + 0.5) * self.cell_size,
            (coord.y as f32 + 0.5) * self.cell_size,
        )
    }

    /// Convert grid coordinates to world position (top-left corner of cell).
    pub fn grid_to_world_top_left(&self, coord: GridCoord) -> Vec2 {
        Vec2::new(
            coord.x as f32 * self.cell_size,
            coord.y as f32 * self.cell_size,
        )
    }

    /// Check if a grid coordinate is valid (within bounds).
    pub fn is_valid(&self, coord: &GridCoord) -> bool {
        coord.x >= 0 && coord.x < self.width as i32 && coord.y >= 0 && coord.y < self.height as i32
    }

    /// Get the cell data at the given coordinate.
    /// Returns `None` if the coordinate is out of bounds.
    pub fn get(&self, coord: GridCoord) -> Option<&T> {
        if !self.is_valid(&coord) {
            return None;
        }
        let index = (coord.y as usize) * self.width + (coord.x as usize);
        self.cells.get(index)
    }

    /// Get mutable access to the cell data at the given coordinate.
    /// Returns `None` if the coordinate is out of bounds.
    pub fn get_mut(&mut self, coord: GridCoord) -> Option<&mut T> {
        if !self.is_valid(&coord) {
            return None;
        }
        let index = (coord.y as usize) * self.width + (coord.x as usize);
        self.cells.get_mut(index)
    }

    /// Set the cell data at the given coordinate.
    /// Returns `false` if the coordinate is out of bounds.
    pub fn set(&mut self, coord: GridCoord, value: T) -> bool {
        if let Some(cell) = self.get_mut(coord) {
            *cell = value;
            true
        } else {
            false
        }
    }

    /// Get neighbors of a coordinate (4-directional: up, down, left, right).
    pub fn neighbors_4(&self, coord: &GridCoord) -> Vec<GridCoord> {
        let directions = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let mut neighbors = Vec::new();
        for (dx, dy) in directions.iter() {
            let neighbor = GridCoord::new(coord.x + dx, coord.y + dy);
            if self.is_valid(&neighbor) {
                neighbors.push(neighbor);
            }
        }
        neighbors
    }

    /// Get neighbors of a coordinate (8-directional: includes diagonals).
    pub fn neighbors_8(&self, coord: &GridCoord) -> Vec<GridCoord> {
        let directions = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        let mut neighbors = Vec::new();
        for (dx, dy) in directions.iter() {
            let neighbor = GridCoord::new(coord.x + dx, coord.y + dy);
            if self.is_valid(&neighbor) {
                neighbors.push(neighbor);
            }
        }
        neighbors
    }

    /// Iterate over all coordinates in the grid.
    pub fn iter_coords(&self) -> impl Iterator<Item = GridCoord> {
        let width = self.width as i32;
        let height = self.height as i32;
        (0..height).flat_map(move |y| (0..width).map(move |x| GridCoord::new(x, y)))
    }

    /// Iterate over all cells with their coordinates.
    pub fn iter(&self) -> impl Iterator<Item = (GridCoord, &T)> {
        self.iter_coords()
            .map(move |coord| (coord, self.get(coord).unwrap()))
    }

    /// Iterate over all cells mutably with their coordinates.
    /// Note: This returns indices rather than mutable references due to Rust's borrowing rules.
    /// Use `get_mut` with the coordinates from `iter_coords()` if you need mutable access.
    pub fn iter_mut_indices(&mut self) -> impl Iterator<Item = (GridCoord, usize)> {
        let width = self.width;
        (0..self.height as i32).flat_map(move |y| {
            (0..width as i32).map(move |x| {
                let coord = GridCoord::new(x, y);
                let index = (y as usize) * width + (x as usize);
                (coord, index)
            })
        })
    }
}

/// Helper trait for grid-based pathfinding.
/// Types that implement this can be used with pathfinding algorithms.
pub trait GridPathfinding {
    /// Check if a cell is walkable/passable.
    fn is_walkable(&self, coord: &GridCoord) -> bool;
}

/// Default implementation for boolean grids (true = walkable).
impl GridPathfinding for Grid<bool> {
    fn is_walkable(&self, coord: &GridCoord) -> bool {
        self.get(*coord).copied().unwrap_or(false)
    }
}
