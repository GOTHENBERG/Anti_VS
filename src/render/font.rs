use macroquad::prelude::*;

pub const BASE_FONT_SIZE: f32 = 32.0;

fn sanitize_text(text: &str) -> String {
    // If text only contains standard chars, avoid unnecessary allocations
    if text.chars().all(|c| {
        c < '\u{2000}'
            || c == '•'
            || c == '—'
            || c == '–'
            || c == '«'
            || c == '»'
            || c == '…'
            || c == '№'
    }) {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{fe0f}' => {} // strip emoji variation selector
            '➔' | '→' => out.push_str("->"),
            '▶' => out.push('>'),
            '◀' => out.push('<'),
            '▲' | '▴' => out.push('^'),
            '▼' | '▾' => out.push('v'),
            '⏸' => out.push_str("||"),
            '⟲' | '🔄' => out.push_str("↺"),
            '⚙' => out.push_str("[*]"),
            '⚡' => out.push('!'),
            '⭐' => out.push('*'),
            '⚔' => out.push_str("X"),
            '🛡' => out.push_str("[#]"),
            '👑' => out.push_str("[*]"),
            '🔥' => out.push('*'),
            '✨' => out.push('*'),
            '🔮' => out.push('*'),
            '💡' => out.push('*'),
            '📈' => out.push('+'),
            '📖' | '📜' => out.push_str("[i]"),
            '⏱' => out.push_str("T:"),
            '🔊' | '🔈' | '🔉' => out.push_str("))"),
            '🔇' => out.push_str("[X]"),
            '🎵' | '🎶' => out.push('~'),
            '🔀' => out.push('~'),
            '🎮' => {}
            '🚪' => {}
            '🏛' => {}
            '🧱' => {}
            '⚠' => out.push_str("[!]"),
            '🔒' => out.push_str("[L]"),
            '↳' => out.push_str("->"),
            // Filter out SMP emoji characters (> 0xFFFF) that cause missing glyph boxes
            c if (c as u32) >= 0x1F000 => {}
            _ => out.push(c),
        }
    }
    out
}

pub struct GameFonts {
    pub heading_font: Option<Font>,
    pub text_font: Option<Font>,
}

impl GameFonts {
    pub fn load() -> Self {
        // Приоритетная загрузка согласованной пары шрифтов (Bold для заголовков, Regular для текста)
        let heading_paths = [
            "assets/fonts/GameFont-Bold.ttf",
            "assets/fonts/Rubik-Bold.ttf",
            "C:/Windows/Fonts/segoeuib.ttf",
            "C:/Windows/Fonts/bahnschrift.ttf",
            "C:/Windows/Fonts/arialbd.ttf",
            "assets/fonts/RussoOne-Regular.ttf",
        ];

        let text_paths = [
            "assets/fonts/GameFont-Regular.ttf",
            "assets/fonts/Rubik-Regular.ttf",
            "assets/fonts/Rubik.ttf",
            "C:/Windows/Fonts/segoeui.ttf",
            "C:/Windows/Fonts/calibri.ttf",
            "C:/Windows/Fonts/arial.ttf",
            "assets/fonts/RussoOne-Regular.ttf",
        ];

        let mut heading_font = None;
        let mut text_font = None;

        for path in heading_paths {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(mut font) = load_ttf_font_from_bytes(&bytes) {
                    font.set_filter(miniquad::FilterMode::Linear);
                    heading_font = Some(font);
                    break;
                }
            }
        }

