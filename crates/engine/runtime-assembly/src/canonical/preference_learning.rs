//! Deterministic preference learning (the `cognitive.preference_learning`
//! slot, previously deferred).
//!
//! Closes the first real learning loop:
//!
//! ```text
//! turn N user text (evidence)
//!   → AfterTurn → PreferenceLearningModule (deterministic extractor)
//!   → existing PreferenceStore
//!   → turn N+1 → existing PreferenceRecallModule overlay
//! ```
//!
//! # Donor truth, honored
//!
//! The v1 donor (`apeireth-companion::memory_extractor`) classified
//! preferences with an LLM extractor and wrote/injected them through a
//! deterministic mechanism whose `preference_injection()` applied the
//! preference profile across scenarios unconditionally. This first v2
//! implementation inverts the split: **explicit preference statements are
//! extracted deterministically** (no model call at all — the closed loop runs
//! with zero provider side-calls), the write path reuses the existing
//! `PreferenceStore`, and recall keeps its existing slot. The donor's
//! LLM-based semantic extraction of *implicit* preferences is deferred, not
//! faked: nothing here pretends to infer unstated preferences.
//!
//! # Evidence semantics (documented, deterministic)
//!
//! - Only the turn's last user message is scanned, per sentence/clause.
//! - Transient frames ("tonight", "today", "right now", …) and desire forms
//!   ("I want", "I need") are skipped entirely: a current wish is not a stable
//!   preference.
//! - Recognized explicit forms: "I (really) like/love/enjoy X",
//!   "I don't/do not (really) like X", "I dislike/hate X",
//!   "I prefer X (to/over Y)", "my favorite X is Y" and the Chinese
//!   equivalents 我(很/不)喜欢、我讨厌、我更喜欢、最喜欢的…是.
//! - Confidence is an honest bounded scale, not Bayesian: explicit 0.7,
//!   intensified ("really"/"love"/…) 0.8, comparative 0.8.
//! - Rows are keyed by `SHA-256(session_id:topic)[:16]` (the documented v1
//!   derivation); the store's PK upsert therefore reinforces instead of
//!   duplicating. Re-observation keeps the first `created_at`, takes the max
//!   confidence, and refreshes the stance/evidence to the latest wording.
//!   Contradictory evidence follows the same rule: the latest observation
//!   defines the stance/polarity; confidence still never decreases.
//! - Ownership scope: rows are session-scoped (`UserPreference.session_id`);
//!   the current store has no cross-session/user profile identity, and this
//!   module invents none.
//!
//! # Authority
//!
//! The module never stops, retries, or amends a committed turn; it returns
//! `Continue` from every hook and holds no invoker, factory, router,
//! governance hook, or session store. Write failures are counted and
//! observable, and the hook stays fail-open (MemoryWriteback precedent).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use apeireth_core::kernel::SessionId;
use apeireth_plugin::preference::{PreferenceStore, UserPreference};
use apeireth_protocol::canonical::{ContentPart, MessageRole};
use async_trait::async_trait;
use sha2::{Digest, Sha256};

use super::cognitive::CognitiveTelemetry;
use super::module::{
    AgentModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
};

/// Stable slot id (the ledger's canonical deferred id, now implemented).
pub const PREFERENCE_LEARNING_MODULE_ID: &str = "cognitive.preference_learning";

/// Sentence/clause splitting: punctuation, then coordinating conjunctions.
const SENTENCE_SEPARATORS: &[char] = &['.', '!', '?', ';', '\n', '。', '！', '？', '；', '，', ','];

/// Frames that make a statement temporary rather than a stable preference.
const TRANSIENT_MARKERS: &[&str] = &[
    "tonight",
    "today",
    "right now",
    "this evening",
    "this week",
    "this morning",
    "currently",
    "for now",
    "今晚",
    "今天",
    "现在",
    "此刻",
    "这几天",
    "目前",
];

/// Desire/request openings: a current wish, not a stable preference.
const DESIRE_MARKERS: &[&str] = &[
    "i want",
    "i need",
    "i'd like",
    "i would like",
    "let's",
    "please",
    "我想",
    "我想要",
    "我需要",
    "我想吃",
    "我想喝",
    "请帮我",
];

/// The polarity of one piece of extracted evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreferencePolarity {
    /// "I like Rust"
    Positive,
    /// "I don't like Python"
    Negative,
}

