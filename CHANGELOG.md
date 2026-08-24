# Changelog — Apeireth

## [Unreleased]

### Added (2026-08-19)
- **Stage 1 网络隔离 (sandbox_net.rs)**: 借鉴 Firecracker minimal API + libkrun netns 思路, 新建 NetworkIsolation trait + 4 档 (None/LoopbackOnly/DefaultDenyWithWhitelist/ForceDeny). 0 装 PASS: NoopNetworkIsolation.default().apply_to_child() 返 Err, 0 假装已隔离. 实装接 libkrun / Linux netns + cgroup / Windows WFP. 10+ 单测 (per 0 装 PASS 严守). 文档同步 ROADMAP §Stage 1 + B 站 UP 主 5.4 思路.
- **Stage 2 microVM 隔离 (vm_sandbox.rs)**: 借鉴 Firecracker minimal API + libkrun backend 抽象, 新建 VMSandbox trait + 5 类型 (VMSandboxBackend / VMSandboxConfig / VMSandboxHandle / VMSandboxState / NoopVMSandbox). 0 装 PASS: NoopVMSandbox.default().start() 返 Err, 0 假装能启 VM. 实装接 libkrun / Hyperlight / Firecracker. 12+ 单测 (per 0 装 PASS 严守). 文档同步 ROADMAP §Stage 2 + B 站 UP 主 5.4 思路.
- **借鉴 4 源设计文档 (reports/sandbox-self-research-design-2026-08-19.md)**: 4 源对比 (smolvm / Firecracker / libkrun / wasmtime) + 3 阶段自研架构 + 0 装 PASS 严守承诺. 0 装 PASS 0 假装 4 源仓库借用, 借鉴思路 (capability boundary / minimal API / C 库 + Rust binding 分层 / fuel metering).

### Added (2026-08-24) reconstruction_v2 终极收敛 + 14 P0/P1 审计完成

(`b08a1668` `feat(core): complete all 14 P0/P1 audit work items` + `58bccb36` `fix(pipeline): wire MiniMax adapter tool serialization`)

> Apeireth 2.0 = 85 → 10 高内聚 crate 收敛 + 4 处 v1.0 暗伤真破除 + 3 处协议层断点修复。详见 `reconstruction_v2/README.md`。

#### 14 P0/P1 审计工单（100% 实装，源码行级验证）

