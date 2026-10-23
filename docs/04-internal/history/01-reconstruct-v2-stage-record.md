# reconstruct_v2 阶段施工卷宗（历史归拢册）

> **本册定位**：把 reconstruct_v2 时期的**阶段施工稿**按 M1A → M3A 的真实施工顺序合成一册，保留当时的阶段目标、关键设计决策、**冻结契约**、证据与提交号、当时的未决项。**正文原意不改写**——叙述压缩，事实 / 表格 / 残余清单 / owner 决策位不压缩。本册是**索引 + 归拢**，不是替代；原件仍在原位。

## 覆盖范围

合并自以下 **18** 份原件（路径相对仓库根）：

| # | 原件路径 | 并入节 | 性质 |
|---|---|---|---|
| 1 | `docs/01-architecture/master-functionality-port-audit.md` | §1.1 | 历史 |
| 2 | `docs/01-architecture/reconstruct-v2-migration-map.md` | §1.2 | 历史 |
| 3 | `docs/01-architecture/canonical-skeleton-freeze-audit.md` | §2 | 历史 |
| 4 | `docs/01-architecture/m1a-canonical-storage-foundation.md` | §3 | 历史 |
| 5 | `docs/01-architecture/m1b1-memory-foundation.md` | §4.1 | 历史 |
| 6 | `docs/01-architecture/m1b2-memory-retrieval.md` | §4.2 | 历史 |
| 7 | `docs/01-architecture/m1b3-vector-graph.md` | §4.3 | 历史 |
| 8 | `docs/01-architecture/m1c-governance-donor-primitives.md` | §5 | 历史 |
| 9 | `docs/01-architecture/m2a-simple-tool-ports.md` | §6 | 历史 |
| 10 | `docs/01-architecture/m2b-process-containment.md` | §7.1 | 历史 |
| 11 | `docs/01-architecture/m2b-x-cross-platform-isolation.md` | §7.2 | 历史 |
| 12 | `docs/01-architecture/m2c-shell-readiness-review.md` | §8.1 | 历史（只读评审） |
| 13 | `docs/01-architecture/m2c-approval-resume-lifecycle.md` | §8.2 | 历史 |
| 14 | `docs/01-architecture/m2c-trusted-shell.md` | §8.3 | 历史 |
| 15 | `docs/01-architecture/m2d-network-egress-enforcement.md` | §9 | 历史 |
| 16 | `docs/01-architecture/m3a-canonical-fetch.md` | §10 | 历史 |
| 17 | `docs/01-architecture/organ-orchestrator-spec.md` | §11.1 | **现行依据，保留原位** |
| 18 | `docs/01-architecture/organ-orchestrator-completion-plan.md` | §11.2 | **现行依据，保留原位** |

## 时间跨度

- **2026-08-23 → 08-27**：reconstruct_v2 分支 M1A → M3A 施工主线（§1–§10）。
- **2026-08-28**：A 块 OrganOrchestrator 规范与完整化方案（§11），与 M 序列**并行而非串行**。
- 原件自带基线横幅（2026-08-27）：默认分支 `main`、13-crate 工作区、tag `v2.0.0-alpha.1` @ `d6910cf7`；旧 86-crate 代码整体在 `legacy/`（workspace exclude）。**该横幅是历史口径，本册不追溯改写。**

## 读法

1. **「M1A 到 M3A 按什么顺序落地、每步冻了什么」**——§1 前置审计、§2 冻结基线、§3–§10 严格按施工顺序，每阶段五段式（阶段目标 / 关键设计决策 / 冻结契约 / 证据与提交号 / 当时的未决项）。
2. **「同一主题散在几篇里」**——「审批」横跨 §8.1/§8.2/§8.3，「隔离」横跨 §7.1/§7.2，「出站 + Fetch」横跨 §9/§10；本册串起来并加导航，读者不必按文件名考古。
3. **「当年结论今天还成立吗」**——不在历史叙述里写新数字；现行口径看 **§12 现状对照（2026-10-23）**，它只指向现行真源。

## §0 导航

| 阶段 | 主题 | 节 | 当时主线结论（一句） |
|---|---|---|---|
| 前置 | master→canonical 差距审计 + 迁移图 | §1 | 27 项「Lost Capabilities」定出 P0/P1/P2/P3 优先级 |
| 前置 | canonical 骨架冻结现实审计 | §2 | 0 个 P0 违规；4 项 P1 违规（协议层依赖 / runtime 根 / core 根） |
| M1A | 存储地基 | §3 | 单写者 + 读池 + SQLite WAL + `user_version` 迁移 |
| M1B | 记忆（地基 / 检索 / 向量图） | §4 | 记忆归 memory crate；ACT-R 激活公式与确定性排序冻结 |
| M1C | 治理 primitive 迁回 | §5 | Allow/Deny/RequireApproval 三态 + PII/注入 hook + 审计哈希链 |
| M2A | 简单工具 | §6 | filesystem/search/repo 三只读工具，路径包含化 |
| M2B | 进程封装 + 跨平台隔离 | §7 | fail-closed 启动；非 Windows 仅 guardrails，非 OS 沙箱 |
| M2C | 审批 / Shell | §8 | 先做审批恢复生命周期，再做 Trusted Shell |
| M2D | 网络出站 | §9 | 受控 egress + DNS 钉扎 + 逐跳重校验；进程网络隔离三平台全 UNSUPPORTED |
| M3A | 受控 Fetch | §10 | GET-only、默认 DISABLED、冻结 egress 策略 |
| 附录 | 非 M-序列同期真源 | §11 | OrganOrchestrator 规范 + 5 stage 完整化方案（**保留原位**） |
| — | 现状对照 | §12 | 现行真源指向（不写新数字进历史） |
| — | 未决项汇总 | §13 | 全期挂账清单（owner 决策位） |

---

## §1 前置基线：master → canonical 差距审计与迁移图

### §1.1 Master Functionality Port Audit（原件 1）

**审计时点 Git 状态（历史口径）**：canonical 分支 `reconstruct_v2`，canonical HEAD `21646e79b82e7cffc95b0942604111c4e7b3a9b3`；`origin/reconstruct_v2` `b34418834fce69fd2332c09580a1e214d18e3a01`；`origin/master` `46c25b4bfb258351fb2185baad22b0b3eb7b302d`；merge base `b0a017f060841119b46d79e28870732ae80e1aed`；工作树干净（审计开始时）。`git fetch origin` 成功；`git log HEAD..origin/reconstruct_v2` 为空（**远端未前进**）；`origin/reconstruct_v2` 是本地 HEAD 的直接祖先，本地领先 4 个冻结提交 `736962db` / `0b17f66c` / `21bd45e6` / `21646e79`；**允许快进推送**。

**「Lost Capabilities」27 项（生产路径层面 master 有、canonical 缺）——M1A→M3A 施工单的直接来源**（原为表；此处按优先级压缩，字段：Feature / Value / Canonical target / Port difficulty / Dependencies）：

- **P0**：SQLite read/write pool（HIGH → `apeireth-storage`（新），LOW，rusqlite/r2d2）。
- **P1**：ACT-R memory store + temporal/tombstone（HIGH → `apeireth-storage`，LOW，pool/clock）；Hybrid vector index cosine+BM25（MEDIUM → `apeireth-storage`/retrieval，LOW，none）；Filesystem/search/repo tools（HIGH → `ToolCapability` plugins，LOW，std/tokio）；Platform sandbox Windows JobObject（HIGH → tool sandbox capability，MEDIUM，winapi）；MCP protocol/transport（HIGH → MCP transport boundary，MEDIUM，tokio/serde）；PII scrub + prompt injection hook（HIGH → governance hooks，LOW，regex）；Audit hash chain（HIGH → governance/audit primitive，LOW，sha2）；Event bus（HIGH → core event primitive，LOW，tokio）；Plutchik/PAD emotion（HIGH → companion，LOW，serde）；Borbely rhythm/drive（HIGH → companion，LOW，std）；Prompt assembler（HIGH → companion，LOW，emotion）。
- **P2**：Graph primitives（MEDIUM → `apeireth-storage`/graph capability，MEDIUM，none）；Fetch/browser tools（MEDIUM → `ToolCapability` plugins，MEDIUM，reqwest + egress）；Worktree sandbox / factory（MEDIUM → factory capability，HIGH，git subprocess）；Onion/ABAC policy（MEDIUM → governance hooks，MEDIUM，std）；Scheduler（MEDIUM → runtime service，LOW，tokio）；Telemetry metrics（MEDIUM → observability adapter，LOW，std/tracing）；Dream engine（MEDIUM → companion，MEDIUM，memory + world model）；Epistemic healer（MEDIUM → companion，LOW，std）；Presence snapshot（MEDIUM → companion/gateway，LOW，emotion）；Voice VAD/duplex（MEDIUM → voice capability，LOW，std）；Gemini adapter DTO/parser（MEDIUM → future `ProviderCapability`，LOW，serde）；SDK client（MEDIUM → SDK API surface，LOW，reqwest）。
- **P3**：Curiosity（LOW → companion，MEDIUM，memory）；Voice TTS（LOW → voice capability，HIGH，external provider）；Avatar expression mapping（LOW → avatar/desktop，LOW，emotion）。

> **原件自注**：上表（原 :563-589，实为 27 项）**多数已由 M1A/M1B/M1C/M2A/M2B/M2C/M2D/M3A 重新实现，本表已过时**；DEFER 清单仍有效并对应 ROADMAP §4 P3–P7。

**Deferred Functionality（原件 §21，原样）**：Dream engine and world model；Curiosity engine；Voice TTS and full voice stack；Avatar Live2D/VRM runtime；Bridge (social/lark/livekit/stock)；Desktop/vision full stack；Worktree factory and ToolSynthesizer；Invest and Learning tools；PyBridge；SDK polish（待 canonical gateway 契约稳定）。

**Dropped Architecture（原件 §22，**永不搬运**清单）**：`UnifiedRuntimeHost`（God Object，拆解但不搬）；`ModelRouter`（被 `ProviderRouter` 取代）；`ProtocolAdapter::execute(api_key)`（被 `ProviderCapability` + `CredentialResolver` 取代）；master `ToolRegistry`；master runtime `CapabilityRegistry`；`SessionManager` 作第二 session owner；`LifecycleHandle` facade；`HybridCognitiveRouter` fast path（硬编码模板 = 第二 loop）；runtime 里的裸 `api_key: String`；裸 `reasoning_cot` / `reasoning_content` 公共字段；Gateway 直连 host 子系统（直调 `ToolRegistry`/`MemoryStore`/`SessionManager`）；`McpRegistry` / MCP 装进 `ToolRegistry`。

**当时 5 个 Open Questions（owner 决策位，§23）**：① 新的 canonical `apeireth-storage` 是根 crate 还是 `reconstruction_v2` storage crate 内模块？架构说「根 storage crate 待定」，确切路径未定。② canonical runtime 是否需在 `Runtime::execute` 里做 memory retrieval 服务，还是 companion prompt 装配应是接收 session transcript 的 plugin capability？master 把两者融合，canonical 拆分在代码层未规定。③ `PiiDetector` 与 `AuditHashChain` 是 governance hook 还是独立可观测/安全 crate？`ARCHITECTURE.md` 说「独立成 crate 并成为 `GovernanceHook` 实现」，crate 名未定。④ 首个 storage 搬运是否保留 master 的「JSON 装一表」schema，还是引入规范化 schema？影响迁移 SQL 与未来图遍历。⑤ MCP 安装/卸载生命周期：canonical plugin 是静态/进程内的，spawn 出来的 MCP server 该如何表示成 plugin manifest？

### §1.2 Legacy Migration Map（原件 2）

**决策词汇**：`KEEP`（已 canonical，不重写）/ `PORT`（搬到 canonical owner，大体照搬）/ `MERGE`（折进已有 canonical 组件）/ `REDESIGN`（职责保留、形态不保留）/ `DEPRECATE`（调用方迁完即删）。

**优先级前三项（阻塞其余一切）**：① 从 `apeireth-runtime` 驱逐 legacy orchestrator——在此之前 canonical runtime crate 传递依赖十个 legacy crate；② 把 `apeireth-core` 排空到 `kernel`——在此之前 core 不是 primitives crate，38 个 crate 依赖这份差异；③ 把 `apeireth-provider` 的 providers 包成 `ProviderCapability` plugin——在此之前 canonical runtime 没有真 provider，只有 test double。

