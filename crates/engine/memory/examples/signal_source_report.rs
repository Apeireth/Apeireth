//! 非答案侧**题面信号来源**的**真实语料对照报告**（《紅樓夢》+《水滸傳》）。
//!
//! # 这个例子回答什么
//!
//! W-30 证明了「题面多带一个判别信号 ⇒ recall@10 翻倍」，但那是 **oracle 上界**
//! （信号**来自答案片段**）。本例子回答**端到端的那一半**：
//! **信号从哪来？**（且**不许**从答案来）
//!
//! | 来源 | 想法 | 本批的实现 |
//! |---|---|---|
//! | 同主题历史 | 从**同一主体**的其它记录**开头**取词 | 主体记录开头 60 字的词频 top-3 |
//! | 实体共现 | 语料里与主体**强共现**的词 | `df ≥ 3`、`强度 ≥ 0.5`、`df ≤ 语料×0.25`，按 `df` 降序 |
//! | 时间锚 | 「第 N 回」这类时间信号 | **只从题面扫**（找不到就空 ⇒ **不编造**） |
//! | 段落首句 | 目标段落的开头句 | PRF top-1 记录里**含主体那��段**的首句 |
//! | 原文首句 | 整条记录的开头句 | PRF top-1 记录的**首句** |
//!
//! # ⚠️ 口径（钉死，否则数字不可比）
//!
//! * `k = 10`（与 `scripts/golden-baseline.json` 一致）；
//! * 检索 = **生产用的 BM25**（`golden_run::run` 内部 ⇒ 与生产同口径）；
//! * 注入 = **生产口径**（`InjectionMode::Production`）；
//! * 挖题 = **放宽配置**（每记录 6 题 ⇒ 与 2026-10-26 内部卷首跑、W-20R/W-30 同口径）；
//! * 可检索粒度 = **整回**（W-20R 已证段落化更差 ⇒ 本批不重复该实验）；
//! * 每个来源**最多加 3 个词** ⇒ **不许**为某个来源调权重凑分数。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-27）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能跑出真实对照（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有人工在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✔（部分）：已在两本**真实**小说上跑过（2026-10-27），
//!   **未在第二台机器复现** ⇒ 不宣称跨机器可复现。

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::mined_to_golden;
use apeireth_memory::golden_set::GoldenCase;
use apeireth_memory::novel_corpus;
use apeireth_memory::signal_sources::{
    compare_sources, render_source_report, signal_for_case, SourceKind, SOURCE_KINDS,
};

/// 评测的 top-k（**必须与基线文件的 `k` 一致**）。
const K: usize = 10;

/// 挖题预算（每条记录最多几道题）—— 与基线同口径。
const MAX_CASES_PER_RECORD: usize = 6;

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

/// 跑一份语料 ⇒ 打印逐来源对照表。
fn report_one(name: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let mine_records = novel_corpus::to_mine_records(&chapters);
    if mine_records.is_empty() {
        return Err("语料解析出 0 条记录".into());
    }

    // 可检索记录（**整回**：正文，不含回目联 ⇒ 题面不泄漏）。
    let records: Vec<(String, String)> = mine_records
        .iter()
        .map(|record| (record.id.clone(), record.content.clone()))
        .collect();

    // 挖题（与 W-20R / 2026-10-26 内部卷首跑同口径）。
    let mine_started = std::time::Instant::now();
    let mined = golden_mining::mine(&mine_records, mining_config());
    let mine_elapsed = mine_started.elapsed();
    let cases: Vec<GoldenCase> = mined_to_golden(mined);
    if cases.is_empty() {
        return Err("挖不出题".into());
    }
    let golden = apeireth_memory::golden_set::GoldenSet::new(format!("source-{name}"), cases);

    println!("════════ {name} ════════");
    println!(
        "语料 {} 回 · 题 {} 道 · k={K} · 检索 = 生产 BM25 · 注入 = 生产口径",
        records.len(),
        golden.len()
    );
    println!();

    let compare_started = std::time::Instant::now();
    let outcomes = compare_sources(&golden, &records, K);
    let compare_elapsed = compare_started.elapsed();
    println!(
        "（耗时：挖题 {:.1}s · 逐来源对照 {:.1}s）",
        mine_elapsed.as_secs_f64(),
        compare_elapsed.as_secs_f64()
    );
    println!();
    let baseline_recall = outcomes
        .iter()
        .find(|outcome| outcome.kind == SourceKind::Baseline)
        .map(|outcome| outcome.recall_at_k)
        .unwrap_or(0.0);

    println!("-- render_source_report（原样输出）--");
    println!("{}", render_source_report(&outcomes, baseline_recall));
    println!();

    print_signal_samples(&golden, &records);
    print_conclusion(&outcomes, baseline_recall);
    Ok(())
}

