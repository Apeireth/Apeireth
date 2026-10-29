//! 题面**信号增强**的**真实语料对照报告**（《紅樓夢》+《水滸傳》）。
//!
//! # 这个例子回答什么
//!
//! W-20R 的实测结论：**瓶颈不是切分粒度，是查询的判别力**（题面只有主体词一个信号，
//! 而主体在 73% 的记录里都出现）⇒ 本例子回答：
//! **题面多带一个信号，recall@10 到底涨不涨？涨多少？**
//!
//! # 四个候选方案（**每个都真的跑检索**）
//!
//! | 方案 | 题面长什么样 | 加了什么信号 |
//! |---|---|---|
//! | `baseline` | 「寶玉后来是怎么改的？」 | **什么都不加**（原题面） |
//! | `subject` | 同上 | 只有主体（**逐字未改** ⇒ 自检行） |
//! | `subject+predicate` | 「寶玉后来是怎么改的？（限流閾值）？」 | 主体 + 答案片段里的**谓词/对象** |
//! | `subject+answer-term` | 「寶玉后来是怎么改的？（限流閾值）？」 | 主体 + 答案片段里的**另一个实词**（上下文锚） |
//! | `two-subjects` | 「寶玉后来是怎么改的？（黛玉）？」 | 主体 + **第二主体** |
//!
//! # ⚠️ 两条口径提醒（**报告里也会重复一遍**）
//!
//! ① **题面被改写 ⇒ 分数不可与基线直接比**（这是对照实验；基线也跑了一遍放同一张表）。
//! ② **判别力是信息量不是效果**（判别力只回答「候选集缩小多少」）。
//!
//! ⚠️ **再加一条（更重要）**：加进去的信号**来自答案片段** ⇒
//! 跑出来的 recall 是「**信号到位之后**能涨多少」的**上界**
//! ⇒ **不是**「端到端可用」的结论（真上线还需要「信号从哪来」这条链路）。
//!
//! # 口径（钉死，否则数字不可比）
//!
//! * `k = 10`（与 `scripts/golden-baseline.json` 一致）；
//! * 检索 = **生产用的 BM25**（`golden_run::run` 内部 ⇒ 与生产同口径）；
//! * 注入 = **生产口径**（`InjectionMode::Production`）；
//! * 挖题 = **放宽配置**（每记录 6 题 ⇒ 与 2026-10-26 内部卷首跑、W-20R 同口径）；
//! * 可检索粒度 = **整回**（W-20R 已证段落化更差 ⇒ 本批不重复该实验）。
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

