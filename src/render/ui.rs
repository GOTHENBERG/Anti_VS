use macroquad::prelude::*;
use crate::content::mobs::get_squad_config;
use crate::content::passives::get_passive_def;
use crate::content::structures::BLOOD_ALTAR_COST;
use crate::content::weapons::get_weapon_def;
use crate::core::time::GameClock;
use crate::ecs::components::inventory::{PassiveSlot, PassiveType, WeaponSlot, WeaponType};
use crate::ecs::world::World;
use crate::render::font::GameFonts;
use crate::render::textures::GameTextures;

/// Результат взаимодействия с экранным интерфейсом
#[derive(Default, Debug)]
pub struct UiInteraction {
    pub open_dossier_tier: Option<u8>,
    pub direct_summon_tier: Option<u8>,
    pub clicked_pause: bool,
    pub clicked_codex: bool,
    pub is_mouse_over_ui: bool,
    pub map_click_world_pos: Option<Vec2>,
}

enum HoverTooltip {
    Weapon(WeaponSlot, f32, f32),
    Passive(PassiveSlot, f32, f32),
    WorldEntity(Vec2, Vec<(String, Color)>),
    AltarWidget(f32, f32),
}

pub fn render_ui(
    world: &World,
    clock: &GameClock,
    fonts: &GameFonts,
    cam_pos: Vec2,
    cam_zoom: f32,
    textures: &GameTextures,
) -> UiInteraction {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let (mx, my) = mouse_position();
    let mut hovered_tooltip: Option<HoverTooltip> = None;
    let mut interaction = UiInteraction::default();

    // 1. Верхний таймер 17:00 обратного отсчета
    let timer_text = clock.formatted_remaining();
    let timer_font_size = 36.0;
    let timer_w = fonts.measure_heading(&timer_text, timer_font_size).width;
    fonts.draw_heading(&timer_text, (screen_w - timer_w) / 2.0, 38.0, timer_font_size, WHITE);

    // 2. Полоска здоровья Босса и Опыт (сверху под таймером)
    let bar_w = 480.0;
    let bar_h = 22.0;
    let bar_x = (screen_w - bar_w) / 2.0;
    let bar_y = 50.0;

    let hp_ratio = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0);
    draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.12, 0.12, 0.18, 0.95));
    draw_rectangle(bar_x, bar_y, bar_w * hp_ratio, bar_h, Color::new(0.9, 0.2, 0.2, 1.0));
    draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 2.0, WHITE);

    let hp_text = format!(
        "{:.0} / {:.0} HP  (Босс ур. {}) | Броня: {:.1}",
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

    if mx >= start_inv_x && mx <= start_inv_x + total_slots_w && my >= inv_y && my <= inv_y + slot_h {
        interaction.is_mouse_over_ui = true;
    }

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

            if mx >= sx && mx <= sx + slot_w && my >= inv_y && my <= inv_y + slot_h {
                hovered_tooltip = Some(HoverTooltip::Weapon(w.clone(), sx, inv_y));
            }
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

            if mx >= sx && mx <= sx + slot_w && my >= inv_y && my <= inv_y + slot_h {
                hovered_tooltip = Some(HoverTooltip::Passive(p.clone(), sx, inv_y));
            }
        }
    }

    // 4. Баннер Level-Up / Эволюции Босса
    if world.boss_inventory.perk_announcement_timer > 0.0 {
        if let Some(ref perk_text) = world.boss_inventory.last_chosen_perk {
            let banner_w = 480.0;
            let banner_h = 36.0;
            let bx = (screen_w - banner_w) / 2.0;
            let by = inv_y + slot_h + 8.0;

            draw_rectangle(bx, by, banner_w, banner_h, Color::new(0.08, 0.08, 0.14, 0.96));
            draw_rectangle_lines(bx, by, banner_w, banner_h, 2.0, GOLD);

            let tw = fonts.measure_heading(perk_text, 18.0).width;
            fonts.draw_heading(perk_text, (screen_w - tw) / 2.0, by + 24.0, 18.0, GOLD);
        }
    }

    // 5. Верхние кнопки: Кодекс (слева) и Пауза (справа)
    // 5.1 Кнопка Кодекса [ 📖 Кодекс (I) ]
    let codex_btn_w = 145.0;
    let codex_btn_h = 34.0;
    let codex_btn_x = 20.0;
    let codex_btn_y = 14.0;
    let codex_hover = mx >= codex_btn_x && mx <= codex_btn_x + codex_btn_w && my >= codex_btn_y && my <= codex_btn_y + codex_btn_h;
    if codex_hover {
        interaction.is_mouse_over_ui = true;
    }
    draw_rectangle(codex_btn_x, codex_btn_y, codex_btn_w, codex_btn_h, if codex_hover { Color::new(0.24, 0.16, 0.35, 1.0) } else { Color::new(0.12, 0.10, 0.18, 0.95) });
    draw_rectangle_lines(codex_btn_x, codex_btn_y, codex_btn_w, codex_btn_h, 1.5, if codex_hover { GOLD } else { GRAY });
    fonts.draw_heading("📖 Кодекс [I]", codex_btn_x + 14.0, codex_btn_y + 22.0, 14.0, if codex_hover { GOLD } else { WHITE });

    if is_mouse_button_pressed(MouseButton::Left) && codex_hover {
        interaction.clicked_codex = true;
    }

    // Подсказка навигации под кнопкой кодекса (в 2 компактные строки)
    fonts.draw_text("WASD: Камера | Колесо: Зум", 20.0, 62.0, 11.5, YELLOW);
    fonts.draw_text("[C]: Центр босса | ЛКМ: Двигать карту", 20.0, 78.0, 11.5, YELLOW);

    // 5.2 Кнопка Паузы [ ⏸ Пауза (Space) ]
    let pause_btn_w = 160.0;
    let pause_btn_h = 34.0;
    let pause_btn_x = screen_w - pause_btn_w - 20.0;
    let pause_btn_y = 14.0;
    let pause_hover = mx >= pause_btn_x && mx <= pause_btn_x + pause_btn_w && my >= pause_btn_y && my <= pause_btn_y + pause_btn_h;
    if pause_hover {
        interaction.is_mouse_over_ui = true;
    }
    draw_rectangle(pause_btn_x, pause_btn_y, pause_btn_w, pause_btn_h, if pause_hover { Color::new(0.35, 0.25, 0.12, 1.0) } else { Color::new(0.14, 0.12, 0.10, 0.95) });
    draw_rectangle_lines(pause_btn_x, pause_btn_y, pause_btn_w, pause_btn_h, 1.5, if pause_hover { GOLD } else { GRAY });
    fonts.draw_heading("⏸ Пауза [Space]", pause_btn_x + 14.0, pause_btn_y + 22.0, 14.0, if pause_hover { GOLD } else { WHITE });

    if is_mouse_button_pressed(MouseButton::Left) && pause_hover {
        interaction.clicked_pause = true;
    }

    // 6. ТАКТИЧЕСКАЯ МИНИКАРТА (Перемещена в левую нижнюю часть экрана)
    draw_minimap(world, cam_pos, cam_zoom, fonts, &mut interaction);

    // 7. ПАНЕЛЬ ДИРЕКТОРА: ОЧКИ ДУШ, ИНКАМ И АЛТАРЬ (Перемещены в правую нижнюю часть экрана)
    if let Some(director) = world.directors.first() {
        let panel_w = 240.0;
        let panel_h = 135.0;
        let panel_x = screen_w - panel_w - 20.0;
        let panel_y = screen_h - panel_h - 20.0;

        if mx >= panel_x && mx <= panel_x + panel_w && my >= panel_y && my <= panel_y + panel_h {
            interaction.is_mouse_over_ui = true;
        }

        draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.08, 0.08, 0.12, 0.94));
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 2.0, DARKGRAY);

        let altar_count = world.structures.len();
        let total_income = director.income_rate + (altar_count as f32 * 3.0);

        let points_text = format!("Очки Душ: {:.0}", director.points);
        let income_text = format!("Доход: +{:.1} / 2с (Алт: {})", total_income, altar_count);
        let spent_text = format!("Потрачено: {:.0} душ", director.total_points_spent);
        let spawn_text = format!("Призвано: {} мобов", director.total_mobs_spawned);
        let stats_text = format!("FPS: {} | Мобы на поле: {}", get_fps(), world.mobs.len());

        fonts.draw_heading(&points_text, panel_x + 12.0, panel_y + 24.0, 20.0, GOLD);
        fonts.draw_text(&income_text, panel_x + 12.0, panel_y + 46.0, 13.0, GREEN);
        fonts.draw_text(&spent_text, panel_x + 12.0, panel_y + 68.0, 12.0, LIGHTGRAY);
        fonts.draw_text(&spawn_text, panel_x + 12.0, panel_y + 90.0, 12.0, SKYBLUE);
        fonts.draw_text(&stats_text, panel_x + 12.0, panel_y + 112.0, 12.0, WHITE);

        // 7.1 Виджет Алтаря Крови (прямо над панелью душ)
        let aw_x = panel_x;
        let aw_w = panel_w;
        let aw_h = 76.0;
        let aw_y = panel_y - aw_h - 10.0;

        if mx >= aw_x && mx <= aw_x + aw_w && my >= aw_y && my <= aw_y + aw_h {
            interaction.is_mouse_over_ui = true;
            hovered_tooltip = Some(HoverTooltip::AltarWidget(aw_x, aw_y));
        }

        let on_cooldown = director.altar_cooldown > 0.0;
        let can_afford_altar = director.points >= BLOOD_ALTAR_COST;

        let aw_border = if on_cooldown {
            Color::new(0.9, 0.25, 0.25, 0.95)
        } else if can_afford_altar {
            PURPLE
        } else {
            GRAY
        };

        draw_rectangle(aw_x, aw_y, aw_w, aw_h, Color::new(0.08, 0.08, 0.14, 0.95));
        draw_rectangle_lines(aw_x, aw_y, aw_w, aw_h, 2.0, aw_border);

        if let Some(altar_tex) = textures.altar.as_ref() {
            let icon_w = 32.0;
            let icon_h = 50.0;
            let icon_tint = if on_cooldown { Color::new(0.8, 0.4, 0.4, 0.8) } else { WHITE };
            draw_texture_ex(
                altar_tex,
                aw_x + 8.0,
                aw_y + (aw_h - icon_h) / 2.0,
                icon_tint,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(icon_w, icon_h)),
                    ..Default::default()
                },
            );
        } else {
            draw_circle(aw_x + 24.0, aw_y + 38.0, 14.0, PURPLE);
        }

        let content_x = aw_x + 46.0;
        fonts.draw_heading("АЛТАРЬ КРОВИ [ПКМ]", content_x, aw_y + 20.0, 13.0, GOLD);

        if on_cooldown {
            let cd_text = format!("Перезарядка: {:.1}с / {:.0}с", director.altar_cooldown, director.altar_max_cooldown);
            fonts.draw_text(&cd_text, content_x, aw_y + 38.0, 11.5, Color::new(1.0, 0.35, 0.35, 1.0));

            let bar_w = 175.0;
            let bar_h = 6.0;
            let prog = (1.0 - director.altar_cooldown / director.altar_max_cooldown).clamp(0.0, 1.0);
            draw_rectangle(content_x, aw_y + 45.0, bar_w, bar_h, Color::new(0.2, 0.1, 0.1, 0.9));
            draw_rectangle(content_x, aw_y + 45.0, bar_w * prog, bar_h, ORANGE);
            draw_rectangle_lines(content_x, aw_y + 45.0, bar_w, bar_h, 1.0, GRAY);

            let count_text = format!("Активно: {} (+{:.0}/2с)", world.structures.len(), world.structures.len() as f32 * 3.0);
            fonts.draw_text(&count_text, content_x, aw_y + 64.0, 11.0, LIGHTGRAY);
        } else if can_afford_altar {
            fonts.draw_text("ГОТОВ К ПОСТРОЙКЕ [50 душ]", content_x, aw_y + 38.0, 12.0, GREEN);
            let bar_w = 175.0;
            draw_rectangle(content_x, aw_y + 45.0, bar_w, 6.0, GREEN);
            let count_text = format!("Активно: {} (+{:.0}/2с)", world.structures.len(), world.structures.len() as f32 * 3.0);
            fonts.draw_text(&count_text, content_x, aw_y + 64.0, 11.0, SKYBLUE);
        } else {
            fonts.draw_text("ГОТОВ (Не хватает: 50 душ)", content_x, aw_y + 38.0, 11.5, GRAY);
            let count_text = format!("Активно: {} (+{:.0}/2с)", world.structures.len(), world.structures.len() as f32 * 3.0);
            fonts.draw_text(&count_text, content_x, aw_y + 58.0, 11.0, LIGHTGRAY);
        }

        // 8. ПАНЕЛЬ ГРЕЙДОВ МОБОВ ДИРЕКТОРА [F, G, H] (Над карточками мобов)
        let g_btn_w = 236.0;
        let g_btn_h = 32.0;
        let g_total_w = 3.0 * g_btn_w + 2.0 * 10.0;
        let g_start_x = (screen_w - g_total_w) / 2.0;
        let card_h = 118.0;
        let cards_y = screen_h - card_h - 20.0;
        let g_y = cards_y - g_btn_h - 8.0;

        let grades = [
            ("F", "Ярость Крови", director.grade_fury, 100.0 * (director.grade_fury as f32 + 1.0), "+12% урон, +6% скор."),
            ("G", "Панцирь", director.grade_carapace, 120.0 * (director.grade_carapace as f32 + 1.0), "+2.5 брони, +5% рез."),
            ("H", "Чумные Споры", director.grade_pestilence, 150.0 * (director.grade_pestilence as f32 + 1.0), "25% шанс лужи яда"),
        ];

        for (i, (key, name, lvl, cost, desc)) in grades.iter().enumerate() {
            let gx = g_start_x + (i as f32) * (g_btn_w + 10.0);
            if mx >= gx && mx <= gx + g_btn_w && my >= g_y && my <= g_y + g_btn_h {
                interaction.is_mouse_over_ui = true;
            }

            let can_buy = director.points >= *cost;
            let bg = if can_buy { Color::new(0.20, 0.15, 0.30, 0.95) } else { Color::new(0.10, 0.10, 0.15, 0.90) };
            let border = if can_buy { PURPLE } else { GRAY };

            draw_rectangle(gx, g_y, g_btn_w, g_btn_h, bg);
            draw_rectangle_lines(gx, g_y, g_btn_w, g_btn_h, 1.5, border);

            let title = format!("[{}] {} (Ур. {}) - {:.0} душ", key, name, lvl, cost);
            fonts.draw_heading(&title, gx + 6.0, g_y + 14.0, 11.5, if can_buy { GOLD } else { LIGHTGRAY });
            fonts.draw_text(desc, gx + 6.0, g_y + 26.0, 10.5, SKYBLUE);
        }

        // 9. КАРТОЧКИ ПРИЗЫВА МОБОВ (Раскрытые Tarot-карты по центру снизу, без всплывающего тултипа при наведении)
        let card_w = 90.0;
        let card_gap = 6.0;
        let total_cards_w = 8.0 * card_w + 7.0 * card_gap; // 762px
        let cards_start_x = (screen_w - total_cards_w) / 2.0;

        for i in 0..8 {
            let tier = (i + 1) as u8;
            let config = get_squad_config(tier);
            let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;
            let can_afford = director.points >= config.cost;

            let cx = cards_start_x + (i as f32) * (card_w + card_gap);
            let cy = cards_y;

            let card_hover = mx >= cx && mx <= cx + card_w && my >= cy && my <= cy + card_h;
            if card_hover {
                interaction.is_mouse_over_ui = true;
            }

            let bg_color = if !is_unlocked {
                Color::new(0.09, 0.09, 0.12, 0.95)
            } else if can_afford {
                if card_hover { Color::new(0.16, 0.38, 0.22, 0.98) } else { Color::new(0.11, 0.28, 0.16, 0.95) }
            } else {
                Color::new(0.20, 0.12, 0.14, 0.95)
            };

            let border_color = if !is_unlocked {
                Color::new(0.35, 0.35, 0.40, 0.8)
            } else if card_hover {
                GOLD
            } else if can_afford {
                GREEN
            } else {
                Color::new(0.8, 0.3, 0.3, 0.9)
            };

            // Фон карты
            draw_rectangle(cx, cy, card_w, card_h, bg_color);
            draw_rectangle_lines(cx, cy, card_w, card_h, if card_hover { 2.5 } else { 1.5 }, border_color);

            // Верхняя плашка с горячей клавишей и римским номером тира
            let hotkey_badge = format!("[{}]", i + 1);
            let tier_romans = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII"];
            let tier_roman = tier_romans.get(i).unwrap_or(&"");

            fonts.draw_heading(&hotkey_badge, cx + 4.0, cy + 14.0, 11.5, if is_unlocked { GOLD } else { GRAY });
            let tr_w = fonts.measure_heading(tier_roman, 11.0).width;
            fonts.draw_heading(tier_roman, cx + card_w - tr_w - 4.0, cy + 14.0, 11.0, SKYBLUE);

            // Иконка / Спрайт моба
            if let Some(mob_tex) = textures.get_mob_texture(tier, 0) {
                let p_size = 36.0;
                let (pw, ph) = if tier == 2 { (p_size * 1.3, p_size * 0.65) } else { (p_size, p_size) };
                let px = cx + (card_w - pw) / 2.0;
                let py = cy + 20.0 + (p_size - ph) / 2.0;
                draw_texture_ex(
                    mob_tex,
                    px,
                    py,
                    if is_unlocked { WHITE } else { Color::new(0.35, 0.35, 0.35, 0.6) },
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(pw, ph)),
                        ..Default::default()
                    },
                );
            }

            // Компактное имя моба
            let short_name = match tier {
                1 => "Скелеты",
                2 => "Гончие",
                3 => "Культисты",
                4 => "Големы",
                5 => "Камикадзе",
                6 => "Демоны",
                7 => "Элементали",
                _ => "Жнецы",
            };
            let nw = fonts.measure_heading(short_name, 11.0).width;
            fonts.draw_heading(short_name, cx + (card_w - nw) / 2.0, cy + 68.0, 11.0, if is_unlocked { WHITE } else { LIGHTGRAY });

            // Стоимость душ
            let cost_str = format!("💀 {:.0}", config.cost);
            let cw = fonts.measure_heading(&cost_str, 12.0).width;
            let cost_col = if !is_unlocked { GRAY } else if can_afford { GOLD } else { Color::new(1.0, 0.4, 0.4, 1.0) };
            fonts.draw_heading(&cost_str, cx + (card_w - cw) / 2.0, cy + 84.0, 12.0, cost_col);

            // Подпись характеристики / роли
            let role_str = match tier {
                1 => "x8 бойцов",
                2 => "x6 скор.",
                3 => "Дальники",
                4 => "Танк 380",
                5 => "Взрыв 220",
                6 => "Телепорт",
                7 => "Магия огня",
                _ => "Титан коса",
            };
            let rw = fonts.measure_text(role_str, 10.0).width;
            fonts.draw_text(role_str, cx + (card_w - rw) / 2.0, cy + 98.0, 10.0, if is_unlocked { SKYBLUE } else { GRAY });

            // Нижняя плашка подсказки Досье
            if is_unlocked {
                let wiki_hint = "📖 Досье";
                let wh_w = fonts.measure_text(wiki_hint, 9.5).width;
                fonts.draw_text(wiki_hint, cx + (card_w - wh_w) / 2.0, cy + 112.0, 9.5, if card_hover { YELLOW } else { LIGHTGRAY });
            } else {
                let lock_str = format!("🔒 {:.0}", config.unlock_spent_threshold);
                let lw = fonts.measure_text(&lock_str, 9.5).width;
                fonts.draw_text(&lock_str, cx + (card_w - lw) / 2.0, cy + 112.0, 9.5, RED);
            }

            // Клик по карте:
            // Левый клик -> открыть подробнейшее досье-википедию
            if is_mouse_button_pressed(MouseButton::Left) && card_hover {
                interaction.open_dossier_tier = Some(tier);
            }
            // Правый клик -> мгновенно призвать отряд, если открыт
            if is_mouse_button_pressed(MouseButton::Right) && card_hover && is_unlocked && can_afford {
                interaction.direct_summon_tier = Some(tier);
            }
        }
    }

    // 10. Всплывающий текст урона и меток в координатах экрана
    for text in &world.floating_texts {
        let alpha = (text.lifetime / 0.8).clamp(0.0, 1.0);
        let mut color = text.color;
        color.a = alpha;
        let font_size = 16.0 * text.scale;

        let sx = (text.pos.x - cam_pos.x) * cam_zoom + screen_w / 2.0;
        let sy = (text.pos.y - cam_pos.y) * cam_zoom + screen_h / 2.0;

        if sx >= -100.0 && sx <= screen_w + 100.0 && sy >= -50.0 && sy <= screen_h + 50.0 {
            let tw = fonts.measure_heading(&text.text, font_size).width;
            fonts.draw_heading(&text.text, sx - tw / 2.0, sy, font_size, color);
        }
    }

    // 11. Проверка наведения курсора на сущности игрового мира (Босс, Алтари, Мобы)
    if hovered_tooltip.is_none() && !interaction.is_mouse_over_ui {
        let world_mx = (mx - screen_w / 2.0) / cam_zoom + cam_pos.x;
        let world_my = (my - screen_h / 2.0) / cam_zoom + cam_pos.y;
        let world_mpos = Vec2::new(world_mx, world_my);

        // 11.1 Наведение на Босса
        if world_mpos.distance(world.boss_pos) < 36.0 {
            let state_str = if world.boss_still_time >= 1.2 {
                ("🧘 МЕДИТАЦИЯ ПОКОЯ (+2% HP/с, +30 XP/с)".to_string(), GREEN)
            } else if world.boss_dash_cooldown <= 0.0 {
                ("⚡ РЫВОК ГОТОВ (8-лучевой Dash)".to_string(), GOLD)
            } else {
                ("⚔️ АКТИВНЫЙ БОЙ (Кайтинг орд)".to_string(), SKYBLUE)
            };

            let ai_str = if let Some(ref model) = world.boss_model {
                let syn = model.total_synapses;
                let desc = if syn > 0 {
                    format!("🧠 ИИ: Глубокая нейросеть ({} синапсов, 36 сенсоров, Пок. {})", syn, model.generation)
                } else {
                    format!("🧠 ИИ: Обученная модель (Поколение {}, Фитнес {:.0})", model.generation, model.fitness)
                };
                (desc, GOLD)
            } else {
                ("⚙️ ИИ: Базовые потенциальные поля".to_string(), LIGHTGRAY)
            };

            let lines = vec![
                ("👑 СОЛО-БОСС (АВТОНОМНЫЙ ГЕРОЙ)".to_string(), YELLOW),
                (format!("Здоровье: {:.0} / {:.0} HP | Броня: {:.1}", world.boss_health.current, world.boss_health.max, world.boss_resistances.flat_armor), WHITE),
                (format!("Уровень: {} | Оружий: {}/6 | Пассивок: {}/6", world.boss_inventory.level, world.boss_inventory.weapons.len(), world.boss_inventory.passives.len()), LIGHTGRAY),
                state_str,
                ai_str,
            ];
            hovered_tooltip = Some(HoverTooltip::WorldEntity(Vec2::new(mx, my), lines));
        }

        // 11.2 Наведение на Алтарь Крови
        if hovered_tooltip.is_none() {
            for st in &world.structures {
                if world_mpos.distance(st.pos) < st.radius + 18.0 {
                    let lines = vec![
                        ("🏛️ АЛТАРЬ КРОВИ (Инфернальная структура)".to_string(), PURPLE),
                        (format!("Здоровье: {:.0} / {:.0} HP | Броня: {:.0}", st.health.current, st.health.max, st.resistances.flat_armor), WHITE),
                        ("+3.0 Очков Душ каждые 2 сек к доходу Директора".to_string(), GREEN),
                        ("Аура Крови: лечит союзных мобов на +35 HP каждые 3 сек в радиусе 180px".to_string(), SKYBLUE),
                        ("⚠️ РАЗРУШАЕМ: Босс атакует и уничтожает алтарь!".to_string(), ORANGE),
                    ];
                    hovered_tooltip = Some(HoverTooltip::WorldEntity(Vec2::new(mx, my), lines));
                    break;
                }
            }
        }

        // 11.3 Наведение на Моба
        if hovered_tooltip.is_none() {
            let nearby_indices = world.spatial_grid.query_nearby(world_mpos, 28.0);
            for idx in nearby_indices {
                if let Some(mob) = world.mobs.get(idx) {
                    if mob.pos.distance(world_mpos) < mob.radius + 12.0 {
                        let config = get_squad_config(mob.tier);
                        let mut lines = vec![
                            (format!("👹 {} (Тир {})", config.name, mob.tier), WHITE),
                            (format!("HP: {:.0} / {:.0} | Скорость: {:.0} | Броня: {:.0}", mob.health.current, mob.health.max, mob.speed, mob.resistances.flat_armor), LIGHTGRAY),
                        ];

                        if mob.status.burn_timer > 0.0 {
                            lines.push((format!("🔥 Горение: -{:.0} HP/сек", mob.status.burn_dps), ORANGE));
                        }
                        if mob.status.is_frozen() {
                            lines.push(("❄️ Заморозка: полностью обездвижен".to_string(), SKYBLUE));
                        } else if mob.status.slow_timer > 0.0 {
                            lines.push(("❄️ Замедление скорости на 50%".to_string(), SKYBLUE));
                        }
                        if mob.status.poison_timer > 0.0 {
                            lines.push((format!("🧪 Отравление: -{:.0} HP/сек", mob.status.poison_dps), LIME));
                        }

                        hovered_tooltip = Some(HoverTooltip::WorldEntity(Vec2::new(mx, my), lines));
                        break;
                    }
                }
            }
        }
    }

    // 12. Отрисовка активной карточки-подсказки поверх всех элементов
    if let Some(tooltip) = hovered_tooltip {
        match tooltip {
            HoverTooltip::Weapon(w, slot_x, slot_y) => {
                draw_weapon_tooltip(&w, slot_x, slot_y, screen_w, fonts);
            }
            HoverTooltip::Passive(p, slot_x, slot_y) => {
                draw_passive_tooltip(&p, slot_x, slot_y, screen_w, fonts);
            }
            HoverTooltip::WorldEntity(screen_pos, lines) => {
                draw_world_entity_tooltip(screen_pos, lines, screen_w, screen_h, fonts);
            }
            HoverTooltip::AltarWidget(aw_x, aw_y) => {
                draw_altar_widget_tooltip(aw_x, aw_y, screen_w, fonts, world);
            }
        }
    }

    interaction
}

