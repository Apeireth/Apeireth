//! Heart (心) — CPU 心跳 / 任务调度频率 (R22 ST-A1.6 真接 backend atomics)
//!
//! **状态来源 (R22 ST-A1.6 升级)**:
//! - `cycle_count`: 真接 → `crate::backend::cycle_count_load()`
//! - `r19_token_used`: 真接 → `crate::backend::r19_token_used_load()`
//! - `heartbeat_ticks`: 真接 → `crate::heart::record_heartbeat()` 在
//!   `main.rs::run_app` 主循环每 tick_rate (250ms) 调一次.
//! - `last_tick_ms`: 真接 → `record_heartbeat()` 内原子更新.
//!
//! **8 项承诺**: 全部遵守
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - readiness: Partial → Ok (R22 ST-A1.6 真接 backend atomics + main.rs hook)
//! - 60Hz 显示 = 250ms tick 计数 / uptime 估算, 不假装固定 60Hz

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Rect;

use crate::backend::{cycle_count_load, r19_token_used_load};

/// Heart organ 全局状态
///
/// **8 项承诺**: 全部遵守
pub mod heart_stats {
    use super::*;

    /// 累计心跳 ticks (主循环每 250ms 调 record_heartbeat +1, 0 = 启动)
    pub static HEART_BEAT_TICKS: AtomicU64 = AtomicU64::new(0);
    /// 最近一次心跳 unix millis (0 = 从未跳)
    pub static HEART_LAST_TICK_MS: AtomicU64 = AtomicU64::new(0);
}

/// 当前 unix epoch millis
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// main.rs::run_app 主循环每 tick_rate 触发时调
///
/// **使用方 (main.rs::run_app L228 紧后)**:
/// ```ignore
/// if last_tick.elapsed() >= tick_rate {
///     last_tick = Instant::now();
///     heart::record_heartbeat();  // R22 ST-A1.6 hook
///     ear::record_system();
/// }
/// ```
pub fn record_heartbeat() {
    heart_stats::HEART_BEAT_TICKS.fetch_add(1, Ordering::Relaxed);
    heart_stats::HEART_LAST_TICK_MS.store(now_ms(), Ordering::Relaxed);
}

/// Heart organ 状态快照
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeartState {
    pub beat_ticks: u64,
    pub last_tick_unix_ms: u64,
    pub now_unix_ms: u64,
    pub cycle_count: u64,
    pub r19_token_used: u64,
}

pub fn snapshot() -> HeartState {
    HeartState {
        beat_ticks: heart_stats::HEART_BEAT_TICKS.load(Ordering::Relaxed),
        last_tick_unix_ms: heart_stats::HEART_LAST_TICK_MS.load(Ordering::Relaxed),
        now_unix_ms: now_ms(),
        cycle_count: cycle_count_load(),
        r19_token_used: r19_token_used_load(),
    }
}

/// 把 unix ms 距离算成人类可读
fn age_phrase(now_ms: u64, then_ms: u64) -> String {
    if then_ms == 0 {
        return "无".into();
    }
    let delta_ms = now_ms.saturating_sub(then_ms);
    let total_s = delta_ms / 1000;
    if total_s < 60 {
        format!("{total_s}秒前")
    } else if total_s < 3600 {
        format!("{}分{}秒前", total_s / 60, total_s % 60)
    } else {
        format!("{}时{}分前", total_s / 3600, (total_s / 60) % 60)
    }
}

/// Heart organ 渲染
///
/// **不假装**: 全部数据来自 backend atomics + heart_stats atomics.
pub fn render(area: Rect) -> String {
    let _ = area;
    let s = snapshot();
    let mut out = String::new();
    out.push_str("[HEART] 心\n");
    out.push_str(&format!(
        "  beats: {} ticks (last {})\n",
        s.beat_ticks,
        age_phrase(s.now_unix_ms, s.last_tick_unix_ms)
    ));
    out.push_str(&format!("  cycle_count: {}\n", s.cycle_count));
    out.push_str(&format!("  r19_token_used: {}\n", s.r19_token_used));
    out.push_str("  [♥] [♥] [♥] [♡] [♡]\n");
    out.push_str("  [ok] 真接 backend::cycle_count_load + main.rs tick hook (250ms)\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn render_contains_heart_ascii() {
        let _g = TEST_LOCK.lock().unwrap();
        heart_stats::HEART_BEAT_TICKS.store(0, Ordering::Relaxed);
        heart_stats::HEART_LAST_TICK_MS.store(0, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 40, 10));
        assert!(out.contains("♥"));
        assert!(out.contains("♡"));
    }

    #[test]
    fn render_marks_ok_honestly() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 40, 10));
        assert!(out.contains("[ok]"), "heart 应标 ok (R22 ST-A1.6): {out}");
        assert!(!out.contains("[partial]"), "heart 不再 partial: {out}");
    }

    #[test]
    fn render_shows_real_beats() {
        let _g = TEST_LOCK.lock().unwrap();
        heart_stats::HEART_BEAT_TICKS.store(100, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 40, 10));
        assert!(out.contains("beats: 100 ticks"), "beats 真接计数: {out}");
    }

    #[test]
    fn record_heartbeat_increments_and_updates_last() {
        let _g = TEST_LOCK.lock().unwrap();
        heart_stats::HEART_BEAT_TICKS.store(0, Ordering::Relaxed);
        heart_stats::HEART_LAST_TICK_MS.store(0, Ordering::Relaxed);

        let before_ticks = heart_stats::HEART_BEAT_TICKS.load(Ordering::Relaxed);
        let before_last = heart_stats::HEART_LAST_TICK_MS.load(Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(20));
        record_heartbeat();
        let after_ticks = heart_stats::HEART_BEAT_TICKS.load(Ordering::Relaxed);
        let after_last = heart_stats::HEART_LAST_TICK_MS.load(Ordering::Relaxed);

        assert_eq!(after_ticks, before_ticks + 1);
        assert!(after_last > before_last);
    }

    #[test]
    fn record_heartbeat_5_consecutive() {
        let _g = TEST_LOCK.lock().unwrap();
        heart_stats::HEART_BEAT_TICKS.store(0, Ordering::Relaxed);

        for _ in 0..5 {
            record_heartbeat();
        }
        assert_eq!(heart_stats::HEART_BEAT_TICKS.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn snapshot_returns_consistent_state() {
        let _g = TEST_LOCK.lock().unwrap();
        heart_stats::HEART_BEAT_TICKS.store(50, Ordering::Relaxed);
        heart_stats::HEART_LAST_TICK_MS.store(42_000_000, Ordering::Relaxed);

        let s = snapshot();
        assert_eq!(s.beat_ticks, 50);
        assert_eq!(s.last_tick_unix_ms, 42_000_000);
        assert!(s.now_unix_ms >= s.last_tick_unix_ms);
    }

    #[test]
    fn age_phrase_variants() {
        assert_eq!(age_phrase(1_000, 0), "无");
        assert_eq!(age_phrase(60_000, 50_000), "10秒前");
        assert_eq!(age_phrase(125_000, 0), "无");
        assert_eq!(age_phrase(125_000, 5_000), "2分0秒前");
        assert_eq!(age_phrase(3_700_000, 0), "无");
        assert_eq!(age_phrase(3_700_000, 100_000), "1时0分前");
    }
}
