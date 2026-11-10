//! **真实查询卷的召回空间分析**（2026-11-08）—— 把「距天花板还差多少」拆成可归因的几段，
//! 并对**候选机制**逐个出 Δ 表。
//!
//! # 为什么需要这个测例
//!
//! 真人查询卷（真实母语问句 + 官方相关性标注）当前 `recall@10 = 0.7246`，
//! 而**池内天花板（oracle）= 0.9389** ⇒ ⇒ 表面上「还差 **0.2143**」。
//! 但这 0.2143 **不是同一种东西**，必须拆开：
//!
//! | 段 | 含义 | 谁能救 |
//! |---|---|---|
//! | **A 金标不在池里** | 采样池没收录该金标段落 | **任何人救不了**（池的边界） |
//! | **B 在池里但零命中** | 词面检索拿不到任何正分 | 词面召回面（换召回机制） |
//! | **C 零命中之外、没进 top-10** | 召回到了但排在 10 名开外 | **排序/扩召回面**（k、粒度、参数） |
//! | **D 已进 top-10** | 达成 | — |
//!
//! ⇒ ⇒ **A 是池的边界（改不了）**，**B/C 才是可动的空间** ⇒ 不拆开就会把
//! 「池子小」误当成「算法差」。
//!
//! # 口径
//!
//! * 语料 = `$APEIRETH_MIRACL_EXAM/corpus.jsonl`（默认 `~/.apeireth/miracl-exam/`）；
//!   考卷 = `exam.jsonl`（真实母语问句 + 官方标注）；
//! * 检索 = **生产同款**词法源（`Bm25LexicalCandidateSource`，生产默认参数）；
//! * 命中判定 = 与既有闸门**逐字同口径**（`golden_set::evaluate`：一题的命中比例 =
//!   top-k 命中的金标数 ÷ 该题金标数，按题平均）；
//! * **口径自证**：本测例在 **k=10 上的读数必须与既有闸门基线逐位相同**
//!   （`scripts/real-query-baseline.json`）；不一致会**如实报差异**，不粉饰。
//!
//! # 候选机制（本测例只**量**，不接生产）
//!
//! 1. **k（top-k）**：5/10/20/30/50/100 ⇒ 看天花板余量**在哪一档**；
//! 2. **BM25 参数**：`k1` / `b` / 中文停用字开关；
//! 3. **分块粒度**：整段 vs 句子窗口（窗口越小 ⇒ 命中越准但 top-10 装不下更多）；
//! 4. **去重**：top-10 里的近似重复对（同一 Jaccard ≥ 0.8 口径）。
//!
//! ⚠️ **不重做已量过的机制**：`first_pos`（真人卷 +16.9%）、`diversity_lambda`（内部卷净损失）、
//! 「锚点重塑」（内部卷无收益）—— 结论直接引用，不重复扫描。
//!
//! # 中性表述
//!
//! 只描述**机制**（召回空间 / 分块 / 参数扫描 / 词面相似度）；
//! 数据集名只出现在**数据集口径**处，不引用任何方法论文或外部实现。
//!
//! # 输出
//!
//! stdout 一行 JSON；stderr 人读摘要；**缺语料 ⇒ 明确说「跳过，不是跑过」**并返回非零。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_set::{evaluate, EvalBatch, Retrieved};
use apeireth_memory::hybrid_search::Bm25Config;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use serde_json::json;

/// 评测用 k（与既有闸门同）。
const K: usize = 10;
/// 取多少候选做排名分析（够覆盖 k 扫描的上限 100）。
const POOL: usize = 200;
/// k 扫描档位。
const K_SWEEP: [usize; 6] = [5, 10, 20, 30, 50, 100];
/// 近似重复判定阈值（词面 Jaccard）。
const DUP_THRESHOLD: f64 = 0.8;
/// 内部卷挖题预算（与既有闸门同口径）。
const MAX_CASES_PER_RECORD: usize = 6;
/// 分块窗口（字符）。0 = 不分块（整段）。
const CHUNK_WINDOWS: [usize; 5] = [0, 600, 300, 150, 80];

struct Case {
    id: String,
    query: String,
    gold: Vec<String>,
}

