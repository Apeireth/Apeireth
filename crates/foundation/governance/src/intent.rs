//! Trusted, structured task intent shared by the runtime and governance.
//!
//! This module deliberately contains no raw user prompt.  An interpreter may
//! inspect a prompt at turn start, but the resulting envelope is the only
//! representation that crosses the safety boundary.

use serde::{Deserialize, Serialize};

/// Stable operation taxonomy used by capability safety descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationClass {
    Read,
    Search,
    Enumerate,
    Create,
    Write,
    Modify,
    Delete,
    Execute,
    SpawnProcess,
    NetworkRead,
    NetworkSend,
    Publish,
    CredentialRead,
    CredentialWrite,
    MemoryRead,
    MemoryWrite,
    AdminChange,
    PersistenceChange,
    Unknown,
}

impl Default for OperationClass {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Semantic class of a user task.  This is not a judgment about the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentClass {
    ReadOnlyInspection,
    Research,
    CodeAnalysis,
    CodeModification,
    FileManagement,
    RepositoryMaintenance,
    RepositoryPublish,
    NetworkResearch,
    DataTransformation,
    MemoryOperation,
    SystemAdministration,
    ExplicitDestructiveMaintenance,
    CredentialOperation,
    Unknown,
}

impl Default for IntentClass {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentExplicitness {
    Explicit,
    Inferred,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentProvenance {
    UserExplicitRequest,
    UiExplicitAction,
    SystemPolicy,
    HumanApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    Deny,
    LocalOnly,
    PublicRead,
    Allow,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialPolicy {
    Deny,
    ReadOnly,
    Allow,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellPolicy {
    Deny,
    ReadOnly,
    Allow,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationPolicy {
    Deny,
    WorkspaceOnly,
    Allow,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DestructivePolicy {
    Deny,
    RequireApproval,
    Allow,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistencePolicy {
    Deny,
    RequireApproval,
    Allow,
    Unknown,
}

/// Versioned intent envelope produced before a provider may use tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskIntentEnvelopeV1 {
    pub schema_version: String,
    pub intent_id: String,
    pub session_id: String,
    pub trace_id: String,
    pub intent_class: IntentClass,
    pub explicitness: IntentExplicitness,
    pub confidence: f64,
    pub requested_operations: Vec<OperationClass>,
    pub allowed_effects: Vec<OperationClass>,
    pub expected_resource_classes: Vec<String>,
    pub expected_capability_classes: Vec<String>,
    pub allowed_scopes: Vec<String>,
    pub allowed_data_sources: Vec<String>,
    pub allowed_sinks: Vec<String>,
    pub network_policy: NetworkPolicy,
    pub credential_policy: CredentialPolicy,
    pub shell_policy: ShellPolicy,
    pub mutation_policy: MutationPolicy,
    pub destructive_policy: DestructivePolicy,
    pub persistence_policy: PersistencePolicy,
    pub destination_constraints: Vec<String>,
    pub provenance: IntentProvenance,
    pub created_at_ms: i64,
}

impl TaskIntentEnvelopeV1 {
    pub const SCHEMA_VERSION: &'static str = "TaskIntentEnvelopeV1";

    pub fn unknown(session_id: impl Into<String>, trace_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        let trace_id = trace_id.into();
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            intent_id: format!("intent:{trace_id}"),
            session_id,
            trace_id,
            intent_class: IntentClass::Unknown,
            explicitness: IntentExplicitness::Unknown,
            confidence: 0.0,
            requested_operations: Vec::new(),
            allowed_effects: Vec::new(),
            expected_resource_classes: Vec::new(),
            expected_capability_classes: Vec::new(),
            allowed_scopes: Vec::new(),
            allowed_data_sources: Vec::new(),
            allowed_sinks: Vec::new(),
            network_policy: NetworkPolicy::Unknown,
            credential_policy: CredentialPolicy::Unknown,
            shell_policy: ShellPolicy::Unknown,
            mutation_policy: MutationPolicy::Unknown,
            destructive_policy: DestructivePolicy::Unknown,
            persistence_policy: PersistencePolicy::Unknown,
            destination_constraints: Vec::new(),
            provenance: IntentProvenance::UserExplicitRequest,
            created_at_ms: 0,
        }
    }

    pub fn allows_operation(&self, operation: OperationClass) -> bool {
        self.allowed_effects.contains(&operation)
            || self.requested_operations.contains(&operation)
            || matches!(operation, OperationClass::Read | OperationClass::Search)
                && self.allowed_effects.is_empty()
                && matches!(
                    self.intent_class,
                    IntentClass::ReadOnlyInspection | IntentClass::CodeAnalysis
                )
    }

    pub fn allows_network(&self) -> bool {
        matches!(
            self.network_policy,
            NetworkPolicy::PublicRead | NetworkPolicy::Allow
        )
    }

    pub fn allows_credentials(&self) -> bool {
        matches!(
            self.credential_policy,
            CredentialPolicy::ReadOnly | CredentialPolicy::Allow
        )
    }

    pub fn allows_shell(&self) -> bool {
        matches!(
            self.shell_policy,
            ShellPolicy::ReadOnly | ShellPolicy::Allow
        )
    }

    pub fn allows_mutation(&self) -> bool {
        matches!(
            self.mutation_policy,
            MutationPolicy::WorkspaceOnly | MutationPolicy::Allow
        )
    }

    pub fn allows_publish(&self) -> bool {
        self.allows_operation(OperationClass::Publish)
            || matches!(self.intent_class, IntentClass::RepositoryPublish)
    }
}

/// Small runtime-owned context carried with every governance request.
/// Guard-specific intent remains behind the governance boundary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnSecurityContext {
    pub intent_id: String,
    pub trace_id: String,
    pub task_scope: Option<String>,
    pub authorization_context: Option<String>,
    #[serde(default)]
    pub intent: Option<TaskIntentEnvelopeV1>,
}

impl TurnSecurityContext {
    pub fn new(intent_id: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self {
            intent_id: intent_id.into(),
            trace_id: trace_id.into(),
            task_scope: None,
            authorization_context: None,
            intent: None,
        }
    }

    #[must_use]
    pub fn with_task_scope(mut self, scope: impl Into<String>) -> Self {
        self.task_scope = Some(scope.into());
        self
    }

    #[must_use]
    pub fn with_intent(mut self, intent: TaskIntentEnvelopeV1) -> Self {
        self.intent = Some(intent);
        self
    }
}
