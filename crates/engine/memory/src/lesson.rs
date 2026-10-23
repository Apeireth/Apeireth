//! 教训对象（Lesson）—— 让「成长」有可验证的载体（L1：表 + 四拍状态机 + 五接口）。
//!
//! 设计页：`docs/04-internal/backend-enhancement/lesson-object-design-2026-10-22.md`。
//!
//! 四拍：**经历 → 候选教训 → 新场景验证 → 保留 / 撤回**。
//! 撤回也算成长，必须留痕；「没有出处的教训」按幻觉拒绝写入。
//!
//! # 四条硬约束（测试逐条钉死）
//!
//! 1. `source_episodes` 非空，且每个 id **必须真实存在于 episodes 表**
//!    —— 没有出处的教训是幻觉；
//! 2. `retracted` 必须带 `retract_reason`；
//! 3. `verified` 必须带 `verified_by` + `verified_at`，且
//!    **`verified_by` 不得是产生教训的源经历**（不能用同一次经历自证）；
//! 4. 状态变更走 `revision` CAS（与记忆治理面同款），冲突即拒。
//!
//! # 与调参日志的区别
//!
//! `tuning-log` 记「参数动了」；本模块记「认知变了」。参数移动 ≠ 学到东西，
//! 两者互不冒充。

/// `lessons` 表与索引（V18 迁移的 SQL 真源 —— 与读写代码同文件，避免两处漂移）。
pub(crate) const LESSON_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS lessons (
    id              TEXT PRIMARY KEY,
    statement       TEXT NOT NULL,
    scope           TEXT NOT NULL,
    source_episodes TEXT NOT NULL,
    evidence        TEXT NOT NULL,
    status          TEXT NOT NULL,
    verified_by     TEXT,
    verified_at     INTEGER,
    retract_reason  TEXT,
    revision        INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_lessons_status ON lessons(status, updated_at);
CREATE INDEX IF NOT EXISTS idx_lessons_scope  ON lessons(scope, updated_at);
"#;

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::SqliteMemoryStore;

/// 教训的适用场景（闭合词表：宁少不多，避免退化成自由文本标签）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonScope {
    /// 问题诊断类（报错、异常、边界条件）。
    Problem,
    /// 文件与产物类（路径、格式、编码）。
    File,
    /// 人际交互类（沟通时机、语气、边界）。
    Interaction,
    /// 工具与命令类（外部二进制、参数、环境）。
    Tool,
}

impl LessonScope {
    /// 落盘词形。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Problem => "problem",
            Self::File => "file",
            Self::Interaction => "interaction",
            Self::Tool => "tool",
        }
    }

    /// 从落盘词形解析（词表外一律拒绝）。
    pub fn parse(raw: &str) -> Result<Self, LessonError> {
        match raw {
            "problem" => Ok(Self::Problem),
            "file" => Ok(Self::File),
            "interaction" => Ok(Self::Interaction),
            "tool" => Ok(Self::Tool),
            other => Err(LessonError::UnknownScope {
                scope: other.to_string(),
            }),
        }
    }
}

/// 教训状态（四拍的三态；「经历」不在状态里，它是来源）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonStatus {
    /// 候选：从经历里提炼出来，尚未在新场景验证。**不注入上下文**。
    Candidate,
    /// 已验证：在新场景里被证实，可被召回。
    Verified,
    /// 已撤回：证据被推翻或场景失效。**永久保留**（它是「我错过、我改了」的证据）。
    Retracted,
}

impl LessonStatus {
    /// 落盘词形。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Verified => "verified",
            Self::Retracted => "retracted",
        }
    }

    /// 从落盘词形解析。
    pub fn parse(raw: &str) -> Result<Self, LessonError> {
        match raw {
            "candidate" => Ok(Self::Candidate),
            "verified" => Ok(Self::Verified),
            "retracted" => Ok(Self::Retracted),
            other => Err(LessonError::Invalid(format!("未知教训状态: {other}"))),
        }
    }
}

