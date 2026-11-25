use macroquad::prelude::*;
use crate::content::passives::get_passive_def;
use crate::content::weapons::get_weapon_def;
use crate::ecs::components::inventory::{PassiveType, WeaponType};
use crate::ecs::world::World;
use crate::render::font::GameFonts;
use crate::render::textures::GameTextures;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemInspectionTarget {
    Weapon(WeaponType, u8, u32), // kind, level, limit_break_level
    Passive(PassiveType, u8),    // kind, level
}

/// Находит связку компонентов эволюции для оружия: (базовое, пассивка, эволюция)
pub fn get_weapon_synthesis_recipe(w: WeaponType) -> Option<(WeaponType, PassiveType, WeaponType)> {
    match w {
        WeaponType::WindBlade | WeaponType::BladeSquall => Some((WeaponType::WindBlade, PassiveType::SpeedGloves, WeaponType::BladeSquall)),
        WeaponType::FireAura | WeaponType::NetherBlaze => Some((WeaponType::FireAura, PassiveType::GiantHeart, WeaponType::NetherBlaze)),
        WeaponType::ArcaneScepter | WeaponType::DeathRay => Some((WeaponType::ArcaneScepter, PassiveType::CooldownTome, WeaponType::DeathRay)),
        WeaponType::HolyHammer | WeaponType::LightBastion => Some((WeaponType::HolyHammer, PassiveType::LightArmor, WeaponType::LightBastion)),
        WeaponType::IceShards | WeaponType::BlizzardStorm => Some((WeaponType::IceShards, PassiveType::FrostRing, WeaponType::BlizzardStorm)),
        WeaponType::PoisonOrbs | WeaponType::ToxicPestilence => Some((WeaponType::PoisonOrbs, PassiveType::CorrosiveFlask, WeaponType::ToxicPestilence)),
        WeaponType::ExecutionerAxe | WeaponType::WhirlwindCleaver => Some((WeaponType::ExecutionerAxe, PassiveType::CrimsonHeartstone, WeaponType::WhirlwindCleaver)),
        WeaponType::CelestialBow | WeaponType::StarfallBarrage => Some((WeaponType::CelestialBow, PassiveType::EagleScope, WeaponType::StarfallBarrage)),
        WeaponType::ThunderOrb | WeaponType::ThunderstormWrath => Some((WeaponType::ThunderOrb, PassiveType::ChronoHourglass, WeaponType::ThunderstormWrath)),
        WeaponType::BloodDaggers | WeaponType::BloodHarvest => Some((WeaponType::BloodDaggers, PassiveType::MagnetSphere, WeaponType::BloodHarvest)),
        WeaponType::SingularityOrb | WeaponType::EventHorizon => Some((WeaponType::SingularityOrb, PassiveType::TeslaCoil, WeaponType::EventHorizon)),
        WeaponType::ChainFrost | WeaponType::AbsoluteZero => Some((WeaponType::ChainFrost, PassiveType::FortuneCoin, WeaponType::AbsoluteZero)),
        WeaponType::BrimstoneRay | WeaponType::InfernalCataclysm => Some((WeaponType::BrimstoneRay, PassiveType::PrismaticFocus, WeaponType::InfernalCataclysm)),
        WeaponType::MissileSwarm | WeaponType::DoomsdayBarrage => Some((WeaponType::MissileSwarm, PassiveType::TitanAegis, WeaponType::DoomsdayBarrage)),
        WeaponType::ChainLightning | WeaponType::StormGodWrath => Some((WeaponType::ChainLightning, PassiveType::StaticCore, WeaponType::StormGodWrath)),
        WeaponType::DivineBell | WeaponType::CathedralAura => Some((WeaponType::DivineBell, PassiveType::DivinityAura, WeaponType::CathedralAura)),
        WeaponType::SoulPhantoms | WeaponType::BansheeWail => Some((WeaponType::SoulPhantoms, PassiveType::SoulHarvester, WeaponType::BansheeWail)),
        WeaponType::AegisOrbiters | WeaponType::InvincibleBastion => Some((WeaponType::AegisOrbiters, PassiveType::RevivalPhylactery, WeaponType::InvincibleBastion)),
        WeaponType::FallingLuminary | WeaponType::ArmageddonStorm => Some((WeaponType::FallingLuminary, PassiveType::Pyromaniac, WeaponType::ArmageddonStorm)),
        WeaponType::VoidRift | WeaponType::SingularityCollapse => Some((WeaponType::VoidRift, PassiveType::OccultPrism, WeaponType::SingularityCollapse)),
        WeaponType::SacredScriptures | WeaponType::InfernalLiturgy => Some((WeaponType::SacredScriptures, PassiveType::ChronoHourglass, WeaponType::InfernalLiturgy)),
        WeaponType::FaithBoomerang | WeaponType::HeavenlyJudgment => Some((WeaponType::FaithBoomerang, PassiveType::FortuneCoin, WeaponType::HeavenlyJudgment)),
        WeaponType::CursedChalice | WeaponType::BloodDeluge => Some((WeaponType::CursedChalice, PassiveType::PrismaticFocus, WeaponType::BloodDeluge)),
        WeaponType::VampiricAura | WeaponType::SoulFeast => Some((WeaponType::VampiricAura, PassiveType::CrimsonHeartstone, WeaponType::SoulFeast)),
        WeaponType::RunicPrism | WeaponType::SpatialShatter => Some((WeaponType::RunicPrism, PassiveType::TwinPrism, WeaponType::SpatialShatter)),
        WeaponType::AbyssalBeam | WeaponType::ApocalypseCascade => Some((WeaponType::AbyssalBeam, PassiveType::CooldownTome, WeaponType::ApocalypseCascade)),
        WeaponType::ChronoStasis | WeaponType::InfiniteDilation => Some((WeaponType::ChronoStasis, PassiveType::MoonDew, WeaponType::InfiniteDilation)),
        WeaponType::AstralCloak | WeaponType::BloodEmperorShroud => Some((WeaponType::AstralCloak, PassiveType::TitanAegis, WeaponType::BloodEmperorShroud)),
    }
}

