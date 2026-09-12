//! Session-level `permission_preset` enforcement as a governance hook.
//!
//! The canonical execution core stays capability-generic; it only consults the
//! injected [`GovernanceHook`]. This production hook layers the session's
//! durable `permission_preset` on top of the existing policy decision chain:
//!
//! - `read_only` refuses write/execute tool calls with a human-readable denial;
//! - `standard` delegates to the inner policy (dangerous tools still require
//!   approval);
//! - `full` allows what the inner policy would only allow after approval, while
//!   keeping the runtime's normal audit/trace logging for the dispatch.
//!
//! The preset is read from the same [`SessionStore`] the runtime uses, so it
//! only affects the session's *subsequent* turns and never rewrites an approval
//! that is already in flight.

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict};
use apeireth_runtime::canonical::{PermissionPreset, SessionStore};
use async_trait::async_trait;

/// Classify write/execute capabilities in the production composition layer.
///
/// This is intentionally NOT in the execution core: the runtime must not know
/// which concrete capabilities exist. The classifier lives here, next to the
/// concrete tool modules it names.
pub fn is_write_or_execute_capability(capability: &CapabilityId) -> bool {
    const WRITE_OR_EXECUTE: &[&str] = &[
        "tool.shell",
        "tool.repo",
        "tool.process",
        "tool.supervisor",
        "tool.std_sub_supervisor",
    ];
    let id = capability.as_str();
    WRITE_OR_EXECUTE.contains(&id) || id.starts_with("tool.mcp")
}

/// Governance hook that applies a session's [`PermissionPreset`] to capability
/// dispatch, delegating everything else to the wrapped inner hook.
pub struct PermissionPresetGovernanceHook {
    inner: Arc<dyn GovernanceHook>,
    sessions: Arc<dyn SessionStore>,
    /// In-process memory of `(session, capability)` pairs a human already
    /// approved in this process. Deliberately not persisted: a restart clears
    /// it, so approval memory never outlives the process.
    approved_pairs: RwLock<HashSet<(String, String)>>,
}

impl PermissionPresetGovernanceHook {
    /// Wrap `inner`, reading session settings from `sessions`.
    pub fn new(inner: Arc<dyn GovernanceHook>, sessions: Arc<dyn SessionStore>) -> Self {
        Self {
            inner,
            sessions,
            approved_pairs: RwLock::new(HashSet::new()),
        }
    }

    fn remembered(&self, session: &SessionId, capability: &CapabilityId) -> bool {
        self.approved_pairs
            .read()
            .expect("approval memory lock poisoned")
            .contains(&(session.to_string(), capability.as_str().to_string()))
    }
}

#[async_trait]
impl GovernanceHook for PermissionPresetGovernanceHook {
    fn name(&self) -> &str {
        "permission_preset"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        self.evaluate_verbose(request).await.decision
    }

    fn approval_resolved(&self, session: &SessionId, capability: &CapabilityId) {
        self.approved_pairs
            .write()
            .expect("approval memory lock poisoned")
            .insert((session.to_string(), capability.as_str().to_string()));
    }

    /// Preserve the identity of the deciding hook: delegated decisions keep the
    /// inner hook's attribution (e.g. `permission_governance`), while preset
    /// denials and `full`-mode approval rewrites are attributed to this hook.
    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let Action::CapabilityDispatch { capability, .. } = &request.action else {
            return self.inner.evaluate_verbose(request).await;
        };

        let loaded = match self.sessions.load(&request.session).await {
            Ok(loaded) => loaded,
            // Fail closed: refusing to read settings must never widen access.
            Err(error) => {
                return GovernanceVerdict::new(
                    self.name(),
                    Decision::deny(format!(
                        "无法读取会话权限预设，已拒绝执行 (fail closed): {error}"
                    )),
                );
            }
        };

        // A session that does not exist yet has no preset; it will be created
        // with the default (`standard`, `approval_remember = false`) when the
        // turn starts.
        let (preset, approval_remember) = loaded
            .as_ref()
            .map(|session| {
                (
                    session.settings.permission_preset,
                    session.settings.approval_remember,
                )
            })
            .unwrap_or_default();