fn exam_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("APEIRETH_MIRACL_EXAM") {
        return PathBuf::from(dir);
    }
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("miracl-exam"),
        None => std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".apeireth")
            .join("miracl-exam"),
    }
}

/// 词面 token 集合（与生产分词同口径：`unicode_tokens`）。
fn token_set(text: &str) -> HashSet<String> {
    apeireth_memory::unicode_tokens(text).into_iter().collect()
}

fn jaccard(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
    if left.is_empty() && right.is_empty() {
        return 1.0;
    }
    let intersection = left.intersection(right).count() as f64;
    let union = left.union(right).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// top-k 里的**近似重复对数**（两两 Jaccard ≥ 阈值）。
fn near_duplicate_pairs(items: &[MemoryCandidate], threshold: f64) -> usize {
    let sets: Vec<HashSet<String>> = items.iter().map(|item| token_set(&item.content)).collect();
    let mut pairs = 0usize;
    for i in 0..sets.len() {
        for j in (i + 1)..sets.len() {
            if jaccard(&sets[i], &sets[j]) >= threshold {
                pairs += 1;
            }
        }
    }
    pairs
}

/// 一道题在某臂下的结果。
struct ArmOutcome {
    /// top-`POOL` 的 id 序列（按分数降序）。
    ranked: Vec<String>,
    /// top-`POOL` 里的近似重复对数。
    dup_pairs: usize,
}

/// 跑一遍检索：返回每题的 top-POOL id 序列。
fn retrieve_arm(
    source: &Bm25LexicalCandidateSource,
    cases: &[Case],
    pool: usize,
) -> Result<Vec<ArmOutcome>, Box<dyn std::error::Error>> {
    let mut outcomes = Vec::with_capacity(cases.len());
    for case in cases {
        let hits = source.candidates(&case.query, pool)?;
        outcomes.push(ArmOutcome {
            dup_pairs: near_duplicate_pairs(&hits[..hits.len().min(K)], DUP_THRESHOLD),
            ranked: hits.into_iter().map(|item| item.id).collect(),
        });
    }
    Ok(outcomes)
}

/// 用既有口径（`golden_set::evaluate`）算 recall@k / nDCG@k。
fn metrics_at(
    cases: &[Case],
    outcomes: &[ArmOutcome],
    k: usize,
) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let mut batch = EvalBatch::default();
    let golden_cases = build_cases(cases);
    for (case, outcome) in golden_cases.iter().zip(outcomes.iter()) {
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids: outcome.ranked.iter().take(k).cloned().collect(),
            fragments: Vec::new(),
        });
    }
    let metrics = evaluate(&batch, k)?;
    Ok((metrics.recall_at_k, metrics.ndcg_at_k))
}

/// 把自有的 `Case` 铸成既有口径的考卷题。
fn build_cases(cases: &[Case]) -> Vec<apeireth_memory::golden_set::GoldenCase> {
    cases
        .iter()
        .map(|case| apeireth_memory::golden_set::GoldenCase {
            id: case.id.clone(),
            query: case.query.clone(),
            expect_ids: case.gold.clone(),
            expect_fragments: Vec::new(),
            tags: Vec::new(),
        })
        .collect()
}

/// 把候选 id 映射回**父段落 id**（分块臂用；未分块时恒等映射）。
struct ParentMap {
    map: HashMap<String, String>,
}

impl ParentMap {
    fn parent_of(&self, id: &str) -> String {
        self.map.get(id).cloned().unwrap_or_else(|| id.to_string())
    }
}

/// 构造候选集：`window == 0` ⇒ 整段；否则按窗口切分（保留父段落映射）。
fn build_documents(
    passages: &[(String, String)],
    window: usize,
) -> (Vec<MemoryCandidate>, ParentMap) {
    let mut documents = Vec::new();
    let mut map = HashMap::new();
    for (id, text) in passages {
        if window == 0 {
            documents.push(candidate(id, text));
            continue;
        }
        let chars: Vec<char> = text.chars().collect();
        let mut index = 0usize;
        let mut part = 0usize;
        while index < chars.len() {
            let end = (index + window).min(chars.len());
            let piece: String = chars[index..end].iter().collect();
            let chunk_id = format!("{id}#{part}");
            map.insert(chunk_id.clone(), id.clone());
            documents.push(candidate(&chunk_id, &piece));
            index = end;
            part += 1;
        }
    }
    (documents, ParentMap { map })
}

