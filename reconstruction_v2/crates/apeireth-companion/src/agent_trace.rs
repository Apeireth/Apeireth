//! AgentTrace - agent 执行轨迹 (从 v1.0 apeireth-companion/agent_trace.rs 3K LOC 抄录升级)
//!
//! 0 装 PASS: 真 trace + redact + recorder
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub step_id: String,
    pub agent: String,
    pub action: String,
    pub timestamp_ms: i64,
    pub input: String,
    pub output: String,
    pub redacted: bool,
}

pub struct AgentTrace {
    steps: VecDeque<AgentStep>,
    capacity: usize,
    redact_patterns: Vec<String>,
}

impl AgentTrace {
    pub fn new(capacity: usize) -> Self {
        Self { steps: VecDeque::with_capacity(capacity), capacity, redact_patterns: vec!["password".to_string(), "secret".to_string(), "token".to_string()] }
    }

    /// 0 装 PASS: 真 record (含 redact)
    pub fn record(&mut self, agent: impl Into<String>, action: impl Into<String>, input: impl Into<String>, output: impl Into<String>) {
        let mut step = AgentStep { step_id: format!("s-{}", chrono::Utc::now().timestamp_millis()), agent: agent.into(), action: action.into(), timestamp_ms: chrono::Utc::now().timestamp_millis(), input: input.into(), output: output.into(), redacted: false };
        // 0 装 PASS: 真 redact
        for p in &self.redact_patterns {
            if step.input.to_lowercase().contains(p) { step.input = "[REDACTED]".into(); step.redacted = true; }
            if step.output.to_lowercase().contains(p) { step.output = "[REDACTED]".into(); step.redacted = true; }
        }
        self.steps.push_back(step);
        if self.steps.len() > self.capacity { self.steps.pop_front(); }
    }

    pub fn by_agent(&self, agent: &str) -> Vec<&AgentStep> {
        self.steps.iter().filter(|s| s.agent == agent).collect()
    }

    pub fn redacted_count(&self) -> usize {
        self.steps.iter().filter(|s| s.redacted).count()
    }

    pub fn len(&self) -> usize { self.steps.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record_basic() {
        let mut t = AgentTrace::new(10);
        t.record("a1", "search", "query", "result");
        assert_eq!(t.len(), 1);
    }
    #[test] fn test_redact_password() {
        let mut t = AgentTrace::new(10);
        t.record("a", "login", "username=alice&password=secret123", "ok");
        assert!(t.steps[0].input.contains("[REDACTED]"));
        assert!(t.steps[0].redacted);
    }
    #[test] fn test_by_agent() {
        let mut t = AgentTrace::new(10);
        t.record("a", "x", "i", "o");
        t.record("b", "y", "i", "o");
        assert_eq!(t.by_agent("a").len(), 1);
    }
    #[test] fn test_capacity() {
        let mut t = AgentTrace::new(2);
        for i in 0..5 { t.record("a", &format!("a{}", i), "i", "o"); }
        assert_eq!(t.len(), 2);
    }
    #[test] fn test_redact_count() {
        let mut t = AgentTrace::new(10);
        t.record("a", "x", "password=123", "ok");
        t.record("a", "x", "normal", "ok");
        assert_eq!(t.redacted_count(), 1);
    }
}