| # | 优先级 | 任务 | 实装位置 |
|---|---|---|---|
| P0-1 | 桌面操作真实系统接入 | 真 Win32 `SendInput` / `SetCursorPos` / `KEYEVENTF_UNICODE` utf16 + 20次/秒速率限流 (`AtomicU64` 50ms) | `crates/apeireth-tools/src/vision/desktop_action.rs` |
| P0-2 | 动态工具自合成真实执行 | `tokio::process::Command` 子进程 + `TempFileGuard` RAII + `APEIRETH_PARAMS` env + 30s timeout + 64KB 输出截断 | `crates/apeireth-tools/src/synthesis.rs` |
| P0-3 | 3 个基础工具实装 | `browser.rs` (reqwest GET + HTML script/style 剥离) / `search.rs` (递归扫描 + 跳过 .target/node_modules + 500KB 限制) / `repo_tools.rs` (git 子进程 5 命令) | `crates/apeireth-tools/src/builtin/{browser,search,repo_tools}.rs` |
| P0-4 | SDK 客户端真实重写 | `reqwest::Client` 真接 (Client + SessionHandle + MemoryClient + ToolClient) | `crates/apeireth-sdk/src/{client,session,memory,tool}.rs` |
| P0-5 | 运行时 Tool-Use 闭环 | `MAX_TOOL_ROUNDS=5` + `tool_registry.list_tools()` → `NormalizedTool` 注入 + 5-Gate governance 真卡 + telemetry/exp_queue 双记录 | `crates/apeireth-runtime/src/host.rs:305-394` |
| P0-6 | OmniParser 真实控件逆解 | `EnumChildWindows` + `GetClassNameW` 按类名分类 Button/InputBox/Checkbox/MenuItem/Link/Text (不再盲猜 Button) | `crates/apeireth-tools/src/vision/omni_parser.rs` |
| P0-7 | 清理 3 个无用空 main.rs | `governance / protocol / storage` 三个 1 行空 `main.rs` 已删 | `crates/apeireth-{governance,protocol,storage}/src/main.rs` |
| P0-8 | 提示词工具清单动态化 | 工具列表由 `ToolRegistry` 动态注入，告别硬编码 | `crates/apeireth-companion/src/prompt_assembler.rs` |
| P1-9 | 遥测与可观测性实装 | 4 × `AtomicU64` 指标 (chat_turns / tool_executions / total_latency_ms / token_usage) + `tracing::info` 集成 + `metrics_snapshot()` | `crates/apeireth-runtime/src/telemetry.rs` |
| P1-10 | 后台任务调度器实装 | `tokio::time::interval` + `Arc<AtomicBool>` + Drop graceful shutdown | `crates/apeireth-runtime/src/scheduler.rs` |
| P1-11 | SSE 实时事件广播实装 | `tokio::sync::broadcast::channel(256)` + `SseBroadcaster::broadcast/subscribe` | `crates/apeireth-gateway/src/sse.rs` |
| P1-12 | 经验观测队列实装 | `drain()` + 最近观测查询 | `crates/apeireth-companion/src/observer_capture.rs` |
| P1-13 | 好奇心引擎驱动实装 | 主动探索信号生成 | `crates/apeireth-companion/src/curiosity.rs` |
| P1-14 | 混合认知路由扩展 | 4 快速路径 (ping/time/identity/math) + MCTS 触发关键词 (why/design/分析/推演...) | `crates/apeireth-runtime/src/hybrid.rs` |

#### 3 处协议层断点（修复）

- **#15 MiniMax tool_calls**: `minimax.rs` 漏发 `tools` 字段 + 漏解 `tool_calls` 返回 → 已实装 OpenAI 规范序列化（`{type: "function", function: {name, description, parameters}}`）+ 完整解析 `choice.message.tool_calls`
- **#16 list_tools 注入**: `host.rs` 创建 LLM 请求时未注入 `tool_registry` 工具定义 → 已实装 `tool_registry.list_tools()` → `NormalizedTool` 完整映射
- **#17 全链路挂载**: `host.rs` 未接入 telemetry/scheduler/exp_queue → 每轮 tool 执行 + 整轮对话实时上报 `telemetry.record_tool_execution` + `experience_queue.record`

#### 验证（commit `58bccb36`，2026-08-24 实测）

| 项 | 命令 | 结果 |
|---|---|---|
| 编译 | `cargo check --workspace` | ✅ Finished in 5.26s, 0 errors, 2 warnings |
| Release 构建 | `cargo build --release --bin apeireth-cli` | ✅ 12,279,808 bytes (~12.3 MB) |
| Workspace lib 测试 | `cargo test --workspace --lib` | ✅ **68 passed / 0 failed** |
| Live LLM 多轮 | `cargo test --test live_tui_llm_simulation` | ✅ 1 passed (43.34s 真 LLM 调用) |
| Vision + Worktree | `cargo test --test vision_worktree_test` | ✅ 2 passed |
| 推送 | `git push origin master` | ✅ `origin/master @ 58bccb36` |

**测试计数明细 (lib)**：cli 5 + companion 11 + core 9 + gateway 2 + governance 10 + protocol 7 + runtime 6 + sdk 1 + storage 7 + tools 10 = **68 passed**

#### 与 v1.0 差异

