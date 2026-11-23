use macroquad::prelude::Vec2;
use crate::ecs::world::StructureEntity;

pub const FLOW_GRID_N: usize = 100;
pub const FLOW_CELL_SIZE: f32 = 30.0;
pub const FLOW_GRID_CELLS: usize = FLOW_GRID_N * FLOW_GRID_N;

const PREBAKED_V_BYTES: &[u8] = include_bytes!("../../assets/flow/v_raw.bin");

/// Динамическое векторное поле (Flow Field) на основе уравнения Пуассона Delta u = -1
/// с условиями Неймана на препятствиях и внешних границах.
#[derive(Clone)]
pub struct FlowField {
    pub n: usize,
    pub cell_size: f32,
    pub boss_cell: (usize, usize),
    pub obstacles: Vec<bool>, // [j * 100 + i], true = непроходимо
    pub u: Vec<f32>,          // [j * 100 + i], скалярный потенциал
    pub v: Vec<Vec2>,         // [j * 100 + i], нормализованный вектор направления к боссу
}

impl Default for FlowField {
    fn default() -> Self {
        Self::load_prebaked()
    }
}

impl FlowField {
    /// Мгновенная загрузка эталонного предрасчитанного поля (0 мс, 0 аллокаций при старте)
    pub fn load_prebaked() -> Self {
        let n = FLOW_GRID_N;
        let mut v = Vec::with_capacity(FLOW_GRID_CELLS);
        for chunk in PREBAKED_V_BYTES.chunks_exact(8) {
            let vx = f32::from_le_bytes(chunk[0..4].try_into().unwrap());
            let vy = f32::from_le_bytes(chunk[4..8].try_into().unwrap());
            v.push(Vec2::new(vx, vy));
        }

        // Базовая маска препятствий арены
        let mut obstacles = vec![false; FLOW_GRID_CELLS];
        // 4 Обелиска 11x11
        for rx in [4..15, 85..96] {
            for ry in [4..15, 85..96] {
                for j in ry.clone() {
                    for i in rx.clone() {
                        obstacles[j * n + i] = true;
                    }
                }
            }
        }
        // Алтарь 4x4
        for j in 50..54 {
            for i in 40..44 {
                obstacles[j * n + i] = true;
            }
        }
        // Баррикады
        for j in 35..=65 {
            obstacles[j * n + 45] = true;
        }
        for j in 40..=60 {
            obstacles[j * n + 58] = true;
        }
        for i in 50..=58 {
            obstacles[60 * n + i] = true;
        }

        let u = vec![0.0; FLOW_GRID_CELLS];

        Self {
            n,
            cell_size: FLOW_CELL_SIZE,
            boss_cell: (50, 50),
            obstacles,
            u,
            v,
        }
    }

    /// Строит маску препятствий сетки напрямую из игровых структур (обелиски, алтари, стены)
    pub fn build_obstacle_mask(structures: &[StructureEntity]) -> Vec<bool> {
        let mut obstacles = vec![false; FLOW_GRID_CELLS];
        for st in structures {
            if !st.health.is_alive() {
                continue;
            }
            let half = match st.structure_type {
                crate::content::structures::STRUCTURE_OBELISK => {
                    crate::content::structures::OBELISK_FOOTPRINT_HALF_EXTENT
                }
                crate::content::structures::STRUCTURE_WALL => {
                    crate::content::structures::WALL_RADIUS
                }
                _ => {
                    crate::content::structures::ALTAR_HALF_EXTENT
                }
            };
            let min_i = (((st.pos.x - half) / FLOW_CELL_SIZE).floor() as isize).clamp(0, (FLOW_GRID_N - 1) as isize) as usize;
            let max_i = (((st.pos.x + half) / FLOW_CELL_SIZE).floor() as isize).clamp(0, (FLOW_GRID_N - 1) as isize) as usize;
            let min_j = (((st.pos.y - half) / FLOW_CELL_SIZE).floor() as isize).clamp(0, (FLOW_GRID_N - 1) as isize) as usize;
            let max_j = (((st.pos.y + half) / FLOW_CELL_SIZE).floor() as isize).clamp(0, (FLOW_GRID_N - 1) as isize) as usize;

            for j in min_j..=max_j {
                for i in min_i..=max_i {
                    obstacles[j * FLOW_GRID_N + i] = true;
                }
            }
        }
        obstacles
    }

