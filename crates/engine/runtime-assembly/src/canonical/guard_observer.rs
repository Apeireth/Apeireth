//! Observer implementing `RuntimeEventSink` for closing the loop on Guard ML dataset collection.
//!
//! Observes runtime events (approval resolutions, capability completions, turn completions/failures)
//! and feeds them as outcome records into `DatasetRecorder`.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use apeireth_guard::{BehaviorChainGuardHook, DatasetRecorder};
use apeireth_runtime::canonical::{RuntimeEvent, RuntimeEventSink, TraceEvent};

/// approval id → (action_id, tool_call_id) 关联表的容量上限 (L 组)。
///
/// 无界累积 = 长跑进程里每条未 resolve 的 approval 永久驻留; 超过容量按
/// 插入顺序驱逐最旧的绑定 (resolve 事件总是先于驱逐到达: 一次 resolve 紧跟
/// 着它的 ApprovalRequired, 不会在队列里等 1024 条)。
const MAX_APPROVAL_BINDINGS: usize = 1024;

/// Observer implementing `RuntimeEventSink` for closing the loop on Guard ML dataset collection.
#[derive(Clone)]
pub struct GuardDatasetObserver {
    recorder: Arc<DatasetRecorder>,
    hook: Option<Arc<BehaviorChainGuardHook>>,
    /// approval id → (action_id, tool_call_id), 插入有序且有界。
    approvals: Arc<Mutex<VecDeque<(String, (String, String))>>>,
}

impl GuardDatasetObserver {
    /// Creates a new observer wrapping the given dataset recorder.
    pub fn new(recorder: Arc<DatasetRecorder>) -> Self {
        Self {
            recorder,
            hook: None,
            approvals: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    #[must_use]
    pub fn with_hook(mut self, hook: Arc<BehaviorChainGuardHook>) -> Self {
        self.hook = Some(hook);
        self
    }

    /// Access the underlying dataset recorder.
    pub fn recorder(&self) -> &Arc<DatasetRecorder> {
        &self.recorder
    }

    /// 取一次 approval 的 (action_id, tool_call_id) 绑定 (后进先查)。
    ///
    /// poison 容错 (L 组): 关联表是 telemetry 级状态, 不是安全边界; 持锁
    /// 线程 panic 后取回数据继续, 不让级联 panic 打死事件线程。
    fn take_binding(&self, approval_id: &str) -> Option<(String, String)> {
        let mut bindings = self
            .approvals
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let position = bindings
            .iter()
            .position(|(id, _)| id == approval_id)?;
        bindings.remove(position).map(|(_, binding)| binding)
    }
}

impl RuntimeEventSink for GuardDatasetObserver {
    fn emit(&self, event: RuntimeEvent) {
        if !self.recorder.is_enabled() {
            return;
        }

        match event {
            RuntimeEvent::ApprovalRequired {
                approval,
                tool_call_id,
                ..
            } => {
                // action_id 用治理请求的真实 action 身份: runtime 在每个
                // dispatch 点都以 `GovernanceRequest::with_action_id(&call.id)`
                // 把 provider tool-call id 绑上 (execute.rs::dispatch_one_tool;
                // M20 后 subloop.rs::dispatch_subloop_tool 同样绑定), guard 的
                // 分类行用的就是这一个 id。subloop 此前不绑 action_id, hook 只能
                // 自行派生 `act:{request}:{round}:{seq}`, 与这里写的 tool_call_id
                // 错配 —— 标签错配的根因在 dispatch 点, 已在 M20 修正。
                let mut bindings = self
                    .approvals
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if bindings.len() >= MAX_APPROVAL_BINDINGS {
                    bindings.pop_front();
                }
                bindings.push_back((
                    approval.to_string(),
                    (tool_call_id.clone(), tool_call_id),
                ));
                // The request is intentionally not a training label. The
                // eventual resolution below is the approval event.
            }
            RuntimeEvent::Trace {
                session,
                trace,
                at: _,
                event,
            } => match event {
                TraceEvent::ApprovalResolved {
                    approval_id,
                    decision,
                    round: _,
                } => {
                    if let Some((action_id, tool_call_id)) =
                        self.take_binding(&approval_id.to_string())
                    {
                        self.recorder.record_approval(
                            &trace.to_string(),
                            &action_id,
                            &tool_call_id,
                            &approval_id.to_string(),
                            &decision,
                        );
                    } else {
                        self.recorder.record_outcome(
                            &trace.to_string(),
                            None,
                            None,
                            Some(&approval_id.to_string()),
                            Some(&decision),
                            None,
                        );
                    }
                }
                TraceEvent::CapabilityCompleted {
                    capability: _,
                    tool_call_id,
                    succeeded,
                    round: _,
                } => {
                    let outcome = if succeeded {
                        apeireth_guard::GuardExecutionOutcome::Success.as_str()
                    } else {
                        apeireth_guard::GuardExecutionOutcome::CapabilityFailure.as_str()
                    };
                    self.recorder.record_execution(
                        &trace.to_string(),
                        &tool_call_id,
                        &tool_call_id,
                        outcome,
                    );
                    if let Some(hook) = &self.hook {
                        hook.update_action_execution(
                            &session,
                            &trace.to_string(),
                            &tool_call_id,
                            succeeded,
                        );
                    }
                }
                TraceEvent::TurnCompleted { .. } => {
                    self.recorder.record_outcome(
                        &trace.to_string(),
                        None,
                        None,
                        None,
                        None,
                        Some(apeireth_guard::GuardExecutionOutcome::Success.as_str()),
                    );
                }
                _ => {}
            },
            RuntimeEvent::TurnCompleted { trace, .. } => {
                self.recorder.record_outcome(
                    &trace.to_string(),
                    None,
                    None,
                    None,
                    None,
                    Some(apeireth_guard::GuardExecutionOutcome::Success.as_str()),
                );
            }
            RuntimeEvent::TurnFailed { trace, error, .. } => {
                let outcome = apeireth_guard::GuardExecutionOutcome::from_failure_hint(&error);
                self.recorder.record_outcome(
                    &trace.to_string(),
                    None,
                    None,
                    None,
                    None,
                    Some(outcome.as_str()),
                );
            }
            _ => {}
        }
    }
}
