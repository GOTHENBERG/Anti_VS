use macroquad::prelude::*;
use std::cell::RefCell;
use crate::content::structures::{can_place_blood_altar, BLOOD_ALTAR_COST};
use crate::ecs::components::combat::Faction;
use crate::ecs::components::inventory::{WeaponType, PassiveType};
use crate::ecs::world::World;
use crate::core::{MAP_WIDTH, MAP_HEIGHT, MAP_DIAGONAL};
use crate::ai::neural_net::{BOSS_DASH_BASE_COOLDOWN_SECONDS, INPUT_SIZE};

thread_local! {
    static SENSOR_GRID_BUFFER: RefCell<Vec<usize>> = RefCell::new(Vec::with_capacity(256));
}

#[inline]
fn with_grid_buffer<F, R>(f: F) -> R
where
    F: FnOnce(&mut Vec<usize>) -> R,
{
    SENSOR_GRID_BUFFER.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        f(&mut buf)
    })
}

const BOSS_GRID: usize = 5;
const BOSS_CELL: f32 = 100.0;
const BOSS_HALF: f32 = 250.0;

#[inline]
fn boss_cell_of(dx: f32, dy: f32) -> Option<(usize, usize)> {
    if dx.abs() >= BOSS_HALF || dy.abs() >= BOSS_HALF {
        return None;
    }
    let c = ((dx + BOSS_HALF) / BOSS_CELL).floor() as isize;
    let r = ((dy + BOSS_HALF) / BOSS_CELL).floor() as isize;
    if r >= 0 && r < BOSS_GRID as isize && c >= 0 && c < BOSS_GRID as isize {
        Some((r as usize, c as usize))
    } else {
        None
    }
}

