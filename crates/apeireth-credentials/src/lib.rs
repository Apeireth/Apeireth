//! # apeireth-credentials — 统一凭据存取层 (TP3/N21 + TP20-S3)
//!
//! **定位** (§10 最后一公里任务包 TP3, 装配主链第一环): 各插件/工具目前各读 env,
//! 本 crate 提供**按服务名读写凭据**的统一层:
//!
//! - [`CredentialsStore`] — 统一存取接口 (get/set/delete/list/contains)
//! - [`FileCredentialsStore`] — 文件形态后端 (JSON 单文件, 权限 600 语义)
//! - [`SecretString`] — 明文脱敏载体 (Debug/Display 恒为 `[REDACTED len=N]`)
//! - [`CredentialGate`] / [`GatedCredentialsStore`] — 权限洋葱衔接审批门
//!
//! **TP20-S3 (塞缝批, 安全凭证)** 增量:
//!
//! - [`KeyringBackend`] — 统一 keyring 后端 trait (get/set/delete/list)
//! - [`PlatformKeyring`] — 平台 keyring (Linux Secret Service / macOS / Windows)
//! - [`EncryptedFileBackend`] — chacha20poly1305 AEAD + master.key fallback
//! - [`InMemoryKeyring`] — 内存 stub (单测 / 限流 / 0 装 placeholder)
//! - [`KeyringSelector`] — `APEIRETH_KEYRING_BACKEND` env 选择 + 自动降级
//! - [`SecretBuf`] — `Drop` zeroize 字节容器 (用于 KeyringBackend trait)
//! - [`AuditSink`] / [`CountingAudit`] / [`NoopAudit`] — 审计 (name_hash 不含明文)
//!
//! ## 安全红线 (任务纪律)
//!
//! 1. **明文不入日志/错误消息**: `SecretString` / `SecretBuf` 覆写 `Debug`/`Display`,
//!    [`CredentialsError`] / [`KeyringError`] 各变体只含服务名等元信息 (均有回归测试)。
//! 2. **高危凭据走审批链**: master token 类服务 (见 [`DEFAULT_HIGH_RISK_SERVICES`])
//!    的读写须经 [`CredentialGate`] 放行 — 复用 sovereignty/constraint 的
//!    master token 批准语义 (不改本体), companion 装配侧挂真审批链。
//!    未挂时默认 [`DenyAllGate`] fail-closed: 高危一律拒, AI 不接触明文 token。
//! 3. **审计 name_hash 不含明文**: 每次 keyring 访问写审计,
//!    `name_hash = SHA-256(service)[:16] hex` 不可逆, 同 service 同 hash (可关联).
//!
//! ## 0 假装边界 (诚实标注)
//!
//! - **本层是凭据存取抽象, 不是加密保险库**: 文件后端为**明文静态存储**,
//!   靠 OS 文件权限 (unix 600) 收敛访问。加密静态存储 (OS keyring/KMS/age)
//!   属后续层, 接入时实现 [`CredentialsStore`] 换后端即可, 接口不变。
//! - **SecretString 非内存安全容器**: 未做 zeroize/mlock, 只保证不泄漏到
//!   输出通道。内存级擦除属 TP20-S3 新增的 `SecretBuf` (Drop zeroize)。
//! - **审计日志不持久化**: 默认实现 `NoopAudit`, 装配侧可挂真 audit sink
//!   (telemetry / 经验库), 此处为 trait 口 (0 装 PASS 标注)。
//! - **平台 keyring crate 3.6 不支持 list**: `PlatformKeyring::list` 返 Backend
//!   错误, 上层应走 `EncryptedFileBackend` 或 `InMemoryKeyring` 兜底 (0 假装)。
//!
//! ## 消费方声明 (N18 规范)
//!
//! 消费方 = companion 装配侧 (§10 装配主链, 随 N17 工具装配统一接入):
//! serve/装配时构造 `GatedCredentialsStore<FileCredentialsStore, 真审批门>`,
//! 或 `KeyringSelector::select(...)` 选 keyring 后端, 工具/插件按服务名取凭据替代各读 env.
//! 当前为 **trait 口 + 后端就绪**, 装配挂接随主链后续环实施 (0 装标注)。

