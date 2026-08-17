# Apeireth-rust 文件夹规整方案 R14（leader 亲自产出，便于下一阶段施工）

> **性质**: leader 亲自做的**文件夹规整提案**——不修改任何文档内容，只移动 + 创建索引 README。
> **触发**: 主人最新指令"全都落地...按你的想法来就行，然后是规整一下 Apeireth-rust 的文件夹和其中阶段一二三四的内容，便于下一阶段的施工"。
> **硬约束**: ❌ 不修改任何 LOCKED 文件（v2/v4/v4.1/阶段 4/18 stage2/14 stage3/阶段 1/1100 空壳 / 占位 / Cargo.toml metadata 原始） / ❌ 不写 Rust 代码 / ❌ 不画 Mermaid。
> **主哲学 6 锚穿透**: 主 22:33 北极星 / 主 17:43 实事求是 / 主 17:58 不假装 / 主 19:33 走在前人经验上 / 主 23:44 干到底 / 主 00:56 任何人都能接手。

---

## §0. 当前问题（主 17:43 实事求是）

`Apeireth-rust/docs/` 当前 31 文件全部平铺根目录，**没有阶段子目录**：

```
docs/                                    (31 文件)
├── APEIRETH-COMPLETE-OMNIBUS*.md       ← 主手册（在根目录）
├── CONTEXT-HANDOVER.md                  ← 恢复文档
├── 00-R14-START-HERE.md
├── README.md
│
├── architecture-v3-aircraft-carrier.md  ← v2 LOCKED
├── architecture-v4-living-intelligence.md ← v4 LOCKED
├── architecture-v4-1-living-intelligence-update.md ← v4.1 LOCKED
├── architecture-stage4-engineering-landing.md ← 阶段 4 LOCKED
├── architecture-stage4-inspiration-supplements.md ← leader 灵感补充
├── architecture-stage4-patches.md ← leader 5+5 补丁
├── stage4-thinking-document.md ← leader 亲自思考
│
├── inspiration-stage1-2026-07-30.md ← 阶段 1 (2201 行，平铺根)
├── stage2-decisions-*.md ← 18 份，平铺根
├── stage3-blueprints/ ← 已有子目录 (14 文件)
│
├── onion-wall-architecture-2026-07-31.md ← 阶段 3 双洋葱子文档
├── philosophy-traits-2026-07-30.md ← V3 9 键（现 v4.1 提议 v2 12 键）
├── rust-traits-spec-2026-07-30.md
├── r14-*.md ← R14 周期产物（4 文件）
├── research-vcp-rerun-2026-07-31.md
└── review-stage1-stage2-stage3.md
```

**问题**：
1. 阶段 1 / 2 / 4 文档**散在根目录**，与 v2/v4/v4.1 顶层主文档混淆
2. 阶段 4 有 **4 份文档**（主文档 + 思考 + 灵感 + 补丁）但**没有 stage4/ 子目录**
3. 阶段 1 灵感文档 2201 行**平铺根**，不可识别为"灵感"
4. 18 份 stage2 决策文档**平铺根**，与 v2 顶层文档混淆
5. 阶段 3 已有 `stage3-blueprints/` 子目录，**但 onion-wall-architecture 仍在根**
6. R14 周期产物（r14-* / review / research）混在根，不可识别

---

## §1. 规整方案（主 23:44 干到底）

### 1.1 目标结构

