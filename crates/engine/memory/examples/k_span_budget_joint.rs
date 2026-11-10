//! **「召回名额 k × 单条跨度 × 总预算」联合扫描**（2026-11-08）。
//!
//! # 这个测例要回答的那一句
//!
//! 「**k↑ 的召回收益，在「送达率不许掉」的前提下能吃到多少？**」
//!
//! 三个旋钮**联动**，不能分开量：
//!
//! | 旋钮 | 变了会怎样 |
//! |---|---|
//! | **k（召回名额）** | top-k 变大 ⇒ 召回面变大 ⇒ 但**每条可用字数变小**（总预算共享） |
//! | **单条跨度** | 每条送出的**选段**长度 ⇒ 选段太短 ⇒ 答案在段外 ⇒ 送达率掉 |
//! | **总预算** | 总量 ⇒ 太大 ⇒ 成本涨；太小 ⇒ 条目被挤掉 |
//!
//! ⇒ ⇒ **只量一个旋钮 ⇒ 量不到联动** ⇒ ⇒ 必须**三个一起动**。
//!
//! # 三把尺子（都量，互为反证）
//!
//! | 尺子 | 是什么 | 为什么要有 |
//! |---|---|---|
//! | **R（无约束召回）** | 纯词面 top-k，**不施加任何预算** | 「k↑ 最多能值多少」的**天花板** |
//! | **P（生产召回）** | 生产管线（`HybridRetrievalPipeline`）＋**召回侧预算**（计费 = `min(正文长度, 单条上限)`） | 生产**真实**召回形态 |
//! | **D（生产送达）** | 生产编译器（`ClosedWorldContextCompiler`）＋**锚定选段**＋**总量预算** | 生产**真实**送达形态 |
//!
//! ⚠️ **R 与 P 的差 = 召回侧预算的代价**；**P 与 D 的差 = 送达侧丢的量**。
//!
//! # 口径自证（**必须逐字相同**，否则本测例量错了东西）
//!
//! * **内部卷**（模板题）：R 臂在 (k=10) 上的 `recall@10` 必须**逐字**等于
//!   `scripts/golden-baseline.json` 里的 `recall_at_k`（红楼梦 / 水浒传）；
//! * **真实查询卷**（真人问句 + 官方标注）：R 臂在 (k=10) 上的 `recall@10` 必须**逐字**等于
//!   `scripts/real-query-baseline.json` 里的 `recall_at_k`。
//!
//! ⇒ 不一致 ⇒ 本测例**如实报差异**（不粉饰、不重算基线）。
//!
//! # 送达率的判据（**整块**，与既有测例逐字同口径）
//!
//! * `delivery_rate_overlay`：**整个注入文本**是否覆盖期望片段
//!   （= 既有「送达漏送分解」测例的口径 ⇒ 可与既有数字逐字对照）。
//!
//! ⚠️ **不按行拆条目**做逐条判据：真实查询卷的条目正文**含换行**
//! （标题 + 正文两行）⇒ 按行拆会把一条证据**切成两半** ⇒ 逐条判据失真。
//!
//! ⇒ 「模型实际看到的证据条数」取**生产回执**（`selected_candidate_ids.len()`）⇒ 不受拆行影响。
//!
//! # 中性表述
//!
//! 只描述**机制**（召回 / 选段 / 注入 / 预算 / 名额）；数据集名只出现在**数据集口径**处。
//!
//! # 输出
//!
//! stdout 一行 JSON（机器读）；stderr 人读联合表；
//! 语料缺失 ⇒ 明确说「跳过，不是跑过」并返回非零。
//!
//! # 用法
//!
//! ```sh
//! cargo run -p apeireth-memory --example k_span_budget_joint            # 全量
//! cargo run -p apeireth-memory --example k_span_budget_joint -- --sample 20   # 小样本（口径自检用）
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::anchor_injection::AnchoredInjectionConfig;
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::mined_to_golden;
use apeireth_memory::golden_set::{
    covers_fragment, evaluate, EvalBatch, GoldenCase, Retrieved, FRAGMENT_COVERAGE_RATIO,
};
use apeireth_memory::layers::{MemoryLayerKind, MemoryRecallResult, RecalledMemoryItem};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_memory::{ClosedWorldContextCompiler, MemoryRankingConfig};
use serde_json::json;