fn candidate(id: &str, content: &str) -> MemoryCandidate {
    MemoryCandidate {
        id: id.to_string(),
        layer: "real_query".to_string(),
        scope: MemoryScope::Global,
        content: content.to_string(),
        score: 0.0,
        score_components: Default::default(),
        provenance: MemoryProvenance::default(),
    }
}

// ================= 机制 2b：内部卷回归检查（**会不会退步**） =================
//
// ⚠️ 口径：内部卷走**既有闸门的同一条路径**（`golden_run::run_with_extra_sources_and_config`
// + `InjectionMode::Production`）⇒ ⇒ 默认参数下的读数**必须**与闸门基线
// （redchamber 0.5305 / watershed 0.7319）**逐位相同** ⇒ 这样每个参数臂的 Δ 才可信。
fn internal_regression(
    param_grid: &[(f32, f32, bool)],
) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
    let mut rows = Vec::new();
    let dir = novels_dir();
    for (name, filename) in CORPORA {
        let path = dir.join(filename);
        if !path.exists() {
            eprintln!(
                "  · {name}：语料不在（{}）⇒ **跳过**（不是「跑过了」）",
                path.display()
            );
            continue;
        }
        let chapters = apeireth_memory::novel_corpus::load_chapters(&path)?;
        let records = apeireth_memory::novel_corpus::to_mine_records(&chapters);
        let config = apeireth_memory::golden_mining::MiningConfig {
            max_cases_per_record: MAX_CASES_PER_RECORD,
            ..apeireth_memory::golden_mining::MiningConfig::default()
        };
        let mined = apeireth_memory::golden_mining::mine(&records, config);
        let golden = apeireth_memory::golden_set::GoldenSet::new(
            format!("internal-{name}"),
            apeireth_memory::golden_run::mined_to_golden(mined),
        );
        // 可检索文本 = 正文（与闸门同口径：题面来源不进可检索文本 ⇒ 不泄漏答案）。
        let index: Vec<apeireth_memory::golden_run::IndexRecord> = records
            .iter()
            .map(|record| apeireth_memory::golden_run::IndexRecord {
                id: record.id.clone(),
                text: record.content.clone(),
            })
            .collect();
        // 信息性：内部卷在 k=20/30 的读数（**闸门口径固定 k=10** ⇒ 不参与判定）。
        for info_k in [20usize, 30] {
            let report = apeireth_memory::golden_run::run_with_extra_sources_and_config(
                &index,
                &golden,
                info_k,
                apeireth_memory::golden_run::InjectionMode::Production,
                &[],
                None,
            )?;
            eprintln!(
                "  · {name} · ℹ️ k={info_k}（信息性，非闸门口径）recall@{info_k} = {:.4}",
                report.metrics.recall_at_k
            );
        }
        let mut baseline: Option<f64> = None;
        let mut arm_rows = Vec::new();
        for (k1, b, strip) in param_grid.iter().copied() {
            let bmc = Bm25Config {
                k1,
                b,
                strip_cn_stopwords: strip,
            };
            let report = apeireth_memory::golden_run::run_with_extra_sources_and_config(
                &index,
                &golden,
                K,
                apeireth_memory::golden_run::InjectionMode::Production,
                &[],
                Some(bmc),
            )?;
            let recall = report.metrics.recall_at_k;
            let base = *baseline.get_or_insert(recall);
            eprintln!(
                "  · {name} · k1={k1} b={b} 去停用字={strip} ⇒ recall@{K} = {recall:.4} · Δ = {:+.4}",
                recall - base
            );
            arm_rows.push(json!({
                "corpus": name,
                "k1": k1, "b": b, "strip_cn_stopwords": strip,
                "recall_at_k": recall,
                "delta_vs_internal_baseline": recall - base,
            }));
        }
        rows.extend(arm_rows);
    }
    Ok(rows)
}

