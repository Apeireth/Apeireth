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
// 受控文件写入生产工具面 (`tool.apply_patch`): 事务补丁原语 + 读前观测门禁
// 接线 + 写入风险映射 (require-approval 档) + 单调边界 Guard + 五段流水线。
pub mod apply_patch_tool;
// G5 备份恢复 (伙伴的「记忆保险箱」): 备份格式 (目录快照 + manifest.json 逐文件
// SHA-256 摘要) + 导出/恢复/校验引擎 (宁拒不损 + 原子落位 + 版本显式拒) +
// `tool.backup_export` (凭据不随行, 密钥/令牌面显式排除)。
pub mod backup;
// 内容流接入 (J1「内容流接入」): 订阅源管理 (`feeds.json` StoredDoc 拒开) +
// 受控拉取 + 结构化摘要 + 入记忆流 (L2 封套披露, 跨源内容不直进提示词);
// 治理 = RequireApproval 级 (拉取与入库都经审批档, 改订阅走 write_intent 同族)。
pub mod content_feed;
// K 轴 · 物理在场与设备控制 (`device/`, K1 在场感知骨架 + L3/L4 受控执行骨架):
// 设备表 `devices.json` (StoredDoc 拒开, 白名单制默认零设备) + 适配器插槽图
// (mock 已接固定 fixture, 真实协议族留插槽不接) + `tool.device_sense` (摄像头类
// 敏感读面: 显式授权 + 审计) + `tool.weather` (mock 档如实标注数据源 + 提醒
// 信号只交信号) + `device_control` (L3/L4 三拍执行骨架, 另批)。感知族总开关
// `APEIRETH_ENABLE_DEVICE_SENSE` 注册即闸, 默认关 fail-closed。
pub mod device;
// 创作工具面 W2: 文档操作 (`tool.doc_ops`, 受控读写族) —— 闭合操作词表
// (pdf_info/pdf_extract_text/docx_extract_text/docx_outline 读 +
// pdf_merge/pdf_split/pdf_extract_pages/pdf_watermark_text/docx_from_markdown/
// docx_append 写) + 工作区限域 +
// 敏感面拒 + 大小/页数上限 + 创建类原子写 (只创建不覆盖) + 批量管道;
// 注册即闸 `APEIRETH_ENABLE_DOC_OPS`, 写面每次人批, 默认关 fail-closed。
// 依赖报备: flate2/quick-xml 为锁定文件既有传递依赖的同版本复用 (0 新外部包)。
pub mod doc_ops;
// 生物机制族 W2（2026-10-19）：工具生态健康度（多样性/冗余/重叠，纯内核未接线）。
pub mod ecosystem_health;
pub mod education;
pub mod egress;
// 工具执行五段流水线: pre-execute 瀑布 → 单调 Guard → around(超时/重试) →
// post-execute 纠错通道 → 输出归一合同; 阶段显式、顺序固定、默认零变化。
pub mod exec_pipeline;
// 沙箱升级阶梯 · 就地提示面: 边界拒绝消息携带结构化升级引导 (缺什么模式 /
// 需要什么理由字段), 在决策点引导, 不让用户去翻设置里的永久开关。
pub mod escalation;
// V5-b 暴露面体检报告 (安全守望 · 守望的产品化): 「帮你看清你暴露了什么」的
// 本地只读自查面 —— 凭据文件分布/明文风险点 (sensitive_path 族词表) + 开放端口
// 面 (受控系统表快照) + 备份包排除项核对 + 令牌↔用户绑定完整性 (W3-②) + 外网
// 暴露面显式「未测」; 报告 = 分项评分 + 总评, 只读零写 + 零明文留痕;
// `tool.exposure_audit` (只读档) 与 CLI `apeireth exposure report` 同一内核两出口。
pub mod exposure_audit;
pub mod fetch;
pub mod filesystem;
// git 写分级生产工具面 (`tool.git_write`, 干活能力轴 P1): 分级词表
// (L1 可逆/L2 出网/L3 毁灭拒绝即帧) + 词法分级执行面 + 受控通道 + 分级风险映射。
pub mod git_write;
pub mod guardrail;
// 创作工具面 W1: 图像操作 (`tool.image_ops`, 受控文件写族 + 受控读面) ——
// 闭合操作词表 (resize/crop/rotate/flip/format_convert/brightness/contrast/
// grayscale/blur/sharpen/watermark_text) + 工作区限域读写 + 敏感面拒 +
// 炸弹图上限 + 格式白名单 (png/jpeg/webp/bmp) + 多操作管道 + 原子写;
// 输出写归档**文件写入族** (读码定归档, 见模块头), 注册即闸
// `APEIRETH_ENABLE_IMAGE_OPS` + `file_write` 唯一写总闸, 默认关 fail-closed。
pub mod image_ops;
pub mod mcp;
// MCP 工具桥 (最后一公里接线): 服务器配置面 (env / 数据目录 StoredDoc 拒开语义) +
// 每服务器一连接 (生命周期 + 断线重连后重发现) + 动态工具 (mcp:<server>:<tool>,
// 调用穿五段流水线) + 外部工具治理映射 (默认审批级, 只读预设可放行)。
pub mod accountability_view;
pub mod mcp_bridge;
pub mod memory_ledger_view;
// Recon Suite 骨架：Evidence 信封 + 授权 Scope + 平台元数据（原生工具共享底座）。
pub mod recon;
// 外部适配器：nmap / exiftool / tshark / yara / binwalk / radare2 / jadx / apktool /
// ghidra / openssl —— 统一 ProcessExecutor 子进程封装：无 shell、超时与输出上限、Scope 限域。
pub mod external;
// Recon 工具的 Canonical 封装（schema / 限域 / Evidence / 结果归一）。
pub mod recon_tool;
// 对象级访问判定 (E-P3 · 威胁模型 G3): 按"对象"而非"API 形态"判定文件操作权力 ——
// 执行前后对象状态对比 (事实判定, 与形态无关) + 越界即帧 + `sandbox.object_violation`
// 审计 + `ObjectAccessPolicy` 挂载点 (星港线深望智能体接此契约)。
pub mod object_access;
// 读前观测门禁: 「未读不得覆盖写」会话期护栏 + 版本 CAS 双钥匙 (纯事件门禁)。
pub mod observed_gate;
pub mod plugin;
pub mod process;
// 自主度第一小步 · 边角料提案面 (保守版: 它提案, 人点头): `tool.proposal_draft`
// 对发现的边角料 (重复劳动/小错漏/待整理) 起草结构化提案 —— **提案只入提案账**
// (`proposals.jsonl` append-only), 不落任何真面; 人审温柔 (detail 恒含「建议你
// 亲自确认」行 + 审批卡逐条请人过目); 通过 = 账面标记 accepted, **提案不直接
// 执行** (执行仍走既有工具面的正常审批链)。风险档 = 低 (读面 + append 账本),
// 开关 `APEIRETH_ENABLE_PROPOSALS` 注册即闸, 默认关 fail-closed。
pub mod proposal;
pub mod proposal_push;
// 出境脱敏双向闸 (V5-a 安全守望): 占位不可逆指纹 (`__AP_<类>_<指纹12>__`,
// 会话随机密钥 HMAC 截断) + 还原双闸 (只在受控回程 + 只认本会话登记簿) +
// 历史回扫 (每次出境前扫工具轨迹/会话历史, 堵历史搭车) + 脱敏账本
// (redaction.jsonl, 类别/指纹/时间/还原与否, 零真值入账)。
pub mod redaction_bridge;
pub mod repo;
pub mod repo_map;
pub mod search;
// 书→技能沉淀 (P 轴吸收自动化 P1): 技能卡模型 (StoredDoc 落盘 + 拒开语义) +
// 沉淀管线 (`tool.skill_distill`, file_write 总闸族 + 审批档) + 调用面
// (`tool.skill_search` 只读档, L2 封套披露的检索式注入, 绝不全量注入)。
pub mod skill;
// 结构化自述面 (自省通道): `tool.self_status` 只读工具 —— 身份 / 能力名册 /
// 记忆账本计数 / 调参状态 / 预算 / 工作区 / 模型画像事实, 全部取真实生效值;
// 缺失字段显式 null + 原因, 不整帧失败。
pub mod self_status;
mod sensitive_path;
// D26 (2026-10-21): 测试共享 bind 重试助手 (仅 Windows WSAEACCES 10013 有界
// 重试); 集成测试经 `tests/common/mod.rs` 的 `#[path]` 引入同一份源文件。
#[cfg(test)]
pub(crate) mod test_support;
// 创作工具面 K3: 表格操作 (`tool.sheet_ops`, 受控读写族) —— 闭合操作词表
// (read/summary/filter/sort/aggregate/formula 读 + export_csv/export_xlsx 写,
// 变换动作带 output 即写) + 工作区限域 + 敏感面拒 + 行列/字节上限 (防炸弹表) +
// 创建类原子写 (只创建不覆盖) + 数值精度如实 (f64 不静默取整, NaN/inf 零产出);
// 注册即闸 `APEIRETH_ENABLE_SHEET_OPS`, 写面受 file_write 唯一写总闸 + 每次
// 人批, 默认关 fail-closed。读面认 CSV + xlsx (xlsx 读端复用 `doc_ops` 已报备
// 的 ZIP 读端); 写面出 CSV + xlsx (xlsx 写端 = 最小单 sheet, 无公式/多表/样式,
// 不支持面显式拒绝, 不冒充)。
pub mod sheet_ops;
pub mod shell;
pub mod spill;
pub mod stealth_crawler;
// P-arch (2026-08-27): B5 process supervisor trait 骨架. 详见 ROADMAP §4 P5.
// v2.0.0-rc.1 RC-8: 加 std_sub_supervisor 模块 (真 impl, std::process::Command 同步启进程).
pub mod std_sub_supervisor;
pub mod supervisor;
// 通用嗓 TTS 闭环 (V3-b): `tool.voice_speak` 发声面 (合成即播 + 受控播放 +
// 语句账本 spoken.jsonl + 安静纪律), 治理 = 发声面档 (注册即闸, 与 shell
// 的执行档分立)。
pub mod voice;
// 文件写入总闸的 shell 侧写意图词法扫描 (内测整改): 命中且总闸未开 → 拒绝即帧。
pub mod write_intent;
// 干活能力轴 P2 · 只读改单面 (`tool.worktree_plan`): 任务目标 → 改单
// (文件列表+动作+预期), 计划不落盘、只读档零审批。
pub mod worktree_plan;
// 干活能力轴 P2 · 四拍受控工作树写 (`tool.worktree_apply`): 计划→改→验证→落,
// 暂存面执行 + 观测门禁 + 写总闸语义 + 落拍全有或全无 (审批级, 每次人批)。
pub mod worktree_apply;

