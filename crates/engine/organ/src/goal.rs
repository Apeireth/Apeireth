//! Goal state machine + crash-safe persist (library, default-off).
//!
//! Recovered from `legacy/donor/apeireth-companion/src/goal.rs` (DSH-style
//! single-current-goal machine). This is a **mechanism**: one current goal,
//! guarded phase transitions, revision CAS, `rounds_started` only on
//! goal-driven turns, atomic-file persist (shared atomic durable writer). It is
//! **not** a round driver, not a daemon, and not a second agent loop.
//!
//! Semantics kept from the donor:
//! - Single current goal. `create` refuses while a non-completed goal exists.
//! - `revision` is strictly +1 on every committed mutation.
//! - Illegal phase transitions leave state unchanged.
//! - One blocked phase with `blocked_reason { code, message }` (no extra states).
//! - `rounds_started` increments only via [`GoalService::admit_round`].
//! - Hitting `max_goal_rounds` auto-blocks with code `max-rounds`.
//!
//! Honest adaptations vs the donor:
//! - Donor claimed compare-and-set (`StaleRevision`) but never compared an
//!   expected revision. This port **enforces** CAS: every mutation takes the
//!   caller's expected revision and rejects a stale handle.
//! - Donor swallowed persist errors (`let _ = store.save`). This port surfaces
//!   typed I/O / serialization errors; in-memory state is not advanced if
//!   persist fails.
//! - No `uuid` / `chrono` crate deps. Ids are minted from
//!   [`apeireth_core::kernel::TaskId`]; timestamps are injected (`now_ms`).
//! - Persist filenames are sanitized (no path escape). Temp-file names are
//!   derived by the shared atomic writer from the sanitized target name.
//!
//! Production wiring: none. Callers that want a Goal organ later compose this
//! library behind `OrganTrait`; this module does not register, tick, or speak.
//!
//! Continuation-drive layer (additive, on top of the machine above):
//! - Strict replay fold: [`fold_goal_events`] folds the full-snapshot
//!   `goal`/`change` event stream into one current goal state; the same event
//!   stream always folds to the same state, and streams the machine could not
//!   have produced are refused.
//! - Reservation vs admission: [`GoalContinuationDrive`] reserves the next
//!   round number (`rounds_started + 1`) up front and consumes quota only
//!   when the reserved round's message actually enters history. A stale
//!   reservation consumes no number; every step rechecks the revision first.
//! - `armed`/`disarmed` is process state only: it is never persisted, and a
//!   resumed goal starts disarmed (ceasefire by default).

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use apeireth_core::kernel::TaskId;
use apeireth_core::storage_atomic;
use serde::{Deserialize, Serialize};

/// Goal phase. One blocked phase — reasons live on the snapshot, not as extra
/// variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalPhase {
    Active,
    Paused,
    Completed,
    Blocked,
}

impl GoalPhase {
    /// Stable lowercase label used by tools / snapshots.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
        }
    }
}

/// Single blocked-phase payload. Does not proliferate GoalPhase variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalBlock {
    pub code: String,
    pub message: String,
}

/// Full goal snapshot. Every committed mutation writes the whole record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalSnapshot {
    pub id: String,
    pub revision: u64,
    pub objective: String,
    pub phase: GoalPhase,
    pub max_goal_rounds: u64,
    pub rounds_started: u64,
    pub blocked_reason: Option<GoalBlock>,
    pub updated_at_ms: i64,
}

impl GoalSnapshot {
    /// Whether this snapshot may be replaced by [`GoalService::create`].
    pub const fn is_replaceable(&self) -> bool {
        matches!(self.phase, GoalPhase::Completed)
    }
}

/// One goal-domain event. Both kinds carry the **full snapshot** committed at
/// that point: the fold never patches individual fields, it replays whole
/// snapshots through [`fold_goal_events`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum GoalEvent {
    /// A goal appeared (its creation was committed).
    Goal(GoalSnapshot),
    /// A committed mutation changed the current goal.
    Change(GoalSnapshot),
}

impl GoalEvent {
    /// The full snapshot this event committed.
    pub fn snapshot(&self) -> &GoalSnapshot {
        match self {
            Self::Goal(snapshot) | Self::Change(snapshot) => snapshot,
        }
    }

    /// Stable event label (`goal` / `change`).
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Goal(_) => "goal",
            Self::Change(_) => "change",
        }
    }
}

/// Why a strict replay fold refused an event stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalFoldError {
    /// A `change` event arrived before any `goal` event.
    ChangeWithoutGoal,
    /// A committed mutation must advance the revision by exactly one.
    RevisionNotSequential { expected: u64, found: u64 },
    /// A `change` event switched goal identity without a `goal` event.
    IdentityChanged { expected: String, found: String },
    /// A new `goal` event arrived while the previous goal is unfinished.
    GoalWhileUnfinished { phase: GoalPhase },
}

impl fmt::Display for GoalFoldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChangeWithoutGoal => {
                write!(f, "goal fold: a change event arrived before any goal event")
            }
            Self::RevisionNotSequential { expected, found } => write!(
                f,
                "goal fold: revision must advance by exactly one (expected {expected}, found {found})"
            ),
            Self::IdentityChanged { expected, found } => write!(
                f,
                "goal fold: change switched goal identity ({expected} → {found}) without a goal event"
            ),
            Self::GoalWhileUnfinished { phase } => write!(
                f,
                "goal fold: a new goal event arrived while the previous goal is {}",
                phase.label()
            ),
        }
    }
}

impl std::error::Error for GoalFoldError {}

