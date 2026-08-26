//! The canonical event primitive.
//!
//! This is the *shape* of an event, not a bus. Transport is somebody else's
//! problem: `apeireth-bus` moves these, observers consume them, the runtime emits
//! them. Core owns only the envelope so that every subsystem agrees on what an
//! event looks like without agreeing on how it travels.
//!
//! An [`Event`] always carries a [`TraceId`], which is what makes a turn
//! reconstructible after the fact without shipping raw model reasoning around.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::{CoreError, CoreResult};
use super::ids::TraceId;
use super::metadata::Metadata;
use super::time::{Clock, Timestamp};

/// A dot-separated event topic, e.g. `runtime.turn.completed`.
///
/// Shares the stable-identifier grammar so topics stay greppable and cannot drift
/// into free text.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Topic(String);

impl Topic {
    /// Validate and wrap a topic.
    pub fn new(raw: impl Into<String>) -> CoreResult<Self> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(CoreError::invalid_id("Topic", raw, "must not be empty"));
        }
        if !raw.starts_with(|c: char| c.is_ascii_lowercase()) {
            return Err(CoreError::invalid_id(
                "Topic",
                raw,
                "must start with an ASCII lowercase letter",
            ));
        }
        for c in raw.chars() {
            let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_');
            if !ok {
                return Err(CoreError::invalid_id(
                    "Topic",
                    &raw,
                    format!("illegal character {c:?}; allowed: a-z 0-9 . - _"),
                ));
            }
        }
        if raw.split('.').any(str::is_empty) {
            return Err(CoreError::invalid_id(
                "Topic",
                raw,
                "must not contain an empty dot-separated segment",
            ));
        }
        Ok(Self(raw))
    }

    /// The topic as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this topic sits under `prefix`.
    ///
    /// Matching is segment-aware: `runtime.turn` covers `runtime.turn.completed`
    /// but not `runtime.turnip`.
    pub fn starts_with_segment(&self, prefix: &str) -> bool {
        match self.0.strip_prefix(prefix) {
            Some("") => true,
            Some(rest) => rest.starts_with('.'),
            None => false,
        }
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Topic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

/// Something that happened, with enough context to correlate it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// What happened.
    pub topic: Topic,
    /// The activity this event belongs to.
    pub trace: TraceId,
    /// When it happened, per the emitter's clock.
    pub at: Timestamp,
    /// Structured payload. `Null` when the topic alone is the whole story.
    pub payload: serde_json::Value,
    /// Additional annotations.
    pub metadata: Metadata,
}

impl Event {
    /// Build an event, reading the time from an injected clock.
    pub fn new(
        topic: Topic,
        trace: TraceId,
        clock: &dyn Clock,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            topic,
            trace,
            at: Timestamp::from_clock(clock),
            payload,
            metadata: Metadata::new(),
        }
    }

    /// Builder-style annotation.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::time::VirtualClock;

    fn fixed_clock() -> VirtualClock {
        VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        )
    }

    #[test]
    fn topic_enforces_the_stable_grammar() {
        assert!(Topic::new("runtime.turn.completed").is_ok());
        assert!(Topic::new("Runtime.Turn").is_err());
        assert!(Topic::new("runtime..turn").is_err());
        assert!(Topic::new("").is_err());
        assert!(Topic::new("runtime turn").is_err());
    }

    #[test]
    fn prefix_matching_respects_segment_boundaries() {
        let t = Topic::new("runtime.turn.completed").unwrap();
        assert!(t.starts_with_segment("runtime"));
        assert!(t.starts_with_segment("runtime.turn"));
        assert!(t.starts_with_segment("runtime.turn.completed"));
        assert!(!t.starts_with_segment("runtime.turnip"));
        assert!(!t.starts_with_segment("run"));
    }

    #[test]
    fn event_takes_its_time_from_the_injected_clock() {
        let clock = fixed_clock();
        let ev = Event::new(
            Topic::new("runtime.turn.started").unwrap(),
            TraceId::new(),
            &clock,
            serde_json::json!({ "rounds": 0 }),
        );
        assert_eq!(ev.at.epoch_millis(), 1_700_000_000_000);

        clock.advance(chrono::Duration::seconds(5));
        let later = Event::new(
            Topic::new("runtime.turn.completed").unwrap(),
            ev.trace,
            &clock,
            serde_json::Value::Null,
        );
        assert_eq!(later.at.epoch_millis() - ev.at.epoch_millis(), 5_000);
        assert_eq!(later.trace, ev.trace, "trace correlates the pair");
    }

    #[test]
    fn round_trips_through_json() {
        let clock = fixed_clock();
        let ev = Event::new(
            Topic::new("plugin.registered").unwrap(),
            TraceId::new(),
            &clock,
            serde_json::json!({ "id": "builtin.calculator" }),
        )
        .with_metadata("source", "test");

        let back: Event = serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
        assert_eq!(ev, back);
    }
}
