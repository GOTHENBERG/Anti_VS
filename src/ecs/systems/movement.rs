use macroquad::prelude::*;
use rayon::prelude::*;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction};
use crate::ecs::components::inventory::{DamageSourceKind, PassiveType, WeaponType};
use crate::ecs::world::World;

#[inline(always)]
fn tick_status_timer(timer: &mut f32, dt: f32) {
    *timer -= dt;
    if *timer <= 1e-4 {
        *timer = 0.0;
    }
}

#[inline(always)]
fn update_single_mob(
    mob_idx: usize,
    mob: &mut crate::ecs::world::MobEntity,
    primary_boss_pos: Vec2,
    _primary_boss_vel: Vec2,
    primary_boss_alive: bool,
    coop_bosses: &[crate::ecs::world::BossPlayerState],
    _flow_field: &crate::ai::flow_field::FlowField,
    crowd_grid: &crate::core::crowd::CrowdDensityGrid,
    prev_positions: &[Vec2],
    spatial_grid: &crate::core::spatial_grid::SpatialGrid,
    dt: f32,
) {
    // Обновление таймеров статусов: вычисляем замедление/заморозку на текущий тик ДО декремента
    // и обнуляем суб-эпсилонный float-остаток, чтобы длительность N * 0.05с работала ровно N тиков.
    mob.status.burned_this_tick = mob.status.burn_timer > 0.0;
    if mob.status.burn_timer > 0.0 {
        tick_status_timer(&mut mob.status.burn_timer, dt);
        mob.health.take_damage(mob.status.burn_dps * dt);
    }
    if mob.status.poison_timer > 0.0 {
        tick_status_timer(&mut mob.status.poison_timer, dt);
        mob.health.take_damage(mob.status.poison_dps * dt);
    }
    let speed_mult = mob.status.get_speed_multiplier();
    if mob.status.slow_timer > 0.0 {
        tick_status_timer(&mut mob.status.slow_timer, dt);
    }
    if mob.status.freeze_timer > 0.0 {
        tick_status_timer(&mut mob.status.freeze_timer, dt);
    }

    // Кулдаун атаки уменьшается непрерывно, в том числе во время движения к боссу
    if mob.attack_timer > 0.0 {
        mob.attack_timer = (mob.attack_timer - dt).max(0.0);
    }

    // Мобы преследуют ближайшего живого Босса: при нескольких игроках каждый моб
    // сам выбирает цель по текущей дистанции, не закрепляясь навсегда за одним игроком.
    let assigned_target = mob.target_boss_id.and_then(|id| {
        if id == 1 && primary_boss_alive {
            Some(primary_boss_pos)
        } else {
            coop_bosses.iter()
                .find(|player| player.id == id && player.boss_health.is_alive())
                .map(|player| player.boss_pos)
        }
    });
    let boss_pos = if let Some(pos) = assigned_target {
        pos
    } else {
        let mut nearest = if primary_boss_alive {
            Some((primary_boss_pos, primary_boss_pos.distance_squared(mob.pos)))
        } else { None };
        for player in coop_bosses.iter().filter(|player| player.boss_health.is_alive()) {
            let distance = player.boss_pos.distance_squared(mob.pos);
            if nearest.map_or(true, |(_, best)| distance < best) {
                nearest = Some((player.boss_pos, distance));
            }
        }
        nearest.map_or(primary_boss_pos, |(pos, _)| pos)
    };

    // Прямой кратчайший путь к боссу: мгновенная реакция на любое перемещение босса
    let target_pos = boss_pos;

    // Движение к цели с учетом замедления и заморозки
    let diff = target_pos - mob.pos;
    let dist = diff.length();

    if dist > 2.0 && speed_mult > 0.0 {
        let goal_dir = diff / dist;
        let speed = mob.speed * speed_mult;

        // Continuum Crowds: комбинируем вектор цели с градиентом давления толпы
        mob.vel = crowd_grid.compute_steering(mob.pos, goal_dir, speed);
        mob.pos += mob.vel * dt;

        // Микро-отталкивание для мобов в той же ячейке (проверяет не более 1-2 соседей)
        crate::core::crowd::CrowdDensityGrid::micro_separate_same_cell(
            mob_idx,
            &mut mob.pos,
            prev_positions,
            spatial_grid,
            mob.radius * 1.8,
        );
    } else {
        mob.vel = Vec2::ZERO;
    }

    // Применение импульса отбрасывания (knockback), кроме иммунных големов
    if !mob.knockback_immune && mob.knockback_vel.length_squared() > 1.0 {
        mob.pos += mob.knockback_vel * dt;
        mob.knockback_vel *= 0.88; // Плавное затухание импульса
    }

    // Ограничение координат мобов границами карты (защита от вылета за карту при отталкивании)
    let margin = 20.0;
    mob.pos.x = mob.pos.x.clamp(margin, crate::core::MAP_WIDTH - margin);
    mob.pos.y = mob.pos.y.clamp(margin, crate::core::MAP_HEIGHT - margin);
}

