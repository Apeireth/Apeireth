# 0020. Capability 模块归属与执行上下文收敛

[Document-Meta]
Document: docs/adr/0020-capability-module-ownership.md
Version: 1.0
Last-Modified: 2026-10-28
Author: Codex

- Status: Accepted（2026-10-28，owner Phase 3 任务授权）
- Baseline: refactor/kc1-kernel-convergence@8fb1ad7039720eaca1ee131c0e5f295504541d96
- Scope: plugin / runtime / runtime-assembly / tools；不改 LOCKED、main 或依赖版本。

## 决策

1. 沿用 PluginManager 管理模块生命周期，沿用 CapabilityRegistry 作为 Runtime 唯一可执行工具目录。
   插件工具也进入该目录；不再在每次调用时拼接两份候选列表。Plugin 的 CapabilityIndex 只保留声明归属索引。
2. 工具模块复用 Plugin/ToolCapability；增加有默认实现的强类型执行策略与逐调用上下文接口。
   老 invoke/invoke_frozen/invoke_in_session 由兼容 Adapter 转发，不另设执行后门。
3. Desktop 的既有 CapabilityProvider 转为有生命周期的工具模块注册；Shell、文件读写/搜索、结构化进程能力
   可以独立选择。Host 未注册时不提供能力；保留既有工具名、参数、冻结载荷、写入总闸及权限判定。
4. 工具目录条目持有 owner scope；卸载拒新调用并取消旧 handle。插件停止复用 Phase 2 的取消/排水。
   动态 Host/MCP 旧 API 若仍须保留，单独登记其归属限制，不声称完成了动态 Plugin 热插拔。
5. 执行上下文包含独立 agent/session/turn/call/caller/module、授权证据、deadline、取消与 trace，
   不依赖全局单一 Agent Context。权限仍由既有治理判断，工具名不构成授权。
6. Shell 的阻塞原生执行必须响应 future 丢弃/模块取消。复用现有 ProcessExecutor 终止树与输出收集；
   原生 worker 持有 ModuleTaskScope lease，stop 不能在子进程清理完成前假称排水完成。
7. ToolEventV1、固定 Hook 与闭合消息投影继续复用，不修改 Provider 为模块拼装业务内容。
   内部注册式纯状态 control 保留 KC 语义；不能借此执行 Shell/文件/进程。

## O-6 三阶审查

- 总体最优：先迁移实际宿主工具及原生取消链，真实回合验证；拒只给已有工具改模块名字。
- 系统最优：保留治理/冻结审批与 OS 执行基础设施；拒新建权限决策器或复制 Shell/路径约束。
- 架构最优：沿用既有生命周期和目录，强类型数据连接旧 ABI；拒第二套 Registry/生命周期系统。

## 验证与限制

实际路径、迁移清单、剩余 Legacy、测试及同步结果随实现记录在
[Phase 3 交付账](../04-internal/capability-modules-phase3-2026-10-28.md)。
不实施完整 Memory/Profile、后台多实例调度、Android App 或依赖批量升级。

## 实施补充

- SubLoop 保留原 allowlist/预算/治理，但工具校验和实际执行进入同一 Dispatcher，携带父回合身份。
- 原生辅助函数使用按 future poll 安装/恢复的取消/期限桥，跨阻塞线程显式传递；不建立全局 Agent Context。
- 原生 worker 同时持有资源 permit，异步超时不能提前释放 Serial/Limited 额度；保存的公开上下文只持 Weak。
- 正常退出也清理留存后代；Unix process-group 仍只承诺原 PARTIAL 等级，不冒称操作系统强隔离。
- 旧非 V1 log-only 模式、动态 MCP owner、专有设备资源、自动 Memory overlay 逐项登记为保留兼容面。
