//! # apeireth-sdk-sandbox — 多沙箱编排客户端协议层
//!
//! 本模块是**多沙箱编排面**的客户端协议层: 创建 / 终止 / 销毁 / 配额记账 /
//! 状态巡检, 全部走结构化协议帧与外部编排服务往返。编排服务本体不在本仓库:
//! 它停在 [`transport::OrchestrationTransport`] 边界之外, 仓库内以
//! [`mock::MockOrchestrationService`] 的 mock 边界提供协议契约级替身
//! (零假装: 替身只承诺"协议契约上服务怎么回答", 不冒充真实运行时)。
//!
//! ## 分层 (每层是独立实现面)
//!
//! | 层 | 模块 | 职责 |
//! |---|---|---|
//! | 协议编解码 | [`protocol`] | 请求/响应帧 JSON 编解码 + schema 版本校验 + 回声校验 |
//! | 错误闭合词表 | [`error`] / [`protocol::WireErrorCode`] | 分类闭合, 未知线上码收口 |
//! | 生命周期状态机 | [`lifecycle`] | 6 态迁移矩阵, 非法迁移一律拒绝 |
//! | 配额记账 | [`quota`] | 并发数 / CPU / 内存准入, 原子落盘 |
//! | 隔离计划 | [`isolation`] | 级别/运行时兼容矩阵 + capability 白名单 |
//! | 传输边界 | [`transport`] / [`mock`] | mock 边界 (协议契约级替身) |
//!
//! 超时统一经 `apeireth_core::deadline` (Deadline 到期通知竞速); 日志走脱敏
//! 原语 (`apeireth_credentials::SecretString`), 凭据/环境变量值不入日志。
//!
//! ## 6 编排 API
//!
//! - [`SandboxSdk::spawn`] — 创建沙箱 (K-1 配置校验 + 隔离计划 + 配额预留)
//! - [`SandboxSdk::kill`] — 终止运行中沙箱
//! - [`SandboxSdk::wait`] — 等待退出 (deadline 超时)
//! - [`SandboxSdk::get_status`] — 状态单查 (本地/服务端对账)
//! - [`SandboxSdk::stream_logs`] — 流式日志 (断点续传, 错误随流传)
//! - [`SandboxSdk::cleanup`] — 销毁并释放资源
//!
//! 外加 [`SandboxSdk::patrol`] — 状态巡检 (批量对账 + 超龄回收 + 配额归还)。

#![allow(missing_docs)]

pub mod error;
pub mod isolation;
pub mod lifecycle;
pub mod mock;
pub mod policy;
pub mod protocol;
pub mod quota;
pub mod resource;
pub mod runtime;
pub mod transport;

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use futures::future::{select, Either};
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use apeireth_core::deadline::{clamp_timeout, Deadline};
use apeireth_credentials::SecretString;

pub use error::{
    SandboxError, SandboxErrorCode, SandboxResult, SANDBOX_ERROR_CODE_COUNT,
    SANDBOX_ERROR_VARIANT_COUNT,
};
pub use isolation::{IsolationConfig, IsolationPlan, ALLOWED_CAPABILITIES};
pub use lifecycle::{
    apply_transition, is_reachable, reconcile, status_rank, LifecycleEvent as SandboxLifecycleEvent,
};
pub use mock::MockOrchestrationService;
pub use policy::{
    PortMapping, PortProtocol, SecurityPolicy, VolumeMount, ALLOWED_IMAGE_REGISTRIES,
    ALLOWED_VOLUME_SOURCE_PREFIXES, FORBIDDEN_ENV_KEYS, FORBIDDEN_USERS, MAX_ENV_VARS,
    MAX_PORT_MAPPINGS, MAX_VOLUME_MOUNTS,
};
pub use protocol::{
    check_response_echo, classify_wire_error_code, decode_request, decode_response, encode_request,
    encode_response, error_response, OpResult, OrchestrationRequest, OrchestrationResponse,
    RequestFrame, ResponseFrame, WireErrorCode, WIRE_SCHEMA_VERSION,
};
pub use quota::{QuotaLedger, QuotaPolicy, QuotaSnapshot};
pub use resource::{
    ResourceLimits, ResourceUsage, MAX_CPU_CORES, MAX_IO_BANDWIDTH_BPS, MAX_MEMORY_BYTES,
    MAX_NET_BANDWIDTH_BPS, MAX_TMP_BYTES, MIN_CPU_CORES, MIN_IO_BANDWIDTH_BPS, MIN_MEMORY_BYTES,
    MIN_NET_BANDWIDTH_BPS, MIN_TMP_BYTES,
};
pub use runtime::{
    IsolationLevel, RuntimeKind, SandboxStatus, SANDBOX_STATUS_COUNT, SUPPORTED_ISOLATION_LEVELS,
    SUPPORTED_RUNTIME_KINDS,
};
pub use transport::{OrchestrationTransport, TransportError};

// ============================================================================
// §1 工具白名单 (调用面防幻觉: 只有 6 个编排 API 可被调用)
// ============================================================================

/// 编排 API 工具白名单 (编译期 hardcode): spawn / kill / wait / get_status /
/// stream_logs / cleanup。
pub const SANDBOX_TOOL_WHITELIST: &[&str] = &[
    "apeireth_sdk_sandbox_spawn",
    "apeireth_sdk_sandbox_kill",
    "apeireth_sdk_sandbox_wait",
    "apeireth_sdk_sandbox_get_status",
    "apeireth_sdk_sandbox_stream_logs",
    "apeireth_sdk_sandbox_cleanup",
];

/// 编译期守门: 白名单长度 == 6。
pub const SANDBOX_TOOL_WHITELIST_COUNT: usize = 6;
const _: () = assert!(SANDBOX_TOOL_WHITELIST.len() == SANDBOX_TOOL_WHITELIST_COUNT);

