//! IM 快捷接入: 入站信封解码 + 出站信封组装 (归一化 wire 面)。
//!
//! 归一化口径: 三个 kind 的回调体都解码成同一个 [`ImInboundEvent`]
//! (文本消息 [`ImInboundMessage`] / 卡片按钮 [`ImCardAction`] / 握手
//! [`ImInboundEvent::Verify`] / 其余 [`ImInboundEvent::Ignored`]),
//! 未知形状 = [`crate::im::error::ImError::Decode`] (0 静默放行)。
//!
//! - `im-feishu` 入站信封**消费** [`crate::lark::webhook::WebhookEvent`] 的
//!   严格解析 (三种信封形状 + 时间戳口径), 本面只做消息/按钮字段提取;
//! - `im-wecom` / `im-qq` 入站按本适配器契约解析 (形状见各 `parse_*`);
//! - 出站信封按 kind 组装后 POST 到配置的 webhook / endpoint,
//!   文本预算走 [`crate::im::segment`] 分段策略。

use serde::{Deserialize, Serialize};

use crate::im::card::{ImApprovalCard, ImButtonPayload};
use crate::im::channel::ImChannelKind;
use crate::im::error::{ImError, ImResult};

/// 入站事件 (归一化)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImInboundEvent {
    /// 对端握手/校验挑战 (HTTP 层回显 challenge)。
    Verify {
        /// 待回显的 challenge (可空)。
        challenge: Option<String>,
    },
    /// 一条文本消息。
    Message(ImInboundMessage),
    /// 一次卡片按钮回调。
    CardAction(ImCardAction),
    /// 已识别但不需要处理的事件 (附事实原因)。
    Ignored {
        /// 事实原因。
        reason: &'static str,
    },
}

/// 归一化入站文本消息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImInboundMessage {
    /// IM 会话 id (映射到桌面会话的 key 之一)。
    pub conversation_id: String,
    /// 平台消息 id。
    pub message_id: String,
    /// 发送者 id。
    pub sender_id: String,
    /// 文本内容。
    pub text: String,
    /// 平台时间戳 (epoch 毫秒; 0 = 平台未给)。
    pub received_at_ms: i64,
}

/// 归一化卡片按钮回调。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImCardAction {
    /// IM 会话 id。
    pub conversation_id: String,
    /// 卡片所在消息 id (可空)。
    pub message_id: String,
    /// 点击者 id。
    pub operator_id: String,
    /// 按钮载荷 (闭合身份 + 超时戳 + action)。
    pub payload: ImButtonPayload,
}

/// 解码一条入站回调体 (kind 决定信封方言)。
pub fn parse_inbound_event(kind: ImChannelKind, raw: &str) -> ImResult<ImInboundEvent> {
    match kind {
        ImChannelKind::ImFeishu => parse_feishu_inbound(raw),
        ImChannelKind::ImWecom => parse_wecom_inbound(raw),
        ImChannelKind::ImQq => parse_qq_inbound(raw),
    }
}

// ---------------------------------------------------------------------------
// im-feishu 入站: 消费 lark webhook 信封解析
// ---------------------------------------------------------------------------

