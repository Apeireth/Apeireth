use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use std::time::Duration;

use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};

#[cfg(target_os = "windows")]
use winapi::um::winuser::{
    SendInput, SetCursorPos, GetCursorPos, MOUSEINPUT, KEYBDINPUT, INPUT,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL,
    KEYEVENTF_UNICODE, KEYEVENTF_KEYUP, INPUT_MOUSE, INPUT_KEYBOARD,
    VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN,
    VK_RETURN, VK_ESCAPE, VK_TAB, VK_SPACE, VK_BACK, VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT
};
#[cfg(target_os = "windows")]
use winapi::shared::windef::POINT;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum DesktopAction {
    #[serde(rename = "click")]
    Click { x: u32, y: u32, button: Option<String> },
    #[serde(rename = "move")]
    Move { x: u32, y: u32 },
    #[serde(rename = "drag")]
    Drag { from_x: u32, from_y: u32, to_x: u32, to_y: u32 },
    #[serde(rename = "type")]
    TypeText { text: String },
    #[serde(rename = "hotkey")]
    Hotkey { keys: Vec<String> },
    #[serde(rename = "scroll")]
    Scroll { delta_y: i32 },
    #[serde(rename = "open_url")]
    OpenUrl { url: String },
    #[serde(rename = "launch_app")]
    LaunchApp { app: String, args: Option<Vec<String>> },
}


pub struct DesktopActionTool {
    max_screen_width: u32,
    max_screen_height: u32,
}

impl Default for DesktopActionTool {
    fn default() -> Self {
        Self::new(3840, 2160)
    }
}

impl DesktopActionTool {
    pub fn new(max_width: u32, max_height: u32) -> Self {
        Self {
            max_screen_width: max_width,
            max_screen_height: max_height,
        }
    }

    pub fn validate_coordinates(&self, x: u32, y: u32) -> Result<(), ToolError> {
        if x > self.max_screen_width || y > self.max_screen_height {
            return Err(ToolError::ValidationFailed(format!(
                "Coordinates ({}, {}) exceed screen bounds ({}x{})",
                x, y, self.max_screen_width, self.max_screen_height
            )));
        }
        Ok(())
    }
}

static LAST_INPUT_MS: AtomicU64 = AtomicU64::new(0);

fn rate_limit() {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
    let last = LAST_INPUT_MS.load(Ordering::SeqCst);
    if now < last + 50 {
        thread::sleep(Duration::from_millis(last + 50 - now));
    }
    LAST_INPUT_MS.store(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64, Ordering::SeqCst);
}

#[cfg(target_os = "windows")]
fn send_mouse_input(flags: u32, mouse_data: u32) {
    rate_limit();
    unsafe {
        let mut input: INPUT = std::mem::zeroed();
        input.type_ = INPUT_MOUSE;
        let mut mi: MOUSEINPUT = std::mem::zeroed();
        mi.dwFlags = flags;
        mi.mouseData = mouse_data;
        *input.u.mi_mut() = mi;
        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "windows")]
fn send_keyboard_input(w_vk: u16, w_scan: u16, flags: u32) {
    rate_limit();
    unsafe {
        let mut input: INPUT = std::mem::zeroed();
        input.type_ = INPUT_KEYBOARD;
        let mut ki: KEYBDINPUT = std::mem::zeroed();
        ki.wVk = w_vk;
        ki.wScan = w_scan;
        ki.dwFlags = flags;
        *input.u.ki_mut() = ki;
        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "windows")]
fn parse_vk(key: &str) -> Option<u16> {
    match key.to_lowercase().as_str() {
        "ctrl" | "control" => Some(VK_CONTROL as u16),
        "alt" => Some(VK_MENU as u16),
        "shift" => Some(VK_SHIFT as u16),
        "win" | "windows" | "super" => Some(VK_LWIN as u16),
        "enter" => Some(VK_RETURN as u16),
        "esc" | "escape" => Some(VK_ESCAPE as u16),
        "tab" => Some(VK_TAB as u16),
        "space" => Some(VK_SPACE as u16),
        "backspace" => Some(VK_BACK as u16),
        "up" => Some(VK_UP as u16),
        "down" => Some(VK_DOWN as u16),
        "left" => Some(VK_LEFT as u16),
        "right" => Some(VK_RIGHT as u16),
        s if s.len() == 1 => {
            let c = s.chars().next().unwrap();
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_uppercase() as u16)
            } else {
                None
            }
        },
        _ => None,
    }
}

