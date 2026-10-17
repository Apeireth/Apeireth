//! 订阅生命周期面 (J2): 条目去重账 + 拉取失败退避 + 订阅健康度 + 注意力日计数。
//!
//! # 与散列门的分工 (两道去重合防刷屏)
//!
//! 记忆保鲜回路的**散列门**按内容去重 (同源同散列跳过); 本面补**条目级幂等**
//! (已摄取条目账, 按 [`crate::content_feed::ingest::item_source`] 的稳定
//! source 记账) —— 同条目跨次拉取即使正文略有出入 (来源改了时间戳原文等),
//! 也因条目身份相同而跳过。两道合起来 = 同条目重复拉取**零新记忆**。
//!
//! # 条目账有界 (如实)
//!
//! 已摄取条目账是**有界 FIFO** ([`ITEM_LEDGER_MAX`] 条): 满了淘汰最旧。淘汰
//! 后极端情况下同条目可能重摄, 但内容级散列门仍兜底; 这是成本/完备的如实
//! 取舍, 不假装账无限。
//!
//! # 退避 + 健康度
//!
//! 拉取失败 → [`FeedHealth::consecutive_failures`] +1, 退避窗口按指数放大
//! ([`LifecyclePolicy::backoff_base_ms`] × 2^失败次数, 有上限), 到点前该订阅
//! **不再拉取** ([`FeedLifecycleBody::pull_due`]); 连续失败达
//! [`LifecyclePolicy::degraded_after_failures`] 次如实标 `degraded`。成功即复位。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat};
use serde::{Deserialize, Serialize};

/// 条目账文件名 (数据目录, 与 `feeds.json` 同目录)。
pub const FEED_LIFECYCLE_FILE_NAME: &str = "feed_lifecycle.json";
/// 条目账文档身份 (StoredDoc 信封 name)。
pub const FEED_LIFECYCLE_DOC_NAME: &str = "apeireth.feed_lifecycle";
/// 条目账格式版本戳。
pub const FEED_LIFECYCLE_DOC_VERSION: u32 = 1;
/// 已摄取条目账上限 (有界 FIFO, 淘汰最旧)。
pub const ITEM_LEDGER_MAX: usize = 4_096;

/// 生命周期策略 (退避基量 / 健康阈值 / 账上限)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecyclePolicy {
    /// 连续失败多少次标 `degraded`。
    pub degraded_after_failures: u32,
    /// 退避基量 (毫秒): 第 n 次失败后退避 = base × 2^(n-1)。
    pub backoff_base_ms: i64,
    /// 退避指数上限 (防退避无限放大)。
    pub backoff_max_exponent: u32,
    /// 已摄取条目账上限。
    pub item_ledger_max: usize,
}

impl Default for LifecyclePolicy {
    fn default() -> Self {
        Self {
            degraded_after_failures: 3,
            backoff_base_ms: 60_000,
            backoff_max_exponent: 6,
            item_ledger_max: ITEM_LEDGER_MAX,
        }
    }
}

/// 一条订阅的健康度 + 退避态。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedHealth {
    /// 连续失败计数 (成功清零)。
    #[serde(default)]
    pub consecutive_failures: u32,
    /// 退避窗口截止 (epoch 毫秒); `None` = 无退避, 可拉。
    #[serde(default)]
    pub next_pull_after_ms: Option<i64>,
    /// 是否标 `degraded` (连续失败达阈值, 如实)。
    #[serde(default)]
    pub degraded: bool,
    /// 最近失败真因 (`None` = 未失败/已复位)。
    #[serde(default)]
    pub last_error: Option<String>,
    /// 最近一次拉取尝试时刻 (epoch 毫秒)。
    #[serde(default)]
    pub last_attempt_ms: Option<i64>,
}

/// 注意力日计数 (按 epoch 日重置; 跨日归零)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyCounter {
    /// 当前计数所属 epoch 日 (`now_ms / 86_400_000`)。
    #[serde(default)]
    pub epoch_day: i64,
    /// 当日已摄取条目数。
    #[serde(default)]
    pub ingested: u64,
}

