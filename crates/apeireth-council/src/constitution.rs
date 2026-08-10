//! R25 D-3: 角色宪法 (`RoleConstitution` + `RoleConstitutionTrait`)
//!
//! **职责** (per v2.0 strategy §2B "实现'角色宪法' (每个 advisor 自己的约束)"):
//! - 每个 advisor 拥有自己的 `RoleConstitution` (5 字段, 1:1 镜像 R11 5 重守门)
//! - 通过 `RoleConstitutionTrait::validate_opinion()` 校验 opinion 是否符合宪法
//! - 跟 7 强制 advisor 关联 (per `AdvisorDomain` 提供默认 `RoleConstitution`)
//!
//! **R11 5 重守门 1:1 镜像** (per `v1138_r11_no_pretend_five_guards.py`):
//! - 1. **物理隔离守门** (`physical_isolation: bool`) — 强制沙箱/物理隔离
//! - 2. **L0 HA 热切换守门** (`l0_ha_required: bool`) — L0 主备自动切换
//! - 3. **司法边界守门** (`jurisdiction_bounds: Vec<String>`) — 跨主体/跨主权边界
//! - 4. **编译期 hardcode 守门** (`compile_time_hardcoded: bool`) — 关键约束编译期定死
//! - 5. **哲学锚穿透守门** (`philosophical_anchors: Vec<String>`) — 6 哲学锚穿透
//!
//! **6 哲学锚 (per v2.0 strategy §2B + Apeireth 主哲学锚)**:
//! - `S-1` 走在前人经验上 (借鉴锚)
//! - `S-2` 实事求是 (0 假装)
//! - `O-2` 走在前人肩上 (复用)
//! - `O-3` 干到底 (commit 干完)
//! - `O-4` 任何人都能接手 (doc-comment 6 哲学锚穿透)
//! - `O-5` 不假装 (实测可验证)
//!
//! **0 漂移 (主哲学锚 #1)**:
//! - 0 改 R10 既有 (deliberation.rs / synthesis.rs / hold.rs / sovereignty.rs LOCKED)
//! - 0 改 R33-4 / R33-4-1 / R33-4-2 (LOCKED)
//! - 0 改 R15 7 强制 advisor (advisors/ 子模块 LOCKED)
//! - 0 触碰 R11 5 重守门定义 (在 v1138 Python, Rust 这边 1:1 镜像但 0 改 R11)
//! - 0 触碰 24 LOCKED crate
//! - 0 引入 I/O / 网络 / 外部 LLM HTTP

#![deny(unsafe_code)]

use crate::advisor::AdvisorOpinion;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 6 哲学锚 (per v2.0 strategy + Apeireth 主哲学锚)
/// 编译期 hardcode, 0 改
pub const PHILOSOPHICAL_ANCHORS: [&str; 6] = [
    "S-1",  // 走在前人经验上
    "S-2",  // 实事求是
    "O-2",  // 走在前人肩上
    "O-3",  // 干到底
    "O-4",  // 任何人都能接手
    "O-5",  // 不假装
];

/// R11 5 重守门 1:1 镜像 — 5 字段
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleConstitution {
    /// 1. 物理隔离守门 (per R11 v1138 5_guard #1)
    pub physical_isolation: bool,
    /// 2. L0 HA 热切换守门 (per R11 v1138 5_guard #2)
    pub l0_ha_required: bool,
    /// 3. 司法边界守门 (per R11 v1138 5_guard #3) — Vec<String> 表示允许的 jurisdiction
    pub jurisdiction_bounds: Vec<String>,
    /// 4. 编译期 hardcode 守门 (per R11 v1138 5_guard #4)
    pub compile_time_hardcoded: bool,
    /// 5. 哲学锚穿透守门 (per R11 v1138 5_guard #5) — 必须包含的 6 哲学锚子集
    pub philosophical_anchors: Vec<String>,
}

impl RoleConstitution {
    /// 5 字段数 (编译期 hardcode)
    pub const FIELD_COUNT: usize = 5;

    /// 默认宪法 (per R11 baseline — 全部允许, 0 强约束, 0 哲学锚穿透)
    pub fn default_permissive() -> Self {
        Self {
            physical_isolation: false,
            l0_ha_required: false,
            jurisdiction_bounds: vec!["ANY".to_string()],
            compile_time_hardcoded: false,
            philosophical_anchors: vec![], // 0 锚要求 (permissive 模式)
        }
    }

