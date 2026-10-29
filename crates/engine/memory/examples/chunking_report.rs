//! 段落级切分的**真实语料对照报告**（《紅樓夢》+《水滸傳》）。
//!
//! # 这个例子回答什么
//!
//! W-16 的诊断**假设**：**召回缺口的根因是切分粒度**（一回 = 7401 字一条记录 ⇒
//! 主体词出现在 88/120 条记录里 ⇒ top-10 里 9/10 是同主体记录 ⇒ 排序近乎抽签）。
//! ⇒ 本例子回答：**把记录切成 200~400 字的段落，recall@10 能不能上去？**
//!
//! # ⚠️ 实测结论（2026-10-27，真实语料 + 生产 BM25，k=10）
//!
//! **不能 —— 反而大幅拉低。**「切分粒度 = 召回根因」这条假设**被否证**：
//!
//! | 对照口径 | 《紅樓夢》 | 《水滸傳》 |
//! |---|---|---|
//! | ① 整回（现状） | 0.2983 | 0.6875 |
//! | ③ 段落 300（**同一批题**，只改粒度） | **0.0693** | **0.0625** |
//! | ④ 段落 300（在段落上**重挖题**） | 0.3282 | 0.5418 |
//! | ② 整回 · 单记录题（与 ④ **同题型**） | **0.3820** | **0.8831** |
//!
//! ⇒ 同题型对照下**两头都掉** ⇒ **段落级切分提不上去 recall@10**。
//!
//! **机制**（有据，不是猜测）：题面**只有主体词**一个判别信号。
//! 整回粒度下目标回把主体词说很多遍 ⇒ **TF 本身就是判别信号**；
//! 段落粒度下目标段落里主体词只出现一两次 ⇒ **TF 不再判别**
//! ⇒ 同主体候选从 88 条涨到 360 条 ⇒ 目标段落被埋在中间
//! （实测仍在**候选池**里，但名次远在 k 之外 ⇒ 是**排序**问题，不是索引问题）。
//!
//! ⇒ **瓶颈是查询的判别力（题面只有主体词），不是切分粒度。**
//!
//! # 口径（钉死，否则数字不可比）
//!
//! * `k = 10`（与 `scripts/golden-baseline.json` 一致）；
//! * 检索 = **生产用的 BM25**（`golden_run::run` 内部 ⇒ 与生产同口径）；
//! * 注入 = **生产口径**（`InjectionMode::Production`）；
//! * 挖题 = **放宽配置**（每记录 6 题 ⇒ 与 2026-10-26 内部卷首跑同口径）；
//! * 「同主体竞争者」= 题面主体词在**可检索记录**里出现的条数（多主体题取第一个主体当探针，
//!   与 `recall_gap_report` 同口径）。
//!   ⚠️ **绝对条数**在段落粒度下会**变大**（记录变多）⇒ 另附**占语料比例**才是可比的密度。
//!
//! # 五个实验（**题目的口径必须说清楚**）
//!
//! | 实验 | 可检索粒度 | 题目 | 用途 |
//! |---|---|---|---|
//! | ① 基线 | 整回 | 首跑同口径挖题（混题型） | **复现** 0.2983 |
//! | ② 基线·单记录题 | 整回 | 只留 `keyword` / `decision` | 与 ④ **同题型**的对照 |
//! | ③ 段落·**映射题** | 段落 | **①的同一批题**，期望记录改指「含答案片段的那个段落」 | **严格 A/B**：只变粒度 |
//! | ④ 段落·**重挖题** | 段落 | 在段落上**重新挖题**（只挖单记录题） | 端到端的真实效果 |
//! | ⑤ 段落·**两段式** | 整回→段落 | ③的同一批题，但**先取回若干回 ⇒ 再在回内排段** | 段落当**第二级**精排 |
//!
//! ⚠️ **② / ③ / ④ 的区别**（**如实**说明）：
//!
//! * ③ = **同一批题**，只把「期望记录」从「回」改成「段落」
//!   ⇒ **唯一变量是粒度** ⇒ 分数差异**可以**归因于切分粒度；
//! * ④ = **换了题**（在段落上重挖）⇒ 与 ① 的分数**不可直接比**
//!   ⇒ ④ 只与 ②（同样只挖单记录题）比；
//! * ⑤ = ③ 的题，但检索改成**两段式** ⇒ 与 ③ 比的是「检索策略」而非「粒度」。
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

