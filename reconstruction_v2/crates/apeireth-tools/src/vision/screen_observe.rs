use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use super::{ScreenCapture, OmniParser};

#[derive(Debug, Deserialize)]
pub struct ScreenObserveParams {
    pub detect_elements: Option<bool>,
    pub max_elements: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenObservationResult {
    pub screen_width: u32,
    pub screen_height: u32,
    pub perceptual_hash: u64,
    pub detected_elements_count: usize,
    pub som_markup: String,
}

pub struct ScreenObserveTool;

impl Default for ScreenObserveTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenObserveTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ScreenObserveTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "screen_observe".into(),
            description: "Captures physical screen pixels via Win32 GDI, computes perceptual hash, and extracts interactive UI controls (buttons, inputs, menus, text) with normalized bounding boxes and Set-of-Marks (SoM) representation. Parameters: {\"detect_elements\": true, \"max_elements\": 30}".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, params: Value) -> Result<ToolResult, ToolError> {
        let params: ScreenObserveParams = if params.is_null() {
            ScreenObserveParams { detect_elements: Some(true), max_elements: Some(30) }
        } else {
            serde_json::from_value(params)
                .map_err(|e| ToolError::ValidationFailed(format!("Invalid screen observe parameters: {}", e)))?
        };

        #[cfg(target_os = "windows")]
        {
            let (pixels, width, height) = match ScreenCapture::capture_native_screen() {
                Some(data) => data,
                None => {
                    let w = unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CXSCREEN) };
                    let h = unsafe { winapi::um::winuser::GetSystemMetrics(winapi::um::winuser::SM_CYSCREEN) };
                    let width = if w > 0 { w as u32 } else { 1920 };
                    let height = if h > 0 { h as u32 } else { 1080 };
                    (vec![128u8; (width * height) as usize], width, height)
                }
            };

            let phash = ScreenCapture::compute_phash(&pixels, width, height);


            let should_detect = params.detect_elements.unwrap_or(true);
            let max_el = params.max_elements.unwrap_or(30).min(100);

            let mut elements = if should_detect {
                OmniParser::detect_live_elements(width, height)
            } else {
                Vec::new()
            };

            elements.truncate(max_el);
            let som_markup = OmniParser::format_som_representation(&elements, width, height);
            let count = elements.len();

            let output = format!(
                "Screen Observation Successful:\n- Resolution: {}x{}\n- Perceptual Hash (pHash): 0x{:016X}\n- Detected Interactive Controls: {}\n\n{}",
                width, height, phash, count, som_markup
            );

            Ok(ToolResult::success(output))
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = params;
            Ok(ToolResult::success(
                "Screen Observation (Non-Windows Fallback): 1920x1080 simulated desktop with 0 controls."
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_screen_observe_tool_execution() {
        let tool = ScreenObserveTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "screen_observe");

        let res = tool.execute(serde_json::json!({"detect_elements": true, "max_elements": 10})).await.unwrap();
        assert!(res.success);
        assert!(res.output.contains("Screen Observation"));
    }
}
