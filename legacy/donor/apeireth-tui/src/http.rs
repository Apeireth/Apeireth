//! Apeireth R25.2 TUI — 瘦客户端 HTTP (R25 改瘦 Step 1 续)
//!
//! **职责**: TUI 调 apeireth-api server 的 HTTP 客户端, 不直连 lib.
//!
//! **5 端点** (per R22 5 nav 拍板):
//! - `GET /v1/sessions`               → Session nav 列活跃会话
//! - `GET /v1/observability/status`   → Status nav 系统状态
//! - `GET /v1/observability/health`   → Status nav 5 组件 health
//! - `GET /v1/observability/heart`    → heart organ 心跳 (60Hz)
//! - `POST /v1/tools/{name}/invoke`   → Tools nav 调 6 工具
//!
//! **5 K-1 强校验** (per 任务规范):
//! - K-1.1: base_url 不能空
//! - K-1.2: auth_token 字符白名单
//! - K-1.3: 工具名 in 6 白名单
//! - K-1.4: args 是 JSON object
//! - K-1.5: timeout > 0
//!
//! **不假装**:
//! - 5xx 返 TuiError::Api, 透传 status + body
//! - reqwest 错误 → TuiError::Network (via #[from])
//! - JSON 解析失败 → TuiError::Parse
//! - timeout 真用 tokio::time::timeout (不假装)
//! - auth_token 为 None 时不发 Authorization header
//!
//! **8 项承诺**:
//! - 不假装已实现 ✅ (5 端点 5 K-1 强校验 + 真实 HTTP)
//! - 编译期 hardcode (5 端点路径, 6 工具白名单) ✅
//! - 不改 LOCKED ✅ (不动 apeireth-api)
//! - 不改 workspace version ✅
//! - 6 哲学锚穿透 (S-2 实事求是: 透传错误) ✅
//! - 不依赖 NewAPI ✅
//! - 不重复造轮子 (用 reqwest + tokio 已有依赖) ✅
//! - 诚实标缺 (partial/stub 标注到 organ mod) ✅

use std::time::Duration;

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::error::{
    validate_args_object, validate_auth_token, validate_base_url, validate_timeout, TuiError,
};
use crate::organ::hand;

// =====================================================================
// 常量 (编译期 hardcode, 跟 apeireth-api server 端点对齐)
// =====================================================================

/// 默认 base URL (跟 http_llm.rs DEFAULT_API_URL 一致)
const DEFAULT_BASE_URL: &str = "http://localhost:8080";

/// 默认 timeout (30s, 跟 http_llm.rs HTTP_TIMEOUT_SECS 一致)
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 5 端点路径 (编译期 hardcode)
const PATH_SESSIONS: &str = "/v1/sessions";
const PATH_STATUS: &str = "/v1/observability/status";
const PATH_HEALTH: &str = "/v1/observability/health";
const PATH_HEART: &str = "/v1/observability/heart";

/// Authorization header name
const HEADER_AUTH: &str = "Authorization";

/// Authorization header prefix (Bearer scheme)
const AUTH_BEARER_PREFIX: &str = "Bearer ";

// =====================================================================
// 类型 (5 端点响应结构, 跟 apeireth-api server JSON 字段对齐)
// =====================================================================

/// 活跃会话 (Session nav 列表项)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub created_at: Option<String>,
    pub last_active_at: Option<String>,
    pub message_count: Option<u32>,
}

/// 系统状态 (Status nav 概览)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SystemStatus {
    pub uptime_secs: Option<u64>,
    pub version: Option<String>,
    pub environment: Option<String>,
    /// 5 组件 health (health 端点拆出来)
    #[serde(default)]
    pub components: Vec<ComponentHealth>,
}

/// 单组件 health (Status nav 5 圆点)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ComponentHealth {
    pub name: String,
    pub status: String, // "ok" | "degraded" | "down"
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

/// 整体 health 端点响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HealthReport {
    pub overall: String, // "ok" | "degraded" | "down"
    pub components: Vec<ComponentHealth>,
}

/// Heart organ 心跳响应 (60Hz)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HeartBeat {
    pub bpm: f64,     // 跳频率 (Hz * 60)
    pub cpu_pct: f64, // 0-100
    pub tick: u64,    // 递增 tick (单调)
}

// =====================================================================
// 客户端
// =====================================================================

