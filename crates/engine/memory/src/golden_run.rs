//! `apeireth-memory::golden_run` — 把考卷接到**真实检索**上，跑出真分数。
//!
//! # 这个模块做什么
//!
//! 之前的批次把「考卷」和「挖题」都做出来了，但**没有把它们接到检索上**
//! ⇒ 卷子的分数一直是空的 ⇒ **「卷子到底测不测东西」没人知道**。
//!
//! 本模块把链条闭合：
//! **语料 ⇒ 记录 ⇒ 索引 ⇒ 检索 ⇒ 评测** ⇒ 产出 [`RunReport`]。
//!
//! # 为什么要「跑一次就知道卷子坏没坏」
//!
//! 2026-10-26 踩过的坑：题目由记录的**标题**生成，而可检索文本**又把标题拼了进去**
//! ⇒ 检索是**字面命中** ⇒ `recall@10` **恒等于满分** ⇒ **卷子测不出任何东西**。
//!
//! ⇒ 本模块**刻意不**假设卷子是好的 ⇒ 它跑出来的数字就是事实：
//! * 如果 `recall@10` 接近 1.0 ⇒ **卷子坏了**（题面可字面命中）；
//! * 如果 `recall@10` 明显低于 1.0 ⇒ 卷子开始**真的在测检索**。
//!
//! # 口径
//!
//! * **NOT IMPLEMENTED（新增未接线）**：库级实现；生产 0 调用点；无开关位。
//! * 检索用**仓里已有的** [`BasicLexicalCandidateSource`]（确定性 token 重叠）
//!   ⇒ **不引新依赖**、**结果可复现**。
//! * 评测用 [`crate::golden_set::evaluate`]（recall@k / nDCG@k / evidence 族）。

use std::collections::BTreeMap;

use crate::golden_set::{EvalBatch, GoldenCase, GoldenSet, Metrics};
use crate::retrieval_pipeline::Bm25LexicalCandidateSource;
use crate::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};

/// 注入口径（**注入给模型什么**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionMode {
    /// **与生产逐字一致**：每条证据 = 记忆正文的**前 120 字**
    /// （`memory_injection::EVIDENCE_MAX_CHARS`）⇒ 从**开头**截。
    Production,
    /// **对照口径**：注入与查询**最相关**的那一段
    /// ⇒ 用来量化「若改注入策略，证据族指标能涨多少」。
    BestSpan,
}

impl InjectionMode {
    /// 生产口径的每条证据字符数（**逐字对齐生产常量**）。
    pub const PRODUCTION_CHARS: usize = crate::memory_injection::EVIDENCE_MAX_CHARS;
}

/// 一条可检索的记录（**可检索文本** = 正文）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRecord {
    /// 记录 id（与考卷里的 `expect_ids` 对得上）。
    pub id: String,
    /// 可检索文本（**不含题面来源** ⇒ 不泄漏答案）。
    pub text: String,
}

/// 一次跑批的结果。
#[derive(Debug, Clone, PartialEq)]
pub struct RunReport {
    /// 语料里的记录数。
    pub corpus_size: usize,
    /// 参与评测的题数。
    pub cases: usize,
    /// 评测出的指标（`k` = 评测的 top-k）。
    pub metrics: Metrics,
    /// **逐题对错**（`true` = 该题的目标记录出现在 top-k 里）⇒ 便于定位「哪几题检索不到」。
    pub per_case_hit: Vec<(String, bool)>,
    /// 检索用的 `k`。
    pub k: usize,
    /// **完全命中的题数**（多目标题：全部命中才算）。
    pub fully_hit: usize,
}

/// 注入片段的长度（字）—— 与真实注入「把命中记忆的一段塞进上下文」同量级。
const EVIDENCE_SNIPPET_CHARS: usize = 160;

/// 取正文的前 N 字（**按字符边界**，不切坏 UTF-8）。
fn snippet(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    trimmed.chars().take(max_chars).collect()
}

/// 注入片段：**记录里与查询最相关的那一段**（不是开头）。
///
/// # 为什么不能注入「开头」
///
/// 第一版注入的是记录**前 160 字** ⇒ 而期望片段在正文**中间**
/// ⇒ `evidence_precision` 只有 0.02（**看着像检索很差，其实是不会注入**）。
/// ⇒ 真实注入是把**命中的那部分**塞进上下文 ⇒ 这里按**词重叠**选最相关的一段，
/// 与真实注入**同口径**。
///
/// # 口径
///
/// * 窗口 = `max_chars` 字，步长 = 窗口的一半（重叠滑窗）；
/// * 评分 = 窗口与查询的**词重叠数**（与检索器同口径：CJK bigram + ASCII 词）；
/// * 并列时取**靠前**的窗口（确定性）。
fn best_snippet(text: &str, query: &str, max_chars: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max_chars {
        return text.trim().to_string();
    }
    let query_tokens = crate::retrieval_pipeline::unicode_tokens(query);
    if query_tokens.is_empty() {
        return snippet(text, max_chars);
    }
    let step = (max_chars / 2).max(1);
    let mut best_score = 0usize;
    let mut best_start = 0usize;
    let mut start = 0usize;
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let window: String = chars[start..end].iter().collect();
        let window_tokens = crate::retrieval_pipeline::unicode_tokens(&window);
        let unique: std::collections::HashSet<&String> = window_tokens.iter().collect();
        let score = query_tokens
            .iter()
            .filter(|token| unique.contains(token))
            .count();
        if score > best_score {
            best_score = score;
            best_start = start;
        }
        if end == chars.len() {
            break;
        }
        start += step;
    }
    chars[best_start..(best_start + max_chars).min(chars.len())]
        .iter()
        .collect()
}

