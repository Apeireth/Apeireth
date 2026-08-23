use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolObservation {
    pub tool_name: String,
    pub input: String,
    pub output: String,
    pub timestamp: i64,
    pub success: bool,
}

#[derive(Debug, Default)]
pub struct ExperienceQueue {
    observations: Vec<ToolObservation>,
    max_capacity: usize,
}

impl ExperienceQueue {
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            observations: Vec::with_capacity(capacity),
            max_capacity: capacity,
        }
    }

    pub fn capture(&mut self, obs: ToolObservation) {
        if self.observations.len() >= self.max_capacity {
            self.observations.remove(0);
        }
        self.observations.push(obs);
    }

    pub fn record(&mut self, tool_name: &str, input: &str, output: &str, success: bool) {
        self.capture(ToolObservation {
            tool_name: tool_name.to_string(),
            input: input.to_string(),
            output: output.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            success,
        });
    }

    /// Drains all observations and clears the queue
    pub fn drain(&mut self) -> Vec<ToolObservation> {
        std::mem::take(&mut self.observations)
    }

    /// Returns the n most recent observations
    pub fn recent(&self, n: usize) -> &[ToolObservation] {
        let start = self.observations.len().saturating_sub(n);
        &self.observations[start..]
    }

    pub fn len(&self) -> usize {
        self.observations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.observations.is_empty()
    }
}
