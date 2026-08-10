//! # apeireth-mcp-winrm
//!
//! WinRM MCP Server (1:1 翻译 v0.9.21 商业版 `out/main/mcp/WinRMMcpServer.js` ~64KB)
//!
//! 商业版 8 大闭源模块之一 (per `v09021-commercial-extract §3.2 #10` 估时 2h, 估 800 LOC 1:1 翻译).
//! R20 阶段 1 P0 必补 (per `commercial-vs-fork-diff §2.2` 8 闭源估缺, WinRM 估含).
//!
//! ## WinRM 协议核心 (per WS-Management SOAP over HTTP/HTTPS)
//!
//! - **Transport**: HTTP / HTTPS (端口 5985 / 5986), 走 SOAP 1.2 envelope
//! - **Auth methods** (per v0.9.21 实查 enum 5 项): `Default` / `Basic` / `Negotiate` / `Kerberos` / `CredSSP`
//! - **Shell model**: WSMan Shell (Create / Command / Receive / Send / Delete)
//! - **PowerShell execution**: 经由 `New-PSSession` + `Copy-Item -ToSession` / `-FromSession`,
//!   用 `CLAUDEOPS_WINRM_COMPLETION_MARKER` 检测命令结束 (per v0.9.21 `U()` helper).
//!
//! ## 关键工具清单 (per v0.9.21 实查)
//!
//! | 工具名 | 1:1 翻译 | 估 LOC |
//! |--------|----------|-------:|
//! | `winrm_connect` | WinRM 连接 + 返回 connectionId | 80 |
//! | `winrm_disconnect` | 断开 + 从 pool 删除 | 30 |
//! | `winrm_list_connections` | 列出活跃连接 | 20 |
//! | `winrm_run_command` | 创建 shell + 跑命令 | 120 |
//! | `winrm_get_command_output` | 取命令输出 (流式) | 100 |
//! | `winrm_command` | 取/删除已完成命令 (cleanup) | 80 |
//! | `winrm_copy_to` | 上传文件 (PowerShell Copy-Item) | 100 |
//! | `winrm_copy_from` | 下载文件 (PowerShell Copy-Item -FromSession) | 100 |
//! | **估总** | — | **估 630** (跟 §3.2 估 800 ± 20%) |
//!
//! ## 设计原则 (per 主人 19:37 "全用 rust" 强调)
//!
//! - 1 TypeScript module = 1 Rust crate
//! - TS interface → Rust trait
//! - TS class → Rust struct + impl
//! - TS union → Rust enum
//! - TS Promise → Rust async fn
//! - 0 复用 TS 业务代码
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 1 实施)
//!
//! 关键 trait + struct + 占位 impl 落地, 完整 1:1 翻译放后续 5 周实施期 (per
//! `v09021-commercial-extract §6` 5 阶段重写). 当前 stage 仅跑 `cargo check` + 5 个 fixture.
//!
//! ## 6 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 v0.9.21 商业版 WinRM (~64KB), 0 业务重设计
//! - **S-2 实事求是**: 实查 v0.9.21 估 800 LOC, 当前 skeleton 估 280 LOC (估 35% 完成)
//! - **O-5 不假装**: 所有 trait 方法 `warn!` 占位, 0 假装已实现 WinRM 协议
//! - **O-2 走在前人肩上**: v0.9.21 5 auth methods (Default/Basic/Negotiate/Kerberos/CredSSP) 直接借鉴
//! - **O-3 干到底**: 8 工具全部 trait 定义, 5 fixture 验证结构
//! - **O-4 任何人都能接手**: §1-§6 跟 apeireth-mcp-ssh 同骨架 + 引用 v0.9.21 路径
//!
//! ## 引用文档 (4 份)
//!
//! 1. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-commercial-extract-2026-08-05.md`
//! 2. `.minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\commercial-vs-fork-diff-2026-08-05.md`
//! 3. v0.9.21 商业版 `out/main/mcp/WinRMMcpServer.js` (~64KB)
//! 4. `.openclaw\workspace\promethean\Apeireth-rust\crates\apeireth-mcp\Cargo.toml` (参考)

