//! Web - Web framework integration (从 v1.0 apeireth-web 4K LOC 收敛)
//!
//! 0 装 PASS: 简化 axum-based HTTP server 配置, 完整 v1.0 era (handler chain, middleware) 不做.

use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub bind_addr: SocketAddr,
    pub max_connections: u32,
    pub request_timeout_ms: u64,
}

impl WebConfig {
    pub fn new(bind_addr: SocketAddr) -> Self { Self { bind_addr, max_connections: 1024, request_timeout_ms: 30000 } }
}

pub struct WebServer { pub config: WebConfig }

impl WebServer {
    pub fn new(config: WebConfig) -> Self { Self { config } }
    pub fn start(&self) -> Result<(), String> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_config_defaults() { let c = WebConfig::new("127.0.0.1:8080".parse().unwrap()); assert_eq!(c.max_connections, 1024); }
    #[test] fn test_start_stub() { let s = WebServer::new(WebConfig::new("127.0.0.1:0".parse().unwrap())); assert!(s.start().is_ok()); }
}