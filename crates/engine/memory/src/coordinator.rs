//! Unified Memory Coordinator (`MemoryCoordinator`).
//!
//! Serves as the central orchestrator across all 4 memory layers:
//! - Working Memory (in-memory fast ring-buffer)
//! - Episodic Memory (governed SQLite, active/forgotten status, content override)
//! - Semantic / Personal Memory (user preferences, profile cards)
//! - Relational / Temporal Memory (knowledge graph facts, entity links)

use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::SessionId;
use apeireth_orchestration::runtime_invariants::{AuditEvent, AuditEventSink};
use apeireth_plugin::experience::{AssociationStore, KnowledgeGraphStore};
use apeireth_plugin::memory_backend::{EpisodeWriteOutcome, MemoryBackend};
use apeireth_plugin::preference::PreferenceStore;

use crate::access_history::ActivationSource;
use crate::consolidation::{ConsolidationReport, MemoryConsolidationJob};
use crate::consolidation_trigger::{ConsolidationOutcome, ConsolidationTrigger};
use crate::context_compiler::{
    ClosedWorldContextCompiler, MemoryEvidenceSource, SelectedMemoryAccess,
};
use crate::continuity_state::{ContinuityCompressor, ContinuityState};
use crate::cross_diary_wiring::{self, CrossDiaryLimits};
use crate::diary::DiaryStore;
use crate::layers::{
    MemoryLayerKind, MemoryRecallQuery, MemoryRecallResult, MemoryWritebackEntry,
    RecalledMemoryItem,
};
use crate::memory_governance::{
    GovernedEpisode, MemoryGovernanceError, MemoryGovernanceStatus, MemoryGovernanceStore,
};
use crate::proactive_recall::{ProactiveRecallPolicy, ProactiveRecallService};
use crate::retrieval_pipeline::{
    diversity, Bm25LexicalCandidateSource, HybridRetrievalPipeline, MemoryCandidateSource,
    RetrievalStatus, StaticVectorCandidateSource,
};
use crate::scope::{
    EmbeddingProvider, MemoryCandidate, MemoryCandidateQuery, MemoryProvenance,
    MemoryRankingConfig, MemoryScope, ScopedMemoryBackend, ScoreComponents,
    TypedMemoryRecallSource, TypedRecallIdentity,
};
use crate::topic_predictor::TopicCue;
use crate::MemoryError;

#[path = "coordinator/owned.rs"]
mod owned;
const WORKING_RING_BUFFER_CAP: usize = 30;

/// Unified memory orchestrator coordinating all memory layers and pipelines.
pub struct MemoryCoordinator {
    backend: Arc<dyn MemoryBackend>,
    scoped_backend: Option<Arc<dyn ScopedMemoryBackend>>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    governance: Arc<dyn MemoryGovernanceStore>,
    working: Mutex<HashMap<String, VecDeque<Episode>>>,
    preferences: Option<Arc<dyn PreferenceStore>>,
    graph: Option<Arc<dyn KnowledgeGraphStore>>,
    associations: Option<Arc<dyn AssociationStore>>,
    compressor: ContinuityCompressor,
    compiler: ClosedWorldContextCompiler,
    evidence_source: Option<Arc<dyn MemoryEvidenceSource>>,
    consolidation: MemoryConsolidationJob,
    /// **可选**的夜间整编驱动（窗口水位 + 只整新料）。
    ///
    /// ⚠️ **缺省 = 无** ⇒ ⇒ 装配方**不接**它 ⇒ ⇒ `run_consolidation` 的行为
    /// **一字不变**（零回退风险）⇒ ⇒ 只有**显式接上**才走增量语义。
    consolidation_trigger: Option<ConsolidationTrigger>,
    ranking: MemoryRankingConfig,
    activation_source: Option<Arc<dyn ActivationSource>>,
    typed_recall_source: Option<Arc<dyn TypedMemoryRecallSource>>,
    typed_identity: Option<TypedRecallIdentity>,
    injection_format: bool,
    /// Optional read-only diary store (cross-diary retrieval wiring).
    ///
    /// Absent = the recall path never touches the diary ⇒ byte-for-byte the
    /// pre-wiring behavior. Present = hits may carry related diary excerpts.
    diary_store: Option<Arc<dyn DiaryStore>>,
    /// Optional read-only user knowledge-base corpus root (`<data>/wiki`).
    ///
    /// Absent = the recall path never touches the corpus ⇒ byte-for-byte the
    /// pre-wiring behavior. Present = matching corpus sections may ride along
    /// as **additional** evidence (items / scores / ordering untouched).
    wiki_root: Option<std::path::PathBuf>,
    /// Optional observation-layer feed for memory-governance audit events.
    /// Emission is after-the-fact and never changes an operation's outcome.
    audit_events: Option<AuditEventSink>,
}

/// 合并两批检索候选：按 `id` 并集去重，同 id 保留**更高分**那份。
///
/// 用于查询扩展的召回增强：并集只会**增加**候选，绝不压低任何已有候选的分数
/// （这正是它不能写成「用扩展查询替换原查询」的原因）。
///
/// 排序口径与 [`HybridRetrievalPipeline::retrieve_with_status`] 一致：
/// 分数降序、同分按 id 升序 —— 确定性输出，便于对账与测试。
fn merge_candidates_by_id(
    primary: Vec<MemoryCandidate>,
    extra: Vec<MemoryCandidate>,
) -> Vec<MemoryCandidate> {
    let mut by_id: std::collections::HashMap<String, MemoryCandidate> =
        std::collections::HashMap::with_capacity(primary.len() + extra.len());
    for candidate in primary.into_iter().chain(extra) {
        match by_id.get_mut(&candidate.id) {
            Some(existing) if candidate.score > existing.score => {
                *existing = candidate;
            }
            Some(_) => {}
            None => {
                by_id.insert(candidate.id.clone(), candidate);
            }
        }
    }
    let mut merged: Vec<MemoryCandidate> = by_id.into_values().collect();
    merged.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    merged
}

/// 单条 episode 的**治理判定**（三态 ⇒ **fail-closed**）。
///
/// # 为什么不能只写 `if let Ok(Some(gov)) = …`
///
/// 旧写法把「**治理查询失败**（`Err`）」与「**查无治理记录**（`Ok(None)`）」、
/// 「**有治理记录**（`Ok(Some(..))`）」压成同一件事 ⇒ ⇒ 查询出错时**整段过滤
/// 逻辑被跳过** ⇒ ⇒ 被标记为 `Forgotten` 的 episode **仍然进入召回**
/// （应当被过滤掉）。
///
/// ⇒ ⇒ 现在三态分开，**`Err` 一律 fail-closed**（不可判定 ⇒ 过滤掉），
/// **绝不**把「查不出来」当「没有治理 ⇒ 放行」。
#[derive(Debug, Clone)]
enum GovernanceDecision {
    /// 治理查询**失败** ⇒ 不可判定 ⇒ **fail-closed**（过滤掉）。
    Undecidable,
    /// **查无治理记录**（该 episode 在治理账里没有行）。
    ///
    /// 存量行口径 = 默认 `active`（见 `memory_governance` 的 V6 sidecar 语义）
    /// ⇒ ⇒ 普通召回面放行；**精确授权面**要求「显式的 `Active` 治理记录」
    /// ⇒ ⇒ 这一面不放行（与既有语义一致）。
    Ungoverned,
    /// 有治理记录 ⇒ 按 `status` 判。
    Governed(GovernedEpisode),
}

impl MemoryCoordinator {
    /// Create a new memory coordinator with core backend and governance store.
    pub fn new(
        backend: Arc<dyn MemoryBackend>,
        governance: Arc<dyn MemoryGovernanceStore>,
    ) -> Self {
        Self {
            backend,
            scoped_backend: None,
            embedding_provider: None,
            governance,
            working: Mutex::new(HashMap::new()),
            preferences: None,
            graph: None,
            associations: None,
            compressor: ContinuityCompressor::new(),
            compiler: ClosedWorldContextCompiler::new(),
            consolidation: MemoryConsolidationJob::new(),
            consolidation_trigger: None,
            ranking: MemoryRankingConfig::default(),
            activation_source: None,
            typed_recall_source: None,
            typed_identity: None,
            injection_format: false,
            diary_store: None,
            wiki_root: None,
            audit_events: None,
            evidence_source: None,
        }
    }

    /// Attach an observation-layer sink for memory-governance audit events
    /// (forget / state-change / protect / unprotect). Events are emitted after
    /// the governance operation has completed and never change its outcome.
    #[must_use]
    pub fn with_audit_events(mut self, sink: AuditEventSink) -> Self {
        self.audit_events = Some(sink);
        self
    }

    fn emit_audit_event(&self, event: AuditEvent) {
        if let Some(sink) = &self.audit_events {
            sink(event);
        }
    }

    /// Attach optional scoped storage backend for cross-session queries.
    #[must_use]
    pub fn with_scoped_backend(mut self, scoped_backend: Arc<dyn ScopedMemoryBackend>) -> Self {
        self.scoped_backend = Some(scoped_backend);
        self
    }

    /// 接上**夜间整编驱动**（窗口水位 + 只整新料）。
    ///
    /// ⚠️ **不接 = 行为不变**：没有驱动时，`run_consolidation` 仍是「取最近 N 条重整编」
    /// ⇒ ⇒ 装配方**零改动**就不受影响 ⇒ ⇒ 只有**显式接上**才走增量语义。
    ///
    /// 驱动由**装配方持有**（而不是这里新建）⇒ ⇒ 水位可以跨进程持久化
    /// （用 `FileConsolidationWatermark`）⇒ ⇒ 「夜里跑一次」的语义跨重启成立。
    #[must_use]
    pub fn with_consolidation_trigger(mut self, trigger: ConsolidationTrigger) -> Self {
        self.consolidation_trigger = Some(trigger);
        self
    }

    /// 当前挂着的夜间整编驱动（没挂 ⇒ `None`）。
    #[must_use]
    pub fn consolidation_trigger(&self) -> Option<&ConsolidationTrigger> {
        self.consolidation_trigger.as_ref()
    }

    /// Attach optional embedding provider for semantic candidate retrieval.
    #[must_use]
    pub fn with_embedding_provider(
        mut self,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        self.embedding_provider = Some(embedding_provider);
        self
    }

    /// Attach optional semantic preference store.
    #[must_use]
    pub fn with_preferences(mut self, preferences: Arc<dyn PreferenceStore>) -> Self {
        self.preferences = Some(preferences);
        self
    }

    /// Attach optional experience and relational stores.
    #[must_use]
    pub fn with_experience(
        mut self,
        graph: Arc<dyn KnowledgeGraphStore>,
        associations: Arc<dyn AssociationStore>,
    ) -> Self {
        self.graph = Some(graph);
        self.associations = Some(associations);
        self
    }

    /// Attach typed durable candidates to the existing unified ranking path.
    #[must_use]
    pub fn with_typed_recall(
        mut self,
        source: Arc<dyn TypedMemoryRecallSource>,
        identity: TypedRecallIdentity,
    ) -> Self {
        self.typed_recall_source = Some(source);
        self.typed_identity = Some(identity);
        self
    }

    /// Switch the prompt overlay renderer to the closed-world injection
    /// format (2026-10-06 W2: `memory_injection` 反幻觉证据清单; 默认 XML 格式不变).
    #[must_use]
    pub fn with_memory_injection_format(mut self) -> Self {
        self.injection_format = true;
        self
    }

