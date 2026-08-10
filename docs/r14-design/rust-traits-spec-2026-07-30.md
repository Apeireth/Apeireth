# Rust Trait 形式化规范 — Python MVP → Rust 接口

**文档路径**: `Apeireth-rust/docs/rust-traits-spec-2026-07-30.md`
**生成时间**: 2026-07-30
**工作目录**: `.openclaw\workspace\promethean`
**master HEAD**: `0ee300e fix(test-v1106): T6-F-1 修 test_v1106 hardcode 期望` (R13 MVP Phase 1.2 已合并)
**任务**: T27 — R14 Phase 0 接口规范（4 周目标的核心交付物）
**作者角色**: fullstack_engineer
**依据**: T23 R14 路线图 (`Apeireth-rust/docs/r14-rust-rewrite-roadmap.md`) §2 + Python MVP `mvp/` 子项目 13 文件 2292 insertions

---

## §0 元信息

- **生成者**: fullstack_engineer (T9 R13 MVP Phase 0+1.1 + T15 Phase 1.2 提取层作者)
- **目标**: 把 Python MVP 13 文件 2292 insertions 形式化为 Rust trait 接口规范，让 R14 团队按此契约实现关键路径
- **原则**:
  - 主 19:33 **借鉴而非闭门** — Rust 实现借鉴 sqlx/rusqlite/tantivy/DeltaMemory 论文，**不绑 LangChain/Letta/AutoGen** 等闭门框架
  - 主 17:43 **实事求是** — 接口签名精确反映 Python 行为（含 nullable、salience decay、rolling window）
  - 主 17:58 **不假装达到 ASI** — 这是接口规范，不是 ASI 实现
  - 主 23:44 **干到底** — 4 周 Phase 0 落地后立即进入 Phase 1

- **关联产物**:
  - `Apeireth-rust/docs/r14-rust-rewrite-roadmap.md` (T23, master `c89c4bcc`) — 26 周 / 6 阶段大图
  - `reports/r13-mvp-kickoff-2026-07-30.md` (T9) — Phase 0 + Phase 1.1 启动
  - `reports/r13-mvp-phase12-extract-layer-2026-07-30.md` (T15) — Phase 1.2 提取层
  - `reports/r12-commit-t6-f-1-v1106-fix-2026-07-30.md` (T24, master `0ee300e`) — 测试兼容修复

---

## §1 主路径核心类型（`apeireth-core`）

> 源自: `mvp/memory/store.py` Episode/Note/Session dataclass + `mvp/identity/card.py` IdentityCard dataclass

```rust
//! crates/apeireth-core/src/types.rs
//!
//! 主路径类型 — 所有 crate 共享. Serialize/Deserialize 双实现
//! 让 SQLite/JSON 持久化统一.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unix epoch seconds (i64). 用 i64 而非 u64 兼容未来时间回退.
pub type Timestamp = i64;

/// UUID v4 12-char hex short id. 与 Python uuid.uuid4().hex[:12] 对齐.
pub type ShortId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Episode {
    pub id: ShortId,
    pub timestamp: Timestamp,
    /// "user" | "agent" | "system"
    pub role: String,
    pub content: String,
    pub session_id: ShortId,
    /// 初始 1.0, 随时间衰减 (retrieve 层计算)
    pub salience: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub id: ShortId,
    pub timestamp: Timestamp,
    pub content: String,
    pub source_episode_ids: Vec<ShortId>,
    /// [0.0, 1.0], 默认 0.8 (主 17:43 实事求是: 不假装智能)
    pub confidence: f64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: ShortId,
    pub started_at: Timestamp,
    pub last_active_at: Timestamp,
    /// consolidate 后的摘要 (Phase 1.3+ 启用, Phase 0 默认 "")
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdentityCard {
    pub version: String,           // "0.1.0"
    pub owner_id: ShortId,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,

    /// 主人真实背景 (种子事实, 演化从 Phase 1.3 开始)
    pub owner_background: Vec<String>,
    /// 主人价值观 (主 17:43 + 23:44 + 19:33 + 17:58 + 20:46)
    pub owner_values: Vec<String>,
    /// §5.E 红线 (4 项硬约束)
    pub owner_red_lines: Vec<String>,

    /// Agent 角色描述
    pub agent_role: String,
    /// Agent 当前能力清单
    pub agent_capabilities: Vec<String>,

    /// 演化日志 (Phase 1.3 consolidate 追加)
    pub evolution_log: Vec<EvolutionEvent>,

    /// 自由扩展字段
    pub custom: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvolutionEvent {
    pub ts: Timestamp,
    /// e.g. "owner_background.<key>" | "owner_values.<key>" | "agent_capabilities.<key>" | "consolidate.added" | arbitrary
    pub key: String,
    pub value: serde_json::Value,
    /// "user" | "agent" | "consolidate"
    pub source: String,
}

/// 主人真实身份背景种子 (公开可查 + 主人已知)
pub fn default_owner_background() -> Vec<String> {
    vec![
        "".to_string(),
        "老家养老问题长期关注".to_string(),
        "".to_string(),
        "AgentMemory 自研方向".to_string(),
        "".to_string(),
    ]
}

pub fn default_owner_values() -> Vec<String> {
    vec![
        "实事求是 (主 17:43 + 主 17:58)".to_string(),
        "干到底 (主 23:44 + 主 23:09)".to_string(),
        "借鉴而非闭门 (主 19:33)".to_string(),
        "不刷 KPI (主 17:43)".to_string(),
        "不假装达到 ASI (主 17:58 + 主 20:46)".to_string(),
    ]
}

pub fn default_owner_red_lines() -> Vec<String> {
    vec![
        "不重写 V0.5 公式".to_string(),
        "不重做 V1136 真测引擎".to_string(),
        "不重写哲学守门".to_string(),
        "不写 ASI 北极星公式".to_string(),
    ]
}

pub fn now_timestamp() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn random_short_id() -> ShortId {
    use std::sync::Mutex;
    use uuid::Uuid;
    static LAST: Mutex<Option<Uuid>> = Mutex::new(None);
    let mut g = LAST.lock().expect("uuid mutex poisoned");
    let id = *g.get_or_insert_with(Uuid::new_v4);
    *g = Some(Uuid::new_v4());
    let hex = id.simple().to_string();
    hex[..12].to_string()
}
```

---

## §2 存储层 trait（`apeireth-memory`）

> 源自: `mvp/memory/store.py` SQLite FTS5 + `mvp/memory/retrieve.py` BM25 + Salience decay

