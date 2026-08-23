use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};

static INCIDENT_COUNTER: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureIncident {
    pub incident_id: String,
    pub action_name: String,
    pub error_message: String,
    pub root_cause: String,
    pub preventative_anchor: String,
}

pub struct EpistemicHealer {
    incidents: Vec<FailureIncident>,
}

impl Default for EpistemicHealer {
    fn default() -> Self {
        Self::new()
    }
}

impl EpistemicHealer {
    pub fn new() -> Self {
        Self {
            incidents: Vec::new(),
        }
    }

    /// Distills a failed execution into a permanent causal preventative anchor
    pub fn distill_failure(&mut self, action_name: &str, error_msg: &str) -> FailureIncident {
        let count = INCIDENT_COUNTER.fetch_add(1, Ordering::SeqCst);
        let incident_id = format!("inc_{:06}", count);
        
        let (root_cause, preventative_anchor) = if error_msg.contains("Path traversal") || error_msg.contains("..") {
            ("Unsanitized relative path traversal attempt", "Always resolve and canonicalize paths within sandbox jail boundaries before access.")
        } else if error_msg.contains("timeout") || error_msg.contains("timed out") {
            ("Process execution exceeded timeout threshold", "Enforce exponential backoff and asynchronous polling for long-running processes.")
        } else if error_msg.contains("Governance") || error_msg.contains("Blocked") {
            ("Governance security gate policy violation", "Check 5-Gate permissions and tenant sovereignty constraints prior to triggering action.")
        } else {
            ("Unexpected runtime error", "Validate tool inputs against schema and handle fallback state gracefully.")
        };

        let incident = FailureIncident {
            incident_id,
            action_name: action_name.to_string(),
            error_message: error_msg.to_string(),
            root_cause: root_cause.to_string(),
            preventative_anchor: format!("[Auto-Learned Anchor]: {}", preventative_anchor),
        };

        self.incidents.push(incident.clone());
        incident
    }

    /// Retrieves all dynamic preventative anchors to inject into cognitive prompt assembler
    pub fn get_preventative_anchors(&self) -> Vec<String> {
        self.incidents.iter().map(|i| i.preventative_anchor.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epistemic_healing_and_rule_distillation() {
        let mut healer = EpistemicHealer::new();

        let inc1 = healer.distill_failure("fs_read", "Path traversal (..) is strictly prohibited");
        assert!(inc1.preventative_anchor.contains("sandbox jail"));

        let inc2 = healer.distill_failure("exec_command", "Governance Gate Rejection: Forbidden command");
        assert!(inc2.preventative_anchor.contains("5-Gate permissions"));

        let anchors = healer.get_preventative_anchors();
        assert_eq!(anchors.len(), 2);
    }
}
