//! IM 快捷接入: 自有 wire 原语 (入站信封解析 / 内容形状 / 消息预算)。
//!
//! 本模块是 `im` 面的唯一 wire 原语来源, 不再跨模块消费已归档的适配族:
//!
//! - [`EventType`] / [`WebhookEvent`] — `im-feishu` 入站信封的严格解析: 三类
//!   信封形状 (URL 校验 / 明文事件回调 / 加密回调) + 时间戳口径 + 未知形状
//!   fail-closed (0 静默放行);
//! - [`TextContent`] / [`CardContent`] — 出站文本/卡片内容形状 (与平台 wire
//!   同形: 文本 `{"text": ...}`, 卡片 `config/header/elements`);
//! - [`MAX_MESSAGE_TEXT_BYTES`] — 单条文本消息的字节预算锚;
//! - [`WireError`] — 解析/校验的事实错误 (只报事实类别, 0 回显共享秘密)。
//!
//! 解析行为与归档前适配族的对应面逐行等价 (归档清单见
//! `legacy/archived/sdk-islands/MANIFEST.md`); 诊断前缀收敛为 `im wire`,
//! 接受/拒绝判定、错误分类与出站 JSON 形状不变。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 单条文本消息的字节上限 (防单消息爆炸; 分段策略的上界锚)。
pub(crate) const MAX_MESSAGE_TEXT_BYTES: usize = 4096;

/// 单 webhook 回调体字节上限 (防超大回调体)。
pub(crate) const MAX_WEBHOOK_CHUNK_BYTES: usize = 16 * 1024;

// ============================================================================
// §1 WireError (解析/校验事实错误)
// ============================================================================

/// wire 原语的事实错误 (内部面; 调用方按 [`ImError`](crate::im::error::ImError)
/// 的闭合分类处理)。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum WireError {
    /// 入站信封形状不在 wire 契约覆盖范围内 (0 静默放行)。
    #[error("unsupported im wire envelope: {0} (not covered by the im wire contract)")]
    Unsupported(&'static str),
    /// @ 用户 open_id 形状非法 (非 `ou_` 前缀)。
    #[error("im wire open id is invalid: {0} (expected prefix 'ou_')")]
    OpenIdInvalid(String),
    /// 其他畸形/解析失败 (永久错误)。
    #[error("im wire error: {0}")]
    Other(String),
}

/// wire 原语结果别名。
pub(crate) type WireResult<T> = Result<T, WireError>;

// ============================================================================
// §2 EventType (4 variant 闭合枚举)
// ============================================================================

/// 入站回调事件类型 (4 variant 闭合枚举)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventType {
    /// URL 校验事件 (`url_verification`)。
    UrlVerification,
    /// 事件回调 (含加密信封 —— 解密前无法细分)。
    EventCallback,
    /// 兼容老式 challenge 字段 (`challenge`)。
    Challenge,
    /// 未知 (兜底; 解析时显式 `Unsupported`, 0 静默放行)。
    #[default]
    Unknown,
}

impl EventType {
    /// 4 variant hardcode 常量。
    pub(crate) const COUNT: usize = 4;

    /// wire 字符串。
    pub(crate) fn as_str(&self) -> &'static str {
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
// §3 WebhookEvent (入站事件领域形态)
// ============================================================================

/// Webhook 入站事件 (领域形态)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WebhookEvent {
    /// 事件类型 (闭合枚举)。
    pub(crate) event_type: EventType,
    /// App ID (来源应用标识, 可空)。
    pub(crate) app_id: String,
    /// 校验 token (应与配置的 webhook token 一致)。
    pub(crate) token: String,
    /// 事件时间戳 (秒; 0 = 未提供, 事件回调会因此被拒)。
    pub(crate) timestamp_secs: u64,
    /// URL 校验 challenge (URL 校验事件专用)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) challenge: Option<String>,
    /// 加密信封 (base64; 有值即先解密再校验)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) encrypt: Option<String>,
    /// 事件体 (解密/校验后可得)。
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) event: HashMap<String, serde_json::Value>,
}

