# A3 Legacy 执行入口审计

[Document-Meta]
Document: docs/04-internal/kernel-convergence-a3-legacy-audit-2026-10-29.md
Version: 1.1 (A3 审计与文档提交收口)
Last-Modified: 2026-10-29
Status: RESIDUAL；本范围未发现独立执行后端，保留此前 CLI TCP 夹具复验缺口
Author: Codex

## 结论与边界

起点 clean `f630f03abf69186fce536575d6732265fbdf7f47`，工作分支仅为
`refactor/kc1-kernel-convergence`。本批审阅活跃 workspace 的 runtime、assembly、CLI、
Gateway 及客户端调用点，沿 SubLoop、ModuleInvoker、with_module、Overlay、V1 日志、Desktop Host
追踪实际执行链。**未发现本范围仍在生产接线中的独立 legacy provider/round 执行后端；
BYPASSES_FIXED=0，没有为了清名字而删除兼容接口，也没有修改生产 Rust 源码。**

这表示本批源码审阅和下列可运行软件回归支持 S1–S6 的单一执行链，级别为
**PRODUCTION WIRED**；不代表真实外部服务或硬件已验证，也不代表任意可信 Host 自编代码
不能调用低层公共 API。此前 A3 审计会话的 CLI 真实装配测试因沙箱 EPERM 无法启动本地 TCP 夹具，
不能把其历史通过记录替代本批通过。A3-commit 仅收口文档提交，未重跑该测试；
因此审计状态仍为 **STATUS=RESIDUAL**，不发布 CLEAN 完整验收结论。
[Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md) 仍为 Partial，
[A1](kernel-convergence-a1-cold-start-2026-10-29.md) 的 Dev 冷启动缺口没有在本批重新测量或解决。

环境日期为 2026-10-29（Asia/Shanghai；日志为 UTC），Linux x86_64、rustc/cargo 1.97.1。
完整环境见 `/tmp/apeireth-a3/a3-environment.log`。证据均在 `/tmp/apeireth-a3/`，
每个执行日志有同名 `.exit`；不以输出管道状态代替真实退出码。

## 逐项判定