/// Strict replay fold: fold the full-snapshot `goal`/`change` event stream
/// into the single current goal state.
///
/// The fold is a pure function of its inputs — the same event stream always
/// folds to the same goal state — and it refuses any stream the machine could
/// not have produced: a `change` needs a `goal` before it, revisions advance
/// by exactly one per committed mutation, one stream carries one goal identity
/// at a time, and a new `goal` may only replace a completed one.
pub fn fold_goal_events(events: &[GoalEvent]) -> Result<Option<GoalSnapshot>, GoalFoldError> {
    let mut current: Option<GoalSnapshot> = None;
    for event in events {
        let snapshot = event.snapshot();
        match (&current, event) {
            (None, GoalEvent::Goal(_)) => current = Some(snapshot.clone()),
            (None, GoalEvent::Change(_)) => return Err(GoalFoldError::ChangeWithoutGoal),
            (Some(previous), GoalEvent::Goal(_)) => {
                if !previous.is_replaceable() {
                    return Err(GoalFoldError::GoalWhileUnfinished {
                        phase: previous.phase,
                    });
                }
                current = Some(snapshot.clone());
            }
            (Some(previous), GoalEvent::Change(_)) => {
                if snapshot.id != previous.id {
                    return Err(GoalFoldError::IdentityChanged {
                        expected: previous.id.clone(),
                        found: snapshot.id.clone(),
                    });
                }
                if snapshot.revision != previous.revision + 1 {
                    return Err(GoalFoldError::RevisionNotSequential {
                        expected: previous.revision + 1,
                        found: snapshot.revision,
                    });
                }
                current = Some(snapshot.clone());
            }
        }
    }
    Ok(current)
}

/// Typed goal-machine errors. Illegal transitions never mutate state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalError {
    NoGoal,
    AlreadyExists,
    IllegalTransition {
        from: GoalPhase,
        to: GoalPhase,
    },
    StaleRevision {
        expected: u64,
        actual: u64,
    },
    NoRoundsRemaining,
    /// The continuation drive is disarmed; no new goal-driven round starts.
    DriveDisarmed,
    /// A reservation names a different goal than the current one.
    ReservationGoalMismatch {
        expected: String,
        actual: String,
    },
    Persist(GoalPersistError),
}

/// Persist / restore failures. Distinct from state-machine errors so callers
/// can tell "illegal transition" from "disk wrote a truncated file".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalPersistError {
    Io {
        operation: &'static str,
        path: PathBuf,
        reason: String,
    },
    Serialization {
        reason: String,
    },
    Corrupt {
        id: String,
        reason: String,
    },
}

impl fmt::Display for GoalPersistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                reason,
            } => write!(
                f,
                "goal persist {operation} failed at {}: {reason}",
                path.display()
            ),
            Self::Serialization { reason } => write!(f, "goal persist serialization: {reason}"),
            Self::Corrupt { id, reason } => write!(f, "goal persist corrupt id={id}: {reason}"),
        }
    }
}

impl std::error::Error for GoalPersistError {}

impl fmt::Display for GoalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoGoal => write!(f, "no current goal"),
            Self::AlreadyExists => write!(f, "unfinished goal already exists"),
            Self::IllegalTransition { from, to } => {
                write!(
                    f,
                    "illegal goal transition {} → {}",
                    from.label(),
                    to.label()
                )
            }
            Self::StaleRevision { expected, actual } => {
                write!(
                    f,
                    "stale goal revision: expected {expected}, actual {actual}"
                )
            }
            Self::NoRoundsRemaining => write!(f, "no goal-driven rounds remaining"),
            Self::DriveDisarmed => write!(
                f,
                "continuation drive is disarmed; no goal-driven round starts"
            ),
            Self::ReservationGoalMismatch { expected, actual } => write!(
                f,
                "reservation bound to goal {expected}, current goal is {actual}"
            ),
            Self::Persist(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for GoalError {}

impl From<GoalPersistError> for GoalError {
    fn from(value: GoalPersistError) -> Self {
        Self::Persist(value)
    }
}

/// Crash-safe per-goal JSON store (`{sanitized-id}.json` via the shared atomic
/// durable writer).
///
/// This is a **file helper**, not a second session/transcript owner. One
/// [`GoalService`] holds at most one current snapshot.
pub struct GoalStore {
    dir: PathBuf,
}

impl GoalStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{}.json", sanitize_goal_id(id)))
    }

    /// Atomically persist a snapshot through the shared atomic writer's durable
    /// tier (goal state must not roll back after a crash): parent dir creation,
    /// exclusive same-directory temp file, write, `sync_all`, then replace.
    pub fn save(&self, g: &GoalSnapshot) -> Result<(), GoalPersistError> {
        let bytes = serde_json::to_vec_pretty(g).map_err(|e| GoalPersistError::Serialization {
            reason: e.to_string(),
        })?;
        let dest = self.path_for(&g.id);
        storage_atomic::write_atomic_durable(&dest, &bytes, storage_atomic::DEFAULT_FILE_MODE)
            .map_err(|e| persist_io("write goal snapshot", &dest, e))
    }

    pub fn load(&self, id: &str) -> Result<Option<GoalSnapshot>, GoalPersistError> {
        let path = self.path_for(id);
        match fs::read(&path) {
            Ok(bytes) => {
                let snap =
                    serde_json::from_slice(&bytes).map_err(|e| GoalPersistError::Corrupt {
                        id: id.to_string(),
                        reason: e.to_string(),
                    })?;
                Ok(Some(snap))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(persist_io("read goal snapshot", &path, e)),
        }
    }

    pub fn clear(&self, id: &str) -> Result<(), GoalPersistError> {
        let path = self.path_for(id);
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(persist_io("remove goal snapshot", &path, e)),
        }
    }

    /// Ids of `*.json` snapshots in the store dir (stems, not sanitized-back).
    /// Used after crash to find the single current goal without knowing the UUID.
    pub fn list_ids(&self) -> Result<Vec<String>, GoalPersistError> {
        let rd = match fs::read_dir(&self.dir) {
            Ok(rd) => rd,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(persist_io("list goal dir", &self.dir, e)),
        };
        let mut ids = Vec::new();
        for entry in rd {
            let entry = entry.map_err(|e| persist_io("read goal dir entry", &self.dir, e))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if !stem.contains(".tmp-") {
                    ids.push(stem.to_string());
                }
            }
        }
        ids.sort();
        Ok(ids)
    }
}

/// Single-current-goal service. Library only: no tokio interval, no organ
/// registration, no LLM.
pub struct GoalService {
    store: GoalStore,
    current: Option<GoalSnapshot>,
    /// Committed `goal`/`change` event stream (in-process, append-only).
    events: Vec<GoalEvent>,
}

