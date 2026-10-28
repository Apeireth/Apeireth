# Capability Modules Phase 3：审计与交付账

[Document-Meta]
Document: docs/04-internal/capability-modules-phase3-2026-10-28.md
Version: 1.0
Last-Modified: 2026-10-28
Author: Codex

Date: 2026-10-28
Status: 生产工具模块与单一目录已接线；完整五段门禁通过；不宣称全部 Legacy 已删除

## 基线

fetch 后远端 KC 与本地均为 `8fb1ad7039720eaca1ee131c0e5f295504541d96`。
三次 Phase 2 提交 `cad81196` / `d63d209f` / `8fb1ad70` 完整，工作区 clean。
main 为 `4ea9c26095e22b83dc30faaf8f7148ce7cbdb82c`，不修改 main。
同一工作区已有 Phase 2 实际全量日志：353 suites / 8949 passed / 0 failed / 51 ignored，
`logs/gate-full-registry-phase2-final2.log` 于 2026-10-28 00:33:08 UTC 完成；不冒称本轮新跑。

## 残留执行路径（迁移前）

| 入口 | 执行位置 / crate | 模型可见 | Dispatcher | 权限/取消 | 迁移方式 / 关联验证 |
|---|---|---|---|---|---|
| ProductionModules.register_into → with_capability | runtime-assembly/tool_modules → tools/{shell,filesystem,search,repo,...} | 是 | Phase 2 LegacyToolAdapter | 既有治理/冻结审批；Host 条目无模块生命周期 | 工具模块经 PluginManager 启停，保留原执行实现；生产装配/真实 Runtime 回归 |
| BuiltinToolsPlugin | tools/plugin.rs 聚合 shell/filesystem/search/repo 等 | 是 | Plugin 工具 Adapter | 整包 scope；不能独立停 Shell | 保留旧构造兼容，提供独立模块组合；无 Shell 与卸载回归 |
| ShellTool | tools/shell.rs → spawn_blocking → process::ProcessExecutor | 是 | 是 | 原生阻塞 worker 不随 future drop 自动退出 | 扩展现有监督器取消信号与原生 lease；PID/超时/输出/审批测试 |
| Repo / external / device / voice / MCP | tools 各工具 → 原生进程/桥 | 是 | 外层统一 Adapter | 各自原有策略与原生资源机制 | 保留受治理的工具内部进程实现，按工具 owner 归属；不把内部 OS API 冒称独立工具 |
| CapabilityRegistry 动态 register/unregister | runtime/capability.rs，MCP bridge 与宿主 | 是 | 是 | 原登记无独立句柄取消 | 同一目录与 scope 适配；动态 owner 限制另记，不新建 Registry |
| SubLoop | runtime/subloop.rs | 是 | Adapter，但保留独立内部治理/预算 | 白名单/治理与父 Session | 保留并审计上下文转发；不称完整独立 Agent |
| MemoryRecall / MemoryWriteback / Organ | runtime-assembly/cognitive.rs / organ_module.rs | 自动业务路径 | 旧 BehaviorModule/overlay | 原 memory namespace/governance | 本轮不重写 Memory；明确保留自动 overlay/整理路径，模型工具按同一目录注册 |
| Session SQLite / 日志 / 配置 / OS 探测 | runtime、assembly 及基础设施 | 否 | 不适用 | 内部基础设施 | 保留内部 I/O，不误迁为 LLM Tool |
| 注册式 control | runtime/plan_mode_control.rs | 是，纯状态 | 同一 admission，KC 控制语义 | Scope 检查；无 OS 副作用 | 保留 KC 原断言，不提供能力后门 |

## 实际实现与接线（PRODUCTION WIRED）

`PluginManager` 仍管理 Plugin/依赖/生命周期；`CapabilityRegistry` 成为 Runtime **唯一可执行工具目录**。
声明的 CapabilityIndex 保留为静态归属索引，不再与可执行目录联合解析或 first-wins。
Runtime 的工具目录、快照、声明、查找、Hook 归属检查与模型事件 owner 都从同一目录派生。
注册式纯状态 control 保留 KC 语义，不执行 OS 业务能力。

实际路径：

```text
LLM ToolCall → CapabilityRegistry 唯一查找 → ToolDispatcher Scope/Schema
 → 原 Governance/权限/冻结审批 → 绑定 ToolExecutionContext
 → LegacyToolAdapter（期限/并发/取消/结果校验）
 → 已启动模块的 ToolCapability → ToolEventV1 → 闭合边界成对投影 → Provider
```

