use macroquad::prelude::*;
use crate::render::font::GameFonts;
use crate::render::textures::GameTextures;

pub fn draw_codex(current_tab: &mut usize, fonts: &GameFonts, textures: &GameTextures) -> Option<u8> {
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
        1 => {
            draw_tab_weapons(content_x, content_y, content_w, content_h, fonts, textures);
            None
        }
        2 => {
            draw_tab_passives(content_x, content_y, content_w, content_h, fonts, textures);
            None
        }
        _ => {
            draw_tab_mechanics(content_x, content_y, content_w, content_h, fonts);
            None
        }
    }
}

/// Вкладка 1: Бестиарий мобов (с подробными характеристиками и кнопкой [i] Досье)
fn draw_tab_bestiary(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) -> Option<u8> {
    let mut requested_tier = None;
    let (mx, my) = mouse_position();

    let mobs_data = [
        (
            1u8,
            "Тир 1: Скелеты-пехотинцы",
            "Цена: 15 душ | Отряд: 8 шт | Дроп: 8 XP | Доход: +0.30/2с",
            "HP: 32 | Скор: 105 px/с | Урон: 7 физ (1.0 уд/с) | Дистанция: 24 px | Броня: 0",
            "Резисты: Физ +5% | Огонь 0% | Холод 0% | Святой -25% (Нежить) | Стойкость: 0%",
            "Роль: Пушечное мясо. Задерживают босса контактом и притормаживают на 2% за удар.",
            "Разблокировка: Доступны со старта игры (0 потрачено душ).",
            Color::new(0.9, 0.9, 0.95, 1.0),
        ),
        (
            2u8,
            "Тир 2: Чёрные гончие",
            "Цена: 35 душ | Отряд: 5 шт | Дроп: 20 XP | Доход: +0.70/2с",
            "HP: 50 | Скор: 160 px/с (⚡ +39% быстрее босса!) | Урон: 12 физ | Броня: 1",
            "Резисты: Физ +10% | Огонь 0% | Холод 0% | Святой -20% | Стойкость: 15%",
            "Роль: Предиктивный перехват. Бегут на опережение вектора скорости босса, срывая транс.",
            "Разблокировка: Открываются после суммарной траты 150 душ.",
            Color::new(0.95, 0.45, 0.45, 1.0),
        ),
        (
            3u8,
            "Тир 3: Культисты-стрелки",
            "Цена: 70 душ | Отряд: 5 шт | Дроп: 45 XP | Доход: +1.40/2с",
            "HP: 75 | Скор: 80 px/с | Урон: 16 маг (1 выстр / 2.0с) | Дистанция: 320 px | Броня: 2",
            "Резисты: Физ +15% | Огонь +20% | Холод -10% | Святой -40% (Слабость) | Стойкость: 0%",
            "Роль: Дальнобойный кайтинг. Вынуждают босса тратить Рывок. Приоритетная цель босса!",
            "Разблокировка: Открываются после суммарной траты 400 душ.",
            Color::new(0.85, 0.55, 1.0, 1.0),
        ),
        (
            4u8,
            "Тир 4: Ледяные големы",
            "Цена: 150 душ | Отряд: 2 шт | Дроп: 130 XP | Доход: +3.00/2с",
            "HP: 380 | Скор: 52 px/с | Урон: 32 лед (1.0 уд/с) | Броня: 14 | Иммунитет к отбросу!",
            "Резисты: Физ +35% | Лед 100% (ИММУНИТЕТ) | Огонь -50% (Уязвим) | Яд +50%",
            "Роль: Тяжёлые танки. Впитывают снаряды, блокируют проходы и замедляют босса ударами.",
            "Разблокировка: Открываются после суммарной траты 1000 душ.",
            Color::new(0.4, 0.85, 1.0, 1.0),
        ),
        (
            5u8,
            "Тир 5: Камикадзе (Огненные подрывники)",
            "Цена: 300 душ | Отряд: 3 шт | Дроп: 160 XP | Доход: +6.00/2с",
            "HP: 130 | Скор: 185 px/с (Сверхзвук) | Урон: 220 Чистый сплэш (Радиус: 95 px) | Броня: 2",
            "Резисты: Физ +10% | Огонь +50% | Холод 0% | Святой 0% | Стойкость: 0%",
            "Роль: Суицидальный прорыв. Взрываются при контакте, нанося 220 чистый урон по площади.",
            "Разблокировка: Открываются после суммарной траты 2500 душ.",
            Color::new(1.0, 0.55, 0.2, 1.0),
        ),
        (
            6u8,
            "Тир 6: Демоны теней (Shadow Demons)",
            "Цена: 500 душ | Отряд: 2 шт | Дроп: 240 XP | Доход: +10.00/2с",
            "HP: 550 | Скор: 140 px/с | Урон: 42 физ/тьма (1.0 уд/с) | Скачок: кд 4.5с | Броня: 8",
            "Резисты: Физ +25% | Огонь +20% | Холод +20% | Святой -10% | Стойкость: 30%",
            "Роль: Скрытный прорыв. Телепортируются за спину босса, сбивая траекторию движения и огонь.",
            "Разблокировка: Открываются после суммарной траты 4000 душ.",
            Color::new(0.75, 0.4, 0.9, 1.0),
        ),
        (
            7u8,
            "Тир 7: Огненные элементали",
            "Цена: 850 душ | Отряд: 2 шт | Дроп: 420 XP | Доход: +17.00/2с",
            "HP: 850 | Скор: 95 px/с | Урон: 55 огонь (1 выстр / 2.2с) | Дистанция: 360 px | Броня: 10",
            "Резисты: Огонь 100% (ПОЛНЫЙ ИММУНИТЕТ) | Физ +20% | Лед -40% | Яд +50%",
            "Роль: Огненный обстрел. Артиллерийские кометы и огненный след. Игнорируют ауры огня босса.",
            "Разблокировка: Открываются после суммарной траты 6500 душ.",
            Color::new(1.0, 0.35, 0.15, 1.0),
        ),
        (
            8u8,
            "Тир 8: Жнец Смерти (Титан)",
            "Цена: 1800 душ | Отряд: 1 шт | Дроп: 1200 XP | Доход: +36.00/2с",
            "HP: 2800 | Скор: 68 px/с | Урон: 95 True (1.0 уд/с) + Волна 420 px | Броня: 24 | Иммун",
            "Резисты: Физ +45% | Огонь +30% | Холод +30% | Святой -20% | Стойкость: 100% (Иммунитет)",
            "Роль: Мини-босс орды! Рассекающие удары косы сквозь любую защиту, иммунитет к отбросу.",
            "Разблокировка: Открываются после суммарной траты 10000 душ.",
            Color::new(0.9, 0.2, 0.3, 1.0),
        ),
    ];

    let col_w = (w - 14.0) / 2.0;
    let row_h = (h - 20.0) / 4.0;

    for (i, (tier, name, cost_info, stats, resists, role, unlock, col)) in mobs_data.iter().enumerate() {
        let col_idx = (i / 4) as f32;
        let row_idx = (i % 4) as f32;

        let cx = x + col_idx * (col_w + 14.0);
        let cy = y + row_idx * (row_h + 5.0);

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
        let blw = fonts.measure_heading("📖 Досье [i]", 11.0).width;
        fonts.draw_heading("📖 Досье [i]", btn_x + (btn_w - blw) / 2.0, btn_y + 16.0, 11.0, if is_btn_hover { GOLD } else { WHITE });

        // Иконка/спрайт моба слева
        if let Some(tex) = textures.get_mob_texture(*tier, 0) {
            let icon_s = 48.0;
            let (iw, ih) = if *tier == 2 { (icon_s * 1.3, icon_s * 0.7) } else { (icon_s, icon_s) };
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
        fonts.draw_heading(name, text_x, cy + 18.0, 12.5, *col);

        // 2. Параметры стоимости и экономики
        fonts.draw_text(cost_info, text_x, cy + 34.0, 10.0, GOLD);

        // 3. Боевые параметры (HP, скорость бега, урон, темп атаки, дальность, броня)
        fonts.draw_text(stats, text_x, cy + 48.0, 10.0, WHITE);

        // 4. Резисты и стойкость к отталкиванию
        fonts.draw_text(resists, text_x, cy + 62.0, 9.5, LIGHTGRAY);

        // 5. Тактическая роль (с авто-переносом по ширине карточки, чтобы текст НИКОГДА не вылезал)
        fonts.draw_text_wrapped(role, text_x, cy + 76.0, 10.0, 12.5, max_text_w, SKYBLUE);

        // 6. Условие разблокировки внизу карточки
        fonts.draw_text(unlock, text_x, cy + card_h - 7.0, 9.5, GREEN);

        // Клик по кнопке [i] или по всей карточке -> открывает полное досье
        if is_mouse_button_pressed(MouseButton::Left) && (is_btn_hover || is_card_hover) {
            requested_tier = Some(*tier);
        }
    }

    requested_tier
}

/// Вкладка 2: Оружие и Эволюции Босса (2 колонки по 5 карточек)
fn draw_tab_weapons(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) {
    let weapons_data = [
        (
            "1. Клинок Ветра -> БУРЯ КЛИНКОВ",
            "Рецепт: Клинок Ветра (ур. 8) + Перчатка Скорости",
            "Базовый: Серповидные клинки ветра по курсу бега.",
            "ЭВОЛЮЦИЯ: Вихрь из 8 вращающихся лезвий вокруг босса.",
            textures.icon_sword.as_ref(),
            SKYBLUE,
        ),
        (
            "2. Огненная Аура -> АДСКОЕ ПЕКЛО",
            "Рецепт: Огненная Аура (ур. 8) + Сердце Гиганта",
            "Базовый: Пульсирующая огненная волна (DoT горение).",
            "ЭВОЛЮЦИЯ: % урон от HP + детонация трупов горящих мобов!",
            textures.icon_elixir.as_ref(),
            ORANGE,
        ),
        (
            "3. Магический Жезл -> ЛУЧ СМЕРТИ",
            "Рецепт: Магический Жезл (ур. 8) + Фолиант Времени",
            "Базовый: Самонаводящиеся сгустки тайной магии.",
            "ЭВОЛЮЦИЯ: Прожигающий лазерный луч через весь экран.",
            textures.icon_wand.as_ref(),
            VIOLET,
        ),
        (
            "4. Святой Молот -> ПРАВЕДНЫЙ ЩИТ",
            "Рецепт: Святой Молот (ур. 8) + Броня Света",
            "Базовый: Тяжёлые удары святым молотом по площади.",
            "ЭВОЛЮЦИЯ: Вращающийся щит света с 12% вампиризмом в HP.",
            textures.icon_shield.as_ref(),
            GOLD,
        ),
        (
            "5. Ледяные Осколки -> ЛЕДНИКОВАЯ БУРЯ",
            "Рецепт: Ледяные Осколки (ур. 8) + Кольцо Заморозки",
            "Базовый: Веер ледяных стрел с замедлением на 40%.",
            "ЭВОЛЮЦИЯ: Пурга на пол-арены с полной заморозкой на 1.5с.",
            textures.icon_bow.as_ref(),
            Color::new(0.5, 0.9, 1.0, 1.0),
        ),
        (
            "6. Чумные Сферы -> ТОКСИЧНЫЙ МОР",
            "Рецепт: Чумные Сферы (ур. 8) + Едкий Сосуд",
            "Базовый: Ядовитые сферы, оставляющие токсичные лужи.",
            "ЭВОЛЮЦИЯ: Едкие лужи бездны, разъедающие броню монстров.",
            textures.icon_bottle.as_ref(),
            LIME,
        ),
        (
            "7. Топор Палача -> ВИХРЕВОЙ СЕКАЧ",
            "Рецепт: Топор Палача (ур. 8) + Кровавый Камень",
            "Базовый: Метательные секиры, прорубающие до 4 врагов.",
            "ЭВОЛЮЦИЯ: Шквал из 6 зазубренных секачей по спирали.",
            textures.icon_ax.as_ref(),
            RED,
        ),
        (
            "8. Небесный Лук -> ЗВЕЗДОПАДНЫЙ ЗАЛП",
            "Рецепт: Небесный Лук (ур. 8) + Орлиный Прицел",
            "Базовый: Скорострельные святые стрелы с пробитием.",
            "ЭВОЛЮЦИЯ: Залп из 8 сияющих стрел звездного света.",
            textures.icon_bow.as_ref(),
            GOLD,
        ),
        (
            "9. Грозовая Сфера -> ГНЕВ ГРОЗЫ",
            "Рецепт: Грозовая Сфера (ур. 8) + Песочные Часы",
            "Базовый: Шаровые молнии, бьющие чистым уроном.",
            "ЭВОЛЮЦИЯ: Каскадный шторм из 6 молний с оглушением.",
            textures.icon_jewel.as_ref(),
            YELLOW,
        ),
        (
            "10. Кровавые Кинжалы -> КРОВАВАЯ ЖАТВА",
            "Рецепт: Кровавые Кинжалы (ур. 8) + Сфера Притяжения",
            "Базовый: Скоростные серии метательных ножей.",
            "ЭВОЛЮЦИЯ: Веер из 8 клинков крови с 20% вампиризмом!",
            textures.icon_sword.as_ref(),
            Color::new(0.9, 0.2, 0.4, 1.0),
        ),
        (
            "11. Гравитационная Сфера -> ГОРИЗОНТ СОБЫТИЙ",
            "Рецепт: Гравитационная Сфера (ур. 8) + Катушка Теслы",
            "Базовый: Черная дыра, стягивающая мобов в эпицентр.",
            "ЭВОЛЮЦИЯ: Космическая червоточина с цепными электро-взрывами!",
            textures.icon_singularity.as_ref(),
            Color::new(0.7, 0.3, 0.9, 1.0),
        ),
        (
            "12. Цепной Ледяной Шар -> АБСОЛЮТНЫЙ НОЛЬ",
            "Рецепт: Цепной Ледяной Шар (ур. 8) + Клевер Судьбы",
            "Базовый: Прыгающая ледяная сфера, скачущая между целями.",
            "ЭВОЛЮЦИЯ: Каскад ледяных сфер с бесконечными отскоками и расколом!",
            textures.icon_chain_frost.as_ref(),
            Color::new(0.4, 0.85, 1.0, 1.0),
        ),
        (
            "13. Кровавый Гиперлуч -> ИНФЕРНАЛЬНЫЙ КАТАКЛИЗМ",
            "Рецепт: Кровавый Гиперлуч (ур. 8) + Призматический Фокус",
            "Базовый: Непрерывный плазменный лазер, прожигающий арену.",
            "ЭВОЛЮЦИЯ: Вращающийся крест из 4 плазменных лучей возмездия!",
            textures.icon_brimstone.as_ref(),
            Color::new(1.0, 0.2, 0.2, 1.0),
        ),
        (
            "14. Рой Микроракет -> ОСАДА СУДНОГО ДНЯ",
            "Рецепт: Рой Микроракет (ур. 8) + Черный Эгис",
            "Базовый: Самонаводящиеся микроракеты по плотным скоплениям.",
            "ЭВОЛЮЦИЯ: Залп тяжелых кассетных ракет с ядерными взрывами!",
            textures.icon_missiles.as_ref(),
            Color::new(1.0, 0.6, 0.2, 1.0),
        ),
    ];

    let col_w = (w - 14.0) / 2.0;
    let row_h = (h - 20.0) / 7.0;

    for (i, (name, recipe, base_desc, evo_desc, icon, col)) in weapons_data.iter().enumerate() {
        let col_idx = (i / 7) as f32;
        let row_idx = (i % 7) as f32;

        let cx = x + col_idx * (col_w + 14.0);
        let cy = y + row_idx * (row_h + 4.0);

        draw_rectangle(cx, cy, col_w, row_h, Color::new(0.11, 0.11, 0.16, 0.92));
        draw_rectangle_lines(cx, cy, col_w, row_h, 1.5, Color::new(0.35, 0.3, 0.45, 0.8));

        if let Some(tex) = icon {
            let isize = 38.0;
            draw_texture_ex(
                tex,
                cx + 10.0,
                cy + (row_h - isize) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(isize, isize)),
                    ..Default::default()
                },
            );
        }

        let tx = cx + 56.0;
        fonts.draw_heading(name, tx, cy + 16.0, 12.5, *col);
        fonts.draw_text(recipe, tx, cy + 32.0, 10.5, GOLD);
        fonts.draw_text(base_desc, tx, cy + 48.0, 10.5, LIGHTGRAY);
        fonts.draw_text(evo_desc, tx, cy + 64.0, 10.5, YELLOW);
    }
}