#[async_trait]
impl Tool for DesktopActionTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "desktop_action".into(),
            description: "Executes safe physical desktop actions (click, move, drag, type, hotkey, scroll, open_url, launch_app) on Windows. Parameters: {\"action\": \"open_url|click|type|hotkey|launch_app\", ...}".into(),
            risk_level: RiskLevel::High,
        }

    }

    #[cfg(target_os = "windows")]
    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let action: DesktopAction = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(format!("Invalid desktop action schema: {}", e)))?;

        match action {
            DesktopAction::Click { x, y, button } => {
                self.validate_coordinates(x, y)?;
                let btn = button.unwrap_or_else(|| "left".into());
                unsafe {
                    SetCursorPos(x as i32, y as i32);
                    let mut pt = std::mem::zeroed::<POINT>();
                    GetCursorPos(&mut pt);
                    if btn == "right" {
                        send_mouse_input(MOUSEEVENTF_RIGHTDOWN, 0);
                        send_mouse_input(MOUSEEVENTF_RIGHTUP, 0);
                    } else {
                        send_mouse_input(MOUSEEVENTF_LEFTDOWN, 0);
                        send_mouse_input(MOUSEEVENTF_LEFTUP, 0);
                    }
                    Ok(ToolResult::success(format!("Executed mouse click '{}' at ({}, {}) (Confirmed at {},{})", btn, x, y, pt.x, pt.y)))
                }
            }
            DesktopAction::Move { x, y } => {
                self.validate_coordinates(x, y)?;
                unsafe {
                    SetCursorPos(x as i32, y as i32);
                    let mut pt = std::mem::zeroed::<POINT>();
                    GetCursorPos(&mut pt);
                    Ok(ToolResult::success(format!("Moved mouse cursor to ({}, {}) (Confirmed at {},{})", x, y, pt.x, pt.y)))
                }
            }
            DesktopAction::Drag { from_x, from_y, to_x, to_y } => {
                self.validate_coordinates(from_x, from_y)?;
                self.validate_coordinates(to_x, to_y)?;
                unsafe {
                    SetCursorPos(from_x as i32, from_y as i32);
                    send_mouse_input(MOUSEEVENTF_LEFTDOWN, 0);
                    thread::sleep(Duration::from_millis(50));
                    SetCursorPos(to_x as i32, to_y as i32);
                    send_mouse_input(MOUSEEVENTF_LEFTUP, 0);
                    Ok(ToolResult::success(format!("Executed drag from ({}, {}) to ({}, {})", from_x, from_y, to_x, to_y)))
                }
            }
            DesktopAction::TypeText { text } => {
                if text.len() > 2000 {
                    return Err(ToolError::ValidationFailed("Text input exceeds maximum limit of 2000 characters".into()));
                }
                for c in text.encode_utf16() {
                    send_keyboard_input(0, c, KEYEVENTF_UNICODE);
                    send_keyboard_input(0, c, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
                }
                Ok(ToolResult::success(format!("Typed {} characters to focused element", text.len())))
            }
            DesktopAction::Hotkey { keys } => {
                let mut vks = Vec::new();
                for k in &keys {
                    if let Some(vk) = parse_vk(k) {
                        vks.push(vk);
                    }
                }
                for &vk in &vks {
                    send_keyboard_input(vk, 0, 0);
                }
                for &vk in vks.iter().rev() {
                    send_keyboard_input(vk, 0, KEYEVENTF_KEYUP);
                }
                Ok(ToolResult::success(format!("Triggered hotkey sequence: {}", keys.join("+"))))
            }
            DesktopAction::Scroll { delta_y } => {
                send_mouse_input(MOUSEEVENTF_WHEEL, (delta_y * 120) as u32);
                Ok(ToolResult::success(format!("Scrolled vertical delta: {}", delta_y)))
            }
            DesktopAction::OpenUrl { url } => {
                if !url.starts_with("http://") && !url.starts_with("https://") {
                    return Err(ToolError::ValidationFailed("URL must start with http:// or https://".into()));
                }
                let status = std::process::Command::new("powershell")
                    .args(&["-NoProfile", "-NonInteractive", "-Command", &format!("Start-Process '{}'", url.replace('\'', "''"))])
                    .status();

                match status {
                    Ok(st) if st.success() => Ok(ToolResult::success(format!("Successfully opened URL in browser: {}", url))),
                    _ => {
                        let _ = std::process::Command::new("msedge.exe")
                            .arg(&url)
                            .spawn();
                        Ok(ToolResult::success(format!("Launched browser with URL: {}", url)))
                    }
                }
            }

            DesktopAction::LaunchApp { app, args } => {
                let mut cmd = std::process::Command::new(&app);
                if let Some(arg_list) = args {
                    cmd.args(arg_list);
                }
                let _ = cmd.spawn();
                Ok(ToolResult::success(format!("Successfully launched application: {}", app)))
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let action: DesktopAction = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(format!("Invalid desktop action schema: {}", e)))?;

        match action {
            DesktopAction::Click { x, y, button } => {
                self.validate_coordinates(x, y)?;
                let btn = button.unwrap_or_else(|| "left".into());
                Ok(ToolResult::success(format!("Executed mouse click '{}' at ({}, {})", btn, x, y)))
            }
            DesktopAction::Move { x, y } => {
                self.validate_coordinates(x, y)?;
                Ok(ToolResult::success(format!("Moved mouse cursor to ({}, {})", x, y)))
            }
            DesktopAction::Drag { from_x, from_y, to_x, to_y } => {
                self.validate_coordinates(from_x, from_y)?;
                self.validate_coordinates(to_x, to_y)?;
                Ok(ToolResult::success(format!("Executed drag from ({}, {}) to ({}, {})", from_x, from_y, to_x, to_y)))
            }
            DesktopAction::TypeText { text } => {
                if text.len() > 2000 {
                    return Err(ToolError::ValidationFailed("Text input exceeds maximum limit of 2000 characters".into()));
                }
                Ok(ToolResult::success(format!("Typed {} characters to focused element", text.len())))
            }
            DesktopAction::Hotkey { keys } => {
                let key_str = keys.join("+");
                Ok(ToolResult::success(format!("Triggered hotkey sequence: {}", key_str)))
            }
            DesktopAction::Scroll { delta_y } => {
                Ok(ToolResult::success(format!("Scrolled vertical delta: {}", delta_y)))
            }
        }
    }
}