/// 生命周期主体 (条目账 + 逐订阅健康 + 累计计数)。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedLifecycleBody {
    /// 逐订阅健康度 + 退避 (按订阅 id)。
    #[serde(default)]
    pub feeds: BTreeMap<String, FeedHealth>,
    /// 已摄取条目账 (稳定 source 的有界 FIFO, 淘汰最旧)。
    #[serde(default)]
    pub items: Vec<String>,
    /// 条目级去重累计 (同条目重复拉取跳过次数真值)。
    #[serde(default)]
    pub dedup_skipped: u64,
    /// 投摄取队列累计 (交进待摄取队列的条目真值)。
    #[serde(default)]
    pub ingest_queued: u64,
    /// 注意力上限截断累计 (超单次/每日上限如实截断的条目数)。
    #[serde(default)]
    pub attention_truncated: u64,
    /// 注意力日计数。
    #[serde(default)]
    pub daily: DailyCounter,
}

/// 生命周期自述真值 (喂 self_status 内容流节)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LifecycleStats {
    /// 条目级去重累计。
    pub dedup_skipped: u64,
    /// 投摄取队列累计。
    pub ingest_queued: u64,
    /// 注意力截断累计。
    pub attention_truncated: u64,
    /// 标 degraded 的订阅数。
    pub degraded_feeds: u64,
    /// 退避中的订阅数。
    pub backing_off_feeds: u64,
}

impl FeedLifecycleBody {
    /// 条目是否已摄取 (条目账查重)。
    pub fn item_ingested(&self, source: &str) -> bool {
        self.items.iter().any(|item| item == source)
    }

    /// 记一条已摄取条目 (有界 FIFO + 日计数 +1)。
    pub fn mark_item(&mut self, source: &str, now_ms: i64, policy: &LifecyclePolicy) {
        if !self.item_ingested(source) {
            self.items.push(source.to_string());
            let max = policy.item_ledger_max.max(1);
            while self.items.len() > max {
                self.items.remove(0);
            }
        }
        self.roll_daily(now_ms);
        self.daily.ingested = self.daily.ingested.saturating_add(1);
    }

    /// 日计数跨日归零。
    fn roll_daily(&mut self, now_ms: i64) {
        let day = epoch_day(now_ms);
        if self.daily.epoch_day != day {
            self.daily.epoch_day = day;
            self.daily.ingested = 0;
        }
    }

    /// 当日剩余摄取预算 (跨日按满额)。
    pub fn daily_remaining(&mut self, now_ms: i64, cap: usize) -> usize {
        self.roll_daily(now_ms);
        cap.saturating_sub(self.daily.ingested as usize)
    }

    /// 条目级去重计数 +1。
    pub fn note_dedup(&mut self) {
        self.dedup_skipped = self.dedup_skipped.saturating_add(1);
    }

    /// 投摄取队列计数 +1。
    pub fn note_ingest_queued(&mut self) {
        self.ingest_queued = self.ingest_queued.saturating_add(1);
    }

    /// 注意力截断计数 +n。
    pub fn note_attention_truncated(&mut self, n: u64) {
        self.attention_truncated = self.attention_truncated.saturating_add(n);
    }

    /// 该订阅此刻是否可拉 (退避窗口内 = 不拉, 防锤失败源)。
    pub fn pull_due(&self, feed_id: &str, now_ms: i64) -> bool {
        self.feeds
            .get(feed_id)
            .and_then(|health| health.next_pull_after_ms)
            .map_or(true, |after| now_ms >= after)
    }

    /// 记一次拉取成功 (清零连续失败 + 复位退避 + 退出 degraded)。
    pub fn record_feed_success(&mut self, feed_id: &str, now_ms: i64) {
        let health = self.feeds.entry(feed_id.to_string()).or_default();
        health.consecutive_failures = 0;
        health.next_pull_after_ms = None;
        health.degraded = false;
        health.last_error = None;
        health.last_attempt_ms = Some(now_ms);
    }

    /// 记一次拉取失败 (连续失败 +1 + 退避放大 + 达阈值标 degraded)。
    pub fn record_feed_failure(
        &mut self,
        feed_id: &str,
        now_ms: i64,
        error: &str,
        policy: &LifecyclePolicy,
    ) {
        let health = self.feeds.entry(feed_id.to_string()).or_default();
        health.consecutive_failures = health.consecutive_failures.saturating_add(1);
        let exponent =
            (health.consecutive_failures.saturating_sub(1)).min(policy.backoff_max_exponent);
        let backoff = policy
            .backoff_base_ms
            .saturating_mul(1i64 << exponent.min(62));
        health.next_pull_after_ms = Some(now_ms.saturating_add(backoff));
        health.degraded = health.consecutive_failures >= policy.degraded_after_failures;
        health.last_error = Some(error.to_string());
        health.last_attempt_ms = Some(now_ms);
    }

