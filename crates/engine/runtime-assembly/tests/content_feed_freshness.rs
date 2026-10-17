//! J2 拉取→摄取全链装配级证明 (`tool.content_feed` → 记忆保鲜回路)。
//!
//! 用真 [`SqliteMemoryStore`] 的 `enqueue_ingest` 作摄取端口 + 真 episode 写入面,
//! 走真 [`ContentFeedTool`] 生产工具面: 拉取条目摘要投保鲜摄取队列 → 泵 → 增量
//! 折入; **同条目二次拉取零新记忆** (条目账去重) + **散列门**内容级去重 (即使
//! 条目账丢失, 同源同内容仍不重复摄取)。回环 mock 端点, 不需外网。

use std::sync::Arc;

use apeireth_core::Episode;
use apeireth_memory::freshness::EnqueueOutcome;
use apeireth_memory::{EpisodeQuery, EpisodeStore, SqliteMemoryStore};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::{
    ingest_port, ContentFeedConfig, ContentFeedTool, ControlledEgress, EgressAllowList,
    EgressPolicy, EpisodeWriter, IngestOutcome, IngestPort, IngestRequest,
};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// ---------------------------------------------------------------------------
// mock 端点 (回环服务器, 可服务多次)
// ---------------------------------------------------------------------------

async fn write_http(socket: &mut TcpStream, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/rss+xml; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
        body.len()
    );
    socket.write_all(head.as_bytes()).await.unwrap();
    socket.write_all(body).await.unwrap();
}

async fn read_request_head(socket: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 512];
    loop {
        let n = socket.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_ascii_lowercase()
}

const RSS_TWO: &[u8] = br#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>Daily</title>
<item><title>First headline</title><link>https://example.com/1</link>
<description>Body one with details.</description>
<pubDate>Wed, 02 Oct 2024 07:00:00 GMT</pubDate></item>
<item><title>Second headline</title><description>Body two.</description></item>
</channel></rss>"#;

async fn serve_rss(times: usize) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        for _ in 0..times {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let _ = read_request_head(&mut socket).await;
            write_http(&mut socket, RSS_TWO).await;
        }
    });
    (port, handle)
}

// ---------------------------------------------------------------------------
// 接缝适配: SqliteMemoryStore::enqueue_ingest → IngestPort (只读消费)
// ---------------------------------------------------------------------------

fn enqueue_port(store: Arc<SqliteMemoryStore>) -> IngestPort {
    ingest_port(move |req: &IngestRequest| {
        match store.enqueue_ingest(&req.source, &req.topic, &req.content, req.now_ms) {
            Ok(EnqueueOutcome::Enqueued { task_id }) => IngestOutcome::Enqueued { task_id },
            Ok(EnqueueOutcome::DuplicateSkipped { skip_count, .. }) => {
                IngestOutcome::DuplicateSkipped { skip_count }
            }
            Ok(EnqueueOutcome::AlreadyQueued { task_id }) => {
                IngestOutcome::AlreadyQueued { task_id }
            }
            Err(error) => IngestOutcome::Failed {
                error: error.to_string(),
            },
        }
    })
}

fn episode_writer(store: Arc<SqliteMemoryStore>) -> EpisodeWriter {
    Arc::new(move |episode: &Episode| {
        EpisodeStore::put_episode(&*store, episode).map_err(|e| e.to_string())
    })
}

fn loopback_egress() -> ControlledEgress {
    let list = EgressAllowList::new().allow("127.0.0.1", None);
    ControlledEgress::new(EgressPolicy::ExplicitAllowList(list))
        .with_timeout(std::time::Duration::from_secs(5))
        .with_max_response_bytes(64 * 1024)
}

fn tool_with(
    dir: &std::path::Path,
    memory: Arc<SqliteMemoryStore>,
) -> (ContentFeedTool, Arc<SqliteMemoryStore>) {
    let config = ContentFeedConfig::new(
        dir,
        Arc::new(loopback_egress()),
        episode_writer(Arc::clone(&memory)),
    )
    .with_ingest_port(enqueue_port(Arc::clone(&memory)));
    (ContentFeedTool::new(config), memory)
}

fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_feed".into(),
        name: "content_feed".into(),
        arguments,
    }
}

fn value_of(result: ToolResult) -> Value {
    match result.outcome {
        ToolOutcome::Ok { value } => value,
        ToolOutcome::Error { message, .. } => panic!("expected ok outcome, got error: {message}"),
    }
}

async fn add_feed(tool: &ContentFeedTool, id: &str, url: String) {
    value_of(
        tool.invoke(&call(json!({
            "action": "feed_add", "id": id, "url": url, "kind": "rss"
        })))
        .await,
    );
}