impl GoalService {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            store: GoalStore::new(dir),
            current: None,
            events: Vec::new(),
        }
    }

    pub fn store(&self) -> &GoalStore {
        &self.store
    }

    /// The committed `goal`/`change` event stream of this service, in commit
    /// order. The stream is append-only; [`fold_goal_events`] replays it to
    /// the same goal state (`current` is a derived view of it).
    pub fn events(&self) -> &[GoalEvent] {
        &self.events
    }

    /// Restore a specific id from disk (crash recovery).
    pub fn restore(&mut self, id: &str) -> Result<Option<GoalSnapshot>, GoalError> {
        self.current = self.store.load(id)?;
        Ok(self.current.clone())
    }

    /// Restore the only snapshot in the store dir. Errors if more than one
    /// `*.json` is present — this machine is single-current by contract.
    pub fn restore_only(&mut self) -> Result<Option<GoalSnapshot>, GoalError> {
        let ids = self.store.list_ids()?;
        match ids.as_slice() {
            [] => {
                self.current = None;
                Ok(None)
            }
            [id] => self.restore(id),
            _ => Err(GoalPersistError::Corrupt {
                id: ids.join(","),
                reason: format!("expected at most one goal snapshot, found {}", ids.len()),
            }
            .into()),
        }
    }

    pub fn current(&self) -> Option<&GoalSnapshot> {
        self.current.as_ref()
    }

    /// Create a new active goal. Refuses if an unfinished goal already exists.
    /// `max_rounds` is clamped to at least 1. Revision starts at 1.
    pub fn create(
        &mut self,
        objective: impl Into<String>,
        max_rounds: u64,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        self.ensure_create_allowed()?;
        let g = GoalSnapshot {
            id: format!("goal-{}", TaskId::new()),
            revision: 0,
            objective: objective.into(),
            phase: GoalPhase::Active,
            max_goal_rounds: max_rounds.max(1),
            rounds_started: 0,
            blocked_reason: None,
            updated_at_ms: now_ms,
        };
        self.commit_new(g)
    }

    /// Create with a caller-supplied id (tests / restore-compatible fixtures).
    /// Id is still sanitized on disk.
    pub fn create_with_id(
        &mut self,
        id: impl Into<String>,
        objective: impl Into<String>,
        max_rounds: u64,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        self.ensure_create_allowed()?;
        let g = GoalSnapshot {
            id: id.into(),
            revision: 0,
            objective: objective.into(),
            phase: GoalPhase::Active,
            max_goal_rounds: max_rounds.max(1),
            rounds_started: 0,
            blocked_reason: None,
            updated_at_ms: now_ms,
        };
        self.commit_new(g)
    }

    /// Edit objective. Completed is not editable. CAS on `expected_revision`.
    pub fn edit(
        &mut self,
        expected_revision: u64,
        new_objective: impl Into<String>,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.cas_clone(expected_revision)?;
        if g.phase == GoalPhase::Completed {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: g.phase,
            });
        }
        g.objective = new_objective.into();
        g.updated_at_ms = now_ms;
        self.commit(g)
    }

    /// Active → Paused.
    pub fn pause(
        &mut self,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.cas_clone(expected_revision)?;
        if g.phase != GoalPhase::Active {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Paused,
            });
        }
        g.phase = GoalPhase::Paused;
        g.updated_at_ms = now_ms;
        self.commit(g)
    }

    /// Paused|Blocked → Active. Requires remaining round budget. Clears block.
    pub fn resume(
        &mut self,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.cas_clone(expected_revision)?;
        if !matches!(g.phase, GoalPhase::Paused | GoalPhase::Blocked) {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Active,
            });
        }
        if g.rounds_started >= g.max_goal_rounds {
            return Err(GoalError::NoRoundsRemaining);
        }
        g.phase = GoalPhase::Active;
        g.blocked_reason = None;
        g.updated_at_ms = now_ms;
        self.commit(g)
    }

    /// Any non-completed → Completed. Clears block.
    pub fn complete(
        &mut self,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.cas_clone(expected_revision)?;
        if g.phase == GoalPhase::Completed {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Completed,
            });
        }
        g.phase = GoalPhase::Completed;
        g.blocked_reason = None;
        g.updated_at_ms = now_ms;
        self.commit(g)
    }

    /// Active|Paused → Blocked, recording code+message.
    pub fn block(
        &mut self,
        expected_revision: u64,
        code: impl Into<String>,
        message: impl Into<String>,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.cas_clone(expected_revision)?;
        if !matches!(g.phase, GoalPhase::Active | GoalPhase::Paused) {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Blocked,
            });
        }
        g.phase = GoalPhase::Blocked;
        g.blocked_reason = Some(GoalBlock {
            code: code.into(),
            message: message.into(),
        });
        g.updated_at_ms = now_ms;
        self.commit(g)
    }

    /// Admit one goal-driven round. Ordinary human turns must not call this.
    /// Over budget → auto-block (`max-rounds`) then `NoRoundsRemaining`.
    pub fn admit_round(
        &mut self,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        let mut g = self.cas_clone(expected_revision)?;
        if g.phase != GoalPhase::Active {
            return Err(GoalError::IllegalTransition {
                from: g.phase,
                to: GoalPhase::Active,
            });
        }
        if g.rounds_started >= g.max_goal_rounds {
            g.phase = GoalPhase::Blocked;
            g.blocked_reason = Some(GoalBlock {
                code: "max-rounds".into(),
                message: "goal-driven round budget exhausted".into(),
            });
            g.updated_at_ms = now_ms;
            self.commit(g)?;
            return Err(GoalError::NoRoundsRemaining);
        }
        g.rounds_started += 1;
        g.updated_at_ms = now_ms;
        self.commit(g)
    }

    /// Drop the current goal (disk + memory). Idempotent.
    pub fn clear(&mut self) -> Result<(), GoalError> {
        if let Some(g) = self.current.take() {
            self.store.clear(&g.id)?;
        }
        Ok(())
    }

    /// Completed goals may be replaced; unfinished ones may not. Replacing
    /// deletes the previous snapshot file so [`Self::restore_only`] still sees
    /// a single current goal.
    ///
    /// L6: 顺序修正 — 旧实现先 `self.current.take()` 再 `store.clear()`:
    /// 若 clear 失败 (IO 错误), 内存里的 current 已被掏空 (自称"无目标") 而
    /// 磁盘快照仍在, `restore_only` 会把它读回来 → 内存/磁盘状态分裂,
    /// 且错误 propagates 时 current 已丢。现在先 clear、成功后才清内存,
    /// clear 失败则内存态保持原样 (错误可重试)。
    fn ensure_create_allowed(&mut self) -> Result<(), GoalError> {
        match &self.current {
            Some(g) if !g.is_replaceable() => Err(GoalError::AlreadyExists),
            Some(_) => {
                let prev_id = self.current.as_ref().ok_or(GoalError::NoGoal)?.id.clone();
                self.store.clear(&prev_id)?;
                self.current = None;
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn cas_clone(&self, expected_revision: u64) -> Result<GoalSnapshot, GoalError> {
        let g = self.current.clone().ok_or(GoalError::NoGoal)?;
        if g.revision != expected_revision {
            return Err(GoalError::StaleRevision {
                expected: expected_revision,
                actual: g.revision,
            });
        }
        Ok(g)
    }

    fn commit_new(&mut self, mut g: GoalSnapshot) -> Result<GoalSnapshot, GoalError> {
        g.revision += 1;
        self.store.save(&g)?;
        self.events.push(GoalEvent::Goal(g.clone()));
        self.current = Some(g.clone());
        Ok(g)
    }

    fn commit(&mut self, mut g: GoalSnapshot) -> Result<GoalSnapshot, GoalError> {
        g.revision += 1;
        self.store.save(&g)?;
        self.events.push(GoalEvent::Change(g.clone()));
        self.current = Some(g.clone());
        Ok(g)
    }
}

/// One reserved goal-driven round.
///
/// Reserving claims the next round number (`rounds_started + 1`) without
/// consuming quota or entering history. Only a reserved round whose message
/// actually enters history commits the reservation and consumes the number;
/// a stale reservation (the goal moved on after reserving) is refused at the
/// pre-step revision recheck and consumes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundReservation {
    goal_id: String,
    expected_revision: u64,
    round_number: u64,
}

impl RoundReservation {
    /// The goal this reservation is bound to.
    pub fn goal_id(&self) -> &str {
        &self.goal_id
    }

    /// The revision this reservation was taken at.
    pub const fn expected_revision(&self) -> u64 {
        self.expected_revision
    }

    /// The round number claimed (`rounds_started + 1` when reserved).
    pub const fn round_number(&self) -> u64 {
        self.round_number
    }
}

/// Whether a reserved round produced a message that entered history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundEntry {
    /// The round's message entered history: the round number is consumed.
    EnteredHistory,
    /// No history entry: the reservation is released without consuming.
    NoHistoryEntry,
}

