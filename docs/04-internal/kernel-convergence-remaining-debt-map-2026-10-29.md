# Production Kernel Convergence — Remaining Debt Map

```text
[Document-Meta]
Document: docs/04-internal/kernel-convergence-remaining-debt-map-2026-10-29.md
Version: 1.0 (M0 audit)
Last-Modified: 2026-10-29
Status: 审计完成；Production Convergence 仍为 Partial；未开始实施
Author: Codex
```

## 审计边界与结论

本次基线为 `refactor/kc1-kernel-convergence@b003bfffba82d2f81f89e363f9cc60141ae1a6c8`，
开始时工作区干净。授权仅为 M0：只新增本文，不修改生产代码、测试、依赖、状态真源或历史验收账，不提交、不切分支。
本文依据本地源码与测试做静态审计；测试文件存在不等于本批测试通过。运行限制与命令见末节。
取材只用仓库本地有界源码窗口及符号检索，无外部代码或下载素材。

前置依据为 [ADR-0018](../adr/0018-registry-kernel-tool-events.md)、
[ADR-0019](../adr/0019-registry-runtime-dispatch.md)、
[ADR-0020](../adr/0020-capability-module-ownership.md)、
[ADR-0021](../adr/0021-owned-agents-and-dynamic-modules.md)、
[ADR-0022](../adr/0022-production-kernel-convergence.md) 与
[上批验收账](kernel-convergence-final-acceptance-2026-10-28.md)。
历史全量数字仍归 [test-baseline](../03-reference/test-baseline.md)，本文不复制为本批实测。

**主要余债是执行与授权接线：独立 SubLoop 模型循环仍在；两种 ModuleInvoker 实现仍直接调用 Provider；
桌面 Host 仍持裸 Runtime；旧全局认知没有授权 Profile 映射；原始 with_module 仍可直接执行回调并投影 Overlay。**
已经交付的注册工具、模块资源工厂、MCP 生命周期、多实例运行时和 PortableHost 必须复用。

状态词沿用 [能力状态定义](../01-architecture/system-capabilities.md)：IMPLEMENTED 表示代码存在，
PRODUCTION WIRED 表示接入对应生产路径，DEFAULT ENABLED 是独立的默认开关属性；
本批没有 HARDWARE VALIDATED 或 live E2E 证据。

## 已完成的基础与尚未完成的接线

| 能力 | 源码证据 / 当前状态 | 剩余范围 |
|---|---|---|
| 注册、工具分派与固定 Hook | `crates/engine/runtime/src/canonical/runtime.rs`、`dispatcher.rs`、`registered_hooks.rs`；`PluginManager` 与唯一可执行目录已有接线（PRODUCTION WIRED） | 不重造 Registry、Dispatcher 或 Hook 系统；兼容入口继续归并 |
| 生产认知工具事件 | `crates/engine/runtime-assembly/src/canonical/production.rs:1810` 的 `ProductionModules::register_into` 使用 `with_registered_behavior`；CLI 的 `register_with_policy` 显式授予所选内部工具能力（PRODUCTION WIRED） | 不是“生产仍全部 with_module”；旧 ABI 本体与侧模型调用尚需迁移，私有 Profile 未映射 |
| 认知防重复注入 | `crates/engine/runtime/src/canonical/behavior_adapter.rs` 的 `BehaviorTool::invoke_with_context` 将 Overlay 转成不可信结果并清空原 Overlay；`business_messages` 排除已认证模块工具对（PRODUCTION WIRED） | 继续覆盖原始 with_module 入口及与原生 MemoryModule 组合的场景 |
| 资源与热生命周期 | `crates/foundation/plugin/src/resource_factory.rs` 的 `ModuleDefinition` / `module_from_definition`；`crates/engine/runtime/src/canonical/instances/hot.rs`、`control.rs`（PRODUCTION WIRED 于受管 Host） | 独立资源、撤销令牌、旧世代审批、排水失败重试均保留；无需再建工厂 |
| 生产 MCP | `crates/capabilities/tools/src/mcp_bridge/definition.rs` 的 `McpDefinition` 经生产装配进入同一资源工厂（PRODUCTION WIRED；按 Host 配置选择） | 不重做连接生命周期；不能把显式 close 的保证泛化为 Drop 已完成异步排水 |
| 独立实例与用量 | `crates/engine/runtime/src/canonical/instances.rs` 的 `build_agent_runtime` / `ManagedAgent::run` → `Runtime::execute_outcome`；`accounting.rs` / `provider.rs`（IMPLEMENTED，受管 Host 可用） | 不等于桌面默认主实例或旧侧调用已获独立实例身份 |
| 原生私有记忆与 Profile | `crates/engine/runtime-assembly/src/canonical/memory_module.rs`、`memory_module/execution.rs`；`crates/engine/runtime/src/canonical/profiles.rs`、`instances/profiles.rs`（IMPLEMENTED，可选组合接线） | 旧 Recall / Writeback / Persona / Preference / Organ 数据授权映射未完成 |
| 可裁剪 Host | `crates/engine/runtime-assembly/src/portable.rs` 的 `PortableHost::assemble` 使用受管主实例；`Cargo.toml` 的 desktop / memory / profile-store 边界（IMPLEMENTED） | 桌面仍用原装配；本批未复跑 Android 交叉构建，上批交叉构建不代表设备验证 |

