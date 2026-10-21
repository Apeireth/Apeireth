//! SQLite column seam: 把 [`SealingLayer`] 真接进真实存储表 (B6-DEEP 列级接入示范).
//!
//! 真实表 = 本 crate [`crate::run_migrations`] 建出的 `(id, data)` 形态表
//! (episodes/notes/sessions/agent_traces/facts/links/topic_groups/provenance,
//! schema v1)。[`SqliteFieldSink`] 是 [`FieldSink`] 的真实 SQLite 后端,
//! [`SealingLayer<SqliteFieldSink>`] = **列级接入端到端示范**: `data` 列走
//! [`SealedField`] (分区 DEK 封缄), 敏感列永不落明文。
//!
//! ## 接缝边界 (接入登记归口)
//!
//! 本文件是 **storage 侧接缝** —— 各存储 owner (memory/episode.rs 等
//! 读写路径, 属他人战场) 的真接入归口其 owner, 本批只交付:
//! ① 真实表上的列级封缄读写接缝; ② 往返/重启/篡改拒读端到端测试。
//! 列级定级见 [`crate::crypto::classification`] (哪些列该走本接缝)。
//!
//! ## 落盘形态
//!
//! `data` 列 (TEXT 亲和) 存 [`SealedField`] **hex 形态** (TEXT 列口径,
//! 见 [`crate::crypto::sealed`]); 读回 hex 解码失败 = 显式拒
//! ([`CryptoError::Encoding`]), 不回退明文、不猜编码 (fail-closed)。

use rusqlite::Connection;

use crate::crypto::error::CryptoError;
use crate::crypto::layer::FieldSink;
use crate::migrations::run_migrations;

/// 可列级封缄的真实表 (storage schema v1 的 `(id, data)` 形态表, 白名单制)。
///
/// 白名单 = 表名**不走字符串拼接** (SQL 注入面为零), 也保证接缝只用于
/// 已知形态的表 (id TEXT PRIMARY KEY, data TEXT)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SealedTable {
    /// `episodes` (id, data)。
    Episodes,
    /// `notes` (id, data)。
    Notes,
    /// `sessions` (id, data)。
    Sessions,
    /// `agent_traces` (id, data)。
    AgentTraces,
    /// `facts` (id, data)。
    Facts,
    /// `links` (id, data)。
    Links,
    /// `topic_groups` (id, data)。
    TopicGroups,
    /// `provenance` (id, data)。
    Provenance,
}

impl SealedTable {
    /// 真实表名 (静态串, 白名单制)。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Episodes => "episodes",
            Self::Notes => "notes",
            Self::Sessions => "sessions",
            Self::AgentTraces => "agent_traces",
            Self::Facts => "facts",
            Self::Links => "links",
            Self::TopicGroups => "topic_groups",
            Self::Provenance => "provenance",
        }
    }
}

/// [`FieldSink`] 的真实 SQLite 后端 (装饰目标 = 真实表)。
pub struct SqliteFieldSink {
    conn: Connection,
    table: SealedTable,
}

impl SqliteFieldSink {
    /// 打开文件库并跑 storage 迁移 (真实 schema)。
    pub fn open_file(
        path: impl AsRef<std::path::Path>,
        table: SealedTable,
    ) -> Result<Self, CryptoError> {
        let conn = Connection::open(path)?;
        run_migrations(&conn)?;
        Ok(Self { conn, table })
    }

    /// 打开内存库并跑 storage 迁移 (真实 schema, 单连接口径)。
    pub fn in_memory(table: SealedTable) -> Result<Self, CryptoError> {
        let conn = Connection::open_in_memory()?;
        run_migrations(&conn)?;
        Ok(Self { conn, table })
    }

    /// 装饰成列级封缄层 (data 列走 [`crate::crypto::SealedField`])。
    pub fn seal_with(self, key: crate::crypto::PartitionKey) -> crate::crypto::SealingLayer<Self> {
        crate::crypto::SealingLayer::new(self, key)
    }

    /// 真实表名。
    pub fn table(&self) -> SealedTable {
        self.table
    }
}

impl FieldSink for SqliteFieldSink {
    fn get_field(&self, field_key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        use rusqlite::OptionalExtension;

        let sql = format!("SELECT data FROM {} WHERE id = ?1", self.table.as_str());
        let stored: Option<String> = self
            .conn
            .query_row(&sql, [field_key], |row| row.get(0))
            .optional()?;
        match stored {
            None => Ok(None),
            Some(hex_text) => {
                let bytes = hex::decode(hex_text.trim()).map_err(|error| {
                    CryptoError::Encoding(format!(
                        "表 `{}` 件 `{}` 的 data 列不是 hex 密文形态 (fail-closed 拒读): {error}",
                        self.table.as_str(),
                        field_key
                    ))
                })?;
                Ok(Some(bytes))
            }
        }
    }

    fn put_field(&mut self, field_key: &str, value: Vec<u8>) -> Result<(), CryptoError> {
        let sql = format!(
            "INSERT INTO {} (id, data) VALUES (?1, ?2) \
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            self.table.as_str()
        );
        self.conn
            .execute(&sql, rusqlite::params![field_key, hex::encode(value)])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_tables_map_to_the_real_storage_schema_names() {
        assert_eq!(SealedTable::Episodes.as_str(), "episodes");
        assert_eq!(SealedTable::Notes.as_str(), "notes");
        assert_eq!(SealedTable::Provenance.as_str(), "provenance");
    }

    #[test]
    fn in_memory_sink_round_trips_raw_values_on_a_real_table() {
        let mut sink = SqliteFieldSink::in_memory(SealedTable::Notes).expect("sink");
        assert_eq!(sink.get_field("n1").expect("get"), None);
        sink.put_field("n1", b"raw-value".to_vec()).expect("put");
        assert_eq!(
            sink.get_field("n1").expect("get"),
            Some(b"raw-value".to_vec())
        );
    }

    #[test]
    fn non_hex_column_value_is_refused_not_guessed() {
        let mut sink = SqliteFieldSink::in_memory(SealedTable::Notes).expect("sink");
        sink.put_field("n1", b"raw".to_vec()).expect("put");
        // 直连改列值成明文垃圾: 接缝必须 fail-closed 拒读, 不回退明文。
        sink.conn
            .execute(
                "UPDATE notes SET data = 'not-hex-plaintext' WHERE id = 'n1'",
                [],
            )
            .expect("tamper");
        let error = sink.get_field("n1").expect_err("非 hex 必须拒");
        assert!(matches!(error, CryptoError::Encoding(_)));
    }
}
