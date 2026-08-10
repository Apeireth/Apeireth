//! Apeireth 感知器官 (A9 落点 — R14 Phase 4).
//!
//! **职责**: 外部输入接入层 — 把来自不同来源（CLI/TTY/HTTP/Python 桥）的
//! 信号/IO/Token 流统一为 `PerceptionEvent`，交给 cognition 器官处理。
//!
//! **架构位置**: 阶段 4 §2 主路径 17 crate 之 A9 器官 (本源推导 9 维: 感知).
//!
//! **本 crate 提供**:
//! - [`PerceptionInput`] trait + 5 种输入 (Text/Voice/Vision/Tactile/Command)
//! - [`Attention`] trait + 2 种内置策略 (TopK/Threshold)
//! - [`PerceptionChannel`] trait + 5 种通道 (一对一对应输入类型)
//! - [`PerceptionEvent`] — cognition 的统一输入格式
//!
//! **诚实登记**: 按 `leader-handover-final-2026-08-01` §B 简化实现 (5+ pub fn,
//! 5+ tests, 1+ integration test, examples). 完整显著性与多模态融合待阶段 5.
//!
//! **禁止**:
//! - ❌ 不修改 apeireth-core / apeireth-cognition 任何已实装类型签名
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不碰 apeireth-legacy/

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
// R37-2: 9 organ 部分合并 — consciousness → perception 透明 re-export (workspace member 真删)
// 下游调用方 `use apeireth_consciousness::X` 仍能用 (R37-2 后 0 breaking)
pub use apeireth_consciousness::*;

/// 顶层错误: perception 子系统的 fallback error.
#[derive(Debug, Error)]
pub enum PerceptionError {
    /// 输入参数非法.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// 通道不接受该输入.
    #[error("channel mismatch: {0}")]
    ChannelMismatch(String),
    /// 序列化错误.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// 统一结果类型.
pub type PerceptionResult<T> = Result<T, PerceptionError>;

// ============================================
// 顶层便捷函数 (5+ pub fn 在 lib 层暴露)
// ============================================

/// 当前 Unix 时间戳 (秒).
pub fn now_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 默认注意力阈值 (0.5) — 平衡信噪比的工程常量.
pub fn default_attention_threshold() -> f64 {
    0.5
}

/// 默认 Top-K 数量 (5) — 与 R11 baseline 单批处理上限对齐.
pub fn default_top_k() -> usize {
    5
}

/// 给定通道批量处理一批输入.
pub fn batch_process<C: PerceptionChannel>(
    channel: &C,
    inputs: Vec<C::Input>,
) -> Vec<PerceptionEvent> {
    process_all(channel, inputs)
}

/// 端到端便捷函数: 输入 → 通道 → 注意力过滤 → 事件.
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

/// 校验 `PerceptionEvent` 基本字段 (用于内部测试 / 反思期守门).
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
        // 应在 2024-01-01 之后, 2100-01-01 之前.
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
        // 直接构造一个越界事件 (绕过 PerceptionEvent::new 的 clamp).
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
