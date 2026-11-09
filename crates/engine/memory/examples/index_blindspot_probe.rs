//! **索引盲区探针**（2026-11-09）—— 「词在文里，但索引里没有」到底占多少、是哪一类。
//!
//! # 与既有探针的关系
//!
//! `examples/lexical_blindspot_probe.rs`（**别人的资产，不改**）已经把内部卷的
//! 词法盲区分成「空题面 / 词在文里但索引漏了 / 词真的不在文里 / 只有单字命中」。
//! ⇒ 本探针**复用它的口径与分类思路**，但补三件事：
//!
//! 1. **三把尺子都跑**（真实查询卷 + 两套内部卷）—— 既有探针只跑内部卷；
//! 2. **分类细化到 (a)~(e)**（见下）—— 把「词在文里但索引漏了」拆成**可归因**的子类；
//! 3. **同时报「旧口径 vs 新口径」**（中文单字是否入索引）—— 量化修复前后。
//!
//! # 缺陷类的分类口径
//!
//! | 代码 | 含义 | 判据 |
//! |---|---|---|
//! | **(a) 分词口径不一致** | 查询侧与索引侧用了不同的分词/归一 | 题面**多字**词是正文**子串**，但索引里没有这个词 |
//! | **(b) 单字/停用字过滤** | 单字（或被停用字拆开的词）把答案词滤掉了 | 题面与正文**只共享单字** |
//! | **(c) 段落切分截断** | 答案在同一文档的**兄弟段**里，不在金标指向的那段 | 同父文档的其它段能命中 |
//! | **(d) 索引根本没建** | 金标 id 压根不在索引里 | 金标 id 不在索引表 |
//! | **(e) 其它** | 题面词**真的不在**正文里（题面与金标不匹配）等 | 兜底 |
//!
//! # 「可达率」的口径
//!
//! * **盲区金标** = 该金标在 BM25 下**一分都拿不到**（`score == 0`）；
//! * **可达** ⟺ 题面词与正文词**有交集**（BM25 给分 ⟺ 至少一个词命中）⇒
//!   ⇒ 可以**只靠词集合**判定可达性（与真跑一次 BM25 **逐字等价**）；
//! * **闸门口径可达上限** = 逐题「可达金标数 ÷ 该题金标数」再按题平均。
//!
//! # 用法
//!
//! ```sh
//! APEIRETH_DATA_DIR=/home/bai/.apeireth cargo run -p apeireth-memory --example index_blindspot_probe
//! ```
//!
//! # 中性表述
//!
//! 只描述**机制**（分词粒度 / 索引口径 / 候选可达性）；数据集名只出现在**数据集口径**处。

use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_mining::{mine, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, run as run_golden, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::hybrid_search::{normalize_for_index, tokenize, tokenize_with_unigrams};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

/// 评测的 top-k（与内部卷基线闸门同口径）。
const K: usize = 10;
/// 内部卷挖题预算（与既有闸门**逐项一致** ⇒ 题数可比）。
const MAX_CASES_PER_RECORD: usize = 6;
/// 每个分类最多打印几条样例。
const SAMPLES_PER_CLASS: usize = 4;

/// 一个盲区金标的归因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Cause {
    /// (a) 查询侧与索引侧口径不一致（多字词在文里、索引里没有）。
    TokenizerMismatch,
    /// (b) 只共享单字（单字粒度把答案词滤掉了）。
    SingleCharOnly,
    /// (c) 答案在同一文档的兄弟段里（金标指向的那段被切掉了）。
    TruncatedSegment,
    /// (d) 金标压根没进索引。
    IndexMissing,
    /// (e) 其它（题面词真的不在正文里 / 题面被规范化成空）。
    Other,
}

impl Cause {
    fn label(self) -> &'static str {
        match self {
            Cause::TokenizerMismatch => "(a) 分词口径不一致",
            Cause::SingleCharOnly => "(b) 只共享单字（单字粒度）",
            Cause::TruncatedSegment => "(c) 段落切分把答案截掉",
            Cause::IndexMissing => "(d) 索引根本没建",
            Cause::Other => "(e) 其它（题面词不在正文里）",
        }
    }
}

/// 一套考卷（题 + 可检索文本）。
struct Ruler {
    label: String,
    role: String,
    cases: Vec<GoldenCase>,
    index: Vec<IndexRecord>,
}

