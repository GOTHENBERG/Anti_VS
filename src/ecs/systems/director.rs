use crate::ecs::world::World;

pub fn update_director(world: &mut World, dt: f32) {
    for director in &mut world.directors {
        // Темп призыва: сколько секунд прошло с последнего отряда
        director.time_since_last_spawn += dt;

        // Уменьшение кулдаунов призыва отрядов
        for cd in &mut director.squad_cooldowns {
            if *cd > 0.0 {
                *cd = (*cd - dt).max(0.0);
            }
        }

        // Уменьшение персонального кулдауна Алтаря Крови
        if director.altar_cooldown > 0.0 {
            director.altar_cooldown = (director.altar_cooldown - dt).max(0.0);
        }

        // Тиковый доход (ровно каждые 2.0 секунды)
        director.income_timer += dt;
        if director.income_timer >= 2.0 {
            director.income_timer -= 2.0;
            // Базовый доход плюс бонус от каждого живого Алтаря Крови (+3.0 душ)
            let altar_income = world.structures.len() as f32 * 3.0;
            director.points += director.income_rate + altar_income;
        }
    }
}
