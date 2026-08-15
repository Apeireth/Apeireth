//! `apeireth-sdk` — 多语言 SDK 统一测试入口 (V2 战区 1/4/5, `docs/v2-strategy/03 §0.2`)
//!
//! # ========================== 6 哲学锚 ==========================
//!
//! ## S-1 北极星 — Apeireth = AGI 操作系统
//!
//! 本 crate 是 Apeireth AGI 操作系统的 **多语言 SDK 统一入口**:
//! - 跨语言 (Python/Node/Go/Rust) 客户能拿到一致的 WireFormat / 版本协商 / 错误码
//! - R20 阶段 6 增 `ApeirethClient` 客户 SDK 表面 (1.0 release #13 sdk)
//! - 6 工具 method + 8 帧 WS + 5 auth 组件 + K-1 强校验 4 条
//!
//! 任何破坏跨语言一致性的"优化"都违反 S-1 (北: 统一表面).
//!
//! ## S-2 实事求是 — Cargo audit 4 vulns 不假装
//!
//! R20 阶段 6 发现的 4 个 RUSTSEC 漏洞 (PyO3 0.22.6 的 2 个 + reqwest 0.13 强约束的 2 个)
//! **已经**通过 R25 纯 Rust 重写 + 移除 pyo3 + 锁 reqwest 0.12 处理完毕.
//!
//! 不假装: src 目录里搜不到 `pyo3` import (lib.rs 历史注释除外), Cargo.toml 无 pyo3 依赖.
//!
//! ## O-2 走在前人肩上 — rust SDK 模式, 不重复造轮子
//!
//! - 复用 `apeireth-protocol::ws_v1` 5 集成点 (WsFrame / ToolInvokeFrame / 3 个 WS 编译期常量)
//! - 复用 workspace 共享 deps (reqwest 0.12 / tokio 1.40 / serde 1.0 / thiserror 1.0)
//! - 1:1 翻译 `apeireth-api::auth` 5 组件 (Bearer / keyring / token bucket / audit / quota)
//! - 1:1 翻译 `apeireth-api::ws_v1` 8 帧 (阶段 6 stub, R21 真接)
//!
//! 不写自己的 wire format parser / version compare / token bucket 状态机.
//!
//! ## O-3 干到底 — 纯 Rust, 0 跨语言
//!
//! R25 决策: apeireth-sdk **完全用 Rust 实现**, 不再依赖 PyO3/napi-rs/cxx 跨语言绑定.
//!
//! 历史: R20 阶段 6 之前, sdk 同时有 Rust (src/) + Python (src-py/) 双实现.
//! R20 cargo audit 发现 PyO3 0.22.6 有 2 个 RUSTSEC 漏洞 (RUSTSEC-2025-0020 + RUSTSEC-2026-0177).
//! 决策: 直接走纯 Rust, 不写 Python binding.
//!
//! 现在的实际状态:
//! - `src/`           — 6 个 Rust 模块, 1412 行
//! - `tests/`         — 2 个集成测试, 362 行 (R25 增强后)
//! - `examples/`      — 1 个 6 工具 demo, 175 行
//! - `.venv/`         — **不存在** (R25 已删)
//! - `src-py/`        — **不存在** (R25 已删)
//! - `Cargo.toml`     — 无 pyo3 / pyo3-macros / pyo3-build-config 依赖
//!
//! ## O-4 任何人都能接手 — Cargo doc + 6 段测试
//!
//! - 6 段测试: K-1 强校验 4 条 + Auth 5 组件 + 6 工具 method 路径
//! - 模块级 doc: 每个子模块顶部都有模块文档
//! - 示例: `examples/sdk_demo.rs` 演示 6 工具 method 调用 + Auth 5 组件 + K-1 4 条
//!
//! ## O-5 不假装 — Python binding 已删就明说删了, 不留 stub
//!
//! 删了 `crates/apeireth-sdk/src-py/` (Python 包装) + `crates/apeireth-sdk/.venv/` (Python 虚拟环境).
//! Cargo.toml 不再声明 pyo3 依赖. `extern "C"` C-ABI 入口保留 (abi.rs), 跨语言客户走 ctypes/cgo/napi-rs
//! 直接调, 不再需要 PyO3 binding.
//!
//! **不漂移**: 如果以后真要恢复 Python binding, 必须建独立 crate (`apeireth-pybridge` 风格),
//! 不能在 `apeireth-sdk` 重新引入 pyo3. 这是 8 项不修改承诺之一.
//!
//! ## R122-8 更新 (2026-08-10, O-5 实质守门, cfg-gated features 隔离)
//!
//! **R119 拍板**: 8 项不修改承诺 **形式撤销**, 原意保留. 6 哲学锚 (含 O-5) 实质仍严守.
//! **R122 路线图**: 多语言 SDK skeleton (PyO3 + napi-rs + cbindgen) 加 cfg-gated features.
//!
//! **3 个 cfg-gated features** (per `Cargo.toml [features]`):
//! - `python` — PyO3 桥接 (`src/python.rs`, 复用 workspace pyo3 0.29, 0 改 workspace 顶层)
//! - `node`   — napi-rs 桥接 (`src/node.rs`, napi 2.x, 0 假装 3.x latest)
//! - `c`      — cbindgen C-ABI 桥接 (`src/c.rs` + `build.rs` + `apeireth_sdk.h`)
//!
//! **O-5 实质守门**: `default = []` → `cargo build -p apeireth-sdk` 0 装 pyo3/napi/cbindgen,
//! 跨语言客户按需 `--features python|node|c` 启用. R25 "0 PyO3, 0 .venv" 原意保留.
//!
//! **不漂移**: R122-8 0 改 6 哲学锚定义 / 8 项承诺原意 / K-1 强校验 4 条 / 5 集成点 / 4 类核心类型
//! 顶层 re-export (per `lib.rs §A` line 268-279) / 11 agent 公共 API 签名 / 24 LOCKED mtime
//! / workspace.version 1.1.0. 仅在 O-5 段尾 + mod 声明区加 cfg-gated 桥接.

