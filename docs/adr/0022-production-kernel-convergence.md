# 0022. 生产认知兼容与统一实例执行

[Document-Meta]
Document: docs/adr/0022-production-kernel-convergence.md
Version: 1.0
Last-Modified: 2026-10-28
Status: Accepted；owner Final Production Convergence 授权，实际状态由验收账登记
Author: Codex

## 基线

本地及 fetch 后远端均为 b33ba7e5bf35db5682882fb030403dca6b68a9b1，五个前序提交均存在且为 HEAD 祖先，工作区 clean。
不恢复重造提交，不修改 main/LOCKED，不批量升级依赖。

## 实施依赖与决策

1. 先将 ProductionCognitiveModules 的既有业务对象包装成普通 Plugin/Module Tool，保留检索、排序、调优、物化及 Organ 引擎。
   Runtime 只负责提供只读边界快照、统一准入与协议；动态数据不再作为 System Overlay。
2. 兼容工具的执行凭证绑定 session/request/trace/module，不接受模型伪造的上下文。静态 Host policy 继续使用显式配置入口。
   空观察回调仍可审计，但不向模型注入空工具对，避免随历史不断增加无用上下文。
3. 模块工厂在现有 PluginManager 内创建独立实例；资源 owner 的逻辑撤销、关闭排水与 Arc 内存释放分开。
4. Managed 热控制需要释放请求期间的全局控制读锁；不可变执行快照保存旧 generation，热替换不能借同名工具调用新权限。
5. 旧 SubLoop ABI 最终只适配独立 AgentInstance，执行仍进入同一个 Runtime.execute_outcome；不保留另一份模型循环。
6. 平台组合优先 feature 隔离既有 assembly，使用同一 Kernel/实例管理及可选 Memory/Profile，不复制 Runtime。

以上为连续实施顺序，不表示全部已完成。每个切片须实际运行并提交，再继续下一切片。

## 生产路径审计

| 当前入口 / 实现 | 实际缺口 | 目标及兼容约束 | 验收 |
|---|---|---|---|
| CLI bootstrap → ProductionCognitiveModules.register_into → with_module → Runtime.run_hook | callback 直接执行，PromptOverlay 进入 provider system 消息 | 普通注册模块工具执行原业务；原 DB/排序/typed sink/调优保持；动态数据变 V1 工具结果，保留静态 Host policy | 旧库/Persona/Organ 真 Runtime、配对、禁用及不重复注入 |
| RuntimeSubLoopSpawner.spawn_bounded / invoke | 自有 while round/provider 路径 | 旧 ABI 映射 AgentInstance policy，独立 owner/context/预算/权限，不扩大父权限 | 原 subloop 语义 + 实例核算/取消证据 |
| PluginManager.register(Arc<Plugin>) | 声明和资源同对象，可跨 Runtime 意外共享 | Definition→factory→独立 ModuleInstanceId，失败回滚、撤销及排水 | 双实例、初始化失败、重启资源替换 |
| McpPlugin→bridge/connections/transports | 显式 stop 已关闭；外部 Arc 放弃路径和旧句柄重启世代仍需收口 | 失效 token/generation 不可复活，关闭不依赖 strong_count | 持旧 handle 的 stop/restart/unload/Drop 原生资源测试 |
| AgentRuntime.Inner.kernel RwLock | read guard 覆盖整个 provider future，控制面等待 quiescent | 请求持不可变执行快照，控制变更串行且有界，旧 scope 即时撤销 | 并发模型请求期间热替换，旧响应不能调用新代工具 |
| runtime-assembly 无 feature 边界 | Kernel 自身 Android 库可构建，但 assembly 强依赖桌面工具 | 可裁剪 Host 装配；Memory/Profile 可选，默认不注册桌面高权限能力 | Android check/build + 无桌面模块 mock turn/shutdown |

## 公开兼容与权限

保留 with_module/invoke/SubLoop 等入口时必须清楚委托关系，不能将兼容名称当第二执行后端。
生产 Host 显式为所选认知模块授予内部工具 capability；注册和工具名称本身不是授权。
受限 Profile 不自动继承旧全局 Persona/Memory 数据；跨身份映射须明确、可审计并由测试证明。

## O-6

- 总体最优：用生产 Host 的真实回合证明迁移；拒只更新接口和文档。
- 系统最优：复用成熟认知引擎、数据、审批和 V1；拒数据库重建、重复注入和新权限旁路。
- 架构最优：沿用 Registry/Dispatcher/Instance Runtime；拒新的注册表、全局模型串行锁或复制子 Loop。

## 已实现的生产认知切片

ProductionModules.register_into 为既有业务对象注册 RegisteredBehavior Plugin；CLI 使用 register_with_policy 显式授予所选内部能力。
回调凭证只在当前边界有效，绑定 session/request/trace；模型无法调用 module-only 工具，即使猜到名字或已获同名 capability grant。
动态结果丢弃原 Overlay 的消息角色，作为不可信 ToolEventV1 payload，沿用成对投影；静态 TurnRequest.system 不变。
无输出的观察调用用既有会话账新增 ModuleToolEventObserved 确认审计消费，不伪称已投影或向模型增加空工具对。
认知引擎读取排除已认证模块投影的业务历史，避免把自己的检索内容再学习成用户输入。

