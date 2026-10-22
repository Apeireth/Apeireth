//! # lark webhook 面 (入站事件校验 + 加密事件解密)
//!
//! 事件订阅回调的完整协议逻辑 (本地完成, 0 网络):
//!
//! ## wire 契约 (三类入站信封, 未知字段容错)
//!
//! 1. **URL 校验**: `{"type": "url_verification", "challenge": "...", "token": "..."}`
//!    (兼容 `{"type": "challenge", ...}`) → 校验通过后原样返回 challenge。
//! 2. **明文事件回调**:
//!    - 形状 A: `{"type": "event_callback", "token": "...", "ts"|"create_time": <时间戳>,
//!      "app_id": "...", "event": {...}}`
//!    - 形状 B: `{"schema": "2.0", "header": {"event_id", "event_type", "create_time",
//!      "token", "app_id"}, "event": {...}}`
//! 3. **加密回调**: `{"encrypt": "<base64>"}` —— 整个信封被加密, 解密后是 1/2 之一。
//!    嵌套加密 (解出仍是加密信封) 一律拒绝, 防递归解密炸弹。
//!
//! ## 加密契约 (AES-256-CBC + SHA-256 密钥派生 + PKCS#7)
//!
//! - 密钥: `key = SHA-256(encrypt_key)` (32 字节);
//! - `blob = base64(encrypt 字段)`, `IV = blob[0..16]`, `密文 = blob[16..]`;
//! - 解密明文 = 16 字节随机前缀 (丢弃) + 事件 JSON;
//! - 帧长/填充严格校验: 畸形 base64 / 长度 / 填充 = 永久错误。
//!
//! ## 校验流程 (闭合步骤)
//!
//! 1. 加密信封 → 先解密再进入 2..4 (解密失败 = 永久错误, 0 静默放行);
//! 2. token 恒定时间比较 (失败消息只报 "mismatch", 0 回显共享秘密);
//! 3. 事件回调的重放窗口: 时间戳必填且与当前时间差 ≤ [`WEBHOOK_TIMESTAMP_SKEW_SECS`];
//! 4. URL 校验事件返回 challenge; 事件回调返回携带解密事件体的 `Accepted`;
//!    未知事件形状 = `Err(LarkError::Unsupported("webhook_event_type"))` (显式不支持)。
//!
//! 入口:
//! - [`WebhookEvent::from_raw_json`] — 原始回调 body → 领域事件;
//! - [`verify_webhook_event`] / [`verify_webhook_event_at`] — 校验 (后者供测试注入时间);
//! - [`decrypt_event_payload`] / [`encrypt_event_payload`] — 加解密 (后者供测试/演练构造夹具)。

use std::collections::HashMap;

use aes::cipher::{Block, BlockDecrypt, BlockEncrypt, KeyInit};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::lark::auth::WebhookToken;
use crate::lark::error::{LarkError, LarkResult};

/// 事件时间戳允许的最大偏差 (秒, 防重放)。
pub const WEBHOOK_TIMESTAMP_SKEW_SECS: u64 = 300;

/// 加密明文的随机前缀长度 (字节, 解密后丢弃)。
pub const EVENT_PAYLOAD_PREFIX_BYTES: usize = 16;

/// AES 分组长度 (字节)。
const BLOCK_BYTES: usize = 16;

// ============================================================================
// §1 EventType (4 variant 闭合枚举)
// ============================================================================

/// Webhook 事件类型 (4 variant 闭合枚举)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// URL 校验事件 (`url_verification`)。
    UrlVerification,
    /// 事件回调 (含加密信封 —— 解密前无法细分)。
    EventCallback,
    /// 兼容老式 challenge 字段 (`challenge`)。
    Challenge,
    /// 未知 (兜底; 校验时显式 `Unsupported`, 0 静默放行)。
    #[default]
    Unknown,
}

impl EventType {
    /// 4 variant hardcode 常量。
    pub const COUNT: usize = 4;

