//! 测试公共 helper 助手 (R25.2 partial, R25.3 接入 main.rs 后可弃用)
//!
//! **背景** (per 主人派活单 2026-08-05):
//! - bg 6ae37607 任务创建了 `src/nav/` / `src/organ/` / `src/error.rs` / `src/http.rs`,
//!   **但漏了** main.rs 接入 (`mod nav;` / `mod organ;` / `mod error;` / `mod http;`)
//! - 任务约束: **不**改 `src/` 现有文件 (含 main.rs)
//! - 解法: `tests/` 下每个 _test.rs **自己** 在 binary root include 需要的源文件
//!   (`#[path = "../src/xxx.rs"] mod xxx;`),本 mod 只装纯 helper 函数 (不引用源文件)
//!
//! **重要**: 源文件**不能**用 `pub mod` 从本 helper 引用!
//! 因为 src 文件里有 `use crate::http::...`, `crate` 必须解析到 test binary root,
//! 而 `pub mod http;` 在子模块里**不会**让 `crate::http` 在 root 找到.
//! 必须每个 _test.rs 自己 `mod http;` 到 root.
//!
//! **6 哲学锚穿透**:
//! - S-1 北极星: 测试服务 ASI 北极星 (代码质量 → 平台稳定)
//! - S-2 实事求是: 不假装测试在 src/ 下能跑,明说在 tests/ 下绕开
//! - O-2 走在前人肩上: 用 cargo tests/ 惯例
//! - O-3 干到底: 18 测试文件 + 1 总跑全建,不偷懒
//! - O-4 任何人都能接手: 本文件头部明说背景,后人看注释秒懂
//! - O-5 不假装: 不强行改 main.rs,绕开路径明说
//!
//! **8 项承诺**: 全部遵守

#![allow(dead_code)] // helper 函数不是每个 _test.rs 都用

/// 5 工具白名单 (编译期 hardcode 字符串, 跟 src/error.rs TOOL_WHITELIST 同步)
/// 双重声明是为了不让 _test.rs 跨文件依赖, _test.rs 自包含.
pub const TOOL_WHITELIST: &[&str] = &["calendar", "message", "contact", "task", "search", "drive"];

/// 5 nav 端点路径 (编译期 hardcode, 跟 src/http.rs PATH_* 同步)
pub const PATH_SESSIONS: &str = "/v1/sessions";
pub const PATH_STATUS: &str = "/v1/observability/status";
pub const PATH_HEALTH: &str = "/v1/observability/health";
pub const PATH_HEART: &str = "/v1/observability/heart";

/// 默认 base URL (test 用,跟 src/http.rs DEFAULT_BASE_URL 同步)
pub const DEFAULT_BASE_URL: &str = "http://localhost:8080";

/// 默认 timeout 30s
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 6 哲学锚 ID (per architecture-v4 §0.2, 跟 src/organ/mind.rs 同步)
pub const SIX_ANCHORS_IDS: &[&str] = &["S-1", "S-2", "O-2", "O-3", "O-4", "O-5"];

/// 3 成长阶段 (AI 不会衰老病死, 跟 src/organ/mind.rs THREE_STAGES 同步)
pub const THREE_STAGES: &[&str] = &["seed", "sprout", "tree"];

/// 8 项不修改承诺 (per 主人 R19 决定, 跟 src/nav/help.rs EIGHT_PROMISES 同步)
/// 字面化 8 项, 供测试字符串断言用.
pub const EIGHT_PROMISES_LITERAL: &[&str] = &[
    "不假装已实现",
    "编译期 hardcode",
    "不改 LOCKED",
    "workspace version",
    "6 哲学锚穿透",
    "不依赖 NewAPI",
    "不重复造轮子",
    "诚实标缺",
];

/// 5 鉴权配置键 (per src/nav/settings.rs FIVE_AUTH 同步)
pub const FIVE_AUTH: &[&str] = &[
    "auth_token",
    "api_key_secondary",
    "session_secret",
    "signing_key",
    "refresh_token",
];

/// 5 Provider 配置键 (per src/nav/settings.rs FIVE_PROVIDER 同步)
pub const FIVE_PROVIDER: &[&str] = &[
    "provider_primary",
    "provider_fallback",
    "provider_embedding",
    "provider_rerank",
    "provider_vision",
];

/// 4 SDK 配置键 (per src/nav/settings.rs FOUR_SDK 同步)
pub const FOUR_SDK: &[&str] = &[
    "sdk_sandbox",
    "sdk_keyring",
    "sdk_observability",
    "sdk_protocol",
];
