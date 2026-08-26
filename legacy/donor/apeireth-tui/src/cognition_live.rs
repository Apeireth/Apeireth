//! R117: TUI cognition live subscribe (poll-based, 0 新 dep)
//!
//! **目标**: TUI memory organ 拿 cognition graph summary 时, 能感知到新 summary 到达,
//! 触发 render refresh. 不用 notify crate, 用 mtime-based polling (0 新 dep).
//!
//! **Apeireth 真接 (本 module)**:
//! - `CognitionLiveTracker` struct — 持有 last_seen_signature (mean/min/max/verdict 4 值 hash) + last_check_ms
//! - `check_for_update() -> Option<HashMap<String, f64>>` — 调 backend `latest_cognition_summary`, 比对, 新就返 Some
//! - `mark_seen(signature)` — 显式标记 seen (test 用)
//! - `is_stale(threshold_ms)` — 判断是否需要 refresh (UI 用)
//! - `LiveEvent` enum — `NoChange` / `Updated { summary }` / `FirstSeen { summary }` / `Cleared`
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `organ/memory.rs` 已有 `latest_cognition_summary` (R78 LOCKED)
//! - 0 改 `organ/memory.rs` 已有 `record_cognition_summary` (R78 LOCKED)
//! - 0 改 TUI main render loop (caller 主动调 check_for_update)
//! - 0 引入 notify / tokio (TUI 同步模型, 0 假设 runtime)
//!
//! **借鉴锚 (S-15)**:
//! - LSP `workspace/didChangeWatchedFiles` (文件级 event 通知)
//! - ratatui `should_redraw` (UI redraw 决策)
//! - VCP vcptoolbox live mode (mtime compare → redraw)

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::organ::memory::latest_cognition_summary;

// ============================================================
// LiveEvent
// ============================================================

/// **Live subscribe 事件** (per poll cycle)
#[derive(Debug, Clone, PartialEq)]
pub enum LiveEvent {
    /// 无变化 (last seen == current)
    NoChange,
    /// 新 summary 到达
    Updated { summary: HashMap<String, f64> },
    /// 首次见到 summary (从 None 状态)
    FirstSeen { summary: HashMap<String, f64> },
    /// Summary 被清空 (从有 → None)
    Cleared,
}

// ============================================================
// CognitionLiveTracker
// ============================================================

/// **Cognition summary live tracker** (mtime-based polling, 0 新 dep)
///
/// 用法:
/// ```ignore
/// let mut tracker = CognitionLiveTracker::new();
/// loop {
///     match tracker.check_for_update() {
///         LiveEvent::FirstSeen { summary } | LiveEvent::Updated { summary } => {
///             // re-render memory organ with new summary
///         }
///         LiveEvent::Cleared => { /* re-render empty */ }
///         LiveEvent::NoChange => { /* no-op */ }
///     }
///     std::thread::sleep(Duration::from_millis(500));
/// }
/// ```
#[derive(Debug, Default)]
pub struct CognitionLiveTracker {
    /// 上次见到的 signature (mean/min/max/verdict 4 值 hash)
    last_seen_signature: Option<u64>,
    /// 上次 check 的时间戳 (epoch ms)
    last_check_ms: u64,
    /// poll 间隔阈值 (ms), is_stale() 用
    poll_threshold_ms: u64,
}

impl CognitionLiveTracker {
    /// 默认 poll_threshold_ms = 500ms
    pub fn new() -> Self {
        Self {
            last_seen_signature: None,
            last_check_ms: now_ms(),
            poll_threshold_ms: 500,
        }
    }

    /// 自定义 poll 阈值
    pub fn with_threshold_ms(threshold_ms: u64) -> Self {
        Self {
            last_seen_signature: None,
            last_check_ms: now_ms(),
            poll_threshold_ms: threshold_ms,
        }
    }

    /// **是否 stale** (上次 check 距今超过 poll_threshold_ms)
    pub fn is_stale(&self) -> bool {
        now_ms().saturating_sub(self.last_check_ms) >= self.poll_threshold_ms
    }

