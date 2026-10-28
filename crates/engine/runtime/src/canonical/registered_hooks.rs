//! Actual V1 hook boundaries for registered plugins. Callbacks read snapshots;
//! only canonical tool dispatch can create an authenticated module fact.

use std::collections::BTreeSet;
use std::sync::Arc;

use apeireth_core::kernel::{Lifecycle, PluginId, RequestId, SessionId, TaskId, TraceId};
use apeireth_governance::TurnSecurityContext;
use apeireth_plugin::hooks::{HookContextV1, HookToolRequest};
use apeireth_protocol::canonical::{
    validate_closed_tool_context, ModuleHookV1, NormalizedResponse, ToolCall, ToolEventBatch,
    ToolEventLimits, ToolEventSourceV1, ToolEventV1, ToolResult,
};

use super::error::{RuntimeError, RuntimeResult};
use super::execute::ToolDispatch;
use super::module::HookPoint;
use super::runtime::Runtime;
use super::session::{Session, SessionEventKind};
use super::trace::ExecutionTrace;

pub(super) fn fixed_hook(point: HookPoint) -> Option<ModuleHookV1> {
    match point {
        HookPoint::TurnStart => Some(ModuleHookV1::SessionStart),
        HookPoint::BeforeModelCall => Some(ModuleHookV1::BeforeModelCall),
        HookPoint::AfterModelResponse => Some(ModuleHookV1::AfterModelCall),
        HookPoint::BeforeToolCall => Some(ModuleHookV1::BeforeToolCall),
        HookPoint::AfterToolResult => Some(ModuleHookV1::AfterToolCall),
        HookPoint::AfterTurn => Some(ModuleHookV1::OnIdle),
        HookPoint::BeforeFinalCommit => Some(ModuleHookV1::BeforeFinalCommit),
        HookPoint::OnError => Some(ModuleHookV1::OnError),
    }
}

impl Runtime {
    pub(super) fn has_automatic_after_tool_work(&self) -> bool {
        self.plugins.plugins().ids().any(|id| {
            self.plugins
                .state(id)
                .is_ok_and(|state| state == Lifecycle::Active)
                && self
                    .plugins
                    .task_scope(id)
                    .is_ok_and(|scope| scope.is_open() || scope.active_tasks() > 0)
                && self
                    .plugins
                    .plugins()
                    .get(id)
                    .is_ok_and(|plugin| plugin.hooks().contains(&ModuleHookV1::AfterToolCall))
        })
    }

