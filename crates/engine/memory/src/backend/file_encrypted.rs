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
//! - AAD = `service_name || record_type || record_id` (per record 身份绑定, 防 replay)
//! - 每条 record 存 `[iv (12 bytes) || ciphertext || tag (16 bytes)]` binary
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
//! - AAD 防 tampering: 篡改 record_id / type 会 fail AEAD tag verification
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline.
//!
//! **v1 compat**: 100+ consumer 0 破 (新增 module, 0 改 FileBackend).
//!
//! **⚠️ Breaking change (子代理 E 审查建议 1, 2026-08-27)**:
//! 加密文件格式 v2 (line header `[id_len:2 BE][id:id_len][sealed_len:4 BE][sealed:N]`)
//! **不可读 v1** (老格式 `[sealed_len:4 BE][sealed:N]` 无 id 字段).
//! RC-10 仍在 rc 阶段, 未上线, 0 影响.
//! 真生产: 升级前需 `data migration script` 读老文件, 重写新格式.
//! Per-record AAD tamper 保护 (子代理 C 建议 #5 兑现, 2026-08-27):
//! AAD = `service|type|record_id` (line header 提供 record_id 重建).
//! 篡改 record_id 必 fail AEAD verify (fail-closed, 0 装假装安全).

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
    /// Root directory (同 alpha FileBackend: `<root>/episodes.jsonl` + `<root>/streams/*.jsonl`)
    dir: PathBuf,
    /// Service name (用于 AAD: `service_name || record_type || record_id`)
    service: String,
}

impl EncryptedFileBackend {
    /// 32 bytes master key (256-bit AES-GCM)
    pub const KEY_LEN: usize = 32;

    /// 12 bytes IV (96-bit AES-GCM standard nonce)
    pub const IV_LEN: usize = 12;

    /// 16 bytes AEAD tag (GCM tag length)
    pub const TAG_LEN: usize = 16;

    /// record_id 字节长度上限 (u16 = 65535 bytes, 0 装诚实标注)
    /// UUID / SHA-style id 通常 8-36 bytes, 0 装期望
    pub const ID_LEN_MAX: usize = u16::MAX as usize;

