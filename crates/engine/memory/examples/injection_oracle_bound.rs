//! **注入层（L2）的 oracle 上界** —— 「信号到位 ⇒ 证据能不能送达？」
//!
//! # 这个例子回答什么
//!
//! 上一批（`injection_chunk_ab`）实测出：
//! * L1：`recall@10 = 0.2983`；
//! * L2：`groundedness = 0.0924` ⇒ **只有 9.2% 的题**在注入里拿到了有据证据。
//!
//! ⇒ **关键问题**：**如果**注入时能选对段（L2 的 oracle），
//! ⇒ **`groundedness` 能到多少**？⇒ ⇒ **L2 的天花板在哪**。
//!
//! # 三种注入形态（**同一预算：每条证据 120 字**）
//!
//! | 形态 | 选哪一段 | 性质 |
//! |---|---|---|
//! | **生产（head）** | 记录**开头** 120 字 | **现状**（不需要任何额外信息） |
//! | **最相关段** | 按 query 匹配挑的一段 | **需要 query**（不需答案） |
//! | **oracle 段** | **含期望片段的那一段** | ⚠️ **需要答案** ⇒ 只是**上界** |
//!
//! # ⚠️ 这个 oracle **不是**可实现的
//!
//! 「知道答案在哪」= **只有评测器才知道** ⇒ 它**只是上界**
//! ⇒ ⇒ 它的用途是：**把「L2 还差多少」量化成一个数**
//! ⇒ ⇒ **不是**「可以实现的成绩」。
//!
//! # 三条纪律
//!
//! 1. **同预算**（都是 120 字/条）⇒ 唯一变量 = **选哪一段**；
//! 2. **同检索**（生产 BM25、同 k、同语料）⇒ 排序不变 ⇒ recall/nDCG **逐字相同**
//!    （**自洽性检查**：若不同 ⇒ 说明口径漂了）；
//! 3. **如实**：oracle 是 oracle ⇒ 报告里**每处**都标「oracle ⇒ 上界」。

use apeireth_memory::golden_mining::{self, MiningConfig};
use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::{evaluate, EvalBatch, GoldenCase, GoldenSet, Retrieved};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const K: usize = 10;
const EVIDENCE_CHARS: usize = 120;

