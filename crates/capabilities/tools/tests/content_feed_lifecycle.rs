//! J2 内容流生命周期集成测试 (`tool.content_feed` 拉取→摄取接缝)。
//!
//! 用进程内回环 mock 端点 + 显式白名单出口策略 (不需要外网) + **录制摄取端口**
//! (忠实模拟 `enqueue_ingest` 的入队/去重语义)。覆盖: 拉取→摄取接缝 / 同条目
//! 二次拉取零新记忆 (条目账去重) / 幂等账 / 注意力上限截断如实 / 拉取失败退避 /
//! degraded 标注 / 自述生命周期节 / 摄取端口故障即帧。

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::memory::Episode;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::content_feed::ingest::{
    ingest_port, AttentionLimits, IngestOutcome, IngestPort, IngestRequest,
};
use apeireth_tools_canonical::content_feed::lifecycle::LifecyclePolicy;
use apeireth_tools_canonical::{
    render_snapshot, ContentFeedConfig, ContentFeedStatus, ContentFeedTool, ControlledEgress,
    EgressAllowList, EgressPolicy, EpisodeWriter, SelfStatusSnapshot, StatusProbe,
};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

// ---------------------------------------------------------------------------
// mock 端点 (回环服务器, 可服务多次)
// ---------------------------------------------------------------------------

async fn write_http(socket: &mut TcpStream, status: &str, headers: &[(&str, &str)], body: &[u8]) {
    let mut head = format!(
        "HTTP/1.1 {status}\r\ncontent-length: {}\r\nconnection: close\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
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

/// 服务 `times` 次同款 RSS 响应 (每个拉取一次连接)。
async fn serve_rss(times: usize) -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        for _ in 0..times {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let _ = read_request_head(&mut socket).await;
            write_http(
                &mut socket,
                "200 OK",
                &[("content-type", "application/rss+xml; charset=utf-8")],
                RSS_TWO,
            )
            .await;
        }
    });
    (port, handle)
}

/// 接受连接但永不响应 (受控通道超时 = 拉取失败真因)。
async fn serve_silent_forever() -> (u16, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let _ = read_request_head(&mut socket).await;
            tokio::time::sleep(Duration::from_secs(5)).await;
            drop(socket);
        }
    });
    (port, handle)
}

// ---------------------------------------------------------------------------
// 录制摄取端口 (忠实模拟 enqueue_ingest: 同源同内容去重 / 入队)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct RecordingIngest {
    requests: Mutex<Vec<IngestRequest>>,
    seen: Mutex<BTreeSet<(String, String)>>,
}

impl RecordingIngest {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn port(self: &Arc<Self>) -> IngestPort {
        let this = Arc::clone(self);
        ingest_port(move |req: &IngestRequest| {
            this.requests.lock().unwrap().push(req.clone());
            let key = (req.source.clone(), req.content.clone());
            let mut seen = this.seen.lock().unwrap();
            if seen.contains(&key) {
                IngestOutcome::DuplicateSkipped { skip_count: 1 }
            } else {
                seen.insert(key);
                IngestOutcome::Enqueued {
                    task_id: format!("task-{}", seen.len()),
                }
            }
        })
    }

    fn requests(&self) -> Vec<IngestRequest> {
        self.requests.lock().unwrap().clone()
    }
}

/// 可配置为端口自身故障的摄取端口。
fn failing_port() -> IngestPort {
    ingest_port(|_req: &IngestRequest| IngestOutcome::Failed {
        error: "ingest backend down".to_string(),
    })
}

// ---------------------------------------------------------------------------
// 装配辅助
// ---------------------------------------------------------------------------

#[derive(Default)]
struct RecordingWriter {
    episodes: Mutex<Vec<Episode>>,
}

impl RecordingWriter {
    fn writer(self: &Arc<Self>) -> EpisodeWriter {
        let sink = Arc::clone(self);
        Arc::new(move |episode: &Episode| {
            sink.episodes.lock().unwrap().push(episode.clone());
            Ok(())
        })
    }

    fn episodes(&self) -> Vec<Episode> {
        self.episodes.lock().unwrap().clone()
    }
}

fn allowlist_egress(timeout: Duration) -> ControlledEgress {
    let list = EgressAllowList::new().allow("127.0.0.1", None);
    ControlledEgress::new(EgressPolicy::ExplicitAllowList(list))
        .with_timeout(timeout)
        .with_max_response_bytes(64 * 1024)
}

fn tool_with_port(
    dir: &std::path::Path,
    port: IngestPort,
    attention: AttentionLimits,
    lifecycle: LifecyclePolicy,
) -> (ContentFeedTool, Arc<RecordingWriter>) {
    tool_with_timeout(dir, port, attention, lifecycle, Duration::from_secs(5))
}

