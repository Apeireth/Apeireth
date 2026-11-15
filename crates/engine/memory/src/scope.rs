//! Canonical Memory Plane scope, provenance, ranking, and provider contracts.
//!
//! These types are deliberately independent of Runtime, providers, and
//! credentials. Assembly owns concrete adapters; Memory owns the semantics.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::sync::Mutex;

use apeireth_core::kernel::Timestamp;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::layers::MemoryRecallQuery;

/// Visibility boundary for a memory record.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum MemoryScope {
    /// Shared across all users and personas in the installation.
    Global,
    /// Visible to one user.
    User { user_id: String },
    /// Visible to one persona for one user.
    Persona { persona_id: String, user_id: String },
    /// Visible to one project.
    Project { project_id: String },
    /// Visible only to one session.
    Session { session_id: String },
}

/// Query specification for storage backends that can retrieve candidate episodes
/// according to explicit visibility scopes, rather than restricting only to
/// the active session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryCandidateQuery {
    pub source_session: Option<String>,
    pub visible_scopes: Vec<MemoryScope>,
    pub limit: usize,
    pub as_of_ms: Option<i64>,
    /// **相关词**（可选）：候选发现的「相关度切片」用。
    ///
    /// # 为什么需要它（长历史关键事实）
    ///
    /// 候选阶段若只按**最新时间**截断 ⇒ ⇒ 早期确立的关键事实（姓名 / 住址 /
    /// 长期约定）在进入正式排序与评分**之前**就被近期记录挤掉
    /// ⇒ ⇒ 后面的检索层再强也救不回来（它根本不在候选池里）。
    /// ⇒ ⇒ 调用方把问句的**相关词**带下来 ⇒ ⇒ 存储层可以在**同一个预算**内
    /// 额外给「按词命中的老记录」留一条切片。
    ///
    /// # 口径
    ///
    /// * **空 = 只要新鲜度切片** ⇒ ⇒ 未接相关度切片的后端**行为不变**；
    /// * 词条由 [`relevance_terms`] 产出（用检索层同一个分词器 ⇒ ⇒ 词是正文
    ///   里真实出现的子串 ⇒ ⇒ 子串命中才有意义）；
    /// * 这条切片只是**候选前置筛**，不是最终排序口径 ⇒ ⇒ 最终排序仍由检索层
    ///   自己的覆盖度与权重决定；
    /// * 切片条数由 [`MemoryCandidateQuery::relevance_slice_limit`] 决定
    ///   （**绝对上限**，不是无界放大）。
    pub query_terms: Vec<String>,
}

/// **相关度切片**的条数上限（行）。
///
/// # 为什么是绝对上限
///
/// 候选池的总量仍由 [`MemoryCandidateQuery::limit`] 兜住 ⇒ ⇒ 相关度切片
/// 只是**占用**预算里的位置，而不是**追加**预算 ⇒ ⇒ 读取量与时延有界。
pub const CANDIDATE_RELEVANCE_SLICE_MAX: usize = 16;

/// **相关词**的条数上限（词）。
///
/// # 为什么有上限
///
/// 相关词会进存储层的排序表达式 ⇒ ⇒ 条数越多，单行打分层越贵
/// ⇒ ⇒ 固定上限 ⇒ ⇒ 成本可预期。
pub const CANDIDATE_RELEVANCE_MAX_TERMS: usize = 16;

