//! # Lark Webhook (per @larksuiteoapi/lark-sdk v0.9.21 1:1 翻译)
//!
//! 飞书事件订阅 Webhook 翻译源.
//!
//! **3 核心概念** (per v0.9.21 1:1):
//! 1. **url_verification** — 配置事件订阅 URL 时, 飞书 server 发送 `challenge` 字段, 客户端原样返回
//! 2. **event_callback** — 真实事件回调, 需校验 `token` + 解密 `encrypt` (用 `encrypt_key`)
//! 3. **verify_webhook** — 一次性 token 校验入口
//!
//! **1 核心 API** (per v0.9.21):
//! - `verify_webhook` — 校验入站 webhook 事件
//!
//! **当前 STUB**: 字段保留 1:1 翻译, 走 `verify_webhook` 返 `NotImplemented` (待 R21+ 续真接 AES 解密).
//!
//! ## 4 EventType 守门 (per v0.9.21 1:1)
//!
//! - `UrlVerification` — URL 校验事件
//! - `EventCallback` — 事件回调 (im.message.receive_v1 / contact.user.created_v3 / etc)
//! - `Challenge` — 单独 challenge 字段
//! - `Unknown` — 未知事件 (兜底)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::lark::auth::WebhookToken;
use crate::lark::error::LarkError;

// ============================================================================
// §1 EventType (4 variant, 1:1 翻译 v0.9.21)
// ============================================================================

/// Webhook 事件类型 (4 variant, per v0.9.21 `type` / `header.event_type` 字段).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// URL 校验事件 (per v0.9.21 `type: "url_verification"`, R21 真接).
    UrlVerification,
    /// 事件回调 (per v0.9.21 `header.event_type: "im.message.receive_v1"` 等).
    EventCallback,
    /// Challenge 字段 (per v0.9.21 `type: "challenge"`, 兼容老版).
    Challenge,
    /// 未知事件 (兜底, R21 真接后细化).
    #[default]
    Unknown,
}

impl EventType {
    /// 4 variant hardcode 常量.
    pub const COUNT: usize = 4;

    /// 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::UrlVerification => "url_verification",
            EventType::EventCallback => "event_callback",
            EventType::Challenge => "challenge",
            EventType::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §2 WebhookEvent (per v0.9.21 1:1)
// ============================================================================

/// Webhook 事件顶层结构 (per v0.9.21 event callback 1:1).
///
/// 飞书 server POST 到客户端 URL 的 JSON body, 包含:
/// - `challenge` — URL 校验时, 客户端原样返回
/// - `token` — 校验 token (per WebhookToken.token)
/// - `timestamp` — 事件时间戳
/// - `encrypt` — 加密的事件数据 (用 encrypt_key 解密, R21 续真接)
/// - `type` / `header.event_type` — 事件类型
/// - `event` — 解密后的事件内容 (R21 续真接)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// 事件类型 (per `type` / `header.event_type` 字段, 4 variant).
    pub event_type: EventType,
    /// App ID (per `app_id` / `header.app_id` 字段).
    pub app_id: String,
    /// 校验 token (per `token` / `header.token` 字段, 应等于 WebhookToken.token).
    pub token: String,
    /// 时间戳 (per `create_time` / `header.create_time` 字段, 秒).
    pub timestamp_secs: u64,
    /// Challenge 字段 (URL 校验时存在, 客户端原样返回).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
    /// 加密的事件数据 (R21 续真接时用 encrypt_key 解密).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypt: Option<String>,
    /// 解密后的事件内容 (R21 续真接, STUB 模式为原始 JSON).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub event: HashMap<String, serde_json::Value>,
}

impl WebhookEvent {
    /// 构造 URL 校验事件.
    pub fn new_url_verification(challenge: String) -> Self {
        Self {
            event_type: EventType::UrlVerification,
            app_id: String::new(),
            token: String::new(),
            timestamp_secs: 0,
            challenge: Some(challenge),
            encrypt: None,
            event: HashMap::new(),
        }
    }

    /// 构造事件回调.
    pub fn new_event_callback(
        app_id: String,
        token: String,
        timestamp_secs: u64,
        event: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            event_type: EventType::EventCallback,
            app_id,
            token,
            timestamp_secs,
            challenge: None,
            encrypt: None,
            event,
        }
    }
}

// ============================================================================
// §3 校验入口 (per verify_webhook 1:1)
// ============================================================================

/// 校验 webhook 事件 (per `verify_webhook` 1:1).
///
/// 4 步骤:
/// 1. token 校验: `event.token == webhook_token.token` (恒定时间比较, per H7)
/// 2. app_id 校验: `event.app_id` 必为配置 app_id (R21 续真接)
/// 3. timestamp 校验: 误差 < 5 分钟 (防 replay attack, R21 续真接)
/// 4. URL 校验: 若是 url_verification/challenge, 返 challenge; event_callback/unknown
///    显式返 `NotImplemented` (STUB 期 0 静默放行未解密事件体, per H7)
///
/// **安全 (per H7 修复)**: token 校验失败的错误消息只含 "mismatch", **0 含期望值/入站值** —
/// webhook token 是长期共享秘密, 一旦进错误串被日志/回显即泄秘.
pub fn verify_webhook_event(
    event: &WebhookEvent,
    webhook_token: &WebhookToken,
) -> Result<WebhookVerifyResult, LarkError> {
    // 1. token 校验 (恒定时间比较, per auth.rs WebhookToken::verify)
    if !webhook_token.verify(&event.token) {
        // H7: 错误只报 "mismatch" 不含 expected/got — 共享秘密 0 进错误串
        return Err(LarkError::Other("webhook token mismatch".to_string()));
    }
    // 2. URL 校验
    match event.event_type {
        EventType::UrlVerification | EventType::Challenge => {
            if let Some(challenge) = &event.challenge {
                Ok(WebhookVerifyResult::Challenge(challenge.clone()))
            } else {
                Err(LarkError::Other(
                    "url_verification event missing challenge field".to_string(),
                ))
            }
        }
        // 3+4 步 (event callback decrypt + handle) 留 R21+: 非 URL 校验事件显式
        // NotImplemented — 未解密事件体被静默 Accepted 比显式 stub 更危险 (per H7)
        EventType::EventCallback => Err(LarkError::NotImplemented("event_callback_decrypt")),
        EventType::Unknown => Err(LarkError::NotImplemented("unknown_event")),
    }
}