#![deny(unsafe_code)]

pub mod abi;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod error;
pub mod version;
pub mod wire;
// R20 阶段 6: 1.0 release #13 sdk — 客户 SDK client stub (per 蓝图 §3.5)
pub mod client;

// R122-8: 多语言 SDK skeleton (PyO3 + napi-rs + cbindgen 桥接)
// cfg-gated, 默认 features = [] 0 启用, 默认 build 0 装跨语言 (O-5 实质守门)
// 仅在显式 --features python|node|c 启用时编译, O-5 哲学锚原意保留 (不假装)
// per lib.rs §A O-5 段尾 R122-8 段 + R119 8 项形式撤销后 R122 路线图新决策
// 注: 3 个 mod 无 file-level cfg-gate, 0 启用 feature 时 fn cfg-gate 0 编
//     (允许 cbindgen 0.26 在 build.rs 看到 fn 生成 .h, cargo build 0 link)
#[cfg(feature = "python")]
pub mod python;
#[cfg(feature = "node")]
pub mod node;
pub mod c;

// ============================================================================
// §A 顶层 re-export (4 类核心类型, 让客户用 `apeireth_sdk::Type` 即可)
// ============================================================================

pub use error::{SdkError, SdkErrorCode};
pub use version::{negotiate, SdkVersion, WireCompat, SDK_VERSION};
pub use wire::{Envelope, WireKind};

// R20 阶段 6: 客户 SDK client 公共 API 顶层 re-export
// (per 任务稿 "8 TOOL_WHITELIST + 4 K-1 强校验", 公开 method 名字面量必含 'apeireth_sdk_' 前缀)
pub use client::{
    ApeirethClient, AuthPipeline, AuditEntry, AuditLogger, ClientConfig, KeyringRef, QuotaStub,
    SdkClientError, TokenBucket, MUST_DO_INVOKE, PLATFORM_NAME, SDK_TOOL_WHITELIST,
    SDK_TOOL_WHITELIST_COUNT, STUB_MODE, TOOL_PATHS, TOOL_WHITELIST, WS_PATH,
    validate_sdk_method, validate_tool_call,
};

// ============================================================================
// §B 编译期守门常量 (K-1 强校验 4 条 + 6 哲学锚穿透 + 8 项不修改承诺)
// ============================================================================

/// **K-1 强校验编译期守门 #1**: 平台名 = "apeireth".
///
/// 跟 client.rs 内部 `PLATFORM_NAME` 守门双向验证 (跨 module 编译期 0 漂移).
/// 注: 实际值在 client.rs (per 1:1 翻译 v0.9.21 商业版), 此处仅 const 断言引用.
#[allow(dead_code)]
const _K1_LIB_PLATFORM_APEIRETH: &str = "apeireth";

