//! **融合通道 A/B 报告**（2026-11-05，2026-11-06 扩到两卷）—— 「**不许不涨**」的尺子（B）。
//!
//! # 这个例子回答什么
//!
//! CI 闸门（`scripts/check_golden_baseline.py`）量的是**单路 BM25**
//! ⇒ ⇒ **通道级改动（加召回通道 / 加 as-of / 换融合配方）在那个闸门上完全看不见**
//! ⇒ ⇒ ⇒ 「不许变差」的纪律**对通道级改动是失效的**。
//!
//! 本例子让**同一份考卷跑两遍**：
//! - **基线**：单路 BM25；
//! - **加通道**：BM25 **并联**联想通道；
//! ⇒ ⇒ 差值 = **该通道在这份卷子上的收益**。
//!
//! # ⭐ **两卷并报**（2026-11-06）
//!
//! | 卷 | 主体 | 问什么 |
//! |---|---|---|
//! | **词面卷**（`corpora`） | 记录里**最突出**的实体（内部卷口径） | 「词面最强」的那道题 |
//! | **公平卷**（`fair_corpora`） | 联想实体集里**次级**的实体（[`golden_fair`]） | 「词面弱、但该在场」的那道题 |
//!
//! ⇒ ⇒ **两卷同一套题面模板、同一批记录、同一个 runner、同一把尺子、同一个 k**
//! ⇒ ⇒ **唯一的差别是主体是谁** ⇒ ⇒ **两个 delta 直接可比**。
//!
//! # ⚠️ **口径（读数字前必须知道）**
//!
//! 1. **词面卷的 delta 是**下界**：内部卷的主体 = 该记录的最高频实体
//!    ⇒ ⇒ **词面通道天然占优** ⇒ ⇒ 它测的是「**不许有害**」，不是「**有大用**」。
//! 2. **联想边怎么造的**：写入侧真实来源是内容里的 `associate:<实体> <实体>` 标记
//!    ⇒ ⇒ 小说语料**没有**那种标记 ⇒ ⇒ 这里用**最接近的替身**：
//!    **同一记录内的实词共现** ⇒ ⇒ ⇒ **替身 ⇒ ⇒ 数字只能当参考**
//!    （⇒ ⇒ 公平卷的 delta 是生产收益的**上界**）。
//! 3. **不拼进查询**：额外源与 BM25 **并联**（候选并集 ⇒ **按分数重排** ⇒ 取 top-k）
//!    ⇒ ⇒ 与生产召回链同结构 ⇒ ⇒ **不稀释** BM25。
//!
//! # 输出
//!
//! **一行 JSON**（机器读）：
//! ```json
//! {"k":10,
//!  "corpora":[{"name":"redchamber","exam":"lexical",
//!    "baseline":{"recall_at_k":0.2983,"cases":476},
//!    "with_channel":{"recall_at_k":0.2983,"cases":476},
//!    "delta":0.0}],
//!  "fair_corpora":[{"name":"redchamber","exam":"fair",
//!    "baseline":{"recall_at_k":0.21,"cases":118},
//!    "with_channel":{"recall_at_k":0.26,"cases":118},
//!    "delta":0.05}]}
//! ```

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use apeireth_memory::golden_fair::{
    build_fair_exam, CorpusAssociativeSource, CorpusAssociations, FairExamConfig,
};
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{
    mined_to_golden, run_with_extra_sources, IndexRecord, InjectionMode, RunReport,
};
use apeireth_memory::golden_set::{evaluate, EvalBatch, GoldenSet, Retrieved};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::MemoryCandidateSource;

/// 评测的 top-k（**必须与 `scripts/golden-baseline.json` 的 `k` 一致**）。
const K: usize = 10;

/// 每条记录挖几道题（**与 `golden_baseline_report` 同口径**）。
const MAX_CASES_PER_RECORD: usize = 6;