| 层 | Legacy | 当前职责 | Canonical owner | 决定 | 迁移策略 | 前置 |
|---|---|---|---|---|---|---|
| 1 | `apeireth-runtime`（lib.rs, `g5_runtime_bridge`, `workflow_worker`） | 七模块编排驱动：heartbeat / task store / bus / arbitration / search / group chat / emotion | 新 `apeireth-orchestration`，或并入 `apeireth-companion` | **REDESIGN** | 把 driver 搬出 crate，让 `canonical/` 成为整 crate；只有 `apeireth-gateway` 与 `apeireth-tui` 依赖它，爆炸半径 = 两处 Cargo.toml + 调用点 | 决定七模块 loop 是否存活，或变成 companion 认知由 `Runtime::execute` 驱动 |
| 1 | `apeireth-core`（`memory`,`onion`,`philosophy`,`gate`,`lifecycle`,`eight_anchors`） | memory items / 权限洋葱 / 哲学 verdict cache / 五闸 guard / 九相认知生命周期 | `apeireth-storage`（memory）、`apeireth-governance` hooks（onion/philosophy/gate）、`apeireth-companion`（认知生命周期） | **REDESIGN** | 逐模块搬，crate 根留 `pub use` shim，调用方迁完后删 shim | 每个目标 owner crate 须先存在；`apeireth-storage` 当时尚不存在 |
| 1 | `apeireth-protocol` | 四协议归一、零内部依赖、无 I/O | 自身 | **KEEP** | 无；这就是 canonical protocol 层 | — |
| 1 | `apeireth-companion`（`plugin.rs`,`capability.rs`,`capabilities_manifest.rs`,`runtime_capabilities.rs`） | 一套 `Plugin` trait + 一个 `PluginRegistry` + 三份独立 `Capability` 结构 | `apeireth-plugin` | **MERGE** | 各改写为一个 manifest 里的 `CapabilityDescriptor`；删掉对手 registry | 调用方审计——这些是 companion 内部，最不危险的 registry 合并 |
| 1 | `crates/_frozen/apeireth-plugin` | 冻结的旧 plugin 系统，1276 LOC，无调用方 | `apeireth-plugin` | **DEPRECATE** | 先挖出 canonical 模型缺的东西，再删冻结副本 | 无 |
| 1 | `reconstruction_v2/`（10 crate，独立 workspace） | 并行原型，根 workspace 不构建 | — | **REMOVED** | pre-freeze 清理时删除（审计已捕获其治理与时钟思路）；Git 历史是恢复路径 | 无产品/构建引用 |
| 2 | `apeireth-llm-iface` | 单一 `LlmProvider` trait、能力 bitflags、health、streaming | `apeireth-plugin::ProviderCapability` | **MERGE** | 保留 legacy 契约 + 加 shim，让五家 provider 一次性变成 capability | 议定 shim 的 `LlmRequest` ↔ `NormalizedRequest` 转换 |
| 2 | `apeireth-provider` | 五家合并 provider 实现 | 声明 `provider.*` 的插件 | **PORT** | 一 vendor 一 plugin，声明 `provider.<vendor>`，key 走 `CredentialResolver` | 上述 `LlmProvider` shim |
| 2 | `apeireth-api::llm::router`（`MultiLlmRouter`） | fallback 序、健康 EMA、可重试分类 | `apeireth-runtime::canonical::provider::ProviderRouter` | **PORT** | **算法已搬完**；`apeireth-api` 调用方迁完后删原件 | `apeireth-api` 调用方迁移 |
| 2 | `apeireth-api`（其余） | 四协议 handler、chat 管线、keep-alive 池 | `apeireth-protocol` + provider 插件 + `apps/gateway` | **REDESIGN** | 拆：协议处理在 `apeireth-protocol` 已重复；管线与 `Runtime::execute` 重叠；HTTP 面归 gateway | gateway app 必须存在 |
| 2 | `apeireth-pipeline`、`-g5` | 五阶段 chat 管线；通用五阶段基座 | `apeireth-runtime::canonical::execute` | **MERGE** | agent loop 即 canonical 管线；token 预算与抑制窗口收成 loop 关注点或治理 hook | 决定通用 `Pipeline<T,I,O>` 基座在 chat 之外是否还有用户 |
| 2 | `apeireth-http-client` | keep-alive LIFO 连接池 | provider 插件 | **KEEP** | 留作 provider 插件用的基础设施；**绝不可**从 `apeireth-protocol` 可达 | — |
| 2 | `apeireth-protocol::gateway`,`bridge`,`bridge_ext` | adapters 之上的高层 facade | `apeireth-protocol` | **KEEP** | 留；它们是翻译侧 facade，不是路由 | — |
| 3 | `apeireth-tool-registry` | 六类工具枚举、五正交轴、token 预算、热重载、异步任务存储 | `apeireth-plugin` registries + 带类型的视图 | **MERGE** | 类别变成 `CapabilityKind` + 元数据；registry 变成 `CapabilityRegistry` 上的视图；`AsyncTaskStore` 属 runtime 关注点并随之迁移 | `AsyncTaskStore` 在 `apeireth-runtime` 有调用方；先做第 1 项 |
| 3 | `apeireth-tool-runtime` | 工具调用解析器、执行器、记录、隐私 guard | `Runtime::execute` + 治理 hooks | **MERGE** | 解析与执行即 loop；隐私 guard 变 `GovernanceHook` | — |
| 3 | `apeireth-tool-shell`,`-filesystem`,`-browser`,`-fetch`,`-search`,`-codesearch`,`-image-gen`,`-image-process` | 八个具体工具 | 声明 `tool.*` 的插件 | **PORT** | 各包成一个 `ToolCapability`，id `tool.shell` / `tool.filesystem` … | 无；最容易的迁移，最佳首批证明 |
| 3 | `apeireth-tool-approval` | 工具审批流 | `apeireth-governance` | **PORT** | 变成返回 `Decision::RequireApproval` 的 `GovernanceHook`——**该变体正是为此存在** | — |
| 3 | `apeireth-tools` | 五个集成 trait、文件操作器、MCP adapter | 插件 + `transport.mcp` | **MERGE** | 拆工具实现与 MCP adapter | MCP 归属见下 |
| 3 | `apeireth-skills` | 可复用能力声明 + schema、`SkillRegistry`（双通道） | `apeireth-plugin` | **MERGE** | skill 即能力声明；registry 变视图，**不留第二存储** | 调和 skill semver 规则与 `PluginManifest::version` |
| 3 | `apeireth-extension` | 六个扩展类、`extension.toml` schema、沙箱、调用审计 | `apeireth-plugin`（`CapabilityKind::Extension`） | **MERGE** | `extension.toml` 映到 `PluginManifest`；沙箱留作基础设施；调用审计变观察者能力 | 议定 manifest schema 超集 |
| 3 | `apeireth-evolution`（`traits.rs`） | 第三套 `Plugin` trait + 第三套 `PluginRegistry` + `Extension` trait | `apeireth-plugin` | **MERGE** | 删掉对手 trait 与 registry；evolution 保留状态机，消费 canonical 能力 | 审计 `apeireth-evolution` 内部调用方 |
| 3 | `apeireth-mcp` | MCP client/server、JSON-RPC 2.0、stdio/SSE 传输、tool-registry 桥 | 声明 `transport.mcp` 的插件 | **PORT** | 插件把每个远端 MCP 工具暴露为 id `tool.mcp.<server>.<name>` 的 `ToolCapability`；**不建** `McpRegistry`/`McpRuntime`/`McpAgent`/`McpPermissionSystem`（那是第二能力生态） | 动态能力注册：MCP 工具在连接时发现，而 `PluginManager` 当前在注册时索引能力 |
| 3 | `apeireth-agent` | agent 别名解析、LRU 缓存、热重载、subagents | `apeireth-runtime` | **REDESIGN** | agent = 配置过的 `Runtime` + system prompt + 能力子集，不是另一台机器 | canonical 支持把 runtime 收窄到能力子集 |
| 4 | `apeireth-guard` | PII 检测、脱敏、审计 | `apeireth-governance` hook | **PORT** | 实现 `GovernanceHook`，带 detector 理由拒 | — |
| 4 | `apeireth-council` | 七强制 advisor、hold 机制、synthesis | `apeireth-governance` hook | **PORT** | 实现 `GovernanceHook`，未达共识返 `RequireApproval` | council 当前持 mock LLM provider，应改为 `ProviderCapability` |
| 4 | `apeireth-library-governance` | 策略策略、验证不变量、一致性检查 | `apeireth-governance` hook | **PORT** | 每个策略包成 hook 并用 `GovernancePipeline` 组合 | — |
| 4 | `apeireth-onion`、`apeireth-sovereignty` | 原则/权限洋葱、租户主权 DSL | `apeireth-governance` hooks | **PORT** | 洋葱层变成有序 pipeline——正是 `GovernancePipeline` 已有形状 | 解决与 `apeireth-core` onion 副本的重叠（第 2 项） |
| 4 | `apeireth-constraint`、`apeireth-rate-limiter` | 约束、限流 | `apeireth-governance` hooks | **PORT** | 都是带理由的 `Decision::Deny` | — |
| 4 | `apeireth-arbitration` | HASH-SQL append-only canonical timeline | `apeireth-storage` + 观察者能力 | **REDESIGN** | 审计链是 `ExecutionTrace` 的消费者，不是平行的自有记录 | `apeireth-storage` 必须存在 |
| 5 | `apeireth-memory`、`-memory-extensions` | 记忆引擎、九个 provider 后端 | `apeireth-storage`，暴露为 `memory.*` 能力 | **PORT** | 九个后端变成 `CapabilityKind::Memory` 能力，各一插件 | `apeireth-storage` 已建 |
| 5 | `apeireth-vector`、`-graph`、`-graph-primitive` | 向量检索、图编排 | `apeireth-storage` | **MERGE** | 折进 storage 作为检索后端 | 同上 |
| 5 | `apeireth-context-fold` | 上下文折叠 | `apeireth-runtime` 或 `apeireth-storage` | **REDESIGN** | 决定折叠属 transcript 关注点（runtime）还是 recall 关注点（storage） | — |
| 5 | `apeireth-credentials` | OS keyring、`SecretBuf` zeroize、加密文件后端 | 自身，在 `plugin::CredentialResolver` 之后 | **KEEP** | 加一个由该 crate 支撑的 `CredentialResolver` 实现——它就是该 trait 的意图生产后端 | 无——小而高价值的下一步 |
| 5 | `apeireth-state`、`-config` | 共享状态模式、配置 | `apeireth-runtime` 组合 | **MERGE** | 配置喂给 `RuntimeBuilder`；状态模式被 runtime 自持状态取代 | — |
| 6 | `apeireth-gateway` | HTTP/WS 面、可观测路由、语义路由 | `apps/gateway` | **REDESIGN** | 停做 session 管理 / provider 路由 / tool dispatch，改调 `runtime.execute`；`semantic_router.rs` 是第四个 router，应并为 provider-router 策略 | 第 1 项（gateway 依赖 `apeireth-runtime`） |
| 6 | `apeireth-cli` | CLI | `apps/cli` | **PORT** | 移到 `apps/`，驱动 `Runtime::execute` | — |
| 6 | `apeireth-tui`、`-tui-e2e` | 终端 UI | `apps/` 或 UI crate | **KEEP** | 移到 `apps/`；必须调 runtime 而非重实现 chat | 第 1 项 |
| 6 | `apeireth-web`、`crates/_frozen/apeireth-tauri-stub` | Leptos web 面；冻结 Tauri 参考 | `apps/desktop` | **REDESIGN** | 本阶段不在范围；先定桌面形态 | — |
| 6 | `apeireth-sdk`、`apeireth-sdk-*` | 纯 Rust SDK client 与语言绑定 | 自己的 crate | **KEEP** | 重定向到 canonical 契约类型 | canonical 类型稳定 |
| 7 | companion cognition 家族（`-cognition`,`-consciousness`,`-perception`,`-motivation`,`-emotion`（在 companion 内）,`-life-force`,`-value`,`-experience`,`-asi`,`-central`） | 认知 / 基础设施 / 领域内容，**不被收敛阻塞** | 逐步并入 `apeireth-companion` | **MERGE** | 多个已是彼此的透明 re-export | — |
| 7 | orchestration support（`-supervisor`,`-cron`,`-bus`,`-workflow`,`-team-lead`,`-acp`） | 同上 | 待第 1 项决定 | **KEEP** | canonical 归属随 legacy-orchestrator 决定走 | — |
| 7 | infrastructure（`-host`,`-telemetry`,`-i18n`,`-repo-tools`,`-environment`,`-upgrade`,`-guard`） | 同上 | 插件/app 可用的基础设施 | **KEEP** | — | — |
| 7 | quality and tooling（`-test`,`-eval`,`-bench`,`-verify`,`-integration-e2e`,`release-tools`,`-naming-v05`,`-blueprint-impl`） | 同上 | 无 canonical 含义 | **KEEP** | — | — |
| 7 | domain and frontier（`-voice`,`-livekit`,`-lark`,`-stock`,`-wiki`,`-pybridge`） | 同上 | 审计保留、不扩展 | **KEEP**（冻结） | scope guard | — |
| 7 | `crates/_archived/*`、`crates/_frozen/*` | 已归档 | — | **DEPRECATE** | 无引用后删除 | — |

**Known gap（原文）**：MCP 是唯一需要一个尚不存在的 canonical 特性的迁移——plugin 激活**之后**才出现的能力。registry 故意拒绝迟注册重复项，所以动态能力需要显式、受检的 API，而不是在现有 API 上开洞。

**执行这张图的 5 条规则（原文）**：① **一次 commit 一项迁移**——搬两个职责的 commit 无法独立回滚。② **先 shim，再迁调用方，最后删**——绝不在一次改动里搬一个类型并更新四十个调用点。③ **还有依赖方时不删 legacy crate**——每次删除前跑 `cargo tree -i`。④ **说不出前置条件的迁移不算就绪**——每行都有前置，或显式写「—」。⑤ **canonical registry 永不新增对手**——若某项迁移似乎需要第二个 registry，它需要的是一个带类型的视图。

---

## §2 骨架冻结现实审计（原件 3）

**审计时点**：`reconstruct_v2`，Starting HEAD `b34418834fce69fd2332c09580a1e214d18e3a01`；状态「recorded before freeze corrections」。本文记录 canonical 骨架的**归属事实**，是冻结修正与不变量测试的判定基线。

**方法**：读当前 canonical crates（`apeireth-core`、`apeireth-protocol`、`apeireth-plugin`、`apeireth-governance`、`apeireth-provider`、`apeireth-runtime::canonical`、`apeireth-gateway::canonical_entry`、`apeireth-cli` canonical bootstrap）、生产 provider 能力与既有集成测试。

**Provider 迁移前置**：MiniMax / Anthropic / OpenAI-compatible 三家**全部**直接实现 `ProviderCapability`、凭据走 `CredentialResolver`、进生产路径（CLI + gateway bootstrap）、**无** `LegacyLlmCapability` 残留（`crates/**/*.rs` 全仓 0 命中）。

**组件归属与违规（表，节选：owner / must-not-own / 违规 / 处置）**：

| 组件 | 应拥有 | **不得**拥有 | 违规与处置 |
|---|---|---|---|
| Core | 稳定 id、通用元数据、生命周期 primitive、时钟/时间抽象、通用领域/内核错误、通用事件 | HTTP、LLM vendor、provider 路由、tool 执行、Axum、CLI、凭据后端 | canonical `kernel` 干净；crate 根仍 re-export legacy 内容（已登记为迁移项）→ 本阶段无动作，冻结记录边界 |
| Protocol | canonical 请求/响应 DTO、消息/内容表示、tool-call/tool-result 表示、usage、finish reason、stream 事件契约 | reqwest client、API key、Bearer、x-api-key、vendor endpoint、provider health、provider routing、retry loop、Runtime | `reqwest`/`tokio` 声明为 normal 依赖但只被 examples/tests 用 → 移到 `[dev-dependencies]`（违规 #1、#2） |
| Plugin | `Plugin` trait、plugin 生命周期、`PluginManifest`、能力声明与实现注册、能力归属、canonical capability registry | runtime、gateway、storage | 无发现；`PluginManager::register` 校验 manifest 与实现、拒重复能力 id 与重复模型面工具名 |
| Capability Registry | 权威 id→owner 索引；plugin registry 拥有 plugin 状态 | 第二份能力声明副本、`ProviderRegistry2`/`ToolRegistry2`/`RuntimeRegistry`/`VendorRegistry`/`PluginManagerV2`/`CapabilityManager` | 无发现；重复归属 fail-closed；迭代按 id 排序 |
| Governance | `Allow`/`Deny`/`RequireApproval` 语义、`GovernanceHook`、`GovernancePipeline` | runtime、provider、gateway、具体策略库 | 无发现；决策带稳定 tag 序列化并彼此区分 |
| Provider | vendor 协议适配、vendor wire model 身份、vendor HTTP transport、认证头构造、provider 本地配置、`ProviderCapability` 实现 | session、runtime loop、tool 执行、governance、gateway 路由、全局重试编排 | 三家 canonical provider 无发现；按 turn 解析 key，自持 `reqwest::Client` |
| Tool | `apeireth-plugin::ToolCapability` 契约（runtime 拥有 dispatch）：工具身份、schema、执行接口、工具结果、风险/能力元数据 | provider 绝不可直接执行工具；tool 不得拥有 session/loop | canonical 路径无发现；注册时拒模型面名称冲突 |
| Runtime | `apeireth-runtime::canonical`：session 生命周期、执行轮次、治理评估、provider 选择/调用、tool dispatch/continuation、失败持久化、执行 trace | vendor 实现、tool 实现、HTTP transport、凭据存储、UI | canonical 模块干净；周边 crate 仍带历史七模块 driver 及其依赖（迁移项 #1）→ 加源码级不变量守护 canonical 模块 vendor-free |
| Gateway | `apeireth-gateway::canonical_entry` 生产 chat HTTP：只做 transport adapter | 直接调 provider、直接执行工具、直接改 session repo、自做记忆编排、实现治理、含 vendor 分支 | canonical entry 干净；周边仍带历史 gateway/语义路由代码 → 本阶段无动作，冻结记录 canonical entry 为生产 chat 路径 |
| CLI | `build_canonical_runtime_from_env` + `execute_canonical_cli_turn`：配置、bootstrap、输入输出适配 | agent loop、provider 重试、tool loop、session 业务逻辑、治理 | canonical chat 路径干净；历史 CLI 命令保留但非 canonical chat 路径 |
| Credentials | `CredentialResolver`/`Secret` 契约、secret 脱敏、secret 边界 | runtime/gateway/provider 上的长期 `api_key: String`、硬编码凭据、固定路径 | canonical 生产路径无发现；`apeireth-credentials` 后端仍是 `CredentialResolver` 背后的待落 drop-in（P1） |
| Session | `apeireth-runtime::canonical::session`：生命周期、transcript、结构化 session 事件、持久化 seam | gateway session manager、provider session 状态、CLI session store | canonical 路径无发现；`SessionManager` 是唯一生命周期 owner；`InMemorySessionStore` 是 seam |
| Trace | 结构化执行 trace：provider invoked/succeeded/failed、治理决策、tool invoked/succeeded/failed、round completed | 裸私有思维链、`reasoning_cot`、`raw_chain_of_thought`、`internal_reasoning` | 无发现；trace 类型无 reasoning 字段，且已有测试守护序列化形态 |

