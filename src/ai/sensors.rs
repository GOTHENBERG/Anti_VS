use macroquad::prelude::*;
use crate::ecs::world::World;
use crate::ecs::components::combat::Faction;
use crate::core::{MAP_WIDTH, MAP_HEIGHT};
use crate::ai::neural_net::INPUT_SIZE;
use crate::ai::pathing::WaypointPlanner;

/// Извлекает 150-мерный вектор сенсорного восприятия Босса из игрового мира.
/// Архитектура восприятия (Двухслойная эго-центрическая сетка "Матрешка"):
/// 1. [0..120] Внутренняя 2D-сетка 11x11 (ячейки по 55 px, радиус 302.5 px, охват 605x605 px):
///    - Плотность ближней толпы мобов (0.0..1.0)
///    - Опасность летящих снарядов Директора (снаряды фракции Director с весом урона)
///    - Опасность Чумных Спор Директора (лужи яда acid_pools на земле)
///    - Центр [5, 5] (позиция самого Босса) исключен -> 120 ячеек.
/// 2. [120..136] Внешний макро-пояс (16 радиальных секторов, радиус 302.5..550.0 px):
///    - Плотность дальнобойщиков (Тир 3, 11 и 16) без шумящих таймеров.
/// 3. [136..150] Стратегические сенсоры (14 входов):
///    - Координаты, вектор, % ХП и расчетное время прибытия до ближайшего Алтаря Крови
///    - Флаг активности, вектор и время прибытия до супер-кристалла "Магнит"
///    - Внутренний статус: % HP и готовность рывка (Dash)
///    - Статус медитации: прогресс таймера покоя (0.0..1.0)
///    - Текущая скорость движения Босса (0.0..1.5)
pub fn extract_boss_observations(world: &World) -> [f32; INPUT_SIZE] {
    let mut obs = [0.0f32; INPUT_SIZE];
    let boss_pos = world.boss_pos;

    // ------------------------------------------------------------------------------------
    // 1. [0..120] Внутренняя 2D-сетка 11x11 (55 px на ячейку, охват 605x605 px)
    // ------------------------------------------------------------------------------------
    const CELL_SIZE: f32 = 55.0;
    const GRID_SIZE: usize = 11;
    const HALF_GRID_PX: f32 = 5.5 * CELL_SIZE; // 302.5 px

    let mut mob_grid = [[0.0f32; GRID_SIZE]; GRID_SIZE];
    let mut proj_grid = [[0.0f32; GRID_SIZE]; GRID_SIZE];
    let mut spore_grid = [[0.0f32; GRID_SIZE]; GRID_SIZE];

    // А. Опрос мобов в радиусе 305 px через Spatial Grid
    let nearby_mobs = world.spatial_grid.query_nearby(boss_pos, HALF_GRID_PX + 5.0);
    for idx in nearby_mobs {
        if let Some(mob) = world.mobs.get(idx) {
            let dx = mob.pos.x - boss_pos.x;
            let dy = mob.pos.y - boss_pos.y;
            if dx.abs() < HALF_GRID_PX && dy.abs() < HALF_GRID_PX {
                let col = ((dx + HALF_GRID_PX) / CELL_SIZE).floor() as isize;
                let row = ((dy + HALF_GRID_PX) / CELL_SIZE).floor() as isize;
                if row >= 0 && row < GRID_SIZE as isize && col >= 0 && col < GRID_SIZE as isize {
                    let weight = if mob.is_elite { 2.5 } else { 1.0 };
                    mob_grid[row as usize][col as usize] += weight;
                }
            }
        }
    }

    // Б. Опрос летящих снарядов Директора (вражеские пули и стрелы)
    for proj in &world.projectiles {
        if proj.faction == Faction::Director {
            let dx = proj.pos.x - boss_pos.x;
            let dy = proj.pos.y - boss_pos.y;
            if dx.abs() < HALF_GRID_PX && dy.abs() < HALF_GRID_PX {
                let col = ((dx + HALF_GRID_PX) / CELL_SIZE).floor() as isize;
                let row = ((dy + HALF_GRID_PX) / CELL_SIZE).floor() as isize;
                if row >= 0 && row < GRID_SIZE as isize && col >= 0 && col < GRID_SIZE as isize {
                    let danger = (proj.damage / 80.0).clamp(0.2, 2.0);
                    proj_grid[row as usize][col as usize] += danger;
                }
            }
        }
    }

    // В. Опрос Чумных Спор Директора (защитные миазмы вокруг Алтарей Крови)
    let pestilence_lvl = world.directors.first().map_or(0, |d| d.grade_pestilence);
    if pestilence_lvl > 0 {
        let miasma_radius = 160.0f32;
        let miasma_dps = 10.0 + (pestilence_lvl as f32) * 2.5;
        for st in &world.structures {
            let dx = st.pos.x - boss_pos.x;
            let dy = st.pos.y - boss_pos.y;
            if dx.abs() < HALF_GRID_PX + miasma_radius && dy.abs() < HALF_GRID_PX + miasma_radius {
                for r in 0..GRID_SIZE {
                    let cell_y = -HALF_GRID_PX + (r as f32 + 0.5) * CELL_SIZE;
                    for c in 0..GRID_SIZE {
                        let cell_x = -HALF_GRID_PX + (c as f32 + 0.5) * CELL_SIZE;
                        let dist_sq = (cell_x - dx) * (cell_x - dx) + (cell_y - dy) * (cell_y - dy);
                        if dist_sq < (miasma_radius + CELL_SIZE * 0.4) * (miasma_radius + CELL_SIZE * 0.4) {
                            let hazard = (miasma_dps / 15.0).clamp(0.4, 1.2);
                            spore_grid[r][c] += hazard;
                        }
                    }
                }
            }
        }
    }

    // Запись 120 ячеек (пропуская центр [5, 5])
    let mut out_idx = 0;
    for r in 0..GRID_SIZE {
        for c in 0..GRID_SIZE {
            if r == 5 && c == 5 {
                continue; // Позиция самого Босса
            }
            let mob_val = (mob_grid[r][c] / 5.0).clamp(0.0, 1.0);
            let proj_val = proj_grid[r][c].clamp(0.0, 1.0);
            let spore_val = spore_grid[r][c].clamp(0.0, 1.0);
            // Комплексная опасность ячейки: сочетание толпы, летящих пуль и чумных спор
            obs[out_idx] = (mob_val * 0.35 + proj_val * 0.70 + spore_val * 0.60).clamp(0.0, 1.0);
            out_idx += 1;
        }
    }
    debug_assert_eq!(out_idx, 120);

    // ------------------------------------------------------------------------------------
    // 2. [120..136] Внешний макро-пояс снайперов (16 радиальных секторов, 302.5 .. 550.0 px)
    // ------------------------------------------------------------------------------------
    let mut outer_snipers = [0.0f32; 16];
    let far_mobs = world.spatial_grid.query_nearby(boss_pos, 550.0);
    for idx in far_mobs {
        if let Some(mob) = world.mobs.get(idx) {
            // Учитываем только дальнобойных мобов (Тир 3 - культист, 11 - маг, 16 - палач-снайпер)
            if mob.tier == 3 || mob.tier == 11 || mob.tier == 16 {
                let diff = mob.pos - boss_pos;
                let dist = diff.length();
                if dist >= HALF_GRID_PX && dist <= 550.0 {
                    let angle = diff.y.atan2(diff.x);
                    let norm_angle = if angle < 0.0 { angle + std::f32::consts::PI * 2.0 } else { angle };
                    let sector = ((norm_angle / (std::f32::consts::PI * 2.0)) * 16.0) as usize % 16;
                    let weight = if mob.tier == 16 { 2.0 } else { 1.0 };
                    outer_snipers[sector] += weight;
                }
            }
        }
    }

    for i in 0..16 {
        obs[120 + i] = (outer_snipers[i] / 4.0).clamp(0.0, 1.0);
    }

    // ------------------------------------------------------------------------------------
    // 3. [136..150] Стратегические сенсоры (14 входов)
    // ------------------------------------------------------------------------------------
    let nav = WaypointPlanner::calculate_strategic_navigation(world);

    // Алтарь Крови [136..142]
    if let Some(altar_pos) = nav.closest_altar_pos {
        obs[136] = (altar_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
        obs[137] = (altar_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);

        let diff = altar_pos - boss_pos;
        obs[138] = (diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
        obs[139] = (diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);

        // Поиск % ХП этого алтаря
        let mut altar_hp = 1.0f32;
        for s in &world.structures {
            if s.pos.distance_squared(altar_pos) < 25.0 {
                altar_hp = (s.health.current / s.health.max).clamp(0.0, 1.0);
                break;
            }
        }
        obs[140] = altar_hp;
    } else {
        obs[136] = 0.5;
        obs[137] = 0.5;
        obs[138] = 0.0;
        obs[139] = 0.0;
        obs[140] = 0.0;
    }
    obs[141] = (nav.time_to_closest_altar / 20.0).clamp(0.0, 1.0);

    // Магнит [142..146]
    obs[142] = if nav.has_active_magnet { 1.0 } else { 0.0 };
    if let Some(m_pos) = nav.closest_magnet_pos {
        let m_diff = m_pos - boss_pos;
        obs[143] = (m_diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
        obs[144] = (m_diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);
        obs[145] = (nav.time_to_closest_magnet / 20.0).clamp(0.0, 1.0);
    } else {
        obs[143] = 0.0;
        obs[144] = 0.0;
        obs[145] = 1.0;
    }

    // Статус Босса [146..148]
    obs[146] = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0);
    obs[147] = if world.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 };

    // Сенсор медитации и скорости [148..150]
    obs[148] = (world.boss_still_time / 1.2).clamp(0.0, 1.0); // 0.0 = в движении, 1.0 = активная медитация
    obs[149] = (world.boss_vel.length() / 160.0).clamp(0.0, 1.5); // Нормализованная скорость

    obs
}
