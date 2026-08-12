# apeireth-tool-codesearch

**R140** — 代码搜索工具

## 职责

代码仓库语义搜索: grep / ripgrep / AST 切分 / 调用图.

## 核心能力

- regex 全文搜索
- AST 切分 (tree-sitter 集成)
- 跨文件引用追踪
- 增量索引

## 区别

- `apeireth-tool-codesearch` (本 crate): 代码结构理解
- `apeireth-tool-search` (R145): 通用全文 + 聚合

## 0 假装

✅ 47 单元测试 | ⚠️ tree-sitter AST 在 feature flag 下
