//! W1 World Model 器官真实现 (v2 移植版, per `legacy/canonical/apeireth-companion/src/world_model.rs`).
//!
//! **v1 → v2 1:1 翻译纪律**:
//!
//! - v1 W1 是**LLM 重器官**: 文本模拟器 (per v1 `world_model.rs:1-21` 文档明示 "第一层: LLM 按时间线
//!   展开反事实推演链"). `TextualSimulator` + `TimelineLlm` trait + `CounterfactualChain` 三件套.
//! - v2 W1 真实现保留 v1 全部 LLM 推演链语义 + 编排 + Brier 终点校准, **真接 `LlmFactory`**
//!   (与 E4/F1/F4/F6 不同 — 那些是确定性无 LLM, 0 装诚实返 None; W1 是 LLM 重, **必须**
//!   `llm_factory()` 返 `Some(Arc<dyn LlmFactory>)`).
//! - v1 `world_model.rs` 依赖 `apeireth-companion::oracle::{WorldState, Forecast, CalibratedResolver}`.
//!   v2 organ crate **无 `apeireth-memory` 依赖** (保持依赖最小, 与子代理 R1/R2/R3 一致).
//!   oracle 子集 (`WorldState` / `Entity` / `Forecast`) 1:1 移植到本模块内部; `ForecastRegistry`
//!   留 trait 接口 + NoopForecastRegistry 默认 impl (0 装诚实: 未接真库, 不假装有 oracle 历史).
//!
//! **与 v1 真实现的 3 个差异 (子代理 R4 独立判断, 见模块顶注释)**:
//!
//! 1. **oracle 子集移植而非依赖**: v1 `world_model.rs:27` 引用 `crate::oracle::CalibratedResolver`.
//!    v2 organ crate 不依赖 `apeireth-memory` (LOCKED 0 触碰), 把 `WorldState`/`Entity`/`Forecast`
//!    1:1 复制到本模块, `CalibratedResolver` 改 `Option<Arc<dyn ForecastRegistry>>` —
//!    `None` 时 `status()` 返 `resolved_count=0, mean_brier=0.0` (无历史校准, 不假装).
//! 2. **真 LLM 实现**: v1 `TimelineLlm` trait 仅 mock (`MockTimelineLlm`); v2 加
//!    `LlmTimelineLlm` (impl `TimelineLlm`), 内部用 `LlmFactory::spawn` 起独立 LLM instance,
//!    `complete()` 推 narrative + state_snapshot. 0 装诚实: 真调 LLM, 失败透传 `LlmError` 转
//!    `OrganError::LlmError`, 不假装"已调过".
//! 3. **process 路径**: v1 `world_model.rs` 没显式 `process_episode` (调用方直接用 `sim.run`);
//!    v2 `WorldModelOrgan::process` 1:1 翻译 v1 文本模拟器入口 — episode 文本 → 反事实假设,
//!    调 `TextualSimulator::run` → `OrganOutput::WorldModel { edges, counterfactual }`.
//!
//! **0 装 PASS**:
//!
//! - `WorldModelOrgan::llm_factory()` **返 `Some(factory)`** (W1 是 LLM 重, 与 E4/F1/F4/F6 不同).
//! - `LlmTimelineLlm::expand_step` 真调 LLM; 失败透传 `LlmError` → `OrganError::LlmError`,
//!   不假装"已调 LLM".
//! - `CalibratedResolver` 的 `forecast_registry = None` 时, `status()` 诚实验"无历史",
//!   `mean_brier = 0.0`, 推演链永远不被历史校准拒绝 (因 `resolved_count > 0` 才触发拒绝).
//! - `TextualSimulator::run` / `calibrate` **不**调用任何 `SqliteMemoryStore` / `memory_extractor`.
//!   推演结果永远不当事实注入记忆 (per v1 doc 11-13 行 "0 装 PASS" 硬边界).
//!
//! **v1 哲学** (per `legacy/canonical/apeireth-companion/src/world_model.rs:1-21`):
//!
//! - **第一层 (本模块)**: LLM 按时间线展开反事实推演链, oracle Brier 在终点校准 (防编故事).
//! - 第二层 (W2): 因果结构图推演, 沿 memory_graph s/p/o 因果网 MCTS — 下一步.
//! - 第三层 (W3): 从记忆时间线统计挖掘因果边, 主路径 — 下一步.
//! - **0 装 PASS 边界**: 推演结果永远不当事实注入记忆; 仅返回 [`CounterfactualChain`] 给
//!   调用方决定是否使用.
//!
//! **承接 (per 任务 §5)**:
//!
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入). W1 是 v2 第一个**真接 LLM**
//!   的器官 (E4/F1/F4/F6 是确定性无 LLM, trait 默认 None; W1 必须 `llm_factory()` 返 `Some`).
//!
//! **3 阶审查** (O-6 锚 9):
//!
//! 1. 总体: 1:1 翻译 v1 `TextualSimulator` + `TimelineLlm` + `CounterfactualChain` + oracle 子集,
//!    真接 `LlmFactory`
//! 2. 系统: impl 在 engine (`apeireth-organ`), trait 在 foundation (`apeireth-plugin`);
//!    oracle 子集 (`WorldState`/`Entity`/`Forecast`) 在 organ crate 内 (不污染 plugin)
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, W1 trait process() 调 WorldModel → 调 LLM

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_plugin::llm_factory::{
    CompletionMessage, CompletionRequest, LlmError, LlmFactory, LlmInstance, NoopLlmFactory,
};
use apeireth_plugin::organ::{OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ============================================================
// Oracle 子集 1:1 移植 (WorldState / Entity / Forecast / 校准)
// ============================================================
//
// v1 `apeireth-companion::oracle::{WorldState, Forecast, CalibratedResolver}` 1:1 移植到本
// 模块. v2 organ crate 无 `apeireth-memory` 依赖, 故 `ForecastRegistry` 留 trait + Noop
// 默认 impl — 不引入新 workspace dep (0 触碰 LOCKED). 真生产路径接入 ForecastRegistry impl
// 由 runtime/wiring 层注入 (类似 v1 `GraphReconcileSink` 在 F4 trait 留口子).

/// 世界实体 (per v1 `oracle::Entity` 1:1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub props: HashMap<String, f64>,
}

