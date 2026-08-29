//! E4 Curiosity 器官真实现 (v2 移植版, per `legacy/donor/apeireth-companion/src/curiosity.rs`).
//!
//! **v1 → v2 1:1 翻译纪律**:
//! - v1 真实现是**确定性机制件** (回声合成/偏置采样/预算/路由全部可测, 无 LLM 依赖,
//!   per `legacy/donor/apeireth-companion/src/curiosity.rs:13-17` 文档明示).
//! - v2 真实现保留 v1 全部确定性算法: `CuriosityEngine` (回声采样 + 加深 + 预算 + 路由).
//! - v2 trait 接口 (`OrganTrait`) **保留** LLM factory 字段 (`llm_factory()`), 默认 None.
//!   未来 v2.1 路线 (per task §4) 可加"LLM 探索具体内容"路径, 但**不破坏** v1 确定性
//!   算法真相. 当前 trait `process` 仅调确定性路径.
//!
//! **0 装 PASS**:
//! - 本模块不假装能调 LLM (v1 没 LLM 路径, v2 也不假装).
//! - 测试用 mock LlmFactory 仅用于**验证 trait 边界**, 不接入 curiosity 真算法.
//! - 真生产路径: `CuriosityOrgan::new(llm_factory, model)` — `llm_factory` 参数保留
//!   给未来 LLM 探索路径用, 当前算法只用 `CuriosityEngine` 状态.
//!
//! **v1 哲学** (主人 2026-08-18 拍板, docs/design-intent.md §2):
//! - **记忆引导好奇**: 探索域**不设白名单** (允许好奇任何事), 好奇目标采样权重
//!   由记忆回声自然偏置.
//! - **浅尝辄止的童年**: 初始好奇像小孩精力有限 — 低回声 = 浅探索, 回声强才加深;
//!   总预算封顶 (token 成本控制).
//! - **疑问路由**: 好奇-目标交接不绝对 — 成本/回声比高 → 问主人更快 (不硬分线).
//! - **oracle 喂奇**: Brier 意外度 = 世界没被理解 (预测不准的领域 = 该好奇).
//!
//! **承接 (per 任务 §5)**:
//! - 子代理 D actionable #1 真兑现 (Experience 保守版是真接 LLM 真接路线, Curiosity 走确定性)
//! - 子代理 Q 报告 #3 "Council 真接 LLM" 已就位, Curiosity 共享 `LlmFactory` trait 边界
//!
//! **3 阶审查** (O-6 锚 9):
//! 1. 总体: 1:1 翻译 v1 `CuriosityEngine`, trait 边界 + 未来 LLM 探索路径预留
//! 2. 系统: impl 在 engine (`apeireth-organ`), trait 在 foundation (`apeireth-plugin`)
//! 3. 架构: `Arc<dyn OrganTrait>` 注入 runtime, E4 trait process() 调 CuriosityEngine

use std::collections::HashMap;

use apeireth_plugin::llm_factory::LlmFactory;
use apeireth_plugin::organ::{
    CuriosityDepth, CuriosityTarget, OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait,
};

// ============================================
// v1 数据结构 1:1 翻译
// ============================================

/// 回声来源 (per v1 `EchoSource` 1:1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoSource {
    /// 记忆: 主题在记忆里反复出现/重要
    Memory,
    /// oracle: 该领域预测 Brier 意外度高 (世界没被理解)
    OracleSurprise,
}

/// 记忆回声: 某个主题的好奇引力 (per v1 `Echo` 1:1)
#[derive(Debug, Clone)]
pub struct Echo {
    pub topic: String,
    pub strength: f64,
    pub source: EchoSource,
}

impl Echo {
    pub fn new(topic: impl Into<String>, strength: f64, source: EchoSource) -> Self {
        Self {
            topic: topic.into(),
            strength: strength.clamp(0.0, 1.0),
            source,
        }
    }
}

