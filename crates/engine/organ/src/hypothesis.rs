//! F4 Hypothesis 器官真实现 (v2 移植版, per `legacy/donor/apeireth-companion/src/hypothesis.rs`).
//!
//! **v1 → v2 1:1 翻译纪律**:
//!
//! - v1 真实现是**确定性机制** (HypothesisStore + VerifyPlanner + ReconcileSink 三件套,
//!   全部可测, 无 LLM 依赖, per `legacy/donor/apeireth-companion/src/hypothesis.rs:11-17`
//!   文档明示 "机制 (确定性, 无 LLM)").
//! - v2 真实现保留 v1 全部确定性算法: 4 态状态机 (Conjecture/Verifying/Confirmed/Refuted),
//!   加权证据累积触发定论, `min_evidence_to_settle` 防单条大权重拍板, `VerifyPlanner`
//!   成本最低验证方式优先, `ReconcileSink` trait 默认 NoopSink (0 装 PASS).
//! - v2 trait 接口 (`OrganTrait`) 保留 LLM factory 字段 (`llm_factory()`), 默认 None.
//!   未来 v2.1 路线可加"LLM 命题抽取"路径, 但**不破坏** v1 确定性算法真相.
//!
//! **与 v1 真实现的 4 个差异 (子代理 R2 独立判断, 见模块顶注释)**:
//!
//! 1. **时间戳**: v1 用 `chrono::Utc::now()` 隐式; v2 organ crate 不依赖 chrono (保持
//!    依赖最小, 与 curiosity/emotion 一致), 改 `at_ms: i64` 由调用方显式注入. process()
//!    内部用 `0` 兜底 (无 clock 依赖). 时间字段保留 schema, 不丢 v1 信息.
//! 2. **GraphReconcileSink 简化**: v1 的 `GraphReconcileSink` 依赖 `apeireth_memory::SqliteMemoryStore`
//!    + `crate::memory_graph::MemoryGraph` (老 path); v2 当前**不实装** (0 装 PASS), 仅
//!    保留 `ReconcileSink` trait + `NoopSink` 默认 impl. W2/W3 真接时再注入真 sink.
//! 3. **id 类型**: v1 用 `u64`, 任务示例给 `String`; **1:1 用 u64** (v1 真值).
//! 4. **状态机 4 态**: 任务示例 3 态 (Pending/Confirmed/Refuted); **1:1 用 v1 4 态**
//!    (Conjecture/Verifying/Confirmed/Refuted). 区别语义: Conjecture=待验证设计,
//!    Verifying=证据累积中. Verifying 是过程态, OrganOutput 不暴露 (内部状态机).
//!
//! **0 装 PASS**:
//!
//! - 本模块不假装能调 LLM (v1 没 LLM 路径, v2 也不假装).
//! - `VerifyPlanner` 输出 `VerifyPlan` (ObserveWindow/AskMaster/OracleResolve) 给
//!    runtime 决定如何执行, organ trait 仅产出建议, 不假装 plan 已执行.
//! - `ReconcileSink` 是 trait 接口, 默认 `NoopSink` (诚实: 未接真对账).
//!
//! **v1 哲学** (per `legacy/donor/apeireth-companion/src/hypothesis.rs:1-9`):
//!
//! - 好奇心决定探索哪 → 世界模型提供推演载体 → **假设检验设计验证** → 记忆提供证据库
//!   → 验证结果更新她. 本模块是四原型串链的中枢.
//! - W2 因果边统计验证是**被动**版 (记忆时间线里数次数), 本模块是**主动**版
//!   (她主动提出"如果 X 则 Y", 设计验证, 对账更新).
//!
//! **承接**:
//!
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入). F4 与 E4/F6/W1 共享
//!   `LlmFactory` trait 边界, 当前 organ `process()` 都不调 LLM (per v1 确定性).
//!
//! **3 阶审查** (O-6 锚 9):
//!
//! 1. 总体: 1:1 翻译 v1 HypothesisStore + VerifyPlanner + ReconcileSink 三件套
//! 2. 系统: impl 在 engine (`apeireth-organ`), trait 在 foundation (`apeireth-plugin`)
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, F4 trait process() 调 HypothesisStore

