//! Sovereignty 可解释性 — 决策路径追踪 (R20 阶段 6 估补)
//!
//! **职责** (本模块 flesh out 估补, lib.rs LOCKED 不重 export):
//! - **决策路径追踪** (`DecisionTrace`): 记录一次治理动作从 `request` → `evidence` →
//!   `verdict` → `rationale` 的完整因果链
//! - **5 阶段** (编译时 hardcode, 与 5 重治理对齐):
//!   1. `RequestReceived` — 请求进入
//!   2. `EvidenceCollected` — 证据收集
//!   3. `AuthorityConsulted` — 咨询权威
//!   4. `VerdictReached` — 裁决
//!   5. `RationaleStated` — 理由陈述
//! - **3 K-1 强校验** (任何 trace 必须满足, 否则 `Err(ExplainError::K1Violation)`):
//!   1. **K-1.a** — `decision_id` 非空
//!   2. **K-1.b** — `stages.len() >= 2` (至少 Request + Verdict)
//!   3. **K-1.c** — 最后一个 stage 必须是 `VerdictReached` 或 `RationaleStated` (终止态)
//!
//! **6 哲学锚穿透**:
//! - **主 22:33 ASI 北极星** — 可解释性让治理透明, 服务"事后可还原 + 主人可审计"
//! - **主 17:43 实事求是** — rationale 是真实推理, 非装饰
//! - **主 17:58 不假装** — `try_push_stage` 返回 `Err` 表达真实失败, 不 silent pass
//! - **主 19:33 走在前人肩上** — 复用 `serde::Serialize` + `thiserror::Error` + `chrono::Utc`
//! - **主 23:44 干到底** — 3 K-1 强校验在 `try_finalize` 一处集中执行
//! - **主 00:56 任何人都能接手** — 5 阶段枚举化, 公开 API 简单直白
//!
//! **8 项不修改承诺**:
//! - ✅ 编译期 hardcode: 阶段数 = 5, K-1 强校验数 = 3
//! - ✅ 0 触碰 LOCKED
//! - ✅ 0 依赖 NewAPI
//! - ✅ 0 重复造轮子
//! - ✅ 诚实标缺: ❌ **不**含 ML 解释 (SHAP / LIME), 纯 rule-based 阶段追踪
//!
//! **诚实登记**:
//! - ❌ **不**含 ML 解释 (SHAP / LIME) — 纯 rule-based 阶段追踪
//! - ❌ **不**接 LLM reasoning — 真生产可接 LLM 作 rationale, 但本模块不预设
//! - ❌ **不**持久化 — 仅 in-memory; 真生产应接审计日志 (见 `audit.rs`)
//!
//! **用法**:
//! ```ignore
//! use explain::{DecisionTrace, Stage, StageKind, Verdict, VerdictOutcome};
//!
//! let mut trace = DecisionTrace::new("dec-1", "alice");
//! trace.try_push_stage(StageKind::RequestReceived, "modify E_layer".into()).unwrap();
//! trace.try_push_stage(StageKind::EvidenceCollected, "5 evidences collected".into()).unwrap();
//! trace.try_push_stage(StageKind::VerdictReached, "Approved by MEWG".into()).unwrap();
//! trace.try_finalize(VerdictOutcome::Approved, "weighted score 0.85".into()).unwrap();
//! assert!(trace.is_complete());
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 编译时 hardcode: 5 阶段 / 3 K-1 强校验
// ============================================================

/// 阶段数 (编译时硬编码: RequestReceived / EvidenceCollected / AuthorityConsulted / VerdictReached / RationaleStated = 5)
pub const STAGE_KIND_COUNT_HARDCODE: usize = 5;

/// K-1 强校验数 (编译时硬编码: decision_id 非空 / stages ≥ 2 / 终止态 = 3)
pub const K1_STRICT_CHECK_COUNT_HARDCODE: usize = 3;

/// 5 阶段类型 (与 5 重治理对齐)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKind {
    /// 请求进入 — 治理动作起点
    RequestReceived,
    /// 证据收集 — 5 重证据 (用户动作/系统日志/外部审计/同行评议/历史)
    EvidenceCollected,
    /// 咨询权威 — MEWG / Council / HA / 物理多签 / 反思期
    AuthorityConsulted,
    /// 裁决 — 最终 verdict
    VerdictReached,
    /// 理由陈述 — 人类可读 rationale
    RationaleStated,
}

impl StageKind {
    /// 字符串 ID
    pub fn as_str(self) -> &'static str {
        match self {
            StageKind::RequestReceived => "request_received",
            StageKind::EvidenceCollected => "evidence_collected",
            StageKind::AuthorityConsulted => "authority_consulted",
            StageKind::VerdictReached => "verdict_reached",
            StageKind::RationaleStated => "rationale_stated",
        }
    }

    /// 是否终止态 (trace 最后一个 stage 必须是终止态之一, K-1.c)
    pub fn is_terminal(self) -> bool {
        matches!(self, StageKind::VerdictReached | StageKind::RationaleStated)
    }
}

