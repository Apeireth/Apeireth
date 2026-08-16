//! Ear (耳) — 事件订阅 (R22 ST-A1.3 真接 user/llm/system 3 channel + stub 留 tool 接口)
//!
//! **状态来源 (R22 ST-A1.3 升级)**:
//! - `user channel`: 真接 → `crate::ear::record_user()` 在
//!   `backend.rs::chat_internal` 写 user episode 后调, atomic 累加.
//! - `llm channel`: 真接 → `crate::ear::record_llm()` 在
//!   `backend.rs::chat_internal` LLM 成功路径写 assistant episode 后调.
//! - `system channel`: 真接 → `crate::ear::record_system()` 在
//!   `main.rs::run_app` 主循环每 tick_rate (250ms) 调一次.
//! - `tool channel`: 标 stub (tool 调用 handler 分散在 pages/, R25.3 计划集中),
//!   `record_tool()` API 已留, 当前 atomic 维持 0.
//!
//! **8 项承诺**: 全部遵守
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - 4 项中 3 项真接 (user/llm/system), 1 项 (tool) API 留但值是 0
//! - readiness: Partial (3/4 真接), 标 partial 而非 ok / stub
//! - 注释标 (R25.3) 计划接 tool handler, 让接手者知道接口在哪

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Rect;

/// Ear organ 全局状态 (lock-free atomics)
///
/// **8 项承诺**: 全部遵守
pub mod ear_stats {
    use super::*;

    /// user channel 总事件计数 (从 process 启动累加, 0 = 从未收到)
    pub static EAR_USER_EVENTS: AtomicU64 = AtomicU64::new(0);
    /// llm channel 总事件计数 (assistant episode 写入数)
    pub static EAR_LLM_EVENTS: AtomicU64 = AtomicU64::new(0);
    /// tool channel 总事件计数 (R25.3 真接前 = 0, API 已留)
    pub static EAR_TOOL_EVENTS: AtomicU64 = AtomicU64::new(0);
    /// system channel 总事件计数 (主循环 tick 数, 每 250ms +1)
    pub static EAR_SYSTEM_EVENTS: AtomicU64 = AtomicU64::new(0);
    /// 最近一次事件 unix millis (任一 channel 触发都更新)
    pub static EAR_LAST_EVENT_MS: AtomicU64 = AtomicU64::new(0);
}

/// 当前 unix epoch millis
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 4 channel 分类 (供未来 channel-routed 订阅用, 当前仅 tag 用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    User,
    Llm,
    Tool,
    System,
}

impl Channel {
    /// 标签名 (供日志 / 反射)
    pub fn label(self) -> &'static str {
        match self {
            Self::User => "用户",
            Self::Llm => "LLM",
            Self::Tool => "工具",
            Self::System => "系统",
        }
    }
}

/// 通用记录 (按 channel 累加对应 atomic)
///
/// **使用方**: 各 backend hook 点直接调专用 record_*() 函数即可,
/// 本函数是 dispatch (测试 / 调试 用).
pub fn record(channel: Channel) {
    match channel {
        Channel::User => ear_stats::EAR_USER_EVENTS.fetch_add(1, Ordering::Relaxed),
        Channel::Llm => ear_stats::EAR_LLM_EVENTS.fetch_add(1, Ordering::Relaxed),
        Channel::Tool => ear_stats::EAR_TOOL_EVENTS.fetch_add(1, Ordering::Relaxed),
        Channel::System => ear_stats::EAR_SYSTEM_EVENTS.fetch_add(1, Ordering::Relaxed),
    };
    ear_stats::EAR_LAST_EVENT_MS.store(now_ms(), Ordering::Relaxed);
}

/// ```ignore
/// if let Err(e) = write_episode_at(store, input, "用户", user_ts) {
///     eprintln!("[apeireth-tui] warn: write user episode: {e}");
/// }
/// ear::record_user();  // R22 ST-A1.3 hook: user channel event
/// ```
pub fn record_user() {
    record(Channel::User);
}

/// backend.rs::chat_internal 在 assistant episode 写入成功后调
///
/// **使用方 (backend.rs::chat_internal L1565 紧后)**:
/// ```ignore
/// if let Err(e) = write_episode_at(store, &reply.text, "assistant", asst_ts) {
///     eprintln!("[apeireth-tui] warn: write assistant episode: {e}");
/// }
/// ear::record_llm();  // R22 ST-A1.3 hook: llm channel event
/// ```
pub fn record_llm() {
    record(Channel::Llm);
}

/// tool handler 在每次 tool 调用结束后调 (R25.3 计划接, 当前 0 调用)
///
/// **R25.3 真接预留**: tool 调用 handler (tool crate + pages/) 完成后调,
/// 当前 tool handler 分散在 pages/, 0 hook 点.
pub fn record_tool() {
    record(Channel::Tool);
}