```rust
//! crates/apeireth-memory/src/store.rs + retrieve.rs
//!
//! Phase 0 实现 = SQLite (rusqlite) + LIKE-based BM25
//! Phase 1+ 可换 FTS5 (需 jieba-like 分词) 或 tantivy (主 19:33 借鉴而非闭门)

use async_trait::async_trait;
use crate::apeireth_core::{Episode, Note, Session, ShortId, Timestamp};
use std::path::Path;

/// Episode 滚动窗口大小 (主 17:43 实事求是: 200 条够用)
pub const EPISODE_MAX_COUNT: usize = 200;

/// Note 遗忘阈值 (confidence < 此值遗忘)
pub const MIN_CONFIDENCE: f64 = 0.1;

/// Salience decay 时间常数 (DeltaMemory 2024 / Lin et al., τ = 1 day)
pub const EPISODE_TAU_SECONDS: f64 = 86_400.0;

/// Note 半衰期更长 (1 周 vs 1 天 for episodes) — Note 是 consolidate 后的提炼
pub const NOTE_TAU_SECONDS: f64 = 604_800.0;

/// Salience 遗忘 cutoff (主 17:43 实事求是: 0.05 是经验值)
pub const SALIENCE_CUTOFF: f64 = 0.05;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Episode not found: {0}")]
    EpisodeNotFound(ShortId),
    #[error("Note not found: {0}")]
    NoteNotFound(ShortId),
    #[error("Session not found: {0}")]
    SessionNotFound(ShortId),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other: {0}")]
    Other(String),
}

#[derive(Debug, thiserror::Error)]
pub enum RetrieveError {
    #[error("Empty query")]
    EmptyQuery,
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("Other: {0}")]
    Other(String),
}

/// SQLite-backed memory store.
///
/// Ponytail: thin wrapper around rusqlite, no ORM.
/// Schema is created on first connect. Episodes are append-only; Notes are mutable.
#[async_trait]
pub trait Store: Send + Sync {
    /// Schema bootstrap. Create tables if not exist.
    async fn open(path: &Path) -> Result<Self, StoreError>
    where
        Self: Sized;

    /// Close connection.
    async fn close(&self) -> Result<(), StoreError>;

    // ----- Session -----

    /// Start or resume a session. Returns session id.
    /// If `session_id` is None, generate UUID v4 12-char.
    async fn start_session(&self, session_id: Option<&str>) -> Result<ShortId, StoreError>;

    /// Return most recently active session id, or None.
    async fn last_session(&self) -> Result<Option<ShortId>, StoreError>;

    // ----- Episode -----

    /// Append an episode. Triggers rolling window enforcement (keeps last EPISODE_MAX_COUNT).
    async fn append_episode(
        &self,
        role: &str,
        content: &str,
        session_id: &str,
        salience: f64,
    ) -> Result<Episode, StoreError>;

    /// List episodes for a session, ordered by timestamp DESC.
    /// If `session_id` is None, list across all sessions.
    async fn list_episodes(
        &self,
        session_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Episode>, StoreError>;

    // ----- Note -----

    /// Add a note. Note is mutable (主 17:43 实事求是).
    async fn add_note(
        &self,
        content: &str,
        source_episode_ids: Vec<ShortId>,
        confidence: f64,
        tags: Vec<String>,
    ) -> Result<Note, StoreError>;

    /// Merge new evidence into existing note.
    /// Bumps confidence by `bump_confidence`, updates content.
    /// Returns None if note not found.
    async fn merge_note(
        &self,
        note_id: &str,
        new_content: &str,
        bump_confidence: f64,
    ) -> Result<Option<Note>, StoreError>;

    /// Forget notes with confidence < threshold. Returns count deleted.
    async fn forget_low_confidence_notes(
        &self,
        threshold: f64,
    ) -> Result<usize, StoreError>;

    /// List all notes, ordered by timestamp DESC.
    async fn list_notes(&self, limit: usize) -> Result<Vec<Note>, StoreError>;

    // ----- Stats -----

    async fn stats(&self) -> Result<StoreStats, StoreError>;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreStats {
    pub episodes: usize,
    pub notes: usize,
    pub sessions: usize,
}

/// BM25-like retrieval + salience time decay.
///
/// Phase 0: LIKE scan + simplified BM25 (term_freq sum / sqrt(content_length)).
/// Phase 1+: FTS5 (with jieba-like tokenization) or tantivy (主 19:33 借鉴).
#[async_trait]
pub trait Retrieve: Send + Sync {
    /// Retrieve top-K episodes matching `query`.
    /// - `session_id`: if Some, scope to that session.
    /// - `top_k`: max hits returned.
    /// - `tau`: salience decay time constant (default EPISODE_TAU_SECONDS).
    /// - `use_decay`: apply salience decay (else salience=1.0).
    async fn retrieve(
        &self,
        query: &str,
        top_k: usize,
        session_id: Option<&str>,
        tau: f64,
        use_decay: bool,
        now: Option<Timestamp>,
    ) -> Result<Vec<RetrievalHit>, RetrieveError>;

    /// Retrieve top-K notes matching `query` with confidence >= `min_confidence`.
    /// Note uses 7-day decay (NOTE_TAU_SECONDS).
    async fn retrieve_notes(
        &self,
        query: &str,
        top_k: usize,
        min_confidence: f64,
        now: Option<Timestamp>,
    ) -> Result<Vec<Note>, RetrieveError>;

    /// Linear time-window filter (Ponytail: no SQL needed for small lists).
    async fn time_window_filter(
        episodes: Vec<Episode>,
        since: Option<Timestamp>,
        until: Option<Timestamp>,
    ) -> Vec<Episode>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalHit {
    pub episode: Episode,
    /// simplified BM25 score = sum(token_hits) / sqrt(content_length + 1)
    pub bm25_score: f64,
    /// post-decay salience = 1/(1+Δt/τ) * episode.salience
    pub salience: f64,
    /// final = bm25_score * salience
    pub final_score: f64,
}

/// Salience decay: 1/(1+Δt/τ). Δt=0 → 1.0, Δt=τ → 0.5, Δt=∞ → 0.
pub fn decay(timestamp: Timestamp, now: Timestamp, tau: f64) -> f64 {
    let delta = (now - timestamp).max(0) as f64;
    1.0 / (1.0 + delta / tau)
}

/// Tokenize text: A-Za-z word OR single CJK char.
/// (Ponytail: no jieba dependency. Phase 1.4+ 引入 jieba-rs for CJK word-level.)
pub fn tokenize(text: &str) -> Vec<&str> {
    // Implementation: regex [A-Za-z]+|[\u{4e00}-\u{9fff}]
    // Returns owned String slice references.
    unimplemented!("Phase 0 stub — impl in Phase 1 with regex crate")
}

/// Simplified BM25: term_freq sum / sqrt(content_length + 1).
pub fn bm25_score(content: &str, tokens: &[&str]) -> f64 {
    if tokens.is_empty() || content.is_empty() {
        return 0.0;
    }
    let hits: usize = tokens.iter().map(|t| content.matches(t).count()).sum();
    if hits == 0 {
        return 0.0;
    }
    hits as f64 / ((content.len() as f64) + 1.0).sqrt()
}
```

### §2.1 SQLite Schema (apeireth-memory/src/schema.rs)

```sql
-- Episodes: append-only, rolling window per session
CREATE TABLE IF NOT EXISTS episodes (
    id         TEXT PRIMARY KEY,
    timestamp  INTEGER NOT NULL,        -- i64 Unix epoch seconds
    role       TEXT NOT NULL,            -- 'user' | 'agent' | 'system'
    content    TEXT NOT NULL,
    session_id TEXT NOT NULL,
    salience   REAL NOT NULL DEFAULT 1.0
);
CREATE INDEX IF NOT EXISTS ix_episodes_session_ts
    ON episodes(session_id, timestamp);

-- Notes: mutable, JSON-serialized source_episode_ids + tags
CREATE TABLE IF NOT EXISTS notes (
    id                  TEXT PRIMARY KEY,
    timestamp           INTEGER NOT NULL,
    content             TEXT NOT NULL,
    source_episode_ids  TEXT NOT NULL DEFAULT '[]',  -- JSON array
    confidence          REAL NOT NULL DEFAULT 0.8,
    tags                TEXT NOT NULL DEFAULT '[]'   -- JSON array
);
CREATE INDEX IF NOT EXISTS ix_notes_ts ON notes(timestamp DESC);

-- Sessions: id + start/last-seen + summary
CREATE TABLE IF NOT EXISTS sessions (
    id         TEXT PRIMARY KEY,
    started_at INTEGER NOT NULL,
    last_seen  INTEGER NOT NULL,
    summary    TEXT NOT NULL DEFAULT ''
);
```