/// The goal continuation drive: reserves and admits goal-driven rounds.
///
/// Reservation and admission are distinct on purpose. Reserving only claims
/// the next round number; the quota is consumed (and history advances) only
/// when the reserved round's message actually enters history. A stale
/// reservation — the goal moved on between reserving and committing — is
/// refused at the pre-step revision recheck and never consumes a round
/// number.
///
/// `armed`/`disarmed` is process state only. It is never persisted, and a
/// resumed goal starts disarmed: resuming a goal is a ceasefire, not an order
/// to keep firing rounds. Disarming stops *new* rounds; accounting still
/// follows history for a round whose message enters history, because the
/// budget counts what happened, not what the switch says.
#[derive(Debug, Clone, Default)]
pub struct GoalContinuationDrive {
    armed: bool,
    outstanding: Option<RoundReservation>,
}

impl GoalContinuationDrive {
    /// A fresh drive is disarmed (ceasefire by default).
    pub fn new() -> Self {
        Self::default()
    }

    /// Arm the drive: it may start new goal-driven rounds.
    pub fn arm(&mut self) {
        self.armed = true;
    }

    /// Disarm the drive (ceasefire): no new goal-driven round starts.
    pub fn disarm(&mut self) {
        self.armed = false;
    }

    /// Whether the drive may start new rounds.
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// The outstanding reservation, when one is held.
    pub fn outstanding(&self) -> Option<&RoundReservation> {
        self.outstanding.as_ref()
    }

    /// Resume the goal through the drive. A resumed goal starts disarmed
    /// (ceasefire by default) and any held reservation is dropped — resuming
    /// bumps the revision, so a held reservation is stale by construction.
    pub fn resume_goal(
        &mut self,
        goal: &mut GoalService,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        let snapshot = goal.resume(expected_revision, now_ms)?;
        self.armed = false;
        self.outstanding = None;
        Ok(snapshot)
    }

    /// Pre-step check before starting a round: the drive must be armed and
    /// the caller's revision must still be current. A stale handle is
    /// refused before anything changes.
    pub fn pre_step_check(
        &self,
        goal: &GoalService,
        expected_revision: u64,
    ) -> Result<(), GoalError> {
        if !self.armed {
            return Err(GoalError::DriveDisarmed);
        }
        let current = goal.current().ok_or(GoalError::NoGoal)?;
        if current.revision != expected_revision {
            return Err(GoalError::StaleRevision {
                expected: expected_revision,
                actual: current.revision,
            });
        }
        Ok(())
    }

    /// Reserve the next round number (`rounds_started + 1`). Reserving claims
    /// the number only: no quota is consumed, nothing enters history, and
    /// nothing is persisted.
    pub fn reserve_round(
        &mut self,
        goal: &GoalService,
        expected_revision: u64,
    ) -> Result<RoundReservation, GoalError> {
        self.pre_step_check(goal, expected_revision)?;
        let current = goal.current().ok_or(GoalError::NoGoal)?;
        if current.phase != GoalPhase::Active {
            return Err(GoalError::IllegalTransition {
                from: current.phase,
                to: GoalPhase::Active,
            });
        }
        if current.rounds_started >= current.max_goal_rounds {
            return Err(GoalError::NoRoundsRemaining);
        }
        let reservation = RoundReservation {
            goal_id: current.id.clone(),
            expected_revision,
            round_number: current.rounds_started + 1,
        };
        self.outstanding = Some(reservation.clone());
        Ok(reservation)
    }

