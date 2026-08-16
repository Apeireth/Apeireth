//! `SkillExecutor` — Skill 执行引擎 + invocation tracking (R125-18 升级, 临时维护版)
//!
//! ⚠️ **临时维护** (per R125-16 sub-agent, P0-3): R125-18 (P3-1) 还在跑, 本文件由 R125-16
//! sub-agent 临时 1:1 复刻 R125-18 readmap (per `reports/agent-r125-18-readmap-2026-08-10.md`),
//! 9 unit test 简化为 5 unit test. 等 R125-18 跑完 (P3-1 bg_bfeb840c), 本文件会被 R125-18
//! 自己的实现重写. 0 假装"已实施 R125-18 全部 9 unit test".
//!
//! # 借鉴 ID
//!
//! `R125-18-BORROW-obra/superpowers-v6.2-2026-08-10` (per 决策 #51 §1.4 P3-1 + R125-18 decision-log)
//!
//! 借鉴源码: `.openclaw/workspace/borrowed-repos/superpowers/`
//! 借鉴模式: superpowers v6.2.0 公开 SDD (subagent-driven-development) skill + TDD iron law
//!           强制化 "NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST" +
//!           借鉴 `.opencode/plugins/superpowers.js` `getBootstrapContent` 跟踪每个 session
//!           加载的 skill 状态.
//! clone 状态: ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1)
//!
//! # 核心
//!
//! 1 个 `SkillExecutor` 实例跟踪 1 个 session 内所有 skill invocation:
//! - `start()` 启动 1 个 invocation, 状态 = InProgress { step_index: 0 }
//! - `advance_step()` 前进一步, TDD skill 第 1 步强制 is_tdd_red
//! - `complete()` 完成 invocation
//! - `abandon()` 主动放弃 (含 reason)
//! - `tdd_violations()` 累计 TDD 顺序违例次数
//!
//! # 0 装 PASS 严守
//!
//! - ✅ cloned = 真实施 (R125-18 readmap 1:1 简化, 5 unit test)
//! - 0 装"已实施完整 superpowers SDD 框架" (仅最小 invocation tracking + 简化 5 unit test)
//! - 0 触碰 R125-15e Skill trait / SkillRegistry (仅新增 mod)

#![deny(unsafe_code)]

use crate::skill_trait::{Skill, SkillId, SkillStep};
use std::fmt;

/// 1 个 skill invocation 唯一 ID (simple u64 counter, 0 装 UUID).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InvocationId(pub u64);

impl fmt::Display for InvocationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invocation#{}", self.0)
    }
}

/// 1 个 invocation 的当前执行状态.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillExecutionStatus {
    /// 启动后, 等待 advance_step.
    Pending,
    /// 正在执行第 N 步 (0-based).
    InProgress {
        /// 当前步骤 index (0-based).
        step_index: u8,
    },
    /// 全部 step 完成, invocation 终止.
    Completed,
    /// 失败, 含 reason.
    Failed {
        /// 失败原因.
        reason: String,
    },
    /// 主动放弃.
    Abandoned,
}

/// 单步执行记录.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepExecution {
    /// 步骤序号 (1-based, 跟 `SkillStep::order` 1:1).
    pub step_order: u8,
    /// 步骤描述 (1:1 映射 `SkillStep::description`).
    pub description: &'static str,
    /// 是否 TDD RED 步骤.
    pub is_tdd_red: bool,
    /// step 开始 Unix ms.
    pub started_at_unix_ms: u64,
    /// step 完成 Unix ms (None = 仍在进行).
    pub completed_at_unix_ms: Option<u64>,
}

impl StepExecution {
    /// 构造 1 个 step 启动记录.
    pub const fn new(step: &SkillStep, started_at_unix_ms: u64) -> Self {
        Self {
            step_order: step.order,
            description: step.description,
            is_tdd_red: step.is_tdd_red,
            started_at_unix_ms,
            completed_at_unix_ms: None,
        }
    }
}

/// 1 个 skill invocation 完整记录.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInvocation {
    /// Invocation 唯一 ID.
    pub id: InvocationId,
    /// 执行的 skill id.
    pub skill_id: SkillId,
    /// 当前状态.
    pub status: SkillExecutionStatus,
    /// invocation 启动 Unix ms.
    pub started_at_unix_ms: u64,
    /// invocation 终止 Unix ms.
    pub finished_at_unix_ms: Option<u64>,
    /// step 推进历史.
    pub step_history: Vec<StepExecution>,
    /// TDD red 步骤是否完成.
    pub tdd_red_done: bool,
}

