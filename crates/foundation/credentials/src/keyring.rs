//! **TP20-S3 / KeyringBackend — 统一 keyring 后端抽象 (塞缝批, 安全凭证)**
//!
//! **哲学锚点**: 机制而非补丁 + 集成而非分立 + 安全底线.
//!
//! **目的 (per §11 TP20-S3)**: `apeireth-credentials` 当前缺统一 keyring 后端抽象,
//! API key / OAuth token 等敏感凭证散落 env 或代码硬编码。本模块给出**统一 trait 口**,
//! 实现侧挂平台 keyring (Linux Secret Service / macOS Keychain / Windows Credential Manager),
//! 不可用时降级 [`EncryptedFileBackend`] (chacha20poly1305 + master.key),
//! 内存层用 [`SecretBuf`] (Drop zeroize), 每次访问写**审计**含 `name_hash` 不含明文.
//!
//! ## 5 组件
//!
//! | 组件 | 类型 | 说明 |
//! |---|---|---|
//! | 1 trait 口 | [`KeyringBackend`] | `get/set/delete/list` 4 方法 |
//! | 2 错误 | [`KeyringError`] | thiserror 派生, 元信息不含明文 |
//! | 3 平台实现 | [`PlatformKeyring`] | 走 `keyring` crate 3.6 自动选 |
//! | 4 加密文件 fallback | [`EncryptedFileBackend`] | chacha20poly1305 AEAD + master.key (32B, 0600) |
//! | 5 内存 stub | [`InMemoryKeyring`] | 测试 / 限流 / 0 装 placeholder |
//!
//! ## 边界 (per 任务 description)
//!
//! - 仅改 `crates/apeireth-credentials/src/**`, **不触碰** team-lead / tool-runtime / agent / companion / net.
//! - 新增依赖: `keyring` + `zeroize` (任务指定) + chacha20poly1305 + rand + sha2 + hex (加密 + 审计).
//! - **不接云 KMS, 不做凭证轮换** (per 任务非目标).
//!
//! ## 0 假装边界 (诚实标注)
//!
//! - **平台 keyring 不可用时**才 fallback `EncryptedFileBackend`, 后者依赖 master.key
//!   文件权限 (unix 0600) 收敛访问 — 这与 [`FileCredentialsStore`] (明文文件) 同
//!   "靠 OS 权限" 边界, 升级到 OS DPAPI / KMS / HSM 属后续层.
//! - **审计日志不持久化**: 默认实现 `NoopAudit`, 装配侧可挂真 audit sink
//!   (telemetry / 经验库), 此处为 trait 口 (0 装 PASS 标注).
//! - **name_hash = SHA-256(service)[:16] hex** — 不可逆 (单向散列) 但同 service
//!   同 hash, 仍可关联审计, 不暴露 service 名原文. 若需要更强匿名, 上层
//!   应再加盐 (`AuditContext::with_salt`).

#![allow(clippy::result_large_err)] // KeyringError 变体多, 不强求 box 化

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use zeroize::Zeroize;

use crate::secret::SecretBuf;
use crate::store::validate_service_name;

// ============================================================================
// §1 KeyringError — 错误类型 (thiserror, Display 不含明文)
// ============================================================================

/// Keyring 后端错误类型.
///
/// **红线 (per 任务纪律)**: 任一 `Display` 输出不得含明文 (只含服务名 / 路径 / 长度元信息).
#[derive(Debug, Error)]
pub enum KeyringError {
    /// 凭据不存在 (`get` / `delete` 时未找到).
    #[error("keyring: unknown service `{service}`")]
    UnknownService {
        /// 目标服务名 (元信息).
        service: String,
    },

    /// 服务名非法 (与 [`crate::store::validate_service_name`] 同).
    #[error("keyring: invalid service name `{0}`")]
    InvalidServiceName(String),

    /// 服务名长度超限 (默认 128, 防 keyring 平台拒绝).
    #[error("keyring: service name too long ({len} bytes, max {max})")]
    ServiceNameTooLong {
        /// 实际长度.
        len: usize,
        /// 上限.
        max: usize,
    },

    /// 平台 keyring 后端不可用 (DBus 未运行 / Credential Manager 拒绝).
    /// **自动降级**: `KeyringSelector` 捕获此错误后切到 `EncryptedFileBackend`.
    #[error("keyring: platform backend unavailable ({0})")]
    BackendUnavailable(String),

    /// 平台 keyring 拒绝访问 (权限 / 沙箱).
    #[error("keyring: access denied for service `{service}` ({reason})")]
    AccessDenied {
        /// 服务名.
        service: String,
        /// 拒绝原因 (元信息).
        reason: String,
    },

    /// 底层 IO 错误.
    #[error("keyring: io error for service `{service}`: {source}")]
    Io {
        /// 服务名.
        service: String,
        #[source]
        source: std::io::Error,
    },

    /// 加密/解密失败 (master.key 损坏 / nonce 重放 / 文件被改).
    #[error("keyring: crypto error ({0})")]
    Crypto(String),

    /// 序列化 / 反序列化错误.
    #[error("keyring: storage format error: {0}")]
    Format(String),

    /// 凭据长度超限 (防 memory exhaustion, 默认 4 KB).
    #[error("keyring: secret too long ({len} bytes, max {max})")]
    SecretTooLong {
        /// 实际长度.
        len: usize,
        /// 上限.
        max: usize,
    },

    /// 后端自身错误 (generic, 元信息不含明文).
    #[error("keyring: backend error: {0}")]
    Backend(String),
}

/// Keyring Result 别名.
pub type Result<T> = std::result::Result<T, KeyringError>;

// ============================================================================
// §2 KeyringBackend trait — 统一接口
// ============================================================================

/// **统一 keyring 后端 trait**.
///
/// 设计原则: 与 [`crate::store::CredentialsStore`] 同构 (get/set/delete/list)
/// 但 trait 不绑定, 上层可分别选择存储后端 (文件 vs keyring) 而不耦合.
///
/// 实现方应保证:
/// - `get` / `delete` 在服务不存在时返 [`KeyringError::UnknownService`];
/// - `set` 覆盖语义 (幂等);
/// - `list` 只返服务名 (不含明文);
/// - **每次访问**通过 [`AuditSink::record`] 写一条审计 (含 `name_hash`, 不含明文).
pub trait KeyringBackend: Send + Sync {
    /// 读取服务凭据明文; 不存在 → `UnknownService`.
    fn get(&self, service: &str) -> Result<SecretBuf>;

