use crossterm::style::Color;

use crate::editor::annotatedstring::AnnotationType;

pub struct Attribute {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
}

impl From<AnnotationType> for Attribute {
    fn from(annotation_type: AnnotationType) -> Self {
        match annotation_type {
            AnnotationType::Match => Self {
                foreground: Some(Color::Rgb { r: 0, g: 0, b: 0 }),
                background: Some(Color::Rgb {
                    r: 255,
                    g: 251,
                    b: 0,
                }),
            },
            AnnotationType::SelectedMatch => Self {
                foreground: Some(Color::Rgb { r: 0, g: 0, b: 0 }),
                background: Some(Color::Rgb {
                    r: 255,
                    g: 165,
                    b: 0,
                }),
            },
        }
    }
}