    /// wire 字符串。
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
// §2 WebhookEvent (领域事件)
// ============================================================================

/// Webhook 入站事件 (领域形态)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// 事件类型 (闭合枚举)。
    pub event_type: EventType,
    /// App ID (来源应用标识, 可空)。
    pub app_id: String,
    /// 校验 token (应与配置的 webhook token 一致)。
    pub token: String,
    /// 事件时间戳 (秒; 0 = 未提供, 事件回调会因此被拒)。
    pub timestamp_secs: u64,
    /// URL 校验 challenge (URL 校验事件专用)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
    /// 加密信封 (base64; 有值即先解密再校验)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypt: Option<String>,
    /// 事件体 (解密/校验后可得)。
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub event: HashMap<String, serde_json::Value>,
}

impl WebhookEvent {
    /// 构造 URL 校验事件。
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

    /// 构造事件回调。
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

    /// 构造加密信封事件 (整包 base64 密文)。
    pub fn new_encrypted(encrypt: String) -> Self {
        Self {
            event_type: EventType::EventCallback,
            app_id: String::new(),
            token: String::new(),
            timestamp_secs: 0,
            challenge: None,
            encrypt: Some(encrypt),
            event: HashMap::new(),
        }
    }

    /// 原始回调 body → 领域事件 (三类信封严格识别, 未知形状 = `Unsupported`)。
    pub fn from_raw_json(raw: &str) -> LarkResult<Self> {
        if raw.len() > crate::lark::MAX_WEBHOOK_CHUNK_BYTES {
            return Err(LarkError::Other(format!(
                "webhook payload too large: {} > {} bytes",
                raw.len(),
                crate::lark::MAX_WEBHOOK_CHUNK_BYTES
            )));
        }
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|_| LarkError::Other("webhook payload is not valid JSON".to_string()))?;
        let obj = value
            .as_object()
            .ok_or(LarkError::Unsupported("webhook_event_type"))?;

        // 信封 3: 加密回调 (整包密文, 内容解密后才可见)
        if let Some(encrypt) = obj.get("encrypt").and_then(|v| v.as_str()) {
            return Ok(Self::new_encrypted(encrypt.to_string()));
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
                Err(LarkError::Unsupported("webhook_event_type"))
            }
        }
    }
}

/// 事件体必须是 JSON 对象 (缺失/形状错 = 永久错误)。
fn require_event_object(
    value: &serde_json::Value,
) -> LarkResult<HashMap<String, serde_json::Value>> {
    let obj = value
        .get("event")
        .and_then(|e| e.as_object())
        .ok_or_else(|| LarkError::Other("webhook payload missing event object".to_string()))?;
    Ok(obj
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect::<HashMap<String, serde_json::Value>>())
}

/// 时间戳解析 (数字或数字字符串; ≥ 10^12 视为毫秒)。
///
/// 缺失/非数字 → 永久错误; 毫秒口径自动折算为秒。
fn parse_timestamp_secs(value: Option<&serde_json::Value>) -> LarkResult<u64> {
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
            .ok_or_else(|| LarkError::Other("webhook timestamp is not an integer".to_string()))?,
        serde_json::Value::String(s) => s
            .trim()
            .parse::<i128>()
            .map_err(|_| LarkError::Other("webhook timestamp is not numeric".to_string()))?,
        _ => {
            return Err(LarkError::Other(
                "webhook timestamp has wrong type".to_string(),
            ))
        }
    };
    if raw < 0 {
        return Err(LarkError::Other(
            "webhook timestamp is negative".to_string(),
        ));
    }
    let secs = if raw >= 1_000_000_000_000 {
        raw / 1000
    } else {
        raw
    };
    u64::try_from(secs).map_err(|_| LarkError::Other("webhook timestamp out of range".to_string()))
}

// ============================================================================
// §3 校验入口
// ============================================================================

/// 校验 webhook 事件 (当前时间口径)。
pub fn verify_webhook_event(
    event: &WebhookEvent,
    webhook_token: &WebhookToken,
) -> LarkResult<WebhookVerifyResult> {
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    verify_webhook_event_at(event, webhook_token, now_secs, WEBHOOK_TIMESTAMP_SKEW_SECS)
}

