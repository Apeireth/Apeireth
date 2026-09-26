//! The session event log and the derived views folded from it.
//!
//! The log is the sole authority over a session's surface. It is append-only:
//! message appends, surface changes, and masks are recorded once and never
//! rewritten or removed — a masked or replaced original stays in the log (and
//! in the transcript) and remains queryable at any later time. What the
//! provider sees is never stored; it is *derived* by one pure function,
//! [`fold_surface`], so the same log always folds to the same view.
//!
//! Surface vocabulary is deliberately tiny. The only legal surface change is
//! [`SurfaceOp::Replace`] — truncation, compaction, and rewriting all route
//! through it: a rewrite is "replace the span with new content", and the
//! replaced originals are kept on record. A compaction checkpoint is the same
//! operation with a summary as its replacement content; the write discipline
//! that decides which checkpoints are legal is reused unchanged through
//! [`compaction_entries`] + the compaction fold, so this module restates none
//! of its rules. A [`SessionEventKind::Masked`] record hides a message from
//! derived views without touching its content.

use apeireth_core::kernel::{RequestId, SessionId, Timestamp, TraceId};
use apeireth_orchestration::compaction_checkpoint::{
    fold_checkpoints, CompactionCheckpoint, CompactionLogEntry, ViewSegment,
};
use apeireth_protocol::canonical::NormalizedMessage;
use serde::{Deserialize, Serialize};

use super::session::SessionEventKind;

/// The one legal surface change: swap the half-open span `[start_seq, end_seq)`
/// of the derived view's source numbering for new content.
///
/// Truncation, compaction, and rewriting are all this one operation — they
/// differ only in what stands in for the span afterwards (nothing, a summary,
/// or replacement messages). Sequence numbers index the append-only message
/// stream and are therefore stable for the lifetime of the session, so a fold
/// at any later time resolves the same span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceOp {
    /// Replace `[start_seq, end_seq)` with the operation's replacement
    /// content. An empty replacement truncates the span away in the view.
    Replace {
        /// First replaced message index (inclusive).
        start_seq: usize,
        /// First unreplaced message index (exclusive).
        end_seq: usize,
    },
}

impl SurfaceOp {
    /// A replacement over `[start_seq, end_seq)`.
    pub const fn replace(start_seq: usize, end_seq: usize) -> Self {
        Self::Replace { start_seq, end_seq }
    }

    /// The half-open span this op targets, as `(start_seq, end_seq)`.
    pub const fn span(&self) -> (usize, usize) {
        let Self::Replace { start_seq, end_seq } = *self;
        (start_seq, end_seq)
    }
}

/// One record in a session's append-only event log.
///
/// Records carry the same event vocabulary as the execution-facts stream
/// (`SessionEventKind`), with optional provenance: facts recorded through the
/// runtime carry the request and trace that caused them, while session-level
/// writes (appends, masks, surface changes, fork evidence) have none. The log
/// rides inside the session's own persisted record, so no storage schema
/// changes are needed for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    /// When it happened according to the runtime clock.
    pub at: Timestamp,
    /// Inbound request that caused it, when the record has execution
    /// provenance.
    pub request: Option<RequestId>,
    /// Execution trace that caused it, when the record has execution
    /// provenance.
    pub trace: Option<TraceId>,
    /// The event body.
    pub event: SessionEventKind,
}

/// Evidence, kept on the parent session, that a child session was forked from
/// it (see [`super::session::Session::fork_session`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForkRecord {
    /// The child session that was created.
    pub child_id: SessionId,
    /// Where the child's inherited prefix ended: it inherited exactly this
    /// many records of the parent's log.
    pub prefix_end_seq: usize,
}

/// Where the content of a [`SurfaceSegment::Replaced`] segment came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurfaceSource {
    /// An explicit surface-change record (`SurfaceOp::Replace`).
    SurfaceOp,
    /// A closed compaction checkpoint, identified by its marker.
    Compaction {
        /// Marker identity of the checkpoint that produced the summary.
        marker: String,
    },
}

