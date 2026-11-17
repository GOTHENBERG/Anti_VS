use macroquad::prelude::*;
use crate::ecs::world::World;
use crate::ecs::components::combat::Faction;
use crate::ecs::components::inventory::{WeaponType, PassiveType};
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
        let cell_y = boss_pos.y - HALF_GRID_PX + (r as f32 + 0.5) * CELL_SIZE;
        for c in 0..GRID_SIZE {
            if r == 5 && c == 5 {
                continue; // Позиция самого Босса
            }
            let cell_x = boss_pos.x - HALF_GRID_PX + (c as f32 + 0.5) * CELL_SIZE;
            let is_wall = cell_x < 0.0 || cell_x > MAP_WIDTH || cell_y < 0.0 || cell_y > MAP_HEIGHT;
            let wall_val = if is_wall { 1.0 } else { 0.0 };

            let mob_val = (mob_grid[r][c] / 5.0).clamp(0.0, 1.0);
            let proj_val = proj_grid[r][c].clamp(0.0, 1.0);
            let spore_val = spore_grid[r][c].clamp(0.0, 1.0);
            // Комплексная опасность ячейки: сочетание толпы, летящих пуль, чумных спор и стен карты
            obs[out_idx] = (mob_val * 0.35 + proj_val * 0.70 + spore_val * 0.60 + wall_val * 0.85).clamp(0.0, 1.0);
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

    // Тактический сенсор: вектор на ближайший кристалл душ (Soul Gem) [148..150]
    let mut closest_gem_diff = Vec2::ZERO;
    let mut min_gem_dist_sq = 450.0 * 450.0;
    for gem in &world.gems {
        let dist_sq = gem.pos.distance_squared(boss_pos);
        if dist_sq < min_gem_dist_sq {
            min_gem_dist_sq = dist_sq;
            closest_gem_diff = gem.pos - boss_pos;
        }
    }
    obs[148] = (closest_gem_diff.x / 450.0).clamp(-1.0, 1.0);
    obs[149] = (closest_gem_diff.y / 450.0).clamp(-1.0, 1.0);

    obs
}

pub const DIRECTOR_INPUT_SIZE: usize = 114;

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
    }
}