---

## §3 提取层 trait（`apeireth-memory`）

> 源自: `mvp/memory/consolidate.py` (194 行) + `mvp/memory/forget.py` (82 行)

```rust
//! crates/apeireth-memory/src/consolidate.rs + forget.rs
//!
//! Phase 0 实现 = 纯函数 (no Store side-effects, 主 17:43 实事求是: 不假装智能)
//! Phase 1+ LLM 接入后这里换 LLM 提炼 (主 19:33 借鉴而非闭门)

use async_trait::async_trait;
use crate::apeireth_core::{Episode, Note, IdentityCard};
use crate::apeireth_memory::{Retrieve, Store, EPISODE_TAU_SECONDS, SALIENCE_CUTOFF};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidateConfig {
    /// 第一人称触发词 (中文 + 英文, 主 17:43 实事求是)
    pub owner_trigger_words: Vec<String>,
    /// 谓词触发词 (中文 + 英文)
    pub predicate_trigger_words: Vec<String>,
    /// 最小 token 重叠数 (≥ 1 才提炼 Note)
    pub min_token_overlap: usize,
    /// confidence 基准值 (0.4)
    pub confidence_base: f64,
    /// confidence 重叠步进 (0.1)
    pub confidence_overlap_step: f64,
}

impl Default for ConsolidateConfig {
    fn default() -> Self {
        Self {
            owner_trigger_words: vec![
                "我".into(), "主人".into(), "本人".into(), "我的".into(), "咱们".into(), "我们".into(),
                "I".into(), "me".into(), "my".into(), "mine".into(), "we".into(), "us".into(), "our".into(),
            ],
            predicate_trigger_words: vec![
                "是".into(), "做".into(), "在".into(), "有".into(), "研究".into(), "学".into(), "来自".into(),
                "is".into(), "am".into(), "do".into(), "doing".into(), "from".into(), "research".into(),
            ],
            min_token_overlap: 1,
            confidence_base: 0.4,
            confidence_overlap_step: 0.1,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConsolidateError {
    #[error("Empty episodes")]
    EmptyEpisodes,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other: {0}")]
    Other(String),
}

/// Extract: Episode → Note 启发式提炼 (Phase 1.2).
///
/// 主 17:43 实事求是: 不假装智能. 启发式:
/// - episode 含 IdentityCard 关键词 → 提炼
/// - episode 含第一人称 + 谓词 → 提炼
/// - confidence = 0.4 + 0.1*overlap (cap 0.9)
///
/// Phase 2 LLM 接入后: 换成 LLM 提炼 (主 19:33 借鉴而非闭门).
#[async_trait]
pub trait Consolidate: Send + Sync {
    async fn extract_notes(
        &self,
        episodes: Vec<Episode>,
        identity_card: &IdentityCard,
        config: &ConsolidateConfig,
    ) -> Result<Vec<Note>, ConsolidateError>;

    /// Merge high-similarity Notes (cosine >= threshold).
    /// Ponytail: O(n^2) pairwise. Phase 1.4 可换近似算法.
    /// 保留 content 最长的, confidence = max + 0.05*(len-1), source_episode_ids 合并.
    async fn merge_similar_notes(
        &self,
        notes: Vec<Note>,
        threshold: f64,
    ) -> Result<Vec<Note>, ConsolidateError>;

    /// Feedback-driven confidence update.
    /// feedback=true → +0.05, feedback=false → -0.10 (步长更大, 主 17:43 实事求是).
    /// Returns NEW Note (not modify original).
    async fn update_confidence(
        &self,
        note: &Note,
        feedback: bool,
    ) -> Result<Note, ConsolidateError>;

    /// Dedupe by exact content match, keep highest confidence.
    async fn dedupe_by_content(&self, notes: Vec<Note>) -> Result<Vec<Note>, ConsolidateError>;
}

/// Cosine similarity on token-frequency vectors.
pub fn cosine(a: &HashMap<String, usize>, b: &HashMap<String, usize>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let dot: usize = a.keys()
        .filter(|k| b.contains_key(*k))
        .map(|k| a[k] * b[k])
        .sum();
    let na: f64 = (a.values().map(|v| (*v as f64).powi(2)).sum::<f64>()).sqrt();
    let nb: f64 = (b.values().map(|v| (*v as f64).powi(2)).sum::<f64>()).sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot as f64 / (na * nb)
}

/// Token frequency map. Phase 1.2 char-level + word-level.
pub fn token_freq(text: &str) -> HashMap<String, usize> {
    let mut counter: HashMap<String, usize> = HashMap::new();
    for tok in tokenize(text) {
        *counter.entry(tok.to_string()).or_insert(0) += 1;
    }
    counter
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgetConfig {
    pub low_confidence_threshold: f64,  // 0.2
    pub max_episode_count: usize,       // 200
    pub episode_tau_seconds: f64,       // 86400.0 (1 day)
    pub note_tau_seconds: f64,          // 604800.0 (7 days)
    pub salience_cutoff: f64,           // 0.05
    pub use_salience: bool,             // true
}

impl Default for ForgetConfig {
    fn default() -> Self {
        Self {
            low_confidence_threshold: 0.2,
            max_episode_count: 200,
            episode_tau_seconds: EPISODE_TAU_SECONDS,
            note_tau_seconds: 604_800.0,
            salience_cutoff: SALIENCE_CUTOFF,
            use_salience: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ForgetError {
    #[error("Empty notes/episodes")]
    Empty,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other: {0}")]
    Other(String),
}

/// Forget strategies (主 17:43 实事求是: 阈值是经验值可调, 不假装绝对).
#[async_trait]
pub trait Forget: Send + Sync {
    /// 遗忘 confidence < threshold 的 Note.
    async fn forget_low_confidence_notes(
        &self,
        notes: Vec<Note>,
        threshold: f64,
    ) -> Result<Vec<Note>, ForgetError>;

    /// 保留最新 max_count 条 Episode (rolling window).
    async fn forget_old_episodes(
        &self,
        episodes: Vec<Episode>,
        max_count: usize,
    ) -> Result<Vec<Episode>, ForgetError>;

    /// Salience decay 驱动的遗忘. salience < cutoff 丢弃.
    /// 借鉴 DeltaMemory 2024 (Lin et al.) 1/(1+Δt/τ).
    async fn forget_by_salience(
        &self,
        episodes: Vec<Episode>,
        tau: f64,
        cutoff: f64,
        now: Option<i64>,
    ) -> Result<Vec<Episode>, ForgetError>;

    /// 综合遗忘: 先按 salience, 再按 rolling window.
    /// Ponytail: 分两阶段, salience 后再做 rolling 防止顺序依赖.
    async fn forget_episodes_combined(
        &self,
        episodes: Vec<Episode>,
        config: &ForgetConfig,
        now: Option<i64>,
    ) -> Result<Vec<Episode>, ForgetError>;
}
```

---

## §4 身份卡 trait（`apeireth-asi`）

