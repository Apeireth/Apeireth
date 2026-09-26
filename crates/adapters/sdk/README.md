# apeireth-sdk

> Apeireth 纯 Rust SDK 客户端 (1.0 release #2 install), 0 PyO3, 0 .venv

apeireth-sdk 是 Apeireth 1.0 (AGI 操作系统) 工作区 crate 之一。完整架构见 [docs/](../../docs/README.md)。

## 模块 (10 src 文件 + 4 子 SDK / 30 测试 + 2 Kani proof + 35 集成)

- `src/lib.rs` — SDK 入口 + 5 子 SDK (lark/livekit/sandbox/voice) re-export
- `src/client.rs` — ApeirethClient 主入口 (reqwest + rustls-tls, JSON-RPC over HTTP) + 16 测试
- `src/wire.rs` — wire types (Request/Response/Notification, serde derive)
- `src/error.rs` — SdkError enum (thiserror 派生)
- `src/version.rs` — SDK 版本常量 + 编译期 hardcode (workspace version 1.2.0)
- `src/abi.rs` — C-ABI cbindgen header 生成面 (R122-8, c feature 启用时由 build.rs 产出)
- `src/c.rs` — 5 fn C 签名 (c feature)
- `src/python.rs` — PyO3 桥接骨架 (python feature, workspace pyo3 0.29)
- `src/node.rs` — napi-rs 桥接骨架 (node feature, 2.x 真实 branch)
- `src/organ_kani_proofs.rs` — sdk organ Kani proofs (R177, 5 测试 + 2 `#[kani::proof]`)
- 子 SDK 目录: `src/lark/`, `src/livekit/`, `src/sandbox/`, `src/voice/` (5 SDK → 1 crate, R146 feature-gated)
- 集成测试: `tests/multilang_ffi.rs` (6) + `tests/smoke.rs` (8) + `tests/test_sdk_client.rs` (21)
