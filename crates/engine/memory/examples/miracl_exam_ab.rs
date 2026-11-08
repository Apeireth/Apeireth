//! **真实查询考卷（MIRACL 中文）**（2026-11-07）—— 用**真实用户查询 + 官方相关性标注**量检索。
//!
//! # 为什么需要它
//! 本项目两套考卷都是**模板题**（`golden_mining` 5 种模板 / `golden_fair` 单一模板）
//! ⇒ 任何「依赖题面形态」的结论都**不能外推**到真实用户查询。
//! ⇒ 本考卷用 **MIRACL 中文 dev** 的 393 条真实母语查询 + 官方标注。
//!
//! # 口径（报告必须带上）
//! * **采样池**：全部 3786 条**被标注**段落 + 确定性采样（`sha1(docid)`）的 19076 条未标注段落
//!   ⇒ **池子远小于真实全库（5.9M）** ⇒ **召回偏乐观**
//!   ⇒ **绝对数字不可与 MIRACL 官方榜对比**；**臂间相对比较才是用途**。
//! * **相关性**：`rel == 1` 算相关；多相关文档时**任一命中**即算命中。
//!
//! # ⭐⭐⚠️ **闸门口径 vs 二值口径（2026-11-08 修正）**
//! 本例的 `first_pos` α 扫描**同时报两种口径**，且**各配一条同口径基线**：
//!
//! | 口径 | 定义 | 与闸门的关系 |
//! |---|---|---|
//! | **闸门口径** | 逐题「命中比例」再平均（`\|expect ∩ top-k\| / \|expect\|`） | **= `golden_run::metrics.recall_at_k`**，闸门判红/判绿用这个 |
//! | 二值任一命中 | 碰到**任意一个**标准答案即算命中 | 粗看用 |
//!
//! **为什么必须分两种**：此前 α 扫描只统计**二值口径**，却拿它去减**闸门口径**的基线
//!；**混口径减法**；得出的「+0.1227」**无效**
//!；修正后同口径的 `first_pos`（α=0.10）真实收益是 **+0.0192**（不是 +0.1227）。
//!
//! # 四臂 + α 扫描
//! 1. **生产口径**（去疑问词 + 收窄停用字，与 `normalize_for_index` 一致）
//! 2. **原始口径**（不做任何规范化）
//! 3. **生产 + `first_pos` 重排**（α 扫描 · 两种口径）
//! 4. **oracle**（相关文档在池里就算命中）
//!
//! # 输出
//! 人读表 + 一行 JSON。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_run::{run_with_extra_sources_and_config, IndexRecord};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::hybrid_search::{normalize_for_index, tokenize, Bm25Config};
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

const K: usize = 10;
const POOL: usize = 50;

/// α 扫描档位。
const ALPHAS: [f64; 6] = [0.05, 0.10, 0.20, 0.50, 1.00, 2.00];

fn exam_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("APEIRETH_MIRACL_EXAM") {
        return PathBuf::from(dir);
    }
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("miracl-exam"),
        None => {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            home.join(".apeireth").join("miracl-exam")
        }
    }
}

