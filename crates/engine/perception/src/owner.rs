//! Default-off perception owner: multimodal ingest, capture metadata, screen
//! salience, and observation-capture — without AgentModule or final-response
//! ownership.
//!
//! Frozen v2 architecture: perception is an **input adapter**. Runtime already
//! converts text events through `turn_request_from_perception`. This type is
//! the crate-level owner of recovered algorithms so production wiring can
//! later inject it. Construction is enabled only via [`PerceptionOwner::enabled`];
//! [`PerceptionOwner::disabled`] (and [`Default`]) is a no-op sink.

use apeireth_core::kernel::SessionId;
use apeireth_plugin::perception::PerceptionEvent;
use apeireth_plugin::perception_backend::ScreenshotBytes;
use serde_json::Value;

use crate::capture::{capture_attention_score, capture_metadata};
use crate::normalize::{
    command_observation, now_timestamp_ms, pipeline_events, tactile_observation, text_observation,
    top_k_events, validate_event, vision_observation, voice_observation, with_tag, SignalSource,
    DEFAULT_ATTENTION_THRESHOLD, DEFAULT_TOP_K,
};
use crate::observe::{
    args_hash, ObservationCandidate, ObservationOutcome, ObservationQueue, ObservationSource,
};
use crate::screen::{NoopScreenSource, ScreenEvent, ScreenPerception};

/// Bounded pending-observation ring. Diagnosis-sized, never unbounded.
const MAX_PENDING: usize = 256;

/// Crate-level perception owner. Default-off: ingest is a no-op until
/// [`PerceptionOwner::enabled`] is used by a future integrator.
pub struct PerceptionOwner {
    enabled: bool,
    session_id: SessionId,
    pending: Vec<PerceptionEvent>,
    screen: ScreenPerception,
    observations: ObservationQueue,
    attention_threshold: f64,
    top_k: usize,
}

impl PerceptionOwner {
    /// Disabled owner. Algorithms are constructed but ingest / poll are no-ops.
    pub fn disabled(session_id: SessionId) -> Self {
        Self {
            enabled: false,
            session_id,
            pending: Vec::new(),
            screen: ScreenPerception::new(Box::new(NoopScreenSource)),
            observations: ObservationQueue::new(),
            attention_threshold: DEFAULT_ATTENTION_THRESHOLD,
            top_k: DEFAULT_TOP_K,
        }
    }

    /// Enabled owner with the honest no-op screen source. Still **not**
    /// production-wired: callers must construct this explicitly.
    pub fn enabled(session_id: SessionId) -> Self {
        let mut owner = Self::disabled(session_id);
        owner.enabled = true;
        owner
    }

    /// Whether ingest is live.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Replace the screen source (still a no-op unless the owner is enabled).
    pub fn with_screen_source(mut self, source: Box<dyn crate::screen::ScreenEventSource>) -> Self {
        self.screen = ScreenPerception::new(source);
        self
    }

    /// Override attention threshold (clamped by the pipeline).
    pub fn with_attention_threshold(mut self, threshold: f64) -> Self {
        self.attention_threshold = threshold;
        self
    }

