//! B3 · Phase 3: 上下文在线保留决策（Research 前缀，默认关闭）。
//!
//! # 学术账本（铁律 3）
//! - **问题定义**: 每轮在 token 预算内对上下文段做保留/压缩/折叠/丢弃决策；
//!   现状 `context_rot` rot_score 是逐段贪心启发式，无竞争比定义、无 OPT 参照、无切换代价。
//! - **假设**: ① 等尺寸分桶 + core 钉住 + recency 栈（StackPin）落在经典 paging 抽象内，
//!   继承 LRU 的 k-competitive 上界（Sleator-Tarjan 1985，护栏命题，非新定理）；
//!   ② shadow 记录反事实决策可为价值估计提供 logged 数据（bandit 后悔界路线）。
//! - **状态**: 原型已实现 — `ContextPolicy` trait、`StackPinPolicy`（Proposal A）、
//!   `ShadowLogger`（Proposal C）、离线 replay（合成序列 + Belady OPT + 竞争比测量）。
//!   VaultLRU/FTRL（Proposal B，4–6 人周）留后续批次。
//! - **引用**: `_research_mem/ra/ra3-formal-model-and-algorithms.md`（Q1–Q4、Proposal A/C）；
//!   Sleator & Tarjan 1985（LRU k-competitive）；Zinkevich 2003（OGD O(√T) 后悔）。
//! - **baseline**: `research/baselines/baseline-2026-09-phase0.md`（3061 passed）。
//! - **已知局限**: ① 竞争比只在等尺寸 paging 抽象内成立（H1–H4），真实段尺寸不等时
//!   退化为护栏而非端到端保证；② shadow 日志暂为内存/文件 JSONL，未接生产管线；
//!   ③ 切换代价按"受影响段之后的桶数"粗估（RA-3 §1.4 的 tail 近似）。
//!
//! # 默认关闭（铁律 1 + Phase 3 闸门）
//! - 本模块不挂任何生产装配路径；`context_rot` / `context_budget` 行为零变化。
//! - 所有类型/函数带 `Research` 前缀或在 `research_` 命名空间内，显式调用才生效。

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::context_rot::Segment;

/// 段表示层级（RA-3 §1.1 φ）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchRepr {
    /// 原文。
    Raw,
    /// 摘要压缩（不可逆）。
    Comp,
    /// 无损折叠 marker（可 unfold）。
    Fold,
    /// 丢弃。
    Drop,
}

impl ResearchRepr {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Comp => "comp",
            Self::Fold => "fold",
            Self::Drop => "drop",
        }
    }
}

/// 五动作（RA-3 §1.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchPolicyAction {
    Retain,
    Compress,
    Fold,
    Drop,
    Protect,
}

impl ResearchPolicyAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Retain => "retain",
            Self::Compress => "compress",
            Self::Fold => "fold",
            Self::Drop => "drop",
            Self::Protect => "protect",
        }
    }
}

/// 研究用段快照：产品 `Segment` + token 成本（RA-3 §1.1）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchSegment {
    #[serde(flatten)]
    pub segment: Segment,
    /// token 成本（工程近似 chars/4；精确版用 provider TokenUsage 校准）。
    pub token_cost: usize,
    /// 表示层级（研究策略维护；初始 Raw）。
    pub repr: ResearchRepr,
}

impl ResearchSegment {
    pub fn new(name: impl Into<String>, content: impl Into<String>, token_cost: usize) -> Self {
        Self {
            segment: Segment::new(name, content, 0),
            token_cost,
            repr: ResearchRepr::Raw,
        }
    }

    #[must_use]
    pub fn with_core(mut self, core: bool) -> Self {
        self.segment.core = core;
        self
    }

    #[must_use]
    pub fn with_age(mut self, age_turns: usize) -> Self {
        self.segment.age_turns = age_turns;
        self
    }
}

/// 单轮决策结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchPolicyDecision {
    /// 按输入顺序的动作（core 段恒 Retain/Protect）。
    pub actions: Vec<ResearchPolicyAction>,
    /// 保留段 id 列表（Raw + Fold）。
    pub kept_ids: Vec<String>,
    /// 决策后总 token 成本（Raw + Fold marker 近似；Drop=0）。
    pub total_cost: usize,
    /// 切换代价粗估：发生表示变化的段之后的桶数（RA-3 §1.4 tail 近似）。
    pub switch_cost: usize,
    /// 决策依据（审计/诊断）。
    pub rationale: String,
}

