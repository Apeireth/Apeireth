//! 生物机制族 W3.3 · 游戏练习场工具面 (`tool.play_propose` / `tool.play_start` /
//! `tool.play_finish` / `tool.play_abort`)。
//!
//! # 机制
//!
//! 内核 [`apeireth_organ::play`] (W2 已实现, 本批**零改动**) 把「游戏」缩成三段:
//! 按风险/预算提出受控练习提案 → 经用户 opt-in 后有边界地开一场 → 结束只提炼技能
//! 与产物。本模块只做**模型可调的四个出口**, 不新增任何内核语义:
//!
//! | 工具 | 语义 | 副作用 |
//! |---|---|---|
//! | `tool.play_propose` | 候选主题 → 政策过滤 → 一份可审阅提案 (或结构化 null + 原因) | 零 (只读账本 policy) |
//! | `tool.play_start` | 提案 + **显式** `user_opt_in=true` + session_id + sandbox_dir → 开场 | 内存账本 +1 session |
//! | `tool.play_finish` | session_id + 产出 (learned / artifacts 元数据 / budget_used) → `PlayOutcome` | 内存账本状态推进 |
//! | `tool.play_abort` | session_id → 中止 (幂等) | 内存账本状态推进 |
//!
//! # 注册 (默认关, 懒加载)
//!
//! 装配门 = env [`BIO_PLAY_ENABLE_ENV`] (`APEIRETH_ENABLE_BIO_PLAY`, **只认 `"1"`**,
//! 未设置/空/`0`/`true`/其它一律关 = fail-closed) **且** 已登记工作区根注入。
//! 全关 = 不构造 = 不注册 (四族零构造, 装配结果零变化)。开时注册四个模块, 四个
//! 能力共享**同一本** [`apeireth_organ::play::PlayLedger`] (`Arc<Mutex<_>>`, 模块内
//! 自持; 内核不落盘, 本模块也不落盘 —— **重启丢 session**, 不冒充持久)。
//!
//! # 治理与诚实边界 (0 装)
//!
//! - **四件套均为执行档** (非只读): 它们推进练习状态机。审批链由适配器在治理层
//!   接线; 本文件提供 [`PlayApprovalHook`] (RequireApproval 级, 无自动放行档),
//!   但**装配根不注册本钩子** —— 本批无 gateway/CLI 手测, 也**没有 IM opt-in 卡**。
//!   工具面**唯一强制门槛** = env 闸 + `user_opt_in` 必须显式为 `true`
//!   (缺省/`false` 一律 `OptInRequired` 拒绝即帧)。
//! - **不提供自由 shell / 写文件能力**: 四件套的参数里没有命令、没有写入动作;
//!   `sandbox_dir` 只是**受控目录标识** (相对路径, 必须在已登记工作区根内, 只做路径
//!   纪律与包含性证明, **不是 OS 级隔离**, 也不创建目录)。产物只经
//!   [`PlayOutcome::artifacts`] 的字符串元数据带出, **不落地、不写主体记忆**:
//!   本文件不存在任何 memory / episode / wiki 写入调用。
//! - **不冒充 DEFAULT ENABLED**: 默认关; 未过的 live E2E 不写「已上线」。

#![forbid(unsafe_code)]

use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::kernel::{CapabilityId, Clock};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_organ::play::{
    PlayContext, PlayError, PlayLedger, PlayOutcome, PlayPolicy, PlayProposal, PlaySession,
    PlayStatus, PlayTopic,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};

use super::capability::CapabilityProvider;

/// 装配开闸 env 名 (仅显式 `"1"` 开, 其余一律关 fail-closed)。
pub const BIO_PLAY_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_PLAY";

/// 提案面能力标识。
pub const PLAY_PROPOSE_CAPABILITY_ID: &str = "tool.play_propose";
/// 开场能力标识。
pub const PLAY_START_CAPABILITY_ID: &str = "tool.play_start";
/// 结束能力标识。
pub const PLAY_FINISH_CAPABILITY_ID: &str = "tool.play_finish";
/// 中止能力标识。
pub const PLAY_ABORT_CAPABILITY_ID: &str = "tool.play_abort";

/// 提案面模型侧声明名。
pub const PLAY_PROPOSE_TOOL_NAME: &str = "play_propose";
/// 开场模型侧声明名。
pub const PLAY_START_TOOL_NAME: &str = "play_start";
/// 结束模型侧声明名。
pub const PLAY_FINISH_TOOL_NAME: &str = "play_finish";
/// 中止模型侧声明名。
pub const PLAY_ABORT_TOOL_NAME: &str = "play_abort";

/// 四件套能力标识 (闭合词表; 审批钩子与装配断言共用)。
pub const PLAY_CAPABILITY_IDS: &[&str] = &[
    PLAY_PROPOSE_CAPABILITY_ID,
    PLAY_START_CAPABILITY_ID,
    PLAY_FINISH_CAPABILITY_ID,
    PLAY_ABORT_CAPABILITY_ID,
];

/// 一次提案最多接受的候选主题数 (防无界输入; 超出拒绝即帧)。
pub const MAX_PROPOSAL_CANDIDATES: usize = 32;
/// 单个候选主题最多接受的技能目标数 (超出拒绝即帧)。
pub const MAX_SKILLS_PER_TOPIC: usize = 32;
/// 结束面最多接受的 learned / artifacts 条目数 (元数据, 超出拒绝即帧)。
pub const MAX_OUTCOME_ITEMS: usize = 64;
/// 单个 learned / artifact 字符串的最大长度 (字节; 超出拒绝即帧)。
pub const MAX_OUTCOME_ITEM_BYTES: usize = 512;
/// `sandbox_dir` 相对路径的最大长度 (字节; 超出拒绝即帧)。
pub const MAX_SANDBOX_DIR_BYTES: usize = 512;