    /// Commit a reservation: the round's message entered history, so the
    /// claimed round number is consumed through the existing CAS-guarded
    /// admission. The pre-step recheck refuses a stale reservation and
    /// consumes nothing.
    pub fn commit_round(
        &mut self,
        goal: &mut GoalService,
        reservation: &RoundReservation,
        now_ms: i64,
    ) -> Result<GoalSnapshot, GoalError> {
        self.revalidate(goal, reservation)?;
        let snapshot = goal.admit_round(reservation.expected_revision, now_ms)?;
        if self.outstanding.as_ref() == Some(reservation) {
            self.outstanding = None;
        }
        Ok(snapshot)
    }

    /// Release a reservation whose message never entered history: the round
    /// number is not consumed and history does not move.
    pub fn release_round(&mut self, reservation: &RoundReservation) {
        if self.outstanding.as_ref() == Some(reservation) {
            self.outstanding = None;
        }
    }

    /// Drive one goal-driven round: reserve the round number, run `body`, and
    /// consume the number only when `body` reports a history entry. A round
    /// without a history entry releases its reservation untouched.
    pub fn drive_round<F>(
        &mut self,
        goal: &mut GoalService,
        expected_revision: u64,
        now_ms: i64,
        body: F,
    ) -> Result<Option<GoalSnapshot>, GoalError>
    where
        F: FnOnce(&RoundReservation) -> RoundEntry,
    {
        let reservation = self.reserve_round(goal, expected_revision)?;
        match body(&reservation) {
            RoundEntry::EnteredHistory => self.commit_round(goal, &reservation, now_ms).map(Some),
            RoundEntry::NoHistoryEntry => {
                self.release_round(&reservation);
                Ok(None)
            }
        }
    }

    /// Recheck freshness before consuming: the reservation must still match
    /// the current goal and revision. A stale reservation is refused and
    /// consumes nothing.
    fn revalidate(
        &self,
        goal: &GoalService,
        reservation: &RoundReservation,
    ) -> Result<(), GoalError> {
        let current = goal.current().ok_or(GoalError::NoGoal)?;
        if current.id != reservation.goal_id {
            return Err(GoalError::ReservationGoalMismatch {
                expected: reservation.goal_id.clone(),
                actual: current.id.clone(),
            });
        }
        if current.revision != reservation.expected_revision {
            return Err(GoalError::StaleRevision {
                expected: reservation.expected_revision,
                actual: current.revision,
            });
        }
        Ok(())
    }
}

/// ASCII alphanumerics plus `-`/`_`, max 120 chars. Empty → `"goal"`.
///
/// L6: 非 ASCII 碰撞后缀 — 纯中文/emoji id ("目标一" / "目标二" / "🎯") 净化后
/// 会**全部**收敛成 `"goal"` (字符被全部剔除 → 空 → 兜底), 两个不同目标互相
/// 覆盖磁盘快照。对被剥离过字符的 id 追加原始串的 FNV-1a 短哈希后缀,
/// 保证可区分 (总长仍有界: base ≤ 100 + "-" + 8 hex = 109 < 120)。
fn sanitize_goal_id(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(100)
        .collect();
    let base = if cleaned.is_empty() {
        "goal".to_string()
    } else {
        cleaned
    };
    if id == base {
        // 本来就是安全 id, 0 追加 (保持既有文件名兼容)。
        return base;
    }
    // L6: 有字符被剥离 (非 ASCII / 标点 / 超长) → 追加原串短哈希区分碰撞。
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in id.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{base}-{:08x}", hash & 0xffff_ffff)
}

