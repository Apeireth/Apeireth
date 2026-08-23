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

    /// Detects real interactive windows and controls on Windows desktop
    #[cfg(target_os = "windows")]
    pub fn detect_live_elements(width: u32, height: u32) -> Vec<UiElement> {
        use winapi::shared::minwindef::{BOOL, FALSE, LPARAM, TRUE};
        use winapi::shared::windef::{HWND, RECT};
        use winapi::um::winuser::{
            CloseDesktop, CloseWindowStation, EnumChildWindows, EnumDesktopWindows, EnumWindows,
            GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW, IsWindowVisible,
            OpenDesktopW, OpenWindowStationW, SetProcessWindowStation, WINSTA_ALL_ACCESS,
        };

        struct EnumCtx {
            elements: Vec<UiElement>,
            width: u32,
            height: u32,
            id_counter: usize,
        }

        fn map_class_to_type(class_name: &str) -> (UiElementType, bool) {
            let lower = class_name.to_lowercase();
            if lower.contains("button") {
                (UiElementType::Button, true)
            } else if lower.contains("edit") || lower.contains("textbox") || lower.contains("richedit") {
                (UiElementType::InputBox, true)
            } else if lower.contains("check") {
                (UiElementType::Checkbox, true)
            } else if lower.contains("menu") {
                (UiElementType::MenuItem, true)
            } else if lower.contains("link") || lower.contains("hyperlink") {
                (UiElementType::Link, true)
            } else if lower.contains("static") || lower.contains("label") {
                (UiElementType::Text, false)
            } else {
                (UiElementType::Button, true)
            }
        }

        unsafe extern "system" fn inspect_hwnd(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let ctx = &mut *(lparam as *mut EnumCtx);
            if IsWindowVisible(hwnd) != 0 {
                let mut rect: RECT = std::mem::zeroed();
                if GetWindowRect(hwnd, &mut rect) != 0 {
                    let w = rect.right - rect.left;
                    let h = rect.bottom - rect.top;
                    if w > 10 && h > 10 && rect.left >= -100 && rect.top >= -100 {
                        let mut title_buf = [0u16; 256];
                        let len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), 256);
                        let title = if len > 0 {
                            String::from_utf16_lossy(&title_buf[..len as usize])
                        } else {
                            String::new()
                        };

                        let mut class_buf = [0u16; 128];
                        let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 128);
                        let class_name = if class_len > 0 {
                            String::from_utf16_lossy(&class_buf[..class_len as usize])
                        } else {
                            "Window".into()
                        };

                        let (elem_type, is_interactive) = map_class_to_type(&class_name);
                        let label = if !title.trim().is_empty() {
                            title.trim().to_string()
                        } else if is_interactive && !class_name.is_empty() && class_name != "Default IME" && class_name != "MSCTFIME UI" {
                            format!("[{}]", class_name)
                        } else {
                            String::new()
                        };

                        if !label.is_empty() && rect.right > 0 && rect.bottom > 0 {
                            let x_norm = (rect.left.max(0) as f32) / ctx.width.max(1) as f32;
                            let y_norm = (rect.top.max(0) as f32) / ctx.height.max(1) as f32;
                            let w_norm = (w as f32) / ctx.width.max(1) as f32;
                            let h_norm = (h as f32) / ctx.height.max(1) as f32;

                            ctx.id_counter += 1;
                            ctx.elements.push(UiElement {
                                id: ctx.id_counter as u32,
                                element_type: elem_type,
                                label,
                                bbox: [x_norm, y_norm, w_norm, h_norm],
                                is_interactive,
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
            // First inspect foreground active window child controls
            let fg = GetForegroundWindow();
            if !fg.is_null() {
                inspect_hwnd(fg, &mut ctx as *mut _ as LPARAM);
                EnumChildWindows(fg, Some(inspect_hwnd), &mut ctx as *mut _ as LPARAM);
            }

            // Enumerate interactive user desktop windows (WinSta0\Default)
            let winsta_name: Vec<u16> = "WinSta0\0".encode_utf16().collect();
            let winsta = OpenWindowStationW(winsta_name.as_ptr(), FALSE, WINSTA_ALL_ACCESS);
            if !winsta.is_null() {
                SetProcessWindowStation(winsta);
                let desk_name: Vec<u16> = "Default\0".encode_utf16().collect();
                let desk = OpenDesktopW(desk_name.as_ptr(), 0, FALSE, 0x01FF);
                if !desk.is_null() {
                    EnumDesktopWindows(desk, Some(inspect_hwnd), &mut ctx as *mut _ as LPARAM);
                    CloseDesktop(desk);
                }
                CloseWindowStation(winsta);
            } else {
                EnumWindows(Some(inspect_hwnd), &mut ctx as *mut _ as LPARAM);
            }
        }

        ctx.elements.truncate(50);
        ctx.elements
    }

    #[cfg(not(target_os = "windows"))]
    pub fn detect_live_elements(_width: u32, _height: u32) -> Vec<UiElement> {
        Vec::new()
    }
}

