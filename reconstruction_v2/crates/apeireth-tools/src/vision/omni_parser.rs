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

    /// Detects real interactive windows on Windows desktop
    #[cfg(target_os = "windows")]
    pub fn detect_live_elements(width: u32, height: u32) -> Vec<UiElement> {
        use winapi::shared::minwindef::{BOOL, LPARAM, TRUE};
        use winapi::shared::windef::{HWND, RECT};
        use winapi::um::winuser::{EnumWindows, GetWindowRect, GetWindowTextW, IsWindowVisible};

        struct EnumCtx {
            elements: Vec<UiElement>,
            width: u32,
            height: u32,
            id_counter: u32,
        }

        unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam as *mut EnumCtx);
            if IsWindowVisible(hwnd) != 0 {
                let mut rect: RECT = std::mem::zeroed();
                if GetWindowRect(hwnd, &mut rect) != 0 {
                    let w = rect.right - rect.left;
                    let h = rect.bottom - rect.top;
                    if w > 40 && h > 40 && rect.left >= 0 && rect.top >= 0 {
                        let mut title_buf = [0u16; 256];
                        let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256);
                        let title = if len > 0 {
                            String::from_utf16_lossy(&title_buf[..len as usize])
                        } else {
                            "Desktop Control".into()
                        };

                        if !title.trim().is_empty() {
                            let x_norm = rect.left as f32 / ctx.width.max(1) as f32;
                            let y_norm = rect.top as f32 / ctx.height.max(1) as f32;
                            let w_norm = w as f32 / ctx.width.max(1) as f32;
                            let h_norm = h as f32 / ctx.height.max(1) as f32;

                            ctx.id_counter += 1;
                            ctx.elements.push(UiElement {
                                id: ctx.id_counter,
                                element_type: UiElementType::Button,
                                label: title,
                                bbox: [x_norm, y_norm, w_norm, h_norm],
                                is_interactive: true,
                            });
                        }
                    }
                }
            }
            TRUE
        }

        let mut ctx = EnumCtx {
            elements: Vec::new(),
            width,
            height,
            id_counter: 0,
        };

        unsafe {
            EnumWindows(Some(enum_proc), &mut ctx as *mut _ as LPARAM);
        }

        ctx.elements
    }

    #[cfg(not(target_os = "windows"))]
    pub fn detect_live_elements(_width: u32, _height: u32) -> Vec<UiElement> {
        Vec::new()
    }
}

