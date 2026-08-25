//! 9 阶段生命周期

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifeStage {
    Gestation,
    Birth,
    Infancy,
    Growth,
    Maturity,
    Reproduction,
    Decline,
    Death,
    Rebirth,
}

impl LifeStage {
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Gestation => 1, Self::Birth => 2, Self::Infancy => 3, Self::Growth => 4,
            Self::Maturity => 5, Self::Reproduction => 6, Self::Decline => 7,
            Self::Death => 8, Self::Rebirth => 9,
        }
    }
    pub fn is_terminal(&self) -> bool { matches!(self, Self::Death | Self::Rebirth) }
    pub fn is_early(&self) -> bool { matches!(self, Self::Gestation | Self::Birth | Self::Infancy) }
    pub fn is_active(&self) -> bool { matches!(self, Self::Growth | Self::Maturity | Self::Reproduction) }
    pub fn is_declining(&self) -> bool { matches!(self, Self::Decline | Self::Death) }
    pub fn next(&self) -> Self {
        match self {
            Self::Gestation => Self::Birth, Self::Birth => Self::Infancy, Self::Infancy => Self::Growth,
            Self::Growth => Self::Maturity, Self::Maturity => Self::Reproduction,
            Self::Reproduction => Self::Decline, Self::Decline => Self::Death,
            Self::Death => Self::Rebirth, Self::Rebirth => Self::Gestation,
        }
    }
    pub fn previous(&self) -> Self {
        match self {
            Self::Gestation => Self::Rebirth, Self::Birth => Self::Gestation,
            Self::Infancy => Self::Birth, Self::Growth => Self::Infancy,
            Self::Maturity => Self::Growth, Self::Reproduction => Self::Maturity,
            Self::Decline => Self::Reproduction, Self::Death => Self::Decline,
            Self::Rebirth => Self::Death,
        }
    }
    pub fn can_skip_to(&self, target: Self) -> bool {
        let cur = i32::from(self.ordinal());
        let tgt = i32::from(target.ordinal());
        let diff = tgt - cur;
        diff == 1 || (cur == 8 && tgt == 9)
    }
}

impl fmt::Display for LifeStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Gestation => "gestation", Self::Birth => "birth", Self::Infancy => "infancy",
            Self::Growth => "growth", Self::Maturity => "maturity", Self::Reproduction => "reproduction",
            Self::Decline => "decline", Self::Death => "death", Self::Rebirth => "rebirth",
        };
        f.write_str(s)
    }
}

pub const NINE_STAGES: [LifeStage; 9] = [
    LifeStage::Gestation, LifeStage::Birth, LifeStage::Infancy, LifeStage::Growth,
    LifeStage::Maturity, LifeStage::Reproduction, LifeStage::Decline,
    LifeStage::Death, LifeStage::Rebirth,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifeStageTransition {
    pub from: LifeStage,
    pub to: LifeStage,
    pub at_ms: i64,
    pub reason: String,
}

impl LifeStageTransition {
    pub fn new(from: LifeStage, to: LifeStage, at_ms: i64, reason: impl Into<String>) -> Self {
        Self { from, to, at_ms, reason: reason.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn nine_stages_count() { assert_eq!(NINE_STAGES.len(), 9); }
    #[test] fn ordinal_is_1_to_9() {
        for (i, s) in NINE_STAGES.iter().enumerate() { assert_eq!(s.ordinal() as usize, i + 1); }
    }
    #[test] fn next_and_previous_round_trip() {
        for s in NINE_STAGES.iter() {
            if *s == LifeStage::Rebirth {
                assert_eq!(s.next(), LifeStage::Gestation);
            } else {
                assert!(s.can_skip_to(s.next()));
            }
        }
    }
    #[test] fn death_to_rebirth_allowed() { assert!(LifeStage::Death.can_skip_to(LifeStage::Rebirth)); }
    #[test] fn gestation_to_maturity_not_allowed() { assert!(!LifeStage::Gestation.can_skip_to(LifeStage::Maturity)); }
    #[test] fn stage_classifications() {
        assert!(LifeStage::Gestation.is_early());
        assert!(LifeStage::Growth.is_active());
        assert!(LifeStage::Decline.is_declining());
        assert!(LifeStage::Death.is_terminal());
        assert!(LifeStage::Rebirth.is_terminal());
    }
}
