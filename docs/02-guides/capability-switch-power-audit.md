# 开关权力审计表（能力开关名实相符核查）

> 核查命题：**开关名 = 开关权**。每个能力开关的「声称范围」与「实际范围」必须一致——
> 摆设开关（名称管的事实际没管住）与幽灵权力（名称外的实际权力）都是病灶。
> 本表为内测整改（文件写入总闸批）的核查产出，逐行给出处置；行内证据指向真实代码坐标。
> 复查机制：`frontend/companion-desktop/tests/switch-power-audit.mjs` 锁本表结构与行覆盖，
> `frontend/companion-desktop/tests/switch-scope-copy.mjs` 锁全责文案，Rust 侧写意图/总闸
> 测试锁行为（见各行「锁定测试」列）。

## 一、权力面开关审计

| 开关名（env / 配置字段） | 声称范围 | 实际范围（核查结论） | 处置 |
|---|---|---|---|
| `APEIRETH_ENABLE_FILE_WRITE`（file_write） | 受控文件写入 | **整改前有洞**：只管 `tool.apply_patch` 的注册/授权/风险档位；shell 的 `echo xxx > file` 等写命令不经此闸照样落盘（开关名与权力不符）。**整改后 = 唯一写总闸**：apply_patch 三档风险映射 + shell 写意图词法面（重定向 `>`/`>>`/`N>`/`<>`、删除/移动/复制/改名/建目录族、脚本引擎等值命令面）关闸即拒绝即帧（`pipeline.pre_deny`），冻结时与执行时双查 | **已整改**：写意图扫描（`crates/capabilities/tools/src/write_intent.rs`）+ shell 总闸接线（`shell.rs`）+ 组装根同源注入（`production.rs`）；词法面之外的写入（解释器内写入/编码负载/解包工具）由 shell 人工审批链 + 沙箱兑底（明示边界，见 `write_intent.rs` 白名单注释） |
| `APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS`（file_write_auto_pass） | 自动放行已读文件的修改 | 一致：仅**修改类**补丁免逐次审批；创建/删除永不自动放行（仍进人工审批四态闭合）；依赖主开关（单独设=无效果）；未读文件仍被读前观测门禁拒绝 | **保持**（名实相符） |
| `APEIRETH_ENABLE_SHELL`（shell） | Shell 命令工具开关 | 一致（注册 + grant + require_approval，每次调用人工审批）；但 shell 工具的权力由三个开关分层：注册/授权（本开关）、沙箱姿态（`APEIRETH_SHELL_SANDBOX`）、写权力（file_write 总闸）——单说"shell 开关"不等于 shell 全部行为 | **保持 + 描述补全**：设置页 shell 行明示"写命令另受文件写入总闸管辖"（开关分层如实声明） |
| `APEIRETH_SHELL_SANDBOX`（shellSandbox） | shell 沙箱开关 | 一致：开 = 文件限定工作区 + 断网隔离（平台不可实施即拒绝执行，绝不裸跑）；关 = 显式裸跑。只决定隔离要求集，不涉注册/授权/审批 | **保持** |
| `APEIRETH_ENABLE_FETCH`（fetch） | 网络读取工具开关 | 一致：注册 + grant + require_approval；工具内受控出站（目的校验/本地回环拒绝）。**核查问题"fetch 管不管住 fetch？"→ 管住**：未注册即无网络权力；其余网络面归属清晰——外部工具桥的网络行为归 `APEIRETH_ENABLE_MCP` 辖，爬取引擎是库代码、未注册为能力（模型无此权力面） | **保持**（网络面归属入档） |
| `APEIRETH_ENABLE_MCP`（mcp） | 外部工具桥开关 | 一致：装配外部工具桥 + 动态注册外部工具；动态工具经桥接治理映射进授权/风险/拒绝守卫，未知能力默认不授权（fail-closed） | **保持** |
| `APEIRETH_ENABLE_LOCAL_READ_TOOLS` / `APEIRETH_DISABLE_LOCAL_READ_TOOLS`（localReadTools） | 本地只读工具（file / search / repo） | **小口不符**：开关只决定 `tool.filesystem` / `tool.search` 的 grant；`tool.repo` **恒 grant 无开关**（只读合同）。声称含 repo、实际管两件 | **已整改（描述对齐）**：设置页 desc 改为「file / search 两件随开关；repo 恒授只读」；行为面维持只读合同不动 |
| （无开关）`tool.repo` | —— | 只读 git 仓库探查，恒注册（默认）+ 恒 grant；写操作不提供工具（设计边界） | **保持**（表内注明无开关事实） |
| （无开关）`tool.self_status` | —— | 自省只读面，恒注册 + 恒 grant + 零审批；只读取自身状态计数 | **保持** |
| `APEIRETH_ENABLE_EDUCATION`（education） | 换元检查工具开关 | **反向缺口**：开 = 注册 `tool.education`，但默认治理授权面未随开（无对应 grant），开关开了权力未必到手（会话权限档可另授） | **挂账**（本轮不扩权：授权面扩张需单独评审；已在本表如实登记） |

