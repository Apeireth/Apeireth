//! Observation-capture queue recovered from companion `observer_capture.rs`.
//!
//! Engine behaviour kept:
//! - candidate = `{tool, args_hash, outcome, ts_ms, source}`
//! - success/failure summaries truncated to 200 Unicode scalars
//! - 24h dedup on `(tool, args_hash)`
//! - in-memory LRU index (cap 1024) independent of the pending FIFO
//!
//! Engine behaviour discarded:
//! - SQLite persistence via `episodes` (`expc-` prefix) — that is a second
//!   memory store; v2 memory already owns episodes
//! - `PostExecuteHook` / `ToolBridge` wiring — old tool-runtime authority
//!
//! Hash adaptation: canonical used SHA-256 truncated to 16 hex chars. This crate
//! does not take a new `sha2` dependency; FNV-1a 64-bit hex is the same length
//! and is documented as a salvage hash, not a cryptographic claim.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::normalize::now_timestamp_ms;

/// Maximum Unicode scalars kept in an outcome summary.
pub const MAX_SUMMARY_CHARS: usize = 200;

/// Default dedup window (24 hours, milliseconds).
pub const DEFAULT_DEDUP_WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

/// Default LRU capacity for the `(tool, args_hash)` index.
pub const DEFAULT_LRU_CAP: usize = 1024;

/// FNV-1a 64-bit offset basis.
const FNV64_OFFSET: u64 = 0xcbf29ce484222325;
/// FNV-1a 64-bit prime.
const FNV64_PRIME: u64 = 0x100000001b3;

/// Candidate experience source. Engine only shipped `ToolExecution`; the enum
/// is kept so a later owner can add Dialog / Reflection without a schema break.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSource {
    /// Captured from a tool execution result.
    ToolExecution,
}

/// Tool-execution outcome summary (the observation signal, not the full result).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ObservationOutcome {
    /// Success + truncated product summary.
    Success { summary: Option<String> },
    /// Failure + truncated error summary.
    Failure { error: Option<String> },
}

impl ObservationOutcome {
    /// Truncate `s` to [`MAX_SUMMARY_CHARS`] Unicode scalars, appending `…`
    /// when clipped.
    pub fn truncate_summary(s: &str) -> String {
        let mut out: String = s.chars().take(MAX_SUMMARY_CHARS).collect();
        if s.chars().count() > MAX_SUMMARY_CHARS {
            out.push('…');
        }
        out
    }

    /// Derive an outcome from a success flag plus optional output / error text.
    pub fn from_result(success: bool, output: Option<&str>, error: Option<&str>) -> Self {
        if success {
            let raw = output.unwrap_or("ok (null output)");
            Self::Success {
                summary: Some(Self::truncate_summary(raw)),
            }
        } else {
            Self::Failure {
                error: Some(Self::truncate_summary(error.unwrap_or("unknown error"))),
            }
        }
    }

    /// Three-way label aligned with canonical `experience::Experience.outcome`.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Success { .. } => "success",
            Self::Failure { .. } => "failure",
        }
    }
}

/// One observation candidate waiting for a later promote / drain cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationCandidate {
    /// Tool or producer name.
    pub tool: String,
    /// Canonical args digest (16 hex chars).
    pub args_hash: String,
    /// Success / failure summary.
    pub outcome: ObservationOutcome,
    /// Event time (epoch millis).
    pub ts_ms: i64,
    /// How the candidate was produced.
    pub source: ObservationSource,
}

/// Queue configuration.
#[derive(Debug, Clone)]
pub struct ObservationQueueConfig {
    /// Dedup window in milliseconds.
    pub window_ms: i64,
    /// LRU index capacity (does **not** cap the pending FIFO).
    pub lru_cap: usize,
}

impl Default for ObservationQueueConfig {
    fn default() -> Self {
        Self {
            window_ms: DEFAULT_DEDUP_WINDOW_MS,
            lru_cap: DEFAULT_LRU_CAP,
        }
    }
}

struct Inner {
    lru: HashMap<(String, String), i64>,
    pending: Vec<ObservationCandidate>,
    order: VecDeque<(String, String)>,
    lru_cap: usize,
}

/// In-memory observation queue: LRU dedup index + pending FIFO.
pub struct ObservationQueue {
    inner: Mutex<Inner>,
    window_ms: i64,
}

impl ObservationQueue {
    /// Empty queue with default window / LRU cap.
    pub fn new() -> Self {
        Self::with_config(ObservationQueueConfig::default())
    }

    /// Construct with an explicit config.
    pub fn with_config(cfg: ObservationQueueConfig) -> Self {
        Self {
            inner: Mutex::new(Inner {
                lru: HashMap::new(),
                pending: Vec::new(),
                order: VecDeque::new(),
                lru_cap: cfg.lru_cap,
            }),
            window_ms: cfg.window_ms,
        }
    }

    /// Push using wall-clock time. `true` = enqueued, `false` = dedup hit.
    pub fn push(&self, candidate: ObservationCandidate) -> bool {
        self.push_at(candidate, now_timestamp_ms())
    }

