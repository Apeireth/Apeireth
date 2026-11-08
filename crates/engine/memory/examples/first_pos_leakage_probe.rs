//! **`first_pos` 的「标签泄漏」探针**（2026-11-08）—— 决定「+0.12 是不是我们自己造的」。
//!
//! # 要回答的问题
//! `rerank_headroom_ab`（改成**生产口径**后）实测：
//!
//! | 考卷 | `first_pos` α=1.00 的 delta |
//! |---|---|
//! | 公平卷 / redchamber | **+0.1202** |
//! | 公平卷 / watershed | **+0.0806** |
//! | 词面卷 / redchamber | −0.0231 |
//! | 词面卷 / watershed | −0.0323 |
//! | 真人查询卷（MIRACL）| **+0.0192**（闸门口径，同口径）|
//!
//! ⇒ ⇒ **公平卷上 +0.12，比真人卷大 6 倍** ⇒ ⇒ 必须先排除一件事：
//! **是不是我们的考卷「恰好把标准答案放在最早出现的位置」？**
//!
//! # 为什么这是真问题
//! 我们的两套考卷都是**零人工标注挖出来的**：
//! * **公平卷**：题面 = `关于{主体}，我们当时记了什么？`，
//!   标准答案 = 「**该主体的联想实体集**里有这个主体的那条记录」；
//! * **词面卷**：题面 = 5 个模板，标准答案 = 挖题器挑的记录。
//!
//! ⇒ ⇒ **如果这些标签的挑选规则，恰好偏好「主体最早出现」的记录**
//! ⇒ ⇒ **`first_pos` 就是在「读标签的构造规则」**，不是产品收益。
//!
//! # 决定性设计：**同一把探针，量三套卷**
//!
//! | 卷 | 标签来源 | 期望 |
//! |---|---|---|
//! | 词面卷 | 我们挖的（`golden_mining`）| 若「金标最早」比例很高 ⇒ 构造产物 |
//! | 公平卷 | 我们挖的（`golden_fair`）| 同上 |
//! | **真人卷** | **MIRACL 官方人工标注**（与任何位置启发式无关）| **基准**：若这里也高 ⇒ 是语料性质；若这里低 ⇒ 是我们的标签构造 |
//!
//! # 口径
//! * 查询词 = **生产口径** `tokenize(normalize_for_index(query))`；
//! * 候选池 = BM25 前 50 且分数 > 0；
//! * 位置分 = `1/(1+pos)`，`pos` = 题面词在正文里首次出现的字符位置（取最小）；
//! * 只在**池里至少有一个候选含题面词**的题上统计（「有可比对象」的题）；
//! * 「金标最早」= 金标在「含题面词的池内候选」里，位置分**最高**（即位置最靠前）。

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

const POOL: usize = 50;
const MAX_CASES_PER_RECORD: usize = 6;

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

/// 一套卷的探针结果。
#[derive(Default)]
struct Probe {
    /// 总题数。
    cases: usize,
    /// 标准答案在候选池里的题数。
    gold_in_pool: usize,
    /// 「有可比对象」的题数（池里至少一个候选含题面词）。
    comparable: usize,
    /// 标准答案在可比对象里「位置最靠前」的题数。
    gold_earliest: usize,
    /// 标准答案是 BM25 第一名的题数。
    gold_bm25_first: usize,
    /// 标准答案按「位置分」排序的名次之和（1 = 最靠前）。
    position_rank_sum: usize,
    /// 可比对象数之和（用来算「随机期望名次」）。
    comparable_count_sum: usize,
}

impl Probe {
    fn report(&self, label: &str) {
        let comparable = self.comparable.max(1);
        let expected_rank = self.comparable_count_sum as f64 / comparable as f64;
        let observed_rank = self.position_rank_sum as f64 / comparable as f64;
        let earliest_ratio = self.gold_earliest as f64 / comparable as f64;
        let bm25_first_ratio = self.gold_bm25_first as f64 / comparable as f64;
        println!("· {label}");
        println!(
            "     题 {} · 金标在池里 {}/{} · 可比题 {}",
            self.cases, self.gold_in_pool, self.cases, self.comparable
        );
        println!(
            "     金标位置最靠前 {:.1}% · 金标 BM25 第一 {:.1}%",
            earliest_ratio * 100.0,
            bm25_first_ratio * 100.0
        );
        println!(
            "     金标按位置分的名次：实测均值 {:.2} · 随机期望 {:.2}",
            observed_rank, expected_rank
        );
    }

    fn to_json(&self, label: &str) -> serde_json::Value {
        let comparable = self.comparable.max(1) as f64;
        serde_json::json!({
            "卷": label,
            "题数": self.cases,
            "金标在池里": self.gold_in_pool,
            "可比题数": self.comparable,
            "金标位置最靠前比例": self.gold_earliest as f64 / comparable,
            "金标BM25第一比例": self.gold_bm25_first as f64 / comparable,
            "金标按位置分名次均值": self.position_rank_sum as f64 / comparable,
            "随机期望名次": self.comparable_count_sum as f64 / comparable,
        })
    }
}

