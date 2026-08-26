//! E2E 6 工具 (D-02 子路径 + Auth 5 组件 + 8 工具白名单 + K-1 4 强校验)
//!
//! 覆盖 `apeireth-sdk::client` 6 工具方法 (1:1 翻译 v0.9.21 商业版 tools/* 集成面):
//! - 工具 1: `web_search` (POST /v1/tools/web_search/invoke)
//! - 工具 2a: `file_ops_read` / 2b: `file_ops_write`
//! - 工具 3: `git_ops_status`
//! - 工具 4: `code_exec_run`
//! - 工具 5: `calendar_list` (D-01 stub 返 NotImplemented)
//! - 工具 6: `message_send` (D-01 stub 返 NotImplemented)
//!
//! 阶段 6 stub 守门 (per D-01 + 主人 21:35 拍板"不假装"):
//! - `invoke_tool` 返 `SdkClientError::NotImplemented` (R21 真接)
//! - 真实调用应走 `validate_tool_call` + `auth.preflight` 5 组件, 不发 HTTP
//!
//! 主报告: `reports/r20-stage4-integration-2026-08-05.md §1`

use apeireth_sdk::{
    ApeirethClient, AuthPipeline, ClientConfig, QuotaStub, SdkClientError, SDK_TOOL_WHITELIST,
    SDK_TOOL_WHITELIST_COUNT, TOOL_PATHS, TOOL_WHITELIST, WS_PATH,
    validate_sdk_method, validate_tool_call,
};

/// 10 测试覆盖 E2E 6 工具调用
#[cfg(test)]
mod tests {
    use super::*;

    /// 32 字符测试 token (>= API_KEY_MIN_LENGTH = 16)
    fn make_test_token() -> String {
        "test-token-32chars-12345678abc".to_string()
    }

    #[test]
    fn e2e_client_constructs_with_valid_token() {
        let token = make_test_token();
        let client = ApeirethClient::new("http://localhost:8080", &token)
            .expect("valid token 应构造成功");
        assert_eq!(client.base_url, "http://localhost:8080");
        assert_eq!(client.auth.api_key, token);
    }

    #[test]
    fn e2e_client_rejects_short_token() {
        // API_KEY_MIN_LENGTH = 16, 14 字符应被 check_bearer 拒
        let short = "short-token-12";
        let err = ApeirethClient::new("http://localhost:8080", short)
            .expect_err("短 token 应被拒");
        assert!(matches!(err, SdkClientError::AuthFailed(_)));
    }

