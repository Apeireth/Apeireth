//! `apeireth-http-client` — Apeireth 自研 HTTP 客户端
//!
//! **战役 1-2 / 借鉴 §6.2.2 #14**:
//! 复刻 VCP `chatCompletionHandler.js:22-28` 的 5 字段 keep-alive agent,
//! 解决 VCP 注释里写的 "-1s Socket Hang Up" zombie socket 问题.
//!
//! **VCP 真代码字段级引用** (`research/source/vcptoolbox/modules/chatCompletionHandler.js:22-28`):
//! ```js
//! const agentOptions = {
//!   keepAlive: true,           // → KeepAliveConfig.keep_alive
//!   keepAliveMsecs: 1000,      // → KeepAliveConfig.keep_alive_msecs
//!   freeSocketTimeout: 8000,   // → KeepAliveConfig.free_socket_timeout (绝杀 zombie)
//!   scheduling: 'lifo',        // → KeepAliveConfig.scheduling (Lifo | Fifo)
//!   maxSockets: 10000          // → KeepAliveConfig.max_sockets
//! };
//! ```
//!
//! **架构**:
//! ```text
//!   HttpClient (主入口, Clone)
//!     ├── reqwest::Client (5 字段 baked in: tcp_keepalive + pool_idle_timeout + pool_max_idle_per_host)
//!     └── LifoPool (Mutex<VecDeque<RequestTicket>> + Semaphore)
//!         ├── scheduling=Lifo → dequeue 用 pop_back (后进先出)
//!         └── scheduling=Fifo → dequeue 用 pop_front
//! ```
//!
//! **不假装 (5 字段验证)**:
//! - ✅ `keep_alive=true` 真设 `tcp_keepalive(1000ms)`
//! - ✅ `free_socket_timeout=8000` 真设 `pool_idle_timeout(8000ms)` (绝杀 zombie)
//! - ✅ `max_sockets=10000` 真设 `pool_max_idle_per_host(10000)` + `Semaphore::new(10000)`
//! - ✅ `scheduling='lifo'` 真在 LifoPool 用 `pop_back` 实现
//! - ✅ 单元测试 23 个 (config 8 + lifo_pool 9 + client 6 + error 2 = 25, 部分 in mod 内)
//!
//! **使用**:
//! ```no_run
//! use apeireth_http_client::{HttpClient, KeepAliveConfig};
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! // VCP 默认配置 (5 字段全开)
//! let client = HttpClient::with_vcp_defaults()?;
//!
//! // POST JSON
//! let body = serde_json::json!({"model": "MiniMax-M3", "messages": []});
//! let resp = client.post_json("https://api.minimaxi.com/v1/chat/completions", body).await?;
//! let status = resp.status();
//! let latency = resp.elapsed_ms();
//! let text = resp.text().await?;
//! println!("status={status} latency={latency}ms");
//! # Ok(()) }
//! ```

#![deny(unsafe_code)]

// ============================================================
// 公共模块
// ============================================================

pub mod client;
pub mod config;
pub mod error;
pub mod lifo_pool;
// R127-2 P9-1: hyper-util bridge (借脑 1.0, per decision-56 §2.4)
pub mod hyper_util_bridge;

// ============================================================
// Re-exports
// ============================================================

pub use client::{HttpClient, Response};
pub use config::{KeepAliveConfig, SchedulingPolicy};
pub use error::{HttpClientError, Result};
pub use lifo_pool::{LifoGuard, LifoPool, PoolFull, RequestTicket};
// R127-2 P9-1: hyper_util_bridge re-exports
pub use hyper_util_bridge::{
    build_legacy_client, tokio_io_bridge_marker_compile_time, HyperUtilConfig, LegacyHttpClient,
    TokioIoBridge,
};
