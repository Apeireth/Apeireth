//! Canonical builtin tool capabilities.
//!
//! This crate provides the M2A low-risk builtin tools as canonical
//! [`ToolCapability`] implementations wrapped in a single plugin.
//!
//! # Ownership
//!
//! The tools own their identity, input schema, execution implementation, and
//! structured result. They do **not** own the runtime, the gateway, sessions,
//! providers, governance, or the dispatch loop. The runtime reaches them only
//! through the canonical plugin/capability registry path.

#![deny(unsafe_code)]

pub mod apply_patch;
pub mod education;
pub mod egress;
// 工具执行五段流水线: pre-execute 瀑布 → 单调 Guard → around(超时/重试) →
// post-execute 纠错通道 → 输出归一合同; 阶段显式、顺序固定、默认零变化。
pub mod exec_pipeline;
// 沙箱升级阶梯 · 就地提示面: 边界拒绝消息携带结构化升级引导 (缺什么模式 /
// 需要什么理由字段), 在决策点引导, 不让用户去翻设置里的永久开关。
pub mod escalation;
pub mod fetch;
pub mod filesystem;
pub mod guardrail;
pub mod mcp;
// 读前观测门禁: 「未读不得覆盖写」会话期护栏 + 版本 CAS 双钥匙 (纯事件门禁)。
pub mod observed_gate;
pub mod plugin;
pub mod process;
pub mod repo;
pub mod repo_map;
pub mod search;
mod sensitive_path;
pub mod shell;
pub mod spill;
pub mod stealth_crawler;
// P-arch (2026-08-27): B5 process supervisor trait 骨架. 详见 ROADMAP §4 P5.
// v2.0.0-rc.1 RC-8: 加 std_sub_supervisor 模块 (真 impl, std::process::Command 同步启进程).
pub mod std_sub_supervisor;
pub mod supervisor;

pub use apply_patch::{
    ApplyPatchError, FilePatchAction, PatchHunk, PatchReport, TransactionalPatchApplier,
};
pub use education::{DxCheckTool, DxReport, REPLACED_DIFFS};
pub use egress::{ControlledEgress, EgressAllowList, EgressError, EgressPolicy};
pub use escalation::{
    UpgradeHint, ESCALATION_REQUEST_KEY, GRANT_SCOPE, JUSTIFICATION_FIELD, OUT_OF_WORKSPACE_MODE,
};
pub use exec_pipeline::{
    AroundPolicy, CommandFamilyGateHook, ExecutedCall, ExecutionRecord, ExecutionStage,
    GuardRefusal, MonotonicGuards, NormalizationError, OutputSchema, PipelineFailure,
    PipelinedCapability, PostDecision, PostExecuteHook, PostExecuteRequest, PostExecuteWaterfall,
    PostVerdict, PreDecision, PreExecuteHook, PreExecuteRequest, PreExecuteWaterfall, PreVerdict,
    RetryPolicy, RiskLevelGateHook, SchemaField, SchemaKind, StageEntry, SupersededResult,
    ToolExecutionPipeline, ToolGuard, ToolGuardRequest, ToolOutcome,
};
pub use fetch::{FetchConfig, FetchTool};
pub use filesystem::{FilesystemError, FilesystemTool};
pub use guardrail::{LeakedCredentialKind, PreCallGuardError, ToolGuardrail, TripwireScanResult};
pub use mcp::{
    JsonRpcErrorObject, JsonRpcRequest, JsonRpcResponse, McpClient, McpContent, McpError,
    McpToolDescriptor, McpToolResult, McpTransport,
};
pub use observed_gate::{
    gated_write_atomic, gated_write_atomic_durable, ExclusionList, FileVersion, FsVersionProbe,
    GateDenial, GatedWriteError, ObservationState, ObservedGate, VersionProbe, WriteIntent,
    WriteKind, WriteRequest,
};
pub use plugin::{BuiltinToolsOptions, BuiltinToolsPlugin};
pub use repo::{RepoError, RepoTool};
pub use search::{SearchError, SearchTool};
pub use shell::{ShellTool, TrustedShellConfig};
pub use spill::{safe_segment, SpillStore, SPILL_THRESHOLD_CHARS};
pub use stealth_crawler::{ExtractedMediaItem, StealthBrowserConfig, StealthCrawlerEngine};
