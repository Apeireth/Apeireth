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

/// 做梦调度器: 周期触发 → 成对合并记忆 → 写回真库.
pub struct DreamScheduler {
    store: Arc<SqliteMemoryStore>,
    clock: Arc<dyn Clock>,
    sleep: SleepCycle,
    dream: DreamSubsystem,
    session: String,
    merge_batch: usize,
}

impl DreamScheduler {
    pub fn new(store: Arc<SqliteMemoryStore>, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            clock: Arc::clone(&clock),
            sleep: SleepCycle::with_clock(clock),
            dream: DreamSubsystem::new(),
            session: "me".into(),
            merge_batch: 20,
        }
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
    pub fn tick(&self) -> usize {
        if !self.sleep.should_consolidate() {
            return 0;
        }
        let eps = self
            .store
            .recent_episodes(&self.session, self.merge_batch)
            .unwrap_or_default();
        let items: Vec<String> = eps.iter().map(|e| e.content.clone()).collect();
        let merged = std::cell::RefCell::new(Vec::new());
        let n = self.dream.dream_cycle(&items, &|a, b| {
            let m = format!("{a} ◆ {b}");
            merged.borrow_mut().push(m.clone());
            m
        });
        let now_ts = self.clock.now().timestamp();
        for m in merged.borrow().iter() {
            let ep = CoreEpisode {
                id: format!("mem-dream-{}", uuid::Uuid::new_v4()),
                timestamp: now_ts,
                role: "assistant".into(),
                content: format!("【做梦整合】{m}"),
                session_id: self.session.clone(),
            };
            if let Err(e) = self.store.put_episode(&ep) {
                eprintln!("[dream] 写回记忆失败: {e}");
            }
        }
        self.sleep.reset_after_cycle();
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

    #[test]
    fn dream_merges_and_writes_back_to_real_db() {
        let store = Arc::new(apeireth_memory::SqliteMemoryStore::open_in_memory().unwrap());
        seed(&store);
        let vc = VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap());
        let sched = DreamScheduler::new(Arc::clone(&store), Arc::new(vc.clone()));
        // 安静期未到 → 不做梦
        assert_eq!(sched.tick(), 0);
        // 快进 61s → 做梦: 4 条成对合并 → 2 条写回
        vc.advance(chrono::Duration::seconds(61));
        let n = sched.tick();
        assert_eq!(n, 2, "4 条应合并成 2 对");
        let eps = store.recent_episodes("me", 100).unwrap();
        let dreams: Vec<_> = eps.iter().filter(|e| e.id.starts_with("mem-dream-")).collect();
        assert_eq!(dreams.len(), 2, "合并结果应写回真库");
        assert!(dreams[0].content.contains("◆"), "合并应为拼接: {}", dreams[0].content);
        // reset 后不再做
        assert_eq!(sched.tick(), 0);
        // 第二夜
        vc.advance(chrono::Duration::seconds(61));
        assert!(sched.tick() >= 1);
    }
}