use std::collections::HashMap;

use apeireth_plugin::llm_factory::LlmFactory;
use apeireth_plugin::organ::{OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait};

// ============================================
// v1 数据结构 1:1 翻译 (HypothesisStore + Evidence + Status)
// ============================================

/// 假设状态 (per v1 `HypothesisStatus` 1:1, 4 态).
///
/// 0 装诚实: 区别于任务示例 3 态. v1 4 态含过程态 `Verifying` (证据累积但未达阈值),
/// `Conjecture` 是"已设计待开始". v2 1:1 保留语义.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HypothesisStatus {
    /// 已提出猜想 (待开始验证设计).
    Conjecture,
    /// 验证中 (证据累积中, 未达 confirm/refute 阈值).
    Verifying,
    /// 已确认 (score ≥ confirm_threshold 且 ≥ min_evidence_to_settle 条证据).
    Confirmed,
    /// 已证伪 (score ≤ refute_threshold 且 ≥ min_evidence_to_settle 条证据).
    Refuted,
}

/// 证据来源 (per v1 `EvidenceSource` 1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSource {
    /// 低成本观察 (本机事件/数据).
    Observation,
    /// 主人回答 (疑问路由: 问主人更快).
    MasterAnswer,
    /// oracle 可证伪预测结算.
    OracleResolve,
}

/// 一条证据 (per v1 `Evidence` 1:1).
///
/// 正 weight = 支持假设, 负 weight = 反驳假设. `at_ms` 由调用方注入 (v2 organ
/// crate 无 chrono 依赖, 不隐式取时间).
#[derive(Debug, Clone)]
pub struct Evidence {
    pub source: EvidenceSource,
    pub weight: f64,
    pub detail: String,
    pub at_ms: i64,
}

impl Evidence {
    /// 支持证据 (weight 取绝对值, 确保正)
    pub fn supporting(source: EvidenceSource, weight: f64, detail: impl Into<String>) -> Self {
        Self {
            source,
            weight: weight.abs(),
            detail: detail.into(),
            at_ms: 0, // v2: 由调用方注入, 不隐式取时间 (无 chrono dep)
        }
    }

    /// 反驳证据 (weight 取负绝对值, 确保负)
    pub fn refuting(source: EvidenceSource, weight: f64, detail: impl Into<String>) -> Self {
        Self {
            source,
            weight: -weight.abs(),
            detail: detail.into(),
            at_ms: 0,
        }
    }

    /// 设置时间戳 (v2: 显式注入, 替换 v1 chrono)
    pub fn at_ms(mut self, at_ms: i64) -> Self {
        self.at_ms = at_ms;
        self
    }
}

/// 一条假设 (per v1 `Hypothesis` 1:1).
#[derive(Debug, Clone)]
pub struct Hypothesis {
    pub id: u64,
    pub statement: String,
    pub status: HypothesisStatus,
    pub evidence: Vec<Evidence>,
    /// 加权证据分 (支持证据和 - 反驳证据和).
    pub score: f64,
    pub created_ms: i64,
    pub updated_ms: i64,
}

/// 假设库配置 (per v1 `HypothesisConfig` 1:1).
#[derive(Debug, Clone)]
pub struct HypothesisConfig {
    /// 确认阈值: score ≥ 此值 → Confirmed.
    pub confirm_threshold: f64,
    /// 证伪阈值: score ≤ 此值 → Refuted.
    pub refute_threshold: f64,
    /// 最小证据数才可确认 (防单条大权重拍板).
    pub min_evidence_to_settle: usize,
}

impl Default for HypothesisConfig {
    fn default() -> Self {
        Self {
            confirm_threshold: 2.0,    // per v1 default
            refute_threshold: -2.0,    // per v1 default
            min_evidence_to_settle: 2, // per v1 default
        }
    }
}