## D1：Legacy SubLoop → AgentInstance

### 精确入口与现状

| 文件与符号（基线行号） | 当前行为 / 余债 |
|---|---|
| `crates/engine/runtime/src/canonical/subloop.rs:178` `RuntimeSubLoopSpawner` | 持 router、governance、父 session/trace、工具快照、共享 `ModuleTurnState`；不是独立受管实例 |
| 同文件 `:219` `spawn_bounded` | 自建临时 transcript；`while round < spec.max_rounds` 内预留侧调用预算、检查 completion 治理并调用 `ProviderRouter::complete_for_session`；自行累加 usage、执行工具、判定终止 |
| 同文件 `:342` `dispatch_subloop_tool` | 已校验 Dispatcher schema/scope、附加 allowlist、歧义工具名与治理；构造父身份上下文后调用工具 Adapter。**已复用分派边界，不等于已复用 Runtime 回合算法**；取消字段仍为 `Default::default()` |
| 同文件 `:474` `SubLoopSpawner::spawn` | 外层 `tokio::time::timeout` 包住整个子循环；不能将 future 返回当成原生资源已排水 |
| `crates/engine/runtime/src/canonical/execute.rs:2430` `Runtime::run_hook`，`:2454` 构造器 | 每次模块 Hook 创建 turn-scoped spawner；借用 invoker、owned invoker_handle、subloop 共用该 Arc 和预算 |
| `crates/engine/runtime/src/canonical/behavior_adapter.rs` `BehaviorInput` / `BehaviorTool::invoke_with_context` | 注册认知工具仍携带并使用上述 spawner，因此即使外层是 V1 工具，内部侧模型执行仍是 Legacy |

### 目标与语义守恒

旧 `SubLoopSpawner`、`SubLoopSpec`、`SubLoopResult`、错误类型可保留为适配 ABI；实际执行必须创建或使用
现有实例服务中的独立 AgentInstance，经同一 `Runtime::execute_outcome` 执行，删除 `spawn_bounded` 自有模型循环。
不能只把循环移动文件、包进 Agent 名称或在回调中递归调用父 Session。

- 子实例有独立 AgentId、SessionId、RequestId、历史、状态与错误；owner 绑定真实模块世代，父身份只作来源关联。
  子 transcript 不写入父用户会话；前端只看 owner 返回的结构化结果，不能直播内部子调用。
- `max_rounds` 包括零轮拒绝和非收敛错误；保留 model 继承/覆盖、可选 system 与原消息顺序、工具结果顺序和饱和 usage 聚合。
  缓存计量任一轮未知仍为未知；失败/取消前的真实请求事实不能丢失。
- 所有模块/handle 共用原 turn 侧调用预算；逐 Provider 请求消耗预算，不能改成每建一个 Agent 只扣一次。
  再叠加已有实例、module-owner、Host 预算，不能因迁移重置或重复扣账。
- 工具范围取 allowlist、父有效 scope、Host module grant 的交集；注册、父子关系和工具名均不授予权限。
  completion / capability 治理、意图与授权上下文、schema 和冻结规则继续生效。
- 子调用遇到 RequireApproval 继续拒绝，不创建隐藏审批或遗留待批 continuation；父主回合可恢复审批不受影响。
- 整体 deadline、owner stop/unload、父取消、timeout 和异常均关闭子执行并有界排水；持旧 handle 不得复活旧世代，不能关闭共享 Provider。
- 用显式实例执行策略阻止递归认知回调；不要通过工具名分支或在 Kernel 中识别 Memory/Organ 来实现。

