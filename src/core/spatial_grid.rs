use macroquad::prelude::Vec2;

pub const CELL_SIZE: f32 = 64.0;

/// Высокопроизводительная пространственная сетка с предварительно выделенной памятью
/// Позволяет мгновенно находить соседей за O(1) вместо O(N^2)
#[derive(Debug, Clone)]
pub struct SpatialGrid {
    cell_size: f32,
    cols: usize,
    rows: usize,
    /// Одномерный вектор корзин (flat array) для максимального кэш-попадания процессора
    buckets: Vec<Vec<usize>>,
}

impl SpatialGrid {
    pub fn new(width: f32, height: f32) -> Self {
        let cols = ((width / CELL_SIZE).ceil() as usize) + 4;
        let rows = ((height / CELL_SIZE).ceil() as usize) + 4;
        let total_cells = cols * rows;

        let mut buckets = Vec::with_capacity(total_cells);
        for _ in 0..total_cells {
            buckets.push(Vec::with_capacity(16));
        }

        Self {
            cell_size: CELL_SIZE,
            cols,
            rows,
            buckets,
        }
    }

    /// Очищает корзины без перевыделения памяти
    pub fn clear(&mut self) {
        for bucket in &mut self.buckets {
            bucket.clear();
        }
    }

    #[inline(always)]
    fn get_cell_index(&self, pos: Vec2) -> Option<usize> {
        let offset_x = pos.x + CELL_SIZE * 2.0;
        let offset_y = pos.y + CELL_SIZE * 2.0;

        if offset_x < 0.0 || offset_y < 0.0 {
            return None;
        }

        let cx = (offset_x / self.cell_size) as usize;
        let cy = (offset_y / self.cell_size) as usize;

        if cx < self.cols && cy < self.rows {
            Some(cy * self.cols + cx)
        } else {
            None
        }
    }

    /// Вставка сущности в сетку
    pub fn insert(&mut self, entity_idx: usize, pos: Vec2) {
        if let Some(idx) = self.get_cell_index(pos) {
            self.buckets[idx].push(entity_idx);
        }
    }

    /// Запрос всех сущностей в радиусе (проверяет соседние ячейки сетки)
    pub fn query_nearby(&self, pos: Vec2, radius: f32) -> Vec<usize> {
        let mut result = Vec::with_capacity(32);
        let offset_x = pos.x + CELL_SIZE * 2.0;
        let offset_y = pos.y + CELL_SIZE * 2.0;

        let min_cx = ((offset_x - radius).max(0.0) / self.cell_size) as usize;
        let max_cx = (((offset_x + radius).max(0.0) / self.cell_size) as usize).min(self.cols.saturating_sub(1));

        let min_cy = ((offset_y - radius).max(0.0) / self.cell_size) as usize;
        let max_cy = (((offset_y + radius).max(0.0) / self.cell_size) as usize).min(self.rows.saturating_sub(1));

        for cy in min_cy..=max_cy {
            let row_offset = cy * self.cols;
            for cx in min_cx..=max_cx {
                let cell_idx = row_offset + cx;
                if cell_idx < self.buckets.len() {
                    result.extend_from_slice(&self.buckets[cell_idx]);
                }
            }
        }

        result
    }
}
