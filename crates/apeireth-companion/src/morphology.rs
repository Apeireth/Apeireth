//! 查询形态学 softmax (N7: VCP rust-vexus-lite rivermemo_topology_v3:1784-2011 吸收)
//!
//! **职责**: 从查询文本提取确定性文本形态特征 (长度/实体密度/疑问形态/分句数/深度线索)
//! → 三档 logits → softmax 分布 → 检索模式档位 (浅扫/标准/深爬) + CRAWL 期望预算.
//! **纯函数**: 同查询同档位 — 0 随机 / 0 IO / 0 LLM; 温度只调分布锐度, 决策可复现.
//!
//! **与 VCP 的差异 (诚实登记)**: VCP 原版用河网 hop 分布/HHI/前向流占比等图拓扑特征;
//! Apeireth 无河网数据结构, 改用文本形态特征 — 机制同构 (logits+softmax+档位),
//! 特征为手调启发式常量 (0 装 PASS: 未学习/未调参验证).
//!
//! **接线**: assemble.rs inject_memory → `crawl_budget(query)` → memory_graph.crawl(seeds, budget).

/// 检索模式档位.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    /// 浅扫: 短直接查询 (CRAWL 基准预算 1).
    Shallow,
    /// 标准: 多实体/关系型查询 (CRAWL 基准预算 3).
    Standard,
    /// 深爬: 长查询 + 深度线索 (CRAWL 基准预算 6).
    Deep,
}

impl RetrievalMode {
    /// 档位 → CRAWL 基准预算 (crawl budget 语义 = BFS 展开条目数上限).
    pub fn base_budget(self) -> usize {
        match self {
            RetrievalMode::Shallow => 1,
            RetrievalMode::Standard => 3,
            RetrievalMode::Deep => 6,
        }
    }
}

/// 判决: 主导档位 (argmax) + softmax 分布 [浅扫, 标准, 深爬].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MorphologyVerdict {
    pub mode: RetrievalMode,
    pub weights: [f64; 3],
}

impl MorphologyVerdict {
    /// 分布期望预算 (温度影响锐度 → 影响深度), 钳位 [1, 6].
    pub fn budget(&self) -> usize {
        let v = self.weights[0] * 1.0 + self.weights[1] * 3.0 + self.weights[2] * 6.0;
        (v.round() as usize).clamp(1, 6)
    }
}

/// 深度线索词 (叙事/回溯/梳理诉求 → 推高深爬 logit).
const DEPTH_CUES: &[&str] = &[
    "详细", "深入", "全面", "背景", "历史", "过程", "来龙去脉", "前因后果", "为什么", "原因",
    "梳理", "总结", "回顾", "整个",
];
/// 疑问形态词 (直接提问 → 推高浅扫 logit).
const QUESTION_MARKS: &[&str] = &[
    "？", "?", "吗", "呢", "怎么", "如何", "是否", "哪些", "什么", "为什么",
];

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn cue_hits(q: &str, cues: &[&str]) -> f64 {
    cues.iter().map(|c| q.matches(c).count()).sum::<usize>() as f64
}

/// 文本形态特征 (全部确定性, 无随机/IO).
struct Features {
    length: f64,
    entity: f64,
    question: f64,
    clauses: f64,
    depth: f64,
}

fn extract(q: &str) -> Features {
    let total = q.chars().count();
    let length = clamp01(total as f64 / 60.0);
    let entity = if total == 0 {
        0.0
    } else {
        let dense = q.chars().filter(|c| c.is_alphanumeric()).count();
        dense as f64 / total as f64
    };
    let question = clamp01(cue_hits(q, QUESTION_MARKS) / 2.0);
    let depth = clamp01(cue_hits(q, DEPTH_CUES) / 2.0);
    let segs = q
        .split(|c: char| matches!(c, '，' | ',' | '。' | '！' | '!' | '？' | '?' | '；' | ';' | '、' | '：' | ':' | '\n'))
        .filter(|s| !s.trim().is_empty())
        .count();
    let clauses = clamp01(segs.saturating_sub(1) as f64 / 3.0);
    Features { length, entity, question, clauses, depth }
}

/// 三档 logits [浅扫, 标准, 深爬] (系数为手调启发式, 仿 VCP 加权结构).
fn logits(f: &Features) -> [f64; 3] {
    [
        1.45 * (1.0 - f.length) + 0.9 * f.question - 1.25 * f.depth - 0.65 * f.clauses,
        0.35 + 1.25 * f.clauses + 0.7 * f.entity + 0.35 * f.question - 0.45 * f.depth,
        1.4 * f.length + 1.15 * f.depth + 0.8 * f.clauses + 0.3 * f.entity - 0.65 * f.question,
    ]
}

/// 温度净化: NaN/≤0/∞ → 1.0, 有效值钳位 [0.1, 10.0] (防 exp 退化).
pub fn sanitize_temperature(t: f64) -> f64 {
    if !t.is_finite() || t <= 0.0 {
        1.0
    } else {
        t.clamp(0.1, 10.0)
    }
}

