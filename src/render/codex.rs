use macroquad::prelude::*;
use crate::render::font::GameFonts;
use crate::render::textures::GameTextures;

pub fn draw_codex(current_tab: &mut usize, fonts: &GameFonts, textures: &GameTextures) {
    let screen_w = screen_width();
    let screen_h = screen_height();

    // Затемнение фона игры
    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.78));

    let modal_w = (screen_w - 80.0).clamp(920.0, 1180.0);
    let modal_h = (screen_h - 60.0).clamp(560.0, 660.0);
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
        1 => draw_tab_weapons(content_x, content_y, content_w, content_h, fonts, textures),
        2 => draw_tab_passives(content_x, content_y, content_w, content_h, fonts, textures),
        _ => draw_tab_mechanics(content_x, content_y, content_w, content_h, fonts),
    }
}

/// Вкладка 1: Бестиарий мобов
fn draw_tab_bestiary(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) {
    let mobs_data = [
        (
            1u8,
            "Тир 1: Скелеты-пехотинцы",
            "Цена: 15 душ  |  В отряде: 8 шт  |  Кулдаун: МГНОВЕННЫЙ  |  Дроп: 8 XP",
            "HP: 32  |  Скорость: 105  |  Урон: 7 (физ)  |  Броня: 0  |  Физ. рез: 5%  |  Святой рез: -25%",
            "Роль: Пушечное мясо. Задерживают босса контактными столкновениями, заставляя активировать ауры.",
            "Разблокировка: Доступны со старта игры (0 потрачено душ).",
            Color::new(0.9, 0.9, 0.95, 1.0),
        ),
        (
            2u8,
            "Тир 2: Чёрные гончие",
            "Цена: 35 душ  |  В отряде: 5 шт  |  Кулдаун: МГНОВЕННЫЙ  |  Дроп: 20 XP",
            "HP: 50  |  Скорость: 160 (быстрее босса!)  |  Урон: 12 (физ)  |  Броня: 1  |  Физ. рез: 10%",
            "Роль: Предиктивный перехват. Бегут на опережение вектора скорости босса, отрезая пути отхода к кристаллам.",
            "Разблокировка: Открываются после суммарной траты 150 душ.",
            Color::new(0.95, 0.45, 0.45, 1.0),
        ),
        (
            3u8,
            "Тир 3: Культисты-стрелки",
            "Цена: 70 душ  |  В отряде: 5 шт  |  Кулдаун: МГНОВЕННЫЙ  |  Дроп: 45 XP",
            "HP: 75  |  Скорость: 80  |  Урон: 16 (дальний бой)  |  Броня: 2  |  Физ. рез: 15%  |  Святой: -25%",
            "Роль: Дальнобойный кайтинг. Держат дистанцию 350-450 px, расстреливают босса вне радиуса контактных аур.",
            "Разблокировка: Открываются после суммарной траты 400 душ.",
            Color::new(0.85, 0.55, 1.0, 1.0),
        ),
        (
            4u8,
            "Тир 4: Ледяные големы",
            "Цена: 150 душ  |  В отряде: 2 шт  |  Кулдаун: МГНОВЕННЫЙ  |  Дроп: 130 XP",
            "HP: 380  |  Скорость: 52  |  Урон: 32 (холод/замедление)  |  Броня: 14  |  Лед: 100%  |  Огонь: -50%",
            "Роль: Тяжёлые танки. Иммунитет к отбрасыванию (Knockback Immune). Зажимают босса в узких проходах.",
            "Разблокировка: Открываются после суммарной траты 1000 душ.",
            Color::new(0.4, 0.85, 1.0, 1.0),
        ),
        (
            5u8,
            "Тир 5: Камикадзе (Огненные подрывники)",
            "Цена: 300 душ  |  В отряде: 3 шт  |  Кулдаун: МГНОВЕННЫЙ  |  Дроп: 160 XP",
            "HP: 130  |  Скорость: 185 (сверхбыстрые)  |  Урон: 240 AoE (взрыв при контакте, радиус 90 px)",
            "Роль: Суицидальный прорыв. Взрываются при сближении, нанося колоссальный чистый урон по площади.",
            "Разблокировка: Открываются после суммарной траты 2500 душ.",
            Color::new(1.0, 0.55, 0.2, 1.0),
        ),
    ];

    let row_h = (h - 20.0) / 5.0;

    for (i, (tier, name, cost_info, stats, role, unlock, col)) in mobs_data.iter().enumerate() {
        let ry = y + (i as f32) * row_h;

        draw_rectangle(x, ry, w, row_h - 6.0, Color::new(0.11, 0.11, 0.16, 0.92));
        draw_rectangle_lines(x, ry, w, row_h - 6.0, 1.5, Color::new(0.3, 0.3, 0.4, 0.8));

        // Иконка/спрайт моба слева
        if let Some(tex) = textures.get_mob_texture(*tier, 0) {
            let icon_s = 48.0;
            let (iw, ih) = if *tier == 2 { (icon_s * 1.4, icon_s * 0.7) } else { (icon_s, icon_s) };
            draw_texture_ex(
                tex,
                x + 12.0,
                ry + (row_h - 6.0 - ih) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(iw, ih)),
                    ..Default::default()
                },
            );
        }

        let text_x = x + 85.0;
        fonts.draw_text(name, text_x, ry + 18.0, 15.0, *col);
        let cost_w = fonts.measure_text(cost_info, 13.0).width;
        fonts.draw_text(cost_info, x + w - cost_w - 14.0, ry + 18.0, 13.0, GOLD);
        fonts.draw_text(stats, text_x, ry + 36.0, 13.0, WHITE);
        fonts.draw_text(role, text_x, ry + 54.0, 12.0, SKYBLUE);
        fonts.draw_text(unlock, text_x, ry + 72.0, 12.0, GREEN);
    }
}

