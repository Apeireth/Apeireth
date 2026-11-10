//! 「首命中位置」三个口径臂的**对照实测**（2026-11-08）—— 两套考卷同时量。
//!
//! # ⭐⭐ 本例要修的头号问题：**指标口径混用**
//! 此前「`first_pos` 在真实查询上 **+0.1227**」这个数字是**混口径减法**：
//!
//! | 数字 | 它是什么 | 口径 |
//! |---|---|---|
//! | 基线 0.7246 | `golden_run` 的 `metrics.recall_at_k` | **逐题「命中比例」再平均**（`\|expect ∩ top-k\| / \|expect\|`） |
//! | α=0.10 的 0.8473 | `miracl_exam_ab` 里的 α 扫描臂 | **二值「任一命中」**（`gold.iter().any(...)`） |
//!
//!；两个**不同的指标**相减；**+0.1227 无效**。
//!；本例**三种口径同时报**，且**以闸门自己的口径为主**：
//!
//! | 口径 | 定义 | 用途 |
//! |---|---|---|
//! | **闸门口径** | 逐题「命中比例」再平均（= `golden_run::metrics.recall_at_k`） | **闸门判红/判绿用这个** |
//! | 任一命中 | 二值：top-k 里碰到任意一个标准答案 | 粗看 |
//! | 全部命中 | 二值：top-k 里含**全部**标准答案 | 粗看 |
//!
//! # 背景（已实测的机制，见 `first_pos_mechanism.rs`）
//! 1. **模板卷 51%~68% 的题是 `conflict` 题型**（「{主体}后来是怎么改的？」），
//!    而这类题里 **44%（redchamber）/ 27%（watershed）的标准答案根本不在 BM25 前 50 的池里**
//!   ；**任何排序特征都救不了**；模板卷这把尺子**测的是池子大小，不是排序质量**；
//! 2. 在最典型的「记忆题」（「关于{主体}，我们当时记了什么？」）上，
//!    **答案候选与非答案候选的「最早命中位置」几乎完全相同**
//!    （redchamber 两边都是「寶玉 @ 145.1」）； 位置分**没有区分度**
//!   ；它只在基线分数接近的候选之间**按位置乱序**；净掉分。
//!
//! # 本例测什么
//! **位置分该由哪些词决定**？三个臂，除 alpha 外**其余变量全同**：
//!
//! | 臂 | 位置分由哪些词的首次出现决定 |
//! |---|---|
//! | A「现状」 | 全部 token（单字 + bigram）—— **生产现状** |
//! | B「只用长词」 | 只有长度 >= 2 的 token（去掉单字构件） |
//! | C「最判别词」 | 只有**池内命中最少**的那个 token（近似 IDF 最高者） |
//!
//! # 口径（引用数字时必须一起引用）
//! * 实现与生产**逐字一致**：`tokenize(normalize_for_index(query))` 取词，
//!   `content.find(term)` 取首次字符位置，位置分 `1/(1+pos)`，按题归一化到 (0,1]，
//!   最终分 `base + alpha * 位置分`；`base` = **生产排序分**（BM25 + 覆盖率融合）；
//! * **k=10，候选池 50**；
//! * 模板卷另外按**池内可达 / 不可达**拆开报。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

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
const ALPHAS: [f64; 2] = [0.05, 0.10];
const ARM_COUNT: usize = 3;
const ALPHA_COUNT: usize = 2;
const ARM_NAMES: [&str; ARM_COUNT] = ["A 现状(全词)", "B 只用长词", "C 最判别词"];

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

/// 一道题的评测结果。
struct Outcome {
    /// 闸门口径：逐题「命中比例」。
    base_frac: f64,
    base_any: bool,
    base_all: bool,
    arm_frac: [[f64; ALPHA_COUNT]; ARM_COUNT],
    arm_any: [[bool; ALPHA_COUNT]; ARM_COUNT],
    arm_all: [[bool; ALPHA_COUNT]; ARM_COUNT],
    /// 标准答案是否全在候选池里？
    reachable: bool,
}

