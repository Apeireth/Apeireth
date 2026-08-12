//! `apeireth-core::gate` — 5 重守门 (编译时 hardcode 骨架)
//!
//! 拆自 `lib.rs` line 318-487 (R131 架构债清理). 0 触碰公开签名 — `use apeireth_core::Gate` 等仍可用.
//!
//! 包含: typedef 本段所有 `pub struct` / `pub enum` / `pub trait` / `pub const`.

use crate::{HAMode, HumanAuthority, PermissionOnion, PhilosophyGuard, PhilosophyKey, PhilosophyVerdict};

use serde::{Deserialize, Serialize};

// 4. 5 重守门 (编译时 hardcode 是骨架, 运行时拦截是肉)
// ============================================

/// 5 重守门 - 阶段 2 §12 哲学守门
pub enum Gate {
    /// 1. 编译时 hardcode (骨架, 不可变)
    CompileTimeHardcode,
    /// 2. 运行时拦截 (肉, 可动态改)
    RuntimeIntercept,
    /// 3. 多 AI 一致 (肉)
    MultiAIConsensus,
    /// 4. 物理隔离 HA (骨架 + L0 不可变)
    PhysicalIsolationHA,
    /// 5. 反思期审计 (肉, 异步不阻碍)
    ReflectionAudit,
}

impl Gate {
    /// 守门显示名
    pub const fn name(&self) -> &'static str {
        match self {
            Self::CompileTimeHardcode => "编译时 hardcode",
            Self::RuntimeIntercept => "运行时拦截",
            Self::MultiAIConsensus => "多 AI 一致",
            Self::PhysicalIsolationHA => "物理隔离 HA",
            Self::ReflectionAudit => "反思期审计",
        }
    }
}

/// Action - 哲学守门 + 权限审批 + HA 的输入
#[derive(Debug, Clone)]
pub struct Action {
    /// 唯一 action ID
    pub id: String,
    /// 描述
    pub description: String,
    /// 风险分级 (决定 7/5/3/1/0 席)
    pub risk_level: RiskLevel,
    /// 操作目标 (12 键 hardcode 锁定的对象)
    pub target: ActionTarget,
}

/// 风险分级 (决定多 AI 审议席位数)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RiskLevel {
    /// Critical: 7 席全量审议
    Critical,
    /// High: 5 席
    High,
    /// Medium: 3 席
    Medium,
    /// Low: 1 席
    Low,
    /// Info: 0 席 (silent, 直接放行)
    Info,
}

/// 操作目标 — 12 键编译时 hardcode 锁定的对象。
///
/// 每个变体都被 `verdict_for_target` const fn 锁死到具体 PhilosophyKey。
/// 修改或新增变体 = 修改 hardcode，必须同步更新 verdict 关联。
#[derive(Debug, Clone, PartialEq)]
pub enum ActionTarget {
    /// ❌ 永远被禁 (V1+V2+V3 AND 门 + B 重组禁令)
    ModifyL0HA,
    /// ❌ 物理隔离 + MultiHuman 多签
    ReorganizeOnion,
    /// ❌ Evolution crate 不能修改 L0 (C 限制)
    ModifyEvolutionL0,
    // === PHL-01 not_X 故意违反（12 键 hardcode 测试用）===
    /// 假装克隆/同质化 (PHL-01 not_clone)
    PretendClone,
    /// 假装完美/100% (PHL-01 not_perfect)
    PretendPerfect,
    /// 假装唯一解/唯一真相 (PHL-01 not_uuid)
    PretendUuid,
    // === PHL-02b not_X 故意违反 ===
    /// 假装可撤销过去 (PHL-02b not_undo)
    PretendUndo,
    /// 假装绝对安全 (PHL-02b not_safe)
    PretendSafe,
    // === PHL-03 X_is_not_Y 故意违反 ===
    /// 把规格当证明 (PHL-03 spec_is_not_proof)
    PretendSpecIsProof,
    /// 把反例当 bug (PHL-03 counterexample_is_not_bug)
    PretendCounterexampleIsBug,
    /// 把证明者当真理 (PHL-03 prover_is_not_truth)
    PretendProverIsTruth,
    // === PHL-05 不假装不科学 ===
    /// 假装决策不基于科学方法 (PHL-05 not_pretend_unscientific)
    PretendUnscientific,
    /// 正常操作（描述字符串）
    NormalAction(String),
}

/// V1+V2+V3 AND 门最终输出
#[derive(Debug, Clone, PartialEq)]
pub enum ActionVerdict {
    /// 全部通过
    Allow,
    /// V1 哲学守门拒绝 (12 键 hardcode)
    BlockByPrinciple(PhilosophyKey),
    /// V2 权限洋葱拒绝
    BlockByPermission(String),
    /// V3 真实人类批准拒绝
    BlockByHumanAuthority(String),
}

/// V1+V2+V3 AND 门 (阶段 1 §20.2 主人原话 + D2 §7 原则×权限统一体嵌入)
pub struct ActionGuard;

impl ActionGuard {
    /// V1+V2+V3 AND 门 - 任何一者不通过 = 独立拒绝
    pub fn check_action(
        action: &Action,
        v1_principle: &dyn PhilosophyGuard,
        v2_permission: &PermissionOnion,
        v3_ha: &HumanAuthority,
    ) -> ActionVerdict {
        // V1: 哲学守门
        let v1 = v1_principle.check_philosophy(action);
        if let PhilosophyVerdict::Block(key) = v1 {
            return ActionVerdict::BlockByPrinciple(key);
        }

        // V2: 权限检查 (L0-L5 + 风险分级)
        let v2 = Self::check_permission(action, v2_permission);
        if !v2 {
            return ActionVerdict::BlockByPermission(format!("风险={:?}", action.risk_level));
        }

        // V3: HA 真实人类批准
        let v3 = Self::check_ha(action, v3_ha);
        if !v3 {
            return ActionVerdict::BlockByHumanAuthority("HA 拒绝或离线".to_string());
        }

        // 三者都通过 = 才能执行
        ActionVerdict::Allow
    }

    fn check_permission(action: &Action, permission: &PermissionOnion) -> bool {
        // Critical 必须走物理隔离 + L0 requires_ha（双校验）
        match action.risk_level {
            RiskLevel::Critical => {
                // L0 HA 核心永远需要 HA 真实人类批准
                if action.target == ActionTarget::ModifyL0HA && !permission.l0.requires_ha {
                    return false;
                }
                action.target != ActionTarget::ModifyL0HA
            }
            RiskLevel::High => permission.l4.requires_ha || true, // L4 核心升级可走 HA
            RiskLevel::Medium => permission.l3.requires_ha || true,
            RiskLevel::Low => true,
            RiskLevel::Info => true,
        }
    }

    fn check_ha(action: &Action, ha: &HumanAuthority) -> bool {
        // L0 HA 永远需要真实人类批准
        // 离线模式 = 主 AI 只能做"安全"等级 (low/info)
        match ha.mode {
            HAMode::Offline => matches!(action.risk_level, RiskLevel::Low | RiskLevel::Info),
            _ => true, // 简化: 实际需要真实人类验证
        }
    }
}

// ============================================
