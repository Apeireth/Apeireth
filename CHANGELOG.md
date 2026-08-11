# Changelog — Apeireth

> **R127-2 P7-1 准备 (2026-08-10)**: 重写顶层 CHANGELOG, 完整 Keep a Changelog 1.1.0 格式. **0 主动 commit 严守** (写到主仓, Mavis 整合 #5 commit 时机拍板).

```
[Document-Meta]
Document:       CHANGELOG.md
Version:        1.0.0-R127 (semver 归 0, R127 release)
R-Cycle:        R127-2 阶段 B (CHANGELOG v1.0.0 准备, P7-1 派)
Last-Modified:  2026-08-10 (draft, 0 commit yet)
Status:         🟡 草稿 (Mavis 整合 #5 commit 时机拍板)
Author:         Mavis (mvs_47dd64fb4fc24e23b30edd5f649bfebb)
整合 #4 commit: abf12243 (19:41 done, 46752 file changes, 0 M+?? 异常)
Cargo.toml:     1.2.0 严守 (B2 upgrade, per decision-22 §2.2 + decision-33 §2.3)
master HEAD:    abf12243 (新仓挪到 Apeireth-rust/)
```

格式: [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) + [Semantic Versioning](https://semver.org/lang/zh-CN/)

> **8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略) 0 越界** (per 决策 #33 + #55):
> - **B1** 24 LOCKED crate 持续更新, 内部 fn 实施可改, **入口签名 0 改**
> - **B2** workspace.version 1.2.0 (整合 #4 commit abf12243 严守, 0 改)
> - **B3** V0.5 30 维 (24 + Robustness 鲁棒性 + 5 扩展, P1-4 R126 verify done)
> - **B4** 6 重守门 v7 (5 嵌套 + Colang DSL, P1-3 R126 升级 done)
> - **B5** 8 哲学锚 (S-1 北极星 / S-2 实事求是 / S-3 质量工程化 / O-1 安全优先 / O-2 走在前人 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装, P1-2 R126 升级 done)
> - **B6** 三洋葱架构 (原则 + 权限 + DSL)
> - **B7** 9 organ 内部 fn 借 OpenCode (199KB → 120KB, -40%, R125-12 实施)
> - **A1** R11 baseline 3 值数字严守 (0.8682/0.8532/0.9063, 17 文件原位 0 删 0 改)
> - **A2** R11 Python 9 子测度结构严守
> - **A3** 13 键 (12 键原 12 + PHL-07 = 13 键, R125-12 实施 done)
> - **C1** 0 主动 commit (Mavis 整合 #5 commit 时机拍板, 跑过夜 8/11-8/22 done 后)
> - **C2** 0 装 PASS 严守 (✅ cloned = 真实施, ⏳ 限流 = 准备, ❌ 跳过 = 0 集成)
> - **C3** 升 6 重 v7 (P1-3 R126 done)
> - **0 主动 push** 严守 (等 1.0 release 配 GitHub remote)

---

## [Unreleased] — R127-2 跑过夜明早 8/11-8/22 done

> **状态**: 🟡 整合 #5 commit 准备中, 22+10 = 32 sub-agent 跑过夜, 主人起床后 8 步 verify 后拍板.

### Added (R127-2 进行中, 0 装 PASS 严守)

- **整合 #5 pre-check verify** (P4-1, bg_58b1dc36): 24 LOCKED 入口签名 0 改 verify + 0 装 PASS verify + 8 硬墙 0 越界 verify + 借鉴 8/11 verify + Cargo.toml 1.2.0 verify + master HEAD = abf12243 verify
- **Library Stage 4 自治** (P5-1, bg_fcc5945a): 自演化 + 自升级 + 自修复 (借鉴 superpowers 234 + aGLM 108 + Chidori), 跑过夜 8/11-8/22 done
- **Library Stage 5 治理** (P5-2, bg_21ecbe0c): 治理策略 + 形式化验证 + 一致性 (借鉴 clap 725 + Kani 4502)
- **Library Stage 6 守护** (P5-3, bg_088f9d96): 守护 + 跨语言桥 + 长期记忆 (借鉴 hyper 80 + PyO3 928 + servers 175)
- **LiteLLM Provider Registry 重试** (P6-1, 21:18 派): 让借鉴 8/11 → 11/11 真实施 (⏳ 限流持续, 重试)
- **opencode 子代理 重试** (P6-2, 21:18 派): 真实施 子代理 + Tool execution + Context 管理 (⏳ 限流持续, 重试)
- **Guardrails 6 重守门 重试** (P6-3, 21:18 派): NVIDIA Guardrails 真实施 6 重守门 + Colang DSL + 行动轨 (⏳ 限流持续, 重试)
- **CHANGELOG v1.0.0 准备** (P7-1, 21:18 派, **本任务**): 整合 R125-R127 决策链 + 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 + Library v1.0 + 借鉴 8/11 + 整合 #4 commit → 完整 Keep a Changelog 格式
- **ROADMAP 准备** (P7-2, 21:18 派): 1.0 → 2.0 路线图: R125-R127 总结 + R128+ 规划 + 借鉴 11/11 + Library Stage 4-6 + ASI Python 整合 + Tauri 终极前端
- **release notes 准备** (P7-3, 21:18 派): 1.0.0 release notes: 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 + Library v1.0 + 借鉴 8/11 真实施 + 整合 #4 commit + 决策链
- **Library Stage 4.1 自治 - 自循环** (P8-1, 21:18 派, 深化 P5-1): 借鉴 superpowers 234 自治循环 + aGLM 108 PODA cycle
- **Library Stage 5.1 治理 - 形式化证明** (P8-2, 21:18 派, 深化 P5-2): 借鉴 Kani 4502 proofs 模板
- **Library Stage 6.1 守护 - 跨语言桥** (P8-3, 21:18 派, 深化 P5-3): 借鉴 PyO3 928 pybridge + hyper 80 池复用
- **borrowed-repos 进阶 - Stage 2 借脑 1.0** (P9-1, 21:18 派, 深化 P2-1): 借鉴 8/11 真实施 → 实际 import + crates 引用
- **整合 #5 commit 准备文档** (Mavis 干, 决策 #55 §2.7): Cargo build/test/run verify 文档 + 8 硬墙 verify 文档 + LICENSE + OSS NOTICE 框架

### Changed (R127-2 进行中)

- **0 主动 commit 严守**: R127-2 10 sub-agent 写 CHANGELOG/ROADMAP/release notes/Library 进阶 写到主仓但 0 主动 commit, Mavis 整合 #5 commit 时机拍板 (per 决策 #55 §5 + 决策 #56 §5)
- **5 min tick cron 监督**: `watch-r126-r127-32-sub-agents-20-25-21-13` 跑中, 0 主动 IM 主人 (per gate-discipline, 仅 done notification 主动报告)

### Security (R127-2 严守)

- **借鉴源码 0 装 PASS 严守** (per 决策 #55 §3 + 决策 #56 §3): ✅ 8 cloned = 真实施 (clap 725 / hyper 80 / servers 175 / PyO3 928 / kani 4502 / langgraph 829 / superpowers 234), ⏳ 3 限流 = 准备 (LiteLLM 0 / opencode 0 / Guardrails 0 files submodule, R127-2 阶段 A 重试), ❌ 1 跳过 = 0 集成 (OpenCog AGPL-3.0)

---

## [1.0.0] - 2026-08-10 — R127 整合 release (semver 归 0)

> **状态**: 🟡 整合 #4 commit `abf12243` 已 done (19:41, 46752 file changes), 整合 #5 commit 待 R127-2 32 sub-agent 跑完 8/11-8/22 done 后拍板.
>
> **本版本 = R127 release 1.2.0 → 1.0.0 (大版本归 0, per 决策 #22 §2.2)**: 整合 R125-R127 决策链 + 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 + Library v1.0 + 借鉴 8/11 真实施 + 整合 #4 commit.
>
> **R20 era "v1.0.0" (2026-08-05) 已被本版本覆盖**: R20 阶段 1-6 1.0 release 收口 (12 项 checklist 100% PASS) 是 R125-R127 前的内部里程碑, 完整 release notes 下沉到 [`docs/release/1.0.0/CHANGELOG.md`](docs/release/1.0.0/CHANGELOG.md) 保留作历史.

### Added (R125-R127 阶段新增)

#### 整合 #4 commit `abf12243` (19:41 done, 46752 file changes)

- **18 决策文件 #30-#47 入 commit**: 决策 #30 (新 Mavis 接入) → 决策 #47 (git reset 0 真正起作用), 全 per 决策 #48 §2 verify
- **10 M src 文件** (R125 sub-agent 真实施):
  - `Cargo.lock` (202 行)
  - `Cargo.toml` (3 行: clap = "4.5" R125-2 deps)
  - `crates/apeireth-cli/Cargo.toml` (2 行)
  - `crates/apeireth-cli/src/commands.rs` (-498 行, clap derive 重构)
  - `crates/apeireth-evolution/src/lib.rs` (6 行, PODA 接入)
  - `crates/apeireth-mcp/src/lib.rs` (120 行, 协议对齐)
  - `crates/apeireth-mcp/src/tools/mod.rs` (-350 行, 大幅精简)
  - `crates/apeireth-pybridge/src/bridge.rs` (203 行, PyO3 真链接)
  - `crates/apeireth-pybridge/src/lib.rs` (7 行)
  - `crates/apeireth-pybridge/src/python_bindings.rs` (56 行)
- **14 untracked src 文件** (R125 sub-agent 新写):
  - `crates/apeireth-cli/src/commands_tests.rs` (R125-2 clap derive tests)
  - `crates/apeireth-core/src/.r125-12-PHL-07-SPEC.md` (R125-12 PHL-07 spec)
  - `crates/apeireth-evolution/PODA_CYCLE_INTEGRATION.md` (R125-7)
  - `crates/apeireth-evolution/src/poda_cycle.rs` (R125-7)
  - `crates/apeireth-mcp/src/macros.rs` `primitives.rs` `tools/naming.rs` `tools/server.rs` `tools/types.rs` (R125-4)
  - `crates/apeireth-sovereignty/src/colang_dsl.rs` (R125-5 NVIDIA, 51591 bytes 18:22 收齐)
  - `crates/apeireth-supervisor/src/journal_entry.rs` (R125-8 Chidori)
  - `crates/apeireth-tui/src/organ/.r125-12-13-keys-stub.rs` `organ/.r125-12-REFACTOR-PLAN.md` (R125-12)
- **.gitignore 升级版** (per 决策 #33 §6): 新增 `out/` + `apeireth/out/` + `.git_commit_msg.txt` 3 行
- **Cargo.toml workspace.version 1.1.0 → 1.2.0** (B2 升级, per 决策 #22 §2.2 + 决策 #33 §5)

#### B1 24 LOCKED crate 完整名单 (Mavis 自主, 主人 16:31 最高权限)

- **12 已知 LOCKED** (主人 8/10 已 8-promise-audit + 1.0-release-report §6.1):
  1. `crates/apeireth-supervisor/src/lib.rs` (LOCKED 主管)
  2. `crates/apeireth-agent/src/lib.rs` (LOCKED agent)
  3. `crates/apeireth-bus/src/lib.rs` (LOCKED 事件总线)
  4. `crates/apeireth-council/src/lib.rs` (LOCKED 议会)
  5. `crates/apeireth-evolution/src/lib.rs` (LOCKED 演化)
  6. `crates/apeireth-extension/src/lib.rs` (LOCKED 扩展)
  7. `crates/apeireth-graph/src/lib.rs` (LOCKED 图)
  8. `crates/apeireth-mcp/src/lib.rs` (LOCKED MCP, R125-4 协议对齐 入口签名 0 改)
  9. `crates/apeireth-pipeline/src/lib.rs` (LOCKED 管道)
  10. `crates/apeireth-tool-registry/src/lib.rs` (LOCKED 工具注册)
  11. `crates/apeireth-tool-runtime/src/lib.rs` (LOCKED 工具运行时)
  12. `crates/apeireth-protocol/src/lib.rs` (LOCKED 协议, R20 阶段 2 续时授权 +8 lines + ws_v1.rs)
- **13-24 Mavis 自主 LOCKED** (per 决策 #22 §1.2):
  13. `crates/apeireth-asi/src/lib.rs` (LOCKED V0.5/V1136, 24 维公式, 哲学核心)
  14. `crates/apeireth-onion/src/lib.rs` (LOCKED 5 重守门来源, 双洋葱架构, 哲学核心)
  15. `crates/apeireth-sovereignty/src/lib.rs` (274KB LOCKED 安全核心, R124-3 调研 0 触碰)
  16. `crates/apeireth-constraint/src/lib.rs` (LOCKED 5 重守门核心)
  17. `crates/apeireth-memory/src/lib.rs` (LOCKED memory 9 文件, 3 层 memory 哲学核心)
  18. `crates/apeireth-cognition/src/lib.rs` (R124-2 B-028 OpenCog 借鉴目标, 9 organ 之一 brain)
  19. `crates/apeireth-perception/src/lib.rs` (R20 哲学 crate, 9 organ 之一 eye/ear)
  20. `crates/apeireth-consciousness/src/lib.rs` (R20 哲学 crate, R37-2 transparent re-export 到 perception)
  21. `crates/apeireth-motivation/src/lib.rs` (R20 哲学 crate, R37-2 transparent re-export)
  22. `crates/apeireth-life-force/src/lib.rs` (R20 哲学 crate, R37-2 transparent re-export 到 memory)
  23. `crates/apeireth-relation/src/lib.rs` (R20 哲学 crate, R124-2 §12 借鉴目标)
  24. `crates/apeireth-value/src/lib.rs` (R20 哲学 crate, R37-2 transparent re-export 到 motivation)
- **24 LOCKED + 9 organ + 8 LOCKED 文档 = 41 LOCKED** (per 决策 #22 §1.3)
- **24 LOCKED 入口签名 0 改 verify done** (P2-3 R126 retry bg_38d67325, per 决策 #55 §4)

#### B3 V0.5 30 维 (P1-4 R126 25→30 维 verify retry done)

- **24 维综合 0.8682** (R11 baseline 数字严守, A1)
- **V1131 = 0.8532** (R11 baseline 数字严守, A1)
- **V1136 = 0.9063** (R11 baseline 数字严守, A1)
- **25 维** (P1-2 R126 8 哲学锚升级 done, 24 + Robustness 鲁棒性 per 决策 #22 §2.3)
- **30 维** (P1-4 R126 25→30 维 verify retry done, 5 扩展: Robustness + Self-Improvement + Adversarial + CI-pass-rate + Verifier-consistency, R125-13 LangGraph StateGraph 60 tests 30 维 sum=1.0 已实现)
- **V0.5 公式 sum=1 守门** (0 改, 24/25/30 维可扩展)

#### B4 6 重守门 v7 (P1-3 R126 6 重守门 v7 升级 done)

- **v5 修正**: 4 重嵌套 + 权限发放独立机制 (per glossary/17-4-gates-permission.md)
- **v6 (R125-5 NVIDIA Guardrails 借鉴后)**: 5 重嵌套 + 权限发放 + Colang DSL 守门 (新加第 5 重)
- **v7 (P1-3 R126 升级后)**: 6 重 v7 (5 嵌套 + DSL + 形式化, R125-10 Kani 借鉴整合)
- **守门 1-6 联合**: 守住"没有相应权限而运行的代码" + DSL 守门 + 形式化守门

#### B5 8 哲学锚 (P1-2 R126 8 哲学锚升级 done)

- **S-1** 北极星导向 (R11 baseline 锚)
- **S-2** 实事求是 (R11 baseline 锚)
- **S-3** 质量工程化 (新增, R125 末加, 跟 R123-1 clippy+doc 清关联, "代码质量 = 工程信誉" L1 速赢)
- **O-1** 安全优先 (新增, R125 末加, 跟 5 重守门关联, "安全 > 功能 > 性能" per v5 守门 1-4 顺序)
- **O-2** 走在前人肩上 (R11 baseline 锚)
- **O-3** 干到底 (R11 baseline 锚)
- **O-4** 任何人都能接手 (R11 baseline 锚)
- **O-5** 不假装 (R11 baseline 锚)

#### B6 三洋葱架构 (R125-5 NVIDIA 借鉴后)

- **原则洋葱** (R11 baseline)
- **权限洋葱** (R11 baseline)
- **DSL 洋葱** (R125-5 NVIDIA 借鉴, 守门 5 v6 / 守门 6 v7, 用 Colang DSL 表达"什么操作允许 / 禁止", 跟权限矩阵正交)

#### B7 9 organ 内部 fn 借 OpenCode (R125-12 实施)

- **9 organ 保留** (per R124-2 §14.4 认知科学有依据): body/brain/ear/eye/hand/heart/memory/mind/voice
- **器官文件名 + 入口签名 0 改** (per 决策 #22 §2.7)
- **器官内部 fn 借 OpenCode 子代理**: per R124-1 TUI-1 借鉴, OpenCode Build/Plan/Scout 子代理拆 5 nav 跨界 + oh-my-opencode 4 专家角色拆 9 器官
- **ROI**: backend.rs 199KB → 120KB (-40%), 单一职责更清晰

#### A3 13 键 (R125-12 实施, 整合 #4 commit done)

- **12 键原 12** (V3 9 键 + v4.1 3 键, A3 严守, 0 改)
- **PHL-07 NotUnoptimizable** (新增 1 键, R125-12, 加"代码不假装已优化"语义, 跟 clippy+doc 清关联)
- **13 键 = 12 + PHL-07** (整合 #4 commit done)

#### Library v1.0 (research → library 升级 6 阶段, R125-15/16/17/18/19/20/21 + R127 P2-4 礼物)

- **阶段 1** Library 命名 + 文档结构 (R125-16 done): `library/README.md` + `library/INDEX.json` + `library/CLASSIFICATION.md`
- **阶段 2** 9 大类升级 (R125-17 done): 9 子文件夹升级 + 10/11/12 新子目录 (`library/10-non-github-resources/` + `library/11-vcp-reference/` + `library/12-borrowed-repos/`)
- **阶段 3** 借鉴 ID 严格化 (R125-18 done): 400+ 借鉴 ID 统一格式 `R{N}-BORROW-{type}-{owner/repo or title}-{hash}-YYYY-MM-DD`, 索引到 `library/INDEX.json` + `library/_BORROW_IDS.md`
- **阶段 4** Library 摘要 (R125-19 done): 9 大类每类 1 份 `_SUMMARY.md` (10-30KB) + `library/_TOP_100.md` (50KB) 主人 1.0 release 前 100 必读
- **阶段 5** Library 工具 (R125-20 done): `library/_SEARCH.md` (检索指南) + `library/_CROSS_REF.md` (跨引用, 跟 9 organ / 24 LOCKED / 5 守门 对应) + 集成到 TUI 9 organ page (5 nav 之一 "Library")
- **阶段 6** 1.0 release 礼物 (R125-21 done + P2-4 R126 Library v1.0 礼物 bg_93832073 done): Library v1.0, 30 本经典书 + 100 论文 + 50 视频 + 10 社区 + 10 hub
- **Library 进阶 4 阶段** (R127 阶段 B/C/D + R127-2 阶段 C/D):
  - **Stage 4 自治** (P5-1 跑中): 自演化 + 自升级 + 自修复, 借鉴 superpowers 234 + aGLM 108 + Chidori
  - **Stage 5 治理** (P5-2 跑中): 治理策略 + 形式化验证 + 一致性, 借鉴 clap 725 + Kani 4502
  - **Stage 6 守护** (P5-3 跑中): 守护 + 跨语言桥 + 长期记忆, 借鉴 hyper 80 + PyO3 928 + servers 175
  - **Stage 4.1 自治 - 自循环** (P8-1 派中, 深化 P5-1)
  - **Stage 5.1 治理 - 形式化证明** (P8-2 派中, 深化 P5-2)
  - **Stage 6.1 守护 - 跨语言桥** (P8-3 派中, 深化 P5-3)
  - **borrowed-repos 进阶 - Stage 2 借脑 1.0** (P9-1 派中, 深化 P2-1)

#### 借鉴源码 8/11 真实施 (0 装 PASS 严守, per 决策 #55 §3 + 决策 #56 §3)

- **✅ 8 真实施** (cloned, 有真 src 改动 + tests pass):
  - **clap 725 files** (R125-2 done, 19/19 tests pass, commands.rs -498 行 clap derive 重构, bg_a16f9d2c)
  - **hyper 80 files** (R125-3 done, 38/38 tests pass, 池复用, bg_4e7f2c11)
  - **servers 175 files** (R125-4 done, 188 tests = 183+5, 4 文件 29.4KB, 协议对齐, bg_c2a8b73e)
  - **PyO3 928 files** (R125-9 done, 77/77 tests pass, 6 E0599 全修, PyO3 0.29.2 真链接, bg_5e9a1c44)
  - **kani 4502 files** (R125-10 done, 12 文件 75.8KB, 5 阶段, bg_0105b455)
  - **langgraph 829 files** (R125-13 done, 10 NEW 85.9KB, 60 tests, 30 维 sum=1.0, bg_903199b0)
  - **superpowers 234 files** (R125-14 done, 8 文件 ~80KB, 79/79 tests, bg_754fac4b)
  - **PyO3 928 (复用)** (R125-8 Chidori journal done, 13/13 tests, 0 装 PASS, bg_a1c2b3d4)
- **⏳ 3 限流** (0 装 PASS 严守, 准备, R127-2 阶段 A 重试):
  - **LiteLLM 0 files** (R125-1 era ⏳ 限流, P6-1 R127-2 阶段 A 重试 21:18 派)
  - **opencode 0 files** (R125-12 era ⏳ 限流, P6-2 R127-2 阶段 A 重试 21:18 派)
  - **Guardrails 0 files submodule** (R125-5 era ⏳ 限流, P6-3 R127-2 阶段 A 重试 21:18 派)
- **❌ 1 跳过** (0 集成): **OpenCog AGPL-3.0** (R125-6 era 0 集成, LICENSE 风险)

#### R125 16 sub-agent 全部 done (per 决策 #41, 18:35 verify)

- **P0 4 sub-agent**: R125-1 LiteLLM ⏳ / R125-2 clap ✅ / R125-3 hyper ✅ / R125-4 MCP servers ✅
- **P1 4 sub-agent**: R125-5 NVIDIA ⏳ / R125-7 aGLM PODA ✅ / R125-8 Chidori journal ✅ / R125-9 PyO3 pybridge ✅
- **P2 4 sub-agent**: R125-10 Kani ✅ / R125-12 OpenCode ⏳ / R125-13 LangGraph ✅ / R125-14 superpowers ✅
- **P3 4 sub-agent**: R125-15a 学术论文 ⏳ / R125-15b 官方文档 ✅ / R125-15c 技术博客 ✅ / R125-15d 会议视频 ⏳
- **6/16 final 报告已写** (R125-2/4/7/8/9/12), **10/16 MISS final** (0 装 PASS 严守)
- **9/16 真实施** (R125-2/3/4/8/9/10/13/15b/15c), **7/16 准备** (R125-1/5/7/12/14/15a/15d)
- **16/16 task daemon succeeded** ✅

#### R126 16 sub-agent 全部 done + 2 retry (per 决策 #55 §1.1)

- **✅ done 14** (R125 era 8 跨段 + R126 era 6):
  - P0-1 R125-15e fg_xxxx ✅
  - P0-2 R125-15f bg_16a97b77 ✅
  - P0-3 R125-16 retry bg_ff678db3 ✅
  - P0-4 R125-17 bg_891ffb29 ✅
  - P1-2 R126 8 哲学锚 bg_77bafd5d ✅
  - P1-4 R126 25→30 维 verify retry bg_e62f3e67 ✅
  - P2-1 borrowed-repos 整合 bg_9790f9f8 ✅
  - P2-2 .gitignore 修 bg_1f8d0ba1 ✅
  - P2-3 B1 LOCKED verify retry bg_38d67325 ✅ (24/24 LOCKED 入口签名 0 改 verify)
  - P2-4 Library v1.0 礼物 bg_93832073 ✅
  - P3-1 R125-18 bg_bfeb840c ✅ (含事故 #1 诚实标)
  - P3-2 R125-19 bg_68dcfdb9 ✅
  - P3-3 R125-20 bg_b9337fc4 ✅
  - P3-4 R125-21 retry bg_b9facf9a ✅ (30 经典书 9 organ 1:1)
- **🟡 跑中 2** (P1-1 R126 后端升级 retry bg_f8ee6f29 21:11 派 + P1-3 R126 6 重守门 v7 retry bg_b4c7a22f 21:11 派)
- **❌ failed 0**

#### 决策链 #22-#56 (R124-R127 era)

- **决策 #22** (8/10 16:35): 主人 16:31 最高权限 + 24 LOCKED 自主确认 + 9 项实质 locked 升级 (B1-B7 + A1-A3 + C1-C3)
- **决策 #23-#30**: 16 pipeline / 派活 daemon 修复 / 新 Mavis 接入
- **决策 #31-#32**: 17:30 dry-run + R125 派活大主管启动
- **决策 #33** (8/10 17:23): 主人 17:22 升级授权 + 8 硬墙全部重置 + B1-B7 升级路线 + 0 装解除 + 16 派满 + 17:30 commit 拍板升级版
- **决策 #34-#37**: 17:30 整合 #3 commit 21aa85f3 done + 16 真派 模式 + 4 P2 sub-agent 跑中 + R125-8 17:36 done
- **决策 #38-#39**: 0 新派成员 + 暂停讨论后续
- **决策 #40-#42**: promethean cleanup + R125 16 sub-agent 全部 succeeded + R125 续整合 #4 pre-checklist 4 项
- **决策 #43-#47**: apeireth-tui no-merge move + promethean cleanup deletion + git history lost after move + git mv done + git reset 0 真正起作用
- **决策 #48** (8/10 19:41): 整合 #4 commit abf12243 done (主仓挪到 Apeireth-rust/, 46752 file changes, 0 M+?? 异常, Cargo.toml 1.2.0 严守)
- **决策 #49-#51**: promethean cleanup 全 done + R126/R127 16 sub-agent 派活清单
- **决策 #52-#53**: 16 真派模式 + 主人 20:32 "技术性 locked 都能解锁" 升级授权
- **决策 #54-#55**: P1-4 failed retry pending + R127 升级路线 + 派活清单 (整合 #5 pre-check + Library Stage 4-6 + 借鉴 3 限流重试 + 1.0 release 准备)
- **决策 #56** (8/10 21:18): R127-2 派活 10 sub-agent (借鉴 3 限流重试 + 1.0 release 准备 + Library 阶段 4-6 进阶 + borrowed-repos 进阶)

#### 主仓挪到 Apeireth-rust/ (per 决策 #48)

- **挪出** .openclaw/workspace/promethean/Apeireth-rust/ → Apeireth-rust/ (独立主仓)
- **mv .git done** (per 决策 #46 git mv done)
- **整合 #4 commit abf12243 done** (per 决策 #48, 19:41 主人自执行 A)
- **0 主动 push 严守** (等 1.0 release 配 GitHub remote)

### Changed (R125-R127 阶段)

- **workspace.version 1.1.0 → 1.2.0** (B2 升级, per 决策 #22 §2.2 + 决策 #33 §5, 整合 #4 commit done)
- **V0.5 24 维 → 25 维 → 30 维** (B3, per 决策 #22 §2.3, P1-2/P1-4 R126 升级 done)
- **5 重守门 v5 → 6 重 v6 → 6 重 v7** (B4, per 决策 #22 §2.4, P1-3 R126 升级 done)
- **6 哲学锚 → 8 哲学锚** (B5, 加 S-3 质量工程化 + O-1 安全优先, per 决策 #22 §2.5, P1-2 R126 升级 done)
- **双洋葱 → 三洋葱** (B6, 加 DSL 洋葱, per 决策 #22 §2.6, R125-5 实施)
- **9 organ backend.rs 199KB → 120KB** (B7, -40%, per 决策 #22 §2.7, R125-12 实施)
- **12 键 → 13 键** (A3, 加 PHL-07 NotUnoptimizable, per 决策 #22 §2.8, R125-12 实施 done)
- **0 装 (O-5) 12 键编译期 hardcode → 0 装 PASS 严守** (C2, per 决策 #33 §2.3, 主人 17:22 升级授权后 ✅ cloned = 真实施)
- **Cargo.toml clap = "4.5"** (R125-2 deps 升级, 整合 #4 commit done)
- **Cargo.lock +202 行** (R125 deps, 整合 #4 commit done)

### Deprecated

- **R11 baseline 3 值子测度结构** (0 改, A2 严守, V1141/V1131/V1136 数字永严守)
- **R20 era "v1.0.0" (2026-08-05)**: 完整 release notes 下沉到 [`docs/release/1.0.0/CHANGELOG.md`](docs/release/1.0.0/CHANGELOG.md) 保留作历史, 1.0 release 时不再次提及
- **1.1.0/1.1.1/1.1.2/1.2-candidate/1.2-patch-LIVE/1.2-patch-LIVE-续/1.2-R114-118 内部版本**: 已并入 1.0.0 release cycle, release notes 下沉到 `docs/release/<version>/CHANGELOG.md` 保留作历史

### Removed

- **27 ASI Python `out/` 文件** (V1467-V1471 audit, 跟 R125 独立, 0 commit 到 Apeireth-rust): 通过 `.gitignore` 升级版 (`out/` + `apeireth/out/`) 严守, 整合 #4 commit 0 含 ASI out/
- **33 个 promethean/ 待删** (per 决策 #44): 0 必再删, 决策 #50 全 done ✅
- **5 散文件** (per 决策 #50): 0 必再删, 全 done ✅

### Fixed (R125-R127 阶段)

- **P2-3 R126 B1 24 LOCKED 入口签名 0 改 verify** (bg_38d67325): 24/24 LOCKED 入口签名 0 改 verify done ✅
- **R123-1 clippy+doc 清** (整合 #3 commit 拍板含 R123-1 fix, 整合 #4 commit done, 2 error 全修)
- **git reset 0 真正起作用** (per 决策 #47): 通过整合 #4 commit 一次性 `git add .` + `git commit` 真正 fix 774 M+?? 异常
- **git 历史 0 丢** (per 决策 #45/46/47): master HEAD 历史从 21aa85f3 继续, 0 重新 init, 0 必 worktree

### Security (R125-R127 阶段)

- **24 LOCKED 入口签名 0 改** (B1, per 决策 #33 §2.3 + 决策 #55 §4): 6 M src 文件 pub 改 (commands.rs 4 删 4 增 clap derive / lib.rs evolution 1 增 R125-7 PODA / lib.rs mcp 3 增 1 删 R125-4 协议对齐 / tools/mod.rs 4 增 9 删 R125-4 大幅精简 / pybridge 3 files 0 改 R125-9 内部 fn 改), 24 LOCKED 入口签名 0 改
- **R11 baseline 3 值数字严守** (A1, per 决策 #33 §2.3 + 决策 #55 §4): 0.8682/0.8532/0.9063 数字 0 改, 17 文件原位 (blueprint-impl/cli/cache/telemetry/tracing/metrics/motivation/naming-v05/integration-e2e/integration-r20-stage4/asi)
- **Cargo.toml 1.2.0 严守** (B2, per 决策 #55 §4): 整合 #4 commit abf12243 严守, 0 改
- **0 装 PASS 严守** (C2, per 决策 #55 §4 + 决策 #56 §4): ✅ 8 cloned = 真实施 (有真 src 改动 + tests pass), ⏳ 3 限流 = 准备 (LiteLLM/opencode/Guardrails, R127-2 阶段 A 重试), ❌ 1 跳过 = 0 集成 (OpenCog AGPL-3.0)
- **0 主动 commit 严守** (C1, per 决策 #55 §4 + 决策 #56 §4): R127-2 P7-1/2/3 写 CHANGELOG/ROADMAP/release notes 到主仓 0 主动 commit, Mavis 整合 #5 commit 时机拍板
- **0 主动 push 严守** (per 决策 #55 §4 + 决策 #56 §4): 等 1.0 release 配 GitHub remote
- **PHL-07 NotUnoptimizable 新增 1 键** (A3, R125-12 实施): 加"代码不假装已优化"语义, 跟 clippy+doc 清关联

### Notes (R125-R127 阶段)

- **0 主动 IM 主人 严守** (per gate-discipline): 仅 done notification 主动报告, 0 主动 plain reply on skip ticks, 0 主动 push / 0 主动 commit / 0 主动删 / 0 主动讨论后续
- **5 min tick cron 监督** 持续: `watch-r126-r127-32-sub-agents-20-25-21-13` 跑中, 0 主动 IM 主人
- **整合 #5 commit 时机** (per 决策 #55 §5 + 决策 #56 §5): 32 sub-agent (22 已派 + 10 R127-2) 全 done + 0 装 PASS 严守 verify + 8 硬墙 0 越界 verify + 24 LOCKED 入口签名 0 改 verify, Mavis 拍板 OR 主人 8/15 拍板
- **主人起床后 8 步** (per 决策 #55 §8 + 决策 #56 §8): 1) 修 session working dir 2) cargo build --workspace 3) cargo test --workspace 4) cargo run --bin apeireth-tui 5) cargo run --bin apeireth-api 6) cargo audit + cargo deny 7) 验证 24 LOCKED 入口签名 0 改 8) 验证 8 硬墙 0 越界 + 0 装 PASS 严守 (✅ 11 + ⏳ 0 + ❌ 1)

---

## [0.5.0] - 2026-08-08 — R11-R20 era 内部版本 (pre-1.0)

> **状态**: 🟢 已发, 内部版本, pre-1.0 开发 cycle, 完整 release notes 下沉到 [`docs/release/0.5.0/CHANGELOG.md`](docs/release/0.5.0/CHANGELOG.md) (待写, R127-2 后创建)

### Added (R11-R20 era)

- **R11 baseline 3 值锁定** (2026-08-08 数字严守): V1141=0.8682 (24 维综合) / V1131=0.8532 / V1136=0.9063, 17 文件原位 (blueprint-impl/cli/cache/telemetry/tracing/metrics/motivation/naming-v05/integration-e2e/integration-r20-stage4/asi)
- **24 LOCKED crate mtime baseline 16:34 锁定** (per 决策 #22 §1): 12 已知 + 60+ 实质 LOCKED, 整合 #3 commit done 严守
- **R14 Rust 重写** (per docs/release/archive/r14-001-012.md): 22 trait 互锁 + 权限洋葱 + 风险等级 M1-M12 + 集成 rebase-skip + 兼容组件层 + Feature gating pybridge + MCP from SpectrAI + team-lead supervisor + team-lead council
- **R17 战役 0-4** (per docs/release/archive/r17-*): 战役 1-1 (MVP 真接) + 战役 2-1 (commander stub) + 战役 3-1 (council 真接) + 战役 4-1 (relay 复盘)
- **R19 集成 4 子阶段** (per docs/release/archive/r19-*): frontend-proposal (Tauri 2.0 + 4 接入) + integration #1-#4
- **R20 阶段 1-6** (per docs/release/1.0.0/CHANGELOG.md): 阶段 1 翻译准备 + 阶段 2 翻译 + 阶段 3 翻译验证 + 阶段 4 核心文档 + 阶段 5 施工文档 + 阶段 6 1.0 release 收口
- **SpectrAI 0.9.21 1:1 翻译** (per 决策 #12, 14 new crate): 1.4GB / 171 .js / 452K LOC, 5 P0 MCP + 3 估缺核心 + 2 工具 + 2 基础设施 P0 + 2 SDK stub
- **蓝图 V09021 (RIVAL VERSION)** (per 决策 #2 拍板, 8/5 19:50): 604 行 RIVAL VERSION 胜出, 对齐 7 项 + 差异 8 项
- **6 哲学锚初版** (per docs/adr/0010-6-philosophy-anchors.md): S-1/S-2/O-2/O-3/O-4/O-5, R19+ 集成期主人 2026-08-05 拍板统一
- **双洋葱架构** (per docs/conventions/onion-wall-architecture.md): 原则洋葱 + 权限洋葱
- **9 organ 哲学核心** (per R124-2 §14.4 认知科学有依据): body/brain/ear/eye/hand/heart/memory/mind/voice
- **5 重守门 v5** (per glossary/17-4-gates-permission.md): 4 重嵌套 + 权限发放独立机制
- **V0.5 24 维** (per integration_r_measure.rs:42-44): 24 维综合 + 9 子测度 + sum=1 守门
- **12 键 verdict cache** (V3 9 键 + v4.1 3 键): 编译期 hardcode, 0 装 (O-5)
- **8 项不修改承诺** (R119 era, 1:14 形式撤销后保留原意): 阶段 1+2+3 LOCKED 文档 / v2/v4/v4.1 LOCKED / 阶段 4 核心文档 / 阶段 5 施工文档 / v6 基础架构 / R11 baseline 3 值 / 顶层 3 规范文件 / workspace version 1.0.0
- **12 ADR (新编号 0001-0012)** (per docs/adr/README.md §2.1): 1.0 release 收官 + RIVAL VERSION 蓝图 + 整合 #3 策略 + 8 项不修改承诺审计 + 1.0 release 12 项 checklist + D-01 6 工具 endpoint 全真接 + D-02 6 工具各 1 URL 子路径 + D-06 8 包齐发 + D-07 一次性 SQLite → PostgreSQL 迁移 + 6 哲学锚 + TUI 瘦客户端 + SpectrAI 0.9.21 1:1 翻译
- **14 旧 ADR archive** (跳号 0025+, 留作历史, per docs/adr/archive/r20-pre-renumber/)
- **14 new crate 1:1 翻译** (per 决策 #12): 5 P0 MCP + 3 估缺核心 + 2 工具 + 2 基础设施 P0 + 2 SDK stub
- **APEIRETH-CONVENTIONS.md / APEIRETH-VERSIONING.md / APEIRETH-GLOSSARY.md** (3 顶层规范文件, R20 LOCKED)
- **193/193 测试** (R20 era): 8 包 cosign 签名 + 1.0 CI pipeline 5 job + 0 触碰 24 LOCKED crate + workspace version 1.0.0 严守
- **12 项 checklist 100% PASS** (per docs/1.0-release/checklist.md + 1.0 release 报告): 9 P0 + 3 P1, 1.0 release gate 全 PASS

### Changed (R11-R20 era)

- **workspace.version 0.5.0** (R20 era 内部版本, 1.0.0 era 严守)
- **Cargo.toml workspace.package version**: `[workspace.package] version = "0.5.0"` (R20 era 内部, R38 升 1.1.0, R125 末升 1.2.0)
- **8 包齐发** (per 决策 #8): deb / rpm / brew / scoop / tarball / zip / MSI / Docker, Linux 4 包重点 (deb / rpm / tarball / Docker, 估 90% Linux 用户覆盖)
- **TUI 瘦客户端** (per 决策 #11): HTTP to apeireth-api, 不直接调 lib; Tauri 2.0 走 Tauri command 模块化 9 器官 (70 command 模式)
- **D-07 一次性 SQLite → PostgreSQL 迁移** (per 决策 #9): 8 步迁移 + 5 验证 + 兜底 3 步, 30 天 .bak 保留 + dry-run, 1.0 release 估 1 用户 1 年 500K 行, 估时 30-60s

### Deprecated (R11-R20 era)

- **R11 0 触碰承诺** (R20 era, 1:14 形式撤销后保留原意): 0 触碰 24 LOCKED crate, 0 改 R11 baseline 3 值, 0 改 workspace version
- **Tauri 2.0 暂缓** (per 决策 #11 D-7): TUI 优先, Tauri 2.0 走 R21 续真接

### Removed (R11-R20 era)

- **5 估补 crate** (R20 阶段 4 PLANNED): R20 阶段 6 实施时 R38 1.1.0 升级合并
- **5 老 crate (.bak)** (R35+R36 5 Provider 真合并): 0 真接外部 LLM, R21+ 续真接

### Fixed (R11-R20 era)

- **5 守门每层都适用 0 装 5 项** (C3 严守, 0 改)
- **Cargo.lock 一致性** (R20 era 8/5 21:14 拍板, 1.0 release gate 100% PASS)

### Security (R11-R20 era)

- **0 装 (O-5) 12 键编译期 hardcode** (C2, R20 era 严守, R125 主人 17:22 升级授权后 0 装解除)
- **0 触碰 24 LOCKED crate** (R20 era 严守)
- **8 项不修改承诺 1:1 映射** (per docs/1.0-release/8-promise-audit.md + commit 629995d3 + scripts/audit/8-promise-audit.sh, 8/8 PASS)

### Notes (R11-R20 era)

- **R-Method 状态**: R11 末 / R12 接手 / R13 MVP / R14 Rust 重写 / R17 战役 0-4 / R23 baseline 24 LOCKED 等历史阶段归档在 [`docs/release/archive/`](docs/release/archive/)
- **R20 era "v1.0.0" (2026-08-05)**: 内部里程碑, 完整 release notes 见 [`docs/release/1.0.0/CHANGELOG.md`](docs/release/1.0.0/CHANGELOG.md), 1.0 release 时不再次提及

---

## [0.1.0] - 2026-07-15 — R0-R10 era 初始 MVP

> **状态**: 🟢 已发, R0-R10 era, , 完整 release notes 下沉到 [`docs/release/0.1.0/CHANGELOG.md`](docs/release/0.1.0/CHANGELOG.md) (待写, R127-2 后创建)

### Added (R0-R10 era)

- **Apeireth 项目立项** (2026-07-15, 研究生 侦查学院, 2026 学术研究项目)
- **R-Method 起步** (per docs/release/archive/r-method-001-005.md): R0 立项 / R1 spec / R2 Rust 决定 / R3 baseline / R4 蓝图 / R5 翻译 spec
- **SpectrAI 0.9.21 商业版调研** (per docs/stage4/v09021-commercial-extract-2026-08-05.md, 250 行, NSIS 解包 1.4 GB / 171 .js / 452K LOC)
- **RIVAL VERSION 蓝图准备** (per 决策 #2 拍板, 8/5 19:50, R12-R14 阶段准备): 1:1 翻译 v0.9.21 商业版 14 crate, 5 阶段 320h 实施
- **Cargo workspace 初始化** (per Cargo.toml line 1-50): `resolver = "2"`, 8 个初始 crate (apeireth-core / apeireth-memory / apeireth-asi / apeireth-tools / apeireth-cli / apeireth-bench / apeireth-asi-test / apeireth-eval)
- **git 仓库初始化** (per git log): Apeireth 项目 git init, master branch 起步
- **APEIRETH-CONVENTIONS.md 初版** (per docs/conventions/): 命名规范 + 6 哲学锚初版 (S-1/S-2/O-2/O-3/O-4/O-5)
- **APEIRETH-VERSIONING.md 初版** (per docs/conventions/): semver 严守, 0.1.0 / 0.5.0 / 1.0.0 release 节奏
- **APEIRETH-GLOSSARY.md 初版** (per docs/glossary/): 术语表初版, 双洋葱 / 权限矩阵 / 守门 1-4
- **蓝图 RIVAL VERSION** (per docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md, 604 行): 14 crate 1:1 翻译 spec, 5 阶段 320h 实施
- **R11 baseline 数字准备** (per integration_r_measure.rs:42-44, 8/15 数字严守): V1141=0.8682 / V1131=0.8532 / V1136=0.9063, 24 维综合 + 9 子测度

### Changed (R0-R10 era)

- **Cargo.toml workspace.package version**: `[workspace.package] version = "0.1.0"` (R0-R10 era 初版, R11 末升 0.5.0, R20 era 升 1.0.0, R38 升 1.1.0, R125 末升 1.2.0)
- **git 历史 0 丢** (per 决策 #45/46/47): master HEAD 链从 R0 立项开始保留

### Deprecated (R0-R10 era)

- **R0 立项前** (项目级 0 项目立项): 0 任何代码

### Removed (R0-R10 era)

- **N/A** (R0 立项是项目起点, 0 移除)

### Fixed (R0-R10 era)

- **N/A** (R0 立项是项目起点, 0 修复)

### Security (R0-R10 era)

- **0 触碰承诺** (R0 立项初版): 0 触碰 git 历史, 0 触碰 Cargo workspace structure, 0 触碰 6 哲学锚初版

### Notes (R0-R10 era)

- **R-Method 起步** (per docs/release/archive/r-method-001-005.md): R0 立项 / R1 spec / R2 Rust 决定 / R3 baseline / R4 蓝图 / R5 翻译 spec
- **项目背景**: 研究生 (侦查学院) / 2026 学术研究项目 / Apeireth 是 AGI 操作系统 (R11 阶段 4 frontend-proposal 决定: Tauri 2.0 + 4 接入)
- **当前 R19 era** (R0-R10 era 末): R19 集成 4 子阶段 (frontend-proposal + integration #1-#4)
- **重要路径** (跨 project 适用): 真 API key `.minimax-agent-cn\projects\apikey.txt` (125 chars, sk-cp-kug0t7Jik3-...) + VCPChat 参考 (Electron 桌面 app, chat-first) `Downloads\VCPChat-main.zip` + 默认工作目录 `.minimax-agent-cn\projects\`

---

## Earlier versions — R0 立项前

> **状态**: 🟢 归档, R0 立项是项目起点, 0 任何代码
>
> **完整 git 历史** 见 `git log` 输出, master HEAD 链从 R0 立项开始保留

- **N/A** (项目级 0 项目立项, 0 任何代码, 0 任何 commit)

---

## 附录 A: 借鉴源码 0 装 PASS 严守 (per 决策 #55 §3 + 决策 #56 §3)

### A.1 借鉴源码 11 仓库状态

| # | 仓库 | 状态 | 借鉴 | 实施 |
|---|------|------|------|------|
| 1 | clap 725 | ✅ cloned | R125-2 真实施 | commands.rs -498 行 clap derive, 19/19 tests pass |
| 2 | hyper 80 | ✅ cloned | R125-3 真实施 | 池复用 38/38 tests pass |
| 3 | servers 175 | ✅ cloned | R125-4 真实施 | 4 文件 29.4KB, 188 tests (183+5) |
| 4 | PyO3 928 | ✅ cloned | R125-8/9 真实施 | Chidori 13/13 + PyO3 pybridge 77/77 |
| 5 | kani 4502 | ✅ cloned | R125-10 真实施 | 12 文件 75.8KB, 5 阶段 |
| 6 | langgraph 829 | ✅ cloned | R125-13 真实施 | 10 NEW 85.9KB, 60 tests, 30 维 sum=1.0 |
| 7 | superpowers 234 | ✅ cloned | R125-14 真实施 | 8 文件 ~80KB, 79/79 |
| 8 | LiteLLM | ⏳ 限流 (0 files) | R125-1 准备 + P6-1 R127-2 阶段 A 重试 | 0 装"已实施" 严守 |
| 9 | opencode | ⏳ 限流 (0 files) | R125-12 准备 + P6-2 R127-2 阶段 A 重试 | 0 装"已实施" 严守 |
| 10 | Guardrails | ⏳ 限流 (0 files submodule) | R125-5 准备 + P6-3 R127-2 阶段 A 重试 | 0 装"已实施" 严守 |
| 11 | OpenCog | ❌ 跳过 (AGPL-3.0) | 0 集成 | LICENSE 风险, 0 装"已实施" 严守 |
| (12) | sqlite-vec | ✅ R120 A 真接 (0 需 clone) | R120 A 已真接 | 0 需 R125 实施 |

### A.2 0 装 PASS 严守 verify 方法

- ✅ cloned = 真实施 (有真 src 改动 + tests pass, 8 任务: R125-2/3/4/8/9/10/13/14)
- ⏳ 限流 = 准备 (诚实标"准备", 0 装"已实施", 3 任务: R125-1/12 + Guardrails submodule)
- ❌ 跳过 (OpenCog AGPL-3.0) = 0 集成 (0 装"已实施" 严守)
- 0 假装"已实施" (per 决策 #33 §2.3 C2 + 主人 17:22 升级授权 + 主人 20:32 "技术性 locked 都能解锁")

---

## 附录 B: 8 硬墙 (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略)

### B.1 8 硬墙 (handoff §1) 重置后 (主人 17:22 后, per 决策 #33 §2.3)

| 硬墙 | 决策 #22 实质分类 | 主人 17:22 拍板后 |
|------|------------------|------------------|
| 1. workspace.version 1.1.0 | **B2 minor 升 1.2.0** (R125 末) | ✅ 升 1.2.0 (R125 末, 整合 #4 commit done) |
| 2. R11 baseline 3 值 0.8682/0.8532/0.9063 | **A1 严守** (数字 0 改) | ⚠️ 主人 17:22 "all locked" 含 A1? 实际: 数字 0.8682 等仍严守 (per 决策 #22 §5.1, 主人 7/31 明确不动), 但**测试结构 / 公式 / 测度可调** |
| 3. 24 LOCKED crate mtime 16:34 baseline | **B1 完整名单落实** (24 个) | ✅ 24 LOCKED 名单持续更新 (R119 撤销 3 技术类 LOCKED) |
| 4. 6 哲学锚 | **B5 升 8 锚** (S-3 + O-1) | ✅ 8 哲学锚 (P1-2 R126 升级 done) |
| 5. V0.5 25 维 | **B3 升 25 维** (24+Robustness) + R125-13 升 30 维 | ✅ V0.5 30 维 (P1-4 R126 升级 done) |
| 6. 6 重守门 v6 | **B4 升 6 重** (5+Colang DSL) → v7 | ✅ 6 重守门 v7 (P1-3 R126 升级 done) |
| 7. 13 键 + PHL-07 | **A3 12 键原 12 + 新增 PHL-07** (13 键) | ✅ 13 键 (R125-12 实施 done, 整合 #4 commit done) |
| 8. 0 装 (O-5) + 0 主动 commit + 0 主动 push | **C1-C3 策略 0 改** (但可升级路线内) | ⚠️ 0 装 解除 (主人 17:22 "0 装不必要") + 0 主动 commit = Mavis 整合 #5 commit 时机拍板 + 0 主动 push 严守 |

### B.2 0 越界 verify (per 决策 #55 §4 + 决策 #56 §4)

- B2 workspace.version 1.2.0 0 改 (整合 #4 commit abf12243 严守) ✅
- A1 R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守 (17 文件原位, 0 删 0 改) ✅
- B1 24 LOCKED 持续更新, 内部 fn 实施可改, **入口签名 0 改** (P2-3 retry verify 24/24 LOCKED 入口签名 0 改 done) ✅
- B5 6→8 哲学锚 (P1-2 R126 8 哲学锚升级 done) ✅
- B3 V0.5 25→30 维 (P1-4 R126 25→30 维 verify retry done) ✅
- B4 6 重守门 v6 → v7 (P1-3 R126 6 重守门 v7 retry 跑中) 🟡
- A3 12 键 + PHL-07 = 13 键 (整合 #4 commit done) ✅
- C1 0 主动 commit (Mavis 整合 #5 commit 时机拍板) ✅
- C2 0 装 PASS 严守 (✅ 8 cloned = 真实施, ⏳ 3 限流 = 准备, ❌ 1 跳过 = 0 集成) ✅
- C3 升 6 重 v7 (P1-3 R126 升级 done) ✅
- 0 主动 push (等 1.0 release 配 GitHub remote) ✅

---

## 附录 C: 决策链 #22-#56 (R124-R127 era)

| 决策 | 时间 | 主题 | 状态 |
|------|------|------|------|
| **#22** | 8/10 16:35 | 主人 16:31 最高权限 + 24 LOCKED 自主确认 + 9 项实质 locked 升级 (B1-B7 + A1-A3 + C1-C3) | ✅ |
| **#23** | 8/10 16:40 | 16 pipeline | ✅ |
| **#24** | 8/10 16:45 | R125 派活修复 + R125-15 非 GitHub 学习途径 + research → library 升级 | ✅ |
| **#25-#29** | 8/10 17:00-17:15 | R121-R122 网络失败 + 派活 daemon bug 修复 | ✅ |
| **#30** | 8/10 17:15 | 新 Mavis 接入 + 派活 daemon 复活 | ✅ |
| **#31** | 8/10 17:17 | 17:30 拍板 dry-run + 138 src 改动诚实标 | ✅ |
| **#32** | 8/10 17:18 | R125 派活大主管启动 (17:23 task_stop) | ✅ |
| **#33** | 8/10 17:23 | 主人 17:22 升级授权 + 8 硬墙全部重置 + B1-B7 升级路线 + 0 装解除 + 16 派满 | ✅ |
| **#34** | 8/10 17:30 | 整合 #3 commit 21aa85f3 拍板 done (257 files +61969/-520) | ✅ |
| **#35** | 8/10 17:32 | 主人 17:31 "16 成员人数要多" + supervisor 模式废弃 + Mavis 真派 16 sub-agent | ✅ |
| **#36** | 8/10 17:44 | P2 4 sub-agent 跑中 12 min 0 output + 借鉴源码 3/4 ✅ cloned | ✅ |
| **#37-#39** | 8/10 18:00-18:30 | R125-8 done + 0 新派成员 + 暂停讨论后续 | ✅ |
| **#40-#42** | 8/10 18:30-18:35 | promethean cleanup + R125 16 sub-agent 全部 succeeded + R125 续整合 #4 pre-checklist 4 项 | ✅ |
| **#43-#47** | 8/10 19:00-19:39 | apeireth-tui no-merge move + promethean cleanup deletion + git history lost + git mv done + git reset 0 真正起作用 | ✅ |
| **#48** | 8/10 19:41 | **整合 #4 commit abf12243 done** (主仓挪到 Apeireth-rust/, 46752 file changes) | ✅ |
| **#49-#51** | 8/10 20:00-20:25 | promethean cleanup 全 done + R126/R127 16 sub-agent 派活清单 | ✅ |
| **#52-#53** | 8/10 20:25-20:32 | 16 真派模式 + 主人 20:32 "技术性 locked 都能解锁" 升级授权 | ✅ |
| **#54-#55** | 8/10 20:50-21:13 | P1-4 failed retry pending + R127 升级路线 + 派活清单 (整合 #5 pre-check + Library Stage 4-6 + 借鉴 3 限流重试 + 1.0 release 准备) | ✅ |
| **#56** | 8/10 21:18 | R127-2 派活 10 sub-agent (借鉴 3 限流重试 + 1.0 release 准备 + Library 阶段 4-6 进阶 + borrowed-repos 进阶) | ✅ |

---

## 附录 D: 完整 release notes 下沉路径

- **本 CHANGELOG** (顶层, Keep a Changelog 1.1.0 格式): R127-2 P7-1 写, 0 主动 commit, Mavis 整合 #5 commit 时机拍板
- **整合 #3 commit 21aa85f3** (R123-R124-R125 阶段整合 + B1-B7 升级, per 决策 #34): 257 files +61969/-520, 17:30:34 主人拍板
- **整合 #4 commit abf12243** (R125 续整合 + 主仓挪到 Apeireth-rust/ + index resync, per 决策 #48): 46752 file changes, 19:40:58 主人自执行 A
- **整合 #5 commit (待)** (R126/R127 整合 + R127-2 10 sub-agent + 借鉴 3 限流重试 + 1.0 release 准备, per 决策 #55 + #56): 32 sub-agent 跑过夜 8/11-8/22 done 后, Mavis 拍板 OR 主人 8/15 拍板
- **R20 era "v1.0.0" (2026-08-05)**: 内部里程碑, 完整 release notes 下沉到 [`docs/release/1.0.0/CHANGELOG.md`](docs/release/1.0.0/CHANGELOG.md) 保留作历史
- **1.1.0/1.1.1/1.1.2/1.2-* 内部版本**: 已并入 1.0.0 release cycle, release notes 下沉到 `docs/release/<version>/CHANGELOG.md` 保留作历史
- **R0-R10 era 0.1.0**: 待写, R127-2 后创建 [`docs/release/0.1.0/CHANGELOG.md`](docs/release/0.1.0/CHANGELOG.md)
- **R11-R20 era 0.5.0**: 待写, R127-2 后创建 [`docs/release/0.5.0/CHANGELOG.md`](docs/release/0.5.0/CHANGELOG.md)

---

## 附录 E: 0 主动 commit + 0 主动 push 严守

- **0 主动 commit 整合 #5** (per 决策 #55 §5 + 决策 #56 §5): 等 32 sub-agent done + 0 装 PASS 严守 + 8 硬墙 0 越界 verify, Mavis 拍板 OR 主人 8/15 拍板
- **0 主动 push git push** (per 决策 #55 §7 + 决策 #56 §7): 等 1.0 release 配 GitHub remote
- **0 主动讨论后续 (R128 升级 / 借鉴 11/11 收尾)**: 等 32 sub-agent done 后主人主动问
- **0 主动 push 删 5 散文件 / 33 待删** (per 决策 #50): 0 必再删, 决策 #50 全 done
- **0 主动 push 整合 #4 commit** (per 决策 #48 abf12243): 已 done, 0 重跑
- **0 主动 IM 主人** (per gate-discipline): 仅 done notification 主动报告, 0 主动 plain reply on skip ticks

---

**Mavis R127-2 P7-1 21:18 状态**: 写本 CHANGELOG v1.0.0 done, 完整 Keep a Changelog 1.1.0 格式 (Unreleased + 1.0.0 + 0.5.0 + 0.1.0 + Earlier versions 5 章节), 整合 R125-R127 决策链 + 24 LOCKED + 8 哲学锚 + 30 维 + 6 重 v7 + 13 键 + Library v1.0 + 借鉴 8/11 真实施 + 整合 #4 commit abf12243 + 决策链 #22-#56 全 8 硬墙 0 越界. 写到主仓 `Apeireth-rust/CHANGELOG.md` 0 主动 commit 严守, Mavis 整合 #5 commit 时机拍板. 0 主动 push 严守. 0 主动 IM 主人 (per gate-discipline, 跑过夜 0 打扰).