/// 开关解析 (纯函数): 仅显式 `"1"` 开; 未设置 / 空 / `0` / `true` / `yes` /
/// `on` / 任意其它值一律关 (默认关 fail-closed)。`trim` 后比较, 因此 `" 1 "` 算开。
pub fn bio_play_enabled_from_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// 共享练习账本状态 (四个工具模块共持**同一本**账本)。
///
/// 账本只在内存: 内核不落盘, 本模块也不落盘 —— **进程重启丢全部 session**,
/// 不冒充持久。时间由注入 [`Clock`] 现读 (内核不读时钟, `now_ms` 一律调用方给)。
pub struct BioPlayState {
    ledger: Mutex<PlayLedger>,
    workspace_root: PathBuf,
    clock: Arc<dyn Clock>,
}

impl BioPlayState {
    /// 绑定已登记工作区根与注入时钟 (空账本, 默认 policy)。
    pub fn new(workspace_root: impl Into<PathBuf>, clock: Arc<dyn Clock>) -> Self {
        Self {
            ledger: Mutex::new(PlayLedger::default()),
            workspace_root: workspace_root.into(),
            clock,
        }
    }

    /// 已登记工作区根 (只读; `sandbox_dir` 的包含性证明基准)。
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// 当前 policy (只读快照; 默认 20k tokens / 15min / opt-in / 记忆隔离)。
    pub fn policy(&self) -> PlayPolicy {
        *self.ledger().policy()
    }

    /// 全部 session 快照 (含终态, 按插入顺序; 供自述/测试只读核对)。
    pub fn sessions(&self) -> Vec<PlaySession> {
        self.ledger().sessions().to_vec()
    }

    /// 尚未终结的 session 快照 (`Planned` / `Running`)。
    pub fn pending(&self) -> Vec<PlaySession> {
        self.ledger()
            .pending()
            .into_iter()
            .cloned()
            .collect::<Vec<_>>()
    }

    /// poison 容错 (账本可重建; 持锁线程 panic 不应让工具面连锁 panic)。
    fn ledger(&self) -> MutexGuard<'_, PlayLedger> {
        self.ledger
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 注入时钟 → `u64` 毫秒; 负值按 `0` 处理 (不 panic, 不倒流)。
    fn now_ms(&self) -> u64 {
        u64::try_from(self.clock.now().timestamp_millis()).unwrap_or(0)
    }

    fn propose(&self, context: &PlayContext) -> Option<PlayProposal> {
        self.ledger().propose(context)
    }

    fn start_session(
        &self,
        proposal: &PlayProposal,
        id: &str,
        user_opt_in: bool,
    ) -> Result<PlaySession, PlayError> {
        let now_ms = self.now_ms();
        let mut ledger = self.ledger();
        Ok(ledger
            .start(proposal, id.to_string(), now_ms, user_opt_in)?
            .clone())
    }

    fn finish_session(&self, id: &str, outcome: PlayOutcome) -> Result<PlaySession, PlayError> {
        let mut ledger = self.ledger();
        Ok(ledger.finish(id, outcome)?.clone())
    }

    /// 中止并报告「是否幂等重放」: 调用前已 `Aborted` = 重放 (不改变任何字段)。
    fn abort_session(&self, id: &str) -> Result<(PlaySession, bool), PlayError> {
        let mut ledger = self.ledger();
        let replay = ledger
            .sessions()
            .iter()
            .find(|session| session.id == id)
            .is_some_and(|session| session.status == PlayStatus::Aborted);
        let session = ledger.abort(id)?.clone();
        Ok((session, replay))
    }
}

/// 四件套装配结果: 共享账本 + 四个模块 (每个模块贡献一个能力)。
pub struct BioPlayWiring {
    state: Arc<BioPlayState>,
    modules: Vec<Arc<dyn CapabilityProvider>>,
}

impl BioPlayWiring {
    /// 共享账本状态 (测试/自述只读核对口)。
    pub fn state(&self) -> &Arc<BioPlayState> {
        &self.state
    }

    /// 四个模块 (顺序: propose / start / finish / abort)。
    pub fn modules(&self) -> &[Arc<dyn CapabilityProvider>] {
        &self.modules
    }

    /// 消费为模块列表 (装配根注册用)。
    pub fn into_modules(self) -> Vec<Arc<dyn CapabilityProvider>> {
        self.modules
    }
}

/// 装配门 (纯函数, 测试可注入): env 原始值 = `"1"` **且** 工作区根已注入时,
/// 构造四件套; 其余一律 `None` (不构造 = 不注册, 零变化)。
///
/// 缺工作区根 = 无法做 `sandbox_dir` 包含性证明 = **拒绝注册** (fail-closed,
/// 与 `tool.filesystem` 等既有「根注入才注册」的家族同口径), 不静默放行。
pub fn bio_play_wiring_from_env_value(
    raw: Option<&str>,
    workspace_root: Option<&Path>,
    clock: Arc<dyn Clock>,
) -> Option<BioPlayWiring> {
    if !bio_play_enabled_from_env_value(raw) {
        return None;
    }
    let root = workspace_root?;
    let state = Arc::new(BioPlayState::new(root, clock));
    let modules: Vec<Arc<dyn CapabilityProvider>> = vec![
        Arc::new(PlayProposeModule::new(Arc::clone(&state))),
        Arc::new(PlayStartModule::new(Arc::clone(&state))),
        Arc::new(PlayFinishModule::new(Arc::clone(&state))),
        Arc::new(PlayAbortModule::new(Arc::clone(&state))),
    ];
    Some(BioPlayWiring { state, modules })
}

/// 从进程 env 现值装配 (装配根调用; 同 [`bio_play_wiring_from_env_value`] 口径)。
pub fn bio_play_wiring_from_env(
    workspace_root: Option<&Path>,
    clock: Arc<dyn Clock>,
) -> Option<BioPlayWiring> {
    bio_play_wiring_from_env_value(
        std::env::var(BIO_PLAY_ENABLE_ENV).ok().as_deref(),
        workspace_root,
        clock,
    )
}

// ============================================
// sandbox_dir 路径纪律
// ============================================