## 二、不属工具权力面的开关（核查后划出本表）

认知/治理层开关（`APEIRETH_ENABLE_ORGANS` / `PREFERENCE_LEARNING` / `COGNITIVE_JUDGE` / `COGNITIVE_COUNCIL` /
`ENABLE_ONION_LAYER` / 记忆核心族 / `WORKTREE_SANDBOX` 等）不向模型授予任何工具执行权力，
属认知模块装配或治理层收紧旋钮，不产生"开关名 ≠ 开关权"形态，不入权力审计面。

## 三、全责文案锚点（同一句话三处落点，测试锁定）

**「文件写入总闸：apply_patch 与 shell 写命令同受此闸」** 落在：

1. 前端能力开关描述（`frontend/companion-desktop/src/lib/views/SettingsView.svelte`，fileWrite 行）；
2. env 注释（`crates/adapters/cli/src/lib.rs`，`ENABLE_FILE_WRITE_ENV` 与 `file_write_enabled_from_env`）；
3. 用户手册（`docs/02-guides/user-manual.md`，tool.apply_patch 节；`docs/03-reference/api.md` 工具表同口径）。

拒绝帧文案（shell 写命令撞总闸时的原话）：
**「文件写入开关未开——shell 写命令受同一总闸管辖」**（帧 code `pipeline.pre_deny`，拒绝信息即帧）。

## 四、行级锁定测试指针

| 审计行 | 锁定测试 |
|---|---|
| file_write（唯一写总闸） | `crates/capabilities/tools/src/write_intent.rs` 扫描矩阵（重定向/删除类/管道链/白名单误拒注释）、`shell.rs` 写闸四测（关闸拒绝即帧/开闸行为不变/只读零误伤/冻结载荷复核）、`crates/engine/runtime-assembly/tests/file_write_registration.rs` 组装根接线 |
| file_write_auto_pass | `crates/adapters/cli/tests/file_write_knob.rs`（三档风险映射） |
| shell / fetch / mcp / localReadTools | `crates/adapters/cli/tests/production_knobs.rs`、`crates/engine/runtime-assembly/tests/canonical_shell_approval_e2e.rs`、`canonical_fetch_e2e.rs`、`mcp_bridge_runtime_e2e.rs` |
| 全责文案三落点 | `frontend/companion-desktop/tests/switch-scope-copy.mjs` |
| 本表入档（结构 + 行覆盖） | `frontend/companion-desktop/tests/switch-power-audit.mjs` |

## 五、开关生效链审计（续篇）——六环逐开关核查