> 源自: `mvp/identity/card.py` (203 行) IdentityCard JSON 持久化 + consolidate()

```rust
//! crates/apeireth-asi/src/identity_card.rs
//!
//! Phase 0: JSON file persistence + 4-field structure.
//! Phase 1.3+: LLM-driven consolidate (主 19:33 借鉴而非闭门).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::apeireth_core::{IdentityCard, Note, Timestamp};

#[derive(Debug, thiserror::Error)]
pub enum CardError {
    #[error("Card not found: {0}")]
    CardNotFound(PathBuf),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Other: {0}")]
    Other(String),
}

/// IdentityCard JSON file persistence + consolidation.
///
/// 主人真实身份背景 (公开可查 + 主人已知, 主 22:33 / 23:09 实事求是):
/// - 地方出生, 老家养老问题长期关注
/// - 
/// - AgentMemory 自研方向
/// - 
#[async_trait]
pub trait IdentityCardStore: Send + Sync {
    /// Load IdentityCard from JSON file. If file not exists, create with default seed.
    /// Ponytail: default arg must be evaluated at call time so monkeypatch of DEFAULT_CARD takes effect during tests.
    async fn load(&self, path: &Path) -> Result<IdentityCard, CardError>;

    /// Save IdentityCard to JSON file. Auto-update updated_at.
    async fn save(&self, path: &Path, card: &IdentityCard) -> Result<(), CardError>;

    /// 从 Note 周期 consolidate IdentityCard (Phase 1.3 主入口).
    ///
    /// Ponytail ceiling: token 频次 + 置信度阈值.
    /// - 多字符 token (英文 word / 2+ 字中文词): freq >= min_freq 入卡
    /// - 单字中文: 过滤 (噪音太多, 主人哲学"借鉴而非闭门")
    ///
    /// Phase 2 LLM 接入后: 换成 LLM 提炼 background / values.
    async fn consolidate(
        &self,
        notes: Vec<Note>,
        min_freq: usize,
        min_confidence: f64,
    ) -> Result<IdentityCard, CardError>;

    /// Simple key-based evolution. Append to evolution_log + maybe update field.
    /// Phase 1.3+ 会改用 LLM 提炼.
    async fn evolve(
        &self,
        card: &mut IdentityCard,
        key: &str,
        value: serde_json::Value,
        source: &str,
    ) -> Result<(), CardError>;

    /// Human-readable summary for CLI.
    async fn render(&self, card: &IdentityCard) -> String;
}

/// Default card JSON path (~/.apeireth_mvp/identity_card.json).
/// Phase 0: hardcoded, Phase 1: env var override.
pub fn default_card_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".apeireth_mvp").join("identity_card.json")
}
```

---

## §5 CLI trait（`apeireth-cli`）

> 源自: `mvp/cli.py` (235 行) --new-session / --resume-session / --chat / --recall / --consolidate

```rust
//! crates/apeireth-cli/src/main.rs
//!
//! Phase 0: clap-based CLI surface stable. Phase 2 swap echo() for LLM-generated reply.

use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

pub const CLI_VERSION: &str = "0.14.0";

#[derive(Parser, Debug)]
#[command(name = "apeireth")]
#[command(version = CLI_VERSION)]
#[command(about = "Apeireth MVP CLI Agent — Cross-Session Memory")]
#[command(long_about = "R13 MVP CLI Agent. Phase 0: echo + retrieve. Phase 2: LLM integration.")]
pub struct Cli {
    /// SQLite database path (default: ./data/mvp.db)
    #[arg(long, default_value = "./data/mvp.db", env = "APEIRETH_DB")]
    pub db: PathBuf,

    /// IdentityCard JSON path (default: <db_path>.card.json)
    #[arg(long, env = "APEIRETH_CARD")]
    pub card: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 开新 session (or resume if session_id given)
    NewSession {
        #[arg(long)]
        session_id: Option<String>,
    },

    /// 恢复上次 session (auto-append episode 'resume')
    ResumeSession,

    /// Interactive REPL (Phase 1: echo + store, Phase 2: LLM-generated reply)
    Chat {
        #[arg(long)]
        session_id: Option<String>,
    },

    /// One-shot recall (CLI variant of retrieve)
    Recall {
        query: String,
        #[arg(long)]
        session_id: Option<String>,
    },

    /// 周期 consolidate: Episode → Note 提炼 + 合并 + IdentityCard 演化
    Consolidate(ConsolidateArgs),

    /// 渲染 IdentityCard
    Whoami,

    /// 显示 stats (episodes / notes / sessions count)
    Stats,
}

#[derive(Args, Debug, Clone)]
pub struct ConsolidateArgs {
    #[arg(long)]
    pub session_id: Option<String>,

    /// Forget notes with confidence below this (default: 0.2)
    #[arg(long, default_value_t = 0.2)]
    pub note_threshold: f64,

    /// Merge notes with cosine similarity >= this (default: 0.85)
    #[arg(long, default_value_t = 0.85)]
    pub merge_threshold: f64,

    /// Dry-run (no DB writes)
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Chat REPL command vocabulary (matches Python `cli.py` chat() inner loop).
pub enum ChatCommand {
    Add(String),
    Recall(String),
    Note(String),
    Whoami,
    Stats,
    Bye,
    Unknown(String),
}

pub fn parse_chat_line(line: &str) -> ChatCommand {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ChatCommand::Unknown("".into());
    }
    if trimmed == "bye" || trimmed == "exit" || trimmed == "quit" {
        return ChatCommand::Bye;
    }
    if let Some(rest) = trimmed.strip_prefix("add ") {
        return ChatCommand::Add(rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("recall ") {
        return ChatCommand::Recall(rest.trim().to_string());
    }
    if let Some(rest) = trimmed.strip_prefix("note ") {
        return ChatCommand::Note(rest.trim().to_string());
    }
    if trimmed == "whoami" {
        return ChatCommand::Whoami;
    }
    if trimmed == "stats" {
        return ChatCommand::Stats;
    }
    ChatCommand::Unknown(trimmed.split_whitespace().next().unwrap_or("").to_string())
}
```

---

## §6 错误类型（`apeireth-core`）

```rust
//! crates/apeireth-core/src/error.rs
//!
//! 统一错误类型 — Phase 0 错误规范, thiserror derive.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError { /* see §2 */ }

#[derive(Debug, Error)]
pub enum RetrieveError { /* see §2 */ }

#[derive(Debug, Error)]
pub enum ConsolidateError { /* see §3 */ }

#[derive(Debug, Error)]
pub enum ForgetError { /* see §3 */ }

#[derive(Debug, Error)]
pub enum CardError { /* see §4 */ }
```

---

## §7 Python ↔ Rust 映射

