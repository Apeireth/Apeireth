//! Perception channels - `PerceptionChannel` trait + 5 channel implementations.
//!
//! **Architecture position**: stage-4 §3.1 perception channel abstraction (multi-modal: vision/audio/tactile/command/text).
//! **Responsibility**: collect same-modality inputs into `PerceptionEvent`, hand off to cognition.

use crate::input::{
    CommandInput, PerceptionInput, TactileInput, TextInput, VisionInput, VoiceInput,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

/// Channel kind - used for channel routing + reflection audit (PHL-04 no-pretend unobservable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelKind {
    /// Text (CLI / user messages).
    Text,
    /// Voice.
    Voice,
    /// Vision.
    Vision,
    /// Tactile / system heartbeat / error signals.
    Tactile,
    /// System command (slash commands).
    Command,
}

impl ChannelKind {
    /// String label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Voice => "voice",
            Self::Vision => "vision",
            Self::Tactile => "tactile",
            Self::Command => "command",
        }
    }
}

/// Unified perception event - the cognition input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionEvent {
    /// Event unique ID.
    pub event_id: Uuid,
    /// Channel kind.
    pub channel: ChannelKind,
    /// Source signal origin.
    pub source: crate::input::SignalSource,
    /// Event timestamp.
    pub timestamp: i64,
    /// Priority (inherited from PerceptionInput).
    pub priority: f64,
    /// Channel-specific payload (JSON-serialized string, avoids enumifying 5 inputs).
    pub payload: String,
    /// Free-form tags (for downstream cognition / reflection classification).
    pub tags: Vec<String>,
}

impl PerceptionEvent {
    /// Construct.
    pub fn new(
        channel: ChannelKind,
        source: crate::input::SignalSource,
        priority: f64,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            channel,
            source,
            timestamp: chrono::Utc::now().timestamp(),
            priority: priority.clamp(0.0, 1.0),
            payload: payload.into(),
            tags: Vec::new(),
        }
    }

    /// Append tag (chained).
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Perception channel trait - convert a class of `PerceptionInput` into `PerceptionEvent`.
pub trait PerceptionChannel: Send + Sync + Debug {
    /// The input type this channel accepts.
    type Input: PerceptionInput;

    /// Channel kind.
    fn kind(&self) -> ChannelKind;

    /// Channel name (for logs).
    fn name(&self) -> &str;

    /// Process a single input.
    fn process(&self, input: &Self::Input) -> PerceptionEvent;

    /// Batch processing.
    fn process_batch(&self, inputs: Vec<Self::Input>) -> Vec<PerceptionEvent> {
        inputs.iter().map(|i| self.process(i)).collect()
    }
}

// ============================================
// 5 concrete channel implementations
// ============================================

/// Text channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextChannel;

impl PerceptionChannel for TextChannel {
    type Input = TextInput;
    fn kind(&self) -> ChannelKind {
        ChannelKind::Text
    }
    fn name(&self) -> &str {
        "text"
    }
    fn process(&self, input: &Self::Input) -> PerceptionEvent {
        PerceptionEvent::new(
            self.kind(),
            input.source.clone(),
            input.priority,
            input.content.clone(),
        )
        .with_tag("text")
    }
}

/// Voice channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct VoiceChannel;

impl PerceptionChannel for VoiceChannel {
    type Input = VoiceInput;
    fn kind(&self) -> ChannelKind {
        ChannelKind::Voice
    }
    fn name(&self) -> &str {
        "voice"
    }
    fn process(&self, input: &Self::Input) -> PerceptionEvent {
        let payload = serde_json::json!({
            "transcript": input.transcript,
            "loudness": input.loudness,
        })
        .to_string();
        PerceptionEvent::new(self.kind(), input.source.clone(), input.priority, payload)
            .with_tag("voice")
    }
}

/// Vision channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct VisionChannel;

impl PerceptionChannel for VisionChannel {
    type Input = VisionInput;
    fn kind(&self) -> ChannelKind {
        ChannelKind::Vision
    }
    fn name(&self) -> &str {
        "vision"
    }
    fn process(&self, input: &Self::Input) -> PerceptionEvent {
        let payload = serde_json::json!({
            "width": input.width,
            "height": input.height,
            "ocr": input.ocr_text,
        })
        .to_string();
        PerceptionEvent::new(self.kind(), input.source.clone(), input.priority, payload)
            .with_tag("vision")
    }
}

