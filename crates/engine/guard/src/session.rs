//! Turn-scoped behavior summaries and bounded session history.

use std::collections::VecDeque;

use apeireth_governance::IntentClass;
use serde::{Deserialize, Serialize};

use crate::chain::{ActionStatus, BehaviorChain};
use crate::features_v2::CrossTurnRiskSummary;
use crate::observation::{DataSensitivity, SafetyObservation};

pub const MAX_TURN_HISTORY: usize = 16;
pub const RISK_DECAY: f64 = 0.8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnBehaviorSummary {
    pub trace_id: String,
    pub max_risk_score: f64,
    pub denied: bool,
    pub approval_required: bool,
    pub credential_probe_count: u32,
    pub sensitive_read_count: u32,
    pub network_egress_count: u32,
    pub scope_expansion_count: u32,
    pub alternate_tool_count: u32,
    pub retry_after_denial_count: u32,
    pub destructive_count: u32,
    pub publish_count: u32,
    pub intent_class: IntentClass,
    pub completed_at_ms: i64,
}

impl TurnBehaviorSummary {
    pub fn from_chain(
        chain: &BehaviorChain,
        obs: &SafetyObservation,
        risk_score: f64,
        denied: bool,
        approval_required: bool,
        completed_at_ms: i64,
    ) -> Self {
        let actions = chain.actions();
        let mut credential_probe_count = 0;
        let mut sensitive_read_count = 0;
        let mut network_egress_count = 0;
        let mut scope_expansion_count = 0;
        let mut alternate_tool_count = 0;
        let mut retry_after_denial_count = 0;
        let mut destructive_count = 0;
        let mut publish_count = 0;
        for (index, action) in actions.iter().enumerate() {
            credential_probe_count += u32::from(
                action.may_access_credentials()
                    || matches!(
                        action.operation_class,
                        apeireth_governance::OperationClass::CredentialRead
                            | apeireth_governance::OperationClass::CredentialWrite
                    ),
            );
            sensitive_read_count += u32::from(action.is_sensitive_read());
            network_egress_count += u32::from(action.has_network_egress());
            scope_expansion_count += u32::from(matches!(
                action.alignment_class,
                Some(
                    crate::intent::AlignmentClass::ScopeExpansion
                        | crate::intent::AlignmentClass::Contradictory
                        | crate::intent::AlignmentClass::HighRiskMismatch
                )
            ));
            destructive_count += u32::from(action.destructive_effect);
            publish_count += u32::from(
                action
                    .operation_classes
                    .contains(&apeireth_governance::OperationClass::Publish)
                    || action.operation_class == apeireth_governance::OperationClass::Publish,
            );
            if index > 0 {
                let previous = actions[index - 1];
                if previous.denied || previous.status == ActionStatus::Denied {
                    retry_after_denial_count += 1;
                    if previous.capability_id != action.capability_id {
                        alternate_tool_count += 1;
                    }
                }
            }
        }
        if actions.is_empty() {
            credential_probe_count = u32::from(obs.may_access_credentials);
            sensitive_read_count = u32::from(matches!(
                obs.data_sensitivity,
                DataSensitivity::Credential
                    | DataSensitivity::Secret
                    | DataSensitivity::MemoryPrivate
            ));
            network_egress_count = u32::from(obs.requires_network && obs.external_effect);
            destructive_count = u32::from(obs.destructive_effect);
        }
        Self {
            trace_id: chain.trace_id.clone(),
            max_risk_score: risk_score,
            denied,
            approval_required,
            credential_probe_count,
            sensitive_read_count,
            network_egress_count,
            scope_expansion_count,
            alternate_tool_count,
            retry_after_denial_count,
            destructive_count,
            publish_count,
            intent_class: chain
                .intent
                .as_ref()
                .map(|intent| intent.intent_class)
                .unwrap_or(IntentClass::Unknown),
            completed_at_ms,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionBehaviorHistory {
    pub turns: VecDeque<TurnBehaviorSummary>,
}

impl SessionBehaviorHistory {
    pub fn upsert(&mut self, summary: TurnBehaviorSummary) {
        if let Some(existing) = self
            .turns
            .back_mut()
            .filter(|current| current.trace_id == summary.trace_id)
        {
            existing.max_risk_score = existing.max_risk_score.max(summary.max_risk_score);
            existing.denied |= summary.denied;
            existing.approval_required |= summary.approval_required;
            existing.credential_probe_count = summary.credential_probe_count;
            existing.sensitive_read_count = summary.sensitive_read_count;
            existing.network_egress_count = summary.network_egress_count;
            existing.scope_expansion_count = summary.scope_expansion_count;
            existing.alternate_tool_count = summary.alternate_tool_count;
            existing.retry_after_denial_count = summary.retry_after_denial_count;
            existing.destructive_count = summary.destructive_count;
            existing.publish_count = summary.publish_count;
            existing.intent_class = summary.intent_class;
            existing.completed_at_ms = summary.completed_at_ms;
            return;
        }
        if self.turns.len() >= MAX_TURN_HISTORY {
            self.turns.pop_front();
        }
        self.turns.push_back(summary);
    }

    pub fn recent_turn_count(&self) -> u32 {
        self.turns.len() as u32
    }

    pub fn cross_turn_features(&self, current_trace: &str) -> CrossTurnRiskSummary {
        let completed: Vec<&TurnBehaviorSummary> = self
            .turns
            .iter()
            .filter(|turn| turn.trace_id != current_trace)
            .collect();
        let n = completed.len();
        let mut denied_action_count = 0.0;
        let mut credential_probe_count = 0.0;
        let mut sensitive_read_count = 0.0;
        let mut network_egress_count = 0.0;
        let mut repeated_scope_expansion_count = 0.0;
        let mut repeated_alternate_tool_count = 0.0;
        let mut risk_trend = 0.0;
        for (index, turn) in completed.iter().rev().enumerate() {
            let weight = RISK_DECAY.powi(index as i32);
            denied_action_count += weight * f64::from(u8::from(turn.denied));
            credential_probe_count += weight * f64::from(turn.credential_probe_count);
            sensitive_read_count += weight * f64::from(turn.sensitive_read_count);
            network_egress_count += weight * f64::from(turn.network_egress_count);
            repeated_scope_expansion_count += weight * f64::from(turn.scope_expansion_count);
            repeated_alternate_tool_count += weight * f64::from(turn.alternate_tool_count);
            risk_trend += weight * turn.max_risk_score;
        }
        let denom = if n == 0 {
            1.0
        } else {
            (0..n).map(|age| RISK_DECAY.powi(age as i32)).sum()
        };
        CrossTurnRiskSummary {
            recent_turns: self.recent_turn_count(),
            denied_action_count: denied_action_count.round() as u32,
            credential_probe_count: credential_probe_count.round() as u32,
            sensitive_read_count: sensitive_read_count.round() as u32,
            network_egress_count: network_egress_count.round() as u32,
            repeated_scope_expansion_count: repeated_scope_expansion_count.round() as u32,
            repeated_alternate_tool_count: repeated_alternate_tool_count.round() as u32,
            risk_trend: (risk_trend / denom).clamp(0.0, 1.0),
            sensitive_probe_turn_count: completed
                .iter()
                .filter(|turn| turn.credential_probe_count > 0 || turn.sensitive_read_count > 0)
                .count() as u32,
        }
    }
}
