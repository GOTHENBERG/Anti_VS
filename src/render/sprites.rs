use macroquad::prelude::*;
use crate::ecs::world::MobEntity;
use crate::render::textures::GameTextures;

/// Отрисовка фона большой карты: плиты, трещины, текстуры и границы
pub fn draw_environment(map_w: f32, map_h: f32, textures: &GameTextures) {
    // 1. Базовый фон
    clear_background(Color::new(0.07, 0.07, 0.10, 1.0));

    let tile_size = 64.0;
    let cols = (map_w / tile_size) as usize;
    let rows = (map_h / tile_size) as usize;

    // 2. Текстурный каменный пол подземелья
    if let Some(floor_tex) = textures.floor_stone.as_ref() {
        for y in 0..rows {
            let py = (y as f32) * tile_size;
            for x in 0..cols {
                let px = (x as f32) * tile_size;
                let hash = ((x * 73856093) ^ (y * 19349663)) % 100;
                let tint = if hash < 15 {
                    Color::new(0.70, 0.65, 0.72, 0.9)
                } else if hash > 85 {
                    Color::new(0.55, 0.52, 0.60, 0.9)
                } else {
                    Color::new(0.62, 0.60, 0.68, 0.9)
                };
                draw_texture_ex(
                    floor_tex,
                    px,
                    py,
                    tint,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(tile_size, tile_size)),
                        ..Default::default()
                    },
                );

                // Случайные кровавые руны и трещины
                if hash == 7 {
                    draw_circle_lines(px + 32.0, py + 32.0, 12.0, 1.5, Color::new(0.6, 0.08, 0.15, 0.6));
                }
            }
        }
    } else {
        // Процедурная сетка (fallback)
        let grid_color = Color::new(0.11, 0.11, 0.15, 1.0);
        let crack_color = Color::new(0.18, 0.12, 0.14, 0.6);

        for y in 0..rows {
            let py = (y as f32) * tile_size;
            draw_line(0.0, py, map_w, py, 1.0, grid_color);
            for x in 0..cols {
                let px = (x as f32) * tile_size;
                if y == 0 {
                    draw_line(px, 0.0, px, map_h, 1.0, grid_color);
                }
                let hash = ((x * 73856093) ^ (y * 19349663)) % 100;
                if hash < 8 {
                    draw_line(px + 15.0, py + 15.0, px + 55.0, py + 45.0, 1.5, crack_color);
                }
            }
        }
    }

    // 3. Кирпичные границы подземелья
    if let Some(wall_tex) = textures.wall_brick.as_ref() {
        let wall_h = 32.0;
        for x in 0..cols {
            let px = (x as f32) * tile_size;
            draw_texture_ex(wall_tex, px, 0.0, WHITE, DrawTextureParams {
                dest_size: Some(Vec2::new(tile_size, wall_h)),
                ..Default::default()
            });
            draw_texture_ex(wall_tex, px, map_h - wall_h, WHITE, DrawTextureParams {
                dest_size: Some(Vec2::new(tile_size, wall_h)),
                ..Default::default()
            });
        }
        for y in 0..rows {
            let py = (y as f32) * tile_size;
            draw_texture_ex(wall_tex, 0.0, py, WHITE, DrawTextureParams {
                dest_size: Some(Vec2::new(wall_h, tile_size)),
                ..Default::default()
            });
            draw_texture_ex(wall_tex, map_w - wall_h, py, WHITE, DrawTextureParams {
                dest_size: Some(Vec2::new(wall_h, tile_size)),
                ..Default::default()
            });
        }
    }

    // 4. Огненный защитный барьер арены
    let border_color = Color::new(0.85, 0.2, 0.15, 0.85);
    draw_rectangle_lines(0.0, 0.0, map_w, map_h, 8.0, border_color);
    draw_rectangle_lines(16.0, 16.0, map_w - 32.0, map_h - 32.0, 2.0, Color::new(0.5, 0.1, 0.1, 0.4));

    // Угловые и промежуточные обелиски арены
    let pillar_spacing = 300.0;
    let mut px = 0.0;
    while px <= map_w {
        draw_pillar(Vec2::new(px, 0.0));
        draw_pillar(Vec2::new(px, map_h));
        px += pillar_spacing;
    }
    let mut py = 0.0;
    while py <= map_h {
        draw_pillar(Vec2::new(0.0, py));
        draw_pillar(Vec2::new(map_w, py));
        py += pillar_spacing;
    }
}