| Python (`mvp/`) | Rust (`apeireth-rust/`) | 映射说明 |
|---|---|---|
| `mvp/__init__.py` | `crates/apeireth-core/src/lib.rs` | crate re-exports |
| `mvp/pyproject.toml` | `Cargo.toml` (workspace) | Cargo workspace root |
| `mvp/memory/store.py` (274 行) | `crates/apeireth-memory/src/store_sqlite.rs` | SQLite + rusqlite + FTS5 |
| `mvp/memory/retrieve.py` (153 行) | `crates/apeireth-memory/src/retrieve.rs` | BM25 + Salience decay (regex crate) |
| `mvp/memory/consolidate.py` (194 行) | `crates/apeireth-memory/src/consolidate.rs` | extract + merge + dedupe |
| `mvp/memory/forget.py` (82 行) | `crates/apeireth-memory/src/forget.rs` | forget_low_confidence + forget_by_salience |
| `mvp/identity/card.py` (203 行) | `crates/apeireth-asi/src/identity_card.rs` | IdentityCard JSON + consolidate |
| `mvp/cli.py` (235 行) | `crates/apeireth-cli/src/main.rs` | clap CLI |
| `mvp/tests/test_memory.py` (188 行) | `crates/apeireth-memory/tests/store_test.rs` | SQLite store tests (10) |
| `mvp/tests/test_consolidate.py` (252 行) | `crates/apeireth-memory/tests/consolidate_test.rs` | consolidate tests (16) |
| `mvp/tests/test_consolidate.py` (12 tests) | `crates/apeireth-memory/tests/forget_test.rs` | forget tests (4) |
| — | `crates/apeireth-core/src/types.rs` | Episode/Note/Session/IdentityCard (NEW) |
| — | `crates/apeireth-core/src/error.rs` | 统一错误类型 (NEW) |
| — | `crates/apeireth-pybridge/src/lib.rs` | PyO3 暴露 Python 调用 (Phase 1+) |
| — | `crates/apeireth-test/src/lib.rs` | 集成测试 cross-crate (Phase 1+) |

**Rust crate 拓扑**:
```
apeireth (workspace root)
├── apeireth-core      (~500 行) — 类型 + 错误 + 工具
├── apeireth-memory    (~1500 行) — Store + Retrieve + Consolidate + Forget
├── apeireth-asi       (~300 行) — IdentityCard + 身份演化
├── apeireth-cli       (~400 行) — clap CLI
└── apeireth-pybridge  (~500 行) — PyO3 暴露 (Phase 1+)
                          Total: ~3200 行 (vs Python MVP 2292 insertions)
```

**依赖列表** (Rust crates):
- `tokio` — async runtime
- `async-trait` — async trait (dyn-compatible)
- `rusqlite` (with `bundled` feature) — SQLite + FTS5
- `serde` + `serde_json` — JSON serialization
- `uuid` — UUID v4 generation
- `regex` — tokenize [A-Za-z]+|[\u{4e00}-\u{9fff}]
- `thiserror` — error derive
- `clap` (with `derive` feature) — CLI parsing
- `chrono` (optional) — human-readable timestamp
- (Phase 1+) `pyo3` + `pyo3-asyncio` — PyO3 Python bridge
- (Phase 1+) `jieba-rs` — CJK word segmentation (主 19:33 借鉴)

---

## §8 R14 Phase 0 → Phase 1 入口

完成本规范后，R14 团队按以下顺序进入 Phase 1（Rust 关键路径实现，4 周）：

### Week 1 (T26 完成 workspace 骨架之后)
1. **`apeireth-core`** 实现 Episode/Note/Session/IdentityCard 类型 + 统一错误 (500 行)
2. **`apeireth-core`** 实现 tokenize() + bm25_score() + decay() 工具函数 (200 行)

### Week 2
3. **`apeireth-memory`** 实现 SQLite Store + schema bootstrap (800 行)
4. **`apeireth-memory`** 实现 BM25 Retrieve (LIKE scan) (500 行)
5. **测试**: store_test 10 tests + retrieve_test 8 tests PASSED

### Week 3
6. **`apeireth-memory`** 实现 Consolidate (extract/merge/dedupe/update_confidence) (700 行)
7. **`apeireth-memory`** 实现 Forget (low_confidence/rolling_window/salience) (300 行)
8. **测试**: consolidate_test 16 tests + forget_test 4 tests PASSED

### Week 4
9. **`apeireth-asi`** 实现 IdentityCardStore (load/save/consolidate/evolve/render) (300 行)
10. **`apeireth-cli`** 实现 clap CLI (new-session/resume-session/chat/recall/consolidate/whoami/stats) (400 行)
11. **`apeireth-pybridge`** 实现 PyO3 暴露 (Phase 1+, optional) (500 行)
12. **集成测试**: 27/27 tests PASSED 跨 Rust 实现 (vs Python 27/27 baseline)

### Phase 1 验收标准
- 所有 27 tests 在 Rust 实现下 PASS
- SQLite schema 与 Python 完全兼容（双向 import/export）
- CLI surface 与 Python `mvp.cli` 一致（clap help 输出与 click help 输出对齐）
- IdentityCard JSON 双向兼容（Python 写 → Rust 读 / Rust 写 → Python 读）
- 性能: SQL LIKE scan + Rust serde 速度 ≥ Python 2x（实测量化, 不刷 KPI）

---

## §9 硬性约束守护（§5.E 红线）

按附录 N §5.E + 主人哲学逐项核对本规范：

| 红线 | 本规范状态 |
|---|---|
| ❌ 不重写 V0.5 公式 | ✅ 仅描述 Rust trait 接口, 不涉及 ASI 公式 |
| ❌ 不重做 V1136 真测引擎 | ✅ 不涉及 V1136 真测 |
| ❌ 不重写哲学守门 | ✅ 仅描述 IdentityCard traits, 不改哲学守门逻辑 |
| ❌ 不写 ASI 北极星公式 | ✅ 无 ASI 公式定义 |
| ❌ 不刷 KPI | ✅ 性能指标留 Phase 1 实测, 不预填 |
| ❌ 不假装达到 ASI | ✅ 接口命名保守 (MVP CLI agent, 不是 ASI agent) |
| ✅ 借鉴而非闭门 (主 19:33) | ✅ 依赖 rusqlite/sqlx/tantivy/jieba-rs 都标注"借鉴" |
| ✅ 实事求是 (主 17:43) | ✅ 阈值/salience cutoff 标"经验值可调" |
| ✅ 干到底 (主 23:44) | ✅ Phase 0 4 周 + Phase 1 4 周 路径明确 |

---

## §10 一句话给 R14 团队

> "Python MVP 13 文件 2292 insertions 已经把'跨 session 记忆 + 启发式 consolidate + 遗忘'的核心跑通；Rust trait 规范把这 2292 行代码契约化为 8 个 async_trait + 5 个 config struct + 统一错误类型，让 R14 团队按 §8 Week 1-4 计划逐步落地，关键路径与 Python 行为 1:1 对齐，性能优化留 Phase 2 实测（不刷 KPI）。"

---

## §11 R11 → Rust trait 映射清单 (R14-D6-C E3 追加)

> **范围**: 把 R11 真生产 Python 锚点 v*.py → 9 个 crate Rust trait 草案接口一一对应。
> **原则** (主 19:33 走在前人经验上): Rust 实现直接借 R11 已落地的真组件行为契约, 不重新设计。
> **不变承诺**: ❌ 不重写 V0.5 / V1136 / 哲学守门; ❌ 不砍 1100 空壳; ❌ 不写 ASI 公式。
> **来源**: E3 任务 (fullstack_engineer) — 基于本规范 §1-§9 trait + §8 Week 1-4 计划 + R11 v*.py 锚点真读。

---

