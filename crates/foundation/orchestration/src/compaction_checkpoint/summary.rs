//! The summary surface: fixed template, injected generator, and the
//! accept-or-refuse validation.
//!
//! The deterministic layer owns everything except the one call that actually
//! writes prose. That call is injected through [`SummaryGenerator`] (tests
//! supply a scripted one), and its answer is only ever *accepted*:
//! [`validate_summary`] refuses anything that is empty, off-template, not
//! strictly smaller than what it replaces, or that drops a prior summary's key
//! facts. A refused summary means no compaction at all — the session stays
//! exactly as it was.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The fixed eight-section template headers, in template order:
/// intent / concepts / files / errors / todos / current / next / key facts.
pub const SUMMARY_SECTION_HEADERS: [&str; 8] = [
    "## 意图",
    "## 概念",
    "## 文件",
    "## 错误",
    "## 待办",
    "## 当前",
    "## 下一步",
    "## 关键",
];

/// The eight template sections.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SummarySections {
    /// What the conversation is trying to do.
    pub intent: String,
    /// Concepts, decisions, and constraints established.
    pub concepts: String,
    /// Files, paths, and resources named.
    pub files: String,
    /// Errors hit and how they were resolved or left.
    pub errors: String,
    /// Open to-dos.
    pub todos: String,
    /// Where things stand right now.
    pub current: String,
    /// Immediate next steps.
    pub next: String,
    /// Key facts that must survive compaction.
    pub keys: String,
}

impl SummarySections {
    /// Render the sections into the fixed template text.
    pub fn render(&self) -> String {
        let bodies = [
            &self.intent,
            &self.concepts,
            &self.files,
            &self.errors,
            &self.todos,
            &self.current,
            &self.next,
            &self.keys,
        ];
        let mut out = String::new();
        for (header, body) in SUMMARY_SECTION_HEADERS.iter().zip(bodies) {
            out.push_str(header);
            out.push('\n');
            out.push_str(body.trim());
            out.push('\n');
        }
        out
    }

    /// Parse template text back into sections, or `None` when any of the eight
    /// headers is missing or out of order.
    pub fn parse(text: &str) -> Option<Self> {
        let mut bodies: Vec<String> = Vec::new();
        let mut rest = text;
        for (index, header) in SUMMARY_SECTION_HEADERS.iter().enumerate() {
            let start = rest.find(header)? + header.len();
            let end = SUMMARY_SECTION_HEADERS
                .get(index + 1)
                .and_then(|next| rest[start..].find(next).map(|offset| start + offset))
                .unwrap_or(rest.len());
            bodies.push(rest[start..end].trim().to_string());
            rest = &rest[end..];
        }
        let mut bodies = bodies.into_iter();
        Some(Self {
            intent: bodies.next().unwrap_or_default(),
            concepts: bodies.next().unwrap_or_default(),
            files: bodies.next().unwrap_or_default(),
            errors: bodies.next().unwrap_or_default(),
            todos: bodies.next().unwrap_or_default(),
            current: bodies.next().unwrap_or_default(),
            next: bodies.next().unwrap_or_default(),
            keys: bodies.next().unwrap_or_default(),
        })
    }

    /// The key-facts section, split into non-empty lines. These are the
    /// fidelity anchor: a new summary must carry every one of them.
    pub fn key_lines(text: &str) -> Vec<String> {
        Self::parse(text)
            .map(|sections| {
                sections
                    .keys
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Material for one summary call: the real conversation prefix being replaced,
/// plus any earlier summaries folded into the same span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryRequest {
    /// The real conversation prefix being replaced, rendered as transcript
    /// text. This is the only factual source for the summary.
    pub transcript: String,
    /// Earlier checkpoint summaries inside the replaced span. They are merged
    /// into the new summary: their key facts must survive, their stale detail
    /// may not.
    pub prior_summaries: Vec<String>,
    /// The fixed template the answer must follow.
    pub template: String,
}

impl SummaryRequest {
    /// A request over one rendered transcript prefix.
    pub fn new(transcript: impl Into<String>) -> Self {
        Self {
            transcript: transcript.into(),
            prior_summaries: Vec::new(),
            template: SummarySections::default().render(),
        }
    }

    /// Add one earlier summary that must be merged into the new one.
    #[must_use]
    pub fn with_prior_summary(mut self, prior: impl Into<String>) -> Self {
        self.prior_summaries.push(prior.into());
        self
    }

    /// The prompt handed to the generator: fixed template, prior summaries,
    /// then the transcript. Deterministic for identical inputs.
    pub fn render_prompt(&self) -> String {
        let mut out = String::from(
            "Summarize the conversation prefix below into the fixed eight-section template.\n\
             Merge every prior compaction summary into the new summary: keep all key facts,\n\
             drop what is stale.\n\n",
        );
        out.push_str("--- template ---\n");
        out.push_str(&self.template);
        if !self.prior_summaries.is_empty() {
            out.push_str("--- prior compaction summaries ---\n");
            for prior in &self.prior_summaries {
                out.push_str(prior.trim());
                out.push('\n');
            }
        }
        out.push_str("--- transcript ---\n");
        out.push_str(&self.transcript);
        out
    }
}

/// Why a summary attempt produced nothing usable. Every variant means the same
/// thing to the caller: do not compact, leave the session untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SummaryError {
    /// The summary provider could not be reached.
    Unavailable(String),
    /// The summary call failed.
    Failed(String),
    /// The answer is empty.
    Empty,
    /// The answer does not follow the fixed eight-section template.
    TemplateMismatch,
    /// The answer is not strictly smaller than the span it replaces.
    NotSmaller {
        /// Characters of the produced summary.
        summary_chars: usize,
        /// Characters of the material it replaces.
        replaced_chars: usize,
    },
    /// A prior summary's key facts were not carried into the new summary.
    PriorKeyFactsDropped,
}

impl std::fmt::Display for SummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(f, "summary unavailable: {reason}"),
            Self::Failed(reason) => write!(f, "summary failed: {reason}"),
            Self::Empty => write!(f, "summary is empty"),
            Self::TemplateMismatch => write!(f, "summary does not follow the fixed template"),
            Self::NotSmaller {
                summary_chars,
                replaced_chars,
            } => write!(
                f,
                "summary is not smaller than what it replaces ({summary_chars} >= {replaced_chars})"
            ),
            Self::PriorKeyFactsDropped => write!(f, "summary dropped key facts of a prior summary"),
        }
    }
}