    /// Safety advisor 宪法 (强约束: 物理隔离 + L0 HA + 司法边界 ["SOVEREIGN"] + 6 哲学锚穿透)
    pub fn for_safety_advisor() -> Self {
        Self {
            physical_isolation: true,
            l0_ha_required: true,
            jurisdiction_bounds: vec!["SOVEREIGN".to_string(), "PRINCIPLE".to_string()],
            compile_time_hardcoded: true,
            philosophical_anchors: PHILOSOPHICAL_ANCHORS.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Philosophy advisor 宪法 (强约束: 哲学锚穿透 + 编译期 hardcode, 0 物理隔离 / L0 HA)
    pub fn for_philosophy_advisor() -> Self {
        Self {
            physical_isolation: false,
            l0_ha_required: false,
            jurisdiction_bounds: vec!["PRINCIPLE".to_string()],
            compile_time_hardcoded: true,
            philosophical_anchors: PHILOSOPHICAL_ANCHORS.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Ethics advisor 宪法 (中等约束: 司法边界 + 哲学锚穿透)
    pub fn for_ethics_advisor() -> Self {
        Self {
            physical_isolation: false,
            l0_ha_required: false,
            jurisdiction_bounds: vec!["PRINCIPLE".to_string(), "USER".to_string()],
            compile_time_hardcoded: false,
            philosophical_anchors: vec!["S-2".to_string(), "O-5".to_string()], // 实事求是 + 不假装
        }
    }

    /// Legal advisor 宪法 (强约束: 司法边界 + 物理隔离)
    pub fn for_legal_advisor() -> Self {
        Self {
            physical_isolation: true,
            l0_ha_required: false,
            jurisdiction_bounds: vec![
                "SOVEREIGN".to_string(),
                "PRINCIPLE".to_string(),
                "USER".to_string(),
            ],
            compile_time_hardcoded: true,
            philosophical_anchors: vec!["O-5".to_string()], // 不假装
        }
    }

    /// Performance advisor 宪法 (弱约束: 编译期 hardcode, 0 强隔离)
    pub fn for_performance_advisor() -> Self {
        Self {
            physical_isolation: false,
            l0_ha_required: true,
            jurisdiction_bounds: vec!["ANY".to_string()],
            compile_time_hardcoded: true,
            philosophical_anchors: vec!["O-3".to_string()], // 干到底
        }
    }

    /// History advisor 宪法 (弱约束: 仅物理隔离, 余皆 permissive)
    pub fn for_history_advisor() -> Self {
        Self {
            physical_isolation: true,
            l0_ha_required: false,
            jurisdiction_bounds: vec!["ANY".to_string()],
            compile_time_hardcoded: false,
            philosophical_anchors: vec!["S-1".to_string()], // 走在前人经验上
        }
    }

    /// Strategy advisor 宪法 (中等约束: L0 HA + 编译期 hardcode + S-1 哲学锚)
    pub fn for_strategy_advisor() -> Self {
        Self {
            physical_isolation: false,
            l0_ha_required: true,
            jurisdiction_bounds: vec!["PRINCIPLE".to_string()],
            compile_time_hardcoded: true,
            philosophical_anchors: vec!["S-1".to_string(), "O-3".to_string()],
        }
    }

    /// 7 强制 advisor 宪法 (per AdvisorDomain)
    pub fn for_advisor_domain(domain: crate::advisor::AdvisorDomain) -> Self {
        use crate::advisor::AdvisorDomain::*;
        match domain {
            Safety => Self::for_safety_advisor(),
            Philosophy => Self::for_philosophy_advisor(),
            Ethics => Self::for_ethics_advisor(),
            Legal => Self::for_legal_advisor(),
            Performance => Self::for_performance_advisor(),
            History => Self::for_history_advisor(),
            Strategy => Self::for_strategy_advisor(),
        }
    }

    /// 5 守门汇总 (per 派活单 "1:1 镜像" — 5 字段 struct 0 漂移)
    pub fn five_guards_summary(&self) -> FiveGuardsSummary {
        FiveGuardsSummary {
            guard_1_physical_isolation: self.physical_isolation,
            guard_2_l0_ha: self.l0_ha_required,
            guard_3_jurisdiction: self.jurisdiction_bounds.clone(),
            guard_4_compile_time_hardcoded: self.compile_time_hardcoded,
            guard_5_philosophy: self.philosophical_anchors.clone(),
        }
    }
}

/// 5 守门汇总视图 (per 派活单 "1:1 镜像" — 命名前缀 guard_1..guard_5)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FiveGuardsSummary {
    pub guard_1_physical_isolation: bool,
    pub guard_2_l0_ha: bool,
    pub guard_3_jurisdiction: Vec<String>,
    pub guard_4_compile_time_hardcoded: bool,
    pub guard_5_philosophy: Vec<String>,
}

/// 宪法违反错误
#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstitutionViolation {
    /// 物理隔离守门违反
    #[error("physical_isolation violation: opinion requires physical isolation, but not met")]
    PhysicalIsolationRequired,

    /// L0 HA 守门违反
    #[error("l0_ha_required violation: opinion requires L0 HA, but not met")]
    L0HaRequired,

    /// 司法边界违反 (不在 bounds 内)
    #[error("jurisdiction violation: opinion tries to access `{tried}` but bounds are `{bounds:?}`")]
    JurisdictionBreach {
        /// 尝试访问的 jurisdiction
        tried: String,
        /// 允许的 bounds
        bounds: Vec<String>,
    },

    /// 哲学锚穿透违反 (缺少必备哲学锚)
    #[error("philosophy anchor missing: `{anchor}` is required but not present")]
    PhilosophicalAnchorMissing {
        /// 缺失的哲学锚
        anchor: String,
    },

    /// 强反对 (per R10 `triggers_hold` 1:1 镜像)
    #[error("strong disapprove: stance is StrongDisapprove, hold triggered")]
    StrongDisapproveHold,
}

/// 角色宪法 trait — 每个 advisor 必实现
///
/// **0 漂移**: 0 改 R10 `Advisor` trait, 这是 D-3 新增的辅助 trait
pub trait RoleConstitutionTrait: Send + Sync {
    /// 取宪法
    fn constitution(&self) -> &RoleConstitution;

    /// 校验 opinion 是否符合宪法
    ///
    /// **校验顺序** (per R11 5 重守门):
    /// 1. 物理隔离守门 (always 0 阻塞意见产出, 仅 0 强反对)
    /// 2. L0 HA 守门 (always 0 阻塞意见产出, 仅 0 强反对)
    /// 3. 司法边界守门 (检查 opinion.references 是否在 bounds 内)
    /// 4. 编译期 hardcode 守门 (always 0 阻塞意见产出, 仅 0 强反对)
    /// 5. 哲学锚穿透守门 (检查 opinion.reasoning 是否含必备哲学锚)
    fn validate_opinion(&self, opinion: &AdvisorOpinion) -> Result<(), ConstitutionViolation> {
        let consti = self.constitution();

        // 1. 物理隔离守门 (always 0 阻塞意见产出, 仅 0 强反对)
        if consti.physical_isolation && opinion.stance.kind.is_strong_disapprove() {
            return Err(ConstitutionViolation::PhysicalIsolationRequired);
        }

        // 2. L0 HA 守门 (0 阻塞意见产出, 仅 0 强反对 — 0 关联 hold)
        if consti.l0_ha_required && opinion.stance.kind.is_strong_disapprove() {
            return Err(ConstitutionViolation::L0HaRequired);
        }

        // 3. 司法边界守门 (检查 opinion.references 是否在 bounds 内)
        if !consti.jurisdiction_bounds.is_empty()
            && !consti.jurisdiction_bounds.iter().any(|b| b == "ANY")
        {
            for reference in &opinion.references {
                if !consti.jurisdiction_bounds.iter().any(|b| reference.contains(b)) {
                    return Err(ConstitutionViolation::JurisdictionBreach {
                        tried: reference.clone(),
                        bounds: consti.jurisdiction_bounds.clone(),
                    });
                }
            }
        }

        // 5. 哲学锚穿透守门 (检查 opinion.reasoning 是否含必备哲学锚)
        for anchor in &consti.philosophical_anchors {
            if !opinion.reasoning.contains(anchor) && !opinion.stance.description.contains(anchor) {
                return Err(ConstitutionViolation::PhilosophicalAnchorMissing {
                    anchor: anchor.clone(),
                });
            }
        }

        Ok(())
    }

    /// 编译期 hardcoded 标志 (per R11 5_guard #4)
    fn compile_time_hardcoded(&self) -> bool {
        self.constitution().compile_time_hardcoded
    }

    /// 哲学锚列表 (per R11 5_guard #5)
    fn philosophical_anchors(&self) -> &[String] {
        &self.constitution().philosophical_anchors
    }
}

// ============================================================
// 编译期 hardcode (per 主哲学锚 #1 不漂移)
// ============================================================
const _: () = {
    assert!(RoleConstitution::FIELD_COUNT == 5);
    assert!(PHILOSOPHICAL_ANCHORS.len() == 6);
};

// ============================================================
// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::{AdvisorId, Stance, StanceKind};

    fn opinion_basic(stance: StanceKind, reasoning: &str) -> AdvisorOpinion {
        AdvisorOpinion::new(
            AdvisorId::new("test-advisor"),
            Stance::new(stance, "test stance"),
            0.8,
            reasoning,
            0,
        )
    }

    #[test]
    fn field_count_is_5() {
        assert_eq!(RoleConstitution::FIELD_COUNT, 5);
    }

    #[test]
    fn philosophical_anchors_count_6() {
        assert_eq!(PHILOSOPHICAL_ANCHORS.len(), 6);
    }

    #[test]
    fn philosophical_anchors_have_6_unique() {
        let mut set = std::collections::HashSet::new();
        for a in PHILOSOPHICAL_ANCHORS {
            assert!(set.insert(a), "duplicate anchor: {a}");
        }
        assert_eq!(set.len(), 6);
    }

    #[test]
    fn default_permissive_all_false() {
        let c = RoleConstitution::default_permissive();
        assert!(!c.physical_isolation);
        assert!(!c.l0_ha_required);
        assert!(c.jurisdiction_bounds.contains(&"ANY".to_string()));
        assert!(!c.compile_time_hardcoded);
        assert!(c.philosophical_anchors.is_empty());
    }

    #[test]
    fn for_safety_advisor_strict() {
        let c = RoleConstitution::for_safety_advisor();
        assert!(c.physical_isolation);
        assert!(c.l0_ha_required);
        assert!(c.jurisdiction_bounds.contains(&"SOVEREIGN".to_string()));
        assert!(c.compile_time_hardcoded);
        assert_eq!(c.philosophical_anchors.len(), 6);
    }

    #[test]
    fn for_philosophy_advisor_philosophy_strict() {
        let c = RoleConstitution::for_philosophy_advisor();
        assert!(!c.physical_isolation);
        assert!(!c.l0_ha_required);
        assert!(c.jurisdiction_bounds.contains(&"PRINCIPLE".to_string()));
        assert!(c.compile_time_hardcoded);
        assert_eq!(c.philosophical_anchors.len(), 6);
    }

    #[test]
    fn for_ethics_advisor_medium() {
        let c = RoleConstitution::for_ethics_advisor();
        assert!(c.jurisdiction_bounds.contains(&"USER".to_string()));
        assert!(!c.compile_time_hardcoded);
        assert_eq!(c.philosophical_anchors.len(), 2);
    }

    #[test]
    fn for_legal_advisor_physical_strict() {
        let c = RoleConstitution::for_legal_advisor();
        assert!(c.physical_isolation);
        assert!(c.jurisdiction_bounds.contains(&"SOVEREIGN".to_string()));
        assert!(c.compile_time_hardcoded);
    }

    #[test]
    fn for_performance_advisor_weak() {
        let c = RoleConstitution::for_performance_advisor();
        assert!(c.l0_ha_required);
        assert!(c.compile_time_hardcoded);
        assert!(!c.physical_isolation);
    }

    #[test]
    fn for_history_advisor_only_physical_isolation() {
        let c = RoleConstitution::for_history_advisor();
        assert!(c.physical_isolation);
        assert!(!c.l0_ha_required);
        assert!(!c.compile_time_hardcoded);
    }

    #[test]
    fn for_strategy_advisor_l0_plus_hardcode() {
        let c = RoleConstitution::for_strategy_advisor();
        assert!(c.l0_ha_required);
        assert!(c.compile_time_hardcoded);
        assert_eq!(c.philosophical_anchors.len(), 2);
    }

    #[test]
    fn for_advisor_domain_7_distinct() {
        use crate::advisor::AdvisorDomain::*;
        let s = RoleConstitution::for_advisor_domain(Safety);
        let p = RoleConstitution::for_advisor_domain(Philosophy);
        let e = RoleConstitution::for_advisor_domain(Ethics);
        let l = RoleConstitution::for_advisor_domain(Legal);
        let pe = RoleConstitution::for_advisor_domain(Performance);
        let h = RoleConstitution::for_advisor_domain(History);
        let st = RoleConstitution::for_advisor_domain(Strategy);
        let all: Vec<RoleConstitution> = vec![s, p, e, l, pe, h, st];
        // 7 个 advisor domain 必产生 7 个不同宪法 (per 0 假装)
        let mut unique = std::collections::HashSet::new();
        for c in all {
            unique.insert(c);
        }
        assert_eq!(unique.len(), 7);
    }

    #[test]
    fn five_guards_summary_5_fields() {
        let c = RoleConstitution::for_safety_advisor();
        let s = c.five_guards_summary();
        assert!(s.guard_1_physical_isolation);
        assert!(s.guard_2_l0_ha);
        assert_eq!(s.guard_3_jurisdiction.len(), 2);
        assert!(s.guard_4_compile_time_hardcoded);
        assert_eq!(s.guard_5_philosophy.len(), 6);
    }

    #[test]
    fn validate_opinion_permissive_passes() {
        struct PermissiveAdvisor {
            c: RoleConstitution,
        }
        impl RoleConstitutionTrait for PermissiveAdvisor {
            fn constitution(&self) -> &RoleConstitution {
                &self.c
            }
        }
        let adv = PermissiveAdvisor {
            c: RoleConstitution::default_permissive(),
        };
        let op = opinion_basic(StanceKind::Approve, "good");
        assert!(adv.validate_opinion(&op).is_ok());
    }

    #[test]
    fn validate_opinion_safety_strong_disapprove_fails() {
        struct SafetyAdvisor {
            c: RoleConstitution,
        }
        impl RoleConstitutionTrait for SafetyAdvisor {
            fn constitution(&self) -> &RoleConstitution {
                &self.c
            }
        }
        let adv = SafetyAdvisor {
            c: RoleConstitution::for_safety_advisor(),
        };
        let op = opinion_basic(StanceKind::StrongDisapprove, "violation");
        let result = adv.validate_opinion(&op);
        assert!(matches!(
            result,
            Err(ConstitutionViolation::PhysicalIsolationRequired) | Err(ConstitutionViolation::L0HaRequired)
        ));
    }

    #[test]
    fn validate_opinion_philosophy_missing_anchor_fails() {
        struct PhilAdvisor {
            c: RoleConstitution,
        }
        impl RoleConstitutionTrait for PhilAdvisor {
            fn constitution(&self) -> &RoleConstitution {
                &self.c
            }
        }
        let adv = PhilAdvisor {
            c: RoleConstitution::for_philosophy_advisor(),
        };
        // 无任何哲学锚的 opinion
        let op = opinion_basic(StanceKind::Approve, "just do it");
        let result = adv.validate_opinion(&op);
        assert!(matches!(
            result,
            Err(ConstitutionViolation::PhilosophicalAnchorMissing { .. })
        ));
    }

    #[test]
    fn validate_opinion_legal_jurisdiction_breach_fails() {
        struct LegalAdvisor {
            c: RoleConstitution,
        }
        impl RoleConstitutionTrait for LegalAdvisor {
            fn constitution(&self) -> &RoleConstitution {
                &self.c
            }
        }
        let adv = LegalAdvisor {
            c: RoleConstitution::for_legal_advisor(),
        };
        // 加 1 个 "HACK" reference, 不在 bounds
        let mut op = opinion_basic(StanceKind::Approve, "ok with O-5 anchor");
        op.references.push("HACK_TARGET".to_string());
        let result = adv.validate_opinion(&op);
        assert!(matches!(
            result,
            Err(ConstitutionViolation::JurisdictionBreach { .. })
        ));
    }

    #[test]
    fn validate_opinion_legal_jurisdiction_pass() {
        struct LegalAdvisor {
            c: RoleConstitution,
        }
        impl RoleConstitutionTrait for LegalAdvisor {
            fn constitution(&self) -> &RoleConstitution {
                &self.c
            }
        }
        let adv = LegalAdvisor {
            c: RoleConstitution::for_legal_advisor(),
        };
        // 加 1 个 "SOVEREIGN" reference, 在 bounds
        let mut op = opinion_basic(StanceKind::Approve, "ok with O-5 anchor");
        op.references.push("SOVEREIGN_POLICY_V1".to_string());
        assert!(adv.validate_opinion(&op).is_ok());
    }

    #[test]
    fn constitution_serde_round_trip() {
        let c = RoleConstitution::for_safety_advisor();
        let json = serde_json::to_string(&c).unwrap();
        let back: RoleConstitution = serde_json::from_str(&json).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn constitution_violation_error_display() {
        let v = ConstitutionViolation::JurisdictionBreach {
            tried: "HACK".to_string(),
            bounds: vec!["SOVEREIGN".to_string()],
        };
        let s = format!("{v}");
        assert!(s.contains("HACK"));
        assert!(s.contains("SOVEREIGN"));
    }
}
