# Apeireth companion 维护指南（2026-08-16）

> **团队作战总纲见 [docs/team-work-doc.md](team-work-doc.md)**（三哲学 + 工程/文档规范 + 工作全景 + 插件生态矩阵）。
> 本文件是**维护用的活文档**：概念词典、模块地图、加新模块的规范、生命周期接线点。
> 改代码前先看这里，避免概念混用。
> **社区插件开发规范见 [docs/plugin-authoring-guide.md](plugin-authoring-guide.md)**（team-work-doc §5.6 官方交付：Plugin trait 用法 / 白名单与日常包 / 测试模板 / 卸载真清理 / 数据源模板 / 发布检查单）。

## 一、概念词典（澄清易混词）

### 能力栈（上位 → 载体 → 打包）
```
能力 (capability)  ← 上位概念: AI 能做的事 (可预定义, 也可 AI 自己长出来)
├─ 动作 (action)     = 主动动作形态 (问候/问进展/提议帮助/提醒, 动作空间)
├─ 工具 (tool)       = 基地工具形态 (recall_memory / save_memory / FileOperator...)
├─ 技能 (skill)      = 流程封装形态 (CapabilityKind::Skill, 如「换元陪练」)
载体: 插件 (plugin)  = 能力的可分发/可卸载载体 (注册工具+权限+生命周期)
打包: 套件 (suite)   = 插件组的官方打包 (一键装「完整能力」, 如教育/渗透/预测机)
      能力包 (pack)  = 80%→完全体的基础补全 (沙盒/审计/多通道/GUI)
```
**规则**: 对话/文档里「能力」泛指上位; 具体说形态时用 动作/工具/技能; 说生态时用 插件/套件。

### 编译期装配（B2: 三档 cargo feature）
- 清单: workspace 根 `suites.toml`（本体 crate 组 / 包 feature 映射 / 套件 crate 组）; 装配入口: `apeireth-cli` 的 `[features]`。
- 档 1 base: `--features base`（default; 核心 crate 无条件编译, 语义标记）。
- 档 2 capability packs: `local-intel`（→ apeireth-memory/onnx, 真门控）/ `gui`（→ apeireth-api/tui-dashboard, 真门控）; `sandbox` `channels` `audit` 为声明式标记（机制件已无条件编译/独立 crate, 未门控如实标注）。
- 档 3 upgrade suites: `suite-education` / `suite-pentest` / `suite-oracle`（编译期标记; 运行时走 companion::suites::SuiteCatalog 装配）。
- 验证: `scripts/check-assembly-matrix.ps1` → 日志 `logs/assembly-matrix.log`。

### 易混词对
| 词 | 含义 | 易混对象 |
|---|---|---|
| **权限包** (PermissionPack) | 授权凭证 (覆盖哪些工具/多久/预算) | **能力包** (CapabilityPack) = 发布物, 不同物! |
| **continuity_id** | 哲学锚点: 跨载体/重启稳定身份 (唯一真相) | **session_id / subject** — 工程里三者曾混用, 现在统一以 continuity_id 为锚, subject 只是它的别名 |
| **做梦** (dream) | 记忆整合的夜间周期 (合并+摘要写回) | **consolidation** (记忆巩固, 工程别名) |
| **反思** (reflection) | 4 阶段反思周期 (写回反思记录) | — |
| **涌现** (emergence) | 主动/能力「长出来」的机制 (非写死) | — |

### 生命周期词
- 白昼: 主动涌现 (节律+驱动+门禁) → 渲染 → 送达
- 夜间: 做梦 (6h 无互动 → 合并+摘要写回)
- 周期: 反思 (24h → 4 阶段写回)
- 演化: AI 提案能力 (propose_capability) → 宪法评审 → 批准 → 激活 → catalog 动态段 → 可部署为插件

## 二、模块地图（crates/apeireth-companion/src/）