/// 裁决结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VerdictOutcome {
    /// 通过
    Approved,
    /// 拒绝
    Rejected,
    /// 重审 (反思期未结束 / 票数不足)
    PendingReview,
}

impl VerdictOutcome {
    /// 字符串 ID
    pub fn as_str(self) -> &'static str {
        match self {
            VerdictOutcome::Approved => "approved",
            VerdictOutcome::Rejected => "rejected",
            VerdictOutcome::PendingReview => "pending_review",
        }
    }
}

/// 单个阶段
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    /// 阶段类型
    pub kind: StageKind,
    /// 人类可读描述
    pub description: String,
    /// 时间戳 (epoch ms)
    pub timestamp_ms: i64,
}

impl Stage {
    /// 构造阶段 (timestamp 自动)
    pub fn new(kind: StageKind, description: impl Into<String>) -> Self {
        Self {
            kind,
            description: description.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

/// 决策路径
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTrace {
    /// 决策 ID — K-1.a
    pub decision_id: String,
    /// 发起人 (actor)
    pub initiator: String,
    /// 阶段序列
    pub stages: Vec<Stage>,
    /// 终止 verdict (try_finalize 时设置)
    pub verdict: Option<VerdictOutcome>,
    /// 终止 rationale
    pub rationale: Option<String>,
    /// 创建时间戳
    pub created_at_ms: i64,
}

impl DecisionTrace {
    /// 构造决策路径 (空 stages)
    pub fn new(decision_id: impl Into<String>, initiator: impl Into<String>) -> Self {
        Self {
            decision_id: decision_id.into(),
            initiator: initiator.into(),
            stages: Vec::new(),
            verdict: None,
            rationale: None,
            created_at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// 推入阶段 (K-1.b 部分校验: stages 顺序)
    pub fn try_push_stage(
        &mut self,
        kind: StageKind,
        description: impl Into<String>,
    ) -> Result<(), ExplainError> {
        // K-1.a
        if self.decision_id.trim().is_empty() {
            return Err(ExplainError::K1DecisionIdEmpty);
        }
        // 已 finalized 的 trace 不可再 push
        if self.verdict.is_some() {
            return Err(ExplainError::AlreadyFinalized);
        }
        // 终止态之后不可再 push
        if let Some(last) = self.stages.last() {
            if last.kind.is_terminal() {
                return Err(ExplainError::StageAfterTerminal);
            }
        }
        self.stages.push(Stage::new(kind, description));
        Ok(())
    }

    /// 终结 trace (设置 verdict + rationale, K-1.c: 最后一个 stage 必须是终止态)
    pub fn try_finalize(
        &mut self,
        verdict: VerdictOutcome,
        rationale: impl Into<String>,
    ) -> Result<(), ExplainError> {
        // K-1.a
        if self.decision_id.trim().is_empty() {
            return Err(ExplainError::K1DecisionIdEmpty);
        }
        // K-1.b: 至少 2 个 stage (Request + 至少 1)
        if self.stages.len() < 2 {
            return Err(ExplainError::K1StagesTooFew {
                actual: self.stages.len(),
                min: 2,
            });
        }
        // K-1.c: 最后一个 stage 必须是终止态
        let last_kind = self.stages.last().unwrap().kind;
        if !last_kind.is_terminal() {
            return Err(ExplainError::K1LastStageNotTerminal {
                actual: last_kind,
            });
        }
        self.verdict = Some(verdict);
        self.rationale = Some(rationale.into());
        Ok(())
    }

    /// 是否已完成 (verdict 已设置)
    pub fn is_complete(&self) -> bool {
        self.verdict.is_some()
    }

    /// 阶段数
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// 是否空
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// 3 K-1 强校验 (在 try_finalize 中集中执行; 此函数供已完成 trace 验证)
    pub fn validate_k1(&self) -> Result<(), ExplainError> {
        if self.decision_id.trim().is_empty() {
            return Err(ExplainError::K1DecisionIdEmpty);
        }
        if self.stages.len() < 2 {
            return Err(ExplainError::K1StagesTooFew {
                actual: self.stages.len(),
                min: 2,
            });
        }
        let last_kind = self.stages.last().unwrap().kind;
        if !last_kind.is_terminal() {
            return Err(ExplainError::K1LastStageNotTerminal {
                actual: last_kind,
            });
        }
        Ok(())
    }
}

/// 可解释性错误
#[derive(Debug, Error, PartialEq)]
pub enum ExplainError {
    /// K-1.a 强校验失败 — decision_id 非空
    #[error("K-1.a 强校验失败: decision_id 为空 (决策路径必须关联一个决策)")]
    K1DecisionIdEmpty,
    /// K-1.b 强校验失败 — stages 至少 2 个
    #[error("K-1.b 强校验失败: stages 数 {actual} < 最小值 {min} (至少 Request + Verdict)")]
    K1StagesTooFew {
        /// 实际数
        actual: usize,
        /// 最小值
        min: usize,
    },
    /// K-1.c 强校验失败 — 最后一个 stage 必须是终止态
    #[error("K-1.c 强校验失败: 最后一个 stage {actual:?} 不是终止态 (VerdictReached/RationaleStated)")]
    K1LastStageNotTerminal {
        /// 实际的最后一个 stage
        actual: StageKind,
    },
    /// 已 finalized 的 trace 不可再 push
    #[error("trace 已 finalized, 不可再 push stage")]
    AlreadyFinalized,
    /// 终止态之后不可再 push
    #[error("终止态之后不可再 push stage")]
    StageAfterTerminal,
}

const _: () = {
    assert!(STAGE_KIND_COUNT_HARDCODE == 5);
    assert!(K1_STRICT_CHECK_COUNT_HARDCODE == 3);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_kind_count_is_5() {
        assert_eq!(STAGE_KIND_COUNT_HARDCODE, 5);
        assert_eq!(StageKind::RequestReceived.as_str(), "request_received");
        assert_eq!(StageKind::EvidenceCollected.as_str(), "evidence_collected");
        assert_eq!(StageKind::AuthorityConsulted.as_str(), "authority_consulted");
        assert_eq!(StageKind::VerdictReached.as_str(), "verdict_reached");
        assert_eq!(StageKind::RationaleStated.as_str(), "rationale_stated");

        // 终止态
        assert!(StageKind::VerdictReached.is_terminal());
        assert!(StageKind::RationaleStated.is_terminal());
        assert!(!StageKind::RequestReceived.is_terminal());
        assert!(!StageKind::EvidenceCollected.is_terminal());
        assert!(!StageKind::AuthorityConsulted.is_terminal());
    }

    #[test]
    fn k1_strict_checks_three_failures() {
        // K-1.a
        let mut t1 = DecisionTrace::new("", "alice");
        let res1 = t1.try_push_stage(StageKind::RequestReceived, "x");
        assert_eq!(res1.err(), Some(ExplainError::K1DecisionIdEmpty));

        // K-1.b — stages 太短 (只 0 个)
        let mut t2 = DecisionTrace::new("dec-1", "alice");
        let res2 = t2.try_finalize(VerdictOutcome::Approved, "x");
        assert_eq!(
            res2.err(),
            Some(ExplainError::K1StagesTooFew {
                actual: 0,
                min: 2
            })
        );

        // K-1.b — stages 只 1 个
        t2.try_push_stage(StageKind::RequestReceived, "x")
            .unwrap();
        let res3 = t2.try_finalize(VerdictOutcome::Approved, "x");
        assert_eq!(
            res3.err(),
            Some(ExplainError::K1StagesTooFew {
                actual: 1,
                min: 2
            })
        );

        // K-1.c — 最后一个 stage 不是终止态
        t2.try_push_stage(StageKind::EvidenceCollected, "x")
            .unwrap();
        let res4 = t2.try_finalize(VerdictOutcome::Approved, "x");
        assert_eq!(
            res4.err(),
            Some(ExplainError::K1LastStageNotTerminal {
                actual: StageKind::EvidenceCollected
            })
        );
    }

    #[test]
    fn decision_trace_complete_lifecycle() {
        let mut trace = DecisionTrace::new("dec-e-mod", "alice");
        assert!(trace.is_empty());
        assert!(!trace.is_complete());

        // 推入 4 阶段
        trace
            .try_push_stage(StageKind::RequestReceived, "modify E_layer principle")
            .unwrap();
        trace
            .try_push_stage(
                StageKind::EvidenceCollected,
                "5 evidences (1 human + 1 ai + 1 multisig + 1 reflection + 1 historical)",
            )
            .unwrap();
        trace
            .try_push_stage(
                StageKind::AuthorityConsulted,
                "MEWG + Council + HA + PhysicalMultisig",
            )
            .unwrap();
        trace
            .try_push_stage(StageKind::VerdictReached, "weighted score 0.85")
            .unwrap();
        assert_eq!(trace.len(), 4);

        // 终止态之后不能再 push
        let res = trace.try_push_stage(StageKind::RationaleStated, "x");
        assert_eq!(res.err(), Some(ExplainError::StageAfterTerminal));

        // finalize
        trace
            .try_finalize(VerdictOutcome::Approved, "E 层变更经 5 重治理通过, 启动反思期")
            .unwrap();
        assert!(trace.is_complete());
        assert_eq!(trace.verdict, Some(VerdictOutcome::Approved));
        assert!(trace.rationale.as_ref().unwrap().contains("反思期"));

        // 已 finalized 不能再 push
        let res2 = trace.try_push_stage(StageKind::RationaleStated, "x");
        assert_eq!(res2.err(), Some(ExplainError::AlreadyFinalized));

        // K-1 校验通过
        trace.validate_k1().unwrap();
    }
}
