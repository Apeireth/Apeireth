//! 订阅源面: 数据目录 `feeds.json` 的订阅列表 + 拉取台账。
//!
//! # StoredDoc 拒开语义
//!
//! `feeds.json` 落盘为 [`stored_doc::StoredDoc`] 信封 JSON: 身份 / 版本戳 /
//! 主体。打开即校验 —— 畸形 JSON、串档身份、未来版本、不自洽信封、坏条目
//! (非法 id / 非法 URL / 敏感面 URL) 一律**拒绝打开**并报结构化错误
//! ([`stored_doc::StoredDocError`] / [`FeedStoreError`]), 不静默迁移、不回退
//! 默认、不带病运行。文件缺失是合法空态 (`Ok(None)`), 是否可缺省由调用方
//! 决定 (与用户表同口径)。
//!
//! # 敏感面拒绝 (拒, 不是审批)
//!
//! 订阅 URL 属于「敏感面拒绝」而非「审批可放」: 带凭据 (userinfo) 的 URL、
//! 非 http(s) 协议面、内网/回环/链路本地 IP 字面量一律**拒绝**, 不进订阅表
//! 也不出网。IP 分类复用受控出口通道的 [`crate::egress::classify_ip` /
//! `is_public_ip`] (复用不重复: 不另造一套 IP 分类)。主机名的内网判定归
//! 受控出口通道在解析后做 (不在工具里做 DNS 决策), 拉取时逐跳重验。
//!
//! # 写面归类 (write_intent 同族)
//!
//! 改订阅 = 写文件 (`feeds.json`): [`FeedWriteFace::SubscriptionFileWrite`],
//! 与文件写入同一治理族 (每次写停人工审批)。拉取台账 ([`FeedStore::record_pull`])
//! 是拉取动作自带的本地记账, 随拉取同档 (拉取与入库都经审批档)。

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::egress::{self, EgressDestination, EgressPolicy};
use crate::fetch::FETCH_URL_MAX_BYTES;
use crate::self_status::ContentFeedStatus;

use super::lifecycle::FeedLifecycleStore;

/// `feeds.json` 的文档身份 (StoredDoc 信封 name 字段)。
pub const FEEDS_DOC_NAME: &str = "apeireth.content_feeds";
/// `feeds.json` 的格式版本戳 (读端当前版本)。
pub const FEEDS_DOC_VERSION: u32 = 1;
/// 数据目录里的文件名。
pub const FEEDS_FILE_NAME: &str = "feeds.json";
/// 订阅 id 的最大长度 (字节)。
pub const FEED_ID_MAX_BYTES: usize = 64;
/// 单表订阅条数上限 (有界面: 不随模型调用无界增长)。
pub const FEEDS_MAX_COUNT: usize = 256;

/// 读端版本契约: 只认当前版本 (历史/未来版本拒开)。
pub fn feeds_doc_compat() -> DocCompat {
    DocCompat::exact(FEEDS_DOC_NAME, FEEDS_DOC_VERSION)
}

/// 订阅源形态 (摘要解析按形态分派; 文档实际形态仍以内容嗅探为准)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedKind {
    /// RSS 2.x 频道 (`<item>` 条目)。
    Rss,
    /// Atom 1.0 源 (`<entry>` 条目)。
    Atom,
    /// 普通网页 URL (HTML / 纯文本)。
    Web,
}

impl FeedKind {
    /// 稳定词表 (工具参数与落盘共用)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rss => "rss",
            Self::Atom => "atom",
            Self::Web => "web",
        }
    }

    /// 从工具参数词表解析; 未知词拒绝 (闭合词表)。
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "rss" => Ok(Self::Rss),
            "atom" => Ok(Self::Atom),
            "web" => Ok(Self::Web),
            other => Err(format!(
                "unknown feed kind {other:?}; the closed vocabulary is rss / atom / web"
            )),
        }
    }
}

