//! `apeireth-companion::context_rot` — M1: Context Rot 度量 + compaction 段编辑原语.
//!
//! 背景 (台账 M1, 记忆调研 docs/memory-research.md §二.6 吸收):
//! Anthropic context editing/compaction + Chroma context rot 精神 —
//! context.rs 只有尾部截断: 上下文腐烂无度量, 压缩无智能. 本模块机制化:
//! ① rot_score: 重复度/陈旧度/相关性启发式三因子, 确定性公式, 0 LLM 依赖
//! ② compaction 原语: retain/remove/replace 段编辑; LLM 参与版留 [`Compactor`] trait 口
//!    (0 装: 无内置 LLM 实现), 确定性规则版 [`DeterministicCompactor`] 先行
//! ③ 与 context.rs 尾部截断协作: [`compact_then_budget`] rot 驱动选择性压缩优先,
//!    再交 ContextAssembler 预算截尾兜底 — 保留高价值段, 不盲砍
//!
//! 边界 (任务 ab3f5ef7): 自包含新模块, 不动 context.rs 本体与 assemble.rs 注入链.

use crate::context::{ContextAssembler, ContextBlock};

/// 待打分/编辑的上下文段快照.
#[derive(Debug, Clone)]
pub struct Segment {
    /// 段名 (对齐 ContextBlock::name, 供上层分流).
    pub name: &'static str,
    /// 段内容.
    pub content: String,
    /// 核心段 (persona/状态等): compaction 跳过, 对齐 context.rs 核心保护.
    pub core: bool,
    /// 陈旧度输入: 距今轮数 (0 = 最新; 无轮次信息的段传 0).
    pub age_turns: usize,
}

impl Segment {
    pub fn new(name: &'static str, content: impl Into<String>, age_turns: usize) -> Self {
        Self { name, content: content.into(), core: false, age_turns }
    }
    pub fn core(mut self, core: bool) -> Self {
        self.core = core;
        self
    }
}

/// rot_score 权重/衰减配置. 全确定性, 0 LLM.
#[derive(Debug, Clone)]
pub struct RotConfig {
    /// 重复度权重 (默认 0.4 — 三因子中"腐烂"最强信号).
    pub w_repetition: f32,
    /// 陈旧度权重 (默认 0.3).
    pub w_staleness: f32,
    /// 相关性权重 (默认 0.3; 无 query 时权重归一化到前两项).
    pub w_relevance: f32,
    /// 陈旧度半衰期 (轮): staleness = age / (age + half_life), 单调有界 [0,1).
    pub stale_half_life_turns: f32,
}

impl Default for RotConfig {
    fn default() -> Self {
        Self { w_repetition: 0.4, w_staleness: 0.3, w_relevance: 0.3, stale_half_life_turns: 20.0 }
    }
}

/// 三因子分解 (诊断/可审计) + 总分.
#[derive(Debug, Clone, PartialEq)]
pub struct RotBreakdown {
    /// 重复度 [0,1]: 1 = 全是重复.
    pub repetition: f32,
    /// 陈旧度 [0,1): age/(age+half_life).
    pub staleness: f32,
    /// 无关度 [0,1]: 1 = query 词元全未命中; 无 query 时恒 0 且权重归一化.
    pub irrelevance: f32,
    /// 加权总分 [0,1], 越高越"腐烂" (优先压缩候选).
    pub score: f32,
}

/// 重复度因子: 多行→行级去重比; 单行→6 字滑窗去重比. 确定性.
fn repetition_factor(content: &str) -> f32 {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() >= 2 {
        let total = lines.len();
        let mut uniq: Vec<&str> = lines.clone();
        uniq.sort_unstable();
        uniq.dedup();
        return 1.0 - (uniq.len() as f32) / (total as f32);
    }
    // 单行: char 6-gram 滑窗
    let chars: Vec<char> = content.chars().collect();
    if chars.len() < 6 {
        return 0.0;
    }
    let windows: usize = chars.len() - 5;
    let mut seen = std::collections::HashSet::new();
    for i in 0..windows {
        seen.insert(&chars[i..i + 6]);
    }
    1.0 - (seen.len() as f32) / (windows as f32)
}