/// 建索引（把记录变成可检索的候选源）。
///
/// ⚠️ **索引里放的是「可检索文本」**（不含标题）⇒ 题面**无法字面命中**。
///
/// 用**生产用的 BM25 源**（[`Bm25LexicalCandidateSource`]，仓里的确定性 Okapi BM25 引擎）
/// 而不是 legacy 的 token 重叠源 ⇒ 跑出来的分数**与生产检索同口径**。
fn build_source(records: &[IndexRecord]) -> Bm25LexicalCandidateSource {
    let documents = records
        .iter()
        .map(|record| MemoryCandidate {
            id: record.id.clone(),
            layer: "golden".to_string(),
            scope: MemoryScope::Global,
            content: record.text.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance {
                source: "golden_corpus".to_string(),
                source_session: None,
                source_trace: None,
                source_request: None,
            },
        })
        .collect();
    Bm25LexicalCandidateSource::new(documents)
}

/// 跑一次批：给定语料 + 考卷 ⇒ 检索 ⇒ 评测。
///
/// # 口径
///
/// * `k` = 取前 k 个检索结果（**k 与评测口径一致**：同一份 `k` 既用于检索截断，也用于 recall@k）；
/// * 命中判定：目标记录 id 出现在前 k 个结果里；
/// * **多目标题**（`expect_ids` 有多条）：`per_case_hit` 记「**全部命中**」才算命中
///   （部分命中单独统计在 `metrics` 的 recall 口径里）。
pub fn run(
    records: &[IndexRecord],
    golden: &GoldenSet,
    k: usize,
    mode: InjectionMode,
) -> Result<RunReport, crate::MemoryError> {
    use crate::retrieval_pipeline::MemoryCandidateSource;

    let source = build_source(records);
    let mut batch = EvalBatch::default();
    let mut per_case_hit: Vec<(String, bool)> = Vec::new();
    let mut fully_hit = 0usize;

    for case in &golden.cases {
        let retrieved = source.candidates(&case.query, k)?;
        // ⚠️ **零分结果不算检索结果**：BM25 源会**按 id 顺序填满** limit
        // ⇒ 分数为 0 的文档也会被返回 ⇒ 「目标恰好排在第 1」会**假命中**。
        // ⇒ 只保留**分数 > 0** 的结果（真检索器也会设分数阈值）。
        let retrieved: Vec<_> = retrieved
            .into_iter()
            .filter(|item| item.score > 0.0)
            .collect();
        let retrieved_ids: Vec<String> = retrieved.iter().map(|item| item.id.clone()).collect();
        let hit = !case.expect_ids.is_empty()
            && case
                .expect_ids
                .iter()
                .all(|expected| retrieved_ids.contains(expected));
        if hit {
            fully_hit += 1;
        }
        per_case_hit.push((case.id.clone(), hit));
        // `EvalBatch` 的 cases 与 results **必须等长**（`evaluate` 会校验）⇒ 两边都填。
        batch.cases.push(case.clone());
        // ⚠️ **必须填「注入给模型的证据片段」**：评测的
        // `evidence_precision` / `groundedness` 算的是「**注入的片段里有多少是期望片段**」
        // ⇒ 不填 ⇒ 这两个指标恒为 0（**看起来像检索很差，其实是没喂证据**）。
        //
        // 口径：**按检索顺序**给每条返回记录取一段正文（`EVIDENCE_SNIPPET_CHARS` 字），
        // ⇒ 与真实注入（把命中的记忆片段塞进上下文）**同口径**。
        let fragments: Vec<String> = retrieved
            .into_iter()
            .filter_map(|item| {
                records
                    .iter()
                    .find(|record| record.id == item.id)
                    .map(|record| match mode {
                        InjectionMode::Production => {
                            // **与生产逐字一致**：取正文**前 120 字**（从开头截）。
                            record
                                .text
                                .chars()
                                .take(InjectionMode::PRODUCTION_CHARS)
                                .collect()
                        }
                        InjectionMode::BestSpan => {
                            best_snippet(&record.text, &case.query, EVIDENCE_SNIPPET_CHARS)
                        }
                    })
            })
            .collect();
        batch.results.push(crate::golden_set::Retrieved {
            ids: retrieved_ids,
            fragments,
        });
    }

    // `evaluate` 的错误（批次长度不配等）⇒ 用 `MemoryError::Other` 承载
    // （`MemoryError` 没有 `GoldenSetError` 变体；加变体会动公共错误枚举 ⇒ 不值得）。
    let metrics = crate::golden_set::evaluate(&batch, k)
        .map_err(|error| crate::MemoryError::Other(format!("考卷评测失败：{error}")))?;
    Ok(RunReport {
        corpus_size: records.len(),
        cases: golden.cases.len(),
        metrics,
        per_case_hit,
        k,
        fully_hit,
    })
}

