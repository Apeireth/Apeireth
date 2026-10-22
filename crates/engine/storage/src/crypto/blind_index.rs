//! Blind index face (B6-BLIND): 查重/溯源的盲索引, 索引面与内容面分离.
//!
//! ## 索引面与内容面分离 (检索面原则, 自己的行文)
//!
//! 落盘数据分两面: **内容面** (SealedFingerprint/SealedContent 档, 密文
//! 永不出查询面) 与**索引面** (只放单向检索键)。查重/溯源的等值检索
//! 只走索引面: 给原值 → 算单向键 → 查索引列; **查询不触原文**、不需要
//! 解封内容列、不把明文哈希摆上盘。索引键是**盲**的: 从分区密钥派生、
//! 单向、同值恒同键、跨分区不同键 —— 检索能力拿到, 原值拿不回。
//!
//! ## 构造 (0 新外部依赖)
//!
//! - 盲索引键 [`BlindIndexKey`] = HMAC-SHA-256(分区 DEK, 域分隔串 ‖ 分区 id):
//!   **派生子钥**, 不复用加密密钥 (AES 用的是 DEK 本体, 索引用派生子钥),
//!   派生绑定分区 id —— 同 DEK 挪分区照样换键, 分区隔离墙不破;
//! - 索引值 [`BlindIndex`] = HMAC-SHA-256(盲索引键, 原值), 落盘保明但单向;
//! - HMAC 构造复用 [`crate::crypto::kdf::hmac_sha256`] 的仓内纯构造实现
//!   (密钥异或内外垫 + 两次哈希), 正确性由公开标准向量测试钉死 (kdf 面);
//!   锁内无独立 HMAC 包, 按 0 新外部依赖口径复用之, 不引新包。
//!
//! ## 取舍 (如实, 不假装无代价)
//!
//! **换来**: 查重快 (等值命中走索引列) + 查询不触密文 + 明文哈希不上盘。
//! **代价**: 持盲索引键者可**离线枚举候选值** —— 等值匹配的索引面天然可被
//! 字典枚举 (猜候选原值 → 算键 → 对索引), 本构造**不防**持钥者枚举
//! (要防需另建带代价的检索构造, 未做)。另有两条残余:
//! 1. 索引条目**无行级认证** —— 落盘层篡改条目 = **命中失效** (漏检,
//!    本批如实选定的篡改语义; 不吐错值、不毁内容面); 条目删除/重放同样
//!    造成漏检或伪命中 (加固选项 = 条目级校验码, 未做, 登记残余);
//! 2. 盲索引键随分区密钥走: 密钥丢失 = 索引面同失 (与内容面同命运, 见
//!    [`crate::crypto::backup::KEY_LOSS_BOUNDARY`])。
//!
//! ## 与定级/装饰层的关系
//!
//! [`crate::crypto::classification::FieldClass::SealedFingerprint`] 档
//! (content_hash/fingerprint) 的查重检索走本面: 内容列照封
//! ([`crate::crypto::SealingLayer`]), 索引列走 [`BlindIndexSink`]。
//! 白名单与 [`crate::crypto::sqlite_sink::SealedTable`] 同步 (零注入面)。

use rusqlite::Connection;
use zeroize::Zeroizing;

use crate::crypto::error::CryptoError;
use crate::crypto::kdf::hmac_sha256;
use crate::crypto::partition::PartitionKey;
use crate::crypto::sqlite_sink::SealedTable;
use crate::migrations::run_migrations;

/// 盲索引键派生的域分隔串 (与加密用途分域, 防跨用途挪用)。
const BLIND_KEY_CONTEXT: &[u8] = b"apeireth.blind-index.v1";

