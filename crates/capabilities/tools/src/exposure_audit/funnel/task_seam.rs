//! 守望巡检漏斗 · 任务接缝 (巡检 = 持久任务, 队列同账)。
//!
//! 一次巡检就是**一个持久任务**: 本模块的 [`PatrolTaskAccount`] 是巡检侧的任务
//! 账, 状态词面/转移规则/人判边界全部**同款消费 V6-a 持久任务队列的语义**
//! ([`apeireth_orchestration::task_queue`]): 六态状态机、自报必须是执行者本人、
//! **失败只能人判、失败绝不自动重试** (失败留账 = 留给人判的账)。
//!
//! 对队列本体**只读消费** ([`PatrolTaskAccount::load_from_queue`] 只调
//! [`TaskQueue::get`]): 入队/指派/人判等写面全部沿用 V6-a 既有出口 (如
//! `tool.task_board`), 漏斗层不新增队列写面; [`PatrolTaskAccount::to_task_spec`]
//! 产出的正是队列既有入口吃的 [`TaskSpec`], 两边**同一本账**
//! ([`PatrolTaskAccount::sync_from_record`] 对齐同一记录)。

use apeireth_orchestration::task_queue::{
    TaskCaller, TaskQueue, TaskQueueError, TaskQueueResult, TaskRecord, TaskSpec, TaskState,
};

/// 失败留账 (人判记账; 只存原因与人判者, 零秘密值)。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaskFailureRecord {
    /// 失败原因 (中性表述)。
    pub reason: String,
    /// 人判者 id。
    pub judged_by: String,
    /// 人判时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 恒 true: 失败账只能人判进、人判出 (同款 V6-a 语义, 命名即承诺)。
    pub human_judge_only: bool,
}

/// 巡检任务账 (状态词面与转移规则与 V6-a 队列同款)。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PatrolTaskAccount {
    /// 队列任务 id (接入队列后回填; 未接入 = None, 不编号)。
    pub task_id: Option<String>,
    /// 任务标题。
    pub title: String,
    /// 幂等键 (同键重复投递不重复执行 —— 同款语义, 供队列入口使用)。
    pub idempotency_key: String,
    /// 执行者 (自报必须是本人)。
    pub assignee: String,
    /// 当前状态 (V6-a 六态词面)。
    pub state: TaskState,
    /// 执行段计数 (每次 begin +1; 断点续查的会话数, 同款崩溃恢复口径)。
    pub attempts: u32,
    /// 最近留痕的运行错误 (留痕不改态; 错误即帧在上层)。
    pub last_error: Option<String>,
    /// 失败留账 (人判后才有; 未失败 = None)。
    pub failure: Option<TaskFailureRecord>,
}

impl PatrolTaskAccount {
    /// 新巡检任务账 (Pending 起步, 幂等键 = 巡检 id)。
    pub fn new(patrol_id: impl Into<String>, assignee: impl Into<String>) -> Self {
        let patrol_id = patrol_id.into();
        Self {
            task_id: None,
            title: format!("守望巡检 {patrol_id}"),
            idempotency_key: patrol_id,
            assignee: assignee.into(),
            state: TaskState::Pending,
            attempts: 0,
            last_error: None,
            failure: None,
        }
    }

    /// 供队列入口消费的入队规格 (队列既有写面吃的形状)。
    pub fn to_task_spec(&self) -> TaskSpec {
        TaskSpec::new(
            self.title.clone(),
            serde_json::json!({
                "kind": "exposure_patrol",
                "idempotency_key": self.idempotency_key,
            }),
        )
        .with_idempotency_key(self.idempotency_key.clone())
        .with_assignee(self.assignee.clone())
    }

    /// 崩溃恢复 (断点续查的重启口径, 与 `TaskQueue::open` 同款):
    /// `Running` 中途死 → 回 `Pending` 等续跑; 其余态原样。
    pub fn recover_on_resume(&mut self) {
        if self.state == TaskState::Running {
            self.state = TaskState::Pending;
        }
    }

    /// 执行段开始 (Pending/Assigned/Blocked → Running, attempts +1;
    /// 自报必须是执行者本人)。
    pub fn begin(&mut self, caller: &TaskCaller) -> TaskQueueResult<()> {
        self.require_assignee(caller)?;
        match self.state {
            TaskState::Pending | TaskState::Assigned | TaskState::Blocked => {
                self.state = TaskState::Running;
                self.attempts += 1;
                Ok(())
            }
            from => Err(self.invalid_transition(from, "begin")),
        }
    }

    /// 执行段完成 (Running → Done; 自报必须是执行者本人)。
    pub fn complete(&mut self, caller: &TaskCaller) -> TaskQueueResult<()> {
        self.require_assignee(caller)?;
        match self.state {
            TaskState::Running => {
                self.state = TaskState::Done;
                Ok(())
            }
            from => Err(self.invalid_transition(from, "complete")),
        }
    }

    /// 运行错误留痕 (不改态: 错误即帧由上层报真因, 账面留痕可查)。
    pub fn note_error(&mut self, reason: impl Into<String>) {
        self.last_error = Some(reason.into());
    }

