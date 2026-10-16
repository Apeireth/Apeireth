//! Compress checkpoints: surface replacement over an append-only transcript.
//!
//! Long sessions outgrow the provider window. The blunt response is to cut the
//! transcript, which destroys information, cannot be replayed, and can split a
//! tool call from its result. This module replaces that cut with a checkpoint:
//!
//! - **Append-only**: a compaction records one [`CompactionCheckpoint`] event
//!   ([`stream`]); the original messages are never deleted.
//! - **Surface replacement**: deriving provider messages folds the log — the
//!   checkpoint's span is replaced by its summary, everything else survives
//!   verbatim ([`stream::fold_checkpoints`]). The fold is deterministic: the
//!   same log always replays to the same view.
//! - **Crash-detectable lock**: every checkpoint write is bracketed by a
//!   marker pair (`start -> checkpoint -> end`); an unclosed pair is detected
//!   ([`stream::detect_unclosed_compactions`]) and never applied.
//! - **Selection** ([`select`]): the trigger is the shared overflow trigger
//!   math; the tail is retained verbatim within the shared `retain_tail`
//!   parameter; the cut retreats to a tool-pair-safe boundary.
//! - **Summary** ([`summary`]): the material is the real conversation prefix;
//!   the eight-section template is fixed; one injected [`summary::SummaryGenerator`]
//!   call writes the prose; validation refuses anything that is not strictly
//!   smaller than what it replaces or that drops a prior summary's key facts.
//!   Every failure means *no compaction* — the session stays exactly as it was.
//! - **Pipeline** ([`engine`]): plan → summarize once → accept-or-refuse.

pub mod engine;
pub mod select;
pub mod stream;
pub mod summary;

pub use engine::{CompactionEngine, CompactionOutcome};
pub use select::{
    compaction_due, pair_safe_end, select_compaction_range, CompactionBudget, CompactionRange,
};
pub use stream::{
    detect_unclosed_compactions, fold_checkpoints, render_transcript, CompactionCheckpoint,
    CompactionLogEntry, CompactionMessage, CompactionRole, FoldedView, ViewSegment,
};
pub use summary::{
    validate_summary, SummaryError, SummaryGenerator, SummaryRequest, SummarySections,
    SUMMARY_SECTION_HEADERS,
};
