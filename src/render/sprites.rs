use macroquad::prelude::*;
use crate::ecs::components::combat::DamageType;
use crate::ecs::world::MobEntity;

/// Отрисовка фона большой карты: плиты, трещины, лавовые разломы и границы
pub fn draw_environment(map_w: f32, map_h: f32) {
    // 1. Темный базовый пол
    clear_background(Color::new(0.07, 0.07, 0.10, 1.0));

    // 2. Сетка каменных плит
    let tile_size = 80.0;
    let grid_color = Color::new(0.11, 0.11, 0.15, 1.0);
    let crack_color = Color::new(0.18, 0.12, 0.14, 0.6);

    let cols = (map_w / tile_size) as usize;
    let rows = (map_h / tile_size) as usize;

    for y in 0..rows {
        let py = (y as f32) * tile_size;
        draw_line(0.0, py, map_w, py, 1.0, grid_color);
        for x in 0..cols {
            let px = (x as f32) * tile_size;
            if y == 0 {
                draw_line(px, 0.0, px, map_h, 1.0, grid_color);
            }

            // Псевдо-случайные декоративные трещины и руны на плитах
            let hash = ((x * 73856093) ^ (y * 19349663)) % 100;
            if hash < 8 {
                // Древняя трещина на плите
                draw_line(px + 15.0, py + 15.0, px + 55.0, py + 45.0, 1.5, crack_color);
                draw_line(px + 55.0, py + 45.0, px + 65.0, py + 70.0, 1.0, crack_color);
            } else if hash == 9 {
                // Кровавая руна на земле
                draw_circle_lines(px + 40.0, py + 40.0, 12.0, 1.0, Color::new(0.4, 0.08, 0.12, 0.4));
            }
        }
    }

    // 3. Инфернальная граница карты (огненный барьер и древние колонны)
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

/// Отрисовка Босса (Соло-героя: благородный Паладин в латах с плащом и мечом)
pub fn draw_boss(pos: Vec2, vel: Vec2, invulnerable: bool, time: f32) {
    let facing_dir = if vel.length_squared() > 1.0 {
        vel.normalize()
    } else {
        Vec2::new(1.0, 0.0)
    };
    let normal_dir = Vec2::new(-facing_dir.y, facing_dir.x);

    // 1. Тень под ногами
    draw_ellipse(pos.x, pos.y + 14.0, 22.0, 10.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.45));

    // 2. Развевающийся алый плащ за спиной
    let cape_flutter = (time * 8.0).sin() * 5.0;
    let cape_anchor = pos - facing_dir * 12.0;
    let cape_tip_1 = cape_anchor - facing_dir * 24.0 + normal_dir * (12.0 + cape_flutter);
    let cape_tip_2 = cape_anchor - facing_dir * 28.0 - normal_dir * (12.0 - cape_flutter);
    draw_triangle(cape_anchor, cape_tip_1, cape_tip_2, Color::new(0.75, 0.12, 0.15, 1.0));

    // 3. Латы и тело (сталь с золотой окантовкой)
    let body_color = if invulnerable {
        Color::new(1.0, 1.0, 0.4, 0.9)
    } else {
        Color::new(0.85, 0.88, 0.92, 1.0)
    };
    draw_circle(pos.x, pos.y, 18.0, body_color);
    draw_circle_lines(pos.x, pos.y, 18.0, 2.5, Color::new(0.95, 0.75, 0.15, 1.0));

    // Нагрудная магическая руна света
    draw_circle(pos.x + facing_dir.x * 4.0, pos.y + facing_dir.y * 4.0, 6.0, SKYBLUE);
    draw_circle(pos.x + facing_dir.x * 4.0, pos.y + facing_dir.y * 4.0, 3.0, WHITE);

    // 4. Рыцарский шлем с золотым гребнем
    let head_pos = pos + facing_dir * 8.0;
    draw_circle(head_pos.x, head_pos.y, 9.0, Color::new(0.9, 0.9, 0.95, 1.0));
    draw_circle_lines(head_pos.x, head_pos.y, 9.0, 1.5, GOLD);
    // Прорезь забрала
    draw_line(
        head_pos.x + normal_dir.x * 5.0,
        head_pos.y + normal_dir.y * 5.0,
        head_pos.x - normal_dir.x * 5.0,
        head_pos.y - normal_dir.y * 5.0,
        2.0,
        BLACK,
    );

    // 5. Огромный сияющий двуручный меч в правой руке
    let hand_pos = pos + normal_dir * 16.0 + facing_dir * 6.0;
    let sword_tip = hand_pos + facing_dir * 30.0;
    // Лезвие меча
    draw_line(hand_pos.x, hand_pos.y, sword_tip.x, sword_tip.y, 4.5, WHITE);
    draw_line(hand_pos.x, hand_pos.y, sword_tip.x, sword_tip.y, 2.0, SKYBLUE);
    // Гарда
    let hilt_left = hand_pos + normal_dir * 6.0;
    let hilt_right = hand_pos - normal_dir * 6.0;
    draw_line(hilt_left.x, hilt_left.y, hilt_right.x, hilt_right.y, 3.0, GOLD);

    // 6. Щит рыцаря в левой руке
    let shield_pos = pos - normal_dir * 15.0 + facing_dir * 4.0;
    draw_circle(shield_pos.x, shield_pos.y, 8.0, Color::new(0.2, 0.4, 0.7, 1.0));
    draw_circle_lines(shield_pos.x, shield_pos.y, 8.0, 2.0, GOLD);
}