/// 查询 → 档位 + softmax 分布. 纯函数: 同输入同输出.
pub fn classify(query: &str, temperature: f64) -> MorphologyVerdict {
    let t = sanitize_temperature(temperature);
    let l = logits(&extract(query));
    let m = l.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exp = l.map(|v| ((v - m) / t).exp());
    let sum: f64 = exp.iter().sum();
    let weights = exp.map(|v| v / sum.max(1e-12));
    let (idx, _) = weights
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, &0.0));
    let mode = match idx {
        0 => RetrievalMode::Shallow,
        1 => RetrievalMode::Standard,
        _ => RetrievalMode::Deep,
    };
    MorphologyVerdict { mode, weights }
}

/// 边界助手: 从 env 读温度 (APEIRETH_MORPHOLOGY_TEMPERATURE, 默认 1.0, 非法回落 1.0).
pub fn env_temperature() -> f64 {
    std::env::var("APEIRETH_MORPHOLOGY_TEMPERATURE")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(sanitize_temperature)
        .unwrap_or(1.0)
}

/// 挂接点一行式: 查询 → CRAWL 预算 (温度取 env).
pub fn crawl_budget(query: &str) -> usize {
    classify(query, env_temperature()).budget()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_same_query_same_mode() {
        for q in [
            "在吗",
            "",
            "帮我详细梳理项目背景和历史过程",
            "项目进度，测试情况，部署安排，分别怎么样了？",
        ] {
            let a = classify(q, 1.0);
            for _ in 0..5 {
                assert_eq!(classify(q, 1.0), a, "同查询同档位: {q}");
            }
        }
    }

    #[test]
    fn short_question_shallow() {
        let v = classify("在吗", 1.0);
        assert_eq!(v.mode, RetrievalMode::Shallow);
        assert!(v.budget() <= 2, "浅扫预算应接近 1: {}", v.budget());
    }

    #[test]
    fn multi_clause_relational_standard() {
        let v = classify("项目进度，测试情况，还有部署安排，分别怎么样了？", 1.0);
        assert_eq!(v.mode, RetrievalMode::Standard);
        assert_eq!(v.budget(), 3);
    }

    #[test]
    fn long_depth_query_deep() {
        let v = classify(
            "帮我详细梳理一下我们之前讨论的项目背景和历史过程，要全面的来龙去脉和原因",
            1.0,
        );
        assert_eq!(v.mode, RetrievalMode::Deep);
        assert!(v.budget() >= 4, "深爬预算应显著高于标准: {}", v.budget());
    }

    #[test]
    fn empty_query_shallow() {
        let v = classify("", 1.0);
        assert_eq!(v.mode, RetrievalMode::Shallow);
        assert!(v.budget() <= 2, "空查询走最浅: {}", v.budget());
    }

    #[test]
    fn huge_query_deep_no_panic() {
        let q = "背景".repeat(5000); // 1 万字符超长查询
        let v = classify(&q, 1.0);
        assert_eq!(v.mode, RetrievalMode::Deep);
    }

    #[test]
    fn weights_are_valid_distribution() {
        for q in ["你好", "帮我详细回顾整个历史", "进度，风险，资源，时间，分别什么情况？"] {
            let v = classify(q, 1.0);
            let sum: f64 = v.weights.iter().sum();
            assert!((sum - 1.0).abs() < 1e-9, "softmax 应归一: {sum}");
            assert!(v.weights.iter().all(|w| (0.0..=1.0).contains(w)));
        }
    }

    #[test]
    fn temperature_affects_sharpness() {
        let q = "帮我详细梳理一下我们之前讨论的项目背景和历史过程，要全面的来龙去脉和原因";
        let cold = classify(q, 0.1); // 锐化 → 期望预算贴近档位基准
        let hot = classify(q, 10.0); // 摊平 → 期望预算向中间收缩
        assert_eq!(cold.budget(), RetrievalMode::Deep.base_budget());
        assert!(hot.budget() < cold.budget(), "高温摊平应降低期望预算");
    }

    #[test]
    fn invalid_temperature_falls_back() {
        let base = classify("随便聊聊天", 1.0);
        for bad in [0.0, -3.0, f64::NAN, f64::INFINITY] {
            assert_eq!(classify("随便聊聊天", bad), base, "非法温度应回落 1.0: {bad}");
        }
    }

    #[test]
    fn budget_bounds() {
        let v = |w: [f64; 3]| MorphologyVerdict { mode: RetrievalMode::Shallow, weights: w };
        assert_eq!(v([1.0, 0.0, 0.0]).budget(), 1);
        assert_eq!(v([0.0, 1.0, 0.0]).budget(), 3);
        assert_eq!(v([0.0, 0.0, 1.0]).budget(), 6);
        assert_eq!(v([1.0 / 3.0; 3]).budget(), 3);
    }
}
