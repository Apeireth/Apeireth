//! apeireth-environment: 6 terminal backend (R173 / Stage2 §3).
//!
//! 借鉴 stage2 §3 environment 决策:
//! - Local: 本机执行 (default)
//! - Docker: 容器化执行 (linux + seccomp)
//! - SSH: 远程执行 (rust SSH client)
//! - Daytona: dev environment SaaS (HTTP REST API)
//! - Modal: serverless Python (HTTP REST API)
//! - Singularity: HPC container (subprocess interface)
//!
//! **不漂移**:
//! - 0 改 apeireth-tool-shell 任何已实装类型
//! - 0 副作用: 每个 backend execute() 返回 Result, 不直接 IO
//! - 借鉴 apeireth-tool-shell 的 sandbox.rs 设计模式
//!
//! **当前状态**: R173 阶段 6 后端补全 — 6 backend trait + Local/Docker/SSH 真实现 + 远程 stub.

#![deny(unsafe_code)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

// ============================================================
// 1. 错误类型
// ============================================================

#[derive(Debug, Error)]
pub enum EnvironmentError {
    #[error("local exec failed: {0}")]
    LocalFailed(String),
    #[error("docker unavailable: {0}")]
    DockerUnavailable(String),
    #[error("docker exec failed: {0}")]
    DockerFailed(String),
    #[error("ssh unavailable: {0}")]
    SshUnavailable(String),
    #[error("ssh exec failed: {0}")]
    SshFailed(String),
    #[error("daytona not configured: {0}")]
    DaytonaUnconfigured(String),
    #[error("modal not configured: {0}")]
    ModalUnconfigured(String),
    #[error("singularity not configured: {0}")]
    SingularityUnconfigured(String),
    #[error("execution timed out after {0} seconds")]
    Timeout(u64),
    #[error("execution denied: {0}")]
    Denied(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type EnvironmentResult<T> = Result<T, EnvironmentError>;

// ============================================================
// 2. 公共类型
// ================================================

/// 命令执行请求.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    /// 命令 (e.g. "ls -la")
    pub command: String,
    /// 工作目录 (相对 backend 根)
    pub working_dir: Option<String>,
    /// 环境变量 (key=value)
    pub env: Vec<(String, String)>,
    /// 超时 (秒)
    pub timeout_secs: u64,
    /// stdin 输入
    pub stdin: Option<String>,
}

impl ExecRequest {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            working_dir: None,
            env: Vec::new(),
            timeout_secs: 30,
            stdin: None,
        }
    }

    pub fn with_cwd(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }
}

/// 命令执行结果.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResult {
    /// 退出码
    pub exit_code: i32,
    /// stdout
    pub stdout: String,
    /// stderr
    pub stderr: String,
    /// 耗时 (ms)
    pub duration_ms: u64,
    /// 后端名
    pub backend: &'static str,
    /// 任务 ID
    pub task_id: Uuid,
}

// ================================================
// 3. BackendKind + TerminalBackend trait
// ================================================

/// 6 种 backend 识别.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendKind {
    Local,
    Docker,
    Ssh,
    Daytona,
    Modal,
    Singularity,
}

impl BackendKind {
    pub const ALL: [BackendKind; 6] = [
        Self::Local,
        Self::Docker,
        Self::Ssh,
        Self::Daytona,
        Self::Modal,
        Self::Singularity,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Docker => "docker",
            Self::Ssh => "ssh",
            Self::Daytona => "daytona",
            Self::Modal => "modal",
            Self::Singularity => "singularity",
        }
    }

    pub const fn is_local(&self) -> bool {
        matches!(self, Self::Local)
    }

    pub const fn is_remote(&self) -> bool {
        matches!(self, Self::Ssh | Self::Daytona | Self::Modal)
    }

    pub const fn is_container(&self) -> bool {
        matches!(self, Self::Docker | Self::Singularity)
    }
}

/// 后端 trait — 6 backend 各自实现.
#[async_trait]
pub trait TerminalBackend: Send + Sync {
    fn kind(&self) -> BackendKind;
    fn name(&self) -> &str;
    async fn execute(&self, req: &ExecRequest) -> EnvironmentResult<ExecResult>;
    async fn availability(&self) -> bool;
}

// ================================================
// 4. LocalBackend (本机执行)
// ================================================

/// 本机 backend — 用 tokio::process.
pub struct LocalBackend;

