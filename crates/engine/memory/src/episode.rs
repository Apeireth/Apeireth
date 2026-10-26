//! Episode 存储 (D2 §3 episode/episode query).
//!
//! - Append-only: 写入即不可变 (BEFORE UPDATE/DELETE trigger 拒绝)
//! - 索引: (session_id, timestamp), (continuity_id, timestamp), (timestamp)
//! - 查询: 按 session_id / time range / continuity_id / limit
//!
//! 公开 API:
//! - `EpisodeStore` trait
//! - `EpisodeQuery` 复合条件

use rusqlite::{params, Connection, OptionalExtension};

use apeireth_core::kernel::memory::Episode;

use crate::memory_chain::{put_chained_episode, ChainEntry};
use crate::provenance::EpisodeMeta;
use crate::{MemoryError, MemoryResult};

/// v1 存储 `continuity_id` 的**既有占位值**（历史口径, 本批不改）。
///
/// **为什么提成常量而不是散落的字面量**: `continuity_id` 既是「主体引用」
/// 也是**哈希链的链键**（链按主体分链）⇒ 占位值一旦在多处各写一份, 迟早分叉
/// （一处 "default"、一处 "Default" ⇒ 链被切成两条）。单一真源 = 链键口径。
pub(crate) const DEFAULT_CONTINUITY_ID: &str = "default";

/// Episode 复合查询条件.
#[derive(Debug, Clone, Default)]
pub struct EpisodeQuery {
    /// 按 session_id 过滤.
    pub session_id: Option<String>,
    /// 按主体 continuity_id 过滤.
    pub continuity_id: Option<String>,
    /// 起始时间戳 (含, epoch seconds).
    pub since: Option<i64>,
    /// 结束时间戳 (含).
    pub until: Option<i64>,
    /// 角色 (user/assistant/system) 过滤.
    pub role: Option<String>,
    /// 最大返回条数.
    pub limit: Option<usize>,
}

impl EpisodeQuery {
    /// 构造空查询.
    pub fn new() -> Self {
        Self::default()
    }

    /// 链式: 限定 session_id.
    pub fn for_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// 链式: 限定 continuity_id.
    pub fn for_continuity(mut self, cid: impl Into<String>) -> Self {
        self.continuity_id = Some(cid.into());
        self
    }

    /// 链式: 时间窗 `[since, until]`.
    pub fn in_range(mut self, since: Option<i64>, until: Option<i64>) -> Self {
        self.since = since;
        self.until = until;
        self
    }

    /// 链式: 限定角色.
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// 链式: 限制最大返回条数.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = Some(n);
        self
    }
}

/// Episode 存储 trait.
pub trait EpisodeStore {
    /// 写入一条 Episode (append-only).
    fn put_episode(&self, ep: &Episode) -> MemoryResult<()>;
    /// 按 id 读取.
    fn get_episode(&self, id: &str) -> MemoryResult<Option<Episode>>;
    /// 检索某 session 的最近 N 条 (按时间升序, 取尾部 N 条).
    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>>;
    /// 复合条件查询.
    fn query(&self, q: &EpisodeQuery) -> MemoryResult<Vec<Episode>>;
    /// 统计某 session 的 Episode 数量.
    fn count_by_session(&self, session_id: &str) -> MemoryResult<i64>;
    /// 列出某 subject 的所有 Episode (跨 session).
    fn list_by_subject(&self, continuity_id: &str) -> MemoryResult<Vec<Episode>>;
}

