//! Existing lifecycle owner, with cancellable and bounded control-plane waits.
use super::*;

impl PluginManager {
    /// Dependents must stop before their dependencies. Cancel, drain, then cleanup.
    pub async fn stop(&mut self, id: &PluginId) -> PluginResult<()> {
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