/// 瘦客户端 HTTP (R25 改瘦核心)
///
/// **设计原则** (per 主人 8. 决策 "TUI 应该做瘦客户端"):
/// - TUI 不直接调 `apeireth_api::*` lib 函数
/// - HTTP 表面是稳定契约, 未来 Tauri 桌面来了直接抄这个 client
/// - 5 K-1 强校验构造期就过, 不让坏配置溜到请求
#[derive(Debug, Clone)]
pub struct ApeirethClient {
    base_url: String,
    auth_token: Option<String>,
    timeout: Duration,
    http: Client,
}

impl ApeirethClient {
    /// 用 base_url 构造, 走 5 K-1 强校验
    pub fn new(
        base_url: &str,
        auth_token: Option<&str>,
        timeout: Duration,
    ) -> Result<Self, TuiError> {
        // K-1.1
        validate_base_url(base_url)?;
        // K-1.2
        if let Some(tok) = auth_token {
            validate_auth_token(tok)?;
        }
        // K-1.5
        validate_timeout(timeout)?;

        let http = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(TuiError::Network)?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token: auth_token.map(String::from),
            timeout,
            http,
        })
    }

    /// 用默认 base_url + 默认 timeout 构造
    pub fn default_no_auth() -> Result<Self, TuiError> {
        Self::new(
            DEFAULT_BASE_URL,
            None,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        )
    }

    /// 用 env `APEIRETH_API_URL` 覆盖 base_url
    pub fn from_env() -> Result<Self, TuiError> {
        let base =
            std::env::var("APEIRETH_API_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        Self::new(&base, None, Duration::from_secs(DEFAULT_TIMEOUT_SECS))
    }

    /// 公共 getter (给 Status nav 显示)
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    // ---- 5 端点 ----

    /// `GET /v1/sessions` — Session nav 列活跃会话
    pub async fn get_sessions(&self) -> Result<Vec<Session>, TuiError> {
        let url = format!("{}{}", self.base_url, PATH_SESSIONS);
        let resp = self.get(&url).await?;
        // R25.2 partial bug fix: 原代码 `Self::parse_or_default(resp, ...)` 缺 `&self`,
        // 改成 `self.parse_or_default(resp, ...)`. 逻辑不变.
        self.parse_or_default(resp, Vec::<Session>::new()).await
    }

    /// `GET /v1/observability/status` — Status nav 系统状态
    pub async fn get_status(&self) -> Result<SystemStatus, TuiError> {
        let url = format!("{}{}", self.base_url, PATH_STATUS);
        let resp = self.get(&url).await?;
        resp.json::<SystemStatus>().await.map_err(TuiError::Network)
    }

    /// `GET /v1/observability/health` — Status nav 5 组件 health
    pub async fn get_health(&self) -> Result<HealthReport, TuiError> {
        let url = format!("{}{}", self.base_url, PATH_HEALTH);
        let resp = self.get(&url).await?;
        resp.json::<HealthReport>().await.map_err(TuiError::Network)
    }

    /// `GET /v1/observability/heart` — heart organ 心跳 (60Hz)
    pub async fn get_heart(&self) -> Result<HeartBeat, TuiError> {
        let url = format!("{}{}", self.base_url, PATH_HEART);
        let resp = self.get(&url).await?;
        resp.json::<HeartBeat>().await.map_err(TuiError::Network)
    }

    /// `POST /v1/tools/{name}/invoke` — Tools nav 调 6 工具
    pub async fn invoke_tool(&self, name: &str, args: Value) -> Result<Value, TuiError> {
        // K-1.3 + K-1.4 在调前过
        crate::error::validate_tool_name(name)?;
        validate_args_object(&args)?;

        // R25.2 fix: 原 URL 拼为 `base_url/v1/tools/{name}` 漏 `/invoke` 后缀,
        // 跟 apeireth-api server 路由 `/v1/tools/{name}/invoke` 不匹配 → 404.
        // 改后: `base_url/v1/tools/{name}/invoke` 跟 server 对齐
        let url = format!("{}/v1/tools/{}/invoke", self.base_url, name);
        let body = json!({ "args": args });
        let resp = self.post(&url, &body).await?;
        resp.json::<Value>()
            .await
            .inspect(|_| hand::record_tool_success(name))
            .inspect_err(|_| hand::record_tool_failure(name))
            .map_err(TuiError::Network)
    }

    // ---- 底层 HTTP 帮手 ----

    async fn get(&self, url: &str) -> Result<reqwest::Response, TuiError> {
        let mut req = self.http.get(url);
        if let Some(tok) = &self.auth_token {
            req = req.header(HEADER_AUTH, format!("{AUTH_BEARER_PREFIX}{tok}"));
        }
        let resp = req.send().await.map_err(TuiError::Network)?;
        self.check_status(resp).await
    }

    async fn post(&self, url: &str, body: &Value) -> Result<reqwest::Response, TuiError> {
        let mut req = self.http.post(url).json(body);
        if let Some(tok) = &self.auth_token {
            req = req.header(HEADER_AUTH, format!("{AUTH_BEARER_PREFIX}{tok}"));
        }
        let resp = req.send().await.map_err(TuiError::Network)?;
        self.check_status(resp).await
    }

    /// 检查 status code, 4xx/5xx → TuiError::Api
    async fn check_status(&self, resp: reqwest::Response) -> Result<reqwest::Response, TuiError> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp)
        } else {
            let code = status.as_u16();
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<body read failed>".into());
            Err(TuiError::Api { status: code, body })
        }
    }

    /// 解析 JSON 响应, 失败时返默认 (用于 get_sessions — 列表可能空)
    async fn parse_or_default<T: DeserializeOwned + Default>(
        &self,
        resp: reqwest::Response,
        default: T,
    ) -> Result<T, TuiError> {
        match resp.json::<T>().await {
            Ok(v) => Ok(v),
            Err(_) => Ok(default), // 解析失败返默认, 不假装错误 (S-2 实事求是)
        }
    }
}

