use macroquad::prelude::*;
use crate::ecs::world::World;
use crate::core::{MAP_WIDTH, MAP_HEIGHT};
use crate::ai::neural_net::INPUT_SIZE;
use crate::ai::pathing::WaypointPlanner;

/// Извлекает 42-мерный вектор сенсорного восприятия Босса из игрового мира.
/// Сенсоры включают:
/// - 360-градусный 16-лучевой лидар дальности до врагов
/// - 8-секторный радар плотности массы толпы
/// - Дистанции до границ арены
/// - Вектор на ближайший кристалл душ
/// - Точные нормализованные мировые координаты ближайшего Алтаря Крови
/// - Вектор относительного смещения от Босса к Алтарю Крови (без искусственного ограничения радиуса)
/// - Кинематическое расчетное время прибытия от позиции Босса до Алтаря A (в секундах)
/// - Кинематическое расчетное время перехода по цепочке от Точки Y (Алтарь 1) до Точки Z (Алтарь 2 или скопление опыта)
/// - Детектор снайперов-культистов (Тир 3 и Тир 11)
/// - Внутренний статус: здоровье, готовность рывка, скорость бега и общее число активных алтарей
pub fn extract_boss_observations(world: &World) -> [f32; INPUT_SIZE] {
    let mut obs = [0.0f32; INPUT_SIZE];
    let boss_pos = world.boss_pos;

    // 1. [0..16] 16-лучевой лидар дальности до ближайших мобов (сектора по 22.5 градуса)
    // 1.0 = свободно, 0.0 = моб вплотную к боссу
    let max_lidar_dist = 240.0f32;
    let mut min_sector_dist = [max_lidar_dist; 16];

    let nearby_mobs = world.spatial_grid.query_nearby(boss_pos, max_lidar_dist);
    for idx in nearby_mobs {
        if let Some(mob) = world.mobs.get(idx) {
            let diff = mob.pos - boss_pos;
            let dist = diff.length();
            if dist < max_lidar_dist && dist > 0.001 {
                let angle = diff.y.atan2(diff.x);
                let norm_angle = if angle < 0.0 { angle + std::f32::consts::PI * 2.0 } else { angle };
                let sector = ((norm_angle / (std::f32::consts::PI * 2.0)) * 16.0) as usize % 16;
                if dist < min_sector_dist[sector] {
                    min_sector_dist[sector] = dist;
                }
            }
        }
    }

    for i in 0..16 {
        obs[i] = min_sector_dist[i] / max_lidar_dist;
    }

    // 2. [16..24] 8-лучевой радар плотности массы толпы (сектора по 45 градусов)
    // Нормализовано от 0.0 (пусто) до 1.0 (плотная стена из 25+ мобов)
    let max_density_dist = 280.0f32;
    let mut sector_crowd_count = [0.0f32; 8];
    for idx in &world.spatial_grid.query_nearby(boss_pos, max_density_dist) {
        if let Some(mob) = world.mobs.get(*idx) {
            let diff = mob.pos - boss_pos;
            let dist = diff.length();
            if dist < max_density_dist && dist > 0.001 {
                let angle = diff.y.atan2(diff.x);
                let norm_angle = if angle < 0.0 { angle + std::f32::consts::PI * 2.0 } else { angle };
                let sector = ((norm_angle / (std::f32::consts::PI * 2.0)) * 8.0) as usize % 8;
                sector_crowd_count[sector] += 1.0;
            }
        }
    }
    for i in 0..8 {
        obs[16 + i] = (sector_crowd_count[i] / 25.0).clamp(0.0, 1.0);
    }

    // 3. [24..28] Сенсоры стен арены (Left, Right, Top, Bottom)
    let wall_danger_dist = 180.0f32;
    obs[24] = (boss_pos.x / wall_danger_dist).clamp(0.0, 1.0);
    obs[25] = ((MAP_WIDTH - boss_pos.x) / wall_danger_dist).clamp(0.0, 1.0);
    obs[26] = (boss_pos.y / wall_danger_dist).clamp(0.0, 1.0);
    obs[27] = ((MAP_HEIGHT - boss_pos.y) / wall_danger_dist).clamp(0.0, 1.0);

    // 4. [28..30] Вектор к ближайшему кристаллу душ
    let mut closest_gem_dist = 350.0f32;
    let mut gem_vec = Vec2::ZERO;
    for gem in &world.gems {
        let dist = gem.pos.distance(boss_pos);
        if dist < closest_gem_dist {
            closest_gem_dist = dist;
            gem_vec = (gem.pos - boss_pos) / 350.0;
        }
    }
    obs[28] = gem_vec.x;
    obs[29] = gem_vec.y;

    // 5. [30..36] Точные координаты алтарей, вектор направления и кинематические расчеты пути
    let nav = WaypointPlanner::calculate_strategic_navigation(world);
    if let Some(altar_pos) = nav.closest_altar_pos {
        // [30..32] Точные глобальные нормализованные координаты алтаря (X, Y) на карте
        obs[30] = (altar_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
        obs[31] = (altar_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);

        // [32..34] Относительный вектор смещения от Босса к Алтарю (dx, dy)
        let diff = altar_pos - boss_pos;
        obs[32] = (diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
        obs[33] = (diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);
    } else {
        obs[30] = 0.5;
        obs[31] = 0.5;
        obs[32] = 0.0;
        obs[33] = 0.0;
    }

    // [34] Расчетное время (в секундах) прибытия от текущей точки до Алтаря A (нормировано до 20 сек)
    obs[34] = (nav.time_to_closest_altar / 20.0).clamp(0.0, 1.0);

    // [35] Расчетное время (в секундах) перехода по цепочке: от Точки Y (Алтарь 1) до Точки Z (Алтарь 2 или Опыт)
    obs[35] = (nav.time_chained_y_to_z / 20.0).clamp(0.0, 1.0);

    // 6. [36..38] Вектор к ближайшему кластеру снайперов-стрелков (Тир 3 и Тир 11)
    let mut closest_sniper_dist = 450.0f32;
    let mut sniper_vec = Vec2::ZERO;
    for idx in &world.spatial_grid.query_nearby(boss_pos, 450.0) {
        if let Some(mob) = world.mobs.get(*idx) {
            if mob.tier == 3 || mob.tier == 11 {
                let dist = mob.pos.distance(boss_pos);
                if dist < closest_sniper_dist {
                    closest_sniper_dist = dist;
                    sniper_vec = (mob.pos - boss_pos) / 450.0;
                }
            }
        }
    }
    obs[36] = sniper_vec.x;
    obs[37] = sniper_vec.y;

    // 7. [38..42] Внутреннее состояние Босса
    obs[38] = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0); // % HP
    obs[39] = if world.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 }; // Готовность рывка
    obs[40] = (world.boss_inventory.get_movement_speed(160.0) / 160.0).clamp(0.5, 2.5); // Текущий множитель скорости
    obs[41] = (nav.altar_count as f32 / 5.0).clamp(0.0, 1.0); // Число активных алтарей

    // 8. [42..48] Точный радар супер-предметов "Магнит" (Super-Gem Vacuum)
    obs[42] = if nav.has_active_magnet { 1.0 } else { 0.0 };
    if let Some(m_pos) = nav.closest_magnet_pos {
        // [43..45] Точные нормализованные мировые координаты Магнита (X, Y)
        obs[43] = (m_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
        obs[44] = (m_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);

        // [45..47] Относительный вектор направления к Магниту (dx, dy)
        let m_diff = m_pos - boss_pos;
        obs[45] = (m_diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
        obs[46] = (m_diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);

        // [47] Расчетное время прибытия к Магниту (в секундах)
        obs[47] = (nav.time_to_closest_magnet / 20.0).clamp(0.0, 1.0);
    } else {
        obs[43] = 0.5;
        obs[44] = 0.5;
        obs[45] = 0.0;
        obs[46] = 0.0;
        obs[47] = 1.0; // Максимальная задержка если магнита нет
    }

    obs
}
