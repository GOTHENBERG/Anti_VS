use macroquad::prelude::*;
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::world::World;
use crate::render::font::GameFonts;
use crate::render::sprites::{
    draw_blood_altar, draw_boss, draw_environment, draw_mob_sprite, draw_obelisk,
    draw_projectile_sprite, draw_wall,
};
use crate::render::textures::GameTextures;

pub fn render_world(world: &World, fonts: &GameFonts, textures: &GameTextures, cam_pos: Vec2, cam_zoom: f32) {
    let time = get_time() as f32;

    // Расчет границ видимой области экрана (Viewport Frustum Culling)
    let screen_w = screen_width();
    let screen_h = screen_height();
    let zoom_clamped = cam_zoom.max(0.15);
    let half_w = (screen_w / (2.0 * zoom_clamped)) + 90.0;
    let half_h = (screen_h / (2.0 * zoom_clamped)) + 90.0;
    let view_min_x = cam_pos.x - half_w;
    let view_max_x = cam_pos.x + half_w;
    let view_min_y = cam_pos.y - half_h;
    let view_max_y = cam_pos.y + half_h;

    let in_view = |pos: Vec2, radius: f32| -> bool {
        pos.x + radius >= view_min_x
            && pos.x - radius <= view_max_x
            && pos.y + radius >= view_min_y
            && pos.y - radius <= view_max_y
    };

    // 1. Отрисовка большой арены (плиты, разломы, колонны)
    draw_environment(crate::core::MAP_WIDTH, crate::core::MAP_HEIGHT, textures);

    // 2. Кислотные лужи (Poison Pools) — мягкий полупрозрачный токсичный туман
    for pool in &world.acid_pools {
        if !in_view(pool.pos, pool.radius) {
            continue;
        }
        let alpha = (pool.lifetime / 4.0).clamp(0.0, 1.0);
        let (color, border_color) = if pool.is_director_source {
            // Чумные споры Директора: ядовито-фиолетовая слизь (ранит босса)
            (
                Color::new(0.55, 0.10, 0.70, alpha * 0.22),
                Color::new(0.85, 0.25, 0.95, alpha * 0.60),
            )
        } else if pool.corrosive {
            // Разъедающая кислота босса: неоново-зеленая
            (
                Color::new(0.12, 0.75, 0.20, alpha * 0.24),
                Color::new(0.35, 0.95, 0.35, alpha * 0.65),
            )
        } else {
            (
                Color::new(0.20, 0.70, 0.15, alpha * 0.20),
                Color::new(0.40, 0.90, 0.25, alpha * 0.55),
            )
        };
        // Мягкое полупрозрачное основание лужи (не перекрывает обзор поля боя)
        draw_circle(pool.pos.x, pool.pos.y, pool.radius, color);
        draw_circle_lines(pool.pos.x, pool.pos.y, pool.radius, 1.5, border_color);

        // Внутренняя рябь / концентрическое кольцо испарения
        let ripple_r = pool.radius * 0.6 + (time * 3.5 + pool.pos.x * 0.1).sin() * 5.0;
        if ripple_r > 5.0 {
            draw_circle_lines(pool.pos.x, pool.pos.y, ripple_r, 1.0, Color::new(border_color.r, border_color.g, border_color.b, alpha * 0.35));
        }

        // Анимированные токсичные пузыри
        let b_angle = (time * 5.0 + pool.pos.x).sin() * 0.6 * pool.radius;
        let b_y = (time * 4.0 + pool.pos.y).cos() * 0.4 * pool.radius;
        draw_circle(pool.pos.x + b_angle, pool.pos.y + b_y, 3.0, border_color);
        draw_circle(pool.pos.x - b_angle * 0.7, pool.pos.y - b_y * 0.7, 2.0, border_color);
    }

    // 2.1 Инфернальные маркеры движения мыши (RTS Click Indicator) и очередь пути
    draw_move_click_effects(world);

    // 3. Разрушаемые структуры (Алтари Крови, Стены, Обелиски)
    let pestilence_lvl = world.directors.first().map_or(0, |d| d.grade_pestilence);
    for structure in &world.structures {
        let view_pad = if structure.structure_type == crate::content::structures::STRUCTURE_OBELISK { 220.0 } else { 180.0 };
        if !in_view(structure.pos, view_pad) {
            continue;
        }

        match structure.structure_type {
            altar_type if crate::content::structures::is_altar_structure_type(altar_type) => {
                draw_circle_lines(
                    structure.pos.x,
                    structure.pos.y,
                    structure.radius,
                    1.8,
                    Color::new(1.0, 0.78, 0.35, 0.72),
                );
                draw_circle_lines(
                    structure.pos.x,
                    structure.pos.y,
                    crate::content::structures::ALTAR_MIN_SPACING,
                    1.0,
                    Color::new(1.0, 0.78, 0.35, 0.18),
                );
            }
            crate::content::structures::STRUCTURE_WALL => {
                let half = crate::content::structures::WALL_RADIUS;
                draw_rectangle_lines(
                    structure.pos.x - half,
                    structure.pos.y - half,
                    half * 2.0,
                    half * 2.0,
                    1.5,
                    Color::new(0.6, 0.8, 1.0, 0.8),
                );
            }
            crate::content::structures::STRUCTURE_OBELISK => {
                let half = crate::content::structures::OBELISK_FOOTPRINT_HALF_EXTENT;
                let border = if structure.is_active {
                    Color::new(1.0, 0.2, 0.9, 0.95)
                } else {
                    Color::new(0.72, 0.58, 0.32, 0.55)
                };
                draw_rectangle_lines(
                    structure.pos.x - half,
                    structure.pos.y - half,
                    half * 2.0,
                    half * 2.0,
                    1.5,
                    border,
                );
                draw_circle_lines(
                    structure.pos.x,
                    structure.pos.y,
                    crate::content::structures::OBELISK_RADIUS,
                    1.4,
                    Color::new(border.r, border.g, border.b, 0.72),
                );
            }
            _ => {}
        }

        match structure.structure_type {
            altar_type if crate::content::structures::is_altar_structure_type(altar_type) => {
                if pestilence_lvl > 0 {
                    let miasma_radius = 160.0f32;
                    let pulse = (time * 2.5).sin() * 6.0;
                    let r = miasma_radius + pulse;
                    let alpha = 0.12 + (time * 2.0).sin().abs() * 0.06;
                    draw_circle(structure.pos.x, structure.pos.y, r, Color::new(0.55, 0.10, 0.75, alpha));
                    draw_circle_lines(structure.pos.x, structure.pos.y, r, 1.8, Color::new(0.75, 0.20, 0.95, alpha * 2.0));
                }
                let altar_tier = crate::content::structures::altar_tier_from_structure_type(structure.structure_type).unwrap_or(1);
                draw_blood_altar(structure.pos, structure.health.current, structure.health.max, altar_tier, time, textures);
                fonts.draw_heading(
                    &format!("T{}", altar_tier),
                    structure.pos.x - 10.0,
                    structure.pos.y - 54.0,
                    11.0,
                    GOLD,
                );
            }
            crate::content::structures::STRUCTURE_WALL => {
                draw_wall(structure.pos, structure.health.current, structure.health.max, structure.lifetime, structure.max_lifetime, textures);
            }
            crate::content::structures::STRUCTURE_OBELISK => {
                draw_obelisk(structure.pos, structure.health.current, structure.health.max, structure.is_active, structure.state_timer, time, textures, fonts);
            }
            _ => {}
        }
    }

    // 4. Кристаллы душ (Soul Gems & Magnet Orbs)
    for gem in &world.gems {
        if !in_view(gem.pos, 35.0) {
            continue;
        }
        if gem.is_magnet {
            if let Some(chest_tex) = textures.chest.as_ref() {
                let pulse = (time * 6.0).sin() * 2.5;
                let size = 26.0 + pulse;
                draw_texture_ex(
                    chest_tex,
                    gem.pos.x - size / 2.0,
                    gem.pos.y - size / 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(size, size)),
                        ..Default::default()
                    },
                );
            } else {
                let pulse = (time * 6.0).sin() * 2.5;
                draw_circle(gem.pos.x, gem.pos.y, 9.0 + pulse, GOLD);
                draw_circle_lines(gem.pos.x, gem.pos.y, 14.0 + pulse, 1.5, SKYBLUE);
                draw_circle(gem.pos.x, gem.pos.y, 4.0, WHITE);
            }
        } else if let Some(c_tex) = textures.soul_crystal.as_ref() {
            let (tint, scale, aura_color) = if gem.xp_value >= 1000.0 {
                (Color::new(1.0, 0.85, 0.2, 1.0), 2.2, Color::new(1.0, 0.75, 0.1, 0.6))
            } else if gem.xp_value >= 300.0 {
                (Color::new(1.0, 0.45, 0.2, 1.0), 1.8, Color::new(1.0, 0.35, 0.1, 0.5))
            } else if gem.xp_value >= 100.0 {
                (Color::new(0.85, 0.45, 1.0, 1.0), 1.5, Color::new(0.65, 0.25, 0.9, 0.4))
            } else if gem.xp_value >= 30.0 {
                (Color::new(0.4, 1.0, 0.5, 1.0), 1.25, Color::new(0.2, 0.8, 0.3, 0.35))
            } else {
                (Color::new(0.7, 0.9, 1.0, 1.0), 1.0, Color::new(0.2, 0.6, 1.0, 0.25))
            };
            let size = 18.0 * scale;
            let bob = (time * 4.0 + (gem.pos.x * 0.1)).sin() * 2.0;

            // Мягкое эфирное свечение под кристаллом души
            draw_ellipse(gem.pos.x, gem.pos.y + 6.0 * scale, 7.0 * scale, 3.2 * scale, 0.0, aura_color);

            // Если кристалл гигантский (>= 300 XP), рисуем сияющую пульсирующую корону/звезду вокруг него
            if gem.xp_value >= 300.0 {
                let pulse = (time * 6.0).sin() * 4.0;
                draw_circle_lines(gem.pos.x, gem.pos.y + bob, (size * 0.75) + pulse, 1.5, aura_color);
            }

            draw_texture_ex(
                c_tex,
                gem.pos.x - size / 2.0,
                gem.pos.y - size / 2.0 + bob,
                tint,
                DrawTextureParams {
                    dest_size: Some(Vec2::new(size, size)),
                    ..Default::default()
                },
            );
        } else {
            let size = if gem.xp_value > 50.0 { 5.5 } else { 4.0 };
            let (color, border) = if gem.xp_value > 100.0 {
                (GOLD, YELLOW)
            } else if gem.xp_value > 30.0 {
                (GREEN, LIME)
            } else {
                (SKYBLUE, Color::new(0.3, 0.9, 1.0, 1.0))
            };
            draw_circle(gem.pos.x, gem.pos.y, size, color);
            draw_circle_lines(gem.pos.x, gem.pos.y, size, 1.0, border);
        }
    }

    // 5. Отрисовка мобов со спрайтами и джазовым танцем
    for mob in &world.mobs {
        if !in_view(mob.pos, mob.radius + 15.0) {
            continue;
        }
        draw_mob_sprite(mob, time, textures, world.boss_pos);

        // Наложение статусных эффектов (заморозка, горение, яд)
        if mob.status.is_frozen() {
            draw_circle(mob.pos.x, mob.pos.y, mob.radius + 3.0, Color::new(0.5, 0.85, 1.0, 0.4));
            draw_circle_lines(mob.pos.x, mob.pos.y, mob.radius + 3.0, 1.5, WHITE);
        } else if mob.status.burn_timer > 0.0 {
            let flame_pulse = (time * 15.0 + (mob.id as f32)).sin() * 2.0;
            draw_circle_lines(mob.pos.x, mob.pos.y, mob.radius + flame_pulse, 1.5, ORANGE);
        } else if mob.status.poison_timer > 0.0 {
            draw_circle_lines(mob.pos.x, mob.pos.y, mob.radius + 1.0, 1.5, LIME);
        }
    }

    // 6. Отрисовка снарядов (с обновленными эффектами для босса и мобов)
    for proj in &world.projectiles {
        if !in_view(proj.pos, proj.radius + 15.0) {
            continue;
        }
        draw_projectile_sprite(proj, textures);
    }

    // 7. Луч Смерти (Death Ray)
    if let Some((start, end, timer)) = world.death_ray_active {
        let alpha = (timer / 0.35).clamp(0.0, 1.0);
        let ray_color = Color::new(0.4, 0.8, 1.0, alpha);
        let core_color = Color::new(1.0, 1.0, 1.0, alpha);

        draw_line(start.x, start.y, end.x, end.y, 22.0, ray_color);
        draw_line(start.x, start.y, end.x, end.y, 8.0, core_color);
    }

    // 7.1 Дуги цепных молний (TeslaCoil, StaticCore, UkuleleResonance, ChainLightning, StormGodWrath)
    for &(start, end, timer) in &world.lightning_arcs {
        let alpha = (timer / 0.22).clamp(0.0, 1.0);
        draw_line(start.x, start.y, end.x, end.y, 4.5, Color::new(0.3, 0.8, 1.0, alpha * 0.75));
        draw_line(start.x, start.y, end.x, end.y, 2.0, Color::new(1.0, 1.0, 1.0, alpha));
    }

    // 8. Отрисовка Босса (Соло-героя)
    let bx = world.boss_pos.x;
    let by = world.boss_pos.y;

    // Аура медитации (если отдыхает и восстанавливается)
    if world.boss_still_time >= 1.2 {
        let pulse = (time * 5.0).sin() * 6.0;
        draw_circle_lines(bx, by, 45.0 + pulse, 3.5, Color::new(0.2, 1.0, 0.4, 0.95));
        draw_circle_lines(bx, by, 65.0 + pulse * 1.5, 2.0, Color::new(0.3, 1.0, 0.5, 0.6));
        draw_circle(bx, by, 35.0, Color::new(0.1, 0.9, 0.3, 0.12));
    }

    // Аура Спорового Мицелия (Исцеляющие грибы под ногами босса при остановке)
    if world.boss_mushroom_still_time >= 0.4 {
        let pulse = (time * 4.0).sin() * 5.0;
        let ring_radius = 52.0 + pulse;

        // Биолюминесцентная грибная зона
        draw_circle(bx, by, ring_radius, Color::new(0.08, 0.85, 0.45, 0.16));
        draw_circle_lines(bx, by, ring_radius, 2.0, Color::new(0.2, 1.0, 0.6, 0.80));

        // 6 светящихся грибочков по периметру мицелия
        let shroom_count = 6;
        for i in 0..shroom_count {
            let angle = (i as f32) * std::f32::consts::PI * 2.0 / (shroom_count as f32) + 0.35;
            let sx = bx + angle.cos() * (ring_radius - 10.0);
            let sy = by + angle.sin() * (ring_radius - 10.0);

            // Ножка гриба
            draw_line(sx, sy + 5.0, sx, sy - 1.0, 2.5, Color::new(0.90, 0.95, 0.85, 0.95));
            // Шляпка гриба (бирюзово-изумрудная)
            let cap_pulse = (time * 6.0 + i as f32).sin() * 1.5;
            draw_circle(sx, sy - 3.0, 5.5 + cap_pulse, Color::new(0.12, 0.95, 0.55, 0.95));
            draw_circle(sx, sy - 3.0, 2.5, Color::new(0.85, 1.0, 0.90, 1.0));
        }
    }

    // Способность: Огненная Аура / Адское Пекло (Fire Aura & Hellfire Inferno)
    let has_fire_aura = world.boss_inventory.has_weapon(WeaponType::FireAura);
    let has_hellfire = world.boss_inventory.has_weapon(WeaponType::Hellfire);
    if has_fire_aura || has_hellfire {
        let aura_radius = if has_hellfire { 150.0 } else { 110.0 };
        let flame_count = if has_hellfire { 12 } else { 8 };
        let pulse = (time * 6.0).sin() * 4.0;

        // Пылающее поле под ногами
        draw_circle(bx, by, aura_radius + pulse, Color::new(1.0, 0.25, 0.05, 0.15));
        draw_circle_lines(bx, by, aura_radius + pulse, 2.5, Color::new(1.0, 0.5, 0.1, 0.75));

        // Вращающиеся языки пламени по периметру
        for i in 0..flame_count {
            let angle = time * 3.5 + (i as f32) * std::f32::consts::PI * 2.0 / (flame_count as f32);
            let fx = bx + angle.cos() * (aura_radius - 8.0);
            let fy = by + angle.sin() * (aura_radius - 8.0);
            let f_pulse = (time * 12.0 + (i as f32)).sin() * 3.0;

            draw_circle(fx, fy, 7.0 + f_pulse, Color::new(1.0, 0.3, 0.05, 0.9));
            draw_circle(fx, fy, 4.0, Color::new(1.0, 0.85, 0.2, 1.0));
        }
    }

    // Способность: Ледниковая Буря (Blizzard Storm)
    if world.boss_inventory.has_weapon(WeaponType::BlizzardStorm) {
        let blizzard_radius = 160.0;
        let pulse = (time * 4.0).sin() * 5.0;
        draw_circle(bx, by, blizzard_radius + pulse, Color::new(0.2, 0.65, 1.0, 0.12));
        draw_circle_lines(bx, by, blizzard_radius + pulse, 2.0, Color::new(0.5, 0.85, 1.0, 0.7));

        for i in 0..8 {
            let angle = -time * 2.8 + (i as f32) * std::f32::consts::PI / 4.0;
            let sx = bx + angle.cos() * (blizzard_radius * 0.75);
            let sy = by + angle.sin() * (blizzard_radius * 0.75);
            draw_circle(sx, sy, 5.0, Color::new(0.8, 0.95, 1.0, 0.9));
            draw_circle_lines(sx, sy, 8.0, 1.0, WHITE);
        }
    }

    // Эволюция: Буря Клинков (BladeStorm) — вращающийся вихрь клинков
    if world.boss_inventory.has_weapon(WeaponType::BladeStorm) {
        let blade_count = (8 + world.boss_inventory.get_duplicator_projectiles()).min(16) as usize;
        let radius = 78.0;
        for i in 0..blade_count {
            let angle = (time * 5.5) + (i as f32) * std::f32::consts::TAU / (blade_count as f32);
            let b_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);

            if let Some(w_tex) = textures.proj_wind.as_ref() {
                draw_texture_ex(
                    w_tex,
                    b_pos.x - 12.0,
                    b_pos.y - 12.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(24.0, 24.0)),
                        rotation: angle + std::f32::consts::PI / 2.0,
                        pivot: Some(b_pos),
                        ..Default::default()
                    },
                );
            } else {
                draw_circle(b_pos.x, b_pos.y, 7.0, SKYBLUE);
                draw_circle_lines(b_pos.x, b_pos.y, 7.0, 2.0, WHITE);
                let tip = b_pos + Vec2::new(angle.cos(), angle.sin()) * 12.0;
                draw_line(b_pos.x, b_pos.y, tip.x, tip.y, 3.5, WHITE);
            }
        }
    }

    // Святой Молот (HolyHammer) — вращающиеся освященные молоты
    if world.boss_inventory.has_weapon(WeaponType::HolyHammer) {
        let extra_proj = world.boss_inventory.get_weapon(WeaponType::HolyHammer).map(|w| w.bonus_projectiles as usize).unwrap_or(0);
        let hammer_count = (2 + extra_proj + world.boss_inventory.get_duplicator_projectiles()).clamp(2, 6);
        let radius = 86.0;
        for i in 0..hammer_count {
            let angle = (time * 3.8) + (i as f32) * std::f32::consts::TAU / (hammer_count as f32);
            let h_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);

            if let Some(h_tex) = textures.icon_holy_hammer.as_ref().or(textures.proj_holy.as_ref()) {
                draw_texture_ex(
                    h_tex,
                    h_pos.x - 14.0,
                    h_pos.y - 14.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(28.0, 28.0)),
                        rotation: angle + std::f32::consts::PI / 4.0 + time * 6.0,
                        pivot: Some(h_pos),
                        ..Default::default()
                    },
                );
            } else {
                draw_circle(h_pos.x, h_pos.y, 9.0, GOLD);
                draw_circle_lines(h_pos.x, h_pos.y, 9.0, 2.0, WHITE);
                let head = h_pos + Vec2::new(angle.cos(), angle.sin()) * 8.0;
                draw_rectangle(head.x - 6.0, head.y - 4.0, 12.0, 8.0, GOLD);
            }
        }
    }

    // Эволюция: Праведный Щит (DivineShield) — вращающиеся священные щиты
    if world.boss_inventory.has_weapon(WeaponType::DivineShield) {
        let shield_count = (3 + world.boss_inventory.get_duplicator_projectiles()).min(6) as usize;
        let radius = 98.0;
        for i in 0..shield_count {
            let angle = (time * 3.5) + (i as f32) * std::f32::consts::TAU / (shield_count as f32);
            let s_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);

            if let Some(s_tex) = textures.icon_evo_divine_shield.as_ref().or(textures.icon_shield.as_ref()) {
                draw_texture_ex(
                    s_tex,
                    s_pos.x - 16.0,
                    s_pos.y - 16.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(32.0, 32.0)),
                        rotation: angle + std::f32::consts::PI / 2.0,
                        pivot: Some(s_pos),
                        ..Default::default()
                    },
                );
            } else {
                draw_circle(s_pos.x, s_pos.y, 12.0, GOLD);
                draw_circle_lines(s_pos.x, s_pos.y, 12.0, 2.5, WHITE);
                draw_line(s_pos.x - 6.0, s_pos.y, s_pos.x + 6.0, s_pos.y, 2.0, WHITE);
                draw_line(s_pos.x, s_pos.y - 6.0, s_pos.x, s_pos.y + 6.0, 2.0, WHITE);
            }
        }
    }

    // Орбитальные Бастионы (AegisOrbiters) — защитные кристаллы-барьеры
    if world.boss_inventory.has_weapon(WeaponType::AegisOrbiters) {
        let bastion_count = (3 + world.boss_inventory.get_duplicator_projectiles()).clamp(3, 7) as usize;
        let radius = 106.0;
        for i in 0..bastion_count {
            let angle = (-time * 2.8) + (i as f32) * std::f32::consts::TAU / (bastion_count as f32);
            let a_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);

            if let Some(a_tex) = textures.icon_aegis_orbiters.as_ref().or(textures.icon_aegis.as_ref()) {
                draw_texture_ex(
                    a_tex,
                    a_pos.x - 15.0,
                    a_pos.y - 15.0,
                    Color::new(0.4, 0.95, 1.0, 0.95),
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(30.0, 30.0)),
                        rotation: angle,
                        pivot: Some(a_pos),
                        ..Default::default()
                    },
                );
            } else {
                draw_circle(a_pos.x, a_pos.y, 10.0, Color::new(0.2, 0.85, 1.0, 0.85));
                draw_circle_lines(a_pos.x, a_pos.y, 10.0, 2.0, WHITE);
            }
        }
    }

    // Эволюция: Несокрушимая Крепость (InvincibleBastion) — сверхпрочное силовое поле и крепости
    if world.boss_inventory.has_weapon(WeaponType::InvincibleBastion) {
        let bastion_count = (4 + world.boss_inventory.get_duplicator_projectiles()).clamp(4, 8) as usize;
        let radius = 125.0;
        let pulse = (time * 4.0).sin() * 3.0;

        // Силовое кольцо между бастионами
        draw_circle_lines(bx, by, radius + pulse, 2.5, Color::new(0.3, 0.9, 1.0, 0.65));
        draw_circle(bx, by, radius + pulse, Color::new(0.1, 0.6, 1.0, 0.08));

        for i in 0..bastion_count {
            let angle = (-time * 3.4) + (i as f32) * std::f32::consts::TAU / (bastion_count as f32);
            let a_pos = Vec2::new(bx + angle.cos() * (radius + pulse), by + angle.sin() * (radius + pulse));

            if let Some(a_tex) = textures.icon_aegis_orbiters.as_ref().or(textures.icon_aegis.as_ref()) {
                draw_texture_ex(
                    a_tex,
                    a_pos.x - 18.0,
                    a_pos.y - 18.0,
                    GOLD,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(36.0, 36.0)),
                        rotation: angle,
                        pivot: Some(a_pos),
                        ..Default::default()
                    },
                );
            } else {
                draw_circle(a_pos.x, a_pos.y, 13.0, GOLD);
                draw_circle_lines(a_pos.x, a_pos.y, 13.0, 3.0, WHITE);
            }
        }
    }

    // Орбитальный Гримуар / Инфернальная Литургия: вращающиеся книги/скрижали
    let has_scriptures = world.boss_inventory.has_weapon(WeaponType::SacredScriptures);
    let has_liturgy = world.boss_inventory.has_weapon(WeaponType::InfernalLiturgy);
    if has_scriptures || has_liturgy {
        let book_count = (if has_liturgy { 4 } else { 2 }) + world.boss_inventory.get_duplicator_projectiles() as usize;
        let radius = if has_liturgy { 114.0 } else { 90.0 };
        let book_col = if has_liturgy { ORANGE } else { GOLD };
        let speed = if has_liturgy { 5.0 } else { 4.2 };
        for i in 0..book_count {
            let angle = (time * speed) + (i as f32) * std::f32::consts::TAU / (book_count as f32);
            let p_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);
            if let Some(b_tex) = textures.icon_cooldown_tome.as_ref().or(textures.icon_book.as_ref()) {
                draw_texture_ex(
                    b_tex,
                    p_pos.x - 14.0,
                    p_pos.y - 14.0,
                    book_col,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(28.0, 28.0)),
                        rotation: angle + std::f32::consts::PI / 2.0,
                        pivot: Some(p_pos),
                        ..Default::default()
                    },
                );
            } else {
                draw_circle(p_pos.x, p_pos.y, 8.0, book_col);
                draw_circle_lines(p_pos.x, p_pos.y, 8.0, 2.0, WHITE);
            }
        }
    }

    // Защитный барьер Астрального Савана / Савана Кровавого Императора
    if world.boss_inventory.astral_cloak_charges > 0 {
        let charges = world.boss_inventory.astral_cloak_charges;
        let is_shroud = world.boss_inventory.has_weapon(WeaponType::BloodEmperorShroud);
        let ring_col = if is_shroud {
            Color::new(0.9, 0.15, 0.25, 0.75 + (time * 4.0).sin() * 0.2)
        } else {
            Color::new(0.3, 0.8, 1.0, 0.7 + (time * 4.0).sin() * 0.2)
        };
        let barrier_rad = 36.0 + (charges as f32 * 4.0);
        draw_circle_lines(bx, by, barrier_rad, 2.5, ring_col);
        draw_circle_lines(bx, by, barrier_rad + 4.0, 1.0, Color::new(ring_col.r, ring_col.g, ring_col.b, 0.35));
    }

    // Тело и экипировка Босса
    if world.boss_health.is_alive() {
        draw_boss(
            world.boss_pos,
            world.boss_vel,
            world.boss_facing,
            world.boss_invulnerable_timer > 0.0,
            world.boss_skin,
            time,
            textures,
        );
        draw_boss_name(&world.boss_name, world.boss_pos, fonts, Color::new(0.95, 0.88, 0.68, 0.96));
    } else {
        draw_downed_boss(
            world.boss_pos,
            world.boss_revive_progress,
            &world.boss_name,
            textures,
            fonts,
        );
    }

    for player in &world.coop_bosses {
        if !in_view(player.boss_pos, 72.0) {
            continue;
        }
        if player.boss_health.is_alive() {
            draw_boss(
                player.boss_pos,
                player.boss_vel,
                player.boss_facing,
                player.boss_invulnerable_timer > 0.0,
                player.boss_skin,
                time,
                textures,
            );
            draw_boss_name(&player.boss_name, player.boss_pos, fonts, Color::new(0.55, 0.85, 1.0, 0.98));
            let hp_ratio = (player.boss_health.current / player.boss_health.max.max(1.0)).clamp(0.0, 1.0);
            draw_rectangle(player.boss_pos.x - 24.0, player.boss_pos.y + 35.0, 48.0, 4.0, DARKGRAY);
            draw_rectangle(player.boss_pos.x - 24.0, player.boss_pos.y + 35.0, 48.0 * hp_ratio, 4.0, SKYBLUE);
        } else {
            draw_downed_boss(
                player.boss_pos,
                player.revive_progress,
                &player.boss_name,
                textures,
                fonts,
            );
        }
    }

    // 9. Отрисовка частиц
    for p in &world.particles {
        if !in_view(p.pos, p.radius + 5.0) {
            continue;
        }
        let alpha = (p.lifetime / p.max_lifetime).clamp(0.0, 1.0);
        let mut color = p.color;
        color.a = alpha;
        draw_circle(p.pos.x, p.pos.y, p.radius, color);
    }
}