impl std::error::Error for SummaryError {}

/// The one non-deterministic step: write the summary for a real conversation
/// prefix. One compaction attempt makes exactly one call.
#[async_trait]
pub trait SummaryGenerator: Send + Sync {
    /// Produce the eight-section summary for `request`.
    async fn generate(&self, request: &SummaryRequest) -> Result<String, SummaryError>;
}

/// Accept-or-refuse check for a produced summary.
///
/// A summary may replace `replaced_text` (plus the prior summaries that occupy
/// the same span) only when it is non-empty, follows the fixed template, is
/// strictly smaller than everything it replaces, and carries every key fact of
/// every prior summary. Anything else is refused — and a refusal never leaves a
/// partial state behind, because nothing has been written yet.
pub fn validate_summary(
    summary: &str,
    replaced_text: &str,
    prior_summaries: &[String],
) -> Result<(), SummaryError> {
    if summary.trim().is_empty() {
        return Err(SummaryError::Empty);
    }
    if SummarySections::parse(summary).is_none() {
        return Err(SummaryError::TemplateMismatch);
    }
    let replaced_chars = replaced_text.chars().count()
        + prior_summaries
            .iter()
            .map(|prior| prior.chars().count())
            .sum::<usize>();
    let summary_chars = summary.chars().count();
    if summary_chars >= replaced_chars {
        return Err(SummaryError::NotSmaller {
            summary_chars,
            replaced_chars,
        });
    }
    for prior in prior_summaries {
        for key_line in SummarySections::key_lines(prior) {
            if !summary.contains(&key_line) {
                return Err(SummaryError::PriorKeyFactsDropped);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template_with_keys(keys: &str) -> String {
        SummarySections {
            keys: keys.into(),
            ..SummarySections::default()
        }
        .render()
    }

    #[test]
    fn the_template_round_trips_through_eight_sections() {
        let sections = SummarySections {
            intent: "build the thing".into(),
            concepts: "checkpoints".into(),
            files: "src/lib.rs".into(),
            errors: "none".into(),
            todos: "tests".into(),
            current: "mid-flight".into(),
            next: "write tests".into(),
            keys: "the fact".into(),
        };
        let rendered = sections.render();
        for header in SUMMARY_SECTION_HEADERS {
            assert!(rendered.contains(header), "缺节: {header}");
        }
        assert_eq!(SummarySections::parse(&rendered), Some(sections));
    }

    #[test]
    fn off_template_or_empty_summaries_are_refused() {
        assert_eq!(
            validate_summary("", "0123456789", &[]),
            Err(SummaryError::Empty)
        );
        assert_eq!(
            validate_summary("just prose, smaller", "0123456789", &[]),
            Err(SummaryError::TemplateMismatch)
        );
    }

    #[test]
    fn a_summary_not_smaller_than_its_span_is_refused() {
        let big = template_with_keys(&"k".repeat(200));
        assert_eq!(
            validate_summary(&big, "tiny", &[]),
            Err(SummaryError::NotSmaller {
                summary_chars: big.chars().count(),
                replaced_chars: 4,
            })
        );
    }

    #[test]
    fn a_prior_summary_key_fact_must_survive_the_merge() {
        let prior = template_with_keys("keep this fact");
        let merged = template_with_keys("keep this fact\nand this one");
        assert!(validate_summary(&merged, "replaced material span", &[prior.clone()]).is_ok());

        let dropped = template_with_keys("something else entirely");
        assert_eq!(
            validate_summary(&dropped, "replaced material span", &[prior]),
            Err(SummaryError::PriorKeyFactsDropped)
        );
    }
}
