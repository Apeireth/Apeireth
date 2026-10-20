# SDK 三岛归档清单（ADR-0005 R4b / A 档）

- **归档日期**：2026-10-20
- **裁决来源**：ADR-0005 §3 R4b（owner 2026-10-20 授权；A 档 = livekit / voice / sandbox 三岛归档，
  lark 暂缓解耦后另立任务）
- **源提交**：`e8e7db33`（`main`；三岛代码与归档前 `e17f4aca` 相同）
- **原路径**：`crates/adapters/sdk/src/{livekit,voice,sandbox}/`
- **归档路径**：`legacy/archived/sdk-islands/{livekit,voice,sandbox}/`
- **规模实测**（2026-10-20，`Get-ChildItem -Recurse` + `Measure-Object -Line`）：

| 原路径 | 文件数 | 行数 | 原 feature |
|---|---:|---:|---|
| `crates/adapters/sdk/src/livekit/` | 9 | 6,517 | `livekit` |
| `crates/adapters/sdk/src/voice/` | 12 | 6,170 | `voice` |
| `crates/adapters/sdk/src/sandbox/` | 11 | 4,221 | `sandbox`（test 构建恒编译） |
| **合计** | **32** | **16,908** | — |

- **归档原因**：三岛在 `crates/` 内 sdk 之外零引用；SDK 的 `/v1/tools/*/invoke` 契约断头，
  维护成本 > 收益（ADR-0005 §3 取证）。
- **保留面**：`apeireth-sdk` 仍保留 `client` / `wire` / `im` / `abi` / 跨语言桥等在用面；
  孤岛是模块而非 crate，根 workspace members 仍为 **31**。

## 目录与依赖（恢复时按此回补）

| 岛 | 子文件（行数） | 归档前依赖 |
|---|---|---|
| `livekit` | `auth` 693 / `error` 458 / `event` 365 / `mod` 1,202 / `participant` 354 / `room` 356 / `session` 1,914 / `signal` 745 / `track` 430 | `apeireth-core`（deadline）、`crate::error_taxonomy`、`crate::frame_codec`、`crate::redact` |
| `voice` | `auth` 342 / `capture` 336 / `config` 267 / `error` 407 / `mod` 1,178 / `perception_bridge` 323 / `stream` 952 / `stt` 392 / `transport` 308 / `tts` 408 / `vad` 582 / `wake` 675 | `apeireth-core`（deadline）、`apeireth-plugin`（perception_backend）、`crate::error_taxonomy` / `frame_codec` / `redact` |
| `sandbox` | `error` 327 / `isolation` 211 / `lifecycle` 270 / `mock` 280 / `mod` 1,315 / `policy` 631 / `protocol` 450 / `quota` 300 / `resource` 248 / `runtime` 154 / `transport` 35 | `apeireth-core`（deadline / storage_atomic）、`apeireth-credentials`（SecretString）、`uuid`、`fs-err`（test）、`crate::error_taxonomy` / `redact` |

- 三岛均无独立 `examples/` / `tests/` 文件（单测在各 `.rs` 内随目录迁移）；
  `examples/sdk_demo.rs` 与 `tests/lark_mock.rs` 只涉及保留的 lark 面，本批 0 改。
- `crates/adapters/sdk/Cargo.toml` 同步移除：`uuid`、`apeireth-credentials`、`apeireth-plugin`
  三个仅三岛使用的依赖；feature `livekit` / `voice` / `sandbox` / `all-sdk` 移除；
  `default` 由 `["lark","livekit","voice"]` 收敛为 `["lark"]`。
- `src/lib.rs` 同步移除 3 个 `pub mod` 声明与 feature 计数自检；
  `SDK_SUBMODULE_COUNT` 由 4 改为 1（仅剩 lark）。

## 恢复步骤

1. `git mv legacy/archived/sdk-islands/livekit crates/adapters/sdk/src/livekit`
   （voice / sandbox 同）；
2. `crates/adapters/sdk/src/lib.rs` 回补：
   `#[cfg(feature = "livekit")] pub mod livekit;`、
   `#[cfg(any(feature = "sandbox", test))] pub mod sandbox;`、
   `#[cfg(feature = "voice")] pub mod voice;`，并恢复 feature 计数自检（`SDK_SUBMODULE_COUNT = 4`）；
3. `crates/adapters/sdk/Cargo.toml` 回补三个 feature 与 `default`、`all-sdk`，
   以及 `uuid` / `apeireth-credentials` / `apeireth-plugin` 依赖；
4. `cargo check -p apeireth-sdk --all-targets && cargo test -p apeireth-sdk --lib`；
5. 单批回滚等价于 `git revert <R4b-SDK-A-commit>`（移动 + lib.rs + Cargo.toml + MANIFEST 同批）。

## 许可 / NOTICE

- 三岛随 `apeireth-sdk` 继承根 workspace 双协议 **Apache-2.0 OR MIT**；无独立
  `LICENSE` / `NOTICE` / vendored 源码。
- 依赖 `uuid` 等为 crates.io 常规依赖；`apeireth-core` / `apeireth-credentials` /
  `apeireth-plugin` 为仓内 path crate。第三方 attribution 由根 `THIRD-PARTY-NOTICES.md`
  统一覆盖，归档不改许可义务。

## lark 为何不在本批（如实登记）

`lark`（6,741 行）同为孤岛，但 sdk 保留面 `src/im` 在代码上依赖其 wire 原语：
`im/message.rs:81,87,135`（`WebhookEvent` / `EventType` / `LarkError`）、
`im/channel.rs:59`（`MAX_MESSAGE_TEXT_BYTES`）、`im/message.rs:278`（`TextContent`）、
`im/card.rs:197`（`CardContent`，test）；gateway 又依赖 `apeireth_sdk::im`。
直接搬走会弄断 gateway，故本批保留 `pub mod lark`、`lark` feature 与 `im` 的 cfg 门原样。
解耦（抽 wire 原语到 `im` 自有模块，行为保持）后按同一清单归档，另立 commit + 独立验收。

## 未随本批处理（如实登记）

- `docs/01-architecture/founding-design-v2.md §10` 的 SDK 决策项已追加归档裁决标注；
  其余文档（`docs/03-reference/crates.md`、`docs/02-guides/...`）如仍列三岛，留给后续文档批。
- `docs/03-reference/capabilities-matrix.md` 属效率线在途改动，本批 0 触碰。
- `arch_guard` 依赖基线仍含三岛相关边，缩表走 Wave 收口（本批不跑 strict）。