#![warn(missing_docs)] // R19 第 0 阶段: missing_docs warn
#![allow(clippy::all)] // R19 T10: 108 警告 allow, 等 LLM 修

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, instrument, warn};

// 注: 不 re-export `apeireth_mcp::builtin::*` 或 `apeireth_protocol::ProviderEvent`
// (per 1:1 翻译 R20 阶段 1 P0 协调 — 5 crate 并行, 共享模块待 Mavis 整合 commit 统一添加).
// 跟 apeireth-mcp-ssh / apeireth-mcp-relay-image 风格统一, 内部用 `use` 显式导入.

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode, validate_tool_call 在 dispatch 前 schema 校验.
// 防止 minimax m3 模型幻觉调用不存在的工具名.
// ============================================================================

/// m3 防御: WinRM MCP 8 工具白名单 (编译期 hardcode).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_winrm_connect",
    "apeireth_winrm_disconnect",
    "apeireth_winrm_list_connections",
    "apeireth_winrm_run_command",
    "apeireth_winrm_get_command_output",
    "apeireth_winrm_command",
    "apeireth_winrm_copy_to",
    "apeireth_winrm_copy_from",
];

/// m3 防御: 校验工具调用是否在白名单内.
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), WinRmMcpError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(WinRmMcpError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 错误类型 (1:1 翻译 WinRMMcpServer.js 异常类, 估 10 个 → Rust enum 10 变体)
// ============================================================================

/// WinRM MCP Server 错误 (1:1 翻译 WinRMMcpServer.js 异常类)
#[derive(Debug, Error)]
pub enum WinRmMcpError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),
    /// WinRM HTTP 错误 (transport 失败 / 5xx)
    #[error("WinRM HTTP error: {0}")]
    Http(String),

    /// WinRM SOAP fault 解析失败 (per WSMan SOAP 1.2 fault 元素)
    #[error("WinRM SOAP fault: code={code} subcode={subcode} reason={reason}")]
    SoapFault {
        /// SOAP fault code (e.g. "s:Receiver", "s:Sender")
        code: String,
        /// SOAP subcode (e.g. "w:InternalError")
        subcode: String,
        /// SOAP fault reason (人类可读)
        reason: String,
    },

    /// WinRM 认证失败 (per v0.9.21 5 auth methods 之一失败)
    #[error("WinRM auth failed ({method:?}): {reason}")]
    AuthFailed {
        /// 尝试的认证方式
        method: WinRmAuthMethodKind,
        /// 失败原因
        reason: String,
    },

    /// WinRM shell 创建失败
    #[error("WinRM shell creation failed: {0}")]
    ShellCreate(String),

    /// WinRM 命令执行失败 (exit code != 0)
    #[error("WinRM command failed (exit {code}): {stderr}")]
    CommandFailed {
        /// exit code (PowerShell $LASTEXITCODE)
        code: i32,
        /// stderr / 错误流
        stderr: String,
    },

    /// WinRM shell 已关闭
    #[error("WinRM shell {0} closed")]
    ShellClosed(String),

    /// WinRM 连接超时
    #[error("WinRM operation timeout ({0:?})")]
    Timeout(Duration),

    /// WinRM TLS / HTTPS 证书验证失败
    #[error("WinRM TLS error: {0}")]
    Tls(String),

    /// WinRM profile 加密/解密失败 (PBKDF2 + AES-256-GCM)
    #[error("WinRM profile crypto error: {0}")]
    ProfileCrypto(String),

    /// WinRM profile 解析失败 (JSON shape 错误)
    #[error("WinRM profile parse failed: {0}")]
    ProfileParse(String),

    /// reqwest 错误 (transport)
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    /// quick-xml 错误 (SOAP 解析)
    #[error("XML parse error: {0}")]
    Xml(String),

    /// WinRM 通用错误
    #[error("WinRM error: {0}")]
    Other(String),
}