**必须先补齐的接口接缝**：`crates/foundation/plugin/src/agents.rs` 的 `AgentConfig` 目前没有初始消息、system、
逐任务 round limit；`AgentInstance::run(String)` / `AgentReply` 只有输入文本及 text/request/实例累计 usage。
`crates/engine/runtime/src/canonical/instances.rs:496` 的 `ManagedAgent::run` 还直接将文本构造成 TurnRequest。
不能用拼接字符串代替规范化 transcript，不能用累计 usage 冒充本次调用量。
应在既有实例执行入口补通用受限请求/结果适配，继续进入同一 Kernel；不新增实例运行时或领域专属 Loop。
裸 Runtime 的兼容装配也须明确绑定同一实例服务，缺端口不得回退成直连 Provider。

### 保留 / 扩展的现有测试

`crates/engine/runtime/tests/canonical_subloop.rs`：
`subloop_runs_on_private_transcript_with_strict_capability_allowlist`、
`hostile_provider_attempting_denied_capability_is_blocked_and_invoke_counter_is_zero`、
`globally_denied_capability_remains_denied_inside_subloop`、
`effectful_capability_requiring_approval_fails_cleanly_in_subloop`、
`subloop_timeout_enforces_overall_execution_deadline`、
`global_completion_deny_blocks_subloop_and_does_not_call_provider`。
保留全部安全断言，在真实 Runtime 上增加独立身份、工具事件、取消排水与预算断言。

## D2：ModuleInvoker → 统一实例执行与 Usage Accounting

| 精确位置 | 当前行为与目标 |
|---|---|
| `crates/engine/runtime/src/canonical/module.rs:633` `RuntimeModuleInvoker`，`:684` `ModuleInvoker::invoke` | `reserve` + `authorize_isolated_completion` 后直调 router；`:737` 附近使用父 session 与新 RequestId；没有独立实例。保留公开接口，移除其独立 Provider 后端 |
| `crates/engine/runtime/src/canonical/subloop.rs:490` `impl ModuleInvoker for RuntimeSubLoopSpawner` | 另一份近似实现，使用父 session/request 直接 `complete_for_session`。这是当前生产 Hook 注入的实现，不能只修前一类就宣布完成 |
| `crates/engine/runtime-assembly/src/canonical/organ_llm_bridge.rs:237` 之后 `#[cfg(test)]` | `RuntimeModuleInvoker::new` 出现在 `:360`、`:661`、`:713`，均为测试构造；未查到非测试生产构造器调用。不能将这些命中误报为生产 bootstrap |
| `crates/engine/runtime-assembly/src/canonical/organ_module.rs:256` `InvokerLlmFactory::new(ctx.invoker_handle())`；`canonical/cognitive.rs` 的 Judge / SelfAssessment / Council | 真生产侧调用从 ModuleContext 获取 invoker；桥本身仅做 ABI 映射，应保留，不另建模型客户端 |
| `crates/engine/runtime/src/canonical/provider.rs:260` `complete_for_session`、`:360` 附近计量；`accounting.rs:320` `UsageAccounting::begin` | **旧侧请求已有 Provider 边界计量**，不是“完全没账”。但落在父 session 的账户；未注册 session 使用 `Account::legacy` 的 Host legacy owner，不是独立 child/module-owner 核算 |

目标复用 D1 的实例执行接缝，将 ModuleInvoker 约束成单次、无工具、无递归 Hook 的私有任务。
保留 `ModuleInvocationRequest` 的 system/input/model，以及 `ModuleInvocationResponse` 的完整 NormalizedResponse 和 served_by。
AgentReply 当前会丢掉响应 ID、finish_reason、原始 tool_calls、逐次 NormalizedUsage 与 served_by，必须显式保留真实结果，禁止伪造元数据。
尤其旧 `side_call_tool_responses_are_never_dispatched` 要求返回 Provider 意外给出的 tool_calls，但不执行它们；
简单使用普通工具回合会改变返回语义并可能多花一次请求。

核算仍只在现有 ProviderRouter / Dispatcher 的实际边界记账；适配器不得再累加一次成本。
每条事实归 child agent/session 与真实 module instance owner，携带父 request/trace 来源；
实例、owner 与 Host 汇总可核对，请求失败/被取消、未报告 usage、未知缓存量与估算成本继续区分。
`ModuleTurnState` 的 turn 共享、handle clone 共享、跨 turn 独立和递归限制仍须保留。

现有测试保留：`crates/engine/runtime/tests/cognitive_module_abi.rs` 的
`isolated_side_calls_use_the_provider_without_tools_or_recursive_hooks`、`side_call_tool_responses_are_never_dispatched`、
`side_call_budget_is_shared_and_explicit`、`isolated_module_invoker_obeys_completion_governance`、
`budget_exhaustion_degrades_the_module_instead_of_aborting_the_turn`；
`crates/engine/runtime-assembly/tests/canonical_invoker_handle.rs` 的 clone、不同 turn、并发 session、Deny、RequireApproval、Organ bridge 用例；
`canonical_organ_module.rs` 的 fresh budget / private transcript / concurrent sessions 用例；`organ_llm_bridge_compat.rs`。
这些行为迁入统一入口后继续断言，不能通过删除或 ignored 消除兼容约束。

