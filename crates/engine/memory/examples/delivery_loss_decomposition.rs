//! **送达漏送分解**（生产真实链路：召回 → 选段 → 注入 → 预算）。
//!
//! # 这个测例要回答的那一句
//!
//! 「检索到了却**没送到模型面前**」到底**丢在哪一段**？
//! 只看「送达率 = groundedness ÷ recall」这个**比值**看不出位置 ⇒ ⇒ 本测例把漏送拆成**四段**，
//! 每段给**数字**（不是猜测）：
//!
//! | 段 | 含义 | 判据 |
//! |---|---|---|
//! | **④ 召回没找到** | 答案压根没进召回集 | 金标 id ∉ top-k |
//! | **① 找到了但没进注入** | 召回到了，但**没被选中/被预算挤掉** | 金标 id ∉ 注入选中的 id |
//! | **② 进了但内容里没有答案** | 选中了，但**注入的那段文本**里没有期望片段 | 选中了 ∧ 注入文本不覆盖期望片段 |
//! | **③ 判据差异** | 模型**看到了**答案，但按「片段」判据没算 groundedness | 注入文本覆盖期望片段 ∧ 片段判据没算 |
//!
//! # 四条注入口径**并排**量（这是关键）
//!
//! | 口径 | 是什么 | 谁在用 |
//! |---|---|---|
//! | **XML** | 每条证据放**整条正文**（标签形态） | 库的**默认**形态（`memory_injection=false`） |
//! | **注入 + 锚定选段** | 编号证据 + **主体出现处**为中心，单条 **400 字** | **命令行生产**（注入格式**默认开**） |
//! | **闸门头截** | 每条证据取正文**前 120 字** | **基线闸门**（自称「与生产逐字一致」） |
//!
//! ⚠️ 这三条**不是同一件事** ⇒ ⇒ 若闸门量的与生产做的不同 ⇒ ⇒ **「送达率」这个数描述的不是生产**。
//!
//! # 口径
//!
//! * 语料 = `$APEIRETH_DATA_DIR/novels/`（默认 `~/.apeireth/novels/`）；真实查询卷 = `$APEIRETH_DATA_DIR/miracl-exam/`；
//! * 检索 = **生产管线**（`HybridRetrievalPipeline`，单路 BM25 源，`k = 10`）；
//! * 注入总预算 = **4,000 字**（生产 `MemoryRecallQuery::max_chars` 口径）；
//! * groundedness 判据 = `golden_set::covers_fragment`（与基线**逐字同口径**）。
//!
//! # 中性表述
//!
//! 只描述**机制**（召回 / 选段 / 注入 / 预算 / 送达率），不引用任何外部产品名、文档或代码。
//!
//! # 输出
//!
//! stdout 一行 JSON；stderr 人读分解表；语料缺失 ⇒ 明确说「跳过，不是跑过」并返回非零。

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

use apeireth_memory::anchor_injection::AnchoredInjectionConfig;
use apeireth_memory::golden_mining::MiningConfig;
use apeireth_memory::golden_run::mined_to_golden;
use apeireth_memory::golden_set::{covers_fragment, GoldenSet};
use apeireth_memory::layers::{MemoryLayerKind, MemoryRecallResult, RecalledMemoryItem};
use apeireth_memory::novel_corpus;
use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::{ClosedWorldContextCompiler, MemoryRankingConfig};
use serde_json::json;