/// WinRM MCP Server Result 类型
pub type WinRmMcpResult<T> = Result<T, WinRmMcpError>;

// ============================================================================
// §2 关键 enum (1:1 翻译 WinRMMcpServer.js union)
// ============================================================================

/// WinRM 认证方式种类 (per v0.9.21 实查 inputSchema enum 5 项)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum WinRmAuthMethodKind {
    /// 默认 (由 WSMan 端点协商)
    Default,
    /// HTTP Basic (用户名/密码 base64)
    Basic,
    /// Negotiate (SPNEGO, 优先 Kerberos 退 NTLM)
    Negotiate,
    /// Kerberos
    Kerberos,
    /// CredSSP (Credential Security Support Provider, 委派)
    CredSsp,
}

/// WinRM 传输方案 (per v0.9.21 use_ssl boolean)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WinRmTransport {
    /// HTTP (默认端口 5985)
    Http,
    /// HTTPS (默认端口 5986, WSMan 证书)
    Https,
}

/// WinRM shell 状态 (per WSMan Shell 生命周期)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WinRmShellStatus {
    /// 未创建
    None,
    /// 创建中 (POST 到 /wsman)
    Creating,
    /// 已创建 (active)
    Active,
    /// 命令执行中
    Busy,
    /// 已关闭 (正常)
    Closed,
    /// 已关闭 (异常 / 超时)
    Aborted,
}

/// WinRM 命令执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WinRmCommandStatus {
    /// 命令已启动
    Pending,
    /// 执行中
    Running,
    /// 已完成 (exit 0)
    Done,
    /// 失败 (exit != 0)
    Failed,
    /// 取消
    Cancelled,
}

/// WinRM profile (per v0.9.21 claudeops-winrm-profile, 加密存储在 host 上)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WinRmProfile {
    /// profile 名
    pub name: String,
    /// 主机
    pub host: String,
    /// 端口
    pub port: u16,
    /// 用户名
    pub username: String,
    /// 密码 (SecretString 包装, 序列化时脱敏)
    pub password: SecretString,
    /// 认证方式
    pub authentication: WinRmAuthMethodKind,
    /// 是否 HTTPS
    pub use_ssl: bool,
    /// 跳过 CA 检查 (per v0.9.21 skip_ca_check)
    pub skip_ca_check: bool,
    /// 跳过 CN 检查
    pub skip_cn_check: bool,
    /// 跳过证书撤销检查
    pub skip_revocation_check: bool,
    /// PowerShell endpoint configuration name (per v0.9.21 configuration_name)
    pub configuration_name: Option<String>,
}

// ============================================================================
// §3 关键 struct (1:1 翻译 WinRMMcpServer.js class)
// ============================================================================

/// WinRM endpoint 配置 (per v0.9.21 winrm_connect tool inputSchema)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WinRmEndpoint {
    /// 主机 (IP 或 DNS)
    pub host: String,
    /// 端口 (默认 5985 HTTP / 5986 HTTPS)
    pub port: u16,
    /// 传输方案
    pub transport: WinRmTransport,
    /// 认证方式
    pub auth: WinRmAuthMethod,
    /// 跳过 CA 检查
    pub skip_ca_check: bool,
    /// 跳过 CN 检查
    pub skip_cn_check: bool,
    /// 跳过证书撤销检查
    pub skip_revocation_check: bool,
    /// PowerShell endpoint configuration name
    pub configuration_name: Option<String>,
    /// 操作超时 (毫秒, per v0.9.21 operation_timeout_ms)
    pub operation_timeout: Duration,
}

/// WinRM 认证方式 (含凭证, 1:1 翻译 v0.9.21 inputSchema auth 字段)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WinRmAuthMethod {
    /// 默认 (协商)
    Default {
        /// 用户名
        username: String,
        /// 密码
        password: SecretString,
    },
    /// HTTP Basic
    Basic {
        /// 用户名
        username: String,
        /// 密码
        password: SecretString,
    },
    /// Negotiate (SPNEGO)
    Negotiate {
        /// 用户名
        username: String,
        /// 密码
        password: SecretString,
    },
    /// Kerberos
    Kerberos {
        /// 用户名 (含 @REALM 或留空)
        username: String,
        /// 密码
        password: SecretString,
    },
    /// CredSSP
    CredSsp {
        /// 用户名
        username: String,
        /// 密码
        password: SecretString,
    },
}

