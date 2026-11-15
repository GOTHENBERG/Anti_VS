use macroquad::prelude::*;
use crate::content::mobs::get_squad_config;
use crate::content::structures::BLOOD_ALTAR_COST;
use crate::core::time::GameClock;
use crate::ecs::world::World;
use crate::render::font::GameFonts;
use crate::render::textures::GameTextures;

pub fn render_ui(
    world: &World,
    clock: &GameClock,
    fonts: &GameFonts,
    cam_pos: Vec2,
    cam_zoom: f32,
    textures: &GameTextures,
) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // 1. Верхний таймер 30:00 обратного отсчета
    let timer_text = clock.formatted_remaining();
    let timer_font_size = 34.0;
    let timer_w = fonts.measure_text(&timer_text, timer_font_size).width;
    fonts.draw_text(&timer_text, (screen_w - timer_w) / 2.0, 36.0, timer_font_size, WHITE);

    // 2. Полоска здоровья Босса и Опыт (сверху под таймером)
    let bar_w = 480.0;
    let bar_h = 22.0;
    let bar_x = (screen_w - bar_w) / 2.0;
    let bar_y = 48.0;

    let hp_ratio = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0);
    draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.12, 0.12, 0.18, 0.95));
    draw_rectangle(bar_x, bar_y, bar_w * hp_ratio, bar_h, Color::new(0.9, 0.2, 0.2, 1.0));
    draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 2.0, WHITE);

    let hp_text = format!(
        "{:.0} / {:.0} HP  (Босс ур. {}) | Броня: {:.0}",
        world.boss_health.current,
        world.boss_health.max,
        world.boss_inventory.level,
        world.boss_resistances.flat_armor
    );
    let hp_text_w = fonts.measure_text(&hp_text, 15.0).width;
    fonts.draw_text(&hp_text, (screen_w - hp_text_w) / 2.0, bar_y + 16.0, 15.0, WHITE);

    // Полоска опыта босса
    let xp_ratio = (world.boss_inventory.current_xp / world.boss_inventory.xp_to_next_level).clamp(0.0, 1.0);
    draw_rectangle(bar_x, bar_y + bar_h + 3.0, bar_w, 5.0, Color::new(0.15, 0.15, 0.25, 0.9));
    draw_rectangle(bar_x, bar_y + bar_h + 3.0, bar_w * xp_ratio, 5.0, SKYBLUE);

    // 3. Инвентарь Босса (6 Оружий + 6 Пассивок) с HD иконками
    let inv_y = bar_y + bar_h + 12.0;
    let slot_w = 38.0;
    let slot_h = 28.0;
    let total_slots_w = 12.0 * (slot_w + 4.0);
    let start_inv_x = (screen_w - total_slots_w) / 2.0;

    // 6 слотов оружия
    for i in 0..6 {
        let sx = start_inv_x + (i as f32) * (slot_w + 4.0);
        draw_rectangle(sx, inv_y, slot_w, slot_h, Color::new(0.08, 0.1, 0.16, 0.92));
        draw_rectangle_lines(sx, inv_y, slot_w, slot_h, 1.5, GRAY);

        if let Some(w) = world.boss_inventory.weapons.get(i) {
            let bg_color = if w.is_evolved {
                Color::new(0.65, 0.45, 0.05, 0.85)
            } else {
                Color::new(0.12, 0.28, 0.48, 0.85)
            };
            draw_rectangle(sx + 2.0, inv_y + 2.0, slot_w - 4.0, slot_h - 4.0, bg_color);

            // Отрисовка HD иконки оружия
            if let Some(icon) = textures.get_weapon_icon(w.kind) {
                draw_texture_ex(
                    icon,
                    sx + 4.0,
                    inv_y + 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(slot_w - 8.0, slot_h - 4.0)),
                        ..Default::default()
                    },
                );
            }

            let lvl_text = format!("{}", w.level);
            let text_color = if w.is_evolved { GOLD } else { WHITE };
            fonts.draw_text(&lvl_text, sx + 3.0, inv_y + slot_h - 4.0, 13.0, text_color);
        }
    }

    // Разделитель слотов оружия и пассивок
    draw_line(
        start_inv_x + 6.0 * (slot_w + 4.0) - 2.0,
        inv_y,
        start_inv_x + 6.0 * (slot_w + 4.0) - 2.0,
        inv_y + slot_h,
        2.0,
        WHITE,
    );

    // 6 слотов пассивок
    for i in 0..6 {
        let sx = start_inv_x + ((i + 6) as f32) * (slot_w + 4.0);
        draw_rectangle(sx, inv_y, slot_w, slot_h, Color::new(0.08, 0.1, 0.16, 0.92));
        draw_rectangle_lines(sx, inv_y, slot_w, slot_h, 1.5, GRAY);

        if let Some(p) = world.boss_inventory.passives.get(i) {
            draw_rectangle(sx + 2.0, inv_y + 2.0, slot_w - 4.0, slot_h - 4.0, Color::new(0.35, 0.12, 0.42, 0.85));

            // Отрисовка HD иконки пассивки
            if let Some(icon) = textures.get_passive_icon(p.kind) {
                draw_texture_ex(
                    icon,
                    sx + 4.0,
                    inv_y + 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(slot_w - 8.0, slot_h - 4.0)),
                        ..Default::default()
                    },
                );
            }

            let lvl_text = format!("{}", p.level);
            fonts.draw_text(&lvl_text, sx + 3.0, inv_y + slot_h - 4.0, 13.0, WHITE);
        }
    }

    // 4. Баннер Level-Up / Эволюции Босса
    if world.boss_inventory.perk_announcement_timer > 0.0 {
        if let Some(ref perk_text) = world.boss_inventory.last_chosen_perk {
            let banner_w = 460.0;
            let banner_h = 34.0;
            let bx = (screen_w - banner_w) / 2.0;
            let by = inv_y + slot_h + 8.0;

            draw_rectangle(bx, by, banner_w, banner_h, Color::new(0.08, 0.08, 0.14, 0.96));
            draw_rectangle_lines(bx, by, banner_w, banner_h, 2.0, GOLD);

            let tw = fonts.measure_text(perk_text, 18.0).width;
            fonts.draw_text(perk_text, (screen_w - tw) / 2.0, by + 23.0, 18.0, GOLD);
        }
    }

    // 5. Панель Директора (слева вверху)
    if let Some(director) = world.directors.first() {
        draw_rectangle(15.0, 15.0, 280.0, 150.0, Color::new(0.08, 0.08, 0.12, 0.92));
        draw_rectangle_lines(15.0, 15.0, 280.0, 150.0, 2.0, DARKGRAY);

        let points_text = format!("Очки Душ: {:.0}", director.points);
        let income_text = format!("Доход: +{:.1} / 2 сек", director.income_rate);
        let spent_text = format!("Потрачено: {:.0}", director.total_points_spent);
        let spawn_text = format!("Призвано монстров: {}", director.total_mobs_spawned);
        let stats_text = format!("FPS: {} | Мобы на поле: {}", get_fps(), world.mobs.len());
        let cam_info = "WASD: Камера | Колесо: Зум | [C]: Босс";

        fonts.draw_text(&points_text, 25.0, 38.0, 22.0, GOLD);
        fonts.draw_text(&income_text, 25.0, 62.0, 17.0, GREEN);
        fonts.draw_text(&spent_text, 25.0, 84.0, 15.0, LIGHTGRAY);
        fonts.draw_text(&spawn_text, 25.0, 104.0, 15.0, SKYBLUE);
        fonts.draw_text(&stats_text, 25.0, 125.0, 14.0, WHITE);
        fonts.draw_text(cam_info, 25.0, 148.0, 12.0, YELLOW);

        // 6. Подсказка для правой кнопки мыши (ПКМ — Алтарь)
        let rmb_text = format!("ПКМ: Построить Алтарь Крови ({:.0} душ)", BLOOD_ALTAR_COST);
        let rmb_color = if director.points >= BLOOD_ALTAR_COST { PURPLE } else { GRAY };
        fonts.draw_text(&rmb_text, 15.0, 185.0, 15.0, rmb_color);

        // 7. Тактическая миникарта (справа вверху)
        draw_minimap(world, cam_pos, cam_zoom, fonts);

        // 8. Всплывающий текст урона и меток (в экранных координатах, всегда четко и прямо)
        for text in &world.floating_texts {
            let alpha = (text.lifetime / 0.8).clamp(0.0, 1.0);
            let mut color = text.color;
            color.a = alpha;
            let font_size = 16.0 * text.scale;

            let sx = (text.pos.x - cam_pos.x) * cam_zoom + screen_w / 2.0;
            let sy = (text.pos.y - cam_pos.y) * cam_zoom + screen_h / 2.0;

            if sx >= -100.0 && sx <= screen_w + 100.0 && sy >= -50.0 && sy <= screen_h + 50.0 {
                let tw = fonts.measure_text(&text.text, font_size).width;
                fonts.draw_text(&text.text, sx - tw / 2.0, sy, font_size, color);
            }
        }

        // 9. Панель призыва отрядов (снизу экрана с превью монстров)
        let btn_width = 195.0;
        let btn_height = 74.0;
        let total_w = 5.0 * btn_width + 4.0 * 12.0;
        let start_x = (screen_w - total_w) / 2.0;
        let btn_y = screen_h - 88.0;

        for i in 0..5 {
            let tier = (i + 1) as u8;
            let config = get_squad_config(tier);
            let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;
            let cd = director.squad_cooldowns[i];
            let can_afford = director.points >= config.cost;

            let bx = start_x + (i as f32) * (btn_width + 12.0);

            let bg_color = if !is_unlocked {
                Color::new(0.14, 0.14, 0.17, 0.92)
            } else if cd > 0.0 {
                Color::new(0.24, 0.18, 0.18, 0.92)
            } else if !can_afford {
                Color::new(0.28, 0.18, 0.18, 0.92)
            } else {
                Color::new(0.14, 0.32, 0.20, 0.95)
            };

            let border_color = if is_unlocked && can_afford && cd <= 0.0 {
                GREEN
            } else if is_unlocked {
                WHITE
            } else {
                GRAY
            };

            draw_rectangle(bx, btn_y, btn_width, btn_height, bg_color);
            draw_rectangle_lines(bx, btn_y, btn_width, btn_height, 2.0, border_color);

            // Иконка монстра справа на кнопке
            if let Some(mob_tex) = textures.get_mob_texture(tier, 0) {
                let preview_size = 40.0;
                let (pw, ph) = if tier == 2 { (preview_size * 1.3, preview_size * 0.65) } else { (preview_size, preview_size) };
                let px = bx + btn_width - pw - 6.0;
                let py = btn_y + (btn_height - ph) / 2.0;
                draw_texture_ex(
                    mob_tex,
                    px,
                    py,
                    if is_unlocked { WHITE } else { Color::new(0.4, 0.4, 0.4, 0.6) },
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(pw, ph)),
                        ..Default::default()
                    },
                );
            }

            // Название отряда и горячая клавиша
            let header = format!("[{}] {}", i + 1, config.name);
            fonts.draw_text(&header, bx + 6.0, btn_y + 20.0, 14.0, WHITE);

            if !is_unlocked {
                let lock_text = format!("Нужно: {:.0}", config.unlock_spent_threshold);
                fonts.draw_text(&lock_text, bx + 6.0, btn_y + 40.0, 13.0, RED);

                let prog = (director.total_points_spent / config.unlock_spent_threshold).clamp(0.0, 1.0);
                draw_rectangle(bx + 6.0, btn_y + 52.0, (btn_width - 56.0) * prog, 6.0, RED);
            } else if cd > 0.0 {
                let cd_text = format!("Откат: {:.1}с", cd);
                fonts.draw_text(&cd_text, bx + 6.0, btn_y + 40.0, 14.0, YELLOW);

                let cd_ratio = (cd / config.cooldown).clamp(0.0, 1.0);
                draw_rectangle(bx + 6.0, btn_y + 52.0, (btn_width - 56.0) * (1.0 - cd_ratio), 6.0, YELLOW);
            } else {
                let cost_text = format!("Цена: {:.0} ({} шт)", config.cost, config.count);
                let cost_color = if can_afford { GOLD } else { GRAY };
                fonts.draw_text(&cost_text, bx + 6.0, btn_y + 40.0, 13.0, cost_color);

                let grow_text = format!("+ {:.2}/2c доход", config.cost * 0.05);
                fonts.draw_text(&grow_text, bx + 6.0, btn_y + 58.0, 12.0, GREEN);
            }
        }
    }
}

