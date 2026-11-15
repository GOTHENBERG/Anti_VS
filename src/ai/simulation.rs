use macroquad::prelude::*;
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
}

/// Выполняет один раунд автономной ускоренной симуляции сражения без рендеринга
pub fn run_headless_simulation(
    boss_model: &BossModel,
    director_model: &DirectorModel,
    max_duration_seconds: f32,
) -> MatchResult {
    let mut world = World::new();
    let dt = 0.05f32; // 20 тиков на каждую виртуальную секунду
    let mut sim_time = 0.0f32;
    let mut director_decision_timer = 0.0f32;
    let mut altars_built = 1;

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

                // 1.2 Призыв отрядов монстров (Тиры 1 - 8) на основе весов модели Директора
                let remaining_points = if should_spawn_altar { points - BLOOD_ALTAR_COST } else { points };
                let mut max_weight = 0.0;
                for (tier_idx, &w) in director_model.squad_spawn_weights.iter().enumerate() {
                    let tier = (tier_idx + 1) as u8;
                    let config = get_squad_config(tier);
                    let is_unlocked = spent >= config.unlock_spent_threshold;
                    if is_unlocked && remaining_points >= config.cost {
                        if w > max_weight {
                            max_weight = w;
                            tier_to_spawn = Some((tier, config.cost));
                        }
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
                    }
                }
            }
        }

        // 2. Системные обновления игрового мира
        update_director(&mut world, dt);
        update_boss_ai_sim(&mut world, dt, boss_model);
        update_meditation(&mut world, dt);
        update_skills(&mut world, dt);
        update_movement(&mut world, dt);
        update_combat(&mut world, dt);
    }

    let boss_won = world.boss_health.is_alive() && sim_time >= max_duration_seconds;
    let final_level = world.boss_inventory.level;
    let dmg_to_boss = world.total_damage_dealt_to_boss;

    MatchResult {
        survival_time: sim_time,
        boss_level: final_level,
        boss_won,
        damage_dealt_to_boss: dmg_to_boss,
        altars_built,
        altars_destroyed: altars_built.saturating_sub(world.structures.len()),
    }
}

/// Симуляция ИИ Босса с использованием параметров обученной модели и нейросети
fn update_boss_ai_sim(world: &mut World, dt: f32, model: &BossModel) {
    if world.boss_dash_cooldown > 0.0 {
        world.boss_dash_cooldown = (world.boss_dash_cooldown - dt).max(0.0);
    }
    if world.boss_invulnerable_timer > 0.0 {
        world.boss_invulnerable_timer = (world.boss_invulnerable_timer - dt).max(0.0);
    }

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
                0.15 + nn_out[7] * 0.35,
                nn_out[6] > 0.70,
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

    // 2. Притяжение к кристаллам душ с весом модели
    let mut closest_gem_dist_sq = 280.0 * 280.0;
    let mut gem_pull = Vec2::ZERO;
    for gem in &world.gems {
        let dist_sq = gem.pos.distance_squared(boss_pos);
        if dist_sq < closest_gem_dist_sq {
            closest_gem_dist_sq = dist_sq;
            let strength = if gem.is_magnet { gem_pull_weight * 1.8 } else { gem_pull_weight };
            gem_pull = crate::core::math::calculate_attract_force(boss_pos, gem.pos, 280.0, strength);
        }
    }
    total_force += gem_pull;

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
        .filter(|m| m.tier == 3)
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

    // 5. Отталкивание от стен
    let map_w = MAP_WIDTH;
    let map_h = MAP_HEIGHT;
    let wall_margin = 80.0;
    let push = model.wall_repel_weight;

    if boss_pos.x < wall_margin {
        total_force.x += push * (1.0 - boss_pos.x / wall_margin).clamp(0.0, 1.0);
    } else if boss_pos.x > map_w - wall_margin {
        total_force.x -= push * (1.0 - (map_w - boss_pos.x) / wall_margin).clamp(0.0, 1.0);
    }
    if boss_pos.y < wall_margin {
        total_force.y += push * (1.0 - boss_pos.y / wall_margin).clamp(0.0, 1.0);
    } else if boss_pos.y > map_h - wall_margin {
        total_force.y -= push * (1.0 - (map_h - boss_pos.y) / wall_margin).clamp(0.0, 1.0);
    }

    // 6. Экстренный рывок (Dash)
    let should_dash = (mobs_in_close >= dash_threshold || hp_ratio < retreat_hp || (neural_dash && mobs_in_close >= 4))
        && world.boss_dash_cooldown <= 0.0;

    if should_dash {
        let dash_dir = if total_force.length_squared() > 1.0 {
            total_force.normalize()
        } else {
            Vec2::X
        };
        let dash_target = (boss_pos + dash_dir * 180.0)
            .clamp(Vec2::new(40.0, 40.0), Vec2::new(map_w - 40.0, map_h - 40.0));

        world.boss_pos = dash_target;
        world.boss_invulnerable_timer = 0.25;
        world.boss_dash_cooldown = 7.0;
    } else {
        let max_speed = 160.0;
        let desired = total_force.clamp_length_max(max_speed);
        world.boss_vel = world.boss_vel.lerp(desired, (dt * 10.0).min(1.0));
        world.boss_pos += world.boss_vel * dt;
    }

    world.boss_pos.x = world.boss_pos.x.clamp(35.0, map_w - 35.0);
    world.boss_pos.y = world.boss_pos.y.clamp(35.0, map_h - 35.0);
}
