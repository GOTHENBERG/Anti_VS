use macroquad::prelude::*;
use crate::ecs::world::World;
use crate::core::{MAP_WIDTH, MAP_HEIGHT};
use crate::ai::neural_net::INPUT_SIZE;

/// Извлекает 36-мерный вектор сенсорного восприятия Босса из игрового мира
/// Сенсоры включают 360-градусный лидар, радар плотности толпы, дистанцию до стен,
/// векторы на кристаллы, алтари и снайперов, а также статус здоровья и отката рывка.
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
                // Определяем сектор (0..16)
                let angle = diff.y.atan2(diff.x); // -PI .. PI
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
    // 1.0 = далеко в центре, 0.0 = вплотную к стене (опасность зажатия)
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

    // 5. [30..32] Вектор к ближайшему Алтарю Крови
    let mut closest_altar_dist = 600.0f32;
    let mut altar_vec = Vec2::ZERO;
    for st in &world.structures {
        let dist = st.pos.distance(boss_pos);
        if dist < closest_altar_dist {
            closest_altar_dist = dist;
            altar_vec = (st.pos - boss_pos) / 600.0;
        }
    }
    obs[30] = altar_vec.x;
    obs[31] = altar_vec.y;

    // 6. [32..34] Вектор к ближайшему кластеру культистов-стрелков
    let mut closest_sniper_dist = 450.0f32;
    let mut sniper_vec = Vec2::ZERO;
    for idx in &world.spatial_grid.query_nearby(boss_pos, 450.0) {
        if let Some(mob) = world.mobs.get(*idx) {
            if mob.tier == 3 {
                let dist = mob.pos.distance(boss_pos);
                if dist < closest_sniper_dist {
                    closest_sniper_dist = dist;
                    sniper_vec = (mob.pos - boss_pos) / 450.0;
                }
            }
        }
    }
    obs[32] = sniper_vec.x;
    obs[33] = sniper_vec.y;

    // 7. [34..36] Внутреннее состояние Босса
    obs[34] = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0); // % HP
    obs[35] = if world.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 }; // Готовность рывка

    obs
}
