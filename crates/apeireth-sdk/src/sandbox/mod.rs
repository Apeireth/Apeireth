//! # apeireth-sdk-sandbox (STUB MODE)
//!
//! ⚠️ **STUB MODE: R20 阶段 4 效果, 修改需经 6 哲学锚 + 主人审**
//!
//! Sandbox SDK skeleton (1:1 翻译 v0.9.21 商业版 `@anthropic-ai/sandbox` 进程隔离 / 资源
//! 限制 / 安全策略 API 表面, per `commercial-nsis/v0901/app-64/app-extracted/node_modules/
//! @anthropic-ai/` 实查). 商业版 bundle 实查 sandbox 仅有 deps 声明, 未实接 (R21+ 估补),
//! 6 核心 API (spawn / kill / wait / getStatus / streamLogs / cleanup) 1:1 翻译:
//!
//! - **spawn** 创建沙箱 (image + command + user + env + ports + mounts, K-1 强校验 6 字段)
//! - **kill** 终止运行中沙箱 (per v0.9.21 商业版 `kill` 字段)
//! - **wait** 等待沙箱退出 (per v0.9.21 商业版 `wait` 字段, 返 exit code)
//! - **getStatus** 查询状态 (6 状态机: pending / creating / running / stopping / stopped / failed)
//! - **streamLogs** 流式日志 (per v0.9.21 商业版 `streamLogs` 字段, async stream)
//! - **cleanup** 释放资源 (per v0.9.21 商业版 `cleanup` 字段, 删 volume / 关 network)
//!
//! 3 运行时 (K-1 强校验 #2): Docker / Firecracker / gVisor.
//! 3 隔离级别 (K-1 强校验 #3): Process / Container / Vm.
//! 5 资源限制 (K-1 强校验 #1): CPU 核数 / 内存字节 / IO 带宽 / 网络带宽 / 临时目录.
//! 6 K-1 强校验: 镜像名 / 命令 / user / env / 端口 / 卷挂载.
//!
//! **STUB MODE 守门** (per task spec, 0 改):
//! - 任何真实 SDK 引用禁止 (0 引, bollard / firecracker-rs / runsc 都不引)
//! - 6 API 全部返 `SandboxError::NotImplemented(api_name)`, 编译期 hardcode
//! - `STUB_MODE` 编译期 hardcode = `true`, **不允许运行时配置"切到真实模式"**
//! - 真实实现留 **R21+**, 修改本 crate 需 6 哲学锚 (S-1/S-2/O-2/O-3/O-4/O-5) + 主人审
//!
//! ## 状态: ⏳ STUB skeleton (R20 阶段 4 效果, 主人 2026-08-05 派 #X sub-agent 干)
//!
//! ---
//!
//! ## 🧭 6 哲学锚 (RIVAL 蓝图, R20 阶段 4 必守)
//!
//! 1. **S-1 不漂移 (Stay Grounded)**: 0 假装已实现, STUB 模式所有 6 API 全部返
//!    `NotImplemented`. 真实实现留 R21+, 改 STUB_MODE = false 需主人审.
//! 2. **S-2 编译期 hardcode**: `STUB_MODE = true` / `PLATFORM_NAME = "apeireth"` /
//!    `SANDBOX_SCHEMA_VERSION = "1"` 全部 const, 不允许运行时配置覆盖.
//! 3. **O-2 工程铁律 (不引重复造轮子的 dep)**: 0 引 bollard / firecracker-rs / runsc,
//!    留 R21 真接时再加, 现阶段 (R20 阶段 4) 编译期 hardcode 守门.
//! 4. **O-3 m3 防御**: 6 API 工具白名单 `SANDBOX_TOOL_WHITELIST` 编译期 hardcode,
//!    `validate_tool_call` 在 dispatch 前 schema 校验, 防 m3 模型幻觉调用不存在的工具.
//! 5. **O-4 不假装可观测**: 6 API 失败时返 `SandboxError::NotImplemented(api_name)` +
//!    `tracing::warn!` log, 不假装 OK, 不假装是 mock 输出.
//! 6. **O-5 K-1 强校验**: 6 字段 (镜像 / 命令 / user / env / 端口 / 卷挂载) 编译期
//!    hardcode 白名单, 任何配置变更必经 `validate()` 走 6 K-1 检查, 防止恶意/越权配置.
//!
//! ## 🔒 8 项不修改承诺 (per task spec, 跟 apeireth-voice / apeireth-lark 1:1 风格)
//!
//! 1. `version.workspace = true` ✅
//! 2. `edition.workspace = true` ✅
//! 3. `rust-version.workspace = true` ✅
//! 4. `license.workspace = true` ✅
//! 5. `authors.workspace = true` ✅
//! 6. deps 用 `{ workspace = true }` (除 tracing 显式因 workspace 没声明) ✅
//! 7. 不修改 workspace Cargo.toml (由整合 #X sub-agent 加 member) ⏳
//! 8. 不引 unsafe (workspace `#![deny(unsafe_code)]` 继承) ✅
//!
//! ## 🔐 5 K-1 强校验守门字样 (per task spec, 编译期 hardcode 必出现)
//!
//! 1. **apeireth** (品牌一致, 编译期 hardcode `PLATFORM_NAME`)
//! 2. **sandbox** (crate 域名, 模块路径 / TOOL_WHITELIST 命名空间)
//! 3. **stub** (STUB 模式守门字样, `STUB_MODE` const + 6 API 返 NotImplemented)
//! 4. **runtime** (3 RuntimeKind enum, K-1 强校验 #2)
//! 5. **must-do** (整合 #X sub-agent 改 STUB_MODE = false 前必读守门)

