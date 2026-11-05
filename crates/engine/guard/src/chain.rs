//! In-memory behavior chain representation.
//!
//! Stores turn-local action, resource, and data nodes with directed edges
//! capturing temporal sequence, causal impact, data flows, and permission
//! dependencies.

use apeireth_governance::{OperationClass, TaskIntentEnvelopeV1};
use serde::{Deserialize, Serialize};

use crate::intent::AlignmentClass;
use crate::observation::{ResourceClass, SafetyObservation, SinkClass, SourceClass};

/// Directed edge types in a behavior chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Temporal,
    Causal,
    DataFlow,
    PermissionDependency,
    ResourceDependency,
    Retry,
    AlternativeExecution,
    DerivedFrom,
    SameTarget,
    SameEffect,
    Escalation,
    DestinationChange,
}

/// Action node in the behavior chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionNode {
    pub id: String,
    pub round: u32,
    pub capability_id: String,
    pub tool_name: String,
    pub argument_shape: String,
    pub status: ActionStatus,
    pub denied: bool,
    pub external_effect: bool,
    #[serde(default)]
    pub operation_class: OperationClass,
    #[serde(default)]
    pub operation_classes: Vec<OperationClass>,
    #[serde(default)]
    pub resource_classes: Vec<ResourceClass>,
    #[serde(default)]
    pub source_classes: Vec<SourceClass>,
    #[serde(default)]
    pub sink_classes: Vec<SinkClass>,
    #[serde(default)]
    pub alignment_class: Option<AlignmentClass>,
    #[serde(default)]
    pub approval_status: String,
    #[serde(default)]
    pub execution_status: String,
    #[serde(default)]
    pub persistent_effect: bool,
    #[serde(default)]
    pub destructive_effect: bool,
    #[serde(default)]
    pub effect_fingerprint: String,
    #[serde(default)]
    pub descriptor_source: crate::semantics::DescriptorSource,
}

/// Status of an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    Pending,
    Allowed,
    Denied,
    RequireApproval,
    Succeeded,
    Failed,
}

/// Resource node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub id: String,
    pub class: ResourceClass,
    pub target: String,
}

/// Data node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataNode {
    pub id: String,
    pub source: SourceClass,
    /// **首个** sink class（兼容字段：旧数据/旧消费者按它读）。
    pub sink: Option<SinkClass>,
    /// **全部** sink class（一个动作可能有多条出向 sink ⇒ 只留首个会**漏报**）。
    ///
    /// ⚠️ 旧实现只存 `sink_classes.first()` ⇒ ⇒ 「读私有文件 → 出网」这类
    /// **多 sink** 场景**检测不到**（漏报）。⇒ ⇒ 全集留在这里，下游判定用全集。
    #[serde(default)]
    pub sinks: Vec<SinkClass>,
    pub label: String,
}

impl DataNode {
    /// 该节点的**全部** sink class（旧数据只有 `sink` ⇒ 回落到它）。
    pub fn sink_classes(&self) -> Vec<SinkClass> {
        if self.sinks.is_empty() {
            self.sink.into_iter().collect()
        } else {
            self.sinks.clone()
        }
    }
}

/// Node variants in the behavior graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BehaviorNode {
    Action(ActionNode),
    Resource(ResourceNode),
    Data(DataNode),
}

/// Directed edge between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub label: Option<String>,
}

/// Turn-local behavior chain for a single request / trace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorChain {
    pub trace_id: String,
    pub session_id: String,
    pub nodes: Vec<BehaviorNode>,
    pub edges: Vec<BehaviorEdge>,
    pub declared_task_scope: Option<String>,
    #[serde(default)]
    pub intent: Option<TaskIntentEnvelopeV1>,
}

