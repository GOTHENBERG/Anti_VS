use macroquad::prelude::*;
use crate::ecs::components::inventory::WeaponType;
use crate::ecs::world::World;
use crate::render::font::GameFonts;
use crate::render::sprites::{
    draw_blood_altar, draw_boss, draw_environment, draw_mob_sprite, draw_projectile_sprite,
};
use crate::render::textures::GameTextures;

pub fn render_world(world: &World, _fonts: &GameFonts, textures: &GameTextures, cam_pos: Vec2, cam_zoom: f32) {
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

    // 3. Разрушаемые структуры (Алтари Крови)
    for structure in &world.structures {
        if !in_view(structure.pos, 100.0) {
            continue;
        }
        draw_blood_altar(structure.pos, structure.health.current, structure.health.max, time, textures);
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
        let blade_count = 8;
        let radius = 78.0;
        for i in 0..blade_count {
            let angle = world.blade_storm_angle + (i as f32) * std::f32::consts::PI * 2.0 / (blade_count as f32);
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

    // Эволюция: Праведный Щит (DivineShield) — вращающиеся священные щиты
    if world.boss_inventory.has_weapon(WeaponType::DivineShield) {
        let shield_count = 3;
        let radius = 98.0;
        for i in 0..shield_count {
            let angle = world.divine_shield_angle + (i as f32) * std::f32::consts::PI * 2.0 / (shield_count as f32);
            let s_pos = Vec2::new(bx + angle.cos() * radius, by + angle.sin() * radius);

            if let Some(s_tex) = textures.icon_shield.as_ref() {
                draw_texture_ex(
                    s_tex,
                    s_pos.x - 16.0,
                    s_pos.y - 16.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(32.0, 32.0)),
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

    // Тело и экипировка Босса
    draw_boss(
        world.boss_pos,
        world.boss_vel,
        world.boss_invulnerable_timer > 0.0,
        time,
        textures,
    );

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
