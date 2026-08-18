//! `apeireth-cognition::planning` — MCTS/LATS 规划搜索机制 (审计 P2#7, 2026-08-16).
//!
//! 纯机制 (零 LLM 依赖): 状态/动作/评估全部走 trait, LLM 评估器由宿主注入
//! (0 装 PASS: 本模块提供启发式评估器用于测试, 不假装接了真模型)。
//!
//! - `SearchState` / `SearchAction` / `StateEvaluator` — 三个 trait 口
//! - `MctsConfig` — 迭代数 / 探索常数 / 最大深度
//! - `MctsPlanner::search()` — UCT 选择 → 扩展 → 模拟 → 回溯; 返回最优首动作
//!
//! 与 LATS (Language Agent Tree Search) 的关系: LATS = MCTS + LLM 评估/反思;
//! 本模块的 `StateEvaluator` 即 LLM 评估注入点 (宿主用 LLM 实现 evaluate),
//! 反思扩展 (反思节点) 留待动作空间含"反思"动作的宿主自行组合。

use std::cell::Cell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc; // E2 LATS: LlmValueFunction/ReflectionRefiner 注入用

/// 搜索状态 (可克隆快照).
pub trait SearchState: Clone + Send + Sync + Debug {}
impl<T: Clone + Send + Sync + Debug> SearchState for T {}

/// 搜索动作: 应用到状态, 返回后继状态 (None = 该动作不可用).
pub trait SearchAction<S: SearchState>: Clone + Send + Sync + Debug {
    fn apply(&self, state: &S) -> Option<S>;
}

/// 状态评估器: 0..1 值估计 (1 = 目标达成) + 终止判定.
pub trait StateEvaluator<S: SearchState>: Send + Sync {
    fn evaluate(&self, state: &S) -> f64;
    fn is_terminal(&self, state: &S) -> bool;
}

/// MCTS 配置.
#[derive(Debug, Clone, Copy)]
pub struct MctsConfig {
    /// 总迭代数 (节点访问预算).
    pub iterations: usize,
    /// UCT 探索常数 (默认 sqrt(2) ≈ 1.414).
    pub exploration_c: f64,
    /// 模拟最大深度 (防无限 rollout).
    pub max_depth: usize,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            iterations: 200,
            exploration_c: 1.4142,
            max_depth: 20,
        }
    }
}

/// 搜索结果: 最优首动作 + 统计.
#[derive(Debug, Clone)]
pub struct SearchResult<A> {
    /// 根节点下访问数最高的动作 (若根无子节点 → Err 由调用方处理, 不假装).
    pub best_action: A,
    /// 根节点总访问数.
    pub root_visits: u64,
    /// 最优子节点平均值.
    pub best_value: f64,
    /// 达到的深度.
    pub depth: usize,
}

/// 搜索节点 (arena 索引).
#[derive(Debug)]
struct Node<S: SearchState, A: SearchAction<S>> {
    state: S,
    /// 从父到本节点的动作 (根为 None).
    action: Option<A>,
    parent: Option<usize>,
    children: Vec<usize>,
    visits: u64,
    value_sum: f64,
}

/// MCTS 规划器.
pub struct MctsPlanner<S: SearchState, A: SearchAction<S>, E: StateEvaluator<S>> {
    config: MctsConfig,
    evaluator: E,
    /// 确定性伪随机 (xorshift64*, 无外部依赖; Cell 单线程内部可变).
    rng_state: Cell<u64>,
    /// S/A 仅出现在 Node 泛型与 trait 约束中 — 显式标注拥有关系.
    _marker: PhantomData<fn() -> (S, A)>,
}

impl<S: SearchState, A: SearchAction<S>, E: StateEvaluator<S>> MctsPlanner<S, A, E> {
    pub fn new(config: MctsConfig, evaluator: E) -> Self {
        Self {
            config,
            evaluator,
            rng_state: Cell::new(0x9E3779B97F4A7C15),
            _marker: PhantomData,
        }
    }

