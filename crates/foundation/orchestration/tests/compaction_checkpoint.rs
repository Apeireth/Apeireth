//! Compress checkpoints: deterministic selection, replacement, and replay.
//!
//! Everything here is pure logic over pure inputs; the injected summary
//! generator is scripted so the accept-or-refuse contract is exercised without
//! any model call. The runtime wiring lives in the engine crate's tests.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_orchestration::compaction_checkpoint::{
    compaction_due, detect_unclosed_compactions, fold_checkpoints, render_transcript,
    select_compaction_range, validate_summary, CompactionBudget, CompactionCheckpoint,
    CompactionEngine, CompactionLogEntry, CompactionMessage, CompactionOutcome, CompactionRange,
    CompactionRole, SummaryError, SummaryGenerator, SummaryRequest, SummarySections, ViewSegment,
};
use apeireth_orchestration::context_overflow::{
    exceeds_trigger, retain_tail_tokens, trigger_threshold_tokens, DEFAULT_RETAIN_TAIL_RATIO,
    RESERVE_TOKENS,
};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn user(text: &str) -> CompactionMessage {
    CompactionMessage::new(CompactionRole::User, text)
}

/// A message of exactly `chars` visible characters (so `chars / 4` token
/// estimates are exact).
fn sized(index: usize, chars: usize) -> CompactionMessage {
    let label = format!("m{index:02}");
    user(&format!("{label}{}", "x".repeat(chars - label.len())))
}

fn call(id: &str) -> CompactionMessage {
    CompactionMessage::new(CompactionRole::Assistant, "").with_tool_calls(vec![id.into()])
}

fn result(id: &str) -> CompactionMessage {
    CompactionMessage::new(CompactionRole::Tool, format!("result-{id}")).with_tool_result_id(id)
}

fn summary_with_keys(keys: &str) -> String {
    SummarySections {
        keys: keys.into(),
        ..SummarySections::default()
    }
    .render()
}

/// A scripted summary generator: counts its calls, records the requests it
/// received, and answers a fixed result (or a fixed failure).
struct Scripted {
    calls: AtomicUsize,
    seen: Mutex<Vec<SummaryRequest>>,
    answer: Result<String, SummaryError>,
}

impl Scripted {
    fn answers(text: String) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
            answer: Ok(text),
        })
    }

    fn fails(error: SummaryError) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
            answer: Err(error),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn request(&self, index: usize) -> SummaryRequest {
        self.seen.lock().unwrap()[index].clone()
    }
}

#[async_trait::async_trait]
impl SummaryGenerator for Scripted {
    async fn generate(&self, request: &SummaryRequest) -> Result<String, SummaryError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(request.clone());
        self.answer.clone()
    }
}

fn closed(
    marker: &str,
    start_seq: usize,
    end_seq: usize,
    summary: &str,
) -> Vec<CompactionLogEntry> {
    vec![
        CompactionLogEntry::Start {
            marker: marker.into(),
        },
        CompactionLogEntry::Checkpoint(Box::new(CompactionCheckpoint {
            start_seq,
            end_seq,
            summary: summary.into(),
            marker: marker.into(),
        })),
        CompactionLogEntry::End {
            marker: marker.into(),
        },
    ]
}

/// The scripted budget used by engine tests: 6 messages of 10 tokens against a
/// 20-token tail share (2 messages fit), so the compressed span is `[0, 4)`.
fn engine_budget() -> CompactionBudget {
    CompactionBudget::new(1_000, 0).with_retain_tail_ratio(0.02)
}

// ---------------------------------------------------------------------------
// 选段边界
// ---------------------------------------------------------------------------

/// The retained tail is exactly the configured `retain_tail` share: whatever
/// fits the budget stays verbatim, the rest is the compressed span.
#[test]
fn selection_keeps_the_tail_within_the_retain_share() {
    let messages: Vec<_> = (0..10).map(|i| sized(i, 40)).collect();

    // 25 tokens of tail budget: exactly two 10-token messages fit.
    let range = select_compaction_range(&messages, 25).expect("compressible span");
    assert_eq!(
        range,
        CompactionRange {
            start_seq: 0,
            end_seq: 8
        }
    );

    // Zero tail budget: only the newest message is exempt.
    let range = select_compaction_range(&messages, 0).expect("compressible span");
    assert_eq!(
        range,
        CompactionRange {
            start_seq: 0,
            end_seq: 9
        }
    );

    // A budget that fits the whole transcript: nothing to compress.
    assert!(select_compaction_range(&messages, 1_000).is_none());
}

