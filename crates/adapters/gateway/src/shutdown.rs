//! Graceful-shutdown trigger for the resident gateway form.
//!
//! Two independent sources can ask a serving gateway to stop: the process
//! signal face (`SIGTERM` / console control events, wired by the composition
//! root) and the token-gated admin endpoint (`POST /v1/admin/shutdown`, used by
//! the service scripts where a console signal cannot be delivered reliably).
//! Both funnel into one [`ShutdownTrigger`] held by [`crate::GatewayState`],
//! so the serve loop applies exactly one shutdown policy: stop accepting work,
//! write the closing audit fact, flush the observation archive, and exit
//! (优雅停机: 收尾审计 + 落盘).

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

/// Idempotent shutdown signal shared by the shutdown endpoint and the serve
/// loop. `trigger()` is safe to call repeatedly and from any request handler.
#[derive(Debug, Default)]
pub struct ShutdownTrigger {
    fired: AtomicBool,
    notify: Notify,
}

impl ShutdownTrigger {
    /// Fire the trigger. Returns `true` when this call was the first one
    /// (subsequent calls are no-ops, reported as `false`).
    pub fn trigger(&self) -> bool {
        let first = !self.fired.swap(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        first
    }

    /// Whether the trigger has fired.
    pub fn is_triggered(&self) -> bool {
        self.fired.load(Ordering::SeqCst)
    }

    /// Wait until the trigger fires (returns immediately once fired).
    pub async fn fired(&self) {
        loop {
            let notified = self.notify.notified();
            if self.is_triggered() {
                return;
            }
            notified.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn trigger_is_idempotent_and_wakes_waiters() {
        let trigger = ShutdownTrigger::default();
        assert!(!trigger.is_triggered());
        assert!(trigger.trigger(), "first fire reports the transition");
        assert!(!trigger.trigger(), "repeat fires are no-ops");
        assert!(trigger.is_triggered());
        trigger.fired().await; // already fired: returns immediately
    }

    #[tokio::test]
    async fn a_waiter_is_woken_by_a_later_trigger() {
        let trigger = std::sync::Arc::new(ShutdownTrigger::default());
        let waiter = {
            let trigger = trigger.clone();
            tokio::spawn(async move { trigger.fired().await })
        };
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(!waiter.is_finished(), "waiter must not fire early");
        trigger.trigger();
        tokio::time::timeout(std::time::Duration::from_secs(2), waiter)
            .await
            .expect("waiter wakes on trigger")
            .expect("waiter task joins");
    }
}