#![forbid(unsafe_code)]

pub mod error;
pub mod gate;
pub mod keyring;
pub mod secret;
pub mod store;

pub use error::{CredentialsError, Result};
pub use gate::{
    AllowAllGate, CredentialGate, CredentialOp, DenyAllGate, GateDecision, GatedCredentialsStore,
    DEFAULT_HIGH_RISK_SERVICES,
};
pub use keyring::{
    name_hash, AuditEntry, AuditEvent, AuditSink, BackendKind, CountingAudit, EncryptedFileBackend,
    InMemoryKeyring, KeyringBackend, KeyringError, KeyringSelector, NoopAudit, PlatformKeyring,
    SelectedBackend, MAX_SECRET_LEN, MAX_SERVICE_NAME_LEN,
};
pub use secret::{mask_for_display, redact_len, SecretBuf, SecretString};
pub use store::{validate_service_name, CredentialsStore, FileCredentialsStore};

/// 借 VCP 字段数 (编译期自审锚, 对照 apeireth-tool-approval 惯例)。
///
/// 本 crate 为原生新增, 无 VCP 借鉴字段 — 值为 0, 显式标注非遗漏。
pub const BORROWED_VCP_FIELDS: usize = 0;

const _: () = assert!(BORROWED_VCP_FIELDS == 0, "原生 crate, 借鉴字段应为 0");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_reachable() {
        // 顶层导出可达性冒烟
        let _ = redact_len(3);
        let _ = mask_for_display("abcdefgh");
        assert!(DEFAULT_HIGH_RISK_SERVICES.contains(&"master"));
        assert_eq!(BORROWED_VCP_FIELDS, 0);
        // TP20-S3: keyring 顶层 API 可达
        let _ = MAX_SECRET_LEN;
        let _ = MAX_SERVICE_NAME_LEN;
        let _ = name_hash("openai");
    }

    /// **验收项"读写/未知名报错/脱敏"端到端**: 文件后端 + 审批门 + 脱敏一体冒烟。
    #[test]
    fn end_to_end_smoke() {
        let dir =
            std::env::temp_dir().join(format!("apeireth-credentials-e2e-{}", std::process::id()));
        let store = FileCredentialsStore::new(dir.join("creds.json")).unwrap();

        // 普通凭据: 读写自由
        store
            .set("openai", SecretString::new("sk-e2e-001"))
            .unwrap();
        let v = store.get("openai").unwrap();
        assert_eq!(v.expose(), "sk-e2e-001");
        // 脱敏输出不泄漏
        assert!(!format!("{v:?}").contains("sk-e2e-001"));

        // 未知名报错
        assert!(matches!(
            store.get("no-such-service").unwrap_err(),
            CredentialsError::UnknownService(_)
        ));

        // 高危凭据: 未挂审批链 → fail-closed
        store.set("master", SecretString::new("mt-e2e")).unwrap();
        let gated = GatedCredentialsStore::new(store, DenyAllGate);
        assert!(matches!(
            gated.get("master").unwrap_err(),
            CredentialsError::ApprovalRequired { .. }
        ));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **TP20-S3 验收**: KeyringBackend + SecretBuf + Audit 一体冒烟。
    #[test]
    fn keyring_end_to_end_smoke() {
        use std::sync::Arc;
        let audit = Arc::new(CountingAudit::new());
        let k = InMemoryKeyring::new(audit.clone());

        // 写 + 读 + 列 + 删
        k.set("svc", &SecretBuf::from_str("value")).unwrap();
        let v = k.get("svc").unwrap();
        assert_eq!(v.expose(), b"value");
        assert_eq!(k.list().unwrap(), vec!["svc".to_string()]);
        k.delete("svc").unwrap();
        assert!(matches!(
            k.get("svc").unwrap_err(),
            KeyringError::UnknownService { .. }
        ));

        // 审计: 至少 4 条 (set/get/list/delete), name_hash 16 hex
        let entries = audit.entries();
        assert!(entries.len() >= 4, "至少 4 审计: {entries:?}");
        for e in &entries {
            assert_eq!(e.name_hash.len(), 16);
            // 审计不含明文
            assert!(!e.name_hash.contains("svc"));
        }
    }
}