先按名称取得声明以检查 Scope/Schema，不代表已经执行或授权。权限是实际副作用前的门禁。
未知工具返回 `DispatchFailure::Unregistered`，没有命令或 Shell fallback。
为保留协议 ABI，错误文本仍带 `error: ` 前缀，内部错误载荷是可反序列化的 DispatchFailure JSON。
错误、超时或输出超限均不宣称回滚已发生副作用，也不标为可安全自动重试。

### 新旧接口对照

| 旧入口 | 本轮实际入口 / 保留语义 |
|---|---|
| `CapabilityProvider::capabilities` → `with_capability` | `CapabilityProvider::tool_module` → `with_tool_module` / 普通 `with_plugin`；原工具实现不复制 |
| `ProductionModules.register_into` 逐工具注册 | 按 provider 构造 ToolModule，再通过 PluginManager 的同一生命周期启动；capabilities() 仅保留检查快照 |
| 旧 `with_tool` / `with_capability` | 显式兼容为 `host.legacy.tools` 普通 Plugin，保留旧持久日志和参数 ABI；仍进入相同目录和 Dispatcher |
| `invoke` / `invoke_frozen` / `invoke_in_session` | `invoke_with_context` 为新入口；默认适配旧 ABI，保留冻结载荷和 Session；不是另一条模型执行路线 |
| 旧流水线 `PipelinedCapability` | 原五阶段仍运行；显式转发执行上下文和策略，未为了新接口绕开 guard/post/contract |
| SubLoop 私有工具调用 | 仍受原 allowlist/预算/治理；新增同一 Dispatcher 的调用前校验、父 turn/agent/security/trace 和实际工具 owner 绑定 |
| CLI 面板拼接宿主+插件目录 | 单读统一目录；不重复展示，也不以 plugin-managed 文案冒充权限授予 |

### 公开接口与执行上下文

- `ToolModule::new(PluginId, Vec<Arc<dyn ToolCapability>>)` 复用 PluginManifest 和生命周期，默认 V1 事件。
- `RuntimeBuilder::with_tool_module(&dyn CapabilityProvider)` 是宿主组合入口；不会自动安装 Shell 或默认高权工具。
- `Runtime::{register_module, cancel_module, stop_module, unregister_module}` 沿用 Phase 2。
  stop/unregister 仍需要独占 `&mut Runtime`；共享宿主先 cancel、停止回合，再在静止边界排水/卸载。
  本批没有实现共享 Arc 的在线热插拔控制面。
- `ToolCapability::execution_policy` 声明 output_schema、required_capabilities、permission、timeout、
  concurrency、platform 和 Running 生命周期要求；Input Schema/说明/名称继续来自 declaration。
  新参数只经旧 ABI 时显式显示 legacy_arguments，不把旧参数兼容伪装为新工具 strict 模式。
- `invoke_with_context` 携带 AgentId、SessionId、父 RequestId/turn、Tool Call ID、调用者、真实模块归属、
  Scope 快照、授权上下文、截止时间、取消和 TraceId。直接宿主调用没有 turn 时显式为 None，不伪造用户回合。
- `ToolDispatcher::{set_agent, set_scope, scope}` 提供实例身份边界与可查询 Scope；Scope 只是额外收窄，
  不授予治理未允许的权限。required_capabilities 是额外 Scope 前提，不是隐式授权或二次工具调用。
- output_schema 提供时实际校验；多数旧工具尚未补全输出 Schema，不能据此声称所有工具都有完整输出合同。
  Shell 的 Desktop 限制实际执行；其他旧工具继承的 platform=Any 是兼容默认，不代表已经做过各平台/设备验证。
- 工具策略只能收紧：实际期限取工具/Runtime/调用上下文最小值；RequireApproval 不被 AllowAll 覆盖，
  仍使用原审批冻结/恢复状态机。SubLoop 不能建立隐藏审批；附加原生并发配额按工具 ID 隔离。

### 迁移清单（原工具名与业务实现保留）

以下原来均由生产装配 flatten 为 `with_capability`，现通过同一个 collect_tool_module 注册到 PluginManager。
具体构造/调用位置仍在原 `tools` / `runtime-assembly` 文件，不把底层 OS API 搬进 Kernel。

