//! Governance mapping for external tools.
//!
//! Three cooperating pieces, all consulted on every external call:
//!
//! * [`McpRiskMappingHook`] (pre-execute waterfall): a **deny-only gate**.
//!   It asks one question — "does the shared policy refuse this capability?"
//!   — and refuses (`PreVerdict::Deny`) when it does. Whatever the policy
//!   allows (including a capability marked as requiring approval) is
//!   **passed through** to the runtime governance chain, which is the only
//!   place a human approval is obtained;
//! * [`McpToolDenyGuard`] (monotonic guard stage): a deny-only blocklist, so
//!   an operator can hard-refuse specific external capabilities. The
//!   operator's surface is the per-server `deny_tools` list of the
//!   configuration ([`super::config::McpServerSpec`]): the bridge seeds the
//!   gate from that list when it is assembled, so the blocklist has a
//!   **production injection point** and a denied tool is refused **before it
//!   executes** — no matter what else authorizes it;
//! * [`McpPermissionMapping`]: the seeding helpers that turn external tool
//!   permission declarations into [`PermissionPolicy`] grants.
//!
//! # Why the bridge no longer states approval verdicts
//!
//! The runtime governance chain (T1) runs **before** the tool pipeline: it
//! suspends a call for a human and resumes the call once the human decided.
//! The pipeline then walks its own stages. If the bridge's hook also stated
//! an approval verdict, the resume path would state it a second time — the
//! call would be asked about twice and would die on `pipeline.pre_ask` even
//! though the human had already approved it. The structural fix is here:
//!
//! * **the human approval right belongs to the runtime governance chain
//!   alone** (T1 is the only level whose `RequireApproval` suspends a call
//!   for a human);
//! * the tool pipeline keeps a **monotonic deny-only gate** (T2): it can
//!   only tighten, never loosen, and it never asks.
//!
//! The layering is therefore: **runtime governance chain (T1) → tool
//! pipeline (T2)**. The governance decision always comes first; the pipeline
//! only refuses what the chain already refused (plus the operator's own
//! blocklist).

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use apeireth_governance::{Decision, Permission, PermissionPolicy, ToolGuard, ToolGuardRequest};

use crate::exec_pipeline::{PreExecuteHook, PreExecuteRequest, PreVerdict};

use super::catalog::{capability_id_for, McpCatalogError, McpToolCatalog, McpToolEntry};
use super::family::{is_capability_in_family, MCP_FAMILY_ROOT};

/// Family-scoped authorization the bridge holds locally.
///
/// The production [`PermissionPolicy`] does not carry family grants yet: the
/// governance base batch that adds `grant_family` /
/// `require_approval_for_family` and a family decision helper has not
/// landed. Until it does, the bridge keeps the family surface here so
/// "authorized at family scope" is expressible — and says plainly that this
/// copy is **not** what the runtime governance chain reads.
///
/// The family **roots** themselves are not defined here: they come from
/// [`super::family`], the single true source for the family vocabulary, so
/// the bridge's grant surface cannot drift from the tool-taxonomy surface.
///
/// TODO(governance-base): replace this registry with the production family
/// helpers (`PermissionPolicy::grant_family`,
/// `PermissionPolicy::require_approval_for_family`, and the family decision
/// helper) and drop this copy — one policy, one truth. Until the wiring
/// batch hands the runtime the same shared policy, a family-granted
/// capability is still refused by the runtime governance chain
/// (fail-closed); this registry only widens what the **bridge's** gate lets
/// through.
#[derive(Debug, Default)]
pub struct McpFamilyGrants {
    granted: Mutex<BTreeSet<String>>,
    approval_required: Mutex<BTreeSet<String>>,
}

impl McpFamilyGrants {
    /// An empty family registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant one family root (e.g. [`MCP_FAMILY_ROOT`]). Returns whether it
    /// was newly granted.
    pub fn grant_family(&self, family: impl Into<String>) -> bool {
        self.granted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(family.into())
    }

    /// Grant the whole external-tool family.
    pub fn grant_mcp_family(&self) -> bool {
        self.grant_family(MCP_FAMILY_ROOT)
    }

    /// Mark one family as requiring approval even when it is granted.
    ///
    /// The marking is a record of operator intent while the production
    /// family helper is pending (see the type-level note); the bridge's gate
    /// does not act on it — approvals belong to the runtime governance chain.
    pub fn require_approval_for_family(&self, family: impl Into<String>) -> bool {
        self.approval_required
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(family.into())
    }

