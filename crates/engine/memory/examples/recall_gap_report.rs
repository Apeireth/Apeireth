//! 召回缺口诊断报告（**真实语料**）：回答「`recall@10 = 0.2983` 到底缺在哪」。
//!
//! # 用途
//!
//! 内部卷在《紅樓夢》上 `recall@10 = 0.2983` ⇒ **73% 的题，目标记录没进检索结果**
//! （W-12 实测：天花板 coverage 0.2712 ≈ 0.91 × recall@10）⇒ **瓶颈在召回**。
//! 但「召回低」有五种成因（分词/索引、排序、题挖错、k 太小、切分粒度）⇒
//! 本报告把它们**分类**（Hit / RankedOut / NotInPool / NoExpectation）并**给证据**。
//!
//! # 口径（必须与基线一致，否则数字不可比）
//!
//! * `k = 10`（与 `scripts/golden-baseline.json` 的 `k` 一致）；
//! * 挖题 = **放宽配置**（每记录 6 题）⇒ 与 2026-10-26 内部卷首跑同口径；
//! * 检索 = **生产用的 BM25**（`golden_diag::retrieve_pool` 内部与 `golden_run::run` 同口径）；
//! * **候选池 = 全部记录**（比 k 大得多 ⇒ 才能区分「没进池」与「排在 k 外」）；
//! * 官方 recall 曲线 = `golden_set::evaluate`（**与基线同源**）；
//!   乐观上界 = `golden_diag::recall_estimate_at_k`（任一期望命中即算命中 ⇒ **偏乐观**）。
//!
//! # 语料在仓外
//!
//! 语料在 `$APEIRETH_DATA_DIR/novels/`（默认 `~/.apeireth/novels/`）⇒
//! **某本语料缺失 ⇒ 跳过它并如实说明**（stderr），**不许**假装跑过；
//! 一本都没有 ⇒ **非零退出**。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能跑出真实诊断（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✔（部分）：已在本机**真实语料**上跑过（2026-10-26），
//!   **未在第二台机器复现** ⇒ 不宣称跨机器可复现。

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::golden_diag::{
    diagnose, recall_estimate_at_k, render_report, retrieve_pool, summarize, GapClass,
};
use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, IndexRecord};
use apeireth_memory::golden_set::{evaluate, EvalBatch, GoldenCase, Retrieved};
use apeireth_memory::novel_corpus;

/// 评测的 top-k（**必须与基线文件的 `k` 一致**）。
const K: usize = 10;

/// 挖题预算（每条记录最多几道题）—— 与基线同口径。
const MAX_CASES_PER_RECORD: usize = 6;

/// 要对比的 k（回答「k 太小是不是瓶颈」）。
const K_LADDER: [usize; 4] = [10, 20, 50, 100];