/// Вкладка 2: Оружие и Эволюции Босса
fn draw_tab_weapons(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) {
    let weapons_data = [
        (
            "1. Клинок Ветра -> БУРЯ КЛИНКОВ",
            "Рецепт эволюции: Клинок Ветра (ур. 8) + Перчатка Скорости (ур. 1+)",
            "Базовый: Серповидные клинки ветра, рассекающие толпы врагов по курсу движения (физ. урон).",
            "ЭВОЛЮЦИЯ: Создает непрерывный вихрь из 8 вращающихся лезвий вокруг босса, перемалывающих орды мобов.",
            textures.icon_sword.as_ref(),
            SKYBLUE,
        ),
        (
            "2. Огненная Аура -> АДСКОЕ ПЕКЛО",
            "Рецепт эволюции: Огненная Аура (ур. 8) + Сердце Гиганта (ур. 1+)",
            "Базовый: Пульсирующая огненная волна (DoT горение), наносящая периодический урон в радиусе 110 px.",
            "ЭВОЛЮЦИЯ: Процентный урон от макс. HP + трупы горящих монстров детонируют, запуская цепную реакцию взрывов!",
            textures.icon_elixir.as_ref(),
            ORANGE,
        ),
        (
            "3. Магический Жезл -> ЛУЧ СМЕРТИ",
            "Рецепт эволюции: Магический Жезл (ур. 8) + Фолиант Времени (ур. 1+)",
            "Базовый: Самонаводящиеся сгустки тайной магии, поражающие ближайшие цели (чистый урон).",
            "ЭВОЛЮЦИЯ: Прожигающий лазерный луч через весь экран, испаряющий всё на своей траектории без поправки на броню.",
            textures.icon_wand.as_ref(),
            VIOLET,
        ),
        (
            "4. Святой Молот -> ПРАВЕДНЫЙ ЩИТ",
            "Рецепт эволюции: Святой Молот (ур. 8) + Броня Света (ур. 1+)",
            "Базовый: Тяжёлые удары молотом света по площади вокруг босса с бонусом +25% урона против нежити.",
            "ЭВОЛЮЦИЯ: Вращающийся щит света, поглощающий удары и дарующий боссу вампиризм (+12% от нанесенного урона в HP).",
            textures.icon_shield.as_ref(),
            GOLD,
        ),
        (
            "5. Ледяные Осколки -> ЛЕДНИКОВАЯ БУРЯ",
            "Рецепт эволюции: Ледяные Осколки (ур. 8) + Кольцо Заморозки (ур. 1+)",
            "Базовый: Веер ледяных стрел, пробивающих цели и замедляющих скорость движения на 40%.",
            "ЭВОЛЮЦИЯ: Периодическая морозная пурга на пол-арены, полностью замораживающая врагов на месте на 1.5 сек.",
            textures.icon_bow.as_ref(),
            Color::new(0.5, 0.9, 1.0, 1.0),
        ),
        (
            "6. Чумные Сферы -> ТОКСИЧНЫЙ МОР",
            "Рецепт эволюции: Чумные Сферы (ур. 8) + Едкий Сосуд (ур. 1+)",
            "Базовый: Ядовитые сферы, оставляющие на земле токсичные лужи, игнорирующие плоскую броню.",
            "ЭВОЛЮЦИЯ: Кислотные лужи бездны, разъедающие броню и процентные резисты монстров, пока те стоят в луже.",
            textures.icon_bottle.as_ref(),
            LIME,
        ),
    ];

    let row_h = (h - 20.0) / 6.0;

    for (i, (name, recipe, base_desc, evo_desc, icon, col)) in weapons_data.iter().enumerate() {
        let ry = y + (i as f32) * row_h;

        draw_rectangle(x, ry, w, row_h - 5.0, Color::new(0.11, 0.11, 0.16, 0.92));
        draw_rectangle_lines(x, ry, w, row_h - 5.0, 1.5, Color::new(0.35, 0.3, 0.45, 0.8));

        if let Some(tex) = icon {
            let isize = 38.0;
            draw_texture_ex(
                tex,
                x + 12.0,
                ry + (row_h - 5.0 - isize) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(isize, isize)),
                    ..Default::default()
                },
            );
        }

        let tx = x + 65.0;
        fonts.draw_text(name, tx, ry + 16.0, 15.0, *col);
        let rw = fonts.measure_text(recipe, 12.0).width;
        fonts.draw_text(recipe, x + w - rw - 14.0, ry + 16.0, 12.0, GOLD);
        fonts.draw_text(base_desc, tx, ry + 32.0, 12.0, LIGHTGRAY);
        fonts.draw_text(evo_desc, tx, ry + 48.0, 12.0, YELLOW);
    }
}