/// 两套口径下的可达性统计。
#[derive(Default, Clone)]
struct Reach {
    /// 金标总数。
    gold_total: usize,
    /// 可达金标数。
    reachable: usize,
    /// 逐题「可达金标数 ÷ 该题金标数」之和（闸门口径可达上限的分子）。
    frac_sum: f64,
    /// 全部金标都可达的题数。
    all_reachable: usize,
    /// 题数。
    cases: usize,
}

impl Reach {
    /// 闸门口径可达上限（逐题平均）。
    fn ceiling(&self) -> f64 {
        self.frac_sum / self.cases.max(1) as f64
    }
    /// 全可达率（题级二值）。
    fn all_rate(&self) -> f64 {
        self.all_reachable as f64 / self.cases.max(1) as f64
    }
    /// 盲区金标数。
    fn blind(&self) -> usize {
        self.gold_total - self.reachable
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

fn run() -> Result<(), Box<dyn Error>> {
    let mut rulers: Vec<Ruler> = Vec::new();
    if let Some(ruler) = load_real_query_ruler()? {
        rulers.push(ruler);
    }
    for name in ["redchamber", "watershed"] {
        match load_internal_ruler(name)? {
            Some(ruler) => rulers.push(ruler),
            None => eprintln!("· {name}：语料不在，跳过（不是「跑过了」）"),
        }
    }
    if rulers.is_empty() {
        eprintln!("· 一套考卷都没跑到 ⇒ **不能出结论**（不许把「没跑」当「没问题」）");
        return Ok(());
    }

    let mut summary: Vec<serde_json::Value> = Vec::new();
    for ruler in &rulers {
        let entry = report_ruler(ruler)?;
        summary.push(entry);
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "索引盲区探针": summary }))?
    );
    Ok(())
}

/// 真实查询卷（外部标注的**中文**真实问句 + 官方相关性标注）。
///
/// ⚠️ 语料在仓外：`$APEIRETH_MIRACL_EXAM`（默认 `~/.apeireth/miracl-exam/`）。
fn load_real_query_ruler() -> Result<Option<Ruler>, Box<dyn Error>> {
    let root = match std::env::var_os("APEIRETH_MIRACL_EXAM") {
        Some(dir) => PathBuf::from(dir),
        None => match std::env::var_os("APEIRETH_DATA_DIR") {
            Some(root) => PathBuf::from(root).join("miracl-exam"),
            None => PathBuf::from(std::env::var("HOME")?)
                .join(".apeireth")
                .join("miracl-exam"),
        },
    };
    let corpus = root.join("corpus.jsonl");
    let exam = root.join("exam.jsonl");
    if !corpus.exists() || !exam.exists() {
        eprintln!(
            "· 真实查询卷考卷不在（{}），跳过（不是「跑过了」）",
            root.display()
        );
        return Ok(None);
    }
    let mut index: Vec<IndexRecord> = Vec::new();
    for line in std::fs::read_to_string(&corpus)?.lines() {
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
        index.push(IndexRecord { id, text: joined });
    }
    let mut cases: Vec<GoldenCase> = Vec::new();
    for line in std::fs::read_to_string(&exam)?.lines() {
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
            tags: vec!["real_query".to_string()],
        });
    }
    Ok(Some(Ruler {
        label: "真实查询卷（外部标注中文真实问句）".to_string(),
        role: "真实形态 + 独立标签".to_string(),
        cases,
        index,
    }))
}

/// 内部词面卷（仓内挖题器 + 小说语料）。
fn load_internal_ruler(name: &str) -> Result<Option<Ruler>, Box<dyn Error>> {
    let path = corpus_root().join(format!("{name}.txt"));
    if !path.exists() {
        return Ok(None);
    }
    let chapters = novel_corpus::load_chapters(&path)?;
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
    Ok(Some(Ruler {
        label: format!("词面卷 / {name}（金标=主体本身）"),
        role: "产品形态".to_string(),
        cases: GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined)).cases,
        index,
    }))
}

fn corpus_root() -> PathBuf {
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("novels"),
        None => PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join(".apeireth")
            .join("novels"),
    }
}