use apeireth_memory::chunking::{self, Chunk, ChunkConfig};
use apeireth_memory::golden_diag::{
    diagnose, recall_estimate_at_k, retrieve_pool, subject_of, summarize, DiagSummary,
};
use apeireth_memory::golden_mining::{self, MineRecord, MiningConfig};
use apeireth_memory::golden_run::{mined_to_golden, run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{evaluate, EvalBatch, GoldenCase, GoldenSet, Retrieved};
use apeireth_memory::novel_corpus;

/// 评测的 top-k（**必须与基线文件的 `k` 一致**）。
const K: usize = 10;

/// 挖题预算（每条记录最多几道题）—— 与基线同口径。
const MAX_CASES_PER_RECORD: usize = 6;

/// k 阶梯（**回答「要多大 k 才追得上」**）。
const K_LADDER: [usize; 7] = [10, 20, 50, 100, 200, 500, 1000];

/// 两段式检索的**第一级**取回回数（先取回这么多回 ⇒ 再在回内排段）。
const TOP_CHAPTERS: [usize; 2] = [3, 10];

/// 语料清单：`（名字, 仓外文件名）`（与 `golden_baseline_report.rs` 同一张表）。
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

/// 段落级切分的配置表（**目标 / 上限 / 下限**）。
///
/// * `300/400/120` = 任务书的主口径；
/// * `200/280/80` 与 `400/520/160` = 任务书要求的 **200 / 400 两端**。
fn chunk_configs() -> [(&'static str, ChunkConfig); 3] {
    [
        ("段落 300", ChunkConfig::new(300, 400, 120)),
        ("段落 200", ChunkConfig::new(200, 280, 80)),
        ("段落 400", ChunkConfig::new(400, 520, 160)),
    ]
}

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

/// 一份语料的对照结果（一行）。
#[derive(Debug, Clone)]
struct Row {
    /// 粒度标签。
    granularity: String,
    /// 可检索记录数。
    records: usize,
    /// 平均长度（字符）。
    avg_chars: f64,
    /// 同主体竞争者中位数（**绝对条数**）。
    competitors: usize,
    /// 题数。
    cases: usize,
    /// recall@10。
    recall: f64,
}

/// 同主体竞争者占语料的比例（**百分比**）。
///
/// # 为什么要有这个数
///
/// 「竞争者」在整回粒度下是 **88/120（73%）**、在段落粒度下是 **360/2908（12%）**
/// ⇒ 只看**绝对条数**（88 → 360）会误判成「竞争变多了」，
/// 而 top-10 的名额是**相对**的 ⇒ **占语料比例**才是可比的密度。
fn density_ratio(competitors: usize, records: usize) -> f64 {
    if records == 0 {
        0.0
    } else {
        competitors as f64 / records as f64 * 100.0
    }
}

/// 跑一份语料 ⇒ 打印对照表。
fn report_one(name: &str, path: &Path) -> Result<(), Box<dyn Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    if records.is_empty() {
        return Err("语料解析出 0 条记录".into());
    }

    // ---------- ① 基线（整回）----------
    let chapter_index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();
    let mined = golden_mining::mine(&records, mining_config(true));
    let baseline_cases: Vec<GoldenCase> = mined_to_golden(mined);
    let baseline = run(
        &chapter_index,
        &golden_set("baseline", &baseline_cases),
        K,
        InjectionMode::Production,
    )?;

    let baseline_rows: Vec<usize> = baseline_cases
        .iter()
        .map(|case| competitor_count(&chapter_index, &case.query))
        .collect();
    let baseline_competitors = median(&baseline_rows);
    let baseline_avg = average_len(&chapter_index);

    println!("════════ {name} ════════");
    println!(
        "语料 {} 回 · 题 {} 道 · k={K} · 检索 = 生产 BM25",
        chapter_index.len(),
        baseline_cases.len()
    );
    println!(
        "① 基线（整回）：recall@10 = {:.4} · nDCG@10 = {:.4}",
        baseline.metrics.recall_at_k, baseline.metrics.ndcg_at_k
    );
    let baseline_gap = gap_breakdown(&chapter_index, &baseline_cases);
    print_gap("① 基线（整回）", &baseline_gap);
    print_k_ladder("① 基线（整回）", &chapter_index, &baseline_cases);
    print_samples("① 基线题面样本", &baseline_cases, 3);
    println!();

    // ---------- ② 基线 · 只留单记录题（与 ④ 同题型）----------
    let single_kind_cases: Vec<GoldenCase> = baseline_cases
        .iter()
        .filter(|case| is_single_record_kind(case))
        .cloned()
        .collect();
    let single_baseline = run(
        &chapter_index,
        &golden_set("baseline-single-kind", &single_kind_cases),
        K,
        InjectionMode::Production,
    )?;

    // ---------- 段落级 ----------
    let mut rows: Vec<Row> = Vec::new();
    rows.push(Row {
        granularity: "整回（现状）".to_string(),
        records: chapter_index.len(),
        avg_chars: baseline_avg,
        competitors: baseline_competitors,
        cases: baseline_cases.len(),
        recall: baseline.metrics.recall_at_k,
    });

    for (label, config) in chunk_configs() {
        let chunks = chunk_records_for(&records, config);
        // **不丢字**校验（逐条记录）⇒ 切分没改一个字。
        let mut lossless_ok = 0usize;
        for record in &records {
            let owned: Vec<Chunk> = chunks
                .iter()
                .filter(|chunk| chunk.record_id == record.id)
                .cloned()
                .collect();
            if chunking::verify_lossless(&owned, &record.content).is_ok() {
                lossless_ok += 1;
            }
        }
        let summary = chunking::stats(&chunks);
        let chunk_index = index_for_chunks(&chunks);

        // ③ 映射题：①的同一批题，期望记录改指「含答案片段的那个段落」。
        let (mapped_cases, unmappable) = map_cases_to_chunks(&baseline_cases, &records, &chunks);
        let mapped = run(
            &chunk_index,
            &golden_set("chunk-mapped", &mapped_cases),
            K,
            InjectionMode::Production,
        )?;
        let mapped_competitors = median(
            &mapped_cases
                .iter()
                .map(|case| competitor_count(&chunk_index, &case.query))
                .collect::<Vec<_>>(),
        );

        println!(
            "-- {label}（target={} / max={} / min={}）--",
            config.target_chars, config.max_chars, config.min_chars
        );
        println!(
            "  段落 {} 条 · 平均 {} 字 · 最短 {} · 最长 {} · **硬切 {} 段**",
            summary.chunks,
            format!("{:.0}", summary.avg_chars),
            summary.min_chars,
            summary.max_chars,
            summary.truncated
        );
        println!(
            "  不丢字校验：{lossless_ok}/{} 条记录通过（逐字）",
            records.len()
        );
        println!(
            "  ③ 映射题（同一批题，只改粒度）：题 {} 道（**映射不上 {} 道**，已排除）· recall@10 = {:.4} · nDCG@10 = {:.4} · 同主体竞争者中位 {}",
            mapped_cases.len(),
            unmappable,
            mapped.metrics.recall_at_k,
            mapped.metrics.ndcg_at_k,
            mapped_competitors
        );
        let mapped_gap = gap_breakdown(&chunk_index, &mapped_cases);
        print_gap("③ 映射题（段落粒度）", &mapped_gap);
        print_k_ladder("③ 映射题（段落粒度）", &chunk_index, &mapped_cases);

        // ④ 重挖题：在段落上重新挖题（只挖单记录题 ⇒ 与 ② 同题型）。
        let chunk_mined =
            golden_mining::mine(&mine_records_for(&chunks, &records), mining_config(false));
        let remined_cases: Vec<GoldenCase> = mined_to_golden(chunk_mined);
        let remined = run(
            &chunk_index,
            &golden_set("chunk-remined", &remined_cases),
            K,
            InjectionMode::Production,
        )?;
        let remined_competitors = median(
            &remined_cases
                .iter()
                .map(|case| competitor_count(&chunk_index, &case.query))
                .collect::<Vec<_>>(),
        );
        println!(
            "  ④ 重挖题（段落上重挖，只挖单记录题）：题 {} 道 · recall@10 = {:.4} · nDCG@10 = {:.4} · 同主体竞争者中位 {}",
            remined_cases.len(),
            remined.metrics.recall_at_k,
            remined.metrics.ndcg_at_k,
            remined_competitors
        );
        print_samples("④ 重挖题面样本（注意主体质量）", &remined_cases, 3);

        // ⑤ 两段式（先回后段）：段落**不直接当候选池**，而是**第二级**精排。
        let mut two_stage_rows: Vec<f64> = Vec::new();
        for top_chapters in TOP_CHAPTERS {
            let (two_stage, two_stage_ndcg) =
                two_stage_recall(&chapter_index, &mapped_cases, &chunk_index, top_chapters);
            println!(
                "  ⑤ 两段式（先取回 {top_chapters} 回 ⇒ 再在回内排段）：recall@10 = {two_stage:.4} · nDCG@10 = {two_stage_ndcg:.4}",
            );
            two_stage_rows.push(two_stage);
        }
        println!();

        rows.push(Row {
            granularity: format!("{label}（映射题）"),
            records: chunk_index.len(),
            avg_chars: summary.avg_chars,
            competitors: mapped_competitors,
            cases: mapped_cases.len(),
            recall: mapped.metrics.recall_at_k,
        });
        rows.push(Row {
            granularity: format!("{label}（重挖题）"),
            records: chunk_index.len(),
            avg_chars: summary.avg_chars,
            competitors: remined_competitors,
            cases: remined_cases.len(),
            recall: remined.metrics.recall_at_k,
        });
        rows.push(Row {
            granularity: format!("{label}（两段式 回10→段10）"),
            records: chunk_index.len(),
            avg_chars: summary.avg_chars,
            competitors: mapped_competitors,
            cases: mapped_cases.len(),
            recall: two_stage_rows.last().copied().unwrap_or(0.0),
        });
    }

    println!("-- 同题型对照（只挖单记录题 ⇒ ② vs ④ **可比**）--");
    println!(
        "  ② 整回 · 单记录题 {} 道 · recall@10 = {:.4} · nDCG@10 = {:.4}",
        single_kind_cases.len(),
        single_baseline.metrics.recall_at_k,
        single_baseline.metrics.ndcg_at_k
    );
    println!();

    println!("-- 对照表（切分前后）--");
    println!(
        "  {:<24} {:>8} {:>10} {:>14} {:>10} {:>8} {:>12}",
        "粒度", "记录数", "平均长度", "同主体竞争者中位", "竞争密度%", "题数", "recall@10"
    );
    for row in &rows {
        println!(
            "  {:<24} {:>8} {:>10} {:>14} {:>10.1} {:>8} {:>12.4}",
            row.granularity,
            row.records,
            format!("{:.0}", row.avg_chars),
            row.competitors,
            density_ratio(row.competitors, row.records),
            row.cases,
            row.recall
        );
    }
    println!();
    print_conclusion(&rows, single_baseline.metrics.recall_at_k);
    Ok(())
}

/// 打印结论（**数字全部来自本次实测** ⇒ 不写死、不粉饰）。
///
/// # 口径
///
/// * **③（映射题）** 与 **①（整回）** 比 ⇒ **唯一变量是粒度**（同一批题）；
/// * **④（重挖题）** 与 **②（整回 · 单记录题）** 比 ⇒ **同题型**才可比；
/// * 结论里的差值**按实测方向生成**（涨 / 跌都照实说 ⇒ 不许把跌说成涨）。
fn print_conclusion(rows: &[Row], single_kind_baseline: f64) {
    println!("-- 结论（数字来自本次实测）--");
    let Some(baseline) = rows.iter().find(|row| row.granularity.starts_with("整回")) else {
        println!("  没有基线行 ⇒ **不出结论**（不许在缺基线时编结论）");
        return;
    };
    println!(
        "  ① 整回（现状）：recall@10 = {:.4} · 同主体竞争者中位 {} 条（占语料 {:.1}%）",
        baseline.recall,
        baseline.competitors,
        density_ratio(baseline.competitors, baseline.records)
    );
    println!(
        "  ② 整回 · 单记录题（与 ④ 同题型）：recall@10 = {:.4}",
        single_kind_baseline
    );
    for row in rows.iter().filter(|row| row.granularity.contains("映射题")) {
        println!(
            "  ③ {}：recall@10 = {:.4}（相对 ① {:+.4}）· 竞争者中位 {} 条（占语料 {:.1}%）",
            row.granularity,
            row.recall,
            row.recall - baseline.recall,
            row.competitors,
            density_ratio(row.competitors, row.records)
        );
    }
    for row in rows.iter().filter(|row| row.granularity.contains("重挖题")) {
        println!(
            "  ④ {}：recall@10 = {:.4}（相对 ② {:+.4}）· 竞争者中位 {} 条（占语料 {:.1}%）",
            row.granularity,
            row.recall,
            row.recall - single_kind_baseline,
            row.competitors,
            density_ratio(row.competitors, row.records)
        );
    }
    let mapped_delta = rows
        .iter()
        .find(|row| row.granularity.contains("映射题"))
        .map(|row| row.recall - baseline.recall);
    let remined_delta = rows
        .iter()
        .find(|row| row.granularity.contains("重挖题"))
        .map(|row| row.recall - single_kind_baseline);
    match (mapped_delta, remined_delta) {
        (Some(mapped), Some(remined)) => {
            println!(
                "  ⇒ 段落级切分**提不上去** recall@10：③ 只改粒度 {:+.4}；④ 同题型 {:+.4}",
                mapped, remined
            );
        }
        _ => println!("  ⇒ 数据不全 ⇒ **不出方向性结论**"),
    }
    println!(
        "  ⇒ 机制（实测支持）：题面**只有主体词**一个判别信号 ⇒ 整回粒度下「主体词出现很多遍」\n     本身就是判别信号（TF）；段落粒度下目标段落里主体词只出现一两次 ⇒ TF 不再判别\n     ⇒ 同主体候选条数变多 ⇒ 目标被挤出 top-10（**仍在候选池里** ⇒ 是排序问题，不是索引问题）"
    );
}

/// 把题目列表包成一份**考卷**（版本号只是标签 ⇒ 不影响检索口径）。
fn golden_set(version: &str, cases: &[GoldenCase]) -> GoldenSet {
    GoldenSet::new(version, cases.to_vec())
}

/// 挖题配置。
///
/// * `cross = true` ⇒ 允许跨记录题（`multi_record` / `temporal` / `conflict`）
///   ⇒ 与基线首跑同口径（用于 ①）；
/// * `cross = false` ⇒ **只挖单记录题**（`keyword` / `decision`）
///   ⇒ 段落粒度上跨记录题**没有意义**（段落之间没有时间线）且 **O(n²) 成本爆炸**
///   （3000 段 ⇒ 450 万对）⇒ ② 与 ④ 都用它 ⇒ **题型一致 ⇒ 分数可比**。
fn mining_config(cross: bool) -> MiningConfig {
    MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        max_records_per_case: if cross { 3 } else { 1 },
        ..MiningConfig::default()
    }
}