/// Находит связку компонентов эволюции для пассивки (если она служит рецептом)
pub fn get_passive_synthesis_recipe(p: PassiveType) -> Option<(WeaponType, PassiveType, WeaponType)> {
    match p {
        PassiveType::SpeedGloves => Some((WeaponType::WindBlade, p, WeaponType::BladeSquall)),
        PassiveType::GiantHeart => Some((WeaponType::FireAura, p, WeaponType::NetherBlaze)),
        PassiveType::CooldownTome => Some((WeaponType::ArcaneScepter, p, WeaponType::DeathRay)),
        PassiveType::LightArmor => Some((WeaponType::HolyHammer, p, WeaponType::LightBastion)),
        PassiveType::FrostRing => Some((WeaponType::IceShards, p, WeaponType::BlizzardStorm)),
        PassiveType::CorrosiveFlask => Some((WeaponType::PoisonOrbs, p, WeaponType::ToxicPestilence)),
        PassiveType::CrimsonHeartstone => Some((WeaponType::ExecutionerAxe, p, WeaponType::WhirlwindCleaver)),
        PassiveType::EagleScope => Some((WeaponType::CelestialBow, p, WeaponType::StarfallBarrage)),
        PassiveType::ChronoHourglass => Some((WeaponType::ThunderOrb, p, WeaponType::ThunderstormWrath)),
        PassiveType::MagnetSphere => Some((WeaponType::BloodDaggers, p, WeaponType::BloodHarvest)),
        PassiveType::TeslaCoil => Some((WeaponType::SingularityOrb, p, WeaponType::EventHorizon)),
        PassiveType::FortuneCoin => Some((WeaponType::ChainFrost, p, WeaponType::AbsoluteZero)),
        PassiveType::PrismaticFocus => Some((WeaponType::BrimstoneRay, p, WeaponType::InfernalCataclysm)),
        PassiveType::TitanAegis => Some((WeaponType::MissileSwarm, p, WeaponType::DoomsdayBarrage)),
        PassiveType::StaticCore => Some((WeaponType::ChainLightning, p, WeaponType::StormGodWrath)),
        PassiveType::DivinityAura => Some((WeaponType::DivineBell, p, WeaponType::CathedralAura)),
        PassiveType::SoulHarvester => Some((WeaponType::SoulPhantoms, p, WeaponType::BansheeWail)),
        PassiveType::RevivalPhylactery => Some((WeaponType::AegisOrbiters, p, WeaponType::InvincibleBastion)),
        PassiveType::Pyromaniac => Some((WeaponType::FallingLuminary, p, WeaponType::ArmageddonStorm)),
        PassiveType::OccultPrism => Some((WeaponType::VoidRift, p, WeaponType::SingularityCollapse)),
        PassiveType::TwinPrism => Some((WeaponType::RunicPrism, p, WeaponType::SpatialShatter)),
        PassiveType::MoonDew => Some((WeaponType::ChronoStasis, p, WeaponType::InfiniteDilation)),
        _ => None,
    }
}

