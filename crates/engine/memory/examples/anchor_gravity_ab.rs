//! 「锚点引力重塑」（查询锚点命中率）的 **α 敏感性**离线测量。
//!
//! # 测的是什么机制
//!
//! 当前检索管线（融合 → 按路归一化 → 填充带 → 排序 → 残差式多样性重排）对**所有候选平权打分**。
//! 本测例量的是另一类机制：**先从查询里抽一组「锚点词」，再让「锚点命中更强者」得到加成**。
//!
//! ⭐ **只出数字，不接生产**：本文件是**唯一**新增物；`retrieval_pipeline.rs` /
//! `scope.rs` / `diversity.rs` / `first_pos.rs` 一行未动 ⇒ 测完即删也不会留下半接线。
//!
//! # 算法口径（全部自包含在本文件里）
//!
//! 1. **锚点词抽取**：复用生产分词口径 `normalize_for_index`（去疑问词 + 去中文停用字）
//!    + `tokenize`（ASCII 词 + CJK 单字 + CJK 二字组）；**只保留长度 ≥ 2 的 token**
//!    （⇒ CJK 二字组 + ASCII 词；剔除单字 CJK 噪声），并去重。
//! 2. **锚点权重 = 池内 IDF**：`idf(t) = ln((N+1) / (df(t)+0.5))`，`N` = 本题候选池大小，
//!    `df(t)` = 池内包含 `t` 的候选数。⇒ 越少候选命中的词权重越高（这才是「锚点」）。
//! 3. **锚点命中率**：`raw(c) = Σ_{t ∈ 锚点 ∩ c} idf(t)`（候选正文按**子串包含**判定）。
//! 4. **按题归一化到 (0,1]**：`raw(c) / max_{c'} raw(c')`（池内最大值 ⇒ 必须看到全部候选
//!    才能算 ⇒ 只能在并集建好后统一做）。池内最大值为 0 ⇒ 全体 0（不加成）。
//! 5. **加成**：`final(c) = base(c) + α · norm(c)`，α 网格 `[0.0, 0.05, 0.10, 0.20]`。
//!    `base(c)` = 该臂的**基线分**（见下），α = 0 即基线本身。
//!
//! # 两个「基线」臂（都量，因为两者在生产里都真实存在）
//!
//! * **管线臂（pipeline）**：生产检索路径 `HybridRetrievalPipeline::retrieve` 的输出分。
//! * **词面臂（bm25）**：单路 BM25 源自己的输出分（**与仓库既有质量闸门同口径**
//!   ⇒ α=0 这一行应当能复现既有闸门的读数 ⇒ 用来证明「本测例真的在测检索」）。
//!
//! # 两把尺子
//!
//! * **内部卷**：两本内部语料 + 与基线同口径的挖题（每记录 6 题，前 96 题子集）。
//! * **真实查询卷**：仓库既有真实查询考卷产物（`scripts/build_real_query_exam.py`）。
//!
//! ⚠️ **两把尺子结论可能相反**（已知先例：首命中位置加成在内部卷 −0.0252、在真实查询卷
//! +0.1227）⇒ ⇒ **不许因为某一卷好看就宣称有效**。
//!
//! # 额外口径：近似重复对数
//!
//! 每个 α 下统计 **top-K 内两两「近似重复」的对数**：同一份结果里任意两条的**词面
//! Jaccard ≥ 0.8** 记 1 对（阈值写死在这里）。相似度用仓库既有 `scoring.rs` **同口径**
//! 的**本地副本**（`scoring` 是 `pub(super)` ⇒ 这里自己实现一份并注明）。
//! ⇒ 目的：让「相关度 vs 去冗余」的**权衡**有数字，而不是只报一边的损失。
//!
//! # 输出
//!
//! stdout 一行 JSON；stderr 人读摘要。**缺语料 ⇒ 明确说「跳过，不是跑过」并返回非零**。
//!
//! # 中性表述
//!
//! 只描述**机制**（锚点引力重塑 / 查询锚点命中率 / 词面 Jaccard），不引用任何外部产品名。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::mined_to_golden;
use apeireth_memory::golden_set::{evaluate, EvalBatch, GoldenCase, GoldenSet, Retrieved};
use apeireth_memory::hybrid_search::{normalize_for_index, tokenize};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{
    unicode_tokens, Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_memory::MemoryRankingConfig;
use serde_json::json;

/// 与基线一致的 top-k。
const K: usize = 10;
/// 重排的候选池大小（重排只能在池内发生 ⇒ 池越大越能测到「被顶上来的那条」）。
const POOL: usize = 50;
/// 与基线一致的挖题预算（《紅樓夢》476 题 / 《水滸傳》248 题）。
const MAX_CASES_PER_RECORD: usize = 6;
/// 每题最多取前 N 题做扫描（`0` = 全量）。同一个子集横向比 α 仍然可比。
const CASE_SAMPLE: usize = 96;
/// 字符预算：本测例只量排序，给足够预算 ⇒ 不因预算截断丢条目。
const MAX_CHARS: usize = 4_000_000;
/// 近似重复判定阈值（词面 Jaccard）。
const DUP_JACCARD: f64 = 0.8;

/// α 网格（0.0 = 基线）。
const ALPHAS: [f64; 4] = [0.0, 0.05, 0.10, 0.20];

const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

// ---------------------------------------------------------------------------
// 词面相似度：**与 `retrieval_pipeline/scoring.rs` 同口径的本地副本**
// （`scoring` 是 `pub(super)` ⇒ example 里够不着 ⇒ 这里自己实现一份，算法逐字同构：
//  token 集合 = `unicode_tokens` 的去重集合；Jaccard = 交集 / 并集，两空集合记 1.0。）
// ---------------------------------------------------------------------------

fn local_token_set(text: &str) -> HashSet<String> {
    unicode_tokens(text).into_iter().collect()
}

fn local_jaccard(left: &HashSet<String>, right: &HashSet<String>) -> f64 {
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

// ---------------------------------------------------------------------------
// 锚点引力
// ---------------------------------------------------------------------------

/// 锚点词抽取（长度 ≥ 2 的 token，去重；空 ⇒ 该题无锚点）。
fn anchor_terms(query: &str) -> Vec<String> {
    let normalized = normalize_for_index(query);
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for token in tokenize(&normalized) {
        if token.chars().count() < 2 {
            continue;
        }
        if seen.insert(token.clone()) {
            out.push(token);
        }
    }
    out
}

/// 锚点命中分（按题归一化到 (0,1]）。
///
/// `pool` = 本题的候选池（查询 / id / 正文 / 基线分），返回与候选**逐位对齐**。
fn anchor_norm_scores(pool: &Pooled) -> Vec<f64> {
    let terms = anchor_terms(&pool.query);
    if terms.is_empty() || pool.ids.is_empty() {
        return vec![0.0; pool.ids.len()];
    }
    let n = pool.ids.len() as f64;
    let mut weights: Vec<f64> = Vec::with_capacity(terms.len());
    for term in &terms {
        let df = pool
            .contents
            .iter()
            .filter(|content| content.contains(term.as_str()))
            .count() as f64;
        weights.push(((n + 1.0) / (df + 0.5)).ln());
    }
    let raw: Vec<f64> = pool
        .contents
        .iter()
        .map(|content| {
            terms
                .iter()
                .zip(weights.iter())
                .filter(|(term, _)| content.contains(term.as_str()))
                .map(|(_, weight)| *weight)
                .sum()
        })
        .collect();
    let max = raw.iter().copied().fold(0.0_f64, f64::max);
    if max > 0.0 {
        raw.iter().map(|value| value / max).collect()
    } else {
        raw
    }
}

/// 一题的候选池（基线分来自该臂的检索路径）。
struct Pooled {
    query: String,
    ids: Vec<String>,
    contents: Vec<String>,
    scores: Vec<f64>,
}

/// 一臂的 α 扫描结果。
struct ArmRow {
    alpha: f64,
    recall: f64,
    ndcg: f64,
    dup_pairs: usize,
    dup_cases: usize,
}

/// 在给定候选池上跑一个 α，返回指标 + 近似重复对数。
fn measure_alpha(
    cases: &[GoldenCase],
    pools: &[Pooled],
    alpha: f64,
) -> Result<ArmRow, Box<dyn Error>> {
    let mut batch = EvalBatch::default();
    let mut dup_pairs = 0usize;
    let mut dup_cases = 0usize;
    for (case, pool) in cases.iter().zip(pools.iter()) {
        let norm = anchor_norm_scores(pool);
        let mut ranked: Vec<(usize, f64)> = pool
            .scores
            .iter()
            .copied()
            .zip(norm.iter().copied())
            .enumerate()
            .map(|(index, (base, bonus))| (index, base + alpha * bonus))
            .collect();
        ranked.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| pool.ids[left.0].cmp(&pool.ids[right.0]))
        });
        let top: Vec<usize> = ranked.iter().take(K).map(|(index, _)| *index).collect();
        let ids: Vec<String> = top.iter().map(|index| pool.ids[*index].clone()).collect();
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids: ids.clone(),
            fragments: Vec::new(),
        });
        // 近似重复对数（top-K 内两两 Jaccard ≥ 阈值）
        let sets: Vec<HashSet<String>> = top
            .iter()
            .map(|index| local_token_set(&pool.contents[*index]))
            .collect();
        let mut pairs = 0usize;
        for left in 0..sets.len() {
            for right in left + 1..sets.len() {
                if local_jaccard(&sets[left], &sets[right]) >= DUP_JACCARD {
                    pairs += 1;
                }
            }
        }
        dup_pairs += pairs;
        if pairs > 0 {
            dup_cases += 1;
        }
    }
    let metrics = evaluate(&batch, K)?;
    Ok(ArmRow {
        alpha,
        recall: metrics.recall_at_k,
        ndcg: metrics.ndcg_at_k,
        dup_pairs,
        dup_cases,
    })
}

