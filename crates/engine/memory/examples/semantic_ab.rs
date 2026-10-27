//! `semantic_ab` — **语义/向量召回**对照实验（现役 BM25 / 字符级余弦 / 混合）。
//!
//! # 这个例子回答什么
//!
//! W-20R 实测定稿（`docs/status/recall-root-cause-2026-10-27.md`）：
//! 内部卷《紅樓夢》`recall@10 = 0.2983` 的瓶颈**不是**切分粒度，而是「**查询的判别力**」
//! —— 题面**只有主体词一个信号**，而主体在 **73%** 的记录里都出现。
//! ⇒ 下一批该攻「**题面增信号**」**或**「**语义/向量召回**」。
//!
//! 本例子把「语义/向量召回」这条线**真的跑一遍**（不是推算）：
//! **同 k、同语料、同卷子** ⇒ 三路对照 ⇒ 数字可复核。
//!
//! # 三路（口径钉死）
//!
//! 1. **现役** = 生产 BM25（`golden_run::run` ⇒ 与生产检索**同口径**）；
//! 2. **字符级语义** = [`OfflineEmbedder`]（字符 n-gram 哈希投影）⇒ 余弦排序；
//! 3. **混合** = `0.5 × BM25 + 0.5 × 余弦`（权重**固定**，**不许**为凑分数调）。
//!
//! # ⚠️ 诚实口径（不许读成「语义召回已验证」）
//!
//! * 第 2 路**不是语义模型** ⇒ 它测的是「**词面饱和时，字符级信号还有没有用**」
//!   ⇒ **不能**据此断言「语义召回行不行」（那需要真模型 + 网络）。
//! * 「维度阶梯」与「IDF 加权」两行是**诊断行**，**不是**头条口径
//!   ⇒ 它们只用于定位「**是表示弱，还是缺语料统计**」。
//! * 头条口径的维度 = **256**（契约默认，**事前定死**）⇒ 不许看完分数再换。
//!
//! # 语料在仓外
//!
//! 语料在 `$APEIRETH_DATA_DIR/novels/`（默认 `~/.apeireth/novels/`）⇒
//! **某本语料缺失 ⇒ 跳过它并如实说明**（stderr），**不许**假装跑过；
//! 一本都没有 ⇒ **非零退出**。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能跑出真实对照（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 **0 调用点**（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✔（部分）：已在两本**真实小说**上跑过（2026-10-27），
//!   **未在第二台机器复现** ⇒ 不宣称跨机器可复现。

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::golden_diag::{diagnose, summarize};
use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::{self, mined_to_golden, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{evaluate, EvalBatch, GoldenCase, GoldenSet, Retrieved};
use apeireth_memory::memory_injection::EVIDENCE_MAX_CHARS;
use apeireth_memory::novel_corpus;
use apeireth_memory::offline_embed::{OfflineEmbedder, DEFAULT_DIMENSION};
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_orchestration::{cosine, Embedder};

/// 评测的 top-k（**必须与基线文件的 `k` 一致**）。
const K: usize = 10;

/// 挖题预算（每条记录最多几道题）—— 与基线同口径。
const MAX_CASES_PER_RECORD: usize = 6;

/// 头条口径的向量维度（**事前定死** = 契约默认 256 ⇒ 不许看完分数再换）。
const HEADLINE_DIMENSION: usize = DEFAULT_DIMENSION;

/// 维度阶梯（**诊断行**：定位「是不是桶碰撞吃掉了信号」）。
const DIMENSION_LADDER: [usize; 4] = [64, 256, 1024, 4096];

/// 混合权重（**固定 0.5 / 0.5** ⇒ 不许为凑分数调）。
const BM25_WEIGHT: f64 = 0.5;
const COSINE_WEIGHT: f64 = 0.5;

/// 语料清单：`（名字, 仓外文件名）`（与 `golden_baseline_report.rs` 同一张表）。
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

/// 一条检索路径的评测结果。
struct PathOutcome {
    /// 路由名（人话）。
    label: String,
    /// `recall@K`。
    recall: f64,
    /// `nDCG@K`。
    ndcg: f64,
    /// 命中（目标在 K 以内）的题数。
    hits: usize,
    /// 排在 K 外（进了候选池但名次靠后）的题数。
    ranked_out: usize,
    /// 没进候选池的题数。
    not_in_pool: usize,
    /// 无期望（题坏了）的题数。
    no_expectation: usize,
}

fn main() -> ExitCode {
    let dir = novels_dir();
    let mut loaded = 0usize;

    for (name, filename) in CORPORA {
        let path = dir.join(filename);
        if !path.exists() {
            eprintln!(
                "· {name}：语料不在（{}）⇒ **跳过**（不是「跑过了」）",
                path.display()
            );
            continue;
        }
        match run_one(name, &path) {
            Ok(()) => loaded += 1,
            Err(error) => {
                eprintln!("· {name}：**跑失败**（{}）：{error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    if loaded == 0 {
        eprintln!(
            "一份语料都没跑到（目录 {}）⇒ **不能出结论**（不许把「没跑」当「没问题」）",
            dir.display()
        );
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

/// 语料目录（`$APEIRETH_DATA_DIR/novels/`，默认 `~/.apeireth/novels/`）。
fn novels_dir() -> PathBuf {
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("novels"),
        None => {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            home.join(".apeireth").join("novels")
        }
    }
}

/// 跑一份语料 ⇒ 三路对照报告。
fn run_one(name: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let config = MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = golden_mining::mine(&records, config);
    let cases: Vec<GoldenCase> = mined_to_golden(mined);
    if cases.is_empty() {
        eprintln!("· {name}：挖出 0 道题 ⇒ 跳过（如实：不是「跑过了」）");
        return Ok(());
    }
    let golden = GoldenSet::new(format!("semantic-ab-{name}"), cases.clone());
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();
    let texts: BTreeMap<String, String> = index
        .iter()
        .map(|record| (record.id.clone(), record.text.clone()))
        .collect();

    // ── 现役（生产 BM25）**官方口径**：直接用 `golden_run::run` ⇒ 与生产检索同源 ──
    let official = golden_run::run(&index, &golden, K, InjectionMode::Production)
        .map_err(|error| -> Box<dyn Error> { error.to_string().into() })?;

    // ── 三路共用「同语料、同卷子、同 k」──
    let source = build_bm25_source(&index);
    let embedder = OfflineEmbedder::default_embedder();
    let doc_vectors: Vec<Vec<f32>> = index
        .iter()
        .map(|record| embedder.embed(&record.text))
        .collect();

    let mut bm25_rankings: Vec<Vec<(String, f64)>> = Vec::with_capacity(cases.len());
    let mut cosine_rankings: Vec<Vec<(String, f64)>> = Vec::with_capacity(cases.len());
    let mut fused_rankings: Vec<Vec<(String, f64)>> = Vec::with_capacity(cases.len());
    for case in &cases {
        let bm25 = bm25_ranking(&source, index.len(), &case.query)
            .map_err(|error| -> Box<dyn Error> { error.to_string().into() })?;
        let query_vector = embedder.embed(&case.query);
        let cosine_scores = cosine_ranking(&index, &doc_vectors, &query_vector);
        let fused = fused_ranking(&bm25, &cosine_scores, BM25_WEIGHT, COSINE_WEIGHT);
        bm25_rankings.push(bm25);
        cosine_rankings.push(cosine_scores);
        fused_rankings.push(fused);
    }

    let bm25_outcome = evaluate_ranking("现役 BM25（生产口径）", &cases, &bm25_rankings, &texts, K);
    let cosine_outcome = evaluate_ranking(
        "字符级余弦（OfflineEmbedder）",
        &cases,
        &cosine_rankings,
        &texts,
        K,
    );
    let fused_outcome = evaluate_ranking(
        "混合（0.5 BM25 + 0.5 余弦）",
        &cases,
        &fused_rankings,
        &texts,
        K,
    );

    // ── 诊断行 1：IDF 加权（定位「是表示弱，还是缺语料统计」）──
    let idf = bucket_idf(&doc_vectors, HEADLINE_DIMENSION);
    let mut idf_rankings: Vec<Vec<(String, f64)>> = Vec::with_capacity(cases.len());
    for case in &cases {
        let query_vector = idf_weighted(&embedder.embed(&case.query), &idf);
        let mut scored: Vec<(String, f64)> = index
            .iter()
            .zip(doc_vectors.iter())
            .map(|(record, vector)| {
                let score = cosine(&query_vector, &idf_weighted(vector, &idf));
                (record.id.clone(), f64::from(score))
            })
            .collect();
        scored.retain(|(_, score)| *score > 0.0);
        sort_scored(&mut scored);
        idf_rankings.push(scored);
    }
    let idf_outcome = evaluate_ranking(
        "（诊断）字符级余弦 + 桶级 IDF",
        &cases,
        &idf_rankings,
        &texts,
        K,
    );

    // ── 报告 ──
    println!("════════ {name} ════════");
    println!(
        "语料 {} 条（回）· 题 {} 道 · k={K} · 头条维度 {HEADLINE_DIMENSION} · 混合权重 {BM25_WEIGHT}/{COSINE_WEIGHT}",
        index.len(),
        cases.len()
    );
    println!();

    println!("── 三路对照（**头条口径**）──");
    println!(
        "  {:<28}  {:>10}  {:>10}  {:>10}  {:>28}",
        "路由", "recall@10", "nDCG@10", "Δrecall", "缺口分类（命中/k外/没进池/无期望）"
    );
    print_row(&bm25_outcome, None);
    print_row(&cosine_outcome, Some(bm25_outcome.recall));
    print_row(&fused_outcome, Some(bm25_outcome.recall));
    println!();

    // ── 一致性自检：自算 BM25 必须与生产口径官方数字**逐字相等** ──
    println!("── 一致性自检 ──");
    let official_recall = official.metrics.recall_at_k;
    let official_ndcg = official.metrics.ndcg_at_k;
    let recall_match = (bm25_outcome.recall - official_recall).abs() < 1e-9;
    let ndcg_match = (bm25_outcome.ndcg - official_ndcg).abs() < 1e-9;
    println!(
        "  自算 BM25 recall@10 = {:.4} · `golden_run::run` 官方值 = {:.4} ⇒ {}",
        bm25_outcome.recall,
        official_recall,
        if recall_match {
            "逐字相等 ✔"
        } else {
            "**不一致** ✘"
        }
    );
    println!(
        "  自算 BM25 nDCG@10  = {:.4} · `golden_run::run` 官方值 = {:.4} ⇒ {}",
        bm25_outcome.ndcg,
        official_ndcg,
        if ndcg_match {
            "逐字相等 ✔"
        } else {
            "**不一致** ✘"
        }
    );
    println!(
        "  `golden_run::summarize` 摘要：{}",
        golden_run::summarize(&official).replace('\n', " / ")
    );
    println!();

    // ── 诊断行 ──
    println!("── 诊断行（**不是**头条口径）──");
    print_row(&idf_outcome, Some(bm25_outcome.recall));
    println!("  维度阶梯（字符级余弦这一路的 recall@10）：");
    for dimension in DIMENSION_LADDER {
        let outcome = cosine_only_outcome(&index, &cases, &texts, dimension, K)?;
        let marker = if dimension == HEADLINE_DIMENSION {
            "  ← 头条口径"
        } else {
            ""
        };
        println!(
            "    维度 {:>5}：recall@10 = {:.4} · nDCG@10 = {:.4}{}",
            dimension, outcome.recall, outcome.ndcg, marker
        );
    }
    println!();

    // ── 桶饱和度（**解释**上面两行为什么是这个数）──
    print_saturation(&index, &doc_vectors, HEADLINE_DIMENSION);
    println!();

    // ── 结论（**如实**，不许粉饰）──
    println!("── 结论（如实生成）──");
    println!(
        "  字符级余弦 vs 现役 BM25：Δrecall@10 = {:+.4} ⇒ {}",
        cosine_outcome.recall - bm25_outcome.recall,
        verdict(cosine_outcome.recall - bm25_outcome.recall)
    );
    println!(
        "  混合     vs 现役 BM25：Δrecall@10 = {:+.4} ⇒ {}",
        fused_outcome.recall - bm25_outcome.recall,
        verdict(fused_outcome.recall - bm25_outcome.recall)
    );
    println!(
        "  （诊断）字符级 + IDF vs 现役 BM25：Δrecall@10 = {:+.4} ⇒ {}",
        idf_outcome.recall - bm25_outcome.recall,
        verdict(idf_outcome.recall - bm25_outcome.recall)
    );
    println!(
        "  ⚠️ 字符级余弦**不是语义模型** ⇒ 本表只能回答「**词面饱和时字符级信号还有没有用**」"
    );
    println!("     ⇒ **不能**回答「语义召回行不行」（那需要真模型 + 网络 ⇒ 当前环境离线跑不了）");
    Ok(())
}

/// 打印一行对照表。
fn print_row(outcome: &PathOutcome, baseline: Option<f64>) {
    let delta = match baseline {
        Some(base) => format!("{:+.4}", outcome.recall - base),
        None => "—".to_string(),
    };
    println!(
        "  {:<28}  {:>10.4}  {:>10.4}  {:>10}  {:>28}",
        outcome.label,
        outcome.recall,
        outcome.ndcg,
        delta,
        format!(
            "{} / {} / {} / {}",
            outcome.hits, outcome.ranked_out, outcome.not_in_pool, outcome.no_expectation
        )
    );
}

/// 结论措辞（**如实**）。
fn verdict(delta: f64) -> &'static str {
    if delta > 1e-9 {
        "**提上去了**"
    } else if delta < -1e-9 {
        "**没有提上去**（反而降了）"
    } else {
        "**没有变化**"
    }
}

/// 建 BM25 索引（**逐字同口径** `golden_run::build_source` / `golden_diag::build_source`）。
fn build_bm25_source(records: &[IndexRecord]) -> Bm25LexicalCandidateSource {
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

/// BM25 候选（**分数已按当次查询的最大值归一化到 [0,1]**；源已按分数降序排好）。
fn bm25_ranking(
    source: &Bm25LexicalCandidateSource,
    limit: usize,
    query: &str,
) -> Result<Vec<(String, f64)>, apeireth_memory::MemoryError> {
    let candidates = source.candidates(query, limit)?;
    Ok(candidates
        .into_iter()
        .map(|candidate| (candidate.id, candidate.score))
        .collect())
}

/// 余弦候选（分数在 [0,1]；`0` 分**不算**候选 ⇒ 与 BM25 的零分过滤同口径）。
fn cosine_ranking(
    index: &[IndexRecord],
    doc_vectors: &[Vec<f32>],
    query_vector: &[f32],
) -> Vec<(String, f64)> {
    let mut scored: Vec<(String, f64)> = index
        .iter()
        .zip(doc_vectors.iter())
        .map(|(record, vector)| {
            let score = cosine(query_vector, vector);
            (record.id.clone(), f64::from(score))
        })
        .collect();
    scored.retain(|(_, score)| *score > 0.0);
    sort_scored(&mut scored);
    scored
}

/// 融合候选（BM25 与余弦的**线性加权**；两路分数都已在 [0,1] ⇒ 权重可直接相加）。
fn fused_ranking(
    bm25: &[(String, f64)],
    cosine_scores: &[(String, f64)],
    bm25_weight: f64,
    cosine_weight: f64,
) -> Vec<(String, f64)> {
    let cosine_map: BTreeMap<&str, f64> = cosine_scores
        .iter()
        .map(|(id, score)| (id.as_str(), *score))
        .collect();
    let mut fused: Vec<(String, f64)> = bm25
        .iter()
        .map(|(id, bm25_score)| {
            let cosine_score = cosine_map.get(id.as_str()).copied().unwrap_or(0.0);
            (
                id.clone(),
                bm25_weight * bm25_score + cosine_weight * cosine_score,
            )
        })
        .collect();
    fused.retain(|(_, score)| *score > 0.0);
    sort_scored(&mut fused);
    fused
}

/// 排序：**分数降序** ⇒ 同分按 **id 升序**（**确定性** ⇒ 两次跑逐字相等）。
fn sort_scored(scored: &mut [(String, f64)]) {
    scored.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.0.cmp(&right.0))
    });
}

/// 把「每题的有序候选」折算成 `recall@K` / `nDCG@K` + 缺口分类。
fn evaluate_ranking(
    label: &str,
    cases: &[GoldenCase],
    rankings: &[Vec<(String, f64)>],
    texts: &BTreeMap<String, String>,
    k: usize,
) -> PathOutcome {
    let mut batch = EvalBatch::default();
    let mut pools: Vec<Vec<Retrieved>> = Vec::with_capacity(cases.len());
    for (index, case) in cases.iter().enumerate() {
        let ranking = rankings.get(index).cloned().unwrap_or_default();
        let ids: Vec<String> = ranking.iter().map(|(id, _)| id.clone()).collect();
        let top: Vec<String> = ids.iter().take(k).cloned().collect();
        // 注入片段：与生产同口径（正文**前 120 字**）⇒ 证据族指标可比。
        let fragments: Vec<String> = top
            .iter()
            .filter_map(|id| texts.get(id))
            .map(|text| text.chars().take(EVIDENCE_MAX_CHARS).collect())
            .collect();
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids: top,
            fragments,
        });
        // 候选池 = **完整有序结果**（不按 k 截断）⇒ 「没进池」与「排在 k 外」可区分。
        pools.push(vec![Retrieved {
            ids,
            fragments: Vec::new(),
        }]);
    }
    let metrics = evaluate(&batch, k).expect("cases 与 results 必然等长");
    let diags = diagnose(cases, &pools, k);
    let summary = summarize(&diags, k);
    PathOutcome {
        label: label.to_string(),
        recall: metrics.recall_at_k,
        ndcg: metrics.ndcg_at_k,
        hits: summary.hits,
        ranked_out: summary.ranked_out,
        not_in_pool: summary.not_in_pool,
        no_expectation: summary.no_expectation,
    }
}

