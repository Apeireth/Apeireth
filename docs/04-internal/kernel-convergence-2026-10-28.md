# Registry Kernel 最终收敛施工与验证账

[Document-Meta]
Document: docs/04-internal/kernel-convergence-2026-10-28.md
Version: 1.0
Last-Modified: 2026-10-28
Status: 连续实施中；不宣称最终验收已通过
Author: Codex

## 基线与边界

fetch 核实远端 KC、本地均为 `149291e7504f9a3928dd220010aa777f07a3acae`，工作区 clean。
工作分支 `codex/kernel-convergence-final`，不修改 main、不强推。
ADR-0018～0021 与现有 KC/Phase 3 交付账为设计与兼容边界。
历史全量结果不当作本轮实测；全量五段在最终源码稳定后运行。

## 内部依赖清单

| 工作 | 依赖 | 当前状态 |
|---|---|---|
| 分层任务作用域、有界停止、动态工具 owner | 既有 PluginManager / Dispatcher | Implemented / Wired；相关测试通过，收口检查中 |
| 动态 MCP 生命周期及描述符授权 | 上述所有权、既有 bridge/pipeline | Implemented / Wired；真实 Runtime 与原生清理测试收口中 |
| 独立 Agent Session/Scope/取消 | 所有权、既有 Runtime.execute | Implemented / Wired / Tested；11 项真实实例验收 |
| 实例/模块/全局请求核算、预算与 Provider 限流 | 独立实例、实际请求边界 | Implemented / Wired / Tested；未知用量与估算费用明确分开 |
| Memory tools + 固定 V1 Hook + 后台整理实例 | 独立实例、Namespace 授权、既有 Memory 存储 | Not Implemented |
| Profile 持久配置与授权绑定 | 通用身份与实例机制 | Not Implemented |
| Legacy / Host / Android / 最终回归 | 上述接线与现有平台组合 | 待最终验收；不把接口当交付 |

## M1 实际接线

- 一个 PluginManager、一个 CapabilityRegistry。动态工具仅在 owner 声明的 namespace 下发布；
  catalog 原子替换并撤销旧代执行 scope，旧句柄不能继续调用。
- 子作用域的调用/native lease 同时登记到父模块。撤销子工具不取消同级工具；停止父模块会取消所有后代。
- stop 的 Stopping 检查点在排水前落下。有界 stop 超时保留资源 owner/错误，不允许 unregister；资源释放后可重试。
- Dynamic MCP 是普通 McpPlugin。初始化/发现、更新、连接关闭、读取任务 abort+join、stdio 进程回收归其所有。
  可选 MCP 失败记 Failed/错误并清理，其他独立模块可继续；默认必需 Plugin 的启动失败语义不变。
- ProductionModules 的 MCP 开关现在注册 McpPlugin；旧 McpModule bag 构造接口保留兼容，非生产目录真源。
- 重连 pin 工具 Schema/只读声明，漂移后拒绝旧调用而非盲重试；审批冻结描述符快照。
  Schema/风险改变后必须刷新目录并重新授权；只读预设不得保留为后续写能力的授权。
  显式 Host allowlist 按原配置授予；被自动预设覆盖的临时手工授权需在风险变更后重新确认。
- 原生 stdio 复用 Unix 进程组与 Windows Job Object 机制。Linux 实进程清理已测；不声称 Windows/macOS 硬件已验证。
  Unix 仍为既有 PARTIAL 遏制等级，主动脱组后代不在强隔离承诺内。

## 验证记录

2026-10-28，Rust/Cargo 1.97.1、PowerShell 7.6.0、Linux Cloud，jobs=2。
沿用 Phase 3 仓外工具链环境及 subreaper（只回收已死孤儿，不修改测试/信号/退出码），不改门禁脚本。

- `cargo test -p apeireth-plugin -p apeireth-runtime --all-targets --locked -j 2`：31 suites / 643 passed / 0 failed / 0 ignored。
  日志 `/workspace/scratch/apeireth-convergence-m1-runtime-tests.log`。包含 2 项新所有权/超时回归。
