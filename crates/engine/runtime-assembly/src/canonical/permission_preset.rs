//! Session-level `permission_preset` enforcement as a governance hook.
//!
//! The canonical execution core stays capability-generic; it only consults the
//! injected [`GovernanceHook`]. This production hook layers the session's
//! durable `permission_preset` on top of the existing policy decision chain:
//!
//! - `read_only` refuses write/execute tool calls with a human-readable denial;
//! - `standard` delegates to the inner policy (dangerous tools still require
//!   approval);
//! - `full` allows what the inner policy would only allow after approval, while
//!   keeping the runtime's normal audit/trace logging for the dispatch.
//!
//! The preset is read from the same [`SessionStore`] the runtime uses, so it
//! only affects the session's *subsequent* turns and never rewrites an approval
//! that is already in flight.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict};
use apeireth_runtime::canonical::{PermissionPreset, SessionStore};
use async_trait::async_trait;
use sha2::{Digest, Sha256};

/// Classify write/execute capabilities in the production composition layer.
///
/// This is intentionally NOT in the execution core: the runtime must not know
/// which concrete capabilities exist. The classifier lives here, next to the
/// concrete tool modules it names.
///
/// M2: the list is the `read_only` preset's enforcement surface — anything
/// missing from it becomes a silent gap ("只读" 会话仍可执行该能力). It has to
/// cover every egress, credential, and deletion exit, not just the obvious
/// mutators:
/// - 执行面: `tool.shell` / `tool.process` / `tool.supervisor` /
///   `tool.std_sub_supervisor` / `tool.mcp*` (动态能力);
/// - 受控写: `tool.filesystem` (workspace 边界内的读写, 仍是写);
/// - 出口: `tool.fetch` / `tool.repo` (remote 读发布) —— 只读会话不应外联;
/// - 凭据: `credential.read` / `secret.read` / `env.read` (环境变量含 secret);
/// - 删除: `fs.delete`;
/// - 发布/发送: `repo.publish` / `http.send`;
/// - canonical provider 语义能力 id (guard semantics.rs 同一套命名)。
pub fn is_write_or_execute_capability(capability: &CapabilityId) -> bool {
    const WRITE_OR_EXECUTE: &[&str] = &[
        // 执行面 (原名单)
        "tool.shell",
        "tool.process",
        "tool.supervisor",
        "tool.std_sub_supervisor",
        // 受控写
        "tool.filesystem",
        "fs.write",
        // 出口 / 远端副作用
        "tool.fetch",
        "tool.repo",
        "http.get",
        "http.send",
        "repo.publish",
        // 凭据与环境秘密
        "credential.read",
        "secret.read",
        "env.read",
        // 删除
        "fs.delete",
    ];
    let id = capability.as_str();
    WRITE_OR_EXECUTE.contains(&id) || id.starts_with("tool.mcp")
}

/// Governance hook that applies a session's [`PermissionPreset`] to capability
/// dispatch, delegating everything else to the wrapped inner hook.
pub struct PermissionPresetGovernanceHook {
    inner: Arc<dyn GovernanceHook>,
    sessions: Arc<dyn SessionStore>,
    /// In-process approval memory, keyed by `(session, capability, args_hash)`.
    ///
    /// L 组 (permission_preset.rs:81-86): the old key was `(session, capability)`
    /// — approving one `tool.shell` call made every later `tool.shell` call in
    /// that session skip its approval, including `rm -rf ...`. The key now binds
    /// the *arguments*, so only the exact operation a human approved is
    /// remembered. Deliberately not persisted: a restart clears it, so approval
    /// memory never outlives the process.
    approval_memory: RwLock<HashSet<(String, String, String)>>,
    /// The `args_hash` of the most recent `(session, capability)` pair that was
    /// escalated to RequireApproval and is still awaiting a human decision.
    ///
    /// The runtime's `approval_resolved(session, capability)` callback carries
    /// no arguments (trait signature), so the hook records here which exact
    /// operation is waiting, and promotes *that* one on resolution — not every
    /// pending call of the same capability in the same round.
    awaiting_approval: RwLock<HashMap<(String, String), String>>,
}

impl PermissionPresetGovernanceHook {
    /// Wrap `inner`, reading session settings from `sessions`.
    pub fn new(inner: Arc<dyn GovernanceHook>, sessions: Arc<dyn SessionStore>) -> Self {
        Self {
            inner,
            sessions,
            approval_memory: RwLock::new(HashSet::new()),
            awaiting_approval: RwLock::new(HashMap::new()),
        }
    }