/// 从问句里抽**相关词**（候选发现的相关度切片用）。
///
/// # 口径（用检索层同一个分词器 ⇒ ⇒ 词是正文里真实出现的子串）
///
/// * **≥2 字符**的词才留（单字噪声太大）；
/// * 中文按**二字组**（避免单字命中一大片），ASCII 词整体保留（已小写）；
/// * 去重且保持问句顺序；
/// * 超过 [`CANDIDATE_RELEVANCE_MAX_TERMS`] ⇒ ⇒ **等距抽样**（覆盖整句，
///   不是只留开头那截 —— 只留开头会让长问句后半段的关键事实捞不回来）。
#[must_use]
pub fn relevance_terms(query: &str) -> Vec<String> {
    let mut terms: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for token in crate::hybrid_search::tokenize(query) {
        let token = token.trim();
        if token.chars().count() < 2 {
            continue;
        }
        if !seen.insert(token.to_string()) {
            continue;
        }
        terms.push(token.to_string());
    }
    if terms.len() <= CANDIDATE_RELEVANCE_MAX_TERMS {
        return terms;
    }
    let total = terms.len();
    let cap = CANDIDATE_RELEVANCE_MAX_TERMS;
    (0..cap)
        .map(|slot| terms[slot * total / cap].clone())
        .collect()
}

/// 候选发现查询的派生口径。
impl MemoryCandidateQuery {
    /// 由**召回查询**构造候选查询（候选预算与相关词口径的唯一真源）。
    ///
    /// # 为什么放在这里
    ///
    /// 候选预算与相关词口径是**存储层候选发现**的契约 ⇒ ⇒ 与
    /// [`MemoryCandidateQuery`] 住在同一个模块 ⇒ ⇒ 调用方不再各写一遍
    /// （三处后端共用这一份口径）。
    ///
    /// # 口径
    ///
    /// * **候选预算** = `limit × 8`（下限 64）⇒ ⇒ 存储层在**同一个预算**内给
    ///   「新鲜度切片」与「相关度切片」分位置 ⇒ ⇒ 候选池总量不放大；
    /// * **相关词** = 问句经 [`relevance_terms`] 抽出的词 ⇒ ⇒ 空 ⇒ ⇒ 只有
    ///   新鲜度切片（未接相关度切片的后端行为不变）。
    #[must_use]
    pub fn for_recall(query: &MemoryRecallQuery) -> Self {
        Self {
            source_session: Some(query.session_id.clone()),
            visible_scopes: query.visible_scopes.clone(),
            limit: (query.limit * 8).max(64),
            as_of_ms: query.as_of_ms,
            query_terms: relevance_terms(&query.query_text),
        }
    }

    /// **相关度切片**能占用的条数（没有相关词 ⇒ 0 ⇒ 只有新鲜度切片）。
    ///
    /// # 上界从哪来
    ///
    /// `min(绝对上限, 本次候选预算)` ⇒ ⇒ 候选池**总量**仍由 `limit` 兜住
    /// ⇒ ⇒ 成本与时延不会因为这条切片而放大。
    #[must_use]
    pub fn relevance_slice_limit(&self) -> usize {
        if self.query_terms.is_empty() {
            return 0;
        }
        CANDIDATE_RELEVANCE_SLICE_MAX.min(self.limit)
    }
}

/// A storage backend capable of evaluating candidate visibility across scopes.
///
/// This moves candidate generation from `WHERE session_id = ?` into a true
/// scope-aware query at the storage layer.
pub trait ScopedMemoryBackend: Send + Sync {
    fn query_candidates(
        &self,
        query: &MemoryCandidateQuery,
    ) -> Result<Vec<apeireth_core::Episode>, Box<dyn std::error::Error + Send + Sync>>;
}

impl MemoryScope {
    /// A conservative default for legacy session-linked episodes.
    pub fn session(session_id: impl Into<String>) -> Self {
        Self::Session {
            session_id: session_id.into(),
        }
    }

    /// Stable display label for projections and diagnostics.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::User { .. } => "user",
            Self::Persona { .. } => "persona",
            Self::Project { .. } => "project",
            Self::Session { .. } => "session",
        }
    }

    /// Whether this record is visible through the supplied explicit scope.
    /// Matching is exact for scoped identities; Global is only visible when
    /// Global is explicitly present in the query.
    pub fn is_visible_in(&self, visible_scopes: &[Self]) -> bool {
        visible_scopes.iter().any(|visible| match (self, visible) {
            (Self::Global, Self::Global) => true,
            (Self::User { user_id: a }, Self::User { user_id: b }) => a == b,
            (
                Self::Persona {
                    persona_id: a_persona,
                    user_id: a_user,
                },
                Self::Persona {
                    persona_id: b_persona,
                    user_id: b_user,
                },
            ) => a_persona == b_persona && a_user == b_user,
            (Self::Project { project_id: a }, Self::Project { project_id: b }) => a == b,
            (Self::Session { session_id: a }, Self::Session { session_id: b }) => a == b,
            _ => false,
        })
    }
}

