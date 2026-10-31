//! Mandatory review at the existing tool execution seam. This owns no catalog.
use apeireth_plugin::system_one::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SystemOneLimits {
    pub review_timeout: Duration,
    pub audit_timeout: Duration,
    pub max_request_bytes: usize,
}
impl Default for SystemOneLimits {
    fn default() -> Self {
        Self {
            review_timeout: Duration::from_secs(5),
            audit_timeout: Duration::from_secs(1),
            max_request_bytes: 128 * 1024,
        }
    }
}
/// Host-only model binding. Absence/replacement never grants execution.
pub struct SystemOneGate {
    supervisor: RwLock<Option<Arc<dyn SystemOneSupervisor>>>,
    audit: Arc<dyn SystemOneAudit>,
    generation: AtomicU64,
    pub limits: SystemOneLimits,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum ReviewFailure {
    Denied,
    Escalated,
    Unavailable,
    Invalid,
    Timeout,
    Cancelled,
    Audit,
    Revoked,
}
impl SystemOneGate {
    pub fn new(audit: Arc<dyn SystemOneAudit>, limits: SystemOneLimits) -> Result<Self, String> {
        if limits.review_timeout.is_zero()
            || limits.audit_timeout.is_zero()
            || limits.max_request_bytes == 0
        {
            return Err("invalid System One limits".into());
        }
        Ok(Self {
            supervisor: RwLock::new(None),
            audit,
            generation: AtomicU64::new(0),
            limits,
        })
    }
    pub fn bind_supervisor(&self, supervisor: Option<Arc<dyn SystemOneSupervisor>>) {
        let mut slot = self.supervisor.write().unwrap_or_else(|e| e.into_inner());
        *slot = supervisor;
        self.generation.fetch_add(1, Ordering::AcqRel);
    }
    pub(crate) fn is_current(&self, generation: u64) -> bool {
        self.generation.load(Ordering::Acquire) == generation
    }
    pub(crate) async fn review(&self, request: &SystemOneRequest) -> Result<u64, ReviewFailure> {
        let (supervisor, generation) = {
            let slot = self.supervisor.read().unwrap_or_else(|e| e.into_inner());
            (slot.clone(), self.generation.load(Ordering::Acquire))
        };
        let review = async {
            if request.cancel_token.is_cancelled() { return Err(ReviewFailure::Cancelled); }
            if !super::dispatcher::fits_json(request, self.limits.max_request_bytes) { return Err(ReviewFailure::Invalid); }
            let supervisor = supervisor.ok_or(ReviewFailure::Unavailable)?;
            let wait = self.limits.review_timeout.min(Duration::from_millis(request.deadline_remaining_ms));
            let response = tokio::select! {
                biased;
                _ = async { while !request.cancel_token.is_cancelled() { tokio::time::sleep(Duration::from_millis(2)).await; } } => return Err(ReviewFailure::Cancelled),
                response = tokio::time::timeout(wait, supervisor.review(request)) => response,
            };
            let verdict = match response {
                Err(_) => return Err(ReviewFailure::Timeout),
                Ok(Err(SystemOneError::Unavailable)) => return Err(ReviewFailure::Unavailable),
                Ok(Err(SystemOneError::Timeout)) => return Err(ReviewFailure::Timeout),
                Ok(Err(_)) => return Err(ReviewFailure::Invalid),
                Ok(Ok(verdict)) => verdict,
            };
            if !verdict.matches(request) { return Err(ReviewFailure::Invalid); }
            if !self.is_current(generation) { return Err(ReviewFailure::Revoked); }
            if request.cancel_token.is_cancelled() { return Err(ReviewFailure::Cancelled); }
            match verdict.decision {
                SystemOneDecision::Allow => Ok(generation),
                SystemOneDecision::Deny => Err(ReviewFailure::Denied),
                SystemOneDecision::Escalate => Err(ReviewFailure::Escalated),
            }
        }.await;
        let status = match review {
            Ok(_) => SystemOneReviewStatus::Allowed,
            Err(ReviewFailure::Denied) => SystemOneReviewStatus::Denied,
            Err(ReviewFailure::Escalated) => SystemOneReviewStatus::Escalated,
            Err(ReviewFailure::Unavailable) => SystemOneReviewStatus::Unavailable,
            Err(ReviewFailure::Timeout) => SystemOneReviewStatus::Timeout,
            Err(ReviewFailure::Cancelled) => SystemOneReviewStatus::Cancelled,
            Err(ReviewFailure::Revoked) => SystemOneReviewStatus::Revoked,
            Err(_) => SystemOneReviewStatus::Malformed,
        };
        tokio::time::timeout(
            self.limits.audit_timeout,
            self.audit
                .record(&SystemOneAuditRecord::from_review(request, status)),
        )
        .await
        .map_err(|_| ReviewFailure::Audit)?
        .map_err(|_| ReviewFailure::Audit)?;
        review
    }
}
