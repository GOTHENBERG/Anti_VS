use macroquad::prelude::*;
use crate::content::mobs::get_squad_config;
use crate::ecs::world::World;
use crate::render::font::GameFonts;
use crate::render::textures::GameTextures;

/// Результат взаимодействия с досье
pub struct DossierResult {
    pub close_requested: bool,
    pub summon_requested: bool,
}

/// Отрисовывает детальное досье моба (интерактивную энциклопедию-википедию)
pub fn draw_mob_dossier(
    tier: u8,
    world: &World,
    fonts: &GameFonts,
    textures: &GameTextures,
) -> DossierResult {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let (mx, my) = mouse_position();
    let time = get_time() as f32;

    // Затемнение игрового поля
    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.82));

    let modal_w = (screen_w - 100.0).clamp(940.0, 1160.0);
    let modal_h = (screen_h - 70.0).clamp(560.0, 660.0);
    let x = (screen_w - modal_w) / 2.0;
    let y = (screen_h - modal_h) / 2.0;

    // Окно досье
    draw_rectangle(x, y, modal_w, modal_h, Color::new(0.07, 0.07, 0.11, 0.98));
    draw_rectangle_lines(x, y, modal_w, modal_h, 2.5, Color::new(0.85, 0.65, 0.20, 1.0));

    // Верхний заголовок
    draw_rectangle(x, y, modal_w, 44.0, Color::new(0.12, 0.12, 0.18, 1.0));
    draw_rectangle_lines(x, y, modal_w, 44.0, 1.5, DARKGRAY);

    let config = get_squad_config(tier);
    let title = format!("📖 ТАКТИЧЕСКОЕ ДОСЬЕ: {} [ТИР {}]", config.name.to_uppercase(), tier);
    fonts.draw_heading(&title, x + 24.0, y + 29.0, 16.0, GOLD);

    // Кнопка закрытия [ X ]
    let close_btn_w = 40.0;
    let close_btn_h = 32.0;
    let close_btn_x = x + modal_w - close_btn_w - 10.0;
    let close_btn_y = y + 6.0;
    let is_close_hover = mx >= close_btn_x && mx <= close_btn_x + close_btn_w && my >= close_btn_y && my <= close_btn_y + close_btn_h;
    draw_rectangle(close_btn_x, close_btn_y, close_btn_w, close_btn_h, if is_close_hover { RED } else { Color::new(0.2, 0.1, 0.1, 0.9) });
    draw_rectangle_lines(close_btn_x, close_btn_y, close_btn_w, close_btn_h, 1.5, WHITE);
    let cw = fonts.measure_heading("X", 18.0).width;
    fonts.draw_heading("X", close_btn_x + (close_btn_w - cw) / 2.0, close_btn_y + 23.0, 18.0, WHITE);

    let close_requested = is_key_pressed(KeyCode::Escape) || (is_mouse_button_pressed(MouseButton::Left) && is_close_hover);
    let mut summon_requested = false;

    // Сетка: Левая колонка (Портрет + Призыв) и Правая колонка (Вики-статы + Лор)
    let left_w = 320.0;
    let left_x = x + 24.0;
    let right_x = left_x + left_w + 24.0;
    let right_w = modal_w - left_w - 72.0;
    let content_y = y + 60.0;

    // 1. Левая колонка: Окно 3D/HD портрета существа
    let portrait_h = 240.0;
    draw_rectangle(left_x, content_y, left_w, portrait_h, Color::new(0.04, 0.04, 0.07, 0.95));
    draw_rectangle_lines(left_x, content_y, left_w, portrait_h, 2.0, Color::new(0.4, 0.25, 0.5, 0.8));

    // Магический круг призыва под ногами спрайта
    let center_px = left_x + left_w / 2.0;
    let center_py = content_y + portrait_h / 2.0 + 35.0;
    let ring_pulse = (time * 3.0).sin() * 4.0;
    draw_ellipse(center_px, center_py, 65.0 + ring_pulse, 24.0 + ring_pulse * 0.35, 0.0, Color::new(0.5, 0.1, 0.6, 0.35));
    draw_ellipse_lines(center_px, center_py, 65.0 + ring_pulse, 24.0 + ring_pulse * 0.35, 0.0, 1.5, PURPLE);

    // Отрисовка спрайта моба
    if let Some(mob_tex) = textures.get_mob_texture(tier, 0) {
        let mob_render_size = if tier == 8 { 130.0 } else if tier == 4 || tier == 7 { 110.0 } else { 90.0 };
        let (pw, ph) = if tier == 2 { (mob_render_size * 1.3, mob_render_size * 0.65) } else { (mob_render_size, mob_render_size) };
        let float_y = (time * 4.0).sin() * 5.0;
        draw_texture_ex(
            mob_tex,
            center_px - pw / 2.0,
            center_py - ph - 5.0 + float_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(pw, ph)),
                ..Default::default()
            },
        );
    }

    // Подпись класса моба
    let class_str = match tier {
        1 => "Класс: Легкая пехота (Мясной щит)",
        2 => "Класс: Быстрая кавалерия (Анти-медитация)",
        3 => "Класс: Магический стрелок (Дальний бой)",
        4 => "Класс: Осадный голем (Тяжелый танк)",
        5 => "Класс: Осадный камикадзе (Сплэш-урон)",
        6 => "Класс: Теневой ассасин (Телепортация)",
        7 => "Класс: Инфернальный артиллерист (Огонь)",
        _ => "Класс: Инфернальный Титан (Мини-босс)",
    };
    fonts.draw_heading(class_str, left_x + 8.0, content_y + portrait_h - 14.0, 13.0, SKYBLUE);

    // Блок стоимости и параметров призыва
    let summon_box_y = content_y + portrait_h + 16.0;
    let summon_box_h = modal_h - (summon_box_y - y) - 24.0;
    draw_rectangle(left_x, summon_box_y, left_w, summon_box_h, Color::new(0.10, 0.10, 0.15, 0.95));
    draw_rectangle_lines(left_x, summon_box_y, left_w, summon_box_h, 1.5, GRAY);

    let director = world.directors.first();
    let is_unlocked = director.map_or(false, |d| d.total_points_spent >= config.unlock_spent_threshold);
    let can_afford = director.map_or(false, |d| d.points >= config.cost);

    fonts.draw_heading("ПАРАМЕТРЫ ПРИЗЫВА", left_x + 14.0, summon_box_y + 24.0, 14.0, GOLD);
    let cost_str = format!("• Стоимость: {:.0}", config.cost);
    fonts.draw_text(&cost_str, left_x + 14.0, summon_box_y + 48.0, 13.0, if can_afford { GREEN } else { RED });
    let cw = fonts.measure_text(&cost_str, 13.0).width;
    if let Some(sm_tex) = textures.soul_ethereal_sm.as_ref() {
        draw_texture_ex(sm_tex, left_x + 16.0 + cw, summon_box_y + 37.0, WHITE, DrawTextureParams { dest_size: Some(Vec2::new(14.0, 14.0)), ..Default::default() });
    }

    let count_str = format!("• Размер отряда: {} существ", config.count);
    fonts.draw_text(&count_str, left_x + 14.0, summon_box_y + 68.0, 12.5, WHITE);

    let cd_str = "• Время призыва: Мгновенно (Кулдаун снят)";
    fonts.draw_text(cd_str, left_x + 14.0, summon_box_y + 88.0, 12.0, YELLOW);

    let inc_str = format!("• Прирост дохода: +{:.2}", config.cost * 0.02);
    fonts.draw_text(&inc_str, left_x + 14.0, summon_box_y + 108.0, 12.5, SKYBLUE);
    let iw = fonts.measure_text(&inc_str, 12.5).width;
    if let Some(sm_tex) = textures.soul_ethereal_sm.as_ref() {
        draw_texture_ex(sm_tex, left_x + 16.0 + iw, summon_box_y + 97.0, WHITE, DrawTextureParams { dest_size: Some(Vec2::new(14.0, 14.0)), ..Default::default() });
    }
    fonts.draw_text("/ 2 сек", left_x + 34.0 + iw, summon_box_y + 108.0, 11.5, SKYBLUE);

    let total_spawned = director.map_or(0, |d| d.total_mobs_spawned);
    let stats_match = format!("• Всего на поле боя: {} (Призвано: {})", world.mobs.len(), total_spawned);
    fonts.draw_text(&stats_match, left_x + 14.0, summon_box_y + 128.0, 11.5, LIGHTGRAY);

    // Большая кнопка призыва
    let s_btn_w = left_w - 28.0;
    let s_btn_h = 44.0;
    let s_btn_x = left_x + 14.0;
    let s_btn_y = summon_box_y + summon_box_h - s_btn_h - 14.0;

    let s_hover = mx >= s_btn_x && mx <= s_btn_x + s_btn_w && my >= s_btn_y && my <= s_btn_y + s_btn_h;
    let s_bg = if !is_unlocked {
        Color::new(0.2, 0.1, 0.1, 0.8)
    } else if can_afford {
        if s_hover { Color::new(0.22, 0.55, 0.28, 1.0) } else { Color::new(0.14, 0.40, 0.20, 1.0) }
    } else {
        Color::new(0.25, 0.20, 0.20, 0.9)
    };
    let s_border = if is_unlocked && can_afford { GREEN } else { GRAY };

    draw_rectangle(s_btn_x, s_btn_y, s_btn_w, s_btn_h, s_bg);
    draw_rectangle_lines(s_btn_x, s_btn_y, s_btn_w, s_btn_h, 2.0, s_border);

    let btn_label = if !is_unlocked {
        format!("🔒 ТРЕБУЕТ {:.0} ДУШ ТРАТ", config.unlock_spent_threshold)
    } else if can_afford {
        format!("⚔️ ПРИЗВАТЬ ОТРЯД [{}]", tier)
    } else {
        format!("НЕДОСТАТОЧНО ДУШ ({:.0})", config.cost)
    };
    let bw = fonts.measure_heading(&btn_label, 14.0).width;
    fonts.draw_heading(&btn_label, s_btn_x + (s_btn_w - bw) / 2.0, s_btn_y + 28.0, 14.0, WHITE);

    if is_mouse_button_pressed(MouseButton::Left) && s_hover && is_unlocked && can_afford {
        summon_requested = true;
    }

    // 2. Правая колонка: Энциклопедическая статья (Википедия моба)
    // 2.1 Таблица характеристик и Сопротивлений
    let stats_box_h = 170.0;
    draw_rectangle(right_x, content_y, right_w, stats_box_h, Color::new(0.09, 0.09, 0.13, 0.95));
    draw_rectangle_lines(right_x, content_y, right_w, stats_box_h, 1.5, Color::new(0.35, 0.4, 0.5, 0.8));

    fonts.draw_heading("📊 БАЗОВЫЕ БОЕВЫЕ ХАРАКТЕРИСТИКИ И РЕЗИСТЫ", right_x + 16.0, content_y + 22.0, 14.0, GOLD);

    let (hp_val, spd_val, spd_note, dmg_val, dmg_t_name, atk_spd_str, atk_range_str, arm_val, kb_res_str, xp_drop) = match tier {
        1 => (32.0, 105.0, "(Босс: 115)", 7.0, "Физ. контакт", "1.0 уд/сек (7 DPS)", "24 px (Ближний бой)", 0.0, "0%", 8),
        2 => (50.0, 160.0, "(⚡ +39% быстрее босса!)", 12.0, "Физ. наскок", "1.0 уд/сек (12 DPS)", "26 px (Ближний бой)", 1.0, "15%", 20),
        3 => (75.0, 80.0, "(Кайтинг)", 16.0, "Темная магия", "1 выстр / 2.0 сек", "320 px (Дальний бой)", 2.0, "0%", 45),
        4 => (380.0, 52.0, "(Монолит)", 32.0, "Физ + Замедление", "1.0 уд/сек (32 DPS)", "36 px (Ближний бой)", 14.0, "100% (ИММУН)", 130),
        5 => (130.0, 185.0, "(⚡ Сверхзвук)", 220.0, "Чистый сплэш", "Мгновенно при контакте", "95 px (Радиус взрыва)", 2.0, "0%", 160),
        6 => (550.0, 140.0, "(Стремительный)", 42.0, "Физ / Тьма", "1.0 уд/с + Скачок 4.5с", "30 px (Телепорт >320px)", 8.0, "30%", 240),
        7 => (850.0, 95.0, "(Артиллерия)", 55.0, "Огненный шар", "1 выстр / 2.2 сек", "360 px (Дальний бой)", 10.0, "20%", 420),
        _ => (2800.0, 68.0, "(Титан орды)", 95.0, "Чистый урон (True)", "1.0 уд/с + Волна 2.8с", "50 px (Волна 420 px)", 24.0, "100% (ИММУН)", 1200),
    };

    let col1_x = right_x + 16.0;
    let col2_x = right_x + right_w * 0.50;

    // Левая колонка боевых статов
    let h_t = format!("• Здоровье: {:.0} HP", hp_val);
    let s_t = format!("• Скорость бега: {:.0} px/сек {}", spd_val, spd_note);
    let d_t = format!("• Урон: {:.0} ({})", dmg_val, dmg_t_name);
    let as_t = format!("• Скорость атаки: {}", atk_spd_str);
    let ar_t = format!("• Дальность атаки: {}", atk_range_str);
    let a_t = format!("• Плоская броня: {:.0} ед. | Отброс: {}", arm_val, kb_res_str);

    fonts.draw_text(&h_t, col1_x, content_y + 42.0, 12.0, WHITE);
    fonts.draw_text(&s_t, col1_x, content_y + 61.0, 12.0, SKYBLUE);
    fonts.draw_text(&d_t, col1_x, content_y + 80.0, 12.0, GOLD);
    fonts.draw_text(&as_t, col1_x, content_y + 99.0, 12.0, YELLOW);
    fonts.draw_text(&ar_t, col1_x, content_y + 118.0, 12.0, WHITE);
    fonts.draw_text(&a_t, col1_x, content_y + 137.0, 12.0, LIGHTGRAY);

    let (res_phys, res_fire, res_ice, res_holy, res_poison) = match tier {
        1 => ("+5%", "0%", "0%", "-25% (Нежить)", "0%"),
        2 => ("+10%", "0%", "0%", "-20%", "0%"),
        3 => ("+15%", "+20%", "-10%", "-40% (Слабость)", "0%"),
        4 => ("+35%", "-50% (Уязвим)", "100% (ИММУН)", "0%", "+50%"),
        5 => ("+10%", "+50%", "0%", "0%", "0%"),
        6 => ("+25%", "+20%", "+20%", "-10%", "0%"),
        7 => ("+20%", "100% (ИММУН)", "-40% (Уязвим)", "0%", "+50%"),
        _ => ("+45%", "+30%", "+30%", "-20%", "0%"),
    };

    // Правая колонка резистов
    fonts.draw_heading("ТАБЛИЦА РЕЗИСТОВ К УРОНУ:", col2_x, content_y + 42.0, 12.5, YELLOW);
    let rf = format!("• Физ. урон: {} | Огонь: {}", res_phys, res_fire);
    let rh = format!("• Лед/Холод: {} | Святой: {}", res_ice, res_holy);
    let rp = format!("• Яд/Кислота: {} | Чистый: 0%", res_poison);
    let xp_str = format!("• Награда Боссу: {} XP сфер", xp_drop);
    let champ_str = "• Шанс Чемпиона: 12% (+80% HP, +35% DMG)";

    fonts.draw_text(&rf, col2_x, content_y + 61.0, 11.5, LIGHTGRAY);
    fonts.draw_text(&rh, col2_x, content_y + 80.0, 11.5, LIGHTGRAY);
    fonts.draw_text(&rp, col2_x, content_y + 99.0, 11.5, LIGHTGRAY);
    fonts.draw_text(&xp_str, col2_x, content_y + 118.0, 11.5, ORANGE);
    fonts.draw_text(champ_str, col2_x, content_y + 137.0, 11.5, GOLD);

    // 2.2 Тактическое руководство и Лор (Wikipedia Style Lore & Strategy)
    let lore_box_y = content_y + stats_box_h + 12.0;
    let lore_box_h = modal_h - (lore_box_y - y) - 20.0;
    draw_rectangle(right_x, lore_box_y, right_w, lore_box_h, Color::new(0.08, 0.08, 0.12, 0.95));
    draw_rectangle_lines(right_x, lore_box_y, right_w, lore_box_h, 1.5, Color::new(0.4, 0.35, 0.5, 0.8));

    fonts.draw_heading("📜 ТАКТИЧЕСКОЕ РУКОВОДСТВО И СИНЕРГИИ", right_x + 16.0, lore_box_y + 24.0, 14.0, GOLD);

    let (strat1, strat2, strat3, lore_l1, lore_l2) = match tier {
        1 => (
            "• ТАКТИКА: Идеальное пушечное мясо. Дешевые отряды сбивают наводку снарядов босса.",
            "• СИНЕРГИЯ [H]: Чумные Споры создают ядовитые лужи при их быстрой гибели.",
            "• КОНТРА: Слабы против Клинка Ветра и Огненной Ауры — погибают мгновенно.",
            "ЛОР: Бесчисленные скелеты, поднятые из праха падших армий нечестивым ритуалом.",
            "Они не ведают страха и усталости, подчиняясь лишь мысленной воле Режиссера Зла."
        ),
        2 => (
            "• ТАКТИКА: Сверхвысокая скорость (160 px/с). Заставляют босса непрерывно кайтить.",
            "• АНТИ-МЕДИТАЦИЯ: Ни за что не дадут боссу стоять 1.2 сек на месте для регенерации HP!",
            "• СИНЕРГИЯ [F]: Ярость Крови разгоняет их скорость до невообразимых 180+ px/с.",
            "ЛОР: Изголодавшиеся адские псы, чующие живую кровь за десятки лиг вокруг.",
            "Они всегда атакуют стаей, сжимая кольцо окружения и перекрывая пути отхода."
        ),
        3 => (
            "• ТАКТИКА: Дальнобойный обстрел (320 px). Вынуждают босса тратить Рывок (Dash).",
            "• ФОКУС БОССА: Босс запрограммирован считать стрелков высокоприоритетной целью!",
            "• РАЗМЕЩЕНИЕ: Призывайте вокруг алтаря, чтобы аура исцеления поддерживала их жизнь.",
            "ЛОР: Фанатики Инфернального культа. Плетут стрелы концентрированной скверны,",
            "которые прожигают освященную броню и наносят чистый магический урон."
        ),
        4 => (
            "• ТАКТИКА: Живая крепость с 380 HP и 14 брони. Неуязвимы к Ледяным Осколкам (100% иммунитет).",
            "• ПОГЛОЩЕНИЕ: Заслоняют собой стрелков и алтари от прямого урона Магического Жезла.",
            "• СИНЕРГИЯ [G]: Панцирь дает им дополнительную броню, превращая в непробиваемые монолиты.",
            "ЛОР: Колоссы из вечного льда, оскверненные древними рунами погибели.",
            "Их гранитные тела поглощают кинетическую мощь атак, прикрывая хрупких магов."
        ),
        5 => (
            "• ТАКТИКА: Стремительный рывок на 185 px/с и сокрушительный взрыв на 220 урона!",
            "• ИГНОР БРОНИ: Наносят чистый взрывной урон по площади (65 px), моментально сбривая HP босса.",
            "• ПАНИКА ИИ: Заставляют нейросеть босса экстренно спасаться бегством.",
            "ЛОР: Одержимые фанатики с бочками адского пороха за плечами.",
            "В их безумных глазах отражается лишь пламенная жажда самопожертвования во славу Бездны."
        ),
        6 => (
            "• ТАКТИКА: При получении урона телепортируются прямо за спину цели!",
            "• СРЫВ КАЙТИНГА: Лишают босса возможности безнаказанно расстреливать врагов на отходе.",
            "• СИНЕРГИЯ [F+G]: Под Яростью и Панцирем становятся смертоносными дуэлянтами.",
            "ЛОР: Элитные порождения мрака, соскальзывающие сквозь складки пространства.",
            "Их клинки смазаны ядом преисподней, лишающим жертву возможности сопротивляться."
        ),
        7 => (
            "• ТАКТИКА: Залповый огонь разрывными огненными шарами на 350 px. 100% иммунитет к огню!",
            "• АРТИЛЛЕРИЯ: Пока голем или скелеты отвлекают босса, элементали наносят чудовищный урон.",
            "• ПОДЖОГ: Оставляют горящие следы, наносящие периодический огненный урон.",
            "ЛОР: Воплощенная ярость первородного инфернального огня.",
            "Их присутствие раскаляет воздух добела, превращая доспехи паладинов в расплавленный металл."
        ),
        _ => (
            "• ТАКТИКА: Великий Жнец Смерти. 2800 HP, 24 брони и рассекающая круговая коса!",
            "• ЧИСТЫЙ УРОН: Удары косы наносят 95 чистого урона, полностью игнорируя броню босса.",
            "• ФИНАЛЬНЫЙ ШТУРМ: Выпускайте под прикрытием стрелков и алтарей для победного сжатия кольца.",
            "ЛОР: Один из Четырех Всадников Апокалипсиса, восстающий в решающий час.",
            "Его взмах срезает саму ткань жизни, не оставляя героям ни единого шанса на спасение."
        ),
    };

    let text_max_w = right_w - 32.0;
    let mut cur_y = lore_box_y + 46.0;
    cur_y = fonts.draw_text_wrapped(strat1, right_x + 16.0, cur_y, 11.5, 16.0, text_max_w, WHITE) + 4.0;
    cur_y = fonts.draw_text_wrapped(strat2, right_x + 16.0, cur_y, 11.5, 16.0, text_max_w, SKYBLUE) + 4.0;
    cur_y = fonts.draw_text_wrapped(strat3, right_x + 16.0, cur_y, 11.5, 16.0, text_max_w, YELLOW) + 4.0;

    draw_line(right_x + 16.0, cur_y + 4.0, right_x + right_w - 16.0, cur_y + 4.0, 1.0, DARKGRAY);
    cur_y += 20.0;

    fonts.draw_heading("📖 ЭНЦИКЛОПЕДИЧЕСКАЯ СПРАВКА (WIKI LORE):", right_x + 16.0, cur_y, 12.0, GOLD);
    cur_y += 18.0;

    let full_lore = format!("{} {}", lore_l1, lore_l2);
    fonts.draw_text_wrapped(&full_lore, right_x + 16.0, cur_y, 11.5, 16.0, text_max_w, LIGHTGRAY);

    let tip_text = "💡 Совет: Нажмите клавишу [Esc] или кликните [X], чтобы вернуться в бой";
    fonts.draw_text(tip_text, right_x + 16.0, lore_box_y + lore_box_h - 14.0, 11.0, GRAY);

    DossierResult {
        close_requested,
        summon_requested,
    }
}