> 核查命题：**开关开 = 真生效 = 自报如实**。逐开关走完六环：
> 设置开关行（`SettingsView.svelte` 的 `env:` 标注）→ 前端桥（`desktop-bridge.ts` 映射）→
> 桌面 env 注入（`backend_supervisor.rs` 的 `env_pairs`）→ Rust env 解析
> （`crates/adapters/cli/src/lib.rs` 的 `*_enabled_from_env()`）→ 装配消费
> （`crates/engine/runtime-assembly/src/canonical/production.rs`）→ 自报读值
> （`self_status_source.rs` 的名册投影）。
> 六环实测结论：**前三环（设置行 → 注入名 → 解析名）一字不差、无 env 名不一致类断点**
> （`crates/adapters/cli/tests/self_status_e2e.rs` 端到端佐证：env=1 → 名册=true）；
> 本批断点集中在后三环：装配消费缺一截（council 数值旋钮）、治理授权没跟上（education）、
> 自报照抄配置文本而非实际注册条件（挂槽开关族）、以及无开关面（MCP）。

### 5.1 逐开关生效链表

| 开关 | env / 配置接线 | 装配消费 | 运行时生效 | 自报读值 | 断点结论 / 修法 |
|---|---|---|---|---|---|
| council（议会，`APEIRETH_COGNITIVE_COUNCIL`） | 全链一字不差（设置行 `council` → 注入 `APEIRETH_COGNITIVE_COUNCIL=1` → `COGNITIVE_COUNCIL_ENV` 解析 → `config.council`） | `production.rs` 消费 `config.council` 注册 `CouncilModule`（缺 council 后端即拒开，不静默） | 模块真注册（`cognitive.council`，决策环节） | roster `council` = 装配条件，如实 | **附带断点（装配消费缺一截）**：数值旋钮 `APEIRETH_COUNCIL_ADVISORS` / `APEIRETH_COUNCIL_TIMEOUT_MS` 桌面注入了，但生产装配 `build_council_from_env()` 原先不消费（仅显式咨询命令消费）→ 顾问数/超时"注入了但没人读"。**已修**：生产 council 同源 `.with_config(council_config_from_env())`（三条构造路径同改）。开关主链六环无断点 |
| judge（评审，`APEIRETH_COGNITIVE_JUDGE`） | 全链一字不差（`judge` → `cognitive_judge` → `COGNITIVE_JUDGE_ENV` → `config.judge.enabled`） | 消费 `JudgeConfig.enabled` 注册 `JudgeModule` | 模块真注册（`cognitive.judge`，每回复最多一次 side-call） | roster `judge` = `config.judge.enabled`，如实 | 无断点（全链通） |
| education（教育工具，`APEIRETH_ENABLE_EDUCATION`） | 全链一字不差（`education` → `enable_education` → `ENABLE_EDUCATION_ENV` → `config.education`） | 消费注册 `tool.education`（`EducationModule`，纯确定性自查） | **断**：治理授权面没跟上——`tool.education` 注册了但无 grant，模型调用即拒 = "开关开了权力没到手"（上表"挂账"行同源） | roster `education` = 注册条件 | **已整改**：同一开关同源 grant（`education_enabled_from_env()` → 授权 `tool.education`，只读计算档零审批，与 `tool.repo` / `tool.self_status` 同档）；会话权限预设（read_only 等）读能力白名单语义不动；原"挂账"处置就此落账 |
| organs（器官链，`APEIRETH_ENABLE_ORGANS`） | 全链一字不差（`organs` → `enable_organs` → `ENABLE_ORGANS_ENV` → `config.organs`） | 消费注册 `OrganModule` | 模块真注册（`cognitive.organs`，AfterTurn 不阻塞回复） | roster `organs` = `config.organs`，如实 | 无断点（全链通） |
| partner_bond（伙伴羁绊，`APEIRETH_ENABLE_PARTNER_BOND`） | 全链一字不差（`partnerBond` → `enable_partner_bond` → `ENABLE_PARTNER_BOND_ENV` → `config.partner_bond`） | 消费注册 `PartnerBondModule` + 同源注入 `partner_store`（缺 store 即拒开，不静默） | 模块真注册（`cognitive.partner_bond`，TurnStart 注入 + AfterTurn 演化） | roster `partner_bond` = 装配条件，如实 | 无断点（存储现为进程内实现、重启即散属实现现状，非开关断层） |
| reflexion（反思沉淀，`APEIRETH_ENABLE_REFLEXION`） | 全链一字不差（`reflexion` → `enable_reflexion` → `ENABLE_REFLEXION_ENV` → `config.reflexion`） | 消费注册 `ReflexionModule` + 同源注入 `reflexion_store`（`FileReflexionStore`，根目录 `APEIRETH_REFLEXION_DIR`） | 模块真注册（`cognitive.reflexion`，TurnStart 教训 + AfterTurn 沉淀） | roster `reflexion` = 装配条件，如实 | 无断点（全链通） |
| self_tuning（自我调校，`APEIRETH_ENABLE_SELF_TUNING`） | 全链一字不差（`selfTuning` → `enable_self_tuning` → `SELF_TUNING_ENABLE_ENV` → `SelfTuningWire::from_env`） | 消费：`backends.self_tuning` → `MemoryRecallModule.with_self_tuning`（真接检索信号） | 接线在场但**挂在记忆召回模块上**：记忆召回关闭时信号链无处生效 | **断（消费了但自报没读对）**：名册原报"接线存在 = true"，静默失效也照报 | **已修**：名册与调参节 `self_learning` 按实际注册条件取值（接线 AND 记忆召回）；设置页该行 desc 补挂靠说明 |
| mcp（外部工具桥，`APEIRETH_ENABLE_MCP`） | **开关没接线（无开关面）**：桌面设置页原无行、`desktop-bridge.ts` / `backend_supervisor.rs` 无注入字段 —— 桌面面无此开关，CLI 旋钮 | `config.mcp` → `McpServerConfig::load`（坏配置拒开）→ `McpToolBridge` + `McpModule` | **需另行配置**服务器列表（`APEIRETH_MCP_SERVERS` 或数据目录 `mcp-servers.json`）才真正有外部工具 | **断**：已启用但无服务器配置照报 `mcp=true` | **不硬造开关**（无配置面、真生效需另行配置服务器列表）：设置页工具卡加**无开关说明行**，desc 明示"需另行配置"；自报语义改如实（`mcp` 行 = 桥已装配**且**有可用服务器；已启用但无服务器配置 = 无外部工具） |
| morphology_recall（检索深度自适应，`APEIRETH_ENABLE_MORPHOLOGY_RECALL`；温度 `APEIRETH_MORPHOLOGY_TEMPERATURE` 由消费点现读） | 全链一字不差（`morphologyRecall` → `enable_morphology_recall` → `morphology_recall_enabled_from_env()` → `config.morphology_recall`） | 消费：`MemoryRecallModule.with_morphology_recall()` | 挂在记忆召回模块上：记忆召回关闭时静默失效 | **断**：静默失效照抄 config 报 true | **已修**：名册按实际注册条件取值（AND 记忆召回）；desc 补挂靠说明 |
| absorption_insight（认知体操，`APEIRETH_ENABLE_ABSORPTION_INSIGHT`） | 全链一字不差（`absorptionInsight` → `enable_absorption_insight` → 解析 → `config.absorption_insight`） | 消费注册 `AbsorptionInsightModule`（无槽依赖） | 模块真注册（`cognitive.absorption_insight`） | roster = `config.absorption_insight`，如实 | 无断点（全链通） |
| community_triage（图社区分诊，`APEIRETH_ENABLE_COMMUNITY_TRIAGE`） | 全链一字不差（`communityTriage` → `enable_community_triage` → 解析 → `config.community_triage`） | 消费：`MemoryRecallModule.with_community_triage(graph)`（需图谱槽，缺席静默跳过） | 挂在记忆召回模块 + 图谱后端双条件上 | **断**：静默跳过照抄 config 报 true | **已修**：名册按实际注册条件取值（AND 记忆召回 AND 图谱槽）；desc 补"需图谱后端" |
| consolidation（记忆固化，`APEIRETH_ENABLE_CONSOLIDATION`） | 全链一字不差（`consolidation` → `enable_consolidation` → 解析 → `config.consolidation`） | 消费：`MemoryWritebackModule.with_consolidation()` | 挂在记忆写入模块上：记忆写入关闭时静默失效 | **断**：静默失效照抄 config 报 true（旧测试钉了错误语义） | **已修**：名册按实际注册条件取值（AND 记忆写入）；desc 补挂靠说明；旧行为 → 新行为记入测试注释 |