impl fmt::Display for MemoryScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Global => f.write_str("global"),
            Self::User { user_id } => write!(f, "user:{user_id}"),
            Self::Persona {
                persona_id,
                user_id,
            } => write!(f, "persona:{persona_id}@{user_id}"),
            Self::Project { project_id } => write!(f, "project:{project_id}"),
            Self::Session { session_id } => write!(f, "session:{session_id}"),
        }
    }
}

/// Safe provenance attached to every extracted or written memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProvenance {
    pub source: String,
    pub source_session: Option<String>,
    pub source_trace: Option<String>,
    pub source_request: Option<String>,
}

impl Default for MemoryProvenance {
    fn default() -> Self {
        Self {
            source: "runtime".to_string(),
            source_session: None,
            source_trace: None,
            source_request: None,
        }
    }
}

/// Explainable components of a deterministic retrieval score.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoreComponents {
    pub semantic: f64,
    pub lexical: f64,
    pub importance: f64,
    pub recency: f64,
    pub activation: f64,
    pub continuity: f64,
    pub confidence: f64,
    /// Relevance supplied by the knowledge graph or relational layer.
    #[serde(default)]
    pub graph: f64,
    /// A residual/coverage signal favouring information not already represented.
    #[serde(default)]
    pub novelty: f64,
}

impl ScoreComponents {
    pub fn weighted(self, config: &MemoryRankingConfig) -> f64 {
        self.semantic * config.semantic_weight
            + self.lexical * config.lexical_weight
            + self.importance * config.importance_weight
            + self.recency * config.recency_weight
            + self.activation * config.activation_weight
            + self.continuity * config.continuity_weight
            + self.confidence * config.confidence_weight
            + self.graph * config.graph_weight
            + self.novelty * config.novelty_weight
    }
}

/// Centralized ranking weights. No retrieval stage should own magic weights.
///
/// `diversity_lambda` controls the MMR selection stage: zero = relevance-only selection, one = diversity-only.
/// ⭐ Default 0.0 (2026-11-07, measured): a positive lambda lowered recall/nDCG on internal corpora (see `examples/retrieval_diversity_ab.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryRankingConfig {
    pub semantic_weight: f64,
    pub lexical_weight: f64,
    pub importance_weight: f64,
    pub recency_weight: f64,
    pub activation_weight: f64,
    pub continuity_weight: f64,
    pub confidence_weight: f64,
    pub graph_weight: f64,
    pub novelty_weight: f64,
    pub diversity_lambda: f64,
    /// ⭐⭐ **首命中位置**加成强度（`0` = 关，默认关）。
    ///
    /// # 为什么放在**排序配置**里
    /// 它是**管线的重排强度**（像 `diversity_lambda` 一样）⇒ ⇒ 归一化权重集中在一处
    /// ⇒ ⇒ 调权引擎算生效权重时也能原样带上（见 `retrieval_tuning::ranking`）。
    ///
    /// # 实测（**两把尺子结论相反 ⇒ 默认关**）
    /// * 真实查询卷（真人问句，同口径）：α=0.10 ⇒ recall **+0.0192**（0.7246 → 0.7438）；
    /// * 内部卷（模板题，同口径）：α=0.10 ⇒ recall **−0.0431 / −0.0282**。
    /// * ⚠️ **此前记的「+0.1227」是混口径减法（无效），已撤回**（见
    ///   `retrieval_pipeline/first_pos.rs` 文件头与 `examples/first_pos_arms.rs`）。
    ///
    /// ⇒ ⇒ 打开它的正当理由是「**你在自己的语料上量过**」，不是「文档说它更好」。
    pub first_pos_alpha: f64,
    /// ⭐⭐ **候选字符预算的「单条上限」**（计费口径 = **选后长度**）。
    ///
    /// # 为什么要有它（2026-11-08：修一个「静默全失」）
    /// 候选预算过去按**原始正文长度**计费 ⇒ ⇒ 一条 7,400 字的记录在 4,000 字预算下
    /// **一条都装不下** ⇒ ⇒ 整批候选被跳过 ⇒ ⇒ **不报错、不告警、模型一条也收不到**
    /// （静默全失）；而**注入侧**按「选段 ≤ 单条上限」交付 ⇒ ⇒ **两侧口径不一致**。
    ///
    /// # 口径
    /// * 计费 = `min(正文长度, 本字段)` ⇒ ⇒ **与交付侧同一口径**（交付侧也按本上限裁）；
    /// * `0` ⇒ **不限**（按原始正文计费 = 旧口径，留作逃生阀）；
    /// * 默认值 = **单一真源**
    ///   [`crate::anchor_injection::AnchoredInjectionConfig::PRODUCTION`] 的 `span_chars`
    ///   （**不写第二份 400**）。
    pub candidate_span_chars: usize,
}

