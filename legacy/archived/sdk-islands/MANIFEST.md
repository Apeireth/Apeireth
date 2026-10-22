# SDK 孤岛归档清单（ADR-0005 R4b / A 档 + lark 补批）

- **归档日期**：2026-10-20（三岛）；2026-10-20（lark 解耦后补批）
- **裁决来源**：ADR-0005 §3 R4b（owner 2026-10-20 授权；A 档 = livekit / voice / sandbox 三岛归档，
  lark 暂缓解耦后另立任务；补批 = lark 解耦完成后归档）
- **源提交**：`e8e7db33`（三岛；`main`；代码与归档前 `e17f4aca` 相同）；
  `29387d78`（lark 解耦 commit，归档前最后含 lark 源码的树）
- **原路径**：`crates/adapters/sdk/src/{livekit,voice,sandbox,lark}/`、
  `crates/adapters/sdk/tests/lark_mock.rs`
- **归档路径**：`legacy/archived/sdk-islands/{livekit,voice,sandbox,lark}/`、
  `legacy/archived/sdk-islands/lark_mock.rs`
- **规模实测**（2026-10-20，`Get-ChildItem -Recurse` + `Measure-Object -Line`；行数 = 非空行）：

| 原路径 | 文件数 | 行数 | 原 feature |
|---|---:|---:|---|
| `crates/adapters/sdk/src/livekit/` | 9 | 6,517 | `livekit` |
| `crates/adapters/sdk/src/voice/` | 12 | 6,170 | `voice` |
| `crates/adapters/sdk/src/sandbox/` | 11 | 4,221 | `sandbox`（test 构建恒编译） |
| `crates/adapters/sdk/src/lark/` | 10 | 6,741 | `lark` |
| **合计** | **42** | **23,649** | — |

- **随岛归档**：`crates/adapters/sdk/tests/lark_mock.rs`（非空 1,130 / 总 1,235 行）→
  `legacy/archived/sdk-islands/lark_mock.rs`。该文件原 `#![cfg(feature = "lark")]`，feature 删除后
  留在 `crates/` 会变成空壳且仍持有 `apeireth_sdk::lark` 引用，故随岛迁出（恢复时移回 `tests/`）。
- **归档原因**：四岛在 `crates/` 内 sdk 之外零引用；SDK 的 `/v1/tools/*/invoke` 契约断头，
  维护成本 > 收益（ADR-0005 §3 取证）。lark 的额外前置条件已满足：im 的 wire 原语已迁入
  `crates/adapters/sdk/src/im/wire.rs`（commit `29387d78`，行为保持），`im` 对 lark 0 依赖。
- **保留面**：`apeireth-sdk` 保留 `client` / `wire` / `im`（含自有 `im::wire`）/ `abi` /
  跨语言桥等在用面；孤岛是模块而非 crate，根 workspace members 仍为 **30**
  （`cargo metadata --no-deps` 实测；三岛/lark 归档均为模块级迁出，0 members 增减）。

## 目录与依赖（恢复时按此回补）

| 岛 | 子文件（行数） | 归档前依赖 |
|---|---|---|
| `livekit` | `auth` 693 / `error` 458 / `event` 365 / `mod` 1,202 / `participant` 354 / `room` 356 / `session` 1,914 / `signal` 745 / `track` 430 | `apeireth-core`（deadline）、`crate::error_taxonomy`、`crate::frame_codec`、`crate::redact` |
| `voice` | `auth` 342 / `capture` 336 / `config` 267 / `error` 407 / `mod` 1,178 / `perception_bridge` 323 / `stream` 952 / `stt` 392 / `transport` 308 / `tts` 408 / `vad` 582 / `wake` 675 | `apeireth-core`（deadline）、`apeireth-plugin`（perception_backend）、`crate::error_taxonomy` / `frame_codec` / `redact` |
| `sandbox` | `error` 327 / `isolation` 211 / `lifecycle` 270 / `mock` 280 / `mod` 1,315 / `policy` 631 / `protocol` 450 / `quota` 300 / `resource` 248 / `runtime` 154 / `transport` 35 | `apeireth-core`（deadline / storage_atomic）、`apeireth-credentials`（SecretString）、`uuid`、`fs-err`（test）、`crate::error_taxonomy` / `redact` |
| `lark` | `approval` 568 / `auth` 839 / `calendar` 550 / `contact` 568 / `doc` 495 / `error` 581 / `http` 869 / `message` 780 / `mod` 771 / `webhook` 720 | `chrono`（calendar/approval）、`aes` + `base64`（webhook 解密）、`apeireth-core`（deadline / storage_atomic）、`fs-err`、`reqwest` / `url` / `async-trait` |

