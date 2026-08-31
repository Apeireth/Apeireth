//! Audio session state machines recovered from canonical voice + companion.
//!
//! Two independent helpers, both **library-only / default-off**:
//!
//! 1. [`RecordingSession`] — canonical `RecordingStatus` (Pending / Recording /
//!    Stopped / Failed) as a *real* guarded SM. Engine `VoiceRecorder::start`
//!    returned `NotImplemented` under `STUB_MODE` and never transitioned.
//! 2. [`VoiceSession`] — companion `listen → handler → speak` orchestration.
//!    Agent 08 deferred this. The handler is an injected `Fn(&str) -> String`;
//!    this type does **not** own a transcript, a session, or final response.
//!    Callers that want a spoken reply still go through the canonical turn
//!    loop and pass the resulting text into `speak`.
//!
//! No Whisper / MiniMax / LiveKit wiring lives here.

use std::fmt;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::audio_frame::{
    duration_ms, Pcm16Buffer, PCM16_CHANNELS_MONO, PCM16_MAX_AUDIO_SECONDS, PCM16_SAMPLE_RATE_HZ,
};

/// Recording SM errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingError {
    /// Transition is illegal from the current status.
    IllegalTransition {
        from: RecordingStatus,
        attempted: &'static str,
    },
    /// Audio empty / too long / format (from frame helpers).
    Audio(String),
}

impl fmt::Display for RecordingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IllegalTransition { from, attempted } => {
                write!(
                    f,
                    "illegal recording transition: {from:?} cannot {attempted}"
                )
            }
            Self::Audio(msg) => write!(f, "recording audio error: {msg}"),
        }
    }
}

impl std::error::Error for RecordingError {}

/// Engine 4-state recording machine (`Pending` / `Recording` / `Stopped` / `Failed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingStatus {
    /// Wake / arm — not yet writing samples.
    Pending,
    /// Actively capturing into the buffer.
    Recording,
    /// User stop or silence timeout. Terminal until [`RecordingSession::reset`].
    Stopped,
    /// Capture failure. Terminal until reset.
    Failed,
}

/// One capture session. Does not own a microphone or a transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordingSession {
    /// Opaque session id (caller-assigned; canonical used UUID).
    pub id: String,
    /// Wake word / trigger that armed the session.
    pub triggered_by: String,
    /// Arm time.
    pub started_at: SystemTime,
    /// Max capture duration (canonical default 30 s).
    pub max_duration: Duration,
    /// Current status.
    pub status: RecordingStatus,
    samples: Vec<i16>,
}

impl RecordingSession {
    /// Arm a new session in [`RecordingStatus::Pending`].
    pub fn arm(id: impl Into<String>, triggered_by: impl Into<String>) -> Self {
        Self::arm_with_max(
            id,
            triggered_by,
            Duration::from_secs(u64::from(PCM16_MAX_AUDIO_SECONDS)),
        )
    }

    /// Arm with an explicit duration cap.
    pub fn arm_with_max(
        id: impl Into<String>,
        triggered_by: impl Into<String>,
        max_duration: Duration,
    ) -> Self {
        Self {
            id: id.into(),
            triggered_by: triggered_by.into(),
            started_at: SystemTime::now(),
            max_duration,
            status: RecordingStatus::Pending,
            samples: Vec::new(),
        }
    }

    /// `Pending → Recording`.
    pub fn start(&mut self) -> Result<(), RecordingError> {
        match self.status {
            RecordingStatus::Pending => {
                self.status = RecordingStatus::Recording;
                Ok(())
            }
            other => Err(RecordingError::IllegalTransition {
                from: other,
                attempted: "start",
            }),
        }
    }

    /// Append PCM16 samples while [`RecordingStatus::Recording`].
    ///
    /// Crossing [`Self::max_duration`] fails the session (`Pending`/`Recording`
    /// → `Failed`) and returns [`RecordingError::Audio`].
    pub fn append_samples(&mut self, samples: &[i16]) -> Result<(), RecordingError> {
        if self.status != RecordingStatus::Recording {
            return Err(RecordingError::IllegalTransition {
                from: self.status,
                attempted: "append",
            });
        }
        if samples.is_empty() {
            return Ok(());
        }
        self.samples.extend_from_slice(samples);
        let elapsed = duration_ms(
            self.samples.len(),
            PCM16_SAMPLE_RATE_HZ,
            PCM16_CHANNELS_MONO,
        );
        if elapsed > self.max_duration.as_millis() as u64 {
            self.status = RecordingStatus::Failed;
            return Err(RecordingError::Audio(format!(
                "audio too long: got {elapsed}ms, max {}ms",
                self.max_duration.as_millis()
            )));
        }
        Ok(())
    }

