//! 表达模块: ActionIntent + ExpressionChannel + StructuredOutput。

use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::ActionTarget;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::silence::SilenceReason;
use crate::{ActionEngine, ActionExpression, ActionSilence};

/// 表达通道 — 内部意图投影到外部世界的 4 种形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpressionChannel {
    /// 纯文字 (CLI/日志).
    Text,
    /// 语音 (TTS 输出, 真实硬件留给阶段 7).
    Voice,
    /// 多模态 (文字 + 图像 + 视频).
    MultiModal,
    /// 结构化 (JSON / protobuf / 内部 RPC).
    Structured,
}

impl ExpressionChannel {
    /// 通道显示名.
    pub const fn name(&self) -> &'static str {
        match self {
            ExpressionChannel::Text => "text",
            ExpressionChannel::Voice => "voice",
            ExpressionChannel::MultiModal => "multi_modal",
            ExpressionChannel::Structured => "structured",
        }
    }

    /// 是否包含文字 (Text + MultiModal).
    pub fn has_text(&self) -> bool {
        matches!(
            self,
            ExpressionChannel::Text | ExpressionChannel::MultiModal
        )
    }
}

/// 待表达的内部意图 — 由 cognition 器官产出, action 器官投影到通道。
#[derive(Debug, Clone)]
pub struct ActionIntent {
    /// 唯一 intent ID.
    pub intent_id: Uuid,
    /// 关联的 action target.
    pub action: ActionTarget,
    /// 发言方 (默认 "assistant").
    pub speaker: String,
    /// 受众 (可选: session ID / user ID).
    pub audience: Option<String>,
    /// 内容提示 (可选 — 真实生成留给 A19 LLM 集成).
    pub body_hint: Option<String>,
}

impl ActionIntent {
    /// 构造最小 intent.
    pub fn new(action: ActionTarget) -> Self {
        Self {
            intent_id: Uuid::new_v4(),
            action,
            speaker: "assistant".to_string(),
            audience: None,
            body_hint: None,
        }
    }

    /// 链式构造 — 设置 speaker.
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = speaker.into();
        self
    }

    /// 链式构造 — 设置 audience.
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// 链式构造 — 设置 body_hint.
    pub fn with_body_hint(mut self, hint: impl Into<String>) -> Self {
        self.body_hint = Some(hint.into());
        self
    }
}

/// 结构化输出 — 任意通道的表达结果 (JSON 序列化友好)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredOutput {
    /// 通道.
    pub channel: ExpressionChannel,
    /// 发言方.
    pub speaker: String,
    /// 受众 (可选).
    pub audience: Option<String>,
    /// 关联 intent ID.
    pub intent_id: Uuid,
    /// 关联 target 描述.
    pub target_summary: String,
    /// 实际内容 (文字 / 多模态子字段 / 结构化 JSON).
    pub content: Value,
    /// 时间戳.
    pub timestamp: i64,
}

