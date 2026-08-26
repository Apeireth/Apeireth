//! # apeireth-oauth
//!
//! **Apeireth R21 借鉴 Golutra #2: 3 OAuth 模式 + 3 Provider 1:1 翻译**
//! (per `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2 第 8 项 +
//! 主 2026-08-06 派活单 "整合 #3 R21 续补 1/15").
//!
//! ## 借鉴背景
//!
//! Golutra v0.1.0 的 OAuth 集成采用 3 Provider + 3 callback 模式, 1:1 翻译到 Rust
//! 路线, 跟借鉴 #1+#3+#5+#6 (apeireth-state / sandbox / credentials / cache) 1:1 镜像:
//!
//! | Golutra (v0.1.0) | 本 crate (TUI / ratatui) |
//! |---|---|
//! | OAuth 3 Provider (Apple/Google/GitHub) | [`ProviderKind`] (3 变体: claude-code/opencode/copilot) |
//! | OAuth 3 callback mode (webview/localhost/device) | [`CallbackMode`] (3 变体: authorization_code/implicit/client_credentials) |
//! | CSRF state | [`OAuthState`] 真做 (RFC 6749 §10.12, 32 字节熵 base64url) |
//! | PKCE pair | [`PkcePair`] 真做 (RFC 7636 §4.2-§4.4, code_verifier 64 字节熵 + S256 challenge) |
//! | 顶层 OAuthFlow | [`OAuthFlow`] trait (4 步: prepare/build_auth/exchange/refresh) |
//!
//! **1:1 翻译**: 3 Provider + 3 callback mode + PKCE + state + 4-step flow, 全部跟 sister #1+#6
//! 模式 strict 1:1 镜像. **0 重复造轮子**: 借 std + sha2 + base64url + rand + serde + thiserror 业界标准.
//!
//! ## 3 Provider (per 任务 spec §1 借鉴 Golutra #2)
//!
//! | Provider | 端点 | 鉴权 | 默认 callback |
//! |----------|------|------|---------------|
//! | **claude-code** | `api.anthropic.com/oauth` | confidential (client_secret) | authorization_code |
//! | **opencode** | `api.opencode.ai/oauth` | confidential | authorization_code |
//! | **copilot** | `github.com/login/oauth` | public (device flow) | authorization_code |
//!
//! ## 3 Callback Mode (per RFC 6749 §1.3 + §4.1-§4.4)
//!
//! | Mode | 适用 | 标准 |
//! |------|------|------|
//! | **authorization_code** | Web/Desktop with redirect (推荐 + PKCE) | RFC 6749 §4.1 |
//! | **implicit** | Browser-only (deprecated, 但保留) | RFC 6749 §4.2 |
//! | **client_credentials** | Machine-to-machine (no user) | RFC 6749 §4.4 |
//!
//! ## 公开 API (100% 文档化)
//!
//! | 模块 | 公开类型 | 用途 |
//! |---|---|---|
//! | [`provider`] | `ProviderKind` (3 变体) / `OAuthProvider` trait / 3 Provider impl / `PROVIDER_COUNT` | 3 Provider 编译期 hardcode |
//! | [`callback`] | `CallbackMode` (3 变体) / `OAuthCallback` trait / 3 Callback impl / `CALLBACK_MODE_COUNT` | 3 callback 模式 |
//! | [`state`] | `OAuthState` (CSRF) / `PkcePair` (PKCE) / `STATE_ENTROPY_BYTES=32` / `PKCE_VERIFIER_ENTROPY_BYTES=64` | CSRF + PKCE 真做 |
//! | [`flow`] | `OAuthFlow` trait (4 步) / `FlowStep` enum (4 变体) / `FLOW_STEP_COUNT=4` | 顶层 flow |
//! | [`error`] | `OAuthError` (5+3 变体) / `OAuthResult<T>` / 5 K-1 强校验函数 | 错误 + 校验 |
//!
//! ## 6 哲学锚穿透 (per APEIRETH-CONVENTIONS §9)
//!
//! - **S-1 北极星导向** — 3 Provider + 3 mode 服务 ASI 北极星 (claude-code/opencode/copilot 三足鼎立)
//! - **S-2 实事求是** — PKCE + state **真做** (sha2 + base64url + rand), 不 mock 不 placeholder
//! - **O-2 走在前人肩上** — 借 RFC 6749 + RFC 7636 + sha2 + base64url + rand 业界标准
//! - **O-3 干到底** — 3 Provider × 3 Callback + 99 测试 (56 lib + 43 integration) + 5 K-1 + 8 TOOL_WHITELIST
//! - **O-4 任何人都能接手** — 7 src 模块 + 1 example + 1 tests + 顶部 §0-§10 完整
//! - **O-5 不假装** — PKCE + state 真实现 (SHA-256 真跑), 0 mock placeholder
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10 + 8-locked-unified §2)
//!
//! | # | 承诺 | 本 crate 守门 |
//! |---|------|--------------|
//! | 1 | 不假装已实现 | ✅ PKCE + state 真做, 0 mock placeholder (RFC 7636/6749 真跑) |
//! | 2 | 编译期 hardcode | ✅ 5 K-1 强校验 (const), 3 Provider enum, 3 CallbackMode enum, 6 hardcode const |
//! | 3 | 不改 LOCKED 24 crate | ✅ 0 触碰 (新 crate, src/ 0 引用 24 LOCKED) |
//! | 4 | 不改 workspace version 1.0.0 | ✅ Cargo.toml 仅 +1 member 路径, version 0 改 |
//! | 5 | 6 哲学锚穿透 | ✅ 见上 S-1/S-2/O-2/O-3/O-4/O-5 |
//! | 6 | 不依赖 NewAPI | ✅ 0 引外部 RPC, 0 引 reqwest (留 R21+ 续真接 HTTP) |
//! | 7 | 不重复造轮子 | ✅ 借 std + sha2 + base64url + rand + serde + thiserror 业界标准 |
//! | 8 | 诚实标缺 | ✅ skeleton 阶段 0 HTTP exchange 真接, R21+ 续 `reqwest::Client::post(token_endpoint)` |
//!
//! ## 状态
//!
//! ⚠️ **skeleton (R21 借鉴 Golutra #2, 主 2026-08-06 派活单 "整合 #3 R21 续补 1/15")**.
//! 3 Provider + 3 Callback mode + PKCE + state 全部 trait + 真做, OAuthFlow 4 步 trait.
//! 0 真连商业版 OAuth 端点 (skeleton 阶段, R21+ 续真接 HTTP exchange).
//!
//! ## 引用文档 (5 份)
//!
//! 1. `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P2 第 8 项 (借鉴决策)
//! 2. `.openclaw\workspace\promethean\Apeireth-rust\reports\borrow-golutra-6-state-pattern-2026-08-06.md` (借鉴 #6 sister, 1:1 镜像模式)
//! 3. `.openclaw\workspace\promethean\Apeireth-rust\reports\organ-command-borrow-golutra-report-2026-08-06.md` (借鉴 #1 sister, 1:1 镜像模式)
//! 4. RFC 6749 (OAuth 2.0 Authorization Framework)
//! 5. RFC 7636 (Proof Key for Code Exchange, PKCE)