/// 一条教训（持久化视图）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lesson {
    /// 主键（`lsn-<uuid>`）。
    pub id: String,
    /// 一句话教训（可执行、可证伪）。
    pub statement: String,
    /// 适用场景。
    pub scope: LessonScope,
    /// 促成这条教训的经历 id（≥1）。
    pub source_episodes: Vec<String>,
    /// 当时的证据摘要（错误码 / 用户纠正原话 / 测试结果）。
    pub evidence: String,
    /// 当前状态。
    pub status: LessonStatus,
    /// 新场景验证用的经历/演练 id。
    pub verified_by: Option<String>,
    /// 验证时刻（epoch ms）。
    pub verified_at: Option<i64>,
    /// 撤回理由（撤回必填）。
    pub retract_reason: Option<String>,
    /// 修订号（CAS）。
    pub revision: i64,
    /// 创建时刻（epoch ms）。
    pub created_at: i64,
    /// 最近变更时刻（epoch ms）。
    pub updated_at: i64,
}

/// 各状态教训计数（自述面只读）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LessonCounts {
    /// 已验证（可被召回）。
    pub verified: u64,
    /// 候选（未验证，不注入上下文）。
    pub candidate: u64,
    /// 已撤回（留痕，不注入）。
    pub retracted: u64,
}

/// 新建候选教训的入参。
#[derive(Debug, Clone)]
pub struct NewLesson {
    /// 一句话教训。
    pub statement: String,
    /// 适用场景。
    pub scope: LessonScope,
    /// 促成它的经历 id（≥1，且必须真实存在）。
    pub source_episodes: Vec<String>,
    /// 证据摘要。
    pub evidence: String,
}

/// 教训面的错误（每条都对应一条硬约束或一个 CAS 冲突）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LessonError {
    /// 参数非法（空语句 / 空证据等）。
    Invalid(String),
    /// 没有出处：`source_episodes` 为空 —— 没有出处的教训按幻觉拒绝。
    MissingSources,
    /// 出处经历不存在（防编造）。
    UnknownSource(String),
    /// 场景词表外。
    UnknownScope {
        /// 传入的词形。
        scope: String,
    },
    /// 验证不得用来源经历自证。
    SelfVerification {
        /// 被同时用作来源与验证的 id。
        episode: String,
    },
    /// 撤回必须给理由。
    MissingRetractReason,
    /// 教训不存在。
    NotFound(String),
    /// 修订号冲突（并发写）。
    RevisionConflict {
        /// 期望修订号。
        expected: i64,
        /// 实际修订号。
        actual: i64,
    },
    /// 非法状态迁移。
    InvalidTransition {
        /// 当前状态。
        from: &'static str,
        /// 目标状态。
        to: &'static str,
    },
    /// 存储层错误。
    Storage(String),
}

impl std::fmt::Display for LessonError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "教训参数非法: {message}"),
            Self::MissingSources => {
                write!(
                    formatter,
                    "教训必须至少有一条来源经历（没有出处 = 幻觉，拒绝写入）"
                )
            }
            Self::UnknownSource(id) => write!(formatter, "来源经历不存在: {id}"),
            Self::UnknownScope { scope } => {
                write!(
                    formatter,
                    "场景词表外: {scope}（只认 problem/file/interaction/tool）"
                )
            }
            Self::SelfVerification { episode } => {
                write!(formatter, "验证不得用产生教训的那次经历自证: {episode}")
            }
            Self::MissingRetractReason => {
                write!(formatter, "撤回必须写明理由（撤回也是成长，要留痕）")
            }
            Self::NotFound(id) => write!(formatter, "教训不存在: {id}"),
            Self::RevisionConflict { expected, actual } => {
                write!(formatter, "修订号冲突: 期望 {expected}, 实际 {actual}")
            }
            Self::InvalidTransition { from, to } => {
                write!(formatter, "非法状态迁移: {from} → {to}")
            }
            Self::Storage(message) => write!(formatter, "教训存储失败: {message}"),
        }
    }
}

impl std::error::Error for LessonError {}

fn storage(error: impl std::fmt::Display) -> LessonError {
    LessonError::Storage(error.to_string())
}