/// Отрисовка тактической миникарты в левом нижнем углу с рамкой обзора, не выходящей за границы
fn draw_minimap(
    world: &World,
    cam_pos: Vec2,
    cam_zoom: f32,
    fonts: &GameFonts,
    interaction: &mut UiInteraction,
) {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let (mx, my) = mouse_position();

    let mini_w = 200.0;
    let mini_h = 133.0; // Соотношение 3600 / 2400 = 1.5
    let mini_x = 20.0;
    let mini_y = screen_h - mini_h - 20.0;

    let map_w = crate::core::MAP_WIDTH;
    let map_h = crate::core::MAP_HEIGHT;

    // Проверка попадания курсора на миникарту
    if mx >= mini_x && mx <= mini_x + mini_w && my >= mini_y && my <= mini_y + mini_h {
        interaction.is_mouse_over_ui = true;
        if is_mouse_button_pressed(MouseButton::Left) {
            let click_world_x = ((mx - mini_x) / mini_w) * map_w;
            let click_world_y = ((my - mini_y) / mini_h) * map_h;
            interaction.map_click_world_pos = Some(Vec2::new(click_world_x, click_world_y));
        }
    }

    // Фон миникарты
    draw_rectangle(mini_x, mini_y, mini_w, mini_h, Color::new(0.05, 0.05, 0.08, 0.92));
    draw_rectangle_lines(mini_x, mini_y, mini_w, mini_h, 2.0, Color::new(0.6, 0.2, 0.2, 0.95));

    let to_mini = |p: Vec2| -> Vec2 {
        Vec2::new(
            mini_x + (p.x / map_w) * mini_w,
            mini_y + (p.y / map_h) * mini_h,
        )
    };

    // Точки мобов
    for mob in &world.mobs {
        let mp = to_mini(mob.pos);
        draw_circle(mp.x, mp.y, 1.2, Color::new(0.85, 0.25, 0.25, 0.85));
    }

    // Алтари
    for st in &world.structures {
        let sp = to_mini(st.pos);
        draw_circle(sp.x, sp.y, 3.5, PURPLE);
        draw_circle_lines(sp.x, sp.y, 3.5, 1.0, WHITE);
    }

    // Босс
    let bp = to_mini(world.boss_pos);
    draw_circle(bp.x, bp.y, 4.5, YELLOW);
    draw_circle_lines(bp.x, bp.y, 4.5, 1.5, WHITE);

    // Рамка текущего обзора камеры (СТРОГО ОГРАНИЧЕНА ГРАНИЦАМИ МИНИКАРТЫ)
    let view_w = (screen_w / cam_zoom) * (mini_w / map_w);
    let view_h = (screen_height() / cam_zoom) * (mini_h / map_h);
    let view_center = to_mini(cam_pos);

    let raw_vx1 = view_center.x - view_w / 2.0;
    let raw_vy1 = view_center.y - view_h / 2.0;
    let raw_vx2 = view_center.x + view_w / 2.0;
    let raw_vy2 = view_center.y + view_h / 2.0;

    let vx1 = raw_vx1.clamp(mini_x, mini_x + mini_w);
    let vy1 = raw_vy1.clamp(mini_y, mini_y + mini_h);
    let vx2 = raw_vx2.clamp(mini_x, mini_x + mini_w);
    let vy2 = raw_vy2.clamp(mini_y, mini_y + mini_h);

    let clamped_w = (vx2 - vx1).max(0.0);
    let clamped_h = (vy2 - vy1).max(0.0);

    // Радарная полупрозрачная подсветка зоны
    draw_rectangle(vx1, vy1, clamped_w, clamped_h, Color::new(1.0, 0.85, 0.2, 0.12));
    draw_rectangle_lines(vx1, vy1, clamped_w, clamped_h, 1.5, Color::new(1.0, 0.85, 0.3, 0.85));

    // Стилизованные тактические золотые уголки рамки камеры
    let corner_len = 7.0f32.min(clamped_w / 2.0).min(clamped_h / 2.0);
    if corner_len > 1.5 {
        // Верхний левый
        draw_line(vx1, vy1, vx1 + corner_len, vy1, 2.0, GOLD);
        draw_line(vx1, vy1, vx1, vy1 + corner_len, 2.0, GOLD);
        // Верхний правый
        draw_line(vx2, vy1, vx2 - corner_len, vy1, 2.0, GOLD);
        draw_line(vx2, vy1, vx2 - corner_len, vy1 + corner_len, 2.0, GOLD);
        // Нижний левый
        draw_line(vx1, vy2, vx1 + corner_len, vy2, 2.0, GOLD);
        draw_line(vx1, vy2, vx1, vy2 - corner_len, 2.0, GOLD);
        // Нижний правый
        draw_line(vx2, vy2, vx2 - corner_len, vy2, 2.0, GOLD);
        draw_line(vx2, vy2, vx2, vy2 - corner_len, 2.0, GOLD);
    }

    // Центральное перекрестье камеры
    if clamped_w > 12.0 && clamped_h > 12.0 {
        let cx = (vx1 + vx2) / 2.0;
        let cy = (vy1 + vy2) / 2.0;
        draw_line(cx - 3.5, cy, cx + 3.5, cy, 1.0, Color::new(1.0, 1.0, 1.0, 0.65));
        draw_line(cx, cy - 3.5, cx, cy + 3.5, 1.0, Color::new(1.0, 1.0, 1.0, 0.65));
    }

    // Подпись
    fonts.draw_text("КАРТА АРЕНЫ (ЛКМ)", mini_x + 6.0, mini_y + 14.0, 11.0, LIGHTGRAY);
}

