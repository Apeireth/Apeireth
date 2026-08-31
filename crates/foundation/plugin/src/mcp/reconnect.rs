//! Reconnect policy for an MCP stream.
//!
//! Engine SSE transport documented `retry:` / Last-Event-ID and then
//! **did not implement them**. This module is the missing algorithm:
//! exponential backoff with cap, honouring SSE `retry:`, and carrying
//! Last-Event-ID so a later transport can send `Last-Event-ID` on GET.
//!
//! No I/O. Pair with [`crate::mcp::lifecycle::ClientSession::reset_for_reconnect`].

use std::time::Duration;

use crate::mcp::sse::SseFrame;

/// Tunables. Defaults match common SSE practice (1s start, ×2, 30s cap,
/// 8 attempts) — canonical had no numbers because reconnect was stubbed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub initial_backoff: Duration,
    pub multiplier_num: u32,
    pub multiplier_den: u32,
    pub max_backoff: Duration,
    pub max_attempts: u32,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_backoff: Duration::from_millis(1000),
            multiplier_num: 2,
            multiplier_den: 1,
            max_backoff: Duration::from_secs(30),
            max_attempts: 8,
        }
    }
}

impl ReconnectPolicy {
    pub fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }
        let mut wait = self.initial_backoff;
        for _ in 1..attempt {
            wait = wait.saturating_mul(self.multiplier_num) / self.multiplier_den.max(1);
            if wait > self.max_backoff {
                return self.max_backoff;
            }
        }
        wait.min(self.max_backoff)
    }
}

/// Mutable reconnect cursor.
#[derive(Debug, Clone)]
pub struct ReconnectState {
    policy: ReconnectPolicy,
    attempts: u32,
    last_event_id: Option<String>,
    server_retry: Option<Duration>,
}

impl ReconnectState {
    pub fn new(policy: ReconnectPolicy) -> Self {
        Self {
            policy,
            attempts: 0,
            last_event_id: None,
            server_retry: None,
        }
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    pub fn last_event_id(&self) -> Option<&str> {
        self.last_event_id.as_deref()
    }

    /// Observe a parsed frame: keep Last-Event-ID and optional `retry:`.
    pub fn observe_frame(&mut self, frame: &SseFrame) {
        if let Some(id) = &frame.id {
            if !id.is_empty() {
                self.last_event_id = Some(id.clone());
            }
        }
        if let Some(retry) = frame.retry {
            self.server_retry = Some(retry);
        }
    }

    /// Record a disconnect. Returns `Some(wait)` if another attempt is
    /// allowed, or `None` if the budget is exhausted.
    pub fn on_disconnect(&mut self) -> Option<Duration> {
        if self.attempts >= self.policy.max_attempts {
            return None;
        }
        self.attempts += 1;
        let computed = self.policy.backoff_for_attempt(self.attempts);
        Some(self.server_retry.unwrap_or(computed))
    }

    /// Successful (re)connect: reset attempt counter, keep Last-Event-ID.
    pub fn on_connected(&mut self) {
        self.attempts = 0;
        self.server_retry = None;
    }

    /// Header value a GET should send, if any.
    pub fn last_event_id_header(&self) -> Option<(&str, &str)> {
        self.last_event_id
            .as_deref()
            .map(|id| ("Last-Event-ID", id))
    }

    pub fn exhausted(&self) -> bool {
        self.attempts >= self.policy.max_attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        let p = ReconnectPolicy::default();
        assert_eq!(p.backoff_for_attempt(0), Duration::from_millis(0));
        assert_eq!(p.backoff_for_attempt(1), Duration::from_millis(1000));
        assert_eq!(p.backoff_for_attempt(2), Duration::from_millis(2000));
        assert_eq!(p.backoff_for_attempt(3), Duration::from_millis(4000));
        assert_eq!(p.backoff_for_attempt(20), Duration::from_secs(30));
    }

    #[test]
    fn disconnect_budget() {
        let mut s = ReconnectState::new(ReconnectPolicy {
            max_attempts: 2,
            ..ReconnectPolicy::default()
        });
        assert!(s.on_disconnect().is_some());
        assert!(s.on_disconnect().is_some());
        assert!(s.on_disconnect().is_none());
        assert!(s.exhausted());
    }

    #[test]
    fn observe_frame_captures_id_and_retry() {
        let mut s = ReconnectState::new(ReconnectPolicy::default());
        let frame = SseFrame {
            event: Some("message".into()),
            data_lines: vec!["{}".into()],
            id: Some("evt-9".into()),
            retry: Some(Duration::from_millis(250)),
        };
        s.observe_frame(&frame);
        assert_eq!(s.last_event_id(), Some("evt-9"));
        let wait = s.on_disconnect().unwrap();
        assert_eq!(wait, Duration::from_millis(250));
        assert_eq!(s.last_event_id_header(), Some(("Last-Event-ID", "evt-9")));
    }

    #[test]
    fn on_connected_resets_attempts_keeps_id() {
        let mut s = ReconnectState::new(ReconnectPolicy::default());
        s.observe_frame(&SseFrame {
            id: Some("a".into()),
            ..SseFrame::default()
        });
        s.on_disconnect();
        s.on_connected();
        assert_eq!(s.attempts(), 0);
        assert_eq!(s.last_event_id(), Some("a"));
        assert!(!s.exhausted());
    }
}