/// 跑一整条臂（多个 α），返回行 + 基线行（α=0）。
fn measure_arm(cases: &[GoldenCase], pools: &[Pooled]) -> Result<Vec<ArmRow>, Box<dyn Error>> {
    let mut rows = Vec::new();
    for alpha in ALPHAS {
        let row = measure_alpha(cases, pools, alpha)?;
        eprintln!(
            "     α={:<5} recall@{K}={:.4} · nDCG@{K}={:.4} · 近似重复对={}（涉 {} 题）",
            row.alpha, row.recall, row.ndcg, row.dup_pairs, row.dup_cases
        );
        rows.push(row);
    }
    Ok(rows)
}

// ---------------------------------------------------------------------------
// 语料定位
// ---------------------------------------------------------------------------

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

fn exam_root() -> PathBuf {
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

fn read_lines(path: &std::path::Path) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(str::to_string)
        .collect())
}

fn memory_candidates(records: &[(String, String)], layer: &str) -> Vec<MemoryCandidate> {
    records
        .iter()
        .map(|(id, content)| MemoryCandidate {
            id: id.clone(),
            layer: layer.to_string(),
            scope: MemoryScope::Global,
            content: content.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance {
                source: layer.to_string(),
                source_session: None,
                source_trace: None,
                source_request: None,
            },
        })
        .collect()
}

