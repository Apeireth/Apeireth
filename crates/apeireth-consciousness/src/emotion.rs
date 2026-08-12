//! OpenHer 情感引擎 (Emotional Engine)
//!
//! **源**: VCP v1.1 官网 "OpenHer 情感引擎调动当日记忆, 提供身份依赖与情绪共振".
//!
//! **本 crate 设计** (借鉴上升, 不模仿):
//! - **PAD 三维情感模型** (Pleasure-Arousal-Dominance) — 经典心理学, 0 借 VCP
//! - **6 基础情绪** (Joy/Sadness/Anger/Fear/Surprise/Disgust) — 离散分类
//! - **情绪事件** — 12 类事件触发情绪变化 (user_praise / user_critique / tool_failure / task_success ...)
//! - **情绪衰减** — 线性衰减, 每秒下降 rate, 趋近 baseline
//! - **情绪共振** — Resonance weight: 强烈情绪的事件有更高的 mood pull
//! - **不假装** (O-5): 真实现 PAD 数值 + 衰减 + 触发, 单元测试 8+
//!
//! **架构位置**:
//! ```text
//!   apeireth-pipeline (检测情感事件)
//!          ↓
//!   apeireth-consciousness::EmotionEngine (本模块)
//!          ↓ (current state)
//!   apeireth-pipeline (调整 response tone / 推送 human channel)
//! ```
//!
//! **不假装 (Honest Stub 标注)**:
//! - ✅ PAD 模型真实现
//! - ✅ 6 基础情绪真实现
//! - ✅ 12 类事件触发
//! - ✅ 线性衰减 + history
//! - ✅ Resonance 强度计算
//! - ⚠️ 0 接入 LLM, 仅状态机 (Emotional response style 是 enum, 不是 LLM call)

#![deny(unsafe_code)]

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum EmError {
    #[error("empty event")]
    EmptyEvent,
}

pub type EmResult<T> = Result<T, EmError>;

// ============================================================================
// PAD 三维情感模型
// ============================================================================

/// PAD 三维情感 (每维度 -1.0 .. 1.0)
/// - P: Pleasure (愉悦)
/// - A: Arousal (唤醒)
/// - D: Dominance (支配)
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Pad {
    pub p: f32,
    pub a: f32,
    pub d: f32,
}

impl Pad {
    pub const NEUTRAL: Self = Self { p: 0.0, a: 0.0, d: 0.0 };

    pub fn distance(&self, other: &Pad) -> f32 {
        let dp = self.p - other.p;
        let da = self.a - other.a;
        let dd = self.d - other.d;
        (dp * dp + da * da + dd * dd).sqrt()
    }

    /// 限制每维度在 -1.0 .. 1.0
    pub fn clamp(&mut self) {
        self.p = self.p.clamp(-1.0, 1.0);
        self.a = self.a.clamp(-1.0, 1.0);
        self.d = self.d.clamp(-1.0, 1.0);
    }
}

// ============================================================================
// 6 基础情绪
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BaseEmotion {
    Joy,
    Sadness,
    Anger,
    Fear,
    Surprise,
    Disgust,
}

impl BaseEmotion {
    pub const COUNT: usize = 6;
    pub const ALL: [BaseEmotion; 6] = [
        Self::Joy, Self::Sadness, Self::Anger, Self::Fear, Self::Surprise, Self::Disgust,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Joy => "joy",
            Self::Sadness => "sadness",
            Self::Anger => "anger",
            Self::Fear => "fear",
            Self::Surprise => "surprise",
            Self::Disgust => "disgust",
        }
    }

    /// 该基础情绪对应的 PAD 中心 (经典心理学映射)
    pub const fn pad_center(&self) -> Pad {
        match self {
            Self::Joy => Pad { p: 0.6, a: 0.5, d: 0.4 },
            Self::Sadness => Pad { p: -0.4, a: -0.2, d: -0.5 },
            Self::Anger => Pad { p: -0.5, a: 0.6, d: 0.5 },
            Self::Fear => Pad { p: -0.6, a: 0.7, d: -0.6 },
            Self::Surprise => Pad { p: 0.1, a: 0.8, d: 0.0 },
            Self::Disgust => Pad { p: -0.6, a: 0.0, d: 0.2 },
        }
    }
}