    /// Attach a **read-only** diary store so recall hits can carry related
    /// diary excerpts (cross-diary retrieval wiring).
    ///
    /// The store is only ever **read** (`list_days` / `read_day`): the recall
    /// path never writes, edits, or deletes a diary. Without this injection the
    /// recall path stays byte-for-byte identical to the pre-wiring behavior.
    #[must_use]
    pub fn with_diary_store(mut self, diary_store: Arc<dyn DiaryStore>) -> Self {
        self.diary_store = Some(diary_store);
        self
    }

    /// Reference to the optional read-only diary store.
    pub fn diary_store(&self) -> Option<&dyn DiaryStore> {
        self.diary_store.as_deref()
    }

    /// 注入用户知识库语料根（`<data>/wiki`）。**存在性即开关**：
    /// 目录不存在 ⇒ 检索路径整段跳过 ⇒ 逐字节零行为变化。
    ///
    /// 语料面**只读**：检索路径只扫描 `.md` 语料，绝不写回知识库。
    #[must_use]
    pub fn with_wiki_root(mut self, root: std::path::PathBuf) -> Self {
        self.wiki_root = Some(root);
        self
    }

    /// 用户知识库语料根（未注入 ⇒ `None`）。
    pub fn wiki_root(&self) -> Option<&std::path::Path> {
        self.wiki_root.as_deref()
    }

    /// values remain unchanged for backwards-compatible ranking.
    #[must_use]
    pub fn with_activation_source(mut self, source: Arc<dyn ActivationSource>) -> Self {
        self.activation_source = Some(source);
        self
    }

    pub fn governance(&self) -> &dyn MemoryGovernanceStore {
        self.governance.as_ref()
    }

    /// 读单条 episode 的治理判定（**一次查询** ⇒ 过滤与 override 用同一份事实）。
    ///
    /// ⚠️ **`Err` ≠ `Ok(None)`**：`Err` = 查询失败（不可判定）⇒ ⇒ 交给
    /// [`Self::governance_allows`] 做 **fail-closed** 判定。
    fn governance_decision(&self, episode_id: &str) -> GovernanceDecision {
        match self.governance.get_governed(episode_id) {
            Ok(Some(governed)) => GovernanceDecision::Governed(governed),
            Ok(None) => GovernanceDecision::Ungoverned,
            Err(_) => GovernanceDecision::Undecidable,
        }
    }

    /// 治理判定 ⇒ 该 episode **是否允许进入召回**。
    ///
    /// # 三种口径（逐条说清「Err 的含义是什么」）
    ///
    /// | 治理查询结果 | 普通召回面 | 精确授权面（`exact_scope_authorization`） |
    /// |---|---|---|
    /// | `Err`（查询失败 ⇒ 不可判定） | **不放行**（fail-closed） | **不放行** |
    /// | `Ok(None)`（查无治理记录） | 放行（存量行 = 默认 active） | **不放行**（要求显式 `Active` 记录） |
    /// | `Ok(Some(gov))` 且 `status == Active` | 放行 | 放行 |
    /// | `Ok(Some(gov))` 且 `status == Forgotten` | **不放行** | **不放行** |
    ///
    /// ⇒ ⇒ **唯一的行为变化**是普通召回面遇到 `Err` 时不再放行（旧实现把
    /// 查询失败当「无治理 ⇒ 放行」⇒ 被遗忘的条目会漏回来）。
    fn governance_allows(decision: &GovernanceDecision, exact_scope_authorization: bool) -> bool {
        match decision {
            // 治理查询失败 ⇒ 不可判定 ⇒ **fail-closed**（过滤掉）。
            GovernanceDecision::Undecidable => false,
            // 查无治理记录 ⇒ 普通面放行（存量行默认 active）；精确授权面不放行。
            GovernanceDecision::Ungoverned => !exact_scope_authorization,
            GovernanceDecision::Governed(governed) => {
                if exact_scope_authorization {
                    governed.status == MemoryGovernanceStatus::Active
                } else {
                    governed.status != MemoryGovernanceStatus::Forgotten
                }
            }
        }
    }

    /// Reference to the underlying memory backend.
    pub fn backend(&self) -> &dyn MemoryBackend {
        self.backend.as_ref()
    }

    /// Shared backend handle for Host assembly adapters; grants remain external.
    pub fn backend_arc(&self) -> Arc<dyn MemoryBackend> {
        self.backend.clone()
    }

    /// Reference to the optional scoped memory backend.
    pub fn scoped_backend(&self) -> Option<&dyn ScopedMemoryBackend> {
        self.scoped_backend.as_deref()
    }

    /// Reference to the optional embedding provider.
    pub fn embedding_provider(&self) -> Option<&dyn EmbeddingProvider> {
        self.embedding_provider.as_deref()
    }

    /// Configure the centralized deterministic ranking weights.
    #[must_use]
    pub fn with_ranking_config(mut self, ranking: MemoryRankingConfig) -> Self {
        self.ranking = ranking;
        self
    }

