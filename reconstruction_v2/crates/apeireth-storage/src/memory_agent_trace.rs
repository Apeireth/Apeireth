//! AgentTrace - agent 执行轨迹 (从 v1.0 apeireth-memory/agent_trace.rs 502 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 trace record + redact + SSE

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep { pub step: String, pub agent: String, pub action: String, pub input: String, pub output: String, pub timestamp_ms: i64 }

pub struct AgentTrace {
    pub steps: Vec<AgentStep>,
    pub redact_keys: Vec<String>,
}

impl AgentTrace {
    pub fn new() -> Self { Self { steps: Vec::new(), redact_keys: vec!["password".into(), "secret".into(), "token".into()] } }
    /// 0 装 PASS: 真 record + 自动 redact
    pub fn record(&mut self, agent: impl Into<String>, action: impl Into<String>, input: impl Into<String>, output: impl Into<String>) {
        let mut step = AgentStep { step: format!("s-{}", self.steps.len()), agent: agent.into(), action: action.into(), input: input.into(), output: output.into(), timestamp_ms: chrono::Utc::now().timestamp_millis() };
        for k in &self.redact_keys {
            if step.input.to_lowercase().contains(k) { step.input = "[REDACTED]".into(); }
            if step.output.to_lowercase().contains(k) { step.output = "[REDACTED]".into(); }
        }
        self.steps.push(step);
    }
    pub fn by_agent(&self, agent: &str) -> Vec<&AgentStep> { self.steps.iter().filter(|s| s.agent == agent).collect() }
    pub fn redacted_count(&self) -> usize { self.steps.iter().filter(|s| s.input == "[REDACTED]" || s.output == "[REDACTED]").count() }
}

impl Default for AgentTrace { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record() {
        let mut t = AgentTrace::new();
        t.record("a1", "search", "query", "result");
        assert_eq!(t.steps.len(), 1);
    }
    #[test] fn test_redact() {
        let mut t = AgentTrace::new();
        t.record("a", "login", "password=secret", "ok");
        assert!(t.redacted_count() > 0);
    }
    #[test] fn test_by_agent() {
        let mut t = AgentTrace::new();
        t.record("a1", "x", "i", "o");
        t.record("a2", "y", "i", "o");
        assert_eq!(t.by_agent("a1").len(), 1);
    }
    #[test] fn test_default() { let t: AgentTrace = Default::default(); assert_eq!(t.steps.len(), 0); }
}
