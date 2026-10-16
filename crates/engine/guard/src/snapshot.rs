//! Single FeatureV2 truth snapshot shared by classifier, dataset, and fusion.

use serde::{Deserialize, Serialize};

use crate::features_v2::{AgentChainFeatureV2, AGENT_CHAIN_FEATURE_V2};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureSnapshot {
    pub snapshot_id: String,
    pub schema_version: String,
    pub action_id: String,
    pub trace_id: String,
    pub features: AgentChainFeatureV2,
}

impl FeatureSnapshot {
    pub fn capture(trace_id: &str, action_id: &str, features: AgentChainFeatureV2) -> Self {
        Self {
            snapshot_id: format!("{trace_id}:{action_id}:{AGENT_CHAIN_FEATURE_V2}"),
            schema_version: AGENT_CHAIN_FEATURE_V2.to_string(),
            action_id: action_id.to_string(),
            trace_id: trace_id.to_string(),
            features,
        }
    }
}
