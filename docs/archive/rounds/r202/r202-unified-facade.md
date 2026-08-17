# R202 unified code intelligence facade (6 维整合)

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R202
> **日期**: 2026-08-13
> **来源**: R181/R193 ast-grep 集成 + 6 维 code intelligence 整合
> **状态**: 实施完成, 10/10 单测全过 (累计 66/66)

---

## 0. 背景

apeireth-tool-codesearch 现有 6 维 code intelligence (来源 R177-R201 系列调研):
1. Content search (CodeSearcher) — 文本/正则/Aho-Corasick
2. File finder (FileFinder) — walkdir + glob
3. Symbol extraction (extract_symbols) — 5 语言 regex
4. Knowledge graph (KnowledgeGraph) — file → symbol → imports
5. Persistent index (CodeIndex) — rusqlite FTS5 (待补)
6. AST search (AstGrepSearcher, R193) — CLI subprocess

之前 6 个独立 API, 上层 (council / pipeline / API) 调用复杂. R202 整合为统一 facade.

---

## 1. 设计

### 1.1 公共类型

`
ust
pub enum IntelligenceKind { Text, File, Symbol, Graph, Index, Ast }  // 6 维

pub struct UnifiedQuery {
    pub kind: IntelligenceKind,
    pub pattern: String,
    pub path: PathBuf,
    pub lang: Option<String>,
}

pub enum IntelligenceHit {
    Text { file, line, column, text },
    File(FileEntry),
    Symbol(Symbol),
    Graph(GraphNode),
    Index(IndexEntry),
    Ast(AstGrepMatch),
}

pub struct UnifiedCodeIntelligence {
    searcher: CodeSearcher,
    finder: FileFinder,
    graph: Mutex<KnowledgeGraph>,
    index: Mutex<CodeIndex>,
    ast: AstGrepSearcher,
}
`

### 1.2 统一入口

`
ust
impl UnifiedCodeIntelligence {
    pub fn new_in_memory() -> Self;
    pub fn with_ast_binary(self, binary) -> Self;  // 链式
    pub fn query(&self, q: &UnifiedQuery) -> Result<Vec<IntelligenceHit>, UnifiedError>;
    pub fn index_file(&self, path: &str) -> Result<(), UnifiedError>;
}
`

### 1.3 错误

`
ust
pub enum UnifiedError {
    Io, AstGrep, Search, FileFinder, Index, Unsupported
}
`
手动 impl Display + From (避免 thiserror From 冲突)

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- 现有 6 个独立 API: 0 改 (facade 是 alternative, 不是 replacement)
- 现有 7 模块 + enhanced: 0 改 (仅 lib.rs 加 1 行 pub mod)
- tool-codesearch 公开 API: 0 改

---

## 3. 测试 (10/10 新增, 66/66 总)

- t01: kinds 6 维 hardcode
- t02: kind as_str
- t03: default = Text
- t04: UnifiedQuery::new
- t05: UnifiedQuery::with_lang
- t06: hit kind 映射
- t07: new_in_memory 构造
- t08: query text 无文件 graceful
- t09: query file graceful
- t10: query ast 缺 binary graceful

---

## 4. 不假装 (O-5)

- 不替换现有 6 个 API, 仅加 facade
- UnifiedError 手动 impl, 不假装 thiserror auto From
- 缺 ast-grep binary 不 panic, 优雅返回 Err

---

## 5. 风险

- 0 新依赖
- CodeIndex 没 Debug derive, 故 UnifiedCodeIntelligence 也不 derive Debug
- 0 触碰现有 API

---

## 6. 中期路径 (R202+1 候选)

- 集成进 MCP (UnifiedQuery -> unified_query MCP tool)
- 加 streaming query (大文件不阻塞)
- 加 cache (重复 query 不重扫)