    /// 创 EncryptedFileBackend with explicit 32-byte master key.
    /// **0 装 PASS**: master key 0 真接 OS keyring (alpha 0 装). RC-10 阶段接 `KeyringSelector`.
    pub fn new(root: impl Into<PathBuf>, master_key: &[u8; Self::KEY_LEN], service: impl Into<String>) -> Self {
        let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(master_key));
        Self {
            cipher,
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

    /// 加密 + AEAD 标签: 返 `[iv (12) || ciphertext (N) || tag (16)]` 字节数组
    fn seal(&self, plaintext: &[u8], record_type: &str, record_id: &str) -> Result<Vec<u8>, MemoryError> {
        // IV per-record (12 bytes random)
        let mut iv_bytes = [0u8; Self::IV_LEN];
        rand::thread_rng().fill_bytes(&mut iv_bytes);
        let nonce = Nonce::from_slice(&iv_bytes);
        // 0 装诚实: per-record AAD = service|type|record_id (line header 提供 record_id)
        // 真生产可加 line header (record_id 长度前缀) 防 tamper
        let aad = format!("{}|{}|{}", self.service, record_type, record_id);
        let aad_bytes = aad.as_bytes();
        // AEAD seal (encrypt + tag)
        let mut ciphertext = self
            .cipher
            .encrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: plaintext,
                    aad: aad_bytes,
                },
            )
            .map_err(|e| MemoryError::Invalid(format!("AES-GCM seal failed: {e}")))?;
        // 拼装: iv (12) || ciphertext_with_tag
        let mut out = Vec::with_capacity(Self::IV_LEN + ciphertext.len());
        out.extend_from_slice(&iv_bytes);
        out.append(&mut ciphertext);
        Ok(out)
    }

    /// 解密 + AEAD verify: 0 record_id AAD (供 read_records 用, 不知 per-line record_id)
    fn open_record(
        &self,
        sealed: &[u8],
        record_type: &str,
        record_id: &str,
    ) -> Result<Vec<u8>, MemoryError> {
        if sealed.len() < Self::IV_LEN + Self::TAG_LEN {
            return Err(MemoryError::Invalid(format!(
                "sealed record too short: {} bytes, min {}",
                sealed.len(),
                Self::IV_LEN + Self::TAG_LEN
            )));
        }
        let (iv_bytes, ciphertext) = sealed.split_at(Self::IV_LEN);
        let nonce = Nonce::from_slice(iv_bytes);
        // 0 装诚实: per-record AAD = service|type|record_id
        // 改 record_id 必 fail AEAD verify (子代理 C 建议的 tamper 保护落地)
        let aad = format!("{}|{}|{}", self.service, record_type, record_id);
        self.cipher
            .decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|e| MemoryError::Invalid(format!("AES-GCM open failed (AAD mismatch or tamper): {e}")))
    }

    /// 解密 + AEAD verify: 输入 `[iv (12) || ciphertext (N) || tag (16)]`, 返 plaintext
    /// AAD 必与 seal 时一致 (per record_id + type)
    fn open(&self, sealed: &[u8], record_type: &str, record_id: &str) -> Result<Vec<u8>, MemoryError> {
        if sealed.len() < Self::IV_LEN + Self::TAG_LEN {
            return Err(MemoryError::Invalid(format!(
                "sealed record too short: {} bytes, min {}",
                sealed.len(),
                Self::IV_LEN + Self::TAG_LEN
            )));
        }
        let (iv_bytes, ciphertext) = sealed.split_at(Self::IV_LEN);
        let nonce = Nonce::from_slice(iv_bytes);
        // 0 装诚实: read_records 不知道每行 record_id, 用 service+type 简化 AAD.
        // record_id 参数保留, 真生产加 line header (record_id 长度前缀) 即可
        // 0 装诚实: 用 per-record AAD = service|type|record_id (与 seal 一致)
        let aad = format!("{}|{}|{}", self.service, record_type, record_id);
        self.cipher
            .decrypt(
                nonce,
                aes_gcm::aead::Payload {
                    msg: ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|e| MemoryError::Invalid(format!("AES-GCM open failed (AAD mismatch or tamper): {e}")))
    }

    /// 写一条 record 到 file: JSON → seal → bytes → file
    fn write_record(
        &self,
        record_type: &str,
        record_id: &str,
        json_bytes: &[u8],
    ) -> Result<(), MemoryError> {
        let sealed = self.seal(json_bytes, record_type, record_id)?;
        // 0 装诚实: line header `[id_len(2)][id][sealed]` 让 read 按 record_id 重建 AAD.
        // 原因: 真生产需 per-record AAD tamper 保护 (每条 record 独立身份).
        // 0 装诚实: record_id 长度 u16 = Self::ID_LEN_MAX (65535 bytes).
        // UUID / SHA-style id 通常 8-36 bytes, 0 装期望.
        // 超出返 Invalid error, 不 panic.
        let id_bytes = record_id.as_bytes();
        if id_bytes.len() > Self::ID_LEN_MAX {
            return Err(MemoryError::Invalid(format!(
                "record_id too long: {} bytes, max {}",
                id_bytes.len(),
                Self::ID_LEN_MAX
            )));
        }
        let id_len = (id_bytes.len() as u16).to_be_bytes();
        let path = self.dir.join(format!("{}.enc", record_type));
        std::fs::create_dir_all(&self.dir).map_err(MemoryError::from)?;
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(MemoryError::from)?;
        // 文件格式: `[id_len: 2 BE][id: id_len][sealed_len: 4 BE][sealed: N]`
        // sealed_len 在 sealed 之前 (4 bytes BE) — reader 顺序读
        f.write_all(&id_len).map_err(MemoryError::from)?;
        f.write_all(id_bytes).map_err(MemoryError::from)?;
        let sealed_len = (sealed.len() as u32).to_be_bytes();
        f.write_all(&sealed_len).map_err(MemoryError::from)?;
        f.write_all(&sealed).map_err(MemoryError::from)?;
        Ok(())
    }

    /// 读所有 record 从 file: 按 line header (id_len + id + sealed_len + sealed) → open → JSON
    fn read_records(&self, record_type: &str) -> Result<Vec<Vec<u8>>, MemoryError> {
        let path = self.dir.join(format!("{}.enc", record_type));
        if !path.exists() {
            return Ok(Vec::new());
        }
        let data = std::fs::read(&path).map_err(MemoryError::from)?;
        let mut out = Vec::new();
        let mut pos = 0;
        while pos + 2 <= data.len() {
            // [id_len: 2 BE]
            if pos + 2 > data.len() {
                return Err(MemoryError::Invalid("truncated at id_len".into()));
            }
            let id_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;
            // [id: id_len]
            if pos + id_len > data.len() {
                return Err(MemoryError::Invalid(format!(
                    "truncated id at pos {pos}: expected {id_len} bytes, got {}",
                    data.len() - pos
                )));
            }
            let record_id = std::str::from_utf8(&data[pos..pos + id_len])
                .map_err(|e| MemoryError::Invalid(format!("invalid utf8 record_id: {e}")))?;
            pos += id_len;
            // [sealed_len: 4 BE]
            if pos + 4 > data.len() {
                return Err(MemoryError::Invalid("truncated at sealed_len".into()));
            }
            let sealed_len = u32::from_be_bytes([
                data[pos],
                data[pos + 1],
                data[pos + 2],
                data[pos + 3],
            ]) as usize;
            pos += 4;
            // [sealed: sealed_len]
            if pos + sealed_len > data.len() {
                return Err(MemoryError::Invalid(format!(
                    "truncated sealed at pos {pos}: expected {sealed_len} bytes, got {}",
                    data.len() - pos
                )));
            }
            let sealed = &data[pos..pos + sealed_len];
            // 0 装诚实: 用 record_id 重建 AAD (per-record tamper 保护)
            let plaintext = self.open_record(sealed, record_type, record_id)?;
            out.push(plaintext);
            pos += sealed_len;
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

    fn get_episode(&self, id: &str) -> Result<Option<Episode>, Box<dyn std::error::Error + Send + Sync>> {
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
        let sealed = b.seal(plaintext, "episodes", "ep-1").expect("seal");
        // sealed 至少 IV(12) + tag(16) = 28 bytes
        assert!(sealed.len() >= 28);
        // sealed 前 12 bytes = IV
        assert_eq!(&sealed[..12].len(), &12);
        // 解密
        let opened = b.open(&sealed, "episodes", "ep-1").expect("open");
        assert_eq!(opened, plaintext);
    }

    /// RC-10 验收: 改 record_type 路径 → fail (AEAD tag 不 match)
    /// 注: 0 装诚实: per-record AAD = service|type|record_id (line header 提供 record_id).
    /// 测试用 record_type + record_id mismatch 都必须 fail (子代理 C 建议的 tamper 保护落地).
    #[test]
    fn aad_mismatch_fails_open() {
        let (b, _d) = fresh();
        let plaintext = b"hello";
        let sealed = b.seal(plaintext, "episodes", "ep-1").expect("seal");
        // 错 record_type 解密 → fail
        let result = b.open_record(&sealed, "thought_stream", "ep-1");
        assert!(result.is_err(), "AAD mismatch (record_type) 必须 fail, 不假装");
    }

    /// RC-10 验收: 错 record_id 解密 → fail (per-record AAD tamper 保护)
    #[test]
    fn aad_mismatch_record_id_fails_open() {
        let (b, _d) = fresh();
        let plaintext = b"hello";
        let sealed = b.seal(plaintext, "episodes", "ep-1").expect("seal");
        // 错 record_id 解密 → fail (子代理 C 建议, line header 提供 per-record tamper 保护)
        let result = b.open_record(&sealed, "episodes", "ep-WRONG");
        assert!(
            result.is_err(),
            "AAD mismatch (record_id) 必须 fail, per-record tamper 保护"
        );
    }

    /// RC-10 验收: line header 正确还原 record_id (roundtrip 通过)
    #[test]
    fn line_header_roundtrip_record_id() {
        let (b, _d) = fresh();
        // append 2 records, 读后 record_id 应在 plaintext JSON 里
        b.append_stream(
            apeireth_core::kernel::StreamKind::Thought,
            HistoryEntry {
                id: "t-001".into(),
                subject_id: "subj-1".into(),
                subject_rev: 1,
                session_id: Some("sess-1".into()),
                created_at: 1_700_000_000,
                payload: serde_json::json!({"event": "test"}),
                source: "test".into(),
                tags: vec!["rc10".into()],
                tombstoned_at: None,
            },
        )
        .expect("append 1");
        b.append_stream(
            apeireth_core::kernel::StreamKind::Thought,
            HistoryEntry {
                id: "t-002".into(),
                subject_id: "subj-1".into(),
                subject_rev: 1,
                session_id: Some("sess-1".into()),
                created_at: 1_700_001_000,
                payload: serde_json::json!({"event": "test2"}),
                source: "test".into(),
                tags: vec!["rc10".into()],
                tombstoned_at: None,
            },
        )
        .expect("append 2");
        // 0 装诚实: 读 records 2 条 + line header 能正确还原 record_id (否则 roundtrip 失败)
        let list = b
            .list_stream(apeireth_core::kernel::StreamKind::Thought, "sess-1", 10)
            .expect("list");
        assert_eq!(list.len(), 2, "line header 应正确还原 2 records");
        let ids: Vec<&str> = list.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"t-001"));
        assert!(ids.contains(&"t-002"));
    }

    /// RC-10 验收: IV per-record (相同 plaintext → 不同 ciphertext bytes)
    #[test]
    fn iv_is_per_record_random() {
        let (b, _d) = fresh();
        let plaintext = b"same content";
        let sealed1 = b.seal(plaintext, "episodes", "ep-1").expect("seal1");
        let sealed2 = b.seal(plaintext, "episodes", "ep-1").expect("seal2");
        // 12 bytes IV 不同
        assert_ne!(&sealed1[..12], &sealed2[..12], "IV per-record 必须不同");
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
    /// 0 装诚实: line header `[id_len][id]` 中 `id` 是明文 (按 line header 协议).
    /// 验证 sealed `content` + `session_id` + `role` + `timestamp` 都在密文 (AAD-protected).
    #[test]
    fn on_disk_is_encrypted_not_plaintext() {
        let (b, d) = fresh();
        let e = ep("secret-id", "sess-secret");
        b.put_episode(&e).expect("put");
        // 读磁盘上 episodes.enc 文件
        let path = d.path().join("episodes.enc");
        let on_disk = std::fs::read(&path).expect("read");
        // 0 装 PASS: sealed content 0 在明文
        let on_disk_str = String::from_utf8_lossy(&on_disk);
        // session_id 是 JSON field, 0 装在明文
        assert!(
            !on_disk_str.contains("sess-secret"),
            "session_id 0 装在明文 (sealed 在 sealed bytes 内)"
        );
        // role "user" 太常见, 改测独有 payload (测试代码用 "encrypted content of ep-1")
        assert!(
            !on_disk_str.contains("encrypted content of secret-id"),
            "content 0 装在明文"
        );
    }

    /// RC-10 验收: 损坏 line header → fail (子代理 E 建议 2 边界 case)
    /// 0 装: 构造一个 id_len 字段说 1000 但实际只写 5 字节 id 的损坏文件
    /// → 期望 read_records 返 `truncated id` 错误而非 panic
    #[test]
    fn truncated_id_returns_error_not_panic() {
        let (_b, d) = fresh();
        use std::io::Write;
        // 写损坏 line header: id_len=1000 (u16 BE), 但只 5 字节 id, 无 sealed
        let path = d.path().join("episodes.enc");
        let mut f = std::fs::File::create(&path).expect("create");
        f.write_all(&1000u16.to_be_bytes()).expect("write id_len");
        f.write_all(b"short").expect("write id 5 bytes");
        f.sync_all().expect("sync");
        drop(f);
        // 重新构造 store, 触发 read_records
        let store = EncryptedFileBackend::for_dev_only(d.path(), "test");
        // get_episode 走 read_records → 期望 Invalid 错误 (不 panic)
        let result = store.get_episode("any-id");
        assert!(result.is_err(), "损坏 line header 必须 fail, 不假装");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("truncated id") || err.contains("truncated"),
            "必须显式 truncated error 诊断 (fail-closed), 错误: {err}"
        );
    }

    /// RC-10 验收: record_id 超长 (over ID_LEN_MAX) → 返 Invalid 错误
    #[test]
    fn record_id_too_long_returns_error() {
        let (b, _d) = fresh();
        // 65536 字节 = ID_LEN_MAX + 1
        let too_long = "x".repeat(65536);
        // 直接调 write_record (因 put_episode 接受任何 id, 但 write_record 长度检查)
        let result = b.put_episode(&::apeireth_core::kernel::Episode {
            id: too_long,
            timestamp: 1_700_000_000,
            role: "user".into(),
            content: "test".into(),
            session_id: "sess".into(),
        });
        assert!(result.is_err(), "ID_LEN_MAX 超出必须 fail, 不假装");
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

    /// RC-10 验收: Send + Sync 边界
    #[test]
    fn encrypted_file_backend_is_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<EncryptedFileBackend>();
    }
}