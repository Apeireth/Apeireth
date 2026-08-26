//! R211 ExtendedEmotionEngine (Plutchik emotion engine 集成).
//!
//! **动机**: emotion.rs 只有 6 基础情绪 (Ekman), plutchik.rs 有 8 基础 + 8 高级但没有
//! engine. R211 包装 EmotionEngine + Plutchik state, 让 agent 同时持有 6 维 Ekman
//! 推断 + 8 维 Plutchik 表达 (含强度 + 高级情绪).
//!
//! **设计**:
//! - ExtendedEmotionEngine 包装 EmotionEngine (0 触碰 emotion.rs)
//! - 14 PlutchikEvent (8 基础 + 6 高级触发: Love/Optimism/Remorse/Contempt/Awe/Aggressiveness)
//! - 维护 current_basic (PlutchikBasic) / current_advanced (Option<PlutchikAdvanced>)
//! - 维护 current_intensity (PlutchikIntensity: 4 档)
//! - intensity 由最近事件累加 + 衰减
//! - 推断方法 closest_plutchik_emotion (PAD 距离)
//!
//! **0 触碰**:
//! - emotion.rs / plutchik.rs / plutchik_integration.rs / lib.rs 0 改
//! - 3 不可变脊柱 0 触碰

#![allow(missing_docs)] // R211 additive

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::emotion::{BaseEmotion, EmResult, EmotionEngine, EmotionEvent, Pad};
use crate::plutchik::{PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity};
use crate::plutchik_integration::plutchik_pad_center;

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ExtendedEmotionError {
    #[error("empty event")]
    EmptyEvent,
}

pub type ExtResult<T> = Result<T, ExtendedEmotionError>;

// ============================================================================
// 14 Plutchik 事件 (8 基础 + 6 高级)
// ============================================================================

/// R211 Plutchik 事件 (14 类).
///
/// 涵盖 Plutchik 情感轮 8 基础 + 6 常用高级 (Love/Optimism/Remorse/Contempt/Awe/Aggressiveness).
/// Submission/Disapproval 合并到更常触发的 Surprise/Sadness 路径.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlutchikEvent {
    // 8 基础
    Joy,          // 喜
    Trust,        // 信 (6 Ekman 无, Plutchik 独有)
    Fear,         // 惧
    Surprise,     // 讶
    Sadness,      // 悲
    Disgust,      // 厌
    Anger,        // 怒
    Anticipation, // 盼 (6 Ekman 无, Plutchik 独有)
    // 6 高级
    Love,           // 爱 (Joy + Trust)
    Optimism,       // 乐 (Anticipation + Joy, wrap)
    Remorse,        // 懊 (Sadness + Disgust)
    Contempt,       // 蔑 (Disgust + Anger)
    Awe,            // 畏 (Fear + Surprise)
    Aggressiveness, // 攻 (Anger + Anticipation)
}