/// Вкладка 3: Пассивные предметы Босса
fn draw_tab_passives(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts, textures: &GameTextures) {
    let passives_data = [
        (
            "1. Перчатка Скорости (Speed Gloves)",
            "Эффект: -10% к базовому времени перезарядки всех атак за ур. (до -50% на ур. 5).",
            "Связанная эволюция: Клинок Ветра -> БУРЯ КЛИНКОВ",
            textures.icon_glove.as_ref(),
            SKYBLUE,
        ),
        (
            "2. Сердце Гиганта (Giant Heart)",
            "Эффект: +25% к макс. запасу здоровья и +3 HP/сек пассивной регенерации за ур.",
            "Связанная эволюция: Огненная Аура -> АДСКОЕ ПЕКЛО",
            textures.icon_chest.as_ref(),
            RED,
        ),
        (
            "3. Фолиант Времени (Cooldown Tome)",
            "Эффект: -8% ко всем кулдаунам активных умений и рывка (Dash) за ур.",
            "Связанная эволюция: Магический Жезл -> ЛУЧ СМЕРТИ",
            textures.icon_book.as_ref(),
            VIOLET,
        ),
        (
            "4. Броня Света (Light Armor)",
            "Эффект: +3 к плоской броне и +8% к сопротивлению физическому урону за ур.",
            "Связанная эволюция: Святой Молот -> ПРАВЕДНЫЙ ЩИТ",
            textures.icon_armor.as_ref(),
            GOLD,
        ),
        (
            "5. Кольцо Заморозки (Frost Ring)",
            "Эффект: Увеличивает радиус замедления на +20% и силу замедления врагов за ур.",
            "Связанная эволюция: Ледяные Осколки -> ЛЕДНИКОВАЯ БУРЯ",
            textures.icon_ring.as_ref(),
            Color::new(0.4, 0.85, 1.0, 1.0),
        ),
        (
            "6. Едкий Сосуд (Corrosive Flask)",
            "Эффект: Игнорирует 15% брони атакующих врагов и продлевает время жизни кислотных луж.",
            "Связанная эволюция: Чумные Сферы -> ТОКСИЧНЫЙ МОР",
            textures.icon_bottle.as_ref(),
            LIME,
        ),
    ];

    let row_h = (h - 20.0) / 6.0;

    for (i, (name, effect, evo, icon, col)) in passives_data.iter().enumerate() {
        let ry = y + (i as f32) * row_h;

        draw_rectangle(x, ry, w, row_h - 6.0, Color::new(0.11, 0.11, 0.16, 0.92));
        draw_rectangle_lines(x, ry, w, row_h - 6.0, 1.5, Color::new(0.3, 0.35, 0.45, 0.8));

        if let Some(tex) = icon {
            let isize = 42.0;
            draw_texture_ex(
                tex,
                x + 15.0,
                ry + (row_h - 6.0 - isize) / 2.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(isize, isize)),
                    ..Default::default()
                },
            );
        }

        let tx = x + 75.0;
        fonts.draw_text(name, tx, ry + 18.0, 16.0, *col);
        fonts.draw_text(effect, tx, ry + 36.0, 13.0, WHITE);
        fonts.draw_text(evo, tx, ry + 53.0, 13.0, GOLD);
    }
}