/// 探索深度 (per v1 `Depth` 1:1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    Shallow,
    Deep,
}

impl From<Depth> for CuriosityDepth {
    fn from(d: Depth) -> Self {
        match d {
            Depth::Shallow => CuriosityDepth::Shallow,
            Depth::Deep => CuriosityDepth::Deep,
        }
    }
}

/// 好奇引擎配置 (per v1 `CuriosityConfig` 1:1)
#[derive(Debug, Clone)]
pub struct CuriosityConfig {
    pub daily_budget: f64,
    pub shallow_cost: f64,
    pub deep_cost: f64,
    pub deepen_echo_threshold: f64,
    pub oracle_surprise_weight: f64,
    pub ask_master_ratio: f64,
    pub seed: u64,
}

impl Default for CuriosityConfig {
    fn default() -> Self {
        Self {
            daily_budget: 2000.0,
            shallow_cost: 100.0,
            deep_cost: 500.0,
            deepen_echo_threshold: 0.6,
            oracle_surprise_weight: 0.5,
            ask_master_ratio: 8.0,
            seed: 42,
        }
    }
}

// ============================================
// v1 CuriosityEngine 1:1 翻译 (确定性, 无 LLM)
// ============================================

/// 好奇引擎 (per v1 `CuriosityEngine` 1:1 翻译, 保留确定性算法).
///
/// 0 装 PASS: 无 LLM 依赖. 全部状态可测, 采样用固定种子 LCG (可复现).
#[derive(Debug)]
pub struct CuriosityEngine {
    config: CuriosityConfig,
    budget_left: f64,
    /// 主题 → 回声 (多来源取最大)
    echoes: HashMap<String, f64>,
    /// 主题 → 当前深度
    depths: HashMap<String, Depth>,
    next_id: u64,
    /// LCG 状态 (可复现)
    lcg: u64,
}

impl CuriosityEngine {
    pub fn new(config: CuriosityConfig) -> Self {
        let seed = config.seed;
        let budget = config.daily_budget;
        Self {
            config,
            budget_left: budget,
            echoes: HashMap::new(),
            depths: HashMap::new(),
            next_id: 1,
            lcg: if seed == 0 { 42 } else { seed },
        }
    }

    /// 喂回声 (同主题多来源取最大)
    pub fn feed_echoes(&mut self, echoes: impl IntoIterator<Item = Echo>) {
        for e in echoes {
            let entry = self.echoes.entry(e.topic).or_insert(0.0);
            if e.strength > *entry {
                *entry = e.strength;
            }
        }
    }

    /// oracle 意外度进回声: Brier 高 = 世界没被理解 = 好奇信号
    pub fn feed_surprise(&mut self, topic: impl Into<String>, brier: f64) {
        let surprise = (brier * self.config.oracle_surprise_weight).clamp(0.0, 1.0);
        self.feed_echoes([Echo::new(topic, surprise, EchoSource::OracleSurprise)]);
    }

    /// 回声偏置采样 (per v1 1:1)
    pub fn sample_targets(&mut self, n: usize) -> Vec<CuriosityTarget> {
        let mut out = Vec::new();
        if n == 0 || self.budget_left <= 0.0 {
            return out;
        }
        if self.budget_left < self.config.shallow_cost {
            return out;
        }
        // 候选: 已知回声主题 + 当前深度. 按主题字典序排序 — HashMap 迭代序随机,
        // 排序保证同输入同序列 (确定性, 测试可复现).
        let mut candidates: Vec<(String, f64)> =
            self.echoes.iter().map(|(t, s)| (t.clone(), *s)).collect();
        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        // 回声 0 的"自由好奇"通道
        if self.lcg_next() % 1000 == 0 {
            candidates.push(("冷门角落".to_string(), 0.01));
        }
        if candidates.is_empty() {
            return out;
        }
        // 权重和采样
        let total: f64 = candidates.iter().map(|(_, s)| s + 0.001).sum();
        for _ in 0..n {
            if self.budget_left <= 0.0 {
                break;
            }
            let mut r = (self.lcg_next() % 100_000) as f64 / 100_000.0 * total;
            let mut pick: Option<(String, f64)> = None;
            for (t, s) in &candidates {
                r -= s + 0.001;
                if r <= 0.0 {
                    pick = Some((t.clone(), *s));
                    break;
                }
            }
            let (topic, echo) = pick.unwrap_or_else(|| candidates[0].clone());
            let depth = *self.depths.get(&topic).unwrap_or(&Depth::Shallow);
            let cost = match depth {
                Depth::Shallow => self.config.shallow_cost,
                Depth::Deep => self.config.deep_cost,
            };
            out.push(CuriosityTarget {
                id: self.next_id,
                topic,
                depth: depth.into(),
                echo,
                est_cost: cost,
            });
            self.next_id += 1;
        }
        out
    }