    /// 订阅健康度自述真值。
    pub fn stats(&self, now_ms: i64) -> LifecycleStats {
        let mut degraded_feeds = 0u64;
        let mut backing_off_feeds = 0u64;
        for health in self.feeds.values() {
            if health.degraded {
                degraded_feeds += 1;
            }
            if health
                .next_pull_after_ms
                .is_some_and(|after| now_ms < after)
            {
                backing_off_feeds += 1;
            }
        }
        LifecycleStats {
            dedup_skipped: self.dedup_skipped,
            ingest_queued: self.ingest_queued,
            attention_truncated: self.attention_truncated,
            degraded_feeds,
            backing_off_feeds,
        }
    }
}

/// 订阅生命周期存储 (数据目录 `feed_lifecycle.json`, StoredDoc 拒开语义)。
#[derive(Debug, Clone)]
pub struct FeedLifecycleStore {
    path: PathBuf,
}

impl FeedLifecycleStore {
    /// 绑定数据目录 (与 `feeds.json` 同目录)。
    pub fn for_data_dir(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            path: data_dir.into().join(FEED_LIFECYCLE_FILE_NAME),
        }
    }

    /// 从 `feeds.json` 路径派生同目录生命周期账 (自述探测口同源)。
    pub fn beside_feeds(feeds_path: &Path) -> Self {
        let dir = feeds_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        Self::for_data_dir(dir)
    }

    /// 账文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 读端版本契约 (只认当前版本)。
    fn compat() -> DocCompat {
        DocCompat::exact(FEED_LIFECYCLE_DOC_NAME, FEED_LIFECYCLE_DOC_VERSION)
    }

    /// 装载生命周期主体: 文件缺失 = 空态默认; 坏账拒开 (不静默修复)。
    pub fn load(&self) -> Result<FeedLifecycleBody, String> {
        if !self.path.exists() {
            return Ok(FeedLifecycleBody::default());
        }
        let doc = stored_doc::open_single::<FeedLifecycleBody>(&self.path, &Self::compat())
            .map_err(|error| format!("{FEED_LIFECYCLE_FILE_NAME} refused to open: {error}"))?;
        Ok(doc.body)
    }

    /// 落盘生命周期主体 (持久档原子写)。
    pub fn save(&self, body: &FeedLifecycleBody) -> Result<(), String> {
        stored_doc::save_single(
            &self.path,
            &Self::compat(),
            body,
            stored_doc::DEFAULT_DOC_MODE,
        )
        .map_err(|error| format!("{FEED_LIFECYCLE_FILE_NAME} write failed: {error}"))?;
        Ok(())
    }

    /// 自述真值 (只读, 逐维缺源如实)。
    pub fn stats(&self, now_ms: i64) -> Result<LifecycleStats, String> {
        Ok(self.load()?.stats(now_ms))
    }
}