/// Вкладка 4: Алтари, Экономика и Механики
fn draw_tab_mechanics(x: f32, y: f32, w: f32, h: f32, fonts: &GameFonts) {
    let half_w = (w - 16.0) / 2.0;
    let card_h = (h - 16.0) / 2.0;

    // Карточка 1: Алтарь Крови
    draw_card_box(x, y, half_w, card_h, "🏛️ АЛТАРЬ КРОВИ (ПКМ НА КАРТЕ)", fonts);
    let l1 = "• Постройка: Клик Правой Кнопкой Мыши (ПКМ) в любой точке арены.";
    let l2 = "• Стоимость: 50 Очков Душ. Здоровье: 650 HP (Броня 12, уязвим к Святости).";
    let l3 = "• Пассивный доход: Каждый живой алтарь даёт +3 души каждые 2 секунды.";
    let l4 = "• Аура исцеления: Каждые 3 сек лечит союзных мобов в 180 px на +35 HP.";
    let l5 = "• ТАКТИКА: Босс распознаёт алтари как угрозу и идёт их штурмовать!";
    let l6 = "  Защищайте алтари ордами, иначе босс сломает их и получит +150 XP и Магнит!";
    fonts.draw_text(l1, x + 16.0, y + 42.0, 13.0, WHITE);
    fonts.draw_text(l2, x + 16.0, y + 60.0, 13.0, GOLD);
    fonts.draw_text(l3, x + 16.0, y + 78.0, 13.0, GREEN);
    fonts.draw_text(l4, x + 16.0, y + 96.0, 13.0, RED);
    fonts.draw_text(l5, x + 16.0, y + 114.0, 13.0, SKYBLUE);
    fonts.draw_text(l6, x + 16.0, y + 130.0, 13.0, YELLOW);

    // Карточка 2: Экономика Режиссёра
    let x2 = x + half_w + 16.0;
    draw_card_box(x2, y, half_w, card_h, "📈 ЭКОНОМИКА «SPEND TO GROW»", fonts);
    let e1 = "• Базовый доход: +8 Очков Душ каждые 2.0 секунды.";
    let e2 = "• Формула разгона: Прирост дохода = Потраченная сумма × 0.02.";
    let e3 = "  (Потратили 100 душ -> постоянный доход вырос на +2.0 каждые 2 сек).";
    let e4 = "• МГНОВЕННЫЙ ПРИЗЫВ: Кулдауны с кнопок 1-5 сняты!";
    let e5 = "  Есть очки душ — призывайте орды непрерывно без задержек.";
    let e6 = "• Баланс: Не спамьте слабых мобов бездумно, иначе босс наберёт опыт!";
    fonts.draw_text(e1, x2 + 16.0, y + 42.0, 13.0, WHITE);
    fonts.draw_text(e2, x2 + 16.0, y + 60.0, 13.0, GOLD);
    fonts.draw_text(e3, x2 + 16.0, y + 78.0, 13.0, LIGHTGRAY);
    fonts.draw_text(e4, x2 + 16.0, y + 96.0, 13.0, GREEN);
    fonts.draw_text(e5, x2 + 16.0, y + 114.0, 13.0, WHITE);
    fonts.draw_text(e6, x2 + 16.0, y + 130.0, 13.0, ORANGE);

    // Карточка 3: Интеллект Босса (Потенциальные поля и Dash)
    let y2 = y + card_h + 16.0;
    draw_card_box(x, y2, half_w, card_h, "🧠 ТАКТИЧЕСКИЙ ИНТЕЛЛЕКТ БОССА", fonts);
    let a1 = "• Приоритеты огня: Оружие босса целится в культистов-стрелков и Алтари!";
    let a2 = "• Анти-снайперский натиск: При накоплении дальников босс идёт на сближение.";
    let a3 = "• Парирование атак: Щиты и Буря Клинков сбивают стрелы культистов!";
    let a4 = "• Экстренный рывок (8-лучевой Context Steering Dash):";
    let a5 = "  Срабатывает при окружении, низком HP (<30%) или ураганном огне стрелков.";
    let a6 = "  Рывок на 180 px распыляет вражеские стрелы и наносит 80 урона по пути!";
    fonts.draw_text(a1, x + 16.0, y2 + 42.0, 13.0, WHITE);
    fonts.draw_text(a2, x + 16.0, y2 + 60.0, 13.0, SKYBLUE);
    fonts.draw_text(a3, x + 16.0, y2 + 78.0, 13.0, YELLOW);
    fonts.draw_text(a4, x + 16.0, y2 + 96.0, 13.0, LIGHTGRAY);
    fonts.draw_text(a5, x + 16.0, y2 + 114.0, 13.0, WHITE);
    fonts.draw_text(a6, x + 16.0, y2 + 130.0, 13.0, GOLD);

    // Карточка 4: Анти-Пассивность (Медитация) и Сноуболл
    draw_card_box(x2, y2, half_w, card_h, "🧘 АНТИ-ПАССИВНОСТЬ И СНОУБОЛЛ БОССА", fonts);
    let m1 = "• МЕДИТАЦИЯ ПОКОЯ: Если босс стоит без движения > 2 сек (нет угроз):";
    let m2 = "  - Восстанавливает +2% от макс. HP в секунду!";
    let m3 = "  - Получает +30 XP в секунду!";
    let m4 = "  ВНИМАНИЕ: Не оставляйте арену пустой, иначе босс вылечится!";
    let m5 = "• Сноуболл уровня: Каждый уровень даёт боссу +40 Макс. HP,";
    let m6 = "  +0.5 плоской брони и апгрейд оружия/пассивки вплоть до Эволюции.";
    fonts.draw_text(m1, x2 + 16.0, y2 + 42.0, 13.0, RED);
    fonts.draw_text(m2, x2 + 16.0, y2 + 60.0, 13.0, GREEN);
    fonts.draw_text(m3, x2 + 16.0, y2 + 78.0, 13.0, SKYBLUE);
    fonts.draw_text(m4, x2 + 16.0, y2 + 96.0, 13.0, YELLOW);
    fonts.draw_text(m5, x2 + 16.0, y2 + 114.0, 13.0, WHITE);
    fonts.draw_text(m6, x2 + 16.0, y2 + 130.0, 13.0, GOLD);
}

fn draw_card_box(x: f32, y: f32, w: f32, h: f32, title: &str, fonts: &GameFonts) {
    draw_rectangle(x, y, w, h, Color::new(0.10, 0.10, 0.15, 0.94));
    draw_rectangle_lines(x, y, w, h, 1.5, Color::new(0.4, 0.35, 0.5, 0.9));
    draw_rectangle(x, y, w, 28.0, Color::new(0.15, 0.14, 0.22, 1.0));
    draw_rectangle_lines(x, y, w, 28.0, 1.0, DARKGRAY);
    fonts.draw_text(title, x + 12.0, y + 19.0, 14.0, GOLD);
}
