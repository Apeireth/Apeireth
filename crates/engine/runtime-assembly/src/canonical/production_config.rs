//! Production module configuration switches (the "what" half of assembly).
//!
//! `production.rs` owns the assembly procedure ("how" things get installed);
//! this file owns the configuration surface ("what" gets installed, and by
//! default): [`ProductionModulesConfig`] plus its compatibility alias
//! [`CognitiveModuleConfig`].
//!
//! Declared as a child module of `production.rs` via
//! `#[path = "production_config.rs"]` and re-exported from `production.rs`
//! unchanged, so every pre-existing path keeps resolving verbatim —
//! `canonical::production::ProductionModulesConfig` and the crate-root
//! `apeireth_runtime_assembly::ProductionModulesConfig` are unaffected, and
//! no call site changes.
//!
//! # 0 装诚实
//! - **纯搬迁**: 字段、默认值、装配顺序一律不变;
//! - 本文件只声明**配置面**, 不注册任何模块 (注册仍在 `production.rs` 装配根)。

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_memory::ProactiveRecallPolicy;
use apeireth_tools_canonical::{FetchConfig, TrustedShellConfig};

use crate::canonical::cognitive::JudgeConfig;
use crate::canonical::trait_growth::TraitGrowthPaths;

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
    /// Register the authenticated HTTP write face (`tool.http_write`, W1 C-B2)
    /// when config is supplied.
    ///
    /// 带鉴权的受控写面: 受控出网上的 POST/PUT/PATCH/DELETE —— 域名白名单 +
    /// 显式凭据绑定 + 请求/响应体有界 + **不跟随重定向**。治理档位 (写面逐次
    /// 人批) 由治理链钩子承载。默认关 (opt-in, 开关名
    /// `APEIRETH_ENABLE_HTTP_WRITE` 只认 `"1"`)。
    ///
    /// ⚠️ 写面**只认显式域名白名单**策略 ⇒ 宿主注入的配置必须带
    /// `EgressPolicy::ExplicitAllowList`; 白名单为空 ⇒ 每个目的地都表外 ⇒
    /// 拒绝即帧。
    pub http_write: Option<apeireth_tools_canonical::http_write::HttpWriteConfig>,
    /// Register the internet search face (`tool.web_search`, W1 C-H4) when
    /// config is supplied.
    ///
    /// 互联网检索面: 打**运维显式配置**的检索端点, 结果有界收敛 + 来源标注
    /// (出网**只读**面 ⇒ 零审批)。**没有任何内置默认端点**: 端点没配 ⇒
    /// 每次调用拒绝即帧。默认关 (opt-in, 开关名 `APEIRETH_ENABLE_WEB_SEARCH`
    /// 只认 `"1"`)。
    pub web_search: Option<apeireth_tools_canonical::web_search::WebSearchConfig>,
    /// Register the owner calendar face (`tool.calendar`, W1 C-H5) when config
    /// is supplied.
    ///
    /// 主人日程面: 本地日程账 (查询/创建/改期/取消); 写动作逐次人批 (治理档
    /// 在治理链钩子上)。默认关 (opt-in, 开关名 `APEIRETH_ENABLE_CALENDAR`
    /// 只认 `"1"`)。
    pub calendar: Option<apeireth_tools_canonical::calendar::CalendarConfig>,
    /// Register the process-tree snapshot face (`tool.process_tree`, W1 C-B1).
    ///
    /// 进程树快照面: 枚举 (pid / 父 / 命令 / 状态) + 按 pid 幂等取消一棵树。
    /// **零配置** (不需要端点或凭据) ⇒ 用 `bool` 开关。
    ///
    /// ⚠️ 开关口径 = **复用** `APEIRETH_ENABLE_SHELL` (注册即闸 ⇒ shell 关 =
    /// 无进程可看, 语义自洽) ⇒ **0 新 env 名**。默认关 (opt-in)。
    pub process_tree: bool,
    /// Register the life-integration content feed (`tool.content_feed`, J1) when
    /// config is supplied.
    ///
    /// 内容流接入: 订阅源管理 + 受控拉取 + 结构化摘要 + 入记忆流 (L2 封套披露,
    /// 跨源内容不直进提示词)。治理档位 (RequireApproval 级: 拉取与入库都经审批
    /// 档, 改订阅 = 写文件走 write_intent 同族) 由治理链钩子承载。默认关 (opt-in)。
    pub content_feed: Option<apeireth_tools_canonical::ContentFeedConfig>,
    /// Register the proactive-care rule scheduling tool (`tool.care_schedule`,
    /// V3-a 双引擎主动关怀) when the care rules data directory is supplied.
    ///
    /// 关怀规则面: 增删列关怀规则 (四类触发器词表), 写文件面走 write_intent
    /// 同族 (改规则 = 写文件)。治理档位 (RequireApproval 级: 每次调用都停人工
    /// 审批, 无自动放行档) 由治理链钩子承载。关怀发起不走本面 (双引擎判定 →
    /// 标准回合)。默认关 (opt-in)。
    pub care_schedule: Option<PathBuf>,
    /// Register the general-voice speech tool (`tool.voice_speak`, V3-b 通用嗓闭环)
    /// when the voice configuration is supplied.
    ///
    /// 发声面: 文本 → 系统语音引擎合成即播 + 语句账本 (`spoken.jsonl`) + 安静
    /// 纪律 (勿扰窗口, 与 care_rules 同源)。治理 = 发声面档 (注册即闸
    /// `APEIRETH_ENABLE_VOICE_SPEAK`, grant 即发声免逐次人批)。默认关 (opt-in)。
    pub voice_speak: Option<apeireth_tools_canonical::VoiceSpeakConfig>,
    /// Register the backup export tool (`tool.backup_export`, G5 备份恢复) when
    /// config is supplied.
    ///
    /// 备份导出面: 全量打包记忆/账本 (七类数据面) + manifest.json 逐文件摘要
    /// (损坏可检出); **凭据不随行** (密钥/令牌面显式排除 + 测试钉死)。恢复/校验
    /// 是 CLI 面 (`apeireth backup restore|verify`, 本地操作), 不走本面。治理
    /// 档位 (RequireApproval 级: 每次导出都停人工审批) 由授权层同源接线。默认关。
    pub backup: Option<apeireth_tools_canonical::backup::BackupExportConfig>,
    /// Register the durable task board (`tool.task_board`, V6-a 任务编排持久队列)
    /// when the task queue is supplied.
    ///
    /// 任务簿工具面: 增/列/改派/完成 over 持久任务队列 (与会话账本同库的任务表,
    /// 崩溃恢复/幂等唤醒/环路熔断/结算接账在 [`apeireth_orchestration::task_queue`])。
    /// 治理档位 (写面 = write_intent 同族: 改任务簿 = 写文件, 每次人批无自动
    /// 放行档; `task_list` 只读零审批) 由治理链
    /// ([`crate::canonical::task_board_tool::TaskBoardApprovalHook`]) 唯一承载; **无
    /// failed/retry 工具面**。默认关 (opt-in)。
    pub task_board: Option<Arc<apeireth_orchestration::task_queue::TaskQueue>>,
    /// 生物机制族 Wave 4 (SG-14): 上下文预算 (字符) 注入装配面模块;
    /// `None` = 不注入 (默认策略, 行为不变)。与 cli 组装器同一 env 真源。
    pub context_budget_chars: Option<usize>,
    /// 生物机制族 W3.3 · 游戏练习场族总闸 (env `APEIRETH_ENABLE_BIO_PLAY`, 只认 `"1"`)。
    /// 装配根解析后回填 (env 读取收敛到装配根单入口); 注册条件 = 本总闸 AND 工作区根已注入。
    pub bio_play: bool,
    /// 生物机制族 W3.7 · 生物候选总闸 (env `APEIRETH_ENABLE_BIO_CARE_CANDIDATES`, 只认 `"1"`), 仪式完成面复用。
    /// 装配根解析后回填; 注册条件 = 本总闸 AND 关怀数据目录已注入 (`care_schedule`)。
    pub bio_care_candidates: bool,
    /// Register the external tool bridge (MCP) module.
    ///
    /// When enabled, the server list is loaded at assembly time from
    /// `APEIRETH_MCP_SERVERS` (primary entry) or the data directory's
    /// `mcp-servers.json` (secondary entry, stored-document fail-closed
    /// semantics): a defective configuration is a boot error, never a
    /// silent fallback. Off by default.
    ///
    /// Self-report semantics (honest, 2026-11-05 批): the `mcp` roster row is
    /// the **registered tool count**口径 (see [`Self::mcp_registered_tools`]);
    /// the "server list loaded" fact moved to the `mcp_bridge` row.
    pub mcp: bool,
    /// Data directory whose `mcp-servers.json` is the secondary config entry
    /// for the external tool bridge (see [`Self::mcp`]).
    pub mcp_data_dir: Option<PathBuf>,
    /// 外部工具面**实际登记件数**（名册 `mcp` 行的唯一口径）: 实际登记进运行时
    /// 能力表的外部工具件数。0 = 未接线 / 发现失败 / 无服务器 ⇒ 名册照实 false
    /// （fail-closed）。装配期快照拿不到这个数（接线在运行时建成之后）⇒ 装配根
    /// 在自述**调用时**现投影（纯函数
    /// [`crate::canonical::self_status_roster::roster_from_config`] + 晚绑定真值）。
    pub mcp_registered_tools: usize,
    /// 外部工具**桥面**已装配（配置面事实: 开关开且服务器列表已加载）: 与工具
    /// 件数**无关** —— 桥在而一件工具都没登记 ⇒ 本行 true 而 `mcp` false; 本行
    /// 不冒充「外部工具面可用」。
    pub mcp_bridge: bool,
    /// Register the nine native recon tools (`tool.recon.*`, 安全/逆向侦察套件).
    ///
    /// 侦察面 = 「看」不是「做」: 字符串/熵/魔数、ELF/PE/Mach-O、CLR 元数据、
    /// JS 词法、YARA 子集、pcap、TLS 握手 —— 全部只读。网络三件
    /// (`tool.recon.net` / `.web` / `.tls`) 只连 [`apeireth_tools_canonical::recon::Scope`]
    /// 显式授权的目标; 空 scope = 全拒绝 (fail-closed)。注册即闸: 开关
    /// `APEIRETH_ENABLE_RECON=1` 才构造 (默认关), 授权面同源 (同一开关值在治理层
    /// grant)。工作区根缺省时不注册 (文件类工具无落点)。
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
    /// side-calls (首个生产器官接线的明确放量边界).
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
    /// 代码地图工具注册 (`tool.repo_map`, 代码骨架概览)。
    ///
    /// 2026-10-24: 跨语言符号提取 + 跨文件依赖图 + Personalized PageRank
    /// 排序, 在 token 预算内输出紧凑大纲。**只读**、工作区限域、三重预算封顶。
    ///
    /// 扫描根与 `tool.repo` **同源**（[`Self::repo`] 用同一个
    /// `backends.workspace_root`）—— 同一份工作区只有一把尺。
    /// 工作区根缺席时不注册（名册按实际注册条件取值）。
    pub repo_map: bool,
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
    /// `APEIRETH_ENABLE_EXPOSURE_AUDIT`, grant 即用零审批)。默认关 (opt-in)。
    pub exposure_audit: Option<apeireth_tools_canonical::ExposureAuditConfig>,
    /// Register the leftover-proposal tool (`tool.proposal_draft`, 自主度第一小步 ·
    /// 边角料提案面) when the proposal-ledger data directory is supplied.
    ///
    /// 提案面 (保守版: 它提案, 人点头): 对发现的边角料起草结构化提案 ——
    /// **提案只入提案账** (`proposals.jsonl` append-only), 不落任何真面;
    /// 人审温柔 (detail 恒含「建议你亲自确认」行 + 审批卡逐条请人过目);
    /// 通过 = 账面标记 accepted, **提案不直接执行** (执行仍走既有工具面的正常
    /// 审批链)。治理 = **低风险档** (读面 + append 账本, 注册即闸
    /// `APEIRETH_ENABLE_PROPOSALS`, 默认关 fail-closed, grant 即用零审批)。
    pub proposals: Option<apeireth_tools_canonical::proposal::ProposalConfig>,
    /// Register the image-ops tool (`tool.image_ops`, 创作工具面 W1 · 图像操作).
    ///
    /// 图像操作面: 工作区限域读 → 闭合操作词表管道 (resize/crop/rotate/flip/
    /// format_convert/brightness/contrast/grayscale/blur/sharpen/watermark_text)
    /// → 工作区限域原子写 (临时文件 + 换名, 失败不留半成品)。受控面 = 敏感面拒
    /// + 大小/像素上限 (防炸弹图) + 格式白名单 (png/jpeg/webp/bmp) + 读前观测门禁
    /// (未读不得覆盖写, 与读工具同一张观测表)。输出写**归档文件写入族** (读码定
    /// 归档: 治理层族词表是冻结面, 图像写 = 写文件, 不另立图像写入族)。注册条件
    /// = 本开关 + [`Self::file_write`] + 工作区根 + 共享读前观测门禁 (注册即闸)。
    /// 默认关 (opt-in, fail-closed)。
    pub image_ops: bool,
    /// Register the doc-ops tool (`tool.doc_ops`, 创作工具面 W2 · 文档操作).
    ///
    /// 受控读写族: PDF 读 (info/extract_text) + DOCX 读 (extract_text/outline) +
    /// PDF 基础写 (merge/split/extract_pages/watermark_text, 批量管道)。受控面 =
    /// 工作区限域 + 敏感面拒 + 大小/页数上限 + 创建类原子写 (只创建不覆盖)。
    /// **写面双重约束**: 写动作受 [`Self::file_write`] 唯一写总闸 (组装根把同一
    /// 开关值注入 [`apeireth_tools_canonical::DocOpsConfig::with_write_gate`],
    /// 关 = 拒写不拒读) + 审批档每次人批。注册条件 = 本开关 + 工作区根 (注册即闸;
    /// 读面不背写总闸, 文档读是本面的第一公民)。默认关 (opt-in, fail-closed)。
    pub doc_ops: Option<apeireth_tools_canonical::DocOpsConfig>,
    /// Register the sheet-ops tool (`tool.sheet_ops`, 创作工具面 K3 · 表格操作).
    ///
    /// 受控读写族: 表读 CSV / xlsx (read/summary) + 筛排聚合 (filter/sort/aggregate) +
    /// 计算列 (formula) + 导出 (export_csv)。受控面 = 工作区限域 + 敏感面拒 +
    /// 行列/字节上限 (防炸弹表) + 创建类原子写 (只创建不覆盖) + 数值精度如实
    /// (f64 不静默取整, NaN/inf 零产出)。**写面双重约束**: 写动作受
    /// [`Self::file_write`] 唯一写总闸 (组装根把同一开关值注入
    /// [`apeireth_tools_canonical::SheetOpsConfig::with_write_gate`], 关 = 拒写
    /// 不拒读) + 审批档每次人批。注册条件 = 本开关 + 工作区根 (读面不背写总闸,
    /// 表格读是本面的第一公民)。默认关 (opt-in, fail-closed)。
    pub sheet_ops: Option<apeireth_tools_canonical::SheetOpsConfig>,
    /// Register the device-sense tool (`tool.device_sense`, K 轴 L1 在场感知).
    ///
    /// 在场感知面: 设备表 (`devices.json` StoredDoc 拒开, **白名单制默认零设备**)
    /// + 适配器插槽 (mock 已接固定 fixture, 真实协议族留插槽不接) + 摄像头类
    /// 敏感读面 (显式授权 + 审计账)。治理 = **只读档** (注册即闸
    /// `APEIRETH_ENABLE_DEVICE_SENSE`, 默认关 fail-closed)。自述在场节与工具面
    /// 同一本设备表同一进程内感知计数。默认关 (opt-in)。
    pub device_sense: Option<apeireth_tools_canonical::device::sense::DeviceSenseConfig>,
    /// Register the weather tool (`tool.weather`, K 轴 L1/L2 天气感知 + 提醒信号).
    ///
    /// 天气面: 公开天气读数 (mock 档 = 固定 fixture **如实标注数据源**; live 档 =
    /// 受控出口通道 / fetch 族**留插槽不接**) + 提醒信号 (只交信号: 勿扰/频率等
    /// 提醒纪律归提醒侧消费)。治理 = **只读档** (注册即闸, 与 `tool.device_sense`
    /// 同一在场感知族开关, 默认关 fail-closed)。默认关 (opt-in)。
    pub weather: Option<apeireth_tools_canonical::device::weather::WeatherConfig>,
    /// Register the device-control tool (`tool.device_control`, K 轴 L3/L4 受控执行骨架).
    ///
    /// 设备受控执行面: 三拍执行 (plan → preview → execute, 前两拍零副作用、
    /// 执行拍真执行 + 状态账 `device-actions.jsonl` append-only) + 动作三档分级
    /// (reversible 审批档可配自动档 / irreversible 永逐次人批 / money 永逐次
    /// 人批 + IM 卡) + 设备白名单 (只读消费 L1 [`Self::device_sense`] 同一本
    /// `devices.json`, 未登记/未启用拒绝即帧)。执行面 = mock 适配器 (灯/空调/
    /// 微波炉 fixture, 真实协议族后插)。治理档在治理链
    /// (`DeviceControlApprovalHook` 唯一承载)。注册条件 = 配置注入 (双闸
    /// `APEIRETH_ENABLE_DEVICE_SENSE` + `APEIRETH_ENABLE_DEVICE_CONTROL`)。
    pub device_control:
        Option<apeireth_tools_canonical::device::device_control::DeviceControlConfig>,
    /// Register the guard-watch tool (`tool.guard_watch`, K 轴 L5 看家骨架).
    ///
    /// 守护面: 守护规则 (`guard-rules.json` StoredDoc 拒开, 规则 = 设备状态阈值 /
    /// 时序条件) + 巡检判定 (白名单设备只读感知) + 告警账 (`guard-alerts.jsonl`
    /// append-only) —— 告警 = 关怀候选高优先 (打扰纪律守护例外档)。**只看不动**:
    /// 告警不触发任何设备动作 (动手仍走 [`Self::device_control`] 三拍人批)。
    /// 注册条件 = 配置注入 (双闸 `APEIRETH_ENABLE_DEVICE_SENSE` +
    /// `APEIRETH_ENABLE_GUARD_WATCH`, 默认关 fail-closed)。默认关 (opt-in)。
    pub guard_watch: Option<apeireth_tools_canonical::device::guard_watch::GuardWatchConfig>,
}

impl Default for ProductionModulesConfig {
    fn default() -> Self {
        Self {
            memory_recall: true,
            repo_map: true,
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
            http_write: None,
            web_search: None,
            calendar: None,
            process_tree: false,
            content_feed: None,
            care_schedule: None,
            voice_speak: None,
            backup: None,
            task_board: None,
            context_budget_chars: None,
            bio_play: false,
            bio_care_candidates: false,
            mcp: false,
            mcp_data_dir: None,
            mcp_registered_tools: 0,
            mcp_bridge: false,
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
