//! Canonical capability safety semantics.
//!
//! Production truth comes from capability metadata when present. The local
//! registry is the conservative fallback for legacy and unknown tools; unknown
//! external-effect tools are never treated as safe.

use std::collections::BTreeMap;

use apeireth_governance::OperationClass;

use crate::command_effect::CommandEffectAnalyzer;
use crate::observation::{DataSensitivity, ResourceClass, SinkClass, SourceClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DescriptorSource {
    Canonical,
    PluginDeclared,
    AdapterInferred,
    FallbackHeuristic,
    #[default]
    Unknown,
}

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
    #[serde(default)]
    pub source: DescriptorSource,
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
            source: DescriptorSource::Unknown,
        }
    }

    pub fn primary_operation(&self) -> OperationClass {
        self.operation_classes
            .iter()
            .copied()
            .find(|operation| {
                !matches!(
                    *operation,
                    OperationClass::Execute | OperationClass::Unknown
                )
            })
            .or_else(|| self.operation_classes.first().copied())
            .unwrap_or(OperationClass::Unknown)
    }
}

pub trait CapabilitySafetyMetadataProvider: Send + Sync {
    fn safety_descriptor(&self, capability_id: &str) -> Option<CapabilitySafetyDescriptor>;
}

#[derive(Debug, Clone, Default)]
pub struct CapabilitySafetyRegistry {
    descriptors: BTreeMap<String, CapabilitySafetyDescriptor>,
}

impl CapabilitySafetyRegistry {
    pub fn canonical() -> Self {
        let mut registry = Self::default();
        for descriptor in canonical_descriptors() {
            registry.register(descriptor);
        }
        registry
    }

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
            .map(|mut descriptor| {
                merge_command_effects(&mut descriptor, arguments);
                descriptor
            })
            .unwrap_or_else(|| infer_descriptor(capability_id, arguments))
    }
}

impl CapabilitySafetyMetadataProvider for CapabilitySafetyRegistry {
    fn safety_descriptor(&self, capability_id: &str) -> Option<CapabilitySafetyDescriptor> {
        self.descriptors.get(capability_id).cloned()
    }
}

pub fn descriptor_for_capability(
    capability_id: &str,
    arguments: &serde_json::Value,
) -> CapabilitySafetyDescriptor {
    CapabilitySafetyRegistry::canonical().descriptor_for(capability_id, arguments)
}

pub fn resolve_descriptor(
    capability_id: &str,
    arguments: &serde_json::Value,
    provider: Option<&dyn CapabilitySafetyMetadataProvider>,
) -> CapabilitySafetyDescriptor {
    if let Some(mut descriptor) = provider.and_then(|item| item.safety_descriptor(capability_id)) {
        merge_command_effects(&mut descriptor, arguments);
        return descriptor;
    }
    CapabilitySafetyRegistry::canonical().descriptor_for(capability_id, arguments)
}