/// One deterministic extraction from a committed user message.
#[derive(Debug, Clone, PartialEq)]
pub struct PreferenceEvidence {
    /// Normalized subject (the store's topic key).
    pub topic: String,
    /// Human-readable stance ("likes rust", "prefers rust over python").
    pub stance: String,
    pub polarity: PreferencePolarity,
    /// Honest bounded confidence: 0.7 explicit, 0.8 intensified/comparative.
    pub confidence: f64,
    /// "explicit" plus "negative"/"comparison" where applicable.
    pub tags: Vec<String>,
}

/// Non-sensitive learning counters (no preference contents).
#[derive(Debug, Default)]
pub struct PreferenceLearningStats {
    pub hooks_run: AtomicU64,
    pub evidence_extracted: AtomicU64,
    pub preferences_written: AtomicU64,
    pub write_failures: AtomicU64,
}

impl PreferenceLearningStats {
    /// Point-in-time snapshot.
    pub fn snapshot(&self) -> (u64, u64, u64, u64) {
        (
            self.hooks_run.load(Ordering::Relaxed),
            self.evidence_extracted.load(Ordering::Relaxed),
            self.preferences_written.load(Ordering::Relaxed),
            self.write_failures.load(Ordering::Relaxed),
        )
    }
}

/// The ONE preference-learning module: deterministic extraction at AfterTurn
/// into the existing `PreferenceStore`.
pub struct PreferenceLearningModule {
    manifest: ModuleManifest,
    store: Arc<dyn PreferenceStore>,
    stats: Arc<PreferenceLearningStats>,
    telemetry: Mutex<Option<Arc<CognitiveTelemetry>>>,
}

impl PreferenceLearningModule {
    /// Build the learning slot over the existing preference store.
    pub fn new(store: Arc<dyn PreferenceStore>) -> Self {
        Self {
            manifest: ModuleManifest::new(
                PREFERENCE_LEARNING_MODULE_ID,
                "Preference learning (deterministic)",
            ),
            store,
            stats: Arc::new(PreferenceLearningStats::default()),
            telemetry: Mutex::new(None),
        }
    }

    /// Learning counters for embedding callers.
    pub fn stats(&self) -> Arc<PreferenceLearningStats> {
        Arc::clone(&self.stats)
    }

    /// Attach the shared non-sensitive telemetry sink.
    #[must_use]
    pub fn with_telemetry(self, telemetry: Arc<CognitiveTelemetry>) -> Self {
        *self.telemetry.lock().expect("telemetry mutex") = Some(telemetry);
        self
    }

    fn record_telemetry(&self, hook: HookPoint, started: Instant) {
        if let Some(telemetry) = self.telemetry.lock().expect("telemetry mutex").as_ref() {
            telemetry.record(super::cognitive::CognitiveModuleEvent {
                module_id: PREFERENCE_LEARNING_MODULE_ID.to_string(),
                hook: format!("{hook:?}"),
                directive: "continue".to_string(),
                duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
                side_calls: 0,
            });
        }
    }

