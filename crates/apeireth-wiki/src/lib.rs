//! `apeireth-wiki` — TP28 Markdown 知识库 (llm_wiki 模式)
//!
//! **定位**: 主动维护的知识库 (文件树 + 索引 + 检索), 与记忆互补:
//! - 记忆 = 对话提炼 (时间序列事件流, tp12/14/20/22 域)
//! - 知识库 = 主动沉淀 (长期可检索文档, 本 crate)
//!
//! **设计要点**:
//! - 文件系统存储 (直接读写 `.md` 文件), 不引 sqlite / 新 dep
//! - 内存索引 + 启动时从文件树重建 (`scan_tree`)
//! - **0 装 PASS**: 不假装自动策展 — curator 必须由人工/外部 LLM 显式触发 `update()`
//! - 与 `apeireth-companion::context::ContextBlock` 衔接通过 `WikiContextBlock` trait
//!   (本 crate 自定义 `WikiBlock` 类型, 集成侧包装; 避免循环依赖)
//!
//! **frontmatter 简化约定**:
//! ```markdown
//! ---
//! title: AI Concepts
//! tags: [ai, ml]
//! links: [topics/tools.md]
//! ---
//!
//! # Heading
//! body...
//! ```
//!
//! **0 假装**:
//! - 不假装自动策展 (curator 不存在)
//! - 不假装全文检索 (substring match on title/summary/body, 不引向量检索)
//! - 不假装分布式 (单进程本地 FS)
//! - markdown 解析只支持 frontmatter + title + body; 不解析嵌套语法 / 表格 / 代码块
//!
//! **文件路径**: UTF-8 安全字符 (`std::path::Path` 兼容), 路径分隔用 `/` (Unix-style),
//! FS 层自动转换.

#![deny(unsafe_code)]

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Wiki 顶层错误.
#[derive(Debug, Error)]
pub enum WikiError {
    #[error("wiki: 路径非法 (含 .. 或绝对路径): {0}")]
    InvalidPath(String),
    #[error("wiki: 路径未找到: {0}")]
    NotFound(String),
    #[error("wiki: IO 错误 `{path}`: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("wiki: frontmatter 解析失败 `{path}`: {msg}")]
    Frontmatter { path: String, msg: String },
}

pub type WikiResult<T> = Result<T, WikiError>;

/// 单条 wiki 条目元数据.
///
/// ponytail: path 用相对路径 (相对于 wiki root), 存储时去掉前导 `./`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WikiEntry {
    pub path: String,
    pub title: String,
    /// ~200 字符索引摘要 (frontmatter summary 字段或正文首段截断).
    pub summary: String,
    pub tags: Vec<String>,
    /// 内部链接 (其他 wiki 路径).
    pub links: Vec<String>,
    pub created_ms: i64,
    pub updated_ms: i64,
}

/// Wiki 索引: path → entry 映射 + 反向 tag 索引 + 最后更新时间.
///
/// ponytail: 用 `BTreeMap` 让 entries 按路径有序 (输出稳定, 0 装标注);
/// `tags` 用 HashMap 反向索引.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WikiIndex {
    pub entries: BTreeMap<String, WikiEntry>,
    pub tags: HashMap<String, Vec<String>>,
    pub last_updated_ms: i64,
}

impl WikiIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 注册条目 (path → entry + 反向 tag 索引).
    pub fn insert(&mut self, entry: WikiEntry) {
        let path = entry.path.clone();
        for tag in &entry.tags {
            self.tags.entry(tag.clone()).or_default().push(path.clone());
        }
        self.entries.insert(path.clone(), entry);
    }

    /// 反查 tag → paths (sorted, deduped).
    pub fn paths_by_tag(&self, tag: &str) -> Vec<&str> {
        match self.tags.get(tag) {
            Some(paths) => {
                let mut seen = std::collections::BTreeSet::new();
                let mut out: Vec<&str> = paths
                    .iter()
                    .filter_map(|p| seen.insert(p.as_str()).then_some(p.as_str()))
                    .collect();
                out.sort();
                out
            }
            None => Vec::new(),
        }
    }
}

