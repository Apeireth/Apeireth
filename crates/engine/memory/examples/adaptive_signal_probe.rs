//! **自适应融合信号探针**（2026-11-07）—— 找**真正有分辨力**的「主通道信心」信号。
//!
//! # 为什么有这个
//! 联想通道的融合权重现在是**一个常数**（0.20）。实测显示：
//! 通道与底座**此消彼长**（底座越强，通道在「词面强的题」上越挤）
//! ⇒ ⇒ ⇒ 理想是**按题自适应**：主通道虚时让联想顶上，主通道自信时让位。
//!
//! 2026-11-06 我试过「**主通道覆盖率**」⇒ ⇒ **失败**：四组题全部落进同一个箱
//! ⇒ ⇒ ⇒ 该信号**没有分辨力**。⇒ ⇒ ⇒ 这里换**多轮对话语料**（LoCoMo）
//! ⇒ ⇒ ⇒ 再测**四个候选信号**，看哪个能预测「**联想路手里有没有答案**」。
//!
//! | 信号 | 含义 |
//! |---|---|
//! | `top_cover` | 主通道顶分的**覆盖率**（旧方案，已证失败）|
//! | `hit_count` | 主通道**命中条数**（有多少条至少沾了一个词）|
//! | `margin` | 主通道**头名间距**（第一名比第二名强多少）|
//! | `top_score` | 主通道顶分的**绝对值**（归一化前）|
//!
//! # 判据
//! 一个信号**有分辨力** ⇔ **按它分箱后，各箱的「联想路有答案率」明显不同**
//! ⇒ ⇒ **分辨力 = 各箱有答案率的极差**。
//!
//! # 输出
//! 一行 JSON。

use std::error::Error;
use std::process::ExitCode;

use apeireth_memory::locomo_corpus;
use apeireth_memory::retrieval_pipeline::{Bm25LexicalCandidateSource, MemoryCandidateSource};
use apeireth_memory::{MemoryCandidate, MemoryScope};
use apeireth_plugin::experience::AssociationStore;

const K: usize = 10;

/// 词面覆盖率（题面的词有多少比例出现在文档里）。
fn coverage(query: &str, document: &str) -> f64 {
    let terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect();
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
#[derive(Default, Clone)]
struct Bin {
    cases: usize,
    channel_has_target: usize,
}

fn main() -> ExitCode {
    if let Err(error) = run() {
        eprintln!("· 失败：{error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn run() -> Result<(), Box<dyn Error>> {
    let Some(path) = locomo_corpus::default_path() else {
        eprintln!("· 找不到 locomo10.json ⇒ **没测到**");
        return Ok(());
    };
    let samples = locomo_corpus::load(&path).map_err(|e| -> Box<dyn Error> { e.into() })?;
    eprintln!(
        "· LoCoMo：{} 个样本 · {} 段 · {} 题",
        samples.len(),
        samples.iter().map(|s| s.turn_count()).sum::<usize>(),
        samples.iter().map(|s| s.cases.len()).sum::<usize>()
    );

    let names = ["top_cover", "hit_count", "margin", "top_score"];
    let mut bins: Vec<Vec<Bin>> = names.iter().map(|_| vec![Bin::default(); 4]).collect();
    let mut total = 0usize;

    for sample in &samples {
        let docs: Vec<MemoryCandidate> = sample
            .records
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
            .collect();
        if docs.is_empty() {
            continue;
        }
        let bm25 = Bm25LexicalCandidateSource::new(docs);

        // 联想边的替身：同一段记录内的实词共现（与 `golden_fair` **同口径**）
        let mut associations = apeireth_memory::golden_fair::CorpusAssociations::default();
        for record in &sample.records {
            let entities = apeireth_memory::golden_fair::association_entities(&record.text);
            for left in 0..entities.len() {
                for right in (left + 1)..entities.len() {
                    let _ = associations.record_cooccurrence(
                        &entities[left],
                        &entities[right],
                        &record.id,
                    );
                }
            }
        }
        let channel = apeireth_memory::golden_fair::CorpusAssociativeSource {
            associations: std::sync::Arc::new(associations),
            records: sample.records.clone(),
        };

        for case in &sample.cases {
            let hits = bm25.candidates(&case.query, K)?;
            let assoc = channel.candidates(&case.query, K)?;
            if hits.is_empty() {
                continue;
            }
            let values = [
                coverage(&case.query, &hits[0].content),
                hits.iter().filter(|h| h.score > 0.0).count() as f64,
                hits[0].score - hits.get(1).map(|h| h.score).unwrap_or(0.0),
                hits[0].score,
            ];
            let assoc_ids: Vec<&str> = assoc.iter().map(|c| c.id.as_str()).collect();
            let has = !case.expect_ids.is_empty()
                && case
                    .expect_ids
                    .iter()
                    .all(|id| assoc_ids.contains(&id.as_str()));
            for (index, value) in values.iter().enumerate() {
                // 按该信号的**四分位**分箱（用取值域切四段，不依赖全局分布）
                let bin = ((value * 4.0).floor() as usize).min(3);
                let b = &mut bins[index][bin];
                b.cases += 1;
                if has {
                    b.channel_has_target += 1;
                }
            }
            total += 1;
        }
    }

    eprintln!("· 有效题数：{total}");
    let mut payload = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let boxes: Vec<serde_json::Value> = bins[index]
            .iter()
            .enumerate()
            .map(|(i, b)| {
                serde_json::json!({
                    "箱": i,
                    "题数": b.cases,
                    "有答案率": if b.cases == 0 { 0.0 } else { b.channel_has_target as f64 / b.cases as f64 },
                })
            })
            .collect();
        let rates: Vec<f64> = bins[index]
            .iter()
            .filter(|b| b.cases > 0)
            .map(|b| b.channel_has_target as f64 / b.cases as f64)
            .collect();
        let spread = if rates.is_empty() {
            0.0
        } else {
            let max = rates.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min = rates.iter().cloned().fold(f64::INFINITY, f64::min);
            max - min
        };
        payload.push(serde_json::json!({
            "信号": name,
            "各箱": boxes,
            "分辨力(极差)": spread,
        }));
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({ "自适应信号探针": payload }))
            .unwrap_or_default()
    );
    Ok(())
}