fn decode_sources(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

const LESSON_COLS: &str = "id, statement, scope, source_episodes, evidence, status, \
     verified_by, verified_at, retract_reason, revision, created_at, updated_at";

fn read_lesson(row: &rusqlite::Row<'_>) -> rusqlite::Result<Lesson> {
    let scope_raw: String = row.get(2)?;
    let status_raw: String = row.get(5)?;
    let sources_raw: String = row.get(3)?;
    Ok(Lesson {
        id: row.get(0)?,
        statement: row.get(1)?,
        scope: LessonScope::parse(&scope_raw).unwrap_or(LessonScope::Problem),
        source_episodes: decode_sources(&sources_raw),
        evidence: row.get(4)?,
        status: LessonStatus::parse(&status_raw).unwrap_or(LessonStatus::Candidate),
        verified_by: row.get(6)?,
        verified_at: row.get(7)?,
        retract_reason: row.get(8)?,
        revision: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

impl SqliteMemoryStore {
    /// 从真实经历提起一条**候选**教训。
    ///
    /// 硬约束：`source_episodes` 非空，且每个 id 必须真实存在（防编造出处）。
    pub fn append_lesson_candidate(&self, new: NewLesson) -> Result<Lesson, LessonError> {
        if new.statement.trim().is_empty() {
            return Err(LessonError::Invalid("statement 不得为空".to_string()));
        }
        if new.evidence.trim().is_empty() {
            return Err(LessonError::Invalid("evidence 不得为空".to_string()));
        }
        if new.source_episodes.is_empty() {
            return Err(LessonError::MissingSources);
        }
        let conn = self.conn().map_err(storage)?;
        for episode in &new.source_episodes {
            let exists: Option<i64> = conn
                .query_row("SELECT 1 FROM episodes WHERE id = ?1", [episode], |row| {
                    row.get(0)
                })
                .optional()
                .map_err(storage)?;
            if exists.is_none() {
                return Err(LessonError::UnknownSource(episode.clone()));
            }
        }
        let id = format!("lsn-{}", uuid::Uuid::new_v4());
        let timestamp = now_ms();
        let sources = serde_json::to_string(&new.source_episodes)
            .map_err(|error| LessonError::Invalid(error.to_string()))?;
        conn.execute(
            "INSERT INTO lessons (id, statement, scope, source_episodes, evidence, status, \
             verified_by, verified_at, retract_reason, revision, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, 'candidate', NULL, NULL, NULL, 0, ?6, ?6)",
            params![
                id,
                new.statement.trim(),
                new.scope.as_str(),
                sources,
                new.evidence.trim(),
                timestamp
            ],
        )
        .map_err(storage)?;
        // 先放掉连接锁再读回：同一 Mutex 不可重入（否则自己锁死自己）。
        drop(conn);
        self.get_lesson(&id)?
            .ok_or_else(|| LessonError::NotFound(id.clone()))
    }

    /// 新场景验证：`verified_by` 必须是**另一次**经历/演练，不能是源经历。
    pub fn verify_lesson(
        &self,
        id: &str,
        verified_by: &str,
        expected_rev: i64,
    ) -> Result<Lesson, LessonError> {
        if verified_by.trim().is_empty() {
            return Err(LessonError::Invalid("verified_by 不得为空".to_string()));
        }
        let lesson = self
            .get_lesson(id)?
            .ok_or_else(|| LessonError::NotFound(id.to_string()))?;
        if lesson
            .source_episodes
            .iter()
            .any(|source| source == verified_by)
        {
            return Err(LessonError::SelfVerification {
                episode: verified_by.to_string(),
            });
        }
        if lesson.status == LessonStatus::Retracted {
            return Err(LessonError::InvalidTransition {
                from: "retracted",
                to: "verified",
            });
        }
        self.update_lesson_status(
            id,
            expected_rev,
            "verified",
            Some(verified_by),
            Some(now_ms()),
            None,
        )
    }

    /// 撤回：必须给理由；撤回后记录**永久保留**。
    pub fn retract_lesson(
        &self,
        id: &str,
        reason: &str,
        expected_rev: i64,
    ) -> Result<Lesson, LessonError> {
        if reason.trim().is_empty() {
            return Err(LessonError::MissingRetractReason);
        }
        self.update_lesson_status(
            id,
            expected_rev,
            "retracted",
            None,
            None,
            Some(reason.trim()),
        )
    }

    /// 读一条教训。
    pub fn get_lesson(&self, id: &str) -> Result<Option<Lesson>, LessonError> {
        let conn = self.conn().map_err(storage)?;
        conn.query_row(
            &format!("SELECT {LESSON_COLS} FROM lessons WHERE id = ?1"),
            [id],
            read_lesson,
        )
        .optional()
        .map_err(storage)
    }

    /// 列表（可按场景 / 状态过滤），按最近更新倒序。
    pub fn list_lessons(
        &self,
        scope: Option<LessonScope>,
        status: Option<LessonStatus>,
        limit: usize,
    ) -> Result<Vec<Lesson>, LessonError> {
        let conn = self.conn().map_err(storage)?;
        let mut statement = conn
            .prepare(&format!(
                "SELECT {LESSON_COLS} FROM lessons \
                 WHERE (?1 IS NULL OR scope = ?1) AND (?2 IS NULL OR status = ?2) \
                 ORDER BY updated_at DESC, id DESC LIMIT ?3"
            ))
            .map_err(storage)?;
        let rows = statement
            .query_map(
                params![
                    scope.map(|value| value.as_str()),
                    status.map(|value| value.as_str()),
                    limit as i64
                ],
                read_lesson,
            )
            .map_err(storage)?;
        rows.map(|row| row.map_err(storage)).collect()
    }

    /// 各状态教训计数（自述面用：verified / candidate / retracted 分列）。
    pub fn count_lessons_by_status(&self) -> Result<LessonCounts, LessonError> {
        let conn = self.conn().map_err(storage)?;
        let mut counts = LessonCounts::default();
        let mut statement = conn
            .prepare("SELECT status, COUNT(*) FROM lessons GROUP BY status")
            .map_err(storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(storage)?;
        for row in rows {
            let (status, count) = row.map_err(storage)?;
            let count = count.max(0) as u64;
            match LessonStatus::parse(&status)? {
                LessonStatus::Verified => counts.verified = count,
                LessonStatus::Candidate => counts.candidate = count,
                LessonStatus::Retracted => counts.retracted = count,
            }
        }
        Ok(counts)
    }

    /// 上下文注入面：**只取已验证**（候选不注入，撤回的不再注入）。
    pub fn recent_lessons_for_context(
        &self,
        scope: LessonScope,
        limit: usize,
    ) -> Result<Vec<Lesson>, LessonError> {
        self.list_lessons(Some(scope), Some(LessonStatus::Verified), limit)
    }

    fn update_lesson_status(
        &self,
        id: &str,
        expected_rev: i64,
        status: &str,
        verified_by: Option<&str>,
        verified_at: Option<i64>,
        retract_reason: Option<&str>,
    ) -> Result<Lesson, LessonError> {
        let mut conn = self.conn().map_err(storage)?;
        let transaction = conn.transaction().map_err(storage)?;
        let current: Option<(i64, String)> = transaction
            .query_row(
                "SELECT revision, status FROM lessons WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage)?;
        let Some((revision, current_status)) = current else {
            return Err(LessonError::NotFound(id.to_string()));
        };
        if revision != expected_rev {
            return Err(LessonError::RevisionConflict {
                expected: expected_rev,
                actual: revision,
            });
        }
        if current_status == status {
            return Err(LessonError::Invalid(format!(
                "已是 {status} 状态，重复操作拒绝"
            )));
        }
        transaction
            .execute(
                "UPDATE lessons SET status = ?1, verified_by = COALESCE(?2, verified_by), \
                 verified_at = COALESCE(?3, verified_at), \
                 retract_reason = COALESCE(?4, retract_reason), \
                 revision = revision + 1, updated_at = ?5 WHERE id = ?6",
                params![
                    status,
                    verified_by,
                    verified_at,
                    retract_reason,
                    now_ms(),
                    id
                ],
            )
            .map_err(storage)?;
        transaction.commit().map_err(storage)?;
        drop(conn);
        self.get_lesson(id)?
            .ok_or_else(|| LessonError::NotFound(id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().expect("in-memory store")
    }

    /// 造一条真实 episode 当出处（教训必须挂在真实经历上）。
    fn seed_episode(store: &SqliteMemoryStore, content: &str) -> String {
        let id = format!("ep-{}", uuid::Uuid::new_v4());
        let conn = store.conn().expect("conn");
        conn.execute(
            "INSERT INTO episodes (id, continuity_id, session_id, timestamp, role, content) \
             VALUES (?1, 'c1', 's1', 0, 'user', ?2)",
            params![id, content],
        )
        .expect("insert episode");
        id
    }

    fn candidate(source: &str) -> NewLesson {
        NewLesson {
            statement: "专注时不要提醒我喝水，结束后再提醒".to_string(),
            scope: LessonScope::Interaction,
            source_episodes: vec![source.to_string()],
            evidence: "用户原话：我专注时别提醒我喝水".to_string(),
        }
    }

    #[test]
    fn candidate_requires_a_real_source_episode() {
        let store = store();
        let error = store
            .append_lesson_candidate(candidate("ep-does-not-exist"))
            .expect_err("没出处必须拒绝");
        assert!(matches!(error, LessonError::UnknownSource(_)), "{error}");

        let no_sources = store
            .append_lesson_candidate(NewLesson {
                source_episodes: Vec::new(),
                ..candidate("x")
            })
            .expect_err("空出处必须拒绝");
        assert_eq!(no_sources, LessonError::MissingSources);
    }

    #[test]
    fn verified_requires_a_different_scenario_and_retract_requires_a_reason() {
        let store = store();
        let source = seed_episode(&store, "我专注时别提醒我喝水");
        let lesson = store
            .append_lesson_candidate(candidate(&source))
            .expect("append candidate");
        assert_eq!(lesson.status, LessonStatus::Candidate);
        assert_eq!(lesson.revision, 0);

        // 用源经历自证 = 拒绝。
        let error = store
            .verify_lesson(&lesson.id, &source, 0)
            .expect_err("不得用来源自证");
        assert!(
            matches!(error, LessonError::SelfVerification { .. }),
            "{error}"
        );

        // 新场景验证 = 通过。
        let witness = seed_episode(&store, "专注结束后我说了「现在可以提醒了」");
        let verified = store
            .verify_lesson(&lesson.id, &witness, 0)
            .expect("verify");
        assert_eq!(verified.status, LessonStatus::Verified);
        assert_eq!(verified.verified_by.as_deref(), Some(witness.as_str()));
        assert_eq!(verified.revision, 1);

        // 撤回必须给理由。
        let error = store
            .retract_lesson(&lesson.id, "   ", 1)
            .expect_err("空理由必须拒绝");
        assert_eq!(error, LessonError::MissingRetractReason);

        let retracted = store
            .retract_lesson(&lesson.id, "场景变了：现在不分专注/结束", 1)
            .expect("retract");
        assert_eq!(retracted.status, LessonStatus::Retracted);
        assert!(retracted.retract_reason.is_some());
        // 撤回记录永久保留，不删行。
        assert!(store.get_lesson(&lesson.id).expect("get").is_some());
    }

    #[test]
    fn revision_cas_rejects_stale_writes_and_double_transitions() {
        let store = store();
        let source = seed_episode(&store, "edge case 漏了");
        let lesson = store
            .append_lesson_candidate(candidate(&source))
            .expect("append");

        let witness = seed_episode(&store, "新场景复核");
        store
            .verify_lesson(&lesson.id, &witness, 0)
            .expect("verify");
        // 陈旧 rev 再写 = 冲突。
        let error = store
            .verify_lesson(&lesson.id, &witness, 0)
            .expect_err("陈旧修订号必须冲突");
        assert!(
            matches!(error, LessonError::RevisionConflict { .. }),
            "{error}"
        );
        // 同一状态重复迁移 = 拒（不是幂等成功，避免假装状态机在跑）。
        let error = store
            .verify_lesson(&lesson.id, &witness, 1)
            .expect_err("重复验证必须拒绝");
        assert!(matches!(error, LessonError::Invalid(_)), "{error}");
    }

    #[test]
    fn context_surface_only_exposes_verified_lessons() {
        let store = store();
        let source = seed_episode(&store, "第一次");
        let candidate_lesson = store
            .append_lesson_candidate(candidate(&source))
            .expect("append");
        // 候选不注入。
        assert!(store
            .recent_lessons_for_context(LessonScope::Interaction, 10)
            .expect("query")
            .is_empty());

        let witness = seed_episode(&store, "验证场景");
        store
            .verify_lesson(&candidate_lesson.id, &witness, 0)
            .expect("verify");
        let visible = store
            .recent_lessons_for_context(LessonScope::Interaction, 10)
            .expect("query");
        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id, candidate_lesson.id);

        // 撤回后不再注入，但记录仍在。
        store
            .retract_lesson(&candidate_lesson.id, "被新证据推翻", 1)
            .expect("retract");
        assert!(store
            .recent_lessons_for_context(LessonScope::Interaction, 10)
            .expect("query")
            .is_empty());
        assert!(store
            .get_lesson(&candidate_lesson.id)
            .expect("get")
            .is_some());
    }

    #[test]
    fn scope_vocabulary_is_closed() {
        assert!(LessonScope::parse("problem").is_ok());
        let error = LessonScope::parse("自由文本标签").expect_err("词表外必须拒绝");
        assert!(matches!(error, LessonError::UnknownScope { .. }));
    }
}
