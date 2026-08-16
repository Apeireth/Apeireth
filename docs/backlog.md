# 调研/审计 Backlog 台账

> 规范 00 (文档同步自觉) 的落地载体: 凡调研/审计发现的、当下不做的项, 必须显式登记于此,
> 不得散落在聊天记录里丢失。本文件是唯一权威台账, 完成即划 ✅ 并注明提交/文档位置。

## 审计来源索引

| 代号 | 审计 | 日期 | 结果去向 |
|---|---|---|---|
| A1 | 代码 TODO 全量审计 (mempalace/VCP/Zep/Mem0 等借鉴点落地核对) | 2026-08-16 | 本节 |
| A2 | Handoff 交接审计 (docs/CONTEXT-HANDOVER.md 逐项核对) | 2026-08-16 | 本节 |
| A3 | 记忆域深度调研 (memory-research.md §五 backlog) | 2026-08-16 | 本节 |
| A4 | C3 v2 alpha 遗留盘点 (22 项核实) + 上轮自检 21 报告吸收 | 2026-08-17 | 本节 (编号 25-47: P1=25-29 / P2=30-37 / P3=38-45 / P0=46-47, 46/47 后置编号因 23/24 已被 C2 压测自检占用; 报告 reports/06da84cc-…-technical_writer2-report.md) |

## 已完成项 (✅)

| 项 | 来源 | 完成方式 | 证据 |
|---|---|---|---|
| ✅ 记忆分层排名 (imp×3+access×0.3+group+recency) | A3 记忆 v2 升级包 | memory_extractor.rs rank_memory_entries | 提交 + 单测 |
| ✅ 提炼器对账化 (ADD/UPDATE/DELETE + tomb 逻辑删除) | A3 (Mem0 借鉴) | memory_extractor.rs apply_reconcile | 提交 + 单测 |
| ✅ 反思触发 (周期 OR importance 和 > 150) | A3 (反思机制) | reflection.rs importance_surge | 提交 + 单测 |
| ✅ 做梦去重 (文本级近重复) | A3 (做梦质量) | dream.rs dedup_textual | 提交 + 单测 |
| ✅ 滚动摘要持久化 + sum-* 链 | A3 (上下文管理) | companion_serve.rs summarize_dialog | 提交 |
| ✅ 事实图 (双时态边 + 带权链接 + crawl 检索 + 注入) | A3 (Zep+A-MEM) | memory_graph.rs | 提交 + 3 单测 |
| ✅ 状态感知 (时刻/节律/目标/约定/情绪注入) | A2 模块 1 | companion_serve.rs inject_state | 提交 |
| ✅ 目标驱动 (5 工具 + 权限洋葱) | A2 模块 6 | goal_tools.rs + tool_bridge.rs with_goals | 提交 + 单测 |
| ✅ 主动送达 (SSE 广播 + Lark) | A2 模块 4 | MultiSink | 提交 |
| ✅ 上下文滚动摘要 | A2 模块 3 | companion_serve.rs | 提交 |
| ✅ 深度反思 (ReflectionReflector trait) | A2 模块 5 | reflection.rs | 提交 |
| ✅ 统一注入管线 ContextAssembler (核心块保护 + 预算截断) | A3 (L0/L1 前置) | context.rs | 提交 986358e + 3 单测 |
| ✅ CompanionApp 装配器 | A3 审计结论 ★5 | assemble.rs (注入管线/提炼调度/滚动摘要/自成长/3 新 LLM trait) | 提交 cdb6b62 + 7 单测; example 1683→1100 行 |
| ✅ L0/L1 always-loaded 渐进加载 | A1 #1 (mempalace §5.6) | CompanionApp with_identity (L0) + with_essential_budget (L1, essential-*/高 importance) | 提交 cdb6b62 + 单测 |
| ✅ 9 organ 人格化深化: 情绪→语气 + 审议→措辞 (team-work-doc §4 A3) | release-plan 偏差表 🟡 | tone.rs 三层器官语调 (ResponseStyle 7 档确定性映射 + 审议加权分/置信度 4 档强度, 非法输入返 ToneError) + ToneRefiner LLM 措辞 trait 口; organs.rs tick 捕获 DeliberationEcho + AwakeCompanion::tone() | 提交 70110a54 + b5ce015d, 10 个新测试; 未接: ToneRefiner 实现 + 渲染层动态注入 (0 装标注) |
| ✅ integration worktree 过期 (落后 1053) | A4 (DO1 b7f49cfe 自检) | C3 盘点复核实: 已与 master 完全同步 | `git rev-list --left-right --count master...team/e8de47ae-…/integration` = 0 0 (2026-08-17) |
| ✅ B2 矩阵 known-debt 清理: memory semantic cfg 门控 | B2 自审报告 (e3fad97e) case 5 | semantic.rs 向量路径 (SemanticIndex + imports + tests) 挂 `semantic` feature, 纯件 (EmbedFn/HashEmbedder/EmbedderIdentity/episode_uuid) 无条件; lib.rs semantic_persist/user_profile 模块 + 4 向量方法门控; `--no-default-features` 转绿 | 任务 54ed4c7d; 矩阵 case 5/7 PASS (logs/assembly-matrix.log) |