// ================= 机制 5：查询侧（伪相关反馈：用 top-1 文档的显著词回灌查询） =================
//
// # 为什么试它
// 缺口分解显示「召回到了但没进 top-10」占 13.5% ⇒ ⇒ 说明**词面召回面本身能碰到金标**，
// 只是排序靠后 ⇒ ⇒ 若能把**首命中文档的显著词**回灌查询，让更多金标文档的词进入查询，
// 就可能把它们从 10 名开外拉进来。
//
// # 口径
// * 第一遍 = 生产口径（不动）；
// * 扩展词 = 首命中文档里**不在原查询中**的、出现频次最高的若干词（词面口径）；
// * 回灌 = `原查询 + " " + 扩展词`（同一 BM25 源 ⇒ 不重建索引）。
fn query_expansion_arm(
    source: &Bm25LexicalCandidateSource,
    cases: &[Case],
    contents: &HashMap<String, String>,
    expand_top: usize,
    expand_terms: usize,
) -> Result<Vec<ArmOutcome>, Box<dyn std::error::Error>> {
    let mut outcomes = Vec::with_capacity(cases.len());
    for case in cases {
        let first = source.candidates(&case.query, expand_top)?;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for hit in &first {
            let Some(content) = contents.get(&hit.id) else {
                continue;
            };
            for token in apeireth_memory::unicode_tokens(content) {
                if token.chars().count() < 2 {
                    continue;
                }
                *counts.entry(token).or_insert(0) += 1;
            }
        }
        let original: HashSet<String> = apeireth_memory::unicode_tokens(&case.query)
            .into_iter()
            .collect();
        let mut terms: Vec<(String, usize)> = counts
            .into_iter()
            .filter(|(token, _)| !original.contains(token))
            .collect();
        terms.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
        let expansion: Vec<String> = terms
            .into_iter()
            .take(expand_terms)
            .map(|(token, _)| token)
            .collect();
        let expanded_query = if expansion.is_empty() {
            case.query.clone()
        } else {
            format!("{} {}", case.query, expansion.join(" "))
        };
        let hits = source.candidates(&expanded_query, POOL)?;
        outcomes.push(ArmOutcome {
            dup_pairs: near_duplicate_pairs(&hits[..hits.len().min(K)], DUP_THRESHOLD),
            ranked: hits.into_iter().map(|item| item.id).collect(),
        });
    }
    Ok(outcomes)
}

fn novels_dir() -> PathBuf {
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("novels"),
        None => std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".apeireth")
            .join("novels"),
    }
}

