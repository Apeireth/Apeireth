# A4 全量门禁与最终验收复核

[Document-Meta]
Document: docs/04-internal/kernel-convergence-a4-final-gates-2026-10-29.md
Version: 1.1 (approval cross-note; A4 measurements and gates frozen)
Last-Modified: 2026-10-29
Status: A4 software PASS；Kernel 当前判断见已批准定义交叉注记
Author: Codex

## 批准后交叉注记（2026-10-29）

**repository owner / user (xiao ouyang)** 于 **2026-10-29（Asia/Shanghai）** 以 **「当然」**
明确批准已有 DB、完整进程重启、正常 OS 缓存、相同 READY/session-pair 断言的 daily normal_start P50≤10 ms。
状态现为 **APPROVED**，定义原文和批准范围见 [Final Acceptance](kernel-convergence-final-acceptance-2026-10-28.md)
与 [A1b](kernel-convergence-a1b-startup-definitions-2026-10-29.md)。结合保留的 Dev normal_start P50=7.5642 ms（PASS）
和本笔记的 **GATE_FULL=0 / 软件 PASS**，当前限定判断为
**Kernel Convergence Complete (under approved startup definition)**。

A4 **OLD_COLD_P50=11.4143 ms，原 <10 ms 未达（unmet，历史）** 仍报告，
但已不是批准后日常定义的阻塞项。真实 cold_start 仍 **UNAVAILABLE（环境限制，不记 pass）**；
first_run 预算与 relative+absolute regression detection 仍是提案。没有新增测量、重新跑 A4 或清除设备/live 边界。
本次批准不解决 Phase D 的远端 CI/保护与 no-delete 冲突，不进行合并或 E 清理。
下文直至结尾均为 **A4 批准前的冻结执行账**；Partial / PROPOSED_UNAPPROVED 等表述只描述当时状态。

## A4 原正式判定（批准前历史冻结口径）

**STATUS=PARTIAL，GATE_FULL=0，OLD_COLD_P50=11.4143 ms。**
本批从 clean `fffb712ed0d21fdab9768bed4aa0093d0ca13a75` 开始，先 fetch / status / log，
确认仅在 `refactor/kc1-kernel-convergence`。所有必需软件命令均通过；原 Dev 冷启动 **<10 ms**
仍未满足，因此不能发布 Complete。原阈值、负载、初始化、断言与 profile 均未修改。
没有生产源码修复，本批为文档收口；没有模型替换或子代理委派。

A1b 保留的 `normal_start` Dev P50 **7.5642 ms**、P95 **9.0289 ms** 是已有 DB、完整进程重启、
正常 OS 缓存的独立证据，来源与样本见 [A1b](kernel-convergence-a1b-startup-definitions-2026-10-29.md)。
新日常 P50≤10 ms 定义仍为 **PROPOSED_UNAPPROVED**，本批未切换正式 gate，也未重跑完整 A1b 矩阵。
本轮所列必需软件条件已通过；若仅改用该启动定义，条件判语是
**“Complete pending approval of the new startup definition”**，原规格下仍保持 Partial。
该条件判语不是用户已批准的记录，也不代表真实冷缓存、live 服务或硬件补验已经完成。

## 环境、日志与源码冻结

日期 **2026-10-29**（Asia/Shanghai，外层日志 UTC）；Linux x86_64 / kernel 6.12.94+ / glibc 2.41，
rustc/cargo **1.97.1**，完整工具链、CPU/内存、初始 SHA 与环境见
`/tmp/apeireth-a4/environment.json`、`a4-environment.log`。
构建统一 `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0`，
保持 unoptimized 与 debug assertions。显式命令 `-j 2`；原五段脚本未写 jobs 的阶段沿用
`CARGO_BUILD_JOBS=2`，不修改原脚本或测试参数。

证据同时保存在 `/tmp/apeireth-a4/` 与 `/workspace/apeireth-codex/a4-*.log` / `.exit`，
收口归档到 `/workspace/apeireth-codex/a4-evidence/`。
`run.py` 直接记录子进程退出码；完整五段内部日志为 `a4-gate-full.log`，
外层环境/命令/阶段回执为 `a4-gate-full-driver.log`，两者有匹配的真实 `.exit`。
日志中的 exit 0 仅说明命令完成，性能阈值另外判定。

