//! Expression module: ActionIntent + ExpressionChannel + StructuredOutput.

use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::ActionTarget;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::silence::SilenceReason;
use crate::{ActionEngine, ActionExpression, ActionSilence};

/// Expression channel — 4 forms for projecting internal intent to the outside world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExpressionChannel {
    /// Pure text (CLI / logs).
    Text,
    /// Voice (TTS output, real hardware left for stage 7).
    Voice,
    /// Multi-modal (text + image + video).
    MultiModal,
    /// Structured (JSON / protobuf / internal RPC).
    Structured,
}

impl ExpressionChannel {
    /// Channel display name.
    pub const fn name(&self) -> &'static str {
        match self {
            ExpressionChannel::Text => "text",
            ExpressionChannel::Voice => "voice",
            ExpressionChannel::MultiModal => "multi_modal",
            ExpressionChannel::Structured => "structured",
        }
    }

    /// Whether includes text (Text + MultiModal).
    pub fn has_text(&self) -> bool {
        matches!(
            self,
            ExpressionChannel::Text | ExpressionChannel::MultiModal
        )
    }
}

/// Pending internal intent — produced by cognition, projected by action.
#[derive(Debug, Clone)]
pub struct ActionIntent {
    /// Unique intent ID.
    pub intent_id: Uuid,
    /// Associated action target.
    pub action: ActionTarget,
    /// Speaker (default "assistant").
    pub speaker: String,
    /// Audience (optional: session ID / user ID).
    pub audience: Option<String>,
    /// Body hint (optional — real generation left for A19 LLM integration).
    pub body_hint: Option<String>,
}

impl ActionIntent {
    /// Construct a minimal intent.
    pub fn new(action: ActionTarget) -> Self {
        Self {
            intent_id: Uuid::new_v4(),
            action,
            speaker: "assistant".to_string(),
            audience: None,
            body_hint: None,
        }
    }

    /// Chained constructor — set speaker.
    pub fn with_speaker(mut self, speaker: impl Into<String>) -> Self {
        self.speaker = speaker.into();
        self
    }

    /// Chained constructor — set audience.
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Chained constructor — set body_hint.
    pub fn with_body_hint(mut self, hint: impl Into<String>) -> Self {
        self.body_hint = Some(hint.into());
        self
    }
}

/// Structured output — expression result for any channel (JSON-friendly).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredOutput {
    /// Channel.
    pub channel: ExpressionChannel,
    /// Speaker.
    pub speaker: String,
    /// Audience (optional).
    pub audience: Option<String>,
    /// Associated intent ID.
    pub intent_id: Uuid,
    /// Associated target description.
    pub target_summary: String,
    /// Actual content (text / multi-modal subfields / structured JSON).
    pub content: Value,
    /// Timestamp.
    pub timestamp: i64,
}

impl StructuredOutput {
    /// Extract text payload — for multi-modal, prefer "text" field; else serialize entire content.
    pub fn text_payload(&self) -> String {
        if let Some(s) = self.content.get("text").and_then(|v| v.as_str()) {
            return s.to_string();
        }
        match &self.content {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    /// Serialize to JSON string.
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

/// Generate default text payload using body_hint or action description.
fn body_for(intent: &ActionIntent) -> String {
    if let Some(hint) = &intent.body_hint {
        return hint.clone();
    }
    format!("[{}] {}", intent.speaker, action_summary(&intent.action))
}

/// Fold an ActionTarget into a short string description.
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
    fn action_intent_ids_unique() {
        let a = ActionIntent::new(ActionTarget::NormalAction("x".into()));
        let b = ActionIntent::new(ActionTarget::NormalAction("x".into()));
        assert_ne!(a.intent_id, b.intent_id);
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
    fn structured_output_text_payload_string_content() {
        let output = StructuredOutput {
            channel: ExpressionChannel::Text,
            speaker: "assistant".to_string(),
            audience: None,
            intent_id: Uuid::new_v4(),
            target_summary: "x".to_string(),
            content: json!("plain text"),
            timestamp: 0,
        };
        assert_eq!(output.text_payload(), "plain text");
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
        // serde default enum serialization uses variant name ("Text" / "Voice" etc.)
        assert!(s.contains("\"channel\":\"Text\""));
        assert!(s.contains("\"text\":\"hi\""));
    }

    #[test]
    fn express_produces_structured_output_for_each_channel() {
        let engine = ActionEngine::new();
        let intent = ActionIntent::new(ActionTarget::NormalAction("hello".into()))
            .with_body_hint("hello body");
        for ch in [
            ExpressionChannel::Text,
            ExpressionChannel::Voice,
            ExpressionChannel::MultiModal,
            ExpressionChannel::Structured,
        ] {
            let out = engine.express(&intent, ch);
            assert_eq!(out.channel, ch);
            assert_eq!(out.intent_id, intent.intent_id);
            assert_eq!(out.speaker, intent.speaker);
        }
    }

    #[test]
    fn should_silence_for_normal_action_no_hint() {
        let engine = ActionEngine::new();
        let intent = ActionIntent::new(ActionTarget::NormalAction("x".into()));
        assert!(!engine.should_silence(&intent));
        assert_eq!(engine.reason_for_silence(&intent), SilenceReason::NotSilent);
    }

    #[test]
    fn should_silence_for_l0_target() {
        let engine = ActionEngine::new();
        let intent = ActionIntent::new(ActionTarget::ModifyL0HA);
        assert!(engine.should_silence(&intent));
        assert_eq!(
            engine.reason_for_silence(&intent),
            SilenceReason::EthicalDoubt
        );
    }

    #[test]
    fn should_silence_for_pretend_target() {
        let engine = ActionEngine::new();
        let intent = ActionIntent::new(ActionTarget::PretendClone);
        assert!(engine.should_silence(&intent));
        assert_eq!(engine.reason_for_silence(&intent), SilenceReason::NoConsent);
    }

    #[test]
    fn should_silence_for_silent_prefix() {
        let engine = ActionEngine::new();
        let intent = ActionIntent::new(ActionTarget::NormalAction("x".into()))
            .with_body_hint("SILENT:hold fire");
        assert!(engine.should_silence(&intent));
        assert_eq!(
            engine.reason_for_silence(&intent),
            SilenceReason::Deliberate
        );
    }

    #[test]
    fn silent_prefix_with_leading_whitespace() {
        let engine = ActionEngine::new();
        let intent = ActionIntent::new(ActionTarget::NormalAction("x".into()))
            .with_body_hint("  SILENT:hold");
        assert!(engine.should_silence(&intent));
        assert_eq!(
            engine.reason_for_silence(&intent),
            SilenceReason::Deliberate
        );
    }
}
