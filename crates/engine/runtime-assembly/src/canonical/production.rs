//! The single production module composition root.
//!
//! Adapters supply concrete backends, tools, and the optional Council. This module
//! owns ordering and slot validation, so CLI, gateway, and future
//! embedding callers cannot accidentally register a second cognitive or tool spine.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use apeireth_core::kernel::Clock;
use apeireth_guard::hook::SharedImmuneMemory;
use apeireth_memory::{
    AccessHistoryActivationSource, ContextWindowManager, EmbeddingProvider, MemoryCoordinator,
    MemoryExtractor, MemoryGovernanceStore, MemoryMaterializer, MemoryMaterializerPort,
    MemoryTypedMaterializationSink, ProactiveRecallPolicy, ScopedMemoryBackend,
    SqliteAccessHistoryStore, TypedMemoryRecallSource, TypedRecallIdentity,
};
use apeireth_orchestration::Council;
use apeireth_plugin::experience::{AssociationStore, KnowledgeGraphStore, WikiEntryStore};
use apeireth_plugin::memory_backend::MemoryBackend;
use apeireth_plugin::preference::PreferenceStore;
use apeireth_plugin::self_assessment::SelfAssessmentStore;
use apeireth_plugin::{Plugin, ToolCapability, ToolModule};
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::{ContextProjectionError, ContextProjector, RuntimeBuilder};
use apeireth_tools_canonical::mcp_bridge::{McpBridgeOptions, McpServerConfig, McpToolBridge};
use apeireth_tools_canonical::{
    CapabilitySwitch, FetchConfig, MemoryLedgerStats, ObservedGate, StatusProbe, TrustedShellConfig,
};

use super::bio_approval_social::BioApprovalSocialSource;
use super::bio_extra_probes::CircadianProbeConfig;
use super::capability::CapabilityProvider;
use super::care_heartbeat::bio_lifecycle::BioLifecycleState;
use super::care_heartbeat::bio_tone::ToneState;
use super::cognitive::{
    CognitiveTelemetry, CouncilModule, JudgeConfig, JudgeModule, JudgeObservations,
    MemoryRecallAccessStore, MemoryRecallModule, MemoryWritebackModule, PreferenceRecallModule,
    SelfAssessmentModule,
};
use super::error::{RuntimeError, RuntimeResult};
use super::memory_typed_sink::CanonicalMemoryTypedSink;
use super::module::Module;
use super::nervous_bridge::NervousBridgeModule;
use super::nervous_bus::NervousBus;
use super::nervous_switchboard::{BioSwitch, NervousSwitchboard};
use super::organ_module::OrganModule;
use super::preference_learning::PreferenceLearningModule;
use super::self_status_source::{
    doc_ops_status_probe, image_ops_status_probe, proposal_status_probe, roster_from_config,
    sheet_ops_status_probe, task_board_status_probe, ProductionSelfStatusSource,
};
use super::tool_modules::{
    ApplyPatchModule, BackupExportModule, CareScheduleModule, ContentFeedModule, DocOpsModule,
    EducationModule, ExposureAuditModule, FetchModule, FilesystemModule, GitWriteModule,
    ImageOpsModule, McpModule, ProcessModule, ProposalModule, ReconModule, RepoModule,
    SearchModule, SelfStatusModule, SheetOpsModule, ShellModule, SkillDistillModule,
    SkillSearchModule, TaskBoardModule, TraitReportModule, VoiceSpeakModule, WorktreeApplyModule,
    WorktreePlanModule,
};
use super::trait_growth::{trait_growth_probe, trait_status_probe, TraitGrowthPaths};
use crate::startup::{StartupPhase, StartupTimeline};

// 生物机制族装配块（W3.2/W3.3/W3.5/W3.6）搬至子模块；`production.rs` 只留调用与字段。
#[path = "bio_production_wiring.rs"]
mod bio_production_wiring;
pub use bio_production_wiring::BioProbeSlots;

/// Adapter that exposes memory's context-window implementation through the
/// runtime-owned projection port. It is kept in assembly so the runtime kernel
/// does not depend on memory or storage.
pub struct MemoryContextProjector {
    manager: ContextWindowManager,
}

impl MemoryContextProjector {
    pub fn new(manager: ContextWindowManager) -> Self {
        Self { manager }
    }
}

