//! `apeireth-companion::assemble` — CompanionApp 伙伴机制装配器 (lib 层, 零 LLM 依赖).
//!
//! 审计 (2026-08-16, docs/backlog.md P0#1): companion 深度能力全堆在
//! examples/companion_serve.rs (~1600 行), 无 lib 级装配机制 → TUI/CLI 无法复用,
//! 前端接线靠复制。本模块把散装装配抽成 CompanionApp:
//!   - 注入管线: L0 Identity / L1 Essential Story (常驻核心块, mempalace §5.6)
//!     + 状态/记忆(排名+crawl+access)/图谱/偏好/今日/成长 → ContextAssembler 统一预算
//!   - 提炼调度: run_extraction (提炼→图谱→对账→应用) + 节流窗口
//!   - 滚动摘要: summarize_dialog (sum-* 链持久化) + 节流窗口
//!   - 自成长: 反思→经验提炼 (refine_experience) + 晋级候选成文 (export_promotion_candidates)
//!
//! 所有 LLM 调用点 = trait 注入 (与 MemoryExtractor/DreamSummarizer/ReflectionReflector
//! 同一策略模式): 新增 DeepRecall / DialogSummarizer / ExperienceRefiner 三个调用点,
//! MiniMax 实现留在 example (lib 保持零 LLM 依赖, 0 装 PASS)。
//!
//! 0 假装 (诚实):
//!   - L1 Essential Story 无 essential-* 标记时退化选 importance ≥ 8 的记忆 (机制兜底)
//!   - 未注入 extractor/summarizer/refiner → 对应机制如实跳过 (不硬造)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use apeireth_memory::{CoreEpisode, EpisodeStore, HistoryStream, SqliteMemoryStore};

use crate::context::{ContextAssembler, ContextBlock};
use crate::cross_diary::{link_core, CrossDiaryIndex};
use crate::daemon::default_memory_path;
use crate::diary::DiaryStore;
use crate::emergence::RhythmEstimate;
use crate::experience::{Experience, ExperienceStore};
use crate::goal::GoalService;
use crate::memory_extractor::{
    MemoryExtractionService, MemoryExtractor, parse_importance, rank_memory_entries,
};
use crate::memory_graph::MemoryGraph;
use crate::principles::PrincipleStore;

// ============================================================
// LLM 调用点 trait (策略模式; 实现注入, lib 无 LLM 依赖)
// ============================================================

/// 深度召回 (VCP AIMemoHandler 精神): 按当前问题从候选记忆重排, 挑最相关 top N.
#[async_trait::async_trait]
pub trait DeepRecall: Send + Sync {
    async fn recall(&self, query: &str, candidates: &[String]) -> Result<Vec<String>, String>;
}

/// 滚动摘要 (模块 3): 被裁对话旧段 → 一条简洁摘要 (可带上次摘要作链式基线).
#[async_trait::async_trait]
pub trait DialogSummarizer: Send + Sync {
    async fn summarize(&self, text: &str, prev_summary: Option<&str>) -> Result<String, String>;
}

/// 反思 → 经验提炼 (自成长管道 Level 0): 从反思记录提炼一条可复用经验.
/// None = 本次无可提炼 (LLM 判定), 诚实返回.
#[async_trait::async_trait]
pub trait ExperienceRefiner: Send + Sync {
    async fn refine(&self, reflects: &[String]) -> Result<Option<Experience>, String>;
}

// ============================================================
// CompanionApp — 机制装配器
// ============================================================

/// 伙伴机制装配器: 把散装注入/提炼/摘要/自成长接成一套, 供 serve/TUI/CLI 复用.
pub struct CompanionApp {
    store: Arc<SqliteMemoryStore>,
    session: String,
    /// L0: Identity 常驻 (persona 核心; ContextAssembler core 块, 永不截断).
    identity: Option<String>,
    /// L1: Essential Story 常驻预算 (字符; 0 = 禁用).
    essential_budget: usize,
    /// 注入管线总预算 (字符).
    inject_budget: usize,
    /// 节律共享 (daemon 每 tick 写; 注入读 — 模块 1 状态感知).
    rhythm: Option<Arc<Mutex<Option<RhythmEstimate>>>>,
    /// 目标服务共享 (工具桥写; 注入读 — 模块 6).
    goal: Option<Arc<Mutex<GoalService>>>,
    /// 记忆 access 追踪 (记忆 v2: id → (count, last_access); 高频记忆永不冷).
    access: Arc<Mutex<HashMap<String, (u64, i64)>>>,
    /// 提炼节流 (对话后).
    last_extract: Mutex<Instant>,
    extract_interval: Duration,
    /// 摘要节流 (滚动摘要).
    last_summarize: Mutex<Instant>,
    summarize_interval: Duration,
    /// LLM 调用点 (策略注入; None = 机制如实降级).
    extractor: Option<Arc<dyn MemoryExtractor>>,
    summarizer: Option<Arc<dyn DialogSummarizer>>,
    refiner: Option<Arc<dyn ExperienceRefiner>>,
    deep_recall: Option<Arc<dyn DeepRecall>>,
    /// §5.1 收官: 日记本接入 (None = 日记摘要/跨日记关联两源如实缺省, 0 装 PASS)
    diary: Option<Arc<DiaryStore>>,
}

