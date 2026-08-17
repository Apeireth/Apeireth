# 02 设计层版本 (Design-X.Y)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-VERSIONING.md §2 拆出,核验后写。

```
[Document-Meta]
Document: docs/versioning/02-design.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式

`Design-MAJOR.MINOR` (独立编号, 按阶段)

## 设计层 LOCKED 清单 (核验后)

| 阶段 | 设计层版本 | 状态 | 位置 (R119 重建后) |
|---|---|---|---|
| 阶段 1 灵感 | `Design-1.0` | 🔒 LOCKED | `docs/omnibus/stage1/` (R119-3b 拆) |
| 阶段 2 想法设计 | `Design-2.0` | 🔒 LOCKED | `docs/omnibus/stage2/` (R119-3b 拆) |
| 阶段 2 D2 增补 | `Design-2.1` | 🔒 LOCKED | `docs/omnibus/stage2/d2/` (R119-3b 拆) |
| 阶段 3 画图纸 | `Design-3.0` | 🔒 LOCKED | `docs/omnibus/stage3-blueprints/` (R119-3b 拆) |
| 阶段 4 落实架构 | `Design-4.0` | 🔒 LOCKED | `docs/omnibus/stage4/` (R119-3b 拆) |
| 阶段 5 施工文档 | `Design-5.0` | 🔒 LOCKED | `docs/omnibus/stage5/` (R119-3b 拆) |
| 主手册 | `Design-omnibus-1.0` | 🔒 LOCKED | `docs/omnibus/` (R119-3b 拆) |

## v2 / v4 / v4.1 哲学层 (核验后)

| 哲学层 | 状态 | 位置 (R119 重建后) |
|---|---|---|
| 立体架构 v2 | 🔒 LOCKED | `docs/omnibus/design-v2/` (R119-3b 拆) |
| 生命架构 v4 | 🔒 LOCKED | `docs/omnibus/design-v4/` (R119-3b 拆) |
| 哲学层升级 v4.1 | 🔒 LOCKED | `docs/omnibus/design-v4.1/` (R119-3b 拆) |
| 基础架构 v6 (4 重守门 + 权限发放 + E 层修改路径) | 🔒 LOCKED | `docs/omnibus/design-v6/` (R119-3b 拆) |

## 当前标识

`Design-1.0..5.0-R14` (LOCKED, 0 改)

## R119 严守原则

- 🔒 设计层 LOCKED 范围 (Design-1.0..5.0 + Design-omnibus-1.0) **内容不动**
- 🟢 形式可调 (per 主人 8/10 01:14 拍板"推倒重建") — 文件位置可调, 内容严守
- ✅ 7 个设计层 + 4 个哲学层 = 11 个 LOCKED 设计层

## 不漂移

- 0 触碰任何 LOCKED 设计层
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
