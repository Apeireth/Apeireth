//! Integration tests for apeireth-tui (CognitionLiveTracker — R117 live subscribe).
//!
//! **R18 路线图 Stage 2 续**: tui 是 binary crate, 走 `#[path]` 模式 (跟 `app_state.rs` 模板一致),
//! 测 `cognition_live::CognitionLiveTracker` 4 事件 (NoChange / Updated / FirstSeen / Cleared) +
//! `is_stale` / `mark_seen` / `reset` 状态机.
//!
//! **注**: 本测试**不**调 `check_for_update()` (它依赖 `crate::organ::memory::latest_cognition_summary`
//! → backend → memory src, baseline 偏移会阻编译). 只测 tracker 自身 API + LiveEvent 枚举.
//!
//! **不假装**: 测试是真跑 (`cargo nextest run -p apeireth-tui --test cognition_live`).

#[path = "../src/theme.rs"]
mod theme;
#[path = "../src/error.rs"] mod error;
#[path = "../src/http.rs"] mod http;
#[path = "../src/nav/mod.rs"] mod nav;
#[path = "../src/app.rs"]
mod app;
#[path = "../src/config_watcher.rs"] mod config_watcher;
#[path = "../src/backend.rs"] mod backend;
#[path = "../src/http_llm.rs"] mod http_llm;
#[path = "../src/observability.rs"] mod observability;
#[path = "../src/pages/mod.rs"] mod pages;
#[path = "../src/organ/mod.rs"] mod organ;
#[path = "../src/command/mod.rs"] mod command;
#[path = "../src/persistence.rs"] mod persistence;
#[path = "../src/llm_config.rs"] mod llm_config;
#[path = "../src/onboarding.rs"] mod onboarding;
#[path = "../src/cognition_live.rs"]
mod cognition_live;

use cognition_live::{CognitionLiveTracker, LiveEvent};
use std::collections::HashMap;

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
fn tracker_starts_with_no_seen_signature() {
    let t = CognitionLiveTracker::new();
    assert!(t.seen_signature().is_none(), "新 tracker last_seen 应 None");
    assert_eq!(t.poll_threshold_ms(), 500, "默认 poll threshold 500ms");
}

#[test]
fn tracker_with_threshold_ms() {
    let t = CognitionLiveTracker::with_threshold_ms(2_000);
    assert_eq!(t.poll_threshold_ms(), 2_000);
}

#[test]
fn mark_seen_sets_signature() {
    // mark_seen 显式设置 signature (test 用)
    let mut t = CognitionLiveTracker::new();
    let s = make_summary(0.5, 0.0, 1.0, true);
    assert!(t.seen_signature().is_none());
    t.mark_seen(&s);
    assert!(t.seen_signature().is_some(), "mark_seen 后应 Some");
}

#[test]
fn mark_seen_idempotent_same_value() {
    // 同 value 多次 mark_seen 仍是同一 signature
    let mut t = CognitionLiveTracker::new();
    let s = make_summary(0.5, 0.0, 1.0, true);
    t.mark_seen(&s);
    let sig1 = t.seen_signature();
    t.mark_seen(&s);
    let sig2 = t.seen_signature();
    assert_eq!(sig1, sig2, "同 value 多次 mark_seen signature 应一致");
}

#[test]
fn reset_clears_signature() {
    let mut t = CognitionLiveTracker::new();
    let s = make_summary(0.5, 0.0, 1.0, true);
    t.mark_seen(&s);
    assert!(t.seen_signature().is_some());
    t.reset();
    assert!(t.seen_signature().is_none(), "reset 后应 None");
}

#[test]
fn live_event_first_seen_variant_holds_summary() {
    // LiveEvent::FirstSeen { summary } 解构后能拿 summary
    let s = make_summary(0.7, 0.2, 0.9, true);
    let e = LiveEvent::FirstSeen { summary: s.clone() };
    match e {
        LiveEvent::FirstSeen { summary } => {
            assert_eq!(summary["mean"], 0.7);
            assert_eq!(summary["verdict_approve"], 1.0);
        }
        _ => panic!("expected FirstSeen"),
    }
}

#[test]
fn live_event_equality_variants() {
    // 4 variant 区分 (per PartialEq)
    let s = make_summary(0.5, 0.0, 1.0, true);
    assert_eq!(LiveEvent::NoChange, LiveEvent::NoChange);
    assert_eq!(LiveEvent::Cleared, LiveEvent::Cleared);
    assert_eq!(
        LiveEvent::FirstSeen { summary: s.clone() },
        LiveEvent::FirstSeen { summary: s }
    );
    assert_ne!(LiveEvent::NoChange, LiveEvent::Cleared);
    assert_ne!(LiveEvent::Updated { summary: make_summary(0.5, 0.0, 1.0, true) },
               LiveEvent::Updated { summary: make_summary(0.3, 0.0, 0.6, true) });
}

#[test]
fn is_stale_default_threshold_500ms() {
    // 默认 threshold 500ms, 立刻调应不 stale
    let t = CognitionLiveTracker::new();
    assert!(!t.is_stale(), "新 tracker 不应 stale (last_check = now)");
}

#[test]
fn is_stale_after_threshold() {
    // 自定义短 threshold, sleep 后应 stale
    let t = CognitionLiveTracker::with_threshold_ms(50);
    assert!(!t.is_stale());
    std::thread::sleep(std::time::Duration::from_millis(80));
    assert!(t.is_stale(), "sleep 80ms 后应 stale (threshold 50ms)");
}
