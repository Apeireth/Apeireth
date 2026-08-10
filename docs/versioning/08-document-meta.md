# 08 Document-Meta 格式 (跨系统)

> **R119-3a-1 Mavis 重建 (2026-08-10)**: Document-Meta 格式跨 7 子系统 + 12 子规范 + 21 词条。

```
[Document-Meta]
Document: docs/versioning/08-document-meta.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 跨系统统一

Document-Meta 格式是 **Apeireth 7 子系统 + 12 子规范 + 21 词条** 顶层任何文档的统一头部。详见 [`docs/conventions/13-document-meta.md`](../conventions/13-document-meta.md)。

## 例子 (跨 3 文档系统)

### 例子 1: 顶层规范 (conventions/)

```
[Document-Meta]
Document: docs/conventions/01-namespace.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

### 例子 2: 版本号系统 (versioning/)

```
[Document-Meta]
Document: docs/versioning/01-code.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-1
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

### 例子 3: 词条术语 (glossary/)

```
[Document-Meta]
Document: docs/glossary/01-north-star.md
Version: Manual-Rev-L + Fix-17
R-Cycle: R119-3a-2
Last-Modified: 2026-08-10
Status: 🟢 活跃
```

## 兼容方案 (R119 形式撤销后)

**方案 C**: 仅元信息加新版本号, 文件名保留向后兼容

```
文件名 (保留): stage4-correction-v3-onion-embedded-keys-gates.md
元信息 (新增): Version: Fix-3 / Design-4.0 / R14
```

- ✅ 所有 v1-v17 修正链文件保留 (向后兼容)
- ✅ 仅顶部加 Document-Meta
- ✅ 不破坏 LOCKED 内容

## 不漂移

- 0 触碰任何 LOCKED 文档
- 0 改 workspace.version
- 0 改 R11 baseline 3 值
