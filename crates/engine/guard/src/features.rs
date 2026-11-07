//! Stable, desensitized Agent behavior-chain features.
//!
//! `AgentChainFeatureV1` is a versioned contract. Paths, prompts, file
//! contents, memory bodies, credentials, and reasoning text never cross this
//! boundary; only classes, counts, ratios, and hashed/normalized identities do.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::chain::{ActionStatus, BehaviorChain};
use crate::observation::{SinkClass, SourceClass};

pub const AGENT_CHAIN_FEATURE_V1: &str = "AgentChainFeatureV1";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AgentChainFeatureV1 {
    pub schema_version: String,
    pub action_count: u32,
    pub unique_capability_count: u32,
    pub read_count: u32,
    pub write_count: u32,
    pub delete_count: u32,
    pub execute_count: u32,
    pub network_count: u32,
    pub external_effect_count: u32,
    pub sensitive_source_count: u32,
    pub credential_access_count: u32,
    pub private_memory_read_count: u32,
    pub environment_access_count: u32,
    pub external_sink_count: u32,
    pub network_egress_count: u32,
    pub process_execution_count: u32,
    pub denied_count: u32,
    pub approval_count: u32,
    pub retry_count: u32,
    pub scope_mismatch_count: u32,
    pub path_escape_count: u32,
    pub max_chain_depth: u32,
    pub branch_count: u32,
    pub mutation_ratio: f64,
    pub read_write_ratio: f64,
    pub retry_after_denial: bool,
    pub alternate_tool_after_denial: bool,
    pub sensitive_to_external_flow: bool,
    pub source_sink_distance: u32,
    pub tool_transition_features: BTreeMap<String, u32>,
}