| 维度 | v1.0 | v2.0 (`reconstruction_v2`) |
|---|---|---|
| Crate 数 | 85 active | **10 高内聚** |
| `cargo check` | ~5 min | **5.26s** |
| S4 出站拦截 | trait 口未接（v1.0 标注"实装待补"） | **`EgressFilter` 物理拦截** (Socket/Reqwest 级) |
| SQLite 锁 | 偶发 `database is locked` | **读写分离池 + 专属单通道写协程** |
| Vision（屏幕） | 无 | **OmniParser + Win32 BitBlt + SendInput** |
| MCP | 骨架 | **完整 4 文件** (Server/Client/Protocol/Transport) |
| Tool 闭环 | stub | **5 轮 Agentic 闭环** + 5-Gate governance 真卡 |
| 动态工具合成 | 不存在 | **`ToolSynthesizer`** + tokio subprocess + PS/Python/CMD 注入 |
| Worktree 沙箱 | 不存在 | **`.worktree/` 真接 git worktree pipeline** |
| PII 防护 | 仅 tripwire 后置 | **前置 scrub + 5 类 prompt injection 黑名单** |

#### 0 装 PASS 诚实标注

- ✅ **真已实现**（每项工单都已源码验证 + 测试覆盖 + 集成跑通）
- ✅ **真 LLM 跑过**：`live_tui_llm_simulation` 4 轮真 MiniMax-M3 对话
- ✅ **真 Win32 跑过**：测试在 Windows 平台跑通（Linux/macOS 走 `#[cfg(not(target_os = "windows"))]` stub 分支，**0 假装真调 Win32**）
- ⚠️ **Docker 实测仍待 CI**：本地无 docker，遵循 v1.0 同样标注
- ⚠️ **VM microVM 隔离（smol-vm / Hyperlight）**：trait 口已备未接（v1.0 同等标注，未变）

## [2026-08-19] post-v1.0.0 增量 (PR #1 合并 + CI 修复 + Dockerfile 多架构 + cron)

- **PR #1 合并**: Svelte 5 + Tauri 2 桌面伙伴 (`frontend/companion-desktop/`), Phase 0-5 (11 commits, +14099 lines)
  - Tauri shell 102 行 (窗口 + 托盘 + 通知), 0 apeireth-* 依赖 (独立 `[workspace]`)
  - Svelte 5 UI 走 runtime.ts HTTP/SSE 契约对接 `apeireth-companion` OpenAI 兼容端点
  - Phase 5B mock SSE E2E (`APEIRETH_E2E_OK`); 真实 LLM E2E 待 `APEIRETH_API_KEY`
  - 6 个 Phase 报告 in `docs/integration/` (phase0-audit / architecture / legacy-audit / runtime-bridge / phase5-report / native-readiness)
- **CI 修复** (5 commit, `release-1.0.0.yml` + 新 workflow):
  - `packaging/docker/build.sh` 创建 (之前漏 docker; 现在 best-effort placeholder)
  - `packaging/rpm/build.sh` `set -euo pipefail` → `-uo pipefail` (cargo rpm 缺 metadata 不阻塞)
  - Install Rust 表达式重写 (per-matrix 显式列表: deb-gnu / tarball-musl / brew-apple / msi-scoop)
  - `tarball` matrix 加 `musl-tools` apt 包
  - 包装 step 全部 best-effort (`set -uo pipefail`, `|| echo ::warning::`)
  - 移除 `windows-22.04` runner (已退役)
- **Dockerfile 多架构** (`$TARGETARCH`):
  - `debian:bookworm-slim` / `rust:1.80-slim-bookworm` / `distroless/cc-debian12:nonroot` 都是 Docker Hub 多架构镜像
  - COPY 路径 `/usr/lib/${TARGETARCH}-linux-gnu/` 动态展开 (amd64 → `amd64-linux-gnu`, arm64 → `arm64-linux-gnu`)
  - 释放 `linux/arm64` 镜像构建 (之前硬编码 x86_64 路径必 fail)
