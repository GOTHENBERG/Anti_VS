use macroquad::prelude::Vec2;
use std::collections::HashMap;

pub const CELL_SIZE: f32 = 64.0;

#[derive(Debug, Clone)]
pub struct SpatialGrid {
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialGrid {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    fn to_cell_coords(pos: Vec2) -> (i32, i32) {
        (
            (pos.x / CELL_SIZE).floor() as i32,
            (pos.y / CELL_SIZE).floor() as i32,
        )
    }

    pub fn insert(&mut self, entity_id: usize, pos: Vec2) {
        let coords = Self::to_cell_coords(pos);
        self.cells.entry(coords).or_default().push(entity_id);
    }

    /// Возвращает ID всех сущностей в окрестности (9 соседних ячеек)
    pub fn query_nearby(&self, pos: Vec2, radius: f32) -> Vec<usize> {
        let mut result = Vec::new();
        let (min_x, min_y) = Self::to_cell_coords(pos - Vec2::splat(radius));
        let (max_x, max_y) = Self::to_cell_coords(pos + Vec2::splat(radius));

        for cx in min_x..=max_x {
            for cy in min_y..=max_y {
                if let Some(entities) = self.cells.get(&(cx, cy)) {
                    result.extend_from_slice(entities);
                }
            }
        }
        result
    }
}