impl Default for MemoryRankingConfig {
    fn default() -> Self {
        Self {
            semantic_weight: 0.25,
            lexical_weight: 0.20,
            importance_weight: 0.15,
            recency_weight: 0.10,
            activation_weight: 0.10,
            continuity_weight: 0.05,
            confidence_weight: 0.05,
            graph_weight: 0.05,
            novelty_weight: 0.05,
            diversity_lambda: 0.0,
            first_pos_alpha: 0.0,
            // 单条上限取**单一真源**（注入侧的生产跨度）⇒ ⇒ 不写第二份 200。
            candidate_span_chars: crate::anchor_injection::AnchoredInjectionConfig::PRODUCTION
                .span_chars,
        }
    }
}

/// Structured, serializable policy shared by callers performing memory recall.
///
/// The empty default visibility set is intentional: callers must opt into the
/// scopes they want to expose (closed-world recall).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RecallPolicy {
    pub visible_scopes: Vec<MemoryScope>,
    pub layers: Vec<crate::layers::MemoryLayerKind>,
    pub limit: usize,
    pub max_chars: usize,
    pub proactive_budget: usize,
    pub min_score: f64,
    pub as_of_ms: Option<i64>,
    pub token_budget: usize,
}

impl Default for RecallPolicy {
    fn default() -> Self {
        Self {
            visible_scopes: Vec::new(),
            layers: vec![
                crate::layers::MemoryLayerKind::Working,
                crate::layers::MemoryLayerKind::Episodic,
                crate::layers::MemoryLayerKind::Semantic,
                crate::layers::MemoryLayerKind::Relational,
            ],
            // ⭐ **默认名额 20**（2026-11-09 联合裁决，与 `MemoryRecallQuery::new` 同口径）。
            limit: crate::layers::PRODUCTION_RECALL_LIMIT,
            max_chars: 4_000,
            proactive_budget: 2,
            min_score: 0.10,
            as_of_ms: None,
            token_budget: 1_024,
        }
    }
}

impl RecallPolicy {
    /// Construct a policy with the supplied visibility boundary.
    #[must_use]
    pub fn new(visible_scopes: Vec<MemoryScope>) -> Self {
        Self {
            visible_scopes,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_visible_scopes(mut self, visible_scopes: Vec<MemoryScope>) -> Self {
        self.visible_scopes = visible_scopes;
        self
    }

    #[must_use]
    pub fn with_visible_scope(mut self, scope: MemoryScope) -> Self {
        if !self.visible_scopes.contains(&scope) {
            self.visible_scopes.push(scope);
        }
        self
    }

    #[must_use]
    pub fn with_layers(mut self, layers: Vec<crate::layers::MemoryLayerKind>) -> Self {
        self.layers = layers;
        self
    }

    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit.max(1);
        self
    }

    #[must_use]
    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars.max(128);
        self
    }
    #[must_use]
    pub fn with_min_score(mut self, min_score: f64) -> Self {
        self.min_score = min_score;
        self
    }