/// 报告一套考卷的盲区结构。
fn report_ruler(ruler: &Ruler) -> Result<serde_json::Value, Box<dyn Error>> {
    let text_by_id: HashMap<&str, &str> = ruler
        .index
        .iter()
        .map(|record| (record.id.as_str(), record.text.as_str()))
        .collect();
    // 同父文档的兄弟段（真实查询卷的语料是分段切出来的 ⇒ 用来判 (c)）。
    let mut siblings: HashMap<&str, Vec<&str>> = HashMap::new();
    for record in &ruler.index {
        if let Some((parent, _)) = record.id.split_once('#') {
            siblings.entry(parent).or_default().push(record.id.as_str());
        }
    }

    let mut old = Reach::default();
    let mut new = Reach::default();
    let mut causes: HashMap<Cause, usize> = HashMap::new();
    let mut visibility: HashMap<Visibility, usize> = HashMap::new();
    let mut samples: HashMap<Cause, Vec<String>> = HashMap::new();
    let mut blind_questions_old = 0usize;
    let mut blind_questions_new = 0usize;

    for case in &ruler.cases {
        let normalized_query = normalize_for_index(&case.query);
        let q_old: HashSet<String> = tokenize(&normalized_query).into_iter().collect();
        let q_new: HashSet<String> = tokenize_with_unigrams(&normalized_query)
            .into_iter()
            .collect();
        old.cases += 1;
        new.cases += 1;
        let mut old_reachable_here = 0usize;
        let mut new_reachable_here = 0usize;
        for gold_id in &case.expect_ids {
            old.gold_total += 1;
            new.gold_total += 1;
            let Some(text) = text_by_id.get(gold_id.as_str()).copied() else {
                // (d) 金标压根不在索引里
                *causes.entry(Cause::IndexMissing).or_default() += 1;
                push_sample(
                    &mut samples,
                    Cause::IndexMissing,
                    format!("题面「{}」 · 金标 {gold_id} 不在索引里", case.query),
                );
                continue;
            };
            let normalized_text = normalize_for_index(text);
            let d_old: HashSet<String> = tokenize(&normalized_text).into_iter().collect();
            let d_new: HashSet<String> = tokenize_with_unigrams(&normalized_text)
                .into_iter()
                .collect();
            let reachable_old = !q_old.is_disjoint(&d_old);
            let reachable_new = !q_new.is_disjoint(&d_new);
            if reachable_old {
                old.reachable += 1;
                old_reachable_here += 1;
            }
            if reachable_new {
                new.reachable += 1;
                new_reachable_here += 1;
            }
            if reachable_old {
                continue;
            }
            // ---- 旧口径下拿不到 ⇒ 归因 ----
            let cause = classify_blind(&case.query, gold_id, &q_old, text, &text_by_id, &siblings);
            if cause == Cause::SingleCharOnly {
                *visibility
                    .entry(shared_visibility(&normalized_query, &normalized_text))
                    .or_default() += 1;
            }
            *causes.entry(cause).or_default() += 1;
            push_sample(
                &mut samples,
                cause,
                format!(
                    "题面「{}」 · 查询词 {:?} · 金标 {gold_id}",
                    case.query,
                    {
                        let mut v: Vec<&String> = q_old.iter().collect();
                        v.sort();
                        v
                    }
                ),
            );
        }
        old.frac_sum += old_reachable_here as f64 / case.expect_ids.len().max(1) as f64;
        new.frac_sum += new_reachable_here as f64 / case.expect_ids.len().max(1) as f64;
        if old_reachable_here == case.expect_ids.len() {
            old.all_reachable += 1;
        }
        if new_reachable_here == case.expect_ids.len() {
            new.all_reachable += 1;
        }
        if old_reachable_here < case.expect_ids.len() {
            blind_questions_old += 1;
        }
        if new_reachable_here < case.expect_ids.len() {
            blind_questions_new += 1;
        }
    }

    // ---- 现役口径下的真实分数（生产源）----
    let golden = GoldenSet::new(ruler.label.clone(), ruler.cases.clone());
    let report = run_golden(&ruler.index, &golden, K, InjectionMode::Production)?;
    let delivery = apeireth_memory::delivery_metrics::measure_delivery(&report.metrics);

    // ---- 多臂扫描：找「不回归内部卷」的补法 ----
    println!("     -- 多臂扫描（本地复刻；基线臂须与生产读数逐位相同）--");
    for variant in [
        Variant {
            doc_unigrams: false,
            len_bigram_only: true,
            query_unigrams: false,
            query_unigram_df_ratio: None,
        },
        Variant {
            doc_unigrams: true,
            len_bigram_only: true,
            query_unigrams: true,
            query_unigram_df_ratio: None,
        },
        Variant {
            doc_unigrams: true,
            len_bigram_only: true,
            query_unigrams: true,
            query_unigram_df_ratio: Some(0.01),
        },
        Variant {
            doc_unigrams: true,
            len_bigram_only: true,
            query_unigrams: true,
            query_unigram_df_ratio: Some(0.05),
        },
        Variant {
            doc_unigrams: true,
            len_bigram_only: true,
            query_unigrams: true,
            query_unigram_df_ratio: Some(0.20),
        },
        Variant {
            doc_unigrams: true,
            len_bigram_only: true,
            query_unigrams: true,
            query_unigram_df_ratio: None,
        },
    ] {
        let (arm, _) = ab_arms_with(ruler, variant);
        println!(
            "        {:<58} recall@{K}={:.4} · 盲区金标 {}",
            variant.label(),
            arm.recall_at_k,
            arm.blind_golds
        );
    }

    // ---- 口径 A/B（本地复刻；「旧口径」臂必须与生产读数逐位相同）----
    let (old_arm, new_arm) = ab_arms(ruler);
    println!(
        "     口径 A/B：{} recall@{K}={:.4}（盲区金标 {} · 盲区题 {}）· {} recall@{K}={:.4}（盲区金标 {} · 盲区题 {}）",
        old_arm.label,
        old_arm.recall_at_k,
        old_arm.blind_golds,
        old_arm.blind_questions,
        new_arm.label,
        new_arm.recall_at_k,
        new_arm.blind_golds,
        new_arm.blind_questions
    );
    println!(
        "     口径自证：旧臂 recall@{K}={:.6} vs 生产源 {:.6} ⇒ {}",
        old_arm.recall_at_k,
        report.metrics.recall_at_k,
        if (old_arm.recall_at_k - report.metrics.recall_at_k).abs() < 1e-9 {
            "逐位相同 ✓"
        } else {
            "**不一致 ⇒ 本地复刻不可信**"
        }
    );
    println!(
        "     新臂相对旧臂：**新救回** {} 条金标 · **被挤掉** {} 条金标",
        new_arm.rescued, new_arm.displaced
    );

    println!(
        "=== {} · {} · 语料 {} 段 · 题 {} · 金标 {} 条 ===",
        ruler.label,
        ruler.role,
        ruler.index.len(),
        old.cases,
        old.gold_total
    );
    println!(
        "     旧口径（只 bigram）：可达上限 {:.4} · 全可达率 {:.4} · 盲区金标 {} 条（{:.2}%）· 盲区题 {} 题（{:.2}%）",
        old.ceiling(),
        old.all_rate(),
        old.blind(),
        old.blind() as f64 / old.gold_total.max(1) as f64 * 100.0,
        blind_questions_old,
        blind_questions_old as f64 / old.cases.max(1) as f64 * 100.0
    );
    println!(
        "     新口径（+中文单字）：可达上限 {:.4} · 全可达率 {:.4} · 盲区金标 {} 条（{:.2}%）· 盲区题 {} 题（{:.2}%）",
        new.ceiling(),
        new.all_rate(),
        new.blind(),
        new.blind() as f64 / new.gold_total.max(1) as f64 * 100.0,
        blind_questions_new,
        blind_questions_new as f64 / new.cases.max(1) as f64 * 100.0
    );
    println!(
        "     现役分数：recall@{K}={:.4} · nDCG@{K}={:.4} · groundedness={:.4} · 送达率={}",
        report.metrics.recall_at_k,
        report.metrics.ndcg_at_k,
        report.metrics.groundedness,
        delivery
            .delivery_rate
            .map(|rate| format!("{rate:.4}"))
            .unwrap_or_else(|| "没测到".to_string())
    );
    let mut ordered: Vec<(Cause, usize)> = causes.iter().map(|(k, v)| (*k, *v)).collect();
    ordered.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (cause, count) in &ordered {
        println!(
            "     {:<28} {:>4} 条（占旧口径盲区 {:.1}%）",
            cause.label(),
            count,
            *count as f64 / old.blind().max(1) as f64 * 100.0
        );
    }
    if !visibility.is_empty() {
        println!("     -- 共享字可见性（(b) 类的根因分解）--");
        let mut ordered_vis: Vec<(Visibility, usize)> =
            visibility.iter().map(|(k, v)| (*k, *v)).collect();
        ordered_vis.sort_by(|a, b| b.1.cmp(&a.1));
        for (vis, count) in &ordered_vis {
            println!("        {:<52} {:>3} 条", vis.label(), count);
        }
    }
    for (cause, items) in &samples {
        if items.is_empty() {
            continue;
        }
        println!("   -- {} 样例 --", cause.label());
        for item in items {
            println!("      · {item}");
        }
    }

    let mut ordered_vis: Vec<(Visibility, usize)> =
        visibility.iter().map(|(k, v)| (*k, *v)).collect();
    ordered_vis.sort_by(|a, b| b.1.cmp(&a.1));
    let visibility_json: Vec<serde_json::Value> = ordered_vis
        .iter()
        .map(|(vis, count)| {
            serde_json::json!({
                "说明": vis.label(),
                "条数": count,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "考卷": ruler.label,
        "角色": ruler.role,
        "语料段数": ruler.index.len(),
        "题数": old.cases,
        "金标条数": old.gold_total,
        "旧口径": {
            "可达上限": old.ceiling(),
            "全可达率": old.all_rate(),
            "盲区金标": old.blind(),
            "盲区题": blind_questions_old,
        },
        "新口径": {
            "可达上限": new.ceiling(),
            "全可达率": new.all_rate(),
            "盲区金标": new.blind(),
            "盲区题": blind_questions_new,
        },
        "口径AB": {
            "旧口径": {
                "recall_at_k": old_arm.recall_at_k,
                "盲区金标": old_arm.blind_golds,
                "盲区题": old_arm.blind_questions,
            },
            "新口径": {
                "recall_at_k": new_arm.recall_at_k,
                "盲区金标": new_arm.blind_golds,
                "盲区题": new_arm.blind_questions,
                "新救回金标": new_arm.rescued,
                "被挤掉金标": new_arm.displaced,
            },
        },
        "现役分数": {
            "recall_at_k": report.metrics.recall_at_k,
            "ndcg_at_k": report.metrics.ndcg_at_k,
            "groundedness": report.metrics.groundedness,
            "evidence_precision": report.metrics.evidence_precision,
            "delivery_rate": delivery.delivery_rate,
        },
        "归类": ordered.iter().map(|(cause, count)| serde_json::json!({
            "原因": cause.label(),
            "条数": count,
        })).collect::<Vec<_>>(),
        "共享字可见性": visibility_json,
    }))
}

/// 共享字的**可见性**：旧口径下「中文字只有在连续汉字段的开头才独立成词」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Visibility {
    /// 题面侧可见、正文侧不可见 ⇒ 只差**索引侧**补单字。
    QueryOnly,
    /// 正文侧可见、题面侧不可见 ⇒ 只差**查询侧**补单字。
    DocOnly,
    /// 两侧都不可见（字落在字段中间）⇒ **两侧都要**补单字。
    Neither,
}