#![allow(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 Module 声明
// ============================================================================

pub mod error;
pub mod isolation;
pub mod policy;
pub mod resource;
pub mod runtime;

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

// P0 协议归一化 (per apeireth-protocol, sandbox log stream 错误用 ProtocolError).
use apeireth_protocol::ProtocolError as SandboxProtocolError;
// P0 凭证安全 (per apeireth-keyring, image pull credentials 走 keyring 而非明文).
use apeireth_host::keyring::{KeyringConfig, Platform as KeyringPlatform, SecretBytes, TokenType};

pub use error::{SandboxError, SandboxResult, SANDBOX_ERROR_VARIANT_COUNT};
pub use isolation::{IsolationConfig, SandboxRuntime, StubSandboxRuntime};
pub use policy::{
    PortMapping, PortProtocol, SecurityPolicy, VolumeMount, ALLOWED_IMAGE_REGISTRIES,
    ALLOWED_VOLUME_SOURCE_PREFIXES, FORBIDDEN_ENV_KEYS, FORBIDDEN_USERS, MAX_ENV_VARS,
    MAX_PORT_MAPPINGS, MAX_VOLUME_MOUNTS,
};
pub use resource::{
    ResourceLimits, ResourceUsage, MAX_CPU_CORES, MAX_IO_BANDWIDTH_BPS, MAX_MEMORY_BYTES,
    MAX_NET_BANDWIDTH_BPS, MAX_TMP_BYTES, MIN_CPU_CORES, MIN_IO_BANDWIDTH_BPS, MIN_MEMORY_BYTES,
    MIN_NET_BANDWIDTH_BPS, MIN_TMP_BYTES,
};
pub use runtime::{
    IsolationLevel, RuntimeKind, SandboxStatus, SANDBOX_STATUS_COUNT, SUPPORTED_ISOLATION_LEVELS,
    SUPPORTED_RUNTIME_KINDS,
};

// ============================================================================
// §1 m3 hallucination 防御 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 6 工具 (6 商业版 API), validate_tool_call 在 dispatch 前
// schema 校验. 防止 minimax m3 模型幻觉调用不存在的 sandbox 工具.
// ============================================================================

/// m3 防御: Sandbox SDK 6 API 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// **6 工具 = 1:1 翻译 v0.9.21 商业版 @anthropic-ai/sandbox `spawn / kill / wait /
/// getStatus / streamLogs / cleanup`**:
/// - `apeireth_sdk_sandbox_spawn` (创建沙箱)
/// - `apeireth_sdk_sandbox_kill` (终止沙箱)
/// - `apeireth_sdk_sandbox_wait` (等待退出)
/// - `apeireth_sdk_sandbox_get_status` (查状态)
/// - `apeireth_sdk_sandbox_stream_logs` (流日志)
/// - `apeireth_sdk_sandbox_cleanup` (释放资源)
pub const SANDBOX_TOOL_WHITELIST: &[&str] = &[
    "apeireth_sdk_sandbox_spawn",
    "apeireth_sdk_sandbox_kill",
    "apeireth_sdk_sandbox_wait",
    "apeireth_sdk_sandbox_get_status",
    "apeireth_sdk_sandbox_stream_logs",
    "apeireth_sdk_sandbox_cleanup",
];