/// Всплывающая подсказка для виджета Алтаря Крови
fn draw_altar_widget_tooltip(aw_x: f32, aw_y: f32, screen_w: f32, fonts: &GameFonts, world: &World) {
    let card_w = 360.0;
    let card_h = 175.0;
    let card_x = (aw_x - card_w - 12.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = aw_y;

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.06, 0.06, 0.12, 0.96));
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, PURPLE);

    fonts.draw_heading("🏛️ АЛТАРЬ КРОВИ (Инфернальная база)", card_x + 12.0, card_y + 22.0, 15.0, GOLD);
    fonts.draw_text("• Постройка: Клик ПКМ на свободной точке арены", card_x + 12.0, card_y + 44.0, 12.0, WHITE);
    let cost_str = format!("• Стоимость: {:.0} Душ | Кулдаун: 60 сек (Без лимита шт)", BLOOD_ALTAR_COST);
    fonts.draw_text(&cost_str, card_x + 12.0, card_y + 64.0, 12.0, YELLOW);
    let inc_str = format!("• Доход: +3.0 Очков Душ каждые 2 сек за каждый алтарь (Текущий: +{:.0}/2с)", world.structures.len() as f32 * 3.0);
    fonts.draw_text(&inc_str, card_x + 12.0, card_y + 84.0, 12.0, GREEN);
    fonts.draw_text("• Аура Крови: Восстанавливает +35 HP союзникам в радиусе 180px", card_x + 12.0, card_y + 104.0, 12.0, SKYBLUE);
    fonts.draw_text("• Здоровье: 650 HP (Броня: 12, уязвим к святому урону -60%)", card_x + 12.0, card_y + 124.0, 12.0, LIGHTGRAY);
    fonts.draw_text("⚠️ ОПАСНОСТЬ: Босс активно ищет и штурмует алтари (+150 XP)", card_x + 12.0, card_y + 148.0, 12.0, ORANGE);
}