impl Visibility {
    fn label(self) -> &'static str {
        match self {
            Visibility::QueryOnly => "共享字：题面侧可见 / 正文侧不可见（只差索引侧）",
            Visibility::DocOnly => "共享字：正文侧可见 / 题面侧不可见（只差查询侧）",
            Visibility::Neither => "共享字：两侧都不可见（字在字段中间）",
        }
    }
}

/// 连续汉字段的**开头字**（旧口径下只有这些字会独立成词）。
fn run_start_chars(text: &str) -> HashSet<char> {
    let mut out = HashSet::new();
    let mut prev_cjk = false;
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            prev_cjk = false;
        } else if c.is_alphabetic() && (c as u32) > 0x2E80 {
            if !prev_cjk {
                out.insert(c);
            }
            prev_cjk = true;
        } else {
            prev_cjk = false;
        }
    }
    out
}

/// 共享字的可见性（根因的可执行读数）。
fn shared_visibility(normalized_query: &str, normalized_text: &str) -> Visibility {
    let q_start = run_start_chars(normalized_query);
    let d_start = run_start_chars(normalized_text);
    let mut query_side = false;
    let mut doc_side = false;
    for c in normalized_query.chars() {
        if !(c.is_alphabetic() && (c as u32) > 0x2E80) {
            continue;
        }
        if !normalized_text.contains(c) {
            continue;
        }
        if q_start.contains(&c) {
            query_side = true;
        }
        if d_start.contains(&c) {
            doc_side = true;
        }
    }
    match (query_side, doc_side) {
        (true, false) => Visibility::QueryOnly,
        (false, true) => Visibility::DocOnly,
        _ => Visibility::Neither,
    }
}