    #[test]
    fn e2e_tool_paths_cover_six_invoke_paths() {
        // TOOL_PATHS 6 工具 (per 蓝图 §2.2 D-02 子路径)
        let names: Vec<&str> = TOOL_PATHS.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"web_search"), "web_search 路径应在 TOOL_PATHS");
        assert!(names.contains(&"file_ops"), "file_ops 路径应在 TOOL_PATHS");
        assert!(names.contains(&"git_ops"), "git_ops 路径应在 TOOL_PATHS");
        assert!(names.contains(&"code_exec"), "code_exec 路径应在 TOOL_PATHS");
        assert!(names.contains(&"calendar"), "calendar 路径应在 TOOL_PATHS");
        assert!(names.contains(&"message"), "message 路径应在 TOOL_PATHS");
        // 路径前缀必须 /v1/tools/
        for (_name, path) in TOOL_PATHS {
            assert!(path.starts_with("/v1/tools/"), "D-02 子路径前缀: {path}");
            assert!(path.ends_with("/invoke"), "D-02 子路径后缀: {path}");
        }
    }

    #[test]
    fn e2e_ws_path_compiles_to_v1_stream() {
        // WS_PATH = "/v1/stream" (per D-03 决策 1:1 翻译)
        assert_eq!(WS_PATH, "/v1/stream", "WS 端点路径 1:1 翻译 v0.9.21");
    }

    #[test]
    fn e2e_tool_whitelist_has_six_tools() {
        // TOOL_WHITELIST 6 工具 (K-1 强校验: 跟 6 工具 method 1:1)
        assert_eq!(TOOL_WHITELIST.len(), 6, "TOOL_WHITELIST 应有 6 项");
        assert_eq!(SDK_TOOL_WHITELIST_COUNT, 8, "SDK_TOOL_WHITELIST_COUNT 编译期守门 8");
        assert_eq!(SDK_TOOL_WHITELIST.len(), 8, "SDK_TOOL_WHITELIST 应有 8 项 (6+2)");
    }

    #[test]
    fn e2e_invoke_tool_stub_returns_not_implemented() {
        // D-01 stub 守门: R21 真接前必须返 NotImplemented
        let client = ApeirethClient::new("http://localhost:8080", &make_test_token()).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(async {
            client
                .invoke_tool("web_search", "search", serde_json::json!({"query": "test"}))
                .await
        });
        let err = result.expect_err("阶段 6 stub 应返 Err");
        match err {
            SdkClientError::NotImplemented(msg) => {
                assert!(msg.contains("R21"), "未实现错误应指明 R21 路径");
            }
            other => panic!("应返 NotImplemented, 实际: {other:?}"),
        }
    }

    #[test]
    fn e2e_invoke_tool_rejects_non_whitelisted() {
        // m3 防御: 不在 8 工具白名单的 tool 应被 validate_tool_call 拒
        let client = ApeirethClient::new("http://localhost:8080", &make_test_token()).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(async {
            // "ssh_run_shell" 不在 TOOL_WHITELIST, 应被 m3 防御拒
            client
                .invoke_tool("ssh_run_shell", "exec", serde_json::json!({}))
                .await
        });
        let err = result.expect_err("非白名单工具应被拒");
        match err {
            SdkClientError::ToolNotWhitelisted(tool) => {
                assert_eq!(tool, "ssh_run_shell", "错误应含被拒工具名");
            }
            other => panic!("应返 ToolNotWhitelisted, 实际: {other:?}"),
        }
    }

    #[test]
    fn e2e_validate_sdk_method_accepts_known() {
        // validate_sdk_method 应接受 SDK_TOOL_WHITELIST 列出的方法
        assert!(validate_sdk_method("apeireth_sdk_invoke_tool").is_ok());
        assert!(validate_sdk_method("apeireth_sdk_web_search").is_ok());
        // 不在的方法应拒
        assert!(validate_sdk_method("apeireth_sdk_evil_method").is_err());
    }

    #[test]
    fn e2e_validate_tool_call_direct() {
        // 直接调 validate_tool_call (per m3-hallucination-defense §2.4)
        assert!(validate_tool_call("web_search", &serde_json::json!({})).is_ok());
        assert!(validate_tool_call("evil_tool", &serde_json::json!({})).is_err());
    }

    #[test]
    fn e2e_auth_pipeline_preflight_5_components() {
        // Auth 5 组件 preflight 守门 (Bearer → bucket → audit)
        let pipeline = AuthPipeline::new(&make_test_token()).expect("5 组件应就位");
        let result = pipeline.preflight("web_search", "search");
        assert!(result.is_ok(), "5 组件 preflight 应通过白名单工具");
    }

    #[test]
    fn e2e_quota_stub_check_returns_501_stub() {
        // D-05: 阶段 6 stub 永远返 501 (显式 unimplemented, 不假装支持)
        let quota = QuotaStub::new();
        let result = quota.check();
        let err = result.expect_err("D-05 quota stub 应返 501");
        assert!(matches!(err, SdkClientError::QuotaExceeded(_)));
    }

    #[test]
    fn e2e_client_config_default_30s_http_timeout() {
        let config = ClientConfig::default();
        assert_eq!(config.http_timeout_secs, 30, "HTTP 默认 30s timeout");
        assert!(config.audit_enabled, "默认开 audit log");
    }
}
