use macroquad::prelude::*;
use crate::content::mobs::get_squad_config;
use crate::content::passives::get_passive_def;
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
    pub open_item_inspection: Option<crate::render::item_dossier::ItemInspectionTarget>,
    pub direct_summon_tier: Option<u8>,
    pub direct_summon_max_tier: Option<u8>,
    pub upgrade_grade: Option<u8>,
    pub upgrade_grade_max: Option<u8>,
    pub clicked_pause: bool,
    pub clicked_codex: bool,
    pub clicked_sound_toggle: bool,
    pub clicked_next_track: bool,
    pub set_mob_page: Option<u8>,
    pub altar_tier: Option<u8>,
    pub is_mouse_over_ui: bool,
    pub map_click_world_pos: Option<Vec2>,
    pub start_altar_placement: bool,
    pub start_wall_placement: bool,
    pub chosen_perk: Option<crate::ecs::components::inventory::PerkChoice>,
    pub chosen_perk_index: Option<u8>,
    pub toggle_control_scheme: bool,
}

enum HoverTooltip {
    Weapon(WeaponSlot, f32, f32),
    Passive(PassiveSlot, f32, f32),
    WorldEntity(Vec2, Vec<(String, Color)>),
    WallWidget(f32, f32),
    SoulsPlaque(f32, f32),
    AltarButton {
        tier: u8,
        hotkey: String,
        bx: f32,
        by: f32,
    },
}

/// Процедурная отрисовка пиксельного навесного замка (Pixel Padlock)
/// Латунная дужка, металлический корпус и замочная скважина (GPL-3.0 совместимо, процедурный пиксель-арт)
pub fn draw_pixel_lock(x: f32, y: f32, color: Color) {
    let shackle_color = Color::new(color.r * 0.9 + 0.1, color.g * 0.9 + 0.1, color.b * 0.9 + 0.1, color.a);
    // Дужка замка (U-образная дуга)
    draw_line(x + 2.0, y + 1.0, x + 6.0, y + 1.0, 1.2, shackle_color);
    draw_line(x + 2.0, y + 1.0, x + 2.0, y + 4.5, 1.2, shackle_color);
    draw_line(x + 6.0, y + 1.0, x + 6.0, y + 4.5, 1.2, shackle_color);

    // Корпус замка (8x7 пикселей)
    draw_rectangle(x, y + 4.0, 8.5, 7.0, color);
    draw_rectangle_lines(x, y + 4.0, 8.5, 7.0, 1.0, Color::new(color.r * 0.45, color.g * 0.45, color.b * 0.45, color.a));

    // Замочная скважина
    let hole_col = Color::new(0.08, 0.05, 0.08, color.a);
    draw_circle(x + 4.25, y + 6.5, 1.0, hole_col);
    draw_line(x + 4.25, y + 7.0, x + 4.25, y + 9.0, 1.0, hole_col);
}

/// Процедурная отрисовка крупного пиксельного замка (Pixel Padlock Large)
/// для красивой блокировки всей карточки моба и алтаря
pub fn draw_pixel_lock_large(cx: f32, cy: f32, color: Color) {
    let shackle_color = Color::new(color.r * 0.9 + 0.1, color.g * 0.9 + 0.1, color.b * 0.9 + 0.1, color.a);
    // Дужка замка (толстая U-образная)
    draw_line(cx - 6.0, cy - 8.0, cx + 6.0, cy - 8.0, 2.2, shackle_color);
    draw_line(cx - 6.0, cy - 8.0, cx - 6.0, cy - 1.0, 2.2, shackle_color);
    draw_line(cx + 6.0, cy - 8.0, cx + 6.0, cy - 1.0, 2.2, shackle_color);

    // Корпус замка (18x14 пикселей)
    draw_rectangle(cx - 9.0, cy - 1.0, 18.0, 14.0, color);
    draw_rectangle_lines(cx - 9.0, cy - 1.0, 18.0, 14.0, 1.2, Color::new(color.r * 0.45, color.g * 0.45, color.b * 0.45, color.a));

    // Замочная скважина
    let hole_col = Color::new(0.08, 0.05, 0.08, color.a);
    draw_circle(cx, cy + 3.5, 2.0, hole_col);
    draw_line(cx, cy + 3.5, cx, cy + 9.0, 1.8, hole_col);
}

/// Процедурная отрисовка пиксельной молнии (Pixel Lightning Spark)
pub fn draw_pixel_lightning(x: f32, y: f32, color: Color) {
    draw_line(x + 4.0, y, x + 1.0, y + 5.0, 1.5, color);
    draw_line(x + 1.0, y + 5.0, x + 5.0, y + 4.0, 1.2, color);
    draw_line(x + 5.0, y + 4.0, x + 2.0, y + 10.0, 1.5, color);
}