impl CompanionApp {
    /// 装配器: 只需真记忆库 + 会话名; 其余 with_* 按需接线.
    pub fn new(store: Arc<SqliteMemoryStore>, session: impl Into<String>) -> Self {
        Self {
            store,
            session: session.into(),
            identity: None,
            essential_budget: 0,
            inject_budget: 6000,
            rhythm: None,
            goal: None,
            access: Arc::new(Mutex::new(HashMap::new())),
            last_extract: Mutex::new(Instant::now()),
            extract_interval: Duration::from_secs(600),
            last_summarize: Mutex::new(Instant::now()),
            summarize_interval: Duration::from_secs(300),
            extractor: None,
            summarizer: None,
            refiner: None,
            deep_recall: None,
            diary: None,
        }
    }

    // ---------- builder ----------

    /// L0: Identity 常驻 (persona 核心; core 块永不截断).
    pub fn with_identity(mut self, identity: impl Into<String>) -> Self {
        self.identity = Some(identity.into());
        self
    }

    /// L1: Essential Story 常驻预算 (字符; 0 = 禁用).
    pub fn with_essential_budget(mut self, chars: usize) -> Self {
        self.essential_budget = chars;
        self
    }

    /// 注入管线总预算 (字符, 默认 6000).
    pub fn with_inject_budget(mut self, chars: usize) -> Self {
        self.inject_budget = chars.max(100);
        self
    }

    /// §5.1 收官: 日记本接入 — 近 N 日摘要 + 跨日记关联两源启用 (None = 两源缺省).
    pub fn with_diary(mut self, diary: Arc<DiaryStore>) -> Self {
        self.diary = Some(diary);
        self
    }

    /// 节律共享 (daemon 每 tick 写; 注入读).
    pub fn with_rhythm(mut self, r: Arc<Mutex<Option<RhythmEstimate>>>) -> Self {
        self.rhythm = Some(r);
        self
    }

    /// 目标服务共享 (工具桥写; 注入读).
    pub fn with_goal(mut self, g: Arc<Mutex<GoalService>>) -> Self {
        self.goal = Some(g);
        self
    }

    /// 记忆提炼器 (提炼+对账; None = run_extraction 如实跳过).
    pub fn with_extractor(mut self, e: Arc<dyn MemoryExtractor>) -> Self {
        self.extractor = Some(e);
        self
    }

    /// 滚动摘要器 (None = 裁剪时如实提示摘要失败).
    pub fn with_summarizer(mut self, s: Arc<dyn DialogSummarizer>) -> Self {
        self.summarizer = Some(s);
        self
    }

    /// 反思→经验提炼器 (None = 反思完成不提炼经验).
    pub fn with_refiner(mut self, r: Arc<dyn ExperienceRefiner>) -> Self {
        self.refiner = Some(r);
        self
    }

    /// 深度召回器 (None = 记忆暗示词触发时降级普通注入).
    pub fn with_deep_recall(mut self, r: Arc<dyn DeepRecall>) -> Self {
        self.deep_recall = Some(r);
        self
    }

    pub fn with_extract_interval(mut self, d: Duration) -> Self {
        self.extract_interval = d;
        self
    }

    pub fn with_summarize_interval(mut self, d: Duration) -> Self {
        self.summarize_interval = d;
        self
    }

    // ---------- 访问器 ----------

    pub fn store(&self) -> &Arc<SqliteMemoryStore> {
        &self.store
    }

    pub fn session(&self) -> &str {
        &self.session
    }

    /// access 计数快照 (记忆 v2 诊断/测试).
    pub fn access_counts(&self) -> Vec<(String, u64)> {
        let access = self.access.lock().unwrap();
        let mut out: Vec<(String, u64)> = access.iter().map(|(k, (c, _))| (k.clone(), *c)).collect();
        out.sort_by(|a, b| b.1.cmp(&a.1));
        out
    }

    // ============================================================
    // 注入管线 (统一 ContextAssembler; L0/L1 常驻核心块)
    // ============================================================

    /// 构建全部注入块 → 预算化组装. 顺序: L0 Identity → L1 Essential →
    /// 状态 → 记忆 → 图谱 → 偏好 → 今日 → 成长. 返回预算化后的块 (带 name,
    /// 上层可分流: identity 块作独立 persona 消息, 其余合并为记忆注入消息).
    pub async fn build_injection(&self, query: &str) -> Vec<ContextBlock> {
        let mut asm = ContextAssembler::new(self.inject_budget);
        if let Some(id) = &self.identity {
            asm = asm.push(ContextBlock::new("identity", id.clone()).core(true));
        }
        let essential = self.essential_story();
        if !essential.is_empty() {
            asm = asm.push(ContextBlock::new("essential", essential).core(true));
        }
        let state = self.inject_state();
        if !state.is_empty() {
            asm = asm.push(ContextBlock::new("state", state).with_cap(600));
        }
        let mem = self.inject_memory(query).await;
        if !mem.is_empty() {
            asm = asm.push(ContextBlock::new("memory", mem).with_cap(3000));
        }
        let graph = MemoryGraph::new(Arc::clone(&self.store)).graph_injection();
        if !graph.is_empty() {
            asm = asm.push(ContextBlock::new("graph", graph).with_cap(1500));
        }
        let prefs = MemoryExtractionService::new(Arc::clone(&self.store)).preference_injection();
        if !prefs.is_empty() {
            asm = asm.push(ContextBlock::new("preferences", prefs).with_cap(800));
        }
        let today = self.inject_today();
        if !today.is_empty() {
            asm = asm.push(ContextBlock::new("today", today).with_cap(1200));
        }
        let growth = self.inject_growth();
        if !growth.is_empty() {
            asm = asm.push(ContextBlock::new("growth", growth).with_cap(1500));
        }
        asm.assemble_budgeted_blocks()
    }

