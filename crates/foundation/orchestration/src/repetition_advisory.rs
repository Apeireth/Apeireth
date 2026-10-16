//! Advisory notices for consecutive identical tool calls.
//!
//! A loop that re-issues one tool call with identical arguments burns rounds
//! without making progress. This module notices that pattern and produces a
//! purely advisory text which the runtime attaches as *additional context* to
//! the tool result the model already receives.
//!
//! # What this is not
//!
//! - It never blocks execution: the call runs first, every time.
//! - It never changes the tool result value itself; the advisory is appended to
//!   the transported message text around the untouched rendering.
//! - It never enters any governance or audit decision. It is advice to the
//!   model, nothing more.
//!
//! # Identity (the chain key)
//!
//! Two calls are identical when the tool name and the **full** arguments match
//! after canonical serialization: object keys are sorted at every depth (so key
//! order is irrelevant), array order is preserved, and nothing is omitted. The
//! display preview is truncated for readability, but truncation never affects
//! the identity key.
//!
//! # Counting
//!
//! Only *consecutive* occurrences of one chain key count. A different chain key
//! starts a fresh streak of one. A user message clears the streak entirely.
//! Tools on the exclusion list are transparent: they neither advance nor clear
//! any streak.
//!
//! # Reminders
//!
//! Thresholds are configurable constants, default `3 / 5 / 8`: the middle
//! thresholds produce a gentle reminder, the largest threshold produces a
//! detailed reminder that lists the attempt count and the parameter preview and
//! suggests changing the angle or asking the user. A reminder fires exactly
//! when the streak count equals a configured threshold.

use serde_json::Value;

/// Marker that separates the untouched result rendering from the appended
/// advisory context in the transported tool-result message.
pub const RESULT_CONTEXT_MARKER: &str = "[repetition-advisory]";

/// Separator inside chain keys. Not representable in tool names or JSON text
/// produced below, so a key can never be confused across its two halves.
const KEY_SEPARATOR: char = '\u{1f}';

/// Canonical serialization of full tool arguments.
///
/// Object keys are sorted at every depth so that argument key order can never
/// change the output. Array order is preserved because it is semantic. The
/// output is the complete value: this is the identity input, never truncated.
pub fn canonical_arguments(arguments: &Value) -> String {
    let mut out = String::new();
    write_canonical(arguments, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Object(map) => {
            out.push('{');
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(&Value::String((*key).clone()), out);
                out.push(':');
                if let Some(item) = map.get(*key) {
                    write_canonical(item, out);
                }
            }
            out.push('}');
        }
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&other.to_string()),
    }
}

/// The chain key: tool name plus canonical full arguments.
///
/// This is the identity used for streak counting. It is derived from the full
/// arguments, so display-side preview truncation can never merge two distinct
/// calls into one key.
pub fn chain_key(tool_name: &str, arguments: &Value) -> String {
    format!(
        "{tool_name}{KEY_SEPARATOR}{}",
        canonical_arguments(arguments)
    )
}

/// Human-readable parameter preview, truncated to `max_chars` characters.
///
/// Truncation is display-only and appends a visible ellipsis; it never feeds
/// back into [`chain_key`].
pub fn params_preview(arguments: &Value, max_chars: usize) -> String {
    let canonical = canonical_arguments(arguments);
    if canonical.chars().count() <= max_chars {
        return canonical;
    }
    let mut preview: String = canonical.chars().take(max_chars).collect();
    preview.push('…');
    preview
}

/// How strongly a reminder is worded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryLevel {
    /// Short nudge at the middle thresholds.
    Gentle,
    /// Full reminder at the largest threshold: attempt count, parameter
    /// preview, and a concrete suggestion to change angle or ask the user.
    Detailed,
}

/// One produced reminder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepetitionAdvisory {
    /// Tool whose call streak crossed a threshold.
    pub tool_name: String,
    /// Streak length that triggered this reminder (the current occurrence).
    pub occurrence: u32,
    /// Wording strength.
    pub level: AdvisoryLevel,
    /// Truncated parameter preview shown in the detailed wording.
    pub params_preview: String,
    /// The advisory body carried to the model.
    pub text: String,
}

/// Configurable counting and wording policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepetitionPolicy {
    /// Streak counts at which a reminder fires. The largest value produces the
    /// detailed wording; the others are gentle. An empty list disables all
    /// reminders (counting still happens but is never surfaced).
    pub thresholds: Vec<u32>,
    /// Tools that are transparent to the detector: never counted and never
    /// clearing another tool's streak.
    pub excluded_tools: Vec<String>,
    /// Character budget for the displayed parameter preview.
    pub preview_chars: usize,
}

