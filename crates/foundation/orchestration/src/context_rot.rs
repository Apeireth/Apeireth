//! `apeireth-orchestration::context_rot` — 上下文衰减 (Context Rot) 度量与段压缩编辑原语.
//!
//! R12-CoordinationContext-1 实施 (per `r12-coordination-context-1-merge-spec-2026-08-28.md`).
//!
//! **设计哲学 (M1 记忆与上下文编排)**:
//! - **① rot_score 度量**: 重复度 (repetition)、陈旧度 (staleness)、无关度 (irrelevance)
//!   启发式三因子, 确定性公式, 0 LLM 依赖.
//! - **② 段编辑原语**: `Retain` / `Remove` / `Replace` 动作; LLM 参与版留 [`Compactor`] trait 口
//!   (0 装: 生产与测试默认使用 [`DeterministicCompactor`]).
//! - **③ 核心段保护**: `core = true` 的段 (如系统设定/主人核心偏好) 绝不压缩或剔除.
//!
//! **O-6 三阶审查**:
//! 1. 总体: 解决长会话上下文膨胀与腐烂, 为 v2 Agent Loop 提供确定性预裁剪能力
//! 2. 系统: 放置在 `foundation/orchestration`, 上下文分段与打分契约对 runtime 与 memory 统一暴露
//! 3. 架构: 纯确定性算法, 0 unsafe code, 0 外部新依赖

use serde::{Deserialize, Serialize};

/// 待打分与编辑的上下文段快照.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Segment {
    /// 段标识/名称 (如 "persona", "recent_dialogue", "memory_notes").
    pub name: String,
    /// 段正文内容.
    pub content: String,
    /// 核心保护标志 (true 表示该段为不可裁减的核心系统设定/人设).
    pub core: bool,
    /// 陈旧度输入: 距当前轮次的轮数 (0 = 最新或无轮次概念).
    pub age_turns: usize,
}

impl Segment {
    /// 构造新的上下文段.
    pub fn new(name: impl Into<String>, content: impl Into<String>, age_turns: usize) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            core: false,
            age_turns,
        }
    }

    /// 设置是否为核心受保护段.
    #[must_use]
    pub fn with_core(mut self, core: bool) -> Self {
        self.core = core;
        self
    }
}

/// `rot_score` 三因子权重与衰减配置. 全确定性, 0 LLM 依赖.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotConfig {
    /// 重复度权重 (默认 0.4 — 重复内容是上下文腐烂最强信号).
    pub w_repetition: f32,
    /// 陈旧度权重 (默认 0.3).
    pub w_staleness: f32,
    /// 相关性权重 (默认 0.3; 无 query 时权重自动归一化到前两项).
    pub w_relevance: f32,
    /// 陈旧度半衰期轮数: `staleness = age / (age + half_life)`, 单调有界在 `[0, 1)`.
    pub stale_half_life_turns: f32,
}

impl Default for RotConfig {
    fn default() -> Self {
        Self {
            w_repetition: 0.4,
            w_staleness: 0.3,
            w_relevance: 0.3,
            stale_half_life_turns: 20.0,
        }
    }
}

/// 三因子打分细分明细 (用于审计、追踪与诊断).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotBreakdown {
    /// 重复度评分 `[0, 1]`: 1.0 表示整段内容完全重复.
    pub repetition: f32,
    /// 陈旧度评分 `[0, 1)`: `age / (age + half_life)`.
    pub staleness: f32,
    /// 无关度评分 `[0, 1]`: 1.0 表示 query 词元完全未命中; 无 query 时恒为 0.0 且权重归一化.
    pub irrelevance: f32,
    /// 加权总腐烂分 `[0, 1]`: 分数越高表示越腐烂, 越应优先压缩或移除.
    pub score: f32,
}

/// 计算文本重复度因子:
/// - 多行文本 (>=2 行): 计算行级去重比 `1.0 - (uniq_lines / total_lines)`.
/// - 单行文本: 计算 6 字符滑窗去重比.
pub fn repetition_factor(content: &str) -> f32 {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() >= 2 {
        let total = lines.len();
        let mut uniq: Vec<&str> = lines.clone();
        uniq.sort_unstable();
        uniq.dedup();
        return 1.0 - (uniq.len() as f32) / (total as f32);
    }
    // 单行: 6-char 滑窗
    let chars: Vec<char> = content.chars().collect();
    if chars.len() < 6 {
        return 0.0;
    }
    let windows = chars.len() - 5;
    let mut seen = std::collections::HashSet::new();
    for i in 0..windows {
        seen.insert(&chars[i..i + 6]);
    }
    1.0 - (seen.len() as f32) / (windows as f32)
}

