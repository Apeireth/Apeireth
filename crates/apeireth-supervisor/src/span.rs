//! R259: OTel-style Span + SpanTracker (lightweight self-rolled, 0 external dep).
//!
//! Inspired by OTel trace spec (W3C Trace Context + Span) and Rust tracing crate.
//! We mirror:
//! - `SpanEvent` = OTel Span (name + span_id + parent_span_id + start/end + status + attrs)
//! - `SpanTracker` = thread-safe pending + completed lists (HashMap-based)
//!
//! ## Why not pull `tracing` / `opentelemetry` crate?
//!
//! - O-2 walking on shoulders: workspace already has tracing = "0.1" via voice/lark,
//!   but we 0 引 here to keep supervisor pure (tests can run 0-dep).
//! - O-5 not pretending: this is a deliberately-simple Span store for in-process
//!   cycle/task tracking, not a full distributed tracing impl. For real distributed
//!   propagation use OTLP/W3C Trace Context externally.

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique span ID within a tracker (monotonic u64, 0 reserved for "no parent").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SpanId(pub u64);

impl SpanId {
    /// Sentinel value used for "this span has no parent" (root spans).
    pub const ROOT: SpanId = SpanId(0);

    /// Construct a new SpanId from a raw u64 (does NOT validate uniqueness).
    pub const fn new(raw: u64) -> Self { Self(raw) }

    /// Raw u64 value.
    pub const fn raw(self) -> u64 { self.0 }
}

impl std::fmt::Display for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "span#{}", self.0)
    }
}

/// OTel-style span status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpanStatus {
    /// Span ended without error.
    Ok,
    /// Span ended with error (use attrs["error"] = message).
    Err,
    /// Span not yet ended or status unspecified.
    Unset,
}

impl Default for SpanStatus {
    fn default() -> Self { SpanStatus::Unset }
}

/// OTel-style span event (single immutable record after span.end()).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub span_id: SpanId,
    pub parent: Option<SpanId>,
    pub start_unix_ms: u64,
    pub end_unix_ms: u64,
    pub status: SpanStatus,
    pub attrs: Vec<(String, String)>,
}

impl SpanEvent {
    /// Elapsed milliseconds (0 if not yet ended).
    pub fn elapsed_ms(&self) -> u64 {
        if self.end_unix_ms == 0 || self.end_unix_ms < self.start_unix_ms {
            0
        } else {
            self.end_unix_ms - self.start_unix_ms
        }
    }

    /// Get an attribute value by key.
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Thread-safe tracker for active + completed spans.
pub struct SpanTracker {
    next_id: Mutex<u64>,
    pending: Mutex<HashMap<SpanId, SpanEvent>>,
    completed: Mutex<Vec<SpanEvent>>,
    max_pending: usize,
    max_completed: usize,
}

impl SpanTracker {
    /// Create with default capacities (1024 pending + 4096 completed).
    pub fn new() -> Self {
        Self::with_capacity(1024, 4096)
    }

    /// Create with custom pending/completed capacities.
    pub fn with_capacity(max_pending: usize, max_completed: usize) -> Self {
        Self {
            next_id: Mutex::new(1),
            pending: Mutex::new(HashMap::new()),
            completed: Mutex::new(Vec::new()),
            max_pending,
            max_completed,
        }
    }

    /// Start a new span. `parent` = None or SpanId::ROOT for a root span.
    pub fn start_span(&self, parent: Option<SpanId>, name: impl Into<String>) -> Option<SpanId> {
        let mut next = self.next_id.lock().expect("poisoned");
        let id = SpanId(*next);
        *next += 1;
        drop(next);

        let mut pending = self.pending.lock().expect("poisoned");
        if pending.len() >= self.max_pending {
            return None;
        }
        let now = unix_ms();
        let parent = parent.filter(|p| *p != SpanId::ROOT);
        let event = SpanEvent {
            name: name.into(),
            span_id: id,
            parent,
            start_unix_ms: now,
            end_unix_ms: 0,
            status: SpanStatus::Unset,
            attrs: Vec::new(),
        };
        pending.insert(id, event);
        Some(id)
    }

