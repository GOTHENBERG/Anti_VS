use macroquad::prelude::*;
use crate::core::math::{calculate_attract_force, calculate_repel_force};
use crate::ecs::components::inventory::{
    BossInventory, PassiveType, PerkChoice, WeaponType, MAX_PASSIVE_LEVEL, MAX_PASSIVE_SLOTS,
    MAX_WEAPON_LEVEL, MAX_WEAPON_SLOTS,
};
use crate::ecs::world::World;

pub fn update_boss_ai(world: &mut World, dt: f32) {
    if world.boss_dash_cooldown > 0.0 {
        world.boss_dash_cooldown = (world.boss_dash_cooldown - dt).max(0.0);
    }
    if world.boss_invulnerable_timer > 0.0 {
        world.boss_invulnerable_timer = (world.boss_invulnerable_timer - dt).max(0.0);
    }
    if world.boss_inventory.perk_announcement_timer > 0.0 {
        world.boss_inventory.perk_announcement_timer = (world.boss_inventory.perk_announcement_timer - dt).max(0.0);
    }

    // Затухание недавнего урона
    world.boss_inventory.melee_damage_recent *= 1.0 - (dt * 0.25);
    world.boss_inventory.ranged_damage_recent *= 1.0 - (dt * 0.25);

    let boss_pos = world.boss_pos;
    let mut total_force = Vec2::ZERO;

    // Извлекаем сенсорный вектор (36 входов) и пропускаем через глубокую нейросеть
    let nn_steering_bias;
    let (repel_weight, gem_pull_weight, altar_aggression, sniper_aggression, dash_threshold, retreat_hp, neural_dash_trigger) =
        if let Some(ref model) = world.boss_model {
            if let Some(ref net) = model.neural_net {
                let obs = crate::ai::sensors::extract_boss_observations(world);
                let nn_out = net.forward(&obs);
                nn_steering_bias = Vec2::new(nn_out[0], nn_out[1]) * 120.0;
                (
                    200.0 + nn_out[2] * 500.0,
                    100.0 + nn_out[3] * 350.0,
                    150.0 + nn_out[4] * 400.0,
                    150.0 + nn_out[5] * 400.0,
                    (8.0 + nn_out[6] * 10.0) as usize,
                    0.15 + nn_out[7] * 0.35,
                    nn_out[6] > 0.70,
                )
            } else {
                nn_steering_bias = Vec2::ZERO;
                (
                    model.mob_repulsion_weight,
                    model.gem_attraction_weight,
                    model.altar_siege_aggression,
                    model.sniper_hunt_aggression,
                    model.dash_danger_threshold,
                    model.retreat_hp_threshold,
                    false,
                )
            }
        } else {
            nn_steering_bias = Vec2::ZERO;
            (440.0, 180.0, 260.0, 310.0, 13, 0.32, false)
        };

    total_force += nn_steering_bias;

    let mobs_in_close_radius = world.spatial_grid.query_nearby(boss_pos, 90.0)
        .iter()
        .filter(|&&idx| {
            if let Some(mob) = world.mobs.get(idx) {
                mob.pos.distance_squared(boss_pos) < 90.0 * 90.0
            } else {
                false
            }
        })
        .count();

    let hp_ratio = world.boss_health.current / world.boss_health.max;
    let is_critical_hp = hp_ratio < retreat_hp;

    // 1. Потенциальные поля: Отталкивание от мобов в радиусе 180 пикселей (закон обратных квадратов)
    // Используем пространственную сетку для мгновенного запроса соседей за O(1)!
    let nearby_mob_indices = world.spatial_grid.query_nearby(boss_pos, 180.0);
    for idx in nearby_mob_indices {
        if let Some(mob) = world.mobs.get(idx) {
            let repel = calculate_repel_force(boss_pos, mob.pos, 180.0, repel_weight);
            total_force += repel;
        }
    }

    // 2. Потенциальные поля: Притяжение к кристаллам душ
    let mut closest_gem_dist_sq = 260.0 * 260.0;
    let mut gem_pull = Vec2::ZERO;
    for gem in &world.gems {
        let dist_sq = gem.pos.distance_squared(boss_pos);
        if dist_sq < closest_gem_dist_sq {
            closest_gem_dist_sq = dist_sq;
            let strength = if gem.is_magnet { gem_pull_weight * 1.8 } else { gem_pull_weight };
            gem_pull = calculate_attract_force(boss_pos, gem.pos, 260.0, strength);
        }
    }
    total_force += gem_pull;

    // 2.1 Стратегическая цель: Атака и разрушение Алтарей Крови
    // Если босс достаточно здоров (HP > retreat_hp) и рядом нет плотной толпы,
    // он целенаправленно выдвигается на алтарь для его разрушения
    let is_safe_to_siege = hp_ratio > retreat_hp && mobs_in_close_radius < 8;
    if is_safe_to_siege {
        let mut closest_altar_dist_sq = 550.0 * 550.0;
        let mut altar_force = Vec2::ZERO;
        for st in &world.structures {
            let dist_sq = st.pos.distance_squared(boss_pos);
            if dist_sq < closest_altar_dist_sq {
                closest_altar_dist_sq = dist_sq;
                altar_force = calculate_attract_force(boss_pos, st.pos, 550.0, altar_aggression);
            }
        }
        total_force += altar_force;
    }

    // 2.2 Тактическая контратака против дальников (Культисты-стрелки)
    // Если на арене скопились стрелки (Тир 3), босс не пятится в угол,
    // а агрессивно фланкирует и сокращает дистанцию к их кластеру!
    let nearby_cultists: Vec<Vec2> = world.spatial_grid.query_nearby(boss_pos, 420.0)
        .iter()
        .filter_map(|&idx| world.mobs.get(idx))
        .filter(|m| m.tier == 3)
        .map(|m| m.pos)
        .collect();

    if nearby_cultists.len() >= 2 && hp_ratio > retreat_hp && mobs_in_close_radius < 10 {
        let mut center = Vec2::ZERO;
        for cp in &nearby_cultists {
            center += *cp;
        }
        center /= nearby_cultists.len() as f32;

        let to_cluster = center - boss_pos;
        let dist = to_cluster.length();
        if dist > 70.0 {
            let aggression = if world.boss_inventory.ranged_damage_recent > 20.0 { sniper_aggression * 1.15 } else { sniper_aggression };
            let hunt_dir = to_cluster / dist;
            total_force += hunt_dir * aggression;
        }
    }

    // 3. Потенциальные поля: Плавное и заблаговременное отталкивание от границ карты (220 px)
    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;
    let wall_margin = 220.0;
    let wall_push = 650.0;

    if boss_pos.x < wall_margin {
        let factor = 1.0 - (boss_pos.x / wall_margin).clamp(0.0, 1.0);
        total_force.x += wall_push * factor * factor;
    } else if boss_pos.x > map_w - wall_margin {
        let factor = 1.0 - ((map_w - boss_pos.x) / wall_margin).clamp(0.0, 1.0);
        total_force.x -= wall_push * factor * factor;
    }

    if boss_pos.y < wall_margin {
        let factor = 1.0 - (boss_pos.y / wall_margin).clamp(0.0, 1.0);
        total_force.y += wall_push * factor * factor;
    } else if boss_pos.y > map_h - wall_margin {
        let factor = 1.0 - ((map_h - boss_pos.y) / wall_margin).clamp(0.0, 1.0);
        total_force.y -= wall_push * factor * factor;
    }

    // 4. Экстренный рывок: 8-лучевой Context Steering Dash
    // Триггер: в радиусе 90px > dash_threshold мобов ИЛИ HP < retreat_hp ИЛИ плотный обстрел дальников ИЛИ импульс нейросети
    let heavy_ranged_fire = world.boss_inventory.ranged_damage_recent > 40.0;
    let should_dash = (mobs_in_close_radius >= dash_threshold
        || (is_critical_hp && mobs_in_close_radius >= 3)
        || (heavy_ranged_fire && mobs_in_close_radius >= 4)
        || (neural_dash_trigger && mobs_in_close_radius >= 4))
        && world.boss_dash_cooldown <= 0.0;

    if should_dash {
        // Опрос по 8 радиальным лучам (шаг 45 градусов / PI / 4.0)
        let mut best_dir = Vec2::X;
        let mut min_danger = f32::MAX;

        for i in 0..8 {
            let angle = (i as f32) * std::f32::consts::PI / 4.0;
            let dir = Vec2::new(angle.cos(), angle.sin());
            let target_point = boss_pos + dir * 180.0;

            // Строгий штраф за рывок в сторону стены
            let mut wall_penalty = 0.0;
            if target_point.x < 120.0 || target_point.x > map_w - 120.0 {
                wall_penalty += 350.0;
            }
            if target_point.y < 120.0 || target_point.y > map_h - 120.0 {
                wall_penalty += 350.0;
            }

            // Плотность мобов вдоль луча
            let sector_mobs = world.spatial_grid.query_nearby(target_point, 110.0)
                .iter()
                .filter(|&&idx| {
                    if let Some(mob) = world.mobs.get(idx) {
                        mob.pos.distance_squared(target_point) < 110.0 * 110.0
                    } else {
                        false
                    }
                })
                .count() as f32;

            let total_danger = sector_mobs + wall_penalty;
            if total_danger < min_danger {
                min_danger = total_danger;
                best_dir = dir;
            }
        }

        // Мгновенный рывок на 180 пикселей с неуязвимостью на 0.25 сек!
        let old_pos = world.boss_pos;
        let dash_target = (boss_pos + best_dir * 180.0)
            .clamp(Vec2::new(60.0, 60.0), Vec2::new(map_w - 60.0, map_h - 60.0));

        world.boss_pos = dash_target;
        world.boss_invulnerable_timer = 0.25;
        world.boss_dash_cooldown = 7.0;

        // Визуальный эффект рывка
        world.spawn_particles(old_pos, 20, GOLD, 120.0);
        world.spawn_particles(dash_target, 20, YELLOW, 140.0);
        world.add_floating_text(dash_target, "РЫВОК (DASH)!".to_string(), YELLOW, 1.3);

        // При рывке уничтожаются вражеские снаряды на пути и наносится рассекающий урон
        for proj in &mut world.projectiles {
            if proj.faction == crate::ecs::components::combat::Faction::Director
                && proj.pos.distance_squared(dash_target) < 130.0 * 130.0
            {
                proj.lifetime = 0.0;
            }
        }

        let in_dash_slice = world.spatial_grid.query_nearby(dash_target, 90.0);
        for idx in in_dash_slice {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(dash_target) < 90.0 * 90.0 {
                    mob.health.take_damage(80.0);
                    if !mob.knockback_immune {
                        mob.knockback_vel += best_dir * 240.0;
                    }
                }
            }
        }
    } else {
        // Проверяем возможность тактической медитации (если в радиусе 220px нет врагов)
        let nearby_threats = world.spatial_grid.query_nearby(boss_pos, 220.0)
            .iter()
            .any(|&idx| {
                if let Some(mob) = world.mobs.get(idx) {
                    mob.pos.distance_squared(boss_pos) < 220.0 * 220.0
                } else {
                    false
                }
            });

        let hp_ratio = world.boss_health.current / world.boss_health.max;
        // Если вокруг безопасно и здоровье не полное (или босс уже начал медитировать) -> останавливается и медитирует!
        let wants_meditation = !nearby_threats && (hp_ratio < 0.98 || world.boss_still_time > 0.0);

        // Обычное передвижение по вектору сил
        let max_speed = 160.0;
        let desired_vel = if wants_meditation {
            Vec2::ZERO
        } else if total_force.length_squared() > 4.0 {
            total_force.clamp_length_max(max_speed)
        } else {
            Vec2::ZERO
        };

        // Плавная интерполяция скорости для органичного обтекания
        world.boss_vel = world.boss_vel.lerp(desired_vel, (dt * 10.0).min(1.0));
        world.boss_pos += world.boss_vel * dt;
    }

    // Координаты ограничены картой
    world.boss_pos.x = world.boss_pos.x.clamp(35.0, map_w - 35.0);
    world.boss_pos.y = world.boss_pos.y.clamp(35.0, map_h - 35.0);
}

