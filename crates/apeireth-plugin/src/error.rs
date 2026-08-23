//! Failures in the plugin and capability layer.

use apeireth_core::kernel::{CapabilityId, CoreError, PluginId};
use thiserror::Error;

/// Result alias for the plugin layer.
pub type PluginResult<T> = Result<T, PluginError>;

/// A failure registering, starting, stopping, or dispatching to a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PluginError {
    /// A primitive-layer failure, typically an illegal lifecycle transition.
    #[error(transparent)]
    Core(#[from] CoreError),

    /// Two plugins claim the same [`PluginId`].
    #[error("plugin {0} is already registered")]
    DuplicatePlugin(PluginId),

    /// Two plugins declare the same [`CapabilityId`].
    ///
    /// Rejected rather than resolved by precedence: silently letting the second
    /// registration win is how a system acquires a capability whose behaviour
    /// depends on registration order.
    #[error("capability {capability} is declared by both {incumbent} and {challenger}")]
    DuplicateCapability {
        /// The contested capability.
        capability: CapabilityId,
        /// The plugin that declared it first.
        incumbent: PluginId,
        /// The plugin that tried to declare it second.
        challenger: PluginId,
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

    /// A manifest declaration has no implementation on its owning plugin.
    #[error(
        "plugin {plugin} declares {kind} capability {capability} but provides no implementation"
    )]
    MissingCapabilityImplementation {
        /// Plugin whose manifest and implementation disagree.
        plugin: PluginId,
        /// Declared capability with no implementation.
        capability: CapabilityId,
        /// Canonical capability kind.
        kind: &'static str,
    },

    /// A plugin exposes an implementation its manifest does not declare.
    #[error("plugin {plugin} provides undeclared {kind} capability {capability}")]
    UndeclaredCapabilityImplementation {
        /// Plugin whose implementation is not declared.
        plugin: PluginId,
        /// Implementation missing from the manifest.
        capability: CapabilityId,
        /// Canonical capability kind.
        kind: &'static str,
    },

    /// A plugin declares a dependency that was never registered.
    #[error("plugin {dependent} depends on {missing}, which is not registered")]
    MissingDependency {
        /// The plugin with the unsatisfied dependency.
        dependent: PluginId,
        /// The dependency that is absent.
        missing: PluginId,
    },

    /// Plugin dependencies form a cycle, so no valid start order exists.
    #[error("plugin dependency cycle among: {0}")]
    DependencyCycle(String),

    /// No plugin is registered under this id.
    #[error("plugin {0} is not registered")]
    UnknownPlugin(PluginId),

    /// No capability is registered under this id.
    #[error("capability {0} is not registered")]
    UnknownCapability(CapabilityId),

    /// The capability exists but its owning plugin is not currently active.
    #[error("capability {capability} is not available: owning plugin {plugin} is {state}")]
    NotActive {
        /// The capability that was requested.
        capability: CapabilityId,
        /// The plugin that owns it.
        plugin: PluginId,
        /// The state that plugin is actually in.
        state: &'static str,
    },

    /// The plugin's own start-up or shutdown failed.
    #[error("plugin {plugin} failed during {phase}: {reason}")]
    PluginFailed {
        /// The failing plugin.
        plugin: PluginId,
        /// Which phase failed, `initialize` or `shutdown`.
        phase: &'static str,
        /// What the plugin reported.
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
}

impl PluginError {
    /// A plugin's `initialize` failed.
    pub fn init_failed(plugin: PluginId, reason: impl Into<String>) -> Self {
        Self::PluginFailed {
            plugin,
            phase: "initialize",
            reason: reason.into(),
        }
    }

    /// A plugin's `shutdown` failed.
    pub fn shutdown_failed(plugin: PluginId, reason: impl Into<String>) -> Self {
        Self::PluginFailed {
            plugin,
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
        let e = PluginError::DuplicateCapability {
            capability: CapabilityId::new("tool.shell").unwrap(),
            incumbent: PluginId::new("builtin.shell").unwrap(),
            challenger: PluginId::new("thirdparty.shell").unwrap(),
        };
        let msg = e.to_string();
        assert!(msg.contains("tool.shell"), "{msg}");
        assert!(msg.contains("builtin.shell"), "{msg}");
        assert!(msg.contains("thirdparty.shell"), "{msg}");
    }

    #[test]
    fn not_active_reports_the_state_that_blocked_dispatch() {
        let e = PluginError::NotActive {
            capability: CapabilityId::new("tool.shell").unwrap(),
            plugin: PluginId::new("builtin.shell").unwrap(),
            state: "registered",
        };
        assert!(e.to_string().contains("registered"), "{e}");
    }
}