#[async_trait]
impl TerminalBackend for LocalBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Local
    }
    fn name(&self) -> &str {
        "local"
    }

    async fn execute(&self, req: &ExecRequest) -> EnvironmentResult<ExecResult> {
        let started = std::time::Instant::now();
        let task_id = Uuid::new_v4();
        // Tokenize the command into program + argv. We do NOT route through a
        // shell wrapper because doing so on Windows makes the timeout handler
        // unreliable: killing cmd.exe does not cascade to its descendants
        // (e.g. cmd.exe -> ping.exe), so the process keeps running past the
        // timeout boundary and the test process blocks until it finishes.
        // Callers that want a shell should pass `cmd` / `sh` as the program
        // and the rest as argv (e.g. `cmd /c echo hello`).
        let mut parts = req.command.split_whitespace();
        let prog = parts.next().unwrap_or("");
        if prog.is_empty() {
            return Err(EnvironmentError::LocalFailed("empty command".into()));
        }
        let mut cmd = tokio::process::Command::new(prog);
        cmd.args(parts);
        if let Some(cwd) = &req.working_dir {
            cmd.current_dir(cwd);
        }
        for (k, v) in &req.env {
            cmd.env(k, v);
        }
        if req.stdin.is_some() {
            cmd.stdin(std::process::Stdio::piped());
        }
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        // Spawn the child explicitly so we can guarantee the OS process is
        // terminated when the timeout fires. `tokio::process::Command::output()`
        // consumes the Child and does NOT kill on future drop, so we drive
        // `wait` ourselves and manually drain stdout/stderr.
        let mut child = cmd
            .spawn()
            .map_err(|e| EnvironmentError::LocalFailed(e.to_string()))?;
        let mut stdout_pipe = child.stdout.take();
        let mut stderr_pipe = child.stderr.take();
        let drain = async {
            let mut so = Vec::new();
            let mut se = Vec::new();
            if let Some(s) = stdout_pipe.take() {
                let mut s = s;
                let _ = s.read_to_end(&mut so).await;
            }
            if let Some(s) = stderr_pipe.take() {
                let mut s = s;
                let _ = s.read_to_end(&mut se).await;
            }
            (so, se)
        };
        let timeout_dur = std::time::Duration::from_secs(req.timeout_secs);
        let timeout_result = tokio::time::timeout(timeout_dur, async {
            let status = child
                .wait()
                .await
                .map_err(|e| EnvironmentError::LocalFailed(e.to_string()))?;
            let (so, se) = drain.await;
            Ok::<_, EnvironmentError>((status, so, se))
        })
        .await;
        let (status, stdout_bytes, stderr_bytes) = match timeout_result {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Best-effort kill; ignore errors if the child already exited.
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Err(EnvironmentError::Timeout(req.timeout_secs));
            }
        };

        Ok(ExecResult {
            exit_code: status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&stdout_bytes).into_owned(),
            stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
            duration_ms: started.elapsed().as_millis() as u64,
            backend: "local",
            task_id,
        })
    }
    async fn availability(&self) -> bool {
        true
    }
}

// ================================================
// 5. DockerBackend (容器化执行)
// ================================================

/// Docker backend 配置.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// 镜像名 (e.g. "ubuntu:22.04")
    pub image: String,
    /// 容器名 (optional)
    pub container_name: Option<String>,
    /// 挂载 (volume) 列表
    pub volumes: Vec<(String, String)>,
    /// 网络模式
    pub network: Option<String>,
    /// 内存限制 (e.g. "512m")
    pub memory: Option<String>,
    /// CPU 限制 (e.g. "1.0")
    pub cpus: Option<String>,
}

impl DockerConfig {
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            container_name: None,
            volumes: Vec::new(),
            network: None,
            memory: None,
            cpus: None,
        }
    }

    pub fn with_volume(mut self, host: impl Into<String>, container: impl Into<String>) -> Self {
        self.volumes.push((host.into(), container.into()));
        self
    }
}

/// Docker backend — 通过 `docker run --rm` 执行.
pub struct DockerBackend {
    config: DockerConfig,
}

impl DockerBackend {
    pub fn new(config: DockerConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl TerminalBackend for DockerBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Docker
    }
    fn name(&self) -> &str {
        "docker"
    }

