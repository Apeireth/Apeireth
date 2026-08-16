//! `apeireth-companion::continuity` — continuity 锚点解析 + 跨锚点迁移接口.
//!
//! **职责** (backlog N2 / team-work-doc §8.4 OneRing 方向②④):
//! - `current_continuity_id()` — 进程级 continuity 锚点 (env `APEIRETH_CONTINUITY_ID`
//!   → 缺省 `companion-main`). 全链路 (账本/记忆会话/生命周期日志) 共用同一锚点,
//!   消灭散落各处的 "me" 硬编码.
//! - `ensure_identity()` — 为锚点在 IdentityCard 登记载体 (审计可查).
//! - `migrate_subject()` — 旧锚点 → 新锚点的记录迁移接口. **append-only 安全**:
//!   episodes 表 trigger 拒绝 UPDATE/DELETE (append_only.rs), 因此迁移 = 复制前进
//!   (新 id `mig-{原id}` + 保留原时间戳 + INSERT OR IGNORE 幂等), 原始行留痕不删.
//!
//! **哲学锚** (team-work-doc §1.1.4): 记录与连续性 = 最大努力的记录 + 迁移锚点,
//! **不假装灵魂同一** — 迁移只搬记录并留审计痕迹 (mig- 前缀 = lineage),
//! 新锚点是新载体, 不声称与旧锚点是"同一个灵魂".
//!
//! **0 假装**:
//! - IdentityCard.record_migration 通道语义是"载体迁移", 本模块不用它表达锚点改键
//!   (避免语义混用); 锚点改键的审计痕迹 = mig- 前缀 lineage + MigrationReport 日志.
//! - episodes.continuity_id 列在 put_episode 里是 "default" 占位 (memory crate LOCKED,
//!   非本任务包); 迁移副本直接写入真实 continuity_id (本模块自有 SQL 路径).

use std::sync::Arc;

use apeireth_core::{IdentityCard, Migration};
use apeireth_memory::{EpisodeQuery, EpisodeStore, IdentityCardStore, SqliteMemoryStore};
use rusqlite::params;

/// continuity 锚点环境变量名.
pub const CONTINUITY_ENV_VAR: &str = "APEIRETH_CONTINUITY_ID";

/// 缺省 continuity 锚点 (单主体部署的默认身份名).
pub const DEFAULT_CONTINUITY_ID: &str = "companion-main";

/// 迁移副本 episode 的 id 前缀 (lineage 审计: `mig-{原id}`).
pub const MIGRATED_ID_PREFIX: &str = "mig-";

/// 当前进程的 continuity 锚点: env `APEIRETH_CONTINUITY_ID` (非空) → 缺省值.
pub fn current_continuity_id() -> String {
    continuity_id_from_env(DEFAULT_CONTINUITY_ID)
}

/// env `APEIRETH_CONTINUITY_ID` (trim 后非空) → 自定义 `default`. (daemon 装配层入口)
pub fn continuity_id_from_env(default: &str) -> String {
    std::env::var(CONTINUITY_ENV_VAR)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// 归一化外部传入的 continuity 值 (如 HTTP header): trim 后非空 → 原值; 否则 → `fallback`.
/// 诚实兜底, 不让空锚点污染账本.
pub fn normalize_continuity(raw: &str, fallback: &str) -> String {
    let t = raw.trim();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t.to_string()
    }
}

