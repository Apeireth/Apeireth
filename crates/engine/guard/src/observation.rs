//! Normalized Safety Observation.
//!
//! Provides a sanitized, structured observation of an Agent's planned or
//! executed action. Strictly prohibits raw credentials, secrets, private
//! memory bodies, or raw chain-of-thought.

use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, GovernanceRequest, OperationClass};
use serde::{Deserialize, Serialize};

/// Classification of resources touched by an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceClass {
    FilesystemWorkspace,
    FilesystemSystem,
    ProcessExecution,
    NetworkPublic,
    NetworkPrivate,
    CredentialStore,
    MemoryEpisodic,
    MemorySemantic,
    EnvironmentVariables,
    Repository,
    RepositoryRemote,
    UserPrivateData,
    RuntimePolicy,
    GovernancePolicy,
    AuditStore,
    DatasetStore,
    PluginConfiguration,
    SystemPersistence,
    Unknown,
}

/// Sensitivity label propagated through the bounded trace-local taint state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSensitivity {
    Public,
    UserProvided,
    WorkspacePrivate,
    UserPrivate,
    MemoryPrivate,
    Secret,
    Credential,
    SystemSensitive,
    Unknown,
}

impl Default for DataSensitivity {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Source classification for data flow analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceClass {
    UserPrompt,
    WorkspaceFile,
    PrivateFile,
    Environment,
    CredentialStore,
    PrivateMemory,
    ToolOutput,
    ExternalNetwork,
    Unknown,
}

/// Sink classification for data egress analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkClass {
    WorkspaceFile,
    SystemFile,
    ExternalNetwork,
    InternalNetwork,
    ShellExecution,
    MemoryWrite,
    ModelCompletion,
    UserDisplay,
    Unknown,
}

/// Normalized observation of an agent action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyObservation {
    pub trace_id: String,
    pub request_id: String,
    pub session_id: String,
    pub stage: String,
    pub capability_id: String,
    pub tool_name: String,
    pub resource_classes: Vec<ResourceClass>,
    pub source_classes: Vec<SourceClass>,
    pub sink_classes: Vec<SinkClass>,
    pub permission_scope: String,
    pub approval_state: String,
    pub argument_shape: String,
    pub redacted_argument_features: serde_json::Value,
    pub result_class: Option<String>,
    pub result_size_bucket: Option<String>,
    pub retry_count: u32,
    pub denied_before: bool,
    pub prior_actions: Vec<String>,
    pub external_effect: bool,
    #[serde(default)]
    pub operation_class: OperationClass,
    #[serde(default)]
    pub data_sensitivity: DataSensitivity,
    #[serde(default)]
    pub persistent_effect: bool,
    #[serde(default)]
    pub destructive_effect: bool,
    #[serde(default)]
    pub requires_network: bool,
    #[serde(default)]
    pub may_access_credentials: bool,
    #[serde(default)]
    pub effect_fingerprint: String,
}