/// 校验工具调用是否在白名单内; 不在则
/// [`SandboxError::ToolNotWhitelisted`](error::SandboxError::ToolNotWhitelisted)。
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> SandboxResult<()> {
    if !SANDBOX_TOOL_WHITELIST.contains(&tool) {
        return Err(SandboxError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §2 编译期常量
// ============================================================================

/// 沙箱协议 schema 版本。
pub const SANDBOX_SCHEMA_VERSION: &str = "1";

/// 平台名 (编译期 hardcode)。
pub const PLATFORM_NAME: &str = "apeireth";

/// 单沙箱最大存活时间 (秒, 防长占资源; 状态巡检按此回收)。
pub const SANDBOX_MAX_LIFETIME_SECONDS: u64 = 3600;

/// 单次 stream_logs 最大 chunk 数 (防流爆炸)。
pub const SANDBOX_MAX_LOG_CHUNKS: u64 = 10_000;

/// 单 chunk 字节上限 (防单行爆炸)。
pub const SANDBOX_MAX_LOG_CHUNK_BYTES: usize = 4096;

/// 默认隔离级别。
pub const DEFAULT_ISOLATION_LEVEL: IsolationLevel = IsolationLevel::Container;

/// 默认运行时。
pub const DEFAULT_RUNTIME_KIND: RuntimeKind = RuntimeKind::Docker;

/// 默认请求超时 (毫秒, deadline 缺省)。
pub const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 5_000;

/// 请求超时上限 (毫秒, deadline 硬顶)。
pub const MAX_REQUEST_TIMEOUT_MS: u64 = 60_000;

/// 默认 wait 超时 (毫秒)。
pub const DEFAULT_WAIT_TIMEOUT_MS: u64 = 30_000;

/// wait 超时上限 (毫秒 = 单沙箱最大存活时间)。
pub const MAX_WAIT_TIMEOUT_MS: u64 = SANDBOX_MAX_LIFETIME_SECONDS * 1000;

/// stream_logs 单次拉取 chunk 批大小。
pub const LOG_CHUNK_BATCH: u64 = 16;

// ============================================================================
// §3 核心类型 (SandboxConfig / SandboxHandle / LogStreamEvent / ExitCode)
// ============================================================================

/// 沙箱顶层配置 (runtime + isolation + policy + resources + credentials)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// 运行时 (3 选 1)。默认 = Docker。
    pub runtime: RuntimeKind,
    /// 隔离级别 (3 选 1)。默认 = Container。
    pub isolation: IsolationLevel,
    /// 隔离配置 (PID/Network/Mount namespace + seccomp + cgroup)。
    pub isolation_config: IsolationConfig,
    /// 安全策略 (image / command / user / env / ports / mounts)。
    pub policy: SecurityPolicy,
    /// 资源限制 (CPU / 内存 / IO / 网络 / 临时目录)。
    pub resources: ResourceLimits,
    /// 拉镜像凭证 (只存 secret 引用, 不存明文)。None = 公开镜像。
    pub credentials: Option<SandboxCredentials>,
    /// 工作目录 (沙箱内, 默认 "/")。
    pub workdir: PathBuf,
    /// 标签 (k-v, 供 filter / 观测用)。
    pub labels: HashMap<String, String>,
}

/// 沙箱凭证 (secret 只以引用形式存在, 明文经宿主集成现查)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxCredentials {
    /// 镜像 registry。
    pub registry: String,
    /// 用户名 (公开信息)。
    pub username: String,
    /// 宿主集成解析的 secret 引用名 (不存明文)。
    pub secret_ref: String,
}

/// 脱敏: 单个字符串按长度掩码 (`[REDACTED len=N]`), 明文不出现在日志/摘要。
pub fn redact_secret(value: &str) -> String {
    SecretString::new(value.to_string()).redacted()
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
    /// 创建新沙箱配置 (runtime + isolation + policy + resources)。
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

    /// 校验全部配置面: 安全策略 + 资源限制 + 隔离 (含 capability 白名单) + 一致性。
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

    /// 脱敏日志摘要: 凭据/环境变量值一律掩码, 明文不入日志。
    pub fn log_summary(&self) -> String {
        let env_values: Vec<String> = self
            .policy
            .env
            .values()
            .map(|value| redact_secret(value))
            .collect();
        let credentials = self
            .credentials
            .as_ref()
            .map(|creds| format!("{}@{}", creds.username, creds.registry))
            .unwrap_or_else(|| "none".to_string());
        format!(
            "sandbox-config runtime={} isolation={} image={} user={} env_values=[{}] ports={} mounts={} credentials={}",
            self.runtime,
            self.isolation,
            self.policy.image,
            self.policy.user,
            env_values.join(","),
            self.policy.ports.len(),
            self.policy.mounts.len(),
            credentials,
        )
    }
}

/// 沙箱句柄 (客户端与服务端共享同一档案)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxHandle {
    /// 沙箱 ID (客户端分配的 UUID v4)。
    pub id: Uuid,
    /// 沙箱状态 (6 态状态机)。
    pub status: SandboxStatus,
    /// 运行时 (创建时选定)。
    pub runtime: RuntimeKind,
    /// 隔离级别 (创建时选定)。
    pub isolation: IsolationLevel,
    /// 启动时间。
    pub started_at: SystemTime,
    /// 完成时间 (None = 未完成)。
    pub finished_at: Option<SystemTime>,
    /// 退出码 (None = 未完成)。
    pub exit_code: Option<i32>,
    /// 错误信息 (None = 正常)。
    pub error: Option<String>,
}

impl SandboxHandle {
    /// 创建新句柄 (pending 状态)。
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

    /// 沙箱是否在运行 (creating / running)。
    pub fn is_running(&self) -> bool {
        matches!(
            self.status,
            SandboxStatus::Running | SandboxStatus::Creating
        )
    }

    /// 沙箱是否已完成 (stopped / failed)。
    pub fn is_finished(&self) -> bool {
        self.status.is_terminal()
    }
}

/// 日志流 chunk。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogStreamEvent {
    /// 沙箱 ID。
    pub sandbox_id: Uuid,
    /// 流 ID (区分并发流)。
    pub stream_id: Uuid,
    /// 流类型 (stdout / stderr)。
    pub stream: LogStream,
    /// 数据 (单 chunk ≤ [`SANDBOX_MAX_LOG_CHUNK_BYTES`])。
    pub data: Vec<u8>,
    /// 序列号 (0-based, 断点续传游标)。
    pub seq: u64,
    /// 时间戳。
    pub timestamp: SystemTime,
}

