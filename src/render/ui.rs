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

enum HoverTooltip {
    Weapon(WeaponSlot, f32, f32),
    Passive(PassiveSlot, f32, f32),
    Squad(u8, f32, f32),
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
) {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let (mx, my) = mouse_position();
    let mut hovered_tooltip: Option<HoverTooltip> = None;

    // 1. Верхний таймер 30:00 обратного отсчета
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

            // Проверка наведения курсора на оружие
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

            // Проверка наведения курсора на пассивку
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

    // 5. Панель Директора (слева вверху)
    if let Some(director) = world.directors.first() {
        draw_rectangle(15.0, 15.0, 290.0, 155.0, Color::new(0.08, 0.08, 0.12, 0.92));
        draw_rectangle_lines(15.0, 15.0, 290.0, 155.0, 2.0, DARKGRAY);

        let altar_count = world.structures.len();
        let total_income = director.income_rate + (altar_count as f32 * 3.0);

        let points_text = format!("Очки Душ: {:.0}", director.points);
        let income_text = format!("Доход: +{:.1} / 2 сек (Алтарей: {})", total_income, altar_count);
        let spent_text = format!("Потрачено: {:.0}", director.total_points_spent);
        let spawn_text = format!("Призвано монстров: {}", director.total_mobs_spawned);
        let stats_text = format!("FPS: {} | Мобы на поле: {}", get_fps(), world.mobs.len());
        let cam_info = "WASD: Камера | Колесо: Зум | [C]: Босс | [I]: Кодекс";

        fonts.draw_heading(&points_text, 25.0, 40.0, 22.0, GOLD);
        fonts.draw_text(&income_text, 25.0, 64.0, 15.0, GREEN);
        fonts.draw_text(&spent_text, 25.0, 86.0, 14.0, LIGHTGRAY);
        fonts.draw_text(&spawn_text, 25.0, 106.0, 14.0, SKYBLUE);
        fonts.draw_text(&stats_text, 25.0, 126.0, 13.0, WHITE);
        fonts.draw_text(cam_info, 25.0, 148.0, 12.0, YELLOW);

        // 6. Виджет перезарядки и статуса Алтаря Крови (с иконкой и прогресс-баром)
        let aw_x = 15.0;
        let aw_y = 178.0;
        let aw_w = 290.0;
        let aw_h = 76.0;

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

        // Иконка Алтаря слева
        if let Some(altar_tex) = textures.altar.as_ref() {
            let icon_w = 34.0;
            let icon_h = 54.0;
            let icon_tint = if on_cooldown { Color::new(0.8, 0.4, 0.4, 0.8) } else { WHITE };
            draw_texture_ex(
                altar_tex,
                aw_x + 10.0,
                aw_y + (aw_h - icon_h) / 2.0,
                icon_tint,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(icon_w, icon_h)),
                    ..Default::default()
                },
            );
        } else {
            draw_circle(aw_x + 28.0, aw_y + 38.0, 16.0, PURPLE);
        }

        let content_x = aw_x + 52.0;

        // Заголовок виджета
        fonts.draw_heading("АЛТАРЬ КРОВИ [ПКМ]", content_x, aw_y + 20.0, 14.0, GOLD);

        if on_cooldown {
            let cd_text = format!("Перезарядка: {:.1}с / {:.0}с", director.altar_cooldown, director.altar_max_cooldown);
            fonts.draw_text(&cd_text, content_x, aw_y + 38.0, 12.0, Color::new(1.0, 0.35, 0.35, 1.0));

            // Полоска отката кулдауна
            let bar_w = 220.0;
            let bar_h = 7.0;
            let prog = (1.0 - director.altar_cooldown / director.altar_max_cooldown).clamp(0.0, 1.0);
            draw_rectangle(content_x, aw_y + 46.0, bar_w, bar_h, Color::new(0.2, 0.1, 0.1, 0.9));
            draw_rectangle(content_x, aw_y + 46.0, bar_w * prog, bar_h, ORANGE);
            draw_rectangle_lines(content_x, aw_y + 46.0, bar_w, bar_h, 1.0, GRAY);

            let count_text = format!("Активно: {} алт. (+{:.0}/2с доход)", world.structures.len(), world.structures.len() as f32 * 3.0);
            fonts.draw_text(&count_text, content_x, aw_y + 67.0, 11.0, LIGHTGRAY);
        } else if can_afford_altar {
            fonts.draw_text("ГОТОВ К ПОСТРОЙКЕ [50 душ]", content_x, aw_y + 38.0, 13.0, GREEN);
            let bar_w = 220.0;
            draw_rectangle(content_x, aw_y + 46.0, bar_w, 7.0, GREEN);
            let count_text = format!("Активно: {} алт. (+{:.0}/2с доход)", world.structures.len(), world.structures.len() as f32 * 3.0);
            fonts.draw_text(&count_text, content_x, aw_y + 67.0, 11.0, SKYBLUE);
        } else {
            fonts.draw_text("ГОТОВ (Не хватает душ: 50)", content_x, aw_y + 38.0, 12.0, GRAY);
            let count_text = format!("Активно: {} алт. (+{:.0}/2с доход)", world.structures.len(), world.structures.len() as f32 * 3.0);
            fonts.draw_text(&count_text, content_x, aw_y + 58.0, 12.0, LIGHTGRAY);
        }

        // Проверка наведения на виджет Алтаря
        if mx >= aw_x && mx <= aw_x + aw_w && my >= aw_y && my <= aw_y + aw_h {
            hovered_tooltip = Some(HoverTooltip::AltarWidget(aw_x, aw_y));
        }

        // 7. Тактическая миникарта (справа вверху)
        draw_minimap(world, cam_pos, cam_zoom, fonts);

        // 8. Всплывающий текст урона и меток (в экранных координатах, всегда четко)
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
            let can_afford = director.points >= config.cost;

            let bx = start_x + (i as f32) * (btn_width + 12.0);

            let bg_color = if !is_unlocked {
                Color::new(0.14, 0.14, 0.17, 0.92)
            } else if !can_afford {
                Color::new(0.24, 0.18, 0.18, 0.92)
            } else {
                Color::new(0.14, 0.35, 0.20, 0.95)
            };

            let border_color = if is_unlocked && can_afford {
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
            fonts.draw_heading(&header, bx + 6.0, btn_y + 20.0, 14.0, WHITE);

            if !is_unlocked {
                let lock_text = format!("Нужно: {:.0}", config.unlock_spent_threshold);
                fonts.draw_text(&lock_text, bx + 6.0, btn_y + 40.0, 13.0, RED);

                let prog = (director.total_points_spent / config.unlock_spent_threshold).clamp(0.0, 1.0);
                draw_rectangle(bx + 6.0, btn_y + 52.0, (btn_width - 56.0) * prog, 6.0, RED);
            } else {
                let cost_text = format!("Цена: {:.0} ({} шт)", config.cost, config.count);
                let cost_color = if can_afford { GOLD } else { GRAY };
                fonts.draw_text(&cost_text, bx + 6.0, btn_y + 40.0, 13.0, cost_color);

                let grow_text = format!("+ {:.2}/2c доход", config.cost * 0.02);
                fonts.draw_text(&grow_text, bx + 6.0, btn_y + 58.0, 12.0, GREEN);
            }

            // Проверка наведения на кнопку призыва
            if mx >= bx && mx <= bx + btn_width && my >= btn_y && my <= btn_y + btn_height {
                hovered_tooltip = Some(HoverTooltip::Squad(tier, bx, btn_y));
            }
        }
    }

    // 10. Проверка наведения курсора на объекты в игровом мире (World Entities: Алтарь, Босс, Мобы)
    if hovered_tooltip.is_none() && my > 95.0 && my < screen_h - 95.0 {
        let world_mx = (mx - screen_w / 2.0) / cam_zoom + cam_pos.x;
        let world_my = (my - screen_h / 2.0) / cam_zoom + cam_pos.y;
        let world_mpos = Vec2::new(world_mx, world_my);

        // 10.1 Наведение на Босса
        if world_mpos.distance(world.boss_pos) < 36.0 {
            let state_str = if world.boss_still_time >= 2.0 {
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

        // 10.2 Наведение на Алтарь Крови
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

        // 10.3 Наведение на Моба
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

    // 11. Отрисовка активной карточки-подсказки (Tooltip) поверх всех элементов
    if let Some(tooltip) = hovered_tooltip {
        match tooltip {
            HoverTooltip::Weapon(w, slot_x, slot_y) => {
                draw_weapon_tooltip(&w, slot_x, slot_y, screen_w, fonts);
            }
            HoverTooltip::Passive(p, slot_x, slot_y) => {
                draw_passive_tooltip(&p, slot_x, slot_y, screen_w, fonts);
            }
            HoverTooltip::Squad(tier, btn_x, btn_y) => {
                draw_squad_tooltip(tier, btn_x, btn_y, screen_w, fonts, world);
            }
            HoverTooltip::WorldEntity(screen_pos, lines) => {
                draw_world_entity_tooltip(screen_pos, lines, screen_w, screen_h, fonts);
            }
            HoverTooltip::AltarWidget(aw_x, aw_y) => {
                draw_altar_widget_tooltip(aw_x, aw_y, screen_w, fonts, world);
            }
        }
    }
}

/// Всплывающая подсказка для виджета Алтаря Крови
fn draw_altar_widget_tooltip(aw_x: f32, aw_y: f32, screen_w: f32, fonts: &GameFonts, world: &World) {
    let card_w = 360.0;
    let card_h = 175.0;
    let card_x = (aw_x + 300.0).clamp(10.0, screen_w - card_w - 10.0);
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

    // Фон и обводка
    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.06, 0.08, 0.14, 0.96));
    let border_color = if w.is_evolved { GOLD } else { SKYBLUE };
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, border_color);

    // Заголовок
    let title = format!("{} [Ур. {}/8]", def.name, w.level);
    fonts.draw_heading(&title, card_x + 12.0, card_y + 22.0, 16.0, border_color);

    // Характеристики
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

    // Описание механики
    fonts.draw_text(def.description, card_x + 12.0, card_y + 68.0, 12.0, LIGHTGRAY);

    // Рецепт эволюции
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

    // Точный текущий бонус
    let bonus_text = match p.kind {
        PassiveType::SpeedGloves => format!("Текущий бонус: +{}% к скорости атаки оружия", p.level * 15),
        PassiveType::GiantHeart => format!("Текущий бонус: +{}% к макс. здоровью и регенерации", p.level * 25),
        PassiveType::CooldownTome => format!("Текущий бонус: -{}% времени отката оружия", p.level * 20),
        PassiveType::LightArmor => format!("Текущий бонус: +{:.0} к плоской броне, +{}% к защите", p.level as f32 * 8.0, p.level * 15),
        PassiveType::FrostRing => format!("Текущий бонус: +{}% к длительности замедления врагов", p.level * 20),
        PassiveType::CorrosiveFlask => format!("Текущий бонус: +{}% к пробитию брони всеми атаками", p.level * 15),
    };
    fonts.draw_text(&bonus_text, card_x + 12.0, card_y + 44.0, 13.0, GREEN);

    // Описание
    fonts.draw_text(def.description, card_x + 12.0, card_y + 68.0, 12.0, LIGHTGRAY);

    let evo_synergy = match p.kind {
        PassiveType::SpeedGloves => "Связка для эволюции: Клинок Ветра -> Буря Клинков",
        PassiveType::GiantHeart => "Связка для эволюции: Огненная Аура -> Адское Пекло",
        PassiveType::CooldownTome => "Связка для эволюции: Магический Жезл -> Луч Смерти",
        PassiveType::LightArmor => "Связка для эволюции: Святой Молот -> Праведный Щит",
        PassiveType::FrostRing => "Связка для эволюции: Ледяные Осколки -> Ледниковая Буря",
        PassiveType::CorrosiveFlask => "Связка для эволюции: Чумные Сферы -> Токсичный Мор",
    };
    fonts.draw_text(evo_synergy, card_x + 12.0, card_y + 92.0, 12.0, YELLOW);
}

