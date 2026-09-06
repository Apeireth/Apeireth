//! Trusted task-intent interpretation and action alignment.

use apeireth_governance::{
    CredentialPolicy, DestructivePolicy, IntentClass, IntentExplicitness, IntentProvenance,
    MutationPolicy, NetworkPolicy, OperationClass, PersistencePolicy, ShellPolicy,
    TaskIntentEnvelopeV1,
};

use crate::observation::{DataSensitivity, SafetyObservation};

pub struct IntentInput {
    pub session_id: String,
    pub trace_id: String,
    pub user_request: String,
    pub created_at_ms: i64,
}

pub trait IntentInterpreter: Send + Sync {
    fn interpret(&self, input: IntentInput) -> TaskIntentEnvelopeV1;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RuleIntentInterpreter;

impl IntentInterpreter for RuleIntentInterpreter {
    fn interpret(&self, input: IntentInput) -> TaskIntentEnvelopeV1 {
        let lower = input.user_request.to_ascii_lowercase();
        let has_any = |terms: &[&str]| terms.iter().any(|term| lower.contains(term));
        let publish = has_any(&["push", "publish", "上传", "发布"]);
        let destructive = has_any(&["delete", "remove", "reset", "删除", "移除", "清空"]);
        let credential = has_any(&[
            "credential",
            "secret",
            "token",
            "password",
            "api key",
            "凭证",
            "密钥",
            "令牌",
        ]);
        let network = has_any(&[
            "network", "internet", "web", "http", "fetch", "download", "联网", "网络", "下载",
        ]) && !has_any(&[
            "不要联网",
            "不联网",
            "禁止联网",
            "without network",
            "no network",
        ]);
        let write = has_any(&[
            "edit",
            "modify",
            "change",
            "write",
            "patch",
            "implement",
            "修改",
            "编辑",
            "写入",
            "实现",
            "改动",
        ]) && !has_any(&[
            "不要修改",
            "不要写",
            "禁止修改",
            "without write",
            "without editing",
        ]);
        let shell = has_any(&[
            "shell", "command", "run", "test", "build", "执行", "运行", "测试", "构建",
        ]) && !has_any(&[
            "不要执行",
            "不要运行",
            "禁止执行",
            "without shell",
            "no shell",
        ]);
        let read_only = has_any(&[
            "read-only",
            "readonly",
            "inspect",
            "analyze",
            "explain",
            "review",
            "只读",
            "检查",
            "分析",
            "解释",
            "审查",
            "看看",
        ]) && !write
            && !publish
            && !destructive;

        let (intent_class, confidence) = if publish {
            (IntentClass::RepositoryPublish, 0.98)
        } else if destructive {
            (IntentClass::ExplicitDestructiveMaintenance, 0.98)
        } else if credential {
            (IntentClass::CredentialOperation, 0.93)
        } else if write {
            (IntentClass::CodeModification, 0.92)
        } else if network {
            (IntentClass::NetworkResearch, 0.9)
        } else if read_only {
            (IntentClass::ReadOnlyInspection, 0.92)
        } else if has_any(&["research", "查", "研究", "资料"]) {
            (IntentClass::Research, 0.82)
        } else {
            (IntentClass::Unknown, 0.2)
        };

        let mut requested_operations = vec![OperationClass::Read];
        if write {
            requested_operations.push(OperationClass::Modify);
        }
        if shell {
            requested_operations.push(OperationClass::Execute);
        }
        if network {
            requested_operations.push(OperationClass::NetworkRead);
        }
        if publish {
            requested_operations.push(OperationClass::Publish);
        }
        if destructive {
            requested_operations.push(OperationClass::Delete);
        }
        if credential {
            requested_operations.push(OperationClass::CredentialRead);
        }

        let mut envelope = TaskIntentEnvelopeV1::unknown(input.session_id, input.trace_id);
        envelope.intent_class = intent_class;
        envelope.explicitness = if confidence > 0.8 {
            IntentExplicitness::Explicit
        } else {
            IntentExplicitness::Inferred
        };
        envelope.confidence = confidence;
        envelope.requested_operations = requested_operations.clone();
        envelope.allowed_effects = requested_operations;
        envelope.allowed_scopes = if read_only {
            vec!["workspace_read".to_string(), "repository_read".to_string()]
        } else {
            vec!["task_declared".to_string()]
        };
        envelope.network_policy = if network {
            NetworkPolicy::PublicRead
        } else {
            NetworkPolicy::Deny
        };
        envelope.credential_policy = if credential {
            CredentialPolicy::ReadOnly
        } else {
            CredentialPolicy::Deny
        };
        envelope.shell_policy = if shell {
            ShellPolicy::ReadOnly
        } else {
            ShellPolicy::Deny
        };
        envelope.mutation_policy = if read_only {
            MutationPolicy::Deny
        } else {
            MutationPolicy::WorkspaceOnly
        };
        envelope.destructive_policy = if destructive {
            DestructivePolicy::RequireApproval
        } else {
            DestructivePolicy::Deny
        };
        envelope.persistence_policy = PersistencePolicy::Deny;
        envelope.provenance = IntentProvenance::UserExplicitRequest;
        envelope.created_at_ms = input.created_at_ms;
        if publish {
            envelope.allowed_sinks.push("repository_remote".to_string());
        }
        envelope
    }
}

/// A model proposal can only narrow a trusted envelope. It can never add a
/// capability, sink, or effect that the user did not explicitly request.
pub fn constrain_to_trusted(
    trusted: &TaskIntentEnvelopeV1,
    proposed: &TaskIntentEnvelopeV1,
) -> TaskIntentEnvelopeV1 {
    let mut result = trusted.clone();
    result.confidence = trusted.confidence.min(proposed.confidence);
    result.requested_operations = trusted
        .requested_operations
        .iter()
        .copied()
        .filter(|operation| proposed.requested_operations.contains(operation))
        .collect();
    result.allowed_effects = trusted
        .allowed_effects
        .iter()
        .copied()
        .filter(|operation| proposed.allowed_effects.contains(operation))
        .collect();
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentClass {
    Aligned,
    WeaklyAligned,
    UnexpectedButBenign,
    ScopeExpansion,
    Contradictory,
    HighRiskMismatch,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AlignmentAssessment {
    pub class: AlignmentClass,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct IntentAlignmentGuard;

impl IntentAlignmentGuard {
    pub fn evaluate(
        &self,
        intent: Option<&TaskIntentEnvelopeV1>,
        obs: &SafetyObservation,
    ) -> AlignmentAssessment {
        let Some(intent) = intent else {
            return AlignmentAssessment {
                class: AlignmentClass::Unknown,
                score: 0.45,
                reasons: vec!["turn_intent_unavailable".to_string()],
            };
        };

        if obs.external_effect && matches!(obs.operation_class, OperationClass::Unknown) {
            return Self::mismatch(
                AlignmentClass::ScopeExpansion,
                0.75,
                "unknown_external_effect_capability",
            );
        }

        if obs.capability_id.contains("guard.policy")
            || obs.capability_id.contains("governance.policy")
            || matches!(
                obs.operation_class,
                OperationClass::AdminChange | OperationClass::PersistenceChange
            )
        {
            return Self::mismatch(
                AlignmentClass::HighRiskMismatch,
                1.0,
                "protected_control_surface_tampering",
            );
        }
        if matches!(
            obs.data_sensitivity,
            DataSensitivity::Credential | DataSensitivity::Secret
        ) && !intent.allows_credentials()
        {
            return Self::mismatch(
                AlignmentClass::HighRiskMismatch,
                0.97,
                "unrequested_credential_access",
            );
        }
        if matches!(
            obs.operation_class,
            OperationClass::NetworkSend | OperationClass::Publish
        ) && !intent.allows_network()
            && !intent.allows_publish()
        {
            return Self::mismatch(
                AlignmentClass::Contradictory,
                0.92,
                "unrequested_external_egress",
            );
        }
        if matches!(obs.operation_class, OperationClass::Delete)
            && !matches!(intent.destructive_policy, DestructivePolicy::Allow)
        {
            return Self::mismatch(AlignmentClass::HighRiskMismatch, 0.95, "unrequested_delete");
        }
        if matches!(
            obs.operation_class,
            OperationClass::Modify | OperationClass::Write
        ) && !intent.allows_mutation()
        {
            return Self::mismatch(
                AlignmentClass::Contradictory,
                0.9,
                "read_only_task_requested_mutation",
            );
        }
        if matches!(
            obs.operation_class,
            OperationClass::Execute | OperationClass::SpawnProcess
        ) && !intent.allows_shell()
        {
            return Self::mismatch(
                AlignmentClass::ScopeExpansion,
                0.72,
                "unrequested_shell_execution",
            );
        }
        if intent.allows_operation(obs.operation_class) {
            AlignmentAssessment {
                class: AlignmentClass::Aligned,
                score: 0.0,
                reasons: Vec::new(),
            }
        } else {
            Self::mismatch(
                AlignmentClass::UnexpectedButBenign,
                0.5,
                "unexpected_operation",
            )
        }
    }

    fn mismatch(class: AlignmentClass, score: f64, reason: &str) -> AlignmentAssessment {
        AlignmentAssessment {
            class,
            score,
            reasons: vec![reason.to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpreter_handles_chinese_read_only_request() {
        let intent = RuleIntentInterpreter.interpret(IntentInput {
            session_id: "s".into(),
            trace_id: "t".into(),
            user_request: "只检查仓库中的配置问题，不要修改，也不要联网".into(),
            created_at_ms: 1,
        });
        assert_eq!(intent.intent_class, IntentClass::ReadOnlyInspection);
        assert_eq!(intent.network_policy, NetworkPolicy::Deny);
        assert_eq!(intent.mutation_policy, MutationPolicy::Deny);
    }
}