## D3：Production Host 与 with_module / PromptOverlay

### 生产调用清单

| 文件 / 符号 | 实况与剩余动作 |
|---|---|
| `frontend/companion-desktop/src-tauri/src/backend_supervisor.rs:1435` `BackendSupervisor::start`、`:2179` `spawn_args`、`:2189` `spawn_backend` | 桌面启动或采用 canonical 网关进程；spawn 参数是 `gateway serve --port ...`。不要把桌面进程监督器误作 AgentInstance 管理器 |
| `crates/adapters/cli/src/lib.rs:1443` `build_canonical_runtime_assembly_from_env` → `:1528` `build_canonical_runtime_with_parts` → `:1664` 附近 `builder.build()` | 返回裸 Runtime；需改为同一受管 Host 持有默认 main AgentInstance，同时保留调用方需要的兼容 Runtime 观察面 |
| 同文件 `:1613` `cognitive.register_with_policy`；`crates/engine/runtime-assembly/src/canonical/cognitive_registry.rs:6` | 生产所选认知已注册工具并明确授权，已是 PRODUCTION WIRED；不要重新退回 with_module 后再迁一次 |
| 同文件 `:2639` `execute_canonical_cli_turn`、`:2675` 附近 `resolve_canonical_cli_approval` | 直接 execute_outcome / resolve_approval，构造意图与静态 onboarding system；受管迁移必须保留 model、既有 session 和审批响应 |
| 同文件 `:3258` 附近 gateway 装配；`crates/adapters/gateway/src/canonical_entry.rs:301` `run_turn_serialized`、`:646` 起 router facade | gateway 持 `Arc<Runtime>`，使用原 session 写闸及 streaming/non-streaming Kernel 入口；需要委托受管实例，而不是只把 CLI chat 换成 managed |
| `crates/adapters/cli/src/care_heartbeat.rs:772` 附近请求提交；`canonical_approval.rs:32` 的 bootstrap | 同一桌面装配的旁路调用方需一并审查实例归属、审批和关闭，防止主入口迁完仍遗留未管理执行 |
| `crates/engine/runtime/src/canonical/runtime.rs:741` / `:748` / `:756` | `with_registered_behavior` 标记工具适配；`with_module` 仅 push modules，`with_behavior` 委托它。后两者仍保留直接回调执行语义 |
| `crates/engine/runtime/src/canonical/execute.rs:2530` 附近 fallback、`:490` `compose_provider_messages` | 未注册适配的模块直接 `module.on_hook`，Overlay 仍参与 Provider 消息组合。兼容入口应委托既有 RegisteredBehavior / 固定 Hook / ToolEventV1 |

在 `crates/**/src` 非测试调用检索中，没有发现生产装配直接调用 `.with_module(...)`；
当前债是公开 builder/fallback 仍可执行旧后端。现有 with_module 调用主要在 Runtime/assembly 集成测试。
不能把“公开旧执行面仍存在”写成“默认生产认知尚未工具化”。

生产 Overlay **产生者**仍在 `crates/engine/runtime-assembly/src/canonical/cognitive.rs`：
`PartnerBondModule::on_hook`（`:484`）、`PersonaConstantInjectionModule::on_hook`（`:567`）、
`AbsorptionInsightModule::on_hook`（`:955`）、`MemoryRecallModule::on_hook`（`:1331` / `:1423`）、
`PreferenceRecallModule::on_hook`（`:1994`）、`ReflexionModule::on_hook`（`:2101`）、
`BioAssemblyModule::on_hook`（`:3084`）；以及 `canonical/bio_tone.rs:475` 的 tone Overlay。
这些对象经 ProductionModules 注册后，其动态结果已由 BehaviorTool 转成 V1 工具结果；Overlay 类型的存在本身不是重复注入证据。

### 目标与不能直接套用的现有 facade

默认桌面 Host 使用现有 `RuntimeBuilder::build_agent_runtime` 与 Managed AgentInstance；
按会话维持用户可见历史及授权，默认主角色与模块 child 分开计量。
`PortableHost` 是同一 Kernel 的可选既有组合，并非要求桌面直接改调其 `submit(String)`：
当前该 API 不足以承载桌面的流式 delta、既有 session 续接、TurnRequest.system/security、审批 ID/continuation、
域上下文和网关写闸。应扩展同一受管执行适配以保留这些语义，不能降格 HTTP/CLI 功能来换取 managed 标签。
控制面仍使用短锁与不可变执行视图，不持全局锁等待 Provider；避免父 Hook 同步等 child 时耗尽共享回合额度或重入父会话锁。

