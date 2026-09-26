//! Approval closure vocabulary and paired audit writes.
//!
//! An "ask a human" round needs two things this module owns:
//!
//! * A **closed vocabulary** for how the round ended: [`ApprovalOutcome`] with
//!   exactly four states. The only authorization semantics in the vocabulary
//!   is [`ApprovalOutcome::AllowedOnce`]: one closure authorizes exactly one
//!   action (the standing one-approval-one-side-effect invariant).
//!   [`ApprovalOutcome::Cancelled`] and [`ApprovalOutcome::Unavailable`] are
//!   fail-closed like [`ApprovalOutcome::Rejected`]: nothing executes.
//! * **Paired audit writes with atomic commit**: the `asked` record and the
//!   `decision` record of one round are written together through
//!   [`commit_approval_audit_pair`]. Both durable writes land or the whole
//!   pair is rejected and rolled back — there is no committed half-state in
//!   process. The failure mode that cannot be rolled back — a crash between
//!   the two writes — leaves the `asked` record alone, and that unclosed pair
//!   is detected after recovery by [`detect_unclosed_approval_pairs`] (the
//!   same crash-detectable pair shape the compaction checkpoint log uses).
//!
//! The `ask`|`never` policy is recorded as a **log-folding event**
//! ([`PolicyAskEvent`]): the effective policy is a derived view
//! ([`fold_policy_ask_events`]), and the model learns it through the runtime
//! snapshot, never through the transcript.
//!
//! # Verdict semantics are unchanged
//!
//! [`crate::Decision`] and [`crate::GovernancePipeline`] decide *whether* an
//! action may proceed; this vocabulary closes *how an ask round ended*
//! afterwards. The existing approval state machine keeps its own states and
//! labels; its stable resolution labels map onto this vocabulary through
//! [`ApprovalOutcome::from_resolution_label`], and an unknown label maps to
//! `None` instead of being silently reinterpreted.

use std::convert::Infallible;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::sandbox_preset::ApprovalPolicyName;

/// How one ask round ended.
///
/// This is the closure vocabulary, not a verdict type: it is produced after
/// [`crate::Decision::RequireApproval`] already suspended the action. Every
/// state except [`Self::AllowedOnce`] is fail-closed.
///
/// Every `match` on this enum in this module lists one arm per state and has
/// no wildcard: a new state must be classified explicitly before it compiles,
/// and [`Self::ALL`] keeps the runtime enumeration complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalOutcome {
    /// The human allowed the action — exactly once, and only for the action
    /// the pair names. This is the sole authorization state.
    AllowedOnce,
    /// The human refused the action. Fail-closed.
    Rejected,
    /// The round was cancelled without executing the action. Fail-closed.
    Cancelled,
    /// No usable human decision arrived (the ask channel failed, timed out,
    /// or closed). Fail-closed.
    Unavailable,
}

/// Authorization classification of one closure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosureClass {
    /// The state authorizes exactly one named action, one time.
    AuthorizedOnce,
    /// The state authorizes nothing (fail-closed: the action does not run).
    FailClosed,
}

impl ApprovalOutcome {
    /// Every closure state, in vocabulary order. A new state must be added
    /// here and to every exhaustive match in this module.
    pub const ALL: [Self; 4] = [
        Self::AllowedOnce,
        Self::Rejected,
        Self::Cancelled,
        Self::Unavailable,
    ];

