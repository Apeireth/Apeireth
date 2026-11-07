//! 批次 3b: 记忆摄取面的**凭据形状绊线** (M1) —— `content_feed` 摄取内容入库前清扫。
//!
//! # 覆盖面
//!
//! 被测面: `crates/capabilities/tools/src/content_feed/ingest.rs`
//! (`stable_ingest_content` / `mask_ingest_content`)。
//!
//! 记忆摄取面 (J2 拉取→摄取接缝) 把**远端标题 + 正文**写进 episodic 记忆
//! ⇒ ⇒ 那是**凭据进存储**的路径 (不是进 transcript) ⇒ ⇒ 比 transcript 更「深」。
//!
//! # 核心判据 (本文件的主断言)
//!
//! ① **零命中 ⇒ 入库内容逐字节不变** ⇒ ⇒ **散列门的散列不变**
//!    (散列门按内容散列去重 ⇒ 内容不变 ⇒ 散列不变 ⇒ 同条目重复拉取零新记忆);
//! ② 命中 ⇒ 真 key **不进**摄取内容 + 命中**可观测** (裸文本面 ⇒ 末尾一行
//!    JSON 注解);
//! ③ 命中 ⇒ 遮形**确定性** ⇒ 同条目重复拉取仍产生同一散列 ⇒ 去重语义不破。
//!
//! # 诚实边界 (本批**未**覆盖的面)
//!
//! ⚠️ **J1 原料面 (直写 episode) 不在本批写域内**: `content_feed/tool.rs` 的
//! `episode_for_item` 把同一份标题 + 正文**直接**写进 episodic 记忆 (不经摄取
//! 队列) ⇒ ⇒ 该面**尚未**接绊线 ⇒ ⇒ 「episodic 记录无明文」这条验收判据
//! **只对 J2 摄取面成立**; J1 的修点与影响面见交付报告 (下一批候选: 3b-2)。
//! ⇒ ⇒ 本文件**不**对 J1 断言「无明文」(那是未实现面, 断言它等于撒谎)。

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use apeireth_core::kernel::memory::Episode;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{ToolCall, ToolOutcome, ToolResult};
use apeireth_tools_canonical::content_feed::ingest::{
    ingest_port, item_identity, item_source, stable_ingest_content, IngestOutcome, IngestPort,
    IngestRequest,
};
use apeireth_tools_canonical::content_feed::summary::FeedItemSummary;
use apeireth_tools_canonical::{
    ContentFeedConfig, ContentFeedTool, ControlledEgress, EgressAllowList, EgressPolicy,
    EpisodeWriter,
};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinHandle;

// D26 (2026-10-21): 回环 bind 走共享重试助手 (仅 Windows WSAEACCES 10013 有界
// 重试), 见 `tests/common/mod.rs` 与 `src/test_support.rs`。
mod common;

// ---------------------------------------------------------------------------
// 夹具
// ---------------------------------------------------------------------------

/// 一枚**形状合法**的密钥串 (与内核形状层同族: `sk-` + ≥20 字节)。
///
/// ⚠️ 是**形状**夹具, 不是真凭据 (无任何服务端账号与之绑定) ⇒ 可入库于测试
/// 夹具 (仓内纪律: 测试夹具里的假密钥是「形状样本」, 不是凭据)。
const FAKE_KEY: &str = "sk-testfixturekeyabcdefghijklmnopqrstuv";

/// 一枚**形状合法**的 AWS 访问键形态 (`AKIA` + 16 位大写字母数字)。
const FAKE_AWS_KEY: &str = "AKIATESTFIXTUREKEY01";

/// RSS 夹具: 一条**含密钥**的条目 + 一条**干净**的条目。
///
/// ⚠️ 两条都带**条目链接** ⇒ 条目身份走 `link:` 前缀 (不走内容散列退化) ⇒
/// 「遮形不牵动条目身份」这条断言才干净。
const RSS_MIXED: &[u8] = br#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>Daily</title>
<item><title>Leaky headline</title><link>https://example.com/1</link>
<description>Body with sk-testfixturekeyabcdefghijklmnopqrstuv inside.</description>
<pubDate>Wed, 02 Oct 2024 07:00:00 GMT</pubDate></item>
<item><title>Clean headline</title><link>https://example.com/2</link>
<description>Body with nothing sensitive.</description>
<pubDate>Wed, 02 Oct 2024 07:00:00 GMT</pubDate></item>
</channel></rss>"#;