pub fn render_ui(
    world: &World,
    clock: &GameClock,
    fonts: &GameFonts,
    cam_pos: Vec2,
    cam_zoom: f32,
    textures: &GameTextures,
    is_muted: bool,
    current_track: &str,
) -> UiInteraction {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let (mx, my) = mouse_position();
    let mut hovered_tooltip: Option<HoverTooltip> = None;
    let mut interaction = UiInteraction::default();

    // 1. Верхний цифровой таймер обратного отсчета (справа сверху)
    let timer_box_w = 165.0;
    let timer_box_h = 44.0;
    let timer_box_x = screen_w - timer_box_w - 20.0;
    let timer_box_y = 14.0;

    draw_rectangle(timer_box_x, timer_box_y, timer_box_w, timer_box_h, Color::new(0.06, 0.07, 0.11, 0.95));
    draw_rectangle_lines(timer_box_x, timer_box_y, timer_box_w, timer_box_h, 1.8, Color::new(0.4, 0.45, 0.55, 0.9));

    fonts.draw_text("ОСТАЛОСЬ ВРЕМЕНИ", timer_box_x + 12.0, timer_box_y + 14.0, 10.0, Color::new(0.8, 0.75, 0.5, 0.9));
    let timer_text = clock.formatted_remaining();
    fonts.draw_heading(&timer_text, timer_box_x + 12.0, timer_box_y + 36.0, 22.0, WHITE);

    // 1.1 Кнопка Паузы прямо под таймером
    let pause_btn_w = timer_box_w;
    let pause_btn_h = 28.0;
    let pause_btn_x = timer_box_x;
    let pause_btn_y = timer_box_y + timer_box_h + 6.0;
    let pause_hover = mx >= pause_btn_x && mx <= pause_btn_x + pause_btn_w && my >= pause_btn_y && my <= pause_btn_y + pause_btn_h;
    if pause_hover {
        interaction.is_mouse_over_ui = true;
    }
    draw_rectangle(pause_btn_x, pause_btn_y, pause_btn_w, pause_btn_h, if pause_hover { Color::new(0.32, 0.22, 0.12, 1.0) } else { Color::new(0.12, 0.11, 0.10, 0.95) });
    draw_rectangle_lines(pause_btn_x, pause_btn_y, pause_btn_w, pause_btn_h, 1.5, if pause_hover { GOLD } else { GRAY });
    let p_label = "Пауза [Esc]";
    let pw = fonts.measure_heading(p_label, 12.0).width;
    fonts.draw_heading(p_label, pause_btn_x + (pause_btn_w - pw) / 2.0, pause_btn_y + 18.0, 12.0, if pause_hover { GOLD } else { WHITE });

    if is_mouse_button_pressed(MouseButton::Left) && pause_hover {
        interaction.clicked_pause = true;
    }

    // 1.2 Кнопка переключения звука [M] под кнопкой паузы
    let sound_btn_w = timer_box_w;
    let sound_btn_h = 24.0;
    let sound_btn_x = timer_box_x;
    let sound_btn_y = pause_btn_y + pause_btn_h + 5.0;
    let sound_hover = mx >= sound_btn_x && mx <= sound_btn_x + sound_btn_w && my >= sound_btn_y && my <= sound_btn_y + sound_btn_h;
    if sound_hover {
        interaction.is_mouse_over_ui = true;
    }
    draw_rectangle(sound_btn_x, sound_btn_y, sound_btn_w, sound_btn_h, if sound_hover { Color::new(0.25, 0.22, 0.32, 1.0) } else { Color::new(0.10, 0.11, 0.15, 0.95) });
    draw_rectangle_lines(sound_btn_x, sound_btn_y, sound_btn_w, sound_btn_h, 1.2, if sound_hover { GOLD } else { DARKGRAY });
    let s_label = if is_muted { "Звук: ВЫКЛ [M]" } else { "Звук: ВКЛ [M]" };
    let sw = fonts.measure_heading(s_label, 11.0).width;
    fonts.draw_heading(s_label, sound_btn_x + (sound_btn_w - sw) / 2.0, sound_btn_y + 16.0, 11.0, if is_muted { RED } else if sound_hover { GOLD } else { GREEN });

    if is_mouse_button_pressed(MouseButton::Left) && sound_hover {
        interaction.clicked_sound_toggle = true;
    }

    // 1.3 Кнопка переключения трека [N] под кнопкой звука
    let track_btn_w = timer_box_w;
    let track_btn_h = 24.0;
    let track_btn_x = timer_box_x;
    let track_btn_y = sound_btn_y + sound_btn_h + 5.0;
    let track_hover = mx >= track_btn_x && mx <= track_btn_x + track_btn_w && my >= track_btn_y && my <= track_btn_y + track_btn_h;
    if track_hover {
        interaction.is_mouse_over_ui = true;
    }
    draw_rectangle(track_btn_x, track_btn_y, track_btn_w, track_btn_h, if track_hover { Color::new(0.20, 0.25, 0.38, 1.0) } else { Color::new(0.10, 0.11, 0.15, 0.95) });
    draw_rectangle_lines(track_btn_x, track_btn_y, track_btn_w, track_btn_h, 1.2, if track_hover { SKYBLUE } else { DARKGRAY });
    let t_label = "Трек [N]";
    let tw = fonts.measure_heading(t_label, 11.0).width;
    let t_col = if track_hover { GOLD } else { SKYBLUE };
    let t_start_x = track_btn_x + (track_btn_w - (tw + 12.0)) / 2.0;
    fonts.draw_heading(t_label, t_start_x, track_btn_y + 16.0, 11.0, t_col);
    // Процедурные стрелки вперед [►►] вместо символа '>'
    let ar_x = t_start_x + tw + 4.0;
    let ar_y = track_btn_y + track_btn_h / 2.0;
    draw_triangle(Vec2::new(ar_x + 4.0, ar_y), Vec2::new(ar_x, ar_y - 3.5), Vec2::new(ar_x, ar_y + 3.5), t_col);
    draw_triangle(Vec2::new(ar_x + 8.0, ar_y), Vec2::new(ar_x + 4.0, ar_y - 3.5), Vec2::new(ar_x + 4.0, ar_y + 3.5), t_col);

    if is_mouse_button_pressed(MouseButton::Left) && track_hover {
        interaction.clicked_next_track = true;
    }

    // Всплывающая подсказка с названием текущего трека при наведении курсора
    if track_hover {
        let tt_txt = format!("Сейчас играет: {}", current_track);
        let tt_w = fonts.measure_text(&tt_txt, 11.0).width + 16.0;
        let tt_x = (track_btn_x - tt_w - 6.0).max(10.0);
        let tt_y = track_btn_y;
        draw_rectangle(tt_x, tt_y, tt_w, 24.0, Color::new(0.06, 0.08, 0.14, 0.96));
        draw_rectangle_lines(tt_x, tt_y, tt_w, 24.0, 1.0, GOLD);
        fonts.draw_text(&tt_txt, tt_x + 8.0, tt_y + 16.0, 11.0, WHITE);
    }

    // 2. Полоска здоровья Босса и Опыт (сверху по центру)
    let bar_w = 560.0;
    let bar_h = 22.0;
    let bar_x = (screen_w - bar_w) / 2.0;
    let bar_y = 14.0;

    let hp_ratio = (world.boss_health.current / world.boss_health.max).clamp(0.0, 1.0);
    let hp_bar_color = if world.boss_poison_timer > 0.0 {
        Color::new(0.68, 0.15, 0.75, 1.0) // Фиолетово-токсичный цвет при отравлении
    } else {
        Color::new(0.88, 0.18, 0.22, 1.0)
    };
    draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.10, 0.10, 0.15, 0.96));
    draw_rectangle(bar_x, bar_y, bar_w * hp_ratio, bar_h, hp_bar_color);
    draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 2.0, Color::new(0.9, 0.8, 0.5, 1.0));

    let poison_tag = if world.boss_poison_timer > 0.0 {
        format!(" | ЯД (-{:.0}/с)", world.boss_poison_dps)
    } else {
        String::new()
    };
    let hp_text = format!(
        "ВЕРХОВНЫЙ ПАЛАДИН (Ур. {}) | {:.0} / {:.0} HP | Броня: {:.1}{}",
        world.boss_inventory.level,
        world.boss_health.current,
        world.boss_health.max,
        world.boss_resistances.flat_armor,
        poison_tag
    );
    let hp_text_w = fonts.measure_heading(&hp_text, 13.5).width;
    fonts.draw_heading(&hp_text, (screen_w - hp_text_w) / 2.0, bar_y + 16.0, 13.5, WHITE);

    // Полоска опыта босса
    let xp_ratio = (world.boss_inventory.current_xp / world.boss_inventory.xp_to_next_level).clamp(0.0, 1.0);
    draw_rectangle(bar_x, bar_y + bar_h + 3.0, bar_w, 4.0, Color::new(0.12, 0.12, 0.20, 0.9));
    draw_rectangle(bar_x, bar_y + bar_h + 3.0, bar_w * xp_ratio, 4.0, SKYBLUE);

    // 3. ДВА РАЗДЕЛЬНЫХ ПОЛЯ: ОРУЖИЕ И ПАССИВНЫЕ РЕЛИКВИИ
    let field_y = bar_y + bar_h + 10.0;
    let field_w = 270.0;
    let field_h = 78.0;
    let slot_size = 40.0;
    let slot_gap = 4.0;

    // 3.1 Левое поле: Оружие Босса (6 слотов)
    let wep_field_x = bar_x;
    draw_rectangle(wep_field_x, field_y, field_w, field_h, Color::new(0.07, 0.08, 0.12, 0.95));
    draw_rectangle_lines(wep_field_x, field_y, field_w, field_h, 1.5, Color::new(0.35, 0.50, 0.70, 0.85));
    fonts.draw_heading("ОРУЖИЕ БОССА", wep_field_x + 8.0, field_y + 14.0, 11.0, SKYBLUE);

    let wep_slots_start_x = wep_field_x + 4.0;
    let wep_slots_y = field_y + 22.0;

    for i in 0..6 {
        let sx = wep_slots_start_x + (i as f32) * (slot_size + slot_gap);
        let is_hover = mx >= sx && mx <= sx + slot_size && my >= wep_slots_y && my <= wep_slots_y + slot_size;
        if is_hover { interaction.is_mouse_over_ui = true; }

        draw_rectangle(sx, wep_slots_y, slot_size, slot_size, Color::new(0.04, 0.05, 0.08, 0.95));

        if let Some(w) = world.boss_inventory.weapons.get(i) {
            let (border_col, bg_col) = if w.is_evolved {
                (GOLD, Color::new(0.40, 0.22, 0.05, 0.9))
            } else {
                (Color::new(0.3, 0.6, 0.9, 0.9), Color::new(0.08, 0.18, 0.32, 0.9))
            };
            draw_rectangle(sx + 2.0, wep_slots_y + 2.0, slot_size - 4.0, slot_size - 4.0, bg_col);
            draw_rectangle_lines(sx, wep_slots_y, slot_size, slot_size, if w.is_evolved { 2.0 } else { 1.5 }, border_col);

            if let Some(icon) = textures.get_weapon_icon(w.kind) {
                draw_texture_ex(
                    icon,
                    sx + 3.0,
                    wep_slots_y + 3.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(slot_size - 6.0, slot_size - 6.0)),
                        ..Default::default()
                    },
                );
            }

            // Бейдж уровня в правом нижнем углу слота (с поддержкой Запредельного Возвышения Limit Break)
            let lvl_tag = if w.limit_break_level > 0 {
                format!("+{}", w.limit_break_level)
            } else if w.is_evolved {
                "MAX".to_string()
            } else {
                format!("{}", w.level)
            };
            let tag_w = fonts.measure_heading(&lvl_tag, 10.0).width + 4.0;
            let tag_bg = if w.limit_break_level > 0 {
                Color::new(0.50, 0.12, 0.65, 0.95)
            } else {
                Color::new(0.0, 0.0, 0.0, 0.85)
            };
            draw_rectangle(sx + slot_size - tag_w, wep_slots_y + slot_size - 11.0, tag_w, 11.0, tag_bg);
            fonts.draw_heading(&lvl_tag, sx + slot_size - tag_w + 2.0, wep_slots_y + slot_size - 2.0, 10.0, if w.limit_break_level > 0 || w.is_evolved { GOLD } else { WHITE });

            // Значок [i] в правом верхнем углу слота для открытия полного Древа Синтеза
            let btn_i_w = 13.0;
            let btn_i_h = 12.0;
            let btn_i_x = sx + slot_size - btn_i_w - 2.0;
            let btn_i_y = wep_slots_y + 2.0;
            let is_i_hover = mx >= btn_i_x && mx <= btn_i_x + btn_i_w && my >= btn_i_y && my <= btn_i_y + btn_i_h;

            draw_rectangle(btn_i_x, btn_i_y, btn_i_w, btn_i_h, if is_i_hover { Color::new(0.35, 0.15, 0.50, 0.95) } else { Color::new(0.12, 0.14, 0.22, 0.90) });
            draw_rectangle_lines(btn_i_x, btn_i_y, btn_i_w, btn_i_h, 1.0, if is_i_hover { GOLD } else { SKYBLUE });
            let iw_text = fonts.measure_heading("i", 9.0).width;
            fonts.draw_heading("i", btn_i_x + (btn_i_w - iw_text) / 2.0, btn_i_y + 9.5, 9.0, if is_i_hover { GOLD } else { WHITE });

            if is_hover && is_mouse_button_pressed(MouseButton::Left) {
                interaction.open_item_inspection = Some(crate::render::item_dossier::ItemInspectionTarget::Weapon(w.kind, w.level, w.limit_break_level));
            }

            if is_hover {
                hovered_tooltip = Some(HoverTooltip::Weapon(w.clone(), sx, wep_slots_y));
            }
        } else {
            // Пустой слот оружия
            draw_rectangle_lines(sx, wep_slots_y, slot_size, slot_size, 1.0, Color::new(0.2, 0.22, 0.28, 0.7));
            draw_line(sx + 8.0, wep_slots_y + 8.0, sx + slot_size - 8.0, wep_slots_y + slot_size - 8.0, 1.0, Color::new(0.25, 0.25, 0.35, 0.4));
        }
    }

    // Центральный декоративный элемент между полями
    let div_cx = bar_x + bar_w / 2.0;
    draw_line(div_cx, field_y + 6.0, div_cx, field_y + field_h - 6.0, 1.5, Color::new(0.5, 0.45, 0.6, 0.6));
    draw_circle(div_cx, field_y + field_h / 2.0, 3.5, GOLD);

    // 3.2 Правое поле: Пассивные Реликвии (10 слотов: 2 ряда по 5)
    let pas_field_x = bar_x + bar_w - field_w;
    draw_rectangle(pas_field_x, field_y, field_w, field_h, Color::new(0.08, 0.06, 0.12, 0.95));
    draw_rectangle_lines(pas_field_x, field_y, field_w, field_h, 1.5, Color::new(0.60, 0.35, 0.75, 0.85));
    fonts.draw_heading("ПАССИВНЫЕ РЕЛИКВИИ", pas_field_x + 8.0, field_y + 14.0, 11.0, Color::new(0.85, 0.55, 1.0, 1.0));

    let p_slot_s = 26.0;
    let p_slot_gap = 5.0;
    let p_total_w = 5.0 * p_slot_s + 4.0 * p_slot_gap;
    let p_start_x = pas_field_x + (field_w - p_total_w) / 2.0;

    for i in 0..10 {
        let col_idx = (i % 5) as f32;
        let row_idx = (i / 5) as f32;
        let sx = p_start_x + col_idx * (p_slot_s + p_slot_gap);
        let sy = field_y + 19.0 + row_idx * (p_slot_s + 2.0);

        let is_hover = mx >= sx && mx <= sx + p_slot_s && my >= sy && my <= sy + p_slot_s;
        if is_hover { interaction.is_mouse_over_ui = true; }

        draw_rectangle(sx, sy, p_slot_s, p_slot_s, Color::new(0.05, 0.04, 0.08, 0.95));

        if let Some(p) = world.boss_inventory.passives.get(i) {
            draw_rectangle(sx + 1.0, sy + 1.0, p_slot_s - 2.0, p_slot_s - 2.0, Color::new(0.26, 0.10, 0.34, 0.9));
            draw_rectangle_lines(sx, sy, p_slot_s, p_slot_s, 1.5, Color::new(0.75, 0.40, 0.90, 0.95));

            if let Some(icon) = textures.get_passive_icon(p.kind) {
                draw_texture_ex(
                    icon,
                    sx + 2.0,
                    sy + 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(p_slot_s - 4.0, p_slot_s - 4.0)),
                        ..Default::default()
                    },
                );
            }

            let lvl_tag = format!("{}", p.level);
            let tag_w = fonts.measure_heading(&lvl_tag, 9.0).width + 3.0;
            draw_rectangle(sx + p_slot_s - tag_w, sy + p_slot_s - 9.0, tag_w, 9.0, Color::new(0.0, 0.0, 0.0, 0.85));
            fonts.draw_heading(&lvl_tag, sx + p_slot_s - tag_w + 1.0, sy + p_slot_s - 1.0, 9.0, GOLD);

            // Значок [i] в правом верхнем углу слота пассивки
            let btn_i_w = 11.0;
            let btn_i_h = 10.0;
            let btn_i_x = sx + p_slot_s - btn_i_w - 1.0;
            let btn_i_y = sy + 1.0;
            let is_i_hover = mx >= btn_i_x && mx <= btn_i_x + btn_i_w && my >= btn_i_y && my <= btn_i_y + btn_i_h;

            draw_rectangle(btn_i_x, btn_i_y, btn_i_w, btn_i_h, if is_i_hover { Color::new(0.45, 0.15, 0.55, 0.95) } else { Color::new(0.15, 0.10, 0.22, 0.90) });
            draw_rectangle_lines(btn_i_x, btn_i_y, btn_i_w, btn_i_h, 1.0, if is_i_hover { GOLD } else { PURPLE });
            let iw_text = fonts.measure_heading("i", 8.0).width;
            fonts.draw_heading("i", btn_i_x + (btn_i_w - iw_text) / 2.0, btn_i_y + 8.0, 8.0, if is_i_hover { GOLD } else { WHITE });

            if is_hover && is_mouse_button_pressed(MouseButton::Left) {
                interaction.open_item_inspection = Some(crate::render::item_dossier::ItemInspectionTarget::Passive(p.kind, p.level));
            }

            if is_hover {
                hovered_tooltip = Some(HoverTooltip::Passive(p.clone(), sx, sy));
            }
        } else {
            // Пустой слот реликвии
            draw_rectangle_lines(sx, sy, p_slot_s, p_slot_s, 1.0, Color::new(0.25, 0.20, 0.30, 0.7));
            draw_circle_lines(sx + p_slot_s / 2.0, sy + p_slot_s / 2.0, 4.0, 1.0, Color::new(0.35, 0.25, 0.40, 0.4));
        }
    }

    // 4. Баннер Level-Up / Эволюции Босса
    if world.boss_inventory.perk_announcement_timer > 0.0 {
        if let Some(ref perk_text) = world.boss_inventory.last_chosen_perk {
            let banner_w = 480.0;
            let banner_h = 34.0;
            let bx = (screen_w - banner_w) / 2.0;
            let by = field_y + field_h + 8.0;

            draw_rectangle(bx, by, banner_w, banner_h, Color::new(0.08, 0.08, 0.14, 0.96));
            draw_rectangle_lines(bx, by, banner_w, banner_h, 2.0, GOLD);

            let tw = fonts.measure_heading(perk_text, 16.0).width;
            fonts.draw_heading(perk_text, (screen_w - tw) / 2.0, by + 23.0, 16.0, GOLD);
        }
    }

    // 5. Верхняя левая кнопка: Кодекс [ Кодекс (I) ] с графической иконкой книги
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
    if let Some(ref book_tex) = textures.icon_book {
        draw_texture_ex(
            book_tex,
            codex_btn_x + 8.0,
            codex_btn_y + 6.0,
            if codex_hover { GOLD } else { WHITE },
            DrawTextureParams {
                dest_size: Some(Vec2::splat(22.0)),
                ..Default::default()
            },
        );
    }
    fonts.draw_heading("Кодекс [I]", codex_btn_x + 36.0, codex_btn_y + 22.0, 14.0, if codex_hover { GOLD } else { WHITE });

    if is_mouse_button_pressed(MouseButton::Left) && codex_hover {
        interaction.clicked_codex = true;
    }

    // Подсказка навигации под кнопкой кодекса (в 2 компактные строки)
    fonts.draw_text("Стрелки: Камера | Колесо: Зум", 20.0, 62.0, 11.5, YELLOW);
    fonts.draw_text("[Пробел]: Центр босса | ЛКМ: Двигать карту", 20.0, 78.0, 11.5, YELLOW);

    // 6. ТАКТИЧЕСКАЯ МИНИКАРТА (В левой нижней части экрана)
    draw_minimap(world, cam_pos, cam_zoom, fonts, &mut interaction);

    // 7. СТИЛЬНАЯ ПАНЕЛЬ ОЧКОВ ДУШ И ДОХОДА В ОДНУ СТРОКУ (Справа внизу)
    if let Some(director) = world.directors.first() {
        let panel_w = 236.0;
        let panel_h = 40.0;
        let panel_x = screen_w - panel_w - 14.0;
        let panel_y = screen_h - panel_h - 16.0;

        if mx >= panel_x && mx <= panel_x + panel_w && my >= panel_y && my <= panel_y + panel_h {
            interaction.is_mouse_over_ui = true;
            hovered_tooltip = Some(HoverTooltip::SoulsPlaque(panel_x, panel_y));
        }

        let altar_count = world.structures.iter().filter(|s| crate::content::structures::is_altar_structure_type(s.structure_type) && s.health.is_alive()).count();
        let total_income = director.income_rate + (altar_count as f32 * 3.0);

        // Внешняя темная обсидиановая плашка со скошенными углами и золотым кантом
        draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.06, 0.06, 0.09, 0.96));
        draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.8, Color::new(0.85, 0.70, 0.25, 0.95));

        // Внутренний металлический блик
        draw_line(panel_x + 2.0, panel_y + 2.0, panel_x + panel_w - 2.0, panel_y + 2.0, 1.0, Color::new(1.0, 0.9, 0.5, 0.35));

        // Золотые заклепки по углам
        let rivet_col = Color::new(0.95, 0.8, 0.3, 1.0);
        draw_circle(panel_x + 4.0, panel_y + 4.0, 1.5, rivet_col);
        draw_circle(panel_x + panel_w - 4.0, panel_y + 4.0, 1.5, rivet_col);
        draw_circle(panel_x + 4.0, panel_y + panel_h - 4.0, 1.5, rivet_col);
        draw_circle(panel_x + panel_w - 4.0, panel_y + panel_h - 4.0, 1.5, rivet_col);

        // 1. Призрачная дымчатая иконка душ слева
        let time = get_time() as f32;
        let soul_icon_s = 24.0;
        let icon_x = panel_x + 6.0;
        let icon_y = panel_y + (panel_h - soul_icon_s) / 2.0;

        if let Some(s_tex) = textures.soul_ethereal.as_ref() {
            draw_texture_ex(
                s_tex,
                icon_x,
                icon_y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(soul_icon_s, soul_icon_s)),
                    ..Default::default()
                },
            );
        } else if let Some(c_tex) = textures.soul_crystal.as_ref() {
            draw_texture_ex(c_tex, icon_x, icon_y, GOLD, DrawTextureParams { dest_size: Some(Vec2::new(soul_icon_s, soul_icon_s)), ..Default::default() });
        } else {
            draw_circle(icon_x + 12.0, icon_y + 12.0, 8.0, SKYBLUE);
        }

        // Дымчатые парящие частицы душ (живой вайб душ)
        for p_i in 0..3 {
            let p_offset = (time * 2.2 + p_i as f32 * 1.5) % 2.5;
            let p_x = icon_x + 8.0 + (time * 3.5 + p_i as f32).sin() * 4.0;
            let p_y = icon_y + 14.0 - p_offset * 7.0;
            let p_alpha = (1.0 - p_offset / 2.5).clamp(0.0, 1.0) * 0.5;
            draw_circle(p_x, p_y, 1.5, Color::new(0.4, 0.9, 1.0, p_alpha));
        }

        // 2. Число Душ (крупное, золотое, в одну строку)
        let points_text = format!("{:.0}", director.points);
        fonts.draw_heading(&points_text, panel_x + 33.0, panel_y + 26.0, 17.0, GOLD);
        let points_w = fonts.measure_heading(&points_text, 17.0).width;

        // Разделитель 1
        let div1_x = panel_x + 37.0 + points_w;
        draw_line(div1_x, panel_y + 8.0, div1_x, panel_y + panel_h - 8.0, 1.0, DARKGRAY);

        // 3. Индикатор Дохода (в одну строку рядом с числом душ): +X.X [иконка душ] / 2с
        let inc_start_x = div1_x + 7.0;
        let inc_val_text = format!("+{:.1}", total_income);
        fonts.draw_heading(&inc_val_text, inc_start_x, panel_y + 26.0, 12.5, GREEN);
        let inc_w = fonts.measure_heading(&inc_val_text, 12.5).width;

        // Маленький символ очков душ прямо рядом с доходом
        let sm_soul_x = inc_start_x + inc_w + 2.0;
        let sm_soul_s = 15.0;
        if let Some(sm_tex) = textures.soul_ethereal_sm.as_ref() {
            draw_texture_ex(
                sm_tex,
                sm_soul_x,
                panel_y + (panel_h - sm_soul_s) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(sm_soul_s, sm_soul_s)),
                    ..Default::default()
                },
            );
        } else {
            draw_circle(sm_soul_x + 7.0, panel_y + panel_h / 2.0, 4.0, SKYBLUE);
        }

        let per_sec_x = sm_soul_x + sm_soul_s + 1.0;
        fonts.draw_text("/2с", per_sec_x, panel_y + 25.0, 10.5, SKYBLUE);
        let per_sec_w = fonts.measure_text("/2с", 10.5).width;

        // Разделитель 2
        let div2_x = per_sec_x + per_sec_w + 6.0;
        draw_line(div2_x, panel_y + 8.0, div2_x, panel_y + panel_h - 8.0, 1.0, DARKGRAY);

        // 4. Число алтарей
        let altar_str = format!("Алтари: {}", altar_count);
        fonts.draw_heading(&altar_str, div2_x + 6.0, panel_y + 25.0, 11.5, PURPLE);

        // Виджет стены остаётся отдельным; алтари выбираются на карточках мобов.
        let sw_w = panel_w;
        let sw_h = 52.0;
        let sw_y = panel_y - sw_h - 6.0;
        let ww_x = panel_x;
        let on_wall_cd = director.wall_cooldown > 0.0;
        let can_afford_wall = director.points >= crate::content::structures::WALL_COST;

        if mx >= ww_x && mx <= ww_x + sw_w && my >= sw_y && my <= sw_y + sw_h {
            interaction.is_mouse_over_ui = true;
            hovered_tooltip = Some(HoverTooltip::WallWidget(ww_x, sw_y));
            if is_mouse_button_pressed(MouseButton::Left) && !on_wall_cd && can_afford_wall {
                interaction.start_wall_placement = true;
            }
        }

        let ww_border = if on_wall_cd {
            Color::new(0.9, 0.25, 0.25, 0.95)
        } else if can_afford_wall {
            Color::new(0.4, 0.7, 0.9, 0.95)
        } else {
            GRAY
        };

        draw_rectangle(ww_x, sw_y, sw_w, sw_h, Color::new(0.08, 0.08, 0.14, 0.95));
        draw_rectangle_lines(ww_x, sw_y, sw_w, sw_h, 1.8, ww_border);

        if let Some(wall_tex) = textures.wall.as_ref() {
            let icon_s = 26.0;
            let icon_tint = if on_wall_cd { Color::new(0.8, 0.4, 0.4, 0.8) } else { WHITE };
            draw_texture_ex(
                wall_tex,
                ww_x + 5.0,
                sw_y + (sw_h - icon_s) / 2.0,
                icon_tint,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(icon_s, icon_s)),
                    ..Default::default()
                },
            );
        } else {
            draw_rectangle(ww_x + 8.0, sw_y + 14.0, 22.0, 22.0, GRAY);
        }

        let w_cx = ww_x + 38.0;
        fonts.draw_heading("СТЕНА [B]", w_cx, sw_y + 16.0, 10.5, SKYBLUE);
        if on_wall_cd {
            let cd_text = format!("{:.1}с", director.wall_cooldown);
            fonts.draw_text(&cd_text, w_cx, sw_y + 30.0, 10.0, Color::new(1.0, 0.35, 0.35, 1.0));
            let prog = (1.0 - director.wall_cooldown / director.wall_max_cooldown).clamp(0.0, 1.0);
            draw_rectangle(w_cx, sw_y + 34.0, 70.0, 3.0, Color::new(0.2, 0.1, 0.1, 0.9));
            draw_rectangle(w_cx, sw_y + 34.0, 70.0 * prog, 3.0, SKYBLUE);
        } else if can_afford_wall {
            fonts.draw_text("ГОТОВА [10]", w_cx, sw_y + 30.0, 10.5, GREEN);
        } else {
            fonts.draw_text("10 Душ", w_cx, sw_y + 30.0, 10.0, GRAY);
        }
        let live_walls = world.structures.iter().filter(|s| s.structure_type == crate::content::structures::STRUCTURE_WALL && s.health.is_alive()).count();
        let w_cnt = format!("Жизнь: {} шт", live_walls);
        fonts.draw_text(&w_cnt, w_cx, sw_y + 46.0, 9.5, LIGHTGRAY);

        // 8. ПАНЕЛЬ ГРЕЙДОВ МОБОВ ДИРЕКТОРА [Z, X, C] И ПЕРЕКЛЮЧЕНИЯ СТРАНИЦ [ОРДА / ЛЕГИОН]
        let is_shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let card_w = 78.0;
        let card_gap = 5.0;
        let total_cards_w = 8.0 * card_w + 7.0 * card_gap; // 659.0px
        let cards_start_x = (screen_w - total_cards_w) / 2.0;

        let page_tab_w = 68.0;
        let g_gap = 5.0;
        let g_btn_w = (total_cards_w - 3.0 * page_tab_w - 5.0 * g_gap) / 3.0; // 143.0px
        let g_btn_h = 28.0;
        let card_h = 146.0;
        let cards_y = screen_h - card_h - 14.0;
        let g_y = cards_y - g_btn_h - 6.0;

        let grades = [
            ("Z", "Ярость Крови", director.grade_fury, 100.0 * (director.grade_fury as f32 + 1.0), "+12% урон, +6% скор."),
            ("X", "Панцирь", director.grade_carapace, 120.0 * (director.grade_carapace as f32 + 1.0), "+2.5 брони, +5% рез."),
            ("C", "Чумные Споры", director.grade_pestilence, 150.0 * (director.grade_pestilence as f32 + 1.0), "Яд атак + Аура алтарей"),
        ];

        for (i, (key, name, lvl, cost, desc)) in grades.iter().enumerate() {
            let gx = cards_start_x + (i as f32) * (g_btn_w + g_gap);
            let g_hover = mx >= gx && mx <= gx + g_btn_w && my >= g_y && my <= g_y + g_btn_h;
            if g_hover {
                interaction.is_mouse_over_ui = true;
            }

            let is_max = *lvl >= crate::ecs::world::MAX_DIRECTOR_GRADE;
            let can_buy = !is_max && director.points >= *cost;
            let bg = if can_buy {
                if g_hover { Color::new(0.28, 0.20, 0.42, 0.98) } else { Color::new(0.20, 0.15, 0.30, 0.95) }
            } else {
                Color::new(0.10, 0.10, 0.15, 0.90)
            };
            let border = if can_buy { if g_hover { GOLD } else { PURPLE } } else { GRAY };

            draw_rectangle(gx, g_y, g_btn_w, g_btn_h, bg);
            draw_rectangle_lines(gx, g_y, g_btn_w, g_btn_h, 1.5, border);

            let title = if is_max {
                format!("[{}] {} (МАКС 30)", key, name)
            } else {
                format!("[{}] {} (Ур. {}) - {:.0}", key, name, lvl, cost)
            };
            fonts.draw_heading(&title, gx + 6.0, g_y + 13.0, 10.5, if can_buy { GOLD } else { LIGHTGRAY });
            fonts.draw_text(desc, gx + 6.0, g_y + 25.0, 9.5, SKYBLUE);

            if is_mouse_button_pressed(MouseButton::Left) && g_hover && can_buy {
                if is_shift {
                    interaction.upgrade_grade_max = Some(i as u8);
                } else {
                    interaction.upgrade_grade = Some(i as u8);
                }
            }
        }

        // Переключатели Страниц Мобов: [1: ОРДА], [2: ЛЕГИОН], [3: ГЕРОИ]
        let page_tab_h = g_btn_h;
        let p1_x = cards_start_x + 3.0 * (g_btn_w + g_gap);
        let p2_x = p1_x + page_tab_w + g_gap;
        let p3_x = p2_x + page_tab_w + g_gap;

        let p1_hover = mx >= p1_x && mx <= p1_x + page_tab_w && my >= g_y && my <= g_y + page_tab_h;
        let p2_hover = mx >= p2_x && mx <= p2_x + page_tab_w && my >= g_y && my <= g_y + page_tab_h;
        let p3_hover = mx >= p3_x && mx <= p3_x + page_tab_w && my >= g_y && my <= g_y + page_tab_h;
        if p1_hover || p2_hover || p3_hover {
            interaction.is_mouse_over_ui = true;
        }

        let p1_active = director.mob_page == 0;
        let p2_active = director.mob_page == 1;
        let p3_active = director.mob_page == 2;

        // Кнопка Страница 1
        draw_rectangle(p1_x, g_y, page_tab_w, page_tab_h, if p1_active { Color::new(0.24, 0.18, 0.38, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
        draw_rectangle_lines(p1_x, g_y, page_tab_w, page_tab_h, if p1_active { 2.0 } else { 1.0 }, if p1_active { GOLD } else { GRAY });
        fonts.draw_heading("ОРДА [1]", p1_x + 7.0, g_y + 13.0, 10.5, if p1_active { GOLD } else { LIGHTGRAY });
        fonts.draw_text("Тиры 1-8", p1_x + 7.0, g_y + 25.0, 9.0, if p1_active { WHITE } else { GRAY });
        if is_mouse_button_pressed(MouseButton::Left) && p1_hover {
            interaction.set_mob_page = Some(0);
        }

        // Кнопка Страница 2
        draw_rectangle(p2_x, g_y, page_tab_w, page_tab_h, if p2_active { Color::new(0.38, 0.14, 0.22, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
        draw_rectangle_lines(p2_x, g_y, page_tab_w, page_tab_h, if p2_active { 2.0 } else { 1.0 }, if p2_active { GOLD } else { GRAY });
        fonts.draw_heading("ЛЕГИОН [2]", p2_x + 5.0, g_y + 13.0, 10.5, if p2_active { GOLD } else { LIGHTGRAY });
        fonts.draw_text("Тиры 9-16", p2_x + 5.0, g_y + 25.0, 9.0, if p2_active { WHITE } else { GRAY });
        if is_mouse_button_pressed(MouseButton::Left) && p2_hover {
            interaction.set_mob_page = Some(1);
        }

        // Кнопка Страница 3: ГЕРОИ
        draw_rectangle(p3_x, g_y, page_tab_w, page_tab_h, if p3_active { Color::new(0.48, 0.12, 0.18, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
        draw_rectangle_lines(p3_x, g_y, page_tab_w, page_tab_h, if p3_active { 2.0 } else { 1.0 }, if p3_active { GOLD } else { GRAY });
        fonts.draw_heading("ГЕРОИ [3]", p3_x + 5.0, g_y + 13.0, 10.5, if p3_active { GOLD } else { LIGHTGRAY });
        fonts.draw_text("Титаны", p3_x + 5.0, g_y + 25.0, 9.0, if p3_active { WHITE } else { GRAY });
        if is_mouse_button_pressed(MouseButton::Left) && p3_hover {
            interaction.set_mob_page = Some(2);
        }

        // 9. КАРТОЧКИ ПРИЗЫВА МОБОВ (Центральный блок: идеальные отступы от краев, ЛКМ = призыв, кнопка [i] = досье, Shift = на все души)
        let any_obelisk_active = world.structures.iter().any(|s| {
            s.structure_type == crate::content::structures::STRUCTURE_OBELISK && s.is_active
        });
        let hero_alive = world.mobs.iter().any(|m| m.is_hero && m.health.is_alive());
        let on_hero_cd = director.hero_cooldown > 0.0;

        let page_offset = director.mob_page * 8;
        let mob_keys = ["Q", "W", "E", "R", "A", "S", "D", "F"];
        for i in 0..8 {
            let tier = if director.mob_page == 2 {
                if i == 0 { 17 } else if i == 1 { 18 } else { 0 }
            } else {
                (i + 1) as u8 + page_offset
            };

            let cx = cards_start_x + (i as f32) * (card_w + card_gap);
            let cy = cards_y;

            if tier == 0 {
                // Пустой слот для будущих героев
                draw_rectangle(cx, cy, card_w, card_h, Color::new(0.05, 0.05, 0.07, 0.70));
                draw_rectangle_lines(cx, cy, card_w, card_h, 1.0, Color::new(0.20, 0.20, 0.25, 0.40));
                let soon = "СКОРО";
                let sw = fonts.measure_text(soon, 10.0).width;
                fonts.draw_text(soon, cx + (card_w - sw) / 2.0, cy + card_h / 2.0, 10.0, DARKGRAY);
                continue;
            }

            let config = get_squad_config(tier);
            let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;
            let can_afford = director.points >= config.cost;

            let cx = cards_start_x + (i as f32) * (card_w + card_gap);
            let cy = cards_y;

            let card_hover = mx >= cx && mx <= cx + card_w && my >= cy && my <= cy + card_h;
            if card_hover {
                interaction.is_mouse_over_ui = true;
            }

            // Кнопка [i] инфо в верхнем правом углу карточки
            let info_btn_w = 15.0;
            let info_btn_h = 15.0;
            let info_btn_x = cx + card_w - info_btn_w - 3.0;
            let info_btn_y = cy + 3.0;
            let is_info_hover = mx >= info_btn_x && mx <= info_btn_x + info_btn_w && my >= info_btn_y && my <= info_btn_y + info_btn_h;

            let bg_color = if !is_unlocked {
                Color::new(0.08, 0.08, 0.11, 0.95)
            } else if can_afford {
                if card_hover && !is_info_hover { Color::new(0.16, 0.38, 0.22, 0.98) } else { Color::new(0.11, 0.28, 0.16, 0.95) }
            } else {
                Color::new(0.18, 0.11, 0.13, 0.95)
            };

            let border_color = if !is_unlocked {
                Color::new(0.30, 0.28, 0.35, 0.70)
            } else if card_hover && !is_info_hover {
                GOLD
            } else if can_afford {
                GREEN
            } else {
                Color::new(0.8, 0.3, 0.3, 0.9)
            };

            // Фон карты
            draw_rectangle(cx, cy, card_w, card_h, bg_color);
            draw_rectangle_lines(cx, cy, card_w, card_h, if card_hover && !is_info_hover { 2.2 } else { 1.2 }, border_color);

            // Верхняя плашка: Горячая клавиша [Q..F] слева
            let hotkey_badge = format!("[{}]", mob_keys[i]);
            fonts.draw_heading(&hotkey_badge, cx + 4.0, cy + 13.0, 10.5, if is_unlocked { GOLD } else { GRAY });

            // Кнопка [i] справа вверху (только для открытых мобов)
            if is_unlocked {
                draw_rectangle(info_btn_x, info_btn_y, info_btn_w, info_btn_h, if is_info_hover { Color::new(0.35, 0.15, 0.45, 0.95) } else { Color::new(0.12, 0.12, 0.18, 0.85) });
                draw_rectangle_lines(info_btn_x, info_btn_y, info_btn_w, info_btn_h, 1.0, if is_info_hover { GOLD } else { GRAY });
                fonts.draw_heading("i", info_btn_x + 5.0, info_btn_y + 11.0, 10.5, if is_info_hover { GOLD } else { LIGHTGRAY });
            }

            // Иконка / Спрайт моба
            if let Some(mob_tex) = textures.get_mob_texture(tier, 0) {
                let p_size = 28.0;
                let (pw, ph) = if tier == 2 || tier == 10 { (p_size * 1.3, p_size * 0.65) } else { (p_size, p_size) };
                let px = cx + (card_w - pw) / 2.0;
                let py = cy + 15.0 + (p_size - ph) / 2.0;
                draw_texture_ex(
                    mob_tex,
                    px,
                    py,
                    if is_unlocked { WHITE } else { Color::new(0.28, 0.28, 0.32, 0.50) },
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(pw, ph)),
                        ..Default::default()
                    },
                );
            }

            // Имя моба
            let short_name = match tier {
                1 => "Скелеты",
                2 => "Гончие",
                3 => "Культисты",
                4 => "Големы",
                5 => "Камикадзе",
                6 => "Демоны",
                7 => "Элементали",
                8 => "Жнецы",
                9 => "Фаланга",
                10 => "Ад.Гончие",
                11 => "Архикультист",
                12 => "Колоссы",
                13 => "Мучители",
                14 => "Ловчие",
                15 => "Архонты",
                16 => "Аватар",
                17 => "Пустота",
                18 => "Инферно",
                _ => "Герой",
            };
            let nw = fonts.measure_heading(short_name, 10.0).width;
            fonts.draw_heading(short_name, cx + (card_w - nw) / 2.0, cy + 49.0, 10.0, if is_unlocked { WHITE } else { GRAY });

            let altar_cooldown = director.altar_cooldown_for_tier(tier);
            let altar_cost = crate::content::structures::altar_cost(tier);
            let altar_affordable = director.points >= altar_cost;
            let altar_can_build = is_unlocked && altar_affordable && altar_cooldown <= 0.0;
            let altar_button = Rect::new(cx + 3.0, cy + 93.0, card_w - 6.0, 49.0);
            let altar_button_hover = mx >= altar_button.x
                && mx <= altar_button.x + altar_button.w
                && my >= altar_button.y
                && my <= altar_button.y + altar_button.h;

            if !is_unlocked {
                // ЕДИНЫЙ БОЛЬШОЙ ЗАМОК НА ВСЮ КАРТОЧКУ (объединяет моба и алтарь, без дублирования)
                let lock_panel = Rect::new(cx + 3.0, cy + 54.0, card_w - 6.0, card_h - 58.0);
                draw_rectangle(lock_panel.x, lock_panel.y, lock_panel.w, lock_panel.h, Color::new(0.06, 0.05, 0.08, 0.88));
                draw_rectangle_lines(lock_panel.x, lock_panel.y, lock_panel.w, lock_panel.h, 1.0, Color::new(0.38, 0.20, 0.22, 0.70));

                // Крупный навесной замок по центру
                draw_pixel_lock_large(lock_panel.x + lock_panel.w / 2.0, lock_panel.y + 22.0, Color::new(0.90, 0.35, 0.35, 1.0));

                // Требуемое число душ
                let cost_num = format!("{:.0}", config.unlock_spent_threshold);
                let nw = fonts.measure_heading(&cost_num, 11.0).width;
                fonts.draw_heading(&cost_num, lock_panel.x + (lock_panel.w - nw) / 2.0, lock_panel.y + 46.0, 11.0, RED);

                // Аккуратный затемненный бейдж хоткея алтаря внизу
                let pill_w = 46.0;
                let pill_h = 14.0;
                let pill_x = lock_panel.x + (lock_panel.w - pill_w) / 2.0;
                let pill_y = lock_panel.y + 60.0;
                draw_rectangle(pill_x, pill_y, pill_w, pill_h, Color::new(0.08, 0.08, 0.10, 0.85));
                draw_rectangle_lines(pill_x, pill_y, pill_w, pill_h, 1.0, Color::new(0.28, 0.28, 0.32, 0.50));
                if let Some(ref alt_tex) = textures.altar {
                    draw_texture_ex(
                        alt_tex,
                        pill_x + 3.0,
                        pill_y + 1.5,
                        Color::new(0.4, 0.4, 0.45, 0.7),
                        DrawTextureParams {
                            dest_size: Some(Vec2::splat(11.0)),
                            ..Default::default()
                        },
                    );
                }
                let hk_badge = format!("Ctrl+{}", mob_keys[i]);
                let hk_w = fonts.measure_text(&hk_badge, 8.5).width;
                fonts.draw_text(&hk_badge, pill_x + 16.0 + (pill_w - 18.0 - hk_w) / 2.0, pill_y + 10.5, 8.5, GRAY);
            } else {
                // ОТКРЫТЫЙ ТИР: Стоимость призыва моба, роль, подсказка и кнопка алтаря
                let cost_val = format!("{:.0}", config.cost);
                let cost_num_w = fonts.measure_heading(&cost_val, 10.5).width;
                let icon_s = 11.0;
                let total_cost_w = cost_num_w + icon_s + 2.0;
                let start_cost_x = cx + (card_w - total_cost_w) / 2.0;
                let cost_col = if can_afford { GOLD } else { Color::new(1.0, 0.4, 0.4, 1.0) };

                if let Some(sm_tex) = textures.soul_ethereal_sm.as_ref() {
                    draw_texture_ex(
                        sm_tex,
                        start_cost_x,
                        cy + 54.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(icon_s, icon_s)),
                            ..Default::default()
                        },
                    );
                }
                fonts.draw_heading(&cost_val, start_cost_x + icon_s + 2.0, cy + 63.5, 10.5, cost_col);

                // Роль
                let role_str = match tier {
                    1 => "x8 бойцов",
                    2 => "x5 скор.",
                    3 => "Дальники",
                    4 => "Танк 380",
                    5 => "Взрыв 220",
                    6 => "Телепорт",
                    7 => "Магия огня",
                    8 => "Титан коса",
                    9 => "x8 щитов",
                    10 => "x5 ад.псов",
                    11 => "Снайперы",
                    12 => "Танк 1450",
                    13 => "Взрыв 550",
                    14 => "Ловчие фазы",
                    15 => "Архонты огня",
                    16 => "Колосс 8500",
                    17 => "Герой 14K HP",
                    18 => "Герой 18K HP",
                    _ => "Герой",
                };
                let rw = fonts.measure_text(role_str, 9.0).width;
                fonts.draw_text(role_str, cx + (card_w - rw) / 2.0, cy + 74.5, 9.0, SKYBLUE);

                // Строка призыва
                if is_shift {
                    let call_hint = "ВСЕ [Shift]";
                    let ch_w = fonts.measure_text(call_hint, 8.5).width;
                    let spark_x = cx + (card_w - ch_w) / 2.0 - 6.0;
                    draw_pixel_lightning(spark_x, cy + 77.0, YELLOW);
                    fonts.draw_text(call_hint, spark_x + 8.0, cy + 85.5, 8.5, if card_hover { YELLOW } else { GOLD });
                } else {
                    let call_hint = "Призыв";
                    let ch_w = fonts.measure_text(call_hint, 8.5).width;
                    fonts.draw_text(call_hint, cx + (card_w - ch_w) / 2.0, cy + 85.5, 8.5, if card_hover { YELLOW } else { LIGHTGRAY });
                }

                if tier >= 17 {
                    let hero_bar = Rect::new(cx + 3.0, cy + 93.0, card_w - 6.0, 49.0);
                    let (h_bg, h_border, h_txt, h_col) = if on_hero_cd {
                        (Color::new(0.20, 0.12, 0.05, 0.95), ORANGE, format!("КД: {:.0}с", director.hero_cooldown), GOLD)
                    } else if any_obelisk_active {
                        (Color::new(0.25, 0.05, 0.08, 0.95), RED, "ОБЕЛИСК!".to_string(), Color::new(1.0, 0.4, 0.4, 1.0))
                    } else if hero_alive {
                        (Color::new(0.20, 0.08, 0.18, 0.95), PURPLE, "В БОЮ!".to_string(), Color::new(0.9, 0.5, 1.0, 1.0))
                    } else if can_afford {
                        (Color::new(0.12, 0.32, 0.18, 0.95), GREEN, "ГОТОВ".to_string(), WHITE)
                    } else {
                        (Color::new(0.12, 0.12, 0.16, 0.90), GRAY, "МАЛО ДУШ".to_string(), LIGHTGRAY)
                    };
                    draw_rectangle(hero_bar.x, hero_bar.y, hero_bar.w, hero_bar.h, h_bg);
                    draw_rectangle_lines(hero_bar.x, hero_bar.y, hero_bar.w, hero_bar.h, 1.2, h_border);
                    let htw = fonts.measure_heading(&h_txt, 10.0).width;
                    fonts.draw_heading(&h_txt, hero_bar.x + (hero_bar.w - htw) / 2.0, hero_bar.y + 28.0, 10.0, h_col);
                } else {
                    // Кнопка алтаря
                    let hotkey_badge = format!("Ctrl+{}", mob_keys[i]);
                    if altar_button_hover {
                        interaction.is_mouse_over_ui = true;
                        hovered_tooltip = Some(HoverTooltip::AltarButton {
                            tier,
                            hotkey: hotkey_badge.clone(),
                            bx: altar_button.x,
                            by: altar_button.y,
                        });
                        if is_mouse_button_pressed(MouseButton::Left) && altar_can_build {
                            interaction.start_altar_placement = true;
                            interaction.altar_tier = Some(tier);
                        }
                    }

                    let pulse = (macroquad::time::get_time() * 4.5).sin() as f32 * 0.5 + 0.5;

                    if altar_can_build {
                        // Неоновый ореол готовности
                        draw_rectangle(
                            altar_button.x - 1.0,
                            altar_button.y - 1.0,
                            altar_button.w + 2.0,
                            altar_button.h + 2.0,
                            Color::new(0.95, 0.25, 0.45, 0.20 + 0.25 * pulse),
                        );
                        draw_rectangle(
                            altar_button.x,
                            altar_button.y,
                            altar_button.w,
                            altar_button.h,
                            Color::new(0.24 + 0.12 * pulse, 0.05, 0.14 + 0.08 * pulse, 0.98),
                        );
                        let border_col = if altar_button_hover { GOLD } else { Color::new(1.0, 0.40 + 0.40 * pulse, 0.50, 1.0) };
                        draw_rectangle_lines(altar_button.x, altar_button.y, altar_button.w, altar_button.h, if altar_button_hover { 2.0 } else { 1.5 }, border_col);
                    } else if altar_cooldown > 0.0 {
                        draw_rectangle(altar_button.x, altar_button.y, altar_button.w, altar_button.h, Color::new(0.08, 0.05, 0.06, 0.96));
                        draw_rectangle_lines(altar_button.x, altar_button.y, altar_button.w, altar_button.h, 1.0, Color::new(0.65, 0.25, 0.20, 0.85));
                    } else {
                        draw_rectangle(altar_button.x, altar_button.y, altar_button.w, altar_button.h, Color::new(0.08, 0.07, 0.10, 0.95));
                        draw_rectangle_lines(altar_button.x, altar_button.y, altar_button.w, altar_button.h, 1.0, Color::new(0.40, 0.32, 0.40, 0.75));
                    }

                    // Строка 1: Иконка Алтаря + бейдж Ctrl+Q..F (без слова "АЛТАРЬ"!)
                    let row1_y = altar_button.y + 3.0;
                    if let Some(ref alt_tex) = textures.altar {
                        let icon_col = if altar_can_build {
                            Color::new(1.0, 0.85 + 0.15 * pulse, 0.85 + 0.15 * pulse, 1.0)
                        } else {
                            Color::new(0.8, 0.45, 0.45, 0.85)
                        };
                        draw_texture_ex(
                            alt_tex,
                            altar_button.x + 3.0,
                            row1_y,
                            icon_col,
                            DrawTextureParams {
                                dest_size: Some(Vec2::splat(14.0)),
                                ..Default::default()
                            },
                        );
                    }
                    let pill_x = altar_button.x + 19.0;
                    let pill_y = row1_y;
                    let pill_w = altar_button.w - 22.0;
                    let pill_h = 14.0;
                    let pill_bg = if altar_can_build {
                        Color::new(0.12, 0.06, 0.15, 0.95)
                    } else {
                        Color::new(0.08, 0.08, 0.10, 0.90)
                    };
                    let pill_border = if altar_can_build {
                        if altar_button_hover { GOLD } else { Color::new(0.85, 0.65, 0.2, 0.85) }
                    } else {
                        Color::new(0.35, 0.35, 0.40, 0.60)
                    };
                    draw_rectangle(pill_x, pill_y, pill_w, pill_h, pill_bg);
                    draw_rectangle_lines(pill_x, pill_y, pill_w, pill_h, 1.0, pill_border);
                    let hk_w = fonts.measure_heading(&hotkey_badge, 9.0).width;
                    let hk_col = if altar_can_build { GOLD } else { LIGHTGRAY };
                    fonts.draw_heading(&hotkey_badge, pill_x + (pill_w - hk_w) / 2.0, pill_y + 10.5, 9.0, hk_col);

                    // Строка 2: Отдельная строка статуса / таймера / стоимости
                    if altar_can_build {
                        let cost_str = format!("{:.0}", altar_cost);
                        let c_w = fonts.measure_heading(&cost_str, 10.0).width;
                        let s_icon_s = 10.0;
                        let g_w = fonts.measure_heading("ГОТОВ", 9.0).width;
                        let total_r2 = s_icon_s + 2.0 + c_w + 4.0 + g_w;
                        let r2_x = altar_button.x + (altar_button.w - total_r2) / 2.0;

                        if let Some(sm_tex) = textures.soul_ethereal_sm.as_ref() {
                            draw_texture_ex(
                                sm_tex,
                                r2_x,
                                altar_button.y + 21.0,
                                WHITE,
                                DrawTextureParams {
                                    dest_size: Some(Vec2::new(s_icon_s, s_icon_s)),
                                    ..Default::default()
                                },
                            );
                        }
                        fonts.draw_heading(&cost_str, r2_x + s_icon_s + 2.0, altar_button.y + 30.0, 10.0, GOLD);
                        fonts.draw_heading("ГОТОВ", r2_x + s_icon_s + 2.0 + c_w + 4.0, altar_button.y + 30.0, 9.0, GREEN);

                        let hint_str = "[ЛКМ] Спавн";
                        let hw = fonts.measure_text(hint_str, 8.0).width;
                        fonts.draw_text(hint_str, altar_button.x + (altar_button.w - hw) / 2.0, altar_button.y + 42.0, 8.0, if altar_button_hover { YELLOW } else { SKYBLUE });
                    } else if altar_cooldown > 0.0 {
                        let cd_str = format!("Откат: {:.1}с", altar_cooldown);
                        let cdw = fonts.measure_heading(&cd_str, 9.5).width;
                        fonts.draw_heading(&cd_str, altar_button.x + (altar_button.w - cdw) / 2.0, altar_button.y + 30.0, 9.5, Color::new(1.0, 0.45, 0.35, 1.0));

                        // Полоса прогресса отката
                        let max_cd = crate::content::structures::altar_cooldown(tier);
                        let prog = (1.0 - altar_cooldown / max_cd).clamp(0.0, 1.0);
                        let bar_x = altar_button.x + 4.0;
                        let bar_y = altar_button.y + 37.0;
                        let bar_w = altar_button.w - 8.0;
                        let bar_h = 4.0;
                        draw_rectangle(bar_x, bar_y, bar_w, bar_h, Color::new(0.20, 0.08, 0.08, 0.95));
                        draw_rectangle(bar_x, bar_y, bar_w * prog, bar_h, Color::new(0.95, 0.35, 0.30, 0.95));
                        draw_rectangle_lines(bar_x, bar_y, bar_w, bar_h, 1.0, Color::new(0.50, 0.20, 0.20, 0.80));
                    } else {
                        // Не хватает душ: только число стоимости красным цветом с иконкой душ (без текста "не хватает душ"!)
                        let cost_str = format!("{:.0}", altar_cost);
                        let c_w = fonts.measure_heading(&cost_str, 10.5).width;
                        let s_icon_s = 10.0;
                        let total_r2 = s_icon_s + 2.0 + c_w;
                        let r2_x = altar_button.x + (altar_button.w - total_r2) / 2.0;

                        if let Some(sm_tex) = textures.soul_ethereal_sm.as_ref() {
                            draw_texture_ex(
                                sm_tex,
                                r2_x,
                                altar_button.y + 26.0,
                                WHITE,
                                DrawTextureParams {
                                    dest_size: Some(Vec2::new(s_icon_s, s_icon_s)),
                                    ..Default::default()
                                },
                            );
                        }
                        fonts.draw_heading(&cost_str, r2_x + s_icon_s + 2.0, altar_button.y + 35.0, 10.5, Color::new(1.0, 0.4, 0.4, 1.0));
                    }
                }
            }

            // Клик по карте:
            // 1. Клик по кнопке [i] -> открывает подробное Досье
            if is_mouse_button_pressed(MouseButton::Left) && is_info_hover && is_unlocked {
                interaction.open_dossier_tier = Some(tier);
            }
            // 2. Клик ЛКМ по самой карте -> ПРИЗЫВАЕТ МОБА!
            else if (is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right))
                && card_hover && !is_info_hover && (tier >= 17 || !altar_button_hover) && is_unlocked && can_afford
            {
                if tier >= 17 {
                    let hero_can_spawn = !any_obelisk_active && !hero_alive && !on_hero_cd;
                    if hero_can_spawn {
                        interaction.direct_summon_tier = Some(tier);
                    }
                } else if is_shift {
                    interaction.direct_summon_max_tier = Some(tier);
                } else {
                    interaction.direct_summon_tier = Some(tier);
                }
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
                ("[МЕДИТАЦИЯ ПОКОЯ] (+2% HP/с, +30 XP/с)".to_string(), GREEN)
            } else if world.boss_dash_cooldown <= 0.0 {
                ("[РЫВОК ГОТОВ] (8-лучевой Dash)".to_string(), GOLD)
            } else {
                ("[АКТИВНЫЙ БОЙ] (Кайтинг орд)".to_string(), SKYBLUE)
            };

            let ai_str = if let Some(ref model) = world.boss_model {
                let syn = model.total_synapses;
                let desc = if syn > 0 {
                    format!("[НЕЙРОСЕТЬ] Глубокий ИИ ({} параметров, 166 сенсоров, Пок. {})", syn, model.generation)
                } else {
                    format!("[ОБУЧЕННЫЙ ИИ] Модель (Поколение {}, Фитнес {:.0})", model.generation, model.fitness)
                };
                (desc, GOLD)
            } else {
                ("[БАЗОВЫЙ ИИ] Потенциальные поля".to_string(), LIGHTGRAY)
            };

            let lines = vec![
                ("[БОСС-ГЕРОЙ] АВТОНОМНЫЙ ПЕРСОНАЖ".to_string(), YELLOW),
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
                        ("[АЛТАРЬ КРОВИ] Инфернальная структура".to_string(), PURPLE),
                        (format!("Здоровье: {:.0} / {:.0} HP | Броня: {:.0}", st.health.current, st.health.max, st.resistances.flat_armor), WHITE),
                        ("+3.0 Очков Душ каждые 2 сек к доходу Директора".to_string(), GREEN),
                        ("Аура Крови: лечит союзных мобов на +35 HP каждые 3 сек в радиусе 180px".to_string(), SKYBLUE),
                        ("[УЯЗВИМОСТЬ] Разрушаем: Босс атакует и уничтожает алтарь!".to_string(), ORANGE),
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
                            (format!("[ОТРЯД] {} (Тир {})", config.name, mob.tier), WHITE),
                            (format!("HP: {:.0} / {:.0} | Скорость: {:.0} | Броня: {:.0}", mob.health.current, mob.health.max, mob.speed, mob.resistances.flat_armor), LIGHTGRAY),
                        ];

                        if mob.status.burn_timer > 0.0 {
                            lines.push((format!("[ОЖОГ] -{:.0} HP/сек", mob.status.burn_dps), ORANGE));
                        }
                        if mob.status.is_frozen() {
                            lines.push(("[ЗАМОРОЗКА] Полностью обездвижен".to_string(), SKYBLUE));
                        } else if mob.status.slow_timer > 0.0 {
                            lines.push(("[ЗАМЕДЛЕНИЕ] Скорость снижена на 50%".to_string(), SKYBLUE));
                        }
                        if mob.status.poison_timer > 0.0 {
                            lines.push((format!("[ТОКСИН] -{:.0} HP/сек", mob.status.poison_dps), LIME));
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
            HoverTooltip::WallWidget(ww_x, ww_y) => {
                draw_wall_widget_tooltip(ww_x, ww_y, screen_w, fonts);
            }
            HoverTooltip::SoulsPlaque(px, py) => {
                if let Some(director) = world.directors.first() {
                    draw_souls_plaque_tooltip(px, py, screen_w, fonts, world, director);
                }
            }
            HoverTooltip::AltarButton { tier, hotkey, bx, by } => {
                if let Some(director) = world.directors.first() {
                    draw_altar_button_tooltip(tier, &hotkey, bx, by, screen_w, fonts, director);
                }
            }
        }
    }

    interaction
}

pub fn render_boss_ui(
    world: &World,
    clock: &GameClock,
    fonts: &GameFonts,
    textures: &GameTextures,
    is_muted: bool,
    current_track: &str,
    control_scheme: crate::render::main_menu::BossControlScheme,
) -> UiInteraction {
    let w = screen_width();
    let h = screen_height();
    let (mx, my) = mouse_position();
    let mut interaction = UiInteraction::default();
    let mut hovered_tooltip: Option<HoverTooltip> = None;

    // 1. Верхняя панель характеристик Босса (центрированная)
    let panel_w = 680.0f32.min(w - 240.0);
    let panel_x = (w - panel_w) / 2.0;
    let panel_y = 12.0;
    let panel_h = 108.0;

    draw_rectangle(panel_x, panel_y, panel_w, panel_h, Color::new(0.04, 0.045, 0.075, 0.95));
    draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.5, Color::new(0.45, 0.50, 0.65, 0.88));

    // Строка 1: Имя, Уровень, HP полоса, XP полоса
    fonts.draw_heading(&world.boss_name, panel_x + 12.0, panel_y + 17.0, 13.0, GOLD);
    let lvl_str = format!("Ур. {}", world.boss_inventory.level);
    fonts.draw_heading(&lvl_str, panel_x + 190.0, panel_y + 17.0, 12.0, WHITE);

    // HP полоса
    let hp_bar_w = 200.0;
    let hp_bar_x = panel_x + 245.0;
    let hp_bar_y = panel_y + 6.0;
    let hp_frac = (world.boss_health.current / world.boss_health.max.max(1.0)).clamp(0.0, 1.0);
    draw_rectangle(hp_bar_x, hp_bar_y, hp_bar_w, 13.0, Color::new(0.20, 0.06, 0.08, 0.95));
    draw_rectangle(hp_bar_x, hp_bar_y, hp_bar_w * hp_frac, 13.0, Color::new(0.80, 0.18, 0.22, 1.0));
    draw_rectangle_lines(hp_bar_x, hp_bar_y, hp_bar_w, 13.0, 1.0, Color::new(0.5, 0.2, 0.2, 0.9));
    let hp_txt = format!("{:.0} / {:.0} HP", world.boss_health.current, world.boss_health.max);
    let hp_tw = fonts.measure_text(&hp_txt, 10.0).width;
    fonts.draw_text(&hp_txt, hp_bar_x + (hp_bar_w - hp_tw) / 2.0, hp_bar_y + 10.0, 10.0, WHITE);

    // XP полоса
    let xp_bar_w = 200.0;
    let xp_bar_x = panel_x + 460.0;
    let xp_bar_y = panel_y + 6.0;
    let xp_frac = (world.boss_inventory.current_xp / world.boss_inventory.xp_to_next_level.max(1.0)).clamp(0.0, 1.0);
    draw_rectangle(xp_bar_x, xp_bar_y, xp_bar_w, 13.0, Color::new(0.08, 0.12, 0.22, 0.95));
    draw_rectangle(xp_bar_x, xp_bar_y, xp_bar_w * xp_frac, 13.0, SKYBLUE);
    draw_rectangle_lines(xp_bar_x, xp_bar_y, xp_bar_w, 13.0, 1.0, Color::new(0.2, 0.4, 0.7, 0.9));
    let xp_txt = format!("{:.0} / {:.0} XP", world.boss_inventory.current_xp, world.boss_inventory.xp_to_next_level);
    let xp_tw = fonts.measure_text(&xp_txt, 10.0).width;
    fonts.draw_text(&xp_txt, xp_bar_x + (xp_bar_w - xp_tw) / 2.0, xp_bar_y + 10.0, 10.0, WHITE);

    // Строка 2: Броня с формулой физ. резиста, Рывок, Оповещение о Level-Up
    let armor_val = world.boss_resistances.flat_armor;
    let armor_pct = world.boss_resistances.physical_reduction_from_armor() * 100.0;
    let armor_str = format!("Броня: {:.1} ({:.1}% физ. защиты)", armor_val, armor_pct);
    fonts.draw_text(&armor_str, panel_x + 12.0, panel_y + 35.0, 11.0, Color::new(0.85, 0.88, 0.95, 1.0));

    let dash_ready = world.boss_dash_cooldown <= 0.0;
    let dash_str = if dash_ready { "Рывок: ГОТОВ [Пробел]".to_string() } else { format!("Рывок: {:.1}с", world.boss_dash_cooldown) };
    fonts.draw_text(&format!("{}   T: {}", dash_str, clock.formatted_elapsed()), panel_x + 245.0, panel_y + 35.0, 11.0, if dash_ready { GOLD } else { Color::new(0.7, 0.7, 0.8, 1.0) });

    if world.boss_pending_level_ups > 0 {
        let pulse = (get_time() as f32 * 6.0).sin().abs();
        let lvlup_col = Color::new(1.0, 0.7 + pulse * 0.3, 0.1, 1.0);
        fonts.draw_heading(&format!("LEVEL UP! (x{})", world.boss_pending_level_ups), panel_x + 460.0, panel_y + 35.0, 11.5, lvlup_col);
    }

    // Строка 3: Инвентарь Оружия (6 слотов) и Пассивок (6 слотов)
    let w_slot_s = 40.0;
    let w_slot_gap = 4.0;
    let w_start_x = panel_x + 12.0;
    let w_y = panel_y + 44.0;

    fonts.draw_heading("ОРУЖИЕ", w_start_x, w_y + 10.0, 9.5, SKYBLUE);

    let slots_offset_x = w_start_x + 52.0;
    for i in 0..6 {
        let sx = slots_offset_x + (i as f32) * (w_slot_s + w_slot_gap);
        let is_hover = mx >= sx && mx <= sx + w_slot_s && my >= w_y && my <= w_y + w_slot_s;
        if is_hover { interaction.is_mouse_over_ui = true; }

        draw_rectangle(sx, w_y, w_slot_s, w_slot_s, Color::new(0.05, 0.06, 0.09, 0.95));

        if let Some(w) = world.boss_inventory.weapons.get(i) {
            let (border_col, bg_col) = if w.is_evolved {
                (GOLD, Color::new(0.40, 0.22, 0.05, 0.9))
            } else {
                (Color::new(0.3, 0.6, 0.9, 0.9), Color::new(0.08, 0.18, 0.32, 0.9))
            };
            draw_rectangle(sx + 2.0, w_y + 2.0, w_slot_s - 4.0, w_slot_s - 4.0, bg_col);
            draw_rectangle_lines(sx, w_y, w_slot_s, w_slot_s, if w.is_evolved { 2.0 } else { 1.5 }, border_col);

            if let Some(icon) = textures.get_weapon_icon(w.kind) {
                draw_texture_ex(
                    icon,
                    sx + 3.0,
                    w_y + 3.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(w_slot_s - 6.0, w_slot_s - 6.0)),
                        ..Default::default()
                    },
                );
            }

            // Маска перезарядки оружия (Cooldown sweep)
            let cd_ratio = (w.cooldown_timer / w.base_cooldown.max(0.05)).clamp(0.0, 1.0);
            if cd_ratio > 0.0 {
                let mask_h = (w_slot_s - 4.0) * cd_ratio;
                draw_rectangle(sx + 2.0, w_y + 2.0 + (w_slot_s - 4.0 - mask_h), w_slot_s - 4.0, mask_h, Color::new(0.0, 0.0, 0.0, 0.55));
            }

            // Бейдж уровня
            let lvl_tag = if w.limit_break_level > 0 {
                format!("+{}", w.limit_break_level)
            } else if w.is_evolved {
                "MAX".to_string()
            } else {
                format!("{}", w.level)
            };
            let tag_w = fonts.measure_heading(&lvl_tag, 9.5).width + 3.0;
            draw_rectangle(sx + w_slot_s - tag_w, w_y + w_slot_s - 10.0, tag_w, 10.0, Color::new(0.0, 0.0, 0.0, 0.85));
            fonts.draw_heading(&lvl_tag, sx + w_slot_s - tag_w + 1.0, w_y + w_slot_s - 1.5, 9.5, if w.is_evolved { GOLD } else { WHITE });

            if is_hover {
                hovered_tooltip = Some(HoverTooltip::Weapon(w.clone(), sx, w_y));
            }
        } else {
            draw_rectangle_lines(sx, w_y, w_slot_s, w_slot_s, 1.0, Color::new(0.2, 0.22, 0.28, 0.6));
            draw_line(sx + 8.0, w_y + 8.0, sx + w_slot_s - 8.0, w_y + w_slot_s - 8.0, 1.0, Color::new(0.25, 0.25, 0.35, 0.4));
        }
    }

    // Пассивные реликвии
    let p_start_x = slots_offset_x + 6.0 * (w_slot_s + w_slot_gap) + 12.0;
    fonts.draw_heading("РЕЛИКВИИ", p_start_x, w_y + 10.0, 9.5, PURPLE);

    let p_slots_x = p_start_x + 58.0;
    let p_slot_s = 28.0;
    let p_slot_gap = 4.0;
    for i in 0..6 {
        let sx = p_slots_x + (i as f32) * (p_slot_s + p_slot_gap);
        let sy = w_y + 6.0;
        let is_hover = mx >= sx && mx <= sx + p_slot_s && my >= sy && my <= sy + p_slot_s;
        if is_hover { interaction.is_mouse_over_ui = true; }

        draw_rectangle(sx, sy, p_slot_s, p_slot_s, Color::new(0.05, 0.04, 0.08, 0.95));

        if let Some(p) = world.boss_inventory.passives.get(i) {
            draw_rectangle(sx + 1.0, sy + 1.0, p_slot_s - 2.0, p_slot_s - 2.0, Color::new(0.26, 0.10, 0.34, 0.9));
            draw_rectangle_lines(sx, sy, p_slot_s, p_slot_s, 1.5, Color::new(0.75, 0.40, 0.90, 0.95));

            if let Some(icon) = textures.get_passive_icon(p.kind) {
                draw_texture_ex(
                    icon,
                    sx + 2.0,
                    sy + 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(p_slot_s - 4.0, p_slot_s - 4.0)),
                        ..Default::default()
                    },
                );
            }

            let lvl_tag = format!("{}", p.level);
            let tag_w = fonts.measure_heading(&lvl_tag, 9.0).width + 3.0;
            draw_rectangle(sx + p_slot_s - tag_w, sy + p_slot_s - 9.0, tag_w, 9.0, Color::new(0.0, 0.0, 0.0, 0.85));
            fonts.draw_heading(&lvl_tag, sx + p_slot_s - tag_w + 1.0, sy + p_slot_s - 1.0, 9.0, GOLD);

            if is_hover {
                hovered_tooltip = Some(HoverTooltip::Passive(p.clone(), sx, sy));
            }
        } else {
            draw_rectangle_lines(sx, sy, p_slot_s, p_slot_s, 1.0, Color::new(0.25, 0.20, 0.30, 0.5));
        }
    }

    // Кнопки Пауза и Звук в правом верхнем углу
    let pause = Rect::new(w - 180.0, 12.0, 165.0, 28.0);
    let sound = Rect::new(w - 180.0, 44.0, 165.0, 24.0);
    let pause_hover = contains_rect(pause, mx, my);
    let sound_hover = contains_rect(sound, mx, my);
    draw_rectangle(pause.x, pause.y, pause.w, pause.h, if pause_hover { Color::new(0.30, 0.19, 0.10, 1.0) } else { Color::new(0.10, 0.11, 0.16, 0.96) });
    draw_rectangle_lines(pause.x, pause.y, pause.w, pause.h, 1.0, if pause_hover { GOLD } else { GRAY });
    fonts.draw_heading("Пауза [Esc]", pause.x + 20.0, pause.y + 19.0, 11.0, WHITE);

    draw_rectangle(sound.x, sound.y, sound.w, sound.h, if sound_hover { Color::new(0.18, 0.20, 0.29, 1.0) } else { Color::new(0.08, 0.09, 0.14, 0.96) });
    draw_rectangle_lines(sound.x, sound.y, sound.w, sound.h, 1.0, if sound_hover { SKYBLUE } else { DARKGRAY });
    let sound_label = if is_muted { "Звук: ВЫКЛ [M]" } else { "Звук: ВКЛ [M]" };
    fonts.draw_heading(sound_label, sound.x + 12.0, sound.y + 17.0, 10.0, if is_muted { RED } else { GREEN });

    if is_mouse_button_pressed(MouseButton::Left) && pause_hover {
        interaction.clicked_pause = true;
    }
    if is_mouse_button_pressed(MouseButton::Left) && sound_hover {
        interaction.clicked_sound_toggle = true;
    }

    let mode_btn = Rect::new(w - 180.0, 72.0, 165.0, 24.0);
    let mode_hover = contains_rect(mode_btn, mx, my);
    draw_rectangle(mode_btn.x, mode_btn.y, mode_btn.w, mode_btn.h, if mode_hover { Color::new(0.24, 0.18, 0.28, 1.0) } else { Color::new(0.08, 0.09, 0.14, 0.96) });
    draw_rectangle_lines(mode_btn.x, mode_btn.y, mode_btn.w, mode_btn.h, 1.0, if mode_hover { GOLD } else { DARKGRAY });
    let mode_label = match control_scheme {
        crate::render::main_menu::BossControlScheme::DirectKeyboard => "WASD [F2]",
        crate::render::main_menu::BossControlScheme::MouseRts => "ПКМ RTS [F2]",
    };
    fonts.draw_heading(mode_label, mode_btn.x + 12.0, mode_btn.y + 17.0, 10.0, if mode_hover { GOLD } else { WHITE });

    if is_mouse_button_pressed(MouseButton::Left) && mode_hover {
        interaction.toggle_control_scheme = true;
    }

    interaction.is_mouse_over_ui = pause_hover || sound_hover || mode_hover || contains_rect(Rect::new(panel_x, panel_y, panel_w, panel_h), mx, my);

    if !current_track.is_empty() {
        fonts.draw_text(current_track, 18.0, h - 16.0, 10.0, Color::new(0.68, 0.70, 0.78, 0.82));
    }

    // Тултипы при наведении
    if let Some(tooltip) = hovered_tooltip {
        match tooltip {
            HoverTooltip::Weapon(w_slot, sx, sy) => draw_weapon_tooltip(&w_slot, sx, sy, w, fonts),
            HoverTooltip::Passive(p_slot, sx, sy) => draw_passive_tooltip(&p_slot, sx, sy, w, fonts),
            _ => {}
        }
    }

    // 2. МОДАЛЬНОЕ ОКНО LEVEL-UP: ВЫБОР ИЗ 3 КАРТ (КОГДА БОССОМ УПРАВЛЯЕТ ИГРОК)
    if let Some(ref options) = world.boss_level_up_options {
        interaction.is_mouse_over_ui = true;

        // Полупрозрачный фон
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.02, 0.02, 0.06, 0.88));

        let title = "НОВЫЙ УРОВЕНЬ! ВЫБЕРИТЕ УЛУЧШЕНИЕ";
        let title_y = panel_y + panel_h + 24.0;
        let tw = fonts.measure_heading(title, 22.0).width;
        fonts.draw_heading(title, (w - tw) / 2.0, title_y, 22.0, GOLD);

        let sub = "Нажмите [ 1 ], [ 2 ], [ 3 ] на клавиатуре или выберите карту кликом мыши";
        let sw = fonts.measure_text(sub, 12.0).width;
        fonts.draw_text(sub, (w - sw) / 2.0, title_y + 24.0, 12.0, Color::new(0.75, 0.78, 0.88, 1.0));

        let mut card_y = title_y + 54.0;
        if world.boss_pending_level_ups > 1 {
            let pend_str = format!("В очереди ещё уровней: {}", world.boss_pending_level_ups - 1);
            let pw = fonts.measure_heading(&pend_str, 12.0).width;
            fonts.draw_heading(&pend_str, (w - pw) / 2.0, title_y + 44.0, 12.0, ORANGE);
            card_y += 18.0;
        }

        let card_w = 236.0;
        let card_h = 320.0;
        let card_gap = 24.0;
        let count = options.len().min(4);
        let total_w = (count as f32) * card_w + ((count.saturating_sub(1)) as f32) * card_gap;
        let start_x = (w - total_w) / 2.0;

        for (idx, choice) in options.iter().take(4).enumerate() {
            let cx = start_x + (idx as f32) * (card_w + card_gap);
            let is_card_hover = mx >= cx && mx <= cx + card_w && my >= card_y && my <= card_y + card_h;
            let key_shortcut = match idx {
                0 => is_key_pressed(KeyCode::Key1),
                1 => is_key_pressed(KeyCode::Key2),
                2 => is_key_pressed(KeyCode::Key3),
                3 => is_key_pressed(KeyCode::Key4),
                _ => false,
            };

            // Распаковываем данные карты
            let (category, title_str, desc_str, lvl_tag, is_evo, wep_kind, pas_kind) = match choice {
                crate::ecs::components::inventory::PerkChoice::EvolveWeapon { to, .. } => {
                    let def = get_weapon_def(*to);
                    ("ЭВОЛЮЦИЯ СИНТЕЗА", def.name.to_string(), def.description.to_string(), "МАКСИМУМ".to_string(), true, Some(*to), None)
                }
                crate::ecs::components::inventory::PerkChoice::UpgradeWeapon(kind) => {
                    let def = get_weapon_def(*kind);
                    let existing = world.boss_inventory.weapons.iter().find(|w| w.kind == *kind);
                    let (cat, lvl) = if let Some(w) = existing {
                        ("УЛУЧШЕНИЕ ОРУЖИЯ", format!("Ур. {} -> {}", w.level, w.level + 1))
                    } else {
                        ("НОВОЕ ОРУЖИЕ", "НОВОЕ".to_string())
                    };
                    (cat, def.name.to_string(), def.description.to_string(), lvl, false, Some(*kind), None)
                }
                crate::ecs::components::inventory::PerkChoice::UpgradePassive(kind) => {
                    let def = get_passive_def(*kind);
                    let existing = world.boss_inventory.passives.iter().find(|p| p.kind == *kind);
                    let (cat, lvl) = if let Some(p) = existing {
                        ("УЛУЧШЕНИЕ РЕЛИКВИИ", format!("Ур. {} -> {}", p.level, p.level + 1))
                    } else {
                        ("НОВАЯ РЕЛИКВИЯ", "НОВАЯ".to_string())
                    };
                    (cat, def.name.to_string(), def.description.to_string(), lvl, false, None, Some(*kind))
                }
                crate::ecs::components::inventory::PerkChoice::LimitBreakDamage { name, .. } => {
                    ("ВОЗВЫШЕНИЕ", format!("Урон: {}", name), "+12% к урону этого оружия".to_string(), "LIMIT BREAK".to_string(), true, None, None)
                }
                crate::ecs::components::inventory::PerkChoice::LimitBreakCooldown { name, .. } => {
                    ("ВОЗВЫШЕНИЕ", format!("Скорость: {}", name), "-6% времени перезарядки".to_string(), "LIMIT BREAK".to_string(), true, None, None)
                }
                crate::ecs::components::inventory::PerkChoice::LimitBreakProjectiles { name, .. } => {
                    ("ВОЗВЫШЕНИЕ", format!("Снаряды: {}", name), "+1 снаряд к каждому залпу".to_string(), "LIMIT BREAK".to_string(), true, None, None)
                }
                crate::ecs::components::inventory::PerkChoice::LimitBreakTitanVigor => {
                    ("ВОЗВЫШЕНИЕ", "Сила Титана".to_string(), "+150 Макс HP, +25% эффективности лечения".to_string(), "LIMIT BREAK".to_string(), true, None, None)
                }
                crate::ecs::components::inventory::PerkChoice::LimitBreakAdaptiveArmor => {
                    ("ВОЗВЫШЕНИЕ", "Адаптивная Броня".to_string(), "+2.0 Брони, +3% ко всем сопротивлениям".to_string(), "LIMIT BREAK".to_string(), true, None, None)
                }
                crate::ecs::components::inventory::PerkChoice::LimitBreakPredatorSpeed => {
                    ("ВОЗВЫШЕНИЕ", "Скорость Хищника".to_string(), "+6% к скорости перемещения".to_string(), "LIMIT BREAK".to_string(), true, None, None)
                }
            };

            // Фон карты
            let card_bg = if is_card_hover {
                Color::new(0.14, 0.12, 0.22, 0.98)
            } else {
                Color::new(0.07, 0.07, 0.12, 0.96)
            };
            let border_col = if is_card_hover {
                GOLD
            } else if is_evo {
                Color::new(0.9, 0.6, 0.2, 0.9)
            } else if wep_kind.is_some() {
                Color::new(0.35, 0.60, 0.85, 0.85)
            } else {
                Color::new(0.65, 0.35, 0.85, 0.85)
            };

            draw_rectangle(cx, card_y, card_w, card_h, card_bg);
            draw_rectangle_lines(cx, card_y, card_w, card_h, if is_card_hover { 2.5 } else { 1.5 }, border_col);

            // Бейдж клавиши [ 1 / 2 / 3 ]
            let hotkey_badge = format!("[ {} ]", idx + 1);
            let hkw = fonts.measure_heading(&hotkey_badge, 11.0).width;
            draw_rectangle(cx + card_w - hkw - 12.0, card_y + 8.0, hkw + 8.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.7));
            fonts.draw_heading(&hotkey_badge, cx + card_w - hkw - 8.0, card_y + 20.0, 11.0, if is_card_hover { GOLD } else { WHITE });

            // Категория
            fonts.draw_heading(category, cx + 12.0, card_y + 20.0, 10.0, if is_evo { GOLD } else if wep_kind.is_some() { SKYBLUE } else { PURPLE });

            // Иконка по центру
            let icon_box_s = 64.0;
            let icon_box_x = cx + (card_w - icon_box_s) / 2.0;
            let icon_box_y = card_y + 36.0;
            draw_rectangle(icon_box_x, icon_box_y, icon_box_s, icon_box_s, Color::new(0.04, 0.04, 0.08, 0.9));
            draw_rectangle_lines(icon_box_x, icon_box_y, icon_box_s, icon_box_s, 1.2, border_col);

            if let Some(wkind) = wep_kind {
                if let Some(tex) = textures.get_weapon_icon(wkind) {
                    draw_texture_ex(
                        tex,
                        icon_box_x + 4.0,
                        icon_box_y + 4.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(icon_box_s - 8.0, icon_box_s - 8.0)),
                            ..Default::default()
                        },
                    );
                }
            } else if let Some(pkind) = pas_kind {
                if let Some(tex) = textures.get_passive_icon(pkind) {
                    draw_texture_ex(
                        tex,
                        icon_box_x + 4.0,
                        icon_box_y + 4.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(icon_box_s - 8.0, icon_box_s - 8.0)),
                            ..Default::default()
                        },
                    );
                }
            } else {
                draw_circle(icon_box_x + icon_box_s * 0.5, icon_box_y + icon_box_s * 0.5, 14.0, GOLD);
                draw_circle_lines(icon_box_x + icon_box_s * 0.5, icon_box_y + icon_box_s * 0.5, 14.0, 2.0, WHITE);
            }

            // Название
            let title_w = fonts.measure_heading(&title_str, 13.0).width;
            fonts.draw_heading(&title_str, (cx + (card_w - title_w) / 2.0).max(cx + 8.0), icon_box_y + icon_box_s + 20.0, 13.0, WHITE);

            // Бейдж уровня
            let lw = fonts.measure_heading(&lvl_tag, 10.5).width;
            fonts.draw_heading(&lvl_tag, cx + (card_w - lw) / 2.0, icon_box_y + icon_box_s + 38.0, 10.5, if is_evo { GOLD } else { SKYBLUE });

            // Описание (многострочное)
            let mut line_y = icon_box_y + icon_box_s + 56.0;
            for chunk in desc_str.split('\n') {
                for line in wrap_text_lines(chunk, card_w - 24.0, 10.0, fonts) {
                    fonts.draw_text(&line, cx + 12.0, line_y, 10.0, Color::new(0.80, 0.82, 0.90, 0.95));
                    line_y += 13.0;
                }
            }

            // Кнопка ВЫБРАТЬ внизу карты
            let btn_h = 28.0;
            let btn_y = card_y + card_h - btn_h - 10.0;
            let btn_x = cx + 16.0;
            let btn_w = card_w - 32.0;
            let btn_bg = if is_card_hover { Color::new(0.55, 0.35, 0.12, 0.95) } else { Color::new(0.16, 0.18, 0.26, 0.9) };
            draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_bg);
            draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, 1.2, if is_card_hover { GOLD } else { DARKGRAY });
            let pick_txt = format!("ВЫБРАТЬ [{}]", idx + 1);
            let ptw = fonts.measure_heading(&pick_txt, 11.0).width;
            fonts.draw_heading(&pick_txt, btn_x + (btn_w - ptw) / 2.0, btn_y + 19.0, 11.0, if is_card_hover { GOLD } else { WHITE });

            if (is_card_hover && is_mouse_button_pressed(MouseButton::Left)) || key_shortcut {
                interaction.chosen_perk = Some(choice.clone());
                interaction.chosen_perk_index = Some(idx as u8);
            }
        }
    }

    interaction
}

