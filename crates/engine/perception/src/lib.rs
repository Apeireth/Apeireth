//! P-arch (2026-08-27): A3 Perception trait 骨架 (5 modality).
//!
//! 借鉴 v1 `apeireth-experience`... 不对, 借鉴 v1 `apeireth-perception`（5 种输入 +
//! Attention 策略 + PerceptionChannel + PerceptionEvent 统一格式）。**v2 形态**：
//!
//! - `PerceptionInput` trait（5 modality: Text / Voice / Vision / Tactile / Command）
//! - `PerceptionEvent` (统一输出格式) 送到 `apeireth-runtime`
//! - `Attention` trait (TopK / Threshold 策略)
//! - `PerceptionChannel` trait（PerceptionInput -> PerceptionEvent 适配器）
//!
//! **0 装 PASS**：
//! - v2.0 alpha **只**实现 Text modality（生产用）；Voice / Vision / Tactile / Command
//!   是 forward-declared trait method（**返回 NotImplemented**），明确"v2.0 做不到"
//! - 现实路径: v1 companion/desktop 的 voice_session / screen_perception 走
//!   `legacy/donor/apeireth-perception` (workspace exclude), v2 alpha 不移植
//! - 完整 v2.1 实现: 跟 scene-d 例 1 (主人偏好) / 例 2 (自我评估) 同步做
//!
//! **架构原则**：
//! - PerceptionInput 是纯 trait (零 backend 依赖), modality 0装 通过
//!   `unimplemented!()` 守门, 不假装"已实现 voice/vision"
//! - PerceptionEvent **不**包含敏感内容 (per v1 attention policy: top-K 注入)
//! - 与 runtime 集成: runtime 的 agent loop 在每个 turn 调 attention.select(events)
//!   拿 top-K 注入 transcript, **不**让模型决定"我要看什么"
//!
//! 详见 `v2-unabsorbed-features.md` §A3 + `ROADMAP.md` §4 P4.

use serde::{Deserialize, Serialize};

use apeireth_core::kernel::SessionId;

// ============================================
// 统一输入格式 (perception -> runtime)
// ============================================

/// 感知事件 (per v1 `PerceptionEvent` 简化版, modality-agnostic).
///
/// runtime 在每个 turn 调 attention 选 top-K, 注入 transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionEvent {
    /// 唯一 id
    pub id: String,
    /// 来源 modality
    pub source: PerceptionModality,
    /// 所属 session
    pub session_id: SessionId,
    /// 事件时间戳 (epoch millis)
    pub timestamp_ms: i64,
    /// 事件内容 (modality-specific 序列化)
    pub payload: serde_json::Value,
    /// 注意力评分 (0.0-1.0, 由 source channel 计算)
    pub attention_score: f64,
    /// 标签 (用于 attention 过滤)
    pub tags: Vec<String>,
}

/// 5 种输入 modality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PerceptionModality {
    /// 文本 (v2.0 alpha 唯一真实现)
    Text,
    /// 语音 (v2.1 路线, v2.0 alpha 0 装)
    Voice,
    /// 视觉 (v2.1 路线, v2.0 alpha 0 装)
    Vision,
    /// 触觉 (v2.1 路线, v2.0 alpha 0 装)
    Tactile,
    /// 命令 (用户指令, v2.0 alpha 同 Text 路径)
    Command,
}

// ============================================
// Input trait (5 modality impls)
// ============================================

/// 输入 trait: modality-agnostic 入口
pub trait PerceptionInput: Send + Sync {
    /// modality
    fn modality(&self) -> PerceptionModality;

    /// 读下一个事件 (blocking 或 async, 由 impl 决定)
    ///
    /// **0 装 PASS (v2.0 alpha)**: Voice / Vision / Tactile 应当返回
    /// `Err(PerceptionError::NotImplemented)`, 不假装能 stream.
    fn next_event(&self) -> Result<Option<PerceptionEvent>, PerceptionError>;

    /// 健康检查 (启动时调)
    fn ping(&self) -> Result<(), PerceptionError> {
        Ok(())
    }
}

// ============================================
// Channel trait (PerceptionInput -> PerceptionEvent stream)
// ============================================

/// 感知通道 (per v1 PerceptionChannel): 把 modality-specific input 转 unified event stream.
pub trait PerceptionChannel: Send + Sync {
    /// 通道名 (用于配置 + 监控)
    fn name(&self) -> &'static str;

    /// 所属 modality
    fn modality(&self) -> PerceptionModality;

    /// 启动 channel (启动后开始 streaming)
    fn start(&mut self) -> Result<(), PerceptionError>;