/// 上下文保留策略接口（Phase 3 主交付）。
pub trait ResearchContextPolicy {
    /// 给定当前轮、段集合、query 与预算，输出决策。
    fn decide(
        &mut self,
        turn: u64,
        segments: &[ResearchSegment],
        query: &str,
        budget_tokens: usize,
    ) -> ResearchPolicyDecision;

    /// 策略名（评测矩阵标识）。
    fn name(&self) -> &'static str;
}

/// Proposal A — StackPin：可证竞争比护栏（确定性，等尺寸分桶 + core 钉住 + recency 栈）。
///
/// 假设 H1–H4（RA-3 §5）：
/// - H1 段按 `bucket_tokens` 分桶等尺寸化（桶内视为等尺寸页）；
/// - H2 动作限 Retain/Drop（可选 Fold 叠加层，不影响护栏）；
/// - H3 core 段永久钉住，不计入容量 k；
/// - H4 存在 touch 事件（query 命中/检索命中）。
#[derive(Debug, Clone)]
pub struct ResearchStackPinPolicy {
    /// 保留的非 core 桶容量。
    pub capacity_k: usize,
    /// 分桶等尺寸化单位（token）。
    pub bucket_tokens: usize,
    /// 对被 Drop 的段叠加 Fold（无损召回能力），而非真删。
    pub overlay_fold: bool,
    /// recency 栈：最近 touch 在顶。
    stack: VecDeque<String>,
    /// core 钉住集。
    core: HashSet<String>,
}

impl ResearchStackPinPolicy {
    pub fn new(capacity_k: usize, bucket_tokens: usize, overlay_fold: bool) -> Self {
        Self {
            capacity_k,
            bucket_tokens,
            overlay_fold,
            stack: VecDeque::new(),
            core: HashSet::new(),
        }
    }

    /// touch 事件：段移到 recency 栈顶（H4）。
    pub fn touch(&mut self, id: &str) {
        self.stack.retain(|x| x != id);
        self.stack.push_front(id.to_string());
    }

    /// 钉住 core 段（H3）。
    pub fn protect(&mut self, id: &str) {
        self.core.insert(id.to_string());
    }

    /// 解除钉住。
    pub fn unprotect(&mut self, id: &str) {
        self.core.remove(id);
    }

    /// 分桶数（H1：token 成本按桶上取整）。
    fn buckets_of(&self, cost: usize) -> usize {
        cost.div_ceil(self.bucket_tokens.max(1))
    }
}

impl ResearchContextPolicy for ResearchStackPinPolicy {
    fn name(&self) -> &'static str {
        "StackPin"
    }

    fn decide(
        &mut self,
        _turn: u64,
        segments: &[ResearchSegment],
        _query: &str,
        _budget_tokens: usize,
    ) -> ResearchPolicyDecision {
        // touch 是外部观察事件 (query/检索命中), 由调用方经 `touch()` 显式喂入;
        // decide 只按当前 recency 栈做保留决策 (RA-3 §5 第 1 步在观测侧).
        // 2) 按 recency 栈序取前 capacity_k 个非 core 桶。
        let mut kept: Vec<String> = Vec::new();
        let mut buckets_used = 0usize;
        for id in self.stack.iter() {
            if kept.contains(id) {
                continue;
            }
            let seg = segments.iter().find(|s| s.segment.name == *id);
            let Some(seg) = seg else { continue };
            if seg.segment.core {
                kept.push(id.clone());
                continue; // core 不计入容量 (H3)
            }
            let b = self.buckets_of(seg.token_cost);
            if buckets_used + b > self.capacity_k {
                continue;
            }
            buckets_used += b;
            kept.push(id.clone());
        }
        // 3) core 段无条件保留 (H3: 即使未被 touch 也钉住)。
        for s in segments {
            if s.segment.core && !kept.contains(&s.segment.name) {
                kept.push(s.segment.name.clone());
            }
        }
        // 4) 决策：core → Retain；kept → Retain；其余 → Fold(叠加) 或 Drop。
        let mut actions = Vec::with_capacity(segments.len());
        let mut total_cost = 0usize;
        let mut switch_cost = 0usize;
        let mut tail = segments.len();
        for s in segments {
            let id = &s.segment.name;
            tail -= 1;
            if s.segment.core {
                actions.push(ResearchPolicyAction::Retain);
                total_cost += s.token_cost;
                continue;
            }
            if kept.iter().any(|k| k == id) {
                actions.push(ResearchPolicyAction::Retain);
                total_cost += s.token_cost;
                continue;
            }
            // 表示变化 → 其后的桶失效（tail 近似）。
            switch_cost += tail;
            if self.overlay_fold {
                actions.push(ResearchPolicyAction::Fold);
                total_cost += 1; // marker 近似成本
            } else {
                actions.push(ResearchPolicyAction::Drop);
            }
        }
        ResearchPolicyDecision {
            kept_ids: kept,
            actions,
            total_cost,
            switch_cost,
            rationale: format!(
                "StackPin k={} bucket={} overlay_fold={}",
                self.capacity_k, self.bucket_tokens, self.overlay_fold
            ),
        }
    }
}