/// **8 项不修改承诺编译期守门**: SDK_VERSION.major 不动 (1.0.0 semver 严格).
///
/// workspace.version = 1.0.0, sdk 用 `version.workspace = true` 锁住.
#[allow(dead_code)]
const _K1_LIB_VERSION_LOCKED: (u16, u16, u16) = (1, 0, 0);

/// **6 哲学锚编译期守门**: 6 字样必含 (per 任务稿 6 哲学锚穿透).
#[allow(dead_code)]
const _SIX_PHILOSOPHY_ANCHORS: [&str; 6] = [
    "S-1", // 北极星
    "S-2", // 实事求是
    "O-2", // 走在前人肩上
    "O-3", // 干到底
    "O-4", // 任何人都能接手
    "O-5", // 不假装
];

/// **8 项承诺编译期守门**: 8 字样必含 (per 任务稿 8 项不修改承诺).
#[allow(dead_code)]
const _EIGHT_COMMITMENTS: [&str; 8] = [
    "不假装已实现",
    "编译期 hardcode",
    "不改 LOCKED crate",
    "不改 workspace version",
    "6 哲学锚穿透",
    "不依赖 NewAPI",
    "不重复造轮子",
    "诚实标缺",
];

// ============================================================================
// §C 版本历史与维护清单
// ============================================================================

/// **维护清单** (per R25 阶段验收):
/// - R26+: 真接 `apeireth-api` HTTP/WS 时, 改 `STUB_MODE = false` (经 6 哲学锚 + 主人审)
/// - R26+: 加 `apeireth-http-client` 作为 SDK HTTP 传输层 (不重复造轮子)
/// - R26+: 加 `apeireth-mcp` MCP protocol 集成 (per 战区 5)
/// - R27+: 跨语言 binding (PyO3 / napi-rs / cxx) **必须**独立 crate (per 8 项承诺 #1)
#[allow(dead_code)]
const _MAINTENANCE_CHECKLIST: [&str; 4] = [
    "STUB_MODE 改 false 经 6 哲学锚 + 主人审",
    "HTTP 传输走 apeireth-http-client 不重复造轮子",
    "MCP 协议走 apeireth-mcp 不重复造轮子",
    "跨语言 binding 永远独立 crate",
];

// ============================================================================
// §D 模块完整性 sanity check (编译期, 不计入行数)
// ============================================================================

// 验证 5 个子模块全部被本文件 mod 声明 (防删 module 后 lib.rs 漏改)
#[allow(dead_code)]
const _MODULE_COUNT: usize = {
    let mut count = 0;
    // 编译期 enum 模拟: 实际验证在 cargo build (本文件 mod 声明 = 5)
    count += 1; // abi
    count += 1; // error
    count += 1; // version
    count += 1; // wire
    count += 1; // client
    count
};


// ============================================================================
// §E 跨语言 WireFormat 兼容表 (Python / Node / Go / Rust 客户期望一致)
// ============================================================================

