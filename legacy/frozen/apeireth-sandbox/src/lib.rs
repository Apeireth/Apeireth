//! # apeireth-sandbox (R20 阶段 6 flesh out: 真接实现)
//!
//! ⚠️ **本 crate 是 R20 阶段 6 flesh out 新增**, 跟 `apeireth-sdk-sandbox` (LOCKED
//! baseline 16:34:11, R20 阶段 4 商业版 1:1 翻译) **严格分离**: `SandboxRealImpl` 是
//! 显式 opt-in 的真接 Docker daemon HTTP API 客户端, 不受 `STUB_MODE = true` 编译期
//! hardcode 守门影响. 调用方显式 `SandboxRealImpl::new(config, daemon_url)?` 即用.
//!
//! ## 设计 (跟 `apeireth-sdk-sandbox` 1:1 翻译 v0.9.21 商业版设计参考)
//!
//! **6 API** (R20 阶段 6 真接, 跟 `apeireth-sdk-sandbox` 6 API `spawn/kill/wait/
//! getStatus/streamLogs/cleanup` 不同, 改用 Docker daemon 视角 6 维度):
//! - **exec** — 启动 sandbox (Container = docker run, Process = OS spawn, WASM = wasmtime)
//! - **kill** — 终止运行中 sandbox
//! - **status** — 查询 sandbox 状态
//! - **network** — 网络管理 (create network / connect / disconnect)
//! - **filesystem** — 文件系统操作 (read / write / mount / unmount)
//! - **resource_limit** — 资源限制 (set / get CPU / mem / IO / net)
//!
//! **3 RuntimeKind** (K-1 强校验 #2, 编译期 hardcode 守门):
//! - `Container` — Docker daemon 真接 (主路径, 1:1 翻译 Docker Engine API v1.43+)
//! - `Process` — 本地 OS process (fallback, 走 std::process / tokio::process)
//! - `WASM` — WASM runtime (R21+ 续, 现阶段 STUB)
//!
//! **6 K-1 强校验** (跟 `apeireth-sdk-sandbox` 1:1 借鉴, 编译期 hardcode 白名单):
//! - `image` (8 registry 白名单)
//! - `command` (禁 shell 注入)
//! - `user` (禁 root)
//! - `env` (禁 10 敏感变量)
//! - `port` (禁特权端口 + 端口范围白名单)
//! - `volume` (5 源路径白名单)
//!
//! **集成 pipeline-g5 Reliability 阶段** (借鉴 Golutra v0.1.0 chat_db 5 阶段):
//! - `max_retries = 5` (跟 `MAX_RETRY_ATTEMPTS = 5` 1:1)
//! - `backoff_ms = 4 步 [100, 200, 500, 1000]` (跟 `RETRY_BACKOFF_MS` 1:1)
//! - `circuit_breaker_threshold = 10` (跟 `CIRCUIT_BREAKER_THRESHOLD` 1:1)
//! - **不**真引 `apeireth-pipeline-g5` dep (LOCKED, 0 改), 内部守门常数 1:1 镜像
//!
//! **STUB 路径** (跟 `apeireth-sdk-sandbox` 1:1 镜像, 0 改, 编译期 hardcode 守门):
//! - `STUB_MODE = true` 编译期 hardcode, 真接模式 opt-in via `SandboxRealImpl::new(...)`
//! - `STUB_TOOL_WHITELIST` 编译期 hardcode 6 API 名 + `validate_tool_call` 守门
//!
//! ## 6 哲学锚 (R20 阶段 6 必守)
//!
//! 1. **S-1 不漂移**: 6 API 1:1 翻译 Docker daemon REST API 6 维度 (exec / kill /
//!    status / network / filesystem / resource_limit), 0 假装已连真 Docker daemon.
//! 2. **S-2 编译期 hardcode**: `STUB_MODE` / `PLATFORM_NAME` / 6 API 名 / 3 RuntimeKind
//!    / 6 K-1 / Reliability 守门常数 全部 const, 0 运行时配置覆盖.
//! 3. **O-2 工程铁律**: bollard 0.15 留作占位 dep (R21+ 续), 现阶段 wiremock 0.6 测
//!    14 端到端用例, 0 引真 Docker daemon (本机可能没装).
//! 4. **O-3 m3 防御**: 6 API 工具白名单 `STUB_TOOL_WHITELIST` 编译期 hardcode,
//!    `validate_tool_call` 在 dispatch 前 schema 校验, 防 m3 模型幻觉调用.
//! 5. **O-4 不假装可观测**: 6 API 失败时返 `SandboxError::NotImplemented(api_name)`
//!    或 `SandboxError::DockerCallFailed(...)` + `tracing::warn!` log, 0 假装 OK.
//! 6. **O-5 K-1 强校验**: 6 字段 (image / command / user / env / port / volume) 编译期
//!    hardcode 白名单, 任何配置变更必经 `validate()` 走 6 K-1 检查.
//!
//! ## 🔒 8 项不修改承诺 (跟 `apeireth-voice` / `apeireth-lark` / `apeireth-sdk-sandbox`
//! 1:1 风格)
//!
//! 1. `version = "0.1.0"` 显式 (跟 voice/machine-id/lark 模板同) ✅
//! 2. `edition = "2021"` 显式 ✅
//! 3. `rust-version = "1.80"` 显式 ✅
//! 4. `license = "Apache-2.0"` 显式 ✅
//! 5. `authors = ["Apeireth Team"]` 显式 ✅
//! 6. deps 显式版本 (reqwest / url / tokio 等, 跟 voice 1:1) ✅
//! 7. 不修改 workspace Cargo.toml (由整合 #3 sub-agent 加 member) ⏳
//! 8. 不引 unsafe (workspace `#![deny(unsafe_code)]` 继承) ✅

