//! Canonical pending-approval lifecycle types.
//!
//! Governance owns the decision *should this operation require approval?*.
//! Runtime owns *what is waiting, what exact operation is frozen, how it is
//! resumed, and whether it was already resolved*. This module contains the
//! runtime-owned domain objects, not a second governance policy.
//!
//! There is intentionally no `ApprovalManager`, `ApprovalRuntime`, or
//! `ApprovalExecutor` here. The types are inert data; `Runtime` is the only
//! orchestration root that acts on them.

use apeireth_core::kernel::{ApprovalId, CapabilityId, RequestId, SessionId, Timestamp, TraceId};
use apeireth_governance::action_binding::ActionBinding;
use apeireth_governance::approval_family::family_ids_for_arguments;
use apeireth_governance::{redact_secret_surfaces, TurnSecurityContext};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedUsage, ToolCall};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A human decision about one pending approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Execute the frozen operation exactly once.
    Approve,
    /// Do not execute the frozen operation.
    Reject {
        /// Optional human reason. Private by default; the model-facing result
        /// uses a fixed rejection message unless a future contract says
        /// otherwise.
        reason: Option<String>,
    },
    /// Cancel the pending operation without executing it.
    ///
    /// Semantically equivalent to rejection for transcript pairing: the frozen
    /// tool call receives a synthetic result and the turn may continue.
    Cancel {
        /// Optional human reason.
        reason: Option<String>,
    },
}

impl ApprovalDecision {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Reject { .. } => "rejected",
            Self::Cancel { .. } => "cancelled",
        }
    }
}

/// The lifecycle of one pending approval.
///
/// `Pending -> Claimed -> Consumed` is the approval path. `Claimed` is not
/// final: it means a human approved the operation and the runtime has durably
/// claimed it, but the external effect may not have happened (or may have
/// happened and the final `Consumed` save was interrupted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Waiting for a human decision.
    Pending,
    /// A human approved the operation and the runtime atomically claimed it.
    /// No other resolver may execute the operation again. Execution may or may
    /// not have occurred; only `Consumed` records that the result was appended.
    Claimed,
    /// A human rejected the operation. It will never execute.
    Rejected,
    /// The approval expired before a human resolved it. It will never execute.
    Expired,
    /// The approved operation was executed and its result was appended to the
    /// transcript.
    Consumed,
    /// A resolver re-opened a `Claimed` approval whose result was never
    /// recorded. The external effect is unknown and must not be retried
    /// automatically.
    Interrupted,
}

impl ApprovalStatus {
    /// True while the approval is waiting for a human decision.
    pub const fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }

    /// True once a human decision or an expiry transition has been reached.
    /// This includes `Claimed`, whose external effect is not yet final.
    pub const fn is_resolved(self) -> bool {
        !self.is_pending()
    }

    /// True when this approval can no longer move to `Consumed`.
    pub const fn is_final(self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Expired | Self::Consumed | Self::Interrupted
        )
    }

    /// Backwards-compatible alias for [`Self::is_final`].
    pub const fn is_terminal(self) -> bool {
        self.is_final()
    }
}

/// One approval-frozen operation of a turn, kept on the frozen continuation.
///
/// This is the turn's re-entry ledger: every operation (model-facing tool name
/// plus canonical arguments) that governance froze for human approval during
/// the turn is recorded here once. When the model proposes the *same* operation
/// again in the same turn, the runtime collapses the proposal onto this record
/// instead of re-entering the freeze machinery (one pending item per operation
/// per turn; a repeated proposal never mints a second approval).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrozenOperationRecord {
    /// Model-facing tool name of the frozen operation.
    pub tool_name: String,
    /// Content fingerprint of the frozen operation's arguments
    /// ([`operation_arguments_fingerprint`]).
    pub arguments_fingerprint: String,
    /// The approval item this operation was frozen onto.
    pub approval_id: ApprovalId,
    /// Round that produced the freeze. A collapsed repeat proposal does not
    /// advance the turn's charged round budget past this value.
    pub round: u32,
}

