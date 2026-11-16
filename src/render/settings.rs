use macroquad::prelude::*;
use crate::audio::{AudioEvent, AudioManager};
use crate::render::font::GameFonts;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    None,
    Resume,
    Restart,
    OpenCodex,
}

pub fn draw_pause_settings(fonts: &GameFonts, audio: &mut AudioManager) -> SettingsAction {
    let screen_w = screen_width();
    let screen_h = screen_height();
    let (mx, my) = mouse_position();
    let is_clicked = is_mouse_button_pressed(MouseButton::Left);
    let is_down = is_mouse_button_down(MouseButton::Left);

    // 1. Полупрозрачное затемнение поля боя
    draw_rectangle(0.0, 0.0, screen_w, screen_h, Color::new(0.0, 0.0, 0.0, 0.78));

    // 2. Габариты центральной панели настроек
    let panel_w = 540.0;
    let panel_h = 430.0;
    let x = (screen_w - panel_w) / 2.0;
    let y = (screen_h - panel_h) / 2.0;

    // Фоновая панель и рамка
    draw_rectangle(x, y, panel_w, panel_h, Color::new(0.07, 0.07, 0.11, 0.98));
    draw_rectangle_lines(x, y, panel_w, panel_h, 2.5, Color::new(0.85, 0.65, 0.20, 1.0));

    // Верхняя плашка заголовка
    draw_rectangle(x, y, panel_w, 42.0, Color::new(0.12, 0.12, 0.18, 1.0));
    draw_rectangle_lines(x, y, panel_w, 42.0, 1.5, DARKGRAY);

    let title = "⚙ НАСТРОЙКИ ЗВУКА И ПАУЗА";
    fonts.draw_heading(title, x + 20.0, y + 28.0, 16.0, GOLD);

    // Кнопка быстрого закрытия [ X ]
    let close_btn_w = 36.0;
    let close_btn_h = 28.0;
    let close_btn_x = x + panel_w - close_btn_w - 8.0;
    let close_btn_y = y + 7.0;
    let close_hover = mx >= close_btn_x && mx <= close_btn_x + close_btn_w && my >= close_btn_y && my <= close_btn_y + close_btn_h;
    draw_rectangle(close_btn_x, close_btn_y, close_btn_w, close_btn_h, if close_hover { RED } else { Color::new(0.2, 0.1, 0.1, 0.9) });
    draw_rectangle_lines(close_btn_x, close_btn_y, close_btn_w, close_btn_h, 1.2, WHITE);
    let xw = fonts.measure_heading("X", 16.0).width;
    fonts.draw_heading("X", close_btn_x + (close_btn_w - xw) / 2.0, close_btn_y + 20.0, 16.0, WHITE);

    if is_clicked && close_hover {
        audio.handle_event(AudioEvent::UiClick);
        return SettingsAction::Resume;
    }

    // Подсказка под заголовком
    fonts.draw_text("Совет: клавиши [Esc] или [Space] возвращают в бой", x + 20.0, y + 64.0, 12.0, Color::new(0.7, 0.7, 0.8, 0.9));

    // -------------------------------------------------------------
    // СЕКЦИЯ 1: ГРОМКОСТЬ ЗВУКОВЫХ ЭФФЕКТОВ (SFX)
    // -------------------------------------------------------------
    let sfx_y = y + 85.0;
    draw_rectangle(x + 18.0, sfx_y, panel_w - 36.0, 72.0, Color::new(0.04, 0.05, 0.08, 0.95));
    draw_rectangle_lines(x + 18.0, sfx_y, panel_w - 36.0, 72.0, 1.0, Color::new(0.3, 0.35, 0.45, 0.8));

    fonts.draw_heading("🔊 Громкость эффектов (SFX):", x + 30.0, sfx_y + 24.0, 13.0, WHITE);
    let sfx_percent = (audio.sfx_volume * 100.0).round() as i32;
    let sfx_pct_str = format!("{}%", sfx_percent);
    fonts.draw_heading(&sfx_pct_str, x + panel_w - 90.0, sfx_y + 24.0, 15.0, GOLD);

    // Кнопка [-]
    let btn_minus_x = x + 30.0;
    let btn_minus_y = sfx_y + 36.0;
    let btn_w = 32.0;
    let btn_h = 24.0;
    let minus_hover = mx >= btn_minus_x && mx <= btn_minus_x + btn_w && my >= btn_minus_y && my <= btn_minus_y + btn_h;
    draw_rectangle(btn_minus_x, btn_minus_y, btn_w, btn_h, if minus_hover { Color::new(0.3, 0.25, 0.15, 1.0) } else { Color::new(0.12, 0.14, 0.20, 1.0) });
    draw_rectangle_lines(btn_minus_x, btn_minus_y, btn_w, btn_h, 1.2, if minus_hover { GOLD } else { GRAY });
    fonts.draw_heading("-", btn_minus_x + 11.0, btn_minus_y + 18.0, 16.0, WHITE);
    if is_clicked && minus_hover {
        let new_vol = (audio.sfx_volume - 0.05).max(0.0);
        audio.set_sfx_volume(new_vol);
        audio.handle_event(AudioEvent::UiClick);
    }

    // Интерактивный слайдер
    let slider_x = btn_minus_x + btn_w + 12.0;
    let slider_y = btn_minus_y + 4.0;
    let slider_w = 340.0;
    let slider_h = 16.0;

    let slider_hover = mx >= slider_x && mx <= slider_x + slider_w && my >= slider_y - 6.0 && my <= slider_y + slider_h + 6.0;
    if is_down && slider_hover {
        let frac = ((mx - slider_x) / slider_w).clamp(0.0, 1.0);
        audio.set_sfx_volume(frac);
    }

    draw_rectangle(slider_x, slider_y, slider_w, slider_h, Color::new(0.10, 0.11, 0.16, 1.0));
    draw_rectangle(slider_x, slider_y, slider_w * audio.sfx_volume, slider_h, Color::new(0.92, 0.70, 0.20, 1.0));
    draw_rectangle_lines(slider_x, slider_y, slider_w, slider_h, 1.5, if slider_hover { GOLD } else { DARKGRAY });

    // Ручка слайдера
    let handle_x = (slider_x + slider_w * audio.sfx_volume - 4.0).clamp(slider_x, slider_x + slider_w - 8.0);
    draw_rectangle(handle_x, slider_y - 3.0, 8.0, slider_h + 6.0, WHITE);

    // Кнопка [+]
    let btn_plus_x = slider_x + slider_w + 12.0;
    let btn_plus_y = btn_minus_y;
    let plus_hover = mx >= btn_plus_x && mx <= btn_plus_x + btn_w && my >= btn_plus_y && my <= btn_plus_y + btn_h;
    draw_rectangle(btn_plus_x, btn_plus_y, btn_w, btn_h, if plus_hover { Color::new(0.3, 0.25, 0.15, 1.0) } else { Color::new(0.12, 0.14, 0.20, 1.0) });
    draw_rectangle_lines(btn_plus_x, btn_plus_y, btn_w, btn_h, 1.2, if plus_hover { GOLD } else { GRAY });
    fonts.draw_heading("+", btn_plus_x + 9.0, btn_plus_y + 18.0, 16.0, WHITE);
    if is_clicked && plus_hover {
        let new_vol = (audio.sfx_volume + 0.05).min(1.0);
        audio.set_sfx_volume(new_vol);
        audio.handle_event(AudioEvent::UiClick);
    }

    // -------------------------------------------------------------
    // СЕКЦИЯ 2: ГРОМКОСТЬ БОЕВОЙ МУЗЫКИ (MUSIC)
    // -------------------------------------------------------------
    let music_y = sfx_y + 82.0;
    draw_rectangle(x + 18.0, music_y, panel_w - 36.0, 72.0, Color::new(0.04, 0.05, 0.08, 0.95));
    draw_rectangle_lines(x + 18.0, music_y, panel_w - 36.0, 72.0, 1.0, Color::new(0.3, 0.35, 0.45, 0.8));

    fonts.draw_heading("🎵 Громкость боевой музыки (Music):", x + 30.0, music_y + 24.0, 13.0, WHITE);
    let music_percent = (audio.music_volume * 100.0).round() as i32;
    let music_pct_str = format!("{}%", music_percent);
    fonts.draw_heading(&music_pct_str, x + panel_w - 90.0, music_y + 24.0, 15.0, SKYBLUE);

    // Кнопка [-]
    let m_minus_x = x + 30.0;
    let m_minus_y = music_y + 36.0;
    let m_minus_hover = mx >= m_minus_x && mx <= m_minus_x + btn_w && my >= m_minus_y && my <= m_minus_y + btn_h;
    draw_rectangle(m_minus_x, m_minus_y, btn_w, btn_h, if m_minus_hover { Color::new(0.15, 0.25, 0.35, 1.0) } else { Color::new(0.12, 0.14, 0.20, 1.0) });
    draw_rectangle_lines(m_minus_x, m_minus_y, btn_w, btn_h, 1.2, if m_minus_hover { SKYBLUE } else { GRAY });
    fonts.draw_heading("-", m_minus_x + 11.0, m_minus_y + 18.0, 16.0, WHITE);
    if is_clicked && m_minus_hover {
        let new_vol = (audio.music_volume - 0.01).max(0.0);
        audio.set_music_volume(new_vol);
        audio.handle_event(AudioEvent::UiClick);
    }

    // Интерактивный слайдер музыки
    let m_slider_x = m_minus_x + btn_w + 12.0;
    let m_slider_y = m_minus_y + 4.0;
    let m_slider_w = 340.0;
    let m_slider_h = 16.0;

    let m_slider_hover = mx >= m_slider_x && mx <= m_slider_x + m_slider_w && my >= m_slider_y - 6.0 && my <= m_slider_y + m_slider_h + 6.0;
    if is_down && m_slider_hover {
        let frac = ((mx - m_slider_x) / m_slider_w).clamp(0.0, 1.0);
        audio.set_music_volume(frac);
    }

    draw_rectangle(m_slider_x, m_slider_y, m_slider_w, m_slider_h, Color::new(0.10, 0.11, 0.16, 1.0));
    draw_rectangle(m_slider_x, m_slider_y, m_slider_w * audio.music_volume, m_slider_h, Color::new(0.28, 0.65, 0.95, 1.0));
    draw_rectangle_lines(m_slider_x, m_slider_y, m_slider_w, m_slider_h, 1.5, if m_slider_hover { SKYBLUE } else { DARKGRAY });

    // Ручка слайдера
    let m_handle_x = (m_slider_x + m_slider_w * audio.music_volume - 4.0).clamp(m_slider_x, m_slider_x + m_slider_w - 8.0);
    draw_rectangle(m_handle_x, m_slider_y - 3.0, 8.0, m_slider_h + 6.0, WHITE);

    // Кнопка [+]
    let m_plus_x = m_slider_x + m_slider_w + 12.0;
    let m_plus_y = m_minus_y;
    let m_plus_hover = mx >= m_plus_x && mx <= m_plus_x + btn_w && my >= m_plus_y && my <= m_plus_y + btn_h;
    draw_rectangle(m_plus_x, m_plus_y, btn_w, btn_h, if m_plus_hover { Color::new(0.15, 0.25, 0.35, 1.0) } else { Color::new(0.12, 0.14, 0.20, 1.0) });
    draw_rectangle_lines(m_plus_x, m_plus_y, btn_w, btn_h, 1.2, if m_plus_hover { SKYBLUE } else { GRAY });
    fonts.draw_heading("+", m_plus_x + 9.0, m_plus_y + 18.0, 16.0, WHITE);
    if is_clicked && m_plus_hover {
        let new_vol = (audio.music_volume + 0.01).min(1.0);
        audio.set_music_volume(new_vol);
        audio.handle_event(AudioEvent::UiClick);
    }

    // -------------------------------------------------------------
    // СЕКЦИЯ 3: КНОПКА ЗАГЛУШЕНИЯ (MUTE)
    // -------------------------------------------------------------
    let mute_y = music_y + 82.0;
    let mute_btn_w = panel_w - 36.0;
    let mute_btn_h = 32.0;
    let mute_hover = mx >= x + 18.0 && mx <= x + 18.0 + mute_btn_w && my >= mute_y && my <= mute_y + mute_btn_h;

    draw_rectangle(
        x + 18.0,
        mute_y,
        mute_btn_w,
        mute_btn_h,
        if audio.is_muted {
            if mute_hover { Color::new(0.40, 0.15, 0.15, 1.0) } else { Color::new(0.25, 0.10, 0.10, 0.95) }
        } else {
            if mute_hover { Color::new(0.15, 0.35, 0.20, 1.0) } else { Color::new(0.10, 0.22, 0.14, 0.95) }
        },
    );
    draw_rectangle_lines(
        x + 18.0,
        mute_y,
        mute_btn_w,
        mute_btn_h,
        1.5,
        if audio.is_muted { RED } else { GREEN },
    );

    let mute_label = if audio.is_muted {
        "🔇 ВСЕ ЗВУКИ ЗАГЛУШЕНЫ (Кликните или [M], чтобы включить)"
    } else {
        "🔊 ЗВУКИ И МУЗЫКА АКТИВНЫ (Кликните или [M], чтобы заглушить)"
    };
    let mlw = fonts.measure_heading(mute_label, 12.0).width;
    fonts.draw_heading(
        mute_label,
        x + 18.0 + (mute_btn_w - mlw) / 2.0,
        mute_y + 21.0,
        12.0,
        if audio.is_muted { RED } else { Color::new(0.7, 1.0, 0.7, 1.0) },
    );

    if is_clicked && mute_hover {
        audio.toggle_mute();
        audio.handle_event(AudioEvent::UiClick);
    }

    // -------------------------------------------------------------
    // СЕКЦИЯ 4: КНОПКИ ДЕЙСТВИЙ (ВНИЗУ)
    // -------------------------------------------------------------
    let actions_y = mute_y + 44.0;
    let act_btn_w = (panel_w - 52.0) / 3.0;
    let act_btn_h = 36.0;

    // 1. Продолжить бой
    let resume_x = x + 18.0;
    let resume_hover = mx >= resume_x && mx <= resume_x + act_btn_w && my >= actions_y && my <= actions_y + act_btn_h;
    draw_rectangle(resume_x, actions_y, act_btn_w, act_btn_h, if resume_hover { Color::new(0.18, 0.45, 0.25, 1.0) } else { Color::new(0.12, 0.28, 0.18, 0.95) });
    draw_rectangle_lines(resume_x, actions_y, act_btn_w, act_btn_h, 1.5, if resume_hover { GREEN } else { DARKGRAY });
    let rw = fonts.measure_heading("▶ В БОЙ [Esc]", 12.0).width;
    fonts.draw_heading("▶ В БОЙ [Esc]", resume_x + (act_btn_w - rw) / 2.0, actions_y + 23.0, 12.0, WHITE);

    if is_clicked && resume_hover {
        audio.handle_event(AudioEvent::UiClick);
        return SettingsAction::Resume;
    }

    // 2. Кодекс
    let codex_x = resume_x + act_btn_w + 8.0;
    let codex_hover = mx >= codex_x && mx <= codex_x + act_btn_w && my >= actions_y && my <= actions_y + act_btn_h;
    draw_rectangle(codex_x, actions_y, act_btn_w, act_btn_h, if codex_hover { Color::new(0.30, 0.25, 0.40, 1.0) } else { Color::new(0.16, 0.14, 0.24, 0.95) });
    draw_rectangle_lines(codex_x, actions_y, act_btn_w, act_btn_h, 1.5, if codex_hover { GOLD } else { DARKGRAY });
    let cw = fonts.measure_heading("📖 КОДЕКС [I]", 12.0).width;
    fonts.draw_heading("📖 КОДЕКС [I]", codex_x + (act_btn_w - cw) / 2.0, actions_y + 23.0, 12.0, WHITE);

    if is_clicked && codex_hover {
        audio.handle_event(AudioEvent::BookOpen);
        return SettingsAction::OpenCodex;
    }

    // 3. Перезапуск
    let restart_x = codex_x + act_btn_w + 8.0;
    let restart_hover = mx >= restart_x && mx <= restart_x + act_btn_w && my >= actions_y && my <= actions_y + act_btn_h;
    draw_rectangle(restart_x, actions_y, act_btn_w, act_btn_h, if restart_hover { Color::new(0.40, 0.20, 0.15, 1.0) } else { Color::new(0.22, 0.12, 0.10, 0.95) });
    draw_rectangle_lines(restart_x, actions_y, act_btn_w, act_btn_h, 1.5, if restart_hover { ORANGE } else { DARKGRAY });
    let rtw = fonts.measure_heading("🔄 ЗАНОВО [R]", 12.0).width;
    fonts.draw_heading("🔄 ЗАНОВО [R]", restart_x + (act_btn_w - rtw) / 2.0, actions_y + 23.0, 12.0, WHITE);

    if is_clicked && restart_hover {
        audio.handle_event(AudioEvent::UiClick);
        return SettingsAction::Restart;
    }

    SettingsAction::None
}
