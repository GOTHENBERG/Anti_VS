use macroquad::prelude::*;
use crate::core::rand;
use crate::ecs::components::inventory::PassiveType;
use crate::ecs::systems::boss_ai::handle_boss_level_up;
use crate::ecs::world::World;

pub fn update_meditation(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;

    // Снижаем таймер кулдауна после получения урона (3.0 сек)
    if world.boss_damage_taken_timer > 0.0 {
        world.boss_damage_taken_timer = (world.boss_damage_taken_timer - dt).max(0.0);
    }

    // 1. СТАНДАРТНАЯ МЕДИТАЦИЯ
    // Проверяем, есть ли рядом мобы
    let has_nearby_mobs = world.spatial_grid.query_nearby(boss_pos, 160.0)
        .iter()
        .any(|&idx| {
            if let Some(mob) = world.mobs.get(idx) {
                mob.pos.distance_squared(boss_pos) < 160.0 * 160.0
            } else {
                false
            }
        });

    // Медитация работает ТОЛЬКО если:
    // 1) Босс стоит на месте (скорость < 5 px/sec)
    // 2) Рядом нет врагов (!has_nearby_mobs)
    // 3) Босс НЕ получал урон последние 3 секунды (boss_damage_taken_timer <= 0.0)
    if world.boss_vel.length_squared() < 25.0 && !has_nearby_mobs && world.boss_damage_taken_timer <= 0.0 {
        let prev_still = world.boss_still_time;
        world.boss_still_time += dt;

        // Если стоит более 1.2 секунды подряд -> медитация активирована!
        if world.boss_still_time >= 1.2 {
            if prev_still < 1.2 {
                world.add_floating_text(boss_pos + Vec2::new(0.0, -35.0), "МЕДИТАЦИЯ (+HP, +XP)".to_string(), GREEN, 1.2);
            }
            // Восстановление 2% от максимального здоровья в секунду
            let heal_amount = world.boss_health.max * 0.02 * dt;
            world.boss_health.heal(heal_amount);

            // Получение +30 опыта в секунду
            let xp_gain = 30.0 * dt;
            let level_ups = world.boss_inventory.add_xp(xp_gain);
            for _ in 0..level_ups {
                world.boss_health.max += 40.0;
                world.boss_health.current = (world.boss_health.current + 30.0).min(world.boss_health.max);
                world.boss_resistances.flat_armor += 0.5;
                handle_boss_level_up(world);
            }

            // Визуальные частицы медитации (зеленые и золотые искры вокруг босса)
            if !world.is_headless && rand::gen_range(0.0, 1.0) < 0.40 {
                world.particles.push(crate::ecs::world::ParticleEntity {
                    pos: boss_pos + Vec2::new(rand::gen_range(-25.0, 25.0), rand::gen_range(-25.0, 25.0)),
                    vel: Vec2::new(rand::gen_range(-10.0, 10.0), -40.0),
                    color: Color::new(0.2, 1.0, 0.4, 0.85),
                    radius: 3.0,
                    lifetime: 0.7,
                    max_lifetime: 0.7,
                });
            }
        }
    } else {
        world.boss_still_time = 0.0;
    }

    // 2. РЕГЕНЕРАЦИЯ ОТ СПОРОВОГО МИЦЕЛИЯ (АСТРАЛЬНЫЕ ГРИБЫ)
    // Работает ВНЕ ЗАВИСИМОСТИ от врагов и урона — главное стоять на месте!
    let mushroom_lvl = world.boss_inventory.get_passive_level(PassiveType::SporeMushroom);
    if mushroom_lvl > 0 && world.boss_vel.length_squared() < 25.0 {
        let prev_mush = world.boss_mushroom_still_time;
        world.boss_mushroom_still_time += dt;

        // Активация через 0.4 секунды после полной остановки
        if world.boss_mushroom_still_time >= 0.4 {
            let heal_per_sec = 18.0 * (mushroom_lvl as f32) + world.boss_health.max * 0.015 * (mushroom_lvl as f32);
            let heal_amount = heal_per_sec * dt;
            world.boss_health.heal(heal_amount);

            // Текстовая плашка восстановления раз в секунду
            if prev_mush < 0.4 || (world.boss_mushroom_still_time % 1.0 < dt) {
                world.add_floating_text(
                    boss_pos + Vec2::new(0.0, -48.0),
                    format!("+{:.0} HP (МИЦЕЛИЙ)", heal_per_sec),
                    Color::new(0.1, 0.95, 0.55, 1.0),
                    0.85,
                );
            }

            // Специфичная анимация: поднимающиеся биолюминесцентные споры
            if !world.is_headless && rand::gen_range(0.0, 1.0) < 0.65 {
                let spore_x = rand::gen_range(-34.0, 34.0);
                let spore_y = rand::gen_range(-34.0, 34.0);
                let spore_col = if rand::gen_range(0.0, 1.0) < 0.5 {
                    Color::new(0.15, 1.0, 0.60, 0.90) // изумрудные споры
                } else {
                    Color::new(0.20, 0.90, 1.00, 0.90) // бирюзовые споры
                };
                world.particles.push(crate::ecs::world::ParticleEntity {
                    pos: boss_pos + Vec2::new(spore_x, spore_y),
                    vel: Vec2::new(rand::gen_range(-6.0, 6.0), rand::gen_range(-32.0, -14.0)),
                    color: spore_col,
                    radius: rand::gen_range(2.0, 4.2),
                    lifetime: 0.80,
                    max_lifetime: 0.80,
                });
            }
        }
    } else {
        world.boss_mushroom_still_time = 0.0;
    }
}