/// 索引面自建表 (索引面自有 schema; 不动共享迁移面, 战场纪律)。
const BLIND_INDEX_TABLE_SQL: &str = "
CREATE TABLE IF NOT EXISTS blind_index_entries (
    namespace   TEXT NOT NULL,
    blind_index TEXT NOT NULL,
    row_id      TEXT NOT NULL,
    PRIMARY KEY (namespace, blind_index, row_id)
);
CREATE INDEX IF NOT EXISTS idx_blind_index_entries_lookup
    ON blind_index_entries(namespace, blind_index);
";

/// 盲索引键 (派生子钥; `Debug` 不铺键字节, Drop 零化)。
#[derive(Clone)]
pub struct BlindIndexKey {
    partition: crate::crypto::partition::PartitionId,
    key: Zeroizing<[u8; 32]>,
}

impl BlindIndexKey {
    /// 从分区 DEK 派生盲索引键:
    /// `HMAC-SHA-256(DEK, 域分隔串 ‖ 分区 id)`。
    ///
    /// - **不复用加密密钥**: AES 用 DEK 本体, 索引用此派生子钥;
    /// - **派生绑分区 id**: 同 DEK 挪分区 = 换键 (分区墙不破)。
    pub fn derive(dek: &PartitionKey) -> Self {
        let mut message =
            Vec::with_capacity(BLIND_KEY_CONTEXT.len() + 1 + dek.partition().as_str().len());
        message.extend_from_slice(BLIND_KEY_CONTEXT);
        message.push(b'|');
        message.extend_from_slice(dek.partition().as_str().as_bytes());
        let key = hmac_sha256(dek.expose_dek(), &message);
        Self {
            partition: dek.partition().clone(),
            key: Zeroizing::new(key),
        }
    }

    /// 归属分区。
    pub fn partition(&self) -> &crate::crypto::partition::PartitionId {
        &self.partition
    }

    /// 对原值算盲索引 (`HMAC-SHA-256(盲索引键, 原值)`, 同值恒同键)。
    ///
    /// 原值口径由调用方统一 (同一逻辑值的不同序列化 = 不同键, 比对口径
    /// 必须一致)。
    pub fn index(&self, value: &[u8]) -> BlindIndex {
        BlindIndex(hmac_sha256(&self.key[..], value))
    }
}

impl std::fmt::Debug for BlindIndexKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug 永不铺盲索引键字节 (脱敏红线)。
        formatter
            .debug_struct("BlindIndexKey")
            .field("partition", &self.partition)
            .field("key", &"[redacted]")
            .finish()
    }
}

/// 盲索引值 (单向检索键; 落盘保明形态 = hex)。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlindIndex([u8; 32]);

impl BlindIndex {
    /// 原始索引字节。
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// 落盘/查询用 hex 形态。
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// 从 hex 形态读回 (比对/迁移用)。
    pub fn from_hex(text: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(text.trim())
            .map_err(|error| CryptoError::Encoding(format!("盲索引 hex 畸形: {error}")))?;
        let array: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
            CryptoError::Encoding(format!("盲索引长度不符: {} (需 32 字节)", bytes.len()))
        })?;
        Ok(Self(array))
    }
}

impl std::fmt::Debug for BlindIndex {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 索引值本身是单向公开值 (落盘保明), Debug 可见 hex。
        formatter.write_str("BlindIndex(")?;
        formatter.write_str(&self.to_hex())?;
        formatter.write_str(")")
    }
}

/// 索引目标: 白名单表 + 列 (namespace = `<表名>.<列名>`, 绑定参数落值)。
///
/// 表走 [`SealedTable`] 白名单 (零 SQL 注入面); 列名只作**绑定参数值**
/// (不进 SQL 拼接)。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexTarget {
    /// 白名单表 (与 [`SealedTable`] 同步)。
    pub table: SealedTable,
    /// 列名 (如 `content_hash` / `fingerprint`)。
    pub column: String,
}

impl IndexTarget {
    /// 构造索引目标 (列名非空校验)。
    pub fn new(table: SealedTable, column: impl Into<String>) -> Result<Self, CryptoError> {
        let column = column.into();
        if column.trim().is_empty() {
            return Err(CryptoError::WrapRefused("索引列名不得为空".to_string()));
        }
        Ok(Self { table, column })
    }

