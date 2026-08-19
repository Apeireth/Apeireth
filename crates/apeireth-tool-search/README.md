# apeireth-tool-search

> Apeireth R145 VSearch: 全文 + 聚合 + BM25-lite (TF + 长度归一化) 排序内存搜索. 区别于 apeireth-tool-codesearch (regex/AST) + apeireth-tool-image-process (perceptual). 上升为 Rust 编译期保证, 字段级复刻 VSearch/VSearch+ (origin: open-source). 聚合查询: group by source / time bucket / topic (3 维度). 字段过滤: from: / topic: / when: 解析. R236 search_batch + R239 SortBy + SearchOptions + N17/TP2 register (ToolRegistry 装配统一注册件). src 模块 3 个 (lib + organ_kani_proofs + register). 测试数 (#[test]): 31 in-src + 0 集成.

apeireth-tool-search 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。