    fn remembered(
        &self,
        session: &SessionId,
        capability: &CapabilityId,
        args_hash: &str,
    ) -> bool {
        // poison 容错 (L 组): 审批记忆是一次 telemetry 级状态, 不是安全边界
        // 本身; 持锁线程 panic 后取回数据继续, 不让级联 panic 打死进程。
        read_lock_or_recover(&self.approval_memory)
            .contains(&(
                session.to_string(),
                capability.as_str().to_string(),
                args_hash.to_string(),
            ))
    }
}

/// poison 容错读锁 (L 组): 数据未损坏, 不应把一次 panic 放大为全线崩溃。
fn read_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// poison 容错写锁 (L 组): 同上。
fn write_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 审批记忆键中的参数指纹: 规范化 JSON (对象键排序) 后取 SHA-256 前 16 hex。
///
/// 规范化保证"语义相等、键顺序不同"的参数得到同一指纹; 取摘要而非存明文,
/// 因此记忆里不会留下命令文本或可能出现在参数里的秘密值。
fn arguments_fingerprint(arguments: &serde_json::Value) -> String {
    fn canonical(value: &serde_json::Value, out: &mut String) {
        match value {
            serde_json::Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                out.push('{');
                for (index, key) in keys.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&serde_json::to_string(key.as_str()).unwrap_or_default());
                    out.push(':');
                    canonical(&map[*key], out);
                }
                out.push('}');
            }
            serde_json::Value::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    canonical(item, out);
                }
                out.push(']');
            }
            other => out.push_str(&serde_json::to_string(other).unwrap_or_default()),
        }
    }
    let mut canonical_text = String::new();
    canonical(arguments, &mut canonical_text);
    let digest = Sha256::digest(canonical_text.as_bytes());
    format!("{digest:x}")[..16].to_string()
}

#[async_trait]
impl GovernanceHook for PermissionPresetGovernanceHook {
    fn name(&self) -> &str {
        "permission_preset"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        self.evaluate_verbose(request).await.decision
    }

    fn approval_resolved(&self, session: &SessionId, capability: &CapabilityId) {
        // 只提升这一对 (session, capability) 最近一次真正等待审批的那个参数
        // 指纹: trait 回调不带参数, 这里不能把同能力同轮的其它挂起调用一并
        // 免审。
        let args_hash = write_lock_or_recover(&self.awaiting_approval)
            .remove(&(session.to_string(), capability.as_str().to_string()));
        if let Some(args_hash) = args_hash {
            write_lock_or_recover(&self.approval_memory).insert((
                session.to_string(),
                capability.as_str().to_string(),
                args_hash,
            ));
        }
    }

    /// Preserve the identity of the deciding hook: delegated decisions keep the
    /// inner hook's attribution (e.g. `permission_governance`), while preset
    /// denials and `full`-mode approval rewrites are attributed to this hook.
    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let Action::CapabilityDispatch {
            capability, arguments, ..
        } = &request.action
        else {
            return self.inner.evaluate_verbose(request).await;
        };

        let loaded = match self.sessions.load(&request.session).await {
            Ok(loaded) => loaded,
            // Fail closed: refusing to read settings must never widen access.
            Err(error) => {
                return GovernanceVerdict::new(
                    self.name(),
                    Decision::deny(format!(
                        "无法读取会话权限预设，已拒绝执行 (fail closed): {error}"
                    )),
                );
            }
        };

        // A session that does not exist yet has no preset; it will be created
        // with the default (`standard`, `approval_remember = false`) when the
        // turn starts.
        let (preset, approval_remember) = loaded
            .as_ref()
            .map(|session| {
                (
                    session.settings.permission_preset,
                    session.settings.approval_remember,
                )
            })
            .unwrap_or_default();