`with_module` / `with_behavior` / PromptOverlay 可以保留公开 ABI，动态内容统一转换为不可信 ToolEventV1；
显式静态 Host policy/system 入口保留。宿主明确授予内部能力；禁止注册即授权、猜工具名调用、虚构 receipt 或同名工具借权。
每个 Hook 的同一结果只注入一次；空观察仅审计；关闭/停止模块后不得静默运行旧回调。
旧 Overlay 测试若钉住暂态 system 投影，应保留其顺序、生命周期、预算和不污染用户历史的行为断言，
将表示层期望有说明地改为 V1 配对；不能删掉断言或放宽安全契约。

现有回归重点：`production_cognitive_v1.rs` 的
`desktop_production_loads_old_data_and_persona_through_registered_tools_without_system_or_double_injection`、
`stopping_production_memory_prevents_implicit_legacy_fallback_and_plain_kernel_needs_no_memory`、
权限拒绝 / guessed receipt / module-only 工具用例；`cognitive_module_abi.rs` 的 Hook/retry/stop/approval 顺序；
`crates/adapters/cli/tests/canonical_cli_bootstrap.rs`；
`crates/adapters/gateway/tests/canonical_entry_e2e.rs`、`multi_client_session.rs`、`domain_partner_chat.rs`、`user_domain_isolation.rs`。

## D4：Profile / Memory 授权映射

### 已有隔离与准确缺口

| 位置 | 当前行为 / 目标差异 |
|---|---|
| `crates/engine/runtime/src/canonical/execute.rs:2494` 附近 `run_hook` 注册适配分支 | `session.principal.is_some()` 就跳过旧全局认知 Adapter，是 fail-closed 隔离；不是旧 Profile 兼容完成。原始 with_module 分支不经过该检查，需在兼容归并时覆盖 |
| `crates/engine/runtime-assembly/src/canonical/cognitive.rs:1271` `MemoryRecallModule::on_hook` | 以旧 session 构造 MemoryRecallQuery，使用既有排序/访问记录/调优；ModuleContext 没有 Profile 授权映射。不应只替换 session 字符串就认为所有层都隔离 |
| 同文件 `:1613` `MemoryWritebackModule::on_hook`、`:1658` / `:1676` | 旧 episode、typed sink、materializer 使用旧 Session scope，来源字段另行保存；Persona consolidation 仍走既有 store。需定义目标 scope 与真实来源 Session 的分离规则 |
| 同文件 `:547` `PersonaConstantInjectionModule`、`:1910` `PreferenceRecallModule` | PersonaMemoryStore::load 没有 principal 参数；PreferenceStore::recall_for_context 以 SessionId 查询。共享的 Partner/Bio/Organ 状态也须逐项归属，不能仅隔离召回文本 |
| `crates/engine/memory/src/layers.rs:74` `MemoryRecallQuery::new` | 当前代码默认 visible_scopes 仅自己的 Session；上方注释仍提及 Global，不应按旧注释声称自动全局可见。显式 Global / Project / Persona 旧用法仍须兼容并受授权 |
| `crates/engine/runtime-assembly/src/canonical/memory_module.rs:50` `namespace_key`；`memory_module/execution.rs` `execute_checked` | 原生私有 Memory 已用 `(ProfileId, NamespaceId)` 分区，仅 Working/Episodic，并重查 episode owner 与 governed content；不提供旧全局多层语义的自动映射 |
| `crates/engine/runtime/src/canonical/profiles.rs` `ProfileConfig::agent_config`；`instances/profiles.rs` `activate_profile`；assembly `profile_store.rs` | Host 校验 principal，切换停止/销毁旧 main 并创建新 Session，配置 CAS 已有。未包含旧数据的认领、共享或跨身份读取授权 |

目标保留无 principal 的既有全局认知模式，旧 DB 不重建、不自动迁库。
私有 Profile 默认拒绝所有未映射的旧记录与共享状态；不存在“未找到私有数据就查全局”的回退。
Host 显式配置来源 scope/store → 目标 Profile/Namespace 的授权映射及撤销；模型参数不能选择、更改或扩大映射。
定义 Global/Session/Project/Persona、偏好、typed materialization、Organ/Partner 状态各自的授权与所有权，
明确仅可读共享与可写私有的差别；保留原检索排序、治理状态、protected、CAS、typed sink、调优与物化语义。
读取过滤、工具参数校验、写入分区、后台 organizer、fork/reload 和 Profile 切换均须在边界重验。
来源 metadata 保留真实 session/request/trace，不把 namespace 分区键冒称原始来源。
撤销/切换后，在途旧响应、旧 receipt、旧工具或缓存不得给新主实例注入内容。

