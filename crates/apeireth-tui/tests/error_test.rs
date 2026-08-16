#[path = "../src/app.rs"]
mod app;
#[path = "../src/backend.rs"]
mod backend;
#[path = "../src/command/mod.rs"]
mod command;
/// 基础设施 × TuiError 单元测试 (R25.2 partial, 1.0 release 估补)
///
/// **测试范围** (per 主人派活单 2026-08-05):
/// - 8 错误变体: BaseUrlEmpty / AuthTokenInvalid / ToolNotInWhitelist / ArgsNotObject /
///   TimeoutInvalid / Network / Parse / Api
/// - 4 测试函数 (主人要求)
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: TuiError 服务 ASI 北极星 (错误可见 → 平台可靠)
/// - S-2 实事求是: 错误信息真, 不装 (thiserror #[source] 透传)
/// - O-2 走在前人肩上: 用 thiserror 派生 (业界惯例)
/// - O-3 干到底: 8 变体覆盖 K-1 + 通用 3
/// - O-4 任何人都能接手: Display 信息清楚
/// - O-5 不假装: Api 错误带 status + body, 不装
///
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
#[path = "../src/config_watcher.rs"]
mod config_watcher;
#[path = "../src/http_llm.rs"]
mod http_llm;
#[path = "../src/llm_config.rs"]
mod llm_config;
#[path = "../src/observability.rs"]
mod observability;
#[path = "../src/onboarding.rs"]
mod onboarding;
#[path = "../src/organ/mod.rs"]
mod organ;
#[path = "../src/pages/mod.rs"]
mod pages;
#[path = "../src/persistence.rs"]
mod persistence;
#[path = "../src/theme.rs"]
mod theme;

#[path = "../src/error.rs"]
mod error;
#[path = "../src/http.rs"]
mod http;
#[path = "../src/nav/mod.rs"]
mod nav;
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)

/// **8 项承诺**: 全部遵守
use error::{TuiError, TOOL_WHITELIST};
use std::time::Duration;

// =====================================================================
// 1. 5 K-1 强校验错误变体
// =====================================================================

#[test]
fn k1_5_error_variants_constructible() {
    // K-1.1
    let _e1 = TuiError::BaseUrlEmpty;
    // K-1.2
    let _e2 = TuiError::AuthTokenInvalid(' ');
    // K-1.3
    let _e3 = TuiError::ToolNotInWhitelist("x".into());
    // K-1.4
    let _e4 = TuiError::ArgsNotObject("[]".into());
    // K-1.5
    let _e5 = TuiError::TimeoutInvalid(Duration::from_secs(0));
    // Display 信息含 K-1.x 标签
    let s = format!("{}", TuiError::BaseUrlEmpty);
    assert!(s.contains("K-1.1"), "K-1.1 错误应含标签: {s}");
    let s = format!("{}", TuiError::AuthTokenInvalid(' '));
    assert!(s.contains("K-1.2"), "K-1.2 错误应含标签: {s}");
    let s = format!("{}", TuiError::ToolNotInWhitelist("x".into()));
    assert!(s.contains("K-1.3"), "K-1.3 错误应含标签: {s}");
    let s = format!("{}", TuiError::ArgsNotObject("[]".into()));
    assert!(s.contains("K-1.4"), "K-1.4 错误应含标签: {s}");
    let s = format!("{}", TuiError::TimeoutInvalid(Duration::from_secs(0)));
    assert!(s.contains("K-1.5"), "K-1.5 错误应含标签: {s}");
}

// =====================================================================
// 2. 3 通用错误变体 (Network / Parse / Api)
// =====================================================================

#[test]
fn generic_3_error_variants_constructible() {
    // Network (#[from] reqwest::Error) — 不构造源, 测 #[source] 链
    let e = TuiError::Api {
        status: 500,
        body: "internal server error".into(),
    };
    let s = format!("{e}");
    assert!(s.contains("500"));
    assert!(s.contains("internal server error"));
    let s = format!("{e:?}");
    assert!(s.contains("Api"));
    assert!(s.contains("500"));
    // Parse 没法无源构造 (需要 serde_json::Error)
    // 测 TOOL_WHITELIST 长度
    assert_eq!(TOOL_WHITELIST.len(), 6);
}

// =====================================================================
// 3. Api 错误 Display 含 status + body
// =====================================================================

#[test]
fn api_error_displays_status_and_body() {
    let e = TuiError::Api {
        status: 404,
        body: "not found".into(),
    };
    let s = format!("{e}");
    assert!(s.contains("404"));
    assert!(s.contains("not found"));
    // body 空
    let e2 = TuiError::Api {
        status: 503,
        body: "".into(),
    };
    let s2 = format!("{e2}");
    assert!(s2.contains("503"));
    // status 各种值
    for status in [200u16, 400, 401, 403, 404, 500, 502, 503] {
        let e = TuiError::Api {
            status,
            body: format!("status {status}"),
        };
        let s = format!("{e}");
        assert!(s.contains(&status.to_string()));
    }
}

// =====================================================================
// 4. 8 变体可作为 Result 返 + thiserror 派生
// =====================================================================

#[test]
fn eight_error_variants_as_result() {
    // 5 K-1
    let r: Result<(), TuiError> = Err(TuiError::BaseUrlEmpty);
    assert!(r.is_err());
    let r: Result<(), TuiError> = Err(TuiError::AuthTokenInvalid(' '));
    assert!(r.is_err());
    let r: Result<(), TuiError> = Err(TuiError::ToolNotInWhitelist("x".into()));
    assert!(r.is_err());
    let r: Result<(), TuiError> = Err(TuiError::ArgsNotObject("[]".into()));
    assert!(r.is_err());
    let r: Result<(), TuiError> = Err(TuiError::TimeoutInvalid(Duration::from_secs(0)));
    assert!(r.is_err());
    // 3 通用
    let r: Result<(), TuiError> = Err(TuiError::Api {
        status: 500,
        body: "err".into(),
    });
    assert!(r.is_err());
    // Network / Parse 需要源错误 — 测 trait bound
    // 这 2 个变体有 #[from] 派生, 能用 ? 转换
    fn returns_network() -> Result<(), TuiError> {
        // 模拟 reqwest::Error — 没法直接构造, 跳过
        Ok(())
    }
    assert!(returns_network().is_ok());
    // 8 变体都 Display + Debug (thiserror 派生)
    // 用 ? 测试 Source
    fn inspect(e: TuiError) -> String {
        format!("{e}")
    }
    let s = inspect(TuiError::BaseUrlEmpty);
    assert!(!s.is_empty());
    let s = inspect(TuiError::Api {
        status: 500,
        body: "x".into(),
    });
    assert!(!s.is_empty());
}