    /// 注入确定性种子 (测试可复现).
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.rng_state.set(if seed == 0 { 1 } else { seed });
        self
    }

    fn next_rand(&self) -> u64 {
        // xorshift64*
        let mut x = self.rng_state.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state.set(x);
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    /// UCT: 探索项 + 利用项.
    fn uct(&self, parent_visits: u64, child: &Node<S, A>) -> f64 {
        if child.visits == 0 {
            return f64::INFINITY; // 未访问子节点优先探索
        }
        let exploitation = child.value_sum / child.visits as f64;
        let exploration =
            self.config.exploration_c * ((parent_visits as f64).ln() / child.visits as f64).sqrt();
        exploitation + exploration
    }

    /// 主搜索: 返回最优首动作 (根下访问数最高).
    pub fn search(&self, root_state: S, actions: &[A]) -> Option<SearchResult<A>> {
        if actions.is_empty() {
            return None; // 无可用动作, 如实返回 None
        }
        let mut nodes: Vec<Node<S, A>> = Vec::new();
        let root_idx = 0usize;
        nodes.push(Node {
            state: root_state,
            action: None,
            parent: None,
            children: Vec::new(),
            visits: 0,
            value_sum: 0.0,
        });

        for _ in 0..self.config.iterations {
            // 1. selection: 根 → 叶子 (UCT 下钻)
            let mut cur = root_idx;
            let mut depth = 0usize;
            while !nodes[cur].children.is_empty() && depth < self.config.max_depth {
                let parent_visits = nodes[cur].visits.max(1);
                let best_child = nodes[cur]
                    .children
                    .iter()
                    .max_by(|a, b| {
                        self.uct(parent_visits, &nodes[**a])
                            .partial_cmp(&self.uct(parent_visits, &nodes[**b]))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
                    .unwrap();
                cur = best_child;
                depth += 1;
            }
            // 2. expansion: 叶子扩展全部可用动作
            if nodes[cur].children.is_empty() && depth < self.config.max_depth {
                let leaf_state = nodes[cur].state.clone();
                for a in actions {
                    if let Some(next) = a.apply(&leaf_state) {
                        let idx = nodes.len();
                        nodes[cur].children.push(idx);
                        nodes.push(Node {
                            state: next,
                            action: Some(a.clone()),
                            parent: Some(cur),
                            children: Vec::new(),
                            visits: 0,
                            value_sum: 0.0,
                        });
                    }
                }
            }
            // 3. simulation: 从当前节点随机 rollout (若已 terminal → 直接用评估)
            let mut sim_state = nodes[cur].state.clone();
            let mut sim_depth = depth;
            let mut value = if self.evaluator.is_terminal(&sim_state) {
                1.0
            } else {
                self.evaluator.evaluate(&sim_state)
            };
            while sim_depth < self.config.max_depth && !self.evaluator.is_terminal(&sim_state) {
                let usable: Vec<&A> = actions
                    .iter()
                    .filter(|a| a.apply(&sim_state).is_some())
                    .collect();
                if usable.is_empty() {
                    break;
                }
                let pick = (self.next_rand() as usize) % usable.len();
                if let Some(ns) = usable[pick].apply(&sim_state) {
                    sim_state = ns;
                    sim_depth += 1;
                    if self.evaluator.is_terminal(&sim_state) {
                        value = 1.0;
                        break;
                    }
                } else {
                    break;
                }
            }
            // 4. backpropagation
            let mut back = cur;
            loop {
                nodes[back].visits += 1;
                nodes[back].value_sum += value;
                match nodes[back].parent {
                    Some(p) => back = p,
                    None => break,
                }
            }
        }

        // 根下选访问数最高的子节点
        let root_children: Vec<usize> = nodes[root_idx].children.clone();
        let best = root_children
            .iter()
            .max_by(|a, b| nodes[**a].visits.cmp(&nodes[**b].visits))
            .copied()?;
        let best_node = &nodes[best];
        Some(SearchResult {
            best_action: best_node.action.clone()?,
            root_visits: nodes[root_idx].visits,
            best_value: if best_node.visits > 0 {
                best_node.value_sum / best_node.visits as f64
            } else {
                0.0
            },
            depth: best_node.parent.map(|_| 1).unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 数轴寻路: 状态 = 位置, 动作 = ±1, 目标 = 到达 10.
    #[derive(Debug, Clone)]
    struct Pos(i64);

    #[derive(Debug, Clone)]
    struct Step(i64);

    impl SearchAction<Pos> for Step {
        fn apply(&self, s: &Pos) -> Option<Pos> {
            Some(Pos(s.0 + self.0))
        }
    }

    struct TargetEvaluator {
        target: i64,
    }

    impl StateEvaluator<Pos> for TargetEvaluator {
        fn evaluate(&self, s: &Pos) -> f64 {
            let dist = (s.0 - self.target).unsigned_abs() as f64;
            if dist == 0.0 {
                1.0
            } else {
                1.0 / (1.0 + dist)
            }
        }
        fn is_terminal(&self, s: &Pos) -> bool {
            s.0 == self.target
        }
    }

    #[test]
    fn mcts_finds_forward_action_from_start() {
        let planner: MctsPlanner<Pos, Step, TargetEvaluator> = MctsPlanner::new(
            MctsConfig {
                iterations: 300,
                ..Default::default()
            },
            TargetEvaluator { target: 10 },
        )
        .with_seed(42);
        let actions = vec![Step(1), Step(-1)];
        let result = planner.search(Pos(0), &actions).expect("应有结果");
        assert_eq!(result.best_action.0, 1, "应选前进动作 (从 0 到 10)");
        assert!(result.root_visits > 0);
    }

    #[test]
    fn mcts_prefers_terminal_path_with_greedy_evaluator() {
        // 状态机: 只有一条路径到达目标 (动作 B), 动作 A 死循环
        #[derive(Debug, Clone)]
        enum St {
            Start,
            Loop,
            Goal,
        }
        #[derive(Debug, Clone)]
        enum Act {
            A,
            B,
        }
        impl SearchAction<St> for Act {
            fn apply(&self, s: &St) -> Option<St> {
                match (s, self) {
                    (St::Start, Act::A) => Some(St::Loop),
                    (St::Start, Act::B) => Some(St::Goal),
                    (St::Loop, Act::A) => Some(St::Loop),
                    (St::Loop, Act::B) => Some(St::Goal),
                    _ => None,
                }
            }
        }
        struct GoalEval;
        impl StateEvaluator<St> for GoalEval {
            fn evaluate(&self, s: &St) -> f64 {
                match s {
                    St::Goal => 1.0,
                    _ => 0.1,
                }
            }
            fn is_terminal(&self, s: &St) -> bool {
                matches!(s, St::Goal)
            }
        }
        let planner: MctsPlanner<St, Act, GoalEval> =
            MctsPlanner::new(MctsConfig::default(), GoalEval).with_seed(7);
        let result = planner
            .search(St::Start, &[Act::A, Act::B])
            .expect("应有结果");
        assert!(matches!(result.best_action, Act::B), "应选直达目标的动作");
    }

    #[test]
    fn empty_actions_returns_none() {
        let planner: MctsPlanner<Pos, Step, TargetEvaluator> =
            MctsPlanner::new(MctsConfig::default(), TargetEvaluator { target: 10 });
        assert!(planner.search(Pos(0), &[]).is_none(), "无动作如实返回 None");
    }

    #[test]
    fn deterministic_seed_gives_same_result() {
        let a = {
            let p: MctsPlanner<Pos, Step, TargetEvaluator> = MctsPlanner::new(
                MctsConfig {
                    iterations: 100,
                    ..Default::default()
                },
                TargetEvaluator { target: 10 },
            )
            .with_seed(99);
            p.search(Pos(0), &[Step(1), Step(-1)])
        };
        let b = {
            let p: MctsPlanner<Pos, Step, TargetEvaluator> = MctsPlanner::new(
                MctsConfig {
                    iterations: 100,
                    ..Default::default()
                },
                TargetEvaluator { target: 10 },
            )
            .with_seed(99);
            p.search(Pos(0), &[Step(1), Step(-1)])
        };
        assert_eq!(
            a.map(|r| r.best_action.0),
            b.map(|r| r.best_action.0),
            "同种子应可复现"
        );
    }

    #[test]
    fn terminal_root_returns_first_action() {
        let planner: MctsPlanner<Pos, Step, TargetEvaluator> =
            MctsPlanner::new(MctsConfig::default(), TargetEvaluator { target: 0 });
        let result = planner
            .search(Pos(0), &[Step(1), Step(-1)])
            .expect("根已 terminal 也应有动作");
        assert!(result.root_visits > 0);
    }
}

// ================= E2: LATS 扩展 (LLM value + 反思节点 + max-backup) =================
// 扩展式: 上方 MctsPlanner / StateEvaluator / SearchResult 骨架签名零改动.
// LATS = MCTS + LLM 评估/反思 (Language Agent Tree Search 精神).
// 0 装 PASS: LLM value 留 trait 口 (LlmValueFunction), 确定性启发式版 HeuristicValue 先行;
// 反思节点复用 E1 reflexion 模块产物形态 (反思文本), 经 ReflectionRefiner trait 注入.

/// LATS: LLM value 评估口 (**LLM 版 0 装预留**).
/// 宿主可用 LLM 实现 (输出 0..1 分数); 现提供确定性启发式版 [`HeuristicValue`].
pub trait LlmValueFunction<S: SearchState>: Send + Sync {
    /// 状态值估计 (0..1, 1 = 目标达成); depth 为节点深度 (供步数成本项).
    fn value(&self, state: &S, depth: usize) -> f64;
}

/// 确定性启发式 value (**先行版**): 目标接近度 + 步数成本 + 历史成功率, 加权和 clamp01.
/// 纯函数: 同输入同输出, 0 随机.
pub struct HeuristicValue<S: SearchState> {
    /// 目标接近度语义 (宿主注入, 0..1, 1 = 到达目标).
    pub goal_proximity: Arc<dyn Fn(&S) -> f64 + Send + Sync>,
    /// 历史成功率先验 (0..1).
    pub prior_success: f64,
    /// 每层深度的步数成本 (value 递减量).
    pub step_cost_per_level: f64,
    /// 三项权重 (>= 0).
    pub w_proximity: f64,
    pub w_steps: f64,
    pub w_prior: f64,
}

impl<S: SearchState> LlmValueFunction<S> for HeuristicValue<S> {
    fn value(&self, state: &S, depth: usize) -> f64 {
        let prox = clamp01((self.goal_proximity)(state));
        let steps = (1.0 - depth as f64 * self.step_cost_per_level).max(0.0);
        clamp01(
            self.w_proximity * prox
                + self.w_steps * steps
                + self.w_prior * clamp01(self.prior_success),
        )
    }
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// 反思节点: reflect→refine. 接收反思文本 (**E1 reflexion 模块产物**: `[反思·...]` 文本),
/// 产出 refined 后继状态 (None = 该反思不适用于此状态).
pub trait ReflectionRefiner<S: SearchState>: Send + Sync {
    fn refine(&self, state: &S, reflection_text: &str) -> Option<S>;
}

/// LATS 配置.
#[derive(Debug, Clone, Copy)]
pub struct LatsConfig {
    /// 总迭代数.
    pub iterations: usize,
    /// UCT 探索常数.
    pub exploration_c: f64,
    /// 最大深度.
    pub max_depth: usize,
    /// 每节点反思扩展使用的反思文本数 (0 = 禁用反思节点).
    pub reflections_per_node: usize,
}

impl Default for LatsConfig {
    fn default() -> Self {
        Self {
            iterations: 200,
            exploration_c: 1.4142,
            max_depth: 20,
            reflections_per_node: 1,
        }
    }
}

/// LATS 节点 (arena 索引). 与 MCTS 节点差异: best_value (max-backup) + is_reflection 标记.
#[derive(Debug)]
struct LatsNode<S: SearchState, A: SearchAction<S>> {
    state: S,
    /// 从父到本节点的动作; None = 反思节点 (无对外动作, 状态由反思文本 refine 而来).
    action: Option<A>,
    is_reflection: bool,
    parent: Option<usize>,
    children: Vec<usize>,
    visits: u64,
    /// **max-backup**: 子树最大值 (替代 MCTS 平均回溯).
    best_value: f64,
}

/// LATS 规划器: UCT 选择 → 扩展 (普通动作 + 反思节点) → LLM/启发式 value → max-backup.
/// 无随机 rollout → 搜索全程确定性 (同输入同输出).
pub struct LatsPlanner<S: SearchState, A: SearchAction<S>, V: LlmValueFunction<S>> {
    config: LatsConfig,
    value_fn: V,
    refiner: Option<Arc<dyn ReflectionRefiner<S>>>,
    /// 反思文本列表 (E1 reflexion 模块产物 ReflectionText::text).
    reflections: Vec<String>,
    _marker: PhantomData<fn() -> (S, A)>,
}

impl<S: SearchState, A: SearchAction<S>, V: LlmValueFunction<S>> LatsPlanner<S, A, V> {
    pub fn new(config: LatsConfig, value_fn: V) -> Self {
        Self {
            config,
            value_fn,
            refiner: None,
            reflections: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// 注入反思精炼器 + 反思文本 (E1 产物); 不注入则无反思节点.
    pub fn with_refiner(
        mut self,
        refiner: Arc<dyn ReflectionRefiner<S>>,
        reflections: Vec<String>,
    ) -> Self {
        self.refiner = Some(refiner);
        self.reflections = reflections;
        self
    }

    /// UCT: 利用项用 best_value (max-backup 语义).
    fn uct(&self, parent_visits: u64, child: &LatsNode<S, A>) -> f64 {
        if child.visits == 0 {
            return f64::INFINITY;
        }
        let exploration =
            self.config.exploration_c * ((parent_visits as f64).ln() / child.visits as f64).sqrt();
        child.best_value + exploration
    }

    /// 主搜索: 返回最优首动作 (根下访问数最高的**动作**子节点; 反思子节点不作为首动作, 如实).
    pub fn search(&self, root_state: S, actions: &[A]) -> Option<SearchResult<A>> {
        if actions.is_empty() {
            return None; // 无可用动作, 如实返回 None
        }
        let mut nodes: Vec<LatsNode<S, A>> = Vec::new();
        let root_idx = 0usize;
        nodes.push(LatsNode {
            state: root_state,
            action: None,
            is_reflection: false,
            parent: None,
            children: Vec::new(),
            visits: 0,
            best_value: 0.0,
        });

        for _ in 0..self.config.iterations {
            // 1. selection: 根 → 叶 (UCT 下钻)
            let mut cur = root_idx;
            let mut depth = 0usize;
            while !nodes[cur].children.is_empty() && depth < self.config.max_depth {
                let parent_visits = nodes[cur].visits.max(1);
                let best_child = nodes[cur]
                    .children
                    .iter()
                    .max_by(|a, b| {
                        self.uct(parent_visits, &nodes[**a])
                            .partial_cmp(&self.uct(parent_visits, &nodes[**b]))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
                    .unwrap();
                cur = best_child;
                depth += 1;
            }
            // 2. expansion: 普通动作子节点 + 反思节点 (深度 >= 1, 反思产物精炼状态)
            if nodes[cur].children.is_empty() && depth < self.config.max_depth {
                let leaf_state = nodes[cur].state.clone();
                for a in actions {
                    if let Some(next) = a.apply(&leaf_state) {
                        let idx = nodes.len();
                        nodes[cur].children.push(idx);
                        nodes.push(LatsNode {
                            state: next,
                            action: Some(a.clone()),
                            is_reflection: false,
                            parent: Some(cur),
                            children: Vec::new(),
                            visits: 0,
                            best_value: 0.0,
                        });
                    }
                }
                if depth >= 1 {
                    if let Some(refiner) = &self.refiner {
                        for text in self
                            .reflections
                            .iter()
                            .take(self.config.reflections_per_node)
                        {
                            if let Some(refined) = refiner.refine(&leaf_state, text) {
                                let idx = nodes.len();
                                nodes[cur].children.push(idx);
                                nodes.push(LatsNode {
                                    state: refined,
                                    action: None,
                                    is_reflection: true,
                                    parent: Some(cur),
                                    children: Vec::new(),
                                    visits: 0,
                                    best_value: 0.0,
                                });
                            }
                        }
                    }
                }
            }
            // 3. evaluation: LLM/启发式 value 直接估当前节点 (LATS 以评估替代随机 rollout)
            let value = self.value_fn.value(&nodes[cur].state, depth);
            // 4. backpropagation: **max-backup** (取最大值向上传播, 替代平均)
            let mut back = cur;
            loop {
                nodes[back].visits += 1;
                if value > nodes[back].best_value {
                    nodes[back].best_value = value;
                }
                match nodes[back].parent {
                    Some(p) => back = p,
                    None => break,
                }
            }
        }

        // 根下选访问数最高的动作子节点 (反思子节点如实排除)
        let root_children: Vec<usize> = nodes[root_idx]
            .children
            .iter()
            .copied()
            .filter(|i| nodes[*i].action.is_some())
            .collect();
        let best = root_children
            .iter()
            .max_by(|a, b| nodes[**a].visits.cmp(&nodes[**b].visits))
            .copied()?;
        let best_node = &nodes[best];
        Some(SearchResult {
            best_action: best_node.action.clone()?,
            root_visits: nodes[root_idx].visits,
            best_value: best_node.best_value,
            depth: best_node.parent.map(|_| 1).unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod lats_tests {
    use super::*;

    /// 数轴状态 (复用骨架测试同款语义).
    #[derive(Debug, Clone)]
    struct Pos(i64);

    #[derive(Debug, Clone)]
    struct Step(i64);

    impl SearchAction<Pos> for Step {
        fn apply(&self, s: &Pos) -> Option<Pos> {
            Some(Pos(s.0 + self.0))
        }
    }

    fn proximity_to(target: i64) -> Arc<dyn Fn(&Pos) -> f64 + Send + Sync> {
        Arc::new(move |p: &Pos| {
            let dist = (p.0 - target).unsigned_abs() as f64;
            if dist == 0.0 {
                1.0
            } else {
                1.0 / (1.0 + dist)
            }
        })
    }

    fn heuristic(target: i64) -> HeuristicValue<Pos> {
        HeuristicValue {
            goal_proximity: proximity_to(target),
            prior_success: 0.0,
            step_cost_per_level: 0.0,
            w_proximity: 1.0,
            w_steps: 0.0,
            w_prior: 0.0,
        }
    }

    /// 常量 value: 验证 LLM trait 口可插拔 (0 装下以假实现驱动).
    struct ConstValue(f64);
    impl LlmValueFunction<Pos> for ConstValue {
        fn value(&self, _s: &Pos, _d: usize) -> f64 {
            self.0
        }
    }

    /// 反思精炼器: 把状态 refine 到目标附近 (模拟 E1 反思产物驱动的修正).
    struct JumpRefiner {
        target: i64,
    }
    impl ReflectionRefiner<Pos> for JumpRefiner {
        fn refine(&self, _s: &Pos, text: &str) -> Option<Pos> {
            // 只接受 E1 反思产物形态的文本 (诚实: 非反思文本不精炼)
            if text.starts_with("[反思·") {
                Some(Pos(self.target - 1))
            } else {
                None
            }
        }
    }

    #[test]
    fn heuristic_value_deterministic_combo() {
        let hv = HeuristicValue {
            goal_proximity: proximity_to(10),
            prior_success: 0.6,
            step_cost_per_level: 0.1,
            w_proximity: 0.5,
            w_steps: 0.3,
            w_prior: 0.2,
        };
        let v0 = hv.value(&Pos(10), 0); // prox=1, steps=1 → 0.5+0.3+0.12=0.92
        assert!((v0 - 0.92).abs() < 1e-9, "组合分: {v0}");
        let v1 = hv.value(&Pos(10), 15); // steps = max(0, 1-1.5)=0 → 0.5+0.12=0.62
        assert!((v1 - 0.62).abs() < 1e-9, "深度成本截零: {v1}");
        for _ in 0..5 {
            assert_eq!(hv.value(&Pos(3), 2), hv.value(&Pos(3), 2), "确定性");
        }
        // clamp01 上限
        let big = HeuristicValue {
            goal_proximity: proximity_to(10),
            prior_success: 5.0,
            step_cost_per_level: 0.0,
            w_proximity: 9.0,
            w_steps: 9.0,
            w_prior: 9.0,
        };
        assert!(big.value(&Pos(10), 0) <= 1.0, "clamp01 上限");
    }

    #[test]
    fn llm_value_trait_slot_pluggable() {
        let planner: LatsPlanner<Pos, Step, ConstValue> =
            LatsPlanner::new(LatsConfig::default(), ConstValue(0.7));
        let r = planner
            .search(Pos(0), &[Step(1), Step(-1)])
            .expect("应有结果");
        assert!(
            (r.best_value - 0.7).abs() < 1e-9,
            "trait 口 value 直通: {}",
            r.best_value
        );
        assert!(r.root_visits > 0);
    }

    #[test]
    fn max_backup_propagates_max_not_average() {
        // 动作 Step(0)=原地 (子树里混有高值分支), Step(-1)=远离.
        // max-backup: Step(0) 子树含 value=1.0 路径 → 其 best_value 应为 1.0 (平均回溯则 < 1).
        let planner: LatsPlanner<Pos, Step, HeuristicValue<Pos>> = LatsPlanner::new(
            LatsConfig {
                iterations: 400,
                ..Default::default()
            },
            heuristic(10),
        );
        let r = planner
            .search(Pos(0), &[Step(1), Step(-1)])
            .expect("应有结果");
        // Step(1) 方向逼近目标, 深路径中 proximity 递增, max-backup 应捕捉子树最大值
        assert!(
            r.best_value > 0.4,
            "max-backup 应传播子树高值: {}",
            r.best_value
        );
        assert!(matches!(r.best_action, Step(1)), "应选前进方向");
    }

    #[test]
    fn reflection_node_enters_tree_and_lifts_value() {
        // 对照组: 无反思
        let base: LatsPlanner<Pos, Step, HeuristicValue<Pos>> = LatsPlanner::new(
            LatsConfig {
                iterations: 300,
                max_depth: 4,
                ..Default::default()
            },
            heuristic(10),
        );
        let base_r = base.search(Pos(0), &[Step(1)]).expect("对照应有结果");
        // 实验组: 注入 E1 形态反思文本 + refiner (refine 到目标附近)
        let lats: LatsPlanner<Pos, Step, HeuristicValue<Pos>> = LatsPlanner::new(
            LatsConfig {
                iterations: 300,
                max_depth: 4,
                ..Default::default()
            },
            heuristic(10),
        )
        .with_refiner(
            Arc::new(JumpRefiner { target: 10 }),
            vec![
                "[反思·验证失败] task_type=navigate | 教训: 验收未达成 | 重试策略: 直奔目标附近"
                    .to_string(),
            ],
        );
        let r = lats.search(Pos(0), &[Step(1)]).expect("应有结果");
        assert!(
            r.best_value > base_r.best_value,
            "反思节点应提升树值: 反思 {} vs 无反思 {}",
            r.best_value,
            base_r.best_value
        );
        assert!(
            r.best_value >= 0.4,
            "refine 到目标附近应有高值: {}",
            r.best_value
        );
    }

    #[test]
    fn non_reflection_text_not_refined() {
        // 非 E1 反思形态文本 → refiner 返回 None → 不产生反思节点 (诚实)
        let lats: LatsPlanner<Pos, Step, HeuristicValue<Pos>> = LatsPlanner::new(
            LatsConfig {
                iterations: 100,
                ..Default::default()
            },
            heuristic(10),
        )
        .with_refiner(
            Arc::new(JumpRefiner { target: 10 }),
            vec!["随便的文本".to_string()],
        );
        let r = lats.search(Pos(0), &[Step(1)]).expect("应有结果");
        assert!(r.root_visits > 0, "无有效反思文本仍正常搜索");
    }

    #[test]
    fn lats_deterministic_same_input_same_output() {
        let run = || {
            let p: LatsPlanner<Pos, Step, HeuristicValue<Pos>> = LatsPlanner::new(
                LatsConfig {
                    iterations: 150,
                    ..Default::default()
                },
                heuristic(10),
            );
            p.search(Pos(0), &[Step(1), Step(-1)])
                .map(|r| (r.best_action.0, r.best_value, r.root_visits))
        };
        let a = run();
        for _ in 0..3 {
            assert_eq!(a, run(), "LATS 无随机 rollout, 同输入同输出");
        }
    }

    #[test]
    fn empty_actions_returns_none() {
        let planner: LatsPlanner<Pos, Step, HeuristicValue<Pos>> =
            LatsPlanner::new(LatsConfig::default(), heuristic(10));
        assert!(planner.search(Pos(0), &[]).is_none(), "无动作如实返回 None");
    }
}