/// 为锚点确保 IdentityCard 存在 (不存在则创建, 登记载体 `carrier`).
/// 已存在 → 直接返回, 幂等.
pub fn ensure_identity(
    store: &SqliteMemoryStore,
    continuity_id: &str,
    carrier: &str,
) -> Result<(), String> {
    let cid = continuity_id.trim();
    if cid.is_empty() {
        return Err("continuity_id 为空, 无法登记 IdentityCard".into());
    }
    if store.exists(cid).map_err(|e| e.to_string())? {
        return Ok(());
    }
    let card = IdentityCard {
        continuity_id: cid.to_string(),
        birth_time: chrono::Utc::now().timestamp(),
        carriers: vec![carrier.trim().to_string()],
        migration_history: Vec::new(),
    };
    match store.create(&card) {
        Ok(_) => Ok(()),
        // 并发场景下 exists→create 之间可能已被其他进程创建: 视为成功 (幂等语义).
        Err(apeireth_memory::MemoryError::Identity(
            apeireth_memory::IdentityConflict::AlreadyExists(_),
        )) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// 一次锚点迁移的诚实报告 (0 装 PASS: 迁移了多少, 如实说多少).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub from: String,
    pub to: String,
    /// episodes 复制前进成功的条数 (原始行保留, append-only).
    pub episodes_copied: usize,
    /// 已迁移过而跳过的条数 (幂等命中).
    pub episodes_skipped: usize,
    /// 账本表 (onering_messages) 改键行数 (0 = 表不存在或无旧锚点行).
    pub ledger_rekeyed: usize,
    pub executed_at: i64,
}

/// 迁移接口: 把旧锚点 `from` 的记录改键到新锚点 `to`.
///
/// - **episodes**: append-only 不可原地改 → 复制前进
///   (`mig-{原id}`, continuity_id=新锚点, session_id=新锚点, 保留原 timestamp/role/content),
///   INSERT OR IGNORE 幂等 (重复迁移不产生重复副本).
/// - **onering 账本表**: 本模块自有表 (非 append-only) → 直接 UPDATE 改键; 表不存在则 0.
/// - 校验: from/to 非空且不同, 否则 Err (0 装 PASS, 不静默返回空报告).
pub fn migrate_subject(
    store: &SqliteMemoryStore,
    from: &str,
    to: &str,
) -> Result<MigrationReport, String> {
    let from = from.trim().to_string();
    let to = to.trim().to_string();
    if from.is_empty() || to.is_empty() {
        return Err("迁移锚点不能为空 (from/to 均须非空)".into());
    }
    if from == to {
        return Err(format!("迁移锚点相同 (from == to == {from}), 无需迁移"));
    }

    let conn = store.conn().map_err(|e| e.to_string())?;

    // ① episodes 复制前进 (append-only 安全)
    let olds = store
        .query(&EpisodeQuery::new().for_session(&from))
        .map_err(|e| e.to_string())?;
    let mut copied = 0usize;
    let mut skipped = 0usize;
    for ep in &olds {
        let n = conn
            .execute(
                "INSERT OR IGNORE INTO episodes (id, continuity_id, session_id, timestamp, role, content)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    format!("{MIGRATED_ID_PREFIX}{}", ep.id),
                    to,
                    to,
                    ep.timestamp,
                    ep.role,
                    ep.content,
                ],
            )
            .map_err(|e| e.to_string())?;
        if n > 0 {
            copied += 1;
        } else {
            skipped += 1;
        }
    }

    // ② 账本表改键 (表不存在 → 0, 诚实)
    let ledger_rekeyed = if table_exists(&conn, "onering_messages") {
        conn.execute(
            "UPDATE onering_messages SET continuity_id = ?1 WHERE continuity_id = ?2",
            params![to, from],
        )
        .map_err(|e| e.to_string())?
    } else {
        0
    };

    Ok(MigrationReport {
        from,
        to,
        episodes_copied: copied,
        episodes_skipped: skipped,
        ledger_rekeyed,
        executed_at: chrono::Utc::now().timestamp(),
    })
}

