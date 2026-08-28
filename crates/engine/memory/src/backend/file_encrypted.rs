//! P-arch (2026-08-27) + v2.0.0-rc.1 RC-10: EncryptedFileBackend — AES-256-GCM 加密 File backend.
//!
//! **位置**: impl 在 `apeireth-memory` (engine), trait 在 `apeireth-plugin::MemoryBackend` (foundation).
//! 0 装 PASS: alpha FileBackend 明文 fallback (per v2.0.0-rc-roadmap.md §3 RC-10:
//! "0 装: 明文 fallback 已存在 (FileBackend), EncryptedFileBackend 是 opt-in").
//!
//! **加密设计** (per v2.0.0-rc-roadmap.md §3 RC-10):
//! - AES-256-GCM (AEAD) 加密每条 record 的 JSON
//! - master.key 走 `KeyringSelector` (per RC-9 alpha 已就位), 0 装走 hardcoded dev key
//! - IV per-record (12 bytes, `rand::thread_rng().gen()`)
//! - v2 AAD 绑定 service/type、record index、key commitment 和完整 record 长度
//! - v2 每条 record 存 `[magic || version || index || key_commitment || iv || ciphertext || tag]`
//! - v1 无 header 的 `[iv || ciphertext || tag]` 仍可读取，写入永远使用 v2
//!
//! **3 阶审查** (O-6 锚 #9):
//! 1. 总体: 与 RC-1/3/4/8 同样模式 (alpha 写真完整, 0 装诚实标注)
//! 2. 系统: 复用 aes-gcm (workspace dep) + rand (workspace dep), 0 引入新 dep
//! 3. 架构: alpha FileBackend 0 改, EncryptedFileBackend 是 opt-in wrapper (record 格式不同)
//!
//! **0 装诚实**:
//! - alpha FileBackend (明文) 仍存在, 0 改
//! - EncryptedFileBackend 是 opt-in: 显式 `EncryptedFileBackend::new(master_key)` 启用
//! - master.key **当前** 0 装: dev key (zeroed bytes 32). rc 阶段接 `KeyringSelector` 拿真 key
//! - IV 每条独立 (`rand::thread_rng().gen()`), 防 replay (相同 plaintext 不同 IV)
//! - AAD 防 tampering: 篡改 record_id / type / header / framing / record 顺序都会 fail AEAD tag verification
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline.
//!
//! **v1 compat**: 100+ consumer 0 破 (新增 module, 0 改 FileBackend).

use std::path::PathBuf;
use std::sync::Arc;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;

use apeireth_core::kernel::memory::Episode;
use apeireth_core::kernel::HistoryEntry;

use crate::append_only::HistoryStream;
use crate::episode::EpisodeStore;
use crate::MemoryError;
use crate::MemoryResult;

use super::{BackendKind, MemoryBackend};

/// AES-256-GCM 加密 File backend wrapper (RC-10)
///
/// **设计**:
/// - 内部不持 file system (0 重叠 FileBackend 已有 lock)
/// - 持 master key (32 bytes) + cipher 实例 (Aes256Gcm, 0 重复构造)
/// - 持 `dir: PathBuf` (跟 FileBackend 同样 root 目录结构)
/// - **0 装 PASS**: master key dev 0 (zeroed 32 bytes), 真生产走 `KeyringSelector` 拿 OS keyring
pub struct EncryptedFileBackend {
    /// AES-256-GCM cipher (32 bytes key)
    cipher: Aes256Gcm,
    /// Key material used only to derive opaque record identity commitments.
    key: [u8; Self::KEY_LEN],
    /// Root directory (同 alpha FileBackend: `<root>/episodes.jsonl` + `<root>/streams/*.jsonl`)
    dir: PathBuf,
    /// Service name (bound into the v2 AAD envelope)
    service: String,
}

impl EncryptedFileBackend {
    /// 32 bytes master key (256-bit AES-GCM)
    pub const KEY_LEN: usize = 32;