/// 与基线一致的 top-k。
const K: usize = 10;
/// 与基线一致的挖题预算。
const MAX_CASES_PER_RECORD: usize = 6;
/// 生产注入**总**预算（`MemoryRecallQuery::max_chars` 口径）。
const MAX_CHARS: usize = 4_000;
/// **闸门口径**的单条上限（每条证据取正文前 N 字）。
const GATE_HEAD_CHARS: usize = 120;
/// 锚定选段的单条上限（**2026-11-08 时的生产值** 400；现生产值 = 200 ⇒ 本例按 400 复现当时的分解口径）。
const ANCHOR_SPAN_CHARS: usize = 400;
/// 扫的多锚点数（`1` = 与生产现行实现**逐字同**）。
const ANCHOR_VARIANTS: [usize; 4] = [1, 2, 3, 4];
/// 扫的单条跨度（**400 = 2026-11-08 时的生产值**；现生产值 = 200）。
const SPAN_VARIANTS: [usize; 7] = [100, 200, 300, 400, 600, 800, 1200];
const CORPORA: [(&str, &str); 2] = [
    ("redchamber", "redchamber.txt"),
    ("watershed", "watershed.txt"),
];

/// 一道题（**已召回到候选**）。
struct Prepared {
    query: String,
    /// 金标 id。
    gold: HashSet<String>,
    /// 期望片段（**判据**用）。
    fragments: Vec<String>,
    /// 候选（按生产管线的顺序，**放宽字符预算** ⇒ 排序口径不变）。
    candidates: Vec<(String, String)>,
    /// 候选（**生产字符预算**口径：管线按**原始正文**计费后真正入选的）。
    candidates_production: Vec<(String, String)>,
}

/// 一道题的**分段结果**。
#[derive(Default, Debug, Clone)]
struct Outcome {
    total: usize,
    /// ④ 召回没找到。
    not_retrieved: usize,
    /// ① 找到了但没进注入。
    retrieved_not_injected: usize,
    /// ② 进了注入但注入文本里没有期望片段。
    injected_without_answer: usize,
    /// ③ 模型看得见但按片段判据没算。
    seen_but_not_counted: usize,
    /// 召回命中。
    recalled: usize,
    /// 各口径的送达题数。
    grounded_xml: usize,
    grounded_anchored: usize,
    grounded_gate: usize,
    /// 「按选后长度计费」的提案臂送达题数。
    grounded_proposal: usize,
    /// 生产召回口径的送达题数 / 条目数。
    grounded_production_recall: usize,
    entries_production_recall: usize,
    /// 各口径的注入条目总数。
    entries_xml: usize,
    entries_anchored: usize,
    entries_proposal: usize,
    /// ② 的子因。
    subreason_paragraph_without_subject: usize,
    subreason_multiple_occurrences: usize,
    subreason_annotation: usize,
    subreason_other: usize,
    /// 单条跨度扫描：(跨度, 送达题数, 注入条目总数)。
    span_grounded: Vec<(usize, usize, usize)>,
    /// 多锚点扫描：(锚点数, 送达题数)。
    multi_grounded: Vec<(usize, usize)>,
}

