//! 感知输入 — `PerceptionInput` trait + 多模态实现.
//!
//! **架构位置**: 阶段 4 §3.1 感知层 trait (扩展官方 `Signal` sketch).
//! **职责**: 描述一条"从外部世界到来的信号", 附时间戳/来源/优先级.
//!
//! ponytail: trait 字段只保留 3 个核心, 不引入 modality-specific payload 字段,
//! 通道特定数据 (文本/字节流/向量) 由调用方按需嵌入 `PerceptionEvent::payload`.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

/// 信号来源 — 通道不必关心, 但审计/反思需要 (D2 §5 + 阶段 4 v6 守门嵌套).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignalSource {
    /// 命令行 (CLI / TTY / slash command).
    Cli,
    /// HTTP / WebSocket (L3/L4 总线).
    Http,
    /// Python 桥接 (PyO3 — R11 1100+ v*.py 兼容).
    PyBridge,
    /// MCP 客户端 (外部工具协议).
    Mcp,
    /// 内部 (反思期 / Cognitive-Dream 状态机自触发).
    Internal,
    /// 未知 — 必须诚实登记, 不能假装已知.
    Unknown,
}

impl SignalSource {
    /// 标签 (供日志/审计).
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

/// 感知输入 trait — 一条外部信号.
pub trait PerceptionInput: Send + Sync + 'static + Debug + Clone {
    /// 当前 Unix 时间戳 (秒).
    fn timestamp(&self) -> i64;
    /// 信号来源.
    fn source(&self) -> SignalSource;
    /// 注意力优先级 (0.0 - 1.0). 越高越值得被处理.
    fn priority(&self) -> f64;
    /// 输入唯一 ID.
    fn id(&self) -> Uuid;
}

/// 文本输入 — 最常见 (CLI 命令 / 用户消息 / 日志).
#[derive(Debug, Clone)]
pub struct TextInput {
    /// 唯一 ID.
    pub id: Uuid,
    /// 时间戳.
    pub timestamp: i64,
    /// 来源.
    pub source: SignalSource,
    /// 文本内容.
    pub content: String,
    /// 优先级 (默认 0.5).
    pub priority: f64,
}

impl TextInput {
    /// 构造 (时间戳默认现在).
    pub fn new(content: impl Into<String>, source: SignalSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
            source,
            content: content.into(),
            priority: 0.5,
        }
    }

    /// 显式设置优先级 (链式).
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

/// 语音输入 — 来自 ASR / 语音流.
#[derive(Debug, Clone)]
pub struct VoiceInput {
    pub id: Uuid,
    pub timestamp: i64,
    pub source: SignalSource,
    pub transcript: String,
    /// 音量归一化 (0.0 - 1.0). 用于 priority 计算.
    pub loudness: f64,
    pub priority: f64,
}

impl VoiceInput {
    /// 构造 (priority 由 loudness 推导: 越大越值得注意).
    pub fn new(transcript: impl Into<String>, source: SignalSource, loudness: f64) -> Self {
        let loudness = loudness.clamp(0.0, 1.0);
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
            source,
            transcript: transcript.into(),
            loudness,
            // ponytail: 简单启发式 — 音量越大优先级越高. 阶段 5 可替换为声学模型.
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

/// 视觉输入 — 来自屏幕 / 摄像头 / OCR (Stage 5 接入).
#[derive(Debug, Clone)]
pub struct VisionInput {
    pub id: Uuid,
    pub timestamp: i64,
    pub source: SignalSource,
    /// 图像宽度 (px).
    pub width: u32,
    /// 图像高度 (px).
    pub height: u32,
    /// OCR 文本 (可选).
    pub ocr_text: Option<String>,
    pub priority: f64,
}

impl VisionInput {
    /// 构造.
    pub fn new(width: u32, height: u32, source: SignalSource, ocr: Option<String>) -> Self {
        // ponytail: 默认优先级基于分辨率启发式 (越大越高). 阶段 5 可替换为显著性模型.
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

/// 触觉输入 — 来自终端状态 / 错误信号 / 心跳.
#[derive(Debug, Clone)]
pub struct TactileInput {
    pub id: Uuid,
    pub timestamp: i64,
    pub source: SignalSource,
    /// 触感强度 (-1.0 error / 0.0 idle / +1.0 success).
    pub pressure: f64,
    pub priority: f64,
}

impl TactileInput {
    /// 构造.
    pub fn new(pressure: f64, source: SignalSource) -> Self {
        let pressure = pressure.clamp(-1.0, 1.0);
        // ponytail: 压力绝对值越大优先级越高 (错误和成功都值得关注).
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

/// 命令输入 — 来自 slash command / 系统内部信号.
#[derive(Debug, Clone)]
pub struct CommandInput {
    pub id: Uuid,
    pub timestamp: i64,
    pub source: SignalSource,
    pub command: String,
    pub priority: f64,
}

impl CommandInput {
    /// 构造.
    pub fn new(command: impl Into<String>, source: SignalSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now().timestamp(),
            source,
            command: command.into(),
            // ponytail: 命令默认高优先级 (用户显式触发). 阶段 5 可基于命令分级.
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
