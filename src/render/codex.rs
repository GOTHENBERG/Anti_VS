use std::sync::atomic::{AtomicUsize, Ordering};
use macroquad::prelude::*;
use crate::ecs::components::inventory::{PassiveType, WeaponType};
use crate::render::font::GameFonts;
use crate::render::item_dossier::ItemInspectionTarget;
use crate::render::textures::GameTextures;

static BESTIARY_PAGE: AtomicUsize = AtomicUsize::new(0);
static WEAPONS_PAGE: AtomicUsize = AtomicUsize::new(0);
static PASSIVES_PAGE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodexResult {
    MobDossier(u8),
    ItemInspection(ItemInspectionTarget),
    Close,
}

pub fn draw_codex(current_tab: &mut usize, fonts: &GameFonts, textures: &GameTextures) -> Option<CodexResult> {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // Затемнение фона игры
    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.78));

    let modal_w = (screen_w - 80.0).clamp(940.0, 1200.0);
    let modal_h = (screen_h - 60.0).clamp(560.0, 680.0);
    let mx = (screen_w - modal_w) / 2.0;
    let my = (screen_h - modal_h) / 2.0;

    // Главное окно кодекса
    draw_rectangle(mx, my, modal_w, modal_h, Color::new(0.08, 0.08, 0.12, 0.98));
    draw_rectangle_lines(mx, my, modal_w, modal_h, 2.5, Color::new(0.85, 0.65, 0.2, 1.0));

    // Верхняя заголовочная плашка
    draw_rectangle(mx, my, modal_w, 42.0, Color::new(0.12, 0.12, 0.18, 1.0));
    draw_rectangle_lines(mx, my, modal_w, 42.0, 1.5, DARKGRAY);

    let title = "ИНФЕРНАЛЬНЫЙ КОДЕКС РЕЖИССЁРА  |  [I] или [Esc] — Закрыть окно";
    fonts.draw_heading(title, mx + 20.0, my + 28.0, 18.0, GOLD);

    // Кнопка [ X ] в верхнем правом углу
    let close_btn_w = 34.0;
    let close_btn_h = 28.0;
    let close_btn_x = mx + modal_w - close_btn_w - 7.0;
    let close_btn_y = my + 7.0;
    let (mouse_x, mouse_y) = mouse_position();
    let is_close_hover = mouse_x >= close_btn_x && mouse_x <= close_btn_x + close_btn_w && mouse_y >= close_btn_y && mouse_y <= close_btn_y + close_btn_h;
    draw_rectangle(close_btn_x, close_btn_y, close_btn_w, close_btn_h, if is_close_hover { RED } else { Color::new(0.20, 0.10, 0.12, 0.9) });
    draw_rectangle_lines(close_btn_x, close_btn_y, close_btn_w, close_btn_h, 1.5, WHITE);
    let cw = fonts.measure_heading("X", 15.0).width;
    fonts.draw_heading("X", close_btn_x + (close_btn_w - cw) / 2.0, close_btn_y + 19.0, 15.0, WHITE);

    if is_mouse_button_pressed(MouseButton::Left) && is_close_hover {
        return Some(CodexResult::Close);
    }

    // Вкладки
    let tabs = [
        "1. БЕСТИАРИЙ (МОБЫ)",
        "2. ОРУЖИЕ И ЭВОЛЮЦИИ",
        "3. ПАССИВКИ БОССА",
        "4. АЛТАРИ И МЕХАНИКИ",
    ];

    let tab_w = modal_w / 4.0;
    let tab_y = my + 44.0;
    let tab_h = 32.0;

    // Переключение вкладок клавишами
    if is_key_pressed(KeyCode::Key1) { *current_tab = 0; }
    if is_key_pressed(KeyCode::Key2) { *current_tab = 1; }
    if is_key_pressed(KeyCode::Key3) { *current_tab = 2; }
    if is_key_pressed(KeyCode::Key4) { *current_tab = 3; }
    if is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::Right) {
        *current_tab = (*current_tab + 1) % 4;
    }
    if is_key_pressed(KeyCode::Left) {
        *current_tab = if *current_tab == 0 { 3 } else { *current_tab - 1 };
    }

    // Обработка кликов мыши по вкладкам
    let (mouse_x, mouse_y) = mouse_position();
    for (i, &tab_name) in tabs.iter().enumerate() {
        let tx = mx + (i as f32) * tab_w;
        if is_mouse_button_pressed(MouseButton::Left)
            && mouse_x >= tx && mouse_x <= tx + tab_w
            && mouse_y >= tab_y && mouse_y <= tab_y + tab_h
        {
            *current_tab = i;
        }

        let is_active = *current_tab == i;
        let bg_color = if is_active {
            Color::new(0.20, 0.16, 0.28, 1.0)
        } else {
            Color::new(0.10, 0.10, 0.14, 0.95)
        };
        let border_color = if is_active { GOLD } else { GRAY };
        let text_color = if is_active { WHITE } else { LIGHTGRAY };

        draw_rectangle(tx, tab_y, tab_w, tab_h, bg_color);
        draw_rectangle_lines(tx, tab_y, tab_w, tab_h, if is_active { 2.0 } else { 1.0 }, border_color);

        let tw = fonts.measure_text(tab_name, 14.0).width;
        fonts.draw_text(tab_name, tx + (tab_w - tw) / 2.0, tab_y + 22.0, 14.0, text_color);
    }

    // Содержимое выбранной вкладки
    let content_x = mx + 20.0;
    let content_y = tab_y + tab_h + 16.0;
    let content_w = modal_w - 40.0;
    let content_h = modal_h - (content_y - my) - 20.0;

    match *current_tab {
        0 => draw_tab_bestiary(content_x, content_y, content_w, content_h, fonts, textures),
        1 => draw_tab_weapons(content_x, content_y, content_w, content_h, fonts, textures),
        2 => draw_tab_passives(content_x, content_y, content_w, content_h, fonts, textures),
        _ => {
            draw_tab_mechanics(content_x, content_y, content_w, content_h, fonts);
            None
        }
    }
}

