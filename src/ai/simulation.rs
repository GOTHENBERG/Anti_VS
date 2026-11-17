use macroquad::prelude::*;
use crate::core::rand;
use crate::ai::models::{BossModel, DirectorModel};
use crate::content::mobs::{get_squad_config, spawn_squad_perimeter};
use crate::content::structures::{spawn_blood_altar, BLOOD_ALTAR_COST};
use crate::core::{MAP_HEIGHT, MAP_WIDTH};
use crate::ecs::systems::{
    combat::update_combat,
    director::update_director,
    meditation::update_meditation,
    movement::update_movement,
    skills::update_skills,
};
use crate::ecs::world::World;

#[derive(Clone, Debug)]
pub struct MatchResult {
    pub survival_time: f32,
    pub boss_level: u32,
    pub boss_won: bool,
    pub damage_dealt_to_boss: f32,
    pub altars_built: usize,
    pub altars_destroyed: usize,
    pub mobs_spawned_by_tier: [usize; 16],
    pub director_grades: (u32, u32, u32),
    pub total_meditation_time: f32,
    pub distance_walked: f32,
    pub boss_dashes: usize,
    pub boss_damage_stats: crate::ecs::components::inventory::BossDamageStats,
    pub final_weapons: Vec<(crate::ecs::components::inventory::WeaponType, u8)>,
    pub final_passives: Vec<(crate::ecs::components::inventory::PassiveType, u8)>,
}

