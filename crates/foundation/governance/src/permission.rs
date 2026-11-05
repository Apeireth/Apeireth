//! Small deterministic permission primitives, aligned with the onion
//! permission layer.
//!
//! The earlier `PrincipleOnion` (hardcoded philosophical prose) and `DslOnion`
//! (ad-hoc expression parsing) were **not** implemented here. What remains is the useful
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
    /// Administrative override. Kept as an explicit grant; a
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
    /// Granted capability **families** (segment-boundary prefix roots).
    ///
    /// A family is a set of capability ids that share a root but are
    /// discovered at runtime, so they cannot be enumerated one by one (the
    /// external-tool bridge produces `tool.mcp.<server>.<tool>`).
    ///
    /// Families live in their own set instead of new [`Permission`] variants:
    /// the enum is **not** `#[non_exhaustive]`, so a new variant would break
    /// downstream exhaustive matches, and the variant declaration order is a
    /// documented iteration-order contract for [`PermissionSet`].
    #[serde(default)]
    granted_families: BTreeSet<String>,
    /// Capability families whose **every** member call needs a human decision.
    ///
    /// A family approval mark never grants anything: without a matching
    /// grant (exact or family) the capability stays denied.
    #[serde(default)]
    approval_families: BTreeSet<String>,
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

    /// Grant a capability **family** (segment-boundary prefix).
    ///
    /// `family` is a family root such as `tool.mcp`; it covers every
    /// capability id that continues the root with a **dot**
    /// (`tool.mcp.<server>.<tool>`). The boundary is the dot:
    ///
    /// * `tool.mcp.demo.lookup` — covered.
    /// * `tool.mcpsomething` — **not** covered (same text, different segment).
    /// * `tool.mcp` — **not** covered (the bare root is not a member of
    ///   itself; it is the family's root, not one of its members).
    ///
    /// A family grant is a **fallback**: an exact grant still wins, and a
    /// capability with no grant at all stays denied — a family never widens
    /// a [`Decision::Deny`].
    ///
    /// The root is stored verbatim after stripping trailing dots; an empty
    /// root is refused (it would match nothing real, so it stays a no-op).
    pub fn grant_family(&mut self, family: &str) {
        if let Some(root) = normalize_family_root(family) {
            self.granted_families.insert(root);
        }
    }

    /// Mark a capability **family** as requiring human approval on **every**
    /// member call.
    ///
    /// Same segment-boundary semantics as [`PermissionPolicy::grant_family`].
    /// A family approval mark never grants anything: without a matching
    /// grant (exact or family) the capability stays denied — the deny check
    /// runs before the approval check.
    pub fn require_approval_for_family(&mut self, family: &str) {
        if let Some(root) = normalize_family_root(family) {
            self.approval_families.insert(root);
        }
    }

    /// Evaluate capability dispatch for `capability`.
    ///
    /// Decision order:
    ///
    /// 1. **Grant gate** — an exact grant, a family grant, or an
    ///    `AdminOverride` lets the capability through; otherwise
    ///    [`Decision::Deny`].
    /// 2. **Exact approval mark** -> [`Decision::RequireApproval`].
    /// 3. **Family approval mark** -> [`Decision::RequireApproval`].
    /// 4. Otherwise -> [`Decision::Allow`].
    ///
    /// An exact mark always wins over a family mark (both deny the call
    /// either way, but the exact one is evaluated first).
    pub fn decision_for_capability(&self, capability: &str) -> Decision {
        let required = Permission::ExecuteTool(capability.to_string());
        let exact_grant = self.grants.has(&required);
        let family_grant = !exact_grant
            && self
                .granted_families
                .iter()
                .any(|root| capability_in_family(capability, root));
        if !exact_grant && !family_grant && !self.grants.has(&Permission::AdminOverride) {
            return Decision::deny(format!("capability {capability} is not permitted"));
        }

        if self.approval_capabilities.contains(capability) {
            return Decision::require_approval(format!(
                "capability {capability} requires human approval"
            ));
        }

        if self
            .approval_families
            .iter()
            .any(|root| capability_in_family(capability, root))
        {
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

    /// The capability **families** that are granted.
    ///
    /// A family root covers every capability under it on a segment boundary
    /// (see [`PermissionPolicy::grant_family`]). Read-only introspection:
    /// surfaces that show what the owner authorized (grants listing, panel
    /// readout) read it here instead of keeping a second source of truth.
    pub fn granted_families(&self) -> impl Iterator<Item = &str> {
        self.granted_families.iter().map(String::as_str)
    }

    /// The capability **families** whose members each need a human decision.
    pub fn approval_families(&self) -> impl Iterator<Item = &str> {
        self.approval_families.iter().map(String::as_str)
    }
}

/// Normalize a family root: trim surrounding blanks and trailing dots.
///
/// An empty root is refused (`None`): it would stand for "every id that
/// starts with a dot", which is no real capability, so registering it would
/// be a silent no-op at best and a false grant at worst.
fn normalize_family_root(family: &str) -> Option<String> {
    let root = family.trim().trim_end_matches('.');
    if root.is_empty() {
        None
    } else {
        Some(root.to_string())
    }
}

/// Whether `capability` is a member of the family rooted at `root`.
///
/// The **segment boundary** is the dot: the id must continue the root with
/// `.`, so `tool.mcp` covers `tool.mcp.demo.lookup` but never
/// `tool.mcpsomething` and never the bare root itself.
///
/// No allocation: the prefix is stripped, not formatted.
fn capability_in_family(capability: &str, root: &str) -> bool {
    capability
        .strip_prefix(root)
        .is_some_and(|rest| rest.starts_with('.'))
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

    // ---------- 族授权（段边界前缀族） ----------

    #[test]
    fn family_grant_covers_members_but_neither_the_root_nor_a_textual_prefix() {
        let mut policy = PermissionPolicy::new();
        policy.grant_family("tool.mcp");

        // 族成员命中 ⇒ Allow。
        assert!(policy
            .decision_for_capability("tool.mcp.demo.lookup")
            .is_allowed());

        // 族根自身（无后缀点）不命中 ⇒ Deny。
        let root = policy.decision_for_capability("tool.mcp");
        assert!(matches!(root, Decision::Deny { .. }), "{root:?}");
        assert!(root.reason().unwrap().contains("is not permitted"));

        // 同文本前缀不同段（无点）不命中 ⇒ Deny。
        let prefix = policy.decision_for_capability("tool.mcpsomething");
        assert!(matches!(prefix, Decision::Deny { .. }), "{prefix:?}");

        // 别的族不命中 ⇒ Deny。
        let other = policy.decision_for_capability("tool.foo");
        assert!(matches!(other, Decision::Deny { .. }), "{other:?}");

        // 段边界后再一段也不算命中（不是任意前缀）。
        let deeper = policy.decision_for_capability("tool.mcp2.demo");
        assert!(matches!(deeper, Decision::Deny { .. }), "{deeper:?}");
    }

    #[test]
    fn family_grant_does_not_widen_capabilities_outside_the_family() {
        let mut policy = PermissionPolicy::new();
        policy.grant_family("tool.mcp");

        for id in [
            "tool.shell",
            "tool.mcp2.demo",
            "tool.mcpsomething",
            "tool.mcp",
        ] {
            let decision = policy.decision_for_capability(id);
            assert!(
                matches!(decision, Decision::Deny { .. }),
                "{id} must stay denied under a `tool.mcp` family grant, got {decision:?}"
            );
        }
    }

    #[test]
    fn family_approval_requires_a_human_decision_for_every_member() {
        let mut policy = PermissionPolicy::new();
        policy.grant_family("tool.mcp");
        policy.require_approval_for_family("tool.mcp");

        for id in [
            "tool.mcp.demo.lookup",
            "tool.mcp.demo.write-item",
            "tool.mcp.x",
        ] {
            let decision = policy.decision_for_capability(id);
            assert!(
                matches!(decision, Decision::RequireApproval { .. }),
                "{id} must require human approval under a family approval mark, got {decision:?}"
            );
            assert!(decision.reason().unwrap().contains("human approval"));
        }
    }

    #[test]
    fn family_approval_without_a_matching_grant_stays_denied() {
        // 短路顺序：先判 grant 再判 approval —— 只有审批标记没有 grant ⇒ 仍拒。
        let mut policy = PermissionPolicy::new();
        policy.require_approval_for_family("tool.mcp");

        let decision = policy.decision_for_capability("tool.mcp.demo.lookup");
        assert!(matches!(decision, Decision::Deny { .. }), "{decision:?}");
        assert!(decision.reason().unwrap().contains("is not permitted"));
    }

    #[test]
    fn exact_grant_and_exact_approval_win_over_the_family_marks() {
        let mut policy = PermissionPolicy::new();
        // 精确 grant + 族 approval ⇒ 精确 grant 过闸门，族 approval 仍要求人批。
        policy.grant(Permission::ExecuteTool("tool.mcp.demo.lookup".into()));
        policy.require_approval_for_family("tool.mcp");
        let exact = policy.decision_for_capability("tool.mcp.demo.lookup");
        assert!(
            matches!(exact, Decision::RequireApproval { .. }),
            "{exact:?}"
        );

        // 精确 approval 与族 approval 同时命中 ⇒ 仍是人批（精确优先判定）。
        policy.require_approval_for("tool.mcp.demo.lookup");
        let both = policy.decision_for_capability("tool.mcp.demo.lookup");
        assert!(matches!(both, Decision::RequireApproval { .. }), "{both:?}");
        assert!(both.reason().unwrap().contains("human approval"));

        // 精确 grant 命中 ⇒ Allow（族里没有审批标记时）。
        let mut plain = PermissionPolicy::new();
        plain.grant(Permission::ExecuteTool("tool.mcp.demo.lookup".into()));
        plain.grant_family("tool.mcp");
        assert!(plain
            .decision_for_capability("tool.mcp.demo.lookup")
            .is_allowed());
    }

    #[test]
    fn admin_override_waives_the_missing_grant_but_never_the_approval_mark() {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::AdminOverride);
        // override 只豁免「无 grant 即拒」。
        assert!(policy.decision_for_capability("tool.shell").is_allowed());

        // 审批标记不被 override 豁免（既有语义）。
        policy.require_approval_for("tool.shell");
        assert!(matches!(
            policy.decision_for_capability("tool.shell"),
            Decision::RequireApproval { .. }
        ));

        // 族审批标记同样不被 override 豁免。
        policy.require_approval_for_family("tool.mcp");
        let family = policy.decision_for_capability("tool.mcp.demo.lookup");
        assert!(
            matches!(family, Decision::RequireApproval { .. }),
            "{family:?}"
        );
    }

    #[test]
    fn an_empty_family_root_is_refused_instead_of_granting_a_phantom_family() {
        let mut policy = PermissionPolicy::new();
        policy.grant_family("");
        policy.require_approval_for_family("");

        for id in ["tool.shell", "tool.mcp.demo.lookup", ".leading-dot"] {
            let decision = policy.decision_for_capability(id);
            assert!(
                matches!(decision, Decision::Deny { .. }),
                "{id} must stay denied, got {decision:?}"
            );
        }
    }

    #[test]
    fn a_policy_without_families_behaves_exactly_as_before() {
        // 零族登记 ⇒ 逐字沿用既有语义：精确 grant 放行、审批标记要人批、
        // 其余一律拒。
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.calculator".into()));
        policy.grant(Permission::ExecuteTool("tool.shell".into()));
        policy.require_approval_for("tool.shell");

        assert!(policy
            .decision_for_capability("tool.calculator")
            .is_allowed());
        let approved = policy.decision_for_capability("tool.shell");
        assert!(
            matches!(approved, Decision::RequireApproval { .. }),
            "{approved:?}"
        );
        let denied = policy.decision_for_capability("tool.fetch");
        assert!(matches!(denied, Decision::Deny { .. }), "{denied:?}");
        assert_eq!(
            denied.reason(),
            Some("capability tool.fetch is not permitted"),
            "Deny 文案不得因族机制而变"
        );
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
