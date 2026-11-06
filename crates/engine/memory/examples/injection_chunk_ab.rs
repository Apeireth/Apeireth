//! **注入层 × 切分粒度**的对照实验（**L2 侧**，此前**从未被测过**）。
//!
//! # 这个例子回答什么
//!
//! 已知（W-20R / W-15 / W-28）：
//! * **L1（定位）**：`recall@10 = 0.2983`（《紅樓夢》）⇒ 瓶颈是**查询的判别力**；
//! * **L2（注入）**：`evidence_precision = 0.0092` ⇒ **注入的上下文里几乎没有答案片段**
//!   ⇒ ⇒ **就算排序完美，模型也拿不到答案**；
//! * W-15 比过三种注入形态（head / best-window / tail）⇒ **head 全面胜出**
//!   ⇒ ⚠️ **但那些数字本身只有 0.0100 ~ 0.0223**
//!   ⇒ ⇒ **也就是说「三种都不行」**，只是「head 相对最好」；
//! * W-20 测过**切分粒度**对 **L1** 的影响 ⇒ **反而更差**（0.2983 → 0.0693）。
//!
//! ⚠️ **但从来没人测过**：**切分粒度对 L2 的影响**。
//! ⇒ 而这正是关键：整回一条 **7401 字** ⇒ 答案在**深处**
//! ⇒ ⇒ 生产注入取**前 120 字** ⇒ **答案根本不在里面**。
//! ⇒ ⇒ **段落化恰好治这个**（答案变浅）⇒ 但它会**同时**伤 L1（TF 不再判别）。
//!
//! ⇒ **本实验**：四格对照
//! ⇒ **整回 × 生产注入**（现状）· **整回 × 最相关段**（W-15 的 BestSpan）
//! · **段落 300 × 生产注入** · **段落 300 × 最相关段**
//! ⇒ ⇒ **L1 与 L2 的**取舍**被放在同一张表上**（此前它们分开测 ⇒ 看不见这个取舍）。
//!
//! # 口径（钉死，否则数字不可比）
//!
//! * `k = 10`（与 `scripts/golden-baseline.json` 一致）；
//! * 检索 = **生产 BM25**（`golden_run::run` 内部）；
//! * 挖题 = **放宽配置**（每记录 6 题 ⇒ 与 2026-10-26 内部卷首跑同口径）；
//! * 段落粒度用 **300 字**（W-20 的中间档）；
//! * ⚠️ **段落那一列的题是「映射题」**：同一批题、**题面一字不改**，
//!   只把「期望记录」改指「含答案片段的那个段落」
//!   ⇒ **唯一变量 = 粒度** ⇒ 与整回那一列**可比**；
//!   ⇒ 映射不上的题**整题排除**（排除数**如实报出**）。
//!
//! # ⚠️ 本例子**只回答问题**，不宣称结论
//!
//! ⇒ 「哪种粒度更好」**取决于产品选哪层为重**（要召回还是要送对证据）
//! ⇒ ⇒ **本表是给决策用的输入**，不是「结论」。

use apeireth_memory::chunking::{chunk_records, Chunk, ChunkConfig};
use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::novel_corpus;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const K: usize = 10;
const CHUNK_CHARS: usize = 300;

fn main() -> ExitCode {
    let mut paths: Vec<(String, PathBuf)> = Vec::new();
    for (name, path) in novels() {
        paths.push((name.to_string(), path));
    }
    if paths.is_empty() {
        eprintln!("· 语料不在（$APEIRETH_DATA_DIR/novels/ 或 ~/.apeireth/novels/）⇒ **没测到**");
        return ExitCode::FAILURE;
    }
    for (name, path) in paths {
        if let Err(error) = report_one(&name, &path) {
            eprintln!("· {name}：{error}");
        }
    }
    ExitCode::SUCCESS
}

