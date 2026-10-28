//! Normalized, bounded tasks over ManagedAgent and the canonical execution path.
use super::*;

impl ManagedAgent {
    pub(super) async fn run_task(
        &self,
        message: String,
        task: Option<AgentTask>,
    ) -> AgentResult<(AgentReply, Option<AgentTaskReply>)> {
        if message.len() > 128 * 1024 {
            return Err(error(
                AgentErrorCode::InvalidConfig,
                "agent task exceeds input limit",
            ));
        }
        let normalized_task = task.is_some();
        if let Some(task) = &task {
            if task.max_rounds == 0 {
                return Err(error(
                    AgentErrorCode::RoundLimitExceeded,
                    "zero task rounds",
                ));
            }
            if serde_json::to_vec(&task.messages).map_or(true, |v| {
                v.len()
                    .saturating_add(task.system.as_ref().map_or(0, String::len))
                    > 128 * 1024
            }) {
                return Err(error(
                    AgentErrorCode::InvalidConfig,
                    "normalized task exceeds input limit",
                ));
            }
        }
        let timeout = task
            .as_ref()
            .and_then(|t| t.timeout)
            .unwrap_or(Duration::from_millis(self.config.budget.turn_timeout_ms))
            .min(Duration::from_millis(self.config.budget.turn_timeout_ms));
        let inner = self
            .inner
            .upgrade()
            .ok_or_else(|| error(AgentErrorCode::OwnerStopped, "runtime dropped"))?;
        struct Queued<'a>(&'a std::sync::atomic::AtomicUsize);
        impl Drop for Queued<'_> {
            fn drop(&mut self) {
                self.0.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
            }
        }
        let queued = self
            .queued
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let _queued = Queued(&self.queued);
        if queued >= 4 {
            return Err(error(
                AgentErrorCode::LimitExceeded,
                "agent task queue limit reached",
            ));
        }
        let work = async {
            let _turn = self.turn.lock().await;
            {
                let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
                if !matches!(state.0, AgentStatus::Ready) {
                    return Err(error(
                        AgentErrorCode::Paused,
                        "instance must be ready before accepting a turn",
                    ));
                }
                state.0 = AgentStatus::Running;
            }
            let mut running = Running {
                agent: self,
                completed: false,
            };
            // A synchronous child borrows the admitted root turn's slot. It has
            // its own Session/turn lock and still shares Provider concurrency.
            let _permit = if self.execution.is_some() {
                None
            } else {
                Some(
                    inner
                        .turns
                        .acquire()
                        .await
                        .map_err(|_| error(AgentErrorCode::OwnerStopped, "runtime stopped"))?,
                )
            };
            let mut kernel = match &self.execution {
                Some(view) => view.execution_view(),
                None => inner.kernel.read().await.execution_view(),
            };
            if let Some(task) = task {
                kernel.config.max_rounds = task.max_rounds;
                kernel.v1_protocol = true;
                kernel.task = Some(Arc::new(TaskExecution::new(task)));
            }
            let mut request = TurnRequest::new(self.session, message);
            request.model = kernel
                .task
                .as_ref()
                .and_then(|t| t.input.model.clone())
                .or_else(|| self.config.model.clone());
            request.security_context = self.security.clone();
            let result = kernel.execute_outcome(request).await;
            running.completed = true;
            match result {
                Ok(TurnOutcome::Completed(reply)) => {
                    *self.state.lock().unwrap_or_else(|e| e.into_inner()) =
                        (AgentStatus::Ready, None);
                    let full = if let Some(task) = &kernel.task {
                        let session = self
                            .sessions
                            .load(&self.session)
                            .await
                            .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))?
                            .ok_or_else(|| {
                                error(AgentErrorCode::Provider, "task session missing")
                            })?;
                        let progress = task.progress.lock().unwrap_or_else(|e| e.into_inner());
                        let response = progress.response.clone().ok_or_else(|| {
                            error(AgentErrorCode::Provider, "task has no provider response")
                        })?;
                        Some(AgentTaskReply {
                            request: reply.request,
                            response,
                            served_by: reply.served_by.clone(),
                            rounds: reply.rounds,
                            usage: progress.usage.clone(),
                            tool_results: session.events.iter().filter(|e| e.request == reply.request).filter_map(|e| {
                                match &e.event { crate::canonical::session::SessionEventKind::ToolEventRecorded { fact } => Some(fact.result().clone()), _ => None }
                            }).collect(),
                        })
                    } else {
                        None
                    };
                    Ok((
                        AgentReply {
                            text: reply.text,
                            request: reply.request,
                            usage: inner.usage.snapshot(self.session),
                        },
                        full,
                    ))
                }
                Ok(TurnOutcome::PendingApproval(_)) => {
                    *self.state.lock().unwrap_or_else(|e| e.into_inner()) =
                        (AgentStatus::WaitingApproval, None);
                    Err(error(
                        AgentErrorCode::PendingApproval,
                        "host approval required; no automatic approval granted",
                    ))
                }
                Err(e) => {
                    let code = match &e {
                        crate::canonical::RuntimeError::AgentTask(error) => error.code,
                        crate::canonical::RuntimeError::Denied { hook, .. }
                            if hook == "instance_budget" =>
                        {
                            AgentErrorCode::BudgetExceeded
                        }
                        crate::canonical::RuntimeError::Denied { .. } if normalized_task => {
                            AgentErrorCode::Denied
                        }
                        crate::canonical::RuntimeError::ApprovalRequired { .. }
                            if normalized_task =>
                        {
                            AgentErrorCode::ApprovalRequired
                        }
                        crate::canonical::RuntimeError::RoundLimitExceeded { .. }
                            if normalized_task =>
                        {
                            AgentErrorCode::RoundLimitExceeded
                        }
                        _ => AgentErrorCode::Provider,
                    };
                    *self.state.lock().unwrap_or_else(|e| e.into_inner()) =
                        (AgentStatus::Failed, Some(e.to_string()));
                    Err(error(code, e.to_string()))
                }
            }
        };
        match tokio::time::timeout(timeout, self.scope.run(work)).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
                self.scope.close();
                Err(error(AgentErrorCode::Cancelled, "agent cancelled"))
            }
            Err(_) => {
                self.scope.close();
                Err(error(
                    if normalized_task {
                        AgentErrorCode::Timeout
                    } else {
                        AgentErrorCode::Cancelled
                    },
                    "agent turn deadline exceeded",
                ))
            }
        }
    }
}

