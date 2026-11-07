//! 拉取→摄取接缝 (J2): 条目摘要投保鲜回路的 `enqueue_ingest` 端口 + 注意力纪律。
//!
//! # 端口只读消费
//!
//! 内容流不触碰记忆内部件 —— 它只**消费** [`IngestPort`]: 一条把
//! `(source, topic, content, now_ms)` 投进保鲜摄取队列的窄接口, 语义对齐
//! 记忆保鲜回路的 `SqliteMemoryStore::enqueue_ingest` (V4-b)。生产接线把真
//! 存储的 `enqueue_ingest` 适配到本端口 ([`IngestOutcome`] 逐态映射
//! `EnqueueOutcome`), 保鲜回路的**散列门 / 摄取队列 / 增量折入全自动**:
//! 同源同散列重复摄取由散列门跳过, 变内容重摄, 失败留队列。
//!
//! # 条目身份 (source = 订阅 id + 条目链接)
//!
//! 每条摘要投摄取时派一个**稳定 source** ([`item_source`]): 有条目链接用
//! 链接, 没链接退化到内容散列 —— 同条目跨次拉取永远同一 source, 因此
//! (a) 摄取端口的散列门按内容去重, (b) 生命周期条目账 ([`crate::content_feed::lifecycle`])
//! 按 source 补条目级幂等。两道去重合起来 = **同条目重复拉取零新记忆**。
//!
//! # 注意力纪律 (防刷屏进记忆)
//!
//! [`AttentionLimits`] 两道闸: **单次拉取条目上限** + **每日摄取上限**。超限
//! **截断且如实标注** (不静默丢、不假装全摄), 与 A 轴配额真接口联动归后续
//! (J 轴纪律: 防沉迷是产品底线)。
//!
//! # 凭据形状绊线 (记忆摄取面, 批次 3b)
//!
//! 本模块产出的摄取内容**直接进 episodic 记忆** ⇒ ⇒ 它是**凭据进存储**的
//! 路径 (不是进 transcript) ⇒ ⇒ 比 transcript 更「深」⇒ ⇒ 内容在**入库之前**
//! 过一道**形状层**扫描 ([`mask_ingest_content`]): 真 key 不进存储。
//!
//! * **零命中** ⇒ 内容**逐字节原样** (不挂任何标记) ⇒ ⇒ **散列不变**
//!   (散列门按内容散列去重 ⇒ 内容不变 ⇒ 散列不变 ⇒ 同条目重复拉取零新记忆);
//! * **命中** ⇒ 内容换成遮形文本 (单向标签, 不可逆) + 末尾一行 JSON 注解
//!   (命中不静默)。
//!
//! ⚠️ **散列输入与入库内容在本面是同一份字符串** (读码结论, 见
//! [`stable_ingest_content`] 的说明) ⇒ ⇒ 遮形**同时**改这两者; 本面**不能**
//! 「只遮入库不改散列」—— 那要改记忆库那一侧 (不在本面写域内)。

use std::sync::Arc;

use crate::content_feed::summary::FeedItemSummary;
use crate::guardrail::{append_credential_tripwire, scan_result_face};

/// 单次拉取条目上限默认值 (防一次刷屏进记忆)。
pub const DEFAULT_PER_PULL_ITEMS: usize = 50;
/// 每日摄取上限默认值 (防一天刷屏进记忆)。
pub const DEFAULT_DAILY_INGESTS: usize = 500;

/// 注意力纪律上限 (两道闸, 超限截断如实标注)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttentionLimits {
    /// 单次拉取条目上限 (0 = 拒绝拉取任何条目, 如实)。
    pub per_pull_items: usize,
    /// 每日摄取上限 (跨条目累计, 按 epoch 日重置)。
    pub daily_ingests: usize,
}

impl Default for AttentionLimits {
    fn default() -> Self {
        Self {
            per_pull_items: DEFAULT_PER_PULL_ITEMS,
            daily_ingests: DEFAULT_DAILY_INGESTS,
        }
    }
}

impl AttentionLimits {
    /// 覆盖单次拉取条目上限。
    #[must_use]
    pub fn with_per_pull_items(mut self, items: usize) -> Self {
        self.per_pull_items = items;
        self
    }

    /// 覆盖每日摄取上限。
    #[must_use]
    pub fn with_daily_ingests(mut self, ingests: usize) -> Self {
        self.daily_ingests = ingests;
        self
    }
}