fn draw_pillar(pos: Vec2) {
    draw_circle(pos.x, pos.y, 16.0, Color::new(0.2, 0.18, 0.22, 1.0));
    draw_circle_lines(pos.x, pos.y, 16.0, 2.0, Color::new(0.5, 0.1, 0.15, 1.0));
    draw_circle(pos.x, pos.y, 6.0, Color::new(0.9, 0.3, 0.1, 0.9));
}

/// Отрисовка Босса (Соло-героя: благородный Паладин в латах с плащом, мечом и анимацией)
pub fn draw_boss(pos: Vec2, vel: Vec2, invulnerable: bool, time: f32, textures: &GameTextures) {
    let facing_dir = if vel.length_squared() > 1.0 {
        vel.normalize()
    } else {
        Vec2::new(1.0, 0.0)
    };
    let normal_dir = Vec2::new(-facing_dir.y, facing_dir.x);
    let is_moving = vel.length_squared() > 4.0;
    let flip_x = facing_dir.x < -0.1;

    // 1. Тень под ногами
    draw_ellipse(pos.x, pos.y + 14.0, 22.0, 10.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.45));

    // 2. Развевающийся алый плащ за спиной
    let cape_flutter = (time * 8.0).sin() * 5.0;
    let cape_anchor = pos - facing_dir * 10.0;
    let cape_tip_1 = cape_anchor - facing_dir * 22.0 + normal_dir * (12.0 + cape_flutter);
    let cape_tip_2 = cape_anchor - facing_dir * 26.0 - normal_dir * (12.0 - cape_flutter);
    draw_triangle(cape_anchor, cape_tip_1, cape_tip_2, Color::new(0.75, 0.12, 0.15, 1.0));

    // 3. Анимированный спрайт Рыцаря (если доступен)
    let frames = if is_moving { &textures.knight_walk } else { &textures.knight_idle };
    if !frames.is_empty() {
        let frame_rate = if is_moving { 10.0 } else { 4.0 };
        let frame_idx = ((time * frame_rate) as usize) % frames.len();
        let tex = &frames[frame_idx];

        let tint = if invulnerable {
            Color::new(1.0, 1.0, 0.4, 0.95)
        } else {
            WHITE
        };

        let knight_w = 34.0;
        let knight_h = 58.0;

        draw_texture_ex(
            tex,
            pos.x - knight_w / 2.0,
            pos.y - knight_h / 2.0 - 4.0,
            tint,
            DrawTextureParams {
                dest_size: Some(Vec2::new(knight_w, knight_h)),
                flip_x,
                ..Default::default()
            },
        );
    } else {
        // Процедурные латы и шлем (fallback)
        let body_color = if invulnerable {
            Color::new(1.0, 1.0, 0.4, 0.9)
        } else {
            Color::new(0.85, 0.88, 0.92, 1.0)
        };
        draw_circle(pos.x, pos.y, 18.0, body_color);
        draw_circle_lines(pos.x, pos.y, 18.0, 2.5, Color::new(0.95, 0.75, 0.15, 1.0));
        draw_circle(pos.x + facing_dir.x * 4.0, pos.y + facing_dir.y * 4.0, 6.0, SKYBLUE);
        draw_circle(pos.x + facing_dir.x * 4.0, pos.y + facing_dir.y * 4.0, 3.0, WHITE);

        let head_pos = pos + facing_dir * 8.0;
        draw_circle(head_pos.x, head_pos.y, 9.0, Color::new(0.9, 0.9, 0.95, 1.0));
        draw_circle_lines(head_pos.x, head_pos.y, 9.0, 1.5, GOLD);
        draw_line(
            head_pos.x + normal_dir.x * 5.0,
            head_pos.y + normal_dir.y * 5.0,
            head_pos.x - normal_dir.x * 5.0,
            head_pos.y - normal_dir.y * 5.0,
            2.0,
            BLACK,
        );
    }

    // 4. Огромный сияющий двуручный меч в правой руке
    let hand_pos = pos + normal_dir * 18.0 + facing_dir * 4.0;
    let sword_tip = hand_pos + facing_dir * 32.0;
    draw_line(hand_pos.x, hand_pos.y, sword_tip.x, sword_tip.y, 4.5, WHITE);
    draw_line(hand_pos.x, hand_pos.y, sword_tip.x, sword_tip.y, 2.0, SKYBLUE);
    let hilt_left = hand_pos + normal_dir * 6.0;
    let hilt_right = hand_pos - normal_dir * 6.0;
    draw_line(hilt_left.x, hilt_left.y, hilt_right.x, hilt_right.y, 3.0, GOLD);

    // 5. Щит рыцаря в левой руке
    let shield_pos = pos - normal_dir * 16.0 + facing_dir * 2.0;
    draw_circle(shield_pos.x, shield_pos.y, 9.0, Color::new(0.2, 0.4, 0.7, 1.0));
    draw_circle_lines(shield_pos.x, shield_pos.y, 9.0, 2.0, GOLD);
}