fn wrap_text_lines(text: &str, max_width: f32, font_size: f32, fonts: &GameFonts) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let words: Vec<&str> = paragraph.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        let mut cur_line = String::new();
        for word in words {
            let candidate = if cur_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", cur_line, word)
            };
            if fonts.measure_text(&candidate, font_size).width <= max_width {
                cur_line = candidate;
            } else {
                if !cur_line.is_empty() {
                    lines.push(cur_line);
                }
                cur_line = word.to_string();
            }
        }
        if !cur_line.is_empty() {
            lines.push(cur_line);
        }
    }
    lines
}

fn contains_rect(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
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

    let mini_w = 185.0;
    let mini_h = 124.0; // Соотношение 3600 / 2400 = 1.5
    let mini_x = 20.0;
    let mini_y = screen_h - mini_h - 18.0;

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

    // Структуры (Алтари, Стены, Обелиски)
    for st in &world.structures {
        let sp = to_mini(st.pos);
        match st.structure_type {
            altar_type if crate::content::structures::is_altar_structure_type(altar_type) => {
                if st.health.is_alive() {
                    draw_circle(sp.x, sp.y, 3.5, PURPLE);
                    draw_circle_lines(sp.x, sp.y, 3.5, 1.0, WHITE);
                }
            }
            crate::content::structures::STRUCTURE_WALL => {
                if st.health.is_alive() {
                    draw_rectangle(sp.x - 1.5, sp.y - 1.5, 3.0, 3.0, DARKGRAY);
                    draw_rectangle_lines(sp.x - 1.5, sp.y - 1.5, 3.0, 3.0, 1.0, GRAY);
                }
            }
            crate::content::structures::STRUCTURE_OBELISK => {
                if st.is_active {
                    let pulse = (get_time() as f32 * 8.0).sin().abs() * 3.0;
                    draw_circle(sp.x, sp.y, 4.5 + pulse, Color::new(1.0, 0.2, 0.8, 0.9));
                    draw_circle_lines(sp.x, sp.y, 6.5 + pulse, 1.5, GOLD);
                } else {
                    draw_rectangle(sp.x - 2.5, sp.y - 2.5, 5.0, 5.0, Color::new(0.3, 0.2, 0.4, 0.8));
                    draw_rectangle_lines(sp.x - 2.5, sp.y - 2.5, 5.0, 5.0, 1.0, PURPLE);
                }
            }
            _ => {}
        }
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

/// Всплывающая подсказка для плашки Очков Душ и Инкама
fn draw_souls_plaque_tooltip(
    px: f32,
    py: f32,
    screen_w: f32,
    fonts: &GameFonts,
    world: &World,
    director: &crate::ecs::world::DirectorState,
) {
    let card_w = 320.0;
    let card_h = 160.0;
    let card_x = (px - card_w - 12.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = py - 40.0;

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.06, 0.06, 0.12, 0.96));
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, GOLD);

    fonts.draw_heading("КАЗНА И ЭКОНОМИКА ДИРЕКТОРА", card_x + 12.0, card_y + 22.0, 15.0, GOLD);
    let altar_count = world.structures.iter().filter(|s| crate::content::structures::is_altar_structure_type(s.structure_type) && s.health.is_alive()).count();
    let total_income = director.income_rate + (altar_count as f32 * 3.0);
    fonts.draw_text(&format!("• Текущий баланс: {:.0} Очков Душ", director.points), card_x + 12.0, card_y + 44.0, 12.5, WHITE);
    fonts.draw_text(&format!("• Суммарный доход: +{:.1} / 2с (Базовый + Алтари)", total_income), card_x + 12.0, card_y + 64.0, 12.5, GREEN);
    fonts.draw_text(&format!("• Всего потрачено: {:.0} душ за матч", director.total_points_spent), card_x + 12.0, card_y + 84.0, 12.0, LIGHTGRAY);
    fonts.draw_text(&format!("• Всего призвано: {} монстров", director.total_mobs_spawned), card_x + 12.0, card_y + 104.0, 12.0, SKYBLUE);
    fonts.draw_text(&format!("• Производительность: {} FPS | На поле: {}", get_fps(), world.mobs.len()), card_x + 12.0, card_y + 124.0, 12.0, YELLOW);
    fonts.draw_text("Совет: Каждая трата душ навсегда ускоряет доход на +2%!", card_x + 12.0, card_y + 146.0, 11.0, GRAY);
}