/// Всплывающая подсказка для оружия Босса
fn draw_weapon_tooltip(w: &WeaponSlot, sx: f32, sy: f32, screen_w: f32, fonts: &GameFonts) {
    let def = get_weapon_def(w.kind);
    let card_w = 340.0;
    let card_h = 135.0;
    let card_x = (sx + 19.0 - card_w / 2.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = sy + 36.0;

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.06, 0.08, 0.14, 0.96));
    let border_color = if w.is_evolved { GOLD } else { SKYBLUE };
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, border_color);

    let title = format!("{} [Ур. {}/8]", def.name, w.level);
    fonts.draw_heading(&title, card_x + 12.0, card_y + 22.0, 16.0, border_color);

    let dmg_type_str = match def.damage_type {
        crate::ecs::components::combat::DamageType::Physical => "Физический",
        crate::ecs::components::combat::DamageType::Fire => "Огненный",
        crate::ecs::components::combat::DamageType::Ice => "Ледяной",
        crate::ecs::components::combat::DamageType::Holy => "Святой",
        crate::ecs::components::combat::DamageType::True => "Чистый (True)",
        _ => "Обычный",
    };
    let stats_text = format!("Урон: {:.0} ({}) | Кулдаун: {:.2}с", w.damage, dmg_type_str, w.base_cooldown);
    fonts.draw_text(&stats_text, card_x + 12.0, card_y + 44.0, 13.0, WHITE);

    fonts.draw_text(def.description, card_x + 12.0, card_y + 68.0, 12.0, LIGHTGRAY);

    let (evo_str, evo_col) = if w.is_evolved {
        ("⭐ СУПЕРОРУЖИЕ: Максимальная боевая эволюция".to_string(), GOLD)
    } else {
        let (req_passive, evo_weapon) = match w.kind {
            WeaponType::WindBlade => ("Перчатка Скорости", "Буря Клинков"),
            WeaponType::FireAura => ("Сердце Гиганта", "Адское Пекло"),
            WeaponType::MagicWand => ("Фолиант Времени", "Луч Смерти"),
            WeaponType::HolyHammer => ("Броня Света", "Праведный Щит"),
            WeaponType::IceShards => ("Кольцо Заморозки", "Ледниковая Буря"),
            WeaponType::PoisonOrbs => ("Едкий Сосуд", "Токсичный Мор"),
            WeaponType::ExecutionerAxe => ("Кровавый Камень", "Вихревой Секач"),
            WeaponType::CelestialBow => ("Орлиный Прицел", "Звездопадный Залп"),
            WeaponType::ThunderOrb => ("Песочные Часы", "Гнев Грозы"),
            WeaponType::BloodDaggers => ("Сфера Притяжения", "Кровавая Жатва"),
            _ => ("-", "-"),
        };
        (format!("Эволюция (Ур. 8 + {}): -> {}", req_passive, evo_weapon), YELLOW)
    };
    fonts.draw_text(&evo_str, card_x + 12.0, card_y + 92.0, 12.0, evo_col);
    fonts.draw_text("Совет: Босс выбирает апгрейды адаптивно в зависимости от угроз", card_x + 12.0, card_y + 114.0, 11.0, GRAY);
}

