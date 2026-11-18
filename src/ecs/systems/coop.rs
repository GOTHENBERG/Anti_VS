use macroquad::prelude::Vec2;

use crate::ecs::world::World;
use crate::ecs::systems::boss_ai::handle_boss_level_up;

pub const REVIVE_RADIUS: f32 = 50.0;
pub const REVIVE_SECONDS: f32 = 6.0;
const REVIVE_HEALTH_FRACTION: f32 = 0.35;

/// Applies queued level-ups for the Boss currently loaded into `World`'s legacy fields.
pub fn apply_pending_level_ups(world: &mut World) {
    while world.boss_pending_level_ups > 0 {
        world.boss_pending_level_ups -= 1;
        let was_alive = world.boss_health.is_alive();
        world.boss_health.max += 40.0;
        if was_alive {
            world.boss_health.current = (world.boss_health.current + 30.0).min(world.boss_health.max);
        }
        world.boss_resistances.flat_armor += 0.5;
        handle_boss_level_up(world);
        if !was_alive { world.boss_health.current = 0.0; }
    }
}

/// Ticks cooldowns and per-player damage telemetry for every non-primary actor.
pub fn tick_coop_boss_timers(world: &mut World, dt: f32) {
    for player in &mut world.coop_bosses {
        if player.human_controlled {
            continue;
        }
        player.boss_dash_cooldown = (player.boss_dash_cooldown - dt).max(0.0);
        player.boss_invulnerable_timer = (player.boss_invulnerable_timer - dt).max(0.0);
        player.boss_inventory.perk_announcement_timer =
            (player.boss_inventory.perk_announcement_timer - dt).max(0.0);
        player.boss_inventory.update_damage_decay(dt);
    }
}

/// Runs Boss-only simulation systems under each player's swapped field context.
pub fn update_all_boss_effects(world: &mut World, dt: f32) {
    apply_pending_level_ups(world);
    if world.boss_health.is_alive() {
        crate::ecs::systems::meditation::update_meditation(world, dt);
        crate::ecs::systems::skills::update_skills(world, dt);
    } else {
        world.game_time += dt;
        crate::ecs::systems::skills::update_world_structures(world, dt);
    }
    for index in 0..world.coop_bosses.len() {
        if !world.swap_coop_boss_context(index) { continue; }
        apply_pending_level_ups(world);
        if world.boss_health.is_alive() {
            crate::ecs::systems::meditation::update_meditation(world, dt);
            crate::ecs::systems::skills::update_skills(world, dt);
        }
        world.swap_coop_boss_context(index);
    }
    crate::ecs::systems::skills::update_enemy_mob_attacks(world, dt);
}

/// Runs collision, damage, XP pickup and death processing once for each Boss actor.
pub fn update_all_boss_combat(world: &mut World, dt: f32) {
    if world.boss_health.is_alive() {
        crate::ecs::systems::combat::update_combat(world, dt);
    }
    for index in 0..world.coop_bosses.len() {
        if !world.coop_bosses[index].boss_health.is_alive() { continue; }
        if !world.swap_coop_boss_context(index) { continue; }
        crate::ecs::systems::combat::update_combat(world, dt);
        world.swap_coop_boss_context(index);
    }
}

