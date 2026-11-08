//! **排序特征决策矩阵**（2026-11-08）—— 六个单参数重排特征 × 三套考卷，**同一套代码路径**。
//!
//! # 要回答的问题
//! **在「产品形态」的考卷和「官方标注」的考卷上，有没有哪个单参数重排特征是两边都涨的？**
//!
//! ⇒ ⇒ 这是 P2「其它候选排序信号」的**唯一判据**：
//! **两套卷都涨才上**（只在弱底座上涨不算数）。
//!
//! # 之前缺的那一格
//! `rerank_headroom_ab` 只在**我们自己的语料考卷**上量过六个特征
//! （词面卷 / 公平卷），**从来没在真人卷上量过**
//! ⇒ ⇒ 于是「哪个特征值得做」这个问题**一直只有一把尺子**。
//! ⇒ ⇒ 本例把**六个特征 × 三套卷**放进**同一套代码路径**（同一个特征实现、同一套分词、同一个候选池）。
//!
//! # 三套卷是什么角色
//!
//! | 卷 | 标签来源 | 角色 |
//! |---|---|---|
//! | **词面卷** | 我们挖（金标 = **主体本身**的那条记录）| **产品形态**（用户要「关于 X 的那条记录」）|
//! | 公平卷 | 我们挖（金标 = **顺带提及**X 的那条记录）| **诊断用**（不是产品形态，见 `first_pos_leakage_probe`）|
//! | **真人卷 MIRACL** | **官方人工标注** | **真实形态 + 独立标签** |
//!
//! # ⭐ 上线判据
//! **词面卷（产品形态）与真人卷（官方标注）必须同时为正**；任一为负 ⇒ **不上**。
//!
//! # 口径（与生产逐字一致）
//! * 查询词 = `tokenize(normalize_for_index(query))`（**生产分词器 + 生产规范化**）；
//! * 文档词 = `tokenize(content)`；IDF 由**整个考卷语料**算；
//! * 候选池 = BM25 前 50 且分数 > 0；k = 10；
//! * `base` = **生产排序分**（BM25 + 覆盖率融合，按池内 max 归一）；
//! * 特征值按题归一化到 (0,1]；最终分 `base + alpha × 特征值`；
//! * **两种指标都报**：**闸门口径**（逐题「命中比例」再平均）与**二值全部命中**（内部自洽）。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{build_fair_exam, FairExamConfig};
use apeireth_memory::golden_mining::{mine, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, IndexRecord};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::hybrid_search::{normalize_for_index, tokenize};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

const K: usize = 10;
const POOL: usize = 50;
const MAX_CASES_PER_RECORD: usize = 6;
const ALPHAS: [f64; 5] = [0.05, 0.10, 0.20, 0.50, 1.00];
const FEATURES: [&str; 6] = [
    "coverage",
    "idf_sum",
    "first_pos",
    "entity_hit",
    "len_penalty",
    "recency",
];

