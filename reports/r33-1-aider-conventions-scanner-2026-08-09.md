# R33-1: Aider 借鉴 — 项目 conventions scanner

**日期**: 2026-08-09
**作者**: Mavis
**状态**: ✅ 完成
**ROI**: ★★★★ (借鉴 Aider "auto-inject project conventions to LLM system prompt", 0 网络依赖, 0 LLM 成本)

---

## 1. 目标

LLM 默认不知道"这个项目用什么 Rust edition / 哪些 lint / 关键依赖栈", 生成的代码风格往往跟项目脱节 (用错 edition / 钉错 dep 版本 / 漏 workspace lint).

借鉴 **Aider** (`aider` CLI 1.0+):
- `aider/repo.py:Repo.__init__` 启动时扫 `git ls-files` + 读 `README` + 抽 metadata
- `aider/history.py:Chat` 把这些 conventions 注入 system prompt
- `aider/args.py:--edit-format` 自动适配项目风格

我们落地为 `apeireth-tools::conventions_scanner`, 启动时 / 每次 chat 前扫 `<workspace_root>/Cargo.toml`,
抽 edition / rust-version / resolver / lints / deps, 输出 Aider 风格 system block, 注入 LLM.

---

## 2. 设计

### 2.1 `ProjectConventions` (结构体)

```rust
pub struct ProjectConventions {
    pub workspace_root: String,
    pub edition: Option<String>,
    pub rust_version: Option<String>,
    pub resolver: Option<String>,
    pub members_count: usize,
    pub workspace_deps_count: usize,
    pub lint_categories: Vec<String>,  // ["rust", "clippy", ...]
    pub key_deps: Vec<String>,         // 前 8 个 key, 按字母序
    pub scan_error: Option<String>,    // 任何解析失败都记, 不抛错 (优雅降级)
}
```

### 2.2 `scan(workspace_root)` (主入口)

- 读 `<root>/Cargo.toml` (std::fs)
- 用 `toml = "0.8"` crate 真解析 (跟 apeireth-api 一致, 0 新 dep 类型)
- 抽 `[workspace]` / `[workspace.package]` / `[workspace.dependencies]` / `[workspace.lints.*]`
- 任何解析失败 → 写 `scan_error` 字段, 仍返部分结果 (block 仍可用)

### 2.3 `to_system_prompt_block()` (Aider 风格 block)

```markdown
# 项目约定 (auto-scanned from Cargo.toml, Aider-style)

- Rust edition: 2021
- Rust version: 1.75
- Cargo resolver: 2
- Workspace members: 91 个 crate
- Workspace deps: 12 个 (key: anyhow, async-trait, chrono, criterion, ...)
- Lint 类别: rust, clippy

# 风格提示 (Aider-style hint)
- 写代码时遵循上面抽到的 edition / rust-version / lints
- 复用 workspace deps 用 `{ workspace = true }`, 不要钉版本
- 子 crate Cargo.toml 末尾加 `[lints]\nworkspace = true` 继承 workspace lint
- 保持现状 (不漂移): workspace version = 1.0.0 已是 1.0 release 锁版, 勿改
```

### 2.4 不漂移 (主哲学锚 #1)

- 0 改 Cargo.toml, 只读
- 0 写 system prompt (LLM 拿 block 后自己消化)
- 0 业务耦合 (`apeireth-tools` 不依赖 `apeireth-tui`, 任意消费者能调)
- 用 `toml` crate 真解析 (跟 R20 阶段 1 apeireth-api 一致)

---

## 3. 改动

### 3.1 新增 `crates/apeireth-tools/src/conventions_scanner.rs` (368 LOC)

- 公开 API: `ProjectConventions::scan` + `to_system_prompt_block` + `summary`
- 9 unit test (conventions_scanner_tests mod, 涵盖 4 类输入: full / missing / empty / malformed)

### 3.2 `crates/apeireth-tools/src/lib.rs`

- 加 `pub mod conventions_scanner;` + `pub use ProjectConventions`

### 3.3 `crates/apeireth-tools/Cargo.toml`

- 加 `toml = "0.8"` (跟 apeireth-api 一致, 0 新增 dep 类型)
- `tempfile = "3"` 已是 dev-dep (R30 加过), 复用

---

## 4. 测试

### 4.1 9 个新 unit test 全过 (apeireth-tools)

```
test conventions_scanner::conventions_scanner_tests::scan_workspace_root_with_full_cargo_toml ... ok
test conventions_scanner::conventions_scanner_tests::scan_missing_cargo_toml_records_error ... ok
test conventions_scanner::conventions_scanner_tests::scan_empty_cargo_toml_uses_defaults ... ok
test conventions_scanner::conventions_scanner_tests::scan_malformed_cargo_toml_records_error ... ok
test conventions_scanner::conventions_scanner_tests::scan_real_workspace_root_extracts_conventions ... ok
test conventions_scanner::conventions_scanner_tests::to_system_prompt_block_contains_key_sections ... ok
test conventions_scanner::conventions_scanner_tests::summary_one_liner_format ... ok
test conventions_scanner::conventions_scanner_tests::key_deps_truncated_to_8 ... ok
test conventions_scanner::conventions_scanner_tests::scan_error_display_includes_path ... ok

test result: ok. 9 passed; 0 failed
```

### 4.2 回归 (apeireth-tools 全 workspace)

- apeireth-tools 122/122 unit test pass (R30 113 + R33-1 9 = 122)
- 0 fail, 0 退化

---

## 5. 后续集成口子

- **TUI 集成 (R33-1.1, 1d)**: TUI backend `call_llm_stream_sync` 之前注入固定 system prompt, 加 conventions_scanner 调用拼 block. 真接 LLM 时 LLM 自动知道项目风格.
- **Council 集成 (R33-1.2, 0.5d)**: `apeireth-council` 多 LLM 协商, 每个成员拿同一份 conventions block, 输出更贴项目.
- **Eval harness 集成 (R32-3, 2d)**: eval task 跑前先扫 conventions, 验证 LLM 输出的 diff 符合项目风格 (e.g. 不应出现 edition = "2015" 写错).

---

## 6. 借鉴 vs 抄

- Aider 是 Python, 我们落地为 Rust
- Aider `repo.py` 抽 git metadata (commits, remotes, branch), 我们只抽 Cargo.toml (Rust 项目专属, 0 网络)
- Aider 注入到 `Chat.history`, 我们注入到 system prompt block (用法同)
- 借鉴是抽象层借鉴 (auto-inject conventions), 不是字面抄

---

## 7. 后续路线

- ✅ R33-1 完成
- ⏭ R32-3 (eval smoke test, 2d) — 借 R32-2 `run_tool_loop` + R33-1 conventions block
- ⏭ R36 (91→40 瘦身, 5d) — 5 老 provider crate 真删
- ⏭ R37-1 (ProtocolRouter 砍 1 层, 1d)
- ⏭ R37-2 (9 organ 部分合并, 3-5d)
- ⏭ R33-3 (MCP resources, 2d)
- ⏭ R33-4 (AutoGen council, 2d)

---

**Total LOC**: 1 new file (368) + 2 modify (lib.rs re-export + Cargo.toml 加 1 dep) + 9 new test.
**依赖**: `toml = "0.8"` (已存在, 0 新增).
**build/test**: 全 workspace pass.
