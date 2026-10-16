//! Governance mapping for external tools.
//!
//! Three cooperating pieces, all consulted on every external call:
//!
//! * [`McpRiskMappingHook`] (pre-execute waterfall): an external tool's
//!   permission declaration maps to explicit [`PermissionPolicy`] grants —
//!   granted and unmarked means authorized; with no explicit authorization
//!   the call sits at the require-approval level and goes to a human;
//!   `APEIRETH_MCP_READONLY_PRESET=1` releases server-declared read-only
//!   tools (still through the full five-stage pipeline);
//! * [`McpToolDenyGuard`] (monotonic guard stage): a deny-only blocklist, so
//!   an operator can hard-refuse specific external capabilities;
//! * [`McpPermissionMapping`]: the seeding helpers that turn external tool
//!   permission declarations into [`PermissionPolicy`] grants.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use apeireth_governance::{Decision, Permission, PermissionPolicy, ToolGuard, ToolGuardRequest};

use crate::exec_pipeline::{PreExecuteHook, PreExecuteRequest, PreVerdict};

use super::catalog::{McpToolCatalog, McpToolEntry};

/// Pre-execute risk mapping for external tools.
///
/// Verdict order for one call:
///
/// 1. an explicit grant in the shared [`PermissionPolicy`] (and no approval
///    marking on it) authorizes the call;
/// 2. an approval marking keeps it at the require-approval level (`ask`);
/// 3. otherwise the read-only preset may release a tool the server declared
///    read-only; anything else stays at the require-approval level.
///
/// Nothing here executes a tool or mints an approval — it only states what a
/// human still has to decide.
pub struct McpRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
    catalog: Arc<McpToolCatalog>,
    readonly_preset: bool,
}

impl McpRiskMappingHook {
    /// A hook over the shared policy and the live tool catalog.
    pub fn new(
        policy: Arc<Mutex<PermissionPolicy>>,
        catalog: Arc<McpToolCatalog>,
        readonly_preset: bool,
    ) -> Self {
        Self {
            policy,
            catalog,
            readonly_preset,
        }
    }
}

impl PreExecuteHook for McpRiskMappingHook {
    fn name(&self) -> &str {
        "mcp_risk_mapping"
    }

    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict {
        let capability = request.capability.as_str();
        let decision = self
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .decision_for_capability(capability);
        match decision {
            Decision::Allow => PreVerdict::Allow,
            Decision::RequireApproval { reason } => PreVerdict::Ask { reason },
            Decision::Deny { .. } => {
                if self.readonly_preset && self.catalog.is_declared_read_only(request.capability) {
                    PreVerdict::Allow
                } else {
                    PreVerdict::Ask {
                        reason: format!(
                            "external tool {capability} sits at the require-approval level: \
                             no explicit authorization for it"
                        ),
                    }
                }
            }
        }
    }
}

/// A monotonic, deny-only blocklist for external capabilities.
pub struct McpToolDenyGuard {
    blocked: Mutex<BTreeSet<String>>,
}

impl McpToolDenyGuard {
    /// An empty blocklist.
    pub fn new() -> Self {
        Self {
            blocked: Mutex::new(BTreeSet::new()),
        }
    }

    /// Block one capability. Returns whether it was newly blocked.
    pub fn deny_capability(&self, capability: impl Into<String>) -> bool {
        self.blocked
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(capability.into())
    }

    /// Unblock one capability.
    pub fn allow_capability(&self, capability: &str) -> bool {
        self.blocked
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(capability)
    }

    /// Whether a capability is currently blocked.
    pub fn is_blocked(&self, capability: &str) -> bool {
        self.blocked
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(capability)
    }
}

impl Default for McpToolDenyGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolGuard for McpToolDenyGuard {
    fn name(&self) -> &str {
        "mcp_tool_deny"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        let capability = request.capability.as_str();
        if self.is_blocked(capability) {
            Some(format!(
                "external tool {capability} is blocked for this run"
            ))
        } else {
            None
        }
    }
}

/// Seeding helpers: external tool permission declarations become explicit
/// [`PermissionPolicy`] grants.
pub struct McpPermissionMapping;