fn main() -> ExitCode {
    let dir = novels_dir();
    let mut rows = Vec::new();
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
        match decompose_internal(name, &path) {
            Ok(row) => {
                loaded += 1;
                rows.push(row);
            }
            Err(error) => {
                eprintln!("· {name}：**跑失败**（{}）：{error}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }
    let real_query = match decompose_real_query() {
        Ok(row) => row,
        Err(error) => {
            eprintln!("· 真实查询卷：**跑失败**：{error}");
            return ExitCode::FAILURE;
        }
    };
    if loaded == 0 && real_query.is_none() {
        eprintln!(
            "一份卷都没跑到（目录 {}）⇒ **不能出数字**（不许把「没跑」当「没问题」）",
            dir.display()
        );
        return ExitCode::FAILURE;
    }
    println!(
        "{}",
        json!({
            "k": K,
            "max_chars": MAX_CHARS,
            "gate_head_chars": GATE_HEAD_CHARS,
            "production_span_chars": AnchoredInjectionConfig::PRODUCTION.span_chars,
            "corpora": rows,
            "real_query": real_query,
        })
    );
    ExitCode::SUCCESS
}

fn novels_dir() -> PathBuf {
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => PathBuf::from(root).join("novels"),
        None => std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".apeireth")
            .join("novels"),
    }
}

/// 内部卷（模板题）。
fn decompose_internal(
    name: &str,
    path: &std::path::Path,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let chapters = novel_corpus::load_chapters(path)?;
    let records = novel_corpus::to_mine_records(&chapters);
    let mined = apeireth_memory::golden_mining::mine(
        &records,
        MiningConfig {
            max_cases_per_record: MAX_CASES_PER_RECORD,
            ..MiningConfig::default()
        },
    );
    let golden = GoldenSet::new(format!("internal-{name}"), mined_to_golden(mined));

    let documents: Vec<apeireth_memory::MemoryCandidate> = records
        .iter()
        .map(|record| apeireth_memory::MemoryCandidate {
            id: record.id.clone(),
            layer: "golden".to_string(),
            scope: apeireth_memory::MemoryScope::Global,
            content: record.content.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: Default::default(),
        })
        .collect();
    let source = Bm25LexicalCandidateSource::new(documents);
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig::default());
    let compiler = ClosedWorldContextCompiler::new();

    let mut prepared: Vec<Prepared> = Vec::new();
    for case in &golden.cases {
        if case.expect_ids.is_empty() || case.expect_fragments.is_empty() {
            continue;
        }
        let retrieved = pipeline.retrieve(
            &case.query,
            &[apeireth_memory::MemoryScope::Global],
            &[&source],
            K,
            4_000_000,
        )?;
        // 生产口径：管线**按原始正文**给字符预算计费（`max_chars = 4000`）。
        let production_retrieved = pipeline.retrieve(
            &case.query,
            &[apeireth_memory::MemoryScope::Global],
            &[&source],
            K,
            MAX_CHARS,
        )?;
        prepared.push(Prepared {
            query: case.query.clone(),
            gold: case.expect_ids.iter().cloned().collect(),
            fragments: case.expect_fragments.clone(),
            candidates: retrieved
                .iter()
                .map(|item| (item.id.clone(), item.content.clone()))
                .collect(),
            candidates_production: production_retrieved
                .iter()
                .map(|item| (item.id.clone(), item.content.clone()))
                .collect(),
        });
    }
    // 记录正文（② 的子因诊断要用）。
    let body_by_id: std::collections::HashMap<String, String> = records
        .iter()
        .map(|record| (record.id.clone(), record.content.clone()))
        .collect();
    let outcome = analyze(&prepared, &compiler, &body_by_id);
    Ok(report(name, &outcome))
}

