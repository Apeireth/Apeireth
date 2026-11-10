//! **内部卷基线**（B0-b 验收：卷子定稿 + **基线数字**）。
//!
//! # 设计稿要什么（§8 B0-b）
//!
//! > 内部卷：从本仓日记/记忆挖 2000 条候选 → 精标 300
//! > 验收：**卷子定稿 + 基线数字**
//!
//! # 本例子做什么
//!
//! 1. 用 `scripts/internal_golden.py --records` 生成的**章节级记录**（JSONL）建索引；
//! 2. 用 `docs/03-reference/internal-golden-v1.jsonl`（**本仓自有资产**）作考卷；
//! 3. 走**生产 BM25**（`golden_run::run` + `InjectionMode::Production`）算 `recall@k` / nDCG /
//!    groundedness / **送达率**。
//!
//! # 口径（钉死）
//!
//! * 记录 = 文档**章节**（`id = sha256(路径#标题)[:12]`，**确定性**）；
//! * 考卷每条的 `expect_fragments` 必须是记录正文的**逐字子串**
//!   （由 `scripts/internal_golden.py --check` 机检，**不是靠自觉**）；
//! * `k = 10`（与其它卷一致）。
//!
//! # 怎么跑
//!
//! ```sh
//! python3 scripts/internal_golden.py --records > /tmp/records.jsonl
//! cargo run -p apeireth-memory --example internal_golden_baseline --locked -- /tmp/records.jsonl
//! ```

use apeireth_memory::delivery_metrics::measure_delivery;
use apeireth_memory::golden_run::{run, IndexRecord, InjectionMode};
use apeireth_memory::golden_set::GoldenSet;
use std::path::PathBuf;
use std::process::ExitCode;

const K: usize = 10;
const GOLDEN: &str = "docs/03-reference/internal-golden-v1.jsonl";

fn main() -> ExitCode {
    let records_path: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp/records.jsonl"));
    let records_text = match std::fs::read_to_string(&records_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!(
                "· 读不到记录文件 {}：{error}\n  ⇒ 先跑 `python3 scripts/internal_golden.py --records > {}`",
                records_path.display(),
                records_path.display()
            );
            return ExitCode::FAILURE;
        }
    };
    // ① 记录：`{"id":…, "text":…}` 每行一条。
    let mut index: Vec<IndexRecord> = Vec::new();
    for line in records_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) else {
            continue;
        };
        let (Some(id), Some(text)) = (
            value.get("id").and_then(|v| v.as_str()),
            value.get("text").and_then(|v| v.as_str()),
        ) else {
            continue;
        };
        index.push(IndexRecord {
            id: id.to_string(),
            text: text.to_string(),
        });
    }
    if index.is_empty() {
        eprintln!("· 记录文件里一条都没解析出来 ⇒ **没测到**");
        return ExitCode::FAILURE;
    }
    // ② 考卷：本仓自有资产（无许可证问题）。
    let golden_text = match std::fs::read_to_string(GOLDEN) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("· 读不到考卷 {GOLDEN}：{error} ⇒ **没测到**");
            return ExitCode::FAILURE;
        }
    };
    let golden = match GoldenSet::load_jsonl(&golden_text) {
        Ok(set) => set,
        Err(error) => {
            eprintln!("· 考卷加载失败：{error:?}");
            return ExitCode::FAILURE;
        }
    };

    println!("════════ 内部卷基线（B0-b）════════");
    println!(
        "记录 **{}** 条（文档章节）· 考卷 **{}** 题 · k = {K} · 生产 BM25",
        index.len(),
        golden.cases.len()
    );
    let report = match run(&index, &golden, K, InjectionMode::Production) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("· 评测失败：{error}");
            return ExitCode::FAILURE;
        }
    };
    let delivery = measure_delivery(&report.metrics);
    println!();
    println!(
        "  L1 `recall@{K}`      = **{:.4}**",
        report.metrics.recall_at_k
    );
    println!("  L1 `nDCG@{K}`        = {:.4}", report.metrics.ndcg_at_k);
    println!(
        "  L2 `groundedness`   = **{:.4}**",
        report.metrics.groundedness
    );
    println!(
        "  L2 `evidence_prec`  = {:.4}",
        report.metrics.evidence_precision
    );
    println!(
        "  **送达率**          = {}",
        delivery
            .delivery_rate
            .map(|rate| format!("**{rate:.4}**"))
            .unwrap_or_else(|| "**没测到**（recall = 0）".to_string())
    );
    println!("  完全命中题数        = {}", report.fully_hit);
    println!();
    println!("  ⚠️ 边界（如实）：");
    println!("    ① ⚠️⚠️ **自造卷红利**：题是**读着这些文档**写的 ⇒ 用词与正文接近");
    println!("       ⇒ ⇒ `recall` 会**偏高**（和当年「两部小说自造卷」是同一类问题）");
    println!("       ⇒ 干净的外部对照请看 LoCoMo（人工标注 evidence / 非自造卷）；");
    println!("    ② 记录 = **文档章节**（不是对话轮次）⇒ 长度与「对话记忆」不同形态；");
    println!(
        "    ② 考卷目前 {} 条 ⇒ **未到 300 条**（B0-b 仍在进行中）；",
        golden.cases.len()
    );
    println!("    ④ 只测检索+注入，**不测答案正确性**（生成侧不在本轮范围）。");
    ExitCode::SUCCESS
}
