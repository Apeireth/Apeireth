//! Plan mode as a collaborative state projection, not an enforcement layer.
//!
//! Two postures alternate inside one conversation — planning and executing —
//! and mixing them mid-turn is what produces mode flapping. This module makes
//! the posture a *pure fold over one append-only event stream*:
//!
//! - **State belongs to the log.** [`PlanModeEvent`] entries carry `init` and
//!   `apply`; [`fold_plan_mode`] projects them into a [`PlanModeProjection`] of
//!   `(mode, state_version)`. The same stream always folds to the same state,
//!   so a resumed or rebuilt session needs no side storage: replay the
//!   persisted entries and the posture (including a parked switch) is back.
//! - **Landing is deferred.** A switch request is parked, never posted. It is
//!   posted (`Apply`) only at the pre-step of the *next accepted turn*
//!   ([`PlanModePhase::AcceptedPreStep`]); a mid-turn attempt to land is
//!   refused ([`PlanModeSwitchBlocked::MidTurnFlip`]), so a turn can never
//!   observe its own projection flip underneath it.
//! - **Enforcement is independent.** A switch alters exactly one thing: the
//!   prompt / behavior-convention projection
//!   ([`PlanModeProjection::prompt_projection`], asserted by
//!   [`PlanModeSwitchEffect::PromptProjectionOnly`]). This module has no code
//!   path to a tool catalog, a sandbox posture, or an approval decision, so a
//!   switch cannot widen permissions — nothing here can grant anything.
//! - **The exit control is resident.** Leaving plan mode is a single fixed
//!   surface (`exit_plan_mode` at the runtime layer) that exists in both
//!   postures; switching changes only the prompt projection, never the set of
//!   surfaces a model can see.
//!
//! Library primitive only: this module performs no I/O, owns no session, and
//! never sees a provider request. The runtime turn chain owns the wiring.

use serde::{Deserialize, Serialize};

/// The two collaborative postures a session alternates between.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanMode {
    /// Planning: research, analysis and proposal design only. The runtime does
    /// **not** block tools in this posture — the restraint is a behavior
    /// convention carried by the prompt projection.
    Planning,
    /// Executing: the default posture. Its prompt projection is empty, so a
    /// session that never engaged plan mode is byte-for-byte unchanged.
    #[default]
    Executing,
}

/// The planning posture's behavior-convention prompt text.
///
/// This is the *only* thing a plan-mode switch changes. It is a behavior
/// agreement rendered into the provider-facing prompt projection; it grants no
/// capability and forbids nothing at the enforcement layer.
pub const PLAN_MODE_BEHAVIOR_CONVENTION: &str = "【协作状态 · 计划模式】当前会话处于计划阶段：\
本阶段只做调研、分析与方案设计，不执行写入、修改或执行类操作，也不调用这类工具；\
把方案写清楚交由用户确认。结束计划阶段请调用始终在场的 exit_plan_mode 工具。\
注意：本段是行为约定提示，不改变可用工具清单与审批判定；\
模式切换在下一被接受回合开始时才生效。";

impl PlanMode {
    /// The prompt / behavior-convention projection of this posture.
    ///
    /// `None` for the default posture: nothing is added to the provider
    /// request, which is the zero-change guarantee for unengaged sessions.
    pub fn prompt_projection(self) -> Option<&'static str> {
        match self {
            Self::Planning => Some(PLAN_MODE_BEHAVIOR_CONVENTION),
            Self::Executing => None,
        }
    }
}

/// One append-only entry of the plan-mode ledger.
///
/// The projected state is defined by `Init` and `Apply`
/// ([`fold_plan_mode`] counts applies into `state_version`).
/// `SwitchRequested` only parks an intent: it never changes the projected
/// mode, so a request recorded mid-turn cannot flip the current turn's
/// projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanModeEvent {
    /// `init`: the posture a session (or a rebuild) starts from.
    Init {
        /// The posture established at initialization.
        mode: PlanMode,
    },
    /// A switch parked but not yet posted. It lands at the pre-step of the
    /// next accepted turn.
    SwitchRequested {
        /// The posture the switch targets.
        target: PlanMode,
    },
    /// `apply`: the parked switch posted. Written only at an accepted turn's
    /// pre-step; `state_version` advances by one.
    Apply {
        /// The posture that just took effect.
        target: PlanMode,
    },
}