impl WinRmAuthMethod {
    /// 取认证方式种类
    pub fn kind(&self) -> WinRmAuthMethodKind {
        match self {
            Self::Default { .. } => WinRmAuthMethodKind::Default,
            Self::Basic { .. } => WinRmAuthMethodKind::Basic,
            Self::Negotiate { .. } => WinRmAuthMethodKind::Negotiate,
            Self::Kerberos { .. } => WinRmAuthMethodKind::Kerberos,
            Self::CredSsp { .. } => WinRmAuthMethodKind::CredSsp,
        }
    }
}

/// SecretString 包装 (per R19 lints `private_interfaces` 替代, 跟 apeireth-mcp-ssh 同款)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// 创建 secret
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    /// 暴露值 (仅在必要时, e.g. HTTP Authorization header)
    pub fn expose(&self) -> &str {
        &self.0
    }
    /// 暴露 Basic auth header value (`base64(user:pass)`)
    pub fn basic_auth_header(&self, username: &str) -> String {
        use base64::Engine;
        let raw = format!("{}:{}", username, self.0);
        base64::engine::general_purpose::STANDARD.encode(raw)
    }
}

impl Serialize for SecretString {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("***REDACTED***")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::new(s))
    }
}

/// WinRM 连接 (per v0.9.21 connectionId 4 字节 base64, connection pool 1 项)
#[derive(Debug, Clone)]
pub struct WinRmConnection {
    /// 连接 ID (per v0.9.21 `randomBytes(4).toString('base64')`)
    pub id: String,
    /// endpoint 配置
    pub endpoint: WinRmEndpoint,
    /// 创建时间 (per v0.9.21 `new Date().toISOString()`)
    pub created_at: SystemTime,
    /// 最后活跃时间
    pub last_active_at: SystemTime,
}

/// WinRM shell (1 个 shell = 1 个 PowerShell session)
#[derive(Debug, Clone)]
pub struct WinRmShell {
    /// shell ID (per WSMan ShellId, UUID 格式)
    pub id: String,
    /// 所属 connection
    pub connection_id: String,
    /// 状态
    pub status: WinRmShellStatus,
    /// 创建时间
    pub created_at: SystemTime,
}

/// WinRM 命令执行输出
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WinRmCommandOutput {
    /// stdout (PowerShell Write-Output)
    pub stdout: String,
    /// stderr (PowerShell error stream)
    pub stderr: String,
    /// exit code (PowerShell $LASTEXITCODE)
    pub exit_code: i32,
    /// 执行时长
    pub duration: Duration,
    /// 命令 ID (per WSMan CommandId)
    pub command_id: String,
}

/// WinRM MCP Server 状态 (1:1 翻译 WinRMMcpServer.js class WinRmMcpServer)
#[derive(Debug)]
pub struct WinRmMcpServer {
    /// 配置
    config: WinRmMcpConfig,
    /// 连接池 (connectionId → WinRmConnection)
    connections: HashMap<String, WinRmConnection>,
    /// shell 表 (shellId → WinRmShell)
    shells: HashMap<String, WinRmShell>,
    /// 命令输出表 (commandId → WinRmCommandOutput)
    commands: HashMap<String, WinRmCommandOutput>,
    /// 是否运行中
    running: AtomicBool,
}