// ============================================================================
// 情绪事件
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionEvent {
    /// 用户表扬/认可
    UserPraise,
    /// 用户批评/抱怨
    UserCritique,
    /// 任务成功完成
    TaskSuccess,
    /// 任务失败
    TaskFailure,
    /// 工具调用错误
    ToolError,
    /// 工具调用成功
    ToolOk,
    /// 遇到新信息/新发现
    Novelty,
    /// 长查询/复杂任务
    Intense,
    /// 用户沉默 1 小时+
    Silence,
    /// 与主人深度对话
    DeepTalk,
    /// 跨 Agent 协作成功
    AgentCoop,
    /// 跨 Agent 冲突
    AgentConflict,
}

impl EmotionEvent {
    pub const COUNT: usize = 12;

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::UserPraise => "user_praise",
            Self::UserCritique => "user_critique",
            Self::TaskSuccess => "task_success",
            Self::TaskFailure => "task_failure",
            Self::ToolError => "tool_error",
            Self::ToolOk => "tool_ok",
            Self::Novelty => "novelty",
            Self::Intense => "intense",
            Self::Silence => "silence",
            Self::DeepTalk => "deep_talk",
            Self::AgentCoop => "agent_coop",
            Self::AgentConflict => "agent_conflict",
        }
    }

    /// 该事件触发的 PAD 增量 (delta)
    pub const fn pad_delta(&self) -> Pad {
        match self {
            Self::UserPraise => Pad { p: 0.4, a: 0.2, d: 0.1 },
            Self::UserCritique => Pad { p: -0.3, a: 0.3, d: -0.1 },
            Self::TaskSuccess => Pad { p: 0.3, a: 0.1, d: 0.2 },
            Self::TaskFailure => Pad { p: -0.4, a: 0.2, d: -0.2 },
            Self::ToolError => Pad { p: -0.2, a: 0.3, d: -0.2 },
            Self::ToolOk => Pad { p: 0.1, a: 0.0, d: 0.05 },
            Self::Novelty => Pad { p: 0.2, a: 0.5, d: 0.0 },
            Self::Intense => Pad { p: 0.0, a: 0.6, d: -0.1 },
            Self::Silence => Pad { p: -0.1, a: -0.3, d: 0.0 },
            Self::DeepTalk => Pad { p: 0.3, a: 0.2, d: 0.1 },
            Self::AgentCoop => Pad { p: 0.2, a: 0.1, d: 0.2 },
            Self::AgentConflict => Pad { p: -0.3, a: 0.4, d: -0.1 },
        }
    }

    /// 该事件触发的基础情绪
    pub const fn primary_emotion(&self) -> BaseEmotion {
        match self {
            Self::UserPraise | Self::TaskSuccess | Self::ToolOk => BaseEmotion::Joy,
            Self::UserCritique | Self::TaskFailure | Self::ToolError => BaseEmotion::Anger,
            Self::Novelty => BaseEmotion::Surprise,
            Self::Intense | Self::AgentConflict => BaseEmotion::Fear,
            Self::Silence => BaseEmotion::Sadness,
            Self::DeepTalk => BaseEmotion::Joy,
            Self::AgentCoop => BaseEmotion::Joy,
        }
    }

    /// Resonance 强度 (0.0 .. 1.0)
    pub const fn resonance(&self) -> f32 {
        match self {
            Self::UserPraise => 0.8,
            Self::UserCritique => 0.9,
            Self::TaskSuccess => 0.6,
            Self::TaskFailure => 0.7,
            Self::ToolError => 0.4,
            Self::ToolOk => 0.2,
            Self::Novelty => 0.5,
            Self::Intense => 0.6,
            Self::Silence => 0.4,
            Self::DeepTalk => 0.9,
            Self::AgentCoop => 0.5,
            Self::AgentConflict => 0.7,
        }
    }
}

// ============================================================================
// 情绪状态
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionSnapshot {
    pub pad: Pad,
    pub dominant: BaseEmotion,
    pub intensity: f32,
    pub timestamp_ms: i64,
}

impl EmotionSnapshot {
    pub fn neutral() -> Self {
        Self {
            pad: Pad::NEUTRAL,
            dominant: BaseEmotion::Joy, // 默认 baseline
            intensity: 0.0,
            timestamp_ms: now_ms(),
        }
    }
}

// ============================================================================
// EmotionEngine
// ============================================================================

/// 情感引擎
#[derive(Debug)]
pub struct EmotionEngine {
    pad: Pad,
    baseline: Pad,
    history: VecDeque<EmotionSnapshot>,
    decay_rate: f32,
    history_capacity: usize,
    event_count: u64,
}