    /// 人判失败 (**只能人判** —— agent 调用结构性拒绝, 失败留账同款)。
    pub fn human_judge_fail(
        &mut self,
        caller: &TaskCaller,
        reason: impl Into<String>,
        at_ms: i64,
    ) -> TaskQueueResult<()> {
        if !caller.is_human {
            return Err(TaskQueueError::HumanOnly {
                action: "human_judge_fail",
            });
        }
        match self.state {
            TaskState::Done | TaskState::Failed => {
                Err(self.invalid_transition(self.state, "human_judge_fail"))
            }
            _ => {
                self.state = TaskState::Failed;
                self.failure = Some(TaskFailureRecord {
                    reason: reason.into(),
                    judged_by: caller.id.clone(),
                    at_ms,
                    human_judge_only: true,
                });
                Ok(())
            }
        }
    }

    /// 人判重试 (**显式人判动作**, 失败绝不自动重试 —— 同款语义;
    /// agent 调用结构性拒绝)。
    pub fn human_judge_retry(&mut self, caller: &TaskCaller) -> TaskQueueResult<()> {
        if !caller.is_human {
            return Err(TaskQueueError::HumanOnly {
                action: "human_judge_retry",
            });
        }
        match self.state {
            TaskState::Failed => {
                self.state = TaskState::Pending;
                Ok(())
            }
            from => Err(self.invalid_transition(from, "human_judge_retry")),
        }
    }

    /// 只读消费队列: 读回同一本账的队列记录并对齐 (队列同账)。
    pub fn load_from_queue(&mut self, queue: &TaskQueue, task_id: &str) -> TaskQueueResult<()> {
        let record = queue.get(task_id)?;
        self.sync_from_record(&record);
        Ok(())
    }

    /// 与队列记录对齐同一本账 (id / 状态 / 执行段计数)。
    pub fn sync_from_record(&mut self, record: &TaskRecord) {
        self.task_id = Some(record.id.clone());
        self.state = record.state;
        self.attempts = record.attempts;
    }

    /// JSON 合同值 (报告里的任务账节)。
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "task_id": self.task_id,
            "title": self.title,
            "idempotency_key": self.idempotency_key,
            "assignee": self.assignee,
            "state": self.state.to_string(),
            "attempts": self.attempts,
            "last_error": self.last_error,
            "failure": self.failure.as_ref().map(|failure| serde_json::json!({
                "reason": failure.reason,
                "judged_by": failure.judged_by,
                "at_ms": failure.at_ms,
                "human_judge_only": failure.human_judge_only,
            })),
        })
    }

    /// 认领授权: 自报必须是执行者本人 (同款)。
    fn require_assignee(&self, caller: &TaskCaller) -> TaskQueueResult<()> {
        if caller.id == self.assignee {
            Ok(())
        } else {
            Err(TaskQueueError::NotAssignee {
                task_id: self.display_id(),
                caller: caller.id.clone(),
            })
        }
    }

    /// 状态机不允许的转移 (拒绝即帧, 同款错误形状)。
    fn invalid_transition(&self, from: TaskState, action: &'static str) -> TaskQueueError {
        TaskQueueError::InvalidTransition {
            task_id: self.display_id(),
            from,
            action,
        }
    }

    /// 错误帧里的任务身份 (未接入队列时用幂等键, 不编队列号)。
    fn display_id(&self) -> String {
        self.task_id
            .clone()
            .unwrap_or_else(|| self.idempotency_key.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> PatrolTaskAccount {
        PatrolTaskAccount::new("patrol-1", "keeper")
    }

    #[test]
    fn begin_and_complete_require_the_assignee_themselves() {
        let mut task = account();
        assert!(task.begin(&TaskCaller::agent("keeper")).is_ok());
        assert_eq!(task.attempts, 1);
        let stranger = task.begin(&TaskCaller::agent("other"));
        assert!(matches!(stranger, Err(TaskQueueError::NotAssignee { .. })));
        assert!(task.complete(&TaskCaller::agent("other")).is_err());
        assert!(task.complete(&TaskCaller::agent("keeper")).is_ok());
        assert_eq!(task.state, TaskState::Done);
    }

    #[test]
    fn failure_is_human_judged_only_and_never_auto_retries() {
        let mut task = account();
        task.begin(&TaskCaller::agent("keeper")).unwrap();
        let agent_fail = task.human_judge_fail(&TaskCaller::agent("keeper"), "boom", 1);
        assert!(matches!(agent_fail, Err(TaskQueueError::HumanOnly { .. })));
        assert_eq!(task.state, TaskState::Running, "agent 碰不到失败账");

        assert!(task
            .human_judge_fail(&TaskCaller::human("owner"), "boom", 7)
            .is_ok());
        assert_eq!(task.state, TaskState::Failed);
        let failure = task.failure.clone().expect("失败留账");
        assert!(failure.human_judge_only);
        assert_eq!(failure.judged_by, "owner");

        // 失败绝不自动重试: 重试只能人判, agent 结构性拒绝。
        let agent_retry = task.human_judge_retry(&TaskCaller::agent("keeper"));
        assert!(matches!(agent_retry, Err(TaskQueueError::HumanOnly { .. })));
        assert_eq!(task.state, TaskState::Failed, "失败不自动回队");
        assert!(task.human_judge_retry(&TaskCaller::human("owner")).is_ok());
        assert_eq!(task.state, TaskState::Pending);
    }

    #[test]
    fn resume_recovers_running_to_pending_like_queue_open() {
        let mut task = account();
        task.begin(&TaskCaller::agent("keeper")).unwrap();
        task.recover_on_resume();
        assert_eq!(
            task.state,
            TaskState::Pending,
            "中途死: running→pending 同款"
        );
        task.begin(&TaskCaller::agent("keeper")).unwrap();
        assert_eq!(task.attempts, 2, "每个执行段计一次");
    }
}