/// Отрисовывает детальное модальное окно осмотра предмета с Древом Синтеза Эволюции
pub fn draw_item_inspection_modal(
    target: ItemInspectionTarget,
    world: &World,
    fonts: &GameFonts,
    textures: &GameTextures,
) -> bool {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let (mx, my) = mouse_position();

    // Затемнение фона
    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.82));

    let modal_w = (screen_w - 120.0).clamp(880.0, 1080.0);
    let modal_h = (screen_h - 80.0).clamp(540.0, 640.0);
    let x = (screen_w - modal_w) / 2.0;
    let y = (screen_h - modal_h) / 2.0;

    // Фон окна
    draw_rectangle(x, y, modal_w, modal_h, Color::new(0.07, 0.07, 0.11, 0.98));
    draw_rectangle_lines(x, y, modal_w, modal_h, 2.5, Color::new(0.85, 0.65, 0.20, 1.0));

    // Верхняя заголовочная плашка
    draw_rectangle(x, y, modal_w, 42.0, Color::new(0.12, 0.12, 0.18, 1.0));
    draw_rectangle_lines(x, y, modal_w, 42.0, 1.5, DARKGRAY);

    let (modal_title, title_color) = match target {
        ItemInspectionTarget::Weapon(w, lvl, lb) => {
            let def = get_weapon_def(w);
            if lb > 0 {
                (format!("ДОСЬЕ ОРУЖИЯ: {} [ВОЗВЫШЕНИЕ +{}]", def.name.to_uppercase(), lb), GOLD)
            } else {
                (format!("ДОСЬЕ ОРУЖИЯ: {} [УРОВЕНЬ {}/8]", def.name.to_uppercase(), lvl), SKYBLUE)
            }
        }
        ItemInspectionTarget::Passive(p, lvl) => {
            let def = get_passive_def(p);
            (format!("ДОСЬЕ РЕЛИКВИИ: {} [УРОВЕНЬ {}/5]", def.name.to_uppercase(), lvl), Color::new(0.85, 0.55, 1.0, 1.0))
        }
    };
    fonts.draw_heading(&modal_title, x + 24.0, y + 28.0, 16.0, title_color);

    // Кнопка [ X ] закрытия
    let close_btn_w = 38.0;
    let close_btn_h = 30.0;
    let close_btn_x = x + modal_w - close_btn_w - 8.0;
    let close_btn_y = y + 6.0;
    let is_close_hover = mx >= close_btn_x && mx <= close_btn_x + close_btn_w && my >= close_btn_y && my <= close_btn_y + close_btn_h;
    draw_rectangle(close_btn_x, close_btn_y, close_btn_w, close_btn_h, if is_close_hover { RED } else { Color::new(0.20, 0.10, 0.12, 0.9) });
    draw_rectangle_lines(close_btn_x, close_btn_y, close_btn_w, close_btn_h, 1.5, WHITE);
    let cw = fonts.measure_heading("X", 16.0).width;
    fonts.draw_heading("X", close_btn_x + (close_btn_w - cw) / 2.0, close_btn_y + 21.0, 16.0, WHITE);

    let close_requested = is_key_pressed(KeyCode::Escape)
        || is_key_pressed(KeyCode::I)
        || (is_mouse_button_pressed(MouseButton::Left) && is_close_hover);

    // Содержимое окна
    let content_x = x + 24.0;
    let content_y = y + 54.0;
    let content_w = modal_w - 48.0;

    // ВЕРХНЯЯ СЕКЦИЯ: Иконка + Базовые характеристики + Прогресс уровня
    let top_h = 175.0;
    draw_rectangle(content_x, content_y, content_w, top_h, Color::new(0.05, 0.05, 0.08, 0.95));
    draw_rectangle_lines(content_x, content_y, content_w, top_h, 1.5, Color::new(0.3, 0.35, 0.50, 0.8));

    // Большая иконка предмета слева
    let icon_box_s = 96.0;
    let icon_x = content_x + 16.0;
    let icon_y = content_y + 16.0;
    draw_rectangle(icon_x, icon_y, icon_box_s, icon_box_s, Color::new(0.09, 0.10, 0.15, 0.95));
    draw_rectangle_lines(icon_x, icon_y, icon_box_s, icon_box_s, 2.0, GOLD);

    let icon_tex = match target {
        ItemInspectionTarget::Weapon(w, _, _) => textures.get_weapon_icon(w),
        ItemInspectionTarget::Passive(p, _) => textures.get_passive_icon(p),
    };
    if let Some(tex) = icon_tex {
        draw_texture_ex(
            tex,
            icon_x + 8.0,
            icon_y + 8.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(icon_box_s - 16.0, icon_box_s - 16.0)),
                ..Default::default()
            },
        );
    }

    // Текстовая информация в верхней секции
    let desc_x = icon_x + icon_box_s + 20.0;
    match target {
        ItemInspectionTarget::Weapon(w, level, lb) => {
            let def = get_weapon_def(w);
            let dmg_type_str = match def.damage_type {
                crate::ecs::components::combat::DamageType::Physical => "Физический",
                crate::ecs::components::combat::DamageType::Fire => "Огненный (DoT горение)",
                crate::ecs::components::combat::DamageType::Ice => "Ледяной (Замедление/Заморозка)",
                crate::ecs::components::combat::DamageType::Holy => "Святой (Игнорирует 25% брони)",
                crate::ecs::components::combat::DamageType::True => "Чистый (True Damage, игнорирует броню)",
                _ => "Стихийный",
            };

            // Название и тип урона
            fonts.draw_heading(def.name, desc_x, content_y + 26.0, 16.0, GOLD);
            fonts.draw_text(&format!("Тип: {} | Урон: {} | Откат: {:.2}с | Базовый: {:.0}", def.delivery.name_ru(), dmg_type_str, def.base_cooldown, def.base_damage), desc_x, content_y + 46.0, 11.0, SKYBLUE);

            // Описание боевой роли
            fonts.draw_text(def.description, desc_x, content_y + 68.0, 11.5, LIGHTGRAY);

            // Детали следующего уровня / Limit Break
            draw_line(desc_x, content_y + 84.0, content_x + content_w - 16.0, content_y + 84.0, 1.0, DARKGRAY);

            if lb > 0 {
                fonts.draw_heading("СВЕРХ-ВОЗВЫШЕНИЕ (LIMIT BREAK):", desc_x, content_y + 106.0, 13.0, GOLD);
                fonts.draw_text(&format!("• Текущий бонус возвышения: +{}% урона, +{} снарядов к каждому залпу", lb * 10, (lb / 3) + 1), desc_x, content_y + 124.0, 12.0, GREEN);
                fonts.draw_text("• Каждый следующий уровень возвышения увеличивает урон на +10% и сокращает кулдаун атак!", desc_x, content_y + 144.0, 11.5, YELLOW);
            } else if level >= 8 {
                fonts.draw_heading("МАКСИМАЛЬНЫЙ БАЗОВЫЙ УРОВЕНЬ (8/8):", desc_x, content_y + 106.0, 13.0, GOLD);
                fonts.draw_text("• Оружие готово к Синтезу Эволюции! При наличии парной пассивной реликвии при следующем Level-Up", desc_x, content_y + 124.0, 12.0, GREEN);
                fonts.draw_text("  вам будет предложена карта совершенного оружия. После эволюции откроется Limit Break!", desc_x, content_y + 144.0, 11.5, YELLOW);
            } else {
                fonts.draw_heading("ПРОГРЕСС УЛУЧШЕНИЙ:", desc_x, content_y + 106.0, 13.0, SKYBLUE);
                let next_dmg = def.base_damage * (1.0 + (level as f32) * 0.15);
                let cur_dmg = def.base_damage * (1.0 + (level as f32 - 1.0) * 0.15);
                let next_cd = (def.base_cooldown - (level as f32 * 0.04)).max(0.20);
                fonts.draw_text(&format!("• Текущий урон: {:.0} -> Следующий уровень (Ур. {}): {:.0} (+15%) | Откат: {:.2}с", cur_dmg, level + 1, next_dmg, next_cd), desc_x, content_y + 124.0, 12.0, GREEN);
                fonts.draw_text("• На 8 уровне полностью открывается рецепт трансформации в Совершенный Артефакт!", desc_x, content_y + 144.0, 11.5, YELLOW);
            }
        }
        ItemInspectionTarget::Passive(p, level) => {
            let def = get_passive_def(p);
            fonts.draw_heading(def.name, desc_x, content_y + 26.0, 16.0, Color::new(0.85, 0.55, 1.0, 1.0));
            fonts.draw_text("Категория: Пассивная Реликвия Босса | Максимальный уровень: 5", desc_x, content_y + 46.0, 12.0, SKYBLUE);
            fonts.draw_text(def.description, desc_x, content_y + 68.0, 11.5, LIGHTGRAY);

            draw_line(desc_x, content_y + 84.0, content_x + content_w - 16.0, content_y + 84.0, 1.0, DARKGRAY);

            fonts.draw_heading("ЭФФЕКТИВНОСТЬ ПРОКАЧКИ:", desc_x, content_y + 106.0, 13.0, GOLD);
            let cur_bonus = format_passive_bonus(p, level);
            let next_bonus = format_passive_bonus(p, (level + 1).min(5));
            if level >= 5 {
                fonts.draw_text(&format!("• ТЕКУЩИЙ УРОВЕНЬ (5/5 МАКС): {}", cur_bonus), desc_x, content_y + 126.0, 12.0, GREEN);
                fonts.draw_text("• Достигнут предел силы реликвии! Все сопутствующие эволюции получают максимальный катализ.", desc_x, content_y + 146.0, 11.5, YELLOW);
            } else {
                fonts.draw_text(&format!("• Сейчас (Ур. {}): {}", level, cur_bonus), desc_x, content_y + 126.0, 12.0, WHITE);
                fonts.draw_text(&format!("• Следующий (Ур. {}): {}", level + 1, next_bonus), desc_x, content_y + 146.0, 12.0, GREEN);
            }
        }
    }

    // НИЖНЯЯ СЕКЦИЯ: ДРЕВО СИНТЕЗА ЭВОЛЮЦИИ (Дерево компонентов сборки)
    let bot_y = content_y + top_h + 16.0;
    let bot_h = modal_h - (bot_y - y) - 20.0;
    draw_rectangle(content_x, bot_y, content_w, bot_h, Color::new(0.05, 0.05, 0.08, 0.95));
    draw_rectangle_lines(content_x, bot_y, content_w, bot_h, 1.5, Color::new(0.70, 0.50, 0.20, 0.8));

    // Заголовок древа синтеза
    draw_rectangle(content_x, bot_y, content_w, 32.0, Color::new(0.12, 0.10, 0.18, 0.95));
    draw_rectangle_lines(content_x, bot_y, content_w, 32.0, 1.0, DARKGRAY);
    fonts.draw_heading("ДРЕВО СИНТЕЗА ЭВОЛЮЦИИ (РЕЦЕПТ СБОРКИ АРТЕФАКТА)", content_x + 16.0, bot_y + 22.0, 13.0, GOLD);

    // Определяем рецепт в зависимости от выбранного предмета
    let maybe_recipe = match target {
        ItemInspectionTarget::Weapon(w, _, _) => get_weapon_synthesis_recipe(w),
        ItemInspectionTarget::Passive(p, _) => get_passive_synthesis_recipe(p),
    };

    if let Some((base_w, req_p, evo_w)) = maybe_recipe {
        draw_synthesis_recipe_tree(
            content_x + 16.0,
            bot_y + 44.0,
            content_w - 32.0,
            bot_h - 52.0,
            base_w,
            req_p,
            evo_w,
            world,
            fonts,
            textures,
        );
    } else {
        // Если это чистая легендарная реликвия без оружейного рецепта
        fonts.draw_heading("АВТОНОМНАЯ ЛЕГЕНДАРНАЯ РЕЛИКВИЯ", content_x + 30.0, bot_y + 70.0, 15.0, GOLD);
        fonts.draw_text("Этот предмет является уникальным сокровищем и не требует базового оружия для активации.", content_x + 30.0, bot_y + 98.0, 12.5, WHITE);
        fonts.draw_text("Он напрямую усиливает пассивные характеристики Босса, добавляя вампиризм, воскрешение,", content_x + 30.0, bot_y + 118.0, 12.5, LIGHTGRAY);
        fonts.draw_text("непрерывную регенерацию при остановке (Мицелий) или цепные электрические разряды.", content_x + 30.0, bot_y + 138.0, 12.5, LIGHTGRAY);
    }

    close_requested
}