    /// Learn from one committed turn: extract, then write with
    /// reinforce/contradiction semantics. Fail-open.
    fn learn(&self, session_id: &SessionId, user_text: &str, turn_ref: &str) {
        let evidence = extract_evidence(user_text);
        self.stats
            .evidence_extracted
            .fetch_add(evidence.len() as u64, Ordering::Relaxed);
        if evidence.is_empty() {
            return;
        }
        let Ok(existing) = self.store.list_for_session(session_id) else {
            self.stats.write_failures.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let now_ms = now_millis();
        for item in evidence {
            let row = build_row(session_id, &item, turn_ref, now_ms, &existing);
            match self.store.record(&row) {
                Ok(()) => {
                    self.stats
                        .preferences_written
                        .fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    self.stats.write_failures.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}

#[async_trait]
impl AgentModule for PreferenceLearningModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        let started = Instant::now();
        if hook == HookPoint::AfterTurn {
            // Evidence source: the committed turn's own user message, taken
            // from the canonical context (no second transcript, no history
            // reconstruction).
            let user_text = ctx
                .messages
                .iter()
                .rev()
                .find(|message| message.role == MessageRole::User)
                .map(|message| ContentPart::join_text(&message.content))
                .unwrap_or_default();
            let turn_ref = format!("turn:{}", ctx.session_id);
            self.learn(ctx.session_id, &user_text, &turn_ref);
            self.stats.hooks_run.fetch_add(1, Ordering::Relaxed);
            self.record_telemetry(hook, started);
        }
        // Fail-open, post-commit: learning never amends the committed reply.
        Ok(ModuleOutcome::continue_())
    }
}

// ---------------------------------------------------------------------
// Deterministic extractor
// ---------------------------------------------------------------------

/// Extract explicit preference evidence from a user message.
///
/// Bounded and testable: sentence/clause split, a fixed pattern list, no NLP
/// grammar, no model. First matching pattern wins per clause.
pub fn extract_evidence(user_text: &str) -> Vec<PreferenceEvidence> {
    let mut evidence = Vec::new();
    for sentence in split_sentences(user_text) {
        if TRANSIENT_MARKERS
            .iter()
            .any(|marker| sentence.contains(marker))
        {
            continue;
        }
        if DESIRE_MARKERS
            .iter()
            .any(|marker| sentence.contains(marker))
        {
            continue;
        }
        // Intensifiers boost confidence but must not break pattern matching.
        let boosted = sentence.contains("really")
            || sentence.contains("absolutely")
            || sentence.contains('很')
            || sentence.contains('超')
            || sentence.contains("love")
            || sentence.contains("爱");
        let working = sentence
            .replace("really ", "")
            .replace("absolutely ", "")
            .replace('很', "")
            .replace("  ", " ");

        if let Some(item) = match_comparison(&working, boosted)
            .or_else(|| match_favorite(&working, boosted))
            .or_else(|| match_negative(&working, boosted))
            .or_else(|| match_positive(&working, boosted))
        {
            evidence.push(item);
        }
    }
    evidence
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    for sentence in text.split(SENTENCE_SEPARATORS) {
        let mut queue = vec![sentence.trim().to_string()];
        while let Some(current) = queue.pop() {
            // Split on coordinating conjunctions once per level (no duplicate
            // whole-sentence re-matches).
            if let Some(position) = current.find(" and ") {
                queue.push(current[..position].trim().to_string());
                queue.push(current[position + 5..].trim().to_string());
                continue;
            }
            if let Some(position) = current.find(" but ") {
                queue.push(current[..position].trim().to_string());
                queue.push(current[position + 5..].trim().to_string());
                continue;
            }
            if !current.trim().is_empty() {
                clauses.push(current.trim().to_lowercase());
            }
        }
    }
    clauses
}

fn match_comparison(clause: &str, _boosted: bool) -> Option<PreferenceEvidence> {
    [" to ", " over ", "而不是", "而非", "胜过"]
        .iter()
        .find_map(|separator| comparison_for(clause, separator))
        .or_else(|| {
            // "i prefer X" / "我更喜欢X" without an explicit alternative is
            // still an explicit pick.
            for prefix in ["i prefer ", "我更喜欢"] {
                if let Some(rest) = clause.strip_prefix(prefix) {
                    let topic = normalize_topic(rest);
                    if !topic.is_empty() {
                        return Some(PreferenceEvidence {
                            topic,
                            stance: format!("prefers {}", normalize_topic(rest)),
                            polarity: PreferencePolarity::Positive,
                            confidence: 0.8,
                            tags: vec!["explicit".into()],
                        });
                    }
                }
            }
            None
        })
}

fn comparison_for(clause: &str, separator: &str) -> Option<PreferenceEvidence> {
    let (preferred, rejected) = split_pair(clause, "i prefer ", separator)
        .or_else(|| split_pair(clause, "我更喜欢", separator))?;
    let stance = format!("prefers {preferred} over {rejected}");
    Some(PreferenceEvidence {
        topic: preferred,
        stance,
        polarity: PreferencePolarity::Positive,
        confidence: 0.8,
        tags: vec!["explicit".into(), "comparison".into()],
    })
}

fn match_favorite(clause: &str, _boosted: bool) -> Option<PreferenceEvidence> {
    [("my favorite ", " is "), ("我最喜欢的", "是")]
        .iter()
        .find_map(|(prefix, separator)| favorite_for(clause, prefix, separator))
}

fn favorite_for(clause: &str, prefix: &str, separator: &str) -> Option<PreferenceEvidence> {
    let (subject, value) = split_pair(clause, prefix, separator)?;
    Some(PreferenceEvidence {
        topic: format!("favorite {subject}"),
        stance: format!("favorite {subject} is {value}"),
        polarity: PreferencePolarity::Positive,
        confidence: 0.8,
        tags: vec!["explicit".into(), "favorite".into()],
    })
}

fn match_negative(clause: &str, boosted: bool) -> Option<PreferenceEvidence> {
    [
        "i don't like ",
        "i do not like ",
        "i don't enjoy ",
        "i dislike ",
        "i hate ",
        "i can't stand ",
        "我不喜欢",
        "我讨厌",
    ]
    .iter()
    .find_map(|prefix| {
        let rest = clause.strip_prefix(prefix)?;
        let base: f64 = if clause.contains("hate") || clause.contains("讨厌") {
            0.8
        } else {
            0.7
        };
        Some(PreferenceEvidence {
            topic: normalize_topic(rest),
            stance: format!("dislikes {}", normalize_topic(rest)),
            polarity: PreferencePolarity::Negative,
            confidence: if boosted { (base + 0.1).min(0.9) } else { base },
            tags: vec!["explicit".into(), "negative".into()],
        })
    })
}

fn match_positive(clause: &str, boosted: bool) -> Option<PreferenceEvidence> {
    [
        "i like ",
        "i love ",
        "i enjoy ",
        "我喜欢",
        "我很喜欢",
        "我爱",
    ]
    .iter()
    .find_map(|prefix| {
        let rest = clause.strip_prefix(prefix)?;
        positive(rest, boosted)
    })
}

fn positive(rest: &str, boosted: bool) -> Option<PreferenceEvidence> {
    Some(PreferenceEvidence {
        topic: normalize_topic(rest),
        stance: format!("likes {}", normalize_topic(rest)),
        polarity: PreferencePolarity::Positive,
        confidence: if boosted { 0.8 } else { 0.7 },
        tags: vec!["explicit".into()],
    })
}

/// Split "<prefix>X<separator>Y" into normalized (X, Y); None if absent.
fn split_pair<'a>(clause: &'a str, prefix: &str, separator: &str) -> Option<(String, String)> {
    let rest = clause.strip_prefix(prefix)?;
    let position = rest.find(separator)?;
    let first = normalize_topic(&rest[..position]);
    let second = normalize_topic(&rest[position + separator.len()..]);
    if first.is_empty() || second.is_empty() {
        return None;
    }
    Some((first, second))
}

/// Normalize a captured topic: trim articles/punctuation, lowercase, bound.
fn normalize_topic(raw: &str) -> String {
    let mut topic = raw.trim().to_lowercase();
    for article in ["the ", "a ", "an "] {
        if let Some(stripped) = topic.strip_prefix(article) {
            topic = stripped.to_string();
        }
    }
    let topic = topic.trim_matches(|c: char| !c.is_alphanumeric() && !is_cjk(c));
    let topic: String = topic.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut topic: String = topic.chars().take(64).collect();
    topic = topic.trim_end().to_string();
    if topic.chars().all(|c| is_cjk(c) || c.is_alphanumeric()) && !topic.is_empty() {
        topic
    } else {
        String::new()
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF)
}

// ---------------------------------------------------------------------
// Store write semantics
// ---------------------------------------------------------------------

/// Build the row for one piece of evidence, applying the documented
/// reinforce/contradiction policy against existing rows of this session.
fn build_row(
    session_id: &SessionId,
    item: &PreferenceEvidence,
    turn_ref: &str,
    now_ms: i64,
    existing: &[UserPreference],
) -> UserPreference {
    let id = preference_id(session_id, &item.topic);
    let previous = existing
        .iter()
        .find(|row| row.id == id || normalize_topic(&row.topic) == item.topic);
    let mut evidence_refs = previous
        .map(|row| row.evidence_refs.clone())
        .unwrap_or_default();
    evidence_refs.push(turn_ref.to_string());
    evidence_refs.truncate(8);
    UserPreference {
        id,
        session_id: *session_id,
        topic: item.topic.clone(),
        stance: item.stance.clone(),
        evidence_refs,
        created_at: previous.map(|row| row.created_at).unwrap_or(now_ms),
        confidence: previous
            .map(|row| row.confidence.max(item.confidence))
            .unwrap_or(item.confidence),
        tags: item.tags.clone(),
    }
}

/// v1-documented derivation: SHA-256(session_id + ':' + topic)[:16] hex.
fn preference_id(session_id: &SessionId, topic: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(session_id.to_string().as_bytes());
    hasher.update(b":");
    hasher.update(topic.as_bytes());
    let digest = hasher.finalize();
    digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_plugin::memory_backend::CapabilityResult;
    use apeireth_plugin::preference::UserPreference;
    use std::sync::atomic::AtomicU64;

    struct MemoryPrefStore {
        rows: Mutex<Vec<UserPreference>>,
    }

    impl MemoryPrefStore {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                rows: Mutex::new(Vec::new()),
            })
        }