        match preset {
            PermissionPreset::ReadOnly if is_write_or_execute_capability(capability) => {
                GovernanceVerdict::new(
                    self.name(),
                    Decision::deny(format!(
                        "当前会话为只读权限预设 (read_only)，已拒绝写/执行类工具 {}。如需执行，请将会话权限预设调整为 standard 或 full。",
                        capability
                    )),
                )
            }
            PermissionPreset::Full => {
                let verdict = self.inner.evaluate_verbose(request).await;
                match verdict.decision {
                    Decision::RequireApproval { .. } => {
                        GovernanceVerdict::new(self.name(), Decision::Allow)
                    }
                    _ => verdict,
                }
            }
            PermissionPreset::Standard => {
                // `approval_remember` only applies to the standard preset: it
                // skips a *previously approved* approval prompt, never a denial.
                // `read_only` stays refuse-closed and `full` already allows.
                // L 组: 记忆键是 (session, capability, args_hash) —— 只放行
                // 人类真正批准过的那一次操作, 同会话换个参数 (rm -rf) 仍要审批。
                let args_hash = arguments_fingerprint(arguments);
                if approval_remember
                    && self.remembered(&request.session, capability, &args_hash)
                {
                    let verdict = self.inner.evaluate_verbose(request).await;
                    match verdict.decision {
                        Decision::RequireApproval { .. } => {
                            GovernanceVerdict::new(self.name(), Decision::Allow)
                        }
                        // Fail closed: memory skips approval only; it must not
                        // widen a real denial into an allow.
                        _ => verdict,
                    }
                } else {
                    let verdict = self.inner.evaluate_verbose(request).await;
                    if approval_remember
                        && matches!(verdict.decision, Decision::RequireApproval { .. })
                    {
                        write_lock_or_recover(&self.awaiting_approval).insert(
                            (
                                request.session.to_string(),
                                capability.as_str().to_string(),
                            ),
                            args_hash,
                        );
                    }
                    verdict
                }
            }
            // read_only-but-read-tool keeps existing policy.
            PermissionPreset::ReadOnly => self.inner.evaluate_verbose(request).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{system_clock, TraceId};
    use apeireth_governance::{Permission, PermissionGovernanceHook, PermissionPolicy};
    use apeireth_runtime::canonical::{InMemorySessionStore, Session};

    async fn build_hook() -> (Arc<PermissionPresetGovernanceHook>, Arc<dyn SessionStore>) {
        let mut policy = PermissionPolicy::new();
        policy.grant(Permission::ExecuteTool("tool.shell".into()));
        policy.require_approval_for("tool.shell");
        policy.grant(Permission::ExecuteTool("tool.calculator".into()));
        policy.require_approval_for("tool.calculator");

        let inner = Arc::new(PermissionGovernanceHook::new(policy));
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let hook = Arc::new(PermissionPresetGovernanceHook::new(inner, store.clone()));
        (hook, store)
    }

    async fn save_session(
        store: &Arc<dyn SessionStore>,
        session: SessionId,
        preset: PermissionPreset,
        approval_remember: bool,
    ) {
        let clock = system_clock();
        let mut stored = Session::new(session, clock.as_ref());
        stored.settings.permission_preset = preset;
        stored.settings.approval_remember = approval_remember;
        store.save(&stored).await.unwrap();
    }

    fn dispatch_request<'a>(
        session: SessionId,
        capability: &'a CapabilityId,
        arguments: &'a serde_json::Value,
    ) -> GovernanceRequest<'a> {
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            session,
            TraceId::new(),
            1,
        )
    }

    #[tokio::test]
    async fn approved_pair_skips_approval_in_standard_with_remember() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;

        let args = serde_json::Value::Null;
        let before = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &args))
            .await;
        assert!(
            matches!(before.decision, Decision::RequireApproval { .. }),
            "before approval the tool must still require approval"
        );

        approve_once(&hook, session, &capability, &args).await;

        let after = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &args))
            .await;
        assert!(
            after.decision.is_allowed(),
            "remembered approval must allow"
        );
        assert_eq!(after.hook, "permission_preset");
    }

    /// 走一遍真实流程: 评估登记 awaiting → 人类批准, 使审批记忆里留下
    /// `(session, capability, args_hash)` 三元组 (旧实现只需一次回调)。
    async fn approve_once(
        hook: &PermissionPresetGovernanceHook,
        session: SessionId,
        capability: &CapabilityId,
        arguments: &serde_json::Value,
    ) {
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, capability, arguments))
            .await;
        assert!(
            matches!(verdict.decision, Decision::RequireApproval { .. }),
            "the first evaluation must require approval before it can be approved"
        );
        hook.approval_resolved(&session, capability);
    }

    #[tokio::test]
    async fn approval_memory_is_scoped_to_session_capability_and_arguments() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let other_session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        let other_capability = CapabilityId::new("tool.calculator").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;
        save_session(&store, other_session, PermissionPreset::Standard, true).await;

        let approved_args = serde_json::json!({"command": "echo approved-only"});
        approve_once(&hook, session, &capability, &approved_args).await;

        // 同一个 (session, capability) + 同一个参数指纹 → 免审。
        let remembered = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &approved_args))
            .await;
        assert!(
            remembered.decision.is_allowed(),
            "the approved operation must be remembered"
        );

        // L 组回归 (permission_preset.rs:81-86): 同会话同能力但**不同参数**
        // 不得继承免审 —— 旧键 (session, capability) 下"批准一次 echo 之后,
        // 同会话的 rm -rf 也跳过审批"。
        let dangerous_args = serde_json::json!({"command": "rm -rf /tmp/data"});
        let different_args = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &dangerous_args))
            .await;
        assert!(
            matches!(
                different_args.decision,
                Decision::RequireApproval { .. }
            ),
            "a different operation of the same capability must ask again, got {:?}",
            different_args.decision
        );

        // 其它会话 / 其它能力同样不继承。
        let other_session_verdict = hook
            .evaluate_verbose(&dispatch_request(other_session, &capability, &approved_args))
            .await;
        assert!(
            matches!(
                other_session_verdict.decision,
                Decision::RequireApproval { .. }
            ),
            "a different session must not inherit the memory"
        );

        let other_capability_verdict = hook
            .evaluate_verbose(&dispatch_request(session, &other_capability, &approved_args))
            .await;
        assert!(
            matches!(
                other_capability_verdict.decision,
                Decision::RequireApproval { .. }
            ),
            "a different capability must not inherit the memory"
        );
    }

    #[tokio::test]
    async fn approval_memory_never_bypasses_a_denial() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.unpermitted").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;

        // Seed memory directly to prove the fail-closed path is independent of
        // how the entry got there.
        let args = serde_json::Value::Null;
        write_lock_or_recover(&hook.approval_memory).insert((
            session.to_string(),
            capability.as_str().to_string(),
            arguments_fingerprint(&args),
        ));

        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &args))
            .await;
        assert!(
            matches!(verdict.decision, Decision::Deny { .. }),
            "memory must skip approval only, never a denial"
        );
    }

    #[tokio::test]
    async fn approval_memory_is_disabled_when_remember_is_false() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        save_session(&store, session, PermissionPreset::Standard, false).await;

        // 直接把记忆种满, 再证明开关关着时它不被使用。
        let args = serde_json::Value::Null;
        write_lock_or_recover(&hook.approval_memory).insert((
            session.to_string(),
            capability.as_str().to_string(),
            arguments_fingerprint(&args),
        ));

        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &args))
            .await;
        assert!(
            matches!(verdict.decision, Decision::RequireApproval { .. }),
            "approval_remember = false must keep prompting every time"
        );
    }

    /// 审批记忆键的参数指纹: 键顺序无关、按内容稳定, 且只存摘要不存明文。
    #[test]
    fn arguments_fingerprint_is_key_order_independent_and_never_plaintext() {
        let a = serde_json::json!({"command": "echo hi", "cwd": "/tmp"});
        let b = serde_json::json!({"cwd": "/tmp", "command": "echo hi"});
        assert_eq!(arguments_fingerprint(&a), arguments_fingerprint(&b));
        assert_ne!(
            arguments_fingerprint(&a),
            arguments_fingerprint(&serde_json::json!({"command": "echo bye", "cwd": "/tmp"}))
        );
        let fingerprint =
            arguments_fingerprint(&serde_json::json!({"token": "sk-top-secret-value"}));
        assert!(
            !fingerprint.contains("sk-top-secret-value"),
            "the memory key must be a digest, not plaintext: {fingerprint}"
        );
    }

    /// M2 回归: `read_only` 预设的执法面必须覆盖出口/凭据/删除类能力 —— 名单
    /// 缺一项, "只读" 会话就能静默执行该能力。    #[test]
    fn read_only_preset_covers_egress_credential_and_delete_exits() {
        for id in [
            // 执行面 (原名单)
            "tool.shell",
            "tool.process",
            "tool.supervisor",
            "tool.std_sub_supervisor",
            "tool.mcp.dynamic",
            // 受控写
            "tool.filesystem",
            "fs.write",
            // 出口 / 远端副作用
            "tool.fetch",
            "tool.repo",
            "http.get",
            "http.send",
            "repo.publish",
            // 凭据与环境秘密
            "credential.read",
            "secret.read",
            "env.read",
            // 删除
            "fs.delete",
        ] {
            assert!(
                is_write_or_execute_capability(&CapabilityId::new(id).unwrap()),
                "{id} must be refused under the read_only preset"
            );
        }
        // 纯读能力不受影响。
        for id in ["tool.search", "tool.education", "fs.read", "unknown.reader"] {
            assert!(
                !is_write_or_execute_capability(&CapabilityId::new(id).unwrap()),
                "{id} is a read capability and must stay allowed"
            );
        }
    }

    /// M2 (行为面): read_only 会话下, 新纳入名单的能力必须被本 hook 显式
    /// 拒绝 (而不是落到 inner policy 的可能放行)。
    #[tokio::test]
    async fn read_only_preset_refuses_newly_covered_capabilities() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        save_session(&store, session, PermissionPreset::ReadOnly, false).await;
        let args = serde_json::json!({});

        for id in ["tool.fetch", "http.send", "credential.read", "fs.delete"] {
            let capability = CapabilityId::new(id).unwrap();
            let verdict = hook
                .evaluate_verbose(&dispatch_request(session, &capability, &args))
                .await;
            assert!(
                matches!(verdict.decision, Decision::Deny { .. }),
                "{id} must be denied under read_only, got {:?}",
                verdict.decision
            );
            assert_eq!(verdict.hook, "permission_preset");
        }
    }
}
