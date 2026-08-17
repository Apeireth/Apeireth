# apeireth-wiki

TP28 Markdown 知识库 (llm_wiki 模式) — 主动维护的 markdown 文件树 + 内存索引 + 检索。

## 定位

- 记忆 = 对话提炼 (时间序列事件流)
- 知识库 = 主动沉淀 (长期可检索文档) — 本 crate

## 用法

```rust
use apeireth_wiki::{FilesystemWiki, WikiStore, WikiContextBlock};

// 打开 wiki 根目录 (自动扫描建索引)
let wiki = FilesystemWiki::open("./wiki")?;

// curator 显式触发更新 (0 装: 不假装自动)
wiki.update("topics/ai-concepts.md", "---\ntitle: AI\ntags: [ai]\n---\n# body\n")?;

// 检索
let hits = wiki.search("neural", 10);
let by_tag = wiki.get_by_tag("ai");

// 与 context.rs 衔接 (TP21 渐进式披露)
let directory = wiki.directory_block(800);  // 目录级
let detail = wiki.expand_block("topics/ai-concepts.md"); // 详情级
```

## 边界

- 文件系统存储 (无 sqlite / 向量索引)
- 启动时从文件树重建索引 (rebuild_index 可重复触发)
- 路径必须 UTF-8 安全 + 不含 `..` / 绝对前缀
- 简化 markdown 解析: 仅 frontmatter (`--- ... ---`) + 标题 + 段落摘要 + `[text](path)` 链接兜底