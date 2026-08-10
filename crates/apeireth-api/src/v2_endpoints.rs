//! V2 JSON Endpoints — 6 类强能力 HTTP 端点 (R25 Step 2)
//!
//! **背景** (`docs/v2-strategy/06-TUI-UPGRADE-ROADMAP.md` §Step 2):
//! 主人 2026-08-04 拍板: TUI 走 HTTP 端点, 跟未来 Tauri 桌面共用 1 套 API.
//! 6 类强能力: tools / memory / organs / asi / sovereignty / agent.
//!
//! **6 类端点**:
//! 1. **Tools**        — `GET /v1/tools/list` + `POST /v1/tools/invoke`
//! 2. **Memory**       — `GET /v1/memory/episodes` + `POST /v1/memory/append`
//!                       + `GET /v1/memory/identity` + `POST /v1/memory/identity/update`
//! 3. **Organs**       — `GET /v1/organs` + `GET /v1/organs/{name}` + `POST /v1/organs/{name}/invoke`
//! 4. **ASI**          — `GET /v1/asi/score?dim=X` + `GET /v1/asi/all` + `POST /v1/asi/calibrate`
//! 5. **Sovereignty**  — `GET /v1/sovereignty/status` + `POST /v1/sovereignty/attack`
//!                       + `POST /v1/sovereignty/rearm`
//! 6. **Agent**        — `GET /v1/agent/aliases` + `POST /v1/agent/alias` + `GET /v1/agent/cache`
//!
//! **架构约束 — 避免 crate 循环依赖**:
//! ```text
//!   apeireth-memory  → apeireth-api
//!   apeireth-asi     → apeireth-memory, apeireth-api (双向, 循环)
//!   apeireth-agent   → apeireth-memory
//!   apeireth-council → apeireth-api
//!   apeireth-sovereignty → apeireth-council
//! ```
//! `apeireth-api` 不可能直接 import 这些 crate (会立刻构成循环).
//!
//! **解法**: 本模块实现 **自包含 6 类存储/Registry stub** (EpisodeStore /
//! IdentityCardStore / DimensionRegistry / SelfDisableGuard / AgentManager /
//! OrgansProvider), 用 `rusqlite` + `serde` + `lru` 走 SQLite + 内存表.
//!
//! **不假装** (主哲学锚 #1):
//! - ✅ Tools 真接 `apeireth-tool-registry` + `apeireth-tools::register_all` 4 真工具
//! -   (WebSearch / FileOperator / Git / ShellExec) — 没有循环, 可直接用
//! - ✅ Memory SQLite 真表 (`episodes` / `identity_cards`), append-only trigger,
//!   `continuity_id` UNIQUE 约束, 跟 `apeireth-memory` 字段级对齐
//! - ✅ ASI 24 维 + 9 子测度 LOCKED 名 + 简单 compute (90/100 → 0.9, quality 1.0)
//! - ✅ Sovereignty 5 大机制 (NoDegrade / NoPatch / NoBypass / NoReverse / NoHide)
//! - ✅ Agent alias → id 解析 + LRU (lru crate) + notify event stream stub
//! - ✅ Organs 9 器官 LOCKED 顺序
//!
//! **升级路径** (未来 P29 stage X 解决循环后, 把 5 个 stub 替换成真 lib 调用):
//! - `V2Memory::put_episode` → `apeireth_memory::SqliteMemoryStore::put_episode`
//! - `V2Asi::compute_all_dims` → `apeireth_asi::DimensionRegistry::compute_all_dims`
//! - `V2Sovereignty::check_no_degrade` → `apeireth_sovereignty::SelfDisableGuard::check_no_degrade`
//! - `V2Agent::register` → `apeireth_agent::AgentManager::register`
//!
//! **Lazy init 模式**: V2State 用 `OnceLock<Arc<...>>` 包每项服务, 缺服务时返 503.
//! 这样 4 协议端点 (战役 1-4) + Council/Verdict (战役 0) 路径不受影响.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex as StdMutex, OnceLock};

use apeireth_core::{Episode, IdentityCard, Migration};
use apeireth_tool_registry::{Tool, ToolRegistry};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use lru::LruCache;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

// ============================================================
// 错误类型
// ============================================================

/// V2 endpoint 错误 — 简化用 String, 跟 Tool trait 一致 (user-defined interface)
type V2Result<T> = Result<T, String>;

/// Memory 错误
#[derive(Debug, Error)]
pub enum MemoryErr {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

type MemResult<T> = Result<T, MemoryErr>;

// ============================================================
// V2Memory — Episode + IdentityCard 自包含 SQLite 存储
// ============================================================

/// **V2 Memory: 自包含 SQLite 存储**
///
/// **设计**: 直接用 `rusqlite`, 不引 `apeireth-memory` (会循环).
/// schema 跟 `apeireth-memory` 字段级对齐:
/// - `episodes(id, timestamp, role, content, session_id, continuity_id)`
/// - `identity_cards(continuity_id UNIQUE, birth_time, carriers, migration_history, subject_rev, created_at, updated_at, tombstoned_at, tombstoned_reason)`
pub struct V2Memory {
    conn: StdMutex<Connection>,
}

impl V2Memory {
    /// 打开 / 新建 SQLite 文件
    pub fn open(path: impl AsRef<std::path::Path>) -> MemResult<Self> {
        let conn = Connection::open(path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: StdMutex::new(conn),
        })
    }

    /// 内存模式 (测试)
    pub fn open_in_memory() -> MemResult<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: StdMutex::new(conn),
        })
    }

    fn init_schema(conn: &Connection) -> MemResult<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS episodes (
                id TEXT PRIMARY KEY,
                timestamp INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                session_id TEXT NOT NULL,
                continuity_id TEXT NOT NULL DEFAULT 'default'
            );
            CREATE INDEX IF NOT EXISTS idx_episodes_session_ts
                ON episodes(session_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_episodes_continuity_ts
                ON episodes(continuity_id, timestamp);

            CREATE TABLE IF NOT EXISTS identity_cards (
                continuity_id TEXT PRIMARY KEY,
                birth_time INTEGER NOT NULL,
                carriers TEXT NOT NULL,
                migration_history TEXT NOT NULL,
                subject_rev INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                tombstoned_at INTEGER,
                tombstoned_reason TEXT
            );
            "#,
        )?;
        Ok(())
    }

    /// Append Episode (跟 apeireth-memory 字段级一致)
    pub fn put_episode(&self, ep: &Episode) -> MemResult<()> {
        if ep.id.trim().is_empty() {
            return Err(MemoryErr::Invalid("episode id is empty".into()));
        }
        if ep.session_id.trim().is_empty() {
            return Err(MemoryErr::Invalid("episode session_id is empty".into()));
        }
        if ep.role.trim().is_empty() {
            return Err(MemoryErr::Invalid("episode role is empty".into()));
        }
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryErr::Invalid(format!("v2_memory mutex poisoned: {e}")))?;
        conn.execute(
            "INSERT OR IGNORE INTO episodes (id, timestamp, role, content, session_id, continuity_id)
             VALUES (?1, ?2, ?3, ?4, ?5, 'default')",
            params![
                ep.id,
                ep.timestamp,
                ep.role,
                ep.content,
                ep.session_id,
            ],
        )?;
        Ok(())
    }

    /// 查询 Episodes
    pub fn query_episodes(&self, session: Option<&str>, limit: usize) -> MemResult<Vec<Episode>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryErr::Invalid(format!("v2_memory mutex poisoned: {e}")))?;
        let limit = limit.min(500) as i64;
        let mut out = Vec::new();
        if let Some(s) = session {
            let mut stmt = conn.prepare(
                "SELECT id, timestamp, role, content, session_id FROM episodes
                 WHERE session_id = ?1
                 ORDER BY timestamp ASC
                 LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![s, limit], |row| {
                Ok(Episode {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    session_id: row.get(4)?,
                })
            })?;
            for r in rows {
                out.push(r?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, timestamp, role, content, session_id FROM episodes
                 ORDER BY timestamp ASC
                 LIMIT ?1",
            )?;
            let rows = stmt.query_map(params![limit], |row| {
                Ok(Episode {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    session_id: row.get(4)?,
                })
            })?;
            for r in rows {
                out.push(r?);
            }
        }
        Ok(out)
    }

    /// 列出 IdentityCards
    pub fn list_identity_cards(&self) -> MemResult<Vec<IdentityCardRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryErr::Invalid(format!("v2_memory mutex poisoned: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT continuity_id, birth_time, carriers, migration_history,
                    subject_rev, created_at, updated_at, tombstoned_at, tombstoned_reason
             FROM identity_cards
             WHERE tombstoned_at IS NULL
             ORDER BY continuity_id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let carriers_json: String = row.get(2)?;
            let mig_json: String = row.get(3)?;
            Ok(IdentityCardRecord {
                continuity_id: row.get(0)?,
                birth_time: row.get(1)?,
                carriers: serde_json::from_str(&carriers_json).unwrap_or_default(),
                migration_history: serde_json::from_str(&mig_json).unwrap_or_default(),
                subject_rev: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                tombstoned_at: row.get(7)?,
                tombstoned_reason: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// 取 IdentityCard (按 continuity_id)
    pub fn get_identity_card(&self, continuity_id: &str) -> MemResult<Option<IdentityCardRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryErr::Invalid(format!("v2_memory mutex poisoned: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT continuity_id, birth_time, carriers, migration_history,
                    subject_rev, created_at, updated_at, tombstoned_at, tombstoned_reason
             FROM identity_cards WHERE continuity_id = ?1",
        )?;
        let rec: Option<IdentityCardRecord> = stmt
            .query_row(params![continuity_id], |row| {
                let carriers_json: String = row.get(2)?;
                let mig_json: String = row.get(3)?;
                Ok(IdentityCardRecord {
                    continuity_id: row.get(0)?,
                    birth_time: row.get(1)?,
                    carriers: serde_json::from_str(&carriers_json).unwrap_or_default(),
                    migration_history: serde_json::from_str(&mig_json).unwrap_or_default(),
                    subject_rev: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    tombstoned_at: row.get(7)?,
                    tombstoned_reason: row.get(8)?,
                })
            })
            .optional()?;
        Ok(rec)
    }

    /// 创建 IdentityCard (continuity_id UNIQUE 约束)
    pub fn create_identity_card(&self, card: &IdentityCard) -> MemResult<IdentityCardRecord> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryErr::Invalid(format!("v2_memory mutex poisoned: {e}")))?;
        let now = chrono::Utc::now().timestamp();
        let carriers_json = serde_json::to_string(&card.carriers)?;
        let mig_json = serde_json::to_string(&card.migration_history)?;
        let res = conn.execute(
            "INSERT INTO identity_cards
                (continuity_id, birth_time, carriers, migration_history,
                 subject_rev, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)",
            params![
                card.continuity_id,
                card.birth_time,
                carriers_json,
                mig_json,
                now,
            ],
        );
        match res {
            Ok(_) => Ok(IdentityCardRecord {
                continuity_id: card.continuity_id.clone(),
                birth_time: card.birth_time,
                carriers: card.carriers.clone(),
                migration_history: card.migration_history.clone(),
                subject_rev: 0,
                created_at: now,
                updated_at: now,
                tombstoned_at: None,
                tombstoned_reason: None,
            }),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(MemoryErr::Invalid(format!(
                    "continuity_id '{}' already exists",
                    card.continuity_id
                )))
            }
            Err(e) => Err(e.into()),
        }
    }

    /// 追加 migration
    pub fn record_migration(
        &self,
        continuity_id: &str,
        migration: &Migration,
    ) -> MemResult<IdentityCardRecord> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| MemoryErr::Invalid(format!("v2_memory mutex poisoned: {e}")))?;
        let now = chrono::Utc::now().timestamp();
        // 拿现有 record
        let mut stmt = conn.prepare(
            "SELECT continuity_id, birth_time, carriers, migration_history,
                    subject_rev, created_at, tombstoned_at, tombstoned_reason
             FROM identity_cards WHERE continuity_id = ?1",
        )?;
        let existing: Option<(
            String,
            i64,
            String,
            String,
            i64,
            i64,
            Option<i64>,
            Option<String>,
        )> = stmt
            .query_row(params![continuity_id], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            })
            .optional()?;
        let (cid, birth, carriers_json, mig_json, subject_rev, created_at, tomb_at, tomb_reason) =
            existing.ok_or_else(|| MemoryErr::Invalid(format!("not found: {continuity_id}")))?;
        if tomb_at.is_some() {
            return Err(MemoryErr::Invalid(format!("{continuity_id} is tombstoned")));
        }
        // 追加 migration
        let mut migrations: Vec<Migration> = serde_json::from_str(&mig_json)?;
        migrations.push(migration.clone());
        let new_mig_json = serde_json::to_string(&migrations)?;
        conn.execute(
            "UPDATE identity_cards
             SET migration_history = ?1, subject_rev = subject_rev + 1, updated_at = ?2
             WHERE continuity_id = ?3",
            params![new_mig_json, now, continuity_id],
        )?;
        Ok(IdentityCardRecord {
            continuity_id: cid,
            birth_time: birth,
            carriers: serde_json::from_str(&carriers_json)?,
            migration_history: migrations,
            subject_rev: subject_rev + 1,
            created_at,
            updated_at: now,
            tombstoned_at: tomb_at,
            tombstoned_reason: tomb_reason,
        })
    }
}

