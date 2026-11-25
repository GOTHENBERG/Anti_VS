use macroquad::prelude::Vec2;
use crate::core::spatial_grid::{SpatialGrid, CELL_SIZE};

pub const CROWD_COLS: usize = 207;
pub const CROWD_ROWS: usize = 139;
pub const CROWD_CELLS: usize = CROWD_COLS * CROWD_ROWS; // 28,773
pub const CROWD_PRESSURE_WEIGHT: f32 = 0.45; // Коэффициент выталкивания из зон высокой плотности

/// Поле плотности и градиентов сплошной среды (Continuum Crowds Density Field).
/// Позволяет 20 000 мобов мгновенно избегать скоплений и течь как неньютоновская жидкость.
#[derive(Debug, Clone)]
pub struct CrowdDensityGrid {
    pub cols: usize,
    pub rows: usize,
    pub cell_size: f32,
    density: Vec<f32>,
    blurred: Vec<f32>,
    grad_x: Vec<f32>,
    grad_y: Vec<f32>,
    /// Интегральная таблица (Summed-Area Table) строится 1 раз за кадр из `density`
    /// и позволяет находить сумму мобов в любом прямоугольнике ровно за 4 чтения O(1).
    sat: Vec<f32>,
    active_indices: Vec<usize>,
    min_active_cx: usize,
    max_active_cx: usize,
    min_active_cy: usize,
    max_active_cy: usize,
}

impl Default for CrowdDensityGrid {
    fn default() -> Self {
        Self::new(crate::core::MAP_WIDTH, crate::core::MAP_HEIGHT)
    }
}

impl CrowdDensityGrid {
    pub fn new(width: f32, height: f32) -> Self {
        let cols = ((width / CELL_SIZE).ceil() as usize) + 4;
        let rows = ((height / CELL_SIZE).ceil() as usize) + 4;
        let total_cells = cols * rows;

        Self {
            cols,
            rows,
            cell_size: CELL_SIZE,
            density: vec![0.0; total_cells],
            blurred: vec![0.0; total_cells],
            grad_x: vec![0.0; total_cells],
            grad_y: vec![0.0; total_cells],
            sat: vec![0.0; total_cells],
            active_indices: Vec::with_capacity(16384),
            min_active_cx: usize::MAX,
            max_active_cx: 0,
            min_active_cy: usize::MAX,
            max_active_cy: 0,
        }
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        !self.active_indices.is_empty()
    }

    /// Быстрая, кристально чистая очистка через SIMD memset (.fill(0.0)) за ~0.01 мс.
    /// Полностью исключает призрачные хвосты данных и утечки векторов.
    #[inline]
    pub fn clear(&mut self) {
        if self.active_indices.is_empty() {
            return;
        }
        self.density.fill(0.0);
        self.blurred.fill(0.0);
        self.grad_x.fill(0.0);
        self.grad_y.fill(0.0);
        self.sat.fill(0.0);
        self.active_indices.clear();
        self.min_active_cx = usize::MAX;
        self.max_active_cx = 0;
        self.min_active_cy = usize::MAX;
        self.max_active_cy = 0;
    }

    #[inline(always)]
    pub fn cell_coords(&self, pos: Vec2) -> Option<(usize, usize)> {
        let offset_x = pos.x + self.cell_size * 2.0;
        let offset_y = pos.y + self.cell_size * 2.0;
        if offset_x < 0.0 || offset_y < 0.0 {
            return None;
        }
        let cx = (offset_x / self.cell_size) as usize;
        let cy = (offset_y / self.cell_size) as usize;
        if cx < self.cols && cy < self.rows {
            Some((cx, cy))
        } else {
            None
        }
    }

    /// Растеризация (Splatting) позиций мобов в сетку плотностей за O(N).
    pub fn splat_positions(&mut self, positions: &[Vec2]) {
        for &pos in positions {
            if let Some((cx, cy)) = self.cell_coords(pos) {
                let idx = cy * self.cols + cx;
                if self.density[idx] == 0.0 {
                    self.active_indices.push(idx);
                    self.min_active_cx = self.min_active_cx.min(cx);
                    self.max_active_cx = self.max_active_cx.max(cx);
                    self.min_active_cy = self.min_active_cy.min(cy);
                    self.max_active_cy = self.max_active_cy.max(cy);
                }
                self.density[idx] += 1.0;
            }
        }
    }

