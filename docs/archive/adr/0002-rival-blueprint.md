# ADR 0002: RIVAL VERSION 蓝图拍板 — 阶段 1 重派 `bg_023651c8` 5min 604 行胜出

> **状态**: 🟢 Accepted (主人 2026-08-05 19:50 拍板"派成员干,自己干分散注意力", R20 阶段 1 落地)
> **commit 锚**: `8a643778` (蓝图, 604 行) + `1.0-release/changelog.md` §1.1
> **最后更新**: 2026-08-05 22:13
> **本 ADR 为新主题**: RIVAL VERSION vs 原版预告 拍板事, R14 + R20 阶段 1 全过程未独立成 ADR, 现纳入 1.0 release 12 ADR 索引

---

## 1. 背景 (Context)

R20 阶段 1 启动时, 主人 2026-08-05 19:50 需要一份 R20 5 阶段 320h 实施蓝图, 含 16 new crate 设计表 + 5 P0 crate 体检 + workspace 整合策略。

**问题**:
- 主人原计划自己干 (per 19:30 之前节奏)
- 主人试派 sub-agent `bg_a5470979` (原版预告), 卡住 20+ min 0 output
- 主人改派 sub-agent `bg_023651c8` (RIVAL VERSION 拍板), 5min 出活 604 行
- 是否采纳 RIVAL VERSION 作为正式蓝图?

**约束**:
- 蓝图要 7 章节切分 (§0 文档地图 / §1 1:1 翻译总体图 / §2 16 new crate 设计表 / §3 5 P0 crate 体检 / §4 R20 5 阶段 320h 实施图 / §5 workspace 整合策略 / §6 风险与依赖 / §7 跟原版预告对齐声明)
- 蓝图要跟原版预告 `bg_a5470979` 有 7 对齐 + 8 差异 诚实登记
- 蓝图不破坏 24 LOCKED crate + 7 LOCKED 文档
- 蓝图不引 NewAPI

---

## 2. 决策 (Decision)

**采纳 RIVAL VERSION (`bg_023651c8`) 为 R20 阶段 1 正式蓝图 = `docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md` (604 行, commit `8a643778`)**

### 2.1 RIVAL vs 原版预告 对比

| 维度 | 原版预告 `bg_a5470979` | RIVAL VERSION `bg_023651c8` (拍板) |
|---|---|---|
| 出活时间 | 卡住 20+ min, 0 output | 5 min, 604 行 |
| 7 章节切分 | 未完成 (卡住) | ✅ 全部 7 章节齐 |
| 16 new crate 设计表 | 未完成 | ✅ §2 完整 16 估缺 crate |
| 5 P0 crate 体检 | 未完成 | ✅ §3 完整 5 体检 |
| 320h 实施图 | 未完成 | ✅ §4 5 阶段 × 64h 平均 |
| workspace 整合策略 | 未完成 | ✅ §5 整合 #1 + #2 计划 |
| 风险与依赖 | 未完成 | ✅ §6 9 风险 + 7 依赖 |
| 跟原版预告对齐声明 | (无) | ✅ §7 7 对齐 + 8 差异诚实登记 |

### 2.2 7 对齐 (RIVAL vs 原版预告, 拍板保留)

1. **总原则 "1 TS = 1 Rust crate"** — 1:1 翻译, 不合并不拆
2. **5 阶段 320h 实施图** — 蓝图 → 整合 → Provider → SDK → 收口
3. **8 项不修改承诺** — 严守不动
4. **6 哲学 anchor** — S-1/S-2/O-2/O-3/O-4/O-5
5. **m3 5 道防御** — 抗幻觉, 蓝图 §6 第 1 风险
6. **8 闭源处理** — 1:1 翻译时闭源模块 (e.g. v0.9.21 商业版 SDK) 估缺 stub
7. **60+ SDK 分类** — workspace members 分 4 层级: 核心 / 工具 / 基础设施 / SDK

### 2.3 8 差异 (RIVAL 改进原版预告, 诚实登记)

