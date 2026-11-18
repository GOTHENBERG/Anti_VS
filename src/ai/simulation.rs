use macroquad::prelude::*;
use crate::core::rand;
use crate::ai::models::{BossModel, DirectorModel};
use crate::ai::neural_net::{
    DirectorActionContext, BOSS_DASH_BASE_COOLDOWN_SECONDS,
    INPUT_SIZE, OUT_DASH, OUT_DASH_X, OUT_DASH_Y, OUT_MOVE_MAG, OUT_MOVE_X, OUT_MOVE_Y, OUT_REST,
};
use crate::content::mobs::{get_squad_config, spawn_squad_in_sector};
use crate::content::structures::{
    can_place_blood_altar, spawn_blood_altar, BLOOD_ALTAR_COST,
};
use crate::core::{MAP_HEIGHT, MAP_WIDTH};
use crate::ecs::components::combat::Faction;
use crate::ecs::systems::{
    director::update_director,
    movement::update_movement,
};
use crate::ecs::world::World;

/// Решение Босса, закэшированное между «мыслями»; по умолчанию обучение решает на каждом 20-Гц тике.
/// Частоту можно понизить параметром --think-ms, не меняя частоту физики.
#[derive(Clone, Copy, Debug)]
struct BossAction {
    move_dir: Vec2,
    move_mag: f32,
    dash: bool,
    dash_dir: Vec2,
    rest: bool,
}

#[derive(Clone, Debug)]
pub struct MatchResult {
    pub survival_time: f32,
    pub boss_level: u32,
    pub boss_won: bool,
    pub damage_dealt_to_boss: f32,
    pub boss_max_health: f32,
    pub altars_built: usize,
    pub altars_destroyed: usize,
    pub obelisks_deactivated: usize,
    pub total_altar_uptime_seconds: f32,
    pub director_points_spent: f32,
    pub director_points_unspent: f32,
    pub mobs_spawned_by_tier: [usize; 16],
    pub director_grades: (u32, u32, u32),
    pub total_meditation_time: f32,
    pub distance_walked: f32,
    pub boss_dashes: usize,
    pub boss_damage_stats: crate::ecs::components::inventory::BossDamageStats,
    pub final_weapons: Vec<(crate::ecs::components::inventory::WeaponType, u8)>,
    pub final_passives: Vec<(crate::ecs::components::inventory::PassiveType, u8)>,
    /// Лимит длительности матча (для нормализации фитнеса между поколениями).
    pub match_cap: f32,
    /// Seed матча: позволяет воспроизвести любой результат.
    pub seed: u64,
}

/// Выполняет один раунд автономной ускоренной симуляции сражения без рендеринга.
///
/// `seed` детерминирует весь матч: одинаковые модели + одинаковый seed дают
/// идентичный результат при любом количестве потоков Rayon.
pub fn run_headless_simulation(
    boss_model: &BossModel,
    director_model: &DirectorModel,
    seed: u64,
    max_duration_seconds: f32,
    boss_decision_interval: f32,
) -> MatchResult {
    run_headless_simulation_with_boss_count(
        boss_model,
        director_model,
        seed,
        max_duration_seconds,
        boss_decision_interval,
        1,
    )
}

