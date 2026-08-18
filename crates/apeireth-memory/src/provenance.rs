//! TP24 (M5 + N25): 记忆来源链 + 时间元数据.
//!
//! 设计要点:
//! - `Provenance` 五来源枚举 (默认 Manual, 对账兼容)
//! - `EpisodeMeta` 4 字段结构: created_ms (必填), valid_from_ms/valid_until_ms (Option), provenance (默认 Manual)
//! - 在 `SqliteMemoryStore` 上加 inherent 方法 (不引入 trait, 不破坏 `EpisodeStore` API):
//!   - `put_episode_full(ep, meta)`: 一次 INSERT 写 10 列 (含 V4 加的 4 列元数据)
//!   - `query_with_time_range(from_ms, until_ms)`: 按 ms 时间窗过滤, 跨 session 扫
//!   - `read_episode_meta(id)`: 读 4 列元数据 (兼容默认由 [`normalize_meta`] 提供)
//! - 老条目 (V4 迁移前) 4 列均为 NULL → 读取时按任务纪律自动填默认
//!
//! 向后兼容铁律:
//! - `EpisodeStore::put_episode(ep)` 签名不变; 旧调用方 4 列写 NULL (默认 Manual / 永久有效)
//! - 旧 row 通过 `query_with_time_range` 也能正常返回 (created_ms 兜底为 timestamp * 1000)

use rusqlite::{params, Connection, OptionalExtension};

use apeireth_core::Episode;

use crate::{MemoryError, MemoryResult};

/// 记忆来源 (TP24/M5+N25).
///
/// 默认 `Manual`: 兼容既有记忆条目 (无 provenance 时默认人工注入, 不假装是 LLM 提炼).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// 对话中提取 (memory_extractor 提炼或 LLM 直接吐).
    Dialog,
    /// 工具执行产物 (tool result 写入).
    Tool,
    /// 反思周期 (做梦期 / reflection 批).
    Reflection,
    /// Observer 钩子 (life-force / heartbeat 写).
    Observation,
    /// 手动注入 (用户主动写 / 老条目默认).
    Manual,
}

impl Default for Provenance {
    fn default() -> Self {
        Provenance::Manual
    }
}

impl Provenance {
    /// DB 序列化 (snake_case 字符串).
    pub fn as_str(&self) -> &'static str {
        match self {
            Provenance::Dialog => "dialog",
            Provenance::Tool => "tool",
            Provenance::Reflection => "reflection",
            Provenance::Observation => "observation",
            Provenance::Manual => "manual",
        }
    }

    /// DB 反序列化 (未知值按纪律降级 Manual).
    pub fn from_db(s: &str) -> Self {
        match s {
            "dialog" => Provenance::Dialog,
            "tool" => Provenance::Tool,
            "reflection" => Provenance::Reflection,
            "observation" => Provenance::Observation,
            _ => Provenance::Manual,
        }
    }
}

/// 时间元数据 + 来源 (V4 迁移新增 4 列).
///
/// 写入时 `created_ms` 必须设 (i64), `valid_from_ms`/`valid_until_ms` 可空 (永久有效);
/// 读取时老条目 (V4 前 NULL) 按 [`normalize_meta`] 填默认.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeMeta {
    /// 生效起点 (epoch ms). None = 永久生效 (永久有效记忆).
    pub valid_from_ms: Option<i64>,
    /// 失效时间 (epoch ms). None = 永久有效 (per task 验收 #4 兼容默认).
    pub valid_until_ms: Option<i64>,
    /// 创建时间 (epoch ms, 必填).
    pub created_ms: i64,
    /// 来源 (默认 Manual).
    pub provenance: Provenance,
}

impl EpisodeMeta {
    /// 构造 (基于给定 now_ms + 来源). valid_from 默认为 created_ms, valid_until 永久.
    pub fn now(created_ms: i64, provenance: Provenance) -> Self {
        Self {
            valid_from_ms: Some(created_ms),
            valid_until_ms: None,
            created_ms,
            provenance,
        }
    }
}