/// ⭐ **本地 BM25 复刻**（口径对照用）—— 用来做「旧口径 vs 新口径」的 A/B。
///
/// # 为什么要有
/// 生产只有一个口径 ⇒ 要量化「另一种口径会怎样」就得能**并排跑两套**。
///
/// # 自证（**必须成立**，否则这个 A/B 不可信）
/// * 本地实现**逐字复刻** [`apeireth_memory::hybrid_search::Bm25Index`] 的打分；
/// * 「旧口径」臂的 `recall@10` 必须与**生产源**的读数**逐位相同**
///   （见输出里的「口径自证」一行）⇒ ⇒ 不一致就说明复刻错了。
///
/// # 口径注意
/// 生产源在中文查询下 `coverage ≡ 0`（分词口径不同，见 `query_coverage`）
/// ⇒ ⇒ 源里的「按 max 归一」是**单调变换** ⇒ ⇒ **排序与本实现逐字相同**。
struct LocalBm25 {
    ids: Vec<String>,
    tf: Vec<HashMap<String, u32>>,
    lens: Vec<usize>,
    df: HashMap<String, usize>,
    total_tokens: usize,
    /// 查询侧是否补单字。
    query_unigrams: bool,
    /// 查询侧补单字时，只保留 **文档频率 ≤ 该比例** 的字（`None` = 全留）。
    query_unigram_df_ratio: Option<f64>,
}

