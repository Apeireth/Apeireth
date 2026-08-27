# ROADMAP — Apeireth (v2.0 工程重构时代)

```
[Document-Meta]
Document:        ROADMAP.md
Version:         3.0-reconstruct-v2
Last-Modified:   2026-08-27
Status:          🟢 活跃
Branch:          main（默认分支；2026-08-27 由 reconstruct_v2 晋升，旧 master 归档为 archive/v1.0-master）
HEAD:            d6910cf7
Tag:             v2.0.0-alpha.1 → d6910cf7（v2 工程重构首个 alpha）
Source-of-Truth: CHANGELOG.md + ARCHITECTURE.md + docs/01-architecture/ 系列审计
```

> **本次重写 (2026-08-27)**：顶层 ROADMAP 从 v1.0-post1.0 时代（8/19 版）升级到 **reconstruct_v2 工程重构完成** 的真实状态。
> 核心定位：**重构版是 1.0 的工程进步**——内核、设计、哲学、愿景 0 变化（见 §13 思想层保留）；
> 变的是工程形态（86-crate 分裂 → 13-crate 单一工作区）。v1.0 时代详单下沉 `docs/archive/roadmap/`。

---

## 0. TL;DR

- **v1.0.0 已发布**（2026-08-18，tag `v1.0.0` → `993e9107`；旧线归档 `archive/v1.0-master`）。
- **reconstruct_v2 工程重构第一阶段完成**（2026-08-23 → 08-27，tag `v2.0.0-alpha.1` → `d6910cf7`）：
  - 旧 86-crate 工作区（58.8 万行 / 23,806 测试）整体归档 `legacy/`（workspace exclude，ref-only）；
  - 新单一工作区 **15 crates / ~74k 行 / ~1476 tests**，按职责分组（foundation 7 / engine 5 / capabilities 1 / adapters 3）；
  - 嵌套 `reconstruction_v2/` 工作区从 git 删除（本地磁盘残留 target/db 未跟踪垃圾，可安全清理）；
  - **agent loop 真实现**——旧审计结论"任何地方都没有 agent loop"已被 `crates/engine/runtime/src/canonical/execute.rs` 推翻；
  - 3 家 provider 插件化（MiniMax/Anthropic/OpenAI-compatible）、5 内置工具（3 只读默认可用；shell/fetch 默认关）、三 OS 进程封装（Windows Job Object 完整 / Linux·macOS 进程组部分）；
  - CI 全绿：cargo-nextest ~1476、clippy 3 档、fmt、audit、deny、miri、rustdoc、coverage、13 键测试契约、M2B/M2C/M3A 三 OS 验证。