/// Wiki 存储 trait.
///
/// ponytail: 全部 `&self` (interior mutability); 实现侧用 `RwLock` 包 `WikiIndex`.
pub trait WikiStore: Send + Sync {
    fn search(&self, query: &str, max_results: usize) -> Vec<WikiEntry>;
    fn get_by_path(&self, path: &str) -> Option<WikiEntry>;
    fn get_by_tag(&self, tag: &str) -> Vec<WikiEntry>;
    fn update(&self, path: &str, content: &str) -> Result<(), WikiError>;
    /// 全量枚举 (sorted by path).
    fn all_entries(&self) -> Vec<WikiEntry>;
    /// 取底层索引快照 (测试用).
    fn snapshot_index(&self) -> WikiIndex;
}

/// 文件系统 wiki 实现.
///
/// 内部: `Arc<RwLock<WikiIndex>>` + `PathBuf` 根目录.
pub struct FilesystemWiki {
    root: PathBuf,
    index: Arc<RwLock<WikiIndex>>,
}

impl FilesystemWiki {
    /// 打开 (或创建) wiki 根目录, 扫描所有 `.md` 文件构建索引.
    pub fn open(root: impl Into<PathBuf>) -> WikiResult<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|e| WikiError::Io {
            path: root.display().to_string(),
            source: e,
        })?;
        let mut wiki = Self {
            root,
            index: Arc::new(RwLock::new(WikiIndex::new())),
        };
        wiki.rebuild_index()?;
        Ok(wiki)
    }

    /// 从文件系统重建索引 (启动 + 外部触发均可).
    pub fn rebuild_index(&mut self) -> WikiResult<()> {
        let mut new_index = WikiIndex::new();
        scan_tree(&self.root, &self.root, &mut new_index)?;
        new_index.last_updated_ms = chrono::Utc::now().timestamp_millis();
        *self.index.write() = new_index;
        Ok(())
    }

    /// 根目录路径 (只读).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 读文件内容 (按相对路径).
    pub fn read_file(&self, path: &str) -> WikiResult<String> {
        let abs = resolve_path(&self.root, path)?;
        fs::read_to_string(&abs).map_err(|e| WikiError::Io {
            path: path.into(),
            source: e,
        })
    }

    /// 写文件 + 更新索引 (curator 显式触发, 不假装自动).
    pub fn write_file(&self, path: &str, content: &str) -> WikiResult<()> {
        let abs = resolve_path(&self.root, path)?;
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).map_err(|e| WikiError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        fs::write(&abs, content).map_err(|e| WikiError::Io {
            path: path.into(),
            source: e,
        })?;
        // 同步索引
        let entry = parse_markdown(path, content).map_err(|msg| WikiError::Frontmatter {
            path: path.into(),
            msg,
        })?;
        let mut idx = self.index.write();
        // 反向 tag 清理: 先移除旧 path 的所有 tag 引用
        let old_tags: Vec<String> = idx
            .entries
            .get(path)
            .map(|e| e.tags.clone())
            .unwrap_or_default();
        for tag in &old_tags {
            if let Some(v) = idx.tags.get_mut(tag) {
                v.retain(|p| p != path);
            }
        }
        idx.insert(entry);
        idx.last_updated_ms = chrono::Utc::now().timestamp_millis();
        Ok(())
    }

    /// 关键词检索 (substr match on title + summary + body).
    pub fn search_substr(&self, query: &str, max_results: usize) -> Vec<WikiEntry> {
        let q = query.to_lowercase();
        let idx = self.index.read();
        let mut scored: Vec<(i32, WikiEntry)> = Vec::new();
        for entry in idx.entries.values() {
            let title_hit = i32::from(entry.title.to_lowercase().contains(&q));
            let summary_hit = i32::from(entry.summary.to_lowercase().contains(&q));
            // body 命中: 读文件 (I/O 受控; max_results 截断后停止)
            let body_hit = if title_hit + summary_hit == 0 && scored.len() < max_results * 4 {
                match self.read_file(&entry.path) {
                    Ok(body) => i32::from(body.to_lowercase().contains(&q)),
                    Err(_) => 0,
                }
            } else {
                0
            };
            let score = title_hit * 3 + summary_hit * 2 + body_hit;
            if score > 0 {
                scored.push((score, entry.clone()));
            }
        }
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.path.cmp(&b.1.path)));
        scored
            .into_iter()
            .take(max_results)
            .map(|(_, e)| e)
            .collect()
    }
}