/// 服务 `times` 次同款 RSS 响应 (每次拉取一条连接)。
async fn serve_rss(times: usize, body: &'static [u8]) -> (u16, JoinHandle<()>) {
    let listener = common::bind_loopback_ephemeral()
        .await
        .expect("bind loopback listener");
    let port = listener.local_addr().unwrap().port();
    let handle = tokio::spawn(async move {
        for _ in 0..times {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 512];
            let _ = socket.read(&mut buf).await;
            let head = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/rss+xml; charset=utf-8\r\n\
                 content-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            socket.write_all(head.as_bytes()).await.unwrap();
            socket.write_all(body).await.unwrap();
        }
    });
    (port, handle)
}

/// 录制摄取端口 (忠实模拟 `enqueue_ingest`: 同源同内容去重 / 入队)。
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

fn allowlist_egress(timeout: Duration) -> ControlledEgress {
    let list = EgressAllowList::new().allow("127.0.0.1", None);
    ControlledEgress::new(EgressPolicy::ExplicitAllowList(list))
        .with_timeout(timeout)
        .with_max_response_bytes(64 * 1024)
}

fn call(arguments: Value) -> ToolCall {
    ToolCall {
        id: "call_feed_3b".into(),
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

/// 摘要夹具 (直调摄取内容构造函数, 不经网络)。
fn summary(title: &str, body: &str, link: Option<&str>) -> FeedItemSummary {
    FeedItemSummary {
        feed_id: "news".to_string(),
        source_url: "https://example.com/rss".to_string(),
        item_index: 0,
        title: title.to_string(),
        body_excerpt: body.to_string(),
        body_truncated: false,
        published_raw: Some("Wed, 02 Oct 2024 07:00:00 GMT".to_string()),
        item_link: link.map(str::to_string),
        fetched_at_ms: 1_700_000_000_000,
    }
}

/// FNV-1a 64 (与记忆保鲜回路散列门同口径; 自含, 不引依赖) —— 测试侧**独立**
/// 复算一遍, 不复用被测实现的私有散列函数。
fn fnv1a64(input: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

// ---------------------------------------------------------------------------
// ① 零命中 ⇒ 逐字节不变 ⇒ 散列不变
// ---------------------------------------------------------------------------

/// 核心判据: **零命中 ⇒ 入库内容逐字节不变 ⇒ 散列门的散列不变**。
///
/// 判据来源: 摄取内容构造函数的既有合同 (「来源稳定字段 ⇒ 同条目重复拉取产生
/// 同散列 ⇒ 散列门跳过」) ⇒ ⇒ 遮形面必须对**干净内容零行为变更**, 否则
/// 去重语义被遮形悄悄改掉。
#[test]
fn clean_ingest_content_is_byte_identical_and_keeps_the_hash() {
    let item = summary(
        "Ordinary headline",
        "Ordinary body with nothing sensitive.",
        Some("https://example.com/clean"),
    );
    let legacy = format!(
        "{}\n{}\n{}\n{}",
        item.title,
        item.body_excerpt,
        item.item_link.as_deref().unwrap_or(""),
        item.published_raw.as_deref().unwrap_or("")
    );
    let content = stable_ingest_content(&item);
    assert_eq!(
        content, legacy,
        "零命中 ⇒ 入库内容必须逐字节不变 (散列门依赖内容稳定)"
    );
    assert_eq!(
        fnv1a64(&content),
        fnv1a64(&legacy),
        "零命中 ⇒ 散列门的散列必须逐字节不变 (⇒ 同条目重复拉取仍被散列门跳过)"
    );
    assert!(
        !content.contains("credential_tripwire"),
        "零命中 ⇒ 不挂任何标记 (干净答案的形状不变): {content}"
    );
}

/// 核心判据的**另一半**: 同条目不同拉取时刻 ⇒ 内容逐字相同 ⇒ 散列相同。
#[test]
fn clean_ingest_content_is_stable_across_fetch_times() {
    let item = summary("标题", "正文", Some("https://example.com/1"));
    let mut later = item.clone();
    later.fetched_at_ms = 9_999_999_999_999;
    let first = stable_ingest_content(&item);
    let second = stable_ingest_content(&later);
    assert_eq!(first, second, "同条目不同拉取时刻 ⇒ 内容逐字相同");
    assert_eq!(fnv1a64(&first), fnv1a64(&second), "⇒ 散列相同");
}

// ---------------------------------------------------------------------------
// ② 命中 ⇒ 真 key 不进摄取内容 + 命中可观测
// ---------------------------------------------------------------------------

/// 命中 ⇒ 真 key **不进**摄取内容 (凭据进存储的面被挡住) + 命中可观测。
#[test]
fn credential_shaped_body_is_masked_before_it_reaches_the_memory() {
    let item = summary(
        "Leaky headline",
        &format!("Body with {FAKE_KEY} inside."),
        Some("https://example.com/1"),
    );
    let content = stable_ingest_content(&item);
    assert!(
        !content.contains(FAKE_KEY),
        "真 key 不得进摄取内容 (那是凭据进存储): {content}"
    );
    assert!(
        content.contains("[REDACTED_OPENAI_KEY]"),
        "命中必须换成遮形标签: {content}"
    );
    assert!(
        content.contains("credential_tripwire"),
        "命中必须可观测 (裸文本面 ⇒ 末尾一行 JSON 注解): {content}"
    );
    // 注解**零真值**: 只有命中族名, 没有凭据值。
    let marker = content
        .lines()
        .find(|line| line.contains("credential_tripwire"))
        .expect("命中必须留一行 JSON 注解");
    assert!(!marker.contains(FAKE_KEY), "注解不得含凭据真值: {marker}");
    assert!(marker.contains("OpenAiKey"), "注解必须报命中族: {marker}");
}

/// 命中 ⇒ 遮形**确定性** ⇒ 同条目重复拉取仍产生同一散列 ⇒ 去重语义不破。
#[test]
fn masked_ingest_content_is_deterministic_across_fetch_times() {
    let item = summary(
        "标题",
        &format!("正文含 {FAKE_KEY}"),
        Some("https://example.com/1"),
    );
    let mut later = item.clone();
    later.fetched_at_ms = 9_999_999_999_999;
    let first = stable_ingest_content(&item);
    let second = stable_ingest_content(&later);
    assert_eq!(first, second, "遮形后仍须逐字相同 (确定性)");
    assert_eq!(fnv1a64(&first), fnv1a64(&second), "⇒ 散列仍相同");
}

/// 命中 ⇒ 内容**确实变了** (否则「遮形」是空转) 且变的是**值**不是布局。
#[test]
fn masked_ingest_content_differs_from_the_raw_layout() {
    let item = summary(
        "标题",
        &format!("正文含 {FAKE_KEY}"),
        Some("https://example.com/1"),
    );
    let raw = format!(
        "{}\n{}\n{}\n{}",
        item.title,
        item.body_excerpt,
        item.item_link.as_deref().unwrap_or(""),
        item.published_raw.as_deref().unwrap_or("")
    );
    assert_ne!(
        stable_ingest_content(&item),
        raw,
        "命中 ⇒ 入库内容必须与原文不同 (否则凭据仍进存储)"
    );
}

/// 另一族形状 (AWS 访问键形态) 也被挡住 (形状层是多族, 不是单点)。
#[test]
fn other_credential_shapes_are_masked_too() {
    let item = summary(
        "标题",
        &format!("正文含 {FAKE_AWS_KEY}"),
        Some("https://example.com/1"),
    );
    let content = stable_ingest_content(&item);
    assert!(
        !content.contains(FAKE_AWS_KEY),
        "AWS 键形态也不得进摄取内容: {content}"
    );
    assert!(
        content.contains("[REDACTED_AWS_KEY]"),
        "命中必须换成遮形标签: {content}"
    );
}

/// 遮形**不牵动条目身份 / 来源** (身份走**原始**链接与原始标题正文 ⇒ 遮形
/// 不改去重第一维 ⇒ 条目账与散列门两道去重都还认这条条目)。
#[test]
fn masking_does_not_disturb_the_item_identity_or_source() {
    let item = summary(
        "标题",
        &format!("正文含 {FAKE_KEY}"),
        Some("https://example.com/1"),
    );
    assert_eq!(
        item_identity(&item),
        "link:https://example.com/1",
        "有链接 ⇒ 身份走链接 (与遮形无关)"
    );
    assert_eq!(
        item_source("news", &item),
        "content_feed:news:link:https://example.com/1",
        "来源 = 订阅 id + 条目身份 (遮形不改它)"
    );

    // 缺链接 ⇒ 身份退化到**原始**标题正文的散列 (遮形不改身份派号口径)。
    let mut no_link = item.clone();
    no_link.item_link = None;
    let expected = format!(
        "hash:{:016x}",
        fnv1a64(&format!("{}\n{}", no_link.title, no_link.body_excerpt))
    );
    assert_eq!(
        item_identity(&no_link),
        expected,
        "缺链接 ⇒ 身份按原始标题正文散列派号 (与遮形后的内容散列是两回事)"
    );
}

// ---------------------------------------------------------------------------
// ③ 端到端: 含真 key 的 feed ⇒ 投摄取端口的内容无明文
// ---------------------------------------------------------------------------

/// 端到端 (回环 mock RSS → 真实工具 → 真实摄取端口): 含形状密钥的 feed
/// ⇒ **投进摄取端口的内容无明文** (那是进 episodic 记忆的那一份)。
#[tokio::test]
async fn feed_with_a_credential_shape_reaches_the_ingest_port_without_plaintext() {
    let (port, server) = serve_rss(1, RSS_MIXED).await;
    let dir = tempfile::tempdir().expect("tempdir");
    let ingest = RecordingIngest::new();
    let episode_writer: EpisodeWriter = Arc::new(|_episode: &Episode| Ok(()));
    let config = ContentFeedConfig::new(
        dir.path(),
        Arc::new(allowlist_egress(Duration::from_secs(5))),
        episode_writer,
    )
    .with_ingest_port(ingest.port());
    let tool = ContentFeedTool::new(config);
    value_of(
        tool.invoke(&call(json!({
            "action": "feed_add",
            "id": "news",
            "url": format!("http://127.0.0.1:{port}/rss"),
            "kind": "rss"
        })))
        .await,
    );
    let pulled = value_of(tool.invoke(&call(json!({ "action": "pull" }))).await);
    server.await.unwrap();
    assert_eq!(pulled["ingested"], 2, "两条摘要都投摄取");

    let requests = ingest.requests();
    assert_eq!(requests.len(), 2, "每条摘要投一次摄取");
    for request in &requests {
        assert!(
            !request.content.contains(FAKE_KEY),
            "投进摄取端口的内容不得含凭据明文: {}",
            request.content
        );
    }
    let leaky = requests
        .iter()
        .find(|request| request.source.contains("/1"))
        .expect("含密钥那条");
    assert!(
        leaky.content.contains("[REDACTED_OPENAI_KEY]"),
        "含密钥那条必须被遮形: {}",
        leaky.content
    );
    let clean = requests
        .iter()
        .find(|request| request.source.contains("/2"))
        .expect("干净那条");
    assert_eq!(
        clean.content, "Clean headline\nBody with nothing sensitive.\nhttps://example.com/2\nWed, 02 Oct 2024 07:00:00 GMT",
        "干净那条必须逐字节不变 (零命中 ⇒ 散列不变)"
    );
}

// ---------------------------------------------------------------------------
// ④ 接线守卫 + 命名纪律
// ---------------------------------------------------------------------------

/// 接线回归守卫: 记忆摄取面必须接**共享助手** (`scan_result_face`)。
///
/// 口径与批次 2 的族级守卫同源 (按**族文件或族子模块**里有无接线调用判定,
/// 不是按字面量计)。
#[test]
fn the_memory_ingest_face_is_wired_to_the_shared_result_face_helper() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = vec![src.join("content_feed").join("ingest.rs")];
    let submodule_dir = src.join("content_feed").join("ingest");
    if submodule_dir.is_dir() {
        let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(&submodule_dir)
            .expect("族子模块目录可读")
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
            .collect();
        entries.sort();
        sources.extend(entries);
    }
    let wired = sources.iter().any(|path| {
        std::fs::read_to_string(path)
            .map(|text| text.contains("scan_result_face"))
            .unwrap_or(false)
    });
    assert!(
        wired,
        "记忆摄取面必须接共享助手 scan_result_face (族文件或其子模块)"
    );
}

/// 命名纪律: 新增的常量 / 字符串**不得**命中 `APEIRETH_[A-Z0-9_]+`
/// (那是本仓 env 开关命名空间 ⇒ `arch_guard` 的 env 棘轮会红)。
#[test]
fn the_new_ingest_face_surface_enters_no_env_namespace_name() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("content_feed")
            .join("ingest.rs"),
    )
    .expect("摄取面源文件必须可读");
    let mut cursor = source.as_str();
    while let Some(index) = cursor.find("APEIRETH_") {
        let tail = &cursor[index + "APEIRETH_".len()..];
        let name_len = tail
            .find(|c: char| !c.is_ascii_uppercase() && !c.is_ascii_digit() && c != '_')
            .unwrap_or(tail.len());
        panic!(
            "摄取面不得引入 env 命名空间名 (APEIRETH_[A-Z0-9_]+): {}",
            &cursor[index..index + "APEIRETH_".len() + name_len]
        );
    }
}
