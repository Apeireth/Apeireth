# 04 成就编号系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §4 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/04-achievement.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`A<n>`

## A1-A20 范围与主题

| 范围 | 主题 |
|---|---|
| **A1-A8** | 最小可行 demo |
| A1 | CLI 启动 |
| A2 | 集成测试 |
| A3 | 12 键 |
| A4 | SQLite |
| A5 | AND 门 |
| A6 | CI |
| A7 | Self-Disable |
| A8 | R-Measure |
| **A9-A17** | 9 器官 crate |
| A9 | perception |
| A10 | cognition |
| A11 | action + motivation + value |
| A12 | consciousness + relation |
| A13 | life-force |
| A14 | council |
| A15 | upgrade |
| A16 | bus + extension + pybridge |
| A17 | philosophy 物理删除 |
| **A18-A20** | 收尾 |
| A18 | OTA |
| A19 | Cognitive-Dream |
| A20 | 17 crate 集成 |

## 报告路径(per §5 报告路径系统)

`reports/achievement-A<n>-<role>-<name>.md` (单个成就)
`reports/retrospective-A<n>-<role>.md` (5 个成就回顾)
`reports/final-A20-<role>-<name>.md` (最终总结)

## 核验

- 实际 reports/achievement-*.md 文件: 20+ (R11-R14 阶段产出)
- 实际 reports/retrospective-*.md 文件: 5+ (R11-R14 阶段产出)
- 实际 reports/final-A20-*.md 文件: 1+ (A20 收尾)

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