/// 校验 webhook 事件 (显式时间口径, 供测试/回放注入)。
///
/// 步骤见模块文档; 任何失败都是显式错误, 0 静默放行未校验事件体。
pub fn verify_webhook_event_at(
    event: &WebhookEvent,
    webhook_token: &WebhookToken,
    now_secs: u64,
    max_skew_secs: u64,
) -> LarkResult<WebhookVerifyResult> {
    // 加密信封: 解密后按普通事件校验 (拒绝嵌套加密)
    if let Some(encrypt) = &event.encrypt {
        let inner_json = decrypt_event_payload(encrypt, &webhook_token.encrypt_key)?;
        let inner = WebhookEvent::from_raw_json(&inner_json)?;
        if inner.encrypt.is_some() {
            return Err(LarkError::Other(
                "webhook nested encrypted envelope rejected".to_string(),
            ));
        }
        return verify_webhook_event_at(&inner, webhook_token, now_secs, max_skew_secs);
    }

    // token 恒定时间比较 (错误消息 0 回显共享秘密)
    if !webhook_token.verify(&event.token) {
        return Err(LarkError::Other("webhook token mismatch".to_string()));
    }

    match event.event_type {
        EventType::UrlVerification | EventType::Challenge => match &event.challenge {
            Some(challenge) if !challenge.is_empty() => {
                Ok(WebhookVerifyResult::Challenge(challenge.clone()))
            }
            _ => Err(LarkError::Other(
                "url_verification event missing challenge field".to_string(),
            )),
        },
        EventType::EventCallback => {
            if event.timestamp_secs == 0 {
                return Err(LarkError::Other(
                    "webhook event missing timestamp".to_string(),
                ));
            }
            let drift = now_secs.abs_diff(event.timestamp_secs);
            if drift > max_skew_secs {
                return Err(LarkError::Other(format!(
                    "webhook timestamp outside allowed window (drift={drift}s > {max_skew_secs}s)"
                )));
            }
            if event.event.is_empty() {
                return Err(LarkError::Other("webhook event body is empty".to_string()));
            }
            Ok(WebhookVerifyResult::Accepted {
                event: event.event.clone(),
            })
        }
        EventType::Unknown => Err(LarkError::Unsupported("webhook_event_type")),
    }
}

/// Webhook 校验结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookVerifyResult {
    /// URL 校验通过: 客户端应原样返回 challenge。
    Challenge(String),
    /// 事件已校验 (含解密完成): 携带验证后的事件体。
    Accepted {
        /// 验证后的事件体。
        event: HashMap<String, serde_json::Value>,
    },
}

// ============================================================================
// §4 加密契约 (AES-256-CBC + SHA-256 派生密钥 + PKCS#7)
// ============================================================================