## 待办项 (按优先级)

> **2026-08-16 backlog 全清**: 全部 ⬜ → ✅ (主人拍板"排队的全做了, 全干完")。
> **2026-08-16 新增**: VCP 新版调研 (Rust 重写 + 84 插件) 进行中 — 可吸收点登记见下。
> **2026-08-17 新增 (A4)**: C3 v2 alpha 遗留盘点 (22 项重核实: 12 ✅ 达成/已解决 + 7 ❌ 产物失传 + 1 ⚪ 不可核实) + 上轮自检 21 份报告吸收 → 编号 25-47 (P0 两项为 46/47, 因 23/24 已被 C2 压测自检并行占用); 盘点报告 reports/06da84cc-848a-4087-b42f-2679d6c6c4d0-technical_writer2-report.md。

### 新调研跟踪

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| N1 | VCP 新版调研 (rust-vexus-lite + 89 插件 + 核心 modules) | 主人 2026-08-16 指示 | 源码 research/source/vcptoolbox (从 Downloads 迁入工作区, git 排除); Rust 记忆层 (RiverMemo V3) + 89 插件 manifest 已核实; 可吸收清单已并入 team-work-doc §8 | ✅ 调研完成 (team-work-doc §8.2/8.3/8.4) |
| N2 | OneRing 统一上下文账本 | N1 发现 | 跨前端统一时间线 — 并入 A2 (continuity 锚点升级) | ⬜ 并入 §4 A2, 待实施 |
| N3 | DigitalOracle 金融数据源 | N1 发现 | 预测机套件旗舰数据源候选 (含预测市场源) | ⬜ 并入 §5.2, 待实施 |
| N4 | ThoughtClusterManager 元自学习 | N1 发现 | AI 思维链文件 + 元自学习 — 并入记忆域深化包 | ⬜ 并入 §5.1, 待实施 |
| N5 | artifact_sig 内容寻址缓存 | N1 发现 (Rust 层) | semantic/图资产"内容签名→跳过重算"门禁 | ✅ 提交 f8245f28 (流水线整合): semantic_persist.rs — artifact_sig (SHA-256 手写, NIST 向量锚定, 0 新依赖) + artifact_gate_decision 五条失效规则 (无记录/内容变/normalize stale/schema stale/Hit) + reindex_all 门禁全量重建 (clear+set_dim+upsert_batch) + .artifact_sig.json sidecar; 8 测试四路径全绿 (cargo test -p apeireth-memory -j 4, 报告 reports/5f492ccb-…-database_engineer2-report.md) |
| N6 | Intrinsic Residual 锚增益 | N1 发现 (Rust 层) | memory_graph 节点"特异性"信号 (与 importance 正交) | ✅ 提交 ab777c2: memory_graph.rs 实体逆频特异性 + 组合排序权重可配 (GraphRankConfig) + 增量计数维护 + crawl 残差锚增益; 7 单测全绿 (报告 reports/2a1e262e-2f0f-458e-b5f0-130b1e232834-database_engineer-report.md) |
| N7 | 查询形态学 softmax | N1 发现 (Rust 层) | 驱动 CRAWL 深度/检索模式切换 (纯函数) | ✅ 提交 08c6f00d: morphology.rs 纯函数 (特征→softmax→档位/期望预算) + assemble.rs inject_memory 一处挂接; 10 单测 (确定性/空查询/超长/温度/分布归一, rustc --test 独立全绿; 全 crate 测试被并行 WIP 阻塞, 见报告); env APEIRETH_MORPHOLOGY_TEMPERATURE |
| N8 | generation 绑定观测缓存 | N1 发现 (Rust 层) | 查询管线中间产物复用 + 防跨代脏读 | ⬜ P0, 待实施 |
| N9 | 提示词装配引擎 (占位符变量宇宙) | N1 发现 (插件扫描) | messageProcessor 范式: 特权角色+单次展开+环检测+分型变量源 — Apeireth 空白区最高价值 | ⬜ P0, 待实施 |
| N10 | 宽松文本工具协议层 | N1 发现 (插件扫描) | vcpLoop TOOL_REQUEST 语法: 始末/ESCAPE/模糊匹配/archery/思考块剥离 → tool-runtime 增强 | ✅ 代码被流水线收编提交 fad23d81: tool-runtime 新增 text_protocol.rs (5 机制字段级移植 VCP vcpLoop: 始末语法+ESCAPE 防注入+模糊标记+archery 分流+思考块剥离) + executor.rs execute_separated/ArcheryHandle (fire-and-forget); cargo test -p apeireth-tool-runtime -j 4 全绿 (lib 112 + 集成 20 + doctest 2, 含 N10 新增 19 测); 0 新依赖; 自审报告 reports/77b9efce-edba-4452-b7b3-92b7fa3debda-backend_engineer2-report.md |
| N11 | foldProtocol 分级显隐 | N1 发现 (插件扫描) | context-fold 增强: FoldBlock 数据模型 (同文档分级+语义阈值展开) | ✅ 提交 1f5d2fd: context-fold 加 fold_block.rs (FoldBlock serde 模型+行标记解析+相似度≥阈值展开+收纳提示) + semantic.rs (语义折叠, 记忆域深化 §5.1); 45 测试全绿; 自审报告 reports/1d7bc7ee-*-code_reviewer-report.md |
| N12 | 语义模型路由 + 推理归一化 | N1 发现 (插件扫描) | gateway 层: 语义选模型+容灾链; 13 别名推理字段→think 块 | ✅ 提交 5fa725e6 (gateway semantic_router: 虚拟模型名+意图选模型+容灾链, Embedder/ModelExecutor trait 口, 17 测试) + 350c0255 (provider reasoning_adapter: 实为 12 别名, 台账"13"系笔误已按 VCP 源码核实; 片段级去重+think 块+白名单下发+http_dispatch 接线默认关, 22 测试); 集成合并曾冲回本行, 2026-08-17 复登记; 自审报告 reports/d6bc5357-bbc8-4ad4-aa7a-748ff67d7c9d-mcp_integration_expert-report.md |
| N13 | apeireth-guard 补 env 行级 + 密钥 token 模式脱敏 | team-work-doc §8.3 toolResultPrivacyGuard 行 (团队任务 ae12d9eb) | pii.rs 新增 SecretToken (sk-/sk-proj-/xox*/ghp_/github_pat_/glpat-/AKIA 7 类前缀) + EnvSecret (敏感键名 KEY=VALUE/KEY: VALUE 值部); redact_text 修重叠匹配错乱; organ_kani_proofs 6→8 类 | ✅ 完成: cargo test -p apeireth-guard 59 全绿 + gateway 84+7 全绿; 0 新依赖; 报告 reports/ae12d9eb-fe0c-4267-8f23-b225880430d1-security_reviewer2-report.md |
| N14 | 工作区脏树: apeireth-companion 编译失败 (他人未提交改动) | 安全审查2 任务 ae12d9eb 期间发现 | companion lib 编译错: SqliteMemoryStore 缺 put_episode/recent_episodes 方法 + unstable result_option_map_or_default; 涉及 memory_graph/capability/reflection 未提交改动 (非本任务引入) | ⬜ 待 Leader 指派相关成员处理 |
| N15 | dynamicToolRegistry 预算化 (注入注意力预算 + 分类四级降级链) | team-work-doc §8.4 P1 可吸收清单 (团队任务 4b2da00a) | apeireth-tool-registry 新增 injection.rs (render_injection: light list 一行式清单 + 仅相关工具展开详情 + 超预算裁剪 展开段→轻清单尾行→硬切留 TRUNCATION_HINT 提示, 16000 字符上限) + chain.rs (ClassifyChain 自定义→小模型→RAG→关键词 四级降级, ClassifyStage 记录决定级; 小模型/RAG 级 Option<Arc<dyn Classifier>> trait 注入口未接真模型如实标注, 关键词级 HeuristicClassifier 实装, CustomMapClassifier 自定义级实装); impl Classifier → 可直接给 register_with_classifier | ✅ 完成: 提交 8b6a825d; cargo test -p apeireth-tool-registry -j 4 全绿 (139 lib 测试, 新增 19: 预算内/超预算/空表/极小预算/仅相关展开/四级降级各路径); 报告 reports/4b2da00a-9556-4d9c-9420-06aa23b91272-mcp_integration_expert2-report.md |
| N16 | §5.1 记忆主题分组 + 主题索引注入 (VCP SemanticGroupManager 精神) | team-work-doc §5.1 记忆域深化包 机制② (团队任务 a227fc3f) | apeireth-companion 新增 topic_groups.rs (确定性分组: CJK bigram+拉丁词 token, 停用词切分点防桥接误并簇 → 贪心聚簇 → 主题名=最高频 token; build_topic_index 预算感知索引块 600 字, 每主题名称+条目数+代表条目, 超预算砍尾行留收纳提示) + assemble.rs memory_block 一处挂接 (主题索引+记忆证据块合并, 不另立平行注入系统) | ✅ 完成: 提交 17483af0 (3 files +347); 独立验证 rustc --test topic_groups.rs 8/8 全绿 (分组正常/空记忆/单主题/停用词/预算截断/确定性); crate 级测试被他人 WIP 阻塞 (HEAD 缺 prompt_assembler.rs, N9 进行中) 如实标注; 报告 reports/a227fc3f-f412-4a6e-b7d7-eea6cac30b5f-mcp_integration_expert2-report.md |
| N19 | toolApprovalManager 增强 (命令级粒度+静默拒绝+结构化拒绝) | team-work-doc §8.3/§8.4 吸收清单 (团队任务 fe468acf) | apeireth-tool-approval: ApprovalListRule (`Tool:command` 审批键, 命令从 args command/command1..N 提取, specificity 2>1 同级静默优先 — VCP considerMatch 字段级) + `::SilentReject` 静默拒绝 (silent=true 不打扰 AI, 审计台账 silent_rejection_audit 留痕) + 结构化拒绝 wait_for_approval_outcome → ApprovalOutcome/Rejection `{rejected_by_user, error_type}` (rejected_by_user/approval_timeout/policy_deny/channel_unavailable 四错误码) + ApprovalRule trait 加 silent_on_reject/matched_command 默认方法 + CallRecord 审计字段; check/wait_for_approval(bool) 签名与 ApprovalDecision 变体零改动 (消费方零破坏); 洋葱安全红线: 高危仍 RequireApproval 走主人批准, 无通道 fail-safe 拒绝 | ✅ 完成 (任务 fe468acf): cargo test -p apeireth-tool-approval -j 4 全绿 (106 单测+39 集成+1 doctest); 0 新依赖; 报告 reports/fe468acf-7515-48c9-9744-a197206ef5ab-security_reviewer-report.md |
| N20 | ApprovalBridge silent/matched_command 透传 | N19 实施中发现 | PolicyVerdict (tool-runtime) 无 silent/matched_command 字段, bridge 侧静默标记仍是已知丢失 (approval_bridge.rs 注释已载) — 待 N10 后续/tool-runtime 增强时补 ctx 字段 | ⬜ P2, 并入 N10 后续 |
| N16 | **团队协作三件套接线 (spectrai 机制落地)** | 主人 2026-08-17 全 workspace 孤儿体检 (0 外部 Cargo 引用) | apeireth-team-lead (Orchestrator 8 调度工具, 13 测试全过) + apeireth-agent (Agent 管理, 87 测试) + apeireth-supervisor (进程监督树, 76 测试) — 翻译完毕但从未被消费: 接进 companion (执行体 = 插件形态, 挂 ToolBridge + apeireth-bus L4 跨进程), 补 send_to_agent/wait_agent/worktree 占位实现; **主人指示: 团队当前批完成后下一批优先干这个** | ⬜ 下一批 P0 (主人拍板) |
| N17 | **工具子 crate 装配 (9 个 0 引用)** | 同上孤儿体检 | apeireth-tool-shell / tool-fetch / tool-browser / tool-codesearch / tool-image-gen / tool-image-process / tool-search / tool-filesystem 等 — 独立实现齐全但 apeireth-tools::register_all 未装配: 逐个核实能力 → 挂 ToolBridge 注册 + 白名单 + CapabilityCatalog (与 N16 同批)。**边界 (主人 2026-08-17 拍板, 防团队干偏)**: ① 禁止合并成一个大工具 crate (工具=插件单元, 保持独立 crate = 热插拔 + 依赖隔离 + 社区贡献拼积木); ② 装配必须统一走 `Tool trait + ToolRegistry.register + ToolBridge` 三件套, 禁止各工具自写调用/执行方式 (集成而非分立); ③ 每个工具 crate 提供 register 函数, 统一注册进同一 registry | ⬜ 下一批 P0 (随 N16) |
| N18 | **新 crate 必须声明消费方 (规范)** | 孤儿体检暴露机制漏洞 | 审计搜 TODO 模式抓不到"翻译了未接线"的 crate: maintenance-guide 加规范 — 新建 crate 必须登记消费方 (谁依赖/谁装配), 缺消费方的 crate 在台账显式标注"独立待装配"; 孤儿体检 (0 引用扫描) 纳入定期检查 | ⬜ 随 N16 批次落地 |
| N21 | **apeireth-credentials (统一 API key 管理)** | 主人 2026-08-17 设计蓝图对照 (R136 计划: "API key 管理统一走 apeireth-credentials crate (已存在)" — 实际不存在) | 各插件/工具目前各读 env: 新建 credentials crate (密钥安全存储/按服务名读写/权限洋葱衔接 master token, 0 假装: 不存明文到日志), 工具装配 (N17) 时统一接入; 与 N17 同批 | ⬜ 下一批 P0 (随 N17) |
| N22 | **ShellPreset (shell 预设命令机制)** | 主人 2026-08-17 设计蓝图对照 (R136: "VCP preset 机制 (preset:预设名?参数) 值得保留 — 减少 LLM 记忆成本") | tool-shell 无实现: ShellPreset { name, command_template } + 白名单预设 (预设名展开为完整命令, 参数走模板) — 与 N17 装配同批 | ⬜ 下一批 P0 (随 N17) |