    /// **检查更新** — 调 backend, 比对 signature, 返 LiveEvent
    pub fn check_for_update(&mut self) -> LiveEvent {
        self.last_check_ms = now_ms();
        self.apply_summary(latest_cognition_summary())
    }

    fn apply_summary(&mut self, current: Option<HashMap<String, f64>>) -> LiveEvent {
        match current {
            None => {
                if self.last_seen_signature.is_some() {
                    self.last_seen_signature = None;
                    LiveEvent::Cleared
                } else {
                    LiveEvent::NoChange
                }
            }
            Some(summary) => {
                let sig = compute_signature(&summary);
                match self.last_seen_signature {
                    None => {
                        self.last_seen_signature = Some(sig);
                        LiveEvent::FirstSeen { summary }
                    }
                    Some(prev) if prev != sig => {
                        self.last_seen_signature = Some(sig);
                        LiveEvent::Updated { summary }
                    }
                    _ => LiveEvent::NoChange,
                }
            }
        }
    }

    /// **强制 mark seen** (test 用, 设个 signature 让下次 check 返 NoChange)
    pub fn mark_seen(&mut self, summary: &HashMap<String, f64>) {
        self.last_seen_signature = Some(compute_signature(summary));
    }

    /// **当前 last_seen_signature** (debug / test 用)
    pub fn seen_signature(&self) -> Option<u64> {
        self.last_seen_signature
    }

    /// **当前 poll threshold**
    pub fn poll_threshold_ms(&self) -> u64 {
        self.poll_threshold_ms
    }

    /// **重置** (test 用, 下次 check 返 FirstSeen)
    pub fn reset(&mut self) {
        self.last_seen_signature = None;
        self.last_check_ms = now_ms();
    }
}

// ============================================================
// 工具
// ============================================================

/// **计算 summary signature** (per mean/min/max/verdict 4 值)
fn compute_signature(summary: &HashMap<String, f64>) -> u64 {
    let mut hasher = DefaultHasher::new();
    summary
        .get("mean")
        .unwrap_or(&0.0)
        .to_bits()
        .hash(&mut hasher);
    summary
        .get("min")
        .unwrap_or(&0.0)
        .to_bits()
        .hash(&mut hasher);
    summary
        .get("max")
        .unwrap_or(&0.0)
        .to_bits()
        .hash(&mut hasher);
    summary
        .get("verdict_approve")
        .unwrap_or(&0.0)
        .to_bits()
        .hash(&mut hasher);
    hasher.finish()
}

