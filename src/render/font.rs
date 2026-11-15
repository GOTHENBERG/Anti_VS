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
}