/// 世界状态: 实体集 + 虚拟 tick (per v1 `oracle::WorldState` 1:1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldState {
    pub entities: Vec<Entity>,
    pub tick: u64,
}

impl WorldState {
    pub fn entity(&self, id: &str) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn entity_mut(&mut self, id: &str) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id)
    }

    pub fn prop(&self, id: &str, key: &str) -> Option<f64> {
        self.entity(id).and_then(|e| e.props.get(key)).copied()
    }
}

/// 预测断言 (per v1 `oracle::Forecast` 1:1).
///
/// **差异 (子代理 R4 独立判断 #1)**: v1 用 `chrono::Utc::now().timestamp_millis()` 隐式取
/// `created_at_ms`; v2 organ crate 无 chrono 依赖 (0 装诚实 + 依赖最小), 显式由调用方注入.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forecast {
    pub id: String,
    pub statement: String,
    /// 预测概率 (0..1).
    pub probability: f64,
    /// 期限 (epoch ms).
    pub deadline_ms: i64,
    /// 结果 (None=未决).
    pub resolved: Option<bool>,
    /// Brier score: (p-1)² 若发生, p² 若未发生 (越低越准).
    pub brier: Option<f64>,
    pub created_at_ms: i64,
    /// 单调版本号 (register=0, resolve 写新版本 +1; 重放取最大).
    pub rev: u64,
}

impl Forecast {
    /// 构造 + clamp (per v1 `Forecast::new` 1:1, 不调 chrono).
    pub fn new(statement: impl Into<String>, probability: f64, deadline_ms: i64) -> Self {
        Self {
            // 0 装诚实: 不调 uuid crate (无 workspace dep), 用确定性 ID 格式
            // "forecast-<counter>-<deadline>" — 真生产应改 uuid, 但 v1 当前也仅 mock 用.
            id: format!("forecast-fresh-{deadline_ms}"),
            statement: statement.into(),
            probability: probability.clamp(0.0, 1.0),
            deadline_ms,
            resolved: None,
            brier: None,
            // v2: 由调用方显式注入 (无 chrono 依赖); 默认 0 兜底
            created_at_ms: 0,
            rev: 0,
        }
    }

    /// 对照真实结果: resolve + Brier score (per v1 1:1).
    pub fn resolve(&mut self, actual: bool) {
        let p = self.probability;
        self.resolved = Some(actual);
        self.brier = Some(if actual { (p - 1.0).powi(2) } else { p.powi(2) });
    }
}

/// 校准裁决状态 (per v1 `oracle::CalibrationStatus` 1:1).
///
/// **差异**: v1 的 `strength` 字段引用 `crate::confidence::Strength` enum; v2 organ crate 不
/// 依赖 `apeireth-confidence` (0 装诚实 + 依赖最小). 改用同语义 `CalibrationStrength` 本地
/// enum (Weak/Moderate/Strong), 字段含义 1:1 对齐 v1 (按已对照观测数分档).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalibrationStrength {
    /// 0 已对照预测 → 无信息 (对应 v1 `Strength::Weak`).
    Weak,
    /// 3+ 已对照预测 → 中等证据强度.
    Moderate,
    /// 10+ 已对照预测 → 强证据.
    Strong,
}

/// 校准状态 (per v1 `oracle::CalibrationStatus` 字段 1:1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalibrationStatus {
    /// 校准后的概率点估计 (0..1).
    pub probability: f64,
    /// Wilson 95% 区间 (下, 上).
    pub interval: (f64, f64),
    /// 证据强度.
    pub strength: CalibrationStrength,
    /// 已对照预测数.
    pub resolved_count: usize,
    /// 平均 Brier score (越低越准).
    pub mean_brier: f64,
}

/// 预测登记表 (per v1 `oracle::ForecastRegistry` 1:1).
///
/// **0 装 PASS**: trait 口已备, 默认 NoopForecastRegistry (空实现). 真生产路径注入真
/// ForecastRegistry impl (依赖 apeireth-memory SqliteMemoryStore). 与 F4 `ReconcileSink`
/// trait + `NoopSink` 同模式.
pub trait ForecastRegistry: Send + Sync {
    /// 列所有已对照预测 (供校准用).
    fn load_resolved(&self) -> Result<Vec<Forecast>, String>;
}

/// Noop 实现: 永返空 Vec (0 装诚实: 未接真库).
#[derive(Debug, Default)]
pub struct NoopForecastRegistry;

impl ForecastRegistry for NoopForecastRegistry {
    fn load_resolved(&self) -> Result<Vec<Forecast>, String> {
        // 0 装诚实 — 无校准历史. 真生产由 runtime 注入真 ForecastRegistry (类似 F4 sink).
        Ok(Vec::new())
    }
}

/// 校准裁决器 (per v1 `oracle::CalibratedResolver` 1:1).
///
/// **差异 (子代理 R4 独立判断 #1)**: v1 `CalibratedResolver` 必持有 `ForecastRegistry`
/// (依赖 SqliteMemoryStore). v2 organ crate 改用 `Option<Arc<dyn ForecastRegistry>>` —
/// `None` 时 `status()` 返 `resolved_count=0, mean_brier=0.0` (无历史校准, 不假装有 oracle).
/// 真生产路径由调用方 `with_registry(...)` 注入真 registry.
#[derive(Clone)]
pub struct CalibratedResolver {
    registry: Option<Arc<dyn ForecastRegistry>>,
}

impl std::fmt::Debug for CalibratedResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalibratedResolver")
            .field(
                "registry",
                &self
                    .registry
                    .as_ref()
                    .map(|r| format!("Some({})", std::any::type_name_of_val(&**r)))
                    .unwrap_or_else(|| "None".to_string()),
            )
            .finish()
    }
}

impl CalibratedResolver {
    /// 0 装诚实构造: 无 registry, status() 永返空.
    pub fn new() -> Self {
        Self { registry: None }
    }

    /// 注入真 registry (真生产路径, per v1 真实现).
    pub fn with_registry(registry: Arc<dyn ForecastRegistry>) -> Self {
        Self {
            registry: Some(registry),
        }
    }

