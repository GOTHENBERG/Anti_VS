use macroquad::prelude::*;

pub struct GameFonts {
    pub font: Option<Font>,
}

impl GameFonts {
    pub fn load() -> Self {
        let font_paths = [
            "C:/Windows/Fonts/segoeui.ttf",
            "C:/Windows/Fonts/segoeuib.ttf",
            "C:/Windows/Fonts/arial.ttf",
            "C:/Windows/Fonts/arialbd.ttf",
            "C:/Windows/Fonts/calibri.ttf",
        ];

        let mut loaded_font = None;
        for path in font_paths {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(font) = load_ttf_font_from_bytes(&bytes) {
                    loaded_font = Some(font);
                    break;
                }
            }
        }

        Self { font: loaded_font }
    }

    pub fn draw_text(&self, text: &str, x: f32, y: f32, size: f32, color: Color) {
        if let Some(ref font) = self.font {
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
        if let Some(ref font) = self.font {
            measure_text(text, Some(font), size.round() as u16, 1.0)
        } else {
            measure_text(text, None, size.round() as u16, 1.0)
        }
    }
}
