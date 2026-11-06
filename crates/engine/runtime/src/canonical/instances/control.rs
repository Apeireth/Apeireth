//! Bounded lifecycle controls over the existing module registry.
use super::*;
impl AgentRuntime {
    pub async fn stop_module(&self, id: &PluginId) -> AgentResult<()> {
        let scope = {
            let mut kernel =
                tokio::time::timeout(self.inner.policy.stop_timeout, self.inner.kernel.write())
                    .await
                    .map_err(|_| {
                        error(AgentErrorCode::NotReady, "module control deadline exceeded")
                    })?;
            kernel
                .plugins
                .validate_stop(id)
                .map_err(|e| error(AgentErrorCode::NotReady, e.to_string()))?;
            if kernel
                .plugins
                .plugins()
                .get(id)
                .map_err(|e| error(AgentErrorCode::NotFound, e.to_string()))?
                .manifest()
                .capabilities
                .iter()
                .any(|c| c.kind == apeireth_plugin::CapabilityKind::Provider)
            {
                return Err(error(
                    AgentErrorCode::InvalidConfig,
                    "stop provider modules through runtime shutdown",
                ));
            }
            kernel
                .plugins
                .begin_stop(id)
                .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))?
        };
        tokio::time::timeout(self.inner.policy.stop_timeout, scope.close_and_wait())
            .await
            .map_err(|_| {
                error(
                    AgentErrorCode::NotReady,
                    "module drain deadline exceeded; owner retained",
                )
            })?;
        let mut kernel =
            tokio::time::timeout(self.inner.policy.stop_timeout, self.inner.kernel.write())
                .await
                .map_err(|_| {
                    error(
                        AgentErrorCode::NotReady,
                        "module cancelled; control plane is not quiescent",
                    )
                })?;
        tokio::time::timeout(self.inner.policy.stop_timeout, kernel.stop_module(id))
            .await
            .map_err(|_| {
                error(
                    AgentErrorCode::NotReady,
                    "module cleanup deadline exceeded; owner retained",
                )
            })?
            .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))?;
        drop(kernel);
        self.inner.record_interrupted_tasks().await
    }
    pub async fn unregister_module(&self, id: &PluginId) -> AgentResult<()> {
        self.stop_module(id).await?;
        let mut kernel =
            tokio::time::timeout(self.inner.policy.stop_timeout, self.inner.kernel.write())
                .await
                .map_err(|_| error(AgentErrorCode::NotReady, "control plane is not quiescent"))?;
        kernel
            .unregister_module(id)
            .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))
    }
    pub async fn shutdown(&self) -> AgentResult<()> {
        self.inner.scope.close();
        tokio::time::timeout(
            self.inner.policy.stop_timeout,
            self.inner.scope.close_and_wait(),
        )
        .await
        .map_err(|_| {
            error(
                AgentErrorCode::NotReady,
                "runtime draining; resources remain owned",
            )
        })?;
        self.inner.record_interrupted_tasks().await?;
        let mut kernel =
            tokio::time::timeout(self.inner.policy.stop_timeout, self.inner.kernel.write())
                .await
                .map_err(|_| {
                    error(
                        AgentErrorCode::NotReady,
                        "runtime control plane is not quiescent",
                    )
                })?;
        let failures = tokio::time::timeout(self.inner.policy.stop_timeout, kernel.shutdown())
            .await
            .map_err(|_| error(AgentErrorCode::NotReady, "module cleanup deadline exceeded"))?;
        if failures.is_empty() {
            Ok(())
        } else {
            Err(error(
                AgentErrorCode::Provider,
                failures
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            ))
        }
    }
}
impl Drop for Inner {
    fn drop(&mut self) {
        self.scope.close();
    }
}
impl AgentRuntime {
    /// Host approval entry; the instance retains its original scope and resource identity.
    pub async fn resolve_agent_approval(
        &self,
        id: AgentId,
        approval: apeireth_core::kernel::ApprovalId,
        decision: crate::canonical::approval::ApprovalDecision,
    ) -> AgentResult<AgentReply> {
        let agent = self
            .inner
            .instances
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned()
            .ok_or_else(|| error(AgentErrorCode::NotFound, "unknown instance"))?;
        let work = async {
            let _turn = agent.turn.lock().await;
            if agent.snapshot().status != AgentStatus::WaitingApproval {
                return Err(error(
                    AgentErrorCode::NotReady,
                    "instance is not waiting for approval",
                ));
            }
            let _permit = self
                .inner
                .turns
                .acquire()
                .await
                .map_err(|_| error(AgentErrorCode::OwnerStopped, "runtime stopped"))?;
            let kernel = self.inner.kernel.read().await.execution_view();
            match kernel
                .resolve_approval(agent.session, approval, decision)
                .await
            {
                Ok(crate::ApprovalResolution::Resumed(TurnOutcome::Completed(reply))) => {
                    *agent.state.lock().unwrap_or_else(|e| e.into_inner()) =
                        (AgentStatus::Ready, None);
                    Ok(AgentReply {
                        text: reply.text,
                        request: reply.request,
                        usage: agent.usage.snapshot(agent.session),
                    })
                }
                Ok(_) => Err(error(
                    AgentErrorCode::PendingApproval,
                    "approval did not complete this turn",
                )),
                Err(e) => {
                    *agent.state.lock().unwrap_or_else(|e| e.into_inner()) =
                        (AgentStatus::Failed, Some(e.to_string()));
                    Err(error(AgentErrorCode::Provider, e.to_string()))
                }
            }
        };
        match tokio::time::timeout(
            Duration::from_millis(agent.config.budget.turn_timeout_ms),
            agent.scope.run(work),
        )
        .await
        {
            Ok(Ok(result)) => result,
            _ => {
                agent.scope.close();
                Err(error(
                    AgentErrorCode::Cancelled,
                    "approval continuation cancelled or deadline exceeded",
                ))
            }
        }
    }
}
