//! `seven_fold_guard`: 7 重守门 v7 衔接器 (B4 6 重守门 v6 → v7 升级)
//!
//! **借鉴信息** (R126-guard-7 / 决策 #33 + 决策 #47 + 决策 #51 §1.2 P1-3):
//! - 借鉴: superpowers 234 cloned (R125-14/R125-15e 实施时已研究, per 决策 #36 §1.1)
//! - 借鉴 ID: `R126-guard-7-BORROW-obra/superpowers-2026-05-2026-08-10`
//! - 借鉴源码: `.openclaw\workspace\borrowed-repos\superpowers\`
//!
//! **设计意图** (B4 6 重守门 v6 → v7 升级):
//! - **0 改** `Governance.process` / `GovernanceOutcome` / `GovernanceStep` / `MEWG_FIVE_FOLDS_HARDCODE`
//!   (B1 24 LOCKED 入口签名 0 改严守, per 决策 #33 §2.3 + 决策 #41 §2)
//! - 提供新 wrapper `SevenFoldGuardRunner.process()` 跑 7 重:
//!   1. **守门 1** = MultiAi (Governance.process step 1)
//!   2. **守门 2** = MultiHuman (Governance.process step 2)
//!   3. **守门 3** = PhysicalMultisig (Governance.process step 3)
//!   4. **守门 4** = Reflection (Governance.process step 4)
//!   5. **守门 5** = Mewg (Governance.process step 5)
//!   6. **守门 6** = Colang DSL (colang_dsl.rs 1442 行, R125-5 实施) — **0 改**
//!   7. **守门 7** = Superpowers Skill Guard (skill_guard.rs, R126-guard-7 NEW)
//! - 守门 6 在守门 1-5 之前 (DSL 守门便宜, 先做) — 也可后置, 取决于业务
//! - 守门 7 在守门 1-6 之后 (Skill 化守门 = 总调度, 必须 6 重先跑)
//!
//! **7 重守门 v7 硬墙** (per 决策 #33 §2.3 + 决策 #52 §4):
//! - 守门 1-5 入口签名 0 改 (B1 实质保留)
//! - 守门 6 (colang_dsl.rs) 0 改 (R125-5 实施已完成, 整合 #4 commit done)
//! - 守门 7 (本模块 + skill_guard.rs) 是新模块, 内部实施可改
//! - `GovernanceOutcome` / `GovernanceStep` enum 不增 variant (避免破坏外部 match)
//!
//! **禁止**:
//! - ❌ 不修改 `Governance.process` / `GovernanceOutcome` / `GovernanceStep` 公开签名
//! - ❌ 不修改 `MEWG_FIVE_FOLDS_HARDCODE` 编译期 hardcode (5 严守, 不变 6 或 7)
//! - ❌ 不引入 PyO3 / 不调 LLM / 不引入 I/O
//! - ❌ 不引入新 crate 依赖 (仅 std + 已有 module)
//! - ❌ 不引入 `unsafe`

#![warn(missing_docs)]
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::colang_dsl::{DslOnionLayer, DslOnionVerdict};
use crate::governance::{Governance, GovernanceOutcome};
use crate::mewg::Decision;
use crate::skill_guard::{SkillGuard, SkillGuardOutcome, SkillRegistry};

// ============================================================
// 7 重守门 v7 衔接器 (B4 升 v7)
// ============================================================

/// 7 重守门 v7 衔接器 (R126-guard-7 实施, 守门 1-6 0 改 + 守门 7 NEW)
///
/// **设计**:
/// - 不修改 `Governance.process` 入口签名 (B1 入口签名 0 改严守)
/// - 提供新 wrapper `SevenFoldGuardRunner.process()` 跑 7 重:
///   - 守门 6 (Colang DSL) — 先跑, 便宜
///   - 守门 1-5 (Governance.process) — 后跑, 重
///   - 守门 7 (Superpowers Skill Guard) — 最后跑, 中心调度
///
/// **7 重守门 v7 硬墙**:
/// - 守门 1-5 入口签名 0 改 (B1 实质保留)
/// - 守门 6 (colang_dsl.rs) 0 改
/// - 守门 7 (skill_guard.rs) 是新模块, 内部实施可改
/// - `GovernanceOutcome` / `GovernanceStep` enum 不增 variant (避免破坏外部 match)
pub struct SevenFoldGuardRunner<'a> {
    /// 守门 1-5 (现有 5 重治理, 24 LOCKED 入口签名 0 改)
    pub governance: &'a Governance,
    /// 守门 6 (Colang DSL, R125-5 实施, 0 改)
    pub dsl_layer: DslOnionLayer,
    /// 守门 7 (Superpowers Skill Guard, R126-guard-7 NEW)
    pub skill_registry: SkillRegistry,
    /// 守门 7 Skill 验证器
    pub skill_guard: SkillGuard,
}

