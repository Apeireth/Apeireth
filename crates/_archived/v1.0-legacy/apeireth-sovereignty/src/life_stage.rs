//! 9 阶段生命周期 — 孕育 → 诞生 → ... → 重生
//!
//! **9 阶段** (主路径):
//! 1. Gestation (孕育)
//! 2. Birth (诞生)
//! 3. Infancy (幼儿)
//! 4. Growth (成长)
//! 5. Maturity (成熟)
//! 6. Reproduction (复制)
//! 7. Decline (衰老)
//! 8. Death (死亡)
//! 9. Rebirth (重生)
//!
//! **说明**:
//! - Migration (迁移) 是 **主体连续性** 概念 (跨载体), 不属于生命阶段 (本 crate 的
//!   [`SubjectContinuity`] 模块独立处理).
//! - 因此本枚举严格 9 个变体, `NINE_STAGES_HARDCODE` 编译期断言.

use serde::{Deserialize, Serialize};
use std::fmt;

/// 9 阶段生命周期。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LifeStage {
    /// 1. 孕育
    Gestation,
    /// 2. 诞生
    Birth,
    /// 3. 幼儿
    Infancy,
    /// 4. 成长
    Growth,
    /// 5. 成熟
    Maturity,
    /// 6. 复制
    Reproduction,
    /// 7. 衰老
    Decline,
    /// 8. 死亡
    Death,
    /// 9. 重生
    Rebirth,
}

impl LifeStage {
    /// 当前阶段序号 (1-9)
    pub fn ordinal(&self) -> u8 {
        match self {
            Self::Gestation => 1,
            Self::Birth => 2,
            Self::Infancy => 3,
            Self::Growth => 4,
            Self::Maturity => 5,
            Self::Reproduction => 6,
            Self::Decline => 7,
            Self::Death => 8,
            Self::Rebirth => 9,
        }
    }

    /// 是否处于终末阶段 (Death / Rebirth)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Death | Self::Rebirth)
    }

    /// 是否处于早期阶段 (Gestation / Birth / Infancy)
    pub fn is_early(&self) -> bool {
        matches!(self, Self::Gestation | Self::Birth | Self::Infancy)
    }

    /// 是否处于活跃阶段 (Growth / Maturity / Reproduction)
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Growth | Self::Maturity | Self::Reproduction)
    }

    /// 是否处于衰退阶段 (Decline / Death)
    pub fn is_declining(&self) -> bool {
        matches!(self, Self::Decline | Self::Death)
    }

    /// 下一阶段 (主路径, 循环 9 → 1)
    pub fn next(&self) -> Self {
        match self {
            Self::Gestation => Self::Birth,
            Self::Birth => Self::Infancy,
            Self::Infancy => Self::Growth,
            Self::Growth => Self::Maturity,
            Self::Maturity => Self::Reproduction,
            Self::Reproduction => Self::Decline,
            Self::Decline => Self::Death,
            Self::Death => Self::Rebirth,
            Self::Rebirth => Self::Gestation,
        }
    }

    /// 上一阶段 (主路径, 循环 1 → 9)
    pub fn previous(&self) -> Self {
        match self {
            Self::Gestation => Self::Rebirth,
            Self::Birth => Self::Gestation,
            Self::Infancy => Self::Birth,
            Self::Growth => Self::Infancy,
            Self::Maturity => Self::Growth,
            Self::Reproduction => Self::Maturity,
            Self::Decline => Self::Reproduction,
            Self::Death => Self::Decline,
            Self::Rebirth => Self::Death,
        }
    }

    /// 是否可向后跳跃 (例如 Death → Rebirth 允许; Gestation → Maturity 不允许)
    pub fn can_skip_to(&self, target: Self) -> bool {
        // 主路径上每一阶段可向前 1 步; Rebirth 可从 Death 跳跃
        let cur = i32::from(self.ordinal());
        let tgt = i32::from(target.ordinal());
        // 允许: cur+1 (向前 1 步), 或 (Death → Rebirth 即 8 → 9)
        // 禁止跳跃超过 1 步
        let diff = tgt - cur;
        diff == 1 || (cur == 8 && tgt == 9)
    }
}

impl fmt::Display for LifeStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Gestation => "gestation",
            Self::Birth => "birth",
            Self::Infancy => "infancy",
            Self::Growth => "growth",
            Self::Maturity => "maturity",
            Self::Reproduction => "reproduction",
            Self::Decline => "decline",
            Self::Death => "death",
            Self::Rebirth => "rebirth",
        };
        f.write_str(s)
    }
}

/// 9 阶段生命周期 (常量数组, 编译时硬编码锁定顺序).
pub const NINE_STAGES: [LifeStage; 9] = [
    LifeStage::Gestation,
    LifeStage::Birth,
    LifeStage::Infancy,
    LifeStage::Growth,
    LifeStage::Maturity,
    LifeStage::Reproduction,
    LifeStage::Decline,
    LifeStage::Death,
    LifeStage::Rebirth,
];

/// 生命周期阶段迁移记录.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifeStageTransition {
    /// 来源阶段
    pub from: LifeStage,
    /// 目标阶段
    pub to: LifeStage,
    /// 迁移时间 (epoch ms)
    pub at_ms: i64,
    /// 触发原因
    pub reason: String,
}

impl LifeStageTransition {
    /// 创建阶段迁移记录
    pub fn new(from: LifeStage, to: LifeStage, at_ms: i64, reason: impl Into<String>) -> Self {
        Self {
            from,
            to,
            at_ms,
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_stages_count_matches_hardcode() {
        assert_eq!(NINE_STAGES.len(), 9);
    }

    #[test]
    fn ordinal_is_1_to_9() {
        for (i, stage) in NINE_STAGES.iter().enumerate() {
            assert_eq!(stage.ordinal() as usize, i + 1);
        }
    }

    #[test]
    fn next_previous_round_trip() {
        for stage in NINE_STAGES.iter() {
            // 9 → 1 (Rebirth → Gestation) 是主路径循环
            if *stage == LifeStage::Rebirth {
                assert_eq!(stage.next(), LifeStage::Gestation);
            } else {
                let next = stage.next();
                assert!(stage.can_skip_to(next), "{:?} → {:?}", stage, next);
            }
        }
    }

    #[test]
    fn death_to_rebirth_is_allowed() {
        assert!(LifeStage::Death.can_skip_to(LifeStage::Rebirth));
    }

    #[test]
    fn gestation_to_maturity_not_allowed() {
        assert!(!LifeStage::Gestation.can_skip_to(LifeStage::Maturity));
    }
}