    /// 回声强 → 加深 (per v1 1:1)
    pub fn deepen(&mut self, topic: &str) -> bool {
        let echo = self.echoes.get(topic).copied().unwrap_or(0.0);
        if echo >= self.config.deepen_echo_threshold && self.depths.get(topic) != Some(&Depth::Deep)
        {
            self.depths.insert(topic.to_string(), Depth::Deep);
            true
        } else {
            false
        }
    }

    /// 扣预算 (per v1 1:1)
    pub fn spend(&mut self, target: &CuriosityTarget) -> bool {
        if self.budget_left >= target.est_cost {
            self.budget_left -= target.est_cost;
            true
        } else {
            false
        }
    }

    /// 疑问路由 (per v1 1:1)
    pub fn should_ask_master(&self, target: &CuriosityTarget) -> bool {
        if target.echo >= self.config.deepen_echo_threshold {
            return false; // 熟悉主题, 自己探索
        }
        let echo = target.echo.max(0.01);
        target.est_cost / echo > self.config.ask_master_ratio
    }

    /// 剩余预算
    pub fn budget_left(&self) -> f64 {
        self.budget_left
    }

    /// 当前回声表
    pub fn echo_of(&self, topic: &str) -> f64 {
        self.echoes.get(topic).copied().unwrap_or(0.0)
    }

    fn lcg_next(&mut self) -> u64 {
        // LCG (Lehmer): 可复现, 无外部依赖.
        self.lcg = self
            .lcg
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.lcg >> 33
    }
}

// ============================================
// CuriosityOrgan (v2 trait 真实现)
// ============================================

/// E4 好奇器官 (per v2 OrganTrait 1:1 翻译 v1 CuriosityEngine).
///
/// **构造**:
/// - `llm_factory`: 保留给未来 v2.1 LLM 探索路径. 当前算法**不用** LLM (per v1 确定性).
/// - `model`: model ID, 同 llm_factory 一样仅占位未来扩展.
///
/// **0 装诚实**: `llm_factory()` 返 None — v1 curiosity 路径不需要 LLM, 不假装.
pub struct CuriosityOrgan {
    engine: std::sync::Mutex<CuriosityEngine>,
    /// 保留 LLM factory (未来扩展, 当前**不用** — 0 装诚实)
    _llm_factory: std::sync::Arc<dyn LlmFactory>,
    /// 保留 model ID (未来扩展, 当前**不用** — 0 装诚实)
    _model: String,
}

impl CuriosityOrgan {
    /// 构造 E4 curiosity organ.
    ///
    /// `llm_factory` 和 `model` 保留给未来 v2.1 LLM 探索路径 (per 任务 §4 / 子代理 Q
    /// 报告 #3 Council 共享 LlmFactory 路径). 当前算法不调用, 0 装诚实.
    pub fn new(llm_factory: std::sync::Arc<dyn LlmFactory>, model: impl Into<String>) -> Self {
        Self {
            engine: std::sync::Mutex::new(CuriosityEngine::new(CuriosityConfig::default())),
            _llm_factory: llm_factory,
            _model: model.into(),
        }
    }