/// **召回名额 k** 的扫描档位（含现行闸门口径 10）。
const K_ARMS: [usize; 4] = [10, 15, 20, 30];
/// **单条跨度**的扫描档位（含 2026-11-08 时的基线值 400）。
const SPAN_ARMS: [usize; 3] = [200, 300, 400];
/// **总预算**的扫描档位（含 4000 预算档）。
const BUDGET_ARMS: [usize; 3] = [4000, 6000, 8000];
/// **基线臂** = 2026-11-08 时的生产形态（k=10 · 跨度 400 · 预算 4000）；
/// **现生产默认已是 k=20 · 跨度 200**（见 `AnchoredInjectionConfig::PRODUCTION`）。
const BASE_K: usize = 10;
const BASE_SPAN: usize = 400;
const BASE_BUDGET: usize = 4000;
/// 内部卷挖题预算（**与闸门同口径**：每记录最多 6 题）。
const MAX_CASES_PER_RECORD: usize = 6;
/// 注入会话 id（只作归因用）。
const SESSION: &str = "k-span-budget-joint";
/// 内部卷语料清单（`（基线里的语料名, 仓外文件名）`）。
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

/// 一道题（**已带金标与期望片段**）。
struct Case {
    id: String,
    query: String,
    /// 金标 id。
    gold: Vec<String>,
    /// 期望片段（**送达判据**用）。
    fragments: Vec<String>,
}

/// 一份卷子（内部卷或真实查询卷）。
struct Corpus {
    name: String,
    cases: Vec<Case>,
    /// 词面候选源（生产同款）。
    source: Bm25LexicalCandidateSource,
    /// 口径自证用的基线 `recall@10`（读基线文件；读不到 ⇒ `None` ⇒ 标注「未验证」）。
    baseline_recall_at_10: Option<f64>,
    /// 口径自证用的基线题数。
    baseline_cases: Option<usize>,
    /// 候选源里的段落数（**样本池规模**）。
    documents: usize,
}

/// 一道题在某个 (跨度, 预算) 臂上的**逐 k** 量测。
#[derive(Default)]
struct CaseArm {
    /// R 尺子：逐 k 的召回命中比例（= 命中金标数 ÷ 金标数）。
    relaxed_hit: Vec<f64>,
    /// R 尺子（只留正分候选；内部卷闸门口径）。
    relaxed_hit_pos: Vec<f64>,
    /// P 尺子：逐 k 的召回命中比例。
    prod_hit: Vec<f64>,
    /// D 尺子（整块判据）：该题是否送达。
    delivered_overlay: Vec<bool>,
    /// 对照臂（放宽候选集 · 整块判据）。
    delivered_relaxed_candidates: Vec<bool>,
    /// 模型**实际看到**的证据条数。
    entries: Vec<usize>,
    /// 注入总字数（含标题行与规则文案）。
    injected_chars: Vec<usize>,
    /// 召回侧入选候选数。
    candidates: Vec<usize>,
}

/// 一个臂（k × 跨度 × 预算）的量测结果。
struct Arm {
    k: usize,
    span: usize,
    budget: usize,
    recall_relaxed: f64,
    recall_relaxed_pos: f64,
    recall_prod: f64,
    delivery_rate_overlay: f64,
    delivery_rate_relaxed_candidates: f64,
    cases: usize,
    avg_entries: f64,
    avg_injected_chars: f64,
    avg_candidates: f64,
}

fn main() -> ExitCode {
    let sample = parse_sample();
    match run(sample) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("· 失败：{error}");
            ExitCode::FAILURE
        }
    }
}

/// `--sample N`（小样本口径自检用；不给 ⇒ 全量）。
fn parse_sample() -> Option<usize> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--sample" {
            return args.next().and_then(|value| value.parse::<usize>().ok());
        }
        if let Some(value) = arg.strip_prefix("--sample=") {
            return value.parse::<usize>().ok();
        }
    }
    None
}