### P0 — 近期做 (机制缺口, 高价值)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 1 | CompanionApp 装配器 | A3 审计结论 ★5 | companion_serve.rs (~1600 行) 装配逻辑抽进 lib: 注入链/提炼调度/工具桥/多 sink 统一为 CompanionApp::new(...).start(); example 变薄, TUI/CLI 可复用 | ✅ 提交 cdb6b62 |
| 2 | L0/L1 always-loaded 渐进加载 | A1 #1 (mempalace §5.6) | Identity (~100 token) + Essential Story (~500-800 token) 常驻; 与 ContextAssembler core 块天然契合, 挂 context.rs | ✅ 提交 cdb6b62 |
| 46 | Dockerfile COPY crates 互覆盖修复 | A4 (DO2 af2676fa W1) | `COPY crates/apeireth-*/Cargo.toml ./crates/` 同名互覆盖且不建 member 子目录, dummy 依赖缓存 build 大概率失效/失败 — 发布产物阻塞级; 本机无 docker 未实测, 需 buildx/有 docker 环境验证 | ⬜ P0, 待实施 |
| 47 | compose POSTGRES_PASSWORD 强制外部注入 | A4 (DO2 af2676fa W3) | docker-compose.yml `POSTGRES_PASSWORD:-secret` 默认弱密码且 DB URL 内联, 上线前必须禁止默认值 | ⬜ P0, 待实施 |