/// One segment of a derived surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SurfaceSegment {
    /// Original messages `[start_seq, end_seq)` survive verbatim.
    Original {
        /// First covered original index (inclusive).
        start_seq: usize,
        /// First uncovered original index (exclusive).
        end_seq: usize,
    },
    /// The one legal surface change stood in for `[start_seq, end_seq)`: the
    /// span is the op's interval, and `replacement` is the new content (empty
    /// means the span is truncated away in the view).
    Replaced {
        /// The replacement interval that produced this segment.
        op: SurfaceOp,
        /// New content standing in for the span.
        replacement: Vec<NormalizedMessage>,
        /// Whether an explicit change or a compaction checkpoint produced it.
        source: SurfaceSource,
    },
    /// Masked originals `[start_seq, end_seq)`: hidden from derived views,
    /// never deleted, and still replayable from the log and the transcript.
    Masked {
        /// First masked original index (inclusive).
        start_seq: usize,
        /// First unmasked original index (exclusive).
        end_seq: usize,
    },
}

impl SurfaceSegment {
    /// The half-open span of original sequence numbers this segment covers.
    pub fn span(&self) -> (usize, usize) {
        match self {
            Self::Original { start_seq, end_seq } => (*start_seq, *end_seq),
            Self::Replaced { op, .. } => op.span(),
            Self::Masked { start_seq, end_seq } => (*start_seq, *end_seq),
        }
    }
}

/// The deterministic result of folding one event log into its derived surface.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurfaceView {
    /// Every appended original, in sequence order — including the ones masks
    /// and replacements hide from the view. The log never deletes anything.
    pub originals: Vec<NormalizedMessage>,
    /// View segments in source order.
    pub segments: Vec<SurfaceSegment>,
    /// Records the fold refused, with a legible reason each. A refused record
    /// changes nothing, exactly like a rejected compaction checkpoint.
    pub rejected: Vec<String>,
}

impl SurfaceView {
    /// Materialize the provider-facing messages: original spans copy their
    /// appended messages verbatim, replaced spans emit their replacement
    /// content, masked spans emit nothing.
    pub fn messages(&self) -> Vec<NormalizedMessage> {
        let mut view = Vec::with_capacity(self.originals.len());
        for segment in &self.segments {
            match segment {
                SurfaceSegment::Original { start_seq, end_seq } => {
                    view.extend_from_slice(&self.originals[*start_seq..*end_seq]);
                }
                SurfaceSegment::Replaced { replacement, .. } => {
                    view.extend(replacement.iter().cloned());
                }
                SurfaceSegment::Masked { .. } => {}
            }
        }
        view
    }
}

/// The compaction projection of one event log: the marker-pair entries and the
/// checkpoints they bracket, in record order.
///
/// A compaction checkpoint is one form of surface replacement — the interval
/// `[start_seq, end_seq)` with its summary as replacement content — and its
/// write discipline (the marker pair, wholesale supersession, refusal rules)
/// stays exactly as the compaction fold defines it. This mapper feeds that
/// fold verbatim so the two models cannot drift apart.
pub fn compaction_entries(log: &[LogEntry]) -> Vec<CompactionLogEntry> {
    log.iter()
        .filter_map(|entry| match &entry.event {
            SessionEventKind::CompactionStarted { marker } => Some(CompactionLogEntry::Start {
                marker: marker.clone(),
            }),
            SessionEventKind::CompactionCheckpoint {
                start_seq,
                end_seq,
                summary,
                marker,
            } => Some(CompactionLogEntry::Checkpoint(Box::new(
                CompactionCheckpoint {
                    start_seq: *start_seq,
                    end_seq: *end_seq,
                    summary: summary.clone(),
                    marker: marker.clone(),
                },
            ))),
            SessionEventKind::CompactionClosed { marker } => Some(CompactionLogEntry::End {
                marker: marker.clone(),
            }),
            _ => None,
        })
        .collect()
}

