//! **W4① 真审批链接线** — 高危凭据审批门接成真治理 hook (2026-10-10)。
//!
//! 原 [`crate::gate`] 只给 trait 口 + `DenyAllGate` fail-closed 默认, 真审批链
//! "留 companion 装配侧" (0 装)。本模块把它接成**真治理 hook** (IMPLEMENTED):
//! 高危凭据操作经 [`apeireth_governance::GovernanceHook`] 进入规范审批流 ——
//! 判定走 [`Decision::RequireApproval`] (挂起等人, 与 [`Decision::Deny`] 严格区分,
//! 不枪毙回合), 人类批准经 [`GovernanceHook::approval_resolved`] 回流为**会话内记忆**
//! (approval_remember 语义: 同会话同能力批准一次即放行; Deny 永不被记忆绕过)。
//!
//! **一份记忆, 两张脸**:
//! - [`CredentialApprovalHook`] — 治理侧 (runtime 审批流的真 hook);
//! - [`ApprovalMemoryGate`] — 存取侧 (喂 [`crate::GatedCredentialsStore`] 的真门)。
//! 两者共享 [`ApprovalMemory`], 人类批准一次, 两侧同步生效。
//!
//! **识别约定**: `Action::CapabilityDispatch` 的 `arguments` 携带
//! `credential_service` (字符串) 即视为凭据操作; 可选 `credential_op`
//! (`"read"|"write"|"delete"`, 缺省 `"read"`)。不携带该字段的动作一律放行
//! (本 hook 只管高危凭据, 不越权)。
//!
//! **0 假装边界**:
//! - 生产装配挂接 (`GovernancePipeline` 加本 hook / CLI bootstrap 换
//!   `GatedCredentialsStore<_, ApprovalMemoryGate>`) **未做** —— 装配侧在
//!   adapters/runtime-assembly (W2 批次并行施工区), 留显式后续;
//! - 记忆是**进程内** (重启即忘, 与 approval_remember 同口径); 跨进程持久批准
//!   属审批链本体, 不在本模块。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use async_trait::async_trait;
use serde_json::Value;

use crate::gate::{CredentialGate, CredentialOp, GateDecision, DEFAULT_HIGH_RISK_SERVICES};

/// 从能力参数里识别凭据服务名的字段 (识别约定, 见模块头)。
pub const CREDENTIAL_SERVICE_FIELD: &str = "credential_service";
/// 从能力参数里识别凭据操作的字段 (缺省 read)。
pub const CREDENTIAL_OP_FIELD: &str = "credential_op";

/// **批准记忆** (进程内, 会话内记住 — approval_remember 语义)。
///
/// 记两本账:
/// - `approved`: `(scope, service)` 已获人类批准 (scope = 会话键或 `"global"`);
/// - `intents`: `(session, capability) → service` 映射, 供
///   [`GovernanceHook::approval_resolved`] 的 `(session, capability)` 粒度回调
///   反查到 service 后入账。
///
/// **红线**: 只记 service 名与 ID, **绝不**记凭据明文。
#[derive(Debug, Default)]
pub struct ApprovalMemory {
    approved: Mutex<HashSet<(String, String)>>,
    intents: Mutex<HashMap<(String, String), String>>,
}

impl ApprovalMemory {
    /// 新建空记忆。
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 记入 `(scope, service)` 批准。
    pub fn remember(&self, scope: &str, service: &str) {
        self.approved
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert((scope.to_string(), service.to_string()));
    }

    /// 是否已批准。
    pub fn is_approved(&self, scope: &str, service: &str) -> bool {
        self.approved
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .contains(&(scope.to_string(), service.to_string()))
    }

    /// 记录一次待批意图 (evaluate 时挂账, 批准回调时反查)。
    pub fn note_intent(&self, session: &str, capability: &str, service: &str) {
        self.intents
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(
                (session.to_string(), capability.to_string()),
                service.to_string(),
            );
    }

    /// 审批通过回调: 反查意图并入账。未挂账的 `(session, capability)` 忽略 (不炸)。
    pub fn resolve(&self, session: &str, capability: &str) {
        let service = self
            .intents
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .remove(&(session.to_string(), capability.to_string()));
        if let Some(service) = service {
            self.remember(session, &service);
        }
    }

    /// 已批准条目数 (测试/诊断用)。
    pub fn approved_len(&self) -> usize {
        self.approved
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }
}

/// ID → 稳定键 (Debug 形态; 仅作内存键, 不落任何输出通道)。
fn id_key(id: &impl std::fmt::Debug) -> String {
    format!("{id:?}")
}

/// 参数里的凭据操作标签 (缺省 read; 仅用于人类可读的审批文案)。
fn op_label(arguments: &Value) -> &'static str {
    match arguments
        .get(CREDENTIAL_OP_FIELD)
        .and_then(|v| v.as_str())
        .unwrap_or("read")
    {
        "write" => "写入",
        "delete" => "删除",
        _ => "读取",
    }
}