/// BM25 超参数（与生产 `Bm25Config::default()` 逐字相同）。
const K1: f64 = 1.2;
/// BM25 长度归一参数（与生产 `Bm25Config::default()` 逐字相同）。
const B: f64 = 0.75;

impl LocalBm25 {
    fn build(docs: &[IndexRecord], variant: &Variant) -> Self {
        let mut ids = Vec::with_capacity(docs.len());
        let mut tf = Vec::with_capacity(docs.len());
        let mut lens = Vec::with_capacity(docs.len());
        let mut df: HashMap<String, usize> = HashMap::new();
        let mut total_tokens = 0usize;
        for record in docs {
            let normalized = normalize_for_index(&record.text);
            let base = tokenize(&normalized);
            let tokens = if variant.doc_unigrams {
                let mut all = base.clone();
                all.extend(tokenize_with_unigrams(&normalized));
                all
            } else {
                base.clone()
            };
            let mut map: HashMap<String, u32> = HashMap::new();
            for token in &tokens {
                *map.entry(token.clone()).or_default() += 1;
            }
            for token in map.keys() {
                *df.entry(token.clone()).or_default() += 1;
            }
            ids.push(record.id.clone());
            // ⭐ 长度归一的分母：可选「只算 bigram」⇒ ⇒ 分词口径变了而**归一口径不变**
            // ⇒ ⇒ 主通道的打分**逐字不变** ⇒ ⇒ 变化只来自新增的单字项。
            let len = if variant.len_bigram_only {
                base.len()
            } else {
                tokens.len()
            };
            lens.push(len);
            total_tokens += len;
            tf.push(map);
        }
        Self {
            ids,
            tf,
            lens,
            df,
            total_tokens,
            query_unigrams: variant.query_unigrams,
            query_unigram_df_ratio: variant.query_unigram_df_ratio,
        }
    }

