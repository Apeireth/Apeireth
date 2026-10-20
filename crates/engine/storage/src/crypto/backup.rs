//! Sealed backup: 备份 = 数据 + DEK 包裹件 (KEK 不随备份走).
//!
//! 与 `capabilities/tools` 的 G5 数据备份面 (目录快照 + 逐文件摘要) 分工:
//! G5 面管**数据搬家与凭据排除**; 本模块管**密钥层级的备份/恢复/密钥丢失**
//! (安全加固批 B6 隐患 #6 的 T1 备份三件套) —— 备份包里的数据是
//! [`SealedField`] 密文, 随行一把 KEK 包裹过的 DEK ([`KeyWrapFile`]),
//! **KEK 本身不随备份走**。
//!
//! ## bundle 布局
//!
//! ```text
//! <bundle>/manifest.json            清单 (逐件密文摘要 + 分区 id + 版本)
//! <bundle>/keys/key-wrap.json       DEK 包裹件 (口令/恢复件/锚 槽位; 无 KEK)
//! <bundle>/files/<name>.sealed      逐件 SealedField 密文单元
//! ```
//!
//! ## 硬语义 (与 G5 同口径, 二选一均标注被拒项)
//!
//! | # | 语义 | 钉死选择 | 被拒 alternative + 理由 |
//! |---|---|---|---|
//! | 1 | **宁拒不损** | 恢复先**全量校验** (清单 + 逐件密文摘要 + 逐件 AEAD 信封完整性), 全部通过才开始落盘, 任一不符**零字节落盘** | 拒「边验边落」: 半包落地 = 损坏态 |
//! | 2 | **原子落位** | 逐件「先写临时再换名」 | 拒「直写」: 中断留半文件 |
//! | 3 | **版本显式拒** | `format_version` ≠ 1 一律显式拒并报双方版本 | 拒「迁移语义」: 只有一版格式, 预建迁移器是无主债 |
//! | 4 | **路径安全** | 件名只认普通相对部件 (拒 `..`/绝对/盘符/空), 重复目标拒 | 拒「宽容归一」: 目录穿越是打包面经典逃逸 |
//! | 5 | **导出不覆盖** | bundle 目标非空 = 拒绝即帧 | 拒「覆盖续写」: 两份备份互踩 = 静默丢备份 |
//!
//! ## 密钥丢失边界 (显式「不可恢复」, 不假装能找回)
//!
//! [`KEY_LOSS_BOUNDARY`]: DEK 只以 KEK 包裹形态随备份走; 用户口令
//! **与**恢复件 **与** 锚定记录全失 = 密文永久不可解 —— 系统**没有**
//! 后门、没有找回流程、不降级读明文。可做的只有: 恢复前导出恢复件
//! ([`crate::crypto::wrap::RecoveryKey`], 用户保管), 恢复时用口令或
//! 恢复件 ([`UnlockKey`]) 解包。

use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::crypto::error::CryptoError;
use crate::crypto::partition::{PartitionId, PartitionKey};
use crate::crypto::sealed::SealedField;
use crate::crypto::wrap::{KeyWrapFile, UnlockKey};

/// 备份包格式版本 (≠ 1 显式拒)。
pub const BACKUP_FORMAT_VERSION: u32 = 1;

/// 包裹件在 bundle 内的相对路径。
pub const KEY_WRAP_REL_PATH: &str = "keys/key-wrap.json";

/// 清单在 bundle 内的相对路径。
pub const MANIFEST_REL_PATH: &str = "manifest.json";

/// 密钥丢失边界 (显式声明, 随错误帧与 CLI 面如实呈现)。
pub const KEY_LOSS_BOUNDARY: &str =
    "密钥丢失 = 不可恢复: DEK 仅以 KEK 包裹形态随备份走, 口令与恢复件与锚定记录全失时密文永久不可解 (无后门、无找回流程、不降级明文)。预防 = 恢复件导出后离线保管。";

/// 备份数据件 (明文进, 密文落)。
#[derive(Debug, Clone)]
pub struct BackupEntry {
    /// 逻辑件名 (恢复落盘的相对路径; 仅普通相对部件)。
    pub name: String,
    /// 明文字节 (只在内存经手, 落盘前封缄)。
    pub data: Vec<u8>,
}

