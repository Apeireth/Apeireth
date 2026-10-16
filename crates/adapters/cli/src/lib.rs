//! Canonical Apeireth command-line entry points.
//!
//! The CLI is a thin adapter: it bootstraps one canonical runtime, constructs
//! a canonical turn request, and delegates execution to
//! `Runtime::execute_outcome`. Pending approvals keep their `ApprovalId`.

// v2.0.0-rc.1 RC-9: KeyringSelector 真接 OS keyring / EncryptedFile backend
// (per `v2.0.0-rc-roadmap.md` §3 RC-9: "keyring 真正接到 EnvCredentialResolver 之前").
// 0 装诚实: 4 backend + KeyringSelector alpha 已真 impl; 本模块只做 bootstrap 集成.
pub mod gateway_panels;
pub mod keyring_bootstrap;
pub mod onboarding;
pub mod portable_bundle;
// W3-② 用户身份与角色: `apeireth user add|list|role|remove` 管理面 (owner 角色门)。
pub mod user_admin;

pub use portable_bundle::{PortableBundleManifest, PortableBundleSynthesizer};

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_core::kernel::{ApprovalId, CapabilityId, SessionId};
use apeireth_governance::TurnSecurityContext;
use apeireth_governance::{
    CredentialDisclosureHook, GovernancePipeline, Permission, PermissionGovernanceHook,
    PermissionPolicy, PromptInjectionHook,
};
use apeireth_guard::{
    ClassifierEnforcementMode, DatasetRecorder, IntentInput, IntentInterpreter,
    JointRiskClassifier, RuleIntentInterpreter,
};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, PendingApprovalView, Runtime, SessionStore, TurnOutcome,
    TurnRequest, TurnResponse,
};
use apeireth_runtime_assembly::SqliteSessionStore;
use apeireth_tools_canonical::{FetchConfig, TrustedShellConfig, APPLY_PATCH_CAPABILITY_ID};

/// One persistent SQLite database is shared by the cognitive backends.
/// `APEIRETH_COGNITIVE_DB` may override the path; Judge remains opt-in.
const COGNITIVE_DB_ENV: &str = "APEIRETH_COGNITIVE_DB";
const SESSION_DB_ENV: &str = "APEIRETH_SESSION_DB";
const COGNITIVE_JUDGE_ENV: &str = "APEIRETH_COGNITIVE_JUDGE";
const COGNITIVE_COUNCIL_ENV: &str = "APEIRETH_COGNITIVE_COUNCIL";
/// Enables the desensitized Guard dataset event stream when set to `1`.
pub const GUARD_DATASET_ENABLED_ENV: &str = "APEIRETH_GUARD_DATASET_ENABLED";
/// Optional JSONL path for the composition-owned Guard dataset recorder.
pub const GUARD_DATASET_PATH_ENV: &str = "APEIRETH_GUARD_DATASET_PATH";
/// Local Guard model mode: disabled, shadow, advisory, or enforce.
pub const GUARD_ML_MODE_ENV: &str = "APEIRETH_GUARD_ML_MODE";
/// Local JSON model artifact path. Paths are never returned in Guard status.
pub const GUARD_ML_MODEL_ENV: &str = "APEIRETH_GUARD_ML_MODEL";

/// Legacy opt-in for the local filesystem and search tools. Accepted for
/// compatibility; the tools are granted by default now.
pub const ENABLE_LOCAL_READ_TOOLS_ENV: &str = "APEIRETH_ENABLE_LOCAL_READ_TOOLS";
/// Privacy escape hatch: when set to `1`, the local filesystem and search
/// tools are NOT granted, even though they default to on.
pub const DISABLE_LOCAL_READ_TOOLS_ENV: &str = "APEIRETH_DISABLE_LOCAL_READ_TOOLS";
// 2026-09-08 用户旋钮 (per docs/01-architecture/forget-three-phase-production-spec 同批):
// shell/fetch 注册 + 策略 grant + require_approval (每次调用仍走人工审批);
// organs / preference_learning 为认知模块装配旋钮。
const ENABLE_SHELL_ENV: &str = "APEIRETH_ENABLE_SHELL";
const ENABLE_FETCH_ENV: &str = "APEIRETH_ENABLE_FETCH";
// 内容流接入旋钮 (`tool.content_feed`, J1): 注册 + 策略 grant + 审批档钩子。
// 默认关 (fail-closed); `APEIRETH_ENABLE_CONTENT_FEED=1` = 注册工具 + grant
// (「工具面有人拍板」) + [`apeireth_tools_canonical::ContentFeedApprovalHook`]
// 审批档 —— 拉取与入库都经审批档, 每次调用都人批 (无自动放行档)。
const ENABLE_CONTENT_FEED_ENV: &str = "APEIRETH_ENABLE_CONTENT_FEED";
// 受控文件写入旋钮 (`tool.apply_patch`, 第七件生产工具): 注册 + 策略 grant +
// 风险档位。默认关 (fail-closed); `APEIRETH_ENABLE_FILE_WRITE=1` = 注册工具 +
// grant + require_approval —— 每次写入都要人批 (本地审批面板 + IM 审批卡同链)。
// **文件写入总闸：apply_patch 与 shell 写命令同受此闸** (内测整改, 宪法级):
// 关 = shell 写意图 (重定向/删除类/写命令面) 拒绝即帧 (pipeline.pre_deny);
// 开 = shell 写命令行为不变 (照旧走审批链与既有风险映射)。
pub const ENABLE_FILE_WRITE_ENV: &str = "APEIRETH_ENABLE_FILE_WRITE";
// 「自动放行已读文件的修改」子档 (依赖主开关, 同 shellSandbox 嵌套依赖模式):
// `APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS=1` 时仅**修改类**补丁免逐次审批;
// 创建/删除永不自动放行 (仍进人工审批四态闭合 / IM 审批卡)。未读文件的修改
// 由读前观测门禁直接拒绝 (先读后写)。
pub const ENABLE_FILE_WRITE_AUTO_PASS_ENV: &str = "APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS";
// git 写分级旋钮 (`tool.git_write`, 干活能力轴 P1): 注册 + 策略 grant + 分级
// 风险档位。默认关 (fail-closed); `APEIRETH_ENABLE_GIT_WRITE=1` = 注册工具 +
// grant + 分级钩子 —— L1 可逆默认每次人批 (本地审批面板 + IM 审批卡同链),
// L2 出网**永远人批** (无自动放行档), L3 毁灭拒绝即帧 (无工具面)。
pub const ENABLE_GIT_WRITE_ENV: &str = "APEIRETH_ENABLE_GIT_WRITE";
// 「自动放行 L1」子档 (依赖主开关): `APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1=1`
// 时仅 **L1 可逆本地写** (add/commit/branch 建·切/tag 本地) 免逐次审批;
// **L2 出网永不自动放行** (push/fetch/remote 每次都人批)。
pub const ENABLE_GIT_WRITE_AUTO_PASS_L1_ENV: &str = "APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1";
// 干活能力轴 P2 两级工具旋钮 (worktree_plan 只读改单面 / worktree_apply 四拍
// 受控工作树写): 子依赖主 + file_write 唯一写总闸, fail-closed —— 计划拍要
// 写总闸同开, 落盘拍再要计划拍同开 (改单是 apply 的输入合同, 两级成对)。
// 默认关 (fail-closed); 开 = 注册 + 授权同源, apply 每次落盘都人批 (无自动
// 放行档), plan 只读零审批。
pub const ENABLE_WORKTREE_PLAN_ENV: &str = "APEIRETH_ENABLE_WORKTREE_PLAN";
pub const ENABLE_WORKTREE_APPLY_ENV: &str = "APEIRETH_ENABLE_WORKTREE_APPLY";
// 书→技能沉淀旋钮 (I 轴吸收自动化 I1): `tool.skill_distill` 是写文件面 ——
// 子依赖 file_write 唯一写总闸 (fail-closed, 单独设本旋钮而写总闸关 = 无效果),
// 注册 + grant + 审批档 (每次沉淀都人批, 无自动放行档); `tool.skill_search`
// 是只读档 (零审批、零写盘), 独立旋钮不依赖写总闸。
pub const ENABLE_SKILL_DISTILL_ENV: &str = "APEIRETH_ENABLE_SKILL_DISTILL";
pub const ENABLE_SKILL_SEARCH_ENV: &str = "APEIRETH_ENABLE_SKILL_SEARCH";
// MCP 外部工具桥旋钮 (轻默认, 默认关): `APEIRETH_ENABLE_MCP=1` 时装配外部工具桥,
// 服务器列表走 `APEIRETH_MCP_SERVERS` / 数据目录 `mcp-servers.json` (拒开语义)。
const ENABLE_MCP_ENV: &str = "APEIRETH_ENABLE_MCP";
const ENABLE_ORGANS_ENV: &str = "APEIRETH_ENABLE_ORGANS";
const ENABLE_PREFERENCE_LEARNING_ENV: &str = "APEIRETH_ENABLE_PREFERENCE_LEARNING";
// 2026-10-06 W2 接线批 (engineering-review-handoff-2026-10-06.md §5 W2):
// 记忆召回三组旋钮 —— proactive recall 补缺 (已接线无开关)、typed 写读对称
// (默认开 + 逃生门)、语义向量阶段 (真实现 + opt-in, 词法回退兜底)。
const ENABLE_PROACTIVE_RECALL_ENV: &str = "APEIRETH_ENABLE_PROACTIVE_RECALL";
const TYPED_RECALL_DISABLE_ENV: &str = "APEIRETH_DISABLE_TYPED_RECALL";
// 记忆核心族三旋钮 (PROACTIVE_RECALL / PREFERENCE_LEARNING / MEMORY_INJECTION):
// **默认开** (未设 = 开)。显式关走两条路, 与 typed_recall 逃生门同款语义:
// `APEIRETH_ENABLE_*=0` 或 `APEIRETH_DISABLE_*=1` (DISABLE 优先)。
// env 变量名不变 (向后兼容), 各配一个 DISABLE 逃生门。
const DISABLE_PROACTIVE_RECALL_ENV: &str = "APEIRETH_DISABLE_PROACTIVE_RECALL";
const DISABLE_PREFERENCE_LEARNING_ENV: &str = "APEIRETH_DISABLE_PREFERENCE_LEARNING";
const DISABLE_MEMORY_INJECTION_ENV: &str = "APEIRETH_DISABLE_MEMORY_INJECTION";
const PERSONA_ID_ENV: &str = "APEIRETH_PERSONA_ID";
const SUBJECT_ID_ENV: &str = "APEIRETH_SUBJECT_ID";
const DEFAULT_PERSONA_ID: &str = "apeireth";
const DEFAULT_SUBJECT_ID: &str = "local-user";
// 2026-10-06 W2 记忆闭环批: 反幻觉注入格式 / 每轮记忆整理 / 失败闭环。
const ENABLE_MEMORY_INJECTION_ENV: &str = "APEIRETH_ENABLE_MEMORY_INJECTION";
const ENABLE_CONSOLIDATION_ENV: &str = "APEIRETH_ENABLE_CONSOLIDATION";
const ENABLE_REFLEXION_ENV: &str = "APEIRETH_ENABLE_REFLEXION";
const REFLEXION_DIR_ENV: &str = "APEIRETH_REFLEXION_DIR";
const CONTEXT_BUDGET_CHARS_ENV: &str = "APEIRETH_CONTEXT_BUDGET_CHARS";
const TURN_ROUNDS_ENV: &str = "APEIRETH_MAX_TURN_ROUNDS";
const TOOL_CALLS_ENV: &str = "APEIRETH_MAX_TOOL_CALLS";
const QUOTA_MAX_TOKENS_ENV: &str = "APEIRETH_QUOTA_MAX_TOKENS";
const QUOTA_MAX_TOOL_STEPS_ENV: &str = "APEIRETH_QUOTA_MAX_TOOL_STEPS";
const QUOTA_MAX_COST_MICROS_ENV: &str = "APEIRETH_QUOTA_MAX_COST_MICROS";