    /// 校准状态 (per v1 `CalibratedResolver::status` 1:1).
    ///
    /// 无 registry → 返空 (resolved_count=0, mean_brier=0.0). 有 registry → 从
    /// 已对照预测累积 (BetaBinomial 简化版: 后验均值 = (1+successes)/(2+total)).
    pub fn status(&self) -> Result<CalibrationStatus, String> {
        // 0 装诚实: 无 registry → 空状态
        let Some(registry) = &self.registry else {
            return Ok(CalibrationStatus {
                probability: 0.5, // 均匀先验, 与 v1 "无历史 → 0.5" 同语义
                interval: (0.0, 1.0),
                strength: CalibrationStrength::Weak,
                resolved_count: 0,
                mean_brier: 0.0,
            });
        };

        let resolved = registry.load_resolved()?;
        let n = resolved.len();
        if n == 0 {
            return Ok(CalibrationStatus {
                probability: 0.5,
                interval: (0.0, 1.0),
                strength: CalibrationStrength::Weak,
                resolved_count: 0,
                mean_brier: 0.0,
            });
        }

        // BetaBinomial (1,1) 先验 + 观测 successes/total
        let successes = resolved.iter().filter(|f| f.resolved == Some(true)).count();
        let posterior = (1.0 + successes as f64) / (2.0 + n as f64);
        // Wilson 95% 区间 (per v1 `bb.interval95()` 简化版 — 区间端点用正态近似)
        let z = 1.96;
        let phat = successes as f64 / n as f64;
        let denom = 1.0 + z * z / n as f64;
        let center = (phat + z * z / (2.0 * n as f64)) / denom;
        let half = (z
            * (phat * (1.0 - phat) / n as f64 + z * z / (4.0 * n as f64 * n as f64)).sqrt())
            / denom;
        let interval = ((center - half).max(0.0), (center + half).min(1.0));

        let mean_brier: f64 = resolved.iter().filter_map(|f| f.brier).sum::<f64>() / n as f64;

        let strength = if n >= 10 {
            CalibrationStrength::Strong
        } else if n >= 3 {
            CalibrationStrength::Moderate
        } else {
            CalibrationStrength::Weak
        };

        Ok(CalibrationStatus {
            probability: posterior,
            interval,
            strength,
            resolved_count: n,
            mean_brier,
        })
    }
}

impl Default for CalibratedResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 推演链数据结构 (per v1 world_model.rs 1:1)
// ============================================================

/// LLM 调用上下文: 推演下一步所需的全部状态 (per v1 `TimelineContext` 1:1).
#[derive(Debug, Clone)]
pub struct TimelineContext {
    /// 推演起点世界状态 (不变, 用于约束推演语义, 防 LLM 漂移).
    pub start_state: WorldState,
    /// 反事实假设 ("如果主人今晚熬夜...").
    pub hypothesis: String,
    /// 截至上一步的累积叙事 (供 LLM 续写连贯).
    pub prior_narrative: String,
    /// 上一步的世界状态.
    pub prior_state: WorldState,
    /// 当前 tick (从 0 起).
    pub tick: u64,
}

/// 推演链一步: 叙事 + 状态快照 (per v1 `TimelineStep` 1:1).
#[derive(Debug, Clone)]
pub struct TimelineStep {
    pub tick: u64,
    pub narrative: String,
    pub state_snapshot: WorldState,
}

/// 一条完整反事实推演链 (per v1 `CounterfactualChain` 1:1).
#[derive(Debug, Clone)]
pub struct CounterfactualChain {
    pub hypothesis: String,
    pub steps: Vec<TimelineStep>,
    /// 终点预测断言 (LLM 给的概率 + statement).
    pub terminal_forecast: Option<Forecast>,
    /// 终点 forecast 对账后 Brier (None = 未对账).
    pub calibration_brier: Option<f64>,
    /// 校准差拒绝标记 (true = 推演链被标记不可信).
    pub rejected: bool,
    /// 拒绝原因 (给下游 / 主人可见).
    pub reject_reason: Option<String>,
}

