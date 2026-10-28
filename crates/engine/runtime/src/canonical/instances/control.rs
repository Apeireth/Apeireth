//! Bounded lifecycle controls over the existing module registry.
use super::*;
impl AgentRuntime {
    pub async fn stop_module(&self, id: &PluginId) -> AgentResult<()> {
        let scope = {
            let kernel = self.inner.kernel.read().await;
            kernel
                .cancel_module(id)
                .map_err(|e| error(AgentErrorCode::NotFound, e.to_string()))?;
            kernel
                .plugins
                .task_scope(id)
                .map_err(|e| error(AgentErrorCode::NotFound, e.to_string()))?
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
            .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))
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
