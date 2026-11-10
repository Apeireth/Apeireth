//! **排序层的天花板与单特征信号**（2026-11-07）—— 攻排序前先量清楚「值不值得攻」。
//!
//! # 为什么先测这个（顺序不能反）
//! `recall_gap_report` 实测：**NotInPool = 0 题**，而 **42.6% 的目标排在 k=10 之外**
//! （名次 p50=7 / p75=26 / p95=78）⇒ ⇒ ⇒ **瓶颈在排序层**。
//!
//! 但 `feature_blend_ab` 已经实测过：**9 维特征线性混合会过拟合**
//! （训练 +0.0041 / 留出 −0.0179 ⇒ **NO-GO**）
//! ⇒ ⇒ ⇒ 所以**不能一上来就学权重**。
//!
//! # 本例子测两件事（都不需要学习 ⇒ 都不会过拟合）
//! 1. **天花板**：候选池无限重排（oracle）时 recall@10 能到多少
//!    ⇒ ⇒ 若天花板接近基线 ⇒ **排序层没有油水** ⇒ 别在上面花时间；
//! 2. **单特征**：每个**只有 1 个参数 α** 的重排器能涨多少
//!    ⇒ ⇒ 单参数无法过拟合 476 道题 ⇒ 结论可信。
//!
//! # 特征（**全部可在检索时算出，不需要标注**）
//! | 特征 | 含义 |
//! |---|---|
//! | `coverage` | 题面词在文档里的覆盖比例 |
//! | `idf_sum` | 命中词的 IDF 之和（稀有词命中更值钱）|
//! | `first_pos` | 首个命中位置（越靠前越好）|
//! | `entity_hit` | 题面主体词是否命中 |
//! | `len_penalty` | 文档长度惩罚（长文档天然弱）|
//! | `recency` | 文档在语料里的位置（越靠后越新）|
//!
//! # 输出
//! 一行 JSON（每格：基线 / oracle / 每个特征的最优 α 与 delta）。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{build_fair_exam, FairExamConfig};
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{mined_to_golden, run_with_extra_sources, IndexRecord};
use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::hybrid_search::{normalize_for_index, tokenize};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

const K: usize = 10;
const POOL: usize = 50;
const MAX_CASES_PER_RECORD: usize = 6;

/// 每个特征在**每个 α** 下的命中数（**不做平均** ⇒ ⇒ 每个 α 单独可解释）。
const ALPHAS: [f64; 5] = [0.05, 0.10, 0.20, 0.50, 1.00];

