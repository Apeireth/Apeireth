//! # apeireth-mcp-ssh
//!
//! SSH MCP Server (1:1 翻译 v0.9.21 商业版 `out/main/mcp/SSHMcpServer.js` ~448KB).
//! 商业版 8 大闭源 MCP Server 之一 (per `commercial-vs-fork-diff §2.9` 估时 8h, ~6000 LOC 1:1 翻译).
//! R20 阶段 1 P0 必补.
//!
//! ## v0.9.21 SSHMcpServer.js 实查 (obfuscated webpack bundle, 448KB, 单行)
//!
//! | Token | 命中 | 推断 | Token | 命中 | 推断 |
//! |-------|-----:|------|-------|-----:|------|
//! | `auth` | 46 | 核心 4 变体 union | `Handshak` | 20 | SSH handshake state |
//! | `username` | 27 | auth 字段 | `connect` | 23 | ssh_connect 工具 |
//! | `exec` | 9 | ssh_exec 工具 | `sftp` | 5 | list/upload/download |
//! | `hostKey` | 5 | host key 验证 | `privateKey` | 6 | publickey 认证 |
//! | `Password` | 2 | password 认证 | `PublicKey` | 1 | publickey 变体名 |
//! | `keepalive` | 4 | ssh_keepalive 工具 | `Agent` | 29 | **Node.js EventEmitter Symbol, 非 ssh-agent (O-5 不假装)** |
//! | `JumpHost` / `bastion` / `ProxyJump` / `tunnel` | **0** | **v0.9.21 无跳板 (O-5 不假装)** | | | |
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 1 实施, 主 19:37 拍板"全用 rust" 1:1 翻译)

#![warn(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// apeireth-mcp 实际导出 tool_bridge::ToolDef, R20 阶段 1 实施期集成 (Mavis 整合 cargo build 时验证).
// skeleton 阶段不锁死具体类型, 9 工具用 JSON 字符串占位.

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的工具名 (eg. "ssh_run_shell" 实际不存在).
// ============================================================================

/// m3 防御: SSH MCP 8 工具白名单 (编译期 hardcode, 不可运行时改).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_ssh_connect",
    "apeireth_ssh_disconnect",
    "apeireth_ssh_exec",
    "apeireth_ssh_upload",
    "apeireth_ssh_download",
    "apeireth_ssh_list",
    "apeireth_ssh_jump",
    "apeireth_ssh_keepalive",
];

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返回 ToolNotWhitelisted).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), SshMcpError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(SshMcpError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// §1 Error ─────────────────────────────────────────────────────────────────

/// SSH MCP Server 错误.
#[derive(Debug, Error)]
pub enum SshMcpError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),
    #[error("SSH connection failed: {0}")] ConnectionFailed(String),
    #[error("SSH auth failed: {0}")] AuthFailed(String),
    #[error("SSH command failed (exit {code}): {stderr}")] CommandFailed { code: i32, stderr: String },
    #[error("SSH channel closed")] ChannelClosed,
    #[error("SSH session timeout ({0:?})")] Timeout(Duration),
    #[error("SSH file transfer failed: {0}")] FileTransfer(String),
    #[error("SSH port forwarding failed: {0}")] PortForward(String),
    #[error("SSH config parse failed: {0}")] ConfigParse(String),
    #[error("SSH host key mismatch: {0}")] HostKeyMismatch(String),
    #[error("SSH private key invalid: {0}")] InvalidPrivateKey(String),
    #[error("SSH jump host failed: {0}")] JumpHost(String),
    #[error("SSH I/O error: {0}")] Io(#[from] std::io::Error),
    #[error("SSH error: {0}")] Other(String),
}

pub type SshMcpResult<T> = Result<T, SshMcpError>;

// §2 enum / 工具类型 ───────────────────────────────────────────────────────

/// SSH 认证方式. O-5 登记: Password + PublicKey 1:1 翻译; JumpHost/Agent R21+ P3 扩展.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SshAuthMethod {
    Password { username: String, password: SecretString },
    PublicKey { username: String, private_key_path: PathBuf, passphrase: Option<SecretString> },
    Agent { username: String, socket_path: Option<PathBuf> },
    JumpHost { jumps: Vec<JumpHostConfig> },
}

/// 跳板配置 (R21+ P3, v0.9.21 SSHMcpServer.js 0 命中).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JumpHostConfig { pub host: String, pub port: u16, pub auth: Box<SshAuthMethod> }

