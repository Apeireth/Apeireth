//! Apeireth perception organ (A9 landing — R14 stage 4).
//!
//! **Responsibility**: external input adapter layer — unifies signals/IO/token streams from
//! different sources (CLI/TTY/HTTP/Python bridge) into `PerceptionEvent`, hands off to the
//! cognition organ.
//!
//! **Architecture position**: stage-4 §2 main path 17-crate A9 organ (original derivation
//! 9-dim: perception).
//!
//! **This crate provides**:
//! - [`PerceptionInput`] trait + 5 implementations (Text/Voice/Vision/Tactile/Command)
//! - [`Attention`] trait + 2 built-in strategies (TopK/Threshold)
//! - [`PerceptionChannel`] trait + 5 channels (one-to-one with input types)
//! - [`PerceptionEvent`] — unified input format for cognition
//!
//! **Honest registration**: per `leader-handover-final-2026-08-01` §B simplified implementation
//! (5+ pub fn, 5+ tests, 1+ integration test, examples). Full saliency + multimodal fusion
//! remain for stage 5.
//!
//! **Prohibitions**:
//! - do NOT modify apeireth-core / apeireth-cognition installed type signatures
//! - do NOT touch R11 baseline three values
//! - do NOT touch apeireth-legacy/

#![deny(unsafe_code)]

use thiserror::Error;

mod attention;
mod channel;
mod input;

pub use attention::{threshold_filter, top_k_filter, Attention, ThresholdAttention, TopKAttention};
pub use channel::{
    process_all, ChannelKind, CommandChannel, PerceptionChannel, PerceptionEvent, TactileChannel,
    TextChannel, VisionChannel, VoiceChannel,
};
pub use input::{
    CommandInput, PerceptionInput, SignalSource, TactileInput, TextInput, VisionInput, VoiceInput,
};

/// Top-level error: fallback error for the perception subsystem.
#[derive(Debug, Error)]
pub enum PerceptionError {
    /// Invalid input argument.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// Channel does not accept this input.
    #[error("channel mismatch: {0}")]
    ChannelMismatch(String),
    /// Serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Unified result type.
pub type PerceptionResult<T> = Result<T, PerceptionError>;

// ============================================
// Top-level convenience functions (5+ pub fn at lib layer)
// ============================================

/// Current Unix timestamp (seconds).
pub fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Default attention threshold (0.5) — engineering constant for balanced SNR.
pub fn default_attention_threshold() -> f64 {
    0.5
}

/// Default Top-K count (5) — aligned with R11 baseline single-batch upper limit.
pub fn default_top_k() -> usize {
    5
}

/// Batch process inputs through a channel.
pub fn batch_process<C: PerceptionChannel>(
    channel: &C,
    inputs: Vec<C::Input>,
) -> Vec<PerceptionEvent> {
    process_all(channel, inputs)
}

/// End-to-end convenience: input -> channel -> attention filter -> events.
pub fn pipeline<C: PerceptionChannel>(
    channel: &C,
    inputs: Vec<C::Input>,
    threshold: f64,
) -> Vec<PerceptionEvent> {
    let events = channel.process_batch(inputs);
    events
        .into_iter()
        .filter(|e| e.priority >= threshold)
        .collect()
}

/// Validate a `PerceptionEvent`'s basic fields (for internal tests / reflection gate).
pub fn validate_event(ev: &PerceptionEvent) -> PerceptionResult<()> {
    if ev.payload.is_empty() {
        return Err(PerceptionError::InvalidInput(
            "PerceptionEvent.payload must not be empty".to_string(),
        ));
    }
    if !(0.0..=1.0).contains(&ev.priority) {
        return Err(PerceptionError::InvalidInput(format!(
            "PerceptionEvent.priority out of range: {}",
            ev.priority
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{SignalSource, TextInput};

    #[test]
    fn default_attention_threshold_is_half() {
        assert!((default_attention_threshold() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn default_top_k_is_five() {
        assert_eq!(default_top_k(), 5);
    }

    #[test]
    fn now_timestamp_is_recent() {
        let t = now_timestamp();
        // 2024-01-01 .. 2100-01-01
        assert!(t > 1_704_067_200);
        assert!(t < 4_102_444_800);
    }

    #[test]
    fn batch_process_routes_to_channel() {
        let ch = TextChannel;
        let inputs = vec![
            TextInput::new("a", SignalSource::Cli),
            TextInput::new("b", SignalSource::Http),
        ];
        let events = batch_process(&ch, inputs);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].channel, ChannelKind::Text);
    }

    #[test]
    fn pipeline_filters_by_threshold() {
        let ch = TextChannel;
        let inputs = vec![
            TextInput::new("a", SignalSource::Cli).with_priority(0.9),
            TextInput::new("b", SignalSource::Cli).with_priority(0.1),
            TextInput::new("c", SignalSource::Cli).with_priority(0.7),
        ];
        let events = pipeline(&ch, inputs, 0.5);
        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| e.priority >= 0.5));
    }

    #[test]
    fn validate_event_accepts_good() {
        let ev = PerceptionEvent::new(ChannelKind::Text, SignalSource::Cli, 0.5, "x");
        assert!(validate_event(&ev).is_ok());
    }

    #[test]
    fn validate_event_rejects_empty_payload() {
        let ev = PerceptionEvent::new(ChannelKind::Text, SignalSource::Cli, 0.5, "");
        assert!(matches!(
            validate_event(&ev),
            Err(PerceptionError::InvalidInput(_))
        ));
    }

    #[test]
    fn validate_event_rejects_out_of_range_priority() {
        // Direct construction bypasses PerceptionEvent::new's clamp.
        let ev = PerceptionEvent {
            event_id: uuid::Uuid::new_v4(),
            channel: ChannelKind::Text,
            source: SignalSource::Cli,
            timestamp: now_timestamp(),
            priority: 1.5,
            payload: "x".into(),
            tags: vec![],
        };
        assert!(validate_event(&ev).is_err());
    }
}