/// The exact point inside a turn where a pending approval must resume.
///
/// The provider transcript already contains the assistant tool-call message.
/// [`Self::next_tool_index`] points at the first tool call that has not been
/// executed yet. When the list is empty the turn is at the start of a round.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrozenTurnContinuation {
    pub request_id: RequestId,
    pub trace_id: TraceId,
    /// The model that was serving this turn when it paused.
    pub model: String,
    /// The round that was being executed.
    pub round: u32,
    /// The original assistant tool-call batch for the round. The model is
    /// never re-queried to regenerate these calls.
    pub tool_calls: Vec<ToolCall>,
    /// Index of the next tool call to process.
    pub next_tool_index: usize,
    /// When set, the tool call at this index was already approved and must be
    /// dispatched without a second governance evaluation.
    pub approved_tool_index: Option<usize>,
    /// When `approved_tool_index` is set, this is the approval that authorized
    /// it. The approval is moved to [`ApprovalStatus::Consumed`] after the tool
    /// has been invoked and its result appended.
    pub approved_approval_id: Option<ApprovalId>,
    /// Number of isolated module calls already spent by this turn.
    ///
    /// This is persisted with an approval pause so resolving the approval
    /// cannot reset the turn's side-call budget.
    #[serde(default)]
    pub module_invocations: usize,
    /// The turn's immutable intent/authorization context, frozen with the
    /// continuation.
    ///
    /// M27: without it the resumed turn ran `advance(..., None, ...)`, so
    /// every governance evaluation in the resumed round — and the guard's
    /// intent-alignment layer behind it — saw an *unbound* turn and treated
    /// it as "no intent", silently downgrading high-risk effects from
    /// Deny/RequireApproval to Allow (H3). The context is serializable, so
    /// it is persisted verbatim with the approval; `#[serde(default)]`
    /// keeps approvals persisted before this field loadable as `None`.
    #[serde(default)]
    pub security_context: Option<TurnSecurityContext>,
    /// Assistant text generated by this turn so far, carried across a pause so
    /// a pending close can hand the UI the preserved text instead of dropping
    /// what the model already said.
    #[serde(default)]
    pub generated_text: String,
    /// Provider capability that served the most recent round of this turn.
    #[serde(default)]
    pub served_by: Option<CapabilityId>,
    /// Token accounting of the most recent model response of this turn.
    #[serde(default)]
    pub usage: NormalizedUsage,
    /// The turn's approval-freeze ledger: every operation frozen for human
    /// approval in this turn, once each.
    #[serde(default)]
    pub frozen_operations: Vec<FrozenOperationRecord>,
}

impl FrozenTurnContinuation {
    /// A fresh continuation at the start of a round.
    pub fn start_of_round(
        request_id: RequestId,
        trace_id: TraceId,
        model: impl Into<String>,
        round: u32,
    ) -> Self {
        Self {
            request_id,
            trace_id,
            model: model.into(),
            round,
            tool_calls: Vec::new(),
            next_tool_index: 0,
            approved_tool_index: None,
            approved_approval_id: None,
            module_invocations: 0,
            security_context: None,
            generated_text: String::new(),
            served_by: None,
            usage: NormalizedUsage::default(),
            frozen_operations: Vec::new(),
        }
    }
}

/// One durable pending approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingApproval {
    pub approval_id: ApprovalId,
    pub session_id: SessionId,
    pub request_id: RequestId,
    pub trace_id: TraceId,
    /// The runtime round that produced the pending operation.
    pub round: u32,
    /// The capability whose dispatch required approval.
    pub capability_id: CapabilityId,
    /// The model-facing tool name.
    pub tool_name: String,
    /// The frozen, immutable tool call. Resolvers may not substitute a new
    /// tool name, arguments, cwd, or any other execution parameter.
    pub tool_call: ToolCall,
    /// Capability-frozen effective invocation, when the capability supplied
    /// one. This binds derived security-relevant values (for example shell
    /// cwd, shell executable, timeout, and environment profile) beyond the raw
    /// provider tool arguments. This is the executable payload and may contain
    /// environment values required to execute; it is never shown directly in
    /// approval views.
    pub effective_invocation: Option<FrozenInvocation>,
    /// Governance hook that produced `RequireApproval`.
    pub governance_hook: String,
    /// Governance reason that is shown to the human.
    pub governance_reason: String,
    /// One-line human-readable command text, e.g.
    /// `shell: bash -c 'ls -la /tmp'`.
    ///
    /// `#[serde(default)]`: approvals persisted before this field existed
    /// (pre-2026-09-12 session blobs) must still deserialize — a missing field
    /// is an empty display string, never a load failure.
    #[serde(default)]
    pub command_text: String,
    /// Shorter human summary, e.g. `执行命令 ls -la /tmp`.
    #[serde(default)]
    pub arguments_summary: String,
    /// 审批卡族标注 / 本记录的授权对象（E-P2 审批族归一）：被批命令的**族**
    /// （族 id 词表，如 `["删除族"]`）。批"删除族" = 该族所有形态生效，
    /// 同族换形重放被本记录覆盖（不重批），跨族仍逐族审批。
    /// 空 = 非命令面操作（授权对象 = 操作本身，逐操作授权）。
    ///
    /// `#[serde(default)]`：本字段加入前持久化的存量记录（逐字命令形态）
    /// 读取时按**迁移语义**由 [`PendingApproval::effective_authorization_families`]
    /// 映射生效（命令→族映射后生效，不失效重批）。
    #[serde(default)]
    pub authorization_families: Vec<String>,
    /// G2.5 动作绑定（审批时冻结）：被批动作内容（操作 / 执行主体 / 目标 /
    /// 工具面版本 / 结构化 argv）的**绑定**，摘要随记录落库（跨重启持久）。
    /// 执行边界重算比对（[`approval_action_binding`]），载荷被改（参数 / 目标 /
    /// 顺序变）→ 摘要不符 → 拒绝即帧 + `approval.binding_mismatch` 审计。
    ///
    /// `#[serde(default)]`：本字段加入前持久化的存量记录读入为 `None` ——
    /// 迁移面**不给假摘要**，执行边界退为逐字载荷比对（仍拒载荷替换）。
    #[serde(default)]
    pub action_binding: Option<ActionBinding>,
    /// Canonical fingerprint over every operation-relevant field.
    pub operation_fingerprint: String,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
    pub status: ApprovalStatus,
    /// Exact point the turn will resume from when this approval is resolved.
    pub continuation: FrozenTurnContinuation,
    /// Optional human reason recorded at resolution time.
    pub human_reason: Option<String>,
    /// Assistant text generated by the turn before it paused, preserved so the
    /// pending state can hand the UI the generated text instead of dropping it.
    #[serde(default)]
    pub generated_text: String,
    /// How many round slots the turn had consumed when it paused for this
    /// approval. The pause itself never burns further round budget.
    #[serde(default)]
    pub rounds_used: u32,
}