/// Адаптивный выбор из 3 карт при Level-Up босса
pub fn handle_boss_level_up(world: &mut World) {
    let choices = generate_perk_choices(&world.boss_inventory);
    if choices.is_empty() {
        return;
    }

    // Оценка каждой карты с адаптивными весами в зависимости от полученного урона и здоровья
    let mut best_choice = choices[0].clone();
    let mut best_score = -1000.0;

    let melee_dmg = world.boss_inventory.melee_damage_recent;
    let ranged_dmg = world.boss_inventory.ranged_damage_recent;
    let hp_ratio = world.boss_health.current / world.boss_health.max;

    for choice in &choices {
        let mut score = 50.0;

        match choice {
            PerkChoice::EvolveWeapon { to, .. } => {
                // Эволюция — абсолютный приоритет!
                score += 300.0;
                match to {
                    WeaponType::BladeStorm | WeaponType::Hellfire => {
                        if melee_dmg > 50.0 {
                            score += 50.0;
                        }
                    }
                    WeaponType::DeathRay | WeaponType::BlizzardStorm => {
                        if ranged_dmg > 50.0 {
                            score += 50.0;
                        }
                    }
                    WeaponType::DivineShield => {
                        if hp_ratio < 0.4 {
                            score += 80.0; // Вампиризм при низком здоровье!
                        }
                    }
                    _ => {}
                }
            }
            PerkChoice::UpgradeWeapon(kind) => {
                // Били вблизи -> берёт круговой урон (FireAura, WindBlade, HolyHammer)
                if matches!(kind, WeaponType::FireAura | WeaponType::WindBlade | WeaponType::HolyHammer) {
                    score += (melee_dmg * 0.4).min(45.0);
                }
                // Били издали -> берёт дальний бой и контроль (MagicWand, IceShards)
                if matches!(kind, WeaponType::MagicWand | WeaponType::IceShards) {
                    score += (ranged_dmg * 0.4).min(45.0);
                }
                // Если оружие высокого уровня (6-7), ускоряем его до 8 для эволюции!
                if let Some(w) = world.boss_inventory.weapons.iter().find(|w| w.kind == *kind) {
                    score += (w.level as f32) * 8.0;
                }
            }
            PerkChoice::UpgradePassive(kind) => {
                // Мало здоровья -> берёт Сердце Гиганта (реген/хп)
                if *kind == PassiveType::GiantHeart && hp_ratio < 0.5 {
                    score += 60.0 + (1.0 - hp_ratio) * 60.0;
                }
                // Били вблизи -> берёт Броню Света (плоская броня)
                if *kind == PassiveType::LightArmor && melee_dmg > 30.0 {
                    score += 50.0;
                }
                // Били издали -> берёт скорость атаки и кд (SpeedGloves, CooldownTome)
                if matches!(kind, PassiveType::SpeedGloves | PassiveType::CooldownTome) && ranged_dmg > 30.0 {
                    score += 45.0;
                }
                // Кольцо заморозки для контроля орд
                if *kind == PassiveType::FrostRing && world.mobs.len() > 100 {
                    score += 40.0;
                }
            }
            PerkChoice::LimitBreakDamage { .. } | PerkChoice::LimitBreakCooldown { .. } | PerkChoice::LimitBreakProjectiles { .. } => {
                score += 70.0;
            }
            PerkChoice::LimitBreakTitanVigor => {
                if hp_ratio < 0.6 { score += 95.0; } else { score += 55.0; }
            }
            PerkChoice::LimitBreakAdaptiveArmor => {
                score += 75.0;
            }
            PerkChoice::LimitBreakPredatorSpeed => {
                score += 50.0;
            }
        }

        // Небольшой случайный фактор для разнообразия билдов
        score += rand::gen_range(0.0, 10.0);

        if score > best_score {
            best_score = score;
            best_choice = choice.clone();
        }
    }

    // Применение выбранного перка
    apply_perk_choice(world, best_choice);
}

