use macroquad::prelude::*;
use crate::ecs::world::World;

pub fn render_world(world: &World) {
    clear_background(Color::new(0.08, 0.08, 0.12, 1.0));

    // 1. Отрисовка разрушаемых структур
    for structure in &world.structures {
        draw_circle(structure.pos.x, structure.pos.y, structure.radius, PURPLE);
        draw_circle_lines(structure.pos.x, structure.pos.y, structure.radius, 2.0, WHITE);
    }

    // 2. Отрисовка кристаллов душ (светящиеся ромбики/круги)
    for gem in &world.gems {
        draw_circle(gem.pos.x, gem.pos.y, 4.0, SKYBLUE);
    }

    // 3. Отрисовка мобов (цвет зависит от тира)
    for mob in &world.mobs {
        let color = match mob.tier {
            1 => Color::new(0.8, 0.8, 0.8, 1.0), // Скелеты (серые)
            2 => Color::new(0.9, 0.5, 0.2, 1.0), // Гончие (оранжевые)
            3 => Color::new(0.7, 0.2, 0.8, 1.0), // Культисты (фиолетовые)
            4 => Color::new(0.2, 0.7, 0.9, 1.0), // Големы (ледяные)
            _ => Color::new(0.9, 0.2, 0.2, 1.0), // Камикадзе (красные)
        };
        draw_circle(mob.pos.x, mob.pos.y, mob.radius, color);
    }

    // 4. Отрисовка снарядов
    for proj in &world.projectiles {
        draw_circle(proj.pos.x, proj.pos.y, proj.radius, GOLD);
    }

    // 5. Отрисовка босса
    let boss_pos = world.boss_pos;

    // Аура медитации (если босс отдыхает)
    if world.boss_still_time >= 2.0 {
        let pulse = (get_time() as f32 * 4.0).sin() * 4.0;
        draw_circle_lines(boss_pos.x, boss_pos.y, 35.0 + pulse, 3.0, Color::new(0.3, 1.0, 0.4, 0.8));
    }

    // Тело босса
    draw_circle(boss_pos.x, boss_pos.y, 20.0, YELLOW);
    draw_circle_lines(boss_pos.x, boss_pos.y, 20.0, 2.5, WHITE);

    // Индикатор взгляда/скорости
    if world.boss_vel.length_squared() > 1.0 {
        let dir = world.boss_vel.normalize();
        draw_line(boss_pos.x, boss_pos.y, boss_pos.x + dir.x * 26.0, boss_pos.y + dir.y * 26.0, 3.0, RED);
    }
}