/// Всплывающая подсказка тактического досье отряда мобов
fn draw_squad_tooltip(tier: u8, bx: f32, by: f32, screen_w: f32, fonts: &GameFonts, world: &World) {
    let config = get_squad_config(tier);
    let card_w = 370.0;
    let card_h = 195.0;
    let card_x = (bx + 97.0 - card_w / 2.0).clamp(10.0, screen_w - card_w - 10.0);
    let card_y = by - card_h - 10.0;

    let director = world.directors.first();
    let is_unlocked = director.map_or(false, |d| d.total_points_spent >= config.unlock_spent_threshold);

    draw_rectangle(card_x, card_y, card_w, card_h, Color::new(0.06, 0.08, 0.12, 0.97));
    let border_color = if is_unlocked { GREEN } else { RED };
    draw_rectangle_lines(card_x, card_y, card_w, card_h, 2.0, border_color);

    let title = format!("Отряд: {} (Тир {})", config.name, tier);
    fonts.draw_heading(&title, card_x + 14.0, card_y + 24.0, 16.0, if is_unlocked { GOLD } else { LIGHTGRAY });

    let cost_info = format!("Цена: {:.0} Душ | Отряд: {} бойцов | +{:.2}/2с доход", config.cost, config.count, config.cost * 0.02);
    fonts.draw_text(&cost_info, card_x + 14.0, card_y + 46.0, 13.0, GREEN);

    // Детальные статы моба
    let (hp_val, spd_val, dmg_val, arm_val, res_str) = match tier {
        1 => (32.0, 105.0, 7.0, 0.0, "Физ: 5% | Огонь: 0% | Холод: 0% | Святость: -40%"),
        2 => (50.0, 160.0, 12.0, 1.0, "Физ: 10% | Огонь: 0% | Холод: 0% | Святость: -20%"),
        3 => (75.0, 80.0, 16.0, 2.0, "Физ: 15% | Огонь: 20% | Холод: -10% | Святость: -40%"),
        4 => (380.0, 52.0, 32.0, 14.0, "Физ: 35% | Холод: 100% (Иммун) | Огонь: -50% (Уязвим!)"),
        _ => (130.0, 185.0, 220.0, 2.0, "Физ: 10% | Огонь: 50% | Холод: 0% | Взрыв по площади: 220"),
    };

    let combat_text = if tier == 5 {
        format!("HP: {:.0} | Скорость: {:.0} | Броня: {:.0} | Взрыв: 220 урона", hp_val, spd_val, arm_val)
    } else if tier == 3 {
        format!("HP: {:.0} | Скорость: {:.0} | Броня: {:.0} | Урон: {:.0} (Дальний бой 260px)", hp_val, spd_val, arm_val, dmg_val)
    } else {
        format!("HP: {:.0} | Скорость: {:.0} | Броня: {:.0} | Урон: {:.0} (Ближний бой)", hp_val, spd_val, arm_val, dmg_val)
    };
    fonts.draw_text(&combat_text, card_x + 14.0, card_y + 68.0, 13.0, WHITE);
    fonts.draw_text(res_str, card_x + 14.0, card_y + 90.0, 12.0, SKYBLUE);

    // Тактическое назначение
    fonts.draw_text(config.description, card_x + 14.0, card_y + 116.0, 12.0, YELLOW);

    if !is_unlocked {
        let req = format!("🔒 Откроется после траты еще {:.0} Душ", config.unlock_spent_threshold - director.map_or(0.0, |d| d.total_points_spent));
        fonts.draw_text(&req, card_x + 14.0, card_y + 146.0, 13.0, RED);
    } else {
        fonts.draw_text("⚡ Нажмите [1..5] или кликните ЛКМ для призыва отряда кольцом вокруг арены", card_x + 14.0, card_y + 146.0, 11.0, LIGHTGRAY);
        fonts.draw_text("Кулдаун призыва снят: призывайте мгновенно при наличии Очков Душ!", card_x + 14.0, card_y + 168.0, 11.0, GOLD);
    }
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
