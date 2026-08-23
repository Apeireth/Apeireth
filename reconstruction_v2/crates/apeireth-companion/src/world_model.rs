use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioPrediction {
    pub scenario: String,
    pub probability: f64,
    pub expected_utility: f64,
    pub epistemic_uncertainty: f64,
}

/// W1: Timeline Scenario Simulator
pub struct W1Simulator {
    base_confidence: f64,
}

impl Default for W1Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl W1Simulator {
    pub fn new() -> Self {
        Self { base_confidence: 0.85 }
    }

    pub fn simulate_scenario(&self, scenario_description: &str, evidence_count: usize) -> ScenarioPrediction {
        // Epistemic uncertainty decreases as evidence count increases
        let uncertainty = (1.0 / (evidence_count as f64 + 1.0).sqrt()).min(0.9);
        let prob = (self.base_confidence * (1.0 - uncertainty * 0.5)).clamp(0.05, 0.98);
        let utility = if scenario_description.contains("fail") || scenario_description.contains("reject") {
            -0.8
        } else {
            0.9
        };

        ScenarioPrediction {
            scenario: scenario_description.to_string(),
            probability: prob,
            expected_utility: utility,
            epistemic_uncertainty: uncertainty,
        }
    }
}

/// W2: MCTS Causal State Tree Node
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MctsNode {
    pub state: String,
    pub visits: usize,
    pub total_reward: f64,
    pub children: Vec<MctsNode>,
}

impl MctsNode {
    pub fn new(state: String) -> Self {
        Self {
            state,
            visits: 0,
            total_reward: 0.0,
            children: Vec::new(),
        }
    }

    pub fn ucb1(&self, parent_visits: usize, c: f64) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        let q = self.total_reward / self.visits as f64;
        let exploration = c * ((parent_visits as f64).ln() / self.visits as f64).sqrt();
        q + exploration
    }
}

/// W2: MCTS Causal Graph Search
pub struct W2CausalGraphSimulator {
    pub root: MctsNode,
    pub exploration_constant: f64,
}

impl W2CausalGraphSimulator {
    pub fn new(initial_state: String) -> Self {
        Self {
            root: MctsNode::new(initial_state),
            exploration_constant: 1.414,
        }
    }

    pub fn expand_node(&mut self, possible_actions: &[&str]) {
        for &action in possible_actions {
            self.root.children.push(MctsNode::new(format!("{}:{}", self.root.state, action)));
        }
    }

    pub fn search(&mut self, iterations: usize) -> Option<String> {
        if self.root.children.is_empty() {
            return None;
        }

        for _ in 0..iterations {
            // 1. Selection
            let mut best_idx = 0;
            let mut best_score = -1.0;
            for (i, child) in self.root.children.iter().enumerate() {
                let score = child.ucb1(self.root.visits + 1, self.exploration_constant);
                if score > best_score {
                    best_score = score;
                    best_idx = i;
                }
            }

            // 2. Rollout / Simulation (Reward based on causal alignment)
            let reward = if self.root.children[best_idx].state.contains("safe") { 1.0 } else { 0.5 };

            // 3. Backpropagation
            self.root.visits += 1;
            self.root.total_reward += reward;
            self.root.children[best_idx].visits += 1;
            self.root.children[best_idx].total_reward += reward;
        }

        // Return best action state by visit count
        self.root.children.iter().max_by_key(|c| c.visits).map(|c| c.state.clone())
    }
}

/// W3: Counterfactual Generator
pub struct W3CounterfactualGenerator;

impl W3CounterfactualGenerator {
    pub fn generate_counterfactuals(factual_premise: &str, intervention_nodes: &[&str]) -> Vec<String> {
        let mut counterfactuals = Vec::new();
        for &node in intervention_nodes {
            counterfactuals.push(format!("What if [{}] had not occurred during '{}'?", node, factual_premise));
            counterfactuals.push(format!("What if [{}] was inverted/negated during '{}'?", node, factual_premise));
        }
        counterfactuals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_w1_scenario_simulation() {
        let w1 = W1Simulator::new();
        let p1 = w1.simulate_scenario("safe_plan_execution", 5);
        assert!(p1.probability > 0.5);
        assert!(p1.epistemic_uncertainty < 0.5);
    }

    #[test]
    fn test_w2_mcts_tree_search() {
        let mut w2 = W2CausalGraphSimulator::new("root_intent".into());
        w2.expand_node(&["safe_action_a", "risky_action_b", "neutral_action_c"]);

        let best_action = w2.search(50);
        assert!(best_action.is_some());
        assert!(best_action.unwrap().contains("root_intent"));
        assert_eq!(w2.root.visits, 50);
    }

    #[test]
    fn test_w3_counterfactual_generator() {
        let cf = W3CounterfactualGenerator::generate_counterfactuals("user asked for refactor", &["network_timeout", "cache_hit"]);
        assert_eq!(cf.len(), 4);
        assert!(cf[0].contains("network_timeout"));
    }
}