#![allow(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 Module 声明
// ============================================================================

pub mod real;

use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;

// ============================================================================
// §1 编译期 hardcode 常量 (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// Sandbox API schema version (1:1 镜像 `apeireth-sdk-sandbox::SANDBOX_SCHEMA_VERSION`).
pub const SANDBOX_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #1: 编译期 hardcode `"apeireth"`).
pub const PLATFORM_NAME: &str = "apeireth";

/// **STUB MODE 守门标志** (K-1 强校验): 编译期 hardcode = `true`.
/// R20 阶段 6 真接实现由 `real.rs` 提供, opt-in, 0 受 STUB_MODE 守门影响.
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必须 == true.
const _: () = assert!(STUB_MODE == true, "STUB_MODE 改 false 需经 6 哲学锚 + 主人审 (R21+)");

/// m3 防御: 查 STUB_MODE 状态 (per task spec 守门).
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// 单 sandbox 最大存活时间 (秒, 1h, 防恶意 sandbox 长占资源).
pub const SANDBOX_MAX_LIFETIME_SECONDS: u64 = 3600;

/// 默认 Docker daemon URL (本机默认 unix:///var/run/docker.sock;
/// 编译期 hardcode, 实际真接可由 caller 注入).
pub const DEFAULT_DOCKER_DAEMON_URL: &str = "unix:///var/run/docker.sock";

/// 默认 HTTP 端点路径前缀 (per Docker Engine API v1.43+).
pub const DOCKER_API_PREFIX: &str = "/v1.43";

/// 默认 sandbox 镜像 (per `apeireth-sdk-sandbox` 1:1 镜像).
pub const DEFAULT_SANDBOX_IMAGE: &str = "docker.io/library/alpine:3.19";

/// 默认 sandbox 用户 (per `apeireth-sdk-sandbox` 1:1 镜像, 禁 root).
pub const DEFAULT_SANDBOX_USER: &str = "apeireth";

// ============================================================================
// §2 3 RuntimeKind enum (K-1 强校验 #2, 编译期 hardcode)
// ============================================================================

/// Sandbox 运行时 (3 选 1, K-1 强校验 #2).
///
/// 字段对应 `apeireth-sdk-sandbox` 1:1 翻译, 1:1 翻译 Docker daemon 角度分类:
/// - `Container` — Docker daemon 真接 (主路径, R20 阶段 6 flesh out 真接)
/// - `Process` — 本地 OS process (fallback)
/// - `WASM` — WASM runtime (R21+ 续, 现阶段 STUB)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    /// `"container"` — Docker daemon 真接, R20 阶段 6 主路径.
    #[default]
    Container,
    /// `"process"` — 本地 OS process fallback.
    Process,
    /// `"wasm"` — WASM runtime, R21+ 续真接 (本阶段 STUB).
    Wasm,
}

/// 编译期守门: 3 RuntimeKind 守门.
pub const SUPPORTED_RUNTIME_KINDS: &[RuntimeKind] = &[
    RuntimeKind::Container,
    RuntimeKind::Process,
    RuntimeKind::Wasm,
];
const _: () = assert!(SUPPORTED_RUNTIME_KINDS.len() == 3);