| 模块 | 职责 | 关键接线 |
|---|---|---|
| emergence.rs | 涌现循环: 节律/驱动/门禁/反馈 | AwakeCompanion.loop_ |
| organs.rs | 全器官: 情绪/审议/演化/主权/洋葱; 情绪/审议既门控「是否开口」, 也经 tone() 调制「怎么开口」(情绪→语气、审议→措辞强度) | daemon.awake |
| actions.rs | 动作空间 + CapabilityCatalog (静态+动态) | 渲染层 |
| daemon.rs | 总装: 心跳/做梦/反思/送达 | CompanionDaemon |
| dream.rs | 做梦调度 (SleepCycle+DreamSubsystem+摘要) | daemon.dream |
| reflection.rs | 反思周期 (ReflectionCycleScheduler) | daemon.reflection |
| judicator.rs | 宪法评审 (LlmJudicator 按 E 层判案) | ToolBridge.with_judicator |
| tool_bridge.rs | 工具桥: 洋葱门/评审/权限/路径/隔离/spill/post 钩子 | 干活链路 |
| packs.rs | 权限包 (授权凭证) | ToolBridge.packs |
| security.rs | SecurityGate / SovereigntyGate | ToolBridge.gate |
| exec_worker.rs | 执行体隔离 worker (per-call 子进程) | ToolBridge.with_isolation |
| spill.rs | 工具结果溢出 (超大输出落私有文件) | ToolBridge.with_spill |
| continuation.rs | 续行快照 (原子写+崩溃恢复) | multi_turn 循环 |
| session_log.rs | 事件溯源会话 (append-only 日志+surface+崩溃修复) | 多轮循环 |
| goal.rs | Goal 状态机 (AI 长目标, 严格 fold+持久化) | 独立 |
| capability.rs | 能力提案状态机 (AI 自己长能力第一段): pending→approved→active→retired/**rolled_back** (部署后差评/失败率回滚留痕) + 回滚收据 | propose_capability 工具 / deploy.rs |
| deploy.rs | 能力演化回路后半段 (部署→监控→回滚): DeployChannel trait (mock 可测, 真执行体挂 exec_worker/sandbox=接线点) + DeployManager (监控登记: 调用计数/失败率/差评信号 + 预测线期限 VirtualClock) + 越限自动回滚留痕 | capability.rs / evolution_gate.rs LoopAction |
| plugin.rs | 插件机制 (生态最小单元) | ToolBridge.registry |
| suites.rs | 三件套目录+装配 (套件=插件组) | install_with_plugins |
| prompt_cache.rs | Prompt Cache 稳定化 (稳定前缀+动态单点) | 渲染组装 |
| tone.rs | 三层器官语调: Bond 关系基线 + 情绪→语气 (ResponseStyle 7 档确定性映射) + 审议→措辞强度 (加权分/置信度 4 档, 非法输入返 ToneError); ToneRefiner = LLM 措辞注入 trait 口 (实现未接) | 渲染层 / AwakeCompanion::tone() |
| daily_summary.rs | 每日摘要数据源 | §6.4 UI 后端 |
| clock.rs (core) | 虚拟时钟 (时间机制快进测试) | 全部时间敏感模块 |
| constitution_gate.rs | 结构化宪法硬门 (编译期规则表, 零成本, LLM 评审前) | ToolBridge (全风险级别) |
| memory_injection.rs | 反幻觉记忆注入 (闭世界证据: 编号列表+禁止声称记得) | 渲染层 |
| confidence.rs | Beta-Binomial 置信度 (数学化自信度) | capability / 自测 |
| evolution_gate.rs | 验证闸门流水线 (fix loop/no-progress/预算 fail-open/回滚收据) + LoopAction 回路挂接 (Promoted→部署/Rejected→回滚/fail-open→挂起) | 能力演化回路 → deploy.rs |
| oracle.rs | 预言机套件核心: WorldState/ScenarioEngine/Forecast+Brier+BetaBinomial 校准/DecisionEngine expectimax/ForecastRegistry | simulate/forecast 工具 |
| oracle_adapters.rs | 预测机套件数据源适配器 (N3, VCP DigitalOracle 精神): MarketAdapter trait (拉取→规范化→喂 oracle 可证伪预测登记) + CoinGecko 加密/美债 fiscaldata 宏观利率双旗舰 (免费无 key) + MockAdapter/FallbackAdapter 限流降级 + AdapterRegistry 热插拔 + ForecastPipeline (挂 ForecastRegistry, oracle.rs 0 改动, 基线元数据走 adapterfc- 事件) | 待接 ToolBridge (后续任务) |
| web_crawl.rs (tools) | Crawl v2: 并发 BFS+重试退避+限速 (调研驱动, 实战验证) | Crawl 工具 (9 号) |
| education.rs | 教育套件真内容: dx_check 规则层检查器 (忘换 dx/混用/缺微分/残留 x/根号模式表) + EducationDxPlugin (注册+授权, 卸载真清理) | dx_check 工具 |
| pentest.rs | 渗透套件真内容: recon_plan (计划编排+E-1 范围闸) + scan_report (nmap 行解析) + 双插件 (卸载真清理) | recon_plan/scan_report 工具 |
| gh_accel.rs (companion) + github_accel.rs (tools) | GitHub 加速插件: xiake.pro 节点池 → 本机并发实测 (2xx+PNG魔数内容验证) → 选最快 → 加速 URL/命令 (docs/ref-gh-accel.md) | gh_accel 工具 (插件注册) |
| audit.rs | 审计能力包: audit_log 工具 (留痕查询, masked 脱敏不还原, append-only) | audit_log 工具 (内置注册) |
| append_only.rs (memory) | HistoryStream 新增 list_recent (最近 N 条, 审计/摘要用) | 6 流共享 |
| preset.rs (tool-shell) | ShellPreset 预设命令模板 (TP4/N22, §10 官方包最后一件): 白名单预设登记 (git-log-recent/git-status-short/echo-text builtin + register 扩展) + argv 模板展开 (占位符 `{arg}` 独占槽位, 嵌入式注册即拒) + 参数独立 shell_words::quote 填充防注入 (split/quote 往返闭环, `;`/`&&`/`|`/`$()` 无法逃逸单 token) + PresetShell 挂既有 exec_sandboxed 执行链 (不自写 shell 调用); 敏感预设审批走既有 tool-approval/guard (不改本体) | PresetShell::exec_preset |
| experience.rs | 自成长 Level 0/1: 经验库 (场景/做法/结果/验证计数/EMA) + 达标促能力提案 (versioned chain, rev 单调) | save/list/verify_experience 工具 |
| principles.rs | 自成长 Level 2/3: 动态原则层 (AI 提案→主人 master token 批准→执行检查拦截) + 晋级候选导出 (内层=主人侧工程) | propose/approve_principle 工具 |
| approval_requests.rs | 授权请求机制: 工具被拒→待批请求 (apreq-*, 同参数去重) → 前端轮询展示+一键批准 (权限洋葱真实载体) | GET /v1/apeireth/approval-requests |
| panel_readonly.rs (apeireth-api crate) + assets/panel/ (companion) | **B1 Web 面板 v2**: 7 个只读面板端点 `panel_router(store)` (sessions/sessions:id/timeline/memory/streams/memory/episodes/graph/approvals/audit, 数据全真接 SqliteMemoryStore: 会话表/6 历史流/factg-图/link-链/apreq-授权/action_stream 审计) + 静态多页面板 (总览/会话/记忆/图谱 SVG/授权/审计, 原生 JS 无构建链, include_str! 内嵌) | companion_serve `/panel*` (静态) + `/v1/panel/*` (nest, 只读); 批准走已有 /v1/apeireth/grant, 0 新安全口; 升级点: N2 OneRing (会话) / GraphBackend 结构化 (图) |
| memory_extractor.rs | 通用记忆提炼器: LLM 提炼 facts/preferences/commitments/emotional/graph (带 importance) + Mem0 式对账 (ADD/UPDATE/DELETE, tomb 逻辑删除) + 偏好库 (pref-*) + active_episodes 过滤 | 对话后节流 + 6h 批量 |
| memory_graph.rs | 时序知识图谱 (Zep 双时态边 factg-*, rev 链内单调+无效化=max+1 新边=max+2) + A-MEM 带权链接/CRAWL (link-*, 规则重叠) + N6 Intrinsic Residual 锚增益 (实体逆频特异性×importance 组合排序, GraphRankConfig 权重可配; crawl 字符集残差锚增益; entity_counts 增量维护) + 注入【事实图】 | graph 三元组 + crawl 注入 |
| semantic_persist.rs (memory) | N5 artifact_sig 内容寻址缓存门禁 (VCP 吸收): SHA-256 内容签名 (纯手写, NIST 向量锚定) + 五条失效规则 (无记录/内容变/normalize stale/schema stale→重算, 全匹配→Hit 复用) + reindex_all 门禁全量重建 (clear+set_dim+upsert_batch 防脏读) | PersistentSemanticIndex::check_artifact / reindex_all + .artifact_sig.json sidecar |
| thought_cluster.rs | 思维簇管理 (N4, VCP ThoughtClusterManager 吸收): AI 思维链文件按主题聚簇落盘 (「簇」后缀目录 + 按日归档 {日期}-{序}.md + 链注册 meta_thinking_chains.json + 确定性编辑/检索) + ThoughtClusterReader trait 口 (元自学习: 反思/做梦回读历史思考链做"思考的再思考") | reflection/dream `with_thought_reader` 注入点; 写入侧 LLM 驱动留部署层 (0 装 PASS) |
| morphology.rs | 查询形态学 softmax (N7): 确定性文本特征 (长度/实体密度/疑问形态/分句/深度线索) → logits → softmax 分布 → 检索档位 (浅扫 1/标准 3/深爬 6) + CRAWL 期望预算; 纯函数同查询同档位, 温度可配 | assemble.rs inject_memory → crawl 预算 |
| diary.rs | 日记本中心 (§5.1 机制⑤, RAGDiaryPlugin 精神): 按日归档 `{YYYY-MM-DD}.json` (root+clock 注入) + 确定性检索 (日期范围/关键词子串) + 注入块 (近 N 日摘要, 预算截断) + DiaryInjector trait 口 (实接线延后 N14) | 注入链挂接待 N14 解锁 |
| reflexion.rs | 口头强化闭环 (E1, Reflexion 式): 失败轨迹登记 (决策拒绝/验证失败/经验失败三类, seq 序) + CRITIC 反思 (Critic trait LLM 口 0 装, RuleCritic 确定性规则版先行, 事实/教训/重试策略三段模板, reflected_until 水位幂等) + 反思记忆 (task_type 标签) + 同类重试注入 (精确>子串相似度, 预算截断); 实接线留公开口 (不改 reflection.rs 周期本体) | 事件实接线/注入消费侧待后续接线 |
| goal_tools.rs | 目标驱动 (模块 6): goal_create/status/complete/pause/block (严格状态机) | 5 目标工具 |
| context.rs | 统一注入管线 ContextAssembler: 有序块 + 总预算 + 核心块保护 + 单块 cap (identity/essential 常驻 core) | 注入链统一入口 |
| assemble.rs | CompanionApp 机制装配器 (审计 P0#1): L0 Identity + L1 Essential Story 常驻 (mempalace §5.6) + 注入管线 + 提炼调度 (run_extraction/extraction_due) + 滚动摘要 (summarize_dialog/summarize_due) + 自成长 (refine_experience/export_promotion_candidates) + LLM 调用点 trait (DeepRecall/DialogSummarizer/ExperienceRefiner) | serve/TUI/CLI 复用 |
| semantic_router.rs (gateway) | N12①: 语义模型路由适配件 (VCP semanticModelRouter 吸收): 虚拟模型名 (ApeirethModelAuto/预设名) + 意图选模型 (上下文加权向量×route 描述余弦相似度×阈值) + 容灾链 (命中+failoverPool→default→fallback 去重, dispatch 按链容灾); trait 口 Embedder/ModelExecutor (真实现留部署层); 0 假装: 未接 Gateway 帧管线 | gateway lib (N12) |
| reasoning_adapter.rs (provider) | N12②: 推理字段归一化适配件 (VCP reasoningContentAdapter 吸收): 12 别名递归提取→片段级去重→think 块包装→按模型白名单下发 + 出向剥离; http_dispatch 响应路径已接线 (默认关, env 显式开启) | http_dispatch (N12) |
| semantic.rs + fold_block.rs (apeireth-context-fold) | 记忆域深化 §5.1 语义折叠 (注入段按相关度评分, 低相关段折叠为摘要占位, 嵌入可 mock+确定性内置评分器, 无损展开) + N11 FoldBlock 分级显隐 (`[===vcp_fold:阈值===]` 行标记, 相似度≥阈值才展开, 未展开留"还收纳了 N 组"提示); VCP ContextFoldingV2/foldProtocol 精神 Rust 原生移植 | 注入段折叠后仍可过 fold() 预算截断, 与 ContextAssembler 协作不冲突 |
| prompt_assembler.rs | 提示词装配引擎 (占位符变量宇宙, backlog N9, VCP messageProcessor 范式吸收): 分型变量源 (VariableSource trait: identity/state/goals/memory/time) + 特权角色 (agent/toolbox 仅 system 展开, 系统标记 user 可配置) + AgentGuard 全上下文单 agent + ToolboxGuard 每种一次 + 循环依赖检测 (递归栈+深度上限) + assemble() 消费 ContextAssembler (预算→展开→复用预算语义重截断) | ContextAssembler 输出 (接线 serve 链路属后续任务, 0 装 PASS) |
| pii.rs + redactor.rs (apeireth-guard crate) | PrivacyGuard 文本脱敏: 8 类检测 (Email/Phone/Ssn/CreditCard/Ip/UrlWithCredentials + SecretToken 7 类密钥前缀 sk-/ghp_/AKIA... + EnvSecret 敏感键名 KEY=VALUE/KEY: VALUE 值部) + 4 策略脱敏 + ring buffer 审计; 重叠匹配安全 | tool_bridge.rs / gateway guard_bridge.rs / daemon.rs 出站护栏 |
| text_protocol.rs (apeireth-tool-runtime crate, N10) | 宽松文本工具协议层 (VCP vcpLoop TOOL_REQUEST 移植): 始末语法 `<<<[TOOL_REQUEST]>>>`/`「始」…「末」` + ESCAPE 转义防注入 (块结束扫描跳过 escape 区 + 字面量映射还原) + 模糊标记匹配 (块标记大小写/空白/尖括号数容错, 字段标记 4 括号变体) + 思考块剥离 (think/thinking 大小写/属性/嵌套, 未闭合丢弃尾部防潜藏调用) + archery 分流 (separate → normal/archery) | TextToolProtocol::parse → ToolExecutor::execute_separated (archery fire-and-forget, ArcheryHandle) |
| injection.rs + chain.rs (apeireth-tool-registry crate, N15) | dynamicToolRegistry 预算化 (VCP 吸收 §8.4 P1): injection.rs 注入注意力预算 — render_injection 三段式 (light list 一行式清单 → 仅相关工具展开详情 → 超预算裁剪: 展开段→轻清单尾行→硬切留 TRUNCATION_HINT, 16000 字符上限), InjectionEntry::from_description(ToolDescription) 为描述注入挂接点; chain.rs 分类四级降级链 — ClassifyChain 自定义→小模型→RAG→关键词 (ClassifyStage 记录决定级, 小模型/RAG 级 Option<Arc<dyn Classifier>> trait 注入口, 未接真模型如实标注 has_small_model/has_rag; CustomMapClassifier 自定义级实装) | render_injection(entries, relevant 闭包, InjectionBudget) → InjectionOutput 拼 system prompt; ClassifyChain impl Classifier → register_with_classifier |
| topic_groups.rs (apeireth-companion crate, §5.1 机制②) | 记忆主题分组 + 主题索引注入 (VCP SemanticGroupManager 精神, 确定性分组): topic_tokens (CJK bigram + 拉丁词, 停用词为切分点防桥接误并簇) → group_topics 贪心聚簇 (共享 token≥1 入最高分簇, 0 嵌入 0 远程 0 随机) → build_topic_index 预算感知索引块 (TOPIC_INDEX_MAX_CHARS=600, 每主题"名称+条目数+代表条目", 超预算砍尾行留"还收纳了 N 组"提示) | assemble.rs memory_block 一处挂接 (主题索引块 + 反幻觉记忆证据块合并), inject_memory 深度/普通两路共用; 0 装 PASS (未接真嵌入, 如实标注) |
| cross_diary.rs (apeireth-companion crate, §5.1④) | 跨日记关联 — diary↔memory_graph 确定性联动 (记忆域深化包最后一件): link_core 纯函数共享 token 建链 (复用 topic_groups::topic_tokens, CJK bigram+拉丁词, 0向量0嵌入) + CrossDiaryIndex::build 只经已有公开接口采集 (DiaryStore::list_days/read_day + MemoryGraph::active_facts, 不改两模块本体) + 双向查询 diary_for_fact (记忆节点→日记片段) / facts_for_diary (日记→记忆节点) + CrossLink shared_tokens 审计证据 | CrossDiaryInjector trait 注入机制口 (0 装 PASS: 统一接线延后); lib.rs 一行注册 |
| **apeireth-tool-approval (crate, N19)** | 工具审批: 5+1 规则 (Trust/Risk/Frequency/Whitelist/Blacklist/ApprovalList) + 5min 审批窗口 + 命令级粒度 (`Tool:command` 审批键, specificity 2>1 同级静默优先) + 静默拒绝 (`::SilentReject` → 不打扰 AI, 审计留痕 `silent_rejection_audit`) + 结构化拒绝 (`wait_for_approval_outcome` → `{rejected_by_user, error_type}` 四错误码) | ToolBridge.approval + ApprovalBridge→ToolPolicyRule; 高危走主人批准通道 (洋葱安全) |
| **apeireth-credentials (crate, N21/TP3)** | 统一凭据存取层 (§10 装配主链第一环): CredentialsStore trait (get/set/delete/list/contains 按服务名) + FileCredentialsStore 文件后端 (JSON 单文件, unix 权限 600 语义) + SecretString 脱敏载体 (Debug/Display 恒 `[REDACTED len=N]`, 明文仅 expose) + 服务名校验 (拒路径分隔符/点开头/超长) + CredentialGate 审批门 trait 口 (master token 类高危 fail-closed DenyAllGate, 真审批链挂 companion 装配侧, 复用 sovereignty master token 批准语义不改本体) | 0 装边界: 明文静态存储非加密保险库 (加密属后续层); SecretString 非内存擦除容器; 消费方=companion 装配侧 (随 N17 工具装配统一接入, trait 口就绪 0 装) |

## 三、加新模块规范（维护 checklist）

0. **基础工具工程原则（强制）**：高可靠性基础工具（爬虫/网络/文件/执行等）**不得独写**——① GitHub 调研同类成熟实现 → ② 吸收先进写法（并发/重试/限速/上限）→ ③ 实战验证（真环境跑通, 如 crawl_probe）→ 才可提交；调研结论记 docs/ref-*.md。

00. **文档同步自觉（强制, 2026-08-16 主人确立）**：**工程有更新, 文档就同步**——新增/修改模块时同步更新: ① 本文件模块地图+概念词典 ② 相关设计/调研文档 (docs/*.md) ③ 接口变更同步示例与 env 清单。**调研了但未落地的项, 必须显式记入 docs/backlog.md 台账** (权威唯一, 防欠账堆积; 完成划 ✅ 并注明提交/文档位置)。

000. **消费方登记规范（强制, 2026-08-17 TP9/N18, "先立规范再干活"）**：新增 workspace crate **必须显式声明消费方**：① 谁依赖（消费方 crate 名 / 宿主进程装配点）② 为何依赖（一句话职责）。**无消费方的 crate 不得以"翻译了未接线"状态静默入 workspace**——必须在 docs/backlog.md 显式登记"独立待装配 + 接线计划"（教训: C3 盘点揪出 12 个零内部消费者孤儿 crate, 台账 #33）。定期检查: `_scripts/orphan-scan.ps1`（数据源 cargo metadata, 依赖 kind=normal/dev 权威判定——孤儿 = 纯 lib 且零内部 normal 消费者; dev-only 专用件与 bin 终点件单列不算孤儿; 含 dev-dep 自引用/双向环/dev↔normal 互指环检测 + #33 清单自动对账）。**建议每次新增 crate 后与 release 前各跑一次, 结果入台账; 孤儿处置决策归 Leader, 工具只报不删**。用法: `powershell -NoProfile -ExecutionPolicy Bypass -File _scripts\orphan-scan.ps1 [-OutFile reports\orphan-scan.md]`。

1. `src/<module>.rs` — 头部写 `//!` 职责 + 0 假装标注 (诚实: 什么没做)
2. `lib.rs` 注册 `pub mod` + 顶层 `pub use` re-export
3. 单测覆盖: 正常路径 / 失败路径 / 非法输入 (0 装 PASS)
4. 时间敏感机制 → 用 `apeireth_core::clock::VirtualClock` 可快进测试 (0 真等待)
5. 若是机制件 → 加进 `virtual_time_simulation` 模拟验收段
6. 若是工具 → ToolBridge 注册 + 白名单/日常包 + CapabilityCatalog 描述
7. 若接 daemon → 加字段 + `with_*` builder + `step()` 接线 (0 阻塞语义)
8. 更新本文档 (模块地图 + 概念词典如有新概念) + 同步相关设计/调研文档
9. 全量 `cargo test --workspace -j 4` (降并行防页文件) + 工作区干净再提交

## 四、环境变量与示例清单

| 示例 | 用途 |
|---|---|
| companion_serve | **伙伴端点 (主入口)**: OpenAI 兼容 + 状态感知 + 记忆生命周期 + 目标/自成长/SSE 主动送达 + `/panel` Web 面板 v2 (B1, 会话/记忆/图谱/授权/审计) |
| companion_daemon | 常驻主动问候 (env: TICK/MAX_TICKS/MEMORY_PATH/SUBJECT/MIN_LLM_INTERVAL/SINK/LARK_*/DREAM/REFLECT/SEED_DEMO) |
| production_daemon | 全机制集成验收 (宪法评审+隔离+spill+日志+goal+做梦+反思+每日摘要) |
| release_acceptance | AI 自己长能力端到端 (提案→评审→激活→干活) |
| multi_turn_agent | 多轮 function calling + 断点续传 (--crash-after / --resume) |
| full_acceptance / self_summary_engineering | 一期/二期验收 (真 MiniMax) |
| virtual_time_simulation | 时间机制模拟验收 (23 项, 虚拟时钟) |
| constitution_demo | 宪法评审真 LLM 判案演示 |
| education/pentest/gh_accel 各 demo | 三套件+加速插件演示 |
| exec_worker (bin) | 执行体隔离 worker (被 ToolBridge spawn) |