    /// Revoke one family grant. Returns whether it was granted.
    pub fn revoke_family(&self, family: &str) -> bool {
        self.granted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(family)
    }

    /// Whether one family is granted.
    pub fn is_granted(&self, family: &str) -> bool {
        self.granted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(family)
    }

    /// Whether one family is marked as requiring approval.
    pub fn requires_approval(&self, family: &str) -> bool {
        self.approval_required
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(family)
    }

    /// Whether any granted family covers `capability_id`.
    ///
    /// Membership is decided by the shared family vocabulary (segment
    /// boundary), so a grant of `tool.mcp` covers `tool.mcp.demo.lookup` and
    /// nothing else.
    pub fn grants_capability(&self, capability_id: &str) -> bool {
        self.granted
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .any(|family| is_capability_in_family(capability_id, family))
    }
}

/// Pre-execute risk mapping for external tools — a **deny-only gate**.
///
/// One call, one question: does the shared policy refuse this capability?
///
/// * `Decision::Deny` → `PreVerdict::Deny` — the call is **refused**. A
///   refusal is final for that call (the pipeline closes it as
///   `pipeline.pre_deny`);
/// * `Decision::Allow` and `Decision::RequireApproval` →
///   `PreVerdict::Allow` — the call is **passed through** to the runtime
///   governance chain, which is where a `RequireApproval` suspends for a
///   human and where the human decision is taken.
///
/// The hook never asks and never mints an approval: `PreVerdict::Ask` would
/// close as a permanent error (`pipeline.pre_ask`), not as a suspended
/// approval, and would break the runtime's approval-resume path (the resumed
/// call would ask a second time and fail even though the human had already
/// approved).
pub struct McpRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
    catalog: Arc<McpToolCatalog>,
    family_grants: Arc<McpFamilyGrants>,
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
            family_grants: Arc::new(McpFamilyGrants::new()),
            readonly_preset,
        }
    }

    /// Attach the bridge-local family grant registry (builder style).
    ///
    /// Without it the gate only knows the shared policy's per-capability
    /// grants. See [`McpFamilyGrants`] for what this copy is (and is not).
    pub fn with_family_grants(self, family_grants: Arc<McpFamilyGrants>) -> Self {
        Self {
            family_grants,
            ..self
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
            // The policy allows it: hand it to the runtime governance chain.
            Decision::Allow => PreVerdict::Allow,
            // The policy wants a human: that is the runtime chain's business
            // (T1 suspends the call and resumes it after the decision). The
            // pipeline passes it through so the resume path is not asked
            // twice.
            Decision::RequireApproval { .. } => PreVerdict::Allow,
            Decision::Deny { reason } => {
                // The read-only preset releases server-declared read-only
                // tools. The release is one fact expressed twice: as a
                // policy grant (so the runtime chain can see it) and here
                // (so the gate does not refuse what the operator released).
                if self.readonly_preset && self.catalog.is_declared_read_only(request.capability) {
                    return PreVerdict::Allow;
                }
                // A family grant widens authorization from the server scope
                // to the whole external family.
                if self.family_grants.grants_capability(capability) {
                    return PreVerdict::Allow;
                }
                PreVerdict::Deny {
                    reason: format!("external tool {capability} is not authorized: {reason}"),
                }
            }
        }
    }
}

