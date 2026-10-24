//! Canonical Apeireth command-line entry points.
//!
//! The CLI is a thin adapter: it bootstraps one canonical runtime, constructs
//! a canonical turn request, and delegates execution to
//! `Runtime::execute_outcome`. Pending approvals keep their `ApprovalId`.

// v2.0.0-rc.1 RC-9: KeyringSelector 真接 OS keyring / EncryptedFile backend; 本模块只做 bootstrap 集成.
pub mod backup;
pub mod env_switches;
// 审批面分派 (W3.7 拆分: 装配根文件尺寸棘轮已到顶; 公开路径经下方 re-export 不变).
pub mod canonical_approval;
// 心跳关怀常驻宿主接线面: FnInitiativeGate 注入 CareEngine + dispatch 接 Runtime::execute + 自述心跳两列.
pub mod bio_command;
pub mod bio_switches;
pub mod bundle_command;
pub mod care_heartbeat;
// B6 会话/记忆静态加密（隐患 #6）密钥面：engine `apeireth_storage::crypto` 的 CLI 出口（密钥丢失 = 显式不可恢复）。
pub mod crypto_keys;
// V5-b 暴露面体检 CLI 面 (只读零写; 与 `tool.exposure_audit` 同一内核两出口)。
pub mod egress_command;
pub mod exposure;
pub mod gateway_panels;
pub mod governance_chain;
pub mod keyring_bootstrap;
pub mod lessons_command;
pub mod principal_forget_port;
pub mod rate_limit_wire;
pub mod read_only_grants;
pub mod relationship_stores;
mod self_status_probe;

use self_status_probe::self_status_ledger_probe;
pub mod onboarding;
pub mod panel_governance_port;
pub mod portable_bundle;
// 裁定 ④⑤ 性格锁定 + 性格周报 (成长计既有账只读周报 + 四杆锁定/解锁显式动作面)。
pub mod trait_command;
// W3-② 用户身份与角色: `apeireth user add|list|role|remove` 管理面 (owner 角色门)。
pub mod user_admin;

pub use canonical_approval::dispatch_canonical_approval;
pub use care_heartbeat::CareHeartbeatSource;
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
    immune_memory::{ImmuneMemory, DEFAULT_CAPACITY},
    ClassifierEnforcementMode, DatasetRecorder, IntentInput, IntentInterpreter,
    JointRiskClassifier, RuleIntentInterpreter,
};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_runtime::canonical::{
    ApprovalDecision, ApprovalResolution, PendingApprovalView, Runtime, SessionStore, TurnOutcome,
    TurnRequest, TurnResponse,
};
use apeireth_runtime_assembly::canonical::BioApprovalSocialSource;
use apeireth_runtime_assembly::{canonical::production::BioProbeSlots, SqliteSessionStore};
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

/// 免疫记忆旋钮（生物机制族 W3.2，默认关）：`APEIRETH_ENABLE_IMMUNE_MEMORY=1`
/// 时才把 [`apeireth_guard::immune_memory::ImmuneMemory`] 挂到
/// [`BehaviorChainGuardHook`]；未设 / 空值 / 其它值一律关（fail-closed）。
/// 共享句柄经 hook 的 [`BehaviorChainGuardHook::shared_immune_memory`] 取回，
/// 经 `attach_immune_memory` 交器官审议门 + W3.6 自述槽（同一份 `Arc`）。
pub const ENABLE_IMMUNE_MEMORY_ENV: &str = "APEIRETH_ENABLE_IMMUNE_MEMORY";

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
// 双引擎主动关怀旋钮 (`tool.care_schedule`, V3-a): 注册 + 策略 grant + 审批档
// 钩子。默认关 (fail-closed); `APEIRETH_ENABLE_CARE=1` = 注册工具 + grant
// (「工具面有人拍板」) + [`apeireth_organ::care::CareScheduleApprovalHook`]
// 审批档 —— 改规则 = 写文件 (write_intent 同族), 每次调用都人批 (无自动放行档)。
const ENABLE_CARE_ENV: &str = "APEIRETH_ENABLE_CARE";
// G5 备份导出旋钮 (`tool.backup_export`): 注册 + 策略 grant + require_approval
// (全量打包记忆/账本落盘是敏感面, **每次导出都人批**, 无自动放行档)。
// 默认关 (fail-closed); 恢复/校验不走工具面 (CLI 本地操作: `apeireth backup`)。
pub const ENABLE_BACKUP_EXPORT_ENV: &str = "APEIRETH_ENABLE_BACKUP_EXPORT";
// 通用嗓发声旋钮 (`tool.voice_speak`, V3-b): 注册 + 策略 grant (发声面档)。
// 默认关 (fail-closed); `APEIRETH_ENABLE_VOICE_SPEAK=1` = 注册工具 + grant ——
// 发声面档 = grant 即发声 (免逐次人批, 与 shell 的 grant+require_approval
// 执行档分立), 拦截面 = 参数边界 + 勿扰窗口 (与 care_rules 同源) + 独占播放
// + 语句账本全责 (spoken.jsonl)。
pub const ENABLE_VOICE_SPEAK_ENV: &str = "APEIRETH_ENABLE_VOICE_SPEAK";
// V6-a 任务簿 (`tool.task_board`) 旋钮 (V6-a 任务编排持久队列): 默认关
// (fail-closed); `APEIRETH_ENABLE_TASK_BOARD=1` = 注册工具 + grant + 写面
// 审批钩子 + 任务队列装配 (与会话账本同库的任务表)。
pub const ENABLE_TASK_BOARD_ENV: &str = "APEIRETH_ENABLE_TASK_BOARD";
// V5-b 暴露面体检旋钮 (`tool.exposure_audit`): 注册 + 策略 grant (只读档)。
// 默认关 (fail-closed); `APEIRETH_ENABLE_EXPOSURE_AUDIT=1` = 注册工具 + grant
// —— 只读档 = grant 即用 (零审批, 与 `tool.self_status` 同档): 本地只读自查
// (体检不写被测面任何字节, 报告是唯一产出), 与 CLI `apeireth exposure report`
// 同一内核两个出口。
pub const ENABLE_EXPOSURE_AUDIT_ENV: &str = "APEIRETH_ENABLE_EXPOSURE_AUDIT";
// 自主度第一小步 · 边角料提案面旋钮 (`tool.proposal_draft`): 注册 + 策略 grant
// (低风险档: 读面 + append 提案账, 不写工作面)。默认关 (fail-closed);
// `APEIRETH_ENABLE_PROPOSALS=1` = 注册工具 + grant —— grant 即用零审批 (与
// `tool.self_status` 同档): 它提案, 人点头 (提案只入提案账 `proposals.jsonl`,
// 通过 = 账面标记 accepted, 提案不直接执行 —— 执行仍走既有工具面的正常审批链)。
// 开关语义真源在 `apeireth_tools_canonical::proposal` (命名即承诺)。
pub const ENABLE_PROPOSALS_ENV: &str = "APEIRETH_ENABLE_PROPOSALS";
// 创作工具面 W2 · 文档操作旋钮 (`tool.doc_ops`, 受控读写族): 注册 + 策略 grant。
// 默认关 (fail-closed); `APEIRETH_ENABLE_DOC_OPS=1` = 注册工具 + grant —— 读面
// (pdf_info/pdf_extract_text/docx_extract_text/docx_outline) grant 即用零审批;
// 写面 (merge/split/extract_pages/watermark_text) 受 `file_write` 唯一写总闸
// (关 = 拒写不拒读) + `DocOpsApprovalHook` 每次人批 (无自动放行档)。开关语义
// 真源在 `apeireth_tools_canonical::doc_ops` (命名即承诺)。
pub const ENABLE_DOC_OPS_ENV: &str = "APEIRETH_ENABLE_DOC_OPS";
// 创作工具面 K3 · 表格操作旋钮 (`tool.sheet_ops`, 受控读写族): 注册 + 策略 grant。
// 默认关 (fail-closed); `APEIRETH_ENABLE_SHEET_OPS=1` = 注册工具 + grant —— 读面
// (read/summary/filter/sort/aggregate/formula 不带 output) grant 即用零审批;
// 写面 (带 output 的变换动作 / export_csv) 受 `file_write` 唯一写总闸
// (关 = 拒写不拒读) + `SheetOpsApprovalHook` 每次人批 (无自动放行档)。开关语义
// 真源在 `apeireth_tools_canonical::sheet_ops` (命名即承诺)。
pub const ENABLE_SHEET_OPS_ENV: &str = "APEIRETH_ENABLE_SHEET_OPS";
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
// 书→技能沉淀旋钮 (P 轴吸收自动化 P1): `tool.skill_distill` 是写文件面 ——
// 子依赖 file_write 唯一写总闸 (fail-closed, 单独设本旋钮而写总闸关 = 无效果),
// 注册 + grant + 审批档 (每次沉淀都人批, 无自动放行档); `tool.skill_search`
// 是只读档 (零审批、零写盘), 独立旋钮不依赖写总闸。
pub const ENABLE_SKILL_DISTILL_ENV: &str = "APEIRETH_ENABLE_SKILL_DISTILL";
pub const ENABLE_SKILL_SEARCH_ENV: &str = "APEIRETH_ENABLE_SKILL_SEARCH";
// 创作工具面 W1 图像操作旋钮 (`tool.image_ops`, 受控文件写族): 子依赖
// file_write 唯一写总闸 (fail-closed, 单独设本旋钮而写总闸关 = 无效果)。
// 默认关 (fail-closed); 开 = 注册 + grant + require_approval (输出写归档
// 文件写入族, 每次图像写入都人批, 无自动放行档)。开关语义真源在
// `apeireth_tools_canonical::image_ops` (命名即承诺)。
pub const ENABLE_IMAGE_OPS_ENV: &str = "APEIRETH_ENABLE_IMAGE_OPS";
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
// V4-a 人格提取步旋钮 (`persona_consolidation`): 注册即闸, 默认关 (fail-closed);
// `APEIRETH_ENABLE_PERSONA_CONSOLIDATION=1` = 挂上人格提取步 —— 提取步随整编
// 跑 (整编触发点上从近期情节提炼人格增量 → 人格层合并), 整编关时显式不跑。
pub const ENABLE_PERSONA_CONSOLIDATION_ENV: &str = "APEIRETH_ENABLE_PERSONA_CONSOLIDATION";
// V4-b 整编散列门旋钮 (`consolidation_hash_gate`): 注册即闸, 默认关
// (fail-closed); `APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE=1` = 挂上散列增量门
// —— 整编素材同源同散列跳过重复整编 (省算力防重复记忆), 整编关时显式不跑。
pub const ENABLE_CONSOLIDATION_HASH_GATE_ENV: &str = "APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE";
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

