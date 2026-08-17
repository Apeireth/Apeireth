# R193 design — ast-grep CLI 包装进 tool-codesearch

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R193
> **日期**: 2026-08-13
> **来源**: R181 调研推荐短期路径
> **状态**: 设计稿 → 实施中

---

## 0. 目标

apeireth-tool-codesearch 当前 5 维 code intelligence (search / files / symbols / graph / index), 加第 6 维: **AST 级别结构化搜索** (R181 调研的 ast-grep 短期方案).

---

## 1. 设计

### 1.1 子模块

`
crates/apeireth-tool-codesearch/src/
  ast_grep.rs    (新, ~150 行)
  ast_grep_test.rs (新, ~100 行, 集成测试)
`

### 1.2 公共 API

`
ust
// ast_grep.rs

use std::path::Path;
use std::process::Command;

pub struct AstGrepSearcher {
    binary: PathBuf,            // ast-grep 路径, 默认 'ast-grep'
    lang: Option<String>,       // rust / python / ts / go / ...
}

pub struct AstGrepMatch {
    pub file: PathBuf,
    pub range: (usize, usize),  // (start_line, end_line)
    pub text: String,
    pub rule_id: Option<String>, // YAML rule name, if any
}

pub enum AstGrepError {
    BinaryNotFound,
    SpawnFailed(String),
    NonZeroExit(String),
    JsonParse(String),
}

pub trait AstSearcher: Send + Sync {
    fn search(&self, root: &Path, pattern: &str) 
        -> Result<Vec<AstGrepMatch>, AstGrepError>;
    fn search_with_rule(&self, root: &Path, rule_file: &Path)
        -> Result<Vec<AstGrepMatch>, AstGrepError>;
}

impl AstSearcher for AstGrepSearcher { ... }
`

### 1.3 CLI 调用

`ash
# 单 pattern 搜索
ast-grep run --pattern 'fn ($)' --lang rust /path

# YAML rule 搜索
ast-grep scan --rule rule.yml /path

# JSON 输出
ast-grep run --pattern '...' --lang rust --json=stream /path
`

我们的 wrapper:
1. Command::new("ast-grep") spawn
2. args 构造
3. stdout pipe + parse JSON
4. 错误处理 (binary not found / non-zero / parse fail)

### 1.4 集成点

- **lib.rs**: 加 pub mod ast_ggrep; + pub use ast_grep::{AstGrepSearcher, AstGrepMatch, AstSearcher, AstGrepError};
- **mcp.rs**: 加 MCP tool codesearch_ast 暴露给 LLM
- **compat.rs**: 加 compat router 命令
- **0 改**: 现有 7 模块 + 现有 MCP tools 完整保留

---

## 2. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- tool-codesearch 公开 API: 0 改 (新能力在 ast_grep 子模块内, 通过新 trait)
- 现有 7 模块源码: 0 改 (仅 lib.rs 加 1 行 pub mod)

---

## 3. 测试计划

- **t01**: AstGrepSearcher::new() 默认 'ast-grep' binary
- **t02**: search() 找到已知 pattern
- **t03**: search() 找不到 pattern
- **t04**: binary 不存在 -> BinaryNotFound 错误
- **t05**: search_with_rule() YAML rule
- **t06**: MCP tool codesearch_ast 端到端
- **t07**: 集成测试 (在临时 git 仓库跑)

总计 7 个测试, 目标全过.

---

## 4. 风险

- **R1**: ast-grep 在 Windows 路径 — 用 which crate 检测
- **R2**: subprocess 性能 — 毫秒级, 可接受
- **R3**: ast-grep 版本差异 — 锁定 0.39+

---

## 5. 0 触碰风险

- 0 风险 (新增子模块, 0 改现有)

---

## 6. 实施步骤

1. 写 ast_grep.rs (主实现)
2. 写 lib.rs pub mod 导出
3. 写 mcp.rs 新 tool
4. 写 compat.rs 新命令
5. 写 7 测试
6. cargo check + cargo test
7. demo example
8. 写 docs/r193/r193-ast-grep-integration.md 设计稿
9. commit

---

## 7. 依赖

- which crate (~30KB) — 检测 ast-grep binary
- 0 新增核心 dep
- subprocess 用 std::process (已有)