fn run(sample: Option<usize>) -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = repo_root();
    let internal_baseline = read_internal_baseline(&repo_root);
    let real_baseline = read_real_baseline(&repo_root);

    let mut corpus_rows: Vec<serde_json::Value> = Vec::new();
    let mut loaded = 0usize;

    for (name, filename) in CORPORA {
        let path = novels_dir().join(filename);
        if !path.exists() {
            eprintln!(
                "· {name}：语料不在（{}）⇒ **跳过**（不是「跑过了」）",
                path.display()
            );
            continue;
        }
        let corpus = build_internal_corpus(name, &path, internal_baseline.as_ref(), sample)?;
        eprintln!(
            "· {name}：题 {} 道 · 候选源 {} 段",
            corpus.cases.len(),
            corpus.documents
        );
        let arms = scan_corpus(&corpus)?;
        corpus_rows.push(json!({
            "name": name,
            "cases": corpus.cases.len(),
            "documents": corpus.documents,
            "baseline_recall_at_10": corpus.baseline_recall_at_10,
            "baseline_cases": corpus.baseline_cases,
            "self_check": self_check(&corpus, &arms),
            "arms": arms.iter().map(arm_row).collect::<Vec<_>>(),
        }));
        loaded += 1;
    }

    let real_row = match build_real_corpus(real_baseline.as_ref(), sample)? {
        Some(corpus) => {
            eprintln!(
                "· real_query：题 {} 道 · 候选源 {} 段",
                corpus.cases.len(),
                corpus.documents
            );
            let arms = scan_corpus(&corpus)?;
            Some(json!({
                "name": "real_query",
                "cases": corpus.cases.len(),
                "documents": corpus.documents,
                "baseline_recall_at_10": corpus.baseline_recall_at_10,
                "baseline_cases": corpus.baseline_cases,
                "self_check": self_check(&corpus, &arms),
                "arms": arms.iter().map(arm_row).collect::<Vec<_>>(),
            }))
        }
        None => None,
    };

    if loaded == 0 && real_row.is_none() {
        eprintln!("一份卷都没跑到 ⇒ **不能出数字**（不许把「没跑」当「没问题」）");
        return Err("没有可测的语料".into());
    }

    println!(
        "{}",
        json!({
            "sample_limit": sample,
            "k_arms": K_ARMS,
            "span_arms": SPAN_ARMS,
            "budget_arms": BUDGET_ARMS,
            "base_arm": { "k": BASE_K, "span": BASE_SPAN, "budget": BASE_BUDGET },
            "internal_corpora": corpus_rows,
            "real_query": real_row,
        })
    );
    Ok(())
}

// ============================ 语料装载 ============================

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

fn exam_dir() -> PathBuf {
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

/// 仓根（**从 manifest 目录反推** ⇒ 不依赖 CWD）。
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .find(|dir| dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir())
        .map(Path::to_path_buf)
        .unwrap_or(manifest)
}

/// 读内部卷基线（`scripts/golden-baseline.json`）。
fn read_internal_baseline(repo_root: &Path) -> Option<HashMap<String, (f64, usize)>> {
    let path = repo_root.join("scripts").join("golden-baseline.json");
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let mut out = HashMap::new();
    for corpus in value.get("corpora")?.as_array()? {
        let name = corpus.get("name")?.as_str()?.to_string();
        let recall = corpus.get("recall_at_k")?.as_f64()?;
        let cases = corpus.get("cases")?.as_u64()? as usize;
        out.insert(name, (recall, cases));
    }
    Some(out)
}

/// 读真人卷基线（`scripts/real-query-baseline.json`）。
fn read_real_baseline(repo_root: &Path) -> Option<(f64, usize)> {
    let path = repo_root.join("scripts").join("real-query-baseline.json");
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let recall = value.get("recall_at_k")?.as_f64()?;
    let cases = value.get("cases")?.as_u64()? as usize;
    Some((recall, cases))
}

