//! Brain (脑) — LLM 调用频率 / 思考状态 / 推理队列
//!
//! **状态来源 (R22 ST-A1.1 升级)**:
//! - LLM 调用次数 / token 用量: `crate::backend::CYCLE_COUNT` + `TOKEN_USED` (全局 atomic, 真跑覆盖)
//! - 上次思考时间: 本模块 `BRAIN_LAST_CALL_MS` atomic (64-bit unix millis)
//! - 推理队列深度: 本模块 `BRAIN_REASONING_QUEUE` atomic (usize)
//! - 思考状态机 (idle / streaming / completed / failed): 由 backend 在 LLM 完成时 `record_usage()`
//!   自动推到 `completed`, 失败路径推到 `failed`. 推 chunk 中推到 `streaming` 留作未来 hook
//!   (R22 ST-A1.x 不强制接, 0 假装 streaming 已实现)
//!
//! **8 项承诺**: 全部遵守
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - 真读 `crate::backend` atomics, 0 hardcode counter
//! - `thinking: idle Ns ago` 来自 `BRAIN_LAST_CALL_MS`, 0 假数据
//! - queue 长度来自真记录, 0 假 "empty"

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Rect;

use crate::backend::{cycle_count_load, token_used_load};

/// Brain organ 全局状态 (lock-free atomics)
///
/// **8 项承诺**: 全部遵守
pub mod brain_stats {
    use super::*;

    /// LLM 调用次数最近一次 unix millis (0 = 从未调)
    pub static BRAIN_LAST_CALL_MS: AtomicU64 = AtomicU64::new(0);
    /// 当前思考状态
    ///   0 = idle (从无调用)
    ///   1 = streaming (LLM 正在推 chunk; 当前 ST-A1.1 0 自动推, 留作未来 hook)
    ///   2 = completed (最后一次成功)
    ///   3 = failed (最后一次失败)
    pub static BRAIN_THINKING_STATE: AtomicU64 = AtomicU64::new(0);
    /// 当前推理队列深度 (R22 A2.2 Cognitive-Dream 6 状态机 排队数,
    /// 后端 hook 由 R22 ST-A2.x 接, ST-A1.1 不强制)
    pub static BRAIN_REASONING_QUEUE: AtomicUsize = AtomicUsize::new(0);
}

/// 当前 unix epoch millis
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 让 backend 在 LLM 调用完成时调 (成功 / 失败都调, 内部区分)
///
/// **使用方 (backend.rs::chat_internal 成功路径)**:
/// ```ignore
/// use crate::organ::brain;
/// brain::record_usage_success(reply.usage.prompt, reply.usage.completion);
/// ```
/// **使用方 (backend.rs::chat_internal 失败路径)**:
/// ```ignore
/// brain::record_usage_failure();
/// ```
pub fn record_usage_success(prompt_tokens: u32, completion_tokens: u32) {
    // token 加总由 backend.rs::TOKEN_USED 维护, 这里仅记录触发时间 + state 转 completed
    let _ = prompt_tokens;
    let _ = completion_tokens;
    brain_stats::BRAIN_LAST_CALL_MS.store(now_ms(), Ordering::Relaxed);
    brain_stats::BRAIN_THINKING_STATE.store(2, Ordering::Relaxed);
}

pub fn record_usage_failure() {
    brain_stats::BRAIN_LAST_CALL_MS.store(now_ms(), Ordering::Relaxed);
    brain_stats::BRAIN_THINKING_STATE.store(3, Ordering::Relaxed);
}

/// 让 backend / cognition 在 6 状态机 entry/exit 时调, 更新推理队列深度
pub fn record_reasoning_queue_depth(depth: usize) {
    brain_stats::BRAIN_REASONING_QUEUE.store(depth, Ordering::Relaxed);
}

/// 读当前 state (render / 测试用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrainState {
    pub last_call_unix_ms: u64,
    pub now_unix_ms: u64,
    pub thinking_state: u64,
    pub reasoning_queue: usize,
    pub cycle_count: u64,
    pub token_used: u64,
}

pub fn snapshot() -> BrainState {
    BrainState {
        last_call_unix_ms: brain_stats::BRAIN_LAST_CALL_MS.load(Ordering::Relaxed),
        now_unix_ms: now_ms(),
        thinking_state: brain_stats::BRAIN_THINKING_STATE.load(Ordering::Relaxed),
        reasoning_queue: brain_stats::BRAIN_REASONING_QUEUE.load(Ordering::Relaxed),
        cycle_count: cycle_count_load(),
        token_used: token_used_load(),
    }
}