### P1 — 计划内 (成本明确)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 3 | Normalize 版本 schema | A1 #2 (~1 天) | semantic_persist 加 SEMANTIC_NORMALIZE_VERSION, 换 chunk 规则后识别 stale 向量 | ✅ 提交 5bd0d4e: CURRENT_NORMALIZE_VERSION + normalize_is_stale + .normalize.json sidecar + needs_reindex; 3 测试 |
| 4 | 5 lifecycle hooks | A1 #5 | UserPromptSubmit / SessionStart / SessionEnd / PostToolUse / Stop, 挂 apeireth-bus | ✅ 提交 9c7a5cf (bus lifecycle.rs) + 2d07c604 (companion_serve 接线: SessionStart/UserPromptSubmit/PostToolUse 真实时机) |
| 5 | 图持久化后端 Kùzu | A1 #3 (~1.5 周) | memory_graph 目前进程内存; 换 Kùzu 持久化, trait 接口已备 | ✅ 提交 b8fdd455: GraphBackend trait + SqliteGraphBackend + with_backend 注入 + GraphQuery 结构化查询; Kùzu 物理后端因本机无 cmake + GitHub 墙不可构建, trait 口已备 (0 装 PASS 如实标注) |
| 25 | cargo fmt 全仓修复 + nightly 工具链 | A4 (QA2 397a85ec) | cargo fmt --check 不通过: 1154/1588 文件 (72.7%) 不合规 (stable 口径); 先修本机 nightly (`rustup toolchain install nightly --force`), 再 `cargo +nightly fmt --all` 一次性修复; Windows 侧用分批 rustfmt 命令规避 error 206 (命令见 QA2 报告) | ⬜ P1, 待实施 |
| 26 | 版本号口径统一 (release 前必须) | A4 (TW2 f3f9fa0c + AR2 b74fc48b) | RELEASE_NOTES v1.0.0 标题 ≠ workspace 1.2.0; CHANGELOG 顶部日期条目未归 semver + R131-R178 未归版本条目; RELEASE_NOTES 行号引用漂移 (:246→:224); 11 个活动 crate 硬编码版本; ROADMAP 头部双轨表述未标明 + 进度止于 R127 未同步 R178 — 需 Leader 拍板单一口径 | ⬜ P1, 待 Leader 拍板 |
| 27 | cosign.pub 生成 + release 工具链预装 | A4 (AO2 b88db7ed) | docs/security/cosign.pub 缺失 → cosign-sign-all.sh/cosign-verify.sh 必失败; 发布环境需预装 cosign/gh/jq (本机均缺) | ⬜ P1, release 前置 |
| 28 | .gitignore 密钥类加固 | A4 (SEC2 97a4bfce) | 追加 `*.pem` `*.key` `*.p12` `*.pfx` `id_rsa*` (现仅针对性忽略 `**/cosign.key`, 实测 secret.pem 不被忽略) + 补 `_research_mem/` | ⬜ P1, 待实施 |
| 29 | README crate 计数修正 | A4 (AR2 b74fc48b) | README 写「81 (80 顶层 + 嵌套)」, 实测 workspace members=82 (81 顶层 + 1 嵌套) | ⬜ P1, 顺手修 |