/// 登记一次真实的载体迁移到 IdentityCard (record_migration 通道, 语义: 同一 continuity
/// 换载体). 与 `migrate_subject` (锚点改键) 是两件事, 分开提供避免语义混用.
pub fn record_carrier_migration(
    store: &SqliteMemoryStore,
    continuity_id: &str,
    from_carrier: &str,
    to_carrier: &str,
) -> Result<(), String> {
    let m = Migration {
        from_carrier: from_carrier.trim().to_string(),
        to_carrier: to_carrier.trim().to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };
    store
        .record_migration(continuity_id, &m)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn table_exists(conn: &rusqlite::Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![name],
        |r| r.get::<_, i64>(0),
    )
    .map(|n| n > 0)
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_memory::CoreEpisode;

    fn store_with_episodes(session: &str, contents: &[&str]) -> Arc<SqliteMemoryStore> {
        let store = Arc::new(SqliteMemoryStore::open_in_memory().unwrap());
        for (i, c) in contents.iter().enumerate() {
            store
                .put_episode(&CoreEpisode {
                    id: format!("ep-{i}"),
                    timestamp: 100 + i as i64,
                    role: if i % 2 == 0 { "user" } else { "assistant" }.into(),
                    content: c.to_string(),
                    session_id: session.into(),
                })
                .unwrap();
        }
        store
    }

    #[test]
    fn normalize_trims_and_falls_back() {
        assert_eq!(normalize_continuity("  c1  ", "fb"), "c1");
        assert_eq!(normalize_continuity("   ", "fb"), "fb");
        assert_eq!(normalize_continuity("", "fb"), "fb");
    }

    #[test]
    fn current_continuity_default_is_anchor() {
        // env 未设/被其他测试改动的最坏情况: 函数不得返回空串 (锚点非空是硬约束).
        let id = current_continuity_id();
        assert!(!id.trim().is_empty());
    }

    #[test]
    fn ensure_identity_is_idempotent() {
        let store = store_with_episodes("me", &[]);
        ensure_identity(&store, "c-main", "carrier-a").unwrap();
        ensure_identity(&store, "c-main", "carrier-a").unwrap(); // 二次 → Ok
        assert!(store.exists("c-main").unwrap());
    }

    #[test]
    fn ensure_identity_rejects_empty() {
        let store = store_with_episodes("me", &[]);
        assert!(ensure_identity(&store, "  ", "carrier-a").is_err());
    }

    #[test]
    fn migrate_copies_forward_and_keeps_originals() {
        let store = store_with_episodes("me", &["事实一", "事实二"]);
        let r = migrate_subject(&store, "me", "c-main").unwrap();
        assert_eq!(r.episodes_copied, 2);
        assert_eq!(r.episodes_skipped, 0);
        // 原始行仍在 (append-only, 不删)
        assert_eq!(store.count_by_session("me").unwrap(), 2);
        // 副本在新锚点, 内容/时间戳保真
        let news = store.recent_episodes("c-main", 10).unwrap();
        assert_eq!(news.len(), 2);
        assert!(news.iter().all(|e| e.id.starts_with(MIGRATED_ID_PREFIX)));
        assert_eq!(news[0].content, "事实一");
        assert_eq!(news[0].timestamp, 100);
    }

    #[test]
    fn migrate_is_idempotent() {
        let store = store_with_episodes("me", &["事实一"]);
        let r1 = migrate_subject(&store, "me", "c-main").unwrap();
        assert_eq!(r1.episodes_copied, 1);
        let r2 = migrate_subject(&store, "me", "c-main").unwrap();
        assert_eq!(r2.episodes_copied, 0);
        assert_eq!(r2.episodes_skipped, 1);
        // 新锚点仍只有一条 (无重复副本)
        assert_eq!(store.count_by_session("c-main").unwrap(), 1);
    }

    #[test]
    fn migrate_rejects_empty_or_same_anchor() {
        let store = store_with_episodes("me", &["x"]);
        assert!(migrate_subject(&store, "", "c").is_err());
        assert!(migrate_subject(&store, "me", "  ").is_err());
        assert!(migrate_subject(&store, "same", "same").is_err());
    }

    #[test]
    fn migrate_moved_episodes_carry_real_continuity_id() {
        let store = store_with_episodes("me", &["事实"]);
        migrate_subject(&store, "me", "c-main").unwrap();
        // EpisodeQuery.for_continuity 走 episodes.continuity_id 列 — 副本应可被主体检索命中
        let by_cont = store
            .query(&EpisodeQuery::new().for_continuity("c-main"))
            .unwrap();
        assert_eq!(by_cont.len(), 1, "迁移副本应写入真实 continuity_id");
    }
}
