//! Routing between providers.
//!
//! # Where the boundary sits
//!
//! A [`ProviderCapability`] knows how to talk to one vendor: its credentials, its
//! HTTP client, its wire format. A [`ProviderRouter`] knows how to choose *between*
//! providers: order, fallback, and health. Neither knows the other's job, and the
//! runtime holds only the router — which is why no part of the runtime names a
//! vendor.
//!
//! # Ported, not reinvented
//!
//! The selection and health algorithm is ported from `MultiLlmRouter`
//! (`crates/apeireth-api/src/llm/router.rs`), which is mature and already
//! handles the cases a fresh implementation gets wrong. Preserved exactly:
//!
//! - candidates are filtered by `supports_model`, then ordered by their position
//!   in an explicit fallback list, with unlisted providers going last;
//! - no candidate at all is a distinct error that names the providers that *are*
//!   registered, rather than a generic failure;
//! - a **retryable** failure falls through to the next candidate;
//! - a **permanent** failure returns immediately without trying anyone else.
//!   This is the subtle one, and it is deliberate: falling back after a rejected
//!   API key would turn a misconfiguration into a silent, expensive round-robin
//!   across every provider;
//! - health tracks an EMA of latency (`(p50 * 7 + sample) / 8`) and a decaying
//!   error rate (`* 0.9`, plus `0.1` on failure), with a provider considered
//!   unhealthy after three consecutive failures or an error rate at or above 0.5.
//!
//! Changed deliberately: the ported version was generic over `apeireth-api`'s
//! `LlmRequest`/`LlmResponse` and read latency from a field the provider filled
//! in. This one speaks canonical `NormalizedRequest`/`NormalizedResponse` and
//! measures latency against the runtime's injected clock, so a virtual clock
//! makes routing behaviour reproducible.

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Clock, Timestamp};
use apeireth_plugin::{ProviderCapability, ProviderError};
use apeireth_protocol::canonical::{NormalizedRequest, NormalizedResponse};
use parking_lot::RwLock;

use super::error::{RuntimeError, RuntimeResult};

/// Observed health of one provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderHealth {
    /// Whether the router currently considers this provider usable.
    pub healthy: bool,
    /// Exponential moving average of observed latency, in milliseconds.
    pub latency_p50_ms: u64,
    /// Decaying error rate in `[0, 1]`.
    pub error_rate: f64,
    /// Failures since the last success.
    pub consecutive_failures: u32,
    /// When health was last updated.
    pub last_check: Option<Timestamp>,
}

impl Default for ProviderHealth {
    fn default() -> Self {
        Self {
            healthy: true,
            latency_p50_ms: 0,
            error_rate: 0.0,
            consecutive_failures: 0,
            last_check: None,
        }
    }
}

/// Consecutive failures after which a provider is considered unhealthy.
const UNHEALTHY_AFTER_FAILURES: u32 = 3;
/// Error rate at or above which a provider is considered unhealthy.
const UNHEALTHY_ERROR_RATE: f64 = 0.5;

/// Chooses among providers and falls back when one fails transiently.
pub struct ProviderRouter {
    providers: Vec<Arc<dyn ProviderCapability>>,
    fallback_order: Vec<CapabilityId>,
    health: RwLock<BTreeMap<CapabilityId, ProviderHealth>>,
    clock: Arc<dyn Clock>,
}

impl ProviderRouter {
    /// A router over the given providers, tried in the order supplied.
    pub fn new(providers: Vec<Arc<dyn ProviderCapability>>, clock: Arc<dyn Clock>) -> Self {
        let fallback_order = providers.iter().map(|p| p.id().clone()).collect();
        Self {
            providers,
            fallback_order,
            health: RwLock::new(BTreeMap::new()),
            clock,
        }
    }

    /// Override the order candidates are tried in.
    ///
    /// Providers absent from `order` are still usable; they are simply tried
    /// after every listed one.
    #[must_use]
    pub fn with_fallback_order(mut self, order: Vec<CapabilityId>) -> Self {
        self.fallback_order = order;
        self
    }

    /// How many providers are registered.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Whether no provider is registered.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Registered provider ids, in fallback order.
    pub fn provider_ids(&self) -> Vec<CapabilityId> {
        let mut sorted: Vec<&Arc<dyn ProviderCapability>> = self.providers.iter().collect();
        sorted.sort_by_key(|p| self.rank(p.id()));
        sorted.iter().map(|p| p.id().clone()).collect()
    }

    /// Current health of a provider, if it has been exercised.
    pub fn health(&self, id: &CapabilityId) -> Option<ProviderHealth> {
        self.health.read().get(id).cloned()
    }

