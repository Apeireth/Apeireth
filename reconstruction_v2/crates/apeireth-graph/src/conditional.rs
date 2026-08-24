//! Conditional Edge — LangGraph `add_conditional_edges` 1:1.

use std::collections::BTreeMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::{NodeId, State};

/// LangGraph END sentinel.
pub const END_LABEL: &str = "__end__";

/// Conditional edge.
#[derive(Clone)]
pub struct ConditionalEdge {
    pub from: NodeId,
    pub path_map: BTreeMap<String, NodeId>,
    pub default: Option<NodeId>,
    pub condition: Arc<dyn Fn(&State) -> String + Send + Sync>,
}

impl std::fmt::Debug for ConditionalEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConditionalEdge")
            .field("from", &self.from)
            .field("path_map", &self.path_map)
            .field("default", &self.default)
            .finish()
    }
}

/// Conditional decision result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalDecision {
    /// Go to target.
    GoTo(NodeId),
    /// End execution.
    End,
    /// No match, use default if available.
    Default,
}

impl ConditionalEdge {
    /// Evaluate condition and produce a decision.
    pub fn decide(&self, state: &State) -> ConditionalDecision {
        let label = (self.condition)(state);
        if label == END_LABEL {
            return ConditionalDecision::End;
        }
        if let Some(target) = self.path_map.get(&label) {
            return ConditionalDecision::GoTo(target.clone());
        }
        if let Some(default) = &self.default {
            return ConditionalDecision::GoTo(default.clone());
        }
        ConditionalDecision::Default
    }
}

/// Conditional error.
#[derive(Debug, Error)]
pub enum ConditionalError {
    #[error("conditional source `{0}` not found in graph")]
    SourceNotFound(NodeId),
    #[error("conditional target `{0}` not found in graph")]
    TargetNotFound(NodeId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_returns_goto_on_match() {
        let mut pm = BTreeMap::new();
        pm.insert("left".into(), "L".to_string());
        let ce = ConditionalEdge {
            from: "src".into(),
            path_map: pm,
            default: Some("D".into()),
            condition: Arc::new(|_| "left".to_string()),
        };
        assert!(matches!(ce.decide(&State::new()), ConditionalDecision::GoTo(ref t) if t == "L"));
    }

    #[test]
    fn decide_returns_default_on_miss() {
        let ce = ConditionalEdge {
            from: "src".into(),
            path_map: BTreeMap::new(),
            default: Some("D".into()),
            condition: Arc::new(|_| "unknown".to_string()),
        };
        assert!(matches!(ce.decide(&State::new()), ConditionalDecision::GoTo(ref t) if t == "D"));
    }

    #[test]
    fn decide_end_label() {
        let ce = ConditionalEdge {
            from: "src".into(),
            path_map: BTreeMap::new(),
            default: None,
            condition: Arc::new(|_| END_LABEL.to_string()),
        };
        assert!(matches!(ce.decide(&State::new()), ConditionalDecision::End));
    }
}
