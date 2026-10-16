//! The pre-execute waterfall: policy hooks decide before anything runs.
//!
//! Hooks are consulted in registration order and each answers with exactly one
//! of [`PreVerdict`]: `allow` continues the waterfall, while `deny`, `cancel`,
//! and `ask` are decisive — the first non-`allow` verdict wins, the remaining
//! hooks are not consulted, and no later hook can flip the verdict. The order
//! is therefore irreversible in both directions: what ran before is recorded,
//! what comes after never re-decides.
//!
//! # Folding in the existing determinations
//!
//! Two adapters fold determinations that already exist at the tool boundary
//! into the waterfall **without changing what they judge**:
//!
//! * [`RiskLevelGateHook`] delegates the existing approval rule engine
//!   (risk / list / frequency rules, including its risk gate that asks for a
//!   human on high-risk calls) and maps its verdict one-to-one;
//! * [`CommandFamilyGateHook`] delegates the existing command-family boundary
//!   ([`ToolGuardrail::verify_shell_command`]) and carries its refusals
//!   verbatim.
//!
//! They are ordinary hooks: nothing is installed by default, so a pipeline
//! without them behaves exactly as before.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{extract_commands, ApprovalPolicyEngine, CallRecord, Decision};
use apeireth_protocol::canonical::ToolCall;

use crate::guardrail::ToolGuardrail;

use super::PipelineFailure;

/// What a policy hook decides for one call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreVerdict {
    /// Continue the waterfall.
    Allow,
    /// Do not run this call; the refusal is final for it.
    Deny {
        /// Why the hook refuses.
        reason: String,
    },
    /// Abort the call before execution (the caller is going away).
    Cancel {
        /// Why the call is cancelled.
        reason: String,
    },
    /// Do not run this call without a human decision.
    Ask {
        /// What a human is being asked to decide.
        reason: String,
    },
}

impl PreVerdict {
    /// Whether the waterfall may continue.
    pub const fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Stable label for records and traces.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny { .. } => "deny",
            Self::Cancel { .. } => "cancel",
            Self::Ask { .. } => "ask",
        }
    }
}

impl From<Decision> for PreVerdict {
    /// Map the canonical action verdict onto the waterfall vocabulary,
    /// one-to-one: `Allow` → allow, `Deny` → deny, `RequireApproval` → ask.
    /// The judgment semantics are untouched; only the vocabulary changes.
    fn from(decision: Decision) -> Self {
        match decision {
            Decision::Allow => Self::Allow,
            Decision::Deny { reason } => Self::Deny { reason },
            Decision::RequireApproval { reason } => Self::Ask { reason },
        }
    }
}

/// The facts a pre-execute hook may judge.
#[derive(Debug, Clone, Copy)]
pub struct PreExecuteRequest<'a> {
    /// The call about to be executed.
    pub call: &'a ToolCall,
    /// Stable capability identity of the tool that would run it.
    pub capability: &'a CapabilityId,
}

impl<'a> PreExecuteRequest<'a> {
    /// A request over one call and its capability identity.
    pub const fn new(call: &'a ToolCall, capability: &'a CapabilityId) -> Self {
        Self { call, capability }
    }
}

/// One policy hook in the pre-execute waterfall.
pub trait PreExecuteHook: Send + Sync {
    /// Stable name of the hook, used in verdicts and records.
    fn name(&self) -> &str;

    /// Judge one call. `Allow` lets the waterfall continue; anything else is
    /// decisive.
    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict;
}

/// The outcome of walking the waterfall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreDecision {
    /// The verdict that decided.
    pub verdict: PreVerdict,
    /// The hook that decided, absent when every hook allowed.
    pub decided_by: Option<String>,
    /// Hook names actually consulted, in consultation order.
    pub consulted: Vec<String>,
}

impl PreDecision {
    /// A one-line record of the decision.
    pub fn summary(&self) -> String {
        match &self.decided_by {
            Some(hook) => format!("{} by {}", self.verdict.label(), hook),
            None => self.verdict.label().to_string(),
        }
    }