/// A monotonic, deny-only blocklist for external capabilities.
///
/// The gate may only **tighten**: a blocked capability stays blocked, and
/// nothing that runs after it (the read-only preset, the operator allowlist,
/// a family grant) can turn the refusal back into a pass. It answers one
/// question — *is this capability blocked?* — and it answers it at the
/// **guard stage of the tool pipeline**, which runs **before** the tool
/// executes.
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

    /// Block every listed remote tool of one server.
    ///
    /// The capability id is composed exactly the way the dynamic directory
    /// composes it (`tool.mcp.<server>.<tool>`, see
    /// [`capability_id_for`]), so the blocklist and the directory can never
    /// disagree about which identity a tool carries.
    ///
    /// This is the bridge's **production injection point** for the deny
    /// surface: the operator's `deny_tools` list is folded into the gate
    /// when the bridge is assembled, so the gate is armed before the first
    /// discovery runs.
    ///
    /// Returns how many identities were newly blocked. A name that cannot
    /// compose into a stable id is refused (fail-closed) rather than
    /// silently skipped.
    pub fn deny_server_tools(
        &self,
        server: &str,
        tool_names: &[String],
    ) -> Result<usize, McpCatalogError> {
        let mut blocked = 0;
        for name in tool_names {
            let capability = capability_id_for(server, name)?;
            if self.deny_capability(capability.as_str()) {
                blocked += 1;
            }
        }
        Ok(blocked)
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

    /// The one question this stage asks: is the capability blocked?
    ///
    /// A refusal here is **final for the call**: the pipeline closes it as
    /// `pipeline.guard_deny` and the tool never runs. Nothing that ran
    /// earlier — the read-only preset release, the operator allowlist, a
    /// family grant — can prevent the refusal, and nothing that runs later
    /// can overturn it (the guard vocabulary has no allow verdict).
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
///
/// # The grant says "in the authorization surface", never "exempt from the human"
///
/// From this batch on, **every external call is approved one by one** — the
/// `full` session preset does not exempt it either (the approval floor in
/// the governance base enforces that). A grant therefore answers one
/// question only: *is this capability inside the authorization surface at
/// all?* The per-call human decision belongs to the runtime governance
/// chain, so the seeding helpers that the bridge uses pair every grant with
/// the per-capability approval marking ([`McpPermissionMapping::authorize_for_approval`]):
/// granted **and** marked ⇒ "in the surface, but every call still waits for
/// a human".
pub struct McpPermissionMapping;

impl McpPermissionMapping {
    /// Explicitly authorize one external tool in `policy`.
    ///
    /// This is a bare grant: it says the capability is in the authorization
    /// surface. It does **not** exempt the capability from the per-call human
    /// approval — that exemption does not exist for external tools.
    pub fn authorize(policy: &mut PermissionPolicy, entry: &McpToolEntry) -> bool {
        policy.grant(Permission::ExecuteTool(
            entry.capability_id.as_str().to_string(),
        ))
    }

    /// Authorize one external tool **and** keep it at the per-call human
    /// approval level.
    ///
    /// The grant says "this capability is in the authorization surface"; the
    /// approval marking says "every call still needs a human". This is the
    /// shape every grant the bridge itself seeds takes.
    pub fn authorize_for_approval(policy: &mut PermissionPolicy, entry: &McpToolEntry) -> bool {
        let granted = Self::authorize(policy, entry);
        Self::require_approval(policy, entry);
        granted
    }

    /// Map every server-declared read-only tool in `catalog` to an explicit
    /// grant. Returns how many grants were added.
    ///
    /// Bare grants only — see [`McpPermissionMapping::authorize_for_approval`]
    /// for the grant-plus-approval shape the bridge uses.
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

    /// Map every server-declared read-only tool in `catalog` to a grant
    /// **plus** the per-call approval marking. Returns how many grants were
    /// added.
    ///
    /// The read-only preset releases server-declared read-only tools into the
    /// authorization surface; it never releases them from the human.
    pub fn authorize_readonly_declarations_for_approval(
        policy: &mut PermissionPolicy,
        catalog: &McpToolCatalog,
    ) -> usize {
        let mut granted = 0;
        for entry in catalog.list() {
            if entry.declared_read_only && Self::authorize_for_approval(policy, &entry) {
                granted += 1;
            }
        }
        granted
    }

    /// Authorize every tool of `server` whose remote name is listed in
    /// `names`; returns the count. The list is operator-authored (per-server
    /// `auto_authorize_tools`), not a server declaration: it covers servers
    /// that under-declare read-only semantics.
    ///
    /// Bare grants only — see [`McpPermissionMapping::authorize_listed_for_approval`]
    /// for the grant-plus-approval shape the bridge uses.
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

    /// Authorize every listed tool **and** keep each of them at the per-call
    /// human approval level; returns the count.
    ///
    /// **Degraded semantics of the operator allowlist**
    /// (`auto_authorize_tools`): the list decides *whether the capability is
    /// granted* — whether it is inside the authorization surface at all. It
    /// does **not** mean "exempt from the per-call human approval": external
    /// calls are approved one by one, allowlisted or not. An operator reading
    /// "auto_authorize" must not conclude "no human will be asked".
    pub fn authorize_listed_for_approval(
        policy: &mut PermissionPolicy,
        catalog: &McpToolCatalog,
        server: &str,
        names: &[String],
    ) -> usize {
        let mut granted = 0;
        for entry in catalog.list() {
            if entry.server == server
                && names.iter().any(|name| name == &entry.remote_name)
                && Self::authorize_for_approval(policy, &entry)
            {
                granted += 1;
            }
        }
        granted
    }

    /// Mark one external tool as requiring approval even when its permission
    /// is granted.
    ///
    /// This is the T1 signal ("this capability needs a human, every time"),
    /// not a bridge-level verdict: the bridge's gate passes such calls
    /// through to the runtime governance chain.
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
    fn ungranted_external_tools_are_denied_by_the_gate() {
        // The gate refuses what the shared policy refuses. It does not ask:
        // asking is the runtime governance chain's business, and an `ask`
        // verdict would close as `pipeline.pre_ask` instead of a suspended
        // approval.
        let hook = McpRiskMappingHook::new(
            Arc::new(Mutex::new(PermissionPolicy::new())),
            catalog_with("demo", "lookup", false),
            false,
        );
        let (call, capability) = call_on("tool.mcp.demo.lookup");
        let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
        assert!(matches!(verdict, PreVerdict::Deny { .. }), "{verdict:?}");
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
    fn approval_marking_passes_through_to_the_runtime_chain() {
        // A capability the policy marks as requiring approval is handed to
        // the runtime governance chain, which is where the human decision is
        // taken. The gate must not turn it into an `ask` verdict: the resume
        // path would ask a second time and the approved call would still
        // fail.
        let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
        let catalog = catalog_with("demo", "lookup", false);
        let entry = catalog.get("mcp:demo:lookup").unwrap();
        McpPermissionMapping::authorize(&mut policy.lock().unwrap(), &entry);
        McpPermissionMapping::require_approval(&mut policy.lock().unwrap(), &entry);
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

        // A tool that did not declare itself read-only is refused, not asked.
        let write_catalog = catalog_with("demo", "write-item", false);
        let hook = McpRiskMappingHook::new(
            Arc::new(Mutex::new(PermissionPolicy::new())),
            write_catalog,
            true,
        );
        let (call, capability) = call_on("tool.mcp.demo.write-item");
        let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
        assert!(matches!(verdict, PreVerdict::Deny { .. }), "{verdict:?}");
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
            PreVerdict::Deny { .. }
        ));
    }

    #[test]
    fn listed_grants_keep_the_per_call_approval_level() {
        // The allowlist decides the authorization surface, never the human:
        // a listed tool is granted **and** marked to require approval.
        let catalog = Arc::new(McpToolCatalog::new());
        let entry = McpToolEntry::new(
            "demo",
            &super::super::connection::DiscoveredTool {
                name: "inspect_one".to_string(),
                description: None,
                input_schema: None,
                declared_read_only: false,
            },
        )
        .unwrap();
        catalog.insert(entry).unwrap();

        let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
        let granted = McpPermissionMapping::authorize_listed_for_approval(
            &mut policy.lock().unwrap(),
            &catalog,
            "demo",
            &["inspect_one".to_string()],
        );
        assert_eq!(granted, 1);

        let hook = McpRiskMappingHook::new(Arc::clone(&policy), Arc::clone(&catalog), false);
        let (call, capability) = call_on("tool.mcp.demo.inspect_one");
        // Granted + marked ⇒ handed to the runtime chain (which asks).
        assert_eq!(
            hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
            PreVerdict::Allow
        );
        let decision = policy
            .lock()
            .unwrap()
            .decision_for_capability("tool.mcp.demo.inspect_one");
        assert!(
            matches!(decision, Decision::RequireApproval { .. }),
            "a listed tool still waits for a human: {decision:?}"
        );
    }

    #[test]
    fn family_grant_widens_the_gate_to_the_whole_family() {
        // Family scope is the bridge's own record until the governance base
        // ships the family helpers; it widens the gate from one capability
        // to the whole external family.
        let family_grants = Arc::new(McpFamilyGrants::new());
        assert!(family_grants.grant_mcp_family());
        assert!(family_grants.is_granted(MCP_FAMILY_ROOT));
        assert!(family_grants.grants_capability("tool.mcp.demo.lookup"));
        assert!(!family_grants.grants_capability("tool.shell"));

        let hook = McpRiskMappingHook::new(
            Arc::new(Mutex::new(PermissionPolicy::new())),
            catalog_with("demo", "lookup", false),
            false,
        )
        .with_family_grants(Arc::clone(&family_grants));
        let (call, capability) = call_on("tool.mcp.demo.lookup");
        assert_eq!(
            hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
            PreVerdict::Allow
        );

        // A capability outside the family stays refused.
        let (call, capability) = call_on("tool.shell");
        assert!(matches!(
            hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
            PreVerdict::Deny { .. }
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