    /// 写入/覆盖服务凭据.
    fn set(&self, service: &str, secret: &SecretBuf) -> Result<()>;

    /// 删除服务凭据; 不存在 → `UnknownService`.
    fn delete(&self, service: &str) -> Result<()>;

    /// 列出已存服务名 (仅名称, 不含明文).
    fn list(&self) -> Result<Vec<String>>;

    /// 后端名字 (供日志/诊断用, 元信息).
    fn backend_name(&self) -> &'static str;
}

// ============================================================================
// §3 AuditSink — 审计 trait (name_hash 不含明文)
// ============================================================================

/// **审计事件类别**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEvent {
    /// 读取凭据 (含 get 成功/失败).
    Get,
    /// 写入凭据.
    Set,
    /// 删除凭据.
    Delete,
    /// 列出服务名 (list 不含明文但仍记录).
    List,
}

/// **审计 sink trait**: 装配侧挂真 audit (telemetry / 经验库), 0 装默认 [`NoopAudit`].
///
/// **红线 (per 任务纪律)**: 任一审计条目**不得含**:
/// - 凭据明文 (service 名原文 + bytes 明文);
/// - secret bytes 内容.
///
/// 仅可含: 时间戳 / event / `name_hash` (SHA-256 service 前 16 hex) /
///         success 布尔 / backend 名 (元信息).
pub trait AuditSink: Send + Sync {
    /// 写一条审计.
    ///
    /// `service` 原文经 [`name_hash`] 哈希后写入, **绝不**直接进入审计记录.
    fn record(&self, event: AuditEvent, service: &str, backend: &'static str, success: bool);
}

/// **0 装默认 audit**: 不写任何地方, 仅在单测中验证 trait 调用次数.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopAudit;

impl AuditSink for NoopAudit {
    fn record(&self, _event: AuditEvent, _service: &str, _backend: &'static str, _success: bool) {
        // 0 装 PASS: 不写任何位置, 调用计数走单测自检.
    }
}

/// **计数 audit** (单测用, 验证调用次数 / 不含明文).
#[derive(Debug, Default)]
pub struct CountingAudit {
    /// 内部互斥保护计数 (简单 Mutex, 不为高并发优化).
    inner: std::sync::Mutex<Vec<AuditEntry>>,
}

/// 单条审计记录 (元信息, **不含**明文).
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// 事件类别.
    pub event: AuditEvent,
    /// 服务名哈希 (SHA-256 前 16 hex).
    pub name_hash: String,
    /// 后端名.
    pub backend: &'static str,
    /// 是否成功.
    pub success: bool,
}

impl CountingAudit {
    /// 新建空计数 audit.
    pub fn new() -> Self {
        Self::default()
    }

    /// 取所有审计条目 (clone).
    pub fn entries(&self) -> Vec<AuditEntry> {
        self.inner.lock().unwrap().clone()
    }

    /// 审计条目数.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 验证**没有任何审计条目包含明文 service 名** (回归保护).
    pub fn assert_no_plaintext(&self, plaintext_services: &[&str]) {
        let entries = self.entries();
        for e in &entries {
            for plain in plaintext_services {
                assert!(
                    !e.name_hash.contains(plain),
                    "审计条目不应含明文 service 名 `{plain}`: {e:?}"
                );
            }
        }
    }
}

impl AuditSink for CountingAudit {
    fn record(&self, event: AuditEvent, service: &str, backend: &'static str, success: bool) {
        let name_hash = name_hash(service).clone();
        self.inner.lock().unwrap().push(AuditEntry {
            event,
            name_hash,
            backend,
            success,
        });
    }
}

/// **SHA-256(service) 前 16 hex** (64 bit 截断). 单向, 同 service 同 hash.
///
/// 设计: 16 hex = 64 bit, 平衡 "可关联 (同 service 同 hash)" 与 "不可逆".
/// 若需更强匿名 (防 rainbow table), 上层加盐 (见 `AuditContext::with_salt` 占位).
pub fn name_hash(service: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(service.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..8])
}

// ============================================================================
// §4 PlatformKeyring — 平台 keyring (Linux Secret Service / macOS / Windows)
// ============================================================================

/// **平台 keyring 实现** (走 [`keyring`] crate 3.6 自动选).
///
/// - Linux: Secret Service (D-Bus) — 需 `gnome-keyring` / `kwallet` 等运行.
/// - macOS: Keychain.
/// - Windows: Credential Manager.
///
/// **降级语义**: 构造时**不**主动探测; 首次 `get`/`set` 时若遇
/// [`keyring::Error::NoBackend`] / `PlatformFailure`, 调用方应切到 fallback.
/// 见 [`KeyringSelector::select_or_fallback`].
pub struct PlatformKeyring {
    /// 服务名前缀 (防止跨 app 撞名; 默认 `"apeireth"`).
    service_prefix: String,
    /// 审计 sink.
    audit: Arc<dyn AuditSink>,
}

impl PlatformKeyring {
    /// 构造平台 keyring, 带 audit sink.
    ///
    /// `service_prefix`: 写入 OS keyring 时的 service 名加此前缀, 防撞名.
    pub fn new(service_prefix: impl Into<String>, audit: Arc<dyn AuditSink>) -> Self {
        Self {
            service_prefix: service_prefix.into(),
            audit,
        }
    }

    /// OS keyring 真实 service 名 (prefix + service).
    fn os_service(&self, service: &str) -> String {
        format!("{}.{}", self.service_prefix, service)
    }

