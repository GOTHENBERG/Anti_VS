use macroquad::prelude::*;
use crate::core::rand;
use crate::core::math::{calculate_attract_force, calculate_repel_force};
use crate::ai::neural_net::{
    BOSS_DASH_BASE_COOLDOWN_SECONDS, BOSS_DECISION_INTERVAL, OUT_DASH, OUT_DASH_X, OUT_DASH_Y,
    OUT_MOVE_MAG, OUT_MOVE_X, OUT_MOVE_Y, OUT_REST,
};
use crate::ecs::components::inventory::{
    BossInventory, PassiveType, PerkChoice, WeaponType, MAX_PASSIVE_LEVEL, MAX_PASSIVE_SLOTS,
    MAX_WEAPON_LEVEL, MAX_WEAPON_SLOTS,
};
use crate::ecs::world::World;

pub fn update_boss_ai(world: &mut World, dt: f32) {
    if !world.boss_health.is_alive() {
        world.boss_vel = Vec2::ZERO;
        return;
    }
    update_boss_timers(world, dt);

    let boss_pos = world.boss_pos;
    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;

    let (desired_vel, should_dash, dash_dir) = if let Some(ref model) = world.boss_model {
        if let Some(ref net) = model.neural_net {
            world.boss_policy_think_timer += dt;
            if !world.boss_policy_has_cached_action
                || world.boss_policy_think_timer >= BOSS_DECISION_INTERVAL
            {
                world.boss_policy_think_timer = 0.0;
                let obs = crate::ai::sensors::extract_boss_observations(world);
                let nn_out = net.forward(&obs);
                let move_intent = Vec2::new(nn_out[OUT_MOVE_X], nn_out[OUT_MOVE_Y]);
                let rest = nn_out[OUT_REST] > 0.5;
                let move_mag = if rest { 0.0 } else { nn_out[OUT_MOVE_MAG].clamp(0.0, 1.0) };
                let move_dir = if move_intent.length_squared() > 0.0025 {
                    move_intent.normalize()
                } else {
                    Vec2::ZERO
                };
                let max_speed = world.boss_inventory.get_movement_speed(160.0);

                let velocity = if !rest {
                    move_dir * max_speed * move_mag
                } else {
                    Vec2::ZERO
                };
                let dash_raw = Vec2::new(nn_out[OUT_DASH_X], nn_out[OUT_DASH_Y]);
                world.boss_policy_move_dir = move_dir;
                world.boss_policy_move_mag = move_mag;
                world.boss_policy_dash = nn_out[OUT_DASH] > 0.5;
                world.boss_policy_dash_dir = if dash_raw.length_squared() > 0.04 {
                    dash_raw.normalize()
                } else if velocity.length_squared() > 1.0 {
                    move_dir
                } else {
                    Vec2::X
                };
                world.boss_policy_rest = rest;
                world.boss_policy_has_cached_action = true;
            }

            let max_speed = world.boss_inventory.get_movement_speed(160.0);
            let desired_velocity = if world.boss_policy_rest {
                Vec2::ZERO
            } else {
                world.boss_policy_move_dir * max_speed * world.boss_policy_move_mag
            };
            (
                desired_velocity,
                world.boss_policy_dash && world.boss_dash_cooldown <= 0.0,
                world.boss_policy_dash_dir,
            )
        } else {
            compute_heuristic_movement(world, boss_pos, map_w, map_h, model)
        }
    } else {
        let default_model = crate::ai::models::BossModel::default();
        compute_heuristic_movement(world, boss_pos, map_w, map_h, &default_model)
    };

    if should_dash {
        perform_boss_dash(world, dash_dir);
    } else {
        world.boss_vel = desired_vel;
        world.boss_pos += world.boss_vel * dt;
        if world.boss_vel.length_squared() > 1.0 {
            world.boss_facing = world.boss_vel.normalize();
        }
    }

    clamp_boss_to_arena(world);
}

