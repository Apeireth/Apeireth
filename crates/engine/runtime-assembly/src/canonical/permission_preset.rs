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

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
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
}

impl PermissionPresetGovernanceHook {
    /// Wrap `inner`, reading session settings from `sessions`.
    pub fn new(inner: Arc<dyn GovernanceHook>, sessions: Arc<dyn SessionStore>) -> Self {
        Self { inner, sessions }
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
        // with the default (`standard`) when the turn starts.
        let preset = loaded
            .as_ref()
            .map(|session| session.settings.permission_preset)
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
            // `standard` and read-only-but-read-tool keep existing policy.
            _ => self.inner.evaluate_verbose(request).await,
        }
    }
}