### P2 — Backlog (有价值, 时机未到)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 6 | telemetry cache 接线 | A3 | memory-extensions 7 provider 已实装未接线 | ✅ 提交 c5849829: MemoryProviderRegistry + ProviderFactory (9 provider, 7 env 常量) + CachedMemoryProvider cache 语义; 7 集成测试; 接线结论: 主链路默认不走 extensions 属有意决策 (0 装 PASS) |
| 7 | LATS/MCTS 规划搜索 | A3 (cognition) | 决策时做树搜索, 需要真模型预算 | ✅ 提交 5ec4a17: cognition/planning.rs MCTS (UCT/扩展/模拟/回溯, xorshift64* 确定性), StateEvaluator = LLM 评估注入点; 5 测试 |
| 8 | lightmemo 双轨语义决策 | A3 | episodes=事实源, L1-L4=分层索引 | ✅ 提交 5ec4a17: dual_track.rs (同 id 去重 episodes 优先/来源诚实标注/多模式加分); 6 测试 |
| 9 | tree-sitter 代码记忆 | A3 | 代码结构级记忆, 超出当前文本记忆域 | ✅ 提交 d2054e5a: codesearch feature tree-sitter (默认关) — 全语法树 Rust 符号提取, 与 regex 版共存同 API; 4 测试 |
| 10 | CRITIC 反思带工具调用 | A3 | 反思可调用工具验证, 依赖工具桥扩展 | ✅ 提交 08079cb7: critic.rs (声明提取 + ClaimVerifier trait + ReflectionCritic + CritiqueReport); 6 测试 |
| 11 | Telegram 送达 | A2 (Sink 扩展) | 第三 Sink, LarkSink 模式复制 | ✅ 提交 3e0ab1a6: TelegramSink (from_env/from_env_with, 真 HTTP, APEIRETH_TELEGRAM_BOT_TOKEN/CHAT_ID) + companion_serve 接线; 5 测试 |
| 12 | ONNX 本地嵌入 | A1 | 本地 embedding, 去 MiniMax 依赖 | ✅ 提交 b8fdd455: onnx.rs tract 纯 Rust 推理 (feature onnx 默认关) + APEIRETH_LOCAL_EMBEDDER 选择 + hash 降级链; 4 测试双配置 |
| 13 | UncertaintyResolver 接真 (oracle) | A2 | 目前 stub, oracle-suite 就绪后接线 | ✅ 提交 3e0ab1a6: CalibratedResolver (Brier + BetaBinomial 校准, Wilson 区间, 0 历史→0.5 诚实); 4 测试 |
| 14 | SDK 三通道 stub | A2 | 主人拍板跳过 Node, 其余通道待定 | ✅ 已实装 (R122-8): python/node/c 三 cfg-gated 桥接均有真函数; 台账确认完成 |
| 30 | apeireth-mcp 文档对齐 | A4 (MCP1 19809d9e) | ①lib.rs/Cargo.toml 引用 `docs/v2-strategy/05` 悬空 (已迁 docs/stage2/05-EXECUTION-NOW.md) ②lib.rs 头部过时 (称 SSE/resources/prompts 未实现, 实际均已实现且测试通过) — LOCKED crate, 需走对应修改流程 | ⬜ P2, 待实施 |
| 31 | CODEOWNERS 悬空 crate 清理 | A4 (MCP1 19809d9e) | CODEOWNERS:49-51 声明 apeireth-mcp-ssh/winrm/relay-image 目录不存在 (C3 复核仍在) — 清理条目或补建 crate | ⬜ P2, 待实施 |
| 32 | 仓库卫生: 误产物 + db 泄漏清理 | A4 (AR1 91bb7d42 + DB1 e5a173c8 + DB2 c7e494b3) | ①`git rm ersXXXApeireth-rust` (11KB ANSI git log 转储, R125 迁仓误产物, 仍被跟踪) ②删 crates/apeireth-memory.db{,-shm,-wal} (486KB WAL 泄漏进源码树, 未跟踪) + .gitignore 补 `crates/*.db*` | ⬜ P2, 待实施 |
| 33 | 孤儿 crate 确认 + dev-dep 治理 | A4 (AR1 91bb7d42) | ①12 个零内部消费者 lib crate 待负责人确认去留 (provider/cron/experience/environment/config/state/naming-v05/livekit/blueprint-impl/library-governance/voice/context-fold) ②tool-fetch 自引用 dev-dep (Cargo.toml:28) 修复 ③verify/supervisor/sovereignty 三角 + tool-runtime↔tool-approval dev-dep 回环边界腐化, 建议抽公共接口 | ⬜ P2, 待实施 |
| 34 | assemble.rs chrono unwrap DST 修复 | A4 (CR2 03cf86e9) | assemble.rs:399 `and_local_timezone(...).unwrap()` 在 DST/时钟回拨时歧义 panic — 一行改 `.single()`/Option 兜底; 4 处 Mutex poison 风险仅记录不阻塞 | ⬜ P2, 待实施 |
| 35 | v2 alpha 失传产物诚实标注 | A4 (C3 盘点) | 7 份验收报告 + 09-ADDENDUM + V2-INDEX + 07-V2-BASELINE 从未入 git 历史 (不可恢复, 详见 C3 报告 §二) — 在 RELEASE-NOTES-v2.0.0-alpha 对应位置加注"产物已失传"或 Leader 决策重写; 不重建伪造 (0 装 PASS) | ⬜ P2, 待 Leader 决策 |
| 36 | round15-03 丢失内容恢复决策 | A4 (MCP2 380a2218) | 嵌套侧 CHANGELOG +28 行 / ROADMAP +43/-5 行未进根版本; blob 9aa1791c/0efb4322 可随时恢复 (commit 8bcad630) — 已通报 Leader | ⬜ P2, 待 Leader 决策 |
| 37 | 根 tests/ 死代码归档 | A4 (QA1 5c888b1c) | 根 tests/ 12 个 .rs 不被任何活跃 crate 编译 (纯 workspace 无 root package), README 自述占位 — 清理或归档, 防误以为在执行 | ⬜ P2, 待实施 |