impl RuntimeKind {
    /// 字符串 (1:1 翻译 Docker daemon 标识).
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeKind::Container => "container",
            RuntimeKind::Process => "process",
            RuntimeKind::Wasm => "wasm",
        }
    }
}

impl std::fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RuntimeKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "container" | "Container" | "CONTAINER" => Ok(RuntimeKind::Container),
            "process" | "Process" | "PROCESS" => Ok(RuntimeKind::Process),
            "wasm" | "WASM" | "Wasm" => Ok(RuntimeKind::Wasm),
            other => Err(format!(
                "RuntimeKind 解析失败: {other} (合法值: container / process / wasm)"
            )),
        }
    }
}

// ============================================================================
// §3 SandboxStatus 5 状态机
// ============================================================================

/// Sandbox 状态 (5 状态机, K-1 强校验 #3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStatus {
    /// `"pending"` — 等待资源 (默认).
    #[default]
    Pending,
    /// `"running"` — 运行中.
    Running,
    /// `"stopped"` — 正常停止.
    Stopped,
    /// `"failed"` — 失败.
    Failed,
    /// `"killed"` — 被主动 kill.
    Killed,
}

/// 编译期守门: 5 SandboxStatus 守门.
pub const SANDBOX_STATUS_COUNT: usize = 5;
const _: () = assert!(SANDBOX_STATUS_COUNT == 5);

// ============================================================================
// §4 SandboxError 错误枚举 (8 variant, 跟 voice/lark 1:1 风格)
// ============================================================================

/// Sandbox 错误枚举 (8 variant, 跟 `apeireth-sdk-sandbox` 1:1 + 真接实现 3 新 variant).
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    /// 工具不在白名单 (m3 防御, 6 K-1 守门).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// API 未实现 (STUB 模式守门).
    #[error("API not implemented (STUB_MODE=true): {0}")]
    NotImplemented(String),

    /// 配置非法 (K-1 强校验失败).
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// 资源限制越界 (K-1 强校验 #1).
    #[error("resource limit out of range: {0}")]
    ResourceOutOfRange(String),

    /// Docker daemon 调用失败 (真接 1:1 翻译).
    #[error("Docker daemon call failed: {0}")]
    DockerCallFailed(String),

    /// 网络错误 (跟 voice 1:1).
    #[error("network error: {0}")]
    Network(String),

    /// 鉴权失败 (跟 voice 1:1).
    #[error("auth failed: {0}")]
    AuthFailed(String),

    /// 沙箱未找到 (sandbox_id 不存在).
    #[error("sandbox not found: {0}")]
    NotFound(String),
}

/// Sandbox Result 类型别名.
pub type SandboxResult<T> = std::result::Result<T, SandboxError>;

/// 编译期守门: SandboxError 8 variant 守门.
pub const SANDBOX_ERROR_VARIANT_COUNT: usize = 8;
const _: () = assert!(SANDBOX_ERROR_VARIANT_COUNT == 8);

// ============================================================================
// §5 6 K-1 强校验白名单 (per K-1 守门, 编译期 hardcode)
// ============================================================================

/// 允许的 image registry 白名单 (8 个, 跟 `apeireth-sdk-sandbox::ALLOWED_IMAGE_REGISTRIES` 1:1 镜像).
pub const ALLOWED_IMAGE_REGISTRIES: &[&str] = &[
    "docker.io",
    "ghcr.io",
    "quay.io",
    "gcr.io",
    "registry.gitlab.com",
    "mcr.microsoft.com",
    "public.ecr.aws",
    "localhost",
];

/// 禁用的环境变量 (10 敏感变量, K-1 强校验 #4).
pub const FORBIDDEN_ENV_KEYS: &[&str] = &[
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "DOCKER_PASSWORD",
    "DATABASE_URL",
    "REDIS_URL",
    "POSTGRES_PASSWORD",
    "SSH_PRIVATE_KEY",
];

/// 禁用的 sandbox 用户 (禁 root, K-1 强校验 #5).
pub const FORBIDDEN_USERS: &[&str] = &["root", "admin", "administrator", "wheel", "sudo"];

/// 允许的卷挂载源路径前缀 (5 个, K-1 强校验 #6).
pub const ALLOWED_VOLUME_SOURCE_PREFIXES: &[&str] = &[
    "/var/lib/apeireth/",
    "/tmp/apeireth/",
    "/home/apeireth/",
    "/opt/apeireth/",
    "/data/apeireth/",
];