| 新实际 owner | 提供的原有能力 / 既有实现位置 |
|---|---|
| `module.tool.shell` | `shell` → tools/shell.rs + shell/execution.rs → ProcessExecutor；原 cwd/沙箱/写总闸/对象判定/冻结审批全保留 |
| `module.tool.filesystem` | `filesystem` read/list/stat → tools/filesystem.rs；共享 ObservedGate 原句柄保留 |
| `module.tool.search` | `search` → tools/search.rs；独立选装的文件检索模块 |
| `module.tool.repo` | `repo` → tools/repo.rs；固定只读 git argv，不是任意 Shell |
| `module.tool.apply_patch` | `apply_patch` → 原授权文件写策略和读前观测门禁；文件写总闸不变 |
| `module.tool.worktree_plan` / `.worktree_apply` / `.git_write` | 原受控工作树四拍与分级 git 写；保留审批和工作区边界 |
| `module.tool.process` | 原 external 的 10 个结构化程序适配器：nmap/exiftool/tshark/yara/binwalk/radare2/jadx/apktool/ghidra/openssl；保留 scope/路径/参数/时间/输出约束，不新增任意 exec 权限 |
| `module.tool.fetch` / `.content_feed` | 原出网受控抓取及订阅内容入口 |
| `module.tool.care_schedule` / `.backup_export` / `.task_board` | 原关怀日程、备份导出和任务表工具 |
| `module.tool.education` / `.self_status` / `.trait_report` | 原确定性练习、自述与性格报告 |
| `module.tool.skill_distill` / `.skill_search` | 原技能沉淀、检索；写入开关与审批不变 |
| `module.tool.voice_speak` / `.exposure_audit` | 原语音与暴露面检查，内部特有设备/服务资源限制另列 |
| `module.tool.proposal_draft` / `.image_ops` / `.doc_ops` / `.sheet_ops` | 原提案、图像、文档及表格能力 |
| `module.tool.device_sense` / `.weather` / `.device_control` / `.guard_watch` | 原设备观察/天气/受控设备执行/守护；原分级授权不变 |
| `module.tool.recon` | 原 scope 限定侦察族；不新增权限或网络目标 |
| `module.tool.host_extensions` | 由原生物装配函数直接追加的 play/ritual 等剩余工具，保留同一实际实现并有显式 owner |

原 `ExternalToolsModule` 和 `BuiltinToolsPlugin` 构造器仍兼容，显式使用时也是普通模块工具，
没有 Kernel 特权。Desktop 生产根的 external 配置现在选择 ProcessModule；原开关、工具名和网络 scope 不变。
Filesystem 族的读、搜索、授权写入仍可独立组合；省略读模块不会暗中安装写工具。
Tool Scope 限定工具入口，**不是 OS 沙箱**；显式授权 Shell 的 OS 权力仍按原沙箱/写策略执行，不能把隐藏某个工具名当成物理权限隔离。

### 原生取消、资源与并发

`ModuleTaskScope` 继续是唯一任务作用域，增加原生 lease 与取消信号。
`ProcessRequest` 从本次调用捕获取消/期限元数据，ProcessExecutor 实际检查、终止、wait/reap，并保留输出上限。
`run_native` 把这些元数据显式转交阻塞线程；Shell、Repo、Worktree Apply 和 external 公共执行辅助函数已接入。
正常退出同样清理仍存活的后台后代，避免输出管道被持有到超时；异常退出有有界清理兜底。
Unix 进程组仍按原 backend 声明 PARTIAL，不能保证回收主动脱离组的后代；没有改称强沙箱。

旧工具内部的原生辅助函数通过**按 future poll 安装/恢复**的兼容上下文取得取消与期限。
该桥只带取消、期限、scope 和资源 lease，不保存单一 Agent 或 Session 状态；重入和 panic 都恢复旧值。
新工具也能直接读取显式 ToolExecutionContext；未来可以逐个移除旧工具对这个桥的依赖。

异步 future 超时/丢弃后，原生 worker 仍持有 ModuleTaskScope lease，stop 必须等其清理完。
不可中断的同步工作按排水处理，不假称强杀 Rust 线程或回滚外部副作用。
Serial/Limited 的 permit 同样由原生 worker 保持至退出；公开上下文只持 Weak，保存诊断上下文不会锁住并发。
不同工具配额、Agent/Session 和 Provider 请求不被一个全局锁串行化；真实双会话屏障测试验证并行。