/// Webhook 校验结果 (per v0.9.21 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookVerifyResult {
    /// URL 校验通过, 客户端返 challenge 给飞书 server.
    Challenge(String),
    /// 事件已接受 (R21 续真接 event_callback AES 解密后才产生; STUB 期不产生 —
    /// per H7, 未解密事件体现在走 `NotImplemented` 而非静默 Accepted).
    Accepted,
}

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_4_variants() {
        assert_eq!(EventType::COUNT, 4);
    }

    #[test]
    fn url_verification_event_construction() {
        let event = WebhookEvent::new_url_verification("test-challenge-12345".to_string());
        assert_eq!(event.event_type, EventType::UrlVerification);
        assert_eq!(event.challenge.as_deref(), Some("test-challenge-12345"));
    }

    #[test]
    fn event_callback_construction() {
        let mut ev = HashMap::new();
        ev.insert(
            "type".to_string(),
            serde_json::json!("im.message.receive_v1"),
        );
        let event = WebhookEvent::new_event_callback(
            "cli_a1b2c3d4e5f6".to_string(),
            "token_xxx".to_string(),
            1234567890,
            ev.clone(),
        );
        assert_eq!(event.event_type, EventType::EventCallback);
        assert_eq!(event.timestamp_secs, 1234567890);
        assert_eq!(event.event.len(), 1);
    }

    #[test]
    fn verify_webhook_url_verification() {
        let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        let event = WebhookEvent {
            event_type: EventType::UrlVerification,
            app_id: "cli_a1b2c3d4e5f6".to_string(),
            token: "token_xxx".to_string(),
            timestamp_secs: 0,
            challenge: Some("test-challenge-12345".to_string()),
            encrypt: None,
            event: HashMap::new(),
        };
        let result = verify_webhook_event(&event, &wh_token).expect("valid");
        assert!(matches!(result, WebhookVerifyResult::Challenge(_)));
    }

    #[test]
    fn verify_webhook_url_verification_rejects_missing_challenge() {
        let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        let event = WebhookEvent {
            event_type: EventType::UrlVerification,
            app_id: "cli_a1b2c3d4e5f6".to_string(),
            token: "token_xxx".to_string(),
            timestamp_secs: 0,
            challenge: None,
            encrypt: None,
            event: HashMap::new(),
        };
        let result = verify_webhook_event(&event, &wh_token);
        assert!(matches!(result, Err(LarkError::Other(_))));
    }

    #[test]
    fn verify_webhook_token_mismatch_does_not_leak_expected_token() {
        let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        let event = WebhookEvent {
            event_type: EventType::UrlVerification,
            app_id: "cli_a1b2c3d4e5f6".to_string(),
            token: "wrong_token".to_string(),
            timestamp_secs: 0,
            challenge: Some("test".to_string()),
            encrypt: None,
            event: HashMap::new(),
        };
        let result = verify_webhook_event(&event, &wh_token);
        match result {
            Err(LarkError::Other(msg)) => {
                assert!(msg.contains("mismatch"), "错误消息应含 mismatch: {msg}");
                // H7: 错误消息 0 含期望 token / 入站 token (共享秘密 0 进错误串)
                assert!(!msg.contains("token_xxx"), "H7: 0 含期望 token: {msg}");
                assert!(!msg.contains("wrong_token"), "H7: 0 含入站 token: {msg}");
            }
            other => panic!("expected Other(mismatch), got {other:?}"),
        }
    }

    #[test]
    fn verify_webhook_event_callback_not_implemented() {
        let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        let mut ev = HashMap::new();
        ev.insert(
            "type".to_string(),
            serde_json::json!("im.message.receive_v1"),
        );
        let event = WebhookEvent::new_event_callback(
            "cli_a1b2c3d4e5f6".to_string(),
            "token_xxx".to_string(),
            1234567890,
            ev,
        );
        // H7: STUB 期非 URL 校验事件显式 NotImplemented, 0 静默 Accepted
        // (未解密事件体被放行比显式 stub 更危险)
        let result = verify_webhook_event(&event, &wh_token);
        assert!(matches!(result, Err(LarkError::NotImplemented(_))));
    }

    #[test]
    fn verify_webhook_unknown_event_not_implemented() {
        let wh_token = WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string())
            .expect("valid");
        let event = WebhookEvent {
            event_type: EventType::Unknown,
            app_id: "cli_a1b2c3d4e5f6".to_string(),
            token: "token_xxx".to_string(),
            timestamp_secs: 0,
            challenge: None,
            encrypt: Some("encrypted_blob".to_string()),
            event: HashMap::new(),
        };
        let result = verify_webhook_event(&event, &wh_token);
        assert!(matches!(result, Err(LarkError::NotImplemented(_))));
    }
}