    /// Stable label for structured traces and audit records.
    pub const fn label(self) -> &'static str {
        match self {
            Self::AllowedOnce => "allowed_once",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Unavailable => "unavailable",
        }
    }

    /// Classify the closure state. One arm per state: a new state fails to
    /// compile here until it is classified explicitly.
    pub const fn class(self) -> ClosureClass {
        match self {
            Self::AllowedOnce => ClosureClass::AuthorizedOnce,
            Self::Rejected => ClosureClass::FailClosed,
            Self::Cancelled => ClosureClass::FailClosed,
            Self::Unavailable => ClosureClass::FailClosed,
        }
    }

    /// Whether this closure authorizes anything — and then only its own
    /// action, exactly once.
    pub const fn authorizes_once(self) -> bool {
        matches!(self.class(), ClosureClass::AuthorizedOnce)
    }

    /// Whether this closure refuses execution outright.
    pub const fn fail_closed(self) -> bool {
        !self.authorizes_once()
    }

    /// Map the stable resolution labels of the existing approval state
    /// machine onto the closure vocabulary. An unknown label yields `None`:
    /// a label this vocabulary has never seen is never reinterpreted.
    pub fn from_resolution_label(label: &str) -> Option<Self> {
        match label {
            "approved" => Some(Self::AllowedOnce),
            "rejected" => Some(Self::Rejected),
            "cancelled" => Some(Self::Cancelled),
            "expired" | "interrupted" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

impl fmt::Display for ApprovalOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Audit event kind for the ask half of a pair.
pub const AUDIT_EVENT_ASKED: &str = "approval.asked";

/// Audit event kind for the decision half of a pair.
pub const AUDIT_EVENT_DECISION: &str = "approval.decision";

/// Which half of the ask↔decision pair one record fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalAuditSlot {
    /// The round asked a human.
    Asked,
    /// The round reached a closure decision.
    Decision,
}

/// One durable record of one approval round.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "slot", rename_all = "snake_case")]
pub enum ApprovalAuditRecord {
    /// The ask half: a human was asked in this round.
    Asked {
        /// Identity shared with the decision half.
        pair_id: String,
        /// Round the pair belongs to; pairing is same-round.
        round: u64,
        /// The action under approval (capability identity).
        subject: String,
    },
    /// The decision half: the ask round closed with this outcome.
    Decision {
        /// Identity shared with the ask half.
        pair_id: String,
        /// Round the pair belongs to; pairing is same-round.
        round: u64,
        /// The action under approval (capability identity).
        subject: String,
        /// How the round closed.
        outcome: ApprovalOutcome,
    },
}

impl ApprovalAuditRecord {
    /// Identity shared by the two records of one pair.
    pub fn pair_id(&self) -> &str {
        match self {
            Self::Asked { pair_id, .. } | Self::Decision { pair_id, .. } => pair_id,
        }
    }

    /// The round both halves of the pair belong to.
    pub const fn round(&self) -> u64 {
        match self {
            Self::Asked { round, .. } | Self::Decision { round, .. } => *round,
        }
    }

    /// The action under approval.
    pub fn subject(&self) -> &str {
        match self {
            Self::Asked { subject, .. } | Self::Decision { subject, .. } => subject,
        }
    }

    /// Which half of the pair this record fills.
    pub const fn slot(&self) -> ApprovalAuditSlot {
        match self {
            Self::Asked { .. } => ApprovalAuditSlot::Asked,
            Self::Decision { .. } => ApprovalAuditSlot::Decision,
        }
    }

    /// The closure outcome, on the decision half only.
    pub const fn outcome(&self) -> Option<ApprovalOutcome> {
        match self {
            Self::Asked { .. } => None,
            Self::Decision { outcome, .. } => Some(*outcome),
        }
    }

    /// The audit event kind, matching the standing `approval.*` vocabulary.
    pub const fn event_kind(&self) -> &'static str {
        match self {
            Self::Asked { .. } => AUDIT_EVENT_ASKED,
            Self::Decision { .. } => AUDIT_EVENT_DECISION,
        }
    }
}

/// The two records of one approval round, committed as one unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalAuditPair {
    asked: ApprovalAuditRecord,
    decision: ApprovalAuditRecord,
}

impl ApprovalAuditPair {
    /// Build the pair. Both halves share one pair identity, one round, and
    /// one subject, so pairing is same-round by construction.
    pub fn new(
        pair_id: impl Into<String>,
        round: u64,
        subject: impl Into<String>,
        outcome: ApprovalOutcome,
    ) -> Self {
        let pair_id = pair_id.into();
        let subject = subject.into();
        Self {
            asked: ApprovalAuditRecord::Asked {
                pair_id: pair_id.clone(),
                round,
                subject: subject.clone(),
            },
            decision: ApprovalAuditRecord::Decision {
                pair_id,
                round,
                subject,
                outcome,
            },
        }
    }

    /// The ask half.
    pub fn asked(&self) -> &ApprovalAuditRecord {
        &self.asked
    }

    /// The decision half.
    pub fn decision(&self) -> &ApprovalAuditRecord {
        &self.decision
    }

    /// How the round closed.
    pub fn outcome(&self) -> ApprovalOutcome {
        match &self.decision {
            ApprovalAuditRecord::Decision { outcome, .. } => *outcome,
            ApprovalAuditRecord::Asked { .. } => {
                unreachable!("a pair always carries a decision half")
            }
        }
    }

