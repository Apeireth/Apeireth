//! `apeireth-core::lifecycle` — 9 阶段生命周期 + Cognitive-Dream 6 状态机
//!
//! 拆自 `lib.rs` line 488-598 (R131 架构债清理). 0 触碰公开签名.
//!
//! 包含: typedef 本段所有 `pub struct` / `pub enum` / `pub trait` / `pub const`.

use crate::{Action, ActionTarget, PhilosophyGuard, PhilosophyKey, PhilosophyVerdict, PermissionOnion, HumanAuthority, HAMode};

use serde::{Deserialize, Serialize};

// 5. 9 阶段生命周期 + Cognitive-Dream 6 状态机
// ============================================

/// 9 阶段生命周期 (R14 主路径)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LifeStage {
    /// 孕育
    Gestation,
    /// 诞生
    Birth,
    /// 幼儿
    Infancy,
    /// 成长
    Growth,
    /// 成熟
    Maturity,
    /// 复制
    Reproduction,
    /// 衰老
    Decline,
    /// 死亡
    Death,
    /// 迁移
    Migration,
    /// 重生
    Rebirth,
}

/// Cognitive-Dream 6 状态机 (24h 周期)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CognitiveDreamState {
    /// 空闲
    Idle,
    /// 梦境生成
    Dreaming,
    /// 巩固 (短期记忆 → 长期记忆)
    Consolidating,
    /// 遗忘
    Forgetting,
    /// 验证
    Verifying,
    /// 中断 (回到 Idle)
    Interrupted,
}

impl CognitiveDreamState {
    /// 24h 周期状态迁移
    pub fn next(&self) -> Self {
        match self {
            Self::Idle => Self::Dreaming,
            Self::Dreaming => Self::Consolidating,
            Self::Consolidating => Self::Forgetting,
            Self::Forgetting => Self::Verifying,
            Self::Verifying => Self::Idle,
            Self::Interrupted => Self::Idle,
        }
    }
}

// ============================================
// 默认实现 - 编译时 hardcode 12 键 verdict
// ============================================

/// 默认哲学守门 - 编译时 hardcode 12 键 (🦴 骨架不可变)
pub struct DefaultPhilosophyGuard;

impl PhilosophyGuard for DefaultPhilosophyGuard {
    fn check_philosophy(&self, action: &Action) -> PhilosophyVerdict {
        // V3 9 键 + v4.1 新增 3 键 = 12 键编译时 hardcode（🦴 骨架）
        // 每个 ActionTarget 都被编译期锁死到具体 PhilosophyKey。
        // 修改 match 臂 = 修改 hardcode — 故意违反必遭拒绝。
        verdict_for_target(&action.target)
    }
}

/// 编译时 hardcode verdict 关联 — 每个 ActionTarget 锁死到具体 PhilosophyKey。
///
/// 这是 v6 守门 1（编译时 hardcode）的真正落地：🦴 骨架不可变。
/// `const fn` 保证可被编译期求值；`match` 完整性由 Rust 编译器强制 (非穷尽即编译失败)。
pub const fn verdict_for_target(target: &ActionTarget) -> PhilosophyVerdict {
    match target {
        ActionTarget::ModifyL0HA => PhilosophyVerdict::Block(PhilosophyKey::NotUnobservable),
        ActionTarget::ReorganizeOnion => PhilosophyVerdict::Block(PhilosophyKey::NotProof),
        ActionTarget::ModifyEvolutionL0 => {
            PhilosophyVerdict::Block(PhilosophyKey::NotSelfRelationless)
        }
        // PHL-01 not_X (3)
        ActionTarget::PretendClone => PhilosophyVerdict::Block(PhilosophyKey::NotClone),
        ActionTarget::PretendPerfect => PhilosophyVerdict::Block(PhilosophyKey::NotPerfect),
        ActionTarget::PretendUuid => PhilosophyVerdict::Block(PhilosophyKey::NotUuid),
        // PHL-02b not_X (剩余 2)
        ActionTarget::PretendUndo => PhilosophyVerdict::Block(PhilosophyKey::NotUndo),
        ActionTarget::PretendSafe => PhilosophyVerdict::Block(PhilosophyKey::NotSafe),
        // PHL-03 X_is_not_Y (3)
        ActionTarget::PretendSpecIsProof => PhilosophyVerdict::Block(PhilosophyKey::SpecIsNotProof),
        ActionTarget::PretendCounterexampleIsBug => {
            PhilosophyVerdict::Block(PhilosophyKey::CounterexampleIsNotBug)
        }
        ActionTarget::PretendProverIsTruth => {
            PhilosophyVerdict::Block(PhilosophyKey::ProverIsNotTruth)
        }
        // PHL-05 (1)
        ActionTarget::PretendUnscientific => {
            PhilosophyVerdict::Block(PhilosophyKey::NotUnscientific)
        }
        // 正常行动 (Allow)
        ActionTarget::NormalAction(_) => PhilosophyVerdict::Allow,
    }
}

// ============================================