/// Отрисовка моба (текстурные спрайты с процедурным fallback)
pub fn draw_mob_sprite(mob: &MobEntity, time: f32, textures: &GameTextures) {
    let frame_idx = (time * 8.0 + (mob.id as f32) * 0.7) as usize;
    let flip_x = mob.vel.x < -0.1;

    if let Some(tex) = textures.get_mob_texture(mob.tier, frame_idx) {
        // Тень под мобом
        draw_ellipse(
            mob.pos.x,
            mob.pos.y + mob.radius * 0.7,
            mob.radius * 1.1,
            mob.radius * 0.45,
            0.0,
            Color::new(0.0, 0.0, 0.0, 0.38),
        );

        let size = match mob.tier {
            1 => 30.0, // Скелет
            2 => 36.0, // Гончая
            3 => 32.0, // Культист
            4 => 48.0, // Ледяной голем
            5 => 30.0, // Камикадзе
            6 => 38.0, // Демон теней
            7 => 42.0, // Огненный элементаль
            8 => 62.0, // Жнец Смерти (Титан)
            _ => 28.0,
        };

        // Гончая шире (32x16)
        let (draw_w, draw_h) = if mob.tier == 2 {
            (size * 1.6, size * 0.8)
        } else {
            (size, size)
        };

        let tint = if mob.status.is_frozen() {
            Color::new(0.5, 0.85, 1.0, 1.0)
        } else if mob.status.burn_timer > 0.0 {
            Color::new(1.0, 0.6, 0.4, 1.0)
        } else if mob.status.poison_timer > 0.0 {
            Color::new(0.6, 1.0, 0.5, 1.0)
        } else {
            WHITE
        };

        // Покачивание/левитация при ходьбе
        let bob_y = if mob.tier == 1 || mob.tier == 5 {
            (time * 12.0 + (mob.id as f32)).sin() * 2.0
        } else if mob.tier == 3 {
            (time * 4.0 + (mob.id as f32)).sin() * 3.0 // Культист левитирует
        } else {
            0.0
        };

        draw_texture_ex(
            tex,
            mob.pos.x - draw_w / 2.0,
            mob.pos.y - draw_h / 2.0 + bob_y,
            tint,
            DrawTextureParams {
                dest_size: Some(Vec2::new(draw_w, draw_h)),
                flip_x,
                ..Default::default()
            },
        );

        // Полоска здоровья голема (Тир 4)
        if mob.tier == 4 {
            let hp_pct = (mob.health.current / mob.health.max).clamp(0.0, 1.0);
            draw_rectangle(mob.pos.x - 20.0, mob.pos.y - 30.0, 40.0, 5.0, DARKGRAY);
            draw_rectangle(mob.pos.x - 20.0, mob.pos.y - 30.0, 40.0 * hp_pct, 5.0, SKYBLUE);
            draw_rectangle_lines(mob.pos.x - 20.0, mob.pos.y - 30.0, 40.0, 5.0, 1.0, WHITE);
        }

        // Полоска здоровья Жнеца Смерти (Тир 8)
        if mob.tier == 8 {
            let hp_pct = (mob.health.current / mob.health.max).clamp(0.0, 1.0);
            draw_rectangle(mob.pos.x - 30.0, mob.pos.y - 42.0, 60.0, 6.0, DARKGRAY);
            draw_rectangle(mob.pos.x - 30.0, mob.pos.y - 42.0, 60.0 * hp_pct, 6.0, RED);
            draw_rectangle_lines(mob.pos.x - 30.0, mob.pos.y - 42.0, 60.0, 6.0, 1.5, GOLD);
        }

        // Элитный Чемпион (Золотая аура и корона)
        if mob.is_elite {
            let crown_y = mob.pos.y - draw_h / 2.0 - 6.0;
            let pulse = (time * 8.0 + (mob.id as f32)).sin() * 2.0;
            draw_circle_lines(mob.pos.x, mob.pos.y, mob.radius * 1.35 + pulse, 2.0, GOLD);
            draw_circle(mob.pos.x, crown_y, 4.0, GOLD);
            draw_circle(mob.pos.x - 5.0, crown_y + 1.0, 3.0, YELLOW);
            draw_circle(mob.pos.x + 5.0, crown_y + 1.0, 3.0, YELLOW);
        }

        // Искрящийся фитиль камикадзе (Тир 5)
        if mob.tier == 5 {
            let spark_pulse = (time * 30.0).sin() * 3.0;
            let spark_pos = mob.pos + Vec2::new(0.0, -draw_h / 2.0 - 2.0);
            draw_circle(spark_pos.x, spark_pos.y, 3.5 + spark_pulse, ORANGE);
            draw_circle(spark_pos.x, spark_pos.y, 1.5, YELLOW);
        }
    } else {
        // Процедурный fallback
        match mob.tier {
            1 => draw_skeleton(mob, time),
            2 => draw_hound(mob, time),
            3 => draw_cultist(mob, time),
            4 => draw_ice_golem(mob, time),
            _ => draw_kamikaze(mob, time),
        }
    }
}