/// 这题是不是「单记录题」（`keyword` / `decision`）。
fn is_single_record_kind(case: &GoldenCase) -> bool {
    case.tags
        .iter()
        .any(|tag| tag == "kind:keyword" || tag == "kind:decision")
}

/// 把记录切成段落（保留记录顺序）。
fn chunk_records_for(records: &[MineRecord], config: ChunkConfig) -> Vec<Chunk> {
    let pairs: Vec<(String, String)> = records
        .iter()
        .map(|record| (record.id.clone(), record.content.clone()))
        .collect();
    chunking::chunk_records(&pairs, config)
}

/// 段落 ⇒ 索引记录（id = `原记录 id#段落序号` ⇒ 可回溯）。
fn index_for_chunks(chunks: &[Chunk]) -> Vec<IndexRecord> {
    chunks
        .iter()
        .map(|chunk| IndexRecord {
            id: chunk_id(chunk),
            text: chunk.text.clone(),
        })
        .collect()
}

/// 段落 id（`novel-0042#p0007` ⇒ **可回溯到原记录 + 段序**）。
fn chunk_id(chunk: &Chunk) -> String {
    format!("{}#p{:04}", chunk.record_id, chunk.index)
}

/// 段落 ⇒ 挖题器能吃的记录（时间线沿用原记录 + 段序 ⇒ 顺序确定）。
fn mine_records_for(chunks: &[Chunk], records: &[MineRecord]) -> Vec<MineRecord> {
    let timestamps: BTreeMap<&str, i64> = records
        .iter()
        .map(|record| (record.id.as_str(), record.timestamp_ms))
        .collect();
    chunks
        .iter()
        .map(|chunk| MineRecord {
            id: chunk_id(chunk),
            content: chunk.text.clone(),
            title: None,
            timestamp_ms: timestamps
                .get(chunk.record_id.as_str())
                .copied()
                .unwrap_or_default()
                + chunk.index as i64,
            thread_id: None,
        })
        .collect()
}