/// Извлекает 114-мерный вектор сенсорного восприятия Директора (Director Perception Vector):
/// 1. [0..9]   Карта и кинематика Босса (позиция X/Y, дистанции до 4 стен арены, вектор скорости, скорость)
/// 2. [9..16]  Жизненный статус и действия Босса (% HP, макс HP, уровень, рывок/неуязвимость, откат рывка, медитация, отравление)
/// 3. [16..37] Алтари Крови (общее число, топ-4 ближайших алтаря: флаг наличия, вектор dx/dy, дистанция, % HP)
/// 4. [37..46] Экономика и армия Директора (очки душ, темп дохода, кд алтаря, уровни Ярости, Панциря, Чумных спор, мобы на арене, потрачено душ, время матча)
/// 5. [46..86] Арсенал Босса (40 видов оружия и эволюций: нормализованный уровень 0.0..1.0)
/// 6. [86..114] Пассивки и реликвии Босса (28 видов предметов: нормализованный уровень 0.0..1.0)
pub fn extract_director_observations(world: &World, sim_time: f32) -> [f32; DIRECTOR_INPUT_SIZE] {
    let mut obs = [0.0f32; DIRECTOR_INPUT_SIZE];
    let boss_pos = world.boss_pos;

    // 1. [0..9] Карта и кинематика Босса
    obs[0] = (boss_pos.x / MAP_WIDTH).clamp(0.0, 1.0);
    obs[1] = (boss_pos.y / MAP_HEIGHT).clamp(0.0, 1.0);
    obs[2] = (boss_pos.x / MAP_WIDTH).clamp(0.0, 1.0); // дистанция до левой стены
    obs[3] = ((MAP_WIDTH - boss_pos.x) / MAP_WIDTH).clamp(0.0, 1.0); // до правой стены
    obs[4] = (boss_pos.y / MAP_HEIGHT).clamp(0.0, 1.0); // до верхней стены
    obs[5] = ((MAP_HEIGHT - boss_pos.y) / MAP_HEIGHT).clamp(0.0, 1.0); // до нижней стены
    obs[6] = (world.boss_vel.x / 300.0).clamp(-1.5, 1.5);
    obs[7] = (world.boss_vel.y / 300.0).clamp(-1.5, 1.5);
    obs[8] = (world.boss_vel.length() / 300.0).clamp(0.0, 2.0);

    // 2. [9..16] Жизненный статус и действия Босса
    obs[9] = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0);
    obs[10] = (world.boss_health.max / 2000.0).clamp(0.1, 2.5);
    obs[11] = (world.boss_inventory.level as f32 / 50.0).clamp(0.0, 2.0);
    obs[12] = if world.boss_invulnerable_timer > 0.0 { 1.0 } else { 0.0 }; // Активный рывок (неуязвим прямо сейчас)
    obs[13] = (world.boss_dash_cooldown / 7.0).clamp(0.0, 1.0); // Готовность рывка (0.0 = готов)
    obs[14] = if world.boss_still_time >= 1.2 { 1.0 } else { (world.boss_still_time / 1.2).clamp(0.0, 1.0) }; // Медитация
    obs[15] = if world.boss_poison_timer > 0.0 { (world.boss_poison_timer / 2.5).clamp(0.0, 1.0) } else { 0.0 }; // Отравление

    // 3. [16..37] Алтари Крови
    obs[16] = (world.structures.len() as f32 / 10.0).clamp(0.0, 1.0);

    // Сортировка живых алтарей по расстоянию до босса
    let mut altars_sorted: Vec<&crate::ecs::world::StructureEntity> = world
        .structures
        .iter()
        .filter(|s| s.health.is_alive())
        .collect();
    altars_sorted.sort_by(|a, b| {
        let da = a.pos.distance_squared(boss_pos);
        let db = b.pos.distance_squared(boss_pos);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });

    let map_diag = (MAP_WIDTH * MAP_WIDTH + MAP_HEIGHT * MAP_HEIGHT).sqrt();
    for i in 0..4 {
        let base = 17 + i * 5;
        if let Some(st) = altars_sorted.get(i) {
            obs[base] = 1.0; // Алтарь существует
            let diff = st.pos - boss_pos;
            obs[base + 1] = (diff.x / MAP_WIDTH).clamp(-1.0, 1.0);
            obs[base + 2] = (diff.y / MAP_HEIGHT).clamp(-1.0, 1.0);
            obs[base + 3] = (diff.length() / map_diag).clamp(0.0, 1.0);
            obs[base + 4] = (st.health.current / st.health.max).clamp(0.0, 1.0);
        } else {
            obs[base] = 0.0;
            obs[base + 1] = 0.0;
            obs[base + 2] = 0.0;
            obs[base + 3] = 1.0;
            obs[base + 4] = 0.0;
        }
    }

    // 4. [37..46] Экономика и армия Директора
    if let Some(dir) = world.directors.first() {
        obs[37] = (dir.points / 1000.0).clamp(0.0, 10.0);
        obs[38] = (dir.income_rate / 50.0).clamp(0.0, 5.0);
        obs[39] = (dir.altar_cooldown / 60.0).clamp(0.0, 1.0);
        obs[40] = (dir.grade_fury as f32 / 20.0).clamp(0.0, 5.0);
        obs[41] = (dir.grade_carapace as f32 / 20.0).clamp(0.0, 5.0);
        obs[42] = (dir.grade_pestilence as f32 / 20.0).clamp(0.0, 5.0);
        obs[43] = (dir.total_points_spent / 50000.0).clamp(0.0, 10.0);
    }
    obs[44] = (world.mobs.len() as f32 / 500.0).clamp(0.0, 2.0);
    obs[45] = (sim_time / 1020.0).clamp(0.0, 1.5);

    // 5. [46..86] Оружия и скиллы Босса (40 слотов)
    for w in &world.boss_inventory.weapons {
        let idx = weapon_type_to_index(w.kind);
        if idx < 40 {
            obs[46 + idx] = (w.level as f32 / 5.0).clamp(0.2, 1.6);
        }
    }

    // 6. [86..114] Пассивки и реликвии Босса (28 слотов)
    for p in &world.boss_inventory.passives {
        let idx = passive_type_to_index(p.kind);
        if idx < 28 {
            obs[86 + idx] = (p.level as f32 / 5.0).clamp(0.2, 1.6);
        }
    }

    obs
}