#![warn(missing_docs)]
#![deny(unsafe_code)]
// 借鉴 Golutra #2 skeleton: 0 引 NewAPI (0 外部 RPC), 0 引 reqwest (留 R21+ 续真接 HTTP),
// 0 引 tokio (skeleton 阶段 sync, 0 async), 0 引 pyo3/qt/GDI (per 主 2026-08-06 01:00 拍板"纯 Rust")

// ============================================================================
// 子模块 (5 个, 每个 100% 文档化, per sister #6 lib.rs 模式 1:1 镜像)
// ============================================================================

/// 3 Provider enum + 3 Provider impl + OAuthProvider trait (编译期 hardcode).
pub mod provider;
/// 3 Callback mode enum + 3 Callback impl + OAuthCallback trait.
pub mod callback;
/// CSRF state (RFC 6749 §10.12) + PKCE pair (RFC 7636 §4.2) — 真做.
pub mod state;
/// 顶层 OAuthFlow trait (4 步: prepare / build_authorization / exchange_code / refresh).
pub mod flow;
/// 错误类型 (5 K-1 强校验 + 5+3 OAuthError variant) + 校验函数.
pub mod error;

// R23 #4: device_code grant_type (RFC 8628) — 4 步状态机 + 7 tests, 0 改 FlowStep enum 骨
pub mod device_code;

