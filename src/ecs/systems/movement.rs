use macroquad::prelude::*;
use rayon::prelude::*;
use crate::ecs::components::combat::{DamagePacket, DamageType, Faction};
use crate::ecs::components::inventory::{DamageSourceKind, WeaponType};
use crate::ecs::world::World;

#[inline(always)]
fn update_single_mob(
    mob: &mut crate::ecs::world::MobEntity,
    primary_boss_pos: Vec2,
    primary_boss_vel: Vec2,
    primary_boss_alive: bool,
    coop_bosses: &[crate::ecs::world::BossPlayerState],
    dt: f32,
) {
    // Обновление таймеров статусов
    if mob.status.burn_timer > 0.0 {
        mob.status.burn_timer = (mob.status.burn_timer - dt).max(0.0);
        mob.health.take_damage(mob.status.burn_dps * dt);
    }
    if mob.status.poison_timer > 0.0 {
        mob.status.poison_timer = (mob.status.poison_timer - dt).max(0.0);
        mob.health.take_damage(mob.status.poison_dps * dt);
    }
    if mob.status.slow_timer > 0.0 {
        mob.status.slow_timer = (mob.status.slow_timer - dt).max(0.0);
    }
    if mob.status.freeze_timer > 0.0 {
        mob.status.freeze_timer = (mob.status.freeze_timer - dt).max(0.0);
    }

    // Мобы преследуют ближайшего живого Босса: при нескольких игроках каждый моб
    // сам выбирает цель по текущей дистанции, не закрепляясь навсегда за одним игроком.
    let assigned_target = mob.target_boss_id.and_then(|id| {
        if id == 1 && primary_boss_alive {
            Some((primary_boss_pos, primary_boss_vel))
        } else {
            coop_bosses.iter()
                .find(|player| player.id == id && player.boss_health.is_alive())
                .map(|player| (player.boss_pos, player.boss_vel))
        }
    });
    let (boss_pos, boss_vel) = if let Some(target) = assigned_target {
        target
    } else {
        let mut nearest = if primary_boss_alive {
            Some((primary_boss_pos, primary_boss_vel, primary_boss_pos.distance_squared(mob.pos)))
        } else { None };
        for player in coop_bosses.iter().filter(|player| player.boss_health.is_alive()) {
            let distance = player.boss_pos.distance_squared(mob.pos);
            if nearest.map_or(true, |(_, _, best)| distance < best) {
                nearest = Some((player.boss_pos, player.boss_vel, distance));
            }
        }
        nearest.map_or((primary_boss_pos, primary_boss_vel), |(pos, vel, _)| (pos, vel))
    };

    // Вычисление целевой точки в зависимости от роли моба
    let target_pos = match mob.tier {
        // Тир 2 (Гончие): бегут на упреждение траектории босса (фланкирование)
        2 => boss_pos + boss_vel * 0.45,
        // Тир 3 (Культисты-стрелки): кайтят босса, держа дистанцию 240-280 пикселей
        3 => {
            let diff = boss_pos - mob.pos;
            let dist = diff.length();
            if dist < 230.0 {
                // Отступают назад
                mob.pos - diff.normalize_or_zero() * 100.0
            } else if dist > 280.0 {
                // Приближаются
                boss_pos
            } else {
                // Стоят и стреляют
                mob.pos
            }
        }
        // Тир 6 (Демон теней): если далеко, телепортируется за спину босса!
        6 => {
            let diff = boss_pos - mob.pos;
            let dist = diff.length();
            mob.attack_timer -= dt;
            if dist > 320.0 && mob.attack_timer <= 0.0 {
                mob.attack_timer = 4.5;
                let behind_dir = -boss_vel.normalize_or_zero();
                let offset = if behind_dir.length_squared() > 0.5 {
                    behind_dir * 140.0
                } else {
                    Vec2::new(120.0, 0.0)
                };
                mob.pos = boss_pos + offset;
            }
            boss_pos
        }
        // Тир 7 (Огненный элементаль): кружит вокруг босса на средней дистанции (180px)
        7 => {
            let diff = boss_pos - mob.pos;
            let dist = diff.length();
            if dist < 170.0 {
                mob.pos - diff.normalize_or_zero() * 80.0
            } else if dist > 260.0 {
                boss_pos
            } else {
                // Орбитальное движение
                let perp = Vec2::new(-diff.y, diff.x).normalize_or_zero();
                mob.pos + perp * 60.0
            }
        }
        // Остальные идут напролом
        _ => boss_pos,
    };

    // Движение к цели с учетом замедления и заморозки
    let speed_mult = mob.status.get_speed_multiplier();
    let diff = target_pos - mob.pos;
    let dist = diff.length();

    if dist > 2.0 && speed_mult > 0.0 {
        let dir = diff / dist;
        mob.vel = dir * (mob.speed * speed_mult);
        mob.pos += mob.vel * dt;
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
    let coop_bosses = &world.coop_bosses;
    let live_coop_bosses: Vec<(u8, Vec2)> = coop_bosses
        .iter()
        .filter(|player| player.boss_health.is_alive())
        .map(|player| (player.id, player.boss_pos))
        .collect();

    // 1. Обновление движения, физики и эффектов мобов:
    // В Headless (обучение): последовательный цикл внутри своего потока Rayon (0 оверхеда).
    // В игре с графикой: параллельное обновление на всех 8 потоках i3-10100F для максимального FPS.
    if world.is_headless {
        world.mobs.iter_mut().for_each(|mob| update_single_mob(mob, boss_pos, boss_vel, primary_boss_alive, coop_bosses, dt));
    } else {
        world.mobs.par_iter_mut().for_each(|mob| update_single_mob(mob, boss_pos, boss_vel, primary_boss_alive, coop_bosses, dt));
    }

    // 2. Перестроение Пространственной Сетки (Spatial Grid) для мгновенных запросов O(1)
    world.rebuild_spatial_grid();

    // 3. Обновление снарядов (включая активное самонаведение в полете)
    let mut homing_buf: Vec<usize> = Vec::with_capacity(64);
    for proj in &mut world.projectiles {
        if proj.homing_strength > 0.0 {
            let search_radius = 450.0f32;
            let mut closest_target_pos = None;
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
                            }
                        }
                    }
                }
            } else if proj.faction == Faction::Director {
                if primary_boss_alive {
                    closest_target_pos = Some(boss_pos);
                    min_dist_sq = proj.pos.distance_squared(boss_pos);
                }
                for &(_, player_pos) in &live_coop_bosses {
                    let dist_sq = proj.pos.distance_squared(player_pos);
                    if dist_sq < min_dist_sq {
                        min_dist_sq = dist_sq;
                        closest_target_pos = Some(player_pos);
                    }
                }
            }

            if let Some(target_pos) = closest_target_pos {
                let to_target = target_pos - proj.pos;
                let dist_sq = to_target.length_squared();
                if dist_sq > 4.0 {
                    let target_dir = to_target.normalize();
                    let current_speed = proj.vel.length();
                    if current_speed > 1.0 {
                        let current_dir = proj.vel / current_speed;
                        let turn_factor = (proj.homing_strength * dt).min(1.0);
                        let new_dir = current_dir.lerp(target_dir, turn_factor).normalize_or_zero();
                        proj.vel = new_dir * current_speed;
                    }
                }
            }
        }

        // Механика бумеранга (FaithBoomerang и HeavenlyJudgment):
        // После первой фазы полета плавно возвращается к боссу
        if let Some(src) = proj.source_weapon {
            if (src == WeaponType::FaithBoomerang || src == WeaponType::HeavenlyJudgment) && proj.lifetime < 1.9 {
                let to_boss = (boss_pos - proj.pos).normalize_or_zero();
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

        proj.pos += proj.vel * dt;
        proj.lifetime -= dt;
    }
    world.projectiles.retain(|p| p.lifetime > 0.0);

    // 4. Обновление кислотных луж Босса (Чумные Сферы и Токсичный Мор)
    for pool in &mut world.acid_pools {
        pool.lifetime -= dt;

        // Лужи Босса наносят урон МОБАМ (Босса они не трогают)
        let nearby = world.spatial_grid.query_nearby(pool.pos, pool.radius);
        for idx in nearby {
            if let Some(mob) = world.mobs.get_mut(idx) {
                if mob.pos.distance_squared(pool.pos) < pool.radius * pool.radius {
                    mob.status.apply_poison(2.5, pool.dps * 0.5);
                    if pool.corrosive {
                        // Разъедает плоскую броню
                        mob.resistances.flat_armor = (mob.resistances.flat_armor - dt * 2.0).max(0.0);
                    }
                    world.damage_queue.push(DamagePacket {
                        source_id: Some(0),
                        target_id: mob.id,
                        amount: pool.dps * dt,
                        damage_type: DamageType::Poison,
                        armor_penetration: 0.5,
                    });
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

        if !world.is_headless && macroquad::prelude::rand::gen_range(0.0, 1.0) < 0.25 {
            world.particles.push(crate::ecs::world::ParticleEntity {
                pos: world.boss_pos + Vec2::new(macroquad::prelude::rand::gen_range(-14.0, 14.0), macroquad::prelude::rand::gen_range(-14.0, 14.0)),
                vel: Vec2::new(macroquad::prelude::rand::gen_range(-10.0, 10.0), macroquad::prelude::rand::gen_range(-25.0, -8.0)),
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
            }
        }
    }

    // 5. Магнетизм кристаллов душ к боссу (с учетом пассивки MagnetSphere)
    let effective_magnet_r = world.boss_inventory.get_effective_magnet_radius();
    let magnet_r_sq = effective_magnet_r.powi(2);
    let mut magnet_active = false;

    // Проверяем, подобрал ли босс предмет "Магнит"
    for gem in &world.gems {
        if gem.is_magnet && gem.pos.distance_squared(boss_pos) < magnet_r_sq {
            magnet_active = true;
            break;
        }
    }

    for gem in &mut world.gems {
        let diff = boss_pos - gem.pos;
        let dist_sq = diff.length_squared();

        if magnet_active || (dist_sq < magnet_r_sq && dist_sq > 1.0) {
            let dist = dist_sq.sqrt().max(1.0);
            let dir = diff / dist;
            let pull_speed = if magnet_active { 550.0 } else { 280.0 };
            gem.pos += dir * (pull_speed * dt);
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
}