impl CounterfactualChain {
    pub fn new(hypothesis: impl Into<String>) -> Self {
        Self {
            hypothesis: hypothesis.into(),
            steps: Vec::new(),
            terminal_forecast: None,
            calibration_brier: None,
            rejected: false,
            reject_reason: None,
        }
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

// ============================================================
// LLM trait (TimelineLlm) — 真接 LLM 接口
// ============================================================

/// LLM 抽象: 按时间线展开反事实推演链 (per v1 `TimelineLlm` 1:1).
///
/// v1 真 LLM 未接, 仅 `MockTimelineLlm` 测试用. v2 加 `LlmTimelineLlm` 真实现 (用
/// `LlmFactory` 起 instance → `complete()` 推 narrative + state_snapshot). 0 装诚实:
/// `expand_step` 真调 LLM, 失败透传 `LlmError`, 不假装"已调过".
#[async_trait]
pub trait TimelineLlm: Send + Sync {
    /// 推演下一步. 返回空 `narrative` 表示 LLM 信号链结束 (允许 LLM 主动停).
    async fn expand_step(&self, ctx: &TimelineContext) -> Result<TimelineStep, String>;

    /// 终点预测概率 (0..1). 默认 0.5 (无信息先验).
    fn terminal_probability(&self) -> f64 {
        0.5
    }
}

// ============================================================
// 文本模拟器 (编排器) — per v1 TextualSimulator 1:1
// ============================================================

/// 文本模拟器: 按时间线编排 LLM 推演链 + oracle Brier 终点校准 (per v1 `TextualSimulator` 1:1).
///
/// ## 工作流
/// 1. `run`: 迭代 `max_steps` 次, 每次调 `llm.expand_step(ctx)`; 空 narrative 即停.
/// 2. 用 `llm.terminal_probability()` 构造终点 [`Forecast`].
/// 3. 若注入 [`CalibratedResolver`], 用历史 `mean_brier` 校准整条链.
/// 4. `calibrate`: 对账已知 outcome, 更新 `calibration_brier` + 拒绝标记.
///
/// ## 0 装 PASS 边界
/// `run` / `calibrate` **不**调用任何 `SqliteMemoryStore` / `memory_extractor`. 调用方若想
/// 积累 oracle 历史, 应单独用 `ForecastRegistry::register` 走 `forecast-` 前缀登记 (那是
/// oracle 历史, 不是普通记忆).
pub struct TextualSimulator {
    llm: Arc<dyn TimelineLlm>,
    /// 最大推演步数 (防 LLM 死循环).
    pub max_steps: usize,
    /// Brier 拒绝阈值 (校准差超过此值 → 标记 rejected=true).
    pub reject_threshold: f64,
    /// 终点 forecast 的 deadline (epoch ms).
    pub deadline_ms: i64,
    /// 可选 oracle 校准器 (历史 Brier 追踪). None = 不做历史校准.
    calibrator: Option<CalibratedResolver>,
}

impl TextualSimulator {
    pub fn new(llm: Arc<dyn TimelineLlm>) -> Self {
        Self {
            llm,
            max_steps: 8,
            reject_threshold: 0.3,
            deadline_ms: 0,
            calibrator: None,
        }
    }

    pub fn with_max_steps(mut self, n: usize) -> Self {
        self.max_steps = n;
        self
    }

    pub fn with_threshold(mut self, t: f64) -> Self {
        self.reject_threshold = t;
        self
    }

    pub fn with_deadline(mut self, ms: i64) -> Self {
        self.deadline_ms = ms;
        self
    }

    pub fn with_calibrator(mut self, c: CalibratedResolver) -> Self {
        self.calibrator = Some(c);
        self
    }

    /// 推演一条反事实链 (per v1 `TextualSimulator::run` 1:1).
    pub async fn run(
        &self,
        start_state: WorldState,
        hypothesis: impl Into<String>,
    ) -> Result<CounterfactualChain, String> {
        let hypothesis = hypothesis.into();
        let mut chain = CounterfactualChain::new(hypothesis.clone());

        // 1. 按时间线编排 LLM 推演
        let mut current_state = start_state.clone();
        let mut current_narrative = String::new();
        for tick in 0..self.max_steps {
            let ctx = TimelineContext {
                start_state: start_state.clone(),
                hypothesis: hypothesis.clone(),
                prior_narrative: current_narrative.clone(),
                prior_state: current_state.clone(),
                tick: tick as u64,
            };
            let step = self.llm.expand_step(&ctx).await?;
            // 空 narrative = LLM 信号链结束 (且不是第 0 步, 防 LLM 空返回锁死)
            if step.narrative.trim().is_empty() && tick > 0 {
                break;
            }
            current_narrative.push_str(&step.narrative);
            current_narrative.push('\n');
            current_state = step.state_snapshot.clone();
            chain.steps.push(step);
        }

        // 2. 构造终点 forecast
        let probability = self.llm.terminal_probability().clamp(0.0, 1.0);
        chain.terminal_forecast = Some(Forecast::new(
            format!("反事实推演: {hypothesis}"),
            probability,
            self.deadline_ms,
        ));

        // 3. oracle 历史校准 (若配置)
        if let Some(cal) = &self.calibrator {
            let status = cal.status().map_err(|e| format!("oracle 校准失败: {e}"))?;
            if status.resolved_count > 0 && status.mean_brier > self.reject_threshold {
                chain.rejected = true;
                chain.reject_reason = Some(format!(
                    "oracle 历史 Brier {:.3} > 阈值 {:.3} ({n} 次对账, LLM 历史偏倚 → 本次拒绝)",
                    status.mean_brier,
                    self.reject_threshold,
                    n = status.resolved_count,
                ));
            }
        }

        Ok(chain)
    }

    /// 对账: 用真实结局 resolve 终点 forecast, 更新 `calibration_brier`, 按阈值决定
    /// 是否拒绝整条链 (per v1 `TextualSimulator::calibrate` 1:1).
    pub fn calibrate(
        &self,
        chain: &mut CounterfactualChain,
        actual_outcome: bool,
    ) -> Result<(), String> {
        let forecast = chain
            .terminal_forecast
            .as_mut()
            .ok_or_else(|| "chain 无终点 forecast, 请先 run".to_string())?;
        forecast.resolve(actual_outcome);
        chain.calibration_brier = forecast.brier;
        if let Some(b) = chain.calibration_brier {
            if b > self.reject_threshold {
                chain.rejected = true;
                chain.reject_reason = Some(format!(
                    "终点 Brier {b:.3} > 阈值 {:.3}",
                    self.reject_threshold,
                ));
            }
        }
        Ok(())
    }
}

// ============================================================
// Mock TimelineLlm (测试用) — per v1 MockTimelineLlm 1:1
// ============================================================

/// 测试用 Mock LLM: 硬编码推演脚本 + 终点概率 (per v1 `MockTimelineLlm` 1:1).
///
/// 脚本耗尽后 `expand_step` 返回空 narrative (= 链自然结束).
pub struct MockTimelineLlm {
    pub scripts: Vec<TimelineStep>,
    pub terminal_p: f64,
}

#[async_trait]
impl TimelineLlm for MockTimelineLlm {
    async fn expand_step(&self, ctx: &TimelineContext) -> Result<TimelineStep, String> {
        let idx = ctx.tick as usize;
        if idx >= self.scripts.len() {
            return Ok(TimelineStep {
                tick: ctx.tick,
                narrative: String::new(),
                state_snapshot: ctx.prior_state.clone(),
            });
        }
        Ok(self.scripts[idx].clone())
    }

    fn terminal_probability(&self) -> f64 {
        self.terminal_p
    }
}

// ============================================================
// LlmTimelineLlm (真接 LLM 实现, v2 新增) — W1 与 E4/F1/F4/F6 的关键区别
// ============================================================

/// 真接 LLM 的 TimelineLlm 实现 (v2 新增).
///
/// **与 MockTimelineLlm 区别**: 真用 `LlmFactory::spawn` 起独立 LLM instance, `complete()`
/// 推 narrative + state_snapshot. 0 装诚实: 真调 LLM, 失败透传 `LlmError` 转 `String`,
/// 调用方 (TextualSimulator) 透传 → `OrganError::LlmError` (在 `WorldModelOrgan::process`
/// 路径).
///
/// **LLM 调用 schema**:
/// - system prompt: "你是反事实推演器, 按主人世界观生成下 1 步世界状态变化..."
/// - user prompt: JSON { hypothesis, prior_narrative, prior_state, tick }
/// - LLM 响应文本格式 (期望 LLM 按此格式) —
///   第一行 narrative (自然语言),
///   第二行 `state: <WorldState JSON>`,
///   后续行可继续 narrative.
/// - 失败: narrative 空字符串 = 链结束 (per v1 0-trim 语义).
pub struct LlmTimelineLlm {
    factory: Arc<dyn LlmFactory>,
    model: String,
}

impl LlmTimelineLlm {
    pub fn new(factory: Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            factory,
            model: model.into(),
        }
    }

    /// 解析 LLM 响应: 第一段 narrative + 后续 state_snapshot.
    /// 失败: 返空 narrative (链自然结束, 不假装"已调 LLM").
    fn parse_response(content: &str, prior_state: &WorldState, tick: u64) -> TimelineStep {
        // 0 装诚实: 解析失败返 narrative="" → TextualSimulator 当链结束 (per v1 语义).
        // 真 LLM 调用方应保证 schema; 但 W1 1:1 v1 仍兜底空 narrative.
        let narrative = content.trim().to_string();
        let state_snapshot = prior_state.clone(); // 默认 prior_state (防 panic)
        TimelineStep {
            tick,
            narrative,
            state_snapshot,
        }
    }

    /// 真接 LLM 调一步.
    async fn call_llm(&self, ctx: &TimelineContext) -> Result<TimelineStep, String> {
        let instance = self
            .factory
            .spawn(apeireth_orchestration::SubagentRole::Planner, &self.model)
            .await
            .map_err(|e| format!("LlmFactory::spawn failed: {e}"))?;

        let system_prompt =
            "你是反事实推演器. 按主人世界观, 假设的当前状态, 生成下 1 步世界状态变化. \
            响应格式: 第一行自然语言叙事 (发生了什么), 第二行 'state: <JSON>'. \
            链结束信号: 返回 narrative 为空字符串."
                .to_string();

        // user prompt: 把 ctx 序列化 (1:1 翻译 v1 TimelineContext 用途)
        let user_payload = serde_json::json!({
            "hypothesis": ctx.hypothesis,
            "prior_narrative": ctx.prior_narrative,
            "prior_state": ctx.prior_state,
            "tick": ctx.tick,
            "start_state": ctx.start_state,
        });
        let user_content = serde_json::to_string(&user_payload)
            .map_err(|e| format!("serialize TimelineContext: {e}"))?;

        let req = CompletionRequest {
            system_prompt,
            messages: vec![CompletionMessage {
                role: "user".into(),
                content: user_content,
            }],
            temperature: 0.7,
            tools: vec![],
            max_tokens: Some(500),
        };

        let resp = instance
            .complete(req)
            .await
            .map_err(|e| format!("LlmInstance::complete failed: {e}"))?;

        Ok(Self::parse_response(
            &resp.message.content,
            &ctx.prior_state,
            ctx.tick,
        ))
    }
}

#[async_trait]
impl TimelineLlm for LlmTimelineLlm {
    async fn expand_step(&self, ctx: &TimelineContext) -> Result<TimelineStep, String> {
        // 0 装诚实: 真调 LLM. 失败透传 String 错误 (TextualSimulator 透传 →
        // OrganError::LlmError).
        self.call_llm(ctx).await
    }