impl WebhookEvent {
    /// 原始回调 body → 领域事件 (三类信封严格识别, 未知形状 = `Unsupported`)。
    pub(crate) fn from_raw_json(raw: &str) -> WireResult<Self> {
        if raw.len() > MAX_WEBHOOK_CHUNK_BYTES {
            return Err(WireError::Other(format!(
                "webhook payload too large: {} > {} bytes",
                raw.len(),
                MAX_WEBHOOK_CHUNK_BYTES
            )));
        }
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|_| WireError::Other("webhook payload is not valid JSON".to_string()))?;
        let obj = value
            .as_object()
            .ok_or(WireError::Unsupported("webhook_event_type"))?;

        // 信封 3: 加密回调 (整包密文, 内容解密后才可见)
        if let Some(encrypt) = obj.get("encrypt").and_then(|v| v.as_str()) {
            return Ok(Self {
                event_type: EventType::EventCallback,
                app_id: String::new(),
                token: String::new(),
                timestamp_secs: 0,
                challenge: None,
                encrypt: Some(encrypt.to_string()),
                event: HashMap::new(),
            });
        }

        let type_str = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match type_str {
            // 信封 1: URL 校验 (challenge 缺失留给校验步拒绝)
            "url_verification" => Ok(Self {
                event_type: EventType::UrlVerification,
                app_id: obj
                    .get("app_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                token: obj
                    .get("token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                timestamp_secs: parse_timestamp_secs(
                    obj.get("ts").or_else(|| obj.get("create_time")),
                )?,
                challenge: obj
                    .get("challenge")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                encrypt: None,
                event: HashMap::new(),
            }),
            "challenge" => Ok(Self {
                event_type: EventType::Challenge,
                app_id: obj
                    .get("app_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                token: obj
                    .get("token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                timestamp_secs: parse_timestamp_secs(
                    obj.get("ts").or_else(|| obj.get("create_time")),
                )?,
                challenge: obj
                    .get("challenge")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                encrypt: None,
                event: HashMap::new(),
            }),
            // 信封 2 形状 A: 明文事件回调
            "event_callback" => {
                let event = require_event_object(&value)?;
                Ok(Self {
                    event_type: EventType::EventCallback,
                    app_id: obj
                        .get("app_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    token: obj
                        .get("token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    timestamp_secs: parse_timestamp_secs(
                        obj.get("ts").or_else(|| obj.get("create_time")),
                    )?,
                    challenge: None,
                    encrypt: None,
                    event,
                })
            }
            _ => {
                // 信封 2 形状 B: header/event 结构
                if let Some(header) = obj.get("header").and_then(|h| h.as_object()) {
                    let event = require_event_object(&value)?;
                    return Ok(Self {
                        event_type: EventType::EventCallback,
                        app_id: header
                            .get("app_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        token: header
                            .get("token")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        timestamp_secs: parse_timestamp_secs(header.get("create_time"))?,
                        challenge: None,
                        encrypt: None,
                        event,
                    });
                }
                Err(WireError::Unsupported("webhook_event_type"))
            }
        }
    }
}

/// 事件体必须是 JSON 对象 (缺失/形状错 = 永久错误)。
fn require_event_object(
    value: &serde_json::Value,
) -> WireResult<HashMap<String, serde_json::Value>> {
    let obj = value
        .get("event")
        .and_then(|e| e.as_object())
        .ok_or_else(|| WireError::Other("webhook payload missing event object".to_string()))?;
    Ok(obj
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<HashMap<String, serde_json::Value>>())
}

/// 时间戳解析 (数字或数字字符串; ≥ 10^12 视为毫秒)。
///
/// 缺失/非数字 → 永久错误; 毫秒口径自动折算为秒。
fn parse_timestamp_secs(value: Option<&serde_json::Value>) -> WireResult<u64> {
    let Some(value) = value else {
        return Ok(0);
    };
    if value.is_null() {
        return Ok(0);
    }
    let raw: i128 = match value {
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(i128::from)
            .ok_or_else(|| WireError::Other("webhook timestamp is not an integer".to_string()))?,
        serde_json::Value::String(s) => s
            .trim()
            .parse::<i128>()
            .map_err(|_| WireError::Other("webhook timestamp is not numeric".to_string()))?,
        _ => {
            return Err(WireError::Other(
                "webhook timestamp has wrong type".to_string(),
            ))
        }
    };
    if raw < 0 {
        return Err(WireError::Other(
            "webhook timestamp is negative".to_string(),
        ));
    }
    let secs = if raw >= 1_000_000_000_000 {
        raw / 1000
    } else {
        raw
    };
    u64::try_from(secs).map_err(|_| WireError::Other("webhook timestamp out of range".to_string()))
}

// ============================================================================
// §4 内容形状 (出站文本 / 卡片)
// ============================================================================

/// 文本消息内容 (`content: {"text": "...", "at_open_ids": [...]}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TextContent {
    /// 文本正文。
    pub(crate) text: String,
    /// @用户 open_id 列表 (可选)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) at_open_ids: Vec<String>,
}

impl TextContent {
    /// 创建文本内容。
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            at_open_ids: Vec::new(),
        }
    }

