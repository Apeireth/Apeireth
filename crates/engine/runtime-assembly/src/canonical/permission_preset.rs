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
use apeireth_governance::approval_family::family_ids_for_arguments;
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
/// - 受控写: `tool.filesystem` (workspace 边界内的读写, 仍是写) /
///   `tool.apply_patch` (补丁式受控写文件, 创建/修改/删除都算写) /
///   `tool.worktree_apply` (四拍受控工作树写, 改单落盘仍是写;
///   `tool.worktree_plan` 是只读改单面, 不入本名单) /
///   `tool.skill_distill` (技能库沉淀写, 同受 file_write 写总闸;
///   `tool.skill_search` 是只读检索面, 不入本名单) /
///   `tool.backup_export` (备份导出写文件面: 全量打包落盘 bundle, 虽读记忆
///   但产出是持久写, 只读会话不应打包外带);
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
        "tool.apply_patch",
        "tool.git_write",
        "tool.worktree_apply",
        "tool.skill_distill",
        "tool.backup_export",
        // V3-b 通用嗓发声 (`tool.voice_speak`): 发声面执行档 —— 合成即播是
        // 对外发声的执行面 (语句账本落盘同为副作用), 只读会话不应开口;
        // 「读」在 `tool.self_status` (语音节自述), 不在本名单。
        "tool.voice_speak",
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

/// 「L2 永不因预设放行」地板: 命中即该调用**不可**被预设改写放行。
///
/// git 写分级 (D-P1) 的 L2 出网写 (push/fetch/remote) 永远停人工审批 ——
/// `full` 预设的 RequireApproval→Allow 改写与 `standard` 的 `approval_remember`
/// 免审记忆都不得越过它 (手机上批 push, 但每次都得人批)。L1 可逆本地写不受
/// 地板约束 (照常走预设语义); L3/词表外本来就拒绝即帧, 无需地板。
pub fn human_approval_floor_applies(
    capability: &CapabilityId,
    arguments: &serde_json::Value,
) -> bool {
    capability.as_str() == apeireth_tools_canonical::GIT_WRITE_CAPABILITY_ID
        && apeireth_tools_canonical::git_write_human_approval_floor(arguments)
}

/// Governance hook that applies a session's [`PermissionPreset`] to capability
/// dispatch, delegating everything else to the wrapped inner hook.
pub struct PermissionPresetGovernanceHook {
    inner: Arc<dyn GovernanceHook>,
    sessions: Arc<dyn SessionStore>,
    /// In-process approval memory, keyed by `(session, capability, authorization)`.
    ///
    /// L 组 (permission_preset.rs:81-86): the old key was `(session, capability)`
    /// — approving one `tool.shell` call made every later `tool.shell` call in
    /// that session skip its approval, including `rm -rf ...`.
    ///
    /// E-P2 审批族归一: 第三键是**授权对象** ([`authorization_key`]) ——
    /// 命令面记录的授权对象 = **族集** (批"删除族" = 该族所有形态生效:
    /// 同族换形重放被同一授权覆盖、不重批; 跨族仍逐族审批, 批删除≠批网络;
    /// 审批卡族标注随卡明示授权族, 人见族而批)。无族命令退回**精确操作
    /// 指纹** (逐字语义, L 组口径对无族命令原样保留)。Deny 永不被记忆绕过。
    /// Deliberately not persisted: a restart clears it, so approval memory
    /// never outlives the process.
    approval_memory: RwLock<HashSet<(String, String, String)>>,
    /// The authorization key of the most recent `(session, capability)` pair
    /// that was escalated to RequireApproval and is still awaiting a human
    /// decision.
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
        authorization: &str,
    ) -> bool {
        // poison 容错 (L 组): 审批记忆是一次 telemetry 级状态, 不是安全边界
        // 本身; 持锁线程 panic 后取回数据继续, 不让级联 panic 打死进程。
        read_lock_or_recover(&self.approval_memory).contains(&(
            session.to_string(),
            capability.as_str().to_string(),
            authorization.to_string(),
        ))
    }
}

/// poison 容错读锁 (L 组): 数据未损坏, 不应把一次 panic 放大为全线崩溃。
fn read_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// poison 容错写锁 (L 组): 同上。
fn write_lock_or_recover<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