impl Default for RepetitionPolicy {
    fn default() -> Self {
        Self {
            thresholds: vec![3, 5, 8],
            excluded_tools: Vec::new(),
            preview_chars: 120,
        }
    }
}

impl RepetitionPolicy {
    /// Set the reminder thresholds (configurable constants).
    #[must_use]
    pub fn with_thresholds(mut self, thresholds: Vec<u32>) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Set the exclusion list.
    #[must_use]
    pub fn with_excluded_tools(mut self, tools: Vec<String>) -> Self {
        self.excluded_tools = tools;
        self
    }

    /// Set the parameter preview budget.
    #[must_use]
    pub fn with_preview_chars(mut self, chars: usize) -> Self {
        self.preview_chars = chars;
        self
    }

    /// Whether a tool is excluded from counting (and from clearing).
    pub fn is_excluded(&self, tool_name: &str) -> bool {
        self.excluded_tools.iter().any(|tool| tool == tool_name)
    }

    /// The reminder level for a streak count, or `None` when no threshold is
    /// crossed at that count.
    pub fn level_for(&self, occurrence: u32) -> Option<AdvisoryLevel> {
        if !self.thresholds.contains(&occurrence) {
            return None;
        }
        let largest = self.thresholds.iter().copied().max();
        if Some(occurrence) == largest {
            Some(AdvisoryLevel::Detailed)
        } else {
            Some(AdvisoryLevel::Gentle)
        }
    }
}

/// Session-scoped streak state for consecutive identical tool calls.
#[derive(Debug, Clone, Default)]
pub struct RepetitionDetector {
    policy: RepetitionPolicy,
    streak_key: Option<String>,
    streak_tool: String,
    streak_preview: String,
    streak: u32,
}

impl RepetitionDetector {
    /// A detector under one policy.
    pub fn new(policy: RepetitionPolicy) -> Self {
        Self {
            policy,
            streak_key: None,
            streak_tool: String::new(),
            streak_preview: String::new(),
            streak: 0,
        }
    }

    /// The active policy.
    pub fn policy(&self) -> &RepetitionPolicy {
        &self.policy
    }

    /// Current consecutive count for the active chain key.
    pub fn streak(&self) -> u32 {
        self.streak
    }

    /// The active chain key, when a streak is running.
    pub fn streak_key(&self) -> Option<&str> {
        self.streak_key.as_deref()
    }

    /// A user message clears the streak entirely.
    pub fn observe_user_message(&mut self) {
        self.streak_key = None;
        self.streak_tool = String::new();
        self.streak_preview = String::new();
        self.streak = 0;
    }

    /// Observe one executed tool call and return a reminder when the streak
    /// crosses a configured threshold.
    ///
    /// Excluded tools are transparent: no counting and no clearing. The call
    /// itself is never blocked here; this method only reports a pattern.
    pub fn observe_tool_call(
        &mut self,
        tool_name: &str,
        arguments: &Value,
    ) -> Option<RepetitionAdvisory> {
        if self.policy.is_excluded(tool_name) {
            return None;
        }

        let key = chain_key(tool_name, arguments);
        if self.streak_key.as_deref() == Some(key.as_str()) {
            self.streak = self.streak.saturating_add(1);
        } else {
            self.streak_key = Some(key);
            self.streak_tool = tool_name.to_string();
            self.streak_preview = params_preview(arguments, self.policy.preview_chars);
            self.streak = 1;
        }

        let level = self.policy.level_for(self.streak)?;
        let text = advisory_text(level, tool_name, self.streak, &self.streak_preview);
        Some(RepetitionAdvisory {
            tool_name: tool_name.to_string(),
            occurrence: self.streak,
            level,
            params_preview: self.streak_preview.clone(),
            text,
        })
    }
}

/// Build the advisory body for one reminder.
fn advisory_text(
    level: AdvisoryLevel,
    tool_name: &str,
    occurrence: u32,
    params_preview: &str,
) -> String {
    match level {
        AdvisoryLevel::Gentle => format!(
            "同一工具调用已连续重复 {occurrence} 次（工具 {tool_name}，参数完全相同）。\
             本提醒不影响本次执行与结果。若继续原样重复很可能原地打转，\
             建议调整参数或换一个角度后再试。"
        ),
        AdvisoryLevel::Detailed => format!(
            "同一工具调用已连续重复 {occurrence} 次（工具 {tool_name}，参数完全相同）。\
             已尝试次数：{occurrence}。参数预览：{params_preview}。\
             本提醒不影响本次执行与结果。继续原样重复几乎必然原地打转：\
             请换一个角度处理当前问题，或直接向用户询问下一步该怎么做。"
        ),
    }
}