**依赖方向（观测到的 canonical 边）**：`core ← protocol ← plugin ← governance(core only) ← provider(core/protocol/plugin) ← runtime::canonical(core/protocol/plugin/governance) ← gateway / cli`。**未发现**任何禁止边（`core→runtime`、`protocol→runtime`、`plugin→runtime`、`governance→runtime`、`provider→gateway/cli`）。**唯一需要修正的是 protocol crate 的无用 normal 依赖。**

**违规分类（4 项，全 P1，0 项 P0）**：① `apeireth-protocol` 把 `reqwest` 声明为 normal 依赖但库代码不构造 client → **立即修**：移到 `[dev-dependencies]`；② 同上 `tokio` → **立即修**；③ `apeireth-runtime` crate 根仍含 legacy vendor 命名代码（`LlmWorker`、`api_key: String`）在 `canonical` 之外 → **本阶段不重写**（已登记迁移项 #1），加不变量测试保证 canonical 模块 vendor-free；④ `apeireth-core` crate 根仍带 `kernel` 之外的 legacy 内容 → **本阶段不重写**（迁移项 #2），冻结记录边界。

**E2E baseline（冻结修正前）**：`cargo test -p apeireth-runtime --test canonical_agent_loop` → 17 passed；`cargo test -p apeireth-gateway --test canonical_entry_e2e` → 4 passed。成功回路与 Deny / RequireApproval / ProviderFailure / ToolFailure 四种情形已被这两个测试目标覆盖。

---

## §3 M1A —— Canonical Storage Foundation

**原件**：[`../../01-architecture/m1a-canonical-storage-foundation.md`](../../01-architecture/m1a-canonical-storage-foundation.md)

**阶段目标**：从 `origin/master:reconstruction_v2/crates/apeireth-storage` 取存储地基（`Cargo.toml`、`src/pool.rs`、`src/migrations.rs`、`src/lib.rs`），建成 canonical SQLite 存储。

**关键设计决策**：采纳「单序列化写者 + 读池」架构（**已验证**，不是只看类型名）；写路径经 mpsc channel 排队、结果经 oneshot 回传；**写连接直接从 manager 创建并先于 reader pool 建立** → 写者不占读池槽位；读访问是短命闭包 `read` 而非返回 `PooledConnection`，**所有 mutation 必须走 `write`**；迁移引擎改为**版本化**（`PRAGMA user_version`），基线没有；配置显式化（`SqliteConfig` + 校验），基线把 PRAGMA 写死；共享内存库用 `r2d2_sqlite::SqliteConnectionManager::memory`（SQLite `:memory:` 是 per-connection）。

**冻结契约**：PRAGMA = **WAL**、`synchronous=NORMAL`、`foreign_keys=ON`、`busy_timeout=5000`，per-connection PRAGMA 初始化经 `r2d2_sqlite` 的 `with_init`；`StorageError` 六变体 `Open`/`Db`/`Pool`/`WriteQueue`/`Migration`/`InvalidConfiguration`（比基线更窄的变体集拆开 Open/Migration/InvalidConfiguration）；迁移 = 版本化 + **每迁移一事务** + 失败回滚 + v1 幂等；**on-disk 兼容** —— migration version 1 保留基线表集 `episodes, notes, sessions, agent_traces, facts, links, topic_groups, provenance` 与索引 `idx_facts_id`，SQL 一律 `CREATE IF NOT EXISTS`，使旧实现建的 `user_version = 0` 库被**幂等升级且不丢数据**。

**证据与提交号**：`r2d2_sqlite` 由基线 0.24（rusqlite 0.31）升到 **0.25（rusqlite 0.32）**，以服从 workspace rusqlite 0.32 硬锁并避开 `libsqlite3-sys` links 冲突。

**当时的未决项**：**未采纳清单** = MemoryStore / MemoryItem / ACT-R activation / VectorIndex / Graph / Context folding / Session persistence / Runtime-Gateway-CLI 存储集成 / `Memory_*` support modules（归 M1B 及后续阶段）；本阶段不与 runtime / gateway / provider / companion 集成。

---

## §4 M1B —— Canonical Memory（地基 / 检索 / 向量图）

### §4.1 M1B1 记忆地基（原件 5）

**阶段目标**：把 v1 记忆语义落进 canonical `apeireth-memory::canonical`。状态 Implemented on `reconstruct_v2`；**本阶段后迁移版本 = 2**。

**边界决策（冻结契约）**：canonical 记忆 owner = `apeireth-memory::canonical`；存储 owner = `apeireth-storage`（SQLite pool + migrations）；**Runtime integration: none**。

**语义分类表（原件表，保留）**：

| Baseline concept | Classification | Disposition |
|---|---|---|
| `MemoryItem` 字段（`id`,`data`,`importance`,`access_count`,`access_times`,`created_at`,`valid_from`,`valid_until`,`is_tombstone`,`artifact_sig`） | DOMAIN | 采纳为 `canonical::MemoryItem`，时间字段用核心 `Timestamp`，id 用 typed `MemoryId` |
| `MemoryOperation` | DOMAIN operation | 改编为显式 repository 方法（`insert`/`update`/`tombstone`），不用一刀切的 apply op |
| `QueryMode` | RETRIEVAL | **推迟到 M1B2**（时间过滤由 `MemoryFilter` 提供；无排序） |
| ACT-R activation | RETRIEVAL | 推迟到 M1B2 |
| `facts` JSON 表 | PERSISTENCE DETAIL | **canonical 不采纳**。v1 `facts` 保持不动以保 on-disk 兼容；canonical 记忆用 migration v2 的 typed `memory_items` 表 |
| 自动 SHA-256 `artifact_sig` | DOMAIN policy | 改编：`artifact_sig` 作为**可选的调用方提供**指纹持久化；**M1B1 不做自动 hash** |

**领域模型（冻结契约）**：时间有效性确定且显式——`valid_from` **含**、`valid_until` **不含**；`valid_until = None` 表示永久有效；边界测试覆盖全部五种必需情形。`MemoryRepository` 暴露 `insert`/`get`/`update`/`query`/`tombstone`；**普通读排除 tombstone**，历史/墓碑读需 `MemoryFilter::with_include_tombstones(true)`。`SqliteMemoryRepository` 包装 `apeireth_storage::SqliteConnectionPool`，**从不**开裸 `rusqlite::Connection`、**从不**自建 `r2d2` pool、**从不**spawn 写线程。

**Schema（冻结契约）**：v1 不变；v2 = `memory_items` typed 表 + 时间列与 tombstone 列索引。升级测试覆盖 fresh `0→2`、existing `1→2`、幂等 `2→2`、**迁移失败不标记完成**。

**Architecture gate（冻结契约，逐条）**：Memory→Runtime **NO**；Memory→Gateway **NO**；Memory→Provider **NO**；Memory→Companion **NO**；Storage→Memory **NO**；Memory 使用 canonical storage pool **YES**；导入 ACT-R/vector/graph **NO**。

### §4.2 M1B2 检索语义（原件 6）

**阶段目标**：落检索语义。状态 Implemented on `reconstruct_v2`；**本阶段无 schema 变更**。源：`origin/master:reconstruction_v2/crates/apeireth-storage/src/memory_v2.rs`（`calculate_act_r_activation`、`MemoryStore::query`）。

**采纳**：ACT-R 启发式激活公式 `sum = Σ max(current_time - t_j, 1)^(-decay)`，`activation = ln(sum) + beta`（当 `sum > 0`，否则 `beta`）；基线 query score `activation + importance * 2.0`；默认参数 `decay = 0.5`、`beta = 0.0`、importance 权重 `2.0`；未来访问时间戳裁剪到 1 秒差。

**改编**：检索入口是 canonical `MemoryRepository` 上的**纯函数** `retrieve`（不是组合 store 上的方法）；`QueryMode` **未**采纳为公共 enum（时间资格用 repository 的 `MemoryFilter`（`as_of`），墓碑包含显式）；**检索不改写访问元数据**——读是纯的，访问追踪留作后续阶段的显式 repository 级操作。

**拒绝**：Jaccard 相似度与贪心聚类不属于 M1B2；无向量 / embedding / LLM 排序；**无隐藏挂钟**——`as_of` 永远显式。

**API（冻结契约）**：`act_r_activation(access_times, as_of, decay, beta) -> Result<f64, MemoryError>`；`RetrievalOptions`（as_of, include_tombstones, limit, minimum_importance, decay, beta, importance_weight）；`MemoryHit { item, score }`；`retrieve(repo, options) -> Result<Vec<MemoryHit>, MemoryError>`。

**确定性（冻结契约）**：非法数值参数（`decay <= 0`、非有限值）被拒；排序 = score 降序 → `created_at` 升序 → `MemoryId` 升序，**不使用 `HashMap` 迭代序**；空访问历史返回 `beta`。

**测试**：golden 激活值、未来时间戳裁剪、非法参数、时间资格、墓碑包含、importance 过滤、时近与访问次数排序、零访问行为、稳定 tie 排序、确定性 limit。

### §4.3 M1B3 向量 / 图基础设施（原件 7）

**阶段目标**：落向量与图基础设施。状态 Implemented on `reconstruct_v2`；**本阶段无 schema 变更**，两者**仅内存**（与基线语义一致）。

**归属决策（冻结契约）**：vector owner = `apeireth-memory::canonical::vector`；graph owner = `apeireth-memory::canonical::graph`。理由：两者都是记忆/查询基础设施；冻结架构预期记忆拥有索引 primitive；**不新建 crate**——两者当前只有一个消费者、无独立依赖边界。

**源**：`vector.rs`、`graph_primitive.rs`、`graph_ops.rs`、`graph.rs`（均在 `origin/master:reconstruction_v2/crates/apeireth-storage/src/`）。

**采纳**：余弦相似度 + 显式零向量处理（`0.0`）；确定性 top-k 查询形状；有向图基本元（node / 带标签边 / neighbours）；BFS、DFS 风格确定性遍历（**采纳为 BFS**）、来自 `graph_ops.rs` 的无权最短路；基线图爬取的**环安全有界 BFS**。

**改编（冻结契约）**：`VectorIndex` 有**显式固定维度**，维度不匹配与非有限值被拒（不 panic、不毒化排序）；重复 vector id = `Conflict`，update/remove 显式；图边同时带 `relation` 与**有限** `weight`（合并基线 `graph_primitive` 的标签语义与 `graph.rs` 的权重语义）；重复边被拒而非静默重复；`remove_node` 连带删除关联边，`remove_edge` 报告缺失边；**所有迭代序确定**（按 id/relation 排序），非 `HashMap` 序。

**拒绝 / 推迟**：基线 `CausalGraph` MCTS 因果模拟器超出 M1B3 范围；基线 hybrid cosine+BM25 搜索**推迟**（BM25 半边属文本检索而非向量基础设施，宜与检索语义一起、待记忆/查询契约定稿后采纳）；**无持久化**——基线向量与图是内存的，故不承诺 reopen 测试。

**测试**：向量 = insert/update/remove、最近邻查询、稳定 tie 排序、维度不匹配、非有限值、重复 id、top-k 边界、零维度、零向量处理；图 = 节点/边插入、neighbour 顺序、删边/删节点、缺失节点、重复边、环安全有界遍历、最短路。

---

## §5 M1C —— Canonical 治理基线 primitive 迁回

**原件**：[`../../01-architecture/m1c-governance-donor-primitives.md`](../../01-architecture/m1c-governance-donor-primitives.md)

**阶段目标**：从 master 治理实现取算法与 primitive，改编到冻结的 canonical 治理契约。状态 complete；分支 `reconstruct_v2`；Starting HEAD `af49988fd36be5a2e132a634acdabcaf4d29e7f8`。**明确不采纳先前架构。**

| 源模块 | 成熟度 | 采纳？ | 策略 |
|---|---|---|---|
| `guard.rs`（`PiiDetector`） | REAL | yes | ADAPT |
| `audit.rs`（`AuditHashChain`） | REAL | yes | ADAPT |
| `onion.rs`（`Permission`, `PermissionPack`, `PermissionOnion`） | PARTIAL | partial | ADAPT permission core only |
| `onion.rs`（`PrincipleOnion`, `DslOnion`） | PARTIAL | no | DROP |
| `self_disable.rs`（`SelfDisableGuard`, `Scanner`） | PARTIAL | no | DROP / DEFER |
| `gates.rs`（五闸 `GovernancePipeline`） | PARTIAL | no | DROP |
| `sovereignty.rs` | REAL | no | DEFER |

**M1C-A `input_security`（冻结契约）**：基线 `PiiDetector` 一个结构体混了三件事（PII 脱敏、prompt-injection 拦截、隐式安全分类），M1C-A 拆开——`PiiDetector::findings` → 结构化 `PiiFinding { kind, start, end }`，kind 为 `Email` / `Phone`（沿用基线的大陆手机号形状）/ `CredentialKey`（`sk-...`、`AKIA...`、`Bearer` token 形状），**偏移是字节偏移**；`PiiDetector::redact` → 稳定占位符 `[REDACTED_EMAIL]` / `[REDACTED_PHONE]` / `[REDACTED_CREDENTIAL]`；`PromptInjectionHeuristic::signals` → 结构化 `PromptInjectionSignal { kind, start, end }`，来自基线五个模式，大小写不敏感，编译期静态正则；`PromptInjectionHook` 与 `CredentialDisclosureHook` → canonical `GovernanceHook` 实现，检查**能力派发参数**并把发现映射到 `RequireApproval`，**绝不**映射到隐式全局 `Deny`，**单独的 Email/phone PII 不改变决策**。

