# apeireth-cli

> Apeireth CLI (CliRunner, 暴露 Rust 子系统给终端) — R14 Phase 0 接口规范对照

apeireth-cli 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (6 src 文件 / 48 测试 + 2 Kani proof)

- `src/main.rs` — 二进制入口 (clap derive macros, R125-2 重构 -55% argv 解析)
- `src/lib.rs` — CliRunner 库入口 + 19 测试
- `src/commands.rs` — clap subcommand 定义 (skills/eval/council/mcp 等)
- `src/commands_tests.rs` — R116 commands submodule 集成测试 + 19 测试
- `src/output_format.rs` — 输出格式化 + 5 测试
- `src/organ_kani_proofs.rs` — R177 cli organ Kani proofs (5 测试 + 2 `#[kani::proof]`)

## Features (B2 workspace 装配层)

- `base` (default) — 基地本体
- `local-intel` / `gui` / `sandbox` / `channels` / `audit` — capability packs
- `suite-education` / `suite-pentest` / `suite-oracle` — upgrade suites
