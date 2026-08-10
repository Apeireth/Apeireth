//! apeireth-action: 行动器官 (A11.1 落点 — R14 Phase 4)
//!
//! **职责**: 改变环境 + 工具执行 + 表达输出 + 沉默（不行动也是合法行动）。
//! 接收 cognition 器官产出的 ActionPlan / ActionIntent，
//! 通过三个核心 trait 完成执行、表达、沉默三种行动形态。
//!
//! **架构位置**: 阶段 4 §2 主路径 18 crate 之 9 器官第 3 项（行动）
//! （perception → cognition → action → memory → ...）。
//!
//! **当前状态**: A11.1 最小可用落地（P7 任务 3c479302 by fullstack_engineer, 2026-08-01）。
//! 本 crate 提供 3 核心 trait（ActionExecution / ActionExpression / ActionSilence）
//! + 默认实现 `ActionEngine` + 5+ pub fn + 5+ pub struct/enum + 5+ 单元测试 + 1+ 集成测试。
//!
//! **诚实登记**: 行动器官的「执行」侧默认是 in-memory 模拟（不改环境、不调外部工具），
//! 真实工具桥接 / sandbox-validator 留给 A14/A19 深化。本 crate 只承载 trait + dispatcher。
//!
//! **禁止**:
//! - ❌ 不修改 apeireth-core 任何已实装类型签名
//! - ❌ 不碰 R11 baseline 三值
//! - ❌ 不碰 apeireth-legacy/

#![deny(unsafe_code)]

use apeireth_core::ActionTarget;
use thiserror::Error;

mod execution;
mod expression;
mod silence;

pub use execution::{ActionAtom, ActionEngine, ActionPlan, ExecutionResult, RollbackResult, TxId};
pub use expression::{ActionIntent, ExpressionChannel, StructuredOutput};
pub use silence::SilenceReason;

/// 行动器官顶层错误: 所有 action 子系统的 fallback error.
#[derive(Debug, Error)]
pub enum ActionError {
    /// 输入参数非法.
    #[error("invalid action input: {0}")]
    InvalidInput(String),
    /// 序列化错误.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// 统一结果类型.
pub type ActionResult<T> = Result<T, ActionError>;

/// 行动执行的 trait (核心 trait 1/3 — 阶段 4 §3.3 Action + Execution 合并).
///
/// 任何「改变环境」的具体实现都必须实现本 trait。
/// 调度方通过 `execute_plan` / `dispatch_atom` / `rollback_tx` 三个方法驱动。
pub trait ActionExecution: Send + Sync + 'static {
    /// 原子性执行一个 ActionPlan.
    fn execute_plan(&self, plan: &ActionPlan) -> ExecutionResult;
    /// 原子性执行一个更细粒度的 ActionAtom.
    fn dispatch_atom(&self, atom: ActionAtom) -> ExecutionResult;
    /// 按事务 ID 回滚 (PHL-02b not_undo 强制 — 回滚只对「未来」生效, 已落地的副作用不回滚).
    fn rollback_tx(&self, tx_id: TxId) -> RollbackResult;
}

/// 行动表达的 trait (核心 trait 2/3 — 阶段 4 §3.3 Expression).
///
/// 把内部意图投影到外部通道（文字/语音/多模态/结构化）。
pub trait ActionExpression: Send + Sync + 'static {
    /// 把意图投影到目标通道.
    fn express(&self, intent: &ActionIntent, channel: ExpressionChannel) -> StructuredOutput;
    /// 便捷: 默认文字通道.
    fn express_text(&self, intent: &ActionIntent) -> String {
        self.express(intent, ExpressionChannel::Text).text_payload()
    }
}

/// 行动沉默的 trait (核心 trait 3/3 — 阶段 4 §3.3 Silence).
///
/// **不行动也是一种合法行动**。本 trait 显式承认沉默是行动器官的合法输出。
pub trait ActionSilence: Send + Sync + 'static {
    /// 判定当前意图是否应该沉默.
    fn should_silence(&self, intent: &ActionIntent) -> bool;
    /// 给出沉默理由.
    fn reason_for_silence(&self, intent: &ActionIntent) -> SilenceReason;
}

/// 默认聚合入口: 把 execution / expression / silence 三个 trait 聚合成一个对象。
///
/// Ponytail 立场: 不引入 trait object factory — 直接给一个默认 `ActionEngine` struct，
/// 既满足「需要一个能跑的实例」又避免无意义的抽象。
#[derive(Debug, Default)]
pub struct DefaultActionEngine {
    inner: ActionEngine,
}

impl DefaultActionEngine {
    /// 构造默认引擎.
    pub fn new() -> Self {
        Self::default()
    }

    /// 取得内部引擎引用.
    pub fn engine(&self) -> &ActionEngine {
        &self.inner
    }
}