### 主 Loop 与剩余 Legacy（明确未完成）

- 主 Loop 无 Shell/Filesystem/Process 业务分支和 OS 执行。新增内容仅是通用工具批身份检查、
  调用上下文/错误观察身份转发。静态守卫新增 shell/bash/filesystem/process 字面量禁用，覆盖 admission、
  Dispatcher、SubLoop、目录与模块 facade；没有缩小既有 KC 约束。
- 旧 BehaviorModule / PromptOverlay、Retry scaffolding、MemoryRecall / Writeback / Organ 的自动业务路径仍保留。
  这些不是本批新造的工具后门，但尚未迁为固定 V1 Hook 的记忆工具事件；Memory DB 和 Profile 未重写。
- 旧非 V1 插件仍保留 KC 的 log-only 退化模型批/旧日志契约。新 V1 模块在任何工具副作用前拒绝空/重复 wire ID 的同一响应批，
  返回 Provider BadResponse、保持历史闭合且 Kernel 可继续新回合；没有靠改旧不变量断言制造通过。
  跨响应的内部幂等身份仍是 ToolEventV1 TaskId，不混同全局 wire id。
- 动态 Host/MCP 的旧 register/unregister API 仍是兼容入口，但已在同一目录、Dispatcher 和独立取消 scope 下，
  不能 shadow Plugin 工具。`unregister_and_drain` 可等待资源；原同步 unregister 只请求取消。
  这些动态项没有虚构 PluginManifest/完整 Plugin 生命周期；Model ToolEvent 的 module_id 可保持 None，避免伪造归属。
  MCP transport 仍由原 bridge 宿主管理，未迁为完整生命周期插件。下一批应先迁动态 provider owner，再废弃此兼容入口。
- 旧 SubLoop 保留私有上下文、ToolResult 列表和预算，不是独立持久 Agent 实例；工具执行已进同一 Adapter/校验，
  但其独立持久事件账、独立 Token/Cost 统计和完整 Agent 生命周期仍未实现。
- Voice/TTS（tools/voice/{speak,synth}.rs）、MCP（tools/mcp_bridge/transport.rs）、设备桥及宿主 Supervisor
  （tools/std_sub_supervisor.rs）的专有进程/设备资源仍由成熟实现管理；不把它们冒称已经改用 ProcessExecutor。
  本轮所有已注册模型入口都过统一目录/准入，但没有完成这些专有原生资源的全部可取消适配或对应硬件验证。
- Runtime 内部 Session/日志/配置/SQLite、构建命令与宿主服务启动不是模型工具，不因使用 OS API 而迁成工具。

### Host 代码示例

```rust,ignore
// provider/governance/root 来自宿主，不放入 Kernel 的平台分支。
let runtime = Runtime::builder()
    .with_plugin(provider)
    .with_governance(governance)
    .with_tool_module(&FilesystemModule::new(root.clone()))?
    .with_tool_module(&ShellModule::new(shell_config))?
    .build().await?;
// 不调用 with_tool_module(ShellModule...) 就没有 Shell。
let visible = runtime.tool_declarations_for_session(session);
let scope = runtime.tool_dispatcher().scope(session);
```

此片段是 API 组合说明（ignore，依赖宿主提供变量），不是额外通过的 doctest；可运行证据是
[capability_modules_v1.rs](../../crates/engine/runtime-assembly/tests/capability_modules_v1.rs)。
通用 Runtime 构建 Android `--lib` 不依赖 tools / runtime-assembly、Bash、Node.js 或桌面 UI。
未来 Android Host 可选择自己的 CapabilityProvider/Plugin 工具集合；本批未创建 BigFish 项目或 Android 专有代码。

## 验收证据定位与边界

