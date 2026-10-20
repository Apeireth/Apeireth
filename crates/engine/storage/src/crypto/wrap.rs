//! Key wrap file: DEK 信封 (KEK 包裹的分区 DEK, 槽位制).
//!
//! **命名承诺**: 此处的「包裹 (wrap)」是**加密包裹**, 与
//! `foundation/core::stored_doc` 的**格式信封**职责不同, 刻意不共用
//! 「信封」类型名。
//!
//! 一个 [`KeyWrapFile`] = 一个分区的 DEK 被一个或多个 KEK 槽位包裹:
//!
//! | 槽位 | KEK 来源 | 恢复材料 |
//! |---|---|---|
//! | `passphrase` | 用户口令经 PBKDF2-HMAC-SHA-256 派生 ([`crate::crypto::kdf`]) | 用户口令 |
//! | `recovery` | 恢复件 (随机 32 字节, 导出给用户保管) | 恢复件 |
//! | `anchored` | [`KekAnchor`] 锚定 (平台凭据库/桌面侧车 keychain) | 锚可访问者 |
//!
//! **KEK 不随备份走**: 本文件只含 KEK 包裹后的 DEK; 槽位里没有 KEK
//! 本身 (口令槽位只有盐+迭代参数, 锚槽位只有锚 id)。恢复 = 用户口令
//! **或**恢复件 **或** 锚 —— 三者全失 = 显式不可恢复
//! ([`crate::crypto::backup::KEY_LOSS_BOUNDARY`]), 无后门。

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::crypto::anchor::KekAnchor;
use crate::crypto::error::CryptoError;
use crate::crypto::kdf::{derive_kek_from_passphrase, KdfParams};
use crate::crypto::partition::{PartitionId, PartitionKey};
use crate::crypto::sealed::{open_raw, seal_raw, SealedField};

/// 包裹件格式版本。
pub const KEY_WRAP_FORMAT_VERSION: u32 = 1;

/// 包裹 AAD 域分隔串 (与字段密封分域, 防跨用途挪用)。
const KEY_WRAP_AAD_CONTEXT: &[u8] = b"apeireth.keywrap.v1";

/// 恢复件: 裸 KEK 等价物 (随机 32 字节, 用户保管)。
///
/// **敏感面**: 恢复件落在手 = 该分区 DEK 可解包; 导出形态是 hex 文本,
/// 由调用方决定怎么给用户 (文件/纸质)。`Debug` 永不铺字节。
#[derive(Clone)]
pub struct RecoveryKey(Zeroizing<[u8; 32]>);

impl RecoveryKey {
    /// 随机生成恢复件 (OsRng)。
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut bytes = Zeroizing::new([0u8; 32]);
        rand::rngs::OsRng.fill_bytes(&mut *bytes);
        Self(bytes)
    }

    /// 从字节组装 (恢复流程读回用户保管的恢复件)。
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// 从 hex 文本组装 (用户保管的恢复件形态)。
    pub fn from_hex(text: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(text.trim())
            .map_err(|error| CryptoError::Encoding(format!("恢复件 hex 畸形: {error}")))?;
        let array: [u8; 32] = bytes.try_into().map_err(|bytes: Vec<u8>| {
            CryptoError::Encoding(format!("恢复件长度不符: {} (需 32 字节)", bytes.len()))
        })?;
        Ok(Self::from_bytes(array))
    }

    /// hex 导出形态 (**恢复件导出接口**: 给用户保管的文本)。
    pub fn to_hex(&self) -> String {
        hex::encode(*self.0)
    }

    /// 恢复件原始字节 (仅供包裹 DEK)。
    pub fn expose(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for RecoveryKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RecoveryKey([redacted])")
    }
}