/// 语料根目录（**与 `golden_baseline_report` 同口径**）。
fn corpus_root() -> PathBuf {
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

/// **一路跑批的结果**（基线 / 加通道）。
struct Abridged {
    baseline: RunReport,
    with_channel: RunReport,
}

/// 跑一份卷子的 A/B（单路 BM25 vs 并联联想通道）。
fn ab(
    index: &[IndexRecord],
    golden: &GoldenSet,
    channel: &CorpusAssociativeSource,
) -> Result<Abridged, apeireth_memory::MemoryError> {
    let baseline = run_with_extra_sources(index, golden, K, InjectionMode::Production, &[])?;
    let with_channel =
        run_with_extra_sources(index, golden, K, InjectionMode::Production, &[channel])?;
    Ok(Abridged {
        baseline,
        with_channel,
    })
}

/// **只有联想通道一路**时的召回（= 通道的**可达性**）。
///
/// ⚠️ **指标口径与融合跑批完全一致**（同一个 [`evaluate`]）⇒ ⇒ 数字可比。
fn channel_only_recall(
    golden: &GoldenSet,
    channel: &CorpusAssociativeSource,
) -> Result<f64, apeireth_memory::MemoryError> {
    let mut batch = EvalBatch::default();
    for case in &golden.cases {
        let candidates = channel.candidates(&case.query, K)?;
        let ids: Vec<String> = candidates.into_iter().map(|candidate| candidate.id).collect();
        batch.cases.push(case.clone());
        batch.results.push(Retrieved {
            ids,
            fragments: Vec::new(),
        });
    }
    let metrics = evaluate(&batch, K)
        .map_err(|error| apeireth_memory::MemoryError::Other(format!("考卷评测失败：{error}")))?;
    Ok(metrics.recall_at_k)
}

/// 把一次 A/B 渲染成 JSON 条目。
fn entry(name: &str, exam: &str, ab: &Abridged, channel_only: f64) -> serde_json::Value {
    let delta = ab.with_channel.metrics.recall_at_k - ab.baseline.metrics.recall_at_k;
    serde_json::json!({
        "name": name,
        "exam": exam,
        "baseline": {
            "recall_at_k": ab.baseline.metrics.recall_at_k,
            "ndcg_at_k": ab.baseline.metrics.ndcg_at_k,
            "cases": ab.baseline.cases,
        },
        "with_channel": {
            "recall_at_k": ab.with_channel.metrics.recall_at_k,
            "ndcg_at_k": ab.with_channel.metrics.ndcg_at_k,
            "cases": ab.with_channel.cases,
        },
        "channel_only_recall_at_k": channel_only,
        "delta": delta,
    })
}

/// 跑一份语料 ⇒ 两卷各一次 A/B。
fn report_one(
    name: &str,
    path: &Path,
) -> Result<(serde_json::Value, serde_json::Value), Box<dyn Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    if records.is_empty() {
        return Err("语料解析出 0 条记录".into());
    }
    let config = MiningConfig {
        max_cases_per_record: MAX_CASES_PER_RECORD,
        ..MiningConfig::default()
    };
    let mined = apeireth_memory::golden_mining::mine(&records, config);
    let lexical = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));
    // ⭐ **公平卷**（零人工标注）：同一套题面模板，只把主体换成
    // 「内部卷不会问的次级实体」，且该实体必须是**跨记录桥**。
    let fair = GoldenSet::new(
        format!("fair-{name}"),
        build_fair_exam(&records, FairExamConfig::default()),
    );
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();

    // ⭐ **联想边**（替身：同记录内的实词共现；**边权 = 实体对的语料级共现次数**）。
    let associations = Arc::new(CorpusAssociations::build(&records));
    let channel = CorpusAssociativeSource {
        associations: associations.clone(),
        records: index.clone(),
    };

    let lexical_ab = ab(&index, &lexical, &channel)?;
    let fair_ab = ab(&index, &fair, &channel)?;
    let lexical_only = channel_only_recall(&lexical, &channel)?;
    let fair_only = channel_only_recall(&fair, &channel)?;

    println!("· {name}：语料 {} 条", index.len());
    println!(
        "    词面卷：题 {} · 基线 recall@{K}={:.4} → 加联想通道={:.4}（delta {:+.4}）· **联想单独** recall@{K}={:.4}",
        lexical_ab.baseline.cases,
        lexical_ab.baseline.metrics.recall_at_k,
        lexical_ab.with_channel.metrics.recall_at_k,
        lexical_ab.with_channel.metrics.recall_at_k - lexical_ab.baseline.metrics.recall_at_k,
        lexical_only,
    );
    println!(
        "    公平卷：题 {} · 基线 recall@{K}={:.4} → 加联想通道={:.4}（delta {:+.4}）· **联想单独** recall@{K}={:.4}",
        fair_ab.baseline.cases,
        fair_ab.baseline.metrics.recall_at_k,
        fair_ab.with_channel.metrics.recall_at_k,
        fair_ab.with_channel.metrics.recall_at_k - fair_ab.baseline.metrics.recall_at_k,
        fair_only,
    );

    Ok((
        entry(name, "lexical", &lexical_ab, lexical_only),
        entry(name, "fair", &fair_ab, fair_only),
    ))
}

fn main() -> ExitCode {
    let dir = corpus_root();
    let mut corpora = Vec::new();
    let mut fair_corpora = Vec::new();
    let mut loaded = 0usize;
    for (name, file) in [
        ("redchamber", "redchamber.txt"),
        ("watershed", "watershed.txt"),
    ] {
        let path = dir.join(file);
        if !path.exists() {
            eprintln!("· {name}：语料不在（{}）⇒ 跳过", path.display());
            continue;
        }
        match report_one(name, &path) {
            Ok((lexical_entry, fair_entry)) => {
                loaded += 1;
                corpora.push(lexical_entry);
                fair_corpora.push(fair_entry);
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
    let payload = serde_json::json!({
        "k": K,
        "corpora": corpora,
        "fair_corpora": fair_corpora,
    });
    println!("{payload}");
    ExitCode::SUCCESS
}