pub use apply_patch::{
    ApplyPatchError, FilePatchAction, PatchHunk, PatchReport, TransactionalPatchApplier,
};
pub use apply_patch_tool::{
    apply_patch_capability, apply_patch_pipeline, authorized_file_write_policy,
    write_release_class_for_arguments, write_release_class_for_patch, ApplyPatchParams,
    ApplyPatchRiskMappingHook, ApplyPatchTool, ApplyPatchWriteApprovalHook,
    ApplyPatchWriteBoundaryGuard, WriteReleaseClass, APPLY_PATCH_CAPABILITY_ID,
    APPLY_PATCH_TIMEOUT_MS, APPLY_PATCH_TOOL_DESCRIPTION, APPLY_PATCH_TOOL_NAME,
    GIT_WRITE_BOUNDARY_NOTE,
};
pub use backup::{
    backup_status_probe, export_backup, read_backup_ledger, restore_backup, verify_backup,
    BackupAudit, BackupCategory, BackupError, BackupExportConfig, BackupExportTool,
    BackupFileEntry, BackupHooks, BackupItem, BackupLedger, BackupManifest, BackupRestoreReport,
    BackupSources, ExcludedEntry, RestoreOptions, AUDIT_EVENT_EXPORT, AUDIT_EVENT_RESTORE,
    BACKUP_EXPORT_CAPABILITY_ID, BACKUP_FORMAT_VERSION, BACKUP_LEDGER_FILE_NAME,
    CREDENTIAL_ARTIFACTS_EXCLUDED, MANIFEST_FILE_NAME,
};
pub use content_feed::{
    content_feed_approval_reason, content_feed_capability, content_feed_pipeline, episode_for_item,
    ingest_port, item_content_hash, item_identity, item_source, stable_ingest_content,
    write_face_for_action, AttentionLimits, ContentFeedApprovalHook, ContentFeedConfig,
    ContentFeedTool, EpisodeWriter, FeedDocumentForm, FeedHealth, FeedItemSummary, FeedKind,
    FeedLifecycleBody, FeedLifecycleStore, FeedSource, FeedStore, FeedStoreError, FeedWriteFace,
    IngestOutcome, IngestPort, IngestRequest, LifecyclePolicy, LifecycleStats, PullFeedReport,
    PullRecord, ACTION_FEED_ADD, ACTION_FEED_LIST, ACTION_FEED_REMOVE, ACTION_PULL,
    CONTENT_FEED_ACTIONS, CONTENT_FEED_CAPABILITY_ID, CONTENT_FEED_DISCLOSURE_NOTE,
    CONTENT_FEED_EPISODE_ROLE, CONTENT_FEED_SESSION_LABEL, CONTENT_FEED_TOOL_NAME,
    DEFAULT_DAILY_INGESTS, DEFAULT_PER_PULL_ITEMS, FEEDS_DOC_NAME, FEED_LIFECYCLE_FILE_NAME,
    ITEM_LEDGER_MAX,
};
pub use doc_ops::{
    authorized_doc_ops_policy, doc_ops_approval_reason, doc_ops_capability,
    doc_ops_capability_shared, doc_ops_enabled_from_env, doc_ops_params_refusal, doc_ops_pipeline,
    is_doc_ops_write_action, DocOpsApprovalHook, DocOpsBoundaryGuard, DocOpsConfig, DocOpsCounters,
    DocOpsError, DocOpsParams, DocOpsRiskMappingHook, DocOpsRunner, DocOpsTool,
    ACTION_DOCX_EXTRACT_TEXT, ACTION_DOCX_OUTLINE, ACTION_PDF_EXTRACT_PAGES,
    ACTION_PDF_EXTRACT_TEXT, ACTION_PDF_INFO, ACTION_PDF_MERGE, ACTION_PDF_SPLIT,
    ACTION_PDF_WATERMARK_TEXT, DOC_OPS_ACTIONS, DOC_OPS_CAPABILITY_ID, DOC_OPS_ENABLE_ENV,
    DOC_OPS_MAX_INPUT_BYTES, DOC_OPS_MAX_PAGES, DOC_OPS_MAX_TEXT_CHARS, DOC_OPS_READ_ACTIONS,
    DOC_OPS_TIMEOUT_MS, DOC_OPS_TOOL_DESCRIPTION, DOC_OPS_TOOL_NAME, DOC_OPS_WRITE_ACTIONS,
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
    ToolExecutionPipeline, ToolGuard, ToolGuardRequest, ToolOutcome, DEFAULT_MAX_TIMEOUT_MS,
};
pub use exposure_audit::{
    backup_exclusion_item, format_utc_ms, listener_item, parse_netstat_snapshot, probe_netstat,
    run_exposure_audit, run_exposure_audit_at, scan_credential_files, scan_plaintext_points,
    token_binding_item, BindingProbe, BindingSurface, ExposureAuditConfig, ExposureAuditTool,
    ExposureItem, ExposureReport, Finding, ItemStatus, Listener, ListenerProbe, ListenerScope,
    ListenerSnapshot, OverallScore, RiskLevel, EXPOSURE_AUDIT_CAPABILITY_ID,
    EXPOSURE_AUDIT_TOOL_NAME, REPORT_SCHEMA, REPORT_SCHEMA_VERSION,
};
pub use fetch::{FetchConfig, FetchTool};
pub use filesystem::{FilesystemError, FilesystemTool};
pub use git_write::{
    authorized_git_write_policy, classify_git_write, git_write_capability,
    git_write_human_approval_floor, git_write_pipeline, git_write_plan_for_arguments,
    GitWriteBoundaryGuard, GitWriteParams, GitWritePlan, GitWriteReleaseTier,
    GitWriteRiskMappingHook, GitWriteTier, GitWriteTierApprovalHook, GitWriteTool, GradedSegment,
    GIT_WRITE_CAPABILITY_ID, GIT_WRITE_TIMEOUT_MS, GIT_WRITE_TOOL_DESCRIPTION, GIT_WRITE_TOOL_NAME,
    REFUSAL_L3_PREFIX, REFUSAL_MORPH_PREFIX, REFUSAL_OFFLIST_PREFIX,
};
pub use guardrail::{
    EgressSweepResult, LeakedCredentialKind, PreCallGuardError, ToolGuardrail, TripwireScanResult,
};
pub use image_ops::{
    image_ops_enabled_from_env, FlipAxis, ImageFormatKind, ImageOperation, ImageOpsActivity,
    ImageOpsError, ImageOpsParams, ImageOpsReport, ImageOpsTool, OperationSpec, PsdInfoReport,
    IMAGE_OPS_CAPABILITY_ID, IMAGE_OPS_COUNT_SCOPE_NOTE, IMAGE_OPS_ENABLE_ENV, IMAGE_OPS_TOOL_NAME,
    MAX_IMAGE_DIM, MAX_IMAGE_PIXELS, MAX_INPUT_BYTES, MAX_PIPELINE_STEPS, MAX_PSD_LAYER_NAMES,
    MAX_WATERMARK_SCALE, MAX_WATERMARK_TEXT_CHARS, OPERATION_VOCABULARY, SUPPORTED_FORMATS,
};
pub use mcp::{
    JsonRpcErrorObject, JsonRpcRequest, JsonRpcResponse, McpClient, McpContent, McpError,
    McpToolDescriptor, McpToolResult, McpTransport,
};
pub use mcp_bridge::{
    DiscoveredTool, DiscoveryReport, McpBridgeError, McpBridgeOptions, McpCatalogError, McpChannel,
    McpChannelFactory, McpConfigError, McpDynamicTool, McpPermissionMapping, McpRiskMappingHook,
    McpServerConfig, McpServerConnection, McpServerSpec, McpToolBridge, McpToolCatalog,
    McpToolDenyGuard, McpToolEntry, McpToolRegistry, McpTransportKind,
};
pub use object_access::{
    diff_object_states, FsObjectStateProbe, MemoryObjectAuditSink, ObjectAccessJudge,
    ObjectAccessPolicy, ObjectAccessRequest, ObjectAction, ObjectAuditSink, ObjectChange,
    ObjectDomain, ObjectFingerprint, ObjectGuardError, ObjectGuardReport, ObjectKind,
    ObjectPolicyChain, ObjectSnapshot, ObjectStateProbe, ObjectSubject, ObjectVerdict,
    ObjectViolation, ObjectWindow, ObjectZone, WorkspaceObjectPolicy, OBJECT_JUDGE_SOURCE,
    OBJECT_VIOLATION_AUDIT_EVENT,
};
pub use observed_gate::{
    gated_write_atomic, gated_write_atomic_durable, ExclusionList, FileVersion, FsVersionProbe,
    GateDenial, GatedWriteError, ObservationState, ObservedGate, VersionProbe, WriteIntent,
    WriteKind, WriteRequest,
};
pub use plugin::{BuiltinToolsOptions, BuiltinToolsPlugin, ReconConfig};
// 外部适配器与 recon 工具族的根出口（生产接线读这里，不猜模块路径）:
// `external_tool_set` = 10 个 `ext.*`，`recon_tool_set` = 9 个 `tool.recon.*`。
pub use accountability_view::AccountabilityStats;
pub use external::{external_tool_set, ExternalToolsConfig};
pub use proposal::{
    next_proposal_id, normalize_detail, proposal_capability, proposal_draft_refusal, proposal_id,
    proposal_pipeline, proposal_review_refusal, proposals_enabled_from_env, ProposalBoundaryGuard,
    ProposalConfig, ProposalDecision, ProposalDraft, ProposalKind, ProposalLedger,
    ProposalLedgerEntry, ProposalReview, ProposalReviewCard, ProposalState, ProposalTool,
    ACTION_ACCEPT, ACTION_DRAFT, ACTION_LIST, ACTION_REJECT, PROPOSALS_ENABLE_ENV,
    PROPOSAL_CAPABILITY_ID, PROPOSAL_DOC_NAME, PROPOSAL_DOC_VERSION, PROPOSAL_EVIDENCE_HIGHLIGHT,
    PROPOSAL_LEDGER_FILE, PROPOSAL_NO_EXECUTION_NOTE, PROPOSAL_PERSONAL_CONFIRM_LINE,
    PROPOSAL_PERSONAL_CONFIRM_MARKER, PROPOSAL_RECENT_LIMIT, PROPOSAL_REVIEW_TONE,
    PROPOSAL_TIMEOUT_MS, PROPOSAL_TOOL_DESCRIPTION, PROPOSAL_TOOL_NAME,
};
pub use proposal_push::{
    ProposalPushLedger, ProposalPushOutcome, ProposalPushRecord, ProposalPushStats,
    PROPOSAL_PUSH_DOC_NAME, PROPOSAL_PUSH_DOC_VERSION, PROPOSAL_PUSH_LEDGER_FILE,
};
pub use recon_tool::recon_tool_set;
pub use redaction_bridge::{
    EgressRedaction, HistoryRescanReport, LedgerEntry, LedgerEvent, RedactionBridge, RedactionHit,
    RedactionSurface, RestoreReport, REDACTION_LEDGER_FILE,
};
pub use repo::{RepoError, RepoTool};
pub use search::{SearchError, SearchTool};
pub use self_status::{
    capability_roster, derive_tuning_preset, quota_cost_partial_note, render_snapshot,
    BackupStatus, BudgetStatus, CapabilitySwitch, CareStatus, ContentFeedStatus,
    DevicePresenceStatus, DocOpsOpCount, DocOpsStatus, DomainCounts, DomainStatus, ImageOpsOpCount,
    ImageOpsStatus, MemoryLedgerStats, PersonaStatusSection, ProposalPushStatusEntry,
    ProposalStatus, ProposalStatusEntry, QuotaDimensionStatus, QuotaMeterReading, QuotaStatus,
    RetrievalTuningMove, SelfStatusIdentity, SelfStatusSnapshot, SelfStatusSource, SelfStatusTool,
    SessionQuotaProbe, SheetOpsOpCount, SheetOpsStatus, StatusProbe, TaskBoardStatus,
    TraitLockStatus, TraitMoveStatus, TraitStatus, TuningStatus, VoiceStatus, WorkspaceStatus,
    BUDGET_SOURCE_CONSTANT_NOTE, DATA_PROBE_NOT_WIRED, DISPOSITION_PRESETS, MODEL_PROFILE_UNKNOWN,
    PRESET_BALANCED, PRESET_CUSTOM, PRESET_DEEP_MEMORY, PRESET_EFFORTLESS, PRICE_NOT_PROVIDED,
    PRODUCT_NAME, QUOTA_COST_PARTIAL, QUOTA_DEPTH_NOT_WIRED, QUOTA_METER_SCOPE_REQUIRED,
    RUNTIME_ROLE_GATEWAY_SIDECAR, SELF_STATUS_SECTIONS,
};
pub use sheet_ops::{
    authorized_sheet_ops_policy, is_sheet_ops_write_action, sheet_ops_approval_reason,
    sheet_ops_capability, sheet_ops_capability_shared, sheet_ops_enabled_from_env,
    sheet_ops_params_refusal, sheet_ops_pipeline, SheetOpsApprovalHook, SheetOpsBoundaryGuard,
    SheetOpsConfig, SheetOpsCounters, SheetOpsError, SheetOpsParams, SheetOpsRiskMappingHook,
    SheetOpsRunner, SheetOpsTool, ACTION_AGGREGATE, ACTION_EXPORT_CSV, ACTION_FILTER,
    ACTION_FORMULA, ACTION_READ, ACTION_SORT, ACTION_SUMMARY, AGGREGATE_FUNCTIONS,
    DEFAULT_MAX_COLS, DEFAULT_MAX_PREVIEW_ROWS, DEFAULT_MAX_ROWS, FILTER_PREDICATES,
    SHEET_OPS_ACTIONS, SHEET_OPS_CAPABILITY_ID, SHEET_OPS_ENABLE_ENV, SHEET_OPS_READ_ACTIONS,
    SHEET_OPS_TIMEOUT_MS, SHEET_OPS_TOOL_DESCRIPTION, SHEET_OPS_TOOL_NAME,
    SHEET_OPS_TRANSFORM_ACTIONS, SHEET_OPS_WRITE_ACTIONS, SORT_ORDERS,
};
pub use shell::{ShellTool, TrustedShellConfig};
pub use skill::{
    authorized_skill_distill_policy, build_skill_disclosure, distill_into,
    skill_distill_capability, skill_distill_pipeline, skill_search_capability,
    skill_search_pipeline, SkillCard, SkillCardDraft, SkillDistillApprovalHook,
    SkillDistillBoundaryGuard, SkillDistillParams, SkillDistillReport, SkillDistillRiskMappingHook,
    SkillDistillTool, SkillLibrary, SkillLibraryError, SkillRevisionAudit,
    SkillSearchBoundaryGuard, SkillSearchParams, SkillSearchTool, SkillSource, SkillSourceKind,
    SKILL_DISTILL_CAPABILITY_ID, SKILL_DISTILL_TIMEOUT_MS, SKILL_DISTILL_TOOL_DESCRIPTION,
    SKILL_DISTILL_TOOL_NAME, SKILL_SEARCH_CAPABILITY_ID, SKILL_SEARCH_TOOL_DESCRIPTION,
    SKILL_SEARCH_TOOL_NAME,
};
pub use spill::{safe_segment, SpillStore, SPILL_THRESHOLD_CHARS};
pub use stealth_crawler::{ExtractedMediaItem, StealthBrowserConfig, StealthCrawlerEngine};
pub use voice::{
    authorized_voice_speak_policy, system_synth_engine, tune_voice_params, voice_speak_capability,
    voice_speak_pipeline, EmotionBand, EmotionReader, PadReading, PlaybackPolicy, QuietClock,
    QuietHours, SpeakGate, SpeakOutcome, SpeakScope, SpokenLedger, SpokenStatus, SpokenUtterance,
    SynthEngine, TuneDelta, TunedVoiceParams, VoiceSpeakBoundaryGuard, VoiceSpeakConfig,
    VoiceSpeakParams, VoiceSpeakRiskMappingHook, VoiceSpeakTool, DEFAULT_VOICE_LABEL,
    EMOTION_TUNE_TABLE, SPOKEN_LEDGER_FILE, VOICE_SPEAK_CAPABILITY_ID, VOICE_SPEAK_MAX_TEXT_CHARS,
    VOICE_SPEAK_SCOPE_ENV, VOICE_SPEAK_TIMEOUT_MS, VOICE_SPEAK_TOOL_DESCRIPTION,
    VOICE_SPEAK_TOOL_NAME,
};
pub use worktree_apply::{
    apply_boundary_refusal, apply_summary, authorized_worktree_apply_policy,
    cross_check_change_order, prepare_apply, worktree_apply_capability, worktree_apply_pipeline,
    PreparedApply, VerifyCommand, WorktreeApplyApprovalHook, WorktreeApplyBoundaryGuard,
    WorktreeApplyParams, WorktreeApplyReport, WorktreeApplyRiskMappingHook, WorktreeApplyTool,
    REFUSAL_DRIFT_PREFIX, REFUSAL_VERIFY_PREFIX, WORKTREE_APPLY_CAPABILITY_ID,
    WORKTREE_APPLY_TIMEOUT_MS, WORKTREE_APPLY_TOOL_DESCRIPTION, WORKTREE_APPLY_TOOL_NAME,
};
pub use worktree_plan::{
    change_order_refusal, validated_change_order, worktree_plan_capability, worktree_plan_pipeline,
    ActionDistribution, WorktreeChangeAction, WorktreeChangeOrder, WorktreeFileChange,
    WorktreePlanBoundaryGuard, WorktreePlanTool, WORKTREE_GIT_BOUNDARY_NOTE,
    WORKTREE_PLAN_CAPABILITY_ID, WORKTREE_PLAN_TIMEOUT_MS, WORKTREE_PLAN_TOOL_DESCRIPTION,
    WORKTREE_PLAN_TOOL_NAME,
};
pub use write_intent::{scan_shell_write_intent, WriteIntentHit};