/// Fold one event log into its derived surface.
///
/// The fold is a pure function of its input: the same log folded twice — today
/// or after a restart, on any machine — yields the same view. Appended records
/// establish the sequence numbering; closed compaction checkpoints are applied
/// through the compaction fold as replacement intervals; explicit surface
/// changes and masks then apply in record order. Records the fold refuses are
/// reported in [`SurfaceView::rejected`] and change nothing.
pub fn fold_surface(log: &[LogEntry]) -> SurfaceView {
    let mut view = SurfaceView::default();
    for entry in log {
        if let SessionEventKind::MessageAppended { seq, message } = &entry.event {
            if *seq == view.originals.len() {
                view.originals.push(message.clone());
            } else {
                view.rejected.push(format!(
                    "message append {seq} arrives out of sequence (expected {}); ignored",
                    view.originals.len()
                ));
            }
        }
    }
    let count = view.originals.len();

    // Compaction checkpoints project in as replacement intervals. Which
    // checkpoints are legal is decided by the compaction fold itself; only its
    // verdict is consumed here.
    let folded = fold_checkpoints(count, &compaction_entries(log));
    view.rejected.extend(folded.rejected);
    for segment in &folded.segments {
        view.segments.push(match segment {
            ViewSegment::Original { start_seq, end_seq } => SurfaceSegment::Original {
                start_seq: *start_seq,
                end_seq: *end_seq,
            },
            ViewSegment::Summary {
                start_seq,
                end_seq,
                summary,
                marker,
            } => SurfaceSegment::Replaced {
                op: SurfaceOp::replace(*start_seq, *end_seq),
                replacement: vec![NormalizedMessage::user(summary.clone())],
                source: SurfaceSource::Compaction {
                    marker: marker.clone(),
                },
            },
        });
    }

    for entry in log {
        match &entry.event {
            SessionEventKind::SurfaceReplaced {
                op,
                replacement,
                note,
            } => apply_replace(&mut view, *op, replacement, note),
            SessionEventKind::Masked { seq, reason } => apply_mask(&mut view, *seq, reason),
            _ => {}
        }
    }
    view
}

/// Apply one explicit surface change to the current segments.
///
/// The rules mirror the compaction fold's: a span supersedes standing segments
/// wholesale and is refused when it would half-supersede standing replacement
/// content. A masked span carries no content, so a replacement may subsume it
/// freely. Both cases leave the originals in place.
fn apply_replace(
    view: &mut SurfaceView,
    op: SurfaceOp,
    replacement: &[NormalizedMessage],
    note: &str,
) {
    let (start, end) = op.span();
    let count = view.originals.len();
    if start >= end {
        view.rejected.push(format!(
            "surface replace [{start}, {end}) covers an empty span; rejected ({note})"
        ));
        return;
    }
    if end > count {
        view.rejected.push(format!(
            "surface replace [{start}, {end}) reaches past the transcript ({count}); rejected ({note})"
        ));
        return;
    }
    for segment in &view.segments {
        let (s, e) = segment_span(segment);
        if overlaps_partially(s, e, start, end)
            && matches!(segment, SurfaceSegment::Replaced { .. })
        {
            view.rejected.push(format!(
                "surface replace [{start}, {end}) half-supersedes a standing replacement; rejected ({note})"
            ));
            return;
        }
    }

    let replaced = SurfaceSegment::Replaced {
        op,
        replacement: replacement.to_vec(),
        source: SurfaceSource::SurfaceOp,
    };
    let mut next: Vec<SurfaceSegment> = Vec::with_capacity(view.segments.len() + 2);
    let mut inserted = false;
    for segment in &view.segments {
        let (s, e) = segment_span(segment);
        if e <= start || end <= s {
            if !inserted && end <= s {
                next.push(replaced.clone());
                inserted = true;
            }
            next.push(segment.clone());
            continue;
        }
        if start <= s && e <= end {
            // Fully covered: originals, masks, and standing replacements are
            // superseded wholesale.
            if !inserted {
                next.push(replaced.clone());
                inserted = true;
            }
            continue;
        }
        // Partial overlap with an original or masked span: keep the
        // uncovered parts.
        if s < start {
            next.push(trimmed(segment, s, start));
        }
        if !inserted {
            next.push(replaced.clone());
            inserted = true;
        }
        if end < e {
            next.push(trimmed(segment, end, e));
        }
    }
    if !inserted {
        view.rejected.push(format!(
            "surface replace [{start}, {end}) covers no standing segment; rejected ({note})"
        ));
        return;
    }
    view.segments = next;
}