impl crate::SqliteMemoryStore {
    fn validate_episode(ep: &Episode) -> MemoryResult<()> {
        if ep.id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode id is empty".into()));
        }
        if ep.session_id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode session_id is empty".into()));
        }
        if ep.role.trim().is_empty() {
            return Err(MemoryError::Invalid("episode role is empty".into()));
        }
        Ok(())
    }

    fn row_to_episode(row: &rusqlite::Row<'_>) -> rusqlite::Result<Episode> {
        Ok(Episode {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            session_id: row.get(4)?,
        })
    }

    /// Write an episode bound to a real `continuity_id` (not the `"default"` placeholder).
    ///
    /// Companion `continuity::migrate_subject` needed this path because
    /// `EpisodeStore::put_episode` hard-codes `continuity_id = "default"`. The trait
    /// signature is LOCKED; this inherent method is the compatible extension.
    /// Empty continuity_id is rejected (does not silently fall back to `"default"`).
    ///
    /// **D0 哈希链 (W-10)**: 本路径同样**入链** —— 链键 = 本方法绑定的真实
    /// `continuity_id`（不是占位值）⇒ 真实主体的链与占位主体的链互不干扰。
    ///
    /// 本路径只写 6 列（不带 V4 元数据）⇒ 摘要里那 4 个元数据字段按 `NULL`
    /// 口径参与（写入侧与校验侧同一口径 ⇒ 重算得到同一摘要）。
    ///
    /// **口径**: 库级能力 = **IMPLEMENTED**；生产接线 = **DEFAULT ENABLED**。
    pub fn put_episode_for_subject(&self, ep: &Episode, continuity_id: &str) -> MemoryResult<()> {
        Self::validate_episode(ep)?;
        let cid = continuity_id.trim();
        if cid.is_empty() {
            return Err(MemoryError::Invalid(
                "continuity_id is empty (put_episode_for_subject refuses the default placeholder)"
                    .into(),
            ));
        }
        let conn = self.conn()?;
        // 硬失败语义: 链算不出来 / 落库失败 ⇒ 返回 Err, 绝不「无链写入后静默跳过」
        // (无链写入 = 体检报告里的「未纳管」, 而调用方会以为数据受保护).
        let entry = ChainEntry::from_episode(ep, None);
        put_chained_episode(&conn, cid, &entry).map(|_| ())
    }
}

/// 按 `role` **保守推导**记忆来源。
///
/// # 为什么这样够
///
/// 生产写入路径以前只写 6 列，V4 的 `provenance` 列留 NULL ⇒ 读侧一律兜底成
/// `Manual`（"手动注入"）—— 也就是说**一条工具产物和一条用户手写在库里长得
/// 一模一样**。可追溯性没有数据支撑，「记录 ≠ 负责」那一条就断了。
///
/// # 为什么这是「近似」而不是「精确追踪」
///
/// 真正的来源要由**调用点**告诉存储层（谁写的这条记忆）。当前调用点没有这个
/// 通道（trait 签名锁定），所以只能从 `role` 推导 —— 它能区分
/// 「对话来的」/「工具来的」/「系统注入的」，但分不清同 `role` 内部的不同来源。
/// 所以这里**只做不撒谎的最小区分**：认不出来的一律 `Manual`（不假装）。
///
/// `role` 的取值口径见 [`EpisodeQuery::role`]：`user` / `assistant` / `system` / `tool`。
///
/// **为什么是 `pub(crate)` 而不是私有**: 这份「role → 来源」映射是**全仓唯一的
/// 口径**（写入路径用它推导 `provenance` 列, 链摘要也用它 ⇒ 摘要与落库值同源）。
/// 保持私有会让下一个需要它的模块**抄一份** ⇒ 两份口径漂移（本仓反复吃过的亏）。
/// 公开到 crate 内即可: crate 外没有理由绕过存储层自己猜来源。
pub(crate) fn provenance_for_role(role: &str) -> crate::Provenance {
    use crate::Provenance;
    match role.trim().to_ascii_lowercase().as_str() {
        "tool" | "tool_result" | "tool_use" => Provenance::Tool,
        "user" | "assistant" => Provenance::Dialog,
        "system" => Provenance::Observation,
        _ => Provenance::Manual,
    }
}