/// 真实查询卷（真人问句 + 官方标注）。
fn decompose_real_query() -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
    let root = match std::env::var_os("APEIRETH_MIRACL_EXAM") {
        Some(dir) => PathBuf::from(dir),
        None => std::env::var_os("APEIRETH_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("miracl-exam"),
    };
    let corpus_path = root.join("corpus.jsonl");
    let exam_path = root.join("exam.jsonl");
    if !corpus_path.exists() || !exam_path.exists() {
        eprintln!(
            "· 真实查询卷：考卷不在（{}）⇒ **跳过**（不是「跑过了」）",
            root.display()
        );
        return Ok(None);
    }
    let corpus_text = std::fs::read_to_string(&corpus_path)?;
    let exam_text = std::fs::read_to_string(&exam_path)?;
    let mut content_by_id: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut documents: Vec<apeireth_memory::MemoryCandidate> = Vec::new();
    for line in corpus_text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        let id = value["id"].as_str().unwrap_or_default().to_string();
        if id.is_empty() {
            continue;
        }
        let content = format!(
            "{}\n{}",
            value["title"].as_str().unwrap_or_default(),
            value["text"].as_str().unwrap_or_default()
        );
        content_by_id.insert(id.clone(), content.clone());
        documents.push(apeireth_memory::MemoryCandidate {
            id,
            layer: "real_query".to_string(),
            scope: apeireth_memory::MemoryScope::Global,
            content,
            score: 0.0,
            score_components: Default::default(),
            provenance: Default::default(),
        });
    }
    let source = Bm25LexicalCandidateSource::new(documents);
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig::default());
    let compiler = ClosedWorldContextCompiler::new();

    let mut prepared: Vec<Prepared> = Vec::new();
    for line in exam_text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)?;
        let query = value["query"].as_str().unwrap_or_default().to_string();
        let gold: Vec<String> = value["gold"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if query.is_empty() || gold.is_empty() {
            continue;
        }
        // 期望片段 = 金标文档的正文（与内部卷「期望片段是记录正文」的口径对齐）。
        let fragments: Vec<String> = gold
            .iter()
            .filter_map(|id| content_by_id.get(id).cloned())
            .collect();
        if fragments.is_empty() {
            continue;
        }
        let retrieved = pipeline.retrieve(
            &query,
            &[apeireth_memory::MemoryScope::Global],
            &[&source],
            K,
            4_000_000,
        )?;
        // 生产口径：管线**按原始正文**给字符预算计费（`max_chars = 4000`）。
        let production_retrieved = pipeline.retrieve(
            &query,
            &[apeireth_memory::MemoryScope::Global],
            &[&source],
            K,
            MAX_CHARS,
        )?;
        prepared.push(Prepared {
            query,
            gold: gold.into_iter().collect(),
            fragments,
            candidates: retrieved
                .iter()
                .map(|item| (item.id.clone(), item.content.clone()))
                .collect(),
            candidates_production: production_retrieved
                .iter()
                .map(|item| (item.id.clone(), item.content.clone()))
                .collect(),
        });
    }
    let outcome = analyze(&prepared, &compiler, &content_by_id);
    Ok(Some(report("real_query", &outcome)))
}

/// 把候选变成一条 `MemoryRecallResult`。
fn recall_result(candidates: &[(String, String)]) -> MemoryRecallResult {
    MemoryRecallResult {
        items: candidates
            .iter()
            .map(|(id, content)| RecalledMemoryItem {
                id: id.clone(),
                layer: MemoryLayerKind::Episodic,
                content: content.clone(),
                timestamp_ms: 0,
                score: 1.0,
                importance: 0.0,
                source_ref: None,
                score_components: None,
                diary_snippets: Vec::new(),
            })
            .collect(),
        total_candidates: candidates.len(),
        governance_filtered: 0,
        total_chars: 0,
        retrieval_status: None,
        wiki_sections: Vec::new(),
    }
}

/// 注入文本里**是否送达**了期望片段。
fn delivered(
    overlay: Option<&apeireth_memory::SelectedMemoryAccess>,
    fragments: &[String],
) -> bool {
    overlay.is_some_and(|selected| {
        fragments
            .iter()
            .any(|fragment| covers_fragment(&selected.overlay, fragment))
    })
}

