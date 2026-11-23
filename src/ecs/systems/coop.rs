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
        return;
    }

    // Формируем кольцевую топологию звеньев
    let mut links: Vec<(usize, usize, Vec2, Vec2)> = Vec::new();
    if n == 2 {
        links.push((alive_bosses[0].0, alive_bosses[1].0, alive_bosses[0].1, alive_bosses[1].1));
    } else {
        for i in 0..n {
            let next = (i + 1) % n;
            links.push((alive_bosses[i].0, alive_bosses[next].0, alive_bosses[i].1, alive_bosses[next].1));
        }
    }

    // 1. Физика натяжения цепи (максимум 1000 px)
    for &(id_a, id_b, pos_a, pos_b) in &links {
        let diff = pos_b - pos_a;
        let dist = diff.length();
        if dist > MAX_SOUL_LINK_LENGTH {
            let excess = dist - MAX_SOUL_LINK_LENGTH;
            let dir = diff / dist.max(0.001);
            let pull = dir * (excess * 4.5 * dt);

            // Применяем притягивание к боссу A
            if id_a == 0 {
                world.boss_pos += pull;
            } else {
                world.coop_bosses[id_a - 1].boss_pos += pull;
            }

            // Применяем притягивание к боссу B в противоположную сторону
            if id_b == 0 {
                world.boss_pos -= pull;
            } else {
                world.coop_bosses[id_b - 1].boss_pos -= pull;
            }
        }
    }

    // 2. Рассекающий урон мобам, пересекающим цепь
    let damage_slice = 260.0 * dt;
    for &(_, _, pos_a, pos_b) in &links {
        let min_x = pos_a.x.min(pos_b.x) - 40.0;
        let max_x = pos_a.x.max(pos_b.x) + 40.0;
        let min_y = pos_a.y.min(pos_b.y) - 40.0;
        let max_y = pos_a.y.max(pos_b.y) + 40.0;

        for mob in world.mobs.iter_mut() {
            if !mob.health.is_alive() { continue; }
            if mob.pos.x < min_x || mob.pos.x > max_x || mob.pos.y < min_y || mob.pos.y > max_y {
                continue;
            }

            let (dist, hit_pt) = dist_to_segment(mob.pos, pos_a, pos_b);
            if dist <= mob.radius + 14.0 {
                mob.health.take_damage(damage_slice);

                // Эффект рассечения / искры
                if !world.is_headless && crate::core::rand::gen_range(0.0, 1.0) < 0.25 {
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
    update_soul_link(world, dt);
    crate::ecs::systems::skills::update_black_holes(world, dt);
    crate::ecs::systems::skills::update_heroes_and_clones(world, dt);
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
            PerkChoice::UpgradePassive(PassiveType::BerserkerBlood),
            PerkChoice::UpgradePassive(PassiveType::MartyrVanguard),
        ]);

        // Host player (0) chooses index 1 (BerserkerBlood)
        apply_player_perk_choice_by_index(&mut world, 0, 1);

        assert_eq!(world.boss_pending_level_ups, 0);
        assert!(world.boss_level_up_options.is_none());
        assert!(world.boss_inventory.has_passive(PassiveType::BerserkerBlood));

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
            kind: PassiveType::ImmortalAegis,
            level: 1,
        });
        let mut found_upgrade = false;
        for _ in 0..150 {
            let options = crate::ecs::systems::boss_ai::generate_perk_choices(&inv_with_relic);
            if options.iter().any(|o| matches!(o, PerkChoice::UpgradePassive(PassiveType::ImmortalAegis))) {
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
}