impl AgentChainFeatureV1 {
    pub fn from_chain(chain: &BehaviorChain) -> Self {
        let actions = chain.actions();
        let action_count = actions.len() as u32;
        let mut capabilities = std::collections::BTreeSet::new();
        let mut read_count = 0;
        let mut write_count = 0;
        let mut delete_count = 0;
        let mut execute_count = 0;
        let mut network_count = 0;
        let mut external_effect_count = 0;
        let mut sensitive_source_count = 0;
        let mut credential_access_count = 0;
        let mut private_memory_read_count = 0;
        let mut environment_access_count = 0;
        let mut external_sink_count = 0;
        let mut network_egress_count = 0;
        let mut process_execution_count = 0;
        let mut denied_count = 0;
        let mut approval_count = 0;
        let mut retry_count = 0;
        let mut scope_mismatch_count = 0;
        let mut path_escape_count = 0;
        let mut transitions = BTreeMap::new();

        for action in &actions {
            let cap = action.capability_id.to_lowercase();
            capabilities.insert(action.capability_id.clone());
            let is_read = cap.contains("read") || cap.contains("search") || cap.contains("list");
            let is_write = cap.contains("write") || cap.contains("edit") || cap.contains("update");
            let is_delete =
                cap.contains("delete") || cap.contains("remove") || cap.contains("unlink");
            let is_execute =
                cap.contains("shell") || cap.contains("exec") || cap.contains("process");
            let is_network =
                cap.contains("fetch") || cap.contains("http") || cap.contains("network");
            read_count += u32::from(is_read);
            write_count += u32::from(is_write);
            delete_count += u32::from(is_delete);
            execute_count += u32::from(is_execute);
            network_count += u32::from(is_network);
            external_effect_count += u32::from(action.external_effect);
            denied_count += u32::from(action.denied || action.status == ActionStatus::Denied);
            approval_count += u32::from(action.status == ActionStatus::RequireApproval);
            retry_count += u32::from(action.round > 1);
            credential_access_count +=
                u32::from(cap.contains("credential") || cap.contains("secret"));
            environment_access_count += u32::from(cap.contains("env"));
            process_execution_count += u32::from(is_execute);
            network_egress_count += u32::from(is_network && action.external_effect);
            scope_mismatch_count += u32::from(
                chain
                    .declared_task_scope
                    .as_deref()
                    .is_some_and(|scope| scope.to_lowercase().contains("read_only") && !is_read),
            );
            path_escape_count +=
                u32::from(cap.contains("path_escape") || cap.contains("traversal"));
            if let Some(previous) = actions
                .iter()
                .take_while(|candidate| candidate.id != action.id)
                .last()
            {
                let key = format!("{}>{}", previous.capability_id, action.capability_id);
                *transitions.entry(key).or_default() += 1;
            }
        }

        for node in &chain.nodes {
            if let crate::chain::BehaviorNode::Data(data) = node {
                let sensitive = matches!(
                    data.source,
                    SourceClass::PrivateFile
                        | SourceClass::PrivateMemory
                        | SourceClass::CredentialStore
                        | SourceClass::Environment
                );
                sensitive_source_count += u32::from(sensitive);
                private_memory_read_count += u32::from(data.source == SourceClass::PrivateMemory);
                // ⚠️ 用 sink **全集**计数（旧实现只看 `data.sink`＝**首个** ⇒
                // 多 sink 节点（如 `[WorkspaceFile, ExternalNetwork]`）**少计**
                // ⇒ 该特征喂给分类/学习时也**少算** ⇒ 同族漏报）。
                //
                // 语义：`external_sink_count` = **外部 sink 的数量** ⇒ 节点上出现
                // 几个外部 sink（`ExternalNetwork` / `ShellExecution`）就计几个
                // ⇒ 全集里一个都不丢（旧口径每个节点最多计 1，且只看首个）。
                external_sink_count += data
                    .sink_classes()
                    .iter()
                    .filter(|sink| {
                        matches!(sink, SinkClass::ExternalNetwork | SinkClass::ShellExecution)
                    })
                    .count() as u32;
            }
        }
        let retry_after_denial = chain.has_retry_escalation();
        let alternate_tool_after_denial = actions.windows(2).any(|window| {
            window[0].denied
                && window[0].capability_id != window[1].capability_id
                && semantic_category(&window[0].capability_id)
                    == semantic_category(&window[1].capability_id)
        });
        let read_write_ratio = if read_count + write_count == 0 {
            0.0
        } else {
            f64::from(read_count) / f64::from(read_count + write_count)
        };
        let mutation_ratio = if action_count == 0 {
            0.0
        } else {
            f64::from(write_count + delete_count + execute_count) / f64::from(action_count)
        };
        Self {
            schema_version: AGENT_CHAIN_FEATURE_V1.to_string(),
            action_count,
            unique_capability_count: capabilities.len() as u32,
            read_count,
            write_count,
            delete_count,
            execute_count,
            network_count,
            external_effect_count,
            sensitive_source_count,
            credential_access_count,
            private_memory_read_count,
            environment_access_count,
            external_sink_count,
            network_egress_count,
            process_execution_count,
            denied_count,
            approval_count,
            retry_count,
            scope_mismatch_count,
            path_escape_count,
            max_chain_depth: action_count,
            branch_count: chain
                .edges
                .iter()
                .filter(|edge| edge.from != edge.to)
                .count()
                .saturating_sub(action_count.saturating_sub(1) as usize)
                as u32,
            mutation_ratio,
            read_write_ratio,
            retry_after_denial,
            alternate_tool_after_denial,
            sensitive_to_external_flow: chain.has_sensitive_source_to_external_sink(),
            source_sink_distance: if chain.has_sensitive_source_to_external_sink() {
                1
            } else {
                0
            },
            tool_transition_features: transitions,
        }
    }
}

fn semantic_category(capability: &str) -> &str {
    let cap = capability.to_lowercase();
    if cap.contains("network") || cap.contains("fetch") || cap.contains("http") {
        "network_egress"
    } else if cap.contains("shell") || cap.contains("exec") || cap.contains("process") {
        "process_execution"
    } else if cap.contains("file") || cap.contains("fs") {
        "filesystem"
    } else {
        "other"
    }
}