/// **高危凭据治理 hook** (真 `GovernanceHook` 实现)。
///
/// 判定语义:
/// - 非凭据动作 → [`Decision::Allow`] (本 hook 不越权);
/// - 凭据动作但非高危服务 → [`Decision::Allow`] (普通凭据不阻塞);
/// - 高危服务 + 会话内已批准 (且开启会话记忆) → [`Decision::Allow`];
/// - 高危服务 + 未批准 → [`Decision::RequireApproval`] (**挂起**, 非 Deny)。
pub struct CredentialApprovalHook {
    memory: Arc<ApprovalMemory>,
    high_risk: Vec<String>,
    session_memory: bool,
}

impl CredentialApprovalHook {
    /// 新建 (默认高危名单 [`DEFAULT_HIGH_RISK_SERVICES`], 会话记忆开启)。
    pub fn new(memory: Arc<ApprovalMemory>) -> Self {
        Self {
            memory,
            high_risk: DEFAULT_HIGH_RISK_SERVICES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            session_memory: true,
        }
    }

    /// 关闭会话记忆 (严格模式: 每次高危操作都要重新批准)。
    #[must_use]
    pub fn without_session_memory(mut self) -> Self {
        self.session_memory = false;
        self
    }

    /// 覆盖高危服务名单。
    #[must_use]
    pub fn with_high_risk<I, S>(mut self, services: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.high_risk = services.into_iter().map(Into::into).collect();
        self
    }

    /// 共享的批准记忆 (供存取侧 [`ApprovalMemoryGate`] 复用)。
    pub fn memory(&self) -> &Arc<ApprovalMemory> {
        &self.memory
    }

    fn is_high_risk_service(&self, service: &str) -> bool {
        self.high_risk.iter().any(|s| s == service)
    }
}

#[async_trait]
impl GovernanceHook for CredentialApprovalHook {
    fn name(&self) -> &str {
        "credential_approval_gate"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            // Completion 与 (non_exhaustive) 未来动作类别都不属凭据操作。
            return Decision::Allow;
        };
        let Some(service) = arguments
            .get(CREDENTIAL_SERVICE_FIELD)
            .and_then(|v| v.as_str())
        else {
            return Decision::Allow;
        };
        if !self.is_high_risk_service(service) {
            return Decision::Allow;
        }

        let session_key = id_key(&request.session);
        if self.session_memory && self.memory.is_approved(&session_key, service) {
            return Decision::Allow;
        }

        self.memory
            .note_intent(&session_key, &id_key(capability), service);
        Decision::require_approval(format!(
            "高危凭据 `{service}` 的{}需主人批准 (capability {capability}, session {:?})。\
             AI 不接触明文; 未批准不放行。",
            op_label(arguments),
            request.session
        ))
    }

    fn approval_resolved(&self, session: &SessionId, capability: &CapabilityId) {
        if self.session_memory {
            self.memory.resolve(&id_key(session), &id_key(capability));
        }
    }
}

/// **存取侧批准门**: [`CredentialGate`] 的真实现 (共享 [`ApprovalMemory`])。
///
/// 绑定一个 `scope` (会话键或 `"global"`); [`crate::GatedCredentialsStore`] 只对
/// 高危服务问 [`CredentialGate::decide`] —— 记忆中有批准即放行, 否则拒 (fail-closed
/// 默认不变)。
pub struct ApprovalMemoryGate {
    memory: Arc<ApprovalMemory>,
    scope: String,
    high_risk: Vec<String>,
}

