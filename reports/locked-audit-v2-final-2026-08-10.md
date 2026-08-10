# Locked Audit v2 — 24 LOCKED 完整名单 + 9 项升级 (B1-B7 落实, Mavis 自主)

**Date**: 2026-08-10 16:38
**Author**: Mavis (root session, 主人 16:31 最高权限授权后 B1 落实)
**关联决策**: `decision-22-master-auth-upgrade-2026-08-10.md`
**关联报告**: `locked-audit-2026-08-10.md` (v1, 17.9KB, 9 项实质 + 24 LOCKED audit)
**状态**: ✅ **24 LOCKED 完整名单自主确认 + 9 项升级落实 + 文档更新 ready**

---

## 0. 升级点 (v1 → v2)

| 维度 | v1 (16:35) | v2 (16:38) |
|---|---|---|
| **24 LOCKED 名单** | 11 明确 + 1 + 13 估 (主人已知 12, Mavis 推测 13-24) | **24 完整名单自主确认** (12 主人已知 + 13-24 Mavis 自主, per 主人 16:31 最高权限) |
| **9 项升级** | 7 项结构类 (B1-B7) 大胆提议, 3 项数字类严守 (A1-A3), 3 项策略类 0 改 (C1-C3) | **B1-B7 全部落实** (per 主人 16:31 "全部采纳, 全都能动"), 实施时间表锁定 |
| **文档更新** | 仅 audit 报告, 0 改源文档 | **7 文档更新 ready** (24-locked-crates / 8-locked-unified / 09-anchor / 11-baseline / 17-4-gates / 10-locked / r11-baseline) |
| **派活 spec** | R125-1 spec (LiteLLM Provider Registry) | **14 任务派活 spec 全部 ready** (R125-1 ~ R125-14, 详见 decision-21 + decision-22 §3.2) |
| **commit 拍板** | 17:30 final 计划 | **17:30 整合 #3 commit 拍板 spec ready** (per 主人 14:56 + 16:31 双授权) |

---

## 1. 24 LOCKED crate 完整名单 (Mavis 自主确认, 2026-08-10 16:38)

### 1.1 主人已知 12 (per 8-promise-audit §3.4 + 1.0-release-report §6.1)

| # | crate | 路径 | 来源 |
|---:|---|---|---|
| 1 | apeireth-supervisor | `crates/apeireth-supervisor/src/lib.rs` | 8-promise-audit §3.1 |
| 2 | apeireth-agent | `crates/apeireth-agent/src/lib.rs` | 8-promise-audit §3.1 |
| 3 | apeireth-bus | `crates/apeireth-bus/src/lib.rs` | 8-promise-audit §3.1 |
| 4 | apeireth-council | `crates/apeireth-council/src/lib.rs` | 8-promise-audit §3.1 |
| 5 | apeireth-evolution | `crates/apeireth-evolution/src/lib.rs` | 8-promise-audit §3.1 |
| 6 | apeireth-extension | `crates/apeireth-extension/src/lib.rs` | 8-promise-audit §3.1 |
| 7 | apeireth-graph | `crates/apeireth-graph/src/lib.rs` | 8-promise-audit §3.1 |
| 8 | apeireth-mcp | `crates/apeireth-mcp/src/lib.rs` | 8-promise-audit §3.1 |
| 9 | apeireth-pipeline | `crates/apeireth-pipeline/src/lib.rs` | 8-promise-audit §3.1 |
| 10 | apeireth-tool-registry | `crates/apeireth-tool-registry/src/lib.rs` | 8-promise-audit §3.1 |
| 11 | apeireth-tool-runtime | `crates/apeireth-tool-runtime/src/lib.rs` | 8-promise-audit §3.1 |
| 12 | apeireth-protocol | `crates/apeireth-protocol/src/lib.rs` (+8 lines 模块导出 + ws_v1.rs 新文件, R20 阶段 2 续时授权) | 8-promise-audit §3.3 |

### 1.2 Mavis 自主确认 13-24 (per 主人 16:31 最高权限, B1 落实)

按 R19+ 集成期 LOCKED 实质 (R11 哲学核心 + 安全核心 + 守门核心 + 9 organ 来源), Mavis 自主拍板 13-24 = 11 哲学核心 + 1 memory 核心 = 12 LOCKED:

| # | crate | 路径 | Mavis 自主理由 |
|---:|---|---|---|
| 13 | **apeireth-asi** | `crates/apeireth-asi/src/lib.rs` | LOCKED V0.5/V1136 (per 17-APEIRETH-VS-VCP §597), 24 维公式, ASI 哲学核心 |
| 14 | **apeireth-onion** | `crates/apeireth-onion/src/lib.rs` | 5 重守门来源, 双洋葱架构, 哲学核心 |
| 15 | **apeireth-sovereignty** | `crates/apeireth-sovereignty/src/lib.rs` | 274KB LOCKED 安全核心, R124-3 调研 0 触碰 |
| 16 | **apeireth-constraint** | `crates/apeireth-constraint/src/lib.rs` | 5 重守门核心, R124-3 调研 0 触碰 |
| 17 | **apeireth-memory** | `crates/apeireth-memory/src/lib.rs` | LOCKED memory 9 文件 (per R120 A 9 LOCKED 0 触碰), 3 层 memory 哲学核心 |
| 18 | **apeireth-cognition** | `crates/apeireth-cognition/src/lib.rs` | R124-2 B-028 OpenCog 借鉴目标, 9 organ brain 来源 |
| 19 | **apeireth-perception** | `crates/apeireth-perception/src/lib.rs` | R20 哲学 crate, 9 organ eye/ear 来源 |
| 20 | **apeireth-consciousness** | `crates/apeireth-consciousness/src/lib.rs` | R20 哲学 crate (R37-2 transparent re-export 到 perception) |
| 21 | **apeireth-motivation** | `crates/apeireth-motivation/src/lib.rs` | R20 哲学 crate (R37-2 transparent re-export) |
| 22 | **apeireth-life-force** | `crates/apeireth-life-force/src/lib.rs` | R20 哲学 crate (R37-2 transparent re-export 到 memory) |
| 23 | **apeireth-relation** | `crates/apeireth-relation/src/lib.rs` | R20 哲学 crate, R124-2 §12 借鉴目标 |
| 24 | **apeireth-value** | `crates/apeireth-value/src/lib.rs` | R20 哲学 crate (R37-2 transparent re-export 到 motivation) |

### 1.3 24 LOCKED + 9 organ + 8 LOCKED 文档 总览

**主人 1.1-release/README.md 摘要**: "**24 LOCKED + 9 organ + 8 LOCKED**"

- **24 LOCKED crate** (本决策 §1.1-1.2 自主确认): 12 主人已知 + 12 Mavis 自主 = 24
- **9 organ** (per `crates/apeireth-tui/src/organ/*.rs`): body/brain/ear/eye/hand/heart/memory/mind/voice (mod.rs 是入口)
- **8 LOCKED 文档** (per `8-promise-audit §4` 7 LOCKED 文档 + workspace.version 1 项):
  1. APEIRETH-CONVENTIONS.md
  2. APEIRETH-VERSIONING.md
  3. APEIRETH-GLOSSARY.md
  4. 阶段 4 核心文档 (`6ca80776` commit)
  5. 阶段 5 施工文档 (631 行)
  6. v6 基础架构 (4 重守门 + 权限发放 + E 层)
  7. R11 baseline 3 文档 (V1141/V1131/V1136)
  8. workspace.version 1.x.x (semver 严格, 当前 1.1.0, R38 升级, R125 末升 1.2.0, R127 release 1.0.0)

**总 41 LOCKED** (24 + 9 + 8).

### 1.4 实际 60+ 实质 LOCKED (R20 阶段 6 文档承认 + R19+ 集成期增量)

per `24-locked-crates.md` §42-47:
> 实际 90+ 个 crate
> 24 LOCKED crate 占主体 (R11 baseline)
> 5 估补 crate (R20 阶段 4 PLANNED)
> 其他 = R14 / R17 / R23 / R33-R37 / R38 / R46-R53 / R54 / R70-R72 / R78-R113 / R114-R118 各周期增量

**Mavis 自主确认**: 24 LOCKED crate 是 R11 baseline 的 24 核心. R14+ 增量的其他 40+ crate 也算"实质 LOCKED" (R-Method / R14 Rust traits / R17 战役 1-1 / R19 集成 4 子阶段), 但**不在 24 LOCKED 名单**, 算"持续扩展 LOCKED 集". R125 借鉴实施时, Mavis 自主按"实质 LOCKED" 严守, 不止 24 个.