/// KEK 来源 (槽位标签; 写进 AAD, 换槽挪用拒解)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KekSource {
    /// 用户口令经 KDF 派生 KEK (参数随槽落盘)。
    Passphrase {
        /// PBKDF2-HMAC-SHA-256 参数 (盐 + 迭代数; 无口令材料)。
        kdf: KdfParams,
    },
    /// 恢复件包裹 (KEK = 恢复件本身, 不落盘)。
    Recovery,
    /// 锚定包裹 (KEK 在 [`KekAnchor`] 锚里, 此处只记锚 id)。
    Anchored {
        /// 锚定身份 (不含密钥材料)。
        anchor_id: String,
    },
}

/// 一个 KEK 槽位: KEK 来源 + 被包裹的 DEK 密文单元。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyWrapSlot {
    /// KEK 来源。
    pub source: KekSource,
    /// 被包裹 DEK 的密文单元 ([`SealedField`] hex 形态)。
    pub sealed_dek_hex: String,
}

impl KeyWrapSlot {
    fn aad_context(&self, partition: &PartitionId) -> Vec<u8> {
        let mut aad = Vec::new();
        aad.extend_from_slice(KEY_WRAP_AAD_CONTEXT);
        match &self.source {
            KekSource::Passphrase { .. } => aad.extend_from_slice(b"|passphrase|"),
            KekSource::Recovery => aad.extend_from_slice(b"|recovery|"),
            KekSource::Anchored { anchor_id } => {
                aad.extend_from_slice(b"|anchored|");
                aad.extend_from_slice(anchor_id.as_bytes());
                aad.push(b'|');
            }
        }
        aad.extend_from_slice(partition.as_str().as_bytes());
        aad
    }

    fn sealed_dek(&self) -> Result<SealedField, CryptoError> {
        SealedField::from_hex(&self.sealed_dek_hex)
            .map_err(|error| CryptoError::Encoding(format!("包裹槽位密文单元畸形: {error}")))
    }
}

/// 解锁材料 (恢复流程入口: 口令 / 恢复件 / 锚定解析出的 KEK)。
#[derive(Clone)]
pub enum UnlockKey {
    /// 用户口令 (经槽位 KDF 参数派生 KEK)。
    Passphrase(String),
    /// 恢复件 (裸 KEK 等价物)。
    Recovery(RecoveryKey),
    /// 装配面锚定解析出的 KEK (`anchor_id` 用于选槽)。
    Anchored {
        /// 锚定身份 (选槽用)。
        anchor_id: String,
        /// 锚定 KEK (32 字节, [`Zeroizing`] 持钥)。
        kek: Zeroizing<[u8; 32]>,
    },
}

impl std::fmt::Debug for UnlockKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Debug 永不铺口令/恢复件/KEK 字节 (脱敏红线)。
        match self {
            Self::Passphrase(_) => formatter.write_str("UnlockKey::Passphrase([redacted])"),
            Self::Recovery(_) => formatter.write_str("UnlockKey::Recovery([redacted])"),
            Self::Anchored { anchor_id, .. } => formatter
                .debug_struct("UnlockKey::Anchored")
                .field("anchor_id", anchor_id)
                .field("kek", &"[redacted]")
                .finish(),
        }
    }
}

/// 分区 DEK 包裹件 (随备份走的「DEK 信封」; KEK 不随备份走)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyWrapFile {
    /// 包裹件格式版本 (≠ 1 显式拒)。
    pub format_version: u32,
    /// 分区标识 (AAD 绑定; 篡改 = 拒解)。
    pub partition_id: String,
    /// 生成时刻 (epoch 毫秒, 出处信息)。
    pub created_at_ms: i64,
    /// KEK 槽位 (一个分区可多槽并存: 口令 + 恢复件 + 锚)。
    pub slots: Vec<KeyWrapSlot>,
}

impl KeyWrapFile {
    /// 生成分区 DEK 并按「口令槽 + 恢复件槽」双槽包裹 (**恢复件导出接口**随行)。
    pub fn seal_new(
        partition: &PartitionId,
        dek: &PartitionKey,
        passphrase: &[u8],
        recovery: &RecoveryKey,
        kdf: KdfParams,
        now_ms: i64,
    ) -> Result<Self, CryptoError> {
        let mut file = Self {
            format_version: KEY_WRAP_FORMAT_VERSION,
            partition_id: partition.as_str().to_string(),
            created_at_ms: now_ms,
            slots: Vec::new(),
        };
        file.add_passphrase_slot(dek, passphrase, kdf)?;
        file.add_recovery_slot(dek, recovery)?;
        Ok(file)
    }

