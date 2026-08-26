# apeireth-wiki

> TP28 Markdown 知识库 (llm_wiki 模式, 文件树 + 索引 + 检索)

apeireth-wiki 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (1 src 文件 / 24 测试)

- `src/lib.rs` — TP28 Markdown 知识库 facade (WikiError/WikiResult/WikiEntry/WikiIndex/WikiStore trait/FilesystemWiki impl/WikiBlock + WikiContextBlock bridge trait) + 24 测试