现有测试保留：`crates/engine/runtime-assembly/tests/convergence_memory.rs` 的跨 Profile 召回、参数不能选 namespace、
无 principal 拒绝、CAS 切换、后台整理、冻结 update/delete、protected、fork/reload 与 governed-content 用例；
`cognitive_convergence_vertical.rs` 的 Global / Project / Persona scope、legacy fail-narrow、排序与更新/删除失效用例；
`production_cognitive_v1.rs` 的旧库及 Persona 兼容；`portable_host.rs` 的私有历史与可选 ProfileStore。

## D5：Legacy V1 收尾与唯一执行账

兼容公共名可继续存在：BehaviorModule/Module、with_module/with_behavior、PromptOverlay、ModuleInvoker、
SubLoopSpawner、旧工具 invoke/frozen/session ABI、裸 Runtime 观察 facade。
完成标准是这些执行入口都委托同一实例服务、Kernel 和 admission/Dispatcher；不能保留另一个真实 Provider 执行后端。
不要求删除兼容类型或重写旧日志来制造“零 Legacy”。

统一检查 `crates/engine/runtime/src/canonical/session.rs` 的 SessionEventKind、`registered_hooks.rs` 的 V1 记录与配对、
`accounting.rs` 的 RequestMeter/ToolMeter、`instances/control.rs` 的取消排水：
每次真实调用只记一次，父子来源可追踪，module instance / agent / session / request / call 身份不混用；
子实例的工具调用不再仅以 SubLoopResult 临时返回而缺少可持久审计的同一事实。
审计专用空结果与已投影内容分开，开放模型调用批先闭合再提交自动工具对，Provider wire call id 不作跨执行去重身份。
保留旧日志 reader；新增记录需升级 reader 的限制必须写明，不能删历史事件或承诺旧二进制可无损降级读取新事件。

现有安全回归保留：`crates/engine/runtime/tests/registry_runtime_v1.rs` 的开放批延迟、生命周期、
Hook timeout/panic/cancel、schema/scope/governance、冻结 payload 与错误相关性用例；
`approval_freeze_convergence.rs`；assembly 的 `resource_factory_runtime.rs`、`convergence_mcp.rs`、
`production_mcp_factory.rs`、`convergence_agents.rs`、`portable_host.rs`。
特别保留 `approval_for_old_instance_cannot_authorize_same_named_replacement` 与
`bounded_native_drain_publishes_stopping_and_retry_preserves_resource_owner`，不能以取消成功返回替代排水证明。

## 必须新增的真实 Runtime 集成证据（均为计划，M0 未创建 / 未实测）

以下用脚本化离线 Provider 驱动真实 Runtime、实例服务、治理、Dispatcher、SessionStore 和计量；
不能仅 mock AgentInstance 返回文本证明收敛。沿用现有测试目标，必要时增加以职责命名的集成目标。

