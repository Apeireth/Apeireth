//! Silence module: SilenceReason - "not acting is also a legal action" expressed explicitly.

use serde::{Deserialize, Serialize};

/// Silence reason - explicitly states "why we are not acting".
///
/// **Core position**: silence is not a bug; it is a legal output. Any silence must have a reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SilenceReason {
    /// Not silent - normal output.
    NotSilent,
    /// Outside current scope (no permission/context).
    OutOfScope,
    /// No consent (tenant has not authorized / multi-sig not reached).
    NoConsent,
    /// No need to act right now (scenario does not match).
    NoNeed,
    /// Deliberately chose silence (reflection / waiting).
    Deliberate,
    /// Ethical doubt (violates 13-key or core principles).
    EthicalDoubt,
}

impl SilenceReason {
    /// Whether truly silent (anything other than NotSilent).
    pub fn is_silent(&self) -> bool {
        !matches!(self, SilenceReason::NotSilent)
    }

    /// Display name.
    pub const fn name(&self) -> &'static str {
        match self {
            SilenceReason::NotSilent => "not_silent",
            SilenceReason::OutOfScope => "out_of_scope",
            SilenceReason::NoConsent => "no_consent",
            SilenceReason::NoNeed => "no_need",
            SilenceReason::Deliberate => "deliberate",
            SilenceReason::EthicalDoubt => "ethical_doubt",
        }
    }

    /// Priority (higher = more urgent — for scheduling).
    pub const fn priority(&self) -> u8 {
        match self {
            SilenceReason::EthicalDoubt => 5,
            SilenceReason::NoConsent => 4,
            SilenceReason::OutOfScope => 3,
            SilenceReason::NoNeed => 2,
            SilenceReason::Deliberate => 1,
            SilenceReason::NotSilent => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_silent_is_not_silent() {
        assert!(!SilenceReason::NotSilent.is_silent());
    }

    #[test]
    fn other_reasons_are_silent() {
        assert!(SilenceReason::OutOfScope.is_silent());
        assert!(SilenceReason::NoConsent.is_silent());
        assert!(SilenceReason::NoNeed.is_silent());
        assert!(SilenceReason::Deliberate.is_silent());
        assert!(SilenceReason::EthicalDoubt.is_silent());
    }

    #[test]
    fn priority_orders_correctly() {
        // EthicalDoubt > NoConsent > OutOfScope > NoNeed > Deliberate > NotSilent
        assert!(SilenceReason::EthicalDoubt.priority() > SilenceReason::NoConsent.priority());
        assert!(SilenceReason::NoConsent.priority() > SilenceReason::OutOfScope.priority());
        assert!(SilenceReason::OutOfScope.priority() > SilenceReason::NoNeed.priority());
        assert!(SilenceReason::NoNeed.priority() > SilenceReason::Deliberate.priority());
        assert!(SilenceReason::Deliberate.priority() > SilenceReason::NotSilent.priority());
    }

    #[test]
    fn names_are_stable_strings() {
        assert_eq!(SilenceReason::NotSilent.name(), "not_silent");
        assert_eq!(SilenceReason::EthicalDoubt.name(), "ethical_doubt");
    }

    #[test]
    fn priority_uniqueness() {
        let mut seen = std::collections::HashSet::new();
        for reason in [
            SilenceReason::NotSilent,
            SilenceReason::OutOfScope,
            SilenceReason::NoConsent,
            SilenceReason::NoNeed,
            SilenceReason::Deliberate,
            SilenceReason::EthicalDoubt,
        ] {
            seen.insert(reason.priority());
        }
        assert_eq!(seen.len(), 6);
    }
}