/// Отрисовывает схему сборки: [Компонент 1] + [Компонент 2] ➔ [ЭВОЛЮЦИЯ]
fn draw_synthesis_recipe_tree(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    base_w: WeaponType,
    req_p: PassiveType,
    evo_w: WeaponType,
    world: &World,
    fonts: &GameFonts,
    textures: &GameTextures,
) {
    let base_def = get_weapon_def(base_w);
    let pass_def = get_passive_def(req_p);
    let evo_def = get_weapon_def(evo_w);

    // Проверяем наличие компонентов у Босса в инвентаре
    let (has_wep, wep_lvl) = world.boss_inventory.weapons.iter()
        .find(|w| w.kind == base_w || (w.is_evolved && w.kind == evo_w))
        .map(|w| (true, w.level))
        .unwrap_or((false, 0));

    let (has_pass, pass_lvl) = world.boss_inventory.passives.iter()
        .find(|p| p.kind == req_p)
        .map(|p| (true, p.level))
        .unwrap_or((false, 0));

    let is_already_evolved = world.boss_inventory.has_weapon(evo_w);
    let is_ready_to_evolve = (has_wep && wep_lvl >= 8 && has_pass) || is_already_evolved;

    // Сетка: Слева Компонент 1 (Базовое оружие), По центру `+`, Затем Компонент 2 (Реликвия), Стрелка `➔`, Справа Эволюция!
    let card_w = (w - 120.0) / 3.0;
    let card_h = h.min(180.0);

    // 1. КАРТОЧКА: БАЗОВОЕ ОРУЖИЕ
    let c1_x = x;
    let (c1_border, c1_status, c1_col) = if is_already_evolved || wep_lvl >= 8 {
        (GREEN, format!("УР. {}/8 [ГОТОВО К СИНТЕЗУ]", wep_lvl), GREEN)
    } else if has_wep {
        (ORANGE, format!("УР. {}/8 [ТРЕБУЕТСЯ УР. 8]", wep_lvl), ORANGE)
    } else {
        (GRAY, "НЕ В ИНВЕНТАРЕ".to_string(), RED)
    };
    draw_component_card(c1_x, y, card_w, card_h, "КОМПОНЕНТ 1 (ОРУЖИЕ)", base_def.name, &c1_status, c1_col, textures.get_weapon_icon(base_w), c1_border, fonts);

    // Знак плюса `+`
    let plus_x = c1_x + card_w + 14.0;
    fonts.draw_heading("+", plus_x, y + card_h / 2.0 + 8.0, 26.0, GOLD);

    // 2. КАРТОЧКА: ПАССИВНАЯ РЕЛИКВИЯ
    let c2_x = plus_x + 30.0;
    let (c2_border, c2_status, c2_col) = if has_pass {
        (GREEN, format!("УР. {}/5 [В ИНВЕНТАРЕ]", pass_lvl), GREEN)
    } else {
        (GRAY, "ТРЕБУЕТСЯ НАЙТИ".to_string(), RED)
    };
    draw_component_card(c2_x, y, card_w, card_h, "КОМПОНЕНТ 2 (КАТАЛИЗАТОР)", pass_def.name, &c2_status, c2_col, textures.get_passive_icon(req_p), c2_border, fonts);

    // Стрелка `->`
    let arrow_x = c2_x + card_w + 12.0;
    fonts.draw_heading("->", arrow_x, y + card_h / 2.0 + 8.0, 24.0, if is_ready_to_evolve { GOLD } else { GRAY });

    // 3. КАРТОЧКА: РЕЗУЛЬТАТ (СОВЕРШЕННОЕ ОРУЖИЕ)
    let c3_x = arrow_x + 36.0;
    let (c3_border, c3_status, c3_col) = if is_already_evolved {
        (GOLD, "СИНТЕЗИРОВАНО В БОЮ!".to_string(), GOLD)
    } else if is_ready_to_evolve {
        (GREEN, "ГОТОВО К ЭВОЛЮЦИИ!".to_string(), GREEN)
    } else {
        (DARKGRAY, "ЗАБЛОКИРОВАНО".to_string(), LIGHTGRAY)
    };
    draw_component_card(c3_x, y, card_w, card_h, "РЕЗУЛЬТАТ: ЭВОЛЮЦИЯ", evo_def.name, &c3_status, c3_col, textures.get_weapon_icon(evo_w), c3_border, fonts);

    // Нижняя плашка с описанием супер-эффекта
    let desc_box_y = y + card_h + 12.0;
    let desc_box_h = h - card_h - 12.0;
    if desc_box_h > 24.0 {
        draw_rectangle(x, desc_box_y, w, desc_box_h, Color::new(0.08, 0.08, 0.14, 0.95));
        draw_rectangle_lines(x, desc_box_y, w, desc_box_h, 1.0, if is_already_evolved { GOLD } else { DARKGRAY });
        let eff_title = format!("СУПЕРЭФФЕКТ [{}]:", evo_def.name);
        fonts.draw_heading(&eff_title, x + 14.0, desc_box_y + 19.0, 12.5, GOLD);
        fonts.draw_text(evo_def.description, x + 14.0, desc_box_y + 36.0, 11.5, WHITE);
    }
}