    /// 校验 @user 列表 (open_id 形状强校验)。
    pub(crate) fn validate(&self) -> WireResult<()> {
        for oid in &self.at_open_ids {
            validate_open_id(oid)?;
        }
        Ok(())
    }
}

/// open_id 形状校验 (非空 + `ou_` 前缀)。
fn validate_open_id(open_id: &str) -> WireResult<()> {
    let trimmed = open_id.trim();
    if trimmed.is_empty() || !trimmed.starts_with("ou_") {
        return Err(WireError::OpenIdInvalid(trimmed.to_string()));
    }
    Ok(())
}

/// 卡片消息内容 (`content: {"config": {...}, "header": {...}, "elements": [...]}`)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CardContent {
    /// 卡片配置 (wide_screen_mode / enable_forward 等)。
    #[serde(default)]
    pub(crate) config: HashMap<String, serde_json::Value>,
    /// 卡片 header (title / template)。
    #[serde(default)]
    pub(crate) header: HashMap<String, serde_json::Value>,
    /// 卡片 elements (文本 / button / divider / image 等)。
    #[serde(default)]
    pub(crate) elements: Vec<HashMap<String, serde_json::Value>>,
}

impl CardContent {
    /// 创建简单文本卡片。
    pub(crate) fn plain(title: impl Into<String>, body: impl Into<String>) -> Self {
        let mut header = HashMap::new();
        header.insert(
            "title".to_string(),
            serde_json::json!({"tag": "plain_text", "content": title.into()}),
        );
        let mut body_elem = HashMap::new();
        body_elem.insert(
            "tag".to_string(),
            serde_json::Value::String("div".to_string()),
        );
        body_elem.insert(
            "text".to_string(),
            serde_json::json!({"tag": "plain_text", "content": body.into()}),
        );
        Self {
            config: HashMap::new(),
            header,
            elements: vec![body_elem],
        }
    }
}