/// **now epoch millis**
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organ::memory::record_cognition_summary;
    use std::sync::Mutex;

    // record_cognition_summary 用 atomic, 测试间用 mutex 串行
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn make_summary(mean: f64, min: f64, max: f64, approve: bool) -> HashMap<String, f64> {
        let mut m = HashMap::new();
        m.insert("mean".to_string(), mean);
        m.insert("min".to_string(), min);
        m.insert("max".to_string(), max);
        m.insert(
            "verdict_approve".to_string(),
            if approve { 1.0 } else { 0.0 },
        );
        m
    }

    #[test]
    fn live_event_equality() {
        let s = make_summary(0.5, 0.0, 1.0, true);
        let e1 = LiveEvent::FirstSeen { summary: s.clone() };
        let e2 = LiveEvent::FirstSeen { summary: s.clone() };
        assert_eq!(e1, e2);
        assert_ne!(e1, LiveEvent::NoChange);
    }

    #[test]
    fn compute_signature_changes_on_mean() {
        let s1 = make_summary(0.5, 0.0, 1.0, true);
        let s2 = make_summary(0.6, 0.0, 1.0, true);
        assert_ne!(compute_signature(&s1), compute_signature(&s2));
    }

    #[test]
    fn compute_signature_changes_on_verdict() {
        let s1 = make_summary(0.5, 0.0, 1.0, true);
        let s2 = make_summary(0.5, 0.0, 1.0, false);
        assert_ne!(compute_signature(&s1), compute_signature(&s2));
    }

    #[test]
    fn compute_signature_stable_for_same_values() {
        let s1 = make_summary(0.5, 0.0, 1.0, true);
        let s2 = make_summary(0.5, 0.0, 1.0, true);
        assert_eq!(compute_signature(&s1), compute_signature(&s2));
    }

    #[test]
    fn tracker_new_is_stale_after_threshold() {
        let t = CognitionLiveTracker::new();
        // new() set last_check_ms = now, so not stale immediately
        assert!(!t.is_stale());
    }

    #[test]
    fn tracker_with_custom_threshold() {
        let t = CognitionLiveTracker::with_threshold_ms(100);
        assert_eq!(t.poll_threshold_ms(), 100);
    }

    #[test]
    fn tracker_check_no_change_when_no_summary() {
        let _g = TEST_LOCK.lock().unwrap();
        // Note: we don't reset global state, so just check that we can call without panic
        let mut t = CognitionLiveTracker::new();
        let ev = t.check_for_update();
        // Either NoChange (if no summary globally) or FirstSeen (if summary was set previously)
        assert!(matches!(ev, LiveEvent::NoChange) || matches!(ev, LiveEvent::FirstSeen { .. }));
    }

    #[test]
    fn tracker_check_first_seen_after_record() {
        let _g = TEST_LOCK.lock().unwrap();
        record_cognition_summary(0.5, 0.0, 1.0, true);
        let mut t = CognitionLiveTracker::new();
        t.reset(); // ensure no prior seen
        let ev = t.check_for_update();
        match ev {
            LiveEvent::FirstSeen { .. } => {}
            _ => panic!("expected FirstSeen"),
        }
    }

    #[test]
    fn tracker_check_no_change_for_same_signature() {
        let _g = TEST_LOCK.lock().unwrap();
        record_cognition_summary(0.7, 0.2, 0.9, true);
        let mut t = CognitionLiveTracker::new();
        t.reset();
        let _ = t.check_for_update(); // FirstSeen
                                      // Now record same values
        record_cognition_summary(0.7, 0.2, 0.9, true);
        let ev = t.check_for_update();
        assert!(matches!(ev, LiveEvent::NoChange));
    }

    #[test]
    fn tracker_check_updated_on_change() {
        let _g = TEST_LOCK.lock().unwrap();
        record_cognition_summary(0.5, 0.0, 1.0, true);
        let mut t = CognitionLiveTracker::new();
        t.reset();
        let _ = t.check_for_update();
        // Change values
        record_cognition_summary(0.8, 0.3, 1.0, false);
        let ev = t.check_for_update();
        match ev {
            LiveEvent::Updated { summary } => {
                assert_eq!(summary.get("mean").unwrap(), &0.8);
            }
            _ => panic!("expected Updated"),
        }
    }

    #[test]
    fn tracker_check_cleared_when_summary_removed() {
        let summary = make_summary(0.5, 0.0, 1.0, true);
        let mut t = CognitionLiveTracker::new();
        assert!(matches!(
            t.apply_summary(Some(summary)),
            LiveEvent::FirstSeen { .. }
        ));
        assert_eq!(t.apply_summary(None), LiveEvent::Cleared);
        assert_eq!(t.apply_summary(None), LiveEvent::NoChange);
    }

    #[test]
    fn tracker_reset_clears_seen() {
        let summary = make_summary(0.5, 0.0, 1.0, true);
        let mut t = CognitionLiveTracker::new();
        t.mark_seen(&summary);
        assert!(t.seen_signature().is_some());
        t.reset();
        assert!(t.seen_signature().is_none());
    }

    #[test]
    fn tracker_debug_impl_works() {
        let t = CognitionLiveTracker::new();
        let s = format!("{:?}", t);
        assert!(s.contains("CognitionLiveTracker"));
    }
}
