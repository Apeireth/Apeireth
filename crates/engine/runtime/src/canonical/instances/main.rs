//! Rich Host entry over one ManagedAgent, preserving the caller's Session and ABI.
use super::*;
use crate::canonical::approval::ApprovalDecision;
use crate::canonical::ApprovalResolution;
use apeireth_core::kernel::ApprovalId;

type DeltaSink = Arc<dyn Fn(String) + Send + Sync>;
struct MainSession {
    turn: Arc<tokio::sync::Mutex<()>>,
    status: AgentStatus,
}
pub(in crate::canonical) struct MainExecution {
    pub(super) agent: Arc<ManagedAgent>,
    sessions: Mutex<BTreeMap<SessionId, MainSession>>,
}
impl AgentRuntime {
    pub(in crate::canonical) fn has_managed_main(&self) -> bool {
        self.inner.main.get().is_some()
    }
    /// Consume this Host into a compatible observation/execution facade. The
    /// mutable registry stays in this AgentRuntime; views cannot own lifecycle.
    pub async fn into_main_runtime(self, config: AgentConfig) -> RuntimeResult<Runtime> {
        self.bind_main(config, true).await
    }
    /// Host opt-in retaining its existing SessionQuota policy, including unknown
    /// provider usage. Finite instance and shared runtime budgets stay unchanged.
    pub async fn into_main_runtime_with_session_quotas(
        self,
        config: AgentConfig,
    ) -> RuntimeResult<Runtime> {
        self.bind_main(config, false).await
    }
    async fn bind_main(self, config: AgentConfig, instance_budget: bool) -> RuntimeResult<Runtime> {
        if self.has_managed_main() {
            return Err(super::super::RuntimeError::misconfigured(
                "main already bound",
            ));
        }
        let agent = create(
            &self.inner,
            AgentOwner::Host("main".into()),
            self.inner.scope.clone(),
            config,
            None,
            None,
        )
        .await?;
        if instance_budget {
            self.inner
                .usage
                .bind_owner_budget(agent.owner.clone(), (&agent.config.budget).into());
        } else {
            self.inner.usage.use_session_quotas(agent.session);
        }
        self.inner
            .main
            .set(MainExecution {
                agent,
                sessions: Mutex::new(BTreeMap::new()),
            })
            .map_err(|_| super::super::RuntimeError::misconfigured("main already bound"))?;
        let mut view = self.inner.kernel.read().await.execution_view();
        view.instance_host = Some(self);
        Ok(view)
    }
}
impl Runtime {
    /// The desktop facade's one managed identity. Compatibility builds have none.
    pub fn managed_main(&self) -> Option<AgentSnapshot> {
        self.managed_execution().map(|main| main.agent.snapshot())
    }
    /// Existing lifecycle authority, shared by all facade execution entries.
    pub fn agent_runtime(&self) -> Option<&AgentRuntime> {
        self.instance_host
            .as_ref()
            .filter(|host| host.has_managed_main())
    }
    pub(in crate::canonical) fn managed_execution(&self) -> Option<&MainExecution> {
        self.instance_host
            .as_ref()
            .and_then(|host| host.inner.main.get())
    }
}
impl MainExecution {
    pub(super) fn status(&self) -> AgentStatus {
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if sessions.values().any(|s| s.status == AgentStatus::Running) {
            AgentStatus::Running
        } else if sessions
            .values()
            .any(|s| s.status == AgentStatus::WaitingApproval)
        {
            AgentStatus::WaitingApproval
        } else {
            AgentStatus::Ready
        }
    }
    fn session(
        &self,
        session: SessionId,
        inner: &Arc<Inner>,
    ) -> RuntimeResult<Arc<tokio::sync::Mutex<()>>> {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = sessions.get(&session) {
            return Ok(entry.turn.clone());
        }
        if session != self.agent.session {
            // A session owned by another instance cannot be adopted. Durable
            // history is loaded by the kernel, including its principal ceiling.
            inner
                .usage
                .attach_main_session(session, self.agent.session)?;
            let ceiling = inner.dispatcher.scope(session);
            inner.dispatcher.bind_instance(
                session,
                self.agent.id,
                self.agent.config.principal.clone(),
                self.agent.scope.clone(),
            );
            inner.dispatcher.set_scope(
                session,
                ToolScope::Only(
                    self.agent
                        .config
                        .tools
                        .iter()
                        .filter(|id| ceiling.allows(id))
                        .cloned()
                        .collect(),
                ),
            );
        }
        let turn = Arc::new(tokio::sync::Mutex::new(()));
        sessions.insert(
            session,
            MainSession {
                turn: turn.clone(),
                status: AgentStatus::Ready,
            },
        );
        Ok(turn)
    }
    fn start(&self, session: SessionId) -> RuntimeResult<MainTurn<'_>> {
        let state = self.agent.state.lock().unwrap_or_else(|e| e.into_inner());
        if !self.agent.scope.is_open() || state.0 != AgentStatus::Ready {
            return Err(error(AgentErrorCode::NotReady, "main is paused or stopped").into());
        }
        self.sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&session)
            .expect("session admitted")
            .status = AgentStatus::Running;
        Ok(MainTurn {
            main: self,
            session,
            finished: false,
        })
    }
    pub(in crate::canonical) async fn execute(
        &self,
        request: TurnRequest,
        sink: Option<DeltaSink>,
    ) -> RuntimeResult<TurnOutcome> {
        self.run(request.session, MainOperation::Turn(request, sink))
            .await
            .map(|result| match result {
                MainResult::Turn(outcome) => outcome,
                MainResult::Approval(_) => unreachable!(),
            })
    }
    pub(in crate::canonical) async fn resolve(
        &self,
        session: SessionId,
        id: ApprovalId,
        decision: ApprovalDecision,
    ) -> RuntimeResult<ApprovalResolution> {
        self.run(session, MainOperation::Approval(id, decision))
            .await
            .map(|result| match result {
                MainResult::Approval(outcome) => outcome,
                MainResult::Turn(_) => unreachable!(),
            })
    }
    async fn run(&self, session: SessionId, operation: MainOperation) -> RuntimeResult<MainResult> {
        let inner = self
            .agent
            .inner
            .upgrade()
            .ok_or_else(|| error(AgentErrorCode::OwnerStopped, "runtime dropped"))?;
        if !self.agent.scope.is_open() {
            return Err(error(AgentErrorCode::OwnerStopped, "main stopped").into());
        }
        let lock = self.session(session, &inner)?;
        let work = async {
            let _session = lock.lock().await;
            let mut turn = self.start(session)?;
            let _permit = inner
                .turns
                .acquire()
                .await
                .map_err(|_| error(AgentErrorCode::OwnerStopped, "runtime stopped"))?;
            // Release the control lock before any Hook, Provider or continuation.
            let kernel = inner.kernel.read().await.execution_view();
            let result = match operation {
                MainOperation::Turn(mut request, sink) => {
                    request.model = request.model.or_else(|| self.agent.config.model.clone());
                    kernel
                        .execute_outcome_with_sink(request, sink)
                        .await
                        .map(MainResult::Turn)
                }
                MainOperation::Approval(id, decision) => kernel
                    .resolve_approval(session, id, decision)
                    .await
                    .map(MainResult::Approval),
            };
            self.agent.state.lock().unwrap_or_else(|e| e.into_inner()).1 =
                result.as_ref().err().map(ToString::to_string);
            let pending = !kernel
                .pending_approvals(session)
                .await
                .unwrap_or_default()
                .is_empty()
                || matches!(
                    &result,
                    Ok(MainResult::Turn(TurnOutcome::PendingApproval(_)))
                        | Ok(MainResult::Approval(ApprovalResolution::Resumed(
                            TurnOutcome::PendingApproval(_)
                        )))
                );
            turn.finish(if pending {
                AgentStatus::WaitingApproval
            } else {
                AgentStatus::Ready
            });
            result
        };
        tokio::time::timeout(
            Duration::from_millis(self.agent.config.budget.turn_timeout_ms),
            self.agent.scope.run(work),
        )
        .await
        .map_err(|_| error(AgentErrorCode::Timeout, "main turn deadline exceeded"))?
        .map_err(|_| error(AgentErrorCode::Cancelled, "main stopped"))?
    }
}
enum MainOperation {
    Turn(TurnRequest, Option<DeltaSink>),
    Approval(ApprovalId, ApprovalDecision),
}
enum MainResult {
    Turn(TurnOutcome),
    Approval(ApprovalResolution),
}
struct MainTurn<'a> {
    main: &'a MainExecution,
    session: SessionId,
    finished: bool,
}
impl MainTurn<'_> {
    fn finish(&mut self, status: AgentStatus) {
        self.main
            .sessions
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&self.session)
            .expect("session admitted")
            .status = status;
        self.finished = true;
    }
}
impl Drop for MainTurn<'_> {
    fn drop(&mut self) {
        if !self.finished {
            self.finish(AgentStatus::Failed);
        }
    }
}