        for path in text_paths {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(mut font) = load_ttf_font_from_bytes(&bytes) {
                    font.set_filter(miniquad::FilterMode::Linear);
                    text_font = Some(font);
                    break;
                }
            }
        }

        // Если один не загрузился, используем второй
        if heading_font.is_none() {
            heading_font = text_font.clone();
        }
        if text_font.is_none() {
            text_font = heading_font.clone();
        }

        // Предварительный прогрев кэша атласа (все глифы рендерятся ровно 1 раз на базовом размере)
        let mut warmup_chars: Vec<char> = Font::latin_character_list();
        let cyrillic_and_symbols = "абвгдеёжзийклмнопрстуфхцчшщъыьэюяАБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ 0123456789%:-+.,!?/\\()[]{}<>_=#*~|^$\"'`№@&•—–«»…";
        for c in cyrillic_and_symbols.chars() {
            warmup_chars.push(c);
        }

        let base_size_u16 = BASE_FONT_SIZE as u16;
        if let Some(ref font) = heading_font {
            font.populate_font_cache(&warmup_chars, base_size_u16);
        }
        if let Some(ref font) = text_font {
            font.populate_font_cache(&warmup_chars, base_size_u16);
        }

        Self {
            heading_font,
            text_font,
        }
    }

    pub fn draw_text(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        let clean = sanitize_text(text);
        let font_ref = self.text_font.as_ref().or(self.heading_font.as_ref());
        if let Some(font) = font_ref {
            let scale = size / BASE_FONT_SIZE;
            draw_text_ex(
                &clean,
                x,
                y,
                TextParams {
                    font: Some(font),
                    font_size: BASE_FONT_SIZE as u16,
                    font_scale: scale,
                    font_scale_aspect: 1.0,
                    color,
                    ..Default::default()
                },
            );
        } else {
            draw_text(&clean, x, y, size, color);
        }
    }

    pub fn draw_heading(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        let clean = sanitize_text(text);
        let font_ref = self.heading_font.as_ref().or(self.text_font.as_ref());
        if let Some(font) = font_ref {
            let scale = size / BASE_FONT_SIZE;
            draw_text_ex(
                &clean,
                x,
                y,
                TextParams {
                    font: Some(font),
                    font_size: BASE_FONT_SIZE as u16,
                    font_scale: scale,
                    font_scale_aspect: 1.0,
                    color,
                    ..Default::default()
                },
            );
        } else {
            draw_text(&clean, x, y, size, color);
        }
    }

    pub fn measure_text(&self, text: &str, size: f32) -> TextDimensions {
        let clean = sanitize_text(text);
        let font_ref = self.text_font.as_ref().or(self.heading_font.as_ref());
        if let Some(font) = font_ref {
            let scale = size / BASE_FONT_SIZE;
            measure_text(&clean, Some(font), BASE_FONT_SIZE as u16, scale)
        } else {
            measure_text(&clean, None, size.round() as u16, 1.0)
        }
    }

    pub fn measure_heading(&self, text: &str, size: f32) -> TextDimensions {
        let clean = sanitize_text(text);
        let font_ref = self.heading_font.as_ref().or(self.text_font.as_ref());
        if let Some(font) = font_ref {
            let scale = size / BASE_FONT_SIZE;
            measure_text(&clean, Some(font), BASE_FONT_SIZE as u16, scale)
        } else {
            measure_text(&clean, None, size.round() as u16, 1.0)
        }
    }

    /// Разбивает текст на строки так, чтобы ни одна строка не превышала max_width
    pub fn wrap_text(&self, text: &str, size: f32, max_width: f32) -> Vec<String> {
        let clean = sanitize_text(text);
        let mut lines = Vec::new();
        for paragraph in clean.split('\n') {
            let mut current_line = String::new();
            for word in paragraph.split_whitespace() {
                let test_line = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{} {}", current_line, word)
                };
                let w = self.measure_text(&test_line, size).width;
                if w <= max_width {
                    current_line = test_line;
                } else {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = word.to_string();
                    } else {
                        // Слово длиннее всей строки - добавляем как есть
                        lines.push(word.to_string());
                        current_line.clear();
                    }
                }
            }
            if !current_line.is_empty() {
                lines.push(current_line);
            }
        }
        lines
    }

    /// Отрисовывает текст с автоматическим переносом по словам и возвращает конечную y-координату
    pub fn draw_text_wrapped(
        &self,
        text: &str,
        x: f32,
        mut y: f32,
        size: f32,
        line_spacing: f32,
        max_width: f32,
        color: Color,
    ) -> f32 {
        let lines = self.wrap_text(text, size, max_width);
        for line in &lines {
            self.draw_text(line, x, y, size, color);
            y += line_spacing;
        }
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_assets_valid_ttf() {
        // Проверяем, что в assets/fonts нет поврежденных HTML файлов
        let fonts = [
            "assets/fonts/GameFont-Bold.ttf",
            "assets/fonts/GameFont-Regular.ttf",
            "assets/fonts/Rubik-Bold.ttf",
            "assets/fonts/Rubik-Regular.ttf",
        ];

        for path in fonts {
            let bytes = std::fs::read(path).unwrap_or_else(|_| panic!("Шрифт должен существовать: {}", path));
            assert!(bytes.len() > 10000, "Файл шрифта {} слишком мал ({} байт)", path, bytes.len());

            // Проверяем сигнатуру TrueType / OpenType: 0x00010000 или 'OTTO'
            let is_ttf = bytes.starts_with(&[0x00, 0x01, 0x00, 0x00]) || bytes.starts_with(b"OTTO");
            assert!(is_ttf, "Файл {} не является валидным TTF/OTF шрифтом!", path);

            // Проверяем, что файл точно не является HTML-заглушкой
            let header_str = String::from_utf8_lossy(&bytes[..bytes.len().min(128)]);
            assert!(!header_str.contains("<!DOCTYPE"), "Файл {} является HTML-страницей вместо шрифта!", path);
        }
    }

    #[test]
    fn test_sanitize_text_filters_emojis_and_keeps_cyrillic() {
        let input = "⚙ НАСТРОЙКИ ЗВУКА: 100% | ⚔️ БОЙ ➔ ПОБЕДА! • Привет мир";
        let cleaned = sanitize_text(input);

        // Должен сохранить кириллицу, проценты, двоеточие, буллеты
        assert!(cleaned.contains("НАСТРОЙКИ ЗВУКА: 100%"));
        assert!(cleaned.contains("• Привет мир"));

        // Должен заменить стрелку и спецсимволы
        assert!(cleaned.contains("->"));
        assert!(!cleaned.contains('➔'));
        assert!(!cleaned.contains('\u{fe0f}'));
    }
}
