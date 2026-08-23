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
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8"),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            reqwest::header::HeaderValue::from_static("zh-CN,zh;q=0.9,en;q=0.8"),
        );

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0")
            .default_headers(headers)
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

        let resp = match self.client.get(&params.url).send().await {
            Ok(r) => r,
            Err(_) => {
                let proxy_client = reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .proxy(reqwest::Proxy::all("http://127.0.0.1:7897").unwrap_or_else(|_| reqwest::Proxy::all("http://127.0.0.1:7890").unwrap()))
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0")
                    .build()
                    .unwrap_or_default();
                proxy_client.get(&params.url)
                    .send()
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(format!("HTTP fetch failed (both direct and proxy): {}", e)))?
            }
        };

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
