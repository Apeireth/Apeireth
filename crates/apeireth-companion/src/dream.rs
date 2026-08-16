//! `apeireth-companion::dream` — 做梦机制落地: 周期调度 + 合并写回真库.
//!
//! 真实机制 (0 假装):
//! - [`SleepCycle`] 判定「该做梦了」(安静期 / 条目数; 注入时钟, 虚拟可快进)
//! - 拉 session 的近期记忆条目 (episodes), 交给 [`DreamSubsystem::dream_cycle`] 成对合并
//! - 合并结果作为新 episode **写回真 SQLite** (append-only, 前缀 `【做梦整合】`)
//! - 周期结束 `reset_after_cycle`, 等待下一夜
//!
//! 诚实标注:
//! - 当前合并 = 真实字符串拼接 (机制真做); **语义摘要化 (LLM 提炼) 是下一步, 未假装**
//! - 只整合记忆条目, 不碰对话/自由文本 (与宪法评审「只审动作」同哲学)

use std::sync::Arc;

use apeireth_core::clock::Clock;
use apeireth_memory::lightmemo::{DreamSubsystem, SleepCycle};
use apeireth_memory::{CoreEpisode, EpisodeStore, SqliteMemoryStore};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// 做梦摘要化 (LLM 提炼): 把合并结果 (拼接) 提炼成一条语义摘要.
/// lib 不依赖 `apeireth-api`, 真实现由调用方注入 (同 judicator 的 trait 策略).
#[async_trait]
pub trait DreamSummarizer: Send + Sync {
    /// 输入成对合并后的拼接文本, 返回提炼后的摘要 (单条, <= 200 字).
    async fn summarize(&self, merged: &str) -> Result<String, String>;
}