fn novels() -> Vec<(&'static str, PathBuf)> {
    let mut out: Vec<(&'static str, PathBuf)> = Vec::new();
    if let Ok(dir) = std::env::var("APEIRETH_DATA_DIR") {
        let base = PathBuf::from(dir).join("novels");
        out.push(("redchamber", base.join("redchamber.txt")));
        out.push(("watershed", base.join("watershed.txt")));
    }
    if let Ok(home) = std::env::var("HOME") {
        let base = PathBuf::from(home).join(".apeireth/novels");
        out.push(("redchamber", base.join("redchamber.txt")));
        out.push(("watershed", base.join("watershed.txt")));
    }
    out.retain(|(_, path)| path.exists());
    // 去重（两个来源可能指向同一个文件）。
    let mut seen: Vec<PathBuf> = Vec::new();
    out.retain(|(_, path)| {
        if seen.contains(path) {
            false
        } else {
            seen.push(path.clone());
            true
        }
    });
    out
}

fn mining_config() -> MiningConfig {
    // ⚠️ 口径：每记录 6 题（与 2026-10-26 内部卷首跑同口径 ⇒ 数字可比）。
    MiningConfig {
        max_cases_per_record: 6,
        ..MiningConfig::default()
    }
}

fn golden_set(version: &str, cases: &[GoldenCase]) -> GoldenSet {
    GoldenSet::new(version.to_string(), cases.to_vec())
}

fn mined_to_golden(mined: &[golden_mining::MinedCase]) -> Vec<GoldenCase> {
    mined
        .iter()
        .map(|case| GoldenCase {
            id: case.id.clone(),
            query: case.query.clone(),
            expect_ids: case.expect_ids.clone(),
            expect_fragments: case.expect_fragments.clone(),
            tags: vec![case.kind.clone()],
        })
        .collect()
}

fn chunk_id(chunk: &Chunk) -> String {
    format!("{}#p{:04}", chunk.record_id, chunk.index)
}

fn index_for_chunks(chunks: &[Chunk]) -> Vec<IndexRecord> {
    chunks
        .iter()
        .map(|chunk| IndexRecord {
            id: chunk_id(chunk),
            text: chunk.text.clone(),
        })
        .collect()
}

/// 把题目的「期望记录」映射到「含答案片段的那个段落」（**唯一变量 = 粒度**）。
///
/// ⚠️ 映射不上 ⇒ **整题排除**（排除数**如实报出**，不许悄悄少算题）。
fn map_cases_to_chunks(
    cases: &[GoldenCase],
    bodies: &BTreeMap<&str, &str>,
    chunks: &[Chunk],
) -> (Vec<GoldenCase>, usize) {
    let mut by_record: BTreeMap<&str, Vec<&Chunk>> = BTreeMap::new();
    for chunk in chunks {
        by_record
            .entry(chunk.record_id.as_str())
            .or_default()
            .push(chunk);
    }
    let mut out: Vec<GoldenCase> = Vec::new();
    let mut excluded = 0usize;
    for case in cases {
        if case.expect_ids.len() != case.expect_fragments.len() {
            excluded += 1;
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
            let Some(chunk) = best_chunk_for(expect_id, body, fragment, &by_record) else {
                ok = false;
                break;
            };
            ids.push(chunk_id(chunk));
            fragments.push(overlap_text(body, fragment, chunk));
        }
        if !ok {
            excluded += 1;
            continue;
        }
        let mut mapped = case.clone();
        mapped.expect_ids = ids;
        mapped.expect_fragments = fragments;
        out.push(mapped);
    }
    (out, excluded)
}

/// 找与 `fragment` 重叠最多的段落。
fn best_chunk_for<'a>(
    record_id: &str,
    body: &str,
    fragment: &str,
    by_record: &BTreeMap<&str, Vec<&'a Chunk>>,
) -> Option<&'a Chunk> {
    let candidates = by_record.get(record_id)?;
    let start = body.find(fragment)?;
    let start_char = body[..start].chars().count();
    let end_char = start_char + fragment.chars().count();
    let mut best: Option<(&Chunk, usize)> = None;
    for chunk in candidates {
        let chunk_start = chunk.start_char;
        let chunk_end = chunk_start + chunk.text.chars().count();
        let overlap = end_char
            .min(chunk_end)
            .saturating_sub(start_char.max(chunk_start));
        match best {
            Some((_, current)) if current >= overlap => {}
            _ => best = Some((chunk, overlap)),
        }
    }
    best.map(|(chunk, _)| chunk)
}

/// 段落与片段的**重叠文本**（用于期望片段 ⇒ 与注入文本可比）。
fn overlap_text(body: &str, fragment: &str, chunk: &Chunk) -> String {
    let body_chars: Vec<char> = body.chars().collect();
    let start = body.find(fragment).unwrap_or(0);
    let start_char = body[..start].chars().count();
    let end_char = start_char + fragment.chars().count();
    let chunk_start = chunk.start_char;
    let chunk_end = chunk_start + chunk.text.chars().count();
    let lo = start_char.max(chunk_start);
    let hi = end_char.min(chunk_end);
    if lo >= hi {
        return String::new();
    }
    body_chars[lo..hi].iter().collect()
}