const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("· 失败：{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let root = exam_dir();
    let corpus_path = root.join("corpus.jsonl");
    let exam_path = root.join("exam.jsonl");
    if !corpus_path.exists() || !exam_path.exists() {
        eprintln!("· 考卷不在：{} ⇒ **没测到**（不是通过）", root.display());
        return Err("缺语料/考卷".into());
    }

    // ---------- 读语料 ----------
    let mut passages: Vec<(String, String)> = Vec::new();
    let mut pool_ids: HashSet<String> = HashSet::new();
    for line in std::fs::read_to_string(&corpus_path)?.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        let id = value["id"].as_str().unwrap_or_default().to_string();
        if id.is_empty() {
            continue;
        }
        let content = format!(
            "{}\n{}",
            value["title"].as_str().unwrap_or_default(),
            value["text"].as_str().unwrap_or_default()
        );
        pool_ids.insert(id.clone());
        passages.push((id, content));
    }

    // ---------- 读考卷 ----------
    let mut cases: Vec<Case> = Vec::new();
    for line in std::fs::read_to_string(&exam_path)?.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        let query = value["query"].as_str().unwrap_or_default().to_string();
        let gold: Vec<String> = value["gold"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if query.is_empty() || gold.is_empty() {
            continue;
        }
        cases.push(Case {
            id: value["id"].as_str().unwrap_or_default().to_string(),
            query,
            gold,
        });
    }
    eprintln!(
        "· 真实查询卷：语料 {} 段 · 题 {} 道 · k={K} · 取候选 {POOL}",
        passages.len(),
        cases.len()
    );

    // ================= 臂 0：生产口径（整段 + 默认参数） =================
    let (documents, _identity) = build_documents(&passages, 0);
    let source = Bm25LexicalCandidateSource::new(documents);
    let outcomes = retrieve_arm(&source, &cases, POOL)?;

    // ---------- 口径自证 ----------
    let (recall10, ndcg10) = metrics_at(&cases, &outcomes, K)?;
    eprintln!("· 生产口径（整段/默认参数）：recall@{K} = {recall10:.4} · nDCG@{K} = {ndcg10:.4}");
    eprintln!(
        "  ⚠️ **口径自证**：既有闸门基线 = 0.7246 ⇒ 本测例读数 − 基线 = {:+.4}",
        recall10 - 0.7246
    );

    // ================= 缺口分解 =================
    let mut a_not_in_pool = 0usize;
    let mut a_partial_gold = 0usize;
    let mut b_zero_hit = 0usize;
    let mut c_missed_topk = 0usize;
    let mut d_hit = 0usize;
    let mut sum_rank_when_missed: f64 = 0.0;
    let mut missed_rank_hist: HashMap<usize, usize> = HashMap::new();
    for (case, outcome) in cases.iter().zip(outcomes.iter()) {
        let gold_in_pool = case.gold.iter().filter(|id| pool_ids.contains(*id)).count();
        if gold_in_pool < case.gold.len() {
            a_partial_gold += 1;
        }
        let mut best_rank: Option<usize> = None;
        for (index, id) in outcome.ranked.iter().enumerate() {
            if case.gold.iter().any(|gold| gold == id) {
                best_rank = Some(index + 1);
                break;
            }
        }
        match best_rank {
            None => {
                if gold_in_pool == 0 {
                    a_not_in_pool += 1;
                } else {
                    b_zero_hit += 1;
                }
            }
            Some(rank) => {
                if rank <= K {
                    d_hit += 1;
                } else {
                    sum_rank_when_missed += rank as f64;
                    *missed_rank_hist.entry(rank).or_insert(0) += 1;
                    c_missed_topk += 1;
                }
            }
        }
    }
    let total = cases.len().max(1);
    let oracle = (d_hit + c_missed_topk) as f64 / total as f64;
    // ⭐ **天花板不是常数**：它随**候选预算**变化 ⇒ ⇒ 用哪个预算说「天花板」必须写清。
    let retrievable_within_50 = cases
        .iter()
        .zip(outcomes.iter())
        .filter(|(case, outcome)| {
            outcome.ranked[..outcome.ranked.len().min(50)]
                .iter()
                .any(|id| case.gold.iter().any(|gold| gold == id))
        })
        .count();
    let oracle_within_50 = retrievable_within_50 as f64 / total as f64;
    eprintln!("—— 缺口分解 ——");
    eprintln!(
        "  A 金标不在池里（池的边界，任何人救不了）：{a_not_in_pool} 题（{:.2}%）；其中**部分金标**缺失：{a_partial_gold} 题（{:.2}%）",
        a_not_in_pool as f64 * 100.0 / total as f64,
        a_partial_gold as f64 * 100.0 / total as f64
    );
    eprintln!(
        "  B 在池里但零命中（词面召回面）：      {b_zero_hit} 题（{:.2}%）",
        b_zero_hit as f64 * 100.0 / total as f64
    );
    eprintln!(
        "  C 召回到了但没进 top-{K}（排序面）：   {c_missed_topk} 题（{:.2}%）",
        c_missed_topk as f64 * 100.0 / total as f64
    );
    eprintln!(
        "  D 已进 top-{K}（达成）：               {d_hit} 题（{:.2}%）",
        d_hit as f64 * 100.0 / total as f64
    );
    eprintln!(
        "  ⇒ 池内天花板（oracle）= {oracle:.4}（候选预算 {POOL}）；距天花板 = {:+.4}（= B + C 的题占比）",
        oracle - recall10
    );
    eprintln!(
        "  ⇒ ⭐ **天花板随候选预算变化**：候选只取 50 时 oracle = {oracle_within_50:.4}（既有测例报的就是这个数）⇒ ⇒ 用 k=10 的候选预算说「天花板」会**低估**空间"
    );
    if c_missed_topk > 0 {
        eprintln!(
            "  ⇒ 没进 top-{K} 的 {c_missed_topk} 题，其**最好名次**：中位数 ≈ 第 {} 名（均值 {:.1}，最远第 {} 名）",
            median_rank(&missed_rank_hist),
            sum_rank_when_missed / c_missed_topk as f64,
            missed_rank_hist.keys().copied().max().unwrap_or(0)
        );
    }

    // ================= 机制 1：k 扫描 =================
    let mut k_rows = Vec::new();
    eprintln!("—— 机制：k（top-k）扫描 ——");
    for k in K_SWEEP {
        let (recall, ndcg) = metrics_at(&cases, &outcomes, k)?;
        eprintln!(
            "  k={k:<4} recall@{k} = {recall:.4} · nDCG@{k} = {ndcg:.4} · Δrecall = {:+.4}",
            recall - recall10
        );
        k_rows.push(json!({
            "k": k,
            "recall_at_k": recall,
            "ndcg_at_k": ndcg,
            "delta_recall_vs_k10": recall - recall10,
        }));
    }

    // ================= 机制 2：BM25 参数 =================
    let mut param_rows = Vec::new();
    eprintln!("—— 机制：BM25 参数（k1 / b / 中文停用字）——");
    let param_grid: [(f32, f32, bool); 9] = [
        (1.2, 0.75, true),
        (0.9, 0.75, true),
        (1.6, 0.75, true),
        (1.2, 0.30, true),
        (1.2, 1.00, true),
        (1.2, 0.75, false),
        (0.9, 0.30, true),
        (1.6, 1.00, true),
        (2.0, 0.75, true),
    ];
    for (k1, b, strip) in param_grid {
        let config = Bm25Config {
            k1,
            b,
            strip_cn_stopwords: strip,
        };
        let arm_source =
            Bm25LexicalCandidateSource::with_config(build_documents(&passages, 0).0, config);
        let arm_outcomes = retrieve_arm(&arm_source, &cases, POOL)?;
        let (recall, ndcg) = metrics_at(&cases, &arm_outcomes, K)?;
        eprintln!(
            "  k1={k1} b={b} 去停用字={strip} ⇒ recall@{K} = {recall:.4} · nDCG = {ndcg:.4} · Δrecall = {:+.4}",
            recall - recall10
        );
        param_rows.push(json!({
            "k1": k1, "b": b, "strip_cn_stopwords": strip,
            "recall_at_k": recall, "ndcg_at_k": ndcg,
            "delta_recall_vs_production": recall - recall10,
        }));
    }

    // ================= 机制 3：分块粒度 =================
    let mut chunk_rows = Vec::new();
    eprintln!("—— 机制：分块粒度（窗口字符数；0 = 整段）——");
    for window in CHUNK_WINDOWS {
        let (chunk_documents, parents) = build_documents(&passages, window);
        let chunk_source = Bm25LexicalCandidateSource::new(chunk_documents);
        let chunk_outcomes = retrieve_arm(&chunk_source, &cases, POOL)?;
        // 命中判定：chunk 的**父段落**在金标里就算命中（与整段口径同语义）。
        let mut batch = EvalBatch::default();
        let mut chunk_ranked = Vec::with_capacity(cases.len());
        let golden_cases = build_cases(&cases);
        for (case, outcome) in golden_cases.iter().zip(chunk_outcomes.iter()) {
            let parent_ids: Vec<String> = outcome
                .ranked
                .iter()
                .map(|id| parents.parent_of(id))
                .collect();
            chunk_ranked.push(parent_ids.clone());
            batch.cases.push(case.clone());
            batch.results.push(Retrieved {
                ids: parent_ids.iter().take(K).cloned().collect(),
                fragments: Vec::new(),
            });
        }
        let metrics = evaluate(&batch, K)?;
        eprintln!(
            "  窗口={:<5} 单元数={:<6} recall@{K} = {:.4} · nDCG = {:.4} · Δrecall = {:+.4}",
            if window == 0 {
                "整段".to_string()
            } else {
                window.to_string()
            },
            parents.map.len(),
            metrics.recall_at_k,
            metrics.ndcg_at_k,
            metrics.recall_at_k - recall10
        );
        chunk_rows.push(json!({
            "window_chars": window,
            "units": parents.map.len(),
            "recall_at_k": metrics.recall_at_k,
            "ndcg_at_k": metrics.ndcg_at_k,
            "delta_recall_vs_production": metrics.recall_at_k - recall10,
        }));
    }

    // ================= 机制 2b：内部卷回归（**会不会退步**） =================
    eprintln!("—— 机制：内部卷回归检查（闸门同口径）——");
    let internal_rows = internal_regression(&param_grid)?;

    // ================= 机制 5：查询侧（伪相关反馈） =================
    let contents: HashMap<String, String> = passages.iter().cloned().collect();
    let mut expansion_rows = Vec::new();
    eprintln!("—— 机制：查询侧扩展（用 top-N 文档的显著词回灌查询）——");
    for (expand_top, expand_terms) in [(1usize, 5usize), (1, 10), (3, 5), (3, 10)] {
        let arm_outcomes =
            query_expansion_arm(&source, &cases, &contents, expand_top, expand_terms)?;
        let (recall, ndcg) = metrics_at(&cases, &arm_outcomes, K)?;
        eprintln!(
            "  top-{expand_top} 文档 × {expand_terms} 词回灌 ⇒ recall@{K} = {recall:.4} · nDCG = {ndcg:.4} · Δrecall = {:+.4}",
            recall - recall10
        );
        expansion_rows.push(json!({
            "expand_top_docs": expand_top,
            "expand_terms": expand_terms,
            "recall_at_k": recall,
            "ndcg_at_k": ndcg,
            "delta_recall_vs_production": recall - recall10,
        }));
    }

    // ================= 机制 4：去重（top-10 里的近似重复） =================
    let dup_total: usize = outcomes.iter().map(|outcome| outcome.dup_pairs).sum();
    let dup_cases = outcomes
        .iter()
        .filter(|outcome| outcome.dup_pairs > 0)
        .count();
    eprintln!(
        "—— 机制：去重 —— top-{K} 内近似重复对（词面 Jaccard ≥ {DUP_THRESHOLD}）：共 {dup_total} 对，涉 {dup_cases} 题（{:.2}%）",
        dup_cases as f64 * 100.0 / total as f64
    );

    println!(
        "{}",
        json!({
            "corpus_segments": passages.len(),
            "cases": cases.len(),
            "k": K,
            "production": { "recall_at_k": recall10, "ndcg_at_k": ndcg10 },
            "gate_baseline": 0.7246,
            "caliber_self_check_delta": recall10 - 0.7246,
            "gap_decomposition": {
                "A_gold_not_in_pool": a_not_in_pool,
                "B_in_pool_zero_hit": b_zero_hit,
                "C_retrieved_but_outside_topk": c_missed_topk,
                "D_hit_topk": d_hit,
                "oracle_in_pool": oracle,
                "oracle_within_50_candidates": oracle_within_50,
                "A_partial_gold_missing": a_partial_gold,
                "headroom_to_oracle": oracle - recall10,
            },
            "k_sweep": k_rows,
            "bm25_params": param_rows,
            "internal_regression": internal_rows,
            "query_expansion": expansion_rows,
            "chunking": chunk_rows,
            "duplicates": { "pairs_in_top10": dup_total, "cases_affected": dup_cases },
        })
    );
    Ok(())
}

/// 名次直方图的中位数（1-based）。
fn median_rank(hist: &HashMap<usize, usize>) -> usize {
    let mut entries: Vec<(usize, usize)> = hist.iter().map(|(k, v)| (*k, *v)).collect();
    entries.sort();
    let total: usize = entries.iter().map(|(_, v)| *v).sum();
    let mut acc = 0usize;
    for (rank, count) in &entries {
        acc += *count;
        if acc * 2 >= total.max(1) {
            return *rank;
        }
    }
    entries.last().map(|(rank, _)| *rank).unwrap_or(0)
}
