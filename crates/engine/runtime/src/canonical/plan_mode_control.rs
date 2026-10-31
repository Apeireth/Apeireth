//! The resident plan-mode exit control, registered like any other surface.
//!
//! The control is a runtime-owned state request, not a capability dispatch: it
//! parks a posture switch that lands at the pre-step of the next accepted turn.
//! Registering it here keeps the plan-mode mechanism's name, descriptor, and
//! implementation together, and keeps `execute.rs` free of business-name
//! branches.

use apeireth_core::kernel::{CapabilityId, Clock, RequestId, TraceId};
use apeireth_orchestration::plan_mode::{PlanMode, PlanModeLedger, PlanModeRequestOutcome};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};

use super::control::{ControlCommand, ControlContext};
use super::session::{Session, SessionEventKind};

/// The resident plan-mode exit control's model-facing name.
///
/// Resident in *both* plan-mode postures whenever the mechanism is enabled, so
/// a posture switch alters the prompt projection and never the tool catalog.
pub const PLAN_MODE_EXIT_TOOL_NAME: &str = "exit_plan_mode";

/// The resident `exit_plan_mode` tool declaration.
///
/// The control is answered by the runtime itself: it parks a posture switch
/// that lands at the pre-step of the next accepted turn. It dispatches no
/// capability and grants nothing.
pub fn plan_mode_exit_declaration() -> NormalizedTool {
    let mut parameters = ToolParameters::new();
    parameters.insert(
        "plan".to_string(),
        serde_json::json!({
            "type": "string",
            "description": "确认后的计划全文, 随本次工具调用留在转写里, 仅作记录。",
        }),
    );
    NormalizedTool::new(PLAN_MODE_EXIT_TOOL_NAME)
        .with_description(
            "结束计划阶段并进入执行阶段。切换在下一被接受回合开始处落账生效, 本回合的行为约定投影不变; \
             本工具始终在场, 不改变可用工具清单与审批判定。",
        )
        .with_parameters(parameters)
}

/// Park a plan-mode switch in the session ledger.
///
/// The request is journaled immediately as intent and posts at the pre-step of
/// the next accepted turn — never inside the running turn that asked for it.
/// Shared by the model-facing control and the host-facing
/// [`Runtime::request_plan_mode_switch`](super::runtime::Runtime::request_plan_mode_switch)
/// so both journal through exactly one implementation.
pub(crate) fn queue_plan_mode_switch(
    session: &mut Session,
    target: PlanMode,
    request_id: RequestId,
    trace_id: TraceId,
    clock: &dyn Clock,
) -> PlanModeRequestOutcome {
    let mut ledger = PlanModeLedger::from_events(session.plan_mode_log());
    let mark = ledger.events().len();
    let outcome = ledger.request_switch(target);
    record_plan_mode_entries(session, &ledger, mark, request_id, trace_id, clock);
    outcome
}

/// Journal the ledger entries appended since `from` as session events.
pub(crate) fn record_plan_mode_entries(
    session: &mut Session,
    ledger: &PlanModeLedger,
    from: usize,
    request_id: RequestId,
    trace_id: TraceId,
    clock: &dyn Clock,
) {
    for entry in ledger.events().iter().skip(from) {
        session.record(
            request_id,
            trace_id,
            SessionEventKind::PlanMode {
                entry: entry.clone(),
            },
            clock,
        );
    }
}

/// The registered `exit_plan_mode` control.
pub struct PlanModeExitControl {
    id: CapabilityId,
}

impl PlanModeExitControl {
    /// Build the control with its stable capability identity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: CapabilityId::new("control.exit_plan_mode")
                .expect("the plan-mode control id is a valid capability id"),
        }
    }
}

impl Default for PlanModeExitControl {
    fn default() -> Self {
        Self::new()
    }
}

impl ControlCommand for PlanModeExitControl {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        plan_mode_exit_declaration()
    }

    fn invoke(&self, call: &ToolCall, ctx: &mut ControlContext<'_>) -> ToolResult {
        let outcome = queue_plan_mode_switch(
            ctx.session,
            PlanMode::Executing,
            ctx.request,
            ctx.trace,
            ctx.clock,
        );
        let (queued, note) = match outcome {
            PlanModeRequestOutcome::Queued => (
                true,
                "计划模式切换已排队: 在下一被接受回合开始处落账生效; 本回合的行为约定投影不变",
            ),
            PlanModeRequestOutcome::AlreadyQueued => {
                (true, "同样的切换已在排队中, 等待下一被接受回合落账")
            }
            PlanModeRequestOutcome::NoChange => (false, "当前不在计划模式, 无须切换, 未记账"),
        };
        ToolResult::ok(
            &call.id,
            serde_json::json!({
                "queued": queued,
                "target_mode": "executing",
                "note": note,
            }),
        )
        .with_name(&call.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{RequestId, SessionId, TraceId};
    use apeireth_protocol::canonical::ToolCall;

    fn control() -> PlanModeExitControl {
        PlanModeExitControl::new()
    }

    #[test]
    fn the_control_declares_the_resident_name() {
        let declaration = control().declaration();
        assert_eq!(declaration.name, PLAN_MODE_EXIT_TOOL_NAME);
        assert!(declaration.parameters.contains_key("plan"));
    }

    #[test]
    fn the_control_identity_is_namespaced() {
        assert_eq!(control().id().as_str(), "control.exit_plan_mode");
    }

    #[test]
    fn the_control_result_keeps_the_call_identity() {
        let clock = apeireth_core::kernel::system_clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        let call = ToolCall {
            id: "call_1".to_string(),
            name: PLAN_MODE_EXIT_TOOL_NAME.to_string(),
            arguments: serde_json::json!({}),
        };
        let mut ctx = ControlContext::new(
            &mut session,
            RequestId::new(),
            TraceId::new(),
            clock.as_ref(),
        );
        let result = control().invoke(&call, &mut ctx);
        assert_eq!(result.tool_call_id, "call_1");
        assert_eq!(result.name.as_deref(), Some(PLAN_MODE_EXIT_TOOL_NAME));
    }
}