    /// 停止 channel
    fn stop(&mut self) -> Result<(), PerceptionError>;

    /// 拉下一批 event (用于 sync runtime, 异步 impl 可自己 buffer)
    fn poll_events(&mut self, max: usize) -> Result<Vec<PerceptionEvent>, PerceptionError>;
}

// ============================================
// Attention trait (TopK / Threshold 策略)
// ============================================

/// Attention trait: 从 event stream 选 top-K 注入 runtime
pub trait Attention: Send + Sync {
    /// attention 策略名
    fn name(&self) -> &'static str;

    /// 从 events 选 top-K
    fn select(&self, events: Vec<PerceptionEvent>, k: usize) -> Vec<PerceptionEvent>;
}

/// TopK attention: 按 attention_score desc 排序取前 K
pub struct TopKAttention;

impl Attention for TopKAttention {
    fn name(&self) -> &'static str {
        "top_k"
    }

    fn select(&self, mut events: Vec<PerceptionEvent>, k: usize) -> Vec<PerceptionEvent> {
        events.sort_by(|a, b| {
            b.attention_score
                .partial_cmp(&a.attention_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        events.truncate(k);
        events
    }
}

/// Threshold attention: 只保留 score >= threshold
pub struct ThresholdAttention {
    pub threshold: f64,
}

impl Attention for ThresholdAttention {
    fn name(&self) -> &'static str {
        "threshold"
    }

    fn select(&self, events: Vec<PerceptionEvent>, _k: usize) -> Vec<PerceptionEvent> {
        events
            .into_iter()
            .filter(|e| e.attention_score >= self.threshold)
            .collect()
    }
}

// ============================================
// 统一错误
// ============================================

#[derive(Debug)]
pub enum PerceptionError {
    /// 0 装 PASS: modality 在 v2.0 alpha 不实现 (Voice/Vision/Tactile)
    NotImplemented { modality: PerceptionModality, when: &'static str },
    /// 底层 IO 错 (voice 文件读失败 / screen capture 失败)
    Io(String),
    /// 配置错 (channel name 冲突 / attention 阈值非法)
    Config(String),
}

impl std::fmt::Display for PerceptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented { modality, when } => write!(
                f,
                "perception modality {modality:?} not implemented in v2.0 alpha (0 装 PASS; 0 装; 路线: {when})"
            ),
            Self::Io(msg) => write!(f, "perception io error: {msg}"),
            Self::Config(msg) => write!(f, "perception config error: {msg}"),
        }
    }
}

impl std::error::Error for PerceptionError {}

// ============================================
// v2.0 alpha 唯一真实现: Text
// ============================================

/// 文本输入 (唯一 v2.0 alpha 实现的 modality).
///
/// 用途: CLI body / HTTP body / 内部 tool result 转 PerceptionEvent 走 channel.
/// `std::sync::Mutex<Option<String>>` 是 Send+Sync (满足 trait bound),
/// 实现一次性 drain (take 后置 None).
pub struct TextInput {
    pub session_id: SessionId,
    pub source: PerceptionModality,
    next: std::sync::Mutex<Option<String>>,
}

impl TextInput {
    pub fn new(session_id: SessionId, text: String) -> Self {
        Self {
            session_id,
            source: PerceptionModality::Text,
            next: std::sync::Mutex::new(Some(text)),
        }
    }

    pub fn new_command(session_id: SessionId, text: String) -> Self {
        Self {
            session_id,
            source: PerceptionModality::Command,
            next: std::sync::Mutex::new(Some(text)),
        }
    }
}

impl PerceptionInput for TextInput {
    fn modality(&self) -> PerceptionModality {
        self.source
    }

    fn next_event(&self) -> Result<Option<PerceptionEvent>, PerceptionError> {
        // 0 装 PASS: v2.0 alpha 单事件接口. Mutex::lock 拿 + take 置 None,
        // 实现一次性 drain. 真 async streaming 在 v2.1 与 PerceptionChannel 一起.
        let mut guard = self
            .next
            .lock()
            .map_err(|e| PerceptionError::Io(format!("mutex poisoned: {e}")))?;
        let Some(payload) = guard.take() else {
            return Ok(None);
        };
        let event = PerceptionEvent {
            id: format!("text-{}", self.session_id),
            source: self.source,
            session_id: self.session_id,
            timestamp_ms: 0,
            payload: serde_json::json!({ "text": payload }),
            attention_score: 1.0,
            tags: vec!["text".into()],
        };
        Ok(Some(event))
    }
}

// ============================================
// 0 装占位: Voice / Vision / Tactile
// ============================================

