//! Runtime-owned capability registry.
//!
//! Behaviors participate in the turn lifecycle. Capabilities are dispatchable
//! surfaces exposed to a model. Keeping these registries separate prevents a
//! tool-only provider from becoming an inert behavior module just to get into
//! the model tool list.

use std::sync::{Arc, RwLock};

use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::NormalizedTool;

use super::control::ControlCommand;
use super::module::reject_tool_identity_collisions;

/// poison 容错读锁 (L 组): 持锁线程 panic 只标记 poison, 注册表数据本身
/// 未损坏。直接 expect 会把一次 panic 放大为之后所有读写的级联 panic。
fn read_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// poison 容错写锁 (L 组): 同上。
fn write_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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
    /// Runtime-owned internal control commands. They share the one registry so
    /// the dispatch loop has a single lookup surface and one identity space.
    controls: RwLock<Vec<Arc<dyn ControlCommand>>>,
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
            controls: RwLock::new(Vec::new()),
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
        // Lock order is always entries -> controls (see `register_control`).
        let mut entries = write_lock_or_recover(&self.entries);
        let controls = read_lock_or_recover(&self.controls);
        let existing = entries
            .iter()
            .map(|entry| Arc::clone(&entry.capability))
            .collect::<Vec<_>>();
        reject_tool_identity_collisions(&existing, &capabilities, &owner)?;
        for capability in &capabilities {
            for control in controls.iter() {
                if control.id() == capability.id() {
                    return Err(format!(
                        "duplicate capability id {} from {owner} collides with control {}",
                        capability.id(),
                        control.declaration().name
                    ));
                }
                if control.declaration().name == capability.declaration().name {
                    return Err(format!(
                        "duplicate tool name {:?} from {owner} collides with control {}",
                        capability.declaration().name,
                        control.id()
                    ));
                }
            }
        }
        drop(controls);
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

    /// Register one runtime-owned control command.
    ///
    /// Rejects empty, duplicate, or cross-kind identities: a control name may
    /// never silently shadow a tool (or the reverse).
    pub fn register_control(&self, control: Arc<dyn ControlCommand>) -> Result<(), String> {
        let id = control.id().to_string();
        let name = control.declaration().name;
        if id.trim().is_empty() || name.trim().is_empty() {
            return Err("control commands need a non-empty id and name".to_string());
        }
        // Lock order is always entries -> controls.
        let entries = read_lock_or_recover(&self.entries);
        let mut controls = write_lock_or_recover(&self.controls);
        for existing in controls.iter() {
            if existing.id() == control.id() {
                return Err(format!("duplicate control id {id}"));
            }
            if existing.declaration().name == name {
                return Err(format!("duplicate control name {name:?}"));
            }
        }
        for entry in entries.iter() {
            if entry.capability.id() == control.id() {
                return Err(format!(
                    "control id {id} collides with tool capability {:?}",
                    entry.capability.declaration().name
                ));
            }
            if entry.capability.declaration().name == name {
                return Err(format!(
                    "control name {name:?} collides with tool capability {}",
                    entry.capability.id()
                ));
            }
        }
        controls.push(control);
        Ok(())
    }

    /// Remove a control by stable id.
    pub fn unregister_control(&self, control_id: &apeireth_core::kernel::CapabilityId) -> bool {
        let mut controls = write_lock_or_recover(&self.controls);
        let before = controls.len();
        controls.retain(|control| control.id() != control_id);
        controls.len() != before
    }

    /// Find one control by model-facing name, failing closed on ambiguity.
    pub fn find_control_by_name(&self, name: &str) -> Option<Arc<dyn ControlCommand>> {
        let mut found = None;
        for control in read_lock_or_recover(&self.controls).iter() {
            if control.declaration().name == name {
                if found.is_some() {
                    return None;
                }
                found = Some(Arc::clone(control));
            }
        }
        found
    }

    /// Model-facing control declarations in registration order.
    pub fn control_declarations(&self) -> Vec<NormalizedTool> {
        read_lock_or_recover(&self.controls)
            .iter()
            .map(|control| control.declaration())
            .collect()
    }

    /// Number of registered control commands.
    pub fn control_len(&self) -> usize {
        read_lock_or_recover(&self.controls).len()
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

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{CapabilityId, Clock, RequestId, SessionId, TraceId};
    use apeireth_protocol::canonical::{ToolCall, ToolParameters, ToolResult};
    use async_trait::async_trait;

    use super::super::control::ControlContext;
    use super::super::session::Session;

    struct DummyTool {
        id: CapabilityId,
        name: &'static str,
    }

    impl DummyTool {
        fn new(name: &'static str) -> Arc<dyn ToolCapability> {
            Arc::new(Self {
                id: CapabilityId::new(format!("tool.{name}")).expect("capability id"),
                name,
            })
        }
    }

    #[async_trait]
    impl ToolCapability for DummyTool {
        fn id(&self) -> &CapabilityId {
            &self.id
        }

        fn declaration(&self) -> NormalizedTool {
            NormalizedTool::new(self.name).with_parameters(ToolParameters::new())
        }

        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            ToolResult::ok(&call.id, serde_json::json!("ok"))
        }
    }

    struct DummyControl {
        id: CapabilityId,
        name: &'static str,
    }

    impl DummyControl {
        fn new(name: &'static str) -> Arc<dyn ControlCommand> {
            Arc::new(Self {
                id: CapabilityId::new(format!("control.{name}")).expect("capability id"),
                name,
            })
        }
    }

    impl ControlCommand for DummyControl {
        fn id(&self) -> &CapabilityId {
            &self.id
        }

        fn declaration(&self) -> NormalizedTool {
            NormalizedTool::new(self.name).with_parameters(ToolParameters::new())
        }

        fn invoke(&self, call: &ToolCall, ctx: &mut ControlContext<'_>) -> ToolResult {
            let _ = (ctx.session.id, ctx.request, ctx.trace);
            ToolResult::ok(&call.id, serde_json::json!("control"))
        }
    }

    #[test]
    fn a_registered_control_is_declared_and_found_by_name() {
        let registry = CapabilityRegistry::new();
        registry
            .register_control(DummyControl::new("exit_plan_mode"))
            .expect("control registers");

        assert_eq!(registry.control_len(), 1);
        assert_eq!(
            registry
                .control_declarations()
                .iter()
                .map(|declaration| declaration.name.as_str())
                .collect::<Vec<_>>(),
            vec!["exit_plan_mode"]
        );
        assert!(registry.find_control_by_name("exit_plan_mode").is_some());
        assert!(registry.find_control_by_name("absent").is_none());
    }

    #[test]
    fn duplicate_control_names_are_rejected() {
        let registry = CapabilityRegistry::new();
        registry
            .register_control(DummyControl::new("exit_plan_mode"))
            .expect("first control registers");
        let duplicate = Arc::new(DummyControl {
            id: CapabilityId::new("control.other").expect("capability id"),
            name: "exit_plan_mode",
        });
        let error = registry
            .register_control(duplicate)
            .expect_err("duplicate name rejected");
        assert!(error.contains("duplicate control name"));
    }

    #[test]
    fn a_tool_cannot_shadow_a_control_name() {
        let registry = CapabilityRegistry::new();
        registry
            .register_control(DummyControl::new("exit_plan_mode"))
            .expect("control registers");
        let error = registry
            .register_all_for_builder(vec![DummyTool::new("exit_plan_mode")])
            .expect_err("tool shadowing a control is rejected");
        assert!(error.contains("collides with control"));
    }

    #[test]
    fn a_control_cannot_shadow_a_tool_name() {
        let registry = CapabilityRegistry::new();
        registry
            .register_all_for_builder(vec![DummyTool::new("echo")])
            .expect("tool registers");
        let error = registry
            .register_control(DummyControl::new("echo"))
            .expect_err("control shadowing a tool is rejected");
        assert!(error.contains("collides with tool capability"));
    }

    #[test]
    fn unregister_control_removes_only_the_named_control() {
        let registry = CapabilityRegistry::new();
        registry
            .register_control(DummyControl::new("first"))
            .expect("first control registers");
        registry
            .register_control(DummyControl::new("second"))
            .expect("second control registers");
        let first = CapabilityId::new("control.first").expect("capability id");

        assert!(registry.unregister_control(&first));
        assert!(registry.find_control_by_name("first").is_none());
        assert!(registry.find_control_by_name("second").is_some());
        assert_eq!(registry.control_len(), 1);
    }

    /// Compile-time proof that the test clock type satisfies the context port.
    #[allow(dead_code)]
    fn _control_context_accepts_a_clock(session: &mut Session, clock: &dyn Clock) {
        let _ = ControlContext::new(session, RequestId::new(), TraceId::new(), clock);
        let _ = SessionId::new();
    }
}