`source-before.json` 冻结全部受管 Rust/Cargo 文件；`summarize.py` 从原始日志生成
`a4-summary.json`，确认五段顺序/回执、统计与源码哈希。定向运行结果不与全量相加。
测试数量真源仍为 [test-baseline](../03-reference/test-baseline.md)，本批统计仅在 JSON/原始日志中，
未代改其历史数字、README 徽章或 A2 审计表。
整个执行期间磁盘约为 **45%→46%**，未超过 80%；无清理、cargo clean、OOM/ENOSPC 或重试。

## 指定软件检查

所有下表命令均 exit **0**，日志前缀对应同名 `.log` 和 `.exit`；测试/clippy 使用上述环境。

| 范围 | 完整命令主体 | 日志前缀 |
| --- | --- | --- |
| Managed main | cargo test -p apeireth-runtime-assembly --test managed_main --locked -j 2 | a4-managed-main |
| 生产认知 V1 | cargo test -p apeireth-runtime-assembly --test production_cognitive_v1 --locked -j 2 | a4-production-cognitive-v1 |
| PortableHost | cargo test -p apeireth-runtime-assembly --test portable_host --locked -j 2 | a4-portable-host |
| Profile 授权映射 | cargo test -p apeireth-runtime-assembly --test profile_memory_authorization --locked -j 2 | a4-profile-memory-authorization |
| CLI 组合根 | cargo test -p apeireth-cli --test canonical_cli_bootstrap --locked -j 2 | a4-cli-bootstrap |
| Gateway 入口 | cargo test -p apeireth-gateway --test canonical_entry_e2e --locked -j 2 | a4-gateway-entry |
| 多客户端会话 | cargo test -p apeireth-gateway --test multi_client_session --locked -j 2 | a4-multi-client-session |
| 四包 clippy | cargo clippy -p apeireth-runtime -p apeireth-runtime-assembly -p apeireth-cli -p apeireth-gateway --all-targets --locked -j 2 -- -D warnings | a4-clippy |
| fmt | cargo fmt --check | a4-fmt |
| strict arch | python scripts/arch_guard.py --strict --log /tmp/apeireth-a4/a4-arch-native.log | a4-arch |
| 架构生成物 | python scripts/arch_doc.py --check | a4-arch-doc |
| 文档链接 | python scripts/docs_links.py --check | a4-docs-links |
| 文档索引 | python scripts/docs_index.py --check | a4-docs-index |
| 声明逐项核验 | python scripts/docs_truth.py --verify --strict | a4-docs-verify |
| 声明生成物 | python scripts/docs_truth.py --check | a4-docs-check |

CLI 的原 TCP mock-server 夹具在本轮可启动，原断言全部保留，定向与全量均通过。
[A3](kernel-convergence-a3-legacy-audit-2026-10-29.md) 当时的 EPERM / 连带 ENV_LOCK 中毒记录仍保留；
本次闭合的是那个环境条件下未能运行的**软件复验范围**，没有为它修改生产接线或制造绕过修复。
不把 mock provider 的名称或测试名中的 live 字样当成真实模型 API 已验证。

## 原五段 gate_full

