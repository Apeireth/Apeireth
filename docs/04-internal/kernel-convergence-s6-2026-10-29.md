# Kernel Convergence — S6 Legacy V1 closure and Android cross-check evidence

```text
[Document-Meta]
Document: docs/04-internal/kernel-convergence-s6-2026-10-29.md
Version: 1.1 (Android library cross-check evidence)
Last-Modified: 2026-10-29
Status: S6 软件门禁通过；整体 Partial（冷启动目标未达）
Author: Codex
```

## 基线与边界

初始 fetch / status / log 确认为 `refactor/kc1-kernel-convergence@c303af09`，其下为 S5 源码 `2ee10a39`，工作树 clean。
依据[债务地图 D5 / S6](kernel-convergence-remaining-debt-map-2026-10-29.md)、
[S1](kernel-convergence-s1-2026-10-29.md)、[S2](kernel-convergence-s2-2026-10-29.md)、
[S3](kernel-convergence-s3-2026-10-29.md)、[S4](kernel-convergence-s4-2026-10-29.md)、
[S5](kernel-convergence-s5-2026-10-29.md)、ADR-0018～0022 与
[最终验收账](kernel-convergence-final-acceptance-2026-10-28.md)。取材仅为仓内源码、测试与文档。
源码提交为 `3f8048e8d8fca86eceba0e6e2eda89a16eed5ce1`，已按 fetch 后快进推送；随后文档提交不改 Rust / Cargo。

## 执行与事实

**PRODUCTION WIRED（兼容侧执行路径）**：SubLoop / 两种 ModuleInvoker 经既有独立 AgentInstance 进入
`Runtime::execute_outcome`；with_module / with_behavior 经认证 RegisteredBehavior 工具与 Dispatcher。
S1–S5 已删除的 Provider/round 循环、直接 callback fallback 和动态 system Overlay 没有恢复。
本批检索与真实 Runtime 证据未发现需要再迁移的第二个模块 Provider 后端，因此没有重写这些实现。
公开兼容名称与显式 Host API 保留。审计范围是旧模块侧执行与受管桌面入口，不把名称中仍含 Legacy 当作另一执行后端。

实际补齐的缺口是取消审计：future 被取消时，内存账已有事实，但子会话可能只有开始记录。
规范化任务现在在 awaited 工具派发前保存原 Session checkpoint；实例 stop、模块 stop、Host shutdown 与可观察任务取消
复用原 scope 排水，在确认排水后追加一次 `AgentTaskInterrupted`。事件沿用子 request / trace，含真实 AgentId，
并列出最后持久 checkpoint 中未闭合的 wire call id；`AgentTaskStarted` 继续关联真实模块世代与父 agent/session/request/trace。

- pending 表示最后 checkpoint 中未闭合，不证明工具未执行，也不声称副作用已回滚；不伪造成功 ToolResult 或 V1 完成事实。
- 中断上下文不复用；已成功保存的工具事实保留。native lease 排水失败仍报错并保留 owner，可重试。
  Drop 只撤销，不承诺异步审计已落盘；显式排水是可观察边界。审计存储错误 / 超时返回错误。
- 既有 `open_tool_calls` 改为按调用与结果顺序匹配；旧结果不能回答后来复用同一 wire id 的执行。
  原未闭合重复声明的兼容行为保持。内部执行 ID 仍是 V1 去重身份。
- 新枚举需升级 reader；新版保留旧日志与新事件。不重写历史，不承诺旧二进制可无损降级读取新变体。
- 私有子事件与 transcript 不向前端 sink 广播。ProviderRouter / Dispatcher 独占真实计量；
  adapter、取消与事件持久化没有另扣一次，实例 / owner / Host 只是汇总视角。

## 真实 Runtime 证据

