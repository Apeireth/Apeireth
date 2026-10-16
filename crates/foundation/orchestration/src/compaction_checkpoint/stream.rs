//! The append-only compaction log and the deterministic derived-view fold.
//!
//! A session transcript is never rewritten. A compaction records one
//! [`CompactionCheckpoint`] — a *surface replacement*: the derived provider view
//! swaps the checkpoint's `[start_seq, end_seq)` span for its summary, while the
//! original messages stay in the transcript and remain replayable.
//!
//! Every checkpoint write is bracketed by a marker pair in the log
//! (`start -> checkpoint -> end`). The pair is a crash-detectable lock: a
//! marker whose closing entry never arrived leaves an unclosed pair that
//! [`detect_unclosed_compactions`] reports and [`fold_checkpoints`] refuses to
//! apply. Replay therefore only ever applies fully closed checkpoints, and the
//! same log always folds to the same view.

use serde::{Deserialize, Serialize};

/// Role classes the compaction planner distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionRole {
    /// Persistent system instruction.
    System,
    /// User turn.
    User,
    /// Assistant turn, possibly carrying tool calls.
    Assistant,
    /// Tool result answering one tool call.
    Tool,
}

/// The minimal transcript shape the compaction planner needs: role class, text,
/// and tool call / tool result correlation. The runtime maps its own message
/// type onto this shape; nothing here depends on the wire-level message model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionMessage {
    /// Role class of the message.
    pub role: CompactionRole,
    /// Visible text of the message (joined content in the runtime mapping).
    pub text: String,
    /// Tool call ids this message issues (assistant tool-call batches).
    pub tool_call_ids: Vec<String>,
    /// Tool call id this message answers (tool results).
    pub tool_result_id: Option<String>,
}

impl CompactionMessage {
    /// A message with no tool correlation.
    pub fn new(role: CompactionRole, text: impl Into<String>) -> Self {
        Self {
            role,
            text: text.into(),
            tool_call_ids: Vec::new(),
            tool_result_id: None,
        }
    }

    /// Attach the tool call ids an assistant message issues.
    #[must_use]
    pub fn with_tool_calls(mut self, ids: Vec<String>) -> Self {
        self.tool_call_ids = ids;
        self
    }

    /// Attach the tool call id a tool result answers.
    #[must_use]
    pub fn with_tool_result_id(mut self, id: impl Into<String>) -> Self {
        self.tool_result_id = Some(id.into());
        self
    }

    /// Estimated token cost of the visible text (the shared `chars / 4`
    /// estimate), floored at one so a retained tail cannot absorb an unbounded
    /// number of empty messages for free.
    pub fn estimated_tokens(&self) -> u64 {
        crate::context_fold::approx_tokens(&self.text).max(1) as u64
    }
}

/// One compaction action: the summary that surface-replaces the original span.
///
/// Sequence numbers index the append-only message stream and are therefore
/// stable for the lifetime of the session: replaying the fold at any later time
/// resolves the same span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompactionCheckpoint {
    /// First original message index covered by the summary (inclusive).
    pub start_seq: usize,
    /// First original message index *not* covered by the summary (exclusive).
    pub end_seq: usize,
    /// The summary that replaces the span in the derived view.
    pub summary: String,
    /// Marker identity shared by the bracketing pair of log entries.
    pub marker: String,
}

/// One append-only compaction log entry.
///
/// The intended write order for one compaction is
/// `Start -> Checkpoint -> End`; anything else is either a rejected entry or an
/// unclosed marker pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "entry", rename_all = "snake_case")]
pub enum CompactionLogEntry {
    /// Opens the marker lock for one compaction write.
    Start {
        /// Marker identity of the lock.
        marker: String,
    },
    /// The checkpoint itself.
    Checkpoint(Box<CompactionCheckpoint>),
    /// Closes the marker lock.
    End {
        /// Marker identity of the lock.
        marker: String,
    },
}

/// One segment of the derived view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewSegment {
    /// Original messages `[start_seq, end_seq)` survive verbatim.
    Original {
        /// First covered original index (inclusive).
        start_seq: usize,
        /// First uncovered original index (exclusive).
        end_seq: usize,
    },
    /// A closed checkpoint summary stands in for `[start_seq, end_seq)`.
    Summary {
        /// First replaced original index (inclusive).
        start_seq: usize,
        /// First unreplaced original index (exclusive).
        end_seq: usize,
        /// The standing summary.
        summary: String,
        /// Marker of the checkpoint that produced it.
        marker: String,
    },
}

/// The deterministic result of folding one compaction log over one transcript.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FoldedView {
    /// View segments in stream order.
    pub segments: Vec<ViewSegment>,
    /// Markers whose pair never closed; their checkpoints are not applied.
    pub unclosed_markers: Vec<String>,
    /// Entries the fold refused, with a legible reason each.
    pub rejected: Vec<String>,
    /// Markers of checkpoints the fold applied, in application order.
    pub applied_markers: Vec<String>,
}