/// Отрисовка моба в зависимости от Тира
pub fn draw_mob_sprite(mob: &MobEntity, time: f32) {
    match mob.tier {
        1 => draw_skeleton(mob, time),
        2 => draw_hound(mob, time),
        3 => draw_cultist(mob, time),
        4 => draw_ice_golem(mob, time),
        _ => draw_kamikaze(mob, time),
    }
}

/// Тир 1: Скелет-пехотинец (череп, ребра, клинок и деревянный щит)
fn draw_skeleton(mob: &MobEntity, time: f32) {
    let p = mob.pos;
    let dir = mob.vel.normalize_or_zero();
    let norm = Vec2::new(-dir.y, dir.x);
    let wobble = (time * 10.0 + (mob.id as f32)).sin() * 2.0;

    // Тень
    draw_ellipse(p.x, p.y + 10.0, 10.0, 4.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.35));

    // Реберная клетка
    draw_circle(p.x, p.y + 2.0, 7.0, Color::new(0.8, 0.8, 0.8, 1.0));

    // Череп
    let head = p + Vec2::new(wobble, -8.0);
    draw_circle(head.x, head.y, 6.0, Color::new(0.9, 0.9, 0.9, 1.0));
    // Черные глазницы
    draw_circle(head.x + dir.x * 2.0 - norm.x * 2.0, head.y - 1.0, 1.5, BLACK);
    draw_circle(head.x + dir.x * 2.0 + norm.x * 2.0, head.y - 1.0, 1.5, BLACK);

    // Ржавый меч
    let sword_base = p + norm * 8.0;
    let sword_tip = sword_base + dir * 14.0;
    draw_line(sword_base.x, sword_base.y, sword_tip.x, sword_tip.y, 2.5, Color::new(0.65, 0.45, 0.35, 1.0));

    // Круглый щит
    let shield_pos = p - norm * 8.0;
    draw_circle(shield_pos.x, shield_pos.y, 6.0, Color::new(0.5, 0.35, 0.2, 1.0));
    draw_circle_lines(shield_pos.x, shield_pos.y, 6.0, 1.0, Color::new(0.7, 0.7, 0.7, 1.0));
}