    /// Override Top-K.
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }

    /// Session this owner tags onto events.
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    fn push_pending(&mut self, event: PerceptionEvent) {
        if validate_event(&event).is_err() {
            return;
        }
        if self.pending.len() == MAX_PENDING {
            self.pending.remove(0);
        }
        self.pending.push(event);
    }

    /// Ingest a text observation. No-op when disabled.
    pub fn ingest_text(&mut self, source: SignalSource, content: impl Into<String>, priority: f64) {
        if !self.enabled {
            return;
        }
        self.push_pending(text_observation(
            self.session_id,
            source,
            content,
            priority,
            now_timestamp_ms(),
        ));
    }

    /// Ingest a voice transcript + loudness observation.
    pub fn ingest_voice(
        &mut self,
        source: SignalSource,
        transcript: impl Into<String>,
        loudness: f64,
    ) {
        if !self.enabled {
            return;
        }
        self.push_pending(voice_observation(
            self.session_id,
            source,
            transcript,
            loudness,
            now_timestamp_ms(),
        ));
    }

    /// Ingest a vision / OCR observation.
    pub fn ingest_vision(
        &mut self,
        source: SignalSource,
        width: u32,
        height: u32,
        ocr_text: Option<String>,
    ) {
        if !self.enabled {
            return;
        }
        self.push_pending(vision_observation(
            self.session_id,
            source,
            width,
            height,
            ocr_text,
            now_timestamp_ms(),
        ));
    }

    /// Ingest a tactile / heartbeat observation.
    pub fn ingest_tactile(&mut self, source: SignalSource, pressure: f64) {
        if !self.enabled {
            return;
        }
        self.push_pending(tactile_observation(
            self.session_id,
            source,
            pressure,
            now_timestamp_ms(),
        ));
    }

    /// Ingest an explicit command observation.
    pub fn ingest_command(&mut self, source: SignalSource, command: impl Into<String>) {
        if !self.enabled {
            return;
        }
        self.push_pending(command_observation(
            self.session_id,
            source,
            command,
            now_timestamp_ms(),
        ));
    }

    /// Attach capture metadata from an existing screenshot and ingest a vision
    /// observation. Unknown dimensions are recorded honestly (score 0) rather
    /// than invented.
    pub fn ingest_screenshot(&mut self, screenshot: &ScreenshotBytes, ocr_text: Option<String>) {
        if !self.enabled {
            return;
        }
        let metadata = capture_metadata(screenshot);
        let width = metadata.width.unwrap_or(0);
        let height = metadata.height.unwrap_or(0);
        let mut event = vision_observation(
            self.session_id,
            metadata.source.clone(),
            width,
            height,
            ocr_text,
            metadata.captured_at_ms,
        );
        if let Some(score) = capture_attention_score(&metadata) {
            event.attention_score = score;
        } else {
            event.attention_score = 0.0;
            event = with_tag(event, "dims_unknown");
        }
        event.payload["format"] = serde_json::json!(metadata.format);
        event.payload["byte_len"] = serde_json::json!(metadata.byte_len);
        event = with_tag(event, "capture");
        self.push_pending(event);
    }

    /// Poll the screen source and ingest events that pass significance.
    pub fn poll_screen(&mut self) {
        if !self.enabled {
            return;
        }
        let events = self.screen.poll_events();
        self.ingest_screen_events(events);
    }

    /// Time-injected screen poll (tests).
    pub fn poll_screen_at(&mut self, now_ms: i64) {
        if !self.enabled {
            return;
        }
        let events = self.screen.poll_events_at(now_ms);
        self.ingest_screen_events(events);
    }

    fn ingest_screen_events(&mut self, events: Vec<ScreenEvent>) {
        for event in events {
            if !self.screen.should_perceive(&event) {
                continue;
            }
            self.push_pending(self.screen.to_observation(self.session_id, &event));
        }
    }

    /// Capture a tool-execution observation. Returns whether it was enqueued
    /// (`false` = disabled or 24h dedup). Does not mutate the tool result.
    pub fn capture_tool_observation(
        &self,
        tool: impl Into<String>,
        args: &Value,
        success: bool,
        output: Option<&str>,
        error: Option<&str>,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        let candidate = ObservationCandidate {
            tool: tool.into(),
            args_hash: args_hash(args),
            outcome: ObservationOutcome::from_result(success, output, error),
            ts_ms: now_timestamp_ms(),
            source: ObservationSource::ToolExecution,
        };
        self.observations.push(candidate)
    }

    /// Drain captured tool observations.
    pub fn drain_tool_observations(&self) -> Vec<ObservationCandidate> {
        self.observations.drain_pending()
    }

    /// Pending multimodal events (clone; does not drain).
    pub fn pending(&self) -> &[PerceptionEvent] {
        &self.pending
    }

    /// Drain pending events through threshold then Top-K. Disabled owners
    /// always return empty (they never ingested).
    pub fn select(&mut self) -> Vec<PerceptionEvent> {
        if !self.enabled {
            self.pending.clear();
            return Vec::new();
        }
        let drained = std::mem::take(&mut self.pending);
        let above = pipeline_events(drained, self.attention_threshold);
        top_k_events(above, self.top_k)
    }
}

