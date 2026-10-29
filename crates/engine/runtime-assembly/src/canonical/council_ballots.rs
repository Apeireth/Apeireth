//! D-2：council 顾问评价 ⇒ rubric 立场票（守夜人「立场平衡复盘」的输入）。
//!
//! `orchestration::AdvisorEvaluation { advisor, kind, verdict }` 与
//! `governance::rubric::Ballot` 形状对不上 ⇒ 守夜人的「立场平衡复盘」段永远空转
//! （见 `canonical/nightwatch.rs` ⑤：`council_ballots` 空 = 具名缺口）。
//! 本模块是唯一的折算点：一条顾问评价 ⇒ 一张票 + 一条可落盘的记录。
//!
//! owner 裁定的口径（**不编造**顾问没给的信息）：
//!
//! * `stance`：`Allow → Approve`、`Stop → Disapprove`、`Retry →` 既非赞成也非反对、
//!   `Abstain → Abstain`。
//! * `confidence`：`verdict.confidence` 是 `Some(c)` ⇒ 用 `c`（顾问**显式**置信度），
//!   `confidence_defaulted = false`；是 `None` ⇒ 用 `verdict.score`（顾问自评质量分
//!   0..=1）并标 `confidence_defaulted = true`。**不用 1.0 兜底**——那等于凭空宣称
//!   满置信 = 编造。
//! * `weight`：`rubric::AdvisorDomain::default_weight()`（现有顾问注册权重，不另造一套）。
//! * `domain`：7 个顾问域一一同名对应（`Safety`/`Performance`/`Philosophy`/`History`/
//!   `Strategy`/`Ethics`/`Legal`），并写上票。

use apeireth_governance::rubric;
use apeireth_orchestration::{AdvisorDecision, AdvisorEvaluation, AdvisorKind};

use crate::canonical::council_ballot_log::CouncilBallotRecord;

/// `AdvisorKind` → `rubric::AdvisorDomain`（7 域一一同名对应；**不用字符串解析**）。
pub fn domain_for_kind(kind: AdvisorKind) -> rubric::AdvisorDomain {
    match kind {
        AdvisorKind::Safety => rubric::AdvisorDomain::Safety,
        AdvisorKind::Performance => rubric::AdvisorDomain::Performance,
        AdvisorKind::Philosophy => rubric::AdvisorDomain::Philosophy,
        AdvisorKind::History => rubric::AdvisorDomain::History,
        AdvisorKind::Strategy => rubric::AdvisorDomain::Strategy,
        AdvisorKind::Ethics => rubric::AdvisorDomain::Ethics,
        AdvisorKind::Legal => rubric::AdvisorDomain::Legal,
    }
}

/// `AdvisorDecision` → `rubric::StanceKind`（owner 裁定口径）。
///
/// `Retry`（要重做）**既非赞成也非反对** ⇒ 落在 rubric 现有的 `Neutral`（score 0.0）；
/// rubric 的 `StanceKind` 枚举里没有 `Hold` 变体（见 `governance::rubric`）。
/// 若日后 rubric owner 补上 `Hold`，改这一行即可（落盘记录里的 stance 字符串
/// 本来就按 owner 口径写 `"hold"`）。
pub fn stance_for_decision(decision: AdvisorDecision) -> rubric::StanceKind {
    match decision {
        AdvisorDecision::Allow => rubric::StanceKind::Approve,
        AdvisorDecision::Stop => rubric::StanceKind::Disapprove,
        AdvisorDecision::Retry => rubric::StanceKind::Neutral,
        AdvisorDecision::Abstain => rubric::StanceKind::Abstain,
    }
}

/// stance 的 snake_case 字符串（供落盘记录用）。
pub fn stance_slug(decision: AdvisorDecision) -> &'static str {
    match decision {
        AdvisorDecision::Allow => "approve",
        AdvisorDecision::Stop => "disapprove",
        AdvisorDecision::Retry => "hold",
        AdvisorDecision::Abstain => "abstain",
    }
}

/// `AdvisorKind` 的 snake_case 字符串（供落盘记录用）。
pub fn kind_slug(kind: AdvisorKind) -> &'static str {
    match kind {
        AdvisorKind::Safety => "safety",
        AdvisorKind::Performance => "performance",
        AdvisorKind::Philosophy => "philosophy",
        AdvisorKind::History => "history",
        AdvisorKind::Strategy => "strategy",
        AdvisorKind::Ethics => "ethics",
        AdvisorKind::Legal => "legal",
    }
}