/// The folded plan-mode state: a pure projection of one event stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlanModeProjection {
    /// The posture in force.
    pub mode: PlanMode,
    /// `init`/`apply` version: zero after initialization, one more per posted
    /// apply. Two folds of the same stream always agree on this number.
    pub state_version: u64,
    /// A parked switch that has not been posted yet, if any.
    pub pending_switch: Option<PlanMode>,
}

impl PlanModeProjection {
    /// The prompt / behavior-convention projection of the folded posture.
    pub fn prompt_projection(self) -> Option<&'static str> {
        self.mode.prompt_projection()
    }

    /// What a plan-mode switch may alter. Exactly one answer exists, and it is
    /// not a tool, a sandbox, or an approval.
    pub fn switch_effect(self) -> PlanModeSwitchEffect {
        PlanModeSwitchEffect::PromptProjectionOnly
    }
}

/// What a plan-mode switch changes. The single variant is the assertion: there
/// is no code path from a switch to a tool catalog, sandbox posture, or
/// approval decision, because no such output exists on
/// [`PlanModeProjection`] beyond the prompt projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanModeSwitchEffect {
    /// The switch alters the prompt / behavior-convention projection and
    /// nothing else.
    PromptProjectionOnly,
}

/// Fold one event stream into its plan-mode projection.
///
/// Pure and deterministic: the same stream always yields the same projection,
/// and an empty stream yields the default posture at version zero with nothing
/// parked.
pub fn fold_plan_mode(events: &[PlanModeEvent]) -> PlanModeProjection {
    let mut projection = PlanModeProjection::default();
    for event in events {
        match event {
            PlanModeEvent::Init { mode } => {
                projection.mode = *mode;
                projection.pending_switch = None;
            }
            PlanModeEvent::SwitchRequested { target } => {
                projection.pending_switch = Some(*target);
            }
            PlanModeEvent::Apply { target } => {
                projection.mode = *target;
                projection.state_version += 1;
                projection.pending_switch = None;
            }
        }
    }
    projection
}

/// Where in the turn chain a landing is being attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanModePhase {
    /// The pre-step of an accepted turn — the only place a switch may land.
    AcceptedPreStep,
    /// Somewhere inside a running turn. Requests park; nothing lands.
    InTurn,
}

/// Why a switch could not be posted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanModeSwitchBlocked {
    /// A switch may never post in the middle of a running turn: the turn's own
    /// prompt projection would flip underneath it.
    MidTurnFlip,
}

impl std::fmt::Display for PlanModeSwitchBlocked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MidTurnFlip => f.write_str(
                "a plan-mode switch may only be posted at an accepted turn's pre-step, never mid-turn",
            ),
        }
    }
}

impl std::error::Error for PlanModeSwitchBlocked {}

/// The outcome of parking a switch request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanModeRequestOutcome {
    /// The request is parked; it posts at the next accepted turn's pre-step.
    Queued,
    /// The same target was already parked; nothing was appended.
    AlreadyQueued,
    /// The requested posture is already in force and nothing is parked;
    /// nothing was appended (zero-change).
    NoChange,
}

/// Append-only plan-mode ledger with the fold exposed at the tip.
///
/// The ledger is a working copy of the persisted event stream: callers replay
/// the durable entries into it, let it append, and journal the appended
/// entries back. The durable stream — not this object — is the source of
/// truth, which is what makes resume/rebuild recovery automatic.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanModeLedger {
    events: Vec<PlanModeEvent>,
}

impl PlanModeLedger {
    /// An empty ledger folding to the default posture.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuild a ledger by replaying a persisted event stream (resume path).
    pub fn from_events(events: Vec<PlanModeEvent>) -> Self {
        Self { events }
    }

    /// Every appended entry, in order.
    pub fn events(&self) -> &[PlanModeEvent] {
        &self.events
    }