**M1C-B `audit`（冻结契约）**：基线是真实 append-only 哈希链，但用 `chrono::Utc::now()` 追加并 hash 一个 `:` 分隔的 `format!` 串。M1C-B：`append` 取显式 `apeireth_core::kernel::Timestamp`（primitive 内部**无挂钟**）；canonical 序列化固定字段序 + 长度前缀（`sequence:u64`、`timestamp_epoch_millis:i64`、长度前缀 `event_kind` / `subject` / `previous_hash`）；创世 previous hash 是 64 字符零哈希；`verify()` 检测 sequence 不匹配、创世不匹配、previous-hash 指针断裂、记录 hash 损坏——**删除与重排因此失败**；**仅内存存储**，`ExecutionTrace` 未被替换。

**M1C-C `permission`（冻结契约）**：只采纳确定性核心——`Permission` 枚举（`ReadMemory`, `WriteMemory`, `ExecuteTool(String)`, `NetworkEgress(String)`, `ModifyIdentity`, `AdminOverride`）；`PermissionSet` 基于 `BTreeSet`（确定性迭代）；`PermissionPolicy::decision_for_capability` 把能力派发映射到既有 canonical `Decision`：缺授权且无 `AdminOverride` → `Deny`，已授权但标记需审批 → `RequireApproval`，否则 `Allow`；`PermissionGovernanceHook` 把该 policy 包成 `GovernancePipeline` 的 hook。

**拒绝（原文）**：基线五闸 `GovernancePipeline`（第二条带哲学键与 council 投票的流水线——canonical `GovernancePipeline` 已能组合 hook）；`PrincipleOnion` 与 `DslOnion`（facade 分层 + 硬编码产品哲学）；基线 `standard_agent` 权限包（硬编码 vendor 出站 `api.minimax.chat` 与产品专用工具授权）；`SelfDisableGuard`（与既有 `apeireth-core` self-disable 基线重复）；`LifecycleHandle`（基线 runtime facade）；`SovereignControl`（真实但属角色/暂停-恢复策略，超出 M1C 范围）。

**推迟**：持久化审计链后端；在每个治理事件上 runtime 接线审计链追加；runtime/gateway 的完成内容输入安全集成（canonical `GovernanceRequest` 当前携带能力参数但**不带 provider transcript 文本**——加它是 canonical context 扩展，不是 detector 改动）；`SovereignControl` 及角色/暂停-恢复策略；gateway auth、tool 沙箱策略、网络/出站策略、MCP 策略、凭据后端集成。

**安全局限（原文）**：`PiiDetector` 是启发式、基于模式，只探测配置的模式，**不探测所有 secret 或所有 PII**；`PromptInjectionHeuristic` 是给人工复核的信号，**不是** prompt injection 的完整防护，且匹配映射到 `RequireApproval` 而非当作安全边界；`AuditHashChain` 是**防篡改证据（tamper-evident）而非防篡改（tamper-proof）**——它证明内存记录变了，但不阻止变更也不持久化；脱敏是对配置类别的尽力字符串替换；测试只用假 fixture，不存不打印真 secret。

---

## §6 M2A —— Canonical 简单工具能力

**原件**：[`../../01-architecture/m2a-simple-tool-ports.md`](../../01-architecture/m2a-simple-tool-ports.md)

**阶段目标**：把三个低风险工具从 master 改编到冻结的 canonical `ToolCapability` / `Plugin` / `CapabilityRegistry` 架构。状态 complete；分支 `reconstruct_v2`；Starting HEAD `d9796345`；源 `origin/master:reconstruction_v2/crates/apeireth-tools/src/builtin/`。**明确不采纳基线 `Tool` trait、`ToolRegistry`、runtime tool 接线。**

| Tool | Source path | Strategy | Canonical implementation | Risk metadata | Status |
|---|---|---|---|---|---|
| Filesystem | `builtin/filesystem.rs` | ADAPT | `apeireth-tools-canonical::filesystem` | `medium` | read/list/stat 采纳；write/delete 推迟 |
| Search | `builtin/search.rs` | ADAPT | `apeireth-tools-canonical::search` | `low` | 本地确定性子串搜索采纳 |
| Repo | `builtin/repo_tools.rs` | REIMPLEMENT | `apeireth-tools-canonical::repo` | `low` | 固定只读 git 操作采纳 |

**关键设计决策（逐工具）**：Filesystem 复用操作模型（read/list）、可配置 root 概念、测试结构；**改编**为只读子集（read/list/stat）、**规范化路径包含**取代字符串 `..` 检查、显式文件大小上限、结构化 UTF-8 错误、确定目录序、JSON 结果；**拒绝**基线 write/delete；**推迟** write/delete/rename/copy 直到 M2B 沙箱强制存在。Search 复用字面量、大小写不敏感子串语义、默认/最大结果数、500KB 每文件内容上限、深度界、hidden/target/node_modules 跳过；**改编**为 root 约束 + 规范化、确定性 (path, line, text) 序、显式 `truncated` 标志、`.git` 也跳过、二进制文件跳过且不致命；**拒绝**无（基线 search 无测试，测试为新写）；**推迟**正则搜索、ignore 文件支持、结构化二进制检测。Repo 复用固定只读 git 命令集（`status`/`diff`/`log`/`branch`/`summary`）；**改编**为显式仓库根（`git -C`）、**无任意 `args` 字段**、仅固定参数构造、有界输出 + 显式截断、非 git 目录与 git 失败的结构化错误；**拒绝**基线 `args: Option<Vec<String>>` 任意参数路径；**推迟**任何 mutation 操作、任意 revision/path 选择、timeout 与进程封装（M2B）。

**冻结契约（安全边界）**：Filesystem 只读且由规范化路径包含做 root 约束，但**不是**进程/文件系统沙箱（M2B 才加真封装）；Search 只读本地 UTF-8 文本，对二进制文件是启发式（非法 UTF-8 文件被跳过）；Repo 跑固定只读 `git` 子进程，不是 shell 工具、不接受任意 git 参数，但**尚无进程 timeout 或 OS 封装**；Shell / browser / fetch-network / MCP 工具**未采纳**；**不声称任何真沙箱**。

---

## §7 M2B —— 进程封装与跨平台隔离

### §7.1 M2B 进程封装地基（原件 10）

**阶段目标**：建 canonical 进程执行边界。状态 complete；分支 `reconstruct_v2`；Starting HEAD `3ec66cd2c714178a0ce42e3218d28afdb247b22e`；源 `origin/master:reconstruction_v2/crates/apeireth-tools/src/sandbox.rs` 及同族文件。

| Feature | Baseline status | Canonical strategy | Final status | Evidence |
|---|---|---|---|---|
| Job Object 创建 | REAL | REIMPLEMENT low-level primitive | ENFORCED | `process/windows.rs::JobObject::create` 用 `CreateJobObjectW` + `SetInformationJobObject` |
| 子进程附加 | BROKEN / NOT INTEGRATED | REIMPLEMENT fail-closed spawn | ENFORCED | `process_executor::process_executor_attaches_child_to_a_real_job_object`；子进程第一条指令即见 `IN_JOB` |
| 分配时机 | spawn-then-(never)-assign | CREATE_SUSPENDED → assign → ResumeThread | ENFORCED (fail-closed) | `process/windows.rs` `spawn_and_supervise` |
| 后代在 job 内 | PARTIAL（Job Object 语义支持；基线从未附加） | ADAPT Job Object 树语义 | ENFORCED | `windows_tests::timeout_terminates_the_whole_job_tree` |
| Kill-on-close | REAL（基线 job 上设了 flag）但从未测试 | REIMPLEMENT + 真测试 | ENFORCED | `windows_tests::kill_on_job_close_terminates_a_running_child` |
| 内存上限 | DOC-ONLY / PARTIAL（基线设 256MB 但从无子进程被分配） | REIMPLEMENT opt-in | ENFORCED (opt-in) | `windows_tests::process_memory_limit_rejects_oversized_allocation` |
| 活跃进程数 | **基线未设** | REIMPLEMENT opt-in | ENFORCED (opt-in) | `windows_tests::active_process_limit_blocks_extra_child_creation` |
| RestrictedToken 创建 | PARTIAL（`OpenProcessToken` 成功才建） | DEFERRED | DEFERRED | 基线 `sandbox.rs` 调 `CreateRestrictedToken`；无启动路径用它 |
| Restricted-token 启动 | NOT ENFORCED | DEFERRED | DEFERRED | 基线用 `std::process::Command` / `tokio::process::Command` 启动；`restricted_token` 从未被用 |
| Timeout | PARTIAL（仅 `ToolSynthesizer` 对 `cmd.output()` 用 `tokio::time::timeout`） | REIMPLEMENT 确定性监管循环 | ENFORCED | `timeout_terminates_the_child`；`timeout_terminates_the_whole_job_tree` |
| stdout/stderr 上界 | PARTIAL（synthesis 读满后才截断） | REIMPLEMENT 读时上界 + 显式截断标志 | ENFORCED | `stdout_limit_truncates_and_reports`、`stderr_limit_truncates_and_reports` |
| Worktree 沙箱 | REAL 但只是 workspace 隔离，非 OS 封装 | DEFER（未来 workspace 隔离 primitive） | DEFERRED | `origin/master:.../worktree.rs` |
| ToolSynthesizer 集成 | BROKEN（存了 `Arc<PlatformSandbox>` 但从未附加子进程） | DROP / 不采纳 | NOT ADOPTED | `origin/master:.../synthesis.rs`；`sandbox` 字段是 `#[allow(dead_code)]` |

**基线结论（原文）**：master 有真的 Job Object 和 restricted-token 句柄，但 spawn 出去的 shell/repo/synthesis 子进程**从不调用 `assign_process`**，所以沙箱是装饰而非强制。M2B 因此**没有**直接采纳 `PlatformSandbox`，只复用正确的低层 Windows primitive 并建了一个 fail-closed 的 canonical executor。

**canonical owner（冻结契约）**：进程执行边界在 `crates/apeireth-tools-canonical/src/process/`——`mod.rs`（`ProcessRequest` / `ProcessLimits` / `ProcessResult` / `ProcessError` + 平台中立监管循环）、`windows.rs`（Windows Job Object + CREATE_SUSPENDED → assign → resume）、`platform.rs`（非 Windows 仅 guardrails 实现）。放在工具基础设施 crate 内，因为 builtin 工具能力是其唯一当前消费者；只依赖同 crate 公开类型（Windows 上加 `windows-sys`）；**不**依赖 runtime / gateway / provider / governance，**不**引入第二个 registry/manager。

**公开契约（冻结契约）**：`ProcessRequest { executable, args, working_directory, environment, limits }`（builder `ProcessRequest::new("git").with_args([...]).with_working_directory(...).with_limits(...)`；**无 `command: String`**、无 `cmd /c`、无 `sh -c`）；`ProcessLimits { max_runtime, max_stdout_bytes, max_stderr_bytes, max_process_memory_bytes, max_active_processes, kill_on_job_close }`（`default()` 有界：30s、64KiB stdout、64KiB stderr、kill-on-close 开；`unrestricted()` 是显式退出且**不被 builtin 工具使用**）；`EnvironmentSpec { Inherit, Clear, Explicit(Vec<(OsString, OsString)>) }`；`ProcessResult { termination, stdout, stderr, stdout_truncated, stderr_truncated, enforcement }`；`TerminationReason { Exited { code }, TimedOut { code } }`；`PlatformEnforcement { platform, containment, fail_closed_spawn }`；`ProcessError { InvalidConfiguration, SpawnFailed, ContainmentFailed, PlatformUnsupported, Io }`。**子进程非零退出是 `ProcessResult`，绝不是 `ProcessError`。**

| Boundary | Windows | Non-Windows |
|---|---|---|
| Timeout | ENFORCED via Job Object `TerminateJobObject`（整棵树） | ENFORCED（仅直接子进程 `child.kill`） |
| Output bound | ENFORCED（读时 cap + 截断标志） | ENFORCED（同） |
| Working directory | ENFORCED（`Command::current_dir`） | ENFORCED（同） |
| Environment policy | ENFORCED（`env_clear` / 显式变量） | ENFORCED（同） |
| Job Object attachment | ENFORCED（首条指令前，`CREATE_SUSPENDED`） | 不适用 |
| Kill-on-close | ENFORCED | 不适用 |
| Descendant containment | ENFORCED（Job Object 树语义 + 真测试） | PARTIAL（仅直接子进程 kill；无 namespaces/cgroups/seccomp） |
| Memory limit | ENFORCED（配置时，opt-in，已测） | DEFERRED |
| Active process limit | ENFORCED（配置时，opt-in，已测） | DEFERRED |
| Restricted token | DEFERRED | DEFERRED |
| Filesystem OS isolation | NOT ENFORCED（不声称） | NOT ENFORCED |
| Network isolation | NOT ENFORCED（不声称） | NOT ENFORCED |

**原文措辞纪律**：非 Windows 被准确描述为 **process execution guardrails only**，不是 OS 特权沙箱。

**Windows 安全启动路径（冻结契约）**：`std::process::Command` + `creation_flags(CREATE_SUSPENDED)` + stdout/stderr/stdin 管道 → `CreateProcessW`（挂起）→ `AssignProcessToJobObject` → `ResumeThread`（主线程，经 Toolhelp 快照）→ 监管循环（`try_wait` + timeout + 有界输出读取器）。**子进程在 job 附加前不能执行。** 证据测试 `process_executor_attaches_child_to_a_real_job_object` 跑一个 helper，其第一条指令调用 `IsProcessInJob`，观察到 `IN_JOB`。

**测试**：真子进程测试在 `crates/apeireth-tools-canonical/tests/process_executor.rs`，用测试专用 helper 二进制 `sandbox-test-child`（`src/bin/sandbox_test_child.rs`，只接受固定模式、不解析任何用户命令串）。通用（全平台）13 项：成功执行 + stdout 捕获、结构化参数保留（Unicode 与空格）、显式工作目录、stderr 捕获、非零退出是结果而非 executor 错误、spawn 失败是 executor 错误、timeout 终止子进程、stdout 上限截断并上报、stderr 上限截断并上报、大 stdout + 大 stderr 同时不死锁、环境清空、环境显式模式。Windows-only 5 项：子进程附到真 Job Object、kill-on-job-close 终止运行中子进程、timeout 终止整棵 job 树、活跃进程上限阻止额外子进程创建、进程内存上限拒绝超大分配。**这些测试里没有 mock process executor。**

**局限（原文）**：`std::process::Command` 负责命令行引号、Unicode、工作目录、环境、管道；自定义 Windows 代码只加挂起创建、job 分配、线程恢复。若当前进程已在不允许嵌套 job 的 Job Object 内，`AssignProcessToJobObject` 失败，executor 返回 `ContainmentFailed` 并**杀掉挂起的子进程**，而不是无封装继续。非 Windows 上 timeout 只杀直接子进程；daemonize 的后代可能残留。非 Windows 上若后代继承 stdout/stderr 管道且在直接子进程退出后仍持有，输出读取器由 channel timeout 设界以免永久挂起——该边界下**输出尾部可能丢失**。**Restricted-token 启动未实现**——Job Object 是进程封装/资源边界，**不是**文件系统或网络边界。

