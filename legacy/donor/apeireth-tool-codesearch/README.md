# apeireth-tool-codesearch

> Apeireth R140: code search + knowledge graph (regex + Aho-Corasick + symbol extraction), 12 MCP tools, borrows codebase-memory-mcp design

apeireth-tool-codesearch 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (15 src 文件 / 112 测试 + 2 Kani proof)

- `src/lib.rs` — 入口 re-export (ToolBridge 装配)
- `src/search.rs` — regex + Aho-Corasick 多模式搜索 + 8 测试
- `src/files.rs` — find_files / list_languages 跨 FS 扫描 (ignore 0.4 + walkdir 2.5) + 6 测试
- `src/symbols.rs` — symbol extraction (regex 版; tree-sitter feature 启用时切换 AST) + 14 测试
- `src/index.rs` — sqlite-backed 倒排索引 + 5 测试
- `src/cache.rs` — 索引缓存层 (LRU + 持久化) + 10 测试
- `src/lru_cache.rs` — LRU 内核 (lru crate 二次封装) + 12 测试
- `src/graph.rs` — import/caller 知识图谱 + 5 测试
- `src/unified.rs` — R202/203 unified 6-dim query facade + 15 测试
- `src/ast_grep.rs` — R201 AST-level search (ast-grep CLI 桥接, 可选 feature) + 8 测试
- `src/pure_pattern.rs` — 纯 Rust pattern matcher (无 tree-sitter 后备) + 8 测试
- `src/mcp.rs` — MCP server (12 工具: SearchText/FindFiles/ExtractSymbols/ListLanguages/LookupSymbol/IndexFile/IndexStats/TraceImports/FindCallers/ProjectOverview/AstGrepSearch/UnifiedQuery) + 8 测试
- `src/register.rs` — ToolBridge catalog 接入 + 1 测试
- `src/compat.rs` — 兼容层 adapter + 4 测试
- `src/enhanced.rs` — enhanced path (含可选 tree-sitter 注入) + 3 测试
- `src/organ_kani_proofs.rs` — codesearch organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
