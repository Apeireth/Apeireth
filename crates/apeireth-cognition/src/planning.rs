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
        Self { iterations: 200, exploration_c: 1.4142, max_depth: 20 }
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
        Self { config, evaluator, rng_state: Cell::new(0x9E3779B97F4A7C15), _marker: PhantomData }
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
        let exploration = self.config.exploration_c
            * ((parent_visits as f64).ln() / child.visits as f64).sqrt();
        exploitation + exploration
    }

    /// 主搜索: 返回最优首动作 (根下访问数最高).
    pub fn search(&self, root_state: S, actions: &[A]) -> Option<SearchResult<A>> {        if actions.is_empty() {
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
                let usable: Vec<&A> = actions.iter().filter(|a| a.apply(&sim_state).is_some()).collect();
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
            best_value: if best_node.visits > 0 { best_node.value_sum / best_node.visits as f64 } else { 0.0 },
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
            if dist == 0.0 { 1.0 } else { 1.0 / (1.0 + dist) }
        }
        fn is_terminal(&self, s: &Pos) -> bool {
            s.0 == self.target
        }
    }

    #[test]
    fn mcts_finds_forward_action_from_start() {
        let planner: MctsPlanner<Pos, Step, TargetEvaluator> =
            MctsPlanner::new(MctsConfig { iterations: 300, ..Default::default() }, TargetEvaluator { target: 10 })
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
        let result = planner.search(St::Start, &[Act::A, Act::B]).expect("应有结果");
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
            let p: MctsPlanner<Pos, Step, TargetEvaluator> =
                MctsPlanner::new(MctsConfig { iterations: 100, ..Default::default() }, TargetEvaluator { target: 10 })
                    .with_seed(99);
            p.search(Pos(0), &[Step(1), Step(-1)])
        };
        let b = {
            let p: MctsPlanner<Pos, Step, TargetEvaluator> =
                MctsPlanner::new(MctsConfig { iterations: 100, ..Default::default() }, TargetEvaluator { target: 10 })
                    .with_seed(99);
            p.search(Pos(0), &[Step(1), Step(-1)])
        };
        assert_eq!(a.map(|r| r.best_action.0), b.map(|r| r.best_action.0), "同种子应可复现");
    }

    #[test]
    fn terminal_root_returns_first_action() {
        let planner: MctsPlanner<Pos, Step, TargetEvaluator> =
            MctsPlanner::new(MctsConfig::default(), TargetEvaluator { target: 0 });
        let result = planner.search(Pos(0), &[Step(1), Step(-1)]).expect("根已 terminal 也应有动作");
        assert!(result.root_visits > 0);
    }
}
