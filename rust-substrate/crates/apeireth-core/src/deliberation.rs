//! Deliberation Engine — Rust implementation of Thought Branching (Tree-of-Thoughts).
//!
//! 主人 20:29 真哲学指令: "底层记得用rust, 我们追求极致"
//! "思考为核, ASI绝对会自己思考, 任何LLM接入即ASI"
//!
//! Python `apeireth/deliberation.py` 是高层 wrapper.
//! 这个 Rust 实现是 hot path (ToT BFS/DFS 树搜索) — 极致性能.

use serde::{Deserialize, Serialize};

/// One thought step in deliberation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtStep {
    pub step_id: String,
    pub step_type: StepType,
    pub content: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum StepType {
    Thought,
    Action,
    Observation,
    Critique,
}

/// A branch in Tree-of-Thoughts — explores one hypothesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtBranch {
    pub branch_id: String,
    pub hypothesis: String,
    pub steps: Vec<ThoughtStep>,
    pub score: f64,
    pub status: BranchStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BranchStatus {
    Active,
    Completed,
    Pruned,
    Selected,
}

/// Result of deliberation — best plan + reasoning summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberationResult {
    pub deliberation_id: String,
    pub query: String,
    pub mode: String,
    pub branches: Vec<ThoughtBranch>,
    pub selected_branch_id: Option<String>,
    pub final_plan: Vec<String>,
    pub self_score: f64,
    pub total_steps: usize,
}

/// Tree-of-Thoughts engine — Rust hot path for ASI deliberation.
pub struct TotEngine {
    max_branches: usize,
    max_depth: usize,
}

impl TotEngine {
    pub fn new(max_branches: usize, max_depth: usize) -> Self {
        Self { max_branches, max_depth }
    }

    /// Score a branch by averaging its steps' confidences.
    pub fn score(branch: &ThoughtBranch) -> f64 {
        if branch.steps.is_empty() {
            return 0.0;
        }
        let sum: f64 = branch.steps.iter().map(|s| s.confidence).sum();
        sum / branch.steps.len() as f64
    }

    /// Select the best branch (highest score).
    pub fn select_best(branches: &[ThoughtBranch]) -> Option<usize> {
        if branches.is_empty() {
            return None;
        }
        let mut best_idx = 0;
        let mut best_score = Self::score(&branches[0]);
        for (i, b) in branches.iter().enumerate().skip(1) {
            let s = Self::score(b);
            if s > best_score {
                best_score = s;
                best_idx = i;
            }
        }
        Some(best_idx)
    }

    /// BFS-style expansion: explore N hypotheses in parallel.
    /// (real LLM calls happen in Python layer, this is the structural search)
    pub fn select_top_k(branches: &[ThoughtBranch], k: usize) -> Vec<usize> {
        let mut indexed: Vec<(usize, f64)> = branches
            .iter()
            .enumerate()
            .map(|(i, b)| (i, Self::score(b)))
            .collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        indexed.into_iter().take(k).map(|(i, _)| i).collect()
    }
}

/// Default ToT config for ASI (主人 20:29 "极致").
pub fn default_asi_tot_engine() -> TotEngine {
    TotEngine::new(5, 8)  // 5 branches, depth 8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_score_branch() {
        let branch = ThoughtBranch {
            branch_id: "b1".into(),
            hypothesis: "test".into(),
            steps: vec![
                ThoughtStep { step_id: "s1".into(), step_type: StepType::Thought, content: "a".into(), confidence: 0.8 },
                ThoughtStep { step_id: "s2".into(), step_type: StepType::Action, content: "b".into(), confidence: 0.6 },
            ],
            score: 0.0,
            status: BranchStatus::Active,
        };
        assert!((TotEngine::score(&branch) - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_select_best() {
        let branches = vec![
            ThoughtBranch {
                branch_id: "b1".into(),
                hypothesis: "h1".into(),
                steps: vec![ThoughtStep { step_id: "s1".into(), step_type: StepType::Thought, content: "a".into(), confidence: 0.3 }],
                score: 0.0,
                status: BranchStatus::Active,
            },
            ThoughtBranch {
                branch_id: "b2".into(),
                hypothesis: "h2".into(),
                steps: vec![ThoughtStep { step_id: "s2".into(), step_type: StepType::Thought, content: "b".into(), confidence: 0.9 }],
                score: 0.0,
                status: BranchStatus::Active,
            },
        ];
        assert_eq!(TotEngine::select_best(&branches), Some(1));
    }

    #[test]
    fn test_top_k() {
        let branches = vec![
            ThoughtBranch { branch_id: "a".into(), hypothesis: "".into(), steps: vec![ThoughtStep { step_id: "".into(), step_type: StepType::Thought, content: "".into(), confidence: 0.1 }], score: 0.0, status: BranchStatus::Active },
            ThoughtBranch { branch_id: "b".into(), hypothesis: "".into(), steps: vec![ThoughtStep { step_id: "".into(), step_type: StepType::Thought, content: "".into(), confidence: 0.5 }], score: 0.0, status: BranchStatus::Active },
            ThoughtBranch { branch_id: "c".into(), hypothesis: "".into(), steps: vec![ThoughtStep { step_id: "".into(), step_type: StepType::Thought, content: "".into(), confidence: 0.3 }], score: 0.0, status: BranchStatus::Active },
        ];
        let top = TotEngine::select_top_k(&branches, 2);
        assert_eq!(top, vec![1, 2]);  // b (0.5), c (0.3)
    }

    #[test]
    fn test_default_asi_tot_engine() {
        let engine = default_asi_tot_engine();
        assert_eq!(engine.max_branches, 5);
        assert_eq!(engine.max_depth, 8);
    }
}