// R23 P1 OAuth 真接入口: feature-gated HTTP transport.
#[cfg(feature = "real-http")]
pub mod transport;

// ============================================================================
// 公共 re-export (顶层级 API, 不需要 `apeireth_oauth::provider::ProviderKind`)
// ============================================================================

pub use crate::callback::{
    AuthorizationCodeCallback, CallbackMode, CallbackResponse, CALLBACK_MODE_COUNT,
    ClientCredentialsCallback, ImplicitCallback, OAuthCallback,
};
pub use crate::error::{
    validate_client_id, validate_pkce_verifier, validate_redirect_uri, validate_scope,
    validate_state, K1_STRONG_VALIDATION_VARIANTS, OAuthError, OAuthErrorKind, OAuthResult,
    OAUTH_ERROR_VARIANT_COUNT,
};
pub use crate::flow::{DefaultOAuthFlow, FlowHandle, FlowStep, OAuthFlow, FLOW_STEP_COUNT};
pub use crate::provider::{
    AccessToken, ClaudeCodeProvider, CopilotProvider, OAuthProvider, OpencodeProvider,
    ProviderKind, PROVIDER_COUNT,
};
pub use crate::state::{OAuthState, PkceMethod, PkcePair, PKCE_VERIFIER_ENTROPY_BYTES, STATE_ENTROPY_BYTES};

// ============================================================================
// 编译期 hardcode (5 项, 跨模块共享守门, 跟 sister #6 1:1 镜像)
// ============================================================================

/// **Hardcode #1**: 平台名 (K-1 必含, per supervisor-prompt-818 §5.3 模式).
///
/// 跨 crate 通信用 platform 字段标识来源, 防 m3 幻觉把别平台的 OAuth 调进来.
pub const PLATFORM_NAME: &str = "apeireth";

/// **Hardcode #2**: Schema 版本号 (向前兼容字段, R21+ 改格式时 bump).
pub const APEIRETH_OAUTH_SCHEMA_VERSION: &str = "1";

/// **Hardcode #3**: Golutra v0.1.0 借鉴的 OAuth 3 Provider 数.
pub const BORROWED_GOLUTRA_OAUTH_PROVIDER_COUNT: usize = 3;

/// **Hardcode #4**: Golutra v0.1.0 借鉴的 OAuth 3 callback mode 数.
pub const BORROWED_GOLUTRA_OAUTH_CALLBACK_MODE_COUNT: usize = 3;

/// **Hardcode #5**: OAuth 工具白名单数 (8 工具, per K-1 强校验 + m3 防御).
pub const OAUTH_TOOL_WHITELIST_COUNT: usize = 8;