/// 相关性词元: ASCII 小写词 + CJK char-bigram (确定性, 无分词器依赖).
fn query_tokens(query: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut ascii_word = String::new();
    let mut cjk_prev: Option<char> = None;
    for c in query.chars() {
        if c.is_ascii_alphanumeric() {
            ascii_word.push(c.to_ascii_lowercase());
            cjk_prev = None;
        } else if c.is_alphabetic() && (c as u32) > 0x2E80 {
            // CJK: 先冲刷 ascii 词
            if !ascii_word.is_empty() {
                out.push(std::mem::take(&mut ascii_word));
            }
            if let Some(p) = cjk_prev {
                out.push(format!("{p}{c}"));
            } else {
                out.push(c.to_string()); // 孤立单字 (query 仅 1 个 CJK 字时保底)
            }
            cjk_prev = Some(c);
        } else {
            if !ascii_word.is_empty() {
                out.push(std::mem::take(&mut ascii_word));
            }
            cjk_prev = None;
        }
    }
    if !ascii_word.is_empty() {
        out.push(ascii_word);
    }
    out.sort();
    out.dedup();
    out
}

/// rot_score 三因子分解 (确定性公式, 0 LLM). query=None 时相关性权重归一化.
pub fn rot_breakdown(seg: &Segment, query: Option<&str>, cfg: &RotConfig) -> RotBreakdown {
    let repetition = repetition_factor(&seg.content).clamp(0.0, 1.0);
    let age = seg.age_turns as f32;
    let hl = cfg.stale_half_life_turns.max(1.0);
    let staleness = (age / (age + hl)).clamp(0.0, 1.0);
    let (irrelevance, w_rel_used) = match query {
        Some(q) if !q.trim().is_empty() => {
            let toks = query_tokens(q);
            if toks.is_empty() {
                (0.0, 0.0)
            } else {
                let hit = toks
                    .iter()
                    .filter(|t| seg.content.to_lowercase().contains(t.as_str()))
                    .count();
                (1.0 - (hit as f32) / (toks.len() as f32), cfg.w_relevance)
            }
        }
        _ => (0.0, 0.0),
    };
    let w_sum = cfg.w_repetition + cfg.w_staleness + w_rel_used;
    let score = if w_sum <= 0.0 {
        0.0
    } else {
        ((cfg.w_repetition * repetition + cfg.w_staleness * staleness + w_rel_used * irrelevance)
            / w_sum)
            .clamp(0.0, 1.0)
    };
    RotBreakdown { repetition, staleness, irrelevance, score }
}

/// rot_score 快捷口 (总分 [0,1], 越高越应优先压缩).
pub fn rot_score(seg: &Segment, query: Option<&str>, cfg: &RotConfig) -> f32 {
    rot_breakdown(seg, query, cfg).score
}

/// compaction 段编辑原语 (Anthropic context editing 精神).
#[derive(Debug, Clone, PartialEq)]
pub enum CompactionOp {
    /// 保留原段.
    Retain,
    /// 移除整段.
    Remove,
    /// 以摘要文本替换整段.
    Replace(String),
}

/// compaction 决策口 — LLM 参与版留口 (0 装: 无内置 LLM 实现).
///
/// 实现者可调 LLM 做语义级 retain/remove/replace; 确定性规则版见
/// [`DeterministicCompactor`]. 返回 ops 与 segments 一一对应.
pub trait Compactor {
    fn decide(&self, segments: &[Segment], query: Option<&str>) -> Vec<CompactionOp>;
}

/// 确定性规则版 compactor (先行): rot_score 超阈值 → 抽取式摘要替换, 无可摘要内容 → 移除.
#[derive(Debug, Clone)]
pub struct DeterministicCompactor {
    /// rot_score ≥ threshold 的非核心段触发压缩 (默认 0.6).
    pub threshold: f32,
    /// 抽取式摘要字符上限 (默认 120).
    pub summary_chars: usize,
    /// 打分配置.
    pub rot: RotConfig,
}