/// SecretString 包装 (Serialize 脱敏 `***REDACTED***`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
    pub fn expose(&self) -> &str { &self.0 }
}
impl Serialize for SecretString { fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str("***REDACTED***") } }
impl<'de> Deserialize<'de> for SecretString { fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> { Ok(Self::new(String::deserialize(d)?)) } }

/// SSH 连接状态 (1:1 翻译 SSHMcpServer.js 9 状态).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SshSessionStatus {
    Disconnected, Connecting, Authenticating, Connected, Busy, Idle, JumpConnecting, Closed, TimedOut,
}

/// SSH 命令执行结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshCommandOutput { pub stdout: String, pub stderr: String, pub exit_code: i32, pub duration: Duration }

/// SFTP 文件信息.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshFileInfo { pub path: String, pub size: u64, pub is_dir: bool, pub modified: Option<SystemTime>, pub permissions: u32 }

/// 会话信息.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshSessionInfo { pub id: String, pub host: String, pub port: u16, pub status: SshSessionStatus, pub created_at: SystemTime, pub last_active_at: SystemTime, pub commands_executed: u64 }

// §3 struct ───────────────────────────────────────────────────────────────

/// SSH MCP Server 配置.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshMcpConfig {
    pub default_host: Option<String>,
    pub default_port: u16,
    pub default_username: Option<String>,
    pub connect_timeout: Duration,
    pub command_timeout: Duration,
    pub keepalive_interval: Duration,
    pub strict_host_key: bool,
    pub known_hosts_path: PathBuf,
    pub default_private_key: Option<PathBuf>,
    pub max_sessions: usize,
}

impl Default for SshMcpConfig {
    fn default() -> Self {
        Self {
            default_host: None, default_port: 22, default_username: None,
            connect_timeout: Duration::from_secs(30),
            command_timeout: Duration::from_secs(60),
            keepalive_interval: Duration::from_secs(15),
            strict_host_key: true,
            known_hosts_path: PathBuf::from("~/.ssh/known_hosts"),
            default_private_key: None,
            max_sessions: 16,
        }
    }
}

/// SSH MCP Server (1:1 翻译 SSHMcpServer.js class).
#[derive(Debug)]
pub struct SshMcpServer {
    config: SshMcpConfig,
    sessions: HashMap<String, SshSession>,
    tools: HashMap<String, String>, // skeleton: tool name → JSON def (R20 阶段 1 换成 apeireth_mcp::ToolDef)
    running: AtomicBool,
}

/// SSH 会话.
#[derive(Debug)]
pub struct SshSession {
    pub id: String, pub host: String, pub port: u16, pub auth: SshAuthMethod,
    pub status: SshSessionStatus, pub created_at: SystemTime, pub last_active_at: SystemTime,
    pub commands_executed: u64,
}

// §4 trait ────────────────────────────────────────────────────────────────

/// SSH MCP Server trait (9 方法, per v0.9.21 实查 1:1 翻译).
#[async_trait]
pub trait SshMcpServerTrait: Send + Sync {
    async fn connect(&self, session_id: &str, host: &str, port: u16, auth: SshAuthMethod) -> SshMcpResult<()>;
    async fn disconnect(&self, session_id: &str) -> SshMcpResult<()>;
    async fn exec(&self, session_id: &str, command: &str) -> SshMcpResult<SshCommandOutput>;
    async fn upload(&self, session_id: &str, local: &PathBuf, remote: &str) -> SshMcpResult<u64>;
    async fn download(&self, session_id: &str, remote: &str, local: &PathBuf) -> SshMcpResult<u64>;
    async fn list(&self, session_id: &str, path: &str) -> SshMcpResult<Vec<SshFileInfo>>;
    async fn jump(&self, session_id: &str, jumps: Vec<JumpHostConfig>) -> SshMcpResult<()>;
    async fn keepalive(&self, session_id: &str) -> SshMcpResult<Duration>;
    async fn list_sessions(&self) -> SshMcpResult<Vec<SshSessionInfo>>;
    async fn get_session_status(&self, session_id: &str) -> SshMcpResult<SshSessionStatus>;
}

// §5 占位实现 (R20 阶段 1 完整 1:1 翻译 v0.9.21 ~6000 LOC) ───────────────