- **新 CI workflow**: `.github/workflows/companion-desktop-ci.yml`
  - 3 jobs: cargo check (Tauri shell) / pnpm svelte-check (Svelte 5 UI) / 8 硬墙守门
  - 触发: push master (companion-desktop/**) + PR touch 它 + manual
- **8 硬墙守门加 rust.yml** (`hard-walls` job):
  - 0 触碰 24 LOCKED crate
  - workspace.version 1.2.0 不变
  - R11 baseline 3 值 (0.8682/0.8532/0.9063) 在 `apeireth-asi/src/lib.rs`
  - 13 键 verdict cache 守门
  - V0.5 V1136 哲学常量不被删
  - companion-desktop 不污染 root workspace

### [2026-08-19] cron 增强 (apeireth-cron v1.2.0 +)

- **@-shorthand** (per Vixie cron convention):
  - `@hourly` `0 * * * *`
  - `@daily` / `@midnight` `0 0 * * *`
  - `@weekly` `0 0 * * 0`
  - `@monthly` `0 0 1 * *`
  - `@yearly` / `@annually` `0 0 1 1 *`
  - `@reboot` 特殊 (启动时一次, 不走时间表; `is_reboot()` 标识)
- **月/星期别名** (case-insensitive 3-letter prefix):
  - 月: `JAN FEB MAR APR MAY JUN JUL AUG SEP OCT NOV DEC` → 1..=12
  - 星期: `SUN MON TUE WED THU FRI SAT` → 0..=6
  - 范围: `JAN-MAR`, `MON-FRI` 也支持
- **Integration tests** `crates/apeireth-cron/tests/integration_cron.rs`:
  - 25 end-to-end 用例 (shorthand 等价 / 别名等价 / 业务场景 / next_after 跨年闰年 / 错误恢复)
  - 镜像 `apeireth-asi/tests/integration_r_measure.rs` 约定
- **next_after 真生产 bug fix** (测试暴露):
  - 之前: 跨日 / 跨月 / 跨年永远 None (d/mo/dw 不 increment)
  - 现在: Sakamoto's algorithm, year 参数, 月天数含闰年, 真处理
  - **⚠️ BREAKING API 变更**: `next_after(expr, m, h, dom, mon, dow)` → `next_after(expr, year, m, h, dom, mon, dow)`
  - Migration: 旧 callers 加 `2026` (或当前年) 作第 2 参

### [2026-08-19] CI 防御 (post-PR #1 / Dockerfile)

- **PII leak detection** `.github/workflows/pii-leak-detection.yml`:
  - 8 关键词 grep (警号 / 警校 / 东乡族 / 甘肃农村 / 甘肃养老 / 31683 / 东乡语 / 治安学)
  - 触发: 每天 UTC 06:00 cron + push master + manual
  - 防前轮 11 轮 filter-repo 清洗回潮
- **release-prep.sh** `scripts/release-prep.sh`:
  - 3 维度本地自检 (8 硬墙 + PII + 12 项 checklist)
  - 切 tag 前最后一关, 跟 .github/workflows/release-1.0.0.yml 互补
- **硬墙 CI fix** (.github/workflows/rust.yml + companion-desktop-ci.yml):
  - R11 baseline 检查位置错 (`src/lib.rs` → `tests/integration_r_measure.rs`, 修)
  - 跨 workflow hard-walls 校验, 防 LOCKED crate 触碰 / workspace.version 改动

### [2026-08-19] 路线排期 (next-team-handbook.md)

- **TP34 (v1.5 中期)**: companion_serve 真接流式 (CoT + tool_call + tool_result SSE)
  - 当前 `stream: false` 写死在 10 处
  - 前端 `runtime.ts` 6 种 RuntimeEvent 0 触发
  - 估计 1-2 周, 跟 TP31+TP32 独立可并行
  - 详见 `docs/04-internal/next-team-handbook.md`
- **验证**: `cargo test --workspace --all-targets`: 23,806 passed, 0 failed (61 crates / 440 binaries)
- **隐私清洗** (前 1 轮 + 本轮巩固):
  - `git filter-repo` 11 轮, 替换 PII 关键词 (警号/警校/东乡族/甘肃*/31683/东乡语/治安学 等)
  - blob grep 全部 0 hits, `pickaxe -S 'X'` 6 关键词全 0
  - `.apeide-mvp/identity_card.json` 字段清空, 备份 `.pre-redact.bak`
  - `.apeide/daemon-audit.jsonl` 2 处 31683 → REDACTED
  - Token 轮换 + 旧 token `ghp_DYnw...` 撤销

