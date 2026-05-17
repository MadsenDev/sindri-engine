//! A* pathfinding implementation for 2D grids.

use crate::math::Vec2;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// A node in the pathfinding grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridNode {
    pub x: i32,
    pub y: i32,
}

impl GridNode {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &GridNode) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn manhattan_distance(&self, other: &GridNode) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }
}

/// Pathfinding grid that tracks walkable/non-walkable tiles.
#[derive(Clone, Debug)]
pub struct PathfindingGrid {
    width: usize,
    height: usize,
    cell_size: f32,
    walkable: Vec<bool>, // Row-major: [y * width + x]
}

impl PathfindingGrid {
    /// Create a new pathfinding grid.
    ///
    /// # Arguments
    /// * `width` - Grid width in cells
    /// * `height` - Grid height in cells
    /// * `cell_size` - Size of each cell in world units
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self {
        Self {
            width,
            height,
            cell_size,
            walkable: vec![true; width * height],
        }
    }

    /// Convert world position to grid coordinates.
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridNode {
        GridNode {
            x: (world_pos.x / self.cell_size).floor() as i32,
            y: (world_pos.y / self.cell_size).floor() as i32,
        }
    }

    /// Convert grid coordinates to world position (center of cell).
    pub fn grid_to_world(&self, node: GridNode) -> Vec2 {
        Vec2::new(
            (node.x as f32 + 0.5) * self.cell_size,
            (node.y as f32 + 0.5) * self.cell_size,
        )
    }

    /// Check if a grid node is valid (within bounds).
    pub fn is_valid(&self, node: &GridNode) -> bool {
        node.x >= 0 && node.x < self.width as i32 && node.y >= 0 && node.y < self.height as i32
    }

    /// Check if a grid node is walkable.
    pub fn is_walkable(&self, node: &GridNode) -> bool {
        if !self.is_valid(node) {
            return false;
        }
        let index = (node.y as usize) * self.width + (node.x as usize);
        self.walkable[index]
    }

    /// Set a grid node as walkable or not.
    pub fn set_walkable(&mut self, node: GridNode, walkable: bool) {
        if self.is_valid(&node) {
            let index = (node.y as usize) * self.width + (node.x as usize);
            self.walkable[index] = walkable;
        }
    }

    /// Set a rectangular area as walkable or not.
    pub fn set_area_walkable(&mut self, x: i32, y: i32, width: i32, height: i32, walkable: bool) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_walkable(GridNode::new(x + dx, y + dy), walkable);
            }
        }
    }

    /// Get neighbors of a node (8-directional).
    pub fn get_neighbors(&self, node: &GridNode) -> Vec<GridNode> {
        let mut neighbors = Vec::new();
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

        for (dx, dy) in directions.iter() {
            let neighbor = GridNode::new(node.x + dx, node.y + dy);
            if self.is_walkable(&neighbor) {
                neighbors.push(neighbor);
            }
        }

        neighbors
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }
}

/// A* pathfinding algorithm.
pub struct AStarPathfinder;

#[derive(Clone, Copy, PartialEq, Eq)]
struct NodeWithCost {
    node: GridNode,
    f_cost: i32, // Total cost (g + h)
}

impl Ord for NodeWithCost {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse order for min-heap (lowest cost first)
        other.f_cost.cmp(&self.f_cost)
    }
}

impl PartialOrd for NodeWithCost {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AStarPathfinder {
    /// Find a path from start to goal using A* algorithm.
    ///
    /// Returns `Some(Vec<Vec2>)` with world positions if a path is found, `None` otherwise.
    pub fn find_path(
        grid: &PathfindingGrid,
        start_world: Vec2,
        goal_world: Vec2,
    ) -> Option<Vec<Vec2>> {
        let start = grid.world_to_grid(start_world);
        let goal = grid.world_to_grid(goal_world);

        if !grid.is_walkable(&start) || !grid.is_walkable(&goal) {
            return None;
        }

        if start == goal {
            return Some(vec![start_world, goal_world]);
        }

        // Open set: nodes to be evaluated (priority queue)
        let mut open_set = BinaryHeap::new();
        open_set.push(NodeWithCost {
            node: start,
            f_cost: 0,
        });

        // Maps to track path and costs
        let mut came_from: HashMap<GridNode, GridNode> = HashMap::new();
        let mut g_score: HashMap<GridNode, i32> = HashMap::new();
        g_score.insert(start, 0);

        let mut closed_set: HashSet<GridNode> = HashSet::new();

        while let Some(NodeWithCost { node: current, .. }) = open_set.pop() {
            if current == goal {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = goal;
                path.push(grid.grid_to_world(node));

                while let Some(&prev) = came_from.get(&node) {
                    path.push(grid.grid_to_world(prev));
                    node = prev;
                    if node == start {
                        break;
                    }
                }

                path.reverse();
                return Some(path);
            }

            closed_set.insert(current);

            for neighbor in grid.get_neighbors(&current) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                // Calculate movement cost (diagonal = 14, cardinal = 10)
                let is_diagonal =
                    (neighbor.x - current.x).abs() == 1 && (neighbor.y - current.y).abs() == 1;
                let move_cost = if is_diagonal { 14 } else { 10 };

                let tentative_g = g_score.get(&current).unwrap_or(&i32::MAX) + move_cost;

                if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g);

                    let h_cost = neighbor.manhattan_distance(&goal);
                    let f_cost = tentative_g + h_cost;

                    open_set.push(NodeWithCost {
                        node: neighbor,
                        f_cost,
                    });
                }
            }
        }

        None // No path found
    }

    /// Find a path and return grid nodes instead of world positions.
    pub fn find_path_grid(
        grid: &PathfindingGrid,
        start: GridNode,
        goal: GridNode,
    ) -> Option<Vec<GridNode>> {
        if !grid.is_walkable(&start) || !grid.is_walkable(&goal) {
            return None;
        }

        if start == goal {
            return Some(vec![start, goal]);
        }

        let mut open_set = BinaryHeap::new();
        open_set.push(NodeWithCost {
            node: start,
            f_cost: 0,
        });

        let mut came_from: HashMap<GridNode, GridNode> = HashMap::new();
        let mut g_score: HashMap<GridNode, i32> = HashMap::new();
        g_score.insert(start, 0);

        let mut closed_set: HashSet<GridNode> = HashSet::new();

        while let Some(NodeWithCost { node: current, .. }) = open_set.pop() {
            if current == goal {
                let mut path = Vec::new();
                let mut node = goal;
                path.push(node);

                while let Some(&prev) = came_from.get(&node) {
                    path.push(prev);
                    node = prev;
                    if node == start {
                        break;
                    }
                }

                path.reverse();
                return Some(path);
            }

            closed_set.insert(current);

            for neighbor in grid.get_neighbors(&current) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                let is_diagonal =
                    (neighbor.x - current.x).abs() == 1 && (neighbor.y - current.y).abs() == 1;
                let move_cost = if is_diagonal { 14 } else { 10 };

                let tentative_g = g_score.get(&current).unwrap_or(&i32::MAX) + move_cost;

                if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g);

                    let h_cost = neighbor.manhattan_distance(&goal);
                    let f_cost = tentative_g + h_cost;

                    open_set.push(NodeWithCost {
                        node: neighbor,
                        f_cost,
                    });
                }
            }
        }

        None
    }
}