fn draw_boss_name(name: &str, pos: Vec2, fonts: &GameFonts, color: Color) {
    let name_width = fonts.measure_heading(name, 13.0).width;
    fonts.draw_heading(name, pos.x - name_width * 0.5, pos.y - 49.0, 13.0, color);
}

fn draw_downed_boss(pos: Vec2, revive_progress: f32, name: &str, textures: &GameTextures, fonts: &GameFonts) {
    let radius = crate::ecs::systems::coop::REVIVE_RADIUS;
    draw_circle(pos.x, pos.y, radius, Color::new(0.12, 0.55, 0.85, 0.12));
    draw_circle_lines(pos.x, pos.y, radius, 2.0, Color::new(0.35, 0.8, 1.0, 0.85));

    let progress = (revive_progress / crate::ecs::systems::coop::REVIVE_SECONDS).clamp(0.0, 1.0);
    let segments = (progress * 36.0).ceil() as usize;
    for i in 0..segments {
        let a0 = (i as f32 / 36.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let a1 = ((i + 1) as f32 / 36.0) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        draw_line(
            pos.x + a0.cos() * (radius - 3.0),
            pos.y + a0.sin() * (radius - 3.0),
            pos.x + a1.cos() * (radius - 3.0),
            pos.y + a1.sin() * (radius - 3.0),
            4.0,
            SKYBLUE,
        );
    }

    if let Some(frame) = textures.knight_idle.first() {
        draw_texture_ex(
            frame,
            pos.x - 22.0,
            pos.y - 22.0,
            Color::new(0.55, 0.62, 0.68, 0.82),
            DrawTextureParams {
                dest_size: Some(Vec2::new(44.0, 44.0)),
                rotation: std::f32::consts::FRAC_PI_2,
                pivot: Some(pos),
                ..Default::default()
            },
        );
    } else {
        draw_ellipse(pos.x, pos.y + 7.0, 25.0, 9.0, 0.0, Color::new(0.22, 0.27, 0.31, 0.9));
        draw_circle(pos.x - 16.0, pos.y - 3.0, 7.0, Color::new(0.55, 0.62, 0.68, 0.85));
    }
    draw_boss_name(name, pos, fonts, SKYBLUE);
    let remaining = (crate::ecs::systems::coop::REVIVE_SECONDS - revive_progress.max(0.0)).max(0.0);
    fonts.draw_heading(&format!("ВОСКРЕШЕНИЕ {:.1}с", remaining), pos.x - 50.0, pos.y + 63.0, 10.0, SKYBLUE);
}

fn draw_move_click_effects(world: &World) {
    // 1. Отрисовка очереди маршрута (Shift-waypoint path)
    if !world.boss_waypoint_queue.is_empty() {
        let mut prev = world.boss_pos;
        for &wp in &world.boss_waypoint_queue {
            draw_line(prev.x, prev.y, wp.x, wp.y, 2.0, Color::new(0.92, 0.28, 0.16, 0.45));
            draw_circle(wp.x, wp.y, 4.0, Color::new(1.0, 0.60, 0.18, 0.75));
            draw_circle_lines(wp.x, wp.y, 6.5, 1.2, Color::new(0.92, 0.22, 0.12, 0.85));
            prev = wp;
        }
    }

    // 2. Анимированный расплывающийся инфернальный кружок на месте клика (ПКМ RTS)
    for eff in &world.move_click_effects {
        let dur = eff.duration.max(0.001);
        let progress = (1.0 - (eff.timer / dur)).clamp(0.0, 1.0); // 0.0 -> 1.0
        let fade = (eff.timer / dur).clamp(0.0, 1.0);
        let alpha = fade * fade;

        // Внешнее расширяющееся инфернальное кольцо
        let outer_r = 8.0 + progress * 24.0;
        let outer_col = Color::new(1.0, 0.32, 0.12, alpha * 0.95);
        draw_circle_lines(eff.pos.x, eff.pos.y, outer_r, 2.2, outer_col);

        // Внутреннее вторичное пульсирующее кольцо
        let inner_r = 4.0 + progress * 13.0;
        let inner_col = Color::new(0.95, 0.70, 0.20, alpha * 0.75);
        draw_circle_lines(eff.pos.x, eff.pos.y, inner_r, 1.6, inner_col);

        // 4 рунических засечки, направленные к центру (стиль Dota/Warcraft, адаптированный под Anti_VS)
        let rot = progress * 0.4;
        for i in 0..4 {
            let angle = (i as f32) * std::f32::consts::FRAC_PI_2 + rot;
            let dir = Vec2::new(angle.cos(), angle.sin());
            let p1 = eff.pos + dir * (outer_r - 3.0);
            let p2 = eff.pos + dir * (outer_r + 6.0);
            draw_line(p1.x, p1.y, p2.x, p2.y, 2.0, Color::new(1.0, 0.85, 0.30, alpha));
        }

        // Яркая точка в центре
        let dot_r = (1.0 - progress) * 3.5;
        if dot_r > 0.5 {
            draw_circle(eff.pos.x, eff.pos.y, dot_r, Color::new(1.0, 0.50, 0.15, alpha));
        }
    }
}