pub fn run_headless_simulation_with_boss_count(
    boss_model: &BossModel,
    director_model: &DirectorModel,
    seed: u64,
    max_duration_seconds: f32,
    boss_decision_interval: f32,
    boss_count: usize,
) -> MatchResult {
    rand::seed(seed);

    let mut world = World::new();
    world.is_headless = true;
    world.boss_model = Some(boss_model.clone());
    let boss_count = boss_count.clamp(1, 8);
    for player_index in 1..boss_count {
        let angle = player_index as f32 * std::f32::consts::TAU / boss_count as f32;
        let offset = Vec2::new(angle.cos(), angle.sin()) * 140.0;
        world.add_coop_boss(
            player_index as u8 + 1,
            Vec2::new(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5) + offset,
            format!("AI-{}", player_index + 1),
            (player_index % 5) as u8,
        );
    }

    let dt = 0.05f32; // 20 тиков на каждую виртуальную секунду
    let boss_decision_interval = boss_decision_interval.max(dt);
    let mut sim_time = 0.0f32;
    let mut director_decision_timer = 0.4; // решение в первом же тике
    let mut altars_built = 1;
    let mut total_altar_uptime_seconds = 0.0f32;
    let mut total_meditation_time = 0.0f32;
    let mut total_distance_walked = 0.0f32;
    let mut total_dashes = 0usize;
    let mut mobs_spawned_by_tier = [0usize; 16];

    // Начальный алтарь
    spawn_blood_altar(&mut world, Vec2::new(MAP_WIDTH * 0.5 - 280.0, MAP_HEIGHT * 0.5));
    crate::content::structures::spawn_corner_obelisks(&mut world);

    let mut cached_action: Option<BossAction> = None;
    let mut coop_cached_actions = vec![None; world.coop_bosses.len()];
    let mut coop_think_timers = vec![boss_decision_interval; world.coop_bosses.len()];
    let mut boss_think_timer = boss_decision_interval;

    while world.any_boss_alive() && sim_time < max_duration_seconds {
        sim_time += dt;
        director_decision_timer += dt;

        // 1. Тактические решения Директора (каждые 0.4 сек)
        if director_decision_timer >= 0.4 {
            director_decision_timer -= 0.4;
            run_director_decision(
                &mut world,
                sim_time,
                director_model,
                &mut altars_built,
                &mut mobs_spawned_by_tier,
            );
        }

        // 2. Системные обновления мира
        update_director(&mut world, dt);

        if world.boss_dash_cooldown > 0.0 {
            world.boss_dash_cooldown = (world.boss_dash_cooldown - dt).max(0.0);
        }
        if world.boss_invulnerable_timer > 0.0 {
            world.boss_invulnerable_timer = (world.boss_invulnerable_timer - dt).max(0.0);
        }
        world.boss_inventory.update_damage_decay(dt);

        // 3. Мышление Босса и применение закэшированного действия
        boss_think_timer += dt;
        if cached_action.is_none() || boss_think_timer >= boss_decision_interval {
            boss_think_timer = 0.0;
            cached_action = Some(think_boss(&world, boss_model));
        }
        let action = cached_action.unwrap_or(BossAction {
            move_dir: Vec2::ZERO,
            move_mag: 0.0,
            dash: false,
            dash_dir: Vec2::X,
            rest: false,
        });
        let (walk_dist, dashed) = if world.boss_health.is_alive() {
            apply_boss_action(&mut world, action, dt)
        } else {
            (0.0, false)
        };
        total_distance_walked += walk_dist;
        if dashed {
            total_dashes += 1;
        }

        for index in 0..world.coop_bosses.len() {
            if !world.coop_bosses[index].boss_health.is_alive() { continue; }
            coop_think_timers[index] += dt;
            if coop_cached_actions[index].is_none() || coop_think_timers[index] >= boss_decision_interval {
                coop_think_timers[index] = 0.0;
                world.swap_coop_boss_context(index);
                coop_cached_actions[index] = Some(think_boss(&world, boss_model));
                world.swap_coop_boss_context(index);
            }
            let coop_action = coop_cached_actions[index].unwrap_or(BossAction {
                move_dir: Vec2::ZERO,
                move_mag: 0.0,
                dash: false,
                dash_dir: Vec2::X,
                rest: false,
            });
            world.swap_coop_boss_context(index);
            let (walk_dist, dashed) = apply_boss_action(&mut world, coop_action, dt);
            world.swap_coop_boss_context(index);
            total_distance_walked += walk_dist;
            if dashed { total_dashes += 1; }
        }

        crate::ecs::systems::coop::tick_coop_boss_timers(&mut world, dt);
        crate::ecs::systems::coop::update_all_boss_effects(&mut world, dt);
        if world.boss_still_time >= 1.2 {
            total_meditation_time += dt;
        }
        total_meditation_time += world.coop_bosses.iter().filter(|p| p.boss_still_time >= 1.2).count() as f32 * dt;
        update_movement(&mut world, dt);
        crate::ecs::systems::coop::update_coop_boss_environment(&mut world, dt);
        crate::ecs::systems::coop::update_all_boss_combat(&mut world, dt);
        crate::ecs::systems::coop::update_revives(&mut world, dt);

        // Учитываем только алтари: стены и обелиски не должны раздувать награду за uptime.
        for st in &world.structures {
            if crate::content::structures::is_altar_structure_type(st.structure_type)
                && st.health.is_alive()
            {
                total_altar_uptime_seconds += dt;
            }
        }
    }

    let boss_won = world.any_boss_alive();
    let total_levels = world.boss_inventory.level as usize
        + world.coop_bosses.iter().map(|player| player.boss_inventory.level as usize).sum::<usize>();
    let final_level = (total_levels / world.boss_player_count()) as u32;
    let dmg_to_boss = world.total_damage_dealt_to_boss;
    let (director_grades, points_spent, points_unspent) = world
        .directors
        .first()
        .map(|d| {
            (
                (d.grade_fury, d.grade_carapace, d.grade_pestilence),
                d.total_points_spent,
                d.points,
            )
        })
        .unwrap_or(((0, 0, 0), 0.0, 0.0));
    let final_weapons = world.boss_inventory.weapons.iter().map(|w| (w.kind, w.level)).collect();
    let final_passives = world.boss_inventory.passives.iter().map(|p| (p.kind, p.level)).collect();
    MatchResult {
        survival_time: sim_time,
        boss_level: final_level,
        boss_won,
        damage_dealt_to_boss: dmg_to_boss,
        boss_max_health: world.boss_health.max + world.coop_bosses.iter().map(|player| player.boss_health.max).sum::<f32>(),
        altars_built,
        altars_destroyed: world.boss_altars_destroyed_total,
        obelisks_deactivated: world.boss_obelisks_deactivated_total,
        total_altar_uptime_seconds,
        director_points_spent: points_spent,
        director_points_unspent: points_unspent,
        mobs_spawned_by_tier,
        director_grades,
        total_meditation_time,
        distance_walked: total_distance_walked,
        boss_dashes: total_dashes,
        boss_damage_stats: world.boss_inventory.damage_stats.clone(),
        final_weapons,
        final_passives,
        match_cap: max_duration_seconds,
        seed,
    }
}