/// Proposal C — ShadowLogger：影子决策记录器（生产路径零改动，研究策略只读日志）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchShadowEntry {
    pub turn: u64,
    pub query: String,
    /// 段 id + token 成本（特征精简版）。
    pub segments: Vec<(String, usize)>,
    /// 研究策略决策（动作序列）。
    pub pi_res: Vec<String>,
    /// 生产策略决策（动作序列；本批次恒为现状 rot+fold 的占位描述）。
    pub pi_prod: Vec<String>,
    /// 后验成功信号（评测集判分；真实场景用显式反馈）。
    pub success: Option<bool>,
    /// 实测 prompt token（离线回放用近似值）。
    pub prompt_tokens: usize,
    /// 前缀命中率（cache 护栏指标）。
    pub cache_prefix_ratio: f32,
}

/// 影子日志器：内存积累 + JSONL 导出（Proposal C §3.4 落盘格式）。
#[derive(Debug, Clone, Default)]
pub struct ResearchShadowLogger {
    entries: Vec<ResearchShadowEntry>,
}

impl ResearchShadowLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, entry: ResearchShadowEntry) {
        self.entries.push(entry);
    }

    pub fn entries(&self) -> &[ResearchShadowEntry] {
        &self.entries
    }

    /// 导出 JSONL（每行一条，schema 对齐 `research/logs/README.md`）。
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for e in &self.entries {
            if let Ok(line) = serde_json::to_string(e) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }
}

/// 离线 replay：合成请求序列（局部性模式）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchReplayRequest {
    pub query: String,
    /// 本轮访问的页（段 id）。
    pub touched: Vec<String>,
    /// 后验成功信号（可选）。
    pub success: Option<bool>,
}

/// 确定性合成请求生成：universe 页中 hot 集概率 p_hot，其余均匀。
pub fn research_synthetic_requests(
    seed: u64,
    n: usize,
    universe: usize,
    hot_size: usize,
    p_hot: f64,
) -> Vec<ResearchReplayRequest> {
    // xorshift64* 确定性 PRNG（无外部依赖）。
    let mut state = seed.max(1);
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    };
    let hot = hot_size.min(universe);
    (0..n)
        .map(|t| {
            let u = next();
            let page = if u < p_hot && hot > 0 {
                (next() * hot as f64) as usize
            } else if hot > 0 {
                hot + ((next() * (universe - hot) as f64) as usize)
            } else {
                (next() * universe as f64) as usize
            };
            ResearchReplayRequest {
                query: format!("q{t}"),
                touched: vec![format!("seg-{page}")],
                success: None,
            }
        })
        .collect()
}