    /// Сглаживание 3x3 и аналитический расчет градиентов давления.
    /// Обрабатывает активные ячейки вместе с их 2-соседним ореолом (Halo),
    /// гарантируя непрерывность поля и корректное выталкивание на границах толпы.
    pub fn compute_blurred_gradients(&mut self) {
        if self.active_indices.is_empty() {
            return;
        }

        let cols = self.cols;
        let rows = self.rows;

        // Расширяем охват на 2 ячейки в стороны для честного вычисления ореола
        let min_x = self.min_active_cx.saturating_sub(2);
        let max_x = (self.max_active_cx + 2).min(cols - 1);
        let min_y = self.min_active_cy.saturating_sub(2);
        let max_y = (self.max_active_cy + 2).min(rows - 1);

        // 1. Сглаживание 3x3 внутри расширенного прямоугольника (кэш-линейный обход)
        for cy in min_y..=max_y {
            let row_cy = cy * cols;
            let ny_min = cy.saturating_sub(1);
            let ny_max = (cy + 1).min(rows - 1);

            for cx in min_x..=max_x {
                let nx_min = cx.saturating_sub(1);
                let nx_max = (cx + 1).min(cols - 1);

                let mut sum = 0.0f32;
                let mut count = 0.0f32;
                for ny in ny_min..=ny_max {
                    let row_ny = ny * cols;
                    for nx in nx_min..=nx_max {
                        sum += self.density[row_ny + nx];
                        count += 1.0;
                    }
                }
                self.blurred[row_cy + cx] = sum / count;
            }
        }

        // 2. Центральные разности для расчета градиента nabla rho
        for cy in min_y..=max_y {
            let row_cy = cy * cols;
            for cx in min_x..=max_x {
                let idx = row_cy + cx;
                let left = if cx > 0 { self.blurred[idx - 1] } else { self.blurred[idx] };
                let right = if cx + 1 < cols { self.blurred[idx + 1] } else { self.blurred[idx] };
                let down = if cy > 0 { self.blurred[idx - cols] } else { self.blurred[idx] };
                let up = if cy + 1 < rows { self.blurred[idx + cols] } else { self.blurred[idx] };

                self.grad_x[idx] = (right - left) * 0.5;
                self.grad_y[idx] = (up - down) * 0.5;
            }
        }

        // 3. Построение Summed-Area Table (SAT) ровно 1 раз за кадр из raw density.
        // Обеспечивает мгновенный O(1) расчёт плотности для любого AoE-радиуса за 4 чтения из памяти.
        if cols > 0 && rows > 0 {
            let mut first_row_prefix = 0.0f32;
            for cx in 0..cols {
                first_row_prefix += self.density[cx];
                self.sat[cx] = first_row_prefix;
            }
            for cy in 1..rows {
                let row = cy * cols;
                let prev_row = (cy - 1) * cols;
                let mut row_prefix = 0.0f32;
                for cx in 0..cols {
                    row_prefix += self.density[row + cx];
                    self.sat[row + cx] = self.sat[prev_row + cx] + row_prefix;
                }
            }
        }
    }

    /// Мировые координаты центра ячейки (cx, cy).
    #[inline(always)]
    pub fn cell_center(&self, cx: usize, cy: usize) -> Vec2 {
        Vec2::new(
            (cx as f32 + 0.5) * self.cell_size - self.cell_size * 2.0,
            (cy as f32 + 0.5) * self.cell_size - self.cell_size * 2.0,
        )
    }

    /// Сумма плотностей в произвольном прямоугольнике ячеек [min_cx..=max_cx, min_cy..=max_cy] за O(1) (4 чтения).
    #[inline(always)]
    pub fn query_sat_rect(&self, min_cx: usize, min_cy: usize, max_cx: usize, max_cy: usize) -> f32 {
        if self.cols == 0 || self.rows == 0 || min_cx > max_cx || min_cy > max_cy {
            return 0.0;
        }
        let x1 = max_cx.min(self.cols - 1);
        let y1 = max_cy.min(self.rows - 1);
        if min_cx > x1 || min_cy > y1 {
            return 0.0;
        }
        let total = self.sat[y1 * self.cols + x1];
        let top = if min_cy > 0 { self.sat[(min_cy - 1) * self.cols + x1] } else { 0.0 };
        let left = if min_cx > 0 { self.sat[y1 * self.cols + (min_cx - 1)] } else { 0.0 };
        let top_left = if min_cx > 0 && min_cy > 0 {
            self.sat[(min_cy - 1) * self.cols + (min_cx - 1)]
        } else {
            0.0
        };
        (total - top - left + top_left).max(0.0)
    }