canonical_subloop 新增同 wire id 的多 child / 多 round 事实与计量用例，以及已提交工具后阻塞 Provider 的父取消用例。
既有 timeout、父停止、模块 owner 停止的 native drain 场景增强 checkpoint、中断事件、serde reader 与重复停止幂等断言。
验证成功事实与内部执行 ID 独立、前端只见父事件、实际请求尝试与 child / owner / Host 汇总一致、取消保账。
`open_tool_calls_are_the_ones_no_result_answers` 保留原断言并增加闭合后复用场景。
不删测试、不加 ignore、不降低审批、治理、冻结载荷或资源排水断言。

## 验证口径

日期 2026-10-29（Asia/Shanghai）；Linux x86_64，rustc / cargo 1.97.1、PowerShell 7.5.4、Python 3.13.5。
命令与退出码落在 `/workspace/apeireth-codex/s6-results.jsonl`、`s6-*.log` / `.exit`。
统计脚本 `s6-counts.py` 从日志生成 `s6-test-counts.json`，重叠定向结果不相加，不替代
[test-baseline](../03-reference/test-baseline.md)。原测试名 / ignore 检查为 `s6-preserved-tests.json`；
Rust / Cargo 冻结清单为 `s6-source-manifest.json`，收口复核源码一致。

首轮全量在编译阶段 ENOSPC，rustc 与 PowerShell 日志写入失败，尚未进入测试。
外层退出码文件也因磁盘满写入失败，数字未记录；`s6-gate-full-first-disk.exit` 明记 `UNRECORDED_ENOSPC`。
原日志保留为 `s6-gate-full-first-disk.log` / `-console.log` / `-note.log`，不记作通过。
先清理闲置 incremental，再在构建退出后删除 deps 内超过 100 MiB 的产物；未 cargo clean，未删除源码、测试或依赖版本。
清单为 `s6-incremental-cleanup.jsonl` / `s6-disk-retry-cleanup.json`。
只重试一次原五段脚本；Native jobs=2，重试设 dev/test debug info=0、incremental=0，debug assertions 保留，
源码、断言、优化级别、依赖、脚本与门禁参数不变。定向测试使用默认 dev/test profile。


### 收口结果

| 定向命令（均 `--locked -j 2`） | passed / failed / ignored | 日志（同名 `.exit`） |
| --- | --- | --- |
| `cargo test -p apeireth-runtime-assembly --test managed_main` | 4 / 0 / 0 | `s6-managed.log` |
| `cargo test -p apeireth-runtime-assembly --test production_cognitive_v1` | 5 / 0 / 0 | `s6-production.log` |
| `cargo test -p apeireth-runtime-assembly --test portable_host` | 7 / 0 / 0 | `s6-portable.log` |
| `cargo test -p apeireth-runtime-assembly --test profile_memory_authorization` | 10 / 0 / 0 | `s6-profile.log` |
| `cargo test -p apeireth-cli --test canonical_cli_bootstrap` | 7 / 0 / 0 | `s6-cli.log` |
| `cargo test -p apeireth-gateway --test canonical_entry_e2e` | 10 / 0 / 0 | `s6-entry.log` |
| `cargo test -p apeireth-gateway --test multi_client_session` | 12 / 0 / 0 | `s6-clients.log` |
| `cargo test -p apeireth-runtime --test canonical_subloop` | 19 / 0 / 0 | `s6-subloop-final-verified.log` |

上述定向命令均 exit 0；最终源码覆盖以冻结五段全量为准，定向结果不与全量相加。
四 crate all-targets clippy -D warnings、cargo fmt --check、strict arch 均通过。
前端 frozen-lockfile install 后 check / test / check:assets / build 全通过；原 lockfile 没有改动。

原版 `pwsh scripts/gate_full.ps1 -Log /workspace/apeireth-codex/s6-gate-full.log` 五段全部通过，exit 0；
本批实测 365 个 test-result / 9067 passed / 0 failed / 51 ignored。
日志为 `s6-gate-full.log`，退出码为 `s6-gate-full.exit`；逐阶段统计见 `s6-test-counts.json`。
完整 Runtime / 工厂 / MCP / 旧代审批 / native drain / Profile / CLI / gateway / 桌面网络回归均在该命令范围内。

