//! Memory AgentTrace - agent 轨迹 (抄 v1 apeireth-memory/agent_trace.rs)
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep { pub step: String, pub agent: String, pub action: String, pub timestamp_ms: i64 }

#[derive(Default)]
pub struct AgentTraceLog { pub steps: HashMap<String, Vec<AgentStep>>, pub redact_keys: Vec<String> }

impl AgentTraceLog {
    pub fn new() -> Self { Self { steps: HashMap::new(), redact_keys: vec!["password".into(), "token".into()] } }
    pub fn record(&mut self, agent: impl Into<String>, action: impl Into<String>) {
        let step = AgentStep { step: format!("s-{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)), agent: agent.into(), action: action.into(), timestamp_ms: chrono::Utc::now().timestamp_millis() };
        self.steps.entry(step.agent.clone()).or_default().push(step);
    }
    pub fn by_agent(&self, agent: &str) -> Vec<&AgentStep> { self.steps.get(agent).map(|v| v.iter().collect()).unwrap_or_default() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record() { let mut t = AgentTraceLog::new(); t.record("a", "search"); assert_eq!(t.steps.len(), 1); }
    #[test] fn test_unknown() { let t = AgentTraceLog::new(); assert!(t.by_agent("x").is_empty()); }
}