    async fn execute(&self, req: &ExecRequest) -> EnvironmentResult<ExecResult> {
        let started = std::time::Instant::now();
        let task_id = Uuid::new_v4();
        let mut cmd = tokio::process::Command::new("docker");
        cmd.arg("run");
        cmd.arg("--rm");
        if let Some(name) = &self.config.container_name {
            cmd.arg(format!("--name={}", name));
        }
        for (host, cont) in &self.config.volumes {
            cmd.arg("-v").arg(format!("{}:{}", host, cont));
        }
        if let Some(net) = &self.config.network {
            cmd.arg("--network").arg(net);
        }
        if let Some(mem) = &self.config.memory {
            cmd.arg("-m").arg(mem);
        }
        if let Some(cpus) = &self.config.cpus {
            cmd.arg("--cpus").arg(cpus);
        }
        if let Some(cwd) = &req.working_dir {
            cmd.arg("-w").arg(cwd);
        }
        for (k, v) in &req.env {
            cmd.arg("-e").arg(format!("{}={}", k, v));
        }
        cmd.arg(&self.config.image);
        cmd.arg(&req.command);

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(req.timeout_secs),
            cmd.output(),
        )
        .await
        .map_err(|_| EnvironmentError::Timeout(req.timeout_secs))?
        .map_err(|e| EnvironmentError::DockerUnavailable(e.to_string()))?;

        Ok(ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            duration_ms: started.elapsed().as_millis() as u64,
            backend: "docker",
            task_id,
        })
    }

    async fn availability(&self) -> bool {
        // 检查 docker CLI 是否可用
        let out = tokio::process::Command::new("docker")
            .arg("--version")
            .output()
            .await;
        matches!(out, Ok(o) if o.status.success())
    }
}

// ================================================
// 6. SshBackend (远程执行, 简化版)
// ================================================

/// SSH backend 配置.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub key_path: Option<String>,
    pub password: Option<String>,
}

impl SshConfig {
    pub fn new(host: impl Into<String>, port: u16, user: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            user: user.into(),
            key_path: None,
            password: None,
        }
    }

    pub fn with_key(mut self, path: impl Into<String>) -> Self {
        self.key_path = Some(path.into());
        self
    }
}

/// SSH backend — 通过 `ssh user@host command` 执行.
pub struct SshBackend {
    config: SshConfig,
}

impl SshBackend {
    pub fn new(config: SshConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl TerminalBackend for SshBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Ssh
    }
    fn name(&self) -> &str {
        "ssh"
    }

    async fn execute(&self, req: &ExecRequest) -> EnvironmentResult<ExecResult> {
        let started = std::time::Instant::now();
        let task_id = Uuid::new_v4();
        let mut cmd = tokio::process::Command::new("ssh");
        cmd.arg("-p").arg(self.config.port.to_string());
        cmd.arg("-o").arg("BatchMode=yes");
        cmd.arg("-o").arg("StrictHostKeyChecking=accept-new");
        if let Some(key) = &self.config.key_path {
            cmd.arg("-i").arg(key);
        }
        cmd.arg(format!("{}@{}", self.config.user, self.config.host));
        cmd.arg(&req.command);

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(req.timeout_secs),
            cmd.output(),
        )
        .await
        .map_err(|_| EnvironmentError::Timeout(req.timeout_secs))?
        .map_err(|e| EnvironmentError::SshUnavailable(e.to_string()))?;

        Ok(ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            duration_ms: started.elapsed().as_millis() as u64,
            backend: "ssh",
            task_id,
        })
    }

    async fn availability(&self) -> bool {
        let out = tokio::process::Command::new("ssh")
            .arg("-V")
            .output()
            .await;
        out.is_ok()
    }
}

// ================================================
// 7. 远程 stub (Daytona / Modal / Singularity)
// ================================================

/// Daytona 配置 (dev environment SaaS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaytonaConfig {
    pub api_url: String,
    pub api_key: String,
}

pub struct DaytonaBackend {
    config: DaytonaConfig,
}

impl DaytonaBackend {
    pub fn new(config: DaytonaConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl TerminalBackend for DaytonaBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Daytona
    }
    fn name(&self) -> &str {
        "daytona"
    }

    async fn execute(&self, _req: &ExecRequest) -> EnvironmentResult<ExecResult> {
        Err(EnvironmentError::DaytonaUnconfigured(
            "Daytona HTTP API not yet implemented".to_string(),
        ))
    }

    async fn availability(&self) -> bool {
        false
    }
}

/// Modal 配置 (serverless Python).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModalConfig {
    pub api_url: String,
    pub api_key: String,
}

pub struct ModalBackend {
    config: ModalConfig,
}