    fn query_terms(&self, query: &str) -> Vec<String> {
        let normalized = normalize_for_index(query);
        let mut terms = tokenize(&normalized);
        if self.query_unigrams {
            let n = self.ids.len().max(1) as f64;
            for token in tokenize_with_unigrams(&normalized) {
                if token.chars().count() != 1 {
                    continue;
                }
                if let Some(ratio) = self.query_unigram_df_ratio {
                    let df = *self.df.get(&token).unwrap_or(&0) as f64;
                    if df / n > ratio {
                        continue;
                    }
                }
                if !terms.contains(&token) {
                    terms.push(token);
                }
            }
        }
        terms
    }

    /// 返回 `score > 0` 的候选（按分数降序、id 升序）。
    fn search(&self, query: &str, top_k: usize) -> Vec<(String, f64)> {
        if self.ids.is_empty() || top_k == 0 {
            return Vec::new();
        }
        let n = self.ids.len() as f64;
        let avg_dl = (self.total_tokens as f64 / n).max(1.0);
        let mut score_map: HashMap<String, f64> = HashMap::new();
        for term in self.query_terms(query) {
            let df = *self.df.get(&term).unwrap_or(&0) as f64;
            if df == 0.0 {
                continue;
            }
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            for (index, map) in self.tf.iter().enumerate() {
                let Some(&tf) = map.get(&term) else {
                    continue;
                };
                let tf = f64::from(tf);
                let dl = self.lens[index].max(1) as f64;
                let num = tf * (K1 + 1.0);
                let den = tf + K1 * (1.0 - B + B * (dl / avg_dl));
                let add = idf * (num / den);
                *score_map.entry(self.ids[index].clone()).or_default() += add;
            }
        }
        let mut scores: Vec<(String, f64)> = score_map.into_iter().collect();
        scores.retain(|(_, score)| *score > 0.0);
        scores.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        scores.truncate(top_k);
        scores
    }
}

/// 一套分词口径（用来做多臂 A/B）。
#[derive(Debug, Clone, Copy)]
struct Variant {
    /// 索引侧是否把每个中文字单独立词。
    doc_unigrams: bool,
    /// 长度归一的分母是否只算 bigram（旧口径的分母）。
    len_bigram_only: bool,
    /// 查询侧是否补单字。
    query_unigrams: bool,
    /// 查询侧补单字时只保留文档频率 ≤ 该比例的字（`None` = 全留）。
    query_unigram_df_ratio: Option<f64>,
}

impl Variant {
    fn label(self) -> String {
        format!(
            "索引单字={} 分母={} 查询单字={} 查询单字df上限={}",
            self.doc_unigrams,
            if self.len_bigram_only {
                "bigram"
            } else {
                "含单字"
            },
            self.query_unigrams,
            match self.query_unigram_df_ratio {
                Some(ratio) => format!("{ratio}"),
                None => "无".to_string(),
            }
        )
    }
}

/// 一套口径的 A/B 读数。
struct Arm {
    label: String,
    recall_at_k: f64,
    blind_golds: usize,
    gold_total: usize,
    blind_questions: usize,
    cases: usize,
    /// 与另一臂相比：**新救回**（旧臂盲、新臂在 top-k 里）的金标数。
    rescued: usize,
    /// **被挤掉**（旧臂在 top-k 里、新臂不在）的金标数。
    displaced: usize,
}

/// 对同一套考卷跑两套口径并对比。
fn ab_arms_with(ruler: &Ruler, variant: Variant) -> (Arm, Arm) {
    let old = LocalBm25::build(
        &ruler.index,
        &Variant {
            doc_unigrams: true,
            len_bigram_only: true,
            query_unigrams: false,
            query_unigram_df_ratio: None,
        },
    );
    let new = LocalBm25::build(&ruler.index, &variant);
    ab_arms_pair(ruler, &old, &new)
}

fn ab_arms(ruler: &Ruler) -> (Arm, Arm) {
    let old = LocalBm25::build(
        &ruler.index,
        &Variant {
            doc_unigrams: true,
            len_bigram_only: true,
            query_unigrams: false,
            query_unigram_df_ratio: None,
        },
    );
    let new = LocalBm25::build(
        &ruler.index,
        &Variant {
            doc_unigrams: true,
            len_bigram_only: true,
            query_unigrams: true,
            query_unigram_df_ratio: None,
        },
    );
    ab_arms_pair(ruler, &old, &new)
}