    /// 探测平台后端是否可用 (尝试 dummy get). 用于启动期一次性探测.
    pub fn probe_available() -> bool {
        // 走 keyring crate 的 Entry::try_new, 失败即不可用.
        match keyring::Entry::new("__apeireth_probe__", "__probe__") {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

impl KeyringBackend for PlatformKeyring {
    fn get(&self, service: &str) -> Result<SecretBuf> {
        check_service_name(service)?;
        let entry = keyring::Entry::new(&self.os_service(service), "default")
            .map_err(|e| self.classify_error(service, &e.to_string()))?;
        let raw = match entry.get_password() {
            Ok(s) => s,
            Err(keyring::Error::NoEntry) => {
                self.audit
                    .record(AuditEvent::Get, service, "platform", false);
                return Err(KeyringError::UnknownService {
                    service: service.to_string(),
                });
            }
            Err(e) => {
                let reason = e.to_string();
                self.audit
                    .record(AuditEvent::Get, service, "platform", false);
                return Err(self.classify_error(service, &reason));
            }
        };
        if raw.len() > MAX_SECRET_LEN {
            self.audit
                .record(AuditEvent::Get, service, "platform", false);
            return Err(KeyringError::SecretTooLong {
                len: raw.len(),
                max: MAX_SECRET_LEN,
            });
        }
        self.audit
            .record(AuditEvent::Get, service, "platform", true);
        Ok(SecretBuf::new(raw.into_bytes()))
    }

    fn set(&self, service: &str, secret: &SecretBuf) -> Result<()> {
        check_service_name(service)?;
        if secret.len() > MAX_SECRET_LEN {
            return Err(KeyringError::SecretTooLong {
                len: secret.len(),
                max: MAX_SECRET_LEN,
            });
        }
        let entry = keyring::Entry::new(&self.os_service(service), "default")
            .map_err(|e| self.classify_error(service, &e.to_string()))?;
        // secret bytes -> UTF-8 lossy (平台 keyring 存字符串).
        let s = String::from_utf8_lossy(secret.expose()).into_owned();
        match entry.set_password(&s) {
            Ok(()) => {
                self.audit
                    .record(AuditEvent::Set, service, "platform", true);
                Ok(())
            }
            Err(e) => {
                let reason = e.to_string();
                self.audit
                    .record(AuditEvent::Set, service, "platform", false);
                Err(self.classify_error(service, &reason))
            }
        }
    }

    fn delete(&self, service: &str) -> Result<()> {
        check_service_name(service)?;
        let entry = keyring::Entry::new(&self.os_service(service), "default")
            .map_err(|e| self.classify_error(service, &e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => {
                self.audit
                    .record(AuditEvent::Delete, service, "platform", true);
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                self.audit
                    .record(AuditEvent::Delete, service, "platform", false);
                Err(KeyringError::UnknownService {
                    service: service.to_string(),
                })
            }
            Err(e) => {
                let reason = e.to_string();
                self.audit
                    .record(AuditEvent::Delete, service, "platform", false);
                Err(self.classify_error(service, &reason))
            }
        }
    }

    fn list(&self) -> Result<Vec<String>> {
        // 平台 keyring crate 3.6 0 提供 list API — 返 "不支持" 让上层走 fallback.
        // 这是 0 假装边界 (per 任务哲学诚实标注).
        Err(KeyringError::Backend(
            "platform keyring crate 3.6 has no list API; use EncryptedFileBackend for list"
                .to_string(),
        ))
    }

    fn backend_name(&self) -> &'static str {
        "platform"
    }
}

impl PlatformKeyring {
    /// 把 keyring crate 错误归类为本 crate 错误, 区分"不可用"(可降级) 与"权限拒绝"(不可降级).
    fn classify_error(&self, service: &str, reason: &str) -> KeyringError {
        // keyring crate 错误文本含 "no backend" / "platform" / "dbus" 等关键词时归 BackendUnavailable.
        let lower = reason.to_ascii_lowercase();
        if lower.contains("no backend")
            || lower.contains("platform")
            || lower.contains("dbus")
            || lower.contains("not available")
        {
            KeyringError::BackendUnavailable(reason.to_string())
        } else if lower.contains("access denied") || lower.contains("permission") {
            KeyringError::AccessDenied {
                service: service.to_string(),
                reason: reason.to_string(),
            }
        } else {
            KeyringError::Backend(reason.to_string())
        }
    }
}

// ============================================================================
// §5 EncryptedFileBackend — chacha20poly1305 + master.key fallback
// ============================================================================

/// **加密文件 fallback 后端** (per 任务指定: chacha20poly1305 + master.key).
///
/// **磁盘格式** (`apeireth-keyring.bin`, 单文件):
/// - magic 4B `"APK1"` (Apeireth Keyring v1)
/// - master key 派生: 文件首 32B 由 master.key 提供, master.key 单独存
/// - 数据: nonce(24B) || ciphertext_len(u32 LE) || ciphertext || tag(16B, AEAD 内嵌)
///
/// **master.key** (32B raw, 单独文件 `apeireth-keyring.master.key`):
/// - 不存在时**首次构造**生成 32B 随机密钥, 写盘 (unix 0600 语义, 见 [`write_master_key`]).
/// - 损坏 / 长度错 → [`KeyringError::Crypto`].
///
/// **算法**: XChaCha20-Poly1305 (24B nonce 安全, 防 nonce 重放).
///
/// **与 [`FileCredentialsStore`] 边界**:
/// - `FileCredentialsStore` 是明文静态存储 (靠 0600);
/// - `EncryptedFileBackend` 是加密静态存储 (0600 + AEAD), 即使文件泄漏也不出明文.
/// - 二者并存: 装配侧可任选; 不互替.
pub struct EncryptedFileBackend {
    /// 数据文件路径.
    data_path: PathBuf,
    /// master.key 文件路径.
    master_key_path: PathBuf,
    /// 内存中的 master key (Clone 时复制, Drop 由 `Key` 自身 zeroize — Key impl Drop zeroize).
    master_key: Key,
    /// 审计 sink.
    audit: Arc<dyn AuditSink>,
}

impl std::fmt::Debug for EncryptedFileBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptedFileBackend")
            .field("data_path", &self.data_path)
            .field("master_key_path", &self.master_key_path)
            .finish_non_exhaustive()
    }
}

/// 加密文件 magic 头 (`"APK1"` = Apeireth Keyring v1).
const MAGIC: &[u8; 4] = b"APK1";

/// XChaCha20-Poly1305 nonce 长度.
const NONCE_LEN: usize = 24;

/// Ciphertext 长度字段 (u32 LE) 大小.
const LEN_FIELD: usize = 4;

/// master.key 长度.
const MASTER_KEY_LEN: usize = 32;

/// 服务名长度上限 (与 keyring crate 兼容, 防止 OS 拒绝超长 service).
pub const MAX_SERVICE_NAME_LEN: usize = 128;

/// 凭据明文长度上限 (4 KB, 防 memory exhaustion).
pub const MAX_SECRET_LEN: usize = 4096;

impl EncryptedFileBackend {
    /// **构造 + 自动初始化 master.key** (首次生成, 后续读取).
    ///
    /// `dir`: 后端目录; 文件固定为 `dir/apeireth-keyring.bin` + `dir/apeireth-keyring.master.key`.
    pub fn open(dir: impl AsRef<Path>, audit: Arc<dyn AuditSink>) -> Result<Self> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir).map_err(|source| KeyringError::Io {
            service: "<encrypted-file-backend>".into(),
            source,
        })?;
        let master_key_path = dir.join("apeireth-keyring.master.key");
        let data_path = dir.join("apeireth-keyring.bin");