/// Отрисовывает одну карточку компонента сборки
fn draw_component_card(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    role: &str,
    name: &str,
    status: &str,
    status_col: Color,
    icon: Option<&Texture2D>,
    border_col: Color,
    fonts: &GameFonts,
) {
    draw_rectangle(x, y, w, h, Color::new(0.08, 0.08, 0.12, 0.95));
    draw_rectangle_lines(x, y, w, h, 2.0, border_col);

    // Роль компонента
    fonts.draw_heading(role, x + 10.0, y + 18.0, 10.5, border_col);

    // Иконка компонента
    let icon_s = 48.0;
    let ix = x + 12.0;
    let iy = y + 26.0;
    draw_rectangle(ix, iy, icon_s, icon_s, Color::new(0.04, 0.04, 0.07, 0.95));
    draw_rectangle_lines(ix, iy, icon_s, icon_s, 1.5, border_col);
    if let Some(tex) = icon {
        draw_texture_ex(
            tex,
            ix + 2.0,
            iy + 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(icon_s - 4.0, icon_s - 4.0)),
                ..Default::default()
            },
        );
    }

    // Имя и статус компонента справа от иконки
    let tx = ix + icon_s + 10.0;
    fonts.draw_heading(name, tx, iy + 20.0, 12.0, WHITE);
    fonts.draw_text(status, tx, iy + 40.0, 10.5, status_col);
}