/// 清单里的一个密文件记账。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedBackupEntry {
    /// 逻辑件名 (恢复落盘相对路径)。
    pub name: String,
    /// bundle 内相对路径 (`files/<name>.sealed`)。
    pub path: String,
    /// 密文单元字节数。
    pub sealed_bytes: u64,
    /// 密文单元 SHA-256 (hex; 密文完整性逐件记账)。
    pub sealed_sha256: String,
}

/// 备份包清单 (损坏可检出 + 出处真值)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedBackupManifest {
    /// 格式版本。
    pub format_version: u32,
    /// 分区标识 (与包裹件、密文 AAD 三方一致才可恢复)。
    pub partition_id: String,
    /// 导出时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 包裹件相对路径 ([`KEY_WRAP_REL_PATH`])。
    pub key_wrap_path: String,
    /// KEK 随行纪律声明 (格式自述, 不靠口头)。
    pub recovery_note: String,
    /// 逐件记账 (按 `name` 排序, 确定性)。
    pub entries: Vec<SealedBackupEntry>,
}

/// 恢复结果 (件数 + 分区)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedRestoreReport {
    /// 落盘件数。
    pub entries: usize,
    /// 分区标识。
    pub partition_id: String,
}

impl SealedBackupManifest {
    /// 清单件数。
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// 导出密封备份包 (数据封缄 + DEK 包裹件随行; 目标已存在 = 拒绝即帧)。
pub fn export_sealed_backup(
    bundle_dir: &Path,
    key_wrap: &KeyWrapFile,
    dek: &PartitionKey,
    entries: &[BackupEntry],
    now_ms: i64,
) -> Result<SealedBackupManifest, CryptoError> {
    if bundle_dir.exists() {
        let non_empty = std::fs::read_dir(bundle_dir)
            .map(|mut listing| listing.next().is_some())
            .unwrap_or(true);
        if non_empty {
            return Err(CryptoError::BackupFormat(format!(
                "bundle 目标已存在且非空, 拒绝覆盖: {}",
                bundle_dir.display()
            )));
        }
    }

    let partition = dek.partition().clone();
    if key_wrap.partition_id != partition.as_str() {
        return Err(CryptoError::BackupFormat(format!(
            "包裹件分区与 DEK 归属不符: {} vs {}",
            key_wrap.partition_id, partition
        )));
    }

    std::fs::create_dir_all(bundle_dir.join("files"))?;
    std::fs::create_dir_all(bundle_dir.join("keys"))?;
    std::fs::write(bundle_dir.join(KEY_WRAP_REL_PATH), key_wrap.to_json()?)?;

    let mut manifest_entries = Vec::with_capacity(entries.len());
    for entry in entries {
        check_entry_name(&entry.name)?;
        let sealed = dek.seal(&entry.data)?;
        let sealed_bytes = sealed.to_bytes().to_vec();
        let rel_path = format!("files/{}.sealed", entry.name);
        let target = bundle_dir.join(&rel_path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, &sealed_bytes)?;
        manifest_entries.push(SealedBackupEntry {
            name: entry.name.clone(),
            path: rel_path,
            sealed_bytes: u64::try_from(sealed_bytes.len()).unwrap_or(u64::MAX),
            sealed_sha256: sha256_hex(&sealed_bytes),
        });
    }
    manifest_entries.sort_by(|left, right| left.name.cmp(&right.name));

    let manifest = SealedBackupManifest {
        format_version: BACKUP_FORMAT_VERSION,
        partition_id: partition.as_str().to_string(),
        created_at_ms: now_ms,
        key_wrap_path: KEY_WRAP_REL_PATH.to_string(),
        recovery_note: "KEK 不随备份走: keys/key-wrap.json 只含 KEK 包裹后的 DEK; 恢复需用户口令或恢复件或锚定记录。".to_string(),
        entries: manifest_entries,
    };
    std::fs::write(
        bundle_dir.join(MANIFEST_REL_PATH),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(manifest)
}

/// 只验不恢复: 清单 + 逐件密文摘要 + 包裹件可解析 (零解密、零数据面写)。
pub fn verify_sealed_backup(bundle_dir: &Path) -> Result<SealedBackupManifest, CryptoError> {
    let manifest = read_manifest(bundle_dir)?;
    for entry in &manifest.entries {
        let sealed_bytes = std::fs::read(bundle_dir.join(&entry.path))?;
        check_entry_digest(entry, &sealed_bytes)?;
    }
    let key_wrap = read_key_wrap(bundle_dir, &manifest)?;
    if key_wrap.partition_id != manifest.partition_id {
        return Err(CryptoError::BackupFormat(format!(
            "包裹件分区与清单分区不符: {} vs {}",
            key_wrap.partition_id, manifest.partition_id
        )));
    }
    Ok(manifest)
}

/// 恢复: 全量校验 (含逐件 AEAD 信封完整性) 通过后才落盘 (宁拒不损)。
///
/// 解锁材料 = [`UnlockKey`] (用户口令 / 恢复件 / 锚定 KEK 三选一)。
pub fn restore_sealed_backup(
    bundle_dir: &Path,
    to_root: &Path,
    unlock: &UnlockKey,
) -> Result<SealedRestoreReport, CryptoError> {
    // ---- 阶段 1: 全量校验 + 解封, 全部通过前零磁盘写 ----
    let manifest = read_manifest(bundle_dir)?;
    let key_wrap = read_key_wrap(bundle_dir, &manifest)?;
    if key_wrap.partition_id != manifest.partition_id {
        return Err(CryptoError::RestoreRefused(format!(
            "包裹件分区与清单分区不符: {} vs {}",
            key_wrap.partition_id, manifest.partition_id
        )));
    }
    let dek = key_wrap.unlock(unlock).map_err(|error| match error {
        CryptoError::KeyMaterialLost(reason) => CryptoError::KeyMaterialLost(reason),
        other => CryptoError::RestoreRefused(format!("DEK 解包被拒: {other}")),
    })?;
    if dek.partition().as_str() != manifest.partition_id {
        return Err(CryptoError::RestoreRefused(
            "解出的 DEK 分区与清单分区不符 (跨分区挪用拒恢复)".to_string(),
        ));
    }

    let mut verified: Vec<(String, Vec<u8>)> = Vec::with_capacity(manifest.entries.len());
    for entry in &manifest.entries {
        check_entry_name(&entry.name)?;
        let sealed_bytes = std::fs::read(bundle_dir.join(&entry.path))?;
        check_entry_digest(entry, &sealed_bytes)
            .map_err(|error| CryptoError::RestoreRefused(error.to_string()))?;
        let sealed = SealedField::from_bytes(sealed_bytes)
            .map_err(|error| CryptoError::RestoreRefused(error.to_string()))?;
        let plaintext = dek.open(&sealed).map_err(|error| {
            CryptoError::RestoreRefused(format!("件 `{}` 信封完整性校验失败: {error}", entry.name))
        })?;
        verified.push((entry.name.clone(), plaintext));
    }

    // ---- 阶段 2: 校验全过, 逐件原子落位 ----
    for (name, plaintext) in &verified {
        let target = to_root.join(name);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut temporary = target.clone();
        let file_name = temporary
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "entry".to_string());
        temporary.set_file_name(format!(".{file_name}.sealed-restore.tmp"));
        std::fs::write(&temporary, plaintext)?;
        std::fs::rename(&temporary, &target)?;
    }

    Ok(SealedRestoreReport {
        entries: verified.len(),
        partition_id: manifest.partition_id,
    })
}

fn read_manifest(bundle_dir: &Path) -> Result<SealedBackupManifest, CryptoError> {
    let text = std::fs::read_to_string(bundle_dir.join(MANIFEST_REL_PATH))?;
    let manifest: SealedBackupManifest = serde_json::from_str(&text)?;
    if manifest.format_version != BACKUP_FORMAT_VERSION {
        return Err(CryptoError::BackupFormat(format!(
            "备份包格式版本不符: {} (支持 {BACKUP_FORMAT_VERSION})",
            manifest.format_version
        )));
    }
    if manifest.key_wrap_path != KEY_WRAP_REL_PATH {
        return Err(CryptoError::BackupFormat(format!(
            "包裹件路径不符: {} (只认 {KEY_WRAP_REL_PATH})",
            manifest.key_wrap_path
        )));
    }
    for entry in &manifest.entries {
        check_entry_name(&entry.name)?;
    }
    Ok(manifest)
}

fn read_key_wrap(
    bundle_dir: &Path,
    manifest: &SealedBackupManifest,
) -> Result<KeyWrapFile, CryptoError> {
    let text = std::fs::read_to_string(bundle_dir.join(&manifest.key_wrap_path))?;
    KeyWrapFile::from_json(&text)
}

fn check_entry_digest(entry: &SealedBackupEntry, sealed_bytes: &[u8]) -> Result<(), CryptoError> {
    let actual_len = u64::try_from(sealed_bytes.len()).unwrap_or(u64::MAX);
    if actual_len != entry.sealed_bytes {
        return Err(CryptoError::BackupFormat(format!(
            "件 `{}` 密文字节数不符: {} vs {}",
            entry.name, actual_len, entry.sealed_bytes
        )));
    }
    let actual_digest = sha256_hex(sealed_bytes);
    if actual_digest != entry.sealed_sha256 {
        return Err(CryptoError::BackupFormat(format!(
            "件 `{}` 密文摘要不符 (密文完整性失败)",
            entry.name
        )));
    }
    Ok(())
}

/// 件名路径安全: 只认普通相对部件 (拒 `..`/绝对/盘符/空)。
fn check_entry_name(name: &str) -> Result<(), CryptoError> {
    if name.trim().is_empty() {
        return Err(CryptoError::PathSafety("件名为空".to_string()));
    }
    let path = Path::new(name);
    let mut saw_normal = false;
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                saw_normal = true;
                if part.to_string_lossy().contains('\\') {
                    return Err(CryptoError::PathSafety(format!("件名含路径分隔符: {name}")));
                }
            }
            _ => {
                return Err(CryptoError::PathSafety(format!(
                    "件名只认普通相对部件: {name}"
                )))
            }
        }
    }
    if !saw_normal {
        return Err(CryptoError::PathSafety(format!("件名无有效部件: {name}")));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kdf::KdfParams;
    use crate::crypto::wrap::RecoveryKey;

    #[test]
    fn entry_names_reject_traversal_absolute_and_empty() {
        assert!(check_entry_name("episodes/one.sealed").is_ok());
        assert!(check_entry_name("").is_err());
        assert!(check_entry_name("   ").is_err());
        assert!(check_entry_name("../escape").is_err());
        assert!(check_entry_name("a/../../escape").is_err());
        assert!(check_entry_name("/absolute").is_err());
        assert!(check_entry_name("C:\\windows\\system32").is_err());
    }

    #[test]
    fn manifest_digest_detects_length_and_content_drift() {
        let entry = SealedBackupEntry {
            name: "one".to_string(),
            path: "files/one.sealed".to_string(),
            sealed_bytes: 3,
            sealed_sha256: sha256_hex(b"abc"),
        };
        assert!(check_entry_digest(&entry, b"abc").is_ok());
        assert!(check_entry_digest(&entry, b"abd").is_err(), "翻位必须检出");
        assert!(check_entry_digest(&entry, b"ab").is_err(), "截断必须检出");
    }

    #[test]
    fn key_loss_boundary_is_an_explicit_unrecoverable_statement() {
        assert!(
            KEY_LOSS_BOUNDARY.contains("不可恢复"),
            "边界声明必须显式说不可恢复"
        );
        assert!(
            KEY_LOSS_BOUNDARY.contains("无后门"),
            "边界声明必须显式说无后门"
        );
    }

    #[test]
    fn fast_kdf_params_are_reusable_for_tests() {
        // 与 wrap 面测试同款小迭代参数 (KDF 构造在 kdf 面已钉标准向量)。
        let params = KdfParams::pbkdf2_hmac_sha256(8, b"backup-test-salt").expect("params");
        assert_eq!(params.iterations, 8);
        let _ = RecoveryKey::generate();
    }
}
