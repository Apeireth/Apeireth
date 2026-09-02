//! Small deterministic permission primitives, adapted from the donor's onion
//! permission layer.
//!
//! The donor's `PrincipleOnion` (hardcoded philosophical prose) and `DslOnion`
//! (ad-hoc expression parsing) were **not** ported. What remains is the useful
//! deterministic core: a [`Permission`] set and a [`PermissionPolicy`] that maps
//! capability dispatch to the canonical [`Decision`] semantics.
//!
//! A [`PermissionGovernanceHook`] wraps the policy so it can participate in the
//! canonical [`GovernancePipeline`]. It does not create a second pipeline or a
//! second decision enum.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::{Action, Decision, GovernanceHook, GovernanceRequest};

/// A single permission grant.
///
/// Variants are intentionally generic capability/resource shapes, not concrete
/// tool names or vendor endpoints.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "permission", content = "value", rename_all = "snake_case")]
pub enum Permission {
    /// Read from the canonical memory domain.
    ReadMemory,
    /// Write to the canonical memory domain.
    WriteMemory,
    /// Execute a named tool capability.
    ExecuteTool(String),
    /// Egress to a named network scope.
    NetworkEgress(String),
    /// Modify identity or self-model state.
    ModifyIdentity,
    /// Administrative override. Kept from the donor as an explicit grant; a
    /// policy that does not want override semantics simply never grants it.
    AdminOverride,
}

impl Permission {
    /// A stable label for reports.
    pub fn label(&self) -> String {
        match self {
            Self::ReadMemory => "read_memory".to_string(),
            Self::WriteMemory => "write_memory".to_string(),
            Self::ExecuteTool(name) => format!("execute_tool:{name}"),
            Self::NetworkEgress(scope) => format!("network_egress:{scope}"),
            Self::ModifyIdentity => "modify_identity".to_string(),
            Self::AdminOverride => "admin_override".to_string(),
        }
    }
}

/// A deterministic permission set.
///
/// Uses a `BTreeSet`, so iteration and decision evaluation are reproducible
/// across runs with the same grants.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionSet {
    permissions: BTreeSet<Permission>,
}

impl PermissionSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant a permission. Returns `true` when it was newly inserted.
    pub fn grant(&mut self, permission: Permission) -> bool {
        self.permissions.insert(permission)
    }

    /// Revoke a permission. Returns `true` when it was present.
    pub fn revoke(&mut self, permission: &Permission) -> bool {
        self.permissions.remove(permission)
    }

    /// Whether the set contains the permission.
    pub fn has(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Number of granted permissions.
    pub fn len(&self) -> usize {
        self.permissions.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }

    /// Iterate the grants in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &Permission> {
        self.permissions.iter()
    }
}

/// A small permission policy for capability dispatch.
///
/// The policy answers one question: may this named capability execute? The
/// answer is always one of the canonical [`Decision`] variants.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionPolicy {
    grants: PermissionSet,
    approval_capabilities: BTreeSet<String>,
}

impl PermissionPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant a permission.
    pub fn grant(&mut self, permission: Permission) -> bool {
        self.grants.grant(permission)
    }

    /// Revoke a permission.
    pub fn revoke(&mut self, permission: &Permission) -> bool {
        self.grants.revoke(permission)
    }

    /// Whether a permission is currently granted.
    pub fn has(&self, permission: &Permission) -> bool {
        self.grants.has(permission)
    }

    /// Iterate the current grants in deterministic order (panel surface).
    pub fn iter(&self) -> impl Iterator<Item = &Permission> {
        self.grants.iter()
    }

    /// Mark a capability as requiring human approval even when the permission
    /// is granted.
    pub fn require_approval_for(&mut self, capability: impl Into<String>) {
        self.approval_capabilities.insert(capability.into());
    }

    /// Evaluate capability dispatch for `capability`.
    ///
    /// * Missing grant (and no `AdminOverride`) -> [`Decision::Deny`].
    /// * Grant present but capability marked for approval -> [`Decision::RequireApproval`].
    /// * Otherwise -> [`Decision::Allow`].
    pub fn decision_for_capability(&self, capability: &str) -> Decision {
        let required = Permission::ExecuteTool(capability.to_string());
        if !self.grants.has(&required) && !self.grants.has(&Permission::AdminOverride) {
            return Decision::deny(format!("capability {capability} is not permitted"));
        }

        if self.approval_capabilities.contains(capability) {
            return Decision::require_approval(format!(
                "capability {capability} requires human approval"
            ));
        }

        Decision::Allow
    }

    /// The capabilities that are marked for human approval.
    pub fn approval_capabilities(&self) -> impl Iterator<Item = &str> {
        self.approval_capabilities.iter().map(String::as_str)
    }
}

/// Canonical governance hook wrapper for [`PermissionPolicy`].
///
/// The policy lives behind `Arc<Mutex<PermissionPolicy>>` so the composition
/// root can share one mutable policy between the live hook and introspection
/// surfaces (grants listing / session-scoped hot revoke).
#[derive(Debug, Clone)]
pub struct PermissionGovernanceHook {
    name: &'static str,
    policy: Arc<Mutex<PermissionPolicy>>,
}

impl PermissionGovernanceHook {
    pub fn new(policy: PermissionPolicy) -> Self {
        Self {
            name: "permission_governance",
            policy: Arc::new(Mutex::new(policy)),
        }
    }