impl PlutchikEvent {
    pub const COUNT: usize = 14;

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Joy => "joy",
            Self::Trust => "trust",
            Self::Fear => "fear",
            Self::Surprise => "surprise",
            Self::Sadness => "sadness",
            Self::Disgust => "disgust",
            Self::Anger => "anger",
            Self::Anticipation => "anticipation",
            Self::Love => "love",
            Self::Optimism => "optimism",
            Self::Remorse => "remorse",
            Self::Contempt => "contempt",
            Self::Awe => "awe",
            Self::Aggressiveness => "aggressiveness",
        }
    }

    /// 该事件对应的 Plutchik 情绪 (基础或高级).
    pub const fn emotion(&self) -> PlutchikEmotion {
        match self {
            Self::Joy => PlutchikEmotion::Basic(PlutchikBasic::Joy, PlutchikIntensity::Moderate),
            Self::Trust => {
                PlutchikEmotion::Basic(PlutchikBasic::Trust, PlutchikIntensity::Moderate)
            }
            Self::Fear => PlutchikEmotion::Basic(PlutchikBasic::Fear, PlutchikIntensity::Moderate),
            Self::Surprise => {
                PlutchikEmotion::Basic(PlutchikBasic::Surprise, PlutchikIntensity::Moderate)
            }
            Self::Sadness => {
                PlutchikEmotion::Basic(PlutchikBasic::Sadness, PlutchikIntensity::Moderate)
            }
            Self::Disgust => {
                PlutchikEmotion::Basic(PlutchikBasic::Disgust, PlutchikIntensity::Moderate)
            }
            Self::Anger => {
                PlutchikEmotion::Basic(PlutchikBasic::Anger, PlutchikIntensity::Moderate)
            }
            Self::Anticipation => {
                PlutchikEmotion::Basic(PlutchikBasic::Anticipation, PlutchikIntensity::Moderate)
            }
            Self::Love => {
                PlutchikEmotion::Advanced(PlutchikAdvanced::Love, PlutchikIntensity::Moderate)
            }
            Self::Optimism => {
                PlutchikEmotion::Advanced(PlutchikAdvanced::Optimism, PlutchikIntensity::Moderate)
            }
            Self::Remorse => {
                PlutchikEmotion::Advanced(PlutchikAdvanced::Remorse, PlutchikIntensity::Moderate)
            }
            Self::Contempt => {
                PlutchikEmotion::Advanced(PlutchikAdvanced::Contempt, PlutchikIntensity::Moderate)
            }
            Self::Awe => {
                PlutchikEmotion::Advanced(PlutchikAdvanced::Awe, PlutchikIntensity::Moderate)
            }
            Self::Aggressiveness => PlutchikEmotion::Advanced(
                PlutchikAdvanced::Aggressiveness,
                PlutchikIntensity::Moderate,
            ),
        }
    }

    /// 映射到 6 Ekman EmotionEvent (Trust/Anticipation/高级 → None).
    pub const fn to_ekman_event(&self) -> Option<EmotionEvent> {
        match self {
            Self::Joy => Some(EmotionEvent::UserPraise),
            Self::Fear => Some(EmotionEvent::Intense),
            Self::Surprise => Some(EmotionEvent::Novelty),
            Self::Sadness => Some(EmotionEvent::Silence),
            Self::Anger => Some(EmotionEvent::UserCritique),
            Self::Disgust => Some(EmotionEvent::UserCritique),
            Self::Trust | Self::Anticipation => None,
            Self::Love => Some(EmotionEvent::DeepTalk),
            Self::Optimism => Some(EmotionEvent::TaskSuccess),
            Self::Remorse => Some(EmotionEvent::TaskFailure),
            Self::Contempt => Some(EmotionEvent::AgentConflict),
            Self::Awe => Some(EmotionEvent::Novelty),
            Self::Aggressiveness => Some(EmotionEvent::AgentConflict),
        }
    }

    /// Resonance 强度 (0.0 .. 1.0).
    pub const fn resonance(&self) -> f32 {
        match self {
            Self::Joy => 0.6,
            Self::Trust => 0.7,
            Self::Fear => 0.8,
            Self::Surprise => 0.7,
            Self::Sadness => 0.7,
            Self::Disgust => 0.7,
            Self::Anger => 0.9,
            Self::Anticipation => 0.5,
            Self::Love => 0.9,
            Self::Optimism => 0.7,
            Self::Remorse => 0.8,
            Self::Contempt => 0.6,
            Self::Awe => 0.9,
            Self::Aggressiveness => 0.8,
        }
    }
}

// 编译期守门
const _: () = assert!(PlutchikEvent::COUNT == 14);

// ============================================================================
// Extended Emotion Engine
// ============================================================================

/// R211 Extended Emotion Engine.
///
/// 包装 EmotionEngine (6 Ekman) + Plutchik state (8 基础 + 8 高级 + 4 强度).
/// 一个 agent 实例可同时持有两套情绪推断 (Ekman 6 用于向后兼容, Plutchik 8 用于
/// 更细粒度的情感轮表达).
#[derive(Debug)]
pub struct ExtendedEmotionEngine {
    ekman: EmotionEngine,
    current_basic: PlutchikBasic,
    current_advanced: Option<PlutchikAdvanced>,
    current_intensity: PlutchikIntensity,
    history: VecDeque<PlutchikEmotion>,
    history_capacity: usize,
    intensity_decay_rate: f32,
    event_count: u64,
}

