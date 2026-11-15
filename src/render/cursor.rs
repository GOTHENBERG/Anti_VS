use macroquad::prelude::*;

/// Отрисовывает инфернальный курсор Повелителя (Демоническая Длань Режиссера)
pub fn draw_overlord_cursor(pos: Vec2, is_down: bool, is_hovering: bool, time: f32) {
    let p = pos;

    if is_down {
        // Режим захвата/сжатия кулака (Drag / Grip)
        // 1. Аура хватки
        draw_circle(p.x, p.y, 14.0, Color::new(0.8, 0.1, 0.2, 0.25));

        // 2. Сжатые когти (4 пальца образуют хищный замок)
        let claw_col = Color::new(0.18, 0.16, 0.22, 1.0);
        let gold_rim = Color::new(0.95, 0.75, 0.20, 1.0);

        draw_circle(p.x, p.y, 8.5, claw_col);
        draw_circle_lines(p.x, p.y, 8.5, 2.0, gold_rim);

        // 3. Пылающее инфернальное ядро в центре кулака
        let pulse = (time * 18.0).sin() * 1.5;
        draw_circle(p.x, p.y, 4.0 + pulse, Color::new(1.0, 0.15, 0.25, 0.95));
        draw_circle(p.x, p.y, 2.0, WHITE);

        // 4. Искры силы
        for i in 0..4 {
            let angle = (i as f32) * std::f32::consts::PI / 2.0 + time * 6.0;
            let sp = p + Vec2::new(angle.cos() * 12.0, angle.sin() * 12.0);
            draw_circle(sp.x, sp.y, 1.5, GOLD);
        }
    } else {
        // Режим указателя: Демонический костяной коготь с руническим глазом
        let base_col = if is_hovering {
            Color::new(0.24, 0.12, 0.32, 1.0)
        } else {
            Color::new(0.12, 0.10, 0.16, 0.98)
        };
        let rim_col = if is_hovering { GOLD } else { Color::new(0.7, 0.25, 0.30, 0.95) };

        // Тень под курсором
        draw_triangle(
            p + Vec2::new(3.0, 3.0),
            p + Vec2::new(19.0, 13.0),
            p + Vec2::new(13.0, 19.0),
            Color::new(0.0, 0.0, 0.0, 0.45),
        );

        // Главный хищный коготь-указатель
        draw_triangle(
            p,
            p + Vec2::new(16.0, 10.0),
            p + Vec2::new(10.0, 16.0),
            base_col,
        );
        draw_triangle_lines(
            p,
            p + Vec2::new(16.0, 10.0),
            p + Vec2::new(10.0, 16.0),
            1.8,
            rim_col,
        );

        // Рукоять/сустав когтя
        draw_circle(p.x + 13.0, p.y + 13.0, 4.5, base_col);
        draw_circle_lines(p.x + 13.0, p.y + 13.0, 4.5, 1.5, rim_col);

        // Боковые защитные шипы
        draw_line(p.x + 10.0, p.y + 16.0, p.x + 6.0, p.y + 22.0, 2.0, rim_col);
        draw_line(p.x + 16.0, p.y + 10.0, p.x + 22.0, p.y + 6.0, 2.0, rim_col);

        // Пульсирующий Багровый Глаз Душ в ладони
        let eye_col = if is_hovering {
            Color::new(1.0, 0.85, 0.20, 1.0) // Золотое пламя при наведении на интерактивные кнопки
        } else {
            let pulse = (time * 8.0).sin() * 0.25 + 0.75;
            Color::new(1.0, 0.15 * pulse, 0.25 * pulse, 0.95)
        };

        draw_circle(p.x + 9.0, p.y + 9.0, 3.2, eye_col);
        draw_circle(p.x + 8.5, p.y + 8.5, 1.2, WHITE);
    }
}
