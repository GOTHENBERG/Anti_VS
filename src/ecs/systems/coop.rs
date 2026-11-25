use macroquad::prelude::Vec2;

use crate::ecs::world::World;
use crate::ecs::systems::boss_ai::handle_boss_level_up;

pub const REVIVE_RADIUS: f32 = 50.0;
pub const REVIVE_SECONDS: f32 = 6.0;
const REVIVE_HEALTH_FRACTION: f32 = 0.35;

/// Applies queued level-ups for the Boss currently loaded into `World`'s legacy fields.
pub fn apply_pending_level_ups(world: &mut World) {
    if world.boss_human_controlled {
        // Для игрока-человека не применяем перки автоматически:
        // генерируем 3 карты на выбор, если они ещё не открыты
        if world.boss_level_up_options.is_none() {
            // В первую очередь открываем выбор легендарных реликвий (Дар Разлома за Обелиск)
            if world.boss_pending_relic_choices > 0 {
                let options = crate::ecs::systems::boss_ai::generate_relic_choices(&world.boss_inventory);
                if !options.is_empty() {
                    world.boss_level_up_options = Some(options);
                } else {
                    world.boss_pending_relic_choices -= 1;
                    world.boss_health.max += 250.0;
                    world.boss_health.heal(250.0);
                    world.boss_resistances.flat_armor += 3.0;
                }
                return;
            }

            // Стандартный выбор перка при level-up
            if world.boss_pending_level_ups > 0 {
                let options = crate::ecs::systems::boss_ai::generate_perk_choices(&world.boss_inventory);
                if !options.is_empty() {
                    world.boss_level_up_options = Some(options);
                } else {
                    world.boss_pending_level_ups -= 1;
                    let was_alive = world.boss_health.is_alive();
                    world.boss_health.max += 40.0;
                    if was_alive {
                        world.boss_health.current = (world.boss_health.current + 30.0).min(world.boss_health.max);
                    }
                    world.boss_resistances.flat_armor += 0.5;
                    if !was_alive { world.boss_health.current = 0.0; }
                }
            }
        }
        return;
    }

    // Для AI-босса:
    while world.boss_pending_relic_choices > 0 {
        world.boss_pending_relic_choices -= 1;
        let options = crate::ecs::systems::boss_ai::generate_relic_choices(&world.boss_inventory);
        if let Some(first) = options.first().cloned() {
            crate::ecs::systems::boss_ai::apply_perk_choice(world, first);
        } else {
            world.boss_health.max += 250.0;
            world.boss_health.heal(250.0);
            world.boss_resistances.flat_armor += 3.0;
        }
    }

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

/// Применяет перк, выбранный игроком-Боссом из модального окна Level-Up.
pub fn apply_player_perk_choice(world: &mut World, choice: crate::ecs::components::inventory::PerkChoice) {
    if world.boss_pending_relic_choices > 0 {
        world.boss_pending_relic_choices -= 1;
        crate::ecs::systems::boss_ai::apply_perk_choice(world, choice);
    } else if world.boss_pending_level_ups > 0 {
        world.boss_pending_level_ups -= 1;
        let was_alive = world.boss_health.is_alive();
        world.boss_health.max += 40.0;
        if was_alive {
            world.boss_health.current = (world.boss_health.current + 30.0).min(world.boss_health.max);
        }
        world.boss_resistances.flat_armor += 0.5;
        crate::ecs::systems::boss_ai::apply_perk_choice(world, choice);
        if !was_alive { world.boss_health.current = 0.0; }
    } else {
        crate::ecs::systems::boss_ai::apply_perk_choice(world, choice);
    }
    world.boss_level_up_options = None;
}

/// Применяет перк по индексу для конкретного игрока (0 = хост-босс, 1..N = кооп-клиенты).
pub fn apply_player_perk_choice_by_index(world: &mut World, player_id: u8, choice_index: u8) {
    if player_id == 0 {
        if let Some(options) = world.boss_level_up_options.take() {
            if let Some(choice) = options.get(choice_index as usize).cloned() {
                apply_player_perk_choice(world, choice);
            }
        }
    } else {
        let coop_idx = (player_id - 1) as usize;
        if world.swap_coop_boss_context(coop_idx) {
            if let Some(options) = world.boss_level_up_options.take() {
                if let Some(choice) = options.get(choice_index as usize).cloned() {
                    apply_player_perk_choice(world, choice);
                }
            }
            world.swap_coop_boss_context(coop_idx);
        }
    }
}

pub const MAX_SOUL_LINK_LENGTH: f32 = 1000.0;

/// Проверяет расстояние от точки p до отрезка ab.
pub fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> (f32, Vec2) {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 0.0001 {
        return (p.distance(a), a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let proj = a + ab * t;
    (p.distance(proj), proj)
}

/// Обновляет связующую Энергетическую цепь (Soul Link) между всеми живыми боссами.
/// Кольцевая топология, ограничение длины 1000 px, рассекающий урон мобам.
pub fn update_soul_link(world: &mut World, dt: f32) {
    // Собираем позиции и индексы всех живых боссов
    let mut alive_bosses: Vec<(usize, Vec2)> = Vec::new(); // 0 = primary, 1..N = coop
    if world.boss_health.is_alive() {
        alive_bosses.push((0, world.boss_pos));
    }
    for (i, coop) in world.coop_bosses.iter().enumerate() {
        if coop.boss_health.is_alive() {
            alive_bosses.push((i + 1, coop.boss_pos));
        }
    }

    let n = alive_bosses.len();
    if n < 2 {
        world.soul_link_tension = 0.0;
        return;
    }

    // Формируем кольцевую топологию звеньев по индексам внутри `alive_bosses`
    let mut link_pairs: Vec<(usize, usize)> = Vec::with_capacity(n);
    if n == 2 {
        link_pairs.push((0, 1));
    } else {
        for i in 0..n {
            link_pairs.push((i, (i + 1) % n));
        }
    }

    // 1. XPBD (Extended Position-Based Dynamics) ограничение длины цепи (максимум 1000 px).
    // В отличие от простого пружинного `pull` и классического PBD, податливость `alpha = compliance / dt^2`
    // не взрывается на больших dt и возвращает физическую силу натяжения через множитель Лагранжа `lambda`.
    let mut positions: Vec<Vec2> = alive_bosses.iter().map(|&(_, p)| p).collect();
    let mut lambdas = vec![0.0f32; link_pairs.len()];
    let safe_dt = dt.max(1e-4);
    const XPBD_COMPLIANCE: f32 = 0.0005;
    let alpha = XPBD_COMPLIANCE / (safe_dt * safe_dt);
    const XPBD_ITERS: usize = 4;

    for _ in 0..XPBD_ITERS {
        for (k, &(ia, ib)) in link_pairs.iter().enumerate() {
            let diff = positions[ib] - positions[ia];
            let dist = diff.length();
            if dist > MAX_SOUL_LINK_LENGTH || lambdas[k] < 0.0 {
                let c = dist - MAX_SOUL_LINK_LENGTH;
                let dir = if dist > 1e-5 { diff / dist } else { Vec2::X };
                // w_a = 1.0, w_b = 1.0 -> w_sum = 2.0; градиенты: grad_a = -dir, grad_b = +dir
                // Одностороннее ограничение: lambda <= 0 (связь тянет, но никогда не расталкивает).
                // Применяем именно разность new_lambda - old_lambda, чтобы ослабленные звенья корректно разгружались.
                let old_lambda = lambdas[k];
                let d_lambda = (-c - alpha * old_lambda) / (2.0 + alpha);
                let new_lambda = (old_lambda + d_lambda).min(0.0);
                let eff_d_lambda = new_lambda - old_lambda;
                lambdas[k] = new_lambda;
                positions[ia] -= dir * eff_d_lambda;
                positions[ib] += dir * eff_d_lambda;
            }
        }
    }

    // Записываем обновлённые позиции обратно в боссов и фиксируем нормированное натяжение [0..1+]
    let inv_dt_sq = 1.0 / (safe_dt * safe_dt);
    let mut max_tension = 0.0f32;
    for &lambda in &lambdas {
        max_tension = max_tension.max((-lambda) * inv_dt_sq);
    }
    world.soul_link_tension = (max_tension / 200000.0).clamp(0.0, 2.0);

    for (idx, &(boss_slot, _)) in alive_bosses.iter().enumerate() {
        if boss_slot == 0 {
            world.boss_pos = positions[idx];
        } else {
            world.coop_bosses[boss_slot - 1].boss_pos = positions[idx];
        }
    }

    // 2. Рассекающий урон мобам, пересекающим цепь (Thick Supercover DDA вместо перебора 20к мобов)
    let damage_slice = 260.0 * dt;
    let is_headless = world.is_headless;
    let use_spatial = !world.spatial_grid.is_empty() && world.spatial_grid.len() == world.mobs.len();

    for &(ia, ib) in &link_pairs {
        let pos_a = positions[ia];
        let pos_b = positions[ib];

        if use_spatial {
            let spatial_grid = &world.spatial_grid;
            let mobs = &mut world.mobs;
            let particles = &mut world.particles;
            // Запас толщины 176 px покрывает даже максимально разросшегося яростного Героя (радиус 160 + 14)
            spatial_grid.query_segment_thick(pos_a, pos_b, 176.0, |mob_idx| {
                if let Some(mob) = mobs.get_mut(mob_idx) {
                    if !mob.health.is_alive() {
                        return;
                    }
                    let (dist, hit_pt) = dist_to_segment(mob.pos, pos_a, pos_b);
                    if dist <= mob.radius + 14.0 {
                        mob.health.take_damage(damage_slice);
                        if !is_headless && crate::core::visual_rand::gen_range(0.0, 1.0) < 0.25 {
                            particles.push(crate::ecs::world::ParticleEntity {
                                pos: hit_pt,
                                vel: (mob.pos - hit_pt).normalize_or_zero() * 45.0,
                                color: macroquad::prelude::Color::new(0.4, 0.9, 1.0, 0.9),
                                radius: 3.5,
                                lifetime: 0.35,
                                max_lifetime: 0.35,
                            });
                        }
                    }
                }
            });
        } else {
            // Запасной путь для изолированных юнит-тестов, добавляющих мобов без rebuild_spatial_grid()
            let min_x = pos_a.x.min(pos_b.x) - 180.0;
            let max_x = pos_a.x.max(pos_b.x) + 180.0;
            let min_y = pos_a.y.min(pos_b.y) - 180.0;
            let max_y = pos_a.y.max(pos_b.y) + 180.0;

            for mob in world.mobs.iter_mut() {
                if !mob.health.is_alive() { continue; }
                if mob.pos.x < min_x || mob.pos.x > max_x || mob.pos.y < min_y || mob.pos.y > max_y {
                    continue;
                }

                let (dist, hit_pt) = dist_to_segment(mob.pos, pos_a, pos_b);
                if dist <= mob.radius + 14.0 {
                    mob.health.take_damage(damage_slice);

                    if !is_headless && crate::core::visual_rand::gen_range(0.0, 1.0) < 0.25 {
                        world.particles.push(crate::ecs::world::ParticleEntity {
                            pos: hit_pt,
                            vel: (mob.pos - hit_pt).normalize_or_zero() * 45.0,
                            color: macroquad::prelude::Color::new(0.4, 0.9, 1.0, 0.9),
                            radius: 3.5,
                            lifetime: 0.35,
                            max_lifetime: 0.35,
                        });
                    }
                }
            }
        }
    }
}

/// Ticks cooldowns and per-player damage telemetry for AI co-op Bosses when callers
/// advance actions directly (`apply_boss_action`) without calling `update_boss_ai`/`update_player_boss`.
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
    update_soul_link(world, dt);
    crate::ecs::systems::skills::update_black_holes(world, dt);
    crate::ecs::systems::skills::update_heroes_and_clones(world, dt);
}

/// Applies queued level-ups across the primary Boss and every co-op Boss.
pub fn apply_all_pending_level_ups(world: &mut World) {
    apply_pending_level_ups(world);
    for index in 0..world.coop_bosses.len() {
        if !world.swap_coop_boss_context(index) {
            continue;
        }
        apply_pending_level_ups(world);
        world.swap_coop_boss_context(index);
    }
}

/// Runs collision, damage, XP pickup and death processing once per tick for all Boss actors.
pub fn update_all_boss_combat(world: &mut World, dt: f32) {
    if world.any_boss_alive() {
        crate::ecs::systems::combat::update_combat(world, dt);
    }
}

/// Runs movement, co-op environment hazards, global combat, and revives in canonical tick order.
pub fn update_post_effects_tick(world: &mut World, dt: f32) {
    crate::ecs::systems::movement::update_movement(world, dt);
    update_coop_boss_environment(world, dt);
    update_all_boss_combat(world, dt);
    update_revives(world, dt);
}

/// Applies the same Director poison, altar miasma and wall collision to each additional player.
pub fn update_coop_boss_environment(world: &mut World, dt: f32) {
    let pestilence = world.directors.first().map_or(0, |director| director.grade_pestilence);
    let miasma_rad_sq = 160.0 * 160.0;

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
        if pestilence > 0 {
            let in_altar_miasma = world.structures.iter().any(|st| {
                st.health.is_alive()
                    && crate::content::structures::is_altar_structure_type(st.structure_type)
                    && st.pos.distance_squared(player.boss_pos) < miasma_rad_sq
            });
            if in_altar_miasma {
                let resistance = player.boss_resistances.poison_res.clamp(-0.5, 0.85);
                let damage = (10.0 + pestilence as f32 * 2.5) * (1.0 - resistance) * dt;
                player.boss_health.take_damage(damage);
                player.boss_inventory.record_damage_taken(damage, crate::ecs::components::combat::DamageType::Poison,
                    crate::ecs::components::inventory::DamageSourceKind::Hazard);
                world.total_damage_dealt_to_boss += damage;
                if damage > 0.0 { player.boss_damage_taken_timer = 3.0; player.boss_still_time = 0.0; }
            }
        }
        for st in &world.structures {
            if st.health.is_alive() && st.structure_type == crate::content::structures::STRUCTURE_WALL {
                let diff = player.boss_pos - st.pos;
                let min_dist = 18.0 + st.radius;
                let dist_sq = diff.length_squared();
                if dist_sq < min_dist * min_dist && dist_sq > 0.001 {
                    player.boss_pos = st.pos + diff / dist_sq.sqrt() * min_dist;
                }
            }
        }
    }
}

/// Progresses revive channels for downed Boss players. A dead player's XP and
/// inventory stay on their player state; reviving only restores a portion of HP.
pub fn update_revives(world: &mut World, dt: f32) {
    let mut alive_positions = [Vec2::ZERO; 8];
    let mut alive_count = 0usize;
    if world.boss_health.is_alive() {
        alive_positions[alive_count] = world.boss_pos;
        alive_count += 1;
    }
    for player in &world.coop_bosses {
        if player.boss_health.is_alive() && alive_count < 8 {
            alive_positions[alive_count] = player.boss_pos;
            alive_count += 1;
        }
    }
    let alive_slice = &alive_positions[..alive_count];

    if world.boss_health.is_alive() {
        world.boss_revive_progress = 0.0;
    } else if alive_slice
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
        let has_reviver = alive_slice
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::components::combat::Health;
    use crate::ecs::components::inventory::PerkChoice;
    use crate::ecs::world::{MobEntity, World};

    #[test]
    fn test_dist_to_segment() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(100.0, 0.0);

        // Point directly in the middle
        let (dist, proj) = dist_to_segment(Vec2::new(50.0, 10.0), a, b);
        assert!((dist - 10.0).abs() < 0.001);
        assert_eq!(proj, Vec2::new(50.0, 0.0));

        // Point before start a
        let (dist_before, proj_before) = dist_to_segment(Vec2::new(-20.0, 0.0), a, b);
        assert!((dist_before - 20.0).abs() < 0.001);
        assert_eq!(proj_before, a);

        // Point after end b
        let (dist_after, proj_after) = dist_to_segment(Vec2::new(130.0, 0.0), a, b);
        assert!((dist_after - 30.0).abs() < 0.001);
        assert_eq!(proj_after, b);
    }

    #[test]
    fn test_soul_link_physics_and_slicing_damage() {
        let mut world = World::new();
        world.is_headless = true;
        // Boss 1 at (0, 0)
        world.boss_pos = Vec2::new(0.0, 0.0);

        // Boss 2 at (1200, 0) -> distance 1200 > MAX_SOUL_LINK_LENGTH (1000)
        world.add_coop_boss(2, Vec2::new(1200.0, 0.0), "Boss2".to_string(), 0);

        // Place a mob right on the soul link line at (500, 5) with HP 100
        let mob_id = world.alloc_id();
        let mut mob = MobEntity::new_test(mob_id, Vec2::new(500.0, 5.0), 1);
        mob.health = Health::new(100.0);
        mob.radius = 12.0;
        world.mobs.push(mob);

        // Update soul link for dt = 0.1
        update_soul_link(&mut world, 0.1);

        // 1. Bosses should be pulled closer together
        assert!(world.boss_pos.x > 0.0, "Boss 1 should be pulled towards Boss 2");
        assert!(world.coop_bosses[0].boss_pos.x < 1200.0, "Boss 2 should be pulled towards Boss 1");
        let new_dist = (world.coop_bosses[0].boss_pos - world.boss_pos).length();
        assert!(new_dist < 1200.0, "Distance between bosses should decrease due to tether");

        // 2. Mob on the link line should take slicing damage (260 * 0.1 = 26 damage)
        assert!(world.mobs[0].health.current < 100.0, "Mob on soul link must take slicing damage");
        let dmg = 100.0 - world.mobs[0].health.current;
        assert!((dmg - 26.0).abs() < 0.01, "Expected ~26 damage, got {}", dmg);
    }

    #[test]
    fn test_soul_link_ring_topology_closes_on_boss_death() {
        let mut world = World::new();
        world.is_headless = true;
        world.boss_pos = Vec2::new(0.0, 0.0);
        world.add_coop_boss(2, Vec2::new(500.0, 0.0), "Boss2".to_string(), 0);
        world.add_coop_boss(3, Vec2::new(250.0, 400.0), "Boss3".to_string(), 0);

        // All 3 alive: living count = 3
        assert_eq!(world.living_boss_count(), 3);

        // Kill Boss 2
        world.coop_bosses[0].boss_health.current = 0.0;
        assert_eq!(world.living_boss_count(), 2);

        // Place mob on segment between Boss 1 (0,0) and Boss 3 (250, 400)
        let mid = (world.boss_pos + world.coop_bosses[1].boss_pos) * 0.5;
        let mob_id = world.alloc_id();
        let mut mob = MobEntity::new_test(mob_id, mid, 1);
        mob.health = Health::new(100.0);
        mob.radius = 12.0;
        world.mobs.push(mob);

        update_soul_link(&mut world, 0.1);

        // Direct link between Boss 1 and Boss 3 is active and slices the mob!
        assert!(world.mobs[0].health.current < 100.0, "Soul link must connect remaining living bosses in a ring");
    }

    #[test]
    fn test_player_perk_selection_by_index() {
        use crate::ecs::components::inventory::{PassiveType, WeaponType};

        let mut world = World::new();
        world.boss_pending_level_ups = 1;
        world.boss_level_up_options = Some(vec![
            PerkChoice::UpgradeWeapon(WeaponType::ChainLightning),
            PerkChoice::UpgradePassive(PassiveType::FrenzyIchor),
            PerkChoice::UpgradePassive(PassiveType::MartyrVanguard),
        ]);

        // Host player (0) chooses index 1 (FrenzyIchor)
        apply_player_perk_choice_by_index(&mut world, 0, 1);

        assert_eq!(world.boss_pending_level_ups, 0);
        assert!(world.boss_level_up_options.is_none());
        assert!(world.boss_inventory.has_passive(PassiveType::FrenzyIchor));

        // Test for co-op player (player_id 1)
        world.add_coop_boss(2, Vec2::new(100.0, 100.0), "CoopPlayer".to_string(), 0);
        world.coop_bosses[0].pending_level_ups = 1;
        world.coop_bosses[0].level_up_options = Some(vec![
            PerkChoice::UpgradePassive(PassiveType::MartyrVanguard),
        ]);

        apply_player_perk_choice_by_index(&mut world, 1, 0);
        assert_eq!(world.coop_bosses[0].pending_level_ups, 0);
        assert!(world.coop_bosses[0].level_up_options.is_none());
        assert!(world.coop_bosses[0].boss_inventory.has_passive(PassiveType::MartyrVanguard));
    }

    #[test]
    fn test_boss_xp_scaling_and_relic_flow() {
        use crate::ecs::components::inventory::{BossInventory, PassiveSlot, PassiveType, PerkChoice};
        use crate::content::passives::is_legendary_relic;

        // 1. Проверяем стартовый порог 65 XP и скейлинг 1.3
        let mut inv = BossInventory::new();
        assert_eq!(inv.xp_to_next_level, 65.0);
        let l_up = inv.add_xp(65.0);
        assert_eq!(l_up, 1);
        assert_eq!(inv.level, 2);
        assert_eq!(inv.xp_to_next_level, 85.0); // round(65.0 * 1.30) = 85.0

        let l_up2 = inv.add_xp(85.0);
        assert_eq!(l_up2, 1);
        assert_eq!(inv.level, 3);
        assert_eq!(inv.xp_to_next_level, 110.0); // round(85.0 * 1.30) = 110.0

        // 2. Проверяем, что при обычном level-up НОВЫЕ реликвии не выпадают
        let fresh_inv = BossInventory::new();
        for _ in 0..10 {
            let options = crate::ecs::systems::boss_ai::generate_perk_choices(&fresh_inv);
            for opt in &options {
                if let PerkChoice::UpgradePassive(kind) = opt {
                    assert!(
                        !is_legendary_relic(*kind),
                        "Normal level-up must not offer new legendary relics! Found: {:?}",
                        kind
                    );
                }
            }
        }

        // 3. Проверяем, что уже имеющуюся реликвию (полученную из Обелиска) МОЖНО прокачивать на level-up
        let mut inv_with_relic = BossInventory::new();
        inv_with_relic.passives.push(PassiveSlot {
            kind: PassiveType::RevivalPhylactery,
            level: 1,
        });
        let mut found_upgrade = false;
        for _ in 0..150 {
            let options = crate::ecs::systems::boss_ai::generate_perk_choices(&inv_with_relic);
            if options.iter().any(|o| matches!(o, PerkChoice::UpgradePassive(PassiveType::RevivalPhylactery))) {
                found_upgrade = true;
                break;
            }
        }
        assert!(found_upgrade, "Owned relic must be upgradeable via normal level-up");

        // 4. Проверяем генерацию выбора реликвий из Обелиска (generate_relic_choices)
        let relic_options = crate::ecs::systems::boss_ai::generate_relic_choices(&fresh_inv);
        assert_eq!(relic_options.len(), 3);
        for opt in &relic_options {
            if let PerkChoice::UpgradePassive(kind) = opt {
                assert!(is_legendary_relic(*kind), "Obelisk relic choices must be legendary relics");
            }
        }
    }

    #[test]
    fn test_soul_link_xpbd_lag_stability_and_thick_dda() {
        let mut world = World::new();
        world.is_headless = true;
        world.boss_pos = Vec2::new(5000.0, 5000.0);
        world.add_coop_boss(2, Vec2::new(6400.0, 5000.0), "Boss2".to_string(), 0); // dist = 1400 > 1000

        // Моб 0 прямо на луче (в центре, x=5700, y=5008)
        let id_on = world.alloc_id();
        let mut mob_on = MobEntity::new_test(id_on, Vec2::new(5700.0, 5008.0), 1);
        mob_on.health = Health::new(500.0);
        mob_on.radius = 12.0;
        world.mobs.push(mob_on);

        // Добавляем 1000 далёких фоновых мобов, которых Thick DDA обязан проигнорировать
        for i in 0..1000usize {
            let id = world.alloc_id();
            let pos = Vec2::new(8000.0 + (i % 50) as f32 * 40.0, 8000.0 + (i / 50) as f32 * 40.0);
            world.mobs.push(MobEntity::new_test(id, pos, 1));
        }
        world.rebuild_spatial_grid();

        // Огромный спайк dt = 1.0 сек: XPBD НЕ должен перелететь и взорвать позиции!
        update_soul_link(&mut world, 1.0);

        let new_dist = (world.coop_bosses[0].boss_pos - world.boss_pos).length();
        assert!(
            new_dist >= MAX_SOUL_LINK_LENGTH - 1.0 && new_dist <= 1010.0,
            "XPBD must stably clamp near MAX_SOUL_LINK_LENGTH without overshoot on large dt, got {}",
            new_dist
        );
        assert!(world.soul_link_tension > 0.0, "XPBD must report positive tether tension when taut");
        assert!(world.mobs[0].health.current < 500.0, "Thick DDA must slice mob on link");
        assert!(
            (world.mobs[1].health.current - world.mobs[1].health.max).abs() < 1e-4,
            "Distant mobs must not take slicing damage"
        );
    }

    #[test]
    fn test_soul_link_xpbd_unilateral_slack_and_ring_relaxation() {
        // 1. Слабые / ненатянутые звенья (dist < MAX_SOUL_LINK_LENGTH) не должны расталкивать боссов
        let mut world = World::new();
        world.is_headless = true;
        world.boss_pos = Vec2::new(1000.0, 1000.0);
        world.add_coop_boss(2, Vec2::new(1400.0, 1000.0), "SlackBoss".to_string(), 0);
        update_soul_link(&mut world, 0.05);
        assert_eq!(world.boss_pos, Vec2::new(1000.0, 1000.0));
        assert_eq!(world.coop_bosses[0].boss_pos, Vec2::new(1400.0, 1000.0));
        assert_eq!(world.soul_link_tension, 0.0);

        // 2. Кольцо из 3 боссов, где ребро (0, 1) изначально чуть натянуто (1040 > 1000),
        // а ребро (1, 2) (1500) стягивает (1) к (2) и (0), ослабляя звено (0, 1) (< 1000).
        // Унилатеральный XPBD с (new_lambda - old_lambda) обязан полностью разгрузить
        // ослабленное ребро (0, 1), вернув B0 ровно в (0, 0), и сойтись на следующих тиках.
        let mut ring = World::new();
        ring.is_headless = true;
        ring.boss_pos = Vec2::new(0.0, 0.0);
        ring.add_coop_boss(2, Vec2::new(1040.0, 0.0), "B1".to_string(), 0);
        ring.add_coop_boss(3, Vec2::new(-460.0, 0.0), "B2".to_string(), 0);
        update_soul_link(&mut ring, 0.05);

        let p0 = ring.boss_pos;
        let p1 = ring.coop_bosses[0].boss_pos;
        let p2 = ring.coop_bosses[1].boss_pos;
        assert!(p0.is_finite() && p1.is_finite() && p2.is_finite());
        assert!(
            p0.distance(Vec2::ZERO) < 1e-3,
            "Slackened edge (0,1) must release B0 back to origin via (new_lambda - old_lambda), got {:?}",
            p0
        );
        assert!(p0.distance(p1) <= MAX_SOUL_LINK_LENGTH, "Edge (0,1) should be slack: {}", p0.distance(p1));
        assert!(p1.distance(p2) <= 1050.0, "Edge (1,2) single-step compliant bound exceeded: {}", p1.distance(p2));
        assert!(p2.distance(p0) <= MAX_SOUL_LINK_LENGTH, "Edge (2,0) exceeded limit: {}", p2.distance(p0));

        // За несколько тиков мягкая податливость XPBD полностью сжимает и ребро (1, 2) <= MAX_SOUL_LINK_LENGTH + 1
        for _ in 0..4 {
            update_soul_link(&mut ring, 0.05);
        }
        let f0 = ring.boss_pos;
        let f1 = ring.coop_bosses[0].boss_pos;
        let f2 = ring.coop_bosses[1].boss_pos;
        assert!(f1.distance(f2) <= MAX_SOUL_LINK_LENGTH + 1.0, "Edge (1,2) did not converge: {}", f1.distance(f2));
        assert!(f0.distance(f1) > 400.0, "Relaxed edge should not over-collapse: {}", f0.distance(f1));
    }

    #[test]
    fn test_coop_crit_and_lifesteal_belong_to_projectile_owner() {
        use crate::ecs::components::inventory::{PassiveSlot, PassiveType, WeaponSlot, WeaponType};

        crate::core::rand::seed(42);
        let mut world = World::new();
        world.is_headless = true;

        // Boss 1 далеко, БЕЗ оружия, но с максимальным критом и OccultPrism
        world.boss_pos = Vec2::new(1000.0, 1000.0);
        world.boss_health.current = 500.0;
        world.boss_inventory.weapons.clear();
        world.boss_inventory.passives.push(PassiveSlot { kind: PassiveType::FortuneCoin, level: 5 });
        world.boss_inventory.passives.push(PassiveSlot { kind: PassiveType::GoldenSeal, level: 1 });
        world.boss_inventory.passives.push(PassiveSlot { kind: PassiveType::FateWeaver, level: 1 });
        world.boss_inventory.passives.push(PassiveSlot { kind: PassiveType::OccultPrism, level: 1 });

        // Boss 2 (id = 2) стреляет BloodHarvest (100 физ. урона, 20% вампиризма), БЕЗ крита и БЕЗ OccultPrism
        world.add_coop_boss(2, Vec2::new(5000.0, 5000.0), "Boss2".to_string(), 0);
        world.coop_bosses[0].boss_health.current = 500.0;
        world.coop_bosses[0].boss_inventory.weapons = vec![WeaponSlot {
            kind: WeaponType::BloodHarvest,
            level: 1,
            cooldown_timer: 0.0,
            base_cooldown: 10.0,
            damage: 100.0,
            is_evolved: true,
            limit_break_level: 0,
            bonus_projectiles: 0,
        }];

        // Моб на +86px по +X (4 тика * 21px = 84px, dist=2px < 22px; соседние лучи на 60px в стороне; на тике 3 x=63px, dist=23 > 22)
        let mob_id = world.alloc_id();
        let mut mob = MobEntity::new_test(mob_id, Vec2::new(5086.0, 5000.0), 1);
        mob.health = Health::new(1000.0);
        mob.resistances = Default::default();
        mob.speed = 0.0;
        world.mobs.push(mob);
        world.rebuild_spatial_grid();

        for _ in 0..4 {
            update_all_boss_effects(&mut world, 0.05);
            crate::ecs::systems::movement::update_movement(&mut world, 0.05);
            update_all_boss_combat(&mut world, 0.05);
        }

        let mob_dmg_taken = 1000.0 - world.mobs.iter().find(|m| m.id == mob_id).unwrap().health.current;
        assert!(
            (mob_dmg_taken - 100.0).abs() < 1e-3,
            "Boss 2 has 0% crit, so mob must take 100 damage (not Boss 1's crit), got {}",
            mob_dmg_taken
        );
        assert!(
            (world.boss_health.current - 500.0).abs() < 1e-3,
            "Boss 1 did not fire anything and must NOT heal from Boss 2's projectile, got HP {}",
            world.boss_health.current
        );
        assert!(
            (world.coop_bosses[0].boss_health.current - 520.0).abs() < 1e-3,
            "Boss 2 must receive 20% BloodHarvest lifesteal (+20 HP), got HP {}",
            world.coop_bosses[0].boss_health.current
        );
    }

    #[test]
    fn test_coop_simultaneous_hits_split_crit_and_lifesteal_correctly() {
        use crate::ecs::components::inventory::{PassiveSlot, PassiveType, WeaponSlot, WeaponType};

        crate::core::rand::seed(12345);
        let mut world = World::new();
        world.is_headless = true;

        // Boss 1: ArcaneScepter (32 True dmg), 0% крита, OccultPrism (6% lifesteal -> 1.92 HP)
        world.boss_pos = Vec2::new(1000.0, 1000.0);
        world.boss_health.current = 500.0;
        world.boss_inventory.weapons = vec![WeaponSlot {
            kind: WeaponType::ArcaneScepter,
            level: 1,
            cooldown_timer: 0.0,
            base_cooldown: 10.0,
            damage: 32.0,
            is_evolved: false,
            limit_break_level: 0,
            bonus_projectiles: 0,
        }];
        world.boss_inventory.passives.push(PassiveSlot { kind: PassiveType::OccultPrism, level: 1 });

        // Boss 2: BloodHarvest (100 Phys dmg, 20% lifesteal), max crit + FateWeaver, NO OccultPrism
        world.add_coop_boss(2, Vec2::new(5000.0, 5000.0), "Boss2".to_string(), 0);
        world.coop_bosses[0].boss_health.current = 500.0;
        world.coop_bosses[0].boss_inventory.weapons = vec![WeaponSlot {
            kind: WeaponType::BloodHarvest,
            level: 1,
            cooldown_timer: 0.0,
            base_cooldown: 10.0,
            damage: 100.0,
            is_evolved: true,
            limit_break_level: 0,
            bonus_projectiles: 0,
        }];
        world.coop_bosses[0].boss_inventory.passives.push(PassiveSlot { kind: PassiveType::FortuneCoin, level: 5 });
        world.coop_bosses[0].boss_inventory.passives.push(PassiveSlot { kind: PassiveType::GoldenSeal, level: 1 });
        world.coop_bosses[0].boss_inventory.passives.push(PassiveSlot { kind: PassiveType::FateWeaver, level: 1 });

        let id_a = world.alloc_id();
        let mut mob_a = MobEntity::new_test(id_a, Vec2::new(1086.0, 1000.0), 1);
        mob_a.health = Health::new(1000.0);
        mob_a.resistances = Default::default();
        mob_a.speed = 0.0;
        world.mobs.push(mob_a);

        let id_b = world.alloc_id();
        let mut mob_b = MobEntity::new_test(id_b, Vec2::new(5086.0, 5000.0), 1);
        mob_b.health = Health::new(1000.0);
        mob_b.resistances = Default::default();
        mob_b.speed = 0.0;
        world.mobs.push(mob_b);
        world.rebuild_spatial_grid();

        for _ in 0..4 {
            update_all_boss_effects(&mut world, 0.05);
            crate::ecs::systems::movement::update_movement(&mut world, 0.05);
            update_all_boss_combat(&mut world, 0.05);
        }

        let dmg_a = 1000.0 - world.mobs.iter().find(|m| m.id == id_a).unwrap().health.current;
        let dmg_b = 1000.0 - world.mobs.iter().find(|m| m.id == id_b).unwrap().health.current;
        assert!((dmg_a - 32.0).abs() < 1e-3, "Boss 1 has 0% crit, expected 32 dmg, got {}", dmg_a);
        assert!(
            (dmg_b - 240.0).abs() < 1e-3,
            "Boss 2 has GoldenSeal (+20% dmg) + max crit -> expected 240 dmg, got {}",
            dmg_b
        );
        assert!(
            (world.boss_health.current - (500.0 + 32.0 * 0.06)).abs() < 1e-2,
            "Boss 1 must heal only 6% of its own 32 dmg (501.92), got {}",
            world.boss_health.current
        );
        assert!(
            (world.coop_bosses[0].boss_health.current - (500.0 + dmg_b * 0.20)).abs() < 1e-2,
            "Boss 2 must heal 20% of its own crit dmg, got {}",
            world.coop_bosses[0].boss_health.current
        );
    }

    #[test]
    fn test_coop_structure_contribution_and_kill_reward_ownership() {
        use crate::ecs::components::inventory::{WeaponSlot, WeaponType};

        let mut world = World::new();
        world.is_headless = true;

        // Boss 1 далеко без оружия и без SoulFeast
        world.boss_pos = Vec2::new(1000.0, 1000.0);
        world.boss_health.current = 500.0;
        world.boss_inventory.weapons.clear();

        // Boss 2 рядом с Алтарём и слабым мобом: наносит урон FireAura (очередь урона) и SoulFeast
        world.add_coop_boss(2, Vec2::new(5000.0, 5000.0), "Boss2".to_string(), 0);
        world.coop_bosses[0].boss_health.current = 500.0;
        world.coop_bosses[0].boss_inventory.weapons = vec![
            WeaponSlot {
                kind: WeaponType::FireAura,
                level: 1,
                cooldown_timer: 0.0,
                base_cooldown: 1.0,
                damage: 50.0,
                is_evolved: false,
                limit_break_level: 0,
                bonus_projectiles: 0,
            },
            WeaponSlot {
                kind: WeaponType::SoulFeast,
                level: 1,
                cooldown_timer: 99.0,
                base_cooldown: 1.0,
                damage: 100.0,
                is_evolved: true,
                limit_break_level: 0,
                bonus_projectiles: 0,
            },
        ];

        crate::content::structures::spawn_blood_altar(&mut world, Vec2::new(5080.0, 5000.0));
        let altar_id = world.structures[0].id;

        let mob_id = world.alloc_id();
        let mut mob = MobEntity::new_test(mob_id, Vec2::new(5050.0, 5000.0), 1);
        mob.health = Health::new(10.0);
        mob.resistances = Default::default();
        world.mobs.push(mob);
        world.rebuild_spatial_grid();

        update_all_boss_effects(&mut world, 0.05);
        update_all_boss_combat(&mut world, 0.05);

        let contributors = world
            .structure_contributors
            .get(&altar_id)
            .expect("Altar contribution must be recorded");
        assert!(contributors.contains(&2), "Co-op Boss 2 must be recorded as contributor");
        assert!(!contributors.contains(&1), "Idle Boss 1 must NOT steal Boss 2's structure contribution");
        assert!(
            (world.boss_health.current - 500.0).abs() < 1e-3,
            "Idle Boss 1 must NOT gain Boss 2's SoulFeast kill heal, got {}",
            world.boss_health.current
        );
        assert!(
            (world.coop_bosses[0].boss_health.current - 503.5).abs() < 1e-3,
            "Boss 2 must receive its own SoulFeast +3.5 HP on kill, got {}",
            world.coop_bosses[0].boss_health.current
        );
    }

    #[test]
    fn test_20hz_fast_projectile_no_tunneling_and_status_burn_on_death() {
        use crate::ecs::components::combat::DamageType;
        use crate::ecs::components::inventory::{WeaponSlot, WeaponType};
        use crate::ecs::world::ProjectileEntity;

        let mut world = World::new();
        world.is_headless = true;
        world.boss_pos = Vec2::new(1000.0, 1000.0);
        world.boss_inventory.weapons = vec![WeaponSlot {
            kind: WeaponType::NetherBlaze,
            level: 1,
            cooldown_timer: 99.0,
            base_cooldown: 1.0,
            damage: 50.0,
            is_evolved: true,
            limit_break_level: 0,
            bonus_projectiles: 0,
        }];

        // 1. Быстрый снаряд (1000 px/s -> 50px за 1 тик 0.05с) с последним тиком жизни lifetime=0.05
        // пролетает через моба на полпути в (1025, 1000) при radius=8 + 12 = 20 (< 25 до конца и старта!)
        let proj_id = world.alloc_id();
        world.projectiles.push(ProjectileEntity {
            id: proj_id,
            pos: Vec2::new(1000.0, 1000.0),
            prev_pos: Vec2::new(1000.0, 1000.0),
            vel: Vec2::new(1000.0, 0.0),
            damage: 80.0,
            damage_type: DamageType::True,
            armor_penetration: 1.0,
            lifetime: 0.05,
            radius: 8.0,
            pierce: 1,
            faction: crate::ecs::components::combat::Faction::Boss,
            owner_boss_id: Some(1),
            source_weapon: Some(WeaponType::ArcaneScepter),
            homing_strength: 0.0,
        });

        let mob_tunnel_id = world.alloc_id();
        let mut mob_tunnel = MobEntity::new_test(mob_tunnel_id, Vec2::new(1025.0, 1000.0), 1);
        mob_tunnel.health = Health::new(200.0);
        mob_tunnel.speed = 0.0;
        world.mobs.push(mob_tunnel);

        // 2. Моб умирает от горения в тот же тик (dt=0.05), когда burn_timer обнуляется с 0.05 до 0.0.
        // NetherBlaze все равно обязан взорвать труп и задеть соседа в пределах 55px.
        let burning_id = world.alloc_id();
        let mut burning_mob = MobEntity::new_test(burning_id, Vec2::new(3000.0, 3000.0), 1);
        burning_mob.health = Health::new(5.0);
        burning_mob.status.apply_burn(0.05, 200.0); // 200 * 0.05 = 10 dmg > 5 HP
        burning_mob.last_hit_boss_id = Some(1);
        burning_mob.speed = 0.0;
        world.mobs.push(burning_mob);

        let bystander_id = world.alloc_id();
        let mut bystander = MobEntity::new_test(bystander_id, Vec2::new(3030.0, 3000.0), 1);
        bystander.health = Health::new(100.0);
        bystander.speed = 0.0;
        world.mobs.push(bystander);

        world.rebuild_spatial_grid();
        crate::ecs::systems::movement::update_movement(&mut world, 0.05);
        update_all_boss_combat(&mut world, 0.05);

        let tunneled = world.mobs.iter().find(|m| m.id == mob_tunnel_id).expect("Mob A alive");
        assert!(
            (tunneled.health.current - 120.0).abs() < 1e-3,
            "Fast projectile at 20Hz must sweep segment and hit mob instead of tunneling/expiring: HP={}",
            tunneled.health.current
        );

        let neighbor = world.mobs.iter().find(|m| m.id == bystander_id).expect("Bystander alive");
        assert!(
            neighbor.health.current < 100.0,
            "NetherBlaze corpse explosion must still trigger when burn kills mob on its final 20Hz tick: HP={}",
            neighbor.health.current
        );
    }

    #[test]
    fn test_20hz_freeze_duration_dash_clear_and_coop_timers_no_double_tick() {
        use crate::ecs::components::combat::{DamageType, Faction};
        use crate::ecs::world::ProjectileEntity;

        let mut world = World::new();
        world.is_headless = true;
        world.boss_pos = Vec2::new(500.0, 500.0);
        world.boss_facing = Vec2::X;

        // 1. Freeze (0.15s = ровно 3 тика по 0.05s) должен замораживать моба на 3 тика и отпускать на 4-й
        let mob_id = world.alloc_id();
        let mut mob = MobEntity::new_test(mob_id, Vec2::new(2000.0, 2000.0), 1);
        mob.speed = 100.0;
        mob.status.apply_freeze(0.15);
        let initial_mob_pos = mob.pos;
        world.mobs.push(mob);

        // 2. Co-op AI Boss 2: единичный шаг update_boss_ai (runtime путь без повторного tick_coop_boss_timers)
        world.add_coop_boss(2, Vec2::new(3000.0, 3000.0), "Bot2".to_string(), 0);
        world.coop_bosses[0].boss_dash_cooldown = 1.0;
        world.coop_bosses[0].boss_invulnerable_timer = 0.25;

        // 3. Рывок Primary Boss очищает вражеский снаряд, чтобы тот не нанес swept-урон на том же тике
        let proj_id = world.alloc_id();
        world.projectiles.push(ProjectileEntity {
            id: proj_id,
            pos: Vec2::new(680.0, 500.0),
            prev_pos: Vec2::new(660.0, 500.0),
            vel: Vec2::new(400.0, 0.0),
            damage: 50.0,
            damage_type: DamageType::Physical,
            armor_penetration: 0.0,
            lifetime: 1.0,
            radius: 8.0,
            pierce: 1,
            faction: Faction::Director,
            owner_boss_id: None,
            source_weapon: None,
            homing_strength: 0.0,
        });
        crate::ecs::systems::boss_ai::resolve_boss_dash(&mut world, Vec2::X);
        assert!(world.projectiles[0].lifetime <= 0.0 && world.projectiles[0].pierce == 0);
        world.boss_invulnerable_timer = 0.0; // снимаем i-frames, чтобы проверить очистку именно самого снаряда
        let hp_before = world.boss_health.current;

        for tick in 0..4 {
            if tick == 0 && world.swap_coop_boss_context(0) {
                crate::ecs::systems::boss_ai::update_boss_ai(&mut world, 0.05);
                world.swap_coop_boss_context(0);
            }
            crate::ecs::systems::movement::update_movement(&mut world, 0.05);
            update_all_boss_combat(&mut world, 0.05);

            let mob_now = world.mobs.iter().find(|m| m.id == mob_id).unwrap();
            if tick < 3 {
                assert_eq!(
                    mob_now.pos, initial_mob_pos,
                    "Freeze 0.15s at 20Hz must keep mob stationary through tick {}",
                    tick + 1
                );
            } else {
                assert_ne!(
                    mob_now.pos, initial_mob_pos,
                    "Mob must resume moving on tick 4 after 0.15s freeze expires"
                );
            }
        }

        assert!(
            (world.boss_health.current - hp_before).abs() < 1e-4,
            "Director projectile cleared by dash must not hit on that tick"
        );
        assert!(
            (world.coop_bosses[0].boss_dash_cooldown - 0.95).abs() < 1e-4,
            "AI co-op dash cooldown must tick once by 0.05s (got {})",
            world.coop_bosses[0].boss_dash_cooldown
        );
        assert!(
            (world.coop_bosses[0].boss_invulnerable_timer - 0.20).abs() < 1e-4,
            "AI co-op invulnerability must tick once by 0.05s (got {})",
            world.coop_bosses[0].boss_invulnerable_timer
        );
    }
}