fn parse_feishu_inbound(raw: &str) -> ImResult<ImInboundEvent> {
    let event = crate::lark::webhook::WebhookEvent::from_raw_json(raw).map_err(map_lark_error)?;
    if let Some(challenge) = event.challenge.clone() {
        return Ok(ImInboundEvent::Verify {
            challenge: Some(challenge),
        });
    }
    if event.event_type == crate::lark::webhook::EventType::Challenge {
        return Ok(ImInboundEvent::Verify { challenge: None });
    }

    let body = &event.event;
    if let Some(message) = body.get("message") {
        return Ok(ImInboundEvent::Message(ImInboundMessage {
            conversation_id: required_str(message, "chat_id")?,
            message_id: required_str(message, "message_id")?,
            sender_id: body
                .get("sender")
                .and_then(|sender| sender.get("sender_id"))
                .and_then(|ids| ids.get("open_id"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            text: text_of(message)?,
            received_at_ms: 0,
        }));
    }
    if let Some(action) = body.get("action") {
        let payload = button_payload_of(action.get("value"))?;
        let context = body.get("context");
        return Ok(ImInboundEvent::CardAction(ImCardAction {
            conversation_id: context
                .and_then(|ctx| ctx.get("open_chat_id"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            message_id: context
                .and_then(|ctx| ctx.get("open_message_id"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            operator_id: body
                .get("operator")
                .and_then(|op| op.get("open_id"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            payload,
        }));
    }
    Ok(ImInboundEvent::Ignored {
        reason: "unhandled im-feishu event body",
    })
}

fn map_lark_error(error: crate::lark::error::LarkError) -> ImError {
    ImError::Decode {
        reason: error.to_string(),
    }
}

/// 消息文本: `content` 是 `{"text": ...}` 的 JSON 字符串, 也允许直接文本。
fn text_of(message: &serde_json::Value) -> ImResult<String> {
    let content = required_str(message, "content")?;
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
        if let Some(text) = parsed.get("text").and_then(serde_json::Value::as_str) {
            return Ok(text.to_string());
        }
    }
    if content.trim().is_empty() {
        return Err(ImError::Decode {
            reason: "message content is empty".to_string(),
        });
    }
    Ok(content)
}

// ---------------------------------------------------------------------------
// im-wecom 入站: 文本消息 {msgtype,text:{content},chatid,msgid,from:{userid}}
// 按钮回调 {action:"callback",action_value:<json>,chatid,msgid,operator}
// ---------------------------------------------------------------------------

fn parse_wecom_inbound(raw: &str) -> ImResult<ImInboundEvent> {
    let value: serde_json::Value = parse_json(raw)?;
    if value
        .get("echo")
        .and_then(serde_json::Value::as_str)
        .is_some()
    {
        return Ok(ImInboundEvent::Verify {
            challenge: value
                .get("echo")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        });
    }
    if value.get("action").and_then(serde_json::Value::as_str) == Some("callback") {
        let payload = button_payload_of(value.get("action_value"))?;
        return Ok(ImInboundEvent::CardAction(ImCardAction {
            conversation_id: required_str(&value, "chatid")?,
            message_id: optional_str(&value, "msgid"),
            operator_id: optional_str(&value, "operator"),
            payload,
        }));
    }
    if value.get("msgtype").and_then(serde_json::Value::as_str) == Some("text") {
        let text = value
            .get("text")
            .and_then(|text| text.get("content"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ImError::Decode {
                reason: "im-wecom text message is missing text.content".to_string(),
            })?;
        return Ok(ImInboundEvent::Message(ImInboundMessage {
            conversation_id: required_str(&value, "chatid")?,
            message_id: optional_str(&value, "msgid"),
            sender_id: value
                .get("from")
                .and_then(|from| from.get("userid"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            text: text.to_string(),
            received_at_ms: 0,
        }));
    }
    Ok(ImInboundEvent::Ignored {
        reason: "unhandled im-wecom message type",
    })
}

// ---------------------------------------------------------------------------
// im-qq 入站: 文本消息 {msg_id,channel_id,author:{id},content,timestamp}
// 按钮回调 {data:{resolved_button_id,value,message_id,channel_id,user_id}}
// ---------------------------------------------------------------------------

fn parse_qq_inbound(raw: &str) -> ImResult<ImInboundEvent> {
    let value: serde_json::Value = parse_json(raw)?;
    if let Some(data) = value.get("data") {
        let payload = button_payload_of(data.get("value"))?;
        return Ok(ImInboundEvent::CardAction(ImCardAction {
            conversation_id: optional_str(data, "channel_id"),
            message_id: optional_str(data, "message_id"),
            operator_id: optional_str(data, "user_id"),
            payload,
        }));
    }
    if let Some(content) = value.get("content").and_then(serde_json::Value::as_str) {
        return Ok(ImInboundEvent::Message(ImInboundMessage {
            conversation_id: required_str(&value, "channel_id")?,
            message_id: optional_str(&value, "msg_id"),
            sender_id: value
                .get("author")
                .and_then(|author| author.get("id"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            text: content.to_string(),
            received_at_ms: parse_epoch_ms(value.get("timestamp")),
        }));
    }
    Ok(ImInboundEvent::Ignored {
        reason: "unhandled im-qq event body",
    })
}

// ---------------------------------------------------------------------------
// 出站信封
// ---------------------------------------------------------------------------

/// 一条待发文本 (已分段)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImOutboundText {
    /// 目标 IM 会话 id。
    pub conversation_id: String,
    /// 文本内容 (不超过该 kind 的字节预算)。
    pub text: String,
}

/// 出站回执。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImSendReceipt {
    /// 对端颁发的消息引用 (对端未给时是本地接受标记)。
    pub message_ref: String,
}

/// 组装文本出站信封 (kind 方言; `im-feishu` 消费 [`crate::lark::message`] 内容形状)。
pub fn build_text_body(
    kind: ImChannelKind,
    message: &ImOutboundText,
) -> ImResult<serde_json::Value> {
    if message.text.is_empty() {
        return Err(ImError::InvalidConfig {
            reason: "outbound text must not be empty".to_string(),
        });
    }
    match kind {
        ImChannelKind::ImFeishu => {
            let content = crate::lark::message::TextContent::new(message.text.clone());
            content.validate().map_err(map_lark_error)?;
            let content_json =
                serde_json::to_string(&content).map_err(|e| ImError::InvalidConfig {
                    reason: format!("text content serialization failed: {e}"),
                })?;
            Ok(serde_json::json!({
                "receive_id": message.conversation_id,
                "msg_type": "text",
                "content": content_json,
            }))
        }
        ImChannelKind::ImWecom => Ok(serde_json::json!({
            "chatid": message.conversation_id,
            "msgtype": "text",
            "text": {"content": message.text},
        })),
        ImChannelKind::ImQq => Ok(serde_json::json!({
            "channel_id": message.conversation_id,
            "msg_type": "text",
            "content": message.text,
        })),
    }
}

/// 组装审批卡片出站信封 (kind 方言; 卡片本体由 [`crate::im::card`] 渲染)。
pub fn build_card_body(
    kind: ImChannelKind,
    conversation_id: &str,
    card: &ImApprovalCard,
) -> ImResult<serde_json::Value> {
    let rendered = crate::im::card::render_approval_card(kind, card)?;
    let envelope = match kind {
        ImChannelKind::ImFeishu => {
            let content_json =
                serde_json::to_string(&rendered).map_err(|e| ImError::InvalidConfig {
                    reason: format!("card content serialization failed: {e}"),
                })?;
            serde_json::json!({
                "receive_id": conversation_id,
                "msg_type": "interactive",
                "content": content_json,
            })
        }
        ImChannelKind::ImWecom => serde_json::json!({
            "chatid": conversation_id,
            "msgtype": "approval_card",
            "card": rendered.get("card"),
        }),
        ImChannelKind::ImQq => serde_json::json!({
            "channel_id": conversation_id,
            "msgtype": "approval_card",
            "card": rendered.get("card"),
        }),
    };
    Ok(envelope)
}

// ---------------------------------------------------------------------------
// 共享小工具
// ---------------------------------------------------------------------------

fn parse_json(raw: &str) -> ImResult<serde_json::Value> {
    serde_json::from_str(raw).map_err(|e| ImError::Decode {
        reason: format!("inbound payload is not valid JSON: {e}"),
    })
}

fn required_str(value: &serde_json::Value, key: &str) -> ImResult<String> {
    optional_str_value(value, key).ok_or_else(|| ImError::Decode {
        reason: format!("inbound payload is missing {key}"),
    })
}

fn optional_str(value: &serde_json::Value, key: &str) -> String {
    optional_str_value(value, key).unwrap_or_default()
}

fn optional_str_value(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// 按钮载荷提取: 接受对象或 JSON 字符串两种携带方式。
fn button_payload_of(value: Option<&serde_json::Value>) -> ImResult<ImButtonPayload> {
    let value = value.ok_or_else(|| ImError::Decode {
        reason: "button callback is missing its payload".to_string(),
    })?;
    let payload: ImButtonPayload = match value {
        serde_json::Value::String(raw) => {
            serde_json::from_str(raw).map_err(|e| ImError::Decode {
                reason: format!("button payload could not be decoded: {e}"),
            })?
        }
        serde_json::Value::Object(_) => {
            serde_json::from_value(value.clone()).map_err(|e| ImError::Decode {
                reason: format!("button payload could not be decoded: {e}"),
            })?
        }
        _ => {
            return Err(ImError::Decode {
                reason: "button payload has the wrong shape".to_string(),
            })
        }
    };
    payload.validate()?;
    Ok(payload)
}

fn parse_epoch_ms(value: Option<&serde_json::Value>) -> i64 {
    match value {
        Some(serde_json::Value::Number(number)) => number.as_i64().unwrap_or(0),
        Some(serde_json::Value::String(raw)) => raw.trim().parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::im::card::{IM_BUTTON_APPROVE, IM_BUTTON_REJECT};

    fn payload(action: &str) -> ImButtonPayload {
        ImButtonPayload {
            approval_ref: "apr_1".to_string(),
            pair_id: "pair-1".to_string(),
            round: 1,
            subject: "tool.shell".to_string(),
            expires_at_ms: 1_700_000_300_000,
            action: action.to_string(),
        }
    }

    fn card() -> ImApprovalCard {
        ImApprovalCard {
            approval_ref: "apr_1".to_string(),
            pair_id: "pair-1".to_string(),
            round: 1,
            subject: "tool.shell".to_string(),
            command_text: "shell: bash -c 'ls'".to_string(),
            arguments_summary: "执行命令 ls".to_string(),
            risk_level: "high".to_string(),
            governance_reason: "write effect".to_string(),
            created_at_ms: 1_700_000_000_000,
            expires_at_ms: 1_700_000_300_000,
        }
    }

    #[test]
    fn feishu_message_event_decodes_to_normalized_message() {
        let raw = serde_json::json!({
            "header": {"event_type": "im.message.receive_v1", "app_id": "app", "token": "t"},
            "event": {
                "message": {
                    "chat_id": "oc_demo",
                    "message_id": "om_1",
                    "content": "{\"text\":\"hello world\"}",
                },
                "sender": {"sender_id": {"open_id": "ou_1"}},
            }
        })
        .to_string();
        let event = parse_inbound_event(ImChannelKind::ImFeishu, &raw).unwrap();
        match event {
            ImInboundEvent::Message(message) => {
                assert_eq!(message.conversation_id, "oc_demo");
                assert_eq!(message.message_id, "om_1");
                assert_eq!(message.sender_id, "ou_1");
                assert_eq!(message.text, "hello world");
            }
            other => panic!("expected a message, got {other:?}"),
        }
    }

    #[test]
    fn feishu_button_event_decodes_the_closure_payload() {
        let raw = serde_json::json!({
            "header": {"event_type": "card.action.trigger", "app_id": "app", "token": "t"},
            "event": {
                "operator": {"open_id": "ou_2"},
                "action": {"tag": "button", "value": payload(IM_BUTTON_APPROVE)},
                "context": {"open_chat_id": "oc_demo", "open_message_id": "om_9"},
            }
        })
        .to_string();
        let event = parse_inbound_event(ImChannelKind::ImFeishu, &raw).unwrap();
        match event {
            ImInboundEvent::CardAction(action) => {
                assert_eq!(action.conversation_id, "oc_demo");
                assert_eq!(action.message_id, "om_9");
                assert_eq!(action.operator_id, "ou_2");
                assert_eq!(action.payload, payload(IM_BUTTON_APPROVE));
            }
            other => panic!("expected a card action, got {other:?}"),
        }
    }

    #[test]
    fn feishu_verification_challenge_is_echoed() {
        let raw = r#"{"type":"url_verification","challenge":"nonce-1"}"#;
        match parse_inbound_event(ImChannelKind::ImFeishu, raw).unwrap() {
            ImInboundEvent::Verify { challenge } => {
                assert_eq!(challenge.as_deref(), Some("nonce-1"))
            }
            other => panic!("expected a verify event, got {other:?}"),
        }
    }

    #[test]
    fn wecom_and_qq_envelopes_decode_messages_and_buttons() {
        let wecom_text = serde_json::json!({
            "msgtype": "text",
            "text": {"content": "hi there"},
            "chatid": "wm_1",
            "msgid": "msg_1",
            "from": {"userid": "u_1"},
        })
        .to_string();
        match parse_inbound_event(ImChannelKind::ImWecom, &wecom_text).unwrap() {
            ImInboundEvent::Message(message) => {
                assert_eq!(message.conversation_id, "wm_1");
                assert_eq!(message.text, "hi there");
            }
            other => panic!("expected a message, got {other:?}"),
        }

        let wecom_button = serde_json::json!({
            "action": "callback",
            "action_value": serde_json::to_string(&payload(IM_BUTTON_REJECT)).unwrap(),
            "chatid": "wm_1",
            "msgid": "msg_2",
            "operator": "u_1",
        })
        .to_string();
        match parse_inbound_event(ImChannelKind::ImWecom, &wecom_button).unwrap() {
            ImInboundEvent::CardAction(action) => {
                assert_eq!(action.payload.action, IM_BUTTON_REJECT)
            }
            other => panic!("expected a card action, got {other:?}"),
        }

        let qq_text = serde_json::json!({
            "msg_id": "m_1",
            "channel_id": "qc_1",
            "author": {"id": "u_2"},
            "content": "qq hello",
            "timestamp": 1_700_000_000_000_i64,
        })
        .to_string();
        match parse_inbound_event(ImChannelKind::ImQq, &qq_text).unwrap() {
            ImInboundEvent::Message(message) => {
                assert_eq!(message.conversation_id, "qc_1");
                assert_eq!(message.text, "qq hello");
                assert_eq!(message.received_at_ms, 1_700_000_000_000);
            }
            other => panic!("expected a message, got {other:?}"),
        }

        let qq_button = serde_json::json!({
            "data": {
                "resolved_button_id": IM_BUTTON_APPROVE,
                "value": payload(IM_BUTTON_APPROVE),
                "message_id": "m_2",
                "channel_id": "qc_1",
                "user_id": "u_2",
            }
        })
        .to_string();
        match parse_inbound_event(ImChannelKind::ImQq, &qq_button).unwrap() {
            ImInboundEvent::CardAction(action) => {
                assert_eq!(action.payload.action, IM_BUTTON_APPROVE)
            }
            other => panic!("expected a card action, got {other:?}"),
        }
    }

    #[test]
    fn malformed_payloads_fail_closed() {
        assert!(matches!(
            parse_inbound_event(ImChannelKind::ImWecom, "not json"),
            Err(ImError::Decode { .. })
        ));
        let missing_chat = serde_json::json!({
            "msgtype": "text",
            "text": {"content": "hi"},
            "msgid": "m",
        })
        .to_string();
        assert!(matches!(
            parse_inbound_event(ImChannelKind::ImWecom, &missing_chat),
            Err(ImError::Decode { .. })
        ));
        let bad_button = serde_json::json!({
            "action": "callback",
            "action_value": "{\"approval_ref\":\"a\"}",
            "chatid": "wm_1",
        })
        .to_string();
        assert!(matches!(
            parse_inbound_event(ImChannelKind::ImWecom, &bad_button),
            Err(ImError::Decode { .. })
        ));
    }

    #[test]
    fn outbound_envelopes_carry_the_conversation_and_payload() {
        let message = ImOutboundText {
            conversation_id: "oc_demo".to_string(),
            text: "hello".to_string(),
        };
        for kind in ImChannelKind::ALL {
            let body = build_text_body(kind, &message).unwrap();
            let text = body.to_string();
            assert!(text.contains("oc_demo"), "{kind}: {text}");
            assert!(text.contains("hello"), "{kind}: {text}");
        }
        assert!(matches!(
            build_text_body(
                ImChannelKind::ImWecom,
                &ImOutboundText {
                    conversation_id: "oc".into(),
                    text: String::new()
                }
            ),
            Err(ImError::InvalidConfig { .. })
        ));

        let card_body = build_card_body(ImChannelKind::ImFeishu, "oc_demo", &card()).unwrap();
        let text = card_body.to_string();
        assert!(text.contains("interactive"), "{text}");
        assert!(text.contains("pair-1"), "{text}");
    }
}
