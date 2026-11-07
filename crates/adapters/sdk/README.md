# apeireth-sdk

> Apeireth 客户侧 SDK —— 纯 Rust 实现，提供跨语言一致的 wire 面、版本协商、错误码与工具调用面。

`apeireth-sdk` 面向外部应用（Rust / Python / Node / Go 客户端）：把请求编码成规范 envelope、
把服务端错误映射成稳定错误码，并给出 6 项工具白名单的调用面。纯 Rust 实现，无 Python 虚拟环境。

## 主入口

- `src/lib.rs` — SDK 入口与顶层 re-export：`Envelope` / `WireKind`（`src/wire.rs`，wire 面
  `v` / `kind` / `id` / `body` 四字段）、`SdkVersion` / `WireCompat` / `negotiate()` /
  `SDK_VERSION`（`src/version.rs`，当前 `0.1.0`）、`SdkError` / `SdkErrorCode`（`src/error.rs`）、
  `ApeirethClient` / `ClientConfig` / `AuthPipeline`（`src/client.rs`）与 `TOOL_WHITELIST`（6 项）/
  `SDK_TOOL_WHITELIST`（8 项）/ `TOOL_PATHS` / `WS_PATH` / `STUB_MODE` 等常量。
- `src/client.rs` — 客户端主实现：`ApeirethClient::invoke_tool()` 走真实 HTTP；`AuthPipeline`
  集齐鉴权五组件（Bearer / keyring / `TokenBucket` / `AuditLogger` / `QuotaStub` / `KeyringRef`）。
- `src/error_taxonomy.rs` / `src/frame_codec.rs` / `src/redact.rs` — 协议族共用基础设施：
  闭合错误词表、长度前缀帧编解码、日志脱敏；`src/im/` — IM 快捷接入 wire 面（渠道 kind、
  入站信封、审批卡片、分段、传输、秘密脱敏），原语在模块私有的 `im::wire` 内维护。
- `src/abi.rs` — `extern "C"` 顶层入口（`apeireth_sdk_init` / `apeireth_sdk_last_error`）；
  `src/c.rs` — 5 个 `extern "C"` 函数；跨语言桥 feature 见「已知边界」。

## 测试

```
cargo test -p apeireth-sdk --locked
```

集成测试在 `crates/adapters/sdk/tests/`：`smoke.rs`、`test_sdk_client.rs`（用 `wiremock` 起假
服务端）、`multilang_ffi.rs`；测试数量以 [docs/03-reference/test-baseline.md](../../../docs/03-reference/test-baseline.md) 为单一真源。

## 已知边界

- 跨语言桥默认关：`Cargo.toml` 的 `default = []`，`python`（PyO3）/ `node`（napi-rs）/ `c`
  （cbindgen）三个 feature 需显式启用；`build.rs` 只在 `--features c` 时生成 `apeireth_sdk.h`。
- WS 流式与配额仍是桩：`ApeirethClient::invoke_stream()` 返回显式 `NotImplemented` 错误，
  `AuthPipeline::check_quota()` 恒返未实现错误码（`src/client.rs`）。
- SDK 请求的 `/v1/tools/{tool}/invoke` 路由在本仓网关侧不存在（网关只暴露
  `/v1/tools/list`），集成测试因此走 `wiremock` 假服务端；自托管直连需另建对端路由。

## 文档

- 架构: [docs/01-architecture/architecture.md](../../../docs/01-architecture/architecture.md)
- 索引: [docs/03-reference/crates.md](../../../docs/03-reference/crates.md)