/// Mask one appended message out of the derived view. The message itself is
/// untouched: masking hides, it never deletes.
fn apply_mask(view: &mut SurfaceView, seq: usize, reason: &str) {
    let count = view.originals.len();
    if seq >= count {
        view.rejected.push(format!(
            "mask on {seq} reaches past the transcript ({count}); rejected ({reason})"
        ));
        return;
    }
    let position = view
        .segments
        .iter()
        .position(|segment| segment_span(segment).0 <= seq && seq < segment_span(segment).1);
    let Some(position) = position else {
        view.rejected.push(format!(
            "mask on {seq} covers no standing segment; rejected ({reason})"
        ));
        return;
    };
    let segment = view.segments[position].clone();
    match segment {
        SurfaceSegment::Masked { .. } => {
            view.rejected.push(format!(
                "message {seq} is already masked; rejected ({reason})"
            ));
        }
        SurfaceSegment::Replaced { .. } => {
            view.rejected.push(format!(
                "mask on {seq} lands inside a standing replacement; rejected ({reason})"
            ));
        }
        SurfaceSegment::Original { start_seq, end_seq } => {
            let mut parts: Vec<SurfaceSegment> = Vec::with_capacity(3);
            if start_seq < seq {
                parts.push(SurfaceSegment::Original {
                    start_seq,
                    end_seq: seq,
                });
            }
            parts.push(SurfaceSegment::Masked {
                start_seq: seq,
                end_seq: seq + 1,
            });
            if seq + 1 < end_seq {
                parts.push(SurfaceSegment::Original {
                    start_seq: seq + 1,
                    end_seq,
                });
            }
            view.segments.splice(position..=position, parts);
        }
    }
}

fn segment_span(segment: &SurfaceSegment) -> (usize, usize) {
    segment.span()
}

fn trimmed(segment: &SurfaceSegment, start_seq: usize, end_seq: usize) -> SurfaceSegment {
    match segment {
        SurfaceSegment::Masked { .. } => SurfaceSegment::Masked { start_seq, end_seq },
        _ => SurfaceSegment::Original { start_seq, end_seq },
    }
}

fn overlaps_partially(s: usize, e: usize, start: usize, end: usize) -> bool {
    let disjoint = e <= start || end <= s;
    !disjoint && !(start <= s && e <= end)
}

/// Tool call ids that assistant messages issue and no tool result in the view
/// answers, in first-seen order.
///
/// Used when a session is forked mid-tool-loop: an open call would leave the
/// new session's view incomplete, so the fork closes it with a placeholder.
pub fn open_tool_calls(messages: &[NormalizedMessage]) -> Vec<String> {
    let mut issued: Vec<String> = Vec::new();
    let mut answered: Vec<String> = Vec::new();
    for message in messages {
        for call in &message.tool_calls {
            if !issued.contains(&call.id) {
                issued.push(call.id.clone());
            }
        }
        if let Some(id) = &message.tool_call_id {
            if !answered.contains(id) {
                answered.push(id.clone());
            }
        }
    }
    issued
        .into_iter()
        .filter(|id| !answered.contains(id))
        .collect()
}

