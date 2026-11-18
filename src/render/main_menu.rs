use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use crate::render::font::GameFonts;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameRole {
    Director,
    Boss,
}

#[derive(Clone, Debug)]
pub enum MenuAction {
    Local(GameRole),
    Host { bind_address: String, duration_secs: f32 },
    Join { address: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NetworkField {
    HostBind,
    MatchMinutes,
    HostAddress,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerControl {
    Up,
    Down,
    Left,
    Right,
    Dash,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerSettings {
    pub nickname: String,
    pub skin: u8,
    pub move_up: String,
    pub move_down: String,
    pub move_left: String,
    pub move_right: String,
    pub dash: String,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            nickname: "Игрок".to_string(),
            skin: 0,
            move_up: "W".to_string(),
            move_down: "S".to_string(),
            move_left: "A".to_string(),
            move_right: "D".to_string(),
            dash: "Space".to_string(),
        }
    }
}

impl PlayerSettings {
    pub fn load() -> Self {
        std::fs::read_to_string("player_settings.json")
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .map(|mut settings| {
                settings.nickname = settings.nickname.chars().take(32).collect();
                settings.skin %= 5;
                settings
            })
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write("player_settings.json", text);
        }
    }

    pub fn key_for(&self, control: PlayerControl) -> KeyCode {
        let name = match control {
            PlayerControl::Up => &self.move_up,
            PlayerControl::Down => &self.move_down,
            PlayerControl::Left => &self.move_left,
            PlayerControl::Right => &self.move_right,
            PlayerControl::Dash => &self.dash,
        };
        parse_key_code(name).unwrap_or(match control {
            PlayerControl::Up => KeyCode::W,
            PlayerControl::Down => KeyCode::S,
            PlayerControl::Left => KeyCode::A,
            PlayerControl::Right => KeyCode::D,
            PlayerControl::Dash => KeyCode::Space,
        })
    }

    fn set_key(&mut self, control: PlayerControl, key: KeyCode) {
        let name = format!("{:?}", key);
        match control {
            PlayerControl::Up => self.move_up = name,
            PlayerControl::Down => self.move_down = name,
            PlayerControl::Left => self.move_left = name,
            PlayerControl::Right => self.move_right = name,
            PlayerControl::Dash => self.dash = name,
        }
    }

    fn key_name(&self, control: PlayerControl) -> &str {
        match control {
            PlayerControl::Up => &self.move_up,
            PlayerControl::Down => &self.move_down,
            PlayerControl::Left => &self.move_left,
            PlayerControl::Right => &self.move_right,
            PlayerControl::Dash => &self.dash,
        }
    }
}

pub struct MainMenu {
    pub settings: PlayerSettings,
    editing_nickname: bool,
    show_controls: bool,
    rebinding: Option<PlayerControl>,
    show_multiplayer: bool,
    network_field: Option<NetworkField>,
    host_bind_address: String,
    host_match_minutes: String,
    host_address: String,
    notice: Option<String>,
}

impl MainMenu {
    pub fn load() -> Self {
        Self {
            settings: PlayerSettings::load(),
            editing_nickname: false,
            show_controls: false,
            rebinding: None,
            show_multiplayer: false,
            network_field: None,
            host_bind_address: "0.0.0.0:45678".to_string(),
            host_match_minutes: "17".to_string(),
            host_address: "26.0.0.1:45678".to_string(),
            notice: None,
        }
    }

    pub fn set_notice(&mut self, notice: String) {
        self.notice = Some(notice);
    }

