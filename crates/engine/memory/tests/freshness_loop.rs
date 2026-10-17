//! V4-b 记忆保鲜回路集成测试: 三件套串起来的硬证。
//!
//! 覆盖: ①散列增量门 (同内容跳过 / 变内容重摄) 端到端; ②可恢复摄取队列
//! (**进程重启续摄** / 失败留队列); ③增量折入 (同主题追加不重写); ④真值面
//! (待摄取数 / 最近折入 / 去重计数, 尚未折入 = 显式 null 不编时间)。

use apeireth_memory::freshness::{EnqueueOutcome, IngestState, PumpOutcome};
use apeireth_memory::SqliteMemoryStore;

/// 管线端到端: 摄取请求 → 散列门 → 队列 → 折入 → 自述真值。
#[test]
fn freshness_pipeline_runs_end_to_end() {
    let store = SqliteMemoryStore::open_in_memory().expect("store");

    // 摄取请求: 放行入队 → 泵折入 → 主题视图见内容。
    let outcome = store
        .enqueue_ingest("feed:news", "主题甲", "第一段材料", 1_000)
        .expect("enqueue");
    assert!(matches!(outcome, EnqueueOutcome::Enqueued { .. }));
    assert_eq!(
        store.freshness_status().expect("status").pending_ingest,
        1,
        "待摄取数真值"
    );

    match store.pump_next_ingest(1_500).expect("pump") {
        PumpOutcome::Folded { topic, seq, .. } => {
            assert_eq!(topic, "主题甲");
            assert_eq!(seq, 1);
        }
        other => panic!("必须折入: {other:?}"),
    }
    assert_eq!(
        store.topic_text("主题甲").expect("text"),
        "第一段材料",
        "折入后主题视图可见"
    );

    // 同源同散列再来 = 散列门跳过 (省算力防重复记忆), 去重计数 +1。
    let repeat = store
        .enqueue_ingest("feed:news", "主题甲", "第一段材料", 2_000)
        .expect("repeat");
    assert!(matches!(repeat, EnqueueOutcome::DuplicateSkipped { .. }));
    let status = store.freshness_status().expect("status");
    assert_eq!(status.pending_ingest, 0);
    assert_eq!(status.dedup_skipped, 1, "去重计数真值");
    assert_eq!(status.last_fold_at_ms, Some(1_500), "最近折入真值");

    // 变内容重摄: 新散列放行, 折入成为第二段 (追加不重写)。
    let changed = store
        .enqueue_ingest("feed:news", "主题甲", "第二段材料", 3_000)
        .expect("changed");
    assert!(matches!(changed, EnqueueOutcome::Enqueued { .. }));
    assert!(matches!(
        store.pump_next_ingest(3_500).expect("pump"),
        PumpOutcome::Folded { seq: 2, .. }
    ));
    let segments = store.topic_segments("主题甲").expect("segments");
    assert_eq!(segments[0].content, "第一段材料", "旧段字节原样");
    assert_eq!(segments[1].content, "第二段材料", "新内容追加");
    assert_eq!(
        store.freshness_status().expect("status").last_fold_at_ms,
        Some(3_500)
    );
}

/// 可恢复摄取队列: **进程重启续摄** —— 待摄取清单落库, 重开接着摄。
#[test]
fn ingest_queue_resumes_after_restart() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("cognitive.sqlite3");

    {
        let store = SqliteMemoryStore::open(&path).expect("open");
        store
            .enqueue_ingest("src-a", "主题甲", "素材一", 1_000)
            .expect("enqueue 1");
        store
            .enqueue_ingest("src-a", "主题甲", "素材二", 2_000)
            .expect("enqueue 2");
        // 只摄一条就「崩溃」(丢弃句柄), 剩下的必须在重启后还在。
        assert!(matches!(
            store.pump_next_ingest(2_500).expect("pump"),
            PumpOutcome::Folded { .. }
        ));
    }

    let reopened = SqliteMemoryStore::open(&path).expect("reopen");
    let pending = reopened.pending_ingests().expect("pending");
    assert_eq!(pending.len(), 1, "重启后待摄取清单还在 (续摄)");
    assert_eq!(pending[0].content, "素材二");
    assert_eq!(
        reopened.freshness_status().expect("status").pending_ingest,
        1
    );

    // 续摄: 泵把剩下的收场。
    assert!(matches!(
        reopened.pump_next_ingest(3_000).expect("pump"),
        PumpOutcome::Folded { .. }
    ));
    assert_eq!(
        reopened.topic_text("主题甲").expect("text"),
        "素材一\n素材二",
        "两次摄取按序折入同一主题"
    );
    assert_eq!(reopened.pending_ingests().expect("pending").len(), 0);
}