fn report_one(name: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    if records.is_empty() {
        return Err("语料解析出 0 条记录".into());
    }
    let chapter_index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();
    let mined = golden_mining::mine(&records, mining_config());
    let cases = mined_to_golden(&mined);

    // ---------- 整回（基线）----------
    let baseline_set = golden_set("baseline", &cases);
    let whole_production = run(&chapter_index, &baseline_set, K, InjectionMode::Production)?;
    let whole_best_span = run(&chapter_index, &baseline_set, K, InjectionMode::BestSpan)?;

    // ---------- 段落（映射题：唯一变量 = 粒度）----------
    let bodies: BTreeMap<&str, &str> = records
        .iter()
        .map(|record| (record.id.as_str(), record.content.as_str()))
        .collect();
    let pairs: Vec<(String, String)> = records
        .iter()
        .map(|record| (record.id.clone(), record.content.clone()))
        .collect();
    let config = ChunkConfig::new(CHUNK_CHARS, CHUNK_CHARS, CHUNK_CHARS / 2);
    let chunks = chunk_records(&pairs, config);
    let chunk_index = index_for_chunks(&chunks);
    let (mapped, excluded) = map_cases_to_chunks(&cases, &bodies, &chunks);
    let mapped_set = golden_set("chunked", &mapped);
    let chunk_production = run(&chunk_index, &mapped_set, K, InjectionMode::Production)?;
    let chunk_best_span = run(&chunk_index, &mapped_set, K, InjectionMode::BestSpan)?;

    // ---------- 报告 ----------
    println!("════════ {name} ════════");
    println!(
        "语料 {} 回 · 题 {} 道（映射题 {} 道 · 排除 {} 道）· k={K} · 检索 = 生产 BM25",
        chapter_index.len(),
        cases.len(),
        mapped.len(),
        excluded
    );
    println!(
        "整回：{} 条（均长 {} 字） · 段落 {}：{} 条（均长 {} 字）",
        chapter_index.len(),
        average_len(&chapter_index),
        CHUNK_CHARS,
        chunk_index.len(),
        average_len(&chunk_index)
    );
    println!();
    println!(
        "{:<26} {:>12} {:>12} {:>16} {:>16}",
        "口径", "recall@10", "nDCG@10", "evidence_prec", "groundedness"
    );
    println!("{}", "-".repeat(86));
    print_row(
        "① 整回 · 生产注入（现状）",
        &whole_production,
        &whole_production,
    );
    print_row("② 整回 · 最相关段", &whole_best_span, &whole_production);
    print_row("③ 段落 · 生产注入", &chunk_production, &whole_production);
    print_row("④ 段落 · 最相关段", &chunk_best_span, &whole_production);
    println!();

    // ---------- 结论（**如实**，由数据说话）----------
    println!("── 关键对照 ──");
    println!(
        "  L1（召回）：整回生产注入 {:.4} → 段落生产注入 {:.4}（Δ {:+.4}）",
        whole_production.metrics.recall_at_k,
        chunk_production.metrics.recall_at_k,
        chunk_production.metrics.recall_at_k - whole_production.metrics.recall_at_k
    );
    println!(
        "  L2（送对证据）：整回生产注入 {:.4} → 段落生产注入 {:.4}（Δ {:+.4}）",
        whole_production.metrics.evidence_precision,
        chunk_production.metrics.evidence_precision,
        chunk_production.metrics.evidence_precision - whole_production.metrics.evidence_precision
    );
    println!(
        "  L2（题级有据）：整回生产注入 {:.4} → 段落生产注入 {:.4}（Δ {:+.4}）",
        whole_production.metrics.groundedness,
        chunk_production.metrics.groundedness,
        chunk_production.metrics.groundedness - whole_production.metrics.groundedness
    );
    println!(
        "  「最相关段」对照：整回 {:.4} → 段落 {:.4}（evidence_prec）",
        whole_best_span.metrics.evidence_precision, chunk_best_span.metrics.evidence_precision
    );
    println!();
    println!("  ⚠️ 口径提醒：");
    println!("    · 段落那两行的**期望片段被改成段落里的重叠文本** ⇒ 与整回那两行**不完全同题**");
    println!("      （题面一字未改；改的是「答案片段」的口径）⇒ 跨行比较时**记住这一点**；");
    println!("    · `evidence_prec` = 注入片段里**含期望片段**的比例；");
    println!("      `groundedness` = 题级「**至少拿到一条有据证据**」的比例");
    println!("    · 本表是**给决策用的输入**（L1 与 L2 的取舍），**不是**结论");
    Ok(())
}

fn print_row(
    label: &str,
    report: &apeireth_memory::golden_run::RunReport,
    _base: &apeireth_memory::golden_run::RunReport,
) {
    println!(
        "{:<26} {:>12.4} {:>12.4} {:>16.4} {:>16.4}",
        label,
        report.metrics.recall_at_k,
        report.metrics.ndcg_at_k,
        report.metrics.evidence_precision,
        report.metrics.groundedness
    );
}

fn average_len(records: &[IndexRecord]) -> usize {
    if records.is_empty() {
        return 0;
    }
    let total: usize = records.iter().map(|r| r.text.chars().count()).sum();
    total / records.len()
}