/// 只跑「字符级余弦」一路（**维度阶梯**用）。
fn cosine_only_outcome(
    index: &[IndexRecord],
    cases: &[GoldenCase],
    texts: &BTreeMap<String, String>,
    dimension: usize,
    k: usize,
) -> Result<PathOutcome, Box<dyn Error>> {
    let embedder =
        OfflineEmbedder::new(dimension).map_err(|error| -> Box<dyn Error> { error.into() })?;
    let doc_vectors: Vec<Vec<f32>> = index
        .iter()
        .map(|record| embedder.embed(&record.text))
        .collect();
    let mut rankings: Vec<Vec<(String, f64)>> = Vec::with_capacity(cases.len());
    for case in cases {
        let query_vector = embedder.embed(&case.query);
        rankings.push(cosine_ranking(index, &doc_vectors, &query_vector));
    }
    Ok(evaluate_ranking(
        "字符级余弦（指定维度）",
        cases,
        &rankings,
        texts,
        k,
    ))
}

/// 打印**桶饱和度**诊断（解释「字符级余弦为什么这么弱」）。
///
/// # 为什么要有这个
///
/// 固定维度的 hashing vectorizer **必然碰撞** ⇒ 桶越少 ⇒ 撞得越狠
/// ⇒ 撞到**每篇文档都点亮几乎所有桶** ⇒ 桶与桶之间**不再有区分度**
/// ⇒ 那正是 W-20R 说的「**词面饱和**」在**向量侧**的同一个现象。
///
/// # 怎么读
///
/// * 「每篇点亮的桶占比」接近 100% ⇒ 桶空间已饱和 ⇒ 向量**几乎不携带**区分信息；
/// * 此时 **IDF 也救不了**（每个桶的文档频次都接近语料大小 ⇒ IDF 几乎是常数）
///   ⇒ 这解释了「加了 IDF 与不加 IDF 数字一样」的现象。
fn print_saturation(index: &[IndexRecord], doc_vectors: &[Vec<f32>], dimension: usize) {
    let lit_per_doc: Vec<usize> = doc_vectors
        .iter()
        .map(|vector| vector.iter().filter(|value| **value > 0.0).count())
        .collect();
    let mean_lit = if lit_per_doc.is_empty() {
        0.0
    } else {
        lit_per_doc.iter().sum::<usize>() as f64 / lit_per_doc.len() as f64
    };
    let mut df = vec![0usize; dimension];
    for vector in doc_vectors {
        for (index, value) in vector.iter().enumerate() {
            if *value > 0.0 {
                df[index] += 1;
            }
        }
    }
    let used_buckets = df.iter().filter(|count| **count > 0).count();
    let mean_df = if used_buckets == 0 {
        0.0
    } else {
        df.iter().sum::<usize>() as f64 / used_buckets as f64
    };
    println!("── 桶饱和度（为什么字符级这一路弱）──");
    println!(
        "  维度 {dimension} · 语料 {} 篇 · 每篇平均点亮桶数 {mean_lit:.1}（占维度 {:.1}%）",
        index.len(),
        mean_lit / dimension as f64 * 100.0
    );
    println!(
        "  非零桶 {used_buckets}/{dimension} · 它们的文档频次均值 {mean_df:.1}（占语料 {:.1}%）",
        mean_df / index.len().max(1) as f64 * 100.0
    );
    if mean_lit / dimension as f64 > 0.9 {
        println!("  ⇒ **桶空间已被词面饱和**（几乎每篇都点亮几乎所有桶）⇒ 桶失去区分度");
        println!("  ⇒ 此时**桶级 IDF 几乎是常数** ⇒ 「加不加 IDF 数字一样」是**必然**，不是 bug");
    }
}

