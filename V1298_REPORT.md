# V1298 — Cargo Workspace Lints Audit

- version: **0.1.0**
- workspace: `.openclaw\workspace\promethean\Apeireth-rust`
- crates scanned: **63**
- rust lints: **9**
- clippy lints: **38**
- total lints: **47**
- crates inherit [lints] workspace=true: **47/63** (74.60%)
- duration_ms: **6**

## 假说 (主 13:08 真自问, Popper 可证伪)

- ? PASS **h_rust_lints_present**: [workspace.lints.rust] 段存在 + lint 定义 ≥ 5
    - observed=9.0, threshold=5.0
    - details: section present=True, count=9
- ? PASS **h_clippy_lints_present**: [workspace.lints.clippy] 段存在 + lint 定义 ≥ 10
    - observed=38.0, threshold=10.0
    - details: section present=True, count=38
- ? PASS **h_rust_vs_clippy_separation**: [workspace.lints.rust] 段不含 clippy lint (e.g. unused_async)
    - observed=0.0, threshold=0.0
    - details: 误放 rust 段的 clippy lint: None
- ? PASS **h_unexpected_cfgs_present**: [workspace.lints.rust.unexpected_cfgs] 子段 + check-cfg ≥ 1
    - observed=2.0, threshold=1.0
    - details: section present=True, level=warn, check_cfg=['cfg(kani)', 'cfg(fuzzing)']
- ? FAIL **h_lints_inherit_pct**: workspace members [lints] workspace=true 占比 ≥ 95.0%
    - observed=74.60317460317461, threshold=95.0
    - details: 47/63 子 crate 继承 workspace.lints; missing: ['apeireth-keyring', 'apeireth-lark', 'apeireth-machine-id', 'apeireth-repo-analyzer', 'apeireth-repo-scan']
- ? PASS **h_no_deny_in_workspace_lints**: workspace.lints 不含 deny='all' / '*' / 'warnings' 全局拒绝
    - observed=0.0, threshold=0.0
    - details: 禁用全局 deny 命中: None

## unexpected_cfgs (kani/fuzzing 等防护)

- level: **warn**
- check_cfg: `['cfg(kani)', 'cfg(fuzzing)']`

## 子 crate 缺 [lints] workspace = true

- `apeireth-keyring`
- `apeireth-lark`
- `apeireth-machine-id`
- `apeireth-repo-analyzer`
- `apeireth-repo-scan`
- `apeireth-voice`
- `apeireth-team-lead`
- `example_plugin`
- `apeireth-plugin`
- `apeireth-image-prompt`
- `apeireth-template`
- `apeireth-schema`
- `apeireth-evolve`
- `apeireth-mcp-server`
- `apeireth-mcp-client`
- `apeireth-tree-sitter`

