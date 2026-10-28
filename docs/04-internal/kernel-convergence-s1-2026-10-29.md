# Kernel Convergence — S1 SubLoop instance adapter

```text
[Document-Meta]
Document: docs/04-internal/kernel-convergence-s1-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: S1 定向验证通过；整体 Production Convergence 仍为 Partial
Author: Codex
```

## 基线与交付边界

基线为 `refactor/kc1-kernel-convergence@5d0be742`，开始时 fetch 成功、工作树干净，包含 M0 债务图。
本批只收敛[债务图 D1 / S1](kernel-convergence-remaining-debt-map-2026-10-29.md)；
沿用 ADR-0018～0022 与[历史验收报告](kernel-convergence-final-acceptance-2026-10-28.md)，不追改历史状态。
取材为本地源码及恢复脚本；恢复序列先在仓库外重建、审查，再采用并修正，验证全部重新执行。

**PRODUCTION WIRED（SubLoop 路径）**：`SubLoopSpawner::spawn` → 既有实例服务创建独立 AgentInstance →
`AgentInstance::execute(AgentTask)` → 同一个 `Runtime::execute_outcome` → 原治理 / ProviderRouter / Dispatcher。
`spawn_bounded` 的独立 Provider/round 循环和 `dispatch_subloop_tool` 已删除。
旧 `SubLoopSpawner` / `SubLoopSpec` / `SubLoopResult` / 错误 ABI 保留。

**IMPLEMENTED（通用任务端口）**：`apeireth-plugin::agents` 增加规范化消息、可选 system/model、逐任务轮数与期限、
Conversation / Isolated 策略、逐 Provider 尝试的共享 admission gate，以及完整最终 NormalizedResponse、served_by、
本任务饱和汇总 usage 和有序工具结果。原 `run(String)` / AgentReply 的累计 usage 语义保留。
未绑定新端口的兼容 AgentInstance 明确返回 NotReady，不伪造执行结果。

## 不变量与失败行为

- 子实例有独立 AgentId / SessionId / RequestId；AgentTaskStarted 持久记录真实 owner 及父 agent/session/request/trace。
  子 transcript、V1 工具对和使用事实在子会话；父用户历史不混入私有输入或中间结果，子视图不向前端事件 sink 发帧。
  新会话事件要求升级后的 reader；不承诺旧二进制可读取新增枚举，不删除旧事实。
- SubLoop 每次创建新实例。Isolated 显式禁止自动认知/注册 Hook 递归，并拒绝交互工具审批；
  completion RequireApproval 仍沿用非可恢复的明确错误，不创建隐藏 ApprovalId 或待批 continuation。
  父 Conversation 的冻结审批入口保持，原回归继续运行。
- 权限为 spec allowlist、父有效范围、Host module grant 的交集，治理与 schema 仍在原 admission/Dispatcher。
  SubLoop 适配显式在 Dispatcher 绑定父范围链接，父后续收窄立即约束子调用，不能靠最初 allowlist 恢复权限。
  子 principal 和安全上下文绑定父来源；principal 不能由模型参数选择。
- 注册行为使用真实 PluginId / ModuleInstanceId 与该世代 task scope；停止或旧目录视图不能重新开放关闭的 scope。
  裸 Runtime 的 build 也绑定同一实例服务，保留原 mutable registry owner，不把主会话冒称 managed Main Agent。
  其兼容 catalog ceiling 延续原宿主已选择的工具集合，仍需 allowlist、父范围及原治理授权；注册不替代权限判定。
- ModuleTurnState 仍跨模块、handle 与子实例共享；在 ProviderRouter 实际候选请求边界、账户检查通过后预留，
  fallback / overflow retry 同样扣减。Provider/Dispatcher 独占真实核算，适配层只聚合响应，不重复记成本。
  缓存任一轮未知则汇总未知；失败或取消前的请求事实保留。实例、owner 和 Host 原预算继续叠加。
- 同步子任务借用已获准的父 turn slot，避免单槽 Host 在父 Hook 等子任务时死锁；Provider 局部并发仍由同一服务控制。
- 父取消、模块停止、整体 timeout 传播到子 scope；显式 stop 等待 native lease 排水后才报告 SubLoop 完成。
  超过既有 stop_timeout 返回可观察错误并保留 owner，不假称资源已释放；future Drop 只撤销，不能当作已完成异步排水。
  裸 Runtime shutdown 先关闭并有界排水兼容子实例，再关闭模块；单个子实例不 shutdown 共享 Provider。
- 私有会话与实例审计记录沿用现有 Host 有界保留策略；达到 max_records 明确拒绝，不静默删除事实。

## 本批证据

