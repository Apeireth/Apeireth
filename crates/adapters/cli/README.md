# apeireth-cli

> Apeireth 规范命令行入口（`apeireth` 单二进制）—— 装配一次规范运行时，把回合交给 `Runtime::execute_outcome`。

`apeireth-cli` 是终端侧的薄适配层：只把命令行参数装配成一次规范回合。运行时、会话、治理与
工具执行都归规范运行时所有，本 crate 不复制业务语义；CLI 与 HTTP 网关共用同一套装配入口。

## 主入口

- `src/main.rs` — `apeireth` 二进制入口（`Cargo.toml` 里 `[[bin]] name = "apeireth"`）。
  `HELP_TEXT` 是帮助文本的单一真源，`print_help()` 与防漂移测试
  `help_covers_every_dispatched_command` 共用它（分发面有的命令，帮助面必须出现）。
- `src/lib.rs` — 库入口与公共 API：`build_canonical_runtime_from_env()`（从环境变量装配唯一
  那套规范运行时）、`execute_canonical_cli_turn()`（构造 `TurnRequest` 并调
  `Runtime::execute_outcome`）、`dispatch_canonical_chat()` / `dispatch_canonical_approval()`、
  `dispatch_gateway_serve_with()`；回合结果类型 `CanonicalCliTurn` = `Completed(TurnResponse)`
  或 `PendingApproval(PendingApprovalView)`（挂起审批保留 `ApprovalId`）。
- `src/env_switches.rs` — 装配期环境开关读取面（`onion_layer_enabled_from_env()` 等）。

## 测试

```
cargo test -p apeireth-cli --locked
```

集成测试在 `crates/adapters/cli/tests/`（35 个测试文件），`src/` 内另有单元测试。
其中 `tests/canonical_cli_architecture.rs` 是架构守卫：钉住「CLI 聊天路径必须进
`Runtime::execute_outcome`，不得回退成 agent loop」。测试数量以
[docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md)
为单一真源（此处不抄写）。

## 已知边界

- feature 面只有 `base`（`default = ["base"]`）。此前本 README 列的 `local-intel` / `gui` /
  `sandbox` / `channels` / `audit` 与三个 suite feature，在 `Cargo.toml` 里并不存在。
- 洋葱层默认关：`APEIRETH_ENABLE_ONION_LAYER=1` 才在治理管线末层追加双洋葱判定
  （`src/env_switches.rs`）；侦察与外部适配器同样默认关（`APEIRETH_ENABLE_RECON` /
  `APEIRETH_ENABLE_EXTERNAL`）。
- 没有 LLM 工厂配置时，依赖 LLM 的子命令直接报错并给出可行动提示
  （`src/llm_factory_env.rs` 的 `LLM_FACTORY_MISSING_HINT`），不做「调用时才失败」。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../../docs/03-reference/crates.md)
