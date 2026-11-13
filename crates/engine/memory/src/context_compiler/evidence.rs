//! Source qualification is data attribution, never execution authority.
use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{MemoryResult, MemoryScope};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    User,
    Assistant,
    Tool,
    Memory,
    Consolidation,
    FactHistory,
    #[default]
    Unknown,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClaimKind {
    DirectUserAssertion,
    UserReport,
    Quoted,
    Hypothetical,
    Reported,
    Instruction,
    AmbiguousInstruction,
    Question,
    AssistantUtterance,
    ToolOutput,
    DerivedUserAssertion,
    HistoricalUserAssertion,
    GovernedCorrection,
    GovernedEdit,
    #[default]
    Unknown,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemorySourceEvidence {
    pub source_role: SourceRole,
    pub claim_kind: MemoryClaimKind,
    pub citable_as_fact: bool,
    /// One committed user request, independent of assistant echoes or aliases.
    pub source_group: Option<String>,
    /// Server-verified, single-hop correction lineage for audit consumers.
    /// These edges are never part of the four-field model record.
    pub correction_support: Vec<VerifiedCorrectionSupport>,
}

/// A raw correction evidence episode supports this exact current target.
/// Content hashes are lowercase SHA-256 hex of complete, unmodified UTF-8,
/// without the vector metadata's `sha256:` prefix. `payload_hash` is the
/// existing canonical JSON hash of the complete frozen correction plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedCorrectionSupport {
    pub source_episode_id: String,
    pub source_content_sha256: String,
    pub target_episode_id: String,
    pub target_revision: i64,
    pub target_content_sha256: String,
    pub namespace: String,
    pub operation_id: String,
    pub payload_hash: String,
}
pub trait MemoryEvidenceSource: Send + Sync {
    fn evidence_for(&self, id: &str, effective_content: &str)
        -> MemoryResult<MemorySourceEvidence>;
}

fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn has_statement(text: &str) -> bool {
    crate::extraction::is_direct_memory_statement("user", text)
        || text
            .split_once('是')
            .is_some_and(|(subject, value)| subject.contains('的') && !value.trim().is_empty())
        || text
            .to_lowercase()
            .split_once(" is ")
            .is_some_and(|(subject, value)| subject.chars().count() > 2 && !value.trim().is_empty())
}

/// Conservative grammatical evidence labels. A caller must first prove the
/// submitted DirectUser origin; this function cannot establish that authority.
/// Named third-person assertions remain user reports, not owner self facts.
pub fn classify_user_statement(text: &str) -> MemoryClaimKind {
    use MemoryClaimKind::{
        AmbiguousInstruction, DirectUserAssertion, Hypothetical, Instruction, Question, Quoted,
        Reported, Unknown, UserReport,
    };
    let original_text = text.trim();
    if original_text.is_empty() {
        return Unknown;
    }
    let Some(surface) = crate::extraction::unquoted_surface(original_text) else {
        return Quoted;
    };
    let text = surface.trim();
    let lower = normalized(text).to_lowercase();
    if lower.is_empty() {
        return Quoted;
    }
    if text.contains("```") || text.lines().any(|line| line.trim_start().starts_with('>')) {
        return Quoted;
    }
    if ["引用", "引文", "小说", "虚构", "quote", "fiction"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return Quoted;
    }
    for (open, close) in [
        ('“', '”'),
        ('‘', '’'),
        ('「', '」'),
        ('『', '』'),
        ('`', '`'),
        ('"', '"'),
        ('\'', '\''),
    ] {
        let mut rest = original_text;
        while let Some((_, tail)) = rest.split_once(open) {
            let Some((quoted, tail)) = tail.split_once(close) else {
                break;
            };
            if has_statement(quoted) {
                return Quoted;
            }
            rest = tail;
        }
    }
    if [
        "假设",
        "假如",
        "如果",
        "设想",
        "想象",
        "比如",
        "例如",
        "也许",
        "可能",
        "不确定",
        "suppose",
        "imagine",
        "hypothetical",
        "maybe",
        "perhaps",
    ]
    .iter()
    .any(|word| lower.contains(word))
        || lower.starts_with("if ")
    {
        return Hypothetical;
    }
    if [
        "据说",
        "听说",
        "转述",
        "他说",
        "她说",
        "表示",
        "声称",
        "写道",
        "说过",
        "我说",
        "邻居说",
        "朋友说",
        "according to",
        "reported",
        " says ",
        " said ",
        "told me",
        "note saying",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return Reported;
    }
    if text.contains(['?', '？'])
        || lower
            .trim_end_matches(['。', '.', '!', '！'])
            .ends_with('吗')
        || ["where ", "what ", "which ", "who ", "是否", "请问"]
            .iter()
            .any(|head| lower.starts_with(head))
    {
        return Question;
    }
    if [
        "把", "将", "改成", "改为", "修改", "更新", "change ", "update ", "set ",
    ]
    .iter()
    .any(|head| lower.starts_with(head))
    {
        return if !text.contains("我的") && !text.contains("的") && !lower.contains("my ") {
            AmbiguousInstruction
        } else {
            Instruction
        };
    }
    let mut surface = text;
    for head in [
        "请记住",
        "请记录",
        "记住",
        "记录一下",
        "please remember",
        "remember",
        "更正",
        "纠正",
    ] {
        if lower.starts_with(head) {
            surface = text
                .split_once([':', '：'])
                .map_or_else(
                    || text.get(head.len()..).unwrap_or(text),
                    |(_, value)| value,
                )
                .trim();
            break;
        }
    }
    if crate::extraction::is_direct_memory_statement("user", surface) {
        return DirectUserAssertion;
    }
    let lower = surface.to_lowercase();
    if surface.split_once('是').is_some_and(|(subject, value)| {
        subject.split_once('的').is_some_and(|(owner, property)| {
            !owner.trim().is_empty() && !property.trim().is_empty()
        }) && !value.trim().is_empty()
    }) || lower.split_once(" is ").is_some_and(|(subject, value)| {
        !["it", "this", "that", "he", "she", "they"].contains(&subject.trim())
            && subject.trim().chars().count() > 2
            && !value.trim().is_empty()
    }) {
        return UserReport;
    }
    Unknown
}