impl EpisodeStore for crate::SqliteMemoryStore {
    fn put_episode(&self, ep: &Episode) -> MemoryResult<()> {
        Self::validate_episode(ep)?;
        let conn = self.conn()?;
        // Note: 现有 schema 没有 continuity_id 列在 episodes 表上, 续作兼容.
        // 主体引用通过 session_id 间接表达; 真正跨 session 的主体关联走 6 历史流.
        //
        // **V4 的 4 列一并写入**（`valid_from_ms` / `valid_until_ms` / `created_ms` /
        // `provenance`）。此前本路径只写 6 列，那 4 列留 NULL ⇒ 读侧
        // `normalize_meta` 一律兜底成 `Provenance::Manual` ⇒
        // **「这条记忆从哪来」在生产上永远是假的**（工具产物与用户手写无法区分）。
        // 那正是「记录 ≠ 负责」里最关键的一环断了 —— 可追溯性没有数据支撑。
        //
        // 来源按 `role` **保守推导**（不是精确追踪，如实标注）：
        //   tool → Tool · user/assistant → Dialog · system → Observation
        //   其它 → Manual（不假装）。
        //
        // **D0 哈希链 (W-10)**: 本路径是 v1 存储的**默认写入路径** ⇒ 产出的记录
        // 必须**带链字段**（`chain_hash` / `chain_prev_hash`）。此前只写 10 列 ⇒
        // 体检（`memory verify`）把这些记录算成「未纳管」⇒ 链覆盖率取决于调用方。
        //
        // **为什么只能在 INSERT 时写链列**: `episodes` 有 append-only 触发器
        // （`BEFORE UPDATE` / `BEFORE DELETE` 直接 ABORT）⇒ 事后 UPDATE 补链字段
        // **不可能** ⇒ 链摘要必须在插入前算好、和内容一起一次 INSERT。
        //
        // **链键 = `continuity_id`**（链按主体分链）⇒ 本路径沿用既有占位口径
        // [`DEFAULT_CONTINUITY_ID`]（真实主体走 `put_episode_for_subject`）。
        // 本批**不改口径**：占位值就是占位值，如实沿用。
        //
        // **失败语义**: 链算不出来 / 落库失败 ⇒ **硬失败**（宁可拒写，不可无链写入
        // —— 无链写入 = 缺口原地不动，而调用方会以为数据受保护）。
        //
        // **口径**: 库级能力 = **IMPLEMENTED**（链摘要随内容一次 INSERT 落库, 有测试
        // 覆盖）；生产接线 = **DEFAULT ENABLED**（默认写入路径自动入链, 无 feature
        // 开关, 调用方零改动 ⇒ 链覆盖率不再取决于调用方）。
        let meta = EpisodeMeta {
            valid_from_ms: Some(ep.timestamp.saturating_mul(1000)),
            valid_until_ms: None,                          // 永久有效
            created_ms: ep.timestamp.saturating_mul(1000), // 秒 → 毫秒
            provenance: provenance_for_role(&ep.role),
        };
        let entry = ChainEntry::from_episode(ep, Some(&meta));
        put_chained_episode(&conn, DEFAULT_CONTINUITY_ID, &entry).map(|_| ())
    }

