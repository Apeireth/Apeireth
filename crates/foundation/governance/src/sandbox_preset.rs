//! Named permission presets: a sandbox mode and an approval policy bundled
//! under one configuration name.
//!
//! A preset is a *naming* convenience for the configuration surface: the same
//! pair `(SandboxMode, ApprovalPolicyName)` can always be expressed directly,
//! and any pair outside the named set derives [`PresetName::Custom`]. The
//! mapping here is data only — no decision semantics live in it.

use serde::{Deserialize, Serialize};

use crate::sandbox_ladder::SandboxMode;

/// The approval half of a preset: whether a human decision is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPolicyName {
    /// Gated calls ask a human (the approval channel stays in force).
    Ask,
    /// No approval prompts; the operator accepts the risk in advance.
    Never,
}

impl ApprovalPolicyName {
    /// Stable wire label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Never => "never",
        }
    }
}

impl std::fmt::Display for ApprovalPolicyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A named (sandbox mode, approval policy) bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetName {
    /// [`SandboxMode::Strict`] + [`ApprovalPolicyName::Ask`].
    StrictAsk,
    /// [`SandboxMode::Standard`] + [`ApprovalPolicyName::Ask`] — the default
    /// bundle: sandboxed execution with every gated call asking a human.
    StandardAsk,
    /// [`SandboxMode::Relaxed`] + [`ApprovalPolicyName::Ask`].
    RelaxedAsk,
    /// [`SandboxMode::Permissive`] + [`ApprovalPolicyName::Never`].
    PermissiveNever,
    /// Derived: the requested pair is not one of the named bundles.
    Custom,
}

impl PresetName {
    /// Stable wire label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::StrictAsk => "strict_ask",
            Self::StandardAsk => "standard_ask",
            Self::RelaxedAsk => "relaxed_ask",
            Self::PermissiveNever => "permissive_never",
            Self::Custom => "custom",
        }
    }
}

impl std::fmt::Display for PresetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// The named bundles: `PresetName -> (mode, approval_policy)`.
pub const PRESET_BINDINGS: &[(PresetName, (SandboxMode, ApprovalPolicyName))] = &[
    (
        PresetName::StrictAsk,
        (SandboxMode::Strict, ApprovalPolicyName::Ask),
    ),
    (
        PresetName::StandardAsk,
        (SandboxMode::Standard, ApprovalPolicyName::Ask),
    ),
    (
        PresetName::RelaxedAsk,
        (SandboxMode::Relaxed, ApprovalPolicyName::Ask),
    ),
    (
        PresetName::PermissiveNever,
        (SandboxMode::Permissive, ApprovalPolicyName::Never),
    ),
];

/// The pair a named preset stands for. [`PresetName::Custom`] has no fixed
/// pair — that is what makes it custom.
pub fn preset_binding(name: PresetName) -> Option<(SandboxMode, ApprovalPolicyName)> {
    PRESET_BINDINGS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, binding)| *binding)
}

/// The name of a `(mode, approval_policy)` pair, deriving
/// [`PresetName::Custom`] for anything outside [`PRESET_BINDINGS`].
pub fn derive_preset(mode: SandboxMode, approval: ApprovalPolicyName) -> PresetName {
    PRESET_BINDINGS
        .iter()
        .find(|(_, binding)| *binding == (mode, approval))
        .map(|(name, _)| *name)
        .unwrap_or(PresetName::Custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_bindings_map_names_to_mode_and_approval_policy() {
        assert_eq!(
            preset_binding(PresetName::StrictAsk),
            Some((SandboxMode::Strict, ApprovalPolicyName::Ask))
        );
        assert_eq!(
            preset_binding(PresetName::StandardAsk),
            Some((SandboxMode::Standard, ApprovalPolicyName::Ask))
        );
        assert_eq!(
            preset_binding(PresetName::RelaxedAsk),
            Some((SandboxMode::Relaxed, ApprovalPolicyName::Ask))
        );
        assert_eq!(
            preset_binding(PresetName::PermissiveNever),
            Some((SandboxMode::Permissive, ApprovalPolicyName::Never))
        );
        // The naming round-trips through derive.
        for name in [
            PresetName::StrictAsk,
            PresetName::StandardAsk,
            PresetName::RelaxedAsk,
            PresetName::PermissiveNever,
        ] {
            let (mode, approval) = preset_binding(name).unwrap();
            assert_eq!(derive_preset(mode, approval), name, "{name}");
        }
    }

    #[test]
    fn unlisted_pairs_derive_custom() {
        // The named set is not the full product: any other bundle is custom.
        assert_eq!(
            derive_preset(SandboxMode::Permissive, ApprovalPolicyName::Ask),
            PresetName::Custom
        );
        assert_eq!(
            derive_preset(SandboxMode::Strict, ApprovalPolicyName::Never),
            PresetName::Custom
        );
        assert_eq!(
            derive_preset(SandboxMode::Standard, ApprovalPolicyName::Never),
            PresetName::Custom
        );
        // Custom stands for no fixed pair.
        assert_eq!(preset_binding(PresetName::Custom), None);
        assert_eq!(PresetName::Custom.label(), "custom");
    }
}