impl Default for DeterministicCompactor {
    fn default() -> Self {
        Self { threshold: 0.6, summary_chars: 120, rot: RotConfig::default() }
    }
}

/// 抽取式摘要 (确定性, 0 LLM): 按行去重保序, 截到上限字符.
fn extractive_summary(content: &str, max_chars: usize) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = String::new();
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || !seen.insert(t) {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(t);
        if out.chars().count() >= max_chars {
            break;
        }
    }
    out.chars().take(max_chars).collect()
}

impl Compactor for DeterministicCompactor {
    fn decide(&self, segments: &[Segment], query: Option<&str>) -> Vec<CompactionOp> {
        segments
            .iter()
            .map(|s| {
                if s.core {
                    return CompactionOp::Retain; // 核心段保护, 对齐 context.rs
                }
                if rot_score(s, query, &self.rot) < self.threshold {
                    return CompactionOp::Retain;
                }
                let summary = extractive_summary(&s.content, self.summary_chars);
                if summary.is_empty() {
                    CompactionOp::Remove
                } else {
                    CompactionOp::Replace(summary)
                }
            })
            .collect()
    }
}

/// 应用段编辑原语: Retain 保留 / Remove 丢弃 / Replace 换内容 (名字/核心/年龄属性保留).
pub fn apply_ops(segments: &[Segment], ops: &[CompactionOp]) -> Vec<Segment> {
    segments
        .iter()
        .zip(ops.iter())
        .filter_map(|(s, op)| match op {
            CompactionOp::Retain => Some(s.clone()),
            CompactionOp::Remove => None,
            CompactionOp::Replace(text) => Some(Segment { content: text.clone(), ..s.clone() }),
        })
        .collect()
}