impl SafetyObservation {
    /// Create a normalized, desensitized safety observation from a governance request.
    pub fn from_governance_request(
        req: &GovernanceRequest<'_>,
        retry_count: u32,
        denied_before: bool,
        prior_actions: Vec<String>,
    ) -> Self {
        let trace_id = req.trace.to_string();
        let session_id = req.session.to_string();
        let request_id = format!("{}:{}:{}", session_id, trace_id, req.round);

        match &req.action {
            Action::Completion {
                model,
                message_count,
            } => Self {
                trace_id,
                request_id,
                session_id,
                stage: "completion".to_string(),
                capability_id: "llm.completion".to_string(),
                tool_name: "completion".to_string(),
                resource_classes: vec![ResourceClass::NetworkPublic],
                source_classes: vec![SourceClass::UserPrompt],
                sink_classes: vec![SinkClass::ModelCompletion],
                permission_scope: "model_inference".to_string(),
                approval_state: "not_required".to_string(),
                argument_shape: format!("model={},messages={}", model, message_count),
                redacted_argument_features: serde_json::json!({
                    "model_family": extract_model_family(model),
                    "message_count": message_count,
                }),
                result_class: None,
                result_size_bucket: None,
                retry_count,
                denied_before,
                prior_actions,
                external_effect: false,
                operation_class: OperationClass::Read,
                data_sensitivity: DataSensitivity::Public,
                persistent_effect: false,
                destructive_effect: false,
                requires_network: false,
                may_access_credentials: false,
                effect_fingerprint: "completion".to_string(),
            },
            Action::CapabilityDispatch {
                capability,
                arguments,
            } => {
                let cap_str = capability.as_str();
                let descriptor = crate::semantics::descriptor_for_capability(cap_str, arguments);
                let (res_classes, src_classes, sink_classes, ext_effect) = (
                    descriptor.resource_classes.clone(),
                    descriptor.input_sources.clone(),
                    descriptor.output_sinks.clone(),
                    descriptor.external_effect,
                );
                let redacted_features = redact_and_extract_features(arguments);

                Self {
                    trace_id,
                    request_id,
                    session_id,
                    stage: "capability_dispatch".to_string(),
                    capability_id: cap_str.to_string(),
                    tool_name: cap_str.to_string(),
                    resource_classes: res_classes,
                    source_classes: src_classes,
                    sink_classes,
                    permission_scope: cap_str.to_string(),
                    approval_state: "pending_guard".to_string(),
                    argument_shape: summarize_argument_shape(arguments),
                    redacted_argument_features: redacted_features,
                    result_class: None,
                    result_size_bucket: None,
                    retry_count,
                    denied_before,
                    prior_actions,
                    external_effect: ext_effect,
                    operation_class: descriptor.primary_operation(),
                    data_sensitivity: descriptor.data_sensitivity,
                    persistent_effect: descriptor.persistent_effect,
                    destructive_effect: descriptor.destructive,
                    requires_network: descriptor.requires_network,
                    may_access_credentials: descriptor.may_access_credentials,
                    effect_fingerprint: crate::semantics::effect_fingerprint(
                        &descriptor,
                        arguments,
                    ),
                }
            }
            _ => Self {
                trace_id,
                request_id,
                session_id,
                stage: "unknown".to_string(),
                capability_id: "unknown".to_string(),
                tool_name: "unknown".to_string(),
                resource_classes: vec![ResourceClass::Unknown],
                source_classes: vec![SourceClass::Unknown],
                sink_classes: vec![SinkClass::Unknown],
                permission_scope: "unknown".to_string(),
                approval_state: "not_required".to_string(),
                argument_shape: "unknown".to_string(),
                redacted_argument_features: serde_json::Value::Null,
                result_class: None,
                result_size_bucket: None,
                retry_count,
                denied_before,
                prior_actions,
                external_effect: false,
                operation_class: OperationClass::Unknown,
                data_sensitivity: DataSensitivity::Unknown,
                persistent_effect: false,
                destructive_effect: false,
                requires_network: false,
                may_access_credentials: false,
                effect_fingerprint: "unknown".to_string(),
            },
        }
    }
}

fn extract_model_family(model: &str) -> String {
    let lower = model.to_lowercase();
    if lower.contains("gpt") {
        "openai_gpt".to_string()
    } else if lower.contains("claude") {
        "anthropic_claude".to_string()
    } else if lower.contains("minimax") || lower.contains("abab") {
        "minimax".to_string()
    } else if lower.contains("deepseek") {
        "deepseek".to_string()
    } else {
        "custom_llm".to_string()
    }
}