/// 内部卷（模板题）：挖题 ⇒ 考卷 ⇒ 候选源。
fn build_internal_corpus(
    name: &str,
    path: &Path,
    baseline: Option<&HashMap<String, (f64, usize)>>,
    sample: Option<usize>,
) -> Result<Corpus, Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let mined = apeireth_memory::golden_mining::mine(
        &records,
        MiningConfig {
            max_cases_per_record: MAX_CASES_PER_RECORD,
            ..MiningConfig::default()
        },
    );
    let golden = apeireth_memory::golden_set::GoldenSet::new(
        format!("internal-{name}"),
        mined_to_golden(mined),
    );
    let documents: Vec<MemoryCandidate> = records
        .iter()
        .map(|record| MemoryCandidate {
            id: record.id.clone(),
            layer: "golden".to_string(),
            scope: MemoryScope::Global,
            content: record.content.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance::default(),
        })
        .collect();
    let mut cases: Vec<Case> = golden
        .cases
        .iter()
        .filter(|case| !case.expect_ids.is_empty() && !case.expect_fragments.is_empty())
        .map(|case| Case {
            id: case.id.clone(),
            query: case.query.clone(),
            gold: case.expect_ids.clone(),
            fragments: case.expect_fragments.clone(),
        })
        .collect();
    if let Some(limit) = sample {
        cases.truncate(limit);
    }
    let (baseline_recall_at_10, baseline_cases) = baseline
        .and_then(|map| map.get(name).copied())
        .map(|(recall, cases)| (Some(recall), Some(cases)))
        .unwrap_or((None, None));
    Ok(Corpus {
        name: name.to_string(),
        cases,
        documents: documents.len(),
        source: Bm25LexicalCandidateSource::new(documents),
        baseline_recall_at_10,
        baseline_cases,
    })
}

/// 真实查询卷（真人问句 + 官方标注）。
fn build_real_corpus(
    baseline: Option<&(f64, usize)>,
    sample: Option<usize>,
) -> Result<Option<Corpus>, Box<dyn std::error::Error>> {
    let root = exam_dir();
    let corpus_path = root.join("corpus.jsonl");
    let exam_path = root.join("exam.jsonl");
    if !corpus_path.exists() || !exam_path.exists() {
        eprintln!(
            "· 真实查询卷：考卷不在（{}）⇒ **跳过**（不是「跑过了」）",
            root.display()
        );
        return Ok(None);
    }
    let mut content_by_id: HashMap<String, String> = HashMap::new();
    let mut documents: Vec<MemoryCandidate> = Vec::new();
    for line in std::fs::read_to_string(&corpus_path)?.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        let id = value["id"].as_str().unwrap_or_default().to_string();
        if id.is_empty() {
            continue;
        }
        let content = format!(
            "{}\n{}",
            value["title"].as_str().unwrap_or_default(),
            value["text"].as_str().unwrap_or_default()
        );
        content_by_id.insert(id.clone(), content.clone());
        documents.push(MemoryCandidate {
            id,
            layer: "real_query".to_string(),
            scope: MemoryScope::Global,
            content,
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance::default(),
        });
    }
    let mut cases: Vec<Case> = Vec::new();
    for line in std::fs::read_to_string(&exam_path)?.lines() {
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
        // 期望片段 = 金标文档的正文（与内部卷「期望片段是记录正文」的口径对齐）。
        let fragments: Vec<String> = gold
            .iter()
            .filter_map(|id| content_by_id.get(id).cloned())
            .collect();
        if fragments.is_empty() {
            continue;
        }
        cases.push(Case {
            id: value["id"].as_str().unwrap_or_default().to_string(),
            query,
            gold,
            fragments,
        });
    }
    if let Some(limit) = sample {
        cases.truncate(limit);
    }
    let (baseline_recall_at_10, baseline_cases) = baseline
        .map(|value| (Some(value.0), Some(value.1)))
        .unwrap_or((None, None));
    Ok(Some(Corpus {
        name: "real_query".to_string(),
        cases,
        documents: documents.len(),
        source: Bm25LexicalCandidateSource::new(documents),
        baseline_recall_at_10,
        baseline_cases,
    }))
}