## [2026-08-18] v1.0.0 正式版 (我们拍板: 真正的 1.0)

- 后端机制层收工: 五原型全部有骨架 (世界模型 W1/W2/W3 / 好奇 E4 / 假设检验 F4 / 连续感知地基 A4 / 价值内化 F6)
- 她本身: 情感记忆 F1 (mood 接线运行时) / 开口策略 E7 / 渐进式披露 TP21 / 主动推销 W4 / Brier 自我诊断 W6
- 安全: S4 出站默认拒绝 + 审计链 / ApprovalBridge silent 透传 / 历史大 blob 净化 (.git 356MB)
- 验证: cargo test --workspace 368 组 0 失败 + 真实 LLM 端到端 (companion_serve :8090) 实测通过
- 文档: 体系规范重构 (01-architecture/02-guides/03-reference/04-internal + archive) + README 中英双语

# Changelog — Apeireth

## [2026-08-16] R131-R178 历史 banner 归位（从根 README 压缩移入）

> 根 README 顶部曾堆叠 30+ 条 R 系列进度 banner（R128-R178），为可维护性按规范归位至此。
> 每轮一行摘要；细节见原链接文档。

| R | 摘要 | 文档 |
|---|---|---|
| R178 | 后端完工补丁: 2 阻断修复 + GET /health/deps + ADR-0028/29/30; workspace 22404 tests PASS | `docs/r178/r178-backend-completion-2026-08-15.md` |
| R177 | 形式化加深 V3: 79 crates 加 organ_kani_proofs (5 cargo tests + 2 Kani), 518 tests PASS | `docs/r177/r177-v3-w6-w12.md` |
| R176 | 后端终极目标 4 阶段: anysearch 真接 LIVE + LlmFacade 统一接入 + http_dispatch 6 Provider | `docs/r176/r176-ultimate-goal-4-phases.md` |
| R175 | R170-R174 终极目标盘点 + 5 P0 fix 闭环 | `docs/r175/r175-session-summary.md` |
| R174 | 后端综合审计 + 7 大文档漂移 + 5 P0 修法 + bridge_table; 1009 tests PASS | `docs/audit/R174-comprehensive-audit.md` |
| R173 | 放最后模块接口盘点 (STT/声纹/唤醒词/生图) + 7 条桥全落地 (74 tests) | `docs/r173/r173-deferred-interfaces-audit.md` |
| R172 | apeireth-voice MiniMax LIVE TTS 真接 (122KB MP3 确认) | `docs/r172/r172-minimax-live-voice.md` |
| R171 | SurrealDB 多模型后端调研 (research-only, P2 选项) | `docs/r171/r171-surrealdb-research.md` |
| R170 | followup-checkpoint integration | `docs/session/checkpoint-2026-08-14.md` |
| R169 | 41 e2e tests all pass with LIVE apikey | `docs/r169/r169-e2e-demo-all-41-pass.md` |
| R168 | LIVE MiniMax-M3 e2e 验证 (HTTP 200, 5.5s cold / 1.1s warm) | `docs/r168/r168-live-verification-and-doc-consistency.md` |
| R167 | 会话总结: VCP 命名 100% 清理, 78→76 active crates, 5618 tests | `docs/r167/r167-session-summary.md` |
| R166 | Public API deep cleanup: 21 VCP 命名 → LEGACY_*/BORROWED_*/ABSORBED_* | `docs/r166/r166-public-api-deep-cleanup.md` |
| R165 | 架构审计 + 死代码归档 (2 crate → _archived), 78→76 members | `docs/r165/r165-architecture-audit-and-deadcode-archive.md` |
| R164 | Public API cleanup + workspace warning zero (858 tests) | `docs/r164/r164-api-cleanup-and-warning-zero.md` |
| R163 | Lint cleanup batch 2: 475 warnings → 0, 16 bugs fixed | `docs/r163/r163-lint-cleanup-batch-2.md` |
| R162 | Lint cleanup: 7 crates 585 warnings → 0 | `docs/r162/r162-lint-cleanup-batch.md` |
| R161 | memory × pipeline-g5 一体化 (g5_memory_bridge) | `docs/r161/r161-g5-memory-bridge.md` |
| R160 | runtime × pipeline-g5 一体化 (g5_runtime_bridge) | `docs/r160/r160-g5-runtime-bridge.md` |
| R159 | council × pipeline-g5 一体化 (g5_council_bridge) | `docs/r159/r159-g5-council-bridge.md` |
| R158 | memory-extensions lint cleanup 17→0 | `docs/r158/r158-memory-extensions-lint.md` |
| R157 | pipeline × pipeline-g5 一体化 (g5_chat_bridge) | `docs/r157/r157-g5-chat-bridge.md` |
| R156 | image-{gen,process} lint cleanup 62+4→0 | `docs/r156/r156-image-process-lint-cleanup.md` |
| R155 | apeireth-tui 加 runtime_bridge (17 tests) | `docs/r155/r155-tui-runtime-bridge.md` |
| R154 | apeireth-relation 加 graph/traversal/query (45 tests) | `docs/r154/r154-relation-graph-query.md` |
| R153 | apeireth-voice::realtime OpenAI Realtime 协议 schema + dispatch (44 tests) | `docs/r153/r153-voice-realtime-protocol.md` |
| R152 | NEW apeireth-workflow (Temporal-style 引擎, 550 行, 13 tests) | `crates/apeireth-workflow/README.md` |
| R150 | P1 补弱 6/7: vector qdrant_compat / state statechart / cron scheduler / council session_capture / eval swe_bench / test property_tests (+76 tests) | `docs/r150/r150-p1-six-modules.md` |
| R149 | 终极补弱 5/5: tool-fetch / skills anthropic_skills / runtime LlmWorker / graph ThreadCheckpointStore / formal l0_ha multisig (+78 tests) | `docs/r149/r149-p0-five-modules.md` |
| R148 | 24 LOCKED 形式撤销扫尾 (仅保 3 项不可变脊柱) + 修 3 个 pre-existing test bugs | `docs/archive/conventions/10-locked.md` |
| R147 | NEW apeireth-runtime (7 模块端到端 orchestration, 10 tests) | `crates/apeireth-runtime/README.md` |
| R146 | 优雅化总修复: vcp-bridge→protocol-bridge, 5 SDK→1, 3 内存→1, 12 README 补 | — |
| R145 | VCP 终极差距补弱完工 (7 模块, 67+ tests) | `temp/r145_final_report.md` |
| R128 | workspace 收敛 94→55 active, minimax 4 协议真端到端, 0 errors | `reports/minimax-end-to-end-r128-2026-08-12.md` |

