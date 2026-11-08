//! 「首命中位置」（first_pos）在模板考卷上失效的机制诊断 —— **按题型分桶**（2026-11-08）。
//!
//! # 要回答的问题
//! `first_pos`（alpha=0.10）在**真实用户查询**上 **+0.0192**，
//! 在**模板词面卷**上 **−0.0431** —— 同一个特性，两种考卷结论相反。
//! 这说明要么**测量仪器有偏差**，要么**特征本身太弱**；**必须先查清是哪一种**。
//!
//! ⚠️⚠️ **2026-11-08 更正**：本文件初版写的真实查询收益是 **+0.1227**；**那个数字无效**
//! （基线 0.7246 是**闸门口径**，0.8473 是**二值「任一命中」**；混口径减法）。
//!；同口径重测 = **+0.0192**。详见 `first_pos_arms.rs` 与
//! `docs/04-internal/01-handoffs/memory-first-pos-mechanism-2026-11-08.md`。
//!
//! # 本例测什么（第二版）
//! 第一版按「题面内容词个数」分桶，**假设被自己的数据推翻**：
//! 内容词越多 delta 越负（8 词 −0.0826 比 6 词 +0.0083 更差），
//! 与「多词更有利于首位置」的假设方向相反。
//! 复盘样例题面发现：**内容词个数其实在代理「题型」**：
//!
//! | 内容词 | 题面 | 挖题器 `kind` |
//! |---|---|---|
//! | 6 | `{主体}后来是怎么改的？` | `conflict` |
//! | 8 | `关于{主体}，我们当时记了什么？` | `keyword` |
//! | 9 | `{主体}这件事最后是怎么定的？` | `decision` |
//!
//! 所以本版**按题型（挖题器 `kind`）分桶**重测。
//!
//! # 每个题型量四件事
//! 1. **基线 recall@10** 与各 alpha 的 delta（判定这个题型是受益还是受害）；
//! 2. **池内可达率**：标准答案是否全在 BM25 前 50 的池里
//!    （把「池子不够」与「排序不对」分开）；
//! 3. **位置分方向 `pos_gap`**：池内答案候选的位置分均值减去非答案候选的位置分均值。
//!    正 = 首位置偏向答案；负 = 首位置与答案**反相关**（系统性惩罚这个特征）；
//! 4. **最早命中词**：top-10 里每个候选最早命中的是题面哪个词
//!    （答案候选与非答案候选分开统计），定位「是什么词把首位置抢走了」。
//!
//! # 口径
//! * 实现与生产**逐字一致**：`tokenize(normalize_for_index(query))` 取词，
//!   `content.find(term)` 取首次字符位置，位置分 `1/(1+pos)`，按题归一化到 (0,1]，
//!   最终分 `base + alpha * 位置分`；
//! * **命中判据**：标准答案**全部**出现在 top-10 才算命中（与 `golden_run` 一致）；
//! * **池**：BM25 取前 50。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_mining::{mine, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, IndexRecord};
use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::hybrid_search::{normalize_for_index, tokenize};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

const K: usize = 10;
const POOL: usize = 50;
const MAX_CASES_PER_RECORD: usize = 6;
const ALPHAS: [f64; 4] = [0.05, 0.10, 0.20, 0.50];

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

/// 一个词的命中统计：次数 + 首次命中位置之和。
#[derive(Debug, Default, Clone)]
struct TermTally {
    count: usize,
    pos_sum: usize,
}

/// 一个题型的累计量。
#[derive(Debug, Default)]
struct Bucket {
    cases: usize,
    gold_len: BTreeMap<usize, usize>,
    base_hits: usize,
    alpha_hits: Vec<usize>,
    pool_has_all_gold: usize,
    pos_gap_sum: f64,
    pos_gap_cases: usize,
    gold_terms: BTreeMap<String, TermTally>,
    other_terms: BTreeMap<String, TermTally>,
    samples: Vec<String>,
}

/// 题面里最早命中正文的那个词（与生产 `first_pos_score` 同一套 `find` 口径）。
fn earliest_match(query_terms: &[String], doc: &str) -> Option<(String, usize)> {
    let mut best: Option<(String, usize)> = None;
    for term in query_terms {
        if let Some(at) = doc.find(term.as_str()) {
            match &best {
                Some((_, position)) if *position <= at => {}
                _ => best = Some((term.clone(), at)),
            }
        }
    }
    best
}

/// 生产同款位置分：`1/(1+pos)`；没命中返回 0。
fn first_pos_value(query_terms: &[String], doc: &str) -> f64 {
    let mut position = f64::MAX;
    for term in query_terms {
        if let Some(at) = doc.find(term.as_str()) {
            position = position.min(at as f64);
        }
    }
    if position.is_finite() {
        1.0 / (1.0 + position)
    } else {
        0.0
    }
}