        match preset {
            PermissionPreset::ReadOnly if is_write_or_execute_capability(capability) => {
                GovernanceVerdict::new(
                    self.name(),
                    Decision::deny(format!(
                        "当前会话为只读权限预设 (read_only)，已拒绝写/执行类工具 {}。如需执行，请将会话权限预设调整为 standard 或 full。",
                        capability
                    )),
                )
            }
            PermissionPreset::Full => {
                let verdict = self.inner.evaluate_verbose(request).await;
                match verdict.decision {
                    Decision::RequireApproval { .. } => {
                        GovernanceVerdict::new(self.name(), Decision::Allow)
                    }
                    _ => verdict,
                }
            }
            PermissionPreset::Standard => {
                // `approval_remember` only applies to the standard preset: it
                // skips a *previously approved* approval prompt, never a denial.
                // `read_only` stays refuse-closed and `full` already allows.
                if approval_remember && self.remembered(&request.session, capability) {
                    let verdict = self.inner.evaluate_verbose(request).await;
                    match verdict.decision {
                        Decision::RequireApproval { .. } => {
                            GovernanceVerdict::new(self.name(), Decision::Allow)
                        }
                        // Fail closed: memory skips approval only; it must not
                        // widen a real denial into an allow.
                        _ => verdict,
                    }
                } else {
                    self.inner.evaluate_verbose(request).await
                }
            }
            // read_only-but-read-tool keeps existing policy.
            PermissionPreset::ReadOnly => self.inner.evaluate_verbose(request).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{system_clock, TraceId};
    use apeireth_governance::{Permission, PermissionGovernanceHook, PermissionPolicy};
    use apeireth_runtime::canonical::{InMemorySessionStore, Session};

    async fn build_hook() -> (Arc<PermissionPresetGovernanceHook>, Arc<dyn SessionStore>) {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.shell".into()));
        policy.require_approval_for("tool.shell");
        policy.grant(Permission::ExecuteTool("tool.calculator".into()));
        policy.require_approval_for("tool.calculator");

        let inner = Arc::new(PermissionGovernanceHook::new(policy));
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let hook = Arc::new(PermissionPresetGovernanceHook::new(inner, store.clone()));
        (hook, store)
    }

    async fn save_session(
        store: &Arc<dyn SessionStore>,
        session: SessionId,
        preset: PermissionPreset,
        approval_remember: bool,
    ) {
        let clock = system_clock();
        let mut stored = Session::new(session, clock.as_ref());
        stored.settings.permission_preset = preset;
        stored.settings.approval_remember = approval_remember;
        store.save(&stored).await.unwrap();
    }

    fn dispatch_request<'a>(
        session: SessionId,
        capability: &'a CapabilityId,
        arguments: &'a serde_json::Value,
    ) -> GovernanceRequest<'a> {
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            session,
            TraceId::new(),
            1,
        )
    }

    #[tokio::test]
    async fn approved_pair_skips_approval_in_standard_with_remember() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;

        let args = serde_json::Value::Null;
        let before = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &args))
            .await;
        assert!(
            matches!(before.decision, Decision::RequireApproval { .. }),
            "before approval the tool must still require approval"
        );

        hook.approval_resolved(&session, &capability);

        let after = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &args))
            .await;
        assert!(after.decision.is_allowed(), "remembered approval must allow");
        assert_eq!(after.hook, "permission_preset");
    }

    #[tokio::test]
    async fn approval_memory_is_scoped_to_session_and_capability() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let other_session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        let other_capability = CapabilityId::new("tool.calculator").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;
        save_session(&store, other_session, PermissionPreset::Standard, true).await;

        hook.approval_resolved(&session, &capability);

        let args = serde_json::Value::Null;
        let other_session_verdict = hook
            .evaluate_verbose(&dispatch_request(other_session, &capability, &args))
            .await;
        assert!(
            matches!(
                other_session_verdict.decision,
                Decision::RequireApproval { .. }
            ),
            "a different session must not inherit the memory"
        );

        let other_capability_verdict = hook
            .evaluate_verbose(&dispatch_request(session, &other_capability, &args))
            .await;
        assert!(
            matches!(
                other_capability_verdict.decision,
                Decision::RequireApproval { .. }
            ),
            "a different capability must not inherit the memory"
        );
    }

    #[tokio::test]
    async fn approval_memory_never_bypasses_a_denial() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.unpermitted").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;

        // Seed memory directly to prove the fail-closed path is independent of
        // how the pair got there.
        hook.approval_resolved(&session, &capability);

        let args = serde_json::Value::Null;
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &args))
            .await;
        assert!(
            matches!(verdict.decision, Decision::Deny { .. }),
            "memory must skip approval only, never a denial"
        );
    }

    #[tokio::test]
    async fn approval_memory_is_disabled_when_remember_is_false() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        save_session(&store, session, PermissionPreset::Standard, false).await;

        hook.approval_resolved(&session, &capability);

        let args = serde_json::Value::Null;
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &args))
            .await;
        assert!(
            matches!(verdict.decision, Decision::RequireApproval { .. }),
            "approval_remember = false must keep prompting every time"
        );
    }
}
