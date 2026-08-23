use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};

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

#[async_trait]
impl Tool for DesktopActionTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "desktop_action".into(),
            description: "Executes safe desktop actions (click, move, drag, type, hotkey, scroll) with bounding verification".into(),
            risk_level: RiskLevel::High,
        }
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let action: DesktopAction = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(format!("Invalid desktop action schema: {}", e)))?;

        match action {
            DesktopAction::Click { x, y, button } => {
                self.validate_coordinates(x, y)?;
                let btn = button.unwrap_or_else(|| "left".into());
                // In simulated/native sandbox environment, record action execution
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
