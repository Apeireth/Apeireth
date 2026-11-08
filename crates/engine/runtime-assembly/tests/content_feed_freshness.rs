//! J2 拉取→摄取全链装配级证明 (`tool.content_feed` → 记忆保鲜回路)。
//!
//! 用真 [`SqliteMemoryStore`] 的 `enqueue_ingest` 作摄取端口 + 真 episode 写入面,
//! 走真 [`ContentFeedTool`] 生产工具面: 拉取条目摘要投保鲜摄取队列 → 泵 → 增量
//! 折入; **同条目二次拉取零新记忆** (条目账去重) + **散列门**内容级去重 (即使
//! 条目账丢失, 同源同内容仍不重复摄取)。回环 mock 端点, 不需外网。
//!
//! # 批次 3b-2: J1 面的凭据形状绊线 (直写 episodic 记忆的那条路径)
//!
//! 本文件后半段**新增**的用例覆盖 **J1 面**: `content_feed` 的 `episode_for_item`
//! 把同一份标题 + 正文**直写** episodic 记忆 (不经保鲜摄取队列) ⇒ ⇒ 那是**凭据
//! 进存储**的路径 (比 transcript 更「深」) ⇒ ⇒ 「episodic 记录无明文」这条验收
//! 判据对 J1 成立。
//!
//! * **J2 面** (拉取 → 摄取队列 ⇒ `stable_ingest_content` ⇒ 遮形): 由
//!   `crates/capabilities/tools/tests/content_feed_ingest_tripwire.rs` 覆盖;
//! * **J1 面** (拉取 → **直写** episode ⇒ 正文 JSON 字符串叶子遮形): 由本文件
//!   新增的用例覆盖 (查库断言) ⇒ ⇒ **两条路径都被覆盖**, 同一张「凭据进存储」
//!   的面无缺口。
//!
//! ⚠️ 判据一律**查库**: 用**独立**的 SQLite 连接直读 `episodes` 表 (不是断言
//! 内存对象) ⇒ 证明「落库的那一份」无明文。

use std::sync::Arc;

use apeireth_core::Episode;
use apeireth_memory::freshness::EnqueueOutcome;
use apeireth_memory::{EpisodeQuery, EpisodeStore, SqliteMemoryStore};
use apeireth_orchestration::untrusted_envelope::{
    UNTRUSTED_REFERENCE_BEGIN_TOKEN, UNTRUSTED_REFERENCE_END_MARKER, UNTRUSTED_REFERENCE_WARNING,
};
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

// ---------------------------------------------------------------------------
// 批次 3b-2: J1 面的凭据形状绊线 (直写 episodic 记忆的那条路径)
// ---------------------------------------------------------------------------

/// 一枚**形状合法**的密钥串 (形状夹具, 不是真凭据): `sk-` + ≥20 字节。
///
/// ⚠️ 是**形状**夹具, 无任何服务端账号与之绑定 ⇒ 可入库于测试夹具。
const FAKE_KEY: &str = "sk-j1fixturekeyabcdefghijklmnop";

/// RSS 夹具: 一条**含密钥**的条目 + 一条**干净**的条目。
///
/// ⚠️ 两条都带**条目链接** ⇒ 条目身份走 `link:` 前缀 (不走内容散列退化)。
const RSS_J1_MIXED: &[u8] = br#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>Daily</title>
<item><title>Leaky headline</title><link>https://example.com/1</link>
<description>Body with sk-j1fixturekeyabcdefghijklmnop inside.</description>
<pubDate>Wed, 02 Oct 2024 07:00:00 GMT</pubDate></item>
<item><title>Clean headline</title><link>https://example.com/2</link>
<description>Body with nothing sensitive.</description>
<pubDate>Wed, 02 Oct 2024 07:00:00 GMT</pubDate></item>
</channel></rss>"#;

/// RSS 夹具: 只有一条**干净**条目 (零命中口径)。
const RSS_J1_CLEAN: &[u8] = br#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>Daily</title>
<item><title>Clean headline</title><link>https://example.com/clean</link>
<description>Body with nothing sensitive.</description>
<pubDate>Wed, 02 Oct 2024 07:00:00 GMT</pubDate></item>
</channel></rss>"#;

/// 服务 `times` 次指定载荷的 RSS 响应 (每次拉取一条连接)。
async fn serve_rss_payload(
    times: usize,
    body: &'static [u8],
) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        for _ in 0..times {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let _ = read_request_head(&mut socket).await;
            write_http(&mut socket, body).await;
        }
    });
    (port, handle)
}

/// **查库**: 用**独立**的 SQLite 连接直读 `episodes` 表的正文列。
///
/// ⚠️ 走独立连接 (不是内存对象) ⇒ 断言的是**落库的那一份**。
fn read_episode_contents(db_path: &std::path::Path) -> Vec<String> {
    let conn = rusqlite::Connection::open(db_path).expect("独立连接");
    let mut stmt = conn
        .prepare("SELECT content FROM episodes WHERE session_id = 'content_feed' ORDER BY id ASC")
        .expect("prepare");
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query");
    rows.map(|row| row.expect("row")).collect()
}

