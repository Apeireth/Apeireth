//! **TP3/N21 / 权限洋葱衔接 — 高危凭据审批门 (trait 口, 0 装)**
//!
//! **衔接语义 (任务纪律 ①)**: master token 一类的**高危凭据**, 其读写必须经过
//! 既有审批链 — 高危操作需**主人批准**, AI 不接触明文 token。sovereignty /
//! constraint 已有该语义 (master token 比对不落日志, 见 companion::principles),
//! 本层**复用不改本体**: 只定义 [`CredentialGate`] trait 口, 由 companion 装配侧
//! 把真审批链 (master token 批准) 挂进来。
//!
//! **0 装边界 (诚实标注)**: 本模块只提供**门控抽象 + 装饰器**, 默认实现
//! [`DenyAllGate`] 对高危服务一律拒绝 (fail-closed 红线)。真审批链挂接在
//! companion 装配层, 此处为 trait 口, 如实标注。

use crate::error::{CredentialsError, Result};
use crate::secret::SecretString;
use crate::store::CredentialsStore;

/// 凭据操作类别 (审批粒度)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialOp {
    /// 读取明文。
    Read,
    /// 写入/覆盖。
    Write,
    /// 删除。
    Delete,
}

/// 审批决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    /// 放行 (已获主人批准)。
    Allow,
    /// 拒绝 (未获批准)。
    Deny,
}

/// **权限洋葱审批门 trait 口** (companion 装配侧挂真审批链)。
///
/// 实现方应把 `(service, op)` 提交既有审批链 (master token 批准);
/// 本 crate 不依赖 sovereignty/constraint 本体 (复用不改)。
pub trait CredentialGate: Send + Sync {
    /// 判定某服务的某操作是否放行。
    fn decide(&self, service: &str, op: CredentialOp) -> GateDecision;

    /// 该服务是否属高危 (需审批)。默认按 [`DEFAULT_HIGH_RISK_SERVICES`]。
    fn is_high_risk(&self, service: &str) -> bool {
        DEFAULT_HIGH_RISK_SERVICES.iter().any(|s| *s == service)
    }
}

/// 默认高危服务名单 (master token 类)。
///
/// 与 companion env `APEIRETH_MASTER_TOKEN` / master token 批准语义对齐。
pub const DEFAULT_HIGH_RISK_SERVICES: &[&str] = &["master", "master_token", "master-token"];

/// **fail-closed 默认门**: 高危一律拒绝 (未挂真审批链时的红线默认)。
///
/// 非高危服务放行 (普通凭据读写不阻塞)。
#[derive(Debug, Default, Clone, Copy)]
pub struct DenyAllGate;

impl CredentialGate for DenyAllGate {
    fn decide(&self, service: &str, _op: CredentialOp) -> GateDecision {
        if self.is_high_risk(service) {
            GateDecision::Deny
        } else {
            GateDecision::Allow
        }
    }
}

/// 放行门 (测试/演示用; 生产勿用于高危)。
#[derive(Debug, Default, Clone, Copy)]
pub struct AllowAllGate;

impl CredentialGate for AllowAllGate {
    fn decide(&self, _service: &str, _op: CredentialOp) -> GateDecision {
        GateDecision::Allow
    }
}

/// **门控装饰器**: 在 [`CredentialsStore`] 外包一层审批门。
///
/// 高危服务的 `get`/`set`/`delete` 先过 [`CredentialGate::decide`];
/// 未获放行 → [`CredentialsError::ApprovalRequired`] (不落明文)。
pub struct GatedCredentialsStore<S: CredentialsStore, G: CredentialGate> {
    inner: S,
    gate: G,
}

impl<S: CredentialsStore, G: CredentialGate> GatedCredentialsStore<S, G> {
    /// 包装存储与门。
    pub fn new(inner: S, gate: G) -> Self {
        Self { inner, gate }
    }

    /// 引用内层存储 (如 list/contains 等非明文操作直通)。
    pub fn inner(&self) -> &S {
        &self.inner
    }

    fn check(&self, service: &str, op: CredentialOp) -> Result<()> {
        if self.gate.is_high_risk(service) && self.gate.decide(service, op) != GateDecision::Allow {
            return Err(CredentialsError::ApprovalRequired {
                service: service.to_string(),
            });
        }
        Ok(())
    }
}

impl<S: CredentialsStore, G: CredentialGate> CredentialsStore for GatedCredentialsStore<S, G> {
    fn get(&self, service: &str) -> Result<SecretString> {
        self.check(service, CredentialOp::Read)?;
        self.inner.get(service)
    }