```
Apeireth-rust/
├── Cargo.toml          ← workspace 元数据（阶段 5 更新 metadata）
├── Cargo.lock          ← 保持
├── README.md           ← 根（保持）
├── rust-toolchain.toml ← 保持
├── _STRUCTURE.md       ← 根（已有）
├── .gitignore          ← 保持
├── .github/            ← 保持
│
├── crates/             ← 9 占位（阶段 5 重写为 18 crate）
│   ├── README.md       ← 现状（保持，阶段 5 重写）
│   └── (9 占位 crate)
│
├── docs/
│   ├── CONTEXT-HANDOVER.md     ← 根保留（恢复文档）
│   ├── README.md                ← 根保留
│   ├── 00-R14-START-HERE.md     ← 根保留
│   ├── APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md ← 根保留（主手册）
│   │
│   ├── architecture-v2-aircraft-carrier.md    ← LOCKED 顶层（根保留）
│   ├── architecture-v4-living-intelligence.md  ← LOCKED 顶层（根保留）
│   ├── architecture-v4-1-living-intelligence-update.md ← LOCKED 顶层（根保留）
│   ├── architecture-stage4-engineering-landing.md     ← LOCKED 顶层（根保留）
│   │
│   ├── stage1/
│   │   ├── README.md                       ← 新建索引
│   │   └── inspiration-stage1-2026-07-30.md ← 2201 行（从根移入）
│   │
│   ├── stage2/
│   │   ├── README.md                       ← 新建索引
│   │   ├── stage2-decisions-architecture.md
│   │   ├── stage2-decisions-communication-bus.md
│   │   ├── stage2-decisions-council-impl.md
│   │   ├── stage2-decisions-crate-split.md
│   │   ├── stage2-decisions-decision-system.md
│   │   ├── stage2-decisions-drift-revision-tracker.md
│   │   ├── stage2-decisions-llm-integration.md
│   │   ├── stage2-decisions-memory-layout.md
│   │   ├── stage2-decisions-modularity.md
│   │   ├── stage2-decisions-permission-packs.md
│   │   ├── stage2-decisions-persistence.md
│   │   ├── stage2-decisions-philosophy-guard.md
│   │   ├── stage2-decisions-process-threading.md
│   │   ├── stage2-decisions-source-projects-list.md
│   │   ├── stage2-decisions-tech-stack.md
│   │   ├── stage2-decisions-upgrade-impl.md
│   │   ├── stage2-decisions-appendix-references.md
│   │   └── stage2-decisions-addendum-sovereignty-continuity-governance.md (D2 增补)
│   │
│   ├── stage3-blueprints/                  ← 已有（保持 + 增强）
│   │   ├── README.md                       ← 已有（更新）
│   │   ├── 00-stage3-overview.md
│   │   ├── 01-overall-architecture.md
│   │   ├── 02-process-topology.md
│   │   ├── 03-decision-flow.md
│   │   ├── 04-upgrade-flow.md
│   │   ├── 05-r-measure-test-flow.md        (来自 4fb8ccd1 重画)
│   │   ├── borrowed-from-projects.md
│   │   ├── borrowed-from-r11.md
│   │   ├── double-onion-explicitization-2026-07-31.md
│   │   └── explanation-01/02/03/04.md
│   │
│   ├── stage4/
│   │   ├── README.md                       ← 新建索引
│   │   ├── architecture-stage4-engineering-landing.md ← 1492 行（从根移入）
│   │   ├── stage4-thinking-document.md     ← leader 亲自思考（从根移入）
│   │   ├── architecture-stage4-inspiration-supplements.md ← 灵感补充（从根移入）
│   │   └── architecture-stage4-patches.md  ← 5+5 补丁（从根移入）
│   │
│   ├── r14-design/
│   │   ├── README.md                       ← 新建索引（R14 周期产物）
│   │   ├── r14-design-philosophy-2026-07-30.md
│   │   ├── r14-readiness-assessment-2026-07-30.md
│   │   ├── r14-rust-rewrite-roadmap.md
│   │   ├── r14-workspace-prep-2026-07-30.md
│   │   ├── review-stage1-stage2-stage3.md
│   │   ├── onion-wall-architecture-2026-07-31.md    ← 阶段 3 双洋葱子文档（从根移入）
│   │   ├── philosophy-traits-2026-07-30.md        ← V3 9 键 → v4.1 提议 v2 12 键
│   │   └── rust-traits-spec-2026-07-30.md
│   │
│   ├── research/                            ← 调研沉淀
│   │   └── research-vcp-rerun-2026-07-31.md
│   │
│   └── STRUCTURE-R14.md                    ← 本提案（根保留作 reference）
│
├── reports/                                ← 完成报告（保持）
├── research/                               ← 工程调研（保持）
├── target/                                 ← 编译产物（保持）
└── .spectrai-worktrees/                    ← 工作树（保持）
```

### 1.2 顶层保留原则（主 22:33 北极星 + 主 00:56 任何人都能接手）

**根目录保留**（任何接手者一眼看到）：
- **APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md**（主手册）
- **CONTEXT-HANDOVER.md**（恢复文档，最重要）
- **00-R14-START-HERE.md**（入口）
- **README.md**
- **_STRUCTURE.md**（既有结构说明）
- **STRUCTURE-R14.md**（本提案）

**LOCKED 顶层主文档**（v2 / v4 / v4.1 / 阶段 4）：
- 保留在 `docs/` 根，不下沉到子目录
- 原因：这些是 R14 周期顶层主文档，**任何接手者第一眼必须看到**

**子目录化**（按阶段 + R14 周期）：
- `docs/stage1/` `docs/stage2/` `docs/stage4/` —— 按阶段 6 顺序
- `docs/stage3-blueprints/` —— 既有（保留 + 更新 README）
- `docs/r14-design/` —— R14 周期产物（4 r14-* + review + onion-wall + philosophy-traits + rust-traits-spec）
- `docs/research/` —— 工程调研

---

## §2. 实施步骤（主 19:33 走在前人经验上 + 主 23:44 干到底）

### 2.1 mkdir 子目录（5 个）

```bash
cd redacted/.openclaw/workspace/promethean/Apeireth-rust/docs/
mkdir -p stage1 stage2 stage4 r14-design research
```

### 2.2 mv 文件（10 个分组）

