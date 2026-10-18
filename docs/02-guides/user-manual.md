# Apeireth 用户手册（v2 工程重构线）

> 给运行 Apeireth 的人：它在做什么、我怎么控制它。机制描述以真实代码为准（写作时点 v2.0.0-alpha.1 / 13-crate；当前 **workspace 31 members（30 crates + bench harness）/ v2.0.0-rc.4**（2026-10-18 文档审计批更新 crate 口径），能力性判断以下方 2026-10-06 对账注与台账为准）。
> 快速跑起来见 [quick-start.md](quick-start.md)；部署见 [deployment.md](deployment.md)；架构总览见根 [ARCHITECTURE.md](../../ARCHITECTURE.md)。

```
[Document-Meta]
Document:        docs/02-guides/user-manual.md
Version:         Manual-Rev-M + R131-D6 (v2 重写)
Last-Modified:   2026-08-27
Status:          🟢 活跃
```

> ⚠️ **2026-10-06 对账注（历史口径）**：本文正文写于 v2.0.0-alpha.1（13-crate）时代。2026-10-06 时点工作区为 **18-crate**（foundation 6 / engine 8 / capabilities 1 / adapters 3，2026-10-06 新增 `crates/engine/guard`）。正文中"记忆端到端管线不在工作区里"等**能力性判断已被后续批次取代**（记忆 v2.2 生产收口于 2026-09-19 整合，见 `docs/04-internal/live-verification-ledger.md` #26），读前请对照该台账；本文主体内容待按当前实现重写。
>
> **2026-10-18 文档审计批注**：crate 口径已再变为 31 members（30 crates + bench harness；许可搬运线 image 家族 +11）；现行测试口径见单一真源 [test-baseline.md](../03-reference/test-baseline.md)。

## 1. Apeireth 是什么（v2）

**基地，不是 AI 本身**：Apeireth 是给 LLM 的操作系统——提供工具、记忆、治理边界、协议、运行时；不定义 AI 是什么。接入一个 LLM（默认 MiniMax-M3，Anthropic / OpenAI 兼容也可），基地给它：工具、记忆生命周期、治理闸门、可观测 trace。

**v2 两种运行形态**（v1 的 TUI / companion_serve / 三件套目录装配 已进 `legacy/`，恢复排期见根 `ROADMAP.md` §4）：

| 形态 | 命令 | 端口 | 说明 |
| | | | |
| **HTTP gateway** | `apeireth gateway serve --port 8080` | :8080 | canonical 入口，HTTP/SSE；OpenAI Chat Completions 兼容端点；薄壳适配器，全部委托 `Runtime::execute` |
| **CLI chat** | `apeireth chat "<prompt>"` | — | 单轮会话，直接走 agent loop（governance → provider → tool dispatch → 回灌续轮）|

`apeireth session` 是就绪自检（打印运行时就绪信息后退出，非交互）；对话用 `apeireth chat "<prompt>"`（单轮）；`apeireth --help` 看完整命令。`frontend/companion-desktop/` 是独立 Svelte 5 + Tauri 2 前端 workspace，不在根 cargo workspace 内。

## 2. 她怎么记得你（v2 形态）

v2 真实在 13-crate 工作区里运行的：