/// 编译期守门: SANDBOX_TOOL_WHITELIST 长度 == 6 (K-1 强校验 + 8 项不修改承诺 #5).
pub const SANDBOX_TOOL_WHITELIST_COUNT: usize = 6;
const _: () = assert!(SANDBOX_TOOL_WHITELIST.len() == SANDBOX_TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `SandboxError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> SandboxResult<()> {
    if !SANDBOX_TOOL_WHITELIST.contains(&tool) {
        return Err(SandboxError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §2 编译期 hardcode 常量 (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// Sandbox API schema version (1:1 翻译 @anthropic-ai/sandbox 商业版 v0.9.21, K-1 强校验).
pub const SANDBOX_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #1: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译, 不写 "SpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// **STUB MODE 守门标志** (K-1 强校验 #4): 编译期 hardcode = `true`.
/// R21+ 真接 docker/firecracker/gvisor 时, **必须经 6 哲学锚 + 主人审才能改 `false`**.
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必须 == true (per STUB MODE 守门 + 8 项不修改承诺).
/// 改 false 需同时改本 assert + STUB_MODE 标志, 强行提醒 reviewer.
const _: () = assert!(
    STUB_MODE == true,
    "STUB_MODE 改 false 需经 6 哲学锚 + 主人审 (R21+)"
);

/// m3 防御: 查 STUB_MODE 状态 (per task spec 守门).
/// **R21+ 改 `STUB_MODE = false` 时, 本函数返 `false`**; 现阶段恒返 `true`.
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// 单沙箱最大存活时间 (秒, 1h, per v0.9.21 商业版估, 防恶意沙箱长占资源).
pub const SANDBOX_MAX_LIFETIME_SECONDS: u64 = 3600;

/// 单次 streamLogs 最大 chunk 数 (per v0.9.21 商业版估 10000, 防 stream 爆炸).
pub const SANDBOX_MAX_LOG_CHUNKS: u64 = 10_000;

/// 单 chunk 字节上限 (4 KiB, per v0.9.21 商业版估, 防单 log line 爆炸).
pub const SANDBOX_MAX_LOG_CHUNK_BYTES: usize = 4096;

/// 默认隔离级别 (per v0.9.21 商业版 `isolation: "container"` 默认).
pub const DEFAULT_ISOLATION_LEVEL: IsolationLevel = IsolationLevel::Container;

/// 默认运行时 (per v0.9.21 商业版 `runtime: "docker"` 默认).
pub const DEFAULT_RUNTIME_KIND: RuntimeKind = RuntimeKind::Docker;

// ============================================================================
// §3 核心类型 (SandboxConfig / SandboxHandle / LogStreamEvent / ExitCode)
// ============================================================================

/// 沙箱顶层配置 (per @anthropic-ai/sandbox 商业版 v0.9.21 `SandboxConfig` 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版:
/// - `runtime` → `runtime`
/// - `isolation` → `isolation`
/// - `policy` → `image` + `command` + `user` + `env` + `ports` + `mounts` (拆 SecurityPolicy)
/// - `resources` → `cpuCores` + `memoryBytes` + `ioBandwidthBps` + `networkBandwidthBps` + `tmpBytes`
/// - `credentials` → 走 apeireth-keyring, 不存明文 (P0 凭证安全铁律)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// 运行时 (3 选 1, K-1 强校验 #2). 默认 = Docker.
    pub runtime: RuntimeKind,
    /// 隔离级别 (3 选 1, K-1 强校验 #3). 默认 = Container.
    pub isolation: IsolationLevel,
    /// 隔离配置 (PID/Network/Mount namespace + seccomp + cgroup).
    pub isolation_config: IsolationConfig,
    /// 安全策略 (6 K-1 强校验字段: image / command / user / env / ports / mounts).
    pub policy: SecurityPolicy,
    /// 资源限制 (5 字段: CPU / 内存 / IO / 网络 / 临时目录).
    pub resources: ResourceLimits,
    /// 凭证 (走 apeireth-keyring, 0 明文, P0 安全铁律). None = 公开镜像.
    pub credentials: Option<SandboxCredentials>,
    /// 工作目录 (沙箱内, 默认 "/").
    pub workdir: PathBuf,
    /// 标签 (k-v, 供 filter / observability 用, per v0.9.21 商业版 `labels`).
    pub labels: HashMap<String, String>,
}

/// 沙箱凭证 (走 apeireth-keyring, 0 明文).
///
/// 字段对应 v0.9.21 商业版 `imagePullCredentials.{registry,username,secret}`:
/// - `registry`: 镜像 registry (e.g. "ghcr.io")
/// - `username`: 用户名 (明文 OK, 公开信息)
/// - `secret_ref`: keyring secret ref (e.g. "ghcr-token"), **不存明文**
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxCredentials {
    /// 镜像 registry.
    pub registry: String,
    /// 用户名.
    pub username: String,
    /// keyring secret 名 (走 `apeireth_host::keyring::KeyringStore.get(secret_ref)`).
    pub secret_ref: String,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            runtime: DEFAULT_RUNTIME_KIND,
            isolation: DEFAULT_ISOLATION_LEVEL,
            isolation_config: IsolationConfig {
                level: DEFAULT_ISOLATION_LEVEL,
                runtime: DEFAULT_RUNTIME_KIND,
                pid_namespace: true,
                network_namespace: true,
                mount_namespace: true,
                seccomp_profile: None,
                cgroup_slice: None,
                capabilities: Vec::new(),
            },
            policy: SecurityPolicy::new(
                "docker.io/library/alpine:3.19",
                vec!["/bin/sh".to_string()],
                "apeireth",
            ),
            resources: ResourceLimits::default(),
            credentials: None,
            workdir: PathBuf::from("/"),
            labels: HashMap::new(),
        }
    }
}

impl SandboxConfig {
    /// 创建新沙箱配置 (Builder 风格, 链式调用, 1:1 翻译 v0.9.21 商业版 `new SandboxConfig(...)`).
    pub fn new(
        runtime: RuntimeKind,
        isolation: IsolationLevel,
        policy: SecurityPolicy,
        resources: ResourceLimits,
    ) -> Self {
        Self {
            runtime,
            isolation,
            isolation_config: IsolationConfig {
                level: isolation,
                runtime,
                pid_namespace: true,
                network_namespace: true,
                mount_namespace: true,
                seccomp_profile: None,
                cgroup_slice: None,
                capabilities: Vec::new(),
            },
            policy,
            resources,
            credentials: None,
            workdir: PathBuf::from("/"),
            labels: HashMap::new(),
        }
    }

    /// 校验全部 6 K-1 强校验 + 5 资源限制 + 隔离兼容性.
    pub fn validate(&self) -> SandboxResult<()> {
        self.policy.validate()?;
        self.resources.validate()?;
        self.isolation_config.validate()?;
        if self.isolation_config.level != self.isolation {
            return Err(SandboxError::InvalidConfig(format!(
                "isolation level mismatch: config.isolation={:?} vs isolation_config.level={:?}",
                self.isolation, self.isolation_config.level
            )));
        }
        if self.isolation_config.runtime != self.runtime {
            return Err(SandboxError::InvalidConfig(format!(
                "runtime mismatch: config.runtime={:?} vs isolation_config.runtime={:?}",
                self.runtime, self.isolation_config.runtime
            )));
        }
        Ok(())
    }
}

/// 沙箱句柄 (per @anthropic-ai/sandbox 商业版 v0.9.21 `SandboxHandle` 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版:
/// - `id` → `id` (UUID v4)
/// - `status` → `status` (6 状态机)
/// - `runtime` → `runtime`
/// - `isolation` → `isolation`
/// - `started_at` → `startedAt`
/// - `finished_at` → `finishedAt` (Option, 完成才填)
/// - `exit_code` → `exitCode` (Option, 完成才填)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxHandle {
    /// 沙箱 ID (UUID v4, 1:1 翻译 v0.9.21 商业版 `id`).
    pub id: Uuid,
    /// 沙箱状态 (6 状态机).
    pub status: SandboxStatus,
    /// 运行时 (记录 spawn 时选定的, 1:1 翻译 v0.9.21 商业版 `runtime`).
    pub runtime: RuntimeKind,
    /// 隔离级别 (记录 spawn 时选定的, 1:1 翻译 v0.9.21 商业版 `isolation`).
    pub isolation: IsolationLevel,
    /// 启动时间.
    pub started_at: SystemTime,
    /// 完成时间 (None = 未完成).
    pub finished_at: Option<SystemTime>,
    /// 退出码 (None = 未完成, Some(0) = 正常, Some(!0) = 异常).
    pub exit_code: Option<i32>,
    /// 错误信息 (None = 正常, Some(msg) = failed 时填).
    pub error: Option<String>,
}

impl SandboxHandle {
    /// 创建新句柄 (pending 状态).
    pub fn new(runtime: RuntimeKind, isolation: IsolationLevel) -> Self {
        Self {
            id: Uuid::new_v4(),
            status: SandboxStatus::Pending,
            runtime,
            isolation,
            started_at: SystemTime::now(),
            finished_at: None,
            exit_code: None,
            error: None,
        }
    }

    /// 沙箱是否在运行.
    pub fn is_running(&self) -> bool {
        matches!(
            self.status,
            SandboxStatus::Running | SandboxStatus::Creating
        )
    }

    /// 沙箱是否已完成 (stopped 或 failed).
    pub fn is_finished(&self) -> bool {
        matches!(self.status, SandboxStatus::Stopped | SandboxStatus::Failed)
    }
}

/// 日志流 chunk (per @anthropic-ai/sandbox 商业版 v0.9.21 `streamLogs` 1:1 翻译).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogStreamEvent {
    /// 沙箱 ID (跟 SandboxHandle.id 对应).
    pub sandbox_id: Uuid,
    /// 流 ID (UUID v4, 区分多个并发 stream).
    pub stream_id: Uuid,
    /// 流类型 (stdout / stderr, per v0.9.21 商业版 `stream`).
    pub stream: LogStream,
    /// 数据 (字节, 单 chunk ≤ SANDBOX_MAX_LOG_CHUNK_BYTES = 4 KiB).
    pub data: Vec<u8>,
    /// 序列号 (0-based, 客户端可断点续传).
    pub seq: u64,
    /// 时间戳.
    pub timestamp: SystemTime,
}

/// 日志流类型 (per v0.9.21 商业版 `stream: "stdout" | "stderr"`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    /// 标准输出.
    #[default]
    Stdout,
    /// 标准错误.
    Stderr,
}

impl LogStream {
    /// 字符串 (1:1 翻译 v0.9.21 商业版).
    pub fn as_str(&self) -> &'static str {
        match self {
            LogStream::Stdout => "stdout",
            LogStream::Stderr => "stderr",
        }
    }
}

impl std::fmt::Display for LogStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 退出码 (per v0.9.21 商业版 `exitCode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitCode {
    /// 正常退出 (0).
    Ok,
    /// 异常退出 (非 0).
    Failed(i32),
    /// 信号终止 (per v0.9.21 商业版估, 128 + signal).
    Signaled(i32),
    /// 沙箱被 kill 调用主动终止.
    Killed,
    /// OOM 终止 (per v0.9.21 商业版估).
    Oom,
}

impl ExitCode {
    /// 数值 (per v0.9.21 商业版 `exitCode` 字段语义).
    pub fn value(&self) -> i32 {
        match self {
            ExitCode::Ok => 0,
            ExitCode::Failed(code) => *code,
            ExitCode::Signaled(sig) => 128 + sig,
            ExitCode::Killed => 137,
            ExitCode::Oom => 137,
        }
    }
}

// ============================================================================
// §4 SandboxSdk 顶层 facade (6 API stub dispatcher, STUB 模式返 NotImplemented)
// ============================================================================

/// Sandbox SDK 顶层 facade (6 API dispatcher, STUB 模式全部返 NotImplemented).
///
/// 字段对应 v0.9.21 商业版 `SandboxSdk` (估 3 fields):
/// - `config` (per `SandboxConfig`)
/// - `handles` (per `HashMap<Uuid, SandboxHandle>`)
/// - `runtime` (per `Box<dyn SandboxRuntime>`, 真实 runtime stub)
#[derive(Debug)]
pub struct SandboxSdk {
    config: SandboxConfig,
    handles: HashMap<Uuid, SandboxHandle>,
    runtime: StubSandboxRuntime,
    initialized: AtomicBool,
}

impl SandboxSdk {
    /// 创建新的 Sandbox SDK (STUB 模式 OK, R21+ 真接 docker/firecracker/gvisor).
    pub fn new(config: SandboxConfig) -> SandboxResult<Self> {
        config.validate()?;
        let runtime = StubSandboxRuntime::new(config.runtime, config.isolation);
        info!(
            target: "apeireth_sdk_sandbox",
            "SandboxSdk::new STUB_MODE={} platform={} schema_version={} runtime={} isolation={}",
            STUB_MODE,
            PLATFORM_NAME,
            SANDBOX_SCHEMA_VERSION,
            config.runtime,
            config.isolation
        );
        Ok(Self {
            config,
            handles: HashMap::new(),
            runtime,
            initialized: AtomicBool::new(true),
        })
    }

    /// 当前 config.
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// 当前活跃沙箱数.
    pub fn active_sandboxes(&self) -> usize {
        self.handles.values().filter(|h| h.is_running()).count()
    }

    /// 查 handle by id.
    pub fn get_handle(&self, id: &Uuid) -> Option<&SandboxHandle> {
        self.handles.get(id)
    }

    /// 列出全部 handle.
    pub fn list_handles(&self) -> Vec<&SandboxHandle> {
        self.handles.values().collect()
    }

    // ========================================================================
    // §4.1 6 API stub 工具 (per @anthropic-ai/sandbox 商业版 v0.9.21)
    // 每个工具返 `SandboxError::NotImplemented(api_name)`, 编译期 hardcode.
    // R21+ 真接 docker/firecracker/gvisor 时, 替换实现 + 改 STUB_MODE = false.
    // ========================================================================

    /// 工具 1: `apeireth_sdk_sandbox_spawn` (STUB 返 NotImplemented).
    ///
    /// 1:1 翻译 v0.9.21 商业版 `spawn(image, command, options) -> SandboxHandle`.
    /// R21+ 真接: 调 bollard::Docker::create_container (Docker 路径) /
    /// firecracker::VM::start (Firecracker 路径) / runsc::Runsc::exec (gVisor 路径).
    pub async fn spawn(&mut self, _policy: SecurityPolicy) -> SandboxResult<SandboxHandle> {
        warn!(target: "apeireth_sdk_sandbox", "spawn STUB_MODE returning NotImplemented");
        Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_spawn"))
    }

    /// 工具 2: `apeireth_sdk_sandbox_kill` (STUB 返 NotImplemented).
    ///
    /// 1:1 翻译 v0.9.21 商业版 `kill(handle, signal?) -> void`.
    /// R21+ 真接: 调 bollard::Docker::kill / firecracker::VM::stop / runsc::Runsc::kill.
    pub async fn kill(&mut self, _id: &Uuid, _signal: Option<i32>) -> SandboxResult<()> {
        warn!(target: "apeireth_sdk_sandbox", "kill STUB_MODE returning NotImplemented");
        Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_kill"))
    }

    /// 工具 3: `apeireth_sdk_sandbox_wait` (STUB 返 NotImplemented).
    ///
    /// 1:1 翻译 v0.9.21 商业版 `wait(handle, timeout?) -> ExitCode`.
    /// R21+ 真接: tokio::select! + container.wait / VM.wait / runsc.wait.
    pub async fn wait(&self, _id: &Uuid, _timeout_secs: Option<u64>) -> SandboxResult<ExitCode> {
        warn!(target: "apeireth_sdk_sandbox", "wait STUB_MODE returning NotImplemented");
        Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_wait"))
    }

    /// 工具 4: `apeireth_sdk_sandbox_get_status` (STUB 返 NotImplemented).
    ///
    /// 1:1 翻译 v0.9.21 商业版 `getStatus(handle) -> SandboxStatus`.
    /// R21+ 真接: 调 bollard::Docker::inspect_container / firecracker::VM::state / runsc.state.
    pub async fn get_status(&self, _id: &Uuid) -> SandboxResult<SandboxStatus> {
        warn!(target: "apeireth_sdk_sandbox", "get_status STUB_MODE returning NotImplemented");
        Err(SandboxError::NotImplemented(
            "apeireth_sdk_sandbox_get_status",
        ))
    }

    /// 工具 5: `apeireth_sdk_sandbox_stream_logs` (STUB 返 NotImplemented).
    ///
    /// 1:1 翻译 v0.9.21 商业版 `streamLogs(handle) -> AsyncIterator<LogChunk>`.
    /// R21+ 真接: 调 bollard::Docker::logs(stream=true) / firecracker console / runsc logs.
    pub async fn stream_logs(
        &self,
        _id: &Uuid,
    ) -> SandboxResult<Pin<Box<dyn Stream<Item = LogStreamEvent> + Send>>> {
        warn!(target: "apeireth_sdk_sandbox", "stream_logs STUB_MODE returning NotImplemented");
        Err(SandboxError::NotImplemented(
            "apeireth_sdk_sandbox_stream_logs",
        ))
    }

    /// 工具 6: `apeireth_sdk_sandbox_cleanup` (STUB 返 NotImplemented).
    ///
    /// 1:1 翻译 v0.9.21 商业版 `cleanup(handle) -> void`.
    /// R21+ 真接: 调 bollard::Docker::remove_container (含 volume) / firecracker::VM::delete / runsc.delete.
    pub async fn cleanup(&mut self, _id: &Uuid) -> SandboxResult<()> {
        warn!(target: "apeireth_sdk_sandbox", "cleanup STUB_MODE returning NotImplemented");
        Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_cleanup"))
    }
}

// ============================================================================
// §5 STUB 守门宏 + 工具状态 helper
// ============================================================================

/// STUB 守门宏: 用于本地 inline STUB 检查 (1:1 翻译 v0.9.21 商业版 `throwNotImplemented`).
///
/// 用法: `sandbox_stub!("spawn")?;` 在函数体顶部守门, R21+ 真接时整体替换.
/// 现阶段 STUB 模式: 全部返 `SandboxError::NotImplemented`.
#[macro_export]
macro_rules! sandbox_stub {
    ($api:literal) => {
        if $crate::sandbox::STUB_MODE {
            $crate::tracing::warn!(
                target: "apeireth_sdk_sandbox",
                concat!($api, " STUB_MODE returning NotImplemented")
            );
            return ::core::result::Result::Err(
                $crate::sandbox::SandboxError::NotImplemented(concat!("apeireth_sdk_sandbox_", $api)),
            );
        }
    };
}

/// m3 防御: 守 6 stub 工具返 NotImplemented, 防止整合时有人"贴心"接 docker/firecracker
/// 但忘了改 STUB_MODE.
pub fn assert_stub_mode_or_panic(api: &'static str) -> SandboxResult<()> {
    if !STUB_MODE {
        // 真接阶段 (R21+ 后) 这里应该返 `Ok(())`, 工具正常执行.
        // 当前 STUB 模式守门: 任何工具调用都返 NotImplemented.
        return Err(SandboxError::NotImplemented(api));
    }
    Err(SandboxError::NotImplemented(api))
}

// ============================================================================
// §6 async trait SandboxSpawner (R21+ 真接时不同 runtime 各自实现)
// ============================================================================

/// Sandbox spawner async trait (R21+ 真接时不同 runtime 各自实现).
///
/// STUB 模式: trait 表面 1:1 翻译, 默认实现返 NotImplemented, 防止整合时漏防.
#[async_trait]
pub trait SandboxSpawner: Send + Sync {
    /// 沙箱 spawner 类型 (Docker / Firecracker / Gvisor).
    fn kind(&self) -> RuntimeKind;
    /// 实际 spawn (STUB 返 NotImplemented, R21+ 真接 bollard / firecracker / runsc).
    async fn do_spawn(&self, _config: &SandboxConfig) -> SandboxResult<SandboxHandle> {
        Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_spawn"))
    }
    /// 实际 kill (STUB 返 NotImplemented).
    async fn do_kill(&self, _handle: &SandboxHandle, _signal: Option<i32>) -> SandboxResult<()> {
        Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_kill"))
    }
    /// 实际 wait (STUB 返 NotImplemented).
    async fn do_wait(
        &self,
        _handle: &SandboxHandle,
        _timeout: Option<std::time::Duration>,
    ) -> SandboxResult<ExitCode> {
        Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_wait"))
    }
}

/// STUB 默认 spawner (R20 阶段 4 skeleton 阶段, 编译期守 STUB_MODE).
#[derive(Debug, Default)]
pub struct StubSandboxSpawner {
    kind: RuntimeKind,
}

impl StubSandboxSpawner {
    pub fn new(kind: RuntimeKind) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl SandboxSpawner for StubSandboxSpawner {
    fn kind(&self) -> RuntimeKind {
        self.kind
    }
    // do_spawn / do_kill / do_wait 走 trait 默认实现, 全部返 NotImplemented.
}

// ============================================================================
// §7 编译期守门 + 占位扩展点
// ============================================================================

// ⏳ R21+ 真接 sandbox 运行时 (per @anthropic-ai/sandbox 商业版) 时, 这里加:
//   - BollardDockerSpawner (per `bollard::Docker::create_container`)
//   - FirecrackerSpawner (per `firecracker::VM::start`)
//   - GvisorSpawner (per `runsc::Runsc::exec`)
//   - 真实 keyring 凭证加载 (per `apeireth_host::keyring::KeyringStore.get`)
//   - 真实 stream 适配 (per `bollard::Docker::logs(stream=true)`)
//   - 真实 cgroup v2 资源下发 (per `cgroupfs` / systemd-run --scope)
// 当前 STUB 模式: 不引 bollard / firecracker-rs / runsc 任何 crate, 编译期 hardcode
// 守门 STUB_MODE = true.

// ============================================================================
// §8 in-crate 测试 (Fixture 1-5, R20 阶段 4 K-1 强校验 4 条 + 2 额外)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture 1: 编译期 hardcode 守门
    #[test]
    fn sandbox_compile_time_constants_match_k1() {
        assert_eq!(SANDBOX_SCHEMA_VERSION, "1");
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert!(STUB_MODE, "STUB_MODE must be true until R21+");
        assert_eq!(SANDBOX_MAX_LIFETIME_SECONDS, 3600);
        assert_eq!(SANDBOX_MAX_LOG_CHUNKS, 10_000);
        assert_eq!(SANDBOX_MAX_LOG_CHUNK_BYTES, 4096);
        assert_eq!(DEFAULT_ISOLATION_LEVEL, IsolationLevel::Container);
        assert_eq!(DEFAULT_RUNTIME_KIND, RuntimeKind::Docker);
    }

    // Fixture 2: 3 RuntimeKind + 3 IsolationLevel 守门
    #[test]
    fn sandbox_runtime_and_isolation_have_3_each() {
        assert_eq!(
            SUPPORTED_RUNTIME_KINDS.len(),
            3,
            "K-1: must be 3 runtime kinds"
        );
        assert_eq!(
            SUPPORTED_ISOLATION_LEVELS.len(),
            3,
            "K-1: must be 3 isolation levels"
        );
        assert_eq!(SUPPORTED_RUNTIME_KINDS[0], RuntimeKind::Docker);
        assert_eq!(SUPPORTED_RUNTIME_KINDS[1], RuntimeKind::Firecracker);
        assert_eq!(SUPPORTED_RUNTIME_KINDS[2], RuntimeKind::Gvisor);
        assert_eq!(SUPPORTED_ISOLATION_LEVELS[0], IsolationLevel::Process);
        assert_eq!(SUPPORTED_ISOLATION_LEVELS[1], IsolationLevel::Container);
        assert_eq!(SUPPORTED_ISOLATION_LEVELS[2], IsolationLevel::Vm);
        // Round-trip Display
        for r in SUPPORTED_RUNTIME_KINDS {
            let parsed: RuntimeKind = r.to_string().parse().unwrap();
            assert_eq!(parsed, *r);
        }
        for i in SUPPORTED_ISOLATION_LEVELS {
            let parsed: IsolationLevel = i.to_string().parse().unwrap();
            assert_eq!(parsed, *i);
        }
    }

    // Fixture 3: SANDBOX_TOOL_WHITELIST 6 工具名守门
    #[test]
    fn sandbox_tool_whitelist_has_6_tools() {
        assert_eq!(
            SANDBOX_TOOL_WHITELIST.len(),
            6,
            "K-1: must be 6 sandbox tools"
        );
        assert_eq!(SANDBOX_TOOL_WHITELIST_COUNT, 6);
        let expected = [
            "apeireth_sdk_sandbox_spawn",
            "apeireth_sdk_sandbox_kill",
            "apeireth_sdk_sandbox_wait",
            "apeireth_sdk_sandbox_get_status",
            "apeireth_sdk_sandbox_stream_logs",
            "apeireth_sdk_sandbox_cleanup",
        ];
        for tool in expected {
            assert!(
                SANDBOX_TOOL_WHITELIST.contains(&tool),
                "SANDBOX_TOOL_WHITELIST must contain {tool}"
            );
        }
    }

    // Fixture 4: validate_tool_call 接受白名单, 拒绝非白名单
    #[test]
    fn sandbox_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({});
        assert!(validate_tool_call("apeireth_sdk_sandbox_spawn", &args).is_ok());
        assert!(validate_tool_call("apeireth_sdk_sandbox_cleanup", &args).is_ok());
    }

    #[test]
    fn sandbox_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let err = validate_tool_call("apeireth_sdk_sandbox_bogus_tool", &args).unwrap_err();
        assert!(matches!(err, SandboxError::ToolNotWhitelisted(_)));
    }

    // Fixture 5: is_stub_mode 返 true (K-1 强校验 #4 守门)
    #[test]
    fn sandbox_is_stub_mode_returns_true() {
        assert!(is_stub_mode());
        assert_eq!(is_stub_mode(), STUB_MODE);
    }

    // 额外 1: 6 stub 工具返 NotImplemented (体现 stub 模式)
    #[tokio::test]
    async fn sandbox_6_stub_tools_return_not_implemented() {
        let mut sdk = SandboxSdk::new(SandboxConfig::default())
            .expect("SandboxSdk::new must succeed in STUB mode");
        let id = Uuid::new_v4();
        let policy = SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "apeireth",
        );

        // 6 stub 工具必须全部返 SandboxError::NotImplemented
        let r1 = sdk.spawn(policy).await;
        assert!(
            matches!(
                r1,
                Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_spawn"))
            ),
            "spawn must return NotImplemented, got {:?}",
            r1
        );

        let r2 = sdk.kill(&id, None).await;
        assert!(
            matches!(
                r2,
                Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_kill"))
            ),
            "kill must return NotImplemented, got {:?}",
            r2
        );

        let r3 = sdk.wait(&id, None).await;
        assert!(
            matches!(
                r3,
                Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_wait"))
            ),
            "wait must return NotImplemented, got {:?}",
            r3
        );

        let r4 = sdk.get_status(&id).await;
        assert!(
            matches!(
                r4,
                Err(SandboxError::NotImplemented(
                    "apeireth_sdk_sandbox_get_status"
                ))
            ),
            "get_status must return NotImplemented, got {:?}",
            r4
        );

        let r5 = sdk.stream_logs(&id).await;
        assert!(
            matches!(
                r5,
                Err(SandboxError::NotImplemented(
                    "apeireth_sdk_sandbox_stream_logs"
                ))
            ),
            "stream_logs must return NotImplemented (Stream type not Debug)"
        );

        let r6 = sdk.cleanup(&id).await;
        assert!(
            matches!(
                r6,
                Err(SandboxError::NotImplemented("apeireth_sdk_sandbox_cleanup"))
            ),
            "cleanup must return NotImplemented, got {:?}",
            r6
        );
    }

    // 额外 2: SandboxConfig.validate 6 K-1 强校验 1:1 翻译
    #[test]
    fn sandbox_config_validate_6_k1_rules() {
        // 默认 config 应通过校验
        let cfg = SandboxConfig::default();
        assert!(cfg.validate().is_ok());

        // K-1 强校验 #1: image 空拒绝
        let mut bad = cfg.clone();
        bad.policy.image = "".to_string();
        assert!(matches!(bad.validate(), Err(SandboxError::InvalidImage(_))));

        // K-1 强校验 #2: command 空拒绝
        let mut bad = cfg.clone();
        bad.policy.command = vec![];
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidCommand(_))
        ));

        // K-1 强校验 #3: user = root 拒绝
        let mut bad = cfg.clone();
        bad.policy.user = "root".to_string();
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        // K-1 强校验 #4: env 含 LD_PRELOAD 拒绝
        let mut bad = cfg.clone();
        bad.policy
            .env
            .insert("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string());
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        // K-1 强校验 #5: container_port = 0 拒绝
        let mut bad = cfg.clone();
        bad.policy.ports.push(PortMapping {
            host_port: 8080,
            container_port: 0,
            protocol: PortProtocol::Tcp,
        });
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        // K-1 强校验 #6: volume mount 源不在白名单拒绝
        let mut bad = cfg.clone();
        bad.policy.mounts.push(VolumeMount {
            source: PathBuf::from("/etc/passwd"),
            target: PathBuf::from("/mnt/passwd"),
            read_only: true,
        });
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));
    }

    // 额外 3: SandboxHandle 状态机判定
    #[test]
    fn sandbox_handle_state_machine() {
        let mut h = SandboxHandle::new(RuntimeKind::Docker, IsolationLevel::Container);
        assert_eq!(h.status, SandboxStatus::Pending);
        assert!(!h.is_running());
        assert!(!h.is_finished());

        h.status = SandboxStatus::Running;
        assert!(h.is_running());
        assert!(!h.is_finished());

        h.status = SandboxStatus::Stopped;
        h.exit_code = Some(0);
        assert!(!h.is_running());
        assert!(h.is_finished());
        assert_eq!(h.exit_code, Some(0));
    }

    // 额外 4: ExitCode 数值映射 (1:1 翻译 v0.9.21 商业版 `exitCode` 字段语义)
    #[test]
    fn sandbox_exit_code_value_mapping() {
        assert_eq!(ExitCode::Ok.value(), 0);
        assert_eq!(ExitCode::Failed(42).value(), 42);
        assert_eq!(ExitCode::Signaled(9).value(), 137); // SIGKILL
        assert_eq!(ExitCode::Killed.value(), 137);
        assert_eq!(ExitCode::Oom.value(), 137);
    }

    // 额外 5: assert_stub_mode_or_panic 守门
    #[test]
    fn sandbox_assert_stub_mode_guard() {
        let r = assert_stub_mode_or_panic("test_api");
        assert!(matches!(r, Err(SandboxError::NotImplemented("test_api"))));
    }
}