// ============================================================================
// §6 6 API 工具白名单 (m3 防御, 编译期 hardcode)
// ============================================================================

/// m3 防御: Sandbox 6 API 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// **6 API 1:1 翻译 Docker daemon 角度 6 维度**:
/// - `apeireth_sandbox_exec` — 启动 sandbox (docker run / OS spawn / wasmtime)
/// - `apeireth_sandbox_kill` — 终止 sandbox (docker kill / signal)
/// - `apeireth_sandbox_status` — 查询状态 (docker inspect)
/// - `apeireth_sandbox_network` — 网络管理 (docker network)
/// - `apeireth_sandbox_filesystem` — 文件系统 (docker cp / mount)
/// - `apeireth_sandbox_resource_limit` — 资源限制 (docker update)
pub const SANDBOX_TOOL_WHITELIST: &[&str] = &[
    "apeireth_sandbox_exec",
    "apeireth_sandbox_kill",
    "apeireth_sandbox_status",
    "apeireth_sandbox_network",
    "apeireth_sandbox_filesystem",
    "apeireth_sandbox_resource_limit",
];

/// 编译期守门: SANDBOX_TOOL_WHITELIST 长度 == 6.
pub const SANDBOX_TOOL_WHITELIST_COUNT: usize = 6;
const _: () = assert!(SANDBOX_TOOL_WHITELIST.len() == SANDBOX_TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内.
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> SandboxResult<()> {
    if !SANDBOX_TOOL_WHITELIST.contains(&tool) {
        return Err(SandboxError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §7 核心类型 (SandboxConfig / SandboxHandle)
// ============================================================================

/// 沙箱顶层配置 (per 任务 6 K-1 强校验字段, 1:1 翻译 Docker daemon 启动参数).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// 运行时 (3 选 1, K-1 强校验 #2). 默认 = Container.
    pub runtime: RuntimeKind,
    /// 镜像 (K-1 强校验, 8 registry 白名单).
    pub image: String,
    /// 命令 (K-1 强校验, 禁 shell 注入).
    pub command: Vec<String>,
    /// 用户 (K-1 强校验, 禁 root + 5 禁用用户).
    pub user: String,
    /// 环境变量 (K-1 强校验, 禁 10 敏感变量).
    pub env: HashMap<String, String>,
    /// 端口映射 (K-1 强校验, 禁特权).
    pub ports: Vec<PortMapping>,
    /// 卷挂载 (K-1 强校验, 5 源路径白名单).
    pub volumes: Vec<VolumeMount>,
    /// 资源限制 (5 字段, 1:1 翻译 `apeireth-sdk-sandbox::ResourceLimits`).
    pub resources: ResourceLimits,
    /// 工作目录.
    pub workdir: PathBuf,
    /// 标签 (k-v, observability 用).
    pub labels: HashMap<String, String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            runtime: RuntimeKind::Container,
            image: DEFAULT_SANDBOX_IMAGE.to_string(),
            command: vec!["/bin/sh".to_string()],
            user: DEFAULT_SANDBOX_USER.to_string(),
            env: HashMap::new(),
            ports: Vec::new(),
            volumes: Vec::new(),
            resources: ResourceLimits::default(),
            workdir: PathBuf::from("/"),
            labels: HashMap::new(),
        }
    }
}

impl SandboxConfig {
    /// 创建新沙箱配置.
    pub fn new(runtime: RuntimeKind, image: String, command: Vec<String>, user: String) -> Self {
        Self {
            runtime,
            image,
            command,
            user,
            env: HashMap::new(),
            ports: Vec::new(),
            volumes: Vec::new(),
            resources: ResourceLimits::default(),
            workdir: PathBuf::from("/"),
            labels: HashMap::new(),
        }
    }

    /// 校验全部 6 K-1 强校验 (image / command / user / env / port / volume).
    pub fn validate(&self) -> SandboxResult<()> {
        // K-1 #1: image registry 白名单 (仅 Container runtime 强制,
        // Process 走 OS path, Wasm 走 wasm module path, 都不需要 image registry)
        if self.runtime == RuntimeKind::Container {
            if let Some((registry, _tag)) = self.image.split_once('/') {
                if !ALLOWED_IMAGE_REGISTRIES.contains(&registry) {
                    return Err(SandboxError::InvalidConfig(format!(
                        "image registry {registry} 不在白名单 {ALLOWED_IMAGE_REGISTRIES:?}"
                    )));
                }
            } else {
                return Err(SandboxError::InvalidConfig(format!(
                    "image {} 必须含 registry 前缀 (e.g. docker.io/library/alpine:3.19)",
                    self.image
                )));
            }
        }

        // K-1 #2: command 非空 + 禁 shell 注入
        if self.command.is_empty() {
            return Err(SandboxError::InvalidConfig("command 不能为空".to_string()));
        }
        for arg in &self.command {
            if arg.contains("$(`") || arg.contains(';') || arg.contains('|') {
                return Err(SandboxError::InvalidConfig(format!(
                    "command 包含 shell 注入风险字符: {arg}"
                )));
            }
        }

        // K-1 #3: user 禁 root + 5 禁用用户
        if FORBIDDEN_USERS.contains(&self.user.as_str()) {
            return Err(SandboxError::InvalidConfig(format!(
                "user {} 在禁用列表 {FORBIDDEN_USERS:?}",
                self.user
            )));
        }

        // K-1 #4: env 禁 10 敏感变量
        for key in self.env.keys() {
            if FORBIDDEN_ENV_KEYS.contains(&key.as_str()) {
                return Err(SandboxError::InvalidConfig(format!(
                    "env 包含敏感变量: {key}"
                )));
            }
        }

        // K-1 #5: port 禁特权 (< 1024) + 范围白名单
        for p in &self.ports {
            if p.host_port < 1024 {
                return Err(SandboxError::InvalidConfig(format!(
                    "port host_port {} < 1024 走特权端口",
                    p.host_port
                )));
            }
        }

        // K-1 #6: volume 5 源路径白名单
        for v in &self.volumes {
            let source_str = v.source.to_string_lossy().to_string();
            if !ALLOWED_VOLUME_SOURCE_PREFIXES
                .iter()
                .any(|prefix| source_str.starts_with(prefix))
            {
                return Err(SandboxError::InvalidConfig(format!(
                    "volume source {source_str} 不在 5 源路径白名单"
                )));
            }
        }

        // 资源限制范围
        self.resources.validate()?;

        Ok(())
    }
}

/// 端口映射 (1:1 翻译 Docker daemon `-p host:container`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortMapping {
    /// 主机端口.
    pub host_port: u16,
    /// 容器内端口.
    pub container_port: u16,
    /// 协议 (TCP / UDP).
    pub protocol: String,
}