use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::golden_mining::{self, MineRecord, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, IndexRecord};
use apeireth_memory::golden_set::GoldenCase;
use apeireth_memory::novel_corpus;
use apeireth_memory::query_signals::{
    compare_schemes, measure_discriminability, parse_signals, render_signal_report, scheme_signals,
    scheme_signals_in_batch, SchemeOutcome, BASELINE_SCHEME, SCHEMES,
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

/// 跑一份语料 ⇒ 打印方案对照表。
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
    let mined = golden_mining::mine(&mine_records, mining_config());
    let cases: Vec<GoldenCase> = mined_to_golden(mined);
    if cases.is_empty() {
        return Err("挖不出题".into());
    }
    let golden = apeireth_memory::golden_set::GoldenSet::new(format!("signal-{name}"), cases);

    println!("════════ {name} ════════");
    println!(
        "语料 {} 回 · 题 {} 道 · k={K} · 检索 = 生产 BM25 · 注入 = 生产口径",
        records.len(),
        golden.len()
    );
    print_subject_stats(&records, &golden.cases);
    println!();

    let outcomes = compare_schemes(&golden, &records, K);
    let baseline_recall = outcomes
        .iter()
        .find(|outcome| outcome.scheme == BASELINE_SCHEME)
        .map(|outcome| outcome.recall_at_k)
        .unwrap_or(0.0);

    println!("-- render_signal_report（原样输出）--");
    println!("{}", render_signal_report(&outcomes, baseline_recall));
    println!();

    println!("-- 逐方案明细（判别力 = 平均候选集收缩比）--");
    println!(
        "  {:<22} {:>10} {:>12} {:>14} {:>12}",
        "方案", "recall@10", "平均判别力", "平均候选记录数", "相对基线"
    );
    for outcome in &outcomes {
        let measured = measure_discriminability(&golden.cases, &records, &outcome.scheme);
        println!(
            "  {:<22} {:>10.4} {:>12.4} {:>14} {:>+12.4}",
            outcome.scheme,
            outcome.recall_at_k,
            outcome.distinctiveness,
            measured.matching_records,
            outcome.recall_at_k - baseline_recall
        );
    }
    println!();

    print_query_samples(&golden.cases);
    print_term_overlap(&golden.cases);
    print_signal_origin(&golden.cases);
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

/// 主体词的**分布**（这批题的瓶颈就在这儿）。
fn print_subject_stats(records: &[(String, String)], cases: &[GoldenCase]) {
    let mut subjects: BTreeMap<String, usize> = BTreeMap::new();
    let mut detected = 0usize;
    for case in cases {
        match parse_signals(case).subject {
            Some(subject) => {
                detected += 1;
                *subjects.entry(subject).or_insert(0) += 1;
            }
            None => {
                *subjects.entry("（认不出）".to_string()).or_insert(0) += 1;
            }
        }
    }
    let total = records.len();
    println!(
        "  主体认得出 {}/{} 题（{:+.1}%）",
        detected,
        cases.len(),
        (detected as f64 / cases.len().max(1) as f64) * 100.0
    );
    // 每个主体出现在多少条记录里（**竞争密度**）⇒ 判别力的根。
    let mut densities: Vec<usize> = Vec::new();
    for (subject, count) in &subjects {
        if subject == "（认不出）" {
            continue;
        }
        let matching = records
            .iter()
            .filter(|(_, text)| text.contains(subject.as_str()))
            .count();
        densities.push(matching);
        if *count >= 3 {
            println!(
                "    · 主体「{subject}」出现在 {matching}/{total} 条记录（{:.1}%）· 命中 {count} 道题",
                matching as f64 / total.max(1) as f64 * 100.0
            );
        }
    }
    densities.sort_unstable();
    let median = densities.get(densities.len() / 2).copied().unwrap_or(0);
    println!(
        "    · 主体竞争密度中位 {median}/{total} 条（{:.1}%）⇒ 判别力上界 ≈ {:.4}",
        median as f64 / total.max(1) as f64 * 100.0,
        1.0 - median as f64 / total.max(1) as f64
    );
}

/// 打印**改写后的题面样本**（人工看「像不像人话」）。
fn print_query_samples(cases: &[GoldenCase]) {
    println!("-- 改写后的题面样本（每个方案取前 2 题）--");
    for scheme in SCHEMES {
        println!("  [{scheme}]");
        for case in cases.iter().take(2) {
            let signals = scheme_signals(case, scheme);
            let extra: Vec<String> = signals
                .iter()
                .filter(|signal| !case.query.contains(&signal.term))
                .map(|signal| signal.term.clone())
                .collect();
            let rewritten = if extra.is_empty() {
                case.query.clone()
            } else {
                format!(
                    "{}（{}）",
                    case.query.trim_end_matches('？'),
                    extra.join("、")
                )
            };
            println!("    · {rewritten}");
        }
    }
    println!();
}

/// 三个增强方案**选中同一个词**的比例 ⇒ 解释「为什么三个分数可能一样」。
fn print_term_overlap(cases: &[GoldenCase]) {
    let schemes = ["subject+predicate", "subject+answer-term", "two-subjects"];
    let mut same_all = 0usize;
    let mut compared = 0usize;
    for case in cases {
        let picks: Vec<Option<String>> = schemes
            .iter()
            .map(|scheme| {
                scheme_signals_in_batch(cases, case, scheme)
                    .get(1)
                    .map(|signal| signal.term.clone())
            })
            .collect();
        let mut unique: Vec<&String> = Vec::new();
        for pick in &picks {
            if let Some(term) = pick {
                if !unique.contains(&term) {
                    unique.push(term);
                }
            }
        }
        compared += 1;
        if unique.len() == 1 {
            same_all += 1;
        }
    }
    println!(
        "-- 三个增强方案**选同一个词**的题数：{same_all}/{compared}（{:.1}%）⇒ 分数必然相同 ⇒ 读表时要看这一行 --",
        same_all as f64 / compared.max(1) as f64 * 100.0
    );
    println!();
}

/// 第二主体的**来源分布**（题面里有 / 从答案片段借的）⇒ 解释「两主体」方案的行为。
fn print_signal_origin(cases: &[GoldenCase]) {
    let mut from_query = 0usize;
    let mut from_fragment = 0usize;
    let mut none = 0usize;
    for case in cases {
        let signals = scheme_signals(case, "two-subjects");
        match signals.get(1) {
            Some(signal) => {
                if case.query.contains(&signal.term) {
                    from_query += 1;
                } else {
                    from_fragment += 1;
                }
            }
            None => none += 1,
        }
    }
    println!(
        "-- 「两主体」方案的第二主体来源：题面里有 {from_query} 题 · 从答案片段借 {from_fragment} 题 · 没有 {none} 题 --"
    );
    println!();
}

/// 打印结论（**数字全部来自本次实测** ⇒ 不写死、不粉饰）。
fn print_conclusion(outcomes: &[SchemeOutcome], baseline_recall: f64) {
    println!("-- 结论（数字来自本次实测）--");
    println!("  基线（原题面）recall@10 = {baseline_recall:.4}");
    let mut best: Option<(&SchemeOutcome, f64)> = None;
    for outcome in outcomes {
        if outcome.scheme == BASELINE_SCHEME {
            continue;
        }
        let delta = outcome.recall_at_k - baseline_recall;
        println!(
            "  · {:<22} recall@10 = {:.4}（相对基线 {:+.4}）· 判别力 {:.4}",
            outcome.scheme, outcome.recall_at_k, delta, outcome.distinctiveness
        );
        if best.is_none_or(|(_, current)| delta > current) {
            best = Some((outcome, delta));
        }
    }
    match best {
        Some((outcome, delta)) if delta > 0.0 => {
            println!(
                "  ⇒ **有提升**：最优方案「{}」recall@10 {:+.4}（{:.4} → {:.4}）",
                outcome.scheme, delta, baseline_recall, outcome.recall_at_k
            );
            println!(
                "  ⇒ ⚠️ 这是**上界**（信号来自答案片段）⇒ 说明「题面多带一个判别信号」**方向有效**，\n     但不等于端到端可用（还需要「信号从哪来」这条链路，且要防答案侧泄漏）。"
            );
        }
        Some((outcome, delta)) => {
            println!(
                "  ⇒ **没有方案真提升 recall**：最优方案「{}」也只有 {:+.4}（持平或更差）",
                outcome.scheme, delta
            );
            println!(
                "  ⇒ ⇒ 判别力涨了但 recall 没涨 ⇒ 瓶颈**不在**「信号不够」，而在**排序**（BM25 用不上这个信号）。"
            );
        }
        None => {
            println!("  ⇒ 数据不全 ⇒ **不出方向性结论**");
        }
    }
}