/// Вкладка 3: Пассивные предметы Босса (2 колонки по 5 карточек)
fn draw_tab_passives(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) {
    let passives_data = [
        (
            "1. Перчатка Скорости",
            "-10% времени перезарядки атак за ур. (до -50% на ур. 5).",
            "Эволюция: Клинок Ветра -> БУРЯ КЛИНКОВ",
            textures.icon_glove.as_ref(),
            SKYBLUE,
        ),
        (
            "2. Сердце Гиганта",
            "+25% к макс. HP и +3 HP/сек пассивного регена за ур.",
            "Эволюция: Огненная Аура -> АДСКОЕ ПЕКЛО",
            textures.icon_chest.as_ref(),
            RED,
        ),
        (
            "3. Фолиант Времени",
            "-8% ко всем кулдаунам оружия и рывка (Dash) за ур.",
            "Эволюция: Магический Жезл -> ЛУЧ СМЕРТИ",
            textures.icon_book.as_ref(),
            VIOLET,
        ),
        (
            "4. Броня Света",
            "+3 к плоской броне и +8% к сопротивлению физ. урону.",
            "Эволюция: Святой Молот -> ПРАВЕДНЫЙ ЩИТ",
            textures.icon_armor.as_ref(),
            GOLD,
        ),
        (
            "5. Кольцо Заморозки",
            "+20% к радиусу замедления и силе замедления врагов.",
            "Эволюция: Ледяные Осколки -> ЛЕДНИКОВАЯ БУРЯ",
            textures.icon_ring.as_ref(),
            Color::new(0.4, 0.85, 1.0, 1.0),
        ),
        (
            "6. Едкий Сосуд",
            "Игнорирует 15% брони врагов и продлевает лужи яда.",
            "Эволюция: Чумные Сферы -> ТОКСИЧНЫЙ МОР",
            textures.icon_bottle.as_ref(),
            LIME,
        ),
        (
            "7. Кровавый Камень",
            "+12% к общему урону и +2.5 HP/сек регенерации за ур.",
            "Эволюция: Топор Палача -> ВИХРЕВОЙ СЕКАЧ",
            textures.icon_bottle.as_ref(),
            RED,
        ),
        (
            "8. Орлиный Прицел",
            "+20% к скорости и дальности полета снарядов за ур.",
            "Эволюция: Небесный Лук -> ЗВЕЗДОПАДНЫЙ ЗАЛП",
            textures.icon_ring.as_ref(),
            GOLD,
        ),
        (
            "9. Песочные Часы",
            "+25% к длительности заморозки, горения и яда за ур.",
            "Эволюция: Грозовая Сфера -> ГНЕВ ГРОЗЫ",
            textures.icon_hourglass.as_ref(),
            YELLOW,
        ),
        (
            "10. Сфера Притяжения",
            "+45 px к радиусу сбора кристаллов душ за ур.",
            "Эволюция: Кровавые Кинжалы -> КРОВАВАЯ ЖАТВА",
            textures.icon_jewel.as_ref(),
            SKYBLUE,
        ),
        (
            "11. Катушка Теслы",
            "+12% шанс цепной молнии на 3 цели за ур.",
            "Эволюция: Гравитационная Сфера -> ГОРИЗОНТ СОБЫТИЙ",
            textures.icon_tesla_coil.as_ref(),
            Color::new(0.3, 0.8, 1.0, 1.0),
        ),
        (
            "12. Клевер Судьбы",
            "+12% шанс критического удара (х2 урон) за ур.",
            "Эволюция: Цепной Ледяной Шар -> АБСОЛЮТНЫЙ НОЛЬ",
            textures.icon_clover.as_ref(),
            Color::new(0.3, 0.9, 0.4, 1.0),
        ),
        (
            "13. Призматический Фокус",
            "+15% к радиусу поражения всех атак (AoE) за ур.",
            "Эволюция: Кровавый Гиперлуч -> ИНФЕРНАЛЬНЫЙ КАТАКЛИЗМ",
            textures.icon_prismatic.as_ref(),
            Color::new(0.9, 0.5, 0.9, 1.0),
        ),
        (
            "14. Черный Эгис",
            "-2.5 плоского входящего урона от каждого удара за ур.",
            "Эволюция: Рой Микроракет -> ОСАДА СУДНОГО ДНЯ",
            textures.icon_aegis.as_ref(),
            GOLD,
        ),
    ];

    let col_w = (w - 14.0) / 2.0;
    let row_h = (h - 20.0) / 7.0;

    for (i, (name, effect, evo, icon, col)) in passives_data.iter().enumerate() {
        let col_idx = (i / 7) as f32;
        let row_idx = (i % 7) as f32;

        let cx = x + col_idx * (col_w + 14.0);
        let cy = y + row_idx * (row_h + 4.0);

        draw_rectangle(cx, cy, col_w, row_h, Color::new(0.11, 0.11, 0.16, 0.92));
        draw_rectangle_lines(cx, cy, col_w, row_h, 1.5, Color::new(0.3, 0.35, 0.45, 0.8));

        if let Some(tex) = icon {
            let isize = 40.0;
            draw_texture_ex(
                tex,
                cx + 10.0,
                cy + (row_h - isize) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(isize, isize)),
                    ..Default::default()
                },
            );
        }

        let tx = cx + 58.0;
        fonts.draw_heading(name, tx, cy + 18.0, 13.0, *col);
        fonts.draw_text(effect, tx, cy + 36.0, 11.0, WHITE);
        fonts.draw_text(evo, tx, cy + 54.0, 10.5, GOLD);
    }
}