impl ActionExecution for DefaultActionEngine {
    fn execute_plan(&self, plan: &ActionPlan) -> ExecutionResult {
        self.inner.execute_plan(plan)
    }
    fn dispatch_atom(&self, atom: ActionAtom) -> ExecutionResult {
        self.inner.dispatch_atom(atom)
    }
    fn rollback_tx(&self, tx_id: TxId) -> RollbackResult {
        self.inner.rollback_tx(tx_id)
    }
}

impl ActionExpression for DefaultActionEngine {
    fn express(&self, intent: &ActionIntent, channel: ExpressionChannel) -> StructuredOutput {
        self.inner.express(intent, channel)
    }
}

impl ActionSilence for DefaultActionEngine {
    fn should_silence(&self, intent: &ActionIntent) -> bool {
        self.inner.should_silence(intent)
    }
    fn reason_for_silence(&self, intent: &ActionIntent) -> SilenceReason {
        self.inner.reason_for_silence(intent)
    }
}

/// 顶层便捷函数: 执行一个 plan, 错误时返回 ActionError.
pub fn run_execute(
    engine: &dyn ActionExecution,
    plan: &ActionPlan,
) -> ActionResult<ExecutionResult> {
    plan.validate().map_err(ActionError::InvalidInput)?;
    Ok(engine.execute_plan(plan))
}

/// 顶层便捷函数: 表达一个 intent.
pub fn run_express(
    engine: &dyn ActionExpression,
    intent: &ActionIntent,
    channel: ExpressionChannel,
) -> StructuredOutput {
    engine.express(intent, channel)
}

/// 顶层便捷函数: 判定沉默并返回理由.
pub fn run_silence(engine: &dyn ActionSilence, intent: &ActionIntent) -> SilenceReason {
    if engine.should_silence(intent) {
        engine.reason_for_silence(intent)
    } else {
        SilenceReason::NotSilent
    }
}

/// 工具函数: 判定一个 plan 是否可执行 (非空 + 不是永远禁止的 12 键 variant).
///
/// 12 键 hardcode 拒绝 — ModifyL0HA / ReorganizeOnion / ModifyEvolutionL0 永远不可执行。
pub fn is_actionable(plan: &ActionPlan) -> bool {
    !plan.steps.is_empty()
        && !matches!(
            plan.target,
            ActionTarget::ModifyL0HA
                | ActionTarget::ReorganizeOnion
                | ActionTarget::ModifyEvolutionL0
        )
}

/// 工具函数: 分配新的 TxId (UUID-backed).
pub fn new_tx_id() -> TxId {
    TxId(uuid::Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::ActionTarget;

    fn safe_target() -> ActionTarget {
        ActionTarget::NormalAction("noop".to_string())
    }

    #[test]
    fn action_plan_validate_rejects_empty_steps() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec![],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(plan.validate().is_err());
    }

    #[test]
    fn action_plan_validate_accepts_non_empty_steps() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec!["step1".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(plan.validate().is_ok());
    }

    #[test]
    fn is_actionable_rejects_modify_l0_ha() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: ActionTarget::ModifyL0HA,
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(!is_actionable(&plan));
    }

    #[test]
    fn is_actionable_rejects_reorganize_onion() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: ActionTarget::ReorganizeOnion,
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(!is_actionable(&plan));
    }

    #[test]
    fn is_actionable_rejects_empty_steps() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec![],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(!is_actionable(&plan));
    }

    #[test]
    fn is_actionable_accepts_normal_action_with_steps() {
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "test".to_string(),
        };
        assert!(is_actionable(&plan));
    }

    #[test]
    fn new_tx_id_is_unique() {
        let a = new_tx_id();
        let b = new_tx_id();
        assert_ne!(a, b);
    }

    #[test]
    fn run_execute_returns_invalid_input_for_empty_steps() {
        let engine = DefaultActionEngine::new();
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec![],
            created_at: 0,
            context: "test".to_string(),
        };
        let res = run_execute(&engine, &plan);
        assert!(matches!(res, Err(ActionError::InvalidInput(_))));
    }

    #[test]
    fn default_engine_dispatches_through_all_three_traits() {
        let engine = DefaultActionEngine::new();
        // ActionExecution
        let plan = ActionPlan {
            plan_id: uuid::Uuid::new_v4(),
            target: safe_target(),
            steps: vec!["x".to_string()],
            created_at: 0,
            context: "x".to_string(),
        };
        assert!(matches!(
            engine.execute_plan(&plan),
            ExecutionResult::Applied(_)
        ));
        // ActionExpression
        let intent = ActionIntent::new(safe_target());
        let out = engine.express(&intent, ExpressionChannel::Text);
        assert_eq!(out.channel, ExpressionChannel::Text);
        // ActionSilence
        assert!(!engine.should_silence(&intent));
        assert_eq!(engine.reason_for_silence(&intent), SilenceReason::NotSilent);
    }
}