    /// 校验包裹件自洽 (格式版本 + 分区 id 对 DEK 归属)。
    fn check(&self, partition: &PartitionId) -> Result<(), CryptoError> {
        if self.format_version != KEY_WRAP_FORMAT_VERSION {
            return Err(CryptoError::WrapRefused(format!(
                "包裹件格式版本不符: {} (支持 {KEY_WRAP_FORMAT_VERSION})",
                self.format_version
            )));
        }
        if self.partition_id != partition.as_str() {
            return Err(CryptoError::WrapRefused(format!(
                "包裹件分区 id 与 DEK 归属不符: {} vs {}",
                self.partition_id, partition
            )));
        }
        Ok(())
    }

    fn wrap_dek(
        &self,
        dek: &PartitionKey,
        kek: &[u8; 32],
        source: KekSource,
    ) -> Result<KeyWrapSlot, CryptoError> {
        let mut slot = KeyWrapSlot {
            source,
            sealed_dek_hex: String::new(),
        };
        let partition = PartitionId::new(self.partition_id.clone())
            .map_err(|error| CryptoError::WrapRefused(error.to_string()))?;
        let sealed = seal_raw(kek, &slot.aad_context(&partition), dek.expose_dek())?;
        slot.sealed_dek_hex = sealed.to_hex();
        Ok(slot)
    }

    /// 加口令槽 (口令经槽位 KDF 参数派生 KEK)。
    pub fn add_passphrase_slot(
        &mut self,
        dek: &PartitionKey,
        passphrase: &[u8],
        kdf: KdfParams,
    ) -> Result<(), CryptoError> {
        let partition = PartitionId::new(self.partition_id.clone())
            .map_err(|error| CryptoError::WrapRefused(error.to_string()))?;
        self.check(&partition)?;
        let kek = derive_kek_from_passphrase(passphrase, &kdf)?;
        let slot = self.wrap_dek(dek, &kek, KekSource::Passphrase { kdf })?;
        self.slots
            .retain(|slot| !matches!(slot.source, KekSource::Passphrase { .. }));
        self.slots.push(slot);
        Ok(())
    }

    /// 加恢复件槽 (KEK = 恢复件本身; **恢复件不落包裹件**)。
    pub fn add_recovery_slot(
        &mut self,
        dek: &PartitionKey,
        recovery: &RecoveryKey,
    ) -> Result<(), CryptoError> {
        let partition = PartitionId::new(self.partition_id.clone())
            .map_err(|error| CryptoError::WrapRefused(error.to_string()))?;
        self.check(&partition)?;
        let slot = self.wrap_dek(dek, recovery.expose(), KekSource::Recovery)?;
        self.slots
            .retain(|slot| !matches!(slot.source, KekSource::Recovery));
        self.slots.push(slot);
        Ok(())
    }

    /// 加锚定槽 (KEK 锚进 [`KekAnchor`], 包裹件只记锚 id)。
    pub fn add_anchor_slot(
        &mut self,
        dek: &PartitionKey,
        anchor: &dyn KekAnchor,
    ) -> Result<(), CryptoError> {
        let partition = PartitionId::new(self.partition_id.clone())
            .map_err(|error| CryptoError::WrapRefused(error.to_string()))?;
        self.check(&partition)?;
        use rand::RngCore;
        let mut kek = Zeroizing::new([0u8; 32]);
        rand::rngs::OsRng.fill_bytes(&mut *kek);
        anchor.store_kek(&partition, &kek)?;
        let slot = self.wrap_dek(
            dek,
            &kek,
            KekSource::Anchored {
                anchor_id: anchor.anchor_id().to_string(),
            },
        )?;
        self.slots.retain(|slot| {
            !matches!(&slot.source, KekSource::Anchored { anchor_id } if anchor_id == anchor.anchor_id())
        });
        self.slots.push(slot);
        Ok(())
    }