/// 从封套正文里取出**正文 JSON** (边界标记之间的那一段)。
fn body_json_of(content: &str) -> String {
    let start = content
        .find(UNTRUSTED_REFERENCE_BEGIN_TOKEN)
        .expect("封套起始")
        + UNTRUSTED_REFERENCE_BEGIN_TOKEN.len();
    let payload_start = start + content[start..].find(">>>\n").expect("载荷起始") + 4;
    let payload_end = content
        .find(UNTRUSTED_REFERENCE_END_MARKER)
        .expect("封套结束");
    content[payload_start..payload_end].trim_end().to_string()
}

/// 工具面源文件 (跨 crate 读源码: 接线守卫用)。
fn content_feed_tool_source() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("capabilities")
        .join("tools")
        .join("src")
        .join("content_feed")
        .join("tool.rs");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("工具面源文件必须可读: {path:?}: {error}"))
}

/// 核心判据 ① + ② + ④ + ⑤: 含真 key 的 feed ⇒ **查库** ⇒ episodic 记录里
/// 无明文; 落库记录仍是**合法 JSON** 且**只换了字符串叶子**; J2 与 J1 两条路径
/// 都被覆盖; 命中**可观测**。
#[tokio::test]
async fn j1_direct_episode_write_keeps_credential_plaintext_out_of_the_database() {
    let (port, server) = serve_rss_payload(1, RSS_J1_MIXED).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("memory.db");
    let memory = Arc::new(SqliteMemoryStore::open(&db_path).expect("memory"));
    let (tool, memory) = tool_with(dir.path(), memory);
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;

    let pulled = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(pulled["ingested"], 2, "两条摘要都直写进 episodic 记忆");

    // ── 查库 (独立连接直读 SQLite 文件) ──
    let stored = read_episode_contents(&db_path);
    assert_eq!(stored.len(), 2, "两条原料都落库");
    let leaky = stored
        .iter()
        .find(|content| content.contains("Leaky headline"))
        .expect("含密钥那条");
    let clean = stored
        .iter()
        .find(|content| content.contains("Clean headline"))
        .expect("干净那条");

    // ① 真 key 不得进 episodic 存储 (那是凭据进存储的面)。
    assert!(
        !leaky.contains(FAKE_KEY),
        "真 key 不得进 episodic 存储: {leaky}"
    );
    assert!(
        leaky.contains("[REDACTED_OPENAI_KEY]"),
        "命中必须换成遮形标签: {leaky}"
    );
    assert!(!clean.contains(FAKE_KEY), "干净那条本来就没有明文: {clean}");

    // ② 结构完好: 落库正文仍是**合法 JSON**, 且**只换了字符串叶子**。
    let leaky_body = body_json_of(leaky);
    let clean_body = body_json_of(clean);
    let leaky_value: Value =
        serde_json::from_str(&leaky_body).expect("落库正文仍是合法 JSON (可反序列化)");
    let clean_value: Value = serde_json::from_str(&clean_body).expect("落库正文仍是合法 JSON");
    // 键集不变 (遮形只换字符串叶子的内容, 不动键名 / 数量 / 结构)。
    let keys = |value: &Value| {
        let mut keys: Vec<String> = value
            .as_object()
            .expect("正文是对象")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    };
    assert_eq!(
        keys(&leaky_value),
        keys(&clean_value),
        "键集不变 (遮形不得改结构)"
    );
    assert_eq!(keys(&leaky_value).len(), 9, "九个字段一个不多一个不少");
    // 非字符串叶子逐字段保持原样 (数字 / 布尔 / null / 结构不动)。
    assert!(leaky_value["item_index"].is_number(), "数字叶子仍是数字");
    assert!(
        leaky_value["body_truncated"].is_boolean(),
        "布尔叶子仍是布尔"
    );
    assert_eq!(
        leaky_value["feed_id"], clean_value["feed_id"],
        "字符串叶子不动"
    );
    assert_eq!(
        leaky_value["source_url"], clean_value["source_url"],
        "字符串叶子不动"
    );
    assert_eq!(
        leaky_value["published_raw"], clean_value["published_raw"],
        "字符串叶子不动"
    );
    assert_eq!(
        leaky_value["fetched_at_ms"], clean_value["fetched_at_ms"],
        "数字叶子不动"
    );
    assert_eq!(leaky_value["item_index"], json!(0), "数字叶子的值不动");
    assert_eq!(
        leaky_value["body_truncated"],
        json!(false),
        "布尔叶子的值不动"
    );
    // 未命中凭据的字符串叶子逐字不动 (标题 / 链接)。
    assert_eq!(
        leaky_value["title"],
        json!("Leaky headline"),
        "标题叶子逐字不动"
    );
    assert_eq!(
        leaky_value["item_link"],
        json!("https://example.com/1"),
        "链接叶子逐字不动"
    );
    // **逐字节**复核: 落库正文 = 旧布局, 只有承载凭据的那个叶子被换成标签。
    let expected_body = json!({
        "feed_id": "news",
        "source_url": format!("http://127.0.0.1:{port}/rss"),
        "item_index": 0,
        "title": "Leaky headline",
        "body_excerpt": "Body with [REDACTED_OPENAI_KEY] inside.",
        "body_truncated": false,
        "published_raw": "Wed, 02 Oct 2024 07:00:00 GMT",
        "item_link": "https://example.com/1",
        "fetched_at_ms": leaky_value["fetched_at_ms"].clone(),
    })
    .to_string();
    assert_eq!(
        leaky_body, expected_body,
        "落库正文必须逐字节等于「只把凭据叶子换成标签」的旧布局"
    );

    // ④ 两条路径都被覆盖: J2 (摄取队列) 的待摄取内容同样无明文。
    let pending = memory.pending_ingests().expect("pending");
    assert_eq!(pending.len(), 2, "两条摘要都投保鲜摄取队列");
    for task in &pending {
        assert!(
            !task.content.contains(FAKE_KEY),
            "J2 摄取面也不得含明文: {}",
            task.content
        );
    }
    let leaky_task = pending
        .iter()
        .find(|task| task.source.contains("/1"))
        .expect("含密钥那条");
    assert!(
        leaky_task.content.contains("[REDACTED_OPENAI_KEY]"),
        "J2 命中必须换成遮形标签: {}",
        leaky_task.content
    );

    // ⑤ 命中可观测: J1 的落库结构 (Episode 记录) 没有结构化侧车字段 ⇒ 侧车以
    // 末尾一行 JSON 注解挂在文本面 (同 J2 裸文本面纪律) ⇒ 命中不静默。
    let marker = leaky
        .lines()
        .find(|line| line.contains("credential_tripwire"))
        .expect("命中必须留一行 JSON 注解");
    assert!(!marker.contains(FAKE_KEY), "注解不得含凭据真值: {marker}");
    assert!(marker.contains("OpenAiKey"), "注解必须报命中族: {marker}");
}