    /// L1: Essential Story (mempalace §5.6 渐进加载) — 常驻核心块.
    /// 选择: essential-* 显式标记优先, 不足由 importance ≥ 8 记忆补足 (预算内).
    fn essential_story(&self) -> String {
        if self.essential_budget == 0 {
            return String::new();
        }
        let eps = self.store.recent_episodes(&self.session, 200).unwrap_or_default();
        let mut chosen: Vec<String> = Vec::new();
        let mut total = 0usize;
        let mut take = |e: &CoreEpisode, chosen: &mut Vec<String>, total: &mut usize| {
            let c = e.content.chars().count();
            if c == 0 || *total + c > self.essential_budget {
                return;
            }
            if chosen.contains(&e.content) {
                return;
            }
            chosen.push(e.content.clone());
            *total += c;
        };
        // 1. essential-* 显式标记 (主人/提炼器标「长期要事」)
        for e in eps.iter().filter(|e| e.id.starts_with("essential-")) {
            take(e, &mut chosen, &mut total);
        }
        // 2. importance ≥ 8 高价值记忆补足 (机制兜底)
        if total < self.essential_budget {
            let mut high: Vec<&CoreEpisode> = eps
                .iter()
                .filter(|e| parse_importance(&e.content) >= 8)
                .collect();
            high.sort_by(|a, b| {
                parse_importance(&b.content).cmp(&parse_importance(&a.content))
            });
            for e in high {
                take(e, &mut chosen, &mut total);
            }
        }
        if chosen.is_empty() {
            String::new()
        } else {
            let body = chosen.join("\n");
            format!("【长期要事】(常驻) 以下是与主人的长期要事/核心关系信息, 任何时候都应记得:\n{body}")
        }
    }

    /// 状态感知块 (模块 1): 时刻+节律+目标+约定+情绪.
    fn inject_state(&self) -> String {
        use chrono::{Datelike, Local, Timelike};
        let now = Local::now();
        let week = ["周一", "周二", "周三", "周四", "周五", "周六", "周日"]
            [now.weekday().num_days_from_monday() as usize];
        let mut parts: Vec<String> = vec![format!(
            "【当前状态】{} {} {}:{} (本机时区, 时间推理以此为准)",
            now.format("%Y-%m-%d"), week, now.format("%H"), now.format("%M")
        )];
        if let Some(rhythm) = &self.rhythm {
            if let Ok(r) = rhythm.lock() {
                if let Some(est) = r.as_ref() {
                    if est.days > 0 {
                        parts.push(format!(
                            "· 此刻主人活跃概率约 {:.0}% (节律观察 {} 天, 置信 {:.0}%)",
                            est.active_probability * 100.0, est.days, est.confidence * 100.0
                        ));
                    }
                }
            }
        }
        if let Some(goal) = &self.goal {
            if let Ok(g) = goal.lock() {
                if let Some(snap) = g.current() {
                    parts.push(format!(
                        "· 当前目标: {} (阶段 {}, 修订 {})",
                        snap.objective,
                        snap.phase.label(),
                        snap.revision
                    ));
                }
            }
        }
        let eps = self.store.recent_episodes(&self.session, 100).unwrap_or_default();
        let commitments: Vec<&String> = eps
            .iter()
            .filter(|e| e.content.contains("【约定】"))
            .take(3)
            .map(|e| &e.content)
            .collect();
        if !commitments.is_empty() {
            let list = commitments
                .iter()
                .map(|c| c.chars().take(80).collect::<String>())
                .collect::<Vec<_>>()
                .join("; ");
            parts.push(format!("· 近期约定: {list}"));
        }
        if let Some(e) = eps.iter().rev().find(|e| e.content.contains("【情绪信号】")) {
            parts.push(format!(
                "· 主人最近情绪信号: {}",
                e.content.chars().take(80).collect::<String>()
            ));
        }
        parts.join("\n")
    }