/// Тир 1: Скелет-пехотинец (процедурный fallback)
fn draw_skeleton(mob: &MobEntity, time: f32) {
    let p = mob.pos;
    let dir = mob.vel.normalize_or_zero();
    let norm = Vec2::new(-dir.y, dir.x);
    let wobble = (time * 10.0 + (mob.id as f32)).sin() * 2.0;

    draw_ellipse(p.x, p.y + 10.0, 10.0, 4.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.35));
    draw_circle(p.x, p.y + 2.0, 7.0, Color::new(0.8, 0.8, 0.8, 1.0));

    let head = p + Vec2::new(wobble, -8.0);
    draw_circle(head.x, head.y, 6.0, Color::new(0.9, 0.9, 0.9, 1.0));
    draw_circle(head.x + dir.x * 2.0 - norm.x * 2.0, head.y - 1.0, 1.5, BLACK);
    draw_circle(head.x + dir.x * 2.0 + norm.x * 2.0, head.y - 1.0, 1.5, BLACK);

    let sword_base = p + norm * 8.0;
    let sword_tip = sword_base + dir * 14.0;
    draw_line(sword_base.x, sword_base.y, sword_tip.x, sword_tip.y, 2.5, Color::new(0.65, 0.45, 0.35, 1.0));

    let shield_pos = p - norm * 8.0;
    draw_circle(shield_pos.x, shield_pos.y, 6.0, Color::new(0.5, 0.35, 0.2, 1.0));
    draw_circle_lines(shield_pos.x, shield_pos.y, 6.0, 1.0, Color::new(0.7, 0.7, 0.7, 1.0));
}

/// Тир 2: Чёрная гончая (процедурный fallback)
fn draw_hound(mob: &MobEntity, time: f32) {
    let p = mob.pos;
    let dir = if mob.vel.length_squared() > 1.0 { mob.vel.normalize() } else { Vec2::X };
    let norm = Vec2::new(-dir.y, dir.x);
    let run_bob = (time * 18.0 + (mob.id as f32)).sin() * 3.0;

    draw_ellipse(p.x, p.y + 11.0, 14.0, 5.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.4));
    let body_pos = p + Vec2::new(0.0, run_bob);
    draw_ellipse(body_pos.x, body_pos.y, 13.0, 8.0, 0.0, Color::new(0.18, 0.12, 0.14, 1.0));
    draw_ellipse_lines(body_pos.x, body_pos.y, 13.0, 8.0, 0.0, 1.5, Color::new(0.35, 0.15, 0.1, 1.0));

    let head = body_pos + dir * 12.0;
    draw_circle(head.x, head.y, 6.0, Color::new(0.15, 0.1, 0.12, 1.0));
    draw_circle(head.x + norm.x * 2.5, head.y - 1.5, 1.5, RED);
    draw_circle(head.x - norm.x * 2.5, head.y - 1.5, 1.5, RED);
    draw_triangle(head, head - dir * 4.0 + norm * 5.0, head + dir * 2.0 + norm * 3.0, Color::new(0.3, 0.1, 0.1, 1.0));
    draw_triangle(head, head - dir * 4.0 - norm * 5.0, head + dir * 2.0 - norm * 3.0, Color::new(0.3, 0.1, 0.1, 1.0));
}

