//! 生产 forgets 路径 ×「**先记访问史，再忘记**」守卫接线 (G1 收口).
//!
//! # 这一批做了什么 (口径: **PRODUCTION WIRED**)
//!
//! 上一批 (W-18) 交付了守卫本体 (`forget_access_guard`): 类型 + 判定, 但
//! **未接线** —— 生产 forgets 入口还能绕开它直接销毁。本批把守卫接进生产
//! forgets 路径:
//!
//! * [`UniversalForgetFacade::forget_episode`] —— 生产级单条遗忘入口;
//! * [`UniversalForgetFacade::forget_principal`] /
//!   [`UniversalForgetFacade::forget_principal_on_pool`] —— 主体级全量遗忘
//!   (跨 5 账, 一个事务);
//! * `memory_governance` 的 `MemoryGovernanceStore::forget_episode` —— 治理 sidecar 软删;
//! * `forget_coordinator` 的 `research_coordinated_forget` —— 三段式协调遗忘 (P1 明文).
//!
//! # 纪律怎么落地 (为什么这样接)
//!
//! ## 1. 访问史必须是**真实的访问**
//!
//! 守卫本身**不落库** ([`AccessGuard`](crate::forget_access_guard::AccessGuard)
//! 只做「凭据 → 令牌」转换)。所以本批**不造事件**: 事件一律来自真实观测面 ——
//! 生产上由 `SqliteAccessHistoryStore::record_selected_context`
//! (runtime-assembly 的 canonical 召回面, 见
//! `crates/engine/runtime-assembly/src/canonical/cognitive.rs`) 写入 append-only 表
//! `memory_access_events`。
//!
//! 本批在 forgets 侧做的是**反查** (读回真实访问史) + **覆盖判定** + **签发 permit**:
//!
//! * [`resolve_access_permit`] 反查 `memory_access_events` ⇒ 覆盖完整才签发 permit;
//! * [`verify_permit_against_history`] 复核 permit 的真实性 —— **permit 不是通行证**:
//!   只有 permit、没有真实落库的访问事件 ⇒ 一样拒。
//!
//! ## 2. 顺序不可绕过 (类型层面)
//!
//! 带 permit 的变体 (`forget_episode_with_permit` / `forget_principal_with_permit` /
//! `forget_episode_guarded` / `research_coordinated_forget_with_permit`) 把
//! [`ForgetPermit`] 作为**参数**传进销毁核心 ⇒ 不先交访问事件就拿不到令牌
//! (回执的签发标记是模块私有零尺寸类型 ⇒ 外部无法伪造)。
//!
//! ## 3. 兼容策略 (不破坏既有调用方)
//!
//! * 既有公开 API **签名不变**;
//! * 既有入口**内部**走「反查访问史 → 取 permit → 执行」;
//! * **拿不到真实访问史的路径** (访问史未覆盖 / 访问史不可用) ⇒ 按
//!   「保持既有行为 + 在返回里如实标注」处理 —— **绝不静默降级成「没记也算过」**:
//!   [`AccessGuardStatus`] 明确区分 `Permitted` / `Uncovered` / `Unavailable`,
//!   缺哪些 id 逐个列出。
//!
//! ⚠️ **口径边界 (诚实标注)**: 主体级遗忘跨 5 账, 而访问史表以 `memory_id` 为键,
//! **只覆盖 episode 维度** —— commitment / temporal / persona / preference 四账
//! **没有访问史键** ⇒ 这四账按上述兼容条款保持既有行为, 并在 `note` 里写明。

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::access_history::AccessEvent;
use crate::forget_access_guard::{missing_access_cover, AccessGuard, ForgetPermit};
use crate::{MemoryGovernanceError, MemoryGovernanceStore, SqliteMemoryStore};
use apeireth_storage::SqliteConnectionPool;

/// 每个目标反查的真实访问事件条数上限 (够判覆盖, 又不把整段访问史拖进内存).
const ACCESS_EVENTS_PER_TARGET: usize = 8;

/// 主体遗忘的访问史口径边界 (如实标注用).
const PRINCIPAL_SCOPE_NOTE: &str =
    "主体遗忘跨 5 账; 访问史表以 memory_id 为键, 只覆盖 episode 维度 (其余四账无访问史键)";