pub fn update_movement(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let boss_vel = world.boss_vel;
    let primary_boss_alive = world.boss_health.is_alive();

    // 1. Построение плоского массива позиций (SoA) и сетки плотностей Continuum Crowds
    world.mob_positions_scratch.clear();
    for mob in &world.mobs {
        world.mob_positions_scratch.push(mob.pos);
    }
    world.crowd_grid.clear();
    world.crowd_grid.splat_positions(&world.mob_positions_scratch);
    world.crowd_grid.compute_blurred_gradients();

    // 2. Построение SpatialGrid по текущим координатам для мгновенного микро-отталкивания
    world.rebuild_spatial_grid();

    // 3. Обновление движения, физики и эффектов мобов:
    // В Headless (обучение): последовательный цикл внутри своего потока Rayon (0 оверхеда).
    // В игре с графикой: параллельное обновление на всех потоках для максимального FPS.
    let is_headless = world.is_headless;
    let flow_field = &world.flow_field;
    let crowd_grid = &world.crowd_grid;
    let prev_positions = &world.mob_positions_scratch;
    let spatial_grid = &world.spatial_grid;
    let coop_bosses = &world.coop_bosses;

    if is_headless {
        world.mobs.iter_mut().enumerate().for_each(|(idx, mob)| {
            update_single_mob(
                idx,
                mob,
                boss_pos,
                boss_vel,
                primary_boss_alive,
                coop_bosses,
                flow_field,
                crowd_grid,
                prev_positions,
                spatial_grid,
                dt,
            );
        });
    } else {
        world.mobs.par_iter_mut().enumerate().for_each(|(idx, mob)| {
            update_single_mob(
                idx,
                mob,
                boss_pos,
                boss_vel,
                primary_boss_alive,
                coop_bosses,
                flow_field,
                crowd_grid,
                prev_positions,
                spatial_grid,
                dt,
            );
        });
    }

    // 4. Перестроение Пространственной Сетки (Spatial Grid) для мгновенных запросов снарядов O(1)
    world.rebuild_spatial_grid();

    let live_coop_bosses: Vec<(u8, Vec2, Vec2)> = world.coop_bosses
        .iter()
        .filter(|player| player.boss_health.is_alive())
        .map(|player| (player.id, player.boss_pos, player.boss_vel))
        .collect();

    // 5. Обновление снарядов (включая активное самонаведение в полете)
    let primary_id = world.active_boss_id;
    let primary_has_divinity = world.boss_inventory.has_passive(PassiveType::DivinityAura);
    let boss_owner_states: Vec<(u8, Vec2, bool)> = std::iter::once((
        primary_id,
        boss_pos,
        primary_has_divinity,
    ))
    .chain(world.coop_bosses.iter().map(|p| {
        (p.id, p.boss_pos, p.boss_inventory.has_passive(PassiveType::DivinityAura))
    }))
    .collect();

    let mut homing_buf: Vec<usize> = Vec::with_capacity(64);
    let mut divinity_buf: Vec<usize> = Vec::with_capacity(32);
    for proj in &mut world.projectiles {
        if proj.lifetime <= 0.0 || proj.pierce == 0 {
            continue;
        }
        if proj.homing_strength > 0.0 {
            let search_radius = 450.0f32;
            let mut closest_target_pos = None;
            let mut closest_target_vel = Vec2::ZERO;
            let mut min_dist_sq = search_radius * search_radius;

            if proj.faction == Faction::Boss {
                // Ищем ближайшего живого моба через Spatial Grid без аллокаций
                world.spatial_grid.query_nearby_into(proj.pos, search_radius, &mut homing_buf);
                for &idx in &homing_buf {
                    if let Some(mob) = world.mobs.get(idx) {
                        if mob.health.is_alive() {
                            let dist_sq = mob.pos.distance_squared(proj.pos);
                            if dist_sq < min_dist_sq {
                                min_dist_sq = dist_sq;
                                closest_target_pos = Some(mob.pos);
                                closest_target_vel = mob.vel;
                            }
                        }
                    }
                }
                // Если мобов в радиусе нет — проверяем живые алтари
                if closest_target_pos.is_none() {
                    for st in &world.structures {
                        if st.health.is_alive() {
                            let dist_sq = st.pos.distance_squared(proj.pos);
                            if dist_sq < min_dist_sq {
                                min_dist_sq = dist_sq;
                                closest_target_pos = Some(st.pos);
                                closest_target_vel = Vec2::ZERO;
                            }
                        }
                    }
                }
            } else if proj.faction == Faction::Director {
                if primary_boss_alive {
                    closest_target_pos = Some(boss_pos);
                    closest_target_vel = boss_vel;
                    min_dist_sq = proj.pos.distance_squared(boss_pos);
                }
                for &(_, player_pos, player_vel) in &live_coop_bosses {
                    let dist_sq = proj.pos.distance_squared(player_pos);
                    if dist_sq < min_dist_sq {
                        min_dist_sq = dist_sq;
                        closest_target_pos = Some(player_pos);
                        closest_target_vel = player_vel;
                    }
                }
            }

            if let Some(target_pos) = closest_target_pos {
                proj.vel = crate::core::math::calculate_proportional_navigation_vel(
                    proj.pos,
                    proj.vel,
                    target_pos,
                    closest_target_vel,
                    proj.homing_strength,
                    dt,
                );
            }
        }

        // Механика бумеранга (FaithBoomerang и HeavenlyJudgment):
        // После первой фазы полета плавно возвращается к выпустившему его боссу
        let owner_id = proj.owner_boss_id.unwrap_or(primary_id);
        let owner_state = boss_owner_states.iter().find(|&&(bid, _, _)| bid == owner_id);
        if let Some(src) = proj.source_weapon {
            let remaining_after_step = proj.lifetime - dt;
            if (src == WeaponType::FaithBoomerang && remaining_after_step < 0.65)
                || (src == WeaponType::HeavenlyJudgment && remaining_after_step < 0.70)
            {
                let return_pos = owner_state.map_or(boss_pos, |&(_, pos, _)| pos);
                let to_boss = (return_pos - proj.pos).normalize_or_zero();
                let current_spd = proj.vel.length().max(280.0);
                proj.vel = proj.vel.lerp(to_boss * current_spd, (5.0 * dt).min(1.0));
            }

            // Механика рикошета от стен арены (RunicPrism и SpatialShatter)
            if src == WeaponType::RunicPrism || src == WeaponType::SpatialShatter {
                let map_w = crate::core::MAP_WIDTH;
                let map_h = crate::core::MAP_HEIGHT;
                if (proj.pos.x <= 25.0 && proj.vel.x < 0.0) || (proj.pos.x >= map_w - 25.0 && proj.vel.x > 0.0) {
                    proj.vel.x = -proj.vel.x;
                }
                if (proj.pos.y <= 25.0 && proj.vel.y < 0.0) || (proj.pos.y >= map_h - 25.0 && proj.vel.y > 0.0) {
                    proj.vel.y = -proj.vel.y;
                }
            }
        }

        // Аура Божественности (DivinityAura): святой урон вокруг летящих снарядов владельца
        let owner_has_divinity = proj
            .owner_boss_id
            .map_or(primary_has_divinity, |_| owner_state.is_some_and(|&(_, _, has)| has));
        if owner_has_divinity && proj.faction == Faction::Boss {
            let aura_rad = 52.0;
            world.spatial_grid.query_nearby_into(proj.pos, aura_rad, &mut divinity_buf);
            for &idx in &divinity_buf {
                if let Some(mob) = world.mobs.get_mut(idx) {
                    if mob.health.is_alive() && mob.pos.distance_squared(proj.pos) < aura_rad * aura_rad {
                        mob.health.take_damage(35.0 * dt);
                        if proj.owner_boss_id.is_some() {
                            mob.last_hit_boss_id = proj.owner_boss_id;
                        }
                    }
                }
            }
        }

        proj.prev_pos = proj.pos;
        proj.pos += proj.vel * dt;
    }

    // Обновление таймеров электрических дуг молний
    world.lightning_arcs.retain_mut(|(_, _, timer)| {
        *timer -= dt;
        *timer > 0.0
    });

    // 4. Обновление кислотных луж Босса (Чумные Сферы и Токсичный Мор)
    for pool in &mut world.acid_pools {
        pool.lifetime -= dt;
        if pool.is_director_source {
            continue;
        }

        // Лужи Босса наносят урон МОБАМ (Босса они не трогают)
        world.spatial_grid.query_nearby_into(pool.pos, pool.radius, &mut homing_buf);
        for &idx in &homing_buf {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(pool.pos) < pool.radius * pool.radius {
                    mob.status.apply_poison(2.5, pool.dps * 0.5);
                    if pool.corrosive {
                        // Разъедает плоскую броню
                        mob.resistances.flat_armor = (mob.resistances.flat_armor - dt * 2.0).max(0.0);
                    }
                    world.damage_queue.push(DamagePacket::mob(
                        Some(0),
                        idx,
                        mob.id,
                        pool.dps * dt,
                        DamageType::Poison,
                        0.5,
                    ).with_owner(pool.owner_boss_id));
                }
            }
        }
    }
    world.acid_pools.retain(|p| p.lifetime > 0.0);

    // 4.1 Чумные Споры Директора: периодический урон отравления (Poison DoT по Боссу)
    if world.boss_poison_timer > 0.0 {
        world.boss_poison_timer = (world.boss_poison_timer - dt).max(0.0);
        let poison_res = world.boss_resistances.poison_res.clamp(-0.5, 0.85);
        let effective_dps = world.boss_poison_dps * (1.0 - poison_res);
        let poison_dmg = effective_dps * dt;
        world.boss_health.take_damage(poison_dmg);
        world.boss_inventory.record_damage_taken(poison_dmg, DamageType::Poison, DamageSourceKind::Hazard);
        world.total_damage_dealt_to_boss += poison_dmg;
        if poison_dmg > 0.0 {
            world.boss_damage_taken_timer = 3.0;
            world.boss_still_time = 0.0;
        }

        if world.boss_poison_timer <= 0.0 {
            world.boss_poison_dps = 0.0;
        }

        if !world.is_headless && crate::core::visual_rand::gen_range(0.0, 1.0) < 0.25 {
            world.particles.push(crate::ecs::world::ParticleEntity {
                pos: world.boss_pos + Vec2::new(crate::core::visual_rand::gen_range(-14.0, 14.0), crate::core::visual_rand::gen_range(-14.0, 14.0)),
                vel: Vec2::new(crate::core::visual_rand::gen_range(-10.0, 10.0), crate::core::visual_rand::gen_range(-25.0, -8.0)),
                color: Color::new(0.65, 0.15, 0.85, 0.75),
                radius: 2.2,
                lifetime: 0.35,
                max_lifetime: 0.35,
            });
        }
    } else {
        world.boss_poison_dps = 0.0;
    }

    // 4.2 Чумные Споры Директора: защитные миазмы вокруг Алтарей Крови
    let pestilence_lvl = world.directors.first().map_or(0, |d| d.grade_pestilence);
    if pestilence_lvl > 0 {
        let miasma_radius = 160.0f32;
        let miasma_dps = 10.0 + (pestilence_lvl as f32) * 2.5;
        let poison_res = world.boss_resistances.poison_res.clamp(-0.5, 0.85);
        for st in &world.structures {
            if crate::content::structures::is_altar_structure_type(st.structure_type) && world.boss_pos.distance_squared(st.pos) < miasma_radius * miasma_radius {
                let dmg = miasma_dps * (1.0 - poison_res) * dt;
                world.boss_health.take_damage(dmg);
                world.boss_inventory.record_damage_taken(dmg, DamageType::Poison, DamageSourceKind::Hazard);
                world.total_damage_dealt_to_boss += dmg;
                if dmg > 0.0 {
                    world.boss_damage_taken_timer = 3.0;
                    world.boss_still_time = 0.0;
                }
                break;
            }
        }
    }

    // 4.3 Физическая коллизия Босса со Стенами-Баррикадами
    for st in &world.structures {
        if st.structure_type == crate::content::structures::STRUCTURE_WALL && st.health.is_alive() {
            let diff = world.boss_pos - st.pos;
            let dist_sq = diff.length_squared();
            let min_dist = 18.0 + st.radius; // 18 + 25 = 43.0 px
            if dist_sq < min_dist * min_dist && dist_sq > 0.001 {
                let dist = dist_sq.sqrt();
                let normal = diff / dist;
                world.boss_pos = st.pos + normal * min_dist;

                // Скольжение вдоль стены: устраняем составляющую скорости, направленную внутрь баррикады
                let normal_vel = world.boss_vel.dot(normal);
                if normal_vel < 0.0 {
                    world.boss_vel -= normal * normal_vel;
                }
            }
        }
    }

    // 5. Магнетизм кристаллов душ ко всем живым боссам (с учетом их MagnetSphere)
    let mut magnet_bosses: Vec<(Vec2, f32, bool)> = Vec::with_capacity(world.coop_bosses.len() + 1);
    if primary_boss_alive {
        let r = world.boss_inventory.get_effective_magnet_radius();
        let r_sq = r * r;
        let picked_magnet = world
            .gems
            .iter()
            .any(|g| g.is_magnet && g.pos.distance_squared(boss_pos) < r_sq);
        magnet_bosses.push((boss_pos, r_sq, picked_magnet));
    }
    for player in &world.coop_bosses {
        if player.boss_health.is_alive() {
            let r = player.boss_inventory.get_effective_magnet_radius();
            let r_sq = r * r;
            let picked_magnet = world
                .gems
                .iter()
                .any(|g| g.is_magnet && g.pos.distance_squared(player.boss_pos) < r_sq);
            magnet_bosses.push((player.boss_pos, r_sq, picked_magnet));
        }
    }

    if !magnet_bosses.is_empty() {
        let any_magnet_active = magnet_bosses.iter().any(|&(_, _, m)| m);
        for gem in &mut world.gems {
            let mut best_target: Option<(Vec2, f32, bool)> = None;
            for &(b_pos, r_sq, b_magnet) in &magnet_bosses {
                if any_magnet_active && !b_magnet {
                    continue;
                }
                let dist_sq = b_pos.distance_squared(gem.pos);
                if (any_magnet_active || dist_sq < r_sq)
                    && dist_sq > 1.0
                    && best_target.map_or(true, |(_, best_sq, _)| dist_sq < best_sq)
                {
                    best_target = Some((b_pos, dist_sq, any_magnet_active));
                }
            }
            if let Some((target_pos, dist_sq, is_global_magnet)) = best_target {
                let dist = dist_sq.sqrt().max(1.0);
                let dir = (target_pos - gem.pos) / dist;
                let pull_speed = if is_global_magnet { 550.0 } else { 280.0 };
                gem.pos += dir * (pull_speed * dt);
            }
        }
    }

    // 6. Обновление всплывающего текста (Floating text)
    for text in &mut world.floating_texts {
        text.pos += text.vel * dt;
        text.lifetime -= dt;
    }
    world.floating_texts.retain(|t| t.lifetime > 0.0);

    // 7. Обновление частиц (Particles)
    for p in &mut world.particles {
        p.pos += p.vel * dt;
        p.vel *= 0.95; // трение
        p.lifetime -= dt;
    }
    world.particles.retain(|p| p.lifetime > 0.0);

    // 8. Тряска экрана
    if world.screen_shake > 0.0 {
        world.screen_shake = (world.screen_shake - dt * 15.0).max(0.0);
    }

    // 9. Анимация кругов приказов перемещения (Move Click Indicators)
    for eff in &mut world.move_click_effects {
        eff.timer -= dt;
    }
    world.move_click_effects.retain(|e| e.timer > 0.0);
}