/// Вкладка 1: Бестиарий мобов (с переключением страниц: Страница 1 [Тир 1-8] и Страница 2 [Тир 9-16])
fn draw_tab_bestiary(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) -> Option<CodexResult> {
    let mut requested_action = None;
    let (mx, my) = mouse_position();
    let current_page = BESTIARY_PAGE.load(Ordering::Relaxed);

    // Верхние переключатели страниц Бестиария
    let p_btn_w = 210.0;
    let p_btn_h = 28.0;
    let p1_x = x;
    let p2_x = x + p_btn_w + 12.0;
    let p_btn_y = y;

    let p1_hover = mx >= p1_x && mx <= p1_x + p_btn_w && my >= p_btn_y && my <= p_btn_y + p_btn_h;
    let p2_hover = mx >= p2_x && mx <= p2_x + p_btn_w && my >= p_btn_y && my <= p_btn_y + p_btn_h;

    // Кнопка Страница 1 (Тир 1-8)
    draw_rectangle(p1_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 0 { Color::new(0.24, 0.18, 0.38, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
    draw_rectangle_lines(p1_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 0 { 2.0 } else { 1.0 }, if current_page == 0 { GOLD } else { GRAY });
    fonts.draw_heading("СТРАНИЦА 1: ОРДА (ТИР 1-8)", p1_x + 12.0, p_btn_y + 19.0, 11.5, if current_page == 0 { GOLD } else { LIGHTGRAY });
    if is_mouse_button_pressed(MouseButton::Left) && p1_hover {
        BESTIARY_PAGE.store(0, Ordering::Relaxed);
    }

    // Кнопка Страница 2 (Тир 9-16)
    draw_rectangle(p2_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 1 { Color::new(0.38, 0.14, 0.22, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
    draw_rectangle_lines(p2_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 1 { 2.0 } else { 1.0 }, if current_page == 1 { GOLD } else { GRAY });
    fonts.draw_heading("СТРАНИЦА 2: ЛЕГИОН (ТИР 9-16)", p2_x + 10.0, p_btn_y + 19.0, 11.5, if current_page == 1 { GOLD } else { LIGHTGRAY });
    if is_mouse_button_pressed(MouseButton::Left) && p2_hover {
        BESTIARY_PAGE.store(1, Ordering::Relaxed);
    }

    let mobs_page1 = [
        (
            1u8,
            "Тир 1: Скелеты-пехотинцы",
            "Цена: 15 душ | Отряд: 8 шт | Доход: +0.30/2с | Дроп: 8 XP",
            "HP: 45 | Скор: 105 px/с | Урон: 7 физ (1.0 уд/с) | Броня: 0",
            "Резисты: Физ +5% | Огонь 0% | Холод 0% | Святой -25%",
            "Роль: Пушечное мясо. Задерживают босса и сбивают наводку.",
            "Разблокировка: Доступны со старта игры (0 потрачено душ).",
            Color::new(0.9, 0.9, 0.95, 1.0),
        ),
        (
            2u8,
            "Тир 2: Чёрные гончие",
            "Цена: 35 душ | Отряд: 5 шт | Доход: +0.70/2с | Дроп: 20 XP",
            "HP: 70 | Скор: 160 px/с (+39% быстрее!) | Урон: 12 физ | Броня: 1",
            "Резисты: Физ +10% | Огонь 0% | Холод 0% | Святой -20%",
            "Роль: Предиктивный перехват на опережение вектора скорости.",
            "Разблокировка: Открываются после суммарной траты 150 душ.",
            Color::new(0.95, 0.45, 0.45, 1.0),
        ),
        (
            3u8,
            "Тир 3: Культисты-стрелки",
            "Цена: 70 душ | Отряд: 5 шт | Доход: +1.40/2с | Дроп: 45 XP",
            "HP: 105 | Скор: 80 px/с | Урон: 16 маг (1/2.0с) | Дистанция: 320 px | Броня: 2",
            "Резисты: Физ +15% | Огонь +20% | Холод -10% | Святой -40%",
            "Роль: Дальнобойный кайтинг. Вынуждают босса тратить Рывок.",
            "Разблокировка: Открываются после суммарной траты 400 душ.",
            Color::new(0.85, 0.55, 1.0, 1.0),
        ),
        (
            4u8,
            "Тир 4: Ледяные големы",
            "Цена: 150 душ | Отряд: 2 шт | Доход: +3.00/2с | Дроп: 130 XP",
            "HP: 532 | Скор: 52 px/с | Урон: 32 лед | Броня: 14 | Иммун к отбросу",
            "Резисты: Лед 100% (ИММУН) | Физ +35% | Огонь -50% | Яд +50%",
            "Роль: Тяжёлые танки. Впитывают урон и замедляют босса ударами.",
            "Разблокировка: Открываются после суммарной траты 1000 душ.",
            Color::new(0.4, 0.85, 1.0, 1.0),
        ),
        (
            5u8,
            "Тир 5: Камикадзе (Огненные подрывники)",
            "Цена: 300 душ | Отряд: 3 шт | Доход: +6.00/2с | Дроп: 160 XP",
            "HP: 182 | Скор: 185 px/с (Сверхзвук) | Урон: 220 Чистый сплэш (95px) | Броня: 2",
            "Резисты: Физ +10% | Огонь +50% | Холод 0% | Святой 0%",
            "Роль: Суицидальный прорыв. Взрываются при контакте по площади.",
            "Разблокировка: Открываются после суммарной траты 2500 душ.",
            Color::new(1.0, 0.55, 0.2, 1.0),
        ),
        (
            6u8,
            "Тир 6: Демоны теней (Shadow Demons)",
            "Цена: 500 душ | Отряд: 2 шт | Доход: +10.00/2с | Дроп: 240 XP",
            "HP: 770 | Скор: 140 px/с | Урон: 42 физ/тьма | Скачок: кд 4.5с | Броня: 8",
            "Резисты: Физ +25% | Огонь +20% | Холод +20% | Святой -10%",
            "Роль: Скрытный прорыв. Телепортируются за спину босса, сбивая прицел.",
            "Разблокировка: Открываются после суммарной траты 4000 душ.",
            Color::new(0.75, 0.4, 0.9, 1.0),
        ),
        (
            7u8,
            "Тир 7: Огненные элементали",
            "Цена: 850 душ | Отряд: 2 шт | Доход: +17.00/2с | Дроп: 420 XP",
            "HP: 1190 | Скор: 95 px/с | Урон: 55 огонь (1/2.2с) | Дистанция: 360 px | Броня: 10",
            "Резисты: Огонь 100% (ИММУН) | Физ +20% | Лед -40% | Яд +50%",
            "Роль: Огненный обстрел. Артиллерийские кометы и огненный след.",
            "Разблокировка: Открываются после суммарной траты 6500 душ.",
            Color::new(1.0, 0.35, 0.15, 1.0),
        ),
        (
            8u8,
            "Тир 8: Жнец Смерти (Титан)",
            "Цена: 1800 душ | Отряд: 1 шт | Доход: +36.00/2с | Дроп: 1200 XP",
            "HP: 3920 | Скор: 68 px/с | Урон: 95 True + Волна 420 px | Броня: 24 | Иммун",
            "Резисты: Физ +45% | Огонь +30% | Холод +30% | Святой -20%",
            "Роль: Мини-босс орды! Рассекающие удары косы сквозь броню.",
            "Разблокировка: Открываются после суммарной траты 10000 душ.",
            Color::new(0.9, 0.2, 0.3, 1.0),
        ),
    ];

    let mobs_page2 = [
        (
            9u8,
            "Тир 9: Дред-Фаланга",
            "Цена: 250 душ | Отряд: 8 шт | Доход: +5.00/2с | Дроп: 35 XP",
            "HP: 168 | Скор: 115 px/с | Урон: 24 физ (1.0 уд/с) | Броня: 6 | Иммун к отбросу",
            "Резисты: Физ +25% | Огонь +10% | Холод +10% | Святой -20%",
            "Роль: Элитная нежить с башенными щитами. Несгибаемый строй.",
            "Разблокировка: Открываются после суммарной траты 15000 душ.",
            Color::new(0.85, 0.85, 0.95, 1.0),
        ),
        (
            10u8,
            "Тир 10: Адские Гончие",
            "Цена: 600 душ | Отряд: 5 шт | Доход: +12.00/2с | Дроп: 85 XP",
            "HP: 266 | Скор: 180 px/с (+56% быстрее!) | Урон: 38 огонь | Броня: 4",
            "Резисты: Огонь +50% | Физ +20% | Холод 0% | Святой -15%",
            "Роль: Церберы с огненным наскоком. Стремительно сжимают кольцо.",
            "Разблокировка: Открываются после суммарной траты 22000 душ.",
            Color::new(1.0, 0.4, 0.3, 1.0),
        ),
        (
            11u8,
            "Тир 11: Архикультисты Бездны",
            "Цена: 1100 душ | Отряд: 5 шт | Доход: +22.00/2с | Дроп: 180 XP",
            "HP: 448 | Скор: 90 px/с | Урон: 48 True (1/1.6с) | Дистанция: 380 px | Броня: 6",
            "Резисты: Физ +25% | Огонь +10% | Холод +10% | Святой -30%",
            "Роль: Снайперы Бездны. Самонаводящиеся чистые сгустки издалека.",
            "Разблокировка: Открываются после суммарной траты 30000 душ.",
            Color::new(0.75, 0.45, 1.0, 1.0),
        ),
        (
            12u8,
            "Тир 12: Обсидиановые Колоссы",
            "Цена: 2200 душ | Отряд: 2 шт | Доход: +44.00/2с | Дроп: 550 XP",
            "HP: 2030 | Скор: 58 px/с | Урон: 85 физ (1.0 уд/с) | Броня: 30 | Иммун к отбросу",
            "Резисты: Лед +80% | Физ +55% | Огонь -30% | Яд +50%",
            "Роль: Живые крепости 2030 HP. Блокируют сотни снарядов босса.",
            "Разблокировка: Открываются после суммарной траты 40000 душ.",
            Color::new(0.5, 0.65, 0.85, 1.0),
        ),
        (
            13u8,
            "Тир 13: Проклятые Мучители",
            "Цена: 3500 душ | Отряд: 3 шт | Доход: +70.00/2с | Дроп: 650 XP",
            "HP: 910 | Скор: 205 px/с (Сверхзвук) | Урон: 550 Чистый сплэш (120px) | Броня: 8",
            "Резисты: Физ +20% | Огонь +10% | Холод +10% | Святой 0%",
            "Роль: Ядерные подрывники. Смертоносный контактный подрыв арены.",
            "Разблокировка: Открываются после суммарной траты 52000 душ.",
            Color::new(1.0, 0.3, 0.1, 1.0),
        ),
        (
            14u8,
            "Тир 14: Призрачные Ловчие",
            "Цена: 5500 душ | Отряд: 2 шт | Доход: +110.00/2с | Дроп: 950 XP",
            "HP: 2520 | Скор: 155 px/с | Урон: 110 физ/тьма | Фаза 3.0с | Броня: 16",
            "Резисты: Физ +40% | Огонь +30% | Холод +30% | Святой -15%",
            "Роль: Высшие демоны мрака. Фазовый сдвиг сквозь преграды к боссу.",
            "Разблокировка: Открываются после суммарной траты 68000 душ.",
            Color::new(0.85, 0.3, 0.95, 1.0),
        ),
        (
            15u8,
            "Тир 15: Солнечные Архонты",
            "Цена: 8500 душ | Отряд: 2 шт | Доход: +170.00/2с | Дроп: 1500 XP",
            "HP: 3640 | Скор: 110 px/с | Урон: 140 огонь (1/1.8с) | Дистанция: 440 px | Броня: 22",
            "Резисты: Огонь 100% (ИММУН) | Физ +35% | Холод -20% | Яд +50%",
            "Роль: Падшие серафимы. Лучевые кометы с колоссальным уроном.",
            "Разблокировка: Открываются после суммарной траты 85000 душ.",
            Color::new(1.0, 0.85, 0.2, 1.0),
        ),
        (
            16u8,
            "Тир 16: Аватар Забвения (Колосс)",
            "Цена: 18000 душ | Отряд: 1 шт | Доход: +360.00/2с | Дроп: 3600 XP",
            "HP: 11900 | Скор: 75 px/с | Урон: 280 True + Волна 500 px | Броня: 45 | Иммун",
            "Резисты: Физ +65% | Огонь +50% | Холод +50% | Яд +50%",
            "Роль: Верховный владыка Бездны! 11900 HP, сокрушительные волны смерти.",
            "Разблокировка: Открываются после суммарной траты 110000 душ.",
            Color::new(0.95, 0.15, 0.25, 1.0),
        ),
    ];

    let active_mobs = if current_page == 0 { mobs_page1 } else { mobs_page2 };

    let grid_y = y + 34.0;
    let grid_h = h - 38.0;
    let col_w = (w - 14.0) / 2.0;
    let row_h = (grid_h - 15.0) / 4.0;

    for (i, (tier, name, cost_info, stats, resists, role, unlock, col)) in active_mobs.iter().enumerate() {
        let col_idx = (i / 4) as f32;
        let row_idx = (i % 4) as f32;

        let cx = x + col_idx * (col_w + 14.0);
        let cy = grid_y + row_idx * (row_h + 5.0);

        let card_h = row_h - 2.0;
        let is_card_hover = mx >= cx && mx <= cx + col_w && my >= cy && my <= cy + card_h;

        // Кнопка [i] Досье в правом верхнем углу
        let btn_w = 82.0;
        let btn_h = 24.0;
        let btn_x = cx + col_w - btn_w - 8.0;
        let btn_y = cy + 6.0;
        let is_btn_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;

        // Фон карточки
        let bg_col = if is_card_hover && !is_btn_hover {
            Color::new(0.14, 0.13, 0.20, 0.95)
        } else {
            Color::new(0.11, 0.11, 0.16, 0.92)
        };
        draw_rectangle(cx, cy, col_w, card_h, bg_col);
        draw_rectangle_lines(cx, cy, col_w, card_h, if is_card_hover { 2.0 } else { 1.5 }, if is_card_hover { GOLD } else { Color::new(0.3, 0.3, 0.4, 0.8) });

        // Отрисовка кнопки [i] Досье
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, if is_btn_hover { Color::new(0.38, 0.18, 0.50, 0.95) } else { Color::new(0.16, 0.16, 0.22, 0.90) });
        draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, if is_btn_hover { 1.5 } else { 1.0 }, if is_btn_hover { GOLD } else { GRAY });
        let blw = fonts.measure_heading("Досье [i]", 11.0).width;
        fonts.draw_heading("Досье [i]", btn_x + (btn_w - blw) / 2.0, btn_y + 16.0, 11.0, if is_btn_hover { GOLD } else { WHITE });

        // Иконка/спрайт моба слева
        if let Some(tex) = textures.get_mob_texture(*tier, 0) {
            let icon_s = 48.0;
            let (iw, ih) = if *tier == 2 || *tier == 10 { (icon_s * 1.3, icon_s * 0.7) } else { (icon_s, icon_s) };
            draw_texture_ex(
                tex,
                cx + 8.0,
                cy + (card_h - ih) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(iw, ih)),
                    ..Default::default()
                },
            );
        }

        let text_x = cx + 64.0;
        let max_text_w = col_w - 74.0;

        // 1. Имя моба
        fonts.draw_heading(name, text_x, cy + 18.0, 13.0, *col);

        // 2. Параметры стоимости и экономики
        fonts.draw_text(cost_info, text_x, cy + 33.0, 11.0, GOLD);

        // 3. Боевые параметры (HP, скорость бега, урон, темп атаки, дальность, броня)
        fonts.draw_text(stats, text_x, cy + 46.0, 10.5, WHITE);

        // 4. Резисты и стойкость к отталкиванию
        fonts.draw_text(resists, text_x, cy + 59.0, 10.5, LIGHTGRAY);

        // 5. Тактическая роль (с авто-переносом по ширине карточки, чтобы текст НИКОГДА не вылезал)
        fonts.draw_text_wrapped(role, text_x, cy + 72.0, 10.5, 13.0, max_text_w, SKYBLUE);

        // 6. Условие разблокировки внизу карточки
        fonts.draw_text(unlock, text_x, cy + card_h - 6.0, 10.0, GREEN);

        // Клик по кнопке [i] или по всей карточке -> открывает полное досье
        if is_mouse_button_pressed(MouseButton::Left) && (is_btn_hover || is_card_hover) {
            requested_action = Some(CodexResult::MobDossier(*tier));
        }
    }

    requested_action
}

/// Вкладка 2: Оружие и Эволюции Босса (с переключением страниц: Оружие 1-10 и Оружие 11-20)
fn draw_tab_weapons(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) -> Option<CodexResult> {
    let mut requested_action = None;
    let (mx, my) = mouse_position();
    let current_page = WEAPONS_PAGE.load(Ordering::Relaxed);

    // Верхние переключатели страниц Оружия
    let p_btn_w = 210.0;
    let p_btn_h = 28.0;
    let p1_x = x;
    let p2_x = x + p_btn_w + 10.0;
    let p3_x = x + (p_btn_w + 10.0) * 2.0;
    let p_btn_y = y;

    let p1_hover = mx >= p1_x && mx <= p1_x + p_btn_w && my >= p_btn_y && my <= p_btn_y + p_btn_h;
    let p2_hover = mx >= p2_x && mx <= p2_x + p_btn_w && my >= p_btn_y && my <= p_btn_y + p_btn_h;
    let p3_hover = mx >= p3_x && mx <= p3_x + p_btn_w && my >= p_btn_y && my <= p_btn_y + p_btn_h;

    // Кнопка Страница 1 (1-10)
    draw_rectangle(p1_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 0 { Color::new(0.24, 0.18, 0.38, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
    draw_rectangle_lines(p1_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 0 { 2.0 } else { 1.0 }, if current_page == 0 { GOLD } else { GRAY });
    fonts.draw_heading("СТРАНИЦА 1: ОРУЖИЕ [1-10]", p1_x + 10.0, p_btn_y + 19.0, 11.5, if current_page == 0 { GOLD } else { LIGHTGRAY });
    if is_mouse_button_pressed(MouseButton::Left) && p1_hover {
        WEAPONS_PAGE.store(0, Ordering::Relaxed);
    }

    // Кнопка Страница 2 (11-20)
    draw_rectangle(p2_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 1 { Color::new(0.38, 0.14, 0.22, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
    draw_rectangle_lines(p2_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 1 { 2.0 } else { 1.0 }, if current_page == 1 { GOLD } else { GRAY });
    fonts.draw_heading("СТРАНИЦА 2: ОРУЖИЕ [11-20]", p2_x + 10.0, p_btn_y + 19.0, 11.5, if current_page == 1 { GOLD } else { LIGHTGRAY });
    if is_mouse_button_pressed(MouseButton::Left) && p2_hover {
        WEAPONS_PAGE.store(1, Ordering::Relaxed);
    }

    // Кнопка Страница 3 (21-28)
    draw_rectangle(p3_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 2 { Color::new(0.18, 0.32, 0.44, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
    draw_rectangle_lines(p3_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 2 { 2.0 } else { 1.0 }, if current_page == 2 { GOLD } else { GRAY });
    fonts.draw_heading("СТРАНИЦА 3: ОРУЖИЕ [21-28]", p3_x + 10.0, p_btn_y + 19.0, 11.5, if current_page == 2 { GOLD } else { LIGHTGRAY });
    if is_mouse_button_pressed(MouseButton::Left) && p3_hover {
        WEAPONS_PAGE.store(2, Ordering::Relaxed);
    }

    let weapons_page1 = [
        (
            WeaponType::WindBlade,
            "1. Клинок Ветра -> БУРЯ КЛИНКОВ",
            "Рецепт: Клинок Ветра (ур. 8) + Перчатка Скорости",
            "Базовый: Серповидные клинки ветра по курсу бега.",
            "ЭВОЛЮЦИЯ: Вихрь из 8 вращающихся лезвий вокруг босса.",
            textures.icon_wind_blade.as_ref().or(textures.icon_sword.as_ref()),
            SKYBLUE,
        ),
        (
            WeaponType::FireAura,
            "2. Огненная Аура -> АДСКОЕ ПЕКЛО",
            "Рецепт: Огненная Аура (ур. 8) + Сердце Гиганта",
            "Базовый: Пульсирующая огненная волна (DoT горение).",
            "ЭВОЛЮЦИЯ: % урон от HP + детонация трупов горящих мобов!",
            textures.icon_fire_aura.as_ref().or(textures.icon_elixir.as_ref()),
            ORANGE,
        ),
        (
            WeaponType::MagicWand,
            "3. Магический Жезл -> ЛУЧ СМЕРТИ",
            "Рецепт: Магический Жезл (ур. 8) + Фолиант Времени",
            "Базовый: Самонаводящиеся сгустки тайной магии.",
            "ЭВОЛЮЦИЯ: Прожигающий лазерный луч через весь экран.",
            textures.icon_magic_wand.as_ref().or(textures.icon_wand.as_ref()),
            VIOLET,
        ),
        (
            WeaponType::HolyHammer,
            "4. Святой Молот -> ПРАВЕДНЫЙ ЩИТ",
            "Рецепт: Святой Молот (ур. 8) + Броня Света",
            "Базовый: Тяжёлые удары святым молотом по площади.",
            "ЭВОЛЮЦИЯ: Вращающийся щит света с 12% вампиризмом в HP.",
            textures.icon_holy_hammer.as_ref().or(textures.icon_shield.as_ref()),
            GOLD,
        ),
        (
            WeaponType::IceShards,
            "5. Ледяные Осколки -> ЛЕДНИКОВАЯ БУРЯ",
            "Рецепт: Ледяные Осколки (ур. 8) + Кольцо Заморозки",
            "Базовый: Веер ледяных стрел с замедлением на 40%.",
            "ЭВОЛЮЦИЯ: Пурга на пол-арены с полной заморозкой на 1.5с.",
            textures.icon_ice_shards.as_ref().or(textures.icon_bow.as_ref()),
            Color::new(0.4, 0.85, 1.0, 1.0),
        ),
        (
            WeaponType::PoisonOrbs,
            "6. Чумные Сферы -> ТОКСИЧНЫЙ МОР",
            "Рецепт: Чумные Сферы (ур. 8) + Едкий Сосуд",
            "Базовый: Ядовитые сферы, оставляющие лужи яда.",
            "ЭВОЛЮЦИЯ: Разъедающая кислота, игнорирующая 50% брони!",
            textures.icon_poison_orbs.as_ref().or(textures.icon_corrosive_flask.as_ref()).or(textures.icon_bottle.as_ref()),
            LIME,
        ),
        (
            WeaponType::ExecutionerAxe,
            "7. Топор Палача -> ВИХРЕВОЙ СЕКАЧ",
            "Рецепт: Топор Палача (ур. 8) + Кровавый Камень",
            "Базовый: Вращающиеся секиры с пробитием 4 целей.",
            "ЭВОЛЮЦИЯ: 12 рассекающих топоров по спирали с кровотечением.",
            textures.icon_executioner_axe.as_ref().or(textures.icon_ax.as_ref()),
            RED,
        ),
        (
            WeaponType::CelestialBow,
            "8. Небесный Лук -> ЗВЕЗДОПАДНЫЙ ЗАЛП",
            "Рецепт: Небесный Лук (ур. 8) + Орлиный Прицел",
            "Базовый: Скорострельные стрелы света в дальних стрелков.",
            "ЭВОЛЮЦИЯ: Ливень падающих звезд с неба по всей арене.",
            textures.icon_celestial_bow.as_ref().or(textures.icon_bow.as_ref()),
            GOLD,
        ),
        (
            WeaponType::ThunderOrb,
            "9. Грозовая Сфера -> ГНЕВ ГРОЗЫ",
            "Рецепт: Грозовая Сфера (ур. 8) + Песочные Часы",
            "Базовый: Электрические разряды по случайным мобам.",
            "ЭВОЛЮЦИЯ: Постоянные удары молний, парализующие врагов.",
            textures.icon_thunder.as_ref().or(textures.icon_jewel.as_ref()),
            YELLOW,
        ),
        (
            WeaponType::BloodDaggers,
            "10. Кровавые Кинжалы -> КРОВАВАЯ ЖАТВА",
            "Рецепт: Кровавые Кинжалы (ур. 8) + Сфера Притяжения",
            "Базовый: Быстрые броски кинжалов во фронтальный конус.",
            "ЭВОЛЮЦИЯ: 360-градусный веер клинков с 20% вампиризмом!",
            textures.icon_blood_daggers.as_ref().or(textures.icon_sword.as_ref()),
            Color::new(0.9, 0.2, 0.2, 1.0),
        ),
    ];

    let weapons_page2 = [
        (
            WeaponType::SingularityOrb,
            "11. Грави-Сфера -> ГОРИЗОНТ СОБЫТИЙ",
            "Рецепт: Гравитационная Сфера (ур. 8) + Катушка Теслы",
            "Базовый: Стягивает толпы врагов в единую точку.",
            "ЭВОЛЮЦИЯ: Черная дыра, взрывающаяся сверхновой звездой!",
            textures.icon_singularity.as_ref(),
            PURPLE,
        ),
        (
            WeaponType::BrimstoneRay,
            "12. Кровавый Луч -> ИНФЕРНАЛЬНЫЙ КАТАКЛИЗМ",
            "Рецепт: Кровавый Гиперлуч (ур. 8) + Призматический Фокус",
            "Базовый: Непрерывный плазменный лазер, прожигающий арену.",
            "ЭВОЛЮЦИЯ: Вращающийся крест из 4 плазменных лучей!",
            textures.icon_brimstone.as_ref(),
            Color::new(1.0, 0.2, 0.2, 1.0),
        ),
        (
            WeaponType::ChainLightning,
            "13. Цепная Молния -> ТЕСЛА-ШТОРМ",
            "Рецепт: Цепная Молния (ур. 8) + Катушка Теслы",
            "Базовый: Электрическая дуга, бьющая по 5 целям подряд.",
            "ЭВОЛЮЦИЯ: Непрерывная цепная буря на 12 целей с шоком!",
            textures.icon_chain_lightning.as_ref().or(textures.icon_wand.as_ref()),
            Color::new(0.3, 0.85, 1.0, 1.0),
        ),
        (
            WeaponType::DivineBell,
            "14. Колокол Собора -> АУРА ВЕЛИЧИЯ",
            "Рецепт: Колокол Собора (ур. 8) + Благословенное Ядро",
            "Базовый: Священный звон наносит чистый урон и отбрасывает.",
            "ЭВОЛЮЦИЯ: Молитвенный купол: 120 чистого DPS + щит 150 HP!",
            textures.icon_divine_bell.as_ref().or(textures.icon_shield.as_ref()),
            GOLD,
        ),
        (
            WeaponType::SoulPhantoms,
            "15. Фантомы Душ -> АРМИЯ ПРИЗРАКОВ",
            "Рецепт: Призрачные Фантомы (ур. 8) + Призрачный Сосуд",
            "Базовый: Духи рыцарей преследуют элиту и алтари.",
            "ЭВОЛЮЦИЯ: 6 неуязвимых призраков, блокирующих мобов!",
            textures.icon_soul_phantoms.as_ref().or(textures.icon_book.as_ref()),
            SKYBLUE,
        ),
        (
            WeaponType::AegisOrbiters,
            "16. Бастион Эгиды -> НЕСОКРУШИМАЯ КРЕПОСТЬ",
            "Рецепт: Бастион Эгиды (ур. 8) + Эгида Бессмертия",
            "Базовый: 4 парящих щита, отражающих снаряды мобов.",
            "ЭВОЛЮЦИЯ: 8 щитов-клинков: 100% отражение дальних атак!",
            textures.icon_aegis_orbiters.as_ref().or(textures.icon_shield.as_ref()),
            GOLD,
        ),
        (
            WeaponType::ChaosMeteor,
            "17. Метеорит Хаоса -> АРМАГЕДДОН",
            "Рецепт: Метеорит Хаоса (ур. 8) + Октариновое Ядро",
            "Базовый: Падение пылающих комет в скопления мобов.",
            "ЭВОЛЮЦИЯ: Залп из 5 метеоритов: кратеры с магмой на 8 сек!",
            textures.icon_chaos_meteor.as_ref().or(textures.icon_elixir.as_ref()),
            ORANGE,
        ),
        (
            WeaponType::VoidRift,
            "18. Разрыв Бездны -> ЧЕРНАЯ ДЫРА БЕЗДНЫ",
            "Рецепт: Разрыв Бездны (ур. 8) + Ткач Судеб",
            "Базовый: Пространственные воронки, затягивающие пехоту.",
            "ЭВОЛЮЦИЯ: Разрыв пространства: поглощает титанов!",
            textures.icon_void_rift.as_ref().or(textures.icon_singularity.as_ref()),
            PURPLE,
        ),
        (
            WeaponType::ChainFrost,
            "19. Кольцо Холода -> ВЕЧНАЯ МЕРЗЛОТА",
            "Рецепт: Кольцо Холода (ур. 8) + Кольцо Заморозки",
            "Базовый: Кольцевая волна с замедлением врагов на 60%.",
            "ЭВОЛЮЦИЯ: Оковы льда: раскалывает замерзших врагов!",
            textures.icon_chain_frost.as_ref().or(textures.icon_ring.as_ref()),
            Color::new(0.4, 0.9, 1.0, 1.0),
        ),
        (
            WeaponType::MissileSwarm,
            "20. Рой Микроракет -> ОСАДА СУДНОГО ДНЯ",
            "Рецепт: Рой Микроракет (ур. 8) + Черный Эгис",
            "Базовый: Самонаводящиеся микроракеты по скоплениям.",
            "ЭВОЛЮЦИЯ: Залп тяжелых баллистических ракет со сплэшем!",
            textures.icon_missiles.as_ref(),
            Color::new(1.0, 0.6, 0.2, 1.0),
        ),
    ];

    let weapons_page3 = [
        (
            WeaponType::SacredScriptures,
            "21. Орбитальный Гримуар -> ИНФЕРНАЛЬНАЯ ЛИТУРГИЯ",
            "Рецепт: Орбитальный Гримуар (ур. 8) + Фолиант Времени",
            "Базовый: Священные фолианты непрерывно кружатся вокруг босса.",
            "ЭВОЛЮЦИЯ: Пылающие адские скрижали с бесконечным пробитием врагов!",
            textures.icon_cooldown_tome.as_ref().or(textures.icon_book.as_ref()),
            GOLD,
        ),
        (
            WeaponType::FaithBoomerang,
            "22. Клинок Веры -> НЕБЕСНЫЙ ВЕРДИКТ",
            "Рецепт: Клинок Веры (ур. 8) + Клевер Судьбы",
            "Базовый: Вращающийся крест-бумеранг, рассекающий ряды монстров.",
            "ЭВОЛЮЦИЯ: Каскад святых клинков судного дня с повышенным критом!",
            textures.icon_sword.as_ref().or(textures.proj_holy.as_ref()),
            Color::new(0.5, 0.9, 1.0, 1.0),
        ),
        (
            WeaponType::CursedChalice,
            "23. Осквернённая Чаша -> КРОВАВЫЙ ПОТОП",
            "Рецепт: Осквернённая Чаша (ур. 8) + Сердце Гиганта",
            "Базовый: Проклятая чаша, детонирующая фонтаном ядовитой крови.",
            "ЭВОЛЮЦИЯ: Непрерывный кровавый шторм, исцеляющий босса от павших врагов!",
            textures.icon_bottle.as_ref().or(textures.icon_chest.as_ref()),
            Color::new(0.9, 0.2, 0.3, 1.0),
        ),
        (
            WeaponType::VampiricAura,
            "24. Аура Вампиризма -> ПИРШЕСТВО ДУШ",
            "Рецепт: Аура Вампиризма (ур. 8) + Кровавый Камень",
            "Базовый: Алая аура, непрерывно вытягивающая жизнь из мобов в радиусе.",
            "ЭВОЛЮЦИЯ: Колоссальный радиус кражи HP и постоянный рост силы за души!",
            textures.icon_blood_stone.as_ref().or(textures.icon_fire_aura.as_ref()),
            Color::new(1.0, 0.15, 0.25, 1.0),
        ),
        (
            WeaponType::RunicPrism,
            "25. Руническая Призма -> ИСКРИВЛЕНИЕ ПРОСТРАНСТВА",
            "Рецепт: Руническая Призма (ур. 8) + Призматический Фокус",
            "Базовый: Световые лучи, преломляющиеся между скоплениями врагов.",
            "ЭВОЛЮЦИЯ: Пространственные разломы, расщепляющие врагов на атомы!",
            textures.icon_prismatic.as_ref().or(textures.icon_jewel.as_ref()),
            Color::new(0.7, 0.4, 1.0, 1.0),
        ),
        (
            WeaponType::AbyssalBeam,
            "26. Столп Бездны -> КАСКАД АПОКАЛИПСИСА",
            "Рецепт: Столп Бездны (ур. 8) + Камень Проклятия Бездны",
            "Базовый: Нисходящий луч Бездны, выжигающий область арены.",
            "ЭВОЛЮЦИЯ: Орбитальная бомбардировка десятками черных лучей смерти!",
            textures.icon_void_rift.as_ref().or(textures.icon_brimstone.as_ref()),
            Color::new(0.85, 0.2, 0.95, 1.0),
        ),
        (
            WeaponType::ChronoStasis,
            "27. Хроно-Стазис -> БЕСКОНЕЧНАЯ ДИЛАТАЦИЯ",
            "Рецепт: Хроно-Стазис (ур. 8) + Песочные Часы",
            "Базовый: Купол замедления времени, тормозящий мобов и снаряды на 60%.",
            "ЭВОЛЮЦИЯ: Полная заморозка времени для всех врагов на 2.5 сек каждые 8 сек!",
            textures.icon_chrono_hourglass.as_ref().or(textures.icon_hourglass.as_ref()),
            Color::new(0.4, 0.8, 1.0, 1.0),
        ),
        (
            WeaponType::AstralCloak,
            "28. Астральный Саван -> САВАН КРОВАВОГО ИМПЕРАТОРА",
            "Рецепт: Астральный Саван (ур. 8) + Броня Света",
            "Базовый: Астральные заряды, полностью поглощающие входящий урон раз в 12 сек.",
            "ЭВОЛЮЦИЯ: Блокирует удары, ограничивает урон макс. 12 HP и взрывается кровью!",
            textures.icon_armor.as_ref().or(textures.icon_shield.as_ref()),
            GOLD,
        ),
    ];

    let active_weapons: &[(WeaponType, &str, &str, &str, &str, Option<&Texture2D>, Color)] = match current_page {
        0 => &weapons_page1,
        1 => &weapons_page2,
        _ => &weapons_page3,
    };

    let grid_y = y + 34.0;
    let grid_h = h - 38.0;
    let col_w = (w - 14.0) / 2.0;
    let row_h = (grid_h - 15.0) / 5.0;

    for (i, (w_kind, name, recipe, base_desc, evo_desc, icon, col)) in active_weapons.iter().enumerate() {
        let col_idx = (i / 5) as f32;
        let row_idx = (i % 5) as f32;

        let cx = x + col_idx * (col_w + 14.0);
        let cy = grid_y + row_idx * (row_h + 3.0);
        let card_h = row_h - 2.0;

        let is_card_hover = mx >= cx && mx <= cx + col_w && my >= cy && my <= cy + card_h;

        let btn_w = 84.0;
        let btn_h = 22.0;
        let btn_x = cx + col_w - btn_w - 6.0;
        let btn_y = cy + 5.0;
        let is_btn_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;

        let bg_col = if is_card_hover && !is_btn_hover {
            Color::new(0.14, 0.13, 0.20, 0.95)
        } else {
            Color::new(0.11, 0.11, 0.16, 0.92)
        };
        draw_rectangle(cx, cy, col_w, card_h, bg_col);
        draw_rectangle_lines(cx, cy, col_w, card_h, if is_card_hover { 2.0 } else { 1.5 }, if is_card_hover { GOLD } else { Color::new(0.35, 0.3, 0.45, 0.8) });

        // Отрисовка кнопки [i] Рецепт
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, if is_btn_hover { Color::new(0.38, 0.18, 0.50, 0.95) } else { Color::new(0.16, 0.16, 0.22, 0.90) });
        draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, if is_btn_hover { 1.5 } else { 1.0 }, if is_btn_hover { GOLD } else { GRAY });
        let blw = fonts.measure_heading("Синтез [i]", 10.5).width;
        fonts.draw_heading("Синтез [i]", btn_x + (btn_w - blw) / 2.0, btn_y + 15.0, 10.5, if is_btn_hover { GOLD } else { WHITE });

        if is_mouse_button_pressed(MouseButton::Left) && (is_btn_hover || is_card_hover) {
            requested_action = Some(CodexResult::ItemInspection(ItemInspectionTarget::Weapon(*w_kind, 1, 0)));
        }

        if let Some(tex) = icon {
            let isize = 40.0;
            draw_texture_ex(
                tex,
                cx + 10.0,
                cy + (card_h - isize) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(isize, isize)),
                    ..Default::default()
                },
            );
        }

        let tx = cx + 58.0;
        let text_max_w = col_w - 68.0;

        fonts.draw_heading(name, tx, cy + 17.0, 13.0, *col);
        fonts.draw_text(recipe, tx, cy + 32.0, 11.0, GOLD);
        fonts.draw_text_wrapped(base_desc, tx, cy + 46.0, 10.5, 13.5, text_max_w, LIGHTGRAY);
        fonts.draw_text_wrapped(evo_desc, tx, cy + 68.0, 10.5, 13.5, text_max_w, YELLOW);
    }

    requested_action
}

/// Вкладка 3: Пассивные предметы и Легендарные Реликвии Босса
fn draw_tab_passives(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) -> Option<CodexResult> {
    let mut requested_action = None;
    let (mx, my) = mouse_position();
    let current_page = PASSIVES_PAGE.load(Ordering::Relaxed);

    // Верхние переключатели страниц Пассивок
    let p_btn_w = 210.0;
    let p_btn_h = 28.0;
    let p1_x = x;
    let p2_x = x + p_btn_w + 10.0;
    let p3_x = x + (p_btn_w + 10.0) * 2.0;
    let p_btn_y = y;

    let p1_hover = mx >= p1_x && mx <= p1_x + p_btn_w && my >= p_btn_y && my <= p_btn_y + p_btn_h;
    let p2_hover = mx >= p2_x && mx <= p2_x + p_btn_w && my >= p_btn_y && my <= p_btn_y + p_btn_h;
    let p3_hover = mx >= p3_x && mx <= p3_x + p_btn_w && my >= p_btn_y && my <= p_btn_y + p_btn_h;

    // Кнопка Страница 1 (Базовые пассивки 1-14)
    draw_rectangle(p1_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 0 { Color::new(0.24, 0.18, 0.38, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
    draw_rectangle_lines(p1_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 0 { 2.0 } else { 1.0 }, if current_page == 0 { GOLD } else { GRAY });
    fonts.draw_heading("СТРАНИЦА 1: ПАССИВКИ [1-14]", p1_x + 10.0, p_btn_y + 19.0, 11.5, if current_page == 0 { GOLD } else { LIGHTGRAY });
    if is_mouse_button_pressed(MouseButton::Left) && p1_hover {
        PASSIVES_PAGE.store(0, Ordering::Relaxed);
    }

    // Кнопка Страница 2 (Легендарные реликвии 15-28)
    draw_rectangle(p2_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 1 { Color::new(0.38, 0.14, 0.22, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
    draw_rectangle_lines(p2_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 1 { 2.0 } else { 1.0 }, if current_page == 1 { GOLD } else { GRAY });
    fonts.draw_heading("СТРАНИЦА 2: РЕЛИКВИИ [15-28]", p2_x + 10.0, p_btn_y + 19.0, 11.5, if current_page == 1 { GOLD } else { LIGHTGRAY });
    if is_mouse_button_pressed(MouseButton::Left) && p2_hover {
        PASSIVES_PAGE.store(1, Ordering::Relaxed);
    }

    // Кнопка Страница 3 (Высшие артефакты 29-35)
    draw_rectangle(p3_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 2 { Color::new(0.18, 0.32, 0.44, 0.98) } else { Color::new(0.10, 0.10, 0.14, 0.90) });
    draw_rectangle_lines(p3_x, p_btn_y, p_btn_w, p_btn_h, if current_page == 2 { 2.0 } else { 1.0 }, if current_page == 2 { GOLD } else { GRAY });
    fonts.draw_heading("СТРАНИЦА 3: АРТЕФАКТЫ [29-35]", p3_x + 10.0, p_btn_y + 19.0, 11.5, if current_page == 2 { GOLD } else { LIGHTGRAY });
    if is_mouse_button_pressed(MouseButton::Left) && p3_hover {
        PASSIVES_PAGE.store(2, Ordering::Relaxed);
    }

    let passives_page1 = [
        (
            PassiveType::SpeedGloves,
            "1. Перчатка Скорости",
            "-10% времени перезарядки атак за ур. (до -50%).",
            "Эволюция: Клинок Ветра -> БУРЯ КЛИНКОВ",
            textures.icon_speed_gloves.as_ref().or(textures.icon_glove.as_ref()),
            SKYBLUE,
        ),
        (
            PassiveType::GiantHeart,
            "2. Сердце Гиганта",
            "+25% к макс. HP и +3.0 HP/сек регенерации за ур.",
            "Эволюция: Огненная Аура -> АДСКОЕ ПЕКЛО",
            textures.icon_giant_heart.as_ref().or(textures.icon_chest.as_ref()),
            RED,
        ),
        (
            PassiveType::CooldownTome,
            "3. Фолиант Времени",
            "-8% ко всем кулдаунам оружия и рывка (Dash).",
            "Эволюция: Магический Жезл -> ЛУЧ СМЕРТИ",
            textures.icon_cooldown_tome.as_ref().or(textures.icon_book.as_ref()),
            VIOLET,
        ),
        (
            PassiveType::LightArmor,
            "4. Броня Света",
            "+3 к плоской броне и +8% к сопротивлению физ. урону.",
            "Эволюция: Святой Молот -> ПРАВЕДНЫЙ ЩИТ",
            textures.icon_light_armor.as_ref().or(textures.icon_armor.as_ref()),
            GOLD,
        ),
        (
            PassiveType::FrostRing,
            "5. Кольцо Заморозки",
            "+20% к радиусу замедления и силе заморозки.",
            "Эволюция: Ледяные Осколки -> ЛЕДНИКОВАЯ БУРЯ",
            textures.icon_frost_ring.as_ref().or(textures.icon_ring.as_ref()),
            Color::new(0.4, 0.85, 1.0, 1.0),
        ),
        (
            PassiveType::CorrosiveFlask,
            "6. Едкий Сосуд",
            "Игнорирует 15% брони врагов и продлевает лужи яда.",
            "Эволюция: Чумные Сферы -> ТОКСИЧНЫЙ МОР",
            textures.icon_corrosive_flask.as_ref().or(textures.icon_bottle.as_ref()),
            LIME,
        ),
        (
            PassiveType::BloodStone,
            "7. Кровавый Камень",
            "+12% к общему урону и +2.5 HP/сек регенерации.",
            "Эволюция: Топор Палача -> ВИХРЕВОЙ СЕКАЧ",
            textures.icon_blood_stone.as_ref().or(textures.icon_bottle.as_ref()),
            RED,
        ),
        (
            PassiveType::EagleScope,
            "8. Орлиный Прицел",
            "+20% к скорости и дальности полета снарядов.",
            "Эволюция: Небесный Лук -> ЗВЕЗДОПАДНЫЙ ЗАЛП",
            textures.icon_eagle_scope.as_ref().or(textures.icon_ring.as_ref()),
            GOLD,
        ),
        (
            PassiveType::ChronoHourglass,
            "9. Песочные Часы",
            "+25% к длительности заморозки, горения и яда.",
            "Эволюция: Грозовая Сфера -> ГНЕВ ГРОЗЫ",
            textures.icon_chrono_hourglass.as_ref().or(textures.icon_hourglass.as_ref()),
            YELLOW,
        ),
        (
            PassiveType::MagnetSphere,
            "10. Сфера Притяжения",
            "+45 px к радиусу сбора кристаллов душ за ур.",
            "Эволюция: Кровавые Кинжалы -> КРОВАВАЯ ЖАТВА",
            textures.icon_magnet_sphere.as_ref().or(textures.icon_jewel.as_ref()),
            SKYBLUE,
        ),
        (
            PassiveType::TeslaCoil,
            "11. Катушка Теслы",
            "+12% шанс цепной молнии на 3 цели за ур.",
            "Эволюция: Гравитационная Сфера / Цепная Молния",
            textures.icon_tesla_coil.as_ref(),
            Color::new(0.3, 0.8, 1.0, 1.0),
        ),
        (
            PassiveType::CloverOfFate,
            "12. Клевер Судьбы",
            "+12% шанс критического удара (х2 урон) за ур.",
            "Эволюция: Удваивает весь исходящий крит!",
            textures.icon_clover.as_ref(),
            Color::new(0.3, 0.9, 0.4, 1.0),
        ),
        (
            PassiveType::PrismaticFocus,
            "13. Призматический Фокус",
            "+15% к радиусу поражения всех атак (AoE) за ур.",
            "Эволюция: Кровавый Луч -> ИНФЕРНАЛЬНЫЙ КАТАКЛИЗМ",
            textures.icon_prismatic.as_ref(),
            Color::new(0.9, 0.5, 0.9, 1.0),
        ),
        (
            PassiveType::TitanAegis,
            "14. Черный Эгис",
            "-2.5 плоского входящего урона от каждого удара.",
            "Эволюция: Рой Микроракет -> ОСАДА СУДНОГО ДНЯ",
            textures.icon_aegis.as_ref(),
            GOLD,
        ),
    ];

    let passives_page2 = [
        (
            PassiveType::OctarineCore,
            "15. Октариновое Ядро",
            "+25% снижение отката всех способностей, 6% вампиризм от заклинаний.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Безграничная магическая мощь.",
            textures.icon_octarine_core.as_ref().or(textures.icon_jewel.as_ref()),
            VIOLET,
        ),
        (
            PassiveType::DivineRapier,
            "16. Божественная Рапира",
            "+50% ко всему наносимому урону, +35% пробития брони врагов.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Сокрушительный удар паладина.",
            textures.icon_divine_rapier.as_ref().or(textures.icon_sword.as_ref()),
            GOLD,
        ),
        (
            PassiveType::SolarRelic,
            "17. Солнечная Реликвия",
            "+40 Святого урона/сек всем мобам в радиусе 220 px вокруг босса.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Испепеляющее сияние небес.",
            textures.icon_solar_relic.as_ref().or(textures.icon_elixir.as_ref()),
            GOLD,
        ),
        (
            PassiveType::FateWeaver,
            "18. Ткач Судеб",
            "+1 дополнительный снаряд ко всем атакам, +15% шанс крита.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Нити судьбы множат снаряды.",
            textures.icon_fate_weaver.as_ref().or(textures.icon_hourglass.as_ref()),
            Color::new(0.4, 0.9, 1.0, 1.0),
        ),
        (
            PassiveType::ImmortalAegis,
            "19. Эгида Бессмертия",
            "ВОСКРЕШЕНИЕ: При гибели босс восстает со 100% HP и отбрасывает орду!",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Второе дыхание в смертельном бою.",
            textures.icon_immortal_aegis.as_ref().or(textures.icon_shield.as_ref()),
            GOLD,
        ),
        (
            PassiveType::ColossalEye,
            "20. Око Колосса",
            "+50% к размеру всех снарядов и +2 пробития сквозь толпу.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Гигантские снаряды сметают строй.",
            textures.icon_colossal_eye.as_ref().or(textures.icon_jewel.as_ref()),
            RED,
        ),
        (
            PassiveType::SoulHarvester,
            "21. Призрачный Сосуд",
            "+25% к макс HP, призывает 2 призраков-защитников на арену.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Защита верных духов.",
            textures.icon_spectral_jar.as_ref().or(textures.icon_bottle.as_ref()),
            SKYBLUE,
        ),
        (
            PassiveType::ArchmageShard,
            "22. Осколок Архимага",
            "+20% к урону, все заклинания пробивают +1 дополнительного врага.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Сквозная магическая волна.",
            textures.icon_archmage_shard.as_ref().or(textures.icon_wand.as_ref()),
            Color::new(0.85, 0.4, 1.0, 1.0),
        ),
        (
            PassiveType::Pyromaniac,
            "23. Сердце Пепла",
            "+30% к урону огнем, поджигает врагов и восстанавливает 8% HP.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Вампиризм пламени очищения.",
            textures.icon_cinder_heart.as_ref().or(textures.icon_chest.as_ref()),
            ORANGE,
        ),
        (
            PassiveType::StaticCore,
            "24. Ядро Буревестника",
            "+20% к урону молний, оглушает пораженных врагов на 0.4 сек.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Статический шок парализует мобов.",
            textures.icon_stormhammer_core.as_ref().or(textures.icon_tesla_coil.as_ref()),
            YELLOW,
        ),
        (
            PassiveType::SiegeMortar,
            "25. Мортира Бедствия",
            "+40% к урону по площади (AoE), сотрясает ряды наступающих.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Сокрушительная взрывная мощь.",
            textures.icon_calamity_mortar.as_ref().or(textures.icon_cinder_heart.as_ref()),
            Color::new(1.0, 0.4, 0.2, 1.0),
        ),
        (
            PassiveType::UkuleleResonance,
            "26. Резонирующие Струны",
            "+25% к радиусу поражения, враги резонируют на 30 урона вокруг.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Музыка битвы разрывает плоть.",
            textures.icon_resonant_strings.as_ref().or(textures.icon_bow.as_ref()),
            Color::new(0.3, 0.9, 0.7, 1.0),
        ),
        (
            PassiveType::BlessedHeart,
            "27. Благословенное Ядро",
            "+35% к эффективности лечения и регенерации, +4 к броне.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Неиссякаемый источник благодати.",
            textures.icon_blessed_core.as_ref().or(textures.icon_shield.as_ref()),
            GOLD,
        ),
        (
            PassiveType::DivinityAura,
            "28. Сияющий Нимб",
            "+20% к урону святостью, ослепляет врагов в радиусе 160 px.",
            "ЛЕГЕНДАРНАЯ РЕЛИКВИЯ: Святое сияние истинного Паладина.",
            textures.icon_halo_radiance.as_ref().or(textures.icon_armor.as_ref()),
            GOLD,
        ),
    ];

    let passives_page3 = [
        (
            PassiveType::SporeMushroom,
            "29. Споровый Мицелий",
            "Пускает корни при остановке: +18 HP/с (+1.5% макс HP), игнорируя урон и врагов!",
            "АРТЕФАКТ: Непрерывная биолюминесцентная регенерация.",
            textures.icon_spore_mushroom.as_ref(),
            Color::new(0.2, 0.95, 0.6, 1.0),
        ),
        (
            PassiveType::DuplicatorRelic,
            "30. Реликвия Раздвоения",
            "+1 дополнительный снаряд/орбиталка ко всем навыкам за каждый уровень!",
            "КЛЮЧЕВОЙ АРТЕФАКТ: Умножает огневую мощь и плотность щитов в 1.5 - 2 раза.",
            textures.icon_prismatic.as_ref().or(textures.icon_jewel.as_ref()),
            GOLD,
        ),
        (
            PassiveType::CandelabraOfFury,
            "31. Канделябр Ярости",
            "+25% к площади действия (AoE) всех аур, взрывов и орбитальных барьеров.",
            "КЛЮЧЕВОЙ АРТЕФАКТ: Огненная Аура и Буря Клинков охватывают огромную зону.",
            textures.icon_cinder_heart.as_ref().or(textures.icon_fire_aura.as_ref()),
            ORANGE,
        ),
        (
            PassiveType::AbyssalCurseStone,
            "32. Камень Проклятия Бездны",
            "+40% к частоте спавна и скорости мобов, но +75% к получаемому опыту (XP)!",
            "АРТЕФАКТ ЭВОЛЮЦИИ: Столп Бездны -> КАСКАД АПОКАЛИПСИСА.",
            textures.icon_void_rift.as_ref().or(textures.icon_jewel.as_ref()),
            PURPLE,
        ),
        (
            PassiveType::SilverTears,
            "33. Серебряные Слёзы",
            "Периодическое благословение: мгновенное исцеление +10 HP каждые 3 секунды.",
            "СВЯТОСТЬ: Защищает босса от периодического DoT-урона яда и горения.",
            textures.icon_halo_radiance.as_ref().or(textures.icon_elixir.as_ref()),
            SKYBLUE,
        ),
        (
            PassiveType::GoldenSeal,
            "34. Золотая Печать",
            "+15% к скорости набора опыта (XP) и -12% ко всем перезарядкам способностей.",
            "ИМПЕРСКАЯ РЕЛИКВИЯ: Ускоряет достижение 8 уровня для эволюций.",
            textures.icon_solar_relic.as_ref().or(textures.icon_ring.as_ref()),
            GOLD,
        ),
        (
            PassiveType::TitanBloodArmor,
            "35. Броня Титанической Крови",
            "+30% к максимальному запасу здоровья и -15% входящего физического урона.",
            "НЕСОКРУШИМОСТЬ: Превращает босса в неприступную живую крепость.",
            textures.icon_light_armor.as_ref().or(textures.icon_armor.as_ref()),
            RED,
        ),
        (
            PassiveType::BerserkerBlood,
            "36. Кровь Берсерка",
            "Регенерация HP и скорость атаки оружия растут тем сильнее, чем ниже здоровье босса.",
            "СВЯЗКА ДЛЯ ВЫЖИВАНИЯ: Превращает предсмертное состояние в сокрушительную ярость.",
            textures.icon_blood_stone.as_ref().or(textures.icon_sword.as_ref()),
            RED,
        ),
        (
            PassiveType::MartyrVanguard,
            "37. Авангард Мучеников",
            "Скорость бега растёт пропорционально ранам союзников. Убийство любого моба лечит +1.5 HP.",
            "КОМАНДНЫЙ СТРАЖ: Стремительное спасение раненых соратников и вампиризм.",
            textures.icon_speed_gloves.as_ref().or(textures.icon_aegis.as_ref()),
            GOLD,
        ),
    ];

    let active_passives = match current_page {
        0 => passives_page1.as_slice(),
        1 => passives_page2.as_slice(),
        _ => passives_page3.as_slice(),
    };

    let grid_y = y + 34.0;
    let grid_h = h - 38.0;
    let col_w = (w - 14.0) / 2.0;
    let rows_per_col = ((active_passives.len() + 1) / 2).max(1);
    let row_h = (grid_h - 15.0) / (rows_per_col as f32);

    for (i, (p_kind, name, effect, evo, icon, col)) in active_passives.iter().enumerate() {
        let col_idx = (i / rows_per_col) as f32;
        let row_idx = (i % rows_per_col) as f32;

        let cx = x + col_idx * (col_w + 14.0);
        let cy = grid_y + row_idx * (row_h + 3.0);
        let card_h = row_h - 2.0;

        let is_card_hover = mx >= cx && mx <= cx + col_w && my >= cy && my <= cy + card_h;

        let btn_w = 78.0;
        let btn_h = 20.0;
        let btn_x = cx + col_w - btn_w - 6.0;
        let btn_y = cy + 4.0;
        let is_btn_hover = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;

        let bg_col = if is_card_hover && !is_btn_hover {
            Color::new(0.14, 0.13, 0.20, 0.95)
        } else {
            Color::new(0.11, 0.11, 0.16, 0.92)
        };
        draw_rectangle(cx, cy, col_w, card_h, bg_col);
        draw_rectangle_lines(cx, cy, col_w, card_h, if is_card_hover { 2.0 } else { 1.5 }, if is_card_hover { GOLD } else { Color::new(0.3, 0.35, 0.45, 0.8) });

        // Отрисовка кнопки [i] Инфо
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, if is_btn_hover { Color::new(0.38, 0.18, 0.50, 0.95) } else { Color::new(0.16, 0.16, 0.22, 0.90) });
        draw_rectangle_lines(btn_x, btn_y, btn_w, btn_h, if is_btn_hover { 1.5 } else { 1.0 }, if is_btn_hover { GOLD } else { GRAY });
        let blw = fonts.measure_heading("Инфо [i]", 10.0).width;
        fonts.draw_heading("Инфо [i]", btn_x + (btn_w - blw) / 2.0, btn_y + 14.0, 10.0, if is_btn_hover { GOLD } else { WHITE });

        if is_mouse_button_pressed(MouseButton::Left) && (is_btn_hover || is_card_hover) {
            requested_action = Some(CodexResult::ItemInspection(ItemInspectionTarget::Passive(*p_kind, 1)));
        }

        if let Some(tex) = icon {
            let isize = 36.0;
            draw_texture_ex(
                tex,
                cx + 10.0,
                cy + (card_h - isize) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(isize, isize)),
                    ..Default::default()
                },
            );
        }

        let tx = cx + 54.0;
        let text_max_w = col_w - 64.0;

        fonts.draw_heading(name, tx, cy + 15.0, 13.0, *col);
        fonts.draw_text_wrapped(effect, tx, cy + 28.0, 10.5, 13.5, text_max_w, WHITE);
        fonts.draw_text_wrapped(evo, tx, cy + 49.0, 10.5, 13.5, text_max_w, GOLD);
    }

    requested_action
}

/// Вкладка 4: Алтари, Экономика и Механики
fn draw_tab_mechanics(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts) {
    let half_w = (w - 16.0) / 2.0;
    let card_h = (h - 16.0) / 2.0;
    let text_w = half_w - 24.0;

    // Карточка 1: Алтарь Крови
    draw_card_box(x, y, half_w, card_h, "АЛТАРЬ КРОВИ (ПКМ НА КАРТЕ)", fonts);
    let mut cy1 = y + 40.0;
    cy1 = fonts.draw_text_wrapped("• Постройка: Клик Правой Кнопкой Мыши (ПКМ) в любой точке арены.", x + 12.0, cy1, 11.0, 14.5, text_w, WHITE) + 3.0;
    cy1 = fonts.draw_text_wrapped("• Стоимость: 50 Очков Душ. Здоровье: 650 HP (Броня 12, уязвим к Святости).", x + 12.0, cy1, 11.0, 14.5, text_w, GOLD) + 3.0;
    cy1 = fonts.draw_text_wrapped("• Пассивный доход: Каждый живой алтарь даёт +3.0 души каждые 2 секунды.", x + 12.0, cy1, 11.0, 14.5, text_w, GREEN) + 3.0;
    cy1 = fonts.draw_text_wrapped("• Аура исцеления: Каждые 3 сек лечит союзных мобов в 180 px на +35 HP.", x + 12.0, cy1, 11.0, 14.5, text_w, RED) + 3.0;
    cy1 = fonts.draw_text_wrapped("• ТАКТИКА: Босс распознаёт алтари как стратегическую угрозу и идёт их штурмовать!", x + 12.0, cy1, 11.0, 14.5, text_w, SKYBLUE) + 3.0;
    fonts.draw_text_wrapped("[!] Защищайте алтари, иначе босс сломает их и получит +150 XP и Магнит!", x + 12.0, cy1, 11.0, 14.5, text_w, YELLOW);

    // Карточка 2: Экономика и Грейды Монстров
    let x2 = x + half_w + 16.0;
    draw_card_box(x2, y, half_w, card_h, "ЭКОНОМИКА И ГРЕЙДЫ МОБОВ [F, G, H]", fonts);
    let mut cy2 = y + 40.0;
    cy2 = fonts.draw_text_wrapped("• Базовый доход: +8 Душ/2с. Разгон экономики: +2% от суммы каждой траты душ.", x2 + 12.0, cy2, 11.0, 14.5, text_w, WHITE) + 3.0;
    cy2 = fonts.draw_text_wrapped("• ГРЕЙД [F] Ярость Крови: +12% урон, +6% скорость всех мобов.", x2 + 12.0, cy2, 11.0, 14.5, text_w, RED) + 3.0;
    cy2 = fonts.draw_text_wrapped("• ГРЕЙД [G] Панцирь: +2.5 плоской брони, +5% физ. защиты мобов.", x2 + 12.0, cy2, 11.0, 14.5, text_w, SKYBLUE) + 3.0;
    cy2 = fonts.draw_text_wrapped("• ГРЕЙД [H] Чумные Споры: Атаки мобов заражают босса ядом (DoT); Алтари Крови защищены миазмами.", x2 + 12.0, cy2, 11.0, 14.5, text_w, GREEN) + 3.0;
    fonts.draw_text_wrapped("• ЭЛИТНЫЕ ЧЕМПИОНЫ: 12% шанс спавна моба с золотой короной, +80% HP, +35% урона и 2.5x дропом душ и опыта!", x2 + 12.0, cy2, 11.0, 14.5, text_w, GOLD);

    // Карточка 3: Интеллект Босса (Потенциальные поля и Dash)
    let y2 = y + card_h + 16.0;
    draw_card_box(x, y2, half_w, card_h, "ТАКТИЧЕСКИЙ ИНТЕЛЛЕКТ БОССА", fonts);
    let mut cy3 = y2 + 40.0;
    cy3 = fonts.draw_text_wrapped("• Приоритеты огня: Оружие босса целится в культистов-стрелков и Алтари!", x + 12.0, cy3, 11.0, 14.5, text_w, WHITE) + 3.0;
    cy3 = fonts.draw_text_wrapped("• Анти-снайперский натиск: При накоплении дальников босс идёт на сближение.", x + 12.0, cy3, 11.0, 14.5, text_w, SKYBLUE) + 3.0;
    cy3 = fonts.draw_text_wrapped("• Парирование атак: Вращающиеся щиты и Буря Клинков сбивают стрелы культистов!", x + 12.0, cy3, 11.0, 14.5, text_w, YELLOW) + 3.0;
    fonts.draw_text_wrapped("• Экстренный рывок (8-лучевой Context Steering Dash): Срабатывает при окружении или HP <30%. Рывок на 180 px наносит 80 урона по пути!", x + 12.0, cy3, 11.0, 14.5, text_w, GOLD);

    // Карточка 4: Анти-Пассивность (Медитация) и Сноуболл
    draw_card_box(x2, y2, half_w, card_h, "АНТИ-ПАССИВНОСТЬ И LIMIT BREAK", fonts);
    let mut cy4 = y2 + 40.0;
    cy4 = fonts.draw_text_wrapped("• МЕДИТАЦИЯ ПОКОЯ: Если босс стоит без движения > 1.2 сек, не получал урона последние 3 секунды и рядом нет врагов (в радиусе 300 px): восстанавливает +2% HP/с и +30 XP/с. Любой входящий урон сбивает медитацию на 3 сек!", x2 + 12.0, cy4, 10.5, 14.0, text_w, RED) + 3.0;
    cy4 = fonts.draw_text_wrapped("• СПОРОВЫЙ МИЦЕЛИЙ (Реликвия 29): Если босс находит грибницу, при остановке корни лечат его (+18 HP/с + 1.5% макс HP) вне зависимости от атак и урона!", x2 + 12.0, cy4, 10.5, 14.0, text_w, GREEN) + 3.0;
    cy4 = fonts.draw_text_wrapped("• ЗАПРЕДЕЛЬНОЕ ВОЗВЫШЕНИЕ (Limit Break после 10-й мин):", x2 + 12.0, cy4, 10.5, 14.0, text_w, GOLD) + 3.0;
    fonts.draw_text_wrapped("  После эволюции всех 6 слотов босс бесконечно качает: +10% урона, -8% отката, +1 снаряд, Закалку Титана (+80 HP) и Адаптивную Броню (+2.0 брони).", x2 + 12.0, cy4, 11.0, 14.5, text_w, WHITE);
}

fn draw_card_box(x: f32, y: f32, w: f32, h: f32, title: &str, fonts: &GameFonts) {
    draw_rectangle(x, y, w, h, Color::new(0.10, 0.10, 0.15, 0.94));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(0.4, 0.35, 0.5, 0.9));
    draw_rectangle(x, y, w, 28.0, Color::new(0.15, 0.14, 0.22, 1.0));
    draw_rectangle_lines(x, y, w, 28.0, 1.0, DARKGRAY);
    fonts.draw_text(title, x + 12.0, y + 19.0, 14.0, GOLD);
}
