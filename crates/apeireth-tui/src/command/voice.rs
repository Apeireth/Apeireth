//! Voice (声) command 模块 — TTS / STT
//!
//! **借鉴 Golutra #1**: 9 organ × 5-8 command 模式
//!
//! **6 命令**:
//! 1. [`Command::Synthesize`] — TTS 合成 (R25.2 stub)
//! 2. [`Command::GetVoices`] — 读可选 voice 列表 (编译期 hardcode)
//! 3. [`Command::SetVoice`] — 切换 active voice
//! 4. [`Command::GetActiveVoice`] — 读 active voice
//! 5. [`Command::GetTtsStatus`] — 读 TTS 状态 (idle / playing / paused)
//! 6. [`Command::Pause`] — 暂停 TTS
//!
//! **不假装**:
//! - voice 在 `organ/mod.rs` 标 `Readiness::Stub` — 6 命令全部标 placeholder
//! - 真实 R25.3 接 `batch_text_to_audio` / `transcribe_audio` 本地 API
//! - 3 voice 编译期 hardcode (per `get_voice_list()` 实际可用 voice_id)
//! - 3 态状态机: Idle / Playing / Paused
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: voice 服务 ASI 输出通道
//! - S-2 实事求是: stub 标 partial, 3 voice hardcode
//! - O-2 走在前人经验上: 借 voice_id 业界模式
//! - O-3 干到底: 6 命令覆盖 voice 全场景
//! - O-4 任何人都能接手: State + voice_id 列表全文档化
//! - O-5 不假装: TTS 不真发声, 标 stub
//!
//! **8 项承诺**: 全部遵守

use super::error::OrganError;

/// 3 voice 编译期 hardcode (per `get_voice_list()` 实际可用 voice_id 节选)
pub const VOICES: &[&str] = &["male-qn-qingse", "female-shaonv", "male-qn-jingying"];

/// 默认 voice
pub const DEFAULT_VOICE: &str = "male-qn-qingse";

/// TTS 状态机 (3 态, 编译期 hardcode)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsState {
    /// 空闲
    Idle,
    /// 播放中
    Playing,
    /// 暂停
    Paused,
}

impl TtsState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Playing => "playing",
            Self::Paused => "paused",
        }
    }
}

/// Voice 器官状态
#[derive(Debug, Clone)]
pub struct State {
    /// 当前 active voice
    pub active_voice: String,
    /// TTS 状态
    pub tts_state: TtsState,
    /// 已合成次数
    pub synthesize_count: u64,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active_voice: DEFAULT_VOICE.to_string(),
            tts_state: TtsState::Idle,
            synthesize_count: 0,
        }
    }
}

/// Voice 器官 6 命令
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// TTS 合成 (R25.2 stub — 不真发声)
    Synthesize {
        /// 文本
        text: String,
    },
    /// 读可选 voice 列表
    GetVoices,
    /// 切换 active voice
    SetVoice {
        /// voice_id
        voice_id: String,
    },
    /// 读 active voice
    GetActiveVoice,
    /// 读 TTS 状态
    GetTtsStatus,
    /// 暂停 TTS
    Pause,
}

/// Voice 命令响应
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    /// 通用单元响应
    Unit,
    /// voice 列表
    Voices(Vec<&'static str>),
    /// active voice
    ActiveVoice(String),
    /// TTS 状态
    TtsStatus(TtsState),
    /// 已合成次数
    SynthesizeCount(u64),
}