/// 7 重守门 v7 总结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SevenFoldGuardOutcome {
    /// 全部通过 (7 重都 OK)
    Approved {
        /// 守门 1-5 结果 (引用, 不破坏签名)
        governance: GovernanceOutcome,
        /// 守门 6 结果
        dsl: DslOnionVerdict,
        /// 守门 7 结果
        skill: SkillGuardOutcome,
    },
    /// 守门 6 拒绝 (DSL 不通过, 不跑守门 1-5 + 守门 7)
    BlockedAtDsl {
        /// 拒绝原因
        reason: String,
        /// 失败行
        line: Option<usize>,
    },
    /// 守门 1-5 拒绝 (DSL 通过但 governance 失败)
    BlockedAtGovernance {
        /// governance 结果
        governance: GovernanceOutcome,
        /// dsl 结果 (供参考)
        dsl: DslOnionVerdict,
        /// skill 结果 (供参考, 守门 7 0 跑)
        skill: Option<SkillGuardOutcome>,
    },
    /// 守门 7 拒绝 (守门 1-6 通过但 Skill 化守门失败, 极少见)
    BlockedAtSkill {
        /// 拒绝原因
        reason: String,
        /// governance 结果 (供参考)
        governance: GovernanceOutcome,
        /// dsl 结果 (供参考)
        dsl: DslOnionVerdict,
    },
    /// 待重审 (任一重 pending)
    PendingReview {
        /// 等待状态描述
        state: String,
        /// governance 内部状态 (若已知)
        governance: Option<GovernanceOutcome>,
        /// dsl 状态 (若已知)
        dsl: Option<DslOnionVerdict>,
        /// skill 状态 (若已知)
        skill: Option<SkillGuardOutcome>,
    },
}

impl<'a> SevenFoldGuardRunner<'a> {
    /// 新建 7 重守门衔接器 (默认 DSL 层 + 默认 Skill Registry + 默认 Skill Guard)
    pub fn new(governance: &'a Governance) -> Self {
        Self {
            governance,
            dsl_layer: DslOnionLayer::new(),
            skill_registry: SkillRegistry::new(),
            skill_guard: SkillGuard::new(),
        }
    }
    /// 自定义 DSL 洋葱层
    pub fn with_dsl_layer(mut self, layer: DslOnionLayer) -> Self {
        self.dsl_layer = layer;
        self
    }
    /// 自定义 Skill Registry
    pub fn with_skill_registry(mut self, registry: SkillRegistry) -> Self {
        self.skill_registry = registry;
        self
    }
    /// 自定义 Skill Guard
    pub fn with_skill_guard(mut self, guard: SkillGuard) -> Self {
        self.skill_guard = guard;
        self
    }