**对 M2C 的前置要求（原文）**：M2C Shell 只有在进程边界被认为适合任意命令执行时才可开始；fail-closed Windows 启动路径必须保持完整且有测试；`max_runtime`、输出上界、job 附加必须保持为任何 M2C 命令工具的默认；shell 的环境策略必须显式/清空而非环境继承；对不可信命令执行应重新评估 restricted-token 或等价身份/ACL 加固；**网络出站仍是 M2D，不得声称**。

### §7.2 M2B-X 跨平台进程隔离（原件 11）

**阶段目标**：把进程隔离从「Windows 专属」扩到三平台，并让**能力声明与实际强制力**分离。状态 complete（Windows / Ubuntu / macOS CI 验证）。

**canonical 契约（冻结契约）**：公开层（`crates/apeireth-tools-canonical/src/process/mod.rs`）**只表达平台中立类型**——`ProcessRequest`（结构化可执行 + 参数 + 显式 cwd + 环境策略 + 上限 + 隔离要求）、`ProcessLimits`（timeout、stdout/stderr 上界、可选内存/进程数/CPU/文件大小上限）、`IsolationProfile`（`Trusted`/`Restricted`/`Untrusted` 预设）、`IsolationRequirement`（调用方声明的最小 `(IsolationCapability, EnforcementLevel)` 对）、`IsolationCapabilities`（可观测的平台能力集）、`ProcessResult`、`PlatformEnforcement`（平台种类 + 本次运行实际生效的能力）、`ProcessError`（含 `IsolationRequirementUnsatisfied` 与 `UnsupportedLimit`）。**后端模块（`windows`/`linux`/`macos`）含全部平台机制**；`JobObject` / `RestrictedToken` / `CreateProcessW` / `cgroup` / `seccomp` / `Landlock` / `namespace` / `setrlimit` / `Seatbelt` / `sandbox-exec` 等名字**不出现在公开契约源码中**。

**后端模型（冻结契约）**：`ProcessExecutor` → `WindowsProcessBackend` / `LinuxProcessBackend` / `MacOsProcessBackend`，用**静态 `cfg` 分派**；**未创建** `PlatformManager`、`SandboxManager`、`BackendRegistry`、`SecurityFacade`。

**强制等级（冻结契约）**：`Unsupported < Partial < Enforced`。要求 `(capability, required_level)` 仅当平台对该能力的实际等级 `>= required_level` 时满足；**缺要求时在 spawn 任何子进程之前失败**，返回 `ProcessError::IsolationRequirementUnsatisfied`。

| Capability | Windows | Linux | macOS | Evidence |
|---|---|---|---|---|
| Structured spawn | ENFORCED | ENFORCED | ENFORCED | `structured_args_are_preserved_with_unicode_and_spaces` |
| Explicit cwd | ENFORCED | ENFORCED | ENFORCED | `working_directory_is_explicit_not_ambient` |
| Timeout | ENFORCED | ENFORCED | ENFORCED | `timeout_terminates_the_child` |
| stdout/stderr bound | ENFORCED | ENFORCED | ENFORCED | `stdout_limit_truncates_and_reports` / `stderr_limit_truncates_and_reports` |
| Environment isolation | ENFORCED | ENFORCED | ENFORCED | `environment_clearing_denies_ambient_secrets` |
| Process-tree containment | ENFORCED (Job Object) | PARTIAL (process group；创建新组/会话的后代可逃逸) | PARTIAL（同） | Windows: `timeout_terminates_the_whole_job_tree`；Linux/macOS: `timeout_terminates_the_whole_process_group_tree` |
| Memory limit | ENFORCED when configured (Job Object) | PARTIAL (`RLIMIT_AS`) | PARTIAL (`RLIMIT_AS`) | Windows: `process_memory_limit_rejects_oversized_allocation`；Unix: exec 前 setrlimit |
| Process-count limit | ENFORCED when configured | PARTIAL (`RLIMIT_NPROC`，按 UID) | PARTIAL（同） | Unix 测试刻意避开全局 UID 依赖 |
| CPU limit | UNSUPPORTED | ENFORCED (`RLIMIT_CPU`) | ENFORCED (`RLIMIT_CPU`) | Unix: pre-exec setrlimit |
| File-size limit | UNSUPPORTED | ENFORCED (`RLIMIT_FSIZE`) | ENFORCED (`RLIMIT_FSIZE`) | Unix: pre-exec setrlimit |
| Privilege reduction | ENFORCED（restricted-token 启动可行时）；否则 UNSUPPORTED | PARTIAL (`PR_SET_NO_NEW_PRIVS`) | UNSUPPORTED | Windows: `privilege_reduction_requirement_is_enforced_or_fails_closed`；Linux: `privilege_reduction_requirement_sets_no_new_privs`；macOS: `privilege_reduction_is_honestly_unsupported` |
| Filesystem isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | `current_platform_capabilities` 运行时探测 |
| Network isolation | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED | `unsupported_network_requirement_fails_closed_before_child_starts` |
| Fail-closed pre-exec containment | ENFORCED (CREATE_SUSPENDED → assign → resume) | ENFORCED (pre_exec setup) | ENFORCED (pre_exec setup) | 各后端启动路径；fail-closed 测试用标记文件证明子进程从未启动 |

**预设（冻结契约）**：`Trusted` = 结构化 spawn + 显式 cwd + timeout + stdout/stderr 上界；`Restricted` = Trusted + 环境隔离 + 进程树遏制 PARTIAL + 内存/进程数/CPU/文件大小 PARTIAL + 特权削减 PARTIAL + fail-closed pre-exec 强制 ENFORCED；`Untrusted` = Restricted + 特权削减 ENFORCED + 文件系统隔离 ENFORCED + 网络隔离 ENFORCED——**在当前后端该预设不可满足，这是有意为之且 fail-closed**。

**Fail-closed 规则（冻结契约）**：要求检查发生在子进程启动前；平台无法强制的可选上限在启动前以 `ProcessError::UnsupportedLimit` 拒绝；**要求未满足时不回退到更弱模式**；安全敏感预设**绝不静默降级运行**。

**安全声明（原文）**：文件系统隔离本阶段三平台全 `UNSUPPORTED`；网络隔离三平台全 `UNSUPPORTED`；特权隔离——Windows 在账号持所需特权时走 restricted-token 子进程启动，否则 `UNSUPPORTED`（**普通非管理员用户通常见到 `UNSUPPORTED`**）；Linux 只有 `PR_SET_NO_NEW_PRIVS`，只宣称 PARTIAL，**明确不称 seccomp 或 namespace 沙箱**；macOS `UNSUPPORTED`。

**CI 证据**：workflow `M2B-XV three-OS process isolation validation`（`.github/workflows/m2b-xv-isolation.yml`）；run `https://github.com/Apeireth/Apeireth/actions/runs/32871298920`；commit `49ea32ec8947d5b184de72c3af6a513c73fc3127`（`test(tool): accept canonicalized cwd path on macOS`）。各 job 跑 `cargo check -p apeireth-tools-canonical --locked`、`cargo test -p apeireth-tools-canonical --locked`、`cargo test -p apeireth-tools-canonical --test process_executor --locked -- --nocapture`。结果：Windows success（25 lib + 3 architecture passed；21 process-executor passed）、Ubuntu success（25 lib + 3 architecture；18）、macOS success（25 lib + 3 architecture；17）。

**残余缺口（原文）**：Linux/macOS 进程树遏制基于进程组，创建自己进程组/会话的后代可逃逸；**本阶段无** Linux cgroup v2 / Landlock / namespace / seccomp 强制（cgroup v2 与 Landlock 仍是未来候选）；macOS 对任意子进程**没有受支持的**文件系统/网络沙箱机制，且**不使用私有 API**；Windows 文件系统/网络隔离未实现；**在出站与文件系统隔离要求落定（M2D+）之前，任何平台都还不具备任意 shell 的就绪条件**。

---

## §8 M2C —— 审批恢复生命周期与 Trusted Shell

### §8.1 M2C-R 就绪评审（原件 12，只读 / 设计冻结 / 威胁模型）

**性质**：**READ-ONLY / DESIGN-FREEZE / THREAT-MODEL only**——**不实现 shell**，不加 `ShellTool`、`cmd /c`、`sh -c`、`powershell` 或任何任意命令执行能力。状态 review complete；分支 `reconstruct_v2`；Starting HEAD `732abddce3d1a22eb8f81cd9adf50e23ecc2d011`；工作树干净；push：no（文档评审，最终验证后再推）。

**总体裁决（原文）**：**BLOCKED_BY_APPROVAL_LIFECYCLE.** Trusted Shell 是当前跨平台隔离事实下唯一技术上自洽的 shell 形态，且**仅在逐次人工审批下**自洽。但 canonical Runtime 停在 `RuntimeError::ApprovalRequired`，无持久化的 pending-approval 实体、无稳定 approval id、无 approve/reject API、无 resume-same-turn 路径。此时实现 `tool.shell` 会得到一个能说「RequireApproval」却无法在人类同意后安全恢复的能力。**因此下一阶段必须是 M2C-A —— Canonical Approval Resume Lifecycle，而不是 Shell 实现。**

**三形态定义（冻结契约）**：**Trusted Shell** —— 用户明知并批准**一条**确切的本地命令/脚本；命令以用户有效 OS 账号权限运行；产品**不做**文件系统隔离声明、**不做**网络隔离声明；提供三平台都有的常见进程护栏（经 `ProcessExecutor` 的结构化 shell 调用、显式 cwd、有界 timeout、有界 stdout/stderr、与环境进程环境的隔离）；可选资源上限（内存/进程数）可配但不属跨平台基线。**Trusted 意味着「用户接受了这条命令的权限」，不意味着命令本身安全。** **Restricted Shell** —— 平台能给才要求削减身份/权限（Windows restricted-token 启动（可用时）；Linux `PR_SET_NO_NEW_PRIVS`（只宣称 PARTIAL）；macOS 没有）；要求更强进程封装（Windows Job Object ENFORCED；Linux/macOS 进程组 PARTIAL 仅在披露逃逸风险后接受）；资源上限至少须 PARTIAL 可强制且在用该预设处配置；文件系统/网络隔离仍可能缺席——**故 Restricted Shell 不是沙箱 shell**。**Untrusted Shell** —— 适用于自主的、模型生成的任意命令，要求强物理封装：进程树遏制 ENFORCED + 特权削减 ENFORCED + 文件系统隔离 ENFORCED + 网络隔离 ENFORCED；**这在当前所有后端上有意不可满足，且必须 fail-closed。**

**关键区分（原文）**：**审批**回答「用户批准了这个动作吗」；**沙箱**回答「这个动作物理上能影响什么」。`RequireApproval` 决策**不是**封装。经审批的 Trusted Shell 可能有意的宽本地访问——只要标签诚实，这是有效产品形态。沙箱是 `ProcessExecutor` / `IsolationRequirement` 的事，审批是 `Governance` 的事，**二者互不替代**。

**威胁模型（22 行大表，四档判定：PREVENT / LIMIT / REQUIRE APPROVAL / DO NOT PREVENT）**：任意文件读、任意文件写、文件型凭据窃取（SSH key / 浏览器 token / git 凭据）、SSH key 访问、浏览器/会话 token 访问、git 凭据访问、网络外泄、网络扫描、本地服务访问、持久化、fork/process bomb、磁盘写满、内存耗尽、CPU 耗尽、工作目录逃逸、命令替换、下载并执行、子进程逃逸、平台特定提权——在 Filesystem isolation UNSUPPORTED 与 Process `NetworkIsolation` UNSUPPORTED 的事实下，**在 Untrusted 形态全是 Blocker**；环境 secret 窃取由 Shell 环境契约 PREVENT。进程派生属预期能力（受 timeout 限定）；shell 元字符注入与命令替换是 shell 的**有意**能力（由封装承担风险）。完整 22 行判定表见原件。

**产品合同（原文，冻结契约）**：
> `tool.shell` 是经用户批准的本地 shell。它以用户 OS 账号权限运行确切被批准的命令。它**不是**文件系统沙箱，也**不是**网络沙箱。Apeireth 在支持处约束进程生命周期与输出，并保护子进程不接触环境里的 secret。

强制可因平台而异，但**合同不暴露** `JobObject`、`rlimit`、`no_new_privs` 或任何后端机制名；**用户不得选择 `JobObject` 或 `rlimit`**。M2C v1 **只发一种 canonical 形态：TrustedShell**（逐次审批）；RestrictedShell 与 UntrustedShell 是后续独立阶段。

**M2C v1 初始产品决定（原文）**：只发 TrustedShell；**始终** `RequireApproval`，永不自动批准；显式 cwd；Clear/Explicit 环境；有界 timeout；有界输出；ProcessExecutor 封装；不声称文件系统隔离、不声称网络隔离；UI/API 明确警告命令以本地用户权限运行；对自主/后台操作禁用。**两个强制前置**：① **M2C-A 审批恢复生命周期必须先存在**（否则返回 `RequireApproval` 的 shell 调用无法被恢复）；② 工具**默认禁用**，仅显式启用时注册；**无显式审批策略时启用它必须 fail-closed（Deny），不得落到 `AllowAll`**。

**审批展示合同（冻结契约，10 项必填字段）**：能力标识 `tool.shell`；shell 种类 / 解析出的可执行文件（如 `cmd.exe`、`/bin/sh`）或稳定标签；**完整原始**命令/脚本（保留换行、空格、`&&`、`;`、管道、重定向等所有元字符；多行内容必须保留换行，且除非用户可展开全文否则不截断）；cwd；timeout（默认与配置值）；配置了的资源上限（内存、进程数、CPU、文件大小）；环境模式（`Clear` 或 `Explicit`，**shell 永不用 `Inherit`），带确切环境键值或敏感时的安全摘要；网络隔离状态（`UNSUPPORTED` / 「none」）；文件系统隔离状态（同）；平台隔离预设标签（`TrustedShell` 或预设名，**但非内部机制名**）。可选：风险原因、prompt-injection 信号（作为上下文，不是 shell 过滤器）。**不得有批准后还能变化的隐藏字段。**

**不可变审批绑定（冻结契约）**：被批准的必须就是被执行的——绑定 tool id（`tool.shell`）、**完整命令/脚本字节**（稳定 UTF-8；不做改变语义的规范化）、解析出的 shell 可执行文件/shell 种类、cwd、环境模式与显式环境条目、timeout、输出上界、隔离要求/预设与任何配置的资源上限。**M2C v1 必须拒绝「绑定操作与被批准操作不精确匹配」的执行。** 当时状态：canonical Governance API 只携带 `CapabilityId` 与 `arguments`，**无操作身份/hash、无冻结执行操作**——原文判为**实现 blocker**，要求 M2C-A 先加 canonical 审批绑定 primitive。