    /// The projection folded from the entries so far.
    pub fn projection(&self) -> PlanModeProjection {
        fold_plan_mode(&self.events)
    }

    /// Park a switch request. Allowed from any phase: a request only parks an
    /// intent and never changes the projected posture.
    ///
    /// Requesting the posture already in force appends nothing, so a session
    /// that keeps asking for its current posture stays byte-for-byte
    /// unchanged.
    pub fn request_switch(&mut self, target: PlanMode) -> PlanModeRequestOutcome {
        let projection = self.projection();
        if projection.pending_switch == Some(target) {
            return PlanModeRequestOutcome::AlreadyQueued;
        }
        if projection.pending_switch.is_none() && projection.mode == target {
            return PlanModeRequestOutcome::NoChange;
        }
        if self.events.is_empty() {
            // The stream is self-describing: the first entry establishes the
            // posture the rebuild will restore before any switch applies.
            self.events.push(PlanModeEvent::Init {
                mode: projection.mode,
            });
        }
        self.events.push(PlanModeEvent::SwitchRequested { target });
        PlanModeRequestOutcome::Queued
    }

    /// Post the parked switch — only at an accepted turn's pre-step.
    ///
    /// The refusal is the deferred-landing guarantee: the exact same parked
    /// request lands when it is due and cannot land a moment earlier. With
    /// nothing parked this is a no-op that still returns the current
    /// projection.
    pub fn land_pending_switch(
        &mut self,
        phase: PlanModePhase,
    ) -> Result<PlanModeProjection, PlanModeSwitchBlocked> {
        if phase != PlanModePhase::AcceptedPreStep {
            return Err(PlanModeSwitchBlocked::MidTurnFlip);
        }
        if let Some(target) = self.projection().pending_switch {
            self.events.push(PlanModeEvent::Apply { target });
        }
        Ok(self.projection())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 投影折叠确定性: the same event stream folds to the same state, however
    /// many times and through whichever entry point it is folded.
    #[test]
    fn one_event_stream_folds_to_exactly_one_projection() {
        let events = vec![
            PlanModeEvent::Init {
                mode: PlanMode::Executing,
            },
            PlanModeEvent::SwitchRequested {
                target: PlanMode::Planning,
            },
            PlanModeEvent::Apply {
                target: PlanMode::Planning,
            },
            PlanModeEvent::SwitchRequested {
                target: PlanMode::Executing,
            },
            PlanModeEvent::Apply {
                target: PlanMode::Executing,
            },
        ];

        let first = fold_plan_mode(&events);
        let second = fold_plan_mode(&events.clone());
        let via_ledger = PlanModeLedger::from_events(events).projection();

        assert_eq!(first, second, "folding the same stream twice must agree");
        assert_eq!(
            first, via_ledger,
            "the ledger fold must equal the pure fold"
        );
        assert_eq!(first.mode, PlanMode::Executing);
        assert_eq!(first.state_version, 2, "two applies posted");
        assert_eq!(first.pending_switch, None);
    }

    /// 切换延迟落账: a parked switch never posts mid-turn — the mid-turn flip
    /// is refused and the projected posture is unchanged — and posts at the
    /// next accepted turn's pre-step.
    #[test]
    fn a_parked_switch_lands_only_at_an_accepted_turn_pre_step() {
        let mut ledger = PlanModeLedger::new();
        assert_eq!(
            ledger.request_switch(PlanMode::Planning),
            PlanModeRequestOutcome::Queued
        );

        // Round N of a running turn: the switch must not land here.
        assert_eq!(
            ledger.land_pending_switch(PlanModePhase::InTurn),
            Err(PlanModeSwitchBlocked::MidTurnFlip)
        );
        let mid_turn = ledger.projection();
        assert_eq!(mid_turn.mode, PlanMode::Executing, "no mid-turn flip");
        assert_eq!(mid_turn.state_version, 0, "nothing posted");
        assert_eq!(mid_turn.pending_switch, Some(PlanMode::Planning));

        // The next accepted turn's pre-step: it lands exactly here.
        let landed = ledger
            .land_pending_switch(PlanModePhase::AcceptedPreStep)
            .expect("a pre-step landing is never blocked");
        assert_eq!(landed.mode, PlanMode::Planning);
        assert_eq!(landed.state_version, 1);
        assert_eq!(landed.pending_switch, None);
    }

    /// 执法独立: a switch alters the prompt projection and nothing else. The
    /// projection carries no tool-catalog, sandbox, or approval output at all.
    #[test]
    fn a_switch_alters_only_the_prompt_projection() {
        let planning = PlanModeLedger::from_events(vec![
            PlanModeEvent::Init {
                mode: PlanMode::Executing,
            },
            PlanModeEvent::Apply {
                target: PlanMode::Planning,
            },
        ])
        .projection();
        let executing = fold_plan_mode(&[]);

        assert_ne!(
            planning.prompt_projection(),
            executing.prompt_projection(),
            "the prompt projection is what differs"
        );
        assert!(
            planning.prompt_projection().is_some(),
            "planning projects its behavior convention"
        );
        assert!(
            executing.prompt_projection().is_none(),
            "the default posture projects nothing"
        );
        assert_eq!(
            planning.switch_effect(),
            PlanModeSwitchEffect::PromptProjectionOnly
        );
        assert_eq!(
            executing.switch_effect(),
            PlanModeSwitchEffect::PromptProjectionOnly
        );
    }

    /// resume 恢复: a rebuilt ledger replaying the persisted stream restores
    /// the same state, including a switch that was parked but not yet posted.
    #[test]
    fn resuming_replays_the_persisted_stream_into_the_same_state() {
        let mut ledger = PlanModeLedger::new();
        ledger.request_switch(PlanMode::Planning);
        ledger
            .land_pending_switch(PlanModePhase::AcceptedPreStep)
            .unwrap();
        ledger.request_switch(PlanMode::Executing); // parked, not posted yet

        let persisted = serde_json::to_string(ledger.events()).unwrap();
        let restored: Vec<PlanModeEvent> = serde_json::from_str(&persisted).unwrap();
        let rebuilt = PlanModeLedger::from_events(restored).projection();

        assert_eq!(rebuilt, ledger.projection());
        assert_eq!(rebuilt.mode, PlanMode::Planning);
        assert_eq!(rebuilt.state_version, 1);
        assert_eq!(
            rebuilt.pending_switch,
            Some(PlanMode::Executing),
            "a parked switch survives a rebuild"
        );
    }

    /// 默认行为零变化: a stream that never engaged plan mode folds to the
    /// default posture with no prompt projection and no appended entries.
    #[test]
    fn an_unengaged_ledger_stays_at_the_default_posture() {
        let mut ledger = PlanModeLedger::new();
        assert_eq!(ledger.projection(), PlanModeProjection::default());
        assert_eq!(ledger.projection().mode, PlanMode::Executing);
        assert_eq!(ledger.projection().state_version, 0);
        assert_eq!(ledger.projection().prompt_projection(), None);

        // Asking for the posture already in force appends nothing at all.
        assert_eq!(
            ledger.request_switch(PlanMode::Executing),
            PlanModeRequestOutcome::NoChange
        );
        assert!(
            ledger.events().is_empty(),
            "zero-change must append nothing"
        );
    }

    /// Repeated identical requests coalesce instead of stacking, and a later
    /// request replaces the parked target (latest intent wins).
    #[test]
    fn switch_requests_coalesce_and_the_latest_intent_wins() {
        let mut ledger = PlanModeLedger::new();
        assert_eq!(
            ledger.request_switch(PlanMode::Planning),
            PlanModeRequestOutcome::Queued
        );
        assert_eq!(
            ledger.request_switch(PlanMode::Planning),
            PlanModeRequestOutcome::AlreadyQueued
        );
        assert_eq!(
            ledger.request_switch(PlanMode::Executing),
            PlanModeRequestOutcome::Queued
        );
        assert_eq!(
            ledger.projection().pending_switch,
            Some(PlanMode::Executing)
        );

        // Only the first request initialized and parked; the duplicate added
        // nothing; the replacement added one entry.
        assert_eq!(ledger.events().len(), 3);
    }
}