/// 报告里的人话摘要。
pub fn summarize(report: &RunReport) -> String {
    let ratio = if report.cases == 0 {
        0.0
    } else {
        report.fully_hit as f64 / report.cases as f64
    };
    let mut lines = Vec::new();
    lines.push(format!(
        "语料 {} 条 · 题 {} 道 · k={}",
        report.corpus_size, report.cases, report.k
    ));
    lines.push(format!(
        "全中题数 {} / {}（{:.1}%）",
        report.fully_hit,
        report.cases,
        ratio * 100.0
    ));
    lines.push(format!(
        "recall@{}={:.4} · nDCG@{}={:.4} · evidence_precision={:.4} · groundedness={:.4}",
        report.k,
        report.metrics.recall_at_k,
        report.k,
        report.metrics.ndcg_at_k,
        report.metrics.evidence_precision,
        report.metrics.groundedness
    ));
    lines.push(format!(
        "跳过 {} 题（无标注/不适用）· 有效 {} 题",
        report.metrics.skipped_cases, report.metrics.total_cases
    ));
    // ⚠️ **口径陷阱**：`k` ≥ 语料条数 ⇒ 全部记录都进 top-k ⇒ recall **恒为 1**
    // ⇒ 这个分数**没有信息量**（不是「检索很好」）。
    if report.k >= report.corpus_size && report.corpus_size > 0 {
        lines.push(format!(
            "⚠️ **k={} ≥ 语料条数 {}** ⇒ 全部记录都进 top-k ⇒ recall **恒为 1** ⇒ 这个口径**没有信息量**",
            report.k, report.corpus_size
        ));
    }
    // **最关键的一句**：卷子有没有在「真的测东西」。
    lines.push(if ratio > 0.95 {
        format!(
            "⚠️ **全中率 {:.1}% 过高** ⇒ 题面可能被**字面命中** ⇒ 卷子**测不出东西**（查「题面来源是否进了可检索文本」）",
            ratio * 100.0
        )
    } else {
        format!(
            "✓ 全中率 {:.1}% ⇒ 卷子**不是**字面命中 ⇒ 它**真的在测检索**",
            ratio * 100.0
        )
    });
    lines.join("\n")
}

/// 按题型统计全中率（便于定位「哪类题检索不到」）。
pub fn hits_by_kind(report: &RunReport, golden: &GoldenSet) -> BTreeMap<String, (usize, usize)> {
    let mut stats: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (case, (_, hit)) in golden.cases.iter().zip(report.per_case_hit.iter()) {
        let kind = case
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("kind:").map(str::to_string))
            .unwrap_or_else(|| "unknown".to_string());
        let entry = stats.entry(kind).or_insert((0, 0));
        entry.1 += 1;
        if *hit {
            entry.0 += 1;
        }
    }
    stats
}

/// 把挖题器的候选（[`crate::golden_mining::MinedCase`]）转成**考卷**格式。
///
/// # 为什么需要这一步
///
/// 挖题器产出的是 `MinedCase`（带 `kind` 字段），考卷要的是 `GoldenCase`
/// （带 `tags` 字段）⇒ 两者**不同型** ⇒ 中间必须有这一层转换。
/// （之前一直没接上 ⇒ 挖出来的题**进不了评测** ⇒ 卷子的分数一直是空的。）
///
/// # 口径
///
/// * `kind`（题型）⇒ 变成 `kind:<题型>` 标签 ⇒ 评测能**按题型分组**看分数；
/// * 再补 `internal` 标签 ⇒ 区分「内部卷」与「外部卷」；
/// * `expect_ids` / `expect_fragments` / `query` / `id` **逐字保留**（不重写）。
pub fn mined_to_golden(cases: Vec<crate::golden_mining::MinedCase>) -> Vec<GoldenCase> {
    cases
        .into_iter()
        .map(|case| {
            let mut tags = vec!["internal".to_string(), format!("kind:{}", case.kind)];
            tags.sort();
            GoldenCase {
                id: case.id,
                query: case.query,
                expect_ids: case.expect_ids,
                expect_fragments: case.expect_fragments,
                tags,
            }
        })
        .collect()
}
