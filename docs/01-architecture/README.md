# docs/01-architecture — 架构说明（常青）

> **性质**：L3 常青 —— 长期有效的"系统是什么/为什么这么设计"。
> **不是**：施工日志与阶段记录（那些在 `../04-internal/history/`，是历史证据）。

## 现行核心（先读这几篇）

| 文件 | 讲什么 |
|---|---|
| `philosophy.md` | **9 哲学锚 + 14 键 + 三洋葱**（LOCKED 本体，0 触碰） |
| `vision.md` | 产品愿景与定义（「不假装有心」）、五原型与产品北极星 |
| `architecture.md` | 系统总览（当前工作区与独立桌面工作区） |
| `system-capabilities.md` | 全域能力规范手册（能力体系 + 安全治理契约 + **四级口径定义**） |
| `security.md` | 安全模型（对齐实际机制） |
| `brand.md` | 品牌：命名、宣言、Logo 设计纲要 |
| `engineering-report.md` | 工程报告（历史实测数据与里程碑） |

## 其它

| 类别 | 文件 |
|---|---|
| 阶段施工史（已归拢） | `m1a-*` `m1b*` `m1c-*` `m2a-*` `m2b-*` `m2c-*` `m2d-*` `m3a-*` → 已合入 [`../04-internal/history/01-reconstruct-v2-stage-record.md`](../04-internal/history/01-reconstruct-v2-stage-record.md) |
| 专项设计 | `cognitive-9-organ-integration-spec.md`、`organ-orchestrator-spec.md`、`organ-orchestrator-completion-plan.md`、`forget-three-phase-production-spec.md`、`deferred-slot-activation-preference_learning-spec.md` |
| 审计与对照 | `canonical-skeleton-freeze-audit.md`、`master-functionality-port-audit.md`、`reconstruct-v2-migration-map.md`、`v2-architecture-reflection.md` |
| 对外/研究 | `v2-master-lineage-and-upgrade-blueprint.md`、`vcp-vs-apeireth-deep-comparison.md`、`apeireth-true-understanding-2026-08-28.md` |

## 纪律

- **LOCKED 5 项**（9 锚本体 / 键集 / 3 脊柱 / `workspace.version` / R11 baseline）**0 触碰**。
- 本目录不写"当前有多少个 crate"这类会漂的数字 —— 引用 [`../03-reference/crates.md`](../03-reference/crates.md)。
