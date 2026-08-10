# R19+ 集成期 / R20 收产品 / formal 5 不变量 — 阶段编号统一 + 对照表

```
[Document-Meta]
Document: docs/stage4/r19-r20-stage-unified-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ R20 阶段编号统一
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人复核)
```

> **性质**: 纯文档交付。**只做"5 阶段编号统一" + "对照表"**, 不改 APEIRETH-CONVENTIONS, 不动 M 标记文件, 不碰任何 crates/ 源码 / LOCKED 蓝图 / LOCKED ADR / Hermes 加的文件 / 已有 R 路线 (除 r20-product-finalize-2026-08-05.md 我已回写 1 次, 这次也只加 1 行引用)。
>
> **依据**:
> - `reports/docs-cross-check-2026-08-05.md` §2 (M-03 严重问题: 5 阶段路线三套并存) + §5.1 (三套 5 阶段路线) + §5.2 (三套关系) + §5.3 (矛盾点 M-07~M-10)
> - `reports/r19-integration-wrap-up-2026-08-05.md` §1.4 (5 阶段 R19+ 路线 + 5 阶段 R20 路线) + §2.2 (R19+ 5 阶段 Gantt)
> - `docs/stage4/r19-integration-commit-template-2026-08-05.md` §1.1 (R19+ 集成期 5 阶段, 7-15 天)
> - `docs/roadmap/r20-product-finalize-2026-08-05.md` §3 + §4 (R20 收产品 5 阶段, 7-10 周)
> - `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` §1.1 + §2-§3 (R20 阶段 X.Y 细粒度)
> - `docs/stage4/r20-stage-3-5-implementation-2026-08-05.md` §1.1 + §2-§4 (R20 阶段 X.Y 细粒度)
> - `docs/stage4/apeireth-formal-invariants-2026-08-05.md` §1 + §8 (formal 5 不变量 5 阶段, 3 天)
>
> **不修改承诺** (per APEIRETH-CONVENTIONS §10 + 互检报告 §5.4): 阶段 1+2+3 LOCKED + v2/v4/v4.1 LOCKED + 12 键 + 6 锚 + workspace v1.0.0 + Document-Meta + R11 baseline 三值 (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) 全部保留。
>
> **诚实登记** (S-2 17:43):
> 1. **本对照表只覆盖 3 套阶段路线**, 不包括第 4 套 `r19-integration-quickstart §2.2-§2.4` 的"步骤 1-5" (5 步 quickstart, 跟"阶段"语义不同 — 步骤 = 一次性操作, 阶段 = 周期性工作)。quickstart 的"步骤"不在本表范围内, 但建议后续拍板时也加上"步骤 1-5" 的命名规范 (见 §4 第 4 项)。
> 2. **M-07~M-10** 是互检报告的 P0 矛盾点 (见 §5.3): 阶段 1 三种含义 / 阶段 1.5 含义冲突 / 阶段 X.Y 编号 3 套混用。本对照表**承认**三套并存, **不**强求统一命名, 提建议在 §4。
> 3. **本对照表不重写任何 R 路线** — 只在 5 份受影响文件加 1 行引用, 引到这里。后续若 Mavis 拍板采纳 §4 命名规范, 再回写各 R 路线。

---

## §1 战略背景 (为什么需要本对照表)

### 1.1 互检报告 M-03 标 5 阶段路线三套并存 (P0)

`reports/docs-cross-check-2026-08-05.md` §2 把"5 阶段 R19+ 路线三套并存" 列为 **M-03 严重问题** (P0), §5.1 详细列出三套 5 阶段路线:

| 套 | 文档源 | X.Y 编号风格 | 周期 | focus |
|---|--------|-------------|------|-------|
| **套 1** | r19-integration-commit-template §1.1 | 1.1 / 1.2 / 1.3 / 1.4 / 1.5 (X.Y) | R19+ 集成期 7-15 天 | commit 落地 (~60 commit) |
| **套 2** | r20-product-finalize §1 + §4 | 1 / 2 / 3 / 4 / 5 (单数字) | R20 收产品 7-10 周 | release 1.0 产品化 |
| **套 3** | r20-stage-1-2 + r20-stage-3-5 implementation | 1.1-1.5 / 2.1-2.5 / 3.1-3.5 / 4.1-4.5 / 5.1-5.5 (X.Y) | R20 实施 7-10 周 | 套 2 的子集细化 |

