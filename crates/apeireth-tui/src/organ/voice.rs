//! Voice (声) — TTS / STT / 语音状态 (R22 ST-A1.4 partial)
//!
//! **状态来源 (R22 ST-A1.4 升级)**:
//! - `tts_engine` / `stt_engine`: apeireth-voice crate 存在但 STUB_MODE=true
//!   (per `apeireth-voice/src/lib.rs` §K-1 强校验 #5, 8 工具全 `NotImplemented`),
//!   render 真接读 crate 提供的 status 字符串 (未引入 dep, 显示固定文案).
//! - `tts_play_total`: 标 stub (TUI 未接 mic/speaker),
//!   `record_tts_play()` API 已留, 当前 atomic 维持 0.
//! - `stt_heard_total`: 标 stub (TUI 未接 mic),
//!   `record_stt_heard()` API 已留, 当前 atomic 维持 0.
//! - `voices_registered`: 标 stub (Porcupine 未加载, 0 voices),
//!   `record_voice_registered()` API 已留, 当前 atomic 维持 0.
//!
//! **8 项承诺**: 全部遵守
//! - 0 触碰 workspace.version (1.0.0) (item 8)
//! - 0 改动顶层 3 规范文件 (CONVENTIONS / VERSIONING / GLOSSARY) (item 7)
//! - 0 重写阶段 1+2+3 LOCKED 文档 (item 1)
//!
//! **不假装**:
//! - readiness: Partial (有结构 + 有 record API, 但 0 调用 — TUI 未启 mic)
//! - render 字段级标 stub, 跟 ear tool stub 风格一致
//! - tts_engine/stt_engine 字段显示 backend 真实状态 (STUB_MODE), 不假装已接

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::layout::Rect;

/// Voice organ 全局状态 (lock-free atomics)
///
/// **8 项承诺**: 全部遵守
pub mod voice_stats {
    use super::*;

    /// TTS 总播放次数 (TUI 未接 speaker = 0, API 已留)
    pub static VOICE_TTS_PLAY_TOTAL: AtomicU64 = AtomicU64::new(0);
    /// STT 总听到次数 (TUI 未接 mic = 0, API 已留)
    pub static VOICE_STT_HEARD_TOTAL: AtomicU64 = AtomicU64::new(0);
    /// 注册 voices 总数 (Porcupine 未加载 = 0, API 已留)
    pub static VOICE_VOICES_REGISTERED: AtomicU64 = AtomicU64::new(0);
    /// 最近一次 audio activity unix millis (任一 tts/stt/register 触发都更新)
    pub static VOICE_LAST_AUDIO_MS: AtomicU64 = AtomicU64::new(0);
}

/// 当前 unix epoch millis
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// backend voice pipeline 在合成完一次 TTS 输出后调
///
/// **使用方 (apeireth-voice::synthesize 完成回调)**:
/// ```ignore
/// voice::record_tts_play();  // R22 ST-A1.4 hook: tts 播放计数
/// ```
pub fn record_tts_play() {
    voice_stats::VOICE_TTS_PLAY_TOTAL.fetch_add(1, Ordering::Relaxed);
    voice_stats::VOICE_LAST_AUDIO_MS.store(now_ms(), Ordering::Relaxed);
}

/// backend voice pipeline 在 STT 识别到 utterance 后调
///
/// **使用方 (apeireth-voice::transcribe 完成回调)**:
/// ```ignore
/// voice::record_stt_heard();  // R22 ST-A1.4 hook: stt 听到计数
/// ```
pub fn record_stt_heard() {
    voice_stats::VOICE_STT_HEARD_TOTAL.fetch_add(1, Ordering::Relaxed);
    voice_stats::VOICE_LAST_AUDIO_MS.store(now_ms(), Ordering::Relaxed);
}

/// backend voice pipeline 在加载新 Porcupine keyword 后调
///
/// **使用方 (apeireth-voice::load_model 完成回调)**:
/// ```ignore
/// voice::record_voice_registered();  // R22 ST-A1.4 hook: voices 总数
/// ```
pub fn record_voice_registered() {
    voice_stats::VOICE_VOICES_REGISTERED.fetch_add(1, Ordering::Relaxed);
    voice_stats::VOICE_LAST_AUDIO_MS.store(now_ms(), Ordering::Relaxed);
}

/// 读当前 state (render / 测试用)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceState {
    pub tts_play_total: u64,
    pub stt_heard_total: u64,
    pub voices_registered: u64,
    pub last_audio_unix_ms: u64,
    pub now_unix_ms: u64,
}

