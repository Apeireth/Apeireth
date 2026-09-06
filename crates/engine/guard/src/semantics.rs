//! Canonical capability safety semantics.
//!
//! Providers should eventually supply these descriptors with their capability
//! metadata. The local registry is the conservative fallback for legacy and
//! unknown tools; unknown external-effect tools are never treated as safe.

use std::collections::BTreeMap;

use apeireth_governance::OperationClass;

use crate::observation::{DataSensitivity, ResourceClass, SinkClass, SourceClass};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CapabilitySafetyDescriptor {
    pub capability_id: String,
    pub operation_classes: Vec<OperationClass>,
    pub resource_classes: Vec<ResourceClass>,
    pub input_sources: Vec<SourceClass>,
    pub output_sinks: Vec<SinkClass>,
    pub external_effect: bool,
    pub destructive: bool,
    pub persistent_effect: bool,
    pub requires_network: bool,
    pub may_access_credentials: bool,
    pub effect_scope: String,
    pub risk_tags: Vec<String>,
    pub data_sensitivity: DataSensitivity,
    pub known: bool,
}

impl CapabilitySafetyDescriptor {
    pub fn unknown(capability_id: impl Into<String>) -> Self {
        Self {
            capability_id: capability_id.into(),
            operation_classes: vec![OperationClass::Unknown],
            resource_classes: vec![ResourceClass::Unknown],
            input_sources: vec![SourceClass::Unknown],
            output_sinks: vec![SinkClass::Unknown],
            external_effect: true,
            destructive: false,
            persistent_effect: false,
            requires_network: false,
            may_access_credentials: false,
            effect_scope: "unknown".to_string(),
            risk_tags: vec!["unknown_capability".to_string()],
            data_sensitivity: DataSensitivity::Unknown,
            known: false,
        }
    }