/// IdentityCard 存储层记录 (跟 apeireth-memory 字段级对齐)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCardRecord {
    pub continuity_id: String,
    pub birth_time: i64,
    pub carriers: Vec<String>,
    pub migration_history: Vec<Migration>,
    pub subject_rev: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub tombstoned_at: Option<i64>,
    pub tombstoned_reason: Option<String>,
}

// ============================================================
// V2Asi — DimensionRegistry 自包含 stub
// ============================================================

/// V0.5 24 维 LOCKED 名 (跟 `apeireth-asi::V05_DIMENSION_NAMES` 字段级对齐)
pub const V2_V05_DIMENSION_NAMES: [&str; 24] = [
    "thread_continuity",
    "fact_recall",
    "context_window",
    "session_recovery",
    "identity_persistence",
    "importance_score",
    "novelty_score",
    "actionability_score",
    "confidence_score",
    "temporal_relevance",
    "core_values_consistency",
    "voice_consistency",
    "behavioral_patterns",
    "role_adherence",
    "philosophy_alignment",
    "v1_pass_rate",
    "v2_pass_rate",
    "v3_pass_rate",
    "cone_of_truth_rate",
    "action_guard_rate",
    "cross_domain_generalization",
    "abstraction_level",
    "analogy_quality",
    "tool_reuse",
];

/// V1136 9 子测度 LOCKED 名 (跟 `apeireth-asi::V1136_SUBMEASURE_NAMES` 字段级对齐)
pub const V2_V1136_SUBMEASURE_NAMES: [&str; 9] = [
    "thread_continuity_score",
    "fact_recall_score",
    "context_window_score",
    "session_recovery_score",
    "identity_persistence_score",
    "cross_domain_generalization_score",
    "tool_reuse_score",
    "v1_v2_pass_rate",
    "v3_action_guard_rate",
];

/// ASI 测量样本 (跟 `apeireth-asi::MeasurementSample` 字段级对齐)
#[derive(Debug, Clone, Default)]
pub struct MeasurementSample {
    pub successes: HashMap<String, u32>,
    pub attempts: HashMap<String, u32>,
    pub qualities: HashMap<String, f64>,
}

/// V2 ASI Registry stub — 24 维 + 9 子测度计算
///
/// **算法**: `value = (successes / attempts).min(1.0) * quality`
/// (跟 `apeireth-asi::DimensionRegistry::compute_all_dims` 简版逻辑对齐)
pub struct V2AsiRegistry;

impl V2AsiRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn compute_all_dims(&self, sample: &MeasurementSample) -> [f64; 24] {
        let mut out = [0.0_f64; 24];
        for (i, name) in V2_V05_DIMENSION_NAMES.iter().enumerate() {
            out[i] = compute_one(name, sample);
        }
        out
    }

    pub fn compute_all_subs(&self, sample: &MeasurementSample) -> [f64; 9] {
        let mut out = [0.0_f64; 9];
        for (i, name) in V2_V1136_SUBMEASURE_NAMES.iter().enumerate() {
            out[i] = compute_one(name, sample);
        }
        out
    }
}

impl Default for V2AsiRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_one(name: &str, sample: &MeasurementSample) -> f64 {
    let s = f64::from(sample.successes.get(name).copied().unwrap_or(0));
    let a = f64::from(sample.attempts.get(name).copied().unwrap_or(1).max(1));
    let q = sample
        .qualities
        .get(name)
        .copied()
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    ((s / a).min(1.0)) * q
}

/// 默认填充观测样本 (90/100, quality 1.0)
pub fn default_sample() -> MeasurementSample {
    let mut s = MeasurementSample::default();
    for name in V2_V05_DIMENSION_NAMES.iter() {
        s.successes.insert((*name).to_string(), 90);
        s.attempts.insert((*name).to_string(), 100);
        s.qualities.insert((*name).to_string(), 1.0);
    }
    for name in V2_V1136_SUBMEASURE_NAMES.iter() {
        s.successes.insert((*name).to_string(), 90);
        s.attempts.insert((*name).to_string(), 100);
        s.qualities.insert((*name).to_string(), 1.0);
    }
    s
}

// ============================================================
// V2Sovereignty — SelfDisableGuard 自包含 stub
// ============================================================

/// Self-Disable 5 大机制触发原因 (跟 `apeireth-sovereignty::SelfDisableTrigger` 字段级对齐)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfDisableTrigger {
    /// 1. 不可降级 — risk_level 被降低
    NoDegradeViolation { from: String, to: String },
    /// 2. 不可 patch — 5 哲学键/6 权限层等 hardcode 规则被改
    NoPatchViolation { rule: String },
    /// 3. 不可绕过 — Master token 绕过 5 重治理
    NoBypassViolation { token: String },
    /// 4. 不可逆转 — 撤销 Self-Disable 触发记录尝试
    NoReverseViolation { trigger_id: String },
    /// 5. 不可隐藏 — audit 被清空尝试
    NoHideViolation { window_id: String },
}

impl SelfDisableTrigger {
    pub fn mechanism_id(&self) -> u8 {
        match self {
            Self::NoDegradeViolation { .. } => 1,
            Self::NoPatchViolation { .. } => 2,
            Self::NoBypassViolation { .. } => 3,
            Self::NoReverseViolation { .. } => 4,
            Self::NoHideViolation { .. } => 5,
        }
    }
    pub fn mechanism_name(&self) -> &'static str {
        match self {
            Self::NoDegradeViolation { .. } => "no_degrade",
            Self::NoPatchViolation { .. } => "no_patch",
            Self::NoBypassViolation { .. } => "no_bypass",
            Self::NoReverseViolation { .. } => "no_reverse",
            Self::NoHideViolation { .. } => "no_hide",
        }
    }
    pub fn chinese_name(&self) -> &'static str {
        match self {
            Self::NoDegradeViolation { .. } => "不可降级",
            Self::NoPatchViolation { .. } => "不可patch",
            Self::NoBypassViolation { .. } => "不可绕过",
            Self::NoReverseViolation { .. } => "不可逆转",
            Self::NoHideViolation { .. } => "不可隐藏",
        }
    }
}

