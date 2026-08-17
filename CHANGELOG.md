# Changelog — Apeireth

## [2026-08-18] v1.0.0 正式版 (主人拍板: 真正的 1.0)

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
| R148 | 24 LOCKED 形式撤销扫尾 (仅保 3 项不可变脊柱) + 修 3 个 pre-existing test bugs | `docs/conventions/10-locked.md` |
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

- 新建 [`docs/conventions/16-crate-merge-policy.md`](docs/conventions/16-crate-merge-policy.md) (16 子规范, §1-§7: 入口签名冻结降级 / frozen / merge / archive 流程)
- [`docs/conventions/10-locked.md`](docs/conventions/10-locked.md) 加 R128 段
- [`docs/CONTEXT-HANDOVER.md`](docs/CONTEXT-HANDOVER.md) §12 R128 补记
- [`docs/pages-source/roadmap.md`](docs/pages-source/roadmap.md) §3.5 R128 实际执行
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
- 主对话 8/11 22:31 (主人 locked 解锁授权 — 本次多个架构改动前提)

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
