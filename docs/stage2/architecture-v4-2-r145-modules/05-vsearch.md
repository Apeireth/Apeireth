# 5. VSearch 全文聚合 — S-2 实事求是

```
[Document-Meta]
Document: docs/architecture-v4-2-r145-modules/05-vsearch.md
Version: R145-Init
R-Cycle: R145
Last-Modified: 2026-08-12
Status: 🟢 活跃
```

## 设计

`SearchEngine` 内存倒排索引:
- `HashMap<term, HashSet<doc_id>>` 倒排
- BM25-lite 评分 (TF + 长度归一化)
- 3 维聚合: `Source` / `Topic` / `TimeBucket`(Hour/Day/Week)
- 字段过滤: `FieldFilter` (source / topic / time range / tag)
- 中文分词: `unicode-segmentation` (0 依赖 jieba)

## 借鉴 vs 上升

| VCP | 我们 |
|---|---|
| VSearch + ChromeBridge 网页 + 深网 | 通用全文 + 聚合 (无网页抓取) |
| TF-IDF / 余弦相似度 | BM25-lite (TF + 长度归一化) |
| 实时更新 | 静态索引 + 重新 index |
| 跨域分布式 | 单进程内存 (R146+ 外置) |

## 哲学基础

**S-2 实事求是**: "查得到" = 实事求是的最低要求. VCP 强调深网, 我们强调"自家记忆能查得到".
**O-2 走在前人**: TF-IDF 经典算法, 不重造.

## 局限性

- 内存限制 (大 corpus 需外置)
- 0 语义搜索 (R146+ 接 vector)
- 无增量索引 (重建需全部重做)

## 借鉴

VCP v1.1 "自研 VSearch / VSearch+".

## 区分

- `apeireth-tool-codesearch` (R140): 代码结构 (regex + AST)
- `apeireth-tool-search` (本模块): 通用全文 + 聚合
- `apeireth-tool-image-process` (R141): 图像感知哈希

## 内部参考

- 实现: [`crates/apeireth-tool-search/src/lib.rs`](../../crates/apeireth-tool-search/src/lib.rs)
- 索引: [`README.md`](README.md)