### 5.2 同型附注与口径

1. **同型附注（未点名行，同口径顺手修正）**：`proactive_recall`（挂记忆召回）、`memory_injection`
   （挂记忆协调器 = 记忆召回或记忆写入在场）、`typed_recall`（同左）原样照抄配置/接线值，
   同属"消费了但自报没读对"；已按实际注册条件取值，`self_status_source.rs` 单测锁定。
2. **名册口径（自报读值总则）**：名册行按**实际注册条件**取值，不复述"配置说开"而注册没成的
   事实（沿用 file_write 行先例）；`tool.self_status` 的"缺位显式 null + 原因"既有口径保留不变。
3. **council 数值旋钮**（`APEIRETH_COUNCIL_ADVISORS` / `APEIRETH_COUNCIL_TIMEOUT_MS`）：
   修法为生产装配与显式咨询命令同一条 `council_config_from_env()` 解析（同源注入口径）。
4. **education 授权面**：修法为开关同源 grant（开关名与权力相符口径），只读计算档零审批；
   上表"挂账"行的处置就此落账（保留原行留痕）。
5. **MCP 不造开关**：桌面面无此开关（CLI 旋钮 `APEIRETH_ENABLE_MCP`）；如未来补桌面注入，
   注入名必须与 CLI 一字不差。

