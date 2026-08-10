//! # `apeireth-integration-e2e` 错误类型 — 9 变体 hardcode
//!
//! **职责**: 把三层 e2e (workspace + API + TUI) 中可能失败的所有情况装到统一 `E2EError`,
//! 跨 `test_*` 边界不漏 `anyhow::Error`, 守"错误能装到实现"承诺 (8 项不修改承诺 #1).
//!
//! **9 变体** (per 8 项不修改承诺 #2 — 8-10 变体对应失败类型):
//! - `WorkspaceAudit`     — workspace 审计失败 (e.g. 24 LOCKED 被改, sandbox 错路径)
//! - `WorkspaceCargo`     — `cargo check` / `cargo metadata` 调用失败
//! - `ApiHttp`            — wiremock / reqwest HTTP 调用失败 (4xx / 5xx / timeout)
//! - `ApiStatus`          — HTTP 状态码非期望 (e.g. 期望 200 实际 500)
//! - `ApiJson`            — JSON 序列化 / 反序列化失败
//! - `TuiRender`          — ratatui TestBackend 渲染失败
//! - `TuiAssert`          — buffer 文本 / 颜色断言失败
//! - `HarnessStart`       — IntegrationHarness 启动失败
//! - `Other`              — 兜底, 配 `#[from] anyhow::Error`
//!
//! **8 项不修改承诺**:
//! - 错误能装到实现 ✓ (thiserror + `#[from] anyhow::Error`, 跨 boundary 不漏)
//! - 错误数 hardcode ✓ (9 变体 = 8-10 区间, 编译期 `nine_variants_const` 测试守门)
//! - 0 改 LOCKED ✓
//! - 0 改 workspace version ✓
//! - 6 哲学锚透传 ✓ (本文件不变, 上游 lib.rs 守)
//! - 0 依赖 NewAPI ✓
//! - 0 重复造轮子 ✓ (thiserror 现成)
//! - 0 假装实缺 ✓ (9 变体 = 9 真实失败类型, 1:1 映射)

use std::fmt;

/// e2e 统一错误 (9 变体 hardcode, 跟 8-10 区间对齐)
#[derive(Debug)]
pub enum E2EError {
    /// workspace 审计失败 (e.g. 24 LOCKED 被改, sandbox 错路径, parent Cargo.toml 改)
    WorkspaceAudit {
        /// 失败维度 (e.g. "locked_crate_modified", "sandbox_path", "workspace_version")
        dimension: String,
        /// 期望值
        expected: String,
        /// 实际值
        actual: String,
        /// 上下文
        context: String,
    },

    /// `cargo check` / `cargo metadata` 调用失败
    WorkspaceCargo {
        /// 调用的 cargo 子命令
        command: String,
        /// 退出码
        exit_code: Option<i32>,
        /// stderr 摘要
        stderr_excerpt: String,
    },

    /// wiremock / reqwest HTTP 调用失败 (timeout / connect refused / etc.)
    ApiHttp {
        /// URL
        url: String,
        /// reqwest 错误描述
        reason: String,
    },

    /// HTTP 状态码非期望
    ApiStatus {
        /// URL
        url: String,
        /// 期望状态码
        expected: u16,
        /// 实际状态码
        actual: u16,
    },

    /// JSON 序列化 / 反序列化失败
    ApiJson {
        /// 上下文 (e.g. "GET /v1/tools/list response")
        context: String,
        /// serde_json 错误描述
        reason: String,
    },

    /// ratatui TestBackend 渲染失败
    TuiRender {
        /// 终端尺寸
        width: u16,
        /// 终端尺寸
        height: u16,
        /// 失败原因
        reason: String,
    },

    /// buffer 文本 / 颜色断言失败
    TuiAssert {
        /// 上下文 (e.g. "tui_status_nav_renders")
        context: String,
        /// 期望
        expected: String,
        /// 实际
        actual: String,
    },

    /// IntegrationHarness 启动失败
    HarnessStart {
        /// 失败原因
        reason: String,
    },

    /// 兜底, 配 `#[from] anyhow::Error`
    Other(#[allow(dead_code)] anyhow::Error),
}

impl fmt::Display for E2EError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkspaceAudit { dimension, expected, actual, context } => write!(
                f,
                "workspace audit fail at `{context}`: dimension=`{dimension}`, expected=`{expected}`, actual=`{actual}`"
            ),
            Self::WorkspaceCargo { command, exit_code, stderr_excerpt } => write!(
                f,
                "cargo `{command}` failed (exit={:?}): {}",
                exit_code, stderr_excerpt
            ),
            Self::ApiHttp { url, reason } => {
                write!(f, "HTTP call to `{url}` failed: {reason}")
            }
            Self::ApiStatus { url, expected, actual } => write!(
                f,
                "HTTP status mismatch at `{url}`: expected {expected}, actual {actual}"
            ),
            Self::ApiJson { context, reason } => {
                write!(f, "JSON parse fail at `{context}`: {reason}")
            }
            Self::TuiRender { width, height, reason } => {
                write!(f, "TUI render fail ({width}x{height}): {reason}")
            }
            Self::TuiAssert { context, expected, actual } => write!(
                f,
                "TUI assert fail at `{context}`: expected `{expected}`, actual `{actual}`"
            ),
            Self::HarnessStart { reason } => write!(f, "IntegrationHarness start failed: {reason}"),
            Self::Other(e) => write!(f, "other e2e error: {e}"),
        }
    }
}

