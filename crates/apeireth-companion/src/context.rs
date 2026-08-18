//! `apeireth-companion::context` — 统一注入管线 (ContextAssembler).
//!
//! 背景 (2026-08-16 主人原则: 机制而非补丁, 集成而非分立):
//! 注入链曾是散装 push (状态/记忆/图谱/偏好/今日/成长 各自截断, 无统一预算) —
//! 补丁态。本模块机制化:
//! - 有序块管线: 各块按顺序注册, 统一总预算 (total_budget_chars)
//! - 预算保护: persona/状态块 (核心块) 永不截断; 其余块按注册顺序截断
//! - 截断语义: 超预算块先截长块 (从尾部), 保证核心信息存活
//! - 输出: 一条合并 system 文本 (或逐块列表, 供上层自由组装)
//!
//! 契合点: 与 prompt_cache 的 assemble_tiered (分层组装) 互补 — 本管线管
//! 注入块预算, prompt_cache 管协议层组装.

/// 注入块 (命名 + 内容).
#[derive(Debug, Clone)]
pub struct ContextBlock {
    /// 块名 (调试/预算报告).
    pub name: &'static str,
    /// 块内容 (可能含换行).
    pub content: String,
    /// 是否核心块 (永不截断).
    pub core: bool,
    /// 单块上限 (None = 不限, 受总预算约束).
    pub cap_chars: Option<usize>,
}

impl ContextBlock {
    pub fn new(name: &'static str, content: impl Into<String>) -> Self {
        Self { name, content: content.into(), core: false, cap_chars: None }
    }
    pub fn core(mut self, core: bool) -> Self {
        self.core = core;
        self
    }
    pub fn with_cap(mut self, cap: usize) -> Self {
        self.cap_chars = Some(cap);
        self
    }
}

/// 统一注入管线: 有序块 + 总预算 + 核心保护 + 截断.
pub struct ContextAssembler {
    blocks: Vec<ContextBlock>,
    total_budget_chars: usize,
}

impl ContextAssembler {
    /// 总预算字符数 (默认 6000; 超预算块从尾部截断, 核心块保护).
    pub fn new(total_budget_chars: usize) -> Self {
        Self { blocks: Vec::new(), total_budget_chars: total_budget_chars.max(100) }
    }

    /// 总预算 (只读口) — 补 getter 解除 prompt_assembler 编译阻塞 (agent_orchestrator2 代加, 供主人知悉)
    pub fn total_budget_chars(&self) -> usize {
        self.total_budget_chars
    }

    /// 注册块 (保持顺序; 核心块在前更安全).
    pub fn push(mut self, block: ContextBlock) -> Self {
        self.blocks.push(block);
        self
    }

    /// 预算报告 (诊断): 各块字符数 + 总占用.
    pub fn budget_report(&self) -> Vec<(String, usize)> {
        self.blocks.iter().map(|b| (b.name.to_string(), b.content.chars().count())).collect()
    }

    /// 预算化组装 (不可变): 返回按预算截断后的块列表.
    pub fn assemble_budgeted(&self) -> Vec<String> {
        self.assemble_budgeted_blocks()
            .into_iter()
            .map(|b| b.content)
            .collect()
    }

    /// 预算化组装 (不可变, 保留块名): 上层可依 name 分流 (如 identity 独立成消息).
    pub fn assemble_budgeted_blocks(&self) -> Vec<ContextBlock> {
        // 1. 单块上限
        let mut capped: Vec<String> = self
            .blocks
            .iter()
            .map(|b| {
                let s: String = b.content.chars().take(b.cap_chars.unwrap_or(usize::MAX)).collect();
                s
            })
            .collect();
        // 2. 总预算超标 → 非核心块按"字符多者优先"截断 (贪心砍大头)
        let mut total: usize = capped.iter().map(|s| s.chars().count()).sum();
        if total > self.total_budget_chars {
            let mut order: Vec<usize> = (0..self.blocks.len()).filter(|&i| !self.blocks[i].core).collect();
            order.sort_by_key(|&i| std::cmp::Reverse(capped[i].chars().count()));
            for i in order {
                if total <= self.total_budget_chars {
                    break;
                }
                let len = capped[i].chars().count();
                if len == 0 {
                    continue;
                }
                let over = total - self.total_budget_chars;
                let cut = len.min(over);
                capped[i] = capped[i].chars().take(len - cut).collect();
                total -= cut;
            }
        }
        self.blocks
            .iter()
            .zip(capped)
            .filter(|(_, s)| !s.trim().is_empty())
            .map(|(b, s)| ContextBlock {
                name: b.name,
                content: s,
                core: b.core,
                cap_chars: b.cap_chars,
            })
            .collect()
    }
}