fn tally(map: &mut BTreeMap<String, TermTally>, term: &str, position: usize) {
    let entry = map.entry(term.to_string()).or_default();
    entry.count += 1;
    entry.pos_sum += position;
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
    let mut all_rows: Vec<serde_json::Value> = Vec::new();

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

        let mined = mine(
            &records,
            MiningConfig {
                max_cases_per_record: MAX_CASES_PER_RECORD,
                ..MiningConfig::default()
            },
        );
        let set = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));
        eprintln!(
            "· {name}：语料 {} 段 · 模板词面卷 {} 题",
            index.len(),
            set.cases.len()
        );

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
        let content: HashMap<&str, &str> = index
            .iter()
            .map(|record| (record.id.as_str(), record.text.as_str()))
            .collect();

        // 分桶：题型（挖题器 kind）
        let mut buckets: HashMap<String, Bucket> = HashMap::new();

        for case in &set.cases {
            let kind = case
                .tags
                .iter()
                .find_map(|tag| tag.strip_prefix("kind:"))
                .unwrap_or("unknown")
                .to_string();

            let candidates = bm25.candidates(&case.query, POOL)?;
            let positive: Vec<_> = candidates.iter().filter(|hit| hit.score > 0.0).collect();
            if positive.is_empty() {
                continue;
            }
            let gold: HashSet<&str> = case.expect_ids.iter().map(String::as_str).collect();

            let query_terms = tokenize(&normalize_for_index(&case.query));
            let max_base = positive
                .iter()
                .map(|hit| hit.score)
                .fold(0.0_f64, f64::max)
                .max(1e-12);

            // 每个候选的 (id, 归一化基线分, 归一化位置分, 最早命中词)
            let mut values: Vec<(&str, f64, f64, Option<(String, usize)>)> = positive
                .iter()
                .map(|hit| {
                    let base = hit.score / max_base;
                    let doc = content.get(hit.id.as_str()).copied().unwrap_or("");
                    let raw = first_pos_value(&query_terms, doc);
                    (hit.id.as_str(), base, raw, earliest_match(&query_terms, doc))
                })
                .collect();
            let max_value = values.iter().map(|item| item.2).fold(0.0_f64, f64::max);
            if max_value > 0.0 {
                for item in values.iter_mut() {
                    item.2 /= max_value;
                }
            }

            // 方向指标：池内答案候选与非答案候选的位置分均值差
            let mut gold_pos: Vec<f64> = Vec::new();
            let mut other_pos: Vec<f64> = Vec::new();
            for (id, _, value, _) in values.iter() {
                if gold.contains(id) {
                    gold_pos.push(*value);
                } else {
                    other_pos.push(*value);
                }
            }
            let pos_gap = if !gold_pos.is_empty() && !other_pos.is_empty() {
                let mean_gold = gold_pos.iter().sum::<f64>() / gold_pos.len() as f64;
                let mean_other = other_pos.iter().sum::<f64>() / other_pos.len() as f64;
                Some(mean_gold - mean_other)
            } else {
                None
            };

            // 基线排序（前 K）
            let mut baseline: Vec<(&str, f64)> =
                values.iter().map(|(id, base, _, _)| (*id, *base)).collect();
            baseline.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(b.0))
            });
            let base_ids: Vec<&str> = baseline.iter().take(K).map(|item| item.0).collect();
            let in_top = |ids: &[&str]| -> bool {
                !gold.is_empty() && gold.iter().all(|id| ids.contains(id))
            };
            let base_hit = usize::from(in_top(&base_ids));

            // 最早命中词统计（只看基线 top-10）
            for (id, _, _, earliest) in values.iter() {
                if !base_ids.contains(id) {
                    continue;
                }
                if let Some((term, position)) = earliest {
                    if gold.contains(id) {
                        tally(&mut buckets.entry(kind.clone()).or_default().gold_terms, term, *position);
                    } else {
                        tally(
                            &mut buckets.entry(kind.clone()).or_default().other_terms,
                            term,
                            *position,
                        );
                    }
                }
            }

            // 各 alpha 臂
            let mut alpha_hits = vec![0usize; ALPHAS.len()];
            for (alpha_index, alpha) in ALPHAS.iter().copied().enumerate() {
                let mut ranked: Vec<(&str, f64)> = values
                    .iter()
                    .map(|(id, base, value, _)| (*id, base + alpha * value))
                    .collect();
                ranked.sort_by(|a, b| {
                    b.1.partial_cmp(&a.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.0.cmp(b.0))
                });
                let ids: Vec<&str> = ranked.iter().take(K).map(|item| item.0).collect();
                if in_top(&ids) {
                    alpha_hits[alpha_index] += 1;
                }
            }

            let bucket = buckets.entry(kind).or_default();
            bucket.cases += 1;
            *bucket.gold_len.entry(gold.len()).or_default() += 1;
            bucket.base_hits += base_hit;
            for (index, hit) in alpha_hits.iter().enumerate() {
                if index >= bucket.alpha_hits.len() {
                    bucket.alpha_hits.resize(ALPHAS.len(), 0);
                }
                bucket.alpha_hits[index] += *hit;
            }
            let pool_ids: Vec<&str> = values.iter().map(|item| item.0).collect();
            if in_top(&pool_ids) {
                bucket.pool_has_all_gold += 1;
            }
            if let Some(gap) = pos_gap {
                bucket.pos_gap_sum += gap;
                bucket.pos_gap_cases += 1;
            }
            if bucket.samples.len() < 3 {
                bucket.samples.push(format!(
                    "{}｜标准答案 {} 条｜基线命中={}｜α0.10命中={}",
                    case.query,
                    gold.len(),
                    base_hit,
                    alpha_hits[1]
                ));
            }
        }

        let mut keys: Vec<String> = buckets.keys().cloned().collect();
        keys.sort();
        for key in keys {
            let bucket = &buckets[&key];
            let denominator = bucket.cases.max(1) as f64;
            let base_recall = bucket.base_hits as f64 / denominator;
            let pool_rate = bucket.pool_has_all_gold as f64 / denominator;
            let pos_gap = if bucket.pos_gap_cases > 0 {
                bucket.pos_gap_sum / bucket.pos_gap_cases as f64
            } else {
                f64::NAN
            };

            println!("· {name} · 题型 {key}：题 {} · 基线 {:.4}", bucket.cases, base_recall);
            println!(
                "     标准答案条数分布 {:?} · 池内全可达率 {:.4}",
                bucket.gold_len, pool_rate
            );
            println!("     位置分方向 pos_gap {:+.4}（正 = 偏向答案）", pos_gap);
            let mut arms = Vec::new();
            for (alpha_index, alpha) in ALPHAS.iter().copied().enumerate() {
                let hits = bucket.alpha_hits.get(alpha_index).copied().unwrap_or(0);
                let recall = hits as f64 / denominator;
                println!(
                    "     α={:<5} recall {:.4} ({:+.4})",
                    alpha,
                    recall,
                    recall - base_recall
                );
                arms.push(serde_json::json!({
                    "alpha": alpha,
                    "recall": recall,
                    "delta": recall - base_recall,
                }));
            }
            println!("     最早命中词 · 答案候选：");
            for (term, tally) in top_terms(&bucket.gold_terms, 6) {
                println!(
                    "       {:<12} 次数 {:<5} 平均首次位置 {:.1}",
                    term,
                    tally.count,
                    tally.pos_sum as f64 / tally.count as f64
                );
            }
            println!("     最早命中词 · 非答案候选：");
            for (term, tally) in top_terms(&bucket.other_terms, 6) {
                println!(
                    "       {:<12} 次数 {:<5} 平均首次位置 {:.1}",
                    term,
                    tally.count,
                    tally.pos_sum as f64 / tally.count as f64
                );
            }
            println!("     样例：");
            for sample in &bucket.samples {
                println!("       {sample}");
            }

            all_rows.push(serde_json::json!({
                "语料": name,
                "题型": key,
                "题数": bucket.cases,
                "标准答案条数分布": bucket.gold_len,
                "基线recall": base_recall,
                "池内全可达率": pool_rate,
                "位置分方向pos_gap": pos_gap,
                "各alpha": arms,
                "最早命中词_答案": top_terms_json(&bucket.gold_terms, 6),
                "最早命中词_非答案": top_terms_json(&bucket.other_terms, 6),
            }));
        }
    }

    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "考卷": "模板词面卷",
            "分桶": all_rows,
        }))?
    );
    Ok(())
}

/// 取出现次数最多的几个词。
fn top_terms(map: &BTreeMap<String, TermTally>, limit: usize) -> Vec<(String, TermTally)> {
    let mut items: Vec<(String, TermTally)> = map
        .iter()
        .map(|(term, tally)| (term.clone(), tally.clone()))
        .collect();
    items.sort_by(|a, b| b.1.count.cmp(&a.1.count).then_with(|| a.0.cmp(&b.0)));
    items.truncate(limit);
    items
}

fn top_terms_json(map: &BTreeMap<String, TermTally>, limit: usize) -> Vec<serde_json::Value> {
    top_terms(map, limit)
        .into_iter()
        .map(|(term, tally)| {
            serde_json::json!({
                "词": term,
                "次数": tally.count,
                "平均首次位置": tally.pos_sum as f64 / tally.count as f64,
            })
        })
        .collect()
}