impl PendingApproval {
    pub fn is_expired(&self, now: Timestamp) -> bool {
        now > self.expires_at
    }

    /// 本记录的授权族（E-P2，二选一钉死 = **迁移生效**）：
    /// 新记录冻结时算好存在 `authorization_families`；存量记录（该字段加入前
    /// 持久化的逐字命令形态，读入后为空）按迁移语义由 `tool_call` 的逐字命令
    /// 映射生效 —— 老记录的命令→族映射后生效，不是失效重批。
    pub fn effective_authorization_families(&self) -> Vec<String> {
        if !self.authorization_families.is_empty() {
            return self.authorization_families.clone();
        }
        family_ids_for_arguments(&self.tool_call.arguments)
    }

    /// 本记录的动作绑定摘要（G2.5）：审批时冻结、随记录落库（跨重启持久）。
    /// 存量记录（[`PendingApproval::action_binding`] 字段加入前持久化）为
    /// `None` —— 迁移面不给假摘要，执行边界退为逐字载荷比对。
    pub fn action_binding_digest(&self) -> Option<String> {
        self.action_binding.as_ref().map(ActionBinding::digest)
    }
}

/// A provider- and adapter-facing view of one pending approval.
///
/// This is deliberately smaller than [`PendingApproval`]: it exposes the exact
/// operation that must be approved and nothing else. It never exposes secret
/// values beyond the operation itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingApprovalView {
    pub approval_id: ApprovalId,
    pub session_id: SessionId,
    pub request_id: RequestId,
    pub trace_id: TraceId,
    pub round: u32,
    pub capability_id: CapabilityId,
    pub tool_name: String,
    pub tool_call: ToolCall,
    /// Redacted, human-facing capability-frozen effective invocation, when one
    /// was supplied. This is [`PendingApproval::effective_invocation`]'s
    /// display payload, never the raw executable payload.
    pub effective_invocation: Option<serde_json::Value>,
    pub governance_hook: String,
    pub governance_reason: String,
    /// One-line human-readable command text (from [`PendingApproval`]).
    #[serde(default)]
    pub command_text: String,
    /// Shorter human summary (from [`PendingApproval`]).
    #[serde(default)]
    pub arguments_summary: String,
    /// 审批卡族标注 / 授权对象（E-P2，取自
    /// [`PendingApproval::effective_authorization_families`]，存量记录按
    /// 迁移语义映射后随视图给出）。
    #[serde(default)]
    pub authorization_families: Vec<String>,
    /// G2.5 动作摘要（短摘要即全量载荷的 SHA-256 hex）：审批卡"动作摘要"行
    /// 与执行边界比对的锚。存量记录（无冻结绑定）为空串 —— 明示不给假摘要。
    /// 视图只出摘要，不出绑定载荷（短摘要非全量载荷）。
    #[serde(default)]
    pub action_binding_digest: String,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
    pub operation_fingerprint: String,
    /// Assistant text generated by the turn before it paused (from
    /// [`PendingApproval`]): the pending state preserves it for the UI.
    #[serde(default)]
    pub generated_text: String,
    /// Round slots the turn consumed when it paused (from
    /// [`PendingApproval`]).
    #[serde(default)]
    pub rounds_used: u32,
}

