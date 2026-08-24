//! 感知通道 — `PerceptionChannel` trait + 5 种通道实现.
//!
//! **架构位置**: 阶段 4 §3.1 感知通道抽象 (多模态: 视觉/听觉/触觉/命令/文本).
//! **职责**: 把同模态的输入归一成 `PerceptionEvent`, 交给 cognition.

use crate::input::{
    CommandInput, PerceptionInput, TactileInput, TextInput, VisionInput, VoiceInput,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

/// 通道种类 — 用于通道路由 + 反思期审计 (PHL-04 不假装不可观测).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChannelKind {
    /// 文本 (CLI/用户消息).
    Text,
    /// 语音.
    Voice,
    /// 视觉.
    Vision,
    /// 触觉 / 系统心跳 / 错误信号.
    Tactile,
    /// 系统命令 (slash commands).
    Command,
}

impl ChannelKind {
    /// 字符串标签.
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

/// 统一感知事件 — cognition 的输入.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionEvent {
    /// 事件唯一 ID.
    pub event_id: Uuid,
    /// 通道种类.
    pub channel: ChannelKind,
    /// 来源信号源.
    pub source: crate::input::SignalSource,
    /// 事件时间戳.
    pub timestamp: i64,
    /// 优先级 (继承自 PerceptionInput).
    pub priority: f64,
    /// 通道特定负载 (JSON 序列化字符串, 避免把 5 种 input 枚举化).
    pub payload: String,
    /// 自由标签 (供下游 cognition/反思期分类).
    pub tags: Vec<String>,
}

impl PerceptionEvent {
    /// 构造.
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

    /// 追加标签 (链式).
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// 感知通道 trait — 把一类 `PerceptionInput` 转成 `PerceptionEvent`.
pub trait PerceptionChannel: Send + Sync + Debug {
    /// 此通道接受的输入类型.
    type Input: PerceptionInput;

    /// 通道种类.
    fn kind(&self) -> ChannelKind;

    /// 通道名 (供日志).
    fn name(&self) -> &str;

    /// 处理单条输入.
    fn process(&self, input: &Self::Input) -> PerceptionEvent;

    /// 批量处理.
    fn process_batch(&self, inputs: Vec<Self::Input>) -> Vec<PerceptionEvent> {
        inputs.iter().map(|i| self.process(i)).collect()
    }
}

// ============================================
// 5 种具体通道实现
// ============================================

/// 文本通道.
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

/// 语音通道.
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

/// 视觉通道.
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

/// 触觉通道.
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

/// 命令通道.
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

/// 便捷函数: 批量处理并按通道返回事件.
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