// ============================================================================
// §5 单元测试 (信封形状 / 内容形状 / 预算锚)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_type_closed_vocabulary() {
        assert_eq!(EventType::COUNT, 4);
        assert_eq!(EventType::default(), EventType::Unknown);
        assert_eq!(EventType::UrlVerification.as_str(), "url_verification");
        assert_eq!(EventType::EventCallback.to_string(), "event_callback");
    }

    #[test]
    fn url_verification_is_parsed_with_unknown_field_tolerance() {
        let raw = r#"{"type":"url_verification","challenge":"nonce-1","token":"t","future":42}"#;
        let event = WebhookEvent::from_raw_json(raw).expect("parse");
        assert_eq!(event.event_type, EventType::UrlVerification);
        assert_eq!(event.challenge.as_deref(), Some("nonce-1"));
        assert_eq!(event.token, "t");
    }

    #[test]
    fn event_callback_shapes_are_parsed_with_timestamp_discipline() {
        // 形状 A: type/ts + event
        let raw = r#"{"type":"event_callback","token":"t","ts":1700000000,"event":{"id":"evt_1"}}"#;
        let event = WebhookEvent::from_raw_json(raw).expect("parse");
        assert_eq!(event.event_type, EventType::EventCallback);
        assert_eq!(event.timestamp_secs, 1_700_000_000);
        assert_eq!(event.event.get("id"), Some(&serde_json::json!("evt_1")));

        // 形状 B: schema 2.0 header/event (毫秒时间戳折算为秒)
        let raw = r#"{"schema":"2.0","header":{"create_time":"1700000000000","token":"t","app_id":"app"},"event":{"id":"evt_2"}}"#;
        let event = WebhookEvent::from_raw_json(raw).expect("parse");
        assert_eq!(event.event_type, EventType::EventCallback);
        assert_eq!(event.timestamp_secs, 1_700_000_000);
        assert_eq!(event.app_id, "app");
    }

    #[test]
    fn encrypted_envelope_is_recognized_without_decrypting() {
        let raw = r#"{"encrypt":"ciphertext-blob"}"#;
        let event = WebhookEvent::from_raw_json(raw).expect("parse");
        assert_eq!(event.event_type, EventType::EventCallback);
        assert_eq!(event.encrypt.as_deref(), Some("ciphertext-blob"));
        assert!(event.event.is_empty());
    }

    #[test]
    fn unknown_envelope_shapes_fail_closed() {
        assert!(matches!(
            WebhookEvent::from_raw_json(r#"{"foo":"bar"}"#),
            Err(WireError::Unsupported("webhook_event_type"))
        ));
        assert!(matches!(
            WebhookEvent::from_raw_json("[1,2,3]"),
            Err(WireError::Unsupported("webhook_event_type"))
        ));
        assert!(matches!(
            WebhookEvent::from_raw_json(r#"{"type":"event_callback","token":"t"}"#),
            Err(WireError::Other(_))
        ));
        assert!(matches!(
            WebhookEvent::from_raw_json(
                r#"{"type":"event_callback","token":"t","ts":"yesterday","event":{}}"#
            ),
            Err(WireError::Other(_))
        ));
        assert!(matches!(
            WebhookEvent::from_raw_json("not json"),
            Err(WireError::Other(_))
        ));
    }

    #[test]
    fn oversized_envelope_is_rejected() {
        let huge = format!(
            r#"{{"type":"url_verification","challenge":"{}"}}"#,
            "x".repeat(MAX_WEBHOOK_CHUNK_BYTES)
        );
        assert!(matches!(
            WebhookEvent::from_raw_json(&huge),
            Err(WireError::Other(_))
        ));
    }

    #[test]
    fn text_content_serializes_to_the_platform_shape() {
        let content = TextContent::new("hello");
        content.validate().expect("empty @ list is valid");
        assert_eq!(
            serde_json::to_string(&content).expect("serialize"),
            r#"{"text":"hello"}"#
        );

        let mut with_at = TextContent::new("hi");
        with_at
            .at_open_ids
            .push("ou_user1234567890abcdef".to_string());
        with_at.validate().expect("valid open_id");
        let mut bad = TextContent::new("hi");
        bad.at_open_ids.push("user-without-prefix".to_string());
        assert!(matches!(bad.validate(), Err(WireError::OpenIdInvalid(_))));
    }

    #[test]
    fn card_content_plain_has_header_and_one_div_element() {
        let content = CardContent::plain("待审批操作", "shell: ls");
        assert_eq!(content.elements.len(), 1);
        assert_eq!(
            content.header.get("title"),
            Some(&serde_json::json!({"tag": "plain_text", "content": "待审批操作"}))
        );
        assert_eq!(
            content.elements[0].get("tag"),
            Some(&serde_json::Value::String("div".to_string()))
        );
        assert_eq!(
            content.elements[0].get("text"),
            Some(&serde_json::json!({"tag": "plain_text", "content": "shell: ls"}))
        );
    }

    #[test]
    fn message_budget_anchor_is_bounded() {
        assert_eq!(MAX_MESSAGE_TEXT_BYTES, 4096);
    }
}
