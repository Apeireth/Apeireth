//! Trusted task-intent interpretation and action alignment.

use apeireth_governance::{
    CredentialPolicy, DestructivePolicy, IntentClass, IntentExplicitness, IntentProvenance,
    MutationPolicy, NetworkPolicy, OperationClass, PersistencePolicy, ShellPolicy,
    TaskIntentEnvelopeV1,
};

use crate::observation::{DataSensitivity, ResourceClass, SafetyObservation};

pub struct IntentInput {
    pub session_id: String,
    pub trace_id: String,
    pub user_request: String,
    pub created_at_ms: i64,
}

pub trait IntentInterpreter: Send + Sync {
    fn interpret(&self, input: IntentInput) -> TaskIntentEnvelopeV1;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationPolarity {
    Requested,
    Denied,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractedOperations {
    pub write: Option<OperationPolarity>,
    pub network: Option<OperationPolarity>,
    pub shell: Option<OperationPolarity>,
    pub publish: Option<OperationPolarity>,
    pub delete: Option<OperationPolarity>,
    pub credential: Option<OperationPolarity>,
    pub persistence: Option<OperationPolarity>,
    pub admin: Option<OperationPolarity>,
    pub read_only_explicit: bool,
    pub credential_disclosure_denied: bool,
}

impl ExtractedOperations {
    pub fn allows(&self, polarity: Option<OperationPolarity>) -> bool {
        matches!(polarity, Some(OperationPolarity::Requested))
    }

    pub fn denied(&self, polarity: Option<OperationPolarity>) -> bool {
        matches!(polarity, Some(OperationPolarity::Denied))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NegationAwareOperationExtractor;

impl NegationAwareOperationExtractor {
    pub fn extract(text: &str) -> ExtractedOperations {
        let lower = text.to_ascii_lowercase();
        let mut extracted = ExtractedOperations {
            read_only_explicit: contains_any(
                &lower,
                &[
                    "read-only",
                    "readonly",
                    "only inspect",
                    "只读",
                    "只检查",
                    "只分析",
                    "仅检查",
                    "仅分析",
                ],
            ),
            credential_disclosure_denied: contains_any(
                &lower,
                &[
                    "不要显示 token",
                    "不要显示token",
                    "不要读取 token 内容",
                    "不要显示令牌",
                    "don't show the token",
                    "do not show the token",
                    "don't display the token",
                    "without revealing the token",
                    "without reading its value",
                    "without reading the value",
                    "不读取其值",
                    "不要读取值",
                ],
            ),
            ..ExtractedOperations::default()
        };
        extracted.write = polarity(
            &lower,
            &[
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
                "提交",
                "commit",
            ],
            &[
                "不要修改",
                "不要写",
                "禁止修改",
                "不能修改",
                "别改",
                "无需修改",
                "不需要修改",
                "不允许修改",
                "不可修改",
                "without write",
                "without editing",
                "do not modify",
                "don't modify",
                "must not modify",
                "should not modify",
                "don't change",
                "do not change",
            ],
        );
        extracted.network = polarity(
            &lower,
            &[
                "network", "internet", "web", "http", "fetch", "download", "联网", "网络", "下载",
            ],
            &[
                "不要联网",
                "不联网",
                "禁止联网",
                "不能联网",
                "无需联网",
                "不需要联网",
                "不允许联网",
                "without network",
                "no network",
                "don't use the network",
                "do not use the network",
                "must not use the network",
            ],
        );
        extracted.shell = polarity(
            &lower,
            &[
                "shell", "command", "run", "execute", "test", "build", "执行", "运行", "测试",
                "构建",
            ],
            &[
                "不要执行",
                "不要运行",
                "不要运行命令",
                "禁止执行",
                "不能执行",
                "不允许执行",
                "without shell",
                "no shell",
                "don't run",
                "do not run",
                "don't execute",
                "do not execute",
                "must not run",
                "never run",
            ],
        );
        extracted.publish = polarity(
            &lower,
            &["push", "publish", "upload", "上传", "发布"],
            &[
                "不要 push",
                "不要push",
                "不要发布",
                "不要上传",
                "禁止发布",
                "不能发布",
                "don't push",
                "do not push",
                "don't publish",
                "do not publish",
                "must not push",
                "never push",
            ],
        );
        extracted.delete = polarity(
            &lower,
            &["delete", "remove", "reset", "删除", "移除", "清空"],
            &[
                "不要删除",
                "不要移除",
                "禁止删除",
                "不能删除",
                "不允许删除",
                "不可删除",
                "don't delete",
                "do not delete",
                "don't remove",
                "must not delete",
                "never delete",
            ],
        );
        extracted.credential = polarity(
            &lower,
            &[
                "credential",
                "secret",
                "token",
                "password",
                "api key",
                "凭证",
                "密钥",
                "令牌",
            ],
            &[
                "不要读 token",
                "不要读取 token",
                "不要读取token",
                "不要读令牌",
                "don't read the token",
                "do not read the token",
                "don't read token",
                "must not read the token",
            ],
        );
        extracted.persistence = polarity(
            &lower,
            &["install", "persist", "安装", "持久化"],
            &[
                "不要安装",
                "禁止安装",
                "不能安装",
                "无需安装",
                "don't install",
                "do not install",
                "must not install",
                "never install",
            ],
        );
        extracted.admin = polarity(
            &lower,
            &["admin", "chmod", "sudo", "管理员", "系统权限"],
            &[
                "不要改权限",
                "禁止管理员",
                "don't change permissions",
                "do not change permissions",
            ],
        );
        extracted
    }
}

fn polarity(text: &str, positive: &[&str], deny_phrases: &[&str]) -> Option<OperationPolarity> {
    // Evaluate every occurrence so a denied clause cannot hide a requested
    // operation that appears earlier or later in the same request.
    let denied = deny_phrases.iter().any(|phrase| text.contains(phrase))
        || positive.iter().any(|term| negated_match(text, term));
    let requested = positive.iter().any(|term| {
        has_unnegated_match(text, term)
            && !deny_phrases
                .iter()
                .any(|p| p.contains(term) && text.contains(p))
    });
    if denied {
        Some(OperationPolarity::Denied)
    } else if requested {
        Some(OperationPolarity::Requested)
    } else {
        None
    }
}

fn has_unnegated_match(text: &str, term: &str) -> bool {
    let mut offset = 0;
    while let Some(relative) = text[offset..].find(term) {
        let index = offset + relative;
        if !negated_at(text, index) {
            return true;
        }
        offset = index + term.len();
        if offset >= text.len() {
            break;
        }
    }
    false
}

fn negated_match(text: &str, term: &str) -> bool {
    let mut offset = 0;
    while let Some(relative) = text[offset..].find(term) {
        let index = offset + relative;
        if negated_at(text, index) {
            return true;
        }
        offset = index + term.len();
        if offset >= text.len() {
            break;
        }
    }
    false
}

fn negated_at(text: &str, index: usize) -> bool {
    let before = text[..index].trim_end_matches(|ch: char| {
        ch.is_whitespace() || matches!(ch, ',' | '.' | ';' | ':' | '，' | '。' | '、')
    });
    const MARKERS: &[&str] = &[
        "不要再",
        "不要",
        "不能",
        "别",
        "禁止",
        "无需",
        "不需要",
        "不允许",
        "不可",
        "don't",
        "do not",
        "without",
        "never",
        "must not",
        "should not",
        "no",
    ];
    MARKERS.iter().any(|marker| {
        if !before
            .to_ascii_lowercase()
            .ends_with(&marker.to_ascii_lowercase())
        {
            return false;
        }
        let prefix_len = before.len().saturating_sub(marker.len());
        prefix_len == 0 || {
            let boundary = before.is_char_boundary(prefix_len)
                && before[prefix_len..]
                    .to_ascii_lowercase()
                    .starts_with(&marker.to_ascii_lowercase());
            if !boundary {
                return false;
            }
            prefix_len == 0
                || before[..prefix_len].chars().next_back().is_none_or(|ch| {
                    ch.is_whitespace()
                        || !ch.is_ascii_alphabetic()
                        || !marker
                            .chars()
                            .next()
                            .is_some_and(|ch| ch.is_ascii_alphabetic())
                })
        }
    })
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

#[derive(Debug, Clone, Copy)]
pub struct IntentConfidencePolicy {
    pub explicit_min: f64,
    pub conservative_min: f64,
}

impl Default for IntentConfidencePolicy {
    fn default() -> Self {
        Self {
            explicit_min: 0.85,
            conservative_min: 0.50,
        }
    }
}

impl IntentConfidencePolicy {
    pub fn apply(&self, mut envelope: TaskIntentEnvelopeV1) -> TaskIntentEnvelopeV1 {
        if envelope.confidence >= self.explicit_min {
            envelope.explicitness = IntentExplicitness::Explicit;
            return envelope;
        }
        if envelope.confidence < self.conservative_min {
            return fail_narrow(envelope);
        }
        envelope.explicitness = IntentExplicitness::Inferred;
        if !envelope.allows_mutation() {
            envelope.mutation_policy = MutationPolicy::Deny;
        }
        if !envelope.allows_network() {
            envelope.network_policy = NetworkPolicy::Deny;
        }
        if !envelope.allows_shell() {
            envelope.shell_policy = ShellPolicy::Deny;
        }
        if !envelope.allows_publish() {
            envelope
                .allowed_effects
                .retain(|op| *op != OperationClass::Publish);
            envelope
                .requested_operations
                .retain(|op| *op != OperationClass::Publish);
        }
        if !envelope.allows_credentials() {
            envelope.credential_policy = CredentialPolicy::Deny;
        }
        envelope.destructive_policy = match envelope.destructive_policy {
            DestructivePolicy::Allow => DestructivePolicy::RequireApproval,
            other => other,
        };
        envelope.persistence_policy = PersistencePolicy::Deny;
        envelope
    }
}

fn fail_narrow(mut envelope: TaskIntentEnvelopeV1) -> TaskIntentEnvelopeV1 {
    envelope.intent_class = IntentClass::Unknown;
    envelope.explicitness = IntentExplicitness::Unknown;
    envelope.requested_operations = vec![OperationClass::Read];
    envelope.allowed_effects = vec![OperationClass::Read];
    envelope.allowed_scopes = vec!["workspace_read".to_string()];
    envelope.network_policy = NetworkPolicy::Deny;
    envelope.credential_policy = CredentialPolicy::Deny;
    envelope.shell_policy = ShellPolicy::Deny;
    envelope.mutation_policy = MutationPolicy::Deny;
    envelope.destructive_policy = DestructivePolicy::Deny;
    envelope.persistence_policy = PersistencePolicy::Deny;
    envelope.allowed_sinks.clear();
    envelope
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RuleIntentInterpreter;

impl IntentInterpreter for RuleIntentInterpreter {
    fn interpret(&self, input: IntentInput) -> TaskIntentEnvelopeV1 {
        let extracted = NegationAwareOperationExtractor::extract(&input.user_request);
        let write = extracted.allows(extracted.write);
        let network = extracted.allows(extracted.network);
        let shell = extracted.allows(extracted.shell);
        let publish = extracted.allows(extracted.publish);
        let destructive = extracted.allows(extracted.delete);
        let credential =
            extracted.allows(extracted.credential) && !extracted.denied(extracted.credential);
        let persistence = extracted.allows(extracted.persistence);
        let read_only = extracted.read_only_explicit
            || (contains_any(
                &input.user_request.to_ascii_lowercase(),
                &[
                    "inspect", "analyze", "explain", "review", "检查", "分析", "解释", "审查",
                    "看看", "look at",
                ],
            ) && !write
                && !publish
                && !destructive);

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
        } else if contains_any(
            &input.user_request.to_ascii_lowercase(),
            &["research", "查", "研究", "资料"],
        ) {
            (IntentClass::Research, 0.82)
        } else if shell {
            (IntentClass::CodeAnalysis, 0.86)
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
        envelope.credential_policy = if extracted.credential_disclosure_denied {
            CredentialPolicy::Deny
        } else if credential {
            CredentialPolicy::ReadOnly
        } else {
            CredentialPolicy::Deny
        };
        envelope.shell_policy = if shell && !extracted.denied(extracted.shell) {
            if write {
                ShellPolicy::Allow
            } else {
                ShellPolicy::ReadOnly
            }
        } else {
            ShellPolicy::Deny
        };
        envelope.mutation_policy = if write {
            MutationPolicy::WorkspaceOnly
        } else {
            MutationPolicy::Deny
        };
        envelope.destructive_policy = if destructive {
            DestructivePolicy::RequireApproval
        } else {
            DestructivePolicy::Deny
        };
        envelope.persistence_policy = if persistence {
            PersistencePolicy::RequireApproval
        } else {
            PersistencePolicy::Deny
        };
        envelope.provenance = IntentProvenance::UserExplicitRequest;
        envelope.created_at_ms = input.created_at_ms;
        if publish {
            envelope.allowed_sinks.push("repository_remote".to_string());
        }
        if extracted.credential_disclosure_denied {
            envelope
                .allowed_sinks
                .retain(|sink| sink != "user_display_secret");
            envelope
                .destination_constraints
                .push("no_secret_disclosure".to_string());
        }
        if matches!(intent_class, IntentClass::Unknown) {
            envelope = fail_narrow(envelope);
            envelope.confidence = confidence;
        }
        IntentConfidencePolicy::default().apply(envelope)
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

        let operations = obs.all_operations();
        if obs.external_effect
            && operations
                .iter()
                .all(|operation| matches!(*operation, OperationClass::Unknown))
        {
            return Self::mismatch(
                AlignmentClass::ScopeExpansion,
                0.75,
                "unknown_external_effect_capability",
            );
        }

        let protected = obs.resource_classes.iter().any(|class| {
            matches!(
                class,
                ResourceClass::GovernancePolicy
                    | ResourceClass::RuntimePolicy
                    | ResourceClass::AuditStore
                    | ResourceClass::DatasetStore
                    | ResourceClass::PluginConfiguration
                    | ResourceClass::SystemPersistence
            )
        }) || obs.capability_id.contains("guard.policy")
            || obs.capability_id.contains("governance.policy");
        if protected
            && operations.iter().any(|operation| {
                matches!(
                    *operation,
                    OperationClass::AdminChange
                        | OperationClass::PersistenceChange
                        | OperationClass::Write
                        | OperationClass::Modify
                        | OperationClass::Delete
                )
            })
        {
            return Self::mismatch(
                AlignmentClass::HighRiskMismatch,
                1.0,
                "protected_control_surface_tampering",
            );
        }
        if (matches!(
            obs.data_sensitivity,
            DataSensitivity::Credential | DataSensitivity::Secret
        ) || operations.iter().any(|operation| {
            matches!(
                *operation,
                OperationClass::CredentialRead | OperationClass::CredentialWrite
            )
        })) && !intent.allows_credentials()
        {
            return Self::mismatch(
                AlignmentClass::HighRiskMismatch,
                0.97,
                "unrequested_credential_access",
            );
        }
        if operations.iter().any(|operation| {
            matches!(
                *operation,
                OperationClass::NetworkSend | OperationClass::Publish
            )
        }) && !intent.allows_network()
            && !intent.allows_publish()
        {
            let reason = if operations.contains(&OperationClass::Publish) {
                "unrequested_publish"
            } else {
                "unrequested_external_egress"
            };
            return Self::mismatch(AlignmentClass::Contradictory, 0.92, reason);
        }
        if operations.contains(&OperationClass::Delete)
            && !matches!(intent.destructive_policy, DestructivePolicy::Allow)
        {
            return Self::mismatch(AlignmentClass::HighRiskMismatch, 0.95, "unrequested_delete");
        }
        if operations
            .iter()
            .any(|operation| matches!(*operation, OperationClass::Modify | OperationClass::Write))
            && !intent.allows_mutation()
        {
            return Self::mismatch(
                AlignmentClass::Contradictory,
                0.9,
                "read_only_task_requested_mutation",
            );
        }
        if operations.iter().any(|operation| {
            matches!(
                *operation,
                OperationClass::Execute | OperationClass::SpawnProcess
            )
        }) && !intent.allows_shell()
        {
            return Self::mismatch(
                AlignmentClass::ScopeExpansion,
                0.72,
                "unrequested_shell_execution",
            );
        }
        if operations
            .iter()
            .all(|operation| intent.allows_operation(*operation))
        {
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

    fn interpret(text: &str) -> TaskIntentEnvelopeV1 {
        RuleIntentInterpreter.interpret(IntentInput {
            session_id: "s".into(),
            trace_id: "t".into(),
            user_request: text.into(),
            created_at_ms: 1,
        })
    }

    #[test]
    fn interpreter_handles_chinese_read_only_request() {
        let intent = interpret("只检查仓库中的配置问题，不要修改，也不要联网");
        assert_eq!(intent.intent_class, IntentClass::ReadOnlyInspection);
        assert_eq!(intent.network_policy, NetworkPolicy::Deny);
        assert_eq!(intent.mutation_policy, MutationPolicy::Deny);
    }

    #[test]
    fn unknown_intent_fails_narrow() {
        let intent = interpret("嗯");
        assert_eq!(intent.intent_class, IntentClass::Unknown);
        assert_eq!(intent.network_policy, NetworkPolicy::Deny);
        assert_eq!(intent.credential_policy, CredentialPolicy::Deny);
        assert_eq!(intent.shell_policy, ShellPolicy::Deny);
        assert_eq!(intent.mutation_policy, MutationPolicy::Deny);
        assert_eq!(intent.destructive_policy, DestructivePolicy::Deny);
        assert_eq!(intent.persistence_policy, PersistencePolicy::Deny);
    }

    #[test]
    fn explicit_denial_dominates_write_token() {
        let intent = interpret("看看这个项目，不要修改");
        assert_eq!(intent.mutation_policy, MutationPolicy::Deny);
        assert_eq!(intent.intent_class, IntentClass::ReadOnlyInspection);
    }

    #[test]
    fn do_not_push_allows_local_commit() {
        let intent = interpret("不要 push，只提交本地修改");
        assert!(!intent.allows_publish());
        assert!(intent.allows_mutation());
    }

    #[test]
    fn clause_local_denial_does_not_disable_editing() {
        let intent = interpret("you may edit, but do not push");
        assert!(intent.allows_mutation());
        assert!(!intent.allows_publish());
    }

    #[test]
    fn clause_local_denial_does_not_disable_tests() {
        let intent = interpret("run tests, but do not install anything");
        assert!(intent.allows_shell());
        assert_eq!(intent.persistence_policy, PersistencePolicy::Deny);
    }

    #[test]
    fn credential_existence_check_denies_value_read() {
        let intent = interpret("check whether a credential exists without reading its value");
        assert!(!intent.allows_credentials());
    }
}