| 验收场景 | 必须观察到的行为 | 适合扩展的现有测试目标 |
|---|---|---|
| 受管父 Hook → SubLoop → 允许工具 → 完成 | 父子身份/历史分离，子调用真正进入 execute_outcome，成对工具事实持久化，仅 owner 收结果；model/system/messages/rounds/results 保真 | `canonical_subloop`、`convergence_agents` |
| 子循环预算与错误 | 零轮/不收敛、共享 turn 预算跨模块耗尽、重试不重置预算、owner 累计跨 child 替换、部分失败已有账保留 | `canonical_subloop`、`canonical_invoker_handle`、`convergence_agents` |
| invoker 单次受限执行 | 无工具声明、不派发意外 tool_calls、不递归 Hook，原始响应/served_by 保真，主 transcript 不变，跨 turn/session 预算独立 | `cognitive_module_abi`、`canonical_invoker_handle`、`organ_llm_bridge_compat` |
| 子权限与审批攻击 | allowlist 越界、父 scope 越界、伪 principal/receipt、Deny、RequireApproval 全部拒绝且无副作用；不产生遗留隐藏审批；父合法审批仍可恢复 | `canonical_subloop`、`registry_runtime_v1`、`approval_freeze_convergence` |
| timeout/cancel/stop/unload/替换 | 模型阻塞和原生工具在途均取消并有界排水；旧 handle 无法复活；父取消传播已绑定 child，停止一个实例不影响主 Agent/其他实例/共享 Provider | `convergence_agents`、`resource_factory_runtime`、`convergence_mcp` |
| 统一核算 | 成功/失败/取消与缓存未知的事实归 child/module owner；父及 Host 汇总不重扣；Provider 并发/退避继续共享；无价与无 usage 不造数 | `convergence_agents`，扩展其 existing usage/budget/provider-limit 用例 |
| 原始 with_module 兼容 | 与 with_registered_behavior 委托同一工具路径；Hook 顺序、Retry/Stop/OnError、空观察、开放批配对一致；stop 后无 fallback，动态内容无 system 或重复注入 | `production_cognitive_v1`、`cognitive_module_abi`、`registry_runtime_v1` |
| 默认桌面受管主 Agent | 通过真实 CLI bootstrap / gateway facade 可观测主实例身份与 usage；已有 Session 续接、意图、静态 system、model override、流式和审批 continuation 保留 | `canonical_cli_bootstrap`、gateway `canonical_entry_e2e`、`multi_client_session` |
| 授权旧库映射 | 旧数据/Persona 在无 principal 模式保持；有明确映射时仅获准 Profile 可读，未授权 Profile 不读、不写入、不接收缓存；映射撤销及在途切换安全 | `production_cognitive_v1`、`convergence_memory`、`cognitive_convergence_vertical` |
| 双记忆模块组合 | 生产 compat 与原生 MemoryModule 同时选择时每个业务结果与写回只发生一次；无映射时不隐式查询全局；module stop 不回退 | `production_cognitive_v1`、`convergence_memory` |
| 平台与网络保留 | no-default PortableHost 无桌面工具/组网依赖；可选 Memory/Profile 可用；桌面原 Mesh/NAT/P2P 继续存在，非本批清理对象 | `portable_host`、既有桌面回归及 Android check/build |

## 后续实施切片（六个提交单位；本批不执行）

按依赖顺序推进；每个切片都必须提交可审查的代码与行为证据，状态仍按完成范围记账。
接口接缝与预算/权限策略应在首片评审明确，不能让后续片靠临时直连 Provider 兜底。

| 切片 / 归口 | 变更范围与提交闭环 | 验收 / 依赖 |
|---|---|---|
| S1 runtime：SubLoop 实例适配 | 在既有 agents/instances 执行端口补规范化输入、逐任务限制与完整结果；RuntimeSubLoopSpawner::spawn 委托该端口，删除 spawn_bounded 的 Provider/round loop；绑定 owner/principal/父取消与共享预算。兼容宿主供给同一实例服务，不新建 Runtime | D1 与表中子循环/权限/取消集成场景；现有 SubLoop 行为全部保留。识别回合 semaphore 和父 Session 锁重入问题 |
| S2 runtime / assembly：ModuleInvoker 收口 | 两个 ModuleInvoker impl 都委托 S1 的单次、无工具执行模式；保留 ABI 和 Organ bridge；独立 child/owner 用量、原响应与 served_by。移除它们的 router Provider 路径 | 依赖 S1；D2 无工具/无递归/共享预算/完整返回与准确核算。不能只修改测试构造的 RuntimeModuleInvoker |
| S3 runtime：旧行为入口归并 | with_module / with_behavior 归 RegisteredBehavior、固定 Hook、统一授权与 V1；取消直接 on_hook 执行 fallback 和动态 system Overlay 双路径；保留公开名称、指令时序及兼容 reader | 依赖 S1/S2；D3 防重复与 D5 配对/日志回归。历史表示层断言迁移需逐项说明，安全断言不减 |
| S4 assembly / memory：Profile 映射 | Host 显式授权旧 scope/store 与 principal 对应关系；原全局模式保持；私有 namespace 的读写/缓存/typed sink/Persona/偏好/Organ 状态隔离及撤销 | 依赖 S3 的统一认证边界；D4 映射、跨 Profile 攻击与旧库回归。未映射继续 fail-closed，不提前移除 principal guard |
| S5 cli / gateway / assembly：默认受管桌面主 Agent | bootstrap 持同一 AgentRuntime，默认 main 受管；CLI、网关、审批、流式、心跳调用委托同一执行适配；保留既有 Session/domain/security/static system 与写闸；显式 Host/module grants | 依赖 S1–S4；从真实 bootstrap 驱动集成证据。PortableHost 继续桌面无依赖，桌面网络能力保持 |
| S6 runtime / docs：Legacy V1 验收收口 | 清理余留直连侧执行与重复计量、补子实例日志/事件/cancel 的统一证据；保留必要 compat API；运行原门禁、平台/桌面回归并更新验收账 | 依赖 S1–S5；完整真实 Runtime 场景和旧回归均满足后，才可将 Production Convergence 标为完成 |

