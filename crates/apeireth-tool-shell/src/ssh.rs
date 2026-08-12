//! SSH client wrapper (russh 0.44).
//!
//! Pure Rust SSH2 implementation supporting:
//! - keep-alive
//! - known_hosts verification
//! - connection pool (placeholder; real pool needs runtime)
//!
//! **Honest** (per O-5 不假装):
//! - This module defines the trait + client struct + a connect() stub.
//! - Real connection establishment requires runtime tokio integration with
//!   russh::client::connect — kept off the default build path so the crate
//!   compiles without a live SSH target.

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SshError {
    #[error("connection error: `{0}`")]
    Connection(String),
    #[error("auth error: `{0}`")]
    Auth(String),
    #[error("exec error: `{0}`")]
    Exec(String),
}

#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub auth: SshAuth,
    pub known_hosts: Option<PathBuf>,
    pub keep_alive: Option<Duration>,
}

#[derive(Debug, Clone)]
pub enum SshAuth {
    /// Password authentication
    Password(String),
    /// Public key authentication (path to private key)
    PublicKey(PathBuf),
    /// Agent-based authentication
    Agent,
}

pub struct SshClient {
    config: SshConfig,
}

impl SshClient {
    pub fn new(config: SshConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &SshConfig {
        &self.config
    }

    /// Establish connection. This is a stub — real connect requires
    /// `russh::client::connect(...)` async runtime which is not exercised
    /// here so the crate compiles without a live SSH server.
    pub async fn connect(&self) -> Result<SshSession, SshError> {
        Err(SshError::Connection(
            "russh connect not yet implemented in R138 stub (real impl deferred to R139+ when network target available)".to_string(),
        ))
    }
}

/// Owned SSH session handle. Future work: wraps russh::client::Handle.
pub struct SshSession {
    host: String,
}

impl SshSession {
    pub fn host(&self) -> &str {
        &self.host
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn client_construction() {
        let c = SshClient::new(SshConfig {
            host: "localhost".to_string(),
            port: 22,
            user: "test".to_string(),
            auth: SshAuth::Agent,
            known_hosts: None,
            keep_alive: Some(Duration::from_secs(30)),
        });
        assert_eq!(c.config().host, "localhost");
        assert_eq!(c.config().port, 22);
    }

    #[tokio::test]
    async fn connect_returns_stub_error() {
        let c = SshClient::new(SshConfig {
            host: "127.0.0.1".to_string(),
            port: 22,
            user: "x".to_string(),
            auth: SshAuth::Agent,
            known_hosts: None,
            keep_alive: None,
        });
        let r = c.connect().await;
        assert!(r.is_err());
    }
}