§5.3 矛盾点:
- **M-07** (P0): "阶段 1" 三套含义不同 (套 1 = 14 工具+3 SDK commit; 套 2 = 产品打磨; 套 3 = TUI 9 器官)
- **M-08** (P0): "阶段 1.5" 含义冲突 (套 1 = R-Measure verify CI; 套 3 = "集成")
- **M-10** (P0): 套 1 vs 套 3 用同一套 X.Y 编号, 但 X 起点和内容**完全不同**

### 1.2 实际 6 文档用 3 套不同编号

| 文档 | 阶段编号 | 含义 |
|------|---------|------|
| r19-integration-commit-template §1.1 | "阶段 1.1" (R19+ 集成期 7-15 天) | R19+ 集成期 X.Y 细粒度 |
| r20-product-finalize §3 + §4 | "阶段 1-5" (R20 产品化 7-10 周) | R20 收产品 5 大阶段 |
| r20-stage-1-2-implementation §2-§3 | "阶段 1.1-1.5 / 2.1-2.5" (X.Y 细粒度) | R20 阶段 1-2 实施细分 |
| r20-stage-3-5-implementation §2-§4 | "阶段 3.1-3.5 / 4.1-4.4 / 5.1-5.4" | R20 阶段 3-5 实施细分 |
| apeireth-formal-invariants §8 | "阶段 1-5" (5 阶段实施) | formal 5 不变量实施 |
| r19-integration-quickstart §2.2-§2.4 | "步骤 1-5" (5 步 quickstart) | quickstart 操作步骤 |

### 1.3 第 4 套: quickstart 5 步 (不在本对照表范围, 仅说明)

`r19-integration-quickstart-2026-08-05.md` §2.2-§2.4 的"步骤 1-5" 是 **5 步 quickstart 操作流程** (一次性 install / clone / build / run / verify), 跟"阶段"语义不同 (步骤 = 一次性操作, 阶段 = 周期性工作)。本对照表不覆盖 quickstart, 但建议后续拍板时也给 quickstart"步骤 1-5"加命名规范 (见 §4 第 4 项)。

### 1.4 R19+ 集成期 / R20 收产品 / formal 5 不变量 / quickstart 5 步

四套编号并存, 互不重叠但都被叫"阶段"或"步骤", 实际是 4 个不同维度的切分:

| 维度 | 切分依据 | 时长粒度 | 文档套 |
|------|---------|---------|--------|
| **R19+ 集成期** | commit 模板 5 类 (A/B/C/D/E) | 7-15 天 | 套 A (本表 §2.1) |
| **R20 收产品** | 产品 / 部署 / API / SDK / 文档营销 5 大块 | 7-10 周 | 套 B (本表 §2.2) |
| **R20 实施细分** | 套 B 每阶段拆 5 子阶段 | 7-10 周 | 套 B 子表 (本表 §2.2 末) |
| **formal 5 不变量** | 5 个 Kani 不变量 + CI 集成 | 3 天 | 套 C (本表 §2.3) |
| **quickstart 5 步** | 一次性 install/build/run 流程 | 1-2 小时 | (本表外, 见 §1.3) |

---

## §2 三套阶段路线定义 (3 套 = 3 分类)

### 2.1 套 A: R19+ 集成期 5 阶段 (7-15 天, commit 落地)

> **来源**: `r19-integration-commit-template-2026-08-05.md` §1.1 + `r19-integration-wrap-up-2026-08-05.md` §2.2 Gantt
> **焦点**: R19 → R20 中间的工程化收尾期, 重点是 commit 落地 (~60 commit)
> **周期**: 2026-08-05 ~ 2026-10-15 (7-15 周, 估 1-2 天 1 commit)