仅执行一次原脚本，完整命令：

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 pwsh scripts/gate_full.ps1 -Log /workspace/apeireth-codex/a4-gate-full.log
```

| 原阶段 | 结果 |
| --- | --- |
| main.cargo-test（workspace/all-targets/locked/no-fail-fast，原 vendored exclude） | PASS |
| main.clippy（workspace/all-targets/locked，原 vendored exclude，-D warnings） | PASS |
| main.fmt | PASS |
| image.cargo-test（contract.toml 唯一清单，原 all-targets） | PASS |
| image.clippy（同清单，-D warnings） | PASS |

脚本内部记录 **GATE FULL PASSED**；`/workspace/apeireth-codex/a4-gate-full.exit` 为 **0**。
没有以 MainOnly/ImageOnly、过滤测试、改 ignore、降低安全断言或更改门禁代替五段。
所有执行均使用同一 Rust/Cargo 源码冻结；A1 的迁移优化、A2 的恢复项、A1b 新 harness 均进入当前全量范围。
没有发生资源失败，因此也没有使用一次清理重试例外。

## 原性能门槛与保留数据

在全量构建/测试负载之前单独测量，以原命令、默认样本与同一 production assembly 复验：

```bash
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- cold-start
CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 BENCH_RAW=1 cargo run -p apeireth-bench-harness --locked -j 2 -- idle-footprint
```

| 本批 Dev 观测 | n | min | mean | P50 | max | 原目标判定 |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| legacy cold-start，ms | 30 | 10.1952 | 13.3166 | **11.4143** | 60.9009 | **未达 <10 ms** |
| idle-footprint，MiB | 5 | 30.1094 | 30.1094 | **30.1094** | 30.1094 | 满足原同数值 35 MiB |

证据为 `a4-cold-start.log` / `a4-idle-footprint.log`，RAW 保留异常样本，没有筛除尾部数值。
legacy cold-start 是同进程内、提前建立 Tokio、各样本 fresh DB 的完整组装；不含进程/Tokio/目录创建与 Runtime 销毁。
它没有 drop OS 文件缓存；旧输出中的“真冷”仍是历史新目录用语，不能当成 A1b 的冷缓存测量。
[A1](kernel-convergence-a1-cold-start-2026-10-29.md) 的 11.9248 ms、A1b 的 11.5277 ms 及完整历史数据不追溯改写。
本批没有产品性能源码改动，不依据这些相邻观测波动声称性能优化或回归。

A1b 的真正冷缓存模式仍为 **UNAVAILABLE**：当时真实 drop_caches 写入被拒绝，两档 n=0，
P50/P95 没有实测。A4 没有补测、提权、伪造驱逐或用已有 Runtime/暖缓存替换。
normal_start 的新日常定义没有批准，故即使它的保留数值低于 10 ms，也不能解除旧 gate 的正式阻塞。

## Android 与验证边界

已安装 `aarch64-linux-android` target。以下命令沿用同一环境并 `--locked -j 2`，均 exit **0**：

| 完整命令主体 | 日志前缀 |
| --- | --- |
| cargo check -p apeireth-runtime --lib --target aarch64-linux-android --locked -j 2 | a4-android-runtime-check |
| cargo build -p apeireth-runtime --lib --target aarch64-linux-android --locked -j 2 | a4-android-runtime-build |
| cargo check -p apeireth-runtime-assembly --no-default-features --lib --target aarch64-linux-android --locked -j 2 | a4-android-assembly-check |

`a4-android-status.log` 记录无 adb、emulator、cargo-ndk 与 SDK/NDK 环境变量。
范围仅为 runtime 库 check/build、assembly 最小库 check；没有 assembly build、Android 可选 SQLite 组合、
APK、设备/JNI/UI 或 **HARDWARE VALIDATED**。没有真实外部 LLM/MCP、原生桌面窗口或硬件联调。
A4 未重跑前端检查/构建，S6 的前端记录仅保留为历史证据，不列为本批执行。

## 阻塞、声明与交付

| 项目 | 本批结论 / 退出条件 |
| --- | --- |
| 正式 Complete 阻塞 | 原 Dev legacy cold-start P50≥10 ms；需同口径实际达标，或用户明确批准新验收定义后另行记录规则变更 |
| 新日常定义 | PROPOSED_UNAPPROVED；不可在当前 gate 中静默切换，条件判语不等于正式 Complete |
| 真实 cold_start | UNAVAILABLE；需允许真实 cache drop 的 runner 提供同 workload 的进程重启样本 |
| A3 CLI 夹具 | 历史为 RESIDUAL；A4 原测试定向和全量通过，软件复验缺口已补齐；外部服务仍未实测 |
| Android / live / 硬件 | 库编译通过；其他范围继续按上述未实测边界保留 |

没有源码变化触发 claim staleness；严格声明核验与生成物检查通过，无须虚构更新 verified_at 或生成 STATUS 变化。
文档交付包括本笔记、Final Acceptance A4 addendum、CHANGELOG 与 INDEX 项；历史记录保留。
按 Codex author / commit.ps1 / ArchStrict 提交，fetch、祖先核验后普通快进推送；实际 SHA 和推送回执在外部 STATE。
仓库 hook 文件未设可执行位，收口另用 Bash 显式运行两个 hook 的全部检查，不修改 hook 配置或绕过门禁。
更新 STATE 为 **A4 done / STATUS=PARTIAL / next=B**；本批不启动 B，不改 main，不 force-push。