/// 跑一遍探针（**按题型分组**，因为「金标是主体本身」与「金标是顺带提及」是两种标签规则）。
fn probe_by_kind(
    bm25: &Bm25LexicalCandidateSource,
    content: &HashMap<&str, &str>,
    cases: &[GoldenCase],
) -> Result<(HashMap<String, Probe>, Probe), Box<dyn Error>> {
    let mut grouped: HashMap<String, Vec<GoldenCase>> = HashMap::new();
    let mut all: Vec<GoldenCase> = Vec::new();
    for case in cases {
        let kind = case
            .tags
            .iter()
            .find_map(|tag| tag.strip_prefix("kind:"))
            .unwrap_or("unknown")
            .to_string();
        grouped.entry(kind).or_default().push(case.clone());
        all.push(case.clone());
    }
    let mut by_kind = HashMap::new();
    for (kind, subset) in grouped {
        by_kind.insert(kind.clone(), probe(bm25, content, &subset)?);
    }
    let overall = probe(bm25, content, &all)?;
    Ok((by_kind, overall))
}

/// 跑一遍探针。
fn probe(
    bm25: &Bm25LexicalCandidateSource,
    content: &HashMap<&str, &str>,
    cases: &[GoldenCase],
) -> Result<Probe, Box<dyn Error>> {
    let mut result = Probe::default();
    for case in cases {
        result.cases += 1;
        let hits = bm25.candidates(&case.query, POOL)?;
        let positive: Vec<_> = hits.iter().filter(|hit| hit.score > 0.0).collect();
        if positive.is_empty() {
            continue;
        }
        let gold: HashSet<&str> = case.expect_ids.iter().map(String::as_str).collect();
        let pool_ids: Vec<&str> = positive.iter().map(|hit| hit.id.as_str()).collect();
        if gold.iter().all(|id| pool_ids.contains(id)) {
            result.gold_in_pool += 1;
        }

        let terms = tokenize(&normalize_for_index(&case.query));
        if terms.is_empty() {
            continue;
        }
        // 只保留「含题面词」的候选（可比对象）。
        let mut comparable: Vec<(&str, f64)> = Vec::new();
        for hit in &positive {
            let doc = content.get(hit.id.as_str()).copied().unwrap_or("");
            let mut position = f64::MAX;
            for term in &terms {
                if let Some(at) = doc.find(term.as_str()) {
                    position = position.min(at as f64);
                }
            }
            if position.is_finite() {
                comparable.push((hit.id.as_str(), position));
            }
        }
        if comparable.is_empty() {
            continue;
        }
        result.comparable += 1;
        result.comparable_count_sum += comparable.len();
        // 按位置升序（位置越靠前 ⇒ 位置分越高）。
        comparable.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(b.0))
        });
        if let Some(rank) = comparable
            .iter()
            .position(|(id, _)| gold.contains(*id))
            .map(|index| index + 1)
        {
            result.position_rank_sum += rank;
            if rank == 1 {
                result.gold_earliest += 1;
            }
        }
        if positive
            .first()
            .map(|hit| gold.contains(hit.id.as_str()))
            .unwrap_or(false)
        {
            result.gold_bm25_first += 1;
        }
    }
    Ok(result)
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
    let mut rows: Vec<serde_json::Value> = Vec::new();

    // ---------- 真人查询卷（MIRACL：标签 = 官方人工标注）----------
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
                tags: vec!["miracl".to_string()],
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
        let result = probe(&bm25, &content, &cases)?;
        println!("=== ⭐ 真人查询卷（MIRACL 中文 dev · 标签 = **官方人工标注**）===");
        result.report("真人查询卷");
        rows.push(result.to_json("真人查询卷"));
    } else {
        println!("=== 真人查询卷：考卷不在 ⇒ 跳过（这不是通过）===");
    }

    // ---------- 我们的两套卷（标签 = 零标注挖出来的）----------
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

        // 词面卷
        let mined = mine(
            &records,
            MiningConfig {
                max_cases_per_record: MAX_CASES_PER_RECORD,
                ..MiningConfig::default()
            },
        );
        let lexical = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));
        let (by_kind, overall) = probe_by_kind(&bm25, &content, &lexical.cases)?;
        println!("=== 词面卷 / {name}（标签 = `golden_mining` 挖的）===");
        overall.report(&format!("词面卷 {name} 全部"));
        rows.push(overall.to_json(&format!("词面卷 {name} 全部")));
        let mut kinds: Vec<String> = by_kind.keys().cloned().collect();
        kinds.sort();
        for kind in kinds {
            let item = &by_kind[&kind];
            println!("  --- 题型 {kind} ---");
            item.report(&format!("词面卷 {name} · {kind}"));
            rows.push(item.to_json(&format!("词面卷 {name} · {kind}")));
        }

        // 公平卷
        let fair = GoldenSet::new(
            format!("fair-{name}"),
            build_fair_exam(
                &records,
                FairExamConfig {
                    max_cases_per_record: MAX_CASES_PER_RECORD,
                    ..FairExamConfig::default()
                },
            ),
        );
        let result = probe(&bm25, &content, &fair.cases)?;
        println!("=== 公平卷 / {name}（标签 = `golden_fair` 挖的）===");
        result.report(&format!("公平卷 {name}"));
        rows.push(result.to_json(&format!("公平卷 {name}")));
    }

    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "标签泄漏探针": rows }))?
    );
    Ok(())
}