统一锚点: `APEIRETH_CONTINUITY_ID` (默认 companion-main) — 记忆/日志/目标/反思共用。

### companion_serve 环境变量
`APEIRETH_API_KEY` (必) · `APEIRETH_MASTER_TOKEN` (主人批准用) · `APEIRETH_PORT`/`PORT` (默认 8090)
· `APEIRETH_DEEP_RECALL=1` (推理召回) · `APEIRETH_MAX_TOKENS` (输出上限, 默认 8192)
· `APEIRETH_EXTRACT_INTERVAL_SECONDS` (提炼节流, 默认 600) · `APEIRETH_DREAM_QUIET_SECONDS` (做梦安静期, 默认 6h)
· `APEIRETH_REFLECT_PERIOD_HOURS` (反思周期, 默认 24h) · `APEIRETH_GRANT` (启动即授权 "工具:小时")
· `APEIRETH_LARK_APP_ID/SECRET/RECEIVE_ID` (离线送达, 可选) · `APEIRETH_TELEGRAM_BOT_TOKEN/CHAT_ID` (Telegram 离线送达, 可选) · `APEIRETH_SEED_MEMORY` (种子, 演示)

### gateway/provider 适配层环境变量 (N12)
`APEIRETH_REASONING_ENABLED=1/true` (推理归一化总开关, 默认关, VCP 对齐)
· `APEIRETH_REASONING_MODEL_FILTERS` (逗号分隔模型子串白名单, 空=不转换任何模型)
· `APEIRETH_REASONING_TAG` (think 块标签, 默认 think; 仅 think/thinking 两种归一结果)