impl BehaviorChain {
    /// Create a new behavior chain for a session and trace.
    pub fn new(session_id: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            trace_id: trace_id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            declared_task_scope: None,
            intent: None,
        }
    }

    /// Set the declared user intent / task scope (e.g. "read_only", "explore", "edit").
    pub fn set_declared_scope(&mut self, scope: impl Into<String>) {
        self.declared_task_scope = Some(scope.into());
    }

    pub fn set_intent(&mut self, intent: TaskIntentEnvelopeV1) {
        self.declared_task_scope = intent.allowed_scopes.first().cloned();
        self.intent = Some(intent);
    }

    /// Add an action observation to the chain and wire temporal and dataflow edges.
    pub fn add_action(&mut self, obs: &SafetyObservation, round: u32) -> String {
        self.add_action_with_id(obs, round, None)
    }

    /// Add an action using an identity supplied by the runtime when one is
    /// available (for example a provider tool-call id).  The fallback keeps
    /// the historic deterministic identity for callers that only have a
    /// governance request.
    pub fn add_action_with_id(
        &mut self,
        obs: &SafetyObservation,
        round: u32,
        requested_id: Option<&str>,
    ) -> String {
        let sequence = self.actions().len() as u32;
        let action_id = requested_id
            .filter(|id| !id.trim().is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("act:{}:{}:{}", obs.request_id, round, sequence));

        // Previous action for temporal edge
        let prev_action_id = self.nodes.iter().rev().find_map(|n| match n {
            BehaviorNode::Action(act) => Some(act.id.clone()),
            _ => None,
        });

        let action_node = ActionNode {
            id: action_id.clone(),
            round,
            capability_id: obs.capability_id.clone(),
            tool_name: obs.tool_name.clone(),
            argument_shape: obs.argument_shape.clone(),
            status: ActionStatus::Pending,
            denied: false,
            external_effect: obs.external_effect,
            operation_class: obs.operation_class,
            operation_classes: obs.all_operations(),
            resource_classes: obs.resource_classes.clone(),
            source_classes: obs.source_classes.clone(),
            sink_classes: obs.sink_classes.clone(),
            alignment_class: None,
            approval_status: "pending".to_string(),
            execution_status: "not_started".to_string(),
            persistent_effect: obs.persistent_effect,
            destructive_effect: obs.destructive_effect,
            effect_fingerprint: obs.effect_fingerprint.clone(),
            descriptor_source: obs.descriptor_source,
        };
        self.nodes.push(BehaviorNode::Action(action_node));

        if let Some(prev) = prev_action_id {
            self.edges.push(BehaviorEdge {
                from: prev.clone(),
                to: action_id.clone(),
                edge_type: EdgeType::Temporal,
                label: Some("seq".to_string()),
            });
            let previous_action = {
                let actions = self.actions();
                actions.iter().rev().nth(1).map(|action| (*action).clone())
            };
            if let Some(previous_action) = previous_action {
                if previous_action.denied || previous_action.status == ActionStatus::Denied {
                    self.edges.push(BehaviorEdge {
                        from: prev.clone(),
                        to: action_id.clone(),
                        edge_type: if previous_action.capability_id == obs.capability_id {
                            EdgeType::Retry
                        } else {
                            EdgeType::AlternativeExecution
                        },
                        label: Some("post_denial".to_string()),
                    });
                }
                if previous_action.effect_fingerprint == obs.effect_fingerprint
                    && !obs.effect_fingerprint.is_empty()
                {
                    self.edges.push(BehaviorEdge {
                        from: prev.clone(),
                        to: action_id.clone(),
                        edge_type: EdgeType::SameEffect,
                        label: Some("same_semantic_effect".to_string()),
                    });
                    if previous_action.denied || previous_action.status == ActionStatus::Denied {
                        self.edges.push(BehaviorEdge {
                            from: prev.clone(),
                            to: action_id.clone(),
                            edge_type: EdgeType::Escalation,
                            label: Some("same_effect_after_denial".to_string()),
                        });
                    }
                }
            }
        }

        // Add resource nodes & edges
        for res_class in &obs.resource_classes {
            let res_id = format!("res_{:?}_{}", res_class, self.nodes.len());
            self.nodes.push(BehaviorNode::Resource(ResourceNode {
                id: res_id.clone(),
                class: *res_class,
                target: obs.capability_id.clone(),
            }));
            self.edges.push(BehaviorEdge {
                from: action_id.clone(),
                to: res_id,
                edge_type: EdgeType::ResourceDependency,
                label: Some("accesses".to_string()),
            });
        }

        // Add data nodes & edges
        for src_class in &obs.source_classes {
            let data_id = format!("data_{:?}_{}", src_class, self.nodes.len());
            // ⚠️ **全部** sink class 都要留（只留首个 ⇒ 多 sink 场景**漏报**）。
            let sink_classes = obs.sink_classes.clone();
            let sink_class = sink_classes.first().copied();
            let label = match sink_classes.len() {
                0 => format!("{src_class:?}->None"),
                1 => format!("{src_class:?}->{:?}", sink_classes[0]),
                _ => format!("{src_class:?}->{sink_classes:?}"),
            };
            self.nodes.push(BehaviorNode::Data(DataNode {
                id: data_id.clone(),
                source: *src_class,
                sink: sink_class,
                sinks: sink_classes,
                label,
            }));
            self.edges.push(BehaviorEdge {
                from: data_id,
                to: action_id.clone(),
                edge_type: EdgeType::DataFlow,
                label: Some("consumes".to_string()),
            });
        }

        action_id
    }

    /// Update status of an action node.
    pub fn update_action_status(&mut self, action_id: &str, status: ActionStatus) {
        for node in &mut self.nodes {
            if let BehaviorNode::Action(act) = node {
                if act.id == action_id {
                    act.status = status;
                    if matches!(status, ActionStatus::Denied) {
                        act.denied = true;
                    }
                    act.approval_status = match status {
                        ActionStatus::RequireApproval => "pending".to_string(),
                        ActionStatus::Allowed | ActionStatus::Succeeded => {
                            "not_required".to_string()
                        }
                        ActionStatus::Denied => "denied".to_string(),
                        _ => act.approval_status.clone(),
                    };
                    break;
                }
            }
        }
    }

    pub fn set_action_alignment(&mut self, action_id: &str, alignment: AlignmentClass) {
        if let Some(BehaviorNode::Action(action)) = self.nodes.iter_mut().find(
            |node| matches!(node, BehaviorNode::Action(candidate) if candidate.id == action_id),
        ) {
            action.alignment_class = Some(alignment);
        }
    }

    pub fn has_same_effect_after_denial(&self) -> bool {
        self.edges.iter().any(|edge| {
            edge.edge_type == EdgeType::SameEffect
                && self.edges.iter().any(|other| {
                    other.from == edge.from
                        && other.to == edge.to
                        && matches!(
                            other.edge_type,
                            EdgeType::Escalation | EdgeType::AlternativeExecution
                        )
                })
        })
    }

    /// Retrieve all action nodes.
    pub fn actions(&self) -> Vec<&ActionNode> {
        self.nodes
            .iter()
            .filter_map(|n| match n {
                BehaviorNode::Action(act) => Some(act),
                _ => None,
            })
            .collect()
    }

    /// Retrieve an action by its stable identity.
    pub fn action_by_id(&self, action_id: &str) -> Option<&ActionNode> {
        self.nodes.iter().find_map(|node| match node {
            BehaviorNode::Action(action) if action.id == action_id => Some(action),
            _ => None,
        })
    }
    /// Check whether a sensitive source was read prior to an external sink write.
    ///
    /// ⚠️ **用 sink 全集判定**（旧实现只看 `sink_classes.first()` ⇒ ⇒ 多 sink
    /// 场景「读私有文件 → 出网」**漏报**）。
    pub fn has_sensitive_source_to_external_sink(&self) -> bool {
        let mut saw_sensitive_source = false;
        for node in &self.nodes {
            match node {
                BehaviorNode::Data(data) => {
                    if matches!(
                        data.source,
                        SourceClass::PrivateFile
                            | SourceClass::PrivateMemory
                            | SourceClass::CredentialStore
                            | SourceClass::Environment
                    ) {
                        saw_sensitive_source = true;
                    }
                    if saw_sensitive_source
                        && data.sink_classes().iter().any(|sink| {
                            matches!(sink, SinkClass::ExternalNetwork | SinkClass::ShellExecution)
                        })
                    {
                        return true;
                    }
                }
                BehaviorNode::Action(act) => {
                    let cap = act.capability_id.to_lowercase();
                    if cap.contains("credential") || cap.contains("secret") || cap.contains("env") {
                        saw_sensitive_source = true;
                    }
                    if saw_sensitive_source
                        && (cap.contains("fetch")
                            || cap.contains("http")
                            || cap.contains("curl")
                            || cap.contains("network"))
                    {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check for retry escalation pattern:
    /// An action was denied, followed immediately by parameter perturbation or alternative tool.
    pub fn has_retry_escalation(&self) -> bool {
        let actions = self.actions();
        if actions.len() < 2 {
            return false;
        }

        for window in actions.windows(2) {
            let prev = window[0];
            let curr = window[1];

            if prev.denied || prev.status == ActionStatus::Denied {
                // If the next action attempts shell, process, or external network right after a denial
                let curr_cap = curr.capability_id.to_lowercase();
                if curr_cap.contains("shell")
                    || curr_cap.contains("exec")
                    || curr_cap.contains("fetch")
                    || curr_cap.contains("fs.delete")
                    || curr_cap == prev.capability_id
                {
                    return true;
                }
            }
        }
        false
    }

    /// Check for privilege escalation chain:
    /// e.g. read config/env -> extract credential -> invoke network.
    pub fn has_privilege_escalation(&self) -> bool {
        let actions = self.actions();
        let mut read_config = false;
        let mut discovered_cred = false;

        for act in actions {
            let cap = act.capability_id.to_lowercase();
            if cap.contains("fs.read") || cap.contains("read_file") {
                read_config = true;
            }
            if read_config
                && (cap.contains("keyring") || cap.contains("secret") || cap.contains("credential"))
            {
                discovered_cred = true;
            }
            if discovered_cred
                && (cap.contains("fetch") || cap.contains("shell") || cap.contains("http"))
            {
                return true;
            }
        }
        false
    }

    /// Check for destructive sequence:
    /// e.g. discover -> modify -> delete -> publish.
    pub fn has_destructive_chain(&self) -> bool {
        let actions = self.actions();
        let mut saw_delete = false;
        let mut saw_external = false;

        for act in actions {
            let cap = act.capability_id.to_lowercase();
            if cap.contains("delete") || cap.contains("remove") || cap.contains("unlink") {
                saw_delete = true;
            }
            if saw_delete
                && (cap.contains("fetch") || cap.contains("network") || cap.contains("publish"))
            {
                saw_external = true;
            }
        }
        saw_delete && saw_external
    }
}

impl ActionNode {
    pub fn all_operations(&self) -> Vec<OperationClass> {
        if self.operation_classes.is_empty() {
            vec![self.operation_class]
        } else {
            self.operation_classes.clone()
        }
    }

    pub fn may_access_credentials(&self) -> bool {
        self.source_classes.contains(&SourceClass::CredentialStore)
            || self
                .resource_classes
                .contains(&ResourceClass::CredentialStore)
            || self.all_operations().iter().any(|operation| {
                matches!(
                    *operation,
                    OperationClass::CredentialRead | OperationClass::CredentialWrite
                )
            })
    }

    pub fn is_sensitive_read(&self) -> bool {
        self.source_classes.iter().any(|source| {
            matches!(
                source,
                SourceClass::CredentialStore
                    | SourceClass::Environment
                    | SourceClass::PrivateMemory
                    | SourceClass::PrivateFile
            )
        })
    }

    pub fn has_network_egress(&self) -> bool {
        self.sink_classes.contains(&SinkClass::ExternalNetwork)
            || self.all_operations().iter().any(|operation| {
                matches!(
                    *operation,
                    OperationClass::NetworkSend | OperationClass::Publish
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::DataSensitivity;

    fn observation(
        source_classes: Vec<SourceClass>,
        sink_classes: Vec<SinkClass>,
    ) -> SafetyObservation {
        SafetyObservation {
            trace_id: "trace-multi-sink".to_string(),
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

    /// **多 sink 不得漏报**（旧实现只存首个 sink ⇒ 「读私有文件 → 出网」漏报）。
    #[test]
    fn a_data_node_with_two_sinks_still_detects_the_external_egress() {
        let mut chain = BehaviorChain::new("session-1", "trace-multi-sink");
        // ⚠️ 出网 sink **不是首个** ⇒ 旧实现只看 `first()` ⇒ 漏报。
        chain.add_action(
            &observation(
                vec![SourceClass::PrivateFile],
                vec![SinkClass::WorkspaceFile, SinkClass::ExternalNetwork],
            ),
            0,
        );

        // 数据节点**留全集**（两个 sink 都在）。
        let sinks: Vec<Vec<SinkClass>> = chain
            .nodes
            .iter()
            .filter_map(|node| match node {
                BehaviorNode::Data(data) => Some(data.sink_classes()),
                _ => None,
            })
            .collect();
        assert!(
            sinks.iter().any(|all| all.len() == 2),
            "Data 节点必须留全部 sink class: {sinks:?}"
        );
        assert!(
            chain.has_sensitive_source_to_external_sink(),
            "多 sink ⇒ 私有源 → 出网必须被发现（原先漏报）"
        );
    }

    /// 单 sink 的既有语义不变（回归钉住：写工作区文件不算出网）。
    #[test]
    fn a_single_sink_data_node_keeps_its_behavior() {
        let mut chain = BehaviorChain::new("session-1", "trace-single-sink");
        chain.add_action(
            &observation(
                vec![SourceClass::PrivateFile],
                vec![SinkClass::WorkspaceFile],
            ),
            0,
        );
        assert!(
            !chain.has_sensitive_source_to_external_sink(),
            "写工作区文件不算出网"
        );
    }
}