    /// 按解锁材料解出分区 DEK (口令 / 恢复件 / 锚定 KEK 三选一)。
    pub fn unlock(&self, unlock: &UnlockKey) -> Result<PartitionKey, CryptoError> {
        if self.format_version != KEY_WRAP_FORMAT_VERSION {
            return Err(CryptoError::UnwrapRefused {
                reason: "包裹件格式版本不符",
            });
        }
        let partition = PartitionId::new(self.partition_id.clone()).map_err(|_| {
            CryptoError::UnwrapRefused {
                reason: "包裹件分区 id 空",
            }
        })?;
        match unlock {
            UnlockKey::Passphrase(passphrase) => {
                let slots: Vec<&KeyWrapSlot> = self
                    .slots
                    .iter()
                    .filter(|slot| matches!(slot.source, KekSource::Passphrase { .. }))
                    .collect();
                if slots.is_empty() {
                    return Err(CryptoError::KeyMaterialLost(
                        "包裹件无口令槽位: 口令无法解此包裹件 (口令遗忘且恢复件缺失 = 不可恢复)"
                            .to_string(),
                    ));
                }
                for slot in slots {
                    let KekSource::Passphrase { kdf } = &slot.source else {
                        continue;
                    };
                    if let Ok(kek) = derive_kek_from_passphrase(passphrase.as_bytes(), kdf) {
                        if let Ok(dek) = self.open_slot(slot, &kek, &partition) {
                            return Ok(dek);
                        }
                    }
                }
                Err(CryptoError::UnwrapRefused {
                    reason: "口令错误或包裹槽位被篡改",
                })
            }
            UnlockKey::Recovery(recovery) => {
                let slot = self
                    .slots
                    .iter()
                    .find(|slot| matches!(slot.source, KekSource::Recovery))
                    .ok_or_else(|| {
                        CryptoError::KeyMaterialLost(
                            "包裹件无恢复件槽位: 恢复件无法解此包裹件 (恢复件缺失且口令遗忘 = 不可恢复)"
                                .to_string(),
                        )
                    })?;
                self.open_slot(slot, recovery.expose(), &partition)
                    .map_err(|_| CryptoError::UnwrapRefused {
                        reason: "恢复件错误或包裹槽位被篡改",
                    })
            }
            UnlockKey::Anchored { anchor_id, kek } => {
                let slot = self
                    .slots
                    .iter()
                    .find(|slot| matches!(&slot.source, KekSource::Anchored { anchor_id: id } if id == anchor_id))
                    .ok_or_else(|| {
                        CryptoError::KeyMaterialLost(format!(
                            "包裹件无锚 `{anchor_id}` 的槽位 (锚槽位缺失 = 该锚不可解)"
                        ))
                    })?;
                self.open_slot(slot, kek, &partition)
                    .map_err(|_| CryptoError::UnwrapRefused {
                        reason: "锚定 KEK 不符或包裹槽位被篡改",
                    })
            }
        }
    }

    /// 经 [`KekAnchor`] 解出分区 DEK (锚定槽位路径)。
    pub fn unlock_with_anchor(&self, anchor: &dyn KekAnchor) -> Result<PartitionKey, CryptoError> {
        let partition = PartitionId::new(self.partition_id.clone()).map_err(|_| {
            CryptoError::UnwrapRefused {
                reason: "包裹件分区 id 空",
            }
        })?;
        let kek = anchor.load_kek(&partition)?.ok_or_else(|| {
            CryptoError::KeyMaterialLost(format!(
                "锚 `{}` 无该分区 KEK 记录 (锚记录缺失 = 不可恢复)",
                anchor.anchor_id()
            ))
        })?;
        self.unlock(&UnlockKey::Anchored {
            anchor_id: anchor.anchor_id().to_string(),
            kek,
        })
    }

