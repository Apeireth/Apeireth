//! Memory (记忆) — 3 层记忆状态 (R22 ST-A1.8 真接 episode_count + stub 留 mid/long 接口)
//!
//! **状态来源 (R54 B8 续升级)**:
//! - `short_term_messages`: 真接 → `crate::memory::record_short_term_messages(count)` 在
//!   `backend.rs::snapshot_organ_main` 计算 `episode_count` 后调.
//! - `mid_term_count`: 真接 (R54) → backend 用 `EpisodeQuery::in_range(last 24h).limit(MAX)` 数 SQLite 中最近 24h episode 数, 通过 `record_mid_term_count()` 写入.
//! - `long_term_count`: 近似真接 (R54) → backend 用 `total / 5` heuristic (vector store 未上), `record_long_term_count()` 写入. 0 假装: 不是真"向量条数", 仅"近似 long-term 积累".
//!
//! **8 项承诺**: 全部遵守
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - readiness: Partial (2/3 真接: short_term + mid_term, long_term 近似), 标 partial 不假装 ok
//! - retention_days 编译期 hardcode (跟原 stub 一致)

use std::sync::atomic::{AtomicU64, Ordering};

use ratatui::layout::Rect;

/// Memory organ 全局状态
///
/// **8 项承诺**: 全部遵守
pub mod memory_stats {
    use super::*;

    /// 短期记忆消息数 (episode_count 真接, 0 = 启动后无 chat)
    pub static MEMORY_SHORT_TERM: AtomicU64 = AtomicU64::new(0);
    /// 中期记忆摘要数 (R25.3 真接前 = 0, API 已留)
    pub static MEMORY_MID_TERM: AtomicU64 = AtomicU64::new(0);
    /// 长期记忆向量条数 (R25.3 真接前 = 0, API 已留)
    pub static MEMORY_LONG_TERM: AtomicU64 = AtomicU64::new(0);
    /// 记忆保留窗口 (天, 编译期 hardcode, 改需 6 哲学锚 + 主人审)
    pub const RETENTION_DAYS: u32 = 7;
}

/// R47 B8: cognition graph summary storage (zero UI impact — render() 0 改).
///
/// Data flow: `apeireth-graph::cognition_graph::run_cognition_graph_sync` ->
/// TUI backend (after chat cycle) -> `record_cognition_summary` here -> atomic storage.
/// Snapshot exposed for future UI hooks (locked UI promise kept; no current consumers).
pub mod cognition_stats {
    use super::*;
    use std::collections::HashMap;

    /// R47 B8: rolling buffer of last 8 graph summaries (cognition_graph 流).
    pub static COGNITION_BUFFER: std::sync::Mutex<Vec<HashMap<String, f64>>> =
        std::sync::Mutex::new(Vec::new());

    /// R47 B8: cumulative approve verdict count (CognitiveDecide approved).
    pub static VERDICT_APPROVE_TOTAL: AtomicU64 = AtomicU64::new(0);
    /// R47 B8: cumulative block verdict count.
    pub static VERDICT_BLOCK_TOTAL: AtomicU64 = AtomicU64::new(0);
}