固定 Hook 词汇追加 TurnStart / BeforeFinalCommit / OnError，以保留旧 ABI 的准确时序；
SessionStart 仍为每 session 一次，TurnStart 是每 turn 一次。没有动态脚本或任意可插流水线。
ToolEventV1 字段、version、call/result 相关性与 Provider 投影格式未改；新 reader 读取所有旧事件，
新增 Hook/会话审计变体要求升级后的 reader，不声称旧二进制可降级读取新增记录。

当前 Profile principal 存在时，旧全局认知 Adapter 不执行，防止旧未隔离数据进入私有 Namespace。
现有原生 Namespace MemoryModule 仍可组合使用。生产旧数据的通用 Profile 映射尚需下一切片完成，不能将该项标为已迁移。

## Resource factories (M2)

`apeireth_plugin::resource_factory::ModuleDefinition` is configuration plus an
async `instantiate(context, resources)` function. Hosts pass
`module_from_definition(definition)` to the existing `with_plugin` or
`register_module` entry. This adapter is an ordinary Plugin; it does not introduce
another registry. Each registration receives the registry's ModuleInstanceId,
read-only activation token, task scope, and explicitly granted agent port.

The factory registers resources immediately after acquisition in ModuleResources.
Shutdown revokes them synchronously, drains the existing module task scope, and
closes resources in reverse acquisition order. A failed or cancelled close retains
ownership for retry. ResourceStatus exposes close/error state. Factories must not
register shared providers or host database pools as private resources.

Deferred capabilities are checked against the definition's exact manifest and
validated again before activation. Factory tool handles hold an instance-specific
activation token and task scope. Initialized tools cannot execute, and stop,
unregister, replacement, and manager drop revoke old handles even if external Arcs
remain. Restart means creating a new adapter/instance, not reopening old resources.
Hot registration also rejects provider declarations before factory construction;
provider membership still requires rebuilding the router.

Verified by foundation factory rollback/retry/independence tests and a real
Runtime turn followed by replacement while retaining an old raw tool Arc.

## Managed hot lifecycle and MCP (M3)

Managed turns and approval continuations obtain an immutable Runtime execution
view under a short read lock, then release that lock before awaiting a provider.
The view copies catalog membership, not running resources. It shares the original
provider router, accounting, dispatcher, session locks, event sink, metrics,
module scopes and activation tokens. Its PluginManager is an explicitly read-only
derived view: it cannot register, start, stop or cancel, and dropping it cannot
stop the real lifecycle owner. This preserves one mutable registry authority.

AgentRuntime exposes bounded register_module, refresh_module_tools, stop_module
and unregister_module operations. A new registration receives the existing scoped
module-agent factory. Re-registering a stopped definition creates fresh resources;
requests already in flight keep their old tool membership and closed scopes.
Refused dependency/provider stops are checked before cancellation. Shutdown
races are checked again after acquiring the control lock.

Factory approvals also freeze ModuleInstanceId in a versioned invocation envelope;
the factory adapter unwraps the original tool-owned frozen operation only for
that instance. Old approvals cannot authorize a replacement with identical tool
names, schemas or arguments. Existing non-factory frozen ABIs are unchanged.

Desktop ProductionModules now registers a configuration-only McpDefinition through
the same resource factory. It creates a fresh bridge, connection set and catalog
per registration. McpDefinition keeps only a Weak observation of its latest
bridge. MCP permissions remain explicit and are not copied into a new instance.
The production compatibility accessor mcp_bridge now returns an owned Arc only
after initialization; pre-initialize configuration is available through
mcp_definition().config(). Existing construction-only McpModule bags do not own
production discovery or a second execution backend.

Connection revocation closes an epoch-specific task scope and invalidates raw
tool handles. Stdio/SSE revocation aborts readers and fails pending replies;
explicit close joins/reaps owned work even while external bridge/channel Arcs
remain. Connection close retains its owned channel across cancellation/failure
for retry. Reopening a connection is crate-private; tools from an old activation
cannot revive when the bridge is reused. Explicit shutdown is required for
observable drain/close results; Drop performs synchronous revocation only.

Runtime tests cover hot replacement during a blocked model call, stale approvals,
module-owned hot agents, preserved main-agent concurrency, and an actual controlled
stdio server through ProductionModules. No real external API was used.

## Portable assembly (M6)

runtime-assembly now has default desktop composition and independent memory and
profile-store features. The minimal feature set composes the same managed
AgentRuntime with injected RuntimeBuilder ports. It does not depend on the desktop
tool crate, cognition modules, SQLite, Gateway or desktop networking. The original
desktop surface remains re-exported with default features; no desktop subsystem is
deleted. Cargo.lock and external dependency versions are unchanged.

PortableHost owns a single main instance and exposes submit, cancellation, profile
binding, tool scope, durable V1 events, usage and shutdown. Host policy, provider
plugins and module selection remain explicit. Cancellation stops the current main
instance; profile replacement creates a fresh one. A failed replacement does not
reactivate the old private context. Profile configuration types remain portable;
SQLite persistence and MemoryModule can be compiled independently of desktop.

Offline native tests cover minimal turns, unknown shell errors without fallback,
private profile histories, blocked-request cancellation, owned background task
drain with an external resource Arc, optional Memory Hook and durable profile
loading. Android target check/build and dependency-tree evidence are recorded in
the final acceptance report; cross compilation is not device validation.
See the [portable Host guide](../02-guides/portable-runtime-assembly.md) for APIs,
feature matrix and exact commands.