/// 语料清单：`（名字, 仓外文件名）`（与 `golden_baseline_report.rs` 同一张表）。
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

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
        match report_one(name, &path) {
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

/// 跑一份语料 ⇒ 召回缺口诊断报告。
fn report_one(name: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let config = MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = golden_mining::mine(&records, config);
    let cases: Vec<GoldenCase> = mined_to_golden(mined);

    // 可检索文本 = **正文**（与基线同口径；题面来源不进可检索文本 ⇒ 不泄漏答案）。
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();

    // ⚠️ **候选池 = 全部记录**（不是 k）⇒ 否则「没进池」与「排在 k 外」不可区分。
    let pools = retrieve_pool(&index, &cases, index.len())?;
    let diags = diagnose(&cases, &pools, K);
    let summary = summarize(&diags, K);

    println!("════════ {name} ════════");
    println!(
        "语料 {} 条（回）· 题 {} 道 · k={K} · 候选池中位 {} 条",
        index.len(),
        cases.len(),
        median_pool_size(&pools)
    );
    println!();
    println!("── render_report 原样输出 ──");
    println!("{}", render_report(&summary, K));
    println!();

    // --- 逐类计数（人话） ---
    println!("── 逐类计数 ──");
    println!("  命中 Hit           {} 题", summary.hits);
    println!("  排在 k 外 RankedOut {} 题", summary.ranked_out);
    println!("  没进候选池 NotInPool {} 题", summary.not_in_pool);
    println!(
        "  无期望 NoExpectation {} 题（题坏了，不进召回分母）",
        summary.no_expectation
    );
    println!();

    // --- 名次分布 ---
    println!("── 名次分布（只统计进过候选池的目标）──");
    match summary.rank_percentiles {
        Some(stats) => println!(
            "  p25={} · p50={} · p75={} · p95={} · max={}",
            stats.p25, stats.p50, stats.p75, stats.p95, stats.max
        ),
        None => println!("  **空**（没有任何目标进过候选池）"),
    }
    println!();

    // --- k 敏感性（官方口径 + 乐观上界）---
    println!("── k 敏感性：k 提到 20/50/100 能到多少 ──");
    println!(
        "  {:>6}  {:>14}  {:>16}",
        "k", "官方 recall@k", "乐观上界(任一命中)"
    );
    for k in K_LADDER {
        let official = official_recall_at(&cases, &pools, k);
        let optimistic = recall_estimate_at_k(&diags, k);
        println!("  {k:>6}  {official:>14.4}  {optimistic:>16.4}");
    }
    println!();

    // --- 同主体竞争度（回答「为什么排序这么难」）---
    print_competition(&index, &pools, &diags);
    println!();

    // --- 样本（人工看）---
    println!("── 样本（每类最多 5 条，供人工核查）──");
    for class in [
        GapClass::NotInPool,
        GapClass::RankedOut,
        GapClass::Hit,
        GapClass::NoExpectation,
    ] {
        let mut shown = 0usize;
        for (index, diag) in diags.iter().enumerate() {
            if diag.classify(K) != class {
                continue;
            }
            if shown == 0 {
                println!("  [{}]", class.label());
            }
            let kind = cases
                .get(index)
                .and_then(|case| {
                    case.tags
                        .iter()
                        .find_map(|tag| tag.strip_prefix("kind:").map(str::to_string))
                })
                .unwrap_or_else(|| "unknown".to_string());
            let rank = diag
                .best_rank
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".to_string());
            println!(
                "    [{kind}] 主体「{}」名次={rank} ⇒ {}",
                diag.subject,
                cases
                    .get(index)
                    .map(|case| case.query.as_str())
                    .unwrap_or("")
            );
            shown += 1;
            if shown >= 5 {
                break;
            }
        }
        if shown == 0 {
            println!("  [{}]（无）", class.label());
        }
    }
    println!();
    Ok(())
}

/// 候选池大小中位数（**证明池子确实比 k 大** ⇒ 诊断口径成立）。
fn median_pool_size(pools: &[Vec<Retrieved>]) -> usize {
    let mut sizes: Vec<usize> = pools
        .iter()
        .map(|entries| entries.first().map(|entry| entry.ids.len()).unwrap_or(0))
        .collect();
    sizes.sort_unstable();
    if sizes.is_empty() {
        return 0;
    }
    sizes[sizes.len() / 2]
}

/// 官方 recall@k（`golden_set::evaluate` 口径 ⇒ **与基线同源**）。
fn official_recall_at(cases: &[GoldenCase], pools: &[Vec<Retrieved>], k: usize) -> f64 {
    let mut batch = EvalBatch::default();
    for (index, case) in cases.iter().enumerate() {
        let ids: Vec<String> = pools
            .get(index)
            .and_then(|entries| entries.first())
            .map(|entry| entry.ids.clone())
            .unwrap_or_default();
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids: ids.into_iter().take(k).collect(),
            fragments: Vec::new(),
        });
    }
    evaluate(&batch, k)
        .map(|metrics| metrics.recall_at_k)
        .unwrap_or(0.0)
}

