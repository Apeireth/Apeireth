# R181 GitHub 优秀项目调研 — tool-codesearch 模块

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R181
> **日期**: 2026-08-13
> **范围**: apeireth-tool-codesearch 当前实现 + Rust 原生 / 多语言代码搜索 SOTA
> **状态**: 调研为升级预备.

---

## 0. 现状

apeireth-tool-codesearch:
- Tier 1.4 (R140) 已真接
- 当前功能: ripgrep 调用 + 简单 glob 过滤
- 缺失: AST 级别搜索 / 跨语言结构化匹配 / 代码改写 / 语义理解

---

## 1. Rust 原生代码搜索 SOTA

### 1.1 ast-grep (ast-grep/ast-grep) — **RECOMMENDED 强**

- **GitHub**: https://github.com/ast-grep/ast-grep
- **Stars**: 31.5K+ (2026-08)
- **License**: MIT
- **主语言**: Rust
- **定位**: \"A code search tool that understands your code structure\"
- **核心能力**:
  - 30+ 语言 (Rust / Python / TS / Go / Java / C++ / ... 全覆盖)
  - AST-based 模式匹配 (YAML 规则)
  - 重写/重构 (rewrite)
  - LSP server (ast-grep lsp)
  - 项目级扫描 (scan)
  - 增量更新
  - 性能: 比 ripgrep 慢 5-10x, 但仍是毫秒级

**为什么强推荐**:
- 行业新标杆, 我们 tool-codesearch 升级到它就是 SOTA
- 纯 Rust 集成无 FFI 风险
- License 友好 (MIT)
- 团队活跃 (持续每周 release)
- 30+ 语言直接覆盖我们的多语言需求

**集成方案**:
`
ust
// apeireth-tool-codesearch/src/ast_grep.rs
use ast_grep::{Pattern, Language};

pub struct AstGrepSearcher {
    lang: ast_grep::Lang,
}

impl CodeSearcher for AstGrepSearcher {
    async fn search(&self, root: &Path, pattern: &str) 
        -> Result<Vec<Match>, SearchError> {
        let pattern = Pattern::try_new(pattern, &self.lang)?;
        // call ast-grep as lib (in-process) or subprocess
        // for now, subprocess via CLI is more stable
        // ...
    }
}
`

**实际策略**:
- 短期: CLI subprocess 包装 (st-grep run --pattern ...)
- 长期: in-process library (ast-grep 已有 st-grep lib crate)
- **优先短期** — 稳定且 0 编译产物增加

### 1.2 ripgrep (BurntSushi/ripgrep) — **当前在用, 强化**

- **Stars**: 54K+
- **License**: MIT/Unlicense
- **定位**: 文本搜索最快
- **当前状态**: 我们 tool-codesearch 用它
- **强化方向**:
  - 启用 PCRE2 (.gitignore / type filtering)
  - 多线程调优 (--threads)
  - 自定义类型 (--type-add)

### 1.3 ugrep (Genivia/ugrep) — 备选

- **License**: BSD-3
- **优势**: 兼容 grep / egrep / fgrep 全部选项
- **不选**: ripgrep 已经够用, ugrep 优势不明显

### 1.4 Comby (comby-tools/comby) — **学习 (代码改写)**

- **License**: Apache 2.0
- **定位**: 结构化代码搜索 + 改写
- **能力**: 模板匹配 ([old]([args]) -> [new]([args]))
- **价值**: 跨语言 AST 改写
- **不集成**: 不如 ast-grep 活跃, Python 实现

---

## 2. 解析器底层 (tree-sitter 生态)

### 2.1 tree-sitter (tree-sitter/tree-sitter) — **RECOMMENDED 学习**

- **GitHub**: https://github.com/tree-sitter/tree-sitter
- **Stars**: 20K+
- **License**: MIT
- **定位**: 解析器生成器 + 增量解析
- **能力**:
  - 60+ 语言 grammar
  - 增量解析 (编辑器级响应)
  - C / Rust 绑定
  - 错误恢复 (代码不完整也能解析)
- **价值**: 我们如果做 in-process 集成 ast-grep, 底层就是 tree-sitter

### 2.2 rust-lang/rust-analyzer (基于 tree-sitter) — 学习

- 1.6K+ stars
- 不直接用, 但其 query engine 设计值得学习

---

## 3. Python 生态 SOTA (学习)

### 3.1 semgrep (semgrep/semgrep) — **学习**