    /// namespace 值 (表名.列名)。
    pub fn namespace(&self) -> String {
        format!("{}.{}", self.table.as_str(), self.column)
    }
}

/// 盲索引存储 (查询面): 等值查重/溯源, 不触内容面。
pub struct BlindIndexSink {
    conn: Connection,
    key: BlindIndexKey,
}

impl BlindIndexSink {
    /// 打开文件库 (跑共享迁移 + 索引面自建表)。
    pub fn open_file(
        path: impl AsRef<std::path::Path>,
        key: BlindIndexKey,
    ) -> Result<Self, CryptoError> {
        let conn = Connection::open(path)?;
        run_migrations(&conn)?;
        conn.execute_batch(BLIND_INDEX_TABLE_SQL)?;
        Ok(Self { conn, key })
    }

    /// 打开内存库 (跑共享迁移 + 索引面自建表)。
    pub fn in_memory(key: BlindIndexKey) -> Result<Self, CryptoError> {
        let conn = Connection::open_in_memory()?;
        run_migrations(&conn)?;
        conn.execute_batch(BLIND_INDEX_TABLE_SQL)?;
        Ok(Self { conn, key })
    }

    /// 归属分区 (与内容面分区一致才有意义)。
    pub fn partition_key(&self) -> &BlindIndexKey {
        &self.key
    }

    /// 登记一条索引 (原值 → 盲索引; 幂等, 同 (目标, 原值, 行) 重复登记不炸)。
    pub fn insert(
        &mut self,
        target: &IndexTarget,
        row_id: &str,
        value: &[u8],
    ) -> Result<(), CryptoError> {
        let index = self.key.index(value);
        self.conn.execute(
            "INSERT OR IGNORE INTO blind_index_entries (namespace, blind_index, row_id) \
             VALUES (?1, ?2, ?3)",
            rusqlite::params![target.namespace(), index.to_hex(), row_id],
        )?;
        Ok(())
    }

    /// 等值查询: 给原值 → 算盲索引 → 命中行 id (升序, 确定性)。
    ///
    /// **查询不触内容面**: 只走索引表, 不解封任何密文列。
    pub fn find(&self, target: &IndexTarget, value: &[u8]) -> Result<Vec<String>, CryptoError> {
        let index = self.key.index(value);
        let mut statement = self.conn.prepare(
            "SELECT row_id FROM blind_index_entries \
             WHERE namespace = ?1 AND blind_index = ?2 ORDER BY row_id ASC",
        )?;
        let rows = statement.query_map(
            rusqlite::params![target.namespace(), index.to_hex()],
            |row| row.get::<_, String>(0),
        )?;
        let mut hits = Vec::new();
        for row in rows {
            hits.push(row?);
        }
        Ok(hits)
    }

    /// 是否存在命中 (查重布尔口)。
    pub fn contains(&self, target: &IndexTarget, value: &[u8]) -> Result<bool, CryptoError> {
        Ok(!self.find(target, value)?.is_empty())
    }