/// Applies the same Director poison, altar miasma and wall collision to each additional player.
pub fn update_coop_boss_environment(world: &mut World, dt: f32) {
    let pestilence = world.directors.first().map_or(0, |director| director.grade_pestilence);
    let altar_positions: Vec<Vec2> = world.structures.iter()
        .filter(|structure| structure.health.is_alive()
            && crate::content::structures::is_altar_structure_type(structure.structure_type))
        .map(|structure| structure.pos)
        .collect();
    let wall_positions: Vec<(Vec2, f32)> = world.structures.iter()
        .filter(|structure| structure.health.is_alive()
            && structure.structure_type == crate::content::structures::STRUCTURE_WALL)
        .map(|structure| (structure.pos, structure.radius))
        .collect();

    for player in &mut world.coop_bosses {
        if !player.boss_health.is_alive() { continue; }
        if player.boss_poison_timer > 0.0 {
            player.boss_poison_timer = (player.boss_poison_timer - dt).max(0.0);
            let resistance = player.boss_resistances.poison_res.clamp(-0.5, 0.85);
            let damage = player.boss_poison_dps * (1.0 - resistance) * dt;
            player.boss_health.take_damage(damage);
            player.boss_inventory.record_damage_taken(damage, crate::ecs::components::combat::DamageType::Poison,
                crate::ecs::components::inventory::DamageSourceKind::Hazard);
            world.total_damage_dealt_to_boss += damage;
            if damage > 0.0 { player.boss_damage_taken_timer = 3.0; player.boss_still_time = 0.0; }
            if player.boss_poison_timer <= 0.0 { player.boss_poison_dps = 0.0; }
        }
        if pestilence > 0 && altar_positions.iter().any(|pos| pos.distance_squared(player.boss_pos) < 160.0 * 160.0) {
            let resistance = player.boss_resistances.poison_res.clamp(-0.5, 0.85);
            let damage = (10.0 + pestilence as f32 * 2.5) * (1.0 - resistance) * dt;
            player.boss_health.take_damage(damage);
            player.boss_inventory.record_damage_taken(damage, crate::ecs::components::combat::DamageType::Poison,
                crate::ecs::components::inventory::DamageSourceKind::Hazard);
            world.total_damage_dealt_to_boss += damage;
            if damage > 0.0 { player.boss_damage_taken_timer = 3.0; player.boss_still_time = 0.0; }
        }
        for &(wall_pos, radius) in &wall_positions {
            let diff = player.boss_pos - wall_pos;
            let min_dist = 18.0 + radius;
            let dist_sq = diff.length_squared();
            if dist_sq < min_dist * min_dist && dist_sq > 0.001 {
                player.boss_pos = wall_pos + diff / dist_sq.sqrt() * min_dist;
            }
        }
    }
}

/// Progresses revive channels for downed Boss players. A dead player's XP and
/// inventory stay on their player state; reviving only restores a portion of HP.
pub fn update_revives(world: &mut World, dt: f32) {
    let mut alive_positions: Vec<Vec2> = Vec::with_capacity(world.coop_bosses.len() + 1);
    if world.boss_health.is_alive() {
        alive_positions.push(world.boss_pos);
    }
    alive_positions.extend(
        world
            .coop_bosses
            .iter()
            .filter(|player| player.boss_health.is_alive())
            .map(|player| player.boss_pos),
    );

    if world.boss_health.is_alive() {
        world.boss_revive_progress = 0.0;
    } else if alive_positions
        .iter()
        .any(|pos| pos.distance_squared(world.boss_pos) <= REVIVE_RADIUS * REVIVE_RADIUS)
    {
        // A downed primary cannot be present in alive_positions, so this branch
        // is satisfied only by a living co-op player.
        world.boss_revive_progress += dt;
        if world.boss_revive_progress >= REVIVE_SECONDS {
            world.boss_health.current = (world.boss_health.max * REVIVE_HEALTH_FRACTION).max(1.0);
            world.boss_invulnerable_timer = 1.0;
            world.boss_revive_progress = 0.0;
            world.add_floating_text(world.boss_pos, "СОЮЗНИК ВОСКРЕШЁН".to_string(), macroquad::prelude::SKYBLUE, 1.5);
        }
    } else {
        world.boss_revive_progress = 0.0;
    }

    for index in 0..world.coop_bosses.len() {
        if world.coop_bosses[index].boss_health.is_alive() {
            world.coop_bosses[index].revive_progress = 0.0;
            continue;
        }
        let target_pos = world.coop_bosses[index].boss_pos;
        let has_reviver = alive_positions
            .iter()
            .any(|pos| pos.distance_squared(target_pos) <= REVIVE_RADIUS * REVIVE_RADIUS);
        if !has_reviver {
            world.coop_bosses[index].revive_progress = 0.0;
            continue;
        }

        world.coop_bosses[index].revive_progress += dt;
        if world.coop_bosses[index].revive_progress >= REVIVE_SECONDS {
            let player = &mut world.coop_bosses[index];
            player.boss_health.current = (player.boss_health.max * REVIVE_HEALTH_FRACTION).max(1.0);
            player.boss_invulnerable_timer = 1.0;
            player.revive_progress = 0.0;
            let pos = player.boss_pos;
            world.add_floating_text(pos, "СОЮЗНИК ВОСКРЕШЁН".to_string(), macroquad::prelude::SKYBLUE, 1.5);
        }
    }
}