/// Всплывающая подсказка для пассивного предмета Босса
fn draw_passive_tooltip(p: &PassiveSlot, sx: f32, sy: f32, screen_w: f32, fonts: &GameFonts) {
    let def = get_passive_def(p.kind);
    let card_w = 330.0;
    let card_h = 125.0;
    let card_x = (sx + 19.0 - card_w / 2.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = sy + 36.0;

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.08, 0.06, 0.14, 0.96));
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, PURPLE);

    let title = format!("{} [Ур. {}/5]", def.name, p.level);
    fonts.draw_heading(&title, card_x + 12.0, card_y + 22.0, 16.0, GOLD);

    let bonus_text = match p.kind {
        PassiveType::SpeedGloves => format!("Текущий бонус: +{}% к скорости атаки оружия", p.level * 15),
        PassiveType::GiantHeart => format!("Текущий бонус: +{}% к макс. здоровью и регенерации", p.level * 25),
        PassiveType::CooldownTome => format!("Текущий бонус: -{}% времени отката оружия", p.level * 20),
        PassiveType::LightArmor => format!("Текущий бонус: +{:.0} к плоской броне, +{}% к защите", p.level as f32 * 8.0, p.level * 15),
        PassiveType::FrostRing => format!("Текущий бонус: +{}% к длительности замедления врагов", p.level * 20),
        PassiveType::CorrosiveFlask => format!("Текущий бонус: +{}% к пробитию брони всеми атаками", p.level * 15),
        PassiveType::BloodStone => format!("Текущий бонус: +{}% к общему урону, +{:.1} HP/с реген", p.level * 12, p.level as f32 * 2.5),
        PassiveType::EagleScope => format!("Текущий бонус: +{}% к скорости и дальности полета снарядов", p.level * 20),
        PassiveType::ChronoHourglass => format!("Текущий бонус: +{}% к длительности эффектов", p.level * 25),
        PassiveType::MagnetSphere => format!("Текущий бонус: +{} px к радиусу сбора кристаллов", p.level * 45),
    };
    fonts.draw_text(&bonus_text, card_x + 12.0, card_y + 44.0, 13.0, GREEN);

    fonts.draw_text(def.description, card_x + 12.0, card_y + 68.0, 12.0, LIGHTGRAY);

    let evo_synergy = match p.kind {
        PassiveType::SpeedGloves => "Связка для эволюции: Клинок Ветра -> Буря Клинков",
        PassiveType::GiantHeart => "Связка для эволюции: Огненная Аура -> Адское Пекло",
        PassiveType::CooldownTome => "Связка для эволюции: Магический Жезл -> Луч Смерти",
        PassiveType::LightArmor => "Связка для эволюции: Святой Молот -> Праведный Щит",
        PassiveType::FrostRing => "Связка для эволюции: Ледяные Осколки -> Ледниковая Буря",
        PassiveType::CorrosiveFlask => "Связка для эволюции: Чумные Сферы -> Токсичный Мор",
        PassiveType::BloodStone => "Связка для эволюции: Топор Палача -> Вихревой Секач",
        PassiveType::EagleScope => "Связка для эволюции: Небесный Лук -> Звездопадный Залп",
        PassiveType::ChronoHourglass => "Связка для эволюции: Грозовая Сфера -> Гнев Грозы",
        PassiveType::MagnetSphere => "Связка для эволюции: Кровавые Кинжалы -> Кровавая Жатва",
    };
    fonts.draw_text(evo_synergy, card_x + 12.0, card_y + 92.0, 12.0, YELLOW);
}

/// Всплывающая подсказка при наведении на объекты на карте
fn draw_world_entity_tooltip(
    screen_pos: Vec2,
    lines: Vec<(String, Color)>,
    screen_w: f32,
    screen_h: f32,
    fonts: &GameFonts,
) {
    let card_w = 340.0;
    let card_h = (lines.len() as f32 * 22.0) + 16.0;
    let card_x = (screen_pos.x + 16.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = (screen_pos.y - card_h - 12.0).clamp(10.0, screen_h - card_h - 10.0);

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.05, 0.05, 0.10, 0.95));
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 1.5, WHITE);

    for (i, (text, color)) in lines.iter().enumerate() {
        if i == 0 {
            fonts.draw_heading(text, card_x + 10.0, card_y + 20.0, 14.0, *color);
        } else {
            fonts.draw_text(text, card_x + 10.0, card_y + 20.0 + (i as f32 * 21.0), 12.0, *color);
        }
    }
}
