use super::TextureHandle;
use crate::math::Vec2;

/// A single tile in a tilemap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    /// Tile ID (index into tileset, 0 = empty/no tile)
    pub id: u32,
}

impl Tile {
    pub fn new(id: u32) -> Self {
        Self { id }
    }

    pub fn empty() -> Self {
        Self { id: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.id == 0
    }
}

/// Tilemap component for rendering tile-based maps.
#[derive(Clone, Debug)]
pub struct Tilemap {
    /// The tileset texture (contains all tile graphics in a grid)
    pub tileset: TextureHandle,
    /// Number of tiles in the tileset (columns, rows)
    pub tileset_size: (u32, u32),
    /// Size of each tile in pixels
    pub tile_size: Vec2,
    /// Map dimensions (width, height) in tiles
    pub map_size: (u32, u32),
    /// Tile data (row-major: [y * width + x])
    pub tiles: Vec<Tile>,
    /// Position of the tilemap in world space (top-left corner)
    pub position: Vec2,
    /// Tint color applied to all tiles
    pub tint: [f32; 4],
}

impl Tilemap {
    /// Create a new empty tilemap.
    pub fn new(
        tileset: TextureHandle,
        tileset_size: (u32, u32),
        tile_size: Vec2,
        map_size: (u32, u32),
        position: Vec2,
    ) -> Self {
        let (width, height) = map_size;
        Self {
            tileset,
            tileset_size,
            tile_size,
            map_size,
            tiles: vec![Tile::empty(); (width * height) as usize],
            position,
            tint: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// Set a tile at the given coordinates.
    pub fn set_tile(&mut self, x: u32, y: u32, tile_id: u32) {
        let (width, height) = self.map_size;
        if x < width && y < height {
            let index = (y * width + x) as usize;
            self.tiles[index] = Tile::new(tile_id);
        }
    }

    /// Get a tile at the given coordinates.
    pub fn get_tile(&self, x: u32, y: u32) -> Option<Tile> {
        let (width, height) = self.map_size;
        if x < width && y < height {
            let index = (y * width + x) as usize;
            Some(self.tiles[index])
        } else {
            None
        }
    }

    /// Fill a rectangular area with a tile ID.
    pub fn fill_rect(&mut self, x: u32, y: u32, width: u32, height: u32, tile_id: u32) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_tile(x + dx, y + dy, tile_id);
            }
        }
    }

    /// Get the world position of a tile's center.
    pub fn tile_to_world(&self, x: u32, y: u32) -> Vec2 {
        Vec2::new(
            self.position.x + (x as f32 + 0.5) * self.tile_size.x,
            self.position.y + (y as f32 + 0.5) * self.tile_size.y,
        )
    }

    /// Get the tile coordinates for a world position.
    pub fn world_to_tile(&self, world_pos: Vec2) -> (i32, i32) {
        let local_x = world_pos.x - self.position.x;
        let local_y = world_pos.y - self.position.y;
        (
            (local_x / self.tile_size.x).floor() as i32,
            (local_y / self.tile_size.y).floor() as i32,
        )
    }

    /// Get the UV rectangle for a tile ID in the tileset.
    pub fn tile_uv_rect(&self, tile_id: u32) -> Option<[f32; 4]> {
        if tile_id == 0 {
            return None; // Empty tile
        }

        let (cols, rows) = self.tileset_size;
        let tile_index = tile_id - 1; // 0-indexed (tile_id 1 = first tile)

        if tile_index >= cols * rows {
            return None; // Invalid tile ID
        }

        let col = tile_index % cols;
        let row = tile_index / cols;

        let uv_width = 1.0 / cols as f32;
        let uv_height = 1.0 / rows as f32;
        let u = col as f32 * uv_width;
        let v = row as f32 * uv_height;

        Some([u, v, uv_width, uv_height])
    }
}