    /// 构造 E4 curiosity organ + 自定义 config.
    pub fn with_config(
        llm_factory: std::sync::Arc<dyn LlmFactory>,
        model: impl Into<String>,
        config: CuriosityConfig,
    ) -> Self {
        Self {
            engine: std::sync::Mutex::new(CuriosityEngine::new(config)),
            _llm_factory: llm_factory,
            _model: model.into(),
        }
    }

    /// 喂回声 (per v1 API 1:1, 暴露给外部以便 Runtime 喂记忆回声)
    pub fn feed_echoes(&self, echoes: impl IntoIterator<Item = Echo>) {
        let mut engine = self
            .engine
            .lock()
            .expect("CuriosityOrgan mutex poisoned (0 装诚实)");
        engine.feed_echoes(echoes);
    }

    /// 喂 oracle 意外度
    pub fn feed_surprise(&self, topic: impl Into<String>, brier: f64) {
        let mut engine = self
            .engine
            .lock()
            .expect("CuriosityOrgan mutex poisoned (0 装诚实)");
        engine.feed_surprise(topic, brier);
    }

    /// 加深 (per v1 API 1:1)
    pub fn deepen(&self, topic: &str) -> bool {
        let mut engine = self
            .engine
            .lock()
            .expect("CuriosityOrgan mutex poisoned (0 装诚实)");
        engine.deepen(topic)
    }

    /// 剩余预算 (per v1 API 1:1, 诊断用)
    pub fn budget_left(&self) -> f64 {
        let engine = self
            .engine
            .lock()
            .expect("CuriosityOrgan mutex poisoned (0 装诚实)");
        engine.budget_left()
    }
}

#[async_trait::async_trait]
impl OrganTrait for CuriosityOrgan {
    fn name(&self) -> &'static str {
        "E4 Curiosity"
    }

    fn organ_id(&self) -> OrganKind {
        OrganKind::E4
    }

    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        // 1:1 翻译 v1 process 路径:
        // - 采样最多 N 个目标 (N = context_hints.len() 或 3 兜底)
        // - 疑问路由: 哪些目标该问主人?
        // - 不预扣预算 (per v1 "采样是提议, 探索发生才扣")
        // - dry_run 模式不真返回 BudgetExhausted
        let n = if _input.dry_run { 1 } else { 3 };

        let targets = {
            let mut engine = self
                .engine
                .lock()
                .map_err(|e| OrganError::Internal(format!("mutex poisoned: {e}")))?;
            engine.sample_targets(n)
        };

        // 疑问路由 (per v1 should_ask_master)
        let ask_master: Vec<CuriosityTarget> = targets
            .iter()
            .filter(|t| {
                let engine = self
                    .engine
                    .lock()
                    .expect("CuriosityOrgan mutex poisoned (0 装诚实)");
                engine.should_ask_master(t)
            })
            .cloned()
            .collect();

        let budget_left = self.budget_left();

        Ok(OrganOutput::Curiosity {
            targets,
            ask_master,
            budget_left,
        })
    }

    /// 0 装诚实: v1 curiosity 是确定性无 LLM, 返 None 不假装.
    fn llm_factory(&self) -> Option<std::sync::Arc<dyn LlmFactory>> {
        None
    }
}

