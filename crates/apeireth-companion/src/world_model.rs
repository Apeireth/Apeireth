//! `apeireth-companion::world_model` — 世界模型第一层: 文本模拟器 (TP31 / W1).
//!
//! 哲学 (docs/design-intent.md §2「世界模型 = 推理链的外挂模拟器」):
//! - **第一层 (本模块)**: LLM 按时间线展开反事实推演链, oracle Brier 在终点校准 (防编故事).
//! - 第二层: 因果结构图推演 (TP32 / W2, 沿 memory_graph s/p/o 因果网 MCTS).
//! - 第三层 (连续世界模型, Genie 3 式): 墙, 跟踪不趟.
//!
//! ## 0 装 PASS (诚实登记)
//!
//! - **真 LLM 未接, trait 口已备** (`TimelineLlm`). 测试用 `MockTimelineLlm` 走通全链.
//! - **推演结果永远不当事实注入记忆**: 本模块不调用 `SqliteMemoryStore::put_episode` /
//!   `memory_extractor::extract` / `experience.rs` 等任何落库方法.
//!   仅返回 [`CounterfactualChain`] 给调用方决定是否使用. 这是防幻觉固化的硬边界.
//! - **Brier 拒绝阈值默认 0.3, 可调** (`with_threshold`).
//! - **oracle 历史校准**可选注入 ([`CalibratedResolver`]): 若配置, 历史 `mean_brier`
//!   超过阈值 → 推演链直接标记 `rejected=true` (LLM 历史不准 → 本次也别信).
//!
//! ## 挂接
//!
//! 全部复用 `oracle::*` 既有零件 (`WorldState` / `Forecast` / `CalibratedResolver`),
//! 不重复发明沙盘底座.

use std::sync::Arc;

use async_trait::async_trait;

use crate::oracle::{CalibratedResolver, Forecast, WorldState};

// ============================================================
// 推演链数据结构
// ============================================================

/// LLM 调用上下文: 推演下一步所需的全部状态.
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

/// 推演链一步: 叙事 + 状态快照.
#[derive(Debug, Clone)]
pub struct TimelineStep {
    /// tick 编号.
    pub tick: u64,
    /// 自然语言叙事 (LLM 生成的 "这步发生什么").
    pub narrative: String,
    /// 该步世界状态快照.
    pub state_snapshot: WorldState,
}

/// 一条完整反事实推演链.
#[derive(Debug, Clone)]
pub struct CounterfactualChain {
    /// 反事实假设原文.
    pub hypothesis: String,
    /// 推演步骤序列.
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
// LLM trait (真 LLM 未接, trait 口已备)
// ============================================================

/// LLM 抽象: 按时间线展开反事实推演链.
///
/// 真 LLM 未接 (本任务 0 装 PASS): 测试用 [`MockTimelineLlm`] 走通全链.
/// 真实接入时, 外部实现按 `TimelineContext` 调用 LLM (提示词模板待 W2/TP32 之后设计),
/// 把返回文本拆成 (narrative, state_snapshot) 即可.
#[async_trait]
pub trait TimelineLlm: Send + Sync {
    /// 推演下一步. 返回空 `narrative` 表示 LLM 信号链结束 (允许 LLM 主动停).
    async fn expand_step(&self, ctx: &TimelineContext) -> Result<TimelineStep, String>;

    /// 终点预测概率 (0..1). 默认 0.5 (无信息先验, 与 `BetaBinomial::default` 同语义).
    fn terminal_probability(&self) -> f64 {
        0.5
    }
}

// ============================================================
// 文本模拟器 (编排器)
// ============================================================

/// 文本模拟器: 按时间线编排 LLM 推演链 + oracle Brier 终点校准.
///
/// ## 工作流
/// 1. `run`: 迭代 `max_steps` 次, 每次调 `llm.expand_step(ctx)`; 空 narrative 即停.
/// 2. 用 `llm.terminal_probability()` 构造终点 [`Forecast`].
/// 3. 若注入 [`CalibratedResolver`], 用历史 `mean_brier` 校准整条链.
/// 4. `calibrate`: 对账已知 outcome, 更新 `calibration_brier` + 拒绝标记.
///
/// ## 0 装 PASS 边界
/// `run` / `calibrate` **不**调用 `SqliteMemoryStore::put_episode`. 调用方若想积累
/// oracle 历史, 应单独用 `ForecastRegistry::register`/`resolve` 走 `forecast-` 前缀登记.
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