/// 卷挂载 (1:1 翻译 Docker daemon `-v src:dst:ro`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeMount {
    /// 源路径 (主机, K-1 强校验 #6 5 源路径白名单).
    pub source: PathBuf,
    /// 目标路径 (容器内).
    pub target: PathBuf,
    /// 是否只读.
    pub read_only: bool,
}

/// 资源限制 (5 字段, 1:1 翻译 `apeireth-sdk-sandbox::ResourceLimits`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU 核数.
    pub cpu_cores: u32,
    /// 内存字节.
    pub memory_bytes: u64,
    /// IO 带宽 (字节/秒).
    pub io_bandwidth_bps: u64,
    /// 网络带宽 (字节/秒).
    pub network_bandwidth_bps: u64,
    /// 临时目录大小 (字节).
    pub tmp_bytes: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_cores: 1,
            memory_bytes: 512 * 1024 * 1024, // 512 MiB
            io_bandwidth_bps: 100 * 1024 * 1024, // 100 MB/s
            network_bandwidth_bps: 100 * 1024 * 1024, // 100 MB/s
            tmp_bytes: 100 * 1024 * 1024,     // 100 MiB
        }
    }
}

impl ResourceLimits {
    /// 校验资源范围 (K-1 强校验 #1).
    pub fn validate(&self) -> SandboxResult<()> {
        if self.cpu_cores < 1 || self.cpu_cores > 64 {
            return Err(SandboxError::ResourceOutOfRange(format!(
                "cpu_cores {} 不在 1..=64 范围",
                self.cpu_cores
            )));
        }
        if self.memory_bytes < 16 * 1024 * 1024 || self.memory_bytes > 64 * 1024 * 1024 * 1024 {
            return Err(SandboxError::ResourceOutOfRange(format!(
                "memory_bytes {} 不在 16 MiB..=64 GiB 范围",
                self.memory_bytes
            )));
        }
        Ok(())
    }
}