/// 投摄取请求 (对齐 `enqueue_ingest` 的入参)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestRequest {
    /// 来源 (订阅 id + 条目链接稳定派号; 散列门去重第一维)。
    pub source: String,
    /// 折入主题 (增量折入合并键; 取订阅 id, 同订阅同主题追加)。
    pub topic: String,
    /// 摄取内容 (稳定摘要, 不含拉取时刻 —— 否则散列门无法按内容去重)。
    pub content: String,
    /// 投递时刻 (epoch 毫秒)。
    pub now_ms: i64,
}

/// 摄取结果 (逐态映射保鲜回路 `EnqueueOutcome`, 外加端口自身故障)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestOutcome {
    /// 新任务已入摄取队列 (待摄取)。
    Enqueued {
        /// 任务 id (由摄取端口派号)。
        task_id: String,
    },
    /// 同源同散列已摄取过 = 散列门跳过 (去重计数真值)。
    DuplicateSkipped {
        /// 累计去重计数 (含本次)。
        skip_count: u64,
    },
    /// 同源同散列已在队 (含失败待重试) = 单行, 不重复摄。
    AlreadyQueued {
        /// 既有任务 id。
        task_id: String,
    },
    /// 摄取端口自身故障 (真因, 不静默)。
    Failed {
        /// 失败原因。
        error: String,
    },
}

impl IngestOutcome {
    /// 是否已进待摄取队列 (Enqueued / AlreadyQueued = 投摄取成功, 只是可能还在队)。
    pub const fn queued(&self) -> bool {
        matches!(self, Self::Enqueued { .. } | Self::AlreadyQueued { .. })
    }

    /// 是否被去重跳过。
    pub const fn deduped(&self) -> bool {
        matches!(self, Self::DuplicateSkipped { .. })
    }
}

/// 摄取端口 (生产接线把记忆保鲜回路的 `enqueue_ingest` 适配到此)。
///
/// 只读消费: 内容流不读记忆内部状态, 只投递并取回结果。
pub type IngestPort = Arc<dyn Fn(&IngestRequest) -> IngestOutcome + Send + Sync>;

/// 从闭包构造摄取端口 (测试/适配用)。
pub fn ingest_port<F>(f: F) -> IngestPort
where
    F: Fn(&IngestRequest) -> IngestOutcome + Send + Sync + 'static,
{
    Arc::new(f)
}

/// 一条摘要的稳定 source (`content_feed:<订阅 id>:<条目身份>`)。
///
/// 有条目链接用链接; 没链接退化到内容散列 ([`item_content_key`]) —— 同条目
/// 跨次拉取永远同一 source (散列门 + 条目账两道去重的第一维)。
pub fn item_source(feed_id: &str, item: &FeedItemSummary) -> String {
    format!("content_feed:{}:{}", feed_id, item_identity(item))
}

/// 条目身份: 条目链接优先, 缺链接退化到内容散列 (稳定, 不随拉取时刻变)。
pub fn item_identity(item: &FeedItemSummary) -> String {
    match item.item_link.as_deref() {
        Some(link) if !link.trim().is_empty() => format!("link:{link}"),
        _ => format!("hash:{:016x}", item_content_key(item)),
    }
}

/// 记忆摄取面的**凭据形状绊线** (M1): 一段**纯文本**摄取内容过形状层扫描。
///
/// # 口径 (与内核既有语义一致, 零可见变化优先)
///
/// * **零命中** ⇒ 内容**逐字节原样**返回 (不挂任何标记) ⇒ ⇒ **散列门的散列
///   不变** (散列门按内容散列去重 ⇒ 内容逐字节不变 ⇒ 散列逐字节不变 ⇒
///   同条目重复拉取仍是零新记忆);
/// * **命中** ⇒ 内容换成**遮形文本** (单向标签, 不可逆) + **末尾一行 JSON 注解**
///   (裸文本面挂不住结构化侧车 ⇒ 注解显形 ⇒ 命中不静默; 注解**零真值**:
///   只有命中族名, 没有凭据值)。
///
/// # 为什么在**摄取内容构造**这一层扫
///
/// 摄取内容**直接进 episodic 记忆** ⇒ ⇒ 那是**凭据进存储**的路径 (不是进
/// transcript) ⇒ ⇒ 比 transcript 更「深」⇒ ⇒ 真 key 不许进存储 ⇒ ⇒ 在**入库
/// 之前**扫。
pub(crate) fn mask_ingest_content(content: &str) -> String {
    let scan = scan_result_face(content);
    if scan.leaked_kinds.is_empty() {
        // 零命中 ⇒ 逐字节原样 (内核助手在干净面本就原样回传; 这里再钉一次口径:
        // 散列门依赖内容稳定 ⇒ 遮形面必须对干净内容**零行为变更**)。
        return scan.text;
    }
    // 命中 ⇒ 遮形文本 + 末尾一行 JSON 注解 (命中不静默; 注解零真值)。
    append_credential_tripwire(&scan.text, &scan.leaked_kinds)
}