/// 派生事件加密密钥: `key = SHA-256(encrypt_key)`。
pub fn derive_event_key(encrypt_key: &str) -> [u8; 32] {
    let digest = Sha256::digest(encrypt_key.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

/// 解密加密事件体 (契约见模块文档; 任何畸形 = 永久错误)。
///
/// 返回事件 JSON 字符串 (已去掉 16 字节随机前缀)。
pub fn decrypt_event_payload(encrypt_b64: &str, encrypt_key: &str) -> LarkResult<String> {
    let blob = base64::engine::general_purpose::STANDARD
        .decode(encrypt_b64.trim())
        .map_err(|_| LarkError::Other("webhook payload base64 is malformed".to_string()))?;
    if blob.len() < 2 * BLOCK_BYTES || blob.len() % BLOCK_BYTES != 0 {
        return Err(LarkError::Other(format!(
            "webhook payload has invalid frame length: {} bytes",
            blob.len()
        )));
    }
    let (iv, ciphertext) = blob.split_at(BLOCK_BYTES);
    let key = derive_event_key(encrypt_key);
    let padded = aes256_cbc_decrypt(&key, iv, ciphertext)?;
    let plain = pkcs7_unpad(&padded)?;
    if plain.len() < EVENT_PAYLOAD_PREFIX_BYTES {
        return Err(LarkError::Other(
            "webhook payload plaintext is too short".to_string(),
        ));
    }
    let json_bytes = &plain[EVENT_PAYLOAD_PREFIX_BYTES..];
    std::str::from_utf8(json_bytes)
        .map(str::to_string)
        .map_err(|_| LarkError::Other("webhook payload plaintext is not UTF-8".to_string()))
}

/// 加密事件体 (契约见模块文档) —— 测试/演练夹具构造入口。
///
/// `iv` 由调用方给出 (真实回调每包随机); 明文前缀固定 16 字节 0
/// (真实回调是随机前缀, 解密侧按契约丢弃, 内容不受影响)。
pub fn encrypt_event_payload(
    event_json: &str,
    encrypt_key: &str,
    iv: [u8; BLOCK_BYTES],
) -> LarkResult<String> {
    let mut plain = vec![0u8; EVENT_PAYLOAD_PREFIX_BYTES];
    plain.extend_from_slice(event_json.as_bytes());
    let padded = pkcs7_pad(&plain);
    let key = derive_event_key(encrypt_key);
    let ciphertext = aes256_cbc_encrypt(&key, &iv, &padded);
    let mut blob = Vec::with_capacity(BLOCK_BYTES + ciphertext.len());
    blob.extend_from_slice(&iv);
    blob.extend_from_slice(&ciphertext);
    Ok(base64::engine::general_purpose::STANDARD.encode(blob))
}

fn aes256_cbc_decrypt(key: &[u8; 32], iv: &[u8], ciphertext: &[u8]) -> LarkResult<Vec<u8>> {
    let cipher = aes::Aes256::new_from_slice(key)
        .map_err(|_| LarkError::Other("aes key init failed".to_string()))?;
    let mut prev: Vec<u8> = iv.to_vec();
    let mut out = Vec::with_capacity(ciphertext.len());
    for chunk in ciphertext.chunks_exact(BLOCK_BYTES) {
        let mut block = Block::<aes::Aes256>::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        let mut plain_block = block.to_vec();
        for i in 0..BLOCK_BYTES {
            plain_block[i] ^= prev[i];
        }
        out.extend_from_slice(&plain_block);
        prev = chunk.to_vec();
    }
    Ok(out)
}

fn aes256_cbc_encrypt(key: &[u8; 32], iv: &[u8], padded_plain: &[u8]) -> Vec<u8> {
    let cipher = aes::Aes256::new_from_slice(key).expect("aes key length is fixed at 32 bytes");
    let mut prev: Vec<u8> = iv.to_vec();
    let mut out = Vec::with_capacity(padded_plain.len());
    for chunk in padded_plain.chunks_exact(BLOCK_BYTES) {
        let mut block = Block::<aes::Aes256>::clone_from_slice(chunk);
        for i in 0..BLOCK_BYTES {
            block[i] ^= prev[i];
        }
        cipher.encrypt_block(&mut block);
        out.extend_from_slice(&block);
        prev = block.to_vec();
    }
    out
}

fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let pad = BLOCK_BYTES - (data.len() % BLOCK_BYTES);
    let mut out = data.to_vec();
    out.extend(std::iter::repeat(pad as u8).take(pad));
    out
}

fn pkcs7_unpad(data: &[u8]) -> LarkResult<Vec<u8>> {
    if data.is_empty() || data.len() % BLOCK_BYTES != 0 {
        return Err(LarkError::Other(
            "webhook payload padding frame is invalid".to_string(),
        ));
    }
    let pad = *data.last().expect("non-empty") as usize;
    if pad == 0 || pad > BLOCK_BYTES || pad > data.len() {
        return Err(LarkError::Other(
            "webhook payload padding value is invalid".to_string(),
        ));
    }
    let (body, padding) = data.split_at(data.len() - pad);
    if !padding.iter().all(|&b| b == pad as u8) {
        return Err(LarkError::Other(
            "webhook payload padding bytes are inconsistent".to_string(),
        ));
    }
    Ok(body.to_vec())
}

// ============================================================================
// §5 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn token() -> WebhookToken {
        WebhookToken::new("token_xxx".to_string(), "encrypt_key_xxx".to_string()).expect("valid")
    }

    fn raw_event_callback(ts: u64) -> String {
        format!(
            r#"{{"type":"event_callback","token":"token_xxx","app_id":"cli_a1b2c3d4e5f6","ts":{ts},"event":{{"type":"im.message.receive_v1","id":"evt_1"}}}}"#
        )
    }

    #[test]
    fn event_type_4_variants() {
        assert_eq!(EventType::COUNT, 4);
    }

    #[test]
    fn from_raw_json_parses_url_verification_with_unknown_field_tolerance() {
        let raw = r#"{"type":"url_verification","challenge":"test-challenge-12345","token":"token_xxx","future":42}"#;
        let event = WebhookEvent::from_raw_json(raw).expect("parse");
        assert_eq!(event.event_type, EventType::UrlVerification);
        assert_eq!(event.challenge.as_deref(), Some("test-challenge-12345"));
    }

    #[test]
    fn from_raw_json_parses_event_callback_shapes() {
        // 形状 A
        let event = WebhookEvent::from_raw_json(&raw_event_callback(1_700_000_000)).expect("parse");
        assert_eq!(event.event_type, EventType::EventCallback);
        assert_eq!(event.timestamp_secs, 1_700_000_000);
        assert_eq!(event.token, "token_xxx");
        assert!(!event.event.is_empty());

        // 形状 B (header/event)
        let raw = r#"{"schema":"2.0","header":{"event_id":"e1","event_type":"im.message.receive_v1","create_time":"1700000000000","token":"token_xxx","app_id":"cli_a1b2c3d4e5f6"},"event":{"id":"evt_1"}}"#;
        let event = WebhookEvent::from_raw_json(raw).expect("parse");
        assert_eq!(event.event_type, EventType::EventCallback);
        assert_eq!(event.timestamp_secs, 1_700_000_000, "毫秒时间戳应折算为秒");
        assert_eq!(event.app_id, "cli_a1b2c3d4e5f6");
    }

    #[test]
    fn from_raw_json_rejects_unknown_shapes() {
        assert!(matches!(
            WebhookEvent::from_raw_json(r#"{"foo":"bar"}"#),
            Err(LarkError::Unsupported("webhook_event_type"))
        ));
        assert!(matches!(
            WebhookEvent::from_raw_json("[1,2,3]"),
            Err(LarkError::Unsupported("webhook_event_type"))
        ));
        // event_callback 缺 event 对象 → 永久错误
        assert!(matches!(
            WebhookEvent::from_raw_json(r#"{"type":"event_callback","token":"t"}"#),
            Err(LarkError::Other(_))
        ));
        // 时间戳非数字 → 永久错误
        assert!(matches!(
            WebhookEvent::from_raw_json(
                r#"{"type":"event_callback","token":"t","ts":"yesterday","event":{}}"#
            ),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn url_verification_round_trip() {
        let event = WebhookEvent::from_raw_json(
            r#"{"type":"url_verification","challenge":"test-challenge-12345","token":"token_xxx"}"#,
        )
        .expect("parse");
        let result = verify_webhook_event(&event, &token()).expect("verify");
        assert!(matches!(
            result,
            WebhookVerifyResult::Challenge(ref c) if c == "test-challenge-12345"
        ));
    }

    #[test]
    fn url_verification_rejects_missing_challenge() {
        let event =
            WebhookEvent::from_raw_json(r#"{"type":"url_verification","token":"token_xxx"}"#)
                .expect("parse");
        assert!(matches!(
            verify_webhook_event(&event, &token()),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn token_mismatch_does_not_leak_shared_secret() {
        let event = WebhookEvent::from_raw_json(
            r#"{"type":"url_verification","challenge":"x","token":"wrong_token"}"#,
        )
        .expect("parse");
        match verify_webhook_event(&event, &token()) {
            Err(LarkError::Other(msg)) => {
                assert!(msg.contains("mismatch"), "错误应报 mismatch: {msg}");
                assert!(!msg.contains("token_xxx"), "0 回显期望 token: {msg}");
                assert!(!msg.contains("wrong_token"), "0 回显入站 token: {msg}");
            }
            other => panic!("expected Other(mismatch), got {other:?}"),
        }
    }

    #[test]
    fn event_callback_replay_window_is_enforced() {
        let now = 1_700_000_000u64;
        let within = WebhookEvent::from_raw_json(&raw_event_callback(now - 60)).expect("parse");
        let result = verify_webhook_event_at(&within, &token(), now, WEBHOOK_TIMESTAMP_SKEW_SECS)
            .expect("ok");
        assert!(matches!(result, WebhookVerifyResult::Accepted { .. }));

        // 过旧 (超出窗口) → 拒
        let stale = WebhookEvent::from_raw_json(&raw_event_callback(now - 10_000)).expect("parse");
        assert!(matches!(
            verify_webhook_event_at(&stale, &token(), now, WEBHOOK_TIMESTAMP_SKEW_SECS),
            Err(LarkError::Other(_))
        ));

        // 缺时间戳 → 拒
        let mut missing = WebhookEvent::from_raw_json(&raw_event_callback(now)).expect("parse");
        missing.timestamp_secs = 0;
        assert!(matches!(
            verify_webhook_event_at(&missing, &token(), now, WEBHOOK_TIMESTAMP_SKEW_SECS),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn unknown_event_type_is_explicitly_unsupported() {
        let mut ev = HashMap::new();
        ev.insert(
            "type".to_string(),
            serde_json::json!("im.message.receive_v1"),
        );
        let event = WebhookEvent {
            event_type: EventType::Unknown,
            app_id: "cli_a1b2c3d4e5f6".to_string(),
            token: "token_xxx".to_string(),
            timestamp_secs: 1_700_000_000,
            challenge: None,
            encrypt: None,
            event: ev,
        };
        assert!(matches!(
            verify_webhook_event_at(&event, &token(), 1_700_000_000, WEBHOOK_TIMESTAMP_SKEW_SECS),
            Err(LarkError::Unsupported("webhook_event_type"))
        ));
    }

    // ---- 加密契约 ----

    #[test]
    fn encrypt_decrypt_round_trip() {
        let json = r#"{"type":"event_callback","token":"token_xxx","ts":1700000000,"event":{"id":"evt_1"}}"#;
        let iv = [7u8; 16];
        let blob = encrypt_event_payload(json, "encrypt_key_xxx", iv).expect("encrypt");
        let plain = decrypt_event_payload(&blob, "encrypt_key_xxx").expect("decrypt");
        assert_eq!(plain, json);
    }

    #[test]
    fn encrypted_callback_verifies_end_to_end() {
        let inner = raw_event_callback(1_700_000_000);
        let blob = encrypt_event_payload(&inner, "encrypt_key_xxx", [3u8; 16]).expect("encrypt");
        let raw = format!(r#"{{"encrypt":"{blob}"}}"#);
        let event = WebhookEvent::from_raw_json(&raw).expect("parse");
        assert!(event.encrypt.is_some());
        let result =
            verify_webhook_event_at(&event, &token(), 1_700_000_000, WEBHOOK_TIMESTAMP_SKEW_SECS)
                .expect("verify");
        match result {
            WebhookVerifyResult::Accepted { event } => {
                assert_eq!(
                    event.get("id"),
                    Some(&serde_json::json!("evt_1")),
                    "解密后的事件体应完整返回"
                );
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn encrypted_callback_rejects_wrong_key_and_malformed_frame() {
        let inner = raw_event_callback(1_700_000_000);
        let blob = encrypt_event_payload(&inner, "encrypt_key_xxx", [3u8; 16]).expect("encrypt");
        // 换 key 解不开 → 永久错误 (填充/UTF-8 校验兜底)
        assert!(matches!(
            decrypt_event_payload(&blob, "another_key"),
            Err(LarkError::Other(_))
        ));
        // 非 base64 → 拒
        assert!(matches!(
            decrypt_event_payload("!!!not-base64!!!", "encrypt_key_xxx"),
            Err(LarkError::Other(_))
        ));
        // 帧长非法 (非 16 倍数) → 拒
        let short = base64::engine::general_purpose::STANDARD.encode([0u8; 17]);
        assert!(matches!(
            decrypt_event_payload(&short, "encrypt_key_xxx"),
            Err(LarkError::Other(_))
        ));
    }

    #[test]
    fn nested_encrypted_envelope_is_rejected() {
        // 内层再套一层加密信封 → 拒 (防递归解密炸弹)
        let inner_encrypted =
            encrypt_event_payload(r#"{"encrypt":"doubled"}"#, "encrypt_key_xxx", [5u8; 16])
                .expect("encrypt");
        let outer = encrypt_event_payload(
            &format!(r#"{{"encrypt":"{inner_encrypted}"}}"#),
            "encrypt_key_xxx",
            [9u8; 16],
        )
        .expect("encrypt");
        let event = WebhookEvent::new_encrypted(outer);
        assert!(matches!(
            verify_webhook_event_at(&event, &token(), 1_700_000_000, WEBHOOK_TIMESTAMP_SKEW_SECS),
            Err(LarkError::Other(_))
        ));
    }
}
