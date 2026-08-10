//! Eye (眼) — 输入监控 (R22 ST-A1.2 真接 keystrokes + stub 留接口)
//!
//! **状态来源 (R22 ST-A1.2 升级)**:
//! - `keystrokes_today`: 真接 → `crate::eye::record_keystroke()` 在 `main.rs`
//!   的 `if key.kind == KeyEventKind::Press` 分支调, atomic 累加.
//! - `mouse_clicks`: 标 stub (R25.3 计划接 crossterm MouseEvent Down),
//!   `record_mouse_click()` API 已留, 当前 atomic 维持 0.
//! - `voice_inputs`: 标 stub (TUI 未接 microphone),
//!   `record_voice_input()` API 已留, 当前 atomic 维持 0.
//! - `attention_focus`: 标 stub (ratatui 焦点未跟踪),
//!   `last_input_ms` 提供, 当前 atomic 维持 0 (无最近输入).
//!
//! **8 项承诺**: 全部遵守
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - 4 项中 1 项真接 (keystrokes_today), 其余 3 项 API 留但值是 0
//!   (不假装鼠标 / 语音 / 焦点已监控)
//! - readiness: Partial (1/4 真接), 标 partial 而非 ok / stub
//! - 注释标 (R25.3) 计划接, 让接手者知道接口在哪

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Rect;

/// Eye organ 全局状态 (lock-free atomics)
///
/// **8 项承诺**: 全部遵守
pub mod eye_stats {
    use super::*;

    /// 总按键计数 (从 process 启动累加, 0 = 从未按)
    pub static EYE_KEYSTROKES_TOTAL: AtomicU64 = AtomicU64::new(0);
    /// 总鼠标点击计数 (R25.3 真接前 = 0, API 已留)
    pub static EYE_MOUSE_CLICKS_TOTAL: AtomicU64 = AtomicU64::new(0);
    /// 总语音输入计数 (TUI 未接 microphone = 0, API 已留)
    pub static EYE_VOICE_INPUTS_TOTAL: AtomicU64 = AtomicU64::new(0);
    /// 最近一次输入 unix millis (0 = 从未输入; keystroke / mouse / voice 任一触发都更新)
    pub static EYE_LAST_INPUT_MS: AtomicU64 = AtomicU64::new(0);
}

/// 当前 unix epoch millis
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 让 main.rs handle_key 在 KeyEventKind::Press 时调
///
/// **使用方 (main.rs::run 主循环)**:
/// ```ignore
/// if key.kind == KeyEventKind::Press {
///     eye::record_keystroke();  // R22 ST-A1.2 hook
///     if handle_key(app, key) {
///         return Ok(());
///     }
/// }
/// ```
pub fn record_keystroke() {
    eye_stats::EYE_KEYSTROKES_TOTAL.fetch_add(1, Ordering::Relaxed);
    eye_stats::EYE_LAST_INPUT_MS.store(now_ms(), Ordering::Relaxed);
}

/// 让 main.rs handle_mouse 在 Event::Mouse(MouseButton::Left) Down 时调
///
/// **R25.3 真接预留**: crossterm MouseEvent Down 时调,
/// 当前 TUI 未启用 mouse capture, 调用 0 次.
pub fn record_mouse_click() {
    eye_stats::EYE_MOUSE_CLICKS_TOTAL.fetch_add(1, Ordering::Relaxed);
    eye_stats::EYE_LAST_INPUT_MS.store(now_ms(), Ordering::Relaxed);
}

/// 让 voice subsystem 在 STT 识别到 utterance 时调
///
/// **R25.3 真接预留**: apeireth-sdk-voice STT pipeline 调,
/// 当前 TUI 未启 mic = 0 次.
pub fn record_voice_input() {
    eye_stats::EYE_VOICE_INPUTS_TOTAL.fetch_add(1, Ordering::Relaxed);
    eye_stats::EYE_LAST_INPUT_MS.store(now_ms(), Ordering::Relaxed);
}

/// 读当前 state (render / 测试用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EyeState {
    pub keystrokes_total: u64,
    pub mouse_clicks_total: u64,
    pub voice_inputs_total: u64,
    pub last_input_unix_ms: u64,
    pub now_unix_ms: u64,
}

pub fn snapshot() -> EyeState {
    EyeState {
        keystrokes_total: eye_stats::EYE_KEYSTROKES_TOTAL.load(Ordering::Relaxed),
        mouse_clicks_total: eye_stats::EYE_MOUSE_CLICKS_TOTAL.load(Ordering::Relaxed),
        voice_inputs_total: eye_stats::EYE_VOICE_INPUTS_TOTAL.load(Ordering::Relaxed),
        last_input_unix_ms: eye_stats::EYE_LAST_INPUT_MS.load(Ordering::Relaxed),
        now_unix_ms: now_ms(),
    }
}

/// 把 unix ms 距离算成人类可读 (Xh Ym Zs ago / Zs ago / never)
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