pub fn update_player_boss(
    world: &mut World,
    dt: f32,
    move_dir: Vec2,
    aim_dir: Vec2,
    dash_pressed: bool,
) {
    update_boss_timers(world, dt);
    if !world.boss_health.is_alive() {
        world.boss_vel = Vec2::ZERO;
        return;
    }
    if move_dir.length_squared() > 0.01 {
        world.boss_facing = move_dir.normalize();
    }
    if dash_pressed && world.boss_dash_cooldown <= 0.0 {
        let direction = if move_dir.length_squared() > 0.01 {
            move_dir.normalize()
        } else if aim_dir.length_squared() > 0.01 {
            aim_dir.normalize()
        } else {
            world.boss_facing
        };
        perform_boss_dash(world, direction);
    } else {
        let speed = world.boss_inventory.get_movement_speed(160.0);
        world.boss_vel = if move_dir.length_squared() > 0.01 {
            move_dir.normalize() * speed
        } else {
            Vec2::ZERO
        };
        world.boss_pos += world.boss_vel * dt;
    }
    clamp_boss_to_arena(world);
}

fn update_boss_timers(world: &mut World, dt: f32) {
    if world.boss_dash_cooldown > 0.0 {
        world.boss_dash_cooldown = (world.boss_dash_cooldown - dt).max(0.0);
    }
    if world.boss_invulnerable_timer > 0.0 {
        world.boss_invulnerable_timer = (world.boss_invulnerable_timer - dt).max(0.0);
    }
    if world.boss_inventory.perk_announcement_timer > 0.0 {
        world.boss_inventory.perk_announcement_timer = (world.boss_inventory.perk_announcement_timer - dt).max(0.0);
    }

    // Затухание недавнего урона и обновление внутренней телеметрии
    world.boss_inventory.update_damage_decay(dt);
}

fn perform_boss_dash(world: &mut World, dash_dir: Vec2) {
    resolve_boss_dash(world, dash_dir);
}

pub fn resolve_boss_dash(world: &mut World, dash_dir: Vec2) {
    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;
    let dir = if dash_dir.length_squared() > 0.01 { dash_dir.normalize() } else { world.boss_facing };
    let old_pos = world.boss_pos;
    let dash_target = (old_pos + dir * 180.0)
        .clamp(Vec2::new(60.0, 60.0), Vec2::new(map_w - 60.0, map_h - 60.0));
    world.boss_pos = dash_target;
    world.boss_vel = Vec2::ZERO;
    world.boss_facing = dir;
    world.boss_invulnerable_timer = 0.25;
    world.boss_dash_cooldown = BOSS_DASH_BASE_COOLDOWN_SECONDS
        * world.boss_inventory.get_cooldown_multiplier();

    if !world.is_headless {
        world.push_audio(crate::audio::AudioEvent::BossDash);
        world.spawn_particles(old_pos, 20, GOLD, 120.0);
        world.spawn_particles(dash_target, 20, YELLOW, 140.0);
        world.add_floating_text(dash_target, "РЫВОК (DASH)!".to_string(), YELLOW, 1.3);
    }

    for proj in &mut world.projectiles {
        if proj.faction == crate::ecs::components::combat::Faction::Director
            && proj.pos.distance_squared(dash_target) < 130.0 * 130.0
        {
            proj.lifetime = 0.0;
        }
    }
    let mut dash_hits: Vec<usize> = Vec::with_capacity(64);
    world.spatial_grid.query_nearby_into(dash_target, 90.0, &mut dash_hits);
    for idx in dash_hits {
        if let Some(mob) = world.mobs.get_mut(idx) {
            if mob.health.is_alive() && mob.pos.distance_squared(dash_target) < 90.0 * 90.0 {
                mob.health.take_damage(80.0);
                if !mob.knockback_immune {
                    mob.knockback_vel += dir * 240.0;
                }
            }
        }
    }
    let mut obelisk_contribution_ids = Vec::new();
    for st in &mut world.structures {
        if st.health.is_alive()
            && crate::content::structures::boss_can_damage_structure(st)
            && st.pos.distance_squared(dash_target) < 65.0 * 65.0
        {
            st.health.take_damage(160.0);
            if st.structure_type == crate::content::structures::STRUCTURE_OBELISK {
                obelisk_contribution_ids.push(st.id);
            }
        }
    }
    for id in obelisk_contribution_ids { world.record_obelisk_contribution(id); }
}