/// Тир 3: Культист-стрелок (процедурный fallback)
fn draw_cultist(mob: &MobEntity, time: f32) {
    let p = mob.pos;
    let float_y = (time * 4.0 + (mob.id as f32)).sin() * 2.5;

    draw_ellipse(p.x, p.y + 12.0, 12.0, 5.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.4));
    let top = p + Vec2::new(0.0, -12.0 + float_y);
    let bot_l = p + Vec2::new(-9.0, 10.0 + float_y);
    let bot_r = p + Vec2::new(9.0, 10.0 + float_y);
    draw_triangle(top, bot_l, bot_r, Color::new(0.35, 0.12, 0.45, 1.0));
    draw_triangle_lines(top, bot_l, bot_r, 1.5, Color::new(0.55, 0.2, 0.7, 1.0));

    let head = p + Vec2::new(0.0, -8.0 + float_y);
    draw_circle(head.x, head.y, 7.0, Color::new(0.2, 0.05, 0.25, 1.0));
    draw_circle(head.x - 2.5, head.y + 1.0, 1.5, Color::new(0.9, 0.5, 1.0, 1.0));
    draw_circle(head.x + 2.5, head.y + 1.0, 1.5, Color::new(0.9, 0.5, 1.0, 1.0));

    let staff_base = p + Vec2::new(12.0, 10.0 + float_y);
    let staff_top = p + Vec2::new(12.0, -18.0 + float_y);
    draw_line(staff_base.x, staff_base.y, staff_top.x, staff_top.y, 2.5, Color::new(0.4, 0.25, 0.15, 1.0));
    let pulse = (time * 8.0).sin() * 1.5;
    draw_circle(staff_top.x, staff_top.y, 4.5 + pulse, Color::new(0.8, 0.2, 0.9, 1.0));
    draw_circle(staff_top.x, staff_top.y, 2.0, WHITE);
}

/// Тир 4: Ледяной голем (процедурный fallback)
fn draw_ice_golem(mob: &MobEntity, _time: f32) {
    let p = mob.pos;
    draw_ellipse(p.x, p.y + 20.0, 24.0, 9.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.5));
    draw_circle(p.x, p.y, 20.0, Color::new(0.25, 0.65, 0.85, 1.0));
    draw_circle_lines(p.x, p.y, 20.0, 3.0, Color::new(0.7, 0.9, 1.0, 1.0));
    draw_circle(p.x - 18.0, p.y - 8.0, 10.0, Color::new(0.35, 0.75, 0.95, 1.0));
    draw_circle(p.x + 18.0, p.y - 8.0, 10.0, Color::new(0.35, 0.75, 0.95, 1.0));

    let hp_pct = (mob.health.current / mob.health.max).clamp(0.0, 1.0);
    draw_rectangle(p.x - 20.0, p.y - 28.0, 40.0, 5.0, DARKGRAY);
    draw_rectangle(p.x - 20.0, p.y - 28.0, 40.0 * hp_pct, 5.0, SKYBLUE);
    draw_rectangle_lines(p.x - 20.0, p.y - 28.0, 40.0, 5.0, 1.0, WHITE);
}

/// Тир 5: Камикадзе (процедурный fallback)
fn draw_kamikaze(mob: &MobEntity, time: f32) {
    let p = mob.pos;
    let dir = mob.vel.normalize_or_zero();
    let frenzied_bob = (time * 24.0 + (mob.id as f32)).sin() * 2.0;

    draw_ellipse(p.x, p.y + 14.0, 15.0, 5.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.4));
    let barrel_pos = p - dir * 4.0 + Vec2::new(0.0, frenzied_bob);
    draw_circle(barrel_pos.x, barrel_pos.y, 14.0, Color::new(0.85, 0.15, 0.15, 1.0));
    draw_circle_lines(barrel_pos.x, barrel_pos.y, 14.0, 2.0, Color::new(0.2, 0.2, 0.2, 1.0));

    let head = p + dir * 8.0 + Vec2::new(0.0, frenzied_bob - 2.0);
    draw_circle(head.x, head.y, 7.0, Color::new(0.85, 0.45, 0.2, 1.0));
    draw_circle(head.x - 2.5, head.y - 1.0, 2.0, YELLOW);
    draw_circle(head.x + 2.5, head.y - 1.0, 2.0, YELLOW);

    let fuse_top = barrel_pos + Vec2::new(0.0, -18.0);
    draw_line(barrel_pos.x, barrel_pos.y - 12.0, fuse_top.x, fuse_top.y, 2.0, Color::new(0.6, 0.4, 0.2, 1.0));
    let spark_pulse = (time * 30.0).sin() * 3.0;
    draw_circle(fuse_top.x, fuse_top.y, 4.0 + spark_pulse, ORANGE);
    draw_circle(fuse_top.x, fuse_top.y, 2.0, YELLOW);
}

