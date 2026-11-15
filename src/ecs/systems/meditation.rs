use macroquad::prelude::*;
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
        world.boss_still_time += dt;

        // Если стоит более 2.0 секунд подряд -> медитация активирована!
        if world.boss_still_time >= 2.0 {
            // Восстановление 2% от максимального здоровья в секунду
            let heal_amount = world.boss_health.max * 0.02 * dt;
            world.boss_health.heal(heal_amount);

            // Получение +30 опыта в секунду
            let xp_gain = 30.0 * dt;
            let leveled_up = world.boss_inventory.add_xp(xp_gain);
            if leveled_up {
                handle_boss_level_up(world);
            }

            // Визуальные частицы медитации
            if rand::gen_range(0.0, 1.0) < 0.25 {
                world.particles.push(crate::ecs::world::ParticleEntity {
                    pos: boss_pos + Vec2::new(rand::gen_range(-25.0, 25.0), rand::gen_range(-25.0, 25.0)),
                    vel: Vec2::new(0.0, -35.0),
                    color: Color::new(0.2, 1.0, 0.4, 0.8),
                    radius: 2.5,
                    lifetime: 0.6,
                    max_lifetime: 0.6,
                });
            }
        }
    } else {
        world.boss_still_time = 0.0;
    }
}
