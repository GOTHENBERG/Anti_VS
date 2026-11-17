use macroquad::prelude::*;
use crate::content::structures::BLOOD_ALTAR_COST;
use crate::ecs::components::combat::Faction;
use crate::ecs::components::inventory::{WeaponType, PassiveType};
use crate::ecs::world::World;
use crate::core::{MAP_WIDTH, MAP_HEIGHT};
use crate::ai::neural_net::INPUT_SIZE;

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
/// СЕНСОРЫ БОССА — 150 входов
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
/// [135..150] Цели: ближайший алтарь (вектор, HP, ETA), магнит, кристалл душ,
///            флаг атаки алтаря, время матча, недавний урон, дистанция до элиты.
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

    let mut buf: Vec<usize> = Vec::with_capacity(64);

    // --- 1. Ядро 5×5: мобы ---
    world.spatial_grid.query_nearby_into(boss_pos, HALF + 8.0, &mut buf);
    for &idx in &buf {
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
    world.spatial_grid.query_nearby_into(boss_pos, 600.0, &mut buf);
    let mut mobs_160 = 0usize;
    let mut ranged_400 = 0usize;
    let mut elites_500 = 0usize;
    let mut nearest_elite_dist = f32::MAX;

    for &idx in &buf {
        if let Some(mob) = world.mobs.get(idx) {
            if !mob.health.is_alive() {
                continue;
            }
            let diff = mob.pos - boss_pos;
            let dist = diff.length();
            if dist <= 160.0 {
                mobs_160 += 1;
            }
            let is_ranged = matches!(mob.tier, 3 | 11 | 16);
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
    for i in 0..16 {
        obs[100 + i] = (outer[i] / 6.0).clamp(0.0, 1.0);
    }

    // --- 3. Внутреннее состояние Босса [116..124] ---
    obs[116] = (world.boss_health.current / world.boss_health.max.max(1.0)).clamp(0.0, 1.0);
    obs[117] = (world.boss_resistances.flat_armor / 60.0).clamp(0.0, 1.0);
    obs[118] = if world.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 };
    obs[119] = (world.boss_dash_cooldown / 7.0).clamp(0.0, 1.0);
    obs[120] = (world.boss_inventory.level as f32 / 50.0).clamp(0.0, 1.0);
    let xp_next = world.boss_inventory.xp_to_next_level.max(1.0);
    obs[121] = (world.boss_inventory.current_xp / xp_next).clamp(0.0, 1.0);
    obs[122] = (world.boss_vel.length() / max_speed).clamp(0.0, 1.5);
    obs[123] = (world.boss_still_time / 1.2).clamp(0.0, 1.0);

    // --- 4. Экономика Директора [124..130] ---
    if let Some(dir) = world.directors.first() {
        obs[124] = (dir.points / 1000.0).clamp(0.0, 5.0);
        obs[125] = (dir.income_rate / 50.0).clamp(0.0, 3.0);
        obs[126] = (world.structures.iter().filter(|s| s.health.is_alive()).count() as f32 / 10.0).clamp(0.0, 1.0);
        obs[127] = (dir.grade_fury as f32 / 20.0).clamp(0.0, 2.0);
        obs[128] = (dir.grade_carapace as f32 / 20.0).clamp(0.0, 2.0);
        obs[129] = (dir.grade_pestilence as f32 / 20.0).clamp(0.0, 2.0);
    }

    // --- 5. Агрегаты угроз [130..135] ---
    let mut best_tti = f32::MAX;
    let mut best_lateral = 0.0f32;
    for proj in &world.projectiles {
        if proj.faction != Faction::Director || proj.lifetime <= 0.0 || proj.pierce == 0 {
            continue;
        }
        let diff = boss_pos - proj.pos;
        let dist = diff.length();
        let speed = proj.vel.length().max(1.0);
        let tti = dist / speed;
        if tti < best_tti {
            best_tti = tti;
            let dir = proj.vel / speed;
            let lateral = dir.x * diff.y - dir.y * diff.x;
            best_lateral = (lateral / dist.max(20.0)).clamp(-1.0, 1.0);
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
    let mut closest_altar: Option<Vec2> = None;
    let mut closest_altar_dist_sq = f32::MAX;
    let mut closest_altar_hp = 0.0f32;
    let mut altars_under_attack = 0.0f32;
    for st in &world.structures {
        if !st.health.is_alive() {
            continue;
        }
        let d_sq = st.pos.distance_squared(boss_pos);
        if st.health.current < st.health.max * 0.99 {
            altars_under_attack = 1.0;
        }
        if d_sq < closest_altar_dist_sq {
            closest_altar_dist_sq = d_sq;
            closest_altar = Some(st.pos);
            closest_altar_hp = (st.health.current / st.health.max.max(1.0)).clamp(0.0, 1.0);
        }
    }

    if let Some(a_pos) = closest_altar {
        let diff = a_pos - boss_pos;
        obs[135] = (diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
        obs[136] = (diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);
        obs[137] = closest_altar_hp;
        obs[138] = (diff.length() / max_speed / 20.0).clamp(0.0, 1.0);
    } else {
        obs[135] = 0.0;
        obs[136] = 0.0;
        obs[137] = 0.0;
        obs[138] = 1.0; // Максимальная задержка / недостижимо (как и для магнита)
    }

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

    obs[139] = if closest_magnet.is_some() { 1.0 } else { 0.0 };
    if let Some(m_pos) = closest_magnet {
        let diff = m_pos - boss_pos;
        obs[140] = (diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
        obs[141] = (diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);
        obs[142] = (diff.length() / max_speed / 20.0).clamp(0.0, 1.0);
    } else {
        obs[142] = 1.0;
    }
    if let Some(g_pos) = closest_gem {
        let diff = g_pos - boss_pos;
        obs[143] = (diff.x / 450.0).clamp(-1.0, 1.0);
        obs[144] = (diff.y / 450.0).clamp(-1.0, 1.0);
    }

    obs[145] = altars_under_attack;
    obs[146] = (world.game_time / 1020.0).clamp(0.0, 1.5);
    obs[147] = (world.boss_inventory.damage_stats.melee_recent / 200.0).clamp(0.0, 1.0);
    obs[148] = (world.boss_inventory.damage_stats.ranged_recent / 200.0).clamp(0.0, 1.0);
    obs[149] = if nearest_elite_dist.is_finite() {
        (nearest_elite_dist / 600.0).clamp(0.0, 1.0)
    } else {
        1.0
    };

    obs
}

/// ============================================================================
/// СЕНСОРЫ ДИРЕКТОРА — 132 входа
/// ============================================================================
pub const DIRECTOR_INPUT_SIZE: usize = crate::ai::neural_net::DIRECTOR_INPUT_SIZE;

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
    }
}

/// Преобразует тип пассивки/реликвии Босса в фиксированный индекс [0..27]
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
    }
}

/// 132-мерный вектор Директора.
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
pub fn extract_director_observations(world: &World, sim_time: f32) -> [f32; DIRECTOR_INPUT_SIZE] {
    let mut obs = [0.0f32; DIRECTOR_INPUT_SIZE];
    let boss_pos = world.boss_pos;

    // 1. [0..9] Карта и кинематика Босса
    obs[0] = (boss_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
    obs[1] = (boss_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);
    obs[2] = ((MAP_WIDTH - boss_pos.x) / MAP_WIDTH).clamp(0.0, 1.0);
    obs[3] = (boss_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
    obs[4] = ((MAP_HEIGHT - boss_pos.y) / MAP_HEIGHT).clamp(0.0, 1.0);
    obs[5] = (boss_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);
    obs[6] = (world.boss_vel.x / 300.0).clamp(-1.5, 1.5);
    obs[7] = (world.boss_vel.y / 300.0).clamp(-1.5, 1.5);
    obs[8] = (world.boss_vel.length() / 300.0).clamp(0.0, 2.0);

    // 2. [9..18] HP, XP и действия Босса
    obs[9] = (world.boss_health.current / world.boss_health.max.max(1.0)).clamp(0.0, 1.0);
    obs[10] = (world.boss_health.max / 2000.0).clamp(0.1, 2.5);
    obs[11] = (world.boss_inventory.level as f32 / 50.0).clamp(0.0, 2.0);
    let next_xp = world.boss_inventory.xp_to_next_level.max(1.0);
    obs[12] = (world.boss_inventory.current_xp / next_xp).clamp(0.0, 1.0);
    obs[13] = (next_xp / 500.0).clamp(0.05, 5.0);
    obs[14] = if world.boss_invulnerable_timer > 0.0 { 1.0 } else { 0.0 };
    obs[15] = if world.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 };
    obs[16] = if world.boss_still_time >= 1.2 { 1.0 } else { 0.0 };
    obs[17] = if world.boss_poison_timer > 0.0 { 1.0 } else { 0.0 };

    // 3. [18..39] Алтари: 4 ближайших, без сортировки всей карты
    let mut nearest: [Option<(usize, f32)>; 4] = [None; 4];
    let mut alive_altars = 0usize;
    let mut under_attack = 0usize;
    let mut closest_altar_pos = Vec2::ZERO;
    let mut closest_altar_dist_sq = f32::MAX;

    for (idx, st) in world.structures.iter().enumerate() {
        if !st.health.is_alive() {
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
        obs[39] = (dir.points / 1000.0).clamp(0.0, 10.0);
        obs[40] = (dir.income_rate / 50.0).clamp(0.0, 5.0);
        obs[41] = (dir.altar_cooldown / 60.0).clamp(0.0, 1.0);
        obs[42] = (dir.grade_fury as f32 / 20.0).clamp(0.0, 5.0);
        obs[43] = (dir.grade_carapace as f32 / 20.0).clamp(0.0, 5.0);
        obs[44] = (dir.grade_pestilence as f32 / 20.0).clamp(0.0, 5.0);
        obs[45] = (dir.total_points_spent / 50000.0).clamp(0.0, 10.0);

        director_points = dir.points;
        director_spent = dir.total_points_spent;
        altar_ready = dir.altar_cooldown <= 0.0 && dir.points >= BLOOD_ALTAR_COST;
        time_since_spawn = dir.time_since_last_spawn;
        grade_fury = dir.grade_fury;
        grade_carapace = dir.grade_carapace;
        grade_pestilence = dir.grade_pestilence;
        squad_cooldowns = dir.squad_cooldowns;
    }
    obs[46] = (world.mobs.len() as f32 / 500.0).clamp(0.0, 2.0);
    obs[47] = (sim_time / 1020.0).clamp(0.0, 1.5);

    // 5-6. [48..88] Армия: тиры + пространственная карта 4×4 + агрегаты
    let mut tier_counts = [0usize; 16];
    let mut army_grid = [[0.0f32; 4]; 4];
    let mut total_mobs = 0usize;
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
    let mut archetype_mask = 0u8;

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
        archetype_mask |= 1 << (weapon_type_to_index(w.kind) % 6);
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
    obs[103] = (archetype_mask.count_ones() as f32 / 6.0).clamp(0.0, 1.0);

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
        if crate::ecs::components::inventory::BossInventory::get_evolution_passive_for_weapon(
            world.boss_inventory.weapons.first().map(|w| w.kind).unwrap_or(WeaponType::WindBlade),
        ) == Some(p.kind)
        {
            evolution_pairs += 1;
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
    let mut nearby_buf: Vec<usize> = Vec::with_capacity(64);
    world.spatial_grid.query_nearby_into(boss_pos, 250.0, &mut nearby_buf);
    for &m_idx in &nearby_buf {
        if let Some(m) = world.mobs.get(m_idx) {
            if m.health.is_alive() {
                mobs_within_250 += 1;
            }
        }
    }

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
    obs[114] = if world.boss_dash_cooldown <= 0.0 { 1.0 } else { 0.0 };
    obs[115] = if world.boss_invulnerable_timer > 0.0 { 1.0 } else { 0.0 };
    obs[116] = (world.boss_health.current / world.boss_health.max.max(1.0)).clamp(0.0, 1.0);
    obs[117] = (recent_damage / 500.0).clamp(0.0, 1.0);
    obs[118] = (near_boss as f32 * inv).clamp(0.0, 1.0);
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
    obs[125] = (world.boss_inventory.level as f32 / 50.0).clamp(0.0, 2.0);
    obs[126] = (world.boss_inventory.current_xp / next_xp).clamp(0.0, 1.0);
    obs[127] = (world.directors.first().map_or(0.0, |d| d.income_rate) / 50.0).clamp(0.0, 5.0);
    obs[128] = (mobs_within_250 as f32 / 100.0).clamp(0.0, 1.0);
    obs[129] = boss_to_altar;
    obs[130] = boss_to_center.clamp(0.0, 1.0);
    obs[131] = (director_spent / 100000.0).clamp(0.0, 1.0);

    obs
}
