//! Existing lifecycle owner, with cancellable and bounded control-plane waits.
use super::*;

impl PluginManager {
    /// Dependents must stop before their dependencies. Cancel, drain, then cleanup.
    pub async fn stop(&mut self, id: &PluginId) -> PluginResult<()> {
        self.validate_stop(id)?;
        self.cancel(id)?;
        let state = self.state(id)?;
        let scope = self.task_scope(id)?;

        if state == Lifecycle::Registered {
            scope.close();
            return self.plugins.transition(id, Lifecycle::Stopped);
        }
        if state == Lifecycle::Stopped {
            return Ok(());
        }
        if !matches!(state, Lifecycle::Failed | Lifecycle::Stopping) {
            self.plugins.transition(id, Lifecycle::Stopping)?;
        }
        scope.close_and_wait().await;
        if self.plugins.entry(id).expect("registered").needs_cleanup {
            let plugin = Arc::clone(self.plugins.get(id)?);
            match std::panic::AssertUnwindSafe(plugin.shutdown())
                .catch_unwind()
                .await
            {
                Ok(Ok(())) => {
                    self.plugins
                        .entry_mut(id)
                        .expect("registered")
                        .needs_cleanup = false
                }
                result => {
                    if state != Lifecycle::Failed {
                        self.plugins.transition(id, Lifecycle::Failed)?;
                    }
                    let reason = match result {
                        Ok(Err(error)) => error.to_string(),
                        _ => "shutdown panicked".into(),
                    };
                    return Err(self.failure(id, "shutdown", reason));
                }
            }
        }
        self.start_order.retain(|started| started != id);
        if state != Lifecycle::Failed {
            self.plugins.transition(id, Lifecycle::Stopped)?;
        }
        Ok(())
    }

    /// Publish Stopping before waiting for native drain. Failed/Stopped remain terminal.
    pub fn begin_stop(
        &mut self,
        id: &PluginId,
    ) -> PluginResult<crate::task_scope::ModuleTaskScope> {
        self.validate_stop(id)?;
        self.cancel(id)?;
        if matches!(
            self.state(id)?,
            Lifecycle::Initializing
                | Lifecycle::Initialized
                | Lifecycle::Starting
                | Lifecycle::Active
        ) {
            self.plugins.transition(id, Lifecycle::Stopping)?;
        }
        self.task_scope(id)
    }

    /// Read-only preflight before cancellation; a refused stop has no effects.
    pub fn validate_stop(&self, id: &PluginId) -> PluginResult<()> {
        self.require_owner()?;
        self.state(id)?;
        for other in self.plugins.ids() {
            if self
                .plugins
                .get(other)?
                .manifest()
                .dependencies
                .contains(id)
                && matches!(
                    self.state(other)?,
                    Lifecycle::Initializing
                        | Lifecycle::Initialized
                        | Lifecycle::Starting
                        | Lifecycle::Active
                        | Lifecycle::Stopping
                )
            {
                return Err(PluginError::PluginFailed {
                    plugin: id.clone(),
                    phase: "stop",
                    reason: format!("dependent {other} must stop first"),
                });
            }
        }
        Ok(())
    }

    /// Timeout leaves the scope closed and state Stopping; cleanup can be retried.
    /// The host supplies its timer so foundation has no executor dependency.
    pub async fn stop_until(
        &mut self,
        id: &PluginId,
        deadline: impl std::future::Future<Output = ()>,
    ) -> PluginResult<()> {
        let completed = {
            match futures::future::select(Box::pin(self.stop(id)), Box::pin(deadline)).await {
                futures::future::Either::Left((result, _)) => Some(result),
                futures::future::Either::Right(((), pending)) => {
                    drop(pending);
                    None
                }
            }
        };
        completed.unwrap_or_else(|| {
            Err(self.failure(
                id,
                "stop",
                "drain or cleanup deadline exceeded; resources remain owned",
            ))
        })
    }

    /// Reverse dependency cleanup, continuing after individual failures.
    pub async fn shutdown_all(&mut self) -> Vec<PluginError> {
        let order = self
            .resolve_order()
            .unwrap_or_else(|_| self.plugins.ids().cloned().collect());
        let mut failures = Vec::new();
        for id in order.into_iter().rev() {
            if let Err(error) = self.stop(&id).await {
                failures.push(error);
            }
        }
        failures
    }
}
