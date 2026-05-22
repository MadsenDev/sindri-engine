//! A* pathfinding for 2D grids: top-down (4/8-dir) and platformer (jump-arc graph).

use crate::math::Vec2;
use std::collections::{BinaryHeap, HashMap, HashSet};

// ── Mode ─────────────────────────────────────────────────────────────────────

/// How movement is modeled on this nav grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathfindingMode {
    /// 8-directional movement (standard top-down RPG / twin-stick).
    TopDown8,
    /// 4-directional cardinal movement (top-down roguelike / grid strategy).
    TopDown4,
    /// Platform-game movement: walk, fall, and jump arcs.
    Platformer {
        /// Gravity acceleration in world-units/s² (positive = downward).
        gravity: f32,
        /// Initial upward velocity when jumping (world-units/s).
        jump_velocity: f32,
        /// Horizontal move speed in world-units/s.
        move_speed: f32,
    },
}

impl Default for PathfindingMode {
    fn default() -> Self {
        Self::TopDown8
    }
}

// ── GridNode ─────────────────────────────────────────────────────────────────

/// Integer grid coordinate used during pathfinding.
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

// ── PathfindingGrid ───────────────────────────────────────────────────────────

/// Walkability grid that drives top-down A* and serves as input to the platform graph builder.
#[derive(Clone, Debug)]
pub struct PathfindingGrid {
    width: usize,
    height: usize,
    cell_size: f32,
    /// World-space top-left origin of this grid.
    origin: Vec2,
    /// Row-major: `[y * width + x]`; `true` = walkable.
    walkable: Vec<bool>,
}

impl PathfindingGrid {
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self {
        Self::with_origin(width, height, cell_size, Vec2::ZERO)
    }

    pub fn with_origin(width: usize, height: usize, cell_size: f32, origin: Vec2) -> Self {
        Self {
            width,
            height,
            cell_size,
            origin,
            walkable: vec![true; width * height],
        }
    }

    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }
    pub fn cell_size(&self) -> f32 { self.cell_size }
    pub fn origin(&self) -> Vec2 { self.origin }
    pub fn set_origin(&mut self, origin: Vec2) { self.origin = origin; }

    /// World position → grid coordinate.
    pub fn world_to_grid(&self, world_pos: Vec2) -> GridNode {
        GridNode {
            x: ((world_pos.x - self.origin.x) / self.cell_size).floor() as i32,
            y: ((world_pos.y - self.origin.y) / self.cell_size).floor() as i32,
        }
    }

    /// Grid coordinate → world center of cell.
    pub fn grid_to_world(&self, node: GridNode) -> Vec2 {
        Vec2::new(
            self.origin.x + (node.x as f32 + 0.5) * self.cell_size,
            self.origin.y + (node.y as f32 + 0.5) * self.cell_size,
        )
    }

    pub fn is_valid(&self, node: &GridNode) -> bool {
        node.x >= 0
            && node.x < self.width as i32
            && node.y >= 0
            && node.y < self.height as i32
    }

    pub fn is_walkable(&self, node: &GridNode) -> bool {
        if !self.is_valid(node) {
            return false;
        }
        self.walkable[(node.y as usize) * self.width + (node.x as usize)]
    }

    /// Returns `true` if the cell is NOT walkable (a solid wall/floor tile).
    pub fn is_solid(&self, node: &GridNode) -> bool {
        if !self.is_valid(node) {
            return true; // out-of-bounds counts as solid for platform detection
        }
        !self.walkable[(node.y as usize) * self.width + (node.x as usize)]
    }

    pub fn set_walkable(&mut self, node: GridNode, walkable: bool) {
        if self.is_valid(&node) {
            let idx = (node.y as usize) * self.width + (node.x as usize);
            self.walkable[idx] = walkable;
        }
    }

    pub fn set_area_walkable(&mut self, x: i32, y: i32, width: i32, height: i32, walkable: bool) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_walkable(GridNode::new(x + dx, y + dy), walkable);
            }
        }
    }

    /// Fill the entire grid with a single walkability value.
    pub fn fill(&mut self, walkable: bool) {
        self.walkable.fill(walkable);
    }

    /// Raw access to the walkable slice (for serialisation / gizmo rendering).
    pub fn walkable_slice(&self) -> &[bool] {
        &self.walkable
    }

    fn neighbors_topdown8(&self, node: &GridNode) -> Vec<GridNode> {
        let dirs: [(i32, i32); 8] = [
            (-1, -1), (0, -1), (1, -1),
            (-1,  0),           (1,  0),
            (-1,  1), (0,  1), (1,  1),
        ];
        let mut result = Vec::new();
        for (dx, dy) in dirs {
            let nb = GridNode::new(node.x + dx, node.y + dy);
            if !self.is_walkable(&nb) {
                continue;
            }
            // No corner cutting: diagonal moves require both orthogonal neighbours to be walkable.
            if dx != 0 && dy != 0 {
                if !self.is_walkable(&GridNode::new(node.x + dx, node.y))
                    || !self.is_walkable(&GridNode::new(node.x, node.y + dy))
                {
                    continue;
                }
            }
            result.push(nb);
        }
        result
    }

    fn neighbors_topdown4(&self, node: &GridNode) -> Vec<GridNode> {
        let dirs = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        dirs.iter()
            .map(|(dx, dy)| GridNode::new(node.x + dx, node.y + dy))
            .filter(|n| self.is_walkable(n))
            .collect()
    }

    pub fn get_neighbors(&self, node: &GridNode, mode: PathfindingMode) -> Vec<GridNode> {
        match mode {
            PathfindingMode::TopDown4 => self.neighbors_topdown4(node),
            PathfindingMode::TopDown8 | PathfindingMode::Platformer { .. } => {
                self.neighbors_topdown8(node)
            }
        }
    }
}