    #[must_use]
    pub fn with_as_of_ms(mut self, as_of_ms: i64) -> Self {
        self.as_of_ms = Some(as_of_ms);
        self
    }
    #[must_use]
    pub fn with_token_budget(mut self, token_budget: usize) -> Self {
        self.token_budget = token_budget.max(1);
        self
    }
}

/// A retrieval candidate with enough safe metadata to explain its ranking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCandidate {
    pub id: String,
    pub layer: String,
    pub scope: MemoryScope,
    pub content: String,
    pub score: f64,
    pub score_components: ScoreComponents,
    pub provenance: MemoryProvenance,
}

/// Explicit principal identity used by typed durable-memory recall.
/// A session identifier is deliberately not a substitute for either field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedRecallIdentity {
    pub persona_id: String,
    pub subject_id: String,
}

/// Provider-neutral source of typed durable-memory candidates.
/// Implementations return candidates for the coordinator's existing ranking path.
pub trait TypedMemoryRecallSource: Send + Sync {
    fn candidates(
        &self,
        query: &crate::layers::MemoryRecallQuery,
        identity: &TypedRecallIdentity,
        now_ms: i64,
    ) -> Result<Vec<MemoryCandidate>, Box<dyn std::error::Error + Send + Sync>>;
}

/// Embedding adapter boundary. Memory never constructs HTTP clients or reads
/// credentials; Assembly injects an implementation when semantic recall is
/// available.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError>;
    fn model_id(&self) -> &str {
        "unknown"
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmbeddingError {
    #[error("embedding provider unavailable: {0}")]
    Unavailable(String),
    #[error("embedding provider returned invalid vector: {0}")]
    InvalidVector(String),
}

/// Explicit no-op semantic provider. Lexical retrieval remains available.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for NoEmbeddingProvider {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Err(EmbeddingError::Unavailable("not configured".to_string()))
    }
}

/// Optional second-stage reranking adapter.
#[async_trait]
pub trait MemoryReranker: Send + Sync {
    async fn rerank(
        &self,
        query: &str,
        candidates: Vec<MemoryCandidate>,
        token_budget: usize,
    ) -> Vec<MemoryCandidate>;
}

/// Deterministic default reranker; it preserves the already computed order.
#[derive(Debug, Default, Clone, Copy)]
pub struct DeterministicReranker;

#[async_trait]
impl MemoryReranker for DeterministicReranker {
    async fn rerank(
        &self,
        _query: &str,
        candidates: Vec<MemoryCandidate>,
        _token_budget: usize,
    ) -> Vec<MemoryCandidate> {
        candidates
    }
}

/// Persona profile kept separate from generic memory records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonaMemoryProfile {
    pub persona_id: String,
    pub subject_id: String,
    pub portrait: String,
    pub traits: Vec<String>,
    pub known_facts: Vec<String>,
    pub shared_experiences: Vec<String>,
    pub revision: u64,
    pub provenance: MemoryProvenance,
    pub updated_at: Timestamp,
}

/// Optimistic, provenance-bearing profile patch. It never replaces the whole
/// profile supplied by a model.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersonaProfileDelta {
    pub portrait_replace: Option<String>,
    pub traits_add: Vec<String>,
    pub traits_remove: Vec<String>,
    pub facts_add: Vec<String>,
    pub facts_update: Vec<(String, String)>,
    pub experiences_add: Vec<String>,
    pub provenance: MemoryProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaGovernanceState {
    pub tombstoned_at_ms: Option<i64>,
    pub protected: bool,
    pub revision: u64,
}

#[async_trait]
pub trait PersonaProfileStore: Send + Sync {
    fn supports_recovery_journal(
        &self,
        journal: &crate::writeback_recovery::SqliteWritebackJournal,
    ) -> bool {
        let _ = journal;
        false
    }
    async fn get_profile(
        &self,
        persona_id: &str,
        subject_id: &str,
    ) -> Result<Option<PersonaMemoryProfile>, String>;