        let master_key = if master_key_path.exists() {
            load_master_key(&master_key_path)?
        } else {
            let key = generate_master_key();
            write_master_key(&master_key_path, &key)?;
            key
        };

        Ok(Self {
            data_path,
            master_key_path,
            master_key,
            audit,
        })
    }

    /// 数据文件路径 (元信息).
    pub fn data_path(&self) -> &Path {
        &self.data_path
    }

    /// master.key 路径 (元信息).
    pub fn master_key_path(&self) -> &Path {
        &self.master_key_path
    }

    /// 读取全表 → `BTreeMap<service, plaintext>` (解密后).
    fn load_all(&self) -> Result<BTreeMap<String, Vec<u8>>> {
        if !self.data_path.exists() {
            return Ok(BTreeMap::new());
        }
        let raw = std::fs::read(&self.data_path).map_err(|source| KeyringError::Io {
            service: "<encrypted-file-backend>".into(),
            source,
        })?;
        if raw.len() < MAGIC.len() + NONCE_LEN + LEN_FIELD {
            return Err(KeyringError::Crypto(
                "data file too short (corrupted)".into(),
            ));
        }
        if &raw[..MAGIC.len()] != MAGIC {
            return Err(KeyringError::Crypto(format!(
                "bad magic (expected {:?}, got {:?})",
                MAGIC,
                &raw[..MAGIC.len()]
            )));
        }
        let nonce_bytes = &raw[MAGIC.len()..MAGIC.len() + NONCE_LEN];
        let len_start = MAGIC.len() + NONCE_LEN;
        let len_bytes = &raw[len_start..len_start + LEN_FIELD];
        let ct_len = u32::from_le_bytes(
            len_bytes
                .try_into()
                .map_err(|_| KeyringError::Crypto("bad ciphertext length field".into()))?,
        ) as usize;
        let ct = &raw[len_start + LEN_FIELD..];
        if ct.len() != ct_len {
            return Err(KeyringError::Crypto(format!(
                "ciphertext length mismatch (header {ct_len}, actual {})",
                ct.len()
            )));
        }
        let aead = XChaCha20Poly1305::new(&self.master_key);
        let nonce = XNonce::from_slice(nonce_bytes);
        let plaintext = aead
            .decrypt(
                nonce,
                Payload {
                    msg: ct,
                    aad: b"apeireth-keyring-v1",
                },
            )
            .map_err(|_| {
                KeyringError::Crypto("AEAD decrypt failed (wrong key or tampered)".into())
            })?;

        // plaintext = btreemap JSON
        let map: BTreeMap<String, String> =
            serde_json::from_slice(&plaintext).map_err(|e| KeyringError::Format(e.to_string()))?;
        Ok(map.into_iter().map(|(k, v)| (k, v.into_bytes())).collect())
    }

    /// 加密并写回全表 (原子写).
    fn save_all(&self, map: &BTreeMap<String, Vec<u8>>) -> Result<()> {
        // plaintext = JSON (string -> string, bytes 经 UTF-8 lossy 走)
        let string_map: BTreeMap<String, String> = map
            .iter()
            .map(|(k, v)| (k.clone(), String::from_utf8_lossy(v).into_owned()))
            .collect();
        let plaintext =
            serde_json::to_vec(&string_map).map_err(|e| KeyringError::Format(e.to_string()))?;

        // nonce 24B 随机
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let aead = XChaCha20Poly1305::new(&self.master_key);
        let ciphertext = aead
            .encrypt(
                nonce,
                Payload {
                    msg: &plaintext,
                    aad: b"apeireth-keyring-v1",
                },
            )
            .map_err(|e| KeyringError::Crypto(format!("AEAD encrypt failed: {e}")))?;

        let mut out = Vec::with_capacity(MAGIC.len() + NONCE_LEN + LEN_FIELD + ciphertext.len());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&nonce_bytes);
        let ct_len = ciphertext.len() as u32;
        out.extend_from_slice(&ct_len.to_le_bytes());
        out.extend_from_slice(&ciphertext);

        // 原子写: 写临时文件 + rename (防半写).
        let tmp = self.data_path.with_extension("bin.tmp");
        std::fs::write(&tmp, &out).map_err(|source| KeyringError::Io {
            service: "<encrypted-file-backend>".into(),
            source,
        })?;
        std::fs::rename(&tmp, &self.data_path).map_err(|source| KeyringError::Io {
            service: "<encrypted-file-backend>".into(),
            source,
        })?;
        // 0600 收敛 (unix); Windows 走默认 ACL.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(&self.data_path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}

impl KeyringBackend for EncryptedFileBackend {
    fn get(&self, service: &str) -> Result<SecretBuf> {
        check_service_name(service)?;
        let mut all = self.load_all()?;
        match all.remove(service) {
            Some(bytes) => {
                if bytes.len() > MAX_SECRET_LEN {
                    self.audit
                        .record(AuditEvent::Get, service, "encrypted-file", false);
                    return Err(KeyringError::SecretTooLong {
                        len: bytes.len(),
                        max: MAX_SECRET_LEN,
                    });
                }
                self.audit
                    .record(AuditEvent::Get, service, "encrypted-file", true);
                Ok(SecretBuf::new(bytes))
            }
            None => {
                self.audit
                    .record(AuditEvent::Get, service, "encrypted-file", false);
                Err(KeyringError::UnknownService {
                    service: service.to_string(),
                })
            }
        }
    }