/// Всплывающая подсказка для кнопки постройки Алтаря Крови
fn draw_altar_button_tooltip(
    tier: u8,
    hotkey: &str,
    bx: f32,
    by: f32,
    screen_w: f32,
    fonts: &GameFonts,
    director: &crate::ecs::world::DirectorState,
) {
    let config = crate::content::mobs::get_squad_config(tier);
    let card_w = 340.0;
    let card_h = 168.0;
    let card_x = (bx + 35.0 - card_w * 0.5).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = (by - card_h - 10.0).max(10.0);

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.07, 0.06, 0.12, 0.98));
    let altar_cooldown = director.altar_cooldown_for_tier(tier);
    let altar_cost = crate::content::structures::altar_cost(tier);
    let is_unlocked = director.total_points_spent >= config.unlock_spent_threshold;
    let affordable = director.points >= altar_cost;
    let can_build = is_unlocked && affordable && altar_cooldown <= 0.0;

    let border_color = if can_build { GOLD } else if altar_cooldown > 0.0 { Color::new(0.9, 0.35, 0.3, 0.95) } else { PURPLE };
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, border_color);

    // Заголовок
    let title = format!("АЛТАРЬ КРОВИ: {} [ТИР {}]", config.name.to_uppercase(), tier);
    fonts.draw_heading(&title, card_x + 12.0, card_y + 22.0, 13.5, GOLD);

    // Управление
    let ctrl_hint = format!("• Горячая клавиша: [{}] или клик ЛКМ", hotkey);
    fonts.draw_text(&ctrl_hint, card_x + 12.0, card_y + 42.0, 11.5, YELLOW);

    // Характеристики
    let spawn_int = crate::content::structures::altar_spawn_interval(tier);
    let stats = format!("• Стоимость: {:.0} Душ | Откат: {:.0}с | Спавн: каждые {:.1}с", altar_cost, crate::content::structures::altar_cooldown(tier), spawn_int);
    fonts.draw_text(&stats, card_x + 12.0, card_y + 62.0, 11.0, WHITE);

    // Эффекты
    fonts.draw_text("• Призыв: Непрерывно спавнит бойцов для каждого живого Босса", card_x + 12.0, card_y + 82.0, 10.5, SKYBLUE);
    fonts.draw_text("• Аура исцеления: Лечит мобов в радиусе 180px на +35 HP / 3с", card_x + 12.0, card_y + 100.0, 10.5, GREEN);
    fonts.draw_text("• Экономика: Приносит +3 Душ/сек пассивного дохода Директору", card_x + 12.0, card_y + 118.0, 10.5, Color::new(0.85, 0.65, 1.0, 1.0));

    // Статус
    if can_build {
        fonts.draw_heading("[ГОТОВ] К ПОСТРОЙКЕ! Нажмите Ctrl+Q..F или кликните ЛКМ", card_x + 12.0, card_y + 144.0, 11.0, GREEN);
    } else if altar_cooldown > 0.0 {
        fonts.draw_heading(&format!("[ОТКАТ] Перезарядка: осталось {:.1} сек", altar_cooldown), card_x + 12.0, card_y + 144.0, 11.0, ORANGE);
    } else if !is_unlocked {
        fonts.draw_heading(&format!("[БЛОК] Заблокировано: нужно потратить ещё {:.0} душ", config.unlock_spent_threshold - director.total_points_spent), card_x + 12.0, card_y + 144.0, 11.0, RED);
    } else {
        fonts.draw_heading(&format!("[ЦЕНА] Недостаточно душ: не хватает {:.0} душ", altar_cost - director.points), card_x + 12.0, card_y + 144.0, 11.0, Color::new(1.0, 0.4, 0.4, 1.0));
    }
}

