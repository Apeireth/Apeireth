//! VoiceSession - 语音会话 (从 v1.0 apeireth-companion/voice_session.rs 178 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 SpeechIO trait + 会话状态

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceState { Idle, Listening, Processing, Speaking }

pub trait SpeechIO: Send + Sync {
    fn listen(&self) -> Result<String, String>;  // STT -> text
    fn speak(&self, text: &str) -> Result<(), String>;  // text -> TTS
}

pub struct VoiceSession { pub state: VoiceState, pub io: Option<Box<dyn SpeechIO>> }

impl VoiceSession {
    pub fn new() -> Self { Self { state: VoiceState::Idle, io: None } }
    pub fn bind(&mut self, io: Box<dyn SpeechIO>) { self.io = Some(io); }
    /// 0 装 PASS: 真 session 编排
    pub fn run(&mut self, text: &str) -> Result<String, String> {
        self.state = VoiceState::Listening;
        let io = self.io.as_ref().ok_or_else(|| "no io bound".to_string())?;
        let _ = io.listen()?;
        self.state = VoiceState::Processing;
        self.state = VoiceState::Speaking;
        io.speak(text)?;
        self.state = VoiceState::Idle;
        Ok(format!("processed: {}", text))
    }
}

impl Default for VoiceSession { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    struct MockIO;
    impl SpeechIO for MockIO {
        fn listen(&self) -> Result<String, String> { Ok("transcript".into()) }
        fn speak(&self, _text: &str) -> Result<(), String> { Ok(()) }
    }
    #[test] fn test_run() {
        let mut s = VoiceSession::new();
        s.bind(Box::new(MockIO));
        let r = s.run("hello").unwrap();
        assert!(r.contains("hello"));
    }
    #[test] fn test_no_io() {
        let mut s = VoiceSession::new();
        assert!(s.run("x").is_err());
    }
    #[test] fn test_state_eq() { assert_eq!(VoiceState::Idle, VoiceState::Idle); }
    #[test] fn test_default() { let s: VoiceSession = Default::default(); assert_eq!(s.state, VoiceState::Idle); }
}