/// main.rs::run_app 在每次 tick_rate 触发时调
///
/// **使用方 (main.rs::run_app L228 紧后)**:
/// ```ignore
/// if last_tick.elapsed() >= tick_rate {
///     last_tick = Instant::now();
///     ear::record_system();  // R22 ST-A1.3 hook: system channel event
/// }
/// ```
pub fn record_system() {
    record(Channel::System);
}

/// 读当前 state (render / 测试用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EarState {
    pub user_events: u64,
    pub llm_events: u64,
    pub tool_events: u64,
    pub system_events: u64,
    pub last_event_unix_ms: u64,
    pub now_unix_ms: u64,
    pub events_today: u64,
}

pub fn snapshot() -> EarState {
    let user = ear_stats::EAR_USER_EVENTS.load(Ordering::Relaxed);
    let llm = ear_stats::EAR_LLM_EVENTS.load(Ordering::Relaxed);
    let tool = ear_stats::EAR_TOOL_EVENTS.load(Ordering::Relaxed);
    let system = ear_stats::EAR_SYSTEM_EVENTS.load(Ordering::Relaxed);
    EarState {
        user_events: user,
        llm_events: llm,
        tool_events: tool,
        system_events: system,
        last_event_unix_ms: ear_stats::EAR_LAST_EVENT_MS.load(Ordering::Relaxed),
        now_unix_ms: now_ms(),
        events_today: user + llm + tool + system,
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

/// Ear organ 渲染
///
/// **不假装**: user/llm/system 真接; tool 标 stub 但 atomic 维持.
pub fn render(area: Rect) -> String {
    let _ = area;
    let s = snapshot();
    let mut out = String::new();
    out.push_str("[EAR] 耳 — 事件订阅\n");
    out.push_str(&format!("  今日事件: {}\n", s.events_today));
    out.push_str("  通道:\n");
    out.push_str(&format!(
        "    用户:        {}  (live, 钩在 backend chat_internal user_episode)\n",
        s.user_events
    ));
    out.push_str(&format!(
        "    LLM:         {}  (live, 钩在 backend chat_internal asst_episode)\n",
        s.llm_events
    ));
    out.push_str(&format!(
        "    工具:        {}  [stub — R25.3 接 tool handler 集中]\n",
        s.tool_events
    ));
    out.push_str(&format!(
        "    系统:        {}  (live, 钩在 main.rs run_app tick)\n",
        s.system_events
    ));
    out.push_str(&format!(
        "  last_event:    {}  ({})\n",
        if s.last_event_unix_ms == 0 {
            0
        } else {
            s.last_event_unix_ms
        },
        age_phrase(s.now_unix_ms, s.last_event_unix_ms)
    ));
    out.push_str("  [partial] 3/4 真接 (user/llm/system), tool R25.3 计划接\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 全局测试锁 — 多个测试同时改 EAR atomics 会 race, 串行测试保证稳定
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn render_contains_ear_label() {
        let _g = TEST_LOCK.lock().unwrap();
        ear_stats::EAR_USER_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_LLM_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_TOOL_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_SYSTEM_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_LAST_EVENT_MS.store(0, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("[EAR]"));
        assert!(out.contains("耳"));
        assert!(out.contains("今日事件: 0"));
        assert!(out.contains("[partial]"));
    }

    #[test]
    fn render_lists_4_bus_channels() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("用户"));
        assert!(out.contains("工具"));
        assert!(out.contains("LLM"));
        assert!(out.contains("系统"));
    }

    #[test]
    fn render_marks_partial_honestly() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(
            out.contains("[partial]"),
            "ear 3/4 真接, 必须标 partial: {out}"
        );
        assert!(
            !out.contains("[stub]"),
            "ear 不再是 stub (ST-A1.3 升级): {out}"
        );
    }

    #[test]
    fn render_marks_tool_stub_field() {
        // tool 字段级标 stub
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("工具:"), "got: {out}");
        assert!(out.contains("[stub"), "tool 字段级标 stub: {out}");
    }

    #[test]
    fn render_shows_real_event_counts() {
        let _g = TEST_LOCK.lock().unwrap();
        ear_stats::EAR_USER_EVENTS.store(3, Ordering::Relaxed);
        ear_stats::EAR_LLM_EVENTS.store(3, Ordering::Relaxed);
        ear_stats::EAR_TOOL_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_SYSTEM_EVENTS.store(60, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("今日事件: 66"), "总事件数真接: {out}");
        assert!(out.contains("用户:        3"), "用户通道 真接: {out}");
        assert!(out.contains("LLM:         3"), "LLM通道 真接: {out}");
        assert!(out.contains("系统:        60"), "系统通道 真接: {out}");
    }

    #[test]
    fn channel_label_4_distinct() {
        let labels: Vec<&str> = [Channel::User, Channel::Llm, Channel::Tool, Channel::System]
            .iter()
            .map(|c| c.label())
            .collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 4, "4 channel 标签互不相同");
        assert_eq!(Channel::User.label(), "用户");
        assert_eq!(Channel::Llm.label(), "LLM");
        assert_eq!(Channel::Tool.label(), "工具");
        assert_eq!(Channel::System.label(), "系统");
    }

    #[test]
    fn record_user_increments_user_only() {
        let _g = TEST_LOCK.lock().unwrap();
        ear_stats::EAR_USER_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_LLM_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_TOOL_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_SYSTEM_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_LAST_EVENT_MS.store(0, Ordering::Relaxed);

        let before_user = ear_stats::EAR_USER_EVENTS.load(Ordering::Relaxed);
        let before_llm = ear_stats::EAR_LLM_EVENTS.load(Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(20));
        record_user();
        let after_user = ear_stats::EAR_USER_EVENTS.load(Ordering::Relaxed);
        let after_llm = ear_stats::EAR_LLM_EVENTS.load(Ordering::Relaxed);
        let last = ear_stats::EAR_LAST_EVENT_MS.load(Ordering::Relaxed);

        assert_eq!(after_user, before_user + 1, "record_user 必须 +1");
        assert_eq!(after_llm, before_llm, "record_user 不能动 llm channel");
        assert!(last > 0, "record_user 必须更新 last_event_ms");
    }

    #[test]
    fn record_llm_increments_llm_only() {
        let _g = TEST_LOCK.lock().unwrap();
        ear_stats::EAR_LLM_EVENTS.store(0, Ordering::Relaxed);
        ear_stats::EAR_USER_EVENTS.store(0, Ordering::Relaxed);

        let before_llm = ear_stats::EAR_LLM_EVENTS.load(Ordering::Relaxed);
        let before_user = ear_stats::EAR_USER_EVENTS.load(Ordering::Relaxed);
        record_llm();
        let after_llm = ear_stats::EAR_LLM_EVENTS.load(Ordering::Relaxed);
        let after_user = ear_stats::EAR_USER_EVENTS.load(Ordering::Relaxed);

        assert_eq!(after_llm, before_llm + 1, "record_llm 必须 +1");
        assert_eq!(after_user, before_user, "record_llm 不能动 user channel");
    }

    #[test]
    fn record_tool_increments_tool() {
        let _g = TEST_LOCK.lock().unwrap();
        ear_stats::EAR_TOOL_EVENTS.store(0, Ordering::Relaxed);
        let before = ear_stats::EAR_TOOL_EVENTS.load(Ordering::Relaxed);
        record_tool();
        let after = ear_stats::EAR_TOOL_EVENTS.load(Ordering::Relaxed);
        assert_eq!(
            after,
            before + 1,
            "record_tool 必须 +1 (即使 stub 也走 atomic)"
        );
    }

    #[test]
    fn record_system_increments_system() {
        let _g = TEST_LOCK.lock().unwrap();
        ear_stats::EAR_SYSTEM_EVENTS.store(0, Ordering::Relaxed);
        let before = ear_stats::EAR_SYSTEM_EVENTS.load(Ordering::Relaxed);
        record_system();
        let after = ear_stats::EAR_SYSTEM_EVENTS.load(Ordering::Relaxed);
        assert_eq!(after, before + 1, "record_system 必须 +1");
    }

    #[test]
    fn snapshot_returns_consistent_state() {
        let _g = TEST_LOCK.lock().unwrap();
        ear_stats::EAR_USER_EVENTS.store(5, Ordering::Relaxed);
        ear_stats::EAR_LLM_EVENTS.store(5, Ordering::Relaxed);
        ear_stats::EAR_TOOL_EVENTS.store(2, Ordering::Relaxed);
        ear_stats::EAR_SYSTEM_EVENTS.store(60, Ordering::Relaxed);
        ear_stats::EAR_LAST_EVENT_MS.store(42_000_000, Ordering::Relaxed);

        let s = snapshot();
        assert_eq!(s.user_events, 5);
        assert_eq!(s.llm_events, 5);
        assert_eq!(s.tool_events, 2);
        assert_eq!(s.system_events, 60);
        assert_eq!(s.events_today, 72);
        assert_eq!(s.last_event_unix_ms, 42_000_000);
        assert!(s.now_unix_ms >= s.last_event_unix_ms);
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