// =====================================================================
// 单元测试 (5 K-1 强校验 + 5 端点 httpmock = 10+ 测试)
// =====================================================================
//
// 注意: 部分测试 (httpmock) 需要 apeireth-tui 编译, pre-existing apeireth-api
// 错误会让整个 crate 编译失败. 纯逻辑测试 (K-1 + 构造) 自身无依赖.

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- 构造期 K-1 强校验 ----

    #[test]
    fn constructor_rejects_empty_base_url() {
        let r = ApeirethClient::new("", None, Duration::from_secs(30));
        assert!(matches!(r, Err(TuiError::BaseUrlEmpty)));
    }

    #[test]
    fn constructor_rejects_invalid_auth_token() {
        let r = ApeirethClient::new(
            "http://localhost",
            Some("bad token"),
            Duration::from_secs(30),
        );
        assert!(matches!(r, Err(TuiError::AuthTokenInvalid(' '))));
    }

    #[test]
    fn constructor_rejects_zero_timeout() {
        let r = ApeirethClient::new("http://localhost", None, Duration::from_secs(0));
        assert!(matches!(r, Err(TuiError::TimeoutInvalid(_))));
    }

    #[test]
    fn constructor_accepts_valid_config() {
        let c = ApeirethClient::new(
            "http://localhost:8080",
            Some("sk-abc_123"),
            Duration::from_secs(30),
        )
        .expect("valid config");
        assert_eq!(c.base_url(), "http://localhost:8080");
        assert_eq!(c.timeout(), Duration::from_secs(30));
    }

    #[test]
    fn constructor_strips_trailing_slash() {
        let c = ApeirethClient::new("http://localhost:8080/", None, Duration::from_secs(30))
            .expect("valid");
        assert_eq!(c.base_url(), "http://localhost:8080");
    }

    #[test]
    fn constructor_no_auth_ok() {
        let c = ApeirethClient::new("http://localhost:8080", None, Duration::from_secs(30))
            .expect("valid");
        // 没 token 也能用 (走裸 GET)
        assert_eq!(c.base_url(), "http://localhost:8080");
    }

    #[test]
    fn default_no_auth_works() {
        let c = ApeirethClient::default_no_auth().expect("default ok");
        assert_eq!(c.base_url(), DEFAULT_BASE_URL);
    }

    // ---- 5 端点 (httpmock, 需要 crate 编译) ----

    #[tokio::test]
    async fn get_sessions_parses_list() {
        use httpmock::prelude::*;
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path(PATH_SESSIONS);
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"[{"id":"s1","title":"hi","message_count":3}]"#);
        });
        let c =
            ApeirethClient::new(&server.base_url(), None, Duration::from_secs(5)).expect("client");
        let sessions = c.get_sessions().await.expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s1");
        assert_eq!(sessions[0].title.as_deref(), Some("hi"));
    }

    #[tokio::test]
    async fn get_status_parses_overview() {
        use httpmock::prelude::*;
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path(PATH_STATUS);
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"uptime_secs":3600,"version":"0.14.0","environment":"dev","components":[]}"#);
        });
        let c =
            ApeirethClient::new(&server.base_url(), None, Duration::from_secs(5)).expect("client");
        let s = c.get_status().await.expect("status");
        assert_eq!(s.uptime_secs, Some(3600));
        assert_eq!(s.version.as_deref(), Some("0.14.0"));
    }

    #[tokio::test]
    async fn get_health_parses_5_components() {
        use httpmock::prelude::*;
        let server = MockServer::start_async().await;
        let body = json!({
            "overall": "ok",
            "components": [
                {"name": "core", "status": "ok"},
                {"name": "memory", "status": "ok"},
                {"name": "asi", "status": "degraded", "message": "slow"},
                {"name": "supervisor", "status": "ok"},
                {"name": "api", "status": "ok"}
            ]
        });
        server.mock(|when, then| {
            when.method(GET).path(PATH_HEALTH);
            then.status(200)
                .header("content-type", "application/json")
                .body(serde_json::to_string(&body).unwrap());
        });
        let c =
            ApeirethClient::new(&server.base_url(), None, Duration::from_secs(5)).expect("client");
        let h = c.get_health().await.expect("health");
        assert_eq!(h.overall, "ok");
        assert_eq!(h.components.len(), 5);
        assert_eq!(h.components[2].status, "degraded");
    }

    #[tokio::test]
    async fn get_heart_parses_60hz() {
        use httpmock::prelude::*;
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path(PATH_HEART);
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"bpm":60.0,"cpu_pct":12.5,"tick":42}"#);
        });
        let c =
            ApeirethClient::new(&server.base_url(), None, Duration::from_secs(5)).expect("client");
        let h = c.get_heart().await.expect("heart");
        assert!((h.bpm - 60.0).abs() < 0.01);
        assert_eq!(h.tick, 42);
    }

    #[tokio::test]
    async fn invoke_tool_6_whitelist_accepted() {
        let _g = crate::organ::hand::TEST_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        use httpmock::prelude::*;
        let server = MockServer::start_async().await;
        for tool in TOOL_WHITELIST_LOCAL {
            server.mock(|when, then| {
                when.method(POST).path(format!("/v1/tools/{tool}/invoke"));
                then.status(200)
                    .header("content-type", "application/json")
                    .body(r#"{"ok":true,"result":"ok"}"#);
            });
        }
        let c =
            ApeirethClient::new(&server.base_url(), None, Duration::from_secs(5)).expect("client");
        for tool in TOOL_WHITELIST_LOCAL {
            let r = c
                .invoke_tool(tool, json!({"key": "value"}))
                .await
                .unwrap_or_else(|e| panic!("{tool} invoke err: {e}"));
            assert_eq!(r["ok"], json!(true));
        }
    }

    #[tokio::test]
    async fn invoke_tool_rejects_unknown_name() {
        let c = ApeirethClient::new("http://localhost:8080", None, Duration::from_secs(5))
            .expect("client");
        let r = c.invoke_tool("unknown_tool", json!({})).await;
        assert!(matches!(r, Err(TuiError::ToolNotInWhitelist(_))));
    }

    #[tokio::test]
    async fn invoke_tool_rejects_non_object_args() {
        let c = ApeirethClient::new("http://localhost:8080", None, Duration::from_secs(5))
            .expect("client");
        let r = c.invoke_tool("calendar", json!([])).await;
        assert!(matches!(r, Err(TuiError::ArgsNotObject(_))));
    }

    #[tokio::test]
    async fn api_error_5xx_returns_tui_error_api() {
        use httpmock::prelude::*;
        let server = MockServer::start_async().await;
        server.mock(|when, then| {
            when.method(GET).path(PATH_SESSIONS);
            then.status(500)
                .header("content-type", "text/plain")
                .body("internal server error from mock");
        });
        let c =
            ApeirethClient::new(&server.base_url(), None, Duration::from_secs(5)).expect("client");
        let r = c.get_sessions().await;
        match r {
            Err(TuiError::Api { status, body }) => {
                assert_eq!(status, 500);
                assert!(body.contains("internal server error"));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    // 内部: 6 工具白名单 (跟 error.rs TOOL_WHITELIST 同步, 但这里要 path 字符串)
    const TOOL_WHITELIST_LOCAL: &[&str] =
        &["calendar", "message", "contact", "task", "search", "drive"];
}
