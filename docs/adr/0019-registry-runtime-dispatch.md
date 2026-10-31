# 0019. Registry Runtime 接线：沿用插件生命周期与唯一治理入口

```text
[Document-Meta]
Document: 0019-registry-runtime-dispatch.md
Version: 1.0
Last-Modified: 2026-10-28
Status: Accepted；本轮 owner Phase 2 授权；真实 Runtime 切片已接线，验证见交付账
Author: Codex
```

- 延续 ADR-0018，基线本地提交 `cad81196e7e11c7082d60ef7ea1e360412e58daa`。
- 不建立新 Module Registry。`PluginManager` 仍拥有插件、能力归属、依赖与生命周期；
  `CapabilityRegistry` 保留宿主动态工具兼容入口，Dispatcher 只提供联合查询与执行策略，不复制工具目录。
- 为既有 `Plugin` 增加默认 `start` 与固定 V1 Hook 接口；旧实现无须修改。
  生命周期增加初始化完成/启动中检查点，Active 仍表示 Running。旧 `start_all` 失败语义保持；
  Runtime 使用带失败清理的启动入口。停止关闭任务作用域、取消并排水，再调用 shutdown；依赖者先停。
- Hook 接收不可变快照，仅返回带内部执行 ID 的已注册自有工具请求。
  回调可并发，结果按依赖拓扑确定顺序提交；超时、取消、panic 与模块错误隔离并落会话日志。
  自动工具通过原 admission/治理/审批判断与 Dispatcher 执行；需要交互审批的自动调用明确拒绝，
  不生成隐藏审批，不递归触发工具 Hook。模型主动调用继续支持原有可恢复审批。
- 所有新自动内容用 `ToolEventV1` 记录，等调用批闭合后追加合法调用/结果对。
  相同内部执行 ID 不重复执行；冲突拒绝。未结束的模型批次中产生的结果暂存，不改在途请求。
- Dispatcher 收口 legacy invoke/invoke_frozen/invoke_in_session、参数 Schema、权限范围、超时、取消、
  结果相关性与限额。既有治理仍是唯一决策入口，内部纯状态 control 仍保留 KC 注册式语义。
  Session 工具范围作为未来实例范围端口，不能等同完整 Multi-Instance Runtime。
- 参数兼容由 LegacyToolAdapter 明确承担：V1/strict 工具完整验证；旧非 strict ABI 只放开根对象额外字段，
  保留其原有反序列化语义，冻结指纹继续绑定完整原始参数。不是按具体工具名绕过校验或审批。
- 参数验证使用 `jsonschema`（关闭默认网络/文件解析功能），拒绝无必要的自制半套 Schema 解释器。
  净额：crate ±0、内部生产边 ±0、运行期开关 ±0、外部直接依赖 +1；不触及 LOCKED。
- 既有 kernel 执行边界守卫同时覆盖抽出的 admission、Dispatcher、注册 Hook 和 SubLoop；
  不能用文件拆分绕过 KC 的注册式分派约束，不降低原守卫或增加例外。

## O-6 三阶审查

- 总体最优：优先可运行的注册→自动 Hook→工具→模型回合纵向切片；拒批量迁移成熟业务模块。
- 系统最优：权限/冻结审批留在既有 admission，回调内容走协议；拒建立不经治理的 Hook 执行后门。
- 架构最优：扩展现有 PluginManager，Dispatcher 不拥有第二份目录；拒平行 Module Manager 和事件总线。

## 兼容、失败与回滚

旧 BehaviorModule/PromptOverlay、宿主工具注册、SubLoop 和注册式 control 逐项记账，
不能将它们全部声称为新的生命周期模块。旧插件默认不新增持久 Hook/工具事实；
`Plugin::uses_tool_events_v1` 显式升级持久事件格式，声明 V1 Hook 自动升级。
所有工具仍共用 Dispatcher 的 LegacyToolAdapter；旧 ABI 返回 ToolResult，
旧会话日志保持原审计路径，V1 Runtime 额外记 ToolEventV1。此边界用于保留 KC 观察者开关逐字节不变契约。
新 V1 模块在 BeforeModelCall 严格校验派生历史；四种协议适配器也独立校验。
旧自定义测试 Provider 的 log-only 非拦截语义保留，不能用修改该断言制造统一协议已完成的假象。

V1 AfterToolCall 自动工具尚未参加既有滚动池的资源分类，所以该订阅构成模型工具派发屏障，
使用串行工具路径；旧配置并行窗口保持。Hook 回调本身仍可并发、跨会话仍可并发，
没有新建全局模型锁。恢复该场景的工具并行需要先将自动工作纳入同一资源计划。
持久化只追加可序列化事件，不改 Memory DB。新字段使用默认值，旧日志继续可读。
失败隔离不承诺回滚已经完成的副作用，取消/超时不得标为安全自动重试。
回滚优先关闭 V1 模块注册/执行，保留 ADR-0018 协议及本批新增 Session 事件的读取能力和已保存消息，
无需重写 Memory DB。不能直接删除新增事件类型后声称旧二进制可读取 V1 会话；
完整代码回退必须保留兼容 reader 或先导出会话副本，不删除事实来制造兼容。

实际接线、限制、测试及远端推送状态见
[Phase 2 交付账](../04-internal/registry-kernel-phase2-2026-10-27.md)。