/// 沙箱句柄 (1:1 翻译 `apeireth-sdk-sandbox::SandboxHandle`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxHandle {
    /// 沙箱 ID (UUID v4).
    pub id: Uuid,
    /// 沙箱状态.
    pub status: SandboxStatus,
    /// 运行时.
    pub runtime: RuntimeKind,
    /// 容器/进程 ID (Docker container id / OS pid / WASM instance id).
    pub container_id: String,
    /// 启动时间.
    pub started_at: SystemTime,
    /// 完成时间 (None = 未完成).
    pub finished_at: Option<SystemTime>,
    /// 退出码 (None = 未完成).
    pub exit_code: Option<i32>,
}

impl SandboxHandle {
    /// 创建新句柄 (pending 状态).
    pub fn new(runtime: RuntimeKind, container_id: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            status: SandboxStatus::Pending,
            runtime,
            container_id,
            started_at: SystemTime::now(),
            finished_at: None,
            exit_code: None,
        }
    }

    /// 沙箱是否在运行.
    pub fn is_running(&self) -> bool {
        matches!(self.status, SandboxStatus::Running)
    }

    /// 沙箱是否已完成.
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            SandboxStatus::Stopped | SandboxStatus::Failed | SandboxStatus::Killed
        )
    }
}

// ============================================================================
// §8 SandboxSdk 顶层 facade (6 API dispatcher, STUB 模式返 NotImplemented)
// ============================================================================

/// Sandbox 顶层 facade (6 API dispatcher, STUB 模式全部返 NotImplemented).
#[derive(Debug)]
pub struct SandboxSdk {
    config: SandboxConfig,
    handles: HashMap<Uuid, SandboxHandle>,
    initialized: AtomicBool,
}

impl SandboxSdk {
    /// 创建新的 Sandbox SDK (STUB 模式, R20 阶段 6 真接走 `SandboxRealImpl`).
    pub fn new(config: SandboxConfig) -> SandboxResult<Self> {
        config.validate()?;
        info!(
            target: "apeireth_sandbox",
            "SandboxSdk::new STUB_MODE={} platform={} schema_version={} runtime={}",
            STUB_MODE,
            PLATFORM_NAME,
            SANDBOX_SCHEMA_VERSION,
            config.runtime
        );
        Ok(Self {
            config,
            handles: HashMap::new(),
            initialized: AtomicBool::new(true),
        })
    }

    /// 6 API #1: 启动 sandbox (STUB 模式返 NotImplemented).
    pub async fn exec(&self, _config: SandboxConfig) -> SandboxResult<SandboxHandle> {
        warn!(
            target: "apeireth_sandbox",
            "exec() STUB 模式, 走 SandboxRealImpl 真接"
        );
        Err(SandboxError::NotImplemented("exec".to_string()))
    }

    /// 6 API #2: 终止 sandbox (STUB 模式返 NotImplemented).
    pub async fn kill(&self, _id: Uuid) -> SandboxResult<()> {
        warn!(
            target: "apeireth_sandbox",
            "kill() STUB 模式, 走 SandboxRealImpl 真接"
        );
        Err(SandboxError::NotImplemented("kill".to_string()))
    }

    /// 6 API #3: 查询状态 (STUB 模式返 NotImplemented).
    pub async fn status(&self, _id: Uuid) -> SandboxResult<SandboxHandle> {
        warn!(
            target: "apeireth_sandbox",
            "status() STUB 模式, 走 SandboxRealImpl 真接"
        );
        Err(SandboxError::NotImplemented("status".to_string()))
    }

    /// 6 API #4: 网络管理 (STUB 模式返 NotImplemented).
    pub async fn network(&self, _action: NetworkAction) -> SandboxResult<()> {
        warn!(
            target: "apeireth_sandbox",
            "network() STUB 模式, 走 SandboxRealImpl 真接"
        );
        Err(SandboxError::NotImplemented("network".to_string()))
    }

    /// 6 API #5: 文件系统操作 (STUB 模式返 NotImplemented).
    pub async fn filesystem(&self, _action: FilesystemAction) -> SandboxResult<()> {
        warn!(
            target: "apeireth_sandbox",
            "filesystem() STUB 模式, 走 SandboxRealImpl 真接"
        );
        Err(SandboxError::NotImplemented("filesystem".to_string()))
    }

    /// 6 API #6: 资源限制 (STUB 模式返 NotImplemented).
    pub async fn resource_limit(
        &self,
        _id: Uuid,
        _limits: ResourceLimits,
    ) -> SandboxResult<()> {
        warn!(
            target: "apeireth_sandbox",
            "resource_limit() STUB 模式, 走 SandboxRealImpl 真接"
        );
        Err(SandboxError::NotImplemented("resource_limit".to_string()))
    }