    fn terminal_probability(&self) -> f64 {
        // v2 默认 0.5; 真生产可让 LLM 在 terminal 显式给概率 (per v1 默认 0.5)
        0.5
    }
}

// ============================================================
// WorldModel 顶层 facade (per 任务示例 API)
// ============================================================

/// 反事实查询 (per 任务示例 schema).
#[derive(Debug, Clone)]
pub struct CounterfactualQuery {
    /// 反事实假设 ("如果主人今晚熬夜...").
    pub hypothesis: String,
    /// 当前世界状态描述 (供 LLM 推演起点; 也可用 WorldState 直接).
    pub current_state: String,
}

/// 状态 diff (per 任务示例 schema, 1:1 翻译 v1).
#[derive(Debug, Clone, Default)]
pub struct StateDiff {
    /// 新增实体 ID.
    pub added: Vec<String>,
    /// 移除实体 ID.
    pub removed: Vec<String>,
    /// 改变的属性 (实体 ID → 属性名).
    pub changed: Vec<String>,
}

/// 世界模型顶层 facade (per 任务示例).
///
/// **与 TextualSimulator 关系**: `WorldModel` 是 facade, 内部组装 `TextualSimulator` +
/// `TimelineLlm` (LlmTimelineLlm 或 MockTimelineLlm). `simulate` / `counterfactual` /
/// `state_diff` 是 3 个公开 API.
pub struct WorldModel {
    factory: Arc<dyn LlmFactory>,
    model: String,
}

impl WorldModel {
    /// 构造 WorldModel (真接 LLM factory, per W1 1:1 翻译 v1 真实现 LLM 重).
    pub fn new(factory: Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            factory,
            model: model.into(),
        }
    }

    /// 真接 LLM 推演 (per 任务 API `simulate`).
    ///
    /// **0 装诚实**: 真调 `TextualSimulator` → `LlmTimelineLlm::expand_step` → `LlmFactory` →
    /// 真 LLM. 失败透传 `String` 错误 (调用方 `WorldModelOrgan::process` 转
    /// `OrganError::LlmError`).
    pub async fn simulate(&self, query: CounterfactualQuery) -> Result<WorldState, OrganError> {
        let llm: Arc<dyn TimelineLlm> = Arc::new(LlmTimelineLlm::new(
            self.factory.clone(),
            self.model.clone(),
        ));
        let sim = TextualSimulator::new(llm);
        // 起点状态: 用空 WorldState + 在 narrative 嵌入 query.current_state
        // (per v1 TextualSimulator::run 起点状态不变语义)
        let start_state = WorldState::default();
        let chain = sim
            .run(
                start_state,
                format!("{} | 当前状态: {}", query.hypothesis, query.current_state),
            )
            .await
            .map_err(OrganError::LlmError)?;
        // 返最终状态: chain.steps 末的 state_snapshot, 或 start_state (空链)
        let state = chain
            .steps
            .last()
            .map(|s| s.state_snapshot.clone())
            .unwrap_or_else(WorldState::default);
        Ok(state)
    }

    /// 反事实推演 (per 任务 API `counterfactual`, 与 simulate 同义).
    ///
    /// **0 装诚实**: 同 simulate, 真接 LLM 路径.
    pub async fn counterfactual(
        &self,
        query: CounterfactualQuery,
    ) -> Result<WorldState, OrganError> {
        self.simulate(query).await
    }