    /// Находит ячейку с максимальным двухступенчатым KDE-откликом в пределах `max_range` от `boss_pos`
    /// с помощью кэшированной за кадр таблицы SAT (0 аллокаций, O(1) на ячейку).
    pub fn find_peak_density_cell(
        &self,
        boss_pos: Vec2,
        max_range: f32,
        aoe_radius: f32,
    ) -> Option<(Vec2, usize, usize)> {
        if self.active_indices.is_empty() || self.cols == 0 || self.rows == 0 {
            return None;
        }

        let offset = self.cell_size * 2.0;
        let inv_h = 1.0 / self.cell_size;
        let min_cx = (((boss_pos.x + offset - max_range) * inv_h).floor().max(0.0) as usize).min(self.cols - 1);
        let max_cx = (((boss_pos.x + offset + max_range) * inv_h).floor().max(0.0) as usize).min(self.cols - 1);
        let min_cy = (((boss_pos.y + offset - max_range) * inv_h).floor().max(0.0) as usize).min(self.rows - 1);
        let max_cy = (((boss_pos.y + offset + max_range) * inv_h).floor().max(0.0) as usize).min(self.rows - 1);

        let r_inner = ((aoe_radius * 0.45) * inv_h).floor().max(0.0) as usize;
        let r_outer = (aoe_radius * inv_h).round().max(1.0) as usize;
        let max_dist_pad = max_range + self.cell_size * 0.5;
        let max_dist_pad_sq = max_dist_pad * max_dist_pad;
        let max_range_safe = max_range.max(1.0);

        let mut best_score = 0.0f32;
        let mut best_cell = None;

        for cy in min_cy..=max_cy {
            for cx in min_cx..=max_cx {
                let c_pos = self.cell_center(cx, cy);
                let dist_sq = c_pos.distance_squared(boss_pos);
                if dist_sq > max_dist_pad_sq {
                    continue;
                }

                let inner_sum = self.query_sat_rect(
                    cx.saturating_sub(r_inner),
                    cy.saturating_sub(r_inner),
                    cx + r_inner,
                    cy + r_inner,
                );
                let outer_sum = self.query_sat_rect(
                    cx.saturating_sub(r_outer),
                    cy.saturating_sub(r_outer),
                    cx + r_outer,
                    cy + r_outer,
                );

                if outer_sum <= 0.0 {
                    continue;
                }

                let dist = dist_sq.sqrt().min(max_range_safe);
                let prox_weight = 1.0 + (max_range_safe - dist) / max_range_safe;
                let score = (2.0 * inner_sum + outer_sum) * prox_weight;
                if score > best_score {
                    best_score = score;
                    best_cell = Some((c_pos, cx, cy));
                }
            }
        }

        best_cell
    }

    /// Сэмплирование градиента плотности в произвольной точке мира за O(1)
    #[inline(always)]
    pub fn sample_gradient(&self, pos: Vec2) -> Vec2 {
        if let Some((cx, cy)) = self.cell_coords(pos) {
            let idx = cy * self.cols + cx;
            Vec2::new(self.grad_x[idx], self.grad_y[idx])
        } else {
            Vec2::ZERO
        }
    }

    /// Локальная плотность в точке
    #[inline(always)]
    pub fn sample_density(&self, pos: Vec2) -> f32 {
        if let Some((cx, cy)) = self.cell_coords(pos) {
            let idx = cy * self.cols + cx;
            self.density[idx]
        } else {
            0.0
        }
    }