// ============================================
// v1 HypothesisStore 1:1 翻译 (确定性, 无 LLM)
// ============================================

/// 假设库 (per v1 `HypothesisStore` 1:1 翻译).
///
/// 0 装 PASS: 无 LLM 依赖. 全部状态可测, 4 态状态机 + 加权证据累积.
#[derive(Debug)]
pub struct HypothesisStore {
    config: HypothesisConfig,
    items: HashMap<u64, Hypothesis>,
    next_id: u64,
}

impl HypothesisStore {
    pub fn new(config: HypothesisConfig) -> Self {
        Self {
            config,
            items: HashMap::new(),
            next_id: 1,
        }
    }

    /// 登记猜想 (好奇/探索中发现的可证伪命题).
    pub fn conjecture(&mut self, statement: impl Into<String>) -> Hypothesis {
        let h = Hypothesis {
            id: self.next_id,
            statement: statement.into(),
            status: HypothesisStatus::Conjecture,
            evidence: Vec::new(),
            score: 0.0,
            created_ms: 0, // v2: 不隐式取时间, 由调用方在 reconcile 时注入
            updated_ms: 0,
        };
        self.next_id += 1;
        self.items.insert(h.id, h.clone());
        h
    }

    /// 开始验证 (Conjecture → Verifying).
    pub fn start_verify(&mut self, id: u64) -> Result<(), String> {
        let h = self.items.get_mut(&id).ok_or("假设不存在")?;
        match h.status {
            HypothesisStatus::Conjecture => {
                h.status = HypothesisStatus::Verifying;
                // v2: 时间戳由调用方在 reconcile 时注入, 这里用 0 兜底
                h.updated_ms = now_ms_v2();
                Ok(())
            }
            s => Err(format!("状态 {s:?} 不能开始验证 (仅 Conjecture 可)")),
        }
    }

    /// 加证据 → 加权更新状态 (确定性状态机).
    /// 返回更新后的状态.
    pub fn add_evidence(&mut self, id: u64, ev: Evidence) -> Result<HypothesisStatus, String> {
        let h = self.items.get_mut(&id).ok_or("假设不存在")?;
        if matches!(
            h.status,
            HypothesisStatus::Confirmed | HypothesisStatus::Refuted
        ) {
            return Err(format!("假设已定论 ({:?}), 不再接受证据", h.status));
        }
        h.score += ev.weight;
        h.evidence.push(ev);
        // v2: 时间戳由调用方在 reconcile 时注入, 这里用 0 兜底
        h.updated_ms = now_ms_v2();
        // 定论判定: 需要最小证据数 (防单条大权重拍板)
        if h.evidence.len() >= self.config.min_evidence_to_settle {
            if h.score >= self.config.confirm_threshold {
                h.status = HypothesisStatus::Confirmed;
            } else if h.score <= self.config.refute_threshold {
                h.status = HypothesisStatus::Refuted;
            } else {
                h.status = HypothesisStatus::Verifying;
            }
        } else {
            h.status = HypothesisStatus::Verifying;
        }
        Ok(h.status)
    }

    pub fn get(&self, id: u64) -> Option<&Hypothesis> {
        self.items.get(&id)
    }