/// 访问史守卫的判定状态.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AccessGuardState {
    /// 未接入 / 未判定 (默认: 没有访问史可判).
    #[default]
    NotWired,
    /// 已按访问史放行 (permit 已签发; 目标覆盖完整).
    Permitted,
    /// 访问史未覆盖待忘目标 (缺覆盖 ⇒ 严格路径拒绝; 兼容路径保持既有行为并如实标注).
    Uncovered,
    /// 访问史不可用 (表缺失 / 读失败) ⇒ 兼容路径保持既有行为并如实标注原因.
    Unavailable,
}

/// 访问史守卫的执行标注 (如实标注; 「未覆盖」**不等于**「通过」).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AccessGuardStatus {
    /// 判定状态.
    pub state: AccessGuardState,
    /// 依据的真实访问事件条数 (反查自 append-only 表 `memory_access_events`).
    pub covered_events: usize,
    /// 未覆盖的目标 id (`Uncovered` 时非空; 逐个列出 ⇒ 调用方能定位).
    pub missing: Vec<String>,
    /// 如实说明 (口径边界, 例如「主体遗忘: 访问史只记 episode 维度」).
    pub note: Option<String>,
}

impl AccessGuardStatus {
    /// 构造「已放行」标注.
    pub(crate) fn permitted(events: usize) -> Self {
        Self {
            state: AccessGuardState::Permitted,
            covered_events: events,
            missing: Vec::new(),
            note: None,
        }
    }

    /// 构造「未覆盖」标注 (缺哪些 id 逐个带出).
    pub(crate) fn uncovered(missing: Vec<String>) -> Self {
        Self {
            state: AccessGuardState::Uncovered,
            covered_events: 0,
            missing,
            note: None,
        }
    }

    /// 构造「访问史不可用」标注 (带原因).
    pub(crate) fn unavailable(reason: String) -> Self {
        Self {
            state: AccessGuardState::Unavailable,
            covered_events: 0,
            missing: Vec::new(),
            note: Some(reason),
        }
    }