// ============================ 扫描 ============================

/// 扫一份卷子的全部臂。
///
/// # 为什么这么扫（**性能等价，不改口径**）
///
/// * 候选源按分数降序 ⇒ `top-10 ⊂ top-15 ⊂ top-20 ⊂ top-30` ⇒ **k 的前缀性质**；
/// * 生产管线按 `candidate_cap`（= 64）取候选 ⇒ **与 `limit` 无关**
///   ⇒ `limit=10` 的选中集是 `limit=30` 选中集的**前缀**；
/// * 注入按预算顺序装填 ⇒ 同理。
///
/// ⇒ ⇒ **每个 (跨度, 预算) 只跑一次检索 + 一次注入**，再按 k 取前缀
/// ⇒ ⇒ 数字与逐臂单独跑**逐字相同**，但快 ~4 倍。
fn scan_corpus(corpus: &Corpus) -> Result<Vec<Arm>, Box<dyn std::error::Error>> {
    let max_k = K_ARMS.iter().copied().max().unwrap_or(10);

    // —— R 尺子的候选（无预算）：每题取 max_k 条 ⇒ 逐 k 取前缀 ——
    let relaxed_per_case: Vec<Vec<MemoryCandidate>> = parallel_map(&corpus.cases, |case| {
        corpus
            .source
            .candidates(&case.query, max_k)
            .map_err(|error| error.to_string())
    })?;

    let mut arms: Vec<Arm> = Vec::new();
    for span in SPAN_ARMS {
        for budget in BUDGET_ARMS {
            let ranking = MemoryRankingConfig {
                candidate_span_chars: span,
                ..MemoryRankingConfig::default()
            };
            let pipeline = HybridRetrievalPipeline::new(ranking);
            let compiler = ClosedWorldContextCompiler::new();
            let config = AnchoredInjectionConfig {
                enabled: true,
                span_chars: span,
            };

            // —— P 尺子的候选（生产管线 + 召回侧预算）：每题取 max_k 条 ——
            let prod_per_case: Vec<Vec<MemoryCandidate>> = parallel_map(&corpus.cases, |case| {
                pipeline
                    .retrieve(
                        &case.query,
                        &[MemoryScope::Global],
                        &[&corpus.source],
                        max_k,
                        budget,
                    )
                    .map_err(|error| error.to_string())
            })?;

            // —— 逐题量测（注入 + 送达判据）——
            let case_arms: Vec<CaseArm> = parallel_map(&corpus.cases, |case| {
                let index = index_of(corpus, case);
                measure_case(
                    case,
                    &compiler,
                    config,
                    budget,
                    &prod_per_case[index],
                    &relaxed_per_case[index],
                    max_k,
                )
            })?;

            // —— ⭐ **口径交叉校验**：把**第一个 k 臂**的命中比例与既有评测器
            //   （`golden_set::evaluate`）**逐字对一遍** ⇒ ⇒ 证明本测例自带的
            //   `hit_fraction` 与官方口径**一致**（否则扫描结论不可比）。
            if span == SPAN_ARMS[0] && budget == BUDGET_ARMS[0] {
                let ids_per_case: Vec<Vec<String>> = prod_per_case
                    .iter()
                    .map(|candidates| {
                        candidates
                            .iter()
                            .take(K_ARMS[0])
                            .map(|item| item.id.clone())
                            .collect()
                    })
                    .collect();
                let official = recall_via_evaluate(&corpus.cases, &ids_per_case, K_ARMS[0])?;
                let mine = case_arms.iter().map(|arm| arm.prod_hit[0]).sum::<f64>()
                    / case_arms.len().max(1) as f64;
                assert!(
                    (official - mine).abs() < 1e-9,
                    "口径自证失败：既有评测器 {official:.9} vs 本测例 {mine:.9} ⇒ 扫描口径漂移"
                );
            }

            // —— 聚合 ——
            for (k_index, k) in K_ARMS.iter().copied().enumerate() {
                let total = case_arms.len().max(1) as f64;
                let relaxed_sum: f64 = case_arms.iter().map(|c| c.relaxed_hit[k_index]).sum();
                let relaxed_pos_sum: f64 =
                    case_arms.iter().map(|c| c.relaxed_hit_pos[k_index]).sum();
                let prod_sum: f64 = case_arms.iter().map(|c| c.prod_hit[k_index]).sum();
                let overlay_ok = case_arms
                    .iter()
                    .filter(|c| c.delivered_overlay[k_index])
                    .count();
                let relaxed_ok = case_arms
                    .iter()
                    .filter(|c| c.delivered_relaxed_candidates[k_index])
                    .count();
                let entries_sum: usize = case_arms.iter().map(|c| c.entries[k_index]).sum();
                let injected_sum: usize = case_arms.iter().map(|c| c.injected_chars[k_index]).sum();
                let candidates_sum: usize = case_arms.iter().map(|c| c.candidates[k_index]).sum();
                let arm = Arm {
                    k,
                    span,
                    budget,
                    recall_relaxed: relaxed_sum / total,
                    recall_relaxed_pos: relaxed_pos_sum / total,
                    recall_prod: prod_sum / total,
                    delivery_rate_overlay: overlay_ok as f64 / total,
                    delivery_rate_relaxed_candidates: relaxed_ok as f64 / total,
                    cases: case_arms.len(),
                    avg_entries: entries_sum as f64 / total,
                    avg_injected_chars: injected_sum as f64 / total,
                    avg_candidates: candidates_sum as f64 / total,
                };
                eprintln!(
                    "  · {name} k={k:<3} 跨度={span:<5} 预算={budget:<5} ⇒ R recall={r:.4} · P recall={p:.4} · 送达率={do_:.4} · 条目/题={en:.2} · 注入字/题={ic:.0}",
                    name = corpus.name,
                    r = arm.recall_relaxed,
                    p = arm.recall_prod,
                    do_ = arm.delivery_rate_overlay,
                    en = arm.avg_entries,
                    ic = arm.avg_injected_chars,
                );
                arms.push(arm);
            }
        }
    }
    Ok(arms)
}

