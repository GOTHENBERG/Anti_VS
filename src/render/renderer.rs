use macroquad::prelude::*;
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::world::World;
use crate::render::font::GameFonts;
use crate::render::sprites::{
    draw_blood_altar, draw_boss, draw_environment, draw_mob_sprite, draw_projectile_sprite,
};

pub fn render_world(world: &World, fonts: &GameFonts) {
    let time = get_time() as f32;

    // 1. Отрисовка большой арены (плиты, разломы, колонны)
    draw_environment(crate::core::MAP_WIDTH, crate::core::MAP_HEIGHT);

    // 2. Кислотные лужи (Poison Pools)
    for pool in &world.acid_pools {
        let alpha = (pool.lifetime / 4.0).min(0.65);
        let color = if pool.corrosive {
            Color::new(0.1, 0.85, 0.2, alpha)
        } else {
            Color::new(0.25, 0.75, 0.1, alpha)
        };
        draw_circle(pool.pos.x, pool.pos.y, pool.radius, color);
        draw_circle_lines(pool.pos.x, pool.pos.y, pool.radius, 2.0, Color::new(0.3, 0.95, 0.3, alpha * 0.9));

        // Пузыри на поверхности лужи
        let bubble_angle = (time * 6.0 + pool.pos.x).sin() * 0.5 * pool.radius;
        draw_circle(pool.pos.x + bubble_angle, pool.pos.y + (time * 4.0).cos() * 10.0, 4.0, Color::new(0.4, 1.0, 0.4, alpha));
    }

    // 3. Разрушаемые структуры (Алтари Крови)
    for structure in &world.structures {
        draw_blood_altar(structure.pos, structure.health.current, structure.health.max, time);
    }

    // 4. Кристаллы душ (Soul Gems & Magnet Orbs)
    for gem in &world.gems {
        if gem.is_magnet {
            // Редкий артефакт "Магнит"
            let pulse = (time * 6.0).sin() * 2.5;
            draw_circle(gem.pos.x, gem.pos.y, 9.0 + pulse, GOLD);
            draw_circle_lines(gem.pos.x, gem.pos.y, 14.0 + pulse, 1.5, SKYBLUE);
            draw_circle(gem.pos.x, gem.pos.y, 4.0, WHITE);
        } else {
            let size = if gem.xp_value > 50.0 { 5.5 } else { 4.0 };
            let (color, border) = if gem.xp_value > 100.0 {
                (GOLD, YELLOW)
            } else if gem.xp_value > 30.0 {
                (GREEN, LIME)
            } else {
                (SKYBLUE, Color::new(0.3, 0.9, 1.0, 1.0))
            };
            draw_circle(gem.pos.x, gem.pos.y, size, color);
            draw_circle_lines(gem.pos.x, gem.pos.y, size, 1.0, border);
        }
    }

    // 5. Отрисовка мобов с процедурными модельками
    for mob in &world.mobs {
        draw_mob_sprite(mob, time);

        // Наложение статусных эффектов (заморозка, горение, яд)
        if mob.status.is_frozen() {
            draw_circle(mob.pos.x, mob.pos.y, mob.radius + 2.0, Color::new(0.5, 0.85, 1.0, 0.5));
            draw_circle_lines(mob.pos.x, mob.pos.y, mob.radius + 2.0, 1.5, WHITE);
        } else if mob.status.burn_timer > 0.0 {
            let flame_pulse = (time * 15.0 + (mob.id as f32)).sin() * 2.0;
            draw_circle_lines(mob.pos.x, mob.pos.y, mob.radius + flame_pulse, 1.5, ORANGE);
        } else if mob.status.poison_timer > 0.0 {
            draw_circle_lines(mob.pos.x, mob.pos.y, mob.radius + 1.0, 1.5, LIME);
        }
    }

    // 6. Отрисовка снарядов с уникальными модельками
    for proj in &world.projectiles {
        draw_projectile_sprite(proj.pos, proj.vel, proj.radius, proj.damage_type);
    }

    // 7. Луч Смерти (Death Ray)
    if let Some((start, end, timer)) = world.death_ray_active {
        let alpha = (timer / 0.35).clamp(0.0, 1.0);
        let ray_color = Color::new(0.4, 0.8, 1.0, alpha);
        let core_color = Color::new(1.0, 1.0, 1.0, alpha);

        draw_line(start.x, start.y, end.x, end.y, 22.0, ray_color);
        draw_line(start.x, start.y, end.x, end.y, 8.0, core_color);
    }

    // 8. Отрисовка Босса (Соло-героя)
    let bx = world.boss_pos.x;
    let by = world.boss_pos.y;

    // Аура медитации (если отдыхает)
    if world.boss_still_time >= 2.0 {
        let pulse = (time * 5.0).sin() * 6.0;
        draw_circle_lines(bx, by, 45.0 + pulse, 3.5, Color::new(0.2, 1.0, 0.4, 0.9));
        draw_circle_lines(bx, by, 60.0 + pulse * 1.5, 2.0, Color::new(0.2, 1.0, 0.4, 0.5));
    }

    // Эволюция: Буря Клинков (BladeStorm) — вращающийся вихрь клинков
    if world.boss_inventory.has_weapon(WeaponType::BladeStorm) {
        let blade_count = 8;
        let radius = 78.0;
        for i in 0..blade_count {
            let angle = world.blade_storm_angle + (i as f32) * std::f32::consts::PI * 2.0 / (blade_count as f32);
            let b_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);
            draw_circle(b_pos.x, b_pos.y, 7.0, SKYBLUE);
            draw_circle_lines(b_pos.x, b_pos.y, 7.0, 2.0, WHITE);
            // Лезвие клинка
            let tip = b_pos + Vec2::new(angle.cos(), angle.sin()) * 12.0;
            draw_line(b_pos.x, b_pos.y, tip.x, tip.y, 3.5, WHITE);
        }
    }

    // Эволюция: Праведный Щит (DivineShield) — вращающиеся священные щиты
    if world.boss_inventory.has_weapon(WeaponType::DivineShield) {
        let shield_count = 3;
        let radius = 98.0;
        for i in 0..shield_count {
            let angle = world.divine_shield_angle + (i as f32) * std::f32::consts::PI * 2.0 / (shield_count as f32);
            let s_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);
            draw_circle(s_pos.x, s_pos.y, 12.0, GOLD);
            draw_circle_lines(s_pos.x, s_pos.y, 12.0, 2.5, WHITE);
            // Крест на щите
            draw_line(s_pos.x - 6.0, s_pos.y, s_pos.x + 6.0, s_pos.y, 2.0, WHITE);
            draw_line(s_pos.x, s_pos.y - 6.0, s_pos.x, s_pos.y + 6.0, 2.0, WHITE);
        }
    }

    // Тело и экипировка Босса
    draw_boss(
        world.boss_pos,
        world.boss_vel,
        world.boss_invulnerable_timer > 0.0,
        time,
    );

    // 9. Отрисовка частиц
    for p in &world.particles {
        let alpha = (p.lifetime / p.max_lifetime).clamp(0.0, 1.0);
        let mut color = p.color;
        color.a = alpha;
        draw_circle(p.pos.x, p.pos.y, p.radius, color);
    }

    // 10. Отрисовка всплывающего текста урона (шрифтом с поддержкой кириллицы)
    for text in &world.floating_texts {
        let alpha = (text.lifetime / 0.8).clamp(0.0, 1.0);
        let mut color = text.color;
        color.a = alpha;
        let font_size = 18.0 * text.scale;
        let tw = fonts.measure_text(&text.text, font_size).width;
        fonts.draw_text(&text.text, text.pos.x - tw / 2.0, text.pos.y, font_size, color);
    }
}
