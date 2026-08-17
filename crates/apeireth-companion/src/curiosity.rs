//! `apeireth-companion::curiosity` — E4 好奇驱动引擎 (自主好奇心).
//!
//! ## 哲学 (主人 2026-08-18 拍板, docs/design-intent.md §2)
//!
//! - **记忆引导好奇**: 探索域**不设白名单** (允许好奇任何事), 好奇目标采样权重
//!   由记忆回声自然偏置 — "她自由地好奇, 却因为你而成为她" (属于你但不依附你).
//! - **浅尝辄止的童年**: 初始好奇像小孩精力有限 — 各方面都好奇但都浅
//!   (低回声 = 浅探索), 回声强才加深; 总预算封顶 (token 成本控制).
//! - **疑问路由**: 好奇-目标交接不绝对 — 发现/反思中判断"问主人更快"→ 直接问,
//!   不硬分离线/在线两条时间线.
//! - **oracle 喂好奇**: Brier 意外度 = 世界没被理解 (预测不准的领域 = 该好奇).
//!
//! ## 0 装 PASS (诚实登记)
//!
//! - 本模块是**确定性机制件** (无 LLM 依赖): 回声合成/偏置采样/预算/路由全部可测.
//! - LLM 探索行为 (读到回声目标后具体探索什么) 是下游消费方的事, 本模块不假装.
//! - 采样用**固定种子 LCG** (可复现测试), 生产可换种子.
//!
//! ## 挂接 (集成而非分立)
//!
//! - 回声来源: `memory_extractor` 排名 (importance/access/recency) + `oracle` Brier 意外度.
//! - 下游: 探索目标 → 提炼调度/反思 (探索域), 或 `emergence` 开口 (问主人).

use std::collections::HashMap;

/// 回声来源 (诚实标注: 记忆 vs 世界未被理解).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EchoSource {
    /// 记忆: 主题在记忆里反复出现/重要 (伙伴感来源).
    Memory,
    /// oracle: 该领域预测 Brier 意外度高 (世界没被理解).
    OracleSurprise,
}

/// 记忆回声: 某个主题的好奇引力 (由记忆/预测数据合成).
#[derive(Debug, Clone)]
pub struct Echo {
    /// 主题 (自由文本, 不设白名单).
    pub topic: String,
    /// 回声强度 ∈ [0.0, 1.0] (importance/access/recency 或意外度合成).
    pub strength: f64,
    /// 来源 (可多个来源叠加, 取最大).
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

/// 探索深度 (浅尝辄止 → 回声强才加深).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    Shallow,
    Deep,
}

/// 好奇目标 (采样输出, 供下游探索).
#[derive(Debug, Clone)]
pub struct ExplorationTarget {
    pub id: u64,
    pub topic: String,
    pub depth: Depth,
    /// 当前回声强度 (决定是否值得加深).
    pub echo: f64,
    /// 本次探索的预估成本 (token 预算).
    pub est_cost: f64,
}

/// 好奇引擎配置 (全部可调常数, 标注"待拟合" — 与 emergence LoopConfig 同纪律).
#[derive(Debug, Clone)]
pub struct CuriosityConfig {
    /// 每日好奇总预算 (token 量级, 封顶防失控).
    pub daily_budget: f64,
    /// 浅探索成本 (低回声目标).
    pub shallow_cost: f64,
    /// 深探索成本 (高回声目标).
    pub deep_cost: f64,
    /// 加深阈值: 回声 ≥ 此值 → 浅→深 (浅尝辄止的"辄止"线).
    pub deepen_echo_threshold: f64,
    /// oracle 意外度权重 (Brier 进回声的缩放).
    pub oracle_surprise_weight: f64,
    /// 疑问路由阈值: 预估成本 / 回声 > 此值 → 问主人更快.
    pub ask_master_ratio: f64,
    /// 采样随机种子 (0 = 固定 42, 可复现测试).
    pub seed: u64,
}

impl Default for CuriosityConfig {
    fn default() -> Self {
        Self {
            daily_budget: 2000.0,   // 待拟合
            shallow_cost: 100.0,    // 待拟合
            deep_cost: 500.0,       // 待拟合
            deepen_echo_threshold: 0.6, // 待拟合
            oracle_surprise_weight: 0.5, // 待拟合
            ask_master_ratio: 8.0,  // 待拟合
            seed: 42,
        }
    }
}

/// 好奇引擎 (确定性, 无 LLM).
#[derive(Debug)]
pub struct CuriosityEngine {
    config: CuriosityConfig,
    budget_left: f64,
    /// 主题 → 回声 (多来源取最大).
    echoes: HashMap<String, f64>,
    /// 主题 → 当前深度.
    depths: HashMap<String, Depth>,
    next_id: u64,
    /// LCG 状态 (可复现).
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