fn clamp_boss_to_arena(world: &mut World) {
    world.boss_pos.x = world.boss_pos.x.clamp(35.0, crate::core::MAP_WIDTH - 35.0);
    world.boss_pos.y = world.boss_pos.y.clamp(35.0, crate::core::MAP_HEIGHT - 35.0);
}

/// Вспомогательный расчет эвристического движения (только для моделей без обученной нейросети)
fn compute_heuristic_movement(
    world: &World,
    boss_pos: Vec2,
    map_w: f32,
    map_h: f32,
    model: &crate::ai::models::BossModel,
) -> (Vec2, bool, Vec2) {
    let mut total_force = Vec2::ZERO;
    let mobs_in_close = world.spatial_grid.query_nearby(boss_pos, 90.0)
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
    let is_critical_hp = hp_ratio < model.retreat_hp_threshold;

    // 1. Отталкивание от мобов
    let nearby = world.spatial_grid.query_nearby(boss_pos, 180.0);
    for idx in nearby {
        if let Some(mob) = world.mobs.get(idx) {
            let repel = calculate_repel_force(boss_pos, mob.pos, 180.0, model.mob_repulsion_weight);
            total_force += repel;
        }
    }

    // 2. Притяжение к кристаллам
    let mut closest_magnet = None;
    let mut min_magnet_dist_sq = f32::MAX;
    for gem in &world.gems {
        if gem.is_magnet {
            let dist_sq = gem.pos.distance_squared(boss_pos);
            if dist_sq < min_magnet_dist_sq {
                min_magnet_dist_sq = dist_sq;
                closest_magnet = Some(gem.pos);
            }
        }
    }
    if let Some(m_pos) = closest_magnet {
        let magnet_pull = calculate_attract_force(boss_pos, m_pos, map_w, model.gem_attraction_weight * 3.5);
        total_force += magnet_pull;
    } else {
        let mut closest_gem_dist_sq = 260.0 * 260.0;
        let mut gem_pull = Vec2::ZERO;
        for gem in &world.gems {
            let dist_sq = gem.pos.distance_squared(boss_pos);
            if dist_sq < closest_gem_dist_sq {
                closest_gem_dist_sq = dist_sq;
                gem_pull = calculate_attract_force(boss_pos, gem.pos, 260.0, model.gem_attraction_weight);
            }
        }
        total_force += gem_pull;
    }

    // 3. Отталкивание от стен
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

    let heavy_ranged_fire = world.boss_inventory.ranged_damage_recent > 40.0;
    let should_dash = (mobs_in_close >= model.dash_danger_threshold || is_critical_hp || heavy_ranged_fire)
        && world.boss_dash_cooldown <= 0.0;
    let dash_dir = if total_force.length_squared() > 1.0 { total_force.normalize() } else { Vec2::X };

    let max_speed = world.boss_inventory.get_movement_speed(160.0);
    let desired_vel = if total_force.length_squared() > 4.0 {
        total_force.normalize_or_zero() * max_speed
    } else {
        Vec2::ZERO
    };

    (desired_vel, should_dash, dash_dir)
}