/// Compose the transported tool-result message text: the untouched result
/// rendering first, then the advisory as clearly marked additional context.
///
/// The tool result value itself is never modified; only the message text that
/// carries it gains an appended section after the original rendering.
pub fn append_result_context(rendered_result: &str, advisory: &RepetitionAdvisory) -> String {
    format!(
        "{rendered_result}\n\n{RESULT_CONTEXT_MARKER}\n{}",
        advisory.text
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn chain_key_is_stable_across_object_key_order() {
        let a = json!({ "alpha": 1, "beta": { "x": true, "y": [1, 2] } });
        let b = json!({ "beta": { "y": [1, 2], "x": true }, "alpha": 1 });
        assert_eq!(
            chain_key("tool.lookup", &a),
            chain_key("tool.lookup", &b),
            "object key order must not change the identity key"
        );
        assert_ne!(
            chain_key("tool.lookup", &a),
            chain_key("tool.other", &a),
            "the tool name is part of the key"
        );
    }

    #[test]
    fn chain_key_covers_full_arguments_while_preview_truncates() {
        // Two calls that agree inside the preview window but differ past it.
        let mut first = json!({ "path": "aaaa" });
        let mut second = json!({ "path": "aaaa" });
        first["path"] = json!(format!("{}1", "a".repeat(50)));
        second["path"] = json!(format!("{}2", "a".repeat(50)));

        assert_eq!(
            params_preview(&first, 16),
            params_preview(&second, 16),
            "the truncated previews coincide"
        );
        assert!(
            params_preview(&first, 16).ends_with('…'),
            "a truncated preview must show the ellipsis"
        );
        assert_ne!(
            chain_key("tool.lookup", &first),
            chain_key("tool.lookup", &second),
            "preview truncation must never merge two distinct chain keys"
        );

        // The preview is a prefix of the full canonical arguments.
        let full = canonical_arguments(&first);
        let preview = params_preview(&first, 16);
        assert!(full.starts_with(&preview.trim_end_matches('…').to_string()));
    }

    #[test]
    fn consecutive_identical_calls_count_up_until_a_threshold_fires() {
        let mut detector = RepetitionDetector::new(RepetitionPolicy::default());
        let args = json!({ "query": "same" });

        assert!(detector.observe_tool_call("search", &args).is_none());
        assert!(detector.observe_tool_call("search", &args).is_none());
        let third = detector
            .observe_tool_call("search", &args)
            .expect("the third consecutive occurrence must remind");
        assert_eq!(third.occurrence, 3);
        assert_eq!(detector.streak(), 3);

        assert!(detector.observe_tool_call("search", &args).is_none());
        assert!(detector.observe_tool_call("search", &args).is_some());
    }

    #[test]
    fn a_user_message_clears_the_streak() {
        let mut detector = RepetitionDetector::new(RepetitionPolicy::default());
        let args = json!({ "query": "same" });

        detector.observe_tool_call("search", &args);
        detector.observe_tool_call("search", &args);
        assert_eq!(detector.streak(), 2);

        detector.observe_user_message();
        assert_eq!(detector.streak(), 0);
        assert_eq!(detector.streak_key(), None);

        assert!(detector.observe_tool_call("search", &args).is_none());
        assert_eq!(
            detector.streak(),
            1,
            "counting restarts from one after a user message"
        );
    }

    #[test]
    fn a_different_chain_key_breaks_the_streak() {
        let mut detector = RepetitionDetector::new(RepetitionPolicy::default());
        let first = json!({ "query": "one" });
        let second = json!({ "query": "two" });

        detector.observe_tool_call("search", &first);
        detector.observe_tool_call("search", &first);
        detector.observe_tool_call("search", &second);
        assert_eq!(detector.streak(), 1, "a different call starts a new streak");

        detector.observe_tool_call("search", &first);
        assert_eq!(detector.streak(), 1, "returning to a key restarts at one");
    }

    #[test]
    fn excluded_tools_are_transparent_to_counting_and_clearing() {
        let policy = RepetitionPolicy::default()
            .with_excluded_tools(vec!["clock".to_string(), "heartbeat".to_string()]);
        let mut detector = RepetitionDetector::new(policy);
        let args = json!({ "query": "same" });

        detector.observe_tool_call("search", &args);
        detector.observe_tool_call("search", &args);

        // The excluded tool is neither counted nor clearing.
        for _ in 0..4 {
            assert!(detector.observe_tool_call("clock", &args).is_none());
        }
        assert_eq!(
            detector.streak(),
            2,
            "excluded calls must not clear the streak"
        );

        let third = detector
            .observe_tool_call("search", &args)
            .expect("the streak survives excluded calls and reaches the threshold");
        assert_eq!(third.occurrence, 3);

        // And an excluded tool never reminds, even absurdly deep into a loop.
        let mut solo = RepetitionDetector::new(
            RepetitionPolicy::default().with_excluded_tools(vec!["clock".to_string()]),
        );
        for _ in 0..20 {
            assert!(solo.observe_tool_call("clock", &args).is_none());
        }
        assert_eq!(solo.streak(), 0);
    }

    #[test]
    fn threshold_progression_uses_gentle_then_detailed_text() {
        let mut detector = RepetitionDetector::new(RepetitionPolicy::default());
        let args = json!({ "query": "same" });

        let mut fired = Vec::new();
        for occurrence in 1..=8 {
            if let Some(advisory) = detector.observe_tool_call("search", &args) {
                fired.push((occurrence, advisory));
            }
        }

        assert_eq!(
            fired.iter().map(|(o, _)| *o).collect::<Vec<_>>(),
            vec![3, 5, 8],
            "reminders fire exactly at the configured thresholds"
        );
        assert_eq!(fired[0].1.level, AdvisoryLevel::Gentle);
        assert_eq!(fired[1].1.level, AdvisoryLevel::Gentle);
        assert_eq!(fired[2].1.level, AdvisoryLevel::Detailed);

        let gentle = &fired[0].1.text;
        assert!(
            gentle.contains("3"),
            "the gentle text names the count: {gentle}"
        );

        let detailed = &fired[2].1.text;
        assert!(
            detailed.contains("已尝试次数：8"),
            "the detailed text lists the attempts: {detailed}"
        );
        assert!(
            detailed.contains("参数预览"),
            "the detailed text includes the parameter preview: {detailed}"
        );
        assert!(
            detailed.contains("换一个角度") && detailed.contains("向用户询问"),
            "the detailed text suggests changing angle or asking the user: {detailed}"
        );
    }

    #[test]
    fn thresholds_are_configurable_constants() {
        let mut detector =
            RepetitionDetector::new(RepetitionPolicy::default().with_thresholds(vec![2, 4]));
        let args = json!({ "query": "same" });

        assert!(detector.observe_tool_call("search", &args).is_none());
        let second = detector.observe_tool_call("search", &args).unwrap();
        assert_eq!(second.level, AdvisoryLevel::Gentle);
        assert!(detector.observe_tool_call("search", &args).is_none());
        let fourth = detector.observe_tool_call("search", &args).unwrap();
        assert_eq!(fourth.level, AdvisoryLevel::Detailed);

        // An empty threshold list disables reminders entirely.
        let mut quiet =
            RepetitionDetector::new(RepetitionPolicy::default().with_thresholds(Vec::new()));
        for _ in 0..10 {
            assert!(quiet.observe_tool_call("search", &args).is_none());
        }
    }

    #[test]
    fn advisory_context_appends_after_the_untouched_result_text() {
        let mut detector = RepetitionDetector::new(RepetitionPolicy::default());
        let args = json!({ "query": "same" });
        for _ in 0..2 {
            detector.observe_tool_call("search", &args);
        }
        let advisory = detector.observe_tool_call("search", &args).unwrap();

        let rendered = "{\"hits\":3}";
        let combined = append_result_context(rendered, &advisory);
        assert!(
            combined.starts_with(rendered),
            "the result rendering must stay first and intact: {combined}"
        );
        let marker_at = combined
            .find(RESULT_CONTEXT_MARKER)
            .expect("the advisory is carried under the marker");
        assert!(marker_at > rendered.len(), "the advisory is appended after");
        assert!(
            combined.contains(&advisory.text),
            "the advisory body rides along with the result"
        );
    }

    #[test]
    fn the_advisory_channel_only_advises_and_never_intercepts_execution() {
        let mut detector = RepetitionDetector::new(RepetitionPolicy::default());
        let args = json!({ "query": "same" });

        // The tool executes and renders its own result first, every time; the
        // detector observes afterwards and can only advise. Nothing in this
        // channel can short-circuit a call or rewrite its output.
        let mut executions = 0u32;
        let mut whole_results = 0u32;
        for _ in 0..9 {
            let rendered = "{\"hits\":3}"; // the tool produced its result
            executions += 1;
            match detector.observe_tool_call("search", &args) {
                Some(advisory) => {
                    // Advice rides along; the result rendering stays first/intact.
                    assert!(append_result_context(rendered, &advisory).starts_with(rendered));
                }
                None => {} // no advice; the result is still returned whole
            }
            whole_results += 1;
        }
        assert_eq!(executions, 9, "every call runs; none is intercepted");
        assert_eq!(whole_results, 9, "every result is returned whole");
        assert_eq!(
            detector.streak(),
            9,
            "counting advanced for every execution"
        );
    }
}