    /// The pair identity shared by both halves.
    pub fn pair_id(&self) -> &str {
        self.asked.pair_id()
    }

    /// The round both halves belong to.
    pub const fn round(&self) -> u64 {
        self.asked.round()
    }

    /// The action under approval.
    pub fn subject(&self) -> &str {
        self.asked.subject()
    }
}

/// Destination for paired approval audit records.
///
/// Records are durable appends; a savepoint (`committed_len`) lets the pair
/// writer roll a half-written pair back so a failed commit leaves no visible
/// half-state.
pub trait ApprovalAuditSink {
    /// Error surfaced when a durable write fails.
    type Error;

    /// Savepoint: how many records are durably committed.
    fn committed_len(&self) -> usize;

    /// Durably append one record.
    fn write(&mut self, record: ApprovalAuditRecord) -> Result<(), Self::Error>;

    /// Roll back to a savepoint after a failed pair commit.
    fn rollback(&mut self, committed_len: usize);
}

/// In-memory reference sink: a durable append is a push. It gives the pairing
/// contract a concrete, dependency-free implementation to commit against.
#[derive(Debug, Clone, Default)]
pub struct MemoryApprovalAuditSink {
    committed: Vec<ApprovalAuditRecord>,
}

impl MemoryApprovalAuditSink {
    /// A sink with no records.
    pub fn new() -> Self {
        Self::default()
    }

    /// The durably committed records, in order.
    pub fn records(&self) -> &[ApprovalAuditRecord] {
        &self.committed
    }
}

impl ApprovalAuditSink for MemoryApprovalAuditSink {
    type Error = Infallible;

    fn committed_len(&self) -> usize {
        self.committed.len()
    }

    fn write(&mut self, record: ApprovalAuditRecord) -> Result<(), Self::Error> {
        self.committed.push(record);
        Ok(())
    }

    fn rollback(&mut self, committed_len: usize) {
        self.committed.truncate(committed_len);
    }
}

/// Why a paired commit was rejected. Either variant means the pair is not in
/// the sink at all: the commit is transactional — both records or neither.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairCommitError<E> {
    /// The `asked` write failed; nothing of the pair was committed.
    Asked {
        /// The write failure.
        reason: E,
    },
    /// The `decision` write failed; the `asked` write was rolled back first.
    Decision {
        /// The write failure.
        reason: E,
    },
}

impl<E: fmt::Display> fmt::Display for PairCommitError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Asked { reason } => write!(
                f,
                "approval audit pair rejected: asked write failed ({reason}), nothing committed"
            ),
            Self::Decision { reason } => write!(
                f,
                "approval audit pair rejected: decision write failed ({reason}), asked write rolled back"
            ),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for PairCommitError<E> {}

/// Commit the ask↔decision pair as one transaction.
///
/// Both durable writes land or the whole pair is rejected and rolled back:
/// a failed `asked` write leaves the sink untouched, and a failed `decision`
/// write rolls the `asked` record back before the error is returned. The
/// failure mode that cannot be rolled back — a crash between the two writes —
/// leaves the `asked` record alone; [`detect_unclosed_approval_pairs`] reports
/// that unclosed pair after recovery.
pub fn commit_approval_audit_pair<S: ApprovalAuditSink>(
    sink: &mut S,
    pair: &ApprovalAuditPair,
) -> Result<(), PairCommitError<S::Error>> {
    let savepoint = sink.committed_len();
    if let Err(reason) = sink.write(pair.asked().clone()) {
        sink.rollback(savepoint);
        return Err(PairCommitError::Asked { reason });
    }
    if let Err(reason) = sink.write(pair.decision().clone()) {
        sink.rollback(savepoint);
        return Err(PairCommitError::Decision { reason });
    }
    Ok(())
}

/// What one approval-audit scan found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ApprovalPairScan {
    /// Pair identities whose `asked` record never received its `decision`, in
    /// first-seen order.
    pub unclosed: Vec<String>,
    /// Records the scan refused, with a legible reason each.
    pub rejected: Vec<String>,
}

