//! Goal state machine + crash-safe persist (library, default-off).
//!
//! Canonical implementation module. (DSH-style
//! single-current-goal machine). This is a **mechanism**: one current goal,
//! guarded phase transitions, revision CAS, `rounds_started` only on
//! goal-driven turns, tmp+rename persist. It is **not** a round driver, not
//! a daemon, and not a second agent loop.
//!
//! Semantics kept from the canonical:
//! - Single current goal. `create` refuses while a non-completed goal exists.
//! - `revision` is strictly +1 on every committed mutation.
//! - Illegal phase transitions leave state unchanged.
//! - One blocked phase with `blocked_reason { code, message }` (no extra states).
//! - `rounds_started` increments only via [`GoalService::admit_round`].
//! - Hitting `max_goal_rounds` auto-blocks with code `max-rounds`.
//!
//! Honest adaptations vs the canonical:
//! - Engine claimed compare-and-set (`StaleRevision`) but never compared an
//!   expected revision. This port **enforces** CAS: every mutation takes the
//!   caller's expected revision and rejects a stale handle.
//! - Engine swallowed persist errors (`let _ = store.save`). This port surfaces
//!   typed I/O / serialization errors; in-memory state is not advanced if
//!   persist fails.
//! - No `uuid` / `chrono` crate deps. Ids are minted from
//!   [`apeireth_core::kernel::TaskId`]; timestamps are injected (`now_ms`).
//! - Persist filenames are sanitized (no path escape). tmp names do not embed
//!   the raw id.
//!
//! Production wiring: none. Callers that want a Goal organ later compose this
//! library behind `OrganTrait`; this module does not register, tick, or speak.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use apeireth_core::kernel::TaskId;
use serde::{Deserialize, Serialize};

/// Monotonic nonce so tmp filenames stay unique without a uuid crate.
static TMP_NONCE: AtomicU64 = AtomicU64::new(1);

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

/// Typed goal-machine errors. Illegal transitions never mutate state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalError {
    NoGoal,
    AlreadyExists,
    IllegalTransition { from: GoalPhase, to: GoalPhase },
    StaleRevision { expected: u64, actual: u64 },
    NoRoundsRemaining,
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

/// Crash-safe per-goal JSON store (`{sanitized-id}.json` via tmp+rename).
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

    /// Atomically persist a snapshot. On POSIX, `rename` replaces. On Windows,
    /// existing dest is unlinked then renamed (documented crash window: dest
    /// briefly absent). tmp is removed on any failure after create.
    pub fn save(&self, g: &GoalSnapshot) -> Result<(), GoalPersistError> {
        fs::create_dir_all(&self.dir).map_err(|e| persist_io("create goal dir", &self.dir, e))?;
        let nonce = TMP_NONCE.fetch_add(1, Ordering::Relaxed);
        let tmp = self
            .dir
            .join(format!("{}.tmp-{nonce}", sanitize_goal_id(&g.id)));
        let bytes = serde_json::to_vec_pretty(g).map_err(|e| GoalPersistError::Serialization {
            reason: e.to_string(),
        })?;
        let write_result = (|| -> Result<(), GoalPersistError> {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp)
                .map_err(|e| persist_io("open goal tmp", &tmp, e))?;
            file.write_all(&bytes)
                .map_err(|e| persist_io("write goal tmp", &tmp, e))?;
            file.sync_all()
                .map_err(|e| persist_io("sync goal tmp", &tmp, e))?;
            Ok(())
        })();
        if let Err(err) = write_result {
            let _ = fs::remove_file(&tmp);
            return Err(err);
        }
        let dest = self.path_for(&g.id);
        if let Err(err) = atomic_replace(&tmp, &dest) {
            let _ = fs::remove_file(&tmp);
            return Err(err);
        }
        Ok(())
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
}

impl GoalService {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            store: GoalStore::new(dir),
            current: None,
        }
    }

    pub fn store(&self) -> &GoalStore {
        &self.store
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
    fn ensure_create_allowed(&mut self) -> Result<(), GoalError> {
        match &self.current {
            Some(g) if !g.is_replaceable() => Err(GoalError::AlreadyExists),
            Some(_) => {
                let prev = self.current.take().expect("just matched Some");
                self.store.clear(&prev.id)?;
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
        self.current = Some(g.clone());
        Ok(g)
    }

    fn commit(&mut self, mut g: GoalSnapshot) -> Result<GoalSnapshot, GoalError> {
        g.revision += 1;
        self.store.save(&g)?;
        self.current = Some(g.clone());
        Ok(g)
    }
}

/// ASCII alphanumerics plus `-`/`_`, max 120 chars. Empty → `"goal"`.
fn sanitize_goal_id(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(120)
        .collect();
    if cleaned.is_empty() {
        "goal".to_string()
    } else {
        cleaned
    }
}

fn persist_io(operation: &'static str, path: &Path, err: io::Error) -> GoalPersistError {
    GoalPersistError::Io {
        operation,
        path: path.to_path_buf(),
        reason: err.to_string(),
    }
}

fn atomic_replace(tmp: &Path, dest: &Path) -> Result<(), GoalPersistError> {
    match fs::rename(tmp, dest) {
        Ok(()) => Ok(()),
        Err(err) => {
            // Windows cannot rename over an existing file. Unlink dest then
            // retry. There is a brief window where dest is absent; documented
            // as a platform limitation, not pretended to be POSIX-atomic.
            if dest.exists() {
                fs::remove_file(dest).map_err(|e| persist_io("replace-remove dest", dest, e))?;
                fs::rename(tmp, dest).map_err(|e| persist_io("replace-rename", dest, e))?;
                Ok(())
            } else {
                Err(persist_io("rename goal snapshot", dest, err))
            }
        }
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
        assert_eq!(ids, vec!["evil".to_string()]);
        // nothing written outside the store root
        assert!(dir.join("evil.json").is_file());
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
}