impl From<&PendingApproval> for PendingApprovalView {
    fn from(value: &PendingApproval) -> Self {
        Self {
            approval_id: value.approval_id,
            session_id: value.session_id,
            request_id: value.request_id,
            trace_id: value.trace_id,
            round: value.round,
            capability_id: value.capability_id.clone(),
            tool_name: value.tool_name.clone(),
            tool_call: value.tool_call.clone(),
            effective_invocation: value
                .effective_invocation
                .as_ref()
                .map(|frozen| frozen.display.clone()),
            governance_hook: value.governance_hook.clone(),
            governance_reason: value.governance_reason.clone(),
            command_text: value.command_text.clone(),
            arguments_summary: value.arguments_summary.clone(),
            authorization_families: value.effective_authorization_families(),
            action_binding_digest: value.action_binding_digest().unwrap_or_default(),
            created_at: value.created_at,
            expires_at: value.expires_at,
            operation_fingerprint: value.operation_fingerprint.clone(),
            generated_text: value.generated_text.clone(),
            rounds_used: value.rounds_used,
        }
    }
}

/// Build a one-line human-readable command text for an approval payload.
///
/// It prefers the redacted frozen-invocation display (the executable shape a
/// human actually approves) and falls back to the raw tool arguments.
///
/// 脱敏 (D-P1): 成品文本过 [`redact_secret_surfaces`] —— 远端 URL 凭据/密钥
/// 赋值/凭据形状键值不入审批卡、审批面板与审计记录 (命令文本是人读面+日志面)。
pub fn approval_command_text(
    tool_name: &str,
    call: &ToolCall,
    effective_invocation: Option<&FrozenInvocation>,
) -> String {
    let command = effective_invocation
        .and_then(|frozen| shell_command_from_display(&frozen.display))
        .or_else(|| command_from_arguments(&call.arguments))
        .unwrap_or_else(|| compact_json(&call.arguments));
    redact_secret_surfaces(&format!("{}: {}", tool_name, command))
}

/// Build a shorter human summary of one approval's arguments.
///
/// This is intentionally heuristic and deterministic: it never invokes a model
/// and never exposes secret values beyond what the approval already shows.
/// 同 [`approval_command_text`] 的脱敏口径: 秘密面不入审批载荷。
pub fn approval_arguments_summary(tool_name: &str, call: &ToolCall) -> String {
    let args = &call.arguments;
    // 先脱敏后截断: 截断发生在半截凭据 URL 上会让脱敏正则漏配 (半截仍泄密)。
    if let Some(command) = args.get("command").and_then(|value| value.as_str()) {
        return format!(
            "执行命令 {}",
            bounded(&redact_secret_surfaces(command), 120)
        );
    }
    if let (Some(operation), Some(path)) = (
        args.get("operation").and_then(|value| value.as_str()),
        args.get("path").and_then(|value| value.as_str()),
    ) {
        return format!(
            "{} {}",
            operation,
            bounded(&redact_secret_surfaces(path), 120)
        );
    }
    if let Some(path) = args.get("path").and_then(|value| value.as_str()) {
        return format!("操作 {}", bounded(&redact_secret_surfaces(path), 120));
    }
    format!(
        "{} {}",
        tool_name,
        bounded(&redact_secret_surfaces(&compact_json(args)), 120)
    )
}

fn shell_command_from_display(display: &serde_json::Value) -> Option<String> {
    let executable = display.get("shell_executable")?.as_str()?;
    let args = display
        .get("shell_args")?
        .as_array()?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    Some(if args.is_empty() {
        executable.to_string()
    } else {
        format!("{} {}", executable, args)
    })
}

fn command_from_arguments(args: &serde_json::Value) -> Option<String> {
    args.get("command")
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn compact_json(value: &serde_json::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "{}".into())
}

/// Canonical deterministic JSON representation used for fingerprints.
///
/// G2.5 起委托治理层单一来源（`apeireth_governance::action_binding`，自研
/// 规范化函数）：对象键排序 / 数组保序 / 字符串最小转义 / 零空白，
/// 字节语义由治理层测试向量钉死。
pub fn canonical_json_bytes(value: &serde_json::Value) -> Vec<u8> {
    apeireth_governance::action_binding::canonical_json_bytes(value)
}

/// 工具面版本摘要（G2.5 ActionBinding 的 `tool_schema_version`）：
/// 工具声明（name / description / parameters / strict）的规范化摘要。
/// 工具面变更 → 摘要变 → 既有批准失效（执行边界重算比对）。
pub fn tool_schema_version_digest(tool: &dyn ToolCapability) -> String {
    let declaration = serde_json::to_value(tool.declaration()).unwrap_or(serde_json::Value::Null);
    let mut hasher = Sha256::new();
    hasher.update(canonical_json_bytes(&declaration));
    format!("{:x}", hasher.finalize())
}