/// One in-flight marker pair while scanning the log.
struct OpenMarker {
    marker: String,
    staged: Option<CompactionCheckpoint>,
}

/// What one log scan found.
struct LogScan {
    /// Marker pairs that never closed, in first-seen order.
    unclosed_markers: Vec<String>,
    /// Entries the scan refused, with a legible reason each.
    rejected: Vec<String>,
    /// Checkpoints whose marker pair closed, in closing order.
    closed: Vec<CompactionCheckpoint>,
}

/// Scan the log once: close marker pairs, stage their checkpoints, and note
/// every anomaly. Shared by [`detect_unclosed_compactions`] and
/// [`fold_checkpoints`] so the two reads can never disagree.
fn scan_log(log: &[CompactionLogEntry]) -> LogScan {
    let mut scan = LogScan {
        unclosed_markers: Vec::new(),
        rejected: Vec::new(),
        closed: Vec::new(),
    };
    let mut open: Vec<OpenMarker> = Vec::new();
    for entry in log {
        match entry {
            CompactionLogEntry::Start { marker } => {
                if open.iter().any(|slot| slot.marker == *marker) {
                    scan.rejected
                        .push(format!("marker {marker} opened twice without closing"));
                } else {
                    open.push(OpenMarker {
                        marker: marker.clone(),
                        staged: None,
                    });
                }
            }
            CompactionLogEntry::Checkpoint(checkpoint) => {
                match open
                    .iter_mut()
                    .find(|slot| slot.marker == checkpoint.marker)
                {
                    Some(slot) if slot.staged.is_none() => {
                        slot.staged = Some((**checkpoint).clone());
                    }
                    Some(slot) => scan.rejected.push(format!(
                        "marker {} staged two checkpoints before closing",
                        slot.marker
                    )),
                    None => scan.rejected.push(format!(
                        "checkpoint {} has no open marker pair",
                        checkpoint.marker
                    )),
                }
            }
            CompactionLogEntry::End { marker } => {
                let closing = open
                    .iter()
                    .position(|slot| slot.marker == *marker)
                    .map(|position| open.remove(position));
                match closing {
                    Some(slot) => match slot.staged {
                        Some(checkpoint) => scan.closed.push(checkpoint),
                        None => scan
                            .rejected
                            .push(format!("marker {marker} closed without a checkpoint")),
                    },
                    None => scan
                        .rejected
                        .push(format!("marker {marker} closed without opening")),
                }
            }
        }
    }
    for slot in open {
        scan.unclosed_markers.push(slot.marker.clone());
        if slot.staged.is_some() {
            scan.rejected.push(format!(
                "marker {} never closed; its checkpoint is not applied",
                slot.marker
            ));
        }
    }
    scan
}

/// Markers whose compaction write never closed.
///
/// This is the crash-detection read: a process that died between the lock
/// opening and the lock closing leaves its marker here forever, and its
/// checkpoint is never folded into any view.
pub fn detect_unclosed_compactions(log: &[CompactionLogEntry]) -> Vec<String> {
    scan_log(log).unclosed_markers
}

/// Fold one compaction log over a transcript of `message_count` messages.
///
/// The fold is a pure function of its inputs: the same log folded twice yields
/// the same view. Only checkpoints whose marker pair fully closed are applied.
/// A closed checkpoint replaces every segment its span fully covers, trims a
/// partially covered original segment to its uncovered remainder, and is
/// rejected when it would half-replace a standing summary (a span can only
/// supersede a summary wholesale, which is also how an older summary gets
/// merged into the newer one).
pub fn fold_checkpoints(message_count: usize, log: &[CompactionLogEntry]) -> FoldedView {
    let scan = scan_log(log);
    let mut folded = FoldedView {
        segments: Vec::new(),
        unclosed_markers: scan.unclosed_markers,
        rejected: scan.rejected,
        applied_markers: Vec::new(),
    };
    let mut segments = if message_count == 0 {
        Vec::new()
    } else {
        vec![ViewSegment::Original {
            start_seq: 0,
            end_seq: message_count,
        }]
    };
    for checkpoint in scan.closed {
        match apply_checkpoint(&mut segments, &checkpoint, message_count) {
            Ok(()) => folded.applied_markers.push(checkpoint.marker),
            Err(reason) => folded.rejected.push(reason),
        }
    }
    folded.segments = segments;
    folded
}