impl ApprovalMemoryGate {
    /// 绑定会话 (scope = 会话键, 与 [`CredentialApprovalHook`] 同源)。
    pub fn for_session(memory: Arc<ApprovalMemory>, session: &SessionId) -> Self {
        Self {
            memory,
            scope: id_key(session),
            high_risk: DEFAULT_HIGH_RISK_SERVICES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    /// 全局作用域 (无会话语境的装配点用; 批准入账 scope = `"global"`)。
    pub fn global(memory: Arc<ApprovalMemory>) -> Self {
        Self {
            memory,
            scope: "global".to_string(),
            high_risk: DEFAULT_HIGH_RISK_SERVICES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }

    /// 覆盖高危服务名单。
    #[must_use]
    pub fn with_high_risk<I, S>(mut self, services: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.high_risk = services.into_iter().map(Into::into).collect();
        self
    }
}

impl CredentialGate for ApprovalMemoryGate {
    fn decide(&self, service: &str, _op: CredentialOp) -> GateDecision {
        if self.memory.is_approved(&self.scope, service) {
            GateDecision::Allow
        } else {
            GateDecision::Deny
        }
    }

    fn is_high_risk(&self, service: &str) -> bool {
        self.high_risk.iter().any(|s| s == service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gate::GatedCredentialsStore;
    use crate::secret::SecretString;
    use crate::store::{CredentialsStore, FileCredentialsStore};
    use apeireth_core::kernel::{SessionId, TraceId};

    fn session() -> SessionId {
        SessionId::new()
    }

    fn cap(name: &str) -> CapabilityId {
        CapabilityId::new(name).expect("capability id")
    }

    fn dispatch<'a>(cap: &'a CapabilityId, args: &'a Value) -> Action<'a> {
        Action::CapabilityDispatch {
            capability: cap,
            arguments: args,
        }
    }

    async fn eval(
        hook: &CredentialApprovalHook,
        session: &SessionId,
        cap: &CapabilityId,
        args: &Value,
    ) -> Decision {
        let action = dispatch(cap, args);
        let req = GovernanceRequest::new(action, *session, TraceId::new(), 1);
        hook.evaluate(&req).await
    }

    #[tokio::test]
    async fn completion_and_non_credential_dispatch_pass() {
        let hook = CredentialApprovalHook::new(ApprovalMemory::new());
        let s = session();
        let c = cap("tool.shell");
        // 无 credential_service 字段 → 放行
        assert!(eval(&hook, &s, &c, &serde_json::json!({"command": "ls"}))
            .await
            .is_allowed());
    }

    #[tokio::test]
    async fn high_risk_credential_dispatch_requires_approval_then_remembers() {
        let hook = CredentialApprovalHook::new(ApprovalMemory::new());
        let s = session();
        let c = cap("credential.read");
        let args =
            serde_json::json!({CREDENTIAL_SERVICE_FIELD: "master", CREDENTIAL_OP_FIELD: "read"});

        // 未批准 → RequireApproval (不是 Deny: 挂起不枪毙)
        let d1 = eval(&hook, &s, &c, &args).await;
        assert!(matches!(d1, Decision::RequireApproval { .. }), "{d1:?}");
        assert!(d1.reason().unwrap().contains("master"));

        // 人类批准回流 → 同会话同能力放行
        hook.approval_resolved(&s, &c);
        assert!(eval(&hook, &s, &c, &args).await.is_allowed());

        // 换会话不继承 (会话内记住, 非全局)
        let s2 = session();
        assert!(matches!(
            eval(&hook, &s2, &c, &args).await,
            Decision::RequireApproval { .. }
        ));
    }

    #[tokio::test]
    async fn strict_mode_never_remembers() {
        let hook = CredentialApprovalHook::new(ApprovalMemory::new()).without_session_memory();
        let s = session();
        let c = cap("credential.write");
        let args = serde_json::json!({CREDENTIAL_SERVICE_FIELD: "master_token"});
        hook.approval_resolved(&s, &c);
        assert!(matches!(
            eval(&hook, &s, &c, &args).await,
            Decision::RequireApproval { .. }
        ));
    }

    #[tokio::test]
    async fn normal_credentials_pass_high_risk_names_gate() {
        let hook = CredentialApprovalHook::new(ApprovalMemory::new());
        let s = session();
        let c = cap("credential.read");
        // 普通服务 → 放行
        assert!(eval(
            &hook,
            &s,
            &c,
            &serde_json::json!({CREDENTIAL_SERVICE_FIELD: "openai"})
        )
        .await
        .is_allowed());
        // 三个默认高危名都拦
        for svc in ["master", "master_token", "master-token"] {
            assert!(matches!(
                eval(
                    &hook,
                    &s,
                    &c,
                    &serde_json::json!({CREDENTIAL_SERVICE_FIELD: svc})
                )
                .await,
                Decision::RequireApproval { .. }
            ));
        }
    }

    #[tokio::test]
    async fn hook_and_store_gate_share_one_memory() {
        // 一份记忆两张脸: hook 批准入账后, 存取侧 GatedCredentialsStore 同步放行。
        let mem = ApprovalMemory::new();
        let hook = CredentialApprovalHook::new(mem.clone());
        let s = session();
        let c = cap("credential.read");
        let args = serde_json::json!({CREDENTIAL_SERVICE_FIELD: "master"});
        assert!(matches!(
            eval(&hook, &s, &c, &args).await,
            Decision::RequireApproval { .. }
        ));
        hook.approval_resolved(&s, &c);

        let dir = std::env::temp_dir().join(format!(
            "apeireth-credentials-hook-share-{}",
            std::process::id()
        ));
        let store = FileCredentialsStore::new(dir.join("creds.json")).expect("store");
        store
            .set("master", SecretString::new("mt-shh"))
            .expect("set via inner");
        let gated =
            GatedCredentialsStore::new(store, ApprovalMemoryGate::for_session(mem.clone(), &s));
        // 已批准 → 读得到 (且明文不入 Debug)
        let v = gated.get("master").expect("approved read");
        assert_eq!(v.expose(), "mt-shh");
        assert!(!format!("{v:?}").contains("mt-shh"));

        // 未批准会话 → fail-closed 仍然拒 (同一后端文件的新 store 实例)
        let s2 = session();
        let store2 = FileCredentialsStore::new(dir.join("creds.json")).expect("store2");
        let gated2 = GatedCredentialsStore::new(store2, ApprovalMemoryGate::for_session(mem, &s2));
        let e = gated2.get("master").unwrap_err();
        assert!(matches!(
            e,
            crate::error::CredentialsError::ApprovalRequired { .. }
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn approval_without_intent_is_ignored_not_fatal() {
        // 元层原则: 审批机制只降级不枪毙 — 未挂账的 resolve 不 panic、不误放行。
        let mem = ApprovalMemory::new();
        mem.resolve("unknown-session", "unknown-cap");
        assert_eq!(mem.approved_len(), 0);
    }
}