- 三岛均无独立 `examples/` / `tests/` 文件（单测在各 `.rs` 内随目录迁移）。
- `lark` 的单测在各 `.rs` 内随目录迁移；集成 mock 测试单独归档为 `lark_mock.rs`（见上）。
- `examples/sdk_demo.rs` 的 lark demo 段已删除（保留 6 工具 demo）；恢复入口见「恢复步骤」。
- `crates/adapters/sdk/Cargo.toml`：三岛批移除 `uuid`、`apeireth-credentials`、`apeireth-plugin`
  三个仅三岛使用的依赖；feature `livekit` / `voice` / `sandbox` / `all-sdk` 移除；
  `default` 由 `["lark","livekit","voice"]` 收敛为 `["lark"]`。lark 补批移除 `lark` feature，
  `default` 收敛为 `[]`；`c` / `default` / `node` / `python` 保留。
  lark 专用的**常规**依赖（`chrono` / `aes` / `base64` / `apeireth-core` / `fs-err`）**保留声明未删**：
  删除会强制改根 `Cargo.lock`，本批硬约束 0 触碰；留依赖清理批统一移除
  （恢复 lark 时因此 0 补依赖）。lark 无专用 optional dep。
- `src/lib.rs` 同步移除 4 个 `pub mod` 声明与 feature 计数自检；
  `SDK_SUBMODULE_COUNT` 由 4（三岛前）→ 1（三岛后）→ 0（lark 后）；
  `src/organ_kani_proofs.rs` 断言同步 1→0；`build.rs` 的 cbindgen include 白名单保留。

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

### lark 补批（先解耦、后归档）

1. `git mv legacy/archived/sdk-islands/lark crates/adapters/sdk/src/lark`；
   `git mv legacy/archived/sdk-islands/lark_mock.rs crates/adapters/sdk/tests/lark_mock.rs`；
2. `src/lib.rs` 回补 `#[cfg(feature = "lark")] pub mod lark;`，`SDK_SUBMODULE_COUNT` 0→1，
   `organ_kani_proofs` 三处断言 0→1；
3. `Cargo.toml` 回补 `lark = []` 与 `default = ["lark"]`（lark 依赖已在位，0 补）；
4. `examples/sdk_demo.rs` 的 lark demo 段从归档 commit 的父树恢复：
   `git show <lark-archive-commit>^:crates/adapters/sdk/examples/sdk_demo.rs`
   （只取 `lark_demo` 模块、lark mock 路由与 main 的调用段）；
5. `cargo check -p apeireth-sdk --all-targets && cargo test -p apeireth-sdk --lib`
   （另跑 `cargo test -p apeireth-sdk --test lark_mock` 覆盖集成 mock 面）。

## 许可 / NOTICE

- 四岛随 `apeireth-sdk` 继承根 workspace 双协议 **Apache-2.0 OR MIT**；无独立
  `LICENSE` / `NOTICE` / vendored 源码。
- 依赖 `uuid` / `chrono` / `aes` / `base64` 等为 crates.io 常规依赖；`apeireth-core` /
  `apeireth-credentials` / `apeireth-plugin` 为仓内 path crate。第三方 attribution 由根
  `THIRD-PARTY-NOTICES.md` 统一覆盖，归档不改许可义务。

## lark 补批记录（本清单已收口）

- A 档（三岛）时 lark 暂缓的**真实原因**：`src/im` 在代码上依赖 lark 的 wire 原语
  （`im/message.rs:81,87,135` 的 `WebhookEvent` / `EventType` / `LarkError`、
  `im/channel.rs:59` 的 `MAX_MESSAGE_TEXT_BYTES`、`im/message.rs:278` 的 `TextContent`、
  `im/card.rs:197` 的 `CardContent`），而 gateway 依赖 `apeireth_sdk::im`；直接搬走会断 gateway。
- 补批前置 commit `29387d78`：把**最小 wire 原语**（信封解析 / 两种内容形状 / 预算锚 /
  3 变体错误）迁入 `src/im/wire.rs`（504 行），im 去掉 lark feature 门；实测
  `cargo test -p apeireth-sdk --lib --no-default-features` = 92 passed / 0 failed，
  gateway 5 个 IM 集成测试 = 40 passed / 0 failed，行为保持（仅诊断前缀收敛为 `im wire`）。
- 本补批 commit：纯 `git mv` + 删 feature / mod / demo + 本清单；
  `git grep -n 'apeireth_sdk::lark\|crate::lark' -- crates/` = 0。

## 未随本批处理（如实登记）

- `docs/01-architecture/founding-design-v2.md §10` 的 SDK 决策项已追加归档裁决标注；
  其余文档（`docs/03-reference/crates.md`、`docs/02-guides/...` 等）如仍列四岛或 lark，
  留给后续文档批。
- `docs/03-reference/capabilities-matrix.md` 属效率线在途改动，本批 0 触碰。
- `arch_guard` 依赖基线仍含四岛相关边（新增 lark 边亦待缩表），缩表走 Wave 收口（本批不跑 strict）。
- 依赖清理批：`crates/adapters/sdk/Cargo.toml` 的 `chrono` / `aes` / `base64` / `apeireth-core` /
  `fs-err` 现仅归档岛使用，待与根 `Cargo.lock` 一次性清理（本批硬约束 0 触碰）。
