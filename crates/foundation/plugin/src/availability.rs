//! Three-axis honesty record recovered from
//! `legacy/canonical/apeireth-companion/src/capabilities_manifest.rs`.
//!
//! The canonical catalog was a 20-row companion front-desk. This helper keeps the
//! **reporting contract**, not the catalog:
//!
//! - `supported` — the code exists
//! - `available` — this build / configuration can actually run it
//! - `reason` — if it cannot run, a byte-level why; never `None` on a false
//!
//! A record with `available = true` and a `reason`, or `available = false`
//! without one, is a construction error. Callers that want a live listing
//! still ask [`crate::PluginManager`] / [`crate::CapabilityRegistry`]; this
//! type is only the observation row.

/// One honesty row: code present, currently usable, and why not if not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Availability {
    name: String,
    supported: bool,
    available: bool,
    reason: Option<String>,
}

impl Availability {
    /// Code exists and this configuration can run it. `reason` is `None`.
    pub fn live(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            supported: true,
            available: true,
            reason: None,
        }
    }

    /// Code exists but this configuration cannot run it. `reason` is required.
    pub fn blocked(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            supported: true,
            available: false,
            reason: Some(reason.into()),
        }
    }

    /// Code is absent. `available` is forced false; `reason` is required.
    pub fn absent(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            supported: false,
            available: false,
            reason: Some(reason.into()),
        }
    }

    /// Fallible constructor that enforces the honesty invariant.
    pub fn try_new(
        name: impl Into<String>,
        supported: bool,
        available: bool,
        reason: Option<String>,
    ) -> Result<Self, AvailabilityError> {
        let name = name.into();
        if available && !supported {
            return Err(AvailabilityError::AvailableWithoutSupport { name });
        }
        if available && reason.is_some() {
            return Err(AvailabilityError::ReasonOnAvailable { name });
        }
        if !available && reason.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            return Err(AvailabilityError::MissingReason { name });
        }
        Ok(Self {
            name,
            supported,
            available,
            reason,
        })
    }

    /// Capability / plugin name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the implementation exists in this tree.
    pub fn supported(&self) -> bool {
        self.supported
    }

    /// Whether this build can actually dispatch it.
    pub fn available(&self) -> bool {
        self.available
    }

    /// Why it is not available. Always `None` when [`Self::available`].
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// `supported && available`.
    pub fn effective(&self) -> bool {
        self.supported && self.available
    }
}

/// Honesty-invariant violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvailabilityError {
    /// `available` was true while `supported` was false.
    AvailableWithoutSupport {
        /// Row name.
        name: String,
    },
    /// `available` was true but a reason was supplied.
    ReasonOnAvailable {
        /// Row name.
        name: String,
    },
    /// `available` was false without a non-empty reason.
    MissingReason {
        /// Row name.
        name: String,
    },
}

impl std::fmt::Display for AvailabilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AvailableWithoutSupport { name } => {
                write!(
                    f,
                    "availability {name:?} cannot be available if unsupported"
                )
            }
            Self::ReasonOnAvailable { name } => {
                write!(
                    f,
                    "availability {name:?} is available so reason must be None"
                )
            }
            Self::MissingReason { name } => {
                write!(
                    f,
                    "availability {name:?} is unavailable so reason is required"
                )
            }
        }
    }
}

impl std::error::Error for AvailabilityError {}

/// A listing of honesty rows. Insertion order is preserved (panel-friendly).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AvailabilityReport {
    rows: Vec<Availability>,
}

impl AvailabilityReport {
    /// Empty listing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a row.
    pub fn push(&mut self, row: Availability) {
        self.rows.push(row);
    }

    /// Builder-style append.
    #[must_use]
    pub fn with(mut self, row: Availability) -> Self {
        self.push(row);
        self
    }

    /// Rows in insertion order.
    pub fn rows(&self) -> &[Availability] {
        &self.rows
    }

    /// Count of `effective()` rows.
    pub fn effective_count(&self) -> usize {
        self.rows.iter().filter(|r| r.effective()).count()
    }

    /// Total rows.
    pub fn total_count(&self) -> usize {
        self.rows.len()
    }

    /// Look up by name.
    pub fn get(&self, name: &str) -> Option<&Availability> {
        self.rows.iter().find(|r| r.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_is_effective_without_reason() {
        let a = Availability::live("tool.filesystem");
        assert!(a.supported());
        assert!(a.available());
        assert!(a.effective());
        assert!(a.reason().is_none());
    }

    #[test]
    fn blocked_is_supported_but_not_effective() {
        let a = Availability::blocked("vm_sandbox", "needs --features libkrun");
        assert!(a.supported());
        assert!(!a.available());
        assert!(!a.effective());
        assert_eq!(a.reason(), Some("needs --features libkrun"));
    }

    #[test]
    fn absent_is_neither() {
        let a = Availability::absent("marketplace", "out of scope");
        assert!(!a.supported());
        assert!(!a.available());
        assert!(!a.effective());
    }

    #[test]
    fn try_new_rejects_dishonest_rows() {
        assert!(matches!(
            Availability::try_new("x", false, true, None),
            Err(AvailabilityError::AvailableWithoutSupport { .. })
        ));
        assert!(matches!(
            Availability::try_new("x", true, true, Some("why".into())),
            Err(AvailabilityError::ReasonOnAvailable { .. })
        ));
        assert!(matches!(
            Availability::try_new("x", true, false, None),
            Err(AvailabilityError::MissingReason { .. })
        ));
        assert!(matches!(
            Availability::try_new("x", true, false, Some("".into())),
            Err(AvailabilityError::MissingReason { .. })
        ));
        assert!(Availability::try_new("x", true, true, None).is_ok());
        assert!(Availability::try_new("x", true, false, Some("no".into())).is_ok());
    }

    #[test]
    fn report_counts_and_lookup() {
        let report = AvailabilityReport::new()
            .with(Availability::live("a"))
            .with(Availability::blocked("b", "off"))
            .with(Availability::absent("c", "missing"));
        assert_eq!(report.total_count(), 3);
        assert_eq!(report.effective_count(), 1);
        assert!(report.get("b").is_some());
        assert!(report.get("nope").is_none());
    }
}