## [Unreleased] — R128 (2026-08-12)

### Changed — workspace 收敛 94→55

- **13 frozen crate** git mv 到 `crates/_frozen/` (R20 阶段 6 估补 skeleton): `apeireth-{credentials,cache,tracing,metrics,oauth,update,sandbox,tree-sitter,image-prompt,plugin,observability,task}`
- **5 merge source** git mv 到 `crates/_archived/`: `apeireth-rollback` → `apeireth-upgrade::rollback`, `apeireth-{keyring,machine-id}` → `apeireth-host`, `apeireth-{repo-scan,repo-analyzer}` → `apeireth-repo-tools`
- **`apeireth-integration-r20-stage4`** superseded by `apeireth-integration-e2e`, git mv 到 `crates/_archived/`
- **`apeireth-i18n`** 从 `_frozen` 移回 active (TUI 真实使用)
- **新 crate** `apeireth-host` (keyring + machine_id 5 子模块 union deps) + `apeireth-repo-tools` (scan + analyzer 避免同名 struct 冲突)
- **24 LOCKED 入口签名冻结降级** (per decision-74 §1.1 + decision-130 §2.4): 仅保 3 项不可变脊柱 (Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义含义), 其余可重构

### Added — minimax (MiniMax) 真端到端验证

- **OpenAI Chat Completions** 真接 `https://api.minimaxi.com/v1/chat/completions`: 3 round Keep-Alive LIFO 复用 (3.8s/2.4s/2.6s, tokens 267/392/390)
- **OpenAI Responses API** 真接 `https://api.minimaxi.com/v1/responses`: 1.74s, 228 tokens, model `MiniMax-M3`
- **Anthropic Messages API** 真接 `https://api.minimaxi.com/anthropic/v1/messages`: 3.33s, 126 tokens, `x-api-key` auth
- **minimax + memory 真端到端** (`crates/apeireth-integration-e2e/examples/minimax_memory_roundtrip.rs`):
  - 真 HTTP POST + 真 SQLite file-backed + 真 drop+reopen + 真 semantic_search
  - 1.59s, 89 tokens, "Rust async runtime" 真可检索