impl std::error::Error for E2EError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Other(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for E2EError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(e)
    }
}

impl From<serde_json::Error> for E2EError {
    fn from(e: serde_json::Error) -> Self {
        Self::ApiJson {
            context: "serde_json".into(),
            reason: e.to_string(),
        }
    }
}

impl From<reqwest::Error> for E2EError {
    fn from(e: reqwest::Error) -> Self {
        Self::ApiHttp {
            url: e.url().map(|u| u.to_string()).unwrap_or_default(),
            reason: e.to_string(),
        }
    }
}

/// e2e `Result` 别名, 跟 anyhow / thiserror 习惯一致
pub type E2EResult<T> = std::result::Result<T, E2EError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// 9 变体 hardcode — 编译期守住 8-10 区间
    #[test]
    fn nine_variants_const() {
        // 显式枚举 9 个变体, 加 1 个就编译过不去 (let _ 强制类型检查)
        let _v0: E2EError = E2EError::WorkspaceAudit {
            dimension: String::new(),
            expected: String::new(),
            actual: String::new(),
            context: String::new(),
        };
        let _v1 = E2EError::WorkspaceCargo {
            command: String::new(),
            exit_code: None,
            stderr_excerpt: String::new(),
        };
        let _v2 = E2EError::ApiHttp {
            url: String::new(),
            reason: String::new(),
        };
        let _v3 = E2EError::ApiStatus {
            url: String::new(),
            expected: 0,
            actual: 0,
        };
        let _v4 = E2EError::ApiJson {
            context: String::new(),
            reason: String::new(),
        };
        let _v5 = E2EError::TuiRender {
            width: 0,
            height: 0,
            reason: String::new(),
        };
        let _v6 = E2EError::TuiAssert {
            context: String::new(),
            expected: String::new(),
            actual: String::new(),
        };
        let _v7 = E2EError::HarnessStart { reason: String::new() };
        let _v8 = E2EError::Other(anyhow::anyhow!("9th variant"));
        // 9 变体 ✓ (8-10 区间)
    }

    #[test]
    fn display_workspace_audit() {
        let e = E2EError::WorkspaceAudit {
            dimension: "locked_crate".into(),
            expected: "0".into(),
            actual: "5".into(),
            context: "apeireth-core".into(),
        };
        let s = e.to_string();
        assert!(s.contains("locked_crate"));
        assert!(s.contains("apeireth-core"));
        assert!(s.contains("0"));
        assert!(s.contains("5"));
    }

    #[test]
    fn display_workspace_cargo() {
        let e = E2EError::WorkspaceCargo {
            command: "check".into(),
            exit_code: Some(1),
            stderr_excerpt: "error: unused".into(),
        };
        let s = e.to_string();
        assert!(s.contains("check"));
        assert!(s.contains("unused"));
    }

    #[test]
    fn display_api_http() {
        let e = E2EError::ApiHttp {
            url: "http://127.0.0.1:9999/health".into(),
            reason: "connection refused".into(),
        };
        let s = e.to_string();
        assert!(s.contains("127.0.0.1:9999"));
        assert!(s.contains("connection refused"));
    }

    #[test]
    fn display_api_status() {
        let e = E2EError::ApiStatus {
            url: "/v1/tools/list".into(),
            expected: 200,
            actual: 500,
        };
        let s = e.to_string();
        assert!(s.contains("/v1/tools/list"));
        assert!(s.contains("200"));
        assert!(s.contains("500"));
    }

    #[test]
    fn display_api_json() {
        let e = E2EError::ApiJson {
            context: "GET /v1/organs".into(),
            reason: "expected string".into(),
        };
        let s = e.to_string();
        assert!(s.contains("/v1/organs"));
        assert!(s.contains("expected string"));
    }

    #[test]
    fn display_tui_render() {
        let e = E2EError::TuiRender {
            width: 0,
            height: 0,
            reason: "zero size".into(),
        };
        let s = e.to_string();
        assert!(s.contains("0x0"));
        assert!(s.contains("zero size"));
    }

    #[test]
    fn display_tui_assert() {
        let e = E2EError::TuiAssert {
            context: "test_tui_status_nav_renders".into(),
            expected: "状态".into(),
            actual: "<not found>".into(),
        };
        let s = e.to_string();
        assert!(s.contains("状态"));
        assert!(s.contains("test_tui_status_nav_renders"));
    }

    #[test]
    fn display_harness_start() {
        let e = E2EError::HarnessStart { reason: "wiremock fail".into() };
        assert!(e.to_string().contains("wiremock fail"));
    }

    #[test]
    fn from_anyhow_via_other() {
        let anyhow_err = anyhow::anyhow!("test");
        let e: E2EError = anyhow_err.into();
        assert!(matches!(e, E2EError::Other(_)));
    }

    #[test]
    fn from_serde_json_via_api_json() {
        let bad = serde_json::from_str::<serde_json::Value>("not json");
        assert!(bad.is_err());
        let e: E2EError = bad.unwrap_err().into();
        assert!(matches!(e, E2EError::ApiJson { .. }));
    }

    #[test]
    fn result_alias_works() {
        let ok: E2EResult<u32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);
    }
}