// ============================================================
// TP16 — Context Rot 度量 (M1, P1) — 确定性 0 LLM
//
// 哲学 (per TP16 §1.2 机制而非补丁 + §1.3 确定性优先):
// - rot_score = w1·duplicate_ratio + w2·stale_ratio + w3·(1 - relevance_score)
// - 默认权重 0.4 / 0.3 / 0.3; 标注"启发式, 待 A/B 调权重" (0 装)
// - LLM 仅参与**段编辑** (retain/remove/replace), 见 continuation.rs SegmentEditor
// - 全部纯函数 + 入参化: cfg 必填, 不引入全局状态
//
// 不动承诺:
// - ❌ 不引入 LLM 客户端
// - ❌ 不引入重型 NLP crate (无 ngrams crate, 手写 5-gram + Jaccard)
// - ❌ 不假装 rot_score 准确 — 必须明示启发式 + 待 A/B 调权
// - ❌ 不改 ContextAssembler 既有注入管线 (P0 业务已锁定)
// ============================================================

use std::collections::{BTreeMap, HashSet};

/// Rot 输入块 (轻量, 不耦合 ContextAssembler).
/// 注入层 (Assembler) 与编辑层 (SegmentEditor, continuation.rs) 共用此形状, 互不依赖.
#[derive(Debug, Clone)]
pub struct RotBlock {
    /// 块 id (上层生成, 用于编辑原语定位)
    pub block_id: String,
    /// 内容 (UTF-8 任意, 但 rot 算法按 char/whitespace split, 0 装: 不分中日韩)
    pub content: String,
    /// 最近一次被 touch 的毫秒时间戳 (用于 stale 度量)
    pub last_touched_ms: i64,
}

impl RotBlock {
    /// 便捷构造 (last_touched_ms 默认 0, 调用方再覆盖)
    pub fn new(block_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            block_id: block_id.into(),
            content: content.into(),
            last_touched_ms: 0,
        }
    }

    /// 设时间戳
    pub fn with_touched_ms(mut self, ms: i64) -> Self {
        self.last_touched_ms = ms;
        self
    }
}

/// Rot 启发式权重 (默认 0.4 / 0.3 / 0.3, 三项之和 = 1.0; 0 装: 启发式, 待 A/B 调)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RotWeights {
    /// 重复权重
    pub w_duplicate: f32,
    /// 陈旧权重
    pub w_stale: f32,
    /// 不相关权重 (= 1 - relevance)
    pub w_irrelevant: f32,
}

impl Default for RotWeights {
    fn default() -> Self {
        Self { w_duplicate: 0.4, w_stale: 0.3, w_irrelevant: 0.3 }
    }
}

/// Rot 配置 (deterministic 0 LLM; 全部入参, 方便 A/B)
#[derive(Debug, Clone)]
pub struct RotConfig {
    /// 时间锚 (当前毫秒); 用于 stale 比较. 默认 0, 调用方应覆盖.
    pub now_ms: i64,
    /// 超过此时间窗 = stale. 默认 30 分钟 (会话窗, 主人惯例)
    pub stale_threshold_ms: i64,
    /// n-gram 大小 (词级). 默认 5
    pub ngram_size: usize,
    /// n-gram Jaccard 超过此 = duplicate 计数. 默认 0.6
    pub duplicate_threshold: f32,
    /// 触发阈值 (rot_score 高于此 = 应触发 compaction). 默认 0.6
    pub trigger_threshold: f32,
    /// 权重
    pub weights: RotWeights,
    /// 最新 user 消息 (用于 relevance 比较). `None` = 不计入 relevance (贡献 = 不相关率 0)
    pub latest_user_message: Option<String>,
    /// 被 retain 后视为永远 fresh 的块 id (不计入 stale / dup / relevance)
    pub pinned_block_ids: Vec<String>,
    /// 内容长度 < min_chars_per_block 的块不计入度量 (太短易抖). 默认 16
    pub min_chars_per_block: usize,
}