/// Выполняет один раунд автономной ускоренной симуляции сражения без рендеринга
pub fn run_headless_simulation(
    boss_model: &BossModel,
    director_model: &DirectorModel,
    max_duration_seconds: f32,
) -> MatchResult {
    let mut world = World::new();
    world.is_headless = true;
    world.boss_model = Some(boss_model.clone());
    let dt = 0.05f32; // 20 тиков на каждую виртуальную секунду
    let mut sim_time = 0.0f32;
    let mut director_decision_timer = 0.0f32;
    let mut altars_built = 1;
    let mut total_meditation_time = 0.0f32;
    let mut total_distance_walked = 0.0f32;
    let mut total_dashes = 0usize;
    let mut mobs_spawned_by_tier = [0usize; 16];

    // Начальный алтарь
    spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));

    while world.boss_health.is_alive() && sim_time < max_duration_seconds {
        sim_time += dt;
        director_decision_timer += dt;

        // 1. Принятие тактических решений Директором-ботом (каждые 0.4 сек)
        if director_decision_timer >= 0.4 {
            director_decision_timer = 0.0;

            let mut should_spawn_altar = false;
            let mut altar_pos = Vec2::ZERO;
            let mut tier_to_spawn = None;

            if let Some(director) = world.directors.first() {
                let cooldown = director.altar_cooldown;
                let points = director.points;
                let spent = director.total_points_spent;

                // 1.1 Постройка Алтаря Крови (Кулдаун 60 сек, без искусственного лимита)
                if cooldown <= 0.0 && points >= BLOOD_ALTAR_COST {
                    let build_roll = rand::gen_range(0.0, 1.0);
                    if build_roll < director_model.altar_build_priority {
                        should_spawn_altar = true;
                        let altar_angle = rand::gen_range(0.0, std::f32::consts::PI * 2.0);
                        let altar_dist = rand::gen_range(350.0, 750.0);
                        altar_pos = (world.boss_pos + Vec2::new(altar_angle.cos() * altar_dist, altar_angle.sin() * altar_dist))
                            .clamp(Vec2::new(60.0, 60.0), Vec2::new(MAP_WIDTH - 60.0, MAP_HEIGHT - 60.0));
                    }
                }

                // 1.2 Призыв отрядов монстров (Тиры 1 - 8) на основе вероятностных весов (Рулетка)
                let remaining_points = if should_spawn_altar { points - BLOOD_ALTAR_COST } else { points };
                let mut eligible_squads = Vec::with_capacity(8);
                let mut total_weight = 0.0f32;

                for (tier_idx, &w) in director_model.squad_spawn_weights.iter().enumerate() {
                    let tier = (tier_idx + 1) as u8;
                    let config = get_squad_config(tier);
                    let is_unlocked = spent >= config.unlock_spent_threshold;
                    if is_unlocked && remaining_points >= config.cost && w > 0.0 {
                        eligible_squads.push((tier, config.cost, w));
                        total_weight += w;
                    }
                }

                if !eligible_squads.is_empty() && total_weight > 0.0 {
                    let mut roll = rand::gen_range(0.0, total_weight);
                    for (tier, cost, w) in eligible_squads {
                        if roll <= w {
                            tier_to_spawn = Some((tier, cost));
                            break;
                        }
                        roll -= w;
                    }
                }
            }

            if should_spawn_altar {
                if let Some(director) = world.directors.first_mut() {
                    director.spend_points(BLOOD_ALTAR_COST);
                    director.altar_cooldown = director.altar_max_cooldown;
                }
                spawn_blood_altar(&mut world, altar_pos);
                altars_built += 1;
            }

            if let Some((tier, cost)) = tier_to_spawn {
                if let Some(director) = world.directors.first_mut() {
                    if director.spend_points(cost) {
                        spawn_squad_perimeter(&mut world, tier);
                        if tier >= 1 && tier <= 16 {
                            let count = get_squad_config(tier).count;
                            mobs_spawned_by_tier[(tier - 1) as usize] += count;
                        }
                    }
                }
            }

            // 1.3 Улучшение грейдов армии Режиссера (Ярость, Панцирь, Чумные споры)
            if let Some(director) = world.directors.first_mut() {
                let fury_cost = 100.0 * (director.grade_fury as f32 + 1.0);
                let carapace_cost = 120.0 * (director.grade_carapace as f32 + 1.0);
                let pestilence_cost = 150.0 * (director.grade_pestilence as f32 + 1.0);

                if director.points >= pestilence_cost && director.grade_pestilence <= director.grade_fury {
                    if director.spend_points(pestilence_cost) {
                        director.grade_pestilence += 1;
                    }
                } else if director.points >= carapace_cost && director.grade_carapace <= director.grade_fury {
                    if director.spend_points(carapace_cost) {
                        director.grade_carapace += 1;
                    }
                } else if director.points >= fury_cost {
                    if director.spend_points(fury_cost) {
                        director.grade_fury += 1;
                    }
                }
            }
        }

        // 2. Системные обновления игрового мира
        update_director(&mut world, dt);
        let (walk_dist, dashed) = update_boss_ai_sim(&mut world, dt, boss_model);
        total_distance_walked += walk_dist;
        if dashed {
            total_dashes += 1;
        }
        update_meditation(&mut world, dt);
        if world.boss_still_time >= 1.2 {
            total_meditation_time += dt;
        }
        update_skills(&mut world, dt);
        update_movement(&mut world, dt);
        update_combat(&mut world, dt);
    }

    let boss_won = world.boss_health.is_alive() && sim_time >= max_duration_seconds;
    let final_level = world.boss_inventory.level;
    let dmg_to_boss = world.total_damage_dealt_to_boss;
    let director_grades = world.directors.first()
        .map(|d| (d.grade_fury, d.grade_carapace, d.grade_pestilence))
        .unwrap_or((0, 0, 0));
    let final_weapons = world.boss_inventory.weapons.iter().map(|w| (w.kind, w.level)).collect();
    let final_passives = world.boss_inventory.passives.iter().map(|p| (p.kind, p.level)).collect();

    MatchResult {
        survival_time: sim_time,
        boss_level: final_level,
        boss_won,
        damage_dealt_to_boss: dmg_to_boss,
        altars_built,
        altars_destroyed: altars_built.saturating_sub(world.structures.len()),
        mobs_spawned_by_tier,
        director_grades,
        total_meditation_time,
        distance_walked: total_distance_walked,
        boss_dashes: total_dashes,
        boss_damage_stats: world.boss_inventory.damage_stats.clone(),
        final_weapons,
        final_passives,
    }
}