/// 运行时动作绑定取值（G2.5）：
///
/// * `operation` = 模型面工具名；
/// * `agent` = 会话身份；
/// * `target` = 目标派生（[`ActionBinding::derive_target`]：`command` / `path`）；
/// * `tool_schema_version` = [`tool_schema_version_digest`]；
/// * `parameters` = `{arguments, invocation}` —— 结构化 argv 与执行载荷全集
///   （数组顺序是语义的一部分：改参数 / 改目标 / 改顺序都得不出同一摘要）。
pub fn approval_action_binding(
    tool_name: &str,
    session_id: SessionId,
    tool_schema_version: &str,
    arguments: &serde_json::Value,
    effective_invocation: Option<&FrozenInvocation>,
) -> ActionBinding {
    let target = ActionBinding::derive_target(arguments);
    let parameters = serde_json::json!({
        "arguments": arguments,
        "invocation": effective_invocation.map(|frozen| frozen.payload.clone()),
    });
    ActionBinding::new(
        tool_name,
        session_id.to_string(),
        target,
        tool_schema_version,
        parameters,
    )
}

/// Compute the canonical operation fingerprint for one tool dispatch.
///
/// The field set is intentionally generic so every capability dispatch —
/// including a future shell invocation — binds the same core identity.
pub fn operation_fingerprint(
    action_kind: &str,
    capability_id: &CapabilityId,
    tool_name: &str,
    tool_call_id: &str,
    arguments: &serde_json::Value,
    session_id: SessionId,
    request_id: RequestId,
    round: u32,
) -> String {
    operation_fingerprint_with_invocation(
        action_kind,
        capability_id,
        tool_name,
        tool_call_id,
        arguments,
        None,
        session_id,
        request_id,
        round,
    )
}

/// Compute the canonical operation fingerprint including a capability-frozen
/// effective invocation, when one exists.
pub fn operation_fingerprint_with_invocation(
    action_kind: &str,
    capability_id: &CapabilityId,
    tool_name: &str,
    tool_call_id: &str,
    arguments: &serde_json::Value,
    effective_invocation: Option<&serde_json::Value>,
    session_id: SessionId,
    request_id: RequestId,
    round: u32,
) -> String {
    let mut hasher = Sha256::new();
    update_len_prefixed(&mut hasher, action_kind.as_bytes());
    update_len_prefixed(&mut hasher, capability_id.as_str().as_bytes());
    update_len_prefixed(&mut hasher, tool_name.as_bytes());
    update_len_prefixed(&mut hasher, tool_call_id.as_bytes());
    update_len_prefixed(&mut hasher, &canonical_json_bytes(arguments));
    if let Some(effective) = effective_invocation {
        update_len_prefixed(&mut hasher, &canonical_json_bytes(effective));
    } else {
        update_len_prefixed(&mut hasher, b"null");
    }
    update_len_prefixed(&mut hasher, session_id.to_string().as_bytes());
    update_len_prefixed(&mut hasher, request_id.to_string().as_bytes());
    hasher.update(round.to_be_bytes());
    format!("{:x}", hasher.finalize())
}

fn update_len_prefixed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