| 阶段 | 内容 | 估时 | 关键 commit 类型 |
|------|------|-----:|-----------------|
| **1** | R18 P0 6 象限 LLM API 深化 (4 协议 + 5 Provider) | 1-2 周 | 模板 A (新 crate 落地) |
| **2** | R18 P0 mid-task bug 3 处修法 (sendMessage throw→Result / sendToAgent child session 状态检查 / broadcast 事件驱动) | 1 天 | 模板 B (mid-task 3 处一起改) |
| **3** | R19 P1 TUI 9 命令深化 + team-lead (apeireth-team-lead 新 crate + 1:1 翻译 supervisorPrompt.ts 808 LOC + 14 工具 + voting trigger) | 1-2 周 | 模板 A (team-lead) + 模板 E (TUI 改瘦续) |
| **4** | R20 P1 收产品 (5 阶段 7-10 周, per r20-product-finalize §4) | 2-4 周 | 套 B 5 阶段全部 commit |
| **5** | R21+ P2 补缺口 (apeireth-session 新 crate + apeireth-formal 4 个新不变量 + apeireth-storage + R-Measure verify 脚本) | 4-8 周 | 模板 A (新 crate) + 模板 C (Kani 不变量) + 模板 D (R-Measure verify) |

**关键约束** (per `r19-integration-commit-template-2026-08-05.md` §1.1):
- 5 阶段 = 5 大阶段, 阶段内再拆 X.Y 子阶段 (套 1 commit 模板 X.Y 编号 = 套 A 5 阶段**内**的子阶段)
- 估 commit 数 ~60 / 7-15 周 = 1-2 天 1 commit 节奏
- 阶段 1-3 跟 R19+ 集成蓝图 (spectrAI-integration-blueprint-r19-plus) 强协同

### 2.2 套 B: R20 收产品 5 阶段 (7-10 周, release 1.0)

> **来源**: `r20-product-finalize-2026-08-05.md` §3 + §4 + `r20-stage-1-2-implementation-2026-08-05.md` + `r20-stage-3-5-implementation-2026-08-05.md`
> **焦点**: R20 release 1.0 = 把 Apeireth OS v2.0.0-alpha 变成"可分发 / 可部署 / 可用" AI 成长平台
> **周期**: 2026-08-05 ~ 2026-10-15 (7-10 周, 目标 2026-09-30 完工)

| 阶段 | 内容 | 估时 | 关键 owner |
|------|------|-----:|----------|
| **1** | 产品基础 (TUI 9 命令深化 + apeireth-team-lead 公开 API + apeireth-mcp::team 14 工具 + mid-task bug 3 处修法) | 1-2 周 | backend_engineer + frontend_engineer |
| **2** | 部署基础 (Docker 多架构 + 离线包 + Linux deb/rpm + macOS Homebrew + Windows scoop + install 脚本) | 2 周 | devops_engineer |
| **3** | API 公开 (HTTP REST 10 端点 + WebSocket 1 端点 + OpenAPI 3.1 规范 + 鉴权 + 限流) | 2 周 | backend_engineer + technical_writer |
| **4** | SDK 完善 (Rust SDK + Python SDK + TypeScript SDK + 文档示例) | 1-2 周 | fullstack_engineer |
| **5** | 文档 + 营销 (用户文档站 + 开发者文档站 + landing page + Discord 社区) | 1-2 周 | technical_writer + frontend_engineer + community_manager |

**关键约束** (per `r20-product-finalize-2026-08-05.md` §4):
- 5 阶段 = 5 大阶段, 阶段内再拆 X.Y 子阶段 (套 3 实施 X.Y 编号 = 套 B 5 阶段**内**的子阶段)
- 总时长 7-10 周, 目标完工 2026-09-30
- 每阶段结束 = 1 份 `reports/r20-stage<N>-complete-<date>.md` + R-Measure 3 baseline 值守住
- 套 B 阶段 1 = 套 A 阶段 3 子集 (TUI + team-lead 公开), 套 B 阶段 3 = 套 A 阶段 4 主体

### 2.3 套 C: formal 5 不变量 5 阶段 (3 天, Kani 形式化)

> **来源**: `apeireth-formal-invariants-2026-08-05.md` §8 (实施时间表 5 阶段估 3 天)
> **焦点**: apeireth-formal 从 1/5 不变量扩到 5/5, 加 Kani harness + 编译期 hardcode
> **周期**: 3 天 (per §8) + 1 天 R20 阶段 1.5 CI (per §8 第 6 阶段)

