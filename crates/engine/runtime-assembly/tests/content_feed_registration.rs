//! J1 内容流接入的装配级证明 (`tool.content_feed`)。
//!
//! 走真 [`ProductionModules`] 生产组装根: 注册条件 / 能力名册行 / 自述内容流节
//! (订阅数·最近拉取) 对账真值; 入记忆流用真 [`SqliteMemoryStore`] 落 episodic
//! 流 (与 `EpisodeSink` 同一 append-only 写入面, 同一条流可召回), 原料**未保护级**
//! (无保护标记, 可正常整理/遗忘), 内容经 L2 封套披露 (来源标注 + 边界隔离)。

use std::sync::Arc;

use apeireth_core::kernel::Clock;
use apeireth_memory::{EpisodeQuery, EpisodeStore, MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome};
use apeireth_runtime_assembly::{ProductionBackends, ProductionModules, ProductionModulesConfig};
use apeireth_tools_canonical::{
    ContentFeedConfig, ContentFeedStatus, ControlledEgress, EgressAllowList, EgressPolicy,
    FeedStore, StatusProbe, CONTENT_FEED_CAPABILITY_ID,
};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn clock() -> Arc<dyn Clock> {
    apeireth_core::kernel::system_clock()
}

fn base_config() -> ProductionModulesConfig {
    ProductionModulesConfig {
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        ..ProductionModulesConfig::default()
    }
}

fn loopback_egress() -> ControlledEgress {
    let list = EgressAllowList::new().allow("127.0.0.1", None);
    ControlledEgress::new(EgressPolicy::ExplicitAllowList(list))
        .with_timeout(std::time::Duration::from_secs(5))
        .with_max_response_bytes(64 * 1024)
}

/// 记忆流写入面: 与感知链 `impl EpisodeSink for SqliteMemoryStore` 委托的同一
/// [`EpisodeStore::put_episode`] 写入面 (同源: 同一 append-only episode 流)。
fn episode_writer(store: Arc<SqliteMemoryStore>) -> apeireth_tools_canonical::EpisodeWriter {
    Arc::new(move |episode| EpisodeStore::put_episode(&*store, episode).map_err(|e| e.to_string()))
}

async fn serve_rss(port_out: &mut u16) -> tokio::task::JoinHandle<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    *port_out = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 512];
        let _ = socket.read(&mut buf).await;
        let body = br#"<?xml version="1.0"?>
<rss version="2.0"><channel>
<item><title>Headline from the world</title>
<description>Raw material body.</description>
<pubDate>Wed, 02 Oct 2024 07:00:00 GMT</pubDate></item>
</channel></rss>"#;
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/rss+xml; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        let _ = socket.write_all(head.as_bytes()).await;
        let _ = socket.write_all(body).await;
    })
}

async fn invoke(tool: &Arc<dyn ToolCapability>, arguments: Value) -> Value {
    let call = ToolCall {
        id: "call_feed".into(),
        name: "content_feed".into(),
        arguments,
    };
    let result = tool.invoke(&call).await;
    match result.outcome {
        ToolOutcome::Ok { value } => value,
        ToolOutcome::Error { message, .. } => panic!("expected ok, got error: {message}"),
    }
}

/// 注册条件 + 能力名册行: 配置注入才注册, 名册照实报生效值。
#[tokio::test]
async fn content_feed_registers_only_when_configured_and_the_roster_says_so() {
    let mut off = base_config();
    off.content_feed = None;
    let modules =
        ProductionModules::build(off, ProductionBackends::default(), clock()).expect("build");
    assert!(modules
        .capabilities()
        .iter()
        .all(|capability| capability.id().as_str() != CONTENT_FEED_CAPABILITY_ID));

    let dir = tempfile::tempdir().expect("tempdir");
    let sink = Arc::new(SqliteMemoryStore::open_in_memory().expect("memory"));
    let config = ContentFeedConfig::new(
        dir.path(),
        Arc::new(loopback_egress()),
        episode_writer(sink),
    );
    let mut on = base_config();
    on.content_feed = Some(config);
    let modules =
        ProductionModules::build(on, ProductionBackends::default(), clock()).expect("build");
    let ids: Vec<String> = modules
        .capabilities()
        .iter()
        .map(|capability| capability.id().to_string())
        .collect();
    assert!(
        ids.iter().any(|id| id == CONTENT_FEED_CAPABILITY_ID),
        "missing {CONTENT_FEED_CAPABILITY_ID}: {ids:?}"
    );

    // 自述能力名册一行 (真实生效值): 与注册条件同源。
    let self_status = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status");
    let call = ToolCall {
        id: "call_status".into(),
        name: "self_status".into(),
        arguments: json!({}),
    };
    let result = self_status.invoke(&call).await;
    let report: Value = serde_json::from_str(&result.render()).expect("self report json");
    assert_eq!(report["capabilities"]["content_feed"], true);
}