// ── A* (top-down) ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
struct AStarNode {
    node: GridNode,
    f_cost: i32,
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.f_cost.cmp(&self.f_cost) // min-heap
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// BFS outward from `node` to find the nearest walkable cell. Returns `None` only if the grid is
/// entirely blocked (pathologically unlikely).
fn nearest_walkable(grid: &PathfindingGrid, node: GridNode) -> Option<GridNode> {
    if grid.is_walkable(&node) {
        return Some(node);
    }
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(node);
    visited.insert(node);
    while let Some(cur) = queue.pop_front() {
        for (dx, dy) in [(-1,0),(1,0),(0,-1),(0,1),(-1,-1),(1,-1),(-1,1),(1,1)] {
            let nb = GridNode::new(cur.x + dx, cur.y + dy);
            if !visited.contains(&nb) && grid.is_valid(&nb) {
                if grid.is_walkable(&nb) {
                    return Some(nb);
                }
                visited.insert(nb);
                queue.push_back(nb);
            }
        }
        if visited.len() > 256 { break; } // give up if completely surrounded
    }
    None
}

/// Top-down A* pathfinder.
pub struct AStarPathfinder;

impl AStarPathfinder {
    /// Find a path returning world-space waypoints, or `None` if unreachable.
    pub fn find_path(
        grid: &PathfindingGrid,
        start_world: Vec2,
        goal_world: Vec2,
        mode: PathfindingMode,
    ) -> Option<Vec<Vec2>> {
        let start = nearest_walkable(grid, grid.world_to_grid(start_world))?;
        let goal  = nearest_walkable(grid, grid.world_to_grid(goal_world))?;

        Self::find_path_grid(grid, start, goal, mode)
            .map(|nodes| nodes.iter().map(|n| grid.grid_to_world(*n)).collect())
    }