环境：2026-10-29（Asia/Shanghai），Linux x86_64，rustc / cargo 1.97.1，PowerShell 7.5.4。
原生 Cargo jobs=2，默认 dev/test profile（debug assertions 保留），无真实外部 API 或硬件实测。
原始命令输出与退出码保存在 `/workspace/apeireth-codex/s1-*.log` / `s1-*.exit`，
命令批次清单为 `/workspace/apeireth-codex/s1-check-results.json`。
下表只记本批实测，不能替代 [test-baseline](../03-reference/test-baseline.md) 全量口径。

| 命令 | 本批实测 |
|---|---|
| `cargo test -p apeireth-runtime --test canonical_subloop --locked -j 2` | 17 passed / 0 failed / 0 ignored |
| `cargo test -p apeireth-runtime --lib --locked -j 2` | 167 passed / 0 failed / 0 ignored |
| `cargo test -p apeireth-runtime --all-targets --locked -j 2` | 371 passed / 0 failed / 0 ignored（含上列测试，不重复相加） |
| `cargo test -p apeireth-runtime-assembly --test convergence_agents --locked -j 2` | 13 passed / 0 failed / 0 ignored |
| `cargo test -p apeireth-runtime-assembly --test convergence_memory --locked -j 2` | 9 passed / 0 failed / 0 ignored |
| `cargo test -p apeireth-plugin --lib --locked -j 2` | 282 passed / 0 failed / 0 ignored |
| `cargo clippy -p apeireth-plugin -p apeireth-runtime --all-targets --locked -j 2 -- -D warnings` | 通过，零警告 |
| `cargo check -p apeireth-runtime-assembly --no-default-features --locked -j 2` | 通过 |
| `cargo fmt --check` / `python3 scripts/arch_guard.py --strict` | 通过 |
| Docs Truth verify / check、wiring、docs links / index | wiring / links / index 通过；Docs Truth 待源码提交后锚定 |

canonical_subloop 保留全部原安全断言；新增真实 Runtime 场景覆盖独立身份、私有历史、持久工具配对、
allowlist/父 scope/Host grant 交集、跨模块/跨 turn 预算、逐轮及 fallback 扣账、完整响应元数据与单任务 usage、
饱和与缓存未知、零轮/不收敛、completion 审批拒绝、父取消/owner stop/timeout 原生排水、共享 Provider 继续服务。
Dispatcher 单测额外钉住父范围撤销；原审批、KC、egress、并发调度与 V1 测试包含在 runtime all-targets 内。

独立循环删除证据：

```bash
rg -n 'while|dispatch_subloop_tool|complete_for_session' crates/engine/runtime/src/canonical/subloop.rs
```

预期只剩 `impl ModuleInvoker` 中的单次 `complete_for_session`（S2，保留兼容后端）。
`/workspace/apeireth-codex/s1-no-second-loop.log` 另记录提取 spawn_bounded 后的符号断言；
该函数无 Provider 调用、无 while 循环，文件中无 dispatch_subloop_tool。

## 后续与诚实账

S2 的两种 ModuleInvoker Provider 后端、S3/S5 桌面 managed 默认与原始 with_module 迁移、S4 旧记忆 Profile 映射，
以及最终 gate_full 尚未在本批实施。无 HARDWARE VALIDATED、真实 API、Android 设备或桌面 UI 验证声明。
Mesh/NAT/P2P 与 PortableHost feature 边界保持；不引入新外部依赖，不修改 Cargo.lock、workspace.version 或 LOCKED 项。

Docs Truth 使用提交 SHA 对比源码；源码落提交后再以独立文档证据提交锚定 verified_at，避免自引用 SHA 或假造零 STALE。
本批的最终文档门禁必须在锚定后无 FAIL / STALE / 生成物漂移；整体能力级别与默认开关不因 S1 改动而抬升。

迭代真账：扩展记忆回归曾发现将父范围继承泛化到所有 AgentConfig.parent 会阻断 Host 明确授权的整理工具。
已将权限交集限定在 SubLoop 适配，通用模块实例继续由独立 owner grant 授权；原记忆测试与断言未改，复跑通过。
失败日志保留为 `/workspace/apeireth-codex/s1-convergence-memory-first-failed.log`。

O-6：总体选择先归并已在生产使用的 SubLoop，拒只移动旧循环；系统复用实例、治理、账本与 task scope，拒独立权限/预算后端；
架构扩展现有通用任务端口，拒领域专用 Kernel 分支或第二 Registry/实例 Runtime。

当前环境的 hook 文件没有执行位；提交前显式以 bash 执行 pre-commit 和 commit-msg，
并使用 scripts/commit.ps1 的显式 author、secret / neutral 检查及 ArchStrict；不改全局 hooksPath。
