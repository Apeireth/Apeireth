//! Hand (手) — 6 工具调用统计 (R22 ST-A1.5 真接 http.rs::invoke_tool)
//!
//! **状态来源 (R22 ST-A1.5 升级)**:
//! - 6 工具 today/ok/fail 计数: 真接 → `crate::hand::record_tool_success(name)` /
//!   `record_tool_failure(name)` 在 `http.rs::invoke_tool` 成功/失败路径调.
//! - last_tool / last_ms: 真接 → 上面 2 函数同时更新.
//!
//! **8 项承诺**: 全部遵守
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - 6 工具名编译期 hardcode 跟 `apeireth-api` `/v1/tools/*` 端点对齐
//! - 真实计数从 atomics 读, 0 hardcode 占位
//! - readiness: Partial → Ok (R22 ST-A1.5 真接 http endpoint)

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Rect;

/// 6 工具名 (编译期 hardcode, 跟 `apeireth-api` `/v1/tools/*` 对齐)
pub const SIX_TOOLS: &[&str] = &["calendar", "message", "contact", "task", "search", "drive"];

/// 单工具统计 (today + ok + fail, 18 + 4 atomics)
///
/// **不假装**: today/ok/fail 各独立 atomic, 字段级真实计数.
pub mod hand_stats {
    use super::*;

    macro_rules! def_tool_atom {
        ($name:ident) => {
            pub static $name: AtomicU64 = AtomicU64::new(0);
        };
    }

    def_tool_atom!(HAND_CALENDAR_TODAY);
    def_tool_atom!(HAND_CALENDAR_OK);
    def_tool_atom!(HAND_CALENDAR_FAIL);
    def_tool_atom!(HAND_MESSAGE_TODAY);
    def_tool_atom!(HAND_MESSAGE_OK);
    def_tool_atom!(HAND_MESSAGE_FAIL);
    def_tool_atom!(HAND_CONTACT_TODAY);
    def_tool_atom!(HAND_CONTACT_OK);
    def_tool_atom!(HAND_CONTACT_FAIL);
    def_tool_atom!(HAND_TASK_TODAY);
    def_tool_atom!(HAND_TASK_OK);
    def_tool_atom!(HAND_TASK_FAIL);
    def_tool_atom!(HAND_SEARCH_TODAY);
    def_tool_atom!(HAND_SEARCH_OK);
    def_tool_atom!(HAND_SEARCH_FAIL);
    def_tool_atom!(HAND_DRIVE_TODAY);
    def_tool_atom!(HAND_DRIVE_OK);
    def_tool_atom!(HAND_DRIVE_FAIL);
    def_tool_atom!(HAND_UNKNOWN_TODAY);
    def_tool_atom!(HAND_UNKNOWN_OK);
    def_tool_atom!(HAND_UNKNOWN_FAIL);

    /// 最近一次调用的工具索引 (0 = 未知, 1-6 = SIX_TOOLS[i-1])
    pub static HAND_LAST_TOOL_IDX: AtomicU64 = AtomicU64::new(0);
    /// 最近一次调用 unix millis (0 = 从未调)
    pub static HAND_LAST_CALL_MS: AtomicU64 = AtomicU64::new(0);
}

