//! The compaction engine: plan, summarize once, accept-or-refuse.
//!
//! One compaction attempt is deliberately linear and boring:
//!
//! 1. select the span (`select.rs`) — tail kept verbatim, cut pair-safe;
//! 2. build the summary request from the *real* conversation prefix plus any
//!    prior summaries folded into the span;
//! 3. one call to the injected [`SummaryGenerator`];
//! 4. accept-or-refuse (`summary.rs`).
//!
//! Any failure at any step returns [`CompactionOutcome::Skipped`] and writes
//! nothing: the session stays exactly as it was. Only a fully accepted summary
//! becomes a [`CompactionCheckpoint`], and the caller then appends it under a
//! marker pair (`start -> checkpoint -> end`).

use std::sync::Arc;

use super::select::{select_compaction_range, CompactionBudget, CompactionRange};
use super::stream::{
    fold_checkpoints, render_transcript, CompactionCheckpoint, CompactionLogEntry,
    CompactionMessage,
};
use super::summary::{validate_summary, SummaryError, SummaryGenerator, SummaryRequest};

/// One compaction attempt's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactionOutcome {
    /// An accepted checkpoint, ready to append to the session's event log
    /// under its marker pair.
    Compacted {
        /// The accepted checkpoint.
        checkpoint: CompactionCheckpoint,
    },
    /// Nothing was compacted; the session must stay exactly as it is.
    Skipped {
        /// Why no compaction happened.
        reason: String,
    },
}

impl CompactionOutcome {
    /// Whether a checkpoint was produced.
    pub fn is_compacted(&self) -> bool {
        matches!(self, Self::Compacted { .. })
    }

    fn skipped(reason: impl Into<String>) -> Self {
        Self::Skipped {
            reason: reason.into(),
        }
    }
}

/// Plans one compaction and runs the injected summary generator once.
pub struct CompactionEngine {
    generator: Arc<dyn SummaryGenerator>,
}

impl CompactionEngine {
    /// An engine whose summaries come from `generator`.
    pub fn new(generator: Arc<dyn SummaryGenerator>) -> Self {
        Self { generator }
    }