impl SshMcpServer {
    pub fn new(config: SshMcpConfig) -> SshMcpResult<Self> {
        let mut s = Self { config, sessions: HashMap::new(), tools: HashMap::new(), running: AtomicBool::new(false) };
        s.register_default_tools();
        Ok(s)
    }
    fn register_default_tools(&mut self) { /* skeleton: 9 默认工具 */ }
    pub async fn start(&self) -> SshMcpResult<()> { self.running.store(true, Ordering::SeqCst); Ok(()) }
    pub async fn stop(&self) -> SshMcpResult<()> { self.running.store(false, Ordering::SeqCst); Ok(()) }
}

#[async_trait]
impl SshMcpServerTrait for SshMcpServer {
    async fn connect(&self, _id: &str, _h: &str, _p: u16, _a: SshAuthMethod) -> SshMcpResult<()> { Ok(()) }
    async fn disconnect(&self, _id: &str) -> SshMcpResult<()> { Ok(()) }
    async fn exec(&self, _id: &str, _c: &str) -> SshMcpResult<SshCommandOutput> {
        Ok(SshCommandOutput { stdout: String::new(), stderr: "skeleton".to_string(), exit_code: -1, duration: Duration::ZERO })
    }
    async fn upload(&self, _id: &str, _l: &PathBuf, _r: &str) -> SshMcpResult<u64> { Ok(0) }
    async fn download(&self, _id: &str, _r: &str, _l: &PathBuf) -> SshMcpResult<u64> { Ok(0) }
    async fn list(&self, _id: &str, _p: &str) -> SshMcpResult<Vec<SshFileInfo>> { Ok(vec![]) }
    async fn jump(&self, _id: &str, _j: Vec<JumpHostConfig>) -> SshMcpResult<()> { Ok(()) }
    async fn keepalive(&self, _id: &str) -> SshMcpResult<Duration> { Ok(Duration::ZERO) }
    async fn list_sessions(&self) -> SshMcpResult<Vec<SshSessionInfo>> { Ok(vec![]) }
    async fn get_session_status(&self, _id: &str) -> SshMcpResult<SshSessionStatus> { Ok(SshSessionStatus::Disconnected) }
}

// §6 测试 fixture (7 个, R20 阶段 1 Fixture 5 test_mcp_in_process 估) ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_mcp_server_skeleton_creates_with_default_config() {
        assert!(SshMcpServer::new(SshMcpConfig::default()).is_ok());
    }

    #[test]
    fn ssh_auth_method_password_serialize_omits_password() {
        let auth = SshAuthMethod::Password { username: "test".into(), password: SecretString::new("hunter2") };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains("***REDACTED***"));
        assert!(!json.contains("hunter2"));
    }

    #[tokio::test]
    async fn ssh_session_status_serde_roundtrip() {
        for s in [SshSessionStatus::Disconnected, SshSessionStatus::Connecting, SshSessionStatus::Authenticating, SshSessionStatus::Connected, SshSessionStatus::Busy, SshSessionStatus::Idle, SshSessionStatus::JumpConnecting, SshSessionStatus::Closed, SshSessionStatus::TimedOut] {
            let j = serde_json::to_string(&s).unwrap();
            let back: SshSessionStatus = serde_json::from_str(&j).unwrap();
            assert_eq!(back, s);
        }
    }

    #[tokio::test]
    async fn ssh_mcp_server_connect_skeleton_returns_ok() {
        let server = SshMcpServer::new(SshMcpConfig::default()).unwrap();
        let auth = SshAuthMethod::Agent { username: "demo".into(), socket_path: None };
        assert!(server.connect("s1", "127.0.0.1", 22, auth).await.is_ok());
    }

    #[tokio::test]
    async fn ssh_mcp_server_exec_skeleton_returns_dummy_output() {
        let server = SshMcpServer::new(SshMcpConfig::default()).unwrap();
        let out = server.exec("s1", "echo hello").await.unwrap();
        assert_eq!(out.exit_code, -1);
        assert!(out.stdout.is_empty());
    }

    #[tokio::test]
    async fn ssh_mcp_server_jump_skeleton_accepts_jumps() {
        let server = SshMcpServer::new(SshMcpConfig::default()).unwrap();
        let jumps = vec![JumpHostConfig { host: "bastion.example.com".into(), port: 22, auth: Box::new(SshAuthMethod::Agent { username: "u".into(), socket_path: None }) }];
        assert!(server.jump("s1", jumps).await.is_ok());
    }

    #[tokio::test]
    async fn ssh_mcp_server_list_sessions_skeleton_returns_empty() {
        let server = SshMcpServer::new(SshMcpConfig::default()).unwrap();
        assert!(server.list_sessions().await.unwrap().is_empty());
    }
}
