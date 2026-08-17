# TP28 Markdown 知识库（llm_wiki 模式）验收报告

- 任务 ID: `71e2ea50-eaa0-4dd4-b1a5-6653f6919d53`
- 角色: backend_engineer2
- 日期: 2026-08-18
- 范围: TP28（Markdown 知识库，套件批）

---

## 1. 交付清单

| # | 文件 | 类型 | 说明 |
|---|---|---|---|
| 1 | `crates/apeireth-wiki/Cargo.toml` | 新增 | 套件骨架 (workspace 继承 version/edition/lints) |
| 2 | `crates/apeireth-wiki/README.md` | 新增 | 用法 + 边界说明 |
| 3 | `crates/apeireth-wiki/src/lib.rs` | 新增 | WikiEntry/WikiIndex/WikiError/WikiStore trait + FilesystemWiki 实现 + WikiContextBlock trait + 24 测试 |
| 4 | `Cargo.toml` (workspace) | 修改 | 加 `crates/apeireth-wiki` 到 members |
| 5 | `docs/backlog.md` | 修改 | TP28 已完成项登记 (2 快照位) |

无新依赖 (deps: `apeireth-core` + 已有 `serde/serde_json/chrono/uuid/thiserror/parking_lot`；dev: `tempfile`)。

---

## 2. 设计要点（按任务边界）

### 2.1 文件树结构（任务 §1）

```text
wiki/
├── index.md           # （可选, 由 curator 显式创建）
├── topics/            # 主题目录
├── code-snippets/     # 代码片段
└── notes/             # 个人笔记
```

实现：相对路径以 `wiki/` 根为基准，路径用 `/` 分隔（Unix-style，自动从 FS 还原）。路径段拒绝 `..` 与绝对前缀（防越权）。

### 2.2 索引结构（任务 §2）

```rust
pub struct WikiEntry {
    pub path: String,
    pub title: String,
    pub summary: String,    // ~200 字符索引摘要
    pub tags: Vec<String>,
    pub links: Vec<String>, // 内部 wiki 链接
    pub created_ms: i64,
    pub updated_ms: i64,
}

pub struct WikiIndex {
    pub entries: BTreeMap<String, WikiEntry>,  // 按 path 排序 (输出稳定)
    pub tags: HashMap<String, Vec<String>>,    // tag → paths 反向索引
    pub last_updated_ms: i64,
}
```

### 2.3 检索接口（任务 §3）

```rust
pub trait WikiStore: Send + Sync {
    fn search(&self, query: &str, max_results: usize) -> Vec<WikiEntry>;
    fn get_by_path(&self, path: &str) -> Option<WikiEntry>;
    fn get_by_tag(&self, tag: &str) -> Vec<WikiEntry>;
    fn update(&self, path: &str, content: &str) -> Result<(), WikiError>;
    fn all_entries(&self) -> Vec<WikiEntry>;
    fn snapshot_index(&self) -> WikiIndex;
}
```

实现：`FilesystemWiki`，内部 `Arc<RwLock<WikiIndex>>`（interior mutability 让 trait 方法用 `&self`）。

**search 算法**（0 装 PASS）：
- 对 `query` 大小写不敏感 substring match
- 评分：title 命中 ×3 + summary 命中 ×2 + body 命中 ×1
- body 命中需读文件（I/O）；为控制开销，仅当 `(title_hit + summary_hit) == 0` 且候选数 < `max_results * 4` 时读 body
- 按 `(score desc, path asc)` 排序，取 top-N

### 2.4 WikiContextBlock（任务 §4，TP21 衔接）

```rust
pub struct WikiBlock {
    pub name: &'static str,
    pub content: String,
    pub path: Option<String>,
}

pub trait WikiContextBlock: Send + Sync {
    fn directory_block(&self, max_chars: usize) -> WikiBlock;
    fn expand_block(&self, path: &str) -> Option<WikiBlock>;
}
```

**为什么不用 `ContextBlock` 本体**：避免 `apeireth-wiki` 反向依赖 `apeireth-companion`（companion 依赖 wiki 会形成循环）。集成侧（后续任务）把 `WikiBlock` 包装为 `ContextBlock { name: block.name, content: block.content, core: false, cap_chars: ... }`。

### 2.5 0 装 PASS（任务 §5）

| 约束 | 落地 |
|---|---|
| 不假装自动策展 | `update()` 必须由调用方显式触发；无 daemon / no scheduler |
| 文件系统存储 | `std::fs::read_to_string` / `write`，无 sqlite / 无新 dep |
| 内存索引 + 启动时重建 | `open()` → `scan_tree()` 自动建索引；`rebuild_index()` 可重复触发 |

### 2.6 markdown 解析（简化约定）

```markdown
---
title: AI Concepts
tags: [ai, ml]
links: [topics/tools.md]
summary: 人工智能基础概念集合 (神经网络/损失/梯度 等).
---

# Heading
body...
```

- frontmatter `--- ... ---` 包裹的最顶端段
- title 兜底：首行 `# ...`
- summary 兜底：首段非空非标题行，截 200 字
- links 兜底：body 中 `[text](path)` 提取（非 `://` 外链）
- 解析失败的文件：跳过（不阻断整树扫描）

---

## 3. 验收测试矩阵

