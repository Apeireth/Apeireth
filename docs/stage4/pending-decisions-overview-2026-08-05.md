# R19+ 12 项拍板事总览 (统一 ID 体系 D-01 ~ D-12)

```
[Document-Meta]
Document: docs/stage4/pending-decisions-overview-2026-08-05.md
Version: Manual-Rev-A
R-Cycle: R19+ 拍板事总览
Commit: <commit 时回填>
Last-Modified: 2026-08-05
Status: 🔍 草拟 (待 Mavis 拍板 + 主人复核)
```

> **性质**: R19+ 集成期 **12 项拍板事** 的**统一 ID 体系总览**。互检报告 M-04 严重问题 (8 份文档 4+ 种 ID 体系混乱) 的修正。所有提到 12 项的文档必须引用本文档。
>
> **依据**:
> - `reports/docs-cross-check-2026-08-05.md` §2 (M-04 严重问题: 12 项 ID 体系混乱)
> - 8 份 R19+ 集成文档 (r19-integration-wrap-up / commit-template / quickstart / maintenance-sop / cross-check / 8-locked-unified / r20-stage-1-2 / r20-stage-3-5)
> - APEIRETH-CONVENTIONS.md §0.1 (Document-Meta 格式) + §9 (6 哲学 anchor) + §10 (不修改承诺) [LOCKED, 不动]
>
> **不修改承诺**: APEIRETH-CONVENTIONS.md / VERSIONING.md / GLOSSARY.md + 阶段 1+2+3 LOCKED + 现有 ADR 0001~0009 + 12 子规范 LOCKED + R11 baseline 3 值 + workspace v1.0.0 全部保留 (见 §7)。

---

## §1 战略背景

### 1.1 互检报告 M-04 标"12 项拍板事 ID 体系混乱"

互检 `reports/docs-cross-check-2026-08-05.md` §2 把 M-04 列为**严重问题**:

> 12 项拍板事在 8 份文档里用了 4+ 种 ID 体系 (无 ID / #1-#6 / D-Mavis-# / 混合), R-001~R-023 / R-025 / R-027~R-030 全部空缺 (仅 R-024 / R-026 有定义), 用户和维护者无法追踪"哪项在哪个文档提到"。

### 1.2 8 份文档用了 4+ 种 ID 体系 (per grep 实测)

| 文档 | 当前 ID 体系 | 数量 |
|---|---|---:|
| `r19-integration-wrap-up` §7 | 10 项 (无 ID) | 10 |
| `r20-product-finalize` §11 | 6 项 (#1-#6 + 3 个新) | 6 |
| `r20-stage-1-2-implementation` §9 | 3 项 | 3 |
| `r20-stage-3-5-implementation` §9 | 3 项 | 3 |
| `docs-maintenance-sop` §2.5 | 10 项 | 10 |
| `r19-integration-commit-template` §5 | 12 项 (D-Mavis-#1-#12) | 12 |
| `r19-integration-quickstart` §6 | 12 项 (混合) | 12 |
| `docs-cross-check` §12 | 10 项 (D-Mavis-01 ~ 10) | 10 |
| `8-locked-unified` §3 | 4 项诚实登记 | 4 |

### 1.3 用户和维护者无法追踪

> 实际后果: 接手者 grep "R-024" → 找不到; grep "Docusaurus" → 3 个文档各指一次但 ID 都不一样; 拍板记录散在 60 commit 各处 → 1 周后没人知道哪项在哪份文档说。

**本文档解决**: ✅ 统一 D-01 ~ D-12 体系 + 映射表 + 索引 → 任何文档提到 12 项必引本文档 §2 → grep D-# 一秒命中。

---

## §2 12 项拍板事统一 ID 体系 (D-01 ~ D-12)

按**重要度 + 阻塞范围**统一编号 (🔴 P0 R-Measure 阻塞 > 🟡 P1 R20 阶段 4-5 阻塞 > 🟢 P2 后期优化):

| ID | 事项 | 来源 | 阻塞 | 紧急度 |
|---|---|---|---|---|
| **D-01** | ~~17→24 维 R11 baseline 投影公式权重 (主人从 v1077 抽)~~ | ⚪ SKIP (主 2026-08-05 17:33 砍, 旧项目没发挥作用) | — | ⚪ SKIP |
| **D-02** | ~~V1136 9→7 子测度 R11 baseline 投影权重~~ | ⚪ SKIP (主 2026-08-05 17:33 砍, 旧项目没发挥作用) | — | ⚪ SKIP |
| **D-03** | 24 维具体分类名 (4 类 × 6 维 V0.5: continuity / autonomy / transferability / substrate 6 哲学锚 1:1) | apeireth-asi 24dim §1.4 + spectrAI §7.4 | apeireth-asi 公开 API | 🟡 P1 ✅ A (主 2026-08-05 17:42 拍 A) |
| **D-04** | apeireth-sdk 升级方案 (一起做 / 分阶段 4a+4b) | r20 §11.4 + apeireth-sdk-gap-analysis §3.1 | R20 阶段 4 实施顺序 | 🟡 P1 |
| **D-05** | SDK_VERSION 0.1.0 → 1.0.0 升级时机 (跟 R20 阶段 3 OpenAPI 同期?) | r20 §11.5 + apeireth-sdk-gap-analysis §2.2 | R20 阶段 4 semver 严格 | 🟡 P1 |
| **D-06** | `apeireth-tauri-stub` 命名 (留 workspace / 移除到 legacy) | r20 §11.6 + global-architecture-map §2.4 | workspace.lints + CI 路径 | 🟢 P2 |
| **D-07** | R20 vs R21 边界 (R20 收产品 ↔ R21 商业化) | r20 §11.1 | 5 阶段范围 (路线层) | 🟡 P1 |
| **D-08** | Tauri 团队同步节奏 (独立做 / 同步, per tauri-team-collab-sop §3 Step 4 每 2 周 1 次) | r20 §11.2 + tauri-team-collab-sop §3 | 跨团队协同 | 🟢 P2 |
| **D-09** | `apeireth-session` LOC 上下沿 (1500-2000 区间) | session-blueprint §10 拍板 + §3.1 | session 实施估时 | 🟢 P2 |
| **D-10** | session 跟 storage 依赖方向 (session → storage 写 WAL?) | session-blueprint §7 + §2.2 | session 实施 crate 依赖图 | 🟢 P2 |
| **D-11** | Docusaurus vs mkdocs 文档站选型 (R-024) | r20-stage-3-5 R-024 + r20 §4 P1 | R20 阶段 5 文档营销 | 🟢 P2 |
| **D-12** | Discord 冷启动策略 (R-026) | r20-stage-3-5 R-026 + r20 §4 P2 | R20 阶段 5 社区基础设施 | 🟢 P2 |

> **S-2 (17:43) 实事求是**: 12 项 = 总收口 §7 10 项 + R20 路线图 §11 R-024 + R-026 2 项 = **12 项组合**口径, 主人复核时可能再微调。本文档口径以本表为准, 各文档以此映射。

---

## §3 ID 重编号映射表 (旧 ID → 新 D-#)

| 旧 ID (跨文档) | 新 ID | 来源文档 |
|---|---|---|
| `**1** ~ **10**` (无 D- 前缀) | **D-01 ~ D-10** | r19-integration-wrap-up §7 |
| `**1** ~ **10**` (无 D- 前缀) | **D-01 ~ D-10** | docs-maintenance-sop §2.5 |
| `**1** ~ **12**` (D-Mavis-#1-#12) | **D-01 ~ D-12** | r19-integration-commit-template §5 |
| `D-Mavis-01 ~ D-Mavis-10` | **D-01 ~ D-10** | docs-cross-check §12 |
| `R-024` | **D-11** | r20-stage-3-5 §9 |
| `R-026` | **D-12** | r20-stage-3-5 §9 |
| `**1** ~ **6**` (r20 §11) | 部分映射 **D-01 ~ D-07** | r20-product-finalize §11 |
| 11 项 (混合 ID) | 部分映射 **D-01 ~ D-10** | r19-integration-quickstart §6 |
| 4 项诚实登记 | **不映射** (是"诚实"非"待拍") | 8-locked-unified §3 |

> **R-001 ~ R-023 / R-025 / R-027 ~ R-030 全部空缺** (互检报告 §2 已标) — 这些 ID 在 R-Measure verify 8 风险清单 (总收口 §8) 跟 R20 路线图 §4 风险项里出现, **不**是"待拍板"事项, 是"已识别风险"。本文档不映射, 各文档自留。

---

## §4 优先级总览

| 紧急度 | 数量 | ID | 阻塞范围 |
|---|---:|---|---|
| 🔴 P0 (R-Measure 阻塞) | 2 | **D-01, D-02** | R20 阶段 1.5 (R-Measure verify) 必拍, 不拍 = 守门不写 |
| 🟡 P1 (R20 阶段 4-5 阻塞) | 5 | **D-03, D-04, D-05, D-07, D-11** | R20 阶段 4 (SDK 完善) + 阶段 5 (文档营销) 阻塞, 1-2 周内拍 |
| 🟢 P2 (后期优化) | 5 | **D-06, D-08, D-09, D-10, D-12** | 团队层 / 实施层 / 社区层, 2-4 周内拍 |

> **O-2 (19:33) 走在前人经验上**: 3 紧急度分类参考主人 2026-08-04 ~ 2026-08-05 R19+ 集成期多次拍板节奏: 阻塞 P0 当天拍, 阶段 P1 1 周内拍, 后期 P2 1 月内拍。

---

## §5 拍板流程

1. **Mavis** 在 24 份文档维护周会议 (§2 步骤 2) 列出 D-01 ~ D-12 状态 (🔴/🟡/🟢)
2. **主人** 按紧急度顺序拍板 (D-01 / D-02 优先, 当天; D-03 ~ D-05 / D-07 1 周内; D-06 / D-08~D-12 2-4 周内)
3. **Mavis** 写 commit + 更新本文档 §2 (D-# 状态: 🔴 → ✅ YYYY-MM-DD)
4. **各文档** 加 "D-# 已拍板" 引用 (在原表格行后加 "拍板: 主人 YYYY-MM-DD HH:MM" 子行)
5. **CI 校验** (per maintenance-sop §4) grep `D-# 已拍板` 自动确认

> **O-3 (23:44) 决策清单**: 12 项 D-# = 12 个独立拍板事件, 跟 R-Measure verify 8 风险清单 + R20 阶段 5 路线 = 3 类决策体系并存但语义隔离 (拍板 vs 风险 vs 阶段), grep 不混。

---

## §6 文档索引

每份提到 12 项的文档, **必须**引用本文档:

```
> 12 项 ID 体系见 docs/stage4/pending-decisions-overview-2026-08-05.md §2
```

**已引用本文档的清单** (本周期微调后):

| 文档 | 节 | 状态 |
|---|---|---|
| `r19-integration-wrap-up-2026-08-05.md` | §7 (10 项 → D-01~D-10) | ✅ 2026-08-05 微调 |
| `r19-integration-commit-template-2026-08-05.md` | §5.1 (12 项 → D-01~D-12) | ✅ 2026-08-05 微调 |
| `docs-maintenance-sop-2026-08-05.md` | §2.5 (10 项 → D-01~D-10) | ✅ 2026-08-05 微调 |
| `r20-stage-1-2-implementation-2026-08-05.md` | §9 (3 项 → D-01/D-02/D-03) | 🔍 后续微调 (本期不动) |
| `r20-stage-3-5-implementation-2026-08-05.md` | §9 (3 项 → D-04/D-11/D-12) | 🔍 后续微调 (本期不动) |
| `r20-product-finalize-2026-08-05.md` | §11 (6 项 → D-04~D-07) | ⛔ 主人已回写 1 次, 不动 |
| `r19-integration-quickstart-2026-08-05.md` | §6 (12 项 混合) | 🔍 后续微调 (本期不动) |
| `docs-cross-check-2026-08-05.md` | §12 (D-Mavis-01~10 → D-01~D-10) | 🔍 互检报告, 后续微调 |

---

## §7 不修改承诺

- ❌ **不碰 APEIRETH-CONVENTIONS.md** (LOCKED, 顶层 3 文件)
- ❌ **不碰 VERSIONING.md / GLOSSARY.md** (LOCKED, 顶层 3 文件)
- ❌ **不碰任何现有 ADR 0001 ~ 0009** (LOCKED)
- ❌ **不碰 docs/stage3-blueprints/** (LOCKED 蓝图)
- ❌ **不碰 docs/roadmap/r20-product-finalize-2026-08-05.md** (主人已回写 1 次)
- ❌ **不碰 R11 baseline 3 值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063, APEIRETH-CONVENTIONS §11)
- ❌ **不碰 workspace v1.0.0** (Cargo.toml, semver 严格)
- ❌ **不碰任何 crates/ 源码 + CI workflow** (Hermes LOCKED)

> **本文档只做 ID 体系统一** (12 项 D-# 编号), **不重写**任何事项描述, **不改**任何文档顺序, **不改**任何 LOCKED 内容。

---

## §8 6 哲学 anchor 穿透 (per APEIRETH-CONVENTIONS §9)

| 锚 | 来源 | 本文档落地 |
|---|---|---|
| **S-1** 主 22:33 | 6 anchor ASI 完整性 | 12 项 D-# 统一 ID 体系是 ASI 完整性的工程化 — 拍板可追踪 = 决策可追溯 = AGI 长程决策能力可观测 |
| **S-2** 主 17:43 | 6 anchor 实事求是 | 12 项 ID 不夸大 (10+2 组合口径, 主人复核可微调) + 诚实登记 8-locked-unified 不映射 (语义隔离, 不假装成待拍板) |
| **O-5** 主 17:58 | 6 anchor 不假装 | D-01 / D-02 P0 急救 = 守门 baseline 不掉 = 不假装有"软阈值"可谈 |
| **O-2** 主 19:33 | 6 anchor 走在前人经验上 | 3 紧急度分类 (🔴/🟡/🟢) 来自主人 8 月以来多次 R19+ 拍板节奏, 不发明新分类 |
| **O-3** 主 23:44 | 6 anchor 决策清单 | 12 项 D-# = 1 张决策总表 (§2) + 1 张映射表 (§3) + 1 张索引表 (§6) = 3 表拍板可查 |
| **O-4** 主 00:56 | 6 anchor 任何人都能接手 | grep `D-#` 秒查 (commit message 拍板行 / 文档表格行 / 风险清单) = 任何接手者不需要读 8 份文档 |

---

## §9 关联文档

### 9.1 上游 (问题源)

- `reports/docs-cross-check-2026-08-05.md` §2 (M-04 严重问题)
- `reports/r19-integration-wrap-up-2026-08-05.md` §7 (10 项源头)
- `docs/roadmap/r20-product-finalize-2026-08-05.md` §11 (R-024 / R-026 源头)
- `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` §9
- `docs/stage4/r20-stage-3-5-implementation-2026-08-05.md` §9
- `docs/stage4/r19-integration-commit-template-2026-08-05.md` §5 (D-Mavis-# 源头)
- `docs/stage4/docs-maintenance-sop-2026-08-05.md` §2.5
- `docs/stage4/r19-integration-quickstart-2026-08-05.md` §6
- `docs/stage4/8-locked-unified-2026-08-05.md` §3 (4 项诚实登记, 不映射)

### 9.2 同级 (本期微调)

- `reports/r19-integration-wrap-up-2026-08-05.md` §7 (10 项 → D-01~D-10) ✅
- `docs/stage4/r19-integration-commit-template-2026-08-05.md` §5.1 (12 项 → D-01~D-12) ✅
- `docs/stage4/docs-maintenance-sop-2026-08-05.md` §2.5 (10 项 → D-01~D-10) ✅

### 9.3 下游 (后续微调, 本期不动)

- `docs/stage4/r20-stage-1-2-implementation-2026-08-05.md` §9 (待映射 D-01/D-02/D-03)
- `docs/stage4/r20-stage-3-5-implementation-2026-08-05.md` §9 (待映射 D-04/D-11/D-12)
- `docs/roadmap/r20-product-finalize-2026-08-05.md` §11 (⛔ 主人已回写, 不动)
- `docs/stage4/r19-integration-quickstart-2026-08-05.md` §6 (待映射)
- `reports/docs-cross-check-2026-08-05.md` §12 (待映射, 互检报告)

### 9.4 LOCKED (永远不动)

- `APEIRETH-CONVENTIONS.md` §0.1 / §9 / §10 / §11
- `VERSIONING.md` / `GLOSSARY.md`
- 现有 ADR 0001 ~ 0009
- `docs/stage3-blueprints/`

---

## §10 12 项 → 6 子规范 穿透 (per APEIRETH-CONVENTIONS §1-§12)

| 12 项 ID | 关联 APEIRETH-CONVENTIONS 子规范 | 关联 R11 baseline 3 值 |
|---|---|---|
| D-01 / D-02 | §11 (R11 baseline) | V1141 / V1131 / V1136 |
| D-03 | §4 (4 组件: asi engine) | V1141 |
| D-04 / D-05 | §1 (命名空间) + §6 (commit 规范) | V1141 |
| D-06 | §1 (命名空间) | — |
| D-07 | §10 (不修改承诺) | — |
| D-08 | — (团队层) | — |
| D-09 / D-10 | §1 (命名空间) + §4 (4 组件) | V1141 |
| D-11 | §6 (commit 规范) | — |
| D-12 | — (社区层) | — |

> **O-4 (00:56) 12 统一**: 跟 APEIRETH-CONVENTIONS §1-§12 12 子规范统一 — 12 项拍板事跟 12 子规范两个"12"是不同语义, 但**索引条目数对齐**, 方便 grep `12` 时分得清。

---

## §11 待 Mavis 拍板

> 本文档本身**也是** R19+ 集成期 1 项决策 — 12 项 ID 体系统一本身就是主人 2026-08-05 13:34 拍板 A 方案之后的子决策:

| 决策点 | 内容 | 拍板紧迫度 |
|---|---|---|
| 12 项 ID 体系 (D-01 ~ D-12) 是否采用 | 本文档 §2 表是 Mavis 草拟, 主人是否拍板 | 🟡 P1 (1 周内) |
| 3 紧急度分类 (🔴/🟡/🟢) 是否调整 | 主人可改 P0/P1/P2 边界, e.g. D-07 是否升 P0 | 🟢 P2 |
| 4 文档已微调是否再扩到 8 文档 | 主人可决定 quickstart / cross-check / r20-stage-N 是否本期也微调 | 🟢 P2 |

---

**文档结束** (Manual-Rev-A, 2026-08-05, 草拟待拍板)