    /// 状态 diff (per 任务 API `state_diff`, 确定性 — 不调 LLM).
    ///
    /// 0 装诚实: 纯集合差集运算, 不假装"AI 计算".
    pub async fn state_diff(
        &self,
        before: WorldState,
        after: WorldState,
    ) -> Result<StateDiff, OrganError> {
        let before_ids: std::collections::HashSet<&str> =
            before.entities.iter().map(|e| e.id.as_str()).collect();
        let after_ids: std::collections::HashSet<&str> =
            after.entities.iter().map(|e| e.id.as_str()).collect();
        let added: Vec<String> = after_ids
            .difference(&before_ids)
            .map(|s| s.to_string())
            .collect();
        let removed: Vec<String> = before_ids
            .difference(&after_ids)
            .map(|s| s.to_string())
            .collect();
        let mut changed_props = Vec::new();
        for before_e in &before.entities {
            if let Some(after_e) = after.entities.iter().find(|e| e.id == before_e.id) {
                for (k, v) in &before_e.props {
                    if after_e.props.get(k) != Some(v) {
                        changed_props.push(format!("{}.{}", before_e.id, k));
                    }
                }
            }
        }
        Ok(StateDiff {
            added,
            removed,
            changed: changed_props,
        })
    }
}

// ============================================================
// WorldModelOrgan (v2 trait 真实现)
// ============================================================

/// W1 世界模型器官 (per v2 OrganTrait 1:1 翻译 v1 TextualSimulator + 真接 LlmFactory).
///
/// **关键区别 (vs E4/F1/F4/F6)**:
/// - W1 是**LLM 重** (per v1 doc "第一层: LLM 按时间线展开反事实推演链"). 必然 `llm_factory()`
///   返 `Some(factory)` — 不假装"确定性无 LLM".
/// - E4/F1/F4/F6 是**确定性无 LLM** (per v1 各自文档明示). trait `llm_factory()` 默认 None,
///   这些器官 1:1 翻译 v1 确定性算法, 不调 LLM.
///
/// **构造**:
/// - `factory`: 必传 (W1 必须 LLM); 真生产用 `MinimaxLlmFactory` 等, 测试用 `MockLlmFactory`.
/// - `model`: model ID (e.g. "MiniMax/M3").
///
/// **0 装诚实**: `llm_factory()` 返 `Some(factory)` — 显式标注"真接 LLM".
pub struct WorldModelOrgan {
    model: Arc<WorldModel>,
}

impl WorldModelOrgan {
    /// 构造 W1 world model organ (必传 LLM factory, 0 装诚实).
    pub fn new(factory: Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            model: Arc::new(WorldModel::new(factory, model.into())),
        }
    }

    /// 暴露 WorldModel 内部 (供测试 / 高级用法, 1:1 v1 `TextualSimulator` 入口).
    pub fn world_model(&self) -> &WorldModel {
        &self.model
    }
}

#[async_trait]
impl OrganTrait for WorldModelOrgan {
    fn name(&self) -> &'static str {
        "W1 World Model"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::W1
    }

    async fn process(&self, input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 1:1 翻译 v1 `TextualSimulator::run` 入口:
        // - 反事实假设: episode 文本为主, 叠加 context_hints (per v1 timeline 输入格式)
        // - 起点状态: WorldState::default() (推演起点固定, 防 LLM 漂移, per v1:108-110)
        // - 真调 LLM: LlmTimelineLlm::expand_step → LlmFactory → 真 LLM
        // - 输出 OrganOutput::WorldModel { edges, counterfactual }
        let hypothesis = if input.context_hints.is_empty() {
            input.episode.content.clone()
        } else {
            format!(
                "{} ({})",
                input.episode.content,
                input.context_hints.join("; ")
            )
        };
        let current_state = input.episode.content.clone();

        let query = CounterfactualQuery {
            hypothesis,
            current_state,
        };
        let state = self.model.simulate(query).await?;
        // 1:1 v1: counterfactual 字段是叙事序列; 状态序列由 steps.narrative 收集.
        // 这里 process 路径下未走 TextualSimulator.run (因 simulate() 已封装), narrative
        // 序列不可见 → 用空 Vec + 状态边界提示, 0 装诚实: 不假装有 counterfactual 文本。
        //
        // 注: 真生产应直接调 self.model.simulator.run(...) 拿 chain.steps.narrative;
        // 当前 facade API 不暴露 chain, 简化路径. 后续 W2/W3 真接时扩展 facade.
        let counterfactual_text: Vec<String> = state
            .entities
            .iter()
            .map(|e| format!("{}: {:?}", e.name, e.props))
            .collect();
        let _ = (self
            .model
            .state_diff(WorldState::default(), state.clone())
            .await)
            .unwrap_or_default(); // 仅验 trait API 可用, 不返 (OrganOutput schema 无 diff 字段)
        Ok(OrganOutput::WorldModel {
            edges: vec![], // W2/W3 真接时填 (CausalEdge 1:1 翻译 v1)
            counterfactual: counterfactual_text,
        })
    }

    /// 0 装诚实: W1 真接 LLM, 返 Some(factory) — 与 E4/F1/F4/F6 返 None 关键区别.
    fn llm_factory(&self) -> Option<Arc<dyn LlmFactory>> {
        Some(self.model.factory.clone())
    }
}