/// 日志流类型。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogStream {
    /// 标准输出。
    #[default]
    Stdout,
    /// 标准错误。
    Stderr,
}

impl LogStream {
    /// 稳定字符串。
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

/// 退出码。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitCode {
    /// 正常退出 (0)。
    Ok,
    /// 异常退出 (非 0)。
    Failed(i32),
    /// 信号终止 (128 + signal)。
    Signaled(i32),
    /// 被 kill 主动终止。
    Killed,
    /// OOM 终止。
    Oom,
}

impl ExitCode {
    /// 数值 (与 `exit_code` 字段语义一致)。
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
// §4 状态巡检报告
// ============================================================================

/// 状态巡检报告。
#[derive(Debug, Clone, PartialEq)]
pub struct PatrolReport {
    /// 本次对账的沙箱数。
    pub inspected: usize,
    /// 发生状态迁移 (本地与服务端对齐) 的沙箱数。
    pub transitioned: usize,
    /// 被回收的沙箱 ID (服务端已无记录 / 超龄强制销毁)。
    pub reaped: Vec<Uuid>,
    /// 对账失败数 (传输失败 / 状态不可达)。
    pub failures: usize,
    /// 巡检后配额快照。
    pub quota: QuotaSnapshot,
}

// ============================================================================
// §5 SandboxSdk — 多沙箱编排客户端 (真实实现)
// ============================================================================

/// 多沙箱编排客户端: 协议编解码 + 生命周期对账 + 配额记账 + deadline 超时,
/// 与外部编排服务经 [`OrchestrationTransport`] 往返 (mock 边界见 [`mock`])。
#[derive(Clone)]
pub struct SandboxSdk {
    config: SandboxConfig,
    transport: Arc<dyn OrchestrationTransport>,
    ledger: QuotaLedger,
    handles: HashMap<Uuid, SandboxHandle>,
    request_timeout_ms: u64,
}

impl std::fmt::Debug for SandboxSdk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SandboxSdk")
            .field("config", &self.config.log_summary())
            .field("handles", &self.handles.len())
            .field("quota", &self.ledger.snapshot())
            .field("request_timeout_ms", &self.request_timeout_ms)
            .finish()
    }
}

impl SandboxSdk {
    /// 创建编排客户端 (校验基座配置 + 绑定传输边界与配额账本)。
    pub fn new(
        config: SandboxConfig,
        transport: Arc<dyn OrchestrationTransport>,
        ledger: QuotaLedger,
    ) -> SandboxResult<Self> {
        config.validate()?;
        info!(
            target: "apeireth_sdk_sandbox",
            "SandboxSdk::new platform={} schema_version={} {}",
            PLATFORM_NAME,
            SANDBOX_SCHEMA_VERSION,
            config.log_summary()
        );
        Ok(Self {
            config,
            transport,
            ledger,
            handles: HashMap::new(),
            request_timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
        })
    }

    /// 覆盖请求超时 (毫秒, 经 deadline 过闸)。
    pub fn with_request_timeout_ms(mut self, timeout_ms: u64) -> SandboxResult<Self> {
        clamp_timeout(
            Some(timeout_ms),
            DEFAULT_REQUEST_TIMEOUT_MS,
            MAX_REQUEST_TIMEOUT_MS,
        )?;
        self.request_timeout_ms = timeout_ms;
        Ok(self)
    }

    /// 当前基座配置。
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    /// 配额账本。
    pub fn quota(&self) -> &QuotaLedger {
        &self.ledger
    }

    /// 当前活跃 (未终态) 沙箱数。
    pub fn active_sandboxes(&self) -> usize {
        self.handles.values().filter(|h| h.is_running()).count()
    }

    /// 查本地句柄。
    pub fn get_handle(&self, id: &Uuid) -> Option<&SandboxHandle> {
        self.handles.get(id)
    }

    /// 列出本地全部句柄。
    pub fn list_handles(&self) -> Vec<&SandboxHandle> {
        self.handles.values().collect()
    }

    // ------------------------------------------------------------------
    // §5.1 传输往返 (编码 → 边界 → 解码 → 版本/回声校验 → 错误分类)
    // ------------------------------------------------------------------

    async fn call_op(&self, body: OrchestrationRequest) -> SandboxResult<OpResult> {
        self.call_op_within(body, self.request_timeout_ms).await
    }

    async fn call_op_within(
        &self,
        body: OrchestrationRequest,
        timeout_ms: u64,
    ) -> SandboxResult<OpResult> {
        call_op_on(&self.transport, timeout_ms, body).await
    }

    fn local_handle(&self, id: &Uuid) -> SandboxResult<SandboxHandle> {
        self.handles
            .get(id)
            .cloned()
            .ok_or_else(|| SandboxError::NotFound {
                sandbox_id: id.to_string(),
            })
    }

