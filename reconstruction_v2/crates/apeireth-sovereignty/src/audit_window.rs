//! Q18 三域分离 — Thought 审计窗口

use crate::decision::{DecisionRequest, SovereigntyDomain};
use crate::three_domain::ThreeDomainGuard;
use serde::{Deserialize, Serialize};

pub const DEFAULT_AUDIT_WINDOW_MS: i64 = 1_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowDecision {
    BestEffortAllowed { request_id: String, downstream: SovereigntyDomain, elapsed_ms: i64 },
    BestEffortAllowedWithCoercion { request_id: String, downstream: SovereigntyDomain, elapsed_ms: i64, stress_level: f32 },
    WindowExpired { request_id: String, downstream: SovereigntyDomain, elapsed_ms: i64 },
    NotApplicable { request_id: String, current_domain: SovereigntyDomain },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditHistoryEntry {
    pub request_id: String,
    pub thought_at_ms: i64,
    pub downstream: Option<SovereigntyDomain>,
    pub downstream_at_ms: Option<i64>,
    pub audited_by_owner: bool,
}

pub trait AuditWindowHistory {
    fn record_thought(&mut self, request_id: &str, thought_at_ms: i64);
    fn record_downstream(&mut self, request_id: &str, downstream: SovereigntyDomain, downstream_at_ms: i64);
    fn mark_audited(&mut self, request_id: &str) -> bool;
    fn history(&self) -> Vec<AuditHistoryEntry>;
    fn lookup(&self, request_id: &str) -> Option<AuditHistoryEntry>;
    fn clear(&mut self);
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryAuditHistory { entries: Vec<AuditHistoryEntry> }

impl InMemoryAuditHistory {
    pub fn new() -> Self { Self::default() }
}

impl AuditWindowHistory for InMemoryAuditHistory {
    fn record_thought(&mut self, request_id: &str, thought_at_ms: i64) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.request_id == request_id) {
            e.thought_at_ms = thought_at_ms;
        } else {
            self.entries.push(AuditHistoryEntry { request_id: request_id.to_string(), thought_at_ms, downstream: None, downstream_at_ms: None, audited_by_owner: false });
        }
    }
    fn record_downstream(&mut self, request_id: &str, downstream: SovereigntyDomain, downstream_at_ms: i64) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.request_id == request_id) {
            e.downstream = Some(downstream);
            e.downstream_at_ms = Some(downstream_at_ms);
        } else {
            self.entries.push(AuditHistoryEntry { request_id: request_id.to_string(), thought_at_ms: downstream_at_ms, downstream: Some(downstream), downstream_at_ms: Some(downstream_at_ms), audited_by_owner: false });
        }
    }
    fn mark_audited(&mut self, request_id: &str) -> bool {
        if let Some(e) = self.entries.iter_mut().find(|e| e.request_id == request_id) { e.audited_by_owner = true; true } else { false }
    }
    fn history(&self) -> Vec<AuditHistoryEntry> { self.entries.clone() }
    fn lookup(&self, request_id: &str) -> Option<AuditHistoryEntry> { self.entries.iter().find(|e| e.request_id == request_id).cloned() }
    fn clear(&mut self) { self.entries.clear(); }
    fn len(&self) -> usize { self.entries.len() }
}

pub struct BestEffortFlow<H: AuditWindowHistory> {
    pub guard: ThreeDomainGuard,
    pub window_ms: i64,
    pub history: H,
}