    fn open_slot(
        &self,
        slot: &KeyWrapSlot,
        kek: &[u8; 32],
        partition: &PartitionId,
    ) -> Result<PartitionKey, CryptoError> {
        let sealed = slot.sealed_dek()?;
        let plaintext = open_raw(kek, &slot.aad_context(partition), &sealed, "包裹槽位拒解")?;
        let dek: [u8; 32] = plaintext.try_into().map_err(|bytes: Vec<u8>| {
            CryptoError::Encoding(format!("包裹槽位 DEK 长度不符: {}", bytes.len()))
        })?;
        Ok(PartitionKey::from_bytes(partition.clone(), dek))
    }

    /// JSON 形态 (落盘/随备份走)。
    pub fn to_json(&self) -> Result<String, CryptoError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// 从 JSON 读回 (畸形显式拒, 不回退默认)。
    pub fn from_json(text: &str) -> Result<Self, CryptoError> {
        Ok(serde_json::from_str(text)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kdf::KdfParams;

    fn fast_kdf() -> KdfParams {
        KdfParams::pbkdf2_hmac_sha256(8, b"unit-test-salt").expect("params")
    }

    fn sample() -> (PartitionId, PartitionKey) {
        let partition = PartitionId::new("user-a").expect("id");
        let key = PartitionKey::generate(partition.clone());
        (partition, key)
    }

    #[test]
    fn wrap_file_round_trips_with_passphrase_and_recovery() {
        let (partition, key) = sample();
        let recovery = RecoveryKey::generate();
        let file = KeyWrapFile::seal_new(
            &partition,
            &key,
            b"correct horse battery staple",
            &recovery,
            fast_kdf(),
            1_729_500_000_000,
        )
        .expect("wrap");

        let via_passphrase = file
            .unlock(&UnlockKey::Passphrase(
                "correct horse battery staple".to_string(),
            ))
            .expect("passphrase unlock");
        assert_eq!(via_passphrase.expose_dek(), key.expose_dek());

        let via_recovery = file
            .unlock(&UnlockKey::Recovery(recovery))
            .expect("recovery unlock");
        assert_eq!(via_recovery.expose_dek(), key.expose_dek());
        assert_eq!(via_recovery.partition(), key.partition());

        let json = file.to_json().expect("json");
        let back = KeyWrapFile::from_json(&json).expect("parse");
        assert_eq!(back, file);
    }

    #[test]
    fn wrong_passphrase_and_wrong_recovery_are_refused() {
        let (partition, key) = sample();
        let recovery = RecoveryKey::generate();
        let file =
            KeyWrapFile::seal_new(&partition, &key, b"pw", &recovery, fast_kdf(), 0).expect("wrap");

        let error = file
            .unlock(&UnlockKey::Passphrase("nope".to_string()))
            .expect_err("错口令必须拒");
        assert!(matches!(error, CryptoError::UnwrapRefused { .. }));

        let error = file
            .unlock(&UnlockKey::Recovery(RecoveryKey::generate()))
            .expect_err("错恢复件必须拒");
        assert!(matches!(error, CryptoError::UnwrapRefused { .. }));
    }

    #[test]
    fn key_material_lost_is_the_explicit_boundary_without_slots() {
        let (partition, key) = sample();
        let recovery = RecoveryKey::generate();
        let mut file =
            KeyWrapFile::seal_new(&partition, &key, b"pw", &recovery, fast_kdf(), 0).expect("wrap");
        file.slots
            .retain(|slot| matches!(slot.source, KekSource::Recovery));

        let error = file
            .unlock(&UnlockKey::Passphrase("pw".to_string()))
            .expect_err("无口令槽 = 显式不可恢复");
        assert!(matches!(error, CryptoError::KeyMaterialLost(_)));
    }

    #[test]
    fn wrap_json_never_contains_the_kek_material() {
        let (partition, key) = sample();
        let recovery = RecoveryKey::generate();
        let file =
            KeyWrapFile::seal_new(&partition, &key, b"pw", &recovery, fast_kdf(), 0).expect("wrap");
        let json = file.to_json().expect("json");

        assert!(!json.contains(&recovery.to_hex()), "包裹件不得含恢复件");
        assert!(!json.contains("correct horse"), "包裹件不得含口令");
        let kek = crate::crypto::kdf::derive_kek_from_passphrase(b"pw", &fast_kdf()).expect("kek");
        assert!(!json.contains(&hex::encode(*kek)), "包裹件不得含派生 KEK");
    }

    #[test]
    fn tampered_partition_id_or_ciphertext_is_refused() {
        let (partition, key) = sample();
        let recovery = RecoveryKey::generate();
        let file =
            KeyWrapFile::seal_new(&partition, &key, b"pw", &recovery, fast_kdf(), 0).expect("wrap");

        let mut moved = file.clone();
        moved.partition_id = "user-b".to_string();
        let error = moved
            .unlock(&UnlockKey::Recovery(recovery.clone()))
            .expect_err("挪分区必须拒");
        assert!(matches!(error, CryptoError::UnwrapRefused { .. }));

        let mut tampered = file.clone();
        let slot = tampered.slots.last_mut().expect("slot");
        let mut bytes = hex::decode(&slot.sealed_dek_hex).expect("hex");
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        slot.sealed_dek_hex = hex::encode(bytes);
        let error = tampered
            .unlock(&UnlockKey::Recovery(recovery))
            .expect_err("篡改密文必须拒");
        assert!(matches!(error, CryptoError::UnwrapRefused { .. }));
    }

    #[test]
    fn recovery_key_hex_round_trips() {
        let recovery = RecoveryKey::generate();
        let back = RecoveryKey::from_hex(&recovery.to_hex()).expect("hex");
        assert_eq!(back.expose(), recovery.expose());
        assert!(RecoveryKey::from_hex("deadbeef").is_err(), "长度不符必须拒");
    }

    /// 测试锚: 内存版 [`KekAnchor`] (装配面真锚归口平台凭据库/桌面侧车)。
    struct MemoryAnchor {
        id: String,
        kek: std::sync::Mutex<Option<Zeroizing<[u8; 32]>>>,
    }

    impl KekAnchor for MemoryAnchor {
        fn anchor_id(&self) -> &str {
            &self.id
        }

        fn load_kek(
            &self,
            _partition: &PartitionId,
        ) -> Result<Option<Zeroizing<[u8; 32]>>, CryptoError> {
            Ok(self.kek.lock().expect("lock").clone())
        }

        fn store_kek(&self, _partition: &PartitionId, kek: &[u8; 32]) -> Result<(), CryptoError> {
            *self.kek.lock().expect("lock") = Some(Zeroizing::new(*kek));
            Ok(())
        }
    }

    #[test]
    fn anchor_slot_wraps_through_the_kek_anchor_trait() {
        let (partition, key) = sample();
        let recovery = RecoveryKey::generate();
        let mut file =
            KeyWrapFile::seal_new(&partition, &key, b"pw", &recovery, fast_kdf(), 0).expect("wrap");

        let anchor = MemoryAnchor {
            id: "test-anchor".to_string(),
            kek: std::sync::Mutex::new(None),
        };
        file.add_anchor_slot(&key, &anchor).expect("anchor slot");

        let unlocked = file.unlock_with_anchor(&anchor).expect("anchor unlock");
        assert_eq!(unlocked.expose_dek(), key.expose_dek());

        // 锚无记录 = 显式不可恢复 (如实报缺, 不编值)。
        let empty = MemoryAnchor {
            id: "empty-anchor".to_string(),
            kek: std::sync::Mutex::new(None),
        };
        let error = file.unlock_with_anchor(&empty).expect_err("空锚必须拒");
        assert!(matches!(error, CryptoError::KeyMaterialLost(_)));
    }
}