fn episode_count(memory: &SqliteMemoryStore) -> usize {
    EpisodeStore::query(
        memory,
        &EpisodeQuery::new().for_session("content_feed".to_string()),
    )
    .expect("query")
    .len()
}

// ---------------------------------------------------------------------------
// 拉取→摄取全链 + 去重
// ---------------------------------------------------------------------------

/// 拉取→摄取全链: 条目摘要投保鲜摄取队列 → 泵 → 增量折入 (真存储)。
#[tokio::test]
async fn pulled_content_lands_in_the_freshness_loop() {
    let (port, server) = serve_rss(1).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let memory = Arc::new(SqliteMemoryStore::open_in_memory().expect("memory"));
    let (tool, memory) = tool_with(dir.path(), memory);
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    let pulled = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(pulled["ingested"], 2);

    // 投摄取: 两条待摄取进队列 (真 enqueue_ingest)。
    assert_eq!(memory.pending_ingests().expect("pending").len(), 2);

    // 泵两条 → 增量折入 (同主题追加)。
    for _ in 0..2 {
        let out = memory.pump_next_ingest(1_700_000_000_000).expect("pump");
        assert!(
            matches!(out, apeireth_memory::freshness::PumpOutcome::Folded { .. }),
            "{out:?}"
        );
    }
    let status = memory.freshness_status().expect("freshness");
    assert_eq!(status.pending_ingest, 0, "泵后无待摄取");
    assert!(status.last_fold_at_ms.is_some(), "已折入记真值");
    assert_eq!(status.dedup_skipped, 0, "首轮无去重");
    assert_eq!(
        memory.topic_segments("news").expect("segments").len(),
        2,
        "两条摘要折入同主题"
    );
    // J1 原料面照旧: episodic 流两条。
    assert_eq!(episode_count(&memory), 2);
}

/// 同条目二次拉取零新记忆 (条目账去重): episode / 折入段都不长。
#[tokio::test]
async fn repeated_pull_of_the_same_items_yields_zero_new_memories() {
    let (port, server) = serve_rss(2).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let memory = Arc::new(SqliteMemoryStore::open_in_memory().expect("memory"));
    let (tool, memory) = tool_with(dir.path(), memory);
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    // 第一次: 2 条入记忆 + 投摄取 + 折入。
    value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    while matches!(
        memory.pump_next_ingest(1_700_000_000_000).expect("pump"),
        apeireth_memory::freshness::PumpOutcome::Folded { .. }
    ) {}
    let episodes_first = episode_count(&memory);
    let segments_first = memory.topic_segments("news").expect("segments").len();
    assert_eq!(episodes_first, 2);
    assert_eq!(segments_first, 2);

    // 同条目二次拉取: 条目账去重 → 零新 episode + 零新折入 + 零新摄取。
    let second = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(second["ingested"], 0);
    assert_eq!(second["deduped"], 2);
    assert_eq!(episode_count(&memory), episodes_first, "零新 episode");
    assert_eq!(
        memory.topic_segments("news").expect("segments").len(),
        segments_first,
        "零新折入段"
    );
    assert_eq!(
        memory.pending_ingests().expect("pending").len(),
        0,
        "零新摄取"
    );
}

/// 散列门内容级去重 (隔离出条目账): 同源同内容即使条目账丢失也不重复摄取。
#[tokio::test]
async fn same_content_is_deduped_by_the_hash_gate_when_the_item_ledger_is_reset() {
    let (port, server) = serve_rss(2).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let memory = Arc::new(SqliteMemoryStore::open_in_memory().expect("memory"));
    let (tool, memory) = tool_with(dir.path(), memory);
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    // 第一次: 2 条投保鲜回路 + 泵折入 (散列账记摄入)。
    value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    while matches!(
        memory.pump_next_ingest(1_700_000_000_000).expect("pump"),
        apeireth_memory::freshness::PumpOutcome::Folded { .. }
    ) {}
    assert_eq!(memory.freshness_status().expect("s").dedup_skipped, 0);

    // 人为清空内容流条目账 (隔离出散列门): 内容流不再按条目短路。
    std::fs::remove_file(dir.path().join("feed_lifecycle.json")).expect("reset item ledger");

    // 同内容再拉: 走到 enqueue_ingest → 散列门按 (源,内容) 去重 → 不重复折入。
    let second = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(second["ingested"], 2, "条目账已清, episode 面重写 (预期)");
    let status = memory.freshness_status().expect("s");
    assert_eq!(status.dedup_skipped, 2, "散列门跳过两条同源同内容");
    assert_eq!(status.pending_ingest, 0, "零新摄取");
    assert_eq!(
        memory.topic_segments("news").expect("segments").len(),
        2,
        "散列门兜底, 折入段不长"
    );
}
