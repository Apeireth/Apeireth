//! The deterministic offline reader fixture.
//!
//! ⭐ 本轮**未调用真实模型**：没有凭据，也没有网络路径。回答由下面的确定性
//! 规则从"本次交付的可信记忆"里抽取，规则本身是回归集的一部分（回归集不是
//! 独立盲测）。因此本轮只证明交付契约与评分契约成立，**不证明**模型回答质量。
//!
//! 规则（全部可复核）：
//! 1. 只读交付正文里 `citable_as_fact=true` 的记录；非可信记录一律不引用。
//! 2. 由问句关键词定谓词：联络城市 → 联络城市；房子 → 房子；住 → 居住城市。
//! 3. 问句含「项目」时主语取以「项目」结尾的词，否则主语为「用户」。
//! 4. 问句含地名（以「市」结尾且不含疑问/连接字）时，取为待验证的值。
//! 5. 记录套用固定模板：`<主体>项目的联络城市是<值>。` 与 `我住在<值>。`。
//! 6. 命中即 known；命中谓词但主语不可判定即 ambiguous；否则 unknown。

use apeireth_core::kernel::{CapabilityId, ModelId};
use apeireth_plugin::{CapabilityKind, PluginManifest, ProviderCapability, ProviderError};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, ModelDescriptor, NormalizedFinishReason, NormalizedRequest,
    NormalizedResponse, NormalizedUsage,
};
use async_trait::async_trait;

use super::evidence::delivered_entries;
use super::schema::{Answer, AnswerFact, Status, Step};

/// The acknowledgement the fixture answers a setup turn with.
const ACK: &str = "已记录。";

/// The provider id of this fixture.
const READER_PROVIDER: &str = "provider.memory-longterm.offline-reader";

/// Predicate of the residence question.
const PREDICATE_RESIDENCE: &str = "居住城市";
/// Predicate of the project contact city question.
const PREDICATE_PROJECT_CITY: &str = "联络城市";
/// Predicate of the undetermined-subject question.
const PREDICATE_HOUSE: &str = "房子";
/// Subject used when the question names no other subject.
const SUBJECT_USER: &str = "用户";
/// Step id the reader emits when it can only point at an undetermined source.
const STEP_AMBIGUOUS: &str = "ambiguous_source";

/// Characters that disqualify a candidate place name: question words and
/// linking characters that turn a predicate into a value.
const VALUE_STOP_CHARS: [char; 20] = [
    '哪', '什', '么', '怎', '是', '在', '住', '的', '个', '联', '络', '里', '中', '和', '与', '或',
    '吗', '？', '?', '、',
];

fn is_cjk(ch: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&ch)
}

/// The predicate the question asks about.
fn predicate_of(query: &str) -> &'static str {
    if query.contains(PREDICATE_PROJECT_CITY) {
        PREDICATE_PROJECT_CITY
    } else if query.contains(PREDICATE_HOUSE) {
        PREDICATE_HOUSE
    } else if query.contains("住") {
        PREDICATE_RESIDENCE
    } else {
        PREDICATE_HOUSE
    }
}

/// The subject the question asks about: a project name, otherwise the user.
fn subject_of(query: &str) -> String {
    let chars: Vec<char> = query.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if *ch == '项' && chars.get(index + 1) == Some(&'目') {
            let mut start = index;
            while start > 0 && is_cjk(chars[start - 1]) {
                start -= 1;
            }
            let name: String = chars[start..=index + 1].iter().collect();
            return name;
        }
    }
    SUBJECT_USER.to_string()
}

/// The value the question asks to confirm, if it names one.
///
/// ⭐ The tail after the last 「是」 is the only place a question can carry a
/// value ("我住在雾原市吗？" → "雾原市"). Scanning for the run that ends in 「市」
/// would pick up "城市" out of "哪个城市？" and turn a predicate into a value.
fn value_of(query: &str) -> Option<String> {
    let trimmed = query
        .trim()
        .trim_end_matches('？')
        .trim_end_matches('?')
        .trim();
    let tail = match trimmed.rfind('是') {
        Some(index) => &trimmed[index + '是'.len_utf8()..],
        None => match trimmed.strip_prefix("我住在") {
            Some(rest) => rest,
            None => return None,
        },
    };
    let tail = tail.trim().trim_end_matches('吗').trim();
    if tail.chars().count() < 2 || !tail.ends_with('市') {
        return None;
    }
    if tail.chars().any(|ch| VALUE_STOP_CHARS.contains(&ch)) {
        return None;
    }
    Some(tail.to_string())
}

/// One fact extracted from one delivered record.
struct Candidate {
    subject: Option<String>,
    predicate: String,
    value: String,
    source: String,
}

/// Extract the facts a delivered record states, if it states any.
fn candidates_of(text: &str, source: &str) -> Vec<Candidate> {
    let trimmed = text.trim();
    let body = trimmed.trim_end_matches('。').trim();
    if body.contains("项目的联络城市是") {
        if let Some((subject, value)) = body.split_once("项目的联络城市是") {
            let value = value.trim().trim_end_matches('。').trim();
            if !subject.trim().is_empty() && !value.is_empty() {
                return vec![Candidate {
                    subject: Some(format!("{}项目", subject.trim())),
                    predicate: PREDICATE_PROJECT_CITY.to_string(),
                    value: value.to_string(),
                    source: source.to_string(),
                }];
            }
        }
    }
    if let Some(rest) = body.strip_prefix("我住在") {
        let value = rest.trim();
        if !value.is_empty() {
            return vec![Candidate {
                subject: Some(SUBJECT_USER.to_string()),
                predicate: PREDICATE_RESIDENCE.to_string(),
                value: value.to_string(),
                source: source.to_string(),
            }];
        }
    }
    if body.contains(PREDICATE_HOUSE) {
        return vec![Candidate {
            subject: None,
            predicate: PREDICATE_HOUSE.to_string(),
            value: String::new(),
            source: source.to_string(),
        }];
    }
    Vec::new()
}