    /// 12 bytes IV (96-bit AES-GCM standard nonce)
    pub const IV_LEN: usize = 12;

    /// 16 bytes AEAD tag (GCM tag length)
    pub const TAG_LEN: usize = 16;

    /// v2 record magic. A missing magic means the legacy v1 record format.
    const MAGIC: [u8; 4] = *b"APX2";

    /// Version byte for AAD-bound records.
    const FORMAT_VERSION: u8 = 2;

    /// v2 header length: magic + version + index + opaque record-id commitment.
    const HEADER_LEN: usize = 4 + 1 + 8 + 32;

    /// 创 EncryptedFileBackend with explicit 32-byte master key.
    /// **0 装 PASS**: master key 0 真接 OS keyring (alpha 0 装). RC-10 阶段接 `KeyringSelector`.
    pub fn new(
        root: impl Into<PathBuf>,
        master_key: &[u8; Self::KEY_LEN],
        service: impl Into<String>,
    ) -> Self {
        let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(master_key));
        Self {
            cipher,
            key: *master_key,
            dir: root.into(),
            service: service.into(),
        }
    }

    /// 创 EncryptedFileBackend with **dev 0 装** master key (zeroed 32 bytes).
    /// **0 装诚实**: 显式 `for_dev_only` 名字, 不假装"我有真 master key".
    /// 真生产用 `new(root, &KeyringSelector::select(...).get("master_key")?, service)`.
    pub fn for_dev_only(root: impl Into<PathBuf>, service: impl Into<String>) -> Self {
        Self::new(root, &[0u8; Self::KEY_LEN], service)
    }

    /// Derive an opaque, keyed identity commitment without putting the logical id on disk.
    fn record_id_commitment(&self, record_id: &str) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(b"apeireth/encrypted-file/record-id/v2");
        hasher.update(self.key);
        let record_id_len = u64::try_from(record_id.len()).expect("record id length fits in u64");
        hasher.update(record_id_len.to_be_bytes());
        hasher.update(record_id.as_bytes());
        hasher.finalize().into()
    }

    /// Build an unambiguous AAD envelope for a v2 record.
    fn aad(
        &self,
        record_type: &str,
        record_index: u64,
        record_id_commitment: &[u8; 32],
        sealed_len: usize,
    ) -> Result<Vec<u8>, MemoryError> {
        let service_len = u32::try_from(self.service.len())
            .map_err(|_| MemoryError::Invalid("service name is too long".to_string()))?;
        let record_type_len = u32::try_from(record_type.len())
            .map_err(|_| MemoryError::Invalid("record type is too long".to_string()))?;
        let sealed_len = u32::try_from(sealed_len)
            .map_err(|_| MemoryError::Invalid("sealed record is too large".to_string()))?;

        let mut aad =
            Vec::with_capacity(1 + 4 + self.service.len() + 4 + record_type.len() + 8 + 32 + 4);
        aad.push(Self::FORMAT_VERSION);
        aad.extend_from_slice(&service_len.to_be_bytes());
        aad.extend_from_slice(self.service.as_bytes());
        aad.extend_from_slice(&record_type_len.to_be_bytes());
        aad.extend_from_slice(record_type.as_bytes());
        aad.extend_from_slice(&record_index.to_be_bytes());
        aad.extend_from_slice(record_id_commitment);
        aad.extend_from_slice(&sealed_len.to_be_bytes());
        Ok(aad)
    }

    /// 加密 + AEAD 标签: 返 v2 `[header || iv (12) || ciphertext || tag (16)]`.
    fn seal(
        &self,
        plaintext: &[u8],
        record_type: &str,
        record_id: &str,
        record_index: u64,
    ) -> Result<Vec<u8>, MemoryError> {
        // IV per-record (12 bytes random)
        let mut iv_bytes = [0u8; Self::IV_LEN];
        rand::thread_rng().fill_bytes(&mut iv_bytes);
        let nonce = Nonce::from_slice(&iv_bytes);
        let record_id_commitment = self.record_id_commitment(record_id);
        let sealed_len = Self::HEADER_LEN + Self::IV_LEN + plaintext.len() + Self::TAG_LEN;
        let aad = self.aad(record_type, record_index, &record_id_commitment, sealed_len)?;
        // AEAD seal (encrypt + tag)
        let mut ciphertext = self
            .cipher
            .encrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|e| MemoryError::Invalid(format!("AES-GCM seal failed: {e}")))?;
        // 拼装: header || iv (12) || ciphertext_with_tag
        let mut out = Vec::with_capacity(sealed_len);
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::FORMAT_VERSION);
        out.extend_from_slice(&record_index.to_be_bytes());
        out.extend_from_slice(&record_id_commitment);
        out.extend_from_slice(&iv_bytes);
        out.append(&mut ciphertext);
        Ok(out)
    }

    /// Open a v2 record at its expected physical index.
    fn open_v2_record(
        &self,
        sealed: &[u8],
        record_type: &str,
        expected_index: u64,
    ) -> Result<Vec<u8>, MemoryError> {
        if sealed.len() < Self::HEADER_LEN + Self::IV_LEN + Self::TAG_LEN {
            return Err(MemoryError::Invalid(format!(
                "sealed record too short: {} bytes, min {}",
                sealed.len(),
                Self::HEADER_LEN + Self::IV_LEN + Self::TAG_LEN
            )));
        }
        if sealed[..4] != Self::MAGIC || sealed[4] != Self::FORMAT_VERSION {
            return Err(MemoryError::Invalid(
                "unsupported encrypted record format".to_string(),
            ));
        }
        let header_index = u64::from_be_bytes(
            sealed[5..13]
                .try_into()
                .map_err(|_| MemoryError::Invalid("invalid record index".to_string()))?,
        );
        if header_index != expected_index {
            return Err(MemoryError::Invalid(format!(
                "encrypted record index mismatch: header {header_index}, expected {expected_index}"
            )));
        }
        let record_id_commitment: [u8; 32] = sealed[13..45]
            .try_into()
            .map_err(|_| MemoryError::Invalid("invalid record identity header".to_string()))?;
        let (iv_bytes, ciphertext) = sealed[Self::HEADER_LEN..].split_at(Self::IV_LEN);
        let nonce = Nonce::from_slice(iv_bytes);
        let aad = self.aad(
            record_type,
            expected_index,
            &record_id_commitment,
            sealed.len(),
        )?;
        self.cipher
            .decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|e| {
                MemoryError::Invalid(format!("AES-GCM open failed (AAD mismatch or tamper): {e}"))
            })
    }

    /// 解密 legacy v1 record. v1 authenticated service and type only.
    fn open_legacy_record(&self, sealed: &[u8], record_type: &str) -> Result<Vec<u8>, MemoryError> {
        if sealed.len() < Self::IV_LEN + Self::TAG_LEN {
            return Err(MemoryError::Invalid(format!(
                "legacy sealed record too short: {} bytes, min {}",
                sealed.len(),
                Self::IV_LEN + Self::TAG_LEN
            )));
        }
        let (iv_bytes, ciphertext) = sealed.split_at(Self::IV_LEN);
        let nonce = Nonce::from_slice(iv_bytes);
        let aad = format!("{}|{}", self.service, record_type);
        self.cipher
            .decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|e| MemoryError::Invalid(format!("legacy AES-GCM open failed: {e}")))
    }

    /// Direct v2 open helper used by tests and callers that already know the logical id.
    fn open(
        &self,
        sealed: &[u8],
        record_type: &str,
        record_id: &str,
    ) -> Result<Vec<u8>, MemoryError> {
        if sealed.len() < Self::HEADER_LEN + Self::IV_LEN + Self::TAG_LEN {
            return Err(MemoryError::Invalid(format!(
                "sealed record too short: {} bytes, min {}",
                sealed.len(),
                Self::HEADER_LEN + Self::IV_LEN + Self::TAG_LEN
            )));
        }
        let expected_commitment = self.record_id_commitment(record_id);
        if sealed[13..45] != expected_commitment {
            return Err(MemoryError::Invalid(
                "AES-GCM open failed (logical record identity mismatch)".to_string(),
            ));
        }
        let index = u64::from_be_bytes(
            sealed[5..13]
                .try_into()
                .map_err(|_| MemoryError::Invalid("invalid record index".to_string()))?,
        );
        self.open_v2_record(sealed, record_type, index)
    }

    /// 写一条 record 到 file: JSON → seal → bytes → file
    fn write_record(
        &self,
        record_type: &str,
        record_id: &str,
        json_bytes: &[u8],
    ) -> Result<(), MemoryError> {
        let record_index = self.next_record_index(record_type)?;
        let sealed = self.seal(json_bytes, record_type, record_id, record_index)?;
        let path = self.dir.join(format!("{}.enc", record_type));
        std::fs::create_dir_all(&self.dir).map_err(MemoryError::from)?;
        // 0 装诚实: 长度前缀 (4 bytes big-endian) 替代 newline 分隔.
        // 原因: 随机 AES-GCM IV + ciphertext 可能含 0x0A (\n) 字节, 假 newline 分隔会切 mid-record
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(MemoryError::from)?;
        let len = u32::try_from(sealed.len())
            .map_err(|_| MemoryError::Invalid("sealed record is too large".to_string()))?;
        let len_bytes = len.to_be_bytes();
        f.write_all(&len_bytes).map_err(MemoryError::from)?;
        f.write_all(&sealed).map_err(MemoryError::from)?;
        Ok(())
    }

    /// Determine the next physical record index without decrypting existing records.
    fn next_record_index(&self, record_type: &str) -> Result<u64, MemoryError> {
        let path = self.dir.join(format!("{}.enc", record_type));
        if !path.exists() {
            return Ok(0);
        }
        let data = std::fs::read(&path).map_err(MemoryError::from)?;
        let mut pos = 0usize;
        let mut count = 0u64;
        while pos < data.len() {
            if data.len() - pos < 4 {
                return Err(MemoryError::Invalid(
                    "truncated record length prefix".to_string(),
                ));
            }
            let len = u32::from_be_bytes(
                data[pos..pos + 4]
                    .try_into()
                    .map_err(|_| MemoryError::Invalid("invalid record length".to_string()))?,
            ) as usize;
            pos += 4;
            if len == 0 || len > data.len() - pos {
                return Err(MemoryError::Invalid(
                    "truncated encrypted record".to_string(),
                ));
            }
            pos += len;
            count = count
                .checked_add(1)
                .ok_or_else(|| MemoryError::Invalid("too many encrypted records".to_string()))?;
        }
        Ok(count)
    }

    /// 读所有 record 从 file: 按长度前缀切 → open → JSON
    fn read_records(&self, record_type: &str) -> Result<Vec<Vec<u8>>, MemoryError> {
        let path = self.dir.join(format!("{}.enc", record_type));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read(&path).map_err(MemoryError::from)?;
        let mut out = Vec::new();
        let mut pos = 0;
        let mut record_index = 0u64;
        while pos < data.len() {
            if data.len() - pos < 4 {
                return Err(MemoryError::Invalid(
                    "truncated record length prefix".to_string(),
                ));
            }
            let len = u32::from_be_bytes(
                data[pos..pos + 4]
                    .try_into()
                    .map_err(|_| MemoryError::Invalid("invalid record length".to_string()))?,
            ) as usize;
            pos += 4;
            if len == 0 || len > data.len() - pos {
                return Err(MemoryError::Invalid(format!(
                    "truncated file at pos {pos}: expected {len} bytes, got {}",
                    data.len() - pos
                )));
            }
            let sealed = &data[pos..pos + len];
            let plaintext = if sealed.starts_with(&Self::MAGIC) {
                self.open_v2_record(sealed, record_type, record_index)?
            } else {
                self.open_legacy_record(sealed, record_type)?
            };
            out.push(plaintext);
            pos += len;
            record_index = record_index
                .checked_add(1)
                .ok_or_else(|| MemoryError::Invalid("too many encrypted records".to_string()))?;
        }
        Ok(out)
    }
}