/// WinRM MCP Server 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinRmMcpConfig {
    /// 默认端口 (5985 HTTP)
    pub default_port_http: u16,
    /// 默认端口 (5986 HTTPS)
    pub default_port_https: u16,
    /// 默认操作超时 (per v0.9.21 operation_timeout_ms 估 60s)
    pub operation_timeout: Duration,
    /// profile 加密 key 派生 PBKDF2 迭代 (per v0.9.21 `100000`)
    pub profile_pbkdf2_iterations: u32,
    /// profile 加密 salt path (per v0.9.21 `~/.claudeops-winrm-profile-salt-v1`)
    pub profile_salt_path: String,
    /// profile 存储 path (per v0.9.21 `~/.claudeops/profiles.json`)
    pub profile_storage_path: String,
    /// 最大并发连接
    pub max_connections: usize,
    /// 缺省认证方式
    pub default_auth_kind: WinRmAuthMethodKind,
}

impl Default for WinRmMcpConfig {
    fn default() -> Self {
        Self {
            default_port_http: 5985,
            default_port_https: 5986,
            operation_timeout: Duration::from_secs(60),
            profile_pbkdf2_iterations: 100_000,
            profile_salt_path: "~/.claudeops-winrm-profile-salt-v1".to_string(),
            profile_storage_path: "~/.claudeops/profiles.json".to_string(),
            max_connections: 16,
            default_auth_kind: WinRmAuthMethodKind::Negotiate,
        }
    }
}

/// WinRM 连接信息 (1:1 翻译 v0.9.21 list_connections 响应)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinRmConnectionInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub transport: WinRmTransport,
    pub auth_kind: WinRmAuthMethodKind,
    pub created_at: SystemTime,
    pub last_active_at: SystemTime,
}

// ============================================================================
// §4 关键 trait (1:1 翻译 WinRMMcpServer.js 8 工具)
// ============================================================================

/// WinRM MCP Server trait (per 1:1 翻译 v0.9.21 WinRMMcpServer.js 8 工具)
#[async_trait]
pub trait WinRmMcpServerTrait: Send + Sync {
    /// 连接 + 返回 connectionId (1:1 翻译 `winrm_connect` tool)
    async fn connect(&self, endpoint: WinRmEndpoint) -> WinRmMcpResult<String>;

    /// 断开 (1:1 翻译 `winrm_disconnect` tool)
    async fn disconnect(&self, connection_id: &str) -> WinRmMcpResult<()>;

    /// 列出活跃连接 (1:1 翻译 `winrm_list_connections` tool)
    async fn list_connections(&self) -> WinRmMcpResult<Vec<WinRmConnectionInfo>>;

    /// 跑命令 + 返回 commandId (1:1 翻译 `winrm_run_command` tool)
    async fn run_command(
        &self,
        connection_id: &str,
        powershell_script: &str,
        timeout: Option<Duration>,
    ) -> WinRmMcpResult<String>;

    /// 取命令输出 (1:1 翻译 `winrm_get_command_output` tool)
    async fn get_command_output(
        &self,
        connection_id: &str,
        command_id: &str,
    ) -> WinRmMcpResult<WinRmCommandOutput>;

    /// 取/清理已完成命令 (1:1 翻译 `winrm_command` tool 估含 cleanup)
    async fn cleanup_command(
        &self,
        connection_id: &str,
        command_id: &str,
    ) -> WinRmMcpResult<WinRmCommandStatus>;

    /// 上传文件 (1:1 翻译 `winrm_copy_to` tool — PowerShell `Copy-Item -ToSession`)
    async fn copy_to(
        &self,
        connection_id: &str,
        local_path: &str,
        remote_path: &str,
        timeout: Option<Duration>,
    ) -> WinRmMcpResult<u64>;

    /// 下载文件 (1:1 翻译 `winrm_copy_from` tool — PowerShell `Copy-Item -FromSession`)
    async fn copy_from(
        &self,
        connection_id: &str,
        remote_path: &str,
        local_path: &str,
        timeout: Option<Duration>,
    ) -> WinRmMcpResult<u64>;
}

// ============================================================================
// §5 占位实现 (TODO: R20 阶段 1 完整 1:1 翻译 v0.9.21 WinRMMcpServer.js ~64KB)
// ============================================================================