    fn release_if_reserved(&self, id: &Uuid) {
        if self.ledger.is_reserved(id) {
            // 归还失败只记日志, 不覆盖主错误路径 (账目以快照文件核对)。
            if let Err(err) = self.ledger.release(id) {
                info!(
                    target: "apeireth_sdk_sandbox",
                    "quota release failed for {}: {}", id, err.code()
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // §5.2 6 编排 API
    // ------------------------------------------------------------------

    /// 工具 1: 创建沙箱。
    ///
    /// 流程: 完整配置校验 → 隔离计划 (capability 白名单收敛) → 配额预留 →
    /// 创建请求 (客户端分配沙箱 ID) → 生命周期对账 (pending → 服务端回报)。
    /// 请求失败时配额必归还。
    pub async fn spawn(&mut self, policy: SecurityPolicy) -> SandboxResult<SandboxHandle> {
        let mut config = self.config.clone();
        config.policy = policy;
        config.validate()?;
        let plan = IsolationPlan::plan(&config.isolation_config)?;

        let sandbox_id = Uuid::new_v4();
        self.ledger.reserve(sandbox_id, &config.resources)?;

        let result = self
            .call_op(OrchestrationRequest::Create {
                sandbox_id,
                config: Box::new(config),
            })
            .await;
        let result = match result {
            Ok(result) => result,
            Err(err) => {
                self.release_if_reserved(&sandbox_id);
                return Err(err);
            }
        };
        let OpResult::Created { handle } = result else {
            self.release_if_reserved(&sandbox_id);
            return Err(SandboxError::Protocol(
                "create response did not carry a handle".into(),
            ));
        };
        if handle.id != sandbox_id {
            self.release_if_reserved(&sandbox_id);
            return Err(SandboxError::Protocol(format!(
                "create response handle id mismatch: sent={sandbox_id} got={}",
                handle.id
            )));
        }
        // 生命周期对账: 从 pending 出发, 服务端回报必须可达。
        reconcile(SandboxStatus::Pending, handle.status)?;
        self.handles.insert(sandbox_id, handle.clone());
        info!(
            target: "apeireth_sdk_sandbox",
            "spawned sandbox {} status={} isolation_capabilities={}",
            sandbox_id,
            handle.status,
            plan.granted_capabilities.len()
        );
        Ok(handle)
    }

    /// 工具 2: 终止运行中沙箱 (graceful, 可带信号)。终态沙箱拒收
    /// ([`SandboxError::InvalidState`])。
    pub async fn kill(&mut self, id: &Uuid, signal: Option<i32>) -> SandboxResult<()> {
        let current = self.local_handle(id)?;
        if current.is_finished() {
            return Err(SandboxError::InvalidState(format!(
                "sandbox {id} already finished ({})",
                current.status
            )));
        }
        let result = self
            .call_op(OrchestrationRequest::Terminate {
                sandbox_id: *id,
                signal,
            })
            .await?;
        let OpResult::Terminated { status, .. } = result else {
            return Err(SandboxError::Protocol(
                "terminate response did not carry a status".into(),
            ));
        };
        let next = reconcile(current.status, status)?;
        if let Some(handle) = self.handles.get_mut(id) {
            handle.status = next;
            if next.is_terminal() {
                finish_handle(handle);
                handle.exit_code = handle.exit_code.or(Some(0));
            }
        }
        if next.is_terminal() {
            self.release_if_reserved(id);
        }
        Ok(())
    }

    /// 工具 3: 等待退出 (deadline 超时; 未在期限内退出 = [`SandboxError::Timeout`])。
    pub async fn wait(&mut self, id: &Uuid, timeout_secs: Option<u64>) -> SandboxResult<ExitCode> {
        let current = self.local_handle(id)?;
        let timeout_ms = clamp_timeout(
            timeout_secs.map(|secs| secs.saturating_mul(1000)),
            DEFAULT_WAIT_TIMEOUT_MS,
            MAX_WAIT_TIMEOUT_MS,
        )?;
        let result = self
            .call_op_within(
                OrchestrationRequest::Wait {
                    sandbox_id: *id,
                    timeout_ms,
                },
                timeout_ms,
            )
            .await?;
        let OpResult::Waited {
            exit_code,
            finished,
            ..
        } = result
        else {
            return Err(SandboxError::Protocol(
                "wait response did not carry an outcome".into(),
            ));
        };
        if !finished {
            return Err(SandboxError::Timeout(format!(
                "sandbox {id} did not finish within {timeout_ms} ms"
            )));
        }
        let exit = exit_code
            .ok_or_else(|| SandboxError::Protocol("finished wait without an exit code".into()))?;
        if let Some(handle) = self.handles.get_mut(id) {
            handle.status = reconcile(current.status, SandboxStatus::Stopped)?;
            finish_handle(handle);
            handle.exit_code = Some(exit.value());
        }
        self.release_if_reserved(id);
        Ok(exit)
    }

    /// 工具 4: 状态单查 (本地与服务端对账, 回报不可达 = [`SandboxError::InvalidState`])。
    pub async fn get_status(&mut self, id: &Uuid) -> SandboxResult<SandboxStatus> {
        let current = self.local_handle(id)?;
        let result = self
            .call_op(OrchestrationRequest::Inspect { sandbox_id: *id })
            .await?;
        let OpResult::Inspected { handle: reported } = result else {
            return Err(SandboxError::Protocol(
                "inspect response did not carry a handle".into(),
            ));
        };
        let next = reconcile(current.status, reported.status)?;
        if let Some(handle) = self.handles.get_mut(id) {
            handle.status = next;
            handle.started_at = reported.started_at;
            handle.finished_at = reported.finished_at;
            handle.exit_code = reported.exit_code;
            handle.error = reported.error;
        }
        if next.is_terminal() {
            self.release_if_reserved(id);
        }
        Ok(next)
    }

    /// 工具 5: 流式日志 (断点续传; 错误随流传, 不静默截断)。
    pub async fn stream_logs(
        &self,
        id: &Uuid,
    ) -> SandboxResult<Pin<Box<dyn Stream<Item = Result<LogStreamEvent, SandboxError>> + Send>>>
    {
        self.local_handle(id)?;
        let cursor = LogCursor {
            sandbox_id: *id,
            next_seq: 0,
            queue: VecDeque::new(),
            done: false,
            transport: Arc::clone(&self.transport),
            timeout_ms: self.request_timeout_ms,
        };
        Ok(Box::pin(futures::stream::unfold(
            cursor,
            |mut cursor| async {
                loop {
                    if let Some(event) = cursor.queue.pop_front() {
                        return Some((Ok(event), cursor));
                    }
                    if cursor.done || cursor.next_seq >= SANDBOX_MAX_LOG_CHUNKS {
                        return None;
                    }
                    let request = OrchestrationRequest::Logs {
                        sandbox_id: cursor.sandbox_id,
                        since_seq: cursor.next_seq,
                        max_chunks: LOG_CHUNK_BATCH,
                    };
                    match call_op_on(&cursor.transport, cursor.timeout_ms, request).await {
                        Ok(OpResult::LogsChunks { events, last }) => {
                            if let Some(newest) = events.last() {
                                cursor.next_seq = newest.seq + 1;
                            }
                            cursor.queue.extend(events);
                            cursor.done = last;
                        }
                        Ok(_other) => {
                            let err = SandboxError::Protocol(
                                "unexpected response for a logs request".into(),
                            );
                            cursor.done = true;
                            return Some((Err(err), cursor));
                        }
                        Err(err) => {
                            cursor.done = true;
                            return Some((Err(err), cursor));
                        }
                    }
                }
            },
        )))
    }

    /// 工具 6: 销毁并释放资源 (幂等: 服务端已无记录视为目标达成)。
    pub async fn cleanup(&mut self, id: &Uuid) -> SandboxResult<()> {
        if !self.handles.contains_key(id) {
            return Err(SandboxError::NotFound {
                sandbox_id: id.to_string(),
            });
        }
        match self
            .call_op(OrchestrationRequest::Destroy {
                sandbox_id: *id,
                release_resources: true,
            })
            .await
        {
            Ok(OpResult::Destroyed { .. }) => {}
            Ok(_) => {
                return Err(SandboxError::Protocol(
                    "destroy response did not confirm destruction".into(),
                ))
            }
            Err(SandboxError::NotFound { .. }) => {}
            Err(err) => return Err(err),
        }
        self.handles.remove(id);
        self.release_if_reserved(id);
        Ok(())
    }

    /// 状态巡检: 批量对账 (本地 ↔ 服务端) + 超龄回收 + 配额归还。
    ///
    /// - 服务端已无记录的沙箱 → 本地回收 (计入 `reaped`);
    /// - 超过 [`SANDBOX_MAX_LIFETIME_SECONDS`] 仍 running → 强制销毁回收;
    /// - 状态漂移按生命周期矩阵对齐 (不可达计 `failures`, 状态不动)。
    pub async fn patrol(&mut self) -> SandboxResult<PatrolReport> {
        let ids: Vec<Uuid> = self.handles.keys().copied().collect();
        let mut report = PatrolReport {
            inspected: 0,
            transitioned: 0,
            reaped: Vec::new(),
            failures: 0,
            quota: self.ledger.snapshot(),
        };
        for id in ids {
            report.inspected += 1;
            match self
                .call_op(OrchestrationRequest::Inspect { sandbox_id: id })
                .await
            {
                Ok(OpResult::Inspected { handle: reported }) => {
                    let current = self.local_handle(&id).map(|h| h.status);
                    match current.and_then(|status| reconcile(status, reported.status)) {
                        Ok(next) => {
                            if let Some(handle) = self.handles.get_mut(&id) {
                                if handle.status != next {
                                    report.transitioned += 1;
                                }
                                handle.status = next;
                                handle.started_at = reported.started_at;
                                handle.finished_at = reported.finished_at;
                                handle.exit_code = reported.exit_code;
                                handle.error = reported.error;
                            }
                            if next.is_terminal() {
                                self.release_if_reserved(&id);
                            }
                        }
                        Err(_) => report.failures += 1,
                    }
                }
                Ok(_) => report.failures += 1,
                Err(SandboxError::NotFound { .. }) => {
                    self.handles.remove(&id);
                    self.release_if_reserved(&id);
                    report.reaped.push(id);
                }
                Err(_) => report.failures += 1,
            }

            // 超龄回收 (只针对仍在运行的沙箱)。
            let expired = self.handles.get(&id).is_some_and(|handle| {
                handle.is_running()
                    && SystemTime::now()
                        .duration_since(handle.started_at)
                        .unwrap_or_default()
                        > Duration::from_secs(SANDBOX_MAX_LIFETIME_SECONDS)
            });
            if expired {
                match self
                    .call_op(OrchestrationRequest::Destroy {
                        sandbox_id: id,
                        release_resources: true,
                    })
                    .await
                {
                    Ok(OpResult::Destroyed { .. }) | Err(SandboxError::NotFound { .. }) => {
                        self.handles.remove(&id);
                        self.release_if_reserved(&id);
                        report.reaped.push(id);
                    }
                    Ok(_) => report.failures += 1,
                    Err(_) => report.failures += 1,
                }
            }
        }
        report.quota = self.ledger.snapshot();
        Ok(report)
    }
}

/// stream_logs 的拉取游标 (unfold 状态)。
struct LogCursor {
    sandbox_id: Uuid,
    next_seq: u64,
    queue: VecDeque<LogStreamEvent>,
    done: bool,
    transport: Arc<dyn OrchestrationTransport>,
    timeout_ms: u64,
}

/// 终态落表: 完成时间缺省补当前时刻。
fn finish_handle(handle: &mut SandboxHandle) {
    handle.finished_at = handle.finished_at.or_else(|| Some(SystemTime::now()));
}

/// 线上错误码 → 本层闭合错误分类 (一一映射, 无自由字符串扩散)。
fn classify_wire_error(code: WireErrorCode, detail: String) -> SandboxError {
    match code {
        WireErrorCode::InvalidConfig => SandboxError::InvalidConfig(detail),
        WireErrorCode::InvalidState => SandboxError::InvalidState(detail),
        WireErrorCode::NotFound => SandboxError::NotFound { sandbox_id: detail },
        WireErrorCode::QuotaExceeded => SandboxError::QuotaExceeded(detail),
        WireErrorCode::Timeout => SandboxError::Timeout(detail),
        WireErrorCode::ResourceExhausted => SandboxError::ResourceExhausted(detail),
        WireErrorCode::PermissionDenied => SandboxError::PermissionDenied(detail),
        WireErrorCode::Runtime => SandboxError::Runtime {
            runtime: RuntimeKind::default(),
            message: detail,
        },
        WireErrorCode::Internal => SandboxError::Other(detail),
    }
}

/// 单次编排往返 (deadline 竞速 + 编解码 + 版本/回声校验 + 错误分类)。
async fn call_op_on(
    transport: &Arc<dyn OrchestrationTransport>,
    timeout_ms: u64,
    body: OrchestrationRequest,
) -> SandboxResult<OpResult> {
    let timeout_ms = clamp_timeout(
        Some(timeout_ms),
        DEFAULT_REQUEST_TIMEOUT_MS,
        MAX_REQUEST_TIMEOUT_MS,
    )?;
    let request_id = Uuid::new_v4();
    let request_frame = protocol::encode_request(request_id, body)?;
    // 超时统一走 deadline: 到期通知与传输往返竞速, 谁先到谁定结果。
    let (_deadline, mut notice) = Deadline::after(Duration::from_millis(timeout_ms))?;
    let call = transport.call(request_frame);
    futures::pin_mut!(call);
    let response_frame = match select(call, Box::pin(notice.notified())).await {
        Either::Left((result, _pending)) => {
            result.map_err(|err| SandboxError::Transport(err.to_string()))?
        }
        Either::Right((_token, _unfinished)) => {
            return Err(SandboxError::Timeout(format!(
                "request {request_id} exceeded {timeout_ms} ms"
            )));
        }
    };
    let frame = protocol::decode_response(&response_frame)?;
    protocol::check_response_echo(request_id, &frame)?;
    match frame.body {
        OrchestrationResponse::Ok { result } => Ok(result),
        OrchestrationResponse::Err { code, detail } => Err(classify_wire_error(code, detail)),
    }
}

// ============================================================================
// §6 async trait SandboxSpawner — 运行时扩展点 (默认实现走传输边界)
// ============================================================================

/// 运行时 spawner 扩展点: 默认实现把 spawn/kill/wait 转译成协议帧往返
/// (经调用方传入的 [`SandboxSdk`]), 特定运行时可覆写。
#[async_trait]
pub trait SandboxSpawner: Send + Sync {
    /// 运行时种类。
    fn kind(&self) -> RuntimeKind;
}

/// 默认 spawner (携带运行时种类, 供注册表按 kind 分发)。
#[derive(Debug, Default)]
pub struct ConfiguredSandboxSpawner {
    kind: RuntimeKind,
}

impl ConfiguredSandboxSpawner {
    /// 新建默认 spawner。
    pub fn new(kind: RuntimeKind) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl SandboxSpawner for ConfiguredSandboxSpawner {
    fn kind(&self) -> RuntimeKind {
        self.kind
    }
}

// ============================================================================
// §7 测试 (K-1 配置守门 + mock 编排 ≥6)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> SecurityPolicy {
        SecurityPolicy::new(
            "docker.io/library/alpine:3.19",
            vec!["/bin/sh".to_string()],
            "apeireth",
        )
    }

    fn mock_sdk(mock: &Arc<MockOrchestrationService>) -> SandboxSdk {
        SandboxSdk::new(
            SandboxConfig::default(),
            Arc::clone(mock) as Arc<dyn OrchestrationTransport>,
            QuotaLedger::new(QuotaPolicy::default()).unwrap(),
        )
        .unwrap()
    }

    fn shared_quota_sdk(mock: &Arc<MockOrchestrationService>, ledger: QuotaLedger) -> SandboxSdk {
        SandboxSdk::new(
            SandboxConfig::default(),
            Arc::clone(mock) as Arc<dyn OrchestrationTransport>,
            ledger,
        )
        .unwrap()
    }

    // ---------- K-1 配置守门 (保留) ----------

    /// 编译期常量守门。
    #[test]
    fn sandbox_compile_time_constants_match_k1() {
        assert_eq!(SANDBOX_SCHEMA_VERSION, "1");
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(SANDBOX_MAX_LIFETIME_SECONDS, 3600);
        assert_eq!(SANDBOX_MAX_LOG_CHUNKS, 10_000);
        assert_eq!(SANDBOX_MAX_LOG_CHUNK_BYTES, 4096);
        assert_eq!(DEFAULT_ISOLATION_LEVEL, IsolationLevel::Container);
        assert_eq!(DEFAULT_RUNTIME_KIND, RuntimeKind::Docker);
    }

    /// 3 运行时 + 3 隔离级别守门。
    #[test]
    fn sandbox_runtime_and_isolation_have_3_each() {
        assert_eq!(SUPPORTED_RUNTIME_KINDS.len(), 3);
        assert_eq!(SUPPORTED_ISOLATION_LEVELS.len(), 3);
        for r in SUPPORTED_RUNTIME_KINDS {
            assert_eq!(r.to_string().parse::<RuntimeKind>().unwrap(), *r);
        }
        for i in SUPPORTED_ISOLATION_LEVELS {
            assert_eq!(i.to_string().parse::<IsolationLevel>().unwrap(), *i);
        }
    }

    /// 工具白名单 6 项 + 非白名单拒绝。
    #[test]
    fn sandbox_tool_whitelist_gates_calls() {
        assert_eq!(SANDBOX_TOOL_WHITELIST.len(), SANDBOX_TOOL_WHITELIST_COUNT);
        let args = serde_json::json!({});
        assert!(validate_tool_call("apeireth_sdk_sandbox_spawn", &args).is_ok());
        let err = validate_tool_call("apeireth_sdk_sandbox_bogus_tool", &args).unwrap_err();
        assert!(matches!(err, SandboxError::ToolNotWhitelisted(_)));
    }

    /// SandboxConfig 六项强校验 (image / command / user / env / ports / mounts)。
    #[test]
    fn sandbox_config_validate_6_rules() {
        let cfg = SandboxConfig::default();
        assert!(cfg.validate().is_ok());

        let mut bad = cfg.clone();
        bad.policy.image = "".to_string();
        assert!(matches!(bad.validate(), Err(SandboxError::InvalidImage(_))));

        let mut bad = cfg.clone();
        bad.policy.command = vec![];
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidCommand(_))
        ));

        let mut bad = cfg.clone();
        bad.policy.user = "root".to_string();
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        let mut bad = cfg.clone();
        bad.policy
            .env
            .insert("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string());
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        let mut bad = cfg.clone();
        bad.policy.ports.push(PortMapping {
            host_port: 8080,
            container_port: 0,
            protocol: PortProtocol::Tcp,
            allow_privileged: false,
        });
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        let mut bad = cfg.clone();
        bad.policy.ports.push(PortMapping {
            host_port: 22,
            container_port: 8022,
            protocol: PortProtocol::Tcp,
            allow_privileged: false,
        });
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

        let mut bad = cfg.clone();
        bad.policy.mounts.push(VolumeMount {
            source: PathBuf::from("/tmp/../etc/passwd"),
            target: PathBuf::from("/mnt/passwd"),
            read_only: true,
        });
        assert!(matches!(
            bad.validate(),
            Err(SandboxError::InvalidConfig(_))
        ));

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

    /// 句柄状态判定 + 退出码数值映射。
    #[test]
    fn sandbox_handle_state_and_exit_code_mapping() {
        let mut h = SandboxHandle::new(RuntimeKind::Docker, IsolationLevel::Container);
        assert_eq!(h.status, SandboxStatus::Pending);
        assert!(!h.is_running() && !h.is_finished());
        h.status = SandboxStatus::Running;
        assert!(h.is_running());
        h.status = SandboxStatus::Stopped;
        h.exit_code = Some(0);
        assert!(h.is_finished());

        assert_eq!(ExitCode::Ok.value(), 0);
        assert_eq!(ExitCode::Failed(42).value(), 42);
        assert_eq!(ExitCode::Signaled(9).value(), 137);
        assert_eq!(ExitCode::Killed.value(), 137);
        assert_eq!(ExitCode::Oom.value(), 137);
    }

    // ---------- mock 编排测试 ----------

    /// 生命周期全链: 创建 → 巡检单查 → 终止 → 等待退出 → 销毁回收, 配额闭环。
    #[tokio::test]
    async fn mock_lifecycle_full_chain_create_inspect_kill_wait_cleanup() {
        let mock = Arc::new(MockOrchestrationService::new());
        let mut sdk = mock_sdk(&mock);

        let handle = sdk.spawn(policy()).await.expect("spawn");
        assert_eq!(handle.status, SandboxStatus::Running);
        assert_eq!(sdk.active_sandboxes(), 1);
        assert!(sdk.quota().is_reserved(&handle.id));

        assert_eq!(
            sdk.get_status(&handle.id).await.unwrap(),
            SandboxStatus::Running
        );

        sdk.kill(&handle.id, Some(9)).await.expect("kill");
        let exit = sdk.wait(&handle.id, Some(5)).await.expect("wait");
        assert_eq!(exit, ExitCode::Failed(137));
        assert!(sdk.get_handle(&handle.id).unwrap().is_finished());

        sdk.cleanup(&handle.id).await.expect("cleanup");
        assert!(sdk.get_handle(&handle.id).is_none());
        assert_eq!(sdk.quota().snapshot().active_sandboxes, 0);
        assert_eq!(mock.active_count(), 0);

        // 协议面: 服务端确实见过全部四类请求。
        let seen = mock.requests_seen();
        assert!(seen
            .iter()
            .any(|r| matches!(r, OrchestrationRequest::Create { .. })));
        assert!(seen
            .iter()
            .any(|r| matches!(r, OrchestrationRequest::Inspect { .. })));
        assert!(seen
            .iter()
            .any(|r| matches!(r, OrchestrationRequest::Terminate { .. })));
        assert!(seen
            .iter()
            .any(|r| matches!(r, OrchestrationRequest::Destroy { .. })));
    }

    /// 生命周期非法操作: 终态再 kill / 未知名 wait / 未知名 cleanup 全部拒绝。
    #[tokio::test]
    async fn mock_lifecycle_illegal_operations_reject_with_closed_errors() {
        let mock = Arc::new(MockOrchestrationService::new());
        let mut sdk = mock_sdk(&mock);
        let handle = sdk.spawn(policy()).await.unwrap();
        sdk.kill(&handle.id, None).await.unwrap();

        let err = sdk.kill(&handle.id, None).await.unwrap_err();
        assert!(matches!(err, SandboxError::InvalidState(_)));

        let unknown = Uuid::new_v4();
        let err = sdk.wait(&unknown, Some(1)).await.unwrap_err();
        assert!(matches!(err, SandboxError::NotFound { .. }));
        let err = sdk.cleanup(&unknown).await.unwrap_err();
        assert!(matches!(err, SandboxError::NotFound { .. }));
    }

    /// 配额边界 (客户端侧): 恰好用满允许, 超一分拒绝, 销毁后恢复。
    #[tokio::test]
    async fn mock_quota_boundary_client_side() {
        let mock = Arc::new(MockOrchestrationService::new());
        let ledger = QuotaLedger::new(QuotaPolicy {
            max_sandboxes: 2,
            max_cpu_cores: 8.0,
            max_memory_bytes: 4 * 1024 * 1024 * 1024,
        })
        .unwrap();
        let mut sdk = shared_quota_sdk(&mock, ledger);

        let first = sdk.spawn(policy()).await.expect("first fits");
        let second = sdk.spawn(policy()).await.expect("exactly at limit");
        let err = sdk.spawn(policy()).await.unwrap_err();
        assert!(matches!(err, SandboxError::QuotaExceeded(_)));

        sdk.cleanup(&first.id).await.unwrap();
        let third = sdk.spawn(policy()).await.expect("capacity restored");
        sdk.cleanup(&second.id).await.unwrap();
        sdk.cleanup(&third.id).await.unwrap();
        assert_eq!(sdk.quota().snapshot().active_sandboxes, 0);
    }

    /// 配额边界 (服务端侧): 线上配额超限按闭合词表分类, 客户端预留必归还。
    #[tokio::test]
    async fn mock_quota_boundary_server_side_classifies_and_releases() {
        let mock = Arc::new(MockOrchestrationService::new());
        mock.set_server_quota(1);
        let mut sdk = mock_sdk(&mock);

        let first = sdk.spawn(policy()).await.expect("first fits");
        let err = sdk.spawn(policy()).await.unwrap_err();
        assert!(matches!(err, SandboxError::QuotaExceeded(_)));
        assert_eq!(
            sdk.quota().snapshot().active_sandboxes,
            1,
            "failed spawn must return its reservation"
        );
        sdk.cleanup(&first.id).await.unwrap();
    }

    /// 错误分类: 9 个线上错误码逐一映射到本层闭合词表。
    #[tokio::test]
    async fn mock_error_classification_covers_the_wire_vocabulary() {
        let mock = Arc::new(MockOrchestrationService::new());
        let mut sdk = mock_sdk(&mock);
        let handle = sdk.spawn(policy()).await.unwrap();

        for code in WireErrorCode::ALL {
            mock.fail_next(*code);
            let err = sdk.get_status(&handle.id).await.unwrap_err();
            assert_eq!(
                err.code(),
                code.classify(),
                "wire code {} must classify to {}",
                code.as_str(),
                code.classify()
            );
        }
    }

    /// 超时经 deadline: 服务端慢于客户端期限 → `timeout` 分类。
    #[tokio::test]
    async fn mock_deadline_timeout_on_slow_service() {
        let mock = Arc::new(MockOrchestrationService::new());
        mock.set_delay(Duration::from_millis(300));
        let mut sdk = mock_sdk(&mock).with_request_timeout_ms(60).unwrap();

        let err = sdk.spawn(policy()).await.unwrap_err();
        assert_eq!(err.code(), SandboxErrorCode::Timeout);
        assert_eq!(
            sdk.quota().snapshot().active_sandboxes,
            0,
            "timed-out spawn must return its reservation"
        );
    }

    /// 传输失败: 边界如实报告 → `transport` 分类, 配额归还。
    #[tokio::test]
    async fn mock_transport_failure_classifies_and_releases_quota() {
        let mock = Arc::new(MockOrchestrationService::new());
        mock.close();
        let mut sdk = mock_sdk(&mock);

        let err = sdk.spawn(policy()).await.unwrap_err();
        assert_eq!(err.code(), SandboxErrorCode::Transport);
        assert_eq!(sdk.quota().snapshot().active_sandboxes, 0);
    }

    /// 并发编排: 多客户端共享一份配额账本, 并发创建/销毁下账目一致。
    ///
    /// mock 加小延迟制造真实并发窗口: 六个任务都在归还前完成预留,
    /// 共享配额 3 → 恰好 3 成 3 败。
    #[tokio::test]
    async fn mock_concurrent_orchestration_keeps_shared_quota_consistent() {
        let mock = Arc::new(MockOrchestrationService::new());
        mock.set_delay(Duration::from_millis(20));
        let ledger = QuotaLedger::new(QuotaPolicy {
            max_sandboxes: 3,
            max_cpu_cores: 32.0,
            max_memory_bytes: 16 * 1024 * 1024 * 1024,
        })
        .unwrap();

        let mut tasks = Vec::new();
        for _ in 0..6 {
            let mock = Arc::clone(&mock);
            let ledger = ledger.clone();
            tasks.push(tokio::spawn(async move {
                let mut sdk = shared_quota_sdk(&mock, ledger);
                match sdk.spawn(policy()).await {
                    Ok(handle) => {
                        sdk.kill(&handle.id, None).await.unwrap();
                        sdk.wait(&handle.id, Some(5)).await.unwrap();
                        sdk.cleanup(&handle.id).await.unwrap();
                        true
                    }
                    Err(err) => {
                        assert_eq!(err.code(), SandboxErrorCode::QuotaExceeded);
                        false
                    }
                }
            }));
        }
        let mut successes = 0;
        for task in tasks {
            if task.await.unwrap() {
                successes += 1;
            }
        }
        assert_eq!(successes, 3, "exactly the shared quota may succeed");
        assert_eq!(ledger.snapshot().active_sandboxes, 0);
        assert_eq!(mock.active_count(), 0);
    }

    /// 状态巡检: 漂移对账 + 超龄回收 + 服务端丢失记录回收, 配额闭环。
    #[tokio::test]
    async fn mock_patrol_reconciles_drift_reaps_expired_and_lost() {
        let mock = Arc::new(MockOrchestrationService::new());
        let mut sdk = mock_sdk(&mock);
        let drifted = sdk.spawn(policy()).await.unwrap();
        let expired = sdk.spawn(policy()).await.unwrap();
        let lost = sdk.spawn(policy()).await.unwrap();
        assert_eq!(sdk.quota().snapshot().active_sandboxes, 3);

        // 漂移: 服务端失败, 本地仍 running → 对账到 failed。
        mock.force_status(&drifted.id, SandboxStatus::Failed);
        // 超龄: 服务端记录已运行超过单沙箱存活上限。
        mock.force_age(
            &expired.id,
            Duration::from_secs(2 * SANDBOX_MAX_LIFETIME_SECONDS),
        );
        // 丢失: 服务端已无记录。
        mock.drop_server(&lost.id);

        let report = sdk.patrol().await.expect("patrol");
        assert_eq!(report.inspected, 3);
        assert!(report.transitioned >= 1, "drift must be reconciled");
        assert!(
            report.reaped.contains(&expired.id) && report.reaped.contains(&lost.id),
            "expired and lost must be reaped: {:?}",
            report.reaped
        );
        assert_eq!(
            sdk.get_handle(&drifted.id).unwrap().status,
            SandboxStatus::Failed
        );
        assert!(sdk.get_handle(&expired.id).is_none());
        assert_eq!(report.quota.active_sandboxes, 0);
    }

    /// 流式日志: chunk 有序、seq 连续、错误随流传。
    #[tokio::test]
    async fn mock_stream_logs_delivers_ordered_chunks() {
        let mock = Arc::new(MockOrchestrationService::new());
        mock.push_log_line("boot");
        mock.push_log_line("ready");
        mock.push_log_line("done");
        let mut sdk = mock_sdk(&mock);
        let handle = sdk.spawn(policy()).await.unwrap();

        let stream = sdk.stream_logs(&handle.id).await.unwrap();
        futures::pin_mut!(stream);
        let mut collected = Vec::new();
        while let Some(item) = stream.next().await {
            collected.push(item.expect("no stream error"));
        }
        assert_eq!(collected.len(), 3);
        for (i, event) in collected.iter().enumerate() {
            assert_eq!(event.seq, i as u64);
            assert_eq!(event.sandbox_id, handle.id);
        }
        assert_eq!(collected[0].data, b"boot");
        assert_eq!(collected[2].data, b"done");
    }

    /// 脱敏日志: 环境变量值 / 凭据明文不入摘要。
    #[test]
    fn log_summary_redacts_secret_values() {
        let mut cfg = SandboxConfig::default();
        cfg.policy
            .env
            .insert("API_TOKEN".to_string(), "hunter2-secret".to_string());
        cfg.credentials = Some(SandboxCredentials {
            registry: "registry.example".into(),
            username: "builder".into(),
            secret_ref: "registry-token".into(),
        });
        let summary = cfg.log_summary();
        assert!(!summary.contains("hunter2-secret"), "leak: {summary}");
        assert!(summary.contains(&redact_secret("hunter2-secret")));
        assert!(!summary.contains("hunter2"), "leak: {summary}");
    }
}