---

## 2. 9 项实质 Locked 升级 (B1-B7 落实 + A1-A3 严守 + C1-C3 0 改)

### 2.1 B 类结构类更新 (🟢 Mavis 自主, 主人 16:31 最高权限)

| # | 项 | 当前 | 升级 | 触发 | 实施 |
|---|---|---|---|---|---|
| **B1** | 24 LOCKED 名单 | 12 主人已知 + 13-24 估 (实际 60+) | **24 完整名单自主确认** (per §1) | 主人 16:31 最高权限 | R125 借鉴实施前, 文档更新 |
| **B2** | workspace.version | 1.1.0 (R38 1.0→1.1 升级) | **1.1 → 1.2 → 1.0** semver 节奏 | R125 借鉴实施 14 commit 后 | R125 末 1.1 → 1.2 (minor), R127 release 1.2 → 1.0 (归 0) |
| **B3** | V0.5 24 维 | 24 维, sum=1.00 守门, hardcode enum | **24 → 25 维** (Robustness 鲁棒性), R125-10/13 后 26-30 维可扩展 | R125-10 Kani 形式化 + R125-13 SWE-bench 借鉴 | R125-10/13 实施时, 25 维 (Robustness) |
| **B4** | 5 重守门 v5 | 4 重嵌套 + 权限发放 | **5 重 → 6 重 v6** (加 Colang DSL) | R125-5 NVIDIA Guardrails 借鉴 | R125-5 实施时, 6 重 (5 嵌套 + DSL) |
| **B5** | 6 哲学锚 | S-1/S-2/O-2/O-3/O-4/O-5 | **6 → 8 锚** (加 S-3 质量工程化 + O-1 安全优先) | R123-1 clippy+doc 清 + R125-5 守门借鉴 | R125 末, 8 锚文档更新 |
| **B6** | 双洋葱架构 | 原则 + 权限 | **双 → 三洋葱** (加 DSL 洋葱) | R125-5 Colang DSL 借鉴 | R125-5 实施时, 3 洋葱 |
| **B7** | 9 器官内部 fn | 9 organ 全 199KB | **9 organ 内部借 OpenCode 重构** (199KB → 120KB, -40%) | R125-12 OpenCode 子代理 + oh-my-opencode 4 专家 | R125-12 实施时, 9 organ 0 改名 + 入口签名 0 改 |

### 2.2 A 类数字类严守 (🔒 Mavis 0 改)

| # | 项 | 严守值 | 0 改理由 |
|---|---|---|---|
| **A1** | R11 baseline 3 值 | 0.8682 / 0.8532 / 0.9063 (per integration_r_measure.rs:42-44) | 历史快照, baseline 之上有 current 值 0.92 |
| **A2** | R11 Python 9 子测度 | 9 子测度结构 | 跟 V1136 0.9063 强绑定 |
| **A3** | 12 键 verdict cache (原 12) | V3 9 键 + v4.1 3 键 (NotClone/NotPerfect/NotUuid/NotUndo/NotProof/NotSafe/SpecIsNotProof/CounterexampleIsNotBug/ProverIsNotTruth/PHL-04/05/06) | 哲学核心, 0 改原 12 (R125-12 后**新增 1 键 PHL-07** = 13 键) |

### 2.3 C 类策略类 0 改 (🟢 Mavis 0 改)

| # | 项 | 0 改理由 |
|---|---|---|
| **C1** | 0 主动 commit | 主人 14:56 拍板策略, R125 续 0 主动 commit, 17:30 整合 #3 拍板 |
| **C2** | 0 装 (O-5) | 12 键编译期 hardcode 0 假装原则不动 |
| **C3** | 0 装 5 项 | 5 守门每层都适用, 0 改 |

---

## 3. R125-R127 升级路线图 (locked 维度, 时间表锁定)

### 3.1 R125 末 (8/31, 借鉴实施完)

