//! **自适应融合权重的可行性探针**（2026-11-06）。
//!
//! # 这个例子回答什么
//!
//! 现在联想通道的融合权重 `w` 是**一个全局常数**（0.45），这是取舍的病根：
//! **一个标量不可能知道「这道题该信哪条路」**。
//!
//! 自研算法要做的第一件事：**把 w 从常数变成「每道题各自算」**。
//! 可用的置信信号现成就有：
//! - **主通道覆盖率**：题面的词有多少出现在它认为最像的那条里；
//! - **主通道命中条数**：有多少条记忆至少沾了一个词；
//! - **主通道头名间距**：第一名比第二名强多少。
//!
//! 逻辑：**主通道自信时别插手；主通道虚时让联想顶上。**
//!
//! # 先要证明的事
//!
//! **主通道弱，能不能预测「联想路手里有答案」？**
//! 能，就有依据；不能，自适应权重就是拍脑袋，**不能做**。
//!
//! # 输出
//!
//! 一行 JSON：两套考卷各自的**分箱命中率**（按主通道覆盖率分箱）。

use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::golden_fair::{
    build_fair_exam, CorpusAssociations, CorpusAssociativeSource, FairExamConfig,
};
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::{mined_to_golden, IndexRecord};
use apeireth_memory::golden_set::GoldenSet;
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};

const K: usize = 10;
const MAX_CASES_PER_RECORD: usize = 6;

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

/// 题面实词（与生产查表键同源的简化版）。
fn terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect()
}

/// 题面的词有多少比例出现在文档里（= 主通道的**覆盖率**）。
fn coverage(query: &str, document: &str) -> f64 {
    let terms = terms(query);
    if terms.is_empty() {
        return 0.0;
    }
    let haystack = document.to_lowercase();
    let hit = terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count();
    hit as f64 / terms.len() as f64
}

/// 一箱的统计。
#[derive(Debug, Default, Clone)]
struct Bin {
    cases: usize,
    channel_has_target: usize,
    main_already_right: usize,
    channel_rescues: usize,
}

impl Bin {
    fn has_target_rate(&self) -> f64 {
        if self.cases == 0 {
            return 0.0;
        }
        self.channel_has_target as f64 / self.cases as f64
    }
}

fn measure(
    name: &str,
    exam: &str,
    golden: &GoldenSet,
    bm25: &Bm25LexicalCandidateSource,
    channel: &CorpusAssociativeSource,
) -> Result<serde_json::Value, Box<dyn Error>> {
    // 分箱：按主通道**顶分的覆盖率**分 4 箱
    let mut bins: Vec<Bin> = vec![Bin::default(); 4];
    for case in &golden.cases {
        let bm25_hits = bm25.candidates(&case.query, K)?;
        let assoc_hits = channel.candidates(&case.query, K)?;

        let cov = bm25_hits
            .first()
            .map(|c| coverage(&case.query, &c.content))
            .unwrap_or(0.0);
        let bin = ((cov * 4.0).floor() as usize).min(3);

        let main_top_ids: Vec<&str> = bm25_hits.iter().map(|c| c.id.as_str()).collect();
        let main_right = !case.expect_ids.is_empty()
            && case
                .expect_ids
                .iter()
                .all(|id| main_top_ids.contains(&id.as_str()));
        let assoc_ids: Vec<&str> = assoc_hits.iter().map(|c| c.id.as_str()).collect();
        let channel_has = !case.expect_ids.is_empty()
            && case
                .expect_ids
                .iter()
                .all(|id| assoc_ids.contains(&id.as_str()));

        let b = &mut bins[bin];
        b.cases += 1;
        if channel_has {
            b.channel_has_target += 1;
        }
        if main_right {
            b.main_already_right += 1;
        }
        if channel_has && !main_right {
            b.channel_rescues += 1;
        }
    }

    let payload: Vec<serde_json::Value> = bins
        .iter()
        .enumerate()
        .map(|(index, bin)| {
            serde_json::json!({
                "箱": format!("覆盖[{:.2},{:.2})", index as f64 / 4.0, (index + 1) as f64 / 4.0),
                "题数": bin.cases,
                "联想路手里有答案": bin.channel_has_target,
                "有答案率": bin.has_target_rate(),
                "主通道已能答对": bin.main_already_right,
                "联想路补上的": bin.channel_rescues,
            })
        })
        .collect();
    Ok(serde_json::json!({ "语料": name, "考卷": exam, "分箱": payload }))
}

fn main() -> ExitCode {
    let mut out: Vec<serde_json::Value> = Vec::new();
    for name in ["redchamber", "watershed"] {
        let path = corpus_root().join(format!("{name}.txt"));
        if !path.exists() {
            eprintln!("· {name}：语料不在，跳过");
            continue;
        }
        let Ok(chapters) = novel_corpus::load_chapters(&path) else {
            eprintln!("· {name}：语料读不了，跳过");
            continue;
        };
        let records = novel_corpus::to_mine_records(&chapters);
        let index: Vec<IndexRecord> = records
            .iter()
            .map(|record| IndexRecord {
                id: record.id.clone(),
                text: record.content.clone(),
            })
            .collect();
        let associations = std::sync::Arc::new(CorpusAssociations::build(&records));
        let channel = CorpusAssociativeSource {
            associations: associations.clone(),
            records: index.clone(),
        };
        let bm25 = Bm25LexicalCandidateSource::new(
            index
                .iter()
                .map(|record| MemoryCandidate {
                    id: record.id.clone(),
                    layer: "golden".to_string(),
                    scope: MemoryScope::Global,
                    content: record.text.clone(),
                    score: 0.0,
                    score_components: Default::default(),
                    provenance: Default::default(),
                })
                .collect(),
        );

        // 词面卷（内部卷）
        let config = MiningConfig {
            max_cases_per_record: MAX_CASES_PER_RECORD,
            ..MiningConfig::default()
        };
        let mined = apeireth_memory::golden_mining::mine(&records, config);
        let lexical = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));
        // 公平卷
        let fair = GoldenSet::new(
            format!("fair-{name}"),
            build_fair_exam(&records, FairExamConfig::default()),
        );

        for (exam, set) in [("词面卷", &lexical), ("公平卷", &fair)] {
            match measure(name, exam, set, &bm25, &channel) {
                Ok(value) => out.push(value),
                Err(error) => eprintln!("· {name}/{exam}：{error}"),
            }
        }
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "自适应探针": out })).unwrap_or_default()
    );
    ExitCode::SUCCESS
}