/// 一个分桶的累计量。
#[derive(Default)]
struct Tally {
    cases: usize,
    base_frac_sum: f64,
    base_any: usize,
    base_all: usize,
    arm_frac_sum: [[f64; ALPHA_COUNT]; ARM_COUNT],
    arm_any: [[usize; ALPHA_COUNT]; ARM_COUNT],
    arm_all: [[usize; ALPHA_COUNT]; ARM_COUNT],
}

impl Tally {
    fn add(&mut self, outcome: &Outcome) {
        self.cases += 1;
        self.base_frac_sum += outcome.base_frac;
        self.base_any += usize::from(outcome.base_any);
        self.base_all += usize::from(outcome.base_all);
        for arm in 0..ARM_COUNT {
            for alpha in 0..ALPHA_COUNT {
                self.arm_frac_sum[arm][alpha] += outcome.arm_frac[arm][alpha];
                self.arm_any[arm][alpha] += usize::from(outcome.arm_any[arm][alpha]);
                self.arm_all[arm][alpha] += usize::from(outcome.arm_all[arm][alpha]);
            }
        }
    }
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

/// 位置分原始值：`1/(1+pos)`；没命中返回 0。
fn position_value(terms: &[String], doc: &str) -> f64 {
    let mut position = f64::MAX;
    for term in terms {
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

fn evaluate(
    bm25: &Bm25LexicalCandidateSource,
    content: &HashMap<&str, &str>,
    query: &str,
    gold: &HashSet<&str>,
) -> Result<Outcome, Box<dyn Error>> {
    let hits = bm25.candidates(query, POOL)?;
    let positive: Vec<_> = hits.iter().filter(|hit| hit.score > 0.0).collect();
    if positive.is_empty() || gold.is_empty() {
        return Ok(Outcome {
            base_frac: 0.0,
            base_any: false,
            base_all: false,
            arm_frac: [[0.0; ALPHA_COUNT]; ARM_COUNT],
            arm_any: [[false; ALPHA_COUNT]; ARM_COUNT],
            arm_all: [[false; ALPHA_COUNT]; ARM_COUNT],
            reachable: false,
        });
    }
    let max_base = positive
        .iter()
        .map(|hit| hit.score)
        .fold(0.0_f64, f64::max)
        .max(1e-12);

    let tokens = tokenize(&normalize_for_index(query));
    let long_tokens: Vec<String> = tokens
        .iter()
        .filter(|term| term.chars().count() >= 2)
        .cloned()
        .collect();

    // 池内命中最少的 token（近似 IDF 最高者）
    let mut df: HashMap<&str, usize> = HashMap::new();
    for term in &tokens {
        let count = positive
            .iter()
            .filter(|hit| {
                content
                    .get(hit.id.as_str())
                    .copied()
                    .unwrap_or("")
                    .contains(term.as_str())
            })
            .count();
        df.insert(term.as_str(), count);
    }
    let mut discriminative: Option<&String> = None;
    for term in &tokens {
        let count = df.get(term.as_str()).copied().unwrap_or(0);
        if count == 0 {
            continue;
        }
        let better = match discriminative {
            None => true,
            Some(current) => {
                let current_count = df.get(current.as_str()).copied().unwrap_or(0);
                (count, std::cmp::Reverse(current.chars().count()))
                    < (current_count, std::cmp::Reverse(term.chars().count()))
            }
        };
        if better {
            discriminative = Some(term);
        }
    }

    // 每个臂的 (基线分, 位置分, id)
    let mut arm_values: [Vec<(f64, f64, &str)>; ARM_COUNT] = [Vec::new(), Vec::new(), Vec::new()];
    for hit in &positive {
        let doc = content.get(hit.id.as_str()).copied().unwrap_or("");
        let base = hit.score / max_base;
        let arm_a = position_value(&tokens, doc);
        let arm_b = position_value(&long_tokens, doc);
        let arm_c = discriminative
            .and_then(|term| doc.find(term.as_str()))
            .map(|at| 1.0 / (1.0 + at as f64))
            .unwrap_or(0.0);
        arm_values[0].push((base, arm_a, hit.id.as_str()));
        arm_values[1].push((base, arm_b, hit.id.as_str()));
        arm_values[2].push((base, arm_c, hit.id.as_str()));
    }

    // 位置分按题归一化到 (0,1]
    for arm in 0..ARM_COUNT {
        let max_value = arm_values[arm]
            .iter()
            .map(|item| item.1)
            .fold(0.0_f64, f64::max);
        if max_value > 0.0 {
            for item in arm_values[arm].iter_mut() {
                item.1 /= max_value;
            }
        }
    }

    let fraction = |ids: &[&str]| -> f64 {
        if gold.is_empty() {
            return 0.0;
        }
        let found = gold.iter().filter(|id| ids.contains(id)).count();
        found as f64 / gold.len() as f64
    };

    // 基线排序
    let mut base_ranked: Vec<(&str, f64)> = arm_values[0]
        .iter()
        .map(|(base, _, id)| (*id, *base))
        .collect();
    base_ranked.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });
    let base_ids: Vec<&str> = base_ranked.iter().take(K).map(|item| item.0).collect();
    let pool_ids: Vec<&str> = arm_values[0].iter().map(|item| item.2).collect();

    let mut outcome = Outcome {
        base_frac: fraction(&base_ids),
        base_any: base_ids.iter().any(|id| gold.contains(id)),
        base_all: gold.iter().all(|id| base_ids.contains(id)),
        arm_frac: [[0.0; ALPHA_COUNT]; ARM_COUNT],
        arm_any: [[false; ALPHA_COUNT]; ARM_COUNT],
        arm_all: [[false; ALPHA_COUNT]; ARM_COUNT],
        reachable: gold.iter().all(|id| pool_ids.contains(id)),
    };

    for arm in 0..ARM_COUNT {
        for (alpha_index, alpha) in ALPHAS.iter().copied().enumerate() {
            let mut ranked: Vec<(&str, f64)> = arm_values[arm]
                .iter()
                .map(|(base, value, id)| (*id, base + alpha * value))
                .collect();
            ranked.sort_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.0.cmp(b.0))
            });
            let ids: Vec<&str> = ranked.iter().take(K).map(|item| item.0).collect();
            outcome.arm_frac[arm][alpha_index] = fraction(&ids);
            outcome.arm_any[arm][alpha_index] = ids.iter().any(|id| gold.contains(id));
            outcome.arm_all[arm][alpha_index] = gold.iter().all(|id| ids.contains(id));
        }
    }
    Ok(outcome)
}

