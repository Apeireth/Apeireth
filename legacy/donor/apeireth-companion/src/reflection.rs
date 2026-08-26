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
use apeireth_memory::{
    CoreEpisode, EpisodeStore, ReflectionCycleScheduler, ReflectionPhase, SqliteMemoryStore,
};
use chrono::{DateTime, Utc};

/// 深度反思器 (LLM 提炼; lib 无 LLM 依赖, 同做梦摘要的 trait 策略).
/// 模块 5: 反思周期完成时, 输入周期上下文, 输出洞察/建议 (JSON 字符串, 由调用方解析).
#[async_trait::async_trait]
pub trait ReflectionReflector: Send + Sync {
    /// 输入反思周期上下文 (近期记忆/事件文本), 返回深度反思结果 (markdown 文本).
    async fn reflect(&self, context: &str) -> Result<String, String>;
}

/// 反思周期调度器: 周期触发 → 状态机推进 → 反思记录写回真库.
pub struct ReflectionScheduler {
    store: Arc<SqliteMemoryStore>,
    clock: Arc<dyn Clock>,
    cycle: ReflectionCycleScheduler,
    period: chrono::Duration,
    last_cycle_at: DateTime<Utc>,
    session: String,
    /// 深度反思器 (可选): 周期完成时 LLM 提炼洞察; 未接 = 只写状态文本 (诚实降级).
    reflector: Option<Arc<dyn ReflectionReflector>>,
    /// N4 元自学习读取口 (可选): 反思时回读历史思维簇, 让反思做"思考的再思考"; 未接 = 不附思维链.
    thought_reader: Option<Arc<dyn crate::thought_cluster::ThoughtClusterReader>>,
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
            reflector: None,
            thought_reader: None,
        }
    }

    /// 接深度反思器 (模块 5): 周期完成时 LLM 提炼洞察; 未接 = 状态文本降级.
    pub fn with_reflector(mut self, r: Arc<dyn ReflectionReflector>) -> Self {
        self.reflector = Some(r);
        self
    }

    /// N4 元自学习读取口: 反思时附历史思维链 (≤3 簇×最新 1 篇×400 字); 未接 = 不附.
    pub fn with_thought_reader(
        mut self,
        r: Arc<dyn crate::thought_cluster::ThoughtClusterReader>,
    ) -> Self {
        self.thought_reader = Some(r);
        self
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

    /// 重要事件积累触发 (Generative Agents: 最近 100 条 importance 和 > 150).
    fn importance_surge(&self) -> bool {
        let eps = self
            .store
            .recent_episodes(&self.session, 100)
            .unwrap_or_default();
        let sum: u64 = eps
            .iter()
            .filter(|e| !e.id.starts_with("reflect-"))
            .map(|e| u64::from(crate::memory_extractor::parse_importance(&e.content)))
            .sum();
        sum > 150
    }

    /// 每 tick 调用: 周期到 或 累计 importance 达阈值 → 推进 4 阶段 → 反思记录写回
    /// (含可选深度反思) → 返回完成周期数 (0/1).
    /// 触发条件 (记忆 v2, Generative Agents 吸收): 周期到 OR 最近 100 条 importance 和 > 150.
    pub async fn tick(&mut self) -> usize {
        let now = self.clock.now();
        let period_due = now - self.last_cycle_at >= self.period;
        let importance_due = self.importance_surge();
        if !period_due && !importance_due {
            return 0;
        }
        if importance_due {
            eprintln!("[reflection] 重要事件积累触发反思 (importance 阈值)");
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
        // 模块 5 深度反思: 周期完成 → LLM 提炼洞察 (失败降级为状态文本, 不丢周期)
        let mut reflect_content = String::new();
        if let Some(r) = &self.reflector {
            let mut context = self
                .store
                .recent_episodes(&self.session, 40)
                .unwrap_or_default()
                .iter()
                .map(|e| {
                    format!(
                        "[{}] {}",
                        e.role,
                        e.content.chars().take(200).collect::<String>()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            // N4 元自学习: 附历史思维链 (≤3 簇×最新 1 篇×400 字, 确定性序), 反思做"思考的再思考"
            if let Some(tr) = &self.thought_reader {
                let mut segs: Vec<String> = Vec::new();
                for c in tr.clusters().into_iter().take(3) {
                    if let Some(f) = tr.read_cluster(&c).into_iter().last() {
                        let body: String = f.content.chars().take(400).collect();
                        segs.push(format!("{}/{}: {body}", c, f.name));
                    }
                }
                if !segs.is_empty() {
                    context = format!("{context}\n【历史思维链】\n{}", segs.join("\n"));
                }
            }
            match r.reflect(&context).await {
                Ok(insight) if !insight.trim().is_empty() => {
                    reflect_content = format!(
                        "【深度反思】第 {} 轮:\n{insight}",
                        self.cycle.cycles_completed
                    );
                }
                Ok(_) => eprintln!("[reflection] 深度反思返回空, 降级状态文本"),
                Err(e) => eprintln!("[reflection] 深度反思失败: {e}, 降级状态文本"),
            }
        }
        let ep = CoreEpisode {
            id: format!("reflect-{}", uuid::Uuid::new_v4()),
            timestamp: now.timestamp(),
            role: "assistant".into(),
            content: if reflect_content.is_empty() {
                content
            } else {
                reflect_content
            },
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

    #[tokio::test]
    async fn no_reflection_before_period() {
        let vc = vclock();
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let mut s = ReflectionScheduler::new(store, Arc::new(vc.clone()), "did-test");
        assert_eq!(s.tick().await, 0);
        assert_eq!(s.cycles_completed(), 0);
    }

    #[tokio::test]
    async fn reflection_runs_on_period_and_writes_back() {
        let vc = vclock();
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let mut s = ReflectionScheduler::new(Arc::clone(&store), Arc::new(vc.clone()), "did-test")
            .with_period(chrono::Duration::days(1));
        vc.advance(chrono::Duration::days(1));
        assert_eq!(s.tick().await, 1, "周期到应完成一轮");
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
        assert_eq!(s.tick().await, 0);
        // 第二周期
        vc.advance(chrono::Duration::hours(12));
        assert_eq!(s.tick().await, 1);
        assert_eq!(s.cycles_completed(), 2);
    }

    #[tokio::test]
    async fn reflection_state_machine_phases_advance() {
        let vc = vclock();
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let mut s = ReflectionScheduler::new(store, Arc::new(vc.clone()), "did-test")
            .with_period(chrono::Duration::hours(1));
        vc.advance(chrono::Duration::hours(1));
        assert_eq!(s.tick().await, 1);
        // Concluded 自动重触发 → 当前应为 Triggered
        assert_eq!(s.cycle.current, ReflectionPhase::Triggered);
    }

    // ---- N4 元自学习注入点测试 ----
    use crate::thought_cluster::ThoughtFile;

    /// N4 测试用 stub 读取口 (2 簇, 确定性).
    struct StubReader;
    impl crate::thought_cluster::ThoughtClusterReader for StubReader {
        fn clusters(&self) -> Vec<String> {
            vec!["甲簇".into(), "乙簇".into()]
        }
        fn read_cluster(&self, _name: &str) -> Vec<ThoughtFile> {
            vec![ThoughtFile {
                name: "2026-08-16-001.md".into(),
                content: "一段可供反思回读的历史思考链内容".into(),
            }]
        }
        fn read_chain(&self, _name: &str) -> Vec<ThoughtFile> {
            Vec::new()
        }
    }

    /// 捕获 reflector 收到的 context, 验证历史思维链被附加.
    struct CaptureReflector(Arc<std::sync::Mutex<Option<String>>>);
    #[async_trait::async_trait]
    impl ReflectionReflector for CaptureReflector {
        async fn reflect(&self, context: &str) -> Result<String, String> {
            *self.0.lock().unwrap() = Some(context.to_string());
            Ok("洞察".into())
        }
    }

    #[tokio::test]
    async fn reflection_context_includes_history_thought_chains() {
        let vc = vclock();
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let captured = Arc::new(std::sync::Mutex::new(None::<String>));
        let mut s = ReflectionScheduler::new(Arc::clone(&store), Arc::new(vc.clone()), "did-test")
            .with_period(chrono::Duration::hours(1))
            .with_reflector(Arc::new(CaptureReflector(Arc::clone(&captured))))
            .with_thought_reader(Arc::new(StubReader));
        vc.advance(chrono::Duration::hours(1));
        assert_eq!(s.tick().await, 1);
        let ctx = captured
            .lock()
            .unwrap()
            .clone()
            .expect("reflector 应收到 context");
        assert!(
            ctx.contains("【历史思维链】"),
            "context 应含历史思维链段: {ctx}"
        );
        assert!(ctx.contains("甲簇/"), "应按簇名序附思维链: {ctx}");
        assert!(
            ctx.contains("历史思考链内容"),
            "应附最新思考文件内容: {ctx}"
        );
    }

    #[tokio::test]
    async fn reflection_without_thought_reader_keeps_plain_context() {
        let vc = vclock();
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let captured = Arc::new(std::sync::Mutex::new(None::<String>));
        let mut s = ReflectionScheduler::new(Arc::clone(&store), Arc::new(vc.clone()), "did-test")
            .with_period(chrono::Duration::hours(1))
            .with_reflector(Arc::new(CaptureReflector(Arc::clone(&captured))));
        vc.advance(chrono::Duration::hours(1));
        assert_eq!(s.tick().await, 1);
        let ctx = captured
            .lock()
            .unwrap()
            .clone()
            .expect("reflector 应收到 context");
        assert!(
            !ctx.contains("【历史思维链】"),
            "未接读取口不应附思维链: {ctx}"
        );
    }
}