/// Отрисовка тактической миникарты
fn draw_minimap(world: &World, cam_pos: Vec2, cam_zoom: f32, fonts: &GameFonts) {
    let screen_w = screen_width();
    let mini_w = 200.0;
    let mini_h = 133.0; // Соотношение 3600 / 2400 = 1.5
    let mini_x = screen_w - mini_w - 15.0;
    let mini_y = 15.0;

    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;

    // Фон миникарты
    draw_rectangle(mini_x, mini_y, mini_w, mini_h, Color::new(0.05, 0.05, 0.08, 0.88));
    draw_rectangle_lines(mini_x, mini_y, mini_w, mini_h, 2.0, Color::new(0.6, 0.2, 0.2, 0.9));

    let to_mini = |p: Vec2| -> Vec2 {
        Vec2::new(
            mini_x + (p.x / map_w) * mini_w,
            mini_y + (p.y / map_h) * mini_h,
        )
    };

    // Точки мобов
    for mob in &world.mobs {
        let mp = to_mini(mob.pos);
        draw_circle(mp.x, mp.y, 1.2, Color::new(0.8, 0.3, 0.3, 0.8));
    }

    // Алтари
    for st in &world.structures {
        let sp = to_mini(st.pos);
        draw_circle(sp.x, sp.y, 3.5, PURPLE);
    }

    // Босс
    let bp = to_mini(world.boss_pos);
    draw_circle(bp.x, bp.y, 4.5, YELLOW);
    draw_circle_lines(bp.x, bp.y, 4.5, 1.5, WHITE);

    // Рамка текущего обзора камеры
    let view_w = (screen_w / cam_zoom) * (mini_w / map_w);
    let view_h = (screen_height() / cam_zoom) * (mini_h / map_h);
    let view_center = to_mini(cam_pos);
    draw_rectangle_lines(
        view_center.x - view_w / 2.0,
        view_center.y - view_h / 2.0,
        view_w,
        view_h,
        1.5,
        WHITE,
    );

    // Подпись
    fonts.draw_text("КАРТА АРЕНЫ", mini_x + 5.0, mini_y + 14.0, 11.0, LIGHTGRAY);
}
