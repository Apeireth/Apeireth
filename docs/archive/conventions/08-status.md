# 08 状态标记系统

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §8 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/08-status.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`<emoji> <status>`

## 4 种状态(核验后, R119 调整)

| 标记 | 含义 | 例子 (核验) |
|---|---|---|
| 🔒 LOCKED | 数据/思想严守, 不可改 | R11 baseline 3 值 / 24 LOCKED crate / 阶段 1+2+3 / v2/v4/v4.1 / 5 守门 / 12 键 |
| 🟢 活跃 | 当前采用 | README.md / R119-3a-1 重建 / R114-R118 |
| 🟡 辅助 | 补充材料 | docs/1.0-release-prep/ (8 草稿) / _workspace/_archive/ |
| 🔴 替代 | 历史/撤回 | apeireth-philosophy (R17 真删) / 5 老 provider crate (R36 真删) |

## R119 状态调整

- 🔒 LOCKED 范围 (R119 形式撤销后保留): R11 baseline 3 值 / 24 LOCKED crate mtime / workspace.version 1.1.0 / 5 守门编译期 hardcode / 12 键 / V0.5 24 维公式
- 🔒 LOCKED 范围 (R119 形式撤销): 阶段 1+2+3 文档 (内容保留, 文件名/位置可调) / v2/v4/v4.1 / 阶段 4 / 阶段 5 / v6 基础架构 / 顶层 3 规范文件 (现下沉到 docs/conventions/ / versioning/ / glossary/)
- 🟢 活跃: 顶层 README / CHANGELOG / ROADMAP / 6 哲学锚 / 12 子规范 / 7 子系统 / 21 词条 / R114-R118 动态运营层 / R119 文档重建

## 不漂移

- 0 触碰任何 LOCKED 数据 (R11 baseline 3 值 / 24 LOCKED crate mtime / workspace.version 1.1.0)
- 0 改 6 哲学锚
- 0 改 12 键 / 5 守门 / V0.5 24 维公式