### 5.3 续篇行级锁定测试指针

| 生效链审计行 | 锁定测试 |
|---|---|
| council / organs / reflexion（点名三链：开关开 → 装配真生效 → 自报如实） | `crates/engine/runtime-assembly/tests/capability_switch_chains.rs`（`council_switch_registers_the_module_and_reports_itself_honestly` / `organ_switch_registers_the_after_turn_module_and_reports_itself_honestly` / `reflexion_switch_registers_with_its_store_and_reports_itself_honestly`） |
| council 数值旋钮消费 | `crates/adapters/cli/src/lib.rs` 内 `production_council_consumes_the_advisor_count_knob` |
| education 同源 grant | `crates/adapters/cli/src/lib.rs` 内 `education_grant_follows_the_registration_switch` |
| judge / partner_bond / absorption_insight / education 注册链 | `capability_switch_chains.rs`（`remaining_switches_register_and_report_on_the_same_chain`） |
| consolidation / 检索深度自适应 / 图社区分诊 / self_tuning 名册按注册条件取值 | `capability_switch_chains.rs`（`consolidation_reports_the_memory_writeback_registration_condition` / `recall_dependent_switches_report_the_memory_recall_registration_condition` / `community_triage_reports_false_without_the_graph_slot`）+ `self_status_source.rs` 单测 `roster_reports_silent_dependency_switches_by_registration_condition` |
| mcp 自报语义（已启用但无服务器配置 = 无外部工具） | `capability_switch_chains.rs`（`mcp_row_reports_external_tools_only_when_the_bridge_serves`） |
| 本续篇入档（表结构 + 行覆盖） | `frontend/companion-desktop/tests/switch-power-audit.mjs` |