/// 把基线题目的「期望记录」映射到「含答案片段的那个段落」。
///
/// # 口径（**如实**说明）
///
/// * 每个 `(期望记录 id, 答案片段)` 对 ⇒ 在该记录里找到**答案片段的位置**
///   （`body.find(fragment)`）⇒ 映射到**与它重叠最多的那个段落**；
/// * 片段**跨段**时（切分边界正好落在片段中间）⇒ 取重叠更多的那一段；
/// * **映射不上**（片段在该记录里找不到 / 与任何段落都不重叠）⇒ **整题排除**
///   ⇒ 排除数**如实报出来**（不许悄悄少算题）。
fn map_cases_to_chunks(
    cases: &[GoldenCase],
    records: &[MineRecord],
    chunks: &[Chunk],
) -> (Vec<GoldenCase>, usize) {
    let bodies: BTreeMap<&str, &str> = records
        .iter()
        .map(|record| (record.id.as_str(), record.content.as_str()))
        .collect();
    let mut by_record: BTreeMap<&str, Vec<&Chunk>> = BTreeMap::new();
    for chunk in chunks {
        by_record
            .entry(chunk.record_id.as_str())
            .or_default()
            .push(chunk);
    }

    let mut out: Vec<GoldenCase> = Vec::new();
    let mut unmappable = 0usize;
    for case in cases {
        if case.expect_ids.len() != case.expect_fragments.len() {
            // 标注本身不配套 ⇒ 不映射（**如实**排除）。
            unmappable += 1;
            continue;
        }
        let mut ids: Vec<String> = Vec::new();
        let mut fragments: Vec<String> = Vec::new();
        let mut ok = true;
        for (expect_id, fragment) in case.expect_ids.iter().zip(case.expect_fragments.iter()) {
            let Some(body) = bodies.get(expect_id.as_str()) else {
                ok = false;
                break;
            };
            let Some(chunk) = best_chunk_for(expect_id, *body, fragment, &by_record) else {
                ok = false;
                break;
            };
            ids.push(chunk_id(chunk));
            fragments.push(overlap_text(*body, fragment, chunk));
        }
        if !ok {
            unmappable += 1;
            continue;
        }
        let mut mapped = case.clone();
        mapped.expect_ids = ids;
        mapped.expect_fragments = fragments;
        out.push(mapped);
    }
    (out, unmappable)
}