    /// Расчет объединенного вектора движения: Вектор Цели (Macro) + Давление толпы (Repulsion)
    #[inline(always)]
    pub fn compute_steering(
        &self,
        mob_pos: Vec2,
        goal_dir: Vec2,
        mob_speed: f32,
    ) -> Vec2 {
        let grad = self.sample_gradient(mob_pos);
        let grad_len_sq = grad.length_squared();

        // Если в ячейке есть градиент толпы — выдавливаем моба в сторону меньшей плотности
        if grad_len_sq > 0.005 {
            let repel_dir = -grad.normalize_or_zero();
            let combined = goal_dir + repel_dir * CROWD_PRESSURE_WEIGHT;
            combined.normalize_or_zero() * mob_speed
        } else {
            goal_dir * mob_speed
        }
    }

    /// Микро-отталкивание (Micro-Separation) от ближайшего соседа в той же ячейке.
    /// Ищет минимальное расстояние среди кандидатов ячейки и отталкивает строго от ближайшего.
    #[inline(always)]
    pub fn micro_separate_same_cell(
        mob_idx: usize,
        mob_pos: &mut Vec2,
        all_positions: &[Vec2],
        grid: &SpatialGrid,
        min_dist: f32,
    ) {
        if let Some((cx, cy)) = grid.get_cell_coords(*mob_pos) {
            let min_dist_sq = min_dist * min_dist;
            let mut closest_diff = Vec2::ZERO;
            let mut best_dist_sq = min_dist_sq;
            let mut found = false;
            let mut checks = 0;

            grid.for_each_in_cell(cx, cy, |other_idx| {
                if other_idx != mob_idx {
                    if let Some(&other_pos) = all_positions.get(other_idx) {
                        let diff = *mob_pos - other_pos;
                        let dist_sq = diff.length_squared();
                        if dist_sq < best_dist_sq && dist_sq > 0.0001 {
                            best_dist_sq = dist_sq;
                            closest_diff = diff;
                            found = true;
                        }
                    }
                    checks += 1;
                }
                checks < 6 // Сканируем до 6 кандидатов ячейки для надежного нахождения ближайшего
            });

            if found {
                let dist = best_dist_sq.sqrt();
                let push = (closest_diff / dist) * (min_dist - dist) * 0.5;
                *mob_pos += push;
            }
        }
    }
}

/// Structure of Arrays (SoA) для координат и скоростей мобов.
/// Позволяет обрабатывать 20 000 позиций непрерывным потоком в L2-кэше (160 КБ вместо 2.4 МБ).
#[derive(Default, Clone, Debug)]
pub struct MobMotionSoA {
    pub pos: Vec<Vec2>,
    pub vel: Vec<Vec2>,
    pub speed: Vec<f32>,
    pub target: Vec<Vec2>,
}