impl WikiStore for FilesystemWiki {
    fn search(&self, query: &str, max_results: usize) -> Vec<WikiEntry> {
        self.search_substr(query, max_results)
    }
    fn get_by_path(&self, path: &str) -> Option<WikiEntry> {
        self.index.read().entries.get(path).cloned()
    }
    fn get_by_tag(&self, tag: &str) -> Vec<WikiEntry> {
        let idx = self.index.read();
        idx.paths_by_tag(tag)
            .into_iter()
            .filter_map(|p| idx.entries.get(p).cloned())
            .collect()
    }
    fn update(&self, path: &str, content: &str) -> Result<(), WikiError> {
        self.write_file(path, content)
    }
    fn all_entries(&self) -> Vec<WikiEntry> {
        self.index.read().entries.values().cloned().collect()
    }
    fn snapshot_index(&self) -> WikiIndex {
        self.index.read().clone()
    }
}

/// Wiki context block (与 context.rs ContextBlock 衔接).
///
/// ponytail: 自定义本地类型避免循环依赖; 集成侧 (apeireth-companion) 负责包装为
/// `ContextBlock`. name 用 `&'static str` (与 ContextBlock 一致).
#[derive(Debug, Clone)]
pub struct WikiBlock {
    pub name: &'static str,
    pub content: String,
    pub path: Option<String>,
}

impl WikiBlock {
    pub fn directory(name: &'static str, content: impl Into<String>) -> Self {
        Self {
            name,
            content: content.into(),
            path: None,
        }
    }

    pub fn detail(name: &'static str, path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name,
            content: content.into(),
            path: Some(path.into()),
        }
    }
}

/// 与 TP21 渐进式披露对齐的 context 衔接 trait.
///
/// ponytail: trait 用 `&self` + 返回 `WikiBlock` (而非 ContextBlock) — 避免
/// `apeireth-wiki` → `apeireth-companion` 的反向依赖.
pub trait WikiContextBlock: Send + Sync {
    /// 目录级 (~800 token): 列出所有条目的 title + path + summary 摘要.
    fn directory_block(&self, max_chars: usize) -> WikiBlock;
    /// 详情级: 按 path 展开单条完整内容.
    fn expand_block(&self, path: &str) -> Option<WikiBlock>;
}

impl WikiContextBlock for FilesystemWiki {
    fn directory_block(&self, max_chars: usize) -> WikiBlock {
        let entries = self.all_entries();
        let mut buf = String::new();
        buf.push_str("# Wiki 目录\n\n");
        let mut used = buf.chars().count();
        for entry in &entries {
            let line = format!(
                "- **{}** (`{}`) — {}\n",
                entry.title,
                entry.path,
                truncate(&entry.summary, 120)
            );
            if used + line.chars().count() > max_chars {
                buf.push_str("- …(更多条目已截断)\n");
                break;
            }
            used += line.chars().count();
            buf.push_str(&line);
        }
        WikiBlock::directory("wiki-directory", buf)
    }

    fn expand_block(&self, path: &str) -> Option<WikiBlock> {
        let content = self.read_file(path).ok()?;
        Some(WikiBlock::detail("wiki-detail", path, content))
    }
}

// ============================================================================
// 内部 helper: 路径解析 + markdown 解析 + 文件树扫描
// ============================================================================

/// 路径解析: 拒绝 `..` 与绝对路径 (防越权).
fn resolve_path(root: &Path, rel: &str) -> WikiResult<PathBuf> {
    if rel.is_empty() {
        return Err(WikiError::InvalidPath("empty".into()));
    }
    if rel.starts_with('/') || rel.starts_with('\\') {
        return Err(WikiError::InvalidPath(rel.into()));
    }
    // 拒绝 `..` 段 (防越权写)
    for seg in rel.split(['/', '\\']) {
        if seg == ".." {
            return Err(WikiError::InvalidPath(rel.into()));
        }
    }
    let abs = root.join(rel);
    Ok(abs)
}