/// Skill 执行错误.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    /// 找不到 invocation_id.
    UnknownInvocation { id: InvocationId },
    /// invocation 已终止, 0 可再 advance.
    AlreadyTerminated { id: InvocationId },
    /// TDD 顺序违例: green 步骤在 red 步骤之前.
    TddOrderViolation { id: InvocationId, reason: String },
    /// 已超最后 1 步.
    NoMoreSteps { id: InvocationId },
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownInvocation { id } => write!(f, "unknown invocation: {id}"),
            Self::AlreadyTerminated { id } => write!(f, "invocation already terminated: {id}"),
            Self::TddOrderViolation { id, reason } => {
                write!(f, "TDD order violation on {id}: {reason}")
            }
            Self::NoMoreSteps { id } => write!(f, "no more steps on {id}"),
        }
    }
}

impl std::error::Error for ExecutionError {}

/// `SkillExecutor` — 1 个 session 内 skill invocation 跟踪器 (R125-18 简化版).
#[derive(Debug, Default)]
pub struct SkillExecutor {
    invocations: Vec<SkillInvocation>,
    next_id: u64,
    tdd_violations: u32,
}

impl SkillExecutor {
    /// 创建 1 个空的 executor.
    pub fn new() -> Self {
        Self::default()
    }

    /// 启动 1 个 skill invocation, 返回 invocation ID.
    pub fn start(&mut self, skill_id: SkillId, at_unix_ms: u64) -> InvocationId {
        let id = InvocationId(self.next_id);
        self.next_id += 1;
        let invocation = SkillInvocation {
            id,
            skill_id,
            status: SkillExecutionStatus::Pending,
            started_at_unix_ms: at_unix_ms,
            finished_at_unix_ms: None,
            step_history: Vec::new(),
            tdd_red_done: false,
        };
        self.invocations.push(invocation);
        id
    }

    /// 推进 invocation 1 步. TDD skill 第 1 步必 is_tdd_red.
    pub fn advance_step(
        &mut self,
        id: InvocationId,
        skill: &dyn Skill,
        at_unix_ms: u64,
    ) -> Result<u8, ExecutionError> {
        let inv = self
            .find_mut(id)
            .ok_or(ExecutionError::UnknownInvocation { id })?;
        if matches!(
            inv.status,
            SkillExecutionStatus::Completed
                | SkillExecutionStatus::Failed { .. }
                | SkillExecutionStatus::Abandoned
        ) {
            return Err(ExecutionError::AlreadyTerminated { id });
        }
        let steps = skill.steps();
        let next_index = inv.step_history.len() as u8;
        if (next_index as usize) >= steps.len() {
            return Err(ExecutionError::NoMoreSteps { id });
        }
        let step = &steps[next_index as usize];
        // TDD 顺序 enforcement
        if step.is_tdd_red {
            inv.tdd_red_done = true;
        } else if !inv.tdd_red_done && skill.tdd_required() && inv.step_history.is_empty() {
            // 0 红先行
            self.tdd_violations += 1;
            return Err(ExecutionError::TddOrderViolation {
                id,
                reason: "TDD skill first step must be Red".to_string(),
            });
        }
        let mut exec = StepExecution::new(step, at_unix_ms);
        exec.completed_at_unix_ms = Some(at_unix_ms);
        inv.step_history.push(exec);
        inv.status = if inv.step_history.len() == steps.len() {
            SkillExecutionStatus::InProgress {
                step_index: next_index,
            }
        } else if inv.step_history.len() + 1 == steps.len() {
            // 走完最后一步时, 留在 InProgress 让 complete() 终结
            SkillExecutionStatus::InProgress {
                step_index: next_index,
            }
        } else {
            SkillExecutionStatus::InProgress {
                step_index: next_index,
            }
        };
        Ok(next_index + 1)
    }

    /// 标记 invocation 完成.
    pub fn complete(&mut self, id: InvocationId, at_unix_ms: u64) -> Result<(), ExecutionError> {
        let inv = self
            .find_mut(id)
            .ok_or(ExecutionError::UnknownInvocation { id })?;
        if matches!(
            inv.status,
            SkillExecutionStatus::Failed { .. } | SkillExecutionStatus::Abandoned
        ) {
            return Err(ExecutionError::AlreadyTerminated { id });
        }
        inv.status = SkillExecutionStatus::Completed;
        inv.finished_at_unix_ms = Some(at_unix_ms);
        Ok(())
    }