    /// Serve a completion, falling back on transient failures.
    ///
    /// Returns which provider succeeded alongside the response, so the caller can
    /// record it: "the turn used provider.b because provider.a was rate limited"
    /// is not reconstructible after the fact otherwise.
    pub async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> RuntimeResult<(CapabilityId, NormalizedResponse)> {
        let mut candidates: Vec<&Arc<dyn ProviderCapability>> = self
            .providers
            .iter()
            .filter(|p| p.supports_model(&request.model))
            .collect();
        candidates.sort_by_key(|p| self.rank(p.id()));

        if candidates.is_empty() {
            let available: Vec<&str> = self.providers.iter().map(|p| p.id().as_str()).collect();
            return Err(RuntimeError::NoProvider {
                model: request.model.clone(),
                available: if available.is_empty() {
                    "none".to_string()
                } else {
                    available.join(", ")
                },
            });
        }

        let mut last_error: Option<ProviderError> = None;

        for provider in candidates {
            let id = provider.id().clone();
            let started = Timestamp::from_clock(self.clock.as_ref());

            match provider.complete(request).await {
                Ok(response) => {
                    let elapsed = Timestamp::from_clock(self.clock.as_ref())
                        .epoch_millis()
                        .saturating_sub(started.epoch_millis())
                        .unsigned_abs();
                    self.record_success(&id, elapsed);
                    return Ok((id, response));
                }
                Err(e) if e.is_retryable() => {
                    self.record_failure(&id);
                    last_error = Some(e);
                }
                Err(e) => {
                    // A permanent failure must not cascade: retrying a rejected
                    // key against every other provider hides the real problem.
                    self.record_failure(&id);
                    return Err(RuntimeError::Provider(e));
                }
            }
        }

        Err(RuntimeError::ProvidersExhausted {
            model: request.model.clone(),
            source: last_error.expect("a non-empty candidate list always sets last_error"),
        })
    }

    /// Position of `id` in the fallback order; unlisted providers sort last.
    fn rank(&self, id: &CapabilityId) -> usize {
        self.fallback_order
            .iter()
            .position(|candidate| candidate == id)
            .unwrap_or(usize::MAX)
    }

    fn record_success(&self, id: &CapabilityId, latency_ms: u64) {
        let now = Timestamp::from_clock(self.clock.as_ref());
        let mut health = self.health.write();
        let entry = health.entry(id.clone()).or_default();

        entry.consecutive_failures = 0;
        entry.latency_p50_ms = if entry.latency_p50_ms == 0 {
            latency_ms
        } else {
            (entry.latency_p50_ms * 7 + latency_ms) / 8
        };
        entry.error_rate *= 0.9;
        entry.healthy = entry.error_rate < UNHEALTHY_ERROR_RATE;
        entry.last_check = Some(now);
    }

    fn record_failure(&self, id: &CapabilityId) {
        let now = Timestamp::from_clock(self.clock.as_ref());
        let mut health = self.health.write();
        let entry = health.entry(id.clone()).or_default();

        entry.consecutive_failures += 1;
        entry.error_rate = entry.error_rate * 0.9 + 0.1;
        entry.healthy = entry.consecutive_failures < UNHEALTHY_AFTER_FAILURES
            && entry.error_rate < UNHEALTHY_ERROR_RATE;
        entry.last_check = Some(now);
    }
}