/// 校验沙箱目录标识: **只接受工作区内的纯相对路径**。
///
/// 规则 (拒绝即帧, 一律 `Err(原因)`):
/// - 非空, 长度 ≤ [`MAX_SANDBOX_DIR_BYTES`];
/// - 拒绝绝对路径 / 盘符前缀 / 根目录 (`Component::Prefix` / `RootDir`);
/// - 拒绝 `..` 父目录穿越 (`Component::ParentDir`; `.` 允许并被规范化掉);
/// - 至少一个正常路径段 (不允许等于工作区根本身);
/// - 过既有前置守门 (`ToolGuardrail::verify_path_access`) 与**最深已存在祖先的
///   canonicalize 包含性证明** (拒符号链接逃逸; 目录不必存在 —— 它只是标识,
///   本函数**不创建**任何目录);
/// - 已存在的目标必须是目录 (不把文件当沙箱目录)。
///
/// 返回规范化后的相对路径 (`/` 分隔; 供输出帧回显)。
pub fn validate_play_sandbox_dir(workspace_root: &Path, raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("sandbox_dir must not be empty".to_string());
    }
    if trimmed.len() > MAX_SANDBOX_DIR_BYTES {
        return Err(format!(
            "sandbox_dir is too long ({} bytes; limit {MAX_SANDBOX_DIR_BYTES})",
            trimmed.len()
        ));
    }

    let mut parts: Vec<&str> = Vec::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(part) => {
                let part = part
                    .to_str()
                    .ok_or_else(|| "sandbox_dir must be valid UTF-8".to_string())?;
                parts.push(part);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(
                    "sandbox_dir must not contain '..' (parent traversal is refused)".to_string(),
                );
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(
                    "sandbox_dir must be a relative path inside the workspace root (absolute \
                     paths are refused)"
                        .to_string(),
                );
            }
        }
    }
    if parts.is_empty() {
        return Err("sandbox_dir must name a subdirectory inside the workspace root".to_string());
    }

    let canonical_root = std::fs::canonicalize(workspace_root).map_err(|error| {
        format!(
            "workspace root {} is not accessible: {error}",
            workspace_root.display()
        )
    })?;
    let relative = parts.iter().collect::<PathBuf>();
    let candidate = canonical_root.join(&relative);

    // 既有前置守门 (词法穿越 / 绝对敏感系统路径 / 绝对路径 root 包含检查)。
    apeireth_tools_canonical::ToolGuardrail::verify_path_access(&canonical_root, &candidate)
        .map_err(|error| format!("sandbox_dir refused by the pre-call guard: {error}"))?;

    // 现实层: 最深已存在祖先 canonicalize 后必须仍在根内 (拒 symlink 逃逸,
    // 且不要求 sandbox 目录本身存在)。
    let mut probe: &Path = candidate.as_path();
    let deepest = loop {
        match std::fs::canonicalize(probe) {
            Ok(resolved) => break resolved,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let parent = probe.parent().ok_or_else(|| {
                    format!(
                        "sandbox_dir {} cannot be resolved inside the workspace root",
                        candidate.display()
                    )
                })?;
                if !parent.starts_with(&canonical_root) {
                    return Err(format!(
                        "sandbox_dir {} resolves outside the workspace root",
                        candidate.display()
                    ));
                }
                probe = parent;
            }
            Err(error) => {
                return Err(format!(
                    "sandbox_dir {} cannot be resolved: {error}",
                    candidate.display()
                ));
            }
        }
    };
    if !deepest.starts_with(&canonical_root) {
        return Err(format!(
            "sandbox_dir {} resolves outside the workspace root (symlinked ancestors are \
             refused)",
            candidate.display()
        ));
    }
    if !deepest.is_dir() {
        return Err(format!(
            "sandbox_dir {} resolves to a non-directory; the sandbox is a directory identifier",
            candidate.display()
        ));
    }

    Ok(parts.join("/"))
}

// ============================================
// 工具面共用: 参数解析 / 帧 / 错误码
// ============================================

const MEMORY_ISOLATION_NOTE: &str =
    "Play never writes host memory: outputs stay inside PlayOutcome metadata strings and the \
     in-memory play ledger.";

fn call_arguments(call: &ToolCall) -> Result<Value, ToolResult> {
    let arguments = match &call.arguments {
        Value::Null => json!({}),
        other => other.clone(),
    };
    if !arguments.is_object() {
        return Err(ToolResult::permanent_error(
            &call.id,
            "play arguments must be a JSON object",
        ));
    }
    Ok(arguments)
}

fn parse_params<T>(call: &ToolCall, tool: &str) -> Result<T, ToolResult>
where
    T: serde::de::DeserializeOwned,
{
    let arguments = call_arguments(call)?;
    serde_json::from_value(arguments)
        .map_err(|error| refusal(call, tool, format!("invalid parameters: {error}")))
}

fn refusal(call: &ToolCall, tool: &str, message: impl Into<String>) -> ToolResult {
    let message = message.into();
    ToolResult::permanent_error(&call.id, format!("{tool}: {message}")).with_name(tool)
}

/// 内核错误 → 稳定机器码 (输出帧错误文本里的 `code=`; 测试与下游按码断言)。
pub fn play_error_code(error: &PlayError) -> &'static str {
    match error {
        PlayError::OptInRequired => "opt_in_required",
        PlayError::MemoryIsolationRequired => "memory_isolation_required",
        PlayError::DuplicateSession(_) => "duplicate_session",
        PlayError::SessionNotFound(_) => "session_not_found",
        PlayError::InvalidTransition { .. } => "invalid_transition",
        PlayError::BudgetExceeded { .. } => "budget_exceeded",
        PlayError::DurationExceeded { .. } => "duration_exceeded",
        PlayError::BudgetUsedExceedsReserved { .. } => "budget_used_exceeds_reserved",
    }
}

fn kernel_refusal(call: &ToolCall, tool: &str, error: PlayError) -> ToolResult {
    let code = play_error_code(&error);
    refusal(call, tool, format!("refused: {error} (code={code})"))
}

