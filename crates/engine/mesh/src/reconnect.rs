//! Keeping a trusted peer connected: redial with bounded, jittered backoff.
//!
//! This is liveness policy only. Every redial runs the same handshake, trust
//! check and pin verification as a first connection, so a reconnect can never
//! be a way around revocation: once trust is withdrawn or the node shuts down,
//! the loop ends instead of retrying.

use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::NodeId;
use ring::rand::{SecureRandom, SystemRandom};

use crate::error::MeshError;
use crate::node::MeshNode;
use crate::transport::MeshTransport;

/// Backoff schedule for [`MeshNode::maintain_connection`].
#[derive(Debug, Clone)]
pub struct ReconnectPolicy {
    /// Delay after the first failure.
    pub initial_backoff: Duration,
    /// Ceiling for the delay.
    pub max_backoff: Duration,
    /// Fraction of each delay randomized (0.0 to 1.0) so a fleet that lost a
    /// server does not redial in lockstep.
    pub jitter: f64,
    /// How often an established connection is checked for loss.
    pub check_interval: Duration,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            jitter: 0.2,
            check_interval: Duration::from_secs(1),
        }
    }
}

impl ReconnectPolicy {
    fn delay(&self, failures: u32) -> Duration {
        let doubled = self
            .initial_backoff
            .saturating_mul(1u32.checked_shl(failures.min(16)).unwrap_or(u32::MAX));
        let base = doubled.min(self.max_backoff);
        let jitter = self.jitter.clamp(0.0, 1.0);
        if jitter == 0.0 {
            return base;
        }
        let mut byte = [0u8; 1];
        let unit = if SystemRandom::new().fill(&mut byte).is_ok() {
            f64::from(byte[0]) / 255.0
        } else {
            0.5
        };
        // Spread over [1 - jitter, 1].
        base.mul_f64(1.0 - jitter * unit)
    }
}

impl MeshNode {
    /// Keep a connection to `peer` alive: dial `endpoint` over `transport`
    /// whenever the peer is not dispatchable, backing off after failures.
    ///
    /// Ends on [`MeshNode::shutdown`] and as soon as the peer is no longer
    /// trusted. An `Untrusted` refusal from the handshake also ends it: the
    /// far end disowning us is not something to hammer.
    pub fn maintain_connection(
        self: &Arc<Self>,
        peer: NodeId,
        endpoint: String,
        transport: Arc<dyn MeshTransport>,
        policy: ReconnectPolicy,
    ) -> tokio::task::JoinHandle<()> {
        let mesh = Arc::clone(self);
        tokio::spawn(async move {
            let mut failures: u32 = 0;
            loop {
                if mesh.is_shutdown() || !mesh.is_trusted(&peer) {
                    return;
                }
                if mesh
                    .peer_state(&peer)
                    .is_some_and(|state| state.is_dispatchable())
                {
                    failures = 0;
                    tokio::time::sleep(policy.check_interval).await;
                    continue;
                }
                let attempt = match transport.dial(&endpoint).await {
                    Ok(link) => mesh.connect(link, peer.clone()).await.map(|_| ()),
                    Err(error) => Err(MeshError::Transport(error.to_string())),
                };
                match attempt {
                    // Superseded means the far end connected to us first.
                    Ok(()) | Err(MeshError::Superseded { .. }) => failures = 0,
                    Err(MeshError::Untrusted { .. }) => return,
                    Err(_) => {
                        tokio::time::sleep(policy.delay(failures)).await;
                        failures = failures.saturating_add(1);
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_is_capped() {
        let policy = ReconnectPolicy {
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_millis(1_000),
            jitter: 0.0,
            check_interval: Duration::from_millis(10),
        };
        let delays: Vec<_> = (0..6).map(|n| policy.delay(n).as_millis()).collect();
        assert_eq!(delays, vec![100, 200, 400, 800, 1_000, 1_000]);
        assert_eq!(policy.delay(u32::MAX), Duration::from_millis(1_000));
    }

    #[test]
    fn jitter_only_ever_shortens_within_its_fraction() {
        let policy = ReconnectPolicy {
            initial_backoff: Duration::from_millis(1_000),
            max_backoff: Duration::from_millis(1_000),
            jitter: 0.25,
            check_interval: Duration::from_millis(10),
        };
        for _ in 0..50 {
            let delay = policy.delay(0).as_millis();
            assert!((750..=1_000).contains(&delay), "{delay}");
        }
    }
}
