use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct BrowserParams {
    pub url: String,
    pub max_chars: Option<usize>,
}

pub struct BrowserTool {
    client: reqwest::Client,
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserTool {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("Apeireth/2.0 (Cognitive OS; Living Companion)")
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Strips basic HTML tags and decodes common entities to plain text
    fn extract_text_from_html(html: &str) -> String {
        let mut in_script = false;
        let mut in_style = false;

        let mut text = String::with_capacity(html.len() / 2);
        let mut current_tag = String::new();

        let chars: Vec<char> = html.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let ch = chars[i];
            if ch == '<' {
                current_tag.clear();
                i += 1;
                while i < len && chars[i] != '>' {
                    current_tag.push(chars[i]);
                    i += 1;
                }
                let tag_lower = current_tag.trim().to_lowercase();
                if tag_lower.starts_with("script") {
                    in_script = true;
                } else if tag_lower.starts_with("/script") {
                    in_script = false;
                } else if tag_lower.starts_with("style") {
                    in_style = true;
                } else if tag_lower.starts_with("/style") {
                    in_style = false;
                } else if tag_lower == "p" || tag_lower == "br" || tag_lower == "div" || tag_lower == "li" || tag_lower.starts_with("h") {
                    text.push('\n');
                }
            } else if !in_script && !in_style {
                text.push(ch);
            }

            i += 1;
        }

        // Clean up excessive whitespace
        let cleaned: Vec<&str> = text.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        cleaned.join("\n")
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser".into(),
            description: "Fetches and reads web page contents in clean text format for knowledge extraction. Parameters: {\"url\": \"https://...\", \"max_chars\": 8000}".into(),
            risk_level: RiskLevel::Low,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let params: BrowserParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(format!("Invalid browser parameters: {}", e)))?;

        if !params.url.starts_with("http://") && !params.url.starts_with("https://") {
            return Err(ToolError::ValidationFailed("URL must start with http:// or https://".into()));
        }

        let resp = self.client.get(&params.url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("HTTP fetch failed: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(ToolError::ExecutionFailed(format!("HTTP error status: {}", status)));
        }

        let raw_html = resp.text()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read response body: {}", e)))?;

        let max_len = params.max_chars.unwrap_or(8000).min(32000);
        let plain_text = Self::extract_text_from_html(&raw_html);
        let truncated: String = plain_text.chars().take(max_len).collect();

        Ok(ToolResult::success(format!(
            "Fetched [{}]: (Status: {})\n\n{}",
            params.url, status, truncated
        )))
    }
}