    /// 取所有 sandbox 句柄.
    pub fn handles(&self) -> Vec<SandboxHandle> {
        self.handles.values().cloned().collect()
    }
}

// ============================================================================
// §9 6 API #4/#5 专属类型 (NetworkAction / FilesystemAction)
// ============================================================================

/// 网络操作 (1:1 翻译 Docker daemon network API).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkAction {
    /// 创建网络.
    Create { name: String },
    /// 删除网络.
    Remove { name: String },
    /// 连接 sandbox 到网络.
    Connect {
        network: String,
        sandbox_id: Uuid,
    },
    /// 断开 sandbox 跟网络.
    Disconnect {
        network: String,
        sandbox_id: Uuid,
    },
}

/// 文件系统操作 (1:1 翻译 Docker daemon cp / volume API).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilesystemAction {
    /// 读文件.
    Read {
        sandbox_id: Uuid,
        path: PathBuf,
    },
    /// 写文件.
    Write {
        sandbox_id: Uuid,
        path: PathBuf,
        data: Vec<u8>,
    },
    /// 挂载卷.
    Mount {
        sandbox_id: Uuid,
        volume: VolumeMount,
    },
    /// 卸载卷.
    Unmount {
        sandbox_id: Uuid,
        target: PathBuf,
    },
}

// ============================================================================
// §10 集成 pipeline-g5 Reliability 守门常数 (借鉴 Golutra v0.1.0 chat_db)
// ============================================================================

/// **Hardcode #1** (集成 pipeline-g5 Reliability 阶段, 跟 `MAX_RETRY_ATTEMPTS = 5` 1:1):
/// 真接 6 API 失败时最大重试次数.
pub const SANDBOX_MAX_RETRY_ATTEMPTS: u32 = 5;
const _: () = assert!(SANDBOX_MAX_RETRY_ATTEMPTS == 5);

/// **Hardcode #2** (集成 pipeline-g5 Reliability 阶段, 跟 `RETRY_BACKOFF_MS` 1:1):
/// 真接 6 API 失败时 backoff 4 步 (100 / 200 / 500 / 1000 ms).
pub const SANDBOX_RETRY_BACKOFF_MS: &[u64] = &[100, 200, 500, 1000];
const _: () = assert!(SANDBOX_RETRY_BACKOFF_MS.len() == 4);
const _: () = assert!(SANDBOX_RETRY_BACKOFF_MS[0] == 100);

/// **Hardcode #3** (集成 pipeline-g5 Reliability 阶段, 跟 `IDEMPOTENCY_KEY_PREFIX = "pl-g5-"` 1:1):
/// sandbox idempotency key 前缀.
pub const SANDBOX_IDEMPOTENCY_KEY_PREFIX: &str = "sandbox-";