fn format_passive_bonus(p: PassiveType, lvl: u8) -> String {
    match p {
        PassiveType::SpeedGloves => format!("+{}% к скорости атаки всего оружия", lvl * 15),
        PassiveType::GiantHeart => format!("+{}% к максимальному здоровью и регенерации", lvl * 25),
        PassiveType::CooldownTome => format!("-{}% ко всем кулдаунам оружия и рывка", lvl * 20),
        PassiveType::LightArmor => format!("+{:.0} к броне, +{}% сопротивления физ. урону", lvl as f32 * 8.0, lvl * 15),
        PassiveType::FrostRing => format!("+{}% к силе и радиусу замедления врагов", lvl * 20),
        PassiveType::CorrosiveFlask => format!("+{}% к пробитию брони для всех атак", lvl * 15),
        PassiveType::CrimsonHeartstone => format!("+{}% к общему урону, +{:.1} HP/сек регенерации", lvl * 12, lvl as f32 * 2.5),
        PassiveType::EagleScope => format!("+{}% к скорости и дальности полета снарядов", lvl * 20),
        PassiveType::ChronoHourglass => format!("+{}% к длительности заморозки, горения и яда", lvl * 25),
        PassiveType::MagnetSphere => format!("+{} px к радиусу сбора кристаллов душ", lvl * 45),
        PassiveType::TeslaCoil => format!("+{}% шанс цепной молнии на 3 цели", lvl * 12),
        PassiveType::FortuneCoin => format!("+{}% шанс критического удара (х2 урон)", lvl * 12),
        PassiveType::PrismaticFocus => format!("+{}% к площади поражения (AoE)", lvl * 15),
        PassiveType::TitanAegis => format!("-{:.1} плоского урона от каждого удара", lvl as f32 * 2.5),
        PassiveType::RevivalPhylactery => "Воскрешение с 100% HP и отбрасывающей волной".to_string(),
        PassiveType::ElderMageShard => format!("+{} рикошет, -{}% кд, +{}% AoE", lvl, lvl * 15, lvl * 20),
        PassiveType::DoomedBlade => format!("+{}% наносимого урона, +{}% входящего", lvl * 60, lvl * 15),
        PassiveType::OccultPrism => "6% вампиризма от любого урона заклинаний".to_string(),
        PassiveType::SolarRelic => format!("{} святого урона/сек в радиусе 220px, -20% урона мобов", lvl * 40),
        PassiveType::StaticCore => format!("{}% шанс цепной молнии на 6 целей", lvl * 25),
        PassiveType::ColossalEye => format!("+{}% урона, +{}% размер снарядов, сквозное пробивание", lvl * 100, lvl * 50),
        PassiveType::BlessedHeart => format!("+{} HP, +{}% урона, самонаведение всех снарядов", lvl * 150, lvl * 25),
        PassiveType::Pyromaniac => format!("{}% исцеления от огненного урона и взрывов", lvl * 35),
        PassiveType::SoulHarvester => format!("{}% шанс вызова мстительных призраков при убийстве", lvl * 20),
        PassiveType::DivinityAura => format!("{} святого урона/сек вокруг летящих снарядов", lvl * 35),
        PassiveType::UkuleleResonance => format!("{}% шанс электро-цепочки на 4 цели", lvl * 25),
        PassiveType::FateWeaver => "Все вероятностные проверки бросаются дважды".to_string(),
        PassiveType::CalamityMortar => "Прямые попадания вызывают взрывы 55px (50% урона)".to_string(),
        PassiveType::SporeMushroom => format!("+{:.0} HP/сек (+{:.1}% макс HP) при остановке", lvl as f32 * 18.0, lvl as f32 * 1.5),
        PassiveType::TwinPrism => format!("+{} дополнительных снаряда ко всем атакам", lvl),
        PassiveType::AbyssalBrazier => format!("+{}% к радиусу всех взрывов, аур и луж", lvl * 20),
        PassiveType::AbyssalCurseStone => format!("+{}% к опыту босса, +{}% к общему урону", lvl * 35, lvl * 25),
        PassiveType::MoonDew => format!("+{:.1} HP/сек регенерации, +{}% сопротивления стихиям", lvl as f32 * 3.0, lvl * 15),
        PassiveType::GoldenSeal => format!("+{}% урона, +{}% крит, +{}px радиус сбора душ", lvl * 20, lvl * 10, lvl * 35),
        PassiveType::TitanBloodArmor => format!("+{:.0} к броне, +{}% урона за каждые 10% потерянного HP", lvl as f32 * 12.0, lvl * 8),
        PassiveType::FrenzyIchor => format!("До +{}% темпа атаки и +{:.1} HP/с регенерации при потере HP", lvl * 20, lvl as f32 * 2.0),
        PassiveType::MartyrVanguard => format!("До +{}% скорости бега от раненых союзников, +{:.1} HP за убийство моба", lvl * 15, lvl as f32 * 1.5),
    }
}
