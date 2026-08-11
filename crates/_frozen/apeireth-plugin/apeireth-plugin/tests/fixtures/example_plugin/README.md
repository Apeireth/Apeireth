# apeireth-plugin-example

Example plugin fixture for `apeireth-plugin` in-process test (Fixture 5, per
`v09021-rust-translation-blueprint-RIVAL §4.1`).

## 1:1 翻译 v0.9.21 商业版

- `plugin.json` 5 字段 (schema_version / name / version / author / entry /
  permissions / min_apeireth_version)
- 4 钩子 (init / on_session_start / on_tool_call / destroy)
- 5 lifecycle 状态 (Loaded / Initialized / Ready / Unloaded / Destroyed)
- 4 permission (file_read / file_write / network / mcp_call)

## 不在 fixture 范围内 (R20 阶段 4 估补)

- 真注册 tool call 到 `apeireth-tool-registry`
- m3 防御 (TOOL_WHITELIST) hardcode
- PluginMetadata 解析 (JSON → struct)
- Sandbox 执行 (`std::process::Command`)

## 跑法

```bash
cargo test --manifest-path crates/apeireth-plugin/Cargo.toml --test test_plugin_in_process
```