    /// `Recording → Stopped`. Buffer must be non-empty.
    pub fn stop(&mut self) -> Result<Pcm16Buffer, RecordingError> {
        if self.status != RecordingStatus::Recording {
            return Err(RecordingError::IllegalTransition {
                from: self.status,
                attempted: "stop",
            });
        }
        if self.samples.is_empty() {
            self.status = RecordingStatus::Failed;
            return Err(RecordingError::Audio("audio buffer is empty".into()));
        }
        self.status = RecordingStatus::Stopped;
        Pcm16Buffer::from_samples(self.samples.clone())
            .map_err(|err| RecordingError::Audio(err.to_string()))
    }

    /// Force `Failed` from `Pending` or `Recording`.
    pub fn fail(&mut self, reason: impl Into<String>) -> RecordingError {
        match self.status {
            RecordingStatus::Pending | RecordingStatus::Recording => {
                self.status = RecordingStatus::Failed;
                RecordingError::Audio(reason.into())
            }
            other => RecordingError::IllegalTransition {
                from: other,
                attempted: "fail",
            },
        }
    }

    /// Re-arm from a terminal state (`Stopped` / `Failed`) back to `Pending`.
    pub fn reset(&mut self) -> Result<(), RecordingError> {
        match self.status {
            RecordingStatus::Stopped | RecordingStatus::Failed => {
                self.samples.clear();
                self.started_at = SystemTime::now();
                self.status = RecordingStatus::Pending;
                Ok(())
            }
            other => Err(RecordingError::IllegalTransition {
                from: other,
                attempted: "reset",
            }),
        }
    }

    /// Samples captured so far (empty unless recording / stopped).
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Whether the session is still writable.
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            RecordingStatus::Pending | RecordingStatus::Recording
        )
    }
}

// ---------------------------------------------------------------------------
// Companion VoiceSession (listen → handler → speak). NOT a second main loop.
// ---------------------------------------------------------------------------

/// Speech input (microphone → text). Implementations live at the adapter.
pub trait SpeechInput: Send + Sync + std::fmt::Debug {
    /// Listen one turn. Timeout / no speech / STT failure → `Err` (honest; never
    /// an empty string pretending to have heard something).
    fn listen(&mut self) -> Result<String, String>;
}

/// Speech output (text → speaker). Implementations live at the adapter.
pub trait SpeechOutput: Send + Sync + std::fmt::Debug {
    /// Speak one utterance.
    fn speak(&mut self, text: &str) -> Result<(), String>;
}

/// Default input: not wired. Honest error.
#[derive(Debug, Default)]
pub struct NoopSpeechInput;

impl SpeechInput for NoopSpeechInput {
    fn listen(&mut self) -> Result<String, String> {
        Err("NoopSpeechInput: microphone not wired".into())
    }
}

/// Default output: not wired. Honest error.
#[derive(Debug, Default)]
pub struct NoopSpeechOutput;

impl SpeechOutput for NoopSpeechOutput {
    fn speak(&mut self, _text: &str) -> Result<(), String> {
        Err("NoopSpeechOutput: speaker not wired".into())
    }
}

/// One completed listen → speak turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceTurn {
    /// STT transcript.
    pub transcript: String,
    /// Handler reply that was spoken.
    pub reply: String,
    /// Completed-turn count after this turn (1-based).
    pub turn_index: u64,
}

/// Orchestration helper. Does not own a session, transcript, or LLM path.
///
/// `handler` is injected per [`VoiceSession::turn`]. Production callers should
/// pass text that already came from the canonical turn loop — never a raw
/// provider completion.
#[derive(Debug)]
pub struct VoiceSession {
    input: Box<dyn SpeechInput>,
    output: Box<dyn SpeechOutput>,
    /// Completed turns (incremented only after a successful speak).
    pub turn_count: u64,
}

impl VoiceSession {
    /// Construct with explicit IO. Default-off: pass [`NoopSpeechInput`] /
    /// [`NoopSpeechOutput`] and the first `turn` fails honestly.
    pub fn new(input: Box<dyn SpeechInput>, output: Box<dyn SpeechOutput>) -> Self {
        Self {
            input,
            output,
            turn_count: 0,
        }
    }

    /// Disabled constructor (noop IO).
    pub fn disabled() -> Self {
        Self::new(Box::new(NoopSpeechInput), Box::new(NoopSpeechOutput))
    }