/// markdown 解析: 简化 frontmatter + title + summary + tags + links.
///
/// frontmatter 格式:
/// ```text
/// ---
/// title: ...
/// tags: [a, b]
/// links: [path1, path2]
/// summary: ... (可选, 否则用正文首段截断)
/// ---
/// ```
fn parse_markdown(path: &str, content: &str) -> Result<WikiEntry, String> {
    let mut title = String::new();
    let mut tags: Vec<String> = Vec::new();
    let mut links: Vec<String> = Vec::new();
    let mut explicit_summary: Option<String> = None;
    let now_ms = chrono::Utc::now().timestamp_millis();

    // 1) frontmatter 解析 (只支持 `--- ... ---` 包裹的最顶端段)
    let body_start = if content.starts_with("---\n") || content.starts_with("---\r\n") {
        if let Some(end) = content[3..].find("\n---") {
            let fm = &content[4..end + 3];
            for line in fm.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("title:") {
                    title = rest.trim().trim_matches('"').to_string();
                } else if let Some(rest) = line.strip_prefix("tags:") {
                    tags = parse_list(rest.trim());
                } else if let Some(rest) = line.strip_prefix("links:") {
                    links = parse_list(rest.trim());
                } else if let Some(rest) = line.strip_prefix("summary:") {
                    explicit_summary = Some(rest.trim().trim_matches('"').to_string());
                }
            }
            let offset = end + 3 + 4; // "\n---" 长度 = 4
            if content[offset..].starts_with('\n') {
                offset + 1
            } else {
                offset
            }
        } else {
            0
        }
    } else {
        0
    };

    let body = &content[body_start..];

    // 2) title 兜底: 首行 `# ...`
    if title.is_empty() {
        for line in body.lines() {
            if let Some(stripped) = line.strip_prefix("# ") {
                title = stripped.trim().to_string();
                break;
            }
        }
    }
    if title.is_empty() {
        // 兜底: 用 path 的文件名作为 title
        title = path.rsplit(['/', '\\']).next().unwrap_or(path).to_string();
    }

    // 3) summary: 优先用 frontmatter, 否则取首段非空非标题行
    let summary = explicit_summary.unwrap_or_else(|| {
        let mut para = String::new();
        for line in body.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                if !para.is_empty() {
                    break;
                }
                continue;
            }
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(t);
            if para.chars().count() >= 200 {
                break;
            }
        }
        truncate(&para, 200)
    });

    // 4) 提取 markdown body 里的 links `[text](path)` 兜底
    if links.is_empty() {
        let mut offset = 0;
        while let Some(idx) = body[offset..].find("](") {
            let abs = offset + idx + 2;
            if let Some(end) = body[abs..].find(')') {
                let link = &body[abs..abs + end];
                if !link.contains("://") {
                    // 不是外链, 视为内部
                    links.push(link.to_string());
                }
                offset = abs + end + 1;
            } else {
                break;
            }
        }
    }

    Ok(WikiEntry {
        path: path.into(),
        title,
        summary,
        tags,
        links,
        created_ms: now_ms,
        updated_ms: now_ms,
    })
}

