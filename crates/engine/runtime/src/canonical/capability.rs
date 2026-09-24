//! Runtime-owned capability registry.
//!
//! Behaviors participate in the turn lifecycle. Capabilities are dispatchable
//! surfaces exposed to a model. Keeping these registries separate prevents a
//! tool-only provider from becoming an inert behavior module just to get into
//! the model tool list.

use std::sync::{Arc, RwLock};

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::NormalizedTool;

use super::module::reject_tool_identity_collisions;

/// poison 容错读锁 (L 组): 持锁线程 panic 只标记 poison, 注册表数据本身
/// 未损坏。直接 expect 会把一次 panic 放大为之后所有读写的级联 panic。
fn read_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// poison 容错写锁 (L 组): 同上。
fn write_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A source of dispatchable capabilities.
///
/// Providers own their concrete capability implementations; the runtime only
/// indexes the stable identities they expose.
pub trait CapabilityProvider: Send + Sync {
    /// Stable provider/source identity.
    fn id(&self) -> &str;

    /// Capabilities currently exposed by this provider.
    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>>;
}

struct RegisteredCapability {
    owner: String,
    capability: Arc<dyn ToolCapability>,
}

/// The dispatchable capability registry owned by one runtime instance.
///
/// Registration order is deterministic. Both capability ids and
/// model-facing names are unique, and lookup returns `None` on any live name
/// ambiguity so a collision can never silently become first-wins dispatch.
pub struct CapabilityRegistry {
    entries: RwLock<Vec<RegisteredCapability>>,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    /// Register all capabilities from one provider.
    pub fn register_provider(&self, provider: &dyn CapabilityProvider) -> Result<(), String> {
        self.register_all(provider.id(), provider.capabilities())
    }

    /// Register one capability under an explicit owner.
    pub fn register(
        &self,
        owner: impl Into<String>,
        capability: Arc<dyn ToolCapability>,
    ) -> Result<(), String> {
        self.register_all(owner, vec![capability])
    }

    pub(crate) fn register_all_for_builder(
        &self,
        capabilities: Vec<Arc<dyn ToolCapability>>,
    ) -> Result<(), String> {
        self.register_all("runtime", capabilities)
    }

    fn register_all(
        &self,
        owner: impl Into<String>,
        capabilities: Vec<Arc<dyn ToolCapability>>,
    ) -> Result<(), String> {
        let owner = owner.into();
        let mut entries = write_lock_or_recover(&self.entries);
        let existing = entries
            .iter()
            .map(|entry| Arc::clone(&entry.capability))
            .collect::<Vec<_>>();
        reject_tool_identity_collisions(&existing, &capabilities, &owner)?;
        entries.extend(
            capabilities
                .into_iter()
                .map(|capability| RegisteredCapability {
                    owner: owner.clone(),
                    capability,
                }),
        );
        Ok(())
    }

    /// Remove a dynamic capability by stable id.
    pub fn unregister(&self, capability_id: &apeireth_core::kernel::CapabilityId) -> bool {
        let mut entries = write_lock_or_recover(&self.entries);
        let before = entries.len();
        entries.retain(|entry| entry.capability.id() != capability_id);
        entries.len() != before
    }

    /// Current capabilities in deterministic registration order.
    pub fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        read_lock_or_recover(&self.entries)
            .iter()
            .map(|entry| Arc::clone(&entry.capability))
            .collect()
    }

    /// Current capabilities together with their source owner.
    pub fn entries(&self) -> Vec<(String, Arc<dyn ToolCapability>)> {
        read_lock_or_recover(&self.entries)
            .iter()
            .map(|entry| (entry.owner.clone(), Arc::clone(&entry.capability)))
            .collect()
    }

    /// Find one model-facing tool name, failing closed on ambiguity.
    pub fn find_by_name(&self, name: &str) -> Option<Arc<dyn ToolCapability>> {
        let mut found = None;
        for entry in read_lock_or_recover(&self.entries).iter() {
            if entry.capability.declaration().name == name {
                if found.is_some() {
                    return None;
                }
                found = Some(Arc::clone(&entry.capability));
            }
        }
        found
    }

    /// Model-facing declarations in registration order.
    pub fn declarations(&self) -> Vec<NormalizedTool> {
        self.capabilities()
            .iter()
            .map(|capability| capability.declaration())
            .collect()
    }

    /// Number of capabilities currently registered.
    pub fn len(&self) -> usize {
        read_lock_or_recover(&self.entries).len()
    }

    /// Whether no capabilities are registered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
