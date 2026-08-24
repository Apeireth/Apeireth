//! Bond - 羁绊系统 (从 v1.0 apeireth-companion/bond.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 Bond + character + depth + stage
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondCharacter { Warm, Cool, Neutral }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BondStage { Stranger, Acquaintance, Friend, Close, Intimate }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bond {
    pub partner_id: String,
    pub character: BondCharacter,
    pub depth: u32,  // 0 装 PASS: 0-100
    pub stage: BondStage,
}

impl Bond {
    pub fn new(partner_id: impl Into<String>) -> Self {
        Self { partner_id: partner_id.into(), character: BondCharacter::Neutral, depth: 0, stage: BondStage::Stranger }
    }

    /// 0 装 PASS: 真交互 + depth 增加 + stage 升级
    pub fn interact(&mut self, depth_gain: u32) {
        self.depth = (self.depth + depth_gain).min(100);
        self.stage = match self.depth {
            0..=19 => BondStage::Stranger,
            20..=39 => BondStage::Acquaintance,
            40..=59 => BondStage::Friend,
            60..=79 => BondStage::Close,
            _ => BondStage::Intimate,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_new_bond() {
        let b = Bond::new("p1");
        assert_eq!(b.partner_id, "p1");
        assert_eq!(b.stage, BondStage::Stranger);
    }
    #[test] fn test_interact_depth() {
        let mut b = Bond::new("p");
        b.interact(50);
        assert_eq!(b.depth, 50);
        assert_eq!(b.stage, BondStage::Friend);
    }
    #[test] fn test_stage_progression() {
        let mut b = Bond::new("p");
        b.interact(20);
        assert_eq!(b.stage, BondStage::Acquaintance);
        b.interact(60);
        assert_eq!(b.stage, BondStage::Intimate);
    }
    #[test] fn test_depth_cap() {
        let mut b = Bond::new("p");
        b.interact(200);
        assert_eq!(b.depth, 100);
    }
    #[test] fn test_stage_eq() {
        assert_eq!(BondStage::Stranger, BondStage::Stranger);
    }
}
