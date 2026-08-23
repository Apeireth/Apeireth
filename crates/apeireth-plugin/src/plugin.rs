//! The plugin contract.
//!
//! A plugin is a **capability provider**, not a tool. `Plugin == Tool` is the
//! assumption that produces a `ToolRegistry`, then an `McpRegistry` when tools
//! start arriving over a transport, then a `SkillRegistry` when some of them are
//! prompts, and finally a system where nothing can answer "what can this runtime
//! do" without consulting five places.
//!
//! One plugin may provide any number of capabilities of any kinds: a plugin that
//! contributes both `provider.acme` and `tool.acme_search` is a single unit of
//! lifecycle with two entries in the capability registry.

use std::sync::Arc;

use apeireth_core::kernel::{Clock, TraceId};
use async_trait::async_trait;

use crate::credentials::CredentialResolver;
use crate::error::PluginResult;
use crate::manifest::PluginManifest;
use crate::provider::ProviderCapability;
use crate::tool::ToolCapability;

/// What a plugin is given when it starts.
///
/// Everything a plugin needs from the outside world arrives here, which is what
/// keeps plugins from reaching for globals, ambient environment, or fixed paths.
pub struct PluginContext {
    /// Time source. Plugins must read time from this, never from `Utc::now()`,
    /// so that a virtual clock makes their behaviour reproducible.
    pub clock: Arc<dyn Clock>,
    /// Secret lookup. A plugin needing an API key resolves it here by logical
    /// name; it never reads a file path or an environment variable directly.
    pub credentials: Arc<dyn CredentialResolver>,
    /// The trace covering this start-up, so a plugin's own events correlate with
    /// the boot that caused them.
    pub trace: TraceId,
}

impl PluginContext {
    /// Assemble a context.
    pub fn new(
        clock: Arc<dyn Clock>,
        credentials: Arc<dyn CredentialResolver>,
        trace: TraceId,
    ) -> Self {
        Self {
            clock,
            credentials,
            trace,
        }
    }
}

impl std::fmt::Debug for PluginContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Neither the clock nor the resolver is Debug, and printing the resolver
        // would be a poor idea even if it were.
        f.debug_struct("PluginContext")
            .field("trace", &self.trace)
            .finish_non_exhaustive()
    }
}

/// A unit of lifecycle that provides capabilities.
///
/// This phase supports **static, in-process plugins only**. Dynamic library
/// loading, WASM, hot reload, remote plugins, and a marketplace are explicitly
/// out of scope; the contract is shaped so they remain possible later, and no
/// part of it is designed around them now.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// What this plugin declares about itself. Must be stable across calls.
    fn manifest(&self) -> &PluginManifest;

    /// Acquire resources and become ready to serve.
    ///
    /// Called at most once, after every declared dependency is active. Returning
    /// an error moves this plugin to `Failed` and aborts the boot.
    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()>;

    /// Release resources.
    ///
    /// Called in reverse dependency order. Errors are reported but do not stop
    /// the remaining plugins from shutting down: a shutdown that abandons half
    /// its plugins because one misbehaved leaks more than it protects.
    async fn shutdown(&self) -> PluginResult<()>;

    /// Tool capabilities this plugin provides. Must match the manifest.
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        Vec::new()
    }

    /// Provider capabilities this plugin provides. Must match the manifest.
    fn providers(&self) -> Vec<Arc<dyn ProviderCapability>> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{system_clock, CapabilityId, PluginId};

    use crate::capability::CapabilityKind;
    use crate::credentials::{NoCredentials, StaticCredentials};

    struct Inert {
        manifest: PluginManifest,
    }

    #[async_trait]
    impl Plugin for Inert {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }
        async fn initialize(&self, _ctx: &PluginContext) -> PluginResult<()> {
            Ok(())
        }
        async fn shutdown(&self) -> PluginResult<()> {
            Ok(())
        }
    }

    fn context() -> PluginContext {
        PluginContext::new(system_clock(), Arc::new(NoCredentials), TraceId::new())
    }

    #[tokio::test]
    async fn a_plugin_that_provides_nothing_is_still_valid() {
        let p = Inert {
            manifest: PluginManifest::new(PluginId::new("builtin.inert").unwrap(), "1.0.0", "none"),
        };
        assert!(p.initialize(&context()).await.is_ok());
        assert!(p.tools().is_empty());
        assert!(p.providers().is_empty());
        assert!(p.shutdown().await.is_ok());
    }

    #[test]
    fn one_plugin_may_declare_capabilities_of_several_kinds() {
        let manifest = PluginManifest::new(PluginId::new("vendor.acme").unwrap(), "1.0.0", "Acme")
            .declare_capability(
                CapabilityId::new("provider.acme").unwrap(),
                CapabilityKind::Provider,
                "Acme completions",
            )
            .unwrap()
            .declare_capability(
                CapabilityId::new("tool.acme_search").unwrap(),
                CapabilityKind::Tool,
                "Acme search",
            )
            .unwrap();

        assert_eq!(manifest.capabilities.len(), 2);
        assert_eq!(
            manifest
                .capabilities_of_kind(CapabilityKind::Provider)
                .count(),
            1
        );
        assert_eq!(
            manifest.capabilities_of_kind(CapabilityKind::Tool).count(),
            1
        );
    }

    #[test]
    fn a_context_does_not_print_its_credentials() {
        let ctx = PluginContext::new(
            system_clock(),
            Arc::new(StaticCredentials::new().with("provider.acme.api_key", "sk-secret")),
            TraceId::new(),
        );
        let printed = format!("{ctx:?}");
        assert!(!printed.contains("sk-secret"), "{printed}");
    }
}
