# 阶段 1-5 设计层索引 (R11 LOCKED)

> **R119-3b Mavis 重建 (2026-08-10)**: OMNIBUS 阶段 1+2+3+4+5 内容下沉到 docs/stage1-6/ 子目录,本文件是索引。

```
[Document-Meta]
Document: docs/omnibus/stage1-5.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3b
Last-Modified: 2026-08-10
Status: 🟢 活跃 (索引层)
```

## 阶段 1: 灵感 (2201 行, R11 LOCKED)

- [`docs/stage1/inspiration-stage1-2026-07-30.md`](../../stage1/inspiration-stage1-2026-07-30.md) — 2201 行灵感 (137KB, mtime 2026/8/6 8:06:43 R11 LOCKED)
- [`docs/stage1/README.md`](../../stage1/README.md) — 索引

## 阶段 2: 想法设计 (18 stage2 + 1 D2 增补 = 19 文件, R11 LOCKED)

- [`docs/stage2/`](../../stage2/) — 19 文件 (~315KB)
  - `stage2-decisions-tech-stack.md` (10.7KB) — 1 技术栈
  - `stage2-decisions-architecture.md` (12.7KB) — 2 架构形态
  - `stage2-decisions-crate-split.md` (14.9KB) — 3 crate 划分
  - `stage2-decisions-process-threading.md` (12.4KB) — 4 进程/线程/协程
  - `stage2-decisions-memory-layout.md` (8.3KB) — 5 内存布局
  - `stage2-decisions-persistence.md` (18.1KB) — 6 持久化
  - `stage2-decisions-llm-integration.md` (18.8KB) — 7 LLM 集成
  - `stage2-decisions-modularity.md` (13.8KB) — 8 模块化
  - `stage2-decisions-communication-bus.md` (15.0KB) — 9 通信总线
  - `stage2-decisions-council-impl.md` (29.3KB) — 10 智囊团实现
  - `stage2-decisions-upgrade-impl.md` (24.9KB) — 11 自我升级实现
  - `stage2-decisions-philosophy-guard.md` (25.0KB) — 12 哲学守门实现
  - `stage2-decisions-permission-packs.md` (5.5KB) — 权限包
  - `stage2-decisions-decision-system.md` (6.1KB) — 决策系统
  - `stage2-decisions-source-projects-list.md` (9.7KB) — 借鉴项目清单
  - `stage2-decisions-drift-revision-tracker.md` (18.1KB) — 漂移修订追踪
  - `stage2-decisions-appendix-references.md` (9.1KB) — 附录引用
  - `stage2-decisions-addendum-sovereignty-continuity-governance.md` (54.4KB) — D2 增补(主权/连续性/治理)
- [`docs/stage2/README.md`](../../stage2/README.md) — 索引

## 阶段 3: 画图纸 (14 文件, R11 LOCKED)

- [`docs/stage3-blueprints/`](../../stage3-blueprints/) — 14 文件 (~190KB)
  - `00-stage3-overview.md` (5.8KB) — 概览
  - `01-overall-architecture.md` (15.0KB) — P1 整体架构
  - `02-process-topology.md` (13.2KB) — P2 进程拓扑
  - `03-decision-flow.md` (22.9KB) — P3 决策流(含 §3.8 双洋葱 + §3.10 反思期)
  - `04-upgrade-flow.md` (14.3KB) — P4 升级流(含 §4.8 HA 4 实现)
  - `05-r-measure-test-flow.md` (11.2KB) — P5 R-Measure 真测
  - `borrowed-from-projects.md` (37.4KB) — 借鉴项目
  - `borrowed-from-r11.md` (19.1KB) — 借鉴 R11
  - `double-onion-explicitization-2026-07-31.md` (23.9KB) — 双洋葱显式化
  - `explanation-01.md` (4.6KB) — 解释 1
  - `explanation-02.md` (3.8KB) — 解释 2
  - `explanation-03.md` (4.0KB) — 解释 3
  - `explanation-04.md` (4.2KB) — 解释 4
- [`docs/stage3-blueprints/README.md`](../../stage3-blueprints/README.md) — 索引

## 阶段 4: 落实架构 (50+ 文件, R11 LOCKED + R19+ 集成期增量)