/// 一条订阅源。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedSource {
    /// 订阅 id (闭合字符集, 见 [`validate_feed_id`])。
    pub id: String,
    /// 订阅 URL (http/https, 无凭据, 无片段)。
    pub url: String,
    /// 形态。
    pub kind: FeedKind,
    /// 入表时刻 (epoch 毫秒)。
    pub added_at_ms: i64,
}

/// 拉取台账一行 (自述面的「最近拉取」真值来源)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRecord {
    /// 拉取时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 本次拉取是否完整成功 (false = 有失败真因, 见 [`Self::note`])。
    pub ok: bool,
    /// 参与拉取的订阅数。
    pub feeds: u64,
    /// 入记忆流的结构化摘要条数。
    pub ingested: u64,
    /// 如实备注 (失败真因 / 口径注记; 不编造)。
    pub note: String,
}

/// `feeds.json` 的主体。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedsBody {
    /// 订阅列表 (有序, 按入表先后)。
    #[serde(default)]
    pub feeds: Vec<FeedSource>,
    /// 最近一次拉取台账; `None` = 尚无拉取。
    #[serde(default)]
    pub last_pull: Option<PullRecord>,
}

/// 订阅面错误 (闭合词表: 拒开 / 拒收 / 表满 / 重复 / 缺席)。
#[derive(Debug, thiserror::Error)]
pub enum FeedStoreError {
    /// `feeds.json` 存在但拒开 (StoredDoc 语义: 不静默修复)。
    #[error("feeds.json refused to open: {0}")]
    Stored(#[from] StoredDocError),
    /// 订阅条目被拒收 (敏感面 / 非法 id / 非法 URL)。
    #[error("feed entry refused: {0}")]
    Refused(String),
    /// 订阅 id 重复。
    #[error("feed id already subscribed: {0}")]
    DuplicateId(String),
    /// 订阅 id 不在表内。
    #[error("feed id not subscribed: {0}")]
    UnknownId(String),
    /// 订阅表已满 (有界面)。
    #[error("feed list is full (max {0})")]
    Full(usize),
}

/// 写面归类 (write_intent 同族: 改订阅 = 写文件)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedWriteFace {
    /// 纯读 (列订阅), 零本地写。
    ReadOnly,
    /// 改订阅 = 写 `feeds.json` —— 与文件写入同一治理族。
    SubscriptionFileWrite,
    /// 拉取 = 读网络 + 写记忆流 (双面)。
    NetworkReadAndMemoryWrite,
}

impl FeedWriteFace {
    /// 稳定标签 (审计/测试用)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::SubscriptionFileWrite => "subscription_file_write",
            Self::NetworkReadAndMemoryWrite => "network_read_and_memory_write",
        }
    }
}

/// 订阅 id 校验: 闭合字符集 `[A-Za-z0-9._-]`, 1..=[`FEED_ID_MAX_BYTES`] 字节。
///
/// id 会进 episode id 与封套来源标注, 故字符集收紧 (无空白/无分隔符/无控制符)。
pub fn validate_feed_id(raw: &str) -> Result<(), String> {
    if raw.is_empty() {
        return Err("feed id must not be empty".to_string());
    }
    if raw.len() > FEED_ID_MAX_BYTES {
        return Err(format!(
            "feed id is {} bytes; the maximum is {FEED_ID_MAX_BYTES}",
            raw.len()
        ));
    }
    if !raw
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err(
            "feed id must match [A-Za-z0-9._-] only (it is embedded in episode ids and envelope \
             source labels)"
                .to_string(),
        );
    }
    Ok(())
}

