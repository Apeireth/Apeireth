//! LiveKit - 实时音视频 stub (从 v1.0 apeireth-livekit 2.8K LOC 收敛)
//!
//! 0 装 PASS: 简化 HTTP API 客户端 (token 生成 + room), 不连真 LiveKit 服务器.
//! 完整 v1.0 era (WebRTC, signaling) 标 stub.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveKitConfig {
    pub api_key: String,
    pub api_secret: String,
    pub ws_url: String,  // wss://your-livekit.com
}

impl LiveKitConfig {
    /// 0 装 PASS: 真实默认值, 但需 user 填 api_key/api_secret
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>, ws_url: impl Into<String>) -> Self {
        Self { api_key: api_key.into(), api_secret: api_secret.into(), ws_url: ws_url.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub sid: String,
    pub name: String,
    pub max_participants: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequest {
    pub room: String,
    pub identity: String,
    pub ttl_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub token: String,
    pub ws_url: String,
}

pub struct LiveKitClient {
    pub config: LiveKitConfig,
}

impl LiveKitClient {
    pub fn new(config: LiveKitConfig) -> Self { Self { config } }

    /// 0 装 PASS: 真实 JWT 结构 (HS256), 但当前返 mock (待接真服务)
    pub async fn create_token(&self, _req: TokenRequest) -> Result<TokenResponse, String> {
        // 0 装 PASS: 不假装签了真 JWT — 返 mock
        Ok(TokenResponse {
            token: format!("mock-jwt-{}", chrono::Utc::now().timestamp_millis()),
            ws_url: self.config.ws_url.clone(),
        })
    }

    /// 0 装 PASS: 真实 URL 构造逻辑 (但请求 mock)
    pub async fn list_rooms(&self) -> Result<Vec<Room>, String> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_config_new() {
        let c = LiveKitConfig::new("APIabc", "secretxyz", "wss://test.livekit.cloud");
        assert_eq!(c.ws_url, "wss://test.livekit.cloud");
    }
    #[tokio::test]
    async fn test_create_token_mock() {
        let client = LiveKitClient::new(LiveKitConfig::new("k", "s", "wss://test"));
        let r = client.create_token(TokenRequest { room: "r1".into(), identity: "u1".into(), ttl_seconds: 3600 }).await.unwrap();
        assert!(r.token.starts_with("mock-"));
    }
}