/// Отрисовка Алтаря Крови
pub fn draw_blood_altar(pos: Vec2, current_hp: f32, max_hp: f32, time: f32, textures: &GameTextures) {
    if let Some(altar_tex) = textures.altar.as_ref() {
        draw_ellipse(pos.x, pos.y + 18.0, 28.0, 10.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.55));
        let pulse = (time * 4.0).sin() * 2.0;
        draw_circle(pos.x, pos.y + 4.0, 22.0 + pulse, Color::new(0.7, 0.05, 0.15, 0.35));

        draw_texture_ex(
            altar_tex,
            pos.x - 20.0,
            pos.y - 40.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(40.0, 64.0)),
                ..Default::default()
            },
        );

        let hp_pct = (current_hp / max_hp).clamp(0.0, 1.0);
        draw_rectangle(pos.x - 25.0, pos.y - 48.0, 50.0, 6.0, DARKGRAY);
        draw_rectangle(pos.x - 25.0, pos.y - 48.0, 50.0 * hp_pct, 6.0, RED);
        draw_rectangle_lines(pos.x - 25.0, pos.y - 48.0, 50.0, 6.0, 1.5, WHITE);
    } else {
        draw_ellipse(pos.x, pos.y + 20.0, 38.0, 16.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.5));
        draw_circle(pos.x, pos.y, 28.0, Color::new(0.18, 0.12, 0.22, 1.0));
        draw_circle_lines(pos.x, pos.y, 28.0, 3.0, Color::new(0.5, 0.1, 0.65, 1.0));

        let blood_pulse = (time * 4.0).sin() * 2.0;
        draw_circle(pos.x, pos.y, 18.0 + blood_pulse, Color::new(0.7, 0.05, 0.15, 1.0));
        draw_circle(pos.x, pos.y, 8.0, Color::new(0.9, 0.1, 0.2, 0.8));

        let hp_pct = (current_hp / max_hp).clamp(0.0, 1.0);
        draw_rectangle(pos.x - 25.0, pos.y - 38.0, 50.0, 6.0, DARKGRAY);
        draw_rectangle(pos.x - 25.0, pos.y - 38.0, 50.0 * hp_pct, 6.0, RED);
        draw_rectangle_lines(pos.x - 25.0, pos.y - 38.0, 50.0, 6.0, 1.5, WHITE);
    }
}

