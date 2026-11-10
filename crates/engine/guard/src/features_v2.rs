//! Joint intent + behavior features for local risk classification.

use serde::{Deserialize, Serialize};

use apeireth_governance::{IntentClass, OperationClass};

use crate::chain::BehaviorChain;
use crate::features::AgentChainFeatureV1;
use crate::intent::AlignmentClass;
use crate::observation::SinkClass;

pub const AGENT_CHAIN_FEATURE_V2: &str = "AgentChainFeatureV2";

/// 动作的 sink class **集合**（去重、与顺序无关）。
///
/// ⚠️ 旧实现只取 `sink_classes.first()` ⇒ 动作对的「目的地是否变了」也只按
/// **首个** sink 判 ⇒ 首个相同但集合不同 ⇒ 判成「没换目的地」⇒ **少计**。
fn sink_class_set(sinks: &[SinkClass]) -> std::collections::HashSet<SinkClass> {
    sinks.iter().copied().collect()
}

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
    #[serde(default)]
    pub sensitive_probe_turn_count: u32,
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
    #[serde(default)]
    pub failed_action_ratio_available: bool,
    pub denial_followup_count: u32,
    pub risk_acceleration: f64,
    pub effect_repetition_count: u32,
    pub scope_expansion_velocity: f64,
    pub cross_turn: CrossTurnRiskSummary,
    #[serde(default)]
    pub target_switch_available: bool,
    #[serde(default)]
    pub destination_switch_available: bool,
    #[serde(default)]
    pub intent_resource_mismatch_available: bool,
    #[serde(default)]
    pub descriptor_source_canonical_ratio: f64,
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
            let operations = action.all_operations();
            if operations.iter().any(|operation| {
                matches!(
                    *operation,
                    OperationClass::NetworkSend | OperationClass::Publish
                )
            }) && !allowed_network
                && !allowed_publish
            {
                unrequested_network_egress = true;
                sink_mismatch_count += 1;
            }
            if operations.iter().any(|operation| {
                matches!(
                    *operation,
                    OperationClass::CredentialRead | OperationClass::CredentialWrite
                )
            }) && !allowed_credentials
            {
                unrequested_credential_access = true;
            }
            if operations.iter().any(|operation| {
                matches!(
                    *operation,
                    OperationClass::Execute | OperationClass::SpawnProcess
                )
            }) && !allowed_shell
            {
                unrequested_shell_execution = true;
            }
            if operations.contains(&OperationClass::Delete) && !allowed_delete {
                unrequested_delete = true;
            }
            if operations.contains(&OperationClass::Publish) && !allowed_publish {
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
        let execution_observed = actions
            .iter()
            .any(|action| !matches!(action.execution_status.as_str(), "" | "not_started"));
        let failed_action_ratio = if action_count == 0 || !execution_observed {
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
        let known_targets = actions
            .iter()
            .filter(|action| {
                action
                    .resource_classes
                    .iter()
                    .any(|class| *class != crate::observation::ResourceClass::Unknown)
            })
            .count();
        let target_switch_available = known_targets >= 2;
        let target_switch_rate = if target_switch_available {
            // ⚠️ **刻意只看首个资源类**（2026-11-05 口径写清，**不是**遗漏）。
            //
            // 同一批里修掉的**同族**问题（`sink_classes.first()` 只取首个）：
            // `external_sink_count`（特征统计 + `fusion.rs:96` 的**判定闸门**）与
            // `destination_switch_rate`（跨动作对比较）⇒ ⇒ 都改成**全集 / 集合**语义。
            //
            // ⇒ 但**同形 ≠ 同族** ⇒ ⇒ 此处保持「首个」：
            // * **sink 集合**：一个动作带 2 个 sink（`[WorkspaceFile, ExternalNetwork]`）
            //   ⇒ ⇒ 真的**多目的地** ⇒ ⇒ 只看首个 = **漏报** ⇒ ⇒ 必须用全集；
            // * **资源类集合**：一个动作触多个资源类是**常态**（管道动作：读文件 +
            //   调网络 + 写日志）⇒ ⇒ 首个资源类 = **主资源**（primary）
            //   ⇒ ⇒ 「目的地是否切换」判的是**主目标**变了没
            //   ⇒ ⇒ 若用集合比较，「同一组资源、不同顺序」也会被算成切换 ⇒ ⇒ **特征被稀释**。
            //
            // ⇒ ⇒ 判据：**改「取首个」之前先问「这里的『首个』是漏报还是本意」**。
            let switches = actions
                .windows(2)
                .filter(|pair| pair[0].resource_classes.first() != pair[1].resource_classes.first())
                .count();
            switches as f64 / f64::from(action_count - 1)
        } else {
            0.0
        };
        let known_destinations = actions
            .iter()
            .filter(|action| {
                action
                    .sink_classes
                    .iter()
                    .any(|class| *class != crate::observation::SinkClass::Unknown)
            })
            .count();
        let destination_switch_available = known_destinations >= 2;
        let destination_switch_rate = if destination_switch_available {
            // ⚠️ 比较 sink **集合**（旧实现只比 `sink_classes.first()` ⇒ 首个相同
            // 但集合不同的动作对被判成「没换目的地」⇒ **少计** ⇒ 同族漏报）。
            let switches = actions
                .windows(2)
                .filter(|pair| {
                    sink_class_set(&pair[0].sink_classes) != sink_class_set(&pair[1].sink_classes)
                })
                .count();
            switches as f64 / f64::from(action_count - 1)
        } else {
            0.0
        };
        let expected_resources = intent
            .map(|value| value.expected_resource_classes.clone())
            .unwrap_or_default();
        let intent_resource_mismatch_available = !expected_resources.is_empty();
        let intent_resource_mismatch_count = if intent_resource_mismatch_available {
            actions
                .iter()
                .filter(|action| {
                    !action.resource_classes.iter().any(|class| {
                        expected_resources
                            .iter()
                            .any(|expected| expected.eq_ignore_ascii_case(&format!("{class:?}")))
                    })
                })
                .count() as u32
        } else {
            0
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
            intent_resource_mismatch_count,
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
            target_switch_rate,
            destination_switch_rate,
            failed_action_ratio,
            failed_action_ratio_available: execution_observed,
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
            target_switch_available,
            destination_switch_available,
            intent_resource_mismatch_available,
            descriptor_source_canonical_ratio: if action_count == 0 {
                0.0
            } else {
                f64::from(
                    actions
                        .iter()
                        .filter(|action| {
                            matches!(
                                action.descriptor_source,
                                crate::semantics::DescriptorSource::Canonical
                            )
                        })
                        .count() as u32,
                ) / f64::from(action_count)
            },
        }
    }
}

