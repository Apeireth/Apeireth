//! Screen salience observation recovered from companion `screen_perception.rs`.
//!
//! Distinct from `XcapVisionBackend`: Xcap captures pixel frames; this module
//! scores **window/idle events** (switch / focus / idle start / idle resume).
//! The default source is an honest no-op: unconnected perception does not
//! pretend to see the desktop.
//!
//! Idle detection is deterministic and does not depend on the OS source.
//! IdleStart is emitted at most once per idle stretch (canonical `poll_events`
//! would re-emit every poll after the threshold; that is a leak, not a
//! semantic).

use apeireth_core::kernel::SessionId;
use apeireth_plugin::perception::PerceptionEvent;

use crate::normalize::{now_timestamp_ms, vision_observation, with_tag, SignalSource};

/// Default idle threshold: 5 minutes without a source event.
pub const DEFAULT_IDLE_THRESHOLD_MS: i64 = 5 * 60 * 1000;

/// Minimum significance that is worth feeding the attention pipeline.
pub const PERCEIVE_THRESHOLD: f64 = 0.3;

/// Screen salience event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenEventKind {
    /// Foreground window changed.
    WindowSwitch,
    /// Application held focus (deep engagement).
    AppFocus,
    /// Idle stretch started.
    IdleStart,
    /// User returned from idle.
    IdleResume,
}

impl ScreenEventKind {
    /// Stable payload label.
    pub fn label(self) -> &'static str {
        match self {
            Self::WindowSwitch => "window_switch",
            Self::AppFocus => "app_focus",
            Self::IdleStart => "idle_start",
            Self::IdleResume => "idle_resume",
        }
    }
}

/// One screen salience event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenEvent {
    /// Kind of salience.
    pub kind: ScreenEventKind,
    /// Application / window identifier (e.g. `"vscode"`).
    pub app: String,
    /// Event time (epoch millis).
    pub at_ms: i64,
}

impl ScreenEvent {
    /// Construct a screen event.
    pub fn new(kind: ScreenEventKind, app: impl Into<String>, at_ms: i64) -> Self {
        Self {
            kind,
            app: app.into(),
            at_ms,
        }
    }
}

/// Screen event source. Implementations poll OS APIs; the default is no-op.
pub trait ScreenEventSource: Send + Sync + std::fmt::Debug {
    /// Poll once: events since the previous poll.
    fn poll(&mut self) -> Vec<ScreenEvent>;
}

/// Honest unconnected source: never claims to see the screen.
#[derive(Debug, Default)]
pub struct NoopScreenSource;

impl ScreenEventSource for NoopScreenSource {
    fn poll(&mut self) -> Vec<ScreenEvent> {
        Vec::new()
    }
}

/// Screen perception: source passthrough + idle detection + significance scoring.
#[derive(Debug)]
pub struct ScreenPerception {
    source: Box<dyn ScreenEventSource>,
    /// Idle starts after this many milliseconds without a source event.
    pub idle_threshold_ms: i64,
    last_event_ms: Option<i64>,
    idle_emitted: bool,
}

impl ScreenPerception {
    /// Wrap a source with the default 5-minute idle threshold.
    pub fn new(source: Box<dyn ScreenEventSource>) -> Self {
        Self {
            source,
            idle_threshold_ms: DEFAULT_IDLE_THRESHOLD_MS,
            last_event_ms: None,
            idle_emitted: false,
        }
    }

    /// Poll source events and, if needed, emit a single IdleStart.
    pub fn poll_events(&mut self) -> Vec<ScreenEvent> {
        self.poll_events_at(now_timestamp_ms())
    }

    /// Time-injected poll (tests and deterministic owners).
    pub fn poll_events_at(&mut self, now_ms: i64) -> Vec<ScreenEvent> {
        let mut out = self.source.poll();
        if out.is_empty() {
            if let Some(last) = self.last_event_ms {
                if !self.idle_emitted && now_ms - last > self.idle_threshold_ms {
                    out.push(ScreenEvent::new(
                        ScreenEventKind::IdleStart,
                        "system",
                        now_ms,
                    ));
                    self.idle_emitted = true;
                }
            }
        } else {
            let was_idle = self.idle_emitted;
            self.last_event_ms = Some(now_ms);
            self.idle_emitted = false;
            if was_idle
                && !out
                    .iter()
                    .any(|event| event.kind == ScreenEventKind::IdleResume)
            {
                out.insert(
                    0,
                    ScreenEvent::new(ScreenEventKind::IdleResume, "system", now_ms),
                );
            }
        }
        out
    }