/// Apply one closed checkpoint to the current segment list.
fn apply_checkpoint(
    segments: &mut Vec<ViewSegment>,
    checkpoint: &CompactionCheckpoint,
    message_count: usize,
) -> Result<(), String> {
    if checkpoint.start_seq >= checkpoint.end_seq {
        return Err(format!(
            "checkpoint {} covers an empty span [{}, {})",
            checkpoint.marker, checkpoint.start_seq, checkpoint.end_seq
        ));
    }
    if checkpoint.end_seq > message_count {
        return Err(format!(
            "checkpoint {} reaches past the transcript ({} > {message_count})",
            checkpoint.marker, checkpoint.end_seq
        ));
    }
    let range = checkpoint.start_seq..checkpoint.end_seq;

    // Precondition: a standing summary can only be superseded wholesale.
    for segment in segments.iter() {
        let ViewSegment::Summary { marker, .. } = segment else {
            continue;
        };
        if overlaps_partially(segment_span(segment), &range) {
            return Err(format!(
                "checkpoint {} half-replaces standing summary {marker}; rejected",
                checkpoint.marker
            ));
        }
    }

    let mut next: Vec<ViewSegment> = Vec::with_capacity(segments.len() + 2);
    let mut inserted = false;
    for segment in segments.iter() {
        let span = segment_span(segment);
        if disjoint(span, &range) {
            if !inserted && range.end <= span.0 {
                next.push(summary_segment(checkpoint));
                inserted = true;
            }
            next.push(segment.clone());
            continue;
        }
        if range.start <= span.0 && span.1 <= range.end {
            // Fully covered: the segment (original span or standing summary) is
            // superseded by the new summary.
            if !inserted {
                next.push(summary_segment(checkpoint));
                inserted = true;
            }
            continue;
        }
        // Partial overlap with an original segment: keep the uncovered parts.
        let ViewSegment::Original { start_seq, end_seq } = segment else {
            unreachable!("partially covered summaries are rejected above");
        };
        if *start_seq < range.start {
            next.push(ViewSegment::Original {
                start_seq: *start_seq,
                end_seq: range.start,
            });
        }
        if !inserted {
            next.push(summary_segment(checkpoint));
            inserted = true;
        }
        if range.end < *end_seq {
            next.push(ViewSegment::Original {
                start_seq: range.end,
                end_seq: *end_seq,
            });
        }
    }
    if !inserted {
        return Err(format!(
            "checkpoint {} covers no standing segment; rejected",
            checkpoint.marker
        ));
    }
    *segments = next;
    Ok(())
}

fn summary_segment(checkpoint: &CompactionCheckpoint) -> ViewSegment {
    ViewSegment::Summary {
        start_seq: checkpoint.start_seq,
        end_seq: checkpoint.end_seq,
        summary: checkpoint.summary.clone(),
        marker: checkpoint.marker.clone(),
    }
}

fn segment_span(segment: &ViewSegment) -> (usize, usize) {
    match segment {
        ViewSegment::Original { start_seq, end_seq } => (*start_seq, *end_seq),
        ViewSegment::Summary {
            start_seq, end_seq, ..
        } => (*start_seq, *end_seq),
    }
}

fn disjoint(span: (usize, usize), range: &std::ops::Range<usize>) -> bool {
    span.1 <= range.start || range.end <= span.0
}

fn overlaps_partially(span: (usize, usize), range: &std::ops::Range<usize>) -> bool {
    !disjoint(span, range) && !(range.start <= span.0 && span.1 <= range.end)
}

/// Render a message span as the summary material: the real conversation
/// prefix, in stream order, with tool correlation spelled out.
pub fn render_transcript(messages: &[CompactionMessage]) -> String {
    let mut out = String::new();
    for message in messages {
        let role = match message.role {
            CompactionRole::System => "system",
            CompactionRole::User => "user",
            CompactionRole::Assistant => "assistant",
            CompactionRole::Tool => "tool",
        };
        out.push('[');
        out.push_str(role);
        out.push_str("] ");
        out.push_str(&message.text);
        if !message.tool_call_ids.is_empty() {
            out.push_str(" (tool_calls: ");
            out.push_str(&message.tool_call_ids.join(", "));
            out.push(')');
        }
        if let Some(answered) = &message.tool_result_id {
            out.push_str(" (answers: ");
            out.push_str(answered);
            out.push(')');
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &str) -> CompactionMessage {
        CompactionMessage::new(CompactionRole::User, text)
    }

    #[test]
    fn a_transcript_without_checkpoints_folds_to_itself() {
        let messages = vec![user("a"), user("b"), user("c")];
        let folded = fold_checkpoints(messages.len(), &[]);
        assert_eq!(
            folded.segments,
            vec![ViewSegment::Original {
                start_seq: 0,
                end_seq: 3
            }]
        );
        assert!(folded.unclosed_markers.is_empty());
    }

    #[test]
    fn unclosed_marker_pairs_are_detected_and_never_applied() {
        let log = vec![
            CompactionLogEntry::Start {
                marker: "compaction/1".into(),
            },
            CompactionLogEntry::Checkpoint(Box::new(CompactionCheckpoint {
                start_seq: 0,
                end_seq: 2,
                summary: "summary".into(),
                marker: "compaction/1".into(),
            })),
            // crash here: the closing entry never arrives
        ];
        assert_eq!(detect_unclosed_compactions(&log), vec!["compaction/1"]);
        let folded = fold_checkpoints(3, &log);
        assert_eq!(
            folded.segments,
            vec![ViewSegment::Original {
                start_seq: 0,
                end_seq: 3
            }],
            "未闭合检查点不得进入视图"
        );
    }
}
