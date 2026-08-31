//! Multimodal observation normalization recovered from canonical `apeireth-perception`.
//!
//! v2 already owns the frozen `PerceptionEvent` / `Attention` / modality traits in
//! `apeireth-plugin`. This module does **not** resurrect canonical `PerceptionChannel`
//! types or a second event schema. It ports the missing **algorithms**:
//!
//! - `SignalSource` provenance labels (cli / http / pybridge / mcp / internal / unknown)
//! - priority heuristics (voice loudness, vision pixel ratio, tactile |pressure|, command 0.9)
//! - event validation (non-empty payload, attention score in `[0, 1]`)
//! - threshold pipeline over the canonical `PerceptionEvent`
//!
//! Ownership stays in this crate. Runtime still converts text events through
//! `turn_request_from_perception`; this code never claims final-response ownership.

use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::SessionId;
use apeireth_plugin::perception::{
    Attention, PerceptionEvent, PerceptionModality, ThresholdAttention, TopKAttention,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Engine default attention threshold (balance of signal vs noise).
pub const DEFAULT_ATTENTION_THRESHOLD: f64 = 0.5;

/// Engine default Top-K (aligned with a single-batch processing cap).
pub const DEFAULT_TOP_K: usize = 5;

/// Full-HD reference used by the vision pixel-priority heuristic.
pub const VISION_REFERENCE_PIXELS: f64 = 1920.0 * 1080.0;

/// Default text priority when the caller does not override it.
pub const DEFAULT_TEXT_PRIORITY: f64 = 0.5;

/// Default command priority (explicit user intent).
pub const DEFAULT_COMMAND_PRIORITY: f64 = 0.9;

/// Provenance of an external signal. Distinct from [`PerceptionModality`]:
/// modality is *what* arrived, source is *where* it arrived from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalSource {
    /// Command line / TTY / slash command.
    Cli,
    /// HTTP / WebSocket ingress.
    Http,
    /// Python bridge.
    PyBridge,
    /// MCP client.
    Mcp,
    /// Internal (reflection / self-trigger / screen salience).
    Internal,
    /// Unknown — recorded honestly rather than coerced into a known source.
    Unknown,
}

impl SignalSource {
    /// Stable log/audit label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Http => "http",
            Self::PyBridge => "pybridge",
            Self::Mcp => "mcp",
            Self::Internal => "internal",
            Self::Unknown => "unknown",
        }
    }
}