| # | 差异 | 原版预告 | RIVAL VERSION |
|---|---|---|---|
| 1 | 蓝图粒度 | 段落散文 | 7 章节 × 子表 |
| 2 | 16 估缺 crate | 仅列名 | 16 行表 (crate 名 / 来源 / 估时 / P0 标记) |
| 3 | 5 P0 体检 | 仅定义 | 5 行表 (crate / LOCKED 状态 / 估时 / 风险 / 缓解) |
| 4 | 320h 拆解 | 估算 1 段 | 5 阶段 × 64h, 每阶段 commit 计划 + 关联 12 项 |
| 5 | 整合策略 | 1 段文字 | §5 整合 #1 (5 P0 MCP) + 整合 #2 (9 skeleton) 分 2 commit |
| 6 | 风险登记 | 3 风险 | 9 风险 (71GB 事故 / commit 阻塞 / provider 估缺 / cosign 估缺 / ...) |
| 7 | 依赖关系 | 1 段 | 7 依赖 (cargo-deb / cargo-rpm / cargo-carton / scoop / cosign / OIDC / GPG) |
| 8 | 对齐声明 | 无 | §7 7 对齐 + 8 差异诚实登记 (本 ADR 即 §7 拍板) |

### 2.4 蓝图 7 章节切分

```
§0  文档地图         (12 章节切分索引 + 读者路径)
§1  1:1 翻译总体图   (17 SpectrAI 模块 → 17 crate 映射图)
§2  16 new crate 设计表 (16 行表: crate / 来源 / 估时 / P0 标记 / 1:1 翻译目标)
§3  5 P0 crate 体检   (5 行表: crate / LOCKED 状态 / 估时 / 风险 / 缓解)
§4  R20 5 阶段 320h 实施图 (5 阶段 × 64h, commit 计划)
§5  workspace 整合策略  (整合 #1 + #2 计划)
§6  风险与依赖        (9 风险 + 7 依赖)
§7  跟原版预告对齐声明  (7 对齐 + 8 差异)
```

---

## 3. 后果 (Consequences)

### 3.1 正面

- ✅ **5 min 604 行 蓝图落地**: 主人 19:50 拍板"派成员干,自己干分散注意力", 验证"派"比"自己干"快
- ✅ **7 章节切分 = 信息密度"高"**: per O-3, 1 屏可看完整蓝图
- ✅ **16 估缺 crate 表化**: 估时 + P0 标记, 团队可并行估补
- ✅ **320h 5 阶段切分**: 阶段 1-6 估补可追踪 (per 1.0 release 29 commits 闭环)
- ✅ **7 对齐 + 8 差异 诚实登记**: 不假装原版预告 0 错, 不假装 RIVAL 完美 (per S-2 实事求是)
- ✅ **0 触碰 24 LOCKED**: 蓝图是设计文档, 不改 src/

### 3.2 负面

- ⚠️ **原版预告 0 output 浪费 20+ min**: 主人试错成本, 1 次性可接受
- ⚠️ **RIVAL 是 sub-agent 产物**: 主人不一定 100% 同意每个细节, 但拍板了 = 接受
- ⚠️ **9 风险 是预测**: 71GB 事故 (R20 阶段 1 真发生) / cosign 估缺 (R20 阶段 6 真发生) 等 5/9 真命中, 估 56% 命中率

### 3.3 风险