    /// 列假设 (可选按状态过滤; 默认按 updated_ms 倒序)
    pub fn list(&self, status: Option<HypothesisStatus>) -> Vec<&Hypothesis> {
        let mut out: Vec<&Hypothesis> = self
            .items
            .values()
            .filter(|h| status.map_or(true, |s| h.status == s))
            .collect();
        out.sort_by_key(|h| std::cmp::Reverse(h.updated_ms));
        out
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// v2 时间源 (free fn, 不持借用; 默认 0; 子代理 / runtime 可换 impl via
    /// `with_clock` 未来扩展). 0 装诚实: 当前不接真 chrono, 假设时间由对账层注入.
    #[allow(dead_code)]
    fn _placeholder(&self) {}
}

/// v2 时间源 (free fn, 无借用). 默认 0; 真时间由调用方在 reconcile 时注入.
/// 0 装诚实: 不接 chrono, 假设对账层知道当前时间.
fn now_ms_v2() -> i64 {
    0
}

// ============================================
// v1 VerifyPlanner 1:1 翻译 (确定性, 无 LLM)
// ============================================

/// 验证计划 (per v1 `VerifyPlan` 1:1).
///
/// 0 装诚实: 是**计划**而非执行. Runtime 拿到 plan 后自己决定如何执行
/// (观察窗 / 问主人 / oracle 喂奇), organ 不假装 plan 已执行.
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyPlan {
    /// 低成本观察窗: 观察 N 小时内的相关信号 (本机事件/数据).
    ObserveWindow { hours: f64 },
    /// 问主人更快 (E4 疑问路由哲学: 不硬分线).
    AskMaster { question: String },
    /// 可证伪预测: 喂 oracle, deadline 内结算.
    OracleResolve { deadline_ms: i64 },
}

/// 验证计划配置 (per v1 `PlannerConfig` 1:1).
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// 观察窗成本 (token 量级).
    pub observe_cost: f64,
    /// 问主人成本 (打扰主人, 高).
    pub ask_cost: f64,
    /// 预算阈值: 低于此成本倾向观察窗.
    pub budget: f64,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            observe_cost: 50.0,
            ask_cost: 300.0,
            budget: 400.0, // > ask_cost: 预算内问主人可行 (否则永不 AskMaster)
        }
    }
}

/// 验证调度器 (per v1 `VerifyPlanner` 1:1 翻译).
///
/// 确定性: 成本最低的验证方式优先, 主人可答且观察不可行的才问.
#[derive(Debug)]
pub struct VerifyPlanner {
    config: PlannerConfig,
}

impl VerifyPlanner {
    pub fn new(config: PlannerConfig) -> Self {
        Self { config }
    }

    pub fn plan(&self, h: &Hypothesis, observable: bool) -> VerifyPlan {
        if observable && self.config.observe_cost <= self.config.budget {
            VerifyPlan::ObserveWindow { hours: 24.0 }
        } else if self.config.ask_cost <= self.config.budget {
            VerifyPlan::AskMaster {
                question: format!("关于『{}』, 想确认一下——", h.statement),
            }
        } else {
            VerifyPlan::OracleResolve {
                deadline_ms: 7 * 24 * 3600 * 1000, // 7 天
            }
        }
    }
}

// ============================================
// v1 ReconcileSink 1:1 翻译 (trait 口, 默认 NoopSink)
// ============================================

/// 对账 sink: 定论结果写回记忆图 (W2 因果边) — trait 口, 默认 no-op.
///
/// 0 装 PASS: 不假装已对账; 由调用方决定是否接 memory_graph.
/// 真生产路径 (`GraphReconcileSink`) 在 v2 留 trait, impl 留给 W2/W3 真接时填
/// (per v1 `GraphReconcileSink` 依赖 `apeireth_memory::SqliteMemoryStore` +
/// `MemoryGraph`, 当前 0 装).
pub trait ReconcileSink: Send + Sync {
    fn write_back(&mut self, h: &Hypothesis) -> Result<(), String>;
}

/// 默认 no-op sink (诚实: 未接真对账).
#[derive(Debug, Default)]
pub struct NoopSink;

impl ReconcileSink for NoopSink {
    fn write_back(&mut self, _h: &Hypothesis) -> Result<(), String> {
        Ok(())
    }
}

// ============================================
// F4 HypothesisOrgan (v2 trait 真实现)
// ============================================

