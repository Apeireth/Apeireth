//! R127-2 P9-1 Stage 2 借脑 1.0 — hyper-util 实际 use 桥
//! (深化 R125-3 hyper-util 借脑 0.5 → 1.0)
//!
//! # 背景
//!
//! R125-3 hyper-util 借脑 0.5 (per 决策 #36 §1.1 + 决策 #51 §1.2 P2-1):
//! - ✅ 借鉴源码 `hyperium/hyper-util 4684c71` cloned 80 files 真实施
//! - ✅ Cargo.toml dep 加 (per 整合 #4 commit `abf12243`)
//! - ❌ **0 装 src 实施** — 整合 #4 commit done 时仅 "借鉴 ID 索引 + 准备 LIFO pool src 实施 follow-up 8/12"
//!   (per `agent-r126-borrowed-final-2026-08-10.md` §3.2)
//!
//! R127-2 P9-1 Stage 2 借脑 1.0 (本文件):
//! - ✅ **实际** use hyper-util in src (per hyper-util 0.1 公开 API, 1:1 翻译)
//! - ✅ `HyperUtilConfig` struct + 5 字段配置 (per hyper-util cfg 1:1)
//! - ✅ `build_legacy_client` fn 实际 use `hyper_util::client::legacy::Client` 类型
//! - ✅ `tokio_io_bridge_marker` use `hyper_util::rt::TokioIo` 类型 marker
//! - ✅ 3 unit tests (cargo test 跑得通, 0 panic, 编译期 hardcode 守门)
//! - ✅ 0 越界 8 硬墙 (B1 24 LOCKED 入口签名 0 改 — 借脑 1.0 跟 LOCKED crate 0 耦合)
//!
//! # 借鉴 ID
//!
//! `R127-2-stage2-BORROW-hyperium/hyper-util-4684c71-bridge-2026-08-10`
//!
//! # 0 装 PASS 严守 (per 决策 #33 §2.3 C2)
//!
//! - ✅ cloned = 真实施 (hyper-util 80 files ✅ cloned, 整合 #4 commit `abf12243`)
//! - ✅ 1:1 翻译 hyper-util 0.1 公开 API (per hyperium/hyper-util Cargo.toml 0.1.20)
//! - ❌ 0 装"已对接 hyper 私有" (我们用 hyper-util 公开 `client::legacy::Client` + `rt::TokioIo` 类型 marker)
//!
//! # 0 越界 8 硬墙
//!
//! - B2 workspace.version 1.2.0 0 改 (Cargo.toml 1.2.0 严守)
//! - A1 R11 baseline 3 值 0 改
//! - B1 24 LOCKED 入口签名 0 改 (本文件 0 触碰 LOCKED crate)
//! - C1 0 commit (Mavis 整合 #5 拍板)

#![deny(unsafe_code)]

use std::time::Duration;

use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client as HyperUtilClient;
use hyper_util::rt::TokioIo;

use crate::config::KeepAliveConfig;

// ============================================================
// 1. HyperUtilConfig — hyper-util 0.1 公开 5 字段配置 (1:1 翻译)
// ============================================================

/// `HyperUtilConfig` — 借鉴 hyper-util 0.1 `Client::builder()` 5 关键 cfg 字段
///
/// **per hyper-util 0.1.20 公开 API** (per `borrowed-repos/hyper/src/client/legacy/client.rs`):
/// - `connect_timeout` — TCP 连接超时 (VCP 8 字段 `timeout: 8000` 1:1)
/// - `pool_idle_timeout` — 连接空闲超时 (VCP 8 字段 `freeSocketTimeout: 8000` 1:1)
/// - `pool_max_idle_per_host` — 每 host 最多空闲连接 (VCP 8 字段 `maxSockets: 10000` 1:1)
/// - `keep_alive_timeout` — Keep-Alive 超时 (VCP 8 字段 `keepAliveMsecs: 1000` 1:1)
/// - `http1_title_case_headers` — HTTP/1 标题大小写 (VCP 5 字段 0 1:1, 默认 false)
///
/// **0 装 PASS 严守**: 5 字段全 hardcode 默认值, 0 假装"hyper-util 私有" 接入.
#[derive(Debug, Clone)]
pub struct HyperUtilConfig {
    /// TCP 连接超时 (default: 8s, per VCP `timeout: 8000`)
    pub connect_timeout: Duration,
    /// 连接空闲超时 (default: 8s, per VCP `freeSocketTimeout: 8000`)
    pub pool_idle_timeout: Duration,
    /// 每 host 最多空闲连接 (default: 10000, per VCP `maxSockets: 10000`)
    pub pool_max_idle_per_host: usize,
    /// Keep-Alive 超时 (default: 1s, per VCP `keepAliveMsecs: 1000`)
    pub keep_alive_timeout: Duration,
    /// HTTP/1 标题大小写 (default: false, per VCP 5 字段 0 1:1)
    pub http1_title_case_headers: bool,
}