    /// Find a path returning grid nodes.
    pub fn find_path_grid(
        grid: &PathfindingGrid,
        start: GridNode,
        goal: GridNode,
        mode: PathfindingMode,
    ) -> Option<Vec<GridNode>> {
        if !grid.is_walkable(&start) || !grid.is_walkable(&goal) {
            return None;
        }
        if start == goal {
            return Some(vec![start]);
        }

        let mut open = BinaryHeap::new();
        open.push(AStarNode { node: start, f_cost: 0 });
        let mut came_from: HashMap<GridNode, GridNode> = HashMap::new();
        let mut g: HashMap<GridNode, i32> = HashMap::new();
        g.insert(start, 0);
        let mut closed: HashSet<GridNode> = HashSet::new();

        while let Some(AStarNode { node: current, .. }) = open.pop() {
            if current == goal {
                return Some(reconstruct(&came_from, start, goal));
            }
            closed.insert(current);

            for neighbor in grid.get_neighbors(&current, mode) {
                if closed.contains(&neighbor) {
                    continue;
                }
                let diagonal =
                    (neighbor.x - current.x).abs() == 1 && (neighbor.y - current.y).abs() == 1;
                let move_cost = if diagonal { 14 } else { 10 };
                let tentative_g = g.get(&current).copied().unwrap_or(i32::MAX) + move_cost;
                if tentative_g < g.get(&neighbor).copied().unwrap_or(i32::MAX) {
                    came_from.insert(neighbor, current);
                    g.insert(neighbor, tentative_g);
                    let h = neighbor.manhattan_distance(&goal);
                    open.push(AStarNode { node: neighbor, f_cost: tentative_g + h });
                }
            }
        }
        None
    }
}

fn reconstruct(came_from: &HashMap<GridNode, GridNode>, start: GridNode, goal: GridNode) -> Vec<GridNode> {
    let mut path = Vec::new();
    let mut current = goal;
    loop {
        path.push(current);
        if current == start {
            break;
        }
        match came_from.get(&current) {
            Some(&prev) => current = prev,
            None => break,
        }
    }
    path.reverse();
    path
}

// ── Platform graph ─────────────────────────────────────────────────────────────

/// A single node in the platform reachability graph.
#[derive(Clone, Debug)]
pub struct PlatformNode {
    pub grid_x: i32,
    pub grid_y: i32,
}

/// How two platform nodes are connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Walk,
    Fall,
    Jump,
}

/// A directed edge in the platform graph.
#[derive(Clone, Debug)]
pub struct PlatformEdge {
    pub to: usize,
    pub kind: EdgeKind,
    pub cost: i32,
}

/// Pre-computed reachability graph for platformer pathfinding.
///
/// Nodes are ground cells (walkable cell where the cell directly below is
/// solid or out-of-bounds).  Edges represent walk, fall, and jump moves.
#[derive(Clone, Debug, Default)]
pub struct PlatformGraph {
    pub nodes: Vec<PlatformNode>,
    node_index: HashMap<(i32, i32), usize>,
    pub edges: Vec<Vec<PlatformEdge>>,
}

impl PlatformGraph {
    /// Build a platform graph from a walkability grid and jump physics.
    ///
    /// * `gravity`       – positive = downward (world-units / s²)
    /// * `jump_velocity` – initial upward speed (world-units / s)
    /// * `move_speed`    – horizontal speed (world-units / s)
    pub fn build(
        grid: &PathfindingGrid,
        gravity: f32,
        jump_velocity: f32,
        move_speed: f32,
    ) -> Self {
        let mut graph = PlatformGraph::default();

        // 1. Collect all ground nodes.
        for y in 0..grid.height() as i32 {
            for x in 0..grid.width() as i32 {
                let node = GridNode::new(x, y);
                if grid.is_walkable(&node) && grid.is_solid(&GridNode::new(x, y + 1)) {
                    let idx = graph.nodes.len();
                    graph.nodes.push(PlatformNode { grid_x: x, grid_y: y });
                    graph.node_index.insert((x, y), idx);
                    graph.edges.push(Vec::new());
                }
            }
        }

        // 2. Add edges.
        let cell = grid.cell_size();

        for src_idx in 0..graph.nodes.len() {
            let src = graph.nodes[src_idx].clone();

            // Walk left / right
            for dx in [-1_i32, 1_i32] {
                let nx = src.grid_x + dx;
                let ny = src.grid_y;
                if let Some(&dst_idx) = graph.node_index.get(&(nx, ny)) {
                    graph.edges[src_idx].push(PlatformEdge {
                        to: dst_idx,
                        kind: EdgeKind::Walk,
                        cost: 10,
                    });
                } else if grid.is_walkable(&GridNode::new(nx, ny)) {
                    // Walkable but not a ground node (above a gap) – fall from here.
                    if let Some(land) = simulate_fall(grid, nx, ny, gravity, move_speed, cell) {
                        if let Some(&dst_idx) = graph.node_index.get(&(land.0, land.1)) {
                            graph.edges[src_idx].push(PlatformEdge {
                                to: dst_idx,
                                kind: EdgeKind::Fall,
                                cost: 20,
                            });
                        }
                    }
                }
            }

            // Jump: try a range of horizontal targets
            let max_jump_range = compute_jump_range(jump_velocity, gravity, move_speed, cell);
            for dx in -max_jump_range..=max_jump_range {
                if dx == 0 {
                    continue;
                }
                let targets = simulate_jump(
                    grid,
                    src.grid_x,
                    src.grid_y,
                    dx,
                    jump_velocity,
                    gravity,
                    move_speed,
                    cell,
                );
                for (lx, ly) in targets {
                    if let Some(&dst_idx) = graph.node_index.get(&(lx, ly)) {
                        if src_idx != dst_idx {
                            // Avoid duplicate edges
                            if !graph.edges[src_idx].iter().any(|e| e.to == dst_idx && e.kind == EdgeKind::Jump) {
                                graph.edges[src_idx].push(PlatformEdge {
                                    to: dst_idx,
                                    kind: EdgeKind::Jump,
                                    cost: 30 + dx.unsigned_abs() as i32,
                                });
                            }
                        }
                    }
                }
            }
        }

        graph
    }