/// Clamp a priority / score into `[0.0, 1.0]`.
pub fn clamp_priority(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

/// Voice priority equals clamped loudness (canonical heuristic; no acoustic model).
pub fn voice_priority(loudness: f64) -> f64 {
    clamp_priority(loudness)
}

/// Vision priority is pixel count over a 1920×1080 reference, clamped to `[0, 1]`.
pub fn vision_priority(width: u32, height: u32) -> f64 {
    let pixels = f64::from(width) * f64::from(height);
    (pixels / VISION_REFERENCE_PIXELS).clamp(0.0, 1.0)
}

/// Tactile priority is absolute pressure. Errors (`-1`) and successes (`+1`)
/// are equally worth noticing; idle (`0`) is not.
pub fn tactile_priority(pressure: f64) -> f64 {
    pressure.clamp(-1.0, 1.0).abs()
}

/// Current Unix time in milliseconds. Returns `0` if the system clock is
/// before the Unix epoch (fail-closed rather than panicking).
pub fn now_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

/// Default attention threshold (0.5).
pub fn default_attention_threshold() -> f64 {
    DEFAULT_ATTENTION_THRESHOLD
}

/// Default Top-K (5).
pub fn default_top_k() -> usize {
    DEFAULT_TOP_K
}

/// True when a JSON payload carries no observation content.
pub fn payload_is_empty(payload: &Value) -> bool {
    match payload {
        Value::Null => true,
        Value::String(text) => text.is_empty(),
        Value::Object(map) => map.is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

/// Validate canonical event fields recovered from canonical `validate_event`.
pub fn validate_event(event: &PerceptionEvent) -> Result<(), String> {
    if payload_is_empty(&event.payload) {
        return Err("PerceptionEvent.payload must not be empty".to_string());
    }
    if !(0.0..=1.0).contains(&event.attention_score) {
        return Err(format!(
            "PerceptionEvent.attention_score out of range: {}",
            event.attention_score
        ));
    }
    Ok(())
}

/// Append a tag (canonical `PerceptionEvent::with_tag`).
pub fn with_tag(mut event: PerceptionEvent, tag: impl Into<String>) -> PerceptionEvent {
    event.tags.push(tag.into());
    event
}

/// Filter events by attention threshold using the canonical `ThresholdAttention`.
///
/// The cutoff is **not** clamped here (matching canonical `pipeline`): a threshold
/// above 1.0 keeps nothing; a threshold below 0.0 keeps everything. Callers
/// that want a clamped cutoff should pass [`clamp_priority`].
pub fn pipeline_events(events: Vec<PerceptionEvent>, threshold: f64) -> Vec<PerceptionEvent> {
    ThresholdAttention { threshold }.select(events, usize::MAX)
}

/// Keep the highest-scoring `k` events using canonical `TopKAttention`.
pub fn top_k_events(events: Vec<PerceptionEvent>, k: usize) -> Vec<PerceptionEvent> {
    TopKAttention.select(events, k)
}

fn mint_event_id(prefix: &str) -> String {
    format!("{prefix}-{}", SessionId::new())
}

fn base_event(
    prefix: &str,
    modality: PerceptionModality,
    session_id: SessionId,
    timestamp_ms: i64,
    payload: Value,
    attention_score: f64,
    tags: Vec<String>,
) -> PerceptionEvent {
    PerceptionEvent {
        id: mint_event_id(prefix),
        source: modality,
        session_id,
        timestamp_ms,
        payload,
        attention_score: clamp_priority(attention_score),
        tags,
    }
}

/// Normalize a text observation into a canonical `PerceptionEvent`.
pub fn text_observation(
    session_id: SessionId,
    source: SignalSource,
    content: impl Into<String>,
    priority: f64,
    timestamp_ms: i64,
) -> PerceptionEvent {
    let content = content.into();
    base_event(
        "text",
        PerceptionModality::Text,
        session_id,
        timestamp_ms,
        json!({
            "text": content,
            "signal_source": source.label(),
        }),
        priority,
        vec!["text".into()],
    )
}

/// Normalize a voice transcript + loudness observation.
pub fn voice_observation(
    session_id: SessionId,
    source: SignalSource,
    transcript: impl Into<String>,
    loudness: f64,
    timestamp_ms: i64,
) -> PerceptionEvent {
    let loudness = clamp_priority(loudness);
    let transcript = transcript.into();
    base_event(
        "voice",
        PerceptionModality::Voice,
        session_id,
        timestamp_ms,
        json!({
            "transcript": transcript,
            "loudness": loudness,
            "signal_source": source.label(),
        }),
        voice_priority(loudness),
        vec!["voice".into()],
    )
}

/// Normalize a vision frame / OCR observation.
pub fn vision_observation(
    session_id: SessionId,
    source: SignalSource,
    width: u32,
    height: u32,
    ocr_text: Option<String>,
    timestamp_ms: i64,
) -> PerceptionEvent {
    base_event(
        "vision",
        PerceptionModality::Vision,
        session_id,
        timestamp_ms,
        json!({
            "width": width,
            "height": height,
            "ocr": ocr_text,
            "signal_source": source.label(),
        }),
        vision_priority(width, height),
        vec!["vision".into()],
    )
}

/// Normalize a tactile / heartbeat / error-signal observation.
pub fn tactile_observation(
    session_id: SessionId,
    source: SignalSource,
    pressure: f64,
    timestamp_ms: i64,
) -> PerceptionEvent {
    let pressure = pressure.clamp(-1.0, 1.0);
    base_event(
        "tactile",
        PerceptionModality::Tactile,
        session_id,
        timestamp_ms,
        json!({
            "pressure": pressure,
            "signal_source": source.label(),
        }),
        tactile_priority(pressure),
        vec!["tactile".into()],
    )
}

/// Normalize an explicit user/system command observation.
pub fn command_observation(
    session_id: SessionId,
    source: SignalSource,
    command: impl Into<String>,
    timestamp_ms: i64,
) -> PerceptionEvent {
    let command = command.into();
    base_event(
        "command",
        PerceptionModality::Command,
        session_id,
        timestamp_ms,
        json!({
            "command": command,
            "text": command,
            "signal_source": source.label(),
        }),
        DEFAULT_COMMAND_PRIORITY,
        vec!["command".into(), "user_initiated".into()],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid() -> SessionId {
        SessionId::new()
    }

    #[test]
    fn default_attention_threshold_is_half() {
        assert!((default_attention_threshold() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn default_top_k_is_five() {
        assert_eq!(default_top_k(), 5);
    }

    #[test]
    fn now_timestamp_ms_is_recent() {
        let timestamp = now_timestamp_ms();
        // After 2024-01-01 and before 2100-01-01, in milliseconds.
        assert!(timestamp > 1_704_067_200_000);
        assert!(timestamp < 4_102_444_800_000);
    }

    #[test]
    fn signal_source_labels_are_stable() {
        assert_eq!(SignalSource::Cli.label(), "cli");
        assert_eq!(SignalSource::Http.label(), "http");
        assert_eq!(SignalSource::PyBridge.label(), "pybridge");
        assert_eq!(SignalSource::Mcp.label(), "mcp");
        assert_eq!(SignalSource::Internal.label(), "internal");
        assert_eq!(SignalSource::Unknown.label(), "unknown");
    }

    #[test]
    fn text_priority_clamps_to_range() {
        let high = text_observation(sid(), SignalSource::Cli, "hi", 5.0, 1);
        assert!(high.attention_score <= 1.0);
        let low = text_observation(sid(), SignalSource::Cli, "hi", -2.0, 1);
        assert!(low.attention_score >= 0.0);
        let mid = text_observation(sid(), SignalSource::Cli, "hi", DEFAULT_TEXT_PRIORITY, 1);
        assert!((mid.attention_score - 0.5).abs() < 1e-9);
        assert_eq!(mid.source, PerceptionModality::Text);
        assert_eq!(mid.payload["text"], "hi");
        assert_eq!(mid.payload["signal_source"], "cli");
    }

    #[test]
    fn voice_priority_equals_loudness_clamped() {
        let voice = voice_observation(sid(), SignalSource::Http, "hello", 0.7, 1);
        assert!((voice.attention_score - 0.7).abs() < 1e-9);
        assert_eq!(voice.payload["transcript"], "hello");
        assert_eq!(voice.payload["loudness"], 0.7);
        let loud = voice_observation(sid(), SignalSource::Mcp, "loud", 2.0, 1);
        assert!((loud.attention_score - 1.0).abs() < 1e-9);
        assert_eq!(loud.source, PerceptionModality::Voice);
    }

    #[test]
    fn vision_priority_proportional_to_pixels() {
        let full = vision_observation(sid(), SignalSource::Internal, 1920, 1080, None, 1);
        assert!((full.attention_score - 1.0).abs() < 1e-6);
        let small = vision_observation(
            sid(),
            SignalSource::PyBridge,
            640,
            480,
            Some("hello".into()),
            1,
        );
        assert!(small.attention_score < 1.0);
        assert_eq!(small.payload["ocr"], "hello");
        assert_eq!(small.payload["width"], 640);
        assert_eq!(small.source, PerceptionModality::Vision);
    }

    #[test]
    fn tactile_priority_uses_absolute_pressure() {
        let error = tactile_observation(sid(), SignalSource::Internal, -0.8, 1);
        assert!((error.attention_score - 0.8).abs() < 1e-9);
        assert_eq!(error.payload["pressure"], -0.8);
        let ok = tactile_observation(sid(), SignalSource::Cli, 0.3, 1);
        assert!((ok.attention_score - 0.3).abs() < 1e-9);
        assert_eq!(ok.source, PerceptionModality::Tactile);
    }

    #[test]
    fn command_input_default_high_priority_and_user_initiated_tag() {
        let command = command_observation(sid(), SignalSource::Cli, "/status", 1);
        assert!((command.attention_score - 0.9).abs() < 1e-9);
        assert!(command.tags.contains(&"user_initiated".to_string()));
        assert!(command.tags.contains(&"command".to_string()));
        assert_eq!(command.payload["command"], "/status");
        // Keep a `text` field so `turn_request_from_perception` can consume commands
        // without a second request path.
        assert_eq!(command.payload["text"], "/status");
        assert_eq!(command.source, PerceptionModality::Command);
    }

    #[test]
    fn validate_event_accepts_good_and_rejects_empty_or_out_of_range() {
        let good = text_observation(sid(), SignalSource::Cli, "x", 0.5, 1);
        assert!(validate_event(&good).is_ok());

        let mut empty = good.clone();
        empty.payload = json!({});
        assert!(validate_event(&empty).is_err());

        let mut bad_score = good;
        bad_score.attention_score = 1.5;
        assert!(validate_event(&bad_score).is_err());
    }

    #[test]
    fn pipeline_filters_by_threshold() {
        let session = sid();
        let events = vec![
            text_observation(session, SignalSource::Cli, "a", 0.9, 1),
            text_observation(session, SignalSource::Cli, "b", 0.1, 1),
            text_observation(session, SignalSource::Cli, "c", 0.7, 1),
        ];
        let kept = pipeline_events(events, 0.5);
        assert_eq!(kept.len(), 2);
        assert!(kept.iter().all(|event| event.attention_score >= 0.5));
    }

    #[test]
    fn top_k_keeps_highest_priorities_and_zero_k_is_empty() {
        let session = sid();
        let events = vec![
            text_observation(session, SignalSource::Cli, "a", 0.1, 1),
            text_observation(session, SignalSource::Cli, "b", 0.9, 1),
            text_observation(session, SignalSource::Cli, "c", 0.3, 1),
            text_observation(session, SignalSource::Cli, "d", 0.8, 1),
            text_observation(session, SignalSource::Cli, "e", 0.5, 1),
        ];
        let top2 = top_k_events(events.clone(), 2);
        assert_eq!(top2.len(), 2);
        assert!((top2[0].attention_score - 0.9).abs() < 1e-9);
        assert!((top2[1].attention_score - 0.8).abs() < 1e-9);
        assert!(top_k_events(events.clone(), 0).is_empty());
        assert_eq!(top_k_events(events, 10).len(), 5);
    }

    #[test]
    fn with_tag_appends() {
        let event = with_tag(
            with_tag(
                text_observation(sid(), SignalSource::Cli, "x", 0.5, 1),
                "alpha",
            ),
            "beta",
        );
        assert!(event.tags.contains(&"alpha".to_string()));
        assert!(event.tags.contains(&"beta".to_string()));
    }

    #[test]
    fn pipeline_threshold_above_one_keeps_nothing() {
        let session = sid();
        let events = vec![text_observation(session, SignalSource::Cli, "a", 1.0, 1)];
        let kept = pipeline_events(events, 2.0);
        assert!(kept.is_empty());
        let all = pipeline_events(
            vec![text_observation(session, SignalSource::Cli, "a", 0.0, 1)],
            -1.0,
        );
        assert_eq!(all.len(), 1);
    }
}