/// V4-a 人格提取步 (`persona_consolidation`) 旋钮: 注册即闸, 默认关
/// (fail-closed)。开 = 挂上人格提取步 —— 提取步**随整编跑** (整编触发点上从
/// 近期情节提炼人格增量 → 人格层合并, 同类合并 / 矛盾标注「人格张力」);
/// 整编 (`APEIRETH_ENABLE_CONSOLIDATION`) 关时显式不跑, 名册照实 false。
pub fn persona_consolidation_enabled_from_env() -> bool {
    std::env::var(ENABLE_PERSONA_CONSOLIDATION_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// V4-b 整编散列门 (`consolidation_hash_gate`) 旋钮: 注册即闸, 默认关
/// (fail-closed)。开 = 挂上散列增量门 —— 整编素材同源同散列跳过重复整编
/// (省算力防重复记忆), 变内容照常重摄; 整编关时显式不跑, 名册照实 false。
pub fn consolidation_hash_gate_enabled_from_env() -> bool {
    std::env::var(ENABLE_CONSOLIDATION_HASH_GATE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// V5-b 暴露面体检 (`tool.exposure_audit`) 旋钮: 独立开关, 默认关 (fail-closed)。
/// 开 = 注册 + grant (只读档: grant 即用零审批, 与 `tool.self_status` 同档)。
pub fn exposure_audit_enabled_from_env() -> bool {
    std::env::var(ENABLE_EXPOSURE_AUDIT_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 暴露面体检配置 (注册即闸的另一半): 数据根 = 面板数据目录, 绑定一致性
/// 探测口与 CLI `apeireth exposure report` 同源 ([`exposure::binding_probe`]),
/// 本机监听探测口走 `netstat` 受控采集 (体检项内显式「未测」语义不变)。
pub fn exposure_audit_config_from_env(
) -> Result<Option<apeireth_tools_canonical::ExposureAuditConfig>, String> {
    if !exposure_audit_enabled_from_env() {
        return Ok(None);
    }
    let data_root = default_panel_data_dir();
    Ok(Some(
        apeireth_tools_canonical::ExposureAuditConfig::new(data_root.clone())
            .with_bindings(crate::exposure::binding_probe(data_root)),
    ))
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
    apeireth_runtime_assembly::canonical::context_budget_chars_from_env()
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

/// 成本分级路由主开关 (H-P2, fail-closed): `APEIRETH_COST_ROUTING=1` 才开,
/// 其他取值一律关 (见 [`apeireth_core::cost_router::CostRoutingConfig::switch_enabled`])。
pub fn cost_routing_enabled_from_env() -> bool {
    apeireth_core::cost_router::CostRoutingConfig::switch_enabled(
        std::env::var(apeireth_core::cost_router::COST_ROUTING_ENV)
            .ok()
            .as_deref(),
    )
}

/// 成本分级路由配置面 (H-P2): 开关 (`APEIRETH_COST_ROUTING`) + 档位映射
/// (`APEIRETH_COST_ROUTING_PREFERRED`, 每档首选模型覆盖)。任一层坏 = 整体
/// 拒开 (fail-closed, 值自由报错)。
pub fn cost_routing_config_from_env(
) -> Result<apeireth_core::cost_router::CostRoutingConfig, String> {
    apeireth_core::cost_router::CostRoutingConfig::from_env_values(
        std::env::var(apeireth_core::cost_router::COST_ROUTING_ENV)
            .ok()
            .as_deref(),
        std::env::var(apeireth_core::cost_router::COST_ROUTING_PREFERRED_ENV)
            .ok()
            .as_deref(),
    )
    .map_err(|error| format!("cost routing configuration refused to load: {error}"))
}

/// 成本分级路由器装配 (H-P2): 画像表 (筛选来源) + 生效模型 (回退目标,
/// [`active_model_id`] 同一解析口径) + 配置面。坏配置拒开: 走启动错误正式
/// 通道, 不静默回退。
pub fn cost_router_from_env() -> Result<Arc<apeireth_core::cost_router::CostRouter>, String> {
    let config = cost_routing_config_from_env()?;
    let profiles = model_profile_table_from_env()?;
    Ok(Arc::new(apeireth_core::cost_router::CostRouter::new(
        profiles,
        active_model_id(),
        config,
    )))
}

/// CLI 任务的路由信号口径 (H-P2): 上下文规模 = 提示字符数 / 4 的**启发式
/// 估计** (非计量, 只够定档用); 工具调用数信号缺位 (如实 `None`, 不猜);
/// `chat` 回合带工具循环 = 任务需要工具。
pub fn routing_request_for_prompt(prompt: &str) -> apeireth_core::cost_router::RoutingRequest {
    let estimated_context_tokens =
        u64::try_from(prompt.chars().count().div_ceil(4)).unwrap_or(u64::MAX);
    apeireth_core::cost_router::RoutingRequest::new()
        .with_requires_tools(true)
        .with_estimated_context_tokens(estimated_context_tokens)
}

/// 回合模型解析 (H-P2 任务选型): 显式 `--model` > 成本路由选定 > 默认模型。
///
/// 显式模型直接胜出 (显式 > 自动); 路由开关关 = 回 `None` (fail-closed,
/// 零行为变化, 回合仍走默认模型); 开 = 回路由决策的选定模型 (无候选回退时
/// 回的就是默认模型 id, 回退标注在自述路由节如实)。
pub fn resolve_turn_model(
    user_model: Option<String>,
    router: &apeireth_core::cost_router::CostRouter,
    prompt: &str,
) -> Option<String> {
    if let Some(model) = user_model {
        return Some(model);
    }
    router
        .route(&routing_request_for_prompt(prompt))
        .and_then(|decision| decision.selected_model)
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

/// 创作工具面 W1 图像操作 (`tool.image_ops`) 旋钮: 受控文件写族 —— 依赖写总闸
/// `APEIRETH_ENABLE_FILE_WRITE=1` (输出写归档文件写入族, fail-closed —— 单独设
/// 本旋钮而写总闸关 = 无效果)。开 = 注册 + grant + require_approval (每次图像
/// 写入都人批, 无自动放行档); 开关取值语义 (只认显式 "1") 真源在
/// [`apeireth_tools_canonical::image_ops`]。
pub fn image_ops_enabled_from_env() -> bool {
    file_write_enabled_from_env()
        && apeireth_tools_canonical::image_ops::image_ops_enabled_from_env()
}

/// 内容流接入 (`tool.content_feed`, J1) 旋钮: 独立开关, 默认关 (fail-closed)。
/// 开 = 注册 + grant + 审批档钩子 (拉取与入库都经审批档, 每次调用都人批)。
pub fn content_feed_enabled_from_env() -> bool {
    std::env::var(ENABLE_CONTENT_FEED_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 双引擎主动关怀 (`tool.care_schedule`, V3-a) 旋钮: 独立开关, 默认关
/// (fail-closed)。开 = 注册 + grant + 审批档钩子 (改规则 = 写文件走
/// write_intent 同族, 每次调用都人批)。
pub fn care_schedule_enabled_from_env() -> bool {
    std::env::var(ENABLE_CARE_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 备份导出 (`tool.backup_export`, G5) 旋钮: 独立开关, 默认关 (fail-closed)。
/// 开 = 注册 + grant + require_approval (每次导出都人批, 无自动放行档)。
pub fn backup_export_enabled_from_env() -> bool {
    std::env::var(ENABLE_BACKUP_EXPORT_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 通用嗓发声 (`tool.voice_speak`, V3-b) 旋钮: 独立开关, 默认关 (fail-closed)。
/// 开 = 注册 + grant (发声面档: grant 即发声免逐次人批, 与 shell 的
/// grant+require_approval 执行档分立; 拦截面 = 参数边界 + 勿扰窗口 + 独占播放
/// + 语句账本全责)。
pub fn voice_speak_enabled_from_env() -> bool {
    std::env::var(ENABLE_VOICE_SPEAK_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// 任务簿 (`tool.task_board`, V6-a) 旋钮: 独立开关, 默认关 (fail-closed)。
/// 开 = 注册 + grant + 写面审批钩子 (写面 = write_intent 同族每次人批,
/// `task_list` 只读零审批; failed 只能人判, 无 failed/retry 工具面) +
/// 任务队列装配 (与会话账本同库的任务表, open 即崩溃恢复点)。
pub fn task_board_enabled_from_env() -> bool {
    std::env::var(ENABLE_TASK_BOARD_ENV)
        .ok()
        .is_some_and(|value| value.trim() == "1")
}

/// R3 生产情绪源 (绑定位, 后绑注入): `voice_speak` 情绪→语气微调的真值源。
///
/// 组合根构造发声配置时把 [`Self::reader`] 注进
/// [`apeireth_tools_canonical::VoiceSpeakConfig::with_emotion_reader`]; 网关装配
/// 出 presence/PAD 合成器后调 [`Self::bind`] 把现值源接到读取面 —— 此后每次发声
/// **现读**合成器现值 ([`apeireth_gateway::PresenceService::current_pad`]), 读到
/// 什么就按冻结映射表微调什么。未绑定 / 合成器缺席 = 无情绪数据 = 原参数零变化
/// (fail-safe 朝零微调, 不猜情绪、不用基线冒充读数)。
#[derive(Clone, Default)]
pub struct VoiceEmotionSource {
    presence: Arc<std::sync::Mutex<Option<std::sync::Weak<apeireth_gateway::PresenceService>>>>,
}

impl VoiceEmotionSource {
    /// 绑定活的 presence/PAD 合成器 (组合根在网关装配处绑; 重复绑定 = 换源)。
    pub fn bind(&self, presence: &Arc<apeireth_gateway::PresenceService>) {
        *self
            .presence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Arc::downgrade(presence));
    }

    /// 绑定位现值 (诊断/测试口; 与读取面同一真值)。
    pub fn current(&self) -> Option<apeireth_tools_canonical::PadReading> {
        let slot = self
            .presence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let pad = slot.as_ref()?.upgrade()?.current_pad()?;
        Some(apeireth_tools_canonical::PadReading {
            pleasure: pad.p,
            arousal: pad.a,
            dominance: pad.d,
        })
    }

    /// 情绪读取面 (每次发声现读一次; 无现值 = `None` = 零微调)。
    pub fn reader(&self) -> apeireth_tools_canonical::EmotionReader {
        let source = self.clone();
        std::sync::Arc::new(move || source.current())
    }
}

/// 发声配置 (注册即闸的另一半): 语句账本落数据目录 (`spoken.jsonl`) +
/// 勿扰窗口 (同源接缝: `care_rules` 打扰纪律配了窗口 = 引用之, 未落/未配 =
/// 发声面最小自建 env) + 排队策略 (`APEIRETH_VOICE_INTERRUPT`, 缺省排队串行)
/// + 发声范围 (`APEIRETH_VOICE_SPEAK_SCOPE`, 缺省仅关怀/提醒, `all` 扩全量)。
/// 坏配置**拒开** (fail-closed, 不静默回退)。
///
/// 生产路径带 R3 情绪源读取面: [`voice_speak_config_with_emotion_from_env`]。
pub fn voice_speak_config_from_env(
) -> Result<Option<apeireth_tools_canonical::VoiceSpeakConfig>, String> {
    voice_speak_config_with_emotion_from_env(&VoiceEmotionSource::default())
}

/// 发声配置 + R3 生产情绪源读取面 (组合根注入真读取面, 兑现 R3 报备的
/// 「归口 CLI 接线面」): `voice_speak` 每次发声现读 presence/PAD 合成器现值
/// ([`VoiceEmotionSource`]), 情绪→语气微调生效; 无情绪数据仍零变化。
pub fn voice_speak_config_with_emotion_from_env(
    emotion: &VoiceEmotionSource,
) -> Result<Option<apeireth_tools_canonical::VoiceSpeakConfig>, String> {
    if !voice_speak_enabled_from_env() {
        return Ok(None);
    }
    let quiet =
        apeireth_runtime_assembly::voice_quiet_hours_from_care_rules(default_panel_data_dir())?;
    let playback = apeireth_tools_canonical::PlaybackPolicy::from_env()?;
    let speak_gate = apeireth_tools_canonical::SpeakGate::from_env()?;
    Ok(Some(
        apeireth_tools_canonical::VoiceSpeakConfig::new(
            default_panel_data_dir(),
            apeireth_tools_canonical::voice::system_quiet_clock(),
        )
        .with_quiet(quiet)
        .with_playback_policy(playback)
        .with_speak_gate(speak_gate)
        .with_emotion_reader(emotion.reader()),
    ))
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
        // 同口径: 显式布尔构建器不含关怀旋钮 (默认不进授权面, fail-closed)。
        false,
        // 同口径: 显式布尔构建器不含备份导出旋钮 (默认不进授权面, fail-closed)。
        false,
        // 同口径: 显式布尔构建器不含任务簿旋钮 (默认不进授权面, fail-closed)。
        false,
        // 同口径: 显式布尔构建器不含创作工具面图像操作旋钮 (默认不进授权面,
        // fail-closed)。
        false,
        // 同口径: 免疫记忆是 env 旋钮 (`APEIRETH_ENABLE_IMMUNE_MEMORY`, 默认关),
        // 显式布尔构建器不挂载 (需要时走 from_env 同源旋钮)。
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
    care_schedule: bool,
    backup_export: bool,
    task_board: bool,
    image_ops: bool,
    immune_memory: bool,
) -> (
    GovernancePipeline,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
) {
    // 默认可用的**只读**工具授予 (自省 / 代码概览 / 性格周报) —— 清单与理由见
    // `read_only_grants` 模块 (尺寸棘轮: 装配根只留一次调用)。
    let mut policy =
        read_only_grants::grant_read_only_tools(PermissionPolicy::new(), enable_local_read_tools);
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
    // 双引擎主动关怀 (`tool.care_schedule`, V3-a) 授权面 (开关开 = 注册 + 授权,
    // 同源整改口径): 只 grant「工具面有人拍板」, 审批档由
    // [`apeireth_organ::care::CareScheduleApprovalHook`] 唯一承载 —— 改规则 =
    // 写文件 (write_intent 同族), 每次调用都人批 (无自动放行档)。
    if care_schedule {
        policy.grant(Permission::ExecuteTool(
            apeireth_organ::care::CARE_SCHEDULE_CAPABILITY_ID.to_string(),
        ));
    }
    // G5 备份导出 (`tool.backup_export`) 授权面 (开关开 = 注册 + 授权, 同源
    // 整改口径): grant + require_approval —— 全量打包记忆/账本落盘是敏感面,
    // **每次导出都停人工审批** (本地审批面板 + IM 审批卡同链, 无自动放行档);
    // 凭据面由显式排除表收口 (凭据不随行, 密钥/令牌面不进包)。
    if backup_export {
        policy.grant(Permission::ExecuteTool(
            apeireth_tools_canonical::BACKUP_EXPORT_CAPABILITY_ID.to_string(),
        ));
        policy.require_approval_for(apeireth_tools_canonical::BACKUP_EXPORT_CAPABILITY_ID);
    }
    // V6-a 任务簿 (`tool.task_board`) 授权面 (开关开 = 注册 + 授权, 同源整改
    // 口径): 只 grant「工具面有人拍板」, 写面审批档由
    // [`apeireth_runtime_assembly::TaskBoardApprovalHook`] 唯一承载 —— 写面
    // (增/改派/完成) = write_intent 同族 (改任务簿 = 写文件), 每次写面都人批
    // (无自动放行档); `task_list` 只读零审批; failed 只能人判 (无工具面)。
    if task_board {
        policy.grant(Permission::ExecuteTool(
            apeireth_runtime_assembly::TASK_BOARD_CAPABILITY_ID.to_string(),
        ));
    }
    // 创作工具面 W1 (`tool.image_ops`, 受控文件写族) 授权面 (开关开 = 注册 +
    // 授权, 同源整改口径): grant + require_approval —— 输出写归档**文件写入族**,
    // **每次图像写入都停人工审批** (本地审批面板 + IM 审批卡同链, 无自动放行档);
    // 受控面 (工作区限域 / 敏感面拒 / 炸弹图上限 / 格式白名单 / 原子写 / 读前
    // 观测门禁) 由工具层全责。
    if image_ops {
        policy.grant(Permission::ExecuteTool(
            apeireth_tools_canonical::IMAGE_OPS_CAPABILITY_ID.to_string(),
        ));
        policy.require_approval_for(apeireth_tools_canonical::IMAGE_OPS_CAPABILITY_ID);
    }
    let policy = Arc::new(std::sync::Mutex::new(policy));
    let mut guard = BehaviorChainGuardHook::new();
    if let Some(dataset) = dataset {
        guard = guard.with_dataset_recorder(dataset);
    }
    // 生物机制族 W3.2: 免疫记忆默认关, 开关开才在 hook 构造处挂载 (挂载后只在
    // Deny / RequireApproval 出口追加分级信号, allow/deny 主结论与风险分不变)。
    guard = attach_immune_memory_if_enabled(guard, immune_memory);
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
    // 关怀规则面审批档映射 (授权之后、内容风险之前, 同一审批链): 改规则 = 写
    // 文件 (write_intent 同族), 每次调用都停人工审批 (RequireApproval 级, 无
    // 自动放行档), 理由载荷按写面如实标注; 非法触发器/素材拒绝即帧 (工具层已拒)。
    if care_schedule {
        pipeline = pipeline.with(Arc::new(
            apeireth_organ::care::CareScheduleApprovalHook::new(),
        ));
    }
    // 任务簿写面审批档映射 (授权之后、内容风险之前, 同一审批链): 写面 (增/
    // 改派/完成) = write_intent 同族 (改任务簿 = 写文件), 每次写面都停人工
    // 审批 (RequireApproval 级, 无自动放行档); `task_list` 只读零审批; 未知
    // 动作 fail-closed 停审批。
    if task_board {
        pipeline = pipeline.with(Arc::new(
            apeireth_runtime_assembly::TaskBoardApprovalHook::new(),
        ));
    }
    // 限流守门 (授权之后、内容风险之前, 同一审批链)。默认挂载, 见
    // `rate_limit_wire` 模块的说明 (为什么默认挂载、阈值从哪来)。
    pipeline = rate_limit_wire::attach_rate_limit(pipeline);
    let mut pipeline = pipeline
        .with(Arc::new(CredentialDisclosureHook::new()))
        .with(Arc::new(PromptInjectionHook::new()))
        .with(guard_hook.clone());
    // W3 三洋葱 L3-L5 物理执行面 (2026-10-10, 默认关): 末层纵深防御 ——
    // 授权已放行的动作再过一次双洋葱权威判定 (只收紧, 不把未授权变审批)。
    if env_switches::onion_layer_enabled_from_env() {
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

/// 免疫记忆旋钮取值解析（fail-closed）：只认显式 `"1"`（容忍首尾空白）。
///
/// `None`（未设）/ `""` / `"0"` / `"true"` / `"yes"` / `"01"` / 任意怪值一律关。
/// 入参是 `Option<&str>` 而不是直接读 env：测试可直接喂值，不依赖也不污染
/// 进程环境。
pub fn immune_memory_enabled_from_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// 从进程环境读免疫记忆旋钮：`APEIRETH_ENABLE_IMMUNE_MEMORY=1` 才开（默认关，
/// fail-closed；不提供 `DISABLE_*` 逃生门，未设即关）。
pub fn immune_memory_enabled_from_env() -> bool {
    immune_memory_enabled_from_value(std::env::var(ENABLE_IMMUNE_MEMORY_ENV).ok().as_deref())
}

/// 按开关值把免疫记忆挂到 guard hook（默认关 = 原样返回，零行为变化）。
///
/// 开时以默认容量 [`DEFAULT_CAPACITY`] / 默认冷却窗建一份进程内
/// [`ImmuneMemory`]。挂载点唯一（生产入口 guard 构造处），保证「开关 → 挂载 →
/// 共享句柄」同源：句柄经返回的 hook 的
/// [`BehaviorChainGuardHook::shared_immune_memory`] 取回（与 hook 内观察同一份
/// `Arc`），cli 诊断 / 自述面尚未读取它（W3.3 再接）。
fn attach_immune_memory_if_enabled(
    guard: BehaviorChainGuardHook,
    enabled: bool,
) -> BehaviorChainGuardHook {
    if !enabled {
        return guard;
    }
    guard.with_immune_memory(Arc::new(ImmuneMemory::with_capacity(DEFAULT_CAPACITY)))
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
        care_schedule_enabled_from_env(),
        backup_export_enabled_from_env(),
        task_board_enabled_from_env(),
        image_ops_enabled_from_env(),
        immune_memory_enabled_from_env(),
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
    env_switches::apply_recon_external_grants(&policy); // Recon/外部：开关开 = 注册 + 授权同源
                                                        // 通用嗓发声 (`tool.voice_speak`, V3-b): **开关开 = 注册 + 授权同源**
                                                        // (同一开关值同时决定装配侧注册与本授权面)。发声面档 = **grant 即发声**
                                                        // (免逐次人批), 与 shell/fetch 的 grant + require_approval 执行档**分立**
                                                        // (不借壳): 声音是输出面 (无任意文件写 —— 唯一落盘是固定路径语句账本的
                                                        // 追加行; 无网络), 拦截面 = 参数边界拒 + 勿扰窗口 (与 care_rules 同源) +
                                                        // 独占播放 + 语句账本全责。宿主要逐次人批 = 给本能力挂 require_approval,
                                                        // 发声面风险钩子会如实映射到同一条人工审批链。
    if voice_speak_enabled_from_env() {
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool(
                apeireth_tools_canonical::VOICE_SPEAK_CAPABILITY_ID.to_string(),
            ));
    }
    // V5-b 暴露面体检 (`tool.exposure_audit`): **开关开 = 注册 + 授权同源**
    // (同一开关值同时决定装配侧注册与本授权面)。只读档 = **grant 即用**
    // (零审批, 与 `tool.self_status` 同档): 本地只读自查 (体检不写被测面任何
    // 字节, 报告是唯一产出, 无网络外联)。
    if exposure_audit_enabled_from_env() {
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool(
                apeireth_tools_canonical::EXPOSURE_AUDIT_CAPABILITY_ID.to_string(),
            ));
    }
    // 自主度第一小步 · 边角料提案面 (`tool.proposal_draft`): **开关开 = 注册 +
    // 授权同源** (同一开关值同时决定装配侧注册与本授权面)。低风险档 = grant
    // 即用 (零审批, 与 `tool.self_status` 同档): 只读 + append 提案账, 不写
    // 任何工作面; 提案不直接执行 (执行走既有工具面的正常审批链)。
    if apeireth_tools_canonical::proposal::proposals_enabled_from_env() {
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool(
                apeireth_tools_canonical::proposal::PROPOSAL_CAPABILITY_ID.to_string(),
            ));
    }
    // 创作工具面 W2 · 文档操作 (`tool.doc_ops`): **开关开 = 注册 + 授权同源**
    // (同一开关值同时决定装配侧注册与本授权面)。读面 grant 即用 (零审批);
    // 写面审批档由 [`apeireth_tools_canonical::DocOpsApprovalHook`] 唯一承载
    // —— 每次写面都人批 (无自动放行档); 写总闸 (`file_write`) 关时写动作在
    // 工具面拒绝即帧 (读面不受影响)。
    let pipeline = if apeireth_tools_canonical::doc_ops_enabled_from_env() {
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool(
                apeireth_tools_canonical::DOC_OPS_CAPABILITY_ID.to_string(),
            ));
        pipeline.with(Arc::new(apeireth_tools_canonical::DocOpsApprovalHook::new()))
    } else {
        pipeline
    };
    // 创作工具面 K3 · 表格操作 (`tool.sheet_ops`): **开关开 = 注册 + 授权同源**
    // (同一开关值同时决定装配侧注册与本授权面)。读面 grant 即用 (零审批);
    // 写面审批档由 [`apeireth_tools_canonical::SheetOpsApprovalHook`] 唯一承载
    // —— 每次写面都人批 (无自动放行档); 写总闸 (`file_write`) 关时写动作在
    // 工具面拒绝即帧 (读面不受影响)。
    let pipeline = if apeireth_tools_canonical::sheet_ops_enabled_from_env() {
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool(
                apeireth_tools_canonical::SHEET_OPS_CAPABILITY_ID.to_string(),
            ));
        pipeline.with(Arc::new(
            apeireth_tools_canonical::SheetOpsApprovalHook::new(),
        ))
    } else {
        pipeline
    };
    // K 轴 L1 在场感知 (`tool.device_sense` + `tool.weather`): **开关开 = 注册 +
    // 授权同源** (同一开关值同时决定装配侧注册与本授权面)。只读档 = **grant
    // 即用** (零审批, 与 `tool.self_status` 同档): 白名单设备读 + 天气公开读;
    // 摄像头类敏感读面的显式授权 + 审计在工具边界强制 (审计缺位不放行)。
    if apeireth_tools_canonical::device::sense::device_sense_enabled_from_env() {
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool(
                apeireth_tools_canonical::device::sense::DEVICE_SENSE_CAPABILITY_ID.to_string(),
            ));
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool(
                apeireth_tools_canonical::device::weather::WEATHER_CAPABILITY_ID.to_string(),
            ));
    }
    // K 轴 L3/L4 受控执行 (`tool.device_control`): **双闸开 = 注册 + 授权同源**
    // (`APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_DEVICE_CONTROL` 齐开,
    // 默认关 fail-closed)。只 grant「工具面有人拍板」, **分级审批档由
    // [`DeviceControlApprovalHook`] 唯一承载** —— reversible 审批档 (自动档
    // 子开关只此档可配) / irreversible **永远逐次人批** / money **永远逐次
    // 人批 + IM 审批卡** (与本地审批面板 / IM 审批卡同一链)。
    let pipeline =
        if apeireth_tools_canonical::device::device_control::device_control_enabled_from_env() {
            policy
                .lock()
                .expect("permission policy lock poisoned (0 装诚实)")
                .grant(Permission::ExecuteTool(
                    apeireth_tools_canonical::device::device_control::DEVICE_CONTROL_CAPABILITY_ID
                        .to_string(),
                ));
            pipeline.with(Arc::new(
            apeireth_tools_canonical::device::device_control::DeviceControlApprovalHook::new()
                .with_auto_pass_reversible(
                    apeireth_tools_canonical::device::device_control::device_control_auto_pass_reversible_from_env(),
                ),
        ))
        } else {
            pipeline
        };
    // K 轴 L5 看家骨架 (`tool.guard_watch`): **双闸开 = 注册 + 授权同源**
    // (`APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_GUARD_WATCH` 齐开,
    // 默认关 fail-closed)。只读档 = grant 即用 (零审批, 与 `tool.self_status`
    // 同档): 守护规则读 + 白名单设备感知 + 告警账 append。**只看不动**: 告警
    // 零设备动作 (动手仍走 `tool.device_control` 三拍人批)。
    if apeireth_tools_canonical::device::guard_watch::guard_watch_enabled_from_env() {
        policy
            .lock()
            .expect("permission policy lock poisoned (0 装诚实)")
            .grant(Permission::ExecuteTool(
                apeireth_tools_canonical::device::guard_watch::GUARD_WATCH_CAPABILITY_ID
                    .to_string(),
            ));
    }
    (pipeline, policy, guard_hook)
}

/// Build the one canonical runtime used by CLI chat and the HTTP gateway.
/// Provider plugins carry credentials resolution at execution time (never stored).
pub async fn build_canonical_runtime_from_env() -> Result<Runtime, String> {
    let (runtime, _, _, _, _) = build_canonical_runtime_with_sessions_from_env().await?;
    Ok(runtime)
}

/// Build the runtime and return the shared session-store / memory-backend /
/// permission-policy handles alongside it (same stores and live policy that the
/// gateway introspection surfaces serve). 情绪源绑定位不随本口出口。
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
    let (
        runtime,
        session_store,
        memory,
        policy,
        guard_hook,
        _voice_emotion,
        _care_heartbeat,
        _cost_router,
    ) = build_canonical_runtime_with_sessions_and_voice_emotion_from_env().await?;
    Ok((runtime, session_store, memory, policy, guard_hook))
}

/// 生产网关路径的运行时组装 (R3 接线出口): 同一装配 + 情绪源/心跳源绑定位 +
/// 成本路由器 (H-P2); 生物 probe 迟绑定槽不随本口出口。
type CanonicalRuntimeParts = (
    Runtime,
    Arc<dyn SessionStore>,
    Arc<dyn MemoryBackend>,
    Arc<std::sync::Mutex<PermissionPolicy>>,
    Arc<BehaviorChainGuardHook>,
    VoiceEmotionSource,
    CareHeartbeatSource,
    Arc<apeireth_core::cost_router::CostRouter>,
);

type BioHandle = Option<Arc<BioApprovalSocialSource>>;
type BioParts = (CanonicalRuntimeParts, BioProbeSlots, BioHandle);

pub async fn build_canonical_runtime_with_sessions_and_voice_emotion_from_env(
) -> Result<CanonicalRuntimeParts, String> {
    let (parts, _slots, _approval) = build_canonical_runtime_assembly_from_env().await?;
    Ok(parts)
}

/// 装配本体: 额外回传 W3.6 probe 槽与 R1b-2c 审批社交源句柄 (开关关 = None)。
async fn build_canonical_runtime_assembly_from_env() -> Result<BioParts, String> {
    // B-P1 并行构造独立模块: 会话存储打开与认知侧组装互不依赖 (不同库/不同表),
    // 双任务并行推进 —— 启动到 ready 的等待 ≈ max(两侧) 而非两者之和。
    // 拆任务后各自持票进运行时工作线程; 顺序语义不变 (运行时组装仍等两侧齐备)。
    let clock: Arc<dyn apeireth_core::kernel::Clock> = apeireth_core::kernel::system_clock();
    let cognitive_clock = Arc::clone(&clock);
    let session_task = tokio::spawn(production_session_store());
    let cognitive_task = tokio::spawn(async move {
        let voice_emotion = VoiceEmotionSource::default();
        let care_heartbeat = CareHeartbeatSource::new();
        let (cognitive, memory, cost_router) =
            build_cognitive_modules_from_env(cognitive_clock, &voice_emotion, &care_heartbeat)
                .await?;
        Ok::<_, String>((
            cognitive,
            memory,
            voice_emotion,
            care_heartbeat,
            cost_router,
        ))
    });
    let session_store = session_task
        .await
        .map_err(|error| format!("session store assembly task failed: {error}"))??;
    let (cognitive, memory, voice_emotion, care_heartbeat, cost_router) =
        cognitive_task
            .await
            .map_err(|error| format!("cognitive assembly task failed: {error}"))??;
    // W3.6: 自述关怀心跳 probe 迟绑定 (槽未武装 = no-op, 默认关零变化)。
    let bio_probe_slots = cognitive.bio_probe_slots();
    bio_probe_slots.fill_care_status(care_heartbeat.status_probe());
    // R1b-2c: 审批社交源句柄必须在模块被装配根消费前取出 (开关关 = None)。
    let bio_approval = cognitive.bio_approval_social_source();
    // W3-①: 默认用户域的会话写闸在此预组装 —— 组合根注册进域平面 (每域恰一把)。
    let sessions = Arc::new(apeireth_runtime::SessionManager::new(
        Arc::clone(&session_store),
        Arc::clone(&clock),
    ));
    let (runtime, policy, guard_hook) =
        build_canonical_runtime_with_parts(sessions, cognitive, clock).await?;
    Ok((
        (
            runtime,
            session_store,
            memory,
            policy,
            guard_hook,
            voice_emotion,
            care_heartbeat,
            cost_router,
        ),
        bio_probe_slots,
        bio_approval,
    ))
}

/// W3.6: 迟绑定痛觉/再生读取口到已 `Arc` 化的 runtime (Weak 无环; Fill 前 = 该格真因)。
fn bind_bio_pain_reader(slots: &BioProbeSlots, runtime: &Arc<Runtime>) {
    let weak = Arc::downgrade(runtime);
    slots.fill_pain_reader(Arc::new(move || {
        weak.upgrade()
            .map(|runtime| runtime.bio_pain_snapshot())
            .ok_or_else(|| "runtime 已释放: 痛觉迟绑定读取口失效".to_string())
    }));
}

/// W3-③ 逐域伙伴运行时组装 (星群正身): 该域自己的会话写闸 + 独立记忆库 + 同款旋钮。
async fn build_domain_partner_runtime(
    sessions: Arc<apeireth_runtime::SessionManager>,
    memory_db: PathBuf,
    voice_emotion: &VoiceEmotionSource,
    care_heartbeat: &CareHeartbeatSource,
) -> Result<Runtime, String> {
    let clock: Arc<dyn apeireth_core::kernel::Clock> = apeireth_core::kernel::system_clock();
    // 域伙伴运行时不注入备份导出面 (跨域打包面收口到 owner 本地操作; 名册照实报
    // `backup_export: false`), 路由器绑定位同样丢弃 (本批任务源路由只接 CLI chat
    // 面, 自述路由节如实报未路由)。
    let (cognitive, _memory, _cost_router) = build_cognitive_modules_at(
        memory_db,
        Arc::clone(&clock),
        None,
        voice_emotion,
        care_heartbeat,
    )
    .await?;
    // W3.6: 域伙伴运行时同样把关怀心跳 probe 迟绑定到自己的槽 (开关关 = no-op)。
    cognitive
        .bio_probe_slots()
        .fill_care_status(care_heartbeat.status_probe());
    let (runtime, _policy, _guard_hook) =
        build_canonical_runtime_with_parts(sessions, cognitive, clock).await?;
    Ok(runtime)
}

async fn build_canonical_runtime_with_parts(
    sessions: Arc<apeireth_runtime::SessionManager>,
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
    // 生物机制族 W3.3 外部事件源: 免疫记忆只在 `APEIRETH_ENABLE_IMMUNE_MEMORY=1`
    // 时挂到 guard hook; `shared_immune_memory()` 取回的是 hook 正在使用的同一份
    // 句柄 (hook 的 observe 与器官审议门的 Conflict 拉取共享状态)。未挂载 = None
    // = 不接线, 器官模块与审议门行为逐字节不变。
    cognitive.attach_immune_memory(guard_hook.shared_immune_memory());
    // Session-level permission presets (read_only/standard/full) layer on top
    // of the production policy. The wrapper reads the session's durable
    // settings on every capability dispatch and delegates everything else.
    let preset_governance =
        apeireth_runtime_assembly::canonical::PermissionPresetGovernanceHook::new(
            Arc::new(governance),
            Arc::clone(sessions.store()),
        );
    builder = builder.with_governance(governance_chain::production(preset_governance));
    // 出网闸 (ADR-0014 §1.2: 把上下文发给模型就是行动): 默认关
    // (`APEIRETH_ENABLE_EGRESS_GATE=1` 才装配; 值也可以直接是授权策略 JSON),
    // 关时请求逐字节不变。
    //
    // 账本落盘到数据根 (`egress-ledger.jsonl`) —— 生产上必须落盘,
    // 否则「记录已知流向」只在内存里, 进程一退就没了。
    let egress_data_root = crate::default_panel_data_dir();
    let egress_ledger_path = egress_data_root.join("egress-ledger.jsonl");
    // 问责账本（影响声明 + 事实→行动索引）——「发现影响时通知你」的前提。
    let accountability_ledger_path = egress_data_root.join("impact-ledger.jsonl");
    // 后果账本（后果状态持久化 —— 不落盘的话重启就丢，「后果追踪」等于没追）。
    let consequence_ledger_path = egress_data_root.join("consequence-ledger.jsonl");
    if let Some(gate) = apeireth_runtime_assembly::egress_gate::egress_gate_from_env_with_all(
        Some(egress_ledger_path),
        Some(accountability_ledger_path),
        Some(consequence_ledger_path),
    ) {
        builder = builder.with_egress_gate(gate);
    }
    // 生物机制族 W3.3: 痛觉评估（默认关）。开关三层优先级（ADR-0015）：
    // env `APEIRETH_ENABLE_BIO_PAIN`（最高，含 =0 显式覆盖）→ 用户配置
    // `bio-mechanisms.json`（`apeireth bio enable nociception`）→ 默认关。
    // 只观测记账，不改变错误/重试/错误处理决策，真账见 runtime 侧测试。
    let bio_switches = crate::bio_switches::BioMechanismSwitches::load(&egress_data_root)
        .unwrap_or_else(|error| {
            // 配置坏了 = 不猜：全关 + stderr 如实报错（不静默、不假装已开）。
            eprintln!("[bio] 生物机制配置读取失败（{error}）—— 全部按关闭处理");
            crate::bio_switches::BioMechanismSwitches::default()
        });
    if bio_switches
        .raw_for("nociception")
        .is_some_and(|value| value.trim() == "1")
    {
        builder = builder.with_bio_pain(true);
    }
    // W3-①/③: 注入预组装写闸 (每域恰一把 —— 默认域与域平面共用同一实例,
    // 伙伴域用自己的那一把)。
    builder = builder.with_session_manager(sessions);

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

/// Build the direct CLI runtime together with its trace/audit observer (both
/// observers see the same event spine) and the cost-tier router (H-P2: one
/// router shared by task selection and self-report).
pub async fn build_canonical_runtime_from_env_with_observability() -> Result<
    (
        Arc<Runtime>,
        Arc<apeireth_gateway::RuntimeObservationSink>,
        Arc<apeireth_core::cost_router::CostRouter>,
        BioHandle,
    ),
    String,
> {
    let (
        (
            runtime,
            sessions,
            memory,
            policy,
            guard_hook,
            _voice_emotion,
            _care_heartbeat,
            cost_router,
        ),
        bio_probe_slots,
        bio_approval,
    ) = build_canonical_runtime_assembly_from_env().await?;
    let runtime = Arc::new(runtime);
    // W3.6: 痛觉/再生 probe 迟绑定到 runtime 自身 (Weak 无环)。
    bind_bio_pain_reader(&bio_probe_slots, &runtime);
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
    Ok((runtime, observer, cost_router, bio_approval))
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
    voice_emotion: &VoiceEmotionSource,
    care_heartbeat: &CareHeartbeatSource,
) -> Result<
    (
        apeireth_runtime_assembly::ProductionCognitiveModules,
        Arc<dyn MemoryBackend>,
        Arc<apeireth_core::cost_router::CostRouter>,
    ),
    String,
> {
    build_cognitive_modules_at(
        cognitive_db_path(),
        clock,
        default_domain_backup_config(),
        voice_emotion,
        care_heartbeat,
    )
    .await
}

/// G5 备份导出的工具面配置 (默认域专属, 旋钮开才注入 = 注册即闸)。
///
/// **域伙伴运行时 (W3-③) 不注入**: 全量打包面能外带整个数据根 (跨域数据
/// 也在内), 伙伴实例只见本域 —— 备份/搬家是 owner 的本地运维面
/// (`apeireth backup`), 不下放给域伙伴工具面 (W3-① 跨域拒绝即帧的备份面
/// 等价承诺)。域伙伴运行时名册照实报 `backup_export: false`。
fn default_domain_backup_config() -> Option<apeireth_tools_canonical::backup::BackupExportConfig> {
    backup_export_enabled_from_env()
        .then(|| backup::backup_export_config(&backup::BackupCliPaths::from_env()))
}

/// 认知模块组装的记忆库路径化形态 (W3-③ 逐域伙伴运行时: 每域自己的
/// `cognitive.sqlite3`, 每户独立记忆域; 默认域 = `cognitive_db_path()` 原位)。
async fn build_cognitive_modules_at(
    memory_db: PathBuf,
    clock: Arc<dyn apeireth_core::kernel::Clock>,
    backup_config: Option<apeireth_tools_canonical::backup::BackupExportConfig>,
    voice_emotion: &VoiceEmotionSource,
    care_heartbeat: &CareHeartbeatSource,
) -> Result<
    (
        apeireth_runtime_assembly::ProductionCognitiveModules,
        Arc<dyn MemoryBackend>,
        Arc<apeireth_core::cost_router::CostRouter>,
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

    // 成本分级路由 (H-P2): 画像表 + 生效模型 + 配置面装配一台路由器 ——
    // 自述路由节与任务选型**同一台** (同源)。坏配置拒开 (启动错误正式通道)。
    let cost_router = cost_router_from_env()?;

    let path = memory_db;
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
            apeireth_memory::SqliteMemoryStore::open(path.clone())
                .map_err(|error| format!("content feed memory store open failed: {error}"))?,
        );
        let writer_store = std::sync::Arc::clone(&store);
        let writer: apeireth_tools_canonical::EpisodeWriter = std::sync::Arc::new(move |episode| {
            apeireth_memory::EpisodeStore::put_episode(&*writer_store, episode)
                .map_err(|error| error.to_string())
        });
        // J2 拉取→摄取接缝: 条目摘要投保鲜回路 `enqueue_ingest` (散列门 / 摄取
        // 队列 / 增量折入全自动, 与 episodic 原料面并行)。只读消费记忆摄取口,
        // 逐态映射 `EnqueueOutcome` → `IngestOutcome`。
        let ingest_store = std::sync::Arc::clone(&store);
        let ingest: apeireth_tools_canonical::IngestPort =
            apeireth_tools_canonical::ingest_port(
                move |req: &apeireth_tools_canonical::IngestRequest| match ingest_store
                    .enqueue_ingest(&req.source, &req.topic, &req.content, req.now_ms)
                {
                    Ok(apeireth_memory::freshness::EnqueueOutcome::Enqueued { task_id }) => {
                        apeireth_tools_canonical::IngestOutcome::Enqueued { task_id }
                    }
                    Ok(apeireth_memory::freshness::EnqueueOutcome::DuplicateSkipped {
                        skip_count,
                        ..
                    }) => apeireth_tools_canonical::IngestOutcome::DuplicateSkipped { skip_count },
                    Ok(apeireth_memory::freshness::EnqueueOutcome::AlreadyQueued { task_id }) => {
                        apeireth_tools_canonical::IngestOutcome::AlreadyQueued { task_id }
                    }
                    Err(error) => apeireth_tools_canonical::IngestOutcome::Failed {
                        error: error.to_string(),
                    },
                },
            );
        Some(
            apeireth_tools_canonical::ContentFeedConfig::new(
                default_panel_data_dir(),
                std::sync::Arc::new(apeireth_tools_canonical::ControlledEgress::new(
                    apeireth_tools_canonical::EgressPolicy::PublicInternetOnly,
                )),
                writer,
            )
            .with_ingest_port(ingest),
        )
    } else {
        None
    };
    let task_board_queue = if task_board_enabled_from_env() {
        // V6-a 任务编排持久队列 (旋钮开才装配, 注册即闸): 任务表与会话账本
        // **同库** —— 落本域 `sessions.sqlite3` (与域伙伴逐域同账; 默认域 =
        // `.apeireth/sessions.sqlite3`, 与会话账本默认路径同文件)。`open` 即
        // 崩溃恢复点: running/assigned → pending 重派一次 (attempt 计数防死
        // 循环), 未投递唤醒重投 (redelivered 计数)。
        let task_db = path.with_file_name(TASK_QUEUE_DB_NAME);
        let store = std::sync::Arc::new(
            apeireth_runtime_assembly::SqliteTaskStore::open(task_db)
                .map_err(|error| format!("task queue store open failed: {error}"))?,
        );
        Some(std::sync::Arc::new(
            apeireth_orchestration::task_queue::TaskQueue::open(
                store,
                apeireth_orchestration::task_queue::TaskQueueConfig::default(),
            )
            .map_err(|error| format!("task queue open failed: {error}"))?,
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
        // V4-a 人格提取步 (`persona_consolidation`): 注册即闸旋钮, 默认关 ——
        // 开 = 挂上人格提取步 (随整编跑; 整编关时显式不跑, 名册照实 false)。
        persona_consolidation: persona_consolidation_enabled_from_env(),
        // V4-b 整编散列门 (`consolidation_hash_gate`): 注册即闸旋钮, 默认关 ——
        // 开 = 挂上散列增量门 (同源同散列跳过重复整编; 整编关时显式不跑)。
        consolidation_hash_gate: consolidation_hash_gate_enabled_from_env(),
        reflexion: reflexion_enabled,
        partner_bond: partner_bond_enabled_from_env(),
        morphology_recall: morphology_recall_enabled_from_env(),
        community_triage: community_triage_enabled_from_env(),
        education: education_enabled_from_env(),
        repo_map: true,
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
        // 双引擎主动关怀 (V3-a): 注册关怀规则面工具 (治理档在治理链: 改规则 =
        // 写文件走 write_intent 同族, 每次调用都人批)。关怀发起不走本面 ——
        // 发起 = 双引擎判定 → 标准回合 (零新消息通道)。
        care_schedule: care_schedule_enabled_from_env().then(default_panel_data_dir),
        // G5 备份导出 (`tool.backup_export`): 注册条件 = 默认域 + 旋钮开
        // (注册即闸; 域伙伴运行时传 None, 跨域打包面收口 owner 本地操作)。
        backup: backup_config,
        // 通用嗓发声 (`tool.voice_speak`, V3-b): 注册即闸 (旋钮开才注入配置),
        // 发声面档 (grant 即发声)。勿扰窗口走同源接缝 (care_rules 窗口优先),
        // 坏配置拒开 (fail-closed)。R3 生产情绪源: 读取面随配置注入 (现读
        // presence/PAD 合成器现值, 无情绪数据零微调)。
        voice_speak: voice_speak_config_with_emotion_from_env(voice_emotion)?,
        // V6-a 任务编排持久队列 (`tool.task_board`): 注册即闸 (旋钮开才注入
        // 队列); 治理档在治理链 (写面 = write_intent 同族每次人批, failed
        // 只能人判无工具面)。自述任务节与工具面同队同账 (探针随队列注入)。
        task_board: task_board_queue,
        context_budget_chars: Some(context_budget_chars_from_env()), // Wave 4 SG-14: 预算同真源交接装配面
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
        // 安全/逆向侦察 9 件 + 外部适配器 10 件: 双开关默认关 (fail-closed), 注册与授权同源;
        // 授权范围走 `APEIRETH_TOOL_SCOPE` (缺省 = 空范围 → 网络目标全拒绝; 坏文件 = 启动失败)。
        recon: env_switches::recon_config_from_env()?,
        external: env_switches::external_config_from_env(workspace_root.clone())?,
        // 性格周报 (`tool.trait_report`, 裁定 ⑤) + 自述性格节: 成长账 =
        // `tuning-log.jsonl` (与「性格养成」自校准学习日志**同账**, 不新造账),
        // 特质锁定档 `trait-locks.json` 同目录。只读零审批; 生产装配默认注入 =
        // 默认可用 (与 `tool.self_status` 同档)。
        trait_report: Some(apeireth_runtime_assembly::TraitGrowthPaths::new(
            apeireth_runtime_assembly::tuning_log_path(),
        )),
        // V5-b 暴露面体检 (`tool.exposure_audit`): 注册即闸 (旋钮开才注入配置),
        // 只读档 (grant 即用零审批)。与 CLI `apeireth exposure report` 同一内核
        // 两个出口, 绑定探测口同源。
        exposure_audit: exposure_audit_config_from_env()?,
        // 自主度第一小步 · 边角料提案面 (`tool.proposal_draft`): 注册即闸
        // (`APEIRETH_ENABLE_PROPOSALS` 旋钮开才注入提案账落位, 默认关
        // fail-closed)。提案账 = 数据目录 `proposals.jsonl` (append-only);
        // 自述提案节与工具面同账同源。
        proposals: apeireth_tools_canonical::proposal::ProposalConfig::from_env(
            default_panel_data_dir(),
        ),
        // 创作工具面 W1 (`tool.image_ops`, 受控文件写族): 注册即闸 (旋钮开 +
        // `file_write` 唯一写总闸 + 工作区根, 注册条件在组装根把关); 输出写归档
        // 文件写入族, 每次图像写入都人批 (授权面同源 grant + require_approval)。
        // 自述创作节与工具面同一计数器。
        image_ops: image_ops_enabled_from_env(),
        // 创作工具面 W2 · 文档操作 (`tool.doc_ops`, 受控读写族): 注册即闸
        // (`APEIRETH_ENABLE_DOC_OPS` 旋钮开才注入配置, 默认关 fail-closed)。
        // 写总闸由组装根注入 (`file_write` 同一开关值, 关 = 拒写不拒读);
        // 自述创作节与工具面同一计数器 (探针随配置注入)。
        doc_ops: apeireth_tools_canonical::DocOpsConfig::from_env(workspace_root.clone()),
        // 创作工具面 K3 · 表格操作 (`tool.sheet_ops`, 受控读写族): 注册即闸
        // (`APEIRETH_ENABLE_SHEET_OPS` 旋钮开才注入配置, 默认关 fail-closed)。
        // 写总闸由组装根注入 (`file_write` 同一开关值, 关 = 拒写不拒读);
        // 自述创作节与工具面同一计数器 (探针随配置注入)。
        sheet_ops: apeireth_tools_canonical::SheetOpsConfig::from_env(workspace_root.clone()),
        // K 轴 L1 在场感知 (`tool.device_sense` + `tool.weather`): 注册即闸
        // (`APEIRETH_ENABLE_DEVICE_SENSE` 旋钮开才注入落位, 默认关 fail-closed)。
        // 设备表 = 数据目录 `devices.json` (StoredDoc 拒开, 白名单制默认零设备);
        // 天气 = mock 档 (如实标注数据源); 自述在场节与工具面同表同源。
        device_sense: apeireth_tools_canonical::device::sense::DeviceSenseConfig::from_env(
            default_panel_data_dir(),
        ),
        weather: apeireth_tools_canonical::device::weather::WeatherConfig::from_env(
            default_panel_data_dir(),
        ),
        // K 轴 L3/L4 受控执行 (`tool.device_control`): 注册即闸 (双闸
        // `APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_DEVICE_CONTROL`
        // 齐开才注入配置, 默认关 fail-closed)。白名单 = `devices.json` 同一本
        // 设备表 (只读消费); 执行面 = mock 适配器 (真实协议族后插) + 状态账
        // `device-actions.jsonl` (append-only); 自述设备节与工具面同账。
        device_control:
            apeireth_tools_canonical::device::device_control::DeviceControlConfig::from_env(
                default_panel_data_dir(),
            ),
        // K 轴 L5 看家骨架 (`tool.guard_watch`): 注册即闸 (双闸
        // `APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_GUARD_WATCH` 齐开才
        // 注入配置, 默认关 fail-closed)。守护规则 = `guard-rules.json` (StoredDoc
        // 拒开, 规则 = 阈值/时序条件); 告警 = `guard-alerts.jsonl` (关怀候选高
        // 优先, 打扰纪律守护例外档); **只看不动** (零设备动作)。
        guard_watch: apeireth_tools_canonical::device::guard_watch::GuardWatchConfig::from_env(
            default_panel_data_dir(),
        ),
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
    // V4-a 人格记忆层落库面: `persona_memory_*` 分表的存储句柄 (与本组合同一
    // `cognitive.sqlite3`; 迁移已随记忆迁移族落位)。人格提取步 / 自述人格节 /
    // TurnStart 恒定注入三面同句柄同账。
    let persona_store: std::sync::Arc<apeireth_memory::SqliteMemoryStore> = std::sync::Arc::new(
        apeireth_memory::SqliteMemoryStore::open(path.clone())
            .map_err(|error| format!("persona memory store open failed: {error}"))?,
    );
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
        // V4-a 人格记忆层落库面 (真接线, 兑现 V4-a 报备的「归口 CLI 接线面」):
        // `persona_memory_*` 分表与本组合同一 `cognitive.sqlite3` 同账 —— 人格
        // 提取步 (persona_consolidation, 默认关) 的落库面、自述人格节的真值源、
        // TurnStart 恒定注入的回取面, 三面同一存储。
        persona_layer: Some(persona_store.clone()),
        // V4-b 保鲜回路散列增量门落库面 (真接线): 散列账 `source_hash` 与本组合
        // 同一 `cognitive.sqlite3` 同账 —— 整编散列门 (consolidation_hash_gate,
        // 默认关) 与自述记忆节的去重计数真值同源。
        ingest_gate: Some(persona_store.clone()),
        reflexion_store,
        // 关系面两个存储（羁绊 + 里程碑）：同开关、同认知库、都必须跨重启存活。
        // 构造与理由见 `relationship_stores` 模块（尺寸棘轮：装配根只留调用）。
        partner_store: relationship_stores::partner_store(partner_bond_enabled_from_env(), &pool),
        milestone_store: relationship_stores::milestone_store(
            partner_bond_enabled_from_env(),
            &pool,
        ),
        // 「性格养成」第一铲: 「从使用中学习」默认关; `APEIRETH_ENABLE_SELF_TUNING=1`
        // 才接自校准接线层 (真接检索命中/未命中信号, 调整可见/可撤销/记录透明,
        // 学习日志 tuning-log.jsonl 与 session db 同目录落位)。
        self_tuning: apeireth_runtime_assembly::SelfTuningWire::from_env(
            apeireth_runtime_assembly::tuning_log_path(),
        ),
        // V4-c 检索维度自进化: 「检索调权」默认关 fail-closed;
        // `APEIRETH_ENABLE_RETRIEVAL_TUNING=1` 才接自进化接线层 (注入后命中回执
        // → 证据 → 层权重/条目显著性小幅调权, 自进化账 `retrieval_tuning.jsonl`
        // 与 session db 同目录落位, append-only)。
        retrieval_tuning: apeireth_runtime_assembly::RetrievalTuningWire::from_env(
            apeireth_runtime_assembly::retrieval_tuning_log_path(),
        ),
        // 自省通道: 记忆账本 / 凭据存在性探测口 + 授权层旋钮的名册行。
        self_status_ledger: Some(self_status_ledger_probe(
            path.clone(),
            reflexion_store_root_from_env(),
            apeireth_runtime_assembly::retrieval_tuning_log_path(),
        )),
        // 自省通道: 问责计数探测口（影响声明 / 后果三态 / 待补救）。
        //
        // **不与出网闸共用同一个实例**是有意的：出网闸在另一个装配函数里。
        // 两者读**同一个 append-only 账本文件**（`impact-ledger.jsonl` /
        // `consequence-ledger.jsonl`）—— 而闸写事件时**先落盘再进内存**，
        // 所以从文件读能拿到最新真值。**文件就是共享真源**，不需要跨函数传状态。
        self_status_accountability: Some(
            apeireth_runtime_assembly::accountability_probe::accountability_status_probe(
                default_panel_data_dir(),
                Arc::new(tokio::sync::Mutex::new(
                    apeireth_runtime_assembly::consequence::ConsequenceTracker::with_ledger(
                        default_panel_data_dir().join("consequence-ledger.jsonl"),
                    ),
                )),
            ),
        ),
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
        // 自省通道: 成本分级路由节 (H-P2) —— 主开关 + 最近一次路由决策真值
        // 同源现读 (尚无任务被路由照实报, 不编档位/选定)。
        self_status_cost_routing: Some(apeireth_runtime_assembly::cost_routing_probe(Arc::clone(
            &cost_router,
        ))),
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
        // 自省通道: 双引擎关怀节探测口 (V3-a 接线 + 心跳态随行): 规则数/下次
        // 触发/最近关怀现探测如实 (与工具同一 `care_rules.json` 同一拒开口径),
        // 心跳两列现读常驻心跳生产者真值 ([`CareHeartbeatSource`] 绑定位;
        // 未起/停机 = 显式 null, 不冒充在跑)。时钟走系统时钟; 交互观察锚点未
        // 接线 (久未联系类规则退化为规则创建时刻锚, 如实)。
        self_status_care: Some({
            use apeireth_organ::care::CareClock;
            apeireth_runtime_assembly::care_status_probe_with_heartbeat(
                default_panel_data_dir(),
                std::sync::Arc::new(|| apeireth_organ::care::SystemCareClock.now_ms()),
                std::sync::Arc::new(|| None),
                Some(care_heartbeat.status_probe()),
            )
        }),
        // 自省通道: 备份节探测口 (G5 接线): 最近备份时间/清单件数真值现探测
        // (与导出同一 `last-backup.json` 记录; 尚无备份显式 null + 原因)。
        self_status_backup: Some(apeireth_runtime_assembly::backup_status_probe(
            backup::backup_ledger_path(&default_panel_data_dir()),
        )),
        // 自省通道: 语音节探测口 (V3-b 接线): 账本数/最近说话/嗓名现探测
        // (与发声工具同一本语句账本 `spoken.jsonl`; 摘录口径不回全文)。
        self_status_voice: Some(voice_status_probe(default_panel_data_dir())),
        // 自省通道: 人格节探测口 (`self_status_persona`, V4-a 接线): 信念/边界/
        // 偏好/张力计数 + 最后整编真值现探测 (与人格层落库面同账, 只计数不回内容)。
        self_status_persona: Some(apeireth_runtime_assembly::persona_status_probe(
            persona_store,
        )),
    };
    let modules =
        apeireth_runtime_assembly::ProductionCognitiveModules::build(config, backends, clock)
            .map_err(|error| error.to_string())?;
    Ok((modules, memory, cost_router))
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

/// 任务队列表的库文件名 (与会话账本同库: 每域恰一个 `sessions.sqlite3`)。
const TASK_QUEUE_DB_NAME: &str = "sessions.sqlite3";

/// 默认域任务队列库路径 (与认知库同目录派生, 组合根一处派口径)。
///
/// 任务面板面与 `tool.task_board` 工具面必须**同店同账**: 工具面在
/// `build_cognitive_modules_at` 按其记忆库路径派生同名文件, 默认域两者合一。
fn task_queue_db_path() -> PathBuf {
    cognitive_db_path().with_file_name(TASK_QUEUE_DB_NAME)
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

/// 自省通道: 语音节探测口 (V3-b) —— 账本数/最近说话/嗓名取自语句账本
/// `spoken.jsonl` 真值 (与发声工具同一本账、同一逐记录读回口径); 每次自述
/// 现探测, 失败显式 `null` + 原因, 不让整帧失败 (0 装诚实: 尚无语句 = 如实
/// 注, 不编时间; 摘录口径不回全文)。
fn voice_status_probe(
    data_dir: PathBuf,
) -> apeireth_tools_canonical::StatusProbe<apeireth_tools_canonical::VoiceStatus> {
    Arc::new(move || apeireth_tools_canonical::SpokenLedger::under_data_dir(&data_dir).status())
}

/// 自省通道: 记忆账本计数探测口 —— 会话/记忆/保护计数取自认知库可查元数据
/// ([`apeireth_memory::SqliteMemoryStore::ledger_counts`]), 教训计数取自反思
/// 教训存储; **只回计数, 不回内容原文**。每次自述现探测; 失败给字段级
/// `null` + 原因, 不让整帧失败。
/// V4-b 记忆节扩: 待摄取数 / 最近折入 / 去重计数取自保鲜回路真值
/// ([`apeireth_memory::SqliteMemoryStore::freshness_status`], 与账本计数同库
/// 同账); 尚未折入 = 显式 null (不编时间)。
/// V4-c 记忆节扩: 进化次数 / 最近调权真值取自检索自进化账
/// (`retrieval_tuning.jsonl`, 与调权同一本账只读读回); 尚未调权 = 显式
/// null (不编移动)。
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
    let (runtime, observer, cost_router, _bio_approval) =
        build_canonical_runtime_from_env_with_observability().await?;
    // 成本分级路由 (H-P2 任务选型): 显式 `--model` > 路由选定 > 默认模型;
    // 开关关 = fail-closed 零行为变化。路由决策即自述路由节真值 (同一路由器)。
    let model = resolve_turn_model(model, &cost_router, &prompt);
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
    // TLS 面 (B5 真接线): 旋钮在场即真加载; 材料坏 = fail-closed 拒启动
    // (不是警告后照跑 —— 接口桩语义已退役)。
    let tls_surface = apeireth_gateway::tls_surface_status();
    if let Some(notice) = tls_surface.notice() {
        eprintln!("{notice}");
    }
    if let Some(refusal) = tls_surface.refusal() {
        return Err(refusal);
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

    // 机械补位 (编译断点, 网关语义零改动): 组装出口新增路由器绑定位 (H-P2),
    // 网关路径不接任务源路由, 丢弃该绑定位 —— 其自述路由节照实报「尚无任务
    // 被路由」(网关面路由接线不在本批战场)。
    let (
        (
            runtime,
            sessions,
            memory,
            policy,
            guard_hook,
            voice_emotion,
            care_heartbeat,
            _cost_router,
        ),
        bio_probe_slots,
        _bio_approval,
    ) = build_canonical_runtime_assembly_from_env().await?;
    let runtime = Arc::new(runtime);
    // W3.6: 常驻宿主同样把痛觉/再生 probe 迟绑定到 runtime。
    bind_bio_pain_reader(&bio_probe_slots, &runtime);
    let governance = Arc::new(
        apeireth_memory::SqliteMemoryStore::open(cognitive_db_path())
            .map_err(|error| format!("memory governance store open failed: {error}"))?,
    );
    let enable_local_read_tools = local_read_tools_enabled_from_env();
    // W3-① 多户数据隔离: 打开用户域数据平面 (默认域 = 数据根原位数据面, 与
    // 运行时共用同一把会话写闸; 其余域 `users/<id>/data/` 分库)。首次启动走
    // 采用语义 (只写域标记, 存量数据零移动零丢失); 坏域配置在可失败边界拒开。
    let domain_registry = apeireth_core::user_domain::DomainRegistry::open(&data_dir)
        .map_err(|error| format!("user domain registry open failed: {error}"))?;
    let domain_plane = apeireth_runtime_assembly::DomainDataPlane::open(
        domain_registry.clone(),
        apeireth_runtime_assembly::DefaultDomainStores {
            sessions: runtime.sessions_arc().clone(),
            memory: governance.clone(),
            memory_backend: memory.clone(),
            sessions_db: session_db_path(),
            // 共树协同补位 (宪章 §二.4 报备, W3-① 并行线结构演进 `memory_db`):
            // 默认域记忆库路径 = 组合根的同一认知库 (cognitive.sqlite3, 原位语义)。
            memory_db: cognitive_db_path(),
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
    // V2-b 用户/域管理面 (双入口同源, 跨范围最小接线报备见交付说明): HTTP 面与
    // CLI `apeireth user` 共用同一 UserTable 服务 (同一 `users.json`), 域管理口
    // 与域平面共用同一域注册表 (同一数据根)。
    services.user_admin = Some(Arc::new(apeireth_gateway::UserAdminService::new(&data_dir)));
    services.domain_admin = Some(Arc::new(apeireth_gateway::DomainAdminService::new(
        domain_registry,
    )));
    // 技能库只读面 (GET /v1/panel/skills): 与 skill_search/skill_distill 同一
    // 技能库 (`<data_dir>/skills/`, StoredDoc 拒开语义) 的元数据投影 —— 只读,
    // 不回卡体 (卡体走 L2 封套面), 坏卡如实列拒不砖库。
    services.skills = Some(Arc::new(
        crate::gateway_panels::CliSkillShelf::under_data_dir(&data_dir),
    ));
    // 关怀面板只读面 (GET /v1/panel/care): `care_list` 帧同形 + 关怀节真值
    // (含心跳两列) —— 与 `tool.care_schedule` 同一 `care_rules.json` 同一拒开
    // 口径; 心跳两列现读常驻心跳生产者真值 (与自述关怀节同一绑定, 未起 = 显式
    // null, 不冒充在跑)。只读零写面 (改规则走工具面, 每调用停人工审批)。
    services.care = Some(Arc::new(crate::gateway_panels::CliCarePanel::new(
        default_panel_data_dir(),
        care_heartbeat.status_probe(),
    )));
    // 性格面板只读面 (GET /v1/panel/traits): 四滑杆现值/锁定态/最近移动 ——
    // 与 `tool.trait_report` **同账同源** (同一成长计既有账 tuning-log.jsonl +
    // 同一锁定档 trait-locks.json)。只读零写面 (锁定/解锁走 CLI 显式动作)。
    services.traits = Some(Arc::new(crate::gateway_panels::CliTraitPanel::new(
        apeireth_runtime_assembly::TraitGrowthPaths::new(
            apeireth_runtime_assembly::tuning_log_path(),
        ),
    )));
    // 任务面板面 (`GET /v1/panel/tasks` 只读 + `POST .../human-judge-retry` 人判
    // 重试): 与 `tool.task_board` 工具面**同队同账** (同一 `sessions.sqlite3`
    // 任务表), 附从句柄不重复崩溃恢复 (恢复点只归工具面主句柄的 `open`)。
    // 任务簿旋钮开才接线 (注册即闸, 默认关 → 501 unsupported 如实报);
    // 面板边界: 只读 + 人判重试, 增删改派走工具面不进面板。
    services.tasks = if task_board_enabled_from_env() {
        Some(Arc::new(crate::gateway_panels::CliTaskBoard::attach_at(
            task_queue_db_path(),
        )?))
    } else {
        None
    };
    // W2-②: 客户端令牌表 (令牌族第二面) 显式注入; 远程绑定闸与 TLS 接口桩的
    // 清场事实随装配进审计面 (逃生阀可裸奔, 但必须留痕)。
    let mut state =
        apeireth_gateway::build_gateway_state_with_services(Arc::clone(&runtime), services);
    // R3 生产情绪源接线 (兑现 R3 报备的「归口 CLI 接线面」): 网关装配出的
    // presence/PAD 合成器接到发声微调读取面 —— 此后每次发声**现读**合成器
    // 现值, 情绪→语气微调生效; 合成器缺席 (如纯 CLI 形态) = 无情绪数据 =
    // 零微调 (fail-safe 朝原参数)。
    voice_emotion.bind(&state.presence);
    // 心跳关怀常驻宿主接线 (care-heartbeat-producer.md §7 接线清单 (b)):
    // 双闸齐开 (APEIRETH_ENABLE_CARE + APEIRETH_ENABLE_CARE_HEARTBEAT) 才起 ——
    // 闸经 FnInitiativeGate 桥 presence `try_take_initiative` 同源闸注入
    // CareEngine, 发起回调接 Runtime::execute (标准回合照常计费), 心跳真值
    // 已随自述关怀节探针就位 (未起 = 两列显式 null)。开关关 = 零动作零线程。
    // 关怀回合目标会话 = 常驻进程级关怀会话 (普通转写); 间隔非法拒绝即帧。
    let care_config = apeireth_runtime_assembly::CareHeartbeatConfig::from_env()?;
    care_heartbeat.start(
        &care_config,
        Arc::clone(&runtime),
        &state.presence,
        default_panel_data_dir(),
        SessionId::new(),
    )?;
    // W3-②: 用户角色面显式注入 (users.json StoredDoc 拒开 + 令牌↔用户绑定一致性
    // fail-closed); 未配置用户表 = 角色面未激活 (兼容路径, 行为逐字不变)。
    let user_access = apeireth_gateway::UserAccess::load(&data_dir, &client_tokens)
        .map_err(|error| format!("user role face refused to open: {error}"))?;
    state.client_tokens = client_tokens;
    state.user_access = user_access;
    // W3-①: 域上下文路由面注入 (会话面按域路由 / 越域拒绝即帧 / 面闸)。
    state.domains = Some(domain_plane.sessions_plane().clone());
    // W3-③ 星群正身: 逐域伙伴运行时组装 (每户各得其伴) —— 每域自己的伙伴
    // 实例 (该域的会话写闸/记忆库/配额账本), 域 chat 面与域事件面走它;
    // 模型/性格人设覆盖随域文档解析 (缺省回落全局)。一户组装失败 = 该域
    // 拒开 (chat 面 403 domain_refused fail-closed), 不砖化其余域 (审计留痕)。
    let mut domain_instances = std::collections::BTreeMap::new();
    for domain_id in domain_plane.sessions_plane().domain_ids() {
        if domain_id.as_str() == apeireth_core::user_domain::DEFAULT_DOMAIN_ID {
            continue;
        }
        let open_handles = domain_plane
            .sessions_plane()
            .registry()
            .load(&domain_id)
            .ok()
            .and_then(|domain| domain_plane.handles(&domain).ok())
            .map(|handles| (Arc::clone(handles.sessions), handles.memory.path.clone()));
        let Some((domain_sessions, domain_memory_db)) = open_handles else {
            state.observations.buffer_audit(
                "gateway.domain_runtime_skipped",
                Some(format!("domain={domain_id} reason=no_open_data_plane")),
            );
            continue;
        };
        match build_domain_partner_runtime(
            domain_sessions,
            domain_memory_db,
            &voice_emotion,
            &care_heartbeat,
        )
        .await
        {
            Ok(partner) => {
                domain_instances.insert(domain_id, Arc::new(partner));
            }
            Err(error) => {
                state.observations.buffer_audit(
                    "gateway.domain_runtime_skipped",
                    Some(format!(
                        "domain={domain_id} reason=partner_runtime_build_failed detail={:.160}",
                        error
                    )),
                );
            }
        }
    }
    state.domain_runtimes = Some(Arc::new(
        apeireth_runtime::canonical::DomainRuntimePlane::new(
            domain_plane.sessions_plane().clone(),
            Arc::clone(&runtime),
            domain_instances,
        ),
    ));
    state.attach_domain_runtime_events();
    for (event, detail) in bind_clearance.audit_facts {
        state.observations.buffer_audit(&event, detail);
    }
    // TLS 面真接线后审计事件如实: 已启用 = `gateway.tls_enabled`
    // (旧 `gateway.tls_unwired` 只属于已退役的接口桩语义, 不再产生)。
    if let Some(notice) = tls_surface.notice() {
        state
            .observations
            .buffer_audit("gateway.tls_enabled", Some(notice));
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
    // URL 协议名如实: TLS 面启用时是 https, 明文面是 http (不谎报协议)。
    let scheme = if tls_surface.is_enabled() {
        "https"
    } else {
        "http"
    };
    let url = format!("{scheme}://{local_addr}");

    if local_addr.ip().is_loopback() {
        eprintln!("canonical gateway started at {url}");
    } else {
        eprintln!(
            "WARNING: canonical gateway is exposed on non-loopback address {local_addr}; the token gate covers the whole API face (see {token_env})",
            token_env = apeireth_gateway::GatewayTokenGate::ENV
        );
    }
    let served =
        apeireth_gateway::serve_state_with_shutdown(listener, state, graceful_shutdown_signal())
            .await;
    // 优雅停机收尾: 心跳线程随停机收 (join 回收, 不僵尸; 停机后零 poll 零发起)。
    care_heartbeat.shutdown();
    served.map_err(|error| format!("gateway server failed: {error}"))?;

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

/// 生物机制族 W3.2 免疫记忆 → 生产入口挂载（默认关）测试组。
///
/// env 是进程全局：两个 env 用例串在同一把锁后，并用 Drop 守卫逐键保存-恢复
/// （用例 panic 也不残留）；解析用例直接喂 `Option<&str>`，完全不碰进程 env。
#[cfg(test)]
mod immune_memory_switch_tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 单个 env 键的保存-恢复守卫（不依赖外部清场，进程 env 零残留）。
    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let previous = std::env::var(key).ok();
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(value) => std::env::set_var(self.key, value),
                None => std::env::remove_var(self.key),
            }
        }
    }

    /// env 解析（开）：只认显式 `"1"`（既有惯例容忍首尾空白）。
    #[test]
    fn switch_value_accepts_only_explicit_one() {
        assert!(immune_memory_enabled_from_value(Some("1")));
        assert!(
            immune_memory_enabled_from_value(Some(" 1 ")),
            "既有惯例容忍首尾空白，语义仍是显式 \"1\""
        );
    }

    /// env 解析（关）：「0」/空/未设/怪值一律关（fail-closed）。
    #[test]
    fn switch_value_rejects_zero_empty_unset_and_odd_values() {
        assert!(!immune_memory_enabled_from_value(None), "未设必须关");
        for value in ["", " ", "0", "true", "yes", "on", "01", "1.0", "2", "十一"] {
            assert!(
                !immune_memory_enabled_from_value(Some(value)),
                "只认显式 \"1\"，值 {value:?} 必须关 (fail-closed)"
            );
        }
    }

    /// 开关开：hook 挂载成功且 enabled；默认容量 / 默认冷却窗；句柄可取回。
    #[test]
    fn switch_on_mounts_immune_memory_into_the_guard_hook() {
        let guard = attach_immune_memory_if_enabled(BehaviorChainGuardHook::new(), true);
        assert!(guard.immune_memory_enabled(), "开关开必须挂载免疫记忆");
        let shared = guard
            .shared_immune_memory()
            .expect("挂载后必须能取回共享句柄 (与 hook 内观察同一份 Arc)");
        assert_eq!(
            shared.lock().cooldown_ms(),
            apeireth_guard::immune_memory::DEFAULT_COOLDOWN_MS,
            "挂载必须用免疫记忆内核的默认冷却窗"
        );
    }

    /// 默认关：默认构造的 hook 与今天逐字节一致（未挂载、无句柄）；开关关时
    /// 挂载面原样返回。
    #[test]
    fn switch_off_keeps_the_hook_default_and_unmounted() {
        let default_hook = BehaviorChainGuardHook::new();
        assert!(
            !default_hook.immune_memory_enabled(),
            "默认构造的 hook 必须 immune_memory_enabled()==false"
        );
        assert!(
            default_hook.shared_immune_memory().is_none(),
            "默认构造的 hook 不得持免疫记忆句柄"
        );
        let off = attach_immune_memory_if_enabled(BehaviorChainGuardHook::new(), false);
        assert!(!off.immune_memory_enabled(), "开关关必须原样返回 (不挂载)");
        assert!(off.shared_immune_memory().is_none());
    }

    /// 生产入口（真装配路径）：env 开关 `"1"` = 返回的 guard hook 已挂载。
    #[test]
    fn production_entry_mounts_immune_memory_when_env_is_one() {
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _env = EnvVarGuard::set(ENABLE_IMMUNE_MEMORY_ENV, Some("1"));
        let (_, _, guard_hook) = build_production_governance_parts_from_env();
        assert!(
            guard_hook.immune_memory_enabled(),
            "APEIRETH_ENABLE_IMMUNE_MEMORY=1 必须经生产入口挂到 guard hook"
        );
    }

    /// 生产入口默认关证据：未设开关 = 返回的 guard hook 未挂载（fail-closed）。
    #[test]
    fn production_entry_leaves_immune_memory_off_by_default() {
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _env = EnvVarGuard::set(ENABLE_IMMUNE_MEMORY_ENV, None);
        let (_, _, guard_hook) = build_production_governance_parts_from_env();
        assert!(
            !guard_hook.immune_memory_enabled(),
            "未设开关必须默认关 (fail-closed): hook 与今天逐字节一致"
        );
        assert!(
            guard_hook.shared_immune_memory().is_none(),
            "默认关不得持免疫记忆句柄"
        );
    }

    /// 0 装诚实 + 不打凭据：本批新增「解析 + 挂载」区间零日志语句 —— 开关只看
    /// 一个布尔 env，不打印 env 值 / 凭据。源码经 `include_str!` 编译期嵌入，
    /// 新增 `println!/eprintln!/dbg!/tracing::/log::` 即红。
    #[test]
    fn switch_surface_adds_no_logging() {
        const SOURCE: &str = include_str!("lib.rs");
        let region = SOURCE
            .split_once("/// 免疫记忆旋钮取值解析")
            .and_then(|(_, rest)| rest.split_once("fn build_production_governance_parts_from_env"))
            .map(|(region, _)| region)
            .expect("本批新增面必须仍可定位 (标记漂移 = 测试失效，不是通过)");
        for needle in ["println!", "eprintln!", "dbg!", "tracing::", "log::"] {
            assert!(
                !region.contains(needle),
                "免疫记忆开关面不得新增日志输出 (命中 {needle}): 不打 env 值 / 凭据"
            );
        }
    }
}