    pub fn primary_operation(&self) -> OperationClass {
        self.operation_classes
            .first()
            .copied()
            .unwrap_or(OperationClass::Unknown)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CapabilitySafetyRegistry {
    descriptors: BTreeMap<String, CapabilitySafetyDescriptor>,
}

impl CapabilitySafetyRegistry {
    pub fn register(&mut self, descriptor: CapabilitySafetyDescriptor) {
        self.descriptors
            .insert(descriptor.capability_id.clone(), descriptor);
    }

    pub fn descriptor_for(
        &self,
        capability_id: &str,
        arguments: &serde_json::Value,
    ) -> CapabilitySafetyDescriptor {
        self.descriptors
            .get(capability_id)
            .cloned()
            .unwrap_or_else(|| infer_descriptor(capability_id, arguments))
    }
}

pub fn descriptor_for_capability(
    capability_id: &str,
    arguments: &serde_json::Value,
) -> CapabilitySafetyDescriptor {
    CapabilitySafetyRegistry::default().descriptor_for(capability_id, arguments)
}

pub fn effect_fingerprint(
    descriptor: &CapabilitySafetyDescriptor,
    arguments: &serde_json::Value,
) -> String {
    use std::hash::{Hash, Hasher};
    let target_class = arguments
        .get("path")
        .or_else(|| arguments.get("url"))
        .map(|value| {
            let text = value.as_str().unwrap_or_default();
            if text.contains(".env") || text.contains("secret") || text.contains("credential") {
                "sensitive_target"
            } else if text.starts_with("http") {
                "external_target"
            } else {
                "local_target"
            }
        })
        .unwrap_or("argument_shape");
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    descriptor.primary_operation().hash(&mut hasher);
    descriptor.resource_classes.hash(&mut hasher);
    descriptor.output_sinks.hash(&mut hasher);
    descriptor.destructive.hash(&mut hasher);
    target_class.hash(&mut hasher);
    format!("effect:{:016x}", hasher.finish())
}

fn infer_descriptor(
    capability_id: &str,
    arguments: &serde_json::Value,
) -> CapabilitySafetyDescriptor {
    let lower = capability_id.to_ascii_lowercase();
    let command = arguments
        .get("command")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let is_shell = lower.contains("shell") || lower.contains("bash") || lower.contains("exec");
    let is_network = lower.contains("fetch")
        || lower.contains("http")
        || lower.contains("network")
        || command.contains("curl")
        || command.contains("wget")
        || command.contains("ssh")
        || command.contains("scp");
    let is_publish =
        lower.contains("push") || lower.contains("publish") || lower.contains("upload");
    let is_credential = lower.contains("credential")
        || lower.contains("secret")
        || lower.contains("keyring")
        || command.contains(".env")
        || command.contains("id_rsa");
    let is_sensitive_probe = is_credential || lower.contains("env");
    let is_delete =
        lower.contains("delete") || lower.contains("remove") || lower.contains("unlink");
    let is_write = lower.contains("write")
        || lower.contains("edit")
        || lower.contains("modify")
        || lower.contains("update")
        || is_delete;
    let operation = if is_credential {
        OperationClass::CredentialRead
    } else if lower.contains("env") {
        OperationClass::Read
    } else if is_delete {
        OperationClass::Delete
    } else if is_publish {
        OperationClass::Publish
    } else if is_network {
        if lower.contains("post") || lower.contains("send") {
            OperationClass::NetworkSend
        } else {
            OperationClass::NetworkRead
        }
    } else if is_shell {
        OperationClass::Execute
    } else if is_write {
        OperationClass::Modify
    } else if lower.contains("search") || lower.contains("grep") {
        OperationClass::Search
    } else if lower.contains("list") || lower.contains("enum") {
        OperationClass::Enumerate
    } else if lower.contains("read") || lower.contains("file") || lower.contains("fs") {
        OperationClass::Read
    } else {
        OperationClass::Unknown
    };

    let mut descriptor = CapabilitySafetyDescriptor {
        capability_id: capability_id.to_string(),
        operation_classes: vec![operation],
        resource_classes: vec![ResourceClass::Unknown],
        input_sources: vec![SourceClass::Unknown],
        output_sinks: vec![SinkClass::Unknown],
        external_effect: is_shell
            || is_network
            || is_publish
            || is_write
            || operation == OperationClass::Unknown,
        destructive: is_delete,
        persistent_effect: is_write,
        requires_network: is_network,
        may_access_credentials: is_sensitive_probe,
        effect_scope: "unknown".to_string(),
        risk_tags: vec!["fallback_semantics".to_string()],
        data_sensitivity: if is_credential {
            DataSensitivity::Credential
        } else if is_sensitive_probe {
            DataSensitivity::Secret
        } else {
            DataSensitivity::Unknown
        },
        known: false,
    };

    if is_credential {
        descriptor.resource_classes = vec![ResourceClass::CredentialStore];
        descriptor.input_sources = vec![SourceClass::CredentialStore];
        descriptor.output_sinks = vec![SinkClass::UserDisplay];
        descriptor.effect_scope = "credential".to_string();
        descriptor.risk_tags.push("sensitive_source".to_string());
    } else if lower.contains("memory") {
        descriptor.resource_classes = vec![ResourceClass::MemoryEpisodic];
        descriptor.input_sources = vec![SourceClass::PrivateMemory];
        descriptor.output_sinks = vec![SinkClass::UserDisplay];
        descriptor.data_sensitivity = DataSensitivity::MemoryPrivate;
        descriptor.effect_scope = "private_memory".to_string();
    } else if lower.contains("env") {
        descriptor.resource_classes = vec![ResourceClass::EnvironmentVariables];
        descriptor.input_sources = vec![SourceClass::Environment];
        descriptor.output_sinks = vec![SinkClass::UserDisplay];
        descriptor.data_sensitivity = DataSensitivity::Secret;
        descriptor.effect_scope = "environment".to_string();
    } else if lower.contains("repo") || lower.contains("git") {
        descriptor.resource_classes = vec![ResourceClass::Repository];
        descriptor.input_sources = vec![SourceClass::WorkspaceFile];
        descriptor.output_sinks = vec![SinkClass::UserDisplay];
        descriptor.effect_scope = "repository".to_string();
    } else if is_network || is_publish {
        descriptor.resource_classes = vec![ResourceClass::NetworkPublic];
        descriptor.input_sources = vec![SourceClass::ToolOutput];
        descriptor.output_sinks = vec![if operation == OperationClass::NetworkRead {
            SinkClass::UserDisplay
        } else {
            SinkClass::ExternalNetwork
        }];
        descriptor.effect_scope = "external_network".to_string();
    } else if is_shell {
        descriptor.resource_classes = vec![ResourceClass::ProcessExecution];
        descriptor.input_sources = vec![SourceClass::UserPrompt];
        descriptor.output_sinks = vec![SinkClass::ShellExecution];
        descriptor.effect_scope = "process".to_string();
    } else if lower.contains("file") || lower.contains("fs") {
        descriptor.resource_classes = vec![ResourceClass::FilesystemWorkspace];
        descriptor.input_sources = if is_write {
            vec![SourceClass::UserPrompt]
        } else {
            vec![SourceClass::WorkspaceFile]
        };
        descriptor.output_sinks = vec![if is_write {
            SinkClass::WorkspaceFile
        } else {
            SinkClass::UserDisplay
        }];
        descriptor.effect_scope = "workspace".to_string();
    }

    descriptor
}