| 范围 | 真实执行链与源码位置 | 保留 API 的原因 | 判定及回归证据 |
| --- | --- | --- | --- |
| SubLoop | [subloop.rs](../../crates/engine/runtime/src/canonical/subloop.rs) 的 spawn_bounded → spawn_side_agent → AgentInstance.execute(AgentTask, Isolated)；[instances/tasks.rs](../../crates/engine/runtime/src/canonical/instances/tasks.rs) 的 run_task → kernel.execute_outcome | SubLoopSpec/Result/Spawner 保留旧 ABI；私有会话、轮数/超时、父级与模块能力交集、共同请求预算和停止排水仍需该封装 | 没有自己的 provider 调用或轮询算法；canonical_subloop 覆盖 allowlist、治理拒绝、预算/回退、取消排水、持久事实和前端隔离，实跑通过 |
| ModuleInvoker | [module.rs](../../crates/engine/runtime/src/canonical/module.rs) 的 RuntimeModuleInvoker.invoke → spawn_side_agent → AgentTask(SingleShot) → 同一 Kernel；SubLoop 的 invoker 实现只委托该适配器 | 需要保留单次侧调用的完整响应、served_by、错误和 turn-scoped 克隆句柄；SingleShot 返回不透明模型结果，不将其中工具调用作为待执行工具 | 未发现旧 provider fallback；实例服务缺失则拒绝。cognitive_module_abi 与 canonical_invoker_handle 验证句柄身份、共同预算、原响应、缺失 usage 和审批拒绝，实跑通过 |
| with_module / with_behavior | [runtime.rs](../../crates/engine/runtime/src/canonical/runtime.rs) 的 with_module → with_registered_behavior；build_kernel 为每个 BehaviorModule 创建 RegisteredBehavior | 兼容构建器拼写与业务回调 trait；注册不自动授予内部能力 | 旧拼写同样经过固定 Hook、模块内部工具、receipt 与 Dispatcher；私有 principal 没有映射时跳过全局引擎。旧 builder、停止/卸载、猜名调用和 receipt 对抗回归通过 |
| Overlay | [behavior_adapter.rs](../../crates/engine/runtime/src/canonical/behavior_adapter.rs) 的 invoke_with_context 校验调用身份后，将 producer overlay 文本转为 untrusted_data/context 工具结果并清空原 overlay；[behavior_projection.rs](../../crates/engine/runtime/src/canonical/behavior_projection.rs) 只预算与真实 V1 fact 匹配的工具对 | PromptOverlay 仍是 producer 的兼容数据结构和来源标记；不是可直接提升为系统权限的执行入口 | 动态数据以工具角色进入 provider view，业务 transcript 过滤自己的模块投影；瞬态重算、注入角色、防双写和 hostile/private 数据回归通过 |
| V1 日志 | [registered_hooks.rs](../../crates/engine/runtime/src/canonical/registered_hooks.rs) 经 dispatch_one_tool 后记录 ToolEventRecorded、flush 投影；[instances/audit.rs](../../crates/engine/runtime/src/canonical/instances/audit.rs) 在显式排水边界记录中断；task reply 只读取事实结果 | 保留旧事件 reader/ABI 和现有日志；V1 是事实与投影表示，不能按名字当成旧回合后端 | 没有因读日志重新执行工具或另行扣费；registry_runtime_v1、子任务中断/重复 wire ID 和一次核算回归通过 |
| Desktop Host | [CLI lib.rs](../../crates/adapters/cli/src/lib.rs) 的生产组合根 → [desktop_host.rs](../../crates/engine/runtime-assembly/src/canonical/desktop_host.rs) build_agent_runtime → into_main_runtime_with_session_quotas；[instances/main.rs](../../crates/engine/runtime/src/canonical/instances/main.rs) MainExecution → 同一 Kernel execution_view | Runtime 是兼容 facade；AgentRuntime 是生命周期所有者；plain library Host 和 PortableHost 是显式不同装配选择 | managed_main、production_cognitive_v1、portable_host 与 Gateway handler 回归通过；CLI TCP mock 装配复验因沙箱 EPERM 失败，需后续补验 |

`/tmp/apeireth-a3/path-map.json` 和 `a3-path-map.log` 保存上述关键边的文件、行号与片段，
`source-sha256.json` 冻结被审阅源码。行号清单便于复核，不作为“任意执行路径均已被形式证明”的替代。

## S1–S6 的共同执行关系

- S1：兼容 SubLoop 的 `spawn_bounded` 已经是实例任务适配层；工具白名单先与父 scope、
  模块 Host grant 取交集。子实例具有自己的会话/状态/usage，关闭先撤销再显式排水。
- S2：ModuleInvoker 同样使用实例端口。SingleShot 与 Isolated 是同一 Kernel 的任务模式，
  保留单次不透明响应与隔离语义，不是另起一个 provider loop。请求预算在实际 Provider 准入处保留，
  fallback 尝试也不绕过共同预算。
- S3：旧模块先由 RegisteredBehavior 制作一次性 receipt，再走固定 Hook 和同一 Dispatcher。
  receipt 校验 session/principal/request/trace/module，模型不能靠猜工具名或同名 capability 伪造。
  producer Overlay 转换为带事实关联的、不可信的工具上下文。
- S4：[profile_memory.rs](../../crates/engine/runtime-assembly/src/profile_memory.rs) 的映射是 Host 显式授权选择，
  仍注册成同一行为适配器；私有 principal 不自动读取旧全局引擎。映射 lease/epoch 撤销后拒绝，
  不是第二个执行服务或默认的旧数据迁移。
- S5：CLI 的生产组合根选择 managed desktop main。CLI chat、Gateway turn/stream 与审批入口
  调用 Runtime facade 后委托同一个 MainExecution；它保留调用者 session，读取共享 Kernel 的短锁视图，
  在等待 Provider 时不持控制写锁。execution_view 清除 instance_host，避免重复进入 main facade；
  providers、sessions、governance、dispatcher 和计量仍共享。