**TOCTOU（原文）**：批准 → 时间流逝 → 执行；当前架构无法快照或绑定 workspace 文件系统。M2C v1 至少要绑定命令文本、cwd、环境模式/条目、timeout 与输出/资源上限、隔离要求/预设、shell 种类/可执行文件。若 cwd 路径在批准与执行之间改变含义（如 symlink 重定向），v1 **不声称**能检测——该残余 TOCTOU 必须作为已知局限记录。

**接口形态决定（原文）**：A（原始命令串交系统 shell 解释）是真正的 Shell 能力，**仅**作为显式 `tool.shell` 合法；B（可执行文件 + 结构化参数）是既有 `ProcessExecutor` 契约，仍是非 shell 工具的唯一进程 API；C（两者拆两个工具）v1 不需要。**决定：A 作为显式 ShellCapability；B 保持 ProcessExecutor；C 不是 v1。** 原始 shell 语法被隔离在 `tool.shell` 内，绝不藏在 Repo/ProcessExecutor 里。

**架构回归（原文）**：评审**不建议改**任何东西——Runtime 单一编排、`ToolCapability` 边界、`ProcessExecutor` 边界、Governance 三态、`PluginRegistry`/`CapabilityRegistry` 唯一真源；**不做第二个 shell runtime**。

**当时的后续要求**：未来 shell 动作必须产出 canonical 事件（`GovernanceEvaluated`、`CapabilityDispatched`、`CapabilityCompleted`/`ToolFailed`、审批事件、进程终止事实入工具结果、工具结果入 transcript）；**无裸 CoT、无 shell 专属 trace 子系统**；`AuditHashChain` 存在但未接线，**不把它的接线做成 shell blocker**；**不加 shell 专属 SQLite 表**；provider 对 shell 实现无感知，不做 vendor 分支；真实 shell 测试只能用无害命令（打印 fixture / `pwd` / 指定退出码 / 有界 sleep），**不破坏文件系统、不访问公网、不用真凭据**。

> **原件自带的后来补充（保留）**：文中「无 persisted approval / 无 resume」已被 M2C-A 实现推翻，见 `crates/engine/runtime/src/canonical/approval.rs`。

### §8.2 M2C-A 审批 / 恢复生命周期（原件 13）

**阶段目标**：解决 M2C-R 判定的 blocker。状态 complete；分支 `reconstruct_v2`。

**问题（原文）**：M2C-A 之前 `Runtime::execute` 返回 `RuntimeError::ApprovalRequired` 并停下这一轮。没有持久化待决操作，没有稳定 approval id，也无法事后批准或拒绝。一个需要审批的工具能挡住执行，但**无法被恢复**。

**归属（冻结契约）**：**Governance** 只拥有「这个操作是否需要审批」；**Runtime** 拥有「什么在等待、冻结的确切操作是什么、如何恢复、是否已被解决、这一轮从哪里继续」；**Session / 持久化** 通过既有 `SessionStore` seam 拥有持久性；**ToolCapability** 拥有工具执行语义；**ProcessExecutor** 拥有 OS 进程执行；**Gateway / CLI** 只负责呈现/适配。**不存在** `ApprovalManager`、`ApprovalRuntime`、`ApprovalExecutor`、`ApprovalEngine`、`ApprovalRegistry`、`ApprovalPipeline`。

**新公开契约（冻结契约）**：`ApprovalId` —— 在 `apeireth_core::kernel::ids` 中按 `SessionId`/`TraceId`/`RequestId` 同一模式生成的 UUID 型 id。`TurnOutcome { Completed(TurnResponse), PendingApproval(PendingApprovalView) }`；`Runtime::execute_outcome` 返回 `TurnOutcome`；`Runtime::execute` 保留为兼容包装，把 `PendingApproval` 映射回 `RuntimeError::ApprovalRequired`。`ApprovalDecision { Approve, Reject { reason: Option<String> } }`；`ApprovalResolution { Resumed(TurnOutcome), AlreadyResolved { status: ApprovalStatus }, Expired, NotFound }`；`Runtime::resolve_approval(session, approval_id, decision)` 解析已存操作。**解析者只提供决策与可选的人类理由；绝不提供替换的工具参数、cwd、脚本文本或进程配置。** 状态机 `Pending → Approved → Consumed`；`Pending → Rejected`；`Pending → Expired`——**`Pending` 是唯一可恢复态**，`Rejected`/`Expired`/`Consumed` 是终态，**`Approved` 是原子认领态**（认领后任何其他解析者都不可执行该操作），`Consumed` 在工具调用与 transcript 追加之后记录。

**冻结延续（冻结契约）**：`FrozenTurnContinuation` 存 `request_id`、`trace_id`、`model`、`round`、**原始 assistant `tool_calls` 批次**、`next_tool_index`、可选 `approved_tool_index` / `approved_approval_id`。**绝不重新查询 provider 去再生成原 tool call；原 `ToolCall` 批次是权威。**

**操作指纹（冻结契约）**：`operation_fingerprint_with_invocation` 用 SHA-256 + canonical JSON 哈希：action kind、capability id、tool name、tool call id、tool arguments、能力侧冻结的 `effective_invocation`（若提供）、session id、request id、round。**JSON 对象键递归排序；数组保持顺序；对象插入序不改变指纹。**

**持久化与重开（冻结契约）**：待决审批存进 `Session` 作为编排元数据——`Session::approvals: BTreeMap<ApprovalId, PendingApproval>`、`Session::active_approval_id: Option<ApprovalId>`；经既有 `SessionStore` seam 持久化，因此与配置的 store 一样耐久；内存 store 支持同进程内 runtime rebuild/reopen；未来耐久 `SessionStore` 后端让同一生命周期跨进程重启也耐久。

**并发（冻结契约）**：`Runtime` 拥有 per-session 锁（`SessionLocks`）；不同 session 可并发；同一 session 在另一操作进行中时不能开新轮或解析审批；**原子认领在 per-session 锁内完成，因此两个并发批准者不能双重执行**；有 pending 审批的 session 上开新轮返回 `RuntimeError::SessionApprovalPending`。

**过期（冻结契约）**：每个待决审批有 `expires_at`，由 runtime 时钟与 `RuntimeConfig::approval_ttl_ms`（默认 **5 分钟**）计算；**过期是惰性的**——只在 `resolve_approval` 时检查；过期审批不可被批准。

**At-most-once 语义（冻结契约）**：实现的保证是**批准后的 at-most-once 执行**——一次解析赢得 `Pending → Approved` 认领；重复批准/解析观察到终态且**绝不重执行**；若进程在认领后、工具调用前崩溃，**动作丢失（未执行）**，审批仍以 `Approved` 态留在 store 里。**原文明确：没有虚假的事务性 exactly-once 声明。**

**测试**（`crates/apeireth-runtime/tests/canonical_approval_lifecycle.rs`）：pending 结果且批准前零工具调用；批准后执行一次并继续剩余 tool calls；拒绝则跳过工具且模型能恢复；双重批准执行一次；并发批准执行一次；先拒后批不执行；过期审批不执行；pending 审批在同一 store 上 runtime rebuild 后存活；pending 时新轮被挡；认领保存**严格早于**工具调用；认领保存失败则工具不调用；认领后调用前崩溃不重执行；工具副作用后、consumed 保存前崩溃不重执行；**指纹损坏则 fail-closed 且不调用**；非法 shell 请求**绝不**产生 pending 审批。另有 `crates/apeireth-runtime/tests/canonical_architecture_invariants.rs` 证明 canonical runtime 源码中**不存在并行的审批引擎**。

### §8.3 M2C-T Trusted Shell（原件 14）

**阶段目标**：落 Trusted Shell。状态 complete（实现；三 OS 验证在 M2C-XV）；分支 `reconstruct_v2`。

**产品合同（冻结契约）**：`tool.shell` 是 **Trusted Shell** —— 模型提出本地 shell 脚本，人类看到**确切的生效调用**并显式批准，然后 Apeireth 经 canonical `ProcessExecutor` **执行一次**。**Trusted Shell 未被沙箱化**：以用户 OS 账号的文件系统与网络权限运行；三平台文件系统隔离与网络隔离仍 `UNSUPPORTED`，**不声称**。

**默认（冻结契约）**：**Shell 默认禁用。** `BuiltinToolsPlugin::new` 只注册 `tool.filesystem`、`tool.search`、`tool.repo`；shell **仅当**显式提供 `BuiltinToolsOptions { shell: Some(TrustedShellConfig) }` 时注册。

**能力声明（冻结契约）**：capability id `tool.shell`；模型面名称 `shell`；risk metadata `"high"`；声明文本：「Executes a platform-native local shell command after explicit user approval. Runs with the user's OS account authority; not a filesystem or network sandbox.」

**模型可提供的参数（冻结契约）**：仅 `command`（必填，≤ 64 KiB UTF-8）、`cwd`（可选，相对受信任 workspace root）、`timeout_ms`（可选，默认 30000，最大 300000）。**模型不能**提供 environment、shell 可执行文件、network mode、sandbox mode、isolation requirement 或资源上限——**安全配置由应用侧掌控**。

**平台 shell（冻结契约）**：Windows `cmd.exe` + `/D /S /C <command>`；Linux `/bin/sh` + `-c <command>`；macOS `/bin/sh` + `-c <command>`。**PowerShell 推迟**；**不假定** `bash`/`zsh`。

**环境（冻结契约）**：**永不用 `EnvironmentSpec::Inherit`**；Shell bootstrap 构建显式最小环境——Windows：父进程存在时带 `SystemRoot`、`WINDIR`、`TEMP`、`TMP`、`PATH`、`PATHEXT`、`COMSPEC`，**不带** `USERPROFILE`/`HOMEDRIVE`/`HOMEPATH`；Linux/macOS：`PATH=/usr/local/bin:/usr/bin:/bin`、`TMPDIR=/tmp`、`LANG=C.UTF-8`，**不带 `HOME`**。**不继承** provider API key、GitHub token、AWS 凭据、云凭据、代理凭据或 Apeireth 内部 secret。

**工作目录（冻结契约）**：每次调用都有显式 cwd；受信任配置给出 workspace root；相对 `cwd` 在 root 下解析并规范化；不存在、不是目录或逃出 root（含 symlink 逃逸）的路径被拒。**显式 cwd 是执行上下文，不是文件系统沙箱。**

**上限（冻结契约）**：脚本大小默认/最大 64 KiB UTF-8（可配）；timeout 默认 30s、最大 5m、**对审批可见**；输出默认 64 KiB stdout 与 64 KiB stderr，返回截断标志；**无零时限也无无限时限**。

**进程要求（冻结契约）**：`ShellTool` 构建显式 `IsolationRequirement` —— `StructuredSpawn Enforced`、`ExplicitCwd Enforced`、`Timeout Enforced`、`StdoutLimit Enforced`、`StderrLimit Enforced`、`EnvironmentIsolation Enforced`、`ProcessTreeContainment Partial`、`FailClosedPreExecutionContainment Enforced`；**Trusted 模式不要求** `FilesystemIsolation`、`NetworkIsolation`、`PrivilegeReduction`。

**审批冻结与冻结执行（冻结契约）**：`ToolCapability` 新增一对可选 canonical hook —— `freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult>` 与 `async fn invoke_frozen(&self, call: &ToolCall, frozen: Option<&FrozenInvocation>) -> ToolResult`。`FrozenInvocation` 分离两个概念 —— `payload`（工具自有的、不透明、带版本的执行载荷）与 `display`（脱敏的人面向视图，供 `PendingApprovalView` 用）。`ShellTool` 冻结版本化的 `ShellFrozenInvocation`，字段为 `version`（`1`）、`shell_executable`、`shell_args`（**确切 argv，含确切脚本**）、`cwd`（规范化路径）、`timeout_ms`、`max_stdout_bytes` / `max_stderr_bytes`、`environment`（**实际的显式环境值**）、`isolation`（确切 `IsolationRequirement`）。**display 视图只含环境变量名，绝不含值。** 被批准的 shell 派发执行 `ShellTool::invoke_frozen`，**只从冻结字段**反序列化并构建 `ProcessRequest`；批准执行期间**绝不**调 `resolve_cwd_for`、`selected_shell`、`minimal_environment`、`resolve_timeout_ms`。**若冻结载荷无法反序列化或版本不支持，工具返回 `ToolResult` 错误 `FrozenInvocationUnavailable`，而不是静默使用当前配置。** 非法 shell 请求（非法 cwd、空命令、超长命令、非法 timeout）从 `freeze_invocation` 返回 `ToolResult` 错误，**因此绝不产生 `PendingApproval`**。

**结果合同（冻结契约）**：shell 结果含 `exit_code`、`timed_out`、`stdout`、`stderr`、`stdout_truncated`、`stderr_truncated`。**非零退出是正常 shell 结果，不是进程错误**；spawn/封装错误以结构化 `ToolResult` 失败返回。

**非目标（原文）**：无交互 shell、无 PTY、无 stdin 流式、无后台/分离进程 API、无持久任务、无 SSH、无预设 shell、无命令黑名单、无命令改写/净化、无 PowerShell v1。

**测试**：`crates/apeireth-tools-canonical/src/shell.rs`（schema、大小、timeout、cwd 逃逸、声明诚实性、冻结校验、脱敏 display、配置变更后仍按冻结 cwd 执行）；`tests/shell_execution.rs`（真实无害 echo、cwd、非零退出、有界 timeout、unicode）；`crates/apeireth-runtime/tests/canonical_shell_approval_e2e.rs`（provider tool call → governance `RequireApproval` → pending approval → approve → 真实 shell 执行 → provider 续写；拒绝路径；非法 shell 永不 pending）。

---

## §9 M2D —— 跨平台网络 / 出站强制

**原件**：[`../../01-architecture/m2d-network-egress-enforcement.md`](../../01-architecture/m2d-network-egress-enforcement.md)

**阶段目标**：建受控出站边界。状态：**M2D-A + M2D-B complete**（受控 egress 策略 + transport）；**进程网络隔离后端在三平台审计后仍全 `UNSUPPORTED`**——这是诚实的平台能力事实，不是受控出站边界的 blocker。分支 `reconstruct_v2`。

**两个不同边界（冻结契约）**：**受控应用出站**（Apeireth 控制请求）走 `Runtime → Governance → ToolCapability → EgressPolicy → EgressTransport → HTTP/DNS`；**任意进程网络隔离**（子进程自开 socket）走 `ToolCapability → ProcessExecutor → 平台 NetworkIsolation 后端 → 子进程`。**它们是不同 API 与不同安全保证**——安全的 HTTP client 不限制子进程里的 `curl`；Job Object / 进程组也不会让 HTTP 目的地策略变安全。

| Baseline primitive | Classification | Notes |
|---|---|---|
| `apeireth-http-client/src/egress.rs` | STUB | `check_outbound` 恒 `Ok`；无策略 |
| `apeireth-http-client/src/lib.rs` | STUB | `get`/`post` 返回捏造响应 |
| `apeireth-gateway/src/egress.rs` | PARTIAL | 有默认 host allowlist、port allowlist、IPv4 私网/回环检查，但**只校验 IP 字面量**——主机名在 allow 决策前不解析；通配后缀匹配粗糙 |
| `apeireth-tool-fetch/src/http_fetch.rs` | BROKEN for SSRF | 用不透明 client 且 `follow_redirects: true`；无重定向重校验、无 DNS 钉扎、无响应上界（除调用方配置） |
| `apeireth-tool-browser/src/fetch.rs` | BROKEN for SSRF | 不透明重定向；无逐跳目的地校验 |
| `apeireth-sandbox-net` / companion `sandbox_net.rs` | PARTIAL | 回环/域名/端口 allowlist 概念，非 OS 进程隔离 |