    /// 记忆注入 (EMI/NEC 反幻觉; 模块 2 排名分层 + A-MEM crawl + access 追踪;
    /// 记忆暗示词 + APEIRETH_DEEP_RECALL=1 → 深度召回 trait).
    async fn inject_memory(&self, query: &str) -> String {
        const HINT_WORDS: &[&str] = &[
            "记得", "之前", "上次", "我说过", "约定", "还记得", "忘", "以前", "计划", "安排",
        ];
        let want_deep = std::env::var("APEIRETH_DEEP_RECALL").ok().as_deref() == Some("1")
            && HINT_WORDS.iter().any(|w| query.contains(w));
        let eps = self.store.recent_episodes(&self.session, 40).unwrap_or_default();
        if eps.is_empty() {
            return String::new();
        }
        // 独立作用域: MutexGuard 在 await 前释放 (axum handler 要求 future Send)
        let (entries, seed_ids): (Vec<String>, Vec<String>) = {
            let mut access = self.access.lock().unwrap();
            let ranked = rank_memory_entries(&eps, &access, 10);
            let seed_ids: Vec<String> = ranked.iter().map(|(id, _)| id.clone()).collect();
            let entries: Vec<String> = ranked.iter().map(|(_, c)| c.clone()).collect();
            let now = chrono::Utc::now().timestamp();
            for (id, _) in &ranked {
                let e = access.entry(id.clone()).or_insert((0, now));
                e.0 += 1;
                e.1 = now;
            }
            (entries, seed_ids)
        };
        let mut entries = entries;
        if !seed_ids.is_empty() {
            let graph_svc = MemoryGraph::new(Arc::clone(&self.store));
            // N7: 查询形态学 softmax → CRAWL 预算 (纯函数, 同查询同档位)
            for c in graph_svc.crawl(&seed_ids, crate::morphology::crawl_budget(query)) {
                if !entries.contains(&c) {
                    entries.push(c);
                }
            }
        }
        if want_deep {
            if let Some(recall) = &self.deep_recall {
                let candidates: Vec<String> = eps.iter().map(|e| e.content.clone()).collect();
                if let Ok(selected) = recall.recall(query, &candidates).await {
                    // §5.1 收官: 四源统一注入 (深度路径同享两源)
                    let diary_summary = self.diary.as_ref().map(|d| d.recent_injection(DIARY_SUMMARY_DAYS, DIARY_SUMMARY_BUDGET)).unwrap_or_default();
                    let cross_related = self.cross_related_for_query(query);
                    return unified_memory_block(&selected, &diary_summary, &cross_related, UNIFIED_MEMORY_BLOCK_BUDGET);
                }
                // 失败 → 降级普通注入 (诚实)
            }
        }
        // §5.1 收官: 四源统一注入 — 主题索引+日记摘要+跨日记关联+记忆证据块 (各自独立预算)
        let diary_summary = self.diary.as_ref().map(|d| d.recent_injection(DIARY_SUMMARY_DAYS, DIARY_SUMMARY_BUDGET)).unwrap_or_default();
        let cross_related = self.cross_related_for_query(query);
        unified_memory_block(&entries, &diary_summary, &cross_related, UNIFIED_MEMORY_BLOCK_BUDGET)
    }

    /// 今日摘要注入.
    fn inject_today(&self) -> String {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        // 台账 #34: `.unwrap()` 在 DST 回拨/时钟回退时遇 LocalResult::Ambiguous 会 panic
        // → `.single()` + Option 兜底 0 (退化为"纳入全部 episode", 同 and_hms_opt 失败语义, 非 panic)
        let day_start = chrono::Local::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .and_then(|d| d.and_local_timezone(chrono::Local).single())
            .map(|t| t.timestamp())
            .unwrap_or(0);
        let all = self.store.recent_episodes(&self.session, 200).unwrap_or_default();
        let pairs: Vec<(&str, &str)> = all
            .iter()
            .filter(|e| e.timestamp >= day_start)
            .map(|e| (e.id.as_str(), e.content.as_str()))
            .collect();
        let tool_records = match self.store.conn() {
            Ok(conn) => {
                let stream = apeireth_memory::ActionStream::new(&conn);
                stream
                    .list_recent(200, false)
                    .map(|es| es.iter().filter(|e| e.created_at >= day_start).count())
                    .unwrap_or(0)
            }
            Err(_) => 0,
        };
        crate::daily_summary::build_daily_summary(&today, &pairs, tool_records).render()
    }

    /// 自成长管道注入: 待提案经验 + 原则状态 (Level 1/2 驱动).
    fn inject_growth(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        let exp_hint = ExperienceStore::new(Arc::clone(&self.store)).build_promotion_hint();
        if !exp_hint.is_empty() {
            parts.push(exp_hint);
        }
        let ps = PrincipleStore::new(Arc::clone(&self.store));
        let pending = ps.list(Some("pending"));
        if !pending.is_empty() {
            let mut s = String::from(
                "【原则候选】以下原则待主人批准 (主人用 approve_principle 传入 master token 批准; 批准后叠加到工具执行检查):\n",
            );
            for p in pending.iter().take(5) {
                s.push_str(&format!(
                    "  • {} (来源: {}) — {}\n",
                    p.statement, p.source, p.rationale
                ));
            }
            parts.push(s);
        }
        let active = ps.active_rules();
        if !active.is_empty() {
            let mut s = String::from("【动态原则(生效中)】工具执行检查会拦截违反这些原则的动作:\n");
            for p in active.iter().take(8) {
                s.push_str(&format!("  • {} (违反 {} 次)\n", p.statement, p.violations));
            }
            parts.push(s);
        }
        parts.join("\n")
    }

    // ============================================================
    // 提炼调度 (记忆 v2: 提炼→图谱→对账→应用)
    // ============================================================

    /// 提炼节流判断: 距上次提炼 ≥ interval 则刷新窗口并返回 true (调用方触发 run_extraction).
    pub fn extraction_due(&self) -> bool {
        let mut last = self.last_extract.lock().unwrap();
        if last.elapsed() >= self.extract_interval {
            *last = Instant::now();
            true
        } else {
            false
        }
    }