- S6：private child 的 ToolEventV1、AgentTaskStarted 与显式排水后的 AgentTaskInterrupted 是持久事实。
  中断恢复读取事实不重执行、不重复计费；pending call 表示 checkpoint 未闭合，不证明外部副作用未发生或已回滚。

实际 Provider 准入集中在 [execute.rs](../../crates/engine/runtime/src/canonical/execute.rs) 的
`complete_for_session_gated` 调用；自动模块调用与模型工具调用同走
[dispatch_admission.rs](../../crates/engine/runtime/src/canonical/dispatch_admission.rs) 和 Dispatcher。
普通 `Runtime::execute` / `execute_streaming` 仅把同一 outcome 转换为兼容返回型。
Gateway 支持注入 plain Runtime 供显式 Host/测试使用；默认生产装配选择 managed main，
两种装配都调用同一 Kernel 算法，不把“没有 managed_main 标记”自动判断为独立 legacy 后端。

## 有意保留的边界与剩余风险

| 边界 / 风险 | 真实含义 | 维护归口与退出条件 |
| --- | --- | --- |
| 公共低层 ProviderRouter / Host API | 可信嵌入方可以直接使用公共路由库；本批没有把任意 Rust Host 代码当成沙箱。审阅到的 CLI/Gateway production providers() 调用用于模型/健康/热配置等，未见这些业务入口另起 completion loop | runtime / CLI / Gateway 维护；新增 Host/adapter 必须证明业务调用仍经 Kernel，不得把低层 API 的存在误称为已删除或不可能误用 |
| 未注册行为名的 standalone invoker | 显式 Host 构造的兼容 invoker 可使用 Host compat owner，但仍走 spawn_side_agent/AgentTask；正常行为句柄绑定真实模块 owner，停止/卸载不回落到备用 provider | runtime 维护；保留 standalone ABI 回归和模块停止后拒绝测试，不把可信 Host 显式构造当成模型可伪造权限 |
| 旧日志与新事实枚举 | 日志兼容表示保留；旧二进制不保证能无损降级读取新增事件。Drop 只撤销，不等于排水完成 | runtime / storage 维护；reader 版本升级与显式 stop/shutdown 成功后才声明完整中断事实，不虚构副作用回滚 |
| 私有 Profile 映射 | 需 Host 明确授权和目标存储，不自动把旧全局数据纳入私有 namespace | profile / assembly 维护；真实装配提供正确 mapping/grant 并维持撤销、跨 principal 拒绝回归 |
| AfterToolCall 串行屏障 | 已接受的调度限制，避免模型工具批未闭合时插入自动模块投影；不是第二执行后端 | runtime 维护；若要并行重排，应另评事件顺序和批闭合合同，不在本批删屏障 |
| SDK/TUI 与归档 | TUI 通过 HTTP Gateway 发送 chat；活跃客户端没有因旧名字获得本地 provider loop。Cargo.toml 明确排除 legacy/research，归档中保留的旧实现不是当前 workspace 接线证据 | adapters / architecture 维护；未来新入口按实际调用链审核，归档不盲删，也不把归档存在当成生产启用 |
| CLI 本轮复验缺口 | mock_vendor 在本地 TCP bind 即失败，未进入生产 bootstrap，无法声明本轮 transport/bootstrap 实测通过 | A3 接续会话；在允许 loopback listener 的环境重跑原 canonical_cli_bootstrap，保持原断言和全部测试 |

这些边界不是本批发现并修复的独立 legacy 后端。整体冷启动未达标、外部 live 服务与硬件验证
继续按既有真账保留；本批没有改接口、关功能、降门槛、增加 ignore 或削弱断言。

## 测试与命令

下列命令统一使用 `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`，
在本批实际执行；日志前缀均位于 `/tmp/apeireth-a3/`，对应同名 `.log` 与 `.exit`：