fn summarize_argument_shape(args: &serde_json::Value) -> String {
    match args {
        serde_json::Value::Object(map) => {
            let keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
            format!("object(keys=[{}])", keys.join(","))
        }
        serde_json::Value::Array(arr) => format!("array(len={})", arr.len()),
        serde_json::Value::String(s) => format!("string(len={})", s.len()),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::Bool(_) => "bool".to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

/// Redact argument features to extract structural properties without retaining
/// sensitive tokens, raw CoT, passwords, or credentials.
fn redact_and_extract_features(args: &serde_json::Value) -> serde_json::Value {
    let mut features = serde_json::Map::new();

    if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
        features.insert("has_command".to_string(), serde_json::Value::Bool(true));
        features.insert(
            "command_head".to_string(),
            serde_json::Value::String(cmd.split_whitespace().next().unwrap_or("").to_string()),
        );
        features.insert("command_length".to_string(), serde_json::json!(cmd.len()));
        features.insert(
            "has_pipeline".to_string(),
            serde_json::Value::Bool(cmd.contains('|') || cmd.contains(';')),
        );
        features.insert(
            "has_redirect".to_string(),
            serde_json::Value::Bool(cmd.contains('>') || cmd.contains('<')),
        );
        let tokens: Vec<&str> = cmd.split_whitespace().collect();
        let command_head = tokens.first().copied().unwrap_or_default();
        let command_family = if ["curl", "wget", "fetch", "http", "ssh", "scp"]
            .iter()
            .any(|value| command_head.eq_ignore_ascii_case(value))
        {
            "network_utility"
        } else if ["rm", "del", "rmdir", "mkfs", "format", "dd"]
            .iter()
            .any(|value| command_head.eq_ignore_ascii_case(value))
        {
            "destructive_utility"
        } else if ["cargo", "npm", "pnpm", "git", "rustc"]
            .iter()
            .any(|value| command_head.eq_ignore_ascii_case(value))
        {
            "development_utility"
        } else {
            "other"
        };
        features.insert(
            "command_family".to_string(),
            serde_json::json!(command_family),
        );
        features.insert(
            "argument_count".to_string(),
            serde_json::json!(tokens.len().saturating_sub(1)),
        );
        features.insert(
            "pipeline_count".to_string(),
            serde_json::json!(cmd.matches('|').count()),
        );
        features.insert(
            "redirection_count".to_string(),
            serde_json::json!(cmd.matches('>').count() + cmd.matches('<').count()),
        );
        features.insert(
            "subshell_present".to_string(),
            serde_json::json!(cmd.contains("$(") || cmd.contains('`')),
        );
        features.insert(
            "background_execution".to_string(),
            serde_json::json!(cmd.trim_end().ends_with('&')),
        );
        features.insert(
            "interpreter_nesting".to_string(),
            serde_json::json!(cmd.matches("bash -c").count() + cmd.matches("sh -c").count()),
        );
        features.insert(
            "network_utility".to_string(),
            serde_json::json!(command_family == "network_utility"),
        );
        features.insert(
            "privilege_modifier".to_string(),
            serde_json::json!(cmd.contains("sudo ") || cmd.contains("runas ")),
        );
        features.insert(
            "recursive_operation".to_string(),
            serde_json::json!(cmd.contains(" -r") || cmd.contains(" --recursive")),
        );
        features.insert(
            "destructive_flag".to_string(),
            serde_json::json!(
                cmd.contains("rm -rf") || cmd.contains(" --delete") || cmd.contains(" --force")
            ),
        );
        features.insert(
            "persistence_indicator".to_string(),
            serde_json::json!(
                cmd.contains("cron") || cmd.contains("startup") || cmd.contains("systemd")
            ),
        );
        features.insert(
            "download_execute_pattern".to_string(),
            serde_json::json!(
                (cmd.contains("curl") || cmd.contains("wget"))
                    && (cmd.contains("| sh") || cmd.contains("| bash"))
            ),
        );
        features.insert(
            "encoded_payload_indicator".to_string(),
            serde_json::json!(
                cmd.contains("base64") || cmd.contains("\u{25}3c") || cmd.contains("\\x")
            ),
        );
        features.insert(
            "credential_path_indicator".to_string(),
            serde_json::json!(
                cmd.contains(".env")
                    || cmd.contains("id_rsa")
                    || cmd.contains("credential")
                    || cmd.contains("token")
            ),
        );
        features.insert(
            "system_path_indicator".to_string(),
            serde_json::json!(
                cmd.contains("/etc/") || cmd.contains("/dev/") || cmd.contains("C:\\Windows")
            ),
        );
    }

    if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
        features.insert("has_path".to_string(), serde_json::Value::Bool(true));
        features.insert(
            "is_absolute_path".to_string(),
            serde_json::Value::Bool(
                path.starts_with('/') || (path.len() >= 2 && path.chars().nth(1) == Some(':')),
            ),
        );
        features.insert(
            "has_parent_traversal".to_string(),
            serde_json::Value::Bool(path.contains("..")),
        );
        features.insert(
            "is_dotfile".to_string(),
            serde_json::Value::Bool(path.contains("/.") || path.starts_with('.')),
        );
    }

    if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
        features.insert("has_url".to_string(), serde_json::Value::Bool(true));
        let is_https = url.starts_with("https://");
        let is_http = url.starts_with("http://");
        features.insert("is_https".to_string(), serde_json::Value::Bool(is_https));
        features.insert("is_http".to_string(), serde_json::Value::Bool(is_http));
        features.insert(
            "is_loopback".to_string(),
            serde_json::Value::Bool(
                url.contains("localhost") || url.contains("127.0.0.1") || url.contains("::1"),
            ),
        );
    }

    serde_json::Value::Object(features)
}