| 阶段 | 内容 | 估时 | Owner | 关键产出 |
|------|------|-----:|-------|---------|
| **1** | 不变量 2 `e_layer_isolation` (E 层隔离) + Kani harness + 4 unit test | 0.5 天 | rust-coder | `e_layer_isolation.rs` 80 LOC |
| **2** | 不变量 3 `permission_grant_l0` (L0 联合签名 + HA) + Kani harness + 7 unit test | 0.5 天 | rust-coder | `permission_grant_l0.rs` 100 LOC |
| **3** | 不变量 4 `mid_task_atomicity` (mid-task 状态转换原子性) + Kani harness + 4 unit test (最复杂) | 1 天 | rust-coder | `mid_task_atomicity.rs` 120 LOC |
| **4** | 不变量 5 `seven_advisor_voting` (7 advisor voting 完整性) + Kani harness + 5 unit test | 0.5 天 | rust-coder | `seven_advisor_voting.rs` 100 LOC |
| **5** | 改 `invariants/mod.rs` (5 `pub mod` + `run_all()`) + 改 `lib.rs` (6 `pub const` + 4 `pub mod` re-export) + 5 harness 全跑通验证 | 0.5 天 | rust-coder | 5/5 不变量 + 5 Kani proof 全 SUCCESS |
| **(R20 阶段 1.5)** | CI workflow `.github/workflows/kani.yml` | 1 天 | devops_engineer | 5 Kani proof 每周日自动跑 |

**关键约束** (per `apeireth-formal-invariants-2026-08-05.md` §1.3 + §3.1):
- 5 阶段 = 5 个不变量, 顺序实施 (不变 2 → 3 → 4 → 5 → mod/lib 整合)
- Kani-friendly POD 模型 (u8 / u32 / bool / 固定 array), 0 个 String / Vec / HashMap
- `Cargo.toml` 不动 (M 标记, 保持零依赖, 0 编译图污染)
- 套 C 阶段 5 跟 R20 阶段 1.5 强协同 (R-Measure verify CI workflow 是套 B 阶段 1 末, Kani CI 是套 C 阶段 5 后, 两条 CI 独立)

---

## §3 对照表 (核心交付: 15 行 × 4 列, 一表全览)

| 套 | 阶段 | 含义 | 文档源 |
|----|------|------|-------|
| A | 1 | R18 P0 6 象限 LLM API 深化 (4 协议 + 5 Provider) | commit-template §1.1 + wrap-up §2.2 Gantt |
| A | 2 | R18 P0 mid-task bug 3 处修法 (sendMessage / sendToAgent / broadcast) | commit-template §1.1 + wrap-up §2.2 Gantt |
| A | 3 | R19 P1 TUI 9 命令深化 + team-lead (新 crate + supervisorPrompt.ts 翻译) | commit-template §1.1 + wrap-up §2.2 Gantt |
| A | 4 | R20 P1 收产品 (5 阶段 7-10 周, per r20-product-finalize §4) | commit-template §1.1 + wrap-up §2.2 Gantt |
| A | 5 | R21+ P2 补缺口 (session / formal 4 invariant / storage / R-Measure verify) | commit-template §1.1 + wrap-up §2.2 Gantt |
| B | 1 | R20 产品基础 (TUI + team-lead 公开 + mcp::team 14 工具 + mid-task 修法) | r20-product-finalize §3.1 + §4 阶段 1 + r20-stage-1-2 §2 |
| B | 2 | R20 部署基础 (Docker 多架构 + 离线包 + Linux deb/rpm + macOS brew + Windows scoop) | r20-product-finalize §3.2 + §4 阶段 2 + r20-stage-1-2 §3 |
| B | 3 | R20 API 公开 (REST 10 端点 + WebSocket 1 端点 + OpenAPI 3.1 + 鉴权 + 限流) | r20-product-finalize §3.3 + §4 阶段 3 + r20-stage-3-5 §2 |
| B | 4 | R20 SDK 完善 (Rust / Python / TypeScript 3 SDK + 文档示例) | r20-product-finalize §3.4 + §4 阶段 4 + r20-stage-3-5 §3 |
| B | 5 | R20 文档 + 营销 (用户站 + 开发者站 + landing + Discord 社区) | r20-product-finalize §3.5 + §4 阶段 5 + r20-stage-3-5 §4 |
| C | 1 | e_layer 隔离 (Kani 不变量 2, 跨 E 层写入必检 has_permission) | formal-invariants §2.2 + §8 阶段 1 |
| C | 2 | permission_grant_l0 (Kani 不变量 3, L0 联合签名 + ≥1 HA 票) | formal-invariants §2.3 + §8 阶段 2 |
| C | 3 | mid_task_atomicity (Kani 不变量 4, 状态转换 CAS 原子性无 race) | formal-invariants §2.4 + §8 阶段 3 |
| C | 4 | seven_advisor_voting (Kani 不变量 5, 7 opinion 全到才 synthesis) | formal-invariants §2.5 + §8 阶段 4 |
| C | 5 | CI 集成 (改 mod.rs + lib.rs + 5 Kani proof 全 SUCCESS, R20 阶段 1.5 加 CI workflow) | formal-invariants §3.3 + §3.4 + §8 阶段 5 |

