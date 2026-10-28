# Kernel Convergence — S2 ModuleInvoker instance tasks

```text
[Document-Meta]
Document: docs/04-internal/kernel-convergence-s2-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: S2 定向验证通过；整体 Production Convergence 仍为 Partial
Author: Codex
```

## 基线与交付范围

基线为 `refactor/kc1-kernel-convergence@e5cfe0c2`，fetch 成功、工作树干净，包含 S1 源码
`0aacb6d7` 与 M0 债务图 `5d0be742`。本批仅实施[债务图 D2 / S2](kernel-convergence-remaining-debt-map-2026-10-29.md)，
沿用 [S1](kernel-convergence-s1-2026-10-29.md)、ADR-0018～0022 与
[历史验收报告](kernel-convergence-final-acceptance-2026-10-28.md)，不追改历史记录。
取材为本地源码与中断补丁提示；补丁经逐项审查后复用部分测试，执行适配基于当前 S1 重新实现，验证全部重新运行。

**PRODUCTION WIRED（ModuleInvoker 路径）**：Runtime Hook 提供的 `RuntimeSubLoopSpawner::invoke`
委托 `RuntimeModuleInvoker::invoke` → 既有 `spawn_side_agent` → `AgentInstance::execute(AgentTask)` →
同一个 `Runtime::execute_outcome`。两种 invoker 都不再持有 ProviderRouter 或直接 completion 后端。
不重造 Registry、Dispatcher、Module Factory、MCP Lifecycle 或 Multi-Instance Runtime。

**IMPLEMENTED（通用 SingleShot 策略）**：`AgentExecutionMode::SingleShot` 与 Isolated 共用私有执行限制，
禁止自动注册/认知 Hook 和交互审批；只返回单个完整模型响应，不声明/派发工具、不进入下一工具轮，
也不启动压缩模型调用或上下文溢出重试。原 Router 候选 fallback 与共享限流/退避保持；每次实际尝试都受原预算约束。
Conversation / Isolated 的原工具执行语义保持。

## 语义与核算

- `ModuleInvoker` trait、ModuleInvocationRequest / Response、错误 ABI 与 InvokerLlmFactory 生产桥保持。
  system/input/model 作为规范化任务字段传递；最终 response 的 id、model、content、finish_reason、tool_calls、
  per-call NormalizedUsage、raw_metadata 与 served_by 原样返回，绝不用累计 AgentUsage 代替。
  RuntimeModuleInvoker 构造器改为显式接收 Runtime、父来源和安全上下文；桥的测试构造也走真实 Runtime。
- 无工具的响应是不可执行结果，包括重复 call id 的意外 tool_calls：保留返回值，既不验证成可执行批也不派发。
  此策略不放宽普通工具批的 schema、授权、审批、相关性或冻结规则。
- 子实例有独立 AgentId / SessionId / RequestId / TraceId 和私有历史，不向前端直播。
  注册行为绑定真实 PluginId / ModuleInstanceId，AgentTaskStarted 持久记录父 agent/session/request/trace。
  原 with_module 的未注册兼容 owner 仍是 S1 的 `Host(compat.<module>)`；不伪称其已完成 S3 生命周期迁移。
- ProviderRouter / Dispatcher 仍独占实际核算；adapter 只转交响应。child、module owner、Host 汇总可核对，
  父实例只记自身请求。成功、失败、取消、未知缓存量、缺失 usage 与估算成本继续区分，不造数、不重复扣账。
- ModuleTurnState 跨模块、借用/owned/clone handle 及 SubLoop 共用，在真实 Provider admission 才原子预留。
  预算耗尽可在创建 child 前快速拒绝；跨 turn/session 独立，递归深度限制保持。
  owner 累计预算不会因换 child 重置；未报告 usage 后的后续预算请求继续 fail-closed。
- 复用 S1 的父取消、模块世代、stop/timeout 与原生排水边界；完成和错误均 stop child，future Drop 只撤销。
  私有错误也禁止 OnError 递归回调，不关闭共享 Provider。

## 行为证据与断言迁移

保留 cognitive_module_abi、canonical_invoker_handle、canonical_organ_module 与 organ_llm_bridge_compat
的全部测试，不删除、不 ignored。治理的 session/trace 断言从旧父身份更新为真实 child 身份，
同时增强为读取持久 AgentTaskStarted 并核对其父 request/trace；并发场景逐一证明来源不串线。

cognitive_module_abi 新增真实 Runtime 场景覆盖独立 invoker 与实际注册 Hook 两条入口：
完整原始响应及重复 tool_calls、无工具声明/副作用/递归、主历史不变、单槽 Host 无重入死锁、
child/真实 owner/Host 用量与成本无重复计数、未知缓存、失败/timeout 保账、缺失 usage 拒绝和 invoker/SubLoop 共享预算。
原 Deny / RequireApproval、clone/跨 turn/并发 session、预算降级和 Organ bridge 回归继续通过。