    pub fn named(name: &'static str, policy: PermissionPolicy) -> Self {
        Self {
            name,
            policy: Arc::new(Mutex::new(policy)),
        }
    }

    /// Wrap an already-shared policy so hot revokes take effect on the live
    /// hook immediately (session-scoped; process restart restores defaults).
    pub fn new_shared(policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self {
            name: "permission_governance",
            policy,
        }
    }

    /// The shared policy handle (for panel introspection).
    pub fn policy(&self) -> &Arc<Mutex<PermissionPolicy>> {
        &self.policy
    }
}

#[async_trait::async_trait]
impl GovernanceHook for PermissionGovernanceHook {
    fn name(&self) -> &str {
        self.name
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        match &request.action {
            Action::CapabilityDispatch { capability, .. } => {
                let policy = self
                    .policy
                    .lock()
                    .expect("permission policy mutex poisoned");
                policy.decision_for_capability(capability.as_str())
            }
            Action::Completion { .. } => Decision::Allow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GovernancePipeline;
    use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
    use serde_json::Value;
    use std::sync::Arc;

    #[test]
    fn permission_set_grant_revoke_and_has_are_deterministic() {
        let mut set = PermissionSet::new();
        set.grant(Permission::ExecuteTool("shell".into()));
        set.grant(Permission::ReadMemory);
        set.grant(Permission::ExecuteTool("fetch".into()));

        let labels: Vec<String> = set.iter().map(Permission::label).collect();
        // BTreeSet orders variants by declaration order first, then by
        // contained value; this is the deterministic contract.
        let expected = vec!["read_memory", "execute_tool:fetch", "execute_tool:shell"];
        assert_eq!(labels, expected);

        assert!(set.has(&Permission::ExecuteTool("shell".into())));
        assert!(set.revoke(&Permission::ExecuteTool("shell".into())));
        assert!(!set.has(&Permission::ExecuteTool("shell".into())));
        assert!(!set.revoke(&Permission::ExecuteTool("shell".into())));
    }

    #[test]
    fn permission_policy_denies_missing_grants_and_allows_granted() {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.calculator".into()));

        assert!(policy
            .decision_for_capability("tool.calculator")
            .is_allowed());
        assert!(matches!(
            policy.decision_for_capability("tool.shell"),
            Decision::Deny { .. }
        ));
    }

    #[test]
    fn permission_policy_requires_approval_for_marked_capability() {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.shell".into()));
        policy.require_approval_for("tool.shell");

        let decision = policy.decision_for_capability("tool.shell");
        assert!(matches!(decision, Decision::RequireApproval { .. }));
        assert!(decision.reason().unwrap().contains("human approval"));
    }

    #[test]
    fn admin_override_can_be_granted_but_is_not_assumed() {
        let mut policy = PermissionPolicy::new();
        assert!(matches!(
            policy.decision_for_capability("tool.shell"),
            Decision::Deny { .. }
        ));

        policy.grant(Permission::AdminOverride);
        assert!(policy.decision_for_capability("tool.shell").is_allowed());
    }

    #[test]
    fn missing_attributes_deny_closed() {
        // A capability with no grant and no override must never be allowed.
        let policy = PermissionPolicy::new();
        assert!(matches!(
            policy.decision_for_capability("tool.shell"),
            Decision::Deny { .. }
        ));
    }

    fn dispatch_request<'a>(
        capability: &'a CapabilityId,
        arguments: &'a Value,
        round: u32,
    ) -> GovernanceRequest<'a> {
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            SessionId::new(),
            TraceId::new(),
            round,
        )
    }

    fn completion_request() -> GovernanceRequest<'static> {
        GovernanceRequest::new(
            Action::Completion {
                model: "fake-model-1",
                message_count: 2,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        )
    }

    #[tokio::test]
    async fn permission_hook_uses_canonical_decisions() {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.calculator".into()));
        let hook = PermissionGovernanceHook::new(policy);

        let calc = CapabilityId::new("tool.calculator").unwrap();
        let shell = CapabilityId::new("tool.shell").unwrap();
        let args = Value::Null;

        assert_eq!(hook.name(), "permission_governance");
        assert!(hook
            .evaluate(&dispatch_request(&calc, &args, 1))
            .await
            .is_allowed());
        assert!(matches!(
            hook.evaluate(&dispatch_request(&shell, &args, 1)).await,
            Decision::Deny { .. }
        ));
    }

    #[tokio::test]
    async fn permission_hook_allows_completions() {
        let hook = PermissionGovernanceHook::new(PermissionPolicy::new());
        assert!(hook.evaluate(&completion_request()).await.is_allowed());
    }

    #[tokio::test]
    async fn permission_hook_reports_hook_identity_in_pipeline() {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.calculator".into()));
        let hook = PermissionGovernanceHook::new(policy);
        let pipeline = GovernancePipeline::new().with(Arc::new(hook));

        let shell = CapabilityId::new("tool.shell").unwrap();
        let args = Value::Null;
        let verdict = pipeline
            .evaluate_verbose(&dispatch_request(&shell, &args, 1))
            .await;
        assert_eq!(verdict.hook, "permission_governance");
        assert!(!verdict.is_allowed());
    }
}