- **Stars**: 30K+
- **License**: LGPL-2.1
- **定位**: 静态分析 + 安全规则引擎
- **学习点**: 规则格式 (pattern-either / pattern-inside / metavariable)
- **价值**: ast-grep 借鉴了它的规则设计
- **不集成**: LGPL 传染, Python

### 3.2 TheSilverSearcher (ggreer/the_silver_searcher) — 历史

- ac / the_silver_searcher, ripgrep 前身
- **不选**: ripgrep 完全超越

---

## 4. Rust 静态分析 SOTA (同生态)

### 4.1 clippy (rust-lang/rust-clippy) — 学习

- rustc 官方 linter
- **学习点**: lint 规则架构

### 4.2 dylint (trailofbits/dyllint) — 学习

- 动态加载的 lint 库
- **学习点**: Rust 动态 lint 机制

### 4.3 cargo-geiger (rust-secure-code/cargo-geiger) — 学习

- unsafe 代码统计
- 我们 Sovereignty 七重守门可以借鉴 unsafe 检测

---

## 5. 代码理解 / 嵌入 (RAG 视角)

### 5.1 aichat (sigoden/aichat) — 学习

- 集成 ripgrep / fzf / bat / eza 的 RAG 上下文工具
- 我们 council + RAG 可以借鉴

### 5.2 aider (Aider-AI/aider) — 学习

- AI pair programming
- 整库上下文管理
- **学习点**: repo map (tree-sitter 提取代码骨架) — 值得我们 tool-codesearch 借鉴

---

## 6. 升级方案 (最终阶段执行)

### 6.1 短期 (1 day)

1. **CLI 包装 ast-grep**: peireth-tool-codesearch search-ast 子命令
2. **ripgrep 类型配置**: 启用 .apeireth.toml 自定义类型
3. **JSON 输出统一**: 标准化 ripgrep / ast-grep 输出格式

### 6.2 中期 (2-3 days)

4. **in-process ast-grep lib**: 走 st-grep crate API
5. **YAML 规则加载**: 用户可定义自己的 AST 规则
6. **tree-sitter 暴露**: 我们的 code intelligence 基础 (query / cursor)

### 6.3 长期 (持续)

7. **嵌入 RAG 上下文**: 代码片段 embedding 进 memory, 让 LLM 直接召回
8. **跨语言重构**: ast-grep rewrite 包装, 批量改写
9. **代码 skeleton**: 借鉴 aider repo map, 提取关键结构

---

## 7. 依赖增量

| crate | 体积 | License | 必需 |
|---|---|---|---|
| ast-grep (CLI) | 0 (subprocess) | MIT | 短期 |
| ast-grep (lib) | ~3MB 编译产物 | MIT | 中期 |
| tree-sitter (lib) | ~10MB 编译产物 | MIT | 中期 |
| ripgrep (已有) | - | MIT/Unlicense | 是 |

**总增加**: 0 (短期, CLI 包装), ~13MB (中期, 全 lib 集成)

---

## 8. 与现有模块的关系

| 模块 | 关系 |
|---|---|
| tool-browser (R179) | 独立 |
| tool-fetch (R174) | 独立 (HTTP vs code) |
| tool-shell (R140) | 互补 (exec code vs search code) |
| memory (R146) | 代码片段可嵌入 memory 做 RAG |
| council | advisor 可调用 codesearch 做研究 |
| pipeline | codesearch 可作为 pipeline step |

---

## 9. 0 触碰声明

- 3 不可变脊柱: 0 触碰
- workspace.version 1.2.0: 0 改
- tool-codesearch 公开 API: 0 改 (新能力在子命令 / trait impl 内)

---

## 10. 参考链接

- ast-grep: https://github.com/ast-grep/ast-grep
- ripgrep: https://github.com/BurntSushi/ripgrep
- ugrep: https://github.com/Genivia/ugrep
- Comby: https://github.com/comby-tools/comby
- tree-sitter: https://github.com/tree-sitter/tree-sitter
- rust-analyzer: https://github.com/rust-lang/rust-analyzer
- semgrep: https://github.com/semgrep/semgrep
- clippy: https://github.com/rust-lang/rust-clippy
- dylint: https://github.com/trailofbits/dyllint
- cargo-geiger: https://github.com/rust-secure-code/cargo-geiger
- aichat: https://github.com/sigoden/aichat
- aider: https://github.com/Aider-AI/aider