impl From<&BehaviorChain> for AgentChainFeatureV1 {
    fn from(value: &BehaviorChain) -> Self {
        Self::from_chain(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::chain::{BehaviorNode, DataNode};
    use crate::observation::{DataSensitivity, ResourceClass, SafetyObservation};
    use apeireth_governance::OperationClass;

    fn observation(
        source_classes: Vec<SourceClass>,
        sink_classes: Vec<SinkClass>,
    ) -> SafetyObservation {
        SafetyObservation {
            trace_id: "trace-feature-sink".to_string(),
            request_id: "req-1".to_string(),
            session_id: "session-1".to_string(),
            stage: "pre_dispatch".to_string(),
            capability_id: "fs.read".to_string(),
            tool_name: "read_file".to_string(),
            resource_classes: vec![ResourceClass::FilesystemWorkspace],
            source_classes,
            sink_classes,
            permission_scope: "workspace".to_string(),
            approval_state: "not_required".to_string(),
            argument_shape: "path=<redacted>".to_string(),
            redacted_argument_features: serde_json::json!({}),
            result_class: None,
            result_size_bucket: None,
            retry_count: 0,
            denied_before: false,
            prior_actions: Vec::new(),
            external_effect: false,
            operation_class: OperationClass::Read,
            operation_classes: vec![OperationClass::Read],
            descriptor_source: crate::semantics::DescriptorSource::Canonical,
            data_sensitivity: DataSensitivity::WorkspacePrivate,
            persistent_effect: false,
            destructive_effect: false,
            requires_network: false,
            may_access_credentials: false,
            effect_fingerprint: String::new(),
        }
    }

    #[test]
    fn feature_schema_is_stable_and_does_not_include_raw_content() {
        let chain = BehaviorChain::new("session", "trace");
        let features = AgentChainFeatureV1::from_chain(&chain);
        assert_eq!(features.schema_version, AGENT_CHAIN_FEATURE_V1);
        let json = serde_json::to_string(&features).unwrap();
        assert!(!json.contains("prompt"));
    }

    /// **多 sink 节点的特征统计按全集**：出网 sink **不是首个** ⇒ 必须被计入
    /// （旧口径只看 `data.sink`＝首个 ⇒ 计 0 ⇒ 少计）。
    #[test]
    fn external_sink_count_counts_the_whole_sink_set() {
        let mut chain = BehaviorChain::new("session-1", "trace-multi-sink-feature");
        chain.add_action(
            &observation(
                vec![SourceClass::PrivateFile],
                vec![SinkClass::WorkspaceFile, SinkClass::ExternalNetwork],
            ),
            0,
        );

        let features = AgentChainFeatureV1::from_chain(&chain);
        assert_eq!(
            features.external_sink_count, 1,
            "出网 sink 在全集里 ⇒ external_sink_count 必须按全集计（首个口径 = 0 ⇒ 少计）"
        );
    }

    /// 全集里有几个外部 sink 就计几个（不按节点封顶 1）。
    #[test]
    fn external_sink_count_counts_every_external_sink_class() {
        let mut chain = BehaviorChain::new("session-1", "trace-two-external-sinks");
        chain.add_action(
            &observation(
                vec![SourceClass::PrivateFile],
                vec![SinkClass::ExternalNetwork, SinkClass::ShellExecution],
            ),
            0,
        );

        let features = AgentChainFeatureV1::from_chain(&chain);
        assert_eq!(
            features.external_sink_count, 2,
            "两个外部 sink ⇒ 必须都计入（首个口径 = 1 ⇒ 少计）"
        );
    }

    /// 非外部 sink **不得**被误计（回归钉住：写工作区文件不算出网）。
    #[test]
    fn external_sink_count_ignores_non_external_sinks() {
        let mut chain = BehaviorChain::new("session-1", "trace-workspace-sink");
        chain.add_action(
            &observation(
                vec![SourceClass::PrivateFile],
                vec![SinkClass::WorkspaceFile, SinkClass::MemoryWrite],
            ),
            0,
        );

        let features = AgentChainFeatureV1::from_chain(&chain);
        assert_eq!(
            features.external_sink_count, 0,
            "写工作区/内存不是外部 sink ⇒ 不得计入"
        );
    }

    /// 兼容：旧数据只有 `sink` 字段（`sinks` 为空）⇒ 仍按旧字段计入。
    #[test]
    fn external_sink_count_falls_back_to_the_legacy_sink_field() {
        let mut chain = BehaviorChain::new("session-1", "trace-legacy-sink");
        chain.nodes.push(BehaviorNode::Data(DataNode {
            id: "data_legacy".to_string(),
            source: SourceClass::PrivateFile,
            sink: Some(SinkClass::ExternalNetwork),
            sinks: Vec::new(),
            label: "PrivateFile->ExternalNetwork".to_string(),
        }));

        let features = AgentChainFeatureV1::from_chain(&chain);
        assert_eq!(
            features.external_sink_count, 1,
            "旧数据（只有 `sink`）⇒ 必须仍被计入（否则旧轨迹的特征会突然掉 0）"
        );
    }
}