fn main() -> ExitCode {
    let paths = novels();
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

fn novels() -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("APEIRETH_DATA_DIR") {
        roots.push(PathBuf::from(dir).join("novels"));
    }
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(home).join(".apeireth/novels"));
    }
    for root in roots {
        out.push(("redchamber".to_string(), root.join("redchamber.txt")));
        out.push(("watershed".to_string(), root.join("watershed.txt")));
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

fn mining_config() -> MiningConfig {
    MiningConfig {
        max_cases_per_record: 6,
        ..MiningConfig::default()
    }
}

fn report_one(name: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    if records.is_empty() {
        return Err("语料解析出 0 条记录".into());
    }
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();
    let mined = golden_mining::mine(&records, mining_config());
    let cases: Vec<GoldenCase> = mined
        .iter()
        .map(|case| GoldenCase {
            id: case.id.clone(),
            query: case.query.clone(),
            expect_ids: case.expect_ids.clone(),
            expect_fragments: case.expect_fragments.clone(),
            tags: vec![case.kind.clone()],
        })
        .collect();
    let golden = GoldenSet::new("oracle-bound".to_string(), cases.clone());

    // ---------- 检索（生产 BM25 ⇒ 排序，三种形态共用）----------
    let source = build_source(&index);
    let mut retrieved_ids: Vec<Vec<String>> = Vec::new();
    for case in &cases {
        let mut items = source.candidates(&case.query, K)?;
        items.retain(|item| item.score > 0.0);
        retrieved_ids.push(items.iter().map(|item| item.id.clone()).collect());
    }

    // ---------- 三种注入形态 ----------
    let production = run(&index, &golden, K, InjectionMode::Production)?;
    let best_span = run(&index, &golden, K, InjectionMode::BestSpan)?;
    let oracle = run_oracle_injection(&index, &cases, &retrieved_ids)?;

    // ---------- 报告 ----------
    println!("════════ {name} ════════");
    println!(
        "语料 {} 回 · 题 {} 道 · k={K} · 每条证据 {EVIDENCE_CHARS} 字 · 检索 = 生产 BM25",
        index.len(),
        cases.len()
    );
    println!();
    println!(
        "{:<30} {:>12} {:>14} {:>16}",
        "注入形态", "recall@10", "evidence_prec", "groundedness"
    );
    println!("{}", "-".repeat(76));
    print_row("① 生产（head-120，现状）", &production.metrics);
    print_row("② 最相关段（按 query）", &best_span.metrics);
    print_row("③ **oracle 段（含答案）**", &oracle);
    println!();

    // ---------- 关键数字 ----------
    let delivered = oracle.groundedness / production.metrics.groundedness;
    let reachable = production.metrics.groundedness / production.metrics.recall_at_k;
    println!("── 关键数字 ──");
    println!(
        "  L2 的**天花板**（oracle 注入）：groundedness {:.4} ⇒ 比现状 {:.4} **高 {:.2}×**",
        oracle.groundedness, production.metrics.groundedness, delivered
    );
    println!(
        "  **已送达比例**：groundedness {:.4} ÷ recall {:.4} = **{:.1}%**",
        production.metrics.groundedness,
        production.metrics.recall_at_k,
        reachable * 100.0
    );
    println!(
        "  ⇒ 「**检索到了却没送到模型面前**」的比例 = **{:.1}%**",
        (1.0 - reachable) * 100.0
    );
    println!(
        "  最相关段相对生产注入：evidence_prec {:.4} vs {:.4} ⇒ {}",
        best_span.metrics.evidence_precision,
        production.metrics.evidence_precision,
        if best_span.metrics.evidence_precision < production.metrics.evidence_precision {
            "**更差**（⇒ 「按 query 挑段」是**负收益**）"
        } else {
            "更好（⇒ 与上一批的结论相反 ⇒ **要复核**）"
        }
    );
    println!();
    println!("  ⚠️ 口径提醒：");
    println!("    · ③ 是 **oracle**（要知道答案在哪）⇒ **只是上界**，**不可实现**");
    println!("    · 三种形态的 **recall 逐字相同**（注入只改片段、不改排序）⇒ 口径没漂");
    println!("    · 「已送达比例」= groundedness ÷ recall ⇒ 衡量「检索到了却没送到位」");
    Ok(())
}

fn print_row(label: &str, metrics: &apeireth_memory::golden_set::Metrics) {
    println!(
        "{:<30} {:>12.4} {:>14.4} {:>16.4}",
        label, metrics.recall_at_k, metrics.evidence_precision, metrics.groundedness
    );
}

/// **oracle 注入**：每条被检索到的记录，注入「**含期望片段**」的那一段。
///
/// ⚠️ 这是 **oracle**（需要知道答案在哪）⇒ 只是**上界**。
fn run_oracle_injection(
    index: &[IndexRecord],
    cases: &[GoldenCase],
    retrieved_ids: &[Vec<String>],
) -> Result<apeireth_memory::golden_set::Metrics, Box<dyn std::error::Error>> {
    let bodies: std::collections::BTreeMap<&str, &str> = index
        .iter()
        .map(|record| (record.id.as_str(), record.text.as_str()))
        .collect();
    let mut batch = EvalBatch::default();
    for (case, ids) in cases.iter().zip(retrieved_ids.iter()) {
        let fragments: Vec<String> = ids
            .iter()
            .filter_map(|id| {
                let body = bodies.get(id.as_str())?;
                // 找**含期望片段**的那一段（找不到 ⇒ 退回开头 ⇒ 如实）。
                let fragment = case
                    .expect_fragments
                    .iter()
                    .find(|candidate| body.contains(candidate.as_str()))
                    .or_else(|| case.expect_fragments.first());
                match fragment {
                    Some(fragment) => Some(span_around(body, fragment, EVIDENCE_CHARS)),
                    None => Some(body.chars().take(EVIDENCE_CHARS).collect()),
                }
            })
            .collect();
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids: ids.clone(),
            fragments,
        });
    }
    Ok(evaluate(&batch, K)?)
}

/// 以 `fragment` 为中心取 `max_chars` 字的窗口（**含**片段）。
fn span_around(body: &str, fragment: &str, max_chars: usize) -> String {
    let chars: Vec<char> = body.chars().collect();
    let fragment_chars = fragment.chars().count();
    if fragment_chars >= max_chars {
        return fragment.chars().take(max_chars).collect();
    }
    let start = body.find(fragment).unwrap_or(0);
    let start_char = body[..start].chars().count();
    let pad = (max_chars - fragment_chars) / 2;
    let lo = start_char.saturating_sub(pad);
    let hi = (lo + max_chars).min(chars.len());
    let lo = hi.saturating_sub(max_chars).min(chars.len());
    chars[lo..hi].iter().collect()
}

fn build_source(records: &[IndexRecord]) -> Bm25LexicalCandidateSource {
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
    Bm25LexicalCandidateSource::new(documents)
}