fn get_weapon_index(kind: WeaponType) -> usize {
    match kind {
        WeaponType::WindBlade | WeaponType::BladeStorm => 0,
        WeaponType::FireAura | WeaponType::Hellfire => 1,
        WeaponType::MagicWand | WeaponType::DeathRay => 2,
        WeaponType::HolyHammer | WeaponType::DivineShield => 3,
        WeaponType::IceShards | WeaponType::BlizzardStorm => 4,
        WeaponType::PoisonOrbs | WeaponType::ToxicPestilence => 5,
        WeaponType::ExecutionerAxe | WeaponType::WhirlwindCleaver => 6,
        WeaponType::CelestialBow | WeaponType::StarfallBarrage => 7,
        WeaponType::ThunderOrb | WeaponType::ThunderstormWrath => 8,
        WeaponType::BloodDaggers | WeaponType::BloodHarvest => 9,
        WeaponType::SingularityOrb | WeaponType::EventHorizon => 10,
        WeaponType::ChainFrost | WeaponType::AbsoluteZero => 11,
        WeaponType::BrimstoneRay | WeaponType::InfernalCataclysm => 12,
        WeaponType::MissileSwarm | WeaponType::DoomsdayBarrage => 13,
        WeaponType::ChainLightning | WeaponType::StormGodWrath => 14,
        WeaponType::AegisOrbiters | WeaponType::InvincibleBastion => 15,
        WeaponType::ChaosMeteor | WeaponType::ArmageddonStorm => 16,
        WeaponType::VoidRift | WeaponType::SingularityCollapse => 17,
        WeaponType::SoulPhantoms | WeaponType::BansheeWail => 18,
        WeaponType::DivineBell | WeaponType::CathedralAura => 19,
        WeaponType::SacredScriptures | WeaponType::InfernalLiturgy => 20,
        WeaponType::FaithBoomerang | WeaponType::HeavenlyJudgment => 21,
        WeaponType::CursedChalice | WeaponType::BloodDeluge => 22,
        WeaponType::VampiricAura | WeaponType::SoulFeast => 23,
        WeaponType::RunicPrism | WeaponType::SpatialShatter => 24,
        WeaponType::AbyssalBeam | WeaponType::ApocalypseCascade => 25,
        WeaponType::ChronoStasis | WeaponType::InfiniteDilation => 26,
        WeaponType::AstralCloak | WeaponType::BloodEmperorShroud => 27,
    }
}

