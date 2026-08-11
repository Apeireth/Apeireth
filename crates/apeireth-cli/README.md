# apeireth-cli

> **Apeireth CLI** — 终端入口, 暴露 Rust 子系统 (skills / eval / council) 给 shell.
> **当前状态**: R116 + R125-2 clap 4.5 derive (3 subcommand 组, 15 命令).
> **二进制名**: `apeireth`.

---

## 启动

```bash
# 编译 + 装到 PATH
cargo install --path crates/apeireth-cli

# 跑命令
apeireth --version
apeireth skills list
apeireth skills show <id>
apeireth skills validate <file>
apeireth skills scenarios
apeireth skills watch <dir>

apeireth eval list-tools
apeireth eval scenarios
apeireth eval smoke <workspace>
apeireth eval markdown-snapshot <workspace>

apeireth council list-members
apeireth council add-member <role> <goal> <backstory> <provider>
apeireth council risk-hint
apeireth council markdown <query>
```

## 3 个 subcommand 组 (per commands.rs)

| Group | 命令数 | 用途 |
|---|---|---|
| `skills` | 5 | skill 描述符 + 验证 + 监控 (Watch mode) |
| `eval` | 4 | eval 场景 + smoke 跑 + markdown 快照 |
| `council` | 4 | 智囊团成员注册 + 风险提示 + 决议模板 |

## 依赖

- `apeireth-core` + `apeireth-memory` + `apeireth-asi` + `apeireth-api` + `apeireth-skills` + `apeireth-eval` + `apeireth-council` + `apeireth-mcp`
- `clap` 4.5 (derive)
- `tokio` + `serde` + `serde_json` + `anyhow` + `thiserror` + `chrono`

## 验证

```bash
cargo check -p apeireth-cli    # 0 errors
cargo build --release -p apeireth-cli
apeireth --version
```

## See also

- [R116 CLI command family spec](../../docs/conventions/)
- [R125-2 clap derive 借用 clap-rs/clap 4.6.6](../../reports/)