fn policy_json(policy: &PlayPolicy) -> Value {
    json!({
        "max_budget_tokens": policy.max_budget_tokens,
        "max_duration_ms": policy.max_duration_ms,
        "require_opt_in_by_default": policy.require_opt_in_by_default,
        "isolate_memory": policy.isolate_memory,
        "allowed_risk": serde_json::to_value(policy.allowed_risk).unwrap_or(Value::Null),
    })
}

fn session_json(session: &PlaySession) -> Value {
    serde_json::to_value(session).unwrap_or(Value::Null)
}

fn outcome_json(outcome: &PlayOutcome) -> Value {
    serde_json::to_value(outcome).unwrap_or(Value::Null)
}

fn validate_candidates(candidates: &[PlayTopic]) -> Result<(), String> {
    if candidates.is_empty() {
        return Err("candidates must contain at least one play topic".to_string());
    }
    if candidates.len() > MAX_PROPOSAL_CANDIDATES {
        return Err(format!(
            "too many candidates ({}; limit {MAX_PROPOSAL_CANDIDATES})",
            candidates.len()
        ));
    }
    for candidate in candidates {
        if candidate.topic.trim().is_empty() {
            return Err("candidate topic must not be empty".to_string());
        }
        if candidate.skills.len() > MAX_SKILLS_PER_TOPIC {
            return Err(format!(
                "candidate {:?} has too many skill targets ({}; limit {MAX_SKILLS_PER_TOPIC})",
                candidate.topic,
                candidate.skills.len()
            ));
        }
        if candidate.skills.iter().any(|skill| skill.trim().is_empty()) {
            return Err(format!(
                "candidate {:?} has an empty skill target",
                candidate.topic
            ));
        }
    }
    Ok(())
}

fn validate_outcome_items(items: &[String], field: &str) -> Result<(), String> {
    if items.len() > MAX_OUTCOME_ITEMS {
        return Err(format!(
            "{field} has too many entries ({}; limit {MAX_OUTCOME_ITEMS})",
            items.len()
        ));
    }
    for item in items {
        if item.trim().is_empty() {
            return Err(format!("{field} entries must not be empty"));
        }
        if item.len() > MAX_OUTCOME_ITEM_BYTES {
            return Err(format!(
                "{field} entry is too long ({} bytes; limit {MAX_OUTCOME_ITEM_BYTES})",
                item.len()
            ));
        }
    }
    Ok(())
}

// ============================================
// tool.play_propose
// ============================================

/// `tool.play_propose` 的参数面 (候选主题 + 可选可用额度)。
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayProposeParams {
    candidates: Vec<PlayTopic>,
    #[serde(default)]
    budget_available_tokens: Option<u64>,
    #[serde(default)]
    duration_available_ms: Option<u64>,
}

/// `tool.play_propose`: 只提案, 不启动, 不写任何面。
pub struct PlayProposeTool {
    id: CapabilityId,
    state: Arc<BioPlayState>,
}

impl PlayProposeTool {
    /// 绑定共享账本状态。
    pub fn new(state: Arc<BioPlayState>) -> Self {
        Self {
            id: CapabilityId::new(PLAY_PROPOSE_CAPABILITY_ID)
                .expect("static play_propose capability id is valid"),
            state,
        }
    }
}

#[async_trait]
impl ToolCapability for PlayProposeTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let params = json!({
            "type": "object",
            "properties": {
                "candidates": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": MAX_PROPOSAL_CANDIDATES,
                    "description": "Candidate practice topics, tried in order (deterministic first-fit). Each has topic, skills (skill targets), and risk (safe / sandboxed / needs_approval).",
                    "items": {
                        "type": "object",
                        "properties": {
                            "topic": { "type": "string" },
                            "skills": {
                                "type": "array",
                                "items": { "type": "string" }
                            },
                            "risk": {
                                "type": "string",
                                "enum": ["safe", "sandboxed", "needs_approval"]
                            }
                        },
                        "required": ["topic", "skills", "risk"],
                        "additionalProperties": false
                    }
                },
                "budget_available_tokens": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Tokens currently available to the caller. Omit to use the policy ceiling; the effective cap is min(policy, this value)."
                },
                "duration_available_ms": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Milliseconds currently available to the caller. Omit to use the policy ceiling; the effective cap is min(policy, this value)."
                }
            },
            "required": ["candidates"],
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap_or_default();
        NormalizedTool::new(PLAY_PROPOSE_TOOL_NAME)
            .with_description(
                "Draft one bounded practice proposal for this turn (play is controlled rehearsal, \
                 not free training). The proposal is filtered by policy: isolate_memory must be on, \
                 topic risk must be at or below allowed risk (default: sandboxed), and the estimated \
                 budget/duration must fit both the policy ceiling (default 20000 tokens / 15 min) \
                 and the caller's available headroom. This tool only proposes: it never starts a \
                 session, never touches the filesystem, and never writes memory. A candidate that \
                 does not fit is skipped, never shrunk into a fake proposal; when nothing fits the \
                 result is {proposal: null, reason}.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: PlayProposeParams = match parse_params(call, PLAY_PROPOSE_TOOL_NAME) {
            Ok(params) => params,
            Err(result) => return result,
        };
        if let Err(message) = validate_candidates(&params.candidates) {
            return refusal(call, PLAY_PROPOSE_TOOL_NAME, message);
        }

        let policy = self.state.policy();
        let context = PlayContext::new(
            params.candidates,
            params
                .budget_available_tokens
                .unwrap_or(policy.max_budget_tokens),
            params
                .duration_available_ms
                .unwrap_or(policy.max_duration_ms),
        );
        let proposal = self.state.propose(&context);
        let proposal_json = match &proposal {
            Some(proposal) => serde_json::to_value(proposal).unwrap_or(Value::Null),
            None => Value::Null,
        };
        let reason = match &proposal {
            Some(_) => String::new(),
            None => format!(
                "no candidate passed the play policy (isolate_memory={}, allowed_risk={}, \
                 budget_cap={} tokens, duration_cap={} ms, candidates={}); nothing was proposed \
                 and nothing was started",
                policy.isolate_memory,
                serde_json::to_value(policy.allowed_risk)
                    .unwrap_or(Value::Null)
                    .as_str()
                    .unwrap_or("unknown"),
                policy
                    .max_budget_tokens
                    .min(context.budget_available_tokens),
                policy.max_duration_ms.min(context.duration_available_ms),
                context.candidates.len()
            ),
        };
        ToolResult::ok(
            &call.id,
            json!({
                "proposal": proposal_json,
                "reason": reason,
                "policy": policy_json(&policy),
                "candidates_considered": context.candidates.len(),
                "started": false,
                "persisted": false,
                "memory_write": false,
            }),
        )
        .with_name(PLAY_PROPOSE_TOOL_NAME)
    }
}