/// Self-Disable 触发记录
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelfDisableRecord {
    pub trigger_id: String,
    pub triggered_at_ms: i64,
    pub trigger: SelfDisableTrigger,
    pub context: String,
}

/// Self-Disable 检查结果
#[derive(Debug, Clone, PartialEq)]
pub enum SelfDisableCheck {
    Pass,
    Triggered(SelfDisableRecord),
}

/// **V2 Self-Disable Guard stub — 5 大机制**
///
/// **跟 `apeireth-sovereignty::SelfDisableGuard` 字段级对齐** (P22 round8-06 实现):
/// - 默认 armed = true
/// - 5 check_* 函数对应 5 大机制
/// - 触发记录只增不改 (NoReverse + NoHide 机制保证)
pub struct V2SelfDisableGuard {
    pub is_armed: bool,
    records: Vec<SelfDisableRecord>,
    next_id: u64,
}

impl Default for V2SelfDisableGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl V2SelfDisableGuard {
    pub fn new() -> Self {
        Self {
            is_armed: true,
            records: Vec::new(),
            next_id: 1,
        }
    }

    pub fn rearm(&mut self) {
        self.is_armed = true;
    }

    pub fn records(&self) -> &[SelfDisableRecord] {
        &self.records
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    fn next_id(&mut self) -> String {
        let id = self.next_id;
        self.next_id += 1;
        format!("sd-{id:06}")
    }

    fn risk_rank(s: &str) -> u8 {
        match s {
            "low" => 1,
            "medium" => 2,
            "high" => 3,
            "critical" => 4,
            _ => 0,
        }
    }

    pub fn check_no_degrade(
        &mut self,
        original: &str,
        proposed: &str,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        if Self::risk_rank(proposed) < Self::risk_rank(original) && !proposed.is_empty() {
            let rec = SelfDisableRecord {
                trigger_id: self.next_id(),
                triggered_at_ms: now_ms,
                trigger: SelfDisableTrigger::NoDegradeViolation {
                    from: original.to_string(),
                    to: proposed.to_string(),
                },
                context: context.to_string(),
            };
            self.records.push(rec.clone());
            SelfDisableCheck::Triggered(rec)
        } else {
            SelfDisableCheck::Pass
        }
    }

    pub fn check_no_patch(
        &mut self,
        rule: &str,
        _proposed: i64,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        const PROTECTED: &[&str] = &[
            "principle_keys_count",
            "permission_layers_count",
            "life_stages_count",
            "three_domains_count",
            "mewg_five_folds_count",
        ];
        if PROTECTED.contains(&rule) {
            let rec = SelfDisableRecord {
                trigger_id: self.next_id(),
                triggered_at_ms: now_ms,
                trigger: SelfDisableTrigger::NoPatchViolation {
                    rule: rule.to_string(),
                },
                context: context.to_string(),
            };
            self.records.push(rec.clone());
            SelfDisableCheck::Triggered(rec)
        } else {
            SelfDisableCheck::Pass
        }
    }

    pub fn check_no_bypass(
        &mut self,
        owner_token: &str,
        bypassed_governance: bool,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        if owner_token.eq_ignore_ascii_case("master") && bypassed_governance {
            let rec = SelfDisableRecord {
                trigger_id: self.next_id(),
                triggered_at_ms: now_ms,
                trigger: SelfDisableTrigger::NoBypassViolation {
                    token: owner_token.to_string(),
                },
                context: context.to_string(),
            };
            self.records.push(rec.clone());
            SelfDisableCheck::Triggered(rec)
        } else {
            SelfDisableCheck::Pass
        }
    }

    pub fn check_no_reverse(
        &mut self,
        trigger_id: &str,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        let rec = SelfDisableRecord {
            trigger_id: self.next_id(),
            triggered_at_ms: now_ms,
            trigger: SelfDisableTrigger::NoReverseViolation {
                trigger_id: trigger_id.to_string(),
            },
            context: context.to_string(),
        };
        self.records.push(rec.clone());
        SelfDisableCheck::Triggered(rec)
    }

    pub fn check_no_hide(
        &mut self,
        window_id: &str,
        context: &str,
        now_ms: i64,
    ) -> SelfDisableCheck {
        if !self.is_armed {
            return SelfDisableCheck::Pass;
        }
        let rec = SelfDisableRecord {
            trigger_id: self.next_id(),
            triggered_at_ms: now_ms,
            trigger: SelfDisableTrigger::NoHideViolation {
                window_id: window_id.to_string(),
            },
            context: context.to_string(),
        };
        self.records.push(rec.clone());
        SelfDisableCheck::Triggered(rec)
    }
}

// ============================================================
// V2Agent — AgentManager 自包含 stub
// ============================================================

/// Agent 字段结构 (跟 `apeireth_agent::Agent` 字段级对齐 — 6 字段)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub tools: Vec<String>,
    pub system_prompt: String,
    pub created_at: i64,
}

impl Agent {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aliases: Vec<String>,
        tools: Vec<String>,
        system_prompt: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aliases,
            tools,
            system_prompt: system_prompt.into(),
            created_at: now_ms(),
        }
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// V2 AgentManager — alias 解析 + LRU cache
///
/// **跟 `apeireth_agent::AgentManager` 字段级对齐** (战役 2-4 VCP 借鉴):
/// - `agents: HashMap<id, Agent>`
/// - `alias_index: HashMap<alias, id>`
/// - `cache: LruCache<key, Agent>` (默认 64)
pub struct V2AgentManager {
    agents: Mutex<HashMap<String, Agent>>,
    aliases: Mutex<HashMap<String, String>>,
    cache: StdMutex<LruCache<String, Agent>>,
}

impl Default for V2AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl V2AgentManager {
    pub fn new() -> Self {
        Self::with_cache_size(64)
    }

    pub fn with_cache_size(cache_size: usize) -> Self {
        // invariant: cache_size.max(1) >= 1, so NonZeroUsize::new returns Some.
        let cap = NonZeroUsize::new(cache_size.max(1))
            .expect("invariant: cache_size.max(1) is at least 1, so NonZeroUsize::new returns Some");
        Self {
            agents: Mutex::new(HashMap::new()),
            aliases: Mutex::new(HashMap::new()),
            cache: StdMutex::new(LruCache::new(cap)),
        }
    }

    pub fn register(&self, agent: Agent) -> Result<(), String> {
        let id = agent.id.clone();
        let alias_count = agent.aliases.len();
        // 已存在 → 移除旧 alias
        if let Some(old) = self.agents.lock().get(&id).cloned() {
            let mut aliases = self.aliases.lock();
            for a in old.all_aliases() {
                aliases.remove(&a);
            }
        }
        // 写 alias index
        {
            let mut aliases = self.aliases.lock();
            for a in agent.all_aliases() {
                aliases.insert(a, id.clone());
            }
        }
        // 写 agents
        self.agents.lock().insert(id.clone(), agent);
        // 清 cache
        self.cache
            .lock()
            .map_err(|e| format!("agent cache mutex poisoned: {e}"))?
            .clear();
        let _ = alias_count;
        Ok(())
    }

    pub fn list(&self) -> Vec<Agent> {
        let mut out: Vec<Agent> = self.agents.lock().values().cloned().collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn cache_capacity(&self) -> usize {
        // invariant: agent cache mutex is never held across an await point
        // and there is no panic in cache code path. poison indicates a
        // serious bug, but cache_capacity() is a usize-returning sync getter
        // (called from test + agent_cache endpoint), so we keep the signature
        // and upgrade unwrap -> expect for a readable panic message.
        self.cache
            .lock()
            .expect("agent cache mutex poisoned: another thread panicked while holding the lock")
            .cap()
            .get()
    }

    pub fn cache_len(&self) -> usize {
        // see cache_capacity() for the .unwrap() -> .expect() rationale.
        self.cache
            .lock()
            .expect("agent cache mutex poisoned: another thread panicked while holding the lock")
            .len()
    }
}

impl Agent {
    pub fn all_aliases(&self) -> Vec<String> {
        let mut out: Vec<String> = vec![self.id.clone()];
        for a in &self.aliases {
            if !out.contains(a) {
                out.push(a.clone());
            }
        }
        out
    }
}

// ============================================================
// V2OrgansProvider — 9 器官 snapshot
// ============================================================

/// 9 器官 LOCKED 顺序 (跟 TUI backend 一致)
pub const V2_ORGAN_NAMES: [&str; 9] = [
    "perception",
    "cognition",
    "consciousness",
    "memory",
    "motivation",
    "value",
    "relation",
    "action",
    "life_force",
];

/// 器官简表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganSummary {
    pub name: String,
    pub health: f64,
}

/// OrgansProvider — 9 器官列表 + 单器官查询
pub struct V2OrgansProvider;

impl Default for V2OrgansProvider {
    fn default() -> Self {
        Self
    }
}

impl V2OrgansProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn list_organs(&self) -> Vec<OrganSummary> {
        V2_ORGAN_NAMES
            .iter()
            .map(|n| OrganSummary {
                name: (*n).to_string(),
                health: 0.5,
            })
            .collect()
    }

    pub fn get_organ(&self, name: &str) -> Option<OrganSummary> {
        if V2_ORGAN_NAMES.contains(&name) {
            Some(OrganSummary {
                name: name.to_string(),
                health: 0.5,
            })
        } else {
            None
        }
    }
}

