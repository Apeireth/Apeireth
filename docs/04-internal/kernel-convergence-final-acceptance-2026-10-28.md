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