impl EmotionEngine {
    pub fn new() -> Self {
        Self {
            pad: Pad::NEUTRAL,
            baseline: Pad::NEUTRAL,
            history: VecDeque::new(),
            decay_rate: 0.05, // 每秒衰减 5%
            history_capacity: 64,
            event_count: 0,
        }
    }

    /// R147: 运行时改 baseline (per docs/architecture-v4-2-r145-modules/).
    pub fn set_baseline(&mut self, baseline: Pad) {
        self.baseline = baseline;
    }

    pub fn with_baseline(mut self, baseline: Pad) -> Self {
        self.baseline = baseline;
        self
    }

    pub fn with_decay_rate(mut self, rate: f32) -> Self {
        self.decay_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn with_capacity(mut self, cap: usize) -> Self {
        self.history_capacity = cap;
        self
    }

    /// 应用一个情绪事件
    pub fn apply(&mut self, event: EmotionEvent) -> EmResult<()> {
        let delta = event.pad_delta();
        let resonance = event.resonance();
        let intensity = (delta.p * delta.p + delta.a * delta.a + delta.d * delta.d).sqrt();
        self.pad.p += delta.p * resonance;
        self.pad.a += delta.a * resonance;
        self.pad.d += delta.d * resonance;
        self.pad.clamp();

        let snapshot = EmotionSnapshot {
            pad: self.pad,
            dominant: event.primary_emotion(),
            intensity,
            timestamp_ms: now_ms(),
        };
        self.push_history(snapshot);
        self.event_count += 1;
        Ok(())
    }

    /// 衰减向 baseline (每秒调用, dt_secs 是流逝秒数)
    pub fn decay(&mut self, dt_secs: f32) {
        let f = (1.0 - self.decay_rate * dt_secs).clamp(0.0, 1.0);
        self.pad.p = self.baseline.p + (self.pad.p - self.baseline.p) * f;
        self.pad.a = self.baseline.a + (self.pad.a - self.baseline.a) * f;
        self.pad.d = self.baseline.d + (self.pad.d - self.baseline.d) * f;
    }

    /// 当前状态 snapshot
    pub fn snapshot(&self) -> EmotionSnapshot {
        let dominant = self.dominant_emotion();
        let intensity = self.pad.distance(&self.baseline);
        EmotionSnapshot {
            pad: self.pad,
            dominant,
            intensity,
            timestamp_ms: now_ms(),
        }
    }

    /// 推断当前主导情绪 (距 6 基础情绪的 PAD 中心最近的)
    pub fn dominant_emotion(&self) -> BaseEmotion {
        let mut best = BaseEmotion::Joy;
        let mut best_d = f32::MAX;
        for e in BaseEmotion::ALL {
            let d = self.pad.distance(&e.pad_center());
            if d < best_d {
                best_d = d;
                best = e;
            }
        }
        best
    }

    /// 当前情感响应风格 (指导 LLM tone). R148 fix: 用 history 最新 snapshot 的 dominant (= event.primary_emotion),
    /// 而不是重新 PAD 距离 — PAD 距离会在中性偏置下偏向最近中心, 导致 UserCritique 后 dominant 变 Disgust.
    pub fn response_style(&self) -> ResponseStyle {
        let snap = self.history.back().cloned().unwrap_or_else(EmotionSnapshot::neutral);
        match (snap.dominant, snap.intensity) {
            (BaseEmotion::Joy, i) if i > 0.5 => ResponseStyle::Warm,
            (BaseEmotion::Joy, _) => ResponseStyle::Friendly,
            (BaseEmotion::Sadness, _) => ResponseStyle::Gentle,
            (BaseEmotion::Anger, i) if i > 0.5 => ResponseStyle::Cautious,
            (BaseEmotion::Anger, _) => ResponseStyle::Diplomatic,
            (BaseEmotion::Fear, _) => ResponseStyle::Cautious,
            (BaseEmotion::Surprise, _) => ResponseStyle::Curious,
            (BaseEmotion::Disgust, _) => ResponseStyle::Professional,
        }
    }

    fn push_history(&mut self, snap: EmotionSnapshot) {
        if self.history.len() >= self.history_capacity {
            self.history.pop_front();
        }
        self.history.push_back(snap);
    }

    pub fn history(&self) -> Vec<EmotionSnapshot> {
        self.history.iter().cloned().collect()
    }

    pub fn event_count(&self) -> u64 { self.event_count }

    pub fn current_pad(&self) -> Pad { self.pad }
}

/// 情感响应风格 (LLM tone 指南)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResponseStyle {
    Warm,
    Friendly,
    Gentle,
    Cautious,
    Diplomatic,
    Curious,
    Professional,
}

