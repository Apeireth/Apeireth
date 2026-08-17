//! `HttpClient` — `apeireth-http-client` 的主体结构
//!
//! **职责**:
//! 1. 包装 `reqwest::Client`, 把 VCP 5 字段 (复刻 `chatCompletionHandler.js:22-28`) 真传到 `ClientBuilder`
//! 2. 提供 `post` / `get` 方法
//! 3. 内置 `LifoPool` 做请求调度 + `max_sockets` 限流
//!
//! **字段级传递** (VCP → reqwest):
//! | VCP 字段 | reqwest 0.12 API | 说明 |
//! |---------|------------------|------|
//! | `keepAlive: true` | `ClientBuilder::tcp_keepalive()` (启用) | TCP 层 keep-alive |
//! | `keepAliveMsecs: 1000` | `tcp_keepalive(Duration::from_millis(1000))` | TCP 探针间隔 |
//! | `freeSocketTimeout: 8000` | `pool_idle_timeout(Duration::from_millis(8000))` | **绝杀 zombie**: 8s 空闲销毁 |
//! | `scheduling: 'lifo'` | 我们的 `LifoPool` (LIFO 调度) | 请求调度顺序 |
//! | `maxSockets: 10000` | `pool_max_idle_per_host(10000)` + `Semaphore::new(10000)` | 每 host 空闲池 + 并发上限 |
//!
//! **不假装**: 5 字段全部真的进 reqwest::Client 配置, 任何一项变更都改 builder 调用.

use std::time::Duration;

use serde::{de::DeserializeOwned, Serialize};

use crate::config::KeepAliveConfig;
use crate::error::{HttpClientError, Result};
use crate::lifo_pool::LifoPool;

/// HTTP 响应包装 — 跟 reqwest::Response 行为一致, 但带 client handle
pub struct Response {
    /// 原始 reqwest 响应
    inner: reqwest::Response,
    /// 请求耗时 (从 post/get 进入到收到 response 的时间)
    pub elapsed: Duration,
    /// 状态码
    pub status: reqwest::StatusCode,
    /// 响应 URL (可能被重定向)
    pub url: String,
}

impl Response {
    /// 读取 body 为字符串
    pub async fn text(self) -> Result<String> {
        self.inner
            .text()
            .await
            .map_err(|e| HttpClientError::Request(e.to_string()))
    }

    /// 读取 body 并反序列化为 JSON
    pub async fn json<T: DeserializeOwned>(self) -> Result<T> {
        self.inner
            .json::<T>()
            .await
            .map_err(|e| HttpClientError::Request(e.to_string()))
    }

    /// 获取状态码
    pub fn status(&self) -> reqwest::StatusCode {
        self.status
    }

    /// 获取响应 URL
    pub fn url(&self) -> &str {
        &self.url
    }

    /// 获取耗时
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed.as_millis() as u64
    }

    /// 获取 content-type header (e.g. "text/html; charset=utf-8")
    ///
    /// R174: apeireth-tool-fetch 的 HttpFetcher 用它判断要不要走 HTML extract_text 路径.
    pub fn content_type(&self) -> String {
        self.inner
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string()
    }
}

/// HTTP 客户端主体 — 复刻 VCP `agentOptions` 5 字段
///
/// `Clone` 是 cheap 的 (`reqwest::Client` 内部 `Arc`, `LifoPool` 内部 `Arc`)
#[derive(Clone)]
pub struct HttpClient {
    /// 内部 reqwest 客户端 (5 字段已 baked in)
    inner: reqwest::Client,
    /// 5 字段配置 (测试 / 日志用)
    config: KeepAliveConfig,
    /// LIFO/FIFO 请求调度池 + `max_sockets` 限流
    pool: LifoPool,
    /// S4 出站网络策略 (None = 未接, 0 装: 不检查; Some = 每次出站过白名单 + 审计链).
    egress: Option<std::sync::Arc<std::sync::Mutex<crate::egress::EgressPolicy>>>,
}