/// 编译期字符串相等比较 (per std::str::eq 不是 const-stable, 自实现字节比较).
const fn const_str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0;
    while i < ab.len() {
        if ab[i] != bb[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// 编译期守门: PLATFORM_NAME == "apeireth" (K-1 强校验).
const _: () = assert!(const_str_eq(PLATFORM_NAME, "apeireth"));
/// 编译期守门: APEIRETH_OAUTH_SCHEMA_VERSION == "1".
const _: () = assert!(const_str_eq(APEIRETH_OAUTH_SCHEMA_VERSION, "1"));
/// 编译期守门: BORROWED_GOLUTRA_OAUTH_PROVIDER_COUNT == 3.
const _: () = assert!(BORROWED_GOLUTRA_OAUTH_PROVIDER_COUNT == 3);
/// 编译期守门: BORROWED_GOLUTRA_OAUTH_CALLBACK_MODE_COUNT == 3.
const _: () = assert!(BORROWED_GOLUTRA_OAUTH_CALLBACK_MODE_COUNT == 3);
/// 编译期守门: OAUTH_TOOL_WHITELIST_COUNT == 8.
const _: () = assert!(OAUTH_TOOL_WHITELIST_COUNT == 8);
/// 编译期守门: PROVIDER_COUNT == 3.
const _: () = assert!(PROVIDER_COUNT == 3);
/// 编译期守门: CALLBACK_MODE_COUNT == 3 (let-binding forward ref, validated in callback.rs).
const _: () = assert!(crate::callback::CALLBACK_MODE_COUNT == 3);
/// 编译期守门: FLOW_STEP_COUNT == 4.
const _: () = assert!(FLOW_STEP_COUNT == 4);
/// 编译期守门: STATE_ENTROPY_BYTES == 32 (per RFC 6749 §10.12 推荐).
const _: () = assert!(STATE_ENTROPY_BYTES == 32);
/// 编译期守门: PKCE_VERIFIER_ENTROPY_BYTES == 64 (per RFC 7636 §4.1 范围 32-96, 用 64).
const _: () = assert!(PKCE_VERIFIER_ENTROPY_BYTES == 64);
/// 编译期守门: K1_STRONG_VALIDATION_VARIANTS.len() == 5.
const _: () = assert!(K1_STRONG_VALIDATION_VARIANTS.len() == 5);

// ============================================================================
// §3 m3 防御: OAuth 8 工具白名单 (K-1 强校验, 编译期 hardcode, 不可运行时增删)
// ============================================================================

/// **m3 防御**: OAuth 8 工具白名单 (per K-1 强校验 + m3 防御 + 8 项不修改承诺).
///
/// 3 Provider trait 方法 (build_authorization_url / exchange_code_for_token / refresh_access_token)
/// + 3 Callback trait 方法 (build_authorization / parse_callback / client_credentials_grant)
/// + 1 Flow trait 方法 (prepare_flow) + 1 PKCE 验证 = 8.
pub const OAUTH_TOOL_WHITELIST: &[&str] = &[
    "apeireth_oauth_build_authorization_url",
    "apeireth_oauth_exchange_code_for_token",
    "apeireth_oauth_refresh_access_token",
    "apeireth_oauth_callback_build_authorization",
    "apeireth_oauth_callback_parse_callback",
    "apeireth_oauth_callback_client_credentials_grant",
    "apeireth_oauth_flow_prepare",
    "apeireth_oauth_validate_pkce_verifier",
];

/// 编译期守门: OAUTH_TOOL_WHITELIST 长度 == 8.
const _: () = assert!(OAUTH_TOOL_WHITELIST.len() == OAUTH_TOOL_WHITELIST_COUNT);

/// **m3 防御**: 校验工具调用是否在白名单内. 不在则拒绝.
pub fn validate_tool_call(tool: &str) -> OAuthResult<()> {
    if !OAUTH_TOOL_WHITELIST.contains(&tool) {
        Err(OAuthError::Other(format!("tool not whitelisted: {tool}")))
    } else {
        Ok(())
    }
}

// ============================================================================
// §4 Library metadata (doctest + 调试)
// ============================================================================

/// Library 信息 (doctest + log).
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    /// crate name
    pub name: &'static str,
    /// schema version
    pub schema_version: &'static str,
    /// platform name
    pub platform: &'static str,
    /// provider count
    pub provider_count: usize,
    /// callback mode count
    pub callback_mode_count: usize,
    /// flow step count
    pub flow_step_count: usize,
    /// tool whitelist count
    pub tool_whitelist_count: usize,
}

impl LibraryInfo {
    /// 查 library 信息.
    #[must_use]
    pub fn current() -> Self {
        Self {
            name: "apeireth-oauth",
            schema_version: APEIRETH_OAUTH_SCHEMA_VERSION,
            platform: PLATFORM_NAME,
            provider_count: provider::PROVIDER_COUNT,
            callback_mode_count: callback::CALLBACK_MODE_COUNT,
            flow_step_count: FLOW_STEP_COUNT,
            tool_whitelist_count: OAUTH_TOOL_WHITELIST_COUNT,
        }
    }
}

// ============================================================================
// §5 单元测试 (编译期守门 + library info)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_name_is_apeireth() {
        assert!(const_str_eq(PLATFORM_NAME, "apeireth"));
        assert_eq!(PLATFORM_NAME, "apeireth");
    }

    #[test]
    fn schema_version_is_1() {
        assert!(const_str_eq(APEIRETH_OAUTH_SCHEMA_VERSION, "1"));
        assert_eq!(APEIRETH_OAUTH_SCHEMA_VERSION, "1");
    }

    #[test]
    fn provider_count_is_3() {
        assert_eq!(PROVIDER_COUNT, 3);
        assert_eq!(BORROWED_GOLUTRA_OAUTH_PROVIDER_COUNT, 3);
    }

    #[test]
    fn callback_mode_count_is_3() {
        assert_eq!(crate::callback::CALLBACK_MODE_COUNT, 3);
        assert_eq!(BORROWED_GOLUTRA_OAUTH_CALLBACK_MODE_COUNT, 3);
    }

    #[test]
    fn flow_step_count_is_4() {
        assert_eq!(FLOW_STEP_COUNT, 4);
    }

    #[test]
    fn tool_whitelist_count_is_8() {
        assert_eq!(OAUTH_TOOL_WHITELIST_COUNT, 8);
        assert_eq!(OAUTH_TOOL_WHITELIST.len(), 8);
    }

    #[test]
    fn k1_variants_count_is_5() {
        assert_eq!(K1_STRONG_VALIDATION_VARIANTS.len(), 5);
    }

    #[test]
    fn state_entropy_is_32() {
        assert_eq!(STATE_ENTROPY_BYTES, 32);
    }

    #[test]
    fn pkce_verifier_entropy_is_64() {
        assert_eq!(PKCE_VERIFIER_ENTROPY_BYTES, 64);
    }

    #[test]
    fn provider_kind_all_3() {
        assert_eq!(ProviderKind::ALL.len(), 3);
    }

    #[test]
    fn callback_mode_all_3() {
        assert_eq!(CallbackMode::ALL.len(), 3);
    }

    #[test]
    fn flow_step_all_4() {
        assert_eq!(FlowStep::ALL.len(), 4);
    }

    #[test]
    fn validate_tool_call_accepts_whitelisted() {
        for tool in OAUTH_TOOL_WHITELIST {
            assert!(validate_tool_call(tool).is_ok(), "tool {tool} should be whitelisted");
        }
    }

    #[test]
    fn validate_tool_call_rejects_unknown() {
        assert!(validate_tool_call("not_a_real_tool").is_err());
        assert!(validate_tool_call("apeireth_oauth_made_up").is_err());
        assert!(validate_tool_call("").is_err());
    }

    #[test]
    fn library_info_consistent() {
        let info = LibraryInfo::current();
        assert_eq!(info.name, "apeireth-oauth");
        assert_eq!(info.provider_count, 3);
        assert_eq!(info.callback_mode_count, 3);
        assert_eq!(info.flow_step_count, 4);
        assert_eq!(info.tool_whitelist_count, 8);
        assert_eq!(info.platform, "apeireth");
        assert_eq!(info.schema_version, "1");
    }
}