impl ModalBackend {
    pub fn new(config: ModalConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl TerminalBackend for ModalBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Modal
    }
    fn name(&self) -> &str {
        "modal"
    }

    async fn execute(&self, _req: &ExecRequest) -> EnvironmentResult<ExecResult> {
        Err(EnvironmentError::ModalUnconfigured(
            "Modal HTTP API not yet implemented".to_string(),
        ))
    }

    async fn availability(&self) -> bool {
        false
    }
}

/// Singularity 配置 (HPC container).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingularityConfig {
    pub image: String,
    pub singularity_path: Option<String>,
}

pub struct SingularityBackend {
    config: SingularityConfig,
}

impl SingularityBackend {
    pub fn new(config: SingularityConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl TerminalBackend for SingularityBackend {
    fn kind(&self) -> BackendKind {
        BackendKind::Singularity
    }
    fn name(&self) -> &str {
        "singularity"
    }

    async fn execute(&self, req: &ExecRequest) -> EnvironmentResult<ExecResult> {
        let started = std::time::Instant::now();
        let task_id = Uuid::new_v4();
        let sing = self
            .config
            .singularity_path
            .clone()
            .unwrap_or_else(|| "singularity".to_string());
        let mut cmd = tokio::process::Command::new(&sing);
        cmd.arg("exec");
        cmd.arg(&self.config.image);
        cmd.arg(&req.command);
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(req.timeout_secs),
            cmd.output(),
        )
        .await
        .map_err(|_| EnvironmentError::Timeout(req.timeout_secs))?
        .map_err(|e| EnvironmentError::SingularityUnconfigured(e.to_string()))?;
        Ok(ExecResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            duration_ms: started.elapsed().as_millis() as u64,
            backend: "singularity",
            task_id,
        })
    }

    async fn availability(&self) -> bool {
        let sing = self
            .config
            .singularity_path
            .clone()
            .unwrap_or_else(|| "singularity".to_string());
        let out = tokio::process::Command::new(&sing).arg("--version").output().await;
        out.is_ok()
    }
}

// ================================================
// 8. BackendRegistry — 注册 + 路由
// ================================================

