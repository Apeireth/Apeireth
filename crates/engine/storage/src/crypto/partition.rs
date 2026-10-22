//! Partition-level DEK: 分区数据加密密钥 (创世标尺 §5 单主体×N分区).
//!
//! **分区隔离墙 → 分区级 DEK**: 每个记忆分区 (人) 一把数据加密密钥,
//! AAD 绑分区 id —— 密文挪到别的分区**拒解** (跨分区披露 R4 零容忍的
//! 密码学执法面); 分区内透明不受损: 密钥在手 = 可读/可导出/可清。

use zeroize::Zeroizing;

use crate::crypto::error::CryptoError;
use crate::crypto::sealed::{open_raw, seal_raw, SealedField};

/// 分区标识 (非空字符串; 与用户域/记忆分区一一对应)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PartitionId(String);

impl PartitionId {
    /// 构造分区标识 (空串拒绝)。
    pub fn new(id: impl Into<String>) -> Result<Self, CryptoError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(CryptoError::WrapRefused("分区 id 不得为空".to_string()));
        }
        Ok(Self(id))
    }

    /// 分区 id 字符串。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PartitionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// 字段密封 AAD: 域分隔串 + 分区 id (跨分区挪用拒解的绑定面)。
fn field_aad(partition: &PartitionId) -> Vec<u8> {
    let mut aad = Vec::with_capacity(b"apeireth.sealed.field.v1".len() + partition.0.len());
    aad.extend_from_slice(b"apeireth.sealed.field.v1");
    aad.extend_from_slice(partition.0.as_bytes());
    aad
}

/// 分区数据加密密钥 (DEK, 32 字节, [`Zeroizing`] 持钥)。
///
/// 每分区一把; 随机生成 ([`PartitionKey::generate`]) 或从备份/包裹恢复
/// ([`PartitionKey::from_bytes`])。Drop 时密钥材料零化。
#[derive(Clone)]
pub struct PartitionKey {
    partition: PartitionId,
    dek: Zeroizing<[u8; 32]>,
}

impl PartitionKey {
    /// 随机生成分区 DEK (OsRng)。
    pub fn generate(partition: PartitionId) -> Self {
        use rand::RngCore;
        let mut dek = Zeroizing::new([0u8; 32]);
        rand::rngs::OsRng.fill_bytes(&mut *dek);
        Self { partition, dek }
    }

    /// 从既有 DEK 字节组装 (备份恢复/包裹解出用)。
    pub fn from_bytes(partition: PartitionId, dek: [u8; 32]) -> Self {
        Self {
            partition,
            dek: Zeroizing::new(dek),
        }
    }

    /// 分区标识。
    pub fn partition(&self) -> &PartitionId {
        &self.partition
    }

    /// DEK 原始字节 (**密钥材料**: 仅供主密钥包裹使用, 不得落明面)。
    pub fn expose_dek(&self) -> &[u8; 32] {
        &self.dek
    }

    /// 封缄一个字段 (AES-256-GCM, nonce 随机, AAD 绑本分区)。
    pub fn seal(&self, plaintext: &[u8]) -> Result<SealedField, CryptoError> {
        seal_raw(&self.dek, &field_aad(&self.partition), plaintext)
    }

    /// 开封一个字段 (AAD 绑本分区: 换分区/换钥/篡改一律拒解)。
    pub fn open(&self, sealed: &SealedField) -> Result<Vec<u8>, CryptoError> {
        open_raw(
            &self.dek,
            &field_aad(&self.partition),
            sealed,
            "分区 DEK/分区 AAD/密文三者任一不符",
        )
    }
}

impl std::fmt::Debug for PartitionKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug 永不铺 DEK 字节。
        formatter
            .debug_struct("PartitionKey")
            .field("partition", &self.partition)
            .field("dek", &"[redacted]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_partition_id_is_refused() {
        assert!(PartitionId::new("  ").is_err());
    }

    #[test]
    fn seal_open_round_trips_inside_one_partition() {
        let key = PartitionKey::generate(PartitionId::new("user-a").expect("id"));
        let sealed = key.seal(b"private memory").expect("seal");
        assert_eq!(key.open(&sealed).expect("open"), b"private memory");
    }

    #[test]
    fn cross_partition_misuse_is_refused_both_directions() {
        let key_a = PartitionKey::generate(PartitionId::new("user-a").expect("id"));
        let key_b = PartitionKey::generate(PartitionId::new("user-b").expect("id"));
        let sealed = key_a.seal(b"private memory").expect("seal");
        assert!(key_b.open(&sealed).is_err(), "他分区 DEK 必须拒解");

        // 同 DEK 不同分区 id: AAD 绑分区, 挪用照样拒 (防跨分区挪用的核心断言)。
        let twin =
            PartitionKey::from_bytes(PartitionId::new("user-b").expect("id"), *key_a.expose_dek());
        assert!(
            twin.open(&sealed).is_err(),
            "同钥换分区 id 必须拒解 (AAD 绑分区)"
        );
    }

    #[test]
    fn debug_never_prints_the_dek() {
        let key = PartitionKey::generate(PartitionId::new("user-a").expect("id"));
        let rendered = format!("{key:?}");
        assert!(
            !rendered.contains(&hex::encode(key.expose_dek())),
            "Debug 不得含 DEK"
        );
    }
}