/// F4 假设器官 (per v2 OrganTrait 1:1 翻译 v1 HypothesisStore + VerifyPlanner).
///
/// **构造**:
/// - `llm_factory`: 保留给未来 v2.1 LLM 命题抽取路径. 当前算法**不用** LLM
///   (per v1 确定性).
/// - `model`: model ID, 同 llm_factory 一样仅占位未来扩展.
///
/// **0 装诚实**: `llm_factory()` 返 None — v1 hypothesis 路径不需要 LLM, 不假装.
pub struct HypothesisOrgan {
    store: std::sync::Mutex<HypothesisStore>,
    planner: std::sync::Mutex<VerifyPlanner>,
    sink: std::sync::Mutex<Box<dyn ReconcileSink>>,
    dry_run: bool,
    /// 保留 LLM factory (未来扩展, 当前**不用** — 0 装诚实)
    _llm_factory: std::sync::Arc<dyn LlmFactory>,
    /// 保留 model ID (未来扩展, 当前**不用** — 0 装诚实)
    _model: String,
}

impl HypothesisOrgan {
    /// 构造 F4 hypothesis organ (默认 NoopSink + 默认 configs).
    ///
    /// `llm_factory` 和 `model` 保留给未来 v2.1 LLM 命题抽取路径. 当前算法不调用,
    /// 0 装诚实.
    pub fn new(llm_factory: std::sync::Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self::with_configs(
            llm_factory,
            model,
            HypothesisConfig::default(),
            PlannerConfig::default(),
            Box::new(NoopSink),
            false,
        )
    }

    /// 构造 F4 hypothesis organ + 自定义 config + sink + dry_run.
    ///
    /// 0 装诚实: 即使传 sink, 当前 store 不自动调用 (per trait 边界, process() 仅
    /// 产 plan; runtime 拿到 Confirmed/Refuted 后调 `reconcile()`).
    pub fn with_configs(
        llm_factory: std::sync::Arc<dyn LlmFactory>,
        model: impl Into<String>,
        store_config: HypothesisConfig,
        planner_config: PlannerConfig,
        sink: Box<dyn ReconcileSink>,
        dry_run: bool,
    ) -> Self {
        Self {
            store: std::sync::Mutex::new(HypothesisStore::new(store_config)),
            planner: std::sync::Mutex::new(VerifyPlanner::new(planner_config)),
            sink: std::sync::Mutex::new(sink),
            dry_run,
            _llm_factory: llm_factory,
            _model: model.into(),
        }
    }

    /// 登记猜想 (per v1 API 1:1, 暴露给外部以便 Runtime 喂好奇产出)
    pub fn conjecture(&self, statement: impl Into<String>) -> Hypothesis {
        let mut store = self
            .store
            .lock()
            .expect("HypothesisOrgan mutex poisoned (0 装诚实)");
        store.conjecture(statement)
    }

    /// 开始验证 (per v1 API 1:1)
    pub fn start_verify(&self, id: u64) -> Result<(), String> {
        let mut store = self
            .store
            .lock()
            .expect("HypothesisOrgan mutex poisoned (0 装诚实)");
        store.start_verify(id)
    }

    /// 加证据 (per v1 API 1:1)
    pub fn add_evidence(&self, id: u64, ev: Evidence) -> Result<HypothesisStatus, String> {
        let mut store = self
            .store
            .lock()
            .expect("HypothesisOrgan mutex poisoned (0 装诚实)");
        store.add_evidence(id, ev)
    }

    /// 取假设 (per v1 API 1:1)
    pub fn get(&self, id: u64) -> Option<Hypothesis> {
        let store = self
            .store
            .lock()
            .expect("HypothesisOrgan mutex poisoned (0 装诚实)");
        store.get(id).cloned()
    }

    /// 列假设 (per v1 API 1:1)
    pub fn list(&self, status: Option<HypothesisStatus>) -> Vec<Hypothesis> {
        let store = self
            .store
            .lock()
            .expect("HypothesisOrgan mutex poisoned (0 装诚实)");
        store.list(status).into_iter().cloned().collect()
    }