    /// 提炼→图谱写入→对账 (Mem0 式)→应用. 无 extractor → 如实跳过.
    /// 失败路径: 对账失败降级全量写入; 提炼失败只记日志 (限流时放弃, 下个窗口再试).
    pub async fn run_extraction(&self, ctx_len: usize) {
        let Some(extractor) = &self.extractor else {
            return;
        };
        let svc = MemoryExtractionService::new(Arc::clone(&self.store));
        let ctx = svc.recent_context(ctx_len);
        match extractor.extract(&ctx).await {
            Ok(ex) if !ex.is_empty() => {
                if !ex.graph.is_empty() {
                    let graph_svc = MemoryGraph::new(Arc::clone(&self.store));
                    svc.apply_graph(&ex.graph, &graph_svc);
                    eprintln!("[extract] 图谱写入: {} 条三元组", ex.graph.len());
                }
                let existing: Vec<String> = svc
                    .active_episodes(30)
                    .iter()
                    .map(|e| format!("{}|{}", e.id, e.content))
                    .collect();
                match extractor.reconcile(&ex, &existing).await {
                    Ok(actions) => match svc.apply_reconcile(&actions) {
                        Ok(_) => eprintln!(
                            "[extract] 对账完成: {} 动作, 库: {:?}",
                            actions.len(),
                            svc.counts()
                        ),
                        Err(e) => eprintln!("[extract] 对账应用失败: {e}"),
                    },
                    Err(e) => {
                        eprintln!("[extract] 对账失败 (限流/解析), 降级全量写入: {e}");
                        let _ = svc.apply(&ex);
                    }
                }
            }
            Ok(_) => eprintln!("[extract] 无新信息可提炼"),
            Err(e) => eprintln!("[extract] 提炼失败 (限流/解析): {e}"),
        }
    }

    // ============================================================
    // 滚动摘要 (模块 3: sum-* 链持久化)
    // ============================================================

    /// 摘要节流判断 (与 extraction_due 同模式).
    pub fn summarize_due(&self) -> bool {
        let mut last = self.last_summarize.lock().unwrap();
        if last.elapsed() >= self.summarize_interval {
            *last = Instant::now();
            true
        } else {
            false
        }
    }

    /// 滚动摘要: 最近 sum-* 作链式基线 → LLM 摘要 → 持久化 (跨会话可回溯).
    /// 无 summarizer / LLM 失败 / 空摘要 → None (调用方如实提示).
    pub async fn summarize_dialog(&self, text: &str) -> Option<String> {
        let summarizer = self.summarizer.as_ref()?;
        let prev = self
            .store
            .recent_episodes(&self.session, 20)
            .unwrap_or_default()
            .iter()
            .find(|e| e.id.starts_with("sum-"))
            .map(|e| e.content.clone());
        let summary = summarizer.summarize(text, prev.as_deref()).await.ok()?;
        let t = summary.trim();
        if t.is_empty() {
            return None;
        }
        let _ = self.store.put_episode(&CoreEpisode {
            id: format!("sum-{}", uuid::Uuid::new_v4()),
            timestamp: chrono::Utc::now().timestamp(),
            role: "assistant".into(),
            content: t.to_string(),
            session_id: self.session.clone(),
        });
        Some(t.to_string())
    }

    // ============================================================
    // 自成长 (Level 0: 反思→经验; Level 2: 晋级候选成文)
    // ============================================================

    /// 反思记录 → 可复用经验 (无 refiner / 无可提炼 → None).
    pub async fn refine_experience(&self, reflects: &[String]) -> Result<Option<Experience>, String> {
        let Some(refiner) = &self.refiner else {
            return Ok(None);
        };
        refiner.refine(reflects).await
    }

    /// 晋级候选自动成文 (数据目录 promotion-candidates.md; 空则不写). 返回写出的路径.
    pub fn export_promotion_candidates(&self) -> Option<std::path::PathBuf> {
        let cands = PrincipleStore::new(Arc::clone(&self.store)).export_promotion();
        if cands.is_empty() {
            return None;
        }
        let path = default_memory_path().ok()?;
        let dir = path.parent()?;
        std::fs::create_dir_all(dir).ok()?;
        let out = dir.join("promotion-candidates.md");
        std::fs::write(&out, &cands).ok()?;
        Some(out)
    }
}

// ============================================================
// §5.1 收官: 注入链统一接线 — 四源合并 (topic_groups/diary/cross_diary 三 Injector 归一挂点)
// ============================================================

/// 日记摘要源预算 (近 N 日; diary.rs recent_injection 自带字符预算, 此处定档)
pub const DIARY_SUMMARY_DAYS: usize = 3;
pub const DIARY_SUMMARY_BUDGET: usize = 600;
/// 跨日记关联片段源预算 (小额: 关联是线索不是正文)
pub const CROSS_RELATED_MAX_CHARS: usize = 400;
/// 统一记忆块总长上限 (超出按砍序裁剪; ContextBlock with_cap(3000) 同口径)
pub const UNIFIED_MEMORY_BLOCK_BUDGET: usize = 3000;