fn persist_io(operation: &'static str, path: &Path, err: io::Error) -> GoalPersistError {
    GoalPersistError::Io {
        operation,
        path: path.to_path_buf(),
        reason: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp(tag: &str) -> PathBuf {
        let n = TEST_SEQ.fetch_add(1, Ordering::Relaxed);
        let d =
            std::env::temp_dir().join(format!("apeireth-goal-{tag}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    fn rev(s: &GoalService) -> u64 {
        s.current().unwrap().revision
    }

    #[test]
    fn create_edit_lifecycle() {
        let mut s = GoalService::new(tmp("life"));
        let g = s.create("learn substitution", 8, 1_000).unwrap();
        assert_eq!(g.revision, 1);
        assert_eq!(g.phase, GoalPhase::Active);
        assert_eq!(g.max_goal_rounds, 8);
        let g2 = s.edit(rev(&s), "learn substitution + rank", 1_001).unwrap();
        assert_eq!(g2.revision, 2);
        assert_eq!(g2.phase, GoalPhase::Active);
        assert_eq!(
            s.create("x", 1, 1_002).unwrap_err(),
            GoalError::AlreadyExists
        );
    }

    #[test]
    fn max_rounds_clamped_to_one() {
        let mut s = GoalService::new(tmp("clamp"));
        let g = s.create("x", 0, 1).unwrap();
        assert_eq!(g.max_goal_rounds, 1);
    }

    #[test]
    fn pause_resume_block_complete() {
        let mut s = GoalService::new(tmp("prbc"));
        s.create("goal", 3, 10).unwrap();
        let p = s.pause(rev(&s), 11).unwrap();
        assert_eq!(p.phase, GoalPhase::Paused);
        assert_eq!(
            s.pause(rev(&s), 12).unwrap_err(),
            GoalError::IllegalTransition {
                from: GoalPhase::Paused,
                to: GoalPhase::Paused
            }
        );
        let b = s
            .block(rev(&s), "provider-limit", "rate limited", 13)
            .unwrap();
        assert_eq!(b.phase, GoalPhase::Blocked);
        assert_eq!(b.blocked_reason.as_ref().unwrap().code, "provider-limit");
        let r = s.resume(rev(&s), 14).unwrap();
        assert_eq!(r.phase, GoalPhase::Active);
        assert!(r.blocked_reason.is_none(), "resume clears block");
        let c = s.complete(rev(&s), 15).unwrap();
        assert_eq!(c.phase, GoalPhase::Completed);
        assert!(s.edit(rev(&s), "x", 16).is_err());
        let prev_id = s.current().unwrap().id.clone();
        let g2 = s.create("new goal", 2, 17).unwrap();
        assert_eq!(g2.phase, GoalPhase::Active);
        assert_eq!(g2.revision, 1);
        assert_ne!(g2.id, prev_id);
        assert_eq!(s.store().list_ids().unwrap().len(), 1);
    }

    #[test]
    fn rounds_budget_blocks_at_max() {
        let mut s = GoalService::new(tmp("rounds"));
        s.create("goal", 2, 1).unwrap();
        s.admit_round(rev(&s), 2).unwrap();
        s.admit_round(rev(&s), 3).unwrap();
        assert_eq!(
            s.admit_round(rev(&s), 4).unwrap_err(),
            GoalError::NoRoundsRemaining
        );
        let cur = s.current().unwrap();
        assert_eq!(cur.phase, GoalPhase::Blocked);
        assert_eq!(cur.blocked_reason.as_ref().unwrap().code, "max-rounds");
        assert_eq!(cur.rounds_started, 2);
        // resume with exhausted budget is refused and leaves Blocked
        assert_eq!(
            s.resume(rev(&s), 5).unwrap_err(),
            GoalError::NoRoundsRemaining
        );
        assert_eq!(s.current().unwrap().phase, GoalPhase::Blocked);
    }

    #[test]
    fn admit_round_refuses_when_not_active() {
        let mut s = GoalService::new(tmp("admit-paused"));
        s.create("goal", 3, 1).unwrap();
        s.pause(rev(&s), 2).unwrap();
        assert_eq!(
            s.admit_round(rev(&s), 3).unwrap_err(),
            GoalError::IllegalTransition {
                from: GoalPhase::Paused,
                to: GoalPhase::Active
            }
        );
        assert_eq!(s.current().unwrap().rounds_started, 0);
    }

    #[test]
    fn stale_revision_is_rejected_and_does_not_mutate() {
        let mut s = GoalService::new(tmp("cas"));
        let g = s.create("goal", 4, 1).unwrap();
        assert_eq!(g.revision, 1);
        let err = s.pause(0, 2).unwrap_err();
        assert_eq!(
            err,
            GoalError::StaleRevision {
                expected: 0,
                actual: 1
            }
        );
        assert_eq!(s.current().unwrap().phase, GoalPhase::Active);
        assert_eq!(s.current().unwrap().revision, 1);
        s.pause(1, 3).unwrap();
        assert_eq!(s.current().unwrap().phase, GoalPhase::Paused);
        assert_eq!(s.current().unwrap().revision, 2);
    }

    #[test]
    fn illegal_transition_does_not_bump_revision() {
        let mut s = GoalService::new(tmp("no-bump"));
        s.create("goal", 2, 1).unwrap();
        s.complete(rev(&s), 2).unwrap();
        let before = s.current().cloned().unwrap();
        assert!(s.edit(before.revision, "nope", 3).is_err());
        let after = s.current().unwrap();
        assert_eq!(after.revision, before.revision);
        assert_eq!(after.objective, before.objective);
        assert_eq!(after.phase, GoalPhase::Completed);
    }

    #[test]
    fn persistence_survives_restart() {
        let dir = tmp("persist");
        let mut s1 = GoalService::new(&dir);
        s1.create_with_id("goal-fixed", "across restart", 5, 42)
            .unwrap();
        drop(s1);

        let mut s2 = GoalService::new(&dir);
        assert!(s2.restore("goal-").unwrap().is_none());
        let g = s2.restore_only().unwrap().unwrap();
        assert_eq!(g.objective, "across restart");
        assert_eq!(g.revision, 1);
        assert_eq!(g.updated_at_ms, 42);
        let g2 = s2.admit_round(g.revision, 43).unwrap();
        assert_eq!(g2.rounds_started, 1);
        assert_eq!(g2.revision, 2);
    }

    #[test]
    fn overwrite_persist_replaces_same_id() {
        let dir = tmp("overwrite");
        let mut s = GoalService::new(&dir);
        s.create_with_id("goal-same", "one", 3, 1).unwrap();
        s.pause(rev(&s), 2).unwrap();
        let mut s2 = GoalService::new(&dir);
        let g = s2.restore("goal-same").unwrap().unwrap();
        assert_eq!(g.phase, GoalPhase::Paused);
        assert_eq!(g.revision, 2);
        assert_eq!(g.objective, "one");
    }

    #[test]
    fn clear_removes_goal_and_file() {
        let dir = tmp("clear");
        let mut s = GoalService::new(&dir);
        s.create_with_id("goal-x", "x", 1, 1).unwrap();
        assert_eq!(s.store().list_ids().unwrap(), vec!["goal-x".to_string()]);
        s.clear().unwrap();
        assert!(s.current().is_none());
        assert!(s.store().list_ids().unwrap().is_empty());
        s.create("y", 1, 2).unwrap();
        assert_eq!(s.current().unwrap().objective, "y");
    }

    #[test]
    fn path_escape_id_is_sanitized() {
        let dir = tmp("sanitize");
        let mut s = GoalService::new(&dir);
        s.create_with_id("../evil", "obj", 1, 1).unwrap();
        let ids = s.store().list_ids().unwrap();
        // L6: 净化后是 "evil" + 短哈希后缀 (原 id 被剥离过字符 → 追加后缀防
        // 非 ASCII 碰撞); 关键是**不逃逸**且单条目。
        assert_eq!(ids.len(), 1, "只应有一个目标: {ids:?}");
        assert!(
            ids[0].starts_with("evil"),
            "../evil 应净化为 evil 前缀, 得到 {ids:?}"
        );
        assert!(!ids[0].contains(".."), "不得保留路径穿越段: {ids:?}");
        // nothing written outside the store root
        assert!(dir.join(format!("{}.json", ids[0])).is_file());
        assert!(
            !dir.parent().unwrap().join("evil.json").is_file(),
            "store 根外不得落文件"
        );
    }

    /// L6: 纯非 ASCII id 不得互相碰撞 ("目标一"/"目标二" 旧实现都收敛到 "goal")。
    #[test]
    fn non_ascii_ids_do_not_collide() {
        let a = sanitize_goal_id("目标一");
        let b = sanitize_goal_id("目标二");
        let c = sanitize_goal_id("🎯");
        assert_ne!(a, b, "不同中文 id 必须可区分");
        assert_ne!(a, c);
        assert_ne!(b, c);
        assert!(a.starts_with("goal"), "剥离后应回落 goal 兜底: {a}");
        // 安全 id 不追加后缀 (保持既有文件名兼容)。
        assert_eq!(sanitize_goal_id("goal-x"), "goal-x");
    }

    #[test]
    fn no_goal_operations_error() {
        let mut s = GoalService::new(tmp("empty"));
        assert_eq!(s.pause(0, 1).unwrap_err(), GoalError::NoGoal);
        assert_eq!(s.complete(0, 1).unwrap_err(), GoalError::NoGoal);
        assert_eq!(s.admit_round(0, 1).unwrap_err(), GoalError::NoGoal);
        s.clear().unwrap();
    }

    #[test]
    fn serde_round_trip_snapshot() {
        let snap = GoalSnapshot {
            id: "goal-a".into(),
            revision: 3,
            objective: "ship".into(),
            phase: GoalPhase::Blocked,
            max_goal_rounds: 4,
            rounds_started: 2,
            blocked_reason: Some(GoalBlock {
                code: "max-rounds".into(),
                message: "exhausted".into(),
            }),
            updated_at_ms: 99,
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: GoalSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, snap);
        assert!(json.contains("\"blocked\""));
    }

    #[test]
    fn restore_only_rejects_two_snapshots() {
        let dir = tmp("two");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.json"), b"{}").unwrap();
        fs::write(dir.join("b.json"), b"{}").unwrap();
        let mut s = GoalService::new(&dir);
        assert!(matches!(
            s.restore_only().unwrap_err(),
            GoalError::Persist(GoalPersistError::Corrupt { .. })
        ));
    }

    fn snap(id: &str, revision: u64, phase: GoalPhase) -> GoalSnapshot {
        GoalSnapshot {
            id: id.into(),
            revision,
            objective: "objective".into(),
            phase,
            max_goal_rounds: 4,
            rounds_started: 0,
            blocked_reason: None,
            updated_at_ms: 1,
        }
    }

    /// fold 确定性: 同一事件流确定折叠同一目标态, 状态是事件流的派生视图。
    #[test]
    fn fold_goal_events_replays_one_stream_to_one_state() {
        let mut s = GoalService::new(tmp("fold"));
        s.create("ship the fold", 4, 1).unwrap();
        s.edit(rev(&s), "ship the fold, strictly", 2).unwrap();
        s.pause(rev(&s), 3).unwrap();

        let folded = fold_goal_events(s.events())
            .unwrap()
            .expect("the stream carries a goal");
        assert_eq!(folded, s.current().cloned().unwrap());
        assert_eq!(folded.revision, 3);

        // 同一事件流再次折叠 = 同一目标态。
        let again = fold_goal_events(s.events()).unwrap().unwrap();
        assert_eq!(folded, again);
        let replay: Vec<GoalEvent> = s.events().to_vec();
        assert_eq!(fold_goal_events(&replay).unwrap().unwrap(), folded);

        // 空流折叠为 None (不发明目标)。
        assert!(fold_goal_events(&[]).unwrap().is_none());
    }

    /// 严格回放: 机器产不出的事件流必须被拒, 不做静默修补。
    #[test]
    fn fold_goal_events_refuses_streams_the_machine_cannot_produce() {
        let a = snap("goal-a", 1, GoalPhase::Active);
        let mut b = snap("goal-b", 1, GoalPhase::Active);

        // change 先于 goal: 拒。
        assert_eq!(
            fold_goal_events(&[GoalEvent::Change(a.clone())]),
            Err(GoalFoldError::ChangeWithoutGoal)
        );

        // revision 必须严格 +1。
        let mut jumped = a.clone();
        jumped.revision = 3;
        assert_eq!(
            fold_goal_events(&[GoalEvent::Goal(a.clone()), GoalEvent::Change(jumped)]),
            Err(GoalFoldError::RevisionNotSequential {
                expected: 2,
                found: 3
            })
        );

        // 同一事件流不换目标身份。
        assert_eq!(
            fold_goal_events(&[GoalEvent::Goal(a.clone()), GoalEvent::Change(b.clone())]),
            Err(GoalFoldError::IdentityChanged {
                expected: "goal-a".into(),
                found: "goal-b".into()
            })
        );

        // 未完成目标不得被新 goal 顶替。
        assert_eq!(
            fold_goal_events(&[GoalEvent::Goal(a.clone()), GoalEvent::Goal(b.clone())]),
            Err(GoalFoldError::GoalWhileUnfinished {
                phase: GoalPhase::Active
            })
        );

        // 完成之后可以立新目标 (create 语义一致)。
        let done = snap("goal-a", 2, GoalPhase::Completed);
        b.revision = 1;
        assert!(fold_goal_events(&[
            GoalEvent::Goal(a),
            GoalEvent::Change(done),
            GoalEvent::Goal(b)
        ])
        .is_ok());
    }

    /// 陈旧 revision 拒绝: pre-step 复核在先, 不产生任何预留。
    #[test]
    fn a_stale_revision_is_rejected_before_reserving() {
        let mut s = GoalService::new(tmp("pre-step"));
        s.create("goal", 3, 1).unwrap();
        let mut drive = GoalContinuationDrive::new();
        drive.arm();

        assert_eq!(
            drive.reserve_round(&s, 0).unwrap_err(),
            GoalError::StaleRevision {
                expected: 0,
                actual: 1
            }
        );
        assert!(drive.outstanding().is_none(), "拒绝的请求不留下预留");

        let reservation = drive.reserve_round(&s, 1).unwrap();
        assert_eq!(reservation.round_number(), 1);
    }

    /// 预留不入史不耗号: 预留只占轮号, 状态/磁盘/事件流都不动。
    #[test]
    fn reserving_alone_never_enters_history_or_consumes_a_round() {
        let mut s = GoalService::new(tmp("reserve-only"));
        s.create_with_id("goal-r", "goal", 3, 1).unwrap();
        let before = s.current().cloned().unwrap();
        let mut drive = GoalContinuationDrive::new();
        drive.arm();

        let reservation = drive.reserve_round(&s, before.revision).unwrap();
        assert_eq!(reservation.round_number(), before.rounds_started + 1);

        assert_eq!(s.current().cloned().unwrap(), before, "预留不动内存态");
        assert_eq!(s.events().len(), 1, "预留不是事件");
        assert_eq!(
            s.store().load("goal-r").unwrap().unwrap(),
            before,
            "预留不落盘"
        );

        // 未入史的预留直接释放: 仍然零消耗。
        drive.release_round(&reservation);
        assert_eq!(s.current().cloned().unwrap(), before);
        assert_eq!(s.current().unwrap().rounds_started, 0);
    }

    /// 真入史才计数: 只有消息真正进入历史的轮才消耗轮号。
    #[test]
    fn a_round_number_is_consumed_only_when_the_message_enters_history() {
        let mut s = GoalService::new(tmp("admit"));
        s.create("goal", 3, 1).unwrap();
        let mut drive = GoalContinuationDrive::new();
        drive.arm();

        // 消息未入史: 驱动不计数。
        let none = drive
            .drive_round(&mut s, 1, 2, |_| RoundEntry::NoHistoryEntry)
            .unwrap();
        assert!(none.is_none());
        assert_eq!(s.current().unwrap().rounds_started, 0);

        // 消息入史: 预留的轮号被消耗。
        let admitted = drive
            .drive_round(&mut s, 1, 3, |reservation| {
                assert_eq!(reservation.round_number(), 1);
                RoundEntry::EnteredHistory
            })
            .unwrap()
            .expect("入史轮提交");
        assert_eq!(admitted.rounds_started, 1);
        assert_eq!(admitted.revision, 2);
        assert!(drive.outstanding().is_none());
    }

    /// 陈旧预留不耗号: 预留后目标前进 → 提交被拒, 轮号不消耗。
    #[test]
    fn a_stale_reservation_consumes_no_round_number() {
        let mut s = GoalService::new(tmp("stale-res"));
        s.create("goal", 3, 1).unwrap();
        let mut drive = GoalContinuationDrive::new();
        drive.arm();
        let reservation = drive.reserve_round(&s, 1).unwrap();

        // 目标在预留之后前进了 (revision 1 → 2)。
        s.edit(rev(&s), "goal, revised", 2).unwrap();

        assert_eq!(
            drive.commit_round(&mut s, &reservation, 3).unwrap_err(),
            GoalError::StaleRevision {
                expected: 1,
                actual: 2
            }
        );
        assert_eq!(s.current().unwrap().rounds_started, 0, "陈旧预留不耗号");

        // 重新预留后正常入史计数。
        let fresh = drive.reserve_round(&s, 2).unwrap();
        assert_eq!(fresh.round_number(), 1);
        let admitted = drive.commit_round(&mut s, &fresh, 4).unwrap();
        assert_eq!(admitted.rounds_started, 1);
    }

    /// resume 后默认停火 + armed/disarmed 不持久化。
    #[test]
    fn resume_disarms_the_drive_and_arming_is_never_persisted() {
        let mut s = GoalService::new(tmp("disarm"));
        s.create("goal", 3, 1).unwrap();
        let mut drive = GoalContinuationDrive::new();
        drive.arm();
        drive.reserve_round(&s, 1).unwrap();
        s.pause(rev(&s), 2).unwrap();

        // resume 后默认停火: 持票预留作废, 不再发新轮。
        drive.resume_goal(&mut s, 2, 3).unwrap();
        assert!(!drive.is_armed(), "resume 后默认停火");
        assert!(drive.outstanding().is_none(), "恢复后旧预留作废");
        assert_eq!(
            drive.reserve_round(&s, rev(&s)).unwrap_err(),
            GoalError::DriveDisarmed
        );

        // armed/disarmed 不持久化: 快照里没有开关, 重建即停火。
        let json = serde_json::to_string(s.current().unwrap()).unwrap();
        assert!(!json.contains("armed"), "武装状态不得进快照: {json}");
        assert!(!json.contains("outstanding"), "预留不得进快照: {json}");
        let restored = GoalContinuationDrive::new();
        assert!(!restored.is_armed());
    }

    /// 与既有 CAS 测试共存零回归: 预留-准入与 admit_round 同一 revision 规则。
    #[test]
    fn the_reservation_flow_composes_with_existing_cas_guards() {
        let mut s = GoalService::new(tmp("cas-coexist"));
        s.create("goal", 4, 1).unwrap();
        let mut drive = GoalContinuationDrive::new();
        drive.arm();

        // 既有 CAS 语义不变: admit_round 仍严格核对 revision。
        assert_eq!(
            s.admit_round(0, 2).unwrap_err(),
            GoalError::StaleRevision {
                expected: 0,
                actual: 1
            }
        );
        s.admit_round(1, 2).unwrap();

        // 驱动预留绑 revision; 另一持票人先消耗一轮 → 预留变陈旧。
        let reservation = drive.reserve_round(&s, 2).unwrap();
        assert_eq!(reservation.round_number(), 2);
        s.admit_round(2, 3).unwrap();
        assert_eq!(
            drive.commit_round(&mut s, &reservation, 4).unwrap_err(),
            GoalError::StaleRevision {
                expected: 2,
                actual: 3
            }
        );
        assert_eq!(s.current().unwrap().rounds_started, 2);
    }
}