/// Всплывающая подсказка для виджета Стены-Баррикады
fn draw_wall_widget_tooltip(ww_x: f32, ww_y: f32, screen_w: f32, fonts: &GameFonts) {
    let card_w = 370.0;
    let card_h = 168.0;
    let card_x = (ww_x - card_w - 12.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = (ww_y - 20.0).max(10.0);

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.06, 0.06, 0.12, 0.96));
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, Color::new(0.4, 0.7, 0.9, 0.95));

    fonts.draw_heading("БАРРИКАДА (50x50 px)", card_x + 12.0, card_y + 22.0, 15.0, GOLD);
    fonts.draw_text("• Постройка: Клик [ЛКМ] или клавиша [B], затем [ЛКМ] на арене", card_x + 12.0, card_y + 44.0, 11.0, WHITE);
    let cost_str = format!("• Стоимость: {:.0} Душ | Кулдаун: {:.1} сек | Время жизни: 60 сек", crate::content::structures::WALL_COST, crate::content::structures::WALL_MAX_COOLDOWN);
    fonts.draw_text(&cost_str, card_x + 12.0, card_y + 64.0, 11.0, YELLOW);
    fonts.draw_text("• Физический барьер: Босс не может пройти сквозь стену!", card_x + 12.0, card_y + 84.0, 11.0, SKYBLUE);
    fonts.draw_text("• Тактика: Ограждайте Алтари, запирайте босса в узких проходах", card_x + 12.0, card_y + 104.0, 11.0, GREEN);
    fonts.draw_text("• Прочность: 350 HP (Броня: 14) | Рывок босса наносит 160 урона", card_x + 12.0, card_y + 124.0, 11.0, LIGHTGRAY);
    fonts.draw_text("• Разрушение боссом: Даёт боссу +15 XP", card_x + 12.0, card_y + 146.0, 11.0, ORANGE);
}