/// Симуляция ИИ Босса с использованием параметров обученной модели и нейросети
fn update_boss_ai_sim(world: &mut World, dt: f32, model: &BossModel) -> (f32, bool) {
    if world.boss_dash_cooldown > 0.0 {
        world.boss_dash_cooldown = (world.boss_dash_cooldown - dt).max(0.0);
    }
    if world.boss_invulnerable_timer > 0.0 {
        world.boss_invulnerable_timer = (world.boss_invulnerable_timer - dt).max(0.0);
    }

    // Затухание недавнего урона и обновление внутренней телеметрии
    world.boss_inventory.update_damage_decay(dt);

    let boss_pos = world.boss_pos;
    let mut total_force = Vec2::ZERO;

    let (nn_bias, repel_weight, gem_pull_weight, altar_aggression, sniper_aggression, dash_threshold, retreat_hp, neural_dash) =
        if let Some(ref net) = model.neural_net {
            let obs = crate::ai::sensors::extract_boss_observations(world);
            let nn_out = net.forward(&obs);
            (
                Vec2::new(nn_out[0], nn_out[1]) * 120.0,
                200.0 + nn_out[2] * 500.0,
                100.0 + nn_out[3] * 350.0,
                150.0 + nn_out[4] * 400.0,
                150.0 + nn_out[5] * 400.0,
                (8.0 + nn_out[6] * 10.0) as usize,
                model.retreat_hp_threshold,
                nn_out[6] > 0.60,
            )
        } else {
            (
                Vec2::ZERO,
                model.mob_repulsion_weight,
                model.gem_attraction_weight,
                model.altar_siege_aggression,
                model.sniper_hunt_aggression,
                model.dash_danger_threshold,
                model.retreat_hp_threshold,
                false,
            )
        };

    total_force += nn_bias;

    let mobs_in_close = world.spatial_grid.query_nearby(boss_pos, 90.0).len();
    let hp_ratio = world.boss_health.current / world.boss_health.max;

    // 1. Отталкивание от мобов с весом модели
    let nearby = world.spatial_grid.query_nearby(boss_pos, 180.0);
    for idx in nearby {
        if let Some(mob) = world.mobs.get(idx) {
            let repel = crate::core::math::calculate_repel_force(boss_pos, mob.pos, 180.0, repel_weight);
            total_force += repel;
        }
    }

    // 2. Притяжение к кристаллам душ и глобальным супер-предметам "Магнит"
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
        let pull = crate::core::math::calculate_attract_force(boss_pos, m_pos, MAP_WIDTH, gem_pull_weight * 3.5);
        total_force += pull;
    } else {
        let mut closest_gem_dist_sq = 280.0 * 280.0;
        let mut gem_pull = Vec2::ZERO;
        for gem in &world.gems {
            let dist_sq = gem.pos.distance_squared(boss_pos);
            if dist_sq < closest_gem_dist_sq {
                closest_gem_dist_sq = dist_sq;
                gem_pull = crate::core::math::calculate_attract_force(boss_pos, gem.pos, 280.0, gem_pull_weight);
            }
        }
        total_force += gem_pull;
    }

    // 3. Осада Алтарей Крови с весом модели
    if hp_ratio > retreat_hp && mobs_in_close < 8 {
        for st in &world.structures {
            let dist_sq = st.pos.distance_squared(boss_pos);
            if dist_sq < 550.0 * 550.0 {
                let pull = crate::core::math::calculate_attract_force(boss_pos, st.pos, 550.0, altar_aggression);
                total_force += pull;
            }
        }
    }

    // 4. Охота на снайперов-культистов с весом модели
    let cultists: Vec<Vec2> = world.spatial_grid.query_nearby(boss_pos, 420.0)
        .iter()
        .filter_map(|&idx| world.mobs.get(idx))
        .filter(|m| m.tier == 3 || m.tier == 11)
        .map(|m| m.pos)
        .collect();

    if cultists.len() >= model.sniper_engage_min_count && hp_ratio > retreat_hp {
        let mut center = Vec2::ZERO;
        for p in &cultists {
            center += *p;
        }
        center /= cultists.len() as f32;
        let to_c = center - boss_pos;
        if to_c.length_squared() > 100.0 {
            total_force += to_c.normalize_or_zero() * sniper_aggression;
        }
    }

    // 5. Отталкивание от стен (плавное параболическое замедление)
    let map_w = MAP_WIDTH;
    let map_h = MAP_HEIGHT;
    let wall_margin = 220.0;
    let push = model.wall_repel_weight;

    if boss_pos.x < wall_margin {
        let factor = 1.0 - (boss_pos.x / wall_margin).clamp(0.0, 1.0);
        total_force.x += push * factor * factor;
    } else if boss_pos.x > map_w - wall_margin {
        let factor = 1.0 - ((map_w - boss_pos.x) / wall_margin).clamp(0.0, 1.0);
        total_force.x -= push * factor * factor;
    }
    if boss_pos.y < wall_margin {
        let factor = 1.0 - (boss_pos.y / wall_margin).clamp(0.0, 1.0);
        total_force.y += push * factor * factor;
    } else if boss_pos.y > map_h - wall_margin {
        let factor = 1.0 - ((map_h - boss_pos.y) / wall_margin).clamp(0.0, 1.0);
        total_force.y -= push * factor * factor;
    }

    // 6. Рывок (Dash): 100% автономия нейросети (Чистый Reinforcement Learning)
    let should_dash = if model.neural_net.is_some() {
        neural_dash && world.boss_dash_cooldown <= 0.0
    } else {
        let heavy_ranged_fire = world.boss_inventory.ranged_damage_recent > 40.0;
        (mobs_in_close >= dash_threshold || hp_ratio < retreat_hp || heavy_ranged_fire)
            && world.boss_dash_cooldown <= 0.0
    };

    let mut walked_distance = 0.0f32;
    let mut dashed = false;

    if should_dash {
        let dash_dir = if neural_dash && nn_bias.length_squared() > 1.0 {
            nn_bias.normalize()
        } else if total_force.length_squared() > 1.0 {
            total_force.normalize()
        } else {
            Vec2::X
        };
        let dash_target = (boss_pos + dash_dir * 180.0)
            .clamp(Vec2::new(40.0, 40.0), Vec2::new(map_w - 40.0, map_h - 40.0));

        world.boss_pos = dash_target;
        world.boss_invulnerable_timer = 0.25;
        world.boss_dash_cooldown = 7.0;
        dashed = true;
        // Движение от рывка (дэша) намеренно НЕ добавляется в дистанцию ходьбы!
    } else {
        let prev_walk_pos = world.boss_pos;
        let max_speed = world.boss_inventory.get_movement_speed(160.0);
        let desired = if total_force.length_squared() > 4.0 {
            total_force.clamp_length_max(max_speed)
        } else {
            Vec2::ZERO
        };
        world.boss_vel = world.boss_vel.lerp(desired, (dt * 10.0).min(1.0));
        world.boss_pos += world.boss_vel * dt;
        world.boss_pos.x = world.boss_pos.x.clamp(35.0, map_w - 35.0);
        world.boss_pos.y = world.boss_pos.y.clamp(35.0, map_h - 35.0);
        walked_distance = prev_walk_pos.distance(world.boss_pos);
    }

    if dashed {
        world.boss_pos.x = world.boss_pos.x.clamp(35.0, map_w - 35.0);
        world.boss_pos.y = world.boss_pos.y.clamp(35.0, map_h - 35.0);
    }

    (walked_distance, dashed)
}