    /// One turn: listen → reject empty → `handler` → speak → increment.
    ///
    /// Speak failure does **not** increment `turn_count` (canonical contract).
    pub fn turn(&mut self, handler: &dyn Fn(&str) -> String) -> Result<VoiceTurn, String> {
        let transcript = self.input.listen()?;
        if transcript.trim().is_empty() {
            return Err("empty transcript (will not pretend speech was heard)".into());
        }
        let reply = handler(&transcript);
        self.output.speak(&reply)?;
        self.turn_count += 1;
        Ok(VoiceTurn {
            transcript,
            reply,
            turn_index: self.turn_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_starts_pending() {
        let session = RecordingSession::arm("rec-1", "apeireth");
        assert_eq!(session.status, RecordingStatus::Pending);
        assert_eq!(session.triggered_by, "apeireth");
        assert!(session.is_active());
    }

    #[test]
    fn pending_to_recording_to_stopped() {
        let mut session = RecordingSession::arm("rec-1", "apeireth");
        session.start().unwrap();
        assert_eq!(session.status, RecordingStatus::Recording);
        session.append_samples(&[1i16; 512]).unwrap();
        let buf = session.stop().unwrap();
        assert_eq!(session.status, RecordingStatus::Stopped);
        assert_eq!(buf.samples.len(), 512);
        assert!(!session.is_active());
    }

    #[test]
    fn cannot_append_while_pending() {
        let mut session = RecordingSession::arm("rec-1", "apeireth");
        let err = session.append_samples(&[1]).unwrap_err();
        assert!(matches!(
            err,
            RecordingError::IllegalTransition {
                from: RecordingStatus::Pending,
                attempted: "append"
            }
        ));
    }

    #[test]
    fn cannot_start_twice() {
        let mut session = RecordingSession::arm("rec-1", "apeireth");
        session.start().unwrap();
        assert!(matches!(
            session.start(),
            Err(RecordingError::IllegalTransition {
                from: RecordingStatus::Recording,
                ..
            })
        ));
    }

    #[test]
    fn stop_on_empty_fails_session() {
        let mut session = RecordingSession::arm("rec-1", "apeireth");
        session.start().unwrap();
        let err = session.stop().unwrap_err();
        assert!(matches!(err, RecordingError::Audio(_)));
        assert_eq!(session.status, RecordingStatus::Failed);
    }

    #[test]
    fn append_over_cap_fails_session() {
        let mut session =
            RecordingSession::arm_with_max("rec-1", "apeireth", Duration::from_millis(10));
        session.start().unwrap();
        // 512 samples @ 16 kHz ≈ 32 ms > 10 ms
        let err = session.append_samples(&[0i16; 512]).unwrap_err();
        assert!(matches!(err, RecordingError::Audio(_)));
        assert_eq!(session.status, RecordingStatus::Failed);
    }

    #[test]
    fn fail_from_recording() {
        let mut session = RecordingSession::arm("rec-1", "apeireth");
        session.start().unwrap();
        let err = session.fail("device lost");
        assert!(matches!(err, RecordingError::Audio(ref msg) if msg == "device lost"));
        assert_eq!(session.status, RecordingStatus::Failed);
    }

    #[test]
    fn reset_from_stopped_rearms() {
        let mut session = RecordingSession::arm("rec-1", "apeireth");
        session.start().unwrap();
        session.append_samples(&[1i16; 16]).unwrap();
        session.stop().unwrap();
        session.reset().unwrap();
        assert_eq!(session.status, RecordingStatus::Pending);
        assert!(session.samples().is_empty());
    }

    #[test]
    fn reset_rejected_while_recording() {
        let mut session = RecordingSession::arm("rec-1", "apeireth");
        session.start().unwrap();
        assert!(matches!(
            session.reset(),
            Err(RecordingError::IllegalTransition {
                from: RecordingStatus::Recording,
                ..
            })
        ));
    }

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
        let mut session = VoiceSession::new(
            Box::new(MockInput {
                texts: vec!["hello world".into()],
            }),
            Box::new(MockOutput::default()),
        );
        let turn = session.turn(&|t| format!("heard: {t}")).unwrap();
        assert_eq!(turn.transcript, "hello world");
        assert_eq!(turn.reply, "heard: hello world");
        assert_eq!(turn.turn_index, 1);
        assert_eq!(session.turn_count, 1);
    }

    #[test]
    fn noop_input_is_honest() {
        let mut session = VoiceSession::disabled();
        let err = session.turn(&|t| t.to_string()).unwrap_err();
        assert!(err.contains("microphone not wired"), "{err}");
        assert_eq!(session.turn_count, 0);
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
        assert!(err.contains("empty transcript"), "{err}");
        assert_eq!(session.turn_count, 0);
    }

    #[test]
    fn speak_failure_does_not_count_turn() {
        #[derive(Debug)]
        struct FailOutput;
        impl SpeechOutput for FailOutput {
            fn speak(&mut self, _t: &str) -> Result<(), String> {
                Err("speaker fault".into())
            }
        }
        let mut session = VoiceSession::new(
            Box::new(MockInput {
                texts: vec!["hi".into()],
            }),
            Box::new(FailOutput),
        );
        let err = session.turn(&|t| t.to_string()).unwrap_err();
        assert!(err.contains("speaker fault"), "{err}");
        assert_eq!(session.turn_count, 0);
    }
}
