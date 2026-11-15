use macroquad::prelude::*;
use crate::core::math::{calculate_attract_force, calculate_repel_force};
use crate::ecs::world::World;

pub fn update_boss_ai(world: &mut World, dt: f32) {
    if world.boss_dash_cooldown > 0.0 {
        world.boss_dash_cooldown -= dt;
    }

    let boss_pos = world.boss_pos;
    let mut total_force = Vec2::ZERO;

    // 1. Отталкивание от мобов (потенциальное поле)
    for mob in &world.mobs {
        let repel = calculate_repel_force(boss_pos, mob.pos, 180.0, 350.0);
        total_force += repel;
    }

    // 2. Притяжение к кристаллам душ
    for gem in &world.gems {
        let attract = calculate_attract_force(boss_pos, gem.pos, 220.0, 150.0);
        total_force += attract;
    }

    // 3. Отталкивание от границ экрана
    let screen_w = screen_width();
    let screen_h = screen_height();
    let wall_margin = 80.0;
    let wall_push = 400.0;

    if boss_pos.x < wall_margin {
        total_force.x += wall_push * (1.0 - boss_pos.x / wall_margin);
    } else if boss_pos.x > screen_w - wall_margin {
        total_force.x -= wall_push * (1.0 - (screen_w - boss_pos.x) / wall_margin);
    }

    if boss_pos.y < wall_margin {
        total_force.y += wall_push * (1.0 - boss_pos.y / wall_margin);
    } else if boss_pos.y > screen_h - wall_margin {
        total_force.y -= wall_push * (1.0 - (screen_h - boss_pos.y) / wall_margin);
    }

    // 4. Проверка на экстренный рывок (Dash) при окружении
    let nearby_mobs = world.mobs.iter()
        .filter(|m| m.pos.distance_squared(boss_pos) < 90.0 * 90.0)
        .count();

    if (nearby_mobs > 15 || world.boss_health.current < world.boss_health.max * 0.3)
        && world.boss_dash_cooldown <= 0.0
    {
        // 8-лучевой радар для поиска самого безопасного сектора
        let mut best_dir = Vec2::X;
        let mut min_danger = usize::MAX;

        for i in 0..8 {
            let angle = (i as f32) * std::f32::consts::PI / 4.0;
            let dir = Vec2::new(angle.cos(), angle.sin());
            let ray_pos = boss_pos + dir * 150.0;

            let danger = world.mobs.iter()
                .filter(|m| m.pos.distance_squared(ray_pos) < 100.0 * 100.0)
                .count();

            if danger < min_danger {
                min_danger = danger;
                best_dir = dir;
            }
        }

        // Выполняем рывок
        world.boss_pos += best_dir * 180.0;
        world.boss_dash_cooldown = 7.0;
    } else {
        // Обычное плавное перемещение
        let max_speed = 160.0;
        let final_vel = if total_force.length_squared() > 1.0 {
            total_force.clamp_length_max(max_speed)
        } else {
            Vec2::ZERO
        };

        world.boss_vel = final_vel;
        world.boss_pos += world.boss_vel * dt;
    }

    // Ограничиваем координаты границами экрана
    world.boss_pos.x = world.boss_pos.x.clamp(20.0, screen_w - 20.0);
    world.boss_pos.y = world.boss_pos.y.clamp(20.0, screen_h - 20.0);
}