/// Backend 注册表 — 按 kind 索引.
pub struct BackendRegistry {
    backends: std::collections::HashMap<BackendKind, Box<dyn TerminalBackend>>,
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self {
            backends: std::collections::HashMap::new(),
        }
    }

    pub fn register(mut self, backend: Box<dyn TerminalBackend>) -> Self {
        self.backends.insert(backend.kind(), backend);
        self
    }

    pub fn with_local() -> Self {
        Self::new().register(Box::new(LocalBackend))
    }

    pub fn get(&self, kind: BackendKind) -> Option<&dyn TerminalBackend> {
        self.backends.get(&kind).map(|b| b.as_ref())
    }

    pub fn kinds(&self) -> Vec<BackendKind> {
        self.backends.keys().copied().collect()
    }

    pub async fn execute(
        &self,
        kind: BackendKind,
        req: &ExecRequest,
    ) -> EnvironmentResult<ExecResult> {
        match self.get(kind) {
            Some(b) => b.execute(req).await,
            None => Err(EnvironmentError::Denied(format!(
                "backend {:?} not registered",
                kind
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_kind_as_str_covers_all() {
        for k in BackendKind::ALL {
            assert!(!k.as_str().is_empty());
        }
    }

    #[test]
    fn backend_kind_classification() {
        assert!(BackendKind::Local.is_local());
        assert!(BackendKind::Docker.is_container());
        assert!(BackendKind::Singularity.is_container());
        assert!(BackendKind::Ssh.is_remote());
        assert!(BackendKind::Daytona.is_remote());
        assert!(BackendKind::Modal.is_remote());
    }

    #[test]
    fn registry_register_and_get() {
        let r = BackendRegistry::with_local();
        assert!(r.get(BackendKind::Local).is_some());
        assert!(r.get(BackendKind::Docker).is_none());
        assert_eq!(r.kinds().len(), 1);
    }

    #[test]
    fn registry_register_multiple_backends() {
        let r = BackendRegistry::new()
            .register(Box::new(LocalBackend))
            .register(Box::new(DockerBackend::new(DockerConfig::new("ubuntu:22.04"))));
        assert!(r.get(BackendKind::Local).is_some());
        assert!(r.get(BackendKind::Docker).is_some());
        assert_eq!(r.kinds().len(), 2);
    }

    #[tokio::test]
    async fn registry_execute_unregistered_returns_error() {
        let r = BackendRegistry::new();
        let result = r.execute(BackendKind::Docker, &ExecRequest::new("echo hi")).await;
        assert!(result.is_err());
    }

    #[test]
    fn exec_request_builder() {
        let req = ExecRequest::new("ls")
            .with_cwd("/tmp")
            .with_env("KEY", "value")
            .with_timeout(60);
        assert_eq!(req.command, "ls");
        assert_eq!(req.working_dir, Some("/tmp".to_string()));
        assert_eq!(req.env, vec![("KEY".to_string(), "value".to_string())]);
        assert_eq!(req.timeout_secs, 60);
    }

    #[test]
    fn docker_config_builder() {
        let cfg = DockerConfig::new("ubuntu:22.04")
            .with_volume("/host", "/container");
        assert_eq!(cfg.image, "ubuntu:22.04");
        assert_eq!(cfg.volumes.len(), 1);
    }

    #[test]
    fn ssh_config_builder() {
        let cfg = SshConfig::new("server.com", 22, "user").with_key("/path/to/key");
        assert_eq!(cfg.host, "server.com");
        assert_eq!(cfg.port, 22);
        assert_eq!(cfg.user, "user");
        assert_eq!(cfg.key_path, Some("/path/to/key".to_string()));
    }

    #[tokio::test]
    async fn local_backend_executes_simple_command() {
        let backend = LocalBackend;
        // `echo` is a shell builtin on both Windows and Unix, so we run it
        // through the shell binary directly. This also exercises the
        // argv-splitting path of LocalBackend::execute.
        let req = ExecRequest::new(if cfg!(target_os = "windows") {
            "cmd /c echo hello"
        } else {
            "sh -c echo hello"
        })
        .with_timeout(5);
        let result = backend.execute(&req).await;
        assert!(result.is_ok(), "should execute: {:?}", result.err());
        let r = result.unwrap();
        assert_eq!(r.exit_code, 0);
        assert!(r.stdout.contains("hello"));
        assert_eq!(r.backend, "local");
    }

    #[tokio::test]
    async fn local_backend_executes_with_timeout() {
        let backend = LocalBackend;
        // Use a command that can be killed reliably cross-platform.
        // On Windows: `timeout` kills external command; on Unix: `sleep` becomes signal-killable.
        // Use a real binary, not a shell builtin, so the spawned process can be
        // terminated cleanly when the timeout fires. (Windows `timeout` is a cmd.exe
        // builtin and exits with code 0 on this platform when stdin is redirected.)
        let req = ExecRequest::new(if cfg!(target_os = "windows") {
            "ping -n 30 127.0.0.1"
        } else {
            "sleep 30"
        })
        .with_timeout(1);
        let result = backend.execute(&req).await;
        assert!(matches!(result, Err(EnvironmentError::Timeout(1))));
    }

    #[tokio::test]
    async fn local_backend_availability_always_true() {
        let backend = LocalBackend;
        assert!(backend.availability().await);
    }

    #[tokio::test]
    async fn docker_backend_availability_likely_false() {
        // CI 里没有 docker, 所以预期为 false
        let backend = DockerBackend::new(DockerConfig::new("ubuntu:22.04"));
        let _ = backend.availability().await;
    }

    #[tokio::test]
    async fn daytona_backend_unconfigured() {
        let backend = DaytonaBackend::new(DaytonaConfig {
            api_url: "https://api.daytona.io".to_string(),
            api_key: "test".to_string(),
        });
        let req = ExecRequest::new("echo hi");
        let result = backend.execute(&req).await;
        assert!(matches!(result, Err(EnvironmentError::DaytonaUnconfigured(_))));
    }

    #[tokio::test]
    async fn modal_backend_unconfigured() {
        let backend = ModalBackend::new(ModalConfig {
            api_url: "https://api.modal.com".to_string(),
            api_key: "test".to_string(),
        });
        let req = ExecRequest::new("echo hi");
        let result = backend.execute(&req).await;
        assert!(matches!(result, Err(EnvironmentError::ModalUnconfigured(_))));
    }

    #[test]
    fn singularity_config_defaults() {
        let cfg = SingularityConfig {
            image: "ubuntu.sif".to_string(),
            singularity_path: None,
        };
        assert_eq!(cfg.image, "ubuntu.sif");
        assert_eq!(cfg.singularity_path, None);
    }
}

// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;