/// Тир 2: Чёрная гончая (хищный четвероногий зверь с горящими глазами)
fn draw_hound(mob: &MobEntity, time: f32) {
    let p = mob.pos;
    let dir = if mob.vel.length_squared() > 1.0 {
        mob.vel.normalize()
    } else {
        Vec2::X
    };
    let norm = Vec2::new(-dir.y, dir.x);
    let run_bob = (time * 18.0 + (mob.id as f32)).sin() * 3.0;

    // Тень
    draw_ellipse(p.x, p.y + 11.0, 14.0, 5.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.4));

    // Мускулистое темное тело
    let body_pos = p + Vec2::new(0.0, run_bob);
    draw_ellipse(body_pos.x, body_pos.y, 13.0, 8.0, 0.0, Color::new(0.18, 0.12, 0.14, 1.0));
    draw_ellipse_lines(body_pos.x, body_pos.y, 13.0, 8.0, 0.0, 1.5, Color::new(0.35, 0.15, 0.1, 1.0));

    // Морда с клыками
    let head = body_pos + dir * 12.0;
    draw_circle(head.x, head.y, 6.0, Color::new(0.15, 0.1, 0.12, 1.0));
    // Горящие алые глаза
    draw_circle(head.x + norm.x * 2.5, head.y - 1.5, 1.5, RED);
    draw_circle(head.x - norm.x * 2.5, head.y - 1.5, 1.5, RED);

    // Заостренные уши
    draw_triangle(head, head - dir * 4.0 + norm * 5.0, head + dir * 2.0 + norm * 3.0, Color::new(0.3, 0.1, 0.1, 1.0));
    draw_triangle(head, head - dir * 4.0 - norm * 5.0, head + dir * 2.0 - norm * 3.0, Color::new(0.3, 0.1, 0.1, 1.0));

    // Хвост
    let tail_tip = body_pos - dir * 14.0 + Vec2::new(0.0, -4.0);
    draw_line(body_pos.x, body_pos.y, tail_tip.x, tail_tip.y, 2.0, Color::new(0.2, 0.1, 0.12, 1.0));
}

/// Тир 3: Культист-стрелок (темный балахон, капюшон и оккультный посох)
fn draw_cultist(mob: &MobEntity, time: f32) {
    let p = mob.pos;
    let float_y = (time * 4.0 + (mob.id as f32)).sin() * 2.5;

    // Тень
    draw_ellipse(p.x, p.y + 12.0, 12.0, 5.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.4));

    // Балахон культиста (коническая мантия)
    let top = p + Vec2::new(0.0, -12.0 + float_y);
    let bot_l = p + Vec2::new(-9.0, 10.0 + float_y);
    let bot_r = p + Vec2::new(9.0, 10.0 + float_y);
    draw_triangle(top, bot_l, bot_r, Color::new(0.35, 0.12, 0.45, 1.0));
    draw_triangle_lines(top, bot_l, bot_r, 1.5, Color::new(0.55, 0.2, 0.7, 1.0));

    // Капюшон и тьма лица
    let head = p + Vec2::new(0.0, -8.0 + float_y);
    draw_circle(head.x, head.y, 7.0, Color::new(0.2, 0.05, 0.25, 1.0));
    // Светящиеся фиолетовые глаза в тени капюшона
    draw_circle(head.x - 2.5, head.y + 1.0, 1.5, Color::new(0.9, 0.5, 1.0, 1.0));
    draw_circle(head.x + 2.5, head.y + 1.0, 1.5, Color::new(0.9, 0.5, 1.0, 1.0));

    // Оккультный посох с кристаллом
    let staff_base = p + Vec2::new(12.0, 10.0 + float_y);
    let staff_top = p + Vec2::new(12.0, -18.0 + float_y);
    draw_line(staff_base.x, staff_base.y, staff_top.x, staff_top.y, 2.5, Color::new(0.4, 0.25, 0.15, 1.0));
    // Пульсирующий кристалл бездны на посохе
    let pulse = (time * 8.0).sin() * 1.5;
    draw_circle(staff_top.x, staff_top.y, 4.5 + pulse, Color::new(0.8, 0.2, 0.9, 1.0));
    draw_circle(staff_top.x, staff_top.y, 2.0, WHITE);
}

