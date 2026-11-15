use macroquad::prelude::*;
use crate::content::mobs::get_squad_config;
use crate::content::structures::BLOOD_ALTAR_COST;
use crate::core::time::GameClock;
use crate::ecs::world::World;

pub fn render_ui(world: &World, clock: &GameClock) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // 1. Верхний таймер 30:00 обратного отсчета
    let timer_text = clock.formatted_remaining();
    let timer_font_size = 34.0;
    let timer_w = measure_text(&timer_text, None, timer_font_size as u16, 1.0).width;
    draw_text(&timer_text, (screen_w - timer_w) / 2.0, 38.0, timer_font_size, WHITE);

    // 2. Полоска здоровья Босса и Опыт (сверху под таймером)
    let bar_w = 460.0;
    let bar_h = 20.0;
    let bar_x = (screen_w - bar_w) / 2.0;
    let bar_y = 48.0;

    let hp_ratio = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0);
    draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.15, 0.15, 0.2, 0.9));
    draw_rectangle(bar_x, bar_y, bar_w * hp_ratio, bar_h, Color::new(0.9, 0.2, 0.2, 1.0));
    draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 2.0, WHITE);

    let hp_text = format!(
        "{:.0} / {:.0} HP  (Босс ур. {}) | Броня: {:.0}",
        world.boss_health.current,
        world.boss_health.max,
        world.boss_inventory.level,
        world.boss_resistances.flat_armor
    );
    let hp_text_w = measure_text(&hp_text, None, 14, 1.0).width;
    draw_text(&hp_text, (screen_w - hp_text_w) / 2.0, bar_y + 15.0, 14.0, WHITE);

    // Полоска опыта босса
    let xp_ratio = (world.boss_inventory.current_xp / world.boss_inventory.xp_to_next_level).clamp(0.0, 1.0);
    draw_rectangle(bar_x, bar_y + bar_h + 2.0, bar_w, 4.0, Color::new(0.2, 0.2, 0.3, 0.8));
    draw_rectangle(bar_x, bar_y + bar_h + 2.0, bar_w * xp_ratio, 4.0, SKYBLUE);

    // 3. Инвентарь Босса (6 Оружий + 6 Пассивок) под полоской ХП
    let inv_y = bar_y + bar_h + 10.0;
    let slot_w = 34.0;
    let slot_h = 24.0;
    let total_slots_w = 12.0 * (slot_w + 4.0);
    let start_inv_x = (screen_w - total_slots_w) / 2.0;

    // 6 слотов оружия
    for i in 0..6 {
        let sx = start_inv_x + (i as f32) * (slot_w + 4.0);
        draw_rectangle(sx, inv_y, slot_w, slot_h, Color::new(0.1, 0.1, 0.15, 0.8));
        draw_rectangle_lines(sx, inv_y, slot_w, slot_h, 1.0, GRAY);

        if let Some(w) = world.boss_inventory.weapons.get(i) {
            let color = if w.is_evolved { GOLD } else { SKYBLUE };
            draw_rectangle(sx + 2.0, inv_y + 2.0, slot_w - 4.0, slot_h - 4.0, color);
            let lvl_text = format!("{}", w.level);
            draw_text(&lvl_text, sx + 4.0, inv_y + 18.0, 14.0, BLACK);
        }
    }

    // Разделитель
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
        draw_rectangle(sx, inv_y, slot_w, slot_h, Color::new(0.1, 0.1, 0.15, 0.8));
        draw_rectangle_lines(sx, inv_y, slot_w, slot_h, 1.0, GRAY);

        if let Some(p) = world.boss_inventory.passives.get(i) {
            draw_rectangle(sx + 2.0, inv_y + 2.0, slot_w - 4.0, slot_h - 4.0, VIOLET);
            let lvl_text = format!("{}", p.level);
            draw_text(&lvl_text, sx + 4.0, inv_y + 18.0, 14.0, WHITE);
        }
    }

    // 4. Баннер Level-Up / Эволюции Босса
    if world.boss_inventory.perk_announcement_timer > 0.0 {
        if let Some(ref perk_text) = world.boss_inventory.last_chosen_perk {
            let banner_w = 420.0;
            let banner_h = 32.0;
            let bx = (screen_w - banner_w) / 2.0;
            let by = inv_y + slot_h + 8.0;

            draw_rectangle(bx, by, banner_w, banner_h, Color::new(0.1, 0.1, 0.15, 0.95));
            draw_rectangle_lines(bx, by, banner_w, banner_h, 2.0, GOLD);

            let tw = measure_text(perk_text, None, 18, 1.0).width;
            draw_text(perk_text, (screen_w - tw) / 2.0, by + 22.0, 18.0, GOLD);
        }
    }

    // 5. Панель Директора (слева вверху)
    if let Some(director) = world.directors.first() {
        draw_rectangle(15.0, 15.0, 260.0, 125.0, Color::new(0.08, 0.08, 0.12, 0.9));
        draw_rectangle_lines(15.0, 15.0, 260.0, 125.0, 2.0, DARKGRAY);

        let points_text = format!("Очки Душ: {:.0}", director.points);
        let income_text = format!("Доход: +{:.1} / 2 сек", director.income_rate);
        let spent_text = format!("Потрачено: {:.0}", director.total_points_spent);
        let spawn_text = format!("Призвано мобов: {}", director.total_mobs_spawned);
        let stats_text = format!("FPS: {} | Активно: {}", get_fps(), world.mobs.len());

        draw_text(&points_text, 25.0, 38.0, 22.0, GOLD);
        draw_text(&income_text, 25.0, 62.0, 17.0, GREEN);
        draw_text(&spent_text, 25.0, 84.0, 15.0, LIGHTGRAY);
        draw_text(&spawn_text, 25.0, 104.0, 15.0, SKYBLUE);
        draw_text(&stats_text, 25.0, 125.0, 14.0, WHITE);

        // 6. Подсказка для правой кнопки мыши (ПКМ — Алтарь)
        let rmb_text = format!("ПКМ: Построить Алтарь Крови ({:.0} душ)", BLOOD_ALTAR_COST);
        let rmb_color = if director.points >= BLOOD_ALTAR_COST { PURPLE } else { GRAY };
        draw_text(&rmb_text, 15.0, 160.0, 15.0, rmb_color);

        // 7. Панель призыва отрядов (снизу экрана)
        let btn_width = 175.0;
        let btn_height = 68.0;
        let total_w = 5.0 * btn_width + 4.0 * 12.0;
        let start_x = (screen_w - total_w) / 2.0;
        let btn_y = screen_h - 85.0;

        for i in 0..5 {
            let tier = (i + 1) as u8;
            let config = get_squad_config(tier);
            let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;
            let cd = director.squad_cooldowns[i];
            let can_afford = director.points >= config.cost;

            let bx = start_x + (i as f32) * (btn_width + 12.0);

            let bg_color = if !is_unlocked {
                Color::new(0.15, 0.15, 0.18, 0.9) // Закрыто
            } else if cd > 0.0 {
                Color::new(0.25, 0.2, 0.2, 0.9) // КД
            } else if !can_afford {
                Color::new(0.28, 0.18, 0.18, 0.9) // Нет очков
            } else {
                Color::new(0.15, 0.35, 0.2, 0.95) // Готово
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

            // Название отряда и горячая клавиша
            let header = format!("[{}] {}", i + 1, config.name);
            draw_text(&header, bx + 6.0, btn_y + 20.0, 15.0, WHITE);

            if !is_unlocked {
                let lock_text = format!("Нужно: {:.0} потрачено", config.unlock_spent_threshold);
                draw_text(&lock_text, bx + 6.0, btn_y + 40.0, 13.0, RED);

                // Полоска прогресса открытия
                let prog = (director.total_points_spent / config.unlock_spent_threshold).clamp(0.0, 1.0);
                draw_rectangle(bx + 6.0, btn_y + 48.0, (btn_width - 12.0) * prog, 6.0, RED);
            } else if cd > 0.0 {
                let cd_text = format!("Перезарядка: {:.1}с", cd);
                draw_text(&cd_text, bx + 6.0, btn_y + 40.0, 14.0, YELLOW);

                // Полоска прогресса кулдауна
                let cd_ratio = (cd / config.cooldown).clamp(0.0, 1.0);
                draw_rectangle(bx + 6.0, btn_y + 48.0, (btn_width - 12.0) * (1.0 - cd_ratio), 6.0, YELLOW);
            } else {
                let cost_text = format!("Цена: {:.0} душ ({} шт)", config.cost, config.count);
                let cost_color = if can_afford { GOLD } else { GRAY };
                draw_text(&cost_text, bx + 6.0, btn_y + 40.0, 14.0, cost_color);

                let grow_text = format!("+ {:.2}/2c доход", config.cost * 0.05);
                draw_text(&grow_text, bx + 6.0, btn_y + 58.0, 12.0, GREEN);
            }
        }
    }
}