/// Tactile channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct TactileChannel;

impl PerceptionChannel for TactileChannel {
    type Input = TactileInput;
    fn kind(&self) -> ChannelKind {
        ChannelKind::Tactile
    }
    fn name(&self) -> &str {
        "tactile"
    }
    fn process(&self, input: &Self::Input) -> PerceptionEvent {
        let payload = serde_json::json!({
            "pressure": input.pressure,
        })
        .to_string();
        PerceptionEvent::new(self.kind(), input.source.clone(), input.priority, payload)
            .with_tag("tactile")
    }
}

/// Command channel.
#[derive(Debug, Clone, Copy, Default)]
pub struct CommandChannel;

impl PerceptionChannel for CommandChannel {
    type Input = CommandInput;
    fn kind(&self) -> ChannelKind {
        ChannelKind::Command
    }
    fn name(&self) -> &str {
        "command"
    }
    fn process(&self, input: &Self::Input) -> PerceptionEvent {
        PerceptionEvent::new(
            self.kind(),
            input.source.clone(),
            input.priority,
            input.command.clone(),
        )
        .with_tag("command")
        .with_tag("user_initiated")
    }
}

/// Convenience function: batch process and return events per channel.
pub fn process_all<C: PerceptionChannel>(
    channel: &C,
    inputs: Vec<C::Input>,
) -> Vec<PerceptionEvent> {
    channel.process_batch(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{SignalSource, TextInput};

    #[test]
    fn channel_kind_labels_distinct() {
        assert_eq!(ChannelKind::Text.label(), "text");
        assert_eq!(ChannelKind::Voice.label(), "voice");
        assert_eq!(ChannelKind::Vision.label(), "vision");
        assert_eq!(ChannelKind::Tactile.label(), "tactile");
        assert_eq!(ChannelKind::Command.label(), "command");
    }

    #[test]
    fn text_channel_emits_event() {
        let ch = TextChannel;
        let inp = TextInput::new("hi", SignalSource::Cli);
        let ev = ch.process(&inp);
        assert_eq!(ev.channel, ChannelKind::Text);
        assert_eq!(ev.payload, "hi");
        assert!(ev.tags.contains(&"text".to_string()));
    }

    #[test]
    fn voice_channel_serializes_payload_as_json() {
        let ch = VoiceChannel;
        let v = VoiceInput::new("hello world", SignalSource::Http, 0.8);
        let ev = ch.process(&v);
        assert_eq!(ev.channel, ChannelKind::Voice);
        assert!(ev.payload.contains("hello world"));
        assert!(ev.payload.contains("0.8"));
    }

    #[test]
    fn vision_channel_includes_dimensions() {
        let ch = VisionChannel;
        let v = VisionInput::new(800, 600, SignalSource::PyBridge, Some("foo".into()));
        let ev = ch.process(&v);
        assert_eq!(ev.channel, ChannelKind::Vision);
        assert!(ev.payload.contains("800"));
        assert!(ev.payload.contains("foo"));
    }

    #[test]
    fn tactile_channel_includes_pressure() {
        let ch = TactileChannel;
        let t = TactileInput::new(-0.7, SignalSource::Internal);
        let ev = ch.process(&t);
        assert!(ev.payload.contains("-0.7"));
        assert!(ev.tags.contains(&"tactile".to_string()));
    }

    #[test]
    fn command_channel_tags_user_initiated() {
        let ch = CommandChannel;
        let c = CommandInput::new("/status", SignalSource::Cli);
        let ev = ch.process(&c);
        assert!(ev.tags.contains(&"user_initiated".to_string()));
        assert_eq!(ev.payload, "/status");
    }

    #[test]
    fn process_all_returns_one_per_input() {
        let ch = TextChannel;
        let inputs = vec![
            TextInput::new("a", SignalSource::Cli),
            TextInput::new("b", SignalSource::Cli),
        ];
        let events = process_all(&ch, inputs);
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn event_with_tag_appends() {
        let ev = PerceptionEvent::new(ChannelKind::Text, SignalSource::Cli, 0.5, "x")
            .with_tag("alpha")
            .with_tag("beta");
        assert_eq!(ev.tags, vec!["alpha".to_string(), "beta".to_string()]);
    }
}