/// Прямой проход сети Босса с новой семантикой 7 выходов.
fn think_boss(world: &World, model: &BossModel) -> BossAction {
    if let Some(ref net) = model.neural_net {
        let obs: [f32; INPUT_SIZE] = crate::ai::sensors::extract_boss_observations(world);
        let out = net.forward(&obs);

        let intent = Vec2::new(out[OUT_MOVE_X], out[OUT_MOVE_Y]);
        let len_sq = intent.length_squared();
        let move_dir = if len_sq > 0.0025 { intent / len_sq.sqrt() } else { Vec2::ZERO };
        let is_moving = out[OUT_MOVE_MAG] >= 0.5 && out[OUT_REST] <= 0.5;
        let move_mag = if is_moving { 1.0 } else { 0.0 };
        let rest = !is_moving;

        let dash_raw = Vec2::new(out[OUT_DASH_X], out[OUT_DASH_Y]);
        let dash_dir = if dash_raw.length_squared() > 0.04 {
            dash_raw.normalize()
        } else if move_dir.length_squared() > 0.5 {
            move_dir
        } else {
            Vec2::X
        };

        BossAction {
            move_dir,
            move_mag,
            dash: out[OUT_DASH] > 0.5,
            dash_dir,
            rest,
        }
    } else {
        // Fallback: простая эвристика выживания для моделей без нейросети.
        let boss_pos = world.boss_pos;
        let mut dir = Vec2::ZERO;
        let mut buf: Vec<usize> = Vec::with_capacity(64);
        world.spatial_grid.query_nearby_into(boss_pos, 220.0, &mut buf);
        for idx in buf.iter().copied() {
            if let Some(mob) = world.mobs.get(idx) {
                if !mob.health.is_alive() {
                    continue;
                }
                let diff = boss_pos - mob.pos;
                let dist = diff.length().max(1.0);
                if dist < 220.0 {
                    dir += diff / dist * (1.0 - dist / 220.0) * 3.0;
                }
            }
        }
        let center = Vec2::new(MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5);
        let to_center = center - boss_pos;
        if to_center.length_squared() > 1.0 {
            dir += to_center.normalize() * 0.4;
        }
        let move_dir = if dir.length_squared() > 0.01 { dir.normalize() } else { Vec2::ZERO };
        BossAction {
            move_dir,
            move_mag: 1.0,
            dash: false,
            dash_dir: move_dir,
            rest: false,
        }
    }
}