impl ContextProjector for MemoryContextProjector {
    fn project(
        &self,
        transcript: &[NormalizedMessage],
        model_context_tokens: Option<u32>,
    ) -> Result<Vec<NormalizedMessage>, ContextProjectionError> {
        let projection = match model_context_tokens {
            // An unknown provider limit must not be treated as a tiny/default
            // budget: preserve the transcript rather than compacting eagerly.
            None => transcript.to_vec(),
            Some(context_tokens) => {
                self.manager
                    .project(transcript, context_tokens as usize)
                    .messages
            }
        };
        Ok(projection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ContentPart;

    #[test]
    fn unknown_context_limit_preserves_transcript() {
        let projector = MemoryContextProjector::new(ContextWindowManager::default());
        let transcript = vec![
            NormalizedMessage::user("older context ".repeat(200)),
            NormalizedMessage::assistant("recent response"),
            NormalizedMessage::user("latest request"),
            NormalizedMessage::assistant("latest answer"),
            NormalizedMessage::user("follow-up"),
        ];
        let projected = projector.project(&transcript, None).unwrap();
        assert_eq!(projected, transcript);
        assert!(ContentPart::join_text(&projected[0].content).contains("older context"));
    }
}

/// Attach the default memory-backed context projector to a runtime builder.
#[must_use]
pub fn with_memory_context_projection(
    builder: RuntimeBuilder,
    manager: ContextWindowManager,
) -> RuntimeBuilder {
    builder.with_context_projector(Arc::new(MemoryContextProjector::new(manager)))
}

/// Feature switches for the production cognitive and tool modules.
/// default when their injected stores exist. Judge, Council, Shell and Fetch are explicitly
/// opt-in; Judge and Council side-calls stay behind the runtime invoker.
#[derive(Debug, Clone)]
pub struct ProductionModulesConfig {
    /// Register memory recall when a memory backend is supplied.
    pub memory_recall: bool,
    /// Opt-in deterministic proactive recall policy; disabled by default.
    pub proactive_recall: Option<ProactiveRecallPolicy>,
    /// Register AfterTurn memory writeback when a memory backend is supplied.
    pub memory_writeback: bool,
    /// Register preference recall when a preference store is supplied.
    pub preference_recall: bool,
    /// Register Judge-backed self-assessment when its store is supplied.
    pub self_assessment: bool,
    /// Enable the AI-evaluates-AI side-call.
    pub judge: JudgeConfig,
    /// Register the no-tool Council adapter.
    pub council: bool,
    /// Register filesystem tool module when workspace root is supplied.
    pub filesystem: bool,
    /// Register search tool module when workspace root is supplied.
    pub search: bool,
    /// Register repository inspection tool module when workspace root is supplied.
    pub repo: bool,
    /// Register shell tool module when config is supplied.
    pub shell: Option<TrustedShellConfig>,
    /// Register fetch tool module when config is supplied.
    pub fetch: Option<FetchConfig>,
    /// Register the life-integration content feed (`tool.content_feed`, J1) when
    /// config is supplied.
    ///
    /// 内容流接入: 订阅源管理 + 受控拉取 + 结构化摘要 + 入记忆流 (L2 封套
    /// 披露, 跨源内容不直进提示词)。治理档位 (RequireApproval 级: 拉取与入库
    /// 都经审批档, 改订阅 = 写文件走 write_intent 同族) 由治理链钩子承载。
    /// 默认关 (opt-in)。
    pub content_feed: Option<apeireth_tools_canonical::ContentFeedConfig>,
    /// Register the proactive-care rule scheduling tool (`tool.care_schedule`,
    /// V3-a 双引擎主动关怀) when the care rules data directory is supplied.
    ///
    /// 关怀规则面: 增删列关怀规则 (四类触发器词表), 写文件面走 write_intent
    /// 同族 (改规则 = 写文件)。治理档位 (RequireApproval 级: 每次调用都停人工
    /// 审批, 无自动放行档) 由治理链钩子承载。关怀发起不走本面: 发起 = 双引擎
    /// 判定 → 标准回合 (零新消息通道)。默认关 (opt-in)。
    pub care_schedule: Option<PathBuf>,
    /// Register the general-voice speech tool (`tool.voice_speak`, V3-b 通用嗓闭环)
    /// when the voice configuration is supplied.
    ///
    /// 发声面: 文本 → 系统语音引擎合成即播 + 语句账本 (`spoken.jsonl`) + 安静
    /// 纪律 (勿扰窗口, 与 care_rules 同源)。治理 = 发声面档 (注册即闸
    /// `APEIRETH_ENABLE_VOICE_SPEAK`, grant 即发声免逐次人批, 与 shell 的执行档
    /// 分立)。默认关 (opt-in)。
    pub voice_speak: Option<apeireth_tools_canonical::VoiceSpeakConfig>,
    /// Register the backup export tool (`tool.backup_export`, G5 备份恢复) when
    /// config is supplied.
    ///
    /// 备份导出面: 全量打包记忆/账本 (七类数据面) + manifest.json 逐文件摘要
    /// (损坏可检出); **凭据不随行** (密钥/令牌面显式排除 + 测试钉死)。恢复/校验
    /// 是 CLI 面 (`apeireth backup restore|verify`, 本地操作), 不走本面。治理
    /// 档位 (RequireApproval 级: 每次导出都停人工审批) 由授权层同源接线。
    /// 默认关 (opt-in)。
    pub backup: Option<apeireth_tools_canonical::backup::BackupExportConfig>,
    /// Register the durable task board (`tool.task_board`, V6-a 任务编排持久队列)
    /// when the task queue is supplied.
    ///
    /// 任务簿工具面: 增/列/改派/完成 over 持久任务队列 (与会话账本同库的任务表,
    /// 崩溃恢复/幂等唤醒/环路熔断/结算接账在 [`apeireth_orchestration::task_queue`])。
    /// 治理档位 (写面 = write_intent 同族: 改任务簿 = 写文件, 每次人批无自动
    /// 放行档; `task_list` 只读零审批) 由治理链
    /// ([`super::task_board_tool::TaskBoardApprovalHook`]) 唯一承载; **无
    /// failed/retry 工具面** (failed 只能人判)。默认关 (opt-in)。
    pub task_board: Option<Arc<apeireth_orchestration::task_queue::TaskQueue>>,
    /// 生物机制族 Wave 4 (SG-14): 上下文预算 (字符) 注入装配面模块;
    /// `None` = 不注入 (默认策略, 行为不变)。与 cli 组装器同一 env 真源。
    pub context_budget_chars: Option<usize>,
    /// Register the external tool bridge (MCP) module.
    ///
    /// When enabled, the server list is loaded at assembly time from
    /// `APEIRETH_MCP_SERVERS` (primary entry) or the data directory's
    /// `mcp-servers.json` (secondary entry, stored-document fail-closed
    /// semantics): a defective configuration is a boot error, never a
    /// silent fallback. Off by default.
    ///
    /// Self-report semantics (honest): the `mcp` roster row reads the
    /// registration condition — the bridge assembled **and** at least one
    /// enabled server entry. Enabled with no server configuration means no
    /// external tools, and the roster says so.
    pub mcp: bool,
    /// Data directory whose `mcp-servers.json` is the secondary config entry
    /// for the external tool bridge (see [`Self::mcp`]).
    pub mcp_data_dir: Option<PathBuf>,
    /// Register the nine native recon tools (`tool.recon.*`, 安全/逆向侦察套件).
    ///
    /// 侦察面 = 「看」不是「做」: 字符串/熵/魔数、ELF/PE/Mach-O、CLR 元数据、
    /// JS 词法、YARA 子集、pcap、TLS 握手 —— 全部只读。网络三件
    /// (`tool.recon.net` / `.web` / `.tls`) 只连 [`apeireth_tools_canonical::recon::Scope`]
    /// 显式授权的目标; 空 scope = 全拒绝 (fail-closed)。
    ///
    /// 注册即闸: 开关 `APEIRETH_ENABLE_RECON=1` 才构造 (默认关), 授权面同源
    /// (同一开关值在治理层 grant)。工作区根缺省时不注册 (文件类工具无落点)。
    pub recon: Option<apeireth_tools_canonical::ReconConfig>,
    /// Register the ten external adapters (`ext.*`: nmap / exiftool / tshark /
    /// yara / binwalk / radare2 / jadx / apktool / ghidra / openssl).
    ///
    /// 外部工具一律**子进程** (argv 数组、无 shell、超时与输出上限、Scope 与
    /// workspace 限域); 系统二进制缺失时该工具返回永久错误, 不影响其余工具,
    /// 启动期不探测 (首次调用才 spawn)。默认关 (opt-in, fail-closed)。
    pub external: Option<apeireth_tools_canonical::ExternalToolsConfig>,
    /// Register deterministic preference learning (AfterTurn, explicit
    /// evidence only). Requires the preference backend when enabled.
    pub preference_learning: bool,
    /// Register the single organ module (post-turn organ cognition).
    ///
    /// Opt-in and off by default: the organ chain includes LLM-heavy W1/W2
    /// side-calls, and this is the explicit rollout boundary for the first
    /// production organ integration.
    pub organs: bool,
    /// Render prompt overlays in the closed-world injection format
    /// (numbered evidence + anti-hallucination rules). Off by default.
    pub memory_injection: bool,
    /// Run the deterministic consolidation report after each turn and persist
    /// its extracted insights. Off by default. Hangs off the memory-writeback
    /// module (`with_consolidation`): inert when [`Self::memory_writeback`] is
    /// off, and the roster reports that registration condition.
    pub consolidation: bool,
    /// V4-a 人格提取步 (默认关): 整编触发点上从近期情节提炼人格增量 →
    /// 人格层合并 (同类合并 / 矛盾标注「人格张力」, 不静默覆盖)。
    ///
    /// 挂在记忆写入模块的整编触发点 (`with_persona_layer`): [`Self::consolidation`]
    /// 关或人格层落库面缺席时显式不跑 (0 装诚实), 名册按实际注册条件取值。
    pub persona_consolidation: bool,
    /// V4-b 散列增量门 (默认关): 整编触发点上先查摄入散列门 —— 整编素材
    /// 同源同散列 = 跳过重复整编 (省算力防重复记忆), 变内容照常重摄。
    ///
    /// 挂在记忆写入模块的整编触发点 (`with_ingest_gate`): [`Self::consolidation`]
    /// 关或散列门落库面缺席时显式不跑 (0 装诚实), 名册按实际注册条件取值。
    pub consolidation_hash_gate: bool,
    /// Register the reflexion failure-feedback module (TurnStart lessons +
    /// AfterTurn judge-failure sedimentation). Off by default.
    pub reflexion: bool,
    /// W2 §4.2 partner 羁绊 (2026-10-10, 默认关): TurnStart 关系注入 + AfterTurn 羁绊演化。
    pub partner_bond: bool,
    /// W2 §4.3 (2026-10-10, 默认关): 查询形态学自适应检索深度 (organ/morphology)。
    /// 挂在记忆召回模块 (`with_morphology_recall`): [`Self::memory_recall`] 关
    /// 时静默失效, 名册按实际注册条件取值。
    pub morphology_recall: bool,
    /// W3 (2026-10-10, 默认关): community 社区分诊接检索前置 (需 graph 后端)。
    /// 挂在记忆召回模块 (`with_community_triage`): [`Self::memory_recall`] 关
    /// 或图谱槽缺席时静默失效, 名册按实际注册条件取值。
    pub community_triage: bool,
    /// W2 §4.3 (2026-10-10, 默认关): education Dx-Check 换元检查工具注册。
    pub education: bool,
    /// W2 §4.4 (2026-10-10, 默认关): 研究吸收批认知体操 (四算法实验性洞察)。
    pub absorption_insight: bool,
    /// Register the structured self-report tool (`tool.self_status`, 自省通道).
    ///
    /// 只读档、默认可用、零审批: 模型可实测自身状态再发言。缺 data 系探测口
    /// 时对应字段显式 null + 原因, 不影响注册与其余字段。
    pub self_status: bool,
    /// Register the controlled file-write tool (`tool.apply_patch`, 第七件).
    ///
    /// 补丁式受控写文件 (创建/修改/删除显式声明), 沿用读前观测门禁。写入风险
    /// 档位默认 require-approval 级 (每次写入都要人批, 与本地审批面板 / IM 审批
    /// 卡同链); 授权层由组装根按同一开关接线。默认关 (opt-in)。
    pub file_write: bool,
    /// 「自动放行已读文件的修改」子档 (依赖 [`Self::file_write`]): 仅修改类
    /// 补丁免逐次审批; 删除/新建永不自动放行 (仍走人工审批)。默认关。
    pub file_write_auto_pass: bool,
    /// Register the graded git-write tool (`tool.git_write`, 干活能力轴 P1)。
    ///
    /// 分级词表: L1 可逆 (add/commit/branch 建·切/tag 本地) / L2 出网
    /// (push/fetch/remote) / L3 毁灭 (无工具面, 拒绝即帧)。L1 默认每次人批
    /// (与本地审批面板 / IM 审批卡同链); L2 **永远人批**。默认关 (opt-in)。
    pub git_write: bool,
    /// 「自动放行 L1」子档 (依赖 [`Self::git_write`]): 仅 L1 可逆本地写免逐次
    /// 审批; **L2 出网永不自动放行** (永远人工审批)。默认关。
    pub git_write_auto_pass_l1: bool,
    /// Register the read-only change-order tool (`tool.worktree_plan`, 干活能力轴 P2 计划拍)。
    ///
    /// 多文件大改「计划→改→验证→落」四拍之首: 任务目标 → 改单 (零落盘,
    /// 只读面); 改单是 [`Self::worktree_apply`] 的输入合同。注册条件 = 本开关
    /// + [`Self::file_write`] (唯一写总闸) + 工作区根已注入 (注册即闸)。
    /// 默认关 (opt-in)。
    pub worktree_plan: bool,
    /// Register the controlled worktree-write tool (`tool.worktree_apply`, 干活能力轴 P2)。
    ///
    /// 四拍受控工作树写 (计划→改→验证→落, 全有或全无); 与 `tool.apply_patch`
    /// 同受 [`Self::file_write`] 唯一写总闸。注册条件 = 本开关 +
    /// [`Self::file_write`] + 工作区根已注入 + 共享读前观测门禁 (`ObservedGate`,
    /// 与读工具同一张观测表 —— 未读不得覆盖/删除)。RequireApproval 级 (每次
    /// 落盘都人批, 无自动放行档)。默认关 (opt-in)。
    pub worktree_apply: bool,
    /// Register the skill distill tool (`tool.skill_distill`, 书→技能沉淀 I1)。
    ///
    /// 知识吸收写入口: 内容输入 (正文 / 文件只读通道) + 模型侧草稿 →
    /// 结构化技能卡 → 技能库 ( [`Self::skills_dir`] )。注册条件 = 本开关 +
    /// [`Self::file_write`] (唯一写总闸, 注册即闸) + 工作区根 + 技能库目录。
    /// **审批档**: 每次沉淀都人批 (无自动放行档)。默认关 (opt-in)。
    pub skill_distill: bool,
    /// Register the read-only skill search tool (`tool.skill_search`, P1 调用面; 原 I1, 审计修 D13 改名)。
    ///
    /// 检索式注入 (触发词命中 / 按需拉取) 经 L2 封套披露进上下文; 只读档
    /// 零审批。注册条件 = 本开关 + 技能库目录 (不依赖写总闸)。默认关 (opt-in)。
    pub skill_search: bool,
    /// 技能库目录 (数据目录 `skills/`, 每卡一 StoredDoc)。
    ///
    /// 缺位时两级技能工具都注册不成 (注册即闸), 名册照实报 false。
    pub skills_dir: Option<PathBuf>,
    /// Register the personality growth report (`tool.trait_report`, 裁定 ⑤)。
    ///
    /// 性格周报: 近 7 天四杆移动量 / 移动原因 top / 锁定态。**只读档零审批**
    /// (与 `tool.self_status` 同档)。数据 = 成长计既有账 (`tuning-log.jsonl`,
    /// 不新造账); 注册条件 = 成长账落位已注入 (注册即闸), 生产装配默认注入 =
    /// 默认可用。自述性格节 (锁定态 + 最近移动) 与本工具同账同源。
    pub trait_report: Option<TraitGrowthPaths>,
    /// Register the exposure audit tool (`tool.exposure_audit`, V5-b 暴露面体检)
    /// when the audit configuration is supplied.
    ///
    /// 暴露面体检面: 「帮你看清你暴露了什么」的本地**只读**自查 (体检不写被
    /// 测面任何字节; 凭据值/密钥值永不出现在报告里, 只报路径词面与计数;
    /// 测不了的显式「未测」不冒充干净)。与 CLI `apeireth exposure report`
    /// 同一内核两个出口。治理 = **只读档** (注册即闸
    /// `APEIRETH_ENABLE_EXPOSURE_AUDIT`, grant 即用零审批, 与
    /// `tool.self_status` 同档)。默认关 (opt-in)。
    pub exposure_audit: Option<apeireth_tools_canonical::ExposureAuditConfig>,
    /// Register the leftover-proposal tool (`tool.proposal_draft`, 自主度第一小步 ·
    /// 边角料提案面) when the proposal-ledger data directory is supplied.
    ///
    /// 提案面 (保守版: 它提案, 人点头): 对发现的边角料起草结构化提案 ——
    /// **提案只入提案账** (`proposals.jsonl` append-only), 不落任何真面;
    /// 人审温柔 (detail 恒含「建议你亲自确认」行 + 审批卡逐条请人过目);
    /// 通过 = 账面标记 accepted, **提案不直接执行** (执行仍走既有工具面的
    /// 正常审批链)。治理 = **低风险档** (读面 + append 账本, 注册即闸
    /// `APEIRETH_ENABLE_PROPOSALS`, 默认关 fail-closed, grant 即用零审批)。
    /// 自述提案节与工具面同一本提案账。默认关 (opt-in)。
    pub proposals: Option<apeireth_tools_canonical::proposal::ProposalConfig>,
    /// Register the image-ops tool (`tool.image_ops`, 创作工具面 W1 · 图像操作).
    ///
    /// 图像操作面: 工作区限域读 → 闭合操作词表管道 (resize/crop/rotate/flip/
    /// format_convert/brightness/contrast/grayscale/blur/sharpen/watermark_text)
    /// → 工作区限域原子写 (临时文件 + 换名, 失败不留半成品)。受控面 = 敏感面拒
    /// + 大小/像素上限 (防炸弹图) + 格式白名单 (png/jpeg/webp/bmp) + 读前观测门禁
    /// (未读不得覆盖写, 与读工具同一张观测表)。输出写**归档文件写入族** (读码定
    /// 归档: 治理层族词表是冻结面, 图像写 = 写文件, 不另立图像写入族)。注册条件
    /// = 本开关 + [`Self::file_write`] (唯一写总闸) + 工作区根 + 共享读前观测门禁
    /// (注册即闸)。默认关 (opt-in, fail-closed)。自述创作节与工具面同一计数器。
    pub image_ops: bool,
    /// Register the doc-ops tool (`tool.doc_ops`, 创作工具面 W2 · 文档操作).
    ///
    /// 受控读写族: PDF 读 (info/extract_text) + DOCX 读 (extract_text/outline) +
    /// PDF 基础写 (merge/split/extract_pages/watermark_text, 批量管道)。受控面 =
    /// 工作区限域 + 敏感面拒 + 大小/页数上限 + 创建类原子写 (只创建不覆盖)。
    /// **写面双重约束**: 写动作受 [`Self::file_write`] 唯一写总闸 (组装根把同一
    /// 开关值注入 [`apeireth_tools_canonical::DocOpsConfig::with_write_gate`],
    /// 关 = 拒写不拒读) + 审批档每次人批 (无自动放行档)。注册条件 = 本开关 +
    /// 工作区根 (注册即闸; 读面不背写总闸, 文档读是本面的第一公民)。默认关
    /// (opt-in, fail-closed)。自述创作节与工具面同一计数器。
    pub doc_ops: Option<apeireth_tools_canonical::DocOpsConfig>,
    /// Register the sheet-ops tool (`tool.sheet_ops`, 创作工具面 K3 · 表格操作).
    ///
    /// 受控读写族: 表读 CSV / xlsx (read/summary) + 筛排聚合 (filter/sort/aggregate) +
    /// 计算列 (formula) + 导出 (export_csv)。受控面 = 工作区限域 + 敏感面拒 +
    /// 行列/字节上限 (防炸弹表) + 创建类原子写 (只创建不覆盖) + 数值精度如实
    /// (f64 不静默取整, NaN/inf 零产出)。**写面双重约束**: 写动作受
    /// [`Self::file_write`] 唯一写总闸 (组装根把同一开关值注入
    /// [`apeireth_tools_canonical::SheetOpsConfig::with_write_gate`], 关 = 拒写
    /// 不拒读) + 审批档每次人批 (无自动放行档)。注册条件 = 本开关 + 工作区根
    /// (注册即闸; 读面不背写总闸, 表格读是本面的第一公民)。默认关 (opt-in,
    /// fail-closed)。自述创作节与工具面同一计数器。读面认 CSV + xlsx
    /// (`.xlsx` 后缀词面判定); 写面只出 CSV (xlsx 写出后置)。
    pub sheet_ops: Option<apeireth_tools_canonical::SheetOpsConfig>,
    /// Register the device-sense tool (`tool.device_sense`, K 轴 L1 在场感知).
    ///
    /// 在场感知面: 设备表 (`devices.json` StoredDoc 拒开, **白名单制默认零设备**)
    /// + 适配器插槽 (mock 已接固定 fixture, 真实协议族留插槽不接) + 摄像头类
    /// 敏感读面 (显式授权 + 审计账)。治理 = **只读档** (注册即闸
    /// `APEIRETH_ENABLE_DEVICE_SENSE`, 默认关 fail-closed, grant 即用零审批)。
    /// 自述在场节与工具面同一本设备表同一进程内感知计数。默认关 (opt-in)。
    pub device_sense: Option<apeireth_tools_canonical::device::sense::DeviceSenseConfig>,
    /// Register the weather tool (`tool.weather`, K 轴 L1/L2 天气感知 + 提醒信号).
    ///
    /// 天气面: 公开天气读数 (mock 档 = 固定 fixture **如实标注数据源**; live 档 =
    /// 受控出口通道 / fetch 族**留插槽不接**) + 提醒信号 (只交信号: 勿扰/频率等
    /// 提醒纪律归提醒侧消费)。治理 = **只读档** (注册即闸, 与 `tool.device_sense`
    /// 同一在场感知族开关 `APEIRETH_ENABLE_DEVICE_SENSE`, 默认关 fail-closed)。
    /// 默认关 (opt-in)。
    pub weather: Option<apeireth_tools_canonical::device::weather::WeatherConfig>,
    /// Register the device-control tool (`tool.device_control`, K 轴 L3/L4 受控执行骨架).
    ///
    /// 设备受控执行面: 三拍执行 (plan → preview → execute, 前两拍零副作用、
    /// 执行拍真执行 + 状态账 `device-actions.jsonl` append-only) + 动作三档分级
    /// (reversible 审批档可配自动档 / irreversible 永逐次人批 / money 永逐次
    /// 人批 + IM 卡) + 设备白名单 (只读消费 L1 [`Self::device_sense`] 同一本
    /// `devices.json`, 未登记/未启用拒绝即帧)。执行面 = mock 适配器 (灯/空调/
    /// 微波炉 fixture, 真实协议族后插)。治理档在治理链
    /// (`DeviceControlApprovalHook` 唯一承载)。注册条件 = 配置注入 (注册即闸,
    /// 双闸 `APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_DEVICE_CONTROL`,
    /// 默认关 fail-closed)。默认关 (opt-in)。
    pub device_control:
        Option<apeireth_tools_canonical::device::device_control::DeviceControlConfig>,
    /// Register the guard-watch tool (`tool.guard_watch`, K 轴 L5 看家骨架).
    ///
    /// 守护面: 守护规则 (`guard-rules.json` StoredDoc 拒开, 规则 = 设备状态阈值 /
    /// 时序条件) + 巡检判定 (白名单设备只读感知) + 告警账 (`guard-alerts.jsonl`
    /// append-only) —— 告警 = 关怀候选高优先 (打扰纪律守护例外档: 破勿扰可配 +
    /// 「守护告警」标注 + 频率独立上限, 提醒侧消费端同源接缝)。**只看不动**:
    /// 告警不触发任何设备动作 (动手仍走 [`Self::device_control`] 三拍人批)。
    /// 注册条件 = 配置注入 (注册即闸, 双闸 `APEIRETH_ENABLE_DEVICE_SENSE` +
    /// `APEIRETH_ENABLE_GUARD_WATCH`, 默认关 fail-closed)。默认关 (opt-in)。
    pub guard_watch: Option<apeireth_tools_canonical::device::guard_watch::GuardWatchConfig>,
}

impl Default for ProductionModulesConfig {
    fn default() -> Self {
        Self {
            memory_recall: true,
            proactive_recall: None,
            memory_writeback: true,
            preference_recall: true,
            self_assessment: true,
            judge: JudgeConfig::default(),
            council: false,
            filesystem: true,
            search: true,
            repo: true,
            shell: None,
            fetch: None,
            content_feed: None,
            care_schedule: None,
            voice_speak: None,
            backup: None,
            task_board: None,
            context_budget_chars: None,
            mcp: false,
            mcp_data_dir: None,
            preference_learning: false,
            organs: false,
            memory_injection: false,
            consolidation: false,
            persona_consolidation: false,
            consolidation_hash_gate: false,
            reflexion: false,
            partner_bond: false,
            morphology_recall: false,
            community_triage: false,
            education: false,
            absorption_insight: false,
            self_status: true,
            file_write: false,
            file_write_auto_pass: false,
            git_write: false,
            git_write_auto_pass_l1: false,
            worktree_plan: false,
            worktree_apply: false,
            skill_distill: false,
            skill_search: false,
            skills_dir: None,
            trait_report: None,
            exposure_audit: None,
            proposals: None,
            image_ops: false,
            doc_ops: None,
            sheet_ops: None,
            device_sense: None,
            weather: None,
            device_control: None,
            guard_watch: None,
            recon: None,
            external: None,
        }
    }
}

/// Compatibility alias for [`ProductionModulesConfig`].
pub type CognitiveModuleConfig = ProductionModulesConfig;

/// Concrete capability handles supplied by an adapter composition root.
///
/// Every field is optional so an embedding caller can choose a deliberate
/// subset.  A requested slot without its backend is a boot-time configuration
/// error, never a silently inert production registration.
#[derive(Default)]
pub struct ProductionBackends {
    /// Episode and history-stream backend.
    pub memory: Option<Arc<dyn MemoryBackend>>,
    /// Memory governance store (active/forgotten status, protection, overrides).
    pub memory_governance: Option<Arc<dyn MemoryGovernanceStore>>,
    /// Optional progressive-disclosure wiki store.
    pub wiki: Option<Arc<dyn WikiEntryStore>>,
    /// Optional knowledge graph store.
    pub graph: Option<Arc<dyn KnowledgeGraphStore>>,
    /// Optional association store.
    pub associations: Option<Arc<dyn AssociationStore>>,
    /// User preference store.
    pub preferences: Option<Arc<dyn PreferenceStore>>,
    /// Self-assessment store.
    pub self_assessments: Option<Arc<dyn SelfAssessmentStore>>,
    /// Council service, supplied only when the adapter explicitly enables it.
    pub council: Option<Arc<Council>>,
    /// Workspace root directory for local file tools.
    pub workspace_root: Option<PathBuf>,
    /// Scoped memory backend for cross-session storage queries.
    pub scoped_memory: Option<Arc<dyn ScopedMemoryBackend>>,
    /// Embedding provider for semantic memory retrieval.
    pub embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    /// Optional durable selected-context access history.
    pub access_history: Option<Arc<SqliteAccessHistoryStore>>,
    /// Optional unified materializer for bounded AfterTurn memory writeback.
    pub memory_materializer: Option<Arc<dyn MemoryMaterializerPort>>,
    /// Optional unified extractor used to build a default materializer.
    pub memory_extractor: Option<Arc<dyn MemoryExtractor>>,
    /// Optional typed durable candidate source for unified recall.
    pub typed_recall: Option<Arc<dyn TypedMemoryRecallSource>>,
    /// Explicit identity paired with the typed recall source.
    pub typed_recall_identity: Option<TypedRecallIdentity>,
    /// Optional concrete typed durable sink for commitment/persona/relation projections.
    pub typed_sink: Option<Arc<dyn MemoryTypedMaterializationSink>>,
    /// V4-a 人格记忆层落库面 (人格提取步的 `persona_memory_*` 分表).
    /// 缺位 = 人格提取步显式不跑 (默认未接线)。
    pub persona_layer: Option<Arc<dyn apeireth_memory::PersonaMemoryStore>>,
    /// V4-b 保鲜回路散列增量门落库面 (散列账 `source_hash` 与记忆库同库).
    /// 缺位 = 整编散列门显式不跑 (默认未接线)。
    pub ingest_gate: Option<Arc<dyn apeireth_memory::IngestGate>>,
    /// Reflexion failure-feedback store (2026-10-06 W2 记忆闭环批).
    pub reflexion_store: Option<Arc<dyn apeireth_memory::reflexion::ReflexionStore>>,
    /// W2 §4.2: partner 羁绊存储 (InMemoryPartnerStore 或未来 sqlite 实现)。
    pub partner_store: Option<Arc<dyn apeireth_memory::partner::PartnerStore>>,
    /// 「性格养成」第一铲 (默认无 = 自学习关): 自校准接线层, 接上后
    /// MemoryRecallModule 的召回结果作为真接信号喂给引擎。
    pub self_tuning: Option<Arc<crate::canonical::self_tuning_wire::SelfTuningWire>>,
    /// V4-c 检索维度自进化 (默认无 = 调权关): 自进化接线层, 接上后
    /// MemoryRecallModule 的注入回执作为真接信号面喂给引擎 (证据裁决 +
    /// `retrieval_tuning.jsonl` 自进化账落盘 + 调权生效值落地)。
    pub retrieval_tuning: Option<Arc<crate::canonical::retrieval_tuning_wire::RetrievalTuningWire>>,
    /// 自省通道: 记忆账本计数探测口 (会话/记忆/保护/教训计数, 只回计数)。
    pub self_status_ledger: Option<StatusProbe<MemoryLedgerStats>>,
    /// 自省通道: **问责计数**探测口 (影响声明 / 后果三态 / 待补救, 只回计数)。
    ///
    /// ADR-0014 §4 指定的「自述面新增只读子对象」。`None` = 不挂载 ⇒
    /// `capabilities.accountability` 子对象省略（旧帧形状不变）。
    pub self_status_accountability:
        Option<StatusProbe<apeireth_tools_canonical::AccountabilityStats>>,
    /// 自省通道: 凭据存在性探测口 (只回布尔, 不回显凭据本体)。
    pub self_status_credentials: Option<StatusProbe<bool>>,
    /// 自省通道: 组装根补充的能力名册行 (例如授权层的 `local_read_tools` 旋钮)。
    pub self_status_extras: Vec<CapabilitySwitch>,
    /// 自省通道: 生效中的预算一节 (可配置旋钮取值; 缺位回退编译期常量)。
    pub self_status_budget: Option<apeireth_tools_canonical::BudgetStatus>,
    /// 自省通道: 会话配额计量探测口 (按会话 id 读已耗真值, 只回数字)。
    pub self_status_quota: Option<apeireth_tools_canonical::SessionQuotaProbe>,
    /// 自省通道: 生效模型的画像 (H-P1 模型画像表解析结果; 未知模型传显式
    /// unknown 行; 缺位时模型节显式 null + 原因)。
    pub self_status_model_profile: Option<apeireth_core::model_profile::ModelProfile>,
    /// 自省通道: 成本分级路由节探测口 (H-P2: 主开关 + 最近一次路由决策真值,
    /// 与任务选型同一台路由器同源现读; 缺位时路由节显式 null + 原因)。
    pub self_status_cost_routing:
        Option<StatusProbe<apeireth_core::cost_router::CostRoutingStatus>>,
    /// 自省通道: 用户域节探测口 (W3-①: 当前域 id/角色/域内计数, 现探测如实)。
    pub self_status_domain: Option<StatusProbe<apeireth_tools_canonical::DomainStatus>>,
    /// 自省通道: 内容流面探测口 (J1: 订阅数/最近拉取, 现探测如实)。
    pub self_status_content_feed: Option<StatusProbe<apeireth_tools_canonical::ContentFeedStatus>>,
    /// 自省通道: 双引擎关怀节探测口 (V3-a: 规则数/下次触发/最近关怀真值, 现探测如实)。
    pub self_status_care: Option<StatusProbe<apeireth_tools_canonical::CareStatus>>,
    /// 自省通道: 备份节探测口 (G5: 最近备份时间/清单件数真值或 null, 现探测如实)。
    pub self_status_backup: Option<StatusProbe<apeireth_tools_canonical::BackupStatus>>,
    /// 自省通道: 语音节探测口 (V3-b: 账本数/最近说话/嗓名, 现探测如实;
    /// 与发声工具同一本语句账本 `spoken.jsonl`)。
    pub self_status_voice: Option<StatusProbe<apeireth_tools_canonical::VoiceStatus>>,
    /// 自省通道: 人格节探测口 (`self_status_persona`, V4-a: 信念/边界/偏好/
    /// 张力计数 + 最后整编真值, 现探测如实; 与人格层落库面同账, 只计数不回
    /// 内容)。
    pub self_status_persona: Option<StatusProbe<apeireth_tools_canonical::PersonaStatusSection>>,
}
/// Compatibility alias for [`ProductionBackends`].
pub type CognitiveBackends = ProductionBackends;

fn collect_tool_module(
    tool_modules: &mut Vec<Arc<dyn Plugin>>,
    capabilities: &mut Vec<Arc<dyn ToolCapability>>,
    provider: &dyn CapabilityProvider,
) -> RuntimeResult<()> {
    let module = provider
        .tool_module()
        .map_err(RuntimeError::misconfigured)?;
    capabilities.extend(module.tools());
    if !module.tools().is_empty() {
        tool_modules.push(Arc::new(module));
    }
    Ok(())
}

/// The validated, ordered module set to pass to [`RuntimeBuilder::with_module`].
pub struct ProductionModules {
    modules: Vec<Arc<dyn Module>>,
    tool_modules: Vec<Arc<dyn Plugin>>,
    capabilities: Vec<Arc<dyn ToolCapability>>,
    telemetry: Arc<CognitiveTelemetry>,
    mcp_bridge: Option<Arc<McpToolBridge>>,
    mcp_module: Option<Arc<McpModule>>,
    /// 器官模块句柄 (`config.organs` 开才 `Some`), 供组合根迟绑定 guard 免疫记忆.
    organ_module: Option<Arc<OrganModule>>,
    /// 生物机制族 W3.3: 调质状态句柄 (开关开才 `Some`, 与注册模块共享同一 `Arc`)。
    bio_tone_state: Option<Arc<ToneState>>,
    /// 生物机制族 W3.5 (SG-18): 生命周期状态句柄 (开关开才 `Some`, 与注册模块同一 `Arc`)。
    bio_lifecycle_state: Option<Arc<BioLifecycleState>>,
    /// 生物机制族 W3.6: 自述 probe 迟绑定共享槽 (开关关 = 槽不武装, 零读取).
    bio_probe_slots: BioProbeSlots,
    /// Wave 4 / ADR-0009 R1b-1a: 神经信号总线句柄 (适配器只 publish; 与注册的桥
    /// 共享同一 `Arc`)。全关时照常构造, 但没有桥/订阅者 = 零投递。
    nervous_bus: Arc<NervousBus>,
    /// Wave 4 / ADR-0009 R1b-1a: 本次装配生效的开关快照 (一次解析, 诊断读).
    nervous_switchboard: NervousSwitchboard,
}

/// Compatibility alias for [`ProductionModules`].
pub use ProductionModules as ProductionCognitiveModules;

/// 模块族 (懒加载断言单位): 一族 = 一个开关族在 [`ProductionModules::build`] 里的构造点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModuleFamily {
    /// 文件读工具族 (`filesystem`)。
    Filesystem,
    /// 搜索工具族 (`search`)。
    Search,
    /// 仓库检视工具族 (`repo`)。
    Repo,
    /// 受控写文件族 (`file_write`)。
    ApplyPatch,
    /// 工作树改单族 (`worktree_plan`)。
    WorktreePlan,
    /// 工作树落盘族 (`worktree_apply`)。
    WorktreeApply,
    /// git 写族 (`git_write`)。
    GitWrite,
    /// 技能沉淀族 (`skill_distill`)。
    SkillDistill,
    /// 技能检索族 (`skill_search`)。
    SkillSearch,
    /// Dx-Check 检查族 (`education`)。
    Education,
    /// 受控 shell 族 (`shell`)。
    Shell,
    /// 受控抓取族 (`fetch`)。
    Fetch,
    /// 内容流族 (`content_feed`)。
    ContentFeed,
    /// 关怀规则族 (`care_schedule`)。
    CareSchedule,
    /// 备份导出族 (`backup`)。
    BackupExport,
    /// 发声族 (`voice_speak`)。
    VoiceSpeak,
    /// 任务簿族 (`task_board`)。
    TaskBoard,
    /// 性格周报族 (`trait_report`)。
    TraitReport,
    /// 暴露面体检族 (`exposure_audit`)。
    ExposureAudit,
    /// 创作工具面 W1 · 图像操作族 (`image_ops`)。
    ImageOps,
    /// 创作工具面 W2 · 文档操作族 (`doc_ops`)。
    DocOps,
    /// 创作工具面 K3 · 表格操作族 (`sheet_ops`)。
    SheetOps,
    /// K 轴 L1 在场感知族 (`device_sense`)。
    DeviceSense,
    /// K 轴 L1 天气感知族 (`weather`)。
    Weather,
    /// K 轴 L3/L4 受控执行族 (`device_control`)。
    DeviceControl,
    /// K 轴 L5 看家骨架族 (`guard_watch`)。
    GuardWatch,
    /// 生物机制族 W3.3 · 游戏练习场工具族 (`play_*`, 默认关): 四个工具模块各自
    /// 一个构造点 (propose/start/finish/abort, 共享同一本内存账本);
    /// env `APEIRETH_ENABLE_BIO_PLAY` 只认 `"1"` 且工作区根注入才构造。
    BioPlay,
    /// 自省通道族 (`self_status`)。
    SelfStatus,
    /// 安全/逆向侦察族 (`tool.recon.*`, 9 件, 默认关)。
    Recon,
    /// 外部适配器族 (`ext.*`, 10 件, 默认关; 子进程封装)。
    ExternalTools,
    /// 外部工具桥族 (`mcp`)。
    Mcp,
    /// 记忆协调器 (memory_recall / memory_writeback 共用)。
    MemoryCoordinator,
    /// 记忆召回族 (`memory_recall`)。
    MemoryRecall,
    /// 偏好召回族 (`preference_recall`)。
    PreferenceRecall,
    /// 偏好学习族 (`preference_learning`)。
    PreferenceLearning,
    /// 评判族 (`judge`)。
    Judge,
    /// 自评族 (`self_assessment`)。
    SelfAssessment,
    /// 议事族 (`council`)。
    Council,
    /// 器官网族 (`organs`)。
    Organs,
    /// 反思族 (`reflexion`)。
    Reflexion,
    /// 吸收洞察族 (`absorption_insight`)。
    AbsorptionInsight,
    /// 羁绊族 (`partner_bond`)。
    PartnerBond,
    /// 恒定人格注入族 (人格层槽)。
    PersonaConstantInjection,
    /// 记忆写回族 (`memory_writeback`)。
    MemoryWriteback,
}

/// 构造探针 (B-P1 懒加载断言口): 族构造点上报一次。
///
/// 「开关关 = 不构造」由构造点上报钉住: 未启用族的构造被短路时探针零上报,
/// 测试即可断言**零构造** (构造点上报, 不是注册点上报)。上报在构造调用紧前:
/// 组装成功的语义下上报 = 构造发生 (个别族构造含前置校验, 校验失败整机组装
/// 拒绝, 计数不成账)。
pub trait ModuleBuildProbe: Send + Sync {
    /// 某族的构造发生了一次。
    fn constructed(&self, family: ModuleFamily);
}

/// 计数构造探针 (测试/诊断用): 按族计构造次数。
#[derive(Default)]
pub struct CountingBuildProbe {
    counts: Mutex<BTreeMap<ModuleFamily, u32>>,
}

impl CountingBuildProbe {
    /// 构造计数探针。
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// 某族的构造次数 (未构造 = 0)。
    pub fn count(&self, family: ModuleFamily) -> u32 {
        *self
            .counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&family)
            .unwrap_or(&0)
    }