/// 审批记忆的**授权对象键** (E-P2 族归一):
///
/// * 命令面 (族可归): `family:<族id+…>` —— 授权对象 = 族集。批"删除族" =
///   该族所有形态生效 (同族换形重放不重批), 跨族不串 (批删除≠批网络);
///   多族命令以族组合为对象 (批"删除族+网络族" ≠ 单批删除族)。
/// * 无族命令: `exact:<参数指纹>` —— 授权对象 = 该次操作本身 (逐字语义,
///   L 组口径原样保留: 同能力不同操作不继承免审)。
///
/// 族集判定走治理层单一来源 (`family_ids_for_arguments`, 段首动词 + 成员
/// 调用归属面): 提到族词 (`echo del is a word`) 不构成授权归属, 不白拿授权。
fn authorization_key(arguments: &serde_json::Value) -> String {
    let families = family_ids_for_arguments(arguments);
    if families.is_empty() {
        format!("exact:{}", arguments_fingerprint(arguments))
    } else {
        format!("family:{}", families.join("+"))
    }
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
        // 只提升这一对 (session, capability) 最近一次真正等待审批的那个授权
        // 对象: trait 回调不带参数, 这里不能把同能力同轮的其它挂起调用一并
        // 免审。
        let authorization = write_lock_or_recover(&self.awaiting_approval)
            .remove(&(session.to_string(), capability.as_str().to_string()));
        if let Some(authorization) = authorization {
            write_lock_or_recover(&self.approval_memory).insert((
                session.to_string(),
                capability.as_str().to_string(),
                authorization,
            ));
        }
    }

    /// Preserve the identity of the deciding hook: delegated decisions keep the
    /// inner hook's attribution (e.g. `permission_governance`), while preset
    /// denials and `full`-mode approval rewrites are attributed to this hook.
    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let Action::CapabilityDispatch {
            capability,
            arguments,
            ..
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
                    // L2 永不因预设放行: 命中人工审批地板的调用, `full` 的
                    // RequireApproval→Allow 改写也不生效 (D-P1 git 写分级)。
                    Decision::RequireApproval { .. }
                        if !human_approval_floor_applies(capability, arguments) =>
                    {
                        GovernanceVerdict::new(self.name(), Decision::Allow)
                    }
                    _ => verdict,
                }
            }
            PermissionPreset::Standard => {
                // `approval_remember` only applies to the standard preset: it
                // skips a *previously approved* approval prompt, never a denial.
                // `read_only` stays refuse-closed and `full` already allows.
                // L 组 + E-P2: 记忆键是 (session, capability, 授权对象) ——
                // 命令面授权对象 = 族集 (批"删除族" = 该族所有形态生效,
                // 同族换形重放不重批; 跨族仍逐族审批), 无族命令按精确操作
                // 指纹逐字记住 (L 组: 同会话换个操作仍要审批)。
                // L2 永不因预设放行: 免审记忆也不越过人工审批地板 (D-P1)。
                let authorization = authorization_key(arguments);
                if approval_remember
                    && !human_approval_floor_applies(capability, arguments)
                    && self.remembered(&request.session, capability, &authorization)
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
                            authorization,
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
    /// `(session, capability, 授权对象)` 三元组 (E-P2 起授权对象 = 族集 /
    /// 精确操作指纹, 见 [`authorization_key`])。
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

    /// 记忆作用域 (L 组回归 + E-P2 授权对象语义): 同 (会话, 能力) 只对
    /// **同一授权对象**免审 —— 无族命令按精确操作指纹逐字记住 (批准一次
    /// `echo` 不免审 `rm -rf`, L 组口径原样); 命令面按族集记住 (见下方
    /// E-P2 三测)。其它会话 / 其它能力一律不继承。
    #[tokio::test]
    async fn approval_memory_is_scoped_to_session_capability_and_authorization_object() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let other_session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        let other_capability = CapabilityId::new("tool.calculator").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;
        save_session(&store, other_session, PermissionPreset::Standard, true).await;

        let approved_args = serde_json::json!({"command": "echo approved-only"});
        approve_once(&hook, session, &capability, &approved_args).await;

        // 同一个 (session, capability) + 同一个授权对象 → 免审。
        let remembered = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &approved_args))
            .await;
        assert!(
            remembered.decision.is_allowed(),
            "the approved operation must be remembered"
        );

        // L 组回归 (permission_preset.rs:81-86): 无族命令逐字授权 —— 批准
        // 一次 echo 之后, 同会话的 rm -rf (另一授权对象) 仍要审批。
        let dangerous_args = serde_json::json!({"command": "rm -rf /tmp/data"});
        let different_args = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &dangerous_args))
            .await;
        assert!(
            matches!(different_args.decision, Decision::RequireApproval { .. }),
            "a different operation of the same capability must ask again, got {:?}",
            different_args.decision
        );

        // 其它会话 / 其它能力同样不继承。
        let other_session_verdict = hook
            .evaluate_verbose(&dispatch_request(
                other_session,
                &capability,
                &approved_args,
            ))
            .await;
        assert!(
            matches!(
                other_session_verdict.decision,
                Decision::RequireApproval { .. }
            ),
            "a different session must not inherit the memory"
        );

        let other_capability_verdict = hook
            .evaluate_verbose(&dispatch_request(
                session,
                &other_capability,
                &approved_args,
            ))
            .await;
        assert!(
            matches!(
                other_capability_verdict.decision,
                Decision::RequireApproval { .. }
            ),
            "a different capability must not inherit the memory"
        );
    }

    /// E-P2 重放对抗①（同族换形不重批）：同族孪生 API 形态换形重放 =
    /// 同族授权覆盖 —— 批一次 `Remove-Item x.txt`（删除族）后，`rm` / 静态
    /// 调用形态重放不再重批（G2 的"重放白拿授权"改写为"重放同族授权覆盖"，
    /// 不再逃出授权面）。
    #[tokio::test]
    async fn same_family_form_change_replay_is_covered_without_reapproval() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;

        let approved = serde_json::json!({"command": "Remove-Item x.txt"});
        approve_once(&hook, session, &capability, &approved).await;

        for replay in [
            "[System.IO.File]::Delete('x.txt')",
            "rm x.txt",
            "del x.txt",
            "([System.IO.FileInfo]'x.txt').Delete()",
        ] {
            let args = serde_json::json!({ "command": replay });
            let verdict = hook
                .evaluate_verbose(&dispatch_request(session, &capability, &args))
                .await;
            assert!(
                verdict.decision.is_allowed(),
                "{replay} 同族换形重放必须被同族授权覆盖（不重批）: {:?}",
                verdict.decision
            );
        }
    }

    /// E-P2 重放对抗②（跨族必批）：批删除族 ≠ 批网络族 —— 同会话换族重放
    /// 仍要人批；跨族复合命令（删除族+网络族）另立授权对象，不吞并单族授权。
    #[tokio::test]
    async fn cross_family_replay_still_requires_separate_approval() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;

        let approved = serde_json::json!({"command": "Remove-Item x.txt"});
        approve_once(&hook, session, &capability, &approved).await;

        let network = serde_json::json!({"command": "Invoke-WebRequest http://example.invalid"});
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &network))
            .await;
        assert!(
            matches!(verdict.decision, Decision::RequireApproval { .. }),
            "批删除族不得顺带免审网络族（批删除≠批网络）: {:?}",
            verdict.decision
        );

        // 复合命令（两族）是另一授权对象: 不被单族授权吞并, 也不吞并单族。
        let compound = serde_json::json!({"command": "del a.txt && curl http://example.invalid"});
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &compound))
            .await;
        assert!(
            matches!(verdict.decision, Decision::RequireApproval { .. }),
            "跨族复合命令必须另批: {:?}",
            verdict.decision
        );
    }

    /// E-P2 授权对象语义钉死（批"删除族" = 该族所有形态生效）：族集授权
    /// 覆盖同族形态与同族目标；审批卡族标注（`authorization_families`）随卡
    /// 明示授权族，人见族而批。族内目标粒度不在本层（要目标级逐批走
    /// AllowedOnce 一次性审批面 / 关闭 approval_remember），如实入档契约。
    #[tokio::test]
    async fn family_authorization_covers_every_form_of_the_family() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        let capability = CapabilityId::new("tool.shell").unwrap();
        save_session(&store, session, PermissionPreset::Standard, true).await;

        let approved = serde_json::json!({"command": "Remove-Item x.txt"});
        approve_once(&hook, session, &capability, &approved).await;

        let other_target = serde_json::json!({"command": "rm -rf /tmp/other-target"});
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &other_target))
            .await;
        assert!(
            verdict.decision.is_allowed(),
            "批\"删除族\" = 该族所有形态生效（族集授权对象语义）: {:?}",
            verdict.decision
        );

        // 无族命令仍逐字授权（L 组口径不因族归一放宽）。
        let familyless = serde_json::json!({"command": "echo hi"});
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &capability, &familyless))
            .await;
        assert!(
            matches!(verdict.decision, Decision::RequireApproval { .. }),
            "无族命令必须逐字逐批: {:?}",
            verdict.decision
        );
    }

    /// G2.5 与族集授权的关系（不推翻 G2）: 动作绑定摘要绑"这一次的内容"
    /// （同族不同目标 = 不同动作内容, 单次授权面按摘要逐次钉死）, 族记忆的
    /// 授权对象仍是**族集**（同族不绑目标, 换形/换目标不重批）——
    /// 绑定只护单次授权, 族级换形重放语义零回归（G2 零回归）。
    #[test]
    fn action_binding_binds_one_shot_content_while_family_keys_stay_target_unbound() {
        let x = serde_json::json!({ "command": "Remove-Item x.txt" });
        let y = serde_json::json!({ "command": "rm -rf /tmp/other-target" });

        // 单次授权面: 同族不同目标是不同的动作内容 (摘要不同)。
        let x_binding = apeireth_governance::ActionBinding::with_derived_target(
            "tool.shell",
            "sess-1",
            "tool-schema-v1",
            x.clone(),
        );
        let y_binding = apeireth_governance::ActionBinding::with_derived_target(
            "tool.shell",
            "sess-1",
            "tool-schema-v1",
            y.clone(),
        );
        assert_ne!(
            x_binding.digest(),
            y_binding.digest(),
            "一次性授权绑定动作内容: 目标变 = 摘要变"
        );

        // 族记忆面: 授权对象键仍是族集, 与目标无关 (G2 语义不变)。
        assert_eq!(authorization_key(&x), "family:删除族");
        assert_eq!(
            authorization_key(&x),
            authorization_key(&y),
            "族集授权键与目标无关: 同族换形/换目标不重批 (G2 零回归)"
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
            authorization_key(&args),
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
            authorization_key(&args),
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
    /// 缺一项, "只读" 会话就能静默执行该能力。
    ///
    /// 债清 (D-P2 进场发现): 本测试的 `#[test]` 属性曾被并入上一行行尾注释而
    /// 静默失效 (陈旧断言 —— 挂名不跑), 现恢复挂测并钉 D-P2 新面。
    #[test]
    fn read_only_preset_covers_egress_credential_and_delete_exits() {
        for id in [
            // 执行面 (原名单)
            "tool.shell",
            "tool.process",
            "tool.supervisor",
            "tool.std_sub_supervisor",
            "tool.mcp.dynamic",
            // 发声执行面 (V3-b): 合成即播 = 对外发声副作用, 只读会话不开口
            "tool.voice_speak",
            // 受控写
            "tool.filesystem",
            "tool.apply_patch",
            "tool.git_write",
            "tool.worktree_apply",
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
        // 纯读能力不受影响 (worktree_plan 是只读改单面, 计划不落盘)。
        for id in [
            "tool.search",
            "tool.education",
            "tool.worktree_plan",
            "fs.read",
            "unknown.reader",
        ] {
            assert!(
                !is_write_or_execute_capability(&CapabilityId::new(id).unwrap()),
                "{id} is a read capability and must stay allowed"
            );
        }
    }

    /// 「L2 永不因预设放行」地板判定 (D-P1 git 写分级): 只有 `tool.git_write`
    /// 的 L2 (出网) 调用命中地板; L1 与其它能力不踩。
    #[test]
    fn human_approval_floor_covers_only_l2_git_write_calls() {
        let capability = CapabilityId::new("tool.git_write").unwrap();
        let other = CapabilityId::new("tool.apply_patch").unwrap();
        let push = serde_json::json!({ "command": "git push origin main" });
        let fetch = serde_json::json!({ "command": "git fetch origin" });
        let commit = serde_json::json!({ "command": "git commit -m x" });
        assert!(human_approval_floor_applies(&capability, &push));
        assert!(human_approval_floor_applies(&capability, &fetch));
        assert!(!human_approval_floor_applies(&capability, &commit));
        assert!(!human_approval_floor_applies(&other, &push));
    }

    /// M2 (行为面): read_only 会话下, 新纳入名单的能力必须被本 hook 显式
    /// 拒绝 (而不是落到 inner policy 的可能放行)。
    #[tokio::test]
    async fn read_only_preset_refuses_newly_covered_capabilities() {
        let (hook, store) = build_hook().await;
        let session = SessionId::new();
        save_session(&store, session, PermissionPreset::ReadOnly, false).await;
        let args = serde_json::json!({});

        for id in [
            "tool.fetch",
            "http.send",
            "credential.read",
            "fs.delete",
            "tool.worktree_apply",
        ] {
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

    /// D-P2 (行为面): `tool.worktree_apply` 的审批级语义按预设逐档钉死 ——
    /// read_only 拒 / standard 审批 (RequireApproval, 理由载荷 = 改单摘要:
    /// 文件数 / 动作分布 / 验证命令) / full 按预设语义放行 (本工具无审批地板)。
    /// 计划面 `tool.worktree_plan` 是只读能力, read_only 下照常可用。
    #[tokio::test]
    async fn worktree_apply_approval_tier_matches_each_permission_preset() {
        let inner: Arc<dyn GovernanceHook> = Arc::new(
            apeireth_tools_canonical::WorktreeApplyApprovalHook::new(std::env::temp_dir()),
        );
        let store: Arc<dyn SessionStore> = Arc::new(InMemorySessionStore::new());
        let hook = PermissionPresetGovernanceHook::new(inner, store.clone());
        let session = SessionId::new();
        let apply = CapabilityId::new("tool.worktree_apply").unwrap();
        let arguments = serde_json::json!({
            "plan": {
                "goal": "demo objective",
                "files": [
                    { "path": "a.txt", "action": "update", "expect": "updated body" }
                ]
            },
            "patch": "*** Begin Patch\n*** Update File: a.txt\n@@\n-old\n+new\n*** End Patch",
            "verify": [ { "program": "cargo", "args": ["--version"] } ]
        });

        // read_only: 预设显式拒 (写能力名单覆盖)。
        save_session(&store, session, PermissionPreset::ReadOnly, false).await;
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &apply, &arguments))
            .await;
        assert!(
            matches!(verdict.decision, Decision::Deny { .. }),
            "read_only must refuse tool.worktree_apply, got {:?}",
            verdict.decision
        );
        assert_eq!(verdict.hook, "permission_preset");

        // standard: 审批级 (RequireApproval), 载荷带改单摘要。
        save_session(&store, session, PermissionPreset::Standard, false).await;
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &apply, &arguments))
            .await;
        let Decision::RequireApproval { reason } = &verdict.decision else {
            panic!(
                "standard must keep tool.worktree_apply at require-approval, got {:?}",
                verdict.decision
            );
        };
        assert!(reason.contains("文件 1"), "审批卡载荷缺文件数: {reason}");
        assert!(
            reason.contains("add 0/update 1/delete 0"),
            "审批卡载荷缺动作分布: {reason}"
        );
        assert!(
            reason.contains("验证命令 1 条") && reason.contains("cargo --version"),
            "审批卡载荷缺验证命令: {reason}"
        );

        // full: 按预设语义 (RequireApproval→Allow 改写; 本工具无审批地板)。
        save_session(&store, session, PermissionPreset::Full, false).await;
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &apply, &arguments))
            .await;
        assert!(
            verdict.decision.is_allowed(),
            "full preset follows preset semantics for tool.worktree_apply, got {:?}",
            verdict.decision
        );

        // 计划面: 只读改单不入写能力名单, read_only 下照常。
        let plan = CapabilityId::new("tool.worktree_plan").unwrap();
        save_session(&store, session, PermissionPreset::ReadOnly, false).await;
        let verdict = hook
            .evaluate_verbose(&dispatch_request(session, &plan, &serde_json::json!({})))
            .await;
        assert!(
            verdict.decision.is_allowed(),
            "tool.worktree_plan is read-only and must stay allowed under read_only, got {:?}",
            verdict.decision
        );
    }
}