// ============================================
// tool.play_start
// ============================================

/// `tool.play_start` 的参数面: 系统从 `play_propose` 返回的 proposal 对象可原样回传。
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayStartParams {
    session_id: String,
    sandbox_dir: String,
    #[serde(default)]
    user_opt_in: Option<bool>,
    proposal: PlayProposalParams,
}

/// start 入参里的提案关键字段 (与 propose 输出同形; 可原样回传)。
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayProposalParams {
    topic: String,
    skill_targets: Vec<String>,
    estimated_budget_tokens: u64,
    estimated_duration_ms: u64,
    #[serde(default)]
    needs_user_opt_in: Option<bool>,
    #[serde(default)]
    rationale: Option<String>,
}

impl PlayProposalParams {
    /// 保守缺省: `needs_user_opt_in` 缺省按 `true` (绝不假设免 opt-in)。
    fn into_proposal(self) -> PlayProposal {
        PlayProposal {
            topic: self.topic,
            skill_targets: self.skill_targets,
            estimated_budget_tokens: self.estimated_budget_tokens,
            estimated_duration_ms: self.estimated_duration_ms,
            needs_user_opt_in: self.needs_user_opt_in.unwrap_or(true),
            rationale: self.rationale.unwrap_or_default(),
        }
    }
}

/// `tool.play_start`: 显式 opt-in 后开一场有边界的练习。
pub struct PlayStartTool {
    id: CapabilityId,
    state: Arc<BioPlayState>,
}

impl PlayStartTool {
    /// 绑定共享账本状态。
    pub fn new(state: Arc<BioPlayState>) -> Self {
        Self {
            id: CapabilityId::new(PLAY_START_CAPABILITY_ID)
                .expect("static play_start capability id is valid"),
            state,
        }
    }
}

#[async_trait]
impl ToolCapability for PlayStartTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let params = json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Caller-chosen session id; must be unique inside the play ledger. Duplicates are refused."
                },
                "sandbox_dir": {
                    "type": "string",
                    "description": "Relative directory identifier for this practice run. Must stay inside the registered workspace root (absolute paths, '..' traversal, and symlinked ancestors outside the root are refused). This is a path-discipline label, not OS-level isolation; the directory is not created."
                },
                "user_opt_in": {
                    "type": "boolean",
                    "description": "Must be explicitly true. Missing or false is refused with code=opt_in_required; there is no IM approval card for play in this build."
                },
                "proposal": {
                    "type": "object",
                    "description": "The proposal object returned by play_propose, passed back verbatim (or an equivalent hand-written proposal; start re-validates opt-in, budget, duration, and unique id).",
                    "properties": {
                        "topic": { "type": "string" },
                        "skill_targets": {
                            "type": "array",
                            "items": { "type": "string" }
                        },
                        "estimated_budget_tokens": { "type": "integer", "minimum": 0 },
                        "estimated_duration_ms": { "type": "integer", "minimum": 0 },
                        "needs_user_opt_in": { "type": "boolean" },
                        "rationale": { "type": "string" }
                    },
                    "required": ["topic", "skill_targets", "estimated_budget_tokens", "estimated_duration_ms"],
                    "additionalProperties": false
                }
            },
            "required": ["session_id", "sandbox_dir", "user_opt_in", "proposal"],
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap_or_default();
        NormalizedTool::new(PLAY_START_TOOL_NAME)
            .with_description(
                "Start one bounded play session after an explicit human opt-in. Requires \
                 user_opt_in: true (missing or false is refused with code=opt_in_required — this \
                 build has no IM approval card, so the explicit boolean is the only gate) and a \
                 sandbox_dir that is a relative directory identifier inside the registered \
                 workspace root (absolute paths, '..' traversal, and symlinks escaping the root \
                 are refused; the directory is NOT created and this is NOT OS-level isolation). \
                 The proposal should come from play_propose: start re-validates opt-in, budget, \
                 duration, and unique session id, but the topic risk gate lives in propose. The \
                 session ledger is in-memory only: a process restart loses every session.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: PlayStartParams = match parse_params(call, PLAY_START_TOOL_NAME) {
            Ok(params) => params,
            Err(result) => return result,
        };
        if params.session_id.trim().is_empty() {
            return refusal(call, PLAY_START_TOOL_NAME, "session_id must not be empty");
        }
        if params.proposal.topic.trim().is_empty() {
            return refusal(
                call,
                PLAY_START_TOOL_NAME,
                "proposal.topic must not be empty",
            );
        }
        // 唯一强制门槛: 必须显式为 true (缺省 / false = 拒绝即帧, 无自动放行)。
        if params.user_opt_in != Some(true) {
            let code = play_error_code(&PlayError::OptInRequired);
            return refusal(
                call,
                PLAY_START_TOOL_NAME,
                format!(
                    "refused: {} (code={code}); pass user_opt_in=true only after a human explicitly \
                     approved this practice run",
                    PlayError::OptInRequired
                ),
            );
        }
        let sandbox_dir =
            match validate_play_sandbox_dir(&self.state.workspace_root(), &params.sandbox_dir) {
                Ok(sandbox_dir) => sandbox_dir,
                Err(message) => return refusal(call, PLAY_START_TOOL_NAME, message),
            };
        let proposal = params.proposal.into_proposal();
        let session = match self
            .state
            .start_session(&proposal, params.session_id.trim(), true)
        {
            Ok(session) => session,
            Err(error) => return kernel_refusal(call, PLAY_START_TOOL_NAME, error),
        };
        ToolResult::ok(
            &call.id,
            json!({
                "session": session_json(&session),
                "sandbox_dir": sandbox_dir,
                "user_opt_in": true,
                "persisted": false,
                "memory_write": false,
            }),
        )
        .with_name(PLAY_START_TOOL_NAME)
    }
}