/// Scan one record stream once: close ask↔decision pairs and note every
/// anomaly. Shared by [`detect_unclosed_approval_pairs`] and the pair
/// verifiers so the reads can never disagree.
fn scan_pairs(records: &[ApprovalAuditRecord]) -> ApprovalPairScan {
    let mut scan = ApprovalPairScan::default();
    // Open asks as (pair identity, round, subject).
    let mut open: Vec<(String, u64, String)> = Vec::new();
    let mut closed: Vec<String> = Vec::new();
    for record in records {
        match record {
            ApprovalAuditRecord::Asked {
                pair_id,
                round,
                subject,
            } => {
                if open.iter().any(|(open_id, _, _)| open_id == pair_id) {
                    scan.rejected
                        .push(format!("pair {pair_id} asked twice without a decision"));
                } else if closed.iter().any(|closed_id| closed_id == pair_id) {
                    scan.rejected
                        .push(format!("pair {pair_id} asked again after its decision"));
                } else {
                    open.push((pair_id.clone(), *round, subject.clone()));
                }
            }
            ApprovalAuditRecord::Decision {
                pair_id,
                round,
                subject,
                ..
            } => match open.iter().position(|(open_id, _, _)| open_id == pair_id) {
                None => scan.rejected.push(format!(
                    "pair {pair_id} decided without an open asked record"
                )),
                Some(position) => {
                    let (_, asked_round, asked_subject) = &open[position];
                    if *asked_round != *round {
                        scan.rejected.push(format!(
                                "pair {pair_id} closed outside its round {asked_round} (decision round {round})"
                            ));
                    } else if asked_subject != subject {
                        scan.rejected
                            .push(format!("pair {pair_id} closed for a different subject"));
                    } else {
                        open.remove(position);
                        closed.push(pair_id.clone());
                    }
                }
            },
        }
    }
    for (pair_id, _, _) in open {
        scan.unclosed.push(pair_id);
    }
    scan
}

/// Pair identities whose ask record never received its decision.
///
/// The crash-detection read: a process that died between the two durable
/// writes leaves its ask record here forever, and the pair is never treated as
/// closed.
pub fn detect_unclosed_approval_pairs(records: &[ApprovalAuditRecord]) -> Vec<String> {
    scan_pairs(records).unclosed
}

/// Scan one record stream, reporting unclosed pairs and refused records.
pub fn scan_approval_pairs(records: &[ApprovalAuditRecord]) -> ApprovalPairScan {
    scan_pairs(records)
}

/// One-shot authorization minted by an [`ApprovalOutcome::AllowedOnce`]
/// closure.
///
/// This carries the standing one-approval-one-side-effect invariant at the
/// closure layer: the grant names the paired action and can be spent exactly
/// once. Every other closure state is fail-closed and mints nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowedOnceGrant {
    pair_id: String,
    action: String,
    spent: bool,
}

/// Why a grant spend was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantSpendError {
    /// The grant was already spent: one closure authorizes one action once.
    AlreadySpent {
        /// The pair the grant came from.
        pair_id: String,
    },
    /// The requested action is not the action the pair named.
    ActionMismatch {
        /// The action the pair named.
        expected: String,
        /// The action the spend asked for.
        requested: String,
    },
}

impl fmt::Display for GrantSpendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadySpent { pair_id } => {
                write!(
                    f,
                    "grant for pair {pair_id} is already spent (one closure, one action, once)"
                )
            }
            Self::ActionMismatch {
                expected,
                requested,
            } => {
                write!(f, "grant covers {expected} only; refusing {requested}")
            }
        }
    }
}

impl std::error::Error for GrantSpendError {}

impl AllowedOnceGrant {
    /// Mint a grant for the pair's action. Only
    /// [`ApprovalOutcome::AllowedOnce`] mints one; every other closure state
    /// is fail-closed and mints nothing.
    pub fn mint(pair: &ApprovalAuditPair) -> Option<Self> {
        if pair.outcome().authorizes_once() {
            Some(Self {
                pair_id: pair.pair_id().to_string(),
                action: pair.subject().to_string(),
                spent: false,
            })
        } else {
            None
        }
    }

    /// Spend the grant on its own action, exactly once. A second spend, or a
    /// spend for a different action, is refused and consumes nothing.
    pub fn spend(&mut self, action: &str) -> Result<(), GrantSpendError> {
        if self.spent {
            return Err(GrantSpendError::AlreadySpent {
                pair_id: self.pair_id.clone(),
            });
        }
        if action != self.action {
            return Err(GrantSpendError::ActionMismatch {
                expected: self.action.clone(),
                requested: action.to_string(),
            });
        }
        self.spent = true;
        Ok(())
    }