    /// Attempt one compaction over `messages`, honouring the checkpoints
    /// already recorded in `log`.
    ///
    /// The caller gates on [`CompactionBudget::is_due`] first: this method only
    /// decides *whether a valid checkpoint results*, never *whether the window
    /// is full enough to try*.
    pub async fn compact(
        &self,
        messages: &[CompactionMessage],
        log: &[CompactionLogEntry],
        budget: &CompactionBudget,
    ) -> CompactionOutcome {
        // Prior summaries whose span is fully inside the new span merge into
        // the new summary; one that only half-overlaps cannot, so compaction
        // declines instead of leaving a hole in the view.
        let folded = fold_checkpoints(messages.len(), log);
        let mut prior_summaries: Vec<(usize, String)> = Vec::new();

        let retain = budget.retain_tail();
        let Some(CompactionRange { start_seq, end_seq }) =
            select_compaction_range(messages, retain)
        else {
            return CompactionOutcome::skipped("no pair-safe compressible span");
        };
        let range = start_seq..end_seq;

        for segment in &folded.segments {
            let super::stream::ViewSegment::Summary {
                start_seq,
                end_seq,
                summary,
                marker,
            } = segment
            else {
                continue;
            };
            let span = *start_seq..*end_seq;
            let disjoint = span.end <= range.start || range.end <= span.start;
            if disjoint {
                continue;
            }
            let fully_covered = range.start <= span.start && span.end <= range.end;
            if !fully_covered {
                return CompactionOutcome::skipped(format!(
                    "span [{}, {}) half-replaces standing summary {marker}",
                    range.start, range.end
                ));
            }
            prior_summaries.push((*start_seq, summary.clone()));
        }
        prior_summaries.sort_by_key(|(start_seq, _)| *start_seq);

        let transcript = render_transcript(&messages[range.clone()]);
        let mut request = SummaryRequest::new(transcript.clone());
        for (_, prior) in &prior_summaries {
            request = request.with_prior_summary(prior.clone());
        }

        let summary = match self.generator.generate(&request).await {
            Ok(summary) => summary,
            Err(error) => {
                let reason = match error {
                    SummaryError::Unavailable(detail) => format!("summary unavailable: {detail}"),
                    other => other.to_string(),
                };
                return CompactionOutcome::skipped(reason);
            }
        };

        let priors: Vec<String> = prior_summaries
            .into_iter()
            .map(|(_, prior)| prior)
            .collect();
        if let Err(error) = validate_summary(&summary, &transcript, &priors) {
            return CompactionOutcome::skipped(error.to_string());
        }

        CompactionOutcome::Compacted {
            checkpoint: CompactionCheckpoint {
                start_seq,
                end_seq,
                summary,
                marker: format!("compaction/{}", uuid::Uuid::new_v4()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compaction_checkpoint::stream::{CompactionRole, ViewSegment};
    use crate::compaction_checkpoint::summary::SummarySections;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Scripted {
        calls: AtomicUsize,
        answer: Result<String, SummaryError>,
    }

    impl Scripted {
        fn answers(text: String) -> Arc<Self> {
            Arc::new(Self {
                calls: AtomicUsize::new(0),
                answer: Ok(text),
            })
        }

        fn fails(error: SummaryError) -> Arc<Self> {
            Arc::new(Self {
                calls: AtomicUsize::new(0),
                answer: Err(error),
            })
        }
    }

    #[async_trait::async_trait]
    impl SummaryGenerator for Scripted {
        async fn generate(&self, _request: &SummaryRequest) -> Result<String, SummaryError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.answer.clone()
        }
    }

    fn summary_with_keys(keys: &str) -> String {
        SummarySections {
            keys: keys.into(),
            ..SummarySections::default()
        }
        .render()
    }

    fn long_user(index: usize) -> CompactionMessage {
        CompactionMessage::new(
            CompactionRole::User,
            format!("REPLACED-{index:02}-{}", "x".repeat(36)),
        )
    }

    #[tokio::test]
    async fn a_failed_summary_leaves_nothing_behind() {
        let messages: Vec<_> = (0..6).map(long_user).collect();
        let engine = CompactionEngine::new(Scripted::fails(SummaryError::Unavailable(
            "provider offline".into(),
        )));
        let outcome = engine
            .compact(
                &messages,
                &[],
                &CompactionBudget::new(1_000, 0).with_retain_tail_ratio(0.02),
            )
            .await;
        assert!(!outcome.is_compacted(), "摘要失败必须不压缩: {outcome:?}");
    }

    #[tokio::test]
    async fn an_oversized_summary_is_refused() {
        let messages: Vec<_> = (0..6).map(long_user).collect();
        let oversized = summary_with_keys(&"k".repeat(2_000));
        let engine = CompactionEngine::new(Scripted::answers(oversized));
        let outcome = engine
            .compact(
                &messages,
                &[],
                &CompactionBudget::new(1_000, 0).with_retain_tail_ratio(0.02),
            )
            .await;
        assert!(!outcome.is_compacted(), "过大摘要必须拒绝替换: {outcome:?}");
    }

    #[tokio::test]
    async fn a_second_compaction_merges_the_prior_summary() {
        let messages: Vec<_> = (0..6).map(long_user).collect();
        let prior = summary_with_keys("FACT-ONE-MUST-SURVIVE");
        let log = vec![
            CompactionLogEntry::Start {
                marker: "compaction/1".into(),
            },
            CompactionLogEntry::Checkpoint(Box::new(CompactionCheckpoint {
                start_seq: 0,
                end_seq: 3,
                summary: prior.clone(),
                marker: "compaction/1".into(),
            })),
            CompactionLogEntry::End {
                marker: "compaction/1".into(),
            },
        ];
        // The scripted answer carries the prior key fact forward, as the merge
        // contract requires.
        let merged = summary_with_keys("FACT-ONE-MUST-SURVIVE\nFACT-TWO");
        let engine = CompactionEngine::new(Scripted::answers(merged.clone()));
        let outcome = engine
            .compact(
                &messages,
                &log,
                &CompactionBudget::new(1_000, 0).with_retain_tail_ratio(0.02),
            )
            .await;
        let CompactionOutcome::Compacted { checkpoint } = outcome else {
            panic!("合并后的摘要应当被接受: {outcome:?}");
        };
        assert!(checkpoint.summary.contains("FACT-ONE-MUST-SURVIVE"));
        assert!(checkpoint.end_seq > 3, "新检查点覆盖旧摘要所在区间");

        // The fold replaces the old summary wholesale: only the new one stands.
        let mut extended = log;
        extended.extend([
            CompactionLogEntry::Start {
                marker: checkpoint.marker.clone(),
            },
            CompactionLogEntry::Checkpoint(Box::new(checkpoint.clone())),
            CompactionLogEntry::End {
                marker: checkpoint.marker.clone(),
            },
        ]);
        let folded = fold_checkpoints(messages.len(), &extended);
        let standing: Vec<_> = folded
            .segments
            .iter()
            .filter_map(|segment| match segment {
                ViewSegment::Summary { summary, .. } => Some(summary.clone()),
                ViewSegment::Original { .. } => None,
            })
            .collect();
        assert_eq!(standing, vec![checkpoint.summary], "旧摘要必须被新摘要顶掉");
    }
}
