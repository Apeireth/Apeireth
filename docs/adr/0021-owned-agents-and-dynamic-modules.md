# 0021. 模块资源、独立 Agent 与隔离边界最终收敛

[Document-Meta]
Document: docs/adr/0021-owned-agents-and-dynamic-modules.md
Version: 1.0
Last-Modified: 2026-10-28
Status: Accepted；owner 连续实施任务授权；验收状态随证据更新
Author: Codex

## 基线与实施顺序

远端与工作区基线为 `149291e7504f9a3928dd220010aa777f07a3acae`，fetch 已核实，工作区 clean。
沿用 ADR-0018～0020，不重写 KC 历史、不修改 main。

1. 模块所有权：分层取消作用域、有界排水、动态目录原子替换与 MCP 连接清理。
2. 实例机制：复用唯一 Runtime.execute 与 SessionManager，模块拥有独立 Session/Agent、Scope、取消与预算。
3. 记忆与身份：复用既有存储/治理，固定 Hook 产生工具事件，Profile 绑定授权 Namespace。
4. 并发与核算：请求事实按实例/模块汇总，缺失 Usage 明确未知；Provider/Host/工具局部限流。
5. 兼容收敛：旧入口委托同一机制，逐项登记旧业务 Overlay/SubLoop；平台集合留在 Host。
6. 系统验证：真实 Runtime 的离线场景、Android 核心、桌面回归、原全量门禁、Git 与 bundle 证据。

先建立实例机制再接后台记忆整理，是因为整理必须拥有真正独立的会话与生命周期；不使用递归主回合冒充实例。
以上是内部连续实施清单，不代表每项已交付。最终状态必须由测试和交付账证明。

## 决策

- PluginManager 仍是模块生命周期真源；CapabilityRegistry 仍是唯一可执行工具目录。
  动态工具必须有明确 owner 与命名空间，更新撤销旧执行句柄，但不能停止无关模块或工具。
- 子作用域的活跃调用同时登记到模块作用域；父停止传播取消，子撤销只影响该代工具。
  原生任务仍持 lease 排水，超时不谎称资源已释放；停止可重试，失败必须可观察。
- AgentRuntime 是既有 Runtime 的实例管理门面，调用同一个 execute；不复制 Agentic Loop。
  模块获得绑定 owner 的创建端口，子实例默认空工具范围、有限预算，不继承父全部权限。
- 独立实例绑定独立 SessionId、AgentId、取消与消息历史；共享 Provider 不随单实例停止而 shutdown。
  Provider 请求可并行，模块/工具共享资源允许局部配额；控制面排他操作须有期限。
- Profile/Namespace 由宿主绑定并持久化配置，模型参数不能改变授权边界；记忆检索结果作为不可信工具数据。
- 自动记忆读写只用固定 V1 Hook/ToolEventV1；不在 Provider 或主 Loop 增加具体记忆分支。
  删除/更新维持既有治理、来源与版本校验，后台模型只提出受权限约束的变更。

## O-6 三阶审查

- 总体最优：以完整运行场景证明所有权与隔离；拒只添加 Trait 后将计划标为已完成。
- 系统最优：复用既有存储、审批、事件及 Loop；拒另写 Memory DB 或权限旁路，避免破坏成熟语义。
- 架构最优：在现有 crate 边界增加可组合机制；拒第二 Registry/Loop 与全局串行请求锁，避免事实分裂及并发退化。

## 兼容与验证

不批量升级依赖，不更改 LOCKED。保留旧 API 时必须委托统一准入；不能为零 Legacy 删除桌面功能。
未验证的平台、资源清理或路径必须明确记为 Partial/Not Implemented；全量失败按原日志报告并修复。