/// 投摄取的稳定内容 (标题 + 正文 + 来源 + 时间原文; **不含拉取时刻**)。
///
/// 拉取时刻每次拉取都变, 若进内容则散列门无法按内容去重; 故摄取内容只取
/// 来源稳定字段, 同条目重复拉取产生同散列 → 散列门跳过 (内容级去重)。
///
/// # 记忆摄取面的凭据形状绊线 (批次 3b)
///
/// 本函数产出的内容**直接进 episodic 记忆** ⇒ ⇒ 它是**凭据进存储**的面
/// ⇒ ⇒ 内容在**入库之前**过一道形状层扫描 ([`mask_ingest_content`]):
/// 零命中 ⇒ **逐字节不变** (⇒ **散列不变**, 去重语义不破); 命中 ⇒ 遮形 +
/// 末尾一行 JSON 注解 (命中不静默)。
///
/// # ⚠️ 散列输入与入库内容是**同一份**字符串 (读码结论)
///
/// 记忆库的 `enqueue_ingest` 用**同一个** `content` 既**算散列** (散列门查门)
/// 又**落库** (待摄取清单的正文列, 之后折入 episodic 记忆) ⇒ ⇒ 本面**没有**
/// 「只遮入库、不改散列」的接缝点 (那要改记忆库那一侧, 不在本面写域内)
/// ⇒ ⇒ 遮形**必然同时**改这两者。
///
/// ⇒ ⇒ **零命中 ⇒ 两者都不变** (散列逐字节不变 ⇒ 同条目重复拉取仍被散列门
/// 跳过 ⇒ 零新记忆); ⇒ ⇒ **命中 ⇒ 散列随之改变**, 但遮形是**确定性**的
/// ⇒ ⇒ 同条目重复拉取仍产生**同一散列** ⇒ ⇒ 散列门的去重语义**不破**。
pub fn stable_ingest_content(item: &FeedItemSummary) -> String {
    let raw = format!(
        "{}\n{}\n{}\n{}",
        item.title,
        item.body_excerpt,
        item.item_link.as_deref().unwrap_or(""),
        item.published_raw.as_deref().unwrap_or("")
    );
    mask_ingest_content(&raw)
}

/// 条目内容散列 (FNV-1a 64, 与保鲜回路散列门同族; 自含, 不引新依赖)。
fn item_content_key(item: &FeedItemSummary) -> u64 {
    fnv1a64(&format!("{}\n{}", item.title, item.body_excerpt))
}

/// FNV-1a 64 (标准参数), 与记忆保鲜回路散列门同口径 (复用同算法, 不重复造)。
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

#[cfg(test)]
mod tests {
    use super::*;