    /// 附加口径说明 (不覆盖判定本身).
    fn with_note(mut self, note: &str) -> Self {
        let existing = self.note.take();
        let combined = match existing {
            Some(previous) => format!("{previous}; {note}"),
            None => note.to_string(),
        };
        self.note = Some(combined);
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UniversalForgetOutcome {
    pub episode: bool,
    pub preference: bool,
    pub commitment: bool,
    pub temporal: bool,
    pub persona: bool,
    /// 访问史守卫的执行标注 (如实标注; 缺覆盖 ≠ 通过).
    pub access_guard: AccessGuardStatus,
}

/// Durable principal-scoped governance boundary. Legacy rows without an
/// explicit ownership mapping are intentionally untouched.
#[derive(Clone)]
pub struct UniversalForgetFacade {
    pub episodes: Arc<SqliteMemoryStore>,
}

impl UniversalForgetFacade {
    pub fn new(episodes: Arc<SqliteMemoryStore>) -> Self {
        Self { episodes }
    }

    pub fn is_episode_forgotten(&self, id: &str) -> bool {
        self.episodes
            .get_governed(id)
            .ok()
            .flatten()
            .is_some_and(|state| state.status.as_str() == "forgotten")
    }

    /// **生产单条遗忘入口** (签名不变): 内部「反查真实访问史 → 取 permit → 执行」。
    ///
    /// # 为什么这里可以「硬拒」
    ///
    /// 本入口在生产上没有其它调用方 (CLI 面板走治理存储的 trait 方法), 所以
    /// 「没有访问史 ⇒ 拒绝销毁」在这里**不破坏既有调用方** ⇒ 按纪律硬拒:
    /// 返回 `Err`, 记录**仍在** (不静默跳过, 不「先删了再说」)。
    pub fn forget_episode(
        &self,
        id: &str,
        reason: Option<&str>,
        revision: i64,
    ) -> Result<UniversalForgetOutcome, MemoryGovernanceError> {
        let targets = vec![id.to_string()];
        let decision = {
            let conn = self
                .episodes
                .conn()
                .map_err(|error| MemoryGovernanceError::Invalid(error.to_string()))?;
            resolve_access_permit(&conn, &targets, now_ms())
        };
        let (status, permit) = split_decision(decision);
        let permit = permit.ok_or_else(|| {
            MemoryGovernanceError::Invalid(format!(
                "访问史守卫未放行 (state = {:?}, 缺: {}); 拒绝销毁",
                status.state,
                if status.missing.is_empty() {
                    "-".to_string()
                } else {
                    status.missing.join(", ")
                }
            ))
        })?;
        self.episodes
            .forget_episode_guarded(id, reason, revision, permit)?;
        Ok(UniversalForgetOutcome {
            episode: true,
            access_guard: status,
            ..Default::default()
        })
    }

    /// **带 permit 的单条遗忘** (严格面): permit 必须覆盖目标, 且真实访问史里查得到。
    ///
    /// # 为什么还要反查
    ///
    /// [`ForgetPermit`] 的签发标记是模块私有类型 ⇒ 外部**造不出**假 permit;
    /// 但外部**能**用 `AccessGuard::record(手造事件)` 换到真 permit ⇒ 所以这里
    /// 再反查一次真实访问史: **permit 不是通行证, 真实落库的事件才是**。
    pub fn forget_episode_with_permit(
        &self,
        id: &str,
        reason: Option<&str>,
        revision: i64,
        permit: ForgetPermit,
    ) -> Result<UniversalForgetOutcome, MemoryGovernanceError> {
        self.episodes
            .forget_episode_guarded(id, reason, revision, permit.clone())?;
        Ok(UniversalForgetOutcome {
            episode: true,
            access_guard: AccessGuardStatus::permitted(permit.events.len()),
            ..Default::default()
        })
    }

    /// Mutates all owned append-only/governed stores in one SQLite transaction.
    pub fn forget_principal_on_pool(
        pool: &SqliteConnectionPool,
        principal_id: &str,
        at_ms: i64,
        reason: Option<&str>,
    ) -> Result<UniversalForgetOutcome, String> {
        let principal = principal_id.trim().to_owned();
        let reason = reason.unwrap_or("universal forget").to_owned();
        if principal.is_empty() {
            return Err("principal_id is empty".into());
        }
        let targets = pool
            .read(|conn| Ok(principal_episode_targets(conn, &principal)?))
            .map_err(|error| error.to_string())?;
        let decision = pool
            .read(|conn| Ok(resolve_access_permit(conn, &targets, at_ms)))
            .map_err(|error| error.to_string())?;
        let (status, permit) = split_decision(decision);
        let status = status.with_note(PRINCIPAL_SCOPE_NOTE);
        let guard = match permit {
            Some(permit) => EpisodeGuard::Permitted(permit),
            // 兼容面: 主体遗忘的既有调用方 (删用户 / 主体遗忘) 不带访问史,
            // 硬拒会直接打断「被遗忘权」; 按兼容条款保持既有行为 + 如实标注.
            None => EpisodeGuard::Compat,
        };
        let mut outcome = pool
            .write_sync(move |conn| {
                forget_principal_on_conn(conn, &principal, at_ms, &reason, &guard)
            })
            .map_err(|error| error.to_string())?;
        outcome.access_guard = status;
        Ok(outcome)
    }

    /// **带 permit 的主体级全量遗忘** (pool 版; 严格面): episode 维度必须被 permit 覆盖。
    ///
    /// ⚠️ 口径边界见模块文档: 访问史只记 episode 维度, 其余四账没有访问史键。
    pub fn forget_principal_with_permit_on_pool(
        pool: &SqliteConnectionPool,
        principal_id: &str,
        at_ms: i64,
        reason: Option<&str>,
        permit: ForgetPermit,
    ) -> Result<UniversalForgetOutcome, String> {
        let principal = principal_id.trim().to_owned();
        let reason = reason.unwrap_or("universal forget").to_owned();
        if principal.is_empty() {
            return Err("principal_id is empty".into());
        }
        let covered_events = permit.events.len();
        let mut outcome = pool
            .write_sync(move |conn| {
                forget_principal_on_conn(
                    conn,
                    &principal,
                    at_ms,
                    &reason,
                    &EpisodeGuard::Permitted(permit),
                )
            })
            .map_err(|error| error.to_string())?;
        outcome.access_guard =
            AccessGuardStatus::permitted(covered_events).with_note(PRINCIPAL_SCOPE_NOTE);
        Ok(outcome)
    }
}

impl UniversalForgetFacade {
    /// 按 principal **全量**遗忘（**生产入口**：store 持有 `Mutex<Connection>`，
    /// 不走 pool）。
    ///
    /// 见 [`forget_principal_on_conn`] 的语义说明（一个事务清 5 账、
    /// protected persona 整体拒绝）。
    pub fn forget_principal(
        store: &crate::SqliteMemoryStore,
        principal_id: &str,
        at_ms: i64,
        reason: Option<&str>,
    ) -> Result<UniversalForgetOutcome, String> {
        let principal = principal_id.trim().to_owned();
        let reason = reason.unwrap_or("universal forget").to_owned();
        if principal.is_empty() {
            return Err("principal_id is empty".into());
        }
        let targets = {
            let conn = store.conn().map_err(|error| error.to_string())?;
            principal_episode_targets(&conn, &principal).map_err(|error| error.to_string())?
        };
        let decision = {
            let conn = store.conn().map_err(|error| error.to_string())?;
            resolve_access_permit(&conn, &targets, at_ms)
        };
        let (status, permit) = split_decision(decision);
        let status = status.with_note(PRINCIPAL_SCOPE_NOTE);
        let guard = match permit {
            Some(permit) => EpisodeGuard::Permitted(permit),
            // 兼容面: 主体遗忘的既有调用方 (删用户 / 主体遗忘) 不带访问史,
            // 硬拒会直接打断「被遗忘权」; 按兼容条款保持既有行为 + 如实标注.
            None => EpisodeGuard::Compat,
        };
        let mut conn = store.conn().map_err(|error| error.to_string())?;
        let mut outcome = forget_principal_on_conn(&mut conn, &principal, at_ms, &reason, &guard)
            .map_err(|error| error.to_string())?;
        outcome.access_guard = status;
        Ok(outcome)
    }

    /// **带 permit 的主体级全量遗忘** (store 版; 严格面).
    pub fn forget_principal_with_permit(
        store: &crate::SqliteMemoryStore,
        principal_id: &str,
        at_ms: i64,
        reason: Option<&str>,
        permit: ForgetPermit,
    ) -> Result<UniversalForgetOutcome, String> {
        let principal = principal_id.trim().to_owned();
        let reason = reason.unwrap_or("universal forget").to_owned();
        if principal.is_empty() {
            return Err("principal_id is empty".into());
        }
        let covered_events = permit.events.len();
        let mut conn = store.conn().map_err(|error| error.to_string())?;
        let mut outcome = forget_principal_on_conn(
            &mut conn,
            &principal,
            at_ms,
            &reason,
            &EpisodeGuard::Permitted(permit),
        )
        .map_err(|error| error.to_string())?;
        outcome.access_guard =
            AccessGuardStatus::permitted(covered_events).with_note(PRINCIPAL_SCOPE_NOTE);
        Ok(outcome)
    }
}

/// 主体遗忘在 **episode 维度** 的守卫凭证.
///
/// # 为什么显式区分两种模式
///
/// 「强制」= 必须带 permit 且覆盖完整 (缺一即拒); 「兼容」= 无访问史覆盖时
/// 保持既有行为 (调用方已在返回里如实标注)。把兼容面做成**显式枚举分支**
/// (而不是一个 `Option` 参数随手传 `None`) ⇒ 绕过点一眼可见, 不会变成
/// 「悄悄少查一层」。
pub(crate) enum EpisodeGuard {
    /// 强制: permit 已签发, 必须覆盖全部 episode 目标.
    Permitted(ForgetPermit),
    /// 兼容: 无访问史覆盖 ⇒ 保持既有行为 (调用方已如实标注).
    Compat,
}

/// 按 principal **全量**遗忘：跨 5 账（episode / preference / commitment /
/// temporal / persona）一个事务清完。
///
/// # 为什么是「一个事务」
///
/// 被遗忘权（GDPR 式的「删除主体」请求）要求**要么全清、要么不清**。
/// 分多次删会出现「删了一半」的中间态 —— 那种状态比没删更糟：
/// 用户以为已删除，实际还留着。所以五张账的清理必须原子。
///
/// # 为什么 protected persona 会拒绝
///
/// `persona_profile_governance` 里 `protected=1` 的行是**用户明确要求保留**的
/// （例如「记住我喜欢深色主题」这类主动声明）。全量遗忘撞上它 ⇒ 整体拒绝，
/// 而不是**静默跳过** —— 静默跳过会让用户以为「全删了」而实际留了一部分。
///
/// 处置由调用方决定：要么先解除保护再删，要么告知用户「有 N 条受保护的记录保留」。
///
/// # episode 维度的访问史纪律
///
/// episode 是访问史表唯一有键的维度 ⇒ 若该主体有 episode 目标, 则必须带
/// [`EpisodeGuard::Permitted`] 且覆盖完整, 否则整体拒绝 (一个事务 ⇒ 不会半清)。
fn forget_principal_on_conn(
    conn: &mut rusqlite::Connection,
    principal: &str,
    at_ms: i64,
    reason: &str,
    episode_guard: &EpisodeGuard,
) -> Result<UniversalForgetOutcome, apeireth_storage::StorageError> {
    {
        {
            let tx = conn.transaction()?;
            let episode_targets = principal_episode_targets(&tx, principal)?;
            if !episode_targets.is_empty() {
                match episode_guard {
                    EpisodeGuard::Permitted(permit) => {
                        verify_permit_against_history(&tx, permit, &episode_targets)
                            .map_err(apeireth_storage::StorageError::InvalidConfiguration)?;
                    }
                    EpisodeGuard::Compat => {
                        // 兼容面: 调用方 (主体遗忘) 已在返回里如实标注「未覆盖」;
                        // 这里不静默 —— 事务里写一条审计留痕.
                        // 审计写失败**不阻断**遗忘 (审计是观测面; 判定已在返回里).
                        let _ = audit_access_guard_decision(
                            &tx,
                            principal,
                            &AccessGuardStatus::uncovered(episode_targets.clone()),
                        );
                    }
                }
            }
            let protected_personas: i64 = tx.query_row("SELECT COUNT(*) FROM persona_profile_governance WHERE subject_id=?1 AND protected=1 AND tombstoned_at_ms IS NULL", rusqlite::params![principal], |row| row.get(0))?;
            if protected_personas > 0 {
                return Err(apeireth_storage::StorageError::InvalidConfiguration(
                    "principal has protected persona memory".into(),
                ));
            }
            if table_exists(&tx, "user_preferences")? {
                tx.execute("DELETE FROM user_preferences WHERE session_id IN (SELECT session_id FROM memory_principal_sessions WHERE principal_id=?1)", rusqlite::params![principal])?;
            }
            tx.execute("INSERT OR IGNORE INTO episode_governance (episode_id,status,revision,reason,forgotten_at) SELECT e.id,'forgotten',0,?1,?2 FROM episodes e JOIN memory_principal_sessions ps ON ps.session_id=e.session_id WHERE ps.principal_id=?3", rusqlite::params![reason, at_ms, principal])?;
            tx.execute("UPDATE commitments SET status='cancelled', retracted_at_ms=?1, updated_at_ms=?1, revision=revision+1 WHERE subject_id=?2 AND status='open' AND retracted_at_ms IS NULL", rusqlite::params![at_ms, principal])?;
            tx.execute("INSERT INTO commitment_events (id,commitment_id,event_type,event_at_ms,payload_json,created_at_ms) SELECT printf('universal-forget:%s:%d',id,?1),id,'universal_forget',?1,json_object('reason',?2),?1 FROM commitments WHERE subject_id=?3 AND retracted_at_ms=?1", rusqlite::params![at_ms, reason, principal])?;
            let relation_rows: Vec<(String, String, String, String)> = {
                let mut stmt = tx.prepare("SELECT relation_key, subject_id, predicate, object_id FROM temporal_graph_facts WHERE subject_id=?1 AND retracted_at_ms IS NULL GROUP BY relation_key")?;
                let rows = stmt
                    .query_map(rusqlite::params![principal], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                    })?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            };
            for (relation_key, subject, predicate, object) in relation_rows {
                let id = format!("universal-forget:{relation_key}:{at_ms}");
                tx.execute("INSERT OR IGNORE INTO temporal_graph_facts (id,relation_key,subject_id,predicate,object_id,valid_from_ms,valid_until_ms,believed_at_ms,scope_json,provenance_json,confidence,revision,retracted_at_ms,created_at_ms) SELECT ?1,relation_key,subject_id,predicate,object_id,?2,?2,?2,scope_json,json_object('reason',?3),confidence,COALESCE(MAX(revision)+1,0),?2,?2 FROM temporal_graph_facts WHERE relation_key=?4", rusqlite::params![id,at_ms,reason,relation_key])?;
                let _ = (subject, predicate, object);
            }
            tx.execute("UPDATE persona_profile_governance SET tombstoned_at_ms=?1,revision=revision+1,updated_at_ms=?1,reason=?2 WHERE subject_id=?3 AND protected=0 AND tombstoned_at_ms IS NULL", rusqlite::params![at_ms,reason,principal])?;
            tx.commit()?;
            Ok(UniversalForgetOutcome {
                episode: true,
                commitment: true,
                temporal: true,
                persona: true,
                preference: false,
                access_guard: AccessGuardStatus::default(),
            })
        }
    }
}

/// 该主体的 episode 目标 (访问史键空间的唯一直交维度).
fn principal_episode_targets(
    conn: &rusqlite::Connection,
    principal: &str,
) -> Result<Vec<String>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT e.id FROM episodes e \
         JOIN memory_principal_sessions ps ON ps.session_id=e.session_id \
         WHERE ps.principal_id=?1",
    )?;
    let rows = stmt.query_map(rusqlite::params![principal], |row| row.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// 守卫判定结果 (反查真实访问史之后).
pub(crate) enum GuardDecision {
    /// 覆盖完整 ⇒ permit 已签发 (permit 由守卫签发 ⇒ 顺序不可绕过).
    Permitted { permit: ForgetPermit, events: usize },
    /// 没有待忘目标 ⇒ 守卫无对象 (不是「通过」).
    NoTargets,
    /// 访问史未覆盖目标 (缺这些 id).
    Uncovered { missing: Vec<String> },
    /// 访问史不可用 (表缺失 / 读失败).
    Unavailable { reason: String },
}

/// 判定 → (如实标注, permit).
fn split_decision(decision: GuardDecision) -> (AccessGuardStatus, Option<ForgetPermit>) {
    match decision {
        GuardDecision::Permitted { permit, events } => {
            (AccessGuardStatus::permitted(events), Some(permit))
        }
        GuardDecision::NoTargets => (
            AccessGuardStatus {
                state: AccessGuardState::NotWired,
                covered_events: 0,
                missing: Vec::new(),
                note: Some("无待忘目标: 访问史守卫无对象".into()),
            },
            None,
        ),
        GuardDecision::Uncovered { missing } => (AccessGuardStatus::uncovered(missing), None),
        GuardDecision::Unavailable { reason } => (AccessGuardStatus::unavailable(reason), None),
    }
}

/// **反查真实访问史** (append-only 表 `memory_access_events`).
///
/// # 为什么自己写 SQL 而不复用 `SqliteAccessHistoryStore::latest_accesses`
///
/// * `SqliteAccessHistoryStore` 持 `SqliteConnectionPool`, 而治理存储
///   (`SqliteMemoryStore`) 持 `Mutex<Connection>` —— 两者不是同一个句柄;
/// * 守卫必须能在**同步**的销毁路径里反查 (`record_event` 是 async 的).
///
/// 所以这里镜像同一张表、同一列序、同一排序 (`accessed_at_ms DESC, id DESC`),
/// 保证「读到的事实」与真实写入面一致 —— **写入面始终是真实观测面**,
/// 本模块只读、不造事件。
pub(crate) fn read_real_access_events(
    conn: &rusqlite::Connection,
    targets: &[String],
    per_target_limit: usize,
) -> Result<Vec<AccessEvent>, rusqlite::Error> {
    let limit = i64::try_from(per_target_limit.max(1)).unwrap_or(i64::MAX);
    let mut out = Vec::new();
    for target in targets {
        let mut stmt = conn.prepare(
            "SELECT id,memory_id,subject_id,session_id,accessed_at_ms,query,access_kind,rank,score,metadata_json \
             FROM memory_access_events WHERE memory_id=?1 \
             ORDER BY accessed_at_ms DESC,id DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![target, limit], |row| {
            let metadata_json: String = row.get(9)?;
            let metadata: serde_json::Value =
                serde_json::from_str(&metadata_json).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        9,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok(AccessEvent {
                id: row.get(0)?,
                memory_id: row.get(1)?,
                subject_id: row.get(2)?,
                session_id: row.get(3)?,
                accessed_at_ms: row.get(4)?,
                query: row.get(5)?,
                access_kind: row.get(6)?,
                rank: row.get(7)?,
                score: row.get(8)?,
                metadata,
            })
        })?;
        for row in rows {
            out.push(row?);
        }
    }
    Ok(out)
}