        fn count(&self) -> usize {
            self.rows.lock().unwrap().len()
        }
    }

    impl PreferenceStore for MemoryPrefStore {
        fn record(&self, pref: &UserPreference) -> CapabilityResult<()> {
            let mut rows = self.rows.lock().unwrap();
            rows.retain(|row| row.id != pref.id);
            rows.push(pref.clone());
            Ok(())
        }

        fn recall_for_context(
            &self,
            _session_id: &SessionId,
            _current_topic: &str,
            _limit: u32,
        ) -> CapabilityResult<Vec<UserPreference>> {
            Ok(Vec::new())
        }

        fn forget(&self, _pref_id: &str) -> CapabilityResult<()> {
            Ok(())
        }

        fn list_for_session(
            &self,
            _session_id: &SessionId,
        ) -> CapabilityResult<Vec<UserPreference>> {
            Ok(self.rows.lock().unwrap().clone())
        }
    }

    struct NullInvoker;
    #[async_trait::async_trait]
    impl super::super::module::ModuleInvoker for NullInvoker {
        async fn invoke(
            &self,
            _request: super::super::module::ModuleInvocationRequest,
        ) -> Result<
            super::super::module::ModuleInvocationResponse,
            super::super::module::ModuleInvocationError,
        > {
            Err(super::super::module::ModuleInvocationError::NoModel)
        }
    }

    struct NullSubLoop;
    #[async_trait::async_trait]
    impl super::super::subloop::SubLoopSpawner for NullSubLoop {
        async fn spawn(
            &self,
            _spec: super::super::subloop::SubLoopSpec,
        ) -> Result<super::super::subloop::SubLoopResult, super::super::subloop::SubLoopError>
        {
            Err(super::super::subloop::SubLoopError::NoModel)
        }
    }

    /// Learning runs ONLY at AfterTurn: no writes from any pre-commit hook.
    #[tokio::test]
    async fn learning_runs_only_at_afterturn() {
        let store = MemoryPrefStore::new();
        let module = PreferenceLearningModule::new(store.clone());
        let session = SessionId::new();
        let messages = vec![apeireth_protocol::canonical::NormalizedMessage::user(
            "I like Rust",
        )];
        static INVOCATION: std::sync::OnceLock<super::super::module::InvocationContext> =
            std::sync::OnceLock::new();
        static NULL_SUBLOOP: NullSubLoop = NullSubLoop;
        let invoker: Arc<dyn super::super::module::ModuleInvoker> = Arc::new(NullInvoker);
        let context = super::super::module::ModuleContext {
            session_id: &session,
            model: "test-model",
            messages: &messages,
            candidate: None,
            tool_call: None,
            tool_result: None,
            invocation: INVOCATION.get_or_init(super::super::module::InvocationContext::user_turn),
            module_id: "probe",
            error: None,
            invoker: &*invoker,
            invoker_handle: Arc::clone(&invoker),
            subloop: &NULL_SUBLOOP,
        };

        for hook in [
            HookPoint::TurnStart,
            HookPoint::BeforeModelCall,
            HookPoint::AfterModelResponse,
            HookPoint::BeforeFinalCommit,
        ] {
            module.on_hook(hook, &context).await.unwrap();
            assert_eq!(store.count(), 0, "no learning at {hook:?}");
        }
        module
            .on_hook(HookPoint::AfterTurn, &context)
            .await
            .unwrap();
        assert_eq!(store.count(), 1, "learning happens exactly at AfterTurn");
        let (hooks, evidence, written, failures) = module.stats().snapshot();
        assert_eq!((hooks, evidence, written, failures), (1, 1, 1, 0));
    }
}
