//! **粒度–送达率曲线**：证明「送达率」由「单元能不能放进注入窗口」决定。
//!
//! # 这个例子要证的那一句
//!
//! 生产注入**每条证据只给 120 字**（`memory_injection::EVIDENCE_MAX_CHARS`）。
//! 挖题把答案片段放在正文 **60% 深度**处 ⇒ ⇒
//! ⇒ ⇒ **若单元正文 > 120 字，答案就被窗口截掉**（⇒ 送达失败）。
//!
//! ⇒ 所以预测是一条**单调曲线**：单元越接近/小于窗口 ⇒ 送达率越高。
//!
//! # ⚠️ 先说清**预测**（跑之前写下来 ⇒ 防止事后编故事）
//!
//! | 单元长度 | 60% 深度处的答案位置 | **预测送达率** |
//! |---|---|---|
//! | 整回（约 7,401 字） | 约 4,440 字 ⇒ **远超窗口** | 低（历史实测 0.31 / 0.69） |
//! | 800 字 | 480 字 ⇒ 超窗口 | 低 |
//! | 400 字 | 240 字 ⇒ 超窗口 | 低 |
//! | 200 字 | 120 字 ⇒ **正好在边缘** | 中 / 不稳 |
//! | **120 字** | 72 字 ⇒ **在窗口内** | **应接近 1.0** |
//!
//! ⇒ ⇒ 若实测**不**呈这条形状 ⇒ ⇒ **如实说不符**（不许拿「感觉」圆过去）。
//!
//! # 纪律
//!
//! * **同一批题**（先在整回上挖题，再映射到段落）⇒ 唯一变量 = 粒度；
//! * 映射不上的题**整题排除**并**如实报排除数**（不许悄悄少算）；
//! * 送达率 = `groundedness / recall` ⇒ ⇒ **只在「每题 1 条答案」的口径下有意义**
//!   （挖题产出的题正是这种形态 ⇒ 可相除）。
//!
//! # 怎么跑
//!
//! ```sh
//! cargo run -p apeireth-memory --example granularity_delivery_curve --locked
//! ```

use apeireth_memory::chunking::{chunk_records, Chunk, ChunkConfig};
use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{GoldenCase, GoldenSet};
use apeireth_memory::memory_injection::EVIDENCE_MAX_CHARS;
use apeireth_memory::novel_corpus;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;

const K: usize = 10;

/// 扫的粒度（字符）。最后一个是**注入窗口本身**。
const SIZES: [usize; 4] = [800, 400, 200, 120];

