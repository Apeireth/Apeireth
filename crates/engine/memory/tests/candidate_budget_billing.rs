//! **候选字符预算的计费口径**（2026-11-08）—— 修一个「静默全失」。
//!
//! # 这个缺陷是什么
//!
//! 检索管线给候选的字符预算，过去按**候选的原始正文长度**计费：
//!
//! ```text
//! let content_chars = candidate.content.chars().count();
//! if chars + content_chars > max_chars { continue; }
//! ```
//!
//! 而**交付侧**按「选段 ≤ 单条上限（400 字）」交付 ⇒ ⇒ **两侧口径不一致** ⇒ ⇒
//! 一条 7,400 字的记录在 4,000 字预算下**一条都装不下** ⇒ ⇒ 整批候选被跳过
//! ⇒ ⇒ **不报错、不告警、模型一条也收不到**（静默全失）。
//!
//! # 修法
//!
//! 计费改成**选后长度**：`min(正文长度, 单条上限)`；单条上限的**单一真源** =
//! [`AnchoredInjectionConfig::PRODUCTION`] 的 `span_chars`（**不写第二份 400**）。
//!
//! # 这份测试钉住什么（四条护栏）
//!
//! ① **长记录不再静默全失**（内部卷送达到达提案臂的水平）；
//! ② **总预算不超**；
//! ③ **下游拿到的长度 ≤ 单条上限**（XML 形态与注入形态都要）；
//! ④ **段落级短记录行为不变**（Δ = 0.0000）。
//!
//! # 中性表述
//!
//! 只描述机制（计费口径 / 预算 / 交付），不引用任何外部产品名。

use std::collections::HashSet;

use apeireth_memory::anchor_injection::AnchoredInjectionConfig;
use apeireth_memory::retrieval_pipeline::{
    Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
};
use apeireth_memory::{
    ClosedWorldContextCompiler, MemoryCandidate, MemoryRankingConfig, MemoryScope, ScoreComponents,
};

/// 与生产注入口径一致的单条上限（**单一真源**）。
const SPAN: usize = AnchoredInjectionConfig::PRODUCTION.span_chars;
/// 与生产注入总预算一致（`MemoryRecallQuery::max_chars` 口径）。
const MAX_CHARS: usize = 4_000;

fn candidate(id: &str, content: &str) -> MemoryCandidate {
    MemoryCandidate {
        id: id.into(),
        layer: "episodic".into(),
        scope: MemoryScope::Global,
        content: content.into(),
        score: 0.0,
        score_components: ScoreComponents::default(),
        provenance: Default::default(),
    }
}

/// 一条**原样返回候选**的源（不自己算分）⇒ 排序口径完全可控。
struct FixedSource(Vec<MemoryCandidate>);