impl HttpClient {
    /// 构造 HTTP 客户端, 用 VCP 5 字段配置
    ///
    /// # 实现细节
    /// - `keep_alive = true` + `keep_alive_msecs = 1000` → `tcp_keepalive(Duration::from_millis(1000))`
    /// - `free_socket_timeout = 8000` → `pool_idle_timeout(Duration::from_millis(8000))`
    ///   **绝杀机制**: 8s 空闲 socket 主动销毁, 防止复用到被上游代理静默杀死的僵尸连接
    /// - `max_sockets = 10000` → `pool_max_idle_per_host(10000)` + `Semaphore::new(10000)`
    /// - `scheduling = lifo` → `LifoPool::new(10000, SchedulingPolicy::Lifo)`
    pub fn new(config: KeepAliveConfig) -> Result<Self> {
        // 1. 字段级校验 (编译期挡不住的运行时兜底)
        config.validate().map_err(HttpClientError::InvalidConfig)?;

        // 2. 构造 reqwest::Client, 5 字段真传
        let mut builder = reqwest::Client::builder();

        if config.keep_alive {
            // **关键**: VCP `keepAliveMsecs: 1000` 映射到 TCP keep-alive 间隔
            // reqwest 没有独立的 keep_alive_msecs, 用 tcp_keepalive 实现
            builder = builder.tcp_keepalive(Duration::from_millis(config.keep_alive_msecs));
        }

        // **关键**: VCP `freeSocketTimeout: 8000` 映射到 reqwest pool_idle_timeout
        // 这是绝杀 zombie socket 的核心机制
        builder =
            builder.pool_idle_timeout(Some(Duration::from_millis(config.free_socket_timeout)));

        // **关键**: VCP `maxSockets: 10000` 映射到 per-host 空闲池大小
        // 注: reqwest 的 `pool_max_idle_per_host` 是每 host 空闲连接数,
        // 实际并发连接数还受 OS 文件描述符 / keep-alive 限制.
        // 我们的 `LifoPool` 额外加 `Semaphore` 限流总并发.
        builder = builder.pool_max_idle_per_host(config.max_sockets);

        // 默认超时: 30s (跟 reqwest 默认对齐)
        builder = builder.timeout(Duration::from_secs(30));

        let inner = builder
            .build()
            .map_err(|e| HttpClientError::ClientBuild(e.to_string()))?;

        // 3. 构造 LIFO 池
        let pool = LifoPool::new(config.max_sockets, config.scheduling);

        Ok(Self {
            inner,
            config,
            pool,
            egress: None,
        })
    }

    /// VCP 默认配置快速构造
    pub fn with_chat_defaults() -> Result<Self> {
        Self::new(KeepAliveConfig::chat_default())
    }

    /// 获取配置 (测试用)
    pub fn config(&self) -> KeepAliveConfig {
        self.config
    }

    /// 获取 LIFO 池 (测试用, 验证调度顺序)
    pub fn pool(&self) -> &LifoPool {
        &self.pool
    }

    /// 接入 S4 出站网络策略 (None = 不检查; 调用方显式接入才启用 — 0 装 PASS).
    pub fn with_egress(mut self, policy: std::sync::Arc<std::sync::Mutex<crate::egress::EgressPolicy>>) -> Self {
        self.egress = Some(policy);
        self
    }

    /// 出站检查: 每次请求前过 egress 白名单 (默认拒绝) + 审计链.
    /// 未接策略 → 放行 (0 装: 不假装已检查).
    fn check_egress(&self, url: &str) -> Result<()> {
        if let Some(p) = &self.egress {
            p.lock()
                .map_err(|_| HttpClientError::Other("egress mutex poisoned".into()))?
                .check_outbound(url, 1.0)
                .map_err(|e| HttpClientError::Other(format!("出站策略拒绝: {e:?}")))?;
        }
        Ok(())
    }

    /// POST JSON 请求
    ///
    /// 走 LIFO 池调度 + `max_sockets` 限流
    pub async fn post<B: Serialize>(&self, url: &str, body: &B) -> Result<Response> {
        self.check_egress(url)?;
        let start = std::time::Instant::now();

        // 1. 拿 LIFO 池 permit (max_sockets 限流, 满了会等)
        let _guard = self.pool.enter().await;

        // 2. 真正发请求
        let resp = self.inner.post(url).json(body).send().await?;

        let elapsed = start.elapsed();
        let status = resp.status();
        let url_final = resp.url().to_string();

        Ok(Response {
            inner: resp,
            elapsed,
            status,
            url: url_final,
        })
    }

    /// POST JSON (typed body — `serde_json::Value` 简写)
    pub async fn post_json(&self, url: &str, body: serde_json::Value) -> Result<Response> {
        self.post(url, &body).await
    }

    /// PUT JSON (per R150 P1 #6 Qdrant compat — Qdrant uses PUT for collection create / upsert points)
    pub async fn put<B: Serialize>(&self, url: &str, body: &B) -> Result<Response> {
        let start = std::time::Instant::now();
        let _guard = self.pool.enter().await;
        let resp = self.inner.put(url).json(body).send().await?;
        let elapsed = start.elapsed();
        let status = resp.status();
        let url_final = resp.url().to_string();
        Ok(Response {
            inner: resp,
            elapsed,
            status,
            url: url_final,
        })
    }

    /// PUT JSON (typed body — `serde_json::Value` 简写)
    pub async fn put_json(&self, url: &str, body: serde_json::Value) -> Result<Response> {
        self.put(url, &body).await
    }