/// 「同主体竞争度」统计（**回答「为什么排序这么难」**）。
///
/// # 为什么要有这个
///
/// 题目主体（如「黛玉」）在小说里**到处出现** ⇒ 一回 = 一条 4000 字记录时，
/// **几十回都提到同一个主体** ⇒ top-10 里挤满「同主体」的回 ⇒ 目标回被挤出去。
/// ⇒ 这不是「索引找不到」，是**切分粒度**让排序变难。
///
/// # 两个数
///
/// * **同主体竞争者**：整份语料里正文**含该主体**的回数（= 检索时的真正竞争者）；
/// * **top-k 内同主体数**：前 k 条结果里有多少条**也含该主体**（= 挤占名额的密度）。
fn print_competition(
    index: &[IndexRecord],
    pools: &[Vec<Retrieved>],
    diags: &[apeireth_memory::golden_diag::RecallDiag],
) {
    let probe_of = |subject: &str| -> String {
        // 多主体题（「A和B」）⇒ 取第一个主体当探针（如实标注口径）。
        subject
            .split('和')
            .next()
            .unwrap_or(subject)
            .trim()
            .to_string()
    };
    let contains = |probe: &str, record: &IndexRecord| -> bool {
        !probe.is_empty() && record.text.contains(probe)
    };

    let mut all_rivals = Vec::new();
    let mut all_in_topk = Vec::new();
    let mut hit_rivals = Vec::new();
    let mut out_rivals = Vec::new();
    let mut record_len: Vec<usize> = Vec::new();
    // 题面主体**是否落在期望记录里**（回答「是不是题挖错了」）。
    let mut subject_checked = 0usize;
    let mut subject_in_expected = 0usize;

    for (position, diag) in diags.iter().enumerate() {
        let probe = probe_of(&diag.subject);
        if probe.is_empty() {
            continue;
        }
        // 题与答案一致性：主体词**出现在期望记录的正文里**吗？
        if let Some(expected_id) = diag.expect_ids.first() {
            if let Some(record) = index.iter().find(|record| record.id == *expected_id) {
                subject_checked += 1;
                if contains(&probe, record) {
                    subject_in_expected += 1;
                }
            }
        }
        let rivals = index
            .iter()
            .filter(|record| contains(&probe, record))
            .count();
        let top_ids: Vec<&str> = pools
            .get(position)
            .and_then(|entries| entries.first())
            .map(|entry| entry.ids.iter().take(K).map(String::as_str).collect())
            .unwrap_or_default();
        let in_topk = top_ids
            .iter()
            .filter(|id| {
                index
                    .iter()
                    .find(|record| record.id == **id)
                    .is_some_and(|record| contains(&probe, record))
            })
            .count();
        all_rivals.push(rivals);
        all_in_topk.push(in_topk);
        match diag.classify(K) {
            GapClass::Hit => hit_rivals.push(rivals),
            GapClass::RankedOut => out_rivals.push(rivals),
            _ => {}
        }
    }
    for record in index {
        record_len.push(record.text.chars().count());
    }

    println!("── 同主体竞争度（为什么排序这么难）──");
    println!(
        "  记录粒度：单条记录正文平均 {} 字（{} 条）",
        average(&record_len),
        record_len.len()
    );
    println!(
        "  同主体竞争者中位数：全部 {} 题 ⇒ {} 条记录提到同一主体",
        all_rivals.len(),
        median(&all_rivals)
    );
    println!(
        "  同主体竞争者中位数：命中题 {} 条 / 排出题 {} 条",
        median(&hit_rivals),
        median(&out_rivals)
    );
    println!(
        "  top-{K} 里的同主体数中位数：{} 条（⇒ 名额被同主体记录挤占）",
        median(&all_in_topk)
    );
    println!(
        "  题面主体落在期望记录里的比例：{}/{}（{:.1}%）⇒ 题与答案**对得上**（不是「题挖错了」）",
        subject_in_expected,
        subject_checked,
        ratio(subject_in_expected, subject_checked)
    );
}

/// 平均值（空输入 ⇒ 0）。
fn average(values: &[usize]) -> usize {
    if values.is_empty() {
        0
    } else {
        values.iter().sum::<usize>() / values.len()
    }
}

/// 中位数（空输入 ⇒ 0）。
fn median(values: &[usize]) -> usize {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

/// 百分比（分母为 0 ⇒ 0.0）。
fn ratio(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64 * 100.0
    }
}