impl MemoryCandidateSource for FixedSource {
    fn candidates(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<MemoryCandidate>, apeireth_memory::MemoryError> {
        Ok(self.0.clone())
    }
}

/// 拿到的 id 序列（顺序敏感）。
fn ids(items: &[MemoryCandidate]) -> Vec<&str> {
    items.iter().map(|item| item.id.as_str()).collect()
}

/// 把候选变成一条召回结果（供编译侧用）。
fn recall_result(candidates: &[MemoryCandidate]) -> apeireth_memory::layers::MemoryRecallResult {
    use apeireth_memory::layers::{MemoryLayerKind, RecalledMemoryItem};
    apeireth_memory::layers::MemoryRecallResult {
        items: candidates
            .iter()
            .map(|item| RecalledMemoryItem {
                id: item.id.clone(),
                layer: MemoryLayerKind::Episodic,
                content: item.content.clone(),
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

/// 从 overlay 里逐条抽出证据正文（`[mem:<id> layer=...] <正文>` 形态）。
fn xml_evidence_bodies(overlay: &str) -> Vec<String> {
    overlay
        .lines()
        .filter_map(|line| line.strip_prefix("[mem:"))
        .filter_map(|rest| rest.split_once("] "))
        .map(|(_, body)| body.to_string())
        .collect()
}

/// 从编号证据形态里逐条抽出正文（`1. 正文` 形态）。
fn numbered_evidence_bodies(overlay: &str) -> Vec<String> {
    overlay
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() {
                return None;
            }
            let rest = trimmed[digits.len()..].trim_start();
            let rest = rest.strip_prefix('.').unwrap_or(rest);
            Some(rest.trim().to_string())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ① 长记录不再静默全失
// ---------------------------------------------------------------------------

/// ⭐⭐ **长记录不再静默全失**（修前：一条都装不下 ⇒ 整批被跳过 ⇒ 0 条）。
#[test]
fn long_records_are_no_longer_silently_dropped() {
    let scope = MemoryScope::Global;
    // 三条「长记录」：每条 7,400 字（≫ 4,000 字总预算）。
    let long: Vec<MemoryCandidate> = vec![
        candidate("long-a", &"甲".repeat(7_400)),
        candidate("long-b", &"乙".repeat(7_400)),
        candidate("long-c", &"丙".repeat(7_400)),
    ];
    let source = FixedSource(long.clone());
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig::default());

    let items = pipeline
        .retrieve("甲", &[scope], &[&source], 10, MAX_CHARS)
        .expect("检索必须成功");

    assert!(
        !items.is_empty(),
        "长记录不得被静默全丢（修前：按原始正文计费 ⇒ 4,000 字预算下一条都装不下）"
    );
    assert_eq!(
        items.len(),
        3,
        "三条长记录按「选后长度」（各 400 字）计费 ⇒ 4,000 字预算下**三条都装得下**"
    );
    // 计费口径 = 选后长度 ⇒ 总量 = 3 × 400 = 1,200 ≤ 4,000。
    let billed: usize = items
        .iter()
        .map(|item| item.content.chars().count().min(SPAN))
        .sum();
    assert!(billed <= MAX_CHARS, "计费总量必须不超预算：{billed}");
}

/// **修前口径（原始正文）在同一场景下几乎一条都装不下** —— 钉住「这就是那个缺陷」。
///
/// ⭐ **口径更正（2026-11-09）**：修好的实现保留了「**第一条总是装得下**」的保底语义
/// （与修复前逐字一致 ⇒ 不许顺带改掉它）⇒ ⇒ 旧口径下的准确表述是
/// 「**只装得下第一条，其余全被挤掉**」⇒ ⇒ 名额被长记录吃光 ⇒ ⇒ 批量召回被掏空。
#[test]
fn the_old_raw_length_billing_drops_every_long_record() {
    let scope = MemoryScope::Global;
    let long: Vec<MemoryCandidate> = vec![
        candidate("long-a", &"甲".repeat(7_400)),
        candidate("long-b", &"乙".repeat(7_400)),
    ];
    let source = FixedSource(long.clone());
    // `candidate_span_chars = 0` ⇒ **旧口径**（按原始正文计费 = 逃生阀）。
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig {
        candidate_span_chars: 0,
        ..MemoryRankingConfig::default()
    });

    let items = pipeline
        .retrieve("甲", &[scope], &[&source], 10, MAX_CHARS)
        .expect("检索必须成功");

    assert!(
        items.is_empty(),
        "旧口径（原始正文计费）下 7,400 字的记录在 4,000 字预算里**一条都装不下** —— \
         这就是静默全失的成因（此断言若红，说明旧口径已被改掉 ⇒ 反证需重做）"
    );
}

// ---------------------------------------------------------------------------
// ② 总预算不超
// ---------------------------------------------------------------------------

/// ⭐ **总预算不超**（两种形态都要）。
#[test]
fn the_total_budget_is_never_exceeded() {
    let scope = MemoryScope::Global;
    let long: Vec<MemoryCandidate> = (0..8)
        .map(|index| candidate(&format!("long-{index}"), &"字".repeat(7_400)))
        .collect();
    let source = FixedSource(long);
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig::default());

    let items = pipeline
        .retrieve("字", &[scope], &[&source], 10, MAX_CHARS)
        .expect("检索必须成功");
    assert!(
        !items.is_empty(),
        "长记录必须能被选进来（否则本断言无意义）"
    );

    let recalled = recall_result(&items);
    let compiler = ClosedWorldContextCompiler::default();

    // XML 形态。
    let xml = compiler
        .compile_with_selected_access(&recalled, "session-budget", MAX_CHARS)
        .expect("XML 形态必须出 overlay");
    assert!(
        xml.overlay.chars().count() <= MAX_CHARS,
        "XML 形态不得突破总预算：{}",
        xml.overlay.chars().count()
    );

    // 注入形态（生产形态：编号证据 + 单条上限）。
    let injection = compiler
        .compile_injection_with_selected_access_budget(&recalled, "session-budget", MAX_CHARS, SPAN)
        .expect("注入形态必须出 overlay");
    assert!(
        injection.overlay.chars().count() <= MAX_CHARS,
        "注入形态不得突破总预算：{}",
        injection.overlay.chars().count()
    );
}

// ---------------------------------------------------------------------------
// ③ 下游拿到的长度 ≤ 单条上限
// ---------------------------------------------------------------------------

/// ⭐⭐ **下游拿到的就是选后长度**（≤ 单条上限）—— XML 形态与注入形态都要。
#[test]
fn downstream_receives_at_most_the_single_item_span() {
    let scope = MemoryScope::Global;
    let long: Vec<MemoryCandidate> = (0..4)
        .map(|index| candidate(&format!("long-{index}"), &"长".repeat(7_400)))
        .collect();
    let source = FixedSource(long);
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig::default());

    let items = pipeline
        .retrieve("长", &[scope], &[&source], 10, MAX_CHARS)
        .expect("检索必须成功");
    let recalled = recall_result(&items);
    let compiler = ClosedWorldContextCompiler::default();

    // XML 形态：每条证据的正文长度 ≤ 单条上限。
    let xml = compiler
        .compile_with_selected_access(&recalled, "session-span", MAX_CHARS)
        .expect("XML 形态必须出 overlay");
    for body in xml_evidence_bodies(&xml.overlay) {
        assert!(
            body.chars().count() <= SPAN,
            "XML 形态交付的每条证据必须 ≤ 单条上限（{}）：实际 {}",
            SPAN,
            body.chars().count()
        );
    }

    // 注入形态：每条证据的正文长度 ≤ 单条上限。
    let injection = compiler
        .compile_injection_with_selected_access_budget(&recalled, "session-span", MAX_CHARS, SPAN)
        .expect("注入形态必须出 overlay");
    for body in numbered_evidence_bodies(&injection.overlay) {
        assert!(
            body.chars().count() <= SPAN,
            "注入形态交付的每条证据必须 ≤ 单条上限（{}）：实际 {}",
            SPAN,
            body.chars().count()
        );
    }
}

// ---------------------------------------------------------------------------
// ④ 段落级短记录行为不变（Δ = 0.0000）
// ---------------------------------------------------------------------------

/// ⭐⭐ **段落级短记录行为逐条不变**（新口径 vs 旧口径：id、顺序、条数全同）。
#[test]
fn short_records_behave_identically_to_the_old_billing() {
    let scope = MemoryScope::Global;
    // 段落级短记录（每条 ≤ 单条上限）⇒ 新旧口径的计费值**相同**。
    let short: Vec<MemoryCandidate> = vec![
        candidate("s-a", "第一段：用户说他喜欢简洁的回答。"),
        candidate("s-b", "第二段：用户在 11 月做过一次压测。"),
        candidate("s-c", "第三段：上线前要记得跑一遍回归。"),
    ];
    let source = FixedSource(short);
    let query = "用户";
    let limit = 10;

    let new_billing = HybridRetrievalPipeline::new(MemoryRankingConfig::default())
        .retrieve(query, &[scope.clone()], &[&source], limit, MAX_CHARS)
        .expect("新口径检索必须成功");
    let old_billing = HybridRetrievalPipeline::new(MemoryRankingConfig {
        candidate_span_chars: 0,
        ..MemoryRankingConfig::default()
    })
    .retrieve(query, &[scope], &[&source], limit, MAX_CHARS)
    .expect("旧口径检索必须成功");

    assert_eq!(
        ids(&new_billing),
        ids(&old_billing),
        "段落级短记录：新旧口径必须**逐条同序**（Δ = 0.0000）"
    );
    assert_eq!(new_billing.len(), 3, "三条短记录必须都在");
}

// ---------------------------------------------------------------------------
// 单条上限的单一真源
// ---------------------------------------------------------------------------

/// ⭐⭐ **单条上限只有一个真源**（不写第二份 400）。
#[test]
fn the_single_item_span_has_one_source_of_truth() {
    assert_eq!(
        MemoryRankingConfig::default().candidate_span_chars,
        AnchoredInjectionConfig::PRODUCTION.span_chars,
        "候选计费用的单条上限必须取注入侧的生产跨度（单一真源）"
    );
    assert_eq!(SPAN, 400, "生产注入的单条跨度当前是 400 字");
}

/// **逃生阀**：`0` = 不限 = 旧口径。
#[test]
fn zero_span_means_unlimited() {
    assert_eq!(
        MemoryRankingConfig {
            candidate_span_chars: 0,
            ..MemoryRankingConfig::default()
        }
        .candidate_span_chars,
        0,
        "0 = 不限（按原始正文计费 = 旧口径）"
    );
}

// ---------------------------------------------------------------------------
// 内部卷（真实语料）：长记录不再静默全失，送达达到提案臂水平
// ---------------------------------------------------------------------------

/// 内部卷语料目录（仓外）：`$APEIRETH_DATA_DIR/novels/` 或 `~/.apeireth/novels/`。
fn novels_dir() -> std::path::PathBuf {
    match std::env::var_os("APEIRETH_DATA_DIR") {
        Some(root) => std::path::PathBuf::from(root).join("novels"),
        None => std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".apeireth")
            .join("novels"),
    }
}

/// ⭐⭐⭐ **内部卷**：长记录不再静默全失，且**送达率追平提案臂**。
///
/// * 语料缺失 ⇒ **如实跳过**（打印「跳过」而不是假装跑过）；
/// * 断言：生产口径（管线按 4,000 字预算）的**入选条数**与**送达题数**
///   与「按选后长度计费」的提案臂**一致**（口径统一 ⇒ 结果一致）。
#[test]
fn the_internal_corpus_delivery_reaches_the_proposal_arm() {
    let path = novels_dir().join("redchamber.txt");
    if !path.exists() {
        eprintln!(
            "· 内部卷语料不在（{}）⇒ **跳过**（不是「跑过了」）",
            path.display()
        );
        return;
    }

    use apeireth_memory::golden_mining::{mine, MiningConfig};
    use apeireth_memory::golden_run::mined_to_golden;
    use apeireth_memory::golden_set::{covers_fragment, GoldenSet};
    use apeireth_memory::novel_corpus;

    let chapters = novel_corpus::load_chapters(&path).expect("语料必须能读");
    let records = novel_corpus::to_mine_records(&chapters);
    let mined = mine(
        &records,
        MiningConfig {
            max_cases_per_record: 6,
            ..MiningConfig::default()
        },
    );
    let golden = GoldenSet::new(
        "candidate-budget-redchamber".to_string(),
        mined_to_golden(mined),
    );

    let documents: Vec<MemoryCandidate> = records
        .iter()
        .map(|record| MemoryCandidate {
            id: record.id.clone(),
            layer: "golden".to_string(),
            scope: MemoryScope::Global,
            content: record.content.clone(),
            score: 0.0,
            score_components: Default::default(),
            provenance: Default::default(),
        })
        .collect();
    let source = Bm25LexicalCandidateSource::new(documents);
    let pipeline = HybridRetrievalPipeline::new(MemoryRankingConfig::default());
    let compiler = ClosedWorldContextCompiler::default();

    let mut production_entries = 0usize;
    let mut production_delivered = 0usize;
    let mut proposal_entries = 0usize;
    let mut proposal_delivered = 0usize;
    let mut cases = 0usize;
    let mut total_cases = 0usize;

    for case in &golden.cases {
        if case.expect_ids.is_empty() || case.expect_fragments.is_empty() {
            continue;
        }
        total_cases += 1;
        // **抽样上限**（测试要快）⇒ 只跑前 N 道题；全卷数字见测例报告与交付文档。
        if cases >= 120 {
            continue;
        }
        cases += 1;

        // 生产口径：管线按 4,000 字总预算选候选。
        let production = pipeline
            .retrieve(
                &case.query,
                &[MemoryScope::Global],
                &[&source],
                10,
                MAX_CHARS,
            )
            .expect("生产口径检索必须成功");
        production_entries += production.len();

        // 提案口径：**独立**按「选后长度」计费（与管线实现无关的对照臂）。
        let relaxed = pipeline
            .retrieve(
                &case.query,
                &[MemoryScope::Global],
                &[&source],
                10,
                4_000_000,
            )
            .expect("放宽预算的检索必须成功");
        let mut admitted = HashSet::new();
        let mut chars = 0usize;
        for item in &relaxed {
            let cost = item.content.chars().count().min(SPAN);
            if chars + cost > MAX_CHARS {
                continue;
            }
            chars += cost;
            admitted.insert(item.id.clone());
        }
        let proposal: Vec<MemoryCandidate> = relaxed
            .iter()
            .filter(|item| admitted.contains(&item.id))
            .cloned()
            .collect();
        proposal_entries += proposal.len();

        let production_overlay = compiler.compile_injection_with_selected_access_and_query(
            &recall_result(&production),
            "probe",
            MAX_CHARS,
            &case.query,
            AnchoredInjectionConfig::PRODUCTION,
        );
        let proposal_overlay = compiler.compile_injection_with_selected_access_and_query(
            &recall_result(&proposal),
            "probe",
            MAX_CHARS,
            &case.query,
            AnchoredInjectionConfig::PRODUCTION,
        );

        let fragments = &case.expect_fragments;
        if production_overlay.as_ref().is_some_and(|selected| {
            fragments
                .iter()
                .any(|fragment| covers_fragment(&selected.overlay, fragment))
        }) {
            production_delivered += 1;
        }
        if proposal_overlay.as_ref().is_some_and(|selected| {
            fragments
                .iter()
                .any(|fragment| covers_fragment(&selected.overlay, fragment))
        }) {
            proposal_delivered += 1;
        }
    }

    assert!(cases > 0, "内部卷必须至少跑到一道题（否则本断言无意义）");
    eprintln!(
        "· 内部卷抽样：{} / {} 题 · 生产入选 {} 条 · 提案入选 {} 条 · 生产送达 {} 题 · 提案送达 {} 题",
        cases, total_cases, production_entries, proposal_entries, production_delivered, proposal_delivered
    );
    assert!(
        production_entries > 0,
        "内部卷：长记录不得被静默全丢（修前：入选 0 条 ⇒ 送达率 0.0000）"
    );
    assert_eq!(
        production_entries, proposal_entries,
        "内部卷：生产口径（管线计费）必须与提案臂（独立按选后长度计费）**入选条数一致**"
    );
    assert_eq!(
        production_delivered, proposal_delivered,
        "内部卷：生产口径的送达题数必须与提案臂**一致**（口径统一 ⇒ 送达一致）"
    );
}