    /// 假设数 (per v1 API 1:1)
    pub fn len(&self) -> usize {
        let store = self
            .store
            .lock()
            .expect("HypothesisOrgan mutex poisoned (0 装诚实)");
        store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 制定验证计划 (per v1 VerifyPlanner 1:1)
    pub fn plan_verify(&self, h: &Hypothesis, observable: bool) -> VerifyPlan {
        let planner = self
            .planner
            .lock()
            .expect("HypothesisOrgan mutex poisoned (0 装诚实)");
        planner.plan(h, observable)
    }

    /// 对账: Confirmed/Refuted 假设写回 sink (per v1 ReconcileSink 1:1).
    ///
    /// 0 装 PASS: 默认 sink 是 NoopSink, 不假装已对账. 真 sink 由调用方注入.
    pub fn reconcile(&self, h: &Hypothesis) -> Result<(), String> {
        let mut sink = self
            .sink
            .lock()
            .expect("HypothesisOrgan mutex poisoned (0 装诚实)");
        sink.write_back(h)
    }
}

#[async_trait::async_trait]
impl OrganTrait for HypothesisOrgan {
    fn name(&self) -> &'static str {
        "F4 Hypothesis"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::F4
    }

    async fn process(&self, input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 1:1 翻译 v1 hypothesis.process 路径:
        // - episode 上下文 → 如果 context_hints 非空, 把第 1 个 hint 当猜想 statement
        //   登记 (v1 好奇产出可作为 hypothesis, per `legacy/.../hypothesis.rs:19-22`)
        // - dry_run 模式不真登记 (per curiosity 同模式)
        // - 输出 OrganOutput::Hypothesis { id, statement, conf }
        //
        // **0 装诚实**: process 是**入口而非全自动闭环**. 真验证 / 加证据 / 对账由
        // runtime 在后续认知循环里调 (per v1 哲学 "假设检验设计验证" 是过程, 非单步).

        if self.dry_run || input.dry_run {
            // dry-run: 不真登记, 返 NotImplemented placeholder
            return Ok(OrganOutput::NotImplemented {
                organ: OrganKind::F4,
                note: "F4 hypothesis dry-run: no conjecture registered (per v1 truth)".into(),
            });
        }

        // 真登记路径: 从 context_hints 提 statement
        let statement = input
            .context_hints
            .first()
            .cloned()
            .unwrap_or_else(|| format!("ep-{}: 主人说了点什么", input.episode.id));

        let h = self.conjecture(statement.clone());

        // 制定验证计划 (per v1 VerifyPlanner); observable=true (默认) 优先观察窗
        let plan = self.plan_verify(&h, true);

        Ok(OrganOutput::Hypothesis {
            id: h.id,
            statement: h.statement.clone(),
            // conf 0.0: 刚登记, 0 装诚实 — 还未验证, 不假装有置信度
            // per v1 4 态状态机: Conjecture 阶段无 confidence 概念
            conf: 0.0,
        })
        // 注: `plan` 当前**不返** (per OrganOutput::Hypothesis schema only has id/statement/conf).
        // 0 装诚实: process 仅"登记猜想", 验证由 runtime 后续循环调度.
        // 未来 v2.1 可加 `OrganOutput::Hypothesis { id, statement, conf, plan: VerifyPlan }` 扩展.
        .map(|o| {
            let _ = plan; // 抑制 unused 警告 — 当前 OrganOutput schema 不含 plan
            o
        })
    }

    /// 0 装诚实: v1 hypothesis 是确定性无 LLM, 返 None 不假装.
    fn llm_factory(&self) -> Option<std::sync::Arc<dyn LlmFactory>> {
        None
    }
}

