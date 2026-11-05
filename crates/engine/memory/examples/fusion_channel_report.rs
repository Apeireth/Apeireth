//! **融合通道 A/B 报告**（2026-11-05）—— 「**不许不涨**」的尺子（B）。
//!
//! # 这个例子回答什么
//!
//! CI 闸门（`scripts/check_golden_baseline.py`）量的是**单路 BM25**
//! ⇒ ⇒ **通道级改动（加联想通道 / 加 as-of / 换融合配方）在闸门上完全看不见**
//! ⇒ ⇒ ⇒ 「不许变差」的纪律**对通道级改动是失效的**。
//!
//! 本例子让**同一份考卷跑两遍**：
//! - **基线**：单路 BM25；
//! - **加通道**：BM25 **并联**联想通道；
//! ⇒ ⇒ 差值 = **该通道在内部卷上的收益**。
//!
//! # ⚠️ **口径（读数字前必须知道）**
//!
//! 1. **内部卷的题是「从记录里挖出来的」**（主体 = 该记录的最高频实体）
//!    ⇒ ⇒ **天然偏向词面命中** ⇒ ⇒ **对「联想/词面不像但该在场」这类通道不利**
//!    ⇒ ⇒ ⇒ **本报告的 delta 是**下界**，不是该通道的真实收益**。
//! 2. **联想边怎么造的**：写入侧真实来源是内容里的 `associate:<实体> <实体>` 标记
//!    ⇒ ⇒ 内部卷**没有**那种标记 ⇒ ⇒ 这里用**最接近的替身**：
//!    **同一记录内的高频实体两两共现** ⇒ ⇒ ⇒ **替身 ⇒ ⇒ 数字只能当参考**。
//! 3. **不拼进查询**：额外源与 BM25 **并联**（候选并集、同 id 取高分）
//!    ⇒ ⇒ 与生产召回链同结构 ⇒ ⇒ **不稀释** BM25。
//!
//! # 输出
//!
//! **一行 JSON**（机器读）：
//! ```json
//! {"k":10,"corpora":[{"name":"redchamber",
//!   "baseline":{"recall_at_k":0.6875,"cases":248},
//!   "with_channel":{"recall_at_k":0.6875,"cases":248},
//!   "delta":0.0}]}
//! ```

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use apeireth_memory::backend::in_memory::InMemoryBackend;
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{
    mined_to_golden, run_with_extra_sources, IndexRecord, InjectionMode,
};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::MemoryCandidateSource;
use apeireth_memory::scope::{MemoryCandidate, MemoryProvenance, MemoryScope};
use apeireth_plugin::experience::AssociationStore;

/// 评测的 top-k（**必须与 `scripts/golden-baseline.json` 的 `k` 一致**）。
const K: usize = 10;

/// 每条记录挖几道题（**与 `golden_baseline_report` 同口径**）。
const MAX_CASES_PER_RECORD: usize = 6;

/// **每条记录取几个高频实体做联想边**（替身口径 ⇒ ⇒ 见文件头 §2）。
const ASSOC_ENTITIES_PER_RECORD: usize = 4;

/// **一条记录内的实体共现边**（**替身**：写入侧真实来源是 `associate:` 显式标记）。
#[derive(Default)]
struct CorpusAssociations {
    edges: std::sync::Mutex<Vec<apeireth_plugin::experience::AssociationEdge>>,
}

impl apeireth_plugin::experience::AssociationStore for CorpusAssociations {
    fn record_cooccurrence(
        &self,
        from: &str,
        to: &str,
        episode_id: &str,
    ) -> apeireth_plugin::CapabilityResult<()> {
        self.edges.lock().expect("corpus associations mutex").push(
            apeireth_plugin::experience::AssociationEdge {
                from_entity: from.to_string(),
                to_entity: to.to_string(),
                co_occurrence_count: 1,
                last_seen_episode_id: episode_id.to_string(),
                last_seen_at: 0,
            },
        );
        Ok(())
    }

    fn top_associations(
        &self,
        entity: &str,
        limit: u32,
    ) -> apeireth_plugin::CapabilityResult<Vec<apeireth_plugin::experience::AssociationEdge>> {
        let edges = self.edges.lock().expect("corpus associations mutex");
        Ok(edges
            .iter()
            .filter(|e| e.from_entity == entity || e.to_entity == entity)
            .take(limit as usize)
            .cloned()
            .collect())
    }
}