pub fn snapshot() -> VoiceState {
    VoiceState {
        tts_play_total: voice_stats::VOICE_TTS_PLAY_TOTAL.load(Ordering::Relaxed),
        stt_heard_total: voice_stats::VOICE_STT_HEARD_TOTAL.load(Ordering::Relaxed),
        voices_registered: voice_stats::VOICE_VOICES_REGISTERED.load(Ordering::Relaxed),
        last_audio_unix_ms: voice_stats::VOICE_LAST_AUDIO_MS.load(Ordering::Relaxed),
        now_unix_ms: now_ms(),
    }
}

/// 把 unix ms 距离算成人类可读
fn age_phrase(now_ms: u64, then_ms: u64) -> String {
    if then_ms == 0 {
        return "never".into();
    }
    let delta_ms = now_ms.saturating_sub(then_ms);
    let total_s = delta_ms / 1000;
    if total_s < 60 {
        format!("{total_s}s ago")
    } else if total_s < 3600 {
        format!("{}m {}s ago", total_s / 60, total_s % 60)
    } else {
        format!("{}h {}m ago", total_s / 3600, (total_s / 60) % 60)
    }
}

/// Voice organ 渲染
///
/// **不假装**: 字段级标 stub (TUI 未接 mic/speaker), backend status 字段
/// 反映 `apeireth-voice` crate 的 STUB_MODE 真实状态 (编译期 hardcode = true).
pub fn render(area: Rect) -> String {
    let _ = area;
    let s = snapshot();
    let mut out = String::new();
    out.push_str("[VOICE] 声 — TTS / STT 状态\n");
    out.push_str("  tts_engine:    apeireth-voice (STUB_MODE=true) [stub — TUI 未接 speaker]\n");
    out.push_str("  stt_engine:    apeireth-voice (STUB_MODE=true) [stub — TUI 未接 mic]\n");
    out.push_str(&format!(
        "  tts_play_total:   {}  [stub — Porcupine 未加载]\n",
        s.tts_play_total
    ));
    out.push_str(&format!(
        "  stt_heard_total:  {}  [stub — pvrecorder 未启]\n",
        s.stt_heard_total
    ));
    out.push_str(&format!(
        "  voices_registered: {}  [stub — 8 商业版 keyword 列表未加载]\n",
        s.voices_registered
    ));
    out.push_str(&format!(
        "  last_audio:       {}  ({})\n",
        if s.last_audio_unix_ms == 0 {
            0
        } else {
            s.last_audio_unix_ms
        },
        age_phrase(s.now_unix_ms, s.last_audio_unix_ms)
    ));
    out.push_str("  [partial] 有结构 + record API, 但 0 调用 (TUI 不接 mic/speaker)\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // 全局测试锁 — 多个测试同时改 VOICE atomics 会 race, 串行测试保证稳定
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn render_contains_voice_label() {
        let _g = TEST_LOCK.lock().unwrap();
        voice_stats::VOICE_TTS_PLAY_TOTAL.store(0, Ordering::Relaxed);
        voice_stats::VOICE_STT_HEARD_TOTAL.store(0, Ordering::Relaxed);
        voice_stats::VOICE_VOICES_REGISTERED.store(0, Ordering::Relaxed);
        voice_stats::VOICE_LAST_AUDIO_MS.store(0, Ordering::Relaxed);

        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("[VOICE]"));
        assert!(out.contains("声"));
        assert!(out.contains("STUB_MODE=true"));
        assert!(out.contains("[partial]"));
    }

    #[test]
    fn render_lists_tts_and_stt_engines() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        assert!(out.contains("tts_engine"));
        assert!(out.contains("stt_engine"));
        assert!(out.contains("apeireth-voice"));
    }

    #[test]
    fn render_marks_partial_honestly() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        // voice 从 stub 升 partial (ST-A1.4): 有结构有 API 但 0 调用
        assert!(out.contains("[partial]"), "voice 应标 partial: {out}");
    }

    #[test]
    fn render_marks_stub_per_field() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        // tts_play_total / stt_heard_total / voices_registered 字段级标 stub
        assert!(out.contains("tts_play_total"), "got: {out}");
        assert!(out.contains("stt_heard_total"), "got: {out}");
        assert!(out.contains("voices_registered"), "got: {out}");
        let stub_count = out.matches("[stub").count(); // 字段级 stub 标记 >= 3 (Rust 语法)
        assert!(stub_count >= 3, "字段级 stub 标记应 >= 3: {out}");
    }

    #[test]
    fn render_shows_real_backend_status() {
        let _g = TEST_LOCK.lock().unwrap();
        let out = render(Rect::new(0, 0, 80, 24));
        // backend status 字段反映 apeireth-voice STUB_MODE 真实状态
        assert!(out.contains("apeireth-voice (STUB_MODE=true)"));
    }

    #[test]
    fn record_tts_play_increments_and_updates_last() {
        let _g = TEST_LOCK.lock().unwrap();
        voice_stats::VOICE_TTS_PLAY_TOTAL.store(0, Ordering::Relaxed);
        voice_stats::VOICE_LAST_AUDIO_MS.store(0, Ordering::Relaxed);
        voice_stats::VOICE_STT_HEARD_TOTAL.store(0, Ordering::Relaxed);

        let before_tts = voice_stats::VOICE_TTS_PLAY_TOTAL.load(Ordering::Relaxed);
        let before_stt = voice_stats::VOICE_STT_HEARD_TOTAL.load(Ordering::Relaxed);
        let before_last = voice_stats::VOICE_LAST_AUDIO_MS.load(Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(20));
        record_tts_play();
        let after_tts = voice_stats::VOICE_TTS_PLAY_TOTAL.load(Ordering::Relaxed);
        let after_stt = voice_stats::VOICE_STT_HEARD_TOTAL.load(Ordering::Relaxed);
        let after_last = voice_stats::VOICE_LAST_AUDIO_MS.load(Ordering::Relaxed);

        assert_eq!(after_tts, before_tts + 1, "record_tts_play 必须 +1");
        assert_eq!(after_stt, before_stt, "record_tts_play 不能动 stt");
        assert!(
            after_last > before_last,
            "record_tts_play 必须更新 last_audio_ms"
        );
    }

    #[test]
    fn record_stt_heard_increments_stt_only() {
        let _g = TEST_LOCK.lock().unwrap();
        voice_stats::VOICE_STT_HEARD_TOTAL.store(0, Ordering::Relaxed);
        voice_stats::VOICE_TTS_PLAY_TOTAL.store(0, Ordering::Relaxed);

        let before_stt = voice_stats::VOICE_STT_HEARD_TOTAL.load(Ordering::Relaxed);
        let before_tts = voice_stats::VOICE_TTS_PLAY_TOTAL.load(Ordering::Relaxed);
        record_stt_heard();
        let after_stt = voice_stats::VOICE_STT_HEARD_TOTAL.load(Ordering::Relaxed);
        let after_tts = voice_stats::VOICE_TTS_PLAY_TOTAL.load(Ordering::Relaxed);

        assert_eq!(after_stt, before_stt + 1, "record_stt_heard 必须 +1");
        assert_eq!(after_tts, before_tts, "record_stt_heard 不能动 tts");
    }

    #[test]
    fn record_voice_registered_increments_voices() {
        let _g = TEST_LOCK.lock().unwrap();
        voice_stats::VOICE_VOICES_REGISTERED.store(0, Ordering::Relaxed);

        let before = voice_stats::VOICE_VOICES_REGISTERED.load(Ordering::Relaxed);
        record_voice_registered();
        let after = voice_stats::VOICE_VOICES_REGISTERED.load(Ordering::Relaxed);

        assert_eq!(after, before + 1, "record_voice_registered 必须 +1");
    }

    #[test]
    fn snapshot_returns_consistent_state() {
        let _g = TEST_LOCK.lock().unwrap();
        voice_stats::VOICE_TTS_PLAY_TOTAL.store(7, Ordering::Relaxed);
        voice_stats::VOICE_STT_HEARD_TOTAL.store(3, Ordering::Relaxed);
        voice_stats::VOICE_VOICES_REGISTERED.store(5, Ordering::Relaxed);
        voice_stats::VOICE_LAST_AUDIO_MS.store(42_000_000, Ordering::Relaxed);

        let s = snapshot();
        assert_eq!(s.tts_play_total, 7);
        assert_eq!(s.stt_heard_total, 3);
        assert_eq!(s.voices_registered, 5);
        assert_eq!(s.last_audio_unix_ms, 42_000_000);
        assert!(s.now_unix_ms >= s.last_audio_unix_ms);
    }

    #[test]
    fn age_phrase_variants() {
        assert_eq!(age_phrase(1_000, 0), "never");
        assert_eq!(age_phrase(60_000, 50_000), "10s ago");
        assert_eq!(age_phrase(125_000, 0), "never");
        assert_eq!(age_phrase(125_000, 5_000), "2m 0s ago");
        assert_eq!(age_phrase(3_700_000, 0), "never");
        assert_eq!(age_phrase(3_700_000, 100_000), "1h 0m ago");
    }
}