fn read_lines(path: &std::path::Path) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(std::fs::read_to_string(path)?
        .lines()
        .map(str::to_string)
        .collect())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("· 失败：{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let root = exam_root();
    let corpus_path = root.join("corpus.jsonl");
    let exam_path = root.join("exam.jsonl");
    if !corpus_path.exists() || !exam_path.exists() {
        eprintln!("· 考卷不在：{} ⇒ **没测到**（不是通过）", root.display());
        return Ok(());
    }

    let mut index: Vec<IndexRecord> = Vec::new();
    let mut content_by_id: HashMap<String, String> = HashMap::new();
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
        content_by_id.insert(id.clone(), joined.clone());
        index.push(IndexRecord { id, text: joined });
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
    eprintln!(
        "· MIRACL 中文考卷：语料 {} 段 · 题 {} 道 · k={K} · 候选上限 {POOL}",
        index.len(),
        cases.len()
    );
    if cases.is_empty() {
        eprintln!("· 题集为空 ⇒ **没测到**");
        return Ok(());
    }
    let set = GoldenSet::new("miracl-zh-dev-sampled".to_string(), cases.clone());

    // 臂 1：生产口径
    let production = run_with_extra_sources_and_config(
        &index,
        &set,
        K,
        apeireth_memory::golden_run::InjectionMode::Production,
        &[],
        None,
    )?
    .metrics
    .recall_at_k;

    // 臂 2：原始口径（关掉规范化）
    let raw = run_with_extra_sources_and_config(
        &index,
        &set,
        K,
        apeireth_memory::golden_run::InjectionMode::Production,
        &[],
        Some(Bm25Config {
            strip_cn_stopwords: false,
            ..Default::default()
        }),
    )?
    .metrics
    .recall_at_k;

    // 臂 3/4：first_pos α 扫描 + oracle
    let bm25 = Bm25LexicalCandidateSource::new(
        index
            .iter()
            .map(|record| MemoryCandidate {
                id: record.id.clone(),
                layer: "miracl".to_string(),
                scope: MemoryScope::Global,
                content: record.text.clone(),
                score: 0.0,
                score_components: Default::default(),
                provenance: Default::default(),
            })
            .collect(),
    );
    let mut first_pos_hits = [0usize; ALPHAS.len()];
    // ⭐⭐ **闸门口径**（= `golden_run::metrics.recall_at_k`）：逐题「命中比例」再平均。
    //
    // ⚠️⚠️ **2026-11-08 修正（混口径减法）**：此前本例只有**二值「任一命中」**的计数，
    // 却拿它去减**闸门口径**的基线（`production`）； **两个不同的指标相减**
    //；得出的「+0.1227」**无效**（真实值 +0.0192）。
    //；现在**两种口径各配一条同口径基线**，delta 永远是同口径减法。
    let mut first_pos_frac = [0.0f64; ALPHAS.len()];
    let mut base_frac = 0.0f64;
    let mut base_any_hits = 0usize;
    let mut oracle_hits = 0usize;
    let mut scored = 0usize;
    for case in &cases {
        let hits = bm25.candidates(&case.query, POOL)?;
        let positive: Vec<_> = hits.iter().filter(|hit| hit.score > 0.0).collect();
        if positive.is_empty() {
            continue;
        }
        scored += 1;
        let gold: HashSet<&str> = case.expect_ids.iter().map(String::as_str).collect();
        if positive.iter().any(|hit| gold.contains(hit.id.as_str())) {
            oracle_hits += 1;
        }

        // ⭐⭐ **必须用生产分词器**（`tokenize(normalize_for_index(query))`）：
        // 本文件此前自带一个「粗分词」，它比生产分词器**多产出每个 CJK 单字**
        //；位置分会被那些单字决定；测出来的不是**上线的那套算法**。
        //；2026-11-08 改为直接调用生产分词器。
        let query_terms: HashSet<String> =
            tokenize(&normalize_for_index(&case.query)).into_iter().collect();
        let max_base = positive
            .iter()
            .map(|h| h.score)
            .fold(0.0_f64, f64::max)
            .max(1e-12);

        // 先把 (id, base, first_pos) 算好（与 α 无关）
        let mut values: Vec<(&str, f64, f64)> = positive
            .iter()
            .map(|hit| {
                let base = hit.score / max_base;
                let doc = content_by_id
                    .get(hit.id.as_str())
                    .map(String::as_str)
                    .unwrap_or("");
                let mut position = f64::MAX;
                for term in &query_terms {
                    if let Some(at) = doc.find(term.as_str()) {
                        position = position.min(at as f64);
                    }
                }
                let value = if position.is_finite() {
                    1.0 / (1.0 + position)
                } else {
                    0.0
                };
                (hit.id.as_str(), base, value)
            })
            .collect();
        // 归一化到 (0,1]（否则 α 的量纲随文档长度漂移）
        let max_value = values.iter().map(|item| item.2).fold(0.0_f64, f64::max);
        if max_value > 0.0 {
            for item in values.iter_mut() {
                item.2 /= max_value;
            }
        }

        // 基线（α=0）—— 与 α 臂**同一口径**（两种口径各一条）
        {
            let mut ranked: Vec<(&str, f64)> = values
                .iter()
                .map(|(id, base, _)| (*id, *base))
                .collect();
            ranked.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(b.0))
            });
            let ids: Vec<&str> = ranked.iter().take(K).map(|item| item.0).collect();
            let found = gold.iter().filter(|id| ids.contains(id)).count() as f64;
            base_frac += found / gold.len() as f64;
            if gold.iter().any(|id| ids.contains(id)) {
                base_any_hits += 1;
            }
        }

        for (alpha_index, alpha) in ALPHAS.iter().copied().enumerate() {
            let mut ranked: Vec<(&str, f64)> = values
                .iter()
                .map(|(id, base, value)| (*id, base + alpha * value))
                .collect();
            ranked.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(b.0))
            });
            let ids: Vec<&str> = ranked.iter().take(K).map(|item| item.0).collect();
            if gold.iter().any(|id| ids.contains(id)) {
                first_pos_hits[alpha_index] += 1;
            }
            let found = gold.iter().filter(|id| ids.contains(id)).count() as f64;
            first_pos_frac[alpha_index] += found / gold.len() as f64;
        }
    }

    let denominator = scored.max(1) as f64;
    let per_alpha: Vec<f64> = first_pos_hits
        .iter()
        .map(|hits| *hits as f64 / denominator)
        .collect();
    let per_alpha_frac: Vec<f64> = first_pos_frac
        .iter()
        .map(|value| *value / denominator)
        .collect();
    let base_frac_avg = base_frac / denominator;
    let base_any_avg = base_any_hits as f64 / denominator;
    let oracle = oracle_hits as f64 / denominator;

    println!("· MIRACL 真实查询：题 {}（有效 {}）", cases.len(), scored);
    println!("     生产口径（含规范化）  {:.4}", production);
    println!(
        "     原始口径（无规范化）  {:.4}（规范化效果 {:+.4}）",
        raw,
        production - raw
    );
    println!(
        "     oracle（池里有就算） {:.4}（天花板余量 {:+.4}）",
        oracle,
        oracle - production
    );
    // ⭐⭐ **两种口径，各配一条同口径基线**，**delta 永远是同口径减法**。
    println!("     == first_pos α 扫描（真实查询） ==");
    println!(
        "        同口径基线：闸门口径 {:.4} · 二值任一命中 {:.4}",
        base_frac_avg, base_any_avg
    );
    for (alpha_index, alpha) in ALPHAS.iter().copied().enumerate() {
        println!(
            "        α={:<5} 闸门口径 {:.4} ({:+.4}) · 二值任一命中 {:.4} ({:+.4})",
            alpha,
            per_alpha_frac[alpha_index],
            per_alpha_frac[alpha_index] - base_frac_avg,
            per_alpha[alpha_index],
            per_alpha[alpha_index] - base_any_avg
        );
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "考卷": "miracl-zh-dev-sampled",
            "语料段数": index.len(),
            "题数": cases.len(),
            "有效题数": scored,
            "生产口径": production,
            "原始口径": raw,
            "规范化效果": production - raw,
            "oracle": oracle,
            "天花板余量": oracle - production,
            "alpha扫描": ALPHAS
                .iter()
                .copied()
                .zip(per_alpha.iter().copied())
                .zip(per_alpha_frac.iter().copied())
                .map(|((alpha, recall), frac)| serde_json::json!({
                    "alpha": alpha,
                    "闸门口径recall": frac,
                    "闸门口径delta": frac - base_frac_avg,
                    "二值任一命中recall": recall,
                    "二值任一命中delta": recall - base_any_avg,
                }))
                .collect::<Vec<_>>(),
        }))?
    );
    Ok(())
}