/// Eye organ 渲染
///
/// **不假装**: keystrokes 真接; mouse / voice / focus 标 stub 但 atomic 维持.
pub fn render(area: Rect) -> String {
    let _ = area;
    let s = snapshot();
    let mut out = String::new();
    out.push_str("[EYE] 眼\n");
    out.push_str(&format!(
        "  今日按键: {}  (live, 钩在 main.rs handle_key)\n",
        s.keystrokes_total
    ));
    out.push_str(&format!(
        "  鼠标点击:         {}  [stub — R25.3 接 crossterm MouseEvent]\n",
        s.mouse_clicks_total
    ));
    out.push_str(&format!(
        "  语音输入:         {}  [stub — mic 未接]\n",
        s.voice_inputs_total
    ));
    out.push_str(&format!(
        "  关注焦点:        -    [stub — ratatui focus 未跟踪]\n"
    ));
    out.push_str(&format!(
        "  上次输入:        {}  ({})\n",
        if s.last_input_unix_ms == 0 { 0 } else { s.last_input_unix_ms },
        age_phrase(s.now_unix_ms, s.last_input_unix_ms)
    ));
    out.push_str("  [partial] 1/4 真接 (按键), 其余 3 项 R25.3 计划接\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 全局测试锁 — 多个测试同时改 EYE atomics 会 race, 串行测试保证稳定
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn render_contains_eye_label() {
        let _g = TEST_LOCK.lock().unwrap();
        eye_stats::EYE_KEYSTROKES_TOTAL.store(0, Ordering::Relaxed);
        eye_stats::EYE_MOUSE_CLICKS_TOTAL.store(0, Ordering::Relaxed);
        eye_stats::EYE_VOICE_INPUTS_TOTAL.store(0, Ordering::Relaxed);
        eye_stats::EYE_LAST_INPUT_MS.store(0, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 40, 10));
        assert!(out.contains("[EYE]"));
        assert!(out.contains("眼"));
        assert!(out.contains("今日按键: 0"));
        assert!(out.contains("[partial]"));
    }

    #[test]
    fn render_lists_4_input_channels() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("按键"));
        assert!(out.contains("鼠标"));
        assert!(out.contains("语音"));
        assert!(out.contains("关注焦点"));
    }

    #[test]
    fn render_marks_partial_honestly() {
        // R22 ST-A1.2: 1/4 真接, 标 partial 而非 ok / stub
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("[partial]"), "eye 1/4 真接, 必须标 partial: {out}");
        assert!(!out.contains("[stub]"), "eye 不再是 stub (ST-A1.2 升级): {out}");
    }

    #[test]
    fn render_marks_stub_per_field() {
        // mouse / voice / focus 字段级标 stub
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("鼠标点击:"), "got: {out}");
        assert!(out.contains("[stub"), "mouse / voice / focus 字段级标 stub: {out}");
    }

    #[test]
    fn render_shows_real_keystroke_count() {
        let _g = TEST_LOCK.lock().unwrap();
        eye_stats::EYE_KEYSTROKES_TOTAL.store(42, Ordering::Relaxed);
        eye_stats::EYE_LAST_INPUT_MS.store(0, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("今日按键: 42"), "keystrokes 真接计数渲染: {out}");
    }

    #[test]
    fn record_keystroke_increments_and_updates_last() {
        let _g = TEST_LOCK.lock().unwrap();
        eye_stats::EYE_KEYSTROKES_TOTAL.store(0, Ordering::Relaxed);
        eye_stats::EYE_LAST_INPUT_MS.store(0, Ordering::Relaxed);

        let before_ks = eye_stats::EYE_KEYSTROKES_TOTAL.load(Ordering::Relaxed);
        let before_last = eye_stats::EYE_LAST_INPUT_MS.load(Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(20));
        record_keystroke();
        let after_ks = eye_stats::EYE_KEYSTROKES_TOTAL.load(Ordering::Relaxed);
        let after_last = eye_stats::EYE_LAST_INPUT_MS.load(Ordering::Relaxed);

        assert_eq!(after_ks, before_ks + 1, "record_keystroke 必须 +1");
        assert!(after_last > before_last, "record_keystroke 必须更新 last_input_ms");
    }

    #[test]
    fn record_mouse_click_increments() {
        let _g = TEST_LOCK.lock().unwrap();
        eye_stats::EYE_MOUSE_CLICKS_TOTAL.store(0, Ordering::Relaxed);

        let before = eye_stats::EYE_MOUSE_CLICKS_TOTAL.load(Ordering::Relaxed);
        record_mouse_click();
        let after = eye_stats::EYE_MOUSE_CLICKS_TOTAL.load(Ordering::Relaxed);

        assert_eq!(after, before + 1, "record_mouse_click 必须 +1 (即使 stub 也走 atomic)");
    }

    #[test]
    fn record_voice_input_increments() {
        let _g = TEST_LOCK.lock().unwrap();
        eye_stats::EYE_VOICE_INPUTS_TOTAL.store(0, Ordering::Relaxed);

        let before = eye_stats::EYE_VOICE_INPUTS_TOTAL.load(Ordering::Relaxed);
        record_voice_input();
        let after = eye_stats::EYE_VOICE_INPUTS_TOTAL.load(Ordering::Relaxed);

        assert_eq!(after, before + 1, "record_voice_input 必须 +1 (即使 stub 也走 atomic)");
    }

    #[test]
    fn snapshot_returns_consistent_state() {
        let _g = TEST_LOCK.lock().unwrap();
        eye_stats::EYE_KEYSTROKES_TOTAL.store(100, Ordering::Relaxed);
        eye_stats::EYE_MOUSE_CLICKS_TOTAL.store(5, Ordering::Relaxed);
        eye_stats::EYE_VOICE_INPUTS_TOTAL.store(0, Ordering::Relaxed);
        eye_stats::EYE_LAST_INPUT_MS.store(42_000_000, Ordering::Relaxed);

        let s = snapshot();
        assert_eq!(s.keystrokes_total, 100);
        assert_eq!(s.mouse_clicks_total, 5);
        assert_eq!(s.voice_inputs_total, 0);
        assert_eq!(s.last_input_unix_ms, 42_000_000);
        assert!(s.now_unix_ms >= s.last_input_unix_ms);
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