// ============================================
// 单元测试 (1:1 翻译 v1 hypothesis.rs 测试)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_factory() -> std::sync::Arc<dyn LlmFactory> {
        // 用 NoopLlmFactory 占位 (测试不真调 LLM; trait 边界验证用)
        std::sync::Arc::new(apeireth_plugin::llm_factory::NoopLlmFactory)
    }

    fn empty_input() -> OrganInput {
        use apeireth_core::kernel::memory::Episode;
        use apeireth_core::kernel::SessionId;
        let ep = Episode {
            id: "test-episode-0".into(),
            session_id: SessionId::new().to_string(),
            role: "user".into(),
            content: "".into(),
            timestamp: 0,
        };
        OrganInput::new(ep, vec![])
    }

    fn input_with_hints(hints: Vec<String>) -> OrganInput {
        use apeireth_core::kernel::memory::Episode;
        use apeireth_core::kernel::SessionId;
        let ep = Episode {
            id: "test-episode-1".into(),
            session_id: SessionId::new().to_string(),
            role: "user".into(),
            content: "".into(),
            timestamp: 0,
        };
        OrganInput::new(ep, hints)
    }

    /// v1 1:1: conjecture → start_verify → 累积证据 → confirmed
    #[test]
    fn conjecture_to_verify_to_confirm() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        let h = organ.conjecture("主人熬夜 → 次日效率低");
        assert_eq!(h.status, HypothesisStatus::Conjecture);
        assert!(organ.start_verify(h.id).is_ok());
        assert_eq!(organ.get(h.id).unwrap().status, HypothesisStatus::Verifying);

        organ
            .add_evidence(
                h.id,
                Evidence::supporting(
                    EvidenceSource::Observation,
                    1.2,
                    "7 次熬夜记录中 5 次效率低",
                ),
            )
            .unwrap();
        organ
            .add_evidence(
                h.id,
                Evidence::supporting(
                    EvidenceSource::MasterAnswer,
                    1.0,
                    "主人确认: 熬夜后确实没精神",
                ),
            )
            .unwrap();
        assert_eq!(organ.get(h.id).unwrap().status, HypothesisStatus::Confirmed);
    }

    /// v1 1:1: 反驳证据主导 → Refuted
    #[test]
    fn refuting_evidence_outweighs() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        let h = organ.conjecture("雨天 → 主人心情差");
        organ.start_verify(h.id).unwrap();
        organ
            .add_evidence(
                h.id,
                Evidence::supporting(EvidenceSource::Observation, 1.0, "一次雨天低落"),
            )
            .unwrap();
        organ
            .add_evidence(
                h.id,
                Evidence::refuting(EvidenceSource::MasterAnswer, 3.0, "主人: 下雨天其实很舒服"),
            )
            .unwrap();
        assert_eq!(organ.get(h.id).unwrap().status, HypothesisStatus::Refuted);
    }

    /// v1 1:1: 已定论假设不再接受证据
    #[test]
    fn settled_hypothesis_rejects_evidence() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        let h = organ.conjecture("X");
        organ.start_verify(h.id).unwrap();
        organ
            .add_evidence(
                h.id,
                Evidence::supporting(EvidenceSource::Observation, 1.5, "a"),
            )
            .unwrap();
        organ
            .add_evidence(
                h.id,
                Evidence::supporting(EvidenceSource::Observation, 1.0, "b"),
            )
            .unwrap();
        assert_eq!(organ.get(h.id).unwrap().status, HypothesisStatus::Confirmed);
        assert!(
            organ
                .add_evidence(
                    h.id,
                    Evidence::refuting(EvidenceSource::Observation, 5.0, "late")
                )
                .is_err(),
            "已定论假设不接受新证据"
        );
    }

    /// v1 1:1: 单条大权重不能拍板 (min_evidence_to_settle 防)
    #[test]
    fn min_evidence_prevents_single_big_weight_settlement() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        let h = organ.conjecture("Y");
        organ.start_verify(h.id).unwrap();
        // 单条 5.0 支持证据, 但 min_evidence_to_settle=2 → 不能确认
        organ
            .add_evidence(
                h.id,
                Evidence::supporting(EvidenceSource::MasterAnswer, 5.0, "一锤定音"),
            )
            .unwrap();
        assert_ne!(organ.get(h.id).unwrap().status, HypothesisStatus::Confirmed);
    }

    /// v1 1:1: VerifyPlanner 优先低成本观察窗
    #[test]
    fn planner_prefers_low_cost_observation() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        let h = organ.conjecture("可观察命题");
        let plan = organ.plan_verify(&h, true);
        assert_eq!(plan, VerifyPlan::ObserveWindow { hours: 24.0 });
        // 不可观察 → 问主人 (成本在预算内)
        let plan2 = organ.plan_verify(&h, false);
        assert!(matches!(plan2, VerifyPlan::AskMaster { .. }));
    }

    /// v1 1:1: NoopSink 诚实 no-op
    #[test]
    fn noop_sink_is_honest_noop() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        let h = organ.conjecture("noop");
        assert!(organ.reconcile(&h).is_ok());
    }

    /// v2 新增: process() 走完 F4 路径 → OrganOutput::Hypothesis { id, statement, conf: 0.0 }
    #[tokio::test]
    async fn process_returns_hypothesis_output_with_registered_id() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        let output = organ
            .process(input_with_hints(vec!["主人熬夜 → 次日效率低".into()]))
            .await
            .expect("process ok");
        match output {
            OrganOutput::Hypothesis {
                id,
                statement,
                conf,
            } => {
                assert_eq!(id, 1, "首条猜想 id=1");
                assert_eq!(statement, "主人熬夜 → 次日效率低");
                // 0 装诚实: conf=0.0 (Conjecture 阶段无置信度, 不假装)
                assert_eq!(conf, 0.0);
                // 登记已写入 store
                assert_eq!(organ.len(), 1);
                let h = organ.get(1).unwrap();
                assert_eq!(h.status, HypothesisStatus::Conjecture);
            }
            other => panic!("expected Hypothesis output, got {other:?}"),
        }
    }

    /// v2 新增: dry_run 模式不真登记, 返 NotImplemented
    #[tokio::test]
    async fn process_dry_run_returns_not_implemented() {
        let organ = HypothesisOrgan::with_configs(
            test_factory(),
            "minimax-m3",
            HypothesisConfig::default(),
            PlannerConfig::default(),
            Box::new(NoopSink),
            true, // dry_run=true
        );
        let output = organ
            .process(input_with_hints(vec!["test".into()]))
            .await
            .expect("dry-run returns Ok with NotImplemented");
        match output {
            OrganOutput::NotImplemented { organ: k, note } => {
                assert_eq!(k, OrganKind::F4);
                assert!(note.contains("dry-run"));
            }
            other => panic!("expected NotImplemented in dry-run, got {other:?}"),
        }
        // 不真登记
        assert!(organ.is_empty());
    }

    /// 0 装诚实: llm_factory() 返 None (v1 hypothesis 是确定性无 LLM)
    #[test]
    fn llm_factory_returns_none_per_v1_truth() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        assert!(
            organ.llm_factory().is_none(),
            "v1 hypothesis 是确定性无 LLM, v2 不假装能调"
        );
    }

    /// 0 装诚实: organ_id + name 锁定 F4
    #[test]
    fn name_and_organ_id_locked_to_f4() {
        let organ = HypothesisOrgan::new(test_factory(), "minimax-m3");
        assert_eq!(organ.name(), "F4 Hypothesis");
        assert_eq!(organ.organ_id(), OrganKind::F4);
    }

    /// v2 时间注入: Evidence::at_ms() 显式设置时间戳
    #[test]
    fn evidence_at_ms_explicit_injection() {
        let ev = Evidence::supporting(EvidenceSource::Observation, 1.0, "x").at_ms(1_700_000_000);
        assert_eq!(ev.at_ms, 1_700_000_000);
        assert!(ev.weight > 0.0);

        let ev2 = Evidence::refuting(EvidenceSource::MasterAnswer, 2.0, "y").at_ms(42);
        assert_eq!(ev2.at_ms, 42);
        assert!(ev2.weight < 0.0);
    }
}