/// 自述内容流节: 订阅数/最近拉取取自 `feeds.json` 真值 (如实)。
#[tokio::test]
async fn self_status_content_feed_section_reports_real_subscription_counts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = FeedStore::new(dir.path(), EgressPolicy::PublicInternetOnly);
    store
        .add_feed(apeireth_tools_canonical::FeedSource {
            id: "news".to_string(),
            url: "https://example.com/rss".to_string(),
            kind: apeireth_tools_canonical::FeedKind::Rss,
            added_at_ms: 1_700_000_000_000,
        })
        .expect("add");
    store
        .record_pull(apeireth_tools_canonical::PullRecord {
            at_ms: 1_700_000_500_000,
            ok: true,
            feeds: 1,
            ingested: 2,
            note: "1 feeds, 2 items".to_string(),
        })
        .expect("record");

    let probe_store = store.clone();
    let probe: StatusProbe<ContentFeedStatus> = Arc::new(move || probe_store.status());
    let backends = ProductionBackends {
        self_status_content_feed: Some(probe),
        ..ProductionBackends::default()
    };
    let modules = ProductionModules::build(base_config(), backends, clock()).expect("build");
    let self_status = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == "tool.self_status")
        .cloned()
        .expect("tool.self_status");
    let call = ToolCall {
        id: "call_status".into(),
        name: "self_status".into(),
        arguments: json!({}),
    };
    let result = self_status.invoke(&call).await;
    let report: Value = serde_json::from_str(&result.render()).expect("self report json");
    let section = &report["content_feed"];
    assert_eq!(section["feeds"], 1);
    assert_eq!(section["last_pull_at_ms"], 1_700_000_500_000i64);
    assert_eq!(section["last_pull_ok"], true);
    assert_eq!(section["last_pull_ingested"], 2);
    assert_eq!(section["reason"], Value::Null);
}

/// 入记忆流 (真存储): EpisodeSink 同源同流 / 来源标注 / L2 封套 / 未保护级。
#[tokio::test]
async fn pulled_content_lands_in_the_shared_episode_stream_unprotected() {
    let mut port = 0u16;
    let server = serve_rss(&mut port).await;

    let dir = tempfile::tempdir().expect("tempdir");
    let memory = Arc::new(SqliteMemoryStore::open_in_memory().expect("memory"));

    // 对照: 同一流里先写一条别的来源的 episode (共享流语义的锚)。
    EpisodeStore::put_episode(
        &*memory,
        &apeireth_core::Episode {
            id: "other-source-1".to_string(),
            timestamp: 1_700_000_000,
            role: "system".to_string(),
            content: "another source's note".to_string(),
            session_id: "content_feed".to_string(),
        },
    )
    .expect("put");

    let config = ContentFeedConfig::new(
        dir.path(),
        Arc::new(loopback_egress()),
        episode_writer(Arc::clone(&memory)),
    );
    let mut on = base_config();
    on.content_feed = Some(config);
    let modules =
        ProductionModules::build(on, ProductionBackends::default(), clock()).expect("build");
    let tool = modules
        .capabilities()
        .iter()
        .find(|capability| capability.id().as_str() == CONTENT_FEED_CAPABILITY_ID)
        .cloned()
        .expect("tool.content_feed must be registered");

    let added = invoke(
        &tool,
        json!({
            "action": "feed_add", "id": "news",
            "url": format!("http://127.0.0.1:{port}/rss"), "kind": "rss"
        }),
    )
    .await;
    assert_eq!(added["action"], "feed_add");
    let pulled = invoke(&tool, json!({ "action": "pull" })).await;
    server.await.unwrap();

    // 工具结果 = 纯元数据 (跨源内容不直进提示词)。
    assert_eq!(pulled["ingested"], 1);
    let rendered = pulled.to_string();
    assert!(
        !rendered.contains("Headline from the world") && !rendered.contains("Raw material body"),
        "拉取内容不得进工具结果: {rendered}"
    );

    // 同一条 episodic 流: 内容流原料与其它来源的 episode 一起可召回。
    let rows = EpisodeStore::query(
        &*memory,
        &EpisodeQuery::new().for_session("content_feed".to_string()),
    )
    .expect("query");
    assert_eq!(rows.len(), 2, "两条 episode 同流可召回: {rows:?}");
    let ingested_row = rows
        .iter()
        .find(|row| row.id.starts_with("content-feed-"))
        .expect("content-feed episode");
    assert!(
        ingested_row.content.starts_with("以下是只读背景材料"),
        "L2 封套披露 (固定警示头): {}",
        ingested_row.content
    );
    assert!(
        ingested_row.content.contains("content_feed:news"),
        "来源标注: {}",
        ingested_row.content
    );
    assert!(
        ingested_row.content.contains("Headline from the world"),
        "结构化摘要 (标题) 入记忆流: {}",
        ingested_row.content
    );
    assert!(ingested_row.content.contains("Raw material body"));
    assert!(ingested_row
        .content
        .contains("Wed, 02 Oct 2024 07:00:00 GMT"));

    // 未保护级: 原料不带保护标记 (可正常整理/遗忘; 保护是人工显式动作)。
    let governed = apeireth_memory::MemoryGovernanceStore::get_governed(&*memory, &ingested_row.id)
        .expect("governance row")
        .expect("governance row exists");
    assert!(!governed.protected, "内容流原料必须是未保护记忆");
    assert_eq!(
        governed.status,
        apeireth_memory::MemoryGovernanceStatus::Active
    );

    // 对照: 保护标记是真实语义 (手动保护后 protected = true)。
    apeireth_memory::MemoryGovernanceStore::protect_episode(&*memory, &ingested_row.id, 0)
        .expect("protect");
    let governed = apeireth_memory::MemoryGovernanceStore::get_governed(&*memory, &ingested_row.id)
        .expect("governance row")
        .expect("governance row exists");
    assert!(governed.protected, "保护标记必须真实可置位");
}