/// A task's projection state; accounting remains exclusively at Provider/Dispatcher.
pub(in crate::canonical) struct TaskExecution {
    pub input: AgentTask,
    progress: Mutex<TaskProgress>,
}
struct TaskProgress {
    pub response: Option<apeireth_protocol::canonical::NormalizedResponse>,
    pub usage: apeireth_protocol::canonical::NormalizedUsage,
}
impl TaskExecution {
    fn new(input: AgentTask) -> Self {
        Self {
            input,
            progress: Mutex::new(TaskProgress {
                response: None,
                usage: apeireth_protocol::canonical::NormalizedUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    prompt_cache_hit_tokens: Some(0),
                },
            }),
        }
    }
    pub fn observe(&self, response: &apeireth_protocol::canonical::NormalizedResponse) {
        let mut progress = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        let usage = &mut progress.usage;
        usage.prompt_tokens = usage
            .prompt_tokens
            .saturating_add(response.usage.prompt_tokens);
        usage.completion_tokens = usage
            .completion_tokens
            .saturating_add(response.usage.completion_tokens);
        usage.total_tokens = usage
            .total_tokens
            .saturating_add(response.usage.total_tokens);
        usage.prompt_cache_hit_tokens = match (
            usage.prompt_cache_hit_tokens,
            response.usage.prompt_cache_hit_tokens,
        ) {
            (Some(a), Some(b)) => Some(a.saturating_add(b)),
            _ => None,
        };
        progress.response = Some(response.clone());
    }
}
impl Runtime {
    pub(in crate::canonical) fn isolated_task(&self) -> bool {
        self.task
            .as_ref()
            .is_some_and(|t| t.input.mode == AgentExecutionMode::Isolated)
    }
    /// Instance audit view, including private child executions (no transcript data).
    pub fn agent_instances(&self) -> Vec<AgentSnapshot> {
        self.instances
            .upgrade()
            .map(|inner| {
                inner
                    .instances
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .values()
                    .map(|a| a.snapshot())
                    .collect()
            })
            .unwrap_or_default()
    }
    pub(in crate::canonical) async fn spawn_side_agent(
        &self,
        module: &str,
        parent_session: SessionId,
        mut config: AgentConfig,
        security: Option<TurnSecurityContext>,
    ) -> AgentResult<SideAgent> {
        let inner = self
            .instances
            .upgrade()
            .ok_or_else(|| error(AgentErrorCode::NotReady, "instance service is not bound"))?;
        let parent_scope = self
            .dispatcher
            .instance_scope(parent_session)
            .unwrap_or_else(|| inner.scope.clone());
        let parent = self.dispatcher.agent(parent_session);
        if inner
            .instances
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(&parent)
        {
            config.parent = Some(parent);
        }
        config.principal = self.dispatcher.principal(parent_session);
        config
            .tools
            .retain(|id| self.dispatcher.allows(parent_session, id));
        let (owner, scope, grant) = if let Some(adapter) = self.registered_behaviors.get(module) {
            let definition = adapter.id().clone();
            let instance = self
                .plugins
                .instance_id(&definition)
                .map_err(|e| error(AgentErrorCode::OwnerStopped, e.to_string()))?;
            let mut grant = inner
                .policy
                .modules
                .get(&definition)
                .cloned()
                .ok_or_else(|| {
                    error(
                        AgentErrorCode::ScopeDenied,
                        "host did not grant module agent creation",
                    )
                })?;
            config.tools.retain(|id| grant.tools.contains(id));
            if config
                .principal
                .as_ref()
                .is_some_and(|p| !grant.principals.contains(p))
            {
                return Err(error(
                    AgentErrorCode::NamespaceDenied,
                    "parent principal is outside module grant",
                ));
            }
            grant.security = security.or(grant.security);
            config.budget = bounded_budget(&inner.policy.max_budget, &grant.budget);
            let module_scope = self
                .plugins
                .task_scope(&definition)
                .map_err(|e| error(AgentErrorCode::OwnerStopped, e.to_string()))?;
            (
                AgentOwner::Module {
                    definition,
                    instance,
                },
                ModuleTaskScope::joined(vec![module_scope, parent_scope]),
                Some(grant),
            )
        } else {
            config.budget = inner.policy.max_budget.clone();
            (
                AgentOwner::Host(format!("compat.{module}")),
                parent_scope,
                Some(ModuleAgentGrant {
                    budget: inner.policy.max_budget.clone(),
                    security,
                    max_instances: inner.policy.max_instances,
                    max_depth: crate::canonical::module::DEFAULT_MAX_INVOCATION_DEPTH,
                    ..Default::default()
                }),
            )
        };
        let mut view = self.execution_view();
        view.v1_protocol = true;
        view.task = None;
        view.event_sink = Arc::new(RwLock::new(Arc::new(
            crate::canonical::events::NoopRuntimeEventSink,
        )));
        let agent = create(&inner, owner, scope, config, grant, Some(Arc::new(view))).await?;
        self.dispatcher.bind_parent(agent.session, parent_session);
        Ok(SideAgent {
            scope: agent.scope.clone(),
            agent,
        })
    }
}
pub(in crate::canonical) struct SideAgent {
    pub agent: Arc<dyn AgentInstance>,
    scope: ModuleTaskScope,
}
impl Drop for SideAgent {
    fn drop(&mut self) {
        self.scope.close();
    }
}
fn bounded_budget(a: &AgentBudget, b: &AgentBudget) -> AgentBudget {
    AgentBudget {
        max_requests: a.max_requests.min(b.max_requests),
        max_tokens: a.max_tokens.min(b.max_tokens),
        max_tool_calls: a.max_tool_calls.min(b.max_tool_calls),
        max_cost_micros: match (a.max_cost_micros, b.max_cost_micros) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        },
        turn_timeout_ms: a.turn_timeout_ms.min(b.turn_timeout_ms),
    }
}
