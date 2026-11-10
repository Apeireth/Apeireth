# 0018. Registry Kernel 迁移：先统一工具事件，再迁移执行与生命周期

- **Status**: Accepted（2026-10-27，owner 本轮 Registry-Driven Agent Kernel 任务授权；本批实施范围为工具事件协议基础设施）
- **Date**: 2026-10-27
- **Owners**: runtime / protocol
- **Supersedes**: —（延续 ADR-0002、0008、0017）
- **契约影响**: 在 `contract.toml` 登记 `ToolEventV1` 的 canonical 定义；不新增注册表、扩展执行入口或 crate 依赖。
- **例外关联**: 无。
- **预算净额**: crate ±0 / 生产依赖边 ±0 / 环境开关 ±0 / LOCKED ±0。

```text
[Document-Meta]
Document: 0018-registry-kernel-tool-events.md
Version: 1.0
Last-Modified: 2026-10-27
Status: Accepted；实施与接线状态分别记录
Author: Codex
```

## 1. 前置门禁与问题

KC 基线为远端 `refactor/kc1-kernel-convergence@82724dc32f927f014e79c4d8aee20d769b87fced`。
默认 `main@4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c` 尚未包含该分支的八个提交。
Cloud 本轮原版 `gate_full.ps1` 五段通过：351 套件 / 8907 passed / 0 failed / 51 ignored。
环境修复、历史数字区别及远端 CI 失败项见[迁移审计](../04-internal/registry-kernel-migration-2026-10-27.md)。

当前已有可独立运行的 canonical Loop、`BehaviorRegistry`、`CapabilityRegistry`、
`PluginManager`、`ExtensionCatalog` 与受限 SubLoop。不能为同一职责再建一套注册表。
但行为模块目前通过 `PromptOverlay` 注入内容；这不等于目标中的模块工具结果协议。
直接将自动结果转换成孤立的 tool 消息，也不能满足需要成对调用/结果的协议。

## 2. 本批决策

1. 在现有 `apeireth-protocol::canonical` 增加 `ToolEventV1`，复用现有 `ToolCall`、
   `ToolResult`、`ToolOutcome`、`NormalizedMessage`。事件明确区分模型调用与模块自动结果。
2. 内部执行身份与 Provider 的 call id 分离。去重使用内部执行身份；不能将上游在不同时刻复用
   的 wire id 错判为同一次执行，也不能将不同执行的结果互相配对。
3. 模块身份复用既有 `PluginId`；固定版本 Hook 名称作为协议数据登记，不引入动态脚本管线。
4. 事件构造与反序列化均检查调用/结果相关性。批次有事件数和 UTF-8 字节预算；重复事件幂等，
   同身份不同内容拒绝，失败不留下半批记录。
5. 向 Provider 投影时自动结果生成匹配的 assistant call + tool result；模块来源信息在工具结果中，
   不插入 user 文本或 system prompt。已有模型结果的内容保持原有渲染规则。
6. 向上下文追加前验证现有调用已闭合，拒绝在一个未完成的调用批中插入另一条 assistant 调用。
   这是纯数据边界，不执行工具、授予权限、建立线程、调用模型或访问存储。
7. 复用并验证现有协议适配器；若发现调用被丢弃等确定的投影缺陷，就地修复并补成对性回归。

## 3. 明确的实施边界

本批只交付 **工具事件与投影基础设施（IMPLEMENTED）**，不是整项 Kernel 重构完成声明。
以下尚未接线：Runtime 自动触发 Hook 后调用 Dispatcher、旧 `PromptOverlay` 业务模块迁移、
统一模块生命周期与卸载取消、完整 Multi-Instance Runtime、Profile Namespace 绑定、平台模块裁剪。
固定 Hook 枚举仅定义 V1 数据词汇，不能据此声称 Runtime 已发出 SessionStart/OnIdle/SessionEnd。

新批次不能绕过现有治理/审批、冻结调用、预算、完成事实观察口或会话日志。
后续 Runtime 接线必须复用上述机制；不能把投影生成的 assistant call 当成已经授权并执行的副作用。
事件字节预算作用于已经构造的记录，不替代 HTTP 输入上限、工具参数 Schema 校验或 Dispatcher 权限检查。
调用方负责按实例/会话隔离批次，提供稳定内部执行身份，并在固定请求边界应用快照。
本批没有异步注入邮箱，也不声称实现后台 Agent 的取消与成本核算。

## 4. 后续迁移顺序

1. 复用既有 Registry/Dispatcher 建立统一模块描述与生命周期控制面，保留旧 API 适配层。
2. 在固定 Hook 边界接入工具事件；通过原 Dispatcher 执行自动调用，审计 source/module/call 身份。
3. 迁移 Memory/Profile 注入与已有 Shell/Filesystem/Process 模块，保持权限和持久化行为。
4. 复用 session/quota/provider/subloop 实现独立实例管理；按实例统计，Provider 控制并发与退避。
5. 通过平台注册集合裁剪桌面能力，验证无 Shell 的 Android 核心构建；保留现有桌面组网功能。

每步必须有可运行的纵向回归后，才能将状态从 IMPLEMENTED 升为 PRODUCTION WIRED。

## 5. O-6 三阶审查

- **总体最优**：先解决自动结果的来源、相关性与 Provider 合法序列，为后续迁移提供可验证边界。
  拒绝直接把旧 overlay 批量替换为 tool 文本：缺少身份与闭合调用会造成协议错误。
- **系统最优**：协议数据放现有 protocol crate，调度、权限、生命周期继续由其既有所有者负责。
  拒绝把 Hook 调度或 Memory 策略放进 Provider：会把业务接线隐藏到传输层。
- **架构最优**：复用既有工具类型、适配器和 PluginId，新增纯数据事件族，不新增 registry/总线/crate。
  拒绝创建平行的 ToolCall/ToolResult 类型与独立 Module ID 注册表：将产生双重事实来源。

## 6. 验证与回滚

验证包括：相关性与反序列化拒绝、幂等与冲突、预算原子性、开放调用边界、不同协议的调用/结果配对、
无模块与 KC 回归、Clippy、格式、架构契约、文档检查。实际命令与结果在迁移审计中登记，不复用历史数字。

回滚本批新增事件模块、导出、适配器修复及对应测试，并删除本批 concept 登记。
没有存储迁移、既有会话重写或新默认模块；不回滚 KC 交付。