impl McpPermissionMapping {
    /// Explicitly authorize one external tool in `policy`.
    pub fn authorize(policy: &mut PermissionPolicy, entry: &McpToolEntry) -> bool {
        policy.grant(Permission::ExecuteTool(
            entry.capability_id.as_str().to_string(),
        ))
    }

    /// Map every server-declared read-only tool in `catalog` to an explicit
    /// grant. Returns how many grants were added.
    pub fn authorize_readonly_declarations(
        policy: &mut PermissionPolicy,
        catalog: &McpToolCatalog,
    ) -> usize {
        let mut granted = 0;
        for entry in catalog.list() {
            if entry.declared_read_only && Self::authorize(policy, &entry) {
                granted += 1;
            }
        }
        granted
    }

    /// Mark one external tool as requiring human approval even when its
    /// permission is granted.
    pub fn require_approval(policy: &mut PermissionPolicy, entry: &McpToolEntry) {
        policy.require_approval_for(entry.capability_id.as_str().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::CapabilityId;

    fn catalog_with(server: &str, tool: &str, read_only: bool) -> Arc<McpToolCatalog> {
        let catalog = Arc::new(McpToolCatalog::new());
        let entry = McpToolEntry::new(
            server,
            &super::super::connection::DiscoveredTool {
                name: tool.to_string(),
                description: None,
                input_schema: None,
                declared_read_only: read_only,
            },
        )
        .unwrap();
        catalog.insert(entry).unwrap();
        catalog
    }

    fn call_on(capability: &str) -> (apeireth_protocol::canonical::ToolCall, CapabilityId) {
        (
            apeireth_protocol::canonical::ToolCall {
                id: "call_1".into(),
                name: "mcp:demo:lookup".into(),
                arguments: serde_json::json!({}),
            },
            CapabilityId::new(capability).unwrap(),
        )
    }

    #[test]
    fn ungranted_external_tools_stay_at_the_require_approval_level() {
        let hook = McpRiskMappingHook::new(
            Arc::new(Mutex::new(PermissionPolicy::new())),
            catalog_with("demo", "lookup", false),
            false,
        );
        let (call, capability) = call_on("tool.mcp.demo.lookup");
        let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
        assert!(matches!(verdict, PreVerdict::Ask { .. }), "{verdict:?}");
    }

    #[test]
    fn explicit_grant_authorizes_the_call() {
        let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
        let catalog = catalog_with("demo", "lookup", true);
        let entry = catalog.get("mcp:demo:lookup").unwrap();
        McpPermissionMapping::authorize(&mut policy.lock().unwrap(), &entry);
        let hook = McpRiskMappingHook::new(Arc::clone(&policy), catalog, false);
        let (call, capability) = call_on("tool.mcp.demo.lookup");
        let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
        assert_eq!(verdict, PreVerdict::Allow);
    }

    #[test]
    fn readonly_preset_releases_declared_read_only_tools_only() {
        let catalog = catalog_with("demo", "lookup", true);
        let hook = McpRiskMappingHook::new(
            Arc::new(Mutex::new(PermissionPolicy::new())),
            Arc::clone(&catalog),
            true,
        );
        let (call, capability) = call_on("tool.mcp.demo.lookup");
        let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
        assert_eq!(verdict, PreVerdict::Allow);

        // A tool that did not declare itself read-only is still approval level.
        let write_catalog = catalog_with("demo", "write-item", false);
        let hook = McpRiskMappingHook::new(
            Arc::new(Mutex::new(PermissionPolicy::new())),
            write_catalog,
            true,
        );
        let (call, capability) = call_on("tool.mcp.demo.write-item");
        let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
        assert!(matches!(verdict, PreVerdict::Ask { .. }), "{verdict:?}");
    }

    #[test]
    fn deny_guard_refuses_blocked_capabilities() {
        let guard = McpToolDenyGuard::new();
        guard.deny_capability("tool.mcp.demo.lookup");
        let (call, capability) = call_on("tool.mcp.demo.lookup");
        let request = ToolGuardRequest::new(&capability, "mcp:demo:lookup", &call.arguments);
        assert!(guard.deny(&request).is_some());
        assert!(!guard.is_blocked("tool.mcp.other"));
    }
}