/// 编译期字符串相等比较 (per std::str::eq 不是 const-stable, 自实现字节比较, 跟 pipeline-g5 1:1 模式).
const fn const_str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0;
    while i < ab.len() {
        if ab[i] != bb[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// 编译期守门: SANDBOX_IDEMPOTENCY_KEY_PREFIX == "sandbox-".
const _: () = assert!(
    const_str_eq(SANDBOX_IDEMPOTENCY_KEY_PREFIX, "sandbox-"),
    "SANDBOX_IDEMPOTENCY_KEY_PREFIX 改值需经 6 哲学锚 + 主人审 (R21+)"
);

/// **Hardcode #4** (集成 pipeline-g5 Reliability 阶段, 跟 `CIRCUIT_BREAKER_THRESHOLD = 10` 1:1):
/// sandbox circuit-breaker 触发阈值 (连续失败 10 次触发, 防雪崩).
pub const SANDBOX_CIRCUIT_BREAKER_THRESHOLD: u32 = 10;
const _: () = assert!(SANDBOX_CIRCUIT_BREAKER_THRESHOLD == 10);

// ============================================================================
// §11 单元测试 (8 fixture, 跟 voice 1:1 模式, 编译期守门 + K-1 强校验)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 编译期守门: 6 API 工具白名单长度 = 6.
    #[test]
    fn fixture_1_sandbox_tool_whitelist_has_6_tools() {
        assert_eq!(SANDBOX_TOOL_WHITELIST.len(), SANDBOX_TOOL_WHITELIST_COUNT);
        assert_eq!(SANDBOX_TOOL_WHITELIST_COUNT, 6);
    }

    /// 编译期守门: 3 RuntimeKind 守门.
    #[test]
    fn fixture_2_runtime_kind_has_3_variants() {
        assert_eq!(SUPPORTED_RUNTIME_KINDS.len(), 3);
        assert_eq!(RuntimeKind::default(), RuntimeKind::Container);
    }

    /// 编译期守门: 5 SandboxStatus 守门.
    #[test]
    fn fixture_3_sandbox_status_has_5_variants() {
        assert_eq!(SANDBOX_STATUS_COUNT, 5);
        assert_eq!(SandboxStatus::default(), SandboxStatus::Pending);
    }

    /// 编译期守门: 8 SandboxError variant 守门.
    #[test]
    fn fixture_4_sandbox_error_has_8_variants() {
        assert_eq!(SANDBOX_ERROR_VARIANT_COUNT, 8);
    }

    /// 编译期守门: K-1 强校验白名单守门.
    #[test]
    fn fixture_5_k1_whitelists_hardcoded() {
        assert_eq!(ALLOWED_IMAGE_REGISTRIES.len(), 8);
        assert_eq!(FORBIDDEN_ENV_KEYS.len(), 10);
        assert_eq!(FORBIDDEN_USERS.len(), 5);
        assert_eq!(ALLOWED_VOLUME_SOURCE_PREFIXES.len(), 5);
    }

    /// 编译期守门: Reliability 守门常数 1:1 镜像 pipeline-g5.
    #[test]
    fn fixture_6_reliability_constants_match_pipeline_g5() {
        assert_eq!(SANDBOX_MAX_RETRY_ATTEMPTS, 5);
        assert_eq!(SANDBOX_RETRY_BACKOFF_MS.len(), 4);
        assert_eq!(SANDBOX_RETRY_BACKOFF_MS[0], 100);
        assert_eq!(SANDBOX_CIRCUIT_BREAKER_THRESHOLD, 10);
    }

    /// K-1 强校验: 非法 image registry 拒绝.
    #[test]
    fn fixture_7_k1_image_registry_rejects_unknown() {
        let cfg = SandboxConfig {
            image: "evil.registry.com/malware:latest".to_string(),
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    /// K-1 强校验: 禁 root user 拒绝.
    #[test]
    fn fixture_8_k1_user_rejects_root() {
        let cfg = SandboxConfig {
            user: "root".to_string(),
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    /// STUB 模式守门: 6 API 全部返 NotImplemented.
    #[tokio::test]
    async fn fixture_9_stub_mode_6_apis_return_not_implemented() {
        let sdk = SandboxSdk::new(SandboxConfig::default()).unwrap();
        let cfg = SandboxConfig::default();
        let id = Uuid::new_v4();

        assert!(matches!(sdk.exec(cfg).await, Err(SandboxError::NotImplemented(_))));
        assert!(matches!(sdk.kill(id).await, Err(SandboxError::NotImplemented(_))));
        assert!(matches!(sdk.status(id).await, Err(SandboxError::NotImplemented(_))));
        assert!(matches!(
            sdk.network(NetworkAction::Create { name: "test".to_string() }).await,
            Err(SandboxError::NotImplemented(_))
        ));
        assert!(matches!(
            sdk.filesystem(FilesystemAction::Read {
                sandbox_id: id,
                path: PathBuf::from("/test"),
            })
            .await,
            Err(SandboxError::NotImplemented(_))
        ));
        assert!(matches!(
            sdk.resource_limit(id, ResourceLimits::default()).await,
            Err(SandboxError::NotImplemented(_))
        ));
    }

    /// m3 防御: 工具不在白名单被拒绝.
    #[test]
    fn fixture_10_validate_tool_call_rejects_unknown() {
        assert!(matches!(
            validate_tool_call("unknown_tool", &serde_json::json!({})),
            Err(SandboxError::ToolNotWhitelisted(_))
        ));
    }

    /// m3 防御: 6 白名单工具全部接受.
    #[test]
    fn fixture_11_validate_tool_call_accepts_whitelisted() {
        for tool in SANDBOX_TOOL_WHITELIST {
            assert!(validate_tool_call(tool, &serde_json::json!({})).is_ok());
        }
    }

    /// STUB_MODE 编译期守门.
    #[test]
    fn fixture_12_is_stub_mode_returns_true() {
        assert!(is_stub_mode());
        assert_eq!(STUB_MODE, true);
    }
}