fn ab_arms_pair(ruler: &Ruler, old: &LocalBm25, new: &LocalBm25) -> (Arm, Arm) {
    let mut old_arm = Arm {
        label: "旧口径（只 bigram）".to_string(),
        recall_at_k: 0.0,
        blind_golds: 0,
        gold_total: 0,
        blind_questions: 0,
        cases: 0,
        rescued: 0,
        displaced: 0,
    };
    let mut new_arm = Arm {
        label: "新口径（bigram + 中文单字）".to_string(),
        recall_at_k: 0.0,
        blind_golds: 0,
        gold_total: 0,
        blind_questions: 0,
        cases: 0,
        rescued: 0,
        displaced: 0,
    };
    for case in &ruler.cases {
        let old_ranked: Vec<String> = old
            .search(&case.query, ruler.index.len().max(1))
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let new_ranked: Vec<String> = new
            .search(&case.query, ruler.index.len().max(1))
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let old_top: Vec<&String> = old_ranked.iter().take(K).collect();
        let new_top: Vec<&String> = new_ranked.iter().take(K).collect();
        old_arm.cases += 1;
        new_arm.cases += 1;
        let mut old_frac = 0.0f64;
        let mut new_frac = 0.0f64;
        let mut old_blind = 0usize;
        let mut new_blind = 0usize;
        for gold in &case.expect_ids {
            old_arm.gold_total += 1;
            new_arm.gold_total += 1;
            let in_old_top = old_top.contains(&gold);
            let in_new_top = new_top.contains(&gold);
            let in_old_rank = old_ranked.contains(gold);
            let in_new_rank = new_ranked.contains(gold);
            if in_old_top {
                old_frac += 1.0;
            }
            if in_new_top {
                new_frac += 1.0;
            }
            if !in_old_rank {
                old_blind += 1;
                old_arm.blind_golds += 1;
            }
            if !in_new_rank {
                new_blind += 1;
                new_arm.blind_golds += 1;
            }
            if !in_old_rank && in_new_top {
                new_arm.rescued += 1;
            }
            if in_old_top && !in_new_top {
                new_arm.displaced += 1;
            }
        }
        let total = case.expect_ids.len().max(1) as f64;
        old_arm.recall_at_k += old_frac / total;
        new_arm.recall_at_k += new_frac / total;
        if old_blind > 0 {
            old_arm.blind_questions += 1;
        }
        if new_blind > 0 {
            new_arm.blind_questions += 1;
        }
    }
    let cases = old_arm.cases.max(1) as f64;
    old_arm.recall_at_k /= cases;
    new_arm.recall_at_k /= cases;
    (old_arm, new_arm)
}

fn push_sample(samples: &mut HashMap<Cause, Vec<String>>, cause: Cause, item: String) {
    let bucket = samples.entry(cause).or_default();
    if bucket.len() < SAMPLES_PER_CLASS {
        bucket.push(item);
    }
}

/// 给**一条旧口径下拿不到的金标**归因。
fn classify_blind(
    query: &str,
    gold_id: &str,
    q_old: &HashSet<String>,
    text: &str,
    text_by_id: &HashMap<&str, &str>,
    siblings: &HashMap<&str, Vec<&str>>,
) -> Cause {
    if q_old.is_empty() {
        return Cause::Other;
    }
    // (c) 答案在同父文档的兄弟段里？（用**金标自己的**父 id）
    if let Some((parent, _)) = gold_id.split_once('#') {
        if let Some(list) = siblings.get(parent) {
            let normalized_query = normalize_for_index(query);
            let q: HashSet<String> = tokenize(&normalized_query).into_iter().collect();
            for sibling in list {
                let Some(sibling_text) = text_by_id.get(sibling).copied() else {
                    continue;
                };
                let d: HashSet<String> = tokenize(&normalize_for_index(sibling_text))
                    .into_iter()
                    .collect();
                if !q.is_disjoint(&d) {
                    return Cause::TruncatedSegment;
                }
            }
        }
    }
    // 题面词是否**是正文子串**？
    let overlapping: Vec<&String> = q_old
        .iter()
        .filter(|term| text.contains(term.as_str()))
        .collect();
    if overlapping.is_empty() {
        return Cause::Other;
    }
    if overlapping.iter().any(|term| term.chars().count() > 1) {
        return Cause::TokenizerMismatch;
    }
    Cause::SingleCharOnly
}