/// **§5.1 收官统一注入块** — 四源合并, 各源独立预算互不侵占
///
/// 源序 (块内呈现序): 主题索引 → 日记摘要 → 跨日记关联片段 → 记忆证据块.
/// 空源 = 空串 = 该块不注入 (诚实, 不注半残块).
/// **砍序** (总长超 `total_budget` 时): 关联片段 → 日记摘要 → 主题索引 → 记忆证据块
/// (记忆证据块 = 反幻觉基石, 最后砍; 独存仍超时硬切+TRUNCATION 提示).
fn unified_memory_block(
    entries: &[String],
    diary_summary: &str,
    cross_related: &str,
    total_budget: usize,
) -> String {
    // 四源各自独立渲染 (各自预算, 互不侵占)
    let topic = crate::topic_groups::build_topic_index(
        entries,
        crate::topic_groups::TOPIC_INDEX_MAX_CHARS,
    );
    let mem = crate::memory_injection::build_memory_injection(entries);
    // 按砍序从高到低: 关联(0) → 日记(1) → 主题(2) → 记忆证据(3, 最后砍)
    let mut blocks: [String; 4] = [
        cross_related.to_string(),
        diary_summary.to_string(),
        topic,
        mem,
    ];
    // 呈现序: 主题 → 日记 → 关联 → 记忆证据
    let order = [2usize, 1, 0, 3];
    let total = |b: &[String; 4]| {
        order.iter().filter(|&&i| !b[i].is_empty()).map(|&i| b[i].chars().count()).sum::<usize>()
            + order.iter().filter(|&&i| !b[i].is_empty()).count().saturating_sub(1) * 2
    };
    // 超预算按砍序丢弃 (0→1→2), 记忆证据块(3)最后; 独存仍超 → 硬切+提示
    for drop_idx in [0usize, 1, 2] {
        if total(&blocks) <= total_budget {
            break;
        }
        blocks[drop_idx].clear();
    }
    if total(&blocks) > total_budget {
        let notice = "…[记忆证据块超预算, 已截断]";
        let keep = total_budget.saturating_sub(notice.chars().count());
        let truncated: String = blocks[3].chars().take(keep).collect();
        blocks[3] = format!("{truncated}{notice}");
    }
    let parts: Vec<&String> = order.iter().filter(|&&i| !blocks[i].is_empty()).map(|&i| &blocks[i]).collect();
    parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n\n")
}

/// **兼容挂点** — 两源形态 (主题索引 + 记忆证据块), serve 自由注入路径沿用.
fn memory_block(entries: &[String]) -> String {
    unified_memory_block(entries, "", "", usize::MAX)
}