    /// Быстрая билинейная интерполяция вектора скорости в мировой точке (x, y)
    #[inline(always)]
    pub fn sample_vector(&self, x: f32, y: f32) -> Vec2 {
        let inv_h = 1.0 / self.cell_size;
        let gx = (x * inv_h).clamp(0.0, (self.n - 1) as f32);
        let gy = (y * inv_h).clamp(0.0, (self.n - 1) as f32);

        let i0 = (gx.floor() as usize).min(self.n - 1);
        let j0 = (gy.floor() as usize).min(self.n - 1);
        let i1 = (i0 + 1).min(self.n - 1);
        let j1 = (j0 + 1).min(self.n - 1);

        let fx = gx - i0 as f32;
        let fy = gy - j0 as f32;

        let idx00 = j0 * self.n + i0;
        let idx10 = j0 * self.n + i1;
        let idx01 = j1 * self.n + i0;
        let idx11 = j1 * self.n + i1;

        if self.obstacles[idx00] && self.obstacles[idx10] && self.obstacles[idx01] && self.obstacles[idx11] {
            return Vec2::ZERO;
        }

        let v00 = self.v[idx00];
        let v10 = self.v[idx10];
        let v01 = self.v[idx01];
        let v11 = self.v[idx11];

        // Билинейная интерполяция между 4 узлами
        let top = v00 * (1.0 - fx) + v10 * fx;
        let bot = v01 * (1.0 - fx) + v11 * fx;
        let interpolated = top * (1.0 - fy) + bot * fy;

        if interpolated.length_squared() > 1e-6 {
            interpolated.normalize()
        } else {
            v00
        }
    }

    /// Сэмплирует вектор движения моба с мягким переходом (lerp) к прямому вектору босса
    /// при входе в радиус ближнего боя (< 140 px).
    #[inline(always)]
    pub fn sample_mob_direction(&self, mob_pos: Vec2, target_pos: Vec2) -> Vec2 {
        let flow_dir = self.sample_vector(mob_pos.x, mob_pos.y);
        let direct_diff = target_pos - mob_pos;
        let direct_dist = direct_diff.length();

        if direct_dist < 100.0 || flow_dir.length_squared() < 0.01 {
            direct_diff.normalize_or_zero()
        } else if direct_dist < 160.0 {
            let blend = ((direct_dist - 100.0) / 60.0).clamp(0.0, 1.0);
            let direct_dir = direct_diff.normalize_or_zero();
            (flow_dir * blend + direct_dir * (1.0 - blend)).normalize_or_zero()
        } else {
            flow_dir
        }
    }