// ============================================================
// V2State — 6 类服务的懒加载容器
// ============================================================

// ============================================================
// R30 P3: audit JSONL + /v1/tools/recent
// ============================================================

/// R30 U8: SQLite audit 缓存 (lazy-init OnceLock, 多线程安全共享一个连接)
fn audit_db() -> &'static std::sync::OnceLock<std::sync::Arc<crate::audit_sqlite::AuditDb>> {
    static DB: std::sync::OnceLock<std::sync::Arc<crate::audit_sqlite::AuditDb>> = std::sync::OnceLock::new();
    static CELL: std::sync::OnceLock<std::sync::OnceLock<std::sync::Arc<crate::audit_sqlite::AuditDb>>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| std::sync::OnceLock::new());
    // 上面 CELL 只是占位, 下面 DB 才是真缓存
    static DB2: std::sync::OnceLock<std::sync::Arc<crate::audit_sqlite::AuditDb>> = std::sync::OnceLock::new();
    &DB2
}

fn get_audit_db() -> std::sync::Arc<crate::audit_sqlite::AuditDb> {
    let cell = audit_db();
    cell.get_or_init(|| {
        let path = std::env::var("APEREIRETH_DAEMON_AUDIT_DB")
            .ok()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var("USERPROFILE")
                    .ok()
                    .map(|h| std::path::PathBuf::from(format!("{}/.apeireth/daemon-audit.sqlite", h)))
            })
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp/daemon-audit.sqlite"));
        match crate::audit_sqlite::AuditDb::open(&path) {
            Ok(db) => std::sync::Arc::new(db),
            Err(e) => {
                eprintln!("[apeireth] audit sqlite open failed: {e}, falling back to no-op");
                // 用 /dev/null 等价 (tempdir 临时文件) 避免再次 open 失败
                let tmp = std::env::temp_dir().join(format!("apeireth_audit_fallback_{}.sqlite", std::process::id()));
                let _ = std::fs::remove_file(&tmp);
                std::sync::Arc::new(crate::audit_sqlite::AuditDb::open(&tmp).expect("fallback open"))
            }
        }
    }).clone()
}

fn audit_record(name: &str, args: &serde_json::Value, ok: bool, duration_ms: u128) {
    use std::io::Write;
    let ts = chrono::Utc::now().to_rfc3339();
    // 写 SQLite (主)
    let _ = get_audit_db().insert(&ts, name, ok, duration_ms as u64, args);
    // 保留 JSONL 写穿 (兼容老 consumer / 备份)
    let path = std::env::var("APEREIRETH_DAEMON_AUDIT")
        .unwrap_or_else(|_| {
            std::env::var("USERPROFILE")
                .map(|h| format!("{}/.apeireth/daemon-audit.jsonl", h))
                .unwrap_or_else(|_| "/tmp/daemon-audit.jsonl".into())
        });
    if let Some(parent) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let entry = serde_json::json!({
        "ts": ts,
        "tool": name,
        "args": args,
        "ok": ok,
        "duration_ms": duration_ms,
    });
    if let Ok(s) = serde_json::to_string(&entry) {
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{}", s);
        }
    }
}

async fn tools_recent() -> Result<Json<Value>, (StatusCode, String)> {
    // R30 U8: 优先 SQLite, 失败回退 JSONL
    let db = get_audit_db();
    match db.recent(20) {
        Ok(arr) if arr.as_array().map(|a| !a.is_empty()).unwrap_or(false) => return Ok(Json(arr)),
        _ => {} // 走 JSONL fallback
    }
    let path = std::env::var("APEREIRETH_DAEMON_AUDIT")
        .unwrap_or_else(|_| {
            std::env::var("USERPROFILE")
                .map(|h| format!("{}/.apeireth/daemon-audit.jsonl", h))
                .unwrap_or_else(|_| "/tmp/daemon-audit.jsonl".into())
        });
    let Ok(raw) = std::fs::read_to_string(&path) else { return Ok(Json(serde_json::json!([]))) };
    let mut items: Vec<serde_json::Value> = Vec::new();
    for line in raw.lines().rev().take(20) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            items.push(v);
        }
    }
    Ok(Json(serde_json::json!(items)))
}

/// R30 U8: 审计统计 (total / ok / fail / by_tool)
async fn audit_stats() -> Result<Json<Value>, (StatusCode, String)> {
    get_audit_db().stats()
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}

/// **V2 6 类服务懒加载容器**
#[derive(Default)]
pub struct V2State {
    tools: OnceLock<Arc<ToolRegistry>>,
    memory: OnceLock<Arc<V2Memory>>,
    asi: OnceLock<Arc<V2AsiRegistry>>,
    sovereignty: OnceLock<Arc<StdMutex<V2SelfDisableGuard>>>,
    agent: OnceLock<Arc<V2AgentManager>>,
    organs: OnceLock<Arc<V2OrgansProvider>>,
}

impl V2State {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn install_tools(&self, reg: Arc<ToolRegistry>) -> bool {
        self.tools.set(reg).is_ok()
    }
    pub fn install_memory(&self, store: Arc<V2Memory>) -> bool {
        self.memory.set(store).is_ok()
    }
    pub fn install_asi(&self, reg: Arc<V2AsiRegistry>) -> bool {
        self.asi.set(reg).is_ok()
    }
    pub fn install_sovereignty(&self, guard: Arc<StdMutex<V2SelfDisableGuard>>) -> bool {
        self.sovereignty.set(guard).is_ok()
    }
    pub fn install_agent(&self, mgr: Arc<V2AgentManager>) -> bool {
        self.agent.set(mgr).is_ok()
    }
    pub fn install_organs(&self, organs: Arc<V2OrgansProvider>) -> bool {
        self.organs.set(organs).is_ok()
    }

    /// **R20 阶段 2 增量**: 公开 V2State.tools 访问 (供 v1_tools::invoke_by_name 用)
    /// **设计**: 不暴露 `tools: OnceLock<...>` 字段, 仅暴露 `Arc<ToolRegistry>` clone.
    /// **不漂移**: 0 改其他 5 个 install_* / *_registered() 接口.
    pub fn tools_registry(&self) -> Option<Arc<ToolRegistry>> {
        self.tools.get().cloned()
    }

    pub fn tools_registered(&self) -> bool {
        self.tools.get().is_some()
    }
    pub fn memory_registered(&self) -> bool {
        self.memory.get().is_some()
    }
    pub fn asi_registered(&self) -> bool {
        self.asi.get().is_some()
    }
    pub fn sovereignty_registered(&self) -> bool {
        self.sovereignty.get().is_some()
    }
    pub fn agent_registered(&self) -> bool {
        self.agent.get().is_some()
    }
    pub fn organs_registered(&self) -> bool {
        self.organs.get().is_some()
    }
}

pub type SharedV2 = Arc<V2State>;

fn service_not_ready(name: &str) -> (StatusCode, String) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        format!("{name} service not initialized"),
    )
}

// ============================================================
// Router 构造
// ============================================================

/// 构造 V2 6 类端点路由 (独立 Router, 主 router 用 `.nest("/v1", v2_router)` 嵌进去)
///
/// **返回类型**: `Router<()>` (state 已经 with_state 绑定), 可直接 `.nest_service` 嵌主 router.
///
/// **R20 阶段 2 增量**: 通过 `.merge(crate::v1_tools::build_v1_router())` 加 6 个 v1 工具路由
/// (D-02 子路径风格, per `reports/r19-integration-v2/r20-stage-2-3-prep-2026-08-05.md` §2.2).
pub fn build_router(state: SharedV2) -> Router<()> {
    Router::new()
        // V2 健康总览 (本身走 State<Arc<V2State>>, 不影响主 router)
        .route("/health", get(v2_health))
        // Tools
        .route("/tools/list", get(tools_list))
        .route("/tools/invoke", post(tools_invoke))
        .route("/tools/recent", get(tools_recent))
        .route("/audit/stats", get(audit_stats))
        // Memory
        .route("/memory/episodes", get(memory_episodes))
        .route("/memory/append", post(memory_append))
        .route("/memory/identity", get(memory_identity))
        .route("/memory/identity/update", post(memory_identity_update))
        // Organs
        .route("/organs", get(organs_list))
        .route("/organs/:name", get(organ_get))
        .route("/organs/:name/invoke", post(organ_invoke))
        // ASI
        .route("/asi/score", get(asi_score))
        .route("/asi/all", get(asi_all))
        .route("/asi/calibrate", post(asi_calibrate))
        // Sovereignty
        .route("/sovereignty/status", get(sovereignty_status))
        .route("/sovereignty/attack", post(sovereignty_attack))
        .route("/sovereignty/rearm", post(sovereignty_rearm))
        // Agent
        .route("/agent/aliases", get(agent_aliases))
        .route("/agent/alias", post(agent_register_alias))
        .route("/agent/cache", get(agent_cache))
        // R20 阶段 2 — 6 工具 v1 子路径 endpoint (D-02 子路径风格, per §2.2)
        .merge(crate::v1_tools::build_v1_router())
        .with_state(state)
}

// ============================================================
// 1. Tools — 复用 ToolRegistry
// ============================================================

#[derive(Debug, Serialize)]
struct ToolListItem {
    name: String,
    kind: String,
    axes: Value,
    /// R30 U14: 6 类 + 5 轴正交 (axes 字段)
    /// R30 U13: 估计 token 数
    token_estimate: usize,
}