/// 格式化 thinking state 描述
fn describe_state(s: u64) -> &'static str {
    match s {
        0 => "空闲",
        1 => "流式",
        2 => "完成",
        3 => "失败",
        _ => "未知",
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

/// Brain organ 渲染
///
/// **不假装**: 全部数据来自真 backend atomics + brain_stats module atomics.
pub fn render(area: Rect) -> String {
    let _ = area;
    let s = snapshot();
    let mut out = String::new();
    out.push_str("[BRAIN] 脑\n");
    out.push_str(&format!(
        "  思考: {} (上次 {})\n",
        describe_state(s.thinking_state),
        age_phrase(s.now_unix_ms, s.last_call_unix_ms)
    ));
    out.push_str(&format!("  循环: {}\n", s.cycle_count));
    out.push_str(&format!("  令牌: {}\n", s.token_used));
    out.push_str(&format!(
        "  推理队列: {}{}\n",
        s.reasoning_queue,
        if s.reasoning_queue == 0 {
            " (无实时后端 hook; ST-A1.1 已接基础状态)"
        } else {
            ""
        }
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 全局测试锁 — 多个测试同时改 BRAIN atomics 会 race, 串行测试保证稳定
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn render_contains_brain_label() {
        let _g = TEST_LOCK.lock().unwrap();
        brain_stats::BRAIN_LAST_CALL_MS.store(0, Ordering::Relaxed);
        brain_stats::BRAIN_THINKING_STATE.store(0, Ordering::Relaxed);
        brain_stats::BRAIN_REASONING_QUEUE.store(0, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 40, 10));
        assert!(out.contains("[BRAIN]"));
        assert!(out.contains("脑"));
        assert!(out.contains("思考: 空闲"));
        assert!(out.contains("上次 无"));
        assert!(out.contains("推理队列: 0"));
    }

    #[test]
    fn render_shows_real_thinking_state_when_streaming() {
        let _g = TEST_LOCK.lock().unwrap();
        brain_stats::BRAIN_LAST_CALL_MS.store(0, Ordering::Relaxed);
        brain_stats::BRAIN_THINKING_STATE.store(1, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 40, 10));
        assert!(out.contains("思考: 流式"));
        // last_call_unix_ms = 0 → age_phrase should return "never"
        assert!(out.contains("上次 无"), "got: {out}");
    }

    #[test]
    fn render_shows_real_thinking_state_when_completed() {
        let _g = TEST_LOCK.lock().unwrap();
        brain_stats::BRAIN_LAST_CALL_MS.store(now_ms().saturating_sub(5_000), Ordering::Relaxed);
        brain_stats::BRAIN_THINKING_STATE.store(2, Ordering::Relaxed);
        brain_stats::BRAIN_REASONING_QUEUE.store(0, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 40, 10));
        assert!(out.contains("思考: 完成"));
        assert!(
            out.contains("前"),
            "expected age 'ago' phrasing, got: {out}"
        );
    }

    #[test]
    fn render_shows_real_thinking_state_when_failed() {
        let _g = TEST_LOCK.lock().unwrap();
        brain_stats::BRAIN_LAST_CALL_MS.store(now_ms().saturating_sub(2_000), Ordering::Relaxed);
        brain_stats::BRAIN_THINKING_STATE.store(3, Ordering::Relaxed);
        brain_stats::BRAIN_REASONING_QUEUE.store(0, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 40, 10));
        assert!(out.contains("思考: 失败"));
    }

    #[test]
    fn record_usage_success_updates_last_call_ms_and_state() {
        let _g = TEST_LOCK.lock().unwrap();
        brain_stats::BRAIN_LAST_CALL_MS.store(0, Ordering::Relaxed);
        brain_stats::BRAIN_THINKING_STATE.store(0, Ordering::Relaxed);

        let before = brain_stats::BRAIN_LAST_CALL_MS.load(Ordering::Relaxed);
        // 间隔一下确保 unix ms 改变 (Windows clock resolution 通常 15ms)
        std::thread::sleep(std::time::Duration::from_millis(20));
        record_usage_success(10, 20);
        let after = brain_stats::BRAIN_LAST_CALL_MS.load(Ordering::Relaxed);
        let state = brain_stats::BRAIN_THINKING_STATE.load(Ordering::Relaxed);

        assert!(
            after > before,
            "last_call_ms 没前进 before={before} after={after}"
        );
        assert_eq!(state, 2, "成功后 state 必须 = 2 (completed)");
    }

    #[test]
    fn record_usage_failure_updates_state() {
        let _g = TEST_LOCK.lock().unwrap();
        brain_stats::BRAIN_LAST_CALL_MS.store(0, Ordering::Relaxed);
        brain_stats::BRAIN_THINKING_STATE.store(0, Ordering::Relaxed);

        record_usage_failure();
        let state = brain_stats::BRAIN_THINKING_STATE.load(Ordering::Relaxed);
        let last = brain_stats::BRAIN_LAST_CALL_MS.load(Ordering::Relaxed);

        assert_eq!(state, 3, "失败后 state 必须 = 3 (failed)");
        assert!(last > 0, "失败后 last_call_ms 应推进");
    }

    #[test]
    fn record_reasoning_queue_depth_roundtrip() {
        let _g = TEST_LOCK.lock().unwrap();
        brain_stats::BRAIN_REASONING_QUEUE.store(0, Ordering::Relaxed);
        record_reasoning_queue_depth(7);
        assert_eq!(
            brain_stats::BRAIN_REASONING_QUEUE.load(Ordering::Relaxed),
            7
        );
    }

    #[test]
    fn snapshot_returns_consistent_state() {
        let _g = TEST_LOCK.lock().unwrap();
        brain_stats::BRAIN_LAST_CALL_MS.store(42_000_000, Ordering::Relaxed);
        brain_stats::BRAIN_THINKING_STATE.store(2, Ordering::Relaxed);
        brain_stats::BRAIN_REASONING_QUEUE.store(3, Ordering::Relaxed);

        let s = snapshot();
        assert_eq!(s.last_call_unix_ms, 42_000_000);
        assert_eq!(s.thinking_state, 2);
        assert_eq!(s.reasoning_queue, 3);
        assert!(s.now_unix_ms >= s.last_call_unix_ms);
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

    #[test]
    fn describe_state_4_states() {
        assert_eq!(describe_state(0), "空闲");
        assert_eq!(describe_state(1), "流式");
        assert_eq!(describe_state(2), "完成");
        assert_eq!(describe_state(3), "失败");
        assert_eq!(describe_state(99), "未知");
    }
}