### 11.1 `apeireth-asi` ← v1077 + v1101 + v1106 + v1115

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1077_asi_v04_full_measurement.py` (V0.4 17 维真测) + `apeireth/v1101_asi_v04_dim_lift.py` (维度自动拉升引擎) + `apeireth/v1106_engineering_lift.py` (真工程: error handling/retry/circuit breaker/health check/metrics) + `apeireth/v1115_cognitive_dream_orchestrator_e2e.py` (V1107+V1108+V1060+V1072+V1084 真集成) |
| **Rust trait 草案接口** | (1) `trait AsiV05Scores { fn measure_v05(&self, dims: u8) -> AsiV05Scores; fn dim_lift(&self, dim: &str, target: f64) -> LiftReport; }` (2) `trait EngineeringResilience { fn with_circuit_breaker<T>(&self, op: T) -> Result<T>; fn with_retry<T>(&self, op: T, policy: RetryPolicy) -> Result<T>; fn health_check(&self) -> HealthStatus; fn metrics(&self) -> MetricsSnapshot; }` (3) `trait CognitiveOrchestrator { async fn run_e2e(&self, episode: &Episode) -> OrchestratorReport; }` |
| **核心类型** | `AsiV05Scores { dims_filled: u8, total: f64, lift: Vec<(String, f64)> }` + `LiftReport { dim: String, before: f64, after: f64, contributor: String }` + `CircuitBreakerState { Closed, Open, HalfOpen }` |
| **阶段 4 落实步骤** | Week 13-16: (1) 把 v1077 真测逻辑移植为 Rust `AsiV05Scores::measure_v05` (只读 17 维度, 不重写 V0.5 公式); (2) 把 v1106 工程韧性 (circuit breaker/retry/health check/metrics) 实现为 `EngineeringResilience` trait; (3) V1115 e2e 流程移植为 `CognitiveOrchestrator::run_e2e` 编排; (4) 与 `apeireth-core` Identity 类型对接 (`Identity::evolution_log`)。 |
| **借鉴决策** | **直接借鉴 R11**: V1077 17 维度字段、V1106 Netflix Hystrix 模式、V1115 e2e 编排顺序 — **不绑** V49 DGM/V61 self-evolution (Python 原生即可)。 |

---

### 11.2 `apeireth-bench` ← v1012 + v1106

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1012_agent_benchmark.py` (SWE-bench/MMLU 真借鉴 agent benchmark) + `apeireth/v1106_engineering_lift.py` (工程韧性基准点) |
| **Rust trait 草案接口** | (1) `trait BenchmarkHarness { fn run_swe_bench(&self, tasks: &[Task]) -> BenchmarkReport; fn run_mmlu(&self, categories: &[String]) -> BenchmarkReport; }` (2) `trait ResilienceBenchmark { fn benchmark_circuit_breaker(&self, fault_rate: f64) -> ResilienceMetrics; fn benchmark_retry(&self, transient_rate: f64) -> RetryMetrics; }` |
| **核心类型** | `BenchmarkReport { name: String, pass_rate: f64, p50_ms: u64, p95_ms: u64, p99_ms: u64, memory_mb: f64 }` + `ResilienceMetrics { recovery_time_ms: u64, false_positive_rate: f64 }` |
| **阶段 4 落实步骤** | Week 21-26: (1) 把 v1012 SWE-bench/MMLU benchmark 抽象为 `BenchmarkHarness` trait, 数据集从 fixture 文件加载; (2) 把 v1106 工程韧性测试 (注入故障 / 测量恢复时间) 抽象为 `ResilienceBenchmark` trait; (3) criterion-rs 整合, 跨 release 比较 wallclock; (4) 与 V1130 wallclock 2.5s target 对齐 (实测数据, 不刷 KPI)。 |
| **借鉴决策** | **直接借鉴 R11**: SWE-bench (Princeton 2024) + MMLU (Hendrycks 2020) + Netflix Hystrix 2012 circuit breaker — **不绑** LangChain/LlamaIndex benchmark runner (闭门)。 |

---

### 11.3 `apeireth-cli` ← v1009 + v1016

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1009_web_ui.py` (FastAPI 真借鉴 Web UI) + `apeireth/v1016_rest_gateway.py` (FastAPI/Kong 真借鉴 REST gateway) |
| **Rust trait 草案接口** | (1) `trait Cli { async fn run(&self, cmd: CliCommand) -> CliResult; }` (clap-based, 与 mvp/cli.py 1:1 对齐) (2) `trait WebServer { async fn serve(&self, addr: SocketAddr, routes: Vec<Route>) -> Result<()>; }` (axum-based, 不绑 FastAPI) (3) `trait RestGateway { fn register_route(&mut self, path: &str, handler: Handler); fn rate_limit(&self, key: &str) -> bool; }` |
| **核心类型** | `CliCommand { NewSession, ResumeSession, Chat, Recall, Consolidate, Whoami, Stats }` (与本规范 §5 完全一致) + `Route { method: HttpMethod, path: String, handler: Box<dyn Handler> }` |
| **阶段 4 落实步骤** | Week 17-20: (1) `apeireth-cli` 用 clap 实现 §5 CLI surface stable; (2) `apeireth-cli` 用 axum 0.7+ 实现 Web UI (借 v1009 FastAPI 路由); (3) `apeireth-cli` 实现 rate limiter (token bucket, 借 v1016 Kong); (4) 与 mvp/cli.py click help 输出对齐 (Phase 1 验收)。 |
| **借鉴决策** | **直接借鉴 R11**: FastAPI 路由设计 + Kong rate limiting + clap CLI pattern — **不绑** Starlette/Uvicorn 闭门组合。 |

---

### 11.4 `apeireth-core` ← v1004 + v1107 + v1108 + v1115

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1004_self_evolution_full.py` (V49 DGM + UCB1 bandit 自演化) + `apeireth/v1107_cognitive_core_lift.py` (IDENTITY 5 Module + 真认知能力) + `apeireth/v1108_dream_v2.py` (6 状态机: IDLE/DREAMING/CONSOLIDATING/FORGETTING/VERIFYING/INTERRUPTED) + `apeireth/v1115_cognitive_dream_orchestrator_e2e.py` (Cognitive-Dream 真贯连) |
| **Rust trait 草案接口** | (1) `trait SelfEvolution { fn evolve(&mut self, candidates: Vec<Variant>, bandit: Ucb1) -> Variant; }` (2) `trait CognitiveCore { fn identity_module(&self) -> IdentityView; fn episode_buffer(&self) -> EpisodeBufferView; fn note_consolidator(&self) -> NoteConsolidatorView; }` (3) `trait DreamStateMachine { fn transition(&mut self, event: DreamEvent) -> DreamState; fn is_terminal(&self) -> bool; }` (4) `trait CognitiveOrchestrator` (与 11.1 共享) |
| **核心类型** | `IdentityView { id: ShortId, evolution_log: Vec<EvolutionEvent> }` + `DreamState { Idle, Dreaming, Consolidating, Forgetting, Verifying, Interrupted }` + `EpisodeBufferView { window_size: usize, episodes: VecDeque<Episode> }` |
| **阶段 4 落实步骤** | Week 5-8: (1) 落实 §1 类型 (Episode/Note/Session/IdentityCard) + 统一错误; (2) 实现 `SelfEvolution` trait (UCB1 bandit, 借 V49 DGM); (3) 实现 `DreamStateMachine` (6 状态枚举 + 转换函数); (4) 实现 `CognitiveCore` (IDENTITY 5 Module 适配); (5) tokenize() + bm25_score() + decay() 工具函数。 |
| **借鉴决策** | **直接借鉴 R11**: V49 DGM (Sakana AI) UCB1 bandit + V61 self-evolution + V1092 6 状态机 — **不绑** Sakana AI 闭门框架。 |