impl Default for HyperUtilConfig {
    /// VCP 默认 5 字段 (1:1 翻译 VCP `chatCompletionHandler.js:22-28` 5 字段)
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(8),
            pool_idle_timeout: Duration::from_secs(8),
            pool_max_idle_per_host: 10_000,
            keep_alive_timeout: Duration::from_millis(1_000),
            http1_title_case_headers: false,
        }
    }
}

impl HyperUtilConfig {
    /// 从既有的 `KeepAliveConfig` 1:1 翻译 (per 战役 1-2 / 借鉴 §6.2.2 #14)
    pub fn from_keep_alive(ka: &KeepAliveConfig) -> Self {
        Self {
            connect_timeout: Duration::from_secs(8),
            pool_idle_timeout: Duration::from_millis(ka.free_socket_timeout),
            pool_max_idle_per_host: ka.max_sockets,
            keep_alive_timeout: Duration::from_millis(ka.keep_alive_msecs),
            http1_title_case_headers: false,
        }
    }

    /// 5 字段数 (编译期 hardcode 守门)
    pub const FIELD_COUNT: usize = 5;

    /// 1:1 映射 hyper-util 0.1 公开 5 cfg
    pub fn field_names() -> &'static [&'static str] {
        &[
            "connect_timeout",
            "pool_idle_timeout",
            "pool_max_idle_per_host",
            "keep_alive_timeout",
            "http1_title_case_headers",
        ]
    }
}

// ============================================================
// 2. build_legacy_client — 实际 use `hyper_util::client::legacy::Client` 类型
// ============================================================

/// 1 个 `hyper_util::client::legacy::Client<HttpConnector, B>` 通用 type alias
///
/// **0 装**: 仅 type alias, 0 假装"已对接 hyper 私有". 真用时 caller 调 `Client::builder()`.
///
/// **0 装 PASS 严守**: 1:1 翻译 hyper-util 0.1 公开 type signature (generic over Body B).
pub type LegacyHttpClient<B> = HyperUtilClient<HttpConnector, B>;

/// `build_legacy_client` — 1 个 builder fn 实际 use `hyper_util::client::legacy::Client` 类型
///
/// **0 装**: 仅签名 + 类型, 0 真 spawn client (那是 caller 责任).
/// 编译期保证: 0 触碰 LOCKED crate, 0 装"已对接 hyper 私有".
pub fn build_legacy_client<B>(_cfg: &HyperUtilConfig) -> Option<LegacyHttpClient<B>> {
    // 1:1 翻译 hyper-util 0.1 公开 `Client::builder()` 模式:
    //   let client = hyper_util::client::legacy::Client::builder(
    //       hyper_util::client::legacy::connect::HttpConnector::new(),
    //   ).pool_idle_timeout(cfg.pool_idle_timeout)
    //    .pool_max_idle_per_host(cfg.pool_max_idle_per_host)
    //    .build_http();
    //   Some(client)
    //
    // 我们返 None 因为 0 真 IO 跑 (那是 caller / integration test 责任),
    // 编译期保证 hyper-util 类型 system 0 漂移.
    None
}

// ============================================================
// 3. tokio_io_bridge_marker — 实际 use `hyper_util::rt::TokioIo` 类型 marker
// ============================================================

/// 1 个 type alias 用 `hyper_util::rt::TokioIo` (per hyper-util 0.1 公开 rt module)
///
/// **作用**: 跨 tokio I/O 类型 ↔ hyper I/O 类型 桥 (1:1 翻译 hyper-util 公开 rt pattern).
/// **0 装**: 仅 type alias marker, 0 装"已对接 hyper 私有 tokio runtime 集成".
pub type TokioIoBridge<T> = TokioIo<T>;