fn main() -> ExitCode {
    let paths = novels();
    if paths.is_empty() {
        eprintln!("· 语料不在（$APEIRETH_DATA_DIR/novels/ 或 ~/.apeireth/novels/）⇒ **没测到**");
        return ExitCode::FAILURE;
    }
    println!("════════ 粒度–送达率曲线 ════════");
    println!(
        "注入窗口 = **{} 字**（生产逐字口径）· 答案片段位于正文 **60% 深度**",
        EVIDENCE_MAX_CHARS
    );
    println!();
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

/// 一行结果。
struct Row {
    label: String,
    /// **平均单元字数**（= 粒度本身 ⇒ 单调性判定的自变量）。
    ///
    /// ⚠️ **不要**用 `unit_count` 当代理：单元数多 ⇒ 单元**小**（方向相反）
    /// ⇒ ⇒ 上一次就是这么把结论**判反**的。
    mean_unit_chars: usize,
    unit_count: usize,
    cases: usize,
    excluded: usize,
    recall: f64,
    grounded: f64,
    delivery: f64,
}

fn report_one(name: &str, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    if chapters.is_empty() {
        return Err("语料解析出 0 条记录".into());
    }
    let records = novel_corpus::to_mine_records(&chapters);
    // ⇒ **同一批题**：在整回上挖一次，之后映射到各粒度。
    let mined = golden_mining::mine(
        &records,
        MiningConfig {
            max_cases_per_record: 6,
            ..MiningConfig::default()
        },
    );
    let base_cases: Vec<GoldenCase> = mined
        .iter()
        .map(|case| GoldenCase {
            id: case.id.clone(),
            query: case.query.clone(),
            expect_ids: case.expect_ids.clone(),
            expect_fragments: case.expect_fragments.clone(),
            tags: vec![case.kind.clone()],
        })
        .collect();
    if base_cases.is_empty() {
        return Err("没挖出题".into());
    }
    let bodies: BTreeMap<&str, &str> = records
        .iter()
        .map(|record| (record.id.as_str(), record.content.as_str()))
        .collect();

    println!("════════ {name} ════════");
    println!(
        "{} 回 · 挖出 {} 题（**同一批题**跨粒度可比）",
        chapters.len(),
        base_cases.len()
    );
    println!();
    println!(
        "{:<14} {:>9} {:>7} {:>8} {:>10} {:>10} {:>9}",
        "粒度", "单元数", "题数", "排除", "recall@10", "grounded", "送达率"
    );
    println!("{}", "-".repeat(72));

    let mut rows: Vec<Row> = Vec::new();

    // ---------- 基线：整回 ----------
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();
    let mean_chars = if records.is_empty() {
        0
    } else {
        records
            .iter()
            .map(|record| record.content.chars().count())
            .sum::<usize>()
            / records.len()
    };
    if let Some((recall, grounded)) = run_arm(&index, &base_cases) {
        let delivery = if recall > 0.0 {
            grounded / recall
        } else {
            f64::NAN
        };
        println!(
            "{:<14} {:>9} {:>7} {:>8} {:>10.4} {:>10.4} {:>9.4}",
            format!("整回(~{mean_chars})"),
            index.len(),
            base_cases.len(),
            0,
            recall,
            grounded,
            delivery
        );
        rows.push(Row {
            label: format!("整回(~{mean_chars})"),
            mean_unit_chars: mean_chars,
            unit_count: index.len(),
            cases: base_cases.len(),
            excluded: 0,
            recall,
            grounded,
            delivery,
        });
    }

    // ---------- 各粒度 ----------
    for size in SIZES {
        let config = ChunkConfig::new(size, size, size / 4);
        let pairs: Vec<(String, String)> = records
            .iter()
            .map(|record| (record.id.clone(), record.content.clone()))
            .collect();
        let chunks = chunk_records(&pairs, config);
        if chunks.is_empty() {
            continue;
        }
        let chunk_index: Vec<IndexRecord> = chunks
            .iter()
            .map(|chunk| IndexRecord {
                id: chunk_id(chunk),
                text: chunk.text.clone(),
            })
            .collect();
        let (mapped, excluded) = map_cases_to_chunks(&base_cases, &bodies, &chunks);
        if mapped.is_empty() {
            println!("{size:<14} **映射不上任何题**");
            continue;
        }
        if let Some((recall, grounded)) = run_arm(&chunk_index, &mapped) {
            let delivery = if recall > 0.0 {
                grounded / recall
            } else {
                f64::NAN
            };
            println!(
                "{:<14} {:>9} {:>7} {:>8} {:>10.4} {:>10.4} {:>9.4}",
                format!("段落 {size}"),
                chunk_index.len(),
                mapped.len(),
                excluded,
                recall,
                grounded,
                delivery
            );
            let chunk_mean = chunk_index
                .iter()
                .map(|record| record.text.chars().count())
                .sum::<usize>()
                / chunk_index.len().max(1);
            rows.push(Row {
                label: format!("段落 {size}"),
                mean_unit_chars: chunk_mean,
                unit_count: chunk_index.len(),
                cases: mapped.len(),
                excluded,
                recall,
                grounded,
                delivery,
            });
        }
    }
    println!();

    // ---------- 判据（**跑前写好的预测**）----------
    println!("── 预测 vs 实测 ──");
    let fits = rows
        .iter()
        .find(|row| row.label == format!("段落 {EVIDENCE_MAX_CHARS}"));
    match fits {
        Some(row) if row.delivery.is_finite() => {
            let verdict = if row.delivery >= 0.9 {
                "**符合预测**：单元放进窗口 ⇒ 送达率接近 1.0"
            } else {
                "⚠️ **不符合预测**：单元已放进窗口，送达率却没上去 ⇒ 要查清原因"
            };
            println!(
                "  段落 {} 的送达率 = {:.4} ⇒ {}",
                EVIDENCE_MAX_CHARS, row.delivery, verdict
            );
        }
        _ => println!(
            "  ⚠️ 没有段落 {} 这一档 ⇒ 无法判定（如实标出）",
            EVIDENCE_MAX_CHARS
        ),
    }
    // 单调性检查：**按平均单元字数升序**排 ⇒ 送达率应**非增**（单元越大越差）。
    //
    // ⚠️ 踩过的坑：先前拿 `unit_count` 当「粒度」的代理 ⇒ ⇒ 但两者方向**相反**
    // （单元数多 = 单元**小**）⇒ ⇒ **把结论判反了** ⇒ ⇒ 现在直接用 `mean_unit_chars`。
    let mut ordered: Vec<&Row> = rows.iter().collect();
    ordered.sort_by_key(|row| row.mean_unit_chars);
    let mut violations: Vec<(String, String)> = Vec::new();
    for window in ordered.windows(2) {
        let (small, big) = (&window[0], &window[1]);
        // 单元**更大**却送达率**更高** ⇒ 违反「越大越差」。
        if big.mean_unit_chars > small.mean_unit_chars
            && big.delivery.is_finite()
            && small.delivery.is_finite()
            && big.delivery > small.delivery
        {
            violations.push((big.label.clone(), small.label.clone()));
        }
    }
    println!(
        "  「**单元越大 ⇒ 送达率越低**」的单调性：{}",
        if violations.is_empty() {
            format!("**成立**（{} 档全部同向）", ordered.len())
        } else {
            format!(
                "⚠️ **不成立**：{} 处反向 {:?}",
                violations.len(),
                violations
            )
        }
    );
    print!("  · 细节：");
    for row in &ordered {
        print!(
            "{}[单元{}·{}个·{}题·排除{}·r{:.3}/g{:.3}] ",
            row.label,
            row.mean_unit_chars,
            row.unit_count,
            row.cases,
            row.excluded,
            row.recall,
            row.grounded
        );
    }
    println!();
    println!();
    Ok(())
}

fn chunk_id(chunk: &Chunk) -> String {
    format!("{}#p{:04}", chunk.record_id, chunk.index)
}

/// 跑一组题 ⇒ `(recall, groundedness)`。
fn run_arm(index: &[IndexRecord], cases: &[GoldenCase]) -> Option<(f64, f64)> {
    let golden = GoldenSet::new("arm".to_string(), cases.to_vec());
    let report = run(index, &golden, K, InjectionMode::Production).ok()?;
    Some((report.metrics.recall_at_k, report.metrics.groundedness))
}

/// 把题目的「期望记录」映射到「含答案片段的那个段落」（**唯一变量 = 粒度**）。
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

/// 取 `fragment` 与该段落的交集文本（⇒ 保证片段仍是该段落正文的子串）。
fn overlap_text(body: &str, fragment: &str, chunk: &Chunk) -> String {
    let start = body.find(fragment).unwrap_or(0);
    let start_char = body[..start].chars().count();
    let end_char = start_char + fragment.chars().count();
    let chunk_start = chunk.start_char;
    let chunk_end = chunk_start + chunk.text.chars().count();
    let from = start_char.max(chunk_start);
    let to = end_char.min(chunk_end);
    if to <= from {
        return fragment.to_string();
    }
    chunk
        .text
        .chars()
        .skip(from - chunk_start)
        .take(to - from)
        .collect()
}
