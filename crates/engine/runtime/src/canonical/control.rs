//! Internal control commands.
//!
//! A control command is a *runtime-owned state request*, not a capability
//! dispatch: it has no external side effect and grants no permission. It is
//! nevertheless registered, declared, and looked up through the one capability
//! registry so the kernel never matches a business tool name in `execute.rs`.
//!
//! # Boundaries
//!
//! - The descriptor ([`ControlCommand::declaration`]) and the implementation
//!   ([`ControlCommand::invoke`]) are separate items.
//! - A control name may not collide with a tool capability or plugin tool; the
//!   registry rejects the collision and the dispatch path fails closed if one
//!   appears later.
//! - Control commands still receive the mutable session only through
//!   [`ControlContext`], so they cannot reach the provider router, governance,
//!   or another turn's state.

use std::collections::BTreeMap;
use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, Clock, RequestId, TraceId};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};

use super::session::Session;

/// The runtime-owned context one control command may use.
///
/// The borrow is scoped to a single dispatch; a control command cannot retain
/// the session, spawn a second execution loop, or read another turn.
pub struct ControlContext<'a> {
    /// The session whose turn is dispatching the control.
    pub session: &'a mut Session,
    /// Request identity of the turn.
    pub request: RequestId,
    /// Trace identity of the turn.
    pub trace: TraceId,
    /// Runtime clock (journal timestamps come from here, never from `SystemTime`).
    pub clock: &'a dyn Clock,
}

impl<'a> ControlContext<'a> {
    /// Build the context for one control dispatch.
    pub(crate) fn new(
        session: &'a mut Session,
        request: RequestId,
        trace: TraceId,
        clock: &'a dyn Clock,
    ) -> Self {
        Self {
            session,
            request,
            trace,
            clock,
        }
    }
}

/// Reject duplicate control/tool identities before the runtime starts serving.
///
/// The registry rejects collisions between controls and builder-registered
/// tools; plugin tools only become visible after start-up, so this check runs
/// at the same boundary as the existing plugin-tool collision check. A name or
/// id may belong to exactly one surface.
pub fn reject_control_tool_collisions(
    controls: &[Arc<dyn ControlCommand>],
    tools: &[Arc<dyn ToolCapability>],
) -> Result<(), String> {
    let mut control_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut control_names: BTreeMap<String, String> = BTreeMap::new();
    for control in controls {
        let id = control.id().to_string();
        let name = control.declaration().name;
        control_ids.insert(id.clone(), name.clone());
        control_names.insert(name, id);
    }
    for tool in tools {
        let id = tool.id().to_string();
        let name = tool.declaration().name;
        if let Some(control_name) = control_ids.get(&id) {
            return Err(format!(
                "plugin tool id {id} collides with control {control_name:?}"
            ));
        }
        if let Some(control_id) = control_names.get(&name) {
            return Err(format!(
                "plugin tool name {name:?} collides with control {control_id}"
            ));
        }
    }
    Ok(())
}

/// A registered internal control command.
///
/// Implementations live with the mechanism they control (for example plan
/// mode); the execution loop only knows this trait.
pub trait ControlCommand: Send + Sync {
    /// Stable identity (namespaced, like a capability id).
    fn id(&self) -> &CapabilityId;

    /// The model-facing declaration that appears in the tool catalog.
    fn declaration(&self) -> NormalizedTool;

    /// Execute the state request and return the model-facing result.
    ///
    /// Returning a result is always successful bookkeeping: control commands
    /// cannot fail the turn, and they cannot widen permissions.
    fn invoke(&self, call: &ToolCall, ctx: &mut ControlContext<'_>) -> ToolResult;
}