/// 与 context.rs 尾部截断协作: rot 驱动选择性压缩优先, 再交 ContextAssembler 预算截尾兜底.
///
/// 语义: 盲截尾 (ContextAssembler 直跑) 不辨价值; 本函数先按 rot 分压缩腐烂段
/// (保留高价值段), 剩余仍超预算时才由既有截尾机制兜底. 返回预算化后的块列表.
pub fn compact_then_budget<C: Compactor>(
    segments: &[Segment],
    compactor: &C,
    query: Option<&str>,
    total_budget_chars: usize,
) -> Vec<ContextBlock> {
    let ops = compactor.decide(segments, query);
    let edited = apply_ops(segments, &ops);
    let mut asm = ContextAssembler::new(total_budget_chars);
    for s in edited {
        asm = asm.push(ContextBlock { name: s.name, content: s.content, core: s.core, cap_chars: None });
    }
    asm.assemble_budgeted_blocks()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> RotConfig {
        RotConfig::default()
    }

    // ── rot_score 三因子 ──────────────────────────────────────────────

    #[test]
    fn rot_score_repetition_factor() {
        let rep = Segment::new("mem", "同一件事说了七遍\n".repeat(7), 0);
        let uniq = Segment::new("mem", "甲乙丙丁戊己庚辛壬癸子丑寅卯辰巳午未申酉戌亥", 0);
        let b_rep = rot_breakdown(&rep, None, &cfg());
        let b_uniq = rot_breakdown(&uniq, None, &cfg());
        assert!(b_rep.repetition > 0.8, "高重复行级因子应近 1, got {}", b_rep.repetition);
        assert!(b_uniq.repetition < 0.2, "唯一内容因子应低, got {}", b_uniq.repetition);
        assert!(b_rep.score > b_uniq.score, "重复段总分应更高");
    }

    #[test]
    fn rot_score_staleness_factor() {
        let fresh = Segment::new("mem", "独有内容新鲜事", 0);
        let stale = Segment::new("mem", "独有内容新鲜事", 60);
        let b_f = rot_breakdown(&fresh, None, &cfg());
        let b_s = rot_breakdown(&stale, None, &cfg());
        assert_eq!(b_f.staleness, 0.0, "age=0 陈旧度为 0");
        assert!(b_s.staleness > b_f.staleness, "更旧 → 陈旧度更高");
        assert!(b_s.staleness < 1.0, "age/(age+hl) 有界 < 1");
        assert!(b_s.score > b_f.score, "陈旧段总分应更高");
    }

    #[test]
    fn rot_score_relevance_factor() {
        let rel = Segment::new("mem", "用户喜欢喝乌龙茶", 5);
        let irr = Segment::new("mem", "完全不相干的天气记录", 5);
        let b_rel = rot_breakdown(&rel, Some("乌龙茶偏好"), &cfg());
        let b_irr = rot_breakdown(&irr, Some("乌龙茶偏好"), &cfg());
        assert!(b_rel.irrelevance < b_irr.irrelevance, "命中 query → 无关度更低");
        assert!(b_rel.score < b_irr.score, "相关段总分应更低 (更该保留)");
    }

    #[test]
    fn rot_score_bounds_and_determinism() {
        let seg = Segment::new("mem", "重复行\n".repeat(5) + "独有尾巴", 40);
        let a = rot_score(&seg, Some("尾巴"), &cfg());
        let b = rot_score(&seg, Some("尾巴"), &cfg());
        assert_eq!(a, b, "确定性: 同输入同输出");
        assert!((0.0..=1.0).contains(&a), "总分有界 [0,1], got {a}");
        let empty = Segment::new("mem", "", 100);
        let b_empty = rot_breakdown(&empty, None, &cfg());
        assert_eq!(b_empty.repetition, 0.0, "空内容 repetition=0");
        assert!((0.0..=1.0).contains(&b_empty.score), "空内容分数仍不越界");
    }

    // ── 阈值触发 + retain/remove/replace ─────────────────────────────

    #[test]
    fn threshold_triggers_replace() {
        let rotten = Segment::new("mem", "旧事重提\n".repeat(20), 80); // 高重复+高陈旧 → 超阈
        let healthy = Segment::new("mem", "新鲜独有内容一条", 0);
        let c = DeterministicCompactor::default();
        assert!(rot_score(&rotten, None, &c.rot) >= c.threshold, "腐烂段应超阈值");
        assert!(rot_score(&healthy, None, &c.rot) < c.threshold, "健康段应低于阈值");
        let ops = c.decide(&[rotten.clone(), healthy], None);
        match &ops[0] {
            CompactionOp::Replace(s) => {
                assert!(s.chars().count() <= c.summary_chars, "摘要受上限约束");
                assert!(s.chars().count() < rotten.content.chars().count(), "摘要应短于原文");
                assert!(s.contains("旧事重提"), "抽取式摘要保留原内容代表行");
            }
            other => panic!("超阈值段应 Replace, got {other:?}"),
        }
        assert_eq!(ops[1], CompactionOp::Retain, "健康段应 Retain");
    }

    #[test]
    fn core_segments_never_compacted() {
        let core_rotten = Segment::new("persona", "重复人格\n".repeat(30), 99).core(true);
        let ops = DeterministicCompactor::default().decide(&[core_rotten], None);
        assert_eq!(ops[0], CompactionOp::Retain, "核心段保护, 对齐 context.rs");
    }

    #[test]
    fn remove_when_nothing_to_summarize() {
        let blank_rotten = Segment::new("mem", "\n\n   \n", 999);
        // 全空白段: 超阈 (陈旧度≈1) 且抽取式摘要为空 → Remove
        let c = DeterministicCompactor { threshold: 0.5, ..DeterministicCompactor::default() };
        assert!(rot_score(&blank_rotten, None, &c.rot) >= c.threshold);
        let ops = c.decide(&[blank_rotten], None);
        assert_eq!(ops[0], CompactionOp::Remove, "无可摘要内容 → Remove");
    }

    #[test]
    fn apply_ops_retain_remove_replace() {
        let segs = vec![
            Segment::new("a", "保留我", 0),
            Segment::new("b", "删掉我", 1),
            Segment::new("c", "整段换成摘要", 2),
        ];
        let ops = vec![
            CompactionOp::Retain,
            CompactionOp::Remove,
            CompactionOp::Replace("摘要".into()),
        ];
        let out = apply_ops(&segs, &ops);
        assert_eq!(out.len(), 2, "Remove 应丢弃段");
        assert_eq!(out[0].content, "保留我");
        assert_eq!(out[1].content, "摘要", "Replace 应换内容");
        assert_eq!(out[1].name, "c", "Replace 保留段名");
        assert_eq!(out[1].age_turns, 2, "Replace 保留年龄属性");
    }

    #[test]
    fn compactor_trait_slot_usable_without_llm() {
        // LLM 参与版留口证明: trait 可被非 LLM 实现 (此处脚本式) 直接插入管线
        struct Scripted(Vec<CompactionOp>);
        impl Compactor for Scripted {
            fn decide(&self, segments: &[Segment], _q: Option<&str>) -> Vec<CompactionOp> {
                assert_eq!(segments.len(), self.0.len());
                self.0.clone()
            }
        }
        let segs = vec![Segment::new("x", "原文", 0)];
        let out = compact_then_budget(&segs, &Scripted(vec![CompactionOp::Replace("脚本摘要".into())]), None, 1000);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].content, "脚本摘要");
    }

    // ── 与尾部截断协作 ──────────────────────────────────────────────

    #[test]
    fn rot_driven_compaction_beats_blind_tail_truncation() {
        // 高价值段: 新鲜+相关 (低 rot); 腐烂段: 长且高重复+陈旧 (高 rot).
        let valuable = Segment::new("mem", "用户今天说喜欢乌龙茶", 0);
        let rotten = Segment::new("mem", "陈年旧事复读机\n".repeat(60), 90); // ~540 字腐烂段
        let segs = vec![valuable.clone(), rotten.clone()];
        let budget = 200; // 总 ~560 字 > 预算

        // 盲截尾 (context.rs 现状行为直跑): 大头先砍, 不辨价值
        let mut blind = ContextAssembler::new(budget);
        for s in &segs {
            blind = blind.push(ContextBlock { name: s.name, content: s.content.clone(), core: s.core, cap_chars: None });
        }
        let blind_out = blind.assemble_budgeted_blocks();

        // rot 驱动选择性压缩 + 截尾兜底
        let smart_out =
            compact_then_budget(&segs, &DeterministicCompactor::default(), Some("乌龙茶"), budget);

        let smart_total: usize = smart_out.iter().map(|b| b.content.chars().count()).sum();
        assert!(smart_total <= budget, "协作后仍受预算约束");
        // 智能版: 高价值段完整保留
        assert!(
            smart_out.iter().any(|b| b.content.contains("用户今天说喜欢乌龙茶")),
            "rot 驱动应完整保留高价值段"
        );
        // 盲截尾对照: 腐烂大头被砍但高价值段也可能受累 — 智能版不劣于盲截尾的价值保留
        let blind_valuable: usize = blind_out
            .iter()
            .filter(|b| b.content.contains("用户今天说喜欢乌龙茶"))
            .map(|b| b.content.chars().count())
            .sum();
        let smart_valuable: usize = smart_out
            .iter()
            .filter(|b| b.content.contains("用户今天说喜欢乌龙茶"))
            .map(|b| b.content.chars().count())
            .sum();
        assert!(smart_valuable >= blind_valuable, "选择性压缩保留的高价值内容不少于盲截尾");
        // 腐烂段在智能版中被摘要化或移除, 不再占据大头
        let smart_rotten: usize = smart_out
            .iter()
            .filter(|b| b.content.contains("陈年旧事复读机"))
            .map(|b| b.content.chars().count())
            .sum();
        assert!(smart_rotten <= DeterministicCompactor::default().summary_chars, "腐烂段应被压缩到摘要上限内");
    }
}