- **已知缺口（诚实）**：13 键 verdict cache 已拍板降级为哲学标准（`philosophy.rs::RUNTIME_ENFORCED = false` 显式标注，详见 `docs/04-internal/v2-unabsorbed-features.md` §A4 与 `docs/04-internal/scene-d-v2-plan.md` §3.4），不接 runtime 强制机制；`apeireth-credentials` 未接线；M1B 记忆/向量/图未移植；MCP、companion 器官、voice/screen 未移植。
- **v2.0.0-alpha.1 = 骨架 + 主链的 alpha**：governance P0 已 ✅ 接线（upstream `873d2857`），13 键降级决策 P0 已 ✅ 拍板完成，场景 D 路线见 `docs/04-internal/scene-d-v2-plan.md`。
- **O-6 重构批次 (2026-08-27 启动, 哲学锚 #9 登记后立刻做)**：v2.0.0-rc.1 前的架构最优整理批次, 详见 `docs/04-internal/v2-arch-refactor-batch.md` (5 项 trait 搬 crate + 12 consumer use 行迁移). 工作量约 1-2 天, "不重做" = 默认接受次优, 这是 O-6 锚的第一次兑现.

---

## 1. v1.0 时代（历史，2026-08-18 及以前）

v1.0.0 实际发布路径（R128-R178 + 1.0-final）与 post-1.0 增量（PR #1 桌面伙伴 + CI 防御 + cron 增强）保留于：
- [`docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md`](docs/archive/roadmap/v1.0-released-r128-r178-2026-08-18.md)
- [`docs/archive/roadmap/roadmap-r127-2-2026-08-10.md`](docs/archive/roadmap/roadmap-r127-2-2026-08-10.md)
- 旧顶层 ROADMAP 原文在 git 历史 `f950198d`（8/19 版）。

---

## 2. reconstruct_v2 已完成清单（2026-08-23 → 08-27）

| 里程碑 | 成果 | 证据 |
|---|---|---|
| M1A 存储地基 | 单写者+读池、SQLite WAL、`PRAGMA user_version` 迁移 → `crates/engine/storage` | `docs/01-architecture/m1a-canonical-storage-foundation.md` |
| M1B 记忆 | vector/graph/检索契约 primitive → `crates/engine/memory`（ACT-R 全量移植未做） | `docs/01-architecture/m1b1..m1b3` |
| M1C 治理移植 | Allow/Deny/RequireApproval + PII/注入检测 + 审计哈希链 → `crates/foundation/governance` | `docs/01-architecture/m1c-governance-donor-primitives.md` |
| M2A 简单工具 | filesystem/search/repo 三个只读工具 → `crates/capabilities/tools` | `docs/01-architecture/m2a-simple-tool-ports.md` |
| M2B 进程封装 | 每 OS ProcessExecutor：Windows Job Object + CREATE_SUSPENDED 完整，Linux/macOS 进程组部分 | `docs/01-architecture/m2b*.md` |
| M2C 审批/Shell | approval 生命周期（冻结调用/恢复）+ opt-in trusted shell（默认关） | `docs/01-architecture/m2c*.md` |
| M2D 出站 | egress 策略 + 受控 HTTP transport | `docs/01-architecture/m2d-network-egress-enforcement.md` |
| M3A 受控 Fetch | GET-only、默认 DISABLED、DNS 钉扎+逐跳重校验 | `docs/01-architecture/m3a-canonical-fetch.md` |
| Provider 阶段 3 | 3 家 canonical provider，`LegacyLlmCapability` 全仓 0 命中，凭据 per-turn 解析 | `canonical-skeleton-freeze-audit.md:28` |
| 拓扑收敛 | 86-crate → 13-crate；legacy/ 归档；嵌套工作区删除 | `r0t-repository-topology-audit.md` |
| 主线晋升 | reconstruct_v2 → main；旧 master → `archive/v1.0-master`；tag `v2.0.0-alpha.1` | 本文件 §0 |

---

## 3. 当前状态（v2.0.0-alpha.1，2026-08-27 收盘）

| 项 | 值（实测） |
|---|---|
| 分支 | `main` @ `9080cc93`（默认分支，v1 → `archive/v1.0-master`） |
| Tag | `v2.0.0-alpha.1` → `d6910cf7`（v1.0.0 / v1.5.0 保留） |
| Workspace | **15 crates**（foundation 7 / engine 5 / capabilities 1 / adapters 3）—— 新增 `apeireth-experience` (B1) / `apeireth-perception` (A3) / `apeireth-orchestration` (A1+A2+场景D) |
| 代码量 | ~74k 行 active（不含 legacy/） |
| 测试 | ~1476 passed / 0 failed |
| CI | 全绿（lint/fmt/audit/deny/miri/rustdoc/coverage/13 键契约/M2B/M2C/M3A 三 OS） + `cargo clippy --workspace --all-targets --locked -- -D warnings` 0 警告 |
| 旧 gate | `release-prep`、`pii-leak-detection` 保持 master-only，不在 main 跑 |
| 生产安全现状 | 工具层 shell/fetch 默认关 + **P0 governance 已装 (upstream `873d2857`)** = `PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook`；**13 键 verdict cache 已降级**（P0 拍板完成，2026-08-27，5 维分析：self-introspection 6 数量级延迟 + 0 模型污染路径 + 场景 D 覆盖，详 `v2-unabsorbed-features.md` §A4） |
| ROADMAP §4 P1-P6 | 全部完成（trait 边界 + 0 装占位）：A4 MemoryBackend / B4 sovereignty M-of-N / credentials 接线 / core drain / B1 Experience / A3 perception / B5 process supervisor / A1 council / A2 team-lead / 场景 D 例 1-3 |

### 3.5 阶段表（含 v2.0.0-rc.1 时间表）

| 阶段 | 状态 | tag | 关键标志 | 预计日期 | 工作量 |
|---|---|---|---|---|---|
| **v1.0.0** | ✅ 已发布（历史） | `v1.0.0` → `993e9107` | 86-crate + 23k tests + 9 organ 完整 + companion_serve | 2026-08-18 已发 | — |
| **v2.0.0-alpha.1** | ✅ 已发布（当前） | `v2.0.0-alpha.1` → `d6910cf7` / main = `9080cc93` | 15-crate 工程重构 + governance P0 + 13 键降级 + ROADMAP §4 P1-P6 trait 边界 | 2026-08-27 已发 | — |
| **v2.0.0-rc.1** | 🎯 下一阶段 | `v2.0.0-rc.1`（待发） | alpha trait 接真 backend：MemoryBackend SQLite impl / Experience / PreferenceStore / Council multi-LLM / Orchestrator runtime LLM harness / keyring / process supervisor tokio | **2026-12 月**（预计） | 14-19 周一人 fulltime（10 个 RC 任务，per `docs/04-internal/v2.0.0-rc-roadmap.md`） |
| **v2.0.0** | 远期 | `v2.0.0`（待发） | rc 全绿 + 至少 1 器官移植（W1/W2/E4/F1/F6 选 1） + frontend companion-desktop 接入 v2 gateway | **2027-02-04 月**（预计） | rc 后约 6-8 周 |
| **v2.x (商业化)** | 远期 | — | 多用户 / 跨载体 / 租赁 / marketplace | 2027-Q3+ | — |
| v1 (legacy) | 维护 | `v1.0.0` / `archive/v1.0-master` | 86-crate 完整功能 + 9 organ + companion；v2 rc 后只修严重 bug | 永久 | — |

---

## 4. v2.0 下一步（按优先级，2026-08-27 起）

| P | 任务 | 说明 | 依赖 |
|---|---|---|---|
| **P0** | ✅ 完成（upstream `873d2857`）：`build_canonical_runtime_from_env` 装 `GovernancePipeline(PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook)` | — |
| **P1** | **文档对账**（本批进行中） | ROADMAP/CHANGELOG/交接手册/审计数字统一到 13-crate 实测值 | 无 |
| P2 | core 脊椎去留 + credentials 接线 | core crate 根 legacy 模块（onion/gate/philosophy/memory）决定接线或移入 legacy；`apeireth-credentials` 接回 CredentialResolver | P0 |
| P3 | M1B 记忆移植 | ACT-R 记忆、检索、向量/图全量移植进 `crates/engine/memory` | P2 |
| P4 | MCP 动态能力注册 | MCP 作为 transport capability 接入 plugin registry | P2 |
| P5 | ProcessSupervisor + 沙箱强化 | 进程树快照、Linux cgroup、macOS 强隔离、文件/网络隔离 | P0 |
| P6 | companion 器官移植 | 世界模型 W1/W2/W3、好奇心 E4、假设检验 F4、情感记忆 F1、价值内化 F6 从 legacy 移植回主链 | P3 |
| P7 | 连续感知 | voice/screen（v1 的"连续感知①②"从未落地 main，实现留 legacy） | P6 |
| P8 | 前端产品化 | companion-desktop 对接主链 + 真实流式（旧 TP34 重映射） | P0 |

---

## 5. 硬墙与纪律现状（2026-08-27）

- **3 项不可变脊柱**（R148 后仅保）：Self-Disable 判定 / L0 HA 物理隔离 / 13 键 verdict cache 语义——其中 13 键 v2 角色 = **哲学标准 / 判别词汇表**（`crates/foundation/core/src/philosophy.rs::RUNTIME_ENFORCED = false` 显式标注；`VERDICT_KEYS_BY_PRINCIPLE` 映射到 5 原则洋葱 E/S/A/M 层）。**不是** v2 runtime 强制机制——v2 治理用 external hook 闸（已装 `PermissionGovernanceHook + CredentialDisclosureHook + PromptInjectionHook`）。13 键仍用于: (a) hook deny reason 引用, (b) CapabilityDescriptor risk 分级, (c) ROADMAP §5 语义定义。
- **0 装 PASS** 仍严守：SDK stub 显式 `unimplemented!()`；shell/fetch 默认关；LIVE e2e 缺 API key 如实标注。
- **O-6 永远追求最优 (新哲学锚 9, 2026-08-27 登记)**：见 `docs/01-architecture/philosophy.md` §O-6 详述, 三阶审查 (总体 > 系统 > 架构) + 不做借口清单 + 可检查信号. O-6 兑现的工程化表现: clippy `--workspace --all-targets --locked -- -D warnings` 0 警告 + commit 0 静默失败 + trait 位置必写明理由.
- **push 状态**：main 晋升与 tag 推送已实际发生（2026-08-27）；本地网络受 hosts 劫持（github.com → 127.0.0.1），远端操作走代理 + 真实 IP + Host 头（凭证在本地 `.git-credentials`，未写入 git config）。
- 旧"8 硬墙"明细（B1-B7/A1-A3/C1-C3）见 v1 时代详单，不再逐条跟踪。

---

## 6. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| 生产路径 governance（已 ✅） | PII/注入/凭据泄漏 3 hook 已装（upstream `873d2857`） | **13 键 verdict cache 已降级为哲学标准**（`RUNTIME_ENFORCED = false`，映射到 5 原则洋葱）；**场景 D 长程 AI 判断待评估**（见 `docs/04-internal/scene-d-v2-plan.md`）|
| 功能退坡（26 项 Lost Capabilities） | 产品能力暂时不可用 | 按 §4 P3-P7 顺序恢复；legacy/ 保留全部实现 |
| 文档数字矛盾（v1 era 残留：23,874 vs 23,806；3 vs 5 provider 等） | 误导接手者 | ✅ 文档对账批已统一为 v2 实测值（1476 tests / 15 crates / 3 provider canonical）|
| `crates/_archived` 1.4GB 未跟踪构建垃圾 | 本地仓库膨胀 | 可删除（git 历史已含） |
| 本地 `reconstruction_v2/` 26GB 未跟踪垃圾 | 磁盘占用 | 可删除 |

---

## 13. 思想层保留（哲学 LOCKED，per R119-2 原则）【原样保留，v2 工程重构 0 改动】

| 主题 | 来源 | 状态 |
|---|---|---|
| 立体架构 v2 | R11 / R14 | 🔒 LOCKED |
| 生命架构 v4 | R11 / R14 | 🔒 LOCKED |
| 哲学层升级 v4.1 | R11 / R14 | 🔒 LOCKED |
| 6→9 哲学锚 (8 锚 + **O-6 永远追求最优 NEW 2026-08-27**, 三阶审查 (总体 > 系统 > 架构) + 不做借口清单 + 可检查信号) | 升 9 锚, 哲学锚 #9 登记批次 O-6 重构启动 | 🔒 LOCKED |
| 12 键 → 13 键编译期 hardcode (+ PHL-07 NotUnoptimizable NEW) | 升 13 键, R125-12 P0-3 done; **v2 角色: 哲学标准 / 5 原则洋葱判别词汇表（`VERDICT_KEYS_BY_PRINCIPLE`）**，不再是 runtime 强制机制（`RUNTIME_ENFORCED = false`） | 🔒 LOCKED |
| 5 重守门 → 6 重 v6 → 7 重 v7 (+ Colang DSL + Superpowers Skill Guard) | 升 7 重, P1-3 retry done | 🔒 LOCKED |
| 双洋葱 → 三洋葱 (+ DSL 洋葱, R125-5 done) | 升三洋葱 | 🔒 LOCKED |
| 9 organ 文件名 + 入口签名 0 改 | TUI 9 organ 内部可改（per R148 LOCKED 撤销扫尾原则） | 🔒 软 LOCKED |
| R11 baseline 3 值 (0.8682/0.8532/0.9063) 数字严守 | R11 ASI R-Measure | 🔒 LOCKED (A1 仍严守) |

详见 [`docs/01-architecture/philosophy.md`](docs/01-architecture/philosophy.md) + [`docs/01-architecture/vision.md`](docs/01-architecture/vision.md) + [`docs/archive/conventions/09-anchor.md`](docs/archive/conventions/09-anchor.md)。

---

_本 ROADMAP 由文档对账批重写 (2026-08-27)，反映 reconstruct_v2 工程重构完成 + main 晋升 + v2.0.0-alpha.1。思想层（8 锚 / 13 键 / 三洋葱 / 9 organ / R11 baseline 3 值）LOCKED 保留。v1.0 时代详单见 `docs/archive/roadmap/`。_