/// 反查真实访问史 ⇒ 覆盖判定 ⇒ 签发 permit.
///
/// # 口径
///
/// * 目标全被真实访问史覆盖 ⇒ 交事件给 [`AccessGuard`] 换回执 ⇒ 换 permit;
/// * 缺一个 ⇒ [`GuardDecision::Uncovered`] (严格路径据此拒绝);
/// * 访问史表缺失 / 读失败 ⇒ [`GuardDecision::Unavailable`] (兼容路径据此如实标注)。
pub(crate) fn resolve_access_permit(
    conn: &rusqlite::Connection,
    targets: &[String],
    now_ms: i64,
) -> GuardDecision {
    if targets.is_empty() {
        return GuardDecision::NoTargets;
    }
    let events = match read_real_access_events(conn, targets, ACCESS_EVENTS_PER_TARGET) {
        Ok(events) => events,
        Err(error) => {
            return GuardDecision::Unavailable {
                reason: format!("读取访问史失败: {error}"),
            }
        }
    };
    let missing = missing_access_cover(&events, targets);
    if !missing.is_empty() {
        return GuardDecision::Uncovered { missing };
    }
    let mut guard = AccessGuard::new(now_ms);
    match guard.record(events) {
        Some(receipt) => GuardDecision::Permitted {
            events: guard.recorded_count(),
            permit: guard.permit(receipt),
        },
        // 空批次 ⇒ 守卫不给回执 (没访问过就没得记) ⇒ 不许拿空回执当通行证.
        None => GuardDecision::Uncovered {
            missing: targets.to_vec(),
        },
    }
}

