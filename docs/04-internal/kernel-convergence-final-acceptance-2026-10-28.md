# Kernel Convergence Final Acceptance Report

[Document-Meta]
Document: docs/04-internal/kernel-convergence-final-acceptance-2026-10-28.md
Version: 1.0
Last-Modified: 2026-10-28
Status: 连续实施中；未宣称 Production Convergence 完成
Author: Codex

## 本轮基线

本地分支 codex/kernel-convergence-final，HEAD b33ba7e5bf35db5682882fb030403dca6b68a9b1，工作区 clean。
fetch 发现远端 KC 已从 149291e7 快进到同一 b33ba7e5；上一轮五提交均实际存在且为 HEAD 祖先。
本轮不是从旧 KC 重建成果；不修改 main、不强推，不使用历史门禁数字冒充实测。

本轮改动前聚焦基线：convergence_agents / convergence_memory / convergence_mcp 实际通过；fmt 与 strict arch 通过。
原有依赖方向、LOCKED、数据库结构和版本不变。

## M1：生产认知兼容工具

Implemented / Wired / Tested（普通桌面默认组合）：

```text
CLI + existing Desktop/Gateway bootstrap
 → ProductionModules.register_with_policy (Host 选择并授权)
 → register_into → with_registered_behavior
 → PluginManager → 注册的 module-only Tool
 → 固定边界 → Hook Dispatcher → 统一 Admission/Schema/Scope/Governance/Dispatcher
 → 既有 Memory / Persona / Organ 引擎
 → ToolEventV1(Module) → 合法成对 Provider 投影
```

- 原检索、排序、治理后的记忆内容、typed sink、物化、调优及 Organ 回调对象均复用。
- 动态数据去除 Overlay 的 system 角色，只作为不可信工具结果；静态 Host system 入口保持显式。
- 内部回调工具不向模型广告，且准入层拒绝模型猜名调用；凭证绑定当前 session/request/trace/owner。
- 空观察结果仅记录 ToolEventV1 和 ModuleToolEventObserved，不制造空工具对或伪称已投影。
- 引擎读取去除已认证模块投影的业务历史，避免检索数据再次作为用户内容被写回。
- 模块停止后 prepare 凭证拒绝新工作，旧 Behavior 元数据不形成隐式 fallback。
- 旧仅声明认知配置但使用 DenyUnconfigured 的嵌入 Host，现在必须显式授权所选 callback capability。
  CLI 使用同一配置选择结果授予精确能力；新测试专门验证拒绝授权不产生记忆副作用。
- 旧 Profile 未隔离的数据不自动纳入新 principal：当前此 Adapter 对有 principal 的会话拒用旧全局引擎。
  通用 Profile 可选装既有 Namespace MemoryModule；完整旧数据/Persona 的授权 Profile 映射仍为 Partial。

实际新增 Runtime 验收覆盖旧文件库重新加载、Persona 恢复、Organ 调用预算、单次工具注入、
无 system 动态数据、禁用无 fallback、无 Memory 对话、权限拒绝和模型伪造内部调用。

本阶段实测（2026-10-28，Rust/Cargo 1.97.1，PowerShell 7.6.0，Linux Cloud，jobs=2）：

- 新生产验收 4 passed / 0 failed；两项既有记忆回归全部原断言保留，并增加工具角色及闭合协议断言。
- runtime + runtime-assembly 全目标相关回归：97 suites / 1171 passed / 0 failed / 0 ignored。
- runtime / runtime-assembly / CLI 全目标 Clippy -D warnings 通过；fmt 与 strict arch（0 violations）通过。
- 日志：/workspace/scratch/apeireth-production-adapter-regression-final.log、apeireth-production-adapter-clippy-final.log、apeireth-production-adapter-arch-final.log。
- 两项旧测试先揭示默认 capability 拒绝；按生产 Host 新授权边界补精确 grant，没有改成 AllowAll 或放宽数据/遗忘断言。
- 未执行本轮最终全量五段、Android 或真机验证，不提前记通过。

## 连续实施状态

| 目标 | 当前真实状态 | 退出条件 |
|---|---|---|
| M1 默认 Memory / Persona / Organ 注册执行 | Implemented / Wired / Tested；授权 Profile 映射 Partial | 补 scoped 旧身份绑定；保留成熟数据语义 |
| M2 Definition→Instance Factory | Not Implemented（下一内部切片） | 独立资源、回滚、停止、重启和旧句柄测试 |
| M3 MCP / Managed 热生命周期 | Partial，沿用上轮基础 | 外部 Arc 撤销、动态替换及释放控制锁 |
| M4 旧 SubLoop 进入 AgentInstance | Not Implemented | 移除独立 while/provider 后端，保留旧 ABI 行为 |
| M5 全路径收敛 | Partial | Raw with_module / SubLoop / 注入出口逐一收口 |
| M6 Android assembly | Not Implemented | 可选 Memory/Profile，无桌面依赖的 Host check/build/turn |
| M7 全量与发布准备 | 相关回归通过；最终门禁待执行 | 全量、性能、文档、Git/bundle 证据一致 |