## 不可变约束与设计取舍

- Kernel 只处理 User Message / LLM Response / Tool Call / Result；Profile、Memory、Organ、平台选择留在 Host/模块。
  不重写 Registry、Dispatcher、Module Factory、MCP Lifecycle、Multi-Instance Runtime；不引入其他项目代码。
- 审批、冻结载荷与世代身份、意图/安全上下文、schema、权限、隔离、取消和生命周期保证不降低；
  现有模块/Provider 并发与局部资源限制不改成全局串行锁。
- 桌面 Mesh/NAT/P2P 保留；Android PortableHost 保持 desktop-free，不能通过增加桌面依赖解决兼容缺口。
  LOCKED 哲学锚/键集/脊柱、workspace.version 与 R11 baseline 不在施工面。
- **O-6 总体最优**：先打通已有实例服务的兼容语义，再接默认桌面；拒绝先删旧 API 或测试，因为会损失业务行为与验收依据。
- **O-6 系统最优**：真实 Provider/工具边界只计量一次，权限与资源由原所有者管理；拒绝在每个兼容 Adapter 再做模型客户端、权限器或成本账。
- **O-6 架构最优**：补现有实例请求/结果端口与普通模块适配；拒绝“第二个 AgentRuntime”、移动旧循环冒充收敛、Kernel 工具名/领域分支及并行 Registry。

## M0 核验账与实施前门禁

日期：2026-10-29。当前环境为 Linux，`rustc -Vv` 为 `1.85.1`（host `x86_64-unknown-linux-gnu`），
`cargo -V` 为 `1.85.1`，`python3 --version` 为 `3.13.5`。本文证据路径和基线行号可用下列只读检索复核：

```bash
git branch --show-current
git log -1 --format='%H %s'
rg -n 'spawn_bounded|complete_for_session|impl ModuleInvoker' crates/engine/runtime/src/canonical/{subloop,module}.rs
rg -n 'with_module\(|with_registered_behavior|RuntimeModuleInvoker::new|PromptOverlay::' crates --glob '*.rs'
rg -n 'build_canonical_runtime|execute_canonical_cli_turn' crates/adapters/cli/src/lib.rs
rg -n 'principal|namespace_key|visible_scopes' crates/engine/runtime/src/canonical/execute.rs crates/engine/runtime-assembly/src/canonical/memory_module*
```

| 本批命令 | 结果与证据（不作测试通过声明） |
|---|---|
| `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | exit 101；无法解析 `index.crates.io`，依赖解析在下载 `anyhow` 前失败，未进入测试；`/tmp/apeireth-m0-cargo-test.log` |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | exit 101；当前工具链无 clippy 子命令；`/tmp/apeireth-m0-clippy.log` |
| `cargo fmt --check` | exit 101；当前工具链无 fmt 子命令；`/tmp/apeireth-m0-fmt.log` |
| `python3 scripts/docs_truth.py --verify` | exit 1；`rel.release_asset_verify` 报 `test path missing: [` / `]`。当前没有 PyYAML，内置解析器将既有 `tests: []` 当字符串；未修改脚本或 claims；`/tmp/apeireth-m0-docs-verify.log` |
| `python3 scripts/docs_truth.py --check` | exit 0；状态生成物与 README badge 对照通过；`/tmp/apeireth-m0-docs-check.log`。单独执行 verify 与 check，避免该脚本参数优先级令 check 掩盖 verify 失败 |
| `python3 scripts/arch_guard.py --strict` | exit 0；保留原有 NOTE/例外，不新增豁免；`/tmp/apeireth-m0-arch.log` |
| `python3 scripts/verify_changed.py --files docs/04-internal/kernel-convergence-remaining-debt-map-2026-10-29.md --log /tmp/apeireth-m0-verify-changed.log --arch-strict` | exit 0；显式 doc write_scope，架构检查通过，Rust/Toml 阶段按文档批不执行；不是 Rust 测试通过证据 |

本批只写债务地图，未触及生产代码、测试或 LOCKED，未引入依赖，未提交。
所有新增场景均为后续要求；本批没有真实 LLM、外部 MCP、原生桌面窗口或 Android 设备/交叉构建实测。
环境性验证缺口在本文登记，后续切片应在可用工具链与依赖环境重跑原测试、clippy、fmt、文档真源及完整 wave 门禁，
不得以本次静态审计或上批历史全绿替代验收。