    /// 行删除时清其全部索引条目 (行没了索引不得悬空)。
    pub fn remove_row(&mut self, target: &IndexTarget, row_id: &str) -> Result<(), CryptoError> {
        self.conn.execute(
            "DELETE FROM blind_index_entries WHERE namespace = ?1 AND row_id = ?2",
            rusqlite::params![target.namespace(), row_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::partition::PartitionId;

    fn dek(partition: &str) -> PartitionKey {
        PartitionKey::generate(PartitionId::new(partition).expect("id"))
    }

    #[test]
    fn same_value_yields_the_same_index_and_different_values_differ() {
        let key = BlindIndexKey::derive(&dek("user-a"));
        let first = key.index(b"sha256:abc");
        let second = key.index(b"sha256:abc");
        let other = key.index(b"sha256:def");
        assert_eq!(first, second, "同值必须恒同键 (查重比对成立)");
        assert_ne!(first, other, "异值必须异键");
        assert_eq!(first.to_hex(), hex::encode(first.as_bytes()));
    }

    #[test]
    fn blind_index_is_one_way_and_never_contains_the_original_value() {
        let key = BlindIndexKey::derive(&dek("user-a"));
        let value = b"sha256:original-fingerprint-value";
        let index = key.index(value);
        assert_ne!(index.as_bytes().as_slice(), value, "索引值不得等于原值");
        assert!(
            !index.to_hex().contains(&hex::encode(value)),
            "索引 hex 不得含原值 hex (单向)"
        );
    }

    #[test]
    fn cross_partition_indexes_differ_even_for_the_same_dek() {
        let dek_a = dek("user-a");
        let key_a = BlindIndexKey::derive(&dek_a);
        // 同 DEK 换分区 id (挪用场景): 派生绑分区, 照样换键。
        let twin =
            PartitionKey::from_bytes(PartitionId::new("user-b").expect("id"), *dek_a.expose_dek());
        let key_b = BlindIndexKey::derive(&twin);

        assert_ne!(
            key_a.index(b"sha256:abc"),
            key_b.index(b"sha256:abc"),
            "同 DEK 挪分区必须换盲索引键 (派生绑分区 id)"
        );
        assert_ne!(
            key_a.index(b"sha256:abc"),
            BlindIndexKey::derive(&dek("user-b")).index(b"sha256:abc"),
            "他分区盲索引必须不同"
        );
    }

    #[test]
    fn index_target_namespace_is_table_dot_column() {
        let target = IndexTarget::new(SealedTable::Notes, "content_hash").expect("target");
        assert_eq!(target.namespace(), "notes.content_hash");
        assert!(
            IndexTarget::new(SealedTable::Notes, "  ").is_err(),
            "空列名必须拒"
        );
    }

    #[test]
    fn blind_index_hex_round_trips() {
        let key = BlindIndexKey::derive(&dek("user-a"));
        let index = key.index(b"sha256:abc");
        let back = BlindIndex::from_hex(&index.to_hex()).expect("hex");
        assert_eq!(back, index);
        assert!(BlindIndex::from_hex("deadbeef").is_err(), "长度不符必须拒");
    }

    #[test]
    fn sink_dedup_round_trip_hits_without_touching_content() {
        let key = BlindIndexKey::derive(&dek("user-a"));
        let mut sink = BlindIndexSink::in_memory(key).expect("sink");
        let target = IndexTarget::new(SealedTable::Notes, "content_hash").expect("target");

        sink.insert(&target, "row-1", b"sha256:abc")
            .expect("insert");
        sink.insert(&target, "row-2", b"sha256:abc")
            .expect("insert");
        sink.insert(&target, "row-3", b"sha256:def")
            .expect("insert");

        assert_eq!(
            sink.find(&target, b"sha256:abc").expect("find"),
            vec!["row-1".to_string(), "row-2".to_string()],
            "同指纹必须全命中 (查重往返)"
        );
        assert!(sink.find(&target, b"sha256:zzz").expect("find").is_empty());
        assert!(sink.contains(&target, b"sha256:def").expect("contains"));
        assert!(!sink.contains(&target, b"sha256:zzz").expect("contains"));

        sink.remove_row(&target, "row-1").expect("remove");
        assert_eq!(
            sink.find(&target, b"sha256:abc").expect("find"),
            vec!["row-2".to_string()],
            "删行必须清索引 (不悬空)"
        );
    }

    #[test]
    fn debug_never_prints_the_blind_index_key() {
        let key = BlindIndexKey::derive(&dek("user-a"));
        let rendered = format!("{key:?}");
        assert!(!rendered.contains("key: ["), "Debug 必须标 redacted");
        assert!(rendered.contains("[redacted]"), "Debug 必须显式脱敏");
    }
}