fn report(label: &str, tally: &Tally, rows: &mut Vec<serde_json::Value>) {
    let denominator = tally.cases.max(1) as f64;
    let base_frac = tally.base_frac_sum / denominator;
    let base_any = tally.base_any as f64 / denominator;
    let base_all = tally.base_all as f64 / denominator;
    println!("· {label}：题 {}", tally.cases);
    println!(
        "     基线：闸门口径（逐题命中比例）{:.4} · 任一命中 {:.4} · 全部命中 {:.4}",
        base_frac, base_any, base_all
    );
    let mut arms_json = Vec::new();
    for arm in 0..ARM_COUNT {
        for alpha_index in 0..ALPHA_COUNT {
            let alpha = ALPHAS[alpha_index];
            let frac = tally.arm_frac_sum[arm][alpha_index] / denominator;
            let any = tally.arm_any[arm][alpha_index] as f64 / denominator;
            let all = tally.arm_all[arm][alpha_index] as f64 / denominator;
            println!(
                "     {:<12} α={:<5} 闸门口径 {:.4} ({:+.4}) · 任一命中 {:.4} ({:+.4}) · 全部命中 {:.4} ({:+.4})",
                ARM_NAMES[arm],
                alpha,
                frac,
                frac - base_frac,
                any,
                any - base_any,
                all,
                all - base_all
            );
            arms_json.push(serde_json::json!({
                "臂": ARM_NAMES[arm],
                "alpha": alpha,
                "闸门口径recall": frac,
                "闸门口径delta": frac - base_frac,
                "任一命中recall": any,
                "任一命中delta": any - base_any,
                "全部命中recall": all,
                "全部命中delta": all - base_all,
            }));
        }
    }
    rows.push(serde_json::json!({
        "分桶": label,
        "题数": tally.cases,
        "基线_闸门口径": base_frac,
        "基线_任一命中": base_any,
        "基线_全部命中": base_all,
        "各臂": arms_json,
    }));
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut rows: Vec<serde_json::Value> = Vec::new();

    // ---------- 真实查询卷（MIRACL 中文 dev） ----------
    let root = exam_root();
    if (root.join("corpus.jsonl")).exists() && (root.join("exam.jsonl")).exists() {
        let mut index: Vec<IndexRecord> = Vec::new();
        let mut content_map: HashMap<String, String> = HashMap::new();
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
            content_map.insert(id.clone(), joined.clone());
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
                tags: Vec::new(),
            });
        }
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
        let content: HashMap<&str, &str> = content_map
            .iter()
            .map(|(id, text)| (id.as_str(), text.as_str()))
            .collect();
        let mut tally = Tally::default();
        for case in &cases {
            let gold: HashSet<&str> = case.expect_ids.iter().map(String::as_str).collect();
            tally.add(&evaluate(&bm25, &content, &case.query, &gold)?);
        }
        println!(
            "=== 真实查询卷（MIRACL 中文 dev · {} 题 · 池 {} 段 · k={K}）===",
            tally.cases,
            index.len()
        );
        report("真实查询卷", &tally, &mut rows);
    } else {
        println!(
            "=== 真实查询卷：考卷不在（{}）， 跳过（这不是通过）===",
            root.display()
        );
    }

    // ---------- 模板词面卷（按题型 + 按可达性拆） ----------
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

        let mut overall = Tally::default();
        let mut by_kind: HashMap<String, Tally> = HashMap::new();
        let mut reachable: HashMap<String, Tally> = HashMap::new();
        for case in &set.cases {
            let kind = case
                .tags
                .iter()
                .find_map(|tag| tag.strip_prefix("kind:"))
                .unwrap_or("unknown")
                .to_string();
            let gold: HashSet<&str> = case.expect_ids.iter().map(String::as_str).collect();
            let outcome = evaluate(&bm25, &content, &case.query, &gold)?;
            overall.add(&outcome);
            by_kind.entry(kind.clone()).or_default().add(&outcome);
            if outcome.reachable {
                reachable.entry(kind).or_default().add(&outcome);
            }
        }

        println!("=== 模板词面卷（{name} · {} 题）===", overall.cases);
        report(&format!("模板卷 {name} 全部"), &overall, &mut rows);
        let mut kinds: Vec<String> = by_kind.keys().cloned().collect();
        kinds.sort();
        for kind in kinds {
            let tally = &by_kind[&kind];
            let reach_note = match reachable.get(&kind) {
                Some(item) => format!(
                    " · 可达子集 {} 题（{:.0}%）",
                    item.cases,
                    item.cases as f64 / tally.cases.max(1) as f64 * 100.0
                ),
                None => String::new(),
            };
            report(
                &format!("模板卷 {name} · 题型 {kind}{reach_note}"),
                tally,
                &mut rows,
            );
            if let Some(item) = reachable.get(&kind) {
                report(
                    &format!("模板卷 {name} · 题型 {kind} · 仅池内可达题"),
                    item,
                    &mut rows,
                );
            }
        }
    }

    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "对照": rows }))?
    );
    Ok(())
}