    async fn governance(
        &self,
        persona_id: &str,
        subject_id: &str,
    ) -> Result<PersonaGovernanceState, String> {
        let _ = (persona_id, subject_id);
        Ok(PersonaGovernanceState {
            tombstoned_at_ms: None,
            protected: false,
            revision: 0,
        })
    }

    async fn tombstone(
        &self,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        at_ms: i64,
        reason: Option<&str>,
    ) -> Result<PersonaGovernanceState, String> {
        let _ = (persona_id, subject_id, expected_revision, at_ms, reason);
        Err("persona governance is not configured".into())
    }

    async fn set_protected(
        &self,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        protected: bool,
        at_ms: i64,
        reason: Option<&str>,
    ) -> Result<PersonaGovernanceState, String> {
        let _ = (
            persona_id,
            subject_id,
            expected_revision,
            protected,
            at_ms,
            reason,
        );
        Err("persona governance is not configured".into())
    }

    async fn apply_delta(
        &self,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        delta: PersonaProfileDelta,
        updated_at: Timestamp,
    ) -> Result<PersonaMemoryProfile, String>;

    /// Durable implementations atomically retain the operation receipt with
    /// the profile mutation. Compatibility stores must not pretend replay safety.
    async fn apply_delta_once(
        &self,
        operation_id: &str,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        delta: PersonaProfileDelta,
        updated_at: Timestamp,
    ) -> Result<PersonaMemoryProfile, String> {
        let _ = (
            operation_id,
            persona_id,
            subject_id,
            expected_revision,
            delta,
            updated_at,
        );
        Err("durable persona recovery is not supported".into())
    }
}

/// Process-local profile store used for deterministic tests and embeddings.
/// Production callers should inject a durable implementation.
#[derive(Debug, Default)]
pub struct InMemoryPersonaProfileStore {
    profiles: Mutex<BTreeMap<(String, String), PersonaMemoryProfile>>,
}

#[async_trait]
impl PersonaProfileStore for InMemoryPersonaProfileStore {
    async fn get_profile(
        &self,
        persona_id: &str,
        subject_id: &str,
    ) -> Result<Option<PersonaMemoryProfile>, String> {
        Ok(self
            .profiles
            .lock()
            .map_err(|_| "persona profile store poisoned".to_string())?
            .get(&(persona_id.to_string(), subject_id.to_string()))
            .cloned())
    }