    pub fn draw(&mut self, fonts: &GameFonts) -> Option<MenuAction> {
        if self.show_multiplayer {
            return self.draw_multiplayer(fonts);
        }
        let w = screen_width();
        let h = screen_height();
        let (mx, my) = mouse_position();
        clear_background(Color::new(0.035, 0.035, 0.065, 1.0));
        draw_circle(w * 0.5, h * 0.39, 250.0, Color::new(0.24, 0.06, 0.12, 0.16));
        draw_circle_lines(w * 0.5, h * 0.39, 250.0, 2.0, Color::new(0.75, 0.25, 0.20, 0.35));
        draw_circle_lines(w * 0.5, h * 0.39, 220.0, 1.0, Color::new(0.75, 0.55, 0.24, 0.22));

        let panel_w = 650.0f32.min(w - 40.0);
        let panel_h = 650.0f32.min(h - 36.0);
        let x = (w - panel_w) * 0.5;
        let y = (h - panel_h) * 0.5;
        draw_rectangle(x, y, panel_w, panel_h, Color::new(0.055, 0.06, 0.095, 0.97));
        draw_rectangle_lines(x, y, panel_w, panel_h, 2.0, Color::new(0.72, 0.45, 0.18, 0.9));
        draw_rectangle(x, y, panel_w, 8.0, Color::new(0.78, 0.17, 0.16, 0.9));

        let title = "REVERSE SURVIVOR";
        let title_w = fonts.measure_heading(title, 34.0).width;
        fonts.draw_heading(title, (w - title_w) * 0.5, y + 64.0, 34.0, Color::new(0.96, 0.78, 0.38, 1.0));
        let subtitle = "THE DIRECTOR  •  АРЕНА, КОТОРАЯ УЧИТСЯ";
        let subtitle_w = fonts.measure_heading(subtitle, 12.0).width;
        fonts.draw_heading(subtitle, (w - subtitle_w) * 0.5, y + 91.0, 12.0, Color::new(0.72, 0.72, 0.82, 0.95));

        let name_rect = Rect::new(x + 42.0, y + 116.0, panel_w - 84.0, 48.0);
        let name_hover = contains(name_rect, mx, my);
        draw_rectangle(name_rect.x, name_rect.y, name_rect.w, name_rect.h, Color::new(0.025, 0.03, 0.055, 1.0));
        draw_rectangle_lines(name_rect.x, name_rect.y, name_rect.w, name_rect.h, 1.3, if self.editing_nickname || name_hover { GOLD } else { GRAY });
        fonts.draw_heading("НИКНЕЙМ", name_rect.x + 12.0, name_rect.y - 4.0, 10.0, Color::new(0.72, 0.72, 0.82, 1.0));
        let cursor = if self.editing_nickname && (get_time() as u64 % 2 == 0) { "|" } else { "" };
        let name_text = format!("{}{}", self.settings.nickname, cursor);
        fonts.draw_heading(&name_text, name_rect.x + 14.0, name_rect.y + 31.0, 18.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && name_hover {
            self.editing_nickname = true;
            self.show_controls = false;
        } else if is_mouse_button_pressed(MouseButton::Left) && !name_hover {
            self.editing_nickname = false;
        }
        if self.editing_nickname {
            let mut changed = false;
            while let Some(ch) = get_char_pressed() {
                if !ch.is_control() && self.settings.nickname.chars().count() < 32 {
                    self.settings.nickname.push(ch);
                    changed = true;
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                self.settings.nickname.pop();
                changed = true;
            }
            if is_key_pressed(KeyCode::Enter) {
                self.editing_nickname = false;
            }
            if changed {
                self.settings.save();
            }
        }

        let skin_y = name_rect.y + 67.0;
        fonts.draw_heading("ОБРАЗ БОССА", x + 46.0, skin_y + 17.0, 12.0, LIGHTGRAY);
        let skin_names = ["Алый рыцарь", "Ледяной страж", "Лесной дух", "Сумрачный", "Золотой"];
        let skin_tints = skin_palette();
        let left = Rect::new(x + panel_w - 238.0, skin_y - 3.0, 38.0, 34.0);
        let right = Rect::new(x + panel_w - 54.0, skin_y - 3.0, 38.0, 34.0);
        let skin_box = Rect::new(x + panel_w - 194.0, skin_y - 3.0, 132.0, 34.0);
        draw_menu_button(left, "‹", contains(left, mx, my), fonts);
        draw_menu_button(right, "›", contains(right, mx, my), fonts);
        draw_rectangle(skin_box.x, skin_box.y, skin_box.w, skin_box.h, Color::new(0.025, 0.03, 0.055, 1.0));
        draw_rectangle_lines(skin_box.x, skin_box.y, skin_box.w, skin_box.h, 1.0, skin_tints[self.settings.skin as usize]);
        draw_circle(skin_box.x + 14.0, skin_box.y + 17.0, 7.0, skin_tints[self.settings.skin as usize]);
        let skin_name = skin_names[self.settings.skin as usize];
        let skin_text_w = fonts.measure_heading(skin_name, 10.0).width;
        fonts.draw_heading(skin_name, skin_box.x + 26.0 + ((skin_box.w - 30.0 - skin_text_w) * 0.5).max(0.0), skin_box.y + 21.0, 10.0, WHITE);
        if is_mouse_button_pressed(MouseButton::Left) && contains(left, mx, my) {
            self.settings.skin = (self.settings.skin + 4) % 5;
            self.settings.save();
        }
        if is_mouse_button_pressed(MouseButton::Left) && contains(right, mx, my) {
            self.settings.skin = (self.settings.skin + 1) % 5;
            self.settings.save();
        }

        let controls_rect = Rect::new(x + 42.0, skin_y + 48.0, panel_w - 84.0, 38.0);
        draw_menu_button(controls_rect, if self.show_controls { "УПРАВЛЕНИЕ  ▴" } else { "УПРАВЛЕНИЕ  ▾" }, contains(controls_rect, mx, my), fonts);
        if is_mouse_button_pressed(MouseButton::Left) && contains(controls_rect, mx, my) {
            self.show_controls = !self.show_controls;
            self.rebinding = None;
            self.editing_nickname = false;
        }

        let mut button_y = controls_rect.y + 56.0;
        if self.show_controls {
            let controls = [
                (PlayerControl::Up, "Движение вверх"),
                (PlayerControl::Down, "Движение вниз"),
                (PlayerControl::Left, "Движение влево"),
                (PlayerControl::Right, "Движение вправо"),
                (PlayerControl::Dash, "Рывок"),
            ];
            for (control, label) in controls {
                let row = Rect::new(x + 42.0, button_y, panel_w - 84.0, 30.0);
                let active = self.rebinding == Some(control);
                draw_rectangle(row.x, row.y, row.w, row.h, Color::new(0.035, 0.04, 0.07, 1.0));
                draw_rectangle_lines(row.x, row.y, row.w, row.h, 0.8, if active { GOLD } else { Color::new(0.25, 0.28, 0.36, 1.0) });
                fonts.draw_text(label, row.x + 12.0, row.y + 20.0, 12.0, LIGHTGRAY);
                let key_label = if active { "нажмите клавишу…" } else { self.settings.key_name(control) };
                let kw = fonts.measure_heading(key_label, 11.0).width;
                fonts.draw_heading(key_label, row.x + row.w - kw - 12.0, row.y + 20.0, 11.0, if active { GOLD } else { WHITE });
                if is_mouse_button_pressed(MouseButton::Left) && contains(row, mx, my) {
                    self.rebinding = Some(control);
                }
                button_y += 34.0;
            }
            if let Some(control) = self.rebinding {
                if let Some(key) = get_last_key_pressed() {
                    if key != KeyCode::Escape && key != KeyCode::Enter {
                        if parse_key_code(&format!("{:?}", key)).is_some() {
                            self.settings.set_key(control, key);
                            self.settings.save();
                            self.rebinding = None;
                        }
                    } else if key == KeyCode::Escape {
                        self.rebinding = None;
                    }
                }
            }
            button_y += 4.0;
        }

        let director_rect = Rect::new(x + 42.0, button_y, panel_w - 84.0, 54.0);
        let boss_rect = Rect::new(x + 42.0, button_y + 65.0, panel_w - 84.0, 54.0);
        let multiplayer_rect = Rect::new(x + 42.0, button_y + 130.0, panel_w - 84.0, 48.0);
        draw_menu_button(director_rect, "ИГРАТЬ ЗА ДИРЕКТОРА  ·  ПРОТИВ БОССА ИИ", contains(director_rect, mx, my), fonts);
        draw_menu_button(boss_rect, "ИГРАТЬ ЗА БОССА  ·  ПРОТИВ ДИРЕКТОРА ИИ", contains(boss_rect, mx, my), fonts);
        draw_menu_button(multiplayer_rect, "СЕТЕВАЯ ИГРА  ·  СОЗДАТЬ / ПОДКЛЮЧИТЬСЯ", contains(multiplayer_rect, mx, my), fonts);
        if !self.show_controls {
            fonts.draw_text("Директор и босс используют сохранённые обученные модели.", x + 44.0, y + panel_h - 14.0, 10.5, Color::new(0.62, 0.64, 0.72, 0.9));
        }
        if is_mouse_button_pressed(MouseButton::Left) && contains(director_rect, mx, my) {
            if self.settings.nickname.trim().is_empty() {
                self.settings.nickname = "Игрок".to_string();
            }
            self.settings.save();
            return Some(MenuAction::Local(GameRole::Director));
        }
        if is_mouse_button_pressed(MouseButton::Left) && contains(boss_rect, mx, my) {
            if self.settings.nickname.trim().is_empty() {
                self.settings.nickname = "Игрок".to_string();
            }
            self.settings.save();
            return Some(MenuAction::Local(GameRole::Boss));
        }
        if is_mouse_button_pressed(MouseButton::Left) && contains(multiplayer_rect, mx, my) {
            self.show_multiplayer = true;
            self.notice = None;
            self.network_field = None;
        }
        None
    }

    fn draw_multiplayer(&mut self, fonts: &GameFonts) -> Option<MenuAction> {
        let w = screen_width();
        let h = screen_height();
        let (mx, my) = mouse_position();
        clear_background(Color::new(0.035, 0.035, 0.065, 1.0));

        let panel_w = 650.0f32.min(w - 40.0);
        let panel_h = 620.0f32.min(h - 36.0);
        let x = (w - panel_w) * 0.5;
        let y = (h - panel_h) * 0.5;
        draw_rectangle(x, y, panel_w, panel_h, Color::new(0.055, 0.06, 0.095, 0.97));
        draw_rectangle_lines(x, y, panel_w, panel_h, 2.0, Color::new(0.72, 0.45, 0.18, 0.9));
        draw_rectangle(x, y, panel_w, 8.0, Color::new(0.78, 0.17, 0.16, 0.9));

        let title = "СЕТЕВАЯ ИГРА";
        let title_w = fonts.measure_heading(title, 29.0).width;
        fonts.draw_heading(title, (w - title_w) * 0.5, y + 52.0, 29.0, GOLD);
        fonts.draw_text(
            &format!("Игрок: {}  ·  Образ {}", self.settings.nickname, self.settings.skin + 1),
            x + 42.0,
            y + 83.0,
            12.0,
            LIGHTGRAY,
        );

        let field_x = x + 42.0;
        let field_w = panel_w - 84.0;
        let bind_rect = Rect::new(field_x, y + 111.0, field_w, 40.0);
        let minutes_rect = Rect::new(field_x, y + 175.0, field_w, 40.0);
        let address_rect = Rect::new(field_x, y + 382.0, field_w, 40.0);
        let create_rect = Rect::new(field_x, y + 235.0, field_w, 54.0);
        let join_rect = Rect::new(field_x, y + 440.0, field_w, 54.0);
        let back_rect = Rect::new(field_x, y + panel_h - 66.0, field_w, 44.0);

        draw_menu_input(
            bind_rect,
            "АДРЕС И ПОРТ ХОСТА",
            &self.host_bind_address,
            self.network_field == Some(NetworkField::HostBind),
            mx,
            my,
            fonts,
        );
        draw_menu_input(
            minutes_rect,
            "ТАЙМЕР МАТЧА, МИНУТ (1–180)",
            &self.host_match_minutes,
            self.network_field == Some(NetworkField::MatchMinutes),
            mx,
            my,
            fonts,
        );
        draw_menu_button(create_rect, "СОЗДАТЬ ИГРУ ЗА ДИРЕКТОРА", contains(create_rect, mx, my), fonts);

        fonts.draw_text(
            "Игроки подключаются по RadminVPN IP. Хост управляет Директором.",
            field_x,
            y + 325.0,
            11.0,
            Color::new(0.68, 0.70, 0.79, 0.95),
        );
        draw_menu_input(
            address_rect,
            "АДРЕС ХОСТА В RADMINVPN",
            &self.host_address,
            self.network_field == Some(NetworkField::HostAddress),
            mx,
            my,
            fonts,
        );
        draw_menu_button(join_rect, "ПОДКЛЮЧИТЬСЯ ЗА БОССА", contains(join_rect, mx, my), fonts);

        if let Some(notice) = &self.notice {
            fonts.draw_text(notice, field_x, y + panel_h - 78.0, 11.0, Color::new(1.0, 0.45, 0.35, 1.0));
        }
        draw_menu_button(back_rect, "НАЗАД", contains(back_rect, mx, my), fonts);

        for (rect, field) in [
            (bind_rect, NetworkField::HostBind),
            (minutes_rect, NetworkField::MatchMinutes),
            (address_rect, NetworkField::HostAddress),
        ] {
            if is_mouse_button_pressed(MouseButton::Left) && contains(rect, mx, my) {
                self.network_field = Some(field);
            }
        }
        if let Some(field) = self.network_field {
            let value = match field {
                NetworkField::HostBind => &mut self.host_bind_address,
                NetworkField::MatchMinutes => &mut self.host_match_minutes,
                NetworkField::HostAddress => &mut self.host_address,
            };
            let max_len = if field == NetworkField::MatchMinutes { 4 } else { 64 };
            while let Some(ch) = get_char_pressed() {
                let allowed = match field {
                    NetworkField::HostBind | NetworkField::HostAddress => ch.is_ascii_alphanumeric() || ".:-[]".contains(ch),
                    NetworkField::MatchMinutes => ch.is_ascii_digit() || ch == '.',
                };
                if allowed && value.len() < max_len {
                    value.push(ch);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                value.pop();
            }
            if is_key_pressed(KeyCode::Enter) {
                self.network_field = None;
            }
        }

        if is_mouse_button_pressed(MouseButton::Left) && contains(back_rect, mx, my) {
            self.show_multiplayer = false;
            self.network_field = None;
            self.notice = None;
        }
        if is_mouse_button_pressed(MouseButton::Left) && contains(create_rect, mx, my) {
            let duration_minutes = self.host_match_minutes.trim().parse::<f32>().ok();
            if let Some(minutes) = duration_minutes.filter(|minutes| (1.0..=180.0).contains(minutes)) {
                self.settings.save();
                return Some(MenuAction::Host {
                    bind_address: self.host_bind_address.trim().to_string(),
                    duration_secs: minutes * 60.0,
                });
            }
            self.notice = Some("Укажи длительность матча от 1 до 180 минут.".to_string());
        }
        if is_mouse_button_pressed(MouseButton::Left) && contains(join_rect, mx, my) {
            self.settings.save();
            return Some(MenuAction::Join { address: self.host_address.trim().to_string() });
        }
        None
    }
}

pub fn skin_tint(skin: u8) -> Color {
    skin_palette()[(skin % 5) as usize]
}

fn skin_palette() -> [Color; 5] {
    [
        Color::new(1.0, 1.0, 1.0, 1.0),
        Color::new(0.65, 0.82, 1.0, 1.0),
        Color::new(0.68, 1.0, 0.76, 1.0),
        Color::new(0.82, 0.68, 1.0, 1.0),
        Color::new(1.0, 0.84, 0.50, 1.0),
    ]
}

fn draw_menu_input(
    rect: Rect,
    label: &str,
    value: &str,
    active: bool,
    mx: f32,
    my: f32,
    fonts: &GameFonts,
) {
    let hovered = contains(rect, mx, my);
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, Color::new(0.025, 0.03, 0.055, 1.0));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.2, if active || hovered { GOLD } else { GRAY });
    fonts.draw_heading(label, rect.x + 12.0, rect.y - 5.0, 9.5, Color::new(0.72, 0.72, 0.82, 1.0));
    let cursor = if active && (get_time() as u64 % 2 == 0) { "|" } else { "" };
    fonts.draw_heading(&format!("{value}{cursor}"), rect.x + 12.0, rect.y + 26.0, 14.0, WHITE);
}

fn parse_key_code(name: &str) -> Option<KeyCode> {
    Some(match name {
        "A" => KeyCode::A, "B" => KeyCode::B, "C" => KeyCode::C, "D" => KeyCode::D,
        "E" => KeyCode::E, "F" => KeyCode::F, "G" => KeyCode::G, "H" => KeyCode::H,
        "I" => KeyCode::I, "J" => KeyCode::J, "K" => KeyCode::K, "L" => KeyCode::L,
        "M" => KeyCode::M, "N" => KeyCode::N, "O" => KeyCode::O, "P" => KeyCode::P,
        "Q" => KeyCode::Q, "R" => KeyCode::R, "S" => KeyCode::S, "T" => KeyCode::T,
        "U" => KeyCode::U, "V" => KeyCode::V, "W" => KeyCode::W, "X" => KeyCode::X,
        "Y" => KeyCode::Y, "Z" => KeyCode::Z,
        "Space" => KeyCode::Space, "Enter" => KeyCode::Enter, "Tab" => KeyCode::Tab,
        "Left" => KeyCode::Left, "Right" => KeyCode::Right, "Up" => KeyCode::Up, "Down" => KeyCode::Down,
        "LeftShift" => KeyCode::LeftShift, "RightShift" => KeyCode::RightShift,
        "LeftControl" => KeyCode::LeftControl, "RightControl" => KeyCode::RightControl,
        "LeftAlt" => KeyCode::LeftAlt, "RightAlt" => KeyCode::RightAlt,
        "Key0" => KeyCode::Key0, "Key1" => KeyCode::Key1, "Key2" => KeyCode::Key2,
        "Key3" => KeyCode::Key3, "Key4" => KeyCode::Key4, "Key5" => KeyCode::Key5,
        "Key6" => KeyCode::Key6, "Key7" => KeyCode::Key7, "Key8" => KeyCode::Key8,
        "Key9" => KeyCode::Key9,
        "Minus" => KeyCode::Minus, "Equal" => KeyCode::Equal, "Apostrophe" => KeyCode::Apostrophe,
        "Comma" => KeyCode::Comma, "Period" => KeyCode::Period, "Slash" => KeyCode::Slash,
        "Semicolon" => KeyCode::Semicolon, "LeftBracket" => KeyCode::LeftBracket,
        "RightBracket" => KeyCode::RightBracket, "GraveAccent" => KeyCode::GraveAccent,
        "F1" => KeyCode::F1, "F2" => KeyCode::F2, "F3" => KeyCode::F3, "F4" => KeyCode::F4,
        "F5" => KeyCode::F5, "F6" => KeyCode::F6, "F7" => KeyCode::F7, "F8" => KeyCode::F8,
        "F9" => KeyCode::F9, "F10" => KeyCode::F10, "F11" => KeyCode::F11, "F12" => KeyCode::F12,
        _ => return None,
    })
}

fn contains(rect: Rect, x: f32, y: f32) -> bool {
    x >= rect.x && x <= rect.x + rect.w && y >= rect.y && y <= rect.y + rect.h
}

fn draw_menu_button(rect: Rect, label: &str, hover: bool, fonts: &GameFonts) {
    let fill = if hover { Color::new(0.28, 0.13, 0.11, 1.0) } else { Color::new(0.09, 0.10, 0.15, 1.0) };
    let edge = if hover { GOLD } else { Color::new(0.30, 0.34, 0.43, 1.0) };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.2, edge);
    let size = if rect.h > 45.0 { 14.0 } else { 11.0 };
    let tw = fonts.measure_heading(label, size).width;
    fonts.draw_heading(label, rect.x + (rect.w - tw) * 0.5, rect.y + rect.h * 0.62, size, if hover { GOLD } else { WHITE });
}