/// Belady OPT（等尺寸 paging 离线最优）：淘汰最远未来使用的页。
/// 返回 miss 数；capacity = 可容纳页数（core 钉住不参与 replay 简化口径）。
pub fn research_belady_opt_misses(requests: &[ResearchReplayRequest], capacity: usize) -> usize {
    if capacity == 0 {
        return requests.len();
    }
    let seq: Vec<&str> = requests
        .iter()
        .flat_map(|r| r.touched.iter().map(|s| s.as_str()))
        .collect();
    let mut cache: HashSet<String> = HashSet::new();
    let mut misses = 0usize;
    for (i, page) in seq.iter().enumerate() {
        if cache.contains(*page) {
            continue;
        }
        misses += 1;
        if cache.len() < capacity {
            cache.insert((*page).to_string());
            continue;
        }
        // 淘汰最远未来使用的页；无未来使用则优先淘汰。
        let mut victim: Option<(usize, String)> = None;
        for c in cache.iter() {
            let next_use = seq[i + 1..]
                .iter()
                .position(|p| p == c)
                .map(|d| i + 1 + d)
                .unwrap_or(usize::MAX);
            match &victim {
                None => victim = Some((next_use, c.clone())),
                Some((v, _)) if next_use > *v => victim = Some((next_use, c.clone())),
                _ => {}
            }
        }
        if let Some((_, v)) = victim {
            cache.remove(&v);
        }
        cache.insert((*page).to_string());
    }
    misses
}