/// 核心：多条口径并排跑 + 分解 + 两个扫描。
fn analyze(
    prepared: &[Prepared],
    compiler: &ClosedWorldContextCompiler,
    body_by_id: &std::collections::HashMap<String, String>,
) -> Outcome {
    let mut outcome = Outcome {
        span_grounded: SPAN_VARIANTS
            .iter()
            .map(|span| (*span, 0usize, 0usize))
            .collect(),
        multi_grounded: ANCHOR_VARIANTS
            .iter()
            .map(|anchors| (*anchors, 0usize))
            .collect(),
        ..Outcome::default()
    };
    for case in prepared {
        outcome.total += 1;
        let recalled = recall_result(&case.candidates);

        // ① XML 路径（库的默认形态）：每条放整条正文。
        let xml = compiler.compile_with_selected_access(&recalled, "probe", MAX_CHARS);
        // ② 注入 + 锚定选段（**命令行生产**口径）。
        let anchored = compiler.compile_injection_with_selected_access_and_query(
            &recalled,
            "probe",
            MAX_CHARS,
            &case.query,
            AnchoredInjectionConfig::PRODUCTION,
        );
        // ③ 闸门头截（**基线闸门**口径）。
        let gate = compiler.compile_injection_with_selected_access_budget(
            &recalled,
            "probe",
            MAX_CHARS,
            GATE_HEAD_CHARS,
        );
        // ③' 生产召回口径：管线按**原始正文**计费后真正入选的候选，走**同一条**锚定路径。
        let production_recalled = recall_result(&case.candidates_production);
        let production_arm = compiler.compile_injection_with_selected_access_and_query(
            &production_recalled,
            "probe",
            MAX_CHARS,
            &case.query,
            AnchoredInjectionConfig::PRODUCTION,
        );
        if delivered(production_arm.as_ref(), &case.fragments) {
            outcome.grounded_production_recall += 1;
        }
        if let Some(selected) = production_arm.as_ref() {
            outcome.entries_production_recall += selected.selected_candidate_ids.len();
        }

        // ④ 提案臂：候选预算按「**选后长度**」计费（见证据文档）。
        let admitted = admit_by_delivered_size(&case.candidates);
        let proposal_candidates: Vec<(String, String)> = case
            .candidates
            .iter()
            .filter(|(id, _)| admitted.contains(id))
            .cloned()
            .collect();
        let proposal = {
            let mut accessed = recall_result(&proposal_candidates);
            let subject = apeireth_memory::anchor_injection::subject_of_query(&case.query);
            for item in accessed.items.iter_mut() {
                let Some(subject) = subject.as_deref() else {
                    continue;
                };
                if let Some(text) = apeireth_memory::anchor_injection::find_subject_window(
                    &item.content,
                    subject,
                    ANCHOR_SPAN_CHARS,
                ) {
                    if !text.trim().is_empty() {
                        item.content = text;
                    }
                }
            }
            compiler.compile_injection_with_selected_access_budget(
                &accessed,
                "probe",
                MAX_CHARS,
                ANCHOR_SPAN_CHARS,
            )
        };

        let seen_xml = delivered(xml.as_ref(), &case.fragments);
        let seen_anchored = delivered(anchored.as_ref(), &case.fragments);
        let seen_gate = delivered(gate.as_ref(), &case.fragments);
        let seen_proposal = delivered(proposal.as_ref(), &case.fragments);
        if seen_xml {
            outcome.grounded_xml += 1;
        }
        if seen_anchored {
            outcome.grounded_anchored += 1;
        }
        if seen_gate {
            outcome.grounded_gate += 1;
        }
        if seen_proposal {
            outcome.grounded_proposal += 1;
        }
        if let Some(selected) = xml.as_ref() {
            outcome.entries_xml += selected.selected_candidate_ids.len();
        }
        if let Some(selected) = anchored.as_ref() {
            outcome.entries_anchored += selected.selected_candidate_ids.len();
        }
        if let Some(selected) = proposal.as_ref() {
            outcome.entries_proposal += selected.selected_candidate_ids.len();
        }

        let hit = case
            .candidates
            .iter()
            .take(K)
            .any(|(id, _)| case.gold.contains(id));
        if hit {
            outcome.recalled += 1;
        }
        let gold_selected = anchored
            .as_ref()
            .map(|selected| {
                selected
                    .selected_candidate_ids
                    .iter()
                    .any(|id| case.gold.contains(id))
            })
            .unwrap_or(false);
        // ③ 判据差异：模型看得见，但按「逐条片段」判据没算。
        if seen_anchored {
            let counted = anchored
                .as_ref()
                .map(|selected| {
                    selected.overlay.lines().any(|line| {
                        let text = line
                            .trim_start()
                            .trim_start_matches(|c: char| c.is_ascii_digit())
                            .trim_start_matches('.')
                            .trim();
                        case.fragments
                            .iter()
                            .any(|fragment| covers_fragment(text, fragment))
                    })
                })
                .unwrap_or(false);
            if !counted {
                outcome.seen_but_not_counted += 1;
            }
        }

        // ② 的**子因**：为什么「选中了但注入文本里没有答案」？
        if hit && gold_selected && !seen_anchored {
            let subject = apeireth_memory::anchor_injection::subject_of_query(&case.query);
            let gold_body = case
                .gold
                .iter()
                .find_map(|id| body_by_id.get(id))
                .cloned()
                .unwrap_or_default();
            let mut answer_para_missing = false;
            let mut answer_para_has_subject = false;
            for fragment in &case.fragments {
                let Some(paragraph) = gold_body.split('\n').find(|line| line.contains(fragment))
                else {
                    answer_para_missing = true;
                    continue;
                };
                if let Some(subject) = subject.as_deref() {
                    if paragraph.contains(subject) {
                        answer_para_has_subject = true;
                    }
                }
            }
            let occurrences = subject
                .as_deref()
                .map(|subject| gold_body.matches(subject).count())
                .unwrap_or(0);
            if answer_para_missing {
                outcome.subreason_annotation += 1;
            } else if !answer_para_has_subject {
                outcome.subreason_paragraph_without_subject += 1;
            } else if occurrences > 1 {
                outcome.subreason_multiple_occurrences += 1;
            } else {
                outcome.subreason_other += 1;
            }
        }

        // 四段归类（互斥）。
        if !hit {
            outcome.not_retrieved += 1;
        } else if !gold_selected {
            outcome.retrieved_not_injected += 1;
        } else if !seen_anchored {
            outcome.injected_without_answer += 1;
        }

        // 单条跨度扫描（注入 + 锚定路径，总预算不变）。
        for span in SPAN_VARIANTS {
            let mut accessed = recall_result(&case.candidates);
            let subject = apeireth_memory::anchor_injection::subject_of_query(&case.query);
            for item in accessed.items.iter_mut() {
                let Some(subject) = subject.as_deref() else {
                    continue;
                };
                if let Some(text) = apeireth_memory::anchor_injection::find_subject_window(
                    &item.content,
                    subject,
                    span,
                ) {
                    if !text.trim().is_empty() {
                        item.content = text;
                    }
                }
            }
            let compiled = compiler
                .compile_injection_with_selected_access_budget(&accessed, "probe", MAX_CHARS, span);
            let seen = delivered(compiled.as_ref(), &case.fragments);
            let entries = compiled
                .as_ref()
                .map(|selected| selected.selected_candidate_ids.len())
                .unwrap_or(0);
            if let Some(slot) = outcome
                .span_grounded
                .iter_mut()
                .find(|(s, _, _)| *s == span)
            {
                slot.1 += usize::from(seen);
                slot.2 += entries;
            }
        }

        // 多锚点扫描（**阴性结果**：分预算给多处并不救「主体出现多次」）。
        for anchors in ANCHOR_VARIANTS {
            let mut accessed = recall_result(&case.candidates);
            let subject = apeireth_memory::anchor_injection::subject_of_query(&case.query);
            for item in accessed.items.iter_mut() {
                let Some(subject) = subject.as_deref() else {
                    continue;
                };
                if let Some(text) =
                    multi_anchor_window(&item.content, subject, ANCHOR_SPAN_CHARS, anchors)
                {
                    if !text.trim().is_empty() {
                        item.content = text;
                    }
                }
            }
            let compiled = compiler.compile_injection_with_selected_access_budget(
                &accessed,
                "probe",
                MAX_CHARS,
                ANCHOR_SPAN_CHARS,
            );
            let seen = delivered(compiled.as_ref(), &case.fragments);
            if let Some(slot) = outcome
                .multi_grounded
                .iter_mut()
                .find(|(a, _)| *a == anchors)
            {
                slot.1 += usize::from(seen);
            }
        }
    }
    outcome
}