- [`docs/stage4/`](../../stage4/) — 50+ 文件 (~1.5MB)
  - **R11 LOCKED** (核心文档, baseline 16:34 之前):
    - `architecture-stage4-engineering-landing.md` (72.6KB) — 阶段 4 落实架构(1,492 行)
    - `stage4-runtime-architecture-revised.md` (77.0KB) — 阶段 4 运行时架构
    - `stage4-correction-v3..v15` (13 LOCKED 修正链文件)
    - `stage4-external-feedback-and-revisions.md` (20.8KB)
    - `stage4-patches-v2-crate-correction.md` (14.4KB)
    - `stage4-thinking-document.md` (21.6KB)
  - **R19+ 集成期** (8/5 增量):
    - `8-locked-unified-2026-08-05.md` (15.9KB) — 8 项不修改承诺统一
    - `r19-integration-doc-index-2026-08-05.md` (16.6KB) — R19 集成期索引
    - `r19-integration-quickstart-2026-08-05.md` (26.3KB) — R19 集成期 5min 入口
    - `r19-integration-mermaid-overview-2026-08-05.md` (47.1KB) — 6 张 Mermaid
    - `r19-integration-commit-template-2026-08-05.md` (40.5KB) — 5 类 commit 模板
    - `r19-r20-stage-unified-2026-08-05.md` (18.9KB) — 3 套阶段对照
    - `pending-decisions-overview-2026-08-05.md` (13.7KB) — 12 项 D-# 拍板
    - `docs-maintenance-sop-2026-08-05.md` (31.6KB) — 5 步 SOP
    - `v09021-rust-translation-blueprint-2026-08-05.md` (54.4KB) — RIVAL 蓝图
    - `v09021-commercial-extract-2026-08-05.md` (14.4KB)
    - `global-architecture-map-2026-08-05.md` (50.1KB) — 13 张 Mermaid
    - `apeireth-formal-invariants-2026-08-05.md` (63.5KB) — 5 Kani invariants
    - `apeireth-session-blueprint-2026-08-05.md` (78.9KB) — session 蓝图
    - `apeireth-team-lead-implementation-guide-2026-08-05.md` (41.8KB) — team-lead 实施
    - `apeireth-sdk-gap-analysis-2026-08-05.md` (21.1KB)
    - `apeireth-architecture-readonly-review-2026-08-05.md` (33.8KB)
    - `apeireth-engineering-optimization-2026-08-05.md` (58.7KB)
    - `r-measure-verification-design-2026-08-05.md` (49.5KB) — R-Measure 守门
    - `spectrAI-integration-blueprint-r19-plus-2026-08-05.md` (64.2KB)
    - `spectrai-branch-coverage-audit-2026-08-05.md` (47.5KB)
    - `commercial-vs-fork-diff-2026-08-05.md` (17.6KB)
    - `m3-hallucination-defense-2026-08-05.md` (43.2KB)
    - `8-NEXT-UPGRADE-DIRECTIONS.md` (54.7KB) — R19 整合升级方向
    - `5-provider-tool-mapping-2026-08-05.md` (53.5KB)
    - `yinta-fork-audit-2026-08-05.md` (47.8KB)
    - `d-01-d-12-commit-plan-2026-08-05.md` (19.7KB)
    - `architecture-frontend-design-proposal.md` (15.0KB)
    - `architecture-stage4-inspiration-supplements.md` (12.2KB)
    - `glossary-spectrAI-additions-2026-08-05.md` (11.3KB)
    - `organ-public-api-survey-2026-08-06.md` (3.6KB)
    - `tauri-assets-from-spectrAI-2026-08-05.md` (16.1KB)
    - `tauri-team-collab-sop-2026-08-05.md` (23.2KB)
    - `supervisor-prompt-818-summary-2026-08-05.md` (56.7KB)
    - `stage4-correction-v10-versioning-system.md` (11.4KB)
    - `stage4-correction-v11-conventions.md` (6.6KB)
    - `stage4-correction-v12-final-check.md` (1.5KB)
    - `stage4-correction-v13-placeholder-dirs.md` (2.2KB)
    - `stage4-correction-v14-final-cleanup.md` (4.2KB)
    - `stage4-correction-v15-four-gates-permission-grant.md` (11.4KB)
- [`docs/stage4/README.md`](../../stage4/README.md) — 索引

## 阶段 5: 施工文档 (631 行, R11 LOCKED)

- [`docs/stage5/stage5-construction-document.md`](../../stage5/stage5-construction-document.md) — 33.4KB, 631 行 (R11 LOCKED)
- [`docs/stage5/construction-kickoff-manual.md`](../../stage5/construction-kickoff-manual.md) — 26.1KB, 旧 kickoff 手册 (向后兼容)
- [`docs/stage5/README.md`](../../stage5/README.md) — 索引

## 思想层 LOCKED 严守 (per 主人 8/10 01:14)

- 🔒 阶段 1+2+3+5 内容严守 (R11 LOCKED baseline 16:34 之前)
- 🔒 阶段 4 LOCKED 修正链严守 (v3-v15)
- 🟢 阶段 4 R19+ 集成期文档 (8/5 增量) 形式可调
- 形式可调: 文件位置 + 章节结构 (per R119 主人 8/10 01:14)

## 6 哲学锚穿透

- **S-1**: 阶段 1-5 服务 ASI 北极星
- **S-2**: 核验后写, R11 LOCKED baseline 16:34 之前
- **O-5**: 思想层严守, 形式按主人 R119 拍板调整

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