/// 订阅 URL 校验 (敏感面拒绝: 凭据 / 非 http(s) / 内网 IP 字面量)。
///
/// 返回归一化 URL (片段剥除, 与受控 fetch 同口径)。拒绝原因**不回显** URL
/// 本体 —— 凭据面拒绝时绝不把密码带回错误文本。
pub fn validate_feed_url(raw: &str, policy: &EgressPolicy) -> Result<Url, String> {
    if raw.is_empty() {
        return Err("feed url must not be empty".to_string());
    }
    if raw.len() > FETCH_URL_MAX_BYTES {
        return Err(format!(
            "feed url is {} bytes; the maximum is {FETCH_URL_MAX_BYTES}",
            raw.len()
        ));
    }
    let mut url = Url::parse(raw).map_err(|e| format!("invalid feed url: {e}"))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(format!(
            "unsupported feed url scheme {:?}; only http and https are supported",
            url.scheme()
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        // 敏感面拒绝: 带凭据的 URL 不进订阅表、不出网、不回显。
        return Err(
            "feed url userinfo is refused (sensitive surface: credentials in URL are never stored \
             or sent)"
                .to_string(),
        );
    }
    if url.host_str().is_none() {
        return Err("feed url must include a host".to_string());
    }
    url.set_fragment(None);

    // 目的地面: 复用受控出口通道的策略判定 (DenyAll / 显式白名单)。
    let destination =
        EgressDestination::parse(&url).map_err(|e| format!("feed url refused: {e}"))?;
    policy
        .validate_destination(&destination)
        .map_err(|e| format!("feed url refused: {e}"))?;

    // 内网地址拒绝 (字面量面): 仅对 IP 字面量目的地做静态分类, 复用受控出口
    // 通道的 IP 分类 (`is_public_ip`), 不另造分类口径。IPv4/IPv6 字面量都按
    // `url::Host` 枚举取地址 (IPv6 字面量带方括号, 字符串解析取不到, 必须走
    // Host 枚举)。主机名的内网判定归受控出口通道解析后逐跳重验 (不在工具里
    // 做 DNS 决策)。
    if matches!(policy, EgressPolicy::PublicInternetOnly) {
        let literal = match url.host() {
            Some(url::Host::Ipv4(address)) => Some(std::net::IpAddr::V4(address)),
            Some(url::Host::Ipv6(address)) => Some(std::net::IpAddr::V6(address)),
            _ => None,
        };
        if let Some(address) = literal {
            if !egress::is_public_ip(address) {
                return Err(format!(
                    "feed url destination {address} is refused (sensitive surface: non-public \
                     address under the public-internet-only egress policy)"
                ));
            }
        }
    }
    Ok(url)
}

/// 订阅表存储 (数据目录 `feeds.json`)。
#[derive(Debug, Clone)]
pub struct FeedStore {
    path: PathBuf,
    policy: EgressPolicy,
    lifecycle: FeedLifecycleStore,
}

impl FeedStore {
    /// 绑定数据目录与出口策略 (URL 校验口径与拉取通道同一策略)。
    ///
    /// 同目录挂生命周期账 ([`FeedLifecycleStore`]: 条目去重 / 退避 / 健康)。
    pub fn new(data_dir: impl Into<PathBuf>, policy: EgressPolicy) -> Self {
        let dir = data_dir.into();
        Self {
            path: dir.join(FEEDS_FILE_NAME),
            policy,
            lifecycle: FeedLifecycleStore::for_data_dir(dir),
        }
    }

    /// `feeds.json` 路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 出口策略 (URL 校验口径)。
    pub fn policy(&self) -> &EgressPolicy {
        &self.policy
    }

    /// 生命周期账 (条目去重 / 退避 / 健康 / 日计数)。
    pub fn lifecycle(&self) -> &FeedLifecycleStore {
        &self.lifecycle
    }

    /// 装载订阅表 (StoredDoc 拒开语义):
    ///
    /// - 文件缺失 = `Ok(None)` (合法空态);
    /// - 存在但坏 (畸形 / 串档 / 未来版本 / 坏条目) = `Err` (拒开, 不回退)。
    pub fn load(&self) -> Result<Option<FeedsBody>, FeedStoreError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let doc = stored_doc::open_single::<FeedsBody>(&self.path, &feeds_doc_compat())?;
        self.validate_body(&doc.body)?;
        Ok(Some(doc.body))
    }

    /// 落盘订阅表 (持久档原子写, 盖当前版本戳)。
    pub fn save(&self, body: &FeedsBody) -> Result<(), FeedStoreError> {
        self.validate_body(body)?;
        stored_doc::save_single(
            &self.path,
            &feeds_doc_compat(),
            body,
            stored_doc::DEFAULT_DOC_MODE,
        )?;
        Ok(())
    }

    /// 表体校验: 条目闭合校验 (id / URL / 重复 / 上限) —— 装载与写出同口径,
    /// 坏表**拒开**也拒写 (不带病落盘)。
    fn validate_body(&self, body: &FeedsBody) -> Result<(), FeedStoreError> {
        if body.feeds.len() > FEEDS_MAX_COUNT {
            return Err(FeedStoreError::Full(FEEDS_MAX_COUNT));
        }
        let mut seen = std::collections::BTreeSet::new();
        for feed in &body.feeds {
            validate_feed_id(&feed.id).map_err(FeedStoreError::Refused)?;
            validate_feed_url(&feed.url, &self.policy).map_err(FeedStoreError::Refused)?;
            if !seen.insert(feed.id.clone()) {
                return Err(FeedStoreError::DuplicateId(feed.id.clone()));
            }
        }
        Ok(())
    }

    /// 增订阅 (改订阅 = 写文件, [`FeedWriteFace::SubscriptionFileWrite`])。
    ///
    /// 落盘的是**归一化** URL (片段剥除, 与拉取口径一致)。
    pub fn add_feed(&self, feed: FeedSource) -> Result<FeedsBody, FeedStoreError> {
        validate_feed_id(&feed.id).map_err(FeedStoreError::Refused)?;
        let normalized =
            validate_feed_url(&feed.url, &self.policy).map_err(FeedStoreError::Refused)?;
        let feed = FeedSource {
            url: normalized.to_string(),
            ..feed
        };
        let mut body = self.load()?.unwrap_or_default();
        if body.feeds.iter().any(|item| item.id == feed.id) {
            return Err(FeedStoreError::DuplicateId(feed.id));
        }
        if body.feeds.len() >= FEEDS_MAX_COUNT {
            return Err(FeedStoreError::Full(FEEDS_MAX_COUNT));
        }
        body.feeds.push(feed);
        self.save(&body)?;
        Ok(body)
    }

    /// 删订阅 (改订阅 = 写文件, [`FeedWriteFace::SubscriptionFileWrite`])。
    pub fn remove_feed(&self, id: &str) -> Result<FeedsBody, FeedStoreError> {
        let mut body = self
            .load()?
            .ok_or_else(|| FeedStoreError::UnknownId(id.to_string()))?;
        let before = body.feeds.len();
        body.feeds.retain(|item| item.id != id);
        if body.feeds.len() == before {
            return Err(FeedStoreError::UnknownId(id.to_string()));
        }
        self.save(&body)?;
        Ok(body)
    }

    /// 列订阅 (纯读)。
    pub fn list(&self) -> Result<FeedsBody, FeedStoreError> {
        Ok(self.load()?.unwrap_or_default())
    }

    /// 记一笔拉取台账 (拉取动作自带的本地记账, 随拉取同档)。
    pub fn record_pull(&self, record: PullRecord) -> Result<FeedsBody, FeedStoreError> {
        let mut body = self.load()?.unwrap_or_default();
        body.last_pull = Some(record);
        self.save(&body)?;
        Ok(body)
    }

    /// 自述面真值来源: 订阅数 + 最近拉取 + 生命周期真值 (如实; 缺位显式 null + 原因)。
    ///
    /// J2 生命周期字段 (条目去重 / 投摄取 / 注意力截断 / 订阅健康) 取自同目录
    /// 生命周期账; 账读失败 = 字段级 `None` + `reason`, 不让整帧失败 (0 装诚实)。
    pub fn status(&self) -> Result<ContentFeedStatus, String> {
        let body = self
            .load()
            .map_err(|error| format!("feeds.json refused to open: {error}"))?
            .unwrap_or_default();
        let last_pull_note = body
            .last_pull
            .as_ref()
            .map(|record| record.note.clone())
            .or_else(|| Some("尚无拉取记录".to_string()));
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let (
            dedup_skipped,
            ingest_queued,
            attention_truncated,
            degraded_feeds,
            backing_off_feeds,
            lifecycle_reason,
        ) = match self.lifecycle.stats(now_ms) {
            Ok(stats) => (
                Some(stats.dedup_skipped),
                Some(stats.ingest_queued),
                Some(stats.attention_truncated),
                Some(stats.degraded_feeds),
                Some(stats.backing_off_feeds),
                None,
            ),
            Err(error) => (None, None, None, None, None, Some(error)),
        };
        Ok(ContentFeedStatus {
            feeds: Some(body.feeds.len() as u64),
            last_pull_at_ms: body.last_pull.as_ref().map(|record| record.at_ms),
            last_pull_ok: body.last_pull.as_ref().map(|record| record.ok),
            last_pull_ingested: body.last_pull.as_ref().map(|record| record.ingested),
            last_pull_note,
            dedup_skipped,
            ingest_queued,
            attention_truncated,
            degraded_feeds,
            backing_off_feeds,
            reason: lifecycle_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> EgressPolicy {
        EgressPolicy::PublicInternetOnly
    }

    fn store(dir: &Path) -> FeedStore {
        FeedStore::new(dir, policy())
    }

    fn feed(id: &str, url: &str) -> FeedSource {
        FeedSource {
            id: id.to_string(),
            url: url.to_string(),
            kind: FeedKind::Rss,
            added_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn add_list_remove_round_trip_persists_the_stored_doc() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        assert!(store.list().expect("empty list").feeds.is_empty());

        store
            .add_feed(feed("news", "https://example.com/rss"))
            .expect("add");
        store
            .add_feed(feed("blog", "https://example.org/atom"))
            .expect("add");
        let body = store.list().expect("list");
        assert_eq!(body.feeds.len(), 2);
        assert_eq!(body.feeds[0].id, "news");

        // 落盘形态是 StoredDoc 信封 JSON (身份 + 版本戳)。
        let raw = std::fs::read_to_string(store.path()).expect("read");
        assert!(raw.contains(FEEDS_DOC_NAME), "{raw}");
        assert!(raw.contains("\"version\""), "{raw}");

        let body = store.remove_feed("news").expect("remove");
        assert_eq!(body.feeds.len(), 1);
        assert_eq!(body.feeds[0].id, "blog");
    }

    #[test]
    fn missing_file_is_an_empty_state_but_broken_files_refuse_to_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        assert!(store.load().expect("missing is empty").is_none());

        // 畸形 JSON = 拒开。
        std::fs::write(store.path(), b"{ not json").expect("write");
        assert!(store.load().is_err(), "坏配置必须拒开");
        assert!(store.add_feed(feed("a", "https://example.com/x")).is_err());

        // 未来版本 = 拒开 (不静默迁移)。
        let future = serde_json::json!({
            "name": FEEDS_DOC_NAME,
            "version": 99,
            "compatible_versions": [99],
            "body": { "feeds": [] }
        });
        std::fs::write(store.path(), future.to_string()).expect("write");
        let err = store.load().unwrap_err();
        assert!(
            err.to_string().contains("99") || matches!(err, FeedStoreError::Stored(_)),
            "{err}"
        );

        // 串档身份 = 拒开。
        let wrong = serde_json::json!({
            "name": "apeireth.something_else",
            "version": FEEDS_DOC_VERSION,
            "compatible_versions": [FEEDS_DOC_VERSION],
            "body": { "feeds": [] }
        });
        std::fs::write(store.path(), wrong.to_string()).expect("write");
        assert!(store.load().is_err(), "串档必须拒开");
    }

    #[test]
    fn broken_entries_inside_an_envelope_refuse_to_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        let bad = serde_json::json!({
            "name": FEEDS_DOC_NAME,
            "version": FEEDS_DOC_VERSION,
            "compatible_versions": [FEEDS_DOC_VERSION],
            "body": { "feeds": [ { "id": "bad id with spaces", "url": "https://example.com/x",
                                   "kind": "web", "added_at_ms": 0 } ] }
        });
        std::fs::write(store.path(), bad.to_string()).expect("write");
        assert!(store.load().is_err(), "坏条目 = 坏配置 = 拒开");
    }

    #[test]
    fn sensitive_and_invalid_urls_are_refused_not_approved() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());

        // 凭据面 (userinfo): 拒, 且不回显密码。
        let err = store
            .add_feed(feed("cred", "https://user:secret@example.com/rss"))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("userinfo") || err.contains("sensitive"),
            "{err}"
        );
        assert!(!err.contains("secret"), "拒绝原因不得回显凭据: {err}");

        // 协议面: 非 http(s) 拒。
        for url in [
            "file:///etc/passwd",
            "ftp://example.com/x",
            "data:text/plain,x",
        ] {
            let err = store.add_feed(feed("proto", url)).unwrap_err().to_string();
            assert!(
                err.contains("scheme") || err.contains("refused"),
                "{url}: {err}"
            );
        }

        // 内网地址面 (IP 字面量): 拒 (复用受控通道 IP 分类)。
        for url in [
            "http://127.0.0.1:9/feed",
            "http://10.0.0.5/feed",
            "http://192.168.1.10/feed",
            "http://169.254.169.254/latest/meta-data",
            "http://[::1]/feed",
        ] {
            let err = store
                .add_feed(feed("private", url))
                .unwrap_err()
                .to_string();
            assert!(err.contains("refused"), "{url}: {err}");
        }

        // 合法公网 URL 过。
        store
            .add_feed(feed("ok", "https://example.com:8443/rss#frag"))
            .expect("public url ok");
        let body = store.list().expect("list");
        assert_eq!(
            body.feeds[0].url, "https://example.com:8443/rss",
            "片段剥除"
        );
    }

    #[test]
    fn duplicate_ids_and_oversized_tables_are_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());
        store
            .add_feed(feed("dup", "https://example.com/a"))
            .expect("add");
        let err = store
            .add_feed(feed("dup", "https://example.com/b"))
            .unwrap_err();
        assert!(matches!(err, FeedStoreError::DuplicateId(_)), "{err}");

        let err = store.remove_feed("missing").unwrap_err();
        assert!(matches!(err, FeedStoreError::UnknownId(_)), "{err}");

        assert!(validate_feed_id("").is_err());
        assert!(validate_feed_id("has space").is_err());
        assert!(validate_feed_id(&"x".repeat(FEED_ID_MAX_BYTES + 1)).is_err());
    }

    #[test]
    fn pull_ledger_records_honestly_and_status_reports_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = store(dir.path());

        // 尚无拉取: 显式 null + 如实原因, 不编造时间。
        let status = store.status().expect("status");
        assert_eq!(status.feeds, Some(0));
        assert_eq!(status.last_pull_at_ms, None);
        assert_eq!(status.last_pull_note.as_deref(), Some("尚无拉取记录"));

        store
            .add_feed(feed("news", "https://example.com/rss"))
            .expect("add");
        store
            .record_pull(PullRecord {
                at_ms: 1_700_000_500_000,
                ok: true,
                feeds: 1,
                ingested: 3,
                note: "3 items".to_string(),
            })
            .expect("record");
        let status = store.status().expect("status");
        assert_eq!(status.feeds, Some(1));
        assert_eq!(status.last_pull_at_ms, Some(1_700_000_500_000));
        assert_eq!(status.last_pull_ok, Some(true));
        assert_eq!(status.last_pull_ingested, Some(3));
        assert_eq!(status.last_pull_note.as_deref(), Some("3 items"));
        assert!(status.reason.is_none());
    }

    #[test]
    fn write_face_classification_is_pinned_per_action_family() {
        // write_intent 同族口径: 改订阅 = 写文件; 拉取 = 读网络 + 写记忆。
        assert_eq!(
            FeedWriteFace::SubscriptionFileWrite.label(),
            "subscription_file_write"
        );
        assert_eq!(
            FeedWriteFace::NetworkReadAndMemoryWrite.label(),
            "network_read_and_memory_write"
        );
        assert_eq!(FeedWriteFace::ReadOnly.label(), "read_only");
    }
}