/// The placeholder tool result for a call a session fork left open: it closes
/// the call/result pair so the new session's view is complete, and it says
/// plainly that the call ended at the fork point instead of inventing an
/// outcome.
pub fn fork_placeholder(tool_call_id: &str) -> NormalizedMessage {
    NormalizedMessage::tool_result(
        tool_call_id,
        None,
        format!("已分叉：会话在工具调用 {tool_call_id} 完成前分叉，该调用没有结果"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::Timestamp;
    use apeireth_protocol::canonical::ToolCall;

    fn at() -> Timestamp {
        Timestamp::from_epoch_millis(1_700_000_000_000).unwrap()
    }

    fn appended(seq: usize, text: &str) -> LogEntry {
        LogEntry {
            at: at(),
            request: None,
            trace: None,
            event: SessionEventKind::MessageAppended {
                seq,
                message: NormalizedMessage::user(text),
            },
        }
    }

    fn replaced(op: SurfaceOp, texts: &[&str]) -> LogEntry {
        LogEntry {
            at: at(),
            request: None,
            trace: None,
            event: SessionEventKind::SurfaceReplaced {
                op,
                replacement: texts.iter().map(|t| NormalizedMessage::user(*t)).collect(),
                note: "test".into(),
            },
        }
    }

    fn masked(seq: usize) -> LogEntry {
        LogEntry {
            at: at(),
            request: None,
            trace: None,
            event: SessionEventKind::Masked {
                seq,
                reason: "test".into(),
            },
        }
    }

    fn texts(view: &SurfaceView) -> Vec<String> {
        view.messages()
            .iter()
            .map(|m| apeireth_protocol::canonical::ContentPart::join_text(&m.content))
            .collect()
    }

    #[test]
    fn the_same_log_always_folds_to_the_same_view() {
        let log = vec![
            appended(0, "a"),
            appended(1, "b"),
            appended(2, "c"),
            masked(1),
            replaced(SurfaceOp::replace(2, 3), &["c2"]),
        ];
        assert_eq!(fold_surface(&log), fold_surface(&log));
        assert_eq!(texts(&fold_surface(&log)), vec!["a", "c2"]);
    }

    #[test]
    fn out_of_sequence_appends_are_refused_and_leave_the_universe_intact() {
        let log = vec![appended(0, "a"), appended(2, "c"), appended(1, "b")];
        let view = fold_surface(&log);
        assert_eq!(view.originals.len(), 2, "只有顺序正确的追加进入编号");
        assert_eq!(
            view.rejected.len(),
            1,
            "只有离序的追加 (seq 2) 被拒, 后续合序追加照常进入编号: {:?}",
            view.rejected
        );
        assert_eq!(texts(&view), vec!["a", "b"], "被拒记录不改变任何既有内容");
    }

    #[test]
    fn a_masked_original_stays_replayable() {
        let log = vec![appended(0, "a"), appended(1, "secret"), masked(1)];
        let view = fold_surface(&log);
        assert_eq!(texts(&view), vec!["a"]);
        assert_eq!(view.originals.len(), 2, "遮蔽不删除原消息");
        assert!(log.iter().any(|entry| matches!(
            &entry.event,
            SessionEventKind::MessageAppended { seq: 1, message } if apeireth_protocol::canonical::ContentPart::join_text(&message.content) == "secret"
        )));
    }

    #[test]
    fn an_empty_replacement_truncates_and_a_full_one_rewrites() {
        let log = vec![
            appended(0, "keep"),
            appended(1, "drop-1"),
            appended(2, "drop-2"),
            replaced(SurfaceOp::replace(1, 3), &[]),
        ];
        assert_eq!(texts(&fold_surface(&log)), vec!["keep"]);

        let rewritten = vec![
            appended(0, "old"),
            replaced(SurfaceOp::replace(0, 1), &["new"]),
        ];
        let view = fold_surface(&rewritten);
        assert_eq!(texts(&view), vec!["new"]);
        assert_eq!(view.originals.len(), 1, "改写留档原文");
    }

    #[test]
    fn replacement_spans_are_half_open_at_both_boundaries() {
        // `[start_seq, end_seq)` 左闭右开: 只换掉被点名的下标, 边界外一个不动。
        let log = vec![
            appended(0, "a"),
            appended(1, "b"),
            appended(2, "c"),
            appended(3, "d"),
            replaced(SurfaceOp::replace(0, 1), &["A"]),
            replaced(SurfaceOp::replace(3, 4), &["D"]),
        ];
        let view = fold_surface(&log);
        assert_eq!(texts(&view), vec!["A", "b", "c", "D"]);
        assert!(view.rejected.is_empty(), "{:?}", view.rejected);
        assert_eq!(view.originals.len(), 4, "替换不删原文");
    }

    #[test]
    fn a_replacement_trims_originals_only_at_its_boundaries() {
        let log = vec![
            appended(0, "a"),
            appended(1, "b"),
            appended(2, "c"),
            appended(3, "d"),
            replaced(SurfaceOp::replace(1, 3), &["bc"]),
        ];
        let view = fold_surface(&log);
        assert_eq!(texts(&view), vec!["a", "bc", "d"]);
        assert_eq!(
            view.segments,
            vec![
                SurfaceSegment::Original {
                    start_seq: 0,
                    end_seq: 1
                },
                SurfaceSegment::Replaced {
                    op: SurfaceOp::replace(1, 3),
                    replacement: vec![NormalizedMessage::user("bc")],
                    source: SurfaceSource::SurfaceOp,
                },
                SurfaceSegment::Original {
                    start_seq: 3,
                    end_seq: 4
                },
            ],
            "区间内被取代, 区间外原样保留"
        );
    }

    #[test]
    fn truncation_rewriting_and_compaction_all_route_through_the_one_surface_change() {
        // 截断、改写、压缩只是替换内容不同 (空 / 新内容 / 摘要),
        // 落到派生视图里全都是同一种段形状: Replaced + SurfaceOp::Replace。
        let marker = "compaction/only-op".to_string();
        let log = vec![
            appended(0, "t"),
            appended(1, "w"),
            appended(2, "k"),
            replaced(SurfaceOp::replace(0, 1), &[]),
            replaced(SurfaceOp::replace(1, 2), &["w2"]),
            LogEntry {
                at: at(),
                request: None,
                trace: None,
                event: SessionEventKind::CompactionStarted {
                    marker: marker.clone(),
                },
            },
            LogEntry {
                at: at(),
                request: None,
                trace: None,
                event: SessionEventKind::CompactionCheckpoint {
                    start_seq: 2,
                    end_seq: 3,
                    summary: "k-sum".into(),
                    marker: marker.clone(),
                },
            },
            LogEntry {
                at: at(),
                request: None,
                trace: None,
                event: SessionEventKind::CompactionClosed {
                    marker: marker.clone(),
                },
            },
        ];
        let view = fold_surface(&log);
        assert_eq!(texts(&view), vec!["w2", "k-sum"]);
        assert_eq!(view.segments.len(), 3, "{:?}", view.segments);
        for segment in &view.segments {
            match segment {
                SurfaceSegment::Replaced { op, source, .. } => {
                    let SurfaceOp::Replace { start_seq, end_seq } = *op;
                    assert_eq!(end_seq - start_seq, 1, "三路都走同一个替换区间");
                    assert!(matches!(
                        source,
                        SurfaceSource::SurfaceOp | SurfaceSource::Compaction { .. }
                    ));
                }
                other => panic!("未预料的段形状: {other:?}"),
            }
        }
    }

    #[test]
    fn a_closed_compaction_checkpoint_projects_as_one_surface_replacement() {
        // 压缩投影统一: 标记对记录原样映射给压缩折叠, 该折叠的裁决直接成为
        // 派生视图里的替换段 —— 压缩规则不被重写, 只被复用。
        let marker = "compaction/m-1".to_string();
        let mut log = vec![appended(0, "a"), appended(1, "b"), appended(2, "c")];
        log.extend([
            LogEntry {
                at: at(),
                request: None,
                trace: None,
                event: SessionEventKind::CompactionStarted {
                    marker: marker.clone(),
                },
            },
            LogEntry {
                at: at(),
                request: None,
                trace: None,
                event: SessionEventKind::CompactionCheckpoint {
                    start_seq: 0,
                    end_seq: 2,
                    summary: "ab-sum".into(),
                    marker: marker.clone(),
                },
            },
            LogEntry {
                at: at(),
                request: None,
                trace: None,
                event: SessionEventKind::CompactionClosed {
                    marker: marker.clone(),
                },
            },
        ]);

        assert_eq!(
            compaction_entries(&log),
            vec![
                CompactionLogEntry::Start {
                    marker: marker.clone()
                },
                CompactionLogEntry::Checkpoint(Box::new(CompactionCheckpoint {
                    start_seq: 0,
                    end_seq: 2,
                    summary: "ab-sum".into(),
                    marker: marker.clone(),
                })),
                CompactionLogEntry::End {
                    marker: marker.clone()
                },
            ],
            "同一份映射同时服务规划器读取与派生视图折叠"
        );

        let view = fold_surface(&log);
        assert_eq!(texts(&view), vec!["ab-sum", "c"]);
        assert_eq!(
            view.segments,
            vec![
                SurfaceSegment::Replaced {
                    op: SurfaceOp::replace(0, 2),
                    replacement: vec![NormalizedMessage::user("ab-sum")],
                    source: SurfaceSource::Compaction {
                        marker: marker.clone()
                    },
                },
                SurfaceSegment::Original {
                    start_seq: 2,
                    end_seq: 3
                },
            ]
        );
        assert_eq!(view.originals.len(), 3, "压缩也不删原文");
        assert!(view.rejected.is_empty(), "{:?}", view.rejected);
    }

    #[test]
    fn a_replacement_that_half_supersedes_a_standing_replacement_is_rejected() {
        let log = vec![
            appended(0, "a"),
            appended(1, "b"),
            appended(2, "c"),
            replaced(SurfaceOp::replace(0, 2), &["ab"]),
            replaced(SurfaceOp::replace(1, 3), &["bc"]),
        ];
        let view = fold_surface(&log);
        assert_eq!(texts(&view), vec!["ab", "c"]);
        assert_eq!(
            view.rejected.len(),
            1,
            "半取代必须被拒: {:?}",
            view.rejected
        );
    }

    #[test]
    fn a_mask_inside_a_standing_replacement_is_rejected() {
        let log = vec![
            appended(0, "a"),
            appended(1, "b"),
            replaced(SurfaceOp::replace(0, 2), &["ab"]),
            masked(1),
        ];
        let view = fold_surface(&log);
        assert_eq!(texts(&view), vec!["ab"]);
        assert_eq!(view.rejected.len(), 1);
    }

    #[test]
    fn open_tool_calls_are_the_ones_no_result_answers() {
        let call = |id: &str| ToolCall {
            id: id.into(),
            name: "shell".into(),
            arguments: serde_json::json!({}),
        };
        let messages = vec![
            NormalizedMessage::assistant_with_tool_calls("two calls", vec![call("c1"), call("c2")]),
            NormalizedMessage::tool_result("c1", None, "done"),
            NormalizedMessage::assistant_with_tool_calls("", vec![call("c2")]),
        ];
        assert_eq!(open_tool_calls(&messages), vec!["c2".to_string()]);

        let mut closed = messages;
        closed.push(fork_placeholder("c2"));
        assert!(open_tool_calls(&closed).is_empty());
    }
}