    /// 跑 7 重守门 v7 — 流程:
    /// 1. 守门 6 (Colang DSL) — 先跑, 便宜
    /// 2. 守门 1-5 (现有 Governance.process) — 后跑, 重
    /// 3. 守门 7 (Superpowers Skill Guard) — 最后跑, 中心调度
    pub async fn process(
        &self,
        decision: &Decision,
        dsl_source: &str,
    ) -> Result<SevenFoldGuardOutcome, crate::governance::GovernanceError> {
        // 守门 6: Colang DSL
        let dsl_verdict = self.dsl_layer.evaluate(dsl_source);
        match &dsl_verdict {
            DslOnionVerdict::Block { reason, line, .. } => {
                return Ok(SevenFoldGuardOutcome::BlockedAtDsl {
                    reason: reason.clone(),
                    line: *line,
                });
            }
            DslOnionVerdict::Pending { state, .. } => {
                return Ok(SevenFoldGuardOutcome::PendingReview {
                    state: state.clone(),
                    governance: None,
                    dsl: Some(dsl_verdict),
                    skill: None,
                });
            }
            DslOnionVerdict::Pass { .. } => {
                // pass → 继续跑守门 1-5
            }
        }

        // 守门 1-5: Governance.process (入口签名 0 改)
        let gov_outcome = self.governance.process(decision).await?;
        let gov_passed = matches!(gov_outcome, GovernanceOutcome::Approved { .. });

        match &gov_outcome {
            GovernanceOutcome::Approved { .. } => {
                // 守门 1-5 通过 → 跑守门 7
                // 统计 TDD RED 步骤数 (从 Skill Registry 7 Skill 累加)
                let mut tdd_red_count = 0usize;
                for id in self.skill_registry.all_ids() {
                    if let Ok(steps) = self.skill_registry.run_skill(id) {
                        tdd_red_count += steps.iter().filter(|s| s.is_tdd_red).count();
                    }
                }
                let skill_outcome = self.skill_guard.check(gov_passed, tdd_red_count);
                match &skill_outcome {
                    SkillGuardOutcome::Approved { .. } => Ok(SevenFoldGuardOutcome::Approved {
                        governance: gov_outcome,
                        dsl: dsl_verdict,
                        skill: skill_outcome,
                    }),
                    SkillGuardOutcome::Blocked { reason } => {
                        Ok(SevenFoldGuardOutcome::BlockedAtSkill {
                            reason: reason.clone(),
                            governance: gov_outcome,
                            dsl: dsl_verdict,
                        })
                    }
                    SkillGuardOutcome::PendingReview { state } => {
                        Ok(SevenFoldGuardOutcome::PendingReview {
                            state: state.clone(),
                            governance: Some(gov_outcome),
                            dsl: Some(dsl_verdict),
                            skill: Some(skill_outcome),
                        })
                    }
                }
            }
            GovernanceOutcome::Blocked { .. } => Ok(SevenFoldGuardOutcome::BlockedAtGovernance {
                governance: gov_outcome,
                dsl: dsl_verdict,
                skill: None,
            }),
            GovernanceOutcome::PendingReview { .. } => Ok(SevenFoldGuardOutcome::PendingReview {
                state: "governance pending".to_string(),
                governance: Some(gov_outcome),
                dsl: Some(dsl_verdict),
                skill: None,
            }),
        }
    }
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 7 重守门 v7 衔接器构造 verify (不需要真跑 Governance, 仅构造 verify)
    #[test]
    fn seven_fold_runner_constructs() {
        // 借用 governance 默认值 (需要一个 5 重治理实例)
        let governance = Governance::default();
        let runner = SevenFoldGuardRunner::new(&governance);
        // 守门 7 Skill Registry 7 entries 严守
        assert_eq!(runner.skill_registry.count(), 7);
        assert_eq!(runner.skill_registry.all_ids().len(), 7);
    }

    /// 守门 7 Skill 化 7 entries 严守 verify
    #[test]
    fn seven_fold_skill_registry_seven_entries() {
        let governance = Governance::default();
        let runner = SevenFoldGuardRunner::new(&governance);
        for id in SkillId::ALL {
            assert!(
                runner.skill_registry.get(id).is_some(),
                "Skill {:?} 未注册",
                id
            );
        }
    }

    /// 守门 7 严守 6-before-7 verify (守门 1-6 未跑时, 守门 7 必 Blocked)
    #[test]
    fn skill_guard_blocks_when_six_not_completed() {
        let governance = Governance::default();
        let runner = SevenFoldGuardRunner::new(&governance);
        // 6-before-7 严守: gov_passed=false 必 Blocked
        let out = runner.skill_guard.check(false, 5);
        assert!(matches!(out, SkillGuardOutcome::Blocked { .. }));
    }

    /// 守门 7 严守 TDD RED ≥ 1 verify
    #[test]
    fn skill_guard_blocks_when_tdd_red_insufficient() {
        let governance = Governance::default();
        let runner = SevenFoldGuardRunner::new(&governance);
        let out = runner.skill_guard.check(true, 0);
        assert!(matches!(out, SkillGuardOutcome::Blocked { .. }));
    }

    /// 守门 7 通过 verify (守门 1-6 跑过 + TDD RED 充足)
    #[test]
    fn skill_guard_approves_when_all_conditions_met() {
        let governance = Governance::default();
        let runner = SevenFoldGuardRunner::new(&governance);
        let out = runner.skill_guard.check(true, 5);
        assert!(matches!(out, SkillGuardOutcome::Approved { .. }));
    }
}

// re-export SkillId for convenience
pub use crate::skill_guard::SkillId;