/// epoch 日 (`now_ms / 86_400_000`)。
fn epoch_day(now_ms: i64) -> i64 {
    now_ms.div_euclid(86_400_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> LifecyclePolicy {
        LifecyclePolicy::default()
    }

    /// 幂等账: 同 source 二次标记不重复计账, 条目只记一行。
    #[test]
    fn item_ledger_is_idempotent_per_source() {
        let mut body = FeedLifecycleBody::default();
        let policy = policy();
        body.mark_item("src-a", 1_000, &policy);
        body.mark_item("src-a", 2_000, &policy);
        assert!(body.item_ingested("src-a"));
        assert_eq!(body.items.len(), 1, "同 source 只记一行");
        assert_eq!(body.daily.ingested, 2, "日计数照实累计 (跨条目)");
    }

    /// 条目账有界: 超上限淘汰最旧 (如实, 不假装账无限)。
    #[test]
    fn item_ledger_is_bounded_and_evicts_oldest() {
        let mut body = FeedLifecycleBody::default();
        let policy = LifecyclePolicy {
            item_ledger_max: 2,
            ..LifecyclePolicy::default()
        };
        body.mark_item("a", 1, &policy);
        body.mark_item("b", 2, &policy);
        body.mark_item("c", 3, &policy);
        assert_eq!(body.items, vec!["b".to_string(), "c".to_string()]);
        assert!(!body.item_ingested("a"), "最旧被淘汰");
    }

    /// 退避放大: 连续失败逐次拉长退避窗口, 达阈值标 degraded 如实。
    #[test]
    fn backoff_grows_on_consecutive_failures_and_degrades() {
        let mut body = FeedLifecycleBody::default();
        let policy = policy();
        assert!(body.pull_due("news", 0), "从未拉取 = 可拉");

        body.record_feed_failure("news", 1_000, "boom", &policy);
        let first = body.feeds["news"].next_pull_after_ms.unwrap();
        assert_eq!(first, 1_000 + policy.backoff_base_ms);
        assert!(!body.pull_due("news", 1_000), "失败后退避窗口内不拉");
        assert!(body.pull_due("news", first), "退避到点可拉");
        assert!(!body.feeds["news"].degraded, "一次失败不标 degraded");

        body.record_feed_failure("news", 2_000, "boom", &policy);
        let second = body.feeds["news"].next_pull_after_ms.unwrap();
        assert_eq!(second, 2_000 + policy.backoff_base_ms * 2, "退避放大");
        assert!(!body.feeds["news"].degraded);

        body.record_feed_failure("news", 3_000, "boom", &policy);
        assert!(body.feeds["news"].degraded, "连续 3 次失败标 degraded 如实");
    }

    /// 成功复位: 连续失败清零 + 退避解除 + 退出 degraded。
    #[test]
    fn success_resets_backoff_and_degraded() {
        let mut body = FeedLifecycleBody::default();
        let policy = policy();
        body.record_feed_failure("news", 1_000, "boom", &policy);
        body.record_feed_failure("news", 2_000, "boom", &policy);
        body.record_feed_success("news", 3_000);
        let health = &body.feeds["news"];
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.next_pull_after_ms, None);
        assert!(!health.degraded);
        assert_eq!(health.last_error, None);
        assert!(body.pull_due("news", 3_000));
    }

    /// 日计数跨日归零 + 每日剩余预算。
    #[test]
    fn daily_counter_rolls_over_and_tracks_remaining() {
        let mut body = FeedLifecycleBody::default();
        let policy = policy();
        let day_ms = 86_400_000;
        body.mark_item("a", day_ms * 10 + 5, &policy);
        assert_eq!(body.daily_remaining(day_ms * 10 + 6, 2), 1);
        assert_eq!(body.daily_remaining(day_ms * 11 + 1, 2), 2, "跨日满额");
    }

    /// 自述真值: degraded/退避计数如实。
    #[test]
    fn stats_counts_degraded_and_backing_off_honestly() {
        let mut body = FeedLifecycleBody::default();
        let policy = policy();
        body.record_feed_failure("a", 1_000, "x", &policy);
        body.record_feed_failure("b", 1_000, "x", &policy);
        body.record_feed_failure("b", 1_000, "x", &policy);
        body.record_feed_failure("b", 1_000, "x", &policy);
        body.note_dedup();
        body.note_ingest_queued();
        body.note_attention_truncated(2);
        let stats = body.stats(1_000);
        assert_eq!(stats.dedup_skipped, 1);
        assert_eq!(stats.ingest_queued, 1);
        assert_eq!(stats.attention_truncated, 2);
        assert_eq!(stats.degraded_feeds, 1, "只有 b 连败 3 次");
        assert_eq!(stats.backing_off_feeds, 2, "a、b 都在退避窗口内");
    }

    /// 存储往返: 落盘再读回一致; 缺文件 = 空态。
    #[test]
    fn lifecycle_store_round_trips_and_missing_is_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FeedLifecycleStore::for_data_dir(dir.path());
        assert_eq!(
            store.load().expect("empty"),
            FeedLifecycleBody::default(),
            "缺文件 = 空态"
        );
        let mut body = FeedLifecycleBody::default();
        body.mark_item("src", 1_000, &policy());
        store.save(&body).expect("save");
        assert_eq!(store.load().expect("load"), body);
    }

    /// 坏账拒开 (不静默修复)。
    #[test]
    fn broken_lifecycle_file_refuses_to_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = FeedLifecycleStore::for_data_dir(dir.path());
        std::fs::write(store.path(), b"{ not json").expect("write");
        assert!(store.load().is_err(), "坏账必须拒开");
    }
}