`cargo run -p apeireth-bench-harness --locked -j 2 -- cold-start`：P50 17.4237 ms，未达原 `<10 ms` 目标；
`-- idle-footprint`：P50 29.9922 MiB，按原 harness 判定达标。两者均使用 dev debug info=0 / incremental=0，
分别为原 harness 默认冷启动和独立进程待机采样口径；完整样本、操作定义与原始统计见 `s6-cold-start.log` / `s6-idle-footprint.log`。
进程 exit 0 只表示观察完成，不代表冷启动阈值通过；没有同环境 release 前后对照，不据此宣称重构导致性能回归。

Docs Truth verify --block-stale / check、wiring verify、arch_doc / docs_links / docs_index 与 strict arch 均通过；
claims 锚定上述源码，原宽泛能力等级与默认值保持，生成 STATUS / wiring，不修改 README 徽章或测试数字真源。
`git fsck --full` 通过，源码 manifest 收口一致；原生 minimal 与 memory,profile-store 可选组合的 portable_host 复验通过。

首轮新增测试 fixture 将全部 Hook 观察误当成只有 TurnStart，失败见 `s6-subloop-first.log`。
更正为对照每个真实 callback 的持久事实计量（包含空观察），原断言与治理不放宽；后续定向与全量均通过。
源码提交前索引门禁也曾检出新增笔记未覆盖，生成 INDEX 后原门禁通过，未启用跳过开关。

## 平台与验收边界

Mesh/NAT/P2P 保留；PortableHost desktop-free；S4 映射仍由 Host 显式选择且未默认启用。
2026-10-29 本批后续已安装 `aarch64-linux-android` target，沿用 Linux x86_64 / Rust、Cargo 1.97.1，
以下库交叉复验均通过。日志位于 `/workspace/apeireth-codex/`，每项有同名 `.exit`：

| 完整命令 | 结果 | 日志 |
| --- | --- | --- |
| `cargo check -p apeireth-runtime --lib --target aarch64-linux-android --locked -j 1` | PASS，exit 0 | `s6-android-runtime-check.log` |
| `cargo build -p apeireth-runtime --lib --target aarch64-linux-android --locked -j 1` | PASS，exit 0 | `s6-android-runtime-build.log` |
| `cargo check -p apeireth-runtime-assembly --no-default-features --lib --target aarch64-linux-android --locked -j 1` | PASS，exit 0 | `s6-android-assembly-check.log` |

早先 `s6-android-status.log` 的 target 未安装 / NOT RUN 记录是复验前快照，保留原样；
本批最终 Android 库结果以上表为准，历史批次的成功仍单独保留。
范围仅为 runtime 库 check/build 与 assembly 最小库 check；本次没有 assembly build 或
可选 Memory/Profile 组合的 Android 构建证据，无设备/JNI/UI 或 HARDWARE VALIDATED 声明。
本次文档收口核验现有日志及退出码，未重跑 gate_full，未修改 Rust / Cargo。
性能基准进程 exit 0 不能冒充其显示目标通过；最终验收账分别记录软件门禁、性能与平台缺口。
没有真实外部 API、外部 MCP 服务、原生桌面窗口或硬件联调实测。
LOCKED、workspace.version、Cargo.lock 与外部依赖零改动；没有新增架构豁免。

O-6：总体补实际取消事实并重跑原门禁，拒用历史全绿替代；系统复用 scope、SessionStore 与唯一计量边界，
拒取消时造成功结果或重复扣账；架构只增加会话事实与排水后的观察，拒第二执行器、Registry 或生命周期系统。

整体验收保持 **Partial**，阻塞 Complete 的现存项为冷启动性能目标。后续需在同口径完成性能验收；
冷启动 P50 **17.4237 ms** 未达 **<10 ms**；Android 库复验通过不消除此阻塞，设备验证另记。