    /// 全部族的构造总次数。
    pub fn total(&self) -> u32 {
        self.counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .sum()
    }
}

impl ModuleBuildProbe for CountingBuildProbe {
    fn constructed(&self, family: ModuleFamily) {
        *self
            .counts
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .entry(family)
            .or_insert(0) += 1;
    }
}

/// 组装期插桩 (B-P1): 构造探针 + 分段计时, 生产传 [`BuildInstrumentation::default`]
/// = 全部无操作, 装配行为与不插桩完全一致。
#[derive(Default)]
pub struct BuildInstrumentation {
    /// 构造探针 (懒加载零构造断言口)。
    pub probe: Option<Arc<dyn ModuleBuildProbe>>,
    /// 分段计时记录面 (模块构造段落账)。
    pub timeline: Option<Arc<StartupTimeline>>,
}

impl BuildInstrumentation {
    /// 组装期插桩: 构造探针。
    #[must_use]
    pub fn with_probe(mut self, probe: Arc<dyn ModuleBuildProbe>) -> Self {
        self.probe = Some(probe);
        self
    }

    /// 组装期插桩: 分段计时记录面。
    #[must_use]
    pub fn with_timeline(mut self, timeline: Arc<StartupTimeline>) -> Self {
        self.timeline = Some(timeline);
        self
    }

