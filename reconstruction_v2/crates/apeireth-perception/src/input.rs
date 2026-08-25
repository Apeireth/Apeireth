//! Perception input - `PerceptionInput` trait + multi-modal implementations.
//!
//! **Architecture position**: stage-4 §3.1 perception layer trait (extension of official `Signal` sketch).
//! **Responsibility**: describe a "signal arriving from the outside world" with timestamp/source/priority.
//!
//! Ponytail: trait keeps only 3 core fields. Modality-specific payloads are stored in
//! the channel-specific structs (TextInput, VoiceInput, ...).

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

/// Signal source - channels need not care, but audit / reflection require it (D2 §5 + stage-4 v6 gate nesting).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalSource {
    /// Command line (CLI / TTY / slash command).
    Cli,
    /// HTTP / WebSocket (L3/L4 bus).
    Http,
    /// Python bridge (PyO3 - R11 1100+ v*.py compatibility).
    PyBridge,
    /// MCP client (external tool protocol).
    Mcp,
    /// Internal (reflection / Cognitive-Dream state machine self-trigger).
    Internal,
    /// Unknown - must be honestly registered, never pretended.
    Unknown,
}

impl SignalSource {
    /// Label (for log/audit).
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

/// Perception input trait - one external signal.
pub trait PerceptionInput: Send + Sync + 'static + Debug + Clone {
    /// Current Unix timestamp (seconds).
    fn timestamp(&self) -> i64;
    /// Signal source.
    fn source(&self) -> SignalSource;
    /// Attention priority (0.0 - 1.0). Higher means more worth processing.
    fn priority(&self) -> f64;
    /// Input unique ID.
    fn id(&self) -> Uuid;
}

/// Text input - most common (CLI commands / user messages / logs).
#[derive(Debug, Clone)]
pub struct TextInput {
    /// Unique ID.
    pub id: Uuid,
    /// Timestamp.
    pub timestamp: i64,
    /// Source.
    pub source: SignalSource,
    /// Text content.
    pub content: String,
    /// Priority (default 0.5).
    pub priority: f64,
}

impl TextInput {
    /// Construct (timestamp default now).
    pub fn new(content: impl Into<String>, source: SignalSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
            source,
            content: content.into(),
            priority: 0.5,
        }
    }

    /// Set priority explicitly (chained).
    pub fn with_priority(mut self, p: f64) -> Self {
        self.priority = p.clamp(0.0, 1.0);
        self
    }
}

impl PerceptionInput for TextInput {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn source(&self) -> SignalSource {
        self.source.clone()
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn id(&self) -> Uuid {
        self.id
    }
}

/// Voice input - from ASR / voice stream.
#[derive(Debug, Clone)]
pub struct VoiceInput {
    pub id: Uuid,
    pub timestamp: i64,
    pub source: SignalSource,
    pub transcript: String,
    /// Normalized loudness (0.0 - 1.0). Used in priority computation.
    pub loudness: f64,
    pub priority: f64,
}

impl VoiceInput {
    /// Construct (priority derived from loudness: louder = more worth attention).
    pub fn new(transcript: impl Into<String>, source: SignalSource, loudness: f64) -> Self {
        let loudness = loudness.clamp(0.0, 1.0);
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
            source,
            transcript: transcript.into(),
            loudness,
            // ponytail: simple heuristic - louder = higher priority. Stage 5 can swap in acoustic model.
            priority: loudness,
        }
    }
}

impl PerceptionInput for VoiceInput {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn source(&self) -> SignalSource {
        self.source.clone()
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn id(&self) -> Uuid {
        self.id
    }
}

/// Vision input - from screen / camera / OCR (stage 5 adoption).
#[derive(Debug, Clone)]
pub struct VisionInput {
    pub id: Uuid,
    pub timestamp: i64,
    pub source: SignalSource,
    /// Image width (px).
    pub width: u32,
    /// Image height (px).
    pub height: u32,
    /// OCR text (optional).
    pub ocr_text: Option<String>,
    pub priority: f64,
}

impl VisionInput {
    /// Construct.
    pub fn new(width: u32, height: u32, source: SignalSource, ocr: Option<String>) -> Self {
        // ponytail: default priority heuristic based on resolution (bigger = more salient).
        // Stage 5 can swap in saliency model.
        let pixels = f64::from(width) * f64::from(height);
        let priority = (pixels / (1920.0 * 1080.0)).clamp(0.0, 1.0);
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
            source,
            width,
            height,
            ocr_text: ocr,
            priority,
        }
    }
}