pub fn effect_fingerprint(
    descriptor: &CapabilitySafetyDescriptor,
    _arguments: &serde_json::Value,
) -> String {
    use std::hash::{Hash, Hasher};
    let destination = descriptor
        .output_sinks
        .iter()
        .copied()
        .find(|sink| {
            matches!(
                *sink,
                SinkClass::ExternalNetwork | SinkClass::WorkspaceFile | SinkClass::SystemFile
            )
        })
        .or_else(|| descriptor.output_sinks.first().copied())
        .unwrap_or(SinkClass::Unknown);
    let target_class = if descriptor.may_access_credentials
        || descriptor
            .resource_classes
            .contains(&ResourceClass::CredentialStore)
    {
        "sensitive_target"
    } else if descriptor
        .resource_classes
        .contains(&ResourceClass::FilesystemSystem)
        || descriptor
            .resource_classes
            .contains(&ResourceClass::SystemPersistence)
    {
        "system_target"
    } else if descriptor.requires_network
        || descriptor
            .output_sinks
            .contains(&SinkClass::ExternalNetwork)
        || descriptor
            .resource_classes
            .contains(&ResourceClass::RepositoryRemote)
    {
        "external_target"
    } else if descriptor
        .resource_classes
        .contains(&ResourceClass::FilesystemWorkspace)
        || descriptor
            .resource_classes
            .contains(&ResourceClass::Repository)
    {
        "local_target"
    } else {
        "argument_shape"
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut ops: Vec<_> = descriptor
        .operation_classes
        .iter()
        .copied()
        .filter(|operation| {
            !matches!(
                *operation,
                OperationClass::Execute | OperationClass::SpawnProcess | OperationClass::Unknown
            )
        })
        .collect();
    if ops.is_empty() {
        ops.push(descriptor.primary_operation());
    }
    ops.sort_by_key(|operation| format!("{operation:?}"));
    ops.dedup();
    ops.hash(&mut hasher);
    destination.hash(&mut hasher);
    descriptor.destructive.hash(&mut hasher);
    descriptor.persistent_effect.hash(&mut hasher);
    target_class.hash(&mut hasher);
    format!("effect:{:016x}", hasher.finish())
}

fn merge_command_effects(
    descriptor: &mut CapabilitySafetyDescriptor,
    arguments: &serde_json::Value,
) {
    let Some(command) = arguments.get("command").and_then(serde_json::Value::as_str) else {
        return;
    };
    let summary = CommandEffectAnalyzer::analyze(command);
    for operation in summary.operation_classes {
        if !descriptor.operation_classes.contains(&operation) {
            descriptor.operation_classes.push(operation);
        }
    }
    for resource in summary.resource_classes {
        if !descriptor.resource_classes.contains(&resource) {
            descriptor.resource_classes.push(resource);
        }
    }
    for sink in summary.sink_classes {
        if !descriptor.output_sinks.contains(&sink) {
            descriptor.output_sinks.push(sink);
        }
    }
    descriptor.destructive |= summary.destructive;
    descriptor.persistent_effect |=
        summary.persistence_change || summary.filesystem_write || summary.repository_publish;
    descriptor.requires_network |= summary.network_read || summary.network_send;
    descriptor.may_access_credentials |= summary.credential_probe;
    descriptor.external_effect |= summary.network_send || summary.repository_publish;
    if summary.destructive {
        descriptor.data_sensitivity = match descriptor.data_sensitivity {
            DataSensitivity::Public => DataSensitivity::SystemSensitive,
            other => other,
        };
    }
}

fn canonical_descriptors() -> Vec<CapabilitySafetyDescriptor> {
    vec![
        describe(
            "fs.read",
            vec![OperationClass::Read],
            vec![ResourceClass::FilesystemWorkspace],
            vec![SourceClass::WorkspaceFile],
            vec![SinkClass::UserDisplay],
            false,
            false,
            false,
            false,
            "workspace",
        ),
        describe(
            "fs.write",
            vec![OperationClass::Write, OperationClass::Modify],
            vec![ResourceClass::FilesystemWorkspace],
            vec![SourceClass::UserPrompt],
            vec![SinkClass::WorkspaceFile],
            false,
            false,
            true,
            false,
            "workspace",
        ),
        describe(
            "fs.delete",
            vec![OperationClass::Delete],
            vec![ResourceClass::FilesystemWorkspace],
            vec![SourceClass::UserPrompt],
            vec![SinkClass::WorkspaceFile],
            false,
            true,
            true,
            false,
            "workspace",
        ),
        describe(
            "shell.exec",
            vec![OperationClass::Execute],
            vec![ResourceClass::ProcessExecution],
            vec![SourceClass::UserPrompt],
            vec![SinkClass::ShellExecution],
            true,
            false,
            false,
            false,
            "process",
        ),
        describe(
            "tool.shell",
            vec![OperationClass::Execute],
            vec![ResourceClass::ProcessExecution],
            vec![SourceClass::UserPrompt],
            vec![SinkClass::ShellExecution],
            true,
            false,
            false,
            false,
            "process",
        ),
        describe(
            "http.get",
            vec![OperationClass::NetworkRead],
            vec![ResourceClass::NetworkPublic],
            vec![SourceClass::ExternalNetwork],
            vec![SinkClass::UserDisplay],
            true,
            false,
            false,
            true,
            "external_network",
        ),
        describe(
            "http.send",
            vec![OperationClass::NetworkSend],
            vec![ResourceClass::NetworkPublic],
            vec![SourceClass::ToolOutput],
            vec![SinkClass::ExternalNetwork],
            true,
            false,
            false,
            true,
            "external_network",
        ),
        describe(
            "tool.fetch",
            vec![OperationClass::NetworkRead],
            vec![ResourceClass::NetworkPublic],
            vec![SourceClass::ExternalNetwork],
            vec![SinkClass::UserDisplay],
            true,
            false,
            false,
            true,
            "external_network",
        ),
        describe(
            "repo.publish",
            vec![OperationClass::Publish, OperationClass::NetworkSend],
            vec![ResourceClass::RepositoryRemote],
            vec![SourceClass::WorkspaceFile],
            vec![SinkClass::ExternalNetwork],
            true,
            false,
            true,
            true,
            "repository_remote",
        ),
        describe(
            "credential.read",
            vec![OperationClass::CredentialRead],
            vec![ResourceClass::CredentialStore],
            vec![SourceClass::CredentialStore],
            vec![SinkClass::UserDisplay],
            false,
            false,
            false,
            false,
            "credential",
        ),
        describe(
            "env.read",
            vec![OperationClass::Read],
            vec![ResourceClass::EnvironmentVariables],
            vec![SourceClass::Environment],
            vec![SinkClass::UserDisplay],
            false,
            false,
            false,
            false,
            "environment",
        ),
        describe(
            "secret.read",
            vec![OperationClass::CredentialRead],
            vec![ResourceClass::CredentialStore],
            vec![SourceClass::CredentialStore],
            vec![SinkClass::UserDisplay],
            false,
            false,
            false,
            false,
            "credential",
        ),
        describe(
            "guard.policy.write",
            vec![
                OperationClass::AdminChange,
                OperationClass::PersistenceChange,
            ],
            vec![ResourceClass::GovernancePolicy],
            vec![SourceClass::UserPrompt],
            vec![SinkClass::SystemFile],
            false,
            false,
            true,
            false,
            "governance",
        ),
    ]
}

fn describe(
    id: &str,
    operations: Vec<OperationClass>,
    resources: Vec<ResourceClass>,
    sources: Vec<SourceClass>,
    sinks: Vec<SinkClass>,
    external: bool,
    destructive: bool,
    persistent: bool,
    network: bool,
    scope: &str,
) -> CapabilitySafetyDescriptor {
    let credential = operations.iter().any(|operation| {
        matches!(
            *operation,
            OperationClass::CredentialRead | OperationClass::CredentialWrite
        )
    }) || resources.contains(&ResourceClass::CredentialStore);
    let data_sensitivity = if credential {
        DataSensitivity::Credential
    } else if resources.contains(&ResourceClass::EnvironmentVariables) {
        DataSensitivity::Secret
    } else {
        DataSensitivity::WorkspacePrivate
    };
    CapabilitySafetyDescriptor {
        capability_id: id.to_string(),
        operation_classes: operations,
        resource_classes: resources,
        input_sources: sources,
        output_sinks: sinks,
        external_effect: external,
        destructive,
        persistent_effect: persistent,
        requires_network: network,
        may_access_credentials: credential,
        effect_scope: scope.to_string(),
        risk_tags: vec!["canonical".to_string()],
        data_sensitivity,
        known: true,
        source: DescriptorSource::Canonical,
    }
}

fn infer_descriptor(
    capability_id: &str,
    arguments: &serde_json::Value,
) -> CapabilitySafetyDescriptor {
    let lower = capability_id.to_ascii_lowercase();
    let command = arguments
        .get("command")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let mut descriptor = CapabilitySafetyDescriptor::unknown(capability_id);
    descriptor.source = DescriptorSource::FallbackHeuristic;
    descriptor.risk_tags = vec!["fallback_heuristic".to_string()];
    let is_shell = lower.contains("shell") || lower.contains("bash") || lower.contains("exec");
    let is_network = lower.contains("fetch")
        || lower.contains("http")
        || lower.contains("network")
        || command.contains("curl")
        || command.contains("wget");
    let is_publish =
        lower.contains("push") || lower.contains("publish") || lower.contains("upload");
    let is_credential = lower.contains("credential")
        || lower.contains("secret")
        || lower.contains("keyring")
        || command.contains(".env")
        || command.contains("id_rsa");
    let is_delete =
        lower.contains("delete") || lower.contains("remove") || lower.contains("unlink");
    let is_write = lower.contains("write")
        || lower.contains("edit")
        || lower.contains("modify")
        || lower.contains("update")
        || is_delete;
    descriptor.operation_classes = vec![if is_credential {
        OperationClass::CredentialRead
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
    } else if lower.contains("read") || lower.contains("fs") {
        OperationClass::Read
    } else {
        OperationClass::Unknown
    }];
    descriptor.external_effect = is_shell
        || is_network
        || is_publish
        || descriptor.primary_operation() == OperationClass::Unknown;
    descriptor.destructive = is_delete;
    descriptor.persistent_effect = is_write;
    descriptor.requires_network = is_network;
    descriptor.may_access_credentials = is_credential || lower.contains("env");
    descriptor.known = false;
    if is_credential {
        descriptor.resource_classes = vec![ResourceClass::CredentialStore];
        descriptor.input_sources = vec![SourceClass::CredentialStore];
        descriptor.data_sensitivity = DataSensitivity::Credential;
    } else if lower.contains("env") {
        descriptor.resource_classes = vec![ResourceClass::EnvironmentVariables];
        descriptor.input_sources = vec![SourceClass::Environment];
        descriptor.data_sensitivity = DataSensitivity::Secret;
    } else if is_network || is_publish {
        descriptor.resource_classes = vec![ResourceClass::NetworkPublic];
        descriptor.output_sinks = vec![SinkClass::ExternalNetwork];
    } else if is_shell {
        descriptor.resource_classes = vec![ResourceClass::ProcessExecution];
        descriptor.output_sinks = vec![SinkClass::ShellExecution];
    } else if lower.contains("file") || lower.contains("fs") {
        descriptor.resource_classes = vec![ResourceClass::FilesystemWorkspace];
    }
    merge_command_effects(&mut descriptor, arguments);
    if descriptor.primary_operation() != OperationClass::Unknown {
        descriptor.source = DescriptorSource::FallbackHeuristic;
    }
    descriptor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_effect_fingerprints_align_across_tool_families() {
        let delete = descriptor_for_capability("fs.delete", &serde_json::json!({}));
        let rm = descriptor_for_capability(
            "shell.exec",
            &serde_json::json!({ "command": "rm file.txt" }),
        );
        assert_eq!(
            effect_fingerprint(&delete, &serde_json::json!({})),
            effect_fingerprint(&rm, &serde_json::json!({ "command": "rm file.txt" }))
        );

        let publish = descriptor_for_capability("repo.publish", &serde_json::json!({}));
        let push = descriptor_for_capability(
            "shell.exec",
            &serde_json::json!({ "command": "git push origin main" }),
        );
        assert_eq!(
            effect_fingerprint(&publish, &serde_json::json!({})),
            effect_fingerprint(
                &push,
                &serde_json::json!({ "command": "git push origin main" })
            )
        );

        let send = descriptor_for_capability("http.send", &serde_json::json!({}));
        let curl = descriptor_for_capability(
            "shell.exec",
            &serde_json::json!({ "command": "curl -X POST https://example.invalid -d a=1" }),
        );
        assert_eq!(
            effect_fingerprint(&send, &serde_json::json!({})),
            effect_fingerprint(
                &curl,
                &serde_json::json!({ "command": "curl -X POST https://example.invalid -d a=1" })
            )
        );
    }
}