impl StructuredOutput {
    /// 取出文字负载 — 多模态时优先 text 字段, 无则 JSON 序列化整个 content.
    pub fn text_payload(&self) -> String {
        if let Some(s) = self.content.get("text").and_then(|v| v.as_str()) {
            return s.to_string();
        }
        match &self.content {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    /// 序列化为 JSON 字符串.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl ActionExpression for ActionEngine {
    fn express(&self, intent: &ActionIntent, channel: ExpressionChannel) -> StructuredOutput {
        let content = match channel {
            ExpressionChannel::Text => json!({
                "text": body_for(intent),
            }),
            ExpressionChannel::Voice => json!({
                "ssml": body_for(intent),
                "voice": "default",
            }),
            ExpressionChannel::MultiModal => json!({
                "text": body_for(intent),
                "image_hint": null,
                "video_hint": null,
            }),
            ExpressionChannel::Structured => json!({
                "intent_id": intent.intent_id.to_string(),
                "action": action_summary(&intent.action),
                "speaker": intent.speaker,
                "audience": intent.audience,
            }),
        };

        StructuredOutput {
            channel,
            speaker: intent.speaker.clone(),
            audience: intent.audience.clone(),
            intent_id: intent.intent_id,
            target_summary: action_summary(&intent.action),
            content,
            timestamp: now_epoch(),
        }
    }
}

impl ActionSilence for ActionEngine {
    fn should_silence(&self, intent: &ActionIntent) -> bool {
        !matches!(intent.action, ActionTarget::NormalAction(_))
            || intent
                .body_hint
                .as_deref()
                .map(|h| h.trim_start().starts_with("SILENT:"))
                .unwrap_or(false)
    }

    fn reason_for_silence(&self, intent: &ActionIntent) -> SilenceReason {
        match &intent.action {
            ActionTarget::ModifyL0HA
            | ActionTarget::ReorganizeOnion
            | ActionTarget::ModifyEvolutionL0 => SilenceReason::EthicalDoubt,
            ActionTarget::PretendClone
            | ActionTarget::PretendPerfect
            | ActionTarget::PretendUuid
            | ActionTarget::PretendUndo
            | ActionTarget::PretendSafe
            | ActionTarget::PretendSpecIsProof
            | ActionTarget::PretendCounterexampleIsBug
            | ActionTarget::PretendProverIsTruth
            | ActionTarget::PretendUnscientific => SilenceReason::NoConsent,
            ActionTarget::NormalAction(_) => {
                if intent
                    .body_hint
                    .as_deref()
                    .map(|h| h.trim_start().starts_with("SILENT:"))
                    .unwrap_or(false)
                {
                    SilenceReason::Deliberate
                } else {
                    SilenceReason::NotSilent
                }
            }
        }
    }
}

/// 用 body_hint 或 action 描述生成默认文字负载.
fn body_for(intent: &ActionIntent) -> String {
    if let Some(hint) = &intent.body_hint {
        return hint.clone();
    }
    format!("[{}] {}", intent.speaker, action_summary(&intent.action))
}

/// 把 ActionTarget 折叠成短字符串描述.
fn action_summary(action: &ActionTarget) -> String {
    match action {
        ActionTarget::NormalAction(s) => format!("normal_action:{}", s),
        ActionTarget::ModifyL0HA => "modify_l0_ha".to_string(),
        ActionTarget::ReorganizeOnion => "reorganize_onion".to_string(),
        ActionTarget::ModifyEvolutionL0 => "modify_evolution_l0".to_string(),
        ActionTarget::PretendClone => "pretend_clone".to_string(),
        ActionTarget::PretendPerfect => "pretend_perfect".to_string(),
        ActionTarget::PretendUuid => "pretend_uuid".to_string(),
        ActionTarget::PretendUndo => "pretend_undo".to_string(),
        ActionTarget::PretendSafe => "pretend_safe".to_string(),
        ActionTarget::PretendSpecIsProof => "pretend_spec_is_proof".to_string(),
        ActionTarget::PretendCounterexampleIsBug => "pretend_counterexample_is_bug".to_string(),
        ActionTarget::PretendProverIsTruth => "pretend_prover_is_truth".to_string(),
        ActionTarget::PretendUnscientific => "pretend_unscientific".to_string(),
    }
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_channel_name_is_stable() {
        assert_eq!(ExpressionChannel::Text.name(), "text");
        assert_eq!(ExpressionChannel::Voice.name(), "voice");
        assert_eq!(ExpressionChannel::MultiModal.name(), "multi_modal");
        assert_eq!(ExpressionChannel::Structured.name(), "structured");
    }

    #[test]
    fn has_text_for_text_and_multimodal() {
        assert!(ExpressionChannel::Text.has_text());
        assert!(!ExpressionChannel::Voice.has_text());
        assert!(ExpressionChannel::MultiModal.has_text());
        assert!(!ExpressionChannel::Structured.has_text());
    }

    #[test]
    fn action_intent_new_defaults_assistant_speaker() {
        let intent = ActionIntent::new(ActionTarget::NormalAction("noop".to_string()));
        assert_eq!(intent.speaker, "assistant");
        assert!(intent.audience.is_none());
        assert!(intent.body_hint.is_none());
    }

    #[test]
    fn action_intent_builder_chain() {
        let intent = ActionIntent::new(ActionTarget::NormalAction("noop".to_string()))
            .with_speaker("user_proxy")
            .with_audience("session_42")
            .with_body_hint("hello world");
        assert_eq!(intent.speaker, "user_proxy");
        assert_eq!(intent.audience.as_deref(), Some("session_42"));
        assert_eq!(intent.body_hint.as_deref(), Some("hello world"));
    }

    #[test]
    fn structured_output_text_payload_prefers_text_field() {
        let output = StructuredOutput {
            channel: ExpressionChannel::MultiModal,
            speaker: "assistant".to_string(),
            audience: None,
            intent_id: Uuid::new_v4(),
            target_summary: "normal_action:noop".to_string(),
            content: json!({ "text": "hello", "image_hint": null }),
            timestamp: 0,
        };
        assert_eq!(output.text_payload(), "hello");
    }

    #[test]
    fn structured_output_to_json_roundtrips() {
        let output = StructuredOutput {
            channel: ExpressionChannel::Text,
            speaker: "assistant".to_string(),
            audience: Some("session_1".to_string()),
            intent_id: Uuid::new_v4(),
            target_summary: "normal_action:greet".to_string(),
            content: json!({ "text": "hi" }),
            timestamp: 1,
        };
        let s = output.to_json().expect("serialize");
        // serde 默认 enum 序列化为 variant 名 ("Text" / "Voice" 等)
        assert!(s.contains("\"channel\":\"Text\""));
        assert!(s.contains("\"text\":\"hi\""));
    }
}