/// Тир 4: Ледяной голем (огромный бронированный титан изо льда с рунами)
fn draw_ice_golem(mob: &MobEntity, _time: f32) {
    let p = mob.pos;

    // Тень гиганта
    draw_ellipse(p.x, p.y + 20.0, 24.0, 9.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.5));

    // Массивное торс-монолит
    draw_circle(p.x, p.y, 20.0, Color::new(0.25, 0.65, 0.85, 1.0));
    draw_circle_lines(p.x, p.y, 20.0, 3.0, Color::new(0.7, 0.9, 1.0, 1.0));

    // Ледяные наплечники
    draw_circle(p.x - 18.0, p.y - 8.0, 10.0, Color::new(0.35, 0.75, 0.95, 1.0));
    draw_circle(p.x + 18.0, p.y - 8.0, 10.0, Color::new(0.35, 0.75, 0.95, 1.0));
    draw_circle_lines(p.x - 18.0, p.y - 8.0, 10.0, 1.5, WHITE);
    draw_circle_lines(p.x + 18.0, p.y - 8.0, 10.0, 1.5, WHITE);

    // Голова-монолит
    draw_rectangle(p.x - 7.0, p.y - 18.0, 14.0, 12.0, Color::new(0.2, 0.5, 0.75, 1.0));
    draw_rectangle_lines(p.x - 7.0, p.y - 18.0, 14.0, 12.0, 1.5, WHITE);
    // Пронзительные морозные глаза
    draw_circle(p.x - 3.5, p.y - 12.0, 2.0, SKYBLUE);
    draw_circle(p.x + 3.5, p.y - 12.0, 2.0, SKYBLUE);

    // Морозные руны на груди
    draw_line(p.x - 6.0, p.y, p.x + 6.0, p.y, 2.0, WHITE);
    draw_line(p.x, p.y - 6.0, p.x, p.y + 6.0, 2.0, WHITE);

    // Полоска здоровья голема
    let hp_pct = (mob.health.current / mob.health.max).clamp(0.0, 1.0);
    draw_rectangle(p.x - 20.0, p.y - 28.0, 40.0, 5.0, DARKGRAY);
    draw_rectangle(p.x - 20.0, p.y - 28.0, 40.0 * hp_pct, 5.0, SKYBLUE);
    draw_rectangle_lines(p.x - 20.0, p.y - 28.0, 40.0, 5.0, 1.0, WHITE);
}

/// Тир 5: Камикадзе (взрывоопасный гоблин с бочонком пороха и горящим фитилем)
fn draw_kamikaze(mob: &MobEntity, time: f32) {
    let p = mob.pos;
    let dir = mob.vel.normalize_or_zero();
    let frenzied_bob = (time * 24.0 + (mob.id as f32)).sin() * 2.0;

    // Тень
    draw_ellipse(p.x, p.y + 14.0, 15.0, 5.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.4));

    // Бочонок со взрывчаткой на спине (красный с обручами)
    let barrel_pos = p - dir * 4.0 + Vec2::new(0.0, frenzied_bob);
    draw_circle(barrel_pos.x, barrel_pos.y, 14.0, Color::new(0.85, 0.15, 0.15, 1.0));
    draw_circle_lines(barrel_pos.x, barrel_pos.y, 14.0, 2.0, Color::new(0.2, 0.2, 0.2, 1.0));
    draw_line(barrel_pos.x - 10.0, barrel_pos.y, barrel_pos.x + 10.0, barrel_pos.y, 2.0, GOLD);

    // Голова безумного подрывника
    let head = p + dir * 8.0 + Vec2::new(0.0, frenzied_bob - 2.0);
    draw_circle(head.x, head.y, 7.0, Color::new(0.85, 0.45, 0.2, 1.0));
    // Безумные круглые желтые глаза
    draw_circle(head.x - 2.5, head.y - 1.0, 2.0, YELLOW);
    draw_circle(head.x + 2.5, head.y - 1.0, 2.0, YELLOW);
    draw_circle(head.x - 2.5, head.y - 1.0, 1.0, BLACK);
    draw_circle(head.x + 2.5, head.y - 1.0, 1.0, BLACK);

    // Горящий фитиль с искрами
    let fuse_top = barrel_pos + Vec2::new(0.0, -18.0);
    draw_line(barrel_pos.x, barrel_pos.y - 12.0, fuse_top.x, fuse_top.y, 2.0, Color::new(0.6, 0.4, 0.2, 1.0));

    let spark_pulse = (time * 30.0).sin() * 3.0;
    draw_circle(fuse_top.x, fuse_top.y, 4.0 + spark_pulse, ORANGE);
    draw_circle(fuse_top.x, fuse_top.y, 2.0, YELLOW);
}