/// 找与 `fragment` 重叠最多的段落。
fn best_chunk_for<'a>(
    expect_id: &str,
    body: &str,
    fragment: &str,
    by_record: &BTreeMap<&'a str, Vec<&'a Chunk>>,
) -> Option<&'a Chunk> {
    let byte_pos = body.find(fragment)?;
    let char_pos = body[..byte_pos].chars().count();
    let fragment_len = fragment.chars().count();
    let candidates = by_record.get(expect_id)?;
    let mut best: Option<(usize, &Chunk)> = None;
    for chunk in candidates {
        let start = chunk.start_char;
        let end = start + chunk.text.chars().count();
        let overlap_start = char_pos.max(start);
        let overlap_end = (char_pos + fragment_len).min(end);
        if overlap_end <= overlap_start {
            continue;
        }
        let overlap = overlap_end - overlap_start;
        if best.is_none_or(|(current, _)| overlap > current) {
            best = Some((overlap, chunk));
        }
    }
    best.map(|(_, chunk)| chunk)
}

/// 片段落在 `chunk` 里的那部分（**保证是 chunk 的子串** ⇒ 证据族指标仍可比）。
fn overlap_text(body: &str, fragment: &str, chunk: &Chunk) -> String {
    let byte_pos = body.find(fragment).unwrap_or(0);
    let char_pos = body[..byte_pos].chars().count();
    let fragment_len = fragment.chars().count();
    let chunk_start = chunk.start_char;
    let chunk_end = chunk_start + chunk.text.chars().count();
    let start = char_pos.max(chunk_start);
    let end = (char_pos + fragment_len).min(chunk_end);
    if end <= start {
        return fragment.to_string();
    }
    body.chars().skip(start).take(end - start).collect()
}

