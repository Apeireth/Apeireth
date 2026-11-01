//! 「**主体 → 会话**」归属映射的**写侧**（2026-11-01 安全审计 P1 收口）。
//!
//! # 为什么要有这个写侧
//!
//! [`crate::universal_forget`] 的「被遗忘权」在 **episode 维度**要成立，
//! 必须能回答「这个 episode 属于**哪个主体**」。
//! ⇒ 而归属靠 `memory_principal_sessions`（主体 → 会话 映射）**连接**：
//! `episodes.session_id` ⇄ `memory_principal_sessions.session_id` ⇒ `principal_id`。
//!
//! ⇒ **生产侧原先没有写入方** ⇒ ⇒ 生产环境的 episode **归不到主体**
//! ⇒ ⇒ 「被遗忘权」在 episode 维度**无法生效**（而旧返回体里这看不见 ——
//! `episode: true` 曾是常量）⇒ ⇒ 现在由 [`crate::universal_forget::AttributionStatus`]
//! 把这个缺口**显式带出来**。
//!
//! # 主体语义（**必须与调用方一致**）
//!
//! `principal_id` = **人 / 账户级**标识（域面 `DomainUserId`，即令牌绑定的 `user_id`）。
//! ⇒ **不是**客户端署名键（`ClientIdentity::client`：令牌名 / `anonymous` / `unattributed`）
//! ⇒ ⇒ `anonymous` / `unattributed` / 客户端级归属**一律不写**（否则会把不同人的
//! episode 归到同一个主体 ⇒ 误删风险）。
//!
//! # 写侧纪律
//!
//! * **幂等**：`INSERT OR IGNORE` ⇒ 同一 `(principal, session)` 重复登记不报错。
//! * **非空校验**：空 `principal_id` / `session_id` ⇒ **拒**（fail-closed：
//!   空主体会把所有人的 episode 归到空主体）。
//! * **只写映射**，不动 `episodes` / `user_preferences` 等任何其它账。

use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 登记一条「主体 → 会话」归属映射。
///
/// # 返回
///
/// * [`Ok`]`true` = **新登记**了一条；
/// * [`Ok`]`false` = 已存在（幂等命中，未改动）。
///
/// # 错误
///
/// * 空 `principal_id` / `session_id` ⇒ [`MemoryError::Invalid`]
///   （**不写** ⇒ 宁可「归属不可判定」，也不要错归属）。
/// * 存储层失败 ⇒ 原样上抛（真因如实）。
pub fn record_principal_session(
    store: &SqliteMemoryStore,
    principal_id: &str,
    session_id: &str,
    at_ms: i64,
) -> MemoryResult<bool> {
    let principal_id = principal_id.trim();
    let session_id = session_id.trim();
    if principal_id.is_empty() {
        return Err(MemoryError::Invalid(
            "principal_id 不能为空（空主体会把所有人的 episode 归到同一主体）".into(),
        ));
    }
    if session_id.is_empty() {
        return Err(MemoryError::Invalid(
            "session_id 不能为空（没有会话就没有归属连接点）".into(),
        ));
    }
    let conn = store.conn()?;
    let inserted = conn.execute(
        "INSERT OR IGNORE INTO memory_principal_sessions \
         (principal_id, session_id, created_at_ms) VALUES (?1, ?2, ?3)",
        rusqlite::params![principal_id, session_id, at_ms],
    )?;
    Ok(inserted > 0)
}

/// 查某主体已登记的会话数（自述面用；读侧口径与
/// 登记一条「主体 → 会话」归属映射（**连接池**版）。
///
/// 与 [`record_principal_session`] **同语义**（幂等 + 非空校验），
/// 只是走 [`apeireth_storage::SqliteConnectionPool`] —— 与
/// [`crate::universal_forget`] 的「读/判」侧**同一条连接路径** ⇒
/// 「写进去的」与「判得出来的」是同一本账。
///
/// # 返回
///
/// * [`Ok`]`true` = **新登记**了一条；[`Ok`]`false` = 幂等命中。
pub async fn record_principal_session_on_pool(
    pool: &apeireth_storage::SqliteConnectionPool,
    principal_id: &str,
    session_id: &str,
    at_ms: i64,
) -> MemoryResult<bool> {
    let principal_id = principal_id.trim();
    let session_id = session_id.trim();
    if principal_id.is_empty() {
        return Err(MemoryError::Invalid(
            "principal_id 不能为空（空主体会把所有人的 episode 归到同一主体）".into(),
        ));
    }
    if session_id.is_empty() {
        return Err(MemoryError::Invalid(
            "session_id 不能为空（没有会话就没有归属连接点）".into(),
        ));
    }
    // `pool.write` 的闭包要求 `'static` ⇒ 参数**移进**闭包（各克隆一份）。
    let principal = principal_id.to_string();
    let session = session_id.to_string();
    let stamp = at_ms;
    let inserted = pool
        .write(move |conn| {
            // `pool.write` 的闭包返回 `Result<_, StorageError>` ⇒ `execute` 的
            // rusqlite 错误用 `?` 自动转换, 再包成 `Ok`。
            Ok(conn.execute(
                "INSERT OR IGNORE INTO memory_principal_sessions \
                 (principal_id, session_id, created_at_ms) VALUES (?1, ?2, ?3)",
                rusqlite::params![principal, session, stamp],
            )?)
        })
        .await
        .map_err(|error| MemoryError::Other(format!("归属映射写入失败: {error}")))?;
    Ok(inserted > 0)
}