    /// End a span. Returns the completed SpanEvent on success.
    pub fn end_span(
        &self,
        id: SpanId,
        status: SpanStatus,
        attrs: impl IntoIterator<Item = (String, String)>,
    ) -> Option<SpanEvent> {
        let mut pending = self.pending.lock().expect("poisoned");
        let mut event = pending.remove(&id)?;
        let now = unix_ms();
        event.end_unix_ms = if event.start_unix_ms > 0 { now } else { 0 };
        event.status = status;
        event.attrs.extend(attrs);
        drop(pending);
        let mut completed = self.completed.lock().expect("poisoned");
        if completed.len() >= self.max_completed {
            completed.remove(0);
        }
        completed.push(event.clone());
        Some(event)
    }

    /// Take all completed spans.
    pub fn take_completed(&self) -> Vec<SpanEvent> {
        let mut completed = self.completed.lock().expect("poisoned");
        std::mem::take(&mut *completed)
    }

    /// Current number of pending spans.
    pub fn active_count(&self) -> usize {
        self.pending.lock().expect("poisoned").len()
    }

    /// Current number of completed spans (not yet taken).
    pub fn completed_len(&self) -> usize {
        self.completed.lock().expect("poisoned").len()
    }
}

impl Default for SpanTracker {
    fn default() -> Self { Self::new() }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_root_span_default_parent() {
        let t = SpanTracker::new();
        let id = t.start_span(None, "cycle").unwrap();
        assert_eq!(id, SpanId(1));
        assert_eq!(t.active_count(), 1);
        assert_eq!(t.completed_len(), 0);
    }

    #[test]
    fn t02_root_span_via_root_const() {
        let t = SpanTracker::new();
        let id = t.start_span(Some(SpanId::ROOT), "cycle").unwrap();
        assert_eq!(id, SpanId(1));
        let ev = t.end_span(id, SpanStatus::Ok, std::iter::empty()).unwrap();
        assert!(ev.parent.is_none());
    }

    #[test]
    fn t03_child_span_parent_set() {
        let t = SpanTracker::new();
        let root = t.start_span(None, "root").unwrap();
        let child = t.start_span(Some(root), "child").unwrap();
        assert_eq!(child, SpanId(2));
        let ev = t.end_span(child, SpanStatus::Ok, std::iter::empty()).unwrap();
        assert_eq!(ev.parent, Some(root));
    }

    #[test]
    fn t04_end_span_returns_event() {
        let t = SpanTracker::new();
        let id = t.start_span(None, "task").unwrap();
        let ev = t.end_span(id, SpanStatus::Ok, vec![("key".into(), "val".into())]).unwrap();
        assert_eq!(ev.name, "task");
        assert_eq!(ev.status, SpanStatus::Ok);
        assert_eq!(ev.attr("key"), Some("val"));
        assert!(ev.elapsed_ms() < 5000);
        assert_eq!(t.active_count(), 0);
        assert_eq!(t.completed_len(), 1);
    }

    #[test]
    fn t05_end_span_unknown_returns_none() {
        let t = SpanTracker::new();
        let bogus = SpanId::new(9999);
        assert!(t.end_span(bogus, SpanStatus::Ok, std::iter::empty()).is_none());
    }

    #[test]
    fn t06_take_completed_clears() {
        let t = SpanTracker::new();
        let id = t.start_span(None, "x").unwrap();
        t.end_span(id, SpanStatus::Ok, std::iter::empty()).unwrap();
        assert_eq!(t.completed_len(), 1);
        let events = t.take_completed();
        assert_eq!(events.len(), 1);
        assert_eq!(t.completed_len(), 0);
    }