/// Всплывающая подсказка для оружия Босса
fn draw_weapon_tooltip(w: &WeaponSlot, sx: f32, sy: f32, screen_w: f32, fonts: &GameFonts) {
    let def = get_weapon_def(w.kind);
    let card_w = 380.0;
    let card_h = 138.0;
    let card_x = (sx + 19.0 - card_w / 2.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = sy + 36.0;

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.06, 0.08, 0.14, 0.96));
    let border_color = if w.is_evolved { GOLD } else { SKYBLUE };
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, border_color);

    let title = if w.limit_break_level > 0 {
        format!("{} [MAX +{}]", def.name, w.limit_break_level)
    } else {
        format!("{} [Ур. {}/8]", def.name, w.level)
    };
    fonts.draw_heading(&title, card_x + 12.0, card_y + 22.0, 15.0, border_color);

    let dmg_type_str = match def.damage_type {
        crate::ecs::components::combat::DamageType::Physical => "Физический",
        crate::ecs::components::combat::DamageType::Fire => "Огненный",
        crate::ecs::components::combat::DamageType::Ice => "Ледяной",
        crate::ecs::components::combat::DamageType::Holy => "Святой",
        crate::ecs::components::combat::DamageType::True => "Чистый (True)",
        _ => "Обычный",
    };
    let stats_text = format!("Урон: {:.0} ({}) | Кулдаун: {:.2}с", w.damage, dmg_type_str, w.base_cooldown);
    fonts.draw_text(&stats_text, card_x + 12.0, card_y + 44.0, 12.0, WHITE);

    fonts.draw_text(def.description, card_x + 12.0, card_y + 68.0, 11.5, LIGHTGRAY);

    let (evo_str, evo_col) = if w.limit_break_level > 0 {
        (format!("[ВОЗВЫШЕНИЕ] +{}% Урона, +{} Снарядов", w.limit_break_level * 10, w.bonus_projectiles), GOLD)
    } else if w.is_evolved {
        ("[СУПЕРОРУЖИЕ] Максимальная боевая эволюция".to_string(), GOLD)
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
            WeaponType::SingularityOrb => ("Катушка Теслы", "Горизонт Событий"),
            WeaponType::ChainFrost => ("Клевер Судьбы", "Абсолютный Ноль"),
            WeaponType::BrimstoneRay => ("Призматический Фокус", "Инфернальный Катаклизм"),
            WeaponType::MissileSwarm => ("Черный Эгис", "Осада Судного Дня"),
            WeaponType::ChainLightning => ("Громовой Молот", "Гнев Бури"),
            WeaponType::DivineBell => ("Аура Божественности", "Аура Собора"),
            WeaponType::SoulPhantoms => ("Жатва Душ", "Вопль Банши"),
            WeaponType::AegisOrbiters => ("Эгида Бессмертия", "Несокрушимая Крепость"),
            WeaponType::ChaosMeteor => ("Сердце Пироманта", "Шторм Армагеддона"),
            WeaponType::VoidRift => ("Оккультная Призма", "Коллапс Сверхновой"),
            WeaponType::SacredScriptures => ("Песочные Часы", "Инфернальная Литургия"),
            WeaponType::FaithBoomerang => ("Клевер Судьбы", "Небесный Вердикт"),
            WeaponType::CursedChalice => ("Призматический Фокус", "Кровавый Потоп"),
            WeaponType::VampiricAura => ("Кровавый Камень", "Пиршество Душ"),
            WeaponType::RunicPrism => ("Зеркальный Дупликатор", "Искривление Пространства"),
            WeaponType::AbyssalBeam => ("Фолиант Времени", "Каскад Апокалипсиса"),
            WeaponType::ChronoStasis => ("Серебряная Слеза", "Бесконечная Дилатация"),
            WeaponType::AstralCloak => ("Черный Эгис", "Саван Кровавого Императора"),
            _ => ("-", "-"),
        };
        (format!("Эволюция (Ур. 8 + {}): -> {}", req_passive, evo_weapon), YELLOW)
    };
    fonts.draw_text(&evo_str, card_x + 12.0, card_y + 92.0, 11.5, evo_col);
    fonts.draw_text("Кликните на слот или значок [i] для детального Древа Синтеза!", card_x + 12.0, card_y + 114.0, 10.5, GOLD);
}