// ============================================
// tool.play_finish
// ============================================

/// `tool.play_finish` 的参数面: 产出只接受元数据字符串。
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayFinishParams {
    session_id: String,
    #[serde(default)]
    learned: Vec<String>,
    #[serde(default)]
    artifacts: Vec<String>,
    #[serde(default)]
    budget_used: u64,
}

/// `tool.play_finish`: 结束练习, 产出只进 `PlayOutcome` (不写主体记忆)。
pub struct PlayFinishTool {
    id: CapabilityId,
    state: Arc<BioPlayState>,
}

impl PlayFinishTool {
    /// 绑定共享账本状态。
    pub fn new(state: Arc<BioPlayState>) -> Self {
        Self {
            id: CapabilityId::new(PLAY_FINISH_CAPABILITY_ID)
                .expect("static play_finish capability id is valid"),
            state,
        }
    }
}

#[async_trait]
impl ToolCapability for PlayFinishTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let params = json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Id of the running play session to finish."
                },
                "learned": {
                    "type": "array",
                    "maxItems": MAX_OUTCOME_ITEMS,
                    "items": { "type": "string" },
                    "description": "Skills actually practised (metadata strings). They stay inside PlayOutcome; nothing is written to host memory."
                },
                "artifacts": {
                    "type": "array",
                    "maxItems": MAX_OUTCOME_ITEMS,
                    "items": { "type": "string" },
                    "description": "Sandbox artifact identifiers (metadata strings only). This tool never writes files or memory."
                },
                "budget_used": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Tokens actually consumed. Must not exceed the budget reserved by the session; an overrun is refused and the session stays running."
                }
            },
            "required": ["session_id"],
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap_or_default();
        NormalizedTool::new(PLAY_FINISH_TOOL_NAME)
            .with_description(
                "Finish a running play session and return its PlayOutcome (learned skills, sandbox \
                 artifact identifiers, budget used). Artifacts are metadata strings only: this tool \
                 writes no files and no host memory, and the outcome exists only in the in-memory \
                 play ledger. budget_used may not exceed the budget reserved at start; an overrun is \
                 refused with code=budget_used_exceeds_reserved and leaves the session running.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: PlayFinishParams = match parse_params(call, PLAY_FINISH_TOOL_NAME) {
            Ok(params) => params,
            Err(result) => return result,
        };
        if params.session_id.trim().is_empty() {
            return refusal(call, PLAY_FINISH_TOOL_NAME, "session_id must not be empty");
        }
        if let Err(message) = validate_outcome_items(&params.learned, "learned") {
            return refusal(call, PLAY_FINISH_TOOL_NAME, message);
        }
        if let Err(message) = validate_outcome_items(&params.artifacts, "artifacts") {
            return refusal(call, PLAY_FINISH_TOOL_NAME, message);
        }
        let outcome = PlayOutcome {
            learned: params.learned,
            artifacts: params.artifacts,
            budget_used: params.budget_used,
        };
        let session = match self.state.finish_session(params.session_id.trim(), outcome) {
            Ok(session) => session,
            Err(error) => return kernel_refusal(call, PLAY_FINISH_TOOL_NAME, error),
        };
        ToolResult::ok(
            &call.id,
            json!({
                "session": session_json(&session),
                "outcome": session.outcome.as_ref().map(outcome_json).unwrap_or(Value::Null),
                "persisted": false,
                "memory_write": false,
                "note": MEMORY_ISOLATION_NOTE,
            }),
        )
        .with_name(PLAY_FINISH_TOOL_NAME)
    }
}

// ============================================
// tool.play_abort
// ============================================

/// `tool.play_abort` 的参数面。
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayAbortParams {
    session_id: String,
}

/// `tool.play_abort`: 中止练习 (对已中止 session 幂等)。
pub struct PlayAbortTool {
    id: CapabilityId,
    state: Arc<BioPlayState>,
}

impl PlayAbortTool {
    /// 绑定共享账本状态。
    pub fn new(state: Arc<BioPlayState>) -> Self {
        Self {
            id: CapabilityId::new(PLAY_ABORT_CAPABILITY_ID)
                .expect("static play_abort capability id is valid"),
            state,
        }
    }
}

#[async_trait]
impl ToolCapability for PlayAbortTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let params = json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Id of the running play session to abort. Aborting an already-aborted session is idempotent (reported as idempotent_replay=true and changes nothing)."
                }
            },
            "required": ["session_id"],
            "additionalProperties": false
        })
        .as_object()
        .cloned()
        .unwrap_or_default();
        NormalizedTool::new(PLAY_ABORT_TOOL_NAME)
            .with_description(
                "Abort a running play session. Aborting is idempotent: repeating the call on an \
                 already-aborted session reports idempotent_replay=true and changes no fields. An \
                 aborted session has no PlayOutcome and can never be finished or revived. This tool \
                 writes no files and no host memory; the only state is the in-memory play ledger.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let params: PlayAbortParams = match parse_params(call, PLAY_ABORT_TOOL_NAME) {
            Ok(params) => params,
            Err(result) => return result,
        };
        if params.session_id.trim().is_empty() {
            return refusal(call, PLAY_ABORT_TOOL_NAME, "session_id must not be empty");
        }
        match self.state.abort_session(params.session_id.trim()) {
            Ok((session, replay)) => ToolResult::ok(
                &call.id,
                json!({
                    "session": session_json(&session),
                    "aborted": true,
                    "idempotent_replay": replay,
                    "persisted": false,
                    "memory_write": false,
                }),
            )
            .with_name(PLAY_ABORT_TOOL_NAME),
            Err(error) => kernel_refusal(call, PLAY_ABORT_TOOL_NAME, error),
        }
    }
}

