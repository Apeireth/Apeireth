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
//! # 四臂 + α 扫描
//! 1. **生产口径**（去疑问词 + 收窄停用字，与 `normalize_for_index` 一致）
//! 2. **原始口径**（不做任何规范化）
//! 3. **生产 + `first_pos` 重排**（α 扫描）
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
use apeireth_memory::hybrid_search::{strip_cn_interrogatives, Bm25Config, CN_STOPWORD_CHARS};
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

/// 粗分词（与生产同构：ASCII 词 + CJK 单字 + bigram）。
fn terms(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut ascii = String::new();
    let mut prev: Option<char> = None;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            ascii.push(ch.to_ascii_lowercase());
            prev = None;
        } else {
            if !ascii.is_empty() {
                out.push(std::mem::take(&mut ascii));
            }
            if (ch as u32) > 0x2E80 && ch.is_alphabetic() {
                out.push(ch.to_string());
                if let Some(p) = prev {
                    out.push(format!("{p}{ch}"));
                }
                prev = Some(ch);
            } else {
                prev = None;
            }
        }
    }
    if !ascii.is_empty() {
        out.push(ascii);
    }
    out
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

        let normalized: String = strip_cn_interrogatives(&case.query)
            .chars()
            .filter(|c| !CN_STOPWORD_CHARS.contains(c))
            .collect();
        let query_terms: HashSet<String> = terms(&normalized).into_iter().collect();
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
        }
    }

    let denominator = scored.max(1) as f64;
    let per_alpha: Vec<f64> = first_pos_hits
        .iter()
        .map(|hits| *hits as f64 / denominator)
        .collect();
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
    println!("     == first_pos α 扫描（真实查询） ==");
    for (alpha_index, alpha) in ALPHAS.iter().copied().enumerate() {
        println!(
            "        α={:<5} recall {:.4} ({:+.4})",
            alpha,
            per_alpha[alpha_index],
            per_alpha[alpha_index] - production
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
                .map(|(alpha, recall)| serde_json::json!({
                    "alpha": alpha,
                    "recall": recall,
                    "delta_vs_production": recall - production,
                }))
                .collect::<Vec<_>>(),
        }))?
    );
    Ok(())
}