/// 提案：候选预算按「**选后长度**」（单条上限）计费，而不是按**原始正文**计费。
fn admit_by_delivered_size(candidates: &[(String, String)]) -> HashSet<String> {
    let mut admitted = HashSet::new();
    let mut chars = 0usize;
    for (id, content) in candidates {
        let cost = content.chars().count().min(ANCHOR_SPAN_CHARS);
        if chars + cost > MAX_CHARS {
            continue;
        }
        chars += cost;
        admitted.insert(id.clone());
    }
    admitted
}

/// **多锚点窗口**（**阴性结果**的实验实现，**只在测例里**）。
///
/// ⚠️ 为什么放在测例里而不是库里：实测（内部卷 476/248 题 + 真实查询卷 393 问）
/// **送达数一条都没涨**（281 → 281 → 281 → 280）⇒ ⇒ 它**不是改进** ⇒ ⇒ 不进库。
fn multi_anchor_window(
    text: &str,
    subject: &str,
    max_chars: usize,
    anchors: usize,
) -> Option<String> {
    if text.is_empty() || max_chars == 0 || anchors <= 1 || subject.is_empty() {
        return apeireth_memory::anchor_injection::find_subject_window(text, subject, max_chars);
    }
    let mut positions: Vec<usize> = Vec::new();
    let mut cursor = 0usize;
    while let Some(found) = text[cursor..].find(subject) {
        let byte_offset = cursor + found;
        positions.push(text[..byte_offset].chars().count());
        cursor = byte_offset + subject.len();
    }
    if positions.len() <= 1 {
        return apeireth_memory::anchor_injection::find_subject_window(text, subject, max_chars);
    }
    let separator = "\n…\n";
    let separator_total = separator.chars().count() * (anchors - 1);
    let per_anchor = max_chars.saturating_sub(separator_total) / anchors;
    if per_anchor == 0 {
        return apeireth_memory::anchor_injection::find_subject_window(text, subject, max_chars);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut windows: Vec<String> = Vec::new();
    for position in positions.iter().take(anchors) {
        let start = position.saturating_sub(per_anchor / 2);
        let end = (start + per_anchor).min(chars.len());
        windows.push(chars[start..end].iter().collect());
    }
    Some(windows.join(separator))
}

/// 人读 + 机器读的报告。
fn report(name: &str, outcome: &Outcome) -> serde_json::Value {
    let total = outcome.total.max(1) as f64;
    let recall = outcome.recalled as f64 / total;
    let xml = outcome.grounded_xml as f64 / total;
    let anchored = outcome.grounded_anchored as f64 / total;
    let gate = outcome.grounded_gate as f64 / total;
    let proposal = outcome.grounded_proposal as f64 / total;
    let rate = |value: f64| if recall > 0.0 { value / recall } else { 0.0 };

    eprintln!("════════ {name} · 送达漏送分解 ════════");
    eprintln!(
        "· 题 {} · 召回命中 {}（recall@10 = {:.4}）",
        outcome.total, outcome.recalled, recall
    );
    eprintln!(
        "· ④ 召回没找到：{}（{:.1}%）· ① 找到了没进注入：{}（{:.1}%）· ② 进了但没答案：{}（{:.1}%）",
        outcome.not_retrieved,
        outcome.not_retrieved as f64 / total * 100.0,
        outcome.retrieved_not_injected,
        outcome.retrieved_not_injected as f64 / total * 100.0,
        outcome.injected_without_answer,
        outcome.injected_without_answer as f64 / total * 100.0
    );
    eprintln!(
        "· ② 子因：段落不含主体 {} · 主体出现多次 {} · 标注/语料问题 {} · 其它 {}",
        outcome.subreason_paragraph_without_subject,
        outcome.subreason_multiple_occurrences,
        outcome.subreason_annotation,
        outcome.subreason_other
    );
    eprintln!(
        "· ③ 判据差异（模型看得见但片段判据没算）：{}（{:.1}%）",
        outcome.seen_but_not_counted,
        outcome.seen_but_not_counted as f64 / total * 100.0
    );
    eprintln!(
        "· 送达率：**注入+锚定（命令行生产）** = **{:.4}** · XML（库默认）= {:.4} · 闸门头截（基线口径）= {:.4}",
        rate(anchored),
        rate(xml),
        rate(gate)
    );
    eprintln!(
        "· **生产召回口径**（管线按原始正文计费）：平均入选 {:.2} 条 · 送达率 **{:.4}**",
        outcome.entries_production_recall as f64 / total,
        rate(outcome.grounded_production_recall as f64 / total)
    );
    eprintln!(
        "· 平均注入条目：注入+锚定 {:.2} · XML {:.2} · 提案（按选后长度计费）{:.2}",
        outcome.entries_anchored as f64 / total,
        outcome.entries_xml as f64 / total,
        outcome.entries_proposal as f64 / total
    );
    eprintln!(
        "· 提案臂（候选预算按**选后长度**计费）送达率 = **{:.4}**（Δ = {:+.4}）",
        rate(proposal),
        rate(proposal) - rate(anchored)
    );
    eprintln!("· 单条跨度扫描（总预算 4000 不变）：");
    for (span, count, entries) in &outcome.span_grounded {
        eprintln!(
            "    单条 {} 字：送达 {} 题（{:.4}）· 送达率 {:.4} · 平均注入条目 {:.2}",
            span,
            count,
            *count as f64 / total,
            rate(*count as f64 / total),
            *entries as f64 / total
        );
    }
    eprintln!("· 多锚点窗口扫描（**阴性结果**）：");
    for (anchors, count) in &outcome.multi_grounded {
        eprintln!(
            "    锚点 {} 处：送达 {} 题（{:.4}）· 送达率 {:.4}",
            anchors,
            count,
            *count as f64 / total,
            rate(*count as f64 / total)
        );
    }

    json!({
        "name": name,
        "cases": outcome.total,
        "recall_at_k": recall,
        "recalled_cases": outcome.recalled,
        "segment_4_not_retrieved": outcome.not_retrieved,
        "segment_1_retrieved_not_injected": outcome.retrieved_not_injected,
        "segment_2_injected_without_answer": outcome.injected_without_answer,
        "segment_3_seen_but_not_counted": outcome.seen_but_not_counted,
        "segment_2_subreason_paragraph_without_subject": outcome.subreason_paragraph_without_subject,
        "segment_2_subreason_multiple_subject_occurrences": outcome.subreason_multiple_occurrences,
        "segment_2_subreason_annotation_or_corpus": outcome.subreason_annotation,
        "segment_2_subreason_other": outcome.subreason_other,
        "groundedness_anchored_injection": anchored,
        "groundedness_xml": xml,
        "groundedness_gate": gate,
        "groundedness_proposal": proposal,
        "groundedness_production_recall": outcome.grounded_production_recall as f64 / total,
        "delivery_rate_production_recall": rate(outcome.grounded_production_recall as f64 / total),
        "avg_entries_production_recall": outcome.entries_production_recall as f64 / total,
        "delivery_rate_anchored_injection": rate(anchored),
        "delivery_rate_xml": rate(xml),
        "delivery_rate_gate": rate(gate),
        "delivery_rate_proposal": rate(proposal),
        "avg_entries_anchored_injection": outcome.entries_anchored as f64 / total,
        "avg_entries_xml": outcome.entries_xml as f64 / total,
        "avg_entries_proposal": outcome.entries_proposal as f64 / total,
        "span_variants": outcome
            .span_grounded
            .iter()
            .map(|(span, count, entries)| {
                json!({
                    "span_chars": span,
                    "grounded_cases": count,
                    "delivery_rate": rate(*count as f64 / total),
                    "avg_selected_entries": *entries as f64 / total,
                })
            })
            .collect::<Vec<_>>(),
        "multi_anchor_grounded": outcome
            .multi_grounded
            .iter()
            .map(|(anchors, count)| {
                json!({
                    "anchors": anchors,
                    "grounded_cases": count,
                    "delivery_rate": rate(*count as f64 / total),
                })
            })
            .collect::<Vec<_>>(),
    })
}
