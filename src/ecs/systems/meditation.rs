use macroquad::prelude::*;
use crate::core::rand;
use crate::ecs::systems::boss_ai::handle_boss_level_up;
use crate::ecs::world::World;

pub fn update_meditation(world: &mut World, dt: f32) {
    let boss_pos = world.boss_pos;

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

    // Если босс стоит на месте (скорость < 5 px/sec) и рядом нет угрозы
    if world.boss_vel.length_squared() < 25.0 && !has_nearby_mobs {
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
}