    /// The closed failure this verdict maps to, absent for `allow`.
    pub fn failure(&self) -> Option<PipelineFailure> {
        let source = self.decided_by.clone().unwrap_or_default();
        match &self.verdict {
            PreVerdict::Allow => None,
            PreVerdict::Deny { reason } => Some(PipelineFailure::PreDenied {
                source,
                reason: reason.clone(),
            }),
            PreVerdict::Cancel { reason } => Some(PipelineFailure::PreCancelled {
                source,
                reason: reason.clone(),
            }),
            PreVerdict::Ask { reason } => Some(PipelineFailure::NeedsApproval {
                source,
                reason: reason.clone(),
            }),
        }
    }
}

/// The pre-execute waterfall: hooks in order, first non-allow verdict wins.
#[derive(Default)]
pub struct PreExecuteWaterfall {
    hooks: Vec<Arc<dyn PreExecuteHook>>,
}

impl PreExecuteWaterfall {
    /// An empty waterfall: everything is allowed through.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one hook. Consultation order equals insertion order.
    #[must_use]
    pub fn with(mut self, hook: Arc<dyn PreExecuteHook>) -> Self {
        self.hooks.push(hook);
        self
    }

    /// Number of hooks.
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// Whether no hook is listening.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Hook names in consultation order.
    pub fn names(&self) -> Vec<&str> {
        self.hooks.iter().map(|hook| hook.name()).collect()
    }

    /// Walk the waterfall in order and stop at the first non-allow verdict.
    ///
    /// The verdict is recorded together with the exact consultation order, so
    /// "hook two denied and hook three never ran" is observable rather than
    /// implied.
    pub fn evaluate(&self, request: &PreExecuteRequest<'_>) -> PreDecision {
        let mut consulted = Vec::new();
        for hook in &self.hooks {
            consulted.push(hook.name().to_string());
            let verdict = hook.pre_verdict(request);
            if !verdict.is_allowed() {
                return PreDecision {
                    verdict,
                    decided_by: Some(hook.name().to_string()),
                    consulted,
                };
            }
        }
        PreDecision {
            verdict: PreVerdict::Allow,
            decided_by: None,
            consulted,
        }
    }
}

/// Folds the existing risk-level gate into the pre-execute waterfall.
///
/// The adapter owns an [`ApprovalPolicyEngine`] and delegates it verbatim on
/// every call, then maps its verdict one-to-one onto [`PreVerdict`] (see
/// [`From<Decision>`]): the engine's risk gate still asks for a human on
/// high-risk calls, its blacklist and frequency rules still deny, its trust
/// and whitelist rules still allow. Nothing about the judgment changes — only
/// where it is consulted.
pub struct RiskLevelGateHook {
    engine: ApprovalPolicyEngine,
    history: Mutex<Vec<CallRecord>>,
    now: fn() -> i64,
}

impl RiskLevelGateHook {
    /// A hook over one rule engine, with an empty call history.
    pub fn new(engine: ApprovalPolicyEngine) -> Self {
        Self {
            engine,
            history: Mutex::new(Vec::new()),
            now: default_clock,
        }
    }

    /// Use `clock` as the evaluation timestamp source (deterministic tests,
    /// simulated time).
    #[must_use]
    pub fn with_clock(mut self, clock: fn() -> i64) -> Self {
        self.now = clock;
        self
    }
}

impl PreExecuteHook for RiskLevelGateHook {
    fn name(&self) -> &str {
        "risk_level_gate"
    }

    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict {
        let now_ms = (self.now)();
        let mut history = self.history.lock().unwrap();
        let (decision, _detail) = self.engine.evaluate(
            request.capability.as_str(),
            &request.call.arguments,
            &history,
            now_ms,
        );
        history.push(CallRecord::new(
            request.capability.as_str().to_string(),
            now_ms,
        ));
        decision.into()
    }
}

fn default_clock() -> i64 {
    0
}

/// Folds the existing command-family boundary into the pre-execute waterfall.
///
/// Commands are read out of the call arguments by the existing extraction
/// helper and each is judged by the existing pre-call command check; its
/// refusals are carried verbatim, so the set of refused commands is exactly
/// the set refused before this waterfall existed.
pub struct CommandFamilyGateHook;

impl CommandFamilyGateHook {
    /// A hook over the static command-family boundary.
    pub fn new() -> Self {
        Self
    }
}

impl Default for CommandFamilyGateHook {
    fn default() -> Self {
        Self::new()
    }
}