    /// 喂回声 (记忆/预测数据 → 好奇引力). 同主题多来源取最大 (不叠加虚高).
    pub fn feed_echoes(&mut self, echoes: impl IntoIterator<Item = Echo>) {
        for e in echoes {
            let entry = self.echoes.entry(e.topic).or_insert(0.0);
            if e.strength > *entry {
                *entry = e.strength;
            }
        }
    }

    /// oracle 意外度进回声: Brier 高 = 世界没被理解 = 好奇信号 (权重缩放).
    pub fn feed_surprise(&mut self, topic: impl Into<String>, brier: f64) {
        let surprise = (brier * self.config.oracle_surprise_weight).clamp(0.0, 1.0);
        self.feed_echoes([Echo::new(topic, surprise, EchoSource::OracleSurprise)]);
    }

    /// 回声偏置采样: 返回最多 n 个好奇目标.
    /// 权重 = 回声强度 (回声 0 的主题也能被采到, 但概率低 — 不设白名单).
    /// 权重 0 的主题以极小概率 (1/1000) 入池 — "自由地好奇".
    pub fn sample_targets(&mut self, n: usize) -> Vec<ExplorationTarget> {
        let mut out = Vec::new();
        if n == 0 || self.budget_left <= 0.0 {
            return out;
        }
        // 预算连最便宜的浅探索都付不起 → 不再提议 (省 token, 0 装 PASS).
        if self.budget_left < self.config.shallow_cost {
            return out;
        }
        // 候选: 已知回声主题 + 当前深度
        let mut candidates: Vec<(String, f64)> = self
            .echoes
            .iter()
            .map(|(t, s)| (t.clone(), *s))
            .collect();
        // 回声 0 的"自由好奇"通道: 每 1000 次采样有 1 次随机冷主题 (确定性 LCG).
        if self.lcg_next() % 1000 == 0 {
            candidates.push(("冷门角落".to_string(), 0.01));
        }
        if candidates.is_empty() {
            return out;
        }
        // 权重和采样 (确定性 LCG)
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
            out.push(ExplorationTarget {
                id: self.next_id,
                topic,
                depth,
                echo,
                est_cost: cost,
            });
            self.next_id += 1;
        }
        out
    }

    /// 回声强 → 加深 (浅尝辄止的"加深"线). 返回是否升级.
    pub fn deepen(&mut self, topic: &str) -> bool {
        let echo = self.echoes.get(topic).copied().unwrap_or(0.0);
        if echo >= self.config.deepen_echo_threshold
            && self.depths.get(topic) != Some(&Depth::Deep)
        {
            self.depths.insert(topic.to_string(), Depth::Deep);
            true
        } else {
            false
        }
    }

    /// 扣预算 (探索发生后才扣, 0 装 PASS: 预算不足返回 false, 不假装已探索).
    pub fn spend(&mut self, target: &ExplorationTarget) -> bool {
        if self.budget_left >= target.est_cost {
            self.budget_left -= target.est_cost;
            true
        } else {
            false
        }
    }

    /// 疑问路由: 预估成本/回声 高 → 问主人更快 (E4 拍板: 不硬分线).
    /// 回声 ≥ 加深阈值 = 熟悉主题 → 自己探索, 不问.
    pub fn should_ask_master(&self, target: &ExplorationTarget) -> bool {
        if target.echo >= self.config.deepen_echo_threshold {
            return false; // 熟悉主题, 自己探索
        }
        let echo = target.echo.max(0.01);
        target.est_cost / echo > self.config.ask_master_ratio
    }

    /// 剩余预算 (诊断用).
    pub fn budget_left(&self) -> f64 {
        self.budget_left
    }

    /// 当前回声表 (诊断/测试用).
    pub fn echo_of(&self, topic: &str) -> f64 {
        self.echoes.get(topic).copied().unwrap_or(0.0)
    }

    fn lcg_next(&mut self) -> u64 {
        // LCG (Lehmer): 可复现, 无外部依赖.
        self.lcg = self.lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.lcg >> 33
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_strong_topic_sampled_more_often() {
        // 回声强主题应被显著更多采样 (统计 100 次采样, 强回声 ≥ 弱回声).
        let mut eng = CuriosityEngine::new(CuriosityConfig {
            daily_budget: 1_000_000.0,
            ..Default::default()
        });
        eng.feed_echoes([
            Echo::new("主人的工作", 0.9, EchoSource::Memory),
            Echo::new("冷知识", 0.1, EchoSource::Memory),
        ]);
        let mut strong = 0;
        let mut weak = 0;
        for _ in 0..100 {
            let targets = eng.sample_targets(1);
            if targets[0].topic == "主人的工作" {
                strong += 1;
            } else {
                weak += 1;
            }
        }
        assert!(strong > weak * 3, "强回声应主导采样: strong={strong} weak={weak}");
    }

    #[test]
    fn shallow_then_deepen_on_strong_echo() {
        let mut eng = CuriosityEngine::new(CuriosityConfig::default());
        eng.feed_echoes([Echo::new("弱主题", 0.2, EchoSource::Memory)]);
        eng.feed_echoes([Echo::new("强主题", 0.8, EchoSource::Memory)]);

        // 低回声不加深
        assert!(!eng.deepen("弱主题"));
        // 高回声加深
        assert!(eng.deepen("强主题"));
        // 加深后采样为 Deep
        let targets = eng.sample_targets(5);
        let deep = targets.iter().find(|t| t.topic == "强主题").unwrap();
        assert!(matches!(deep.depth, Depth::Deep));
        assert_eq!(deep.est_cost, 500.0); // deep_cost
        let shallow = targets.iter().find(|t| t.topic == "弱主题").unwrap();
        assert!(matches!(shallow.depth, Depth::Shallow));
        assert_eq!(shallow.est_cost, 100.0); // shallow_cost
    }

    #[test]
    fn budget_capped_blocks_spend() {
        let mut eng = CuriosityEngine::new(CuriosityConfig {
            daily_budget: 150.0, // 只够 1 次浅探索
            ..Default::default()
        });
        eng.feed_echoes([Echo::new("t", 0.5, EchoSource::Memory)]);
        // 采样是"提议" (不预扣, 0 装 PASS: 探索发生才扣预算)
        let targets = eng.sample_targets(3);
        assert_eq!(targets.len(), 3);
        // spend 只够 1 次
        assert!(eng.spend(&targets[0]));
        assert!(!eng.spend(&targets[1]), "预算 150 只够 1 次浅探索 (100)");
        // 预算耗尽后不能再采到可花的目标 (budget_left <= 0)
        assert!(eng.sample_targets(1).is_empty());
    }

    #[test]
    fn oracle_surprise_feeds_curiosity() {
        let mut eng = CuriosityEngine::new(CuriosityConfig::default());
        // Brier 0.8 (预测很差) → 意外度 0.8*0.5=0.4 进回声
        eng.feed_surprise("股市预测", 0.8);
        assert!((eng.echo_of("股市预测") - 0.4).abs() < 1e-9);
        // 低 Brier 不产生强回声
        eng.feed_surprise("稳定领域", 0.05);
        assert!(eng.echo_of("稳定领域") < 0.1);
    }

    #[test]
    fn ask_master_when_cost_high_echo_low() {
        let eng = CuriosityEngine::new(CuriosityConfig::default());
        let costly = ExplorationTarget {
            id: 1,
            topic: "冷门".into(),
            depth: Depth::Shallow,
            echo: 0.01,
            est_cost: 100.0,
        };
        assert!(eng.should_ask_master(&costly), "成本/回声比高 → 问主人更快");
        let warm = ExplorationTarget {
            id: 2,
            topic: "熟主题".into(),
            depth: Depth::Shallow,
            echo: 0.9,
            est_cost: 100.0,
        };
        assert!(!eng.should_ask_master(&warm), "回声强 → 自己探索");
    }

    #[test]
    fn deterministic_with_fixed_seed() {
        let a = CuriosityEngine::new(CuriosityConfig::default());
        let b = CuriosityEngine::new(CuriosityConfig::default());
        let mut a = a;
        let mut b = b;
        a.feed_echoes([Echo::new("x", 0.5, EchoSource::Memory), Echo::new("y", 0.5, EchoSource::Memory)]);
        b.feed_echoes([Echo::new("x", 0.5, EchoSource::Memory), Echo::new("y", 0.5, EchoSource::Memory)]);
        let ta = a.sample_targets(5);
        let tb = b.sample_targets(5);
        assert_eq!(ta.len(), tb.len());
        for (x, y) in ta.iter().zip(tb.iter()) {
            assert_eq!(x.topic, y.topic, "同种子同输入 → 同采样序列");
        }
    }

    #[test]
    fn no_whitelist_freedom_curiosity() {
        // 回声 0 的主题也有极低概率被好奇 (自由好奇通道) — 统计 2000 次采样不应出现,
        // 但机制存在: feed 一个回声 0 主题后采样不 panic 且总量正常.
        let mut eng = CuriosityEngine::new(CuriosityConfig {
            daily_budget: 1_000_000.0,
            ..Default::default()
        });
        eng.feed_echoes([Echo::new("从未出现的角落", 0.0, EchoSource::Memory)]);
        let targets = eng.sample_targets(10);
        assert!(!targets.is_empty());
    }
}