    /// Collect memory candidates across all requested layers.
    fn collect_candidates(
        &self,
        query: &MemoryRecallQuery,
        now_ms: i64,
    ) -> (
        Vec<MemoryCandidate>,
        HashMap<String, (MemoryLayerKind, i64, Option<String>)>,
        usize,
    ) {
        // ⚠️ **时间锚点**（2026-10-27 生产接线）：问句里说「3年前」⇒
        // ⇒ recency 的**参照时间**应该是「3年前」⇒ 而不是「现在」
        // ⇒ ⇒ 下面 4 处 `delta_hours` **全部**改用这个参照时间
        // ⇒ ⇒ ⚠️ **只影响 recency** ⇒ ⇒ `now_ms` 本身**照旧**传给外部 source
        // ⇒ ⇒ ⇒ 外部 source 看到的仍是「现在」⇒ ⇒ **零副作用**。
        //
        // ⇒ 没有锚点 ⇒ `None` ⇒ ⇒ **完全保持原行为**（**零行为变化**）。
        let recency_reference_ms = crate::anchor_injection::anchor_reference_time_ms(
            now_ms,
            &crate::anchor_injection::parse_time_anchors(&query.query_text),
        );
        let mut candidates = Vec::new();
        let mut candidate_meta = HashMap::new();
        let mut governance_filtered = 0;
        let source_reference = |layer: &str, episode: &Episode, provenance: &MemoryProvenance| {
            let source = provenance.source_session.as_deref();
            if query.exact_scope_authorization
                && (source.is_none() || source == Some(query.session_id.as_str()))
            {
                // The query key may be a storage partition. Unknown provenance
                // must not be presented as the original execution session.
                None
            } else {
                Some(format!("{layer}:{}", source.unwrap_or(&episode.session_id)))
            }
        };

        // Layer 1: Working Memory
        if query.layers.contains(&MemoryLayerKind::Working) {
            let working_lock = self
                .working
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(session_episodes) = working_lock.get(&query.session_id) {
                for ep in session_episodes
                    .iter()
                    .rev()
                    .take((query.limit * 4).max(32))
                {
                    // 治理判定（**fail-closed**：查不出来 = 不可判定 = 过滤掉）。
                    let decision = self.governance_decision(&ep.id);
                    if !Self::governance_allows(&decision, query.exact_scope_authorization) {
                        governance_filtered += 1;
                        continue;
                    }
                    let (scope, provenance) = self.episode_scope_metadata(ep);
                    if !scope.is_visible_in(&query.visible_scopes) {
                        governance_filtered += 1;
                        continue;
                    }
                    let delta_hours =
                        recency_delta_hours(recency_reference_ms, now_ms, ep.timestamp * 1000);
                    let s_rec = (-query.recency_decay_lambda * delta_hours)
                        .exp()
                        .clamp(0.1, 1.0);
                    let id = ep.id.clone();
                    candidate_meta.insert(
                        id.clone(),
                        (
                            MemoryLayerKind::Working,
                            ep.timestamp * 1000,
                            source_reference("working", ep, &provenance),
                        ),
                    );
                    candidates.push(MemoryCandidate {
                        id,
                        layer: "working".to_string(),
                        scope,
                        content: match &decision {
                            GovernanceDecision::Governed(gov) => gov.content_override.clone(),
                            _ => None,
                        }
                        .unwrap_or_else(|| ep.content.clone()),
                        score: 0.0,
                        score_components: ScoreComponents {
                            recency: s_rec,
                            importance: 0.8,
                            confidence: 0.8,
                            activation: 1.0,
                            ..Default::default()
                        },
                        provenance,
                    });
                }
            }
        }

        // Layer 2: Episodic Memory (Governed SQLite / Scoped Storage)
        if query.layers.contains(&MemoryLayerKind::Episodic) {
            if let Some(scoped) = &self.scoped_backend {
                let candidate_query = MemoryCandidateQuery {
                    source_session: Some(query.session_id.clone()),
                    visible_scopes: query.visible_scopes.clone(),
                    limit: (query.limit * 8).max(64),
                    as_of_ms: query.as_of_ms,
                };
                if let Ok(scoped_episodes) = scoped.query_candidates(&candidate_query) {
                    for ep in scoped_episodes {
                        // 治理判定（**fail-closed**：查不出来 = 不可判定 = 过滤掉）。
                        let decision = self.governance_decision(&ep.id);
                        if !Self::governance_allows(&decision, query.exact_scope_authorization) {
                            governance_filtered += 1;
                            continue;
                        }
                        let (scope, provenance) = self.episode_scope_metadata(&ep);
                        if !scope.is_visible_in(&query.visible_scopes) {
                            governance_filtered += 1;
                            continue;
                        }
                        let mut content = ep.content.clone();
                        let mut importance = 0.5;
                        if let GovernanceDecision::Governed(gov) = &decision {
                            if let Some(c_override) = gov.content_override.clone() {
                                content = c_override;
                            }
                            if gov.protected {
                                importance = 0.9;
                            }
                        }
                        let delta_hours =
                            recency_delta_hours(recency_reference_ms, now_ms, ep.timestamp * 1000);
                        let s_rec = (-query.recency_decay_lambda * delta_hours)
                            .exp()
                            .clamp(0.1, 1.0);
                        let id = ep.id.clone();
                        candidate_meta.insert(
                            id.clone(),
                            (
                                MemoryLayerKind::Episodic,
                                ep.timestamp * 1000,
                                source_reference("episodic", &ep, &provenance),
                            ),
                        );
                        candidates.push(MemoryCandidate {
                            id,
                            layer: "episodic".to_string(),
                            scope,
                            content,
                            score: 0.0,
                            score_components: ScoreComponents {
                                recency: s_rec,
                                importance,
                                confidence: importance,
                                activation: 0.8,
                                ..Default::default()
                            },
                            provenance,
                        });
                    }
                }
            } else if let Ok(raw_eps) = self
                .backend
                .recent_episodes(&query.session_id, (query.limit * 8).max(64))
            {
                for ep in raw_eps {
                    // 治理判定（**fail-closed**：查不出来 = 不可判定 = 过滤掉）。
                    let decision = self.governance_decision(&ep.id);
                    if !Self::governance_allows(&decision, query.exact_scope_authorization) {
                        governance_filtered += 1;
                        continue;
                    }
                    let (scope, provenance) = self.episode_scope_metadata(&ep);
                    if !scope.is_visible_in(&query.visible_scopes) {
                        governance_filtered += 1;
                        continue;
                    }
                    let mut content = ep.content.clone();
                    let mut importance = 0.5;
                    if let GovernanceDecision::Governed(gov) = &decision {
                        if let Some(c_override) = gov.content_override.clone() {
                            content = c_override;
                        }
                        if gov.protected {
                            importance = 0.9;
                        }
                    }
                    let delta_hours =
                        recency_delta_hours(recency_reference_ms, now_ms, ep.timestamp * 1000);
                    let s_rec = (-query.recency_decay_lambda * delta_hours)
                        .exp()
                        .clamp(0.1, 1.0);
                    let id = ep.id.clone();
                    candidate_meta.insert(
                        id.clone(),
                        (
                            MemoryLayerKind::Episodic,
                            ep.timestamp * 1000,
                            source_reference("episodic", &ep, &provenance),
                        ),
                    );
                    candidates.push(MemoryCandidate {
                        id,
                        layer: "episodic".to_string(),
                        scope,
                        content,
                        score: 0.0,
                        score_components: ScoreComponents {
                            recency: s_rec,
                            importance,
                            confidence: importance,
                            activation: 0.8,
                            ..Default::default()
                        },
                        provenance,
                    });
                }
            }
        }

        // Layer 3: Semantic / Personal Memory (Preferences)
        if !query.exact_scope_authorization && query.layers.contains(&MemoryLayerKind::Semantic) {
            if let Some(pref_store) = &self.preferences {
                let session_id_parsed =
                    SessionId::from_str(&query.session_id).unwrap_or_else(|_| SessionId::new());
                if let Ok(prefs) = pref_store.recall_for_context(
                    &session_id_parsed,
                    &query.query_text,
                    (query.limit * 2).max(10) as u32,
                ) {
                    for pref in prefs {
                        let pref_ts_ms = if pref.created_at < 10_000_000_000 {
                            pref.created_at * 1000
                        } else {
                            pref.created_at
                        };
                        let delta_hours =
                            recency_delta_hours(recency_reference_ms, now_ms, pref_ts_ms);
                        let s_rec = (-query.recency_decay_lambda * delta_hours)
                            .exp()
                            .clamp(0.1, 1.0);
                        let s_imp = pref.confidence.clamp(0.1, 1.0);
                        let id = format!("pref:{}", pref.id);
                        candidate_meta.insert(
                            id.clone(),
                            (
                                MemoryLayerKind::Semantic,
                                pref_ts_ms,
                                Some(format!("preference:{}", pref.id)),
                            ),
                        );
                        candidates.push(MemoryCandidate {
                            id,
                            layer: "semantic".to_string(),
                            scope: MemoryScope::Session {
                                session_id: query.session_id.clone(),
                            },
                            content: format!("Topic: {}. Preference: {}", pref.topic, pref.stance),
                            score: 0.0,
                            score_components: ScoreComponents {
                                recency: s_rec,
                                importance: s_imp,
                                confidence: s_imp,
                                ..Default::default()
                            },
                            provenance: MemoryProvenance {
                                source: "preference".to_string(),
                                source_session: Some(pref.session_id.to_string()),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
        }

        // Layer 4: Relational / Temporal Memory (Graph & Associations)
        if !query.exact_scope_authorization
            && query.layers.contains(&MemoryLayerKind::Relational)
            && !query.query_text.trim().is_empty()
        {
            if let Some(graph_store) = &self.graph {
                // ⭐ **按 as-of 时间取事实**（2026-11-05）：让**时间锚点**对图事实也生效。
                //
                // 接线前：`facts_from` 的语义是「**现在**有效的事实」（store 内部硬编码 now）
                // ⇒ ⇒ 召回带 `as_of_ms`（例:「3年前」）时 ⇒ ⇒ **问过去、答现在**
                // ⇒ ⇒ ⇒ **答非所问**。
                // ⇒ ⇒ 这里改用 `facts_from_as_of` ⇒ ⇒ 拿到「**当时知道什么**」。
                let graph_as_of_ms = query.as_of_ms.unwrap_or(now_ms);
                if let Ok(facts) = graph_store.facts_from_as_of(
                    &query.query_text,
                    (query.limit * 2).max(10) as u32,
                    graph_as_of_ms,
                ) {
                    for fact in facts {
                        let id = format!(
                            "fact:{}:{}:{}",
                            fact.subject_id, fact.predicate, fact.object_id
                        );
                        candidate_meta.insert(
                            id.clone(),
                            (
                                MemoryLayerKind::Relational,
                                now_ms,
                                Some("knowledge_graph".to_string()),
                            ),
                        );
                        candidates.push(MemoryCandidate {
                            id,
                            layer: "relational".to_string(),
                            scope: MemoryScope::Session {
                                session_id: query.session_id.clone(),
                            },
                            content: format!(
                                "{} {} {}",
                                fact.subject_id, fact.predicate, fact.object_id
                            ),
                            score: 0.0,
                            score_components: ScoreComponents {
                                recency: 1.0,
                                importance: 0.6,
                                confidence: 0.6,
                                activation: 0.7,
                                graph: 1.0,
                                ..Default::default()
                            },
                            provenance: MemoryProvenance {
                                source: "knowledge_graph".to_string(),
                                source_session: Some(query.session_id.clone()),
                                ..Default::default()
                            },
                        });
                    }
                }
            }
        }

        if let (Some(source), Some(identity)) = (&self.typed_recall_source, &self.typed_identity) {
            if let Ok(typed_candidates) = source.candidates(query, identity, now_ms) {
                for candidate in typed_candidates {
                    if !candidate.scope.is_visible_in(&query.visible_scopes)
                        && (query.exact_scope_authorization
                            || !typed_scope_visible(&candidate.scope, identity))
                    {
                        governance_filtered += 1;
                        continue;
                    }
                    let layer = if candidate.layer == "relational" {
                        MemoryLayerKind::Relational
                    } else {
                        MemoryLayerKind::Semantic
                    };
                    candidate_meta.insert(
                        candidate.id.clone(),
                        (layer, now_ms, candidate.provenance.source_request.clone()),
                    );
                    candidates.push(candidate);
                }
            }
        }

        (candidates, candidate_meta, governance_filtered)
    }

    /// Execute the Unified Recall Pipeline across requested layers synchronously.
    pub fn recall(&self, query: &MemoryRecallQuery) -> Result<MemoryRecallResult, MemoryError> {
        if self.embedding_provider.is_none() {
            let now_ms = query
                .as_of_ms
                .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
            let (candidates, candidate_meta, governance_filtered) =
                self.collect_candidates(query, now_ms);
            let total_candidates = candidates.len();
            // ⭐ **联想通道**（第三条）：提前算 ⇒ ⇒ 「有没有候选」**跨通道**判断。
            // ⚠️ 联想通道**没有候选** ⇒ ⇒ 提前返回的**形状与条件一字不改**（零回归）。
            let associative_candidates = self.collect_associative_candidates(query);
            if total_candidates == 0 && associative_candidates.is_empty() {
                return Ok(MemoryRecallResult {
                    items: Vec::new(),
                    total_candidates: 0,
                    governance_filtered,
                    total_chars: 0,
                    retrieval_status: Some(RetrievalStatus {
                        lexical_candidates: 0,
                        vector_candidates: 0,
                        used_lexical_fallback: true,
                        reranked: false,
                    }),
                    wiki_sections: Vec::new(),
                });
            }
            self.execute_hybrid_retrieval(
                query,
                candidates,
                candidate_meta,
                governance_filtered,
                total_candidates + associative_candidates.len(),
                None,
                now_ms,
                associative_candidates,
            )
        } else {
            block_on_future(self.recall_async(query))
        }
    }

    /// Execute the Unified Recall Pipeline asynchronously, running semantic embeddings if configured.
    pub async fn recall_async(
        &self,
        query: &MemoryRecallQuery,
    ) -> Result<MemoryRecallResult, MemoryError> {
        let now_ms = query
            .as_of_ms
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
        let (candidates, candidate_meta, governance_filtered) =
            self.collect_candidates(query, now_ms);

        let total_candidates = candidates.len();
        // ⭐ **联想通道**（第三条）：与同步路径**同口径**（见 `collect_associative_candidates`）。
        let associative_candidates = self.collect_associative_candidates(query);
        if total_candidates == 0 && associative_candidates.is_empty() {
            return Ok(MemoryRecallResult {
                items: Vec::new(),
                total_candidates: 0,
                governance_filtered,
                total_chars: 0,
                retrieval_status: Some(RetrievalStatus {
                    lexical_candidates: 0,
                    vector_candidates: 0,
                    used_lexical_fallback: self.embedding_provider.is_none(),
                    reranked: false,
                }),
                wiki_sections: Vec::new(),
            });
        }

        let mut vector_source = None;
        if let Some(ref provider) = self.embedding_provider {
            if !query.query_text.trim().is_empty() {
                if let Ok(query_vector) = provider.embed(&query.query_text).await {
                    if !query_vector.is_empty() {
                        let mut vector_candidates = Vec::new();
                        for cand in &candidates {
                            let cand_vec_opt = if let Ok(Some(meta)) =
                                self.backend.get_episode_metadata(&cand.id)
                            {
                                meta.get("vector").and_then(|v| {
                                    serde_json::from_value::<Vec<f32>>(v.clone()).ok()
                                })
                            } else {
                                None
                            };
                            if !valid_embedding(&query_vector, None) {
                                continue;
                            }
                            let cand_vec = match cand_vec_opt {
                                Some(v) if valid_embedding(&v, Some(query_vector.len())) => Some(v),
                                _ => provider
                                    .embed(&cand.content)
                                    .await
                                    .ok()
                                    .filter(|v| valid_embedding(v, Some(query_vector.len()))),
                            };
                            if let Some(cand_vec) = cand_vec {
                                let sim = crate::canonical::vector::cosine_similarity(
                                    &query_vector,
                                    &cand_vec,
                                );
                                if sim.is_finite() {
                                    let mut vc = cand.clone();
                                    vc.score_components.semantic = f64::from(sim).clamp(0.0, 1.0);
                                    vector_candidates.push(vc);
                                }
                            }
                        }
                        if !vector_candidates.is_empty() {
                            vector_source =
                                Some(StaticVectorCandidateSource::new(vector_candidates));
                        }
                    }
                }
            }
        }

        self.execute_hybrid_retrieval(
            query,
            candidates,
            candidate_meta,
            governance_filtered,
            total_candidates + associative_candidates.len(),
            vector_source,
            now_ms,
            associative_candidates,
        )
    }

    /// ⭐ **联想候选**（2026-11-05）：第三条召回通道的候选。
    ///
    /// ⚠️ **为什么提前算**（不是塞进 `collect_candidates`）：
    /// 召回入口有个**提前返回**（`total_candidates == 0` ⇒ ⇒ 直接返回空结果）⇒ ⇒
    /// 若联想候选等到管线里才算 ⇒ ⇒ **提前返回会先把它挡在门外** ⇒ ⇒ 通道形同虚设。
    /// ⇒ ⇒ 所以在**提前返回之前**就算一次（跨通道判断「到底有没有候选」）。
    ///
    /// ⚠️ **提前返回的形状一字不改**：联想通道**没有候选**时 ⇒ ⇒ 走的还是老路
    /// ⇒ ⇒ 结果逐字节不变（**零回归**）。
    fn collect_associative_candidates(&self, query: &MemoryRecallQuery) -> Vec<MemoryCandidate> {
        let Some(associations) = &self.associations else {
            return Vec::new();
        };
        if query.exact_scope_authorization
            || !query.layers.contains(&MemoryLayerKind::Relational)
            || query.query_text.trim().is_empty()
        {
            return Vec::new();
        }
        let source = crate::associative_recall::AssociativeCandidateSource::new(
            Arc::clone(associations),
            Arc::clone(&self.backend),
            Arc::clone(&self.governance),
        );
        match source.candidates(&query.query_text, query.limit) {
            Ok(candidates) => candidates,
            Err(_) => Vec::new(),
        }
    }

    fn execute_hybrid_retrieval(
        &self,
        query: &MemoryRecallQuery,
        candidates: Vec<MemoryCandidate>,
        candidate_meta: HashMap<String, (MemoryLayerKind, i64, Option<String>)>,
        governance_filtered: usize,
        total_candidates: usize,
        vector_source: Option<StaticVectorCandidateSource>,
        now_ms: i64,
        associative_candidates: Vec<MemoryCandidate>,
    ) -> Result<MemoryRecallResult, MemoryError> {
        // ⚠️ **会话维度**（多会话串扰修复）：生效检索权重按**会话**分桶 ⇒ ⇒
        // 本次召回必须在自己的会话作用域里读生效值（否则会读到别的会话的
        // 调权结果）。作用域覆盖整个排序段（管线里的 `entry_salience` 也
        // 在作用域内读）。
        let _session_scope = crate::retrieval_tuning::enter_session_scope(&query.session_id);
        let mut candidates = candidates;
        if let Some(source) = &self.activation_source {
            for candidate in &mut candidates {
                if let Some(activation) = source.activation(&candidate.id, now_ms) {
                    candidate.score_components.activation = activation;
                }
            }
        }
        let lexical_source = Bm25LexicalCandidateSource::new(candidates);
        // ⚠️ **声明顺序有讲究**：`sources` 持有各通道的**引用** ⇒ ⇒ 通道源必须
        // **活得更久** ⇒ ⇒ 所以联想源先声明、再建 `sources`（否则借用不过生命周期检查）。
        let associative_source = if !associative_candidates.is_empty() {
            Some(
                crate::associative_recall::AssociativeCandidateSource::from_candidates(
                    associative_candidates,
                ),
            )
        } else {
            None
        };
        let mut sources: Vec<&dyn MemoryCandidateSource> = vec![&lexical_source];
        if let Some(ref vs) = vector_source {
            sources.push(vs);
        }
        // ⭐ **联想召回通道**（2026-11-05 接线）：第三条通道 ⇒ ⇒ **与词法/向量正交**。
        //
        // 接线前：`self.associations` 字段在、`with_experience` 会装上
        // ⇒ ⇒ **但全 crate 零调用 `top_associations`** ⇒ ⇒ **已接线未通电**。
        //
        // 为什么必须是**独立通道**而不是「往候选列表里塞」：
        // 召回链的**唯一入口就是 `sources`** ⇒ ⇒ 混进 `candidates` 只会变成
        // **BM25 的输入**（被词面相似度重新排一遍）⇒ ⇒ ⇒ 联想的信号会被抹掉。
        //
        // ⚠️ **零回归**：联想通道**没有候选**时 ⇒ ⇒ 它不进 `sources`
        // ⇒ ⇒ 管线行为与接线前**逐字节**一致。
        if let Some(ref source) = associative_source {
            sources.push(source);
        }

        // V4-c 检索维度自进化: 排序权重取**生效值** (调权落地 override; 无调权
        // 在场 = 原配置原值, 零改动回落)。
        let pipeline = HybridRetrievalPipeline::new_for_session(&query.session_id, self.ranking);
        let qualified = self.evidence_source.is_some();
        let discover = |text: &str| {
            if qualified {
                pipeline.bounded_ranked_pool_for_session(
                    &query.session_id,
                    text,
                    &query.visible_scopes,
                    &sources,
                )
            } else {
                pipeline.retrieve_with_status_for_session(
                    &query.session_id,
                    text,
                    &query.visible_scopes,
                    &sources,
                    query.limit,
                    query.max_chars,
                )
            }
        };
        let (primary_candidates, mut status) = discover(&query.query_text)?;

        // 查询扩展（`query_expand`）：**召回增强**，不是替换查询。
        // # 为什么不是「直接用 rewritten 查询」
        // 检索打分是**各项之和**（BM25 逐词累加，无查询长度归一化）——
        // 若把扩展后的查询整句送进去，「匹配到的同义/聚类词多」的文档会累出
        // 更高分，反而压过精确匹配那一篇。加词本意是提召回，不该降精度。
        //
        // # 所以
        //
        // 只在**主查询召回不足 `limit`** 时补跑一遍扩展查询，再把两批候选按
        // id **并集去重**（同 id 保留更高分）。于是：
        // - 主查询够用 ⇒ 完全走原路径，**零行为变化**；
        // - 主查询稀疏（用户短查询、生僻词）⇒ 扩展补召回，精确匹配那篇的分数
        //   不会被任何东西压低（并集只增不减）。
        let primary_count = if qualified {
            primary_candidates
                .iter()
                .filter(|candidate| candidate.score >= query.min_score)
                .count()
        } else {
            primary_candidates.len()
        };
        let (mut ranked_candidates, mut status) = if primary_count >= query.limit {
            (primary_candidates, status)
        } else {
            let expanded = crate::query_expand::expand_query(&query.query_text);
            if expanded.extra_terms.is_empty() {
                (primary_candidates, status)
            } else {
                let (extra_candidates, extra_status) = discover(&expanded.rewritten)?;
                status.lexical_candidates += extra_status.lexical_candidates;
                status.vector_candidates += extra_status.vector_candidates;
                (
                    merge_candidates_by_id(primary_candidates, extra_candidates),
                    status,
                )
            }
        };

        if self.embedding_provider.is_none() {
            status.used_lexical_fallback = true;
        }
        if qualified {
            ranked_candidates.truncate(crate::retrieval_pipeline::RANKED_POOL_CAP);
            ranked_candidates.retain(|candidate| candidate.score >= query.min_score);
        }

        let mut final_items = Vec::new();
        let mut seen_content = std::collections::HashSet::new();
        // ⭐ 计费口径与选择侧**同一口径**（2026-11-08）：按交付侧实际裁到的长度计
        let (mut total_chars, span) = (0, self.ranking.candidate_span_chars);
        for cand in self.source_coverage(ranked_candidates, &pipeline.ranking) {
            if cand.score < query.min_score {
                continue;
            }
            if final_items.len() >= query.limit {
                break;
            }
            if qualified {
                let normalized = Self::qualified_content_key(&cand.content);
                if normalized.is_empty() || !seen_content.insert(normalized) {
                    continue;
                }
            }
            let Some(spent) = diversity::fit(total_chars, &cand.content, query.max_chars, span)
            else {
                break;
            };
            total_chars = spent;
            let (layer, timestamp_ms, source_ref) = candidate_meta
                .get(&cand.id)
                .cloned()
                .unwrap_or((MemoryLayerKind::Episodic, now_ms, None));
            final_items.push(RecalledMemoryItem {
                id: cand.id,
                layer,
                content: cand.content,
                timestamp_ms,
                score: cand.score,
                importance: cand.score_components.importance,
                source_ref,
                score_components: Some(cand.score_components),
                diary_snippets: Vec::new(),
            });
        }

        // 跨日记检索接线 (只读): 命中事实附上**关联日记片段**。
        //
        // **缺省零变化**: 未注入日记面 (`diary_store == None`) ⇒ 这一段整段跳过，
        // 召回结果与接线前逐字节一致。注入后也只是**附加**片段：不改内容、
        // 不改分数、不改排序。
        if let Some(diary) = &self.diary_store {
            cross_diary_wiring::attach_diary_snippets(
                &mut final_items,
                diary.as_ref(),
                CrossDiaryLimits::default(),
            );
        }

        let mut result = MemoryRecallResult {
            items: final_items,
            total_candidates,
            governance_filtered,
            total_chars,
            retrieval_status: Some(status),
            wiki_sections: Vec::new(),
        };

        // 用户知识库接线 (只读): 语料在场 ⇒ 命中节作为**附加**证据附在结果上。
        //
        // 缺省 (无 wiki_root / 目录不存在 / 无 .md) ⇒ 这段整段跳过, 结果与接线前一致。
        //
        // **只附加**: 不改 items、不改分数、不改排序、不改 total_candidates ——
        // 命中节是结果级的旁证, 不是召回候选。
        if let Some(root) = &self.wiki_root {
            let ctx = crate::wiki_corpus::WikiContext::build(
                root,
                &query.query_text,
                crate::wiki_corpus::WikiLimits::default(),
            );
            result.wiki_sections = ctx.sections;
        }

        Ok(result)
    }

    /// Persist turn writeback entry into Working and Episodic layers.
    pub fn writeback(&self, entry: &MemoryWritebackEntry) -> Result<String, MemoryError> {
        let episode_id = format!("ep-{}", uuid::Uuid::new_v4());
        let timestamp_secs = if let Some(ms) = entry.timestamp_ms {
            ms / 1000
        } else {
            chrono::Utc::now().timestamp()
        };

        let episode = Episode {
            id: episode_id.clone(),
            timestamp: timestamp_secs,
            role: entry.role.clone(),
            content: entry.content.clone(),
            session_id: entry.session_id.clone(),
        };

        self.persist_episode(&episode, &entry.scope, &entry.provenance)?;
        Ok(episode_id)
    }

    /// Persist a caller-owned episode without replacing its stable identifier.
    ///
    /// This is the coordinator path for runtimes that derive an episode ID before
    /// persistence. The compatibility [`Self::writeback`] API intentionally keeps
    /// generating IDs for [`MemoryWritebackEntry`] callers.
    pub fn writeback_episode(&self, episode: &Episode) -> Result<String, MemoryError> {
        let scope = MemoryScope::Session {
            session_id: episode.session_id.clone(),
        };
        let provenance = MemoryProvenance {
            source_session: Some(episode.session_id.clone()),
            ..MemoryProvenance::default()
        };
        self.persist_episode(episode, &scope, &provenance)?;
        Ok(episode.id.clone())
    }

    fn persist_episode(
        &self,
        episode: &Episode,
        scope: &MemoryScope,
        provenance: &MemoryProvenance,
    ) -> Result<(), MemoryError> {
        self.persist_episode_with_metadata(episode, scope, provenance, serde_json::json!({}))
    }

    fn persist_episode_with_metadata(
        &self,
        episode: &Episode,
        scope: &MemoryScope,
        provenance: &MemoryProvenance,
        extra: serde_json::Value,
    ) -> Result<(), MemoryError> {
        let mut metadata = extra
            .as_object()
            .cloned()
            .ok_or_else(|| MemoryError::Invalid("episode metadata must be an object".into()))?;
        for (key,value) in serde_json::json!({"scope":scope,"provenance":provenance,"layer":"episodic","content_hash":crate::canonical::vector::content_hash(&episode.content)}).as_object().expect("literal object") {
            metadata.insert(key.clone(),value.clone());
        }
        let outcome = self
            .backend
            .put_episode_with_metadata(episode, serde_json::Value::Object(metadata))
            .map_err(|e| MemoryError::Invalid(e.to_string()))?;
        if outcome == EpisodeWriteOutcome::Existing {
            return Ok(());
        }
        // Publish only accepted writes; an idempotent retry must not evict a turn.
        let mut working = self
            .working
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let ring = working
            .entry(episode.session_id.clone())
            .or_insert_with(|| VecDeque::with_capacity(WORKING_RING_BUFFER_CAP));
        if !ring.iter().any(|existing| existing.id == episode.id) {
            if ring.len() >= WORKING_RING_BUFFER_CAP {
                ring.pop_front();
            }
            ring.push_back(episode.clone());
        }
        Ok(())
    }

    /// Compile a structured overlay and report the candidate IDs selected after
    /// sanitization and the overlay character budget is applied.
    pub fn compile_prompt_overlay_with_selected_access(
        &self,
        query: &MemoryRecallQuery,
    ) -> Result<Option<SelectedMemoryAccess>, MemoryError> {
        let recall_result = self.recall(query)?;
        Ok(self.compile_selected(&recall_result, query))
    }

    /// Render one selected overlay with the configured format (XML closed-world
    /// by default; injection format behind `with_memory_injection_format`).
    fn compile_selected(
        &self,
        recalled: &crate::layers::MemoryRecallResult,
        query: &MemoryRecallQuery,
    ) -> Option<SelectedMemoryAccess> {
        if self.evidence_source.is_some() {
            let evidence = recalled
                .items
                .iter()
                .map(|item| {
                    (
                        item.id.clone(),
                        self.source_evidence(&item.id, &item.content),
                    )
                })
                .collect();
            let anchored = crate::anchor_injection::AnchoredInjectionConfig::PRODUCTION;
            let options = crate::context_compiler::SourceRenderOptions {
                per_item_chars: if self.injection_format {
                    if anchored.enabled {
                        anchored.span_chars
                    } else {
                        crate::memory_injection::EVIDENCE_MAX_CHARS
                    }
                } else {
                    self.ranking.candidate_span_chars
                },
                numbered: self.injection_format,
                query: (self.injection_format && anchored.enabled)
                    .then_some(query.query_text.as_str()),
            };
            return self.compiler.compile_with_source_evidence_options(
                recalled,
                &query.session_id,
                query.max_chars,
                &evidence,
                options,
            );
        }
        if self.injection_format {
            // ⚠️ **D-11（owner 2026-10-28 采纳）**：注入「**主体出现处为中心**」的窗口
            // （单条 = 生产跨度，见 `AnchoredInjectionConfig::PRODUCTION`），而不是「正文开头 120 字」。
            //
            // * **总消耗不变**：总量仍由 `query.max_chars` 控制
            //   ⇒ ⇒ 只是**在同一预算内重新分配**；
            // * 主体解析不出来 ⇒ 内部**退回开头截断**（**不注入空证据**）；
            // * 实测（在 400 字/条下）：同预算「挑对段」**0.4202** vs「随便给」**0.0819** ⇒ **5.1×**。
            self.compiler
                .compile_injection_with_selected_access_and_query(
                    recalled,
                    &query.session_id,
                    query.max_chars,
                    &query.query_text,
                    crate::anchor_injection::AnchoredInjectionConfig::PRODUCTION,
                )
        } else {
            self.compiler
                .compile_with_selected_access(recalled, &query.session_id, query.max_chars)
        }
    }

    /// Compile a bounded overlay after optional deterministic proactive candidate selection.
    /// The policy is evaluated only against this invocation's retrieved candidates;
    /// no storage, worker, or provider call is introduced.
    pub fn compile_prompt_overlay_with_proactive_access(
        &self,
        query: &MemoryRecallQuery,
        policy: &ProactiveRecallPolicy,
        cue: &TopicCue,
    ) -> Result<Option<SelectedMemoryAccess>, MemoryError> {
        let mut recalled = self.recall(query)?;
        let candidates = recalled
            .items
            .iter()
            .map(|item| MemoryCandidate {
                id: item.id.clone(),
                layer: item.layer.as_str().to_string(),
                scope: MemoryScope::Global,
                content: item.content.clone(),
                score: item.score,
                score_components: item.score_components.unwrap_or_default(),
                provenance: MemoryProvenance::default(),
            })
            .collect::<Vec<_>>();
        let selected = ProactiveRecallService::new(policy.clone()).recall(cue, &candidates);
        let selected_ids = selected
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        recalled
            .items
            .retain(|item| selected_ids.contains(item.id.as_str()));
        Ok(self.compile_selected(&recalled, query))
    }

    /// Compile a structured closed-world prompt overlay from a memory recall query.
    pub fn compile_prompt_overlay(
        &self,
        query: &MemoryRecallQuery,
    ) -> Result<Option<String>, MemoryError> {
        Ok(self
            .compile_prompt_overlay_with_selected_access(query)?
            .map(|selected| selected.overlay))
    }

    /// Generate a bounded continuity state compression for a session.
    pub fn compress_continuity(
        &self,
        session_id: &str,
        max_summary_chars: usize,
    ) -> Result<ContinuityState, MemoryError> {
        let episodes = self
            .backend
            .recent_episodes(session_id, 50)
            .map_err(|e| MemoryError::Invalid(e.to_string()))?;
        Ok(self
            .compressor
            .compress(session_id, &episodes, max_summary_chars))
    }

    /// Run background or idle memory consolidation job for a session.
    pub fn run_consolidation(&self, session_id: &str) -> Result<ConsolidationReport, MemoryError> {
        let episodes = self
            .governance
            .governed_recent_episodes(session_id, 100)
            .map_err(|error| MemoryError::Invalid(error.to_string()))?;
        Ok(self
            .consolidation
            .consolidate_governed_episodes(session_id, &episodes))
    }

    /// **统一入口**：按「有没有挂驱动」自动走增量语义或旧语义。
    ///
    /// - **挂了驱动** ⇒ 走 [`Self::run_consolidation_incremental`]（只整新料、同窗口只整一次）；
    /// - **没挂驱动** ⇒ 走 [`Self::run_consolidation`]（**一字未改的旧路径**）。
    ///
    /// ⇒ ⇒ **装配方不挂驱动 = 行为完全不变** ⇒ ⇒ 挂驱动是**显式**的升级动作。
    ///
    /// ⚠️ **跳过（Skipped）时返回空报告**（不是错误）⇒ 调用方拿到的
    /// `extracted_insights` 为空 ⇒ ⇒ 「这轮没整编」与「整编了但没提炼出洞察」
    /// 在报告层面**同形** ⇒ ⇒ 需要区分的调用方请用
    /// [`Self::run_consolidation_incremental`]（它返回 [`ConsolidationOutcome`]）。
    pub fn run_consolidation_at(
        &self,
        session_id: &str,
        now_sec: i64,
    ) -> Result<ConsolidationReport, MemoryError> {
        let Some(trigger) = self.consolidation_trigger.as_ref() else {
            return self.run_consolidation(session_id);
        };
        match trigger.run(
            session_id,
            &self.recent_episodes_for(session_id, trigger.policy().max_episodes.max(100))?,
            now_sec,
            Some(self.governance.as_ref()),
        )? {
            ConsolidationOutcome::Consolidated { report, .. } => Ok(report),
            ConsolidationOutcome::Skipped { .. } => Ok(ConsolidationReport::default()),
        }
    }

    /// 取治理可见的近期情节（上限随策略走；至少 100 条，与旧路径同量级）。
    fn recent_episodes_for(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<Episode>, MemoryError> {
        let governed = self
            .governance
            .governed_recent_episodes(session_id, limit)
            .map_err(|error| MemoryError::Invalid(error.to_string()))?;
        Ok(governed.into_iter().map(|item| item.episode).collect())
    }

    /// **带水位的增量整编**：只整「上次整编之后」的新情节，同一窗口只整一次。
    ///
    /// 与 [`Self::run_consolidation`] 的差别（**这是本方法存在的理由**）：
    /// - `run_consolidation` 每次都取**最近 100 条**重新整编 ⇒ ⇒ 老素材被反复重算；
    /// - 本方法按 [`ConsolidationTrigger`] 的**窗口水位**取**新料**，且
    ///   **同一窗口第二次调用直接跳过** ⇒ ⇒ 「夜里整编一次」的语义成立；
    /// - **窗口没到 / 新料不够 ⇒ 不整编、也不推进水位**（不白等一整夜）。
    ///
    /// ⚠️ **老方法一字未改** ⇒ ⇒ 现有调用方（`runtime-assembly` 的 `cognitive.rs`）
    /// **零影响**；要换成增量语义，调用方**多传一个 trigger** 即可。
    ///
    /// - `now_sec`：本轮时刻（**epoch 秒**，与 `Episode.timestamp` 一致）；
    /// - `trigger`：窗口水位与策略（**由调用方持有** ⇒ ⇒ 水位可以跨进程持久化）。
    pub fn run_consolidation_incremental(
        &self,
        session_id: &str,
        now_sec: i64,
        trigger: &ConsolidationTrigger,
    ) -> Result<ConsolidationOutcome, MemoryError> {
        // 取料上限跟策略走（至少 100 条，与老方法同量级）⇒ ⇒ 窗口内的新料不会被截断。
        let limit = trigger.policy().max_episodes.max(100);
        let episodes = self.recent_episodes_for(session_id, limit)?;
        trigger.run(
            session_id,
            &episodes,
            now_sec,
            Some(self.governance.as_ref()),
        )
    }

    /// Forget an episode via the governance sidecar.
    pub fn forget_episode(
        &self,
        episode_id: &str,
        reason: Option<&str>,
        expected_rev: i64,
    ) -> Result<GovernedEpisode, MemoryGovernanceError> {
        let result = self
            .governance
            .forget_episode(episode_id, reason, expected_rev);
        match &result {
            Ok(_) => {
                // Applied forget: one forget event plus the state-change record
                // the governance revision bump wrote.
                self.emit_audit_event(AuditEvent::forget(
                    "memory_governance",
                    episode_id,
                    "forget applied",
                ));
                self.emit_audit_event(AuditEvent::state_change_for_forget(
                    "memory_governance",
                    episode_id,
                    episode_id,
                    "governance revision advanced by forget",
                ));
            }
            Err(MemoryGovernanceError::AlreadyForgotten(_)) => {
                // A repeat forget is observable as a forget event with no new
                // state-change record: the idempotence contract.
                self.emit_audit_event(AuditEvent::forget(
                    "memory_governance",
                    episode_id,
                    "forget repeated on an already-forgotten subject",
                ));
            }
            // A refusal (protected, revision conflict, unknown subject) applies
            // no forget and emits nothing.
            Err(_) => {}
        }
        let result = result?;
        {
            let mut working_lock = self
                .working
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for ring in working_lock.values_mut() {
                ring.retain(|ep| ep.id != episode_id);
            }
        }
        Ok(result)
    }

    /// Protect an episode from automatic purging or forgetting.
    pub fn protect_episode(
        &self,
        episode_id: &str,
        expected_rev: i64,
    ) -> Result<GovernedEpisode, MemoryGovernanceError> {
        let result = self.governance.protect_episode(episode_id, expected_rev);
        if result.is_ok() {
            self.emit_audit_event(AuditEvent::protect(
                "memory_governance",
                episode_id,
                "protect marker set",
            ));
            self.emit_audit_event(AuditEvent::state_change(
                "memory_governance",
                episode_id,
                "protect",
                "governance revision advanced by protect",
            ));
        }
        result
    }

    /// Unprotect an episode.
    pub fn unprotect_episode(
        &self,
        episode_id: &str,
        expected_rev: i64,
    ) -> Result<GovernedEpisode, MemoryGovernanceError> {
        let result = self.governance.unprotect_episode(episode_id, expected_rev);
        if result.is_ok() {
            self.emit_audit_event(AuditEvent::unprotect(
                "memory_governance",
                episode_id,
                "protect marker cleared",
            ));
            self.emit_audit_event(AuditEvent::state_change(
                "memory_governance",
                episode_id,
                "unprotect",
                "governance revision advanced by unprotect",
            ));
        }
        result
    }

    /// Update an episode's content override.
    pub fn update_episode_content(
        &self,
        episode_id: &str,
        new_content: &str,
        updated_by: Option<&str>,
        expected_rev: i64,
    ) -> Result<GovernedEpisode, MemoryGovernanceError> {
        let result = self.governance.update_episode_content(
            episode_id,
            new_content,
            updated_by,
            expected_rev,
        )?;
        if let Ok(Some(mut meta)) = self.backend.get_episode_metadata(episode_id) {
            let new_hash = crate::canonical::vector::content_hash(new_content);
            if let Some(obj) = meta.as_object_mut() {
                obj.insert(
                    "content_hash".to_string(),
                    serde_json::Value::String(new_hash),
                );
                obj.remove("vector");
                let _ = self.backend.put_episode_metadata(episode_id, meta);
            }
        }
        Ok(result)
    }

    pub fn episode_scope_metadata(&self, episode: &Episode) -> (MemoryScope, MemoryProvenance) {
        let fallback_scope = MemoryScope::Session {
            session_id: episode.session_id.clone(),
        };
        let fallback_provenance = MemoryProvenance {
            source_session: Some(episode.session_id.clone()),
            ..MemoryProvenance::default()
        };
        let Ok(Some(metadata)) = self.backend.get_episode_metadata(&episode.id) else {
            return (fallback_scope, fallback_provenance);
        };
        let scope = metadata
            .get("scope")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or(fallback_scope);
        let provenance = metadata
            .get("provenance")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or(fallback_provenance);
        (scope, provenance)
    }
}

fn typed_scope_visible(scope: &MemoryScope, identity: &TypedRecallIdentity) -> bool {
    matches!(scope, MemoryScope::User { user_id } if user_id == &identity.subject_id)
        || matches!(scope, MemoryScope::Persona { persona_id, user_id } if persona_id == &identity.persona_id && user_id == &identity.subject_id)
}

fn valid_embedding(vector: &[f32], expected_len: Option<usize>) -> bool {
    !vector.is_empty()
        && expected_len.is_none_or(|len| vector.len() == len)
        && vector.iter().all(|value| value.is_finite())
}

fn block_on_future<F>(f: F) -> F::Output
where
    F: std::future::Future + Send,
    F::Output: Send,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => match handle.runtime_flavor() {
            tokio::runtime::RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(|| handle.block_on(f))
            }
            _ => std::thread::scope(|s| {
                s.spawn(|| {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("local runtime")
                        .block_on(f)
                })
                .join()
                .expect("thread join")
            }),
        },
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("local runtime")
            .block_on(f),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layers::MemoryLayerKind;

    /// **统一入口**的行为：挂了驱动 ⇒ 走增量语义；**没挂驱动 ⇒ 走旧路径**。
    #[test]
    fn unified_entry_falls_back_to_the_old_path_without_a_trigger() {
        use crate::consolidation_trigger::{ConsolidationPolicy, ConsolidationTrigger};
        use std::sync::Arc;

        const SESSION: &str = "unified-entry-session";
        let coordinator = coordinator();
        for index in 0..2 {
            coordinator
                .writeback_episode(&Episode {
                    id: format!("ep-uni-{index}"),
                    timestamp: 1_000 + index,
                    role: "user".to_string(),
                    content: format!("fixed: 统一入口 {index}"),
                    session_id: SESSION.to_string(),
                })
                .expect("writeback");
        }

        // 没挂驱动 ⇒ 旧路径：每次都整编（哪怕没有新料）⇒ 洞察照样出得来
        let plain = coordinator
            .run_consolidation_at(SESSION, 2_000)
            .expect("plain run");
        assert!(
            plain
                .extracted_insights
                .iter()
                .any(|item| item.contains("统一入口")),
            "没挂驱动 ⇒ 旧路径照样整编"
        );

        // 挂上驱动 ⇒ 增量语义：第一次整编后，**没有新料就整不出东西**
        let trigger = ConsolidationTrigger::new(
            ConsolidationPolicy::new(0).with_min_episodes(1),
            Arc::new(crate::consolidation_trigger::InMemoryConsolidationWatermark::new()),
        );
        let coordinator = coordinator.with_consolidation_trigger(trigger);
        let first = coordinator
            .run_consolidation_at(SESSION, 3_000)
            .expect("incremental run");
        assert!(
            first
                .extracted_insights
                .iter()
                .any(|item| item.contains("统一入口")),
            "挂了驱动 ⇒ 第一轮该整编出新料"
        );
        let second = coordinator
            .run_consolidation_at(SESSION, 3_100)
            .expect("second incremental run");
        assert!(
            second.extracted_insights.is_empty(),
            "挂了驱动 ⇒ 没有新料就不该再整编出洞察：{:?}",
            second.extracted_insights
        );
    }

    /// 增量整编入口的三个关键行为：**整编一次** / **窗口内跳过** / **只整新料**。
    #[test]
    fn incremental_consolidation_runs_once_per_window_and_only_on_new_material() {
        use crate::consolidation_trigger::{
            ConsolidationOutcome, ConsolidationPolicy, ConsolidationTrigger,
            InMemoryConsolidationWatermark,
        };
        use std::sync::Arc;

        const SESSION: &str = "incremental-consolidation-session";
        let coordinator = coordinator();
        let watermark = Arc::new(InMemoryConsolidationWatermark::new());
        let trigger = ConsolidationTrigger::new(
            ConsolidationPolicy::new(86_400).with_min_episodes(2),
            Arc::clone(&watermark) as Arc<dyn crate::consolidation_trigger::ConsolidationWatermark>,
        );

        // 首批 3 条（内容含 "fixed:" ⇒ 可被提炼成洞察）
        for index in 0..3 {
            coordinator
                .writeback_episode(&Episode {
                    id: format!("ep-inc-{index}"),
                    timestamp: 1_000 + index,
                    role: "user".to_string(),
                    content: format!("fixed: 首批成果 {index}"),
                    session_id: SESSION.to_string(),
                })
                .expect("writeback");
        }

        // 第一轮：整编
        let first = coordinator
            .run_consolidation_incremental(SESSION, 2_000, &trigger)
            .expect("first run");
        assert!(first.consolidated(), "第一轮该整编：{first:?}");
        assert_eq!(first.episode_count(), 3);

        // 第二轮：同一窗口内 ⇒ 跳过（不重复整编）
        let second = coordinator
            .run_consolidation_incremental(SESSION, 2_000 + 600, &trigger)
            .expect("second run");
        assert!(!second.consolidated(), "窗口内不该重复整编：{second:?}");

        // 新增 2 条 ⇒ 窗口到期后只整新料
        for index in 0..2 {
            coordinator
                .writeback_episode(&Episode {
                    id: format!("ep-inc-late-{index}"),
                    timestamp: 100_000 + index,
                    role: "user".to_string(),
                    content: format!("fixed: 后来的成果 {index}"),
                    session_id: SESSION.to_string(),
                })
                .expect("writeback");
        }
        let third = coordinator
            .run_consolidation_incremental(SESSION, 200_000, &trigger)
            .expect("third run");
        assert!(third.consolidated(), "窗口到期后该整编：{third:?}");
        assert_eq!(third.episode_count(), 2, "只该整新料");
        match third {
            ConsolidationOutcome::Consolidated { report, .. } => {
                assert!(
                    report
                        .extracted_insights
                        .iter()
                        .all(|item| !item.contains("首批成果")),
                    "老素材不该再被提炼"
                );
                assert!(
                    report
                        .extracted_insights
                        .iter()
                        .any(|item| item.contains("后来的成果")),
                    "新素材该被提炼"
                );
            }
            ConsolidationOutcome::Skipped { .. } => unreachable!(),
        }
    }

    /// 检索会话 (固定 ⇒ 时间探针固定 ⇒ 结果可复现).
    const SESSION: &str = "wiki-wiring-session";
    /// 语料命中标记 (纯 ASCII ⇒ 任何分词口径下都是**整词**命中).
    const MARKER: &str = "wikimarker";

    /// 建一个**进程内**检索器 (零磁盘依赖 ⇒ 便于断言「结果不变」).
    fn coordinator() -> MemoryCoordinator {
        let store = Arc::new(crate::fresh_store());
        let backend = store.clone();
        let governance: Arc<dyn MemoryGovernanceStore> = store;
        MemoryCoordinator::new(backend, governance)
    }

    /// 写一条**可被召回**的记忆 (id 与时间戳固定 ⇒ 两次召回逐字节可比).
    fn seed_memory(coordinator: &MemoryCoordinator) {
        coordinator
            .writeback_episode(&Episode {
                id: "ep-wiki-wiring-fixed".to_string(),
                timestamp: 1_800_000_000,
                role: "user".to_string(),
                content: format!("用户知识库检索标记 {MARKER}"),
                session_id: SESSION.to_string(),
            })
            .expect("writeback episode");
    }

    /// ⭐ **联想召回的夹具**（2026-11-05）：一个**只认固定联想**的 `AssociationStore`
    /// ⇒ ⇒ 用来证明「联想候选真的进了候选集」，而不是「代码写了但没生效」。
    #[derive(Default)]
    struct FakeAssociations {
        edges: std::sync::Mutex<Vec<apeireth_plugin::experience::AssociationEdge>>,
    }

    impl apeireth_plugin::experience::AssociationStore for FakeAssociations {
        fn record_cooccurrence(
            &self,
            _from: &str,
            _to: &str,
            _episode_id: &str,
        ) -> apeireth_plugin::CapabilityResult<()> {
            Ok(())
        }

        fn top_associations(
            &self,
            entity: &str,
            limit: u32,
        ) -> apeireth_plugin::CapabilityResult<Vec<apeireth_plugin::experience::AssociationEdge>>
        {
            let edges = self.edges.lock().expect("fake associations mutex");
            Ok(edges
                .iter()
                .filter(|e| e.from_entity == entity || e.to_entity == entity)
                .take(limit as usize)
                .cloned()
                .collect())
        }
    }

    /// **空图**（测试夹具）：联想召回只需要 `AssociationStore`
    /// ⇒ ⇒ 图那一侧给一个**什么都不返回**的实现（证明「联想」与「图」是两条独立的面）。
    struct EmptyKnowledgeGraph;

    impl apeireth_plugin::experience::KnowledgeGraphStore for EmptyKnowledgeGraph {
        fn put_fact(
            &self,
            _fact: &apeireth_plugin::experience::GraphFact,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn put_link(
            &self,
            _link: &apeireth_plugin::experience::GraphLink,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn facts_from(
            &self,
            _subject_id: &str,
            _limit: u32,
        ) -> apeireth_plugin::CapabilityResult<Vec<apeireth_plugin::experience::GraphFact>>
        {
            Ok(Vec::new())
        }

        fn links_from(
            &self,
            _from_id: &str,
            _limit: u32,
        ) -> apeireth_plugin::CapabilityResult<Vec<apeireth_plugin::experience::GraphLink>>
        {
            Ok(Vec::new())
        }

        fn forget_subject(&self, _subject_id: &str) -> apeireth_plugin::CapabilityResult<()> {
            Ok(())
        }
    }

    /// ⭐ **带时间的图**（2026-11-05 A2 夹具）：两条事实，**有效期不同**
    /// ⇒ ⇒ 一条「一直有效」· 一条「只在过去有效」⇒ ⇒ 用来验证 **as-of 锚点**。
    struct TimedGraph {
        facts: Vec<apeireth_plugin::experience::GraphFact>,
    }

    impl apeireth_plugin::experience::KnowledgeGraphStore for TimedGraph {
        fn put_fact(
            &self,
            _fact: &apeireth_plugin::experience::GraphFact,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn put_link(
            &self,
            _link: &apeireth_plugin::experience::GraphLink,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn facts_from(
            &self,
            _subject_id: &str,
            _limit: u32,
        ) -> apeireth_plugin::CapabilityResult<Vec<apeireth_plugin::experience::GraphFact>>
        {
            // 模拟**旧语义**：「现在有效」⇒ ⇒ **忽略时间锚点**。
            let now = chrono::Utc::now().timestamp_millis();
            Ok(self
                .facts
                .iter()
                .filter(|f| f.valid_until.is_none_or(|u| u > now))
                .cloned()
                .collect())
        }

        fn facts_from_as_of(
            &self,
            _subject_id: &str,
            _limit: u32,
            as_of_ms: i64,
        ) -> apeireth_plugin::CapabilityResult<Vec<apeireth_plugin::experience::GraphFact>>
        {
            Ok(self
                .facts
                .iter()
                .filter(|f| apeireth_plugin::experience::fact_valid_at(f, as_of_ms))
                .cloned()
                .collect())
        }

        fn links_from(
            &self,
            _from_id: &str,
            _limit: u32,
        ) -> apeireth_plugin::CapabilityResult<Vec<apeireth_plugin::experience::GraphLink>>
        {
            Ok(Vec::new())
        }

        fn forget_subject(&self, _subject_id: &str) -> apeireth_plugin::CapabilityResult<()> {
            Ok(())
        }
    }

    /// ⭐ **时间锚点对图事实生效**（2026-11-05 A2）
    ///
    /// 接线前：`facts_from` 的语义 = 「**现在**有效」（store 内部硬编码 now）
    /// ⇒ ⇒ 带 `as_of_ms` 的查询 ⇒ ⇒ **问过去、答现在** ⇒ ⇒ **答非所问**。
    /// ⇒ ⇒ 本测试证明：**带 as-of 的查询只拿到「当时有效」的事实**。
    #[test]
    fn graph_facts_respect_the_as_of_anchor() {
        use apeireth_plugin::experience::GraphFact;

        const T_PAST: i64 = 1_700_000_000_000; // 2023-11
        const T_LATER: i64 = 1_800_000_000_000; // 2027-01
        let coordinator = coordinator();
        let graph = TimedGraph {
            facts: vec![
                GraphFact {
                    id: "fact-always".into(),
                    subject_id: "rust".into(),
                    subject_kind: "language".into(),
                    predicate: "is".into(),
                    object_id: "fast".into(),
                    object_kind: "property".into(),
                    valid_from: T_PAST,
                    valid_until: None,
                    source_episode_id: "ep-1".into(),
                    confidence: 0.9,
                },
                GraphFact {
                    id: "fact-only-in-past".into(),
                    subject_id: "rust".into(),
                    subject_kind: "language".into(),
                    predicate: "had".into(),
                    object_id: "borrow-checker".into(),
                    object_kind: "property".into(),
                    valid_from: T_PAST,
                    // ⚠️ **必须早于「现在」** ⇒ ⇒ 否则它**现在也有效**
                    // ⇒ ⇒ 两条路径（`facts_from` = now 语义 vs `facts_from_as_of`）
                    // ⇒ ⇒ 结果一样 ⇒ ⇒ **测不出区别**（测试没牙）。
                    valid_until: Some(T_PAST + 3_600_000),
                    source_episode_id: "ep-2".into(),
                    confidence: 0.8,
                },
            ],
        };
        let coordinator =
            coordinator.with_experience(Arc::new(graph), Arc::new(FakeAssociations::default()));

        let base = crate::MemoryRecallQuery {
            session_id: SESSION.to_string(),
            query_text: "rust".to_string(),
            layers: vec![MemoryLayerKind::Relational],
            limit: 8,
            max_chars: 4_000,
            recency_decay_lambda: 0.0,
            min_score: 0.0,
            as_of_ms: None,
            visible_scopes: vec![crate::scope::MemoryScope::Session {
                session_id: SESSION.to_string(),
            }],
            exact_scope_authorization: false,
        };

        // ① **带 as-of（过去）** ⇒ ⇒ 只该看到「当时有效」的那条。
        let past = crate::MemoryRecallQuery {
            as_of_ms: Some(T_PAST),
            ..base.clone()
        };
        let past_result = coordinator.recall(&past).expect("recall as-of past");
        let past_ids: Vec<&str> = past_result.items.iter().map(|i| i.id.as_str()).collect();
        assert!(
            past_ids.contains(&"fact:rust:is:fast"),
            "当时有效的事实必须出现（**当时知道什么**）⇒ ⇒ 实际: {past_ids:?}"
        );
        assert!(
            past_ids.contains(&"fact:rust:had:borrow-checker"),
            "当时仍有效的事实必须出现 ⇒ ⇒ 实际: {past_ids:?}"
        );

        // ② **as-of 在那条事实作废之后** ⇒ ⇒ 它不该出现。
        let after = crate::MemoryRecallQuery {
            as_of_ms: Some(T_LATER + 1),
            ..base
        };
        let after_result = coordinator.recall(&after).expect("recall as-of later");
        let after_ids: Vec<&str> = after_result.items.iter().map(|i| i.id.as_str()).collect();
        assert!(
            !after_ids.contains(&"fact:rust:had:borrow-checker"),
            "**作废之后**的事实不该被召回 ⇒ ⇒ 实际: {after_ids:?}"
        );
    }

    /// ⭐ **联想召回接通**（2026-11-05）
    ///
    /// 接线前：`self.associations` 被装上，但**全 crate 零调用** `top_associations`
    /// ⇒ ⇒ 联想能力「算得出来但从不上场」。
    /// ⇒ ⇒ 本测试证明：**装了联想 store ⇒ 联想候选真的进了召回候选集**。
    #[test]
    fn associative_recall_surfaces_associated_episodes() {
        use apeireth_plugin::experience::AssociationEdge;

        const QUERY: &str = "数据库迁移";
        let coordinator = coordinator();
        coordinator
            .writeback_episode(&Episode {
                id: "ep-assoc-target".to_string(),
                timestamp: 1_900_000_000,
                role: "user".to_string(),
                content: "把 Postgres 迁到 SQLite 的那次改动".to_string(),
                session_id: SESSION.to_string(),
            })
            .expect("writeback episode");

        let fake = Arc::new(FakeAssociations::default());
        // ⚠️ **键 = 实体名**（不是整个问句）⇒ ⇒ 写入侧的 marker 也是实体名
        // ⇒ ⇒ 查表键必须与写入侧**同口径**（否则通道形同虚设）。
        fake.edges.lock().expect("lock").push(AssociationEdge {
            from_entity: "postgres".to_string(),
            to_entity: "迁移".to_string(),
            co_occurrence_count: 25,
            last_seen_episode_id: "ep-assoc-target".to_string(),
            last_seen_at: 1_900_000_000,
        });
        let coordinator = coordinator.with_experience(Arc::new(EmptyKnowledgeGraph), fake);

        let query = crate::MemoryRecallQuery {
            session_id: SESSION.to_string(),
            query_text: QUERY.to_string(),
            layers: vec![MemoryLayerKind::Relational],
            limit: 8,
            max_chars: 4_000,
            recency_decay_lambda: 0.0,
            min_score: 0.0,
            as_of_ms: None,
            // ⚠️ **可见域不能空**：`is_visible_in(&[])` = `any` over 空 = **false**
            // ⇒ ⇒ 空可见域 ⇒ ⇒ **所有**候选（含词法通道）全被筛掉。
            visible_scopes: vec![crate::scope::MemoryScope::Session {
                session_id: SESSION.to_string(),
            }],
            exact_scope_authorization: false,
        };
        let result = coordinator.recall(&query).expect("recall");
        assert!(
            result
                .items
                .iter()
                .any(|item| item.id == "assoc:ep-assoc-target"),
            "联想候选必须上场（**接线前它从不上场**）⇒ ⇒ 实际拿到: {:?}",
            result.items.iter().map(|i| &i.id).collect::<Vec<_>>()
        );
    }

    /// ⭐ **没装联想 ⇒ ⇒ 行为逐字节不变**（零回归守）
    #[test]
    fn associative_recall_is_absent_without_store() {
        let coordinator = coordinator();
        coordinator
            .writeback_episode(&Episode {
                id: "ep-assoc-absent".to_string(),
                timestamp: 1_900_000_000,
                role: "user".to_string(),
                content: "数据库迁移的内容".to_string(),
                session_id: SESSION.to_string(),
            })
            .expect("writeback episode");
        let query = crate::MemoryRecallQuery {
            session_id: SESSION.to_string(),
            query_text: "数据库迁移".to_string(),
            layers: vec![MemoryLayerKind::Relational],
            limit: 8,
            max_chars: 4_000,
            recency_decay_lambda: 0.0,
            min_score: 0.0,
            as_of_ms: None,
            // ⚠️ **可见域不能空**：`is_visible_in(&[])` = `any` over 空 = **false**
            // ⇒ ⇒ 空可见域 ⇒ ⇒ **所有**候选（含词法通道）全被筛掉。
            visible_scopes: vec![crate::scope::MemoryScope::Session {
                session_id: SESSION.to_string(),
            }],
            exact_scope_authorization: false,
        };
        let result = coordinator.recall(&query).expect("recall");
        assert!(
            !result
                .items
                .iter()
                .any(|item| item.id.starts_with("assoc:")),
            "没装联想 store ⇒ ⇒ 不该有任何联想候选"
        );
    }

    /// 已写好记忆的检索器.
    fn seeded_coordinator() -> MemoryCoordinator {
        let coordinator = coordinator();
        seed_memory(&coordinator);
        coordinator
    }

    fn recall(coordinator: &MemoryCoordinator) -> MemoryRecallResult {
        let mut request = MemoryRecallQuery::new(SESSION, MARKER)
            .with_layers(vec![MemoryLayerKind::Episodic])
            .with_limit(5)
            .with_as_of_ms(1_800_000_000_000);
        // 下限归零: 本组用例关心的是**附加证据**, 不是打分阈值。
        request.min_score = 0.0;
        coordinator.recall(&request).expect("recall should succeed")
    }

    /// 临时语料根 (进程 id + 用例名 ⇒ 同进程内不撞名; 用完清掉).
    fn temp_wiki_root(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("apeireth-wiki-{tag}-{}", std::process::id()))
    }

    /// **不注入**语料根 ⇒ 结果里没有命中节 (缺省零变化的第一层)。
    #[test]
    fn recall_without_wiki_root_carries_no_wiki_section() {
        let coordinator = coordinator();
        seed_memory(&coordinator);
        assert!(
            coordinator.wiki_root().is_none(),
            "缺省 = 检索路径没有语料根"
        );

        let recalled = recall(&coordinator);

        assert!(
            recalled.wiki_sections.is_empty(),
            "未注入语料根 ⇒ 不得凭空长出命中节: {:?}",
            recalled.wiki_sections
        );
        assert!(
            !recalled.items.is_empty(),
            "召回本身必须照常命中 (否则这条用例是空转)"
        );
    }

    /// 注入**不存在的**目录 ⇒ 优雅降级: 结果与未注入**逐字节一致**，且不 panic。
    #[test]
    fn missing_wiki_directory_degrades_to_the_unchanged_recall() {
        let plain = recall(&seeded_coordinator());

        let missing = temp_wiki_root("missing").join("no-such-dir");
        let degraded = recall(&seeded_coordinator().with_wiki_root(missing));

        assert!(
            degraded.wiki_sections.is_empty(),
            "目录不存在 ⇒ 命中节为空: {:?}",
            degraded.wiki_sections
        );
        assert_eq!(
            serde_json::to_string(&plain).expect("serialize plain"),
            serde_json::to_string(&degraded).expect("serialize degraded"),
            "语料根读不到东西 ⇒ 召回结果逐字节不变"
        );
    }

    /// 注入**在场**的语料根 ⇒ 命中节作为**附加**证据附在结果上 (不改条目)。
    #[test]
    fn wiki_corpus_sections_ride_along_as_additional_evidence() {
        let root = temp_wiki_root("hit");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create wiki root");
        std::fs::write(
            root.join("knowledge.md"),
            format!("# 项目背景 {MARKER}\n\n这是用户知识库语料的检索标记段落。\n"),
        )
        .expect("write wiki markdown");

        let plain = recall(&seeded_coordinator());
        let wired = recall(&seeded_coordinator().with_wiki_root(root.clone()));

        assert!(!wired.wiki_sections.is_empty(), "语料在场 ⇒ 结果应带命中节");
        let section = wired
            .wiki_sections
            .iter()
            .find(|section| section.heading.contains("项目背景"))
            .expect("应命中含中文标题的节");
        assert!(
            section.source_path.ends_with("knowledge.md"),
            "出处应指向语料文件: {:?}",
            section.source_path
        );
        assert!(
            section.heading.contains(MARKER),
            "标题应保留原文: {:?}",
            section.heading
        );
        assert!(
            section.text.contains("检索标记段落"),
            "正文应带该节内容: {:?}",
            section.text
        );
        assert_eq!(
            serde_json::to_string(&plain.items).expect("serialize plain items"),
            serde_json::to_string(&wired.items).expect("serialize wired items"),
            "命中节是附加证据 ⇒ 不得改动召回条目"
        );

        std::fs::remove_dir_all(&root).expect("cleanup wiki root");
    }

    /// 治理查询的**探针替身**：只改 `get_governed` 的返回，其余方法原样转交。
    ///
    /// ⚠️ 用途：把「**查不到**」的两种含义分开测 ——
    /// `Err`（查询失败 ⇒ 不可判定）vs `Ok(None)`（查无治理记录）。
    enum GovernanceProbeMode {
        /// 查询失败（`Err`）。
        Failing,
        /// 查无治理记录（`Ok(None)`）。
        Missing,
        /// 有治理记录且已遗忘（`Ok(Some(..))` + `Forgotten`）。
        Forgotten,
    }

    struct GovernanceProbe {
        inner: Arc<dyn MemoryGovernanceStore>,
        mode: GovernanceProbeMode,
    }

    impl GovernanceProbe {
        fn new(inner: Arc<dyn MemoryGovernanceStore>, mode: GovernanceProbeMode) -> Self {
            Self { inner, mode }
        }
    }

    impl MemoryGovernanceStore for GovernanceProbe {
        fn get_governed(
            &self,
            episode_id: &str,
        ) -> Result<Option<GovernedEpisode>, MemoryGovernanceError> {
            match &self.mode {
                GovernanceProbeMode::Failing => Err(MemoryGovernanceError::Invalid(
                    "governance query failed (probe)".into(),
                )),
                GovernanceProbeMode::Missing => Ok(None),
                GovernanceProbeMode::Forgotten => Ok(Some(GovernedEpisode {
                    episode: Episode {
                        id: episode_id.to_string(),
                        timestamp: 1_800_000_000,
                        role: "user".to_string(),
                        content: String::new(),
                        session_id: SESSION.to_string(),
                    },
                    status: MemoryGovernanceStatus::Forgotten,
                    protected: false,
                    content_override: None,
                    revision: 1,
                    updated_at: None,
                    updated_by: None,
                    forgotten_at: Some(1),
                })),
            }
        }

        fn update_episode_content(
            &self,
            episode_id: &str,
            new_content: &str,
            updated_by: Option<&str>,
            expected_rev: i64,
        ) -> Result<GovernedEpisode, MemoryGovernanceError> {
            self.inner
                .update_episode_content(episode_id, new_content, updated_by, expected_rev)
        }

        fn forget_episode(
            &self,
            episode_id: &str,
            reason: Option<&str>,
            expected_rev: i64,
        ) -> Result<GovernedEpisode, MemoryGovernanceError> {
            self.inner.forget_episode(episode_id, reason, expected_rev)
        }

        fn protect_episode(
            &self,
            episode_id: &str,
            expected_rev: i64,
        ) -> Result<GovernedEpisode, MemoryGovernanceError> {
            self.inner.protect_episode(episode_id, expected_rev)
        }

        fn unprotect_episode(
            &self,
            episode_id: &str,
            expected_rev: i64,
        ) -> Result<GovernedEpisode, MemoryGovernanceError> {
            self.inner.unprotect_episode(episode_id, expected_rev)
        }

        fn governed_recent_episodes(
            &self,
            session_id: &str,
            n: usize,
        ) -> Result<Vec<GovernedEpisode>, MemoryGovernanceError> {
            self.inner.governed_recent_episodes(session_id, n)
        }

        fn governed_query(
            &self,
            q: &crate::EpisodeQuery,
        ) -> Result<Vec<GovernedEpisode>, MemoryGovernanceError> {
            self.inner.governed_query(q)
        }
    }

    /// 建一个**带治理探针**的检索器（探针只影响 `get_governed`）。
    fn coordinator_with_governance_probe(mode: GovernanceProbeMode) -> MemoryCoordinator {
        let store = Arc::new(crate::fresh_store());
        let backend = store.clone();
        let governance: Arc<dyn MemoryGovernanceStore> =
            Arc::new(GovernanceProbe::new(store, mode));
        MemoryCoordinator::new(backend, governance)
    }

    /// 写一条**能被召回**的记忆（正文含检索标记 ⇒ 不治理也会命中）。
    fn seed_marker_episode(coordinator: &MemoryCoordinator, id: &str) {
        coordinator
            .writeback_episode(&Episode {
                id: id.to_string(),
                timestamp: 1_800_000_000,
                role: "user".to_string(),
                content: format!("{MARKER} 治理面探针"),
                session_id: SESSION.to_string(),
            })
            .expect("writeback episode");
    }

    /// **治理查询失败 ⇒ 该 episode 不得进召回**（fail-closed 回归）。
    ///
    /// 旧实现把 `Err` 当成「无治理 ⇒ 放行」⇒ ⇒ 被遗忘的条目会漏回召回面。
    #[test]
    fn a_failing_governance_query_keeps_the_episode_out_of_recall() {
        let coordinator = coordinator_with_governance_probe(GovernanceProbeMode::Failing);
        seed_marker_episode(&coordinator, "ep-governance-probe-failing");

        let recalled = recall(&coordinator);

        assert!(
            recalled.items.is_empty(),
            "治理查询失败 ⇒ 不可判定 ⇒ 该 episode 不得进召回: {:?}",
            recalled.items
        );
        assert!(
            recalled.governance_filtered >= 1,
            "被治理面过滤掉的条目必须计数（可观测）: {}",
            recalled.governance_filtered
        );
    }

    /// **查无治理记录（`Ok(None)`）⇒ 仍然放行**（存量行默认 active 的既有口径）。
    ///
    /// ⇒ ⇒ 钉住「查不到」的两种含义被**分开**处理：`Err` 过滤 / `Ok(None)` 放行。
    #[test]
    fn a_missing_governance_record_still_reaches_the_recall() {
        let coordinator = coordinator_with_governance_probe(GovernanceProbeMode::Missing);
        seed_marker_episode(&coordinator, "ep-governance-probe-missing");

        let recalled = recall(&coordinator);

        assert!(
            !recalled.items.is_empty(),
            "查无治理记录 ≠ 查询失败 ⇒ 不得误伤正常条目: {:?}",
            recalled.items
        );
        assert_eq!(recalled.governance_filtered, 0, "没有任何治理面过滤发生");
    }

    /// **已遗忘 ⇒ 不得进召回**（既有语义，回归钉住）。
    #[test]
    fn a_forgotten_episode_never_reaches_the_recall() {
        let coordinator = coordinator_with_governance_probe(GovernanceProbeMode::Forgotten);
        seed_marker_episode(&coordinator, "ep-governance-probe-forgotten");

        let recalled = recall(&coordinator);

        assert!(
            recalled.items.is_empty(),
            "被遗忘的 episode 不得进召回: {:?}",
            recalled.items
        );
        assert!(recalled.governance_filtered >= 1);
    }
}

/// recency 的**时间差**（小时）。
///
/// # 两种口径（**关键**）
///
/// * **没有时间锚点** ⇒ `(now - ts).max(0)` ⇒ ⇒ **与接线前逐字一致**（**零行为变化**）
///   ⇒ ⇒ 未来时间戳的记忆仍然拿 recency = 1.0（**保持原语义**）；
/// * **有相对时间锚点**（「3年前」）⇒ `|reference - ts|`
///   ⇒ ⇒ **到参照点的距离** ⇒ ⇒ 「3年前」能分出「3 年前」和「10 年前」
///   ⇒ ⇒ ⚠️ **这是实测抓出来的缺陷**：`max(0)` 会让**所有比参照点更老**的记忆
///   ⇒ ⇒ 拿到同一个 recency = 1.0 ⇒ ⇒ 锚点**只在「参照点与现在之间」有效**。
fn recency_delta_hours(reference_ms: Option<i64>, now_ms: i64, timestamp_ms: i64) -> f64 {
    let delta_ms = match reference_ms {
        Some(reference) => (reference - timestamp_ms).abs(),
        None => (now_ms - timestamp_ms).max(0),
    };
    delta_ms as f64 / (1000.0 * 3600.0)
}