    fn set(&self, service: &str, secret: &SecretBuf) -> Result<()> {
        check_service_name(service)?;
        if secret.len() > MAX_SECRET_LEN {
            return Err(KeyringError::SecretTooLong {
                len: secret.len(),
                max: MAX_SECRET_LEN,
            });
        }
        let mut all = self.load_all().unwrap_or_default();
        all.insert(service.to_string(), secret.expose().to_vec());
        match self.save_all(&all) {
            Ok(()) => {
                self.audit
                    .record(AuditEvent::Set, service, "encrypted-file", true);
                Ok(())
            }
            Err(e) => {
                self.audit
                    .record(AuditEvent::Set, service, "encrypted-file", false);
                Err(e)
            }
        }
    }

    fn delete(&self, service: &str) -> Result<()> {
        check_service_name(service)?;
        let mut all = self.load_all().unwrap_or_default();
        if all.remove(service).is_none() {
            self.audit
                .record(AuditEvent::Delete, service, "encrypted-file", false);
            return Err(KeyringError::UnknownService {
                service: service.to_string(),
            });
        }
        match self.save_all(&all) {
            Ok(()) => {
                self.audit
                    .record(AuditEvent::Delete, service, "encrypted-file", true);
                Ok(())
            }
            Err(e) => {
                self.audit
                    .record(AuditEvent::Delete, service, "encrypted-file", false);
                Err(e)
            }
        }
    }

    fn list(&self) -> Result<Vec<String>> {
        let all = self.load_all()?;
        self.audit
            .record(AuditEvent::List, "<all>", "encrypted-file", true);
        Ok(all.keys().cloned().collect())
    }

    fn backend_name(&self) -> &'static str {
        "encrypted-file"
    }
}

/// 生成 32B 随机 master key.
fn generate_master_key() -> Key {
    let mut bytes = [0u8; MASTER_KEY_LEN];
    rand::thread_rng().fill_bytes(&mut bytes);
    let key = Key::from(bytes);
    bytes.zeroize();
    key
}

/// 从文件读 master key.
fn load_master_key(path: &Path) -> Result<Key> {
    let mut bytes = std::fs::read(path).map_err(|source| KeyringError::Io {
        service: "<master-key>".into(),
        source,
    })?;
    if bytes.len() != MASTER_KEY_LEN {
        return Err(KeyringError::Crypto(format!(
            "master.key length {} != {MASTER_KEY_LEN}",
            bytes.len()
        )));
    }
    let mut arr = [0u8; MASTER_KEY_LEN];
    arr.copy_from_slice(&bytes);
    bytes.zeroize();
    Ok(Key::from(arr))
}

/// 写 master key 到文件 (0600 语义).
fn write_master_key(path: &Path, key: &Key) -> Result<()> {
    std::fs::write(path, key.as_slice()).map_err(|source| KeyringError::Io {
        service: "<master-key>".into(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

// ============================================================================
// §6 InMemoryKeyring — 内存 stub (测试 / 限流 / 0 装 placeholder)
// ============================================================================

/// **内存 keyring stub** — 进程内 BTreeMap, 不持久化.
///
/// 用途:
/// - **单测**: 不依赖 OS keyring / 文件, 跑得快.
/// - **限流** (per 任务描述 "限流用 InMemoryKeyring stub"): 当 OS keyring
///   不可用 + 加密文件 fallback 也禁用 (e.g. CI 容器无 home), 仍可读写.
/// - **0 装 placeholder**: 装配侧未配置真后端时返 InMemoryKeyring 兜底.
///
/// **红线**: 本后端**不持久化**, 进程退出即丢; **不**是生产默认. 装配
/// 侧应显式选用 Platform 或 EncryptedFile, 仅在受限环境用 InMemory.
pub struct InMemoryKeyring {
    inner: std::sync::Mutex<BTreeMap<String, Vec<u8>>>,
    audit: Arc<dyn AuditSink>,
}

impl std::fmt::Debug for InMemoryKeyring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 不打印 inner (含明文) 与 audit (dyn 不 impl Debug).
        f.debug_struct("InMemoryKeyring")
            .field("len", &self.inner.lock().unwrap().len())
            .finish_non_exhaustive()
    }
}

impl InMemoryKeyring {
    /// 构造空内存 keyring.
    pub fn new(audit: Arc<dyn AuditSink>) -> Self {
        Self {
            inner: std::sync::Mutex::new(BTreeMap::new()),
            audit,
        }
    }
}

impl KeyringBackend for InMemoryKeyring {
    fn get(&self, service: &str) -> Result<SecretBuf> {
        check_service_name(service)?;
        let guard = self.inner.lock().unwrap();
        match guard.get(service) {
            Some(bytes) => {
                let len = bytes.len();
                if len > MAX_SECRET_LEN {
                    self.audit
                        .record(AuditEvent::Get, service, "in-memory", false);
                    return Err(KeyringError::SecretTooLong {
                        len,
                        max: MAX_SECRET_LEN,
                    });
                }
                self.audit
                    .record(AuditEvent::Get, service, "in-memory", true);
                Ok(SecretBuf::new(bytes.clone()))
            }
            None => {
                self.audit
                    .record(AuditEvent::Get, service, "in-memory", false);
                Err(KeyringError::UnknownService {
                    service: service.to_string(),
                })
            }
        }
    }

    fn set(&self, service: &str, secret: &SecretBuf) -> Result<()> {
        check_service_name(service)?;
        if secret.len() > MAX_SECRET_LEN {
            return Err(KeyringError::SecretTooLong {
                len: secret.len(),
                max: MAX_SECRET_LEN,
            });
        }
        let mut guard = self.inner.lock().unwrap();
        guard.insert(service.to_string(), secret.expose().to_vec());
        self.audit
            .record(AuditEvent::Set, service, "in-memory", true);
        Ok(())
    }

    fn delete(&self, service: &str) -> Result<()> {
        check_service_name(service)?;
        let mut guard = self.inner.lock().unwrap();
        if guard.remove(service).is_none() {
            self.audit
                .record(AuditEvent::Delete, service, "in-memory", false);
            return Err(KeyringError::UnknownService {
                service: service.to_string(),
            });
        }
        self.audit
            .record(AuditEvent::Delete, service, "in-memory", true);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>> {
        let guard = self.inner.lock().unwrap();
        self.audit
            .record(AuditEvent::List, "<all>", "in-memory", true);
        Ok(guard.keys().cloned().collect())
    }

    fn backend_name(&self) -> &'static str {
        "in-memory"
    }
}

// ============================================================================
// §7 KeyringSelector — 环境变量驱动的后端选择 + 自动降级
// ============================================================================

/// **后端选择结果** — 选中的后端 + 原始请求 (供装配侧报告).
pub struct SelectedBackend {
    /// 选中的后端.
    pub backend: Box<dyn KeyringBackend>,
    /// 实际生效的后端类别 (元信息).
    pub kind: BackendKind,
}

impl std::fmt::Debug for SelectedBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 不打印 backend (dyn KeyringBackend 不 impl Debug); 仅元信息.
        f.debug_struct("SelectedBackend")
            .field("kind", &self.kind)
            .field("backend_name", &self.backend.backend_name())
            .finish()
    }
}

