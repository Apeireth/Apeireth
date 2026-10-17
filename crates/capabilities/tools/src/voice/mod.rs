//! 通用嗓 TTS 面 (`tool.voice_speak`) —— 让它开口说话 (V3-b 通用嗓闭环)。
//!
//! 定制音色 = 资产活后置; 本面只做**系统已装嗓**的通用发声闭环 (哲学卡
//! 01-companion-family「原声语音+工具链」: 声音即身份 + 工具可调用 ——
//! 感官与能力并进, 输入侧已在, 本面补输出侧)。
//!
//! # 管线图 (文本 → 发声 → 账本)
//!
//! ```text
//! tool.voice_speak (AI 先用: 它能让自己说话)
//!   │
//!   ├─ 五段流水线 (exec_pipeline)
//!   │    ├─ pre: VoiceSpeakRiskMappingHook ── 授权面 (发声面档, 与 shell
//!   │    │        执行档分立) / 坏参数拒绝即帧 / 勿扰窗口拒绝即帧
//!   │    ├─ guard: VoiceSpeakBoundaryGuard ── 单调边界 (只拒不放)
//!   │    ├─ around: 超时 (可取消 → 票据 Drop 真停)
//!   │    └─ post: 输出归一合同 (status/voice/duration_ms/text_chars/engine/ledger)
//!   │
//!   ├─ 参数校验 (拒绝即帧, 不钳制): 文本非空限长 / 语速 [-10,10] /
//!   │    音量 [0,100] / 嗓名 ∈ 系统已装嗓清单 (无嗓/未知嗓错误即帧)
//!   │
//!   ├─ 安静纪律 (quiet): 勿扰窗口内错误即帧 (同源接缝: care_rules 打扰
//!   │    纪律配了窗口 = 引用之, 否则 APEIRETH_VOICE_QUIET_HOURS 最小
//!   │    自建 env, 默认 22:00-06:00; 时间基准与关怀纪律同为 UTC 当日分钟)
//!   │
//!   ├─ 合成即播 (synth): 系统语音引擎受控子进程
//!   │    Windows = 系统托管语音封装桥 (脚本零插值, 请求走 stdin JSON);
//!   │    其余平台 = 缺席面 (错误即帧, 不假嗓)。执行面是 trait, 测试假引擎注入。
//!   │
//!   ├─ 受控播放 (playback): 独占槽 (同时只播一条) + 新话打断旧话可配
//!   │    (APEIRETH_VOICE_INTERRUPT: 1 打断 / 0 排队串行);
//!   │    SpeechTicket Drop = 真停 + 出槽 (取消/超时/收尾三路零僵尸播放)
//!   │
//!   └─ 语句账本 (ledger): <data_dir>/spoken.jsonl append-only
//!        说了什么 / 何时 / 哪把嗓 / 时长 (合成失败不入账 = 没说话;
//!        被打断 = 说了半句如实入账 interrupted)
//! ```
//!
//! # 治理与脱敏 (命名即承诺)
//!
//! - **开关**: `APEIRETH_ENABLE_VOICE_SPEAK` (注册即闸, 默认关 fail-closed,
//!   组装根接线见 runtime-assembly / cli);
//! - **发声面档**: grant 即发声 (免逐次人批), 拦截面 = 参数边界 + 勿扰窗口 +
//!   独占播放 + 账本全责; 与 shell 的「每次人批」执行档**不借壳不分档混用**;
//! - **脱敏**: 合成文本全文只进语句账本; 错误帧/输出帧只带字符数、嗓名、
//!   时长、状态 (文本原文不上任何日志面)。

pub mod ledger;
pub mod mock;
pub mod playback;
pub mod quiet;
pub mod speak;
pub mod synth;

pub use ledger::{
    spoken_utterance_id, voice_excerpt, SpokenLedger, SpokenStatus, SpokenUtterance,
    DEFAULT_VOICE_LABEL, SPOKEN_DOC_NAME, SPOKEN_DOC_VERSION, SPOKEN_LEDGER_FILE,
    VOICE_EXCERPT_MAX_CHARS,
};
pub use playback::{
    begin_speech, wait_playback, PlaybackController, PlaybackPolicy, SpeechTicket, TryBeginError,
    PLAYBACK_POLL_MS, VOICE_INTERRUPT_ENV,
};
pub use quiet::{
    fixed_clock, system_quiet_clock, utc_minutes_of_day, QuietClock, QuietHours, QuietWindow,
    MINUTES_PER_DAY, VOICE_QUIET_HOURS_DEFAULT, VOICE_QUIET_HOURS_ENV, VOICE_QUIET_HOURS_OFF,
};
pub use speak::{
    authorized_voice_speak_policy, resolve_voice, speak_request_refusal, voice_speak_capability,
    voice_speak_pipeline, SpeakOutcome, VoiceSpeakBoundaryGuard, VoiceSpeakConfig,
    VoiceSpeakParams, VoiceSpeakRiskMappingHook, VoiceSpeakTool, VOICE_RATE_DEFAULT,
    VOICE_RATE_MAX, VOICE_RATE_MIN, VOICE_SPEAK_CAPABILITY_ID, VOICE_SPEAK_MAX_TEXT_CHARS,
    VOICE_SPEAK_MAX_TIMEOUT_MS, VOICE_SPEAK_TIMEOUT_MS, VOICE_SPEAK_TOOL_DESCRIPTION,
    VOICE_SPEAK_TOOL_NAME, VOICE_VOLUME_DEFAULT, VOICE_VOLUME_MAX, VOICE_VOLUME_MIN,
};
pub use synth::{
    build_probe_script, build_speak_script, build_voices_script, parse_synth_payload, parse_voices,
    synth_request_line, system_synth_engine, AbsentSynthEngine, EngineStatus, PlaybackEnd,
    SpeechPlayback, SynthEngine, SynthPayload, SynthRequest, ENGINE_ABSENT, ENGINE_SYSTEM_SAPI,
    SYNTH_OUTPUT_LIMIT, SYNTH_PROBE_ANCHOR, SYNTH_PROBE_TIMEOUT_MS,
};
#[cfg(windows)]
pub use synth::{SapiPlayback, SapiSpeechEngine};