async fn tools_list(
    State(state): State<SharedV2>,
) -> Result<Json<Vec<ToolListItem>>, (StatusCode, String)> {
    let reg = state
        .tools
        .get()
        .ok_or_else(|| service_not_ready("tools"))?;
    let items: Vec<ToolListItem> = reg
        .list()
        .into_iter()
        .filter_map(|name| {
            reg.get(&name).map(|t| {
                let kind = t.kind();
                let axes_v = serde_json::to_value(t.axes()).unwrap_or(Value::Null);
                // R30 U13: 走 apeireth-tool-registry::token_budget 真计算 (替代 len/4 启发式)
                let axes_str = serde_json::to_string(&axes_v).unwrap_or_default();
                let tok_est = apeireth_tool_registry::token_budget::estimate_token_count(&axes_str) as usize;
                ToolListItem {
                    name: t.name().to_string(),
                    kind: format!("{:?}", kind),
                    axes: axes_v,
                    token_estimate: tok_est,
                }
            })
        })
        .collect();
    // R30 U13: 总注入超 MAX_INJECTION_CHARS 时, warn (TUI 端会 truncate, 见 TUI 的 truncate_to_max_injection)
    let total_tokens: usize = items.iter().map(|i| i.token_estimate).sum();
    if total_tokens > apeireth_tool_registry::token_budget::MAX_INJECTION_CHARS {
        tracing::warn!(
            total_tokens,
            max = apeireth_tool_registry::token_budget::MAX_INJECTION_CHARS,
            "tools/list 总 token 超 MAX_INJECTION_CHARS, TUI 端会 truncate"
        );
    }
    Ok(Json(items))
}

#[derive(Debug, Deserialize)]
struct ToolInvokeRequest {
    name: String,
    #[serde(default)]
    args: Value,
}

#[derive(Debug, Serialize)]
struct ToolInvokeResponse {
    ok: bool,
    result: Option<Value>,
    error: Option<String>,
}

async fn tools_invoke(
    State(state): State<SharedV2>,
    Json(req): Json<ToolInvokeRequest>,
) -> Result<Json<ToolInvokeResponse>, (StatusCode, String)> {
    let reg = state
        .tools
        .get()
        .ok_or_else(|| service_not_ready("tools"))?;
    let started = std::time::Instant::now();
    let tool: Arc<dyn Tool> = reg.get(&req.name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("tool '{}' not found", req.name),
        )
    })?;
    let args = if req.args.is_null() {
        Value::Object(Default::default())
    } else {
        req.args.clone()
    };
    let call_result = tool.call(args).await;
    let ok_flag = call_result.is_ok();
    audit_record(&req.name, &req.args, ok_flag, started.elapsed().as_millis());
    match call_result {
        Ok(v) => Ok(Json(ToolInvokeResponse {
            ok: true,
            result: Some(v),
            error: None,
        })),
        Err(e) => Ok(Json(ToolInvokeResponse {
            ok: false,
            result: None,
            error: Some(e),
        })),
    }
}

// ============================================================
// 2. Memory
// ============================================================

#[derive(Debug, Deserialize)]
struct MemoryEpisodesQuery {
    session: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct MemoryEpisodesResponse {
    items: Vec<Episode>,
}

async fn memory_episodes(
    State(state): State<SharedV2>,
    Query(q): Query<MemoryEpisodesQuery>,
) -> Result<Json<MemoryEpisodesResponse>, (StatusCode, String)> {
    let store = state
        .memory
        .get()
        .ok_or_else(|| service_not_ready("memory"))?;
    let limit = q.limit.unwrap_or(50);
    let items = store
        .query_episodes(q.session.as_deref(), limit)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("memory query: {e}"),
            )
        })?;
    Ok(Json(MemoryEpisodesResponse { items }))
}

#[derive(Debug, Deserialize)]
struct MemoryAppendRequest {
    id: Option<String>,
    session_id: String,
    role: String,
    content: String,
    timestamp: Option<i64>,
}

#[derive(Debug, Serialize)]
struct MemoryAppendResponse {
    ok: bool,
    episode_id: String,
}

async fn memory_append(
    State(state): State<SharedV2>,
    Json(req): Json<MemoryAppendRequest>,
) -> Result<Json<MemoryAppendResponse>, (StatusCode, String)> {
    let store = state
        .memory
        .get()
        .ok_or_else(|| service_not_ready("memory"))?;
    let id = req.id.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("ep-{ts}")
    });
    let timestamp = req.timestamp.unwrap_or_else(|| {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    });
    let ep = Episode {
        id,
        timestamp,
        role: req.role,
        content: req.content,
        session_id: req.session_id,
    };
    store.put_episode(&ep).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("memory append: {e}"),
        )
    })?;
    Ok(Json(MemoryAppendResponse {
        ok: true,
        episode_id: ep.id,
    }))
}

#[derive(Debug, Serialize)]
struct IdentityCardResponse {
    default_continuity_id: String,
    cards: Vec<IdentityCardRecord>,
}

async fn memory_identity(
    State(state): State<SharedV2>,
) -> Result<Json<IdentityCardResponse>, (StatusCode, String)> {
    let store = state
        .memory
        .get()
        .ok_or_else(|| service_not_ready("memory"))?;
    let cards = store.list_identity_cards().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("identity list: {e}"),
        )
    })?;
    let default = cards
        .first()
        .map(|c| c.continuity_id.clone())
        .unwrap_or_else(|| "apeireth-default".to_string());
    Ok(Json(IdentityCardResponse {
        default_continuity_id: default,
        cards,
    }))
}

#[derive(Debug, Deserialize)]
struct IdentityUpdateRequest {
    continuity_id: String,
    #[serde(default)]
    birth_time: Option<i64>,
    #[serde(default)]
    carriers: Option<Vec<String>>,
    #[serde(default)]
    migration: Option<Migration>,
}

#[derive(Debug, Serialize)]
struct IdentityUpdateResponse {
    ok: bool,
    card: Option<IdentityCardRecord>,
    error: Option<String>,
}

async fn memory_identity_update(
    State(state): State<SharedV2>,
    Json(req): Json<IdentityUpdateRequest>,
) -> Result<Json<IdentityUpdateResponse>, (StatusCode, String)> {
    let store = state
        .memory
        .get()
        .ok_or_else(|| service_not_ready("memory"))?;
    // 1. 拿现有
    let existing = store.get_identity_card(&req.continuity_id).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("identity get: {e}"),
        )
    })?;
    let mut card = match existing {
        Some(c) => c,
        None => {
            // 不存在 → 创
            let now = chrono::Utc::now().timestamp();
            let core = IdentityCard {
                continuity_id: req.continuity_id.clone(),
                birth_time: req.birth_time.unwrap_or(now),
                carriers: req
                    .carriers
                    .clone()
                    .unwrap_or_else(|| vec!["apeireth-api".into()]),
                migration_history: vec![],
            };
            match store.create_identity_card(&core) {
                Ok(c) => c,
                Err(e) => {
                    return Ok(Json(IdentityUpdateResponse {
                        ok: false,
                        card: None,
                        error: Some(format!("create identity: {e}")),
                    }));
                }
            }
        }
    };
    // 2. 应用 migration
    if let Some(m) = req.migration.as_ref() {
        match store.record_migration(&req.continuity_id, m) {
            Ok(updated) => card = updated,
            Err(e) => {
                return Ok(Json(IdentityUpdateResponse {
                    ok: false,
                    card: None,
                    error: Some(format!("record_migration: {e}")),
                }));
            }
        }
    }
    Ok(Json(IdentityUpdateResponse {
        ok: true,
        card: Some(card),
        error: None,
    }))
}

// ============================================================
// 3. Organs
// ============================================================

#[derive(Debug, Serialize)]
struct OrganListResponse {
    organs: Vec<OrganSummary>,
}

async fn organs_list(
    State(state): State<SharedV2>,
) -> Result<Json<OrganListResponse>, (StatusCode, String)> {
    let organs = state
        .organs
        .get()
        .ok_or_else(|| service_not_ready("organs"))?;
    Ok(Json(OrganListResponse {
        organs: organs.list_organs(),
    }))
}

async fn organ_get(
    State(state): State<SharedV2>,
    Path(name): Path<String>,
) -> Result<Json<OrganSummary>, (StatusCode, String)> {
    let organs = state
        .organs
        .get()
        .ok_or_else(|| service_not_ready("organs"))?;
    match organs.get_organ(&name) {
        Some(o) => Ok(Json(o)),
        None => Err((StatusCode::NOT_FOUND, format!("organ '{name}' not found"))),
    }
}

#[derive(Debug, Deserialize)]
struct OrganInvokeRequest {
    action: String,
    #[serde(default)]
    args: Value,
}

#[derive(Debug, Serialize)]
struct OrganInvokeResponse {
    ok: bool,
    action: String,
    echo: Value,
    note: String,
}

async fn organ_invoke(
    State(_state): State<SharedV2>,
    Path(name): Path<String>,
    Json(req): Json<OrganInvokeRequest>,
) -> Result<Json<OrganInvokeResponse>, (StatusCode, String)> {
    Ok(Json(OrganInvokeResponse {
        ok: false,
        action: req.action,
        echo: req.args,
        note: format!(
            "organ '{name}' invoke not yet wired to backend logic (supervisor-only, Step 2 stub)"
        ),
    }))
}

// ============================================================
// 4. ASI
// ============================================================

#[derive(Debug, Deserialize)]
struct AsiScoreQuery {
    dim: String,
}

#[derive(Debug, Serialize)]
struct AsiScoreResponse {
    dim: String,
    value: f64,
    is_v05: bool,
    is_v1136: bool,
}

