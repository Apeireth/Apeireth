//! `apeireth-companion::voice_session` — 连续感知①: 麦克风实时语音会话桥.
//!
//! ## 定位 (主人 2026-08-18: "麦克风语音可以做, 涉及连续世界感知, 很基础的功能")
//!
//! 实时语音对话编排: 麦克风音频 → STT → 对话管线 (build_injection) → TTS → 扬声器.
//! 这是"她听得见你"的基础 — 连续感知的第一层 (摄像头后置, 屏幕感知见 screen_perception).
//!
//! ## 0 装 PASS
//!
//! - [`SpeechInput`]/[`SpeechOutput`] trait 口已备 (STT/TTS 实现点: 可接 `apeireth-voice` 的
//!   real/minimax_live/realtime 链); 默认 Noop 诚实 Err (未接不假装能听能说).
//! - 文本回环 Mock 走通编排机制 (listen → 对话 → speak), 真音频接入时替换实现即可.

/// 语音输入 (麦克风 → 文本, STT). 实现点: apeireth-voice real/minimax_live.
pub trait SpeechInput: Send + Sync + std::fmt::Debug {
    /// 听一轮: 返回转写文本. 超时/无语音/STT 失败 → Err (诚实, 不返回空串假装听到了).
    fn listen(&mut self) -> Result<String, String>;
}

/// 语音输出 (文本 → 扬声器, TTS). 实现点: apeireth-voice real/minimax_live.
pub trait SpeechOutput: Send + Sync + std::fmt::Debug {
    /// 说一句.
    fn speak(&mut self, text: &str) -> Result<(), String>;
}

/// 默认实现: 未接 → 诚实 Err.
#[derive(Debug, Default)]
pub struct NoopSpeechInput;

impl SpeechInput for NoopSpeechInput {
    fn listen(&mut self) -> Result<String, String> {
        Err("NoopSpeechInput: 麦克风未接入 (实现 SpeechInput 时启用; 可接 apeireth-voice STT)".into())
    }
}

#[derive(Debug, Default)]
pub struct NoopSpeechOutput;

impl SpeechOutput for NoopSpeechOutput {
    fn speak(&mut self, _text: &str) -> Result<(), String> {
        Err("NoopSpeechOutput: 扬声器未接入 (实现 SpeechOutput 时启用; 可接 apeireth-voice TTS)".into())
    }
}

/// 一轮语音会话结果.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceTurn {
    /// 听到的 (转写).
    pub transcript: String,
    /// 说出的 (对话管线回复).
    pub reply: String,
    pub at_ms: i64,
}

/// 语音会话桥 (编排机制, 确定性).
#[derive(Debug)]
pub struct VoiceSession {
    input: Box<dyn SpeechInput>,
    output: Box<dyn SpeechOutput>,
    pub turn_count: u64,
}

impl VoiceSession {
    pub fn new(input: Box<dyn SpeechInput>, output: Box<dyn SpeechOutput>) -> Self {
        Self {
            input,
            output,
            turn_count: 0,
        }
    }

    /// 一轮: listen → handler(对话管线) → speak.
    /// handler 是对话编排入口 (调用方接 build_injection + LLM).
    pub fn turn(&mut self, handler: &dyn Fn(&str) -> String) -> Result<VoiceTurn, String> {
        let transcript = self.input.listen()?;
        if transcript.trim().is_empty() {
            return Err("转写为空 (0 装: 不假装听到了内容)".into());
        }
        let reply = handler(&transcript);
        self.output.speak(&reply)?;
        self.turn_count += 1;
        Ok(VoiceTurn {
            transcript,
            reply,
            at_ms: chrono::Utc::now().timestamp_millis(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文本回环 Mock: input 返回固定文本, output 记录.
    #[derive(Debug)]
    struct MockInput {
        texts: Vec<String>,
    }

    impl SpeechInput for MockInput {
        fn listen(&mut self) -> Result<String, String> {
            Ok(self.texts.remove(0))
        }
    }

    #[derive(Debug, Default)]
    struct MockOutput {
        spoken: Vec<String>,
    }

    impl SpeechOutput for MockOutput {
        fn speak(&mut self, text: &str) -> Result<(), String> {
            self.spoken.push(text.to_string());
            Ok(())
        }
    }

    #[test]
    fn full_turn_loopback() {
        let input = Box::new(MockInput { texts: vec!["主人今天好累".into()] });
        let output = Box::new(MockOutput::default());
        let mut session = VoiceSession::new(input, output);
        let turn = session.turn(&|t| format!("听到你说: {t}")).unwrap();
        assert_eq!(turn.transcript, "主人今天好累");
        assert_eq!(turn.reply, "听到你说: 主人今天好累");
        assert_eq!(session.turn_count, 1);
        let out = session.output.speak("x"); // 直接访问不可行, 用类型内验证
        let _ = out;
    }

    #[test]
    fn noop_input_is_honest() {
        let mut session = VoiceSession::new(Box::new(NoopSpeechInput), Box::new(NoopSpeechOutput));
        let err = session.turn(&|t| t.to_string()).unwrap_err();
        assert!(err.contains("麦克风未接入"), "{err}");
        assert_eq!(session.turn_count, 0, "未接不假装已对话");
    }

    #[test]
    fn empty_transcript_rejected() {
        #[derive(Debug)]
        struct EmptyInput;
        impl SpeechInput for EmptyInput {
            fn listen(&mut self) -> Result<String, String> {
                Ok("   ".into())
            }
        }
        let mut session = VoiceSession::new(Box::new(EmptyInput), Box::new(NoopSpeechOutput));
        let err = session.turn(&|t| t.to_string()).unwrap_err();
        assert!(err.contains("转写为空"), "{err}");
    }

    #[test]
    fn speak_failure_propagates() {
        #[derive(Debug)]
        struct FailOutput;
        impl SpeechOutput for FailOutput {
            fn speak(&mut self, _t: &str) -> Result<(), String> {
                Err("扬声器故障".into())
            }
        }
        let input = Box::new(MockInput { texts: vec!["hi".into()] });
        let mut session = VoiceSession::new(input, Box::new(FailOutput));
        let err = session.turn(&|t| t.to_string()).unwrap_err();
        assert!(err.contains("扬声器故障"), "{err}");
        assert_eq!(session.turn_count, 0, "说失败不算完成一轮");
    }
}