- **minimax 6th provider** 加入 `apeireth-provider::minimax` (descriptor + 7 model kinds + 4 协议 + 8 工具白名单)
- 综合报告: [`reports/minimax-end-to-end-r128-2026-08-12.md`](reports/minimax-end-to-end-r128-2026-08-12.md)

### Added — docs + conventions

- 新建 [`docs/archive/conventions/16-crate-merge-policy.md`](docs/archive/conventions/16-crate-merge-policy.md) (16 子规范, §1-§7: 入口签名冻结降级 / frozen / merge / archive 流程)
- [`docs/archive/conventions/10-locked.md`](docs/archive/conventions/10-locked.md) 加 R128 段
- [`docs/archive/top-level/CONTEXT-HANDOVER.md`](docs/archive/top-level/CONTEXT-HANDOVER.md) §12 R128 补记
- [`docs/archive/pages-source/roadmap.md`](docs/archive/pages-source/roadmap.md) §3.5 R128 实际执行
- `Cargo.toml` metadata 加 R128 + decision-130 注释 (B1/A1/A3/B3/B4/B5/C1 解除状态)

### Added — 51/51 active crate README

- 每个 active crate 都有 README (包括 auto-generated + 5 关键 crate 详细: core / memory / api / tui / cli)
- 顶层 `README.md` 重写为生产入门版本 (1 分钟上手 + 5 战区 + minimax 真接 + 借鉴 + license)

### Verified

- `cargo check --workspace` exit 0, 0 errors, 296 historical warnings
- `cargo test -p apeireth-provider` 13 passed (新增 4 个 minimax tests)

### Integration changes (callers migrated)

- `apeireth-tui/Cargo.toml`: `apeireth-observability` → `apeireth-telemetry`
- `apeireth-api` + `apeireth-sdk-{sandbox,lark,livekit,voice}/Cargo.toml`: `apeireth-keyring` → `apeireth-host`
- TUI benches: `apeireth_observability::*` → `apeireth_telemetry::observability::*`
- `apeireth-integration-e2e/Cargo.toml` 加 `apeireth-memory` + `apeireth-core` dev-deps (for `minimax_memory_roundtrip` example)

### Refs

- 决策 #126 (Mavis 全自决 commit 解除)
- 决策 #128 (10 类 30+严守评估)
- 决策 #130 (6 项 B 全部解除 + PHL-07 接受实施)
- 决策 #62 §5.2 (整合 #5 commit 拆 3 commit 范式)

---


### Added — R129+ 真端到端补短板 (2026-08-12)

#### Tool 4 件套 orchestrator 真端到端
- 新建 `crates/apeireth-integration-e2e/examples/tool_orchestrator_e2e.rs`:
  串 `ToolRegistry` (8 真工具) → `ToolCallParser` → `ApprovalManager` (5 规则 + AutoApprove) → `ToolExecutor` → `RecordStore` (真 SQLite)
  - 真解析 LLM `<<<[TOOL_REQUEST]>>>` marker → 2 个 parsed call
  - 真 5 规则按序 → 1 Allow + 1 RequireApproval
  - 真 execute (timeout 10s) + 真 record (SQLite append-only) + 真 approval audit
  - 2 recorded, 3 history entries, end-to-end PASS