**采纳**：URL/端口/scheme 校验思路、IP 分类方法。**拒绝**：不透明重定向跟随、只查主机名字符串、通配后缀 allowlist、无界响应读取、环境代理继承。

**受控出站实现（冻结契约）**：位置 `crates/apeireth-tools-canonical/src/egress.rs`。流水线 —— 解析 URL → 校验 scheme（http/https）并拒绝 userinfo → 按 `EgressPolicy` 校验 host + port → 解析 host 到 `SocketAddrs` → **对每个解析出的地址**按策略校验 → 构建带 `no_proxy` + `resolve_to_addrs` 钉扎的 `reqwest` client → 以重定向策略 `none` 发 GET → 遇 3xx：解析 `Location` 并**对新 URL 重跑完整策略** → 限制响应大小。**策略**：`DenyAll` / `PublicInternetOnly` / `ExplicitAllowList(EgressAllowList)` / `Unrestricted`（显式退出）。**地址分类**：IPv4 回环/私网/链路本地/组播/未指定/广播用 Rust 标准库分类；IPv6 回环、ULA（`fc00::/7`）、链路本地、组播、未指定均分类；**IPv4-mapped IPv6 按其内嵌 IPv4 地址分类**——故 `::ffff:127.0.0.1` 是回环、`::ffff:10.0.0.1` 是私网。**DNS 语义**：IP 字面量绕过 DNS 并直接校验；主机名**只解析一次**然后校验解析出的地址；`PublicInternetOnly` 下**任一**解析地址非公网即拒（混合公私 DNS 结果 → 拒）；**DNS 失败是 `ResolutionFailed`，绝不是隐式放行**。**DNS 钉扎**：主机名目的地按请求构建 `reqwest::Client`，用 `resolve_to_addrs(host, validated_addrs)` + `no_proxy()`；**URL 主机名不变**，故 Host 头与 TLS SNI/证书校验仍指向原主机名，而连接被钉到已校验地址。**重定向**：`reqwest` 重定向策略 `none`；**每个 3xx 跳**都被解析、scheme/端口校验、解析、地址校验；相对与绝对 `Location` 通过 `url::Url::join` 支持；畸形 `Location` → `RedirectDenied`；**HTTPS → HTTP 降级被拒**；最多 10 跳（超限 `TooManyRedirects`）。**transport 上界**：timeout 默认 30s（`connect_timeout` + `read_timeout` + 总超时）；响应体默认 1 MiB（超限 `ResponseTooLarge`）；**无请求体支持（仅 GET）**；环境 `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY` 经 `no_proxy()` 禁用——**基于代理的受限 transport 明确不支持**。**错误集**：`InvalidUrl`、`UnsupportedScheme`、`ResolutionFailed`、`DestinationDenied`、`RedirectDenied`、`ConnectionFailed`、`TlsFailed`、`ResponseTooLarge`、`TooManyRedirects`、`Timeout`。

**测试**：纯策略测试覆盖公网/私网/回环/链路本地/IPv6/mapped IPv6、混合 DNS 结果、allowlist 精确性、畸形 URL、不支持 scheme、userinfo 拒绝；transport 测试用本地回环监听器 + 注入的假 resolver 证明——被钉扎的 resolver 就是被联系到的地址（主机名在真 DNS 中不解析）、重定向到被拒目的地在联系前就失败、设置了环境代理变量时本地 transport 仍可用。

| Platform | Mechanism audited | Conclusion | Reason |
|---|---|---|---|
| Windows | AppContainer | DEFERRED / UNSUPPORTED | 在保留 CREATE_SUSPENDED + JobObject 集成的前提下，为任意 Win32 子进程做无网络能力的 AppContainer 启动，未在普通用户 runner 上被证明 |
| Windows | WFP / firewall rules | DEFERRED | 需管理员/服务/全局副作用；不适合作为默认非特权后端 |
| Linux | user + network namespace（`CLONE_NEWUSER` + `CLONE_NEWNET`） | DEFERRED / UNSUPPORTED | 需运行时探测与正确的 uid_map/gid_map/setgroups 处理；未在本 runner 验证；还需真实子进程 `connect` 失败测试 |
| Linux | seccomp socket/connect deny | DEFERRED | 半份 syscall 表不是隔离；工作区内无成熟策略 |
| macOS | 公开/稳定的子进程沙箱 | UNSUPPORTED | 未找到公开、稳定、面向任意子进程的网络沙箱 primitive；私有 API 按规则排除 |
| macOS | pf / Network Extension | DEFERRED / UNSUPPORTED | root/全局或 entitlement/打包要求不适合作 canonical 默认后端 |

**原文边界**：面向任意子进程的进程级 allowlist（按主机名/域出站）**有意不在 M2D 范围**；至多先试 deny-all。

| Capability | Windows | Linux | macOS |
|---|---|---|---|
| 受控应用出站 | ENFORCED（应用自有） | ENFORCED | ENFORCED |
| 进程 `NetworkIsolation` | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |
| 文件系统隔离 | UNSUPPORTED | UNSUPPORTED | UNSUPPORTED |

**CI 证据**：workflow `M2B-XV three-OS process isolation validation`（加了一步显式受控出站测试）；run `https://github.com/Apeireth/Apeireth/actions/runs/32875581292`；commit `fdd339a9fc2c018274b7db00dbf68b1868235c84`。结果：Windows success（44 lib + 4 architecture + 21 process-executor + 19 egress，0 failed）；Ubuntu success（44 lib + 4 architecture + 18 + 19，0 failed）；macOS success（44 lib + 4 architecture + 17 + 19，0 failed）。各 job 跑 `cargo check -p apeireth-tools-canonical --locked`、`cargo test -p apeireth-tools-canonical --locked`、`cargo test -p apeireth-tools-canonical --locked egress::tests -- --nocapture`、`cargo test -p apeireth-tools-canonical --test process_executor --locked -- --nocapture`。

**残余缺口（原文）**：受控出站仅支持 GET；POST/HEAD 与请求体是未来项；显式代理支持推迟（受限 transport 禁用环境代理）；**本阶段任何平台都没有进程级网络隔离**；`PublicInternetOnly` 把文档用 IPv6 段当公网——若需更强分类，属未来策略修订。

---

## §10 M3A —— Canonical 受控 Fetch 能力

**原件**：[`../../01-architecture/m3a-canonical-fetch.md`](../../01-architecture/m3a-canonical-fetch.md)

**阶段目标**：`tool.fetch` 是**刻意收窄**的 GET-only HTTP(S) 文本抓取能力——**不是**浏览器、API client、爬虫或文件下载器。本阶段同时是**canonical 架构验收测试**：master 已有 Fetch 功能，M3A 证明它能进入冻结的 Plugin / Governance / Tool / Egress / Runtime 边界，而**不引入 master 架构**。状态 COMPLETE；分支 `reconstruct_v2`；基线 SHA `7f515aab37d1a9e58f1eedb0a92691cfe496f4d6`。

| Baseline component | Actual maturity | Useful behavior | Security gap | Strategy |
|---|---|---|---|---|
| `reconstruction_v2/crates/apeireth-tools/src/builtin/fetch.rs` | BROKEN | schema 思路（`url` + 可选 `method`）、状态即结果的思路、medium risk 标签 | 先校验 DNS 再调裸 `reqwest::Client`；校验后**二次 DNS 查找**；不透明重定向；无 DNS 钉扎；无 `no_proxy()`；支持 POST；无 body 上界（直到 `text()` 全文读完）；`text()` 对非法 UTF-8 返回空串而非错误 | DROP 实现；ADAPT 窄 schema 思路与状态即结果语义 |
| `reconstruction_v2/crates/apeireth-tool-fetch/src/http_fetch.rs` | BROKEN for SSRF | 错误 UX 思路（`TooLarge`、`Http`）、content-type 捕获、`final_url` 元数据思路 | 用 `apeireth-http-client` 且不透明跟随重定向、无逐跳目的地重校验；POST/body 支持；HTML 提取与重试/缓存/指标不在 M3A 范围 | DROP 实现；ADAPT `TooLarge` 式有界失败措辞 |
| `reconstruction_v2/crates/apeireth-tool-fetch/src/config.rs` | PARTIAL | timeout/UA/body-limit 配置形状 | 重试、缓存、重定向开关；无 egress 策略 | DROP；canonical `FetchConfig` 持有 `Arc<ControlledEgress>` 与固定可选 User-Agent |
| `crates/apeireth-tool-fetch`（legacy 当前分支副本） | BROKEN for SSRF | 无超出上述基线者 | 同基线；重复 HTTP 栈与 registry | DROP |

**原文确认的基线核心 SSRF 缺口**：目的地校验由工具做，然后**另一个不透明的 HTTP client** 做一次全新的 DNS 查找并跟随未经重校验的重定向。**canonical Fetch 不得这么做。**

**canonical 架构路径（冻结契约）**：`Provider → ToolCall → Runtime → Governance → PluginRegistry / ToolCapability → FetchTool → EgressPolicy → EgressTransport → DNS / HTTP(S)`。**Runtime 对 fetch 是无感的：NO** —— Runtime 只存不透明的 `FrozenInvocation` 载荷，**从不**看 Fetch 的 URL、策略或 transport 细节。

| Item | Value |
|---|---|
| Capability ID | `tool.fetch` |
| Tool name | `fetch` |
| Default enabled | DISABLED (`BuiltinToolsOptions.fetch = None`) |
| Risk | `medium` |
| Method | `GET` only |
| URL max | 8192 bytes |
| Timeout | transport 默认 30 s；**只能**通过受信任 `ControlledEgress` 配置 |
| Response max | transport 默认 1 MiB；**只能**通过受信任 `ControlledEgress` 配置 |
| Content types | `text/*`、`application/json`、`application/xml`、`application/xhtml+xml`、`application/javascript`、`application/*+json`、`application/*+xml`；缺失 Content-Type 时**仅当**为合法 UTF-8 且无 NUL 才按文本接受 |
| Binary | Rejected. No base64, no save-to-disk |
| Cookies | None. No jar, no persistence, no `Set-Cookie` in results |
| Authentication | None. No Authorization, no bearer token, no Basic Auth |
| Headers | 固定 `accept: */*` + 可选受信任 `user-agent`；**模型不得提供 header** |
| Proxy | 环境代理被 `ControlledEgress::no_proxy()` 禁用 |
| Redirects | transport 所有；**每一跳完整重校验**；HTTPS → HTTP 降级拒绝；默认最多 10 跳 |
| Retry | No hidden retry |

**配置（冻结契约）**：`FetchConfig { egress: Arc<ControlledEgress>, user_agent: Option<String> }`；helper `public_internet_only()` / `explicit_allow_list(list)` / `deny_all()` / `unrestricted()`（有文档的 opt-out）/ `with_user_agent(...)`；启用时若未显式给 transport，默认策略**优先 `PublicInternetOnly`**；`BuiltinToolsOptions.fetch` 默认 `None`，故生产 bootstrap **不会静默获得网络能力**。**模型可控的安全配置？NO —— 模型只提供一个 URL。**

**请求 schema（冻结契约）**：只有 `url`（string，minLength 1，maxLength 8192），`required: ["url"]`，`additionalProperties: false`；**未知参数被工具拒绝**。

**响应合同（冻结契约）**：成功值 `{ url, status, content_type, body, bytes, redirects }`；`url` 是最终生效 URL（去 fragment）；**4xx/5xx 是成功的工具执行**，携带事实性 `status`；204/304/空 body 是空文本而非错误；`body` 是原始 UTF-8 源内容，**无 Markdown 围栏、无 HTML 剥离**。

| Concern | Behavior |
|---|---|
| Scheme validation | `http` / `https` only；`file:`、`data:` 等拒绝 |
| Private IP | `PublicInternetOnly` 下拒绝；显式 allow-list 控制私网目标 |
| IPv6 | 回环、ULA、链路本地、组播、未指定分类 |
| DNS | 每跳解析一次；混合公私结果 fail-closed |
| DNS pinning | `resolve_to_addrs` 固定每跳已校验地址 |
| Redirects | 逐跳手动重校验；无不透明跟随 |
| HTTPS downgrade | Denied |
| TLS | 证书与主机名校验始终开启 |
| Proxy | 环境代理禁用 |
| Body limit | transport 流式设界（在 Fetch 解码之前） |
| Timeout | Connect + read + total |

**Fetch 自身不做 IP 分类，也不做用于策略的 DNS 查找。**

**凭据（冻结契约）**：Authorization **NONE**；Cookie **NONE**；Proxy-Authorization **NONE**；**环境凭据不发送**；**URL userinfo 在 transport 前被拒，绝不记录**。

**内容处理（冻结契约）**：`text/plain` 按 UTF-8 文本返回；`text/html` **原样返回，不做 HTML 提取**；`application/json` / `application/xml` 原样文本返回，**无语义改写**；缺失 Content-Type → 合法 UTF-8 且无 NUL 则接受为文本，否则拒绝；非 UTF-8 → 只接受 UTF-8/ASCII 字符集，其他声明字符集被拒并报 `unsupported charset=`；二进制（`image/*`、`audio/*`、`video/*`、`application/octet-stream`、zip、pdf）被拒为 `unsupported media type`；**PDF 被拒，无解析、无 OCR**。

**HTTP 状态语义（冻结契约）**：2xx = 带 body 的事实性成功；3xx = transport 跟随并重校验，返回最终状态；4xx = 带 status 与 body 的事实性抓取成功；5xx 同；**429 = 带 status 与 body 的事实性结果，不自动 sleep/重试**。

**冻结调用（冻结契约）**：**已实现** —— 理由：`tool.fetch` 可被普通治理配置置于 `RequireApproval` 之下，**被批准的 fetch 不得在审批挂起期间因受信任配置变化而悄悄放宽**。冻结字段：`version`、`method`、`url`、`timeout_ms`、`max_response_bytes`、`max_redirects`、`egress_policy`、`user_agent`。**配置漂移行为**：执行时**从冻结的 `egress_policy` 与冻结上界重建 `ControlledEgress`**，**不使用当前配置**。**DNS 行为**：DNS **有意不冻结** —— 实际连接在执行时由受控 transport 解析并重校验；安全属性是「解析出的地址必须满足被批准的政策」，**不是**「与过期预览一致」。**先认领后生效**：不变 —— 通用 M2C 审批生命周期在派发前持久化认领，故 HTTP 接触只发生在批准之后。

**信任标记（原文）**：抓取到的远端内容是**不可信的外部内容**。目的地策略控制 *Apeireth 连到哪里*，**不控制远端服务器说什么**。prompt-injection 治理仍是**独立的上下文/输入安全层**。