    fn get_episode(&self, id: &str) -> MemoryResult<Option<Episode>> {
        if id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode id is empty".into()));
        }
        let conn = self.conn()?;
        let row = conn
            .query_row(
                "SELECT id, timestamp, role, content, session_id
                 FROM episodes WHERE id = ?1",
                params![id],
                Self::row_to_episode,
            )
            .optional()?;
        Ok(row)
    }

    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>> {
        if n == 0 {
            return Ok(Vec::new());
        }
        let conn = self.conn()?;
        // 取时间最大的 N 条, 然后按时间升序返回 (符合"对话流"语义).
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, role, content, session_id
             FROM episodes
             WHERE session_id = ?1
             ORDER BY timestamp DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![session_id, n as i64], Self::row_to_episode)?;
        let mut out: Vec<Episode> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        out.reverse();
        Ok(out)
    }

    fn query(&self, q: &EpisodeQuery) -> MemoryResult<Vec<Episode>> {
        let conn = self.conn()?;
        let mut sql =
            String::from("SELECT id, timestamp, role, content, session_id FROM episodes WHERE 1=1");
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = &q.session_id {
            sql.push_str(" AND session_id = ?");
            args.push(Box::new(s.clone()));
        }
        if let Some(c) = &q.continuity_id {
            sql.push_str(" AND continuity_id = ?");
            args.push(Box::new(c.clone()));
        }
        if let Some(since) = q.since {
            sql.push_str(" AND timestamp >= ?");
            args.push(Box::new(since));
        }
        if let Some(until) = q.until {
            sql.push_str(" AND timestamp <= ?");
            args.push(Box::new(until));
        }
        if let Some(role) = &q.role {
            sql.push_str(" AND role = ?");
            args.push(Box::new(role.clone()));
        }
        sql.push_str(" ORDER BY timestamp ASC, id ASC");
        if let Some(n) = q.limit {
            sql.push_str(&format!(" LIMIT {}", n));
        }
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_episode)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    fn count_by_session(&self, session_id: &str) -> MemoryResult<i64> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM episodes WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    fn list_by_subject(&self, continuity_id: &str) -> MemoryResult<Vec<Episode>> {
        if continuity_id.trim().is_empty() {
            return Err(MemoryError::Invalid("continuity_id is empty".into()));
        }
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, role, content, session_id
             FROM episodes WHERE continuity_id = ?1
             ORDER BY timestamp ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![continuity_id], Self::row_to_episode)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteMemoryStore;

    fn make_episode(id: &str, session: &str, ts: i64, role: &str) -> Episode {
        Episode {
            id: id.into(),
            timestamp: ts,
            role: role.into(),
            content: format!("content of {id}"),
            session_id: session.into(),
        }
    }

    /// 生产写入路径**必须**带上 V4 的 4 列 —— 否则来源永远是 `Manual`，
    /// 「这条记忆从哪来」在生产上是假的（工具产物与用户手写无法区分）。
    #[test]
    fn put_episode_records_v4_metadata() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let ep = make_episode("ep-meta", "s-1", 1000, "tool");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();

        let meta = store
            .read_episode_meta("ep-meta")
            .unwrap()
            .expect("应有元数据");
        assert_eq!(
            meta.provenance,
            crate::Provenance::Tool,
            "role=tool ⇒ 来源必须是 Tool（不能是 Manual）"
        );
        assert_eq!(meta.created_ms, 1_000_000, "created_ms = 秒×1000");
        assert_eq!(meta.valid_from_ms, Some(1_000_000));
        assert_eq!(meta.valid_until_ms, None, "默认永久有效");
    }

    /// 来源按 role 保守推导；**认不出来的一律 Manual**（不假装）。
    #[test]
    fn provenance_is_derived_from_role_conservatively() {
        use crate::Provenance;
        assert_eq!(provenance_for_role("tool"), Provenance::Tool);
        assert_eq!(provenance_for_role("TOOL"), Provenance::Tool);
        assert_eq!(provenance_for_role("user"), Provenance::Dialog);
        assert_eq!(provenance_for_role("assistant"), Provenance::Dialog);
        assert_eq!(provenance_for_role("system"), Provenance::Observation);
        assert_eq!(provenance_for_role("mystery"), Provenance::Manual);
        assert_eq!(provenance_for_role(""), Provenance::Manual);
    }

    #[test]
    fn put_and_get_roundtrip() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let ep = make_episode("ep-1", "s-1", 1000, "user");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();
        let got = <SqliteMemoryStore as EpisodeStore>::get_episode(&store, "ep-1")
            .unwrap()
            .unwrap();
        assert_eq!(got.id, "ep-1");
        assert_eq!(got.content, "content of ep-1");
    }

    #[test]
    fn append_only_blocks_update_and_delete() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let ep = make_episode("ep-a", "s-a", 1, "user");
        <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &ep).unwrap();
        let conn = store.conn().unwrap();
        let upd = conn
            .execute("UPDATE episodes SET content='x' WHERE id='ep-a'", [])
            .unwrap_err();
        assert!(upd.to_string().contains("append-only"), "got {upd}");
        let del = conn
            .execute("DELETE FROM episodes WHERE id='ep-a'", [])
            .unwrap_err();
        assert!(del.to_string().contains("append-only"), "got {del}");
    }

    #[test]
    fn recent_episodes_returns_in_chronological_order() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        // 故意乱序插入
        for (id, ts) in [("e3", 3000_i64), ("e1", 1000), ("e2", 2000)] {
            <SqliteMemoryStore as EpisodeStore>::put_episode(
                &store,
                &make_episode(id, "sess-A", ts, "user"),
            )
            .unwrap();
        }
        let recent =
            <SqliteMemoryStore as EpisodeStore>::recent_episodes(&store, "sess-A", 3).unwrap();
        assert_eq!(
            recent.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            vec!["e1", "e2", "e3"]
        );
    }

    #[test]
    fn query_combines_filters() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "sess-X", 100, "user"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e2", "sess-X", 200, "assistant"),
        )
        .unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e3", "sess-Y", 300, "user"),
        )
        .unwrap();
        let q = EpisodeQuery::new()
            .for_session("sess-X")
            .in_range(Some(150), Some(250));
        let rows = <SqliteMemoryStore as EpisodeStore>::query(&store, &q).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "e2");
    }

    #[test]
    fn count_by_session_reflects_inserts() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        for i in 0..5 {
            <SqliteMemoryStore as EpisodeStore>::put_episode(
                &store,
                &make_episode(&format!("e{i}"), "sess-C", 100 + i, "user"),
            )
            .unwrap();
        }
        let c = <SqliteMemoryStore as EpisodeStore>::count_by_session(&store, "sess-C").unwrap();
        assert_eq!(c, 5);
    }

    #[test]
    fn list_by_subject_returns_all_timestamps() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        <SqliteMemoryStore as EpisodeStore>::put_episode(
            &store,
            &make_episode("e1", "sess-1", 1, "user"),
        )
        .unwrap();
        let rows = <SqliteMemoryStore as EpisodeStore>::list_by_subject(&store, "default").unwrap();
        assert!(rows.len() >= 1);
    }

    #[test]
    fn empty_validation_errors() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let mut bad = make_episode("e1", "sess-1", 1, "user");
        bad.id = "  ".into();
        let err = <SqliteMemoryStore as EpisodeStore>::put_episode(&store, &bad).unwrap_err();
        assert!(err.to_string().contains("id is empty"));
    }

    #[test]
    fn put_episode_for_subject_writes_real_continuity_id() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        store
            .put_episode_for_subject(&make_episode("e-sub", "sess-A", 1, "user"), "cid-real")
            .unwrap();
        let by_cid = <SqliteMemoryStore as EpisodeStore>::query(
            &store,
            &EpisodeQuery::new().for_continuity("cid-real"),
        )
        .unwrap();
        assert_eq!(by_cid.len(), 1);
        assert_eq!(by_cid[0].id, "e-sub");
        let by_default =
            <SqliteMemoryStore as EpisodeStore>::list_by_subject(&store, "default").unwrap();
        assert!(by_default.iter().all(|e| e.id != "e-sub"));
    }

    #[test]
    fn put_episode_for_subject_rejects_empty_continuity() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let err = store
            .put_episode_for_subject(&make_episode("e1", "s", 1, "user"), "  ")
            .unwrap_err();
        assert!(err.to_string().contains("continuity_id is empty"));
    }
}
