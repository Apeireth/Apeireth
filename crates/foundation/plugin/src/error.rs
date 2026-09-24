//! Failures in the module and capability layer.
//!
//! The type is named `ModuleSystemError` because the runtime entity is a
//! module; the historical spelling [`PluginError`] and [`PluginResult`] survive
//! as aliases so embedders compile unchanged until the follow-up rename.

use apeireth_core::kernel::{CapabilityId, CoreError, ModuleId};
use thiserror::Error;

/// Result alias for the module system layer.
pub type ModuleResult<T> = Result<T, ModuleSystemError>;

/// Compatibility alias for [`ModuleSystemError`].
pub type PluginResult<T> = ModuleResult<T>;

/// Compatibility alias for [`ModuleSystemError`].
pub use self::ModuleSystemError as PluginError;

/// A failure registering, starting, stopping, or dispatching to a module.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ModuleSystemError {
    /// A primitive-layer failure, typically an illegal lifecycle transition.
    #[error(transparent)]
    Core(#[from] CoreError),

    /// Two modules claim the same [`ModuleId`].
    #[error("module {0} is already registered")]
    DuplicateModule(ModuleId),

    /// Two modules declare the same [`CapabilityId`].
    ///
    /// Rejected rather than resolved by precedence: silently letting the second
    /// registration win is how a system acquires a capability whose behaviour
    /// depends on registration order.
    #[error("capability {capability} is declared by both {incumbent} and {challenger}")]
    DuplicateCapability {
        /// The contested capability.
        capability: CapabilityId,
        /// The module that declared it first.
        incumbent: ModuleId,
        /// The module that tried to declare it second.
        challenger: ModuleId,
    },

    /// A capability id does not match the kind it was registered under, e.g.
    /// `provider.foo` declared as a tool.
    #[error("capability {capability} has kind prefix {actual:?} but was declared as {expected:?}")]
    KindMismatch {
        /// The offending capability.
        capability: CapabilityId,
        /// The prefix the id actually carries.
        actual: String,
        /// The prefix its declared kind requires.
        expected: &'static str,
    },

    /// A manifest declaration has no implementation on its owning module.
    #[error(
        "module {module} declares {kind} capability {capability} but provides no implementation"
    )]
    MissingCapabilityImplementation {
        /// Module whose manifest and implementation disagree.
        module: ModuleId,
        /// Declared capability with no implementation.
        capability: CapabilityId,
        /// Canonical capability kind.
        kind: &'static str,
    },

    /// A module exposes an implementation its manifest does not declare.
    #[error("module {module} provides undeclared {kind} capability {capability}")]
    UndeclaredCapabilityImplementation {
        /// Module whose implementation is not declared.
        module: ModuleId,
        /// Implementation missing from the manifest.
        capability: CapabilityId,
        /// Canonical capability kind.
        kind: &'static str,
    },

    /// Two distinct tool capabilities expose the same model-facing name.
    #[error("model-facing tool name {name:?} is provided by both {incumbent} and {challenger}")]
    AmbiguousToolName {
        /// Name a model would emit in a tool call.
        name: String,
        /// Capability registered first.
        incumbent: CapabilityId,
        /// Capability whose registration was rejected.
        challenger: CapabilityId,
    },

    /// A module declares a dependency that was never registered.
    #[error("module {dependent} depends on {missing}, which is not registered")]
    MissingDependency {
        /// The module with the unsatisfied dependency.
        dependent: ModuleId,
        /// The dependency that is absent.
        missing: ModuleId,
    },

    /// Module dependencies form a cycle, so no valid start order exists.
    #[error("module dependency cycle among: {0}")]
    DependencyCycle(String),

    /// No module is registered under this id.
    #[error("module {0} is not registered")]
    UnknownModule(ModuleId),

    /// No capability is registered under this id.
    #[error("capability {0} is not registered")]
    UnknownCapability(CapabilityId),

    /// The capability exists but its owning module is not currently active.
    #[error("capability {capability} is not available: owning module {module} is {state}")]
    NotActive {
        /// The capability that was requested.
        capability: CapabilityId,
        /// The module that owns it.
        module: ModuleId,
        /// The state that module is actually in.
        state: &'static str,
    },

    /// The module's own start-up or shutdown failed.
    #[error("module {module} failed during {phase}: {reason}")]
    ModuleFailed {
        /// The failing module.
        module: ModuleId,
        /// Which phase failed, `initialize` or `shutdown`.
        phase: &'static str,
        /// What the module reported.
        reason: String,
    },

    /// A capability rejected its input.
    #[error("capability {capability} rejected its arguments: {reason}")]
    InvalidArguments {
        /// The capability that rejected them.
        capability: CapabilityId,
        /// Why.
        reason: String,
    },

    /// A module was asked to take a dynamic capability it does not own.
    ///
    /// Only modules with genuine dynamic ownership (an MCP server discovering
    /// tools at runtime) may accept tools after registration. Every other
    /// module reports this, and the manager validates global uniqueness first,
    /// so a static module never sees the request at all.
    #[error("module {module} does not accept dynamic capabilities")]
    DynamicCapabilityUnsupported {
        /// The module that was asked.
        module: ModuleId,
    },
}

impl ModuleSystemError {
    /// A module's `initialize` failed.
    pub fn init_failed(module: ModuleId, reason: impl Into<String>) -> Self {
        Self::ModuleFailed {
            module,
            phase: "initialize",
            reason: reason.into(),
        }
    }

    /// A module's `shutdown` failed.
    pub fn shutdown_failed(module: ModuleId, reason: impl Into<String>) -> Self {
        Self::ModuleFailed {
            module,
            phase: "shutdown",
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_capability_names_both_claimants() {
        let e = ModuleSystemError::DuplicateCapability {
            capability: CapabilityId::new("tool.shell").unwrap(),
            incumbent: ModuleId::new("builtin.shell").unwrap(),
            challenger: ModuleId::new("thirdparty.shell").unwrap(),
        };
        let msg = e.to_string();
        assert!(msg.contains("tool.shell"), "{msg}");
        assert!(msg.contains("builtin.shell"), "{msg}");
        assert!(msg.contains("thirdparty.shell"), "{msg}");
    }

    #[test]
    fn not_active_reports_the_state_that_blocked_dispatch() {
        let e = ModuleSystemError::NotActive {
            capability: CapabilityId::new("tool.shell").unwrap(),
            module: ModuleId::new("builtin.shell").unwrap(),
            state: "registered",
        };
        assert!(e.to_string().contains("registered"), "{e}");
    }

    #[test]
    fn the_historical_spellings_resolve_to_the_same_types() {
        let via_old: PluginResult<()> = Ok(());
        assert!(via_old.is_ok());
    }
}