/// 题在卷子里的下标（`parallel_map` 的闭包里要用）。
fn index_of(corpus: &Corpus, case: &Case) -> usize {
    corpus
        .cases
        .iter()
        .position(|item| std::ptr::eq(item, case))
        .unwrap_or(0)
}

/// 一道题在某个 (跨度, 预算) 臂上的量测。
fn measure_case(
    case: &Case,
    compiler: &ClosedWorldContextCompiler,
    config: AnchoredInjectionConfig,
    budget: usize,
    prod: &[MemoryCandidate],
    relaxed: &[MemoryCandidate],
    max_k: usize,
) -> Result<CaseArm, String> {
    let mut arm = CaseArm {
        relaxed_hit: vec![0.0; K_ARMS.len()],
        relaxed_hit_pos: vec![0.0; K_ARMS.len()],
        prod_hit: vec![0.0; K_ARMS.len()],
        delivered_overlay: vec![false; K_ARMS.len()],
        delivered_relaxed_candidates: vec![false; K_ARMS.len()],
        entries: vec![0; K_ARMS.len()],
        injected_chars: vec![0; K_ARMS.len()],
        candidates: vec![0; K_ARMS.len()],
    };
    for (k_index, k) in K_ARMS.iter().copied().enumerate() {
        let take = k.min(max_k);
        let relaxed_take = take.min(relaxed.len());
        let prod_take = take.min(prod.len());
        let relaxed_ids: Vec<String> = relaxed
            .iter()
            .take(relaxed_take)
            .map(|item| item.id.clone())
            .collect();
        let relaxed_pos_ids: Vec<String> = relaxed
            .iter()
            .filter(|item| item.score > 0.0)
            .take(relaxed_take)
            .map(|item| item.id.clone())
            .collect();
        let prod_ids: Vec<String> = prod
            .iter()
            .take(prod_take)
            .map(|item| item.id.clone())
            .collect();
        arm.relaxed_hit[k_index] = hit_fraction(&relaxed_ids, &case.gold);
        arm.relaxed_hit_pos[k_index] = hit_fraction(&relaxed_pos_ids, &case.gold);
        arm.prod_hit[k_index] = hit_fraction(&prod_ids, &case.gold);
        arm.candidates[k_index] = prod_ids.len();

        // —— D 尺子：生产注入（锚定选段 + 总量预算）——
        let recalled = recall_result(&prod[..prod_take]);
        let selected = compiler.compile_injection_with_selected_access_and_query(
            &recalled,
            SESSION,
            budget,
            &case.query,
            config,
        );
        if let Some(selected) = selected.as_ref() {
            arm.entries[k_index] = selected.selected_candidate_ids.len();
            arm.injected_chars[k_index] = selected.overlay.chars().count();
            arm.delivered_overlay[k_index] = covered(&selected.overlay, &case.fragments);
        }

        // —— 对照臂：候选集放宽（不施加召回侧预算）+ 整块判据 ——
        // （与既有「送达漏送分解」测例同口径 ⇒ 可与既有数字逐字对照）
        let relaxed_recalled = recall_result(&relaxed[..relaxed_take]);
        let relaxed_selected = compiler.compile_injection_with_selected_access_and_query(
            &relaxed_recalled,
            SESSION,
            budget,
            &case.query,
            config,
        );
        if let Some(selected) = relaxed_selected.as_ref() {
            arm.delivered_relaxed_candidates[k_index] = covered(&selected.overlay, &case.fragments);
        }
    }
    Ok(arm)
}