// ============================================
// 执行档审批钩子 (未在装配根注册, 见模块头 0 装边界)
// ============================================

/// 执行档审批理由 (审批卡载荷同文): 四件套都推进练习状态机, 无只读面。
pub fn play_approval_reason(capability: &str) -> String {
    format!(
        "play execution tier ({capability}): the four play tools advance the practice state \
         machine (propose -> opt-in -> start -> finish/abort). RequireApproval level, no auto-pass \
         tier. Explicit user_opt_in=true is the tool-surface gate for start; the sandbox directory \
         is a path-discipline label, not OS-level isolation, and play writes no files and no host \
         memory."
    )
}

/// 四件套的治理档钩子: 命中 [`PLAY_CAPABILITY_IDS`] 一律
/// [`Decision::RequireApproval`] (无自动放行档), 其它能力放行。
///
/// **0 装诚实**: 本钩子**未在装配根注册** —— 审批链由适配器接线, 本批无
/// gateway/CLI 手测, 也没有 IM opt-in 卡; 当前唯一强制门槛是 env 闸 +
/// 工具面显式 `user_opt_in=true`。
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayApprovalHook;

impl PlayApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for PlayApprovalHook {
    fn name(&self) -> &str {
        "play_execution_tier"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch { capability, .. } = &request.action else {
            return Decision::Allow;
        };
        if !PLAY_CAPABILITY_IDS.contains(&capability.as_str()) {
            return Decision::Allow;
        }
        Decision::require_approval(play_approval_reason(capability.as_str()))
    }
}

// ============================================
// 模块包装 (每个模块贡献一个能力, 共享同一本账本)
// ============================================

/// 提案面模块 (`module.tool.play_propose`)。
pub struct PlayProposeModule {
    tool: Arc<dyn ToolCapability>,
}

impl PlayProposeModule {
    /// 绑定共享账本状态。
    pub fn new(state: Arc<BioPlayState>) -> Self {
        Self {
            tool: Arc::new(PlayProposeTool::new(state)),
        }
    }