impl WinRmMcpServer {
    /// 创建 WinRM MCP Server (骨架, 完整实现等 R20 阶段 1)
    #[instrument(skip(config))]
    pub fn new(config: WinRmMcpConfig) -> WinRmMcpResult<Self> {
        info!("Creating WinRM MCP Server with config: {:?}", config);
        let mut server = Self {
            config,
            connections: HashMap::new(),
            shells: HashMap::new(),
            commands: HashMap::new(),
            running: AtomicBool::new(false),
        };
        server.register_default_tools();
        Ok(server)
    }

    /// 注册 8 默认工具 (per v0.9.21 WinRMMcpServer 实查 8 tool)
    fn register_default_tools(&mut self) {
        debug!("Registering 8 default WinRM MCP tools (per v0.9.21 实查)");
        // TODO: 1:1 翻译 WinRMMcpServer.js 8 tool registration
    }

    /// 启动 Server
    #[instrument(skip(self))]
    pub async fn start(&self) -> WinRmMcpResult<()> {
        self.running.store(true, std::sync::atomic::Ordering::SeqCst);
        info!("WinRM MCP Server started (server name: winrm-remote, per v0.9.21)");
        Ok(())
    }

    /// 停止 Server
    #[instrument(skip(self))]
    pub async fn stop(&self) -> WinRmMcpResult<()> {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        info!("WinRM MCP Server stopped");
        Ok(())
    }

    /// 取 config 引用
    pub fn config(&self) -> &WinRmMcpConfig {
        &self.config
    }
}

#[async_trait]
impl WinRmMcpServerTrait for WinRmMcpServer {
    async fn connect(&self, _endpoint: WinRmEndpoint) -> WinRmMcpResult<String> {
        warn!("WinRmMcpServer::connect skeleton — TODO: 1:1 翻译 v0.9.21 WinRMMcpServer.js winrm_connect (POST /wsman SOAP Create Shell)");
        Ok(String::from("skeleton-connection-id"))
    }
    async fn disconnect(&self, _connection_id: &str) -> WinRmMcpResult<()> {
        warn!("WinRmMcpServer::disconnect skeleton");
        Ok(())
    }
    async fn list_connections(&self) -> WinRmMcpResult<Vec<WinRmConnectionInfo>> {
        warn!("WinRmMcpServer::list_connections skeleton");
        Ok(vec![])
    }
    async fn run_command(
        &self,
        _connection_id: &str,
        _powershell_script: &str,
        _timeout: Option<Duration>,
    ) -> WinRmMcpResult<String> {
        warn!("WinRmMcpServer::run_command skeleton — TODO: New-PSSession + CLAUDEOPS_WINRM_COMPLETION_MARKER 模式");
        Ok(String::from("skeleton-command-id"))
    }
    async fn get_command_output(
        &self,
        _connection_id: &str,
        _command_id: &str,
    ) -> WinRmMcpResult<WinRmCommandOutput> {
        warn!("WinRmMcpServer::get_command_output skeleton");
        Ok(WinRmCommandOutput {
            stdout: String::new(),
            stderr: "TODO: skeleton".to_string(),
            exit_code: -1,
            duration: Duration::ZERO,
            command_id: String::from("skeleton"),
        })
    }
    async fn cleanup_command(
        &self,
        _connection_id: &str,
        _command_id: &str,
    ) -> WinRmMcpResult<WinRmCommandStatus> {
        warn!("WinRmMcpServer::cleanup_command skeleton");
        Ok(WinRmCommandStatus::Done)
    }
    async fn copy_to(
        &self,
        _connection_id: &str,
        _local_path: &str,
        _remote_path: &str,
        _timeout: Option<Duration>,
    ) -> WinRmMcpResult<u64> {
        warn!("WinRmMcpServer::copy_to skeleton — TODO: PowerShell Copy-Item -ToSession");
        Ok(0)
    }
    async fn copy_from(
        &self,
        _connection_id: &str,
        _remote_path: &str,
        _local_path: &str,
        _timeout: Option<Duration>,
    ) -> WinRmMcpResult<u64> {
        warn!("WinRmMcpServer::copy_from skeleton — TODO: PowerShell Copy-Item -FromSession");
        Ok(0)
    }
}