impl Default for RotConfig {
    fn default() -> Self {
        Self {
            now_ms: 0,
            stale_threshold_ms: 30 * 60 * 1000,
            ngram_size: 5,
            duplicate_threshold: 0.6,
            trigger_threshold: 0.6,
            weights: RotWeights::default(),
            latest_user_message: None,
            pinned_block_ids: Vec::new(),
            min_chars_per_block: 16,
        }
    }
}

/// 重复对 (调试 / A/B 用, 不假装评分精确)
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicatePair {
    /// 块 a 的 id
    pub a: String,
    /// 块 b 的 id
    pub b: String,
    /// Jaccard 相似度 ∈ [0, 1]
    pub jaccard: f32,
}

/// Rot 评分明细 (全可观察, 0 假装: 明示每条触发证据)
#[derive(Debug, Clone)]
pub struct RotBreakdown {
    /// 总 rot_score ∈ [0, 1]; = w_dup * dup_ratio + w_stale * stale_ratio + w_irrel * irrelevance
    pub total: f32,
    /// 重复率 = `involved_block_count / eligible_block_count` (去重后上限 cap 1)
    pub duplicate_ratio: f32,
    /// 陈旧率 = stale 块数 / eligible 块数
    pub stale_ratio: f32,
    /// 不相关率 = 1 - mean(relevance); 无 user message 时 = 0
    pub irrelevance: f32,
    /// 平均相关性 (relevance_score ∈ [0, 1]), 无 user message 时 = 1.0
    pub relevance: f32,
    /// 触发证据 (调试 / A/B)
    pub duplicate_pairs: Vec<DuplicatePair>,
    pub stale_block_ids: Vec<String>,
    pub low_relevance_block_ids: Vec<String>,
    /// 计入度量的块数 (排除 pinned + 太短 + 空内容)
    pub eligible_block_count: usize,
}

/// 取一段文本的 n-gram 集合 (按词级 whitespace split, lowercase).
/// `0 装`: 不分中日韩, 不做 stemming — 启发式够用, 真语义留给 LLM 段编辑.
///
/// 返回的 n-gram 是 owned `Vec<String>` 以满足 `Hash` (避免借用生命周期).
pub fn ngrams(s: &str, n: usize) -> HashSet<Vec<String>> {
    if n == 0 {
        return HashSet::new();
    }
    let words: Vec<String> = s
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .collect();
    if words.len() < n {
        return HashSet::new();
    }
    let mut out = HashSet::new();
    for w in words.windows(n) {
        out.insert(w.to_vec());
    }
    out
}