**速查口诀** (per O-3 23:44 决策清单):
- **A1-A5** = 集成期 5 大阶段 (commit 模板维度)
- **B1-B5** = 收产品 5 大阶段 (产品化维度)
- **C1-C5** = 形式化 5 不变量 (Kani 守门维度)

---

## §4 命名规范建议 (待 Mavis 拍板)

为避免 §3 对照表之外的文档出现"阶段 1" 歧义, 建议:

1. **套 A (R19+ 集成期)**: 保留 "阶段 1-5" 编号 (大阶段), 阶段内 X.Y 子阶段 (per commit-template §1.1 既有用法)
2. **套 B (R20 收产品)**: 改 "R20 阶段 1-5" 编号 (大阶段加 R20 前缀), 阶段内 X.Y 子阶段 (per r20-stage-1-2 + r20-stage-3-5 既有用法)
3. **套 C (formal 5 不变量)**: 改 "Kani 不变量 1-5" 编号 (不叫"阶段"), 跟 Kani 不变量名一致 (per formal-invariants §2 既有用法)
4. **quickstart 5 步 (套外, 建议)**: 改 "Quick Start 步骤 1-5" 编号 (加 Quick Start 前缀, 跟"阶段"区分)

**核心**: 任何文档引用"阶段 X" 时, 必须用**全名** ("R19+ 集成期阶段 X" / "R20 阶段 X" / "Kani 不变量 X" / "Quick Start 步骤 X"), 避免裸"阶段 X" 引起三套含义混淆。

**拍板后落地** (本任务**不**做, 待 Mavis 拍板后另起):
- 改 r19-integration-commit-template §1.1: "阶段 1" → "R19+ 集成期阶段 1"
- 改 r20-product-finalize §3+§4: "阶段 1" → "R20 阶段 1"
- 改 apeireth-formal-invariants §8: "阶段 1" → "Kani 不变量 2" (阶段 1 = 不变量 2)
- 改 r19-integration-quickstart §2.2-§2.4: "步骤 1" → "Quick Start 步骤 1"

---

## §5 跨文档引用规范 (本任务落地)

每份文档引用"阶段 X"时, 必须注明 (per §4 第 4 项建议, 本任务先把对照表引到位):

- 套 A: `"(R19+ 集成期) 阶段 X"` 或 `R19+ 集成期阶段 X`
- 套 B: `"(R20 收产品) R20 阶段 X"` 或 `R20 阶段 X`
- 套 C: `"(formal 5 不变量) Kani 不变量 X"` 或 `Kani 不变量 X`
- 套外: `"(quickstart) Quick Start 步骤 X"` 或 `Quick Start 步骤 X`

**本任务落地** (5 个微调文件 + 1 行引用):
- `r19-integration-commit-template-2026-08-05.md` §1.1: 加 `> 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3`
- `r20-product-finalize-2026-08-05.md` §3: 加 `> 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3`
- `r20-stage-1-2-implementation-2026-08-05.md` 头部: 加 `> 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3`
- `r20-stage-3-5-implementation-2026-08-05.md` 头部: 加 `> 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3`
- `apeireth-formal-invariants-2026-08-05.md` §1: 加 `> 阶段编号详见 docs/stage4/r19-r20-stage-unified-2026-08-05.md §3`