/// 失败留队列跨重启: 尝试计数与原因都是落库真值, 不是内存态。
#[test]
fn failed_ingest_stays_queued_across_restart_with_attempt_count() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("cognitive.sqlite3");
    let task_id = {
        let store = SqliteMemoryStore::open(&path).expect("open");
        let outcome = store
            .enqueue_ingest("src-a", "主题甲", "难搞的素材", 1_000)
            .expect("enqueue");
        let task_id = match outcome {
            EnqueueOutcome::Enqueued { task_id } => task_id,
            other => panic!("必须新入队: {other:?}"),
        };
        let failed = store
            .pump_next_ingest_with(1_500, |_| Err("下游暂不可用".to_string()))
            .expect("pump");
        assert!(matches!(failed, PumpOutcome::Failed { attempts: 1, .. }));
        task_id
    };

    let reopened = SqliteMemoryStore::open(&path).expect("reopen");
    let pending = reopened.pending_ingests().expect("pending");
    assert_eq!(pending.len(), 1, "失败必须留队列");
    assert_eq!(pending[0].state, IngestState::Pending);
    assert_eq!(pending[0].attempts, 1, "尝试计数落库");
    assert_eq!(pending[0].last_error.as_deref(), Some("下游暂不可用"));

    // 重试成功后收场, 尝试计数累计 (成败都计)。
    assert!(matches!(
        reopened.pump_next_ingest(2_000).expect("pump"),
        PumpOutcome::Folded { .. }
    ));
    let done = reopened
        .ingest_task(&task_id)
        .expect("task")
        .expect("present");
    assert_eq!(done.state, IngestState::Done);
    assert_eq!(done.attempts, 2);
}

/// 增量折入跨重启追加不重写: 重启后旧段字节原样, 新段继续追加。
#[test]
fn incremental_fold_appends_across_restart_without_rewrite() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("cognitive.sqlite3");
    {
        let store = SqliteMemoryStore::open(&path).expect("open");
        store
            .fold_topic("主题甲", "第一版内容", 1_000)
            .expect("fold");
    }
    let reopened = SqliteMemoryStore::open(&path).expect("reopen");
    let report = reopened
        .fold_topic("主题甲", "补充内容", 2_000)
        .expect("fold");
    assert_eq!(report.seq, 2, "同主题续接编号");
    let segments = reopened.topic_segments("主题甲").expect("segments");
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].content, "第一版内容", "旧段字节原样 (不重写)");
    assert_eq!(segments[1].content, "补充内容");
}

/// 自述节真值口径: 尚未折入 = 显式 null (不编时间), 待摄取/去重计数如实 0。
#[test]
fn freshness_status_reports_truth_or_explicit_null() {
    let store = SqliteMemoryStore::open_in_memory().expect("store");
    let fresh = store.freshness_status().expect("status");
    assert_eq!(fresh.pending_ingest, 0);
    assert_eq!(fresh.dedup_skipped, 0);
    assert_eq!(
        fresh.last_fold_at_ms, None,
        "尚未折入 = 显式 null, 不编时间"
    );

    store
        .enqueue_ingest("src-a", "主题甲", "素材", 1_000)
        .expect("enqueue");
    store
        .ingest_gate_mark_ingested("src-b", "重复素材", 1_000)
        .expect("mark");
    store
        .ingest_gate_mark_skipped("src-b", "重复素材", 1_100)
        .expect("skip");
    let mixed = store.freshness_status().expect("status");
    assert_eq!(mixed.pending_ingest, 1, "待摄取数真值");
    assert_eq!(mixed.dedup_skipped, 1, "去重计数真值");
    assert_eq!(mixed.last_fold_at_ms, None, "仍未折入");
}
