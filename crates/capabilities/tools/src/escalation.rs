//! In-place upgrade guidance for tool boundary refusals.
//!
//! When a call is refused at a tool boundary, the refusal message carries a
//! structured hint at the decision point: which sandbox mode is missing and
//! which reason field the caller must fill in to request a one-call upgrade.
//! The operator sees the next step in the refusal itself instead of hunting
//! through a settings surface for a permanent switch.
//!
//! The wire vocabulary matches the canonical escalation ladder in
//! `apeireth-governance::sandbox_ladder`: same request key
//! (`sandbox_escalation`), same mandatory reason field (`justification`), same
//! grant scope (`one_call`). Refusals with no upgrade rung — content guardrail
//! floors such as destructive command patterns and protected credential paths —
//! say so explicitly rather than pointing at an upgrade that does not exist.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Where the caller attaches the structured escalation request.
pub const ESCALATION_REQUEST_KEY: &str = "sandbox_escalation";

/// The mandatory reason field inside the request.
pub const JUSTIFICATION_FIELD: &str = "justification";

/// What an approved grant covers: exactly one call.
pub const GRANT_SCOPE: &str = "one_call";

/// The first ladder rung that lifts the workspace-directory boundary.
pub const OUT_OF_WORKSPACE_MODE: &str = "relaxed";

/// Structured in-place guidance appended to a boundary refusal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeHint {
    /// The ladder rung that would permit the refused operation, or `None` when
    /// the refusal is a floor at every rung and no upgrade applies.
    pub missing_mode: Option<String>,
    /// Where the caller attaches the structured request.
    pub request_key: String,
    /// The mandatory reason field inside the request.
    pub justification_field: String,
    /// What an approved grant covers.
    pub grant_scope: String,
    /// How to proceed at the call site.
    pub guidance: String,
}

impl UpgradeHint {
    /// Hint for a refusal caused by the workspace-directory boundary: the
    /// missing rung is the first one that permits out-of-workspace access.
    pub fn out_of_workspace_access() -> Self {
        Self {
            missing_mode: Some(OUT_OF_WORKSPACE_MODE.to_string()),
            request_key: ESCALATION_REQUEST_KEY.to_string(),
            justification_field: JUSTIFICATION_FIELD.to_string(),
            grant_scope: GRANT_SCOPE.to_string(),
            guidance: format!(
                "to exceed the workspace directory for this call only, attach \
                 {ESCALATION_REQUEST_KEY} = {{\"mode\":\"{OUT_OF_WORKSPACE_MODE}\",\"{JUSTIFICATION_FIELD}\":\"<non-empty reason>\"}}; \
                 a granted upgrade covers exactly one call and never changes configuration"
            ),
        }
    }

    /// Hint for a refusal that is a content floor: no sandbox mode and no
    /// one-call upgrade permits it, and the refusal is final.
    pub fn content_floor() -> Self {
        Self {
            missing_mode: None,
            request_key: ESCALATION_REQUEST_KEY.to_string(),
            justification_field: JUSTIFICATION_FIELD.to_string(),
            grant_scope: GRANT_SCOPE.to_string(),
            guidance: "this refusal is a content floor at every sandbox mode; no one-call \
                       upgrade applies"
                .to_string(),
        }
    }

    /// The hint as one compact JSON object, ready to embed in a refusal.
    pub fn render(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| self.guidance.clone())
    }
}

impl fmt::Display for UpgradeHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_hint_names_missing_mode_and_how_to_apply() {
        let hint = UpgradeHint::out_of_workspace_access();
        let rendered = hint.render();
        assert_eq!(hint.missing_mode.as_deref(), Some("relaxed"));
        assert!(
            rendered.contains("\"missing_mode\":\"relaxed\""),
            "{rendered}"
        );
        assert!(
            rendered.contains("\"request_key\":\"sandbox_escalation\""),
            "{rendered}"
        );
        assert!(
            rendered.contains("\"justification_field\":\"justification\""),
            "{rendered}"
        );
        assert!(
            rendered.contains("\"grant_scope\":\"one_call\""),
            "{rendered}"
        );
        // 就地提示: the guidance shows the exact request shape and the
        // one-call nature of the grant.
        assert!(
            hint.guidance.contains("sandbox_escalation")
                && hint.guidance.contains("justification")
                && hint.guidance.contains("exactly one call"),
            "{}",
            hint.guidance
        );
    }

    #[test]
    fn content_floor_hint_states_that_no_upgrade_exists() {
        let hint = UpgradeHint::content_floor();
        let rendered = hint.render();
        assert_eq!(hint.missing_mode, None);
        assert!(rendered.contains("\"missing_mode\":null"), "{rendered}");
        assert!(
            hint.guidance.contains("content floor") && hint.guidance.contains("no one-call"),
            "{}",
            hint.guidance
        );
    }
}