fn main() -> ExitCode {
    let mut report = json!({
        "k": K,
        "pool": POOL,
        "case_sample": CASE_SAMPLE,
        "alphas": ALPHAS,
        "dup_jaccard_threshold": DUP_JACCARD,
        "internal": [],
        "real_query": [],
    });
    let mut missing: Vec<String> = Vec::new();

    // ---------------- 内部卷 ----------------
    let dir = novels_dir();
    let mut internal_rows = Vec::new();
    for (name, filename) in CORPORA {
        let path = dir.join(filename);
        if !path.exists() {
            eprintln!(
                "· {name}：语料不在（{}）⇒ **跳过**（不是「跑过了」）",
                path.display()
            );
            missing.push(format!("internal:{name}"));
            continue;
        }
        match internal_arm(name, &path) {
            Ok(entry) => internal_rows.push(entry),
            Err(error) => {
                eprintln!("· {name}：**跑失败**（{}）：{error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }
    report["internal"] = json!(internal_rows);

    // ---------------- 真实查询卷 ----------------
    let root = exam_root();
    match real_query_arm(&root) {
        Ok(entry) => report["real_query"] = entry,
        Err(error) => {
            eprintln!("· 真实查询卷：**跑失败**（{}）：{error}", root.display());
            return ExitCode::FAILURE;
        }
    }

    println!("{report}");
    if !missing.is_empty() {
        eprintln!(
            "· ⚠️ 有语料/考卷缺失（{}）⇒ 本轮**不完整**（缺的部分是「跳过」，不是「跑过」）",
            missing.join(", ")
        );
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn internal_arm(name: &str, path: &std::path::Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let config = MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = apeireth_memory::golden_mining::mine(&records, config);
    let golden = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));
    let cases: Vec<GoldenCase> = if CASE_SAMPLE == 0 {
        golden.cases.clone()
    } else {
        golden.cases.iter().take(CASE_SAMPLE).cloned().collect()
    };

    let payload: Vec<(String, String)> = records
        .iter()
        .map(|record| (record.id.clone(), record.content.clone()))
        .collect();
    let source = Bm25LexicalCandidateSource::new(memory_candidates(&payload, "golden"));
    // 管线臂：生产检索路径（多样性 λ 与首命中位置 α 都取「关」⇒ 逐字等于默认生产行为）。
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig {
        diversity_lambda: 0.0,
        ..MemoryRankingConfig::default()
    });

    let mut pipeline_pools: Vec<Pooled> = Vec::new();
    let mut bm25_pools: Vec<Pooled> = Vec::new();
    for case in &cases {
        let hits = pipeline.retrieve(
            &case.query,
            &[MemoryScope::Global],
            &[&source],
            POOL,
            MAX_CHARS,
        )?;
        pipeline_pools.push(Pooled {
            query: case.query.clone(),
            ids: hits.iter().map(|item| item.id.clone()).collect(),
            contents: hits.iter().map(|item| item.content.clone()).collect(),
            scores: hits.iter().map(|item| item.score).collect(),
        });
        let raw = source.candidates(&case.query, POOL)?;
        bm25_pools.push(Pooled {
            query: case.query.clone(),
            ids: raw.iter().map(|item| item.id.clone()).collect(),
            contents: raw.iter().map(|item| item.content.clone()).collect(),
            scores: raw.iter().map(|item| item.score).collect(),
        });
    }

    eprintln!("· {name}：题 {} 道 · 池 {POOL} · k={K}", cases.len());
    eprintln!("   -- 管线臂 --");
    let pipeline_rows = measure_arm(&cases, &pipeline_pools)?;
    eprintln!("   -- 词面臂（单路 BM25，与既有质量闸门同口径）--");
    let bm25_rows = measure_arm(&cases, &bm25_pools)?;

    Ok(json!({
        "name": name,
        "cases": cases.len(),
        "pipeline": rows_json(&pipeline_rows),
        "bm25": rows_json(&bm25_rows),
    }))
}

fn real_query_arm(root: &std::path::Path) -> Result<serde_json::Value, Box<dyn Error>> {
    let corpus_path = root.join("corpus.jsonl");
    let exam_path = root.join("exam.jsonl");
    if !corpus_path.exists() || !exam_path.exists() {
        eprintln!(
            "· 真实查询卷：考卷不在（{}）⇒ **跳过**（不是「跑过了」）",
            root.display()
        );
        return Err(format!("考卷缺失：{}", root.display()).into());
    }
    let mut payload: Vec<(String, String)> = Vec::new();
    for line in read_lines(&corpus_path)? {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&line)?;
        let id = value["id"].as_str().unwrap_or_default().to_string();
        if id.is_empty() {
            continue;
        }
        let joined = format!(
            "{}\n{}",
            value["title"].as_str().unwrap_or_default(),
            value["text"].as_str().unwrap_or_default()
        );
        payload.push((id, joined));
    }
    let mut cases: Vec<GoldenCase> = Vec::new();
    for line in read_lines(&exam_path)? {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&line)?;
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
        cases.push(GoldenCase {
            id: value["id"].as_str().unwrap_or_default().to_string(),
            query,
            expect_ids: gold,
            expect_fragments: Vec::new(),
            tags: Vec::new(),
        });
    }
    if cases.is_empty() {
        return Err("真实查询卷题集为空 ⇒ 没测到".into());
    }

    let source = Bm25LexicalCandidateSource::new(memory_candidates(&payload, "realquery"));
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig {
        diversity_lambda: 0.0,
        ..MemoryRankingConfig::default()
    });
    let mut pipeline_pools: Vec<Pooled> = Vec::new();
    let mut bm25_pools: Vec<Pooled> = Vec::new();
    for case in &cases {
        let hits = pipeline.retrieve(
            &case.query,
            &[MemoryScope::Global],
            &[&source],
            POOL,
            MAX_CHARS,
        )?;
        pipeline_pools.push(Pooled {
            query: case.query.clone(),
            ids: hits.iter().map(|item| item.id.clone()).collect(),
            contents: hits.iter().map(|item| item.content.clone()).collect(),
            scores: hits.iter().map(|item| item.score).collect(),
        });
        let raw = source.candidates(&case.query, POOL)?;
        bm25_pools.push(Pooled {
            query: case.query.clone(),
            ids: raw.iter().map(|item| item.id.clone()).collect(),
            contents: raw.iter().map(|item| item.content.clone()).collect(),
            scores: raw.iter().map(|item| item.score).collect(),
        });
    }

    eprintln!(
        "· 真实查询卷：语料 {} 段 · 题 {} 道 · 池 {POOL} · k={K}",
        payload.len(),
        cases.len()
    );
    eprintln!("   -- 管线臂 --");
    let pipeline_rows = measure_arm(&cases, &pipeline_pools)?;
    eprintln!("   -- 词面臂（单路 BM25，与既有质量闸门同口径）--");
    let bm25_rows = measure_arm(&cases, &bm25_pools)?;

    Ok(json!({
        "name": "real-query",
        "corpus_segments": payload.len(),
        "cases": cases.len(),
        "pipeline": rows_json(&pipeline_rows),
        "bm25": rows_json(&bm25_rows),
    }))
}

/// 把 α 扫描行转成 JSON（带相对基线的 Δ）。
fn rows_json(rows: &[ArmRow]) -> Vec<serde_json::Value> {
    let base = rows.first();
    rows.iter()
        .map(|row| {
            let (delta_recall, delta_ndcg, pct_recall) = match base {
                Some(base) => (
                    row.recall - base.recall,
                    row.ndcg - base.ndcg,
                    if base.recall > 0.0 {
                        (row.recall - base.recall) / base.recall * 100.0
                    } else {
                        0.0
                    },
                ),
                None => (0.0, 0.0, 0.0),
            };
            json!({
                "alpha": row.alpha,
                "recall_at_k": row.recall,
                "ndcg_at_k": row.ndcg,
                "delta_recall_vs_alpha0": delta_recall,
                "delta_ndcg_vs_alpha0": delta_ndcg,
                "pct_recall_vs_alpha0": pct_recall,
                "dup_pairs": row.dup_pairs,
                "dup_cases": row.dup_cases,
            })
        })
        .collect()
}
