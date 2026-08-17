# 12 架构图编号系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §12 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/12-arch-diagram.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 5 主图

| 编号 | 文档 | 主题 | 状态 |
|---|---|---|---|
| **P1** | [`docs/stage3-blueprints/01-overall-architecture.md`](../../stage3-blueprints/01-overall-architecture.md) | 整体架构 | 🔒 LOCKED |
| **P2** | [`docs/stage3-blueprints/02-process-topology.md`](../../stage3-blueprints/02-process-topology.md) | 进程拓扑 | 🔒 LOCKED |
| **P3** | [`docs/stage3-blueprints/03-decision-flow.md`](../../stage3-blueprints/03-decision-flow.md) | 决策流 (含 §3.8 双洋葱 + §3.10 反思期) | 🔒 LOCKED |
| **P4** | [`docs/stage3-blueprints/04-upgrade-flow.md`](../../stage3-blueprints/04-upgrade-flow.md) | 升级流 (含 §4.8 HA 4 实现) | 🔒 LOCKED |
| **P5** | [`docs/stage3-blueprints/05-r-measure-test-flow.md`](../../stage3-blueprints/05-r-measure-test-flow.md) | R-Measure 真测 (含 v2 §9 12 维度) | 🔒 LOCKED |

## R119 实际 Mermaid 图

- `docs/stage4/r19-integration-mermaid-overview-2026-08-05.md` (6 张 Mermaid 总览)
- `docs/1.0-release-prep/07-architecture-mermaid.md` (E-7 草稿, 三架构 mermaid 图: v2 → v4 → v4.1 → 22 trait 互锁)

## 核验

- ✅ 5 主图 P1-P5 (R11 LOCKED 阶段 3 蓝图)
- ✅ R19+ 集成期 6 张 Mermaid
- ✅ R119 1.0 release E-7 草稿

## 不漂移

- 0 触碰任何 LOCKED 文档 (P1-P5 LOCKED)
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