/// Jaccard 相似度. 全等 = 1.0, 全不等 = 0.0, 双空集 = 0.0.
pub fn jaccard<T: Eq + std::hash::Hash>(a: &HashSet<T>, b: &HashSet<T>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// 关键词重叠 (lowercase, alphanumeric 词; share / union ∈ [0, 1]).
/// 与 n-gram 不同: 它只看 bag-of-words, 不看顺序, 用于 relevance.
pub fn keyword_overlap(a: &str, b: &str) -> f32 {
    let words = |s: &str| -> HashSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .map(|w| w.to_lowercase())
            .collect()
    };
    let sa = words(a);
    let sb = words(b);
    if sa.is_empty() && sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count() as f32;
    let union = sa.union(&sb).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// 主入口: 给定 blocks + config, 算 RotBreakdown (deterministic pure function).
///
/// 算法 (公开公式, 0 装):
/// ```text
///   rot_score = w_dup * duplicate_ratio + w_stale * stale_ratio + w_irrel * (1 - relevance_mean)
///   duplicate_ratio = involved_block_count / eligible_block_count    (cap 1.0)
///   stale_ratio     = stale_block_count / eligible_block_count
///   relevance       = mean(keyword_overlap(block, latest_user_message))   ; 无 message → 1.0
/// ```
pub fn compute_rot_score(blocks: &[RotBlock], cfg: &RotConfig) -> RotBreakdown {
    let pinned: HashSet<&str> = cfg.pinned_block_ids.iter().map(|s| s.as_str()).collect();
    let min_chars = cfg.min_chars_per_block.max(1);

    // 过滤: pinned / 太短 / 空内容 不计入度量
    let eligible: Vec<&RotBlock> = blocks
        .iter()
        .filter(|b| {
            !pinned.contains(b.block_id.as_str())
                && b.content.chars().count() >= min_chars
                && !b.content.trim().is_empty()
        })
        .collect();

    let eligible_count = eligible.len();
    if eligible_count == 0 {
        return RotBreakdown {
            total: 0.0,
            duplicate_ratio: 0.0,
            stale_ratio: 0.0,
            irrelevance: 0.0,
            relevance: if cfg.latest_user_message.is_none() { 1.0 } else { 0.0 },
            duplicate_pairs: Vec::new(),
            stale_block_ids: Vec::new(),
            low_relevance_block_ids: Vec::new(),
            eligible_block_count: 0,
        };
    }

    // 1. Stale
    let stale_block_ids: Vec<String> = eligible
        .iter()
        .filter(|b| cfg.now_ms.saturating_sub(b.last_touched_ms) > cfg.stale_threshold_ms)
        .map(|b| b.block_id.clone())
        .collect();
    let stale_ratio = stale_block_ids.len() as f32 / eligible_count as f32;

    // 2. Duplicate (n-gram Jaccard pairwise, 阈值之上 = 一对)
    let mut duplicate_pairs: Vec<DuplicatePair> = Vec::new();
    let ngram_cache: BTreeMap<&str, HashSet<Vec<String>>> = eligible
        .iter()
        .map(|b| (b.block_id.as_str(), ngrams(&b.content, cfg.ngram_size)))
        .collect();
    for (i, ai) in eligible.iter().enumerate() {
        for bj in eligible.iter().skip(i + 1) {
            let grams_a = &ngram_cache[ai.block_id.as_str()];
            let grams_b = &ngram_cache[bj.block_id.as_str()];
            if grams_a.is_empty() || grams_b.is_empty() {
                continue;
            }
            let j = jaccard(grams_a, grams_b);
            if j >= cfg.duplicate_threshold {
                duplicate_pairs.push(DuplicatePair {
                    a: ai.block_id.clone(),
                    b: bj.block_id.clone(),
                    jaccard: j,
                });
            }
        }
    }
    // duplicate_ratio: 一对涉及两个块, 重复块计数去重 (同一块多次重复按 1 个计); 上限 cap 到 1
    let mut involved: HashSet<&str> = HashSet::new();
    for p in &duplicate_pairs {
        involved.insert(p.a.as_str());
        involved.insert(p.b.as_str());
    }
    let duplicate_ratio = (involved.len() as f32 / eligible_count as f32).min(1.0);

    // 3. Relevance (与 latest_user_message 比, 0 装: 无 message = 完美相关 1.0)
    let (relevance_mean, low_relevance_block_ids) = match &cfg.latest_user_message {
        None => (1.0_f32, Vec::new()),
        Some(msg) => {
            let mut rels: Vec<(String, f32)> = eligible
                .iter()
                .map(|b| (b.block_id.clone(), keyword_overlap(&b.content, msg)))
                .collect();
            rels.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap_or(std::cmp::Ordering::Equal));
            let mean = rels.iter().map(|(_, r)| *r).sum::<f32>() / rels.len() as f32;
            // 低相关块: relevance < 0.1 (启发式阈值, 单独标 0 装)
            let low: Vec<String> = rels
                .iter()
                .filter(|(_, r)| *r < 0.1)
                .map(|(id, _)| id.clone())
                .collect();
            (mean, low)
        }
    };
    let irrelevance = 1.0 - relevance_mean;

    // 4. 总评分 (三权重之和约定 = 1.0, 但不硬假设, 容许离线调权)
    let total = cfg.weights.w_duplicate * duplicate_ratio
        + cfg.weights.w_stale * stale_ratio
        + cfg.weights.w_irrelevant * irrelevance;

    RotBreakdown {
        total,
        duplicate_ratio,
        stale_ratio,
        irrelevance,
        relevance: relevance_mean,
        duplicate_pairs,
        stale_block_ids,
        low_relevance_block_ids,
        eligible_block_count: eligible_count,
    }
}