/// 文本重叠率 (公共字符比例, 简单近似).
fn overlap_ratio(a: &str, b: &str) -> f64 {
    let sa: std::collections::HashSet<char> = a.chars().collect();
    let sb: std::collections::HashSet<char> = b.chars().collect();
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// 做梦调度器: 周期触发 → 成对合并记忆 → (可选 LLM 摘要) → 写回真库.
pub struct DreamScheduler {
    store: Arc<SqliteMemoryStore>,
    clock: Arc<dyn Clock>,
    sleep: SleepCycle,
    dream: DreamSubsystem,
    session: String,
    merge_batch: usize,
    /// 可选摘要器: 配置后合并结果经 LLM 提炼再写回; 未配置保持拼接 (诚实降级).
    summarizer: Option<Arc<dyn DreamSummarizer>>,
    /// 上次做梦时刻 (增量合并边界: 只合并此后的记忆, 防旧记忆反复合并/摘要嵌套).
    last_cycle_at: std::sync::Mutex<DateTime<Utc>>,
}

impl DreamScheduler {
    /// 文本级近重复去重 (记忆 v2, Letta sleep-time 吸收):
    /// 归一化后相同 / 互为子串 / 长文本重叠率 > 0.8 → 保留较长者.
    /// 0 假装: 非 embedding 语义去重 (后续可接向量).
    fn dedup_textual(items: &mut Vec<String>) {
        let norm = |s: &str| {
            s.chars()
                .filter(|c| !c.is_whitespace())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        };
        let mut i = 0;
        while i < items.len() {
            let a = norm(&items[i]);
            let mut j = i + 1;
            while j < items.len() {
                let b = norm(&items[j]);
                let dup = a == b
                    || (a.len() >= 20 && b.len() >= 20
                        && (a.contains(&b) || b.contains(&a) || overlap_ratio(&a, &b) > 0.8));
                if dup {
                    // 保留较长者
                    if items[i].chars().count() >= items[j].chars().count() {
                        items.remove(j);
                    } else {
                        items.remove(i);
                        break; // i 指向的元素变了, 重新比较
                    }
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
    pub fn new(store: Arc<SqliteMemoryStore>, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            clock: Arc::clone(&clock),
            sleep: SleepCycle::with_clock(clock),
            dream: DreamSubsystem::new(),
            session: "me".into(),
            merge_batch: 20,
            summarizer: None,
            last_cycle_at: std::sync::Mutex::new(DateTime::<Utc>::from_timestamp(0, 0).unwrap_or(Utc::now())),
        }
    }

    /// 接 LLM 摘要器: 合并结果经提炼写回 (未接则保持拼接, 诚实降级).
    pub fn with_summarizer(mut self, s: Arc<dyn DreamSummarizer>) -> Self {
        self.summarizer = Some(s);
        self
    }

    /// 覆盖安静期阈值 (默认 60s). daemon 场景建议调大 (如 6h) — 「睡觉」语义:
    /// 长时间无互动才做梦, 用户活动会重置安静期.
    pub fn with_quiet_threshold(mut self, quiet: std::time::Duration) -> Self {
        self.sleep.set_quiet_threshold(quiet);
        self
    }

    /// 覆盖记忆 session (默认 "me").
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = session.into();
        self
    }

    /// 覆盖单次最多拉取条目数 (默认 20).
    pub fn with_merge_batch(mut self, n: usize) -> Self {
        self.merge_batch = n.max(2);
        self
    }

    /// 记录一次新记忆写入 (喂 SleepCycle 条目计数).
    pub fn record_item_added(&self) {
        self.sleep.record_item_added();
    }

    /// 记录一次用户活动 (重置安静期).
    pub fn record_activity(&self) {
        self.sleep.record_activity();
    }

    /// 每 tick 调用: 该做梦 → 合并并写回真库; 返回本次写回的合并条数.
    /// 不做梦 → 0. 永不 panic (记忆库错误只记 stderr, 不影响主循环).
    pub async fn tick(&self) -> usize {
        if !self.sleep.should_consolidate() {
            return 0;
        }
        let eps = self
            .store
            .recent_episodes(&self.session, self.merge_batch)
            .unwrap_or_default();
        // 增量合并: 只合并上次做梦之后的记忆; 且不重复整合旧做梦结果 (防摘要嵌套摘要)
        let boundary = *self.last_cycle_at.lock().expect("poisoned");
        let mut items: Vec<String> = eps
            .iter()
            .filter(|e| !e.id.starts_with("mem-dream-"))
            .filter(|e| chrono::DateTime::<Utc>::from_timestamp(e.timestamp, 0).map_or(true, |t| t >= boundary))
            .map(|e| e.content.clone())
            .collect();
        // 记忆 v2 (Letta sleep-time 吸收): 合并前去重 (文本级近重复, 非 embedding — 诚实)
        Self::dedup_textual(&mut items);
        let merged = std::cell::RefCell::new(Vec::new());
        let n = self.dream.dream_cycle(&items, &|a, b| {
            let m = format!("{a} ◆ {b}");
            merged.borrow_mut().push(m.clone());
            m
        });
        let now_ts = self.clock.now().timestamp();
        for m in merged.borrow().iter() {
            // 可选: LLM 摘要化 (提炼拼接结果); 失败 → 保持拼接 (诚实降级, 不丢内容)
            let content = match &self.summarizer {
                Some(s) => match s.summarize(m).await {
                    Ok(summary) => format!("【做梦摘要】{summary}"),
                    Err(e) => {
                        eprintln!("[dream] 摘要失败, 保持拼接: {e}");
                        format!("【做梦整合】{m}")
                    }
                },
                None => format!("【做梦整合】{m}"),
            };
            let ep = CoreEpisode {
                id: format!("mem-dream-{}", uuid::Uuid::new_v4()),
                timestamp: now_ts,
                role: "assistant".into(),
                content,
                session_id: self.session.clone(),
            };
            if let Err(e) = self.store.put_episode(&ep) {
                eprintln!("[dream] 写回记忆失败: {e}");
            }
        }
        self.sleep.reset_after_cycle();
        *self.last_cycle_at.lock().expect("poisoned") = self.clock.now();
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use chrono::{TimeZone, Utc};

    fn seed(store: &Arc<SqliteMemoryStore>) {
        for (i, c) in [
            "线代: 特征值最后一题卡住",
            "高数: 换元忘换 dx",
            "明天交线代作业",
            "council bug: advisor 低频误报",
        ]
        .iter()
        .enumerate()
        {
            store
                .put_episode(&CoreEpisode {
                    id: format!("mem-{i}"),
                    timestamp: 1 + i as i64,
                    role: "assistant".into(),
                    content: c.to_string(),
                    session_id: "me".into(),
                })
                .unwrap();
        }
    }

    #[tokio::test]
    async fn dream_merges_and_writes_back_to_real_db() {
        let store = Arc::new(apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap());
        seed(&store);
        let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
        let sched = DreamScheduler::new(Arc::clone(&store), Arc::new(vc.clone()));
        // 安静期未到 → 不做梦
        assert_eq!(sched.tick().await, 0);
        // 快进 61s → 做梦: 4 条成对合并 → 2 条写回
        vc.advance(chrono::Duration::seconds(61));
        let n = sched.tick().await;
        assert_eq!(n, 2, "4 条应合并成 2 对");
        let eps = store.recent_episodes("me", 100).unwrap();
        let dreams: Vec<_> = eps.iter().filter(|e| e.id.starts_with("mem-dream-")).collect();
        assert_eq!(dreams.len(), 2, "合并结果应写回真库");
        assert!(dreams[0].content.contains("◆"), "合并应为拼接: {}", dreams[0].content);
        // reset 后不再做
        assert_eq!(sched.tick().await, 0);
        // 第二夜: 增量合并语义 (2026-08-16) — 无新记忆 → 0 (旧记忆不再反复合并)
        vc.advance(chrono::Duration::seconds(61));
        assert_eq!(sched.tick().await, 0, "增量合并: 无新记忆不重复整合");
    }

    #[tokio::test]
    async fn dream_skips_old_dream_results() {
        // 修 bug (2026-08-16 实测发现): 旧做梦结果 mem-dream-* 被再次合并 → 摘要嵌套摘要
        let store = Arc::new(apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap());
        seed(&store);
        let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
        let sched = DreamScheduler::new(Arc::clone(&store), Arc::new(vc.clone()));
        vc.advance(chrono::Duration::seconds(61));
        let n1 = sched.tick().await;
        assert_eq!(n1, 2, "第一次: 4 条 mem-* → 2 对");
        // 第二夜: 只剩 mem-dream-* (旧做梦结果) 在最近 20 条内 → 应跳过, 不再嵌套合并
        vc.advance(chrono::Duration::seconds(61));
        let n2 = sched.tick().await;
        assert_eq!(n2, 0, "旧做梦结果不应被再次合并 (防摘要嵌套)");
        let eps = store.recent_episodes("me", 100).unwrap();
        let dreams: Vec<_> = eps.iter().filter(|e| e.id.starts_with("mem-dream-")).collect();
        assert_eq!(dreams.len(), 2, "做梦结果不增");
    }

    struct StubSummarizer;
    #[async_trait]
    impl DreamSummarizer for StubSummarizer {
        async fn summarize(&self, merged: &str) -> Result<String, String> {
            let head: String = merged.chars().take(10).collect();
            Ok(format!("提炼自: {head}"))
        }
    }

    struct FailingSummarizer;
    #[async_trait]
    impl DreamSummarizer for FailingSummarizer {
        async fn summarize(&self, _m: &str) -> Result<String, String> {
            Err("LLM suppressed".into())
        }
    }

    #[tokio::test]
    async fn dream_with_summarizer_writes_summary() {
        let store = Arc::new(apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap());
        seed(&store);
        let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
        let sched = DreamScheduler::new(Arc::clone(&store), Arc::new(vc.clone()))
            .with_summarizer(Arc::new(StubSummarizer));
        vc.advance(chrono::Duration::seconds(61));
        assert_eq!(sched.tick().await, 2);
        let eps = store.recent_episodes("me", 100).unwrap();
        let dreams: Vec<_> = eps.iter().filter(|e| e.id.starts_with("mem-dream-")).collect();
        assert_eq!(dreams.len(), 2);
        assert!(dreams[0].content.starts_with("【做梦摘要】"), "应写摘要: {}", dreams[0].content);
    }

    #[tokio::test]
    async fn dream_summarizer_failure_falls_back_to_merged() {
        let store = Arc::new(apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap());
        seed(&store);
        let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
        let sched = DreamScheduler::new(Arc::clone(&store), Arc::new(vc.clone()))
            .with_summarizer(Arc::new(FailingSummarizer));
        vc.advance(chrono::Duration::seconds(61));
        assert_eq!(sched.tick().await, 2);
        let eps = store.recent_episodes("me", 100).unwrap();
        let dreams: Vec<_> = eps.iter().filter(|e| e.id.starts_with("mem-dream-")).collect();
        // 摘要失败 → 诚实降级为拼接, 不丢内容
        assert!(dreams[0].content.starts_with("【做梦整合】"), "应降级拼接: {}", dreams[0].content);
    }
}


