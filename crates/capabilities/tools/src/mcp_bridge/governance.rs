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

    /// Authorize every tool of `server` whose remote name is listed in
    /// `names`. Returns how many grants were added.
    ///
    /// The list is operator-authored (per-server `auto_authorize_tools`), not
    /// a server declaration: it exists for servers that under-declare
    /// read-only semantics.
    pub fn authorize_listed(
        policy: &mut PermissionPolicy,
        catalog: &McpToolCatalog,
        server: &str,
        names: &[String],
    ) -> usize {
        let mut granted = 0;
        for entry in catalog.list() {
            if entry.server == server
                && names.iter().any(|name| name == &entry.remote_name)
                && Self::authorize(policy, &entry)
            {
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
    fn operator_allowlist_authorizes_only_listed_tools() {
        let catalog = Arc::new(McpToolCatalog::new());
        for name in ["inspect_one", "mutate_two"] {
            let entry = McpToolEntry::new(
                "demo",
                &super::super::connection::DiscoveredTool {
                    name: name.to_string(),
                    description: None,
                    input_schema: None,
                    declared_read_only: false,
                },
            )
            .unwrap();
            catalog.insert(entry).unwrap();
        }

        let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
        let granted = McpPermissionMapping::authorize_listed(
            &mut policy.lock().unwrap(),
            &catalog,
            "demo",
            &["inspect_one".to_string()],
        );
        assert_eq!(granted, 1);

        let hook = McpRiskMappingHook::new(Arc::clone(&policy), Arc::clone(&catalog), false);
        let (call, capability) = call_on("tool.mcp.demo.inspect_one");
        assert_eq!(
            hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
            PreVerdict::Allow
        );

        let (call, capability) = call_on("tool.mcp.demo.mutate_two");
        assert!(matches!(
            hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
            PreVerdict::Ask { .. }
        ));
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

    /// **治理串台的前提**：治理面（授权档 / deny 名单 / 只读声明）全都按
    /// **能力身份**判定 ⇒ 目录里若同时躺着两个「模型名不同、能力身份相同」的
    /// 工具，授权其中一个就会把另一个一起授权（deny 名单同理）。
    /// 防线在登记层（[`McpToolCatalog`] 拒绝撞身份的登记）—— 这里钉住这条前提。
    #[test]
    fn one_capability_identity_never_carries_two_tools() {
        let catalog = McpToolCatalog::new();
        let dotted = McpToolEntry::new(
            "demo",
            &super::super::connection::DiscoveredTool {
                name: "x.y".to_string(),
                description: None,
                input_schema: None,
                declared_read_only: false,
            },
        )
        .unwrap();
        catalog.insert(dotted.clone()).unwrap();

        // 另一个模型名、同一个能力身份（`demo` 的 `x.y` 与 `demo.x` 的 `y`
        // 都合成 `tool.mcp.demo.x.y`）⇒ 两条登记路径都必须拒。
        let colliding = McpToolEntry::new(
            "demo.x",
            &super::super::connection::DiscoveredTool {
                name: "y".to_string(),
                description: None,
                input_schema: None,
                declared_read_only: false,
            },
        )
        .unwrap();
        assert_eq!(dotted.capability_id, colliding.capability_id);
        assert_ne!(dotted.model_name, colliding.model_name);
        assert!(
            catalog.insert(colliding.clone()).is_err(),
            "insert 必须拒撞能力身份的登记"
        );
        assert!(
            catalog.replace_server("demo.x", vec![colliding]).is_err(),
            "replace_server 必须拒撞能力身份的刷新"
        );

        // 目录里每个能力身份只有一个模型名 ⇒ 授权一个 = 授权一个。
        let mut identities = std::collections::BTreeMap::new();
        for entry in catalog.list() {
            assert!(
                identities
                    .insert(
                        entry.capability_id.as_str().to_string(),
                        entry.model_name.clone()
                    )
                    .is_none(),
                "一个能力身份不得承载两个工具"
            );
        }
    }
}