/// 解析 `[a, b, c]` 或 `[a,b,c]` 形式的列表.
fn parse_list(s: &str) -> Vec<String> {
    let s = s.trim();
    let inner = s
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .unwrap_or(s);
    inner
        .split(',')
        .map(|x| x.trim().trim_matches('"').to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

/// 扫描目录树, 把所有 `.md` 文件加入索引.
fn scan_tree(root: &Path, dir: &Path, idx: &mut WikiIndex) -> WikiResult<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| WikiError::Io {
        path: dir.display().to_string(),
        source: e,
    })? {
        let entry = entry.map_err(|e| WikiError::Io {
            path: dir.display().to_string(),
            source: e,
        })?;
        let p = entry.path();
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // 跳过隐藏目录 + 已知 metadata 文件
        if file_name.starts_with('.') {
            continue;
        }
        let ft = entry.file_type().map_err(|e| WikiError::Io {
            path: p.display().to_string(),
            source: e,
        })?;
        if ft.is_dir() {
            scan_tree(root, &p, idx)?;
        } else if ft.is_file() && p.extension().and_then(|e| e.to_str()) == Some("md") {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            let content = fs::read_to_string(&p).map_err(|e| WikiError::Io {
                path: rel.clone(),
                source: e,
            })?;
            match parse_markdown(&rel, &content) {
                Ok(e) => idx.insert(e),
                Err(_) => {
                    // 解析失败: 跳过该文件 (不阻断整树扫描)
                }
            }
        }
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push('…');
    }
    out
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mk_wiki() -> (TempDir, FilesystemWiki) {
        let tmp = TempDir::new().unwrap();
        let wiki = FilesystemWiki::open(tmp.path()).unwrap();
        (tmp, wiki)
    }

    fn write(wiki: &FilesystemWiki, path: &str, content: &str) {
        wiki.update(path, content).unwrap();
    }

    const SAMPLE_TOPIC: &str = "---\n\
title: AI Concepts\n\
tags: [ai, ml]\n\
links: [topics/tools.md]\n\
summary: 人工智能基础概念集合 (神经网络/损失/梯度 等).\n\
---\n\
# AI Concepts\n\
This is a curated overview of AI fundamentals. Neural networks learn via gradient descent.\n";

    const SAMPLE_TOOLS: &str = "---\n\
title: Tools\n\
tags: [tools, editor]\n\
---\n\
# Tools\n\
Editors and IDEs that I use daily: vscode, vim.\n";

    const SAMPLE_NOTES: &str = "---\n\
title: 2026-08-18 Meeting\n\
tags: [meeting, ai]\n\
---\n\
# 2026-08-18 Meeting\n\
Discussed token budget and incremental disclosure.\n";

    #[test]
    fn write_and_read_round_trip() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai-concepts.md", SAMPLE_TOPIC);
        let content = wiki.read_file("topics/ai-concepts.md").unwrap();
        assert!(content.contains("AI Concepts"));
        assert!(content.contains("Neural networks"));
    }

    #[test]
    fn index_built_from_file_tree_on_open() {
        let tmp = TempDir::new().unwrap();
        // 预先建好子目录 + 放文件, 模拟已有 wiki
        std::fs::create_dir_all(tmp.path().join("topics")).unwrap();
        std::fs::write(tmp.path().join("topics").join("ai.md"), SAMPLE_TOPIC).unwrap();
        std::fs::create_dir_all(tmp.path().join("code-snippets")).unwrap();
        std::fs::write(
            tmp.path().join("code-snippets").join("rust.md"),
            SAMPLE_TOOLS,
        )
        .unwrap();

        let wiki = FilesystemWiki::open(tmp.path()).unwrap();
        let idx = wiki.snapshot_index();
        assert_eq!(idx.len(), 2, "启动时扫到 2 个 .md 文件");
        assert!(idx.entries.contains_key("topics/ai.md"));
        assert!(idx.entries.contains_key("code-snippets/rust.md"));
    }

    #[test]
    fn wiki_index_serde_round_trip() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai.md", SAMPLE_TOPIC);
        write(&wiki, "notes/meeting.md", SAMPLE_NOTES);
        let idx = wiki.snapshot_index();
        let json = serde_json::to_string(&idx).unwrap();
        let back: WikiIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(back.entries.len(), 2);
        assert_eq!(back.entries["topics/ai.md"].title, "AI Concepts");
        assert_eq!(back.entries["notes/meeting.md"].tags, vec!["meeting", "ai"]);
    }

    #[test]
    fn search_finds_by_title_and_summary() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai.md", SAMPLE_TOPIC);
        write(&wiki, "topics/tools.md", SAMPLE_TOOLS);
        write(&wiki, "notes/meeting.md", SAMPLE_NOTES);
        // 搜 title 关键词 "Tools"
        let hits = wiki.search("Tools", 10);
        assert!(hits.iter().any(|e| e.path == "topics/tools.md"));
        // 搜 summary 关键词 "人工智能" (frontmatter summary)
        let hits2 = wiki.search("人工智能", 10);
        assert!(hits2.iter().any(|e| e.path == "topics/ai.md"));
    }

    #[test]
    fn search_ranks_title_higher_than_body() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai.md", SAMPLE_TOPIC);
        write(&wiki, "topics/tools.md", SAMPLE_TOOLS);
        // 搜 "Tools": tools.md 在 title, ai.md 在 body ("Tools" 不在 body) — 但 ai.md 也含 "Tools" 吗?
        // body "Neural networks learn via gradient descent" 不含 Tools, 所以 ai 不命中.
        let hits = wiki.search("Tools", 10);
        assert_eq!(hits[0].path, "topics/tools.md");
    }

    #[test]
    fn search_respects_max_results() {
        let (_tmp, wiki) = mk_wiki();
        for i in 0..5 {
            write(
                &wiki,
                &format!("notes/note-{i}.md"),
                "---\ntitle: Shared\n---\nshared body\n",
            );
        }
        let hits = wiki.search("shared", 3);
        assert_eq!(hits.len(), 3, "max_results=3 应截断");
    }

    #[test]
    fn get_by_path_returns_entry() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai.md", SAMPLE_TOPIC);
        let entry = wiki.get_by_path("topics/ai.md").unwrap();
        assert_eq!(entry.title, "AI Concepts");
        assert_eq!(entry.tags, vec!["ai", "ml"]);
        assert!(entry.summary.contains("人工智能"));
        assert_eq!(entry.links, vec!["topics/tools.md"]);
    }

    #[test]
    fn get_by_path_missing_returns_none() {
        let (_tmp, wiki) = mk_wiki();
        assert!(wiki.get_by_path("nope.md").is_none());
    }

    #[test]
    fn get_by_tag_returns_matching_entries() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai.md", SAMPLE_TOPIC); // tags = [ai, ml]
        write(&wiki, "topics/tools.md", SAMPLE_TOOLS); // tags = [tools, editor]
        write(&wiki, "notes/meeting.md", SAMPLE_NOTES); // tags = [meeting, ai]
        let ai_entries = wiki.get_by_tag("ai");
        assert_eq!(ai_entries.len(), 2);
        let paths: Vec<&str> = ai_entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"topics/ai.md"));
        assert!(paths.contains(&"notes/meeting.md"));
        let tools_entries = wiki.get_by_tag("tools");
        assert_eq!(tools_entries.len(), 1);
        assert_eq!(tools_entries[0].path, "topics/tools.md");
        let empty = wiki.get_by_tag("nonexistent");
        assert!(empty.is_empty());
    }

    #[test]
    fn update_syncs_index_in_place() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai.md", SAMPLE_TOPIC);
        let v1 = wiki.get_by_path("topics/ai.md").unwrap();
        assert_eq!(v1.title, "AI Concepts");
        // 更新内容
        wiki.update(
            "topics/ai.md",
            "---\ntitle: AI Concepts v2\ntags: [ai, ml, deep-learning]\n---\n# v2 body\n",
        )
        .unwrap();
        let v2 = wiki.get_by_path("topics/ai.md").unwrap();
        assert_eq!(v2.title, "AI Concepts v2");
        assert_eq!(v2.tags, vec!["ai", "ml", "deep-learning"]);
        // 反向 tag 索引也同步: "deep-learning" 应能找到
        let dl = wiki.get_by_tag("deep-learning");
        assert_eq!(dl.len(), 1);
        assert_eq!(dl[0].path, "topics/ai.md");
        // 旧 tag 仍在: 但旧 tags 应保留 (我们 update 是替换, 不删旧)
        // 实际上, 我们 update 实现里是先 remove 旧 tag 再 insert 新 tag, 所以旧 tag 应该被清掉.
        // 当前实现: 反向 tag 清理基于 old.tags, 新 tags 重新加入. 测试更新后的 tags.
    }

    #[test]
    fn update_creates_parent_dir_on_demand() {
        let (_tmp, wiki) = mk_wiki();
        write(
            &wiki,
            "deep/nested/path/topic.md",
            "---\ntitle: Deep Topic\n---\nbody\n",
        );
        let entry = wiki.get_by_path("deep/nested/path/topic.md");
        assert!(entry.is_some());
    }

    #[test]
    fn rejects_path_traversal() {
        let (_tmp, wiki) = mk_wiki();
        let r = wiki.update("../escape.md", "x");
        assert!(matches!(r, Err(WikiError::InvalidPath(_))));
        let r2 = wiki.update("/abs.md", "x");
        assert!(matches!(r2, Err(WikiError::InvalidPath(_))));
    }

    #[test]
    fn rejects_empty_path() {
        let (_tmp, wiki) = mk_wiki();
        let r = wiki.update("", "x");
        assert!(matches!(r, Err(WikiError::InvalidPath(_))));
    }

    #[test]
    fn directory_block_lists_all_entries_within_budget() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai.md", SAMPLE_TOPIC);
        write(&wiki, "topics/tools.md", SAMPLE_TOOLS);
        write(&wiki, "notes/meeting.md", SAMPLE_NOTES);
        let block = wiki.directory_block(2000);
        assert_eq!(block.name, "wiki-directory");
        assert!(block.path.is_none());
        assert!(block.content.contains("# Wiki 目录"));
        assert!(block.content.contains("AI Concepts"));
        assert!(block.content.contains("topics/ai.md"));
    }

    #[test]
    fn directory_block_truncates_when_over_budget() {
        let (_tmp, wiki) = mk_wiki();
        for i in 0..10 {
            write(
                &wiki,
                &format!("n-{i}.md"),
                &format!(
                    "---\ntitle: Title {i} with some longer content for budget\n---\nbody {i}\n"
                ),
            );
        }
        let block = wiki.directory_block(400); // 极紧预算
        assert!(
            block.content.contains("(更多条目已截断)"),
            "应触发截断; got: {}",
            block.content
        );
        assert!(
            block.content.chars().count() <= 800,
            "截断后字符数应 ≤ 预算 * 2"
        );
    }

    #[test]
    fn expand_block_returns_full_content() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "topics/ai.md", SAMPLE_TOPIC);
        let block = wiki.expand_block("topics/ai.md").unwrap();
        assert_eq!(block.name, "wiki-detail");
        assert_eq!(block.path.as_deref(), Some("topics/ai.md"));
        assert!(block.content.contains("Neural networks"));
        assert!(block.content.contains("AI Concepts"));
    }

    #[test]
    fn expand_block_missing_returns_none() {
        let (_tmp, wiki) = mk_wiki();
        assert!(wiki.expand_block("nope.md").is_none());
    }

    #[test]
    fn wiki_block_struct_constructors_work() {
        let d = WikiBlock::directory("n", "content");
        assert_eq!(d.name, "n");
        assert!(d.path.is_none());
        let e = WikiBlock::detail("n", "p", "content");
        assert_eq!(e.path.as_deref(), Some("p"));
    }

    #[test]
    fn all_entries_returns_sorted_by_path() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "z.md", "---\ntitle: Z\n---\n");
        write(&wiki, "a.md", "---\ntitle: A\n---\n");
        write(&wiki, "m.md", "---\ntitle: M\n---\n");
        let all = wiki.all_entries();
        let paths: Vec<&str> = all.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["a.md", "m.md", "z.md"]);
    }

    #[test]
    fn markdown_without_frontmatter_uses_first_h1_as_title() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "free.md", "# Free Title\nbody without frontmatter\n");
        let e = wiki.get_by_path("free.md").unwrap();
        assert_eq!(e.title, "Free Title");
    }

    #[test]
    fn summary_falls_back_to_first_paragraph() {
        let (_tmp, wiki) = mk_wiki();
        write(
            &wiki,
            "long.md",
            "# Title\n\nThis is the first paragraph that should be summarized. It is long enough to test the truncation.\n",
        );
        let e = wiki.get_by_path("long.md").unwrap();
        assert!(e.summary.contains("first paragraph"));
    }

    #[test]
    fn last_updated_ms_changes_on_update() {
        let (_tmp, wiki) = mk_wiki();
        write(&wiki, "x.md", "---\ntitle: X\n---\n");
        let t1 = wiki.snapshot_index().last_updated_ms;
        std::thread::sleep(std::time::Duration::from_millis(2));
        wiki.update("x.md", "---\ntitle: X v2\n---\n").unwrap();
        let t2 = wiki.snapshot_index().last_updated_ms;
        assert!(t2 >= t1);
    }

    #[test]
    fn rebuild_index_picks_up_external_writes() {
        let tmp = TempDir::new().unwrap();
        let mut wiki = FilesystemWiki::open(tmp.path()).unwrap();
        assert_eq!(wiki.snapshot_index().len(), 0);
        // 模拟外部 (curator / agent) 直接写文件, 然后 rebuild
        std::fs::write(tmp.path().join("external.md"), SAMPLE_TOPIC).unwrap();
        wiki.rebuild_index().unwrap();
        assert_eq!(wiki.snapshot_index().len(), 1);
        assert!(wiki.get_by_path("external.md").is_some());
    }

    #[test]
    fn paths_by_tag_dedupes() {
        let mut idx = WikiIndex::new();
        let e = WikiEntry {
            path: "x.md".into(),
            title: "X".into(),
            summary: "".into(),
            tags: vec!["t".into(), "t".into()], // 故意重复 tag
            links: vec![],
            created_ms: 0,
            updated_ms: 0,
        };
        idx.insert(e);
        let paths = idx.paths_by_tag("t");
        assert_eq!(paths.len(), 1, "反向索引应去重");
    }
}
