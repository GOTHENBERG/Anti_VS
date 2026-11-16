use macroquad::prelude::*;
use rayon::prelude::*;
use crate::ecs::components::combat::{DamagePacket, DamageType};
use crate::ecs::components::inventory::DamageSourceKind;
use crate::ecs::world::World;

#[inline(always)]
fn update_single_mob(mob: &mut crate::ecs::world::MobEntity, boss_pos: Vec2, boss_vel: Vec2, dt: f32) {
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

    // 1. Обновление движения, физики и эффектов мобов:
    // В Headless (обучение): последовательный цикл внутри своего потока Rayon (0 оверхеда).
    // В игре с графикой: параллельное обновление на всех 8 потоках i3-10100F для максимального FPS.
    if world.is_headless {
        world.mobs.iter_mut().for_each(|mob| update_single_mob(mob, boss_pos, boss_vel, dt));
    } else {
        world.mobs.par_iter_mut().for_each(|mob| update_single_mob(mob, boss_pos, boss_vel, dt));
    }

    // 2. Перестроение Пространственной Сетки (Spatial Grid) для мгновенных запросов O(1)
    world.spatial_grid.clear();
    for (idx, mob) in world.mobs.iter().enumerate() {
        world.spatial_grid.insert(idx, mob.pos);
    }

    // 3. Обновление снарядов
    for proj in &mut world.projectiles {
        proj.pos += proj.vel * dt;
        proj.lifetime -= dt;
    }
    world.projectiles.retain(|p| p.lifetime > 0.0);

    // 4. Обновление кислотных луж
    let boss_pos = world.boss_pos;
    let mut overlapping_director_pools = 0;
    let mut max_director_dps = 0.0f32;
    let mut corrosive_active = false;

    for pool in &mut world.acid_pools {
        pool.lifetime -= dt;

        if pool.is_director_source {
            // Лужа Директора (Чумные Споры): отслеживаем нахождение босса в зоне заражения
            if boss_pos.distance_squared(pool.pos) < pool.radius * pool.radius {
                overlapping_director_pools += 1;
                max_director_dps = max_director_dps.max(pool.dps);
                if pool.corrosive {
                    corrosive_active = true;
                }
            }
        } else {
            // Лужа Босса (Чумные Сферы): наносит урон МОБАМ, а самого Босса НЕ трогает!
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
    }

    // Балансировка Чумных Спор: суммирование с затуханием (макс кап 3 стака) и учет резиста к яду
    if overlapping_director_pools > 0 {
        let stack_mult = match overlapping_director_pools {
            1 => 1.0,
            2 => 1.4,
            _ => 1.75, // Ограничение: предотвращает моментальный ваншот босса от сотен наложившихся луж
        };
        let poison_res = world.boss_resistances.poison_res.clamp(-0.5, 0.85);
        let effective_dps = max_director_dps * stack_mult * (1.0 - poison_res);
        let poison_dmg = effective_dps * dt;
        world.boss_health.take_damage(poison_dmg);
        world.boss_inventory.record_damage_taken(poison_dmg, DamageType::Poison, DamageSourceKind::Hazard);
        world.total_damage_dealt_to_boss += poison_dmg;

        if corrosive_active {
            world.boss_resistances.flat_armor = (world.boss_resistances.flat_armor - dt * 1.5).max(0.0);
        }
    }
    world.acid_pools.retain(|p| p.lifetime > 0.0);

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