    fn constructed(&self, family: ModuleFamily) {
        if let Some(probe) = &self.probe {
            probe.constructed(family);
        }
    }
}

impl ProductionModules {
    /// Build the canonical registration order.
    pub fn build(
        config: ProductionModulesConfig,
        backends: ProductionBackends,
        clock: Arc<dyn Clock>,
    ) -> RuntimeResult<Self> {
        Self::build_instrumented(config, backends, clock, &BuildInstrumentation::default())
    }

    /// [`Self::build`] 的可插桩形态 (B-P1): 同一装配, 额外接受构造探针与
    /// 分段计时 (不插桩时与 [`Self::build`] 逐字节同行为)。
    pub fn build_instrumented(
        config: ProductionModulesConfig,
        backends: ProductionBackends,
        clock: Arc<dyn Clock>,
        instrument: &BuildInstrumentation,
    ) -> RuntimeResult<Self> {
        match &instrument.timeline {
            Some(timeline) => timeline.measure(StartupPhase::ModulesBuild, || {
                Self::build_inner(config, backends, clock, instrument)
            }),
            None => Self::build_inner(config, backends, clock, instrument),
        }
    }

    fn build_inner(
        config: ProductionModulesConfig,
        backends: ProductionBackends,
        clock: Arc<dyn Clock>,
        instrument: &BuildInstrumentation,
    ) -> RuntimeResult<Self> {
        let mut modules: Vec<Arc<dyn Module>> = Vec::new();
        // Wave 4 / ADR-0009 R1b-1a: 神经信号总线 + 开关表一次解析 (9 开关 + 1
        // 参数, fail-closed)。全关 = 不注册桥 = 零行为变化; 任一开关开 = 桥在
        // 模块表首位注册, 这是唯一的 HookPoint→信号映射点。
        let nervous_bus = Arc::new(NervousBus::new());
        let nervous_switchboard = NervousSwitchboard::from_env().map_err(|error| {
            RuntimeError::misconfigured(format!("nervous switchboard refused to load: {error}"))
        })?;
        if nervous_switchboard.any_enabled() {
            modules.push(
                Arc::new(NervousBridgeModule::new(nervous_bus.as_ref().clone())) as Arc<dyn Module>,
            );
        }
        let mut capabilities: Vec<Arc<dyn ToolCapability>> = Vec::new();
        let mut tool_modules: Vec<Arc<dyn Plugin>> = Vec::new();
        // W3.3: 器官模块句柄留给组合根迟绑定 guard 免疫记忆 (默认 None = 未注册).
        let mut organ_module: Option<Arc<OrganModule>> = None;
        // W3.6: 自述 probe 迟绑定槽 (安装点按开关武装; cli/组合根后续填句柄).
        let bio_probe_slots = BioProbeSlots::new();
        let observations = Arc::new(JudgeObservations::default());
        let telemetry = Arc::new(CognitiveTelemetry::default());

        // External tool bridge (MCP) server list is loaded BEFORE the roster
        // projection: the self-report must read the same facts the
        // registration uses (已启用但无服务器配置 = 无外部工具). Fail-closed
        // semantics unchanged: a defective configuration is a boot error,
        // never a silent fallback.
        let mcp_server_config = if config.mcp {
            Some(
                McpServerConfig::load(config.mcp_data_dir.as_deref()).map_err(|error| {
                    RuntimeError::misconfigured(format!(
                        "mcp server configuration refused to load: {error}"
                    ))
                })?,
            )
        } else {
            None
        };

        // 自省通道: 能力名册在任何字段移动之前投影 —— 名册即**实际用于装配的**
        // config 生效值; 注入槽级的注册条件 (图谱槽 / 外部工具服务器面) 先折进
        // 生效视图, 名册不复述 "配置说开"而注册没成的事实。
        let mut roster_config = config.clone();
        roster_config.community_triage = config.community_triage && backends.graph.is_some();
        roster_config.mcp = mcp_server_config
            .as_ref()
            .is_some_and(|servers| servers.enabled_servers().next().is_some());
        // 自学习 / 类型化召回 / 检索调权的生效值 = 接线事实 AND 上游槽 (自学习
        // 信号只走记忆召回模块, 类型化召回候选只走记忆协调器, 检索调权的注入
        // 回执面只走记忆召回模块)。
        let self_tuning_effective = backends.self_tuning.is_some() && config.memory_recall;
        let retrieval_tuning_effective =
            backends.retrieval_tuning.is_some() && config.memory_recall;
        let typed_recall_effective =
            backends.typed_recall.is_some() && (config.memory_recall || config.memory_writeback);
        // V4-a 人格提取步的生效值 = 开关 AND 人格层落库面 (面缺席 = 显式不跑).
        roster_config.persona_consolidation =
            config.persona_consolidation && backends.persona_layer.is_some();
        // V4-b 散列增量门的生效值 = 开关 AND 散列门落库面 (面缺席 = 显式不跑)。
        roster_config.consolidation_hash_gate =
            config.consolidation_hash_gate && backends.ingest_gate.is_some();
        let self_status_roster = config.self_status.then(|| {
            roster_from_config(
                &roster_config,
                backends.workspace_root.is_some(),
                self_tuning_effective,
                retrieval_tuning_effective,
                typed_recall_effective,
                &backends.self_status_extras,
            )
        });

        let experience_count = [
            backends.wiki.is_some(),
            backends.graph.is_some(),
            backends.associations.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();
        if experience_count != 0 && experience_count != 3 {
            return Err(RuntimeError::misconfigured(
                "Experience wiring must supply Wiki, knowledge graph, and association stores together",
            ));
        }

        // Register tool capabilities independently of behavior modules.
        // 读前观测门禁一张表共享: 读工具记录的观测就是写入端「已读」的证据
        // (跨工具同一会话观测, 未读不得覆盖写)。
        let observed_gate = Arc::new(ObservedGate::new());
        if config.filesystem {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Filesystem);
                let provider =
                    FilesystemModule::new_with_gate(root.clone(), Arc::clone(&observed_gate));
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        if config.search {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Search);
                let provider = SearchModule::new(root.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        if config.repo {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Repo);
                let provider = RepoModule::new(root.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 受控文件写入 (`tool.apply_patch`, 第七件生产工具, opt-in): 补丁式
        // 创建/修改/删除显式声明 + 读前观测门禁; 写入风险档位默认 require-approval
        // 级 (每次写入都要人批), 授权层按同一开关接线。git 写边界 (分级语义):
        // 仓库写操作走 `tool.git_write` 的分级词表 (L1 默认人批 / L2 永远人批 /
        // L3 毁灭无工具面)。
        if config.file_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::ApplyPatch);
                let provider = ApplyPatchModule::new(root.clone(), Arc::clone(&observed_gate));
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 干活能力轴 P2 (「计划→改→验证→落」四拍受控工作树写, opt-in): 两级
        // 工具面注册条件 = 开关 + `file_write` 唯一写总闸 + 工作区根 (注册即闸,
        // 与 `tool.apply_patch` 同受一闸)。`worktree_plan` 是只读改单面 (零落盘,
        // 其产物是 apply 的输入合同); `worktree_apply` 走**共享**读前观测门禁
        // (与读工具同一张观测表, 未读不得覆盖/删除)。审批级: apply 每次落盘
        // 都人批 (无自动放行档, 审批卡载荷 = 改单摘要), plan 只读零审批。
        if config.worktree_plan && config.file_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::WorktreePlan);
                let provider = WorktreePlanModule::new(root.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        if config.worktree_apply && config.file_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::WorktreeApply);
                let provider = WorktreeApplyModule::new(root.clone(), Arc::clone(&observed_gate));
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // git 写分级 (`tool.git_write`, 干活能力轴 P1, opt-in): 分级词表 +
        // 词法分级执行面 + 受控通道 (工作区限域/超时/输出归一)。L1 默认人批
        // (设置可开「自动放行 L1」档), L2 永远人批, L3 拒绝即帧无工具面。
        if config.git_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::GitWrite);
                let provider = GitWriteModule::new(root.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 书→技能沉淀 (P 轴吸收自动化 P1, 默认关): 两级工具面。`tool.skill_distill`
        // 是写文件面 —— 注册条件 = 开关 + `file_write` 唯一写总闸 + 工作区根 +
        // 技能库目录 (注册即闸, 与 `tool.apply_patch` 同受一闸), 审批档 = 每次
        // 沉淀都人批 (无自动放行档, 审批卡载荷 = 技能卡摘要); `tool.skill_search`
        // 是只读档 (零审批), 只要技能库目录。技能库 = 数据目录 `skills/`,
        // 每卡一 StoredDoc (拒开语义)。
        if config.skill_distill && config.file_write {
            if let (Some(root), Some(skills_dir)) = (&backends.workspace_root, &config.skills_dir) {
                instrument.constructed(ModuleFamily::SkillDistill);
                let provider = SkillDistillModule::new(root.clone(), skills_dir.clone());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        if config.skill_search {
            if let Some(skills_dir) = &config.skills_dir {
                instrument.constructed(ModuleFamily::SkillSearch);
                let provider = SkillSearchModule::new(
                    skills_dir.clone(),
                    apeireth_tools_canonical::skill::DEFAULT_SKILL_DISCLOSURE_BUDGET_CHARS,
                );
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // W2 §4.3 (2026-10-10, 默认关): education Dx-Check 工具 (纯确定性, 0 副作用)。
        if config.education {
            instrument.constructed(ModuleFamily::Education);
            let provider = EducationModule::new();
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 文件写入总闸 (宪法级, 内测整改): `file_write` 是**唯一写总闸** ——
        // 组装根把同一开关值注入 shell 配置, shell 写命令 (重定向/删除类/写
        // 命令面) 与 apply_patch 同受此闸, 开关名与权力相符。关 = 写意图拒绝
        // 即帧 (`pipeline.pre_deny`); 开 = shell 行为不变 (审批链照旧走既有
        // 风险映射)。
        if let Some(shell_config) = config.shell {
            instrument.constructed(ModuleFamily::Shell);
            let provider = ShellModule::new(shell_config.with_file_write(config.file_write));
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        if let Some(fetch_config) = config.fetch {
            instrument.constructed(ModuleFamily::Fetch);
            let provider = FetchModule::new(fetch_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 内容流接入 (`tool.content_feed`, J1, opt-in): 订阅源管理 + 受控拉取 +
        // 结构化摘要 + 入记忆流 (L2 封套披露)。治理档位在治理链
        // (`ContentFeedApprovalHook`): RequireApproval 级 —— 拉取与入库都经
        // 审批档, 改订阅走 write_intent 同族 (改订阅 = 写文件)。
        if let Some(feed_config) = config.content_feed {
            instrument.constructed(ModuleFamily::ContentFeed);
            let provider = ContentFeedModule::new(feed_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 双引擎主动关怀 (`tool.care_schedule`, V3-a, opt-in): 关怀规则面
        // (增删列, 四类触发器词表)。治理档位在治理链 (`CareScheduleApprovalHook`):
        // RequireApproval 级 —— 改规则 = 写文件 (write_intent 同族), 每次调用
        // 都停人工审批。
        // W3.6: 关怀数据目录同时作为生命周期账本持久化目录（开关关 = 纯内存，
        // 不建文件；目录缺省 = 不持久化），故先克隆再按需搬迁。
        let bio_lifecycle_store_dir = config.care_schedule.clone();
        if let Some(care_dir) = config.care_schedule {
            instrument.constructed(ModuleFamily::CareSchedule);
            let provider = CareScheduleModule::new(care_dir);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 备份导出 (`tool.backup_export`, G5 备份恢复, opt-in): 全量打包 +
        // manifest.json 逐文件摘要 (损坏可检出), 凭据不随行。治理档位在授权层
        // 同源接线 (RequireApproval 级: 每次导出都停人工审批)。
        if let Some(backup_config) = config.backup {
            instrument.constructed(ModuleFamily::BackupExport);
            let provider = BackupExportModule::new(backup_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 通用嗓发声 (`tool.voice_speak`, V3-b, opt-in): 文本 → 系统语音引擎
        // 合成即播 + 语句账本 (`spoken.jsonl`) + 安静纪律 (勿扰窗口, 与
        // care_rules 同源)。治理 = 发声面档 (注册即闸, grant 即发声免逐次
        // 人批, 与 shell 的执行档分立)。合成执行面 = 系统引擎桥 (可注入)。
        if let Some(voice_config) = config.voice_speak {
            instrument.constructed(ModuleFamily::VoiceSpeak);
            let provider = VoiceSpeakModule::new(
                voice_config,
                apeireth_tools_canonical::system_synth_engine(),
            );
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 任务簿 (`tool.task_board`, V6-a 任务编排持久队列, opt-in): 增/列/
        // 改派/完成 over 持久任务队列 (与会话账本同库的任务表; 崩溃恢复/幂等
        // 唤醒/环路熔断/结算接账在 orchestration 队列核)。治理档位在治理链
        // (`TaskBoardApprovalHook`): 写面 = write_intent 同族 (改任务簿 = 写
        // 文件) 每次人批, `task_list` 只读零审批; **无 failed/retry 工具面**
        // (failed 只能人判)。自述任务节与工具面同队同账 (task_board_status_probe)。
        if let Some(queue) = config.task_board.clone() {
            instrument.constructed(ModuleFamily::TaskBoard);
            let provider = TaskBoardModule::new(queue);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 性格周报 (`tool.trait_report`, 裁定 ⑤): 近 7 天四杆移动量 / 移动原因
        // top / 锁定态, 数据 = 成长计既有账 (不新造账)。**只读档零审批**
        // (与 `tool.self_status` 同档); 注册即闸 (成长账落位注入才注册), 名册
        // 按实际注册条件取值。自述性格节与工具面同账 (trait_status_probe)。
        if let Some(paths) = &config.trait_report {
            instrument.constructed(ModuleFamily::TraitReport);
            let provider = TraitReportModule::new(trait_growth_probe(paths.clone()));
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 暴露面体检 (`tool.exposure_audit`, V5-b 安全守望, 默认关): 本地只读
        // 自查 (只读零写, 报告是唯一产出), 与 CLI `apeireth exposure report`
        // 同一内核两个出口。治理 = 只读档 (注册即闸, grant 即用零审批)。
        if let Some(audit_config) = config.exposure_audit {
            instrument.constructed(ModuleFamily::ExposureAudit);
            let provider = ExposureAuditModule::new(audit_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 自主度第一小步 · 边角料提案面 (`tool.proposal_draft`, 默认关): 它提案,
        // 人点头 —— 提案只入提案账 (`proposals.jsonl` append-only), 不落任何真面;
        // 通过 = 账面标记 accepted, 提案不直接执行 (执行仍走既有工具面的正常审批
        // 链)。治理 = 低风险档 (读面 + append 账本, 注册即闸, grant 即用零审批)。
        if let Some(proposal_config) = &config.proposals {
            let provider = ProposalModule::new(proposal_config.clone());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 创作工具面 W1 · 图像操作 (`tool.image_ops`, 默认关): 闭合操作词表管道 +
        // 工作区限域读写 + 敏感面拒 + 炸弹图上限 + 格式白名单 + 原子写。注册条件 =
        // 开关 + `file_write` 唯一写总闸 + 工作区根 + 共享读前观测门禁 (注册即闸,
        // 与 `tool.apply_patch` 同受一闸一观测表); 输出写归档文件写入族。
        // 计数器句柄留给自述创作节 (同源同账)。
        let mut image_ops_activity = None;
        if config.image_ops && config.file_write {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::ImageOps);
                let provider = ImageOpsModule::new(root.clone(), Arc::clone(&observed_gate));
                image_ops_activity = Some(provider.activity());
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 创作工具面 W2 · 文档操作 (`tool.doc_ops`, 默认关): 受控读写族 (PDF 读 +
        // DOCX 读 + PDF 基础写批量管道) + 工作区限域 + 敏感面拒 + 大小/页数上限 +
        // 创建类原子写 (只创建不覆盖)。注册条件 = 开关 + 配置落位 (注册即闸);
        // **写面双重约束** = `file_write` 唯一写总闸 (组装根注入同一开关值,
        // 关 = 拒写不拒读) + 审批档每次人批。计数器句柄留给自述创作节 (同源同账)。
        let mut doc_ops_counters = None;
        if let Some(doc_ops_config) = &config.doc_ops {
            instrument.constructed(ModuleFamily::DocOps);
            let provider =
                DocOpsModule::new(doc_ops_config.clone().with_write_gate(config.file_write));
            doc_ops_counters = Some(provider.counters());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // 创作工具面 K3 · 表格操作 (`tool.sheet_ops`, 默认关): 受控读写族 (CSV 读 +
        // 筛排聚合 + 计算列 + 导出) + 工作区限域 + 敏感面拒 + 行列/字节上限
        // (防炸弹表) + 创建类原子写 (只创建不覆盖) + 数值精度如实 (f64 不静默
        // 取整, NaN/inf 零产出)。注册条件 = 开关 + 配置落位 (注册即闸);
        // **写面双重约束** = `file_write` 唯一写总闸 (组装根注入同一开关值,
        // 关 = 拒写不拒读) + 审批档每次人批。计数器句柄留给自述创作节 (同源同账)。
        let mut sheet_ops_counters = None;
        if let Some(sheet_ops_config) = &config.sheet_ops {
            instrument.constructed(ModuleFamily::SheetOps);
            let provider =
                SheetOpsModule::new(sheet_ops_config.clone().with_write_gate(config.file_write));
            sheet_ops_counters = Some(provider.counters());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // K 轴 L1 在场感知 (`tool.device_sense` + `tool.weather`, 默认关): 白名单制
        // 默认零设备 (devices.json StoredDoc 拒开) + mock 适配器 (真实协议插槽留
        // 位不接) + 摄像头类敏感读面 (显式授权 + 审计) + 天气 mock 档如实标注
        // 数据源 + 提醒信号只交信号。注册条件 = 配置注入 (注册即闸
        // `APEIRETH_ENABLE_DEVICE_SENSE`, 默认关 fail-closed)。感知计数器句柄留给
        // 自述在场节 (同源同账)。
        let mut device_sense_activity = None;
        if let Some(device_config) = &config.device_sense {
            instrument.constructed(ModuleFamily::DeviceSense);
            let provider = super::tool_modules::DeviceSenseModule::new(device_config.clone());
            device_sense_activity = Some(provider.activity());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }
        if let Some(weather_config) = &config.weather {
            instrument.constructed(ModuleFamily::Weather);
            let provider = super::tool_modules::WeatherModule::new(weather_config.clone());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // K 轴 L3/L4 受控执行骨架 (`tool.device_control`, 默认关): 三拍执行
        // (plan/preview 零副作用 → execute 真执行 + 状态账 append-only) + 三档
        // 分级审批 (reversible 审批档可配自动档 / irreversible 永逐次人批 /
        // money 永逐次人批 + IM 卡) + 设备白名单 (只读消费 L1 `devices.json`
        // 同一本设备表)。注册条件 = 配置注入 (注册即闸: 双闸
        // `APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_DEVICE_CONTROL`,
        // 见 `DeviceControlConfig::from_env`, 默认关 fail-closed)。
        let mut device_control_runner = None;
        if let Some(device_control_config) = &config.device_control {
            instrument.constructed(ModuleFamily::DeviceControl);
            let provider =
                super::tool_modules::DeviceControlModule::new(device_control_config.clone());
            device_control_runner = Some(Arc::clone(provider.runner()));
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // K 轴 L5 看家骨架 (`tool.guard_watch`, 默认关): 守护规则面 (阈值/时序
        // 条件, StoredDoc 拒开) + 巡检判定 + 告警账 (告警 = 关怀候选高优先,
        // 提醒侧消费端同源接缝)。**只看不动**: 告警零设备动作 (动手仍走
        // `tool.device_control` 三拍人批)。注册条件 = 配置注入 (注册即闸: 双闸
        // `APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_GUARD_WATCH`,
        // 见 `GuardWatchConfig::from_env`, 默认关 fail-closed)。
        if let Some(guard_watch_config) = &config.guard_watch {
            instrument.constructed(ModuleFamily::GuardWatch);
            let provider = super::tool_modules::GuardWatchModule::new(guard_watch_config.clone());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // W3.3 · 游戏练习场 (`tool.play_*`, 默认关): 装配块在 `install_bio_play`
        // (开关值来自装配根一次性解析的 `NervousSwitchboard`, 且要求工作区根注入),
        // 返回共享账本句柄供自述 play probe 只读接入。
        let bio_play_state = bio_production_wiring::install_bio_play(
            &mut capabilities,
            &clock,
            backends.workspace_root.as_deref(),
            instrument,
            nervous_switchboard.switch_enabled(BioSwitch::Play),
        );

        // W3.7 · 仪式完成面 (`tool.ritual_done` / `tool.ritual_skip`, 默认关):
        // 复用族总闸 + 关怀数据目录注入; 双工具写同一本 `<care_dir>/ritual_state.json`
        // (与 care 心跳同一目录), 补「仪式完成」显式事件源。返回的账句柄在自述槽
        // 武装后（见下方 self_status 块之后）填入 SG-22 观测槽。
        let bio_ritual_store = bio_production_wiring::install_bio_ritual_tools(
            &mut capabilities,
            &clock,
            bio_lifecycle_store_dir.as_deref(),
            nervous_switchboard.switch_enabled(BioSwitch::CareCandidates),
        );

        // 自省通道 (与既有 5 内置工具同列): `tool.self_status` 结构化自述面。
        // 只读档、默认可用、零审批; data 系探测口缺位时对应字段显式 null +
        // 原因, 注册与其余字段不受影响。
        if let Some(roster) = self_status_roster {
            let mut source = ProductionSelfStatusSource::new(
                roster,
                self_tuning_effective,
                backends.workspace_root.clone(),
            );
            if let Some(probe) = &backends.self_status_ledger {
                source = source.with_ledger_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_accountability {
                source = crate::accountability_probe::with_accountability_probe(
                    source,
                    Arc::clone(probe),
                );
            }
            if let Some(probe) = &backends.self_status_credentials {
                source = source.with_credentials_probe(Arc::clone(probe));
            }
            if let Some(budget) = &backends.self_status_budget {
                source = source.with_budget(budget.clone());
            }
            if let Some(probe) = &backends.self_status_quota {
                source = source.with_quota_probe(Arc::clone(probe));
            }
            if let Some(profile) = &backends.self_status_model_profile {
                source = source.with_model_profile(profile.clone());
            }
            if let Some(probe) = &backends.self_status_cost_routing {
                source = source.with_cost_routing_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_domain {
                source = source.with_domain_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_content_feed {
                source = source.with_content_feed_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_care {
                source = source.with_care_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_backup {
                source = source.with_backup_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_voice {
                source = source.with_voice_probe(Arc::clone(probe));
            }
            if let Some(probe) = &backends.self_status_persona {
                source = source.with_persona_probe(Arc::clone(probe));
            }
            if let Some(queue) = &config.task_board {
                source = source.with_task_board_probe(task_board_status_probe(Arc::clone(queue)));
            }
            // 自主度第一小步: 自述提案节与 `tool.proposal_draft` 工具面同一本
            // 提案账 (同账同源; 缺落位 = 显式 null + 原因)。
            if let Some(proposal_config) = &config.proposals {
                source = source
                    .with_proposal_probe(proposal_status_probe(proposal_config.data_dir.clone()));
            }
            // 性格节真值 (裁定 ④⑤): 锁定态 + 最近移动与 `tool.trait_report`
            // 同一成长计既有账 (同账同源, 缺落位 = 显式 null + 原因)。
            if let Some(paths) = &config.trait_report {
                source = source.with_trait_growth_probe(trait_status_probe(paths.clone()));
            }
            // 创作工具面 W1: 自述创作节 (最近操作计数) 与 `tool.image_ops` 工具面
            // 同一计数器 (同源同账); 面没注册 = 创作节显式 null + 原因。
            if let Some(activity) = &image_ops_activity {
                source = source.with_image_ops_probe(image_ops_status_probe(Arc::clone(activity)));
            }
            // 创作工具面 W2: 自述创作节 (文档操作计数) 与 `tool.doc_ops` 工具面
            // 同一计数器 (同源同账); 面没注册 = 创作节显式 null + 原因。
            if let Some(counters) = &doc_ops_counters {
                source = source.with_doc_ops_probe(doc_ops_status_probe(Arc::clone(counters)));
            }
            // 创作工具面 K3: 自述创作节与 `tool.sheet_ops` 工具面同一计数器 (同源同账)。
            if let Some(counters) = &sheet_ops_counters {
                source = source.with_sheet_ops_probe(sheet_ops_status_probe(Arc::clone(counters)));
            }
            // W3.2/W3.5/W3.6: `capabilities.bio_mechanisms` 只读聚合面 (装配块已搬
            // 至 `bio_production_wiring`; 迟绑定槽由组合根在 build 后填句柄)。
            source = bio_production_wiring::install_bio_self_status(
                source,
                config.task_board.as_ref(),
                bio_play_state.as_ref(),
                &bio_probe_slots,
                nervous_switchboard.switch_enabled(BioSwitch::SelfStatus),
            );
            // K 轴 L1: 自述在场节 (设备数/白名单数/最近感知) 与 `tool.device_sense`
            // 工具面同一本设备表同一进程内感知计数 (同源同账; 面没注册 = 在场节
            // 显式 null + 原因)。
            if let (Some(device_config), Some(activity)) =
                (&config.device_sense, &device_sense_activity)
            {
                source = source.with_device_presence_probe(
                    super::self_status_source::device_presence_probe(
                        device_config.data_dir.clone(),
                        Arc::clone(activity),
                    ),
                );
            }
            // K 轴 L3/L4 受控执行: 自述设备受控执行节 (控制计数/最近动作真值)
            // 与 `tool.device_control` 工具面同一执行核心同一状态账 (同源同账;
            // 面没注册 = 该节显式 null + 原因, 不编数)。
            if let Some(runner) = &device_control_runner {
                source = source.with_device_control_probe(
                    super::self_status_source::device_control_status_probe(Arc::clone(runner)),
                );
            }
            instrument.constructed(ModuleFamily::SelfStatus);
            // R1b-4a：自述帧 `capabilities.nervous_system.bus` 报总线观测面
            // （装配根唯一 NervousBus；本帧未绑 = null，不编造计数）。
            let provider =
                SelfStatusModule::new_with_bus(Arc::new(source), nervous_bus.as_ref().clone());
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // Wave 4 观测补格: 自述槽此时已按开关武装 → 把仪式账句柄填入 SG-22 观测槽
        //（开关关 / 缺目录 = `None` = 槽保持空；fill 未知武装时 no-op）。
        bio_probe_slots.fill_ritual_store(bio_ritual_store.as_ref());

        // External tool bridge (MCP): the slot is opt-in, and enabling it
        // loads the server list at assembly time (env primary, data-directory
        // stored document secondary) with fail-closed semantics. Connections
        // are established asynchronously after build; the module bag is the
        // registration target for the dynamic tools discovery produces. The
        // server list was loaded above (before the roster projection) — a
        // server list with no enabled entry assembles an inert bridge (no
        // external tools), and the roster reports that honestly.
        let mut mcp_bridge = None;
        let mut mcp_module = None;
        if let Some(server_config) = mcp_server_config {
            instrument.constructed(ModuleFamily::Mcp);
            let options = McpBridgeOptions::from_env();
            let bridge = McpToolBridge::new(server_config, options).map_err(|error| {
                RuntimeError::misconfigured(format!("mcp tool bridge refused to assemble: {error}"))
            })?;
            let bridge = Arc::new(bridge);
            tool_modules.push(Arc::new(
                apeireth_tools_canonical::mcp_bridge::McpPlugin::new(bridge.clone()),
            ));
            mcp_bridge = Some(bridge);
            // Retained construction accessor is a compatibility bag, not the production catalog.
            mcp_module = Some(Arc::new(McpModule::new()));
        }

        // 安全/逆向侦察 (`tool.recon.*`, 9 件, 默认关): 注册即闸 —— 开关
        // `APEIRETH_ENABLE_RECON=1` 才构造, 且必须有工作区根 (文件类工具的落点;
        // 缺省 = 不注册, 不静默降级为「只装网络三件」)。网络目标由 scope 把关,
        // 空 scope 在工具内即拒绝 (fail-closed)。
        if let Some(recon_config) = &config.recon {
            if let Some(root) = &backends.workspace_root {
                instrument.constructed(ModuleFamily::Recon);
                let provider = ReconModule::new(root.clone(), Arc::clone(&recon_config.scope));
                collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
            }
        }

        // 外部适配器 (`ext.*`, 10 件, 默认关): 注册即闸 —— 开关
        // `APEIRETH_ENABLE_EXTERNAL=1` 才构造。配置自带工作区根与 scope
        // (文件类限域 + 网络目标授权), 启动期不探测系统二进制。
        if let Some(external_config) = &config.external {
            instrument.constructed(ModuleFamily::ExternalTools);
            let provider = ProcessModule::new(external_config);
            collect_tool_module(&mut tool_modules, &mut capabilities, &provider)?;
        }

        // SG-25 工具生态健康: 静态能力表装配完毕 → 把剖面填入迟绑定只读槽
        // (开关关 = fill no-op, 槽保持空、probe 未注册, 自述帧逐字节不变)。
        // 快照 = 装配期的静态注册面; 运行期动态发现的外部工具不在其中 (如实缺口)。
        bio_probe_slots.fill_ecosystem_profiles(&capabilities);

        // Unified Memory 2.0 coordinator wiring
        let mut shared_coordinator: Option<Arc<MemoryCoordinator>> = None;
        if config.memory_recall || config.memory_writeback {
            instrument.constructed(ModuleFamily::MemoryCoordinator);
            let memory = required(backends.memory.clone(), "memory", "memory")?;
            let governance = required(
                backends.memory_governance.clone(),
                "memory_governance",
                "memory_governance",
            )?;
            let mut coordinator =
                MemoryCoordinator::new(Arc::clone(&memory), Arc::clone(&governance));
            if let Some(scoped) = &backends.scoped_memory {
                coordinator = coordinator.with_scoped_backend(Arc::clone(scoped));
            }
            if let Some(embed) = &backends.embedding_provider {
                coordinator = coordinator.with_embedding_provider(Arc::clone(embed));
            }
            if config.memory_injection {
                coordinator = coordinator.with_memory_injection_format();
            }
            if let Some(pref) = &backends.preferences {
                coordinator = coordinator.with_preferences(Arc::clone(pref));
            }
            if let (Some(graph), Some(associations)) = (&backends.graph, &backends.associations) {
                coordinator =
                    coordinator.with_experience(Arc::clone(graph), Arc::clone(associations));
            }
            if let Some(history) = &backends.access_history {
                coordinator = coordinator.with_activation_source(Arc::new(
                    AccessHistoryActivationSource::new(Arc::clone(history), 0.5, 0.0),
                ));
            }
            if let (Some(source), Some(identity)) =
                (&backends.typed_recall, &backends.typed_recall_identity)
            {
                coordinator = coordinator.with_typed_recall(Arc::clone(source), identity.clone());
            }
            shared_coordinator = Some(Arc::new(coordinator));
        }

        // Register cognitive modules
        if config.memory_recall {
            let memory = required(backends.memory.clone(), "memory_recall", "memory")?;
            instrument.constructed(ModuleFamily::MemoryRecall);
            let mut module = MemoryRecallModule::new(memory);
            if let Some(coord) = &shared_coordinator {
                module = module.with_coordinator(Arc::clone(coord));
            }
            if let Some(wire) = &backends.self_tuning {
                module = module.with_self_tuning(Arc::clone(wire));
            }
            // V4-c 检索维度自进化: 注入回执面挂记忆召回模块 (默认无 = 调权关)。
            if let Some(wire) = &backends.retrieval_tuning {
                module = module.with_retrieval_tuning(Arc::clone(wire));
            }
            if let (Some(wiki), Some(graph), Some(associations)) =
                (&backends.wiki, &backends.graph, &backends.associations)
            {
                module = module.with_experience(
                    Arc::clone(wiki),
                    Arc::clone(graph),
                    Arc::clone(associations),
                );
            }
            if let Some(history) = &backends.access_history {
                let history: Arc<dyn MemoryRecallAccessStore> = history.clone();
                module = module.with_access_store(history, Arc::clone(&clock));
            }
            if let Some(policy) = &config.proactive_recall {
                module = module.with_proactive_recall(policy.clone());
            }
            if config.morphology_recall {
                module = module.with_morphology_recall();
            }
            if config.community_triage {
                if let Some(graph) = &backends.graph {
                    module = module.with_community_triage(Arc::clone(graph));
                }
            }
            modules.push(Arc::new(module.with_telemetry(Arc::clone(&telemetry))));
        }

        if config.preference_recall {
            instrument.constructed(ModuleFamily::PreferenceRecall);
            modules.push(Arc::new(
                PreferenceRecallModule::new(required(
                    backends.preferences.clone(),
                    "preference_recall",
                    "preferences",
                )?)
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        // Learning sits beside recall: recall overlays at TurnStart, learning
        // writes at AfterTurn for future turns.
        if config.preference_learning {
            instrument.constructed(ModuleFamily::PreferenceLearning);
            modules.push(Arc::new(
                PreferenceLearningModule::new(required(
                    backends.preferences.clone(),
                    "preference_learning",
                    "preferences",
                )?)
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.judge.enabled {
            instrument.constructed(ModuleFamily::Judge);
            modules.push(Arc::new(
                JudgeModule::new(config.judge, Arc::clone(&observations))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.self_assessment {
            instrument.constructed(ModuleFamily::SelfAssessment);
            modules.push(Arc::new(
                SelfAssessmentModule::new(
                    required(
                        backends.self_assessments.clone(),
                        "self_assessment",
                        "self_assessments",
                    )?,
                    Arc::clone(&clock),
                    Arc::clone(&observations),
                )
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.council {
            instrument.constructed(ModuleFamily::Council);
            modules.push(Arc::new(
                CouncilModule::new(
                    required(backends.council, "council", "council")?,
                    Arc::clone(&clock),
                )
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.organs {
            instrument.constructed(ModuleFamily::Organs);
            let module = Arc::new(OrganModule::new(Arc::clone(&clock)));
            organ_module = Some(Arc::clone(&module));
            modules.push(module as Arc<dyn Module>);
        }

        if config.reflexion {
            instrument.constructed(ModuleFamily::Reflexion);
            modules.push(Arc::new(
                super::cognitive::ReflexionModule::new(
                    required(backends.reflexion_store, "reflexion", "reflexion_store")?,
                    Arc::clone(&observations),
                    Arc::clone(&clock),
                )
                .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        if config.absorption_insight {
            instrument.constructed(ModuleFamily::AbsorptionInsight);
            modules.push(Arc::new(super::cognitive::AbsorptionInsightModule::new()));
        }

        if config.partner_bond {
            instrument.constructed(ModuleFamily::PartnerBond);
            let store = required(backends.partner_store, "partner_bond", "partner_store")?;
            let subject_id = backends
                .typed_recall_identity
                .as_ref()
                .map(|identity| identity.subject_id.clone())
                .unwrap_or_else(|| "local-user".to_string());
            modules.push(Arc::new(
                super::cognitive::PartnerBondModule::new(store, subject_id, Arc::clone(&clock))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        // V4-a 恒定小注入 (人格记忆层回取面 ①) 的 TurnStart overlay 装配点:
        // 自我叙事摘要 ≤ 恒预算 (`CONSTANT_INJECTION_BUDGET_CHARS`) 进提示词。
        // **缺人格层 = 不注册本模块 = 零注入零变化**; 人格层无自我叙事沉淀时
        // 同样零注入 (未沉淀就是未沉淀, 不编)。
        if let Some(persona) = &backends.persona_layer {
            instrument.constructed(ModuleFamily::PersonaConstantInjection);
            modules.push(Arc::new(
                super::cognitive::PersonaConstantInjectionModule::new(Arc::clone(persona))
                    .with_telemetry(Arc::clone(&telemetry)),
            ));
        }

        // W3.2 / W3.3 / W3.5 / W3.6: 修复指令段 + 调质 tone + 生命周期装配块 (全默认关,
        // 关 = 不构造不注册), 句柄回传装配根；生命周期账本持久化目录 = 关怀数据目录
        // （缺省 None = 纯内存，不建文件）。
        let bio_wiring = bio_production_wiring::install_bio_modules(
            &mut modules,
            &clock,
            &capabilities,
            bio_lifecycle_store_dir.as_deref(),
            config.context_budget_chars,
            &nervous_switchboard,
        );
        // R1b-2a: 调质事件源收敛到总线订阅——开关开才有订阅者, 拿到总线后注册
        // (关 = `None` = 不注册 = 总线零订阅者 = 零变化)。
        if let Some(subscriber) = &bio_wiring.tone_subscriber {
            subscriber.subscribe(&nervous_bus);
        }
        // Wave 4 (SG-20): 社会修复观测源订阅人工拒绝信号（源 None = 开关关 = 零订阅）。
        if let Some(source) = bio_wiring.social_repair_source.as_ref() {
            source.subscribe(&nervous_bus);
        }
        // R1b-2b: 生命周期账本在模块安装后才存在 → 迟绑定槽填同一 Arc
        //（自述开关关时 fill no-op；槽空 = 单格 null + 未接线真因）。
        bio_probe_slots.fill_lifecycle_state(bio_wiring.lifecycle_state.as_ref());
        // Wave 4 观测补格: 调质句柄与昼夜节律配置在模块安装后填入迟绑定槽
        //（自述开关关时 fill no-op；槽空 = 单格 null + 未接线真因）。
        bio_probe_slots.fill_tone_state(bio_wiring.tone_state.as_ref());
        bio_probe_slots.fill_circadian_config(CircadianProbeConfig::new(
            nervous_switchboard.circadian_tz_offset_min(),
            nervous_switchboard.switch_enabled(BioSwitch::Circadian),
        ));
        // Wave 4 homeostasis 行为接线: 装配装饰器的观测源（开关关时 fill no-op）。
        bio_probe_slots.fill_homeostasis(bio_wiring.homeostasis_source.as_ref());
        bio_probe_slots.fill_social_repair(bio_wiring.social_repair_source.as_ref());

        if config.memory_writeback {
            instrument.constructed(ModuleFamily::MemoryWriteback);
            let mut module = MemoryWritebackModule::new(
                required(backends.memory, "memory_writeback", "memory")?,
                clock,
            );
            if let Some(coord) = &shared_coordinator {
                module = module.with_coordinator(Arc::clone(coord));
            }
            if let Some(materializer) = &backends.memory_materializer {
                module = module.with_materializer(Arc::clone(materializer));
            } else if let Some(extractor) = &backends.memory_extractor {
                module = module.with_extractor(Arc::clone(extractor));
            }
            if let Some(sink) = &backends.typed_sink {
                module = module.with_typed_sink(Arc::clone(sink));
            }
            if config.consolidation {
                module = module.with_consolidation();
            }
            // V4-a 人格提取步: 开关 + 人格层落库面齐备才接 (默认未接线);
            // 运行触发点 = 整编触发点 (with_consolidation), 同一条人格管线。
            if config.persona_consolidation {
                if let Some(persona) = &backends.persona_layer {
                    module = module.with_persona_layer(Arc::clone(persona));
                }
            }
            // V4-b 散列增量门: 开关 + 散列门落库面齐备才接 (默认未接线);
            // 运行触发点 = 整编触发点 (with_consolidation), 同一条整编管线。
            if config.consolidation_hash_gate {
                if let Some(gate) = &backends.ingest_gate {
                    module = module.with_ingest_gate(Arc::clone(gate));
                }
            }
            if let (Some(wiki), Some(graph), Some(associations)) =
                (&backends.wiki, &backends.graph, &backends.associations)
            {
                module = module.with_experience(
                    Arc::clone(wiki),
                    Arc::clone(graph),
                    Arc::clone(associations),
                );
            }
            modules.push(Arc::new(module.with_telemetry(Arc::clone(&telemetry))));
        }

        // W3.6: 器官模块装配后把审议门账读取口交给自述槽 (未装配 = no-op).
        bio_probe_slots.fill_from_organ_module(organ_module.as_ref());

        let mut seen = std::collections::BTreeSet::new();
        for module in &modules {
            let id = module.manifest().id.clone();
            if !seen.insert(id.clone()) {
                return Err(RuntimeError::misconfigured(format!(
                    "duplicate cognitive module id {id:?}"
                )));
            }
        }

        let owned: std::collections::BTreeSet<_> = tool_modules
            .iter()
            .flat_map(|module| module.tools())
            .map(|tool| tool.id().clone())
            .collect();
        let remaining = capabilities
            .iter()
            .filter(|tool| !owned.contains(tool.id()))
            .cloned()
            .collect::<Vec<_>>();
        if !remaining.is_empty() {
            tool_modules.push(Arc::new(ToolModule::new(
                apeireth_core::kernel::PluginId::new("module.tool.host_extensions")
                    .expect("static id"),
                remaining,
            )?));
        }
        Ok(Self {
            modules,
            tool_modules,
            capabilities,
            telemetry,
            mcp_bridge,
            mcp_module,
            organ_module,
            bio_tone_state: bio_wiring.tone_state,
            bio_lifecycle_state: bio_wiring.lifecycle_state,
            bio_probe_slots,
            nervous_bus,
            nervous_switchboard,
        })
    }

    /// Wave 4 / ADR-0009 R1b-1a: 神经信号总线句柄 (适配器只 publish, 不直写
    /// `ToneState`; 与注册的桥共享同一 `Arc`)。
    pub fn nervous_bus(&self) -> Arc<NervousBus> {
        Arc::clone(&self.nervous_bus)
    }

    /// 本次装配生效的开关快照 (`NervousSwitchboard::from_env` 一次解析)。
    pub fn nervous_switchboard(&self) -> &NervousSwitchboard {
        &self.nervous_switchboard
    }

    /// Wave 4 / ADR-0009 R1b-2c: 审批社交信号源句柄。
    ///
    /// 只有调质开关开时才交付 (`Some`); 开关关 = `None` = 审批路径不发布、
    /// 总线零计数, 与未接线逐字节一致。句柄与注册的调质订阅者共享同一条
    /// [`NervousBus`] (同一 `Arc`)。
    #[must_use]
    pub fn bio_approval_social_source(&self) -> Option<Arc<BioApprovalSocialSource>> {
        self.nervous_switchboard
            .switch_enabled(BioSwitch::Tone)
            .then(|| Arc::new(BioApprovalSocialSource::new((*self.nervous_bus).clone())))
    }

    /// Attach the context-window projector using its default policy.
    #[must_use]
    pub fn register_context_projection(
        &self,
        builder: super::runtime::RuntimeBuilder,
    ) -> super::runtime::RuntimeBuilder {
        with_memory_context_projection(builder, ContextWindowManager::default())
    }

    pub fn modules(&self) -> &[Arc<dyn Module>] {
        &self.modules
    }

    /// Capabilities contributed by concrete production providers.
    pub fn capabilities(&self) -> &[Arc<dyn ToolCapability>] {
        &self.capabilities
    }

    /// Consume the set into the builder's module registration calls.
    pub fn register_into(
        self,
        mut builder: super::runtime::RuntimeBuilder,
    ) -> super::runtime::RuntimeBuilder {
        for module in self.modules {
            builder = builder.with_registered_behavior(module);
        }
        for module in self.tool_modules {
            builder = builder.with_plugin(module);
        }
        builder
    }

    /// Stable slot ids in the exact registration order.
    pub fn ids(&self) -> Vec<String> {
        self.modules
            .iter()
            .map(|module| module.manifest().id.clone())
            .collect()
    }

    /// Shared non-sensitive hook telemetry for the registered modules.
    pub fn telemetry(&self) -> Arc<CognitiveTelemetry> {
        Arc::clone(&self.telemetry)
    }

    /// The external tool bridge assembled for the MCP slot, when enabled.
    /// Adapters keep this handle to run the asynchronous connect/discovery pass.
    pub fn mcp_bridge(&self) -> Option<&Arc<McpToolBridge>> {
        self.mcp_bridge.as_ref()
    }

    /// The MCP dynamic tool module bag, when the MCP slot is enabled.
    pub fn mcp_module(&self) -> Option<&Arc<McpModule>> {
        self.mcp_module.as_ref()
    }

    /// W3.3 外部事件源: 把 guard 免疫记忆共享句柄接到器官模块的审议门, 并把
    /// **同一份**句柄填进 W3.6 自述槽 (开关关 = 槽未武装, fill no-op)。传进来的
    /// 必须是 hook 正在使用的同一份句柄 (否则观测与门读的不是同一份状态);
    /// 未启用器官模块或句柄为 `None` 时不做任何事, 返回是否真的挂上审议门。
    ///
    /// **默认零行为变化**: 不调用本方法 / 传 `None` = 审议门从未读免疫记忆。
    pub fn attach_immune_memory(&self, memory: Option<SharedImmuneMemory>) -> bool {
        self.bio_probe_slots.fill_immune_memory(memory.clone());
        match (&self.organ_module, memory) {
            (Some(module), Some(memory)) => {
                module.attach_immune_memory(memory);
                true
            }
            _ => false,
        }
    }
}

fn required<T>(value: Option<Arc<T>>, slot: &str, dependency: &str) -> RuntimeResult<Arc<T>>
where
    T: ?Sized,
{
    value.ok_or_else(|| {
        RuntimeError::misconfigured(format!(
            "cognitive slot {slot:?} requires injected backend {dependency:?}"
        ))
    })
}