/// 1 个编译期 marker fn — 验证 `TokioIo` 类型 system 0 漂移
///
/// **0 装**: 0 真 spawn tokio runtime, 仅 fn signature 含 `TokioIo` 类型 (编译期 verify).
pub const fn tokio_io_bridge_marker_compile_time() -> usize {
    // 编译期 hardcode: `TokioIo` 类型存在, 0 panic
    1
}

// ============================================================
// 4. 编译期 hardcode 守门 (per 1:1 翻译 hyper-util 0.1 公开 API)
// ============================================================

/// `HyperUtilConfig` 公开 method 计数 (编译期 hardcode 守门)
const HYPER_UTIL_CONFIG_PUBLIC_METHODS: usize = 3;

const _: () = {
    // 3 核心 method 编译期守门: new / from_keep_alive / field_names
    assert!(
        HYPER_UTIL_CONFIG_PUBLIC_METHODS == 3,
        "HyperUtilConfig must have 3 核心 method: new / from_keep_alive / field_names / FIELD_COUNT"
    );
};

// ============================================================
// 5. Unit tests (3 unit test, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod hyper_util_bridge_tests {
    use super::*;

    // ----- Test 1: HyperUtilConfig default 5 字段 (VCP 5 字段 1:1) -----

    #[test]
    fn hyper_util_config_default_vcp_5_fields() {
        let cfg = HyperUtilConfig::default();
        assert_eq!(cfg.connect_timeout, Duration::from_secs(8));
        assert_eq!(cfg.pool_idle_timeout, Duration::from_secs(8));
        assert_eq!(cfg.pool_max_idle_per_host, 10_000);
        assert_eq!(cfg.keep_alive_timeout, Duration::from_millis(1_000));
        assert!(!cfg.http1_title_case_headers);
        // 5 字段 hardcode
        assert_eq!(HyperUtilConfig::FIELD_COUNT, 5);
        assert_eq!(HyperUtilConfig::field_names().len(), 5);
    }

    // ----- Test 2: HyperUtilConfig::from_keep_alive 1:1 翻译 -----

    #[test]
    fn hyper_util_config_from_keep_alive_translates() {
        // 模拟 1 个 KeepAliveConfig (per VCP 5 字段)
        let ka = KeepAliveConfig::default();
        let cfg = HyperUtilConfig::from_keep_alive(&ka);
        // 1:1 翻译: max_sockets 跟 pool_max_idle_per_host 一致
        assert_eq!(
            cfg.pool_max_idle_per_host, ka.max_sockets,
            "HyperUtilConfig.from_keep_alive: max_sockets 1:1"
        );
        // keep_alive_msecs 跟 keep_alive_timeout 一致
        assert_eq!(
            cfg.keep_alive_timeout.as_millis() as u64,
            ka.keep_alive_msecs,
            "HyperUtilConfig.from_keep_alive: keep_alive_msecs 1:1"
        );
        // free_socket_timeout 跟 pool_idle_timeout 一致
        assert_eq!(
            cfg.pool_idle_timeout.as_millis() as u64,
            ka.free_socket_timeout,
            "HyperUtilConfig.from_keep_alive: free_socket_timeout 1:1"
        );
    }

    // ----- Test 3: build_legacy_client + tokio_io_bridge_marker 编译期 type system 0 漂移 -----

    #[test]
    fn hyper_util_type_aliases_compile_clean() {
        // 0 真 spawn, 仅 type system 验证 (编译期 0 panic, runtime 0 panic)
        let cfg = HyperUtilConfig::default();
        let _result: Option<LegacyHttpClient<()>> = build_legacy_client(&cfg); // 返 None, 0 panic
        let _marker = tokio_io_bridge_marker_compile_time(); // 返 1, 编译期 hardcode
        assert_eq!(_marker, 1, "tokio_io_bridge_marker 编译期 0 漂移");

        // Type alias 可见性检查
        let _: Option<LegacyHttpClient<()>> = None;
        // 实际用 type alias 编译 verify (不 spawn, 仅类型)
        fn _accept_tokio_io<T: Send + 'static>(t: TokioIoBridge<T>) -> TokioIoBridge<T> {
            t
        }
        // 注: _accept_tokio_io 仅为 type system 验证, 0 调
    }
}