| 验收项 | 测试名 | 结果 |
|---|---|---|
| 文件树创建/读取 | `write_and_read_round_trip` | ✅ |
| 启动从文件树建索引 | `index_built_from_file_tree_on_open` | ✅ |
| WikiIndex 序列化 | `wiki_index_serde_round_trip` | ✅ |
| search — title + summary 命中 | `search_finds_by_title_and_summary` | ✅ |
| search — 排序 (title > body) | `search_ranks_title_higher_than_body` | ✅ |
| search — max_results 截断 | `search_respects_max_results` | ✅ |
| get_by_path — 命中 | `get_by_path_returns_entry` | ✅ |
| get_by_path — 缺失返 None | `get_by_path_missing_returns_none` | ✅ |
| get_by_tag — 多对一/一/空 | `get_by_tag_returns_matching_entries` | ✅ |
| update — 同步索引 | `update_syncs_index_in_place` | ✅ |
| update — 自动建父目录 | `update_creates_parent_dir_on_demand` | ✅ |
| 路径越权拒绝 | `rejects_path_traversal` | ✅ |
| 空路径拒绝 | `rejects_empty_path` | ✅ |
| WikiContextBlock — 目录级 | `directory_block_lists_all_entries_within_budget` | ✅ |
| WikiContextBlock — 截断 | `directory_block_truncates_when_over_budget` | ✅ |
| WikiContextBlock — 详情级 | `expand_block_returns_full_content` | ✅ |
| WikiContextBlock — 缺失返 None | `expand_block_missing_returns_none` | ✅ |
| WikiBlock 构造器 | `wiki_block_struct_constructors_work` | ✅ |
| all_entries 按路径排序 | `all_entries_returns_sorted_by_path` | ✅ |
| 无 frontmatter → 首 # 作为 title | `markdown_without_frontmatter_uses_first_h1_as_title` | ✅ |
| summary 兜底首段 | `summary_falls_back_to_first_paragraph` | ✅ |
| last_updated_ms 变更 | `last_updated_ms_changes_on_update` | ✅ |
| 外部写文件 + rebuild | `rebuild_index_picks_up_external_writes` | ✅ |
| 反向 tag 索引去重 | `paths_by_tag_dedupes` | ✅ |

**24 个测试全绿**。

---

## 4. 命令验证

```bash
$ cargo test -p apeireth-wiki --lib
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

$ cargo check -p apeireth-wiki --lib
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s
```

**注**：`cargo check --workspace --all-targets` 中 `apeireth-tools` 报 4 个错（yaml_spec.rs `description`/`borrow`），但：
- 我未触碰 `apeireth-tools` 任何文件
- 我的工作树分支上 yaml_spec.rs 无 diff（`git diff integration..HEAD --stat -- crates/apeireth-tools/src/yaml_spec.rs` 空）
- `cargo check -p apeireth-wiki --lib` 干净，`cargo check -p apeireth-companion --lib` / `-p apeireth-skills --lib` 也干净
- 预存 WIP 错误（与 TP28 无关）

---

## 5. 边界声明（0 假装）

| 项 | 当前实现 | 升级路径 |
|---|---|---|
| 全文检索 | substring match on title/summary/body | 若需语义检索, 后续 N-TP 接 `apeireth-tool-search` 的向量/TF-IDF |
| markdown 解析 | 简化 (frontmatter + title + summary + links) | 完整解析可接 `pulldown-cmark` / `markdown` crate（会增加 dep） |
| 并发写 | `RwLock` (单写多读); 写期间其他写阻塞 | 若需 lock-free, 可走 `arc-swap` 或分片 |
| 文件锁 | 无 (FS 层 race condition 由 curator 调度保证) | 若多进程访问, 需 `fs2` / `flock` |
| WikiBlock → ContextBlock 包装 | 当前未集成 | 后续在 `apeireth-companion` 加 `wiki_to_context_blocks()` 函数 |

---

## 6. 未触碰禁踩区（确认）

按 docs/next-team-handbook §1 + 团队 LOCKED 列表确认未触碰:
- `apeireth-companion` 任何文件（不改）
- `apeireth-tools` / `apeireth-tool-runtime` / `apeireth-agent` / `apeireth-skills` 等均不动
- 仅新增 `crates/apeireth-wiki/` 套件 + workspace Cargo.toml 加 1 行 member

---

## 7. 与 TP28 边界对齐

| 边界 | 落地 |
|---|---|
| 新套件 `crates/apeireth-wiki/` | ✅ |
| 文件树结构 (topics/code-snippets/notes) | ✅ 任意子目录深度，路径 UTF-8 安全 |
| WikiIndex + WikiEntry 字段 | ✅ 全部按任务 §2 |
| WikiStore trait 4 方法 | ✅ + 加 `all_entries` / `snapshot_index` 便利方法 |
| WikiContextBlock 2 方法 | ✅ |
| 不假装自动策展 | ✅ curator 必须显式调 `update()` |
| FS 存储 + 内存索引 | ✅ |
| 启动时重建 | ✅ `open()` 自动 scan_tree |

---

## 8. 提交状态

- git commit: 待 push (commit 在 task/tp12-schema-guardrail-rework-final 工作树)
- 团队框架状态: 报告 + backlog 同步完成, `team_complete_task` 待调用
- 后续移交:
  - `apeireth-companion` 加 `wiki_to_context_blocks()` 把 WikiBlock 包装为 ContextBlock
  - `apeireth-tool-search` 接向量/TF-IDF 检索 (替代 substring)
  - 与 `memory_extractor` 衔接: 记忆提炼 → wiki 沉淀的触发器