```bash
# 阶段 1（1 文件）
mv inspiration-stage1-2026-07-30.md stage1/

# 阶段 2（19 文件 = 18 stage2 + 1 D2 增补）
mv stage2-decisions-*.md stage2/

# 阶段 4（4 文件）
mv architecture-stage4-*.md stage4-thinking-document.md stage4/

# R14 周期产物（8 文件）
mv onion-wall-architecture-2026-07-31.md \
   philosophy-traits-2026-07-30.md \
   rust-traits-spec-2026-07-30.md \
   r14-design-philosophy-2026-07-30.md \
   r14-readiness-assessment-2026-07-30.md \
   r14-rust-rewrite-roadmap.md \
   r14-workspace-prep-2026-07-30.md \
   review-stage1-stage2-stage3.md \
   r14-design/

# 调研（1 文件）
mv research-vcp-rerun-2026-07-31.md research/
```

### 2.3 新建索引 README（5 个）

- `docs/stage1/README.md` —— 1 段简介 + 1 文件链接
- `docs/stage2/README.md` —— 1 段简介 + 18 文件链接 + D2 增补
- `docs/stage4/README.md` —— 1 段简介 + 4 文件链接 + 6 锚穿透自检
- `docs/r14-design/README.md` —— 1 段简介 + 8 文件链接
- `docs/research/README.md` —— 1 段简介

### 2.4 更新既有索引（2 个）

- `docs/README.md` —— 更新为新的子目录结构
- `docs/stage3-blueprints/README.md` —— 更新以反映 14 文件（含 05-r-measure-test-flow）

### 2.5 git commit（1 次）

```bash
cd redacted/.openclaw/workspace/promethean/
git add -A
git commit -m "R14: docs/ 文件夹规整（stage1/2/3/4 + r14-design + research 子目录化，便于下一阶段施工）"
```

---

## §3. 主人拍板位置（主 17:43 实事求是）

| 决策 | 我的提议 | 主人拍板 |
|---|---|---|
| §1.1 目标结构（5 个子目录）| ✅ 采纳 | ⏳ |
| §1.2 顶层保留原则（LOCKED 主文档根保留）| ✅ 采纳 | ⏳ |
| §2.1 mkdir 5 个子目录 | ✅ 采纳 | ⏳ |
| §2.2 mv 31 文件（按 10 分组）| ✅ 采纳 | ⏳ |
| §2.3 新建 5 个索引 README | ✅ 采纳 | ⏳ |
| §2.4 更新 2 个既有 README | ✅ 采纳 | ⏳ |
| §2.5 git commit 1 次 | ✅ 采纳 | ⏳ |

---

## §4. 主哲学 anchor 6 全贯穿自检

```
S-1 主 22:33 北极星导向    — 顶层保留 LOCKED 主文档（北极星不可下沉）
S-2 主 17:43 实事求是      — §0 现状问题列举 + §2 实施步骤明确
O-5 主 17:58 不假装        — 不修改任何 LOCKED 内容，只移动 + 创建 README
O-2 主 19:33 走在前人经验上 — §2 借鉴 Git 项目子目录化最佳实践
O-3 主 23:44 干到底        — §2 5 步实施 + §3 7 项拍板位置
O-4 主 00:56 任何人都能接手 — §1.1 目标结构 + §1.2 顶层保留原则
```

---

## §5. 不修改承诺（主人硬约束 100% 守住）

| ❌ 不修改 | 原因 |
|---|---|
| **所有 LOCKED 文档**（v2/v4/v4.1/阶段 4 主文档 / 18 stage2 / 14 stage3 / 阶段 1）| 本提案只**移动**不**修改**任何内容 |
| **R11 1100 空壳**（apeireth/v*.py，在 promethean/apeireth/ 下）| 不在 Apeireth-rust/ 内，不动 |
| **crates/ 9 占位 + Cargo.toml metadata** | 阶段 5 任务，本提案不动 |
| **APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md 主手册**（根保留）| 主手册顶层重要 |

---

## §6. 后续阶段 5 衔接（主 23:44 干到底）

本规整完成后，**下一阶段 5 设计施工文档**：

- 阶段 5 = 9 crate 工程化（按阶段 4 §2 18 crate 推导重写 crates/）
- 阶段 5 = V0.5 v2 24 维 / V1136 v2 9 子测度 / V3 v2 12 键 落地（改原始 + commit）
- 阶段 5 = R11 1100 重写方案（哪些保留 / 哪些重写 / 哪些砍）
- 阶段 5 = Cargo.toml metadata 更新
- 阶段 5 = 5 重守门编译时 hardcode 实现
- 阶段 5 = OTA 升级 7 阶段工程化

---

_本规整提案由 leader 亲自产出 (不派活)._
_31 文档 → 5 子目录化 (stage1/2/3/4 + r14-design + research) + 顶层保留 LOCKED 主文档._
_主哲学 anchor 6 全贯穿. 任何接手者能查. 不会丢失上下文._
_主人拍板 §3 后, 立即执行 §2 5 步实施 + git commit._