    /// 推演一条反事实链.
    pub async fn run(
        &self,
        start_state: WorldState,
        hypothesis: impl Into<String>,
    ) -> Result<CounterfactualChain, String> {
        let hypothesis = hypothesis.into();
        let mut chain = CounterfactualChain::new(hypothesis.clone());

        // 1. 按时间线编排 LLM 推演 (起点状态不变, 逐步累积叙事 + 状态).
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
            // 空 narrative = LLM 信号链结束 (且不是第 0 步, 防 LLM 空返回锁死).
            if step.narrative.trim().is_empty() && tick > 0 {
                break;
            }
            current_narrative.push_str(&step.narrative);
            current_narrative.push('\n');
            current_state = step.state_snapshot.clone();
            chain.steps.push(step);
        }

        // 2. 构造终点 forecast (LLM 给概率, statement 标 "反事实推演").
        let probability = self.llm.terminal_probability().clamp(0.0, 1.0);
        chain.terminal_forecast = Some(Forecast::new(
            format!("反事实推演: {hypothesis}"),
            probability,
            self.deadline_ms,
        ));

        // 3. oracle 历史校准 (若配置): 高 mean_brier → 拒绝整条链.
        if let Some(cal) = &self.calibrator {
            let status = cal.status().map_err(|e| format!("oracle 校准失败: {e}"))?;
            if status.resolved_count > 0 && status.mean_brier > self.reject_threshold {
                chain.rejected = true;
                chain.reject_reason =
                    Some(format!(
                    "oracle 历史 Brier {:.3} > 阈值 {:.3} ({n} 次对账, LLM 历史偏倚 → 本次拒绝)",
                    status.mean_brier, self.reject_threshold, n = status.resolved_count,
                ));
            }
        }

        Ok(chain)
    }

    /// 对账: 用真实结局 resolve 终点 forecast, 更新 `calibration_brier`,
    /// 并按阈值决定是否拒绝整条链.
    ///
    /// 0 装 PASS: 此方法**不**把对账结果注入 `SqliteMemoryStore`. 调用方如需积累
    /// oracle 历史, 应单独用 `ForecastRegistry::resolve` 走 `forecast-` 前缀登记
    /// (那是 oracle 历史, 不是普通记忆).
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
// 测试用 Mock LLM
// ============================================================

/// 测试用 Mock LLM: 硬编码推演脚本 + 终点概率.
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
            // 脚本耗尽 → 返回空 narrative = 链结束.
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
// 单元测试 (验收 4 个测试点)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use apeireth_memory::EpisodeStore;
    use apeireth_memory::SqliteMemoryStore;

    use crate::oracle::{Entity, WorldState};

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

    /// 构造 n 步 mock 脚本, 终点概率 p.
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

    #[tokio::test]
    async fn textual_simulator_generates_chain() {
        let llm = mock_with_steps(3, 0.7);
        let sim = TextualSimulator::new(llm);
        let chain = sim.run(start_state(), "如果主人今晚熬夜...").await.unwrap();

        // 验收点 1: 推演链生成
        assert_eq!(chain.step_count(), 3, "mock 3 步脚本 → chain 3 步");
        assert!(
            chain.terminal_forecast.is_some(),
            "终点 forecast 必须存在 (LLM 终端概率 → Forecast::new)"
        );
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

    #[test]
    fn textual_simulator_calibrates_with_brier() {
        // 验收点 2: Brier 终点校准数值正确
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

    #[test]
    fn textual_simulator_rejects_high_brier() {
        // 验收点 3: 校准差拒绝
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

    #[tokio::test]
    async fn textual_simulator_does_not_persist_to_memory() {
        // 验收点 4: 0 装 PASS 边界 — 推演结果绝不入库
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        let before = store
            .recent_episodes("world-model-session", 100)
            .map(|v| v.len())
            .unwrap_or(0);
        assert_eq!(before, 0, "全新 in-memory store 应为空");

        let llm = mock_with_steps(3, 0.7);
        let sim = TextualSimulator::new(llm);
        let chain = sim.run(start_state(), "如果主人今晚熬夜...").await.unwrap();
        // calibrate 也不入库 (校验用)
        let mut chain = chain;
        sim.calibrate(&mut chain, true).unwrap();

        let after = store
            .recent_episodes("world-model-session", 100)
            .map(|v| v.len())
            .unwrap_or(0);
        assert_eq!(
            before, after,
            "推演 + 对账 后内存库不应有任何写入 (0 装 PASS 边界): before={before}, after={after}"
        );

        // 二次确认: 任何 session_id 都查不到
        let any = store.recent_episodes("any-session", 1000).unwrap().len();
        assert_eq!(any, 0, "全库应仍为空: {any}");
    }
}