fn generate_perk_choices(inventory: &BossInventory) -> Vec<PerkChoice> {
    let mut available: Vec<(PerkChoice, f32)> = Vec::new();

    // 1. Проверяем готовые эволюции (приоритетнейшие карты)
    for i in 0..inventory.weapons.len() {
        if let Some(evolved) = inventory.can_evolve(i) {
            let from = inventory.weapons[i].kind;
            available.push((PerkChoice::EvolveWeapon { from, to: evolved }, 500.0));
        }
    }

    // 2. Улучшение существующих оружий (приоритет возрастает к 8 уровню)
    for w in &inventory.weapons {
        if w.level < MAX_WEAPON_LEVEL && !w.is_evolved {
            let weight = if w.level >= 6 { 140.0 } else { 75.0 };
            available.push((PerkChoice::UpgradeWeapon(w.kind), weight));
        }
    }

    // 3. Добавление новых базовых оружий (14 видов)
    if inventory.weapons.len() < MAX_WEAPON_SLOTS {
        let base_weapons = [
            WeaponType::WindBlade,
            WeaponType::FireAura,
            WeaponType::MagicWand,
            WeaponType::HolyHammer,
            WeaponType::IceShards,
            WeaponType::PoisonOrbs,
            WeaponType::ExecutionerAxe,
            WeaponType::CelestialBow,
            WeaponType::ThunderOrb,
            WeaponType::BloodDaggers,
            WeaponType::SingularityOrb,
            WeaponType::ChainFrost,
            WeaponType::BrimstoneRay,
            WeaponType::MissileSwarm,
        ];
        for bw in base_weapons {
            if !inventory.has_weapon(bw) {
                let has_pair = match bw {
                    WeaponType::WindBlade => inventory.has_passive(PassiveType::SpeedGloves),
                    WeaponType::FireAura => inventory.has_passive(PassiveType::GiantHeart),
                    WeaponType::MagicWand => inventory.has_passive(PassiveType::CooldownTome),
                    WeaponType::HolyHammer => inventory.has_passive(PassiveType::LightArmor),
                    WeaponType::IceShards => inventory.has_passive(PassiveType::FrostRing),
                    WeaponType::PoisonOrbs => inventory.has_passive(PassiveType::CorrosiveFlask),
                    WeaponType::ExecutionerAxe => inventory.has_passive(PassiveType::BloodStone),
                    WeaponType::CelestialBow => inventory.has_passive(PassiveType::EagleScope),
                    WeaponType::ThunderOrb => inventory.has_passive(PassiveType::ChronoHourglass),
                    WeaponType::BloodDaggers => inventory.has_passive(PassiveType::MagnetSphere),
                    WeaponType::SingularityOrb => inventory.has_passive(PassiveType::TeslaCoil),
                    WeaponType::ChainFrost => inventory.has_passive(PassiveType::CloverOfFate),
                    WeaponType::BrimstoneRay => inventory.has_passive(PassiveType::PrismaticFocus),
                    WeaponType::MissileSwarm => inventory.has_passive(PassiveType::TitanAegis),
                    _ => false,
                };
                let weight = if has_pair { 180.0 } else { 45.0 };
                available.push((PerkChoice::UpgradeWeapon(bw), weight));
            }
        }
    }

    // 4. Улучшение существующих пассивок
    for p in &inventory.passives {
        if p.level < MAX_PASSIVE_LEVEL {
            let weight = 70.0 + (p.level as f32 * 10.0);
            available.push((PerkChoice::UpgradePassive(p.kind), weight));
        }
    }

    // 5. Добавление новых пассивок (14 видов)
    if inventory.passives.len() < MAX_PASSIVE_SLOTS {
        let all_passives = [
            PassiveType::SpeedGloves,
            PassiveType::GiantHeart,
            PassiveType::CooldownTome,
            PassiveType::LightArmor,
            PassiveType::FrostRing,
            PassiveType::CorrosiveFlask,
            PassiveType::BloodStone,
            PassiveType::EagleScope,
            PassiveType::ChronoHourglass,
            PassiveType::MagnetSphere,
            PassiveType::TeslaCoil,
            PassiveType::CloverOfFate,
            PassiveType::PrismaticFocus,
            PassiveType::TitanAegis,
        ];
        for ap in all_passives {
            if !inventory.has_passive(ap) {
                let has_pair = match ap {
                    PassiveType::SpeedGloves => inventory.has_weapon(WeaponType::WindBlade),
                    PassiveType::GiantHeart => inventory.has_weapon(WeaponType::FireAura),
                    PassiveType::CooldownTome => inventory.has_weapon(WeaponType::MagicWand),
                    PassiveType::LightArmor => inventory.has_weapon(WeaponType::HolyHammer),
                    PassiveType::FrostRing => inventory.has_weapon(WeaponType::IceShards),
                    PassiveType::CorrosiveFlask => inventory.has_weapon(WeaponType::PoisonOrbs),
                    PassiveType::BloodStone => inventory.has_weapon(WeaponType::ExecutionerAxe),
                    PassiveType::EagleScope => inventory.has_weapon(WeaponType::CelestialBow),
                    PassiveType::ChronoHourglass => inventory.has_weapon(WeaponType::ThunderOrb),
                    PassiveType::MagnetSphere => inventory.has_weapon(WeaponType::BloodDaggers),
                    PassiveType::TeslaCoil => inventory.has_weapon(WeaponType::SingularityOrb),
                    PassiveType::CloverOfFate => inventory.has_weapon(WeaponType::ChainFrost),
                    PassiveType::PrismaticFocus => inventory.has_weapon(WeaponType::BrimstoneRay),
                    PassiveType::TitanAegis => inventory.has_weapon(WeaponType::MissileSwarm),
                };
                let weight = if has_pair { 180.0 } else { 40.0 };
                available.push((PerkChoice::UpgradePassive(ap), weight));
            }
        }
    }

    // 6. ЗАПРЕДЕЛЬНОЕ ВОЗВЫШЕНИЕ (Limit Break после 10-й минуты)
    if available.is_empty() {
        for (i, w) in inventory.weapons.iter().enumerate() {
            let def = crate::content::weapons::get_weapon_def(w.kind);
            available.push((PerkChoice::LimitBreakDamage { weapon_idx: i, name: def.name.to_string() }, 90.0));
            available.push((PerkChoice::LimitBreakCooldown { weapon_idx: i, name: def.name.to_string() }, 70.0));
            available.push((PerkChoice::LimitBreakProjectiles { weapon_idx: i, name: def.name.to_string() }, 85.0));
        }
        available.push((PerkChoice::LimitBreakTitanVigor, 110.0));
        available.push((PerkChoice::LimitBreakAdaptiveArmor, 100.0));
        available.push((PerkChoice::LimitBreakPredatorSpeed, 60.0));
    }

    // Взвешенный выбор 3 уникальных карт (Рулетка)
    let mut selected = Vec::new();
    while selected.len() < 3 && !available.is_empty() {
        let total_w: f32 = available.iter().map(|(_, w)| *w).sum();
        if total_w <= 0.0 {
            break;
        }
        let mut pick_val = rand::gen_range(0.0, total_w);
        let mut chosen_idx = 0;
        for (idx, (_, w)) in available.iter().enumerate() {
            if pick_val <= *w {
                chosen_idx = idx;
                break;
            }
            pick_val -= *w;
        }
        let (choice, _) = available.remove(chosen_idx);
        selected.push(choice);
    }

    selected
}