fn get_passive_index(kind: PassiveType) -> usize {
    match kind {
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

fn evaluate_perk_choice(
    choice: &PerkChoice,
    inventory: &BossInventory,
    model: Option<&crate::ai::models::BossModel>,
) -> f32 {
    let mut score = 50.0f32;

    let default_weights = crate::ai::models::default_combo_archetype_weights();
    let archetype_weights = model.map(|m| &m.combo_archetype_weights).unwrap_or(&default_weights);
    let synergy_seeking = model.map(|m| m.synergy_seeking_weight).unwrap_or(1.6);

    let mut archetype_counts = [0usize; 6];
    for w in &inventory.weapons {
        for &a in crate::ai::models::weapon_archetypes(w.kind) {
            archetype_counts[a] += 1;
        }
    }
    for p in &inventory.passives {
        for &a in crate::ai::models::passive_archetypes(p.kind) {
            archetype_counts[a] += 1;
        }
    }

    match choice {
        PerkChoice::EvolveWeapon { from: _, to } => {
            score += 380.0 * synergy_seeking;
            for &a in crate::ai::models::weapon_archetypes(*to) {
                let weight = archetype_weights.get(a).copied().unwrap_or(1.0);
                score += weight * 60.0;
            }
        }
        PerkChoice::UpgradeWeapon(kind) => {
            let w_idx = get_weapon_index(*kind);
            let genetic_priority = model
                .and_then(|m| m.weapon_priorities.get(w_idx).copied())
                .unwrap_or(1.0);
            score += genetic_priority * 45.0;

            if let Some(needed_passive) = BossInventory::get_evolution_passive_for_weapon(*kind) {
                if inventory.has_passive(needed_passive) {
                    score += 190.0 * synergy_seeking;
                }
            }

            for &a in crate::ai::models::weapon_archetypes(*kind) {
                let weight = archetype_weights.get(a).copied().unwrap_or(1.0);
                let count = archetype_counts[a];
                score += weight * (25.0 + count as f32 * 40.0) * synergy_seeking;
            }

            if let Some(w) = inventory.weapons.iter().find(|w| w.kind == *kind) {
                score += (w.level as f32) * 12.0;
            }
        }
        PerkChoice::UpgradePassive(kind) => {
            let p_idx = get_passive_index(*kind);
            let genetic_priority = model
                .and_then(|m| m.passive_priorities.get(p_idx).copied())
                .unwrap_or(1.0);
            score += genetic_priority * 45.0;

            for w in &inventory.weapons {
                if BossInventory::is_evolution_partner(w.kind, *kind) {
                    score += 210.0 * synergy_seeking;
                }
            }

            for &a in crate::ai::models::passive_archetypes(*kind) {
                let weight = archetype_weights.get(a).copied().unwrap_or(1.0);
                let count = archetype_counts[a];
                score += weight * (25.0 + count as f32 * 40.0) * synergy_seeking;
            }
        }
        PerkChoice::LimitBreakTitanVigor => {
            score += 85.0;
        }
        PerkChoice::LimitBreakAdaptiveArmor => {
            score += 85.0;
        }
        PerkChoice::LimitBreakPredatorSpeed => {
            score += 70.0;
        }
        PerkChoice::LimitBreakDamage { .. } | PerkChoice::LimitBreakCooldown { .. } | PerkChoice::LimitBreakProjectiles { .. } => {
            score += 75.0;
        }
    }

    score += rand::gen_range(0.0, 8.0);
    score
}

/// Адаптивный выбор из 3 карт при Level-Up босса на основе синергий и ДНК-приоритетов
pub fn handle_boss_level_up(world: &mut World) {
    let choices = generate_perk_choices(&world.boss_inventory);
    if choices.is_empty() {
        return;
    }

    let model_clone = world.boss_model.clone();

    let mut best_choice = choices[0].clone();
    let mut best_score = -1000.0;

    for choice in &choices {
        let score = evaluate_perk_choice(
            choice,
            &world.boss_inventory,
            model_clone.as_ref(),
        );

        if score > best_score {
            best_score = score;
            best_choice = choice.clone();
        }
    }

    // Применение выбранного перка
    apply_perk_choice(world, best_choice);
}

pub fn generate_perk_choices(inventory: &BossInventory) -> Vec<PerkChoice> {
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
            WeaponType::ChainLightning,
            WeaponType::DivineBell,
            WeaponType::SoulPhantoms,
            WeaponType::AegisOrbiters,
            WeaponType::ChaosMeteor,
            WeaponType::VoidRift,
            WeaponType::SacredScriptures,
            WeaponType::FaithBoomerang,
            WeaponType::CursedChalice,
            WeaponType::VampiricAura,
            WeaponType::RunicPrism,
            WeaponType::AbyssalBeam,
            WeaponType::ChronoStasis,
            WeaponType::AstralCloak,
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
                    WeaponType::ChainLightning => inventory.has_passive(PassiveType::StaticCore),
                    WeaponType::DivineBell => inventory.has_passive(PassiveType::DivinityAura),
                    WeaponType::SoulPhantoms => inventory.has_passive(PassiveType::SoulHarvester),
                    WeaponType::AegisOrbiters => inventory.has_passive(PassiveType::ImmortalAegis),
                    WeaponType::ChaosMeteor => inventory.has_passive(PassiveType::Pyromaniac),
                    WeaponType::VoidRift => inventory.has_passive(PassiveType::OctarineCore),
                    WeaponType::SacredScriptures => inventory.has_passive(PassiveType::ChronoHourglass),
                    WeaponType::FaithBoomerang => inventory.has_passive(PassiveType::CloverOfFate),
                    WeaponType::CursedChalice => inventory.has_passive(PassiveType::PrismaticFocus),
                    WeaponType::VampiricAura => inventory.has_passive(PassiveType::BloodStone),
                    WeaponType::RunicPrism => inventory.has_passive(PassiveType::DuplicatorRelic),
                    WeaponType::AbyssalBeam => inventory.has_passive(PassiveType::CooldownTome),
                    WeaponType::ChronoStasis => inventory.has_passive(PassiveType::SilverTears),
                    WeaponType::AstralCloak => inventory.has_passive(PassiveType::TitanAegis),
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

    // 5. Добавление новых пассивок (35 видов)
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
            PassiveType::ImmortalAegis,
            PassiveType::ArchmageShard,
            PassiveType::DivineRapier,
            PassiveType::OctarineCore,
            PassiveType::SolarRelic,
            PassiveType::StaticCore,
            PassiveType::ColossalEye,
            PassiveType::BlessedHeart,
            PassiveType::Pyromaniac,
            PassiveType::SoulHarvester,
            PassiveType::DivinityAura,
            PassiveType::UkuleleResonance,
            PassiveType::FateWeaver,
            PassiveType::SiegeMortar,
            PassiveType::SporeMushroom,
            PassiveType::DuplicatorRelic,
            PassiveType::CandelabraOfFury,
            PassiveType::AbyssalCurseStone,
            PassiveType::SilverTears,
            PassiveType::GoldenSeal,
            PassiveType::TitanBloodArmor,
        ];
        for ap in all_passives {
            if !inventory.has_passive(ap) {
                let has_pair = match ap {
                    PassiveType::SpeedGloves => inventory.has_weapon(WeaponType::WindBlade),
                    PassiveType::GiantHeart => inventory.has_weapon(WeaponType::FireAura),
                    PassiveType::CooldownTome => inventory.has_weapon(WeaponType::MagicWand) || inventory.has_weapon(WeaponType::AbyssalBeam),
                    PassiveType::LightArmor => inventory.has_weapon(WeaponType::HolyHammer),
                    PassiveType::FrostRing => inventory.has_weapon(WeaponType::IceShards),
                    PassiveType::CorrosiveFlask => inventory.has_weapon(WeaponType::PoisonOrbs),
                    PassiveType::BloodStone => inventory.has_weapon(WeaponType::ExecutionerAxe) || inventory.has_weapon(WeaponType::VampiricAura),
                    PassiveType::EagleScope => inventory.has_weapon(WeaponType::CelestialBow),
                    PassiveType::ChronoHourglass => inventory.has_weapon(WeaponType::ThunderOrb) || inventory.has_weapon(WeaponType::SacredScriptures),
                    PassiveType::MagnetSphere => inventory.has_weapon(WeaponType::BloodDaggers),
                    PassiveType::TeslaCoil => inventory.has_weapon(WeaponType::SingularityOrb),
                    PassiveType::CloverOfFate => inventory.has_weapon(WeaponType::ChainFrost) || inventory.has_weapon(WeaponType::FaithBoomerang),
                    PassiveType::PrismaticFocus => inventory.has_weapon(WeaponType::BrimstoneRay) || inventory.has_weapon(WeaponType::CursedChalice),
                    PassiveType::TitanAegis => inventory.has_weapon(WeaponType::MissileSwarm) || inventory.has_weapon(WeaponType::AstralCloak),
                    PassiveType::ImmortalAegis => inventory.has_weapon(WeaponType::AegisOrbiters),
                    PassiveType::ArchmageShard => inventory.has_weapon(WeaponType::MagicWand) || inventory.has_weapon(WeaponType::ChainLightning),
                    PassiveType::DivineRapier => inventory.has_weapon(WeaponType::WindBlade) || inventory.has_weapon(WeaponType::ExecutionerAxe),
                    PassiveType::OctarineCore => inventory.has_weapon(WeaponType::VoidRift),
                    PassiveType::SolarRelic => inventory.has_weapon(WeaponType::HolyHammer) || inventory.has_weapon(WeaponType::CelestialBow),
                    PassiveType::StaticCore => inventory.has_weapon(WeaponType::ChainLightning),
                    PassiveType::ColossalEye => inventory.has_weapon(WeaponType::IceShards) || inventory.has_weapon(WeaponType::MissileSwarm),
                    PassiveType::BlessedHeart => inventory.has_weapon(WeaponType::CelestialBow) || inventory.has_weapon(WeaponType::DivineBell),
                    PassiveType::Pyromaniac => inventory.has_weapon(WeaponType::ChaosMeteor) || inventory.has_weapon(WeaponType::FireAura),
                    PassiveType::SoulHarvester => inventory.has_weapon(WeaponType::SoulPhantoms) || inventory.has_weapon(WeaponType::BloodDaggers),
                    PassiveType::DivinityAura => inventory.has_weapon(WeaponType::DivineBell),
                    PassiveType::UkuleleResonance => inventory.has_weapon(WeaponType::ChainLightning) || inventory.has_weapon(WeaponType::ThunderOrb),
                    PassiveType::FateWeaver => inventory.has_passive(PassiveType::CloverOfFate),
                    PassiveType::SiegeMortar => inventory.has_weapon(WeaponType::MissileSwarm) || inventory.has_weapon(WeaponType::ChaosMeteor),
                    PassiveType::SporeMushroom => true,
                    PassiveType::DuplicatorRelic => inventory.has_weapon(WeaponType::RunicPrism) || inventory.weapons.len() >= 2,
                    PassiveType::CandelabraOfFury => inventory.has_weapon(WeaponType::CursedChalice) || inventory.has_weapon(WeaponType::FireAura),
                    PassiveType::AbyssalCurseStone => true,
                    PassiveType::SilverTears => inventory.has_weapon(WeaponType::ChronoStasis),
                    PassiveType::GoldenSeal => true,
                    PassiveType::TitanBloodArmor => inventory.has_weapon(WeaponType::VampiricAura) || inventory.has_passive(PassiveType::BloodStone),
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

pub fn apply_perk_choice(world: &mut World, choice: PerkChoice) {
    world.push_audio(crate::audio::AudioEvent::BossLevelUp);
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
                    WeaponType::StormGodWrath => "ГНЕВ БУРИ",
                    WeaponType::CathedralAura => "АУРА СОБОРА",
                    WeaponType::BansheeWail => "ВОПЛЬ БАНШИ",
                    WeaponType::InvincibleBastion => "НЕСОКРУШИМАЯ КРЕПОСТЬ",
                    WeaponType::ArmageddonStorm => "ШТОРМ АРМАГЕДДОНА",
                    WeaponType::SingularityCollapse => "КОЛЛАПС СВЕРХНОВОЙ",
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
                    WeaponType::ChainLightning => 1.0,
                    WeaponType::DivineBell => 1.8,
                    WeaponType::SoulPhantoms => 1.2,
                    WeaponType::AegisOrbiters => 2.5,
                    WeaponType::ChaosMeteor => 2.2,
                    WeaponType::VoidRift => 2.0,
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
                    WeaponType::ChainLightning => 36.0,
                    WeaponType::DivineBell => 55.0,
                    WeaponType::SoulPhantoms => 28.0,
                    WeaponType::AegisOrbiters => 35.0,
                    WeaponType::ChaosMeteor => 75.0,
                    WeaponType::VoidRift => 48.0,
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
                PassiveType::BlessedHeart => {
                    world.boss_health.max += 150.0;
                    world.boss_health.heal(150.0);
                }
                PassiveType::SolarRelic => {
                    world.boss_resistances.fire_res = (world.boss_resistances.fire_res + 0.15).min(0.85);
                    world.boss_resistances.holy_res = (world.boss_resistances.holy_res + 0.15).min(0.85);
                }
                PassiveType::ImmortalAegis => {
                    world.boss_resistances.flat_armor += 5.0;
                }
                PassiveType::SporeMushroom => {
                    world.boss_health.max += 30.0;
                    world.boss_health.heal(50.0);
                }
                PassiveType::DuplicatorRelic => {
                    for w in &mut world.boss_inventory.weapons {
                        w.bonus_projectiles += 1;
                    }
                }
                PassiveType::SilverTears => {
                    world.boss_resistances.poison_res = (world.boss_resistances.poison_res + 0.12).min(0.85);
                    world.boss_resistances.ice_res = (world.boss_resistances.ice_res + 0.12).min(0.85);
                    world.boss_health.heal(60.0);
                }
                PassiveType::TitanBloodArmor => {
                    world.boss_resistances.flat_armor += 3.0;
                    world.boss_resistances.physical_res = (world.boss_resistances.physical_res + 0.05).min(0.75);
                }
                PassiveType::GoldenSeal => {
                    world.boss_inventory.magnet_radius += 35.0;
                    world.boss_health.max += 100.0;
                    world.boss_health.heal(100.0);
                }
                PassiveType::AbyssalCurseStone => {
                    world.boss_inventory.xp_to_next_level = (world.boss_inventory.xp_to_next_level * 0.90).max(20.0);
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
            world.boss_inventory.bonus_move_speed_multiplier += 0.08;
            let text = "⚡ ВОЗВЫШЕНИЕ: Скорость Хищника (+8% Скорости бега)".to_string();
            world.boss_inventory.last_chosen_perk = Some(text.clone());
            world.boss_inventory.perk_announcement_timer = 2.5;
            world.add_floating_text(boss_pos, text, YELLOW, 1.3);
            world.spawn_particles(boss_pos, 20, YELLOW, 140.0);
        }
    }
}
