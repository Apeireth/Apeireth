//! Private task interruption facts at explicit, successful drain boundaries.
use super::*;
use crate::canonical::session::SessionEventKind;

impl Inner {
    pub(super) async fn record_interrupted_tasks(&self) -> AgentResult<()> {
        let agents: Vec<_> = self
            .instances
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|a| !a.scope.is_open() && a.scope.active_tasks() == 0)
            .cloned()
            .collect();
        for agent in agents {
            agent.record_interrupted_task().await?;
        }
        Ok(())
    }
}

impl ManagedAgent {
    pub(super) async fn record_interrupted_task(&self) -> AgentResult<()> {
        let timeout = self
            .inner
            .upgrade()
            .map(|inner| inner.policy.stop_timeout)
            .unwrap_or(Duration::from_secs(5));
        tokio::time::timeout(timeout, self.persist_interruption())
            .await
            .map_err(|_| {
                error(
                    AgentErrorCode::NotReady,
                    "task interruption audit deadline exceeded; retry stop",
                )
            })?
    }

    async fn persist_interruption(&self) -> AgentResult<()> {
        // The turn future (and its session lock) must be gone before loading the
        // checkpoint. Failed drains never publish a successful interruption fact.
        if self.scope.is_open() || self.scope.active_tasks() != 0 {
            return Ok(());
        }
        let _turn = self.turn.lock().await;
        if self.state.lock().unwrap_or_else(|e| e.into_inner()).0 != AgentStatus::Stopping {
            return Ok(());
        }
        let Some(mut session) = self
            .sessions
            .load(&self.session)
            .await
            .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))?
        else {
            return Ok(());
        };
        let Some(start) = session.events.iter().rev().find(|entry| {
            matches!(entry.event, SessionEventKind::AgentTaskStarted { agent, .. } if agent == self.id)
        }) else {
            return Ok(());
        };
        let (request, trace) = (start.request, start.trace);
        if session.events.iter().any(|entry| {
            entry.request == request
                && matches!(
                    entry.event,
                    SessionEventKind::TurnCompleted { .. }
                        | SessionEventKind::AgentTaskInterrupted { .. }
                )
        }) {
            return Ok(());
        }
        let pending = crate::canonical::event_log::open_tool_calls(&session.messages);
        session.record(
            request,
            trace,
            SessionEventKind::AgentTaskInterrupted {
                agent: self.id,
                pending_tool_calls: pending,
            },
            self.sessions.clock().as_ref(),
        );
        self.sessions
            .save(&session)
            .await
            .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))
    }
}