/// **联想候选源**（本例的替身实现）：查联想网络 ⇒ ⇒ 拉出关联记录作为候选。
///
/// ⚠️ 与生产的 [`apeireth_memory::associative_recall::AssociativeCandidateSource`]
/// **同结构**（并联通道、候选并集）⇒ ⇒ 但**键的取法更简**（整条记录的实体都当键）。
struct CorpusAssociativeSource {
    associations: std::sync::Arc<CorpusAssociations>,
    records: Vec<IndexRecord>,
}

impl MemoryCandidateSource for CorpusAssociativeSource {
    fn candidates(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<MemoryCandidate>, apeireth_memory::MemoryError> {
        let terms = apeireth_memory::query_signals::content_terms_of(query);
        if terms.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let mut out: Vec<MemoryCandidate> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for term in terms {
            let Ok(edges) = self.associations.top_associations(&term, limit as u32) else {
                continue;
            };
            for edge in edges {
                if seen.contains(&edge.last_seen_episode_id) {
                    continue;
                }
                seen.push(edge.last_seen_episode_id.clone());
                let Some(record) = self
                    .records
                    .iter()
                    .find(|r| r.id == edge.last_seen_episode_id)
                else {
                    continue;
                };
                out.push(MemoryCandidate {
                    id: record.id.clone(),
                    layer: "golden".to_string(),
                    scope: MemoryScope::Global,
                    content: record.text.clone(),
                    score: 0.0,
                    score_components: Default::default(),
                    provenance: MemoryProvenance {
                        source: "golden_association".to_string(),
                        source_session: None,
                        source_trace: None,
                        source_request: None,
                    },
                });
            }
        }
        Ok(out)
    }
}

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

/// 跑一份语料 ⇒ 单路 vs 加联想通道。
fn report_one(name: &str, path: &Path) -> Result<serde_json::Value, Box<dyn Error>> {
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
    let golden = apeireth_memory::golden_set::GoldenSet::new(
        format!("internal-{name}"),
        mined_to_golden(mined),
    );
    let index: Vec<IndexRecord> = records
        .iter()
        .map(|record| IndexRecord {
            id: record.id.clone(),
            text: record.content.clone(),
        })
        .collect();

    // ⭐ **替身联想边**：同一记录内的高频实体两两共现。
    let associations = std::sync::Arc::new(CorpusAssociations::default());
    for record in &records {
        let entities = apeireth_memory::query_signals::content_terms_of(&record.content);
        let mut unique: Vec<String> = Vec::new();
        for entity in entities {
            if !unique.contains(&entity) {
                unique.push(entity);
            }
            if unique.len() >= ASSOC_ENTITIES_PER_RECORD {
                break;
            }
        }
        for i in 0..unique.len() {
            for j in (i + 1)..unique.len() {
                let _ = associations.record_cooccurrence(&unique[i], &unique[j], &record.id);
            }
        }
    }

    // ① **基线**：单路 BM25（**与 CI 闸门同口径**）。
    let baseline = run_with_extra_sources(&index, &golden, K, InjectionMode::Production, &[])?;

    // ② **加通道**：BM25 **并联**联想通道。
    let channel = CorpusAssociativeSource {
        associations: associations.clone(),
        records: index.clone(),
    };
    let with_channel =
        run_with_extra_sources(&index, &golden, K, InjectionMode::Production, &[&channel])?;

    let delta = with_channel.metrics.recall_at_k - baseline.metrics.recall_at_k;
    println!(
        "· {name}：基线 recall@{K}={:.4} → 加联想通道 recall@{K}={:.4}（delta {:+.4}）· 题 {} → {}",
        baseline.metrics.recall_at_k,
        with_channel.metrics.recall_at_k,
        delta,
        baseline.cases,
        with_channel.cases,
    );

    Ok(serde_json::json!({
        "name": name,
        "baseline": {
            "recall_at_k": baseline.metrics.recall_at_k,
            "ndcg_at_k": baseline.metrics.ndcg_at_k,
            "cases": baseline.cases,
        },
        "with_channel": {
            "recall_at_k": with_channel.metrics.recall_at_k,
            "ndcg_at_k": with_channel.metrics.ndcg_at_k,
            "cases": with_channel.cases,
        },
        "delta": delta,
    }))
}

fn main() -> ExitCode {
    let dir = corpus_root();
    let mut corpora = Vec::new();
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
    let payload = serde_json::json!({ "k": K, "corpora": corpora });
    println!("{payload}");
    ExitCode::SUCCESS
}