| 验收项 | 实际代码/测试定位 | 交付等级 |
|---|---|---|
| 无模块正常对话、未知工具、无 fallback | `capability_modules_v1::{minimal_kernel_chats_without_any_business_module_or_tool, unregistered_shell_returns_paired_structured_error_without_fallback}` | 真实 Runtime 已验证 |
| Shell 执行、拒绝、Scope、输出上限 | 同文件 `shell_module_executes_once_through_registry_context_and_v1_projection`、`shell_permission_denial_and_scope_denial_do_not_execute`、`shell_output_is_bounded_and_reports_truncation` | 真实 Runtime 已验证 |
| 停止、卸载、替换与失效旧 handle | 同文件 `stopped_and_unregistered_shell_rejects_cached_execution_handles`、`shell_can_be_replaced_without_modifying_the_runtime_loop` | 托管模块已接线；共享 Arc 热插拔未实现 |
| PID/子孙进程清理、Caller drop、超时 | 同文件 `module_cancel_and_caller_abort_reap_shell_and_descendant_before_stop_returns`、`shell_timeout_terminates_native_tree_even_when_dispatch_deadline_wins`、`normally_completed_shell_cleans_background_descendants_and_inherited_pipes` | Linux 受控实进程；不冒称 Windows/macOS 实测 |
| 独立 Filesystem / Process、参数错误、执行失败隔离 | 同文件 `filesystem_is_independently_registered_and_schema_validated`、`process_module_executes_existing_structured_adapter_without_shell_module`、`process_failure_and_independent_permission_denial_leave_filesystem_healthy` | 已接线；外部程序使用本地无害 fixture，不是外部工具全套硬件/网络实测 |
| 目录冲突、动态兼容条目独立取消 | 同文件 `catalog_rejects_dynamic_shadowing_and_cancels_unregistered_legacy_handles`、`unloading_one_legacy_dynamic_tool_does_not_cancel_its_batch_peer` | 同一目录/准入已接线；动态 owner 完整 Plugin 生命周期未完成 |
| 上下文隔离、局部并发、原生排水期间配额 | 同文件 `agent_call_contexts_are_isolated_and_tool_concurrency_is_local`、`native_serial_permit_survives_async_timeout_until_worker_has_drained` | 已实现并验证；不是完整多实例调度/独立费用账 |
| Hook / Legacy Adapter / 生命周期原回归 | `runtime/tests/registry_runtime_v1.rs`，含启动回滚、依赖循环、Hook 确定性并发/取消、冻结审批恢复、工具结果配对 | Phase 2 原测试保留，未降低断言 |
| 新 Hook 输出身份、强制审批、流水线兼容 | `capability_modules_v1::{automatic_hook_preserves_module_caller_authorization_and_scope_snapshot, registered_approval_policy_cannot_be_bypassed_by_allow_all_governance, pipeline_preserves_context_and_execution_policy_through_legacy_stages}` | 真实 Runtime 已验证 |
| 输出 Schema、附加 Scope 前提、重复 ID | 同文件 `registered_output_schema_failure_is_a_correlated_tool_error`、`required_capabilities_are_scope_restrictions_not_implicit_grants`、`duplicate_model_call_id_is_rejected_before_any_effect_and_kernel_recovers` | 已接线；旧非 V1 批协议仍保留兼容 |
| 主 Loop 不依赖具体 OS Tool | `docs/architecture/contract.toml [kernel]` 严格静态守卫 + 上述真实 Runtime | 已接线；内部注册式纯状态 control 按 KC 特例保留 |

本批不是“全部 Capability System 已完成”的结项声明：已完成生产模型工具目录与托管 Shell/Filesystem/Process
纵向执行链；仅接口可表达但多数旧工具尚未提供完整 Output Schema；Memory 自动业务路径、动态 MCP
完整生命周期、专有设备原生清理、完整 Multi-Instance 仍未完成。默认启用与否仍取决于 Desktop 原配置，
Kernel 不自动安装任何业务工具。

## 验证记录（2026-10-28 本批实测）

- Rust/Cargo 1.97.1、PowerShell 7.6.0，Linux Cloud，jobs=2；沿用既有 CA/代理与临时本机开发库。
  为适配磁盘/内存，dev/test 仅关闭 debug symbols 与 incremental，debug assertions 保持；未改 Cargo 配置文件或门禁脚本。
  仓外 subreaper 包装器只收割已退出的孤儿子进程（Cloud PID 1 不负责回收），不向被测进程发信号、不改命令/断言/退出码。
- 新真实 Runtime 验收 24 passed；四协议 Adapter 对每个实际请求做配对校验；受控临时目录、无害命令、模拟 Provider，无外部 API。
- 原 Runtime 全目标回归：29 suites / 359 passed / 0 failed / 0 ignored，包含全部 25 项 Phase 2 Registry/Hook 测试。
  scoped 日志：`/workspace/scratch/apeireth-phase3-runtime-tests.log`；24 项新验收日志：`/workspace/scratch/apeireth-phase3-native-final-tests.log`。