    /// DELETE 请求 (per R150 P1 #6 Qdrant compat — Qdrant uses DELETE for point delete)
    pub async fn delete(&self, url: &str) -> Result<Response> {
        self.check_egress(url)?;
        let start = std::time::Instant::now();
        let _guard = self.pool.enter().await;
        let resp = self.inner.delete(url).send().await?;
        let elapsed = start.elapsed();
        let status = resp.status();
        let url_final = resp.url().to_string();
        Ok(Response {
            inner: resp,
            elapsed,
            status,
            url: url_final,
        })
    }

    /// GET 请求
    pub async fn get(&self, url: &str) -> Result<Response> {
        self.check_egress(url)?;
        let start = std::time::Instant::now();
        let _guard = self.pool.enter().await;
        let resp = self.inner.get(url).send().await?;
        let elapsed = start.elapsed();
        let status = resp.status();
        let url_final = resp.url().to_string();
        Ok(Response {
            inner: resp,
            elapsed,
            status,
            url: url_final,
        })
    }

    /// **底层**: 获取 reqwest::Client (高级用户用, e.g. 流式)
    pub fn reqwest_client(&self) -> &reqwest::Client {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SchedulingPolicy;

    #[test]
    fn new_with_chat_default_succeeds() {
        // VCP 5 字段默认配置, 应该能成功构造
        let client = HttpClient::with_chat_defaults().expect("VCP default must build");
        assert_eq!(client.config(), KeepAliveConfig::chat_default());
        assert_eq!(client.config().scheduling, SchedulingPolicy::Lifo);
        assert_eq!(client.config().max_sockets, 10_000);
    }

    #[test]
    fn new_rejects_zero_max_sockets() {
        // 字段级校验, max_sockets=0 必失败
        let cfg = KeepAliveConfig {
            max_sockets: 0,
            ..KeepAliveConfig::chat_default()
        };
        let result = HttpClient::new(cfg);
        assert!(matches!(result, Err(HttpClientError::InvalidConfig(_))));
    }

    #[test]
    fn new_rejects_zero_keep_alive_msecs() {
        let cfg = KeepAliveConfig {
            keep_alive_msecs: 0,
            ..KeepAliveConfig::chat_default()
        };
        let result = HttpClient::new(cfg);
        assert!(matches!(result, Err(HttpClientError::InvalidConfig(_))));
    }

    #[test]
    fn new_rejects_zero_free_socket_timeout() {
        let cfg = KeepAliveConfig {
            free_socket_timeout: 0,
            ..KeepAliveConfig::chat_default()
        };
        let result = HttpClient::new(cfg);
        assert!(matches!(result, Err(HttpClientError::InvalidConfig(_))));
    }

    #[test]
    fn clone_shares_underlying_client() {
        // Clone 必须共享 reqwest::Client (reqwest::Client 本身就是 Arc)
        let client = HttpClient::with_chat_defaults().unwrap();
        let client2 = client.clone();
        // 两个 client 用同一个底层连接池 (LIFO 池也共享)
        let _g0 = client.pool().try_enter().unwrap();
        let _g1 = client2.pool().try_enter().unwrap();
        assert_eq!(client.pool().queue_len(), 2);
        assert_eq!(client2.pool().queue_len(), 2);
    }

    #[test]
    fn fifo_config_uses_fifo_pool() {
        // scheduling='fifo' 真的进 LifoPool 构造
        let cfg = KeepAliveConfig {
            scheduling: SchedulingPolicy::Fifo,
            ..KeepAliveConfig::chat_default()
        };
        let client = HttpClient::new(cfg).unwrap();
        assert_eq!(client.pool().scheduling(), SchedulingPolicy::Fifo);
    }

    #[test]
    fn custom_max_sockets_propagates_to_pool() {
        // max_sockets 自定义值 (e.g. 5) 真的进 Semaphore
        let cfg = KeepAliveConfig {
            max_sockets: 5,
            ..KeepAliveConfig::chat_default()
        };
        let client = HttpClient::new(cfg).unwrap();
        // 拿满 5 个 permit, 第 6 个必失败
        let mut guards = Vec::new();
        for _ in 0..5 {
            guards.push(client.pool().try_enter().unwrap());
        }
        assert!(client.pool().try_enter().is_err());
    }

    #[test]
    fn post_to_invalid_url_returns_request_error() {
        // 网络错误 → Request 变体
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = HttpClient::with_chat_defaults().unwrap();
        rt.block_on(async {
            // 用一个不可达的 URL (端口 1 是保留的, 必失败)
            let result = client
                .post_json("http://127.0.0.1:1/never", serde_json::json!({}))
                .await;
            assert!(matches!(result, Err(HttpClientError::Request(_))));
        });
    }

    #[test]
    fn get_to_invalid_url_returns_request_error() {
        // 同上, 但用 GET
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = HttpClient::with_chat_defaults().unwrap();
        rt.block_on(async {
            let result = client.get("http://127.0.0.1:1/never").await;
            assert!(matches!(result, Err(HttpClientError::Request(_))));
        });
    }
}