### P3 — 归档/低优先 (做了更好, 不做不欠)

| # | 项 | 来源 | 说明 | 状态 |
|---|---|---|---|---|
| 15 | user_profile 误导注释修正 | A1 | 已被偏好库取代, 仅注释误导, 修注释即可 | ✅ 注释已如实修正 (未删除 — 仍是公开 API 路径, 主链路已由 pref-* 库取代) |
| 16 | microsandbox 物理层 | A1 | Windows KVM 风险高, 不建议当前做 | ✅ 提交 5ec4a17: Job Object 加固 (KILL_ON_JOB_CLOSE 防孤儿 + 进程树管辖, 非 KVM, OS 原生); tool_bridge 接线, 失败不阻断 |
| 17 | Sandboxie/landlock 物理隔离 | A1 | 同上, 观察 | ✅ Windows 侧由 Job Object 覆盖 (#16); landlock 是 Linux 专属 — 已如实标注平台机制 (Windows 完成, Linux 留 OS 机制) |
| 18 | OTel 可观测性 | A1 | 归档建议, 现有 [llm]/[daemon] 日志够用 | ✅ apeireth-telemetry/otlp.rs: OtlpSink trait + NoopOtlpSink (is_implemented=false) + JsonLinesOtlpSink; 7 测试, 0 新依赖 |
| 19 | OpenAPI/axum 升级 | A1 | 依赖升级, 无功能缺口 | ✅ 评估完成: axum 0.8/reqwest 0.13/tower-http 0.7 均破坏性升级 → 有意维持 (决策已写入 Cargo.toml + README 依赖版本决策表) |
| 20 | self_update OTA | A1 | 发布流程成熟后再做 | ✅ 已实装 (R223): 真实二进制替换 + 备份 + 原子切换 + 回滚; 台账确认完成 |
| 21 | TUI voice/eye stub | A2 | 前端占位, 不影响机制 | ✅ 修复真 bug: Synthesize 假装成功 → 返回 Unsupported; eye 占位诚实标注; 新测试 |
| 22 | Windows Hello 真绑 | A2 | 生物识别绑定, 需硬件调研 | ✅ 提交 2d07c604: hello.rs 机制口 (detect_hello_capability reg query NGC + HelloBound trait; 0 装 PASS 不假装已绑定); 3 测试 |
| 38 | mkdocs extra.css 资产补齐 | A4 (TW1 fba46921) | mkdocs.yml extra_css 引用 docs/pages-source/assets/css/extra.css 不存在; strict:true 构建会告警 — 补最小 extra.css 或删 extra_css 段 | ⬜ P3, 待实施 |
| 39 | companion 6 clippy 警告 + CI fmt 核对 | A4 (CR1 abf185d2) | cast_lossless×3 (memory_extractor.rs:299/session_log.rs:53/simulation.rs:223) + manual_let_else×3 (session_log.rs:94/tool_bridge.rs:786/796) 机械修; 另核对 CI fmt check 是否真 nightly (否则 tantivy 5 项 nightly-only 规则形同虚设) | ⬜ P3, 待实施 |
| 40 | deny.toml 过期 skip 清理 | A4 (SEC1 02cd644d) | unnecessary-skip (heck 等已单版本) + unmatched-skip (async-channel 已不在依赖图); 过期 skip 会掩盖未来真实多版本问题 | ⬜ P3, 待实施 |
| 41 | rust-toolchain.toml pin 版本 | A4 (BE1 5cb3d314) | 现仅 channel=stable 未 pin 具体版本, stable 升级会导致 CI 与本地漂移; 建议 pin 1.97.1 (可重现构建决策需 Leader 拍板) | ⬜ P3, 待决策 |
| 42 | git 卫生: stash/zombie worktree/log | A4 (DO1 b7f49cfe) | ①29 条历史 stash (round5~R122) 审计清理 ②僵尸 worktree r11-recover 需 `git worktree prune` (写操作待授权) ③reports/*.log 纳入 .gitignore (如 be2-cargo-check.log) | ⬜ P3, 待授权 |
| 43 | frontend/ 残留骨架清理 | A4 (FS1 c7b06a25) | frontend/ 仅存 tauri-prototype 残留 (砍前端决策的遗留, .gitignore:152 有记载); 清理需主人确认 | ⬜ P3, 待主人确认 |
| 44 | rust-ci.yml 重复 workflow 清理 | A4 (DO2 af2676fa W4) | rust-ci.yml 已标 deprecated 与新 rust.yml 并行浪费 runner; R25 注释"1 周后待主人拍板删" | ⬜ P3, 待主人拍板 |
| 45 | 数据目录标准化 + migration 口径统一 | A4 (DB1 e5a173c8 + DB2 c7e494b3) | ①数据文件落 crates/apeireth-memory.db 非常规位置, 建议迁标准数据目录 (需兼容旧文件搬迁) ②DB1 称"无 migration 框架"与 DB2 实测矛盾: apeireth-memory/src/migrations.rs 已有版本化迁移 (V1/V2 + schema_migrations), 其余 CREATE TABLE IF NOT EXISTS 散点 (continuity_link/dailynote/lightmemo) 未接入 — 统一口径并评估接入 | ⬜ P3, 待实施 |
| 23 | cargo fmt 卫生 (workspace 级在 Windows 不可运行) | C2 压测自检 (QA 2026-08) | 复现: `cargo fmt --check` 工作区级直接报 `文件名或扩展名太长 (os error 206)` (CreateProcess 32k 命令行上限); per-crate 可运行但确有未格式化 diff: apeireth-bench 37 / apeireth-memory 261 / apeireth-companion 533 文件 (仓库共 1603 个 .rs, 与上轮报告 1588 规模吻合). 本任务不改 (跨任务包文件) | ⬜ P3 卫生: 建议 CI 用 per-crate fmt gate (或分片脚本), 另择专项批量 cargo fmt 一次 |
| 24 | 向量检索 100k 语料延迟观察点 | C2 压测基线 (QA 2026-08) | SemanticIndex::search 为暴力线性扫描: 100→168µs, 1k→1.25ms, 10k→12.8ms, 100k→144ms/query (top_k=10, dim=32). 当前规模 (<10k) 无碍; 若单会话语义语料逼近 10 万条, 单次检索 ~144ms 将进入交互可感知区. 基线证据: reports/eaf24ba8-…-qa_engineer2-evidence/vector-out.txt | ⬜ P3 观察: 语料 >10 万条时评估 ANN/HNSW 索引 (apeireth-vector 已留 qdrant_compat 口), 非当下欠账 |
| 48 | FileOperator 参数契约是 `op` 非 `archery` — 旧 example 执行必失败 | C1 e2e (QA1 6dec693a, 2026-08) | 实测: `tool_orchestrator_e2e.rs` 用 `archery:<<<read>>>` 构造 FileOperator 调用 → 执行返回 `Tool call error: missing 'op' string` (crates/apeireth-tools/src/file_ops.rs:298-300 只认 args.op). e2e 测试已改用 `op:<<<read>>>` 全绿; example 文件本身未改 (QA 不改产品/examples 代码边界), 建议后端择机修正该 example | ⬜ P3, 待实施 |
| 49 | apeireth-companion WIP 编译阻塞观察点 (N2-N8 吸收项在建) | C1 e2e (QA1 6dec693a, 2026-08) | 2026-08 实测: companion 未跟踪新文件 (thought_cluster/prompt_assembler/continuity/job_object/diary 等) 先后触发 E0658 map_or_default / E0599 缺 Datelike / E0433 缺 rusqlite dep / E0521 闭包借用 / E0277 Send / E0308 类型不匹配, 并行成员迭代修复中 (QA 窗口内一度转绿又因新改动转红, 末态卡 diary.rs:145 BTreeMap collect); 阻塞期间 integration-e2e 完整版 10 场景隔离到 tests/pending_companion_wip/ (基线版 10/10 + 整 crate 76/76 全绿不受影响). companion 恢复后按该文件头部 3 步启用 | ⬜ 观察: 并行成员修复中 |

## 明确不做 (有意决策, 防再调研)

| 项 | 决策理由 |
|---|---|
| 语义向量库自研 | 已有 memory-extensions provider 接口, 自研无增益 |
| Node SDK | 主人拍板跳过, Rust 原生通道优先 |
| 图数据库自研 | 用 Kùzu, 不重造轮子 (见 P1 #5) |