/// 挖题配置（与 W-20R 同口径）。
fn mining_config() -> MiningConfig {
    MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        max_records_per_case: 3,
        ..MiningConfig::default()
    }
}

/// 打印**每个来源实际挑了什么词**（人工看「像不像信号」）。
fn print_signal_samples(
    golden: &apeireth_memory::golden_set::GoldenSet,
    records: &[(String, String)],
) {
    println!("-- 各来源**实际挑的词**（前 3 题 · 人工核对用）--");
    for kind in SOURCE_KINDS {
        println!("  [{kind:?}]");
        for case in golden.cases.iter().take(3) {
            let signal = signal_for_case(case, records, *kind);
            let extra: Vec<String> = signal
                .terms
                .iter()
                .filter(|term| !case.query.contains(term.as_str()))
                .cloned()
                .collect();
            if extra.is_empty() {
                println!("    · （没取到词 ⇒ 题面不改）  「{}」", case.query);
            } else {
                println!(
                    "    · 「{}（{}）？」",
                    case.query.trim_end_matches('？'),
                    extra.join("、")
                );
            }
        }
    }
    println!();
}

/// 打印结论（**数字全部来自本次实测** ⇒ 不写死、不粉饰）。
fn print_conclusion(
    outcomes: &[apeireth_memory::signal_sources::SourceOutcome],
    baseline_recall: f64,
) {
    println!("-- 结论（数字来自本次实测）--");
    println!("  基线（原题面）recall@10 = {baseline_recall:.4}");
    let mut best: Option<(&apeireth_memory::signal_sources::SourceOutcome, f64)> = None;
    let mut useless: Vec<&apeireth_memory::signal_sources::SourceOutcome> = Vec::new();
    for outcome in outcomes {
        if outcome.kind == SourceKind::Baseline {
            continue;
        }
        let delta = outcome.recall_at_k - baseline_recall;
        println!(
            "  · {:<12} recall@10 = {:.4}（相对基线 {:+.4}）· 出词 {}/{} 题",
            outcome.kind.label(),
            outcome.recall_at_k,
            delta,
            outcome.questions_with_terms,
            outcome.questions_covered
        );
        if outcome.questions_with_terms == 0 {
            useless.push(outcome);
        }
        if best.is_none_or(|(_, current)| delta > current) {
            best = Some((outcome, delta));
        }
    }
    match best {
        Some((outcome, delta)) if delta > 0.0 => {
            println!(
                "  ⇒ **有提升**：最优来源「{}」recall@10 {:+.4}（{:.4} → {:.4}）",
                outcome.kind.label(),
                delta,
                baseline_recall,
                outcome.recall_at_k
            );
        }
        Some((outcome, delta)) => {
            println!(
                "  ⇒ **没有来源真提升 recall**：最优来源「{}」也只有 {:+.4}（持平或更差）",
                outcome.kind.label(),
                delta
            );
        }
        None => {
            println!("  ⇒ 数据不全 ⇒ **不出方向性结论**");
        }
    }
    if !useless.is_empty() {
        let names: Vec<&str> = useless.iter().map(|outcome| outcome.kind.label()).collect();
        println!(
            "  ⇒ ⚠️ **实测无效**的来源（出词 0 题 ⇒ 分数=基线）：{} ⇒ **如实列出，不藏**",
            names.join("、")
        );
    }
    println!(
        "  ⇒ ⚠️ **口径**：这些来源**不含答案信息** ⇒ 与 W-30 的 oracle 上界（紅樓夢 0.6113）\n     **不可直接比**；且本批题是**从记录里挖出来的** ⇒ 「原文首句 / 段落首句」\n     两行含**数据集构造红利** ⇒ 不等于真实用户提问的收益。"
    );
}