impl CompanionApp {
    /// **跨日记关联片段渲染** (§5.1 机制④接注入链): 按查询实体取关联日记片段
    ///
    /// 路径: query → topic_tokens 与 active_facts 共享 token 匹配 (link_core, 阈值 2)
    /// → 命中 fact → CrossDiaryIndex.diary_for_fact 取日记片段 → 去重+预算截断.
    /// 只经 diary/memory_graph/cross_diary 已有公开接口, 无向量.
    fn cross_related_for_query(&self, query: &str) -> String {
        let diary = match &self.diary {
            Some(d) => d,
            None => return String::new(),
        };
        let graph = MemoryGraph::new(Arc::clone(&self.store));
        let fact_items: Vec<(String, String)> = graph
            .active_facts()
            .into_iter()
            .map(|f| (f.id, format!("{} {} {}", f.subject, f.predicate, f.object)))
            .collect();
        if fact_items.is_empty() {
            return String::new();
        }
        // 查询实体 → 相关记忆节点 (link_core 纯函数, seed 伪条目)
        let seed_links = link_core(
            &[("seed".to_string(), 0usize, query.to_string())],
            &fact_items,
            2,
        );
        if seed_links.is_empty() {
            return String::new();
        }
        // 记忆节点 → 关联日记片段 (去重, 索引序)
        let idx = CrossDiaryIndex::build(diary, &graph, 2);
        let mut snippets: Vec<String> = Vec::new();
        let mut used = 0usize;
        'outer: for l in &seed_links {
            for link in idx.diary_for_fact(&l.fact_id) {
                if snippets.iter().any(|s| s == &link.snippet) {
                    continue;
                }
                let line = format!("• [{}{}] {}", link.diary_date, link.shared_tokens.first().map(|t| format!(" #{t}")).unwrap_or_default(), link.snippet);
                if used + line.chars().count() > CROSS_RELATED_MAX_CHARS {
                    break 'outer;
                }
                used += line.chars().count();
                snippets.push(line);
            }
        }
        if snippets.is_empty() {
            return String::new();
        }
        format!("【跨日记关联】\n{}", snippets.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> Arc<SqliteMemoryStore> {
        Arc::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    fn put(store: &Arc<SqliteMemoryStore>, id: &str, content: &str, session: &str) {
        let _ = store.put_episode(&CoreEpisode {
            id: id.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            role: "assistant".into(),
            content: content.to_string(),
            session_id: session.to_string(),
        });
    }

    /// 假提炼器: 恒返回空 (验证 run_extraction 无 extractor / 空结果路径不崩).
    struct EmptyExtractor;
    #[async_trait::async_trait]
    impl MemoryExtractor for EmptyExtractor {
        async fn extract(&self, _ctx: &str) -> Result<crate::memory_extractor::ExtractedMemory, String> {
            Ok(crate::memory_extractor::ExtractedMemory::default())
        }
    }

    /// 假摘要器: 返回 "摘要:<前 20 字>".
    struct EchoSummarizer;
    #[async_trait::async_trait]
    impl DialogSummarizer for EchoSummarizer {
        async fn summarize(&self, text: &str, _prev: Option<&str>) -> Result<String, String> {
            Ok(format!("摘要:{}", text.chars().take(20).collect::<String>()))
        }
    }

    #[tokio::test]
    async fn injection_contains_identity_and_essential_core_blocks() {
        let store = test_store();
        put(&store, "essential-1", "【imp:9】主人备考高数期中, 目标是 90 分", "me");
        put(&store, "mem-ex-1", "【imp:6】主人喜欢深色主题", "me");
        let app = CompanionApp::new(store, "me")
            .with_identity("你是阿佩瑞斯, 沉稳古风, 自称本座")
            .with_essential_budget(500);
        let out = app.build_injection("你好").await;
        let joined = out.iter().map(|b| b.content.as_str()).collect::<Vec<_>>().join("\n");
        assert!(joined.contains("沉稳古风"), "L0 Identity 应常驻");
        assert!(joined.contains("【长期要事】"), "L1 Essential 应常驻");
        assert!(joined.contains("备考高数期中"), "essential-* 标记应入选");
        assert!(joined.contains("【当前状态】"), "状态块应注入");
        assert!(joined.contains("【imp:6】主人喜欢深色主题"), "普通记忆也应注入");
    }

    #[tokio::test]
    async fn essential_prefers_tagged_over_high_importance() {
        let store = test_store();
        // 高 importance 但非 essential-*: 仅在预算内补足, 不应挤掉 essential-*
        put(&store, "mem-ex-9", "【imp:10】高价值但未标记的记忆", "me");
        put(&store, "essential-1", "【imp:7】标记为长期要事", "me");
        let app = CompanionApp::new(store, "me").with_essential_budget(80);
        let out = app.build_injection("x").await;
        let joined = out.iter().map(|b| b.content.as_str()).collect::<Vec<_>>().join("\n");
        let ess = joined.split("【长期要事】").nth(1).unwrap_or("");
        assert!(ess.contains("标记为长期要事"), "essential-* 应优先");
        // 预算 80: essential-1 (约 20 字) 后剩余可装 high (约 20 字) — 两者都应出现
        assert!(joined.contains("高价值但未标记"), "高 importance 应补足");
    }

    #[tokio::test]
    async fn extraction_due_throttles() {
        let store = test_store();
        let app = CompanionApp::new(store, "me").with_extract_interval(Duration::from_millis(1));
        assert!(!app.extraction_due(), "启动窗口内不应触发 (与 serve 启动 600s 内不提炼一致)");
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(app.extraction_due(), "窗口过期应触发");
        assert!(!app.extraction_due(), "触发后窗口刷新");
    }

    #[tokio::test]
    async fn run_extraction_without_extractor_is_noop() {
        let store = test_store();
        put(&store, "mem-1", "对话内容", "me");
        let app = CompanionApp::new(store, "me");
        app.run_extraction(10).await; // 不应 panic
        assert!(app.store().recent_episodes("me", 10).unwrap().len() >= 1);
    }

    #[tokio::test]
    async fn summarize_dialog_persists_sum_chain() {
        let store = test_store();
        put(&store, "mem-1", "【imp:5】一段较长的对话内容, 需要被摘要成一条简洁记录", "me");
        let app = CompanionApp::new(store, "me").with_summarizer(Arc::new(EchoSummarizer));
        let s = app.summarize_dialog("你好, 我是主人, 今天天气不错").await;
        assert!(s.is_some(), "假摘要器应成功");
        let sum = s.unwrap();
        assert!(sum.starts_with("摘要:"), "应持久化摘要链");
        let eps = app.store().recent_episodes("me", 20).unwrap();
        assert!(eps.iter().any(|e| e.id.starts_with("sum-")), "sum-* 应入库");
    }

    #[tokio::test]
    async fn summarize_without_summarizer_returns_none() {
        let store = test_store();
        let app = CompanionApp::new(store, "me");
        assert!(app.summarize_dialog("hello").await.is_none(), "无 summarizer 如实降级");
    }

    #[tokio::test]
    async fn injection_budget_truncates_noncore_but_keeps_identity() {
        let store = test_store();
        let long_mem = "【imp:5】".to_string() + &"很长的记忆内容".repeat(300); // ~2400 字
        put(&store, "mem-long", &long_mem, "me");
        let app = CompanionApp::new(store, "me")
            .with_identity("本座是阿佩瑞斯")
            .with_inject_budget(800);
        let out = app.build_injection("x").await;
        let joined = out.iter().map(|b| b.content.as_str()).collect::<Vec<_>>().join("\n");
        assert!(joined.contains("本座是阿佩瑞斯"), "核心块永不截断");
        let total: usize = out.iter().map(|b| b.content.chars().count()).sum();
        assert!(total <= 800, "总预算应约束, got {total}");
    }

    #[tokio::test]
    async fn inject_today_day_start_no_panic_on_ambiguous_local_time() {
        // 回归测试 (台账 #34): DST 回拨/时钟回退时本地零点可能 LocalResult::Ambiguous,
        // 旧 `.unwrap()` 会 panic; 改 `.single()` + Option 兜底后必须是非 panic 路径.
        // ① 直接构造 Ambiguous / None 两变体, 证明 `.single()` 永不 panic 且返 None
        let now = chrono::Local::now();
        let ambiguous: chrono::LocalResult<chrono::DateTime<chrono::Local>> =
            chrono::LocalResult::Ambiguous(now, now);
        assert!(ambiguous.single().is_none(), "Ambiguous → None, 不 panic");
        let none: chrono::LocalResult<chrono::DateTime<chrono::Local>> = chrono::LocalResult::None;
        assert!(none.single().is_none(), "None → None, 不 panic");

        // ② 复刻修复后的表达式 (与 inject_today 同源), 正常机器上应得今日零点戳
        let day_start = chrono::Local::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .and_then(|d| d.and_local_timezone(chrono::Local).single())
            .map(|t| t.timestamp())
            .unwrap_or(0);
        assert!(day_start <= chrono::Local::now().timestamp(), "今日零点 ≤ 现在");

        // ③ 真实 inject_today 生产路径: 今日 episode 应被纳入且不 panic
        let store = test_store();
        put(&store, "ep-today", "今日事件回归", "me");
        let app = CompanionApp::new(store, "me");
        let rendered = app.inject_today();
        assert!(rendered.contains("今日事件回归"), "今日 episode 应入选今日摘要");
    }
    // ===== §5.1 收官: 注入链统一接线 (四源合并/独立预算/空路径/砍序) =====

    #[test]
    fn unified_block_merges_four_sources_in_order() {
        let entries = vec![
            "主人喜欢线代复习, 换元法常练".to_string(),
            "主人爱喝深烘咖啡, 手冲为主".to_string(),
        ];
        let diary = "【近3日日记】\n2026-08-16: 今天复习了线代".to_string();
        let cross = "【跨日记关联】\n• [2026-08-15] 换元法练习记录".to_string();
        let out = unified_memory_block(&entries, &diary, &cross, usize::MAX);
        // 四源皆在
        assert!(out.contains("线代") || out.contains("【记忆索引】"), "主题索引应在");
        assert!(out.contains("近3日日记"), "日记摘要应在");
        assert!(out.contains("跨日记关联"), "关联片段应在");
        assert!(out.contains("记忆证据"), "记忆证据块应在");
        // 呈现序: 日记摘要在关联片段之前, 记忆证据块最后
        let p_diary = out.find("近3日日记").unwrap();
        let p_cross = out.find("跨日记关联").unwrap();
        let p_mem = out.find("记忆证据").unwrap();
        assert!(p_diary < p_cross && p_cross < p_mem, "呈现序应为 日记→关联→记忆证据");
    }

    #[test]
    fn unified_block_independent_budgets_no_bleed() {
        let entries = vec!["一条普通记忆".to_string()];
        let diary_short = "短日记".to_string();
        let diary_long = format!("【近3日日记】\n{}", "长".repeat(590));
        let cross = "【跨日记关联】\n• 关联条目".to_string();
        // 日记源加长不应改变记忆证据块/关联块的渲染内容 (各源独立预算)
        let out_short = unified_memory_block(&entries, &diary_short, &cross, usize::MAX);
        let out_long = unified_memory_block(&entries, &diary_long, &cross, usize::MAX);
        let mem_of = |s: &str| s.split("【记忆证据】").nth(1).unwrap_or("").to_string();
        assert_eq!(mem_of(&out_short), mem_of(&out_long), "日记加长不应侵蚀记忆证据块");
        assert!(out_long.contains(&cross), "日记加长不应侵蚀关联块");
    }

    #[test]
    fn unified_block_empty_paths_honest() {
        let entries = vec!["主人喜欢线代".to_string()];
        // 两源空 → 输出不含半残块 (无空标题/空段)
        let out = unified_memory_block(&entries, "", "", usize::MAX);
        assert!(!out.contains("【近3日日记】"), "空日记不应注入半残块");
        assert!(!out.contains("【跨日记关联】"), "空关联不应注入半残块");
        // 全空 → 空串
        assert_eq!(unified_memory_block(&[], "", "", usize::MAX), "");
        // 只有日记源非空 → 只出日记块
        let out_diary_only = unified_memory_block(&[], "【近3日日记】\n仅日记", "", usize::MAX);
        assert_eq!(out_diary_only, "【近3日日记】\n仅日记");
    }

    #[test]
    fn unified_block_drop_order_mem_last() {
        let entries = vec!["主人喜欢线代复习与换元法".to_string()];
        let diary = format!("【近3日日记】\n{}", "记".repeat(200));
        let cross = format!("【跨日记关联】\n{}", "关".repeat(200));
        // 预算紧: 关联与日记必被砍, 记忆证据块独存 (反幻觉基石最后砍)
        let tight = 400usize;
        let out = unified_memory_block(&entries, &diary, &cross, tight);
        assert!(out.contains("记忆证据"), "记忆证据块最后砍");
        assert!(!out.contains("跨日记关联"), "超预算先砍关联片段");
        // 预算极小: 记忆证据块硬切+提示
        let tiny = 100usize;
        let out_tiny = unified_memory_block(&entries, "", "", tiny);
        assert!(out_tiny.chars().count() <= tiny, "硬切后不得超总预算");
        assert!(out_tiny.contains("已截断"), "硬切须留提示");
        // 宽松预算: 四源全留
        let loose = unified_memory_block(&entries, &diary, &cross, usize::MAX);
        assert!(loose.contains("跨日记关联") && loose.contains("近3日日记"));
    }

    #[test]
    fn memory_block_compat_two_source() {
        let entries = vec!["主人喜欢线代复习, 换元法常练".to_string()];
        let out = memory_block(&entries);
        // 兼容两源: 无日记/关联块, 主题索引或记忆证据在
        assert!(!out.contains("近3日日记") && !out.contains("跨日记关联"));
        assert!(out.contains("线代") || out.contains("记忆索引"), "主题索引或记忆证据应在");
        // 空条目 → 空串 (与旧行为一致)
        assert_eq!(memory_block(&[]), "");
    }

}
