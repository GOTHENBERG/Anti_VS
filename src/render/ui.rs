use macroquad::prelude::*;
use crate::core::time::GameClock;
use crate::content::mobs::get_squad_config;
use crate::ecs::world::World;

pub fn render_ui(world: &World, clock: &GameClock) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // 1. Верхний таймер 30:00 (по центру)
    let timer_text = clock.formatted_remaining();
    let timer_font_size = 32.0;
    let timer_width = measure_text(&timer_text, None, timer_font_size as u16, 1.0).width;
    draw_text(&timer_text, (screen_w - timer_width) / 2.0, 40.0, timer_font_size, WHITE);

    // 2. Полоска здоровья Босса (сверху)
    let hp_ratio = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0);
    let bar_width = 400.0;
    let bar_height = 20.0;
    let bar_x = (screen_w - bar_width) / 2.0;
    let bar_y = 52.0;

    draw_rectangle(bar_x, bar_y, bar_width, bar_height, DARKGRAY);
    draw_rectangle(bar_x, bar_y, bar_width * hp_ratio, bar_height, RED);
    draw_rectangle_lines(bar_x, bar_y, bar_width, bar_height, 2.0, WHITE);

    let hp_text = format!("{:.0} / {:.0} HP  (Lvl {})", world.boss_health.current, world.boss_health.max, world.boss_inventory.level);
    let hp_text_width = measure_text(&hp_text, None, 16, 1.0).width;
    draw_text(&hp_text, (screen_w - hp_text_width) / 2.0, bar_y + 15.0, 16.0, WHITE);

    // 3. Панель Директора (слева вверху)
    if let Some(director) = world.directors.first() {
        let points_text = format!("Очки Душ: {:.0}", director.points);
        let income_text = format!("Доход: +{:.1} каждые 2 сек", director.income_rate);
        let spent_text = format!("Потрачено: {:.0}", director.total_points_spent);

        draw_text(&points_text, 20.0, 30.0, 24.0, GOLD);
        draw_text(&income_text, 20.0, 55.0, 18.0, GREEN);
        draw_text(&spent_text, 20.0, 75.0, 16.0, LIGHTGRAY);

        // 4. Панель призыва отрядов (снизу экрана)
        let btn_width = 160.0;
        let btn_height = 55.0;
        let total_w = 5.0 * btn_width + 4.0 * 15.0;
        let start_x = (screen_w - total_w) / 2.0;
        let btn_y = screen_h - 75.0;

        let names = ["1: Скелеты", "2: Гончие", "3: Культисты", "4: Големы", "5: Камикадзе"];

        for i in 0..5 {
            let tier = (i + 1) as u8;
            let config = get_squad_config(tier);
            let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;
            let cd = director.squad_cooldowns[i];
            let can_afford = director.points >= config.cost;

            let bx = start_x + (i as f32) * (btn_width + 15.0);

            let bg_color = if !is_unlocked {
                Color::new(0.2, 0.2, 0.2, 0.8) // Закрыто
            } else if cd > 0.0 || !can_afford {
                Color::new(0.35, 0.2, 0.2, 0.8) // Кулдаун или нет денег
            } else {
                Color::new(0.2, 0.5, 0.3, 0.9) // Готово к вызову
            };

            draw_rectangle(bx, btn_y, btn_width, btn_height, bg_color);
            draw_rectangle_lines(bx, btn_y, btn_width, btn_height, 2.0, if is_unlocked { WHITE } else { GRAY });

            draw_text(names[i], bx + 8.0, btn_y + 20.0, 16.0, WHITE);

            if !is_unlocked {
                let lock_text = format!("Нужно: {:.0}", config.unlock_spent_threshold);
                draw_text(&lock_text, bx + 8.0, btn_y + 40.0, 14.0, RED);
            } else if cd > 0.0 {
                let cd_text = format!("КД: {:.1}c", cd);
                draw_text(&cd_text, bx + 8.0, btn_y + 40.0, 14.0, YELLOW);
            } else {
                let cost_text = format!("Цена: {:.0}", config.cost);
                draw_text(&cost_text, bx + 8.0, btn_y + 40.0, 14.0, GOLD);
            }
        }
    }
}
