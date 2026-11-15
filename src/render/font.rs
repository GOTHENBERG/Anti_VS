use macroquad::prelude::*;

pub struct GameFonts {
    pub heading_font: Option<Font>,
    pub text_font: Option<Font>,
}

impl GameFonts {
    pub fn load() -> Self {
        // 1. Попытка загрузить стилизованные шрифты из assets/fonts/
        let mut heading_font = None;
        let mut text_font = None;

        let heading_paths = [
            "assets/fonts/RussoOne-Regular.ttf",
            "assets/fonts/Rubik-Bold.ttf",
            "C:/Windows/Fonts/bahnschrift.ttf",
            "C:/Windows/Fonts/arialbd.ttf",
        ];

        let text_paths = [
            "assets/fonts/Rubik-Regular.ttf",
            "assets/fonts/Rubik-Bold.ttf",
            "assets/fonts/RussoOne-Regular.ttf",
            "C:/Windows/Fonts/segoeui.ttf",
            "C:/Windows/Fonts/calibri.ttf",
            "C:/Windows/Fonts/arial.ttf",
        ];

        for path in heading_paths {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = load_ttf_font_from_bytes(&bytes) {
                    heading_font = Some(font);
                    break;
                }
            }
        }

        for path in text_paths {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = load_ttf_font_from_bytes(&bytes) {
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

        Self {
            heading_font,
            text_font,
        }
    }

    pub fn draw_text(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        let font_ref = self.text_font.as_ref().or(self.heading_font.as_ref());
        if let Some(font) = font_ref {
            draw_text_ex(
                text,
                x,
                y,
                TextParams {
                    font: Some(font),
                    font_size: size.round() as u16,
                    color,
                    ..Default::default()
                },
            );
        } else {
            draw_text(text, x, y, size, color);
        }
    }

    pub fn draw_heading(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        let font_ref = self.heading_font.as_ref().or(self.text_font.as_ref());
        if let Some(font) = font_ref {
            draw_text_ex(
                text,
                x,
                y,
                TextParams {
                    font: Some(font),
                    font_size: size.round() as u16,
                    color,
                    ..Default::default()
                },
            );
        } else {
            draw_text(text, x, y, size, color);
        }
    }

    pub fn measure_text(&self, text: &str, size: f32) -> TextDimensions {
        let font_ref = self.text_font.as_ref().or(self.heading_font.as_ref());
        if let Some(font) = font_ref {
            measure_text(text, Some(font), size.round() as u16, 1.0)
        } else {
            measure_text(text, None, size.round() as u16, 1.0)
        }
    }

    pub fn measure_heading(&self, text: &str, size: f32) -> TextDimensions {
        let font_ref = self.heading_font.as_ref().or(self.text_font.as_ref());
        if let Some(font) = font_ref {
            measure_text(text, Some(font), size.round() as u16, 1.0)
        } else {
            measure_text(text, None, size.round() as u16, 1.0)
        }
    }

    /// Разбивает текст на строки так, чтобы ни одна строка не превышала max_width
    pub fn wrap_text(&self, text: &str, size: f32, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in text.split('\n') {
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