impl Default for PerceptionOwner {
    fn default() -> Self {
        Self::disabled(SessionId::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize::DEFAULT_TEXT_PRIORITY;
    use crate::screen::{ScreenEventKind, ScreenEventSource};
    use apeireth_plugin::perception::PerceptionModality;
    use serde_json::json;

    #[derive(Debug)]
    struct MockSource {
        events: Vec<ScreenEvent>,
    }

    impl ScreenEventSource for MockSource {
        fn poll(&mut self) -> Vec<ScreenEvent> {
            std::mem::take(&mut self.events)
        }
    }

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn default_owner_is_disabled_and_swallows_ingest() {
        let mut owner = PerceptionOwner::default();
        assert!(!owner.is_enabled());
        owner.ingest_text(SignalSource::Cli, "hello", 1.0);
        owner.ingest_voice(SignalSource::Http, "hi", 0.9);
        owner.ingest_command(SignalSource::Cli, "/status");
        assert!(owner.pending().is_empty());
        assert!(owner.select().is_empty());
        assert!(!owner.capture_tool_observation("t", &json!({}), true, Some("ok"), None));
    }

    #[test]
    fn enabled_owner_normalizes_five_modalities_and_filters() {
        let mut owner = PerceptionOwner::enabled(SessionId::new()).with_top_k(3);
        owner.ingest_text(SignalSource::Cli, "hello world", 0.6);
        owner.ingest_text(SignalSource::Internal, "noise", 0.1);
        owner.ingest_voice(SignalSource::Http, "say hi", 0.85);
        owner.ingest_vision(SignalSource::PyBridge, 1280, 720, Some("screen".into()));
        owner.ingest_tactile(SignalSource::Internal, -0.9);
        owner.ingest_command(SignalSource::Cli, "/status");
        assert_eq!(owner.pending().len(), 6);
        let selected = owner.select();
        assert!(!selected.is_empty());
        assert!(selected.len() <= 3);
        assert!(selected
            .iter()
            .all(|event| event.attention_score >= DEFAULT_ATTENTION_THRESHOLD));
        for event in &selected {
            assert!(validate_event(event).is_ok());
            assert!(matches!(
                event.source,
                PerceptionModality::Text
                    | PerceptionModality::Voice
                    | PerceptionModality::Vision
                    | PerceptionModality::Tactile
                    | PerceptionModality::Command
            ));
        }
        assert!(owner.pending().is_empty());
    }

    #[test]
    fn ingest_screenshot_attaches_capture_metadata() {
        let mut owner = PerceptionOwner::enabled(SessionId::new());
        let screenshot = ScreenshotBytes {
            bytes: png_header(1920, 1080),
            format: "png".into(),
            captured_at_ms: 99,
        };
        owner.ingest_screenshot(&screenshot, Some("ocr".into()));
        let event = &owner.pending()[0];
        assert_eq!(event.payload["width"], 1920);
        assert_eq!(event.payload["ocr"], "ocr");
        assert_eq!(event.payload["format"], "png");
        assert!((event.attention_score - 1.0).abs() < 1e-6);
        assert!(event.tags.contains(&"capture".to_string()));
    }

    #[test]
    fn poll_screen_drops_window_switch_and_keeps_app_focus() {
        let mut owner =
            PerceptionOwner::enabled(SessionId::new()).with_screen_source(Box::new(MockSource {
                events: vec![
                    ScreenEvent::new(ScreenEventKind::WindowSwitch, "x", 1),
                    ScreenEvent::new(ScreenEventKind::AppFocus, "vscode", 1),
                ],
            }));
        owner.poll_screen_at(1);
        assert_eq!(owner.pending().len(), 1);
        assert_eq!(owner.pending()[0].payload["app"], "vscode");
    }

    #[test]
    fn tool_observation_is_side_channel_and_deduped() {
        let owner = PerceptionOwner::enabled(SessionId::new());
        assert!(owner.capture_tool_observation(
            "recall_memory",
            &json!({"query": "考试"}),
            true,
            Some("found 3"),
            None
        ));
        assert!(!owner.capture_tool_observation(
            "recall_memory",
            &json!({"query": "考试"}),
            true,
            Some("found 3"),
            None
        ));
        let drained = owner.drain_tool_observations();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].tool, "recall_memory");
        assert_eq!(drained[0].outcome.label(), "success");
    }

    #[test]
    fn command_payload_keeps_text_for_canonical_request_boundary() {
        let mut owner = PerceptionOwner::enabled(SessionId::new());
        owner.ingest_command(SignalSource::Cli, "/status");
        let event = &owner.pending()[0];
        assert_eq!(event.source, PerceptionModality::Command);
        assert_eq!(event.payload["text"], "/status");
        assert!(event.tags.contains(&"user_initiated".to_string()));
    }

    #[test]
    fn disabled_select_clears_any_stale_pending() {
        let mut owner = PerceptionOwner::disabled(SessionId::new());
        owner.pending.push(text_observation(
            owner.session_id,
            SignalSource::Cli,
            "stale",
            DEFAULT_TEXT_PRIORITY,
            1,
        ));
        assert!(owner.select().is_empty());
        assert!(owner.pending.is_empty());
    }
}