impl std::fmt::Debug for ProviderRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRouter")
            .field("providers", &self.provider_ids())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use apeireth_core::kernel::{ModelId, VirtualClock};
    use apeireth_protocol::canonical::{
        ModelDescriptor, NormalizedFinishReason, NormalizedMessage, NormalizedUsage,
    };
    use async_trait::async_trait;

    /// A provider that answers, or fails, exactly as instructed.
    struct Scripted {
        id: CapabilityId,
        model: String,
        outcome: Box<dyn Fn() -> Result<(), ProviderError> + Send + Sync>,
        calls: AtomicUsize,
    }

    impl Scripted {
        fn ok(id: &str, model: &str) -> Arc<Self> {
            Arc::new(Self {
                id: CapabilityId::new(id).unwrap(),
                model: model.into(),
                outcome: Box::new(|| Ok(())),
                calls: AtomicUsize::new(0),
            })
        }

        fn failing(id: &str, model: &str, error: ProviderError) -> Arc<Self> {
            Arc::new(Self {
                id: CapabilityId::new(id).unwrap(),
                model: model.into(),
                outcome: Box::new(move || Err(error.clone())),
                calls: AtomicUsize::new(0),
            })
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl ProviderCapability for Scripted {
        fn id(&self) -> &CapabilityId {
            &self.id
        }
        fn models(&self) -> Vec<ModelDescriptor> {
            vec![ModelDescriptor::new(
                ModelId::new(&self.model).unwrap(),
                self.id.clone(),
            )]
        }
        async fn complete(
            &self,
            request: &NormalizedRequest,
        ) -> Result<NormalizedResponse, ProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            (self.outcome)()?;
            Ok(NormalizedResponse {
                id: format!("resp_from_{}", self.id),
                model: request.model.clone(),
                content: format!("answered by {}", self.id),
                finish_reason: Some(NormalizedFinishReason::Stop),
                usage: NormalizedUsage::default(),
                tool_calls: Vec::new(),
                raw_metadata: serde_json::Map::new(),
            })
        }
    }

    fn clock() -> Arc<dyn Clock> {
        Arc::new(VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ))
    }

    fn request(model: &str) -> NormalizedRequest {
        NormalizedRequest::new(model, vec![NormalizedMessage::user("hello")])
    }

    fn rate_limited(provider: &str) -> ProviderError {
        ProviderError::RateLimited {
            provider: provider.into(),
            retry_after_ms: 100,
        }
    }

    fn auth_failed(provider: &str) -> ProviderError {
        ProviderError::AuthFailed {
            provider: provider.into(),
            detail: "rejected key".into(),
        }
    }

    #[tokio::test]
    async fn the_first_supporting_provider_serves_the_request() {
        let a = Scripted::ok("provider.a", "shared-model");
        let b = Scripted::ok("provider.b", "shared-model");
        let router = ProviderRouter::new(vec![a.clone(), b.clone()], clock());

        let (served_by, response) = router.complete(&request("shared-model")).await.unwrap();

        assert_eq!(served_by.as_str(), "provider.a");
        assert_eq!(response.content, "answered by provider.a");
        assert_eq!(a.calls(), 1);
        assert_eq!(b.calls(), 0, "the second provider must not be consulted");
    }

    #[tokio::test]
    async fn a_transient_failure_falls_through_to_the_next_provider() {
        let a = Scripted::failing("provider.a", "shared-model", rate_limited("provider.a"));
        let b = Scripted::ok("provider.b", "shared-model");
        let router = ProviderRouter::new(vec![a.clone(), b.clone()], clock());

        let (served_by, _) = router.complete(&request("shared-model")).await.unwrap();

        assert_eq!(served_by.as_str(), "provider.b");
        assert_eq!(a.calls(), 1);
        assert_eq!(b.calls(), 1);
    }

    #[tokio::test]
    async fn a_permanent_failure_stops_rather_than_cascading() {
        let a = Scripted::failing("provider.a", "shared-model", auth_failed("provider.a"));
        let b = Scripted::ok("provider.b", "shared-model");
        let router = ProviderRouter::new(vec![a.clone(), b.clone()], clock());

        let err = router.complete(&request("shared-model")).await.unwrap_err();

        assert!(matches!(err, RuntimeError::Provider(_)), "{err}");
        assert_eq!(
            b.calls(),
            0,
            "a rejected key must not trigger a round-robin across every provider"
        );
    }

    #[tokio::test]
    async fn exhausting_every_transient_candidate_reports_the_last_error() {
        let a = Scripted::failing("provider.a", "shared-model", rate_limited("provider.a"));
        let b = Scripted::failing("provider.b", "shared-model", rate_limited("provider.b"));
        let router = ProviderRouter::new(vec![a.clone(), b.clone()], clock());

        let err = router.complete(&request("shared-model")).await.unwrap_err();

        match err {
            RuntimeError::ProvidersExhausted { model, source } => {
                assert_eq!(model, "shared-model");
                assert_eq!(source.provider(), "provider.b", "the last one tried");
            }
            other => panic!("expected ProvidersExhausted, got {other}"),
        }
        assert_eq!(a.calls(), 1);
        assert_eq!(b.calls(), 1);
    }

    #[tokio::test]
    async fn an_unservable_model_names_the_providers_that_do_exist() {
        let a = Scripted::ok("provider.a", "model-a");
        let router = ProviderRouter::new(vec![a.clone()], clock());

        let err = router.complete(&request("model-absent")).await.unwrap_err();

        match err {
            RuntimeError::NoProvider { model, available } => {
                assert_eq!(model, "model-absent");
                assert!(available.contains("provider.a"), "{available}");
            }
            other => panic!("expected NoProvider, got {other}"),
        }
        assert_eq!(a.calls(), 0);
    }

    #[tokio::test]
    async fn an_empty_router_reports_that_nothing_is_registered() {
        let router = ProviderRouter::new(Vec::new(), clock());
        assert!(router.is_empty());

        let err = router.complete(&request("anything")).await.unwrap_err();
        match err {
            RuntimeError::NoProvider { available, .. } => assert_eq!(available, "none"),
            other => panic!("expected NoProvider, got {other}"),
        }
    }

    #[tokio::test]
    async fn the_fallback_order_decides_who_is_tried_first() {
        let a = Scripted::ok("provider.a", "shared-model");
        let b = Scripted::ok("provider.b", "shared-model");
        let router = ProviderRouter::new(vec![a.clone(), b.clone()], clock())
            .with_fallback_order(vec![CapabilityId::new("provider.b").unwrap()]);

        // `provider.a` is unlisted, so it sorts after the listed `provider.b`.
        assert_eq!(
            router
                .provider_ids()
                .iter()
                .map(CapabilityId::as_str)
                .collect::<Vec<_>>(),
            ["provider.b", "provider.a"]
        );

        let (served_by, _) = router.complete(&request("shared-model")).await.unwrap();
        assert_eq!(served_by.as_str(), "provider.b");
        assert_eq!(a.calls(), 0);
    }

    #[tokio::test]
    async fn health_records_success_and_failure_separately() {
        let a = Scripted::failing("provider.a", "shared-model", rate_limited("provider.a"));
        let b = Scripted::ok("provider.b", "shared-model");
        let router = ProviderRouter::new(vec![a, b], clock());
        let id_a = CapabilityId::new("provider.a").unwrap();
        let id_b = CapabilityId::new("provider.b").unwrap();

        assert!(router.health(&id_a).is_none(), "unexercised, so unknown");

        router.complete(&request("shared-model")).await.unwrap();

        let health_a = router.health(&id_a).unwrap();
        assert_eq!(health_a.consecutive_failures, 1);
        assert!(health_a.error_rate > 0.0);
        assert!(health_a.healthy, "one failure is not enough to sideline it");

        let health_b = router.health(&id_b).unwrap();
        assert_eq!(health_b.consecutive_failures, 0);
        assert_eq!(health_b.error_rate, 0.0);
        assert!(health_b.healthy);
    }

    #[tokio::test]
    async fn three_consecutive_failures_mark_a_provider_unhealthy() {
        let a = Scripted::failing("provider.a", "shared-model", rate_limited("provider.a"));
        let router = ProviderRouter::new(vec![a], clock());
        let id = CapabilityId::new("provider.a").unwrap();

        for expected in 1..=UNHEALTHY_AFTER_FAILURES {
            let _ = router.complete(&request("shared-model")).await;
            assert_eq!(router.health(&id).unwrap().consecutive_failures, expected);
        }

        assert!(
            !router.health(&id).unwrap().healthy,
            "three consecutive failures must sideline a provider"
        );
    }

    #[tokio::test]
    async fn a_success_clears_the_consecutive_failure_count() {
        // Fails the first two calls, then succeeds.
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let flaky = Arc::new(Scripted {
            id: CapabilityId::new("provider.flaky").unwrap(),
            model: "shared-model".into(),
            outcome: Box::new(move || {
                if counter.fetch_add(1, Ordering::SeqCst) < 2 {
                    Err(rate_limited("provider.flaky"))
                } else {
                    Ok(())
                }
            }),
            calls: AtomicUsize::new(0),
        });
        let router = ProviderRouter::new(vec![flaky], clock());
        let id = CapabilityId::new("provider.flaky").unwrap();

        let _ = router.complete(&request("shared-model")).await;
        let _ = router.complete(&request("shared-model")).await;
        assert_eq!(router.health(&id).unwrap().consecutive_failures, 2);

        router.complete(&request("shared-model")).await.unwrap();
        let health = router.health(&id).unwrap();
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.error_rate < 0.2, "the error rate decays: {health:?}");
    }

    #[tokio::test]
    async fn latency_is_measured_against_the_injected_clock() {
        let virtual_clock = VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        );
        let clock: Arc<dyn Clock> = Arc::new(virtual_clock.clone());
        let router = ProviderRouter::new(vec![Scripted::ok("provider.a", "m")], clock);
        let id = CapabilityId::new("provider.a").unwrap();

        router.complete(&request("m")).await.unwrap();

        // A virtual clock that never advances yields a deterministic zero,
        // which is what keeps routing behaviour reproducible in tests.
        assert_eq!(router.health(&id).unwrap().latency_p50_ms, 0);
    }
}