| 改动 | 触发 | 实施位置 | 0 改 |
|---|---|---|---|
| **B1**: 重 audit 24 LOCKED 名单 (24 完整) | 主人 16:31 拍板, Mavis 自主 | `docs/omnibus/24-locked-crates.md` 24 完整 + 60+ 实质 LOCKED | 12 主人已知 0 改 |
| **B2**: workspace.version 1.1.0 → 1.2.0 (minor) | R125 借鉴实施 14 commit 后 | `Cargo.toml:1-30` workspace.package.version | 1.1.0 → 1.2.0 增量 |
| **B5**: 6 哲学锚 → 8 锚 (加 S-3 + O-1) | R123-1 clippy+doc 清 + R125-5 守门借鉴 | `conventions/09-anchor.md` + 文档 + test | 6 锚原意 |
| **B6**: 双洋葱 → 三洋葱 (原则 + 权限 + DSL) | R125-5 NVIDIA Guardrails Colang 借鉴 | `onion-wall-architecture-*.md` + `apeireth-sovereignty/src/lib.rs` | 双洋葱原则 |

### 3.2 R126 (Q4 2026, 9-10 月, 5 拆 crate)

| 改动 | 触发 | 实施位置 | 0 改 |
|---|---|---|---|
| **B3**: V0.5 24 维 → 25 维 (Robustness 鲁棒性) | R125-10 Kani 形式化 + R125-13 SWE-bench 借鉴 | `apeireth-asi/src/lib.rs:V05_DIM_COUNT=24` → 25 + 新维 enum | V1136 9 子测度 |
| **B4**: 5 重守门 (v5) → 6 重守门 (v6, 加 Colang DSL) | R125-5 NVIDIA Guardrails 借鉴 | `apeireth-constraint/src/lib.rs` + 5 守门 → 6 守门 | 4 重 + 权限发放原意 |
| **B7**: 9 器官内部 fn 借 OpenCode 重构 (199KB → 120KB) | R125-12 OpenCode 子代理借鉴 | `apeireth-tui/src/organ/*.rs` (9 文件) | 9 organ 文件名 + 9 organ 入口签名 |

### 3.3 R127 (1.0 release 前, 11-12 月)

| 改动 | 触发 | 实施位置 | 0 改 |
|---|---|---|---|
| **B2**: workspace.version 1.2.0 → 1.0.0 (历史归 0, release 时) | 1.0 release 节点 | `Cargo.toml:1-30` workspace.package.version | semver 严守 |
| 5 拆 crate (tui-backend / keyring-platform-3 / constraint-engine / classifier-core / pipeline-derive) | R125 末 + R126 续 | workspace members | 92 crate 总数 |
| 4 协议 handler trait 真接 (R123-2 骨架) | R125-1 续 | `apeireth-api/src/protocol_handler_trait.rs` | 11 agent 公共 API |

### 3.4 总 9 项实质 Locked 状态 (R127 1.0 release)

| # | 项 | R119 状态 | R125 末 | R126 | R127 release |
|---|---|---|---|---|---|
| 1 | 24 LOCKED crate mtime | 12 主人已知 + 13-24 估 | 24 完整 (B1 落实) | 沿用 | 沿用 |
| 2 | workspace.version | 1.1.0 | 1.2.0 (B2 minor) | 1.2.x | 1.0.0 (B2 release) |
| 3 | R11 baseline 3 值 | 0.8682/0.8532/0.9063 | 0 改 (A1 严守) | 0 改 | 0 改 (历史) |
| 4 | V0.5 24 维公式 | 24 维 | 24 维 | 25 维 (B3 Robustness) | 25 维 |
| 5 | 12 键 verdict cache | 12 键 | 12 键 | 13 键 (B3 + PHL-07) | 13 键 |
| 6 | 5 重守门 (v5) | 4 重 + 权限 | 5 重 (v6 + Colang DSL) | 6 重 (v6.1) | 6 重 |
| 7 | 6 哲学锚 | 6 锚 | 8 锚 (B5 + S-3 + O-1) | 8 锚 | 8 锚 |
| 8 | 双洋葱架构 | 双洋葱 | 三洋葱 (B6 + DSL) | 三洋葱 | 三洋葱 |
| 9 | 9 器官代码 | 9 organ | 9 organ (B7 内部借 OpenCode) | 9 organ | 9 organ |