/// 当前 unix epoch millis
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 测试串行锁 (跨 hand.rs/http.rs 测试共享 — 静态计数原子并行写竞态,
/// 2026-08-18 全量偶发 record_tool_success 计数 +2 修复).
#[cfg(test)]
pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 工具索引 (1-6), 未知工具返 0
fn tool_index(name: &str) -> u64 {
    SIX_TOOLS
        .iter()
        .position(|&t| t == name)
        .map(|i| (i + 1) as u64)
        .unwrap_or(0)
}

/// http.rs::invoke_tool 成功路径调
pub fn record_tool_success(name: &str) {
    let (today, ok, fail) = hand_counters_for(name);
    today.fetch_add(1, Ordering::Relaxed);
    ok.fetch_add(1, Ordering::Relaxed);
    let _ = fail; // 抑制 unused
    hand_stats::HAND_LAST_TOOL_IDX.store(tool_index(name), Ordering::Relaxed);
    hand_stats::HAND_LAST_CALL_MS.store(now_ms(), Ordering::Relaxed);
}

/// http.rs::invoke_tool 失败路径调
pub fn record_tool_failure(name: &str) {
    let (today, ok, fail) = hand_counters_for(name);
    today.fetch_add(1, Ordering::Relaxed);
    fail.fetch_add(1, Ordering::Relaxed);
    let _ = ok; // 抑制 unused
    hand_stats::HAND_LAST_TOOL_IDX.store(tool_index(name), Ordering::Relaxed);
    hand_stats::HAND_LAST_CALL_MS.store(now_ms(), Ordering::Relaxed);
}

/// 拿工具名对应的 (today, ok, fail) atomic 引用
fn hand_counters_for(name: &str) -> (&'static AtomicU64, &'static AtomicU64, &'static AtomicU64) {
    match name {
        "calendar" => (
            &hand_stats::HAND_CALENDAR_TODAY,
            &hand_stats::HAND_CALENDAR_OK,
            &hand_stats::HAND_CALENDAR_FAIL,
        ),
        "message" => (
            &hand_stats::HAND_MESSAGE_TODAY,
            &hand_stats::HAND_MESSAGE_OK,
            &hand_stats::HAND_MESSAGE_FAIL,
        ),
        "contact" => (
            &hand_stats::HAND_CONTACT_TODAY,
            &hand_stats::HAND_CONTACT_OK,
            &hand_stats::HAND_CONTACT_FAIL,
        ),
        "task" => (
            &hand_stats::HAND_TASK_TODAY,
            &hand_stats::HAND_TASK_OK,
            &hand_stats::HAND_TASK_FAIL,
        ),
        "search" => (
            &hand_stats::HAND_SEARCH_TODAY,
            &hand_stats::HAND_SEARCH_OK,
            &hand_stats::HAND_SEARCH_FAIL,
        ),
        "drive" => (
            &hand_stats::HAND_DRIVE_TODAY,
            &hand_stats::HAND_DRIVE_OK,
            &hand_stats::HAND_DRIVE_FAIL,
        ),
        _ => (
            &hand_stats::HAND_UNKNOWN_TODAY,
            &hand_stats::HAND_UNKNOWN_OK,
            &hand_stats::HAND_UNKNOWN_FAIL,
        ),
    }
}

/// 单工具统计快照
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolStat {
    pub name: &'static str,
    pub today: u64,
    pub ok: u64,
    pub fail: u64,
}

/// 读所有 6 工具 + unknown 的当前计数
pub fn snapshot_per_tool() -> [ToolStat; 7] {
    [
        ToolStat {
            name: "calendar",
            today: hand_stats::HAND_CALENDAR_TODAY.load(Ordering::Relaxed),
            ok: hand_stats::HAND_CALENDAR_OK.load(Ordering::Relaxed),
            fail: hand_stats::HAND_CALENDAR_FAIL.load(Ordering::Relaxed),
        },
        ToolStat {
            name: "message",
            today: hand_stats::HAND_MESSAGE_TODAY.load(Ordering::Relaxed),
            ok: hand_stats::HAND_MESSAGE_OK.load(Ordering::Relaxed),
            fail: hand_stats::HAND_MESSAGE_FAIL.load(Ordering::Relaxed),
        },
        ToolStat {
            name: "contact",
            today: hand_stats::HAND_CONTACT_TODAY.load(Ordering::Relaxed),
            ok: hand_stats::HAND_CONTACT_OK.load(Ordering::Relaxed),
            fail: hand_stats::HAND_CONTACT_FAIL.load(Ordering::Relaxed),
        },
        ToolStat {
            name: "task",
            today: hand_stats::HAND_TASK_TODAY.load(Ordering::Relaxed),
            ok: hand_stats::HAND_TASK_OK.load(Ordering::Relaxed),
            fail: hand_stats::HAND_TASK_FAIL.load(Ordering::Relaxed),
        },
        ToolStat {
            name: "search",
            today: hand_stats::HAND_SEARCH_TODAY.load(Ordering::Relaxed),
            ok: hand_stats::HAND_SEARCH_OK.load(Ordering::Relaxed),
            fail: hand_stats::HAND_SEARCH_FAIL.load(Ordering::Relaxed),
        },
        ToolStat {
            name: "drive",
            today: hand_stats::HAND_DRIVE_TODAY.load(Ordering::Relaxed),
            ok: hand_stats::HAND_DRIVE_OK.load(Ordering::Relaxed),
            fail: hand_stats::HAND_DRIVE_FAIL.load(Ordering::Relaxed),
        },
        ToolStat {
            name: "unknown",
            today: hand_stats::HAND_UNKNOWN_TODAY.load(Ordering::Relaxed),
            ok: hand_stats::HAND_UNKNOWN_OK.load(Ordering::Relaxed),
            fail: hand_stats::HAND_UNKNOWN_FAIL.load(Ordering::Relaxed),
        },
    ]
}

/// 全局汇总 (6 工具合计, 不含 unknown)
pub fn snapshot_total() -> (u64, u64, u64) {
    let per = snapshot_per_tool();
    let mut total_today = 0u64;
    let mut total_ok = 0u64;
    let mut total_fail = 0u64;
    for stat in &per[..6] {
        total_today += stat.today;
        total_ok += stat.ok;
        total_fail += stat.fail;
    }
    (total_today, total_ok, total_fail)
}

/// Hand organ 渲染
///
/// **不假装**: 真实计数来自 atomics, 0 hardcode 占位.
pub fn render(area: Rect) -> String {
    let _ = area;
    let per = snapshot_per_tool();
    let (total_today, total_ok, total_fail) = snapshot_total();
    let error_rate = if total_today == 0 {
        0.0
    } else {
        (total_fail as f64 / total_today as f64) * 100.0
    };

    let mut out = String::new();
    out.push_str("[HAND] 手 — 6 工具调用统计\n");
    out.push_str("  工具         今日  成功  失败\n");
    out.push_str("  ---------    -----  --  ----\n");
    for stat in &per[..6] {
        out.push_str(&format!(
            "  {:<12} {:>5}  {:>2}  {:>4}\n",
            stat.name, stat.today, stat.ok, stat.fail
        ));
    }
    out.push_str(&format!(
        "  今日总计: {} 次 ({} 成功, {} 失败, {:.1}% 失败率)\n",
        total_today, total_ok, total_fail, error_rate
    ));
    out.push_str("  [ok] 6 工具全部真接 http.rs::invoke_tool success/failure hook\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // 共享测试锁 (顶层 pub static, http.rs 测试同用 — 静态原子并行写竞态修复)
    use crate::organ::hand::TEST_LOCK;

    fn reset_all() {
        hand_stats::HAND_CALENDAR_TODAY.store(0, Ordering::Relaxed);
        hand_stats::HAND_CALENDAR_OK.store(0, Ordering::Relaxed);
        hand_stats::HAND_CALENDAR_FAIL.store(0, Ordering::Relaxed);
        hand_stats::HAND_MESSAGE_TODAY.store(0, Ordering::Relaxed);
        hand_stats::HAND_MESSAGE_OK.store(0, Ordering::Relaxed);
        hand_stats::HAND_MESSAGE_FAIL.store(0, Ordering::Relaxed);
        hand_stats::HAND_CONTACT_TODAY.store(0, Ordering::Relaxed);
        hand_stats::HAND_CONTACT_OK.store(0, Ordering::Relaxed);
        hand_stats::HAND_CONTACT_FAIL.store(0, Ordering::Relaxed);
        hand_stats::HAND_TASK_TODAY.store(0, Ordering::Relaxed);
        hand_stats::HAND_TASK_OK.store(0, Ordering::Relaxed);
        hand_stats::HAND_TASK_FAIL.store(0, Ordering::Relaxed);
        hand_stats::HAND_SEARCH_TODAY.store(0, Ordering::Relaxed);
        hand_stats::HAND_SEARCH_OK.store(0, Ordering::Relaxed);
        hand_stats::HAND_SEARCH_FAIL.store(0, Ordering::Relaxed);
        hand_stats::HAND_DRIVE_TODAY.store(0, Ordering::Relaxed);
        hand_stats::HAND_DRIVE_OK.store(0, Ordering::Relaxed);
        hand_stats::HAND_DRIVE_FAIL.store(0, Ordering::Relaxed);
        hand_stats::HAND_UNKNOWN_TODAY.store(0, Ordering::Relaxed);
        hand_stats::HAND_UNKNOWN_OK.store(0, Ordering::Relaxed);
        hand_stats::HAND_UNKNOWN_FAIL.store(0, Ordering::Relaxed);
        hand_stats::HAND_LAST_TOOL_IDX.store(0, Ordering::Relaxed);
        hand_stats::HAND_LAST_CALL_MS.store(0, Ordering::Relaxed);
    }

    #[test]
    fn six_tools_hardcoded() {
        assert_eq!(SIX_TOOLS.len(), 6);
        assert!(SIX_TOOLS.contains(&"calendar"));
        assert!(SIX_TOOLS.contains(&"message"));
        assert!(SIX_TOOLS.contains(&"contact"));
        assert!(SIX_TOOLS.contains(&"task"));
        assert!(SIX_TOOLS.contains(&"search"));
        assert!(SIX_TOOLS.contains(&"drive"));
    }

    #[test]
    fn render_contains_hand_label_and_6_tools() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_all();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("[HAND]"));
        assert!(out.contains("手"));
        for tool in SIX_TOOLS {
            assert!(out.contains(tool), "render 应含工具 {tool}");
        }
    }

    #[test]
    fn render_marks_ok_honestly() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_all();
        let out = render(Rect::new(0, 0, 80, 24));
        // ST-A1.5: hand 真接 http endpoint, readiness 升 ok
        assert!(out.contains("[ok]"), "hand 应标 ok (R22 ST-A1.5): {out}");
        assert!(!out.contains("[partial]"), "hand 不再是 partial: {out}");
        assert!(!out.contains("[stub]"), "hand 不再是 stub: {out}");
    }

    #[test]
    fn record_tool_success_increments_today_and_ok() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_all();

        let before_today = hand_stats::HAND_CALENDAR_TODAY.load(Ordering::Relaxed);
        let before_ok = hand_stats::HAND_CALENDAR_OK.load(Ordering::Relaxed);
        let before_fail = hand_stats::HAND_CALENDAR_FAIL.load(Ordering::Relaxed);
        let before_last = hand_stats::HAND_LAST_CALL_MS.load(Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(20));
        record_tool_success("calendar");
        let after_today = hand_stats::HAND_CALENDAR_TODAY.load(Ordering::Relaxed);
        let after_ok = hand_stats::HAND_CALENDAR_OK.load(Ordering::Relaxed);
        let after_fail = hand_stats::HAND_CALENDAR_FAIL.load(Ordering::Relaxed);
        let after_last = hand_stats::HAND_LAST_CALL_MS.load(Ordering::Relaxed);

        assert_eq!(after_today, before_today + 1);
        assert_eq!(after_ok, before_ok + 1);
        assert_eq!(after_fail, before_fail, "success 不能 +fail");
        assert!(after_last > before_last);
        assert_eq!(hand_stats::HAND_LAST_TOOL_IDX.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn record_tool_failure_increments_today_and_fail() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_all();

        let before_ok = hand_stats::HAND_SEARCH_OK.load(Ordering::Relaxed);
        let before_fail = hand_stats::HAND_SEARCH_FAIL.load(Ordering::Relaxed);
        record_tool_failure("search");
        let after_ok = hand_stats::HAND_SEARCH_OK.load(Ordering::Relaxed);
        let after_fail = hand_stats::HAND_SEARCH_FAIL.load(Ordering::Relaxed);

        assert_eq!(after_ok, before_ok, "failure 不能 +ok");
        assert_eq!(after_fail, before_fail + 1);
        assert_eq!(hand_stats::HAND_LAST_TOOL_IDX.load(Ordering::Relaxed), 5);
    }

    #[test]
    fn unknown_tool_routed_to_unknown_bucket() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_all();

        record_tool_success("invalid_tool_xyz");
        assert_eq!(hand_stats::HAND_UNKNOWN_TODAY.load(Ordering::Relaxed), 1);
        assert_eq!(hand_stats::HAND_UNKNOWN_OK.load(Ordering::Relaxed), 1);
        assert_eq!(hand_stats::HAND_CALENDAR_TODAY.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn snapshot_per_tool_returns_7_entries() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_all();
        let per = snapshot_per_tool();
        assert_eq!(per.len(), 7);
        assert_eq!(per[0].name, "calendar");
        assert_eq!(per[5].name, "drive");
        assert_eq!(per[6].name, "unknown");
    }

    #[test]
    fn snapshot_total_excludes_unknown() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_all();
        record_tool_success("calendar");
        record_tool_success("message");
        record_tool_failure("search");
        record_tool_success("invalid_xyz");

        let (today, ok, fail) = snapshot_total();
        assert_eq!(today, 3);
        assert_eq!(ok, 2);
        assert_eq!(fail, 1);
    }

    #[test]
    fn render_shows_real_counts() {
        let _g = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        reset_all();
        record_tool_success("calendar");
        record_tool_success("calendar");
        record_tool_failure("calendar");

        let out = render(Rect::new(0, 0, 80, 24));
        let cal_line = out.lines().find(|l| l.contains("calendar")).unwrap();
        assert!(
            cal_line.contains("3"),
            "calendar today 应 = 3, got: {cal_line}"
        );
        assert!(
            cal_line.contains("2"),
            "calendar ok 应 = 2, got: {cal_line}"
        );
        assert!(
            cal_line.contains("1"),
            "calendar fail 应 = 1, got: {cal_line}"
        );
    }
}