impl PreExecuteHook for CommandFamilyGateHook {
    fn name(&self) -> &str {
        "command_family_gate"
    }

    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict {
        for command in extract_commands(&request.call.arguments) {
            if let Err(refusal) = ToolGuardrail::verify_shell_command(&command) {
                return PreVerdict::Deny {
                    reason: refusal.to_string(),
                };
            }
        }
        PreVerdict::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedHook {
        name: &'static str,
        verdict: PreVerdict,
        log: Arc<Mutex<Vec<&'static str>>>,
    }

    impl FixedHook {
        fn new(
            name: &'static str,
            verdict: PreVerdict,
            log: &Arc<Mutex<Vec<&'static str>>>,
        ) -> Arc<Self> {
            Arc::new(Self {
                name,
                verdict,
                log: Arc::clone(log),
            })
        }
    }

    impl PreExecuteHook for FixedHook {
        fn name(&self) -> &str {
            self.name
        }

        fn pre_verdict(&self, _request: &PreExecuteRequest<'_>) -> PreVerdict {
            self.log.lock().unwrap().push(self.name);
            self.verdict.clone()
        }
    }

    fn call() -> ToolCall {
        ToolCall {
            id: "call_1".into(),
            name: "demo".into(),
            arguments: serde_json::json!({}),
        }
    }

    #[test]
    fn the_first_non_allow_verdict_wins_and_stops_the_walk() {
        let capability = CapabilityId::new("tool.demo").unwrap();
        let call = call();
        let log = Arc::new(Mutex::new(Vec::new()));
        let waterfall = PreExecuteWaterfall::new()
            .with(FixedHook::new("one", PreVerdict::Allow, &log))
            .with(FixedHook::new(
                "two",
                PreVerdict::Deny {
                    reason: "no".into(),
                },
                &log,
            ))
            .with(FixedHook::new("three", PreVerdict::Allow, &log));

        let decision = waterfall.evaluate(&PreExecuteRequest::new(&call, &capability));
        assert_eq!(decision.verdict.label(), "deny");
        assert_eq!(decision.decided_by.as_deref(), Some("two"));
        assert_eq!(decision.consulted, ["one", "two"]);
        assert_eq!(
            *log.lock().unwrap(),
            ["one", "two"],
            "the refusal stops the walk; the third hook never runs"
        );
    }

    #[test]
    fn an_allow_never_flips_an_earlier_refusal() {
        let capability = CapabilityId::new("tool.demo").unwrap();
        let call = call();
        let log = Arc::new(Mutex::new(Vec::new()));
        let waterfall = PreExecuteWaterfall::new()
            .with(FixedHook::new(
                "refusing",
                PreVerdict::Ask {
                    reason: "human".into(),
                },
                &log,
            ))
            .with(FixedHook::new("late-allow", PreVerdict::Allow, &log));
        let decision = waterfall.evaluate(&PreExecuteRequest::new(&call, &capability));
        assert_eq!(decision.verdict.label(), "ask");
        assert_eq!(decision.consulted, ["refusing"]);
        assert_eq!(
            decision.failure(),
            Some(PipelineFailure::NeedsApproval {
                source: "refusing".into(),
                reason: "human".into()
            })
        );
    }

    #[test]
    fn risk_level_gate_maps_engine_verdicts_one_to_one() {
        let capability = CapabilityId::new("system.exec").unwrap();
        let call = call();
        let hook = RiskLevelGateHook::new(ApprovalPolicyEngine::new());
        assert_eq!(
            hook.pre_verdict(&PreExecuteRequest::new(&call, &capability)),
            PreVerdict::Ask {
                reason: "high-risk capability system.exec requires human approval".into()
            },
            "the risk gate's judgment must pass through unchanged"
        );
    }

    #[test]
    fn command_family_gate_carries_refusals_verbatim() {
        let capability = CapabilityId::new("tool.shell").unwrap();
        let mut call = call();
        call.arguments = serde_json::json!({ "command": "rm -rf / --no-preserve-root" });
        let hook = CommandFamilyGateHook::new();
        let verdict = hook.pre_verdict(&PreExecuteRequest::new(&call, &capability));
        match verdict {
            PreVerdict::Deny { reason } => assert!(reason.contains("rm -rf /"), "{reason}"),
            other => panic!("expected deny, got {other:?}"),
        }
    }
}
