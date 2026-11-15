use macroquad::prelude::*;
use rayon::prelude::*;
use crate::ecs::components::combat::{DamagePacket, DamageType};
use crate::ecs::world::World;

pub fn update_movement(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;
    let boss_vel = world.boss_vel;

    // 1. Параллельное обновление движения, физики и эффектов мобов на всех 8 потоках i3-10100F!
    world.mobs.par_iter_mut().for_each(|mob| {
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
    });

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
    for pool in &mut world.acid_pools {
        pool.lifetime -= dt;

        // Наносим периодический урон всем мобам в луже
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

    // 5. Магнетизм кристаллов душ к боссу
    let magnet_r_sq = world.boss_inventory.magnet_radius.powi(2);
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
