//! Sealing layer: 加密面可装饰任意存储后端 (创世标尺 §3 可插拔存储).
//!
//! §3 的「可插拔存储」要求存储后端可换 (SQLite 现役, 向量/全文后端量级
//! 触发时替换) —— 加密面因此做成**装饰层**而不是某后端的内嵌特性:
//! 任何实现 [`FieldSink`] 的键值后端都能被 [`SealingLayer`] 包一层,
//! 写入封缄、读取开封, 换后端不动加密语义。
//!
//! 装饰层只管**值**的机密性/完整性 (AES-256-GCM + 分区 AAD); 键
//! (`field_key`) 是检索身份, 不在加密范围 —— 列级接入时哪些列作键、
//! 哪些列进值, 由各存储 owner 按敏感字段清单定 (见批报告)。

use crate::crypto::error::CryptoError;
use crate::crypto::partition::PartitionKey;
use crate::crypto::sealed::SealedField;

/// 最小键值后端读写口 (可插拔存储的装饰接口)。
///
/// 语义: `get_field` 无键 = `Ok(None)` (如实报缺); 实现方负责自身
/// 的持久化/错误映射, 本 trait 不含任何加密语义。
pub trait FieldSink {
    /// 读一个字段的原始落盘值。
    fn get_field(&self, field_key: &str) -> Result<Option<Vec<u8>>, CryptoError>;

    /// 写一个字段的原始落盘值。
    fn put_field(&mut self, field_key: &str, value: Vec<u8>) -> Result<(), CryptoError>;
}

/// 加密封装饰层: 写入时 [`PartitionKey::seal`], 读取时 [`PartitionKey::open`]。
///
/// 不变量 (测试钉死):
/// - 后端里落的是密文单元字节, **不含明文子串**;
/// - 换分区 DEK 读同一后端 = 拒解 (跨分区隔离墙);
/// - 落盘值被篡改 = 拒解。
pub struct SealingLayer<S> {
    sink: S,
    key: PartitionKey,
}

impl<S: FieldSink> SealingLayer<S> {
    /// 装饰一个后端 (绑定分区 DEK)。
    pub fn new(sink: S, key: PartitionKey) -> Self {
        Self { sink, key }
    }

    /// 写一个密封字段 (明文只经 [`PartitionKey::seal`] 一次)。
    pub fn put_sealed(&mut self, field_key: &str, plaintext: &[u8]) -> Result<(), CryptoError> {
        let sealed = self.key.seal(plaintext)?;
        self.sink.put_field(field_key, sealed.to_bytes().to_vec())
    }

    /// 读一个密封字段 (无键 = `Ok(None)`; 密文坏/换分区 = 拒解)。
    pub fn get_sealed(&self, field_key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
        let Some(raw) = self.sink.get_field(field_key)? else {
            return Ok(None);
        };
        let sealed = SealedField::from_bytes(raw)?;
        Ok(Some(self.key.open(&sealed)?))
    }

    /// 分区 DEK (装饰层归属分区)。
    pub fn partition_key(&self) -> &PartitionKey {
        &self.key
    }

    /// 拆回被装饰的后端 (密钥不随行)。
    pub fn into_inner(self) -> S {
        self.sink
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::partition::PartitionId;
    use std::collections::BTreeMap;

    /// 测试后端: 内存 BTreeMap (任意实现 [`FieldSink`] 的后端都可装饰)。
    #[derive(Default)]
    struct MemorySink(BTreeMap<String, Vec<u8>>);

    impl FieldSink for MemorySink {
        fn get_field(&self, field_key: &str) -> Result<Option<Vec<u8>>, CryptoError> {
            Ok(self.0.get(field_key).cloned())
        }

        fn put_field(&mut self, field_key: &str, value: Vec<u8>) -> Result<(), CryptoError> {
            self.0.insert(field_key.to_string(), value);
            Ok(())
        }
    }

    #[test]
    fn layer_seals_on_write_and_opens_on_read() {
        let key = PartitionKey::generate(PartitionId::new("user-a").expect("id"));
        let mut layer = SealingLayer::new(MemorySink::default(), key);
        layer
            .put_sealed("episode:1", "秘密对话内容".as_bytes())
            .expect("put");
        assert_eq!(
            layer.get_sealed("episode:1").expect("get"),
            Some("秘密对话内容".as_bytes().to_vec())
        );
        assert_eq!(layer.get_sealed("episode:missing").expect("get"), None);
    }

    #[test]
    fn layer_sink_never_sees_the_plaintext() {
        let key = PartitionKey::generate(PartitionId::new("user-a").expect("id"));
        let mut layer = SealingLayer::new(MemorySink::default(), key);
        layer
            .put_sealed("episode:1", "秘密对话内容".as_bytes())
            .expect("put");
        let sink = layer.into_inner();
        let raw = sink.get_field("episode:1").expect("get").expect("value");
        assert!(
            !raw.windows("秘密对话内容".as_bytes().len())
                .any(|window| window == "秘密对话内容".as_bytes()),
            "后端明文子串必须为零"
        );
    }

    #[test]
    fn layer_refuses_cross_partition_reads_and_tampered_values() {
        let key_a = PartitionKey::generate(PartitionId::new("user-a").expect("id"));
        let key_b = PartitionKey::generate(PartitionId::new("user-b").expect("id"));
        let mut layer_a = SealingLayer::new(MemorySink::default(), key_a.clone());
        layer_a
            .put_sealed("episode:1", "秘密对话内容".as_bytes())
            .expect("put");
        let sink = layer_a.into_inner();

        let layer_b = SealingLayer::new(sink, key_b);
        assert!(layer_b.get_sealed("episode:1").is_err(), "换分区必须拒解");

        let mut sink = layer_b.into_inner();
        let raw = sink.get_field("episode:1").expect("get").expect("value");
        let mut tampered = raw.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        sink.put_field("episode:1", tampered).expect("put");
        let layer_a = SealingLayer::new(sink, key_a);
        assert!(layer_a.get_sealed("episode:1").is_err(), "篡改必须拒解");
    }
}