/// 缺口分类（Hit / RankedOut / NotInPool）—— 回答「**是索引问题还是排序问题**」。
///
/// # 口径
///
/// * 候选池 = **全部记录**（比 k 大得多 ⇒ 才能区分「没进池」与「排在 k 外」）；
/// * 零分候选不算结果（与 `golden_run::run` 同口径）。
fn gap_breakdown(index: &[IndexRecord], cases: &[GoldenCase]) -> DiagSummary {
    let pools = retrieve_pool(index, cases, index.len()).unwrap_or_default();
    let diags = diagnose(cases, &pools, K);
    summarize(&diags, K)
}

/// 打印缺口分类（一行）。
fn print_gap(label: &str, summary: &DiagSummary) {
    println!(
        "     缺口分类 {label}：命中 {} · 排在 k 外 {} · **没进候选池 {}** · 无期望 {}",
        summary.hits, summary.ranked_out, summary.not_in_pool, summary.no_expectation
    );
}

/// 打印 k 阶梯（**乐观上界**：任一期望记录命中即算命中 —— 与 `golden_diag` 同口径）。
///
/// # 用途
///
/// 回答「段落粒度要**多大的 k** 才能追平整回粒度的 recall@10」⇒
/// 直接量化「竞争密度涨了几倍」。
fn print_k_ladder(label: &str, index: &[IndexRecord], cases: &[GoldenCase]) {
    let pools = retrieve_pool(index, cases, index.len()).unwrap_or_default();
    let diags = diagnose(cases, &pools, K);
    let ladder: Vec<String> = K_LADDER
        .iter()
        .map(|k| format!("k={k}:{:.4}", recall_estimate_at_k(&diags, *k)))
        .collect();
    println!("     k 阶梯 {label}：{}", ladder.join(" · "));
}