fn tool_with_timeout(
    dir: &std::path::Path,
    port: IngestPort,
    attention: AttentionLimits,
    lifecycle: LifecyclePolicy,
    timeout: Duration,
) -> (ContentFeedTool, Arc<RecordingWriter>) {
    let sink = Arc::new(RecordingWriter::default());
    let config = ContentFeedConfig::new(dir, Arc::new(allowlist_egress(timeout)), sink.writer())
        .with_ingest_port(port)
        .with_attention(attention)
        .with_lifecycle_policy(lifecycle);
    (ContentFeedTool::new(config), sink)
}

fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_feed_2".into(),
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

fn error_of(result: ToolResult) -> String {
    match result.outcome {
        ToolOutcome::Error { message, .. } => message,
        ToolOutcome::Ok { value } => panic!("expected error outcome, got ok: {value}"),
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

// ---------------------------------------------------------------------------
// 拉取→摄取接缝
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pull_feeds_item_summaries_into_the_ingest_port() {
    let (port, server) = serve_rss(1).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    let (tool, sink) = tool_with_port(
        dir.path(),
        ingest.port(),
        AttentionLimits::default(),
        LifecyclePolicy::default(),
    );
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    let result = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();

    assert_eq!(result["ingested"], 2);
    let requests = ingest.requests();
    assert_eq!(requests.len(), 2, "每条摘要投一次摄取");
    // source = 订阅 id + 条目身份 (拉取→摄取接缝的稳定去重键)。
    assert_eq!(
        requests[0].source,
        "content_feed:news:link:https://example.com/1"
    );
    assert!(requests[1].source.starts_with("content_feed:news:hash:"));
    assert_eq!(requests[0].topic, "news", "折入主题 = 订阅 id");
    // 摄取内容是稳定摘要 (含标题/正文), 不含拉取时刻。
    assert!(requests[0].content.contains("First headline"));
    assert!(requests[0].content.contains("Body one with details."));
    assert_eq!(sink.episodes().len(), 2, "J1 原料面照旧入 episodic 流");
}

#[tokio::test]
async fn repeated_pull_of_the_same_items_yields_zero_new_memories() {
    let (port, server) = serve_rss(2).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    let (tool, sink) = tool_with_port(
        dir.path(),
        ingest.port(),
        AttentionLimits::default(),
        LifecyclePolicy::default(),
    );
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    let first = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    assert_eq!(first["ingested"], 2);
    assert_eq!(first["deduped"], 0);
    let episodes_after_first = sink.episodes().len();
    let requests_after_first = ingest.requests().len();
    assert_eq!(episodes_after_first, 2);
    assert_eq!(requests_after_first, 2);

    // 同条目二次拉取: 条目账去重 → 零新 episode + 零新摄取。
    let second = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(second["ingested"], 0, "二次拉取零新记忆");
    assert_eq!(second["deduped"], 2, "两条都按条目账去重");
    assert_eq!(sink.episodes().len(), episodes_after_first, "零新 episode");
    assert_eq!(ingest.requests().len(), requests_after_first, "零新摄取");
}

// ---------------------------------------------------------------------------
// 幂等账 + 注意力纪律
// ---------------------------------------------------------------------------

#[tokio::test]
async fn item_ledger_is_idempotent_and_survives_a_reopen() {
    let (port, server) = serve_rss(2).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    {
        let (tool, sink) = tool_with_port(
            dir.path(),
            ingest.port(),
            AttentionLimits::default(),
            LifecyclePolicy::default(),
        );
        add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;
        value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
        assert_eq!(sink.episodes().len(), 2);
    }

    // 重开 (同数据目录, 订阅已落盘指向同一端点): 条目账仍在 → 条目级幂等。
    let ingest2 = RecordingIngest::new();
    let (tool2, sink2) = tool_with_port(
        dir.path(),
        ingest2.port(),
        AttentionLimits::default(),
        LifecyclePolicy::default(),
    );
    let second = value_of(tool2.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(second["deduped"], 2, "重开后条目账仍去重");
    assert!(sink2.episodes().is_empty(), "幂等账跨进程重开有效");
}

#[tokio::test]
async fn attention_per_pull_cap_truncates_and_labels_honestly() {
    let (port, server) = serve_rss(1).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    let (tool, sink) = tool_with_port(
        dir.path(),
        ingest.port(),
        AttentionLimits::default().with_per_pull_items(1),
        LifecyclePolicy::default(),
    );
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    let result = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(result["ingested"], 1, "只摄单次上限条数");
    assert_eq!(result["truncated"], 1, "超限截断如实标注");
    assert_eq!(result["feeds"][0]["truncated"], 1);
    assert_eq!(sink.episodes().len(), 1, "被截断条目不入记忆");
}

#[tokio::test]
async fn attention_daily_cap_truncates_and_labels_honestly() {
    let (port, server) = serve_rss(1).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    let (tool, sink) = tool_with_port(
        dir.path(),
        ingest.port(),
        AttentionLimits::default().with_daily_ingests(1),
        LifecyclePolicy::default(),
    );
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    let result = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(result["ingested"], 1, "只摄每日上限条数");
    assert_eq!(result["truncated"], 1, "每日上限截断如实");
    assert_eq!(sink.episodes().len(), 1);
}

// ---------------------------------------------------------------------------
// 失败退避 + 健康度
// ---------------------------------------------------------------------------

#[tokio::test]
async fn pull_failure_backs_off_the_next_pull() {
    let (port, server) = serve_silent_forever().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    let (tool, sink) = tool_with_timeout(
        dir.path(),
        ingest.port(),
        AttentionLimits::default(),
        LifecyclePolicy::default(),
        Duration::from_millis(200),
    );
    add_feed(&tool, "slow", format!("http://127.0.0.1:{port}/slow")).await;

    // 第一次拉取失败 (超时真因)。
    let failed = error_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    assert!(failed.contains("timed out"), "{failed}");
    assert!(sink.episodes().is_empty());

    // 立刻再拉: 该订阅在退避窗口内 → 如实标 backing_off, 不锤失败源。
    let second = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    assert_eq!(second["backing_off"], 1, "退避窗口内不拉");
    assert_eq!(second["ingested"], 0);
    let status = second["feeds"][0]["status"].as_str().unwrap();
    assert!(status.starts_with("backing_off"), "{status}");
    server.abort();
}

#[tokio::test]
async fn consecutive_failures_mark_the_feed_degraded() {
    let (port, server) = serve_silent_forever().await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    // 退避基量 0 = 失败后立即可再拉 (让连续失败能攒到阈值)。
    let lifecycle = LifecyclePolicy {
        backoff_base_ms: 0,
        ..LifecyclePolicy::default()
    };
    let (tool, sink) = tool_with_timeout(
        dir.path(),
        ingest.port(),
        AttentionLimits::default(),
        lifecycle,
        Duration::from_millis(200),
    );
    add_feed(&tool, "slow", format!("http://127.0.0.1:{port}/slow")).await;

    for _ in 0..3 {
        let _ = error_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    }
    assert!(sink.episodes().is_empty());

    // 连续 3 次失败 → 如实标 degraded (生命周期账真值)。
    let health = tool.store().lifecycle().load().expect("lifecycle");
    assert!(health.feeds["slow"].degraded, "连续失败达阈值标 degraded");
    assert_eq!(health.feeds["slow"].consecutive_failures, 3);
    assert!(health.feeds["slow"].last_error.is_some(), "失败真因留账");

    // 自述订阅健康: degraded 计数如实。
    let status = tool.store().status().expect("status");
    assert_eq!(status.degraded_feeds, Some(1));
    server.abort();
}

// ---------------------------------------------------------------------------
// 自述生命周期节 + 摄取端口故障即帧
// ---------------------------------------------------------------------------

#[tokio::test]
async fn self_status_content_feed_section_reports_lifecycle_stats() {
    let (port, server) = serve_rss(2).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    let (tool, _sink) = tool_with_port(
        dir.path(),
        ingest.port(),
        AttentionLimits::default(),
        LifecyclePolicy::default(),
    );
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    // 第一次拉 2 条; 第二次去重 2 条。
    value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    let second = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(second["deduped"], 2);

    // 自述内容流节: 去重数 / 投摄取数 / 注意力截断 / 订阅健康 如实。
    let store = tool.store().clone();
    let probe: StatusProbe<ContentFeedStatus> = Arc::new(move || store.status());
    let mut snapshot = SelfStatusSnapshot::minimal(Default::default(), Vec::new());
    snapshot.content_feed = probe().ok();
    let value = render_snapshot(&snapshot);
    assert_eq!(value["content_feed"]["dedup_skipped"], 2);
    assert_eq!(value["content_feed"]["ingest_queued"], 2);
    assert_eq!(value["content_feed"]["attention_truncated"], 0);
    assert_eq!(value["content_feed"]["degraded_feeds"], 0);
    assert_eq!(value["content_feed"]["backing_off_feeds"], 0);
}

#[tokio::test]
async fn ingest_port_failure_is_a_hard_frame_not_a_silent_drop() {
    let (port, server) = serve_rss(1).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let (tool, _sink) = tool_with_port(
        dir.path(),
        failing_port(),
        AttentionLimits::default(),
        LifecyclePolicy::default(),
    );
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    // 摄取端口故障 = 错误即帧 (真因, 不静默丢内容)。
    let message = error_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    assert!(message.contains("ingest port failed"), "{message}");
    assert!(message.contains("ingest backend down"), "{message}");
    server.abort();
}