    /// Sequential commit in dependency order; callbacks may finish concurrently.
    /// This never recursively emits hooks for the tools requested by a hook.
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn run_registered_hooks(
        &self,
        point: ModuleHookV1,
        session: &mut Session,
        request: RequestId,
        trace: TraceId,
        security: Option<&TurnSecurityContext>,
        candidate: Option<&NormalizedResponse>,
        tool_call: Option<&ToolCall>,
        tool_result: Option<&ToolResult>,
    ) -> RuntimeResult<()> {
        self.run_registered_hooks_for(
            point,
            session,
            request,
            trace,
            security,
            candidate,
            tool_call,
            tool_result,
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn run_registered_hooks_for(
        &self,
        point: ModuleHookV1,
        session: &mut Session,
        request: RequestId,
        trace: TraceId,
        security: Option<&TurnSecurityContext>,
        candidate: Option<&NormalizedResponse>,
        tool_call: Option<&ToolCall>,
        tool_result: Option<&ToolResult>,
        only: Option<&PluginId>,
    ) -> RuntimeResult<()> {
        let context = HookContextV1 {
            hook: point,
            session: session.id,
            request,
            trace,
            messages: session.messages.clone().into(),
            candidate: candidate.cloned(),
            tool_call: tool_call.cloned(),
            tool_result: tool_result.cloned(),
        };
        let mut tasks = tokio::task::JoinSet::new();
        let permits = Arc::new(tokio::sync::Semaphore::new(
            self.dispatcher.limits.hook_concurrency.max(1),
        ));
        let mut owners = Vec::new();
        for id in self.plugins.resolve_order()? {
            if let Some(selected) = only {
                if selected != &id {
                    continue;
                }
            } else if self
                .registered_behaviors
                .values()
                .any(|adapter| adapter.id() == &id)
            {
                continue;
            }
            if self.plugins.state(&id)? != Lifecycle::Active {
                continue;
            }
            let plugin = self.plugins.plugins().get(&id)?.clone();
            if !plugin.hooks().contains(&point) {
                continue;
            }
            // SessionStart means one attempt per module/session, not TurnStart.
            if point == ModuleHookV1::SessionStart && session.events.iter().any(|entry| {
                matches!(&entry.event, SessionEventKind::RegisteredHook { module, hook, .. } if module == &id && *hook == point)
            }) { continue; }
            let index = owners.len();
            owners.push(id.clone());
            let scope = self.plugins.task_scope(&id)?;
            let context = context.clone();
            let timeout = self.dispatcher.limits.hook_timeout;
            let permits = permits.clone();
            tasks.spawn(async move {
                let callback = async {
                    let _permit = permits
                        .acquire_owned()
                        .await
                        .expect("local semaphore stays open");
                    scope.run(plugin.on_hook(&context)).await
                };
                let result = tokio::time::timeout(timeout, callback).await;
                let result = match result {
                    Ok(Ok(Ok(calls))) => Ok(calls),
                    Ok(Ok(Err(_))) => Err("callback_error"),
                    Ok(Err(_)) => Err("cancelled"),
                    Err(_) => Err("timeout"),
                };
                (index, result)
            });
        }
        if owners.is_empty() {
            self.flush_module_events(session, request, trace)?;
            return self.validate_v1_request_boundary(point, session);
        }
        let mut outcomes = (0..owners.len())
            .map(|_| Err("panicked"))
            .collect::<Vec<_>>();
        while let Some(result) = tasks.join_next().await {
            if let Ok((index, outcome)) = result {
                outcomes[index] = outcome;
            }
        }
        for (module, outcome) in owners.into_iter().zip(outcomes) {
            let status = match outcome {
                Ok(calls) if calls.len() <= self.dispatcher.limits.max_hook_calls => {
                    // Validate the entire producer batch before any side effect.
                    match self.validate_hook_calls(session, &module, point, &calls) {
                        Ok(()) => {
                            if self
                                .execute_hook_calls(
                                    session, request, trace, security, &module, point, calls,
                                )
                                .await?
                            {
                                "completed"
                            } else {
                                "event_limit"
                            }
                        }
                        Err(_) => "invalid_batch",
                    }
                }
                Ok(_) => "call_limit",
                Err(status) => status,
            };
            session.record(
                request,
                trace,
                SessionEventKind::RegisteredHook {
                    module,
                    hook: point,
                    status: status.into(),
                },
                self.clock.as_ref(),
            );
        }
        self.flush_module_events(session, request, trace)?;
        self.validate_v1_request_boundary(point, session)?;
        self.sessions.save(session).await?;
        Ok(())
    }

    fn validate_v1_request_boundary(
        &self,
        point: ModuleHookV1,
        session: &Session,
    ) -> RuntimeResult<()> {
        if point == ModuleHookV1::BeforeModelCall && self.uses_v1_tool_events() {
            validate_closed_tool_context(&session.provider_view()).map_err(protocol_error)?;
        }
        Ok(())
    }

    fn validate_hook_calls(
        &self,
        session: &Session,
        module: &PluginId,
        point: ModuleHookV1,
        calls: &[HookToolRequest],
    ) -> Result<(), ()> {
        let mut seen = std::collections::BTreeMap::new();
        for request in calls {
            let call = authenticated_call(request);
            if let Some(existing) = seen.insert(request.execution, call.clone()) {
                if existing != call {
                    return Err(());
                }
            }
            let tool = self.resolve_tool(&call.name).ok_or(())?;
            if self
                .capabilities
                .registration(tool.id())
                .and_then(|entry| entry.module)
                .as_ref()
                != Some(module)
            {
                return Err(());
            }
            ToolEventV1::new(
                request.execution,
                ToolEventSourceV1::Module {
                    module_id: module.clone(),
                    hook: point,
                },
                call.clone(),
                ToolResult::ok(&call.id, serde_json::Value::Null),
            )
            .map_err(|_| ())?;
            if let Some(event) = recorded_event(session, request.execution) {
                if event.call() != &call
                    || event.source()
                        != &(ToolEventSourceV1::Module {
                            module_id: module.clone(),
                            hook: point,
                        })
                {
                    return Err(());
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn execute_hook_calls(
        &self,
        session: &mut Session,
        request: RequestId,
        trace: TraceId,
        security: Option<&TurnSecurityContext>,
        module: &PluginId,
        point: ModuleHookV1,
        calls: Vec<HookToolRequest>,
    ) -> RuntimeResult<bool> {
        for proposed in calls {
            if recorded_event(session, proposed.execution).is_some() {
                continue;
            }
            let call = authenticated_call(&proposed);
            // Pending facts are bounded before execution, including hooks in open
            // model call batches. Reject new work when there is no room to commit.
            let mut pending = pending_batch(session)?;
            let preview = ToolEventV1::new(
                proposed.execution,
                ToolEventSourceV1::Module {
                    module_id: module.clone(),
                    hook: point,
                },
                call.clone(),
                ToolResult::permanent_error(
                    &call.id,
                    "module result exceeds pending event budget; execution may have completed",
                )
                .with_name(&call.name),
            )
            .map_err(protocol_error)?;
            if pending.push(preview).is_err() {
                return Ok(false);
            }
            let session_id = session.id;
            let mut action_trace = ExecutionTrace::new(trace, session_id, request);
            let result = match self.dispatch_one_tool(&mut action_trace, session, &session_id, request, trace, &call, 0, security, false, None, true).await? {
                ToolDispatch::Result(result) => result,
                ToolDispatch::Pending(_) => ToolResult::permanent_error(&call.id, "automatic module call requires interactive approval; not executed; request the tool explicitly").with_name(&call.name),
            };
            for entry in action_trace.entries {
                self.emit_event(super::events::RuntimeEvent::Trace {
                    session: session.id,
                    trace,
                    at: entry.at,
                    event: entry.event,
                });
            }
            let audit_only = self.resolve_tool(&call.name).is_some_and(|tool| {
                tool.module_result_delivery(&result)
                    == apeireth_plugin::tool_execution::ModuleResultDelivery::AuditOnly
            });
            let source = ToolEventSourceV1::Module {
                module_id: module.clone(),
                hook: point,
            };
            let mut event =
                ToolEventV1::new(proposed.execution, source.clone(), call.clone(), result)
                    .map_err(protocol_error)?;
            let mut pending = pending_batch(session)?;
            if pending.push(event.clone()).is_err() {
                event = ToolEventV1::new(
                    proposed.execution,
                    source,
                    call.clone(),
                    ToolResult::permanent_error(
                        &call.id,
                        "module result exceeds pending event budget; execution may have completed",
                    )
                    .with_name(&call.name),
                )
                .map_err(protocol_error)?;
                pending.push(event.clone()).map_err(protocol_error)?;
            }
            session.record(
                request,
                trace,
                SessionEventKind::ToolEventRecorded { fact: event },
                self.clock.as_ref(),
            );
            if audit_only {
                session.record(
                    request,
                    trace,
                    SessionEventKind::ModuleToolEventObserved {
                        call_id: proposed.execution,
                    },
                    self.clock.as_ref(),
                );
            }
            // A crash cannot safely replay an uncommitted external effect. Save
            // completed facts before the next tool; this is not transactional I/O.
            self.sessions.save(session).await?;
        }
        Ok(true)
    }

    /// Results from Before/AfterToolCall wait until all model calls are answered.
    fn flush_module_events(
        &self,
        session: &mut Session,
        request: RequestId,
        trace: TraceId,
    ) -> RuntimeResult<()> {
        let mut batch = pending_batch(session)?;
        if batch.events().is_empty() {
            return Ok(());
        }
        if validate_closed_tool_context(&session.messages).is_err() {
            return Ok(());
        }
        let mut view = session.provider_view();
        let original = view.len();
        batch
            .append_to_closed_context(&mut view)
            .map_err(protocol_error)?;
        session.extend(view.into_iter().skip(original), self.clock.as_ref());
        for event in batch.events() {
            session.record(
                request,
                trace,
                SessionEventKind::ToolEventProjected {
                    call_id: event.call_id(),
                },
                self.clock.as_ref(),
            );
        }
        Ok(())
    }

    pub(super) fn record_model_tool_event(
        &self,
        session: &mut Session,
        request: RequestId,
        trace: TraceId,
        call: &ToolCall,
        result: &ToolResult,
    ) {
        if !self.uses_v1_tool_events() {
            return;
        }
        let owner = self
            .capabilities
            .registration_by_name(&call.name)
            .and_then(|entry| entry.module);
        match ToolEventV1::new(
            TaskId::new(),
            ToolEventSourceV1::Model { module_id: owner },
            call.clone(),
            result.clone(),
        ) {
            Ok(event) => session.record(
                request,
                trace,
                SessionEventKind::ToolEventRecorded { fact: event },
                self.clock.as_ref(),
            ),
            Err(_) => session.record(
                request,
                trace,
                SessionEventKind::ExecutionFailed {
                    phase: "tool_event_rejected".into(),
                    error: "invalid model tool identity or arguments".into(),
                },
                self.clock.as_ref(),
            ),
        }
    }

    /// Host lifecycle boundary, under the same session lock as turns/approvals.
    pub async fn end_session(
        &self,
        id: SessionId,
        security: Option<&TurnSecurityContext>,
    ) -> RuntimeResult<()> {
        let lock = self.session_locks.acquire(id).await;
        let _guard = lock.lock().await;
        let mut session = self.sessions.load_or_create(id).await?;
        if session
            .events
            .iter()
            .any(|entry| matches!(entry.event, SessionEventKind::SessionEnded))
        {
            return Ok(());
        }
        if session.active_approval_id.is_some()
            || validate_closed_tool_context(&session.messages).is_err()
        {
            return Err(RuntimeError::misconfigured(
                "cannot end a session with unresolved tool calls or approval",
            ));
        }
        let request = RequestId::new();
        let trace = TraceId::new();
        self.run_registered_hooks(
            ModuleHookV1::SessionEnd,
            &mut session,
            request,
            trace,
            security,
            None,
            None,
            None,
        )
        .await?;
        session.record(
            request,
            trace,
            SessionEventKind::SessionEnded,
            self.clock.as_ref(),
        );
        self.sessions.save(&session).await?;
        Ok(())
    }
}

fn authenticated_call(request: &HookToolRequest) -> ToolCall {
    let mut call = request.call.clone();
    call.id = format!("module_{}", request.execution);
    call
}

fn recorded_event(session: &Session, id: TaskId) -> Option<&ToolEventV1> {
    session.events.iter().find_map(|entry| match &entry.event {
        SessionEventKind::ToolEventRecorded { fact: event } if event.call_id() == id => Some(event),
        _ => None,
    })
}

fn pending_batch(session: &Session) -> RuntimeResult<ToolEventBatch> {
    let projected: BTreeSet<TaskId> = session
        .events
        .iter()
        .filter_map(|entry| match entry.event {
            SessionEventKind::ToolEventProjected { call_id }
            | SessionEventKind::ModuleToolEventObserved { call_id } => Some(call_id),
            _ => None,
        })
        .collect();
    let mut batch = ToolEventBatch::new(ToolEventLimits::default());
    for entry in &session.events {
        if let SessionEventKind::ToolEventRecorded { fact: event } = &entry.event {
            if matches!(event.source(), ToolEventSourceV1::Module { .. })
                && !projected.contains(&event.call_id())
            {
                batch.push(event.clone()).map_err(protocol_error)?;
            }
        }
    }
    Ok(batch)
}

fn protocol_error(error: apeireth_protocol::canonical::ToolEventError) -> RuntimeError {
    RuntimeError::misconfigured(format!("module tool event: {error}"))
}
