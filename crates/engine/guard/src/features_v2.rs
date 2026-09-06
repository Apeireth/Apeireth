//! Joint intent + behavior features for local risk classification.

use serde::{Deserialize, Serialize};

use apeireth_governance::{IntentClass, OperationClass};

use crate::chain::BehaviorChain;
use crate::features::AgentChainFeatureV1;
use crate::intent::AlignmentClass;

pub const AGENT_CHAIN_FEATURE_V2: &str = "AgentChainFeatureV2";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CrossTurnRiskSummary {
    pub recent_turns: u32,
    pub denied_action_count: u32,
    pub credential_probe_count: u32,
    pub sensitive_read_count: u32,
    pub network_egress_count: u32,
    pub repeated_scope_expansion_count: u32,
    pub repeated_alternate_tool_count: u32,
    pub risk_trend: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentChainFeatureV2 {
    pub schema_version: String,
    pub v1: AgentChainFeatureV1,
    pub intent_class: IntentClass,
    pub intent_confidence: f64,
    pub intent_read_only: bool,
    pub intent_allows_write: bool,
    pub intent_allows_shell: bool,
    pub intent_allows_network: bool,
    pub intent_allows_publish: bool,
    pub intent_allows_credentials: bool,
    pub intent_allows_delete: bool,
    pub expected_capability_count: u32,
    pub expected_resource_class_count: u32,
    pub unexpected_operation_count: u32,
    pub scope_expansion_count: u32,
    pub intent_capability_mismatch_count: u32,
    pub intent_resource_mismatch_count: u32,
    pub intent_sink_mismatch_count: u32,
    pub unrequested_network_egress: bool,
    pub unrequested_credential_access: bool,
    pub unrequested_shell_execution: bool,
    pub unrequested_delete: bool,
    pub unrequested_publish: bool,
    pub unrequested_persistence: bool,
    pub alignment_score: f64,
    pub credential_to_external: bool,
    pub environment_to_external: bool,
    pub private_memory_to_external: bool,
    pub sensitive_to_shell: bool,
    pub taint_age_actions: u32,
    pub sensitive_source_count: u32,
    pub external_sink_count: u32,
    pub capability_switch_rate: f64,
    pub target_switch_rate: f64,
    pub destination_switch_rate: f64,
    pub failed_action_ratio: f64,
    pub denial_followup_count: u32,
    pub risk_acceleration: f64,
    pub effect_repetition_count: u32,
    pub scope_expansion_velocity: f64,
    pub cross_turn: CrossTurnRiskSummary,
}

impl AgentChainFeatureV2 {
    pub fn from_chain(chain: &BehaviorChain) -> Self {
        Self::from_chain_with_cross_turn(chain, CrossTurnRiskSummary::default())
    }

    pub fn from_chain_with_cross_turn(
        chain: &BehaviorChain,
        cross_turn: CrossTurnRiskSummary,
    ) -> Self {
        let v1 = AgentChainFeatureV1::from_chain(chain);
        let intent = chain.intent.as_ref();
        let actions = chain.actions();
        let mut unexpected_operation_count = 0;
        let mut scope_expansion_count = 0;
        let mut capability_mismatch_count = 0;
        let mut sink_mismatch_count = 0;
        let mut alignment_score: f64 = 0.0;
        let mut unrequested_network_egress = false;
        let mut unrequested_credential_access = false;
        let mut unrequested_shell_execution = false;
        let mut unrequested_delete = false;
        let mut unrequested_publish = false;
        let mut unrequested_persistence = false;
        let mut denial_followup_count = 0;
        let mut failed_count = 0;
        let mut effects: std::collections::BTreeMap<String, u32> =
            std::collections::BTreeMap::new();

        for action in &actions {
            match action.alignment_class {
                Some(AlignmentClass::ScopeExpansion) => scope_expansion_count += 1,
                Some(AlignmentClass::Contradictory | AlignmentClass::HighRiskMismatch) => {
                    capability_mismatch_count += 1;
                }
                Some(AlignmentClass::UnexpectedButBenign | AlignmentClass::Unknown) => {
                    unexpected_operation_count += 1;
                }
                _ => {}
            }
            alignment_score = alignment_score.max(match action.alignment_class {
                Some(AlignmentClass::HighRiskMismatch) => 1.0,
                Some(AlignmentClass::Contradictory) => 0.9,
                Some(AlignmentClass::ScopeExpansion) => 0.7,
                Some(AlignmentClass::UnexpectedButBenign) => 0.4,
                _ => 0.0,
            });
            if action.denied && action.operation_class != OperationClass::Unknown {
                denial_followup_count += 1;
            }
            if matches!(action.execution_status.as_str(), "failed") {
                failed_count += 1;
            }
            *effects
                .entry(action.effect_fingerprint.clone())
                .or_default() += 1;
        }

        let intent_class = intent.map_or(IntentClass::Unknown, |value| value.intent_class);
        let allowed_network = intent.is_some_and(|value| value.allows_network());
        let allowed_credentials = intent.is_some_and(|value| value.allows_credentials());
        let allowed_shell = intent.is_some_and(|value| value.allows_shell());
        let allowed_publish = intent.is_some_and(|value| value.allows_publish());
        let allowed_delete = intent.is_some_and(|value| {
            matches!(
                value.destructive_policy,
                apeireth_governance::DestructivePolicy::Allow
            )
        });
        for action in &actions {
            let operation = action.operation_class;
            if matches!(
                operation,
                OperationClass::NetworkSend | OperationClass::Publish
            ) && !allowed_network
                && !allowed_publish
            {
                unrequested_network_egress = true;
                sink_mismatch_count += 1;
            }
            if matches!(
                operation,
                OperationClass::CredentialRead | OperationClass::CredentialWrite
            ) && !allowed_credentials
            {
                unrequested_credential_access = true;
            }
            if matches!(
                operation,
                OperationClass::Execute | OperationClass::SpawnProcess
            ) && !allowed_shell
            {
                unrequested_shell_execution = true;
            }
            if operation == OperationClass::Delete && !allowed_delete {
                unrequested_delete = true;
            }
            if operation == OperationClass::Publish && !allowed_publish {
                unrequested_publish = true;
            }
            if action.persistent_effect
                && intent.is_none_or(|value| {
                    matches!(
                        value.persistence_policy,
                        apeireth_governance::PersistencePolicy::Deny
                    )
                })
            {
                unrequested_persistence = true;
            }
        }
        let action_count = actions.len() as u32;
        let failed_action_ratio = if action_count == 0 {
            0.0
        } else {
            f64::from(failed_count) / f64::from(action_count)
        };
        let transitions = actions
            .windows(2)
            .filter(|pair| pair[0].capability_id != pair[1].capability_id)
            .count();
        let capability_switch_rate = if action_count < 2 {
            0.0
        } else {
            transitions as f64 / f64::from(action_count - 1)
        };
        let effect_repetition_count: u32 =
            effects.values().map(|count| count.saturating_sub(1)).sum();

        Self {
            schema_version: AGENT_CHAIN_FEATURE_V2.to_string(),
            v1: v1.clone(),
            intent_class,
            intent_confidence: intent.map_or(0.0, |value| value.confidence),
            intent_read_only: matches!(intent_class, IntentClass::ReadOnlyInspection),
            intent_allows_write: intent.is_some_and(|value| value.allows_mutation()),
            intent_allows_shell: allowed_shell,
            intent_allows_network: allowed_network,
            intent_allows_publish: allowed_publish,
            intent_allows_credentials: allowed_credentials,
            intent_allows_delete: allowed_delete,
            expected_capability_count: intent
                .map_or(0, |value| value.expected_capability_classes.len() as u32),
            expected_resource_class_count: intent
                .map_or(0, |value| value.expected_resource_classes.len() as u32),
            unexpected_operation_count,
            scope_expansion_count,
            intent_capability_mismatch_count: capability_mismatch_count,
            intent_resource_mismatch_count: 0,
            intent_sink_mismatch_count: sink_mismatch_count,
            unrequested_network_egress,
            unrequested_credential_access,
            unrequested_shell_execution,
            unrequested_delete,
            unrequested_publish,
            unrequested_persistence,
            alignment_score,
            credential_to_external: chain.has_sensitive_source_to_external_sink()
                && v1.credential_access_count > 0,
            environment_to_external: chain.has_sensitive_source_to_external_sink()
                && v1.environment_access_count > 0,
            private_memory_to_external: chain.has_sensitive_source_to_external_sink()
                && v1.private_memory_read_count > 0,
            sensitive_to_shell: v1.sensitive_to_external_flow && v1.process_execution_count > 0,
            taint_age_actions: if v1.sensitive_to_external_flow { 1 } else { 0 },
            sensitive_source_count: v1.sensitive_source_count,
            external_sink_count: v1.external_sink_count,
            capability_switch_rate,
            target_switch_rate: capability_switch_rate,
            destination_switch_rate: capability_switch_rate,
            failed_action_ratio,
            denial_followup_count,
            risk_acceleration: if scope_expansion_count > 0 {
                alignment_score
            } else {
                0.0
            },
            effect_repetition_count,
            scope_expansion_velocity: if action_count == 0 {
                0.0
            } else {
                f64::from(scope_expansion_count) / f64::from(action_count)
            },
            cross_turn,
        }
    }
}

impl From<&BehaviorChain> for AgentChainFeatureV2 {
    fn from(value: &BehaviorChain) -> Self {
        Self::from_chain(value)
    }
}
