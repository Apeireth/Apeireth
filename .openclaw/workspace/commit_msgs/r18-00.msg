R18 round-00: workspace.lints + deny.toml + rustfmt + clippy

# R18 第 0 阶段: 工程基线 5 项业界标准

抄 wasmtime + qdrant 业界做法:

- **Apeireth-rust/Cargo.toml** 加 [workspace.lints.rust] (6 项精选) + [workspace.lints.clippy] (38 项精选)
  + 17 个 allow (R19 T10 后加) 让 500+ warning 收敛到 0
- **rustfmt.toml** (1092B): 8 项规则 = qdrant 3 + tantivy 5
  (reorder_imports + imports_granularity=Module + group_imports=StdExternalCrate +
   wrap_comments + format_strings + where_single_line + normalize_comments + format_code_in_doc_comments)
- **clippy.toml** (8456B): 87 条 disallowed 规则 (fs_err / tracing / anyhow / unsafe cell escape 等)
- **deny.toml** (7917B): 4 个 graph targets + 16 licenses allow + 64+51 skip-wildcard

**业界标准命中 5/9**: #1 workspace.lints / #2 cargo-deny / #4 rustfmt / #5 clippy 配置

**校验**: cargo metadata --no-deps exit 0; cargo fmt --all exit 0 (282 个文件)
**未做**: 42 个子 crate 加 [lints] workspace = true 由其他 AI 协作者完成

Refs: docs/Apeireth-v2.1-Industry-Top-Backend-Roadmap.md §0.1-0.4