    fn set(&self, service: &str, secret: SecretString) -> Result<()> {
        self.check(service, CredentialOp::Write)?;
        self.inner.set(service, secret)
    }

    fn delete(&self, service: &str) -> Result<()> {
        self.check(service, CredentialOp::Delete)?;
        self.inner.delete(service)
    }

    fn list(&self) -> Result<Vec<String>> {
        // 列表只出服务名 (元信息), 不触明文, 直通。
        self.inner.list()
    }

    fn contains(&self, service: &str) -> Result<bool> {
        self.inner.contains(service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::FileCredentialsStore;

    fn tmp_store(name: &str) -> FileCredentialsStore {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-credentials-gate-{}-{}",
            std::process::id(),
            name
        ));
        FileCredentialsStore::new(dir.join("creds.json")).expect("store")
    }

    #[test]
    fn deny_all_gate_blocks_high_risk_read() {
        let store = tmp_store("gate-deny-read");
        // 预置高危凭据 (经 AllowAll 门写入)
        GatedCredentialsStore::new(&NoopStore, AllowAllGate);
        store.set("master", SecretString::new("mt-secret")).unwrap();

        let gated = GatedCredentialsStore::new(store, DenyAllGate);
        let e = gated.get("master").unwrap_err();
        assert!(matches!(e, CredentialsError::ApprovalRequired { .. }));
        // 明文不落错误
        assert!(!format!("{e}").contains("mt-secret"));
        let _ = std::fs::remove_dir_all(tmp_path("gate-deny-read"));
    }

    #[test]
    fn deny_all_gate_allows_normal_service() {
        let store = tmp_store("gate-normal");
        store.set("openai", SecretString::new("sk-x")).unwrap();
        let gated = GatedCredentialsStore::new(store, DenyAllGate);
        assert_eq!(gated.get("openai").unwrap().expose(), "sk-x");
        let _ = std::fs::remove_dir_all(tmp_path("gate-normal"));
    }

    #[test]
    fn allow_all_gate_permits_high_risk() {
        let store = tmp_store("gate-allow");
        store.set("master", SecretString::new("mt")).unwrap();
        let gated = GatedCredentialsStore::new(store, AllowAllGate);
        assert_eq!(gated.get("master").unwrap().expose(), "mt");
        let _ = std::fs::remove_dir_all(tmp_path("gate-allow"));
    }

    #[test]
    fn high_risk_write_blocked_by_deny_gate() {
        let store = tmp_store("gate-write");
        let gated = GatedCredentialsStore::new(store, DenyAllGate);
        let e = gated
            .set("master_token", SecretString::new("x"))
            .unwrap_err();
        assert!(matches!(e, CredentialsError::ApprovalRequired { .. }));
        let _ = std::fs::remove_dir_all(tmp_path("gate-write"));
    }

    #[test]
    fn list_bypasses_gate_names_only() {
        let store = tmp_store("gate-list");
        store.set("master", SecretString::new("mt")).unwrap();
        store.set("openai", SecretString::new("sk")).unwrap();
        let gated = GatedCredentialsStore::new(store, DenyAllGate);
        let mut names = gated.list().unwrap();
        names.sort();
        assert_eq!(names, vec!["master", "openai"]);
        let _ = std::fs::remove_dir_all(tmp_path("gate-list"));
    }

    #[test]
    fn default_high_risk_names() {
        let gate = DenyAllGate;
        assert!(gate.is_high_risk("master"));
        assert!(gate.is_high_risk("master_token"));
        assert!(gate.is_high_risk("master-token"));
        assert!(!gate.is_high_risk("openai"));
    }

    // 辅助: 重算 tmp 目录路径 (与 tmp_store 一致)。
    fn tmp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "apeireth-credentials-gate-{}-{}",
            std::process::id(),
            name
        ))
    }

    // 占位存储 (仅用于类型检查, 不参与断言)。
    struct NoopStore;
    impl CredentialsStore for NoopStore {
        fn get(&self, _s: &str) -> Result<SecretString> {
            Err(CredentialsError::UnknownService(_s.into()))
        }
        fn set(&self, _s: &str, _v: SecretString) -> Result<()> {
            Ok(())
        }
        fn delete(&self, _s: &str) -> Result<()> {
            Ok(())
        }
        fn list(&self) -> Result<Vec<String>> {
            Ok(vec![])
        }
        fn contains(&self, _s: &str) -> Result<bool> {
            Ok(false)
        }
    }
}