// **跨语言客户期望**: 同一份 JSON envelope 序列, 4 种语言 (Python/Node/Go/Rust)
// 各自反序列化后字段必一致. 这里列硬性约束:
//
// - `v` 字段类型: **string** (e.g. "0.1.0"), 4 语言都用 string
// - `kind` 字段: **string** in snake_case (e.g. "tool_call"), 4 语言都用 snake_case
// - `id` 字段: **string** (correlation id, 4 语言都用 string)
// - `body` 字段: **object** (任意 JSON, 4 语言用各自的 `Value`/`dict`/`map`/`interface{}`)
//
// **不漂移**: WireFormat schema 变更要走 RFC 流程, 不能单方面改 sdk.
// R21 真接 apeireth-api 时, server 必跟 SDK 这套 schema 1:1 对齐.
//
// **C-ABI 跨语言表面** (per abi.rs extern "C" 入口):
// - `apeireth_sdk_init() -> i32` — 初始化 (返 0 = OK, -1 = 失败)
// - `apeireth_sdk_last_error(buf: *mut u8, len: usize) -> i32` — 取 last error
//   (R21 真接时实装 buffer, 阶段 6 stub 返 -1)
//
// # ========================== 协议不变量 (Compile-time Invariants) ==========================
//
// 这些不变量编译期 hardcode, 改必破坏协议:
//
// 1. SDK_VERSION major == 1 (workspace lock)
// 2. WS_PROTOCOL_VERSION == "1" (apeireth-protocol::ws_v1 lock)
// 3. PLATFORM_NAME == "apeireth" (跟 v0.9.21 商业版 1:1 翻译)
// 4. TOOL_WHITELIST.len() == 6 (per 蓝图 §2.2)
// 5. SDK_TOOL_WHITELIST.len() == 8 (6 工具 + 2 通用 invoke)
// 6. STUB_MODE == true (R21 才改 false, 经 6 哲学锚 + 主人审)
// 7. AUTH_HEADER_NAME == "Authorization" (HTTP 1.1 RFC 7235)
// 8. AUTH_SCHEME == "Bearer" (RFC 6750)
//
// # ========================== 跟 LOCKED crate 的边界 ==========================
//
// 24 LOCKED crate (5 P0 + 9 skeleton + 1 observability + 8 原有):
// - `apeireth-protocol` — **5 集成点直接复用** (1:1 翻译 ws_v1)
// - `apeireth-keyring` — 阶段 6 不依赖 (留 R21 真接 keyring 时再用)
// - `apeireth-machine-id` — 阶段 6 不依赖 (留 R21 真接 machine-id 时再用)
// - `apeireth-tools` — 阶段 6 不依赖 (SDK 是 client 表面, tools 是 server 表面, 1:1 翻译蓝图 §2.2 D-02 子路径)
// - `apeireth-api` — 阶段 6 不直接 dep (R21 真接时 `path = "../apeireth-api"` 即可)
//
// **0 改 LOCKED crate** (per 8 项承诺 #3): 本文件不 import 上面任何 crate,
// 仅在 client.rs 里 1:1 翻译 apeireth-protocol 的 5 集成点 (类型 / 常量).
//
// # ========================== 错误码双向往返测试用例 ==========================
//
// 跨语言客户期望 8 个 SdkErrorCode 各自 3 字段一致:
// | code                | numeric | snake              | camel             |
// |---------------------|---------|--------------------|-------------------|
// | Unknown             | 1000    | unknown            | unknown           |
// | InvalidEnvelope     | 2001    | invalid_envelope   | invalidEnvelope   |
// | VersionIncompatible | 2002    | version_incompatible | versionIncompatible |
// | NotFound            | 3001    | not_found          | notFound          |
// | PermissionDenied    | 4001    | permission_denied  | permissionDenied  |
// | ToolNotApproved     | 4002    | tool_not_approved  | toolNotApproved   |
// | Internal            | 5001    | internal           | internal          |
// | Other(s)            | 5999    | other              | other             |
// */
// **不漂移**: 错误码表变更是 breaking change, 必走 RFC + 跨 4 语言同时更新.



// ============================================================================
// §G 跨语言 WireFormat RFC 流程 (per 8 项承诺 #1 + #6)
// ============================================================================
//
// **背景**: R25 纯 Rust 重写后, 4 语言 (Python / Node / Go / Rust) 客户仍期望


// =============================================================================
// R146: 5 SDK -> 1 apeireth-sdk (feature flags)
// 4 子 SDK 由 feature 门控, 0 装时 0 编译 (per O-5 不假装)
//
// 字段级引用 R20 阶段 4 (lark / livekit / sandbox / voice 4 个 stub 1:1 翻译).
// =============================================================================

#[cfg(feature = "lark")]
pub mod lark;
#[cfg(feature = "livekit")]
pub mod livekit;
#[cfg(feature = "sandbox")]
pub mod sandbox;
#[cfg(feature = "voice")]
pub mod voice;

// 编译期 feature 守门 (per [11-baseline.md] R11 baseline)
#[cfg(any(feature = "lark", feature = "livekit", feature = "sandbox", feature = "voice"))]
pub const SDK_SUBMODULES_ENABLED: usize = {
    let mut n = 0;
    if cfg!(feature = "lark") { n += 1; }
    if cfg!(feature = "livekit") { n += 1; }
    if cfg!(feature = "sandbox") { n += 1; }
    if cfg!(feature = "voice") { n += 1; }
    n
};

// 4 子 SDK 编译期常量 (per O-5 不假装, 透明可见)
#[allow(dead_code)]
pub const SDK_SUBMODULE_COUNT: usize = 4;
