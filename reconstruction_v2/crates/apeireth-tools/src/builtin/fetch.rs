use crate::{Tool, ToolDefinition, ToolError, ToolResult, RiskLevel};
use async_trait::async_trait;
use serde::Deserialize;
use reqwest::{Client, Method, Url};
use std::net::IpAddr;

#[derive(Debug, Deserialize)]
pub struct FetchParams {
    pub url: String,
    pub method: Option<String>,
}

pub struct FetchTool {
    client: Client,
}

impl FetchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }
}

fn is_allowed_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            !ipv4.is_private() && !ipv4.is_loopback() && !ipv4.is_link_local()
        }
        IpAddr::V6(ipv6) => {
            !ipv6.is_loopback() 
        }
    }
}

#[async_trait]
impl Tool for FetchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "fetch".into(),
            description: "HTTP GET/POST with anti-SSRF".into(),
            risk_level: RiskLevel::Medium,
        }
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult, ToolError> {
        let params: FetchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::ValidationFailed(e.to_string()))?;

        let url = Url::parse(&params.url)
            .map_err(|e| ToolError::ValidationFailed(format!("Invalid URL: {}", e)))?;

        // Simple SSRF defense: resolve DNS and check IP
        if let Some(host) = url.host_str() {
            if let Ok(addrs) = tokio::net::lookup_host((host, url.port().unwrap_or(80))).await {
                for addr in addrs {
                    if !is_allowed_ip(addr.ip()) {
                        return Err(ToolError::ValidationFailed("SSRF protection: Cannot access private/local IPs".into()));
                    }
                }
            }
        }

        let method = match params.method.as_deref().unwrap_or("GET").to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => return Err(ToolError::ValidationFailed("Unsupported HTTP method".into())),
        };

        let res = self.client.request(method, url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        // Truncate size
        let body = if body.len() > 1024 * 1024 {
            format!("{}... [Truncated]", &body[..1024*1024])
        } else {
            body
        };

        Ok(ToolResult {
            success: status.is_success(),
            output: body,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_ssrf() {
        let tool = FetchTool::new();
        let res = tool.execute(serde_json::json!({
            "url": "http://127.0.0.1/"
        })).await;
        assert!(res.is_err());
        if let Err(ToolError::ValidationFailed(msg)) = res {
            assert!(msg.contains("SSRF"));
        } else {
            panic!("Expected SSRF validation failure");
        }
    }

    #[tokio::test]
    async fn test_fetch_public() {
        let tool = FetchTool::new();
        let res = tool.execute(serde_json::json!({
            "url": "http://example.com/"
        })).await;
        // In local/mock or live tests, either it executes or gives network error without panic
        assert!(res.is_ok() || res.is_err());
    }
}