/// D-2：一条顾问评价 ⇒ 一张立场票。
///
/// 口径见 owner 裁定（置信度显式优先、缺省用顾问自评质量分并标注兜底、
/// 权重用现有顾问注册权重、域一一对应）。**不编造**顾问没给的信息。
pub fn ballot_from_evaluation(evaluation: &AdvisorEvaluation) -> rubric::Ballot {
    let domain = domain_for_kind(evaluation.kind);
    rubric::Ballot::from_advice(
        evaluation.advisor.clone(),
        stance_for_decision(evaluation.verdict.verdict),
        evaluation.verdict.confidence,
        evaluation.verdict.score,
        domain.default_weight(),
        domain,
    )
}

/// D-2：一条顾问评价 ⇒ 一条可落盘的记录（供 append-only 账）。
///
/// `at_ms` 是落盘时刻（epoch 毫秒）。记录里的 stance/domain 用 snake_case 字符串，
/// 数值字段与票一致（置信度取票上那个——已被 `from_advice` 夹到 0..=1）。
pub fn record_from_evaluation(evaluation: &AdvisorEvaluation, at_ms: i64) -> CouncilBallotRecord {
    let ballot = ballot_from_evaluation(evaluation);
    CouncilBallotRecord {
        advisor: evaluation.advisor.clone(),
        stance: stance_slug(evaluation.verdict.verdict).to_string(),
        confidence: ballot.confidence,
        confidence_defaulted: ballot.confidence_defaulted,
        weight: ballot.weight,
        domain: Some(kind_slug(evaluation.kind).to_string()),
        advisor_score: evaluation.verdict.score,
        at_ms,
    }
}

/// D-2：账里读回的记录 ⇒ 立场票（守夜人复盘段的输入）。
///
/// * **反解析**：stance/domain 是 snake_case 字符串 ⇒ 认不出来的值**跳过**
///   （宁可少一条，不硬塞一个错的立场）。
/// * 权重取**记录里落盘的值**（落盘时已是「现有顾问注册权重」⇒ 复盘用的权重与
///   当时一致，不因后续改权重而漂移）。
/// * `confidence_defaulted` 原样带回（复盘段据此能说清「这个置信度是不是兜底的」）。
pub fn ballots_from_records(records: &[CouncilBallotRecord]) -> Vec<rubric::Ballot> {
    let mut ballots = Vec::with_capacity(records.len());
    for record in records {
        let Some(stance) = stance_from_slug(&record.stance) else {
            continue;
        };
        let domain = record
            .domain
            .as_deref()
            .and_then(domain_from_slug)
            .unwrap_or(rubric::AdvisorDomain::Safety);
        ballots.push(rubric::Ballot {
            advisor_id: record.advisor.clone(),
            stance,
            confidence: record.confidence.clamp(0.0, 1.0),
            weight: record.weight,
            domain: Some(domain),
            confidence_defaulted: record.confidence_defaulted,
        });
    }
    ballots
}

/// snake_case 立场名 ⇒ `StanceKind`（认不出来 ⇒ None）。
fn stance_from_slug(slug: &str) -> Option<rubric::StanceKind> {
    Some(match slug {
        "strong_approve" => rubric::StanceKind::StrongApprove,
        "approve" => rubric::StanceKind::Approve,
        "neutral" => rubric::StanceKind::Neutral,
        // 「hold」= 顾问要重做（Retry）落盘用的名字（owner 口径）⇒ 语义等价 Neutral。
        // 不认它 ⇒ Retry 的票在回放时被整条丢掉（复盘少一条），故显式承认。
        "hold" => rubric::StanceKind::Neutral,
        "disapprove" => rubric::StanceKind::Disapprove,
        "strong_disapprove" => rubric::StanceKind::StrongDisapprove,
        "abstain" => rubric::StanceKind::Abstain,
        _ => return None,
    })
}

/// snake_case 域名 ⇒ `AdvisorDomain`（认不出来 ⇒ None）。
fn domain_from_slug(slug: &str) -> Option<rubric::AdvisorDomain> {
    Some(match slug {
        "safety" => rubric::AdvisorDomain::Safety,
        "performance" => rubric::AdvisorDomain::Performance,
        "philosophy" => rubric::AdvisorDomain::Philosophy,
        "history" => rubric::AdvisorDomain::History,
        "strategy" => rubric::AdvisorDomain::Strategy,
        "ethics" => rubric::AdvisorDomain::Ethics,
        "legal" => rubric::AdvisorDomain::Legal,
        _ => return None,
    })
}

#[cfg(test)]
#[path = "council_ballots_test.rs"]
mod council_ballots_test;