    fn item(title: &str, body: &str, link: Option<&str>) -> FeedItemSummary {
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

    /// 幂等账身份: 有链接用链接, 同条目跨次拉取同 identity (随拉取时刻不变)。
    #[test]
    fn item_identity_prefers_link_and_is_stable_across_fetch_times() {
        let mut a = item("标题", "正文", Some("https://example.com/1"));
        let mut b = item("标题", "正文", Some("https://example.com/1"));
        b.fetched_at_ms = 1_800_000_000_000; // 换一次拉取时刻
        assert_eq!(item_identity(&a), item_identity(&b), "同链接条目身份稳定");
        assert!(item_identity(&a).starts_with("link:"));
        a.item_link = None;
        assert!(
            item_identity(&a).starts_with("hash:"),
            "缺链接退化到内容散列"
        );
    }

    /// 缺链接条目按内容派身份: 同内容同身份, 异内容异身份。
    #[test]
    fn identity_falls_back_to_content_hash_when_link_is_absent() {
        let a = item("标题", "正文", None);
        let b = item("标题", "正文", None);
        let c = item("标题", "改了正文", None);
        assert_eq!(item_identity(&a), item_identity(&b));
        assert_ne!(item_identity(&a), item_identity(&c));
    }

    /// 稳定摄取内容不含拉取时刻 —— 否则散列门无法按内容去重。
    #[test]
    fn stable_ingest_content_excludes_the_fetch_time() {
        let mut a = item("标题", "正文", Some("https://example.com/1"));
        let mut b = a.clone();
        b.fetched_at_ms = 9_999_999_999_999;
        assert_eq!(
            stable_ingest_content(&a),
            stable_ingest_content(&b),
            "同条目不同拉取时刻摄取内容必须一致 (散列门按内容去重的前提)"
        );
        a.fetched_at_ms = 1;
        assert!(!stable_ingest_content(&a).contains("1700000000000"));
    }

    /// source = 订阅 id + 条目身份 (拉取→摄取接缝的去重第一维)。
    #[test]
    fn item_source_is_namespaced_by_feed() {
        let a = item("标题", "正文", Some("https://example.com/1"));
        assert_eq!(
            item_source("news", &a),
            "content_feed:news:link:https://example.com/1"
        );
    }

    /// 零命中 ⇒ 摄取内容**逐字节不变** ⇒ ⇒ 散列门的散列**不变** (去重不破)。
    #[test]
    fn clean_ingest_content_is_byte_identical_to_the_legacy_layout() {
        let a = item("标题", "正文", Some("https://example.com/1"));
        let legacy = format!(
            "{}\n{}\n{}\n{}",
            a.title,
            a.body_excerpt,
            a.item_link.as_deref().unwrap_or(""),
            a.published_raw.as_deref().unwrap_or("")
        );
        assert_eq!(
            stable_ingest_content(&a),
            legacy,
            "干净内容必须逐字节不变 (散列门依赖内容稳定 ⇒ 遮形面必须零行为变更)"
        );
        assert_eq!(
            fnv1a64(&stable_ingest_content(&a)),
            fnv1a64(&legacy),
            "零命中 ⇒ 散列逐字节不变"
        );
    }

    /// 命中 ⇒ 真 key 不进摄取内容 (凭据进存储的面被挡住) + 命中可观测。
    #[test]
    fn credential_shaped_body_is_masked_before_it_reaches_memory() {
        let secret = "sk-abcdefghijklmnopqrstuvwxyz0123456789";
        let a = item(
            "标题",
            &format!("正文含密钥 {secret}"),
            Some("https://example.com/1"),
        );
        let content = stable_ingest_content(&a);
        assert!(
            !content.contains(secret),
            "真 key 不得进摄取内容: {content}"
        );
        assert!(
            content.contains("[REDACTED_OPENAI_KEY]"),
            "命中必须换成遮形标签: {content}"
        );
        assert!(
            content.contains("credential_tripwire"),
            "命中必须可观测 (裸文本面 ⇒ 末尾一行 JSON 注解): {content}"
        );
    }

    /// 命中 ⇒ 遮形是**确定性**的 ⇒ 同条目重复拉取仍产生同一散列 ⇒ 去重语义不破。
    #[test]
    fn masked_ingest_content_is_deterministic_across_fetch_times() {
        let secret = "sk-abcdefghijklmnopqrstuvwxyz0123456789";
        let a = item(
            "标题",
            &format!("正文含密钥 {secret}"),
            Some("https://example.com/1"),
        );
        let mut b = a.clone();
        b.fetched_at_ms = 9_999_999_999_999;
        assert_eq!(
            stable_ingest_content(&a),
            stable_ingest_content(&b),
            "遮形后仍须逐字相同 (散列门按内容散列去重 ⇒ 同条目重复拉取零新记忆)"
        );
    }

    /// 注意力上限默认值 + 覆盖 (截断口径的输入面)。
    #[test]
    fn attention_limits_default_and_override() {
        let limits = AttentionLimits::default();
        assert_eq!(limits.per_pull_items, DEFAULT_PER_PULL_ITEMS);
        assert_eq!(limits.daily_ingests, DEFAULT_DAILY_INGESTS);
        let custom = AttentionLimits::default()
            .with_per_pull_items(2)
            .with_daily_ingests(3);
        assert_eq!(custom.per_pull_items, 2);
        assert_eq!(custom.daily_ingests, 3);
    }

    /// 摄取结果态判定 (queued / deduped) 如实。
    #[test]
    fn ingest_outcome_flags_are_faithful() {
        assert!(IngestOutcome::Enqueued {
            task_id: "t".into()
        }
        .queued());
        assert!(IngestOutcome::AlreadyQueued {
            task_id: "t".into()
        }
        .queued());
        assert!(IngestOutcome::DuplicateSkipped { skip_count: 1 }.deduped());
        assert!(!IngestOutcome::Failed { error: "x".into() }.queued());
    }
}