// ============================================================================
// §6 测试 fixture (R20 阶段 1 Fixture 5 test_mcp_in_process 估)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winrm_mcp_server_skeleton_creates_with_default_config() {
        let config = WinRmMcpConfig::default();
        let server = WinRmMcpServer::new(config);
        assert!(
            server.is_ok(),
            "WinRmMcpServer::new should succeed with default config"
        );
    }

    #[test]
    fn winrm_auth_method_password_serialize_omits_password() {
        let auth = WinRmAuthMethod::Basic {
            username: "Administrator".to_string(),
            password: SecretString::new("P@ssw0rd!"),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(
            json.contains("***REDACTED***"),
            "Password should be redacted in JSON"
        );
        assert!(
            !json.contains("P@ssw0rd!"),
            "Password value should NOT appear in JSON"
        );
    }

    #[test]
    fn winrm_auth_method_kind_enum_roundtrip() {
        for kind in [
            WinRmAuthMethodKind::Default,
            WinRmAuthMethodKind::Basic,
            WinRmAuthMethodKind::Negotiate,
            WinRmAuthMethodKind::Kerberos,
            WinRmAuthMethodKind::CredSsp,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: WinRmAuthMethodKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind, "roundtrip failed for {:?}", kind);
        }
        // v0.9.21 实查: 5 auth methods — Default / Basic / Negotiate / Kerberos / CredSSP
        assert_eq!(5, [
            WinRmAuthMethodKind::Default,
            WinRmAuthMethodKind::Basic,
            WinRmAuthMethodKind::Negotiate,
            WinRmAuthMethodKind::Kerberos,
            WinRmAuthMethodKind::CredSsp,
        ]
        .len());
    }

    #[test]
    fn winrm_shell_status_serde_roundtrip() {
        for status in [
            WinRmShellStatus::None,
            WinRmShellStatus::Creating,
            WinRmShellStatus::Active,
            WinRmShellStatus::Busy,
            WinRmShellStatus::Closed,
            WinRmShellStatus::Aborted,
        ] {
            let json = serde_json::to_string(&status).unwrap();
            let back: WinRmShellStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(back, status);
        }
    }

    #[test]
    fn winrm_secret_basic_auth_header_encodes_correctly() {
        // HTTP Basic 头: base64("user:pass") = "dXNlcjpwYXNz"
        let pw = SecretString::new("pass");
        let header = pw.basic_auth_header("user");
        assert_eq!(header, "dXNlcjpwYXNz", "base64('user:pass') should match");
        // 反向验证
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&header)
            .unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "user:pass");
    }

    #[tokio::test]
    async fn winrm_skeleton_trait_methods_return_placeholders() {
        let server = WinRmMcpServer::new(WinRmMcpConfig::default()).unwrap();
        let conn_id = server
            .connect(WinRmEndpoint {
                host: "127.0.0.1".to_string(),
                port: 5985,
                transport: WinRmTransport::Http,
                auth: WinRmAuthMethod::Default {
                    username: "test".to_string(),
                    password: SecretString::new("test"),
                },
                skip_ca_check: false,
                skip_cn_check: false,
                skip_revocation_check: false,
                configuration_name: None,
                operation_timeout: Duration::from_secs(30),
            })
            .await
            .unwrap();
        assert_eq!(conn_id, "skeleton-connection-id");

        let cmd_id = server
            .run_command(&conn_id, "Get-Process", None)
            .await
            .unwrap();
        assert_eq!(cmd_id, "skeleton-command-id");

        let output = server.get_command_output(&conn_id, &cmd_id).await.unwrap();
        assert_eq!(output.exit_code, -1);

        let status = server.cleanup_command(&conn_id, &cmd_id).await.unwrap();
        assert_eq!(status, WinRmCommandStatus::Done);

        server.disconnect(&conn_id).await.unwrap();
    }
}
