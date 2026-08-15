//! `apeireth-companion::reflection` — 反思周期调度 (接 daemon).
//!
//! 生命周期补充: daemon 已有「主动涌现 (白昼) + 做梦整合 (夜间)」,
//! 本模块加「反思周期」: 每周期 (默认 24h, 虚拟时钟可快进) 推进
//! [`ReflectionCycleScheduler`] 4 阶段 (Triggered→Reflecting→Consolidating→Concluded,
//! Concluded 自动重触发新周期), 周期完成时把反思记录写回真 SQLite (【反思周期】episode).
//!
//! 0 假装: 状态机与写回是真实机制; 反思内容 (LLM 深度反思) 由上层注人 (同做梦摘要的 trait 策略).

use std::sync::Arc;

use apeireth_core::clock::Clock;
use apeireth_memory::{CoreEpisode, EpisodeStore, ReflectionCycleScheduler, ReflectionPhase, SqliteMemoryStore};
use chrono::{DateTime, Utc};

/// 反思周期调度器: 周期触发 → 状态机推进 → 反思记录写回真库.
pub struct ReflectionScheduler {
    store: Arc<SqliteMemoryStore>,
    clock: Arc<dyn Clock>,
    cycle: ReflectionCycleScheduler,
    period: chrono::Duration,
    last_cycle_at: DateTime<Utc>,
    session: String,
}

impl ReflectionScheduler {
    pub fn new(
        store: Arc<SqliteMemoryStore>,
        clock: Arc<dyn Clock>,
        continuity_id: impl Into<String>,
    ) -> Self {
        let now = clock.now();
        Self {
            store,
            clock: Arc::clone(&clock),
            cycle: ReflectionCycleScheduler::new(continuity_id, now.timestamp()),
            period: chrono::Duration::days(1),
            last_cycle_at: now,
            session: "me".into(),
        }
    }

    /// 覆盖反思周期 (默认 1 天).
    pub fn with_period(mut self, period: chrono::Duration) -> Self {
        self.period = period;
        self
    }

    /// 覆盖写回 session (默认 "me").
    pub fn with_session(mut self, session: impl Into<String>) -> Self {
        self.session = session.into();
        self
    }

    /// 已完成周期数.
    pub fn cycles_completed(&self) -> u64 {
        self.cycle.cycles_completed
    }

    /// 每 tick 调用: 周期到 → 推进 4 阶段 → 写回反思记录 → 返回完成周期数 (0/1).
    pub fn tick(&mut self) -> usize {
        let now = self.clock.now();
        if now - self.last_cycle_at < self.period {
            return 0;
        }
        // 快进状态机 (阶段间用周期起点附近的时间戳, 保证单调)
        let base = self.last_cycle_at.timestamp();
        let _ = self.cycle.advance(ReflectionPhase::Reflecting, base + 1);
        let _ = self.cycle.advance(ReflectionPhase::Consolidating, base + 2);
        let _ = self.cycle.advance(ReflectionPhase::Concluded, base + 3); // 自动重触发 Triggered
        let events: Vec<String> = self
            .cycle
            .recent_events(6)
            .iter()
            .map(|e| format!("{:?}@{}", e.phase, e.ts))
            .collect();
        let content = format!(
            "【反思周期】第 {} 轮完成. 最近事件: {}",
            self.cycle.cycles_completed,
            events.join(" → ")
        );
        let ep = CoreEpisode {
            id: format!("reflect-{}", uuid::Uuid::new_v4()),
            timestamp: now.timestamp(),
            role: "assistant".into(),
            content,
            session_id: self.session.clone(),
        };
        if let Err(e) = self.store.put_episode(&ep) {
            eprintln!("[reflection] 写回记忆失败: {e}");
        }
        self.last_cycle_at = now;
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use chrono::TimeZone;

    fn vclock() -> VirtualClock {
        VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap())
    }

    #[test]
    fn no_reflection_before_period() {
        let vc = vclock();
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let mut s = ReflectionScheduler::new(store, Arc::new(vc.clone()), "did-test");
        assert_eq!(s.tick(), 0);
        assert_eq!(s.cycles_completed(), 0);
    }

    #[test]
    fn reflection_runs_on_period_and_writes_back() {
        let vc = vclock();
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let mut s = ReflectionScheduler::new(Arc::clone(&store), Arc::new(vc.clone()), "did-test")
            .with_period(chrono::Duration::days(1));
        vc.advance(chrono::Duration::days(1));
        assert_eq!(s.tick(), 1, "周期到应完成一轮");
        assert_eq!(s.cycles_completed(), 1);
        let eps = store.recent_episodes("me", 10).unwrap();
        assert!(
            eps.iter().any(|e| e.id.starts_with("reflect-")),
            "反思记录应写回真库"
        );
        assert!(
            eps.iter().any(|e| e.content.contains("第 1 轮完成")),
            "反思内容应含轮次"
        );
        // 未到下一周期 → 0
        vc.advance(chrono::Duration::hours(12));
        assert_eq!(s.tick(), 0);
        // 第二周期
        vc.advance(chrono::Duration::hours(12));
        assert_eq!(s.tick(), 1);
        assert_eq!(s.cycles_completed(), 2);
    }

    #[test]
    fn reflection_state_machine_phases_advance() {
        let vc = vclock();
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let mut s = ReflectionScheduler::new(store, Arc::new(vc.clone()), "did-test")
            .with_period(chrono::Duration::hours(1));
        vc.advance(chrono::Duration::hours(1));
        assert_eq!(s.tick(), 1);
        // Concluded 自动重触发 → 当前应为 Triggered
        assert_eq!(s.cycle.current, ReflectionPhase::Triggered);
    }
}
