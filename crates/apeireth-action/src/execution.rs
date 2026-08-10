//! 执行模块: ActionPlan + ActionAtom + ActionEngine + tx rollback。

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::ActionTarget;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ActionExecution, ActionExpression, ActionSilence};

/// 事务 ID (UUID-backed) — 用于「原子性 + 可回滚」追踪。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxId(pub Uuid);

/// 一次待执行行动的全部信息 — 由 cognition 器官产出, action 器官消费。
#[derive(Debug, Clone)]
pub struct ActionPlan {
    /// 唯一 plan ID.
    pub plan_id: Uuid,
    /// 行动目标 (12 键 hardcode 锁定的对象).
    pub target: ActionTarget,
    /// 执行步骤的有序列表 (描述字符串).
    pub steps: Vec<String>,
    /// 创建时间戳 (epoch seconds).
    pub created_at: i64,
    /// 上下文标签.
    pub context: String,
}

impl ActionPlan {
    /// 构造最小可执行 plan.
    pub fn new(target: ActionTarget, steps: Vec<String>, context: impl Into<String>) -> Self {
        Self {
            plan_id: Uuid::new_v4(),
            target,
            steps,
            created_at: now_epoch(),
            context: context.into(),
        }
    }

    /// 校验 plan 合法性. 空 steps 直接拒绝.
    pub fn validate(&self) -> Result<(), String> {
        if self.steps.is_empty() {
            return Err("action plan must have at least one step".to_string());
        }
        if self.context.is_empty() {
            return Err("action plan context must not be empty".to_string());
        }
        Ok(())
    }

    /// 步骤数量.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

/// 原子动作 — 比 ActionPlan 更细粒度的执行单元 (单步)。
#[derive(Debug, Clone)]
pub struct ActionAtom {
    /// 唯一 atom ID.
    pub atom_id: Uuid,
    /// 行动目标 (同 ActionPlan.target).
    pub target: ActionTarget,
    /// 负载字符串 (单步内容).
    pub payload: String,
}

impl ActionAtom {
    /// 构造最小原子动作.
    pub fn new(target: ActionTarget, payload: impl Into<String>) -> Self {
        Self {
            atom_id: Uuid::new_v4(),
            target,
            payload: payload.into(),
        }
    }
}

/// 执行结果 — applied / rolled-back / failed 三态.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// 已应用, 附带事务 ID (可后续 rollback).
    Applied(TxId),
    /// 已回滚 (罕见 — 通常由 rollback_tx 返回, 而非 execute_plan).
    RolledBack(TxId),
    /// 执行失败, 附带可选 tx_id (如果失败发生在记录之后).
    Failed {
        /// 失败时是否已分配 tx_id (true = 事务被记录但执行失败).
        tx_id: Option<TxId>,
        /// 失败原因.
        reason: String,
    },
}

impl ExecutionResult {
    /// 是否成功应用.
    pub fn is_applied(&self) -> bool {
        matches!(self, ExecutionResult::Applied(_))
    }

    /// 是否失败.
    pub fn is_failed(&self) -> bool {
        matches!(self, ExecutionResult::Failed { .. })
    }

    /// 关联的 tx_id (如果有).
    pub fn tx_id(&self) -> Option<TxId> {
        match self {
            ExecutionResult::Applied(tx) | ExecutionResult::RolledBack(tx) => Some(*tx),
            ExecutionResult::Failed { tx_id, .. } => *tx_id,
        }
    }
}

/// 回滚结果 — 三态 + 永远返回关联 tx_id.
#[derive(Debug, Clone, PartialEq)]
pub enum RollbackResult {
    /// 已回滚.
    RolledBack(TxId),
    /// 未找到对应事务.
    NotFound(TxId),
    /// 事务存在但不可回滚 (PHL-02b not_undo 强制 — 落地副作用不可回滚).
    NotRollbackable(TxId),
}

impl RollbackResult {
    /// 关联的 tx_id.
    pub fn tx_id(&self) -> TxId {
        match self {
            RollbackResult::RolledBack(tx)
            | RollbackResult::NotFound(tx)
            | RollbackResult::NotRollbackable(tx) => *tx,
        }
    }

    /// 是否成功回滚.
    pub fn is_rolled_back(&self) -> bool {
        matches!(self, RollbackResult::RolledBack(_))
    }
}

/// 默认 action 引擎 — 内存模拟执行 + tx log。
///
/// Ponytail 立场: 不引入 trait object factory, 单实例 struct + Mutex tx log 已足够 A11.1
/// 「最小可用」标准。真实工具桥接 / sandbox-validator 留给 A14/A19。
#[derive(Debug, Default)]
pub struct ActionEngine {
    /// 事务日志: TxId → 该事务关联的 plan 快照.
    tx_log: Mutex<HashMap<TxId, ActionPlan>>,
}

impl ActionEngine {
    /// 构造新引擎.
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前 tx_log 中的事务数量.
    pub fn tx_count(&self) -> usize {
        self.tx_log.lock().map(|l| l.len()).unwrap_or(0)
    }

    /// 列出所有已记录事务 (用于审计).
    pub fn list_tx(&self) -> Vec<(TxId, ActionPlan)> {
        self.tx_log
            .lock()
            .map(|l| l.iter().map(|(k, v)| (*k, v.clone())).collect())
            .unwrap_or_default()
    }
}

impl ActionExecution for ActionEngine {
    fn execute_plan(&self, plan: &ActionPlan) -> ExecutionResult {
        // 12 键 hardcode 拒绝 (在 dispatcher 阶段就阻止, 避免污染 tx_log)
        if !crate::is_actionable(plan) {
            return ExecutionResult::Failed {
                tx_id: None,
                reason: "12-key blocked or empty steps".to_string(),
            };
        }

        let tx_id = TxId(Uuid::new_v4());
        match self.tx_log.lock() {
            Ok(mut log) => {
                log.insert(tx_id, plan.clone());
                ExecutionResult::Applied(tx_id)
            }
            Err(poisoned) => {
                // Mutex 已被污染 — 严重错误但仍返回 Failed
                let _ = poisoned;
                ExecutionResult::Failed {
                    tx_id: Some(tx_id),
                    reason: "tx_log mutex poisoned".to_string(),
                }
            }
        }
    }

    fn dispatch_atom(&self, atom: ActionAtom) -> ExecutionResult {
        // 单步原子 — 包装为 ActionPlan 再执行 (复用路径).
        let plan = ActionPlan {
            plan_id: atom.atom_id,
            target: atom.target,
            steps: vec![atom.payload],
            created_at: now_epoch(),
            context: "atom_dispatch".to_string(),
        };
        self.execute_plan(&plan)
    }

    fn rollback_tx(&self, tx_id: TxId) -> RollbackResult {
        match self.tx_log.lock() {
            Ok(mut log) => match log.remove(&tx_id) {
                Some(_) => RollbackResult::RolledBack(tx_id),
                None => RollbackResult::NotFound(tx_id),
            },
            Err(_) => RollbackResult::NotRollbackable(tx_id),
        }
    }
}

/// epoch seconds (stdlib only — 不引入 chrono 依赖).
fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