impl MobMotionSoA {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            pos: Vec::with_capacity(cap),
            vel: Vec::with_capacity(cap),
            speed: Vec::with_capacity(cap),
            target: Vec::with_capacity(cap),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.pos.clear();
        self.vel.clear();
        self.speed.clear();
        self.target.clear();
    }

    #[inline]
    pub fn push(&mut self, pos: Vec2, vel: Vec2, speed: f32, target: Vec2) {
        self.pos.push(pos);
        self.vel.push(vel);
        self.speed.push(speed);
        self.target.push(target);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_continuum_crowds_density_and_gradient() {
        let mut grid = CrowdDensityGrid::new(25920.0, 17280.0);
        let center = Vec2::new(10000.0, 8000.0);

        // Создаем плотный кластер из 10 мобов в одной ячейке
        let mut cluster = Vec::new();
        for _ in 0..10 {
            cluster.push(center + Vec2::new(2.0, 2.0));
        }

        grid.splat_positions(&cluster);
        grid.compute_blurred_gradients();

        let density_at_center = grid.sample_density(center);
        assert!(density_at_center >= 10.0, "Плотность должна быть >= 10");

        // Проверяем вектор движения в СОСЕДНЕЙ ячейке (где градиент строго не равен 0 из-за падения плотности)
        let mob_pos = center + Vec2::new(CELL_SIZE, 0.0);
        let grad = grid.sample_gradient(mob_pos);
        assert!(grad.x < -0.1, "Градиент плотности справа от кластера обязан быть отрицательным! grad: {:?}", grad);

        // Цель направлена строго вниз (y = 1, x = 0).
        // Из-за отталкивания от кластера слева итоговый вектор ОБЯЗАН отклониться вправо (x > 0)!
        let goal_dir = Vec2::new(0.0, 1.0);
        let steering = grid.compute_steering(mob_pos, goal_dir, 100.0);

        assert!(steering.x > 5.0, "Вектор движения обязан отклоняться вправо от кластера! steering: {:?}", steering);
        assert!(steering.y > 0.0, "Составляющая цели должна сохраняться");
        assert!((steering.length() - 100.0).abs() < 1e-3);
    }

    #[test]
    fn test_crowd_anti_stacking_under_high_density() {
        let mut grid = SpatialGrid::new(25920.0, 17280.0);
        let center = Vec2::new(10000.0, 8000.0);

        // Спавним 10 мобов почти в одной точке
        let mut positions = vec![
            center + Vec2::new(0.0, 0.0),
            center + Vec2::new(1.0, 0.5),
            center + Vec2::new(-0.5, 1.0),
            center + Vec2::new(1.5, -0.5),
            center + Vec2::new(-1.0, -1.0),
        ];

        for (i, &p) in positions.iter().enumerate() {
            grid.insert(i, p);
        }

        let initial_dist = positions[0].distance(positions[1]);
        assert!(initial_dist < 2.0);

        // Прогоняем микро-отталкивание
        let old_pos = positions.clone();
        for (i, pos) in positions.iter_mut().enumerate() {
            CrowdDensityGrid::micro_separate_same_cell(i, pos, &old_pos, &grid, 20.0);
        }

        let new_dist = positions[0].distance(positions[1]);
        assert!(new_dist > initial_dist, "Мобы должны оттолкнуться друг от друга!");
    }

    #[test]
    fn test_halo_cleanup_no_ghost_data_between_frames() {
        let mut grid = CrowdDensityGrid::new(25920.0, 17280.0);
        let pos_frame1 = Vec2::new(10000.0, 8000.0);

        // Кадр 1: мобы в точке 1
        grid.splat_positions(&[pos_frame1; 5]);
        grid.compute_blurred_gradients();

        assert!(grid.sample_density(pos_frame1) > 0.0);

        // Кадр 2: очищаем и мобы переместились в точку 2
        grid.clear();
        assert_eq!(grid.sample_density(pos_frame1), 0.0);
        let grad = grid.sample_gradient(pos_frame1);
        assert_eq!(grad, Vec2::ZERO, "Градиент в старой точке после clear обязан быть строго ZERO!");

        let pos_frame2 = Vec2::new(12000.0, 9000.0);
        grid.splat_positions(&[pos_frame2; 5]);
        grid.compute_blurred_gradients();

        // Проверяем, что в точке 1 не появилось никакого призрачного градиента
        assert_eq!(grid.sample_gradient(pos_frame1), Vec2::ZERO);
        assert!(grid.sample_density(pos_frame2) > 0.0);
    }

    #[test]
    fn test_sat_exact_rect_sum_and_peak_cluster_detection() {
        let mut grid = CrowdDensityGrid::new(25920.0, 17280.0);
        let boss_pos = Vec2::new(10000.0, 8000.0);
        let small_cluster = Vec2::new(9800.0, 8000.0); // 3 моба
        let big_cluster = Vec2::new(10280.0, 8150.0);  // 25 мобов

        let mut mobs = vec![small_cluster; 3];
        mobs.extend(vec![big_cluster; 25]);

        grid.splat_positions(&mobs);
        grid.compute_blurred_gradients();

        let (bx, by) = grid.cell_coords(big_cluster).unwrap();
        let (sx, sy) = grid.cell_coords(small_cluster).unwrap();
        assert!((grid.query_sat_rect(bx, by, bx, by) - 25.0).abs() < 1e-4);
        assert!((grid.query_sat_rect(sx, sy, sx, sy) - 3.0).abs() < 1e-4);

        // Общий прямоугольник, охватывающий оба кластера, обязан дать ровно 28
        let sum_all = grid.query_sat_rect(sx.min(bx), sy.min(by), sx.max(bx), sy.max(by));
        assert!((sum_all - 28.0).abs() < 1e-4);

        let peak = grid.find_peak_density_cell(boss_pos, 450.0, 130.0).expect("Peak must be found");
        assert!(peak.0.distance(big_cluster) < CELL_SIZE, "Peak must lock onto the 25-mob cluster, got {:?}", peak);
    }
}