    #[test]
    fn t07_max_pending_capacity() {
        let t = SpanTracker::with_capacity(2, 100);
        let a = t.start_span(None, "a").unwrap();
        let b = t.start_span(None, "b").unwrap();
        assert!(t.start_span(None, "c").is_none());
        t.end_span(a, SpanStatus::Ok, std::iter::empty()).unwrap();
        t.end_span(b, SpanStatus::Ok, std::iter::empty()).unwrap();
    }

    #[test]
    fn t08_max_completed_drops_oldest() {
        let t = SpanTracker::with_capacity(100, 2);
        let a = t.start_span(None, "a").unwrap();
        let b = t.start_span(None, "b").unwrap();
        let c = t.start_span(None, "c").unwrap();
        t.end_span(a, SpanStatus::Ok, std::iter::empty()).unwrap();
        t.end_span(b, SpanStatus::Ok, std::iter::empty()).unwrap();
        t.end_span(c, SpanStatus::Ok, std::iter::empty()).unwrap();
        let events = t.take_completed();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].name, "b");
        assert_eq!(events[1].name, "c");
    }

    #[test]
    fn t09_attrs_extend_on_end() {
        let t = SpanTracker::new();
        let id = t.start_span(None, "llm_call").unwrap();
        let ev = t.end_span(id, SpanStatus::Ok, vec![
            ("model".into(), "MiniMax-M3".into()),
            ("tokens_in".into(), "120".into()),
        ]).unwrap();
        assert_eq!(ev.attr("model"), Some("MiniMax-M3"));
        assert_eq!(ev.attr("tokens_in"), Some("120"));
    }

    #[test]
    fn t10_span_id_display() {
        assert_eq!(format!("{}", SpanId(42)), "span#42");
        assert_eq!(format!("{}", SpanId::ROOT), "span#0");
    }

    #[test]
    fn t11_default_impl() {
        let t = SpanTracker::default();
        assert_eq!(t.active_count(), 0);
        assert_eq!(t.completed_len(), 0);
    }

    #[test]
    fn t12_parent_filter_drops_root() {
        let t = SpanTracker::new();
        let id = t.start_span(Some(SpanId::ROOT), "cycle").unwrap();
        let ev = t.end_span(id, SpanStatus::Ok, std::iter::empty()).unwrap();
        assert!(ev.parent.is_none());
    }

    #[test]
    fn t13_thread_safe_shared_tracker() {
        use std::sync::Arc;
        use std::thread;
        let t = Arc::new(SpanTracker::new());
        let mut handles = Vec::new();
        for i in 0..10 {
            let t2 = t.clone();
            handles.push(thread::spawn(move || {
                let id = t2.start_span(None, format!("span_{i}")).unwrap();
                t2.end_span(id, SpanStatus::Ok, std::iter::empty()).unwrap();
            }));
        }
        for h in handles { h.join().unwrap(); }
        assert_eq!(t.active_count(), 0);
        assert_eq!(t.completed_len(), 10);
    }

    #[test]
    fn t14_status_default_unset() {
        assert_eq!(SpanStatus::default(), SpanStatus::Unset);
        let t = SpanTracker::new();
        let id = t.start_span(None, "x").unwrap();
        let pending = t.pending.lock().unwrap();
        let ev = pending.get(&id).unwrap();
        assert_eq!(ev.status, SpanStatus::Unset);
    }

    #[test]
    fn t15_attrs_are_keyed_lookup() {
        let t = SpanTracker::new();
        let id = t.start_span(None, "x").unwrap();
        let ev = t.end_span(id, SpanStatus::Ok, vec![
            ("a".into(), "1".into()),
            ("b".into(), "2".into()),
        ]).unwrap();
        assert_eq!(ev.attr("a"), Some("1"));
        assert_eq!(ev.attr("b"), Some("2"));
        assert_eq!(ev.attr("c"), None);
    }
}