    /// Time-injected push (tests).
    pub fn push_at(&self, candidate: ObservationCandidate, now_ms: i64) -> bool {
        let key = (candidate.tool.clone(), candidate.args_hash.clone());
        let mut inner = self.inner.lock().expect("observation queue mutex poisoned");
        if let Some(&prev_ts) = inner.lru.get(&key) {
            if now_ms - prev_ts < self.window_ms {
                return false;
            }
        }
        inner.pending.push(candidate);
        if inner.order.len() >= inner.lru_cap {
            if let Some(evicted) = inner.order.pop_front() {
                inner.lru.remove(&evicted);
            }
        }
        inner.order.push_back(key.clone());
        inner.lru.insert(key, now_ms);
        true
    }

    /// Pending FIFO length.
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .expect("observation queue mutex poisoned")
            .pending
            .len()
    }

    /// Whether the pending FIFO is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drain pending candidates (consumer / promote cycle).
    pub fn drain_pending(&self) -> Vec<ObservationCandidate> {
        let mut inner = self.inner.lock().expect("observation queue mutex poisoned");
        std::mem::take(&mut inner.pending)
    }
}

impl Default for ObservationQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// FNV-1a 64-bit, rendered as 16 lowercase hex characters.
pub fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash = FNV64_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV64_PRIME);
    }
    format!("{hash:016x}")
}

/// Stable args digest: JSON serialization (serde_json object keys are sorted)
/// then FNV-1a 64-bit hex.
pub fn args_hash(args: &Value) -> String {
    let canonical = serde_json::to_string(args).unwrap_or_default();
    fnv1a64_hex(canonical.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn candidate(tool: &str, hash: &str, ts_ms: i64) -> ObservationCandidate {
        ObservationCandidate {
            tool: tool.into(),
            args_hash: hash.into(),
            outcome: ObservationOutcome::Success {
                summary: Some("a".into()),
            },
            ts_ms,
            source: ObservationSource::ToolExecution,
        }
    }

    #[test]
    fn args_hash_is_stable_and_distinguishes_args() {
        assert_eq!(
            args_hash(&json!({"a": 1, "b": 2})),
            args_hash(&json!({"a": 1, "b": 2}))
        );
        assert_ne!(args_hash(&json!({"a": 1})), args_hash(&json!({"a": 2})));
        assert_eq!(args_hash(&json!({})).len(), 16);
        // Object key order must not change the digest (BTree Map).
        assert_eq!(
            args_hash(&json!({"b": 2, "a": 1})),
            args_hash(&json!({"a": 1, "b": 2}))
        );
    }

    #[test]
    fn outcome_truncates_long_strings() {
        let big = "x".repeat(1000);
        match ObservationOutcome::from_result(true, Some(&big), None) {
            ObservationOutcome::Success { summary } => {
                let text = summary.expect("summary");
                assert!(text.chars().count() <= 201);
                assert!(text.ends_with('…'));
            }
            other => panic!("expected Success, got {other:?}"),
        }
        match ObservationOutcome::from_result(false, None, Some(&big)) {
            ObservationOutcome::Failure { error } => {
                assert!(error.expect("error").ends_with('…'));
            }
            other => panic!("expected Failure, got {other:?}"),
        }
        assert_eq!(
            ObservationOutcome::from_result(true, None, None).label(),
            "success"
        );
        assert_eq!(
            ObservationOutcome::from_result(false, None, None).label(),
            "failure"
        );
    }

    #[test]
    fn dedup_within_window_suppresses_duplicate() {
        let queue = ObservationQueue::new();
        assert!(queue.push_at(candidate("t", "h", 1_000_000), 1_000_000));
        assert!(!queue.push_at(candidate("t", "h", 1_001_000), 1_001_000));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn dedup_allows_after_window_expires() {
        let queue = ObservationQueue::new();
        assert!(queue.push_at(candidate("t", "h", 1_000_000), 1_000_000));
        let later = 1_000_000 + DEFAULT_DEDUP_WINDOW_MS + 1;
        assert!(queue.push_at(candidate("t", "h", later), later));
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn different_args_hash_not_deduped() {
        let queue = ObservationQueue::new();
        assert!(queue.push_at(candidate("t", args_hash(&json!({"x": 1})).as_str(), 1), 1));
        assert!(queue.push_at(candidate("t", args_hash(&json!({"x": 2})).as_str(), 1), 1));
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn drain_pending_clears_queue() {
        let queue = ObservationQueue::new();
        for index in 0..3 {
            queue.push_at(candidate("t", &format!("h{index}"), 0), 0);
        }
        assert_eq!(queue.len(), 3);
        let drained = queue.drain_pending();
        assert_eq!(drained.len(), 3);
        assert!(queue.is_empty());
    }

    #[test]
    fn lru_cap_evicts_index_not_pending() {
        let queue = ObservationQueue::with_config(ObservationQueueConfig {
            window_ms: 1_000_000,
            lru_cap: 3,
        });
        for index in 0..5 {
            queue.push_at(candidate("t", &format!("h{index}"), 0), 0);
        }
        assert_eq!(queue.len(), 5, "LRU caps the hash index, not the FIFO");
        // Evicted index (h0) can be re-inserted inside the window.
        assert!(queue.push_at(candidate("t", "h0", 1), 1));
        assert_eq!(queue.len(), 6);
    }
}