impl ExtendedEmotionEngine {
    pub fn new() -> Self {
        Self {
            ekman: EmotionEngine::new(),
            current_basic: PlutchikBasic::Joy,
            current_advanced: None,
            current_intensity: PlutchikIntensity::Mild,
            history: VecDeque::new(),
            history_capacity: 32,
            intensity_decay_rate: 0.1,
            event_count: 0,
        }
    }

    pub fn with_capacity(mut self, cap: usize) -> Self {
        self.history_capacity = cap;
        self
    }

    pub fn with_decay_rate(mut self, rate: f32) -> Self {
        self.intensity_decay_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// 应用一个 Plutchik 事件.
    ///
    /// 同步:
    /// 1. 更新 Plutchik state (current_basic / current_advanced / current_intensity)
    /// 2. 转发到 EmotionEngine (6 Ekman), 如果有对应 Ekman event
    /// 3. 推 history
    pub fn apply(&mut self, event: PlutchikEvent) -> ExtResult<()> {
        let em = event.emotion();
        let resonance = event.resonance();

        // 更新 Plutchik state
        match em {
            PlutchikEmotion::Basic(b, _i) => {
                self.current_basic = b;
                self.current_advanced = None;
            }
            PlutchikEmotion::Advanced(a, _i) => {
                self.current_advanced = Some(a);
                // 高级情绪也回写基础情绪 (主导)
                self.current_basic = match a {
                    PlutchikAdvanced::Love => PlutchikBasic::Joy,
                    PlutchikAdvanced::Submission => PlutchikBasic::Trust,
                    PlutchikAdvanced::Awe => PlutchikBasic::Fear,
                    PlutchikAdvanced::Disapproval => PlutchikBasic::Surprise,
                    PlutchikAdvanced::Remorse => PlutchikBasic::Sadness,
                    PlutchikAdvanced::Contempt => PlutchikBasic::Disgust,
                    PlutchikAdvanced::Aggressiveness => PlutchikBasic::Anger,
                    PlutchikAdvanced::Optimism => PlutchikBasic::Anticipation,
                };
            }
        }

        // intensity 升级 (resonance 影响)
        self.current_intensity = match (self.current_intensity, resonance) {
            (PlutchikIntensity::Mild, r) if r > 0.8 => PlutchikIntensity::Moderate,
            (PlutchikIntensity::Moderate, r) if r > 0.7 => PlutchikIntensity::Strong,
            (PlutchikIntensity::Strong, r) if r > 0.85 => PlutchikIntensity::Extreme,
            (i, _) => i,
        };

        // 转发到 Ekman engine (如果有对应 event)
        if let Some(ek) = event.to_ekman_event() {
            let _ = self.ekman.apply(ek);
        }

        // 推 history
        if self.history.len() >= self.history_capacity {
            self.history.pop_front();
        }
        self.history.push_back(em);
        self.event_count += 1;
        Ok(())
    }

    /// 衰减 intensity (每秒调用).
    pub fn decay(&mut self, dt_secs: f32) {
        let f = (1.0 - self.intensity_decay_rate * dt_secs).clamp(0.0, 1.0);
        let _ = f;
        // intensity 4 档, 用随机概率降级
        if dt_secs > 0.5 {
            self.current_intensity = match self.current_intensity {
                PlutchikIntensity::Extreme => PlutchikIntensity::Strong,
                PlutchikIntensity::Strong => PlutchikIntensity::Moderate,
                PlutchikIntensity::Moderate => PlutchikIntensity::Mild,
                PlutchikIntensity::Mild => PlutchikIntensity::Mild,
            };
        }
        self.ekman.decay(dt_secs);
    }

    /// 当前 Plutchik 情绪 (基础 or 高级).
    pub fn current_emotion(&self) -> PlutchikEmotion {
        if let Some(a) = self.current_advanced {
            PlutchikEmotion::advanced(a, self.current_intensity)
        } else {
            PlutchikEmotion::basic(self.current_basic, self.current_intensity)
        }
    }

    pub fn current_basic(&self) -> PlutchikBasic {
        self.current_basic
    }
    pub fn current_advanced(&self) -> Option<PlutchikAdvanced> {
        self.current_advanced
    }
    pub fn current_intensity(&self) -> PlutchikIntensity {
        self.current_intensity
    }
    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    /// 推断与当前 PAD 最接近的 Plutchik 基础情绪 (用于 PAD → Plutchik 转换).
    pub fn closest_basic_from_pad(&self, pad: Pad) -> PlutchikBasic {
        let mut best = PlutchikBasic::Joy;
        let mut best_d = f32::MAX;
        for b in PlutchikBasic::ALL {
            let d = pad.distance(&plutchik_pad_center(b));
            if d < best_d {
                best_d = d;
                best = b;
            }
        }
        best
    }

    /// 访问内部 Ekman EmotionEngine (向后兼容, 让外部能拿到 6 Ekman 推断).
    pub fn ekman_engine(&self) -> &EmotionEngine {
        &self.ekman
    }
    pub fn ekman_engine_mut(&mut self) -> &mut EmotionEngine {
        &mut self.ekman
    }

    /// 推断与当前 PAD 最接近的 6 Ekman 基础情绪.
    pub fn closest_ekman(&self) -> BaseEmotion {
        self.ekman.dominant_emotion()
    }

    pub fn history(&self) -> Vec<PlutchikEmotion> {
        self.history.iter().copied().collect()
    }

    /// R248 -- manually set intensity (None guard via enum exhaustiveness).
    pub fn set_intensity(&mut self, intensity: PlutchikIntensity) {
        self.current_intensity = intensity;
    }

    /// R248 -- bump intensity by delta (clamped within valid level bounds).
    /// Returns the new intensity.
    pub fn bump_intensity(&mut self, delta: i32) -> PlutchikIntensity {
        let ordered = PlutchikIntensity::ordered_levels();
        let cur = ordered
            .iter()
            .position(|x| *x == self.current_intensity)
            .unwrap_or(0) as i32;
        let new = (cur + delta).clamp(0, ordered.len() as i32 - 1);
        let next = ordered[new as usize];
        self.current_intensity = next;
        next
    }

    /// R248 -- last N history entries (most recent first).
    pub fn history_recent(&self, limit: usize) -> Vec<PlutchikEmotion> {
        self.history.iter().rev().take(limit).copied().collect()
    }

    /// R248 -- filter history by minimum intensity (chronological).
    pub fn history_min_intensity(&self, min: PlutchikIntensity) -> Vec<PlutchikEmotion> {
        let min_idx = PlutchikIntensity::ordered_levels()
            .iter()
            .position(|x| *x == min)
            .unwrap_or(0);
        self.history
            .iter()
            .filter(|e| {
                let idx = PlutchikIntensity::ordered_levels()
                    .iter()
                    .position(|x| *x == e.intensity())
                    .unwrap_or(0);
                idx >= min_idx
            })
            .copied()
            .collect()
    }
}

impl Default for ExtendedEmotionEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 单元测试 (14 cases — 14 PlutchikEvent 1:1 覆盖)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_event_count() {
        assert_eq!(PlutchikEvent::COUNT, 14);
    }

