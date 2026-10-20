//! Sealed field: AES-256-GCM 密文单元 (nonce 随机 + 密文 + tag).
//!
//! 字节布局 (v1):
//!
//! ```text
//! [0..4]   magic  "APS1"          (Apeireth Sealed v1)
//! [4]      format version = 1
//! [5..17]  nonce (12 bytes, 每封随机)
//! [17..]   ciphertext || tag (tag 16 bytes 在尾)
//! ```
//!
//! 序列化形态两种: 紧凑字节 ([`SealedField::to_bytes`], BLOB 列) 与
//! hex 字符串 (TEXT 列 / JSON)。**不变量**: 任一序列化形态都不含明文
//! 子串 (测试钉死) —— AES-256-GCM 是带认证加密, 密文+tag 中不含明文位。
//!
//! 低层入口只吃「密钥 + AAD + 明文」三元组; AAD 的分区绑定语义由
//! [`crate::crypto::partition::PartitionKey`] 组装 (跨分区挪用拒解)。

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::crypto::error::CryptoError;

/// 密文单元魔数 (格式信封/stored_doc 不共用: 此处是加密包裹标记)。
pub const SEAL_MAGIC: [u8; 4] = *b"APS1";

/// 密文单元格式版本。
pub const SEAL_FORMAT_VERSION: u8 = 1;

/// AES-GCM 标准 nonce 长度 (96 bit)。
pub const NONCE_LEN: usize = 12;

/// AES-GCM tag 长度 (128 bit)。
pub const TAG_LEN: usize = 16;

/// 头部长度 (magic + version + nonce)。
const HEADER_LEN: usize = 4 + 1 + NONCE_LEN;

/// 一个密封字段: 密文单元字节 (不可变, 克隆为深拷贝)。
#[derive(Clone, PartialEq, Eq)]
pub struct SealedField {
    bytes: Vec<u8>,
}

impl SealedField {
    /// 从完整密文单元字节构造 (校验魔数/版本/长度)。
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, CryptoError> {
        if bytes.len() < HEADER_LEN + TAG_LEN {
            return Err(CryptoError::Encoding(format!(
                "sealed 字节过短: {} < {}",
                bytes.len(),
                HEADER_LEN + TAG_LEN
            )));
        }
        if bytes[..4] != SEAL_MAGIC {
            return Err(CryptoError::Encoding("sealed 魔数不符".to_string()));
        }
        if bytes[4] != SEAL_FORMAT_VERSION {
            return Err(CryptoError::Encoding(format!(
                "sealed 格式版本不符: {} (支持 {})",
                bytes[4], SEAL_FORMAT_VERSION
            )));
        }
        Ok(Self { bytes })
    }

    /// 从 hex 串构造 (TEXT 列 / JSON 形态)。
    pub fn from_hex(text: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(text.trim())
            .map_err(|error| CryptoError::Encoding(format!("sealed hex 畸形: {error}")))?;
        Self::from_bytes(bytes)
    }

    /// 完整密文单元字节 (含魔数/版本/nonce)。
    pub fn to_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// hex 形态 (TEXT 列 / JSON 形态)。
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// 本封 nonce (仅展示/排查用; 解封走完整单元)。
    pub fn nonce(&self) -> &[u8] {
        &self.bytes[5..5 + NONCE_LEN]
    }

    /// 密文单元长度 (字节)。
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// 密文单元是否为空 (恒 false: 构造已校验最小长度; 满足 `len` 配对纪律)。
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// 内部构造: 由 AEAD 输出拼头部 (nonce 已由调用方随机生成)。
    pub(crate) fn assemble(nonce: &[u8; NONCE_LEN], ciphertext_and_tag: &[u8]) -> Self {
        let mut bytes = Vec::with_capacity(HEADER_LEN + ciphertext_and_tag.len());
        bytes.extend_from_slice(&SEAL_MAGIC);
        bytes.push(SEAL_FORMAT_VERSION);
        bytes.extend_from_slice(nonce);
        bytes.extend_from_slice(ciphertext_and_tag);
        Self { bytes }
    }

    /// 解析出 nonce 与密文+tag (只做切分, 不做解密)。
    pub(crate) fn split(&self) -> (&[u8; NONCE_LEN], &[u8]) {
        let nonce: &[u8; NONCE_LEN] = self.bytes[5..5 + NONCE_LEN]
            .try_into()
            .expect("header nonce slice is exactly NONCE_LEN");
        (nonce, &self.bytes[HEADER_LEN..])
    }
}