/// Отрисовка Алтаря Крови
pub fn draw_blood_altar(pos: Vec2, current_hp: f32, max_hp: f32, time: f32) {
    // Основание алтаря
    draw_ellipse(pos.x, pos.y + 20.0, 38.0, 16.0, 0.0, Color::new(0.0, 0.0, 0.0, 0.5));
    draw_circle(pos.x, pos.y, 28.0, Color::new(0.18, 0.12, 0.22, 1.0));
    draw_circle_lines(pos.x, pos.y, 28.0, 3.0, Color::new(0.5, 0.1, 0.65, 1.0));

    // Чаша с бурлящей кровью
    let blood_pulse = (time * 4.0).sin() * 2.0;
    draw_circle(pos.x, pos.y, 18.0 + blood_pulse, Color::new(0.7, 0.05, 0.15, 1.0));
    draw_circle(pos.x, pos.y, 8.0, Color::new(0.9, 0.1, 0.2, 0.8));

    // Четыре колонны по углам чаши
    for i in 0..4 {
        let angle = (i as f32) * std::f32::consts::PI / 2.0 + (time * 0.5);
        let cx = pos.x + angle.cos() * 26.0;
        let cy = pos.y + angle.sin() * 26.0;
        draw_circle(cx, cy, 5.0, Color::new(0.4, 0.15, 0.5, 1.0));
    }

    // Полоска здоровья
    let hp_pct = (current_hp / max_hp).clamp(0.0, 1.0);
    draw_rectangle(pos.x - 25.0, pos.y - 38.0, 50.0, 6.0, DARKGRAY);
    draw_rectangle(pos.x - 25.0, pos.y - 38.0, 50.0 * hp_pct, 6.0, RED);
    draw_rectangle_lines(pos.x - 25.0, pos.y - 38.0, 50.0, 6.0, 1.5, WHITE);
}

/// Отрисовка снаряда с уникальной моделью
pub fn draw_projectile_sprite(pos: Vec2, vel: Vec2, radius: f32, damage_type: DamageType) {
    let dir = vel.normalize_or_zero();
    let norm = Vec2::new(-dir.y, dir.x);

    match damage_type {
        // Клинок ветра: серповидный рассекатель воздуха
        DamageType::Physical => {
            let tip = pos + dir * (radius * 1.6);
            let back_l = pos - dir * radius + norm * (radius * 1.2);
            let back_r = pos - dir * radius - norm * (radius * 1.2);
            draw_triangle(tip, back_l, back_r, Color::new(0.8, 0.95, 0.85, 0.9));
            draw_triangle_lines(tip, back_l, back_r, 1.5, Color::new(0.3, 0.9, 0.5, 1.0));
        }
        // Святой Молот: вращающийся золотой боевой молот
        DamageType::Holy => {
            draw_line(pos.x - dir.x * 10.0, pos.y - dir.y * 10.0, pos.x + dir.x * 6.0, pos.y + dir.y * 6.0, 3.0, Color::new(0.6, 0.4, 0.2, 1.0));
            // Боек молота
            let head_center = pos + dir * 6.0;
            draw_line(head_center.x - norm.x * 8.0, head_center.y - norm.y * 8.0, head_center.x + norm.x * 8.0, head_center.y + norm.y * 8.0, 6.0, GOLD);
            draw_circle(head_center.x, head_center.y, 4.0, WHITE);
        }
        // Ледяной осколок: граненый кристалл льда
        DamageType::Ice => {
            let p1 = pos + dir * (radius * 1.5);
            let p2 = pos + norm * (radius * 0.7);
            let p3 = pos - dir * (radius * 1.5);
            let p4 = pos - norm * (radius * 0.7);
            draw_triangle(p1, p2, p3, SKYBLUE);
            draw_triangle(p1, p4, p3, Color::new(0.6, 0.85, 1.0, 1.0));
            draw_line(p1.x, p1.y, p3.x, p3.y, 1.5, WHITE);
        }
        // Магическая стрела: сияющий тайный снаряд
        DamageType::True => {
            draw_circle(pos.x, pos.y, radius, Color::new(0.8, 0.3, 1.0, 0.9));
            draw_circle(pos.x, pos.y, radius * 0.5, WHITE);
            // Шлейф
            draw_line(pos.x, pos.y, pos.x - dir.x * (radius * 2.5), pos.y - dir.y * (radius * 2.5), 3.0, Color::new(0.6, 0.1, 0.9, 0.4));
        }
        // Огненный шар
        DamageType::Fire => {
            draw_circle(pos.x, pos.y, radius, ORANGE);
            draw_circle(pos.x, pos.y, radius * 0.6, YELLOW);
            draw_circle(pos.x, pos.y, radius * 0.3, WHITE);
        }
        _ => {
            draw_circle(pos.x, pos.y, radius, WHITE);
        }
    }
}