impl<H: AuditWindowHistory> BestEffortFlow<H> {
    pub fn new(history: H) -> Self { Self { guard: ThreeDomainGuard::new(), window_ms: DEFAULT_AUDIT_WINDOW_MS, history } }
    pub fn with_window_ms(mut self, window_ms: i64) -> Self { self.window_ms = window_ms; self }
    pub fn pass_thought(&mut self, request: &DecisionRequest) {
        self.history.record_thought(&request.id, request.submitted_at_ms);
    }
    pub fn process_downstream(&mut self, request: &DecisionRequest) -> WindowDecision {
        if !matches!(request.domain, SovereigntyDomain::Proposal | SovereigntyDomain::Action) {
            return WindowDecision::NotApplicable { request_id: request.id.clone(), current_domain: request.domain };
        }
        let entry = self.history.lookup(&request.id);
        let Some(entry) = entry else {
            return WindowDecision::WindowExpired { request_id: request.id.clone(), downstream: request.domain, elapsed_ms: i64::MAX };
        };
        let elapsed = request.submitted_at_ms - entry.thought_at_ms;
        if elapsed <= self.window_ms {
            self.history.record_downstream(&request.id, request.domain, request.submitted_at_ms);
            WindowDecision::BestEffortAllowed { request_id: request.id.clone(), downstream: request.domain, elapsed_ms: elapsed }
        } else {
            WindowDecision::WindowExpired { request_id: request.id.clone(), downstream: request.domain, elapsed_ms: elapsed }
        }
    }
    pub fn audit_by_owner(&mut self, request_id: &str) -> bool { self.history.mark_audited(request_id) }
    pub fn audit_history(&self) -> Vec<AuditHistoryEntry> { self.history.history() }
    pub fn guard_enforce(&self, request: &DecisionRequest) -> crate::three_domain::DomainCheckResult { self.guard.check(request) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn default_window_is_1_second() { assert_eq!(DEFAULT_AUDIT_WINDOW_MS, 1_000); }
    #[test] fn history_records_thought() {
        let mut h = InMemoryAuditHistory::new();
        h.record_thought("r-1", 100);
        assert_eq!(h.len(), 1);
        let e = h.lookup("r-1").unwrap();
        assert_eq!(e.thought_at_ms, 100);
        assert!(!e.audited_by_owner);
    }
    #[test] fn history_records_downstream() {
        let mut h = InMemoryAuditHistory::new();
        h.record_thought("r-1", 100);
        h.record_downstream("r-1", SovereigntyDomain::Action, 200);
        let e = h.lookup("r-1").unwrap();
        assert_eq!(e.downstream, Some(SovereigntyDomain::Action));
        assert_eq!(e.downstream_at_ms, Some(200));
    }
    #[test] fn history_mark_audited() {
        let mut h = InMemoryAuditHistory::new();
        h.record_thought("r-1", 100);
        assert!(h.mark_audited("r-1"));
        assert!(h.lookup("r-1").unwrap().audited_by_owner);
        assert!(!h.mark_audited("nonexistent"));
    }
    #[test] fn best_effort_within_window() {
        let mut f = BestEffortFlow::new(InMemoryAuditHistory::new()).with_window_ms(1000);
        let t = DecisionRequest::new("r-1", SovereigntyDomain::Thought, "x", 100);
        f.pass_thought(&t);
        let a = DecisionRequest::new("r-1", SovereigntyDomain::Action, "y", 500);
        match f.process_downstream(&a) {
            WindowDecision::BestEffortAllowed { elapsed_ms, .. } => assert_eq!(elapsed_ms, 400),
            other => panic!("expected BestEffortAllowed, got {:?}", other),
        }
    }
    #[test] fn best_effort_window_expired() {
        let mut f = BestEffortFlow::new(InMemoryAuditHistory::new()).with_window_ms(1000);
        f.pass_thought(&DecisionRequest::new("r-1", SovereigntyDomain::Thought, "x", 100));
        let r = f.process_downstream(&DecisionRequest::new("r-1", SovereigntyDomain::Action, "y", 1200));
        assert!(matches!(r, WindowDecision::WindowExpired { .. }));
    }
    #[test] fn best_effort_no_thought() {
        let mut f = BestEffortFlow::new(InMemoryAuditHistory::new());
        let r = f.process_downstream(&DecisionRequest::new("r-unknown", SovereigntyDomain::Action, "y", 100));
        assert!(matches!(r, WindowDecision::WindowExpired { .. }));
    }
    #[test] fn best_effort_thought_not_applicable() {
        let mut f = BestEffortFlow::new(InMemoryAuditHistory::new());
        let r = f.process_downstream(&DecisionRequest::new("r-1", SovereigntyDomain::Thought, "x", 100));
        assert!(matches!(r, WindowDecision::NotApplicable { .. }));
    }
    #[test] fn audit_by_owner() {
        let mut f = BestEffortFlow::new(InMemoryAuditHistory::new());
        f.pass_thought(&DecisionRequest::new("r-1", SovereigntyDomain::Thought, "x", 100));
        assert!(f.audit_by_owner("r-1"));
        assert!(f.audit_history()[0].audited_by_owner);
    }
    #[test] fn clear_history() {
        let mut h = InMemoryAuditHistory::new();
        h.record_thought("r", 0);
        assert_eq!(h.len(), 1);
        h.clear();
        assert_eq!(h.len(), 0);
        assert!(h.is_empty());
    }
    #[test] fn is_empty_default() {
        let h = InMemoryAuditHistory::new();
        assert!(h.is_empty());
    }
}