| 范围 | 命令主体 | 本轮结果 | 证据前缀 |
| --- | --- | --- | --- |
| SubLoop / Invoker / 注册与 V1 | cargo test -p apeireth-runtime --test canonical_subloop --test cognitive_module_abi --test registry_runtime_v1 --locked -j 2 | exit 0 | a3-runtime-regressions |
| managed main / 生产认知 / Portable / invoker handle / 模块工具 | cargo test -p apeireth-runtime-assembly --test managed_main --test production_cognitive_v1 --test portable_host --test canonical_invoker_handle --test canonical_module_tools --locked -j 2 | exit 0 | a3-assembly-regressions |
| CLI 真实组合根 | cargo test -p apeireth-cli --test canonical_cli_bootstrap --locked -j 2 | exit 101，环境拒绝 TCP 夹具 | a3-cli-bootstrap |
| CLI 单项隔离诊断 | cargo test -p apeireth-cli --test canonical_cli_bootstrap --locked -j 2 -- the_cli_bootstrap_boots_keyless_and_fails_explicitly_on_execute --exact --test-threads=1 | exit 101，同一 TCP bind EPERM | a3-cli-keyless |
| Gateway HTTP handler 与审批 | cargo test -p apeireth-gateway --test canonical_entry_e2e --locked -j 2 | exit 0；内存路由调用，不表示实际 listener 已启动 | a3-gateway-entry |
| runtime/assembly 静态检查 | cargo clippy -p apeireth-runtime -p apeireth-runtime-assembly --all-targets --locked -j 2 -- -D warnings | exit 0 | a3-clippy |
| 格式 | cargo fmt --check | exit 0 | a3-fmt |
| 架构 | python scripts/arch_guard.py --strict --log /tmp/apeireth-a3/a3-arch-native.log | exit 0 | a3-arch |

CLI 首个失败的原始位置为
[canonical_cli_bootstrap.rs](../../crates/adapters/cli/tests/canonical_cli_bootstrap.rs) 的 `mock_vendor`：
`tokio::net::TcpListener::bind("127.0.0.1:0")` 返回
`Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }`。
其后共享 ENV_LOCK 的 PoisonError 是连带失败；单独新进程运行仍在同一 bind 处失败，
所以不能归因于测试顺序或已有 SQLite schema，也没有为了本沙箱重写网络测试。
即使名称是 keyless，该原测试也先启动 mock listener，不能把其单项失败忽略掉。

本批未运行 full gate_full，未测外部 provider、真实外部 MCP 或硬件。
测试数量真源和 README 徽章未更新；源码审计未发现已接线独立 legacy 执行后端这一结论
不替代 A4 全量验收或未通过的 CLI 装配复验。

## 历史交付阻塞与 A3-commit 收口

此前 A3 审计会话中，`/workspace/Apeireth/.git` 挂载为 `ext4 ro`，写权限策略也仅允许仓库源码树与 `/tmp`。
该会话因此未 stage/commit/fetch/push，也未写 `/workspace/apeireth-codex/STATE.md`。
当时没有绕过挂载或网络限制，也没有伪造提交 SHA、推送回执或 A3 done 状态。
`/tmp/apeireth-a3/A3-RESULT.md` 保留这次历史阻塞的原始交接记录。

2026-10-29 的 A3-commit 会话确认 Git 与外部 STATE 可写，按用户指令仅提交本文档和
INDEX.csv 项，审阅源码保持基线不变，**BYPASSES_FIXED=0**。CLI TCP 夹具的 EPERM
是此前沙箱的环境失败，不是生产执行绕过；本次不重跑或改变测试，继续保留复验缺口。
提交使用仓库 wrapper、Codex author 与 ArchStrict，fetch 后核验远端祖先关系再普通快进推送。
实际提交 SHA、推送结果和门禁回执记录在 `/tmp/apeireth-a3/a3-commit-*`、
`/workspace/apeireth-codex/a3-commit-*` 与外部 STATE。
交付收口后 STATE 标记 A3 done、STATUS=RESIDUAL，下一步 **A1b，然后 A4**；
本次不启动 A1b/A4，不将文档收口解释为 CLI 复验或完整验收通过。