/// 提取相关性查询词元: ASCII 小写单词 + CJK 连续二字 (char-bigram).
/// 全确定性, 0 外部分词库依赖.
pub fn query_tokens(query: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut ascii_word = String::new();
    let mut cjk_prev: Option<char> = None;

    for c in query.chars() {
        if c.is_ascii_alphanumeric() {
            ascii_word.push(c.to_ascii_lowercase());
            cjk_prev = None;
        } else if c.is_alphabetic() && (c as u32) > 0x2E80 {
            // CJK 字符: 先冲刷已积累的 ascii 单词
            if !ascii_word.is_empty() {
                out.push(std::mem::take(&mut ascii_word));
            }
            if let Some(p) = cjk_prev {
                out.push(format!("{p}{c}"));
            } else {
                out.push(c.to_string()); // 单字保底
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

/// 上下文段三因子分解与打分 (确定性公式, 0 LLM).
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

    RotBreakdown {
        repetition,
        staleness,
        irrelevance,
        score,
    }
}

/// 计算单段上下文腐烂分 (总分 `[0, 1]`, 越高越应优先压缩或剔除).
pub fn rot_score(seg: &Segment, query: Option<&str>, cfg: &RotConfig) -> f32 {
    rot_breakdown(seg, query, cfg).score
}

/// 上下文段压缩编辑操作.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionOp {
    /// 保留原段不变.
    Retain,
    /// 移除整段.
    Remove,
    /// 以抽取或生成的摘要文本替换整段.
    Replace(String),
}

/// 压缩决策器 Trait.
pub trait Compactor: Send + Sync {
    /// 对一组段生成对应的压缩编辑操作 (返回的 ops 与 segments 一一对应).
    fn decide(&self, segments: &[Segment], query: Option<&str>) -> Vec<CompactionOp>;
}

/// 确定性规则压缩器:
/// - 核心段 (`core = true`) 永远 Retain;
/// - `rot_score < threshold` 的段 Retain;
/// - `rot_score >= threshold` 的段生成抽取式摘要 (Replace);
/// - 若无有效内容可摘要则丢弃 (Remove).
#[derive(Debug, Clone)]
pub struct DeterministicCompactor {
    /// 触发压缩的腐烂分阈值 (默认 0.6).
    pub threshold: f32,
    /// 抽取式摘要字符上限 (默认 120).
    pub summary_chars: usize,
    /// 打分配置.
    pub rot: RotConfig,
}

impl Default for DeterministicCompactor {
    fn default() -> Self {
        Self {
            threshold: 0.6,
            summary_chars: 120,
            rot: RotConfig::default(),
        }
    }
}

/// 确定性抽取式摘要 (0 LLM): 按行去重、保留顺序、截断到上限字符数.
pub fn extractive_summary(content: &str, max_chars: usize) -> String {
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
                    return CompactionOp::Retain; // 核心保护
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

/// 应用段编辑操作列表, 返回编辑裁剪后的段集合.
pub fn apply_ops(segments: &[Segment], ops: &[CompactionOp]) -> Vec<Segment> {
    segments
        .iter()
        .zip(ops.iter())
        .filter_map(|(s, op)| match op {
            CompactionOp::Retain => Some(s.clone()),
            CompactionOp::Remove => None,
            CompactionOp::Replace(text) => Some(Segment {
                content: text.clone(),
                ..s.clone()
            }),
        })
        .collect()
}

/// 预算感知的上下文分段聚合块.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetedBlock {
    pub name: String,
    pub content: String,
    pub core: bool,
}

/// 结合 rot 压缩与字符预算的流水线:
/// 1. 先用 Compactor 对高腐烂段进行选择性压缩/丢弃 (保留高价值与核心段);
/// 2. 若总字符数仍超出预算, 按非核心段从后往前做安全截断.
pub fn compact_then_budget<C: Compactor>(
    segments: &[Segment],
    compactor: &C,
    query: Option<&str>,
    total_budget_chars: usize,
) -> Vec<BudgetedBlock> {
    let ops = compactor.decide(segments, query);
    let edited = apply_ops(segments, &ops);

    let mut out = Vec::new();
    let mut current_chars = 0;

    // 先保留所有核心段
    for s in &edited {
        if s.core {
            current_chars += s.content.chars().count();
            out.push(BudgetedBlock {
                name: s.name.clone(),
                content: s.content.clone(),
                core: true,
            });
        }
    }

    // 再在剩余预算内填充非核心段
    for s in &edited {
        if !s.core {
            let seg_chars = s.content.chars().count();
            if current_chars + seg_chars <= total_budget_chars {
                current_chars += seg_chars;
                out.push(BudgetedBlock {
                    name: s.name.clone(),
                    content: s.content.clone(),
                    core: false,
                });
            } else {
                let remaining = total_budget_chars.saturating_sub(current_chars);
                if remaining > 20 {
                    let truncated: String = s.content.chars().take(remaining).collect();
                    out.push(BudgetedBlock {
                        name: s.name.clone(),
                        content: truncated,
                        core: false,
                    });
                    break;
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> RotConfig {
        RotConfig::default()
    }

    #[test]
    fn rot_score_repetition_factor() {
        let rep = Segment::new("mem", "同一件事说了七遍\n".repeat(7), 0);
        let uniq = Segment::new("mem", "甲乙丙丁戊己庚辛壬癸子丑寅卯辰巳午未申酉戌亥", 0);
        let b_rep = rot_breakdown(&rep, None, &cfg());
        let b_uniq = rot_breakdown(&uniq, None, &cfg());
        assert!(b_rep.repetition > 0.8, "高重复行级因子应接近 1");
        assert!(b_uniq.repetition < 0.2, "唯一内容因子应低");
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
        assert!(
            b_rel.irrelevance < b_irr.irrelevance,
            "命中 query → 无关度更低"
        );
        assert!(b_rel.score < b_irr.score, "相关段总分应更低 (更应保留)");
    }

    #[test]
    fn rot_score_bounds_and_determinism() {
        let seg = Segment::new("mem", "重复行\n".repeat(5) + "独有尾巴", 40);
        let a = rot_score(&seg, Some("尾巴"), &cfg());
        let b = rot_score(&seg, Some("尾巴"), &cfg());
        assert_eq!(a, b, "确定性: 同输入同输出");
        assert!((0.0..=1.0).contains(&a), "总分有界 [0, 1]");
    }

    #[test]
    fn threshold_triggers_replace_and_retain() {
        let rotten = Segment::new("mem", "旧事重提\n".repeat(20), 80);
        let healthy = Segment::new("mem", "新鲜独有内容一条", 0);
        let c = DeterministicCompactor::default();
        let ops = c.decide(&[rotten.clone(), healthy], None);
        match &ops[0] {
            CompactionOp::Replace(s) => {
                assert!(s.chars().count() <= c.summary_chars);
                assert!(s.contains("旧事重提"));
            }
            other => panic!("超阈值段应 Replace, got {other:?}"),
        }
        assert_eq!(ops[1], CompactionOp::Retain);
    }

    #[test]
    fn core_segment_protected() {
        let core_rotten = Segment::new("persona", "重复人格\n".repeat(30), 99).with_core(true);
        let ops = DeterministicCompactor::default().decide(&[core_rotten], None);
        assert_eq!(ops[0], CompactionOp::Retain, "核心段必须受保护 Retain");
    }

    #[test]
    fn compact_then_budget_pipeline_works() {
        let segs = vec![
            Segment::new("system", "系统核心设定", 0).with_core(true),
            Segment::new("chat", "新鲜对话内容", 1),
            Segment::new("history", "冗余历史记录\n".repeat(20), 100),
        ];
        let c = DeterministicCompactor::default();
        let budgeted = compact_then_budget(&segs, &c, None, 200);
        assert!(!budgeted.is_empty());
        assert!(budgeted[0].core);
        assert_eq!(budgeted[0].name, "system");
    }
}