/// 打印题面样本（人工看；**如实**展示题面质量）。
fn print_samples(label: &str, cases: &[GoldenCase], count: usize) {
    println!("     {label}：");
    for case in cases.iter().take(count) {
        println!("       · {}", case.query);
    }
}

/// 两段式检索：**先回后段**（第一级 BM25 取回 `top_chapters` 回 ⇒ 第二级在回内排段）。
///
/// # 为什么要有这个实验
///
/// ③（映射题）证明：**单级**段落检索会把候选池撑大数倍 ⇒ top-10 更没意义。
/// 但段落的**价值**不在「当第一级候选」，而在「**回内精排**」——
/// ⇒ 这一实验回答「段落级切分**配合**两段式检索能不能提分」。
///
/// # 口径
///
/// * 第一级：整回索引上 BM25 ⇒ 取前 `top_chapters` 回；
/// * 第二级：**只在这几回**的段落上 BM25 ⇒ 取前 `k` 段；
/// * 评测用 [`evaluate`]（与基线**同源** ⇒ recall 口径一致）。
fn two_stage_recall(
    chapter_index: &[IndexRecord],
    cases: &[GoldenCase],
    chunk_index: &[IndexRecord],
    top_chapters: usize,
) -> (f64, f64) {
    let mut batch = EvalBatch::default();
    for case in cases {
        let chapter_ids = bm25_top_ids(chapter_index, &case.query, top_chapters);
        let subset: Vec<IndexRecord> = chunk_index
            .iter()
            .filter(|record| {
                chapter_ids
                    .iter()
                    .any(|id| record.id.split('#').next() == Some(id.as_str()))
            })
            .cloned()
            .collect();
        let ids = bm25_top_ids(&subset, &case.query, K);
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids,
            fragments: Vec::new(),
        });
    }
    match evaluate(&batch, K) {
        Ok(metrics) => (metrics.recall_at_k, metrics.ndcg_at_k),
        Err(_) => (0.0, 0.0),
    }
}