/// 一道题在某个 top-k 列表上的命中比例（= 既有 `evaluate` 的 Recall@k 口径）。
fn hit_fraction(ids: &[String], gold: &[String]) -> f64 {
    if gold.is_empty() {
        return 0.0;
    }
    let hits = gold
        .iter()
        .filter(|expected| ids.iter().any(|id| id == *expected))
        .count();
    hits as f64 / gold.len() as f64
}

/// 把候选包成**生产的召回结果**（喂给生产编译器 ⇒ 走生产同一条路）。
fn recall_result(candidates: &[MemoryCandidate]) -> MemoryRecallResult {
    MemoryRecallResult {
        items: candidates
            .iter()
            .map(|candidate| RecalledMemoryItem {
                id: candidate.id.clone(),
                layer: MemoryLayerKind::Episodic,
                content: candidate.content.clone(),
                timestamp_ms: 0,
                score: candidate.score,
                importance: 0.0,
                source_ref: None,
                score_components: None,
                diary_snippets: Vec::new(),
            })
            .collect(),
        total_candidates: candidates.len(),
        governance_filtered: 0,
        total_chars: 0,
        retrieval_status: None,
        wiki_sections: Vec::new(),
    }
}

/// 注入文本里**是否送达**了期望片段。
///
/// # 口径
///
/// 与 [`covers_fragment`] **逐字等价**，只是加了一条**等价的短路**：
/// 最长公共子串的长度**不可能超过**注入文本自身的长度
/// ⇒ 当「注入长度 < 覆盖所需长度」时**直接判否**（省掉一次 O(n·m) 的动态规划）。
fn covered(overlay: &str, fragments: &[String]) -> bool {
    let overlay_len = overlay.chars().count();
    fragments.iter().any(|fragment| {
        if overlay.contains(fragment) {
            return true;
        }
        let needed = ((fragment.chars().count() as f64) * FRAGMENT_COVERAGE_RATIO).ceil() as usize;
        if overlay_len < needed.max(4) {
            return false;
        }
        covers_fragment(overlay, fragment)
    })
}