    /// Чистый нативный Conjugate Gradient решатель дискретного уравнения Пуассона:
    /// Delta u = -1, u(boss) = 0, Neumann du/dn = 0 на стенах и границах.
    pub fn solve(
        obstacles: &[bool],
        boss_i: usize,
        boss_j: usize,
        max_iters: usize,
        tolerance: f32,
    ) -> Self {
        let n = FLOW_GRID_N;
        let h = FLOW_CELL_SIZE;
        let mut idx = vec![-1i32; FLOW_GRID_CELLS];
        let mut m = 0usize;

        for j in 0..n {
            for i in 0..n {
                let k = j * n + i;
                if !obstacles[k] && !(i == boss_i && j == boss_j) {
                    idx[k] = m as i32;
                    m += 1;
                }
            }
        }

        let mut diag = vec![0.0f64; m];
        let mut src = Vec::with_capacity(m * 4);
        let mut dst = Vec::with_capacity(m * 4);

        let neighbors = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        for j in 0..n {
            for i in 0..n {
                let cell = j * n + i;
                let k = idx[cell];
                if k < 0 {
                    continue;
                }
                let k_usize = k as usize;
                let mut nfree = 0.0f64;

                for (di, dj) in neighbors {
                    let ni = i as isize + di;
                    let nj = j as isize + dj;
                    if ni < 0 || ni >= n as isize || nj < 0 || nj >= n as isize {
                        continue;
                    }
                    let n_cell = nj as usize * n + ni as usize;
                    if obstacles[n_cell] {
                        continue;
                    }
                    nfree += 1.0;
                    if !(ni as usize == boss_i && nj as usize == boss_j) {
                        src.push(k_usize);
                        dst.push(idx[n_cell] as usize);
                    }
                }
                diag[k_usize] = nfree;
            }
        }

        // Sparse matrix-vector product
        let mv = |x: &[f64], y: &mut [f64]| {
            for i in 0..m {
                y[i] = diag[i] * x[i];
            }
            for e in 0..src.len() {
                y[src[e]] -= x[dst[e]];
            }
        };

        let rhs_val = (h * h) as f64;
        let mut x = vec![0.0f64; m];
        let mut r = vec![rhs_val; m];
        let mut p = r.clone();
        let mut ap = vec![0.0f64; m];

        let mut rsold: f64 = r.iter().map(|&val| val * val).sum();
        let tol_sq = (tolerance as f64) * (tolerance as f64);

        for _it in 0..max_iters {
            mv(&p, &mut ap);
            let denom: f64 = p.iter().zip(ap.iter()).map(|(&pi, &api)| pi * api).sum();
            if denom.abs() < 1e-15 {
                break;
            }
            let alpha = rsold / denom;
            for i in 0..m {
                x[i] += alpha * p[i];
                r[i] -= alpha * ap[i];
            }
            let rsnew: f64 = r.iter().map(|&val| val * val).sum();
            if rsnew < tol_sq {
                break;
            }
            let beta = rsnew / rsold;
            for i in 0..m {
                p[i] = r[i] + beta * p[i];
            }
            rsold = rsnew;
        }

        // Восстановление потенциала U
        let mut u = vec![1e9f32; FLOW_GRID_CELLS];
        u[boss_j * n + boss_i] = 0.0;
        for j in 0..n {
            for i in 0..n {
                let cell = j * n + i;
                let k = idx[cell];
                if k >= 0 {
                    u[cell] = x[k as usize] as f32;
                }
            }
        }

        // Вычисление градиентов Gx, Gy и нормализованного поля V = -grad U / |grad U|
        let mut v = vec![Vec2::ZERO; FLOW_GRID_CELLS];
        for j in 0..n {
            for i in 0..n {
                let cell = j * n + i;
                if obstacles[cell] || (i == boss_i && j == boss_j) {
                    continue;
                }

                // X-градиент (односторонняя разность на краях препятствий)
                let gx = if i == 0 {
                    let r_cell = j * n + (i + 1);
                    if !obstacles[r_cell] { (u[r_cell] - u[cell]) / h } else { 0.0 }
                } else if i == n - 1 {
                    let l_cell = j * n + (i - 1);
                    if !obstacles[l_cell] { (u[cell] - u[l_cell]) / h } else { 0.0 }
                } else {
                    let l_cell = j * n + (i - 1);
                    let r_cell = j * n + (i + 1);
                    let left = if !obstacles[l_cell] { u[l_cell] } else { u[cell] };
                    let right = if !obstacles[r_cell] { u[r_cell] } else { u[cell] };
                    (right - left) / (2.0 * h)
                };

                // Y-градиент (Y вниз)
                let gy = if j == 0 {
                    let d_cell = (j + 1) * n + i;
                    if !obstacles[d_cell] { (u[d_cell] - u[cell]) / h } else { 0.0 }
                } else if j == n - 1 {
                    let u_cell = (j - 1) * n + i;
                    if !obstacles[u_cell] { (u[cell] - u[u_cell]) / h } else { 0.0 }
                } else {
                    let u_cell = (j - 1) * n + i;
                    let d_cell = (j + 1) * n + i;
                    let up = if !obstacles[u_cell] { u[u_cell] } else { u[cell] };
                    let down = if !obstacles[d_cell] { u[d_cell] } else { u[cell] };
                    (down - up) / (2.0 * h)
                };

                let norm = (gx * gx + gy * gy).sqrt() + 1e-12;
                v[cell] = Vec2::new(-gx / norm, -gy / norm);
            }
        }

        Self {
            n,
            cell_size: h,
            boss_cell: (boss_i, boss_j),
            obstacles: obstacles.to_vec(),
            u,
            v,
        }
    }

    /// Пересчитывает поле под динамический мир игры (структуры и положение босса)
    pub fn recompute_for_world(&mut self, boss_pos: Vec2, structures: &[StructureEntity]) {
        let bi = ((boss_pos.x / FLOW_CELL_SIZE).floor() as isize).clamp(0, (FLOW_GRID_N - 1) as isize) as usize;
        let bj = ((boss_pos.y / FLOW_CELL_SIZE).floor() as isize).clamp(0, (FLOW_GRID_N - 1) as isize) as usize;
        let mut obs = Self::build_obstacle_mask(structures);
        // Защита: босс не должен быть заблокирован стеной
        obs[bj * FLOW_GRID_N + bi] = false;
        *self = Self::solve(&obs, bi, bj, 700, 1e-4);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prebaked_loading_and_sampling() {
        let flow = FlowField::load_prebaked();
        assert_eq!(flow.v.len(), 10000);
        assert_eq!(flow.boss_cell, (50, 50));

        let v_left = flow.sample_vector(30.0 * 30.0, 50.0 * 30.0);
        assert!(v_left.x > 0.0, "Моб слева от босса должен бежать на восток!");

        let v_right = flow.sample_vector(70.0 * 30.0, 50.0 * 30.0);
        assert!(v_right.x < 0.0, "Моб справа от босса должен бежать на запад!");
    }

    #[test]
    fn test_native_cg_solver_matches_spec() {
        let prebaked = FlowField::load_prebaked();
        let solved = FlowField::solve(&prebaked.obstacles, 50, 50, 800, 1e-4);

        let trap_idx = 55 * 100 + 55;
        let diff_u = (solved.u[trap_idx] - 4343125.0).abs();
        assert!(diff_u < 50000.0, "Потенциал в ловушке U(55,55) должен быть ~4.34e6");
    }
}