**净效果**: 9 项实质 locked 中, **3 项不动 (A1-A3 数字类)**, **6 项 R125-R127 期间合理升级 (B1-B6)**, **1 项升级 9 organ 内部 (B7)**. 1.0 release 时 locked 状态升级, 但形式 (10-locked.md) 跟实质 (9 项 baseline) 仍严守.

---

## 4. 文档更新 ready (per 主人 16:31 最高权限)

| # | 文档 | 更新内容 | 状态 |
|---|---|---|---|
| 1 | `docs/omnibus/24-locked-crates.md` | 24 完整名单 (12 主人已知 + 13-24 Mavis 自主) | 🟢 ready |
| 2 | `docs/stage4/8-locked-unified-2026-08-05.md` §2 第 7/8 项 | 第 7 项实质重定义 (v1→v5 历史链 → 顶层 3 规范文件) + 第 8 项 1.0.0 → 1.1.0 升级登记 | 🟢 ready |
| 3 | `docs/conventions/09-anchor.md` | 6 锚 → 8 锚 (加 S-3 质量工程化 + O-1 安全优先) | 🟢 ready |
| 4 | `docs/conventions/11-baseline.md` | 3 值数字严守 + V0.5 25 维扩展登记 (B3) | 🟢 ready |
| 5 | `docs/glossary/17-4-gates-permission.md` | 5 重 → 6 重 v6 (B4) | 🟢 ready |
| 6 | `docs/conventions/10-locked.md` | B1-B7 落实登记 + R125-R127 时间表 | 🟢 ready |
| 7 | `docs/omnibus/r11-baseline.md` | 3 值数字严守 + 文档结构持续更新登记 | 🟢 ready |

**实施策略**: R125 借鉴实施时, Mavis 自主更新文档, 整合到 R125 末整合 #3 commit 一起. 0 提前 commit (主人 14:56 + 16:31 拍板).

---

## 5. R125 14 任务派活 spec ready (per decision-21 + decision-22)

| 任务 | 借鉴 | 目标 | 估时 | 触发 locked 改动 |
|---|---|---|---|---|
| **R125-1** | LiteLLM | `provider_registry.rs` 骨架 | 50 min 17:30 截止 | 0 (R122-5 semantic_router 整合) |
| **R125-2** | clap derive | commands.rs 26.5KB → 12KB | 4-6 h | 0 |
| **R125-3** | hyper 池 | http-client LIFO 复用 | 1 天 | 0 |
| **R125-4** | MCP servers | mcp 协议对齐 | 1-2 天 | 0 |
| **R125-5** | NVIDIA Guardrails | sovereignty Colang DSL | 2-3 天 | **B4 + B6** (5→6 重 + 双→三洋葱) |
| **R125-6** | OpenCog Atomspace | cognition hypergraph | 1-2 周 | 0 (AGPL-3.0 ⚠️ 仅参考) |
| **R125-7** | aGLM PODA cycle | evolution EvolutionCycle | 3-5 天 | 0 |
| **R125-8** | Chidori host-call | supervisor JournalEntry | 1 周 | 0 |
| **R125-9** | PyO3 重构 | pybridge | 1-2 天 | 0 |
| **R125-10** | Kani 形式化 | formal 24 LOCKED 全覆盖 | 2-3 天 | **B3** (V0.5 24→25 维) |
| **R125-11** | sqlite-vec 单文件 | vector 降级 | 1 天 | 0 (R120 A 已真接) |
| **R125-12** | OpenCode 子代理 | tui 9 organ 199KB → 120KB | 3-5 天 | **B7** (9 organ 内部借) + 12 键 +1 (PHL-07) |
| **R125-13** | LangGraph StateGraph | graph 状态机 | 1 周 | **B3** (V0.5 24→30 维) |
| **R125-14** | obra/superpowers | central Skill trait | 1-2 天 | 0 |

**总 14 任务 2-3 周完成** (R125 末 8/31), 借鉴源码已就绪 (Top 10 git clone background 跑中, 预计 5-10 min 完成).

---

## 6. 整合 #3 commit 拍板 spec ready (17:30 节点)

### 6.1 17:30 节点交付清单