重要决策及实际审计入口见 [ADR-0022](../adr/0022-production-kernel-convergence.md)。
每个稳定内部切片提交后自动继续，不请求阶段批准。

## M2 resource factory evidence

Implemented / Wired / Tested: configuration-only ModuleDefinition,
module_from_definition compatibility with existing registration, authoritative
ModuleInstanceId, deferred capability validation before activation, owned resource
rollback, synchronous revocation, reverse close, observable/retryable close errors,
and generation-specific tool handles. Existing legacy plugins remain compatible;
they are not automatically converted into resource factories.

Commands: `cargo test -p apeireth-plugin --all-targets --locked --no-fail-fast -j 2`
(3 suites / 287 passed / 0 failed / 0 ignored) and
`cargo test -p apeireth-runtime-assembly --test resource_factory_runtime --locked -j 2`
(1 passed). Factory tests retain old external Arcs across stop and re-registration;
the real Runtime test verifies V1 facts and separate resource call counts.
Plugin Clippy with `-D warnings` and strict architecture guard passed.
Evidence: `apeireth-resource-factory-regression.log`,
`apeireth-resource-factory-runtime.log`, `apeireth-resource-factory-clippy.log`.

## M3 production MCP and hot lifecycle evidence

Implemented / Wired / Tested:

- ProductionModules MCP uses McpDefinition → module_from_definition → existing PluginManager.
- Fresh per-registration connections, catalog and owned stdio resources; the definition's observation is Weak.
- Connection epochs and task scopes invalidate both managed and raw cached tool handles.
- Explicit close revokes and reaps while external Arcs remain, without strong_count assumptions.
- Runtime turns and approval continuations use immutable execution membership views. Shared session locks,
  dispatcher, usage and provider resources remain the original objects; snapshot drop cannot stop the owner.
- Managed hot registration/refresh/stop/unregister use bounded control operations; module-owned agents receive
  scoped creation ports on hot registration. Dependency/provider rejection happens before cancellation.
- An old model response cannot call a replacement generation. An old factory approval carries its original
  ModuleInstanceId and cannot authorize the same named replacement.

`apeireth-production-mcp-process-test.log` proves a real controlled Python JSON-RPC stdio server is
started, invoked through a production Runtime turn and reaped at stop while old bridge/tool Arcs remain;
restart creates a different bridge and old connection handles remain stopped. This is a local transport
acceptance test, not a real external MCP service validation.

`apeireth-hot-lifecycle-tests.log` records concurrent-agent and hot replacement regression.
`apeireth-production-m3-regression.log` records the expanded kernel/plugin/assembly regression
(103 suites / 1471 passed / 0 failed / 0 ignored before the final stale-approval test addition).
Final frozen-source full-gate results are recorded separately below when executed.

Compatibility change: configuration-only callers use mcp_definition().config()/startup_log();
mcp_bridge() now returns Option<Arc<McpToolBridge>> after initialization, rather than a prebuilt
Option<&Arc<...>>. No in-repository production caller relied on the old pre-initialize reference;
existing configuration tests now also assert that no bridge/process is acquired before initialization.

## M6 portable assembly evidence

Implemented / Wired / Tested on native Linux: existing desktop surface preserved
by default; no-default-features exposes PortableHost over the same AgentRuntime,
with independently selectable memory and profile-store features. No new crate or
external dependency was added. Module selection, provider injection, scope and
permissions remain explicit; an empty configuration installs no shell, process,
filesystem, memory or networking module.

PortableHost exposes initialization, main agent creation, submit, profile binding,
snapshot, tool_scope, tool_events, usage, cancellation and shutdown. Core event
sinks and managed approvals remain available through the existing Runtime APIs.
A profile switch replaces only the main instance; a failed replacement leaves the
old instance stopped rather than reviving private state.

Native minimal configuration: 4 passed / 0 failed / 0 ignored.
Native optional memory + profile-store: 6 passed / 0 failed / 0 ignored.
Tests use the real managed Runtime with Mock Provider and real optional SQLite
persistence; no external API or Android device is implied. The resource test also
proves an owned pending background task drains before resource close while the
Host retains an external resource Arc. Evidence:
`apeireth-portable-host-final-tests.log`, `apeireth-portable-optional-final-tests.log`.
The initial Android assembly check/build passed; final frozen-source target
verification is still required below. The guide is
[Portable Runtime Assembly](../02-guides/portable-runtime-assembly.md).
