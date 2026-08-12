# apeireth-tool-search

**R145 VSearch 终极差距补弱** — 全文 + 聚合 + BM25-lite 评分

## 职责

通用搜索引擧: 全文索引 / 聚合查询 / 字段过滤 / 时间桶.

## 核心类型

- `SearchEngine` (内存倒排索引)
- `Document` (id / source / topic / body / tags / timestamp)
- `RankedDoc` (doc + score + matched_terms)
- `AggregateBy` (Source / Topic / TimeBucket)
- `FieldFilter` (source / topic / time range / tag)

## 借鉴

VCP v1.1 "自研 VSearch / VSearch+".

## 区别

- `apeireth-tool-codesearch` (R140): 代码结构 (regex + AST)
- `apeireth-tool-search` (本 crate): 通用全文 + 聚合
- `apeireth-tool-image-process` (R141): 图像感知哈希

## 上升

- BM25-lite 评分 (TF + 长度归一化, 0 依赖 jieba)
- `unicode-segmentation` 切词
- 编译期聚合键类型守门

## 0 假装

✅ 12 单元测试 | ✅ 真实现评分排序 | ✅ 真实现聚合