/// 桶级 **IDF**（诊断用）。
///
/// # 为什么能从向量反推文档频次
///
/// [`embed_text`] 里「某维 > 0」⇔「该桶在这篇文本里出现过」
/// ⇒ 文档频次（df）**可以直接从向量数出来**
/// ⇒ 不需要给 embedder 加任何新接口。
///
/// # 为什么这条诊断有意义
///
/// 纯函数 embedder **拿不到语料统计** ⇒ 题面里的**模板套话**
/// （「我们当时记了什么」）与**主体词**权重**一样**
/// ⇒ 判别力被稀释 ⇒ 若加上 IDF 就涨分 ⇒ 说明**缺的是语料统计，不是表示本身**。
fn bucket_idf(doc_vectors: &[Vec<f32>], dimension: usize) -> Vec<f64> {
    let total = doc_vectors.len() as f64;
    let mut df = vec![0usize; dimension];
    for vector in doc_vectors {
        for (index, value) in vector.iter().enumerate() {
            if *value > 0.0 {
                df[index] += 1;
            }
        }
    }
    df.iter()
        .map(|count| ((total + 1.0) / (*count as f64 + 1.0)).ln() + 1.0)
        .collect()
}

/// 给向量乘 IDF 并重新 L2 归一。
fn idf_weighted(vector: &[f32], idf: &[f64]) -> Vec<f32> {
    let mut weighted: Vec<f32> = vector
        .iter()
        .zip(idf.iter())
        .map(|(weight, idf_value)| weight * *idf_value as f32)
        .collect();
    let norm = weighted
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if norm > 0.0 {
        for value in weighted.iter_mut() {
            *value /= norm;
        }
    }
    weighted
}