/// Отрисовка снаряда с высокодетализированными эффектами для атак босса и мобов
pub fn draw_projectile_sprite(proj: &crate::ecs::world::ProjectileEntity, textures: &GameTextures) {
    let pos = proj.pos;
    let vel = proj.vel;
    let radius = proj.radius;
    let dir = vel.normalize_or_zero();
    let norm = Vec2::new(-dir.y, dir.x);
    let angle = dir.y.atan2(dir.x);

    // 1. Атаки сил Директора / мобов
    if proj.faction == crate::ecs::components::combat::Faction::Director {
        // Огненный шар элементаля (Тир 7)
        if proj.damage_type == crate::ecs::components::combat::DamageType::Fire {
            if let Some(tex) = textures.proj_fire.as_ref() {
                let size = radius * 3.0;
                draw_texture_ex(
                    tex,
                    pos.x - size / 2.0,
                    pos.y - size / 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(size, size)),
                        rotation: angle,
                        pivot: Some(pos),
                        ..Default::default()
                    },
                );
            } else {
                draw_circle(pos.x, pos.y, radius, ORANGE);
                draw_circle(pos.x, pos.y, radius * 0.5, YELLOW);
            }
            return;
        }

        // Взмах косы Жнеца Смерти (Тир 8)
        if proj.damage_type == crate::ecs::components::combat::DamageType::True {
            if let Some(tex) = textures.proj_scythe.as_ref() {
                let rot = get_time() as f32 * 12.0;
                let size = radius * 3.2;
                draw_texture_ex(
                    tex,
                    pos.x - size / 2.0,
                    pos.y - size / 2.0,
                    WHITE,
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(size, size)),
                        rotation: rot,
                        pivot: Some(pos),
                        ..Default::default()
                    },
                );
            } else {
                draw_circle(pos.x, pos.y, radius, PURPLE);
                draw_circle_lines(pos.x, pos.y, radius, 2.0, RED);
            }
            return;
        }

        // Демоническая стрела крови и теней (Культисты, Тир 3)
        let tip = pos + dir * (radius * 2.2);
        let back_l = pos - dir * (radius * 1.2) + norm * (radius * 0.9);
        let back_r = pos - dir * (radius * 1.2) - norm * (radius * 0.9);

        // Внешнее кровавое оперение
        draw_triangle(tip, back_l, back_r, Color::new(0.75, 0.05, 0.20, 0.95));
        draw_triangle_lines(tip, back_l, back_r, 1.5, Color::new(0.95, 0.2, 0.35, 1.0));

        // Внутренний сгусток темной магии
        draw_circle(pos.x, pos.y, radius * 0.65, Color::new(0.35, 0.0, 0.55, 0.95));
        draw_circle(pos.x, pos.y, radius * 0.35, Color::new(0.9, 0.2, 0.6, 1.0));

        // Дымный теневой след
        let trail_len = radius * 3.5;
        draw_line(
            pos.x,
            pos.y,
            pos.x - dir.x * trail_len,
            pos.y - dir.y * trail_len,
            3.5,
            Color::new(0.45, 0.0, 0.6, 0.45),
        );
        draw_line(
            pos.x,
            pos.y,
            pos.x - dir.x * (trail_len * 0.6),
            pos.y - dir.y * (trail_len * 0.6),
            2.0,
            Color::new(0.85, 0.1, 0.3, 0.7),
        );
        return;
    }

    // 2. Атаки Босса (Соло-героя) по видам вооружения
    if let Some(w_kind) = proj.source_weapon {
        match w_kind {
            // Клинок Ветра: светящийся ветряной клинок
            crate::ecs::components::inventory::WeaponType::WindBlade => {
                if let Some(tex) = textures.proj_wind.as_ref() {
                    let size = radius * 3.4;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: angle + std::f32::consts::PI / 2.0,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
                let tip = pos + dir * (radius * 2.2);
                let wing1 = pos + norm * (radius * 1.5) - dir * (radius * 0.4);
                let wing2 = pos - norm * (radius * 1.5) - dir * (radius * 0.4);
                let tail = pos - dir * radius;

                draw_triangle(tip, wing1, tail, Color::new(0.5, 1.0, 0.9, 0.95));
                draw_triangle(tip, wing2, tail, Color::new(0.5, 1.0, 0.9, 0.95));
                draw_line(wing1.x, wing1.y, tip.x, tip.y, 2.5, WHITE);
                draw_line(wing2.x, wing2.y, tip.x, tip.y, 2.5, WHITE);
                draw_line(pos.x - dir.x * 24.0, pos.y - dir.y * 24.0, pos.x, pos.y, 3.0, Color::new(0.3, 0.9, 0.8, 0.6));
                return;
            }
            // Магический Жезл: пульсирующая сфера тайной магии
            crate::ecs::components::inventory::WeaponType::MagicWand => {
                if let Some(tex) = textures.proj_magic.as_ref() {
                    let size = radius * 3.2;
                    let rot = get_time() as f32 * 8.0;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: rot,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
                draw_circle(pos.x, pos.y, radius * 1.3, Color::new(0.7, 0.2, 1.0, 0.5));
                draw_circle(pos.x, pos.y, radius * 0.85, Color::new(0.9, 0.4, 1.0, 0.95));
                draw_circle(pos.x, pos.y, radius * 0.4, WHITE);
                return;
            }
            // Святой Молот: небесный молот
            crate::ecs::components::inventory::WeaponType::HolyHammer => {
                if let Some(tex) = textures.proj_holy.as_ref() {
                    let rot = get_time() as f32 * 12.0;
                    let size = radius * 3.0;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: rot,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
                let rot_time = get_time() as f32 * 12.0;
                let h_dir = Vec2::new(rot_time.cos(), rot_time.sin());
                let h_norm = Vec2::new(-h_dir.y, h_dir.x);
                draw_line(pos.x - h_dir.x * 14.0, pos.y - h_dir.y * 14.0, pos.x + h_dir.x * 10.0, pos.y + h_dir.y * 10.0, 4.0, Color::new(0.55, 0.38, 0.18, 1.0));
                let head_c = pos + h_dir * 10.0;
                draw_line(head_c.x - h_norm.x * 12.0, head_c.y - h_norm.y * 12.0, head_c.x + h_norm.x * 12.0, head_c.y + h_norm.y * 12.0, 8.0, GOLD);
                draw_circle(head_c.x, head_c.y, 5.0, WHITE);
                return;
            }
            // Ледяные Осколки: ограненные кристаллы льда
            crate::ecs::components::inventory::WeaponType::IceShards => {
                if let Some(tex) = textures.proj_ice.as_ref() {
                    let size = radius * 3.2;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: angle,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
                let p_front = pos + dir * (radius * 2.0);
                let p_back = pos - dir * (radius * 1.5);
                let p_left = pos + norm * (radius * 0.9);
                let p_right = pos - norm * (radius * 0.9);
                draw_triangle(p_front, p_left, p_back, Color::new(0.4, 0.85, 1.0, 0.95));
                draw_triangle(p_front, p_right, p_back, Color::new(0.7, 0.95, 1.0, 0.95));
                draw_line(p_front.x, p_front.y, p_back.x, p_back.y, 2.0, WHITE);
                return;
            }
            // Чумные Сферы и Токсичный Мор: ядовитые сгустки кислоты
            crate::ecs::components::inventory::WeaponType::PoisonOrbs | crate::ecs::components::inventory::WeaponType::ToxicPestilence => {
                if let Some(tex) = textures.proj_poison.as_ref() {
                    let size = radius * 3.2;
                    let rot = get_time() as f32 * 6.0;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: rot,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
                draw_circle(pos.x, pos.y, radius * 1.2, Color::new(0.1, 0.95, 0.25, 0.85));
                draw_circle(pos.x, pos.y, radius * 0.7, Color::new(0.4, 1.0, 0.3, 1.0));
                draw_circle(pos.x, pos.y, radius * 0.35, WHITE);
                draw_circle_lines(pos.x, pos.y, radius * 1.2, 1.5, LIME);
                return;
            }
            // Топор Палача и Вихревой Секач: вращающиеся секиры
            crate::ecs::components::inventory::WeaponType::ExecutionerAxe | crate::ecs::components::inventory::WeaponType::WhirlwindCleaver => {
                if let Some(tex) = textures.proj_axe.as_ref() {
                    let rot = get_time() as f32 * 14.0;
                    let size = radius * 2.8;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: rot,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
            }
            // Небесный Лук и Звездопадный Залп: стрелы света
            crate::ecs::components::inventory::WeaponType::CelestialBow | crate::ecs::components::inventory::WeaponType::StarfallBarrage => {
                if let Some(tex) = textures.proj_arrow.as_ref() {
                    let size = radius * 3.0;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: angle,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
            }
            // Грозовая Сфера и Гнев Грозы: сфера молнии
            crate::ecs::components::inventory::WeaponType::ThunderOrb | crate::ecs::components::inventory::WeaponType::ThunderstormWrath => {
                if let Some(tex) = textures.proj_lightning.as_ref() {
                    let size = radius * 2.8;
                    let rot = get_time() as f32 * 8.0;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: rot,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
            }
            // Кровавые Кинжалы и Кровавая Жатва: летящие кинжалы
            crate::ecs::components::inventory::WeaponType::BloodDaggers | crate::ecs::components::inventory::WeaponType::BloodHarvest => {
                if let Some(tex) = textures.proj_dagger.as_ref() {
                    let size = radius * 2.8;
                    draw_texture_ex(
                        tex,
                        pos.x - size / 2.0,
                        pos.y - size / 2.0,
                        WHITE,
                        DrawTextureParams {
                            dest_size: Some(Vec2::new(size, size)),
                            rotation: angle,
                            pivot: Some(pos),
                            ..Default::default()
                        },
                    );
                    return;
                }
            }
            _ => {}
        }
    }

    // Текстурный fallback
    if let Some(tex) = textures.get_projectile_texture(proj.damage_type) {
        let size = radius * 3.2;
        draw_texture_ex(
            tex,
            pos.x - size / 2.0,
            pos.y - size / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(Vec2::new(size, size)),
                rotation: angle,
                pivot: Some(pos),
                ..Default::default()
            },
        );
    } else {
        // Общий процедурный fallback
        draw_circle(pos.x, pos.y, radius, WHITE);
        draw_circle(pos.x, pos.y, radius * 0.6, GOLD);
    }
}