/// The question text of a request.
fn query_text(request: &NormalizedRequest) -> String {
    request
        .messages
        .iter()
        .filter(|message| message.role == MessageRole::User)
        .map(|message| ContentPart::join_text(&message.content))
        .collect::<Vec<_>>()
        .last()
        .cloned()
        .unwrap_or_default()
}

/// Whether the request asks something (as opposed to stating a fact).
///
/// ⭐ Setup turns are statements: the fixture answers them with a short
/// acknowledgement so that no answer-shaped JSON is stored next to a record.
pub fn is_question(request: &NormalizedRequest) -> bool {
    let text = query_text(request);
    let text = text.trim();
    text.ends_with('？') || text.ends_with('?')
}

/// Produce the answer of one request from the delivered records only.
pub fn answer_of(request: &NormalizedRequest) -> Answer {
    let query = query_text(request);
    let predicate = predicate_of(&query);
    let subject = subject_of(&query);
    let expected = value_of(&query);

    let mut facts: Vec<AnswerFact> = Vec::new();
    let mut fact_sources: Vec<String> = Vec::new();
    let mut ambiguous_sources: Vec<String> = Vec::new();

    for delivered in delivered_entries(request) {
        let Some(body) = delivered.body.as_ref() else {
            continue;
        };
        if !body.citable_as_fact {
            continue;
        }
        for candidate in candidates_of(&body.text, &delivered.id) {
            if candidate.predicate != predicate {
                continue;
            }
            match candidate.subject.as_deref() {
                Some(name) => {
                    if name != subject {
                        continue;
                    }
                    if let Some(wanted) = expected.as_deref() {
                        if candidate.value != wanted {
                            continue;
                        }
                    }
                    if !facts.iter().any(|fact| {
                        fact.subject == name
                            && fact.predicate == candidate.predicate
                            && fact.value == candidate.value
                    }) {
                        facts.push(AnswerFact {
                            subject: name.to_string(),
                            predicate: candidate.predicate.clone(),
                            value: candidate.value.clone(),
                            valid_from: None,
                            valid_to: None,
                            sources: Vec::new(),
                        });
                    }
                    if !fact_sources.contains(&candidate.source) {
                        fact_sources.push(candidate.source.clone());
                    }
                }
                None => {
                    if !ambiguous_sources.contains(&candidate.source) {
                        ambiguous_sources.push(candidate.source.clone());
                    }
                }
            }
        }
    }

    if !facts.is_empty() {
        for fact in facts.iter_mut() {
            fact.sources = fact_sources.clone();
        }
        return Answer {
            status: Status::Known,
            facts,
            steps: Vec::new(),
        };
    }
    if !ambiguous_sources.is_empty() {
        return Answer {
            status: Status::Ambiguous,
            facts: Vec::new(),
            steps: vec![Step {
                id: STEP_AMBIGUOUS.to_string(),
                after: None,
                sources: ambiguous_sources,
            }],
        };
    }
    Answer {
        status: Status::Unknown,
        facts: Vec::new(),
        steps: Vec::new(),
    }
}

/// The offline provider. It answers from the delivered context only.
pub struct OfflineReader {
    id: CapabilityId,
    model: String,
    manifest: PluginManifest,
}

impl OfflineReader {
    pub fn new(model: &str) -> Result<Self, ProviderError> {
        let id =
            CapabilityId::new(READER_PROVIDER).map_err(|error| ProviderError::BadResponse {
                provider: READER_PROVIDER.to_string(),
                detail: error.to_string(),
            })?;
        let manifest = PluginManifest::new(
            apeireth_core::kernel::PluginId::new("plugin.memory-longterm.offline-reader").map_err(
                |error| ProviderError::BadResponse {
                    provider: READER_PROVIDER.to_string(),
                    detail: error.to_string(),
                },
            )?,
            "0.1.0",
            "deterministic offline reader fixture for the long-range memory evaluation",
        )
        .declare_capability(
            id.clone(),
            CapabilityKind::Provider,
            "answers from the delivered memory context only",
        )
        .map_err(|error| ProviderError::BadResponse {
            provider: READER_PROVIDER.to_string(),
            detail: error.to_string(),
        })?;
        Ok(Self {
            id,
            model: model.to_string(),
            manifest,
        })
    }

    pub fn manifest(&self) -> PluginManifest {
        self.manifest.clone()
    }
}

#[async_trait]
impl ProviderCapability for OfflineReader {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn models(&self) -> Vec<ModelDescriptor> {
        let model = ModelId::new(self.model.clone()).unwrap_or_else(|_| {
            ModelId::new("memory-longterm-offline-contract")
                .unwrap_or_else(|_| ModelId::new("offline").unwrap())
        });
        vec![ModelDescriptor::new(model, self.id.clone())]
    }

    async fn complete(
        &self,
        request: &NormalizedRequest,
    ) -> Result<NormalizedResponse, ProviderError> {
        let content = if is_question(request) {
            let answer = answer_of(request);
            serde_json::to_string(&answer).map_err(|error| ProviderError::BadResponse {
                provider: READER_PROVIDER.to_string(),
                detail: error.to_string(),
            })?
        } else {
            ACK.to_string()
        };
        Ok(NormalizedResponse {
            id: "offline-reader-0001".to_string(),
            model: self.model.clone(),
            content,
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::default(),
            tool_calls: Vec::new(),
            raw_metadata: serde_json::Map::new(),
        })
    }
}