    async fn apply_delta(
        &self,
        persona_id: &str,
        subject_id: &str,
        expected_revision: u64,
        delta: PersonaProfileDelta,
        updated_at: Timestamp,
    ) -> Result<PersonaMemoryProfile, String> {
        let mut profiles = self
            .profiles
            .lock()
            .map_err(|_| "persona profile store poisoned".to_string())?;
        let key = (persona_id.to_string(), subject_id.to_string());
        let profile = profiles.entry(key).or_insert_with(|| PersonaMemoryProfile {
            persona_id: persona_id.to_string(),
            subject_id: subject_id.to_string(),
            portrait: String::new(),
            traits: Vec::new(),
            known_facts: Vec::new(),
            shared_experiences: Vec::new(),
            revision: 0,
            provenance: MemoryProvenance::default(),
            updated_at,
        });
        profile.apply_delta(&delta, expected_revision, updated_at)?;
        Ok(profile.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⭐⭐ **`diversity_lambda` 的默认值必须保持 0.0**（2026-11-07 实测钉死）。
    ///
    /// # 为什么钉
    /// MMR 折中按设计就是「用相关度换新颖度」。内部两卷的 λ 扫描是**单调下降**：
    /// redchamber recall@10 `λ=0: 0.4010` → `λ=0.2: 0.3802`（相对 **−5.19%**）；
    /// watershed `0.7240` → `0.7031`（**−2.89%**）；`λ=1` 灾难性（−59.7% / −72.7%）。
    /// 本仓产品口径以**相关/送达**为主指标 ⇒ 默认不能自损。
    /// 需要「去冗余展示」的调用方**显式**给 `λ > 0`（这是产品选择，不是默认）。
    /// ⇒ ⇒ 复制口径：`examples/retrieval_diversity_ab.rs`（λ 扫描入口）。
    #[test]
    fn diversity_lambda_defaults_to_zero() {
        assert_eq!(
            MemoryRankingConfig::default().diversity_lambda,
            0.0,
            "MMR 折中默认必须关：内部两卷 λ=0.2 相对 recall 掉 5.19% / 2.89%（见 examples/retrieval_diversity_ab.rs）"
        );
    }

    #[test]
    fn weighted_includes_graph_and_novelty_components() {
        let components = ScoreComponents {
            graph: 2.0,
            novelty: 3.0,
            ..ScoreComponents::default()
        };
        let config = MemoryRankingConfig {
            graph_weight: 0.25,
            novelty_weight: 0.50,
            ..MemoryRankingConfig::default()
        };

        assert!((components.weighted(&config) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn recall_policy_defaults_are_bounded_and_structured() {
        let policy = RecallPolicy::default();

        assert!(policy.visible_scopes.is_empty());
        assert_eq!(policy.layers.len(), 4);
        // ⭐ 名额默认 20（2026-11-09 联合裁决：有效名额 = min(k, floor((预算 − 160)/(跨度 + 8)))
        // ⇒ 跨度 200 时容量 = 20 ⇒ ⇒ k=8/10 会让容量封顶 ⇒ ⇒ 名额给到 20 才吃得到容量）。
        assert_eq!(policy.limit, 20);
        assert_eq!(policy.max_chars, 4_000);
        assert_eq!(policy.proactive_budget, 2);
        assert_eq!(policy.min_score, 0.10);
        assert_eq!(policy.as_of_ms, None);
        assert_eq!(policy.token_budget, 1_024);
    }

    #[test]
    fn recall_policy_builders_clamp_hard_budgets() {
        let policy = RecallPolicy::default()
            .with_limit(0)
            .with_max_chars(0)
            .with_token_budget(0)
            .with_min_score(0.4)
            .with_as_of_ms(123);

        assert_eq!(policy.limit, 1);
        assert_eq!(policy.max_chars, 128);
        assert_eq!(policy.token_budget, 1);
        assert_eq!(policy.min_score, 0.4);
        assert_eq!(policy.as_of_ms, Some(123));
    }
}

impl PersonaMemoryProfile {
    /// Apply a delta only when its expected revision still matches.
    pub fn apply_delta(
        &mut self,
        delta: &PersonaProfileDelta,
        expected_revision: u64,
        updated_at: Timestamp,
    ) -> Result<(), String> {
        if self.revision != expected_revision {
            return Err(format!(
                "persona profile revision conflict: expected {expected_revision}, current {}",
                self.revision
            ));
        }
        if let Some(portrait) = &delta.portrait_replace {
            self.portrait = portrait.clone();
        }
        for trait_name in &delta.traits_remove {
            self.traits.retain(|value| value != trait_name);
        }
        for trait_name in &delta.traits_add {
            if !self.traits.contains(trait_name) {
                self.traits.push(trait_name.clone());
            }
        }
        for fact in &delta.facts_add {
            if !self.known_facts.contains(fact) {
                self.known_facts.push(fact.clone());
            }
        }
        for (old, new) in &delta.facts_update {
            if let Some(fact) = self.known_facts.iter_mut().find(|fact| *fact == old) {
                *fact = new.clone();
            } else if !self.known_facts.contains(new) {
                self.known_facts.push(new.clone());
            }
        }
        for experience in &delta.experiences_add {
            if !self.shared_experiences.contains(experience) {
                self.shared_experiences.push(experience.clone());
            }
        }
        self.revision = self.revision.saturating_add(1);
        self.provenance = delta.provenance.clone();
        self.updated_at = updated_at;
        Ok(())
    }
}