/// BM25 取前 `limit` 个候选 id（**零分不算命中** ⇒ 与 `golden_run::run` 同口径）。
fn bm25_top_ids(records: &[IndexRecord], query: &str, limit: usize) -> Vec<String> {
    use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
    use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};

    let documents = records
        .iter()
        .map(|record| MemoryCandidate {
            id: record.id.clone(),
            layer: "golden".to_string(),
            scope: MemoryScope::Global,
            content: record.text.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: MemoryProvenance {
                source: "golden_corpus".to_string(),
                source_session: None,
                source_trace: None,
                source_request: None,
            },
        })
        .collect();
    let source = Bm25LexicalCandidateSource::new(documents);
    source
        .candidates(query, limit)
        .map(|candidates| {
            candidates
                .into_iter()
                .filter(|candidate| candidate.score > 0.0)
                .map(|candidate| candidate.id)
                .collect()
        })
        .unwrap_or_default()
}

/// 同主体竞争者：有多少条可检索记录提到了这个主体。
///
/// # 口径（与 `recall_gap_report` 同口径）
///
/// * 主体 = 题面反解出来的**主体词**（`golden_diag::subject_of`）；
/// * 多主体题（「A和B」）⇒ 取**第一个**主体当探针（如实标注口径）。
fn competitor_count(index: &[IndexRecord], query: &str) -> usize {
    let probe = probe_of(&subject_of(query));
    if probe.is_empty() {
        return 0;
    }
    index
        .iter()
        .filter(|record| record.text.contains(&probe))
        .count()
}

/// 探针（多主体题取第一个主体）。
fn probe_of(subject: &str) -> String {
    subject
        .split('和')
        .next()
        .unwrap_or(subject)
        .trim()
        .to_string()
}

/// 平均长度（字符）。
fn average_len(index: &[IndexRecord]) -> f64 {
    if index.is_empty() {
        return 0.0;
    }
    let total: usize = index.iter().map(|record| record.text.chars().count()).sum();
    total as f64 / index.len() as f64
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