/// Persistent system messages are never inside the compressed span, whatever
/// the tail budget says.
#[test]
fn selection_never_compresses_the_leading_system_run() {
    let mut messages = vec![
        CompactionMessage::new(CompactionRole::System, "identity"),
        CompactionMessage::new(CompactionRole::System, "safety"),
    ];
    messages.extend((0..6).map(|i| sized(i, 40)));

    let range = select_compaction_range(&messages, 0).expect("compressible span");
    assert_eq!(range.start_seq, 2, "system 段永远留在视图里");
    assert_eq!(range.end_seq, 7, "最新一条永远逐字保留");
}

// ---------------------------------------------------------------------------
// 工具配对平衡切分
// ---------------------------------------------------------------------------

/// The cut retreats to a pair-safe boundary: it may never land between a tool
/// call and its tool result. Swept over every tail budget, so every possible
/// greedy cut is exercised.
#[test]
fn the_cut_never_lands_between_a_tool_call_and_its_result() {
    let messages = vec![
        user("m0"),
        sized(1, 40),
        call("a"),
        result("a"),
        call("b"),
        result("b"),
        sized(6, 40),
        call("c"),
        result("c"),
        user("m9"),
    ];
    for retain in 0..60u64 {
        let Some(CompactionRange { start_seq, end_seq }) =
            select_compaction_range(&messages, retain)
        else {
            continue;
        };
        for index in start_seq..end_seq {
            for call_id in &messages[index].tool_call_ids {
                for tail in end_seq..messages.len() {
                    let answers =
                        messages[tail].tool_result_id.as_deref() == Some(call_id.as_str());
                    assert!(
                        !answers,
                        "retain={retain} 时切点 {end_seq} 劈开了 {call_id} 的调用/结果对"
                    );
                }
            }
            // And the mirror direction: no result in the span answers a call in
            // the retained tail.
            if let Some(answered) = &messages[index].tool_result_id {
                for tail in end_seq..messages.len() {
                    let issues = messages[tail]
                        .tool_call_ids
                        .iter()
                        .any(|call_id| call_id == answered);
                    assert!(
                        !issues,
                        "retain={retain} 时切点 {end_seq} 劈开了 {answered}"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 替换与回放确定性
// ---------------------------------------------------------------------------

/// Folding the same event stream twice produces the same view: replay is
/// deterministic and the summary replaces exactly its span.
#[test]
fn replacement_and_replay_fold_deterministically() {
    let messages: Vec<_> = (0..8).map(|i| sized(i, 40)).collect();
    let log = closed("compaction/1", 0, 5, &summary_with_keys("the key fact"));

    let first = fold_checkpoints(messages.len(), &log);
    let second = fold_checkpoints(messages.len(), &log);
    assert_eq!(first, second, "同一事件流折叠两次必须完全一致");

    assert_eq!(
        first.segments,
        vec![
            ViewSegment::Summary {
                start_seq: 0,
                end_seq: 5,
                summary: summary_with_keys("the key fact"),
                marker: "compaction/1".into(),
            },
            ViewSegment::Original {
                start_seq: 5,
                end_seq: 8
            },
        ]
    );
    assert_eq!(first.applied_markers, vec!["compaction/1"]);
    // The originals are archived: the fold replaced the surface, not the
    // transcript.
    assert_eq!(render_transcript(&messages).lines().count(), 8);
}

/// A session without checkpoints derives exactly its own transcript.
#[test]
fn a_normal_session_view_equals_its_transcript() {
    let messages: Vec<_> = (0..5).map(|i| sized(i, 40)).collect();
    let folded = fold_checkpoints(messages.len(), &[]);
    assert_eq!(
        folded.segments,
        vec![ViewSegment::Original {
            start_seq: 0,
            end_seq: 5
        }]
    );
    assert!(folded.applied_markers.is_empty());
    assert!(folded.unclosed_markers.is_empty());
}

// ---------------------------------------------------------------------------
// 标记对闭合
// ---------------------------------------------------------------------------

/// A crash between the opening entry and the closing entry leaves an unclosed
/// marker pair: it is detectable, and its checkpoint is never applied. Closing
/// the pair later (the durable write completing) applies it.
#[test]
fn an_unclosed_marker_pair_is_detected_after_a_crash() {
    let messages: Vec<_> = (0..6).map(|i| sized(i, 40)).collect();
    let crashed = vec![
        CompactionLogEntry::Start {
            marker: "compaction/7".into(),
        },
        CompactionLogEntry::Checkpoint(Box::new(CompactionCheckpoint {
            start_seq: 0,
            end_seq: 4,
            summary: summary_with_keys("half written"),
            marker: "compaction/7".into(),
        })),
        // The closing entry never made it to disk.
    ];

    assert_eq!(
        detect_unclosed_compactions(&crashed),
        vec!["compaction/7"],
        "崩溃留下的未闭合锁必须可检出"
    );
    let folded = fold_checkpoints(messages.len(), &crashed);
    assert_eq!(
        folded.segments,
        vec![ViewSegment::Original {
            start_seq: 0,
            end_seq: 6
        }],
        "未闭合的检查点不得改变视图"
    );

    let mut repaired = crashed;
    repaired.push(CompactionLogEntry::End {
        marker: "compaction/7".into(),
    });
    assert!(detect_unclosed_compactions(&repaired).is_empty());
    let folded = fold_checkpoints(messages.len(), &repaired);
    assert!(folded.applied_markers.contains(&"compaction/7".to_string()));
}

// ---------------------------------------------------------------------------
// 摘要更小校验
// ---------------------------------------------------------------------------

/// A summary that is not strictly smaller than what it replaces is refused —
/// nothing replaces anything.
#[tokio::test]
async fn an_oversized_summary_is_refused_and_nothing_replaces() {
    let big = summary_with_keys(&"k".repeat(400));
    assert!(matches!(
        validate_summary(&big, "tiny span", &[]),
        Err(SummaryError::NotSmaller { .. })
    ));

    // Through the engine: same refusal, no checkpoint.
    let messages: Vec<_> = (0..6).map(|i| sized(i, 40)).collect();
    let engine = CompactionEngine::new(Scripted::answers(big));
    let outcome = engine.compact(&messages, &[], &engine_budget()).await;
    assert!(!outcome.is_compacted(), "过大摘要必须拒绝替换: {outcome:?}");
}

// ---------------------------------------------------------------------------
// 旧摘要合并
// ---------------------------------------------------------------------------

/// The summary request carries the prior summary so it can be merged; the
/// merged summary keeps the prior's key facts; the fold then supersedes the old
/// summary wholesale — only the merged one stands.
#[tokio::test]
async fn a_prior_summary_is_merged_into_the_next_one() {
    let messages: Vec<_> = (0..6).map(|i| sized(i, 40)).collect();
    let prior = summary_with_keys("FACT-ONE-MUST-SURVIVE");
    let mut log = closed("compaction/1", 0, 3, &prior);

    let merged = summary_with_keys("FACT-ONE-MUST-SURVIVE\nFACT-TWO");
    let scripted = Scripted::answers(merged.clone());
    let generator: Arc<dyn SummaryGenerator> = scripted.clone();
    let engine = CompactionEngine::new(generator);
    let outcome = engine.compact(&messages, &log, &engine_budget()).await;

    // 摘要素材 = 真实对话前缀 + 旧摘要并入。
    assert_eq!(scripted.call_count(), 1, "一次压缩只调一次摘要");
    let request = scripted.request(0);
    assert_eq!(request.prior_summaries, vec![prior.clone()]);
    assert!(request.transcript.contains("m00"), "素材是真实对话前缀");
    assert!(
        !request.transcript.contains("m05"),
        "逐字保留的尾部不进素材"
    );
    assert!(
        request.render_prompt().contains("FACT-ONE-MUST-SURVIVE"),
        "旧摘要必须出现在摘要请求里以便并入"
    );

    let CompactionOutcome::Compacted { checkpoint } = outcome else {
        panic!("合并后的摘要应当被接受: {outcome:?}");
    };
    assert!(checkpoint.summary.contains("FACT-ONE-MUST-SURVIVE"));
    assert!(checkpoint.end_seq > 3, "新检查点须覆盖旧摘要所在区间");

    // The fold supersedes the old summary: one standing summary, the merged one.
    log.extend(closed(
        &checkpoint.marker,
        checkpoint.start_seq,
        checkpoint.end_seq,
        &checkpoint.summary,
    ));
    let folded = fold_checkpoints(messages.len(), &log);
    let standing: Vec<_> = folded
        .segments
        .iter()
        .filter_map(|segment| match segment {
            ViewSegment::Summary { summary, .. } => Some(summary.clone()),
            ViewSegment::Original { .. } => None,
        })
        .collect();
    assert_eq!(standing, vec![checkpoint.summary]);
}

/// A new summary that drops a prior summary's key facts is refused outright.
#[test]
fn a_summary_dropping_prior_key_facts_is_refused() {
    let prior = summary_with_keys("FACT-ONE-MUST-SURVIVE");
    let dropped = summary_with_keys("something else entirely");
    assert_eq!(
        validate_summary(&dropped, "replaced material span", &[prior]),
        Err(SummaryError::PriorKeyFactsDropped)
    );
}

// ---------------------------------------------------------------------------
// 摘要失败零破坏
// ---------------------------------------------------------------------------

/// A summary provider that is unavailable or failing produces no compaction at
/// all: no checkpoint, no event, no change.
#[tokio::test]
async fn a_failed_summary_attempt_changes_nothing() {
    let messages: Vec<_> = (0..6).map(|i| sized(i, 40)).collect();
    for failure in [
        SummaryError::Unavailable("provider offline".into()),
        SummaryError::Failed("bad response".into()),
    ] {
        let scripted = Scripted::fails(failure);
        let generator: Arc<dyn SummaryGenerator> = scripted.clone();
        let engine = CompactionEngine::new(generator);
        let outcome = engine.compact(&messages, &[], &engine_budget()).await;
        assert!(!outcome.is_compacted(), "摘要失败必须不压缩: {outcome:?}");
        assert_eq!(scripted.call_count(), 1);
        // The transcript is untouched: replay of the (empty) log is the
        // transcript itself.
        assert_eq!(
            fold_checkpoints(messages.len(), &[]).segments,
            vec![ViewSegment::Original {
                start_seq: 0,
                end_seq: 6
            }]
        );
    }
}

// ---------------------------------------------------------------------------
// 超窗触发接入
// ---------------------------------------------------------------------------

/// The compaction trigger is the shared overflow trigger math: same threshold,
/// same exclusive boundary.
#[test]
fn the_trigger_gates_on_the_shared_overflow_math() {
    let (window, overhead) = (128_000u64, 8_000u64);
    let trigger = trigger_threshold_tokens(window, overhead, RESERVE_TOKENS);
    assert!(!compaction_due(trigger, window, overhead, RESERVE_TOKENS));
    assert!(compaction_due(
        trigger + 1,
        window,
        overhead,
        RESERVE_TOKENS
    ));
    assert_eq!(
        compaction_due(1, window, overhead, RESERVE_TOKENS),
        exceeds_trigger(1, window, overhead, RESERVE_TOKENS)
    );

    let budget = CompactionBudget::new(window, overhead);
    assert_eq!(budget.trigger_threshold(), trigger);
    assert_eq!(
        budget.retain_tail(),
        retain_tail_tokens(window, overhead, DEFAULT_RETAIN_TAIL_RATIO)
    );
    assert!(!budget.is_due(trigger));
    assert!(budget.is_due(trigger + 1));
}

// ---------------------------------------------------------------------------
// checkpoint 记录往返
// ---------------------------------------------------------------------------

/// Checkpoint records survive a serialization round trip unchanged — the same
/// JSON shape a durable session store writes and reads back.
#[test]
fn checkpoint_records_round_trip_through_json() {
    let log = closed("compaction/3", 2, 9, &summary_with_keys("kept"));
    let json = serde_json::to_string(&log).unwrap();
    let back: Vec<CompactionLogEntry> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, log);
    assert_eq!(
        fold_checkpoints(12, &back),
        fold_checkpoints(12, &log),
        "落盘往返后的折叠结果必须一致"
    );
}