/// 老条目兼容: 把 episode + 4 列原始 Option 转 MemoryEntry 所需的元数据.
///
/// 默认规则 (per task §4 兼容):
/// - provenance: Manual (None → Manual, 不假装)
/// - valid_from_ms: created_ms (None → 以 created_ms 兜; 无 created_ms 则以 timestamp * 1000)
/// - valid_until_ms: None (永久, 保留 None 语义)
/// - created_ms: Option → i64 (无则用 timestamp * 1000 兜底)
pub fn normalize_meta(
    valid_from_ms: Option<i64>,
    valid_until_ms: Option<i64>,
    created_ms: Option<i64>,
    provenance: Option<Provenance>,
    fallback_timestamp_sec: i64,
) -> (Option<i64>, Option<i64>, i64, Provenance) {
    let provenance = provenance.unwrap_or_default();
    let created_ms = created_ms.unwrap_or_else(|| fallback_timestamp_sec.saturating_mul(1000));
    let valid_from_ms = valid_from_ms.or(Some(created_ms));
    let valid_until_ms = valid_until_ms;
    (valid_from_ms, valid_until_ms, created_ms, provenance)
}

/// Inherent 方法集 (TP24): 给 `SqliteMemoryStore` 加 V4 后的能力, 不破坏 `EpisodeStore` API.
impl crate::SqliteMemoryStore {
    /// 写入 Episode + 元数据 (V4 10 列, 一次 INSERT)。
    ///
    /// 与 `EpisodeStore::put_episode(ep)` 不同: 一次性写完 4 个新加列。
    /// append-only trigger 对 INSERT 不拒绝 (仅 BEFORE UPDATE/DELETE), 所以一次 INSERT 安全。
    pub fn put_episode_full(&self, ep: &Episode, meta: &EpisodeMeta) -> MemoryResult<()> {
        // 复用 EpisodeStore 的字段校验 (id/session_id/role 非空)
        if ep.id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode id is empty".into()));
        }
        if ep.session_id.trim().is_empty() {
            return Err(MemoryError::Invalid("episode session_id is empty".into()));
        }
        if ep.role.trim().is_empty() {
            return Err(MemoryError::Invalid("episode role is empty".into()));
        }
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO episodes \
             (id, continuity_id, session_id, timestamp, role, content, \
              valid_from_ms, valid_until_ms, created_ms, provenance) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                ep.id,
                "default",
                ep.session_id,
                ep.timestamp,
                ep.role,
                ep.content,
                meta.valid_from_ms,
                meta.valid_until_ms,
                meta.created_ms,
                meta.provenance.as_str(),
            ],
        )?;
        Ok(())
    }

    /// 按时间窗检索 (epoch_ms). SQL 过滤:
    /// `created_ms >= from_ms AND (valid_until_ms IS NULL OR valid_until_ms >= until_ms)`.
    ///
    /// 跨 session 扫全库; 老条目 (4 列 NULL) 的 created_ms 由 timestamp*1000 兜底,
    /// valid_until NULL 视为永久, 自然落入结果集。
    pub fn query_with_time_range(&self, from_ms: i64, until_ms: i64) -> MemoryResult<Vec<Episode>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, continuity_id, session_id, timestamp, role, content, \
                    valid_from_ms, valid_until_ms, created_ms, provenance \
             FROM episodes \
             WHERE (created_ms IS NOT NULL OR timestamp IS NOT NULL) \
               AND COALESCE(created_ms, timestamp * 1000) >= ?1 \
               AND (valid_until_ms IS NULL OR valid_until_ms >= ?2) \
             ORDER BY COALESCE(created_ms, timestamp * 1000) ASC, id ASC",
        )?;
        let rows = stmt.query_map(params![from_ms, until_ms], row_to_episode_v4)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// 读一条 episode 的 4 列元数据 (用于后续 `normalize_meta` 兜底)。
    pub fn read_episode_meta(&self, id: &str) -> MemoryResult<Option<EpisodeMeta>> {
        let conn = self.conn()?;
        let row = conn
            .query_row(
                "SELECT valid_from_ms, valid_until_ms, created_ms, provenance \
                 FROM episodes WHERE id = ?1",
                params![id],
                |row| {
                    let vf: Option<i64> = row.get(0)?;
                    let vu: Option<i64> = row.get(1)?;
                    let cm: Option<i64> = row.get(2)?;
                    let ps: Option<String> = row.get(3)?;
                    Ok((vf, vu, cm, ps))
                },
            )
            .optional()?;
        let Some((vf, vu, cm, ps)) = row else {
            return Ok(None);
        };
        Ok(Some(EpisodeMeta {
            valid_from_ms: vf,
            valid_until_ms: vu,
            created_ms: cm.unwrap_or(0),
            provenance: ps.as_deref().map(Provenance::from_db).unwrap_or_default(),
        }))
    }
}

