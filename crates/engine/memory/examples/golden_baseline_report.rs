//! 内部卷检索基线的**当前分数**报告器。
//!
//! # 用途
//!
//! `scripts/check_golden_baseline.py` 调它跑出**当前真实分数** ⇒ 与
//! `scripts/golden-baseline.json` 里的**基线**比 ⇒ 掉了就**非零退出**。
//!
//! ⚠️ **为什么检索在 Rust 侧、比较在脚本侧**
//! ⇒ 检索是 Rust 的活（生产同口径的 BM25）⇒ **不许在 Python 里重实现检索**
//! （两套实现 ⇒ 两个分数 ⇒ 基线就失去意义了）⇒ 本 example 负责「跑」，
//! 脚本负责「比」。
//!
//! # 口径（必须与基线一致，否则分数不可比）
//!
//! * `k = 10`（基线文件里的 `k`）；
//! * 检索 = **生产用的 BM25**（`golden_run::run` 内部）⇒ 与生产同口径；
//! * 注入 = **生产口径**（`InjectionMode::Production`：每条证据取正文前 120 字）
//!   ⇒ 与 2026-10-26 内部卷首跑同口径；
//! * 挖题 = **放宽配置**（每记录 6 题；其余取 [`MiningConfig::default`]）
//!   ⇒ 与首跑同口径（《紅樓夢》476 题 / 《水滸傳》248 题）。
//!
//! # 语料在仓外
//!
//! 语料在 `$APEIRETH_DATA_DIR/novels/`（默认 `~/.apeireth/novels/`），
//! 下载器 `scripts/download_novels.sh` 带 SHA-256 校验。
//! ⇒ **某本语料缺失 ⇒ 跳过它并如实说明**（stderr），**不许**假装跑过。
//! ⇒ 一本都没有 ⇒ **非零退出**（否则「没语料」会被读成「分数没问题」）。
//!
//! # 输出
//!
//! **stdout**：**一行 JSON**（机器读）：
//! `{"k":10,"corpora":[{"name":"redchamber","recall_at_k":...,"ndcg_at_k":...,"cases":...}]}`
//!
//! **stderr**：给人看的说明（语料缺失、跳过等）。
//!
//! # 状态：NOT IMPLEMENTED（新增未接线，2026-10-26）
//!
//! 四级口径：
//!
//! - **IMPLEMENTED** ✔：能跑出真实分数（库级实现，仅此一级）；
//! - **PRODUCTION WIRED** ✘：生产主路径 0 调用点（只有门禁脚本在用）；
//! - **DEFAULT ENABLED** ✘：没有开关位；
//! - **HARDWARE VALIDATED** ✔（部分）：已在本机**真实语料**上跑过
//!   （2026-10-26），**未在第二台机器复现** ⇒ 不宣称跨机器可复现。

use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{mined_to_golden, run, IndexRecord, InjectionMode};
use apeireth_memory::novel_corpus;
use serde_json::json;

/// 评测的 top-k（**必须与基线文件的 `k` 一致** ⇒ 改了 k 就换基线版本）。
const K: usize = 10;

/// 挖题预算（每条记录最多几道题）—— 「放宽配置」⇒ 题量翻倍 ⇒ 统计功效更好。
///
/// ⚠️ 与 2026-10-26 内部卷首跑**逐项一致**（《紅樓夢》476 题 / 《水滸傳》248 题）
/// ⇒ 改这个数 ⇒ **分数不可比** ⇒ 必须换基线版本。
const MAX_CASES_PER_RECORD: usize = 6;

/// 语料清单：`（基线里的语料名, 仓外文件名）`。
///
/// ⚠️ **与 `scripts/check_golden_baseline.py` 里的同一张表保持一致**
/// （脚本要先判断语料在不在，才决定跑不跑）。
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

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

fn main() -> ExitCode {
    let dir = novels_dir();
    let mut corpora = Vec::new();
    let mut loaded = 0usize;

    for (name, filename) in CORPORA {
        let path = dir.join(filename);
        if !path.exists() {
            // ⚠️ **如实说明**：跳过 ≠ 通过（脚本那边也会先查语料在不在）。
            eprintln!(
                "· {name}：语料不在（{}）⇒ **跳过**（不是「跑过了」）",
                path.display()
            );
            continue;
        }
        match report_one(name, &path) {
            Ok(entry) => {
                loaded += 1;
                corpora.push(entry);
            }
            Err(error) => {
                eprintln!("· {name}：**跑失败**（{}）：{error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    if loaded == 0 {
        eprintln!(
            "一份语料都没跑到（目录 {}）⇒ **不能出分数**（不许把「没跑」当「没问题」）",
            dir.display()
        );
        return ExitCode::FAILURE;
    }

    // **一行 JSON**（机器读）⇒ 脚本解析它。
    let payload = json!({
        "k": K,
        "corpora": corpora,
    });
    println!("{payload}");
    ExitCode::SUCCESS
}

/// 跑一份语料 ⇒ 当前分数（recall@k / nDCG@k / 题数）。
fn report_one(
    name: &str,
    path: &std::path::Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let config = MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = apeireth_memory::golden_mining::mine(&records, config);
    let golden = apeireth_memory::golden_set::GoldenSet::new(
        format!("internal-{name}"),
        mined_to_golden(mined),
    );

    // 可检索文本 = **正文**（与首跑同口径；题面来源不进可检索文本 ⇒ 不泄漏答案）。
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();

    let report = run(&index, &golden, K, InjectionMode::Production)?;
    // ⚠️ **送达率**（2026-10-27 加）：把「检索到了却没送到模型面前」变成**常驻数字**。
    // ⇒ 它**不参与**基线闸门（闸门只管 recall/ndcg ⇒ 那是「检索准不准」）
    // ⇒ 但**每次跑批都要看见它**（否则 L2 的恶化会**静默**发生）。
    let delivery = apeireth_memory::delivery_metrics::measure_delivery(&report.metrics);
    eprintln!(
        "· {name}：{} 回 · {} 题 · recall@{K}={:.4} · nDCG@{K}={:.4} · groundedness={:.4} · **送达率={}**",
        report.corpus_size,
        report.cases,
        report.metrics.recall_at_k,
        report.metrics.ndcg_at_k,
        report.metrics.groundedness,
        delivery
            .delivery_rate
            .map(|rate| format!("{rate:.4}"))
            .unwrap_or_else(|| "没测到".to_string())
    );
    Ok(json!({
        "name": name,
        "recall_at_k": report.metrics.recall_at_k,
        "ndcg_at_k": report.metrics.ndcg_at_k,
        "cases": report.cases,
        // ⚠️ **观察项**（**不参与闸门**）：注入族指标 + 送达率
        "groundedness": report.metrics.groundedness,
        "evidence_precision": report.metrics.evidence_precision,
        "delivery_rate": delivery.delivery_rate,
    }))
}