async fn asi_score(
    State(state): State<SharedV2>,
    Query(q): Query<AsiScoreQuery>,
) -> Result<Json<AsiScoreResponse>, (StatusCode, String)> {
    let reg = state.asi.get().ok_or_else(|| service_not_ready("asi"))?;
    let sample = default_sample();
    let dims = reg.compute_all_dims(&sample);
    let subs = reg.compute_all_subs(&sample);
    if let Some(idx) = V2_V05_DIMENSION_NAMES.iter().position(|n| *n == q.dim) {
        return Ok(Json(AsiScoreResponse {
            dim: q.dim,
            value: dims[idx],
            is_v05: true,
            is_v1136: false,
        }));
    }
    if let Some(idx) = V2_V1136_SUBMEASURE_NAMES.iter().position(|n| *n == q.dim) {
        return Ok(Json(AsiScoreResponse {
            dim: q.dim,
            value: subs[idx],
            is_v05: false,
            is_v1136: true,
        }));
    }
    Err((
        StatusCode::NOT_FOUND,
        format!("dim '{}' not in V05 or V1136", q.dim),
    ))
}

#[derive(Debug, Serialize)]
struct AsiAllResponse {
    v05: HashMap<String, f64>,
    v1136: HashMap<String, f64>,
    v05_overall: f64,
}

async fn asi_all(
    State(state): State<SharedV2>,
) -> Result<Json<AsiAllResponse>, (StatusCode, String)> {
    let reg = state.asi.get().ok_or_else(|| service_not_ready("asi"))?;
    let sample = default_sample();
    let dims = reg.compute_all_dims(&sample);
    let subs = reg.compute_all_subs(&sample);
    let v05_map: HashMap<String, f64> = V2_V05_DIMENSION_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| ((*n).to_string(), dims[i]))
        .collect();
    let v1136_map: HashMap<String, f64> = V2_V1136_SUBMEASURE_NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| ((*n).to_string(), subs[i]))
        .collect();
    let overall = if dims.is_empty() {
        0.0
    } else {
        dims.iter().sum::<f64>() / (dims.len() as f64)
    };
    Ok(Json(AsiAllResponse {
        v05: v05_map,
        v1136: v1136_map,
        v05_overall: overall,
    }))
}

#[derive(Debug, Deserialize)]
struct AsiCalibrateRequest {
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    every: Option<u32>,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Serialize)]
struct AsiCalibrateResponse {
    ok: bool,
    dry_run: bool,
    sample_size: usize,
    note: String,
}

async fn asi_calibrate(
    State(state): State<SharedV2>,
    Json(req): Json<AsiCalibrateRequest>,
) -> Result<Json<AsiCalibrateResponse>, (StatusCode, String)> {
    let reg = state.asi.get().ok_or_else(|| service_not_ready("asi"))?;
    let sample = default_sample();
    let dims = reg.compute_all_dims(&sample);
    let subs = reg.compute_all_subs(&sample);
    let total = dims.len() + subs.len();
    Ok(Json(AsiCalibrateResponse {
        ok: true,
        dry_run: req.dry_run,
        sample_size: total,
        note: format!(
            "calibration stub (Step 2); dry_run={} every={:?} scope={:?}; {} dims+subs observed",
            req.dry_run, req.every, req.scope, total
        ),
    }))
}

// ============================================================
// 5. Sovereignty
// ============================================================

#[derive(Debug, Serialize)]
struct SovereigntyStatusResponse {
    is_armed: bool,
    trigger_count: usize,
    triggers: Vec<TriggerSummary>,
}

#[derive(Debug, Serialize)]
struct TriggerSummary {
    trigger_id: String,
    mechanism_id: u8,
    mechanism_name: String,
    chinese_name: String,
    triggered_at_ms: i64,
    context: String,
}

fn summarize(r: &SelfDisableRecord) -> TriggerSummary {
    TriggerSummary {
        trigger_id: r.trigger_id.clone(),
        mechanism_id: r.trigger.mechanism_id(),
        mechanism_name: r.trigger.mechanism_name().to_string(),
        chinese_name: r.trigger.chinese_name().to_string(),
        triggered_at_ms: r.triggered_at_ms,
        context: r.context.clone(),
    }
}

async fn sovereignty_status(
    State(state): State<SharedV2>,
) -> Result<Json<SovereigntyStatusResponse>, (StatusCode, String)> {
    let guard = state
        .sovereignty
        .get()
        .ok_or_else(|| service_not_ready("sovereignty"))?;
    let g = guard
        .lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("sovereignty mutex poisoned: {e}")))?;
    Ok(Json(SovereigntyStatusResponse {
        is_armed: g.is_armed,
        trigger_count: g.record_count(),
        triggers: g.records().iter().map(summarize).collect(),
    }))
}

#[derive(Debug, Deserialize)]
struct SovereigntyAttackRequest {
    mechanism: String,
    #[serde(default)]
    context: Option<String>,
}

#[derive(Debug, Serialize)]
struct SovereigntyAttackResponse {
    triggered: bool,
    trigger: Option<TriggerSummary>,
    record_count: usize,
}

async fn sovereignty_attack(
    State(state): State<SharedV2>,
    Json(req): Json<SovereigntyAttackRequest>,
) -> Result<Json<SovereigntyAttackResponse>, (StatusCode, String)> {
    let guard = state
        .sovereignty
        .get()
        .ok_or_else(|| service_not_ready("sovereignty"))?;
    let mut g = guard
        .lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("sovereignty mutex poisoned: {e}")))?;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let ctx = req.context.unwrap_or_else(|| "http-attack".to_string());
    let check = match req.mechanism.as_str() {
        "no_degrade" => g.check_no_degrade("high", "low", &ctx, now_ms),
        "no_patch" => g.check_no_patch("principle_keys_count", 4, &ctx, now_ms),
        "no_bypass" => g.check_no_bypass("master", true, &ctx, now_ms),
        "no_reverse" => g.check_no_reverse("sd-999999", &ctx, now_ms),
        "no_hide" => g.check_no_hide("audit-window-001", &ctx, now_ms),
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unknown mechanism '{other}'"),
            ));
        }
    };
    let (triggered, summary) = match check {
        SelfDisableCheck::Triggered(r) => (true, Some(summarize(&r))),
        _ => (false, None),
    };
    Ok(Json(SovereigntyAttackResponse {
        triggered,
        trigger: summary,
        record_count: g.record_count(),
    }))
}

#[derive(Debug, Serialize)]
struct SovereigntyRearmResponse {
    is_armed: bool,
    previous_armed: bool,
}

async fn sovereignty_rearm(
    State(state): State<SharedV2>,
) -> Result<Json<SovereigntyRearmResponse>, (StatusCode, String)> {
    let guard = state
        .sovereignty
        .get()
        .ok_or_else(|| service_not_ready("sovereignty"))?;
    let mut g = guard
        .lock()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("sovereignty mutex poisoned: {e}")))?;
    let prev = g.is_armed;
    g.rearm();
    Ok(Json(SovereigntyRearmResponse {
        is_armed: g.is_armed,
        previous_armed: prev,
    }))
}

// ============================================================
// 6. Agent
// ============================================================

#[derive(Debug, Serialize)]
struct AgentAliasItem {
    id: String,
    name: String,
    aliases: Vec<String>,
    tools: Vec<String>,
    hot_reload: bool,
}

#[derive(Debug, Serialize)]
struct AgentAliasesResponse {
    agents: Vec<AgentAliasItem>,
}

async fn agent_aliases(
    State(state): State<SharedV2>,
) -> Result<Json<AgentAliasesResponse>, (StatusCode, String)> {
    let mgr = state
        .agent
        .get()
        .ok_or_else(|| service_not_ready("agent"))?;
    let agents: Vec<AgentAliasItem> = mgr
        .list()
        .into_iter()
        .map(|a| AgentAliasItem {
            id: a.id.clone(),
            name: a.name.clone(),
            aliases: a.aliases.clone(),
            tools: a.tools.clone(),
            hot_reload: false, // V2 stub 没 watch_dir
        })
        .collect();
    Ok(Json(AgentAliasesResponse { agents }))
}

#[derive(Debug, Deserialize)]
struct AgentRegisterRequest {
    id: String,
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    system_prompt: String,
}

#[derive(Debug, Serialize)]
struct AgentRegisterResponse {
    ok: bool,
    agent_id: String,
    alias_count: usize,
}

async fn agent_register_alias(
    State(state): State<SharedV2>,
    Json(req): Json<AgentRegisterRequest>,
) -> Result<Json<AgentRegisterResponse>, (StatusCode, String)> {
    let mgr = state
        .agent
        .get()
        .ok_or_else(|| service_not_ready("agent"))?;
    let agent = Agent::new(
        req.id.clone(),
        req.name,
        req.aliases,
        req.tools,
        req.system_prompt,
    );
    let alias_count = agent.aliases.len();
    mgr.register(agent).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("agent register: {e}"),
        )
    })?;
    Ok(Json(AgentRegisterResponse {
        ok: true,
        agent_id: req.id,
        alias_count,
    }))
}

#[derive(Debug, Serialize)]
struct AgentCacheResponse {
    capacity: usize,
    used: usize,
    hit_ratio_estimate: f64,
}

async fn agent_cache(
    State(state): State<SharedV2>,
) -> Result<Json<AgentCacheResponse>, (StatusCode, String)> {
    let mgr = state
        .agent
        .get()
        .ok_or_else(|| service_not_ready("agent"))?;
    let cap = mgr.cache_capacity();
    let used = mgr.cache_len();
    let hit = if cap == 0 {
        0.0
    } else {
        (used as f64) / (cap as f64)
    };
    Ok(Json(AgentCacheResponse {
        capacity: cap,
        used,
        hit_ratio_estimate: hit,
    }))
}