/// Вкладка 4: Алтари, Экономика и Механики
fn draw_tab_mechanics(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts) {
    let half_w = (w - 16.0) / 2.0;
    let card_h = (h - 16.0) / 2.0;
    let text_w = half_w - 24.0;

    // Карточка 1: Алтарь Крови
    draw_card_box(x, y, half_w, card_h, "🏛️ АЛТАРЬ КРОВИ (ПКМ НА КАРТЕ)", fonts);
    let mut cy1 = y + 40.0;
    cy1 = fonts.draw_text_wrapped("• Постройка: Клик Правой Кнопкой Мыши (ПКМ) в любой точке арены.", x + 12.0, cy1, 11.0, 14.5, text_w, WHITE) + 3.0;
    cy1 = fonts.draw_text_wrapped("• Стоимость: 50 Очков Душ. Здоровье: 650 HP (Броня 12, уязвим к Святости).", x + 12.0, cy1, 11.0, 14.5, text_w, GOLD) + 3.0;
    cy1 = fonts.draw_text_wrapped("• Пассивный доход: Каждый живой алтарь даёт +3.0 души каждые 2 секунды.", x + 12.0, cy1, 11.0, 14.5, text_w, GREEN) + 3.0;
    cy1 = fonts.draw_text_wrapped("• Аура исцеления: Каждые 3 сек лечит союзных мобов в 180 px на +35 HP.", x + 12.0, cy1, 11.0, 14.5, text_w, RED) + 3.0;
    cy1 = fonts.draw_text_wrapped("• ТАКТИКА: Босс распознаёт алтари как стратегическую угрозу и идёт их штурмовать!", x + 12.0, cy1, 11.0, 14.5, text_w, SKYBLUE) + 3.0;
    fonts.draw_text_wrapped("⚠️ Защищайте алтари, иначе босс сломает их и получит +150 XP и Магнит!", x + 12.0, cy1, 11.0, 14.5, text_w, YELLOW);

    // Карточка 2: Экономика и Грейды Монстров
    let x2 = x + half_w + 16.0;
    draw_card_box(x2, y, half_w, card_h, "📈 ЭКОНОМИКА И ГРЕЙДЫ МОБОВ [F, G, H]", fonts);
    let mut cy2 = y + 40.0;
    cy2 = fonts.draw_text_wrapped("• Базовый доход: +8 Душ/2с. Разгон экономики: +2% от суммы каждой траты душ.", x2 + 12.0, cy2, 11.0, 14.5, text_w, WHITE) + 3.0;
    cy2 = fonts.draw_text_wrapped("• ГРЕЙД [F] Ярость Крови: +12% урон, +6% скорость всех мобов.", x2 + 12.0, cy2, 11.0, 14.5, text_w, RED) + 3.0;
    cy2 = fonts.draw_text_wrapped("• ГРЕЙД [G] Панцирь: +2.5 плоской брони, +5% физ. защиты мобов.", x2 + 12.0, cy2, 11.0, 14.5, text_w, SKYBLUE) + 3.0;
    cy2 = fonts.draw_text_wrapped("• ГРЕЙД [H] Чумные Споры: 25% шанс ядовитой лужи при гибели моба.", x2 + 12.0, cy2, 11.0, 14.5, text_w, GREEN) + 3.0;
    fonts.draw_text_wrapped("• ЭЛИТНЫЕ ЧЕМПИОНЫ: 12% шанс спавна моба с золотой короной, +80% HP, +35% урона и 2.5x дропом душ и опыта!", x2 + 12.0, cy2, 11.0, 14.5, text_w, GOLD);

    // Карточка 3: Интеллект Босса (Потенциальные поля и Dash)
    let y2 = y + card_h + 16.0;
    draw_card_box(x, y2, half_w, card_h, "🧠 ТАКТИЧЕСКИЙ ИНТЕЛЛЕКТ БОССА", fonts);
    let mut cy3 = y2 + 40.0;
    cy3 = fonts.draw_text_wrapped("• Приоритеты огня: Оружие босса целится в культистов-стрелков и Алтари!", x + 12.0, cy3, 11.0, 14.5, text_w, WHITE) + 3.0;
    cy3 = fonts.draw_text_wrapped("• Анти-снайперский натиск: При накоплении дальников босс идёт на сближение.", x + 12.0, cy3, 11.0, 14.5, text_w, SKYBLUE) + 3.0;
    cy3 = fonts.draw_text_wrapped("• Парирование атак: Вращающиеся щиты и Буря Клинков сбивают стрелы культистов!", x + 12.0, cy3, 11.0, 14.5, text_w, YELLOW) + 3.0;
    fonts.draw_text_wrapped("• Экстренный рывок (8-лучевой Context Steering Dash): Срабатывает при окружении или HP <30%. Рывок на 180 px наносит 80 урона по пути!", x + 12.0, cy3, 11.0, 14.5, text_w, GOLD);

    // Карточка 4: Анти-Пассивность (Медитация) и Сноуболл
    draw_card_box(x2, y2, half_w, card_h, "🧘 АНТИ-ПАССИВНОСТЬ И LIMIT BREAK", fonts);
    let mut cy4 = y2 + 40.0;
    cy4 = fonts.draw_text_wrapped("• МЕДИТАЦИЯ ПОКОЯ: Если босс стоит без движения > 2 сек (нет угроз): восстанавливает +2% HP/с и получает +30 XP/с. Не оставляйте арену пустой!", x2 + 12.0, cy4, 11.0, 14.5, text_w, RED) + 3.0;
    cy4 = fonts.draw_text_wrapped("• ЗАПРЕДЕЛЬНОЕ ВОЗВЫШЕНИЕ (Limit Break после 10-й мин):", x2 + 12.0, cy4, 11.0, 14.5, text_w, GOLD) + 3.0;
    fonts.draw_text_wrapped("  После эволюции всех 6 слотов босс бесконечно качает: +10% урона, -8% отката, +1 снаряд, Закалку Титана (+80 HP) и Адаптивную Броню (+2.0 брони).", x2 + 12.0, cy4, 11.0, 14.5, text_w, WHITE);
}

fn draw_card_box(x: f32, y: f32, w: f32, h: f32, title: &str, fonts: &GameFonts) {
    draw_rectangle(x, y, w, h, Color::new(0.10, 0.10, 0.15, 0.94));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(0.4, 0.35, 0.5, 0.9));
    draw_rectangle(x, y, w, 28.0, Color::new(0.15, 0.14, 0.22, 1.0));
    draw_rectangle_lines(x, y, w, 28.0, 1.0, DARKGRAY);
    fonts.draw_text(title, x + 12.0, y + 19.0, 14.0, GOLD);
}