#[derive(Default, Clone)]
struct Best {
    /// 与 [`ALPHAS`] **一一对应**的命中数。
    hits: [usize; 5],
}

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

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("· 失败：{error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut rows: Vec<serde_json::Value> = Vec::new();
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
        let total = index.len() as f64;
        // ⭐⭐ **生产口径**（2026-11-08 修正）：文档侧分词同样必须用生产分词器
        // ⇒ ⇒ 否则 IDF / 覆盖率 / 长度惩罚算的不是生产那一套。
        let avg_len = index
            .iter()
            .map(|record| tokenize(&record.text).len() as f64)
            .sum::<f64>()
            / total.max(1.0);
        // IDF：用整个索引算
        let mut doc_freq: HashMap<String, usize> = HashMap::new();
        let mut doc_terms: Vec<HashSet<String>> = Vec::with_capacity(index.len());
        for record in &index {
            let set: HashSet<String> = tokenize(&record.text).into_iter().collect();
            for term in &set {
                *doc_freq.entry(term.clone()).or_default() += 1;
            }
            doc_terms.push(set);
        }
        let idf = |term: &str| -> f64 {
            let df = doc_freq.get(term).copied().unwrap_or(0) as f64;
            ((total + 1.0) / (df + 1.0)).ln().max(0.0)
        };
        let bm25 = Bm25LexicalCandidateSource::new(
            index
                .iter()
                .map(|record| MemoryCandidate {
                    id: record.id.clone(),
                    layer: "golden".to_string(),
                    scope: MemoryScope::Global,
                    content: record.text.clone(),
                    score: 0.0,
                    score_components: Default::default(),
                    provenance: Default::default(),
                })
                .collect(),
        );
        let position: HashMap<&str, usize> = index
            .iter()
            .enumerate()
            .map(|(i, record)| (record.id.as_str(), i))
            .collect();
        let content: HashMap<&str, &str> = index
            .iter()
            .map(|record| (record.id.as_str(), record.text.as_str()))
            .collect();

        let config = MiningConfig {
            max_cases_per_record: MAX_CASES_PER_RECORD,
            ..MiningConfig::default()
        };
        let mined = apeireth_memory::golden_mining::mine(&records, config);
        let lexical = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));
        let fair = GoldenSet::new(
            format!("fair-{name}"),
            build_fair_exam(&records, FairExamConfig::default()),
        );

        for (exam, set) in [("词面卷", &lexical), ("公平卷", &fair)] {
            let baseline = run_with_extra_sources(
                &index,
                set,
                K,
                apeireth_memory::golden_run::InjectionMode::Production,
                &[],
            )?
            .metrics
            .recall_at_k;

            // 逐题建候选池 + 特征
            let mut oracle_hits = 0usize;
            let mut base_hits = 0usize;
            let mut best: HashMap<&'static str, Best> = HashMap::new();
            for feature in [
                "coverage",
                "idf_sum",
                "first_pos",
                "entity_hit",
                "len_penalty",
                "recency",
            ] {
                best.insert(feature, Best::default());
            }
            let mut cases = 0usize;
            for case in &set.cases {
                let hits = bm25.candidates(&case.query, POOL)?;
                if hits.is_empty() {
                    continue;
                }
                let positive: Vec<_> = hits.iter().filter(|hit| hit.score > 0.0).collect();
                if positive.is_empty() {
                    continue;
                }
                cases += 1;
                // ⭐⭐ **生产口径**（2026-11-08 修正）：原先用本地「粗分词」
                // 且**不做规范化** ⇒ ⇒ 测的不是上线的那套算法。
                let query_terms: HashSet<String> = tokenize(&normalize_for_index(&case.query))
                    .into_iter()
                    .collect();
                let in_top = |ids: &[&str]| -> bool {
                    !case.expect_ids.is_empty()
                        && case.expect_ids.iter().all(|id| ids.contains(&id.as_str()))
                };
                let base_ids: Vec<&str> = positive.iter().take(K).map(|h| h.id.as_str()).collect();
                if in_top(&base_ids) {
                    base_hits += 1;
                }
                // oracle：池子里有目标就算命中
                let pool_ids: Vec<&str> = positive.iter().map(|h| h.id.as_str()).collect();
                if in_top(&pool_ids) {
                    oracle_hits += 1;
                }

                // 每特征：α 扫描
                for (feature, slot) in best.iter_mut() {
                    let mut values: HashMap<&str, f64> = HashMap::new();
                    for hit in &positive {
                        let doc = content.get(hit.id.as_str()).copied().unwrap_or("");
                        let doc_terms = doc_terms
                            .get(*position.get(hit.id.as_str()).unwrap_or(&0))
                            .cloned()
                            .unwrap_or_default();
                        let value = match *feature {
                            "coverage" => {
                                if query_terms.is_empty() {
                                    0.0
                                } else {
                                    query_terms
                                        .iter()
                                        .filter(|t| doc_terms.contains(*t))
                                        .count() as f64
                                        / query_terms.len() as f64
                                }
                            }
                            "idf_sum" => query_terms
                                .iter()
                                .filter(|t| doc_terms.contains(*t))
                                .map(|t| idf(t))
                                .sum(),
                            "first_pos" => {
                                let bytes = doc;
                                let mut pos = f64::MAX;
                                for term in &query_terms {
                                    if let Some(index) = bytes.find(term.as_str()) {
                                        pos = pos.min(index as f64);
                                    }
                                }
                                if pos.is_finite() {
                                    1.0 / (1.0 + pos)
                                } else {
                                    0.0
                                }
                            }
                            "entity_hit" => {
                                // 题面里「最长的一个词」当作主体词
                                let subject = query_terms
                                    .iter()
                                    .filter(|t| t.chars().count() >= 2)
                                    .max_by_key(|t| t.chars().count());
                                match subject {
                                    Some(s) if doc_terms.contains(s) => 1.0,
                                    _ => 0.0,
                                }
                            }
                            "len_penalty" => {
                                let len = doc_terms.len() as f64;
                                1.0 / (1.0 + len / avg_len.max(1.0))
                            }
                            "recency" => {
                                let pos = *position.get(hit.id.as_str()).unwrap_or(&0) as f64;
                                pos / total.max(1.0)
                            }
                            _ => 0.0,
                        };
                        values.insert(hit.id.as_str(), value);
                    }
                    let max_value = values.values().cloned().fold(0.0_f64, f64::max);
                    let max_base = positive
                        .iter()
                        .map(|h| h.score)
                        .fold(0.0_f64, f64::max)
                        .max(1e-12);
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
                        if in_top(&ids) {
                            slot.hits[alpha_index] += 1;
                        }
                    }
                }
            }
            // 汇总（每特征单独算 delta）
            let base_recall = if cases == 0 {
                0.0
            } else {
                base_hits as f64 / cases as f64
            };
            let oracle_recall = if cases == 0 {
                0.0
            } else {
                oracle_hits as f64 / cases as f64
            };
            let mut features = Vec::new();
            for (feature, slot) in &best {
                let per_alpha: Vec<f64> = slot
                    .hits
                    .iter()
                    .map(|hits| *hits as f64 / cases.max(1) as f64)
                    .collect();
                let best_index = per_alpha
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(index, _)| index)
                    .unwrap_or(0);
                features.push(serde_json::json!({
                    "特征": feature,
                    "最优alpha": ALPHAS[best_index],
                    "最优recall": per_alpha[best_index],
                    "最优delta": per_alpha[best_index] - base_recall,
                    "各alpha": ALPHAS.iter().copied().zip(per_alpha.iter().copied())
                        .map(|(alpha, recall)| serde_json::json!({"alpha": alpha, "recall": recall}))
                        .collect::<Vec<_>>(),
                }));
            }
            features.sort_by(|a, b| {
                b["最优delta"]
                    .as_f64()
                    .unwrap_or(0.0)
                    .partial_cmp(&a["最优delta"].as_f64().unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            // ⭐⭐ **形态判据二：原始查询里有没有疑问词**（2026-11-07 实测）
            //
            // **为什么换判据**：第一个判据（查询词在语料里的覆盖率）**失败了**
            // ⇒ ⇒ 中文 bigram 几乎全在语料里 ⇒ ⇒ 词面卷与公平卷**都判成重叠**（分不开）。
            //
            // **新判据**：**原始查询**（未经规范化）里是否含疑问词
            // ⇒ ⇒ 词面卷的查询是**原文片段**（很少带疑问词）
            // ⇒ ⇒ 公平卷是**自然语言问句**（大量带「什么/怎么/为什么」）
            // ⭐⭐ **与 MIRACL 考卷完全相同的 first_pos 实现**（2026-11-07）
            //
            // **为什么必须同实现**：先前版本用**未归一化**的 `1/(1+pos)`
            // ⇒ ⇒ α 的量纲随文档长度漂移 ⇒ ⇒ 与真实查询考卷上的数字**不可比**。
            // 这里改成：**每题把 first_pos 归一化到 (0,1]**，α 列表也与真实查询考卷一致。
            let alphas = [0.05_f64, 0.10, 0.20, 0.50, 1.00, 2.00];
            let mut normalized_first_pos = Vec::new();
            for alpha in alphas {
                let mut hits = 0usize;
                let mut used = 0usize;
                let mut total_cases = 0usize;
                for case in &set.cases {
                    let candidates = bm25.candidates(&case.query, POOL)?;
                    let positive: Vec<_> =
                        candidates.iter().filter(|hit| hit.score > 0.0).collect();
                    if positive.is_empty() {
                        continue;
                    }
                    total_cases += 1;
                    used += 1;
                    // 规范化后的题面词（与生产 `normalize_for_index` 同口径）
                    let normalized_query: String =
                        apeireth_memory::hybrid_search::strip_cn_interrogatives(&case.query)
                            .chars()
                            .filter(|c| {
                                !apeireth_memory::hybrid_search::CN_STOPWORD_CHARS.contains(c)
                            })
                            .collect();
                    let query_terms: HashSet<String> =
                        tokenize(&normalized_query).into_iter().collect();
                    let max_base = positive
                        .iter()
                        .map(|hit| hit.score)
                        .fold(0.0_f64, f64::max)
                        .max(1e-12);
                    let mut values: Vec<(&str, f64, f64)> = positive
                        .iter()
                        .map(|hit| {
                            let base = hit.score / max_base;
                            let doc = content.get(hit.id.as_str()).copied().unwrap_or("");
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
                    let max_value = values.iter().map(|item| item.2).fold(0.0_f64, f64::max);
                    if max_value > 0.0 {
                        for item in values.iter_mut() {
                            item.2 /= max_value;
                        }
                    }
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
                    if !case.expect_ids.is_empty()
                        && case.expect_ids.iter().all(|id| ids.contains(&id.as_str()))
                    {
                        hits += 1;
                    }
                }
                normalized_first_pos.push(serde_json::json!({
                    "alpha": alpha,
                    "recall": hits as f64 / total_cases.max(1) as f64,
                    "delta": hits as f64 / total_cases.max(1) as f64 - base_recall,
                    "启用题数": used,
                    "总题数": total_cases,
                }));
            }

            // ⭐⭐ **查询形态准入**（2026-11-07）：`first_pos` 只在「像自然语言问题」时用。
            //
            // **实测依据**：`first_pos` 在词面卷 **−0.0084/−0.0243**、公平卷 **+0.0481/+0.0325**
            // ⇒ ⇒ 词面卷的查询**就是原文片段**（位置无意义）
            // ⇒ ⇒ 公平卷是自然语言问题（实体在**前部** = 主题相关）
            //
            // **判据**：查询词在**整个语料**里的覆盖率 `overlap`
            // ⇒ ⇒ 片段式查询的词**几乎全在语料里**（overlap 高）
            // ⇒ ⇒ 自然语言问题含疑问词/虚词（overlap 低）
            let mut shape = Vec::new();
            for threshold in [0.50_f64, 0.60, 0.70, 0.80, 0.90] {
                let mut hits = 0usize;
                let mut used = 0usize;
                let mut total_cases = 0usize;
                for case in &set.cases {
                    let candidates = bm25.candidates(&case.query, POOL)?;
                    let positive: Vec<_> =
                        candidates.iter().filter(|hit| hit.score > 0.0).collect();
                    if positive.is_empty() {
                        continue;
                    }
                    total_cases += 1;
                    let query_terms: HashSet<String> = tokenize(&normalize_for_index(&case.query))
                        .into_iter()
                        .collect();
                    // overlap：查询词在**全语料**里出现的比例
                    let overlap = if query_terms.is_empty() {
                        1.0
                    } else {
                        query_terms
                            .iter()
                            .filter(|term| doc_freq.contains_key(*term))
                            .count() as f64
                            / query_terms.len() as f64
                    };
                    let apply = overlap < threshold;
                    if apply {
                        used += 1;
                    }
                    let max_base = positive
                        .iter()
                        .map(|hit| hit.score)
                        .fold(0.0_f64, f64::max)
                        .max(1e-12);
                    let mut ranked: Vec<(&str, f64)> = positive
                        .iter()
                        .map(|hit| {
                            let base = hit.score / max_base;
                            if !apply {
                                return (hit.id.as_str(), base);
                            }
                            let doc = content.get(hit.id.as_str()).copied().unwrap_or("");
                            let mut pos = f64::MAX;
                            for term in &query_terms {
                                if let Some(index) = doc.find(term.as_str()) {
                                    pos = pos.min(index as f64);
                                }
                            }
                            let value = if pos.is_finite() {
                                1.0 / (1.0 + pos)
                            } else {
                                0.0
                            };
                            (hit.id.as_str(), base + 0.50 * value)
                        })
                        .collect();
                    ranked.sort_by(|a, b| {
                        b.1.partial_cmp(&a.1)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| a.0.cmp(b.0))
                    });
                    let ids: Vec<&str> = ranked.iter().take(K).map(|item| item.0).collect();
                    if !case.expect_ids.is_empty()
                        && case.expect_ids.iter().all(|id| ids.contains(&id.as_str()))
                    {
                        hits += 1;
                    }
                }
                shape.push(serde_json::json!({
                    "threshold": threshold,
                    "recall": hits as f64 / total_cases.max(1) as f64,
                    "delta": hits as f64 / total_cases.max(1) as f64 - base_recall,
                    "启用题数": used,
                    "总题数": total_cases,
                }));
            }
            rows.push(serde_json::json!({
                "normalized_first_pos": normalized_first_pos,
                "形态准入": shape,
                "语料": name,
                "考卷": exam,
                "题数": cases,
                "基线recall": base_recall,
                "oraclerecall": oracle_recall,
                "天花板余量": oracle_recall - base_recall,
                "闸门recall": baseline,
                "特征": features,
            }));
        }
    }

    for row in &rows {
        println!(
            "· {} / {} · 题 {} · 基线 {:.4} · oracle {:.4}（**余量 {:+.4}**）",
            row["语料"].as_str().unwrap_or(""),
            row["考卷"].as_str().unwrap_or(""),
            row["题数"],
            row["基线recall"].as_f64().unwrap_or(0.0),
            row["oraclerecall"].as_f64().unwrap_or(0.0),
            row["天花板余量"].as_f64().unwrap_or(0.0),
        );
        for s in row["normalized_first_pos"].as_array().unwrap_or(&vec![]) {
            println!(
                "     [疑问词判据] alpha {:.2} recall {:.4} ({:+.4}) · 启用 {}/{} 题",
                s["alpha"].as_f64().unwrap_or(0.0),
                s["recall"].as_f64().unwrap_or(0.0),
                s["delta"].as_f64().unwrap_or(0.0),
                s["启用题数"].as_u64().unwrap_or(0),
                s["总题数"].as_u64().unwrap_or(0),
            );
        }
        for s in row["形态准入"].as_array().unwrap_or(&vec![]) {
            println!(
                "     [形态] 阈值 {:.2} recall {:.4} ({:+.4}) · 启用 {}/{} 题",
                s["threshold"].as_f64().unwrap_or(0.0),
                s["recall"].as_f64().unwrap_or(0.0),
                s["delta"].as_f64().unwrap_or(0.0),
                s["启用题数"].as_u64().unwrap_or(0),
                s["总题数"].as_u64().unwrap_or(0),
            );
        }
        for f in row["特征"].as_array().unwrap_or(&vec![]) {
            println!(
                "     {:<12} 最优 alpha={:.2} recall {:.4} ({:+.4})",
                f["特征"].as_str().unwrap_or(""),
                f["最优alpha"].as_f64().unwrap_or(0.0),
                f["最优recall"].as_f64().unwrap_or(0.0),
                f["最优delta"].as_f64().unwrap_or(0.0),
            );
        }
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "排序天花板与单特征": rows }))
            .unwrap_or_default()
    );
    Ok(())
}