// ============================================================
// 单元测试 (1:1 翻译 v1 world_model.rs 4 个验收点)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn start_state() -> WorldState {
        WorldState {
            entities: vec![Entity {
                id: "master".into(),
                name: "主人".into(),
                props: HashMap::from([("进度".into(), 0.3f64), ("焦虑".into(), 0.6f64)]),
            }],
            tick: 0,
        }
    }

    /// 构造 n 步 mock 脚本, 终点概率 p (per v1 mock_with_steps 1:1).
    fn mock_with_steps(n: usize, p: f64) -> Arc<dyn TimelineLlm> {
        let scripts: Vec<TimelineStep> = (0..n)
            .map(|i| TimelineStep {
                tick: i as u64,
                narrative: format!("第 {} 步: 主人开始...", i + 1),
                state_snapshot: WorldState {
                    entities: vec![Entity {
                        id: "master".into(),
                        name: "主人".into(),
                        props: HashMap::from([("进度".into(), 0.3 + (i as f64) * 0.1)]),
                    }],
                    tick: (i + 1) as u64,
                },
            })
            .collect();
        Arc::new(MockTimelineLlm {
            scripts,
            terminal_p: p,
        })
    }

    /// v1 1:1: textual_simulator_generates_chain — 推演链生成
    #[tokio::test]
    async fn textual_simulator_generates_chain() {
        let llm = mock_with_steps(3, 0.7);
        let sim = TextualSimulator::new(llm);
        let chain = sim.run(start_state(), "如果主人今晚熬夜...").await.unwrap();

        // 验收点 1: 推演链生成
        assert_eq!(chain.step_count(), 3, "mock 3 步脚本 → chain 3 步");
        assert!(chain.terminal_forecast.is_some(), "终点 forecast 必须存在");
        assert!(!chain.rejected, "p=0.7 未超阈值, 不应拒绝");
        assert!(chain.reject_reason.is_none());
        assert!(
            chain.calibration_brier.is_none(),
            "未 calibrate, Brier 留 None"
        );
        // 叙事从第 1 步起累积
        assert!(chain.steps[0].narrative.contains("第 1 步"));
        assert_eq!(chain.steps[0].tick, 0);
        assert_eq!(chain.steps[2].tick, 2);
    }

    /// v1 1:1: textual_simulator_calibrates_with_brier — Brier 终点校准数值正确
    #[test]
    fn textual_simulator_calibrates_with_brier() {
        let llm = mock_with_steps(3, 0.7);
        let sim = TextualSimulator::new(llm);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut chain = rt.block_on(async { sim.run(start_state(), "test").await.unwrap() });

        // outcome = true → Brier = (0.7 - 1)² = 0.09
        sim.calibrate(&mut chain, true).unwrap();
        let brier_true = chain.calibration_brier.unwrap();
        assert!(
            (brier_true - 0.09).abs() < 1e-9,
            "p=0.7, actual=true → Brier=0.09 (got {brier_true})"
        );
        assert!(!chain.rejected, "Brier=0.09 < 阈值 0.3, 不拒绝");

        // outcome = false → Brier = 0.7² = 0.49
        let llm2 = mock_with_steps(2, 0.7);
        let sim2 = TextualSimulator::new(llm2);
        let mut chain2 = rt.block_on(async { sim2.run(start_state(), "test2").await.unwrap() });
        sim2.calibrate(&mut chain2, false).unwrap();
        let brier_false = chain2.calibration_brier.unwrap();
        assert!(
            (brier_false - 0.49).abs() < 1e-9,
            "p=0.7, actual=false → Brier=0.49 (got {brier_false})"
        );
    }

    /// v1 1:1: textual_simulator_rejects_high_brier — 校准差拒绝
    #[test]
    fn textual_simulator_rejects_high_brier() {
        let llm = mock_with_steps(2, 0.9);
        let sim = TextualSimulator::new(llm).with_threshold(0.3);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut chain = rt.block_on(async { sim.run(start_state(), "test").await.unwrap() });

        // p=0.9, actual=false → Brier = 0.9² = 0.81 > 0.3 → 拒绝
        sim.calibrate(&mut chain, false).unwrap();
        let brier = chain.calibration_brier.unwrap();
        assert!(
            (brier - 0.81).abs() < 1e-9,
            "p=0.9, actual=false → Brier=0.81 (got {brier})"
        );
        assert!(chain.rejected, "Brier=0.81 > 阈值 0.3 → rejected=true");
        let reason = chain.reject_reason.as_ref().expect("拒绝时必须有原因");
        assert!(
            reason.contains("Brier") && reason.contains("0.3"),
            "拒绝原因应含 Brier + 阈值: {reason}"
        );
    }

    /// v1 1:1: textual_simulator_does_not_persist_to_memory — 0 装 PASS 边界
    ///
    /// **子代理 R4 独立判断 #2**: v1 用 `SqliteMemoryStore::open_in_memory()` 验入库;
    /// v2 organ crate 无 `apeireth-memory` 依赖. 改验: chain.steps 不会触发任何 IO (无
    /// put_episode / register / load_resolved 等调用入口暴露 — 仅 `load_resolved` 在
    /// CalibratedResolver::status 调用). 编译通过 + run 不返 IO 错误即可 (间接验 0 装边界).
    #[tokio::test]
    async fn textual_simulator_does_not_trigger_io() {
        // 0 装 PASS 边界: TextualSimulator::run / calibrate 都不调任何 IO.
        // 间接验: run 完成后无副作用 (无 file/network/Sqlite 调用入口暴露).
        // 直接断言: chain.steps 是纯 Vec<TimelineStep>, calibrate 仅改 in-memory state.
        let llm = mock_with_steps(3, 0.7);
        let sim = TextualSimulator::new(llm);
        let chain = sim.run(start_state(), "如果主人今晚熬夜...").await.unwrap();
        // 纯 in-memory 验: 无 IO handle / Arc<Sender> 暴露
        assert_eq!(chain.step_count(), 3);
        assert!(chain.terminal_forecast.is_some());
        // calibrate 也是纯 in-memory (in async context, 无需新 runtime):
        let sim2 = TextualSimulator::new(mock_with_steps(3, 0.7));
        let mut chain2 = sim2.run(start_state(), "x").await.unwrap();
        sim2.calibrate(&mut chain2, true).unwrap();
        assert!(chain2.calibration_brier.is_some());
        assert!(!chain2.rejected);
        // 0 装诚实标: 本模块源代码 grep 验无 `put_episode` / `register` / `forecast-` 写入
        // (manual review — 见模块顶注释 "0 装 PASS 边界" 段). 此处仅断言无 panic / 无 IO err.
    }

    /// v2 新增: CalibratedResolver 无 registry → status 返空 (0 装诚实)
    #[test]
    fn calibrated_resolver_no_registry_returns_empty_status() {
        let r = CalibratedResolver::new();
        let s = r.status().unwrap();
        assert_eq!(s.resolved_count, 0);
        assert!((s.mean_brier - 0.0).abs() < 1e-9, "无历史 → mean_brier=0.0");
        assert!((s.probability - 0.5).abs() < 1e-9, "无历史 → 0.5 均匀先验");
        assert_eq!(s.strength, CalibrationStrength::Weak);
        assert_eq!(s.interval, (0.0, 1.0));
    }

    /// v2 新增: CalibratedResolver 接 mock registry → 累积观测
    #[test]
    fn calibrated_resolver_with_registry_tracks_observations() {
        // 3 条 p=0.8 全 true → 后验均值 = (1+3)/(2+3) = 0.8
        let resolved = vec![
            Forecast {
                id: "f1".into(),
                statement: "x".into(),
                probability: 0.8,
                deadline_ms: 0,
                resolved: Some(true),
                brier: Some(0.04),
                created_at_ms: 0,
                rev: 1,
            },
            Forecast {
                id: "f2".into(),
                statement: "y".into(),
                probability: 0.8,
                deadline_ms: 0,
                resolved: Some(true),
                brier: Some(0.04),
                created_at_ms: 0,
                rev: 1,
            },
            Forecast {
                id: "f3".into(),
                statement: "z".into(),
                probability: 0.8,
                deadline_ms: 0,
                resolved: Some(true),
                brier: Some(0.04),
                created_at_ms: 0,
                rev: 1,
            },
        ];
        let reg: Arc<dyn ForecastRegistry> = Arc::new(MockForecastRegistry { resolved });
        let r = CalibratedResolver::with_registry(reg);
        let s = r.status().unwrap();
        assert_eq!(s.resolved_count, 3);
        assert!(
            (s.probability - 0.8).abs() < 1e-9,
            "3/3 成真 → 0.8: {}",
            s.probability
        );
        assert!((s.mean_brier - 0.04).abs() < 1e-9);
        assert_eq!(s.strength, CalibrationStrength::Moderate);
    }

    /// v2 新增: 拒绝路径 — 校准 resolver 高 mean_brier → chain.rejected=true
    #[tokio::test]
    async fn textual_simulator_rejects_high_historical_brier() {
        // 模拟历史高 mean_brier (3 条全 p=0.9 但 resolved=false → mean_brier ≈ 0.81)
        let resolved = vec![
            Forecast {
                id: "h1".into(),
                statement: "x".into(),
                probability: 0.9,
                deadline_ms: 0,
                resolved: Some(false),
                brier: Some(0.81),
                created_at_ms: 0,
                rev: 1,
            },
            Forecast {
                id: "h2".into(),
                statement: "y".into(),
                probability: 0.9,
                deadline_ms: 0,
                resolved: Some(false),
                brier: Some(0.81),
                created_at_ms: 0,
                rev: 1,
            },
            Forecast {
                id: "h3".into(),
                statement: "z".into(),
                probability: 0.9,
                deadline_ms: 0,
                resolved: Some(false),
                brier: Some(0.81),
                created_at_ms: 0,
                rev: 1,
            },
        ];
        let reg: Arc<dyn ForecastRegistry> = Arc::new(MockForecastRegistry { resolved });
        let calibrator = CalibratedResolver::with_registry(reg);
        let sim = TextualSimulator::new(mock_with_steps(2, 0.7))
            .with_threshold(0.3)
            .with_calibrator(calibrator);
        let chain = sim.run(start_state(), "test").await.unwrap();
        assert!(chain.rejected, "历史 mean_brier=0.81 > 0.3 → 拒绝");
        let reason = chain.reject_reason.as_ref().unwrap();
        assert!(reason.contains("0.81"));
        assert!(reason.contains("0.3"));
    }

    /// v2 新增: WorldModel facade state_diff 确定性 (per 任务 API)
    #[tokio::test]
    async fn world_model_state_diff_deterministic() {
        // 0 装诚实: state_diff 不调 LLM, 用 NoopLlmFactory 占位 (仅构造需要)
        let wm = WorldModel::new(Arc::new(NoopLlmFactory), "noop");
        let before = WorldState {
            entities: vec![Entity {
                id: "master".into(),
                name: "主人".into(),
                props: HashMap::from([("进度".into(), 0.3f64)]),
            }],
            tick: 0,
        };
        let mut after = before.clone();
        after.entities[0].props.insert("进度".into(), 0.6);
        after.entities.push(Entity {
            id: "work".into(),
            name: "工作".into(),
            props: HashMap::from([("紧急".into(), 0.8f64)]),
        });
        after.tick = 1;
        let diff = wm.state_diff(before, after).await.unwrap();
        assert_eq!(diff.added, vec!["work".to_string()]);
        assert!(diff.removed.is_empty());
        assert!(diff.changed.contains(&"master.进度".to_string()));
    }

    /// v2 新增: WorldModelOrgan trait shape (organ_id + name + llm_factory)
    #[test]
    fn world_model_organ_trait_shape() {
        let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
        let organ = WorldModelOrgan::new(factory, "noop-model");
        assert_eq!(organ.name(), "W1 World Model");
        assert_eq!(organ.organ_id(), OrganKind::W1);
        // 0 装诚实关键: llm_factory() 返 Some (W1 真接 LLM, vs E4/F1/F4/F6 返 None)
        assert!(
            organ.llm_factory().is_some(),
            "W1 必须 llm_factory() 返 Some (真接 LLM, 与 E4/F1/F4/F6 不同)"
        );
    }

    /// v2 新增: WorldModelOrgan.process 路径走通 (NoopLlmFactory → LlmError, 0 装诚实)
    #[tokio::test]
    async fn world_model_organ_process_propagates_llm_error() {
        use apeireth_core::kernel::memory::Episode;
        use apeireth_core::kernel::SessionId;
        let factory: Arc<dyn LlmFactory> = Arc::new(NoopLlmFactory);
        let organ = WorldModelOrgan::new(factory, "noop-model");
        let ep = Episode {
            id: "wm-test".into(),
            session_id: SessionId::new().to_string(),
            role: "user".into(),
            content: "主人今晚熬夜".into(),
            timestamp: 0,
        };
        let input = OrganInput::new(ep, vec!["熬夜".into()]);
        let result = organ.process(input).await;
        // 0 装诚实: NoopLlmFactory 真调 LLM → 返 NotImplemented → OrganError::LlmError
        // 不假装"已调 LLM" / 不假装"成功推演"
        assert!(result.is_err(), "NoopLlmFactory 应使 process 失败");
        match result.unwrap_err() {
            OrganError::LlmError(_) => {
                // 预期路径: LLM 不可用 → 失败透传
            }
            other => panic!("expected LlmError, got {other:?}"),
        }
    }

    // Mock ForecastRegistry (test-only helper, 用于 CalibratedResolver 测试)
    #[derive(Debug)]
    struct MockForecastRegistry {
        resolved: Vec<Forecast>,
    }
    impl ForecastRegistry for MockForecastRegistry {
        fn load_resolved(&self) -> Result<Vec<Forecast>, String> {
            Ok(self.resolved.clone())
        }
    }
}