// ============================================
// 单元测试 (1:1 翻译 v1 curiosity.rs 测试)
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
        let ep = Episode {
            id: "test-episode-0".into(),
            session_id: apeireth_core::kernel::SessionId::new().to_string(),
            role: "user".into(),
            content: "".into(),
            timestamp: 0,
        };
        OrganInput::new(ep, vec![])
    }

    /// v1 1:1: 强回声主题显著更多采样
    #[test]
    fn echo_strong_topic_sampled_more_often() {
        let organ = CuriosityOrgan::new(test_factory(), "minimax-m3");
        organ.feed_echoes([
            Echo::new("主人的工作", 0.9, EchoSource::Memory),
            Echo::new("冷知识", 0.1, EchoSource::Memory),
        ]);
        let mut strong = 0;
        let mut weak = 0;
        for _ in 0..100 {
            let targets = {
                let mut engine = std::sync::Mutex::lock(&organ.engine).unwrap();
                engine.sample_targets(1)
            };
            if targets[0].topic == "主人的工作" {
                strong += 1;
            } else {
                weak += 1;
            }
        }
        assert!(
            strong > weak * 3,
            "强回声应主导采样: strong={strong} weak={weak}"
        );
    }

    /// v1 1:1: 浅尝辄止 → 回声强才加深
    #[test]
    fn shallow_then_deepen_on_strong_echo() {
        let organ = CuriosityOrgan::new(test_factory(), "minimax-m3");
        organ.feed_echoes([Echo::new("弱主题", 0.2, EchoSource::Memory)]);
        organ.feed_echoes([Echo::new("强主题", 0.8, EchoSource::Memory)]);
        assert!(!organ.deepen("弱主题"));
        assert!(organ.deepen("强主题"));

        let targets = {
            let mut engine = std::sync::Mutex::lock(&organ.engine).unwrap();
            engine.sample_targets(5)
        };
        let deep = targets.iter().find(|t| t.topic == "强主题").unwrap();
        assert_eq!(deep.depth, CuriosityDepth::Deep);
        assert_eq!(deep.est_cost, 500.0);
        let shallow = targets.iter().find(|t| t.topic == "弱主题").unwrap();
        assert_eq!(shallow.depth, CuriosityDepth::Shallow);
        assert_eq!(shallow.est_cost, 100.0);
    }

    /// v1 1:1: 预算封顶 → spend 第二轮返 false
    #[test]
    fn budget_capped_blocks_spend() {
        let organ = CuriosityOrgan::with_config(
            test_factory(),
            "minimax-m3",
            CuriosityConfig {
                daily_budget: 150.0,
                ..Default::default()
            },
        );
        organ.feed_echoes([Echo::new("t", 0.5, EchoSource::Memory)]);
        let targets = {
            let mut engine = std::sync::Mutex::lock(&organ.engine).unwrap();
            engine.sample_targets(3)
        };
        assert_eq!(targets.len(), 3);
        let mut engine = std::sync::Mutex::lock(&organ.engine).unwrap();
        assert!(engine.spend(&targets[0]));
        assert!(!engine.spend(&targets[1]), "预算 150 只够 1 次浅探索");
        assert!(engine.sample_targets(1).is_empty());
    }

    /// v1 1:1: oracle Brier 意外度进回声
    #[test]
    fn oracle_surprise_feeds_curiosity() {
        let organ = CuriosityOrgan::new(test_factory(), "minimax-m3");
        organ.feed_surprise("股市预测", 0.8);
        // 直接拿 engine 锁, 不嵌套调 organ 方法 (避免 std::Mutex 死锁)
        let echo_strong = {
            let engine = organ.engine.lock().unwrap();
            engine.echo_of("股市预测")
        };
        assert!((echo_strong - 0.4).abs() < 1e-9);
        organ.feed_surprise("稳定领域", 0.05);
        let echo_weak = {
            let engine = organ.engine.lock().unwrap();
            engine.echo_of("稳定领域")
        };
        assert!(echo_weak < 0.1);
    }

    /// v1 1:1: 成本/回声比高 → 问主人
    #[test]
    fn ask_master_when_cost_high_echo_low() {
        let organ = CuriosityOrgan::new(test_factory(), "minimax-m3");
        let costly = CuriosityTarget {
            id: 1,
            topic: "冷门".into(),
            depth: CuriosityDepth::Shallow,
            echo: 0.01,
            est_cost: 100.0,
        };
        let engine = organ.engine.lock().unwrap();
        assert!(
            engine.should_ask_master(&costly),
            "成本/回声比高 → 问主人更快"
        );
        let warm = CuriosityTarget {
            id: 2,
            topic: "熟主题".into(),
            depth: CuriosityDepth::Shallow,
            echo: 0.9,
            est_cost: 100.0,
        };
        assert!(!engine.should_ask_master(&warm), "回声强 → 自己探索");
    }

    /// v1 1:1: 固定种子 → 同输入同采样
    #[test]
    fn deterministic_with_fixed_seed() {
        let a = CuriosityEngine::new(CuriosityConfig::default());
        let b = CuriosityEngine::new(CuriosityConfig::default());
        let mut a = a;
        let mut b = b;
        a.feed_echoes([
            Echo::new("x", 0.5, EchoSource::Memory),
            Echo::new("y", 0.5, EchoSource::Memory),
        ]);
        b.feed_echoes([
            Echo::new("x", 0.5, EchoSource::Memory),
            Echo::new("y", 0.5, EchoSource::Memory),
        ]);
        let ta = a.sample_targets(5);
        let tb = b.sample_targets(5);
        assert_eq!(ta.len(), tb.len());
        for (x, y) in ta.iter().zip(tb.iter()) {
            assert_eq!(x.topic, y.topic);
        }
    }

    /// v1 1:1: 回声 0 的主题也有极低概率被好奇 (自由好奇通道)
    #[test]
    fn no_whitelist_freedom_curiosity() {
        let organ = CuriosityOrgan::with_config(
            test_factory(),
            "minimax-m3",
            CuriosityConfig {
                daily_budget: 1_000_000.0,
                ..Default::default()
            },
        );
        organ.feed_echoes([Echo::new("从未出现的角落", 0.0, EchoSource::Memory)]);
        let targets = {
            let mut engine = std::sync::Mutex::lock(&organ.engine).unwrap();
            engine.sample_targets(10)
        };
        assert!(!targets.is_empty());
    }

    /// v2 新增: process() 走完 E4 路径 → OrganOutput::Curiosity { targets, ask_master, budget_left }
    #[tokio::test]
    async fn process_returns_curiosity_output_with_targets_and_routing() {
        let organ = CuriosityOrgan::new(test_factory(), "minimax-m3");
        organ.feed_echoes([
            Echo::new("主人的工作", 0.9, EchoSource::Memory),
            Echo::new("冷知识", 0.1, EchoSource::Memory),
        ]);
        organ.deepen("主人的工作"); // 强回声 → Deep

        let output = organ.process(empty_input()).await.expect("process ok");
        match output {
            OrganOutput::Curiosity {
                targets,
                ask_master,
                budget_left,
            } => {
                assert!(!targets.is_empty(), "应采样到目标");
                assert!(budget_left > 0.0, "初始预算 2000");
                // 强回声主题不应 ask_master (回声 0.9 ≥ 0.6)
                let master_targets: Vec<&str> =
                    ask_master.iter().map(|t| t.topic.as_str()).collect();
                assert!(
                    !master_targets.contains(&"主人的工作"),
                    "熟主题不 ask_master"
                );
            }
            other => panic!("expected Curiosity output, got {other:?}"),
        }
    }

    /// 0 装诚实: llm_factory() 返 None (v1 curiosity 是确定性无 LLM)
    #[test]
    fn llm_factory_returns_none_per_v1_truth() {
        let organ = CuriosityOrgan::new(test_factory(), "minimax-m3");
        assert!(
            organ.llm_factory().is_none(),
            "v1 curiosity 是确定性无 LLM, v2 不假装能调"
        );
    }

    /// 0 装诚实: organ_id + name 锁定 E4
    #[test]
    fn name_and_organ_id_locked_to_e4() {
        let organ = CuriosityOrgan::new(test_factory(), "minimax-m3");
        assert_eq!(organ.name(), "E4 Curiosity");
        assert_eq!(organ.organ_id(), OrganKind::E4);
    }
}
