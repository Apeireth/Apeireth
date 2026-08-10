# 13 Document-Meta 元信息格式

> **R119-3a-1 Mavis 重建 (2026-08-10)**: 从 APEIRETH-CONVENTIONS.md §13 (Document-Meta) 拆出,核验后写。

```
[Document-Meta]
Document: docs/conventions/13-document-meta.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 格式 (核验后)

**任何 Apeireth 文档顶部 = 同一格式**:

```markdown
[Document-Meta]
Document: <文档路径>
Version: <Manual-Rev-X> + <Design-X.Y> + <Fix-N>
R-Cycle: <R-N>
Commit: <commit-hash>
Last-Modified: <YYYY-MM-DD>
Status: <🔒 LOCKED / 🟢 活跃 / 🟡 辅助 / 🔴 替代>
```

## 7 字段说明

| 字段 | 格式 | 例子 (核验) |
|---|---|---|
| `Document` | 文档相对路径 | `docs/conventions/13-document-meta.md` |
| `Version` | `Manual-Rev-X + Design-X.Y + Fix-N` (空格分隔) | `Manual-Rev-L + Fix-17` |
| `R-Cycle` | 当前 R 周期 | `R119-3a-1` |
| `Commit` | git commit hash | `5c546a84` |
| `Last-Modified` | YYYY-MM-DD | `2026-08-10` |
| `Status` | 4 种 emoji 之一 | `🟢 活跃` |

## 例子

### 例子 1: 顶层规范文件

```
[Document-Meta]
Document: docs/conventions/13-document-meta.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Commit: (本批)
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

### 例子 2: R11 LOCKED 设计层

```
[Document-Meta]
Document: docs/stage1/inspiration-stage1-2026-07-30.md
Version: Design-1.0
R-Cycle: R14
Commit: (历史)
Last-Modified: 2026-07-30
Status: 🔒 LOCKED
```

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