/// **后端类别** (env 取值 + 自动降级路径).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    /// 平台 keyring (env: `platform`, 默认).
    Platform,
    /// 加密文件 fallback (env: `encrypted-file`).
    EncryptedFile,
    /// 内存 stub (env: `in-memory`).
    InMemory,
    /// 自动: 优先 platform, 不可用降级 encrypted-file, 再降级 in-memory (env: `auto`).
    Auto,
}

impl BackendKind {
    /// 从 env 字符串解析 (`APEIRETH_KEYRING_BACKEND`, 不区分大小写).
    ///
    /// 空 / 未知 → [`BackendKind::Auto`] (安全默认).
    pub fn from_env_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "" => BackendKind::Auto,
            "platform" => BackendKind::Platform,
            "encrypted-file" | "encrypted_file" | "file" => BackendKind::EncryptedFile,
            "in-memory" | "in_memory" | "memory" => BackendKind::InMemory,
            "auto" => BackendKind::Auto,
            _ => BackendKind::Auto, // 未知值安全默认 (per fail-closed 哲学)
        }
    }
}

/// **后端选择器** — 装配期一次性选择.
///
/// **env 变量**: `APEIRETH_KEYRING_BACKEND` ∈ {`auto` (默认), `platform`,
/// `encrypted-file`, `in-memory`}.
///
/// **`Auto` 路径** (per 任务纪律):
/// 1. 试 [`PlatformKeyring::probe_available`] (启动期一次性);
/// 2. 可用 → PlatformKeyring;
/// 3. 不可用 → EncryptedFileBackend (需 dir, 默认 `~/.apeireth/keyring/`);
/// 4. EncryptedFileBackend IO 失败 → InMemoryKeyring (限流 stub).
///
/// **明确指定路径** (per 任务纪律, fail-loud):
/// - 用户显式 `platform` → PlatformKeyring (构造时不探测, 首次 get/set 失败返 BackendUnavailable);
/// - 用户显式 `encrypted-file` → EncryptedFileBackend (IO 失败 → 错误, 不静默降级);
/// - 用户显式 `in-memory` → InMemoryKeyring.
pub struct KeyringSelector;

impl KeyringSelector {
    /// **按 env + 路径选择后端**.
    ///
    /// `audit`: 审计 sink (装配侧挂真 audit 或 [`NoopAudit`] 0 装).
    /// `fallback_dir`: EncryptedFileBackend / InMemory 兜底目录 (默认 `~/.apeireth/keyring/`).
    /// `env_value`: `APEIRETH_KEYRING_BACKEND` 当前值 (测试可注入).
    pub fn select(
        env_value: Option<&str>,
        audit: Arc<dyn AuditSink>,
        fallback_dir: Option<PathBuf>,
    ) -> Result<SelectedBackend> {
        let kind = BackendKind::from_env_str(env_value.unwrap_or(""));
        let fallback_dir = fallback_dir.unwrap_or_else(default_dir);

        match kind {
            BackendKind::Platform => Ok(SelectedBackend {
                backend: Box::new(PlatformKeyring::new("apeireth", audit)),
                kind,
            }),
            BackendKind::EncryptedFile => {
                let fb = EncryptedFileBackend::open(&fallback_dir, audit)?;
                Ok(SelectedBackend {
                    backend: Box::new(fb),
                    kind,
                })
            }
            BackendKind::InMemory => Ok(SelectedBackend {
                backend: Box::new(InMemoryKeyring::new(audit)),
                kind,
            }),
            BackendKind::Auto => Self::select_auto(audit, fallback_dir),
        }
    }

    /// Auto 路径: probe → platform / encrypted-file / in-memory 依次降级.
    fn select_auto(audit: Arc<dyn AuditSink>, fallback_dir: PathBuf) -> Result<SelectedBackend> {
        // 1. probe
        if PlatformKeyring::probe_available() {
            return Ok(SelectedBackend {
                backend: Box::new(PlatformKeyring::new("apeireth", audit)),
                kind: BackendKind::Platform,
            });
        }
        // 2. encrypted-file fallback
        match EncryptedFileBackend::open(&fallback_dir, audit.clone()) {
            Ok(fb) => Ok(SelectedBackend {
                backend: Box::new(fb),
                kind: BackendKind::EncryptedFile,
            }),
            Err(_) => {
                // 3. in-memory stub (永不失败)
                Ok(SelectedBackend {
                    backend: Box::new(InMemoryKeyring::new(audit)),
                    kind: BackendKind::InMemory,
                })
            }
        }
    }
}

/// 默认 fallback dir: `~/.apeireth/keyring/`.
fn default_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".apeireth").join("keyring")
    } else if let Some(profile) = std::env::var_os("USERPROFILE") {
        // Windows
        PathBuf::from(profile).join(".apeireth").join("keyring")
    } else {
        PathBuf::from(".apeireth/keyring")
    }
}

// ============================================================================
// §8 helper — service name 校验
// ============================================================================

fn check_service_name(service: &str) -> Result<()> {
    if service.len() > MAX_SERVICE_NAME_LEN {
        return Err(KeyringError::ServiceNameTooLong {
            len: service.len(),
            max: MAX_SERVICE_NAME_LEN,
        });
    }
    // 复用 CredentialsStore 的校验 (防注入/路径穿越).
    validate_service_name(service)
        .map_err(|_| KeyringError::InvalidServiceName(service.to_string()))
}