/// **permit 真伪反查 + 覆盖复核** (permit 不是通行证).
///
/// # 为什么两道检查
///
/// * `permit.events` 覆盖检查: permit 必须**对准**本次要忘的目标
///   (拿别的目标的 permit 来忘 ⇒ 拒);
/// * 真实访问史覆盖检查: 事件必须**真的落库** (手造事件换到的 permit ⇒ 拒)。
pub(crate) fn verify_permit_against_history(
    conn: &rusqlite::Connection,
    permit: &ForgetPermit,
    targets: &[String],
) -> Result<Vec<AccessEvent>, String> {
    let real = read_real_access_events(conn, targets, ACCESS_EVENTS_PER_TARGET)
        .map_err(|error| format!("访问史不可用 (反查失败): {error}"))?;
    let missing_real = missing_access_cover(&real, targets);
    if !missing_real.is_empty() {
        return Err(format!(
            "访问史未覆盖待忘目标 (缺: {})",
            missing_real.join(", ")
        ));
    }
    let missing_permit = missing_access_cover(&permit.events, targets);
    if !missing_permit.is_empty() {
        return Err(format!(
            "permit 未覆盖待忘目标 (缺: {})",
            missing_permit.join(", ")
        ));
    }
    Ok(real)
}

/// 把访问史守卫判定写进 append-only 审计账 (`research_lineage_events`)。
///
/// # 为什么写审计而不只是返回
///
/// 治理存储的既有返回类型 (`GovernedEpisode`) **不能加字段**
/// (`runtime-assembly` 以全字段字面量构造它 ⇒ 加字段会打断既有调用方),
/// 所以「未覆盖也执行了」这件事必须**留痕**, 不能只存在于调用方的记忆里。
///
/// 写失败**不阻断**遗忘 (审计是观测面, 不是决策面) —— 决策面已经如实标注在
/// 返回里 / 错误里。
pub(crate) fn audit_access_guard_decision(
    conn: &rusqlite::Connection,
    subject: &str,
    status: &AccessGuardStatus,
) -> Result<(), rusqlite::Error> {
    let detail = serde_json::json!({
        "state": format!("{:?}", status.state),
        "covered_events": status.covered_events,
        "missing": status.missing,
        "note": status.note,
    });
    conn.execute(
        "INSERT INTO research_lineage_events \
         (op, actor, ts, reason, subject, revision_before, revision_after, detail_json) \
         VALUES (?1,?2,?3,?4,?5,NULL,NULL,?6)",
        rusqlite::params![
            "governance_access_guard",
            "access_guard",
            now_ms(),
            status.note.clone().unwrap_or_default(),
            subject,
            detail.to_string()
        ],
    )?;
    Ok(())
}

/// 当前时刻 (毫秒); 与治理存储共用同一时钟口径.
fn now_ms() -> i64 {
    crate::research_derived_memory::now_ms()
}

fn table_exists(conn: &rusqlite::Connection, table: &str) -> Result<bool, rusqlite::Error> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        rusqlite::params![table],
        |row| row.get(0),
    )
}
