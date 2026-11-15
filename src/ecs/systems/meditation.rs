use crate::ecs::world::World;

pub fn update_meditation(world: &mut World, dt: f32) {
    // Если босс практически стоит на месте (скорость < 5.0 px/sec)
    if world.boss_vel.length_squared() < 25.0 {
        world.boss_still_time += dt;

        // Если стоит дольше 2 секунд подряд -> медитация
        if world.boss_still_time >= 2.0 {
            // Восстановление 2% макс. ХП в секунду
            let heal_amount = world.boss_health.max * 0.02 * dt;
            world.boss_health.heal(heal_amount);

            // Пассивный опыт
            let xp_gain = 30.0 * dt;
            world.boss_inventory.add_xp(xp_gain);
        }
    } else {
        world.boss_still_time = 0.0;
    }
}