---

## §6 不修改承诺

- 本文档**不**碰 `APEIRETH-CONVENTIONS.md` (LOCKED, 阶段 1+2+3 + v2/v4/v4.1 + 12 键 + 6 锚 + workspace v1.0.0 + R11 baseline 三值)
- 本文档**不**碰任何 M 标记文件 (`lib.rs` / `Cargo.toml` / etc)
- 本文档**不**碰任何其他 `crates/` 源码
- 本文档**不**碰任何 `docs/stage3-blueprints/` LOCKED 蓝图
- 本文档**不**碰任何 `docs/adr/0001-0009` (LOCKED 已有 ADR)
- 本文档**不**碰 Hermes 加的任何文件 (cargo / CI / tests)
- 本文档**不**碰任何 `docs/roadmap/` 已有 R 路线 (除 r20-product-finalize-2026-08-05.md 加 1 行引用, 主人已批准回写 1 次)
- 本文档**只**做"5 阶段编号统一" + "对照表" + "5 个微调文件加 1 行引用"

---

## §7 6 哲学 anchor (per APEIRETH-CONVENTIONS §9)

| Anchor | 时间 | 引用方式 | 本文档落地 |
|--------|------|---------|----------|
| **S-1** | 22:33 | 6 anchor ASI 完整性 | 5 阶段统一是 ASI 完整性的工程化 (3 套对照表让接手者不踩雷) |
| **S-2** | 17:43 | 6 anchor 实验室 | 实事求是承认 3 套并存 (per §1.1 + §1.2 + §2), 不假装"已经统一", 不强求命名一致 |
| **O-5** | 17:58 | 6 anchor 12 急救 | 编号冲突是 P0 急救 (per §1.1 M-03 + §5.3 M-07~M-10), 本对照表是急救"知道三套并存" |
| **O-2** | 19:33 | 6 anchor 4 分类 | 3 套阶段 3 分类 (per §2: 套 A 集成期 / 套 B 收产品 / 套 C 形式化), 跟 O-2 "4 分类" 哲学一致 |
| **O-3** | 23:44 | 6 anchor 决策清单 | 5 阶段总览 (per §3 对照表 + §4 命名规范 + §5 引用规范, 三表拍板即可执行) |
| **O-4** | 00:56 | 6 anchor 12 统一 | 跟 12 子规范统一 (per §5 跨文档引用规范 = 12 项里 "文档编号" 1 项的工程化) |

---

## §8 关联文档

- `reports/docs-cross-check-2026-08-05.md` §2 (M-03 严重问题) + §5.1 (三套 5 阶段) + §5.3 (M-07~M-10 矛盾点)
- `reports/r19-integration-wrap-up-2026-08-05.md` §1.4 (5 阶段路线总览) + §2.2 (R19+ 5 阶段 Gantt) — ⚠️ 此文件被多处引用但实际路径在 `.minimax-agent-cn\spectrai\reports\`, 不在 Apeireth-rust 仓库内 (per 互检报告 §8.2 L-05 断链根源)
- `docs/stage4/r19-integration-commit-template-2026-08-05.md` §1.1 (R19+ 集成期 5 阶段)
- `docs/roadmap/r20-product-finalize-2026-08-05.md` §3 + §4 (R20 收产品 5 阶段)
- `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` §1.1 + §2-§3 (R20 阶段 1-2 实施)
- `docs/stage4/r20-stage-3-5-implementation-2026-08-05.md` §1.1 + §2-§4 (R20 阶段 3-5 实施)
- `docs/stage4/apeireth-formal-invariants-2026-08-05.md` §1 + §8 (formal 5 不变量 5 阶段)
- `docs/stage4/r19-integration-quickstart-2026-08-05.md` §2.2-§2.4 (quickstart 5 步, 套外, 不在本对照表范围)
- 5 份 R19+ 集成文档 (微调 + 1 行引用): 同 §5