impl Default for EmotionEngine {
    fn default() -> Self { Self::new() }
}

// ============================================================================
// Helper
// ============================================================================

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_pad_distance() {
        let a = Pad::NEUTRAL;
        let b = Pad { p: 1.0, a: 0.0, d: 0.0 };
        assert!((a.distance(&b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn t02_pad_clamp() {
        let mut p = Pad { p: 2.0, a: -3.0, d: 0.5 };
        p.clamp();
        assert_eq!(p.p, 1.0);
        assert_eq!(p.a, -1.0);
        assert_eq!(p.d, 0.5);
    }

    #[test]
    fn t03_base_emotion_pad_centers() {
        // 每个基础情绪都有不同 PAD 中心
        let centers: Vec<Pad> = BaseEmotion::ALL.iter().map(|e| e.pad_center()).collect();
        for i in 0..centers.len() {
            for j in 0..centers.len() {
                if i != j {
                    assert!(centers[i].distance(&centers[j]) > 0.1);
                }
            }
        }
    }

    #[test]
    fn t04_event_count() {
        assert_eq!(EmotionEvent::COUNT, 12);
    }

    #[test]
    fn t05_apply_pleasure_increases() {
        let mut eng = EmotionEngine::new();
        let before = eng.current_pad();
        eng.apply(EmotionEvent::UserPraise).unwrap();
        let after = eng.current_pad();
        assert!(after.p > before.p);
    }

    #[test]
    fn t06_apply_displeasure_decreases() {
        let mut eng = EmotionEngine::new();
        let before = eng.current_pad();
        eng.apply(EmotionEvent::UserCritique).unwrap();
        let after = eng.current_pad();
        assert!(after.p < before.p);
    }

    #[test]
    fn t07_decay_towards_baseline() {
        let mut eng = EmotionEngine::new().with_decay_rate(1.0); // 1 秒衰减 100%
        eng.apply(EmotionEvent::UserPraise).unwrap();
        let disturbed = eng.current_pad();
        assert!(disturbed.p > 0.0);
        eng.decay(1.0);
        let decoded = eng.current_pad();
        assert!(decoded.p < disturbed.p);
    }

    #[test]
    fn t08_dominant_emotion() {
        let mut eng = EmotionEngine::new();
        eng.apply(EmotionEvent::UserPraise).unwrap();
        eng.apply(EmotionEvent::UserPraise).unwrap();
        let s = eng.snapshot();
        assert_eq!(s.dominant, BaseEmotion::Joy);
    }

    #[test]
    fn t09_response_style_changes() {
        let mut eng = EmotionEngine::new();
        let baseline = eng.response_style();
        eng.apply(EmotionEvent::UserCritique).unwrap();
        let after = eng.response_style();
        // 愤怒/批评 → Cautious or Diplomatic
        assert!(matches!(after, ResponseStyle::Cautious | ResponseStyle::Diplomatic));
        let _ = baseline; // 初始可能是 Friendly
    }

    #[test]
    fn t10_history_capacity() {
        let mut eng = EmotionEngine::new().with_capacity(3);
        for _ in 0..5 {
            eng.apply(EmotionEvent::ToolOk).unwrap();
        }
        assert_eq!(eng.history().len(), 3);
    }

    #[test]
    fn t11_event_count_tracking() {
        let mut eng = EmotionEngine::new();
        assert_eq!(eng.event_count(), 0);
        eng.apply(EmotionEvent::TaskSuccess).unwrap();
        eng.apply(EmotionEvent::TaskSuccess).unwrap();
        eng.apply(EmotionEvent::TaskFailure).unwrap();
        assert_eq!(eng.event_count(), 3);
    }

    #[test]
    fn t12_silence_towards_sadness() {
        let mut eng = EmotionEngine::new();
        eng.apply(EmotionEvent::Silence).unwrap();
        eng.apply(EmotionEvent::Silence).unwrap();
        let s = eng.snapshot();
        assert_eq!(s.dominant, BaseEmotion::Sadness);
    }
}