/// ============================================================================
/// СЕНСОРЫ БОССА — 260 входов (с полной топологической сетью расстояний)
/// ============================================================================
/// [0..100]   4 канала × сетка 5×5 (ячейка 100 px, охват 500×500 px):
///            канал 0 — мобы, канал 1 — снаряды Директора,
///            канал 2 — опасные зоны (миазмы алтарей, кислотные лужи Директора),
///            канал 3 — стены/границы карты.
/// [100..116] 16 радиальных секторов (250..600 px): все мобы с весом тира.
/// [116..124] Внутреннее состояние: HP, броня, готовность/КД рывка, уровень,
///            прогресс XP, скорость, прогресс медитации.
/// [124..130] Экономика Директора: души, доход, число алтарей, 3 грейда.
/// [130..135] Агрегаты угроз: TTI ближайшего снаряда, боковое смещение,
///            мобы в 160 px, стрелки в 400 px, элиты в 500 px.
/// [135..150] Цели и тактическая память: магнит, кристалл душ, флаг атаки алтаря,
///            время матча, недавний урон, дистанция до элиты, урон окружения,
///            параметры яда, дистанция промаха снаряда.
/// [150..174] 4 Алтаря Крови (4 × 6 = 24 входа): наличие, координаты (X, Y),
///            расстояние до босса, доля HP, тир.
/// [174..194] 4 Обелиска (4 × 5 = 20 входов): наличие, координаты (X, Y),
///            расстояние до босса, статус активности/HP.
/// [194..199] Активный Разлом Обелиска (5 входов): наличие, таймер прорыва,
///            координаты (X, Y), расстояние до босса.
/// [199..205] Сеть расстояний Алтарь ↔ Алтарь (6 входов): парные расстояния (A1-A2, A1-A3, A1-A4, A2-A3, A2-A4, A3-A4).
/// [205..221] Полная двудольная матрица Алтари ↔ Обелиски (16 входов): расстояния от каждого из 4 алтарей до каждого из 4 обелисков.
/// [221..227] Сеть расстояний Обелиск ↔ Обелиск (6 входов): парные расстояния между всеми обелисками арены.
/// [227..231] Маршрутный коридор к Активному Разлому (4 входа): расстояние от каждого алтаря до точки прорыва.
/// [231..239] Граф ближайших соседей (8 входов): расстояние от каждого алтаря до ближайшего другого алтаря и ближайшего обелиска.
/// [239..255] 4 Союзника в коопе (4 × 4 = 16 входов): вектор dx, dy, HP, alive/revive статус.
/// [255..260] Глобальная арена и ориентация (5 входов): доля живых алтарей, активные обелиски, boss_facing (X, Y), средняя дистанция алтарей.
///
/// Ключевое отличие от старой версии: типы угроз больше не смешаны в один
/// скаляр, у снарядов есть time-to-impact, а собственные ресурсы Босса
/// (КД рывка, медитация) видны политике явно.
pub fn extract_boss_observations(world: &World) -> [f32; INPUT_SIZE] {
    let mut obs = [0.0f32; INPUT_SIZE];
    let boss_pos = world.boss_pos;
    let max_speed = world.boss_inventory.get_movement_speed(160.0).max(1.0);

    const GRID: usize = BOSS_GRID;
    const CELL: f32 = BOSS_CELL;
    const HALF: f32 = BOSS_HALF;

    let mut mob_grid = [[0.0f32; GRID]; GRID];
    let mut proj_grid = [[0.0f32; GRID]; GRID];
    let mut hazard_grid = [[0.0f32; GRID]; GRID];
    let mut wall_grid = [[0.0f32; GRID]; GRID];

    #[inline]
    fn tier_threat(tier: u8) -> f32 {
        match tier {
            1 => 1.0,
            2 => 1.6,
            3 => 2.2,
            4 => 3.0,
            5 => 2.6,
            6 => 3.0,
            7 => 3.2,
            8 => 5.0,
            9 => 2.4,
            10 => 3.0,
            11 => 3.5,
            12 => 4.5,
            13 => 3.6,
            14 => 4.0,
            15 => 4.5,
            _ => 6.0,
        }
    }

    // --- 1. Ядро 5×5: мобы ---
    with_grid_buffer(|buf| {
        world.spatial_grid.query_nearby_into(boss_pos, HALF + 8.0, buf);
        for &idx in buf.iter() {
            if let Some(mob) = world.mobs.get(idx) {
                if !mob.health.is_alive() {
                    continue;
                }
                let dx = mob.pos.x - boss_pos.x;
                let dy = mob.pos.y - boss_pos.y;
                if let Some((r, c)) = boss_cell_of(dx, dy) {
                    let elite = if mob.is_elite { 2.0 } else { 1.0 };
                    mob_grid[r][c] += tier_threat(mob.tier) * elite;
                }
            }
        }
    });

    // --- 1.1 Ядро 5×5: снаряды Директора ---
    for proj in &world.projectiles {
        if proj.faction != Faction::Director || proj.lifetime <= 0.0 || proj.pierce == 0 {
            continue;
        }
        let dx = proj.pos.x - boss_pos.x;
        let dy = proj.pos.y - boss_pos.y;
        if let Some((r, c)) = boss_cell_of(dx, dy) {
            proj_grid[r][c] += (proj.damage / 100.0).clamp(0.2, 2.0);
        }
    }

    // --- 1.2 Ядро 5×5: опасные зоны Директора ---
    for pool in &world.acid_pools {
        if pool.lifetime <= 0.0 || !pool.is_director_source {
            continue;
        }
        for r in 0..GRID {
            for c in 0..GRID {
                let px = -HALF + (c as f32 + 0.5) * CELL;
                let py = -HALF + (r as f32 + 0.5) * CELL;
                let dx = px - (pool.pos.x - boss_pos.x);
                let dy = py - (pool.pos.y - boss_pos.y);
                if dx * dx + dy * dy < (pool.radius + CELL * 0.4).powi(2) {
                    hazard_grid[r][c] += (pool.dps / 20.0).clamp(0.3, 1.5);
                }
            }
        }
    }

    let pestilence_lvl = world.directors.first().map_or(0, |d| d.grade_pestilence);
    if pestilence_lvl > 0 {
        let miasma_radius = 160.0f32;
        let miasma_dps = 10.0 + (pestilence_lvl as f32) * 2.5;
        for st in &world.structures {
            if !st.health.is_alive() {
                continue;
            }
            let sdx = st.pos.x - boss_pos.x;
            let sdy = st.pos.y - boss_pos.y;
            if sdx.abs() > HALF + miasma_radius || sdy.abs() > HALF + miasma_radius {
                continue;
            }
            for r in 0..GRID {
                for c in 0..GRID {
                    let px = -HALF + (c as f32 + 0.5) * CELL;
                    let py = -HALF + (r as f32 + 0.5) * CELL;
                    let dx = px - sdx;
                    let dy = py - sdy;
                    if dx * dx + dy * dy < (miasma_radius + CELL * 0.4).powi(2) {
                        hazard_grid[r][c] += (miasma_dps / 25.0).clamp(0.3, 1.2);
                    }
                }
            }
        }
    }

    // --- 1.3 Ядро 5×5: стены карты ---
    for r in 0..GRID {
        for c in 0..GRID {
            let wx = boss_pos.x - HALF + (c as f32 + 0.5) * CELL;
            let wy = boss_pos.y - HALF + (r as f32 + 0.5) * CELL;
            if wx < 0.0 || wx > MAP_WIDTH || wy < 0.0 || wy > MAP_HEIGHT {
                wall_grid[r][c] = 1.0;
            }
        }
    }

    // Запись 4 каналов по 25 ячеек.
    let mut out_idx = 0usize;
    for grid in [&mob_grid, &proj_grid, &hazard_grid, &wall_grid] {
        for r in 0..GRID {
            for c in 0..GRID {
                obs[out_idx] = grid[r][c].clamp(0.0, 1.0);
                out_idx += 1;
            }
        }
    }
    debug_assert_eq!(out_idx, 100);

    // --- 2. Внешний пояс: 16 секторов, все мобы 250..600 px ---
    let mut outer = [0.0f32; 16];
    let mut mobs_160 = 0usize;
    let mut ranged_400 = 0usize;
    let mut elites_500 = 0usize;
    let mut nearest_elite_dist = f32::MAX;

    with_grid_buffer(|buf| {
        world.spatial_grid.query_nearby_into(boss_pos, 600.0, buf);
        for &idx in buf.iter() {
            if let Some(mob) = world.mobs.get(idx) {
                if !mob.health.is_alive() {
                    continue;
                }
                let diff = mob.pos - boss_pos;
                let dist = diff.length();
                if dist <= 160.0 {
                    mobs_160 += 1;
                }
                let is_ranged = mob.is_ranged();
                if is_ranged && dist <= 400.0 {
                    ranged_400 += 1;
                }
                if mob.is_elite && dist <= 500.0 {
                    elites_500 += 1;
                }
                if mob.is_elite {
                    nearest_elite_dist = nearest_elite_dist.min(dist);
                }
                if (250.0..=600.0).contains(&dist) {
                    let angle = diff.y.atan2(diff.x);
                    let norm = if angle < 0.0 { angle + std::f32::consts::TAU } else { angle };
                    let sector = ((norm / std::f32::consts::TAU) * 16.0) as usize % 16;
                    let weight = tier_threat(mob.tier) * if mob.is_elite { 2.0 } else { 1.0 };
                    outer[sector] += weight;
                }
            }
        }
    });
    for i in 0..16 {
        obs[100 + i] = (outer[i] / 6.0).clamp(0.0, 1.0);
    }

    // --- 3. Внутреннее состояние Босса [116..124] ---
    obs[116] = (world.boss_health.current / world.boss_health.max.max(1.0)).clamp(0.0, 1.0);
    obs[117] = world.boss_resistances.flat_armor / (world.boss_resistances.flat_armor + 30.0);
    obs[118] = if world.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 };
    obs[119] = (world.boss_dash_cooldown / BOSS_DASH_BASE_COOLDOWN_SECONDS).clamp(0.0, 1.0);
    obs[120] = world.boss_inventory.level as f32 / (world.boss_inventory.level as f32 + 50.0);
    let xp_next = world.boss_inventory.xp_to_next_level.max(1.0);
    obs[121] = (world.boss_inventory.current_xp / xp_next).clamp(0.0, 1.0);
    // Честная оценка возможности и процесса медитации:
    let has_nearby_threat = mobs_160 > 0 || with_grid_buffer(|buf| {
        world.spatial_grid.query_nearby_into(boss_pos, 450.0, buf);
        buf.iter().any(|&idx| {
            world.mobs.get(idx).map_or(false, |m| m.health.is_alive() && m.pos.distance_squared(boss_pos) < 450.0 * 450.0)
        })
    });
    let can_meditate = !has_nearby_threat && world.boss_damage_taken_timer <= 0.0;
    let is_meditating = can_meditate && world.boss_still_time >= 2.0 && world.boss_vel.length_squared() < 25.0;
    obs[122] = if is_meditating { 1.0 } else if can_meditate { 0.5 } else { 0.0 };
    obs[123] = (world.boss_still_time / 2.0).clamp(0.0, 1.0);

    // --- 4. Экономика Директора [124..130] ---
    if let Some(dir) = world.directors.first() {
        obs[124] = dir.points / (dir.points + 1000.0);
        obs[125] = dir.income_rate / (dir.income_rate + 50.0);
        obs[126] = (world.structures.iter().filter(|s| crate::content::structures::is_altar_structure_type(s.structure_type) && s.health.is_alive()).count() as f32 / 10.0).clamp(0.0, 1.0);
        obs[127] = dir.grade_fury as f32 / (dir.grade_fury as f32 + 10.0);
        obs[128] = dir.grade_carapace as f32 / (dir.grade_carapace as f32 + 10.0);
        obs[129] = dir.grade_pestilence as f32 / (dir.grade_pestilence as f32 + 10.0);
    }

    // --- 5. Агрегаты угроз [130..135] ---
    let mut best_tti = f32::MAX;
    let mut best_lateral = 0.0f32;
    let mut best_miss_distance = f32::MAX;
    for proj in &world.projectiles {
        if proj.faction != Faction::Director || proj.lifetime <= 0.0 || proj.pierce == 0 {
            continue;
        }
        let relative_pos = proj.pos - boss_pos;
        let relative_vel = proj.vel - world.boss_vel;
        let relative_speed_sq = relative_vel.length_squared();
        if relative_speed_sq <= 1.0 {
            continue;
        }

        let t_closest = (-relative_pos.dot(relative_vel) / relative_speed_sq)
            .clamp(0.0, proj.lifetime.max(0.0).min(3.0));
        let closest_offset = relative_pos + relative_vel * t_closest;
        let miss_distance = closest_offset.length();
        let danger_radius = 60.0 + proj.radius.max(0.0);
        if miss_distance <= danger_radius && t_closest < best_tti {
            best_tti = t_closest;
            best_miss_distance = miss_distance;
            let lateral = relative_vel.x * relative_pos.y - relative_vel.y * relative_pos.x;
            best_lateral = (lateral / relative_pos.length().max(20.0)).clamp(-1.0, 1.0);
        }
    }
    obs[130] = if best_tti.is_finite() {
        (1.0 / (1.0 + best_tti)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    obs[131] = best_lateral;
    obs[132] = (mobs_160 as f32 / 30.0).clamp(0.0, 1.5);
    obs[133] = (ranged_400 as f32 / 20.0).clamp(0.0, 1.5);
    obs[134] = (elites_500 as f32 / 5.0).clamp(0.0, 1.0);

    // --- 6. Цели и тактическая память [135..150] ---
    let mut closest_gem: Option<Vec2> = None;
    let mut closest_gem_dist_sq = 450.0f32 * 450.0;
    let mut closest_magnet: Option<Vec2> = None;
    let mut closest_magnet_dist_sq = f32::MAX;
    for gem in &world.gems {
        let d_sq = gem.pos.distance_squared(boss_pos);
        if gem.is_magnet && d_sq < closest_magnet_dist_sq {
            closest_magnet_dist_sq = d_sq;
            closest_magnet = Some(gem.pos);
        }
        if d_sq < closest_gem_dist_sq {
            closest_gem_dist_sq = d_sq;
            closest_gem = Some(gem.pos);
        }
    }

    obs[135] = if closest_magnet.is_some() { 1.0 } else { 0.0 };
    if let Some(m_pos) = closest_magnet {
        let diff = m_pos - boss_pos;
        obs[136] = (diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
        obs[137] = (diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);
        obs[138] = (diff.length() / max_speed / 20.0).clamp(0.0, 1.0);
    } else {
        obs[138] = 1.0;
    }
    if let Some(g_pos) = closest_gem {
        let diff = g_pos - boss_pos;
        obs[139] = (diff.x / 450.0).clamp(-1.0, 1.0);
        obs[140] = (diff.y / 450.0).clamp(-1.0, 1.0);
    }

    let mut altars_under_attack = 0.0f32;
    for st in &world.structures {
        if st.health.is_alive()
            && crate::content::structures::is_altar_structure_type(st.structure_type)
            && st.health.current < st.health.max * 0.99
        {
            altars_under_attack = 1.0;
            break;
        }
    }
    obs[141] = altars_under_attack;

    let match_cap = world.match_duration_secs.max(60.0);
    obs[142] = (world.game_time / match_cap).clamp(0.0, 1.0);
    obs[143] = (world.boss_inventory.damage_stats.melee_recent / 200.0).clamp(0.0, 1.0);
    obs[144] = (world.boss_inventory.damage_stats.ranged_recent / 200.0).clamp(0.0, 1.0);
    obs[145] = if nearest_elite_dist.is_finite() {
        (nearest_elite_dist / 600.0).clamp(0.0, 1.0)
    } else {
        1.0
    };
    obs[146] = (world.boss_inventory.damage_stats.hazard_recent / 200.0).clamp(0.0, 1.0);
    obs[147] = (world.boss_poison_timer / 5.0).clamp(0.0, 1.0);
    obs[148] = (world.boss_poison_dps / 200.0).clamp(0.0, 1.0);
    obs[149] = if best_miss_distance.is_finite() {
        (best_miss_distance / 160.0).clamp(0.0, 1.0)
    } else {
        1.0
    };

    // --- 7. Структуры: 4 ближайших Алтаря Крови [150..174] (4 × 6 = 24 входа) ---
    // [exists, abs_x, abs_y, dist_to_boss, hp_frac, tier_norm]
    struct AltarSensorData {
        pos: Vec2,
        dist: f32,
        hp_frac: f32,
        tier: u8,
    }
    let mut alive_altars = Vec::with_capacity(16);
    for st in &world.structures {
        if st.health.is_alive() {
            if let Some(tier) = crate::content::structures::altar_tier_from_structure_type(st.structure_type) {
                let dist = st.pos.distance(boss_pos);
                let hp_frac = (st.health.current / st.health.max.max(1.0)).clamp(0.0, 1.0);
                alive_altars.push(AltarSensorData {
                    pos: st.pos,
                    dist,
                    hp_frac,
                    tier,
                });
            }
        }
    }
    alive_altars.sort_unstable_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap_or(std::cmp::Ordering::Equal));

    for i in 0..4 {
        let base = 150 + i * 6;
        if let Some(altar) = alive_altars.get(i) {
            obs[base] = 1.0;
            obs[base + 1] = (altar.pos.x / MAP_WIDTH).clamp(0.0, 1.0);
            obs[base + 2] = (altar.pos.y / MAP_HEIGHT).clamp(0.0, 1.0);
            obs[base + 3] = (altar.dist / MAP_DIAGONAL).clamp(0.0, 1.0);
            obs[base + 4] = altar.hp_frac;
            obs[base + 5] = (altar.tier as f32 / 16.0).clamp(0.0, 1.0);
        } else {
            obs[base] = 0.0;
            obs[base + 1] = 0.0;
            obs[base + 2] = 0.0;
            obs[base + 3] = 1.0;
            obs[base + 4] = 0.0;
            obs[base + 5] = 0.0;
        }
    }

    // --- 8. Обелиски: 4 обелиска арены [174..194] (4 × 5 = 20 входов) ---
    // [exists, abs_x, abs_y, dist_to_boss, active_hp]
    struct ObeliskSensorData {
        pos: Vec2,
        dist: f32,
        active_hp: f32,
        is_active: bool,
    }
    let mut obelisk_list = Vec::with_capacity(4);
    for st in &world.structures {
        if st.structure_type == crate::content::structures::STRUCTURE_OBELISK {
            let active_hp = if st.is_active && st.health.is_alive() {
                (st.health.current / st.health.max.max(1.0)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let dist = st.pos.distance(boss_pos);
            obelisk_list.push(ObeliskSensorData {
                pos: st.pos,
                dist,
                active_hp,
                is_active: st.is_active && st.health.is_alive(),
            });
        }
    }
    obelisk_list.sort_unstable_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap_or(std::cmp::Ordering::Equal));

    for j in 0..4 {
        let base = 174 + j * 5;
        if let Some(ob) = obelisk_list.get(j) {
            obs[base] = 1.0;
            obs[base + 1] = (ob.pos.x / MAP_WIDTH).clamp(0.0, 1.0);
            obs[base + 2] = (ob.pos.y / MAP_HEIGHT).clamp(0.0, 1.0);
            obs[base + 3] = (ob.dist / MAP_DIAGONAL).clamp(0.0, 1.0);
            obs[base + 4] = ob.active_hp;
        } else {
            obs[base] = 0.0;
            obs[base + 1] = 0.0;
            obs[base + 2] = 0.0;
            obs[base + 3] = 1.0;
            obs[base + 4] = 0.0;
        }
    }

    // --- 9. Активный Разлом Обелиска [194..199] (5 входов) ---
    let mut active_rift = None;
    for st in &world.structures {
        if st.structure_type == crate::content::structures::STRUCTURE_OBELISK
            && st.is_active
            && st.health.is_alive()
        {
            active_rift = Some((st.pos, st.state_timer));
            break;
        }
    }

    if let Some((rift_pos, state_timer)) = active_rift {
        obs[194] = 1.0;
        obs[195] = (state_timer / 20.0).clamp(0.0, 1.0);
        obs[196] = (rift_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
        obs[197] = (rift_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);
        obs[198] = (rift_pos.distance(boss_pos) / MAP_DIAGONAL).clamp(0.0, 1.0);
    } else {
        obs[194] = 0.0;
        obs[195] = 0.0;
        obs[196] = 0.0;
        obs[197] = 0.0;
        obs[198] = 1.0;
    }

    // --- 10. ТОПОЛОГИЧЕСКАЯ СЕТЬ РАССТОЯНИЙ (GRAPH DISTANCE NETWORK) [199..239] (40 входов) ---
    let get_altar_pos = |idx: usize| alive_altars.get(idx).map(|a| a.pos);
    let get_obelisk_pos = |idx: usize| obelisk_list.get(idx).map(|o| o.pos);

    // 10.1 Парные расстояния Алтарь ↔ Алтарь [199..205] (6 входов):
    let altar_pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    for (idx, &(i, j)) in altar_pairs.iter().enumerate() {
        obs[199 + idx] = match (get_altar_pos(i), get_altar_pos(j)) {
            (Some(p1), Some(p2)) => (p1.distance(p2) / MAP_DIAGONAL).clamp(0.0, 1.0),
            _ => 1.0,
        };
    }

    // 10.2 Полная двудольная матрица расстояний Алтари ↔ Обелиски [205..221] (16 входов):
    for i in 0..4 {
        for j in 0..4 {
            let slot = 205 + i * 4 + j;
            obs[slot] = match (get_altar_pos(i), get_obelisk_pos(j)) {
                (Some(ap), Some(op)) => (ap.distance(op) / MAP_DIAGONAL).clamp(0.0, 1.0),
                _ => 1.0,
            };
        }
    }

    // 10.3 Парные расстояния Обелиск ↔ Обелиск [221..227] (6 входов):
    let obelisk_pairs = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    for (idx, &(i, j)) in obelisk_pairs.iter().enumerate() {
        obs[221 + idx] = match (get_obelisk_pos(i), get_obelisk_pos(j)) {
            (Some(p1), Some(p2)) => (p1.distance(p2) / MAP_DIAGONAL).clamp(0.0, 1.0),
            _ => 1.0,
        };
    }

    // 10.4 Маршрутный коридор к Активному Разлому [227..231] (4 входа):
    for i in 0..4 {
        obs[227 + i] = match (get_altar_pos(i), active_rift) {
            (Some(ap), Some((rp, _))) => (ap.distance(rp) / MAP_DIAGONAL).clamp(0.0, 1.0),
            _ => 1.0,
        };
    }

    // 10.5 Граф ближайших соседей и локальная кластеризация [231..239] (8 входов):
    for i in 0..4 {
        let base = 231 + i * 2;
        if let Some(ap) = get_altar_pos(i) {
            let mut min_altar_d = f32::MAX;
            for j in 0..alive_altars.len() {
                if i != j {
                    min_altar_d = min_altar_d.min(ap.distance(alive_altars[j].pos));
                }
            }
            obs[base] = if min_altar_d < f32::MAX {
                (min_altar_d / MAP_DIAGONAL).clamp(0.0, 1.0)
            } else {
                1.0
            };

            let mut min_obelisk_d = f32::MAX;
            for ob in &obelisk_list {
                min_obelisk_d = min_obelisk_d.min(ap.distance(ob.pos));
            }
            obs[base + 1] = if min_obelisk_d < f32::MAX {
                (min_obelisk_d / MAP_DIAGONAL).clamp(0.0, 1.0)
            } else {
                1.0
            };
        } else {
            obs[base] = 1.0;
            obs[base + 1] = 1.0;
        }
    }

    // --- 11. Союзники: 4 других босса команды [239..255] (16 входов) ---
    let mut ally_slots = [(0.0f32, 0.0f32, 0.0f32, -1.0f32, f32::MAX); 4];
    let mut ally_count = 0usize;

    for player in world.coop_bosses.iter().filter(|player| player.id != world.active_boss_id) {
        if ally_count < 4 {
            let hp = (player.boss_health.current / player.boss_health.max.max(1.0)).clamp(0.0, 1.0);
            let alive_status = if player.boss_health.is_alive() {
                1.0
            } else if player.revive_progress > 0.0 {
                -1.0 + (player.revive_progress / crate::ecs::systems::coop::REVIVE_SECONDS).clamp(0.0, 1.0) * 0.9
            } else {
                -1.0
            };
            let diff = player.boss_pos - boss_pos;
            let d_sq = diff.length_squared();
            ally_slots[ally_count] = (
                (diff.x / MAP_WIDTH).clamp(-1.0, 1.0),
                (diff.y / MAP_HEIGHT).clamp(-1.0, 1.0),
                hp,
                alive_status,
                d_sq,
            );
            ally_count += 1;
        }
    }
    ally_slots[..ally_count].sort_unstable_by(|a, b| a.4.partial_cmp(&b.4).unwrap_or(std::cmp::Ordering::Equal));

    for (index, (dx, dy, hp, alive, _)) in ally_slots.iter().enumerate() {
        let base = 239 + index * 4;
        obs[base] = *dx;
        obs[base + 1] = *dy;
        obs[base + 2] = *hp;
        obs[base + 3] = *alive;
    }

    // --- 12. Глобальная арена и вектор движения [255..260] (5 входов) ---
    obs[255] = (alive_altars.len() as f32 / 12.0).clamp(0.0, 1.0);
    obs[256] = (obelisk_list.iter().filter(|o| o.is_active).count() as f32 / 4.0).clamp(0.0, 1.0);
    obs[257] = world.boss_facing.x.clamp(-1.0, 1.0);
    obs[258] = world.boss_facing.y.clamp(-1.0, 1.0);

    let mut total_pair_dist = 0.0f32;
    let mut pair_count = 0usize;
    for i in 0..alive_altars.len() {
        for j in (i + 1)..alive_altars.len() {
            total_pair_dist += alive_altars[i].pos.distance(alive_altars[j].pos);
            pair_count += 1;
        }
    }
    obs[259] = if pair_count > 0 {
        ((total_pair_dist / pair_count as f32) / MAP_DIAGONAL).clamp(0.0, 1.0)
    } else {
        1.0
    };

    obs
}

/// ============================================================================
/// СЕНСОРЫ ДИРЕКТОРА — 177 входов
/// ============================================================================
pub const DIRECTOR_INPUT_SIZE: usize = crate::ai::neural_net::DIRECTOR_INPUT_SIZE;
pub const DIRECTOR_ALTAR_SLOT_COUNT: usize = 25;

/// Legal slots are shared by the observation and the action mask so the
/// Director never trains against a position rule different from execution.
pub fn director_altar_slot_mask(world: &World) -> [bool; DIRECTOR_ALTAR_SLOT_COUNT] {
    std::array::from_fn(|slot| {
        can_place_blood_altar(
            world,
            crate::ai::neural_net::DirectorBrainNet::altar_slot_position(slot),
        )
    })
}

/// Преобразует тип оружия Босса в фиксированный индекс [0..39]
pub fn weapon_type_to_index(w: WeaponType) -> usize {
    match w {
        WeaponType::WindBlade => 0,
        WeaponType::FireAura => 1,
        WeaponType::MagicWand => 2,
        WeaponType::HolyHammer => 3,
        WeaponType::IceShards => 4,
        WeaponType::PoisonOrbs => 5,
        WeaponType::ExecutionerAxe => 6,
        WeaponType::CelestialBow => 7,
        WeaponType::ThunderOrb => 8,
        WeaponType::BloodDaggers => 9,
        WeaponType::SingularityOrb => 10,
        WeaponType::ChainFrost => 11,
        WeaponType::BrimstoneRay => 12,
        WeaponType::MissileSwarm => 13,
        WeaponType::ChainLightning => 14,
        WeaponType::DivineBell => 15,
        WeaponType::SoulPhantoms => 16,
        WeaponType::AegisOrbiters => 17,
        WeaponType::ChaosMeteor => 18,
        WeaponType::VoidRift => 19,
        WeaponType::BladeStorm => 20,
        WeaponType::Hellfire => 21,
        WeaponType::DeathRay => 22,
        WeaponType::DivineShield => 23,
        WeaponType::BlizzardStorm => 24,
        WeaponType::ToxicPestilence => 25,
        WeaponType::WhirlwindCleaver => 26,
        WeaponType::StarfallBarrage => 27,
        WeaponType::ThunderstormWrath => 28,
        WeaponType::BloodHarvest => 29,
        WeaponType::EventHorizon => 30,
        WeaponType::AbsoluteZero => 31,
        WeaponType::InfernalCataclysm => 32,
        WeaponType::DoomsdayBarrage => 33,
        WeaponType::StormGodWrath => 34,
        WeaponType::CathedralAura => 35,
        WeaponType::BansheeWail => 36,
        WeaponType::InvincibleBastion => 37,
        WeaponType::ArmageddonStorm => 38,
        WeaponType::SingularityCollapse => 39,
        WeaponType::SacredScriptures => 40,
        WeaponType::FaithBoomerang => 41,
        WeaponType::CursedChalice => 42,
        WeaponType::VampiricAura => 43,
        WeaponType::RunicPrism => 44,
        WeaponType::AbyssalBeam => 45,
        WeaponType::ChronoStasis => 46,
        WeaponType::AstralCloak => 47,
        WeaponType::InfernalLiturgy => 48,
        WeaponType::HeavenlyJudgment => 49,
        WeaponType::BloodDeluge => 50,
        WeaponType::SoulFeast => 51,
        WeaponType::SpatialShatter => 52,
        WeaponType::ApocalypseCascade => 53,
        WeaponType::InfiniteDilation => 54,
        WeaponType::BloodEmperorShroud => 55,
    }
}

/// Преобразует тип пассивки/реликвии Босса в фиксированный индекс [0..34]
pub fn passive_type_to_index(p: PassiveType) -> usize {
    match p {
        PassiveType::SpeedGloves => 0,
        PassiveType::GiantHeart => 1,
        PassiveType::CooldownTome => 2,
        PassiveType::LightArmor => 3,
        PassiveType::FrostRing => 4,
        PassiveType::CorrosiveFlask => 5,
        PassiveType::BloodStone => 6,
        PassiveType::EagleScope => 7,
        PassiveType::ChronoHourglass => 8,
        PassiveType::MagnetSphere => 9,
        PassiveType::TeslaCoil => 10,
        PassiveType::CloverOfFate => 11,
        PassiveType::PrismaticFocus => 12,
        PassiveType::TitanAegis => 13,
        PassiveType::ImmortalAegis => 14,
        PassiveType::ArchmageShard => 15,
        PassiveType::DivineRapier => 16,
        PassiveType::OctarineCore => 17,
        PassiveType::SolarRelic => 18,
        PassiveType::StaticCore => 19,
        PassiveType::ColossalEye => 20,
        PassiveType::BlessedHeart => 21,
        PassiveType::Pyromaniac => 22,
        PassiveType::SoulHarvester => 23,
        PassiveType::DivinityAura => 24,
        PassiveType::UkuleleResonance => 25,
        PassiveType::FateWeaver => 26,
        PassiveType::SiegeMortar => 27,
        PassiveType::SporeMushroom => 28,
        PassiveType::DuplicatorRelic => 29,
        PassiveType::CandelabraOfFury => 30,
        PassiveType::AbyssalCurseStone => 31,
        PassiveType::SilverTears => 32,
        PassiveType::GoldenSeal => 33,
        PassiveType::TitanBloodArmor => 34,
    }
}

/// 177-мерный вектор Директора.
///
/// [0..9]     кинематика Босса
/// [9..18]    HP/XP/уровень/действия Босса
/// [18..39]   4 ближайших алтаря (флаг, dx, dy, дистанция, HP)
/// [39..48]   экономика, грейды, время
/// [48..64]   счётчики тиров армии
/// [64..80]   пространственная карта армии 4×4 вокруг Босса
/// [80..88]   агрегаты армии: центр, разброс, ближайший, доля рядом
/// [88..104]  агрегаты арсенала Босса вместо 40 разреженных уровней
/// [104..112] агрегаты пассивок Босса вместо 28 разреженных уровней
/// [112..132] тактические признаки: доступность действий, давление, цели
/// [132..138] присутствие шести семантических архетипов оружия/пассивок
/// [138]       доля свободных слотов для Алтарей Крови
pub fn extract_director_observations(world: &World, sim_time: f32) -> [f32; DIRECTOR_INPUT_SIZE] {
    let mut obs = [0.0f32; DIRECTOR_INPUT_SIZE];

    // Динамический выбор главного живого босса-фокуса для позиционирования и армии.
    // Если Босс №1 пал, Директор переключает внимание на живого соратника, а не сканирует труп.
    let (boss_pos, boss_vel, boss_hp_frac, boss_max_hp, boss_level, boss_cur_xp, boss_next_xp, boss_invuln, boss_dash, boss_still, boss_poison) = if world.boss_health.is_alive() {
        (
            world.boss_pos,
            world.boss_vel,
            (world.boss_health.current / world.boss_health.max.max(1.0)).clamp(0.0, 1.0),
            world.boss_health.max,
            world.boss_inventory.level,
            world.boss_inventory.current_xp,
            world.boss_inventory.xp_to_next_level.max(1.0),
            if world.boss_invulnerable_timer > 0.0 { 1.0 } else { 0.0 },
            if world.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 },
            if world.boss_still_time >= 1.2 { 1.0 } else { 0.0 },
            if world.boss_poison_timer > 0.0 { 1.0 } else { 0.0 },
        )
    } else if let Some(alive_player) = world.coop_bosses.iter().find(|p| p.boss_health.is_alive()) {
        (
            alive_player.boss_pos,
            alive_player.boss_vel,
            (alive_player.boss_health.current / alive_player.boss_health.max.max(1.0)).clamp(0.0, 1.0),
            alive_player.boss_health.max,
            alive_player.boss_inventory.level,
            alive_player.boss_inventory.current_xp,
            alive_player.boss_inventory.xp_to_next_level.max(1.0),
            if alive_player.boss_invulnerable_timer > 0.0 { 1.0 } else { 0.0 },
            if alive_player.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 },
            if alive_player.boss_still_time >= 1.2 { 1.0 } else { 0.0 },
            if alive_player.boss_poison_timer > 0.0 { 1.0 } else { 0.0 },
        )
    } else {
        (
            world.boss_pos,
            world.boss_vel,
            0.0,
            world.boss_health.max,
            world.boss_inventory.level,
            world.boss_inventory.current_xp,
            world.boss_inventory.xp_to_next_level.max(1.0),
            0.0,
            0.0,
            0.0,
            0.0,
        )
    };

    // 1. [0..9] Карта и кинематика Босса
    obs[0] = (boss_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
    obs[1] = (boss_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);
    let dist_to_wall = boss_pos.x.min(MAP_WIDTH - boss_pos.x).min(boss_pos.y).min(MAP_HEIGHT - boss_pos.y);
    obs[2] = (dist_to_wall / (MAP_WIDTH * 0.5)).clamp(0.0, 1.0);
    obs[3] = (boss_vel.x / 300.0).clamp(-1.0, 1.0);
    obs[4] = (boss_vel.y / 300.0).clamp(-1.0, 1.0);
    obs[5] = (boss_vel.length() / 300.0).clamp(0.0, 1.5);
    obs[6] = world.boss_resistances.flat_armor / (world.boss_resistances.flat_armor + 30.0);
    obs[7] = boss_max_hp / (boss_max_hp + 1500.0);
    obs[8] = boss_level as f32 / (boss_level as f32 + 50.0);

    // 2. [9..18] HP, XP и действия Босса
    obs[9] = boss_hp_frac;
    obs[10] = boss_max_hp / (boss_max_hp + 1500.0);
    obs[11] = boss_level as f32 / (boss_level as f32 + 50.0);
    obs[12] = (boss_cur_xp / boss_next_xp).clamp(0.0, 1.0);
    obs[13] = (boss_next_xp / 500.0).clamp(0.05, 5.0);
    obs[14] = boss_invuln;
    obs[15] = boss_dash;
    obs[16] = if boss_still >= 1.0 && world.boss_damage_taken_timer <= 0.0 { 1.0 } else { 0.0 };
    obs[17] = boss_poison;

    // 3. [18..39] Алтари: 4 ближайших (строго алтари, без стен и обелисков)
    let mut nearest: [Option<(usize, f32)>; 4] = [None; 4];
    let mut alive_altars = 0usize;
    let mut under_attack = 0usize;
    let mut closest_altar_pos = Vec2::ZERO;
    let mut closest_altar_dist_sq = f32::MAX;

    for (idx, st) in world.structures.iter().enumerate() {
        if !st.health.is_alive() || !crate::content::structures::is_altar_structure_type(st.structure_type) {
            continue;
        }
        alive_altars += 1;
        if st.health.current < st.health.max * 0.99 {
            under_attack += 1;
        }
        let d_sq = st.pos.distance_squared(boss_pos);
        if d_sq < closest_altar_dist_sq {
            closest_altar_dist_sq = d_sq;
            closest_altar_pos = st.pos;
        }

        for slot in 0..4 {
            let replace = match nearest[slot] {
                None => true,
                Some((_, best_d)) => d_sq < best_d,
            };
            if replace {
                for k in (slot + 1..4).rev() {
                    nearest[k] = nearest[k - 1];
                }
                nearest[slot] = Some((idx, d_sq));
                break;
            }
        }
    }

    obs[18] = (alive_altars as f32 / 10.0).clamp(0.0, 1.0);
    for i in 0..4 {
        let base = 19 + i * 5;
        if let Some((idx, _)) = nearest[i] {
            if let Some(st) = world.structures.get(idx) {
                obs[base] = 1.0;
                let diff = st.pos - boss_pos;
                obs[base + 1] = (diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
                obs[base + 2] = (diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);
                obs[base + 3] = (diff.length() / MAP_WIDTH.max(MAP_HEIGHT)).clamp(0.0, 1.0);
                obs[base + 4] = (st.health.current / st.health.max.max(1.0)).clamp(0.0, 1.0);
            }
        } else {
            obs[base + 3] = 1.0;
        }
    }

    // 4. [39..48] Экономика и статус матча
    let mut director_points = 0.0;
    let mut director_spent = 0.0;
    let mut altar_ready = false;
    let mut time_since_spawn = 10.0;
    let mut grade_fury = 0u32;
    let mut grade_carapace = 0u32;
    let mut grade_pestilence = 0u32;
    let mut squad_cooldowns = [0.0f32; 16];

    if let Some(dir) = world.directors.first() {
        obs[39] = dir.points / (dir.points + 1000.0);
        obs[40] = dir.income_rate / (dir.income_rate + 50.0);
        obs[41] = (dir.altar_cooldown / 60.0).clamp(0.0, 1.0);
        obs[42] = dir.grade_fury as f32 / (dir.grade_fury as f32 + 10.0);
        obs[43] = dir.grade_carapace as f32 / (dir.grade_carapace as f32 + 10.0);
        obs[44] = dir.grade_pestilence as f32 / (dir.grade_pestilence as f32 + 10.0);
        obs[45] = dir.total_points_spent / (dir.total_points_spent + 50000.0);

        director_points = dir.points;
        director_spent = dir.total_points_spent;
        altar_ready = dir.altar_cooldown <= 0.0 && dir.points >= BLOOD_ALTAR_COST;
        time_since_spawn = dir.time_since_last_spawn;
        grade_fury = dir.grade_fury;
        grade_carapace = dir.grade_carapace;
        grade_pestilence = dir.grade_pestilence;
        squad_cooldowns = dir.squad_cooldowns;
    }
    // 5-6. [48..88] Армия: тиры + пространственная карта 4×4 + агрегаты
    let mut tier_counts = [0usize; 16];
    let mut army_grid = [[0.0f32; 4]; 4];
    let mut total_mobs = 0usize;
    let mut ranged_mobs = 0usize;
    let mut centroid = Vec2::ZERO;
    let mut sum_dist = 0.0f32;
    let mut sum_dist_sq = 0.0f32;
    let mut nearest_mob_dist_sq = f32::MAX;
    let mut near_boss = 0usize;
    let mut near_altar = 0usize;
    let mut hp_sum = 0.0f32;
    let mut power = 0.0f32;

    const ARMY_HALF: f32 = 400.0;
    const ARMY_CELL: f32 = 200.0;

    for mob in &world.mobs {
        if !mob.health.is_alive() || mob.tier < 1 || mob.tier > 16 {
            continue;
        }
        total_mobs += 1;
        if mob.is_ranged() {
            ranged_mobs += 1;
        }
        tier_counts[(mob.tier - 1) as usize] += 1;

        let diff = mob.pos - boss_pos;
        let dist = diff.length();
        centroid += mob.pos;
        sum_dist += dist;
        sum_dist_sq += dist * dist;
        nearest_mob_dist_sq = nearest_mob_dist_sq.min(dist * dist);
        if dist <= 250.0 {
            near_boss += 1;
        }
        if closest_altar_dist_sq < f32::MAX && mob.pos.distance_squared(closest_altar_pos) <= 250.0 * 250.0 {
            near_altar += 1;
        }

        let hp_frac = (mob.health.current / mob.health.max.max(1.0)).clamp(0.0, 1.0);
        hp_sum += hp_frac;
        power += mob.tier as f32 * (0.5 + hp_frac * 0.5);

        if diff.x.abs() < ARMY_HALF && diff.y.abs() < ARMY_HALF {
            let c = ((diff.x + ARMY_HALF) / ARMY_CELL).floor().clamp(0.0, 3.0) as usize;
            let r = ((diff.y + ARMY_HALF) / ARMY_CELL).floor().clamp(0.0, 3.0) as usize;
            army_grid[r][c] += 1.0 + mob.tier as f32 * 0.25;
        }
    }

    obs[46] = (total_mobs as f32 / (total_mobs as f32 + 200.0)).clamp(0.0, 1.0);
    let match_cap = world.match_duration_secs.max(60.0);
    obs[47] = (sim_time / match_cap).clamp(0.0, 1.0);

    for i in 0..16 {
        let config = crate::content::mobs::get_squad_config((i + 1) as u8);
        let expected = (config.count as f32 * 3.0).max(1.0);
        obs[48 + i] = (tier_counts[i] as f32 / expected).clamp(0.0, 2.5);
    }

    let mut idx = 64usize;
    for r in 0..4 {
        for c in 0..4 {
            obs[idx] = (army_grid[r][c] / 8.0).clamp(0.0, 1.5);
            idx += 1;
        }
    }

    let inv = if total_mobs > 0 { 1.0 / total_mobs as f32 } else { 0.0 };
    let centroid = if total_mobs > 0 { centroid * inv } else { boss_pos };
    let mean_dist = sum_dist * inv;
    let variance = (sum_dist_sq * inv - mean_dist * mean_dist).max(0.0);

    obs[80] = (total_mobs as f32 / 500.0).clamp(0.0, 2.0);
    obs[81] = ((centroid.x - boss_pos.x) / 800.0).clamp(-1.5, 1.5);
    obs[82] = ((centroid.y - boss_pos.y) / 800.0).clamp(-1.5, 1.5);
    obs[83] = (variance.sqrt() / 800.0).clamp(0.0, 1.5);
    obs[84] = if nearest_mob_dist_sq.is_finite() {
        (nearest_mob_dist_sq.sqrt() / 1000.0).clamp(0.0, 1.5)
    } else {
        1.5
    };
    obs[85] = (near_boss as f32 * inv).clamp(0.0, 1.0);
    obs[86] = (hp_sum * inv).clamp(0.0, 1.0);
    obs[87] = (power / 300.0).clamp(0.0, 2.0);

    // 7. [88..104] Агрегаты арсенала Босса (вместо 40 разреженных уровней)
    let mut total_weapon_levels = 0.0f32;
    let mut weapon_count = 0usize;
    let mut evolved_count = 0usize;
    let mut levels = [0.0f32; 8];
    let mut type_levels = [0.0f32; 5]; // Physical, Fire, Ice, Poison, Holy
    let mut true_levels = 0.0f32;
    let mut archetype_counts = [0usize; 6];

    for w in &world.boss_inventory.weapons {
        weapon_count += 1;
        total_weapon_levels += w.level as f32;
        if w.is_evolved {
            evolved_count += 1;
        }
        if weapon_count <= levels.len() {
            levels[weapon_count - 1] = w.level as f32;
        }

        let def = crate::content::weapons::get_weapon_def(w.kind);
        let v = w.level as f32;
        match def.damage_type {
            crate::ecs::components::combat::DamageType::Physical => type_levels[0] += v,
            crate::ecs::components::combat::DamageType::Fire => type_levels[1] += v,
            crate::ecs::components::combat::DamageType::Ice => type_levels[2] += v,
            crate::ecs::components::combat::DamageType::Poison => type_levels[3] += v,
            crate::ecs::components::combat::DamageType::Holy => type_levels[4] += v,
            crate::ecs::components::combat::DamageType::True => true_levels += v,
        }
        for &archetype in crate::ai::models::weapon_archetypes(w.kind) {
            archetype_counts[archetype] += 1;
        }
    }
    for p in &world.boss_inventory.passives {
        for &archetype in crate::ai::models::passive_archetypes(p.kind) {
            archetype_counts[archetype] += 1;
        }
    }

    if weapon_count > 0 {
        let sort_len = weapon_count.min(levels.len());
        levels[..sort_len]
            .sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    }
    let avg_weapon_level = if weapon_count > 0 {
        total_weapon_levels / weapon_count as f32
    } else {
        0.0
    };

    obs[88] = (total_weapon_levels / 40.0).clamp(0.0, 2.0);
    obs[89] = (weapon_count as f32 / 8.0).clamp(0.0, 1.5);
    obs[90] = (evolved_count as f32 / 8.0).clamp(0.0, 1.0);
    for i in 0..5 {
        obs[91 + i] = (levels[i] / 8.0).clamp(0.0, 1.5);
    }
    obs[96] = (avg_weapon_level / 8.0).clamp(0.0, 1.5);
    obs[97] = (type_levels[1] / 20.0).clamp(0.0, 1.5); // Fire
    obs[98] = (type_levels[2] / 20.0).clamp(0.0, 1.5); // Ice
    obs[99] = (type_levels[3] / 20.0).clamp(0.0, 1.5); // Poison
    obs[100] = (type_levels[4] / 20.0).clamp(0.0, 1.5); // Holy
    obs[101] = (type_levels[0] / 20.0).clamp(0.0, 1.5); // Physical
    obs[102] = (true_levels / 20.0).clamp(0.0, 1.5);
    obs[103] = (archetype_counts.iter().filter(|&&count| count > 0).count() as f32 / 6.0)
        .clamp(0.0, 1.0);
    for (i, count) in archetype_counts.iter().enumerate() {
        obs[132 + i] = if *count > 0 { 1.0 } else { 0.0 };
    }

    // 8. [104..112] Агрегаты пассивок
    let mut passive_count = 0usize;
    let mut passive_total = 0.0f32;
    let mut passive_top = 0.0f32;
    let mut def_levels = 0.0f32;
    let mut off_levels = 0.0f32;
    let mut util_levels = 0.0f32;
    let mut evolution_pairs = 0usize;

    for p in &world.boss_inventory.passives {
        passive_count += 1;
        let lvl = p.level as f32;
        passive_total += lvl;
        passive_top = passive_top.max(lvl);
        match p.kind {
            PassiveType::GiantHeart
            | PassiveType::LightArmor
            | PassiveType::TitanAegis
            | PassiveType::ImmortalAegis
            | PassiveType::BlessedHeart
            | PassiveType::SporeMushroom
            | PassiveType::BloodStone => def_levels += lvl,
            PassiveType::PrismaticFocus
            | PassiveType::ArchmageShard
            | PassiveType::DivineRapier
            | PassiveType::Pyromaniac
            | PassiveType::SolarRelic
            | PassiveType::StaticCore
            | PassiveType::ColossalEye
            | PassiveType::SoulHarvester
            | PassiveType::SiegeMortar
            | PassiveType::TeslaCoil
            | PassiveType::UkuleleResonance => off_levels += lvl,
            _ => util_levels += lvl,
        }
    }
    for weapon in &world.boss_inventory.weapons {
        if let Some(needed_passive) =
            crate::ecs::components::inventory::BossInventory::get_evolution_passive_for_weapon(
                weapon.kind,
            )
        {
            if world.boss_inventory.passives.iter().any(|p| p.kind == needed_passive) {
                evolution_pairs += 1;
            }
        }
    }
    let avg_passive = if passive_count > 0 {
        passive_total / passive_count as f32
    } else {
        0.0
    };

    obs[104] = (passive_count as f32 / 8.0).clamp(0.0, 1.5);
    obs[105] = (passive_total / 20.0).clamp(0.0, 2.0);
    obs[106] = (def_levels / 12.0).clamp(0.0, 2.0);
    obs[107] = (off_levels / 16.0).clamp(0.0, 2.0);
    obs[108] = (util_levels / 12.0).clamp(0.0, 2.0);
    obs[109] = (passive_top / 5.0).clamp(0.0, 1.5);
    obs[110] = (avg_passive / 5.0).clamp(0.0, 1.5);
    obs[111] = (evolution_pairs as f32 / 8.0).clamp(0.0, 1.0);

    // 9. [112..132] Тактика: доступность, давление, цели
    let mut affordable_count = 0usize;
    let mut best_affordable_tier = 0usize;
    for tier in 1..=16u8 {
        let config = crate::content::mobs::get_squad_config(tier);
        let unlocked = director_spent >= config.unlock_spent_threshold;
        let cooldown_ok = squad_cooldowns[(tier - 1) as usize] <= 0.0;
        if unlocked && director_points >= config.cost && cooldown_ok {
            affordable_count += 1;
            best_affordable_tier = tier as usize;
        }
    }

    let mut next_unlock_progress = 1.0f32;
    for tier in 1..=16u8 {
        let config = crate::content::mobs::get_squad_config(tier);
        if director_spent < config.unlock_spent_threshold {
            next_unlock_progress = (director_spent / config.unlock_spent_threshold.max(1.0)).clamp(0.0, 1.0);
            break;
        }
    }

    let mut mobs_within_250 = 0usize;
    with_grid_buffer(|nearby_buf| {
        world.spatial_grid.query_nearby_into(boss_pos, 250.0, nearby_buf);
        for &m_idx in nearby_buf.iter() {
            if let Some(m) = world.mobs.get(m_idx) {
                if m.health.is_alive() {
                    mobs_within_250 += 1;
                }
            }
        }
    });

    let map_diag = (MAP_WIDTH * MAP_WIDTH + MAP_HEIGHT * MAP_HEIGHT).sqrt();
    let boss_to_altar = if closest_altar_dist_sq < f32::MAX {
        closest_altar_dist_sq.sqrt() / map_diag
    } else {
        1.0
    };
    let boss_to_center = boss_pos.distance(Vec2::new(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5)) / map_diag;
    let recent_damage = world.boss_inventory.damage_stats.melee_recent
        + world.boss_inventory.damage_stats.ranged_recent
        + world.boss_inventory.damage_stats.hazard_recent;

    obs[112] = (affordable_count as f32 / 16.0).clamp(0.0, 1.0);
    obs[113] = (best_affordable_tier as f32 / 16.0).clamp(0.0, 1.0);
    // 114: признак медитации Босса (цель для прерывания)
    obs[114] = if world.boss_still_time >= 2.0 && world.boss_damage_taken_timer <= 0.0 { 1.0 } else { 0.0 };
    // 115: число активных обелисков
    obs[115] = world.structures.iter().filter(|s| s.structure_type == crate::content::structures::STRUCTURE_OBELISK && s.is_active && s.health.is_alive()).count() as f32 / 4.0;
    // 116: готовность стены Директора
    obs[116] = if world.directors.first().map_or(0.0, |d| d.wall_cooldown) <= 0.0 { 1.0 } else { 0.0 };
    obs[117] = (recent_damage / 500.0).clamp(0.0, 1.0);
    // 118: доля стрелков (дальнобойных) в армии
    obs[118] = (ranged_mobs as f32 * inv).clamp(0.0, 1.0);
    obs[119] = (near_altar as f32 * inv).clamp(0.0, 1.0);
    obs[120] = if alive_altars > 0 {
        (under_attack as f32 / alive_altars as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    obs[121] = if altar_ready { 1.0 } else { 0.0 };
    obs[122] = (time_since_spawn / 10.0).clamp(0.0, 1.0);
    obs[123] = next_unlock_progress;
    obs[124] = ((grade_fury + grade_carapace + grade_pestilence) as f32 / 60.0).clamp(0.0, 1.0);
    // 125: накопленный урон по боссу за раунд
    let total_boss_dmg = world.boss_inventory.damage_stats.melee_total
        + world.boss_inventory.damage_stats.ranged_total
        + world.boss_inventory.damage_stats.hazard_total;
    obs[125] = total_boss_dmg / (total_boss_dmg + 5000.0);
    // 126: DPS яда по боссу
    obs[126] = world.boss_poison_dps / (world.boss_poison_dps + 50.0);
    // Таймер до следующего тика начисления дохода душ (раз в 2.0 секунды)
    obs[127] = (world.directors.first().map_or(0.0, |d| d.income_timer / 2.0)).clamp(0.0, 1.0);
    obs[128] = (mobs_within_250 as f32 / 100.0).clamp(0.0, 1.0);
    obs[129] = boss_to_altar;
    obs[130] = boss_to_center.clamp(0.0, 1.0);
    obs[131] = (director_spent / 100000.0).clamp(0.0, 1.0);
    let valid_altar_slots = director_altar_slot_mask(world);
    obs[138] = (valid_altar_slots.iter().filter(|&&valid| valid).count() as f32
        / DIRECTOR_ALTAR_SLOT_COUNT as f32)
        .clamp(0.0, 1.0);

    let mut total_hp = 0.0f32;
    let mut lowest_hp = 1.0f32;
    let mut living_players = 0usize;
    let mut centroid = Vec2::ZERO;
    if world.boss_health.is_alive() {
        let hp = (world.boss_health.current / world.boss_health.max.max(1.0)).clamp(0.0, 1.0);
        total_hp += hp;
        lowest_hp = lowest_hp.min(hp);
        living_players += 1;
        centroid += world.boss_pos;
    }
    for player in world.coop_bosses.iter().filter(|player| player.boss_health.is_alive()) {
        let hp = (player.boss_health.current / player.boss_health.max.max(1.0)).clamp(0.0, 1.0);
        total_hp += hp;
        lowest_hp = lowest_hp.min(hp);
        living_players += 1;
        centroid += player.boss_pos;
    }
    obs[139] = (world.boss_player_count() as f32 / 8.0).clamp(0.0, 1.0);
    obs[140] = (living_players as f32 / world.boss_player_count().max(1) as f32).clamp(0.0, 1.0);
    obs[141] = if living_players > 0 { total_hp / living_players as f32 } else { 0.0 };
    obs[142] = if living_players > 0 { lowest_hp } else { 0.0 };
    let centroid = if living_players > 0 { centroid / living_players as f32 } else { boss_pos };
    obs[143] = ((centroid.x - boss_pos.x) / MAP_WIDTH).clamp(-1.0, 1.0);
    obs[144] = ((centroid.y - boss_pos.y) / MAP_HEIGHT).clamp(-1.0, 1.0);

    // Индивидуальные позиции/состояния команды на стеке без динамических аллокаций
    let mut team: [(Vec2, f32, bool); 8] = [(Vec2::ZERO, 0.0, false); 8];
    let mut team_count = 0usize;
    team[team_count] = (
        world.boss_pos,
        (world.boss_health.current / world.boss_health.max.max(1.0)).clamp(0.0, 1.0),
        world.boss_health.is_alive(),
    );
    team_count += 1;
    for player in &world.coop_bosses {
        if team_count < 8 {
            let hp = (player.boss_health.current / player.boss_health.max.max(1.0)).clamp(0.0, 1.0);
            team[team_count] = (player.boss_pos, hp, player.boss_health.is_alive());
            team_count += 1;
        }
    }
    for (slot, (pos, hp, alive)) in team.iter().take(8).enumerate() {
        let base = 145 + slot * 4;
        obs[base] = ((pos.x - boss_pos.x) / MAP_WIDTH).clamp(-1.0, 1.0);
        obs[base + 1] = ((pos.y - boss_pos.y) / MAP_HEIGHT).clamp(-1.0, 1.0);
        obs[base + 2] = *hp;
        obs[base + 3] = if *alive { 1.0 } else { 0.0 };
    }

    obs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::combat::Health;
    use crate::ecs::world::StructureEntity;

    #[test]
    fn test_boss_altar_and_obelisk_sensors_and_inter_distances() {
        let mut world = World::new();
        world.boss_pos = Vec2::new(1000.0, 1000.0);
        world.structures.clear();

        // Ставим Алтарь Крови на (1300, 1400)
        let altar_pos = Vec2::new(1300.0, 1400.0);
        world.structures.push(StructureEntity {
            id: 101,
            pos: altar_pos,
            health: Health::new(500.0),
            resistances: Default::default(),
            faction: Faction::Director,
            radius: 40.0,
            structure_type: crate::content::structures::STRUCTURE_ALTAR,
            pulse_timer: 0.0,
            spawn_timer: 0.0,
            lifetime: 0.0,
            max_lifetime: 0.0,
            is_active: true,
            state_timer: 0.0,
        });

        // Ставим Обелиск на (1600, 1800)
        let obelisk_pos = Vec2::new(1600.0, 1800.0);
        world.structures.push(StructureEntity {
            id: 102,
            pos: obelisk_pos,
            health: Health::new(2000.0),
            resistances: Default::default(),
            faction: Faction::Director,
            radius: 75.0,
            structure_type: crate::content::structures::STRUCTURE_OBELISK,
            pulse_timer: 0.0,
            spawn_timer: 0.0,
            lifetime: 0.0,
            max_lifetime: 0.0,
            is_active: true,
            state_timer: 15.0, // Активный разлом, 15с до прорыва
        });

        let obs = extract_boss_observations(&world);

        // 1. Ближайший алтарь [150..156]:
        assert_eq!(obs[150], 1.0, "Алтарь должен быть обнаружен");
        let expected_altar_x = (altar_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
        let expected_altar_y = (altar_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);
        assert!((obs[151] - expected_altar_x).abs() < 1e-5, "Точный X алтаря");
        assert!((obs[152] - expected_altar_y).abs() < 1e-5, "Точный Y алтаря");

        let boss_to_altar_dist = altar_pos.distance(world.boss_pos); // sqrt(300^2 + 400^2) = 500
        let expected_dist_norm = (boss_to_altar_dist / MAP_DIAGONAL).clamp(0.0, 1.0);
        assert!((obs[153] - expected_dist_norm).abs() < 1e-4, "Расстояние от босса до алтаря");

        // 2. Обелиск [174..179]:
        assert_eq!(obs[174], 1.0, "Обелиск должен присутствовать");
        assert!((obs[175] - obelisk_pos.x / MAP_WIDTH).abs() < 1e-5, "Точный X обелиска");
        assert!((obs[176] - obelisk_pos.y / MAP_HEIGHT).abs() < 1e-5, "Точный Y обелиска");
        let expected_obelisk_dist = obelisk_pos.distance(world.boss_pos); // sqrt(600^2 + 800^2) = 1000
        assert!((obs[177] - expected_obelisk_dist / MAP_DIAGONAL).abs() < 1e-4, "Расстояние босс-обелиск");

        // 3. Активный разлом [194..199]:
        assert_eq!(obs[194], 1.0, "Активный разлом должен быть обнаружен");
        assert!((obs[195] - 15.0 / 20.0).abs() < 1e-5, "Таймер прорыва разлома");
        assert!((obs[198] - expected_obelisk_dist / MAP_DIAGONAL).abs() < 1e-4, "Расстояние босс-разлом");

        // 4. Топологическая сеть расстояний:
        // Двудольная матрица: Алтарь 0 ↔ Обелиск 0 [индекс 205]:
        let altar_to_obelisk_dist = obelisk_pos.distance(altar_pos); // sqrt(300^2 + 400^2) = 500
        assert!((obs[205] - altar_to_obelisk_dist / MAP_DIAGONAL).abs() < 1e-4, "Расстояние алтарь-обелиск в матрице");

        // Маршрутный коридор к Активному Разлому: Altar 0 ↔ Rift [индекс 227]:
        assert!((obs[227] - altar_to_obelisk_dist / MAP_DIAGONAL).abs() < 1e-4, "Расстояние алтарь-разлом в коридоре");
    }

    #[test]
    fn test_altar_adjacent_placement_and_wall_barricade() {
        let mut world = World::new();
        world.structures.clear();

        // 1. Ставим Алтарь в точку (550, 550) - сетка 100x100
        let altar1 = crate::content::structures::snap_altar_to_grid(Vec2::new(550.0, 550.0));
        assert!(crate::content::structures::can_place_blood_altar(&world, altar1));
        crate::content::structures::spawn_blood_altar(&mut world, altar1);

        // 2. Ставим второй Алтарь в СОСЕДНЮЮ клетку 100x100 (650, 550) - соприкасаются гранями, расстояние ровно 100px!
        let altar2 = crate::content::structures::snap_altar_to_grid(Vec2::new(650.0, 550.0));
        assert!(crate::content::structures::can_place_blood_altar(&world, altar2), "Алтарь 100x100 должен ставиться вплотную к другому алтарю");

        // 3. Попытка наложить алтарь со сдвигом 50px (перекрытие) должна блокироваться:
        let overlapping_altar = crate::content::structures::snap_altar_to_grid(Vec2::new(550.0, 550.0));
        assert!(!crate::content::structures::can_place_blood_altar(&world, overlapping_altar), "Нельзя накладывать алтари друг на друга");

        // 4. Ставим Стену в соседнюю клетку 50x50 (475, 525) - расстояние до центра алтаря ровно 75px (50 + 25):
        // Баррикада алтаря вплотную!
        let wall = crate::content::structures::snap_to_build_grid(Vec2::new(475.0, 525.0));
        assert!(crate::content::structures::can_place_wall(&world, wall), "Стена 50x50 должна ставиться вплотную к алтарю 100x100");

        // 5. Но стена поверх алтаря (525, 525) - расстояние 25px < 70px - должна блокироваться:
        let overlapping_wall = crate::content::structures::snap_to_build_grid(Vec2::new(525.0, 525.0));
        assert!(!crate::content::structures::can_place_wall(&world, overlapping_wall), "Нельзя ставить стену поверх алтаря");
    }

    #[test]
    fn test_altar_economy_and_spawn_scaling() {
        // Скелеты (тир 1, count 8): интервал призыва ровно 1.0с
        let spawn_t1 = crate::content::structures::altar_spawn_interval(1);
        assert!((spawn_t1 - 1.0).abs() < 1e-5, "Скелеты спавнятся раз в 1.0с при count=8");

        // Жнец (тир 8, count 1): интервал 8.0с
        let spawn_t8 = crate::content::structures::altar_spawn_interval(8);
        assert!((spawn_t8 - 8.0).abs() < 1e-5, "Жнец смерти спавнится раз в 8.0с при count=1");

        // Стоимость алтаря растет с ценой моба:
        let cost_t1 = crate::content::structures::altar_cost(1);
        let cost_t2 = crate::content::structures::altar_cost(2);
        let cost_t8 = crate::content::structures::altar_cost(8);
        assert!(cost_t2 > cost_t1);
        assert!(cost_t8 > cost_t2);

        // Кулдаун алтарей: Т1 = 14с, Т2 = 21с (+7с), Т8 = 14 + 7*7 = 63с
        let cd_t1 = crate::content::structures::altar_cooldown(1);
        let cd_t2 = crate::content::structures::altar_cooldown(2);
        let cd_t8 = crate::content::structures::altar_cooldown(8);
        assert_eq!(cd_t1, 14.0, "Тир 1 (скелеты) имеет КД ровно 14.0 секунд");
        assert_eq!(cd_t2, 21.0, "Тир 2 (гончие) имеет КД ровно 21.0 секунду (+7с)");
        assert_eq!(cd_t8, 63.0, "Тир 8 (жнец) имеет КД ровно 63.0 секунды (14 + 7*7)");
    }
}