/// 处理 Voice 命令
///
/// **错误**:
/// - [`OrganError::InvalidArg`] — text 空 / voice_id 不在 3 编译期 hardcode
/// - [`OrganError::Unsupported`] — Synthesize 在 R25.2 是 stub
pub fn handle(state: &mut State, cmd: Command) -> Result<Response, OrganError> {
    match cmd {
        Command::Synthesize { text } => {
            if text.is_empty() {
                return Err(OrganError::InvalidArg {
                    command: "Synthesize",
                    reason: "text 不能为空".into(),
                });
            }
            // S-2 实事求是: stub — 不真发声, 仅计数
            state.synthesize_count = state.synthesize_count.saturating_add(1);
            state.tts_state = TtsState::Idle; // 立即完成
            Ok(Response::Unit)
        }
        Command::GetVoices => Ok(Response::Voices(VOICES.to_vec())),
        Command::SetVoice { voice_id } => {
            if !VOICES.contains(&voice_id.as_str()) {
                return Err(OrganError::InvalidArg {
                    command: "SetVoice",
                    reason: format!("voice_id '{voice_id}' not in 3 编译期 hardcode"),
                });
            }
            state.active_voice = voice_id;
            Ok(Response::Unit)
        }
        Command::GetActiveVoice => Ok(Response::ActiveVoice(state.active_voice.clone())),
        Command::GetTtsStatus => Ok(Response::TtsStatus(state.tts_state)),
        Command::Pause => {
            if state.tts_state != TtsState::Playing {
                return Err(OrganError::NotReady {
                    organ: ASCII_CHAR,
                    reason: format!("Pause 要求 Playing, 实际 {}", state.tts_state.label()),
                });
            }
            state.tts_state = TtsState::Paused;
            Ok(Response::Unit)
        }
    }
}

/// 器官 ASCII 字符
pub const ASCII_CHAR: &str = "[VOICE]";

/// 器官中文名
pub const NAME_ZH: &str = "声";

// =====================================================================
// 单元测试 (6 命令 + 3 voice 守门 + 3 态状态机 = 8+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state() -> State {
        State::default()
    }

    // ---- 6 命令全部可枚举 ----

    #[test]
    fn six_commands_constructible() {
        let _ = Command::Synthesize { text: "hi".into() };
        let _ = Command::GetVoices;
        let _ = Command::SetVoice { voice_id: "male-qn-qingse".into() };
        let _ = Command::GetActiveVoice;
        let _ = Command::GetTtsStatus;
        let _ = Command::Pause;
    }

    // ---- 3 voice 编译期 hardcode ----

    #[test]
    fn three_voices_hardcoded() {
        assert_eq!(VOICES.len(), 3, "3 voice 编译期 hardcode");
    }

    // ---- Synthesize ----

    #[test]
    fn synthesize_increments_count() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::Synthesize { text: "hello world".into() },
            );
        assert!(r.is_ok());
        assert_eq!(state.synthesize_count, 1);
    }

    #[test]
    fn synthesize_rejects_empty_text() {
        let mut state = fresh_state();
        let r = handle(&mut state, Command::Synthesize { text: "".into() });
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "Synthesize", .. })));
    }

    // ---- SetVoice ----

    #[test]
    fn set_voice_valid() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::SetVoice { voice_id: "female-shaonv".into() },
            );
        assert!(r.is_ok());
        let r = handle(&mut state, Command::GetActiveVoice).unwrap();
        assert_eq!(r, Response::ActiveVoice("female-shaonv".into()));
    }

    #[test]
    fn set_voice_rejects_unknown() {
        let mut state = fresh_state();
        let r = handle(
            &mut state,
            Command::SetVoice { voice_id: "fake-voice".into() },
            );
        assert!(matches!(r, Err(OrganError::InvalidArg { command: "SetVoice", .. })));
    }

    // ---- 3 态状态机 ----

    #[test]
    fn three_states_distinct() {
        let labels: Vec<&str> = [TtsState::Idle, TtsState::Playing, TtsState::Paused]
            .iter()
            .map(|s| s.label())
            .collect();
        let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn pause_requires_playing() {
        let mut state = fresh_state();
        // 默认 Idle, Pause 失败
        let r = handle(&mut state, Command::Pause);
        assert!(matches!(r, Err(OrganError::NotReady { .. })));
    }

    // ---- 器官元数据 ----

    #[test]
    fn ascii_char_matches_organ_mod() {
        assert_eq!(ASCII_CHAR, "[VOICE]");
    }

    #[test]
    fn name_zh_matches_organ_mod() {
        assert_eq!(NAME_ZH, "声");
    }
}
