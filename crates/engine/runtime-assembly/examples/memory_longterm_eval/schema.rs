//! Data contracts for the standalone long-range memory evaluation.
//!
//! Every contract carries a protocol string so that a stale artifact can be
//! rejected instead of silently reinterpreted. The oracle is authored
//! independently from the input bundle and never enters a model-facing
//! context.

use serde::{Deserialize, Serialize};

pub const INPUT_PROTOCOL: &str = "apeireth.memory-longterm-input.v1";
pub const ORACLE_PROTOCOL: &str = "apeireth.memory-longterm-oracle.v1";
pub const SCORING_PROTOCOL: &str = "apeireth.memory-longterm-score.v1";
pub const EVIDENCE_PROTOCOL: &str = "apeireth.memory-longterm-evidence.v1";
pub const SEAL_PROTOCOL: &str = "apeireth.memory-longterm-seal.v1";
pub const FREEZE_PROTOCOL: &str = "apeireth.memory-longterm-freeze.v1";
pub const PRESSURE_PROTOCOL: &str = "apeireth.memory-longterm-pressure.v1";
pub const MANIFEST_PROTOCOL: &str = "apeireth.memory-longterm-manifest.v1";
pub const REPORT_PROTOCOL: &str = "apeireth.memory-longterm-report.v1";

/// Answer contract for the reader. It is a fixed prompt: its digest is bound
/// by the freeze receipt, so a scoring run cannot silently change the rules.
pub const READER_SYSTEM: &str = "仅依据本次交付的可信记忆回答。引用、假设、虚构或他人陈述不能替代直接用户证据。若记录标记citable_as_fact=false则不得作为事实引用。只输出一个JSON对象：{\"status\":\"known|unknown|ambiguous\",\"facts\":[{\"subject\":\"主体\",\"predicate\":\"属性\",\"value\":\"值\",\"valid_from\":null,\"valid_to\":null,\"sources\":[\"实际记忆ID\"]}],\"steps\":[{\"id\":\"步骤ID\",\"after\":null,\"sources\":[\"实际记忆ID\"]}]}。sources逐字填写交付正文中[mem:ID ...]的ID裸字符串，不加前缀或方括号。unknown或ambiguous的facts必须为空。不要输出其他字段、代码块或解释。";