impl PerceptionInput for VisionInput {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn source(&self) -> SignalSource {
        self.source.clone()
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn id(&self) -> Uuid {
        self.id
    }
}

/// Tactile input - from terminal state / error signals / heartbeat.
#[derive(Debug, Clone)]
pub struct TactileInput {
    pub id: Uuid,
    pub timestamp: i64,
    pub source: SignalSource,
    /// Pressure intensity (-1.0 error / 0.0 idle / +1.0 success).
    pub pressure: f64,
    pub priority: f64,
}

impl TactileInput {
    /// Construct.
    pub fn new(pressure: f64, source: SignalSource) -> Self {
        let pressure = pressure.clamp(-1.0, 1.0);
        // ponytail: absolute pressure = priority (errors and successes both worth attention).
        let priority = pressure.abs();
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
            source,
            pressure,
            priority,
        }
    }
}

impl PerceptionInput for TactileInput {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn source(&self) -> SignalSource {
        self.source.clone()
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn id(&self) -> Uuid {
        self.id
    }
}

/// Command input - from slash command / system internal signal.
#[derive(Debug, Clone)]
pub struct CommandInput {
    pub id: Uuid,
    pub timestamp: i64,
    pub source: SignalSource,
    pub command: String,
    pub priority: f64,
}

impl CommandInput {
    /// Construct.
    pub fn new(command: impl Into<String>, source: SignalSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
            source,
            command: command.into(),
            // ponytail: commands default to high priority (user-initiated). Stage 5 can tier.
            priority: 0.9,
        }
    }
}

impl PerceptionInput for CommandInput {
    fn timestamp(&self) -> i64 {
        self.timestamp
    }
    fn source(&self) -> SignalSource {
        self.source.clone()
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn id(&self) -> Uuid {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_input_priority_clamps_to_range() {
        let inp = TextInput::new("hi", SignalSource::Cli).with_priority(5.0);
        assert!(inp.priority <= 1.0);
        let inp = TextInput::new("hi", SignalSource::Cli).with_priority(-2.0);
        assert!(inp.priority >= 0.0);
    }

    #[test]
    fn voice_priority_equals_loudness_clamped() {
        let v = VoiceInput::new("hello", SignalSource::Http, 0.7);
        assert_eq!(v.priority, 0.7);
        let v = VoiceInput::new("loud", SignalSource::Mcp, 2.0);
        assert_eq!(v.priority, 1.0);
    }

    #[test]
    fn vision_priority_proportional_to_pixels() {
        let v = VisionInput::new(1920, 1080, SignalSource::Internal, None);
        assert!((v.priority - 1.0).abs() < 1e-6);
        let v = VisionInput::new(640, 480, SignalSource::PyBridge, Some("hello".into()));
        assert!(v.priority < 1.0);
        assert_eq!(v.ocr_text.as_deref(), Some("hello"));
    }

    #[test]
    fn tactile_priority_uses_absolute_pressure() {
        let t = TactileInput::new(-0.8, SignalSource::Internal);
        assert_eq!(t.priority, 0.8);
        let t = TactileInput::new(0.3, SignalSource::Cli);
        assert_eq!(t.priority, 0.3);
    }

    #[test]
    fn command_input_default_high_priority() {
        let c = CommandInput::new("/status", SignalSource::Cli);
        assert_eq!(c.priority, 0.9);
    }

    #[test]
    fn signal_source_labels_are_stable() {
        assert_eq!(SignalSource::Cli.label(), "cli");
        assert_eq!(SignalSource::PyBridge.label(), "pybridge");
        assert_eq!(SignalSource::Unknown.label(), "unknown");
    }
}