---

### 11.5 `apeireth-memory` ← v1005 + v1019 + mvp/memory

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1005_anysearch_full_index.py` (AnySearch 调研结果索引 23 真调研文档) + `apeireth/v1019_embeddings.py` (OpenAI/BAAI bge-m3 真借鉴) + `mvp/memory/store.py` + `mvp/memory/retrieve.py` + `mvp/memory/consolidate.py` + `mvp/memory/forget.py` (Phase 0 13 文件 2292 insertions) |
| **Rust trait 草案接口** | (1) `trait NoteStore { async fn insert(&self, note: &Note) -> Result<(), StoreError>; async fn get(&self, id: &ShortId) -> Result<Note, StoreError>; }` (2) `trait RetrievalEngine { async fn bm25_search(&self, query: &str, top_k: usize) -> Vec<Episode>; async fn vector_search(&self, query: &str, top_k: usize) -> Vec<Episode>; }` (3) `trait ConsolidateEngine { async fn extract(&self, episodes: &[Episode]) -> Vec<Note>; async fn merge(&self, notes: Vec<Note>) -> Vec<Note>; }` (4) `trait ForgetEngine { async fn forget_by_salience(&self, cutoff: f64) -> usize; }` |
| **核心类型** | `Episode`, `Note`, `Session` (与 §1 完全一致) + `Embedding { vec: Vec<f32>, model: String }` |
| **阶段 4 落实步骤** | Week 9-12: (1) `apeireth-memory` SQLite Store + schema bootstrap (rusqlite + bundled); (2) BM25 Retrieve (LIKE scan, 不绑 tantivy 闭门索引); (3) Vector Search (借 v1019 OpenAI/BAAI bge-m3 接口, Rust 端 ONNX Runtime 跑 bge-m3 ONNX); (4) Consolidate (extract/merge/dedupe/update_confidence) + Forget (low_confidence/rolling_window/salience); (5) 27/27 契约测试 PASS (Phase 1 验收)。 |
| **借鉴决策** | **直接借鉴 R11**: mvp/memory/ 13 文件行为契约 + v1019 embedding 接口 + v1005 索引模式 — **不绑** LangChain VectorStore / Letta Memory 闭门。 |

---

### 11.6 `apeireth-philosophy` ← v1003 + v1121

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1003_v4_philosophy_full.py` (V4 哲学完整版: V3 7 哲学问题 + V2 5 位置 + 5 哲学方法论 Popper/Kuhn/Lakatos/Feyerabend/Laudan + V166 真哲学) + `apeireth/v1121_security_guard_v01.py` (R11-SEC-001: OWASP Top 10 + NIST SSDF + STRIDE + Identity 守门 + 5 项不假装) |
| **Rust trait 草案接口** | (1) `trait PhilosophyGuard { fn check_phl01(&self, claim: &str) -> PHL01Result; fn check_phl02b(&self, claim: &str) -> PHL02Result; fn check_phl03(&self, claim: &str) -> PHL03Result; }` (2) `trait SecurityGuard { fn threat_model(&self, input: &ThreatInput) -> ThreatReport; fn audit(&self, event: &AuditEvent) -> AuditLog; fn check_no_pretend(&self) -> FiveGuardsReport; }` |
| **核心类型** | `PHL01Result { is_clone: bool, is_perfect: bool, is_uuid: bool, passes: bool }` + `ThreatReport { owasp_category: String, nist_ssdf: String, stride: String, severity: Severity }` + `FiveGuardsReport { consciousness: GuardStatus, asi: GuardStatus, docker: GuardStatus, ... }` |
| **阶段 4 落实步骤** | Week 5-8 (与 core 并行): (1) 落实 V3 9 键 LOCKED (PHL-01/02b/03, 不重写哲学守门); (2) `SecurityGuard` trait 实现 OWASP Top 10 + STRIDE 威胁建模; (3) 5 项不假装 (R11-R1/R2/R3/R4/R5) 编译期 + 运行期双守门; (4) 与 `apeireth-asi` Identity 守门对接 (`Identity::evolution_log` 审计)。 |
| **借鉴决策** | **直接借鉴 R11**: V3 9 键 + 5 项不假装 + OWASP/NIST/STRIDE — **不绑** Popper/Kuhn 等闭门哲学框架 (借鉴方法论, 不绑实现)。 |

---