### companion 环境变量（N7 查询形态学）
`APEIRETH_MORPHOLOGY_TEMPERATURE` (N7 查询形态学 softmax 温度, 默认 1.0, 非法回落 1.0)

## 五、target 构建缓存治理（台账 #51, 2026-08-17 主人指示: 不用的编译产物删掉, 不要无限膨胀）

**基线** (2026-08-17 01:52 实测): target 45.9 万文件 / 269.6 GB, 其中 debug/incremental 105.28 GB (35.3 万文件, 可再生)、debug/deps 133 GB (活跃编译不可动)。

**工具**: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/target-hygiene.ps1`
- 默认只报告: 总体积 + 子目录分布 + 安全清理候选 (incremental/criterion/tmp/顶层垃圾) 及可回收量
- `-Apply` 才真删; 测量用 robocopy /L 摘要 (1.1s 全量, Windows 下 du 极慢已弃用)

**时机规则（铁边界）**:
1. ❌ **禁止在成员活跃编译期全量 cargo clean** —— 会打断在途任务编译
2. ❌ 脚本不碰 deps/ build/ .fingerprint/ —— 活跃编译正确性依赖
3. ✅ 何时清: 评审波结束 / N14 类编译阻塞解除后 / 主人指示的静默窗口
4. ✅ 安全清理项 (脚本候选) 均为可再生缓存, 删除代价 = 下次编译变慢, 无正确性风险

**何时升级手段**: 静默期且 target > 300 GB 时, 可 `cargo clean` 全清后按需重建 (需 Leader 确认全员无在途编译)。