首轮认知与 handle 回归暴露旧测试 Provider 的全零 usage 被原账本判为未报告，后续 owner 预算拒绝。
修正的是离线正常响应 fixture 的确定 usage；生产账本与断言未放宽，缺失 usage 的专门用例保留该拒绝证明。
失败日志为 `/workspace/apeireth-codex/s2-cognitive-first.log` 与 `s2-handles-first.log`。

## 本批验证账

环境：2026-10-29（Asia/Shanghai），Linux x86_64，rustc / cargo 1.97.1，PowerShell 7.5.4，Python 3.13.5。
Native jobs=2，默认 dev/test profile、debug assertions 保留。原始输出与退出码保存于
`/workspace/apeireth-codex/s2-*.log` / `s2-*.exit`；命令清单见 `s2-check-results.json`，
统计脚本 `s2-counts.py` 生成 `s2-test-counts.json`。下列为本批定向实测，重叠测试不相加，
不替代 [test-baseline](../03-reference/test-baseline.md) 的 workspace 全量口径。

| 命令 | 本批实测 passed / failed / ignored |
|---|---|
| `cargo test -p apeireth-runtime --test cognitive_module_abi --locked -j 2` | 25 / 0 / 0 |
| `cargo test -p apeireth-runtime --test canonical_subloop --locked -j 2` | 17 / 0 / 0 |
| `cargo test -p apeireth-runtime --lib --locked -j 2` | 167 / 0 / 0 |
| `cargo test -p apeireth-runtime --all-targets --locked -j 2` | 376 / 0 / 0（包含上述 runtime 目标） |
| `cargo test -p apeireth-runtime-assembly --test canonical_invoker_handle --locked -j 2` | 7 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --test canonical_organ_module --locked -j 2` | 12 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --test organ_llm_bridge_compat --locked -j 2` | 5 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --lib --locked -j 2` | 356 / 0 / 0（含真实 invoker bridge 单元测试） |
| `cargo test -p apeireth-plugin --lib --locked -j 2` | 282 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --test convergence_agents --locked -j 2` | 13 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --test convergence_memory --locked -j 2` | 9 / 0 / 0 |
| `cargo test -p apeireth-runtime-assembly --test production_cognitive_v1 --locked -j 2` | 4 / 0 / 0 |
| `cargo clippy -p apeireth-plugin -p apeireth-runtime -p apeireth-runtime-assembly --all-targets --locked -j 2 -- -D warnings` | 通过，零警告 |
| `cargo check -p apeireth-runtime-assembly --no-default-features --locked -j 2` | 通过 |
| `cargo fmt --check` / `python3 scripts/arch_guard.py --strict` | 通过；无新增豁免或基线放宽 |

后端删除检索原始输出为 `s2-unified-invoker-rg.log`：

```bash
rg -n 'complete_for_session|ProviderRouter|NormalizedRequest' crates/engine/runtime/src/canonical/{module,subloop}.rs
# 无输出，exit 1（无命中）
rg -n 'impl ModuleInvoker|RuntimeModuleInvoker::new|spawn_side_agent|\.execute\(AgentTask|\.invoke\(request\)|SingleShot' crates/engine/runtime/src/canonical/{module,subloop}.rs
```

后者记录 module.rs 的 `spawn_side_agent` / `.execute(AgentTask` / `SingleShot`，以及 subloop.rs
的 `impl ModuleInvoker` / `RuntimeModuleInvoker::new` / `.invoke(request)`。没有第二个侧模型执行后端。

## 后续与诚实账

整体 Production Convergence 仍为 Partial；S3 with_module/PromptOverlay、S4 授权 Profile 映射、
S5 桌面 managed 默认与最终 gate_full 尚未实施。Mesh/NAT/P2P 保留，PortableHost 保持 desktop-free。
没有真实外部 API、MCP 服务、Android 设备/交叉构建或桌面硬件实测；不声明 HARDWARE VALIDATED。
未引入外部依赖，Cargo.lock、workspace.version、LOCKED 哲学锚/键集/脊柱及 R11 baseline 均未触碰。

Docs Truth 按 S1 方法在源码落提交后以独立文档证据提交锚定 verified_at，避免自引用 SHA。
不提高整体能力等级或默认开关，不修改历史验收账、README 徽章或测试数字真源。

O-6：总体优先收敛实际生产 invoker，拒只修测试构造；系统复用原实例、治理与计量，拒 adapter 重复预算/成本账；
架构使用通用 SingleShot 策略，拒借普通工具轮丢弃意外 tool_calls 或新造领域侧循环。