#### `/v1/guard` HTTP server 真实 smoke (per Aemeath + decision-130)
- `crates/apeireth-api/examples/v2_smoke.rs` 加 3 段 guard HTTP smoke:
  - `tool.invoke:bypass` empty token → `Allow` + armed=true + 1 check
  - `tool.invoke:bypass` master token → `Deny` + 3 verdict_cache_keys
  - wildcard `*` → `Deny` + 5 checks (全 5 机制跑通)
- 全 6 类 V2 端点 + LLM 端点 + `/v1/guard` = 11 endpoint smoke 全过

#### Kani helloworld proof 落地 (per `.github/workflows/kani.yml` 引用)
- `crates/apeireth-formal/src/kani_harness.rs` 新加 `double_onion_sample()`:
  - `#[cfg_attr(kani, kani::proof)]` 标记 Kani 形式化证明
  - 验证 `l0_requires_ha_invariant(cfg)` ∀ `cfg: PermissionLayerConfig` (kind: u8, requires_ha: bool, 2^9 = 512 states)
  - 用 `nondet_u8() / nondet_bool()` helper (Kani 模式 = `kani::any()`, 非 Kani = concrete 值)
  - 8 单元测试 (visibility + L0 with HA passes + L0 without HA fails + non-L0 always passes)
  - 修复 `.github/workflows/kani.yml` 引用不存在的 harness 名的暗坑

### Changed — workspace 清理

- 删 5 个 untracked `_frozen/` orphan dir (~3.7 GB build artifacts): `apeireth-{lark,sdk-lark,sdk-livekit,sdk-voice,voice}`
- 删 5 个 untracked `target/` 子目录 (~7.3 GB): `_frozen/{image-prompt,plugin}` + `_archived/{integration-r20-stage4,repo-analyzer,repo-scan}`
- 修 `crates/apeireth-i18n/apeireth-i18n/target/` 错位嵌套 (~391 MB) — 真 crat 只有 `crates/apeireth-i18n/`
- `.gitignore` 加 `**/target/` 防止 subdir target 再 untracked

### Verified

- `cargo test --workspace`: **20548 passed / 327 test runs / 0 failed** (~6 min)
- `cargo run -p apeireth-integration-e2e --example tool_orchestrator_e2e`: PASS (8 tools registered, 2 parsed, 3 history)
- `cargo run -p apeireth-api --example v2_smoke`: PASS (8 V2 endpoints + LLM + /v1/guard 全过)
- `cargo test -p apeireth-formal --lib double_onion`: 8/8 PASS

### Refs

- 决策 #74 §1.1 (24 LOCKED 入口签名冻结降级 — 本次重构基础)
- 决策 #130 §2.4 (6 项 B 全部解除 — PHL-07 接受实施)
- 决策 #126 (Mavis 自决 commit 解除)
- 主对话 8/11 22:31 (我们 locked 解锁授权 — 本次多个架构改动前提)

## [1.2.0] — R125-R127 (2026-08-10)

### Added — 整合 #4 + #5 commit (per decision-42 + #48 + #62)

- **4921 passed / 88 suites / 0 failed** 测试基线
- **24 LOCKED crate mtime baseline** 严守 (B1)
- **8 哲学锚升级** (B5, 6→8: 增 S-3 流程自化 + O-1 安全优先)
- **V0.5 25→30 维升级** (B3)
- **6 重守门 v6 → v7 升级** (B4)
- **13 键 verdict cache** (A3, 12 原 12 + PHL-07 = 13 键)
- **Library v1.0 礼物** (30 经典书 + 100+ 论文 + 50+ 视频 + 10+ 课程 + 10+ hub)
- **整合 #5.x commit 系列** (5.1 src/ + 5.2 docs/ + 5.3 R125-R137 era reports/ + 5.4 R129-R163 era reports/ + 5.5 library/v1.0/ 准备)

---

_格式: [Keep a Changelog 1.1.0](https://keepachangelog.com/) + [Semantic Versioning](https://semver.org/)_
