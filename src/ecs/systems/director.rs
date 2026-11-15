use crate::ecs::world::World;

pub fn update_director(world: &mut World, dt: f32) {
    for director in &mut world.directors {
        // Уменьшение кулдаунов призыва отрядов
        for cd in &mut director.squad_cooldowns {
            if *cd > 0.0 {
                *cd = (*cd - dt).max(0.0);
            }
        }

        // Тиковый доход (ровно каждые 2.0 секунды)
        director.income_timer += dt;
        if director.income_timer >= 2.0 {
            director.income_timer -= 2.0;
            director.points += director.income_rate;
        }
    }
}