/// Setup-turn contract: acknowledge without inventing new facts.
pub const ACK_SYSTEM: &str =
    "对于用户提供的真实陈述，只简短确认，不额外编造。明确标注的引用、假设和虚构不是真实事实。";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TurnKind {
    /// A direct user assertion about the user or a project.
    Assert,
    /// A statement that carries no fact.
    Noise,
    /// A quoted or explicitly fictional statement.
    Quoted,
    /// An instruction whose target subject is not determined.
    Ambiguous,
    /// A statement about another subject.
    Report,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionSpec {
    pub id: String,
    pub owner: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TurnSpec {
    pub id: String,
    pub session: String,
    pub at_utc: String,
    pub text: String,
    pub kind: TurnKind,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum Operation {
    /// Withdraw one recorded source. `turn` names the setup turn whose record
    /// is withdrawn; the runner resolves it to the stored episode.
    Forget { turn: String, reason: String },
    /// Withdraw every source of one owner.
    Revoke { owner: String },
    /// Re-open a session after an interrupted write.
    Recover { session: String, at_utc: String },
    /// Inject or remove a write fault.
    Fault { enabled: bool },
}

/// One query of the corpus. The identifier links a query to its oracle case
/// without carrying any oracle content into the model-facing request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Query {
    pub id: String,
    pub session: String,
    pub at_utc: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InputBundle {
    pub protocol: String,
    pub corpus_id: String,
    pub owner: String,
    pub sessions: Vec<SessionSpec>,
    pub turns: Vec<TurnSpec>,
    pub operations: Vec<Operation>,
    pub queries: Vec<Query>,
    pub notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Known,
    Unknown,
    Ambiguous,
    Denied,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Fact {
    pub subject: String,
    pub predicate: String,
    pub value: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AnswerFact {
    pub subject: String,
    pub predicate: String,
    pub value: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Step {
    pub id: String,
    pub after: Option<String>,
    pub sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Answer {
    pub status: Status,
    pub facts: Vec<AnswerFact>,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExpectedFact {
    pub fact: Fact,
    /// Literal anchors that the cited delivered record must contain. An
    /// anchor is decidable by the grader alone; a storage-internal content
    /// hash is not knowable by an independently authored oracle.
    pub support_anchors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequiredStep {
    pub id: String,
    pub after: Option<String>,
    pub support_anchors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OracleCase {
    pub id: String,
    pub dimension: String,
    pub status: Status,
    pub facts: Vec<ExpectedFact>,
    pub required_steps: Vec<RequiredStep>,
    pub forbidden_steps: Vec<String>,
    /// Literal anchors that must not appear in the delivered text of any
    /// record the answer cites. They encode "this source may not be used".
    #[serde(default)]
    pub forbidden_anchors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OracleBundle {
    pub protocol: String,
    pub input_sha256: String,
    /// Opaque token that must not appear in any model-facing request, stored
    /// record or run artifact. It makes oracle leakage directly decidable.
    pub canary: String,
    pub cases: Vec<OracleCase>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PremiseKind {
    /// The record traces to a committed direct user request.
    DirectUserEvidence,
    /// The Host granted the capability that produced the delivery.
    HostAuthorization,
    /// The record is inside the current scope of the querying partition.
    CurrentScope,
    /// The delivered snapshot still matches the current governed revision.
    RevisionCas,
}

impl PremiseKind {
    pub fn label(self) -> &'static str {
        match self {
            PremiseKind::DirectUserEvidence => "direct_user_evidence",
            PremiseKind::HostAuthorization => "host_authorization",
            PremiseKind::CurrentScope => "current_scope",
            PremiseKind::RevisionCas => "revision_cas",
        }
    }

    /// The question this premise answers, in one line.
    pub fn question(self) -> &'static str {
        match self {
            PremiseKind::DirectUserEvidence => "这条记录是否追溯到一次已提交的直接用户输入？",
            PremiseKind::HostAuthorization => "Host 是否授予了产生这次投递的能力？",
            PremiseKind::CurrentScope => "这条记录是否在当前查询作用域内？",
            PremiseKind::RevisionCas => "投递的快照是否仍等于当前的受管修订？",
        }
    }

    pub const ALL: [PremiseKind; 4] = [
        PremiseKind::DirectUserEvidence,
        PremiseKind::HostAuthorization,
        PremiseKind::CurrentScope,
        PremiseKind::RevisionCas,
    ];
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Premise {
    pub kind: PremiseKind,
    pub satisfied: bool,
    /// Human-checkable proof of the decision.
    pub proof: String,
    /// Concrete locators: ids, revisions, capability names, row names.
    pub locators: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceEvidence {
    pub id: String,
    pub episode_id: String,
    pub revision: i64,
    pub content_sha256: String,
    pub delivered_sha256: String,
    pub delivered_text: String,
    pub source_role: String,
    pub claim_kind: String,
    pub citable_as_fact: bool,
    pub active: bool,
    pub projection_verified: bool,
    pub premises: Vec<Premise>,
    pub eligible: bool,
    pub failures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PremiseSummary {
    pub kind: PremiseKind,
    pub label: String,
    pub source_count: usize,
    pub satisfied_count: usize,
    /// True when the premise was decided for every delivered source.
    pub decidable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OracleIsolation {
    pub canary: String,
    /// True when the canary never reached a model-facing request.
    pub absent_from_requests: bool,
    /// True when the canary never reached a stored memory record.
    pub absent_from_store: bool,
    /// True when the canary never reached a run artifact.
    pub absent_from_artifacts: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EvidenceReceipt {
    pub protocol: String,
    pub case: String,
    pub partition: String,
    pub granted_capabilities: Vec<String>,
    /// How many committed user inputs the runtime-side audit observed. The
    /// direct-user premise is decided against this trail.
    pub observed_committed_inputs: usize,
    pub sources: Vec<SourceEvidence>,
    pub premise_summary: Vec<PremiseSummary>,
    pub oracle_isolation: OracleIsolation,
    pub issues: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CitationTrace {
    /// Identifier as written by the reader.
    pub cited_id: String,
    /// Matching evidence entry, empty when the citation has no evidence.
    pub evidence_id: String,
    pub episode_id: String,
    pub revision: i64,
    pub content_sha256: String,
    pub eligible: bool,
    pub anchors_matched: Vec<String>,
    pub anchors_missing: Vec<String>,
    pub premise_failures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Grade {
    pub protocol: String,
    pub case: String,
    pub passed: bool,
    pub errors: Vec<String>,
    pub citation_traces: Vec<CitationTrace>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Span {
    pub case: String,
    pub stage: String,
    pub elapsed_us: u64,
    pub outcome: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Distribution {
    pub stage: String,
    pub count: usize,
    pub failed: usize,
    pub p50_us: u64,
    pub p95_us: u64,
    pub max_us: u64,
}

/// Input validation: the bundle must be internally consistent before any
/// record is written or any model request is prepared.
pub fn validate_input(bundle: &InputBundle) -> Result<(), String> {
    if bundle.protocol != INPUT_PROTOCOL {
        return Err(format!("input protocol mismatch: {}", bundle.protocol));
    }
    if bundle.corpus_id.trim().is_empty() {
        return Err("corpus id is empty".into());
    }
    if bundle.sessions.is_empty() {
        return Err("input has no session".into());
    }
    let mut session_ids = Vec::new();
    for session in &bundle.sessions {
        if session.id.trim().is_empty() || session.owner.trim().is_empty() {
            return Err("session id or owner is empty".into());
        }
        if session_ids.contains(&session.id) {
            return Err(format!("duplicate session id: {}", session.id));
        }
        session_ids.push(session.id.clone());
    }
    let mut turn_ids = Vec::new();
    for turn in &bundle.turns {
        if turn.id.trim().is_empty() {
            return Err("turn id is empty".into());
        }
        if turn_ids.contains(&turn.id) {
            return Err(format!("duplicate turn id: {}", turn.id));
        }
        turn_ids.push(turn.id.clone());
        if !session_ids.contains(&turn.session) {
            return Err(format!("turn {} names an unknown session", turn.id));
        }
        if turn.text.trim().is_empty() {
            return Err(format!("turn {} has empty text", turn.id));
        }
    }
    for operation in &bundle.operations {
        match operation {
            Operation::Forget { turn, .. } => {
                if !turn_ids.contains(turn) {
                    return Err(format!("forget names an unknown turn: {turn}"));
                }
            }
            Operation::Revoke { owner } => {
                if owner.trim().is_empty() {
                    return Err("revoke has an empty owner".into());
                }
            }
            Operation::Recover { session, .. } => {
                if !session_ids.contains(session) {
                    return Err(format!("recover names an unknown session: {session}"));
                }
            }
            Operation::Fault { .. } => {}
        }
    }
    let mut query_ids = Vec::new();
    for query in &bundle.queries {
        if query.id.trim().is_empty() {
            return Err("query id is empty".into());
        }
        if query_ids.contains(&query.id) {
            return Err(format!("duplicate query id: {}", query.id));
        }
        query_ids.push(query.id.clone());
        if !session_ids.contains(&query.session) {
            return Err(format!("query {} names an unknown session", query.id));
        }
        if query.text.trim().is_empty() {
            return Err(format!("query {} has empty text", query.id));
        }
    }
    Ok(())
}
/// Oracle validation: every case must be decidable and must not contradict
/// the status it declares.
pub fn validate_oracle(oracle: &OracleBundle, input: &InputBundle) -> Result<(), String> {
    if oracle.protocol != ORACLE_PROTOCOL {
        return Err(format!("oracle protocol mismatch: {}", oracle.protocol));
    }
    if oracle.canary.trim().is_empty() {
        return Err("oracle canary is empty".into());
    }
    if oracle.cases.is_empty() {
        return Err("oracle has no case".into());
    }
    let mut ids = Vec::new();
    for case in &oracle.cases {
        if case.id.trim().is_empty() {
            return Err("oracle case id is empty".into());
        }
        if ids.contains(&case.id) {
            return Err(format!("duplicate oracle case id: {}", case.id));
        }
        ids.push(case.id.clone());
        if case.dimension.trim().is_empty() {
            return Err(format!("oracle case {} has no dimension", case.id));
        }
        // ⭐ Three statuses, three rules (see the grading layer): an
        // ambiguous case must not assert a fact, but it may name the source
        // that makes it ambiguous; an unknown or denied case must assert
        // nothing at all.
        match case.status {
            Status::Known => {}
            Status::Ambiguous => {
                if !case.facts.is_empty() {
                    return Err(format!(
                        "oracle case {} is ambiguous but asserts facts",
                        case.id
                    ));
                }
            }
            _ => {
                if !case.facts.is_empty() || !case.required_steps.is_empty() {
                    return Err(format!(
                        "oracle case {} is not known but asserts facts or steps",
                        case.id
                    ));
                }
            }
        }
        for expected in &case.facts {
            if expected.fact.subject.trim().is_empty()
                || expected.fact.predicate.trim().is_empty()
                || expected.fact.value.trim().is_empty()
            {
                return Err(format!("oracle case {} has an empty fact field", case.id));
            }
            if expected.support_anchors.is_empty() {
                return Err(format!(
                    "oracle case {} has a fact without a support anchor",
                    case.id
                ));
            }
        }
        let mut step_ids = Vec::new();
        for step in &case.required_steps {
            if step_ids.contains(&step.id) {
                return Err(format!(
                    "oracle case {} repeats required step {}",
                    case.id, step.id
                ));
            }
            step_ids.push(step.id.clone());
            if case.forbidden_steps.contains(&step.id) {
                return Err(format!(
                    "oracle case {} both requires and forbids step {}",
                    case.id, step.id
                ));
            }
        }
        for step in &case.required_steps {
            if let Some(after) = &step.after {
                if !step_ids.contains(after) {
                    return Err(format!(
                        "oracle case {} requires step {} after unknown step {}",
                        case.id, step.id, after
                    ));
                }
            }
        }
        for anchor in &case.forbidden_anchors {
            if anchor.trim().is_empty() {
                return Err(format!(
                    "oracle case {} has an empty forbidden anchor",
                    case.id
                ));
            }
            if case.facts.iter().any(|expected| {
                expected
                    .support_anchors
                    .iter()
                    .any(|support| support.contains(anchor))
            }) {
                return Err(format!(
                    "oracle case {} both requires and forbids anchor {}",
                    case.id, anchor
                ));
            }
        }
    }
    if !ids.is_empty() && input.turns.is_empty() {
        return Err("oracle is not empty but the input has no turn".into());
    }
    // Denominator equality: every query has exactly one oracle case and every
    // oracle case answers exactly one query.
    let query_ids: Vec<&String> = input.queries.iter().map(|query| &query.id).collect();
    for id in &ids {
        if !query_ids.contains(&id) {
            return Err(format!("oracle case {id} has no matching query"));
        }
    }
    for query_id in &query_ids {
        if !ids.contains(query_id) {
            return Err(format!("query {query_id} has no oracle case"));
        }
    }
    Ok(())
}
