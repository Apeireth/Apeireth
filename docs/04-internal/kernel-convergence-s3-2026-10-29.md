# Kernel Convergence — S3 registered behavior entry convergence

```text
[Document-Meta]
Document: docs/04-internal/kernel-convergence-s3-2026-10-29.md
Version: 1.0
Last-Modified: 2026-10-29
Status: S3 定向及全量门禁通过；整体 Production Convergence 仍为 Partial
Author: Codex
```

## 基线与范围

分支 `refactor/kc1-kernel-convergence`，初始 fetch 成功、工作树 clean，HEAD 为 `031a1366`，
其前为 S2 源码 `14d87369`、S1 与 M0。只实施[债务图 D3 / S3](kernel-convergence-remaining-debt-map-2026-10-29.md)
的旧行为入口归并，并覆盖 D4 所指出的 principal guard 缺口；不实施 S4 数据映射或 S5 桌面受管主入口。
沿用 [S1](kernel-convergence-s1-2026-10-29.md)、[S2](kernel-convergence-s2-2026-10-29.md)、
ADR-0018～0022 与[历史验收报告](kernel-convergence-final-acceptance-2026-10-28.md)，不追改历史账。
取材仅本地源码与既有交付文档。

**PRODUCTION WIRED（兼容行为入口）**：`with_module` / `with_behavior` 保留公开名称，
委托 `with_registered_behavior`。build 为所有行为创建既有 RegisteredBehavior Plugin，
不再用 ID 集区分“已注册适配”和“未注册直接回调”。
`run_hook` 只能经认证 receipt → 固定 Hook → 原 admission / governance / Dispatcher → BehaviorTool
运行回调；缺适配器明确错误，停止/卸载的 scope 不会回退直接执行。
没有重造 Registry、Dispatcher、Module Factory、MCP 生命周期或 AgentRuntime。

## 授权、身份与时序

- 注册不授权：默认 DenyUnconfigured 允许普通对话但拒绝内部 capability；Host 必须明确选择授权策略。
  CLI 原生产 `register_with_policy` 保留，不在本批改桌面 bootstrap / gateway 执行面。
- 内部工具对模型隐藏。receipt 仍绑定真实 session/request/trace/module，模型猜名或假 receipt 被拒绝；
  同名工具目录冲突在 build 拒绝，不能借另一个工具的名称或 grant 运行回调。
- principal 存在时，所有兼容 builder 路径都跳过旧全局认知回调，包括召回与写回。
  这是 fail-closed，未宣称完成 Profile / Memory 授权映射；原生 Namespace Memory 另行可组合。
- Hook 注册序、Stop > Retry > Continue、全部模块先执行完再处理 directive 的规则保持。
  SessionStart 每 session 一次，TurnStart 每 turn 一次；审批恢复、工具批闭合、预算退化与 OnError 保持原语义。
- 子调用现在随兼容入口获得真实模块 owner。受管 Host 的 callback capability、父工具 scope 与模块 child grant
  仍各自独立；没有通过注册动作放宽它们。共享 turn 预算和实例账仍由 S1/S2 原边界负责。

## 唯一数据投影与保留的 Overlay 用法

主循环删除 HookEffects 的 Overlay 聚合、advance 的 pending/next/request overlays、
并行窗口的 Overlay 参数以及 `compose_provider_messages` 的 system Overlay 插槽。
动态文本由 BehaviorTool 转为 `untrusted_data`，仅经 ToolEventV1 的合法 assistant/tool 对注入。
每个结果投影一次；空观察记 ToolEventRecorded + ModuleToolEventObserved，不发空模型工具对。
显式 `TurnRequest.system` 与 Host 管理的静态 policy/system 入口保留。

`PromptOverlay` 类型及构造器、`ModuleOutcome.prompt_overlays` 保留为 ABI 数据表示，
assembly 的 Memory/Persona/Partner/Preference/Reflexion/Bio/Tone 等产生者与直接模块单测仍可使用。
这些值的角色不会进入动态 system 投影；`message()` 只暴露产生者表示。
`execute.rs` 的 Overlay helper 仅复用原 disclosure / budget / spill 算法，返回值文本写回工具载荷。

新增私有 `behavior_projection` 只重组与持久事实精确匹配的认证工具对；用户伪造的 JSON 或相似文本不构成认证。
从原始事实重算总预算、跨源封套、spill 与溢出收缩；静态核心块不截断，协议配对和来源不变，审计原文不改。
无 spill 的“宁长勿丢”、写入失败保原文、严格进展守卫与重试上限均保留。
更新[缓存组装指南](../02-guides/cache-friendly-prompt-assembly.md)反映当前表示。