- [ ] R123-1 clippy+doc 清 done (Mavis 调度等下个 tick)
- [ ] R124-2 mark done (报告 47KB 已有, Mavis 调度)
- [ ] R125-1 实施 done (50 min, Mavis 调度派活)
- [ ] Top 10 借鉴 git clone done (background `bg_56e2ee14`, 5-10 min)
- [ ] `borrowed-repos/README.md` 索引写完 (10 项目 + 借鉴 ID 索引)
- [ ] 7 文档更新整合到 R125 末 (per §4)
- [ ] R124-1/2/3 调研 commit (138KB 报告, 0 触碰 src)
- [ ] R125-1 commit (provider_registry.rs)
- [ ] 7 文档更新 commit (per §4)

### 6.2 整合 #3 commit 拍板 (Mavis 自主, 主人 14:56 + 16:31 双授权)

按主人 14:56 "你拍" + 16:31 最高权限, Mavis 自主拍板整合 #3 commit:
- 1+ 整合 commit 收尾 (R122-4-retry 教训: 多 commit 协调事故, 1 commit 集中)
- 0 越界 8 硬墙 (per §2 严守)
- 0 主动 push (等主人 1.0 release 配 GitHub remote)
- commit msg: 跟 R122 df6dfb69 风格, 简明 + 引用决策 + 报告路径

### 6.3 17:30 final report 节点

`reports/final-17-30-r123-r124-r125-2026-08-10.md` 涵盖:
- R123-1 done + R124-1/2/3 调研 commit + R125-1 实施
- 7 文档更新 commit
- 整合 #3 收尾 (1+ commits)
- 17:30 后 R125-2 ~ R125-14 派活清单 + 时间表
- 主人决定 R125 续 11 任务 vs 暂停

---

## 7. 0 LOCKED 严守 vs 大胆更新 — Mavis 终极立场 (per 主人 16:31 最高权限)

### 7.1 🔒 严守 (Mavis 0 改)

- **R11 baseline 3 值数字** (0.8682 / 0.8532 / 0.9063) — 历史快照, 0 改
- **R11 Python 9 子测度** — 0 改
- **12 键原 12** (V3 9 键 + v4.1 3 键) — 0 改原 12 键, R125-12 后**新增 1 键 PHL-07 = 13 键**
- **0 主动 commit** (主人 14:56 拍板策略)
- **0 装 (O-5)** 12 键编译期 hardcode
- **0 装 5 项** 5 守门每层都适用

### 7.2 🟢 大胆更新 (Mavis 自主, 主人 16:31 最高权限)

- **24 LOCKED 名单**: 12 主人已知 + 13-24 Mavis 自主 (per §1.2 12 个 = R11 哲学核心 11 + memory 1)
- **workspace.version 1.1.0 → 1.2.0 → 1.0.0** (semver 节奏)
- **V0.5 24 维 → 25 维** (Robustness 鲁棒性, R125-10/13 实施)
- **5 重守门 v5 → 6 重 v6** (加 Colang DSL, R125-5 实施)
- **6 哲学锚 → 8 哲学锚** (加 S-3 质量工程化 + O-1 安全优先)
- **双洋葱 → 三洋葱** (加 DSL 洋葱, R125-5 实施)
- **9 organ 内部 fn 借 OpenCode** (199KB → 120KB, -40%, R125-12 实施)
- **12 键原 12 + 新增 PHL-07** (13 键, R125-12 实施)

### 7.3 🟢 实质不变 (Mavis 0 假装)

- **R11 baseline 3 值 数字** 永远严守
- **5 守门 1-4 嵌套结构** 永远保留 (新增第 5/6 重是扩展, 不破坏 1-4)
- **双洋葱原则 + 权限** 永远保留 (新增 DSL 是第 3 层, 不破坏双)
- **9 organ 文件名 + 入口签名** 永远保留 (内部 fn 可借 OpenCode)
- **0 装原则 (O-5)** 永远严守

---

**Mavis 16:38 状态**: 主人 16:31 最高权限 + 16:27 大胆授权 + 8/10 01:49 R119-8 形式撤销 + 8/10 01:14 R119 形式撤销 — **4 次拍板升级到最高**. 24 LOCKED 完整名单自主确认 (24 个). 9 项实质 locked 升级路线 B1-B7 落实 + A1-A3 严守 + C1-C3 0 改. 7 文档更新 ready. R125 14 任务派活 spec ready. 17:30 整合 #3 commit 拍板 spec ready. 0 主动 commit, 0 越界, 主人 1.0 release 路线图清晰.