- **`crates/engine/memory`** 持**域原语**（Episode / Session / IdentityCard 等通过 `apeireth_core::kernel` 复用），vector/graph 检索契约 primitive 已在 M1B 落地；ACT-R 全量记忆模型、记忆 v2 (importance/对账/排名/版本链) 的完整实现**留 legacy/`，排期见 ROADMAP §4 P3**
- **Session 生命周期**（`crates/engine/runtime/src/canonical/session.rs`）：transcript + 持久化接缝（`InMemorySessionStore` + 可换 SQLite 等）；approval 可恢复（pending approval 以 outcome 形式返回，不丢失 turn 上下文）
- **Continuous session**通过 `SessionId` 跨 CLI/gateway 同一会话；前端切换不影响 trace 重建
- **数据落本地**：默认 SQLite WAL + `PRAGMA user_version` 版本化迁移（`crates/engine/storage`）

记忆 v2 的"对话打分 / 对账 / 排名注入"等流水线**功能等价物**将在 P3 阶段在 `crates/engine/memory` 重实现，详见 ROADMAP §4 P3/P6。当前 v2 状态下记忆引擎已能接上 plugin / session / SQLite，**但"打字 → 评分 → 写库 → 检索注入"的端到端管线不在 13-crate 工作区里**——这是一个明确的可观察缺口，不是文档不诚实。

## 3. 主动能力（v2 状态）

- **agent loop 单一入口**（`crates/engine/runtime/src/canonical/execute.rs`）：一次 turn = governance（completion）→ provider → 工具调用则 capability lookup + 插件分发 → 工具结果回灌 transcript → 继续；approval 是 outcome 不是 error；tool 失败不终止回合。
- **模型选择**：`apeireth chat --model MiniMax-M3` 或 gateway request body 显式；fallback 顺序：minimax → anthropic → openai-compatible（已启用时），按环境变量配置
- **当前不会主动找你**——v1 的"涌现循环 / 开口策略 / 安静窗 / 主动送达"机制**整体留在 legacy/**，v2 尚未实现；ROADMAP §4 P6-P7 排期

## 4. 工具（v2 形态）

v2 唯一进程执行边界 = **`crates/capabilities/tools/src/process/`**（`ProcessExecutor`，Windows Job Object + CREATE_SUSPENDED 完整，Linux/macOS 进程组部分）。

**`BuiltinToolsPlugin::new(workspace_root)` 默认注册 3 个只读工具，且执行许可默认放行（与 `tool.repo` 同待遇）**（2026-09-12 起）：

| 工具 | 风险 | 用途 |
| | | |
| `tool.filesystem` | medium | 读 / 列 / stat workspace 内文件 |
| `tool.search` | low | 确定性本地文件名 + 文件内容搜索 |
| `tool.repo` | low | 只读 git 仓库探查（status / diff / log / branch / summary）|

**`tool.shell` 与 `tool.fetch` 默认关闭**，需 `BuiltinToolsOptions { shell: Some(TrustedShellConfig), fetch: Some(FetchConfig) }` 显式开启（opt-in，非默认）。这与 v1 companion_serve `APEIRETH_GRANT=...` 的临时放行模型不同——v2 的批准模型是**编译时 / bootstrap 时**的显式配置 + **运行时** governance pipeline（见 §6）。

**`tool.apply_patch`（受控文件写入，opt-in，第七件生产工具）**：`APEIRETH_ENABLE_FILE_WRITE=1` 开启（注册 + 授权同源）。**文件写入总闸：apply_patch 与 shell 写命令同受此闸**——开关关时 shell 的写命令（重定向 `>`/`>>`、删除/移动/复制/建目录类与脚本引擎等值命令面）同样拒绝即帧（`pipeline.pre_deny`；保守词法扫描，宁可误拒不漏放）；开关开时 shell 写命令行为不变（照旧走审批链与既有风险映射）。补丁式受控写文件——创建（`*** Add File`）/ 修改（`*** Update File`）/ 删除（`*** Delete File`）必须在补丁里显式声明，多文件补丁全有或全无（任一 hunk 失败整笔回滚）；**解析即合同**：空补丁、认不出的补丁行、没有任何可解析 hunk 的修改段一律报错误帧（绝不返回「成功但零动作」的谎帧），成功报告只在真有动作落地后返回（新增/修改/删除计数逐项如实），空/解析不了的补丁按 `Unparseable` 语义 fail-closed 向人工审批；工作区外路径（绝对路径 / `..` 组件 / symlink 逃逸）拒绝（fail-closed 即帧），凭据与密钥面（`.env` 族 / 密钥材料 / 凭据存储件）拒绝；沿用读前观测门禁——**未读不得覆盖写**（先用文件读工具读过，才能修改或删除）。风险档位默认 **RequireApproval 级：每次写入都要人批**（本地审批面板 + IM 审批卡同一条审批链，四态闭合同源）；`APEIRETH_ENABLE_FILE_WRITE_AUTO_PASS=1`（子档，依赖主开关）开「自动放行已读文件的修改」——仅**修改类**补丁免逐次审批，**删除/新建永不自动放行**（仍进人工审批），未读文件的修改仍被读前门禁直接拒绝。

**`tool.git_write`（git 写分级，opt-in，干活能力轴 P1）**：`APEIRETH_ENABLE_GIT_WRITE=1` 开启（注册 + 授权同源）。仓库写操作按**写意图分级词表**给面，命令文本必须是 git 调用段（段间 `&&`/`;`/换行），逐段分级取最严档；执行走受控通道（结构化 argv、`git -C <工作区根>` 限域、超时归 `timeout.*`、输出归一）：
- **L1 可逆**（`git add` / `git commit`（带消息）/ `git branch` 建·`git switch` 切 / `git tag` 本地）——默认 **RequireApproval 级：每次人批**（本地审批面板 + IM 审批卡同一条审批链）；`APEIRETH_ENABLE_GIT_WRITE_AUTO_PASS_L1=1`（子档，依赖主开关）开「自动放行 L1」档，仅 L1 免逐次审批；
- **L2 出网**（`git push`（非强推）/ `git fetch` / `git remote` add·rename·set-url·update）——**永远人工审批**：无自动放行档，任何会话预设/设置都不得越人（IM 审批卡可以批 push，但每一次都得批）；
- **L3 毁灭**（改史 `commit --amend`/`rebase`、强推 `--force`/`+refspec`、引用删除 `update-ref -d`/`branch -d`/`tag -d`/`--prune`、`reset --hard`/`clean -f`）——**拒绝即帧，永远没有工具面**。

参数变形防绕过：长选项按前缀重叠命中（`--ha` 与 `--hard` 缩写同拒），短选项束逐字符查（`-qf` 里的 `f` 也算强推），refspec 的 `+`（强推变体）与空侧 `:`（删除变体）同拒；子命令前的危险全局选项（`-c` / `--exec-path` / `--git-dir` / `--work-tree` 族与任何未列全局选项）拒绝即帧；管道 / 重定向 / shell 元字符面拒绝即帧（命令按结构化参数执行，不经 shell）。组合命令逐段顺序落地，中段失败**如实上报**（逐段报告已落地段与失败因，无回滚承诺，staged 残留照实留存）；remote URL 凭据全链脱敏（命令面 / 输出 / 审批卡不落密）。

**git 写边界（分级语义，显式声明防模型幻觉）**：git 写只有 `tool.git_write` 这一个分级入口——L1 可逆默认人批（设置可开自动放行 L1）、L2 出网永远人批、**L3 毁灭永远没有工具面**（不要请求它）。`tool.repo` 维持只读合同（status / diff / log / branch / summary 探查）。

**`tool.worktree_plan` / `tool.worktree_apply`（多文件大改四拍受控写，opt-in，干活能力轴 P2）**：`APEIRETH_ENABLE_WORKTREE_PLAN=1` / `APEIRETH_ENABLE_WORKTREE_APPLY=1` 开启（注册 + 授权同源；**子依赖主 + file_write**：计划拍要 `APEIRETH_ENABLE_FILE_WRITE=1` 同开，落盘拍再要计划拍同开，fail-closed）。多文件大改走「计划→改→验证→落」四拍：计划拍（`tool.worktree_plan`，只读**零审批**）产**改单**（文件列表 + 每文件动作 + 预期）+ plan_id，零落盘；落盘拍（`tool.worktree_apply`）按改单执行——改单与补丁**交叉一致**（文件集 + 动作标签逐项相符，漂移拒绝即帧），改拍/验证拍在暂存面（工作区零写入，验证命令走受控通道：结构化 argv、限域暂存面、超时），落拍过读前观测门禁（**未读不得覆盖/删除**）+ 持久档原子写，**全有或全无**（任一失败整批回滚）。风险档位 **RequireApproval 级：每次落盘都人批**（无自动放行档，审批卡载荷 = 改单摘要：文件数 / 动作分布 / 验证命令）。两级工具**不含 git 写**——改单只动文件，提交走 `tool.git_write` 分级词表。

CLI 隐私逃生门：`APEIRETH_DISABLE_LOCAL_READ_TOOLS=1` 关闭 filesystem/search 执行许可；旧 `APEIRETH_ENABLE_LOCAL_READ_TOOLS=1` 保留兼容（等同默认）；两者同设时 DISABLE 赢（fail-closed）。敏感路径保护恒开。

**`tool.skill_distill` / `tool.skill_search`（书→技能沉淀，opt-in，P 轴吸收自动化 P1——原 I 轴 / I1，2026-10-16 字母双占统一改名，审计修 D13）**：`APEIRETH_ENABLE_SKILL_DISTILL=1` / `APEIRETH_ENABLE_SKILL_SEARCH=1` 开启（注册 + 授权同源；沉淀**子依赖 file_write 总闸**——要 `APEIRETH_ENABLE_FILE_WRITE=1` 同开，fail-closed；检索只读档独立）。沉淀管线：内容输入（正文，或文件路径走**只读通道**：工作区词法限域 + 敏感面拒）+ 模型侧草稿（调用方 prompt 模板提炼的触发词 / 要点 / 操作步骤）→ 结构化技能卡（坏卡拒 + 触发词归一）→ 技能库（数据目录 `skills/`，每卡一 StoredDoc，**拒开语义**）。创建**重复名拒绝**；版本演进（`update`）= 同名更新，旧版先留 `audit.jsonl` 审计再原子写新版（id / 首沉淀时刻不变）。治理档：沉淀是写文件面——**每次沉淀都停人工审批**（无自动放行档，审批卡载荷 = 技能卡摘要）；检索是只读档——零审批零写盘。调用面（上下文预算纪律）：技能卡进模型上下文的唯一方式是 **L2 封套披露**（检索式注入：触发词命中 / 按需点名，共享引用封套 + 每源预算 + 分级省略措辞），**绝不无差别全量注入**——未命中卡正文零注入，卡名轻清单只含名字。

**`tool.trait_report`（性格周报，只读档零审批，裁定 ④⑤ 落地）**：近 7 天四根特质滑杆（遗忘衰减强度 `memory_fade` / 好奇强度 `curiosity_strength` / 语气情绪饱和度 `tone_saturation` / 整合节奏 `consolidation_cadence`）的**移动量 / 移动原因 top / 锁定态**，外加每杆最近一次移动（带记录原因）。数据 = **成长计既有账**（「性格养成」自校准的沉积记录 `tuning-log.jsonl`，与学习日志**同账**，不新造账）；周报窗 = 近 7 天（两端含，未来时刻不计；零移动留痕不计移动）。治理档：**只读**——不写、不执行、不外联、无参数（未知字段拒绝即帧），**零审批**（与 `tool.self_status` 同档）。`tool.self_status` 的调参/性格节同账带出**四杆锁定态 + 最近移动**（缺源显式 `null` + 原因）。

**性格滑杆锁定（裁定 ④）**：四根滑杆各可**锁定**，锁定态持久在数据目录 `trait-locks.json`（与成长账同目录）——锁定后**成长沉积跳过该杆**并留审计痕（同账零移动留痕行，reason 带「已锁定, 跳过沉积」前缀，可查账）；锁定只挡自动成长沉积，显式人动作（调参面板保存 / 学习日志撤销）不受约束。**解锁 = 显式动作**（`apeireth trait unlock <SLIDER>`），无自动解锁路径。命令面与工具面同内核两出口：`apeireth trait report`（打印周报）/ `apeireth trait lock <SLIDER>` / `apeireth trait unlock <SLIDER>`；锁定档读不出来时 fail-closed 全锁定（宁可不动任何杆，也不把坏档读成全解锁）。

**生产接线旋钮表（2026-10-17 收口批：整编族两旋钮 + 暴露面体检 + 发声情绪源；全部默认关 fail-closed，注册即闸）**：

| 旋钮 | 作用 | 档位 / 纪律 |
|---|---|---|
| `APEIRETH_ENABLE_PERSONA_CONSOLIDATION=1` | 人格提取步：整编触发点上从近期情节提炼人格增量 → 人格层合并（同类合并 / 矛盾标注「人格张力」，不静默覆盖） | **随整编跑**——要 `APEIRETH_ENABLE_CONSOLIDATION=1` 同开才真跑，整编关 = 显式不跑（自述名册 `persona_consolidation` 行照实 false）；默认关 |
| `APEIRETH_ENABLE_CONSOLIDATION_HASH_GATE=1` | 整编散列门：整编素材同源同散列跳过重复整编（省算力防重复记忆），变内容照常重摄 | **随整编跑**（同上，名册 `consolidation_hash_gate` 行照实 false）；门自身故障 fail-open 照常整编（真因进 warning 计数，不静默降级）；默认关 |
| `APEIRETH_ENABLE_EXPOSURE_AUDIT=1` | 暴露面体检工具面 `tool.exposure_audit`：本地只读自查（凭据文件分布 / 明文风险点 / 开放端口面 / 备份包排除项 / 令牌绑定完整性），与 `apeireth exposure report` 同一内核两个出口 | **只读档**——grant 即用零审批（与 `tool.self_status` 同档）；体检不写被测面任何字节、报告永不回密钥值；测不了的项显式「未测」不冒充干净；默认关 |
| （非旋钮，随发声配置注入） | **情绪→语气微调**：`voice_speak` 每次发声现读 presence/PAD 合成器现值，按冻结映射表微调语速/音量（情绪只调「味道」不改内容） | 无情绪数据 = 原参数零变化（不猜情绪）；显式传参覆盖自动微调；微调幅度有限（语速 ±2 / 音量 ±3 格）；自述名册 `voice_emotion_tune` 行按读取面实际注入事实取值 |

**会话级审批策略（2026-09-12；E-P2 族归一后口径如下）**：`SessionSettings` 支持 `permission_preset`（`read_only` 拒写/执行 / `standard` 沿用审批 / `full` 免审批留日志）+ `approval_remember`（`standard` 下"会话内记住"：某次批准后该会话内**同一授权对象**的后续调用跳过审批——命令面授权对象 = **族**（批"删除族" = 该族所有形态生效，同族换形重放不重批；跨族仍逐族审批），无族命令按逐次操作逐字记住；审批卡带**族标注**（「授权族: …」，人见族而批）；进程内记忆、重启清零；inner 的 `Deny` 永不绕过）。桌面端会话头提供 4 档选择器：只读 / 标准·每次审批 / 标准·会话内记住 / 完全放行。

工具调用走 **`<<<[TOOL_REQUEST]>>>` 已废弃**：v2 工具调用由各 provider 的原生 tool_calls 流式协议处理（OpenAI `tool_calls` 数组），不再走 marker 解析。`crates/capabilities/tools/src/plugin.rs` 的 plugin descriptor（`CapabilityDescriptor`）声明每个工具的 CapabilityId / kind / risk / M2* stage 标注，是唯一权威来源。

## 5. 她怎么"想"（v2 状态）

v2 砍掉了"全套器官"复杂度，保留了**可执行机制**和**可插拔接口**：

- **可执行**：`crates/engine/runtime::canonical::execute` 的 single-shot agent loop（governance + provider + tool dispatch + trace）
- **可插拔**：`ProviderCapability` trait（`crates/foundation/plugin`）+ 3 家 canonical provider（minimax/anthropic/openai-compatible）

v1 的"世界模型 / 好奇心 / 假设检验 / 情感记忆 / 价值内化 / 自我诊断"等**实现全部在 `legacy/donor/apeireth-companion`**，v2 尚未实现；详见 ROADMAP §4 P6。

## 6. 治理与安全（v2 现状 + 0 装 PASS 标注）

**合同层**（`crates/foundation/governance/src/lib.rs`）：一个 trait（`GovernanceHook`）+ 三种决策（`Allow` / `Deny{reason}` / `RequireApproval{reason}`，`Deny` 与 `RequireApproval` 不可混淆——runtime 必须区分"拒绝"与"挂起"）；`GovernancePipeline` 按顺序短路第一个非 allow。

**可用的 hook 实现**（全部在 governance crate，**生产默认不挂载**）：

| 类型 | 名字 | 用途 |
| | | |
| `AllowAll` | "allow_all" | 默认 + 测试用；honest default（可见） |
| `DenyCapabilities` | "deny_capabilities" | 拒绝指定 CapabilityId 列表 |
| `MaxRounds` | "max_rounds" | 限制单 turn 工具循环轮数 |
| `PermissionGovernanceHook` | "permission_policy" | PermissionPolicy 包装（grant/revoke/需审批能力）|
| `PromptInjectionHook` | "prompt_injection_heuristic" | 启发式注入信号检测 |
| `CredentialDisclosureHook` | "credential_disclosure" | 凭据泄漏拦截 |
| `AuditHashChain` | — | 防篡改审计哈希链（append-only）|
| `PiiDetector` | — | PII 检测与 redact |

**生产治理接线（✅ 已通过 upstream `873d2857` 落地，2026-08-27）**：

- `crates/adapters/cli/src/lib.rs::build_canonical_runtime_from_env` **挂载 `GovernancePipeline`** = `PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook`（3 个）；配置来源 `build_production_governance_from_env()` 按环境变量 `APEIRETH_GOVERNANCE_*` 装配
- runtime 每个 turn 的 `CapabilityDispatch` **都**先经 governance pipeline 评估，**不再**默认 `AllowAll`
- `MaxRounds` 是**结构性**约束（runtime.rs 内部），`AuditHashChain` 按部署需要挂
- **回合预算显式化**：单回合轮数上限默认 8（`DEFAULT_MAX_ROUNDS`）、单轮工具调用上限默认 16（`MAX_TOOL_CALLS_PER_ROUND`），可用 env `APEIRETH_MAX_TURN_ROUNDS` / `APEIRETH_MAX_TOOL_CALLS` 覆写（越界钳制到 1..=64，未设 / 非法值回默认；CLI `turn_round_limit_from_env` / `tool_call_limit_from_env` 装配）。轮上限失败的 `turn_not_converged` 错误帧带真实数字：`limit`、已耗轮数 `rounds`、当时待批工具名 `pending_tool`（若有）；回合内审批冻结即收束为待批准态（不烧剩余轮预算），同回合重复提议同一待批准项直接收束、不重走冻结循环
- 工具层兜底：`shell` / `fetch` 默认 opt-in 关闭；`ProcessExecutor` 是 Windows Job Object 完整 / Linux·macOS 进程组部分隔离（参见 [architecture.md](../01-architecture/architecture.md) Process ownership 表）
- `credentials` 走 **`EnvCredentialResolver`**（`crates/engine/provider/src/credentials.rs`）：逻辑名→环境变量映射（`provider.minimax.api_key` → `APEIRETH_API_KEY`、`provider.anthropic.api_key` → `APEIRETH_ANTHROPIC_KEY`、`provider.openai-compatible.api_key` → `OPENAI_API_KEY`），无 secret 留 struct、secret 走 `Secret<T>`（debug redact）
- **`apeireth-credentials` crate（keyring / encrypted file / KMS backend）代码存在但未接线**——本批运行时用的是 `EnvCredentialResolver`；legacy OS keyring 集成排期见 ROADMAP §4 P2

**0 装 PASS 总结**：治理机制全部就位且有单测，**生产 bootstrap 装 3 个 hook**（Permission + 凭据泄漏 + 注入检测），其余 hook（MaxRounds 结构性、AuditHashChain 按需、DenyCapabilities 显式）由部署方决定。PII/越权/凭据泄漏 3 类核心攻击面被上游 `873d2857` 守住；剩余延后项按 ROADMAP §4 P2-P8 排。

## 7. SDK（v2 stub）

`crates/adapters/sdk` 提供 `ApeirethClient`（HTTP + WS）客户端类型 + 6 工具 method 签名（`web_search` / `file_ops` / `git_ops` / `code_exec` / `calendar` / `message`）+ 鉴权 5 组件 stub（Bearer / keyring / token bucket / audit / quota）。**真实 HTTP / WebSocket 调 `apeireth-gateway` 走 `unimplemented!()` 守门**，R21 真接。这是显式 stub，不是半成品——`crates/adapters/sdk/src/client.rs` §32 "阶段 6 stub 边界"列出了已就位 vs 待 R21 接的清晰范围。

## 8. 诚实标注（v2.0.0-alpha.1, 2026-08-27）

| 项 | 状态 |
| | |
| **生产 governance 接线** | ✅ 已做（upstream `873d2857`）：`PermissionGovernanceHook` + `CredentialDisclosureHook` + `PromptInjectionHook` 安装到 CLI bootstrap |
| **敏感 workspace 路径保护**（`.env` / `.ssh` / `.aws` / `.gnupg` / `.secret` 等）| ✅ 已做（upstream `ac5cbf5a`）：`tool.filesystem` + `tool.search` 通过 `crates/capabilities/tools/src/sensitive_path.rs` 屏蔽（普通项目元数据如 `.gitignore` / `.cargo/config.toml` 仍可读）|
| **13 键 verdict cache 接线** | ✅ 降级完成（2026-08-27）：`crates/foundation/core/src/philosophy.rs::RUNTIME_ENFORCED = false` 显式标注"非 runtime 强制"；`VERDICT_KEYS_BY_PRINCIPLE` 映射到 5 原则洋葱（E 存在 / S 价值 / A 经验 / M 方法论）。13 键 v2 角色 = 哲学标准 / 判别词汇表（hook deny reason 引用 + CapabilityDescriptor risk 分级）。**v2 取代 13 键强制机制 = external hook 闸**（已装 3 个）。 |
| **`apeireth-credentials` 接线** | ⚠️ P2：env resolver 在 provider，生产足够；keyring 等后端未挂 |
| **M1B 记忆全量实现（ACT-R / 完整管线）** | ⏳ P3：当前 memory crate 有 primitive；端到端管线待 P3 |
| **MCP 动态能力注册** | ⏳ P4 |
| **ProcessSupervisor + 进程树** | ⏳ P5 |
| **companion 器官实现（W1/W2/W3 / E4 / F4 / F1 / F6 / E7）** | ⏳ P6 |
| **连续感知（voice/screen）** | ⏳ P7 |
| **前端产品化对接（companion-desktop ↔ gateway）** | ⏳ P8 |
| Docker 多架构构建 | ✅ 已修（commit 4596357, $TARGETARCH, linux/amd64 + linux/arm64）|
| SDK 真接（HTTP / WS 调 `apeireth-gateway`）| ⏳ R21（stub 守门）|
| 13 键编译期 hardcode + 13 键测试契约 | ✅ 在 core 跑（`crates/foundation/core/tests/verdict_keys.rs` 等）|

## 8.5 2026-10 能力增量补注（文档审计批，2026-10-18）

> 本节只补「正文写作后已落地」的现值能力，不重写上方历史正文；逐项真源见
> [capabilities-matrix.md](../03-reference/capabilities-matrix.md) §0.2（默认开关以
> `crates/engine/runtime-assembly/src/canonical/production.rs` 实况为准）与
> [test-baseline.md](../03-reference/test-baseline.md)（测试口径）。

- **设备面**：`tool.device_control` = 三拍受控执行 + 三档分级 + 白名单 + 状态账；
  money 档必须 IM 审批卡（两本账 + `plan_id` 幂等 + 失败留账重试，见
  [im-quick-access.md](im-quick-access.md) 件四）。`tool.guard_watch` = 看家守护骨架，
  **只看不动**（告警零设备动作）、`APEIRETH_ENABLE_GUARD_WATCH` 依赖
  `APEIRETH_ENABLE_DEVICE_SENSE` 双闸 fail-closed；守护告警按「守护例外档」可破勿扰，
  但必带「守护告警」标注 + 独立频率上限。`tool.care_schedule` 提醒闭环（L2）已接。
- **创作面**：`tool.image_ops` 换引擎级创作内核 `apeireth-image-core`，操作词表 **13** 个
  （新增 `layers_composite` / `psd_read_info`），产品代码对第三方图像库的直接依赖清零；
  `tool.doc_ops` 已支持 **DOCX 写面**（`docx_from_markdown` / `docx_append`，样式/编号层最小：`word/styles.xml` + `word/numbering.xml`（一级 bullet，有序列表暂不区分））；`tool.sheet_ops` 已支持 **xlsx 读入**
  （三种单元格形态、公式格取缓存值不重算；**写面只出 CSV**，xlsx 写出后置）。
- **治理/会话面**：只读会话预设的执法面已覆盖 `tool.device_control` / `tool.care_schedule`
  的写/执行面（`read_only` 下显式拒绝，见 `permission_preset.rs`）；凭据泄漏绊线补 JWT 与
  通用 Bearer 形态；认知调度器淘汰策略改**严格 FIFO**；deny 治理对账清 33 条过期 skip、
  4 条 unmaintained 三处同步。
- **自省面**：`tool.self_status` 冻结合同已扩至**二十一节**（新增 `device_control` 等，
  见 [self-insight-channel.md](self-insight-channel.md)）。

## 9. FAQ

| 问题 | 答案 |
| | |
| 怎么跑？ | `cargo run -p apeireth-cli -- gateway serve --port 8080`（HTTP）或 `cargo run -p apeireth-cli -- chat "<prompt>"`（CLI）。 |
| 换模型？ | `APEIRETH_API_KEY` / `APEIRETH_ANTHROPIC_KEY` / `OPENAI_API_KEY` 任一即可，按 fallback 顺序生效。改 provider 不用重编译（plugin 注入）。 |
| 启用 shell/fetch？ | bootstrap 时 `BuiltinToolsPlugin::with_options(root, BuiltinToolsOptions { shell: Some(...), fetch: Some(...) })`。CLI 默认不启用。 |
| 工具被拒了？ | 先看开关是否开启（opt-in 工具未开 = 未注册即拒，见 [capability-switch-power-audit.md](capability-switch-power-audit.md)）；再看 governance 决策（`Deny` / `RequireApproval` 四态）；最终以 `tool.self_status` 的 `capabilities` 名册生效值为准。 |
| 我想接一个 IDE 一样的自定义模型？ | 实现 `ProviderCapability` trait + 注册为 plugin；参考 `crates/engine/provider/src/canonical_minimax.rs` 作为示例。 |
| Docker？ | `Dockerfile` 多架构已修；本机无 docker 详见 [CI 实测为准](https://github.com/Apeireth/Apeireth/actions)。 |
| 我看到的"13 键"在哪？ | `crates/foundation/core/src/philosophy.rs` + 测试 `crates/foundation/core/tests/verdict_keys.rs` 等。当前未接 canonical 执行路径（见 §8）。|

## 10. 一句话

**v2.0.0-alpha.1 = canonical agent loop + 13-crate 工作区 + 3 provider + 5 工具（2 opt-in）+ 治理 hook 全在 + 生产已挂 3 个（P0 接线 `873d2857`）+ sensitive_path 路径保护 (`ac5cbf5a`)**。alpha 这个名是诚实的：骨架与主链绿了，3 个核心安全 hook 已挂，剩余延后项按 ROADMAP §4 P2-P8 排。

---

_本手册 v2 重写 (2026-08-27)：替换 v1 companion_serve / TUI / `<<<[TOOL_REQUEST]>>>` marker 全部形态为 gateway + canonical agent loop 现实，§6 治理与 §8 诚实标注严格 0 装 PASS（什么在、什么不在、为什么）。v1 形态在 legacy/，恢复排期见根 ROADMAP §4。_