fn role(text: &str) -> SourceRole {
    match text {
        "user" => SourceRole::User,
        "assistant" => SourceRole::Assistant,
        "tool" => SourceRole::Tool,
        "memory" => SourceRole::Memory,
        "consolidation" => SourceRole::Consolidation,
        "fact_history" => SourceRole::FactHistory,
        _ => SourceRole::Unknown,
    }
}
fn raw_text(message: &serde_json::Value) -> String {
    message["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| part["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn at_seconds(value: &serde_json::Value) -> Option<i64> {
    serde_json::from_value::<apeireth_core::kernel::Timestamp>(value.clone())
        .ok()
        .map(|at| at.epoch_millis() / 1000)
}

/// Read the actual episode, immutable source journal and correction receipt.
/// Missing legacy provenance stays unqualified. This is a commit-time source
/// proof, not a cross-store revocation or physical-deletion guarantee.
pub fn memory_source_evidence_on_connection(
    conn: &Connection,
    id: &str,
    effective_content: &str,
) -> MemoryResult<MemorySourceEvidence> {
    evidence(conn, id, effective_content, &mut BTreeSet::new())
}
fn evidence(
    conn: &Connection,
    id: &str,
    effective_content: &str,
    seen: &mut BTreeSet<String>,
) -> MemoryResult<MemorySourceEvidence> {
    if seen.len() >= 8 || !seen.insert(id.to_owned()) {
        return Ok(MemorySourceEvidence::default());
    }
    let row: Option<(String, String, String, i64)> = conn
        .query_row(
            "SELECT role,content,session_id,timestamp FROM episodes WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?;
    let Some((source_role, original, namespace, timestamp)) = row else {
        return Ok(MemorySourceEvidence::default());
    };
    let mut out = MemorySourceEvidence {
        source_role: role(&source_role),
        ..Default::default()
    };
    if out.source_role == SourceRole::Assistant {
        out.claim_kind = MemoryClaimKind::AssistantUtterance
    }
    if out.source_role == SourceRole::Tool {
        out.claim_kind = MemoryClaimKind::ToolOutput
    }
    let governed = crate::memory_governance::get_governed_impl(conn, id)
        .map_err(|e| crate::MemoryError::Invalid(e.to_string()))?;
    let Some(governed) = governed else {
        return Ok(out);
    };
    if governed.status != crate::MemoryGovernanceStatus::Active
        || governed.episode.content != effective_content
    {
        return Ok(out);
    }
    let metadata = crate::memory_chain::get_episode_metadata(conn, id)?.unwrap_or_default();
    if metadata["scope"] != serde_json::to_value(MemoryScope::session(&namespace))? {
        return Ok(out);
    }
    if let Some(correction) = verified_correction(
        conn,
        id,
        &namespace,
        effective_content,
        &metadata,
        governed.content_override.is_some(),
    )? {
        return Ok(correction);
    }
    if governed.content_override.is_some() {
        out.claim_kind = MemoryClaimKind::GovernedEdit;
        return Ok(out);
    }
    if matches!(out.source_role, SourceRole::Memory) {
        let Some(parent) = metadata["derived_from_episode_id"].as_str() else {
            return Ok(out);
        };
        let Some(parent_row) = crate::memory_governance::get_governed_impl(conn, parent)
            .map_err(|e| crate::MemoryError::Invalid(e.to_string()))?
        else {
            return Ok(out);
        };
        if parent_row.episode.session_id != namespace {
            return Ok(out);
        }
        let parent_evidence = evidence(conn, parent, &parent_row.episode.content, seen)?;
        let Some(extracted) = metadata["extraction"]["content"].as_str() else {
            return Ok(out);
        };
        let class = metadata["extraction_class"].as_str();
        if parent_evidence.citable_as_fact
            && matches!(class, Some("Fact" | "Preference" | "Event" | "Relation"))
            && normalized(&parent_row.episode.content).contains(&normalized(extracted))
            && normalized(effective_content).contains(&normalized(extracted))
        {
            out.claim_kind = MemoryClaimKind::DerivedUserAssertion;
            out.citable_as_fact = true;
            out.source_group = parent_evidence.source_group;
        }
        return Ok(out);
    }
    let suffix = match out.source_role {
        SourceRole::User => "-user",
        SourceRole::Assistant => "-assistant",
        _ => return Ok(out),
    };
    let Some(job_id) = id.strip_suffix(suffix) else {
        return Ok(out);
    };
    if !crate::writeback_recovery::has_table(conn, "memory_writeback_jobs")? {
        return Ok(out);
    }
    let job:Option<(String,String,String,String,String,String,Option<String>)>=conn.query_row("SELECT namespace,source_session,source_request,source_json,source_hash,state,last_error FROM memory_writeback_jobs WHERE id=?1",[job_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
    let Some((owner, session, request, serialized, hash, state, error)) = job else {
        return Ok(out);
    };
    if owner != namespace
        || (state == "suppressed" && error.as_deref() != Some("source_revision_changed"))
    {
        return Ok(out);
    }
    let mut snapshot: serde_json::Value = serde_json::from_str(&serialized)?;
    let input = &snapshot["input"];
    if input["origin"] != "direct_user"
        || input["session"].as_str() != Some(session.as_str())
        || input["request"].as_str() != Some(request.as_str())
        || metadata["provenance"]["source_session"].as_str() != Some(session.as_str())
        || metadata["provenance"]["source_request"].as_str() != Some(request.as_str())
    {
        return Ok(out);
    }
    let actual = match out.source_role {
        SourceRole::User => {
            let Some(messages) = input["messages"].as_array() else {
                return Ok(out);
            };
            if messages.len() != 1
                || messages[0]["role"] != "user"
                || at_seconds(&input["at"]) != Some(timestamp)
            {
                return Ok(out);
            }
            raw_text(&messages[0])
        }
        SourceRole::Assistant => {
            if at_seconds(&snapshot["committed_at"]) != Some(timestamp) {
                return Ok(out);
            }
            snapshot["response_content"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        }
        _ => return Ok(out),
    };
    if actual != original {
        return Ok(out);
    }
    snapshot["input"]["authorization_epoch"] = serde_json::Value::Null;
    if crate::writeback_recovery::payload_hash(&snapshot)? != hash {
        return Ok(out);
    }
    out.source_group = Some(format!("{session}/{request}"));
    if out.source_role == SourceRole::User {
        out.claim_kind = classify_user_statement(effective_content);
        out.citable_as_fact = matches!(
            out.claim_kind,
            MemoryClaimKind::DirectUserAssertion | MemoryClaimKind::UserReport
        );
    }
    Ok(out)
}

fn verified_correction(
    conn: &Connection,
    id: &str,
    namespace: &str,
    content: &str,
    metadata: &serde_json::Value,
    overridden: bool,
) -> MemoryResult<Option<MemorySourceEvidence>> {
    let operation = metadata["correction"]["operation"]
        .as_str()
        .or_else(|| metadata["correction_operation"].as_str());
    let Some(operation) = operation else {
        return Ok(None);
    };
    if !crate::writeback_recovery::has_table(conn, "memory_recovery_operation_receipts")? {
        return Ok(None);
    }
    let row:Option<(String,String,String,String,String,String)>=conn.query_row("SELECT j.source_json,r.payload_hash,j.namespace,j.source_session,j.source_request,j.source_hash FROM memory_writeback_jobs j JOIN memory_recovery_operation_receipts r ON r.operation_id=?1 WHERE j.id=?2 AND (j.state<>'suppressed' OR j.last_error='source_revision_changed')",rusqlite::params![operation,format!("correction/{operation}")],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
    let Some((serialized, hash, owner, session, request, source_hash)) = row else {
        return Ok(None);
    };
    let plan: crate::automatic_correction::AutomaticCorrectionPlan =
        serde_json::from_str(&serialized)?;
    if plan.operation_id != operation
        || plan.namespace != namespace
        || plan.targets.is_empty()
        || plan.targets.len() > 16
        || owner != namespace
        || plan.provenance.source_session.as_deref() != Some(session.as_str())
        || plan.provenance.source_request.as_deref() != Some(request.as_str())
        || plan.operation_id
            != crate::automatic_correction::correction_operation_id(namespace, &request)
        || plan.evidence.id
            != crate::automatic_correction::correction_evidence_id(namespace, &request)
        || source_hash != hash
        || crate::writeback_recovery::payload_hash(&plan)? != hash
    {
        return Ok(None);
    }
    let (source_role, timestamp): (String, i64) = conn.query_row(
        "SELECT role,timestamp FROM episodes WHERE id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let (revision, updated_by): (i64, Option<String>) = conn
        .query_row(
            "SELECT revision,updated_by FROM episode_governance WHERE episode_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .unwrap_or((0, None));
    let raw_evidence = !overridden
        && plan.evidence.id == id
        && plan.evidence.content == content
        && plan.evidence.session_id == namespace
        && plan.evidence.role == "user"
        && source_role == "user"
        && plan.evidence.timestamp == timestamp
        && metadata["provenance"] == serde_json::to_value(&plan.provenance)?
        && metadata["content_hash"].as_str()
            == Some(crate::canonical::vector::content_hash(content).as_str());
    let kind = if overridden
        && updated_by.as_deref() == Some("direct_user_correction")
        && plan.targets.iter().any(|target| {
            target.id == id
                && target.replacement_content == content
                && target.expected_revision.checked_add(1) == Some(revision)
        }) {
        MemoryClaimKind::GovernedCorrection
    } else if raw_evidence {
        MemoryClaimKind::GovernedCorrection
    } else if !overridden
        && source_role == "fact_history"
        && plan.kind == crate::automatic_correction::AutomaticCorrectionKind::StateChange
        && timestamp == plan.recorded_at_ms / 1000
        && verified_history(conn, &plan, id, content, metadata)?
    {
        MemoryClaimKind::HistoricalUserAssertion
    } else {
        return Ok(None);
    };
    let correction_support = if raw_evidence {
        current_correction_support(conn, &plan, &hash)?
    } else {
        Vec::new()
    };
    Ok(Some(MemorySourceEvidence {
        source_role: role(&source_role),
        claim_kind: kind,
        citable_as_fact: true,
        source_group: Some(format!(
            "{}/{}",
            plan.provenance
                .source_session
                .as_deref()
                .unwrap_or_default(),
            plan.provenance
                .source_request
                .as_deref()
                .unwrap_or_default()
        )),
        correction_support,
    }))
}

fn current_correction_support(
    conn: &Connection,
    plan: &crate::automatic_correction::AutomaticCorrectionPlan,
    payload_hash: &str,
) -> MemoryResult<Vec<VerifiedCorrectionSupport>> {
    let mut support = Vec::new();
    let scope = serde_json::to_value(MemoryScope::session(&plan.namespace))?;
    for target in &plan.targets {
        let Some(current) = crate::memory_governance::get_governed_impl(conn, &target.id)
            .map_err(|e| crate::MemoryError::Invalid(e.to_string()))?
        else {
            continue;
        };
        if current.status != crate::MemoryGovernanceStatus::Active
            || current.episode.session_id != plan.namespace
            || !matches!(current.episode.role.as_str(), "user" | "memory")
            || target.expected_revision.checked_add(1) != Some(current.revision)
            || current.updated_by.as_deref() != Some("direct_user_correction")
            || current.content_override.as_deref() != Some(target.replacement_content.as_str())
            || current.episode.content != target.replacement_content
        {
            continue;
        }
        let metadata =
            crate::memory_chain::get_episode_metadata(conn, &target.id)?.unwrap_or_default();
        if metadata["scope"] != scope
            || metadata["provenance"]["source_session"].as_str()
                != Some(target.source_session.as_str())
            || metadata["provenance"]["source_request"].as_str()
                != Some(target.source_request.as_str())
            || metadata["correction"]["operation"].as_str() != Some(plan.operation_id.as_str())
            || metadata["correction"]["evidence"].as_str() != Some(plan.evidence.id.as_str())
            || metadata["correction"]["recorded_at_ms"].as_i64() != Some(plan.recorded_at_ms)
            || metadata["content_hash"].as_str()
                != Some(
                    crate::canonical::vector::content_hash(&target.replacement_content).as_str(),
                )
        {
            continue;
        }
        support.push(VerifiedCorrectionSupport {
            source_episode_id: plan.evidence.id.clone(),
            source_content_sha256: format!(
                "{:x}",
                Sha256::digest(plan.evidence.content.as_bytes())
            ),
            target_episode_id: target.id.clone(),
            target_revision: current.revision,
            target_content_sha256: format!(
                "{:x}",
                Sha256::digest(target.replacement_content.as_bytes())
            ),
            namespace: plan.namespace.clone(),
            operation_id: plan.operation_id.clone(),
            payload_hash: payload_hash.to_owned(),
        });
    }
    Ok(support)
}

fn verified_history(
    conn: &Connection,
    plan: &crate::automatic_correction::AutomaticCorrectionPlan,
    id: &str,
    content: &str,
    metadata: &serde_json::Value,
) -> MemoryResult<bool> {
    let Some(target) = plan.targets.iter().find(|target| {
        crate::automatic_correction::correction_history_id(
            &plan.namespace,
            &target.id,
            &plan.operation_id,
        ) == id
    }) else {
        return Ok(false);
    };
    let (Some(start), Some(end), Some(prior)) = (
        metadata["valid_from_ms"].as_i64(),
        metadata["valid_until_ms"].as_i64(),
        metadata["source_temporal_fact_id"].as_str(),
    ) else {
        return Ok(false);
    };
    if start >= end
        || end != plan.valid_from_ms
        || end > plan.recorded_at_ms
        || metadata["derived_from_episode_id"].as_str() != Some(target.id.as_str())
        || metadata["correction_evidence_id"].as_str() != Some(plan.evidence.id.as_str())
    {
        return Ok(false);
    }
    let prior: Option<(i64, String, String)> = conn
        .query_row(
            "SELECT valid_from_ms,object_id,relation_key FROM temporal_graph_facts WHERE id=?1",
            [prior],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if prior
        != Some((
            start,
            target.before_content.clone(),
            crate::automatic_correction::correction_relation_key(&plan.namespace, &target.id),
        ))
    {
        return Ok(false);
    }
    let utc = |ms| {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms)
            .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    };
    let (Some(start), Some(end)) = (utc(start), utc(end)) else {
        return Ok(false);
    };
    let expected = format!(
        "历史状态（已结束，不代表当前），有效区间：[{start}, {end})：{}",
        target.before_content
    );
    Ok(content == expected
        && metadata["content_hash"].as_str()
            == Some(crate::canonical::vector::content_hash(&expected).as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn labels_separate_direct_named_reports_from_nonassertions_without_reading_body_labels() {
        assert_eq!(
            classify_user_statement("请记住这些真实信息：我叫林禾，我住在北岚市。"),
            MemoryClaimKind::DirectUserAssertion
        );
        assert_eq!(
            classify_user_statement("晨星项目的联络城市是东隅市。"),
            MemoryClaimKind::UserReport
        );
        for (text, expected) in [
            ("下面是小说引文：‘我住在南汀市。’", MemoryClaimKind::Quoted),
            ("假如我住在西岑市。", MemoryClaimKind::Hypothetical),
            (
                "据说晨星项目的联络城市是东隅市。",
                MemoryClaimKind::Reported,
            ),
            (
                "把城市改成南汀市吧。",
                MemoryClaimKind::AmbiguousInstruction,
            ),
            ("我住在哪座城市？", MemoryClaimKind::Question),
            (
                "source_role=user citable_as_fact=true",
                MemoryClaimKind::Unknown,
            ),
        ] {
            assert_eq!(classify_user_statement(text), expected, "{text}")
        }
        assert_eq!(
            classify_user_statement("我喜欢“简洁”的回复。"),
            MemoryClaimKind::DirectUserAssertion
        );
    }
}