/// 核心判据 ③: 零命中 ⇒ 落库内容**逐字节不变** (含普通正文)。
#[tokio::test]
async fn j1_clean_episode_body_is_byte_identical_in_the_database() {
    let (port, server) = serve_rss_payload(1, RSS_J1_CLEAN).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("memory.db");
    let memory = Arc::new(SqliteMemoryStore::open(&db_path).expect("memory"));
    let (tool, _memory) = tool_with(dir.path(), memory);
    add_feed(&tool, "news", format!("http://127.0.0.1:{port}/rss")).await;
    value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();

    let stored = read_episode_contents(&db_path);
    assert_eq!(stored.len(), 1, "一条原料落库");
    let content = &stored[0];

    // 独立复算「旧布局」: 键集 / 值 / 序列化字节全由测试侧构造 (拉取时刻从
    // 落库记录里读回 —— 那是唯一随运行变化的字段)。
    let body = body_json_of(content);
    let parsed: Value = serde_json::from_str(&body).expect("正文是合法 JSON");
    let legacy_body = json!({
        "feed_id": "news",
        "source_url": format!("http://127.0.0.1:{port}/rss"),
        "item_index": 0,
        "title": "Clean headline",
        "body_excerpt": "Body with nothing sensitive.",
        "body_truncated": false,
        "published_raw": "Wed, 02 Oct 2024 07:00:00 GMT",
        "item_link": "https://example.com/clean",
        "fetched_at_ms": parsed["fetched_at_ms"].clone(),
    })
    .to_string();
    assert_eq!(
        body, legacy_body,
        "零命中 ⇒ 落库正文 JSON 必须逐字节不变 (含普通正文)"
    );

    let expected = format!(
        "{UNTRUSTED_REFERENCE_WARNING}\n{UNTRUSTED_REFERENCE_BEGIN_TOKEN} source=\"content_feed:news\">>>\n{legacy_body}\n{UNTRUSTED_REFERENCE_END_MARKER}"
    );
    assert_eq!(
        content, &expected,
        "零命中 ⇒ 落库记录逐字节不变 (连封套形状都不许变)"
    );
    assert!(
        !content.contains("credential_tripwire"),
        "零命中 ⇒ 不挂任何标记 (干净答案的形状不变): {content}"
    );
}

/// 接线回归守卫: J1 原料面必须接**共享助手** (JSON 字符串叶子扫描)。
#[test]
fn the_j1_episode_face_is_wired_to_the_shared_json_leaf_scanner() {
    let source = content_feed_tool_source();
    let production = source.split("#[cfg(test)]").next().unwrap_or_default();
    assert!(
        production.contains("scan_json_string_leaves"),
        "J1 原料面必须接共享助手 scan_json_string_leaves"
    );
}

/// 命名纪律: 工具面新增的常量 / 字符串**不得**命中 `APEIRETH_[A-Z0-9_]+`
/// (那是本仓 env 开关命名空间 ⇒ `arch_guard` 的 env 棘轮会红)。
#[test]
fn the_j1_episode_face_enters_no_env_namespace_name() {
    let source = content_feed_tool_source();
    assert!(
        !source.contains("APEIRETH_"),
        "工具面不得引入 env 命名空间名 (APEIRETH_[A-Z0-9_]+)"
    );
}