    pub fn node_index(&self) -> &HashMap<(i32, i32), usize> {
        &self.node_index
    }

    /// Find nearest ground node to an arbitrary grid position.
    pub fn nearest_node(&self, gx: i32, gy: i32) -> Option<usize> {
        if let Some(&idx) = self.node_index.get(&(gx, gy)) {
            return Some(idx);
        }
        // Search downward first (common case: start/goal is in open air)
        for dy in 0..self.nodes.len() as i32 {
            for ddx in [-1_i32, 0, 1] {
                let k = (gx + ddx, gy + dy);
                if let Some(&idx) = self.node_index.get(&k) {
                    return Some(idx);
                }
            }
        }
        // Fallback: linear search
        self.nodes
            .iter()
            .enumerate()
            .min_by_key(|(_, n)| {
                let dx = n.grid_x - gx;
                let dy = n.grid_y - gy;
                dx * dx + dy * dy
            })
            .map(|(i, _)| i)
    }
}

/// Returns how many cells horizontally the jump arc can reach.
fn compute_jump_range(jump_velocity: f32, gravity: f32, move_speed: f32, cell: f32) -> i32 {
    if gravity <= 0.0 || cell <= 0.0 || move_speed <= 0.0 {
        return 3;
    }
    let air_time = 2.0 * jump_velocity / gravity;
    let horizontal = move_speed * air_time;
    (horizontal / cell).ceil() as i32 + 1
}

/// Simulate falling from (start_x, start_y) moving one step horizontally per simulation tick.
/// Returns the landing (x, y) grid coordinate, or `None` if falls out of bounds.
fn simulate_fall(
    grid: &PathfindingGrid,
    start_x: i32,
    start_y: i32,
    gravity: f32,
    _move_speed: f32,
    cell: f32,
) -> Option<(i32, i32)> {
    let x = start_x;
    let mut y = start_y;
    let mut vy: f32 = 0.0;
    let dt = cell / cell.max(1.0); // 1 unit per step

    for _ in 0..100 {
        vy += gravity * dt * 0.05; // simplified time step
        let fall_cells = (vy * dt * 0.05 / cell).round() as i32;
        y += fall_cells.max(1);
        if grid.is_solid(&GridNode::new(x, y)) {
            let land_y = y - 1;
            if grid.is_walkable(&GridNode::new(x, land_y)) {
                return Some((x, land_y));
            }
            return None;
        }
        if y >= grid.height() as i32 {
            return None;
        }
    }
    None
}