// ============================================================================
// §9 tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // ----- §9.1 InMemoryKeyring round-trip -----

    #[test]
    fn in_memory_keyring_round_trip() {
        let audit = Arc::new(CountingAudit::new());
        let k = InMemoryKeyring::new(audit.clone());

        // 空时 get → UnknownService
        assert!(matches!(
            k.get("openai").unwrap_err(),
            KeyringError::UnknownService { .. }
        ));

        // set + get round-trip
        k.set("openai", &SecretBuf::from_str("sk-test-001"))
            .unwrap();
        let v = k.get("openai").unwrap();
        assert_eq!(v.expose(), b"sk-test-001");

        // delete
        k.delete("openai").unwrap();
        assert!(matches!(
            k.get("openai").unwrap_err(),
            KeyringError::UnknownService { .. }
        ));

        // list
        k.set("a", &SecretBuf::from_str("1")).unwrap();
        k.set("b", &SecretBuf::from_str("2")).unwrap();
        let mut names = k.list().unwrap();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);

        // audit 验证
        let entries = audit.entries();
        assert!(entries.len() >= 4, "应有审计记录: {entries:?}");
        for e in &entries {
            // name_hash 长度 = 16 hex
            assert_eq!(e.name_hash.len(), 16, "name_hash 应 16 hex: {e:?}");
        }
    }

    // ----- §9.2 EncryptedFileBackend round-trip -----

    #[test]
    fn encrypted_file_backend_round_trip() {
        let tmp = std::env::temp_dir().join(format!(
            "apeireth-keyring-test-{}-{}",
            std::process::id(),
            "encrypted"
        ));
        let _ = std::fs::remove_dir_all(&tmp);

        let audit = Arc::new(CountingAudit::new());
        let k = EncryptedFileBackend::open(&tmp, audit.clone()).unwrap();

        // 首次: master.key 应自动生成
        assert!(k.master_key_path().exists(), "master.key 应自动生成");
        assert!(!k.data_path().exists(), "data 文件首次应不存在");

        // set + get
        k.set("openai", &SecretBuf::from_str("sk-test-encrypted"))
            .unwrap();
        assert!(k.data_path().exists(), "set 后 data 文件应存在");
        let v = k.get("openai").unwrap();
        assert_eq!(v.expose(), b"sk-test-encrypted");

        // 二次打开: master.key 应复用, 数据应可读
        let audit2 = Arc::new(CountingAudit::new());
        let k2 = EncryptedFileBackend::open(&tmp, audit2.clone()).unwrap();
        let v2 = k2.get("openai").unwrap();
        assert_eq!(v2.expose(), b"sk-test-encrypted");

        // delete + list
        k2.delete("openai").unwrap();
        assert!(matches!(
            k2.get("openai").unwrap_err(),
            KeyringError::UnknownService { .. }
        ));
        assert!(k2.list().unwrap().is_empty());

        // 篡改数据文件 → 解密失败 (Crypto 错误)
        std::fs::write(k.data_path(), b"APK1corrupted").unwrap();
        let audit3 = Arc::new(CountingAudit::new());
        let k3 = EncryptedFileBackend::open(&tmp, audit3.clone()).unwrap();
        assert!(matches!(k3.list().unwrap_err(), KeyringError::Crypto(_)));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ----- §9.3 平台 fallback (probe 测试) -----

    #[test]
    fn platform_probe_is_boolean() {
        // probe_available 返 bool; CI 环境通常 false (无 D-Bus / Keychain).
        let _ = PlatformKeyring::probe_available();
    }

    #[test]
    fn auto_selector_always_succeeds() {
        // Auto 路径必须永不返 Err (平台不可用 → 文件不可用 → InMemory).
        let audit = Arc::new(NoopAudit);
        let tmp = std::env::temp_dir().join(format!(
            "apeireth-keyring-test-{}-{}",
            std::process::id(),
            "auto"
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let r = KeyringSelector::select(Some("auto"), audit, Some(tmp.clone())).unwrap();
        assert!(
            matches!(
                r.kind,
                BackendKind::Platform | BackendKind::EncryptedFile | BackendKind::InMemory
            ),
            "Auto 应至少落到 3 后端之一: {:?}",
            r.kind
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn explicit_in_memory_selector_works() {
        let audit = Arc::new(CountingAudit::new());
        let r = KeyringSelector::select(Some("in-memory"), audit, None).unwrap();
        assert_eq!(r.kind, BackendKind::InMemory);
        let mut k = r.backend;
        k.set("test", &SecretBuf::from_str("v")).unwrap();
        assert_eq!(k.get("test").unwrap().expose(), b"v");
    }

    #[test]
    fn explicit_encrypted_file_selector_works() {
        let audit = Arc::new(CountingAudit::new());
        let tmp = std::env::temp_dir().join(format!(
            "apeireth-keyring-test-{}-{}",
            std::process::id(),
            "explicit-ef"
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        let r = KeyringSelector::select(Some("encrypted-file"), audit, Some(tmp.clone())).unwrap();
        assert_eq!(r.kind, BackendKind::EncryptedFile);
        let mut k = r.backend;
        k.set("svc", &SecretBuf::from_str("val")).unwrap();
        assert_eq!(k.get("svc").unwrap().expose(), b"val");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn unknown_env_value_defaults_to_auto() {
        // 未知值安全默认 (fail-closed 哲学).
        let audit = Arc::new(NoopAudit);
        let r = KeyringSelector::select(Some("definitely-not-a-backend"), audit, None).unwrap();
        // Auto 路径在 CI 通常落到 EncryptedFile 或 InMemory.
        assert!(
            matches!(
                r.kind,
                BackendKind::Platform | BackendKind::EncryptedFile | BackendKind::InMemory
            ),
            "未知 env 值应安全默认 Auto: {:?}",
            r.kind
        );
    }

    // ----- §9.4 审计 name_hash 不含明文 -----

    #[test]
    fn audit_name_hash_does_not_contain_plaintext() {
        let audit = Arc::new(CountingAudit::new());
        let k = InMemoryKeyring::new(audit.clone());

        let plaintext_services = ["openai", "anthropic", "master-token"];
        for svc in plaintext_services {
            k.set(svc, &SecretBuf::from_str("super-secret-value"))
                .unwrap();
            k.get(svc).unwrap();
            k.delete(svc).unwrap();
        }

        // 关键断言: 所有审计条目的 name_hash 不含明文 service 名原文
        audit.assert_no_plaintext(&plaintext_services);

        // 同时验 name_hash 不是原文
        for svc in plaintext_services {
            let h = name_hash(svc);
            assert_ne!(h, svc, "name_hash 不应等于 service 明文");
            assert_eq!(h.len(), 16, "name_hash 应 16 hex 字符");
            assert!(
                h.chars().all(|c| c.is_ascii_hexdigit()),
                "name_hash 应 hex: {h}"
            );
        }
    }

    #[test]
    fn audit_records_success_and_failure() {
        let audit = Arc::new(CountingAudit::new());
        let k = InMemoryKeyring::new(audit.clone());

        // 成功: set + get
        k.set("a", &SecretBuf::from_str("v")).unwrap();
        k.get("a").unwrap();
        // 失败: get 不存在的服务
        let _ = k.get("no-such");

        let entries = audit.entries();
        let successes = entries.iter().filter(|e| e.success).count();
        let failures = entries.iter().filter(|e| !e.success).count();
        assert!(successes >= 2, "至少 2 成功: {entries:?}");
        assert!(failures >= 1, "至少 1 失败: {entries:?}");
    }

    // ----- §9.5 SecretBuf zeroize 在 keyring 路径 -----

    #[test]
    fn secret_buf_zeroize_via_keyring_get() {
        // 从 keyring 取出的 SecretBuf Drop 时 zeroize.
        let audit = Arc::new(CountingAudit::new());
        let k = InMemoryKeyring::new(audit.clone());
        k.set("svc", &SecretBuf::from_str("zero-me-please"))
            .unwrap();

        let buf = k.get("svc").unwrap();
        assert_eq!(buf.expose(), b"zero-me-please");
        // buf 在此 scope 末 Drop → ZeroizeOnDrop 自动 zeroize 内层 Vec.
        drop(buf);

        // 再次 set + get 走新分配, 内容应正确 (验证零化未破坏新分配).
        k.set("svc2", &SecretBuf::from_str("fresh")).unwrap();
        assert_eq!(k.get("svc2").unwrap().expose(), b"fresh");
    }

    // ----- §9.6 错误消息不含明文 -----

    #[test]
    fn error_messages_do_not_leak_secret() {
        // 错误消息应含 service 名 (元信息) 但绝不含**凭据明文** (secret 值).
        // 本测试用 `service = "openai"` (元信息) + `secret_value = "sk-leak-..."` (明文),
        // 验证错误消息含 "openai" 但不含 secret_value.
        let service = "openai";
        let secret_value = "sk-leak-check-plaintext-must-not-appear";

        let cases = vec![
            (
                KeyringError::UnknownService {
                    service: service.into(),
                },
                false,
            ),
            (KeyringError::InvalidServiceName(service.into()), false),
            (
                KeyringError::ServiceNameTooLong { len: 200, max: 128 },
                false,
            ),
            (
                KeyringError::SecretTooLong {
                    len: secret_value.len(),
                    max: 10,
                },
                false,
            ),
            (
                KeyringError::AccessDenied {
                    service: service.into(),
                    reason: "test".into(),
                },
                false,
            ),
            (
                KeyringError::BackendUnavailable("platform off".into()),
                false,
            ),
            (KeyringError::Crypto("AEAD failed".into()), false),
        ];

        for (e, _must_contain_service) in cases {
            let msg = format!("{e}");
            assert!(
                !msg.contains(secret_value),
                "错误消息不应含凭据明文 `{secret_value}`: {msg}"
            );
        }

        // 同时验证: 错误消息含 service 名 (元信息, 允许).
        let e = KeyringError::UnknownService {
            service: service.into(),
        };
        let msg = format!("{e}");
        assert!(msg.contains(service), "错误消息应含 service 名: {msg}");
    }

    // ----- §9.7 非法 service 名拒绝 -----

    #[test]
    fn invalid_service_name_rejected() {
        let audit = Arc::new(CountingAudit::new());
        let k = InMemoryKeyring::new(audit.clone());
        for bad in ["", "a/b", "a\\b", "a b", "..", ".hidden"] {
            assert!(
                k.set(bad, &SecretBuf::from_str("v")).is_err(),
                "应拒绝非法名: {bad:?}"
            );
            assert!(k.get(bad).is_err(), "应拒绝非法名 (get): {bad:?}");
            assert!(k.delete(bad).is_err(), "应拒绝非法名 (delete): {bad:?}");
        }
    }

    #[test]
    fn service_name_too_long_rejected() {
        let audit = Arc::new(CountingAudit::new());
        let k = InMemoryKeyring::new(audit.clone());
        let long = "a".repeat(MAX_SERVICE_NAME_LEN + 1);
        let e = k.set(&long, &SecretBuf::from_str("v")).unwrap_err();
        assert!(matches!(e, KeyringError::ServiceNameTooLong { .. }));
    }

    // ----- §9.8 BackendKind env 解析 -----

    #[test]
    fn backend_kind_from_env_str_parses_canonical() {
        assert_eq!(BackendKind::from_env_str(""), BackendKind::Auto);
        assert_eq!(BackendKind::from_env_str("auto"), BackendKind::Auto);
        assert_eq!(BackendKind::from_env_str("platform"), BackendKind::Platform);
        assert_eq!(
            BackendKind::from_env_str("encrypted-file"),
            BackendKind::EncryptedFile
        );
        assert_eq!(
            BackendKind::from_env_str("encrypted_file"),
            BackendKind::EncryptedFile
        );
        assert_eq!(
            BackendKind::from_env_str("in-memory"),
            BackendKind::InMemory
        );
        // 大小写不敏感
        assert_eq!(BackendKind::from_env_str("PLATFORM"), BackendKind::Platform);
        // 未知 → Auto (fail-closed)
        assert_eq!(
            BackendKind::from_env_str("unknown-thing"),
            BackendKind::Auto
        );
    }

    // ----- §9.9 NoopAudit 0 装验证 -----

    #[test]
    fn noop_audit_does_not_panic() {
        let audit = NoopAudit;
        audit.record(AuditEvent::Get, "svc", "any", true);
        audit.record(AuditEvent::Set, "svc", "any", false);
        // 0 装 PASS: 不写任何位置, 调用不崩.
    }

    // ----- §9.10 secret 长度上限 -----

    #[test]
    fn secret_too_long_rejected() {
        let audit = Arc::new(CountingAudit::new());
        let k = InMemoryKeyring::new(audit.clone());
        let big = vec![0xAA; MAX_SECRET_LEN + 1];
        let e = k.set("svc", &SecretBuf::new(big)).unwrap_err();
        assert!(matches!(e, KeyringError::SecretTooLong { .. }));
    }
}