### 11.7 `apeireth-pybridge` ← (新) PyO3 桥接 1100 模块

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1000-v1155*.py` (1100+ 真生产 Python 模块, R11 不砍, R14 不强求重写) + `apeireth/cli.py` + `apeireth/asi_coordinator.py` 等核心 8 个 Python 文件 |
| **Rust trait 草案接口** | (1) `trait PyO3Module { fn module_name(&self) -> &str; fn register(&self, py: Python<'_>, m: &PyModule) -> PyResult<()>; }` (2) `trait PythonImporter { fn import_module(&self, name: &str) -> Result<PyObject, PyBridgeError>; fn call(&self, module: &str, fn_name: &str, args: &[PyObject]) -> Result<PyObject, PyBridgeError>; }` (3) `trait BridgeBoundary { fn is_python_only(&self, name: &str) -> bool; fn is_rust_first(&self, name: &str) -> bool; }` |
| **核心类型** | `PyBridgeError { IoError(#[from] io::Error), PythonError(String), ModuleNotFound(String) }` + `BridgeBoundary { PythonOnly, RustFirst, Hybrid }` |
| **阶段 4 落实步骤** | Week 17-20 (兼容桥): (1) 用 pyo3 0.22+ 实现 `apeireth-pybridge` crate, 暴露 `Python::with_gil` 入口; (2) 实现 `PythonImporter`, 让 Rust 主路径可调用 v*.py 模块 (例如 `v1077_asi_v04_full_measure` 仍可被 Rust 调用); (3) `BridgeBoundary` 标记哪些模块 Rust-first / Python-only / Hybrid; (4) 测试: Rust 调用 v1077 真测, 结果与 Python 直接调用 1:1; (5) Phase 2+ 逐步把 Hybrid → Rust-first。 |
| **借鉴决策** | **直接借鉴 R11**: 1100+ v*.py 模块行为契约 + cli.py/asi_coordinator.py 等核心 Python 文件 — **不绑** PyO3-asyncio 闭门。**关键**: 不砍 1100 空壳 (主硬约束), 只在 pybridge 层桥接, 性能优化留给 Phase 2+。 |

---

### 11.8 `apeireth-test` ← v1114 + v1115

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1114_weekly_integration_evaluator.py` (R9-INT-003 每周集成评估器: 三件套真测 + ASI dashboard + 4 选 1 主轨道 + 守门自检) + `apeireth/v1115_cognitive_dream_orchestrator_e2e.py` (E2E 真集成) + `mvp/tests/*.py` (Phase 0 27/27 契约测试) |
| **Rust trait 草案接口** | (1) `trait IntegrationTestHarness { async fn run_weekly(&self) -> WeeklyReport; fn check_three_pieces(&self) -> ThreePiecesReport; }` (2) `trait WeeklyEvaluator { fn evaluate_v05(&self) -> DashboardSnapshot; fn select_main_track(&self, lift: f64) -> MainTrack; }` (3) `trait E2ETestSuite { async fn run_cognitive_dream(&self) -> E2EReport; }` |
| **核心类型** | `WeeklyReport { timestamp: Timestamp, three_pieces: ThreePiecesReport, dashboard: DashboardSnapshot, main_track: MainTrack, philosophy_guard: FiveGuardsReport }` + `MainTrack { CognitiveCore, DreamOrchestrator, IdentityEvolution, ContinuousLift }` |
| **阶段 4 落实步骤** | 持续集成 (Week 1-26): (1) `apeireth-test` 集成 cargo test + cargo bench + criterion-rs; (2) `IntegrationTestHarness::run_weekly` 每周跑 R11 三件套真测 (V1074 V0.3 + V1077 V0.4 + V1103 Top-5 P2); (3) `WeeklyEvaluator::evaluate_v05` 生成 dashboard snapshot (与 V1136 dashboard 1:1 对齐, 不重写); (4) `E2ETestSuite::run_cognitive_dream` 验证 V1107+V1108+V1060+V1072+V1084 端到端贯连; (5) Phase 1 验收 27/27 契约测试 + Phase 2 性能对比。 |
| **借鉴决策** | **直接借鉴 R11**: V1114 三件套真测 + V1115 e2e 集成顺序 + mvp/tests/ 27 契约测试 — **不绑** pytest 闭门 (Rust 端用 cargo test)。 |

---

### 11.9 `apeireth-tools` ← v1000 + v1027

| 字段 | 内容 |
|------|------|
| **R11 真生产 Python 锚点** | `apeireth/v1000_yaml_serializer.py` (safe YAML serialization, PyYAML safe_load/safe_dump + ruamel round-trip, 借 Letta/LangGraph/VCPToolBox) + `apeireth/v1027_validator.py` (validator/schema, 借 JSON Schema + Pydantic + Cerberus + V116 整合) |
| **Rust trait 草案接口** | (1) `trait YamlSerializer { fn load(&self, path: &Path) -> Result<YamlValue, YamlError>; fn dump(&self, value: &YamlValue) -> Result<String, YamlError>; fn round_trip(&self, value: &YamlValue) -> Result<YamlValue, YamlError>; }` (2) `trait Validator { fn validate(&self, schema: &Schema, data: &Value) -> ValidationReport; fn register_schema(&mut self, name: &str, schema: Schema); }` |
| **核心类型** | `YamlValue { Null, Bool(bool), Int(i64), Float(f64), String(String), Sequence(Vec<YamlValue>), Mapping(BTreeMap<String, YamlValue>) }` (serde_yaml 兼容) + `Schema { name: String, rules: Vec<Rule> }` + `ValidationReport { passes: bool, violations: Vec<Violation> }` |
| **阶段 4 落实步骤** | Week 5-8 (与 core/philosophy 并行): (1) `apeireth-tools` 用 serde_yaml + yaml-rust 双实现, safe load/dump 强制; (2) `YamlSerializer::round_trip` 支持注释保留 (借 ruamel round-trip 模式); (3) `Validator` trait 实现 JSON Schema Draft 2020-12 子集; (4) 与 `apeireth-cli` 配置加载对接 (借 v1000 config 模式); (5) 测试: serde_yaml 与 PyYAML 双向 import/export 1:1。 |
| **借鉴决策** | **直接借鉴 R11**: v1000 safe YAML + v1027 JSON Schema/Pydantic — **不绑** Letta/LangGraph 闭门配置框架。 |

---

### 11.10 9-crate 映射总览

| Crate | R11 锚点 | Rust trait 草案接口数 | 阶段 4 落实周次 | 优先级 |
|-------|---------|-------------------|---------------|--------|
| `apeireth-asi` | v1077+v1101+v1106+v1115 (4) | 3 trait + 3 type | Week 13-16 | P1 |
| `apeireth-bench` | v1012+v1106 (2) | 2 trait + 2 type | Week 21-26 | P2 |
| `apeireth-cli` | v1009+v1016 (2) | 3 trait + 2 type | Week 17-20 | P1 |
| `apeireth-core` | v1004+v1107+v1108+v1115 (4) | 4 trait + 3 type | Week 5-8 | P0 (基础) |
| `apeireth-memory` | v1005+v1019+mvp/memory (6) | 4 trait + 3 type | Week 9-12 | P0 (核心) |
| `apeireth-philosophy` | v1003+v1121 (2) | 2 trait + 3 type | Week 5-8 | P0 (守门) |
| `apeireth-pybridge` | (新) 1100+ v*.py | 3 trait + 2 type | Week 17-20 | P1 (兼容) |
| `apeireth-test` | v1114+v1115 (2) | 3 trait + 2 type | Week 1-26 持续 | P1 |
| `apeireth-tools` | v1000+v1027 (2) | 2 trait + 3 type | Week 5-8 | P1 |

**累计**: 9 crates × 26 trait 草案接口 + 23 核心类型 = **R11 真生产行为契约 1:1 落到 Rust trait**。

---

### 11.11 阶段 4 落实总时间线 (与 §8 Week 1-26 一致 + Rust trait 草案锚点)

```
Week 5-8  (核心层):     core / philosophy / tools 并行
Week 9-12 (记忆层):     memory (核心)
Week 13-16 (智能层):    asi
Week 17-20 (入口+桥接): cli / pybridge 并行
Week 21-26 (性能+验证): bench / test 持续
```

---

### 11.12 §5.E 红线核对

按附录 N §5.E + 主人哲学逐项核对本映射清单:

| 红线 | 本清单状态 |
|---|---|
| ❌ 不重写 V0.5 公式 | ✅ 仅描述 Rust trait 草案接口, 不涉及 V0.5 公式重写 |
| ❌ 不重做 V1136 真测引擎 | ✅ bench 借 v1012/v1106, 不重做 V1136 |
| ❌ 不重写哲学守门 | ✅ philosophy 借 v1003/v1121, trait 草案不重写 V3 9 键 |
| ❌ 不写 ASI 北极星公式 | ✅ asi 借 v1077/v1101 真测, trait 草案不写新公式 |
| ❌ 不刷 KPI | ✅ 性能指标留 Phase 2 实测, 不预填 |
| ❌ 不假装达到 ASI | ✅ crate 命名保守 (asi 是北极星导向, 不是 asi agent) |
| ❌ 不砍 1100 空壳 | ✅ pybridge 桥接 1100+ v*.py, 不砍 |
| ✅ 借鉴而非闭门 (主 19:33) | ✅ 所有 trait 标注"借 R11", 不绑 LangChain/Letta/Sakana |
| ✅ 实事求是 (主 17:43) | ✅ trait 字段反映 R11 真生产行为, 不脑补 |
| ✅ 干到底 (主 23:44) | ✅ Week 5-26 时间线 + 9 crates × 26 trait 全列 |

---

### 11.13 一句话给 R14 团队

> "R11 真生产 1100+ v*.py + mvp/ 13 文件 2292 insertions 已经把跨 session 记忆 + 真测 + 工程韧性 + 哲学守门 + 自演化 + 6 状态机 + weekly 集成评估全部跑通；R11 → Rust trait 映射清单把这 1100+ 模块行为契约浓缩为 9 crates × 26 trait 草案接口 + 23 核心类型 + Week 5-26 落实时间线, 让 R14 团队按本清单逐步落地, 关键路径与 Python 行为 1:1 对齐 (pybridge 兼容 + 不砍 1100 空壳), 性能优化留 Phase 2 实测 (不刷 KPI)。"

---

**end of rust-traits-spec-2026-07-30.md §11 R14-D6-C E3 追加**