/// 并行映射（**按题分块**给线程；块内串行 ⇒ 无锁）。
///
/// ⚠️ 线程内的错误用 `String` 承载（`Box<dyn Error>` **不是 `Send`** ⇒ 跨线程不得用）。
fn parallel_map<T, R, F>(items: &[T], worker: F) -> Result<Vec<R>, Box<dyn std::error::Error>>
where
    T: Sync,
    R: Send,
    F: Fn(&T) -> Result<R, String> + Sync,
{
    let threads = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .min(items.len().max(1));
    let chunk = items.len().div_ceil(threads.max(1));
    let mut out: Vec<R> = Vec::new();
    let mut failure: Option<String> = None;
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for slice in items.chunks(chunk.max(1)) {
            let worker = &worker;
            handles.push(
                scope.spawn(move || slice.iter().map(worker).collect::<Result<Vec<R>, String>>()),
            );
        }
        for handle in handles {
            match handle.join() {
                Ok(Ok(piece)) => out.extend(piece),
                Ok(Err(error)) => failure = Some(error),
                Err(_) => failure = Some("线程 panic".to_string()),
            }
        }
    });
    if let Some(error) = failure {
        return Err(error.into());
    }
    Ok(out)
}

// ============================ 报告 ============================

/// 口径自证：基线臂（k=10）的 R 尺子读数 vs 基线文件。
fn self_check(corpus: &Corpus, arms: &[Arm]) -> serde_json::Value {
    let Some(base) = arms
        .iter()
        .find(|arm| arm.k == BASE_K && arm.span == BASE_SPAN && arm.budget == BASE_BUDGET)
    else {
        return json!({ "status": "未验证", "reason": "没扫到基线臂" });
    };
    let Some(expected) = corpus.baseline_recall_at_10 else {
        return json!({
            "status": "未验证",
            "reason": "基线文件读不到或该语料不在基线里",
            "measured_recall_at_10": base.recall_relaxed,
        });
    };
    json!({
        "status": if (base.recall_relaxed - expected).abs() < 1e-12 { "逐字相同" } else { "不一致" },
        "arm": { "k": BASE_K, "span": BASE_SPAN, "budget": BASE_BUDGET },
        "measured_recall_at_10_relaxed": base.recall_relaxed,
        "measured_recall_at_10_relaxed_positive_only": base.recall_relaxed_pos,
        "baseline_recall_at_10": expected,
        "baseline_cases": corpus.baseline_cases,
        "measured_cases": corpus.cases.len(),
        "delta_relaxed": base.recall_relaxed - expected,
        "delta_relaxed_positive_only": base.recall_relaxed_pos - expected,
        "delivery_rate_overlay": base.delivery_rate_overlay,
        "delivery_rate_relaxed_candidates": base.delivery_rate_relaxed_candidates,
    })
}

fn arm_row(arm: &Arm) -> serde_json::Value {
    json!({
        "k": arm.k,
        "span": arm.span,
        "budget": arm.budget,
        "recall_relaxed": arm.recall_relaxed,
        "recall_relaxed_positive_only": arm.recall_relaxed_pos,
        "recall_prod": arm.recall_prod,
        "delivery_rate_overlay": arm.delivery_rate_overlay,
        "delivery_rate_relaxed_candidates": arm.delivery_rate_relaxed_candidates,
        "cases": arm.cases,
        "avg_entries_seen_by_model": arm.avg_entries,
        "avg_injected_chars": arm.avg_injected_chars,
        "avg_candidates": arm.avg_candidates,
    })
}

/// 既有口径（`golden_set::evaluate`）的 recall@k —— **交叉校验用**。
///
/// ⚠️ 保留它是为了让「本测例的命中比例」与既有评测器**逐字可比**；
/// 扫描本身用 `hit_fraction`（同一口径的等价实现，省一次批构造）。
fn recall_via_evaluate(
    cases: &[Case],
    ids_per_case: &[Vec<String>],
    k: usize,
) -> Result<f64, Box<dyn std::error::Error>> {
    let mut batch = EvalBatch::default();
    for (case, ids) in cases.iter().zip(ids_per_case.iter()) {
        batch.cases.push(GoldenCase {
            id: case.id.clone(),
            query: case.query.clone(),
            expect_ids: case.gold.clone(),
            expect_fragments: Vec::new(),
            tags: Vec::new(),
        });
        batch.results.push(Retrieved {
            ids: ids.clone(),
            fragments: Vec::new(),
        });
    }
    Ok(evaluate(&batch, k)?.recall_at_k)
}