    /// 能力句柄 (测试/自述只读核对口)。
    pub fn tool(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for PlayProposeModule {
    fn id(&self) -> &str {
        "module.tool.play_propose"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// 开场模块 (`module.tool.play_start`)。
pub struct PlayStartModule {
    tool: Arc<dyn ToolCapability>,
}

impl PlayStartModule {
    /// 绑定共享账本状态。
    pub fn new(state: Arc<BioPlayState>) -> Self {
        Self {
            tool: Arc::new(PlayStartTool::new(state)),
        }
    }

    /// 能力句柄 (测试/自述只读核对口)。
    pub fn tool(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for PlayStartModule {
    fn id(&self) -> &str {
        "module.tool.play_start"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// 结束模块 (`module.tool.play_finish`)。
pub struct PlayFinishModule {
    tool: Arc<dyn ToolCapability>,
}

impl PlayFinishModule {
    /// 绑定共享账本状态。
    pub fn new(state: Arc<BioPlayState>) -> Self {
        Self {
            tool: Arc::new(PlayFinishTool::new(state)),
        }
    }

    /// 能力句柄 (测试/自述只读核对口)。
    pub fn tool(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for PlayFinishModule {
    fn id(&self) -> &str {
        "module.tool.play_finish"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// 中止模块 (`module.tool.play_abort`)。
pub struct PlayAbortModule {
    tool: Arc<dyn ToolCapability>,
}

impl PlayAbortModule {
    /// 绑定共享账本状态。
    pub fn new(state: Arc<BioPlayState>) -> Self {
        Self {
            tool: Arc::new(PlayAbortTool::new(state)),
        }
    }

    /// 能力句柄 (测试/自述只读核对口)。
    pub fn tool(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for PlayAbortModule {
    fn id(&self) -> &str {
        "module.tool.play_abort"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use apeireth_core::kernel::Timestamp;

    const NOW_MS: i64 = 1_700_000_000_123;

    fn clock() -> Arc<dyn Clock> {
        Arc::new(VirtualClock::new(
            Timestamp::from_epoch_millis(NOW_MS)
                .expect("valid fixed timestamp")
                .as_datetime(),
        ))
    }

    fn workspace() -> tempfile::TempDir {
        tempfile::tempdir().expect("create temp workspace")
    }

    fn topic(name: &str, skills: &[&str], risk: apeireth_organ::play::RiskLevel) -> PlayTopic {
        PlayTopic {
            topic: name.to_string(),
            skills: skills.iter().map(|skill| (*skill).to_string()).collect(),
            risk,
        }
    }

    #[test]
    fn the_env_switch_is_fail_closed_and_only_accepts_one() {
        assert_eq!(BIO_PLAY_ENABLE_ENV, "APEIRETH_ENABLE_BIO_PLAY");
        assert!(!bio_play_enabled_from_env_value(None), "未设置 = 关");
        for raw in ["", " ", "0", "true", "yes", "on", "2", "01", "1.0", " 0 "] {
            assert!(
                !bio_play_enabled_from_env_value(Some(raw)),
                "{raw:?} 必须关 (只认 \"1\")"
            );
        }
        assert!(bio_play_enabled_from_env_value(Some("1")));
        assert!(bio_play_enabled_from_env_value(Some(" 1 ")));
    }

    #[test]
    fn the_wiring_gate_needs_the_env_switch_and_a_workspace_root() {
        let dir = workspace();
        assert!(
            bio_play_wiring_from_env_value(Some("0"), Some(dir.path()), clock()).is_none(),
            "env 关 = 不构造"
        );
        assert!(
            bio_play_wiring_from_env_value(Some("1"), None, clock()).is_none(),
            "缺工作区根 = 无法证明 sandbox_dir = 拒绝注册 (fail-closed)"
        );
        let wiring = bio_play_wiring_from_env_value(Some("1"), Some(dir.path()), clock())
            .expect("env 开 + 根注入 = 四件套");
        assert_eq!(wiring.modules().len(), 4);
        let ids: Vec<String> = wiring
            .modules()
            .iter()
            .flat_map(|module| module.capabilities())
            .map(|capability| capability.id().as_str().to_string())
            .collect();
        assert_eq!(
            ids,
            PLAY_CAPABILITY_IDS
                .iter()
                .map(|id| (*id).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_sandbox_validator_accepts_relative_paths_inside_the_root() {
        let dir = workspace();
        assert_eq!(
            validate_play_sandbox_dir(dir.path(), "sandbox/play-1").expect("相对路径合法"),
            "sandbox/play-1"
        );
        assert_eq!(
            validate_play_sandbox_dir(dir.path(), "./sandbox").expect("./ 规范化"),
            "sandbox"
        );
        assert_eq!(
            validate_play_sandbox_dir(dir.path(), "  sandbox/play 2  ").expect("空白 trim"),
            "sandbox/play 2"
        );
    }

    #[test]
    fn the_sandbox_validator_refuses_escapes_and_non_directories() {
        let dir = workspace();
        let root = dir.path();
        for raw in ["", "   ", ".", "..", "../escape", "sandbox/../../escape"] {
            assert!(
                validate_play_sandbox_dir(root, raw).is_err(),
                "{raw:?} 必须拒绝"
            );
        }
        let absolute = root.join("sub");
        assert!(
            validate_play_sandbox_dir(root, &absolute.to_string_lossy()).is_err(),
            "绝对路径必须拒绝 (即使位于根内)"
        );
        // 已存在的普通文件不能被当作沙箱目录。
        std::fs::write(root.join("a.txt"), b"x").expect("write file");
        assert!(validate_play_sandbox_dir(root, "a.txt").is_err());
        // 文件之下的路径也不能当目录。
        assert!(validate_play_sandbox_dir(root, "a.txt/nested").is_err());
    }

    #[test]
    fn kernel_error_codes_are_stable() {
        assert_eq!(
            play_error_code(&PlayError::OptInRequired),
            "opt_in_required"
        );
        assert_eq!(
            play_error_code(&PlayError::DuplicateSession("s".into())),
            "duplicate_session"
        );
        assert_eq!(
            play_error_code(&PlayError::SessionNotFound("s".into())),
            "session_not_found"
        );
        assert_eq!(
            play_error_code(&PlayError::BudgetExceeded {
                requested: 1,
                cap: 0
            }),
            "budget_exceeded"
        );
        assert_eq!(
            play_error_code(&PlayError::DurationExceeded {
                requested: 1,
                cap: 0
            }),
            "duration_exceeded"
        );
        assert_eq!(
            play_error_code(&PlayError::BudgetUsedExceedsReserved {
                used: 2,
                reserved: 1
            }),
            "budget_used_exceeds_reserved"
        );
        assert_eq!(
            play_error_code(&PlayError::InvalidTransition {
                from: PlayStatus::Planned,
                to: PlayStatus::Running
            }),
            "invalid_transition"
        );
        assert_eq!(
            play_error_code(&PlayError::MemoryIsolationRequired),
            "memory_isolation_required"
        );
    }

    #[test]
    fn candidate_and_outcome_caps_refuse_unbounded_input() {
        let risk = apeireth_organ::play::RiskLevel::Safe;
        assert!(validate_candidates(&[]).is_err());
        assert!(validate_candidates(&[topic(" ", &["a"], risk)]).is_err());
        let too_many: Vec<PlayTopic> = (0..MAX_PROPOSAL_CANDIDATES + 1)
            .map(|index| topic(&format!("t{index}"), &["a"], risk))
            .collect();
        assert!(validate_candidates(&too_many).is_err());
        let many_skills: Vec<String> = (0..MAX_SKILLS_PER_TOPIC + 1)
            .map(|index| format!("s{index}"))
            .collect();
        assert!(validate_candidates(&[PlayTopic {
            topic: "t".into(),
            skills: many_skills,
            risk,
        }])
        .is_err());

        assert!(validate_outcome_items(&[], "learned").is_ok());
        assert!(validate_outcome_items(&["ok".into()], "learned").is_ok());
        assert!(validate_outcome_items(&["".into()], "learned").is_err());
        assert!(
            validate_outcome_items(&["x".repeat(MAX_OUTCOME_ITEM_BYTES + 1)], "learned").is_err()
        );
        let too_many = vec!["x".to_string(); MAX_OUTCOME_ITEMS + 1];
        assert!(validate_outcome_items(&too_many, "artifacts").is_err());
    }

    #[tokio::test]
    async fn the_approval_hook_stops_all_four_tools_and_keeps_other_capabilities_free() {
        fn dispatch(capability_id: &'static str) -> GovernanceRequest<'static> {
            let capability: &'static CapabilityId = Box::leak(Box::new(
                CapabilityId::new(capability_id).expect("static capability id is valid"),
            ));
            let arguments: &'static Value = Box::leak(Box::new(json!({})));
            GovernanceRequest::new(
                Action::CapabilityDispatch {
                    capability,
                    arguments,
                },
                apeireth_core::kernel::SessionId::new(),
                apeireth_core::kernel::TraceId::new(),
                1,
            )
        }

        let hook = PlayApprovalHook::new();
        for capability in PLAY_CAPABILITY_IDS {
            let decision = hook.evaluate(&dispatch(capability)).await;
            assert!(
                matches!(decision, Decision::RequireApproval { .. }),
                "{capability} 是执行档, 必须停人批"
            );
        }
        // 本钩子只管自己四个面; 其它能力一律放行。
        assert_eq!(
            hook.evaluate(&dispatch("tool.self_status")).await,
            Decision::Allow
        );
    }
}