/// Resolve the local read-tools switch from the process environment.
///
/// Semantics (fail-closed): `APEIRETH_DISABLE_LOCAL_READ_TOOLS=1` disables
/// the tools; the legacy `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` explicitly
/// enables them; with neither set (or any other value) they default to on.
/// When both are set to `1`, DISABLE wins.
fn local_read_tools_enabled_from_env() -> bool {
    if std::env::var(DISABLE_LOCAL_READ_TOOLS_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
    {
        return false;
    }
    if std::env::var(ENABLE_LOCAL_READ_TOOLS_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
    {
        return true;
    }
    true
}

/// 记忆核心族默认开旋钮的统一语义 (逃生门惯例与
/// `APEIRETH_DISABLE_LOCAL_READ_TOOLS` / `APEIRETH_DISABLE_TYPED_RECALL` 一致):
/// `APEIRETH_DISABLE_*=1` → 关 (最高优先); `APEIRETH_ENABLE_*=0` → 关;
/// 未设 / `=1` / 其余值 → 开。
fn core_memory_knob_enabled(enable_env: &str, disable_env: &str) -> bool {
    if std::env::var(disable_env)
        .ok()
        .is_some_and(|value| value.trim() == "1")
    {
        return false;
    }
    if std::env::var(enable_env)
        .ok()
        .is_some_and(|value| value.trim() == "0")
    {
        return false;
    }
    true
}

/// Proactive recall policy (记忆核心族, **默认开**): 未设或 `=1` → 确定性默认
/// 策略 (enabled, budget 2, 阈值 0.10); `APEIRETH_ENABLE_PROACTIVE_RECALL=0`
/// 或 `APEIRETH_DISABLE_PROACTIVE_RECALL=1` → `None` (DISABLE 优先)。
pub fn proactive_recall_policy_from_env() -> Option<apeireth_memory::ProactiveRecallPolicy> {
    core_memory_knob_enabled(ENABLE_PROACTIVE_RECALL_ENV, DISABLE_PROACTIVE_RECALL_ENV)
        .then(|| apeireth_memory::ProactiveRecallPolicy::default().enabled(true))
}

/// Typed (commitment/persona/relation) 召回读侧开关 (2026-10-06 W2 写读对称修复)。
///
/// **默认开**: 写侧 `CanonicalMemoryTypedSink` 早已在生产落库, 读侧此前恒缺
/// (数据入库永不召回) —— 开关默认开是对称修复而非新功能; `APEIRETH_DISABLE_TYPED_RECALL=1`
/// 为整体逃生门 (fail-closed 惯例)。
pub fn typed_recall_enabled_from_env() -> bool {
    !std::env::var(TYPED_RECALL_DISABLE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// Typed memory 的显式主体身份 (写侧 persona delta 与读侧 typed 召回共用)。
///
/// 本地产品单主体: 默认 `apeireth` / `local-user`, 多主体部署用
/// `APEIRETH_PERSONA_ID` / `APEIRETH_SUBJECT_ID` 覆写 (空白值视为未设)。
pub fn typed_recall_identity_from_env() -> apeireth_memory::TypedRecallIdentity {
    fn value(key: &str, default: &str) -> String {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| default.to_string())
    }
    apeireth_memory::TypedRecallIdentity {
        persona_id: value(PERSONA_ID_ENV, DEFAULT_PERSONA_ID),
        subject_id: value(SUBJECT_ID_ENV, DEFAULT_SUBJECT_ID),
    }
}

/// 语义向量阶段 (2026-10-06 W2): 召回的 embedding 候选打分接线。
///
/// Fail-closed 语义: URL 与 MODEL **双缺** → `Ok(None)` (词法回退, 行为不变);
/// **只设其一** → 报错 (半配是配置事故, 大声失败绝不静默); 双全 → 构造
/// OpenAI-compatible embeddings transport 注入组装根 (构造不发网络)。
/// `APEIRETH_EMBEDDING_KEY` 可选 (本地免鉴权端点不设)。
pub fn embedding_provider_from_env(
) -> Result<Option<Arc<dyn apeireth_memory::EmbeddingProvider>>, apeireth_memory::EmbeddingError> {
    use apeireth_provider::embeddings::{
        OpenAiCompatibleEmbeddingProvider, EMBEDDING_MODEL_ENV, EMBEDDING_URL_ENV,
    };
    let url = std::env::var(EMBEDDING_URL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    let model = std::env::var(EMBEDDING_MODEL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty());
    match (url, model) {
        (None, None) => Ok(None),
        (Some(_), Some(_)) => Ok(Some(Arc::new(OpenAiCompatibleEmbeddingProvider::from_env()?))),
        _ => Err(apeireth_memory::EmbeddingError::Unavailable(format!(
            "partial embedding config: set both {EMBEDDING_URL_ENV} and {EMBEDDING_MODEL_ENV} or neither"
        ))),
    }
}

/// 反幻觉注入格式 (记忆核心族, **默认开**): 开启时记忆 overlay 用编号
/// 证据清单 + 「禁止说『我记得我们以前聊过』」规则; `APEIRETH_ENABLE_MEMORY_INJECTION=0`
/// 或 `APEIRETH_DISABLE_MEMORY_INJECTION=1` 关回 XML 封闭世界格式 (DISABLE 优先)。
pub fn memory_injection_enabled_from_env() -> bool {
    core_memory_knob_enabled(ENABLE_MEMORY_INJECTION_ENV, DISABLE_MEMORY_INJECTION_ENV)
}

/// 偏好学习装配旋钮 (记忆核心族, **默认开**): 开启时 `cognitive.preference_learning`
/// 模块注册 (双索引写回 + 召回三段展开); `APEIRETH_ENABLE_PREFERENCE_LEARNING=0`
/// 或 `APEIRETH_DISABLE_PREFERENCE_LEARNING=1` = 关 (DISABLE 优先)。
pub fn preference_learning_enabled_from_env() -> bool {
    core_memory_knob_enabled(
        ENABLE_PREFERENCE_LEARNING_ENV,
        DISABLE_PREFERENCE_LEARNING_ENV,
    )
}

/// 每轮记忆整理 (2026-10-06 W2): `APEIRETH_ENABLE_CONSOLIDATION=1` 时 AfterTurn
/// 跑确定性 consolidation 报告并把提炼洞察落库 (稳定 ID 幂等); 默认关。
pub fn consolidation_enabled_from_env() -> bool {
    std::env::var(ENABLE_CONSOLIDATION_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// reflexion 失败闭环 (2026-10-06 W2): `APEIRETH_ENABLE_REFLEXION=1` 注册
/// `cognitive.reflexion` 模块 (TurnStart 教训注入 + AfterTurn 判定沉淀); 默认关。
pub fn reflexion_enabled_from_env() -> bool {
    std::env::var(ENABLE_REFLEXION_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// reflexion 存储根目录: `APEIRETH_REFLEXION_DIR` 覆写, 默认 `<data>/reflexion`。
pub fn reflexion_store_root_from_env() -> PathBuf {
    std::env::var(REFLEXION_DIR_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| default_panel_data_dir().join("reflexion"))
}

/// 上下文预算旋钮 (接线批): `APEIRETH_CONTEXT_BUDGET_CHARS=N` = 组装期注入上下文块
/// (记忆注入 / 器官产物 / 教训等) 的**总字符预算**。核心块 (身份 / 系统约定 /
/// 安全指令) 永不截断; 逐块上限先于总预算生效, 超出按长尾先砍贪心截断; 低于预算
/// 逐字节不变。未设 / 非法值 = 默认 `DEFAULT_CONTEXT_BUDGET_CHARS` (24000 字符
/// ≈ 6k token 量级, 推导见 runtime `DEFAULT_CONTEXT_BUDGET_CHARS`)。
pub fn context_budget_chars_from_env() -> usize {
    std::env::var(CONTEXT_BUDGET_CHARS_ENV)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|&chars| chars > 0)
        .unwrap_or(apeireth_runtime::DEFAULT_CONTEXT_BUDGET_CHARS)
}

/// 回合轮数预算旋钮 (预算显式化): `APEIRETH_MAX_TURN_ROUNDS=N` = 单回合逻辑
/// 轮数上限 (含模块重试占位)。越界值钳制到 `MIN_TURN_ROUNDS..=MAX_TURN_ROUNDS`,
/// 未设 / 空 / 非法值 = 默认 `DEFAULT_MAX_ROUNDS` (8)。
pub fn turn_round_limit_from_env() -> u32 {
    apeireth_runtime::canonical::parse_turn_round_limit(
        std::env::var(TURN_ROUNDS_ENV).ok().as_deref(),
    )
}

/// 单轮工具调用预算旋钮 (预算显式化): `APEIRETH_MAX_TOOL_CALLS=N` = 单轮最多
/// 派发多少个 tool call (超出部分截断并补合成结果)。越界值钳制到
/// `MIN_TOOL_CALL_LIMIT..=MAX_TOOL_CALL_LIMIT`, 未设 / 空 / 非法值 = 默认 16。
pub fn tool_call_limit_from_env() -> usize {
    apeireth_runtime::canonical::parse_tool_call_limit(
        std::env::var(TOOL_CALLS_ENV).ok().as_deref(),
    )
}

/// 会话配额旋钮 (opt-in 烧钱闸): `APEIRETH_QUOTA_MAX_TOKENS=N` = 会话级 token
/// 配额上限 (回合计量 usage 累加到上限即预算耗尽)。正整数直通 (越界钳制到
/// `MAX_QUOTA_TOKENS`); 未设 / 空 / 0 / 非法 = 不设限 (`None`) —— 配额是
/// opt-in, 默认不设限的语义显式。
pub fn quota_max_tokens_from_env() -> Option<u64> {
    apeireth_runtime::canonical::parse_quota_max_tokens(
        std::env::var(QUOTA_MAX_TOKENS_ENV).ok().as_deref(),
    )
}

/// 会话配额旋钮 (opt-in 烧钱闸): `APEIRETH_QUOTA_MAX_TOOL_STEPS=N` = 会话级
/// 工具步数上限 (工具调用计数到上限即预算耗尽)。取值纪律同
/// [`quota_max_tokens_from_env`]。
pub fn quota_max_tool_steps_from_env() -> Option<u64> {
    apeireth_runtime::canonical::parse_quota_max_tool_steps(
        std::env::var(QUOTA_MAX_TOOL_STEPS_ENV).ok().as_deref(),
    )
}

/// 会话配额旋钮 (opt-in 烧钱闸): `APEIRETH_QUOTA_MAX_COST_MICROS=N` = 会话级
/// 花费上限 (微单位; 有价轮按 `usage × 单价` 换算累加到上限即预算耗尽)。
/// 取值纪律同 [`quota_max_tokens_from_env`]: 正整数直通 (越界钳制到
/// `MAX_QUOTA_COST_MICROS`), 未设 / 空 / 0 / 非法 = 不设限 (`None`)。
/// 无价轮 (无单价行) 不计入花费 (绝不编价), 该上限只对有价部分生效。
pub fn quota_max_cost_micros_from_env() -> Option<u64> {
    apeireth_runtime::canonical::parse_quota_max_cost_micros(
        std::env::var(QUOTA_MAX_COST_MICROS_ENV).ok().as_deref(),
    )
}

/// 生效模型 id (与运行时 `with_default_model` 同一解析口径):
/// `APEIRETH_MODEL` 显式覆盖 > minimax 族首个默认模型 (提供商面规范 id,
/// 小写)。无模型在力回 `None` (自述模型节显式 null + 原因, 不编造)。
pub fn active_model_id() -> Option<String> {
    if let Some(model) = std::env::var("APEIRETH_MODEL")
        .ok()
        .filter(|model| !model.trim().is_empty())
    {
        return Some(model);
    }
    apeireth_provider::canonical_minimax::MinimaxProviderPlugin::from_env()
        .ok()
        .and_then(|plugin| plugin.model_ids().first().cloned())
}

/// 模型画像表 (H-P1 配置面): 内置基线 ← 数据目录 `model-profiles.json`
/// (StoredDoc 拒开) ← env `APEIRETH_MODEL_PROFILES` (JSON 覆盖), 逐模型整行
/// 覆盖。坏配置拒开: 走启动错误正式通道, 不静默回退。
fn model_profile_table_from_env() -> Result<apeireth_core::model_profile::ModelProfileTable, String>
{
    apeireth_core::model_profile::ModelProfileTable::load(Some(&default_panel_data_dir()))
        .map_err(|error| format!("model profile configuration refused to load: {error}"))
}

/// Build the production governance policy from an explicit local-read choice.
///
/// The explicit boolean keeps the policy deterministic and easy to test. The
/// environment-facing wrapper is [`build_production_governance_from_env`].
/// Authorization is deliberately the first hook: a later content-risk hook
/// must never turn an unauthorized capability into an approval request.
pub fn build_production_governance(enable_local_read_tools: bool) -> GovernancePipeline {
    build_production_governance_parts(enable_local_read_tools).0
}

/// 文件写入 (`tool.apply_patch`) 的治理风险档位 (件一风险映射)。
///
/// 三档与设置开关一一对应; 未授权的能力永不因内容风险被抬进审批
/// (授权仍是第一钩子)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileWriteRiskLevel {
    /// 未开主开关: 不 grant (写入工具不在授权面)。
    Disabled,
    /// 默认档: grant + require_approval 标记 —— **每次写入**都停在人工审批
    /// (本地审批面板 + IM 审批卡同链)。
    RequireApprovalEveryWrite,
    /// 自动放行档: grant + 写入风险钩子 —— 修改类补丁放行; 创建/删除
    /// **永不自动放行**, 停在人工审批 (同一审批链)。
    AutoPassReadFileEdits,
}

/// 从环境变量解析写入风险档位 (子档依赖主开关, fail-closed)。
pub fn file_write_risk_level_from_env() -> FileWriteRiskLevel {
    if !file_write_enabled_from_env() {
        return FileWriteRiskLevel::Disabled;
    }
    if file_write_auto_pass_enabled_from_env() {
        FileWriteRiskLevel::AutoPassReadFileEdits
    } else {
        FileWriteRiskLevel::RequireApprovalEveryWrite
    }
}

/// 受控文件写入主开关: `APEIRETH_ENABLE_FILE_WRITE=1` (默认关, fail-closed)。
/// **文件写入总闸：apply_patch 与 shell 写命令同受此闸** —— 唯一写总闸,
/// 开关名与权力相符 (内测整改)。
pub fn file_write_enabled_from_env() -> bool {
    std::env::var(ENABLE_FILE_WRITE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 「自动放行已读文件的修改」子档: 主开关开且 `APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS=1`
/// 才生效 (依赖主开关; 单独设子档 = 无效果)。
pub fn file_write_auto_pass_enabled_from_env() -> bool {
    file_write_enabled_from_env()
        && std::env::var(ENABLE_FILE_WRITE_AUTO_PASS_ENV)
            .ok()
            .is_some_and(|value| value.trim() == "1")
}

/// git 写分级 (`tool.git_write`) 的治理风险档位 (D-P1 分级风险映射)。
///
/// 三档与设置开关一一对应; 未开主开关不 grant (工具不在授权面, fail-closed)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitWriteRiskLevel {
    /// 未开主开关: 不 grant (git 写工具不在授权面)。
    Disabled,
    /// 默认档: grant + 分级钩子 —— L1 / L2 都停人工审批 (L2 永远如此)。
    RequireApprovalEveryWrite,
    /// 「自动放行 L1」档: grant + 分级钩子 —— 仅 L1 可逆本地写放行;
    /// L2 出网照旧永远人工审批 (同一审批链, IM 审批卡可批 push)。
    AutoPassReversibleL1,
}

/// 从环境变量解析 git 写分级档位 (子档依赖主开关, fail-closed)。
pub fn git_write_risk_level_from_env() -> GitWriteRiskLevel {
    if !git_write_enabled_from_env() {
        return GitWriteRiskLevel::Disabled;
    }
    if git_write_auto_pass_l1_enabled_from_env() {
        GitWriteRiskLevel::AutoPassReversibleL1
    } else {
        GitWriteRiskLevel::RequireApprovalEveryWrite
    }
}

/// git 写主开关: `APEIRETH_ENABLE_GIT_WRITE=1` (默认关, fail-closed)。
pub fn git_write_enabled_from_env() -> bool {
    std::env::var(ENABLE_GIT_WRITE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 「自动放行 L1」子档: 主开关开且 `APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1=1`
/// 才生效 (依赖主开关; 单独设子档 = 无效果)。
pub fn git_write_auto_pass_l1_enabled_from_env() -> bool {
    git_write_enabled_from_env()
        && std::env::var(ENABLE_GIT_WRITE_AUTO_PASS_L1_ENV)
            .ok()
            .is_some_and(|value| value.trim() == "1")
}

/// worktree 计划拍主开关 (`tool.worktree_plan`): `APEIRETH_ENABLE_WORKTREE_PLAN=1`
/// 且写总闸 `APEIRETH_ENABLE_FILE_WRITE=1` 才生效 (子依赖主 + file_write,
/// fail-closed —— 单独设本旋钮而写总闸关 = 无效果)。
pub fn worktree_plan_enabled_from_env() -> bool {
    file_write_enabled_from_env()
        && std::env::var(ENABLE_WORKTREE_PLAN_ENV)
            .ok()
            .is_some_and(|value| value.trim() == "1")
}

/// worktree 落盘拍 (`tool.worktree_apply`) 旋钮: 依赖计划拍主开关 + 写总闸
/// (子依赖主 + file_write, fail-closed —— 单独设本旋钮 = 无效果; 改单是
/// apply 的输入合同, 两级成对启用)。
pub fn worktree_apply_enabled_from_env() -> bool {
    worktree_plan_enabled_from_env()
        && std::env::var(ENABLE_WORKTREE_APPLY_ENV)
            .ok()
            .is_some_and(|value| value.trim() == "1")
}

/// 技能沉淀 (`tool.skill_distill`) 旋钮: 依赖写总闸 `APEIRETH_ENABLE_FILE_WRITE=1`
/// (file_write 总闸族, fail-closed —— 单独设本旋钮而写总闸关 = 无效果)。
/// 开 = 注册 + grant + 审批档 (每次沉淀都人批, 无自动放行档)。
pub fn skill_distill_enabled_from_env() -> bool {
    file_write_enabled_from_env()
        && std::env::var(ENABLE_SKILL_DISTILL_ENV)
            .ok()
            .is_some_and(|value| value.trim() == "1")
}

/// 技能检索 (`tool.skill_search`) 旋钮: 只读档零审批, 独立于写总闸。
pub fn skill_search_enabled_from_env() -> bool {
    std::env::var(ENABLE_SKILL_SEARCH_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 内容流接入 (`tool.content_feed`, J1) 旋钮: 独立开关, 默认关 (fail-closed)。
/// 开 = 注册 + grant + 审批档钩子 (拉取与入库都经审批档, 每次调用都人批)。
pub fn content_feed_enabled_from_env() -> bool {
    std::env::var(ENABLE_CONTENT_FEED_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

use apeireth_guard::BehaviorChainGuardHook;

/// Build the production pipeline and return the shared policy handle alongside
/// it, so the gateway can serve grants listing and session-scoped hot revoke
/// against the same policy the live hooks evaluate.
pub fn build_production_governance_parts(
    enable_local_read_tools: bool,
) -> (
    GovernancePipeline,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
) {
    build_production_governance_parts_with_dataset(
        enable_local_read_tools,
        None,
        FileWriteRiskLevel::Disabled,
        GitWriteRiskLevel::Disabled,
        false,
        false,
        // 显式布尔构建器不含技能旋钮: 技能两级工具默认不进授权面 (fail-closed),
        // 需要时走 [`build_production_governance_parts_from_env`] 的同源旋钮。
        false,
        false,
        // 同口径: 显式布尔构建器不含内容流旋钮 (默认不进授权面, fail-closed)。
        false,
    )
}

fn build_production_governance_parts_with_dataset(
    enable_local_read_tools: bool,
    dataset: Option<Arc<DatasetRecorder>>,
    file_write: FileWriteRiskLevel,
    git_write: GitWriteRiskLevel,
    worktree_plan: bool,
    worktree_apply: bool,
    skill_search: bool,
    skill_distill: bool,
    content_feed: bool,
) -> (
    GovernancePipeline,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
) {
    let mut policy = PermissionPolicy::new();
    policy.grant(Permission::ExecuteTool("tool.repo".to_string()));
    // 自省通道: `tool.self_status` 只读档 —— 默认可用、零审批 (无
    // require_approval)。模型可实测自身状态再发言, 不靠翻文件撞见自己。
    policy.grant(Permission::ExecuteTool("tool.self_status".to_string()));
    if enable_local_read_tools {
        policy.grant(Permission::ExecuteTool("tool.filesystem".to_string()));
        policy.grant(Permission::ExecuteTool("tool.search".to_string()));
    }
    // 件一风险映射: 受控文件写入 (`tool.apply_patch`) 的三档授权。
    // 默认档 grant + require_approval 标记 = 每次写入都停在人工审批 (本地
    // 审批面板 + IM 审批卡同链); 自动放行档只 grant, 放行规则交给写入风险
    // 钩子 (修改类补丁放行, 创建/删除永不自动放行)。Disabled = 不 grant。
    match file_write {
        FileWriteRiskLevel::Disabled => {}
        FileWriteRiskLevel::RequireApprovalEveryWrite => {
            policy.grant(Permission::ExecuteTool(
                APPLY_PATCH_CAPABILITY_ID.to_string(),
            ));
            policy.require_approval_for(APPLY_PATCH_CAPABILITY_ID);
        }
        FileWriteRiskLevel::AutoPassReadFileEdits => {
            policy.grant(Permission::ExecuteTool(
                APPLY_PATCH_CAPABILITY_ID.to_string(),
            ));
        }
    }
    // D-P1 git 写分级 (`tool.git_write`): 授权面只 grant「工具面有人拍板」,
    // **分级风险档位由分级钩子唯一承载** (默认档 = 钩子内 L1/L2 全停人工审批,
    // 结构上不会漏批); L2 出网永远人工审批 (无自动放行档), L3 毁灭拒绝即帧
    // (无工具面)。
    match git_write {
        GitWriteRiskLevel::Disabled => {}
        GitWriteRiskLevel::RequireApprovalEveryWrite | GitWriteRiskLevel::AutoPassReversibleL1 => {
            policy.grant(Permission::ExecuteTool(
                apeireth_tools_canonical::GIT_WRITE_CAPABILITY_ID.to_string(),
            ));
        }
    }
    // 干活能力轴 P2 两级工具 (授权同源, 与 shell/fetch 同款整改口径: 开关开 =
    // 注册 + 授权): `tool.worktree_plan` 只读改单面零审批 (计划不落盘);
    // `tool.worktree_apply` 只 grant, 审批级由 [`WorktreeApplyApprovalHook`]
    // 唯一承载 —— 每次落盘都人批 (无自动放行档), 理由载荷 = 改单摘要。
    if worktree_plan {
        policy.grant(Permission::ExecuteTool(
            apeireth_tools_canonical::WORKTREE_PLAN_CAPABILITY_ID.to_string(),
        ));
    }
    if worktree_apply {
        policy.grant(Permission::ExecuteTool(
            apeireth_tools_canonical::WORKTREE_APPLY_CAPABILITY_ID.to_string(),
        ));
    }
    // 书→技能沉淀 (I1) 授权面 (开关开 = 注册 + 授权, 同源整改口径):
    // `tool.skill_search` 只读档零审批 (grant 即全权, 检索零写盘);
    // `tool.skill_distill` 只 grant, 审批档由 [`SkillDistillApprovalHook`]
    // 唯一承载 —— 每次沉淀都人批 (无自动放行档), 理由载荷 = 技能卡摘要。
    if skill_search {
        policy.grant(Permission::ExecuteTool(
            apeireth_tools_canonical::SKILL_SEARCH_CAPABILITY_ID.to_string(),
        ));
    }
    if skill_distill {
        policy.grant(Permission::ExecuteTool(
            apeireth_tools_canonical::SKILL_DISTILL_CAPABILITY_ID.to_string(),
        ));
    }
    // 内容流接入 (`tool.content_feed`, J1) 授权面 (开关开 = 注册 + 授权, 同源
    // 整改口径): 只 grant「工具面有人拍板」, 审批档由 [`ContentFeedApprovalHook`]
    // 唯一承载 —— 拉取与入库都经审批档, 每次调用都人批 (无自动放行档); 改订阅
    // 走 write_intent 同族 (改订阅 = 写文件) 同档人批。
    if content_feed {
        policy.grant(Permission::ExecuteTool(
            apeireth_tools_canonical::CONTENT_FEED_CAPABILITY_ID.to_string(),
        ));
    }
    let policy = Arc::new(std::sync::Mutex::new(policy));
    let mut guard = BehaviorChainGuardHook::new();
    if let Some(dataset) = dataset {
        guard = guard.with_dataset_recorder(dataset);
    }
    guard = configure_guard_classifier(guard);
    let guard_hook = Arc::new(guard);

    let mut pipeline = GovernancePipeline::new().with(Arc::new(
        PermissionGovernanceHook::new_shared(policy.clone()),
    ));
    // 自动放行档的放行规则 (授权之后、内容风险之前): 修改类补丁放行;
    // 创建/删除停在 Decision::RequireApproval —— 与默认档同一条人工审批链
    // (审批四态闭合 / IM 审批卡), 「删除/新建永不自动放行」单调生效。
    if matches!(file_write, FileWriteRiskLevel::AutoPassReadFileEdits) {
        pipeline = pipeline.with(Arc::new(
            apeireth_tools_canonical::ApplyPatchWriteApprovalHook::new(),
        ));
    }
    // git 写分级风险映射 (授权之后、内容风险之前): L1 按档位放行或审批,
    // L2 永远审批 (理由带分级标注, 进本地面板 / IM 审批卡), L3/词表外拒绝即帧。
    if !matches!(git_write, GitWriteRiskLevel::Disabled) {
        pipeline = pipeline.with(Arc::new(
            apeireth_tools_canonical::GitWriteTierApprovalHook::new(match git_write {
                GitWriteRiskLevel::AutoPassReversibleL1 => {
                    apeireth_tools_canonical::GitWriteReleaseTier::AutoPassReversibleL1
                }
                _ => apeireth_tools_canonical::GitWriteReleaseTier::RequireApprovalEveryWrite,
            }),
        ));
    }
    // worktree 落盘拍审批级映射 (授权之后、内容风险之前, 同一审批链): 每次
    // 落盘都停人工审批 (RequireApproval 级, 无自动放行档), 理由载荷 = 改单
    // 摘要; 计划拍校验不过的调用拒绝即帧 (不进审批)。
    if worktree_apply {
        pipeline = pipeline.with(Arc::new(
            apeireth_tools_canonical::WorktreeApplyApprovalHook::new(
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            ),
        ));
    }
    // 书→技能沉淀审批档映射 (授权之后、内容风险之前, 同一审批链): 每次沉淀
    // 都停人工审批 (RequireApproval 级, 无自动放行档), 理由载荷 = 技能卡摘要;
    // 坏卡 / 来源越界拒绝即帧 (错误即帧, 不进审批)。
    if skill_distill {
        pipeline = pipeline.with(Arc::new(
            apeireth_tools_canonical::SkillDistillApprovalHook::new(),
        ));
    }
    // 内容流审批档映射 (授权之后、内容风险之前, 同一审批链): 每次调用 (拉取
    // 与入库 / 改订阅) 都停人工审批 (RequireApproval 级, 无自动放行档), 理由
    // 载荷按写面如实标注; 敏感面 URL 是拒不是批 (工具层已拒)。
    if content_feed {
        pipeline = pipeline.with(Arc::new(
            apeireth_tools_canonical::ContentFeedApprovalHook::new(),
        ));
    }
    let mut pipeline = pipeline
        .with(Arc::new(CredentialDisclosureHook::new()))
        .with(Arc::new(PromptInjectionHook::new()))
        .with(guard_hook.clone());
    // W3 三洋葱 L3-L5 物理执行面 (2026-10-10, 默认关): 末层纵深防御 ——
    // 授权已放行的动作再过一次双洋葱权威判定 (只收紧, 不把未授权变审批)。
    if onion_layer_enabled_from_env() {
        pipeline = pipeline.with(Arc::new(apeireth_runtime_assembly::OnionLayerHook::new(
            apeireth_core::onion_gate::standard_double_onion_gate(),
        )));
    }
    (pipeline, policy, guard_hook)
}

/// Build the production governance policy using the process environment.
///
/// The local read tools (`tool.filesystem`/`tool.search`) are granted by
/// default, matching `tool.repo`. `APEIRETH_DISABLE_LOCAL_READ_TOOLS=1`
/// disables them (privacy escape hatch) and wins over the legacy
/// `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` opt-in. Shell, fetch, and unknown
/// capabilities remain denied even if a future plugin registers them.
pub fn build_production_governance_from_env() -> GovernancePipeline {
    build_production_governance_parts_from_env().0
}

/// **W3 洋葱层旋钮** (2026-10-10, 默认关): `APEIRETH_ENABLE_ONION_LAYER=1` ——
/// 生产治理管线末层追加双洋葱权威判定 (HA 离线物拒 / L5 E 层兜底)。
fn onion_layer_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_ONION_LAYER")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

fn configure_guard_classifier(mut guard: BehaviorChainGuardHook) -> BehaviorChainGuardHook {
    let mode = std::env::var(GUARD_ML_MODE_ENV)
        .ok()
        .and_then(|value| ClassifierEnforcementMode::parse(&value));
    let Some(mode) = mode else {
        return guard;
    };
    let Some(path) = std::env::var(GUARD_ML_MODEL_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return guard;
    };
    if let Ok(classifier) = JointRiskClassifier::from_path(path).map(|model| model.with_mode(mode))
    {
        guard = guard.with_classifier(Arc::new(classifier));
    }
    guard
}

fn production_guard_dataset_recorder() -> Option<Arc<DatasetRecorder>> {
    let enabled = std::env::var(GUARD_DATASET_ENABLED_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    if !enabled {
        return None;
    }
    let path = std::env::var(GUARD_DATASET_PATH_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| ".apeireth/guard-dataset-v3.jsonl".to_string());
    let recorder = Arc::new(DatasetRecorder::new(path));
    recorder.set_enabled(true);
    Some(recorder)
}

fn build_production_governance_parts_from_env() -> (
    GovernancePipeline,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
) {
    let enable_local_read_tools = local_read_tools_enabled_from_env();
    let (pipeline, policy, guard_hook) = build_production_governance_parts_with_dataset(
        enable_local_read_tools,
        production_guard_dataset_recorder(),
        file_write_risk_level_from_env(),
        git_write_risk_level_from_env(),
        worktree_plan_enabled_from_env(),
        worktree_apply_enabled_from_env(),
        skill_search_enabled_from_env(),
        skill_distill_enabled_from_env(),
        content_feed_enabled_from_env(),
    );

    // 2026-09-08: shell/fetch 用户旋钮 = 注册 + 策略 grant + require_approval.
    // 语义 (fail-closed): 主人 env 显式授权"可以提议执行", 但**每次调用仍走
    // 人工审批流** (与既有 approval 生命周期一致; 无审批者时拒绝)。
    let enable_shell = std::env::var(ENABLE_SHELL_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let enable_fetch = std::env::var(ENABLE_FETCH_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    if enable_shell || enable_fetch {
        let mut guard = policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)");
        if enable_shell {
            guard.grant(Permission::ExecuteTool("tool.shell".to_string()));
            guard.require_approval_for("tool.shell");
        }
        if enable_fetch {
            guard.grant(Permission::ExecuteTool("tool.fetch".to_string()));
            guard.require_approval_for("tool.fetch");
        }
    }
    // education Dx-Check 工具 (`tool.education`, 纯确定性自查, 0 副作用):
    // **开关开 = 注册 + 授权同源** (开关名与权力相符, 与 shell/fetch 同款
    // 整改口径) —— 同一开关值 (`education_enabled_from_env`) 同时决定装配侧
    // 注册与本授权面, 不再出现"注册了但权力没到手"。只读计算档不设
    // require_approval (与 tool.repo / tool.self_status 同档); 会话权限预设
    // (read_only 等) 的读能力白名单语义不受影响。
    if education_enabled_from_env() {
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool("tool.education".to_string()));
    }
    (pipeline, policy, guard_hook)
}

/// Build the one canonical runtime used by CLI chat and the HTTP gateway.
///
/// Provider implementations are injected as plugins. Credentials are resolved
/// at execution time, so neither the runtime nor a provider stores API keys.
pub async fn build_canonical_runtime_from_env() -> Result<Runtime, String> {
    let (runtime, _, _, _, _) = build_canonical_runtime_with_sessions_from_env().await?;
    Ok(runtime)
}

/// Build the runtime and return the shared session-store, memory-backend and
/// permission-policy handles alongside it, so the gateway can serve the
/// sessions/memory/permissions introspection surfaces from the same durable
/// stores and the same live policy.
pub async fn build_canonical_runtime_with_sessions_from_env() -> Result<
    (
        Runtime,
        Arc<dyn SessionStore>,
        Arc<dyn MemoryBackend>,
        Arc<std::sync::Mutex<PermissionPolicy>>,
        Arc<BehaviorChainGuardHook>,
    ),
    String,
> {
    let session_store = production_session_store().await?;
    let clock: Arc<dyn apeireth_core::kernel::Clock> = apeireth_core::kernel::system_clock();
    let (cognitive, memory) = build_cognitive_modules_from_env(Arc::clone(&clock)).await?;
    let (runtime, policy, guard_hook) =
        build_canonical_runtime_with_parts(session_store.clone(), cognitive, clock).await?;
    Ok((runtime, session_store, memory, policy, guard_hook))
}

async fn build_canonical_runtime_with_parts(
    session_store: Arc<dyn SessionStore>,
    cognitive: apeireth_runtime_assembly::ProductionCognitiveModules,
    clock: Arc<dyn apeireth_core::kernel::Clock>,
) -> Result<
    (
        Runtime,
        Arc<std::sync::Mutex<PermissionPolicy>>,
        Arc<BehaviorChainGuardHook>,
    ),
    String,
> {
    use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
    use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
    use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;

    let mut builder = Runtime::builder().with_clock(Arc::clone(&clock));
    // P-arch (2026-08-27) + v2.0.0-rc.1 RC-9: KeyringSelector 真接 OS keyring
    // 优先用 keyring (设 APEIRETH_KEYRING_BACKEND env), fallback 到 EnvCredentialResolver
    // (alpha 0 装路径, 0 行为变化). 详见 `keyring_bootstrap` 模块.
    let resolver: Arc<dyn apeireth_plugin::CredentialResolver> =
        keyring_bootstrap::build_keyring_resolver();
    builder = builder.with_credentials(resolver);
    let (governance, policy, guard_hook) = build_production_governance_parts_from_env();
    // Session-level permission presets (read_only/standard/full) layer on top
    // of the production policy. The wrapper reads the session's durable
    // settings on every capability dispatch and delegates everything else.
    let preset_governance =
        apeireth_runtime_assembly::canonical::PermissionPresetGovernanceHook::new(
            Arc::new(governance),
            Arc::clone(&session_store),
        );
    builder = builder.with_governance(Arc::new(preset_governance));
    builder = builder.with_session_store(session_store);

    // The CLI is the composition root. Gateway reuses this function, while
    // SDK remains an HTTP client and does not host a second Runtime.
    // Builtin tools are owned by ProductionModules, not BuiltinToolsPlugin.
    builder = cognitive.register_context_projection(builder);
    builder = cognitive.register_into(builder);

    let mut fallback_order: Vec<CapabilityId> = Vec::new();

    let minimax = MinimaxProviderPlugin::from_env()
        .map_err(|error| format!("minimax provider activation failed: {error}"))?;
    fallback_order.push(CapabilityId::new("provider.minimax").unwrap());
    builder = builder.with_plugin(Arc::new(minimax));

    let anthropic = AnthropicProviderPlugin::from_env()
        .map_err(|error| format!("anthropic provider activation failed: {error}"))?;
    fallback_order.push(CapabilityId::new("provider.anthropic").unwrap());
    builder = builder.with_plugin(Arc::new(anthropic));

    if std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .as_ref()
        .is_some_and(|models| !models.trim().is_empty())
    {
        let openai = OpenAiCompatibleProviderPlugin::from_env()
            .map_err(|error| format!("openai-compatible provider activation failed: {error}"))?;
        fallback_order.push(CapabilityId::new("provider.openai-compatible").unwrap());
        builder = builder.with_plugin(Arc::new(openai));
    }

    builder = builder.with_fallback_order(fallback_order);
    // 上下文预算旋钮 (APEIRETH_CONTEXT_BUDGET_CHARS): 注入上下文块的总字符预算,
    // 组装期约束 provider 请求的 token 侧注入量 (核心块永不截断)。
    builder = builder.with_context_budget_chars(context_budget_chars_from_env());
    // 轮数 / 单轮工具调用预算旋钮 (APEIRETH_MAX_TURN_ROUNDS /
    // APEIRETH_MAX_TOOL_CALLS): 显式预算, 越界钳制、非法值回默认。
    builder = builder
        .with_max_rounds(turn_round_limit_from_env())
        .with_max_tool_calls_per_round(tool_call_limit_from_env());
    // 会话配额旋钮 (APEIRETH_QUOTA_MAX_TOKENS / APEIRETH_QUOTA_MAX_TOOL_STEPS /
    // APEIRETH_QUOTA_MAX_COST_MICROS): opt-in 烧钱闸, 默认不设限 (None);
    // 执行面按会话账本累计并在检查点截断。
    builder = builder.with_session_quota(apeireth_runtime::canonical::SessionQuotaLimits {
        max_tokens: quota_max_tokens_from_env(),
        max_tool_steps: quota_max_tool_steps_from_env(),
        max_cost_micros: quota_max_cost_micros_from_env(),
    });
    // 花费维单价来源 (H-P1 画像表): 回合计量处按 `usage × 单价` 记成本列,
    // 无价模型不计入只计数 (绝不编价)。与自述模型节同一张表 (同源)。
    builder = builder.with_cost_rates(Arc::new(model_profile_table_from_env()?));
    // 长尾截断的完整原文落盘根目录 (数据目录, 实际写入 `<data>/spill/`): 有落盘点时
    // 截断保留头尾预览 + 取回指引行; 落盘失败时回退内联原文 (宁长勿丢, 不失败调用)。
    builder = builder.with_context_spill_root(default_panel_data_dir());
    // 生效模型 id 与自述模型节同一解析口径 (见 [`active_model_id`])。
    if let Some(model) = active_model_id() {
        builder = builder.with_default_model(model);
    }

    let runtime = builder
        .build()
        .await
        .map_err(|error| format!("canonical runtime bootstrap failed: {error}"))?;
    if let Some(recorder) = guard_hook.dataset_recorder() {
        runtime.add_event_sink(Arc::new(
            apeireth_runtime_assembly::GuardDatasetObserver::new(recorder)
                .with_hook(guard_hook.clone()),
        ));
    }
    Ok((runtime, policy, guard_hook))
}

async fn production_session_store() -> Result<Arc<dyn apeireth_runtime::SessionStore>, String> {
    SqliteSessionStore::open(session_db_path())
        .await
        .map(|store| Arc::new(store) as Arc<dyn apeireth_runtime::SessionStore>)
        .map_err(|error| format!("session store open failed: {error}"))
}

/// 生产会话库路径 (`APEIRETH_SESSION_DB` 覆写, 默认 `.apeireth/sessions.sqlite3`)。
///
/// 自省通道的会话配额探测口读同一路径: 自述与执行面同账本。
fn session_db_path() -> PathBuf {
    std::env::var(SESSION_DB_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".apeireth/sessions.sqlite3"))
}

/// Build the direct CLI runtime with the same trace/audit observer used by the
/// HTTP gateway. The CLI has no SSE transport, but its turns must still be
/// observable and durable through the gateway bounded-context ports.
/// Build the direct CLI runtime together with its trace/audit observer.
///
/// Dataset recording is installed during the inner canonical bootstrap and
/// the returned observation sink is added additively, so both observers see
/// the same runtime event spine.
pub async fn build_canonical_runtime_from_env_with_observability(
) -> Result<(Arc<Runtime>, Arc<apeireth_gateway::RuntimeObservationSink>), String> {
    let (runtime, sessions, memory, policy, guard_hook) =
        build_canonical_runtime_with_sessions_from_env().await?;
    let runtime = Arc::new(runtime);
    let governance = Arc::new(
        apeireth_memory::SqliteMemoryStore::open(cognitive_db_path())
            .map_err(|error| format!("memory governance store open failed: {error}"))?,
    );
    let enable_local_read_tools = local_read_tools_enabled_from_env();
    let panel = Arc::new(
        crate::gateway_panels::CliPanelData::new_with_runtime(
            Arc::clone(&runtime),
            sessions,
            memory,
            governance,
            policy,
            enable_local_read_tools,
            default_panel_data_dir(),
        )
        .with_guard(guard_hook.clone()),
    );
    let services = crate::gateway_panels::gateway_services(panel);
    let observer = Arc::new(apeireth_gateway::RuntimeObservationSink::new(
        services.trace_commands.clone(),
        services.audit_commands.clone(),
    ));
    runtime.add_event_sink(observer.clone());
    Ok((runtime, observer))
}

/// Completed turn or a pending approval that still has its [`ApprovalId`].
#[derive(Debug, Clone)]
pub enum CanonicalCliTurn {
    /// The turn reached a final assistant response.
    Completed(TurnResponse),
    /// The turn is suspended until a human resolves the returned approval.
    PendingApproval(PendingApprovalView),
}

async fn build_cognitive_modules_from_env(
    clock: Arc<dyn apeireth_core::kernel::Clock>,
) -> Result<
    (
        apeireth_runtime_assembly::ProductionCognitiveModules,
        Arc<dyn MemoryBackend>,
    ),
    String,
> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_memory::{
        experience_store_sqlite::SQLiteExperienceStore,
        preference_store_sqlite::SQLitePreferenceStore,
        self_assessment_store_sqlite::SQLiteSelfAssessmentStore,
    };
    use apeireth_runtime_assembly::{CognitiveBackends, CognitiveModuleConfig, JudgeConfig};
    use apeireth_storage::SqliteConnectionPool;

    let path = cognitive_db_path();
    let pool = Arc::new(
        SqliteConnectionPool::open(&path)
            .await
            .map_err(|error| format!("cognitive backend open failed: {error}"))?,
    );

    // The storage foundation has an older generic `episodes(id, data)` table.
    // Refuse that shape explicitly instead of letting an additive
    // `CREATE IF NOT EXISTS` migration produce a runtime write failure.
    pool.read(|conn| {
        let mut statement = conn.prepare("PRAGMA table_info(episodes)")?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        if !columns.is_empty()
            && ["id", "timestamp", "role", "content", "session_id"]
                .iter()
                .any(|required| !columns.iter().any(|column| column == required))
        {
            return Err(apeireth_storage::StorageError::InvalidConfiguration(
                "cognitive database uses incompatible generic episodes schema; migrate or choose a new APEIRETH_COGNITIVE_DB path".into(),
            ));
        }
        Ok(())
    })
    .map_err(|error| format!("cognitive database schema validation failed: {error}"))?;

    // Memory migrations own the episode and six-stream tables. The preference,
    // experience, and assessment stores own their additive tables. All use
    // this one injected pool; no module opens a connection itself.
    apeireth_memory::run_migrations_on_pool(&pool)
        .await
        .map_err(|error| format!("cognitive memory schema failed: {error}"))?;

    let experience = Arc::new(SQLiteExperienceStore::from_arc(Arc::clone(&pool)));
    experience
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive experience schema failed: {error}"))?;
    let preferences = Arc::new(SQLitePreferenceStore::from_arc(Arc::clone(&pool)));
    preferences
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive preference schema failed: {error}"))?;
    let self_assessments = Arc::new(SQLiteSelfAssessmentStore::from_arc(Arc::clone(&pool)));
    self_assessments
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive assessment schema failed: {error}"))?;

    let access_history = Arc::new(apeireth_memory::SqliteAccessHistoryStore::from_arc(
        Arc::clone(&pool),
        256,
    ));
    let commitment_store = Arc::new(apeireth_memory::SqliteCommitmentStore::from_arc(
        Arc::clone(&pool),
    ));
    commitment_store
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive commitment schema failed: {error}"))?;
    let persona_store = Arc::new(apeireth_memory::SqlitePersonaProfileStore::new(
        (*pool).clone(),
    ));
    persona_store
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive persona schema failed: {error}"))?;
    let relation_store = Arc::new(apeireth_memory::SqliteTemporalGraphStore::from_arc(
        Arc::clone(&pool),
    ));
    relation_store
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive relation schema failed: {error}"))?;
    // 2026-10-06 W2 接线批: typed 写读对称修复。写侧 typed_sink 早已在生产
    // 落 commitment/persona/relation, 但读侧 typed_recall 恒 None —— 入库后
    // 永不进召回。此处把同一组 store 接给 SqliteTypedMemoryRecallSource。
    // episodes 槽**不**接: episodic 候选已由 scoped_memory 供, 且该槽吃
    // `SqliteMemoryStore` 与生产池 `SqliteBackend` 不同型, 不为接线引入第二套连接。
    let typed_identity = typed_recall_identity_from_env();
    let typed_recall_enabled = typed_recall_enabled_from_env();
    let typed_source: Arc<dyn apeireth_memory::TypedMemoryRecallSource> = Arc::new(
        apeireth_runtime_assembly::SqliteTypedMemoryRecallSource::new()
            .with_commitments(Arc::clone(&commitment_store))
            .with_persona(Arc::clone(&persona_store))
            .with_relations(Arc::clone(&relation_store)),
    );
    let persona_store: Arc<dyn apeireth_memory::PersonaProfileStore> = persona_store;
    let typed_sink = Arc::new(
        apeireth_runtime_assembly::CanonicalMemoryTypedSink::new()
            .with_commitments(Arc::clone(&commitment_store))
            .with_persona_store(Arc::clone(&persona_store))
            .with_identity(
                typed_identity.persona_id.clone(),
                typed_identity.subject_id.clone(),
            )
            .with_relations(Arc::clone(&relation_store)),
    );
    let typed_sink: Arc<dyn apeireth_memory::MemoryTypedMaterializationSink> = typed_sink;
    access_history
        .ensure_schema()
        .await
        .map_err(|error| format!("cognitive access history schema failed: {error}"))?;
    let judge_enabled = std::env::var(COGNITIVE_JUDGE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let council_enabled = std::env::var(COGNITIVE_COUNCIL_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let organs_enabled = std::env::var(ENABLE_ORGANS_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let preference_learning_enabled = preference_learning_enabled_from_env();
    let shell_enabled = std::env::var(ENABLE_SHELL_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let fetch_enabled = std::env::var(ENABLE_FETCH_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let mcp_enabled = std::env::var(ENABLE_MCP_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1");
    let reflexion_enabled = reflexion_enabled_from_env();
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    // 内容流接入 (J1) 配置: 数据目录 `feeds.json` + 受控出口通道 (公网限定,
    // 与 `tool.fetch` 同通道口径) + 记忆流写入面 (与感知链同一只 episodic
    // store 的写入面 —— EpisodeSink 同源: 同一 append-only episode 写入)。
    let content_feed_config = if content_feed_enabled_from_env() {
        let store = std::sync::Arc::new(
            apeireth_memory::SqliteMemoryStore::open(cognitive_db_path())
                .map_err(|error| format!("content feed memory store open failed: {error}"))?,
        );
        let writer: apeireth_tools_canonical::EpisodeWriter = std::sync::Arc::new(move |episode| {
            apeireth_memory::EpisodeStore::put_episode(&*store, episode)
                .map_err(|error| error.to_string())
        });
        Some(apeireth_tools_canonical::ContentFeedConfig::new(
            default_panel_data_dir(),
            std::sync::Arc::new(apeireth_tools_canonical::ControlledEgress::new(
                apeireth_tools_canonical::EgressPolicy::PublicInternetOnly,
            )),
            writer,
        ))
    } else {
        None
    };
    let config = CognitiveModuleConfig {
        judge: JudgeConfig {
            enabled: judge_enabled,
            ..JudgeConfig::default()
        },
        council: council_enabled,
        organs: organs_enabled,
        preference_learning: preference_learning_enabled,
        proactive_recall: proactive_recall_policy_from_env(),
        memory_injection: memory_injection_enabled_from_env(),
        consolidation: consolidation_enabled_from_env(),
        reflexion: reflexion_enabled,
        partner_bond: partner_bond_enabled_from_env(),
        morphology_recall: morphology_recall_enabled_from_env(),
        community_triage: community_triage_enabled_from_env(),
        education: education_enabled_from_env(),
        absorption_insight: absorption_insight_enabled_from_env(),
        // shell/fetch 旋钮: 只注册工具; 执行许可由治理层 grant+approval 决定.
        // W1 §2.4 (2026-10-10): shell 沙箱全局旋钮, 默认**开** (设计拍板
        // "产品定位=桌面伴侣"); APEIRETH_SHELL_SANDBOX=0 显式裸跑自担风险。
        shell: shell_enabled.then(|| {
            TrustedShellConfig::new(workspace_root.clone())
                .with_sandbox(shell_sandbox_enabled_from_env())
        }),
        fetch: fetch_enabled.then(FetchConfig::public_internet_only),
        // 内容流接入 (J1): 注册工具 (治理档在治理链: 拉取与入库都经审批档)。
        content_feed: content_feed_config,
        // 受控文件写入旋钮 (`tool.apply_patch`): 注册工具 + 风险档位同源
        // (治理层 grant/approval 用同一 env 解析, 即效一致)。子档依赖主开关。
        file_write: file_write_enabled_from_env(),
        file_write_auto_pass: file_write_auto_pass_enabled_from_env(),
        git_write: git_write_enabled_from_env(),
        git_write_auto_pass_l1: git_write_auto_pass_l1_enabled_from_env(),
        // 干活能力轴 P2 两级工具 (子依赖主 + file_write, fail-closed): 计划拍
        // 要写总闸同开, 落盘拍再要计划拍同开; 注册条件仍由组装根按
        // `file_write` + 工作区根 + 共享观测门禁把关 (注册即闸)。
        worktree_plan: worktree_plan_enabled_from_env(),
        worktree_apply: worktree_apply_enabled_from_env(),
        // 书→技能沉淀 (I1): 写面子依赖 file_write 总闸族 (fail-closed); 技能库
        // = 数据目录 `skills/` (每卡一 StoredDoc), 任一旋钮开才落技能库目录。
        skill_distill: skill_distill_enabled_from_env(),
        skill_search: skill_search_enabled_from_env(),
        skills_dir: (skill_distill_enabled_from_env() || skill_search_enabled_from_env())
            .then(|| default_panel_data_dir().join(apeireth_tools_canonical::skill::SKILLS_SUBDIR)),
        // MCP 外部工具桥: 装配时加载服务器列表 (env 主入口 / 数据目录次入口,
        // 坏配置拒开), 数据目录与面板其余持久档同位。
        mcp: mcp_enabled,
        mcp_data_dir: mcp_enabled.then(default_panel_data_dir),
        ..CognitiveModuleConfig::default()
    };
    let sqlite_backend = Arc::new(SqliteBackend::from_arc(Arc::clone(&pool)));
    let memory: Arc<dyn apeireth_plugin::memory_backend::MemoryBackend> = sqlite_backend.clone();
    let memory_governance: Arc<dyn apeireth_memory::MemoryGovernanceStore> = sqlite_backend.clone();
    let wiki: Arc<dyn apeireth_plugin::experience::WikiEntryStore> = experience.clone();
    let graph: Arc<dyn apeireth_plugin::experience::KnowledgeGraphStore> = experience.clone();
    let associations: Arc<dyn apeireth_plugin::experience::AssociationStore> = experience.clone();
    let preferences: Arc<dyn apeireth_plugin::preference::PreferenceStore> = preferences.clone();
    let self_assessments: Arc<dyn apeireth_plugin::self_assessment::SelfAssessmentStore> =
        self_assessments.clone();
    let council = if council_enabled {
        // 2026-09-08: Council 需要真 LlmFactory (镜像 trait 经 llm_mirror_adapter 桥).
        // 优先 openai-compatible (DeepSeek 等), 回退 MiniMax, 最后 Noop (0 装).
        Some(Arc::new(build_council_from_env()))
    } else {
        None
    };
    let scoped_memory: Arc<dyn apeireth_memory::ScopedMemoryBackend> = sqlite_backend.clone();
    let reflexion_store: Option<Arc<dyn apeireth_memory::reflexion::ReflexionStore>> =
        reflexion_enabled.then(|| -> Arc<dyn apeireth_memory::reflexion::ReflexionStore> {
            Arc::new(apeireth_memory::reflexion::FileReflexionStore::new(
                reflexion_store_root_from_env(),
            ))
        });
    let backends = CognitiveBackends {
        memory: Some(memory.clone()),
        memory_governance: Some(memory_governance),
        wiki: Some(wiki),
        graph: Some(graph),
        associations: Some(associations),
        preferences: Some(preferences),
        self_assessments: Some(self_assessments),
        council,
        workspace_root: std::env::current_dir().ok(),
        scoped_memory: Some(scoped_memory),
        embedding_provider: embedding_provider_from_env()
            .map_err(|error| format!("embedding provider config invalid: {error}"))?,
        access_history: Some(access_history),
        memory_extractor: None,
        memory_materializer: None,
        typed_recall: typed_recall_enabled.then(|| Arc::clone(&typed_source)),
        typed_recall_identity: typed_recall_enabled.then_some(typed_identity),
        typed_sink: Some(typed_sink),
        reflexion_store,
        // W2 §4.2: partner 羁绊存储 (InMemory —— 进程内, 重启即散; 持久后端为
        // 后续 sqlite 实现, trait 落点已备)。
        partner_store: partner_bond_enabled_from_env().then(
            || -> Arc<dyn apeireth_memory::partner::PartnerStore> {
                Arc::new(apeireth_memory::partner::InMemoryPartnerStore::new())
            },
        ),
        // 「性格养成」第一铲: 「从使用中学习」默认关; `APEIRETH_ENABLE_SELF_TUNING=1`
        // 才接自校准接线层 (真接检索命中/未命中信号, 调整可见/可撤销/记录透明,
        // 学习日志 tuning-log.jsonl 与 session db 同目录落位)。
        self_tuning: apeireth_runtime_assembly::SelfTuningWire::from_env(
            apeireth_runtime_assembly::tuning_log_path(),
        ),
        // 自省通道: 记忆账本 / 凭据存在性探测口 + 授权层旋钮的名册行。
        self_status_ledger: Some(self_status_ledger_probe(
            path.clone(),
            reflexion_store_root_from_env(),
        )),
        self_status_credentials: Some(self_status_credentials_probe(default_panel_data_dir())),
        self_status_extras: vec![apeireth_tools_canonical::CapabilitySwitch::new(
            "local_read_tools",
            local_read_tools_enabled_from_env(),
        )],
        // 预算一节与运行时装配同源: 同一条可配置旋钮解析路径的取值
        // (APEIRETH_MAX_TURN_ROUNDS / APEIRETH_MAX_TOOL_CALLS /
        // APEIRETH_QUOTA_MAX_*, 未设取默认)。
        self_status_budget: Some(apeireth_runtime_assembly::budget_status_from_configured(
            u64::from(turn_round_limit_from_env()),
            tool_call_limit_from_env() as u64,
            quota_max_tokens_from_env(),
            quota_max_tool_steps_from_env(),
            quota_max_cost_micros_from_env(),
        )),
        // 自省通道: 会话配额计量探测口 —— 只读会话行的账本, 已耗真值与
        // 执行面同一本账 (不设限维显示 "—")。
        self_status_quota: Some(apeireth_runtime_assembly::session_quota_probe(
            session_db_path(),
        )),
        // 自省通道: 模型画像节 (H-P1) —— 画像表三层解析 (基线/数据目录/env,
        // 坏配置拒开) + 生效模型行 (未知模型显式 unknown 行, 不是 None)。
        self_status_model_profile: {
            let table = model_profile_table_from_env()?;
            active_model_id().map(|model| table.resolve(&model).into_owned())
        },
        // 自省通道: 用户域节探测口 (W3-① 接线, 兑现 W3-2 报备的域面归口):
        // 首次组装 = 采用语义 (数据根原位数据标记为默认用户域, 零迁移零丢失),
        // 域节报当前域 id/角色/域内计数真值 (坏域配置在可失败边界拒开)。
        self_status_domain: {
            let registry = std::sync::Arc::new(
                apeireth_core::user_domain::DomainRegistry::open(default_panel_data_dir())
                    .map_err(|error| format!("user domain registry open failed: {error}"))?,
            );
            let default_domain = registry
                .load_default()
                .map_err(|error| format!("default user domain refused: {error}"))?;
            let memory = std::sync::Arc::new(
                apeireth_memory::SqliteMemoryStore::open(cognitive_db_path())
                    .map_err(|error| format!("memory ledger store open failed: {error}"))?,
            );
            Some(apeireth_runtime_assembly::domain_status_probe(
                registry,
                default_domain.user_id,
                session_db_path(),
                memory,
            ))
        },
        // 自省通道: 内容流节探测口 (J1 接线, 兑现并行批报备的「归 J1 线」):
        // 订阅数/最近拉取现探测如实 (与工具同一订阅表同一校验口径)。
        self_status_content_feed: Some(content_feed_status_probe(default_panel_data_dir())),
    };
    let modules =
        apeireth_runtime_assembly::ProductionCognitiveModules::build(config, backends, clock)
            .map_err(|error| error.to_string())?;
    Ok((modules, memory))
}

/// 认知侧 LLM 工厂 (council / orchestrator / dream) 的凭据解析链与主 runtime
/// 同源: 每次请求现解析, admin 热更凭据后下一请求即生效。
fn llm_credential_resolver() -> Arc<dyn apeireth_plugin::CredentialResolver> {
    keyring_bootstrap::build_keyring_resolver()
}

/// 构造 Council 后端（2026-09-08 用户旋钮批）：优先 OpenAI-compatible
/// （DeepSeek 等, env 配置时）→ 回退 MiniMax → 最后 Noop（0 装, advisors 会
/// 显式 NotImplemented 而非静默）。
///
/// **数值旋钮同源接入**（开关生效链审计续）：顾问数 / 单顾问超时
/// （`APEIRETH_COUNCIL_ADVISORS` / `APEIRETH_COUNCIL_TIMEOUT_MS`）与显式
/// `council` 命令走同一条 [`council_config_from_env`] 解析——生产装配的
/// council 也吃这两个旋钮，不再只有显式咨询路径生效。
fn build_council_from_env() -> apeireth_orchestration::Council {
    use apeireth_orchestration::Council;
    use apeireth_plugin::llm_factory::LlmFactory as PluginLlmFactory;

    if std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) =
            apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory::from_env_with_resolver(
                llm_credential_resolver(),
            )
        {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "deepseek-v4-flash".to_string());
            let mirror: Arc<dyn apeireth_orchestration::llm::LlmFactory> =
                Arc::new(apeireth_plugin::MirrorLlmFactory::new(Arc::new(factory)));
            return Council::with_factory(mirror, model).with_config(council_config_from_env());
        }
    }
    // MiniMax 回退仅在显式配了 MiniMax key 时成立 (0 装: 不凭空造一个会在
    // 调用时才失败的 council).
    if std::env::var("APEIRETH_API_KEY")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) =
            apeireth_provider::minimax_llm_factory::MinimaxLlmFactory::from_env_with_resolver(
                llm_credential_resolver(),
            )
        {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "MiniMax-M3".to_string());
            let mirror: Arc<dyn apeireth_orchestration::llm::LlmFactory> =
                Arc::new(apeireth_plugin::MirrorLlmFactory::new(Arc::new(factory)));
            return Council::with_factory(mirror, model).with_config(council_config_from_env());
        }
    }
    Council::default_llm().with_config(council_config_from_env())
}

/// **生产 subagent 长程任务** (2026-10-10): `plan → impl → review` 三步链
/// (`Orchestrator::orchestrate` 默认实现)。plan 步 `require_human_approval`
/// —— 主人在 CLI 交互点头 (无门自动 deny, fail-closed)。
pub async fn dispatch_subagent(
    title: String,
    payload_json: Option<String>,
) -> Result<String, String> {
    use apeireth_orchestration::Orchestrator as _;

    // 引导档案激活（普通用户预设值; 无档案 = 0 行为变化）。
    crate::onboarding::activate_for_process();
    let payload: serde_json::Value = match payload_json {
        Some(raw) => {
            serde_json::from_str(&raw).map_err(|error| format!("payload 不是合法 JSON: {error}"))?
        }
        None => serde_json::json!({}),
    };
    let orchestrator = build_subagent_orchestrator_from_env()?;
    let outcomes = orchestrator
        .orchestrate(title.clone(), payload)
        .await
        .map_err(|error| format!("subagent orchestrate 失败: {error}"))?;

    let mut text = format!(
        "subagent 长程任务「{title}」完成: {} 个结果 (plan→impl→review)\n",
        outcomes.len()
    );
    for outcome in outcomes {
        text.push_str(&format!(
            "- [{}] success={} output={}\n",
            outcome.spec_id,
            outcome.success,
            serde_json::to_string(&outcome.output).unwrap_or_default()
        ));
    }
    Ok(text)
}

/// 构造 subagent Orchestrator (工厂配方与 `build_council_from_env` 同源;
/// 0 装: 没有 key 显式报错, 不造调用时才失败的假可用)。
///
/// **W2 worktree 装饰器** (`APEIRETH_ENABLE_WORKTREE_SANDBOX=1`, 默认关): 开启时
/// 子代理跑在独立 git worktree (物理隔离, 真 `git` 命令执行器)。
fn build_subagent_orchestrator_from_env(
) -> Result<Box<dyn apeireth_orchestration::Orchestrator>, String> {
    use apeireth_orchestration::llm::LlmFactory as MirrorLlmFactoryTrait;
    use apeireth_orchestration::{
        CommandRunner, HumanApprovalGate, LlmSubagentOrchestrator, WorktreeSandboxedOrchestrator,
    };

    let (mirror, model): (Arc<dyn MirrorLlmFactoryTrait>, String) = if std::env::var(
        "APEIRETH_OPENAI_MODELS",
    )
    .ok()
    .is_some_and(|value| !value.trim().is_empty())
    {
        let factory =
            apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory::from_env_with_resolver(
                llm_credential_resolver(),
            )
            .map_err(|error| format!("OpenAI-compatible 工厂配置无效: {error}"))?;
        let model = factory
            .model_ids()
            .into_iter()
            .next()
            .unwrap_or_else(|| "deepseek-v4-flash".to_string());
        (
            Arc::new(apeireth_plugin::MirrorLlmFactory::new(Arc::new(factory))),
            model,
        )
    } else if std::env::var("APEIRETH_API_KEY")
        .ok()
        .is_some_and(|value| !value.trim().is_empty())
    {
        let factory =
            apeireth_provider::minimax_llm_factory::MinimaxLlmFactory::from_env_with_resolver(
                llm_credential_resolver(),
            )
            .map_err(|error| format!("MiniMax 工厂配置无效: {error}"))?;
        let model = factory
            .model_ids()
            .into_iter()
            .next()
            .unwrap_or_else(|| "MiniMax-M3".to_string());
        (
            Arc::new(apeireth_plugin::MirrorLlmFactory::new(Arc::new(factory))),
            model,
        )
    } else {
        return Err(
            "subagent 需要 LLM 工厂: 请设 APEIRETH_OPENAI_MODELS (+URL/KEY) 或 \
             APEIRETH_API_KEY (MiniMax)"
                .to_string(),
        );
    };

    // 人工审批门: CLI 交互 y/N (plan 步要主人点头; 拒绝 = HumanDenied)。
    let gate: HumanApprovalGate = Arc::new(|spec| {
        use std::io::Write;
        print!("subagent [{}] 需要主人批准执行 (y/N): ", spec.id);
        std::io::stdout().flush().ok();
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if line.trim().eq_ignore_ascii_case("y") {
            Ok(())
        } else {
            Err("主人拒绝".to_string())
        }
    });
    let core = LlmSubagentOrchestrator::new(mirror, model).with_approval_gate(gate);

    if worktree_sandbox_enabled_from_env() {
        let repo_root = std::env::current_dir().map_err(|error| error.to_string())?;
        let runner_root = repo_root.clone();
        let runner: CommandRunner = Arc::new(move |args: &[String]| {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&runner_root)
                .status()
                .map_err(|error| error.to_string())?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("git {args:?} 失败: {status}"))
            }
        });
        Ok(Box::new(WorktreeSandboxedOrchestrator::new(
            core, repo_root, runner,
        )))
    } else {
        Ok(Box::new(core))
    }
}

/// **W2 worktree 装饰器旋钮** (2026-10-10, 默认关): `APEIRETH_ENABLE_WORKTREE_SANDBOX=1`
/// —— subagent 跑在独立 git worktree (物理隔离)。
fn worktree_sandbox_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_WORKTREE_SANDBOX")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

fn cognitive_db_path() -> PathBuf {
    std::env::var(COGNITIVE_DB_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".apeireth/cognitive.sqlite3"))
}

/// 自省通道: 内容流节探测口 (J1) —— 订阅数/最近拉取取自 `feeds.json` 真值
/// (与工具同一订阅表、同一 StoredDoc 拒开校验口径); 每次自述现探测, 坏配置
/// 显式 `null` + 原因, 不让整帧失败 (0 装诚实: 尚无拉取 = 如实注, 不编时间)。
fn content_feed_status_probe(
    data_dir: PathBuf,
) -> apeireth_tools_canonical::StatusProbe<apeireth_tools_canonical::ContentFeedStatus> {
    Arc::new(move || {
        let store = apeireth_tools_canonical::FeedStore::new(
            data_dir.clone(),
            apeireth_tools_canonical::EgressPolicy::PublicInternetOnly,
        );
        store.status()
    })
}

/// 自省通道: 记忆账本计数探测口 —— 会话/记忆/保护计数取自认知库可查元数据
/// ([`apeireth_memory::SqliteMemoryStore::ledger_counts`]), 教训计数取自反思
/// 教训存储; **只回计数, 不回内容原文**。每次自述现探测; 失败给字段级
/// `null` + 原因, 不让整帧失败。
fn self_status_ledger_probe(
    cognitive_db: PathBuf,
    lessons_root: PathBuf,
) -> apeireth_tools_canonical::StatusProbe<apeireth_tools_canonical::MemoryLedgerStats> {
    use apeireth_memory::reflexion::ReflexionStore as _;
    Arc::new(move || {
        let store = apeireth_memory::SqliteMemoryStore::open(&cognitive_db)
            .map_err(|error| format!("memory store open failed: {error}"))?;
        let counts = store
            .ledger_counts()
            .map_err(|error| format!("memory ledger query failed: {error}"))?;
        let reflexion = apeireth_memory::reflexion::FileReflexionStore::new(lessons_root.clone());
        let (lessons, reason) = match reflexion.list_reflections() {
            Ok(reflections) => (Some(reflections.len() as u64), None),
            Err(error) => (None, Some(format!("lesson store read failed: {error}"))),
        };
        Ok(apeireth_tools_canonical::MemoryLedgerStats {
            sessions: Some(counts.sessions),
            memories: Some(counts.memories),
            protected: Some(counts.protected),
            lessons,
            reason,
        })
    })
}

/// 自省通道: 凭据存在性探测口 —— **只回布尔, 不回显凭据本体**。
///
/// 判定口径: provider 凭据 env 启动值非空 / 运行时凭据库非空 / 数据目录存在
/// 凭据存储件 (`creds.json` / 钥匙串导出物)。存在与否是自省事实, 值不是。
fn self_status_credentials_probe(data_dir: PathBuf) -> apeireth_tools_canonical::StatusProbe<bool> {
    Arc::new(move || {
        const PROVIDER_KEY_ENVS: &[&str] = &[
            "APEIRETH_API_KEY",
            "APEIRETH_ANTHROPIC_KEY",
            "APEIRETH_OPENAI_KEY",
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
        ];
        let env_present = PROVIDER_KEY_ENVS.iter().any(|key| {
            std::env::var(key)
                .ok()
                .is_some_and(|value| !value.trim().is_empty())
        });
        let hot_present = !keyring_bootstrap::hot_credential_store().is_empty();
        let file_present = [
            "creds.json",
            "apeireth-keyring.bin",
            "apeireth-keyring.master.key",
        ]
        .iter()
        .any(|name| data_dir.join(name).is_file());
        Ok(env_present || hot_present || file_present)
    })
}

/// Execute one CLI turn directly through [`Runtime::execute_outcome`].
pub async fn execute_canonical_cli_turn(
    runtime: &Runtime,
    prompt: impl Into<String>,
    model: Option<String>,
    session: Option<SessionId>,
) -> Result<CanonicalCliTurn, String> {
    let session = session.unwrap_or_else(SessionId::new);
    let prompt = prompt.into();
    let intent = RuleIntentInterpreter.interpret(IntentInput {
        session_id: session.to_string(),
        trace_id: String::new(),
        user_request: prompt.clone(),
        created_at_ms: 0,
    });
    let context = TurnSecurityContext::new(intent.intent_id.clone(), "").with_intent(intent);
    let mut request = TurnRequest::new(session, prompt).with_security_context(context);
    // 引导档案的自我描述词（`apeireth onboard` 写入）: 无档案 / 逃生门开启 =
    // None, 既有对话逐字节不变。
    if let Some(identity) = crate::onboarding::identity_system_block() {
        request = request.with_system(identity);
    }
    if let Some(model) = model {
        request = request.with_model(model);
    }
    match runtime
        .execute_outcome(request)
        .await
        .map_err(|error| error.to_string())?
    {
        TurnOutcome::Completed(response) => Ok(CanonicalCliTurn::Completed(response)),
        TurnOutcome::PendingApproval(view) => Ok(CanonicalCliTurn::PendingApproval(view)),
    }
}

/// Resolve a pending approval through the canonical runtime API.
pub async fn resolve_canonical_cli_approval(
    runtime: &Runtime,
    session: SessionId,
    approval: ApprovalId,
    decision: ApprovalDecision,
) -> Result<ApprovalResolution, String> {
    runtime
        .resolve_approval(session, approval, decision)
        .await
        .map_err(|error| error.to_string())
}

/// Bootstrap and execute the canonical CLI chat path.
pub async fn dispatch_canonical_chat(
    prompt: impl Into<String>,
    model: Option<String>,
    session: Option<String>,
) -> Result<CanonicalCliTurn, String> {
    // 引导档案激活（普通用户预设值 + 自动调参补位; 只补未设 env, 显式 env 最高,
    // 无档案 = 0 行为变化）。gateway / 桌面侧车路径不激活, 互不干扰。
    crate::onboarding::activate_for_process();
    let prompt = prompt.into();
    let session = session
        .map(|id| id.parse::<SessionId>().map_err(|error| error.to_string()))
        .transpose()?;
    let (runtime, observer) = build_canonical_runtime_from_env_with_observability().await?;
    let result = execute_canonical_cli_turn(&runtime, prompt.as_str(), model, session).await;
    observer.flush().await;
    // 普通用户自动调参: 回合一结束回写用量（best-effort, 不影响回合结果;
    // 无档案 / 非普通用户 / 调参关 = no-op）。
    if let Ok(CanonicalCliTurn::Completed(response)) = &result {
        crate::onboarding::record_turn_usage(prompt.chars().count(), response);
    }
    // W3 onering 消费 (2026-10-10, 默认关): 完成的回合留痕到跨前端统一账本
    // (best-effort 旁路, 不影响回合结果; 挂起待审批的回合不记 —— 回合未完成)。
    if onering_ledger_enabled_from_env() {
        if let Ok(CanonicalCliTurn::Completed(response)) = &result {
            onering_record_turn(&prompt, &response.text);
        }
    }
    result
}

/// **守夜人 Nightwatch** (2026-10-10, 主人批准设计): 显式命令即授权 ——
/// 离线闲时审计 (report-only)。读近 N 条 episodes 为被动快照 → 五件组合审计
/// (risk 核词扫描 / eval 行为质量 / no-degrade 复盘 / evidence 断言缺口 /
/// rubric 平衡 / colang 健康) → 报告落 `<data>/nightwatch/`。
/// **不阻塞不批准**: approval_policy 留热路径; 审计链/ballot 未持久化 = 具名缺口。
pub async fn dispatch_nightwatch(session: Option<String>, limit: usize) -> Result<String, String> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_plugin::memory_backend::MemoryBackend;
    use apeireth_runtime_assembly::canonical::nightwatch::{EpisodeSnapshot, NightwatchInputs};
    use apeireth_storage::SqliteConnectionPool;

    // 引导档案激活（普通用户预设值; 无档案 = 0 行为变化）。
    crate::onboarding::activate_for_process();
    let pool = Arc::new(
        SqliteConnectionPool::open(cognitive_db_path())
            .await
            .map_err(|error| format!("cognitive backend open failed: {error}"))?,
    );
    let backend = SqliteBackend::from_arc(Arc::clone(&pool));

    let episodes = match &session {
        Some(session_id) => backend
            .recent_episodes(session_id, limit)
            .map_err(|error| format!("read recent episodes failed: {error}"))?,
        None => Vec::new(),
    };
    let snapshots: Vec<EpisodeSnapshot> = episodes
        .iter()
        .map(|episode| EpisodeSnapshot {
            id: episode.id.clone(),
            session: session.clone().unwrap_or_else(|| "unknown".to_string()),
            role: episode.role.clone(),
            content: episode.content.clone(),
        })
        .collect();

    let inputs = NightwatchInputs {
        episodes: snapshots,
        ..Default::default()
    };
    let report = apeireth_runtime_assembly::canonical::nightwatch_audit(&inputs);
    let report_dir = default_panel_data_dir().join("nightwatch");
    let path = apeireth_runtime_assembly::canonical::write_nightwatch_report(&report_dir, &report)
        .map_err(|error| format!("nightwatch report write failed: {error}"))?;

    let mut text = format!(
        "守夜复盘完成 (report-only, 不阻塞不批准):\n{}\n报告: {}\n",
        report.summary,
        path.display()
    );
    for finding in &report.findings {
        text.push_str(&format!(
            " - [{}] {:?}: {}\n",
            finding.severity, finding.area, finding.detail
        ));
    }
    for gap in &report.advisory_gaps {
        text.push_str(&format!(" - [gap] {gap}\n"));
    }
    Ok(text)
}

/// **守夜人后台守护** (2026-10-10, 设计闭环最后一块): `nightwatch --watch`
/// = 显式授权的长跑循环。**只在用户空闲时复盘** (双闸, 纯状态机):
/// ① 活动闸: 距最近活动 ≥ `idle_secs` —— 活动信号 = 认知库 episode 时间戳
///    (用户说话 = 新 episode; `--session` 指定观测会话);
/// ② 冷却闸: 距上次复盘 ≥ `cooldown_secs` (复盘不刷屏)。
/// 轮询间隔 `interval_secs`; 复盘复用单次审计路径 (同 DB 读 + 报告落盘)。
/// Ctrl-C 退出 (信号即终止; 报告已落盘, 无中间态)。
pub async fn dispatch_nightwatch_watch(
    session: Option<String>,
    limit: usize,
    idle_secs: i64,
    cooldown_secs: i64,
    interval_secs: u64,
) -> Result<String, String> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_plugin::memory_backend::MemoryBackend;
    use apeireth_runtime_assembly::canonical::{IdleGate, NightwatchIdleScheduler};
    use apeireth_storage::SqliteConnectionPool;
    use std::time::{SystemTime, UNIX_EPOCH};

    // 引导档案激活（普通用户预设值; 无档案 = 0 行为变化）。
    crate::onboarding::activate_for_process();
    let pool = Arc::new(
        SqliteConnectionPool::open(cognitive_db_path())
            .await
            .map_err(|error| format!("cognitive backend open failed: {error}"))?,
    );
    let backend = SqliteBackend::from_arc(Arc::clone(&pool));

    let now_ms = || {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or_default()
    };
    let mut scheduler = NightwatchIdleScheduler::new(idle_secs, cooldown_secs, now_ms());
    let mut runs = 0usize;

    loop {
        let now = now_ms();
        // 活动探测: 观测会话最近一条 episode 的时间戳 (epoch 秒 → 毫秒)。
        if let Some(session_id) = &session {
            if let Ok(episodes) = backend.recent_episodes(session_id, 1) {
                if let Some(latest) = episodes.first() {
                    scheduler.note_activity(latest.timestamp.saturating_mul(1000));
                }
            }
        }

        match scheduler.should_run(now) {
            IdleGate::IdleOk => {
                scheduler.mark_ran(now);
                runs += 1;
                let summary = dispatch_nightwatch(session.clone(), limit).await?;
                println!("[nightwatch --watch] 第 {runs} 次空闲复盘:\n{summary}");
            }
            IdleGate::ActiveSoon { wait_secs } => {
                eprintln!("[nightwatch --watch] 用户仍活跃, {wait_secs}s 后再判");
            }
            IdleGate::RecentlyRan { wait_secs } => {
                eprintln!("[nightwatch --watch] 冷却中, {wait_secs}s 后再判");
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(interval_secs.max(1))).await;
    }
}

/// Bootstrap and resolve a pending approval on the production session store.
pub async fn dispatch_canonical_approval(
    session: String,
    approval: String,
    decision: ApprovalDecision,
) -> Result<ApprovalResolution, String> {
    let session = session
        .parse::<SessionId>()
        .map_err(|error| error.to_string())?;
    let approval = approval
        .parse::<ApprovalId>()
        .map_err(|error| error.to_string())?;
    let (runtime, observer) = build_canonical_runtime_from_env_with_observability().await?;
    let result = resolve_canonical_cli_approval(&runtime, session, approval, decision).await;
    observer.flush().await;
    result
}

/// **W3 onering 账本消费旋钮** (2026-10-10, 默认关):
/// `APEIRETH_ENABLE_ONERING_LEDGER=1` —— 完成的 CLI 回合把 user/assistant
/// 发言记入跨前端统一上下文账本 (`ContextLedger` = v1 `OneRingLedger` 的 v2
/// 打捞件, 见 `memory/context_ledger.rs`)。
fn onering_ledger_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_ONERING_LEDGER")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 回合 → 账本条目 (纯函数, 测试用): user prompt + assistant text, 空文本不入账
/// (账本自身也拒空留痕, 0 假装不留空行)。
fn onering_turn_entries<'a>(user: &'a str, assistant: &'a str) -> Vec<(&'static str, &'a str)> {
    let mut entries = Vec::new();
    if !user.trim().is_empty() {
        entries.push((apeireth_memory::ROLE_USER, user));
    }
    if !assistant.trim().is_empty() {
        entries.push((apeireth_memory::ROLE_ASSISTANT, assistant));
    }
    entries
}

/// 记账 (best-effort 旁路): 账本实例与治理同源 (cognitive_db_path 同一 DB,
/// 无第二连接池); **失败只降级不翻转回合** —— 账本是旁路留痕, 不是回合成败条件。
fn onering_record_turn(user: &str, assistant: &str) {
    let entries = onering_turn_entries(user, assistant);
    if entries.is_empty() {
        return;
    }
    let store = match apeireth_memory::SqliteMemoryStore::open(cognitive_db_path()) {
        Ok(store) => store,
        Err(error) => {
            eprintln!("[onering] 账本打开失败 (旁路降级, 不影响回合): {error}");
            return;
        }
    };
    let continuity = apeireth_memory::continuity_link::current_continuity_id();
    let ledger = match apeireth_memory::ContextLedger::new(&store, continuity) {
        Ok(ledger) => ledger,
        Err(error) => {
            eprintln!("[onering] 账本构造失败 (旁路降级): {error}");
            return;
        }
    };
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();
    for (role, content) in entries {
        let sender = if role == apeireth_memory::ROLE_USER {
            "cli-user"
        } else {
            "cli-assistant"
        };
        if let Err(error) = ledger.record(role, Some(sender), "cli", content, ts) {
            eprintln!("[onering] 留痕失败 (旁路降级): {error}");
        }
    }
}

/// Start the HTTP Gateway backed by one long-lived canonical runtime.
/// Blocks until the server exits.
/// **W2 §4.1 dreaming 触发载体 (决策点 D2)**: 显式命令 `apeireth dream` = 显式授权。
///
/// 语义: 取近 N 条 episodes 为梦境素材 (显式 `--session`; 未给 = 空素材, 走引擎的
/// "常规认知结构自整定"路径) → 6 阶段做梦循环 (LLM 思考器 + 确定性降级链) →
/// 苏醒写日记 (`<data>/diary`, source = `dream`)。**无旋钮**: 做梦只在此命令下
/// 发生 (W2 验收门命令变体: 显式命令即授权 + 默认不自动跑)。
pub async fn dispatch_dream(
    session: Option<String>,
    limit: usize,
    date: Option<String>,
) -> Result<String, String> {
    use apeireth_memory::backend::sqlite::SqliteBackend;
    use apeireth_memory::diary::FileDiaryStore;
    use apeireth_memory::dream_wiring::{dream_and_journal, DeterministicMetaThinker};
    use apeireth_memory::dreaming::{DreamEngine, DreamEngineConfig};
    use apeireth_memory::meta_thinking::MetaThinker;
    use apeireth_memory::procedural::InMemoryProceduralStore;
    use apeireth_plugin::memory_backend::MemoryBackend;
    use apeireth_runtime_assembly::canonical::{FallbackMetaThinker, LlmMetaThinker};
    use apeireth_storage::SqliteConnectionPool;

    // 引导档案激活（普通用户预设值; 无档案 = 0 行为变化）。
    crate::onboarding::activate_for_process();
    let path = cognitive_db_path();
    let pool = Arc::new(
        SqliteConnectionPool::open(&path)
            .await
            .map_err(|error| format!("cognitive backend open failed: {error}"))?,
    );
    let backend = SqliteBackend::from_arc(Arc::clone(&pool));

    let recent: Vec<String> = match &session {
        Some(session_id) => backend
            .recent_episodes(session_id, limit)
            .map_err(|error| format!("read recent episodes failed: {error}"))?
            .into_iter()
            .map(|episode| format!("[{}] {}", episode.role, episode.content))
            .collect(),
        None => Vec::new(),
    };

    // 决策点 D1: LLM 思考器 + 确定性降级链; 未配 LLM = 纯规则 (不造假可用)。
    let thinker: Arc<dyn MetaThinker> = match build_dream_llm_thinker() {
        Ok(llm) => Arc::new(FallbackMetaThinker::new(llm, DeterministicMetaThinker)),
        Err(_) => Arc::new(DeterministicMetaThinker),
    };

    let mut engine = DreamEngine::new(DreamEngineConfig::default());
    let procedural = InMemoryProceduralStore::new(1000);
    let diary_root = default_panel_data_dir().join("diary");
    let diary = FileDiaryStore::new(diary_root.clone());

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    // 决策点 D3: DreamReport 落日记 (引擎"苏醒阶段写入日记"本意)。
    let date = date.unwrap_or_else(|| {
        chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string()
    });
    let report = dream_and_journal(
        &mut engine,
        &recent,
        thinker.as_ref(),
        &procedural,
        &diary,
        &date,
        now_ms,
    )?;
    Ok(format!(
        "dream {} 完成 (素材 {} 条, 日记 {}): 6 阶段循环结束\n\n{}",
        report.dream_id,
        recent.len(),
        diary_root.display(),
        report.to_markdown()
    ))
}

/// 从 env 找真 LLM 工厂 (openai-compatible 优先, MiniMax 回退) + 首个模型 id。
/// 都未配 → Err (0 装: 不造"调用时才失败"的假可用)。
fn llm_factory_from_env(
) -> Result<(Arc<dyn apeireth_plugin::llm_factory::LlmFactory>, String), String> {
    if std::env::var("APEIRETH_OPENAI_MODELS")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) = apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory::from_env_with_resolver(
            llm_credential_resolver(),
        ) {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "deepseek-v4-flash".to_string());
            return Ok((
                Arc::new(factory) as Arc<dyn apeireth_plugin::llm_factory::LlmFactory>,
                model,
            ));
        }
    }
    if std::env::var("APEIRETH_API_KEY")
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
    {
        if let Ok(factory) =
            apeireth_provider::minimax_llm_factory::MinimaxLlmFactory::from_env_with_resolver(
                llm_credential_resolver(),
            )
        {
            let model = factory
                .model_ids()
                .into_iter()
                .next()
                .unwrap_or_else(|| "MiniMax-M3".to_string());
            return Ok((
                Arc::new(factory) as Arc<dyn apeireth_plugin::llm_factory::LlmFactory>,
                model,
            ));
        }
    }
    Err("no llm factory configured".to_string())
}

/// **council 旋钮** (2026-10-10 拍板, 默认 3): `APEIRETH_COUNCIL_ADVISORS=N` (1-7) =
/// 裁决顾问数 (规范序取前 N: Safety/Performance/Philosophy/History/Strategy/
/// Ethics/Legal, Safety 恒首位); `APEIRETH_COUNCIL_TIMEOUT_MS` = 单顾问超时
/// (默认 30000, 台账 #46 实测: 思考型模型 reasoning 余量下 10s 不够)。
fn council_config_from_env() -> apeireth_orchestration::CouncilConfig {
    use apeireth_orchestration::CouncilConfig;
    use std::time::Duration;
    let max_advisors = std::env::var("APEIRETH_COUNCIL_ADVISORS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .map(|n| n.clamp(1, 7))
        .unwrap_or(3);
    let per_advisor_timeout = std::env::var("APEIRETH_COUNCIL_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(30_000));
    CouncilConfig {
        max_advisors,
        per_advisor_timeout,
        ..CouncilConfig::default()
    }
}

/// **council 决策环节 D —— 显式咨询** (2026-10-10 拍板): `apeireth council "<议题>"`。
///
/// council 语义改造 (主人 2026-10-10): 从"每轮评审器"改为"决策环节顾问"。触发位点:
/// A 升级/部署批准、B 高危操作 (L3+)、C 待裁冲突批量裁决 (空闲时) —— 随对应模块
/// (upgrade_cycle ratification / 洋葱 L3-L5 / Nightwatch 守夜人) 接入; D = 本命令
/// (显式授权, 免旋钮)。一般轮次不再常开 council (深度档 = 深思直答)。
pub async fn dispatch_council(topic: String) -> Result<String, String> {
    use apeireth_core::kernel::SessionId;
    use apeireth_orchestration::{Council, CouncilVerdict, Proposal};

    // 引导档案激活（普通用户预设值; 无档案 = 0 行为变化）。
    crate::onboarding::activate_for_process();
    let (factory, model) = llm_factory_from_env()?;
    let mirror: Arc<dyn apeireth_orchestration::llm::LlmFactory> =
        Arc::new(apeireth_plugin::MirrorLlmFactory::new(factory));
    let council =
        Council::with_factory(mirror, model.clone()).with_config(council_config_from_env());

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let proposal = Proposal {
        id: format!("council-{now_ms}"),
        proposer: "cli-explicit".to_string(),
        payload: serde_json::json!({ "topic": topic, "kind": "explicit_consultation" }),
        submitted_at: now_ms,
        session_id: SessionId::new(),
    };
    let verdict = council.decide(&proposal).await;
    let summary = match &verdict {
        CouncilVerdict::Approved => "Approved (多数 Allow, 无强反对)".to_string(),
        CouncilVerdict::Vetoed { by, reason } => format!("Vetoed (by: {by:?}): {reason}"),
        CouncilVerdict::DeferToHuman { reason } => format!("DeferToHuman: {reason}"),
    };
    Ok(format!(
        "council 裁决 — 议题「{topic}」 (顾问 {} 个, 模型 {model})\n  {summary}\n\n完整判定: {verdict:?}",
        council.advisors().len()
    ))
}

/// **W1 §2.4 全局旋钮** (2026-10-10, 默认**开**): shell 沙箱 (工作区限定 + 断网)。
/// `APEIRETH_SHELL_SANDBOX=0` = 显式裸跑 (本机全权, 自担风险)。
fn shell_sandbox_enabled_from_env() -> bool {
    !std::env::var("APEIRETH_SHELL_SANDBOX")
        .ok()
        .is_some_and(|v| v.trim() == "0")
}

/// **W2 §4.2 旋钮** (2026-10-10, 默认关): partner 双向羁绊
/// (`APEIRETH_ENABLE_PARTNER_BOND=1`): TurnStart 关系状态注入 + AfterTurn 羁绊演化。
fn partner_bond_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_PARTNER_BOND")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// **W2 §4.3 旋钮** (2026-10-10, 默认关): 查询形态学自适应检索深度
/// (`APEIRETH_ENABLE_MORPHOLOGY_RECALL=1`; 温度另见 `APEIRETH_MORPHOLOGY_TEMPERATURE`)。
fn morphology_recall_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_MORPHOLOGY_RECALL")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// **W3 旋钮** (2026-10-10, 默认关): community 社区分诊接检索前置
/// (`APEIRETH_ENABLE_COMMUNITY_TRIAGE=1`; 图谱 `all_facts` 全量读 + 双级路由)。
fn community_triage_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_COMMUNITY_TRIAGE")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// **W2 §4.3 旋钮** (2026-10-10, 默认关): education Dx-Check 换元检查工具
/// (`APEIRETH_ENABLE_EDUCATION=1`)。
fn education_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_EDUCATION")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// **W2 §4.4 旋钮** (2026-10-10, 默认关): 研究吸收批认知体操
/// (`APEIRETH_ENABLE_ABSORPTION_INSIGHT=1`): AfterTurn 四算法实验性洞察 +
/// TurnStart 注入。
fn absorption_insight_enabled_from_env() -> bool {
    std::env::var("APEIRETH_ENABLE_ABSORPTION_INSIGHT")
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 构造做梦 LLM 思考器 (工厂未配则 Err, 调用方降级确定性思考器)。
fn build_dream_llm_thinker() -> Result<apeireth_runtime_assembly::canonical::LlmMetaThinker, String>
{
    let (factory, model) =
        llm_factory_from_env().map_err(|e| format!("{e} (dream uses deterministic thinker)"))?;
    Ok(apeireth_runtime_assembly::canonical::LlmMetaThinker::new(
        factory, model,
    ))
}

pub async fn dispatch_gateway_serve(port: u16) -> Result<String, String> {
    dispatch_gateway_serve_on("127.0.0.1", port).await
}

/// Start the HTTP Gateway on an explicitly selected bind address.
///
/// Loopback is the safe default. Binding a non-loopback address is an
/// intentional operator decision and is called out before the listener starts.
pub async fn dispatch_gateway_serve_on(bind: &str, port: u16) -> Result<String, String> {
    dispatch_gateway_serve_with(bind, port, None).await
}

/// Start the HTTP Gateway in the daemon-friendly ("resident") form
/// (W2-①): optional single-instance PID-file claim, explicit "already
/// running" refusals, and a graceful shutdown that finishes its audit and
/// flushes its archives before the process exits.
///
/// `pid_file` claims the machine-level resident slot (see
/// [`apeireth_gateway::resident`]): a second start against a live slot is
/// refused with an explicit "already running" error before any runtime is
/// assembled, so a resident gateway is never doubled (不双开). The path is
/// caller-chosen and absolute — never relative to the process CWD (工作目录无关).
///
/// Shutdown arrives through either the process-signal face (console control
/// events / `SIGTERM`) or the token-gated `POST /v1/admin/shutdown` endpoint;
/// both take the same closing path: final audit fact + full observation flush.
pub async fn dispatch_gateway_serve_with(
    bind: &str,
    port: u16,
    pid_file: Option<&std::path::Path>,
) -> Result<String, String> {
    // W2-② 认证面 / 远程面配置闸先行 (fail-closed 即帧, 零副作用): 坏令牌配置
    // 拒开; 逃生阀与令牌表互斥拒开; 非回环绑定必须有可校验令牌 (逃生阀
    // `none` = 操作者明示自担, 放行但留审计)。
    let gate = apeireth_gateway::GatewayTokenGate::from_env();
    let data_dir = default_panel_data_dir();
    let client_tokens = apeireth_gateway::ClientTokenTable::load(Some(&data_dir))
        .map_err(|error| format!("client token configuration refused to open: {error}"))?;
    apeireth_gateway::validate_gate_table_pair(&gate, &client_tokens)
        .map_err(|error| error.to_string())?;
    let bind_clearance = apeireth_gateway::enforce_remote_bind(bind, &gate, &client_tokens)
        .map_err(|error| error.to_string())?;
    // TLS 面接口桩: 占位旋钮被设置就显式喊破 (未接线 = 流量仍明文), 不装死。
    let tls_notice = apeireth_gateway::tls_surface_status().warning();
    if let Some(notice) = &tls_notice {
        eprintln!("WARNING: {notice}");
    }

    // 单实例占位先行: 活着的既有实例直接显式拒绝 (不装配运行时、不双开)。
    // 占位租约随函数退出自动释放 (优雅/出错路径都会清掉自己的 PID 文件)。
    if pid_file.is_some() && port == 0 {
        return Err(
            "gateway serve --pid-file requires an explicit --port: the resident slot records the \
             served port for probes, and port 0 would record a placeholder"
                .to_string(),
        );
    }
    let _resident_slot = match pid_file {
        Some(path) => {
            let record = apeireth_gateway::ResidentRecord::for_this_process(
                bind,
                port,
                apeireth_core::kernel::Timestamp::now().epoch_millis(),
            );
            Some(
                apeireth_gateway::claim_slot(path, record)
                    .await
                    .map_err(|error| error.to_string())?,
            )
        }
        None => None,
    };

    let (runtime, sessions, memory, policy, guard_hook) =
        build_canonical_runtime_with_sessions_from_env().await?;
    let runtime = Arc::new(runtime);
    let governance = Arc::new(
        apeireth_memory::SqliteMemoryStore::open(cognitive_db_path())
            .map_err(|error| format!("memory governance store open failed: {error}"))?,
    );
    let enable_local_read_tools = local_read_tools_enabled_from_env();
    // W3-① 多户数据隔离: 打开用户域数据平面 (默认域 = 数据根原位数据面, 与
    // 运行时共用同一把会话写闸; 其余域 `users/<id>/data/` 分库)。首次启动走
    // 采用语义 (只写域标记, 存量数据零移动零丢失); 坏域配置在可失败边界拒开。
    let domain_plane = apeireth_runtime_assembly::DomainDataPlane::open(
        apeireth_core::user_domain::DomainRegistry::open(&data_dir)
            .map_err(|error| format!("user domain registry open failed: {error}"))?,
        apeireth_runtime_assembly::DefaultDomainStores {
            sessions: runtime.sessions_arc().clone(),
            memory: governance.clone(),
            memory_backend: memory.clone(),
            sessions_db: session_db_path(),
        },
    )
    .await
    .map_err(|error| format!("user domain data plane open failed: {error}"))?;
    let panel = Arc::new(
        crate::gateway_panels::CliPanelData::new_with_runtime(
            Arc::clone(&runtime),
            sessions,
            memory,
            governance,
            policy,
            enable_local_read_tools,
            default_panel_data_dir(),
        )
        .with_guard(guard_hook),
    );
    let mut services = crate::gateway_panels::gateway_services(panel);
    // Wire the hot-reload api_key writer to the shared runtime credential store
    // the runtime's credential resolver reads per request (真热更: admin 写入后
    // 下一请求即生效); a configured keyring backend additionally persists the
    // value across restarts.
    services.credentials = crate::keyring_bootstrap::build_keyring_credential_writer();
    // W2-②: 客户端令牌表 (令牌族第二面) 显式注入; 远程绑定闸与 TLS 接口桩的
    // 清场事实随装配进审计面 (逃生阀可裸奔, 但必须留痕)。
    let mut state = apeireth_gateway::build_gateway_state_with_services(runtime, services);
    // W3-②: 用户角色面显式注入 (users.json StoredDoc 拒开 + 令牌↔用户绑定一致性
    // fail-closed); 未配置用户表 = 角色面未激活 (兼容路径, 行为逐字不变)。
    let user_access = apeireth_gateway::UserAccess::load(&data_dir, &client_tokens)
        .map_err(|error| format!("user role face refused to open: {error}"))?;
    state.client_tokens = client_tokens;
    state.user_access = user_access;
    // W3-①: 域上下文路由面注入 (会话面按域路由 / 越域拒绝即帧 / 面闸)。
    state.domains = Some(domain_plane.sessions_plane().clone());
    for (event, detail) in bind_clearance.audit_facts {
        state.observations.buffer_audit(&event, detail);
    }
    if let Some(notice) = tls_notice {
        state
            .observations
            .buffer_audit("gateway.tls_unwired", Some(notice));
    }
    let address = format!("{bind}:{port}");
    let listener = tokio::net::TcpListener::bind(&address).await.map_err(|error| {
        // 端口占用 = 二次启动的另一显式面: 不吐底层错误号, 直说"已在运行"。
        if error.kind() == std::io::ErrorKind::AddrInUse {
            format!(
                "gateway already running: {address} is already in use (a gateway is likely serving there)"
            )
        } else {
            format!("bind {address} failed: {error}")
        }
    })?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| format!("local_addr: {error}"))?;
    let url = format!("http://{local_addr}");

    if local_addr.ip().is_loopback() {
        eprintln!("canonical gateway started at {url}");
    } else {
        eprintln!(
            "WARNING: canonical gateway is exposed on non-loopback address {local_addr}; the token gate covers the whole API face (see {token_env})",
            token_env = apeireth_gateway::GatewayTokenGate::ENV
        );
    }
    apeireth_gateway::serve_state_with_shutdown(listener, state, graceful_shutdown_signal())
        .await
        .map_err(|error| format!("gateway server failed: {error}"))?;

    Ok(format!("server stopped at {url}"))
}

/// Process-signal face of the graceful shutdown (W2-①): console control events
/// on Windows (`Ctrl+C`), `SIGTERM` + `SIGINT` on Unix — the exact signals a
/// service manager sends when it stops a resident gateway. The other shutdown
/// source (the token-gated admin endpoint) is wired inside the gateway.
async fn graceful_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut terminate = signal(SignalKind::terminate()).ok();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = async {
                match terminate.as_mut() {
                    Some(stream) => { stream.recv().await; }
                    None => std::future::pending::<()>().await,
                }
            } => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

/// Data directory for panel archives. `APEIRETH_DATA_DIR` overrides; the
/// default is `~/.apeireth` (same place as the keyring and session dbs).
pub(crate) fn default_panel_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("APEIRETH_DATA_DIR") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".apeireth")
}

#[cfg(test)]
mod onering_ledger_tests {
    use super::*;

    #[test]
    fn turn_entries_builder_filters_empty_and_orders_roles() {
        // 五件门③ 纯函数部分: 空文本不入账, user 先 assistant 后。
        assert!(onering_turn_entries("  ", "  ").is_empty());
        let both = onering_turn_entries("问题", "回答");
        assert_eq!(both.len(), 2);
        assert_eq!(both[0].0, apeireth_memory::ROLE_USER);
        assert_eq!(both[1].0, apeireth_memory::ROLE_ASSISTANT);
        let only_assistant = onering_turn_entries("", "回答");
        assert_eq!(only_assistant.len(), 1);
        assert_eq!(only_assistant[0].0, apeireth_memory::ROLE_ASSISTANT);
    }

    #[tokio::test]
    async fn ledger_roundtrip_records_and_reads_back() {
        // 真库回路: record → recent 原样读回 (账本语义 = 统一时间线留痕)。
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("onering-test.sqlite3");
        let store = apeireth_memory::SqliteMemoryStore::open(&path).expect("open");
        let continuity = "companion-main";
        let ledger = apeireth_memory::ContextLedger::new(&store, continuity).expect("ledger");
        assert!(ledger.is_empty().expect("empty"));

        ledger
            .record(
                apeireth_memory::ROLE_USER,
                Some("cli-user"),
                "cli",
                "第一问",
                1_700_000_000_000,
            )
            .expect("record user");
        ledger
            .record(
                apeireth_memory::ROLE_ASSISTANT,
                Some("cli-assistant"),
                "cli",
                "第一答",
                1_700_000_000_001,
            )
            .expect("record assistant");

        assert_eq!(ledger.len().expect("len"), 2);
        let recent = ledger.recent(10).expect("recent");
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, "第一问");
        assert_eq!(recent[1].content, "第一答");
        assert_eq!(recent[0].frontend, "cli");
    }

    #[tokio::test]
    async fn ledger_rejects_empty_content_fail_loud() {
        // 账本既有守门 (溯源强制): 空内容/空角色显式拒绝 —— 消费侧不再自带校验。
        let dir = tempfile::tempdir().expect("temp dir");
        let store =
            apeireth_memory::SqliteMemoryStore::open(dir.path().join("onering-empty.sqlite3"))
                .expect("open");
        let ledger = apeireth_memory::ContextLedger::new(&store, "companion-main").expect("ledger");
        assert!(ledger
            .record(
                apeireth_memory::ROLE_USER,
                Some("cli-user"),
                "cli",
                "   ",
                1
            )
            .is_err());
        assert!(ledger
            .record("system", Some("x"), "cli", "内容", 1)
            .is_err());
    }
}

/// 开关生效链审计续 (治理 grant 同源 / council 数值旋钮消费): env 是进程全局,
/// 用例串在一把锁后, 逐键保存-恢复。
#[cfg(test)]
mod capability_switch_chain_tests {
    use super::*;
    use apeireth_orchestration::{
        Advisor, AdvisorDecision, AdvisorVerdict, CouncilCallError, CouncilInvoker, Proposal,
    };
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    const TOUCHED_KEYS: &[&str] = &[
        "APEIRETH_ENABLE_EDUCATION",
        "APEIRETH_COUNCIL_ADVISORS",
        "APEIRETH_COUNCIL_TIMEOUT_MS",
        "APEIRETH_OPENAI_MODELS",
        "APEIRETH_API_KEY",
    ];

    /// 清空本组用例涉及的 env (工厂 env 一并清: council 走 0 装 Noop 路径)。
    fn clear_env() -> Vec<(&'static str, Option<String>)> {
        let previous: Vec<(&'static str, Option<String>)> = TOUCHED_KEYS
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();
        for key in TOUCHED_KEYS {
            std::env::remove_var(key);
        }
        previous
    }

    fn restore_env(previous: Vec<(&'static str, Option<String>)>) {
        for (key, value) in previous {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }

    /// education 开关同源接线: 开关开 = 注册 + 授权同侧生效 (零审批只读档),
    /// 开关关 = 未授权 (fail-closed, 调用被拒)。
    #[test]
    fn education_grant_follows_the_registration_switch() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let previous = clear_env();

        std::env::set_var("APEIRETH_ENABLE_EDUCATION", "1");
        let (_, policy, _) = build_production_governance_parts_from_env();
        let policy = policy.lock().expect("policy lock");
        assert!(
            policy.has(&Permission::ExecuteTool("tool.education".to_string())),
            "开关开必须同源 grant tool.education"
        );
        assert!(
            policy
                .decision_for_capability("tool.education")
                .is_allowed(),
            "只读计算档应放行且零审批"
        );
        drop(policy);

        std::env::remove_var("APEIRETH_ENABLE_EDUCATION");
        let (_, policy, _) = build_production_governance_parts_from_env();
        let policy = policy.lock().expect("policy lock");
        assert!(
            !policy.has(&Permission::ExecuteTool("tool.education".to_string())),
            "开关关不得授权 (fail-closed)"
        );
        assert!(
            !policy
                .decision_for_capability("tool.education")
                .is_allowed(),
            "未授权能力必须拒绝"
        );

        restore_env(previous);
    }

    /// 桩 side-call 适配器: 计数可见即 max_advisors 可见。
    struct CountingInvoker;

    #[async_trait::async_trait]
    impl CouncilInvoker for CountingInvoker {
        async fn invoke(
            &self,
            _advisor: Arc<dyn Advisor>,
            _proposal: &Proposal,
        ) -> Result<AdvisorVerdict, CouncilCallError> {
            AdvisorVerdict::new(1.0, AdvisorDecision::Allow, "", Some(1.0))
                .map_err(CouncilCallError::Provider)
        }
    }

    /// council 数值旋钮同源消费: 生产装配的 council 吃
    /// `APEIRETH_COUNCIL_ADVISORS` (裁决顾问数上限即批次上限, 越界钳制 1..=7)。
    #[tokio::test]
    async fn production_council_consumes_the_advisor_count_knob() {
        let _lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let previous = clear_env();

        let proposal = Proposal {
            id: "council-knob".into(),
            proposer: "test".into(),
            payload: serde_json::json!({}),
            submitted_at: 0,
            session_id: apeireth_core::kernel::SessionId::new(),
        };

        std::env::set_var("APEIRETH_COUNCIL_ADVISORS", "2");
        let council = build_council_from_env();
        let result = council
            .decide_with_invoker(&proposal, &CountingInvoker)
            .await;
        assert_eq!(
            result.side_call_count, 2,
            "APEIRETH_COUNCIL_ADVISORS=2 必须被生产 council 消费: {:?}",
            result.evaluations
        );

        std::env::set_var("APEIRETH_COUNCIL_ADVISORS", "1");
        let council = build_council_from_env();
        let result = council
            .decide_with_invoker(&proposal, &CountingInvoker)
            .await;
        assert_eq!(result.side_call_count, 1, "顾问数上限逐档生效");

        std::env::set_var("APEIRETH_COUNCIL_ADVISORS", "99");
        let council = build_council_from_env();
        let result = council
            .decide_with_invoker(&proposal, &CountingInvoker)
            .await;
        assert_eq!(result.side_call_count, 7, "越界钳制到 7 个规范顾问位");

        restore_env(previous);
    }
}