/// Всплывающая подсказка для пассивного предмета Босса
fn draw_passive_tooltip(p: &PassiveSlot, sx: f32, sy: f32, screen_w: f32, fonts: &GameFonts) {
    let def = get_passive_def(p.kind);
    let card_w = 400.0;
    let card_h = 138.0;
    let card_x = (sx + 19.0 - card_w / 2.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = sy + 36.0;

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.08, 0.06, 0.14, 0.96));
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, PURPLE);

    let title = format!("{} [Ур. {}/5]", def.name, p.level);
    fonts.draw_heading(&title, card_x + 12.0, card_y + 22.0, 15.0, GOLD);

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
        PassiveType::TeslaCoil => format!("Текущий бонус: +{}% шанс цепной молнии на 3 цели", p.level * 12),
        PassiveType::CloverOfFate => format!("Текущий бонус: +{}% шанс критического удара (х2 урон)", p.level * 12),
        PassiveType::PrismaticFocus => format!("Текущий бонус: +{}% к площади поражения (AoE)", p.level * 15),
        PassiveType::TitanAegis => format!("Текущий бонус: -{:.1} плоского урона от каждого удара", p.level as f32 * 2.5),
        PassiveType::ImmortalAegis => format!("Текущий бонус: Воскрешение с 100% HP и импульсом (Статус: {})", if p.level > 0 { "ГОТОВО" } else { "НЕАКТИВНО" }),
        PassiveType::ArchmageShard => format!("Текущий бонус: +{} рикошет, -{}% кд, +{}% AoE", p.level, p.level * 15, p.level * 20),
        PassiveType::DivineRapier => format!("Текущий бонус: +{}% наносимый урон, +{}% входящий урон", p.level * 60, p.level * 15),
        PassiveType::OctarineCore => format!("Текущий бонус: {}% вампиризм от всех заклинаний", p.level * 22),
        PassiveType::SolarRelic => format!("Текущий бонус: {} святого AoE урона/сек, -20% урона врагов", p.level * 40),
        PassiveType::StaticCore => format!("Текущий бонус: {}% шанс молнии на 6 целей", p.level * 25),
        PassiveType::ColossalEye => format!("Текущий бонус: +{}% урон, +{}% размер снарядов, 100% пробивание", p.level * 100, p.level * 50),
        PassiveType::BlessedHeart => format!("Текущий бонус: +{} HP, +{}% урон, самонаведение снарядов", p.level * 150, p.level * 25),
        PassiveType::Pyromaniac => format!("Текущий бонус: {}% исцеления от огненного урона и взрывов", p.level * 35),
        PassiveType::SoulHarvester => format!("Текущий бонус: {}% шанс призыва мстительных духов", p.level * 20),
        PassiveType::DivinityAura => format!("Текущий бонус: {} AoE святого урона от летящих снарядов", p.level * 35),
        PassiveType::UkuleleResonance => format!("Текущий бонус: {}% шанс электро-цепочки на 4 цели", p.level * 25),
        PassiveType::FateWeaver => "Текущий бонус: Все шансы эффектов проверяются дважды в пользу босса".to_string(),
        PassiveType::SiegeMortar => "Текущий бонус: Взрывы 55px при каждом попадании снаряда (50% урона)".to_string(),
        PassiveType::SporeMushroom => format!("Текущий бонус: +{:.0} HP/сек (+{:.1}% макс HP) при остановке", p.level as f32 * 18.0, p.level as f32 * 1.5),
        PassiveType::DuplicatorRelic => format!("Текущий бонус: +{} доп. снаряда(ов) ко всем способностям", p.level),
        PassiveType::CandelabraOfFury => format!("Текущий бонус: +{}% к радиусу всех взрывов, аур и луж", p.level * 20),
        PassiveType::AbyssalCurseStone => format!("Текущий бонус: +{}% к опыту босса, +{}% к общему урону", p.level * 35, p.level * 25),
        PassiveType::SilverTears => format!("Текущий бонус: +{:.1} HP/с реген, +{}% сопротивления стихиям", p.level as f32 * 3.0, p.level * 15),
        PassiveType::GoldenSeal => format!("Текущий бонус: +{}% урон, +{}% крит, +{}px радиус сбора", p.level * 20, p.level * 10, p.level * 35),
        PassiveType::TitanBloodArmor => format!("Текущий бонус: +{:.0} к броне, +{}% урона при потере HP", p.level as f32 * 12.0, p.level * 8),
        PassiveType::BerserkerBlood => format!("Текущий бонус: До +{}% темпа атаки и +{:.1} HP/с при низком HP", p.level * 20, p.level as f32 * 2.0),
        PassiveType::MartyrVanguard => format!("Текущий бонус: До +{}% скорости бега от раненых союзников, +{:.1} HP за убийство моба", p.level * 15, p.level as f32 * 1.5),
    };
    fonts.draw_text(&bonus_text, card_x + 12.0, card_y + 44.0, 12.0, GREEN);

    fonts.draw_text(def.description, card_x + 12.0, card_y + 68.0, 11.5, LIGHTGRAY);

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
        PassiveType::TeslaCoil => "Связка для эволюции: Гравитационная Сфера -> Горизонт Событий",
        PassiveType::CloverOfFate => "Связка для эволюции: Цепной Ледяной Шар -> Абсолютный Ноль",
        PassiveType::PrismaticFocus => "Связка для эволюции: Кровавый Гиперлуч -> Инфернальный Катаклизм",
        PassiveType::TitanAegis => "Связка для эволюции: Рой Микроракет -> Осада Судного Дня",
        PassiveType::ImmortalAegis => "Связка для эволюции: Орбитальные Бастионы -> Несокрушимая Крепость",
        PassiveType::ArchmageShard => "Связка для синергии: Усиливает все заклинания и снаряды (+AoE, -кд)",
        PassiveType::DivineRapier => "Связка для синергии: Колоссальный урон для всех видов вооружения",
        PassiveType::OctarineCore => "Связка для эволюции: Разрыв Бездны -> Коллапс Сверхновой",
        PassiveType::SolarRelic => "Связка для синергии: Священная аура, сжигающая нежить и орду",
        PassiveType::StaticCore => "Связка для эволюции: Цепная Молния -> Гнев Бури",
        PassiveType::ColossalEye => "Связка для синергии: Превращает все снаряды в пробивающие гиганты",
        PassiveType::BlessedHeart => "Связка для синергии: Самонаведение для абсолютно всех атак босса",
        PassiveType::Pyromaniac => "Связка для эволюции: Хаотический Метеор -> Шторм Армагеддона",
        PassiveType::SoulHarvester => "Связка для эволюции: Голодные Призраки -> Вопль Банши",
        PassiveType::DivinityAura => "Связка для эволюции: Священный Колокол -> Аура Собора",
        PassiveType::UkuleleResonance => "Связка для синергии: Электро-цепи при любом ударе",
        PassiveType::FateWeaver => "Связка для синергии: Удваивает удачу для всех критов и проков",
        PassiveType::SiegeMortar => "Связка для синергии: Добавляет взрывы по площади ко всем атакам",
        PassiveType::SporeMushroom => "Связка для выживания: Безусловная регенерация даже под градом ударов",
        PassiveType::DuplicatorRelic => "Связка для эволюции: Руническая Призма -> Искривление Пространства",
        PassiveType::CandelabraOfFury => "Связка для синергии: Колоссальное расширение радиуса поражения всех AoE",
        PassiveType::AbyssalCurseStone => "Связка для синергии: Экстремальный разгон прокачки уровня и урона",
        PassiveType::SilverTears => "Связка для эволюции: Хроно-Стазис -> Бесконечная Дилатация",
        PassiveType::GoldenSeal => "Связка для синергии: Урон, шанс критического удара и глобальный сбор душ",
        PassiveType::TitanBloodArmor => "Связка для синергии: Непробиваемая броня и ярость возмездия при низком HP",
        PassiveType::BerserkerBlood => "Связка для синергии: Экстремальный вампиризм и ярость ближнего боя",
        PassiveType::MartyrVanguard => "Связка для синергии: Командная скорость и неиссякаемое исцеление на передовой",
    };
    fonts.draw_text(evo_synergy, card_x + 12.0, card_y + 92.0, 11.5, YELLOW);
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