/// 是否触发 compaction (rot_score 严格大于 trigger_threshold 触发).
pub fn should_compact(b: &RotBreakdown, cfg: &RotConfig) -> bool {
    b.total > cfg.trigger_threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_blocks_never_truncated() {
        // 总 430 字, 预算 300: 核心 150 保留, 非核心按大头先砍 (mem 200 → 留 70)
        let a = ContextAssembler::new(300)
            .push(ContextBlock::new("persona", "核心人格内容".repeat(30)).core(true))
            .push(ContextBlock::new("mem", "记忆内容".repeat(50)))
            .push(ContextBlock::new("prefs", "偏好内容".repeat(20)));
        let out = a.assemble_budgeted();
        let total: usize = out.iter().map(|s| s.chars().count()).sum();
        assert!(total <= 300, "总预算应约束 (核心保护下 total=300)");
        assert!(out[0].contains("核心人格内容"), "核心块应完整保留");
        // 记忆块 (非核心, 最大) 应被截断: 460-300=160 → mem 200 砍 160 → 40
        assert!(out[1].chars().count() < "记忆内容".repeat(50).chars().count(), "非核心块应被截断");
        assert_eq!(out[1].chars().count(), 40, "mem 200 → 砍 160 → 留 40");
    }

    #[test]
    fn per_block_cap() {
        let a = ContextAssembler::new(100_000)
            .push(ContextBlock::new("x", "abc".repeat(10)).with_cap(12));
        let out = a.assemble_budgeted();
        assert_eq!(out[0], "abcabcabcabc".to_string());
    }

    #[test]
    fn empty_blocks_filtered() {
        let a = ContextAssembler::new(1000)
            .push(ContextBlock::new("a", "hello"))
            .push(ContextBlock::new("b", "   "));
        let out = a.assemble_budgeted();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "hello");
    }

    // ========================================================
    // TP16 — rot_score 模块测试 (确定性纯函数, 0 LLM)
    // ========================================================

    fn mk_block(id: &str, content: &str, touched_ms: i64) -> RotBlock {
        RotBlock::new(id, content).with_touched_ms(touched_ms)
    }

    fn base_cfg(now_ms: i64) -> RotConfig {
        let mut c = RotConfig::default();
        c.now_ms = now_ms;
        c
    }

    #[test]
    fn ngrams_basic_tokenization() {
        let s = "the quick brown fox jumps over the lazy dog";
        let ng = ngrams(s, 5);
        assert!(!ng.is_empty());
        assert!(ng.contains(&vec!["the".into(), "quick".into(), "brown".into(), "fox".into(), "jumps".into()]));
    }

    #[test]
    fn ngrams_short_input_yields_empty() {
        let s = "abc def";
        assert!(ngrams(s, 5).is_empty(), "< 5 个词应返空集");
    }

    #[test]
    fn ngrams_size_zero_yields_empty() {
        assert!(ngrams("anything goes here", 0).is_empty());
    }

    #[test]
    fn jaccard_identical_is_one() {
        let a: HashSet<i32> = [1, 2, 3].into_iter().collect();
        let b = a.clone();
        assert!((jaccard(&a, &b) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn jaccard_disjoint_is_zero() {
        let a: HashSet<i32> = [1, 2].into_iter().collect();
        let b: HashSet<i32> = [3, 4].into_iter().collect();
        assert_eq!(jaccard(&a, &b), 0.0);
    }

    #[test]
    fn keyword_overlap_full_share() {
        let s = "rust lang systems memory rotation";
        assert!((keyword_overlap(s, s) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn keyword_overlap_no_share_is_zero() {
        let a = "rust lang systems memory rotation";
        let b = "completely different words like coffee breakfast toast";
        assert_eq!(keyword_overlap(a, b), 0.0);
    }

    #[test]
    fn rot_empty_blocks_zero_total() {
        let b = compute_rot_score(&[], &base_cfg(0));
        assert_eq!(b.total, 0.0);
        assert_eq!(b.eligible_block_count, 0);
    }

    #[test]
    fn rot_single_block_zero_rot() {
        let blocks = vec![mk_block("a", "this is a sufficiently long block of text content here", 0)];
        let b = compute_rot_score(&blocks, &base_cfg(0));
        assert_eq!(b.duplicate_ratio, 0.0);
        assert_eq!(b.stale_ratio, 0.0);
        // no user message → irrelevance=0
        assert_eq!(b.irrelevance, 0.0);
        assert_eq!(b.total, 0.0);
        assert!(!should_compact(&b, &base_cfg(0)));
    }

    #[test]
    fn rot_two_identical_blocks_high_duplicate() {
        let text = "rust language supports zero cost abstractions and ownership model for safety";
        let now = 10_000_000;
        let blocks = vec![
            mk_block("a", text, now),
            mk_block("b", text, now),
        ];
        let b = compute_rot_score(&blocks, &base_cfg(now));
        // 2 个块都涉及 → dup_ratio = 2/2 = 1.0
        assert_eq!(b.duplicate_ratio, 1.0, "两个完全相同块应记 100% 重复");
        assert_eq!(b.stale_ratio, 0.0, "now == touched → 不陈旧");
        assert_eq!(b.irrelevance, 0.0, "无 user msg → 0 不相关");
        // default weights 0.4,0.3,0.3: 0.4*1.0 + 0 = 0.4
        assert!((b.total - 0.4).abs() < 1e-5);
        assert!(!should_compact(&b, &base_cfg(now)), "0.4 < 0.6 不应触发");
    }

    #[test]
    fn rot_three_identical_blocks_dup_cap_one() {
        // 3 个全等 → involved=3, eligible=3, ratio=1.0 (cap),total=0.4
        let text = "the same content repeated three times in this block to test jaccard capping";
        let blocks = vec![
            mk_block("a", text, 1000),
            mk_block("b", text, 1000),
            mk_block("c", text, 1000),
        ];
        let b = compute_rot_score(&blocks, &base_cfg(1000));
        assert_eq!(b.duplicate_ratio, 1.0);
        assert!(b.duplicate_pairs.len() == 3, "3 块两两对 = C(3,2)=3 重复对, got {}", b.duplicate_pairs.len());
    }

    #[test]
    fn rot_stale_blocks_detected() {
        let now = 10_000_000; // > 30 min after stale blocks
        let blocks = vec![
            mk_block("fresh", "this is a fresh block of text content with enough chars", now),
            // both touched = now - 1h ≈ 3.6M ms ago > 30 min threshold (1.8M)
            mk_block("stale1", "this is a stale block of text content with old timestamp", now - 3_600_000),
            mk_block("stale2", "another stale block of text content with old timestamp too", now - 7_200_000),
        ];
        let b = compute_rot_score(&blocks, &base_cfg(now));
        // 3 eligible, 2 stale → stale_ratio ≈ 0.667
        assert_eq!(b.stale_block_ids.len(), 2);
        assert!((b.stale_ratio - 2.0 / 3.0).abs() < 1e-5);
        assert!(b.stale_block_ids.contains(&"stale1".to_string()));
        assert!(b.stale_block_ids.contains(&"stale2".to_string()));
    }

    #[test]
    fn rot_relevance_with_user_message() {
        let msg = "rust async runtime and tokio scheduler";
        let blocks = vec![
            mk_block("relevant", "discussing rust async runtime with tokio and scheduler design", 1000),
            mk_block("irrelevant", "completely off topic block about breakfast cereal and coffee types", 1000),
        ];
        let mut cfg = base_cfg(1000);
        cfg.latest_user_message = Some(msg.into());
        let b = compute_rot_score(&blocks, &cfg);
        assert!(b.relevance > 0.0, "至少有一条相关, relevance_mean 应 > 0");
        assert!(b.relevance < 1.0, "另一条无关, 应 < 1");
        assert!(b.irrelevance > 0.0);
        assert_eq!(b.low_relevance_block_ids.len(), 1, "无关那条应标 low");
        assert!(b.low_relevance_block_ids.contains(&"irrelevant".to_string()));
    }

    #[test]
    fn rot_pinned_blocks_excluded() {
        // 同一内容, 但 pinned → 应被排除, dup=0
        let text = "this is a long block of text to enable ngram computation reliably";
        let blocks = vec![
            mk_block("a", text, 1000),
            mk_block("b", text, 1000),
        ];
        let mut cfg = base_cfg(1000);
        cfg.pinned_block_ids = vec!["a".into()];
        let b = compute_rot_score(&blocks, &cfg);
        assert_eq!(b.eligible_block_count, 1, "pinned 块不计入 eligible");
        assert_eq!(b.duplicate_ratio, 0.0, "只剩 1 块, 无 pairwise 对 → dup_ratio=0");
    }

    #[test]
    fn rot_short_blocks_excluded_by_min_chars() {
        // 太短的块不计入 (易抖)
        let blocks = vec![
            mk_block("a", "this is a long enough block to be counted as eligible", 1000),
            mk_block("b", "tiny", 1000), // < 16 chars (默认)
        ];
        let b = compute_rot_score(&blocks, &base_cfg(1000));
        assert_eq!(b.eligible_block_count, 1);
    }

    #[test]
    fn rot_total_in_unit_interval() {
        let text = "rust async zero cost abstraction block of text content for tests";
        let blocks = vec![
            mk_block("a", text, 0),
            mk_block("b", text, 0),
            mk_block("c", "totally unrelated content like banana coffee breakfast toast", 0),
        ];
        let mut cfg = base_cfg(60 * 60 * 1000); // 60 min later
        cfg.latest_user_message = Some("rust tokio scheduler".into());
        let b = compute_rot_score(&blocks, &cfg);
        assert!((0.0..=1.0).contains(&b.total), "rot_score 必须 ∈ [0,1], got {}", b.total);
    }

    #[test]
    fn rot_trigger_policy_high_rot_triggers() {
        // 构造高 rot: 全陈旧 + 全重复 + 全无关
        let now = 10_000_000;
        let text = "rust lang memory rotation tokio async scheduler stable tests passing";
        let blocks = vec![
            mk_block("a", text, 0), // > 30 min stale
            mk_block("b", text, 0),
            mk_block("c", text, 0),
        ];
        let mut cfg = base_cfg(now);
        cfg.latest_user_message = Some("cooking recipes for pasta carbonara with eggs".into());
        let b = compute_rot_score(&blocks, &cfg);
        // stale: 3/3=1.0; dup: 3/3=1.0; irrel: 1 - 0 = 1.0
        // total = 0.4*1 + 0.3*1 + 0.3*1 = 1.0
        assert!((b.total - 1.0).abs() < 1e-4, "高 rot total 应 ≈ 1.0, got {}", b.total);
        assert!(should_compact(&b, &cfg), "rot 1.0 > 0.6 应触发 compaction");
    }

    #[test]
    fn rot_trigger_policy_low_rot_does_not_trigger() {
        let now = 1_000;
        let blocks = vec![mk_block(
            "fresh",
            "this fresh block discusses exactly what the user just asked about in detail",
            now,
        )];
        let mut cfg = base_cfg(now);
        cfg.latest_user_message = Some("exactly what the user just asked about".into());
        let b = compute_rot_score(&blocks, &cfg);
        assert!(b.total < 0.6, "fresh + 相关 → 低 rot, got {}", b.total);
        assert!(!should_compact(&b, &cfg));
    }

    #[test]
    fn rot_at_threshold_is_not_compact() {
        // 边界: total == threshold 应不触发 (> 严格)
        let cfg = base_cfg(0);
        let b = RotBreakdown {
            total: 0.6,
            duplicate_ratio: 0.0,
            stale_ratio: 0.0,
            irrelevance: 0.0,
            relevance: 1.0,
            duplicate_pairs: Vec::new(),
            stale_block_ids: Vec::new(),
            low_relevance_block_ids: Vec::new(),
            eligible_block_count: 1,
        };
        assert!(!should_compact(&b, &cfg), "== 阈值不算触发, 严格 >");
    }

    #[test]
    fn rot_above_threshold_triggers() {
        let cfg = base_cfg(0);
        let b = RotBreakdown {
            total: 0.6001,
            duplicate_ratio: 0.0,
            stale_ratio: 0.0,
            irrelevance: 0.0,
            relevance: 1.0,
            duplicate_pairs: Vec::new(),
            stale_block_ids: Vec::new(),
            low_relevance_block_ids: Vec::new(),
            eligible_block_count: 1,
        };
        assert!(should_compact(&b, &cfg));
    }

    #[test]
    fn rot_stale_block_count_grows_with_n_old_blocks() {
        // 多放几个老块 → stale_block_count 单调增; ratio 也保持 1.0 (因为全老)
        let now = 10_000_000;
        let mk_n = |n: usize| -> Vec<RotBlock> {
            (0..n)
                .map(|i| {
                    mk_block(
                        &format!("b{i}"),
                        "this is a long content block for testing stale block count monotonicity",
                        0, // 全老 (> 30 min)
                    )
                })
                .collect()
        };
        let cfg = base_cfg(now);
        let b1 = compute_rot_score(&mk_n(2), &cfg);
        let b4 = compute_rot_score(&mk_n(4), &cfg);
        let b8 = compute_rot_score(&mk_n(8), &cfg);
        assert_eq!(b1.stale_block_ids.len(), 2);
        assert_eq!(b4.stale_block_ids.len(), 4);
        assert_eq!(b8.stale_block_ids.len(), 8);
        // 全老 → ratio 恒为 1.0 (sanity)
        assert_eq!(b1.stale_ratio, 1.0);
        assert_eq!(b8.stale_ratio, 1.0);
    }

    #[test]
    fn rot_total_monotonic_when_more_stale_blocks_added() {
        // 混合: x 个 fresh + y 个 old, total 应随 y 增大而增大
        let now = 10_000_000;
        let fresh_text = "this is a fresh block discussing rust programming topics in detail";
        let old_text = "this is a stale block of older content from earlier conversation history";
        let cfg = base_cfg(now);
        // 1 fresh + 1 old
        let mix1 = vec![
            mk_block("f1", fresh_text, now),
            mk_block("o1", old_text, 0),
        ];
        // 1 fresh + 4 old
        let mix2 = vec![
            mk_block("f1", fresh_text, now),
            mk_block("o1", old_text, 0),
            mk_block("o2", old_text, 0),
            mk_block("o3", old_text, 0),
            mk_block("o4", old_text, 0),
        ];
        let r1 = compute_rot_score(&mix1, &cfg);
        let r2 = compute_rot_score(&mix2, &cfg);
        assert!(
            r2.total > r1.total,
            "更多陈旧块 → rot 应更高: r1={} r2={}",
            r1.total,
            r2.total
        );
        assert_eq!(r1.stale_block_ids.len(), 1);
        assert_eq!(r2.stale_block_ids.len(), 4);
    }

    #[test]
    fn rot_duplicate_pair_jaccard_recorded() {
        let text = "the rust standard library has a wonderful api for vec slice and string types";
        let blocks = vec![
            mk_block("a", text, 1000),
            mk_block("b", text, 1000),
        ];
        let b = compute_rot_score(&blocks, &base_cfg(1000));
        assert_eq!(b.duplicate_pairs.len(), 1);
        assert_eq!(b.duplicate_pairs[0].a, "a");
        assert_eq!(b.duplicate_pairs[0].b, "b");
        assert!((b.duplicate_pairs[0].jaccard - 1.0).abs() < 1e-5);
    }

    #[test]
    fn rot_duplicate_below_threshold_ignored() {
        // 完全不同内容 → jaccard 远低于 0.6 → 不应记为重复对
        let blocks = vec![
            mk_block("a", "completely different first topic about rust systems programming", 1000),
            mk_block("b", "totally unrelated second topic about coffee breakfast and toast", 1000),
        ];
        let b = compute_rot_score(&blocks, &base_cfg(1000));
        assert!(b.duplicate_pairs.is_empty(), "无关两块不应记为重复, got {:?}", b.duplicate_pairs);
    }

    #[test]
    fn rot_default_weights_sum_to_one() {
        let w = RotWeights::default();
        let s = w.w_duplicate + w.w_stale + w.w_irrelevant;
        assert!((s - 1.0).abs() < 1e-5, "默认权重之和应为 1.0, got {s}");
    }

    #[test]
    fn rot_custom_weights_off_default() {
        // 验证非默认权重确实影响 total (而不是只硬编码 default 路径)
        let text = "this is a long content block for testing custom weights effect on rot";
        let blocks = vec![mk_block("a", text, 0), mk_block("b", text, 0)];
        let mut cfg = base_cfg(0);
        cfg.weights = RotWeights { w_duplicate: 1.0, w_stale: 0.0, w_irrelevant: 0.0 };
        let b = compute_rot_score(&blocks, &cfg);
        // dup=1, stale=0, irrel=0 → total = 1.0
        assert!((b.total - 1.0).abs() < 1e-5);
    }
}