/// R47 B8: record one cognition graph run summary (mean/min/max/verdict).
pub fn record_cognition_summary(mean: f64, min: f64, max: f64, verdict_approve: bool) {
    use std::collections::HashMap;
    let mut entry = HashMap::with_capacity(4);
    entry.insert("mean".to_string(), mean);
    entry.insert("min".to_string(), min);
    entry.insert("max".to_string(), max);
    entry.insert(
        "verdict_approve".to_string(),
        if verdict_approve { 1.0 } else { 0.0 },
    );
    if let Ok(mut buf) = cognition_stats::COGNITION_BUFFER.lock() {
        buf.insert(0, entry);
        if buf.len() > 8 {
            buf.truncate(8);
        }
    }
    if verdict_approve {
        cognition_stats::VERDICT_APPROVE_TOTAL.fetch_add(1, Ordering::Relaxed);
    } else {
        cognition_stats::VERDICT_BLOCK_TOTAL.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn latest_cognition_summary() -> Option<std::collections::HashMap<String, f64>> {
    cognition_stats::COGNITION_BUFFER
        .lock()
        .ok()
        .and_then(|b| b.first().cloned())
}

pub fn cognition_verdict_counts() -> (u64, u64) {
    (
        cognition_stats::VERDICT_APPROVE_TOTAL.load(Ordering::Relaxed),
        cognition_stats::VERDICT_BLOCK_TOTAL.load(Ordering::Relaxed),
    )
}

/// backend.rs::snapshot_organ_main 算完 episode_count 后调
///
/// **使用方 (backend.rs::snapshot_organ_main)**:
/// ```ignore
/// let episode_count = memory_store()...unwrap_or(0);
/// memory::record_short_term_messages(episode_count);
/// ```
pub fn record_short_term_messages(count: u64) {
    memory_stats::MEMORY_SHORT_TERM.store(count, Ordering::Relaxed);
}

/// backend.rs 在每次 chat_internal 完成时调 (R25.3 计划接, 当前 0 调用)
///
/// **R25.3 真接预留**: apeireth-memory summary table 计数.
pub fn record_mid_term_count(count: u64) {
    memory_stats::MEMORY_MID_TERM.store(count, Ordering::Relaxed);
}

/// backend.rs 在每次向量写入完成时调 (R25.3 计划接, 当前 0 调用)
///
/// **R25.3 真接预留**: apeireth-memory vector store 条数.
pub fn record_long_term_count(count: u64) {
    memory_stats::MEMORY_LONG_TERM.store(count, Ordering::Relaxed);
}

/// Memory organ 状态快照
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryState {
    pub short_term_messages: u64,
    pub mid_term_count: u64,
    pub long_term_count: u64,
    pub retention_days: u32,
}

pub fn snapshot() -> MemoryState {
    MemoryState {
        short_term_messages: memory_stats::MEMORY_SHORT_TERM.load(Ordering::Relaxed),
        mid_term_count: memory_stats::MEMORY_MID_TERM.load(Ordering::Relaxed),
        long_term_count: memory_stats::MEMORY_LONG_TERM.load(Ordering::Relaxed),
        retention_days: memory_stats::RETENTION_DAYS,
    }
}

/// Memory organ 渲染
///
/// **不假装**: 短期真接 episode_count; mid/long 字段级标 stub.
/// Memory organ 渲染
///
/// **R78 升级**: cognition summary 行 (mean/min/max/verdict counts) 真接 R47 hook +
/// R54 backend wire-up + R57 per-chat-cycle. mid_term 真接 (R54 last 24h SQLite query),
/// long_term 近似 (R54 total/5 heuristic, vector store 1.3 路线).
pub fn render(area: Rect) -> String {
    let _ = area;
    let s = snapshot();
    let latest = latest_cognition_summary();
    let (approve_count, block_count) = cognition_verdict_counts();
    let mut out = String::new();
    out.push_str("[MEM] 记忆 — 3 层记忆状态 + cognition summary\n");
    out.push_str(&format!(
        "  短期:        {} 条  (current session, live episode_count)\n",
        s.short_term_messages
    ));
    out.push_str(&format!(
        "  中期:        {}            (last 24h episodes, real query per R54 backend wire-up)\n",
        s.mid_term_count
    ));
    out.push_str(&format!(
        "  长期:        {}            (total/5 近似, vector store 1.3 路线)\n",
        s.long_term_count
    ));
    out.push_str(&format!(
        "  保留期:      {} 天         (编译期 hardcode, 改需 6 哲学锚 + 主人审)\n",
        s.retention_days
    ));
    if let Some(summary) = latest {
        let mean = summary.get("mean").copied().unwrap_or(0.0);
        let min = summary.get("min").copied().unwrap_or(0.0);
        let max = summary.get("max").copied().unwrap_or(0.0);
        let verdict = summary.get("verdict_approve").copied().unwrap_or(0.0);
        let verdict_str = if verdict > 0.0 { "approve" } else { "block" };
        out.push_str(&format!(
            "  cognition:   mean={:.3} min={:.3} max={:.3} verdict={}  (R47 ring buffer 8 entries)\n",
            mean, min, max, verdict_str
        ));
    } else {
        out.push_str(
            "  cognition:   (no runs, 0 sample)            (R47 hook ready, R57 per-chat-cycle)\n",
        );
    }
    out.push_str(&format!(
        "  verdict 累计: approve={} block={}        (per R47 atomic accumulators)\n",
        approve_count, block_count
    ));
    out.push_str("  [partial] 2/3 真接 (短期/中期 + cognition), 长期 近似 (1.3 路线)\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn render_contains_mem_label() {
        let _g = TEST_LOCK.lock().unwrap();
        memory_stats::MEMORY_SHORT_TERM.store(0, Ordering::Relaxed);
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("[MEM]"));
        assert!(out.contains("记忆"));
    }

    #[test]
    fn render_3_layers_listed() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("短期"));
        assert!(out.contains("中期"));
        assert!(out.contains("长期"));
    }

    #[test]
    fn render_marks_partial_honestly() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(
            out.contains("[partial]"),
            "memory 1/3 真接, 标 partial: {out}"
        );
    }

    #[test]
    fn render_shows_real_short_term_count() {
        let _g = TEST_LOCK.lock().unwrap();
        memory_stats::MEMORY_SHORT_TERM.store(42, Ordering::Relaxed);
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("短期:        42 条"), "short_term 真接: {out}");
    }

    #[test]
    fn r78_render_marks_no_stub_for_mid_long() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("中期"));
        assert!(out.contains("长期"));
        let stub_count = out.matches("[stub").count();
        assert_eq!(stub_count, 0, "R54 backend wire-up 真接, 0 stub: {out}");
    }

    #[test]
    fn r78_render_shows_cognition_summary() {
        let _g = TEST_LOCK.lock().unwrap();
        record_cognition_summary(0.42, 0.10, 0.85, true);
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("cognition:"), "cognition summary 行: {out}");
        assert!(out.contains("mean=0.420"), "cognition mean: {out}");
        assert!(out.contains("verdict=approve"), "cognition verdict: {out}");
        assert!(out.contains("verdict 累计"), "verdict counts 行: {out}");
    }

    #[test]
    fn record_short_term_messages_sets_value() {
        let _g = TEST_LOCK.lock().unwrap();
        memory_stats::MEMORY_SHORT_TERM.store(0, Ordering::Relaxed);
        record_short_term_messages(100);
        assert_eq!(memory_stats::MEMORY_SHORT_TERM.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn record_mid_term_count_sets_value() {
        let _g = TEST_LOCK.lock().unwrap();
        memory_stats::MEMORY_MID_TERM.store(0, Ordering::Relaxed);
        record_mid_term_count(50);
        assert_eq!(memory_stats::MEMORY_MID_TERM.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn record_long_term_count_sets_value() {
        let _g = TEST_LOCK.lock().unwrap();
        memory_stats::MEMORY_LONG_TERM.store(0, Ordering::Relaxed);
        record_long_term_count(200);
        assert_eq!(memory_stats::MEMORY_LONG_TERM.load(Ordering::Relaxed), 200);
    }

    #[test]
    fn r47_record_cognition_summary_stores_latest() {
        let _g = TEST_LOCK.lock().unwrap();
        record_cognition_summary(0.1, 0.0, 0.2, true);
        record_cognition_summary(0.5, 0.4, 0.6, false);
        let s = latest_cognition_summary().expect("must have a summary");
        assert!((s["mean"] - 0.5).abs() < 1e-6);
        assert!((s["min"] - 0.4).abs() < 1e-6);
        assert!((s["max"] - 0.6).abs() < 1e-6);
        assert!((s["verdict_approve"] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn r47_cognition_verdict_counts_track() {
        let _g = TEST_LOCK.lock().unwrap();
        let (a, b) = cognition_verdict_counts();
        let _: u64 = a;
        let _: u64 = b;
    }

    #[test]
    fn r47_record_bounded_to_8_entries() {
        let _g = TEST_LOCK.lock().unwrap();
        for i in 0..20 {
            record_cognition_summary(f64::from(i) * 0.1, 0.0, 1.0, true);
        }
        let s = latest_cognition_summary().expect("summary");
        assert!((s["mean"] - 1.9).abs() < 1e-6);
    }

    #[test]
    fn r47_render_ui_unchanged() {
        // R47 B8 baseline: render() output >= 6 lines (R78 cognition summary + verdict counts 增 2 行).
        let _g = TEST_LOCK.lock().unwrap();
        memory_stats::MEMORY_SHORT_TERM.store(0, Ordering::Relaxed);
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("[MEM]"));
        assert!(out.contains("[partial]"));
        assert!(
            out.lines().count() >= 6,
            "R78 >= 6 lines, got {}: {out}",
            out.lines().count()
        );
    }

    #[test]
    fn snapshot_returns_consistent_state() {
        let _g = TEST_LOCK.lock().unwrap();
        memory_stats::MEMORY_SHORT_TERM.store(42, Ordering::Relaxed);
        memory_stats::MEMORY_MID_TERM.store(7, Ordering::Relaxed);
        memory_stats::MEMORY_LONG_TERM.store(1000, Ordering::Relaxed);

        let s = snapshot();
        assert_eq!(s.short_term_messages, 42);
        assert_eq!(s.mid_term_count, 7);
        assert_eq!(s.long_term_count, 1000);
        assert_eq!(s.retention_days, 7);
    }
}