/// Применяет действие Босса к миру. Полностью совпадает с рантаймом boss_ai.rs.
fn apply_boss_action(world: &mut World, action: BossAction, dt: f32) -> (f32, bool) {
    let map_w = MAP_WIDTH;
    let map_h = MAP_HEIGHT;

    if action.dash && world.boss_dash_cooldown <= 0.0 {
        let dash_target = (world.boss_pos + action.dash_dir * 180.0)
            .clamp(Vec2::new(60.0, 60.0), Vec2::new(map_w - 60.0, map_h - 60.0));

        world.boss_pos = dash_target;
        world.boss_invulnerable_timer = 0.25;
        world.boss_dash_cooldown = BOSS_DASH_BASE_COOLDOWN_SECONDS
            * world.boss_inventory.get_cooldown_multiplier();

        // Рывок уничтожает вражеские снаряды на пути
        for proj in &mut world.projectiles {
            if proj.faction == Faction::Director
                && proj.pos.distance_squared(dash_target) < 130.0 * 130.0
            {
                proj.lifetime = 0.0;
            }
        }

        // Рассекающий урон в точке рывка
        let mut dash_hits: Vec<usize> = Vec::with_capacity(64);
        world.spatial_grid.query_nearby_into(dash_target, 90.0, &mut dash_hits);
        for idx in dash_hits {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.health.is_alive() && mob.pos.distance_squared(dash_target) < 90.0 * 90.0 {
                    mob.health.take_damage(80.0);
                    if !mob.knockback_immune {
                        mob.knockback_vel += action.dash_dir * 240.0;
                    }
                }
            }
        }

        (0.0, true)
    } else {
        let desired_vel = if action.rest {
            Vec2::ZERO
        } else {
            action.move_dir
                * world.boss_inventory.get_movement_speed(160.0)
                * action.move_mag.clamp(0.0, 1.0)
        };

        let prev_pos = world.boss_pos;
        world.boss_vel = desired_vel;
        world.boss_pos += desired_vel * dt;
        world.boss_pos.x = world.boss_pos.x.clamp(35.0, map_w - 35.0);
        world.boss_pos.y = world.boss_pos.y.clamp(35.0, map_h - 35.0);

        (prev_pos.distance(world.boss_pos), false)
    }
}

