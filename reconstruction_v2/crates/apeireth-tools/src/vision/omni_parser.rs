use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UiElementType {
    Button,
    InputBox,
    Text,
    Icon,
    MenuItem,
    Link,
    Checkbox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElement {
    pub id: u32,
    pub element_type: UiElementType,
    pub label: String,
    pub bbox: [f32; 4], // [x, y, width, height] normalized (0.0 - 1.0)
    pub is_interactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniParsedScreen {
    pub elements: Vec<UiElement>,
    pub som_markup_text: String,
    pub screen_width: u32,
    pub screen_height: u32,
}

pub struct OmniParser;

impl OmniParser {
    pub fn new() -> Self {
        Self
    }

    /// Formats detected UI elements into Set-of-Marks (SoM) token representation for LLMs
    pub fn format_som_representation(elements: &[UiElement], width: u32, height: u32) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Screen Size: {}x{}\n", width, height));
        out.push_str("## Interactive Set-of-Marks (SoM) Elements:\n");

        for el in elements {
            let px = (el.bbox[0] * width as f32).round() as u32;
            let py = (el.bbox[1] * height as f32).round() as u32;
            let pw = (el.bbox[2] * width as f32).round() as u32;
            let ph = (el.bbox[3] * height as f32).round() as u32;

            out.push_str(&format!(
                "- [#{}] {:?}: \"{}\" at ({}, {}, {}x{}) [Interactive: {}]\n",
                el.id, el.element_type, el.label, px, py, pw, ph, el.is_interactive
            ));
        }

        out
    }

    /// Resolves SoM mark ID (e.g. #3) into physical screen pixel center coordinates (px, py)
    pub fn resolve_mark_center(element: &UiElement, width: u32, height: u32) -> (u32, u32) {
        let cx = ((element.bbox[0] + element.bbox[2] / 2.0) * width as f32).round() as u32;
        let cy = ((element.bbox[1] + element.bbox[3] / 2.0) * height as f32).round() as u32;
        (cx, cy)
    }

    /// Parses raw visual detection boxes and generates OmniParsedScreen
    pub fn parse_screen(elements: Vec<UiElement>, width: u32, height: u32) -> OmniParsedScreen {
        let markup = Self::format_som_representation(&elements, width, height);
        OmniParsedScreen {
            elements,
            som_markup_text: markup,
            screen_width: width,
            screen_height: height,
        }
    }
}