impl std::fmt::Debug for SealedField {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 密文单元本身不是秘密, 但 Debug 不铺整段: 只报长度 + 魔数。
        formatter
            .debug_struct("SealedField")
            .field("bytes", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

impl Serialize for SealedField {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for SealedField {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::from_hex(&text).map_err(serde::de::Error::custom)
    }
}

/// 低层封缄 (key + AAD + 明文 → 密文单元; nonce 每封随机)。
pub(crate) fn seal_raw(
    key: &[u8; 32],
    aad: &[u8],
    plaintext: &[u8],
) -> Result<SealedField, CryptoError> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Nonce};
    use rand::RngCore;

    let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CryptoError::SealFailed("aes-256-gcm encrypt 拒绝"))?;
    Ok(SealedField::assemble(&nonce_bytes, &ciphertext))
}

/// 低层开封 (key + AAD + 密文单元 → 明文; AEAD tag 不符 = 单一拒解)。
pub(crate) fn open_raw(
    key: &[u8; 32],
    aad: &[u8],
    sealed: &SealedField,
    reason: &'static str,
) -> Result<Vec<u8>, CryptoError> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    use aes_gcm::{Aes256Gcm, Nonce};

    let (nonce_bytes, ciphertext_and_tag) = sealed.split();
    let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext_and_tag,
                aad,
            },
        )
        .map_err(|_| CryptoError::OpenRefused { reason })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_field_round_trips_through_bytes_and_hex() {
        let key = [7u8; 32];
        let sealed = seal_raw(&key, b"aad", b"hello sealed world").expect("seal");
        let via_hex = SealedField::from_hex(&sealed.to_hex()).expect("hex round trip");
        assert_eq!(sealed, via_hex);
        let via_bytes =
            SealedField::from_bytes(sealed.to_bytes().to_vec()).expect("bytes round trip");
        assert_eq!(sealed, via_bytes);
        assert_eq!(
            open_raw(&key, b"aad", &sealed, "test").expect("open"),
            b"hello sealed world"
        );
    }

    #[test]
    fn sealed_field_shape_is_magic_version_nonce_ciphertext() {
        let sealed = seal_raw(&[9u8; 32], b"", b"payload").expect("seal");
        let bytes = sealed.to_bytes();
        assert_eq!(&bytes[..4], &SEAL_MAGIC);
        assert_eq!(bytes[4], SEAL_FORMAT_VERSION);
        assert_eq!(sealed.nonce(), &bytes[5..5 + NONCE_LEN]);
        assert_eq!(sealed.len(), HEADER_LEN + "payload".len() + TAG_LEN);
    }

    #[test]
    fn malformed_sealed_fields_are_refused_at_parse_time() {
        assert!(SealedField::from_bytes(vec![0u8; 4]).is_err(), "过短必须拒");
        let mut bad_magic = seal_raw(&[1u8; 32], b"", b"payload")
            .expect("seal")
            .to_bytes()
            .to_vec();
        bad_magic[0] = b'X';
        assert!(SealedField::from_bytes(bad_magic).is_err(), "坏魔数必须拒");
        let mut bad_version = seal_raw(&[1u8; 32], b"", b"payload")
            .expect("seal")
            .to_bytes()
            .to_vec();
        bad_version[4] = 9;
        assert!(
            SealedField::from_bytes(bad_version).is_err(),
            "未知版本必须拒"
        );
        assert!(SealedField::from_hex("not-hex").is_err(), "坏 hex 必须拒");
    }

    #[test]
    fn open_refuses_wrong_key_wrong_aad_and_tamper_as_one_case() {
        let key = [3u8; 32];
        let sealed = seal_raw(&key, b"aad-a", b"payload").expect("seal");
        assert!(
            open_raw(&[4u8; 32], b"aad-a", &sealed, "test").is_err(),
            "换钥必须拒"
        );
        assert!(
            open_raw(&key, b"aad-b", &sealed, "test").is_err(),
            "换 AAD 必须拒"
        );

        let mut tampered = sealed.to_bytes().to_vec();
        let last = tampered.len() - 1;
        tampered[last] ^= 0x01;
        let tampered = SealedField::from_bytes(tampered).expect("shape ok");
        assert!(
            open_raw(&key, b"aad-a", &tampered, "test").is_err(),
            "翻位必须拒"
        );
    }

    #[test]
    fn every_seal_draws_a_fresh_nonce() {
        let key = [5u8; 32];
        let first = seal_raw(&key, b"", b"same plaintext").expect("seal");
        let second = seal_raw(&key, b"", b"same plaintext").expect("seal");
        assert_ne!(first, second, "同明文两封必须不同 (nonce 随机)");
        assert_ne!(first.nonce(), second.nonce());
    }
}