/// Simulate a jump from (src_x, src_y) with horizontal offset `dx_cells`.
/// Returns all ground cells that the arc passes through and can land on.
fn simulate_jump(
    grid: &PathfindingGrid,
    src_x: i32,
    src_y: i32,
    dx_cells: i32,
    jump_velocity: f32,
    gravity: f32,
    move_speed: f32,
    cell: f32,
) -> Vec<(i32, i32)> {
    if gravity <= 0.0 || cell <= 0.0 || move_speed <= 0.0 {
        return vec![];
    }

    let target_x = src_x + dx_cells;
    let horizontal_dist = dx_cells.abs() as f32 * cell;
    let travel_time = horizontal_dist / move_speed;

    // Check if target x is reachable and land-able
    let land_y_f = src_y as f32 * cell
        - jump_velocity * travel_time
        + 0.5 * gravity * travel_time * travel_time;
    let land_y = (land_y_f / cell).round() as i32;

    let mut results = Vec::new();

    // The exact landing spot
    if grid.is_walkable(&GridNode::new(target_x, land_y))
        && grid.is_solid(&GridNode::new(target_x, land_y + 1))
    {
        results.push((target_x, land_y));
    }

    // Also check one cell up/down for tolerance
    for dy in [-1_i32, 1] {
        let ny = land_y + dy;
        if grid.is_walkable(&GridNode::new(target_x, ny))
            && grid.is_solid(&GridNode::new(target_x, ny + 1))
        {
            results.push((target_x, ny));
        }
    }

    results
}

// ── Platform A* ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
struct PlatformAStarNode {
    idx: usize,
    f_cost: i32,
}

impl Ord for PlatformAStarNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.f_cost.cmp(&self.f_cost)
    }
}

impl PartialOrd for PlatformAStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Pathfinder over a `PlatformGraph`.
pub struct PlatformPathfinder;

impl PlatformPathfinder {
    /// Find a path through the platform graph and return world-space waypoints.
    pub fn find_path(
        graph: &PlatformGraph,
        grid: &PathfindingGrid,
        start_world: Vec2,
        goal_world: Vec2,
    ) -> Option<Vec<Vec2>> {
        let start_grid = grid.world_to_grid(start_world);
        let goal_grid = grid.world_to_grid(goal_world);

        let start_node = graph.nearest_node(start_grid.x, start_grid.y)?;
        let goal_node = graph.nearest_node(goal_grid.x, goal_grid.y)?;

        if start_node == goal_node {
            return Some(vec![start_world, goal_world]);
        }

        let mut open = BinaryHeap::new();
        open.push(PlatformAStarNode { idx: start_node, f_cost: 0 });
        let mut came_from: HashMap<usize, usize> = HashMap::new();
        let mut g: HashMap<usize, i32> = HashMap::new();
        g.insert(start_node, 0);
        let mut closed: HashSet<usize> = HashSet::new();

        while let Some(PlatformAStarNode { idx: current, .. }) = open.pop() {
            if current == goal_node {
                // Reconstruct path
                let mut path_indices = Vec::new();
                let mut cur = goal_node;
                loop {
                    path_indices.push(cur);
                    if cur == start_node {
                        break;
                    }
                    match came_from.get(&cur) {
                        Some(&prev) => cur = prev,
                        None => break,
                    }
                }
                path_indices.reverse();

                let gn = &graph.nodes;
                let mut path: Vec<Vec2> = path_indices
                    .iter()
                    .map(|&i| {
                        grid.grid_to_world(GridNode::new(gn[i].grid_x, gn[i].grid_y))
                    })
                    .collect();

                // Prepend actual start / append actual goal for precision
                if let Some(first) = path.first_mut() {
                    *first = start_world;
                }
                if let Some(last) = path.last_mut() {
                    *last = goal_world;
                }
                return Some(path);
            }

            closed.insert(current);

            for edge in &graph.edges[current] {
                if closed.contains(&edge.to) {
                    continue;
                }
                let tentative_g = g.get(&current).copied().unwrap_or(i32::MAX) + edge.cost;
                if tentative_g < g.get(&edge.to).copied().unwrap_or(i32::MAX) {
                    came_from.insert(edge.to, current);
                    g.insert(edge.to, tentative_g);
                    let h = {
                        let gn = &graph.nodes;
                        let a = &gn[edge.to];
                        let b = &gn[goal_node];
                        ((a.grid_x - b.grid_x).abs() + (a.grid_y - b.grid_y).abs()) * 10
                    };
                    open.push(PlatformAStarNode {
                        idx: edge.to,
                        f_cost: tentative_g + h,
                    });
                }
            }
        }
        None
    }
}