    /// Whether the single authorization has been consumed.
    pub fn is_spent(&self) -> bool {
        self.spent
    }

    /// The action this grant covers.
    pub fn action(&self) -> &str {
        &self.action
    }

    /// The pair this grant was minted from.
    pub fn pair_id(&self) -> &str {
        &self.pair_id
    }
}

/// One log-folding event: the effective `ask`|`never` policy at a log point.
///
/// The policy choice is recorded as a fold event, not a transcript message:
/// the log stays append-only, the effective policy is a derived view
/// ([`fold_policy_ask_events`]), and the model learns it through the runtime
/// snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyAskEvent {
    /// Log position, counting from zero.
    pub sequence: u64,
    /// Effective policy from this point on.
    pub policy: ApprovalPolicyName,
}

impl PolicyAskEvent {
    /// Policy events fold into derived state; they are never echoed into the
    /// model transcript.
    pub const TRANSCRIPT_VISIBLE: bool = false;
}

/// The derived view of the policy fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolicyAskFold {
    /// The effective policy after folding the whole stream.
    pub policy: ApprovalPolicyName,
    /// How many earlier events the fold collapsed into this view.
    pub collapsed_events: usize,
}

/// Fold policy events into one effective policy.
///
/// The fold is a pure function of its inputs in log order: the same event
/// stream always folds to the same policy, superseded events collapse into
/// the derived view, and an empty stream folds to `None` (no policy recorded
/// yet — nothing is invented).
pub fn fold_policy_ask_events(events: &[PolicyAskEvent]) -> Option<PolicyAskFold> {
    let last = events.last()?;
    Some(PolicyAskFold {
        policy: last.policy,
        collapsed_events: events.len().saturating_sub(1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, Decision, DenyUnconfigured, GovernanceHook, GovernanceRequest};
    use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};

    #[test]
    fn closure_vocabulary_four_states_have_exact_semantics() {
        let allowed = ApprovalOutcome::AllowedOnce;
        assert_eq!(allowed.label(), "allowed_once");
        assert!(allowed.authorizes_once());
        assert!(!allowed.fail_closed());

        // 其余三态一律 fail-closed (不执行), 词表语义各自独立。
        for outcome in [
            ApprovalOutcome::Rejected,
            ApprovalOutcome::Cancelled,
            ApprovalOutcome::Unavailable,
        ] {
            assert!(
                !outcome.authorizes_once(),
                "{} must not authorize",
                outcome.label()
            );
            assert!(
                outcome.fail_closed(),
                "{} must fail closed",
                outcome.label()
            );
        }
        assert_eq!(ApprovalOutcome::Rejected.label(), "rejected");
        assert_eq!(ApprovalOutcome::Cancelled.label(), "cancelled");
        assert_eq!(ApprovalOutcome::Unavailable.label(), "unavailable");

        // 与既有审批状态机的稳定标签桥接: 语义一一对应, 不改判定。
        assert_eq!(
            ApprovalOutcome::from_resolution_label("approved"),
            Some(ApprovalOutcome::AllowedOnce)
        );
        assert_eq!(
            ApprovalOutcome::from_resolution_label("rejected"),
            Some(ApprovalOutcome::Rejected)
        );
        assert_eq!(
            ApprovalOutcome::from_resolution_label("cancelled"),
            Some(ApprovalOutcome::Cancelled)
        );
        assert_eq!(
            ApprovalOutcome::from_resolution_label("expired"),
            Some(ApprovalOutcome::Unavailable)
        );
        assert_eq!(
            ApprovalOutcome::from_resolution_label("interrupted"),
            Some(ApprovalOutcome::Unavailable)
        );
    }

    #[test]
    fn allowed_once_grant_is_spent_exactly_once_for_its_own_action() {
        let pair = ApprovalAuditPair::new("pair-1", 3, "tool.shell", ApprovalOutcome::AllowedOnce);
        let mut grant = AllowedOnceGrant::mint(&pair).expect("allowed_once mints a one-shot grant");
        assert_eq!(grant.action(), "tool.shell");
        assert_eq!(grant.pair_id(), "pair-1");
        assert!(!grant.is_spent());

        grant.spend("tool.shell").expect("the one authorization");
        assert!(grant.is_spent());
        assert!(matches!(
            grant.spend("tool.shell"),
            Err(GrantSpendError::AlreadySpent { .. })
        ));

        // 授权只覆盖配对动作本身, 换动作不消耗授权。
        let pair = ApprovalAuditPair::new("pair-2", 4, "tool.shell", ApprovalOutcome::AllowedOnce);
        let mut grant = AllowedOnceGrant::mint(&pair).unwrap();
        assert!(matches!(
            grant.spend("tool.fetch"),
            Err(GrantSpendError::ActionMismatch { .. })
        ));
        assert!(!grant.is_spent(), "refused spends consume nothing");

        // 非授权闭合态一律 fail-closed: 不铸造任何授权。
        for outcome in [
            ApprovalOutcome::Rejected,
            ApprovalOutcome::Cancelled,
            ApprovalOutcome::Unavailable,
        ] {
            let pair = ApprovalAuditPair::new("pair-x", 5, "tool.shell", outcome);
            assert!(
                AllowedOnceGrant::mint(&pair).is_none(),
                "{} must mint no grant",
                outcome.label()
            );
        }
    }

    #[test]
    fn an_unclosed_asked_pair_is_detected_after_a_crash() {
        // 两次落盘之间崩溃: 只留下 asked 记录 → 未闭对可检出。
        let crashed = vec![ApprovalAuditRecord::Asked {
            pair_id: "pair-1".into(),
            round: 7,
            subject: "tool.shell".into(),
        }];
        assert_eq!(
            detect_unclosed_approval_pairs(&crashed),
            vec!["pair-1".to_string()]
        );
        let scan = scan_approval_pairs(&crashed);
        assert!(
            scan.rejected.is_empty(),
            "an orphan ask is an unclosed pair, not a bad record: {:?}",
            scan.rejected
        );

        // 闭合的对检测干净。
        let closed = vec![
            ApprovalAuditRecord::Asked {
                pair_id: "pair-1".into(),
                round: 7,
                subject: "tool.shell".into(),
            },
            ApprovalAuditRecord::Decision {
                pair_id: "pair-1".into(),
                round: 7,
                subject: "tool.shell".into(),
                outcome: ApprovalOutcome::Rejected,
            },
        ];
        assert!(detect_unclosed_approval_pairs(&closed).is_empty());
    }

    /// In-memory sink that fails its Nth write (0-based).
    struct FaultySink {
        committed: Vec<ApprovalAuditRecord>,
        fail_at: usize,
    }

    impl ApprovalAuditSink for FaultySink {
        type Error = &'static str;

        fn committed_len(&self) -> usize {
            self.committed.len()
        }

        fn write(&mut self, record: ApprovalAuditRecord) -> Result<(), Self::Error> {
            if self.committed.len() == self.fail_at {
                return Err("write failed");
            }
            self.committed.push(record);
            Ok(())
        }

        fn rollback(&mut self, committed_len: usize) {
            self.committed.truncate(committed_len);
        }
    }

    #[test]
    fn pair_commit_is_rejected_whole_when_a_write_fails_midway() {
        let pair = ApprovalAuditPair::new("pair-1", 7, "tool.shell", ApprovalOutcome::AllowedOnce);

        // 第二笔中途失败: 首笔回滚, 整对拒绝返回 — 池里不留半态。
        let mut sink = FaultySink {
            committed: Vec::new(),
            fail_at: 1,
        };
        let error = commit_approval_audit_pair(&mut sink, &pair).unwrap_err();
        assert!(matches!(error, PairCommitError::Decision { .. }));
        assert!(
            sink.committed.is_empty(),
            "双写必须同提交, 否则整体拒绝返回: {:?}",
            sink.committed
        );
        assert!(detect_unclosed_approval_pairs(&sink.committed).is_empty());

        // 首笔失败: 同样什么都不留。
        let mut sink = FaultySink {
            committed: Vec::new(),
            fail_at: 0,
        };
        assert!(matches!(
            commit_approval_audit_pair(&mut sink, &pair).unwrap_err(),
            PairCommitError::Asked { .. }
        ));
        assert!(sink.committed.is_empty());

        // 无故障: 两笔同提交, asked↔decision 同回合闭合。
        let mut sink = MemoryApprovalAuditSink::new();
        commit_approval_audit_pair(&mut sink, &pair).expect("both writes commit together");
        assert_eq!(sink.records().len(), 2);
        assert_eq!(sink.records()[0].slot(), ApprovalAuditSlot::Asked);
        assert_eq!(sink.records()[1].slot(), ApprovalAuditSlot::Decision);
        assert_eq!(sink.records()[0].round(), sink.records()[1].round());
        assert!(detect_unclosed_approval_pairs(sink.records()).is_empty());
    }

    #[test]
    fn closure_vocabulary_is_exhaustively_classified() {
        // `class()` 每态一臂且无通配: 新增闭合态必须被显式分类才能编译。
        assert_eq!(ApprovalOutcome::ALL.len(), 4);
        for outcome in ApprovalOutcome::ALL {
            let expected = if outcome == ApprovalOutcome::AllowedOnce {
                ClosureClass::AuthorizedOnce
            } else {
                ClosureClass::FailClosed
            };
            assert_eq!(outcome.class(), expected, "{}", outcome.label());
        }

        // 标签互异且穷尽: 词表不得折叠, 也不得有未命名的态。
        let mut labels: Vec<&str> = ApprovalOutcome::ALL.iter().map(|o| o.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), ApprovalOutcome::ALL.len());

        // 桥接不重释未知标签: 默认行为零改写。
        assert_eq!(
            ApprovalOutcome::from_resolution_label("approved_once"),
            None
        );
        assert_eq!(ApprovalOutcome::from_resolution_label(""), None);
    }

    #[test]
    fn a_pair_must_close_within_the_same_round() {
        // 同回合内配对: 决议落在别的回合 = 配对破坏 — 决议被拒且 asked 仍未闭对。
        let stream = vec![
            ApprovalAuditRecord::Asked {
                pair_id: "pair-1".into(),
                round: 7,
                subject: "tool.shell".into(),
            },
            ApprovalAuditRecord::Decision {
                pair_id: "pair-1".into(),
                round: 8,
                subject: "tool.shell".into(),
                outcome: ApprovalOutcome::AllowedOnce,
            },
        ];
        let scan = scan_approval_pairs(&stream);
        assert!(
            !scan.rejected.is_empty(),
            "a cross-round decision must be refused"
        );
        assert_eq!(
            scan.unclosed,
            vec!["pair-1".to_string()],
            "the ask half stays unclosed"
        );
    }

    #[test]
    fn policy_ask_never_folds_without_entering_the_transcript() {
        // 空日志不发明策略。
        assert!(fold_policy_ask_events(&[]).is_none());

        let events = [
            PolicyAskEvent {
                sequence: 0,
                policy: ApprovalPolicyName::Ask,
            },
            PolicyAskEvent {
                sequence: 1,
                policy: ApprovalPolicyName::Ask,
            },
            PolicyAskEvent {
                sequence: 2,
                policy: ApprovalPolicyName::Never,
            },
        ];
        let fold = fold_policy_ask_events(&events).expect("the stream has events");
        assert_eq!(fold.policy, ApprovalPolicyName::Never);
        assert_eq!(fold.collapsed_events, 2, "重复事件折叠进同一派生视图");

        // 折叠是纯函数: 同一事件流同一结果。
        assert_eq!(
            fold_policy_ask_events(&events),
            fold_policy_ask_events(&events)
        );

        // 策略事件只进日志折叠, 不进模型转写。
        assert!(!PolicyAskEvent::TRANSCRIPT_VISIBLE);
    }

    #[tokio::test]
    async fn default_decision_semantics_are_unchanged() {
        // 判定词表原样: 闭合词表收在判定之后, 不改任何判定。
        assert!(Decision::Allow.is_allowed());
        assert_eq!(Decision::deny("no").label(), "deny");
        assert_eq!(
            Decision::require_approval("ask").label(),
            "require_approval"
        );

        // 默认 fail-closed 门行为零变化。
        let cap = CapabilityId::new("tool.shell").unwrap();
        let args = serde_json::Value::Null;
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &cap,
                arguments: &args,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );
        assert!(!DenyUnconfigured.evaluate(&request).await.is_allowed());

        // 默认闭合语义: fail-closed 态一律不放行, 只有 AllowedOnce 铸造一次性授权。
        for outcome in [
            ApprovalOutcome::Rejected,
            ApprovalOutcome::Cancelled,
            ApprovalOutcome::Unavailable,
        ] {
            let pair = ApprovalAuditPair::new("pair-d", 1, "tool.shell", outcome);
            assert!(AllowedOnceGrant::mint(&pair).is_none());
        }
    }
}