- 原 Shell 六项审批/防篡改 + 两项工具回归通过，未修改断言。
- 桌面面板 7 passed，包含唯一目录/真实 owner 新回归。
- scoped Clippy -D warnings 通过；架构严格检查 violations=0，自检四个 fixture 全部通过；无新增例外、无 baseline 降级。
- Android：`cargo check -p apeireth-runtime --lib --target aarch64-linux-android --locked` 通过。
  最终源码复核日志：`/workspace/scratch/apeireth-phase3-android-final.log`。
  仅编译 Rust 核心，未做 JNI/NDK 链接、Android App 或硬件验证；Windows/macOS 原生分支未在对应 OS 实测。
- 本轮全量 `pwsh scripts/gate_full.ps1 -Log logs/gate-full-capability-phase3-final.log`：**GATE FULL PASSED**，退出码 0，2026-10-28 05:14:21 UTC 完成。
  实测 **354 suites / 8976 passed / 0 failed / 51 ignored**；主工作区与图像家族测试、两组 Clippy -D warnings、全仓 fmt 五段均通过。
  相比基线增加一个验收套件、27 个测试（24 Runtime + 2 plugin + 1 面板），ignored 与基线相同；没有删除测试。
  首次 fmt 失败记录为 `logs/gate-full-capability-phase3-2026-10-28.log`，不是通过证据。
- 所有原测试保留；未删除断言、增加 ignored、全局串行化或关闭检查。Cargo.toml/Cargo.lock 未改，未批量升级依赖。

## 原始失败与修复

1. 新 fixture 初次用错 DenyCapabilities 构造 API，编译发现后修正；不属于生产逻辑回归。
2. 新未注册工具回归首次直接按 JSON 解析渲染文本，忽略原 `error: ` 前缀而失败；修正测试解析，保留错误码和原渲染断言。
3. scoped Clippy 指出 native lease 的 match 应为 let-else；按原规则改正，没有 allow lint。
4. 严格尺寸守卫曾报 Shell/Windows child/Runtime 三项增长；提取既有实现至小文件并复跑 violations=0，未改尺寸 baseline 或新增例外。
5. 首次全量主工作区 277 suites / 6667 passed / 0 failed / 18 ignored，Clippy 通过；随后 fmt 因三个导出块排版失败，未进入图像阶段。按原格式规则修正后完整重跑，失败日志保留。分类器误报 oom，实际是 fmt=1（没有据此冒称环境阻塞）。

6. 暂存前文案检查拒绝三处禁用措辞，diff 检查发现文档 EOF 空行；按既有规则修正文案与空行，未变更检查器。

## Git 与后续

文档状态的注册模块 claim 在 Phase 3 按生产实况从 default OFF 改为 ON：仅指 Desktop 将原配置选择的
工具通过 ToolModule 装配这一机制，不是新增工具授权。Kernel 仍不自动安装业务工具，既有 Shell/网络/写入
开关不变；Phase 2 带日期的历史记录不追改。

源码提交：`d45a96e877a501a0b580b836797c10d2a36be87f`（feat(runtime): unify capability modules and governed tool execution）。
事实核验与文档生成物在后续 docs(status) 提交收口；只改证据，不再修改已通过门禁的 Rust 源码。
最后远端 fetch、推送退出码与增量 bundle 的 SHA256 以本批交付 manifest 为准；本文不把本地提交冒称已推送。
只向 refactor/kc1-kernel-convergence 安全快进推送；403 后不重试、不变更凭据、不碰 main。
Phase 4 优先迁动态 MCP owner 与专有资源 shutdown，再将 Memory 自动 recall/writeback 转为注册工具+固定 Hook，
随后补独立 Agent 统计/生命周期；不通过删除桌面工具或改 Memory DB 简化问题。

### 日志复核方法

从同一完整日志汇总 `test result` 行；suite 包括测试运行器输出的零项套件，不把 benchmark 的 Success 行另计为测试。

```python
from pathlib import Path
import re
text = Path("logs/gate-full-capability-phase3-final.log").read_text()
rows = re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", text)
print(len(rows), *(sum(int(row[i]) for row in rows) for i in range(3)))
```

完整门禁日志 SHA256：`3738a53775bf54a8ca9d78ba9a3e489d48cec7969730222efd53e8d22500ab75`。