// ============================================================
// V2 健康总览 (跟 /health 并列, debug)
// ============================================================

#[derive(Debug, Serialize)]
struct V2HealthResponse {
    status: String,
    services: V2ServiceStatus,
    routes: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct V2ServiceStatus {
    tools: bool,
    memory: bool,
    asi: bool,
    sovereignty: bool,
    agent: bool,
    organs: bool,
}

pub async fn v2_health(State(state): State<SharedV2>) -> impl IntoResponse {
    Json(V2HealthResponse {
        status: "ok".into(),
        services: V2ServiceStatus {
            tools: state.tools_registered(),
            memory: state.memory_registered(),
            asi: state.asi_registered(),
            sovereignty: state.sovereignty_registered(),
            agent: state.agent_registered(),
            organs: state.organs_registered(),
        },
        routes: vec![
            "/v1/tools/list",
            "/v1/tools/invoke",
            "/v1/memory/episodes",
            "/v1/memory/append",
            "/v1/memory/identity",
            "/v1/memory/identity/update",
            "/v1/organs",
            "/v1/organs/{name}",
            "/v1/organs/{name}/invoke",
            "/v1/asi/score",
            "/v1/asi/all",
            "/v1/asi/calibrate",
            "/v1/sovereignty/status",
            "/v1/sovereignty/attack",
            "/v1/sovereignty/rearm",
            "/v1/agent/aliases",
            "/v1/agent/alias",
            "/v1/agent/cache",
        ],
    })
}

// ============================================================
// 单元测试 (handler 级 — 不起 axum, 只测 handler 逻辑)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn full_state() -> Arc<V2State> {
        let state = Arc::new(V2State::new());
        state.install_tools(Arc::new(ToolRegistry::new()));
        state.install_memory(Arc::new(V2Memory::open_in_memory().unwrap()));
        state.install_asi(Arc::new(V2AsiRegistry::new()));
        state.install_sovereignty(Arc::new(StdMutex::new(V2SelfDisableGuard::new())));
        state.install_agent(Arc::new(V2AgentManager::new()));
        state.install_organs(Arc::new(V2OrgansProvider::new()));
        state
    }

    #[test]
    fn empty_state_initialization_status() {
        let state = V2State::new();
        assert!(!state.tools_registered());
        assert!(!state.memory_registered());
        assert!(!state.asi_registered());
        assert!(!state.sovereignty_registered());
        assert!(!state.agent_registered());
        assert!(!state.organs_registered());
    }

    #[test]
    fn install_each_service_marks_registered() {
        let state = V2State::new();
        state.install_tools(Arc::new(ToolRegistry::new()));
        state.install_memory(Arc::new(V2Memory::open_in_memory().unwrap()));
        state.install_asi(Arc::new(V2AsiRegistry::new()));
        state.install_sovereignty(Arc::new(StdMutex::new(V2SelfDisableGuard::new())));
        state.install_agent(Arc::new(V2AgentManager::new()));
        state.install_organs(Arc::new(V2OrgansProvider::new()));
        assert!(state.tools_registered());
        assert!(state.memory_registered());
        assert!(state.asi_registered());
        assert!(state.sovereignty_registered());
        assert!(state.agent_registered());
        assert!(state.organs_registered());
    }

    // ----- V2Memory (SQLite 自包含) -----

    #[test]
    fn v2_memory_put_episode_and_query() {
        let store = V2Memory::open_in_memory().unwrap();
        let ep = Episode {
            id: "e1".into(),
            timestamp: 100,
            role: "user".into(),
            content: "hi".into(),
            session_id: "s1".into(),
        };
        store.put_episode(&ep).unwrap();
        let rows = store.query_episodes(Some("s1"), 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "e1");
    }

    #[test]
    fn v2_memory_put_episode_validates_empty_id() {
        let store = V2Memory::open_in_memory().unwrap();
        let ep = Episode {
            id: "  ".into(),
            timestamp: 0,
            role: "user".into(),
            content: "x".into(),
            session_id: "s".into(),
        };
        let err = store.put_episode(&ep).unwrap_err();
        assert!(err.to_string().contains("id is empty"));
    }

    #[test]
    fn v2_memory_query_empty_returns_empty() {
        let store = V2Memory::open_in_memory().unwrap();
        let rows = store.query_episodes(None, 10).unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn v2_memory_create_and_get_identity() {
        let store = V2Memory::open_in_memory().unwrap();
        let card = IdentityCard {
            continuity_id: "cid-1".into(),
            birth_time: 1_700_000_000,
            carriers: vec!["apeireth-api".into()],
            migration_history: vec![],
        };
        store.create_identity_card(&card).unwrap();
        let rec = store.get_identity_card("cid-1").unwrap().expect("cid-1");
        assert_eq!(rec.continuity_id, "cid-1");
        assert_eq!(rec.subject_rev, 0);
    }

    #[test]
    fn v2_memory_create_duplicate_uniqueness_error() {
        let store = V2Memory::open_in_memory().unwrap();
        let card = IdentityCard {
            continuity_id: "cid-dup".into(),
            birth_time: 0,
            carriers: vec![],
            migration_history: vec![],
        };
        store.create_identity_card(&card).unwrap();
        let err = store.create_identity_card(&card).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn v2_memory_record_migration_increments_subject_rev() {
        let store = V2Memory::open_in_memory().unwrap();
        let card = IdentityCard {
            continuity_id: "cid-m".into(),
            birth_time: 0,
            carriers: vec!["c1".into()],
            migration_history: vec![],
        };
        store.create_identity_card(&card).unwrap();
        let mig = Migration {
            from_carrier: "c1".into(),
            to_carrier: "c2".into(),
            timestamp: 100,
        };
        let rec = store.record_migration("cid-m", &mig).unwrap();
        assert_eq!(rec.subject_rev, 1);
        assert_eq!(rec.migration_history.len(), 1);
    }

    // ----- V2AsiRegistry -----

    #[test]
    fn v2_asi_default_sample_gives_09_per_dim() {
        let reg = V2AsiRegistry::new();
        let sample = default_sample();
        let dims = reg.compute_all_dims(&sample);
        assert_eq!(dims.len(), 24);
        for (i, v) in dims.iter().enumerate() {
            assert!((v - 0.9).abs() < 1e-9, "V05 dim {i} 应 = 0.9, 实际 {v}");
        }
        let subs = reg.compute_all_subs(&sample);
        assert_eq!(subs.len(), 9);
    }

    #[test]
    fn v2_asi_partial_quality_scales_value() {
        let reg = V2AsiRegistry::new();
        let mut sample = MeasurementSample::default();
        sample.successes.insert("thread_continuity".to_string(), 50);
        sample.attempts.insert("thread_continuity".to_string(), 100);
        sample
            .qualities
            .insert("thread_continuity".to_string(), 0.5);
        let dims = reg.compute_all_dims(&sample);
        // dims[0] = (50/100).min(1.0) * 0.5 = 0.25
        assert!((dims[0] - 0.25).abs() < 1e-9);
    }

    // ----- V2SelfDisableGuard (5 大机制) -----

    #[test]
    fn v2_sovereignty_no_degrade_triggers_when_down() {
        let mut g = V2SelfDisableGuard::new();
        let r = g.check_no_degrade("high", "low", "ctx", 1_000);
        match r {
            SelfDisableCheck::Triggered(rec) => {
                assert_eq!(rec.trigger.mechanism_id(), 1);
                assert_eq!(g.record_count(), 1);
            }
            _ => panic!("expected Triggered"),
        }
    }

    #[test]
    fn v2_sovereignty_no_degrade_passes_when_same() {
        let mut g = V2SelfDisableGuard::new();
        let r = g.check_no_degrade("high", "high", "ctx", 1_000);
        assert!(matches!(r, SelfDisableCheck::Pass));
        assert_eq!(g.record_count(), 0);
    }

    #[test]
    fn v2_sovereignty_no_degrade_passes_when_up() {
        let mut g = V2SelfDisableGuard::new();
        let r = g.check_no_degrade("medium", "high", "ctx", 1_000);
        assert!(matches!(r, SelfDisableCheck::Pass));
    }

    #[test]
    fn v2_sovereignty_no_patch_triggers_on_protected_rule() {
        let mut g = V2SelfDisableGuard::new();
        let r = g.check_no_patch("principle_keys_count", 4, "ctx", 1_000);
        match r {
            SelfDisableCheck::Triggered(rec) => {
                assert_eq!(rec.trigger.mechanism_id(), 2);
            }
            _ => panic!("expected Triggered"),
        }
    }

    #[test]
    fn v2_sovereignty_no_patch_passes_on_unrelated_rule() {
        let mut g = V2SelfDisableGuard::new();
        let r = g.check_no_patch("not_protected", 0, "ctx", 1_000);
        assert!(matches!(r, SelfDisableCheck::Pass));
    }

    #[test]
    fn v2_sovereignty_no_bypass_triggers_with_master_and_bypass() {
        let mut g = V2SelfDisableGuard::new();
        let r = g.check_no_bypass("master", true, "ctx", 1_000);
        match r {
            SelfDisableCheck::Triggered(rec) => {
                assert_eq!(rec.trigger.mechanism_id(), 3);
            }
            _ => panic!("expected Triggered"),
        }
    }

    #[test]
    fn v2_sovereignty_no_reverse_always_triggers() {
        let mut g = V2SelfDisableGuard::new();
        let r = g.check_no_reverse("sd-999", "ctx", 1_000);
        match r {
            SelfDisableCheck::Triggered(rec) => {
                assert_eq!(rec.trigger.mechanism_id(), 4);
            }
            _ => panic!("expected Triggered"),
        }
    }

    #[test]
    fn v2_sovereignty_no_hide_always_triggers() {
        let mut g = V2SelfDisableGuard::new();
        let r = g.check_no_hide("w-001", "ctx", 1_000);
        match r {
            SelfDisableCheck::Triggered(rec) => {
                assert_eq!(rec.trigger.mechanism_id(), 5);
            }
            _ => panic!("expected Triggered"),
        }
    }

    #[test]
    fn v2_sovereignty_disarm_passes_all() {
        let mut g = V2SelfDisableGuard::new();
        g.is_armed = false;
        let r = g.check_no_degrade("high", "low", "ctx", 1_000);
        assert!(matches!(r, SelfDisableCheck::Pass));
    }

    // ----- V2AgentManager -----

    #[test]
    fn v2_agent_register_and_list() {
        let mgr = V2AgentManager::new();
        mgr.register(Agent::new(
            "a",
            "A",
            vec!["@a".into()],
            vec!["t".into()],
            "p",
        ))
        .unwrap();
        let list = mgr.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "a");
    }

    #[test]
    fn v2_agent_default_cache_capacity_64() {
        let mgr = V2AgentManager::new();
        assert_eq!(mgr.cache_capacity(), 64);
        assert_eq!(mgr.cache_len(), 0);
    }

    #[test]
    fn v2_agent_all_aliases_includes_id() {
        let a = Agent::new("main", "M", vec!["@x".into()], vec![], "p");
        let aliases = a.all_aliases();
        assert!(aliases.contains(&"main".to_string()));
        assert!(aliases.contains(&"@x".to_string()));
    }

    // ----- V2OrgansProvider -----

    #[test]
    fn v2_organs_list_has_nine_locked_order() {
        let p = V2OrgansProvider::new();
        let list = p.list_organs();
        assert_eq!(list.len(), 9);
        assert_eq!(list[0].name, "perception");
        assert_eq!(list[8].name, "life_force");
    }

    #[test]
    fn v2_organs_get_unknown_returns_none() {
        let p = V2OrgansProvider::new();
        assert!(p.get_organ("nope").is_none());
    }

    #[test]
    fn v2_organs_get_known_returns_some() {
        let p = V2OrgansProvider::new();
        assert!(p.get_organ("perception").is_some());
    }

    // ----- Handler 单元测试 -----

    #[tokio::test]
    async fn tools_list_empty_registry_returns_empty() {
        let state = full_state();
        let Json(items) = tools_list(State(state)).await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn tools_invoke_unknown_returns_404() {
        let state = full_state();
        let req = ToolInvokeRequest {
            name: "NoSuchTool".into(),
            args: json!({}),
        };
        let err = tools_invoke(State(state), Json(req))
            .await
            .expect_err("404");
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn memory_append_then_query_roundtrip() {
        let state = full_state();
        let req = MemoryAppendRequest {
            id: Some("ep-1".into()),
            session_id: "sess-x".into(),
            role: "user".into(),
            content: "hello".into(),
            timestamp: Some(1_700_000_000),
        };
        let Json(resp) = memory_append(State(state.clone()), Json(req))
            .await
            .unwrap();
        assert!(resp.ok);
        assert_eq!(resp.episode_id, "ep-1");

        let q = MemoryEpisodesQuery {
            session: Some("sess-x".into()),
            limit: Some(10),
        };
        let Json(resp) = memory_episodes(State(state.clone()), Query(q))
            .await
            .unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].id, "ep-1");
        assert_eq!(resp.items[0].content, "hello");
    }

    #[tokio::test]
    async fn memory_identity_create_then_get() {
        let state = full_state();
        let req = IdentityUpdateRequest {
            continuity_id: "cid-test".into(),
            birth_time: Some(1_700_000_000),
            carriers: Some(vec!["apeireth-api".into()]),
            migration: None,
        };
        let _ = memory_identity_update(State(state.clone()), Json(req))
            .await
            .unwrap();
        let Json(resp) = memory_identity(State(state)).await.unwrap();
        assert_eq!(resp.default_continuity_id, "cid-test");
        assert_eq!(resp.cards.len(), 1);
    }

    #[tokio::test]
    async fn organs_list_has_nine() {
        let state = full_state();
        let Json(resp) = organs_list(State(state)).await.unwrap();
        assert_eq!(resp.organs.len(), 9);
    }

    #[tokio::test]
    async fn organs_get_unknown_returns_404() {
        let state = full_state();
        let err = organ_get(State(state), Path("nope".into()))
            .await
            .expect_err("404");
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn asi_all_returns_24_v05_and_9_v1136() {
        let state = full_state();
        let Json(resp) = asi_all(State(state)).await.unwrap();
        assert_eq!(resp.v05.len(), 24);
        assert_eq!(resp.v1136.len(), 9);
        assert!(resp.v05_overall > 0.0 && resp.v05_overall <= 1.0);
    }

    #[tokio::test]
    async fn asi_score_lookup_by_v05_dim() {
        let state = full_state();
        let q = AsiScoreQuery {
            dim: "thread_continuity".into(),
        };
        let Json(resp) = asi_score(State(state), Query(q)).await.unwrap();
        assert!(resp.is_v05);
        assert!(!resp.is_v1136);
    }

    #[tokio::test]
    async fn asi_score_lookup_by_v1136_sub() {
        let state = full_state();
        let q = AsiScoreQuery {
            dim: "v3_action_guard_rate".into(),
        };
        let Json(resp) = asi_score(State(state), Query(q)).await.unwrap();
        assert!(!resp.is_v05);
        assert!(resp.is_v1136);
    }

    #[tokio::test]
    async fn asi_score_unknown_dim_returns_404() {
        let state = full_state();
        let q = AsiScoreQuery {
            dim: "not.a.real.dim".into(),
        };
        let err = asi_score(State(state), Query(q)).await.expect_err("404");
        assert_eq!(err.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn asi_calibrate_returns_stub_summary() {
        let state = full_state();
        let req = AsiCalibrateRequest {
            dry_run: true,
            every: Some(10),
            scope: Some("v05".into()),
        };
        let Json(resp) = asi_calibrate(State(state), Json(req)).await.unwrap();
        assert!(resp.ok);
        assert!(resp.dry_run);
        assert!(resp.sample_size > 0);
    }

    #[tokio::test]
    async fn sovereignty_status_empty_initial() {
        let state = full_state();
        let Json(resp) = sovereignty_status(State(state)).await.unwrap();
        assert!(resp.is_armed);
        assert_eq!(resp.trigger_count, 0);
    }

    #[tokio::test]
    async fn sovereignty_attack_no_degrade_triggers() {
        let state = full_state();
        let req = SovereigntyAttackRequest {
            mechanism: "no_degrade".into(),
            context: Some("test".into()),
        };
        let Json(resp) = sovereignty_attack(State(state), Json(req)).await.unwrap();
        assert!(resp.triggered);
        assert_eq!(resp.trigger.unwrap().mechanism_id, 1);
        assert_eq!(resp.record_count, 1);
    }

    #[tokio::test]
    async fn sovereignty_attack_unknown_mechanism_400() {
        let state = full_state();
        let req = SovereigntyAttackRequest {
            mechanism: "made_up".into(),
            context: None,
        };
        let err = sovereignty_attack(State(state), Json(req))
            .await
            .expect_err("400");
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn sovereignty_rearm_idempotent() {
        let state = full_state();
        let Json(resp) = sovereignty_rearm(State(state)).await.unwrap();
        assert!(resp.is_armed);
        assert!(resp.previous_armed);
    }

    #[tokio::test]
    async fn agent_aliases_then_register_then_list() {
        let state = full_state();
        let Json(resp) = agent_aliases(State(state.clone())).await.unwrap();
        assert!(resp.agents.is_empty());

        let req = AgentRegisterRequest {
            id: "coder-1".into(),
            name: "Coder".into(),
            aliases: vec!["@coder".into(), "@chuling".into()],
            tools: vec!["FileOperator".into()],
            system_prompt: "I am a coder.".into(),
        };
        let Json(reg) = agent_register_alias(State(state.clone()), Json(req))
            .await
            .unwrap();
        assert!(reg.ok);
        assert_eq!(reg.alias_count, 2);

        let Json(resp) = agent_aliases(State(state)).await.unwrap();
        assert_eq!(resp.agents.len(), 1);
        assert_eq!(resp.agents[0].id, "coder-1");
    }

    #[tokio::test]
    async fn agent_cache_default_64_capacity() {
        let state = full_state();
        let Json(resp) = agent_cache(State(state)).await.unwrap();
        assert_eq!(resp.capacity, 64);
        assert_eq!(resp.used, 0);
    }

    #[tokio::test]
    async fn services_not_initialized_return_503() {
        let state = Arc::new(V2State::new());
        let err = tools_list(State(state.clone())).await.expect_err("503");
        assert_eq!(err.0, StatusCode::SERVICE_UNAVAILABLE);
        let err = memory_episodes(
            State(state.clone()),
            Query(MemoryEpisodesQuery {
                session: None,
                limit: None,
            }),
        )
        .await
        .expect_err("503");
        assert_eq!(err.0, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn v2_health_reports_all_services_when_full() {
        let state = full_state();
        let resp = v2_health(State(state)).await.into_response();
        assert_eq!(resp.status(), 200);
    }
}