/// Content fingerprint of one proposed operation: tool name plus canonical
/// arguments, nothing else.
///
/// Two proposals of the same tool with semantically equal arguments (key order
/// irrelevant) map to the same fingerprint, so a repeated proposal is
/// recognizable as the same operation. The fingerprint binds no session,
/// request, round, or tool-call id — those differ across the repeats this
/// exists to detect.
pub fn operation_arguments_fingerprint(tool_name: &str, arguments: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    update_len_prefixed(&mut hasher, tool_name.as_bytes());
    update_len_prefixed(&mut hasher, &canonical_json_bytes(arguments));
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;
    use apeireth_governance::TaskIntentEnvelopeV1;

    #[test]
    fn canonical_json_object_key_order_is_irrelevant() {
        let a = serde_json::json!({ "b": 1, "a": [true, null, "x"] });
        let b = serde_json::json!({ "a": [true, null, "x"], "b": 1 });
        assert_eq!(canonical_json_bytes(&a), canonical_json_bytes(&b));
    }

    #[test]
    fn canonical_json_array_order_is_preserved() {
        let a = serde_json::json!([1, 2]);
        let b = serde_json::json!([2, 1]);
        assert_ne!(canonical_json_bytes(&a), canonical_json_bytes(&b));
    }

    #[test]
    fn fingerprint_is_deterministic_for_equivalent_objects() {
        let session = SessionId::new();
        let request = RequestId::new();
        let cap = CapabilityId::new("tool.example").unwrap();
        let args_a = serde_json::json!({ "command": "echo hi", "cwd": "/tmp" });
        let args_b = serde_json::json!({ "cwd": "/tmp", "command": "echo hi" });

        let a = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_1",
            &args_a,
            session,
            request,
            2,
        );
        let b = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_1",
            &args_b,
            session,
            request,
            2,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_changes_with_any_operation_field() {
        let session = SessionId::new();
        let request = RequestId::new();
        let cap = CapabilityId::new("tool.example").unwrap();
        let args = serde_json::json!({ "command": "echo hi" });

        let base = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_1",
            &args,
            session,
            request,
            1,
        );

        let changed_scalar = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_1",
            &serde_json::json!({ "command": "echo bye" }),
            session,
            request,
            1,
        );
        assert_ne!(base, changed_scalar);

        let changed_capability = operation_fingerprint(
            "capability_dispatch",
            &CapabilityId::new("tool.other").unwrap(),
            "example",
            "call_1",
            &args,
            session,
            request,
            1,
        );
        assert_ne!(base, changed_capability);

        let changed_call = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_2",
            &args,
            session,
            request,
            1,
        );
        assert_ne!(base, changed_call);
    }

    #[test]
    fn approval_command_text_prefers_frozen_shell_display() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: serde_json::json!({ "command": "ls -la /tmp" }),
        };
        let frozen = FrozenInvocation::new(
            serde_json::json!({ "command": "ls -la /tmp" }),
            serde_json::json!({
                "shell_executable": "bash",
                "shell_args": ["-c", "ls -la /tmp"],
            }),
        );

        let text = approval_command_text("shell", &call, Some(&frozen));
        assert!(text.starts_with("shell: bash -c"), "{text}");
        assert!(text.contains("ls -la /tmp"), "{text}");

        let summary = approval_arguments_summary("shell", &call);
        assert!(summary.contains("ls -la /tmp"), "{summary}");
    }

    #[test]
    fn approval_command_text_falls_back_to_raw_arguments() {
        let call = ToolCall {
            id: "call_2".into(),
            name: "filesystem".into(),
            arguments: serde_json::json!({ "operation": "read", "path": "notes.txt" }),
        };

        let text = approval_command_text("filesystem", &call, None);
        assert!(text.starts_with("filesystem: "), "{text}");
        assert!(
            text.contains("read") && text.contains("notes.txt"),
            "{text}"
        );

        let summary = approval_arguments_summary("filesystem", &call);
        assert_eq!(summary, "read notes.txt");
    }

    /// 脱敏 (D-P1): 远端 URL 凭据不入审批载荷 (命令文本 + 参数摘要)。
    #[test]
    fn approval_payloads_redact_remote_url_credentials() {
        let call = ToolCall {
            id: "call_3".into(),
            name: "git_write".into(),
            arguments: serde_json::json!({
                "command": "git remote add origin https://user:tok123456@host.example/repo.git"
            }),
        };

        let text = approval_command_text("git_write", &call, None);
        assert!(!text.contains("tok123456"), "{text}");
        assert!(!text.contains("user:"), "{text}");
        assert!(text.contains("[REDACTED_URL_CREDENTIAL]"), "{text}");
        assert!(text.contains("git remote add origin"), "{text}");

        let summary = approval_arguments_summary("git_write", &call);
        assert!(!summary.contains("tok123456"), "{summary}");
        assert!(!summary.contains("user:"), "{summary}");
    }

    #[test]
    fn pending_approval_view_serializes_command_text_and_summary() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: serde_json::json!({ "command": "ls -la /tmp" }),
        };
        let view = PendingApprovalView {
            approval_id: ApprovalId::new(),
            session_id: SessionId::new(),
            request_id: RequestId::new(),
            trace_id: TraceId::new(),
            round: 1,
            capability_id: CapabilityId::new("tool.example").unwrap(),
            tool_name: "shell".into(),
            tool_call: call.clone(),
            effective_invocation: None,
            governance_hook: "hook".into(),
            governance_reason: "reason".into(),
            command_text: approval_command_text("shell", &call, None),
            arguments_summary: approval_arguments_summary("shell", &call),
            authorization_families: vec!["删除族".to_string()],
            action_binding_digest: String::new(),
            created_at: Timestamp::from_epoch_millis(1_700_000_000_000).unwrap(),
            expires_at: Timestamp::from_epoch_millis(1_700_000_300_000).unwrap(),
            operation_fingerprint: "fp".into(),
            generated_text: String::new(),
            rounds_used: 1,
        };

        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["command_text"], "shell: ls -la /tmp");
        assert_eq!(json["arguments_summary"], "执行命令 ls -la /tmp");
        assert_eq!(json["authorization_families"][0], "删除族");
    }

    /// Regression for the 2026-09-12 real-machine failure: sessions persisted
    /// BEFORE `command_text`/`arguments_summary` existed must still load —
    /// a missing field is an empty display string, never
    /// `missing field \`command_text\``.
    #[test]
    fn approval_view_serialized_before_command_text_still_deserializes() {
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: serde_json::json!({ "command": "ls -la /tmp" }),
        };
        let view = PendingApprovalView {
            approval_id: ApprovalId::new(),
            session_id: SessionId::new(),
            request_id: RequestId::new(),
            trace_id: TraceId::new(),
            round: 1,
            capability_id: CapabilityId::new("tool.example").unwrap(),
            tool_name: "shell".into(),
            tool_call: call,
            effective_invocation: None,
            governance_hook: "hook".into(),
            governance_reason: "reason".into(),
            command_text: String::new(),
            arguments_summary: String::new(),
            authorization_families: Vec::new(),
            action_binding_digest: String::new(),
            created_at: Timestamp::from_epoch_millis(1_700_000_000_000).unwrap(),
            expires_at: Timestamp::from_epoch_millis(1_700_000_300_000).unwrap(),
            operation_fingerprint: "fp".into(),
            generated_text: String::new(),
            rounds_used: 1,
        };
        let mut json = serde_json::to_value(&view).unwrap();
        let object = json.as_object_mut().unwrap();
        object.remove("command_text");
        object.remove("arguments_summary");

        let migrated: PendingApprovalView = serde_json::from_value(json).unwrap();
        assert_eq!(migrated.command_text, "");
        assert_eq!(migrated.arguments_summary, "");
        assert_eq!(migrated.tool_name, "shell");
    }

    /// E-P2 审批卡族标注: 冻结记录的授权族随视图给出（族词表 id，
    /// 授权对象 = 族），审批卡渲染面据此标注。
    #[test]
    fn approval_view_carries_the_family_annotation() {
        let pending = pending_delete_approval(vec!["删除族".to_string()]);
        let view = PendingApprovalView::from(&pending);
        assert_eq!(view.authorization_families, vec!["删除族"]);
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["authorization_families"][0], "删除族");
    }

    /// E-P2 存量记录迁移语义（二选一钉死 = **迁移生效**）：`authorization_families`
    /// 字段加入前持久化的逐字命令形态记录（读入后字段为空）按族解释生效 ——
    /// 老记录的命令→族映射后生效，不是失效重批。
    #[test]
    fn persisted_records_before_the_family_field_migrate_by_interpretation() {
        let pending = pending_delete_approval(Vec::new());
        // 序列化面先去掉字段，模拟存量 session blob。
        let mut json = serde_json::to_value(&pending).unwrap();
        json.as_object_mut()
            .unwrap()
            .remove("authorization_families");
        let migrated: PendingApproval = serde_json::from_value(json).unwrap();
        assert!(migrated.authorization_families.is_empty());
        assert_eq!(
            migrated.effective_authorization_families(),
            vec!["删除族"],
            "存量逐字记录必须按族映射后生效（迁移生效）"
        );
        let view = PendingApprovalView::from(&migrated);
        assert_eq!(view.authorization_families, vec!["删除族"]);
    }

    /// G2.5 跨重启持久: 动作摘要随审批记录落库 —— 序列化 round-trip 后
    /// 绑定与摘要原样存活, 视图只出摘要（短摘要非全量载荷）。
    #[test]
    fn action_binding_digest_persists_with_the_approval_record() {
        let pending = pending_delete_approval(vec!["删除族".to_string()]);
        let digest = pending
            .action_binding_digest()
            .expect("new records freeze a binding");

        let blob = serde_json::to_string(&pending).unwrap();
        let restored: PendingApproval = serde_json::from_str(&blob).unwrap();
        assert_eq!(
            restored.action_binding_digest(),
            Some(digest.clone()),
            "摘要必须随记录跨重启存活"
        );
        assert_eq!(restored.action_binding.as_ref().unwrap().digest(), digest);

        let view = PendingApprovalView::from(&restored);
        assert_eq!(view.action_binding_digest, digest);
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["action_binding_digest"], digest);

        // 载荷被改（参数变）→ 重算摘要不符（执行边界拒绝的依据）。
        let mut tampered = restored.clone();
        tampered.tool_call.arguments = serde_json::json!({ "command": "Remove-Item y.txt" });
        let live = approval_action_binding(
            "shell",
            restored.session_id,
            "tool-schema-v1",
            &tampered.tool_call.arguments,
            None,
        );
        assert_ne!(live.digest(), digest, "改参数必须得出不同摘要");
    }

    /// G2.5 存量记录迁移面（不给假摘要）: 绑定字段加入前持久化的记录读入为
    /// `None`, 摘要为空 —— 执行边界退为逐字载荷比对, 不伪造未冻结的摘要。
    #[test]
    fn records_before_the_binding_field_migrate_without_a_fake_digest() {
        let pending = pending_delete_approval(Vec::new());
        let mut json = serde_json::to_value(&pending).unwrap();
        json.as_object_mut().unwrap().remove("action_binding");
        let migrated: PendingApproval = serde_json::from_value(json).unwrap();
        assert!(migrated.action_binding.is_none());
        assert_eq!(migrated.action_binding_digest(), None);
        let view = PendingApprovalView::from(&migrated);
        assert!(view.action_binding_digest.is_empty());
        let view_json = serde_json::to_value(&view).unwrap();
        assert_eq!(view_json["action_binding_digest"], "");

        // 视图序列化面兼容: 旧载荷无摘要字段照读。
        let mut old_view = serde_json::to_value(&view).unwrap();
        old_view
            .as_object_mut()
            .unwrap()
            .remove("action_binding_digest");
        let restored: PendingApprovalView = serde_json::from_value(old_view).unwrap();
        assert!(restored.action_binding_digest.is_empty());
    }

    /// 测试件: 一条 `Remove-Item x.txt`（删除族）待批记录。
    fn pending_delete_approval(authorization_families: Vec<String>) -> PendingApproval {
        let call = ToolCall {
            id: "call_1".into(),
            name: "shell".into(),
            arguments: serde_json::json!({ "command": "Remove-Item x.txt" }),
        };
        let created = Timestamp::from_epoch_millis(1_700_000_000_000).unwrap();
        let session_id = SessionId::new();
        PendingApproval {
            approval_id: ApprovalId::new(),
            session_id,
            request_id: RequestId::new(),
            trace_id: TraceId::new(),
            round: 1,
            capability_id: CapabilityId::new("tool.example").unwrap(),
            tool_name: "shell".into(),
            tool_call: call.clone(),
            effective_invocation: None,
            governance_hook: "hook".into(),
            governance_reason: "approval list matched [family:删除族]".into(),
            command_text: approval_command_text("shell", &call, None),
            arguments_summary: approval_arguments_summary("shell", &call),
            authorization_families,
            action_binding: Some(approval_action_binding(
                "shell",
                session_id,
                "tool-schema-v1",
                &call.arguments,
                None,
            )),
            operation_fingerprint: "fp".into(),
            created_at: created,
            expires_at: Timestamp::from_epoch_millis(1_700_000_300_000).unwrap(),
            status: ApprovalStatus::Pending,
            continuation: FrozenTurnContinuation::start_of_round(
                RequestId::new(),
                TraceId::new(),
                "m",
                1,
            ),
            human_reason: None,
            generated_text: String::new(),
            rounds_used: 1,
        }
    }

    /// M27 兼容性: `security_context` 字段加入前持久化的 approval (session
    /// blob) 必须仍可加载 —— 缺失字段是 `None`, 不是反序列化失败。
    #[test]
    fn frozen_continuation_without_security_context_still_deserializes() {
        let mut continuation =
            FrozenTurnContinuation::start_of_round(RequestId::new(), TraceId::new(), "m", 1);
        continuation.security_context = None;
        let json = serde_json::to_value(&continuation).unwrap();
        let mut object = json.as_object().unwrap().clone();
        object.remove("security_context");

        let migrated: FrozenTurnContinuation =
            serde_json::from_value(serde_json::Value::Object(object)).unwrap();
        assert_eq!(migrated.security_context, None);
        assert_eq!(migrated.round, 1);
    }

    /// M27: 冻结的 security context 随 continuation  round-trip (含 intent)。
    #[test]
    fn frozen_continuation_round_trips_its_security_context() {
        let intent = TaskIntentEnvelopeV1::unknown("s", "t");
        let context = TurnSecurityContext::new(intent.intent_id.clone(), "").with_intent(intent);
        let mut continuation =
            FrozenTurnContinuation::start_of_round(RequestId::new(), TraceId::new(), "m", 3);
        continuation.security_context = Some(context);

        let restored: FrozenTurnContinuation =
            serde_json::from_str(&serde_json::to_string(&continuation).unwrap()).unwrap();
        assert_eq!(
            restored
                .security_context
                .as_ref()
                .and_then(|c| c.intent.as_ref())
                .map(|intent| intent.intent_id.clone()),
            Some(format!("intent:t")),
            "the frozen intent must survive persistence"
        );
    }

    #[test]
    fn approval_status_terminals_are_clear() {
        assert!(ApprovalStatus::Pending.is_pending());
        assert!(!ApprovalStatus::Pending.is_resolved());
        assert!(!ApprovalStatus::Pending.is_final());
        assert!(!ApprovalStatus::Pending.is_terminal());

        assert!(!ApprovalStatus::Claimed.is_pending());
        assert!(ApprovalStatus::Claimed.is_resolved());
        assert!(
            !ApprovalStatus::Claimed.is_final(),
            "Claimed is not final: it must still be able to move to Consumed"
        );
        assert!(!ApprovalStatus::Claimed.is_terminal());

        assert!(ApprovalStatus::Rejected.is_final());
        assert!(ApprovalStatus::Expired.is_final());
        assert!(ApprovalStatus::Consumed.is_final());
        assert!(ApprovalStatus::Interrupted.is_final());
    }
}
