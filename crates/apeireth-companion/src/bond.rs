//! 关系本身 —— Bond (Per 哲学: 关系 = 可成长的, 跨 session 的, 有情感的)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 关系阶段 —— 关系在生命周期里的位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondStage {
    /// 初始接触
    Initial,
    /// 熟悉中
    Familiar,
    /// 信任
    Trusted,
    /// 亲密
    Intimate,
    /// 长期 (5 年级)
    LongTerm,
    /// 暂停 (用户离开一段时间)
    Paused,
    /// 终止 (用户明确表示终结)
    Ended,
}

impl BondStage {
    pub const ALL: [BondStage; 7] = [
        Self::Initial,
        Self::Familiar,
        Self::Trusted,
        Self::Intimate,
        Self::LongTerm,
        Self::Paused,
        Self::Ended,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Initial => "initial",
            Self::Familiar => "familiar",
            Self::Trusted => "trusted",
            Self::Intimate => "intimate",
            Self::LongTerm => "long_term",
            Self::Paused => "paused",
            Self::Ended => "ended",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Ended)
    }
}

/// 关系深度 —— 0.0 ~ 1.0 的连续度 (per principle onion "配额曲线" 模式)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BondDepth(pub f64);

impl BondDepth {
    pub const ZERO: BondDepth = BondDepth(0.0);
    pub const ONE: BondDepth = BondDepth(1.0);

    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    pub fn value(self) -> f64 { self.0 }
}

impl Default for BondDepth {
    fn default() -> Self { Self::ZERO }
}

impl std::fmt::Display for BondDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// 关系特征 —— 关系自身的"性格"
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BondCharacter {
    /// 互依度 (互相依赖的程度)
    pub interdependency: f64,
    /// 韧性 (冲突中的恢复力)
    pub resilience: f64,
    /// 共鸣 (情感同步频率)
    pub resonance: f64,
    /// 创造性 (一起做新事的冲动)
    pub creativity: f64,
    /// 信任 (per principle onion E 层)
    pub trust: f64,
}

impl BondCharacter {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注入情感 (per consciousness bridge, via Plutchik 状态)
    pub fn apply_emotion(&mut self, joy: f64, trust: f64, fear: f64, surprise: f64, sadness: f64, disgust: f64, anger: f64, anticipation: f64) {
        self.resonance = (self.resonance * 0.7 + (joy + trust + anticipation) * 0.1).clamp(0.0, 1.0);
        self.trust = (self.trust * 0.8 + trust * 0.2).clamp(0.0, 1.0);
        let _ = (fear, surprise, sadness, disgust, anger);
    }

    pub fn serialize(&self) -> std::collections::HashMap<&'static str, f64> {
        let mut m = std::collections::HashMap::new();
        m.insert("interdependency", self.interdependency);
        m.insert("resilience", self.resilience);
        m.insert("resonance", self.resonance);
        m.insert("creativity", self.creativity);
        m.insert("trust", self.trust);
        m
    }
}

/// 关系 —— Bond complete
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bond {
    stage: BondStage,
    depth: BondDepth,
    character: BondCharacter,
    formed_at: DateTime<Utc>,
    last_evolved: DateTime<Utc>,
}

impl Bond {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            stage: BondStage::Initial,
            depth: BondDepth::ZERO,
            character: BondCharacter::new(),
            formed_at: now,
            last_evolved: now,
        }
    }

    pub fn stage(&self) -> BondStage { self.stage }
    pub fn depth(&self) -> BondDepth { self.depth }
    pub fn character(&self) -> &BondCharacter { &self.character }
    pub fn character_mut(&mut self) -> &mut BondCharacter { &mut self.character }
    pub fn formed_at(&self) -> DateTime<Utc> { self.formed_at }
    pub fn last_evolved(&self) -> DateTime<Utc> { self.last_evolved }

    pub fn evolve(&mut self, new_stage: BondStage, delta_depth: f64) {
        self.stage = new_stage;
        self.depth = BondDepth::new(self.depth.value() + delta_depth);
        self.last_evolved = Utc::now();
    }

    pub fn apply_emotion(&mut self, joy: f64, trust: f64, fear: f64, surprise: f64, sadness: f64, disgust: f64, anger: f64, anticipation: f64) {
        self.character.apply_emotion(joy, trust, fear, surprise, sadness, disgust, anger, anticipation);
    }
}

impl Default for Bond {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bond_starts_initial() {
        let b = Bond::new();
        assert_eq!(b.stage(), BondStage::Initial);
        assert_eq!(b.depth(), BondDepth::ZERO);
    }

    #[test]
    fn bond_evolve_advances() {
        let mut b = Bond::new();
        b.evolve(BondStage::Trusted, 0.5);
        assert_eq!(b.stage(), BondStage::Trusted);
        assert!((b.depth().value() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn bond_depth_clamps() {
        let d = BondDepth::new(2.0);
        assert_eq!(d.value(), 1.0);
        let d = BondDepth::new(-1.0);
        assert_eq!(d.value(), 0.0);
    }

    #[test]
    fn bond_terminal_check() {
        assert!(BondStage::Ended.is_terminal());
        assert!(!BondStage::Initial.is_terminal());
    }
}