/// Один такт решений Директора. Возвращает всё через `world` и счётчики.
fn run_director_decision(
    world: &mut World,
    sim_time: f32,
    model: &DirectorModel,
    altars_built: &mut usize,
    mobs_spawned_by_tier: &mut [usize; 16],
) {
    let obs = crate::ai::sensors::extract_director_observations(world, sim_time);

    if let Some(ref net) = model.neural_net {
        let ctx = build_director_context(world);
        let decision = net.forward(&obs, &ctx);

        // 1. Постройка Алтаря Крови: сначала проверяем валидность места,
        //    чтобы не списывать души и кулдаун за неудачную постройку.
        if decision.should_build_altar
            && can_place_blood_altar(world, decision.altar_target_pos)
        {
            let altar_tier = decision.altar_tier;
            let altar_cost = crate::content::structures::altar_cost(altar_tier);
            let altar_cd = crate::content::structures::altar_cooldown(altar_tier);
            let spent = world
                .directors
                .first_mut()
                .map(|d| {
                    if d.altar_cooldown_for_tier(altar_tier) <= 0.0 && d.spend_points(altar_cost) {
                        d.set_altar_cooldown_for_tier(altar_tier, altar_cd);
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false);

            if spent && crate::content::structures::try_spawn_tier_altar(world, decision.altar_target_pos, altar_tier) {
                *altars_built += 1;
            }
        }

        // 2. Единая экономическая голова
        match decision.target_action {
            0 => {}
            1..=16 => {
                let tier = decision.target_action as u8;
                let config = get_squad_config(tier);
                let can_spawn = world.directors.first().map_or(false, |d| {
                    d.total_points_spent >= config.unlock_spent_threshold
                        && d.points >= config.cost
                        && d.squad_cooldowns[(tier - 1) as usize] <= 0.0
                });
                if can_spawn {
                    let paid = world
                        .directors
                        .first_mut()
                        .map(|d| {
                            if d.spend_points(config.cost) {
                                d.time_since_last_spawn = 0.0;
                                d.squad_cooldowns[(tier - 1) as usize] = config.cooldown;
                                true
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false);
                    if paid {
                        spawn_squad_in_sector(world, tier, decision.spawn_sector);
                        mobs_spawned_by_tier[(tier - 1) as usize] += config.count * world.living_boss_count();
                    }
                }
            }
            17 => {
                if let Some(director) = world.directors.first_mut() {
                    let cost = 100.0 * (director.grade_fury as f32 + 1.0);
                    if director.spend_points(cost) {
                        director.grade_fury += 1;
                    }
                }
            }
            18 => {
                let mut upgraded = false;
                if let Some(director) = world.directors.first_mut() {
                    let cost = 120.0 * (director.grade_carapace as f32 + 1.0);
                    if director.spend_points(cost) {
                        director.grade_carapace += 1;
                        upgraded = true;
                    }
                }
                if upgraded {
                    crate::content::structures::upgrade_structure_carapace(world, 1);
                }
            }
            19 => {
                if let Some(director) = world.directors.first_mut() {
                    let cost = 150.0 * (director.grade_pestilence as f32 + 1.0);
                    if director.spend_points(cost) {
                        director.grade_pestilence += 1;
                    }
                }
            }
            _ => {}
        }
    } else {
        run_director_fallback(world, model, altars_built, mobs_spawned_by_tier);
    }
}

/// Runs one AI Director action for a live match using the same policy and action masks as training.
pub fn update_director_policy(world: &mut World, sim_time: f32, model: &DirectorModel) {
    let mut altars_built = 0usize;
    let mut mobs_spawned_by_tier = [0usize; 16];
    run_director_decision(
        world,
        sim_time,
        model,
        &mut altars_built,
        &mut mobs_spawned_by_tier,
    );
}

fn build_director_context(world: &World) -> DirectorActionContext {
    let director = world.directors.first();
    DirectorActionContext {
        points: director.map_or(0.0, |d| d.points),
        total_points_spent: director.map_or(0.0, |d| d.total_points_spent),
        altar_cooldown: director.map_or(0.0, |d| d.altar_cooldown),
        altar_cost: BLOOD_ALTAR_COST,
        altar_tier_cooldowns: std::array::from_fn(|index| {
            director.map_or(0.0, |d| d.altar_cooldown_for_tier(index as u8 + 1))
        }),
        altar_tier_costs: std::array::from_fn(|index| {
            crate::content::structures::altar_cost(index as u8 + 1)
        }),
        grade_fury: director.map_or(0, |d| d.grade_fury),
        grade_carapace: director.map_or(0, |d| d.grade_carapace),
        grade_pestilence: director.map_or(0, |d| d.grade_pestilence),
        squad_cooldowns: director.map_or([0.0; 16], |d| d.squad_cooldowns),
        valid_altar_slots: crate::ai::sensors::director_altar_slot_mask(world),
    }
}

/// Старое весовое поведение Директора для моделей без нейросети.
fn run_director_fallback(
    world: &mut World,
    model: &DirectorModel,
    altars_built: &mut usize,
    mobs_spawned_by_tier: &mut [usize; 16],
) {
    let (points, spent, cooldown, squad_cooldowns) = world
        .directors
        .first()
        .map(|d| (d.points, d.total_points_spent, d.altar_cooldown, d.squad_cooldowns))
        .unwrap_or((0.0, 0.0, 0.0, [0.0; 16]));

    // 1. Алтарь
    if cooldown <= 0.0
        && points >= BLOOD_ALTAR_COST
        && rand::gen_range(0.0, 1.0) < model.altar_build_priority
    {
        let mut placed = false;
        for _ in 0..8 {
            let angle = rand::gen_range(0.0, std::f32::consts::TAU);
            let dist = rand::gen_range(350.0, 750.0);
            let candidate = (world.boss_pos
                + Vec2::new(angle.cos() * dist, angle.sin() * dist))
            .clamp(Vec2::new(60.0, 60.0), Vec2::new(MAP_WIDTH - 60.0, MAP_HEIGHT - 60.0));
            if can_place_blood_altar(world, candidate) {
                if let Some(director) = world.directors.first_mut() {
                    if director.spend_points(BLOOD_ALTAR_COST) {
                        director.altar_cooldown = director.altar_max_cooldown;
                        spawn_blood_altar(world, candidate);
                        placed = true;
                        break;
                    }
                }
            }
        }
        if placed {
            *altars_built += 1;
        }
    }

    // 2. Отряд по весам
    let remaining = world.directors.first().map_or(0.0, |d| d.points);
    let mut eligible = Vec::with_capacity(16);
    let mut total_weight = 0.0f32;
    for (tier_idx, &w) in model.squad_spawn_weights.iter().enumerate() {
        if tier_idx >= 16 {
            break;
        }
        let config = get_squad_config((tier_idx + 1) as u8);
        if spent >= config.unlock_spent_threshold
            && remaining >= config.cost
            && squad_cooldowns[tier_idx] <= 0.0
            && w > 0.0
        {
            eligible.push((tier_idx + 1, config.cost, config.count, config.cooldown, w));
            total_weight += w;
        }
    }

    if total_weight > 0.0 {
        let mut roll = rand::gen_range(0.0, total_weight);
        let chosen = eligible
            .iter()
            .copied()
            .find(|(_, _, _, _, w)| {
                if roll <= *w {
                    true
                } else {
                    roll -= *w;
                    false
                }
            });

        if let Some((tier, cost, count, cooldown, _)) = chosen {
            let paid = world
                .directors
                .first_mut()
                .map(|d| {
                    if d.spend_points(cost) {
                        d.time_since_last_spawn = 0.0;
                        d.squad_cooldowns[tier - 1] = cooldown;
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false);
            if paid {
                let sector = rand::gen_range(0usize, 8);
                spawn_squad_in_sector(world, tier as u8, sector);
                mobs_spawned_by_tier[tier - 1] += count * world.living_boss_count();
            }
        }
    }

    // 3. Грейды: выравниваем отстающие
    let carapace_before = world.directors.first().map_or(0, |d| d.grade_carapace);
    if let Some(director) = world.directors.first_mut() {
        let fury_cost = 100.0 * (director.grade_fury as f32 + 1.0);
        let carapace_cost = 120.0 * (director.grade_carapace as f32 + 1.0);
        let pestilence_cost = 150.0 * (director.grade_pestilence as f32 + 1.0);

        if director.points >= pestilence_cost
            && director.grade_pestilence <= director.grade_fury
        {
            if director.spend_points(pestilence_cost) {
                director.grade_pestilence += 1;
            }
        } else if director.points >= carapace_cost
            && director.grade_carapace <= director.grade_fury
        {
            if director.spend_points(carapace_cost) {
                director.grade_carapace += 1;
            }
        } else if director.points >= fury_cost && director.spend_points(fury_cost) {
            director.grade_fury += 1;
        }
    }
    let carapace_after = world.directors.first().map_or(0, |d| d.grade_carapace);
    crate::content::structures::upgrade_structure_carapace(
        world,
        carapace_after.saturating_sub(carapace_before),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::neural_net::BOSS_TRAINING_DECISION_INTERVAL;

    #[test]
    fn deterministic_seed_reproducible() {
        let boss = BossModel::default();
        let director = DirectorModel::default();

        let a = run_headless_simulation(
            &boss,
            &director,
            42,
            30.0,
            BOSS_TRAINING_DECISION_INTERVAL,
        );
        let b = run_headless_simulation(
            &boss,
            &director,
            42,
            30.0,
            BOSS_TRAINING_DECISION_INTERVAL,
        );

        assert_eq!(a.survival_time.to_bits(), b.survival_time.to_bits());
        assert_eq!(
            a.damage_dealt_to_boss.to_bits(),
            b.damage_dealt_to_boss.to_bits()
        );
        assert_eq!(a.boss_level, b.boss_level);
        assert_eq!(
            a.director_points_spent.to_bits(),
            b.director_points_spent.to_bits()
        );
        assert_eq!(a.altars_built, b.altars_built);
        assert_eq!(a.mobs_spawned_by_tier, b.mobs_spawned_by_tier);
    }
}