/// V4 row 解析: SELECT 返回 10 列, 但本任务只返回 `Episode` (5 字段)。
/// 元数据列由调用方按需单独读 (通过 [`crate::SqliteMemoryStore::read_episode_meta`])。
fn row_to_episode_v4(row: &rusqlite::Row<'_>) -> rusqlite::Result<Episode> {
    Ok(Episode {
        id: row.get(0)?,
        timestamp: row.get(3)?, // SELECT 顺序: id(0), continuity_id(1), session_id(2), timestamp(3), role(4), content(5), ...
        role: row.get(4)?,
        content: row.get(5)?,
        session_id: row.get(2)?,
    })
}

/// 校验元数据不变量 (调试 / 测试用)。
pub fn validate_meta(meta: &EpisodeMeta) -> Result<(), String> {
    if meta.valid_from_ms.is_some() && meta.valid_until_ms.is_some() {
        if meta.valid_from_ms.unwrap() > meta.valid_until_ms.unwrap() {
            return Err(format!(
                "valid_from_ms ({:?}) > valid_until_ms ({:?})",
                meta.valid_from_ms, meta.valid_until_ms
            ));
        }
    }
    if meta.created_ms < 0 {
        return Err(format!(
            "created_ms ({}) must be non-negative",
            meta.created_ms
        ));
    }
    Ok(())
}

#[allow(dead_code)]
fn _ensure_conn(_c: &Connection) {} // anchor for Connection import (unused at runtime)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_default_is_manual() {
        assert_eq!(Provenance::default(), Provenance::Manual);
    }

    #[test]
    fn provenance_roundtrip() {
        for p in [
            Provenance::Dialog,
            Provenance::Tool,
            Provenance::Reflection,
            Provenance::Observation,
            Provenance::Manual,
        ] {
            assert_eq!(Provenance::from_db(p.as_str()), p);
        }
        // 未知值降级 Manual
        assert_eq!(Provenance::from_db("garbage"), Provenance::Manual);
        assert_eq!(Provenance::from_db(""), Provenance::Manual);
    }

    #[test]
    fn normalize_meta_fills_defaults() {
        // 老条目 (全 None, timestamp = 1000s)
        let (vf, vu, cm, pr) = normalize_meta(None, None, None, None, 1000);
        assert_eq!(vf, Some(1_000_000)); // created_ms 兜底 timestamp*1000
        assert_eq!(vu, None); // 永久
        assert_eq!(cm, 1_000_000);
        assert_eq!(pr, Provenance::Manual);

        // 部分缺失: valid_from None → created_ms 兜
        let (vf, vu, cm, pr) = normalize_meta(
            None,
            Some(2_000_000),
            Some(1_500_000),
            Some(Provenance::Dialog),
            9999,
        );
        assert_eq!(vf, Some(1_500_000)); // valid_from 缺失 → created_ms
        assert_eq!(vu, Some(2_000_000)); // valid_until 保留
        assert_eq!(cm, 1_500_000);
        assert_eq!(pr, Provenance::Dialog);
    }

    #[test]
    fn validate_meta_ok() {
        let m = EpisodeMeta::now(100, Provenance::Manual);
        assert!(validate_meta(&m).is_ok());
        // valid_until < valid_from 报错
        let bad = EpisodeMeta {
            valid_from_ms: Some(200),
            valid_until_ms: Some(100),
            created_ms: 150,
            provenance: Provenance::Manual,
        };
        assert!(validate_meta(&bad).is_err());
    }

    #[test]
    fn episode_meta_now_helper() {
        let m = EpisodeMeta::now(42, Provenance::Tool);
        assert_eq!(m.created_ms, 42);
        assert_eq!(m.valid_from_ms, Some(42));
        assert_eq!(m.valid_until_ms, None);
        assert_eq!(m.provenance, Provenance::Tool);
    }
}