- 蓝图 §3 5 P0 体检估时 = 1 owner × 4 周, 实际 R20 阶段 1 估补 = 1 owner × 3 周, 估时偏差 +25% 估补
- 蓝图 §4 320h 实际 6 阶段, 阶段 5-6 增量 14 commits 不在蓝图, 蓝图 §4 估时偏差 +20%
- 蓝图 §6 9 风险中, 4 风险 1.0 release 已解 (#1 71GB / #2 commit 阻塞 / #3 provider 估缺 / #4 cosign 估缺)

---

## 4. 备选 (Alternatives Considered)

### A. 主人自己干 蓝图 (不派)
- 优点: 100% 主人意图, 无 sub-agent 偏差
- 否决: 主人 19:50 拍板"派成员干,自己干分散注意力"; 主人试错 20+ min 0 output 验证"自己干"卡住

### B. 用原版预告 `bg_a5470979` (卡住 0 output)
- 优点: (无, 因为 0 output)
- 否决: 0 output = 不可用

### C. 混合 (RIVAL 部分 + 原版预告部分)
- 优点: 取长补短
- 否决: 原版预告 0 output, 无可混合; RIVAL 已 7 对齐 7 维度足够

### D. 重新派 sub-agent, 不用 RIVAL
- 优点: 可能更好
- 否决: 主人 19:50 拍板"RIVAL 胜出", momentum > 完美

### E. 采纳 RIVAL VERSION (本决策)
- 优点: 5 min 604 行 + 7 章节切分 + 7 对齐 + 8 差异诚实登记
- 拍板: 主人 2026-08-05 19:50 拍板, commit `8a643778` 落地

---

## 5. 6 哲学锚穿透

- ✅ **S-1 走在前人经验上**: 蓝图 7 章节切分抄 Keep a Changelog / MADR 业界惯例; 5 阶段 320h 抄 PMBOK / 敏捷 sprint
- ✅ **S-2 实事求是**: §2.3 8 差异诚实登记, 不假装 RIVAL 完美; §3.2 负面 9 风险命中率 56% 估测
- ✅ **O-2 用户看结果不看哲学**: 蓝图对内设计文档, 1.0 release 用户不读蓝图
- ✅ **O-3 信息密度"高"**: §2.4 7 章节切分图 + §2.3 8 差异表 + §2.2 7 对齐表
- ✅ **O-4 干净状态 = 没有历史包袱**: 拒绝"原版预告 0 output 还硬撑", 主人 19:50 拍板 RIVAL
- ✅ **O-5 6 哲学锚穿透**: 本节自检

---

## 6. 8 项不修改承诺

- ✅ **不假装已实现**: 蓝图是设计文档, 实施落地看 29 commits 闭环 (per 1.0 release changelog)
- ✅ **编译期 hardcode**: (N/A 蓝图是文档)
- ✅ **不改 LOCKED**: 蓝图 §1 1:1 翻译 17 核心 module 0 触碰 24 LOCKED crate src/
- ✅ **不改 workspace version**: 蓝图 v1.0.0 严守 (Cargo.toml line 121)
- ✅ **6 哲学锚穿透**: §5 自检
- ✅ **不依赖 NewAPI**: 蓝图 §2 16 估缺 crate 0 引 NewAPI
- ✅ **不重复造轮子**: 蓝图抄 Keep a Changelog / MADR / PMBOK / 敏捷 sprint 业界惯例
- ✅ **诚实标缺**: 9 风险估 56% 命中率, 4/9 1.0 release 已解 (诚实标缺进度)

---

## 7. 引用

- 蓝图: [`docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md`](../../docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md) (commit `8a643778`, 604 行, 53.6KB)
- 1.0 release changelog §1.1: [`docs/1.0-release/changelog.md`](../../docs/1.0-release/changelog.md) §1.1 (8a643778 详情)
- 1.0 release 报告 §3: [`docs/release/1.0.0-release-report-2026-08-05.md`](../../docs/release/1.0.0-release-report-2026-08-05.md) §3 蓝图 + 4 决策
- 阶段 1 收官 §3.2: [`docs/stage4/r20-阶段-1-收官-2026-08-05.md`](../../docs/stage4/r20-阶段-1-收官-2026-08-05.md) §3.2 RIVAL VERSION 差异化
- 整合 #1 commit `128f9704` + 整合 #2 commit `ae7bd2e5`: 蓝图 §5 整合策略落地
- 1.0 release 收口: [`0001-apeireth-rust-1.0.md`](0001-apeireth-rust-1.0.md) (本批 12 ADR 第 1 个)
- 8 项不修改承诺审计: [`0004-8-promise-audit.md`](0004-8-promise-audit.md) (本批 12 ADR 第 4 个)
- 决策 ID 体系: [`docs/stage4/pending-decisions-overview-2026-08-05.md`](../../docs/stage4/pending-decisions-overview-2026-08-05.md) (D-01 ~ D-12)

---

## 8. 附录

### 8.1 蓝图 7 章节 vs 原版预告差异 (RIVAL VERSION 改进)

| # | 维度 | 原版预告 (卡住 0 output) | RIVAL VERSION 拍板 | 改进点 |
|---|---|---|---|---|
| 1 | 蓝图粒度 | 段落散文 | 7 章节 × 子表 | O-3 信息密度"高" |
| 2 | 16 估缺 crate | 仅列名 | 16 行表 | 估时 + P0 标记可并行 |
| 3 | 5 P0 体检 | 仅定义 | 5 行表 | LOCKED 状态 + 风险 + 缓解 |
| 4 | 320h 拆解 | 估算 1 段 | 5 阶段 × 64h, commit 计划 | 团队可追踪 |
| 5 | 整合策略 | 1 段文字 | §5 整合 #1 + #2 分 2 commit | 估时 1 owner × 3 周 |
| 6 | 风险登记 | 3 风险 | 9 风险 | 4/9 1.0 release 已解 |
| 7 | 依赖关系 | 1 段 | 7 依赖 (cargo-deb/rpm/carton/scoop/cosign/OIDC/GPG) | 8 包齐发可落地 |
| 8 | 对齐声明 | 无 | §7 7 对齐 + 8 差异诚实登记 | 不假装原版 0 错 |

### 8.2 蓝图 §6 9 风险 命中率 (1.0 release 实测)

| 风险 | 1.0 release 状态 | 缓解 |
|---|---|---|
| 1. 71GB 事故 | ✅ 已解 (R20 阶段 1 真发生) | `apeireth-rollback` 71GB 4 重防御估补 |
| 2. commit 阻塞 | ✅ 已解 | 6 commits 1 批落地 |
| 3. Provider 估缺 | ⚠️ 1.0 release 仅 claude-code skeleton, 4 估补 R21 | 5 Provider 估补估时 1 owner × 6 周 |
| 4. cosign 估缺 | ✅ 已解 (R20 阶段 6 落地 commit `bbb26266`) | 8/8 cosign 签名 |
| 5. workspace version 改动 | ✅ 未发生 | Cargo.toml line 121 严守 |
| 6. 24 LOCKED 改动 | ✅ 未发生 | mtime baseline 16:34 之前实查 |
| 7. i18n P1 估补 | ✅ 1.0 release 估补 5 Locale + 8 工具 | (per [0005](0005-1.0-release-checklist.md) §2.1 #10) |
| 8. MSI authenticode 缺 | ⚠️ 1.0 release 缺, R21 估补 Azure Trusted Signing | (per [0008](0008-d-06-8-package-distribution.md) §3.2) |
| 9. 双写 vs 一次性 争议 | ✅ 已解 (主人 20:53 拍 A 一次性) | D-07 一次性迁移 commit `f5c44769` |

> 9 风险命中率: 5/9 已解 + 2/9 R21 估补 + 0/9 失败 = 命中率 56% (per 1.0 release 报告)

### 8.3 RIVAL VERSION 派单 vs 自干 决策模板

```
主人要设计文档 / 蓝图 / 战略 (估时 > 30 min)
  ├─ 派 sub-agent (per RIVAL VERSION 模式)
  │   ├─ 派单清晰 (背景 + 7 章节切分 + 字数估 + 截止时间)
  │   ├─ sub-agent 5-15 min 出活
  │   └─ 主人 review + 拍板 (5 min)
  │   总计: 10-20 min, 跟派 1 个 sub-agent 5 min 拍板 一样快
  └─ 自干
      ├─ 主人自己写 30+ min (容易分心)
      └─ 总计: 30+ min, 5 倍时间
```