/// 纯 paging 版 StackPin（LRU + core 钉住）在线 miss 计数。
/// 返回 miss 数。等尺寸页（每页 1 桶）。
pub fn research_stackpin_paging_misses(
    requests: &[ResearchReplayRequest],
    capacity: usize,
) -> usize {
    let mut cache: VecDeque<String> = VecDeque::new();
    let mut misses = 0usize;
    for r in requests {
        for page in &r.touched {
            if cache.contains(page) {
                cache.retain(|p| p != page);
                cache.push_front(page.clone());
                continue;
            }
            misses += 1;
            if cache.len() >= capacity && capacity > 0 {
                cache.pop_back();
            }
            if capacity > 0 {
                cache.push_front(page.clone());
            }
        }
    }
    misses
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(name: &str, tokens: usize, core: bool) -> ResearchSegment {
        ResearchSegment::new(name, format!("content of {name}"), tokens).with_core(core)
    }

    /// 栈属性：容量增大时保留集单调嵌套（RA-3 §2.3 前提）。
    #[test]
    fn stack_property_nested_retention() {
        let segs = vec![seg("a", 1, false), seg("b", 1, false), seg("c", 1, false)];
        let mut p2 = ResearchStackPinPolicy::new(2, 1, false);
        let mut p3 = ResearchStackPinPolicy::new(3, 1, false);
        for id in ["c", "b", "a"] {
            p2.touch(id);
            p3.touch(id);
        }
        let d2 = p2.decide(0, &segs, "", 100);
        let d3 = p3.decide(0, &segs, "", 100);
        let kept2: HashSet<&str> = d2.kept_ids.iter().map(|s| s.as_str()).collect();
        let kept3: HashSet<&str> = d3.kept_ids.iter().map(|s| s.as_str()).collect();
        assert!(
            kept2.is_subset(&kept3),
            "栈属性: 容量 2 保留集 ⊆ 容量 3 保留集"
        );
    }

    /// core 段永不被 Drop（H3）。
    #[test]
    fn core_pinned_never_dropped() {
        let segs = vec![
            seg("core1", 1, true),
            seg("a", 1, false),
            seg("b", 1, false),
            seg("c", 1, false),
            seg("d", 1, false),
        ];
        let mut p = ResearchStackPinPolicy::new(1, 1, false);
        let d = p.decide(0, &segs, "", 100);
        assert_eq!(d.actions[0], ResearchPolicyAction::Retain);
        assert!(d.kept_ids.contains(&"core1".to_string()));
    }

    /// touch 提升 recency：被 touch 的旧段在容量内幸存（touch 是外部观测事件）。
    #[test]
    fn touch_updates_recency_and_survives() {
        let segs = vec![
            seg("old", 1, false),
            seg("mid", 1, false),
            seg("new", 1, false),
        ];
        let mut p = ResearchStackPinPolicy::new(2, 1, false);
        // 观测顺序: new → mid → old ⇒ 栈 [old, mid, new] → 保留 old, mid
        p.touch("new");
        p.touch("mid");
        p.touch("old");
        let d1 = p.decide(0, &segs, "", 100);
        assert!(d1.kept_ids.contains(&"old".to_string()));
        assert!(d1.kept_ids.contains(&"mid".to_string()));
        assert!(!d1.kept_ids.contains(&"new".to_string()));
        // 再观测 new ⇒ 栈 [new, old, mid] → 保留 new, old; mid 出局
        p.touch("new");
        let d2 = p.decide(1, &segs, "", 100);
        assert!(d2.kept_ids.contains(&"new".to_string()));
        assert!(d2.kept_ids.contains(&"old".to_string()));
        assert!(!d2.kept_ids.contains(&"mid".to_string()));
    }

    /// Fold 叠加层：被换出段标 Fold（无损），非 Drop。
    #[test]
    fn fold_overlay_marks_fold_not_drop() {
        let segs = vec![seg("a", 1, false), seg("b", 1, false), seg("c", 1, false)];
        let mut p = ResearchStackPinPolicy::new(1, 1, true);
        p.touch("a"); // 观测 a → 保留 a, 其余 2 段 Fold
        let d = p.decide(0, &segs, "", 100);
        let folded = d
            .actions
            .iter()
            .filter(|a| **a == ResearchPolicyAction::Fold)
            .count();
        assert_eq!(folded, 2, "容量 1 保留 1, 其余 2 段 Fold");
        assert!(!d.actions.contains(&ResearchPolicyAction::Drop));
    }

    /// 竞争比护栏：合成局部性序列上 LRU(StackPin paging) miss ≤ k × OPT miss。
    #[test]
    fn competitive_ratio_within_k_on_synthetic_sequences() {
        for seed in [1u64, 7, 42, 99] {
            let reqs = research_synthetic_requests(seed, 400, 40, 8, 0.7);
            let k = 8usize;
            let online = research_stackpin_paging_misses(&reqs, k);
            let opt = research_belady_opt_misses(&reqs, k);
            assert!(opt > 0);
            assert!(
                online <= k * opt,
                "seed {seed}: online={online} opt={opt} 违反 k-competitive 上界"
            );
        }
    }

    /// ShadowLogger JSONL schema 对齐。
    #[test]
    fn shadow_logger_jsonl_schema() {
        let mut log = ResearchShadowLogger::new();
        log.record(ResearchShadowEntry {
            turn: 0,
            query: "q0".into(),
            segments: vec![("a".into(), 10)],
            pi_res: vec!["retain".into()],
            pi_prod: vec!["rot-fold".into()],
            success: Some(true),
            prompt_tokens: 120,
            cache_prefix_ratio: 0.85,
        });
        let jsonl = log.to_jsonl();
        let parsed: serde_json::Value =
            serde_json::from_str(jsonl.lines().next().unwrap()).unwrap();
        for f in [
            "turn",
            "query",
            "segments",
            "pi_res",
            "pi_prod",
            "success",
            "prompt_tokens",
            "cache_prefix_ratio",
        ] {
            assert!(parsed.get(f).is_some(), "缺字段 {f}");
        }
    }

    /// 分桶等尺寸化：token 成本按桶上取整（H1）。
    #[test]
    fn bucket_equalization_ceil() {
        let p = ResearchStackPinPolicy::new(3, 10, false);
        assert_eq!(p.buckets_of(1), 1);
        assert_eq!(p.buckets_of(10), 1);
        assert_eq!(p.buckets_of(11), 2);
        assert_eq!(p.buckets_of(20), 2);
        assert_eq!(p.buckets_of(21), 3);
    }

    /// 决策总成本 ≤ 保留预算（Fold 叠加时 marker 近似 1 token/段）。
    #[test]
    fn decision_cost_respects_capacity_ratio() {
        let segs = (0..10)
            .map(|i| seg(&format!("s{i}"), 10, false))
            .collect::<Vec<_>>();
        let mut p = ResearchStackPinPolicy::new(5, 10, true);
        let d = p.decide(0, &segs, "", 1000);
        // 保留 ≤ 5 桶 (50 tokens) + fold marker 5×1。
        assert!(d.total_cost <= 55 + 5, "cost={}", d.total_cost);
        assert_eq!(d.actions.len(), 10);
    }

    /// 默认关闭语义：本模块的决策不改变输入段本身（纯函数式产出）。
    #[test]
    fn decision_is_pure_and_inputs_untouched() {
        let segs = vec![seg("a", 1, false), seg("b", 1, false)];
        let snapshot = segs.clone();
        let mut p = ResearchStackPinPolicy::new(1, 1, false);
        let _ = p.decide(0, &segs, "", 100);
        assert_eq!(segs, snapshot, "输入段集合不被策略修改");
    }
}