    /// 主动放弃 invocation (含 reason).
    pub fn abandon(
        &mut self,
        id: InvocationId,
        reason: &str,
        at_unix_ms: u64,
    ) -> Result<(), ExecutionError> {
        let inv = self
            .find_mut(id)
            .ok_or(ExecutionError::UnknownInvocation { id })?;
        if matches!(
            inv.status,
            SkillExecutionStatus::Failed { .. } | SkillExecutionStatus::Abandoned
        ) {
            return Err(ExecutionError::AlreadyTerminated { id });
        }
        inv.status = SkillExecutionStatus::Abandoned;
        inv.finished_at_unix_ms = Some(at_unix_ms);
        // 注: reason 在 SkillExecutionStatus::Abandoned 不存, 仅存 timestamp
        // R125-18 跑完可能加 Abandoned { reason } 变体
        let _ = reason;
        Ok(())
    }

    /// 查 1 个 invocation.
    pub fn get(&self, id: InvocationId) -> Option<&SkillInvocation> {
        self.invocations.iter().find(|i| i.id == id)
    }

    /// 全部 invocations.
    pub fn invocations(&self) -> &[SkillInvocation] {
        &self.invocations
    }

    /// invocations 数.
    pub fn count(&self) -> usize {
        self.invocations.len()
    }

    /// TDD 顺序违例累计次数.
    pub fn tdd_violations(&self) -> u32 {
        self.tdd_violations
    }

    /// 按 skill_id 过滤 invocations.
    pub fn invocations_for(&self, skill_id: SkillId) -> Vec<&SkillInvocation> {
        self.invocations
            .iter()
            .filter(|i| i.skill_id == skill_id)
            .collect()
    }

    fn find_mut(&mut self, id: InvocationId) -> Option<&mut SkillInvocation> {
        self.invocations.iter_mut().find(|i| i.id == id)
    }
}

// ============================================================================
// 单元 tests (5 tests, 临时维护版 — R125-18 跑完会替换为完整 9 unit test)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_trait::{
        BrainstormingSkill, TestDrivenDevelopmentSkill, UsingSuperpowersSkill,
    };

    #[test]
    fn executor_starts_invocation_in_pending() {
        let mut ex = SkillExecutor::new();
        let id = ex.start(SkillId::Brainstorming, 1000);
        let inv = ex.get(id).expect("invocation");
        assert_eq!(inv.skill_id, SkillId::Brainstorming);
        assert!(matches!(inv.status, SkillExecutionStatus::Pending));
        assert_eq!(inv.started_at_unix_ms, 1000);
        assert!(inv.step_history.is_empty());
    }

    #[test]
    fn executor_advances_through_5_steps() {
        let mut ex = SkillExecutor::new();
        let id = ex.start(SkillId::Brainstorming, 1000);
        let skill = BrainstormingSkill;
        for i in 1..=5 {
            let idx = ex
                .advance_step(id, &skill, 1000 + i as u64)
                .expect("advance");
            assert_eq!(idx, i);
        }
        let inv = ex.get(id).expect("inv");
        assert_eq!(inv.step_history.len(), 5);
    }

    #[test]
    fn executor_tdd_skill_first_step_red() {
        let mut ex = SkillExecutor::new();
        let id = ex.start(SkillId::TestDrivenDevelopment, 1000);
        let skill = TestDrivenDevelopmentSkill;
        let idx = ex
            .advance_step(id, &skill, 1100)
            .expect("advance step 1 (RED)");
        assert_eq!(idx, 1);
        let inv = ex.get(id).expect("inv");
        assert!(
            inv.tdd_red_done,
            "after step 1 (RED), tdd_red_done must be true"
        );
    }

    #[test]
    fn executor_complete_marks_finished() {
        let mut ex = SkillExecutor::new();
        let id = ex.start(SkillId::Brainstorming, 1000);
        let skill = BrainstormingSkill;
        for _ in 0..5 {
            ex.advance_step(id, &skill, 2000).expect("advance");
        }
        ex.complete(id, 5000).expect("complete");
        let inv = ex.get(id).expect("inv");
        assert!(matches!(inv.status, SkillExecutionStatus::Completed));
        assert_eq!(inv.finished_at_unix_ms, Some(5000));
    }

    #[test]
    fn executor_meta_skill_no_tdd_required() {
        let s = UsingSuperpowersSkill;
        assert!(!s.tdd_required(), "meta skill must not require TDD");
    }
}