**不在范围 / 推迟（原文）**：浏览器、DOM、JavaScript、cookie、POST/body、带认证的 HTTP、二进制下载、PDF 解析、HTML 提取、Markdown 转换、爬虫、robots、持久 HTTP 会话、MCP HTTP、文件落盘、内容溯源。

**安全措辞（原文，冻结）**：本能力提供的是**策略强制的目的地校验 + DNS 钉扎的受控 transport + 重定向目标重校验**；**它不被宣传为「SSRF-proof」**。

**源码守卫（冻结契约）**：`tests/architecture_invariants.rs` 验证 Fetch 生产源码不含 `reqwest::`、`tokio::net::lookup_host`、`Command::new("curl")`、`Command::new("wget")`、`ProcessExecutor` 或手搓 SSRF 助手；runtime canonical 不变量验证 runtime 源码**不提及** `tool.fetch`、`FetchTool`、`EgressTransport`、`EgressPolicy`。

---

## §11 附录 A —— 非 M-序列同期真源（**现行依据，保留原位**）

> **本节两篇不是 reconstruct_v2 的 M-序列阶段**，而是 2026-08-28（v2.0.0-rc.1 收盘期）与 M 序列**并行**的 A 块（OrganOrchestrator）真源。它们**现行/半现行**，原件**保留原位**不被本册取代；本节只做指针 + 当时状态摘要，供按阶段顺序读历史时不被「章节顺序」误导。

### §11.1 A 块 OrganOrchestrator 串联规范（现行依据，保留原位）

**原件**：[`../../01-architecture/organ-orchestrator-spec.md`](../../01-architecture/organ-orchestrator-spec.md)（子代理 R11 于 `b9026186`（v2.0.0-rc.1 tag 拍板）后写；文档头 HEAD `7d990297`（Round 6 完））

**当时状态**：🟡 **spec 写完，真实施待 v2.0.0 release 后启动**（估 1–3 周，主代理后续派子代理）。定位：v2.0.0-rc.1 真生产前阻塞项 **#6**。

**核心事实（当时）**：v1 `AwakeCompanion`（`legacy/donor/apeireth-companion/src/organs.rs:34`）是真实现 —— 8 重门控（`emergence.rs:460-503`）+ 5 状态机（`apeireth-evolution::EvolutionState`，Idle/Draft/Proposed/Ratified/Active/Retired）+ 9 organ 串联（E4/F1/F4/F6/W1/W2/W3/E7/Council）+ emotion/council/onion 三层合成。**v2 缺的是串联它们的上层 orchestrator** —— 9 organ 已全部真实现（`crates/engine/organ/src/{curiosity,emotion_memory,hypothesis,value_cases,world_model,causal_world_model,causal_world_model_edges,emergence,memory}.rs`），但 runtime 拿 `Arc<dyn OrganTrait>`，**没有 9 organ 之间的 process 串联路径 + 8 重 gate 统一入口 + 5 状态机 transition driver**。

**规范骨架**：L0 人类审批（硬墙，永远不可变）→ L1 自我诊断（`cognitive.self_assessment` via RC-4）→ L2 提案生成（Orchestrator + 7 LlmAdvisor via RC-6）→ L3 验证（9 organ process 串联 + sandbox regression）→ L4 主人审批（governance 3 hook + 7 advisor 加权 + 主人 Veto）→ L5 runtime patch（`git tag v2.x+1`）。

**当时 0 装诚实盘点（原文）**：v1 AwakeCompanion ✅ done；v1 8 重 gate ✅ done；v1 5 状态机 ✅ done（6 状态含 Retired）；v2 9 organ ✅ 全 done；v2 12 slot ledger ✅ done（6 WIRED + 1 SLOT READY + 6 DEFERRED）；**v2 OrganOrchestrator 类似 AwakeCompanion ❌ 缺**（9 organ 只是 9 个独立 trait impl）。**0 装诱导 prevention（原文）**：不假装「全做完」；不假装「v2 E7 emergence 自带 5 状态机」（`PolicyStage` 是前向声明，`policy_stage()` 永远返 `Active` 占位）；不假装「9 organ 已串联」；不假装「5 重守门跑通 OrganOrchestrator」（跑的是 cognitive module ledger + 9 organ 独立测试）。

**当时真生产前阻塞**：#6 OrganOrchestrator（1–3 周）；#11 frontend 对接（4–6 周，估 2027-Q1 启动）；6 DEFERRED slot 激活（各 2–3 周）。

> **口径提示**：spec 正文里「键集」的键数表述为 **13 键**，`workspace.version` 写作 `2.0.0-rc.1`——这两项是**历史口径**，现行口径见 §12。

### §11.2 A 块 OrganOrchestrator 完整化方案（现行依据，保留原位）

**原件**：[`../../01-architecture/organ-orchestrator-completion-plan.md`](../../01-architecture/organ-orchestrator-completion-plan.md)（主代理 Mavis 拍板，2026-08-28；HEAD `2a99332c`）

**当时现状真账（原文）**：R12 `orchestrator.rs` 1525 行（9 organ handle + 13 InitiativeGate + 5 PolicyStage forward-declared + 6 步 tick + L0-L5 enum）；3 integration tests，1726 passed / 0 failed / 12 ignored；clippy 0 警告 / 0 错误；5 重守门 0 触碰 LOCKED 5 项。

**5 缺口**：A `check_8_gates()` 缺后 3 重（RhythmUnknown/RhythmVeto/DriveLow）；B tick 步骤 3 情绪调制（EmotionLow gate）；C tick 步骤 4 Council `decide_with_invoker`；D `ratify_fresh_policy()` 完整 5 状态链；E L0-L5 `UpgradeCycle` driver。

**5 stage 计划（1 stage = 1 commit）**：Stage 1 = 缺口 D（1h，风险 0，纯本地状态机）；Stage 2 = 缺口 B（1–2h，低）；Stage 3 = 缺口 A（3–4h，中，改 trait enum + 9 organ + orchestrator）；Stage 4 = 缺口 C（4–6h，中高，改 `Orchestrator.new()` 是 breaking change）；Stage 5 = 缺口 E（6–8h，中，跨 crate）。总估 16–21h（~3–4 工作日），短于 A 块 1–3 周估时的下限。

| Stage | 缺口 | Commit | Tests | Clippy | 备注 |
|---|---|---|---|---|---|
| 1 | D | `fc159288` | 1726 passed / 0 failed | 0 警告 | ✅ done (pushed)；+ `RatificationChain` struct + `ratify_fresh_policy()` 4 transition 走链 |
| 2 | B | `ea9aa14f` | 1727 passed / 0 failed (+1 new) | 0 警告 | ✅ done (pushed)；+ `extract_emotion_mood()` + tick 步骤 3 真路径 |
| 3 | A | `ed6353f4` | 1728 passed / 0 failed (+1 new) | 0 警告 | ✅ done (pushed)；+ InitiativeGate 移 plugin + `OrganOutput::Emergence.gate` + `extract_e7_gate()` |
| 4 | C | `1972b040` | 1729 passed / 0 failed (+1 new) | 0 警告 | ✅ done (pushed)；+ CouncilInvoker 参数 + MockCouncilInvoker + `decide_with_invoker` |
| 5 | E | `edc98170`（amended from `24d163ff`） | 1739 passed / 0 failed (+10 new) | 0 警告 | ✅ done (pushed)；+ UpgradeCycle + TagSuggester + 6 步骤 `run_full_cycle` |

**O-6 复盘 amend 真账（原文）**：5 个 commit 的 O-6 三阶审查 sections 多是描述 **WHAT** 而非 **WHY 最优 vs alternatives**，属 O-6 失守 + O-5 0 装诚实标（详见 `docs/04-internal/A-block-o6-true-account.md`）；amend 后 sections 改为真答「为什么最优 vs alternatives + 拒理由」。

**Stage 1 实战教训（原文，主代理亲验）**：① `cargo fmt -- file1 file2` **不只**格式化指定文件 —— 实测会格式化整个 workspace 21 个文件；立即 `git checkout HEAD -- crates/...` 回滚非我改动；下次 commit 前改用 `rustfmt file.rs` 单文件格式化。② **Rust 不支持嵌套 impl block** —— 第一次编辑把 `impl RatificationChain` 放进 `impl<RS> OrganOrchestrator<RS>` 内部，编译报 `implementation is not supported in 'trait's or 'impl's`；修法是移到 module-level。③ **R12 内部 0 装诚实标准确** —— `orchestrator.rs:915-918` 旧实现 `policy_stage = Active` 单步跳 + 注释「本地 driver 简化」确实是 0 装诚实标缺口；新实现与 v1 `AwakeCompanion::ratify_fresh_policy` 4 transition 调用 1:1 对齐。

**后续 commit 的 O-6 三阶审查标准（原文）**：① **总体最优** —— 在更大语境（release 路线图 / 当前工作量约束 / 上下游依赖）里是不是最优切入点？**与 alternatives 比较 + 选最优 + 拒理由**；② **系统最优** —— 在 Apeireth 子系统依赖图（governance → orchestration → memory → runtime → organ）里改动放在哪一层最合适？**与 alternatives 比较 + 选最优 + 拒理由**；③ **架构最优** —— 在 workspace 拓扑 + 单向依赖 + trait object 设计下，公开 API 形状 + crate 边界 + 0 引新外部 dep，方案是否最优？**拒的 alternatives + 拒理由**。**不复用 v1 alignment 代替 v2 总体最优；不描述 WHAT 代替 WHY；每段需有具体拒的 alternative + 拒理由。**

---

## §12 现状对照（2026-10-23）

> 本节**只指向现行真源**，不在历史叙述里改写数字。历史章节里的 crate 数、测试数、HEAD、键集数、workspace.version 一律是**当时口径**。

| 主题 | 现行真源（唯一出处） | 说明 |
|---|---|---|
| 测试数字 | [`../../03-reference/test-baseline.md`](../../03-reference/test-baseline.md) | 全仓测试数字的**单一真源**；历史卷宗中的任何 passed/failed 数都是历史口径 |
| 能力状态声明 | [`../../status/claims.yaml`](../../status/claims.yaml) | 口径阶梯 NOT IMPLEMENTED < IMPLEMENTED < PRODUCTION WIRED < HARDWARE VALIDATED；`DEFAULT ENABLED` 是横切属性（`default: ON/OFF`），**不是**一级 |
| 默认开关实况 | [`../../03-reference/capabilities-matrix.md`](../../03-reference/capabilities-matrix.md) §0.2 | 与 `crates/engine/runtime-assembly/src/canonical/production.rs` 实况对应 |
| 现行待办 | [`../backend-enhancement/00-backlog.md`](../backend-enhancement/00-backlog.md) | 现行想法级索引；本册 §13 的历史残余清单**不**等于现行 backlog |
| 工程总纲 / 施工纪律 | [`../ENGINEER-MANIFESTO.md`](../ENGINEER-MANIFESTO.md)、[`../backend-enhancement/CONSTRUCTION-DISCIPLINE.md`](../backend-enhancement/CONSTRUCTION-DISCIPLINE.md) | 改 src / 改 doc / 派子代理前必读 |
| 仓库级路线 | [`../../../ROADMAP.md`](../../../ROADMAP.md) | §2「reconstruct_v2 已完成清单」是本册 §3–§10 的**现行索引**，证据列指向 `docs/01-architecture/m*.md` 原件 |
| 键集口径 | [`../../adr/0006-formalize-14-keys.md`](../../adr/0006-formalize-14-keys.md) | 本册 §5、§11.2 引用的「13 键」为 2026-10-11 PHL-08 之前的历史口径；现行为 **14 键（PHL-01..08）** |
| 版本 | 根 [`Cargo.toml`](../../../Cargo.toml) `[workspace.package] version` | 本册 §11.2 出现的 `2.0.0-rc.1` 为历史口径 |

**当前口径下的 M-序列判断（依据上表真源，不改写历史）**：M1A/M1B/M1C/M2A/M2B/M2C/M2D/M3A 在 `ROADMAP.md` §2「reconstruct_v2 已完成清单」中仍**列为已完成**，证据列指向本册覆盖的原件；本册不复述其成果描述。

---

## §13 当时的未决项 / 残余清单汇总（owner 决策位）

> 下列条目是**当时**的挂账，不是现行 backlog。保留以便追溯「哪一步留了尾巴、尾巴后来由谁接」。

- **M1C 推迟**：持久化审计链后端；每个治理事件上的审计链接线；完成内容（provider transcript）输入安全集成；`SovereignControl` 与角色/暂停-恢复策略；gateway auth / tool 沙箱策略 / 网络出站策略 / MCP 策略 / 凭据后端集成。
- **M2A 推迟**：filesystem write/delete/rename/copy（等 M2B 沙箱强制）；search 正则搜索 / ignore 文件 / 结构化二进制检测；repo 任何 mutation、任意 revision/path 选择、timeout 与进程封装。
- **M2B / M2B-X 推迟**：Restricted-token 启动（两平台 DEFERRED）；非 Windows 后代遏制（PARTIAL）；Linux cgroup v2 / Landlock / namespace / seccomp；macOS 无受支持的子进程沙箱机制；Windows 文件系统/网络隔离；内存/进程数上限在非 Windows 的强制。
- **M2C-R 推迟**：RestrictedShell / UntrustedShell 形态；M2C v1 无「always allow shell」持久授权、无通配授权；TOCTOU 残余（cwd symlink 重定向）明确不声称可检测。
- **M2D 推迟**：受控出站 POST/HEAD 与请求体；显式代理支持；进程级网络隔离（三平台均无）；`PublicInternetOnly` 对文档用 IPv6 段的分类收紧。
- **M3A 推迟**：浏览器 / DOM / JS / cookie / POST / 带认证 HTTP / 二进制下载 / PDF 解析 / HTML 提取 / Markdown 转换 / 爬虫 / robots / 持久 HTTP 会话 / MCP HTTP / 文件落盘 / 内容溯源。
- **§1.1 的 5 个 Open Questions**（storage crate 路径、memory retrieval 归属、PII/audit 的 crate 归属、schema 规范化、MCP 动态注册）——见 §1.1 原文。
- **§1.1 Deferred Functionality 清单**（原样保留）：Dream engine and world model；Curiosity engine；Voice TTS and full voice stack；Avatar Live2D/VRM runtime；Bridge (social/lark/livekit/stock)；Desktop/vision full stack；Worktree factory and ToolSynthesizer；Invest and Learning tools；PyBridge；SDK polish。
- **§1.2 的 Known gap**：MCP 是唯一需要「plugin 激活之后才出现的能力」的迁移；registry 故意拒绝迟注册重复项，动态能力需要显式受检 API。
- **§11.2 当时挂着**：A 块 5 stage 已全部 done + amend（§11.2 表）；后续 commit 的 O-6 三阶审查标准（§11.2 末）为 owner 决策位。

---

_本册为**只读归拢**产物：只新建本文件，未修改任何被合并的原件，未触碰 `docs/architecture/*`、`docs/status/*`、`docs/03-reference/*`、`docs/adr/*`、`docs/archive/**`，未执行任何 git 写操作。_