impl MemoryBackend for EncryptedFileBackend {
    fn name(&self) -> &'static str {
        "file-encrypted"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::File
    }

    fn put_episode(&self, ep: &Episode) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_vec(ep).map_err(MemoryError::Json)?;
        self.write_record("episodes", &ep.id, &json)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn get_episode(
        &self,
        id: &str,
    ) -> Result<Option<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        let records = self.read_records("episodes")?;
        for plaintext in records {
            if let Ok(ep) = serde_json::from_slice::<Episode>(&plaintext) {
                if ep.id == id {
                    return Ok(Some(ep));
                }
            }
        }
        Ok(None)
    }

    fn recent_episodes(
        &self,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<Episode>, Box<dyn std::error::Error + Send + Sync>> {
        let records = self.read_records("episodes")?;
        let mut all: Vec<Episode> = Vec::new();
        for plaintext in records {
            if let Ok(ep) = serde_json::from_slice::<Episode>(&plaintext) {
                if ep.session_id == session_id {
                    all.push(ep);
                }
            }
        }
        all.sort_by_key(|e| e.timestamp);
        if all.len() > n {
            let skip = all.len() - n;
            all.drain(..skip);
        }
        Ok(all)
    }

    fn append_stream(
        &self,
        kind: apeireth_core::kernel::StreamKind,
        entry: HistoryEntry,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let table = crate::StreamKindExt::table_name_ext(kind);
        let json = serde_json::to_vec(&entry).map_err(MemoryError::Json)?;
        self.write_record(&table, &entry.id, &json)
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
    }

    fn list_stream(
        &self,
        kind: apeireth_core::kernel::StreamKind,
        session_id: &str,
        n: usize,
    ) -> Result<Vec<HistoryEntry>, Box<dyn std::error::Error + Send + Sync>> {
        let table = crate::StreamKindExt::table_name_ext(kind);
        let records = self.read_records(&table)?;
        let mut all: Vec<HistoryEntry> = Vec::new();
        for plaintext in records {
            if let Ok(entry) = serde_json::from_slice::<HistoryEntry>(&plaintext) {
                if entry.tombstoned_at.is_some() {
                    continue;
                }
                let matches = match &entry.session_id {
                    None => true,
                    Some(s) => s == session_id,
                };
                if matches {
                    all.push(entry);
                }
            }
        }
        all.sort_by_key(|e| e.created_at);
        if all.len() > n {
            let skip = all.len() - n;
            all.drain(..skip);
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;
    use tempfile::TempDir;

    fn fresh() -> (EncryptedFileBackend, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let backend = EncryptedFileBackend::for_dev_only(dir.path(), "test");
        (backend, dir)
    }

    fn ep(id: &str, session: &str) -> Episode {
        Episode {
            id: id.to_string(),
            timestamp: 1_700_000_000,
            role: "user".to_string(),
            content: format!("encrypted content of {id}"),
            session_id: session.to_string(),
        }
    }

    /// RC-10 验收: AES-256-GCM 加密 + 标签 (AEAD)
    #[test]
    fn seal_then_open_roundtrip() {
        let (b, _d) = fresh();
        let plaintext = b"hello encrypted world";
        let sealed = b.seal(plaintext, "episodes", "ep-1", 0).expect("seal");
        // sealed = v2 header + IV(12) + ciphertext + tag(16)
        assert!(sealed.len() >= EncryptedFileBackend::HEADER_LEN + 28);
        assert_eq!(&sealed[..4], b"APX2");
        // 解密
        let opened = b.open(&sealed, "episodes", "ep-1").expect("open");
        assert_eq!(opened, plaintext);
    }

    /// RC-10 验收: 改 logical record id → fail (identity commitment mismatch)
    #[test]
    fn wrong_logical_name_fails_open() {
        let (b, _d) = fresh();
        let plaintext = b"hello";
        let sealed = b.seal(plaintext, "episodes", "ep-1", 0).expect("seal");
        let result = b.open(&sealed, "episodes", "wrong-id");
        assert!(result.is_err(), "AAD mismatch 必须 fail, 不假装");
    }

    /// RC-10 验收: 改 record type/path → fail (AEAD tag 不 match)
    #[test]
    fn wrong_record_type_fails_open() {
        let (b, _d) = fresh();
        let sealed = b.seal(b"hello", "episodes", "ep-1", 0).expect("seal");
        let result = b.open(&sealed, "thought_stream", "ep-1");
        assert!(result.is_err(), "record type 变更必须 fail");
    }

    /// RC-10 验收: IV per-record (相同 plaintext → 不同 ciphertext bytes)
    #[test]
    fn iv_is_per_record_random() {
        let (b, _d) = fresh();
        let plaintext = b"same content";
        let sealed1 = b.seal(plaintext, "episodes", "ep-1", 0).expect("seal1");
        let sealed2 = b.seal(plaintext, "episodes", "ep-1", 1).expect("seal2");
        // v2 header 后的 12 bytes IV 不同
        let iv_start = EncryptedFileBackend::HEADER_LEN;
        assert_ne!(
            &sealed1[iv_start..iv_start + 12],
            &sealed2[iv_start..iv_start + 12],
            "IV per-record 必须不同"
        );
        // 完整 ciphertext 不同 (除 IV 外, ciphertext 也可能因 GCM stream 不同)
        // (不强求, 但 IV 不同已足够防 replay)
    }

    /// RC-10 验收: episode roundtrip 加密
    #[test]
    fn episode_roundtrip() {
        let (b, _d) = fresh();
        let e = ep("ep-1", "sess-1");
        b.put_episode(&e).expect("put");
        let got = b.get_episode("ep-1").expect("get").expect("exists");
        assert_eq!(got.id, "ep-1");
        assert_eq!(got.content, "encrypted content of ep-1");
    }

    /// RC-10 验收: 文件内容 0 明文 (encrypted bytes on disk)
    #[test]
    fn on_disk_is_encrypted_not_plaintext() {
        let (b, d) = fresh();
        let e = ep("secret-id", "sess-secret");
        b.put_episode(&e).expect("put");
        // 读磁盘上 episodes.enc 文件
        let path = d.path().join("episodes.enc");
        let on_disk = std::fs::read(&path).expect("read");
        // 0 装 PASS: 0 明文 episode content 在磁盘上
        let on_disk_str = String::from_utf8_lossy(&on_disk);
        assert!(
            !on_disk_str.contains("secret-id"),
            "episode id 0 装在明文 (encrypted only)"
        );
        assert!(
            !on_disk_str.contains("sess-secret"),
            "session_id 0 装在明文"
        );
        assert!(
            !on_disk_str.contains("encrypted content of secret-id"),
            "content 0 装在明文"
        );
    }

    /// RC-10 验收: stream append + list
    #[test]
    fn stream_roundtrip() {
        let (b, _d) = fresh();
        let sid = SessionId::new();
        let thought = apeireth_core::kernel::StreamKind::Thought;
        let entry = HistoryEntry {
            id: "t-1".into(),
            subject_id: "subj-1".into(),
            subject_rev: 1,
            session_id: Some(sid.to_string()),
            created_at: 1_700_000_100,
            payload: serde_json::json!({"kind": "test"}),
            source: "test".into(),
            tags: vec!["unit".into()],
            tombstoned_at: None,
        };
        b.append_stream(thought, entry.clone()).expect("append");
        let list = b.list_stream(thought, &sid.to_string(), 10).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "t-1");
    }

    #[test]
    fn ciphertext_tamper_fails() {
        let (b, _d) = fresh();
        let mut sealed = b.seal(b"hello", "episodes", "ep-1", 0).expect("seal");
        sealed[EncryptedFileBackend::HEADER_LEN + EncryptedFileBackend::IV_LEN] ^= 1;
        assert!(b.open(&sealed, "episodes", "ep-1").is_err());
    }

    #[test]
    fn nonce_tamper_fails() {
        let (b, _d) = fresh();
        let mut sealed = b.seal(b"hello", "episodes", "ep-1", 0).expect("seal");
        sealed[EncryptedFileBackend::HEADER_LEN] ^= 1;
        assert!(b.open(&sealed, "episodes", "ep-1").is_err());
    }

    #[test]
    fn header_tamper_fails() {
        let (b, _d) = fresh();
        let mut sealed = b.seal(b"hello", "episodes", "ep-1", 0).expect("seal");
        sealed[4] ^= 1;
        assert!(b.open(&sealed, "episodes", "ep-1").is_err());
    }

    #[test]
    fn record_swap_fails() {
        let (b, d) = fresh();
        b.put_episode(&ep("ep-1", "sess-1")).expect("put first");
        b.put_episode(&ep("ep-2", "sess-1")).expect("put second");
        let path = d.path().join("episodes.enc");
        let data = std::fs::read(&path).expect("read");
        let first_len = u32::from_be_bytes(data[..4].try_into().expect("first length")) as usize;
        let first_end = 4 + first_len;
        let mut swapped = data[first_end..].to_vec();
        swapped.extend_from_slice(&data[..first_end]);
        std::fs::write(&path, swapped).expect("write swapped records");

        assert!(b.get_episode("ep-1").is_err(), "record swap 必须 fail");
    }

    #[test]
    fn truncated_input_fails() {
        let (b, d) = fresh();
        b.put_episode(&ep("ep-1", "sess-1")).expect("put");
        let path = d.path().join("episodes.enc");
        let mut data = std::fs::read(&path).expect("read");
        data.pop();
        std::fs::write(&path, data).expect("write truncated record");
        assert!(b.get_episode("ep-1").is_err(), "截断 record 必须 fail");
    }

    #[test]
    fn framing_length_tamper_fails() {
        let (b, d) = fresh();
        b.put_episode(&ep("ep-1", "sess-1")).expect("put");
        let path = d.path().join("episodes.enc");
        let mut data = std::fs::read(&path).expect("read");
        let declared_len = u32::from_be_bytes(data[..4].try_into().expect("length"));
        data[..4].copy_from_slice(&declared_len.saturating_sub(1).to_be_bytes());
        std::fs::write(&path, data).expect("write tampered frame");
        assert!(
            b.get_episode("ep-1").is_err(),
            "length framing 变更必须 fail"
        );
    }

    #[test]
    fn legacy_v1_record_is_readable() {
        let (b, d) = fresh();
        let json = serde_json::to_vec(&ep("legacy-1", "sess-legacy")).expect("json");
        let mut iv_bytes = [0u8; EncryptedFileBackend::IV_LEN];
        rand::thread_rng().fill_bytes(&mut iv_bytes);
        let aad = format!("{}|episodes", b.service);
        let ciphertext = b
            .cipher
            .encrypt(
                Nonce::from_slice(&iv_bytes),
                aes_gcm::aead::Payload {
                    msg: &json,
                    aad: aad.as_bytes(),
                },
            )
            .expect("legacy seal");
        let mut sealed = iv_bytes.to_vec();
        sealed.extend_from_slice(&ciphertext);

        let path = d.path().join("episodes.enc");
        let mut framed = (sealed.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(&sealed);
        std::fs::write(path, framed).expect("write legacy record");

        let got = b
            .get_episode("legacy-1")
            .expect("read legacy")
            .expect("legacy exists");
        assert_eq!(got.id, "legacy-1");
    }

    /// RC-10 验收: Send + Sync 边界
    #[test]
    fn encrypted_file_backend_is_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<EncryptedFileBackend>();
    }
}