    /// Significance in `[0, 1]`. WindowSwitch is low, IdleResume is high.
    pub fn significance(&self, event: &ScreenEvent) -> f64 {
        match event.kind {
            ScreenEventKind::WindowSwitch => 0.2,
            ScreenEventKind::AppFocus => 0.5,
            ScreenEventKind::IdleStart => 0.3,
            ScreenEventKind::IdleResume => 0.8,
        }
    }

    /// Whether the event is worth feeding the perception pipeline.
    pub fn should_perceive(&self, event: &ScreenEvent) -> bool {
        self.significance(event) >= PERCEIVE_THRESHOLD
    }

    /// Convert a salience event into a canonical vision `PerceptionEvent`.
    ///
    /// Pixel dimensions are unknown for window/idle events (not a screenshot),
    /// so width/height stay 0 and the attention score is the significance
    /// (not the pixel heuristic).
    pub fn to_observation(&self, session_id: SessionId, event: &ScreenEvent) -> PerceptionEvent {
        let mut observation = vision_observation(
            session_id,
            SignalSource::Internal,
            0,
            0,
            Some(format!("{}:{}", event.kind.label(), event.app)),
            event.at_ms,
        );
        observation.attention_score = self.significance(event);
        observation.payload["screen_kind"] = serde_json::json!(event.kind.label());
        observation.payload["app"] = serde_json::json!(event.app);
        observation = with_tag(observation, "screen");
        observation = with_tag(observation, event.kind.label());
        observation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockSource {
        events: Vec<ScreenEvent>,
    }

    impl ScreenEventSource for MockSource {
        fn poll(&mut self) -> Vec<ScreenEvent> {
            std::mem::take(&mut self.events)
        }
    }

    fn ev(kind: ScreenEventKind, app: &str) -> ScreenEvent {
        ScreenEvent::new(kind, app, 1_000)
    }

    #[test]
    fn events_pass_through_with_significance() {
        let mut perception = ScreenPerception::new(Box::new(MockSource {
            events: vec![ev(ScreenEventKind::AppFocus, "vscode")],
        }));
        let events = perception.poll_events_at(1_000);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].app, "vscode");
        assert!(perception.should_perceive(&events[0]));
        assert!(!perception.should_perceive(&ev(ScreenEventKind::WindowSwitch, "x")));
    }

    #[test]
    fn idle_detection_after_threshold_emits_once() {
        let mut perception = ScreenPerception::new(Box::new(MockSource { events: Vec::new() }));
        perception.idle_threshold_ms = 1_000;
        perception.last_event_ms = Some(0);
        let first = perception.poll_events_at(5_000);
        assert!(first
            .iter()
            .any(|event| event.kind == ScreenEventKind::IdleStart));
        let second = perception.poll_events_at(6_000);
        assert!(
            second.is_empty(),
            "IdleStart must not re-emit every empty poll"
        );
    }

    #[test]
    fn idle_resume_is_synthesized_when_activity_returns() {
        let mut perception = ScreenPerception::new(Box::new(MockSource { events: Vec::new() }));
        perception.idle_threshold_ms = 1_000;
        perception.last_event_ms = Some(0);
        let _ = perception.poll_events_at(5_000);
        perception.source = Box::new(MockSource {
            events: vec![ev(ScreenEventKind::AppFocus, "vscode")],
        });
        let events = perception.poll_events_at(6_000);
        assert_eq!(events[0].kind, ScreenEventKind::IdleResume);
        assert_eq!(events[1].kind, ScreenEventKind::AppFocus);
    }

    #[test]
    fn noop_source_is_honest() {
        let mut perception = ScreenPerception::new(Box::new(NoopScreenSource));
        assert!(
            perception.poll_events_at(now_timestamp_ms()).is_empty(),
            "unconnected source must not pretend to see the screen"
        );
    }

    #[test]
    fn significance_ranking() {
        let perception = ScreenPerception::new(Box::new(NoopScreenSource));
        let idle = ev(ScreenEventKind::IdleResume, "s");
        let switch = ev(ScreenEventKind::WindowSwitch, "s");
        assert!(perception.significance(&idle) > perception.significance(&switch));
        assert!((perception.significance(&switch) - 0.2).abs() < 1e-9);
    }

    #[test]
    fn to_observation_carries_screen_tags_and_significance() {
        let perception = ScreenPerception::new(Box::new(NoopScreenSource));
        let event = ev(ScreenEventKind::IdleResume, "vscode");
        let observation = perception.to_observation(SessionId::new(), &event);
        assert_eq!(observation.payload["app"], "vscode");
        assert_eq!(observation.payload["screen_kind"], "idle_resume");
        assert!((observation.attention_score - 0.8).abs() < 1e-9);
        assert!(observation.tags.contains(&"screen".to_string()));
        assert!(observation.tags.contains(&"idle_resume".to_string()));
    }
}