fn corpus_root() -> PathBuf {
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

/// 一套考卷的素材。
struct Exam {
    label: String,
    role: &'static str,
    cases: Vec<GoldenCase>,
    index: Vec<IndexRecord>,
    content: HashMap<String, String>,
}

/// 一个特征在一个 α 下的累计量。
#[derive(Default, Clone)]
struct Cell {
    /// 闸门口径的分子（逐题命中比例之和）。
    frac_sum: f64,
    /// 二值「全部命中」题数。
    all_hits: usize,
    /// 参与统计的题数。
    cases: usize,
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
    let mut exams: Vec<Exam> = Vec::new();

    // ---------- 真人卷（MIRACL：官方人工标注）----------
    let root = exam_root();
    if (root.join("corpus.jsonl")).exists() && (root.join("exam.jsonl")).exists() {
        let mut index: Vec<IndexRecord> = Vec::new();
        let mut content: HashMap<String, String> = HashMap::new();
        for line in std::fs::read_to_string(root.join("corpus.jsonl"))?.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line)?;
            let id = value["id"].as_str().unwrap_or_default().to_string();
            if id.is_empty() {
                continue;
            }
            let joined = format!(
                "{}\n{}",
                value["title"].as_str().unwrap_or_default(),
                value["text"].as_str().unwrap_or_default()
            );
            content.insert(id.clone(), joined.clone());
            index.push(IndexRecord { id, text: joined });
        }
        let mut cases: Vec<GoldenCase> = Vec::new();
        for line in std::fs::read_to_string(root.join("exam.jsonl"))?.lines() {
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
            cases.push(GoldenCase {
                id: value["id"].as_str().unwrap_or_default().to_string(),
                query,
                expect_ids: gold,
                expect_fragments: Vec::new(),
                tags: vec!["miracl".to_string()],
            });
        }
        exams.push(Exam {
            label: "真人卷 / MIRACL（官方人工标注）".to_string(),
            role: "真实形态 + 独立标签",
            cases,
            index,
            content,
        });
    } else {
        eprintln!("· 真人卷考卷不在（{}）⇒ 跳过（这不是通过）", root.display());
    }

    // ---------- 我们的两套卷 ----------
    for name in ["redchamber", "watershed"] {
        let path = corpus_root().join(format!("{name}.txt"));
        if !path.exists() {
            eprintln!("· {name}：语料不在，跳过");
            continue;
        }
        let Ok(chapters) = novel_corpus::load_chapters(&path) else {
            eprintln!("· {name}：语料读不了，跳过");
            continue;
        };
        let records = novel_corpus::to_mine_records(&chapters);
        let index: Vec<IndexRecord> = records
            .iter()
            .map(|record| IndexRecord {
                id: record.id.clone(),
                text: record.content.clone(),
            })
            .collect();
        let content: HashMap<String, String> = index
            .iter()
            .map(|record| (record.id.clone(), record.text.clone()))
            .collect();

        let mined = mine(
            &records,
            MiningConfig {
                max_cases_per_record: MAX_CASES_PER_RECORD,
                ..MiningConfig::default()
            },
        );
        exams.push(Exam {
            label: format!("词面卷 / {name}（金标=主体本身）"),
            role: "产品形态",
            cases: GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined)).cases,
            index: index.clone(),
            content: content.clone(),
        });
        let fair = build_fair_exam(
            &records,
            FairExamConfig {
                max_cases_per_record: MAX_CASES_PER_RECORD,
                ..FairExamConfig::default()
            },
        );
        exams.push(Exam {
            label: format!("公平卷 / {name}（金标=顺带提及）"),
            role: "诊断用（非产品形态）",
            cases: fair,
            index: index.clone(),
            content: content.clone(),
        });
    }

    if exams.is_empty() {
        eprintln!("· 没有可测的考卷");
        return Ok(());
    }

    // ---------- 逐卷量 ----------
    let mut summary: Vec<serde_json::Value> = Vec::new();
    for exam in &exams {
        let result = measure(exam)?;
        println!("=== {} · {} ===", exam.label, exam.role);
        println!(
            "     基线：闸门口径 {:.4} · 二值全部命中 {:.4} · 题 {}",
            result.baseline_frac, result.baseline_all, result.cases
        );
        for feature in FEATURES {
            let mut line = format!("     {feature:<13}");
            for (alpha_index, alpha) in ALPHAS.iter().copied().enumerate() {
                let cell = &result.cells[feature][alpha_index];
                let frac = cell.frac_sum / cell.cases.max(1) as f64;
                let delta = frac - result.baseline_frac;
                line.push_str(&format!("  α{alpha}: {:+.4}", delta));
            }
            println!("{line}");
        }
        summary.push(serde_json::json!({
            "考卷": exam.label,
            "角色": exam.role,
            "题数": result.cases,
            "基线_闸门口径": result.baseline_frac,
            "基线_二值全部命中": result.baseline_all,
            "特征": FEATURES.iter().map(|feature| serde_json::json!({
                "特征": feature,
                "各alpha_delta": ALPHAS.iter().copied().zip(result.cells[*feature].iter()).map(|(alpha, cell)| serde_json::json!({
                    "alpha": alpha,
                    "闸门口径delta": cell.frac_sum / cell.cases.max(1) as f64 - result.baseline_frac,
                    "二值全部命中delta": cell.all_hits as f64 / cell.cases.max(1) as f64 - result.baseline_all,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }));
    }

    // ---------- 决策表：产品形态（词面卷）与真人卷是否同时为正 ----------
    println!("\n=== ⭐ 决策表：每个特征的最优 α（闸门口径）与「两边是否都涨」===");
    let product = summary
        .iter()
        .find(|row| row["角色"] == "产品形态")
        .cloned();
    let real = summary
        .iter()
        .find(|row| row["考卷"].as_str().unwrap_or_default().contains("MIRACL"))
        .cloned();
    if let (Some(product), Some(real)) = (product, real) {
        let mut rows = Vec::new();
        for feature in FEATURES {
            let pick = |exam: &serde_json::Value| -> (f64, f64) {
                let entry = exam["特征"]
                    .as_array()
                    .and_then(|items| items.iter().find(|item| item["特征"] == feature))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let best = entry["各alpha_delta"]
                    .as_array()
                    .and_then(|items| {
                        items.iter().max_by(|a, b| {
                            a["闸门口径delta"]
                                .as_f64()
                                .unwrap_or(f64::MIN)
                                .partial_cmp(&b["闸门口径delta"].as_f64().unwrap_or(f64::MIN))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                    })
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                (
                    best["闸门口径delta"].as_f64().unwrap_or(f64::NAN),
                    best["alpha"].as_f64().unwrap_or(f64::NAN),
                )
            };
            let (product_delta, product_alpha) = pick(&product);
            let (real_delta, real_alpha) = pick(&real);
            let both = product_delta > 0.0 && real_delta > 0.0;
            println!(
                "     {feature:<13} 产品形态 {:+.4}（α={product_alpha}）· 真人卷 {:+.4}（α={real_alpha}）=> {}",
                product_delta,
                real_delta,
                if both { "✅ 两边都涨" } else { "⛔ 不上" }
            );
            rows.push(serde_json::json!({
                "特征": feature,
                "产品形态delta": product_delta,
                "产品形态alpha": product_alpha,
                "真人卷delta": real_delta,
                "真人卷alpha": real_alpha,
                "两边都涨": both,
            }));
        }
        summary.push(serde_json::json!({ "决策表": rows }));
    }

    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "排序特征矩阵": summary }))?
    );
    Ok(())
}

/// 一套卷的测量结果。
struct Measurement {
    cases: usize,
    baseline_frac: f64,
    baseline_all: f64,
    /// `[特征][α]` 的累计量。
    cells: HashMap<&'static str, Vec<Cell>>,
}

fn measure(exam: &Exam) -> Result<Measurement, Box<dyn Error>> {
    // 文档侧：生产分词 + IDF + 长度
    let total = exam.index.len() as f64;
    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    let mut doc_terms: Vec<HashSet<String>> = Vec::with_capacity(exam.index.len());
    let mut doc_len_sum = 0.0f64;
    for record in &exam.index {
        let set: HashSet<String> = tokenize(&record.text).into_iter().collect();
        doc_len_sum += set.len() as f64;
        for term in &set {
            *doc_freq.entry(term.clone()).or_default() += 1;
        }
        doc_terms.push(set);
    }
    let avg_len = (doc_len_sum / total.max(1.0)).max(1.0);
    let idf = |term: &str| -> f64 {
        let df = doc_freq.get(term).copied().unwrap_or(0) as f64;
        ((total + 1.0) / (df + 1.0)).ln().max(0.0)
    };

    let bm25 = Bm25LexicalCandidateSource::new(
        exam.index
            .iter()
            .map(|record| MemoryCandidate {
                id: record.id.clone(),
                layer: "matrix".to_string(),
                scope: MemoryScope::Global,
                content: record.text.clone(),
                score: 0.0,
                score_components: Default::default(),
                provenance: Default::default(),
            })
            .collect(),
    );
    let position: HashMap<&str, usize> = exam
        .index
        .iter()
        .enumerate()
        .map(|(index, record)| (record.id.as_str(), index))
        .collect();

    let mut cells: HashMap<&'static str, Vec<Cell>> = HashMap::new();
    for feature in FEATURES {
        cells.insert(feature, vec![Cell::default(); ALPHAS.len()]);
    }
    let mut baseline_frac_sum = 0.0f64;
    let mut baseline_all = 0usize;
    let mut cases = 0usize;

    for case in &exam.cases {
        let hits = bm25.candidates(&case.query, POOL)?;
        let positive: Vec<_> = hits.iter().filter(|hit| hit.score > 0.0).collect();
        if positive.is_empty() {
            continue;
        }
        cases += 1;
        let gold: HashSet<&str> = case.expect_ids.iter().map(String::as_str).collect();
        let max_base = positive
            .iter()
            .map(|hit| hit.score)
            .fold(0.0_f64, f64::max)
            .max(1e-12);

        let query_terms: HashSet<String> = tokenize(&normalize_for_index(&case.query))
            .into_iter()
            .collect();
        if query_terms.is_empty() {
            continue;
        }

        // 基线排序（α=0）
        let mut base_ranked: Vec<(&str, f64)> = positive
            .iter()
            .map(|hit| (hit.id.as_str(), hit.score / max_base))
            .collect();
        base_ranked.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(b.0))
        });
        let base_ids: Vec<&str> = base_ranked.iter().take(K).map(|item| item.0).collect();
        baseline_frac_sum += fraction(&gold, &base_ids);
        if gold.iter().all(|id| base_ids.contains(id)) {
            baseline_all += 1;
        }

        // 每个特征：算值 ⇒ 归一 ⇒ α 扫描
        for feature in FEATURES {
            let mut values: HashMap<&str, f64> = HashMap::new();
            for hit in &positive {
                let doc = exam
                    .content
                    .get(hit.id.as_str())
                    .map(String::as_str)
                    .unwrap_or("");
                let terms = &doc_terms[position[hit.id.as_str()]];
                let value = match feature {
                    "coverage" => {
                        query_terms.iter().filter(|t| terms.contains(*t)).count() as f64
                            / query_terms.len() as f64
                    }
                    "idf_sum" => query_terms
                        .iter()
                        .filter(|t| terms.contains(*t))
                        .map(|t| idf(t))
                        .sum(),
                    "first_pos" => {
                        let mut pos = f64::MAX;
                        for term in &query_terms {
                            if let Some(at) = doc.find(term.as_str()) {
                                pos = pos.min(at as f64);
                            }
                        }
                        if pos.is_finite() {
                            1.0 / (1.0 + pos)
                        } else {
                            0.0
                        }
                    }
                    "entity_hit" => match query_terms
                        .iter()
                        .filter(|t| t.chars().count() >= 2)
                        .max_by_key(|t| t.chars().count())
                    {
                        Some(subject) if terms.contains(subject) => 1.0,
                        _ => 0.0,
                    },
                    "len_penalty" => 1.0 / (1.0 + terms.len() as f64 / avg_len),
                    "recency" => position[hit.id.as_str()] as f64 / total.max(1.0),
                    _ => 0.0,
                };
                values.insert(hit.id.as_str(), value);
            }
            let max_value = values.values().cloned().fold(0.0_f64, f64::max);
            for (alpha_index, alpha) in ALPHAS.iter().copied().enumerate() {
                let mut ranked: Vec<(&str, f64)> = positive
                    .iter()
                    .map(|hit| {
                        let base = hit.score / max_base;
                        let value = if max_value > 0.0 {
                            values.get(hit.id.as_str()).copied().unwrap_or(0.0) / max_value
                        } else {
                            0.0
                        };
                        (hit.id.as_str(), base + alpha * value)
                    })
                    .collect();
                ranked.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.cmp(b.0))
                });
                let ids: Vec<&str> = ranked.iter().take(K).map(|item| item.0).collect();
                let cell = &mut cells.get_mut(feature).expect("feature")[alpha_index];
                cell.cases += 1;
                cell.frac_sum += fraction(&gold, &ids);
                if gold.iter().all(|id| ids.contains(id)) {
                    cell.all_hits += 1;
                }
            }
        }
    }

    Ok(Measurement {
        cases,
        baseline_frac: baseline_frac_sum / cases.max(1) as f64,
        baseline_all: baseline_all as f64 / cases.max(1) as f64,
        cells,
    })
}

/// 闸门口径：逐题「命中比例」。
fn fraction(gold: &HashSet<&str>, ids: &[&str]) -> f64 {
    if gold.is_empty() {
        return 0.0;
    }
    gold.iter().filter(|id| ids.contains(*id)).count() as f64 / gold.len() as f64
}
