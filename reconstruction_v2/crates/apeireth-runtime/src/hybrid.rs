use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HybridRoutingDecision {
    LocalFastPath {
        intent: String,
        confidence: f64,
        response_template: Option<String>,
    },
    CloudDeepPath {
        reason: String,
        requires_mcts: bool,
    },
}

pub struct HybridCognitiveRouter;

impl HybridCognitiveRouter {
    pub fn new() -> Self {
        Self
    }

    /// Evaluates user input and routes between local sub-5ms fast path and cloud frontier reasoning
    pub fn route(&self, input: &str) -> HybridRoutingDecision {
        let trimmed = input.trim().to_lowercase();

        // 1. Instant local fast paths
        if trimmed == "ping" || trimmed == "health" {
            return HybridRoutingDecision::LocalFastPath {
                intent: "system_health".into(),
                confidence: 0.99,
                response_template: Some("pong - Apeireth 2.0 Living Host Online".into()),
            };
        }

        if trimmed == "time" || trimmed == "几点了" || trimmed == "现在时间" {
            return HybridRoutingDecision::LocalFastPath {
                intent: "clock_query".into(),
                confidence: 0.98,
                response_template: Some(format!("Current UTC time: {}", chrono::Utc::now().to_rfc3339())),
            };
        }

        if trimmed == "who are you" || trimmed == "你是谁" {
            return HybridRoutingDecision::LocalFastPath {
                intent: "identity_self".into(),
                confidence: 0.95,
                response_template: Some("I am Apeireth 2.0, an authentic, sovereign cognitive companion operating with non-negotiable safety and genuine reasoning.".into()),
            };
        }

        // 2. Complex queries needing Deep Cloud LLM & MCTS Causal reasoning
        let requires_mcts = trimmed.contains("why") || trimmed.contains("how to") || trimmed.contains("design")
            || trimmed.contains("refactor") || trimmed.contains("为什么") || trimmed.contains("设计") || trimmed.contains("架构");

        HybridRoutingDecision::CloudDeepPath {
            reason: "Complex conversational reasoning / tool invocation required".into(),
            requires_mcts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_cognitive_routing() {
        let router = HybridCognitiveRouter::new();

        let r1 = router.route("ping");
        match r1 {
            HybridRoutingDecision::LocalFastPath { intent, .. } => assert_eq!(intent, "system_health"),
            _ => panic!("Expected local fast path for ping"),
        }

        let r2 = router.route("How to design a distributed consensus protocol in Rust?");
        match r2 {
            HybridRoutingDecision::CloudDeepPath { requires_mcts, .. } => assert!(requires_mcts),
            _ => panic!("Expected cloud deep path for complex design question"),
        }
    }
}
