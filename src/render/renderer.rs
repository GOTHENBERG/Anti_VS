use macroquad::prelude::*;
use crate::ecs::components::combat::{DamageType, Faction};
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::world::World;

pub fn render_world(world: &World) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // Эффект тряски экрана
    let (shake_x, shake_y) = if world.screen_shake > 0.0 {
        (
            rand::gen_range(-world.screen_shake, world.screen_shake),
            rand::gen_range(-world.screen_shake, world.screen_shake),
        )
    } else {
        (0.0, 0.0)
    };

    // 1. Темный инфернальный фон арены с разметкой
    clear_background(Color::new(0.06, 0.06, 0.09, 1.0));

    // Сетка арены
    let grid_step = 64.0;
    let grid_color = Color::new(0.10, 0.10, 0.14, 1.0);
    let mut x = shake_x % grid_step;
    while x < screen_w {
        draw_line(x, 0.0, x, screen_h, 1.0, grid_color);
        x += grid_step;
    }
    let mut y = shake_y % grid_step;
    while y < screen_h {
        draw_line(0.0, y, screen_w, y, 1.0, grid_color);
        y += grid_step;
    }

    // 2. Кислотные лужи (Poison Pools)
    for pool in &world.acid_pools {
        let px = pool.pos.x + shake_x;
        let py = pool.pos.y + shake_y;
        let alpha = (pool.lifetime / 4.0).min(0.6);
        let color = if pool.corrosive {
            Color::new(0.1, 0.85, 0.2, alpha)
        } else {
            Color::new(0.2, 0.7, 0.1, alpha)
        };
        draw_circle(px, py, pool.radius, color);
        draw_circle_lines(px, py, pool.radius, 1.5, Color::new(0.3, 0.9, 0.3, alpha * 0.8));
    }

    // 3. Разрушаемые структуры (Алтари Крови)
    for structure in &world.structures {
        let sx = structure.pos.x + shake_x;
        let sy = structure.pos.y + shake_y;
        draw_circle(sx, sy, structure.radius + 4.0, Color::new(0.3, 0.0, 0.4, 0.5));
        draw_circle(sx, sy, structure.radius, Color::new(0.5, 0.1, 0.7, 1.0));
        draw_circle_lines(sx, sy, structure.radius, 2.5, PURPLE);

        // Полоска здоровья алтаря
        let hp_pct = structure.health.current / structure.health.max;
        draw_rectangle(sx - 20.0, sy - structure.radius - 12.0, 40.0, 5.0, DARKGRAY);
        draw_rectangle(sx - 20.0, sy - structure.radius - 12.0, 40.0 * hp_pct, 5.0, RED);
    }

    // 4. Кристаллы душ (Soul Gems & Magnet Orbs)
    for gem in &world.gems {
        let gx = gem.pos.x + shake_x;
        let gy = gem.pos.y + shake_y;

        if gem.is_magnet {
            // Редкий артефакт "Магнит"
            let pulse = (get_time() as f32 * 6.0).sin() * 2.0;
            draw_circle(gx, gy, 8.0 + pulse, GOLD);
            draw_circle_lines(gx, gy, 12.0 + pulse, 1.5, SKYBLUE);
        } else {
            let size = if gem.xp_value > 50.0 { 5.0 } else { 3.5 };
            let color = if gem.xp_value > 100.0 {
                GOLD
            } else if gem.xp_value > 30.0 {
                GREEN
            } else {
                SKYBLUE
            };
            draw_circle(gx, gy, size, color);
        }
    }

    // 5. Отрисовка мобов (по тирам с визуальными отличиями)
    for mob in &world.mobs {
        let mx = mob.pos.x + shake_x;
        let my = mob.pos.y + shake_y;

        // Базовый цвет тира
        let base_color = match mob.tier {
            1 => Color::new(0.85, 0.85, 0.85, 1.0), // Скелеты (пепельно-белые)
            2 => Color::new(0.85, 0.35, 0.15, 1.0), // Чёрные гончие (рыже-черные)
            3 => Color::new(0.65, 0.25, 0.85, 1.0), // Культисты (темно-фиолетовые)
            4 => Color::new(0.25, 0.75, 0.95, 1.0), // Ледяные големы (морозно-голубые)
            _ => Color::new(0.95, 0.20, 0.20, 1.0), // Камикадзе (ярко-красные)
        };

        // Наложение статуса
        let render_color = if mob.status.is_frozen() {
            SKYBLUE
        } else if mob.status.burn_timer > 0.0 {
            ORANGE
        } else if mob.status.poison_timer > 0.0 {
            LIME
        } else {
            base_color
        };

        // Тело моба
        draw_circle(mx, my, mob.radius, render_color);
        draw_circle_lines(mx, my, mob.radius, 1.5, BLACK);

        // Индивидуальные черты мобов
        match mob.tier {
            2 => {
                // Гончая: направление взгляда
                let dir = mob.vel.normalize_or_zero();
                draw_circle(mx + dir.x * 5.0, my + dir.y * 5.0, 3.0, RED);
            }
            3 => {
                // Культист: капюшон и посох
                draw_circle(mx, my, 4.0, DARKPURPLE);
            }
            4 => {
                // Голем: бронеплиты и полоска здоровья
                draw_circle_lines(mx, my, mob.radius - 4.0, 2.0, WHITE);
                let hp_pct = mob.health.current / mob.health.max;
                draw_rectangle(mx - 15.0, my - mob.radius - 8.0, 30.0, 4.0, DARKGRAY);
                draw_rectangle(mx - 15.0, my - mob.radius - 8.0, 30.0 * hp_pct, 4.0, SKYBLUE);
            }
            5 => {
                // Камикадзе: пульсирующий фитиль
                let pulse = (get_time() as f32 * 12.0).sin() * 2.0;
                draw_circle(mx, my, 4.0 + pulse, YELLOW);
            }
            _ => {}
        }
    }

    // 6. Отрисовка снарядов
    for proj in &world.projectiles {
        let px = proj.pos.x + shake_x;
        let py = proj.pos.y + shake_y;

        let proj_color = match proj.damage_type {
            DamageType::Holy => GOLD,
            DamageType::Ice => SKYBLUE,
            DamageType::Fire => ORANGE,
            DamageType::Poison => GREEN,
            DamageType::True => PURPLE,
            DamageType::Physical => {
                if proj.faction == Faction::Director {
                    Color::new(0.8, 0.2, 0.4, 1.0)
                } else {
                    WHITE
                }
            }
        };

        draw_circle(px, py, proj.radius, proj_color);
        draw_circle_lines(px, py, proj.radius, 1.0, WHITE);
    }

    // 7. Луч Смерти (Death Ray)
    if let Some((start, end, timer)) = world.death_ray_active {
        let alpha = (timer / 0.35).clamp(0.0, 1.0);
        let ray_color = Color::new(0.4, 0.8, 1.0, alpha);
        let core_color = Color::new(1.0, 1.0, 1.0, alpha);

        draw_line(start.x + shake_x, start.y + shake_y, end.x + shake_x, end.y + shake_y, 16.0, ray_color);
        draw_line(start.x + shake_x, start.y + shake_y, end.x + shake_x, end.y + shake_y, 6.0, core_color);
    }

    // 8. Отрисовка Босса (Соло-героя)
    let bx = world.boss_pos.x + shake_x;
    let by = world.boss_pos.y + shake_y;

    // Аура медитации (если отдыхает)
    if world.boss_still_time >= 2.0 {
        let pulse = (get_time() as f32 * 5.0).sin() * 6.0;
        draw_circle_lines(bx, by, 42.0 + pulse, 3.0, Color::new(0.2, 1.0, 0.4, 0.9));
        draw_circle_lines(bx, by, 55.0 + pulse * 1.5, 1.5, Color::new(0.2, 1.0, 0.4, 0.5));
    }

    // Эволюция: Буря Клинков (BladeStorm) — вращающийся вихрь клинков
    if world.boss_inventory.has_weapon(WeaponType::BladeStorm) {
        let blade_count = 8;
        let radius = 75.0;
        for i in 0..blade_count {
            let angle = world.blade_storm_angle + (i as f32) * std::f32::consts::PI * 2.0 / (blade_count as f32);
            let b_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);
            draw_circle(b_pos.x, b_pos.y, 6.0, SKYBLUE);
            draw_circle_lines(b_pos.x, b_pos.y, 6.0, 1.5, WHITE);
            // Хвост клинка
            let tail_angle = angle - 0.25;
            let t_pos = Vec2::new(bx + tail_angle.cos() * radius, by + tail_angle.sin() * radius);
            draw_line(b_pos.x, b_pos.y, t_pos.x, t_pos.y, 3.0, Color::new(0.5, 0.9, 1.0, 0.6));
        }
    }

    // Эволюция: Праведный Щит (DivineShield) — вращающиеся золотые щиты
    if world.boss_inventory.has_weapon(WeaponType::DivineShield) {
        let shield_count = 3;
        let radius = 95.0;
        for i in 0..shield_count {
            let angle = world.divine_shield_angle + (i as f32) * std::f32::consts::PI * 2.0 / (shield_count as f32);
            let s_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);
            draw_circle(s_pos.x, s_pos.y, 11.0, GOLD);
            draw_circle_lines(s_pos.x, s_pos.y, 11.0, 2.0, WHITE);
        }
    }

    // Тело босса
    let boss_color = if world.boss_invulnerable_timer > 0.0 {
        Color::new(1.0, 1.0, 0.3, 0.7) // Вспышка неуязвимости
    } else {
        YELLOW
    };
    draw_circle(bx, by, 22.0, boss_color);
    draw_circle_lines(bx, by, 22.0, 2.5, WHITE);

    // Указатель направления движения
    if world.boss_vel.length_squared() > 1.0 {
        let dir = world.boss_vel.normalize();
        draw_line(bx, by, bx + dir.x * 30.0, by + dir.y * 30.0, 3.5, RED);
    }

    // 9. Отрисовка частиц
    for p in &world.particles {
        let alpha = (p.lifetime / p.max_lifetime).clamp(0.0, 1.0);
        let mut color = p.color;
        color.a = alpha;
        draw_circle(p.pos.x + shake_x, p.pos.y + shake_y, p.radius, color);
    }

    // 10. Отрисовка всплывающего текста (Floating text)
    for text in &world.floating_texts {
        let alpha = (text.lifetime / 0.8).clamp(0.0, 1.0);
        let mut color = text.color;
        color.a = alpha;
        let font_size = (18.0 * text.scale).round() as u16;
        let tw = measure_text(&text.text, None, font_size, 1.0).width;
        draw_text(&text.text, text.pos.x - tw / 2.0 + shake_x, text.pos.y + shake_y, font_size as f32, color);
    }
}