impl From<&BehaviorChain> for AgentChainFeatureV2 {
    fn from(value: &BehaviorChain) -> Self {
        Self::from_chain(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::observation::{DataSensitivity, ResourceClass, SafetyObservation, SourceClass};
    use apeireth_governance::OperationClass;

    fn observation(sink_classes: Vec<SinkClass>) -> SafetyObservation {
        SafetyObservation {
            trace_id: "trace-destination-switch".to_string(),
            request_id: "req-1".to_string(),
            session_id: "session-1".to_string(),
            stage: "pre_dispatch".to_string(),
            capability_id: "net.fetch".to_string(),
            tool_name: "fetch".to_string(),
            resource_classes: vec![ResourceClass::NetworkPublic],
            source_classes: vec![SourceClass::PrivateFile],
            sink_classes,
            permission_scope: "workspace".to_string(),
            approval_state: "not_required".to_string(),
            argument_shape: "url=<redacted>".to_string(),
            redacted_argument_features: serde_json::json!({}),
            result_class: None,
            result_size_bucket: None,
            retry_count: 0,
            denied_before: false,
            prior_actions: Vec::new(),
            external_effect: true,
            operation_class: OperationClass::NetworkSend,
            operation_classes: vec![OperationClass::NetworkSend],
            descriptor_source: crate::semantics::DescriptorSource::Canonical,
            data_sensitivity: DataSensitivity::WorkspacePrivate,
            persistent_effect: false,
            destructive_effect: false,
            requires_network: true,
            may_access_credentials: false,
            effect_fingerprint: String::new(),
        }
    }

    /// **跨动作对比较用全集**：两个动作的 sink **集合**不同但**首个相同**
    /// ⇒ 必须判为「换了目的地」（旧口径只比首个 ⇒ 判成没换 ⇒ 少计）。
    #[test]
    fn destination_switch_rate_compares_the_whole_sink_set() {
        let mut chain = BehaviorChain::new("session-1", "trace-destination-switch");
        chain.add_action(
            &observation(vec![SinkClass::ExternalNetwork, SinkClass::ShellExecution]),
            0,
        );
        chain.add_action(&observation(vec![SinkClass::ExternalNetwork]), 1);

        let features = AgentChainFeatureV2::from_chain(&chain);
        assert!(
            features.destination_switch_available,
            "两个动作都有已知 sink ⇒ 目的地切换率必须可用"
        );
        assert_eq!(
            features.destination_switch_rate, 1.0,
            "首个 sink 相同但 sink **集合**不同 ⇒ 必须判为「换了目的地」（首个口径 = 0 ⇒ 少计）"
        );
    }

    /// sink 集合相同（顺序不同也算相同）⇒ 不得误判为切换（回归钉住）。
    #[test]
    fn destination_switch_rate_is_zero_for_the_same_sink_set() {
        let mut chain = BehaviorChain::new("session-1", "trace-same-destination");
        chain.add_action(
            &observation(vec![SinkClass::ExternalNetwork, SinkClass::ShellExecution]),
            0,
        );
        chain.add_action(
            &observation(vec![SinkClass::ShellExecution, SinkClass::ExternalNetwork]),
            1,
        );

        let features = AgentChainFeatureV2::from_chain(&chain);
        assert_eq!(
            features.destination_switch_rate, 0.0,
            "sink 集合相同 ⇒ 不是目的地切换"
        );
    }

    /// 单个动作 ⇒ 没有动作对 ⇒ 切换率必须为 0（不得除零/不得 NaN）。
    #[test]
    fn destination_switch_rate_is_zero_for_a_single_action() {
        let mut chain = BehaviorChain::new("session-1", "trace-single-action");
        chain.add_action(&observation(vec![SinkClass::ExternalNetwork]), 0);

        let features = AgentChainFeatureV2::from_chain(&chain);
        assert_eq!(features.destination_switch_rate, 0.0);
    }
}