## 断言迁移与新增证据

不删除测试、不增加 ignore、不放松治理/审批/安全断言。
旧测试函数名保留；“暂态 system / 不持久 Overlay”属于退休表示，明确迁移为 V1 持久工具事实。

| 测试面 | 表示迁移 / 保留与增强 |
| --- | --- |
| cognitive_module_abi Overlay / retry | 内容在工具载荷中逐字核对，注册顺序与重试反馈仍最后；已提交事实保留一次，拒绝候选不重放，用户历史无模块内容 |
| execute 组装单测 / cache_prefix | system 动态项改合法工具对；静态前缀、序列化、来源封套、防伪边界、顺序及重试/压缩断点保留 |
| context_budget / context_overflow | 解包 V1 后保留核心块、长尾、短块、实际落盘原文、预算、进展守卫与重试上限；配对独立校验 |
| canonical_subloop | 旧隐式 callback 执行改显式内部 grant；审批 fixture 只授权本身 callback，敏感工具仍需审批。饱和 usage 用例为真实模块 owner 显式授予原测试预算及父 callback scope，生产限额不改 |
| registry_runtime_v1 | 原 SessionStart 顺序断言聚焦其模块，额外 RegisteredBehavior 审计不冒称是该模块的 Hook；开放批、scope/schema/governance 原用例保留 |
| canonical_invoker_handle / canonical_organ_module | 脚本化 completion 决策与来源捕获只统计 completion；内部 callback 精确显式授权，未授予的其他工具拒绝。原 Deny/RequireApproval、共享预算与身份隔离断言保留 |
| persona_constant_injection | 逐字 Persona 数据改在不可信工具载荷断言，并新增闭合配对和无动态 system；空 Persona 的请求字节一致性保留 |
| 新 Runtime 集成 | 三个公开入口同路径、单次投影、空观察审计、真实 owner、用户/业务历史过滤、stop/unload 无 fallback、无 grant 与 principal fail-closed、模型假 receipt/同名工具拒绝；旧工具 Hook 在开放模型批中等待，闭合后按序投影一次 |
| CLI onboarding | 验收必经的既有 env 竞态修复：用量记录用例与激活用例共用 ENV_LOCK，显式清除并恢复 disable 开关；原累计回合与不写入断言均保留，无 CLI 生产变更 |
| production_cognitive_v1 | 原旧 DB/Persona/Organ、禁用/stop、模型猜名与 grant 边界用例保留；增加 with_module/with_behavior 对真实旧 Memory 的 V1 角色、停止无回退及 private principal 读写隔离 |

## 验证账

环境：2026-10-29（Asia/Shanghai），Linux x86_64，rustc / cargo 1.97.1，PowerShell 7.5.4，Python 3.13.5。
Native jobs=2，默认 dev/test profile，debug assertions 保留。
命令日志、退出码及逐项清单保存于 `/workspace/apeireth-codex/s3-*.log` / `s3-*.exit` / `s3-check-results.json`。
测试数字仅为本批实测，不替代 [test-baseline](../03-reference/test-baseline.md)；重叠套件不相加。

| 命令 | 本批实测 passed / failed / ignored |
| --- | --- |
| `cargo test -p apeireth-runtime --test cognitive_module_abi --locked -j 2` | 29 / 0 / 0 |
| `cargo test -p apeireth-runtime --test registry_runtime_v1 --locked -j 2` | 25 / 0 / 0 |
| `cargo test -p apeireth-runtime --all-targets --locked -j 2` | 380 / 0 / 0（包含上述 runtime 目标） |
| `cargo test -p apeireth-runtime-assembly --test production_cognitive_v1 --locked -j 2` | 5 / 0 / 0 |
| assembly `canonical_invoker_handle` / `canonical_organ_module` / `organ_llm_bridge_compat` / `persona_constant_injection` / `cognitive_convergence_vertical` / `convergence_agents` / `convergence_memory`（`--locked -j 2`） | 61 / 0 / 0；确切命令见 s3-check-results.json |

| `cargo test --workspace --all-targets --locked --no-fail-fast -j 2` | 9049 / 0 / 51（363 个 Rust test-result 汇总；既有 ignored 未增加） |
| `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings` | 通过，零警告 |
| `cargo check -p apeireth-runtime-assembly --no-default-features --locked -j 2` | 通过 |
| `cargo fmt --check` / `python3 scripts/arch_guard.py --strict` | 通过；无新豁免或基线放宽 |