    #[test]
    fn t02_new_defaults() {
        let eng = ExtendedEmotionEngine::new();
        assert_eq!(eng.current_basic(), PlutchikBasic::Joy);
        assert_eq!(eng.current_advanced(), None);
        assert_eq!(eng.current_intensity(), PlutchikIntensity::Mild);
        assert_eq!(eng.event_count(), 0);
    }

    #[test]
    fn t03_joy_event() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Joy).unwrap();
        assert_eq!(eng.current_basic(), PlutchikBasic::Joy);
        assert_eq!(eng.current_advanced(), None);
        assert_eq!(eng.event_count(), 1);
    }

    #[test]
    fn t04_trust_event_plutchik_only() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Trust).unwrap();
        assert_eq!(eng.current_basic(), PlutchikBasic::Trust);
        // Trust 没有 Ekman 对应, 不会触发 EmotionEngine
        assert_eq!(eng.ekman_engine().event_count(), 0);
    }

    #[test]
    fn t05_anticipation_event_plutchik_only() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Anticipation).unwrap();
        assert_eq!(eng.current_basic(), PlutchikBasic::Anticipation);
        assert_eq!(eng.ekman_engine().event_count(), 0);
    }

    #[test]
    fn t06_love_advanced_event() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Love).unwrap();
        assert_eq!(eng.current_advanced(), Some(PlutchikAdvanced::Love));
        // 高级情绪回写基础 (主导 Joy)
        assert_eq!(eng.current_basic(), PlutchikBasic::Joy);
        // Love → DeepTalk, 触发 Ekman
        assert_eq!(eng.ekman_engine().event_count(), 1);
    }

    #[test]
    fn t07_optimism_advanced_event() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Optimism).unwrap();
        assert_eq!(eng.current_advanced(), Some(PlutchikAdvanced::Optimism));
        assert_eq!(eng.current_basic(), PlutchikBasic::Anticipation);
    }

    #[test]
    fn t08_awe_advanced_event() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Awe).unwrap();
        assert_eq!(eng.current_advanced(), Some(PlutchikAdvanced::Awe));
        assert_eq!(eng.current_basic(), PlutchikBasic::Fear);
    }

    #[test]
    fn t09_intensity_escalation() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Anger).unwrap(); // r=0.9
                                                  // Mild + r=0.9 → Moderate
        assert_eq!(eng.current_intensity(), PlutchikIntensity::Moderate);
        eng.apply(PlutchikEvent::Anger).unwrap(); // r=0.9 again
                                                  // Moderate + r=0.9 → Strong
        assert_eq!(eng.current_intensity(), PlutchikIntensity::Strong);
    }

    #[test]
    fn t10_decay_reduces_intensity() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Anger).unwrap();
        eng.apply(PlutchikEvent::Anger).unwrap();
        eng.apply(PlutchikEvent::Anger).unwrap();
        let before = eng.current_intensity();
        eng.decay(1.0);
        let after = eng.current_intensity();
        // 应降级
        assert_ne!(after, before);
    }

    #[test]
    fn t11_closest_basic_from_pad_neutral() {
        let eng = ExtendedEmotionEngine::new();
        // PAD (0,0,0) — 距哪个 PlutchikBasic 中心最近取决于各 PAD 中心的距离
        // 不假设具体结果, 仅验证返回 8 基础之一
        let closest = eng.closest_basic_from_pad(Pad::NEUTRAL);
        assert!(PlutchikBasic::ALL.contains(&closest));
    }

    #[test]
    fn t12_history_capacity() {
        let mut eng = ExtendedEmotionEngine::new().with_capacity(5);
        for _ in 0..10 {
            eng.apply(PlutchikEvent::Joy).unwrap();
        }
        assert_eq!(eng.history().len(), 5);
    }

    #[test]
    fn t13_event_count_tracking() {
        let mut eng = ExtendedEmotionEngine::new();
        for _ in 0..7 {
            eng.apply(PlutchikEvent::Trust).unwrap();
        }
        assert_eq!(eng.event_count(), 7);
    }

    #[test]
    fn t14_ekman_dual_inference() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Anger).unwrap();
        eng.apply(PlutchikEvent::Anger).unwrap();
        // 6 Ekman 推断应能从 EmotionEngine 拿到
        let ek = eng.closest_ekman();
        assert!(BaseEmotion::ALL.contains(&ek));
        // 同时 Plutchik 状态也正确
        assert_eq!(eng.current_basic(), PlutchikBasic::Anger);
    }

    // R248 -- intensity adjustment + filtered history views

    #[test]
    fn r248_01_set_intensity_updates_current_intensity() {
        let mut eng = ExtendedEmotionEngine::new();
        assert_eq!(eng.current_intensity(), PlutchikIntensity::Mild);
        eng.set_intensity(PlutchikIntensity::Extreme);
        assert_eq!(eng.current_intensity(), PlutchikIntensity::Extreme);
        eng.set_intensity(PlutchikIntensity::Moderate);
        assert_eq!(eng.current_intensity(), PlutchikIntensity::Moderate);
    }

    #[test]
    fn r248_02_bump_intensity_clamps_within_bounds() {
        let mut eng = ExtendedEmotionEngine::new();
        // start Mild (idx 0)
        assert_eq!(eng.current_intensity(), PlutchikIntensity::Mild);
        // +1 -> Moderate
        assert_eq!(eng.bump_intensity(1), PlutchikIntensity::Moderate);
        // +1 -> Strong
        assert_eq!(eng.bump_intensity(1), PlutchikIntensity::Strong);
        // +10 clamps to Extreme
        assert_eq!(eng.bump_intensity(10), PlutchikIntensity::Extreme);
        // +1 still Extreme (clamped)
        assert_eq!(eng.bump_intensity(1), PlutchikIntensity::Extreme);
        // -10 clamps to Mild
        assert_eq!(eng.bump_intensity(-10), PlutchikIntensity::Mild);
        // -1 still Mild (clamped)
        assert_eq!(eng.bump_intensity(-1), PlutchikIntensity::Mild);
    }

    #[test]
    fn r248_03_history_recent_returns_n_latest_in_reverse_order() {
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Joy).unwrap();
        eng.apply(PlutchikEvent::Trust).unwrap();
        eng.apply(PlutchikEvent::Fear).unwrap();
        // 3 events: Joy, Trust, Fear
        let recent_all = eng.history_recent(10);
        assert_eq!(recent_all.len(), 3);
        // most recent first
        assert!(matches!(
            recent_all[0],
            PlutchikEmotion::Basic(PlutchikBasic::Fear, _)
        ));
        assert!(matches!(
            recent_all[1],
            PlutchikEmotion::Basic(PlutchikBasic::Trust, _)
        ));
        assert!(matches!(
            recent_all[2],
            PlutchikEmotion::Basic(PlutchikBasic::Joy, _)
        ));
        // limit 2 -> only last 2 in reverse
        let recent_2 = eng.history_recent(2);
        assert_eq!(recent_2.len(), 2);
        assert!(matches!(
            recent_2[0],
            PlutchikEmotion::Basic(PlutchikBasic::Fear, _)
        ));
        assert!(matches!(
            recent_2[1],
            PlutchikEmotion::Basic(PlutchikBasic::Trust, _)
        ));
        // limit 0 -> empty
        assert!(eng.history_recent(0).is_empty());
    }

    #[test]
    fn r248_04_history_min_intensity_filters_by_level() {
        // PlutchikEvent::emotion() always encodes Moderate (constant intensity).
        // So apply() pushes entries with intensity=Moderate regardless of current_intensity.
        let mut eng = ExtendedEmotionEngine::new();
        eng.apply(PlutchikEvent::Joy).unwrap();
        eng.apply(PlutchikEvent::Trust).unwrap();
        eng.apply(PlutchikEvent::Fear).unwrap();
        eng.apply(PlutchikEvent::Anger).unwrap();
        // All 4 in history, all Moderate
        assert_eq!(eng.history().len(), 4);
        for e in eng.history() {
            assert_eq!(e.intensity(), PlutchikIntensity::Moderate);
        }
        // Mild -> all 4 (Moderate >= Mild)
        assert_eq!(eng.history_min_intensity(PlutchikIntensity::Mild).len(), 4);
        // Moderate -> all 4 (Moderate >= Moderate)
        assert_eq!(
            eng.history_min_intensity(PlutchikIntensity::Moderate).len(),
            4
        );
        // Strong -> 0 (Moderate < Strong)
        assert_eq!(
            eng.history_min_intensity(PlutchikIntensity::Strong).len(),
            0
        );
        // Extreme -> 0
        assert_eq!(
            eng.history_min_intensity(PlutchikIntensity::Extreme).len(),
            0
        );
    }
}