- 真实 MCP Runtime 已验证发现→调用→V1 归属→目录变更→失效句柄→重连→停止→卸载→重新注册；
  活跃调用取消、可选失败隔离、Linux stdio 主进程/后代清理及描述符授权漂移另有回归。
- 原 Phase 3 的 24 项 Runtime 验收与旧 MCP 4 项 Runtime 验收通过；原断言未改变。
- 严格架构检查 violations=0；没有新增架构例外、依赖边或环境开关。
- 新增 MCP 验收 5 passed / 0 failed，旧 MCP bridge 14 passed / 0 failed；安全变更后的 scoped Clippy -D warnings 通过。
  日志 `/workspace/scratch/apeireth-convergence-m1-mcp-final.log`、`apeireth-convergence-m1-mcp-security.log`、`apeireth-convergence-m1-clippy-final.log`。
- 全量门禁、Android 和前端兼容检查留待最终集成后执行，未提前记通过。

原始问题：冻结描述符的 display 参数初次使用字符串而非 JSON Value，编译报类型错后按原接口修正；
未削弱 schema、权限、描述符或原断言。

## Git 与退出条件

每个稳定内部单元创建提交后自动继续，不请求阶段批准。最终事实声明 verified_at 必须指向实际源码提交。
推送前 fetch；403 只尝试一次，不变更凭据，导出相对本基线的已验证增量 bundle。
最终必须补齐各功能 Implemented/Wired/Tested/Partial/Not Implemented、真实测试结果、提交与远端状态。

## M2 实例与资源核算接线

`RuntimeBuilder.build_agent_runtime(AgentHostPolicy)` 构建既有 Runtime 的实例门面；
`AgentRuntime.spawn_agent(owner, AgentConfig)` 创建新 Session/Agent，不复制 Loop。
模块从 `PluginContext.agents` 获得绑定 ModuleInstanceId 的端口，默认空工具范围；
Host 显式配置 `ModuleAgentGrant` 的工具、Principal、实例数、深度、预算与安全上下文。
`AgentInstance.run/pause/resume/stop/destroy` 分别提供独立历史、回合边界暂停、取消、排水与回收。

- Agent 与工具模块的任务 scope 组成去重所有权 DAG；原生 lease 同时归属调用实例及工具模块。
  Agent stop 等待原生工作退出，模块停止传播到其拥有的实例。旧 Agent handle 不会延长 Host 生命周期。
- 并发请求使用 Runtime 共享读锁，不以全局互斥串行化；Provider 配置独立 semaphore。
  可重试失败退避 100ms 起、上限 5s，尊重更长 Retry-After，等待可取消。
- 真实 Provider attempt 在请求边界记账；Session/Agent/Module/Runtime 视图不相加重复计费。
  每实例保留最多 64 条请求事实，累计账独立保留；Host 默认最多 16 活跃、256 审计记录。
  配置的 token/cost 预算约束下一请求，输出上限按剩余额度收紧；不承诺上游报告无法预知的绝对零超支。
- Usage 未报告、cache 未报告、未配置价格分别为未知；费用来自既有价格表估算，不是上游账单。
  Provider 错误尝试仍记录请求与未知 usage；成功请求缺失 usage 时拒绝后续预算调用。
- Managed Runtime 默认 V1，包括无业务模块时的未知工具错误；Raw Runtime 的旧日志 ABI 保持兼容。
- 控制面模块 stop/unregister 使用有界排他窗口；不强称长请求期间任何控制操作都无等待。
  模块创建端口在 Host assembly 完成后可用，initialize/start 中不能同步等待创建。

验证：相关全目标回归 95 suites / 1156 passed / 0 failed / 0 ignored（含当时 10 项实例测试）；
增加 Retry-After 验收后的实例测试 11 passed / 0 failed；四 crate 全目标 Clippy -D warnings、fmt、严格架构检查通过。
日志 `apeireth-convergence-m2-regression.log`、`apeireth-convergence-m2-final.log`、
`apeireth-convergence-m2-clippy.log`，均在 `/workspace/scratch/`。
Profile 配置持久化、Memory 数据隔离及 Memory 后台 Agent 仍未交付，继续下一内部步骤。