fn apply_perk_choice(world: &mut World, choice: PerkChoice) {
    let boss_pos = world.boss_pos;
    match choice {
        PerkChoice::EvolveWeapon { from, to } => {
            if let Some(w) = world.boss_inventory.weapons.iter_mut().find(|w| w.kind == from) {
                w.kind = to;
                w.is_evolved = true;
                w.damage *= 1.8;
                w.base_cooldown *= 0.7;

                let name = match to {
                    WeaponType::BladeStorm => "БУРЯ КЛИНКОВ",
                    WeaponType::Hellfire => "АДСКОЕ ПЕКЛО",
                    WeaponType::DeathRay => "ЛУЧ СМЕРТИ",
                    WeaponType::DivineShield => "ПРАВЕДНЫЙ ЩИТ",
                    WeaponType::BlizzardStorm => "ЛЕДНИКОВАЯ БУРЯ",
                    WeaponType::ToxicPestilence => "ТОКСИЧНЫЙ МОР",
                    WeaponType::WhirlwindCleaver => "ВИХРЕВОЙ СЕКАЧ",
                    WeaponType::StarfallBarrage => "ЗВЕЗДОПАДНЫЙ ЗАЛП",
                    WeaponType::ThunderstormWrath => "ГНЕВ ГРОЗЫ",
                    WeaponType::BloodHarvest => "КРОВАВАЯ ЖАТВА",
                    WeaponType::EventHorizon => "ГОРИЗОНТ СОБЫТИЙ",
                    WeaponType::AbsoluteZero => "АБСОЛЮТНЫЙ НОЛЬ",
                    WeaponType::InfernalCataclysm => "ИНФЕРНАЛЬНЫЙ КАТАКЛИЗМ",
                    WeaponType::DoomsdayBarrage => "ОСАДА СУДНОГО ДНЯ",
                    _ => "ЭВОЛЮЦИЯ ОРУЖИЯ",
                };
                let text = format!("ЭВОЛЮЦИЯ: {}!", name);
                world.boss_inventory.last_chosen_perk = Some(text.clone());
                world.boss_inventory.perk_announcement_timer = 3.5;
                world.add_floating_text(boss_pos, text, GOLD, 1.6);
                world.spawn_particles(boss_pos, 40, GOLD, 160.0);
            }
        }
        PerkChoice::UpgradeWeapon(kind) => {
            if let Some(w) = world.boss_inventory.weapons.iter_mut().find(|w| w.kind == kind) {
                w.level += 1;
                w.damage += 8.0;
                w.base_cooldown = (w.base_cooldown * 0.94).max(0.3);

                let text = format!("ОРУЖИЕ +1: {:?} (Lvl {})", kind, w.level);
                world.boss_inventory.last_chosen_perk = Some(text.clone());
                world.boss_inventory.perk_announcement_timer = 2.0;
                world.add_floating_text(boss_pos, text, SKYBLUE, 1.2);
            } else if world.boss_inventory.weapons.len() < MAX_WEAPON_SLOTS {
                let base_cd = match kind {
                    WeaponType::FireAura => 0.8,
                    WeaponType::MagicWand => 0.7,
                    WeaponType::HolyHammer => 1.4,
                    WeaponType::IceShards => 1.0,
                    WeaponType::PoisonOrbs => 1.2,
                    WeaponType::ExecutionerAxe => 1.3,
                    WeaponType::CelestialBow => 0.85,
                    WeaponType::ThunderOrb => 1.5,
                    WeaponType::BloodDaggers => 0.65,
                    WeaponType::SingularityOrb => 1.4,
                    WeaponType::ChainFrost => 1.1,
                    WeaponType::BrimstoneRay => 1.2,
                    WeaponType::MissileSwarm => 0.9,
                    _ => 1.1,
                };
                let base_dmg = match kind {
                    WeaponType::HolyHammer => 45.0,
                    WeaponType::MagicWand => 32.0,
                    WeaponType::FireAura => 18.0,
                    WeaponType::ExecutionerAxe => 42.0,
                    WeaponType::CelestialBow => 32.0,
                    WeaponType::ThunderOrb => 50.0,
                    WeaponType::BloodDaggers => 24.0,
                    WeaponType::SingularityOrb => 38.0,
                    WeaponType::ChainFrost => 30.0,
                    WeaponType::BrimstoneRay => 42.0,
                    WeaponType::MissileSwarm => 34.0,
                    _ => 25.0,
                };
                world.boss_inventory.weapons.push(crate::ecs::components::inventory::WeaponSlot {
                    kind,
                    level: 1,
                    cooldown_timer: 0.0,
                    base_cooldown: base_cd,
                    damage: base_dmg,
                    is_evolved: false,
                    limit_break_level: 0,
                    bonus_projectiles: 0,
                });
                let text = format!("НОВОЕ ОРУЖИЕ: {:?}", kind);
                world.boss_inventory.last_chosen_perk = Some(text.clone());
                world.boss_inventory.perk_announcement_timer = 2.5;
                world.add_floating_text(boss_pos, text, GREEN, 1.3);
            }
        }
        PerkChoice::UpgradePassive(kind) => {
            if let Some(p) = world.boss_inventory.passives.iter_mut().find(|p| p.kind == kind) {
                p.level += 1;
            } else if world.boss_inventory.passives.len() < MAX_PASSIVE_SLOTS {
                world.boss_inventory.passives.push(crate::ecs::components::inventory::PassiveSlot {
                    kind,
                    level: 1,
                });
            }

            // Применяем эффекты пассивки к характеристикам
            match kind {
                PassiveType::GiantHeart => {
                    world.boss_health.max += 250.0;
                    world.boss_health.heal(250.0);
                }
                PassiveType::LightArmor => {
                    world.boss_resistances.flat_armor += 4.0;
                    world.boss_resistances.physical_res = (world.boss_resistances.physical_res + 0.06).min(0.60);
                }
                PassiveType::BloodStone => {
                    world.boss_health.max += 150.0;
                    world.boss_health.heal(150.0);
                }
                PassiveType::TitanAegis => {
                    world.boss_resistances.flat_armor += 3.0;
                    world.boss_resistances.physical_res = (world.boss_resistances.physical_res + 0.04).min(0.70);
                }
                _ => {}
            }

            let text = format!("ПАССИВКА: {:?}", kind);
            world.boss_inventory.last_chosen_perk = Some(text.clone());
            world.boss_inventory.perk_announcement_timer = 2.0;
            world.add_floating_text(boss_pos, text, VIOLET, 1.2);
        }
        PerkChoice::LimitBreakDamage { weapon_idx, name } => {
            if let Some(w) = world.boss_inventory.weapons.get_mut(weapon_idx) {
                w.limit_break_level += 1;
                w.damage *= 1.12;
                let text = format!("⚡ ВОЗВЫШЕНИЕ: {} (+12% Урона)", name);
                world.boss_inventory.last_chosen_perk = Some(text.clone());
                world.boss_inventory.perk_announcement_timer = 2.5;
                world.add_floating_text(boss_pos, text, GOLD, 1.4);
                world.spawn_particles(boss_pos, 25, GOLD, 150.0);
            }
        }
        PerkChoice::LimitBreakCooldown { weapon_idx, name } => {
            if let Some(w) = world.boss_inventory.weapons.get_mut(weapon_idx) {
                w.limit_break_level += 1;
                w.base_cooldown = (w.base_cooldown * 0.94).max(0.12);
                let text = format!("⚡ ВОЗВЫШЕНИЕ: {} (-6% КД)", name);
                world.boss_inventory.last_chosen_perk = Some(text.clone());
                world.boss_inventory.perk_announcement_timer = 2.5;
                world.add_floating_text(boss_pos, text, SKYBLUE, 1.4);
                world.spawn_particles(boss_pos, 20, SKYBLUE, 140.0);
            }
        }
        PerkChoice::LimitBreakProjectiles { weapon_idx, name } => {
            if let Some(w) = world.boss_inventory.weapons.get_mut(weapon_idx) {
                w.limit_break_level += 1;
                w.bonus_projectiles += 1;
                let text = format!("⚡ ВОЗВЫШЕНИЕ: {} (+1 Снаряд)", name);
                world.boss_inventory.last_chosen_perk = Some(text.clone());
                world.boss_inventory.perk_announcement_timer = 2.5;
                world.add_floating_text(boss_pos, text, ORANGE, 1.4);
                world.spawn_particles(boss_pos, 30, ORANGE, 160.0);
            }
        }
        PerkChoice::LimitBreakTitanVigor => {
            world.boss_health.max += 160.0;
            world.boss_health.heal(world.boss_health.max * 0.25);
            let text = "🛡️ ВОЗВЫШЕНИЕ: Закалка Титана (+160 HP, Лечение)".to_string();
            world.boss_inventory.last_chosen_perk = Some(text.clone());
            world.boss_inventory.perk_announcement_timer = 3.0;
            world.add_floating_text(boss_pos, text, GREEN, 1.5);
            world.spawn_particles(boss_pos, 35, GREEN, 180.0);
        }
        PerkChoice::LimitBreakAdaptiveArmor => {
            world.boss_resistances.flat_armor += 2.0;
            world.boss_resistances.physical_res = (world.boss_resistances.physical_res + 0.03).min(0.75);
            world.boss_resistances.fire_res = (world.boss_resistances.fire_res + 0.03).min(0.75);
            let text = "🛡️ ВОЗВЫШЕНИЕ: Адаптивная Броня (+2 Брони, +3% Резисты)".to_string();
            world.boss_inventory.last_chosen_perk = Some(text.clone());
            world.boss_inventory.perk_announcement_timer = 3.0;
            world.add_floating_text(boss_pos, text, PURPLE, 1.5);
            world.spawn_particles(boss_pos, 25, PURPLE, 160.0);
        }
        PerkChoice::LimitBreakPredatorSpeed => {
            let text = "⚡ ВОЗВЫШЕНИЕ: Скорость Хищника (+10% Рефлексы)".to_string();
            world.boss_inventory.last_chosen_perk = Some(text.clone());
            world.boss_inventory.perk_announcement_timer = 2.5;
            world.add_floating_text(boss_pos, text, YELLOW, 1.3);
            world.spawn_particles(boss_pos, 20, YELLOW, 140.0);
        }
    }
}