全量与定向结果不相加；all-targets 的基准目标也由原命令实际执行，退出码为 0。
文档 links / index 检查通过；受影响 claims 在源码提交后以独立文档提交锚定，最终 Docs Truth / wiring 核验随之落账。

首轮定向失败日志保留为 `s3-cognitive-first.log`、`s3-runtime-first.log`、`s3-runtime-second.log`、
`s3-assembly-first.log`：失败原因分别是已退休的表示断言、原本隐式执行的 callback 未显式授权、
真实 owner / 父 scope 的测试预算配置，以及脚本化 completion fixture 把新 callback 准入误算成 completion。
按上表迁移表示和显式测试 Host 授权后通过，生产治理/预算边界不放宽。

首轮 workspace 构建因磁盘耗尽失败，含临时目录 ENOSPC 与链接器 Bus error；原命令和退出码保留为
`s3-workspace-first-disk.log` / `.exit`。仅清理未占用的增量缓存与同一 target 已有新版的旧测试二进制，
清单为 `s3-build-cache-cleanup.json`、`s3-obsolete-binaries-removed*.json`；源码、测试断言、运行中产物与日志保留。
清理二进制时按同名 target 的源码路径及较新文件判断“旧产物”，误把不同 feature 组合的二进制视为替代品；
这使下一轮全量的 CLI bin 测试因文件缺失未执行。该轮也不记通过。后续停止运行中清理二进制，
由 Cargo 重建缺失目标并使用原命令重跑，不更改 profile、依赖或门禁。原日志另存为 `s3-workspace-pre-final.log`。
另一轮全量捕获 CLI onboarding 既有环境竞态：用量记录测试未使用同文件 ENV_LOCK，
会被并发 escape-hatch 测试的禁用开关抑制写入。仅为相关测试补同一锁与开关恢复 guard，
不修改 CLI 生产代码或任何断言；失败日志为 `s3-workspace-env-race.log` / `.exit`。
已确证失败的两轮全量主动停止余下阶段后重跑，取消记录见 `s3-provisional-stop.json` / `s3-env-race-stop.json`；
不把部分通过数作为最终全量结果。

最后复核进一步保持审批 fixture 对所有其他 capability 的原审批要求，只精确放行本身 callback，
双模块预算 fixture 也只授予所选内部能力；修改后的测试随最终全量重跑。

后端删除检索为 `s3-unified-behavior-rg.log`：

```bash
rg -n 'module\.on_hook|prompt_overlays|pending_overlays|next_overlays|request_overlays|provider_overlays' crates/engine/runtime/src/canonical/execute.rs
# 无命中，exit 1
rg -n '\.on_hook\(' crates/engine/runtime/src/canonical/behavior_adapter.rs
# 仅 BehaviorTool 认证执行点，exit 0
```

测试名称与 ignore 增量检查见 `s3-preserved-tests.json`；冻结 Rust 源码哈希为 `s3-source-manifest.json`。
`/workspace/apeireth-codex/s3-counts.py` 可重新从原日志生成 `s3-test-counts.json`，不手算数字。

## 后续与诚实账

整体 Production Convergence 保持 Partial。S4 旧数据/Persona/Memory 显式授权映射、S5 桌面受管默认主入口、
S6 余留兼容账与最终平台验收仍未实施。本批不改 CLI/gateway 为 Managed AgentInstance；Mesh/NAT/P2P 保留，
PortableHost 保持 desktop-free。无新外部依赖，Cargo.lock / workspace.version 与 LOCKED 项不动。
无真实外部 API、硬件、Android 设备、桌面 UI 或真机联调实测，不声明 HARDWARE VALIDATED。
Docs Truth 在源码提交后单独锚定 verified_at，避免自引用 SHA；等级和默认开关不抬升。

O-6：总体优先关闭实际可调用的旧入口，拒只迁移默认 Host 的表面改名；系统复用原注册/治理/配对与预算，
拒绕过 Dispatcher 或取消预算/审批以换绿；架构保留兼容名称但统一执行权，拒平行 Registry/Loop 或 S4/S5 越界实现。

当前 hooks 文件无执行位；提交前显式以 bash 执行 pre-commit / commit-msg，再经 scripts/commit.ps1
的显式 author、secret / neutral 与 ArchStrict 提交；不使用 NoVerify、不更改全局 hooksPath。