/// 0 装: voice input 仅占位, v2.0 alpha next_event() 返 NotImplemented
pub struct VoiceInput {
    pub session_id: SessionId,
}

impl PerceptionInput for VoiceInput {
    fn modality(&self) -> PerceptionModality {
        PerceptionModality::Voice
    }

    fn next_event(&self) -> Result<Option<PerceptionEvent>, PerceptionError> {
        Err(PerceptionError::NotImplemented {
            modality: PerceptionModality::Voice,
            when: "v2.1 (与 scene-d 例 1 / companion-desktop 集成一起做)",
        })
    }
}

pub struct VisionInput {
    pub session_id: SessionId,
}

impl PerceptionInput for VisionInput {
    fn modality(&self) -> PerceptionModality {
        PerceptionModality::Vision
    }

    fn next_event(&self) -> Result<Option<PerceptionEvent>, PerceptionError> {
        Err(PerceptionError::NotImplemented {
            modality: PerceptionModality::Vision,
            when: "v2.1 (screen perception 走 companion-desktop 路由)",
        })
    }
}

pub struct TactileInput {
    pub session_id: SessionId,
}

impl PerceptionInput for TactileInput {
    fn modality(&self) -> PerceptionModality {
        PerceptionModality::Tactile
    }

    fn next_event(&self) -> Result<Option<PerceptionEvent>, PerceptionError> {
        Err(PerceptionError::NotImplemented {
            modality: PerceptionModality::Tactile,
            when: "v2.1 (物理触觉 sensor 集成时再决定是否需要)",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 装 PASS: Text modality 是 v2.0 alpha 唯一真实现, 一次性 drain
    #[test]
    fn text_input_drains_one_event() {
        let sid = SessionId::new();
        let input = TextInput::new(sid, "hello".into());
        let e1 = input.next_event().unwrap().expect("first event");
        assert_eq!(e1.source, PerceptionModality::Text);
        assert_eq!(e1.payload["text"], "hello");
        assert_eq!(e1.attention_score, 1.0);

        // 第二次调用返 None (已 drain)
        let e2 = input.next_event().unwrap();
        assert!(e2.is_none());
    }

    /// TextInput::new_command 走 Command modality
    #[test]
    fn text_input_command_modality() {
        let sid = SessionId::new();
        let input = TextInput::new_command(sid, "deploy".into());
        assert_eq!(input.modality(), PerceptionModality::Command);
    }

    /// 0 装 PASS: Voice/Vision/Tactile next_event() 返 NotImplemented, 标得很清楚
    #[test]
    fn voice_vision_tactile_are_zero_implementation() {
        let sid = SessionId::new();
        for input in [
            Box::new(VoiceInput { session_id: sid }) as Box<dyn PerceptionInput>,
            Box::new(VisionInput { session_id: sid }),
            Box::new(TactileInput { session_id: sid }),
        ] {
            let err = input.next_event().expect_err("should fail").to_string();
            assert!(err.contains("0 装"), "error must document 0 装: {err}");
            assert!(err.contains("v2.1"), "error must mention v2.1 path: {err}");
        }
    }

    /// TopK attention: 按 score desc 排序, 末尾 truncate
    #[test]
    fn topk_selects_highest_scores() {
        let sid = SessionId::new();
        let mut events = Vec::new();
        for (i, score) in [(0, 0.1), (1, 0.9), (2, 0.5), (3, 0.7), (4, 0.3)].iter() {
            events.push(PerceptionEvent {
                id: format!("e-{i}"),
                source: PerceptionModality::Text,
                session_id: sid,
                timestamp_ms: 0,
                payload: serde_json::json!({}),
                attention_score: *score,
                tags: vec![],
            });
        }
        let top2 = TopKAttention.select(events, 2);
        assert_eq!(top2.len(), 2);
        assert_eq!(top2[0].id, "e-1"); // score 0.9
        assert_eq!(top2[1].id, "e-3"); // score 0.7
    }

    /// Threshold attention: 过滤 < threshold
    #[test]
    fn threshold_filters_below_cutoff() {
        let sid = SessionId::new();
        let events = (0..3)
            .map(|i| PerceptionEvent {
                id: format!("e-{i}"),
                source: PerceptionModality::Text,
                session_id: sid,
                timestamp_ms: 0,
                payload: serde_json::json!({}),
                attention_score: f64::from(i) * 0.4, // 0.0, 0.4, 0.8
                tags: vec![],
            })
            .collect();
        let t = ThresholdAttention { threshold: 0.5 };
        let kept = t.select(events, 99);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].id, "e-2");
    }
}
