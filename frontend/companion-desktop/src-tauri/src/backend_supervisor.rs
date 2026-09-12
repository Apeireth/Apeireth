//! Apeireth Desktop Backend Supervisor
//!
//! Manages the lifecycle of the bundled canonical Apeireth gateway backend process.
//!
//! # Responsibilities
//! - Locate bundled apeireth executable
//! - Select free port for gateway
//! - Spawn owned backend process
//! - Track PID and ownership
//! - Health probe for readiness
//! - Capture stdout/stderr to persistent logs
//! - Detect unexpected exit
//! - Expose safe state to frontend
//! - Graceful shutdown on app exit

use crate::keychain;
use crate::logging::{DesktopLogger, LogLevel};
use crate::workspace;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

/// Backend supervisor state machine
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BackendState {
    Stopped,
    Starting,
    Ready,
    Failed,
    Stopping,
}

/// Backend process information
#[derive(Debug, Clone, Serialize)]
pub struct BackendInfo {
    pub state: BackendState,
    pub ownership: BackendOwnership,
    pub pid: Option<u32>,
    pub endpoint: Option<String>,
    pub port: Option<u16>,
    #[serde(skip)]
    pub started_at: Option<Instant>,
    pub restart_count: u32,
    pub last_exit_code: Option<i32>,
    pub last_error: Option<String>,
    pub backend_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum BackendOwnership {
    OwnedByDesktop,
    External,
}

/// Provider environment the desktop injects into the spawned sidecar.
///
/// The Settings UI is the single configuration source for a packaged desktop:
/// the user's provider choice (key / endpoint / models) is pushed here over IPC
/// and becomes the sidecar's environment, exactly as the canonical CLI reads
/// it (`OPENAI_API_KEY` + `APEIRETH_OPENAI_URL` + `APEIRETH_OPENAI_MODELS`,
/// `APEIRETH_API_KEY` + `APEIRETH_API_URL` + `APEIRETH_API_MODELS` for
/// MiniMax, `APEIRETH_ANTHROPIC_KEY` + `APEIRETH_ANTHROPIC_URL` +
/// `APEIRETH_ANTHROPIC_MODELS` for Anthropic). Without this, a packaged app
/// would only ever see whatever environment the user happened to export
/// system-wide — two configuration sources with no guidance.
///
/// Secret hygiene: key fields are in-memory only. They cross IPC to reach the
/// child's environment, but are never persisted to the app-data config file
/// (see [`BackendSupervisor::persist_provider_env`]) and never appear in
/// [`BackendInfo`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BackendProviderEnv {
    pub openai_api_key: Option<String>,
    pub openai_url: Option<String>,
    pub openai_models: Option<String>,
    pub minimax_api_key: Option<String>,
    pub minimax_url: Option<String>,
    pub minimax_models: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub anthropic_url: Option<String>,
    pub anthropic_models: Option<String>,
}

impl BackendProviderEnv {
    /// Normalize for storage: trim whitespace; empty values mean "unset".
    pub fn sanitized(mut self) -> Self {
        let clean = |value: &mut Option<String>| {
            if let Some(v) = value {
                let trimmed = v.trim();
                if trimmed.is_empty() {
                    *value = None;
                } else {
                    *value = Some(trimmed.to_string());
                }
            }
        };
        clean(&mut self.openai_api_key);
        clean(&mut self.openai_url);
        clean(&mut self.openai_models);
        clean(&mut self.minimax_api_key);
        clean(&mut self.minimax_url);
        clean(&mut self.minimax_models);
        clean(&mut self.anthropic_api_key);
        clean(&mut self.anthropic_url);
        clean(&mut self.anthropic_models);
        self
    }

    /// True when nothing would be injected.
    pub fn is_empty(&self) -> bool {
        self.env_pairs().is_empty()
    }

    /// The exact (variable, value) pairs injected into the sidecar.
    pub fn env_pairs(&self) -> Vec<(&'static str, &str)> {
        let mut pairs = Vec::new();
        if let Some(value) = &self.openai_api_key {
            pairs.push(("OPENAI_API_KEY", value.as_str()));
        }
        if let Some(value) = &self.openai_url {
            pairs.push(("APEIRETH_OPENAI_URL", value.as_str()));
        }
        if let Some(value) = &self.openai_models {
            pairs.push(("APEIRETH_OPENAI_MODELS", value.as_str()));
        }
        if let Some(value) = &self.minimax_api_key {
            pairs.push(("APEIRETH_API_KEY", value.as_str()));
        }
        if let Some(value) = &self.minimax_url {
            pairs.push(("APEIRETH_API_URL", value.as_str()));
        }
        if let Some(value) = &self.minimax_models {
            pairs.push(("APEIRETH_API_MODELS", value.as_str()));
        }
        if let Some(value) = &self.anthropic_api_key {
            pairs.push(("APEIRETH_ANTHROPIC_KEY", value.as_str()));
        }
        if let Some(value) = &self.anthropic_url {
            pairs.push(("APEIRETH_ANTHROPIC_URL", value.as_str()));
        }
        if let Some(value) = &self.anthropic_models {
            pairs.push(("APEIRETH_ANTHROPIC_MODELS", value.as_str()));
        }
        pairs
    }

    /// Copy without any key material (what may reach disk).
    pub fn without_secrets(&self) -> Self {
        Self {
            openai_api_key: None,
            minimax_api_key: None,
            anthropic_api_key: None,
            ..self.clone()
        }
    }

    /// Fill missing key fields from the OS keychain (or any injected lookup).
    ///
    /// Priority is `env > keychain`: a key already present in memory (from a
    /// settings save this session) wins; only absent fields fall back to the
    /// keychain, so a fresh launch restores persisted keys without touching
    /// endpoints/models. The lookup is injected so the merge is unit-testable
    /// without touching the real OS credential store.
    pub fn with_keychain_fallback(
        mut self,
        mut lookup: impl FnMut(&str) -> Option<String>,
    ) -> Self {
        if self.openai_api_key.is_none() {
            self.openai_api_key = lookup(keychain::PROVIDER_OPENAI);
        }
        if self.minimax_api_key.is_none() {
            self.minimax_api_key = lookup(keychain::PROVIDER_MINIMAX);
        }
        if self.anthropic_api_key.is_none() {
            self.anthropic_api_key = lookup(keychain::PROVIDER_ANTHROPIC);
        }
        self
    }
}

/// Advanced-capability toggles the desktop injects into the sidecar.
///
/// Mirrors the canonical CLI knobs (`APEIRETH_ENABLE_SHELL/FETCH/ORGANS/
/// PREFERENCE_LEARNING` + `APEIRETH_COGNITIVE_JUDGE/COUNCIL`). Fail-closed by
/// construction: only `true` values emit `"1"`; absent variables mean OFF in
/// the CLI, so a false toggle injects nothing. Shell/fetch stay behind the
/// runtime's require-approval governance even when enabled, so the UI toggle
/// alone never grants unrestricted execution.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BackendCapabilityEnv {
    pub enable_shell: bool,
    pub enable_fetch: bool,
    pub enable_organs: bool,
    pub enable_preference_learning: bool,
    pub cognitive_judge: bool,
    pub cognitive_council: bool,
}

impl BackendCapabilityEnv {
    /// The (variable, "1") pairs injected into the sidecar. Only enabled
    /// capabilities appear; the CLI treats absence as OFF.
    pub fn env_pairs(&self) -> Vec<(&'static str, &'static str)> {
        let mut pairs = Vec::new();
        if self.enable_shell {
            pairs.push(("APEIRETH_ENABLE_SHELL", "1"));
        }
        if self.enable_fetch {
            pairs.push(("APEIRETH_ENABLE_FETCH", "1"));
        }
        if self.enable_organs {
            pairs.push(("APEIRETH_ENABLE_ORGANS", "1"));
        }
        if self.enable_preference_learning {
            pairs.push(("APEIRETH_ENABLE_PREFERENCE_LEARNING", "1"));
        }
        if self.cognitive_judge {
            pairs.push(("APEIRETH_COGNITIVE_JUDGE", "1"));
        }
        if self.cognitive_council {
            pairs.push(("APEIRETH_COGNITIVE_COUNCIL", "1"));
        }
        pairs
    }

    /// True when no capability would be enabled.
    pub fn is_empty(&self) -> bool {
        self.env_pairs().is_empty()
    }
}

/// Body for the gateway's hot-config endpoint (`POST /v1/admin/config`).
///
/// The canonical gateway may serve this endpoint to hot-apply configuration
/// without a process restart; older gateways return 404/405, which the caller
/// turns back into the legacy restart path.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct AdminConfigRequest {
    provider: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    capabilities: BackendCapabilityEnv,
}

impl AdminConfigRequest {
    /// Build the request from the merged provider env + capability toggles.
    ///
    /// Only the active provider family is emitted. When several families are
    /// populated, the first in struct order wins (openai > minimax > anthropic).
    fn from_env(provider: &BackendProviderEnv, capabilities: &BackendCapabilityEnv) -> Self {
        let (name, base_url, api_key, model) = if provider.openai_api_key.is_some()
            || provider.openai_url.is_some()
            || provider.openai_models.is_some()
        {
            (
                Some(keychain::PROVIDER_OPENAI),
                provider.openai_url.clone(),
                provider.openai_api_key.clone(),
                provider.openai_models.clone(),
            )
        } else if provider.minimax_api_key.is_some()
            || provider.minimax_url.is_some()
            || provider.minimax_models.is_some()
        {
            (
                Some(keychain::PROVIDER_MINIMAX),
                provider.minimax_url.clone(),
                provider.minimax_api_key.clone(),
                provider.minimax_models.clone(),
            )
        } else if provider.anthropic_api_key.is_some()
            || provider.anthropic_url.is_some()
            || provider.anthropic_models.is_some()
        {
            (
                Some(keychain::PROVIDER_ANTHROPIC),
                provider.anthropic_url.clone(),
                provider.anthropic_api_key.clone(),
                provider.anthropic_models.clone(),
            )
        } else {
            (None, None, None, None)
        };

        Self {
            provider: name.map(str::to_string),
            base_url,
            api_key,
            model,
            capabilities: capabilities.clone(),
        }
    }
}

/// Outcome of a gateway hot-config apply attempt.
enum HotApply {
    /// The gateway accepted the config; no restart is needed.
    Applied,
    /// The gateway does not support `/v1/admin/config` (404/405) — old build.
    Unsupported,
    /// The request failed for another reason (network / server error).
    Failed(String),
}

impl Default for BackendInfo {
    fn default() -> Self {
        Self {
            state: BackendState::Stopped,
            ownership: BackendOwnership::External,
            pid: None,
            endpoint: None,
            port: None,
            started_at: None,
            restart_count: 0,
            last_exit_code: None,
            last_error: None,
            backend_version: None,
        }
    }
}

/// A live owned child. Port and start time live on [`BackendInfo`] rather than
/// being duplicated here, so there is one source of truth for diagnostics.
struct BackendProcess {
    child: Child,
    pid: u32,
}

pub struct BackendSupervisor {
    info: Arc<RwLock<BackendInfo>>,
    process: Arc<RwLock<Option<BackendProcess>>>,
    logger: Option<Arc<DesktopLogger>>,
    provider_env: RwLock<BackendProviderEnv>,
    capability_env: RwLock<BackendCapabilityEnv>,
    workspace_dir: RwLock<Option<PathBuf>>,
}

impl BackendSupervisor {
    /// Base constructor. Production always attaches a logger via
    /// [`Self::with_logger`]; tests use `build(None)` when no log file is wanted.
    fn build(logger: Option<Arc<DesktopLogger>>) -> Self {
        // Restore the persisted non-secret provider config (endpoints/models),
        // capability toggles, and workspace dir so the very first spawn already
        // reflects the user's last settings. Keys are deliberately NOT persisted
        // to the app-data config; they live in memory for the session and are
        // otherwise restored from the OS keychain at spawn (env > keychain).
        let persisted_provider = logger
            .as_ref()
            .and_then(|l| Self::load_persisted_provider_env(l))
            .unwrap_or_default()
            .without_secrets();
        let persisted_capabilities = logger
            .as_ref()
            .and_then(|l| Self::load_persisted_capability_env(l))
            .unwrap_or_default();
        let persisted_workspace = logger
            .as_ref()
            .map(|l| Self::logger_app_data_dir(l))
            .and_then(|dir| workspace::load_workspace_dir(&dir));
        Self {
            info: Arc::new(RwLock::new(BackendInfo::default())),
            process: Arc::new(RwLock::new(None)),
            logger,
            provider_env: RwLock::new(persisted_provider),
            capability_env: RwLock::new(persisted_capabilities),
            workspace_dir: RwLock::new(persisted_workspace),
        }
    }

    /// App-data directory that owns the logs (e.g. `%LOCALAPPDATA%\Apeireth`).
    /// `None` for logger-less test supervisors, where tests control the
    /// sidecar's store paths through their own environment.
    fn app_data_dir(&self) -> Option<PathBuf> {
        self.logger.as_ref().map(|l| Self::logger_app_data_dir(l))
    }

    /// The directory that owns the logs and every app-data config file.
    fn logger_app_data_dir(logger: &DesktopLogger) -> PathBuf {
        logger
            .log_directory()
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| logger.log_directory().to_path_buf())
    }

    /// App-data file for the non-secret part of the provider environment.
    fn provider_env_path(logger: &DesktopLogger) -> PathBuf {
        logger
            .log_directory()
            .parent()
            .map(|dir| dir.join("backend-provider-env.json"))
            .unwrap_or_else(|| logger.log_directory().join("backend-provider-env.json"))
    }

    /// App-data file for the capability toggles (no secrets involved).
    fn capability_env_path(logger: &DesktopLogger) -> PathBuf {
        logger
            .log_directory()
            .parent()
            .map(|dir| dir.join("backend-capability-env.json"))
            .unwrap_or_else(|| logger.log_directory().join("backend-capability-env.json"))
    }

    fn load_persisted_provider_env(logger: &DesktopLogger) -> Option<BackendProviderEnv> {
        let path = Self::provider_env_path(logger);
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<BackendProviderEnv>(&raw)
            .ok()
            .map(BackendProviderEnv::sanitized)
            .map(|env| env.without_secrets())
    }

    fn load_persisted_capability_env(logger: &DesktopLogger) -> Option<BackendCapabilityEnv> {
        let path = Self::capability_env_path(logger);
        let raw = std::fs::read_to_string(path).ok()?;
        serde_json::from_str::<BackendCapabilityEnv>(&raw).ok()
    }

    /// Persist only the non-secret part of the provider environment so the
    /// next launch restores endpoints/models without a restart cycle.
    fn persist_provider_env(&self, env: &BackendProviderEnv) {
        let Some(logger) = &self.logger else {
            return;
        };
        let path = Self::provider_env_path(logger);
        let Ok(json) = serde_json::to_string_pretty(&env.without_secrets()) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, json);
    }

    /// Persist capability toggles (no secrets) for the next launch.
    fn persist_capability_env(&self, env: &BackendCapabilityEnv) {
        let Some(logger) = &self.logger else {
            return;
        };
        let path = Self::capability_env_path(logger);
        let Ok(json) = serde_json::to_string_pretty(env) else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, json);
    }

    /// Attach a persistent logger so backend stdout/stderr reaches
    /// `apeireth-backend.log` and lifecycle events reach the desktop log.
    pub fn with_logger(logger: Arc<DesktopLogger>) -> Self {
        Self::build(Some(logger))
    }

    /// A logger-less supervisor for integration tests.
    ///
    /// Tests drive the real lifecycle but must not append to the user's actual
    /// log files, so no logger is attached.
    #[doc(hidden)]
    pub fn new_for_test() -> Self {
        Self::build(None)
    }

    /// Test-only production-shaped supervisor whose logs and app-data anchors
    /// live under `dir` (the parent of `dir\logs` becomes the app-data dir).
    /// Used by the regression test proving the sidecar's stores land in
    /// app-data regardless of the launch CWD.
    #[doc(hidden)]
    pub fn with_logger_in_dir(dir: PathBuf) -> Result<Self, String> {
        let logger = DesktopLogger::new_in_dir(dir.join("logs"))?;
        Ok(Self::with_logger(Arc::new(logger)))
    }

    /// Expose dev-build resolution so an integration test can report a missing
    /// backend instead of silently passing with nothing to spawn.
    #[doc(hidden)]
    pub fn resolve_dev_backend_for_test() -> Option<PathBuf> {
        Self::resolve_dev_backend()
    }

    fn log_desktop(&self, level: LogLevel, message: &str) {
        if let Some(logger) = &self.logger {
            logger.log_desktop(level, message);
        }
    }

    /// Drain one child pipe line-by-line into the backend log.
    ///
    /// The drain task is spawned even when no logger is attached. Dropping a
    /// piped stdout/stderr handle without a reader closes the pipe; the
    /// canonical CLI then panics on its first `eprintln!` (Windows exit 101)
    /// and never binds the gateway. Tests use a logger-less supervisor, and a
    /// production logger failure must not kill the child the same way.
    ///
    /// Each logged line is redacted by [`DesktopLogger::log_backend`], so
    /// provider credentials echoed by the runtime never reach disk.
    fn pump_stream<R>(&self, stream: R, channel: &'static str)
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let logger = self.logger.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(logger) = &logger {
                    logger.log_backend(&format!("[{channel}] {line}"));
                }
            }
        });
    }

    /// Kill and reap an owned child after a failed start so a dead gateway
    /// cannot hold the session database or a TCP port.
    async fn reclaim_child(&self) {
        let mut process = self.process.write().await;
        if let Some(mut backend) = process.take() {
            let _ = backend.child.start_kill();
            let _ = tokio::time::timeout(Duration::from_secs(2), backend.child.wait()).await;
        }
    }

    /// Get current backend info (safe for frontend)
    pub async fn info(&self) -> BackendInfo {
        self.info.read().await.clone()
    }

    /// Start the owned backend process
    pub async fn start(&self) -> Result<String, String> {
        let mut info = self.info.write().await;

        if info.state == BackendState::Starting || info.state == BackendState::Ready {
            return Err("Backend already starting or running".to_string());
        }

        info.state = BackendState::Starting;
        info.last_error = None;
        drop(info);

        // Locate bundled backend executable
        let backend_path = self.resolve_backend_binary()?;

        // Select free port
        let port = self.select_free_port().await?;

        self.log_desktop(
            LogLevel::Info,
            &format!("backend.spawn path={backend_path} port={port}"),
        );

        // Spawn backend process
        match self.spawn_backend(&backend_path, port).await {
            Ok(mut child) => {
                let pid = child.id().ok_or("Failed to get backend PID")?;
                let started_at = Instant::now();

                // Drain stdout/stderr into apeireth-backend.log. Taking the
                // pipes here means the child never blocks on a full OS buffer.
                if let Some(stdout) = child.stdout.take() {
                    self.pump_stream(stdout, "stdout");
                }
                if let Some(stderr) = child.stderr.take() {
                    self.pump_stream(stderr, "stderr");
                }
                self.log_desktop(LogLevel::Info, &format!("backend.spawned pid={pid} port={port}"));
                self.watch_for_exit(pid);

                // Store process handle
                let mut process = self.process.write().await;
                *process = Some(BackendProcess { child, pid });
                drop(process);

                // Update info
                let mut info = self.info.write().await;
                info.pid = Some(pid);
                info.port = Some(port);
                info.endpoint = Some(format!("http://127.0.0.1:{}", port));
                info.started_at = Some(started_at);
                info.ownership = BackendOwnership::OwnedByDesktop;
                drop(info);

                // Probe for readiness. A timeout or child death must land in
                // Failed rather than leaving the machine stuck in Starting,
                // and the child must be reaped so the next start is not
                // blocked by a leaked sqlite lock.
                if let Err(error) = self.wait_for_ready(port).await {
                    self.reclaim_child().await;
                    let mut info = self.info.write().await;
                    info.state = BackendState::Failed;
                    info.last_error = Some(error.clone());
                    info.pid = None;
                    info.endpoint = None;
                    info.port = None;
                    drop(info);
                    self.log_desktop(LogLevel::Error, &format!("backend.ready_failed {error}"));
                    return Err(error);
                }

                // Transition to Ready
                let mut info = self.info.write().await;
                info.state = BackendState::Ready;
                let endpoint = info.endpoint.clone().unwrap();
                let latency_ms = started_at.elapsed().as_millis();
                drop(info);
                self.log_desktop(
                    LogLevel::Info,
                    &format!("backend.ready pid={pid} port={port} latency_ms={latency_ms}"),
                );

                Ok(format!("Backend started at {} (PID: {})", endpoint, pid))
            }
            Err(e) => {
                let mut info = self.info.write().await;
                info.state = BackendState::Failed;
                info.last_error = Some(e.clone());
                drop(info);
                self.log_desktop(LogLevel::Error, &format!("backend.spawn_failed {e}"));
                Err(e)
            }
        }
    }

    /// Stop the owned backend process
    pub async fn stop(&self) -> Result<String, String> {
        let mut info = self.info.write().await;

        if info.ownership != BackendOwnership::OwnedByDesktop {
            return Err("Cannot stop external backend".to_string());
        }

        if info.state == BackendState::Stopped {
            return Ok("Backend already stopped".to_string());
        }

        info.state = BackendState::Stopping;
        let pid = info.pid;
        drop(info);

        let mut process = self.process.write().await;
        if let Some(mut backend_process) = process.take() {
            // Attempt graceful shutdown first
            let _ = backend_process.child.start_kill();

            // Wait bounded for exit
            let timeout = Duration::from_secs(5);
            match tokio::time::timeout(timeout, backend_process.child.wait()).await {
                Ok(Ok(status)) => {
                    let mut info = self.info.write().await;
                    info.state = BackendState::Stopped;
                    info.last_exit_code = status.code();
                    info.pid = None;
                    info.endpoint = None;
                    info.port = None;
                    drop(info);
                    self.log_desktop(
                        LogLevel::Info,
                        &format!("backend.stopped pid={pid:?} exit_code={:?}", status.code()),
                    );

                    Ok(format!("Backend stopped (PID: {:?}, exit: {:?})", pid, status.code()))
                }
                Ok(Err(e)) => {
                    let mut info = self.info.write().await;
                    info.state = BackendState::Failed;
                    info.last_error = Some(format!("Wait failed: {}", e));
                    drop(info);
                    self.log_desktop(LogLevel::Error, &format!("backend.stop_failed {e}"));
                    Err(format!("Failed to wait for backend exit: {}", e))
                }
                Err(_) => {
                    // Timeout - force kill
                    let _ = backend_process.child.kill().await;
                    let mut info = self.info.write().await;
                    info.state = BackendState::Stopped;
                    info.last_error = Some("Forced kill after timeout".to_string());
                    info.pid = None;
                    info.endpoint = None;
                    info.port = None;
                    drop(info);
                    self.log_desktop(
                        LogLevel::Warn,
                        &format!("backend.force_killed pid={pid:?} reason=graceful_timeout"),
                    );

                    Ok(format!("Backend force-killed (PID: {:?})", pid))
                }
            }
        } else {
            let mut info = self.info.write().await;
            info.state = BackendState::Stopped;
            Ok("No backend process to stop".to_string())
        }
    }

    /// Restart the backend (stop + start)
    pub async fn restart(&self) -> Result<String, String> {
        let _ = self.stop().await;

        let mut info = self.info.write().await;
        info.restart_count += 1;
        drop(info);

        tokio::time::sleep(Duration::from_millis(500)).await;
        self.start().await
    }

    /// Apply the Settings-UI provider configuration.
    ///
    /// Compatibility wrapper over [`Self::apply_backend_config`]; the frontend
    /// uses the combined command so a settings save with both provider and
    /// capability changes restarts the backend exactly once.
    pub async fn apply_provider_env(
        &self,
        env: BackendProviderEnv,
    ) -> Result<BackendInfo, String> {
        self.apply_backend_config(Some(env), None).await
    }

    /// Apply the Settings-UI configuration (provider env and/or capability
    /// toggles).
    ///
    /// Stores both (keys in memory only; endpoints/models and toggles also
    /// persisted for the next launch) and makes a running backend pick the
    /// changes up. The single IPC call stays a single application:
    ///
    /// - `Ready`: hot-apply via `POST /v1/admin/config` when the gateway
    ///   supports it; a pre-admin gateway (404/405) falls back to the legacy
    ///   whole-process restart;
    /// - `Starting`/`Stopping`: wait for the in-flight transition to settle,
    ///   then hot-apply/restart or start so the change is deterministically
    ///   applied;
    /// - `Failed`: start again with the new environment (a configuration fix
    ///   is exactly what should recover a failed boot);
    /// - `Stopped`: start with the new environment (applying a config implies
    ///   the sidecar should be running).
    ///
    /// An unchanged configuration is a no-op (no restart), so a settings save
    /// that only touched UI state never bounces the gateway.
    pub async fn apply_backend_config(
        &self,
        provider: Option<BackendProviderEnv>,
        capabilities: Option<BackendCapabilityEnv>,
    ) -> Result<BackendInfo, String> {
        let provider_changed = if let Some(env) = provider {
            let sanitized = env.sanitized();
            let changed = {
                let mut current = self.provider_env.write().await;
                if *current == sanitized {
                    false
                } else {
                    *current = sanitized.clone();
                    true
                }
            };
            self.persist_provider_env(&sanitized);
            changed
        } else {
            false
        };
        let capabilities_changed = if let Some(env) = capabilities {
            let changed = {
                let mut current = self.capability_env.write().await;
                if *current == env {
                    false
                } else {
                    *current = env.clone();
                    true
                }
            };
            self.persist_capability_env(&env);
            changed
        } else {
            false
        };

        if !provider_changed && !capabilities_changed {
            return Ok(self.info().await);
        }

        let state = self.info.read().await.state.clone();
        match state {
            BackendState::Stopped => {
                self.start().await?;
            }
            BackendState::Ready => {
                self.apply_to_ready_gateway().await?;
            }
            BackendState::Failed => {
                self.start().await?;
            }
            BackendState::Starting | BackendState::Stopping => {
                self.wait_for_settle().await;
                let settled = self.info.read().await.state.clone();
                match settled {
                    BackendState::Ready => {
                        self.apply_to_ready_gateway().await?;
                    }
                    BackendState::Stopped | BackendState::Failed => {
                        self.start().await?;
                    }
                    _ => {}
                }
            }
        }
        Ok(self.info().await)
    }

    /// Apply the current config to a gateway that is already `Ready`: hot-apply
    /// first, then fall back to the legacy restart when the gateway has no
    /// admin-config endpoint (or the hot apply fails for any reason).
    async fn apply_to_ready_gateway(&self) -> Result<(), String> {
        match self.try_hot_apply().await {
            HotApply::Applied => {
                self.log_desktop(LogLevel::Info, "backend.hot_apply applied");
                Ok(())
            }
            HotApply::Unsupported => {
                self.log_desktop(
                    LogLevel::Info,
                    "backend.hot_apply unsupported (pre-admin gateway); restarting",
                );
                self.restart().await.map(|_| ())
            }
            HotApply::Failed(error) => {
                self.log_desktop(
                    LogLevel::Warn,
                    &format!("backend.hot_apply failed ({error}); restarting"),
                );
                self.restart().await.map(|_| ())
            }
        }
    }

    /// POST the current config to `{endpoint}/v1/admin/config`.
    async fn try_hot_apply(&self) -> HotApply {
        let Some(endpoint) = self.info.read().await.endpoint.clone() else {
            return HotApply::Failed("gateway endpoint not available".to_string());
        };
        let provider_env = self.provider_env.read().await.clone();
        let provider_env =
            provider_env.with_keychain_fallback(|provider| keychain::get_provider_key(provider));
        let capabilities = self.capability_env.read().await.clone();
        let payload = AdminConfigRequest::from_env(&provider_env, &capabilities);
        let url = format!("{endpoint}/v1/admin/config");

        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
        {
            Ok(client) => client,
            Err(error) => return HotApply::Failed(format!("client build failed: {error}")),
        };

        match client.post(&url).json(&payload).send().await {
            Ok(response) if response.status().is_success() => HotApply::Applied,
            Ok(response)
                if response.status() == reqwest::StatusCode::NOT_FOUND
                    || response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED =>
            {
                HotApply::Unsupported
            }
            Ok(response) => HotApply::Failed(format!("HTTP {}", response.status())),
            Err(error) => HotApply::Failed(error.to_string()),
        }
    }

    /// Fetch the gateway's effective config (`GET /v1/admin/config`) and pass
    /// the JSON body through for the settings UI to echo back.
    pub async fn get_gateway_effective_config(&self) -> Result<serde_json::Value, String> {
        let endpoint = self
            .info
            .read()
            .await
            .endpoint
            .clone()
            .ok_or_else(|| "gateway not running".to_string())?;
        let url = format!("{endpoint}/v1/admin/config");
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("failed to build admin client: {e}"))?;
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("gateway admin config request failed: {e}"))?;

        if response.status().is_success() {
            response
                .json::<serde_json::Value>()
                .await
                .map_err(|e| format!("gateway admin config returned invalid JSON: {e}"))
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(format!("gateway admin config unavailable (HTTP {status}): {body}"))
        }
    }

    /// The current workspace directory, or an empty string when unset.
    pub async fn get_workspace_dir(&self) -> String {
        self.workspace_dir
            .read()
            .await
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    /// Validate, persist and apply a workspace directory, returning the new
    /// (normalized) value.
    pub async fn set_workspace_dir(&self, dir: String) -> Result<String, String> {
        let raw = PathBuf::from(dir.trim());
        if raw.as_os_str().is_empty() {
            return Err("workspace directory must not be empty".to_string());
        }
        let normalized = workspace::normalize_dir(&raw);
        workspace::ensure_writable_dir(&normalized)?;

        // The path is non-secret, so plain JSON under app-data is fine.
        if let Some(app_data) = self.app_data_dir() {
            workspace::persist_workspace_dir(&app_data, &normalized)?;
        }

        let value = normalized.to_string_lossy().to_string();
        *self.workspace_dir.write().await = Some(normalized);
        Ok(value)
    }

    /// Common workspace candidates: home, documents, desktop, last-used.
    pub async fn list_workspace_suggestions(&self) -> Vec<String> {
        let last = self.workspace_dir.read().await.clone();
        workspace::workspace_suggestions(last.as_deref())
    }

    /// Wait until the state machine leaves its transitional states.
    async fn wait_for_settle(&self) {
        for _ in 0..60 {
            let state = self.info.read().await.state.clone();
            if state != BackendState::Starting && state != BackendState::Stopping {
                return;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Platform executable name for the canonical CLI.
    pub fn backend_executable_name() -> &'static str {
        if cfg!(windows) {
            "apeireth.exe"
        } else {
            "apeireth"
        }
    }

    /// Find the packaged backend next to the running executable.
    ///
    /// This is the installed layout: Tauri places `externalBin` sidecars in the
    /// same directory as the app binary, so an install needs no source tree,
    /// no Cargo, and no `target/`.
    fn resolve_bundled_backend() -> Option<PathBuf> {
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?;
        let name = Self::backend_executable_name();

        // Alongside the app binary (Windows/Linux), then macOS .app layouts.
        let candidates = [
            dir.join(name),
            dir.join("resources").join(name),
            dir.join("../Resources").join(name),
        ];
        candidates.into_iter().find(|path| path.is_file())
    }

    /// Walk up from `start` to the Cargo workspace root that owns the CLI.
    ///
    /// The Tauri crate is its own isolated workspace whose cwd during
    /// `tauri dev` is `frontend/companion-desktop/src-tauri`, so joining
    /// `target/` onto the cwd finds the *desktop* build directory rather than
    /// the workspace that actually builds `apeireth`. Anchoring on
    /// `crates/adapters/cli` identifies the correct root regardless of cwd.
    fn find_cli_workspace_root(start: &Path) -> Option<PathBuf> {
        start
            .ancestors()
            .find(|dir| dir.join("crates/adapters/cli/Cargo.toml").is_file())
            .map(Path::to_path_buf)
    }

    /// Resolve a development build of the canonical CLI from the workspace.
    fn resolve_dev_backend() -> Option<PathBuf> {
        let name = Self::backend_executable_name();
        let anchors = [
            std::env::current_dir().ok(),
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(Path::to_path_buf)),
            Some(PathBuf::from(env!("CARGO_MANIFEST_DIR"))),
        ];

        for anchor in anchors.into_iter().flatten() {
            let Some(root) = Self::find_cli_workspace_root(&anchor) else {
                continue;
            };
            // Release before debug: an optimized backend is preferred when both exist.
            for profile in ["release", "debug"] {
                let candidate = root.join("target").join(profile).join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }

    /// Resolve the canonical backend executable.
    ///
    /// Bundled sidecar wins in every configuration; a workspace build is only a
    /// development fallback. An installed app therefore never depends on a
    /// source checkout, and a dev run still works before packaging exists.
    fn resolve_backend_binary(&self) -> Result<String, String> {
        if let Some(path) = Self::resolve_bundled_backend() {
            return Ok(path.to_string_lossy().to_string());
        }

        if let Some(path) = Self::resolve_dev_backend() {
            self.log_desktop(
                LogLevel::Warn,
                "backend.resolve source=workspace_dev_build (no bundled sidecar found)",
            );
            return Ok(path.to_string_lossy().to_string());
        }

        Err(format!(
            "canonical backend executable '{}' not found beside the app or in a workspace target directory",
            Self::backend_executable_name()
        ))
    }

    /// Select a free localhost port
    async fn select_free_port(&self) -> Result<u16, String> {
        // Try to bind to port 0 to let OS assign a free port
        let listener = std::net::TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to bind ephemeral port: {}", e))?;

        let port = listener.local_addr()
            .map_err(|e| format!("Failed to get local addr: {}", e))?
            .port();

        drop(listener); // Release the port
        Ok(port)
    }

    /// Canonical spawn arguments, verified against the CLI's own help text:
    /// `apeireth gateway serve [--port PORT]`.
    pub fn spawn_args(port: u16) -> Vec<String> {
        vec![
            "gateway".to_string(),
            "serve".to_string(),
            "--port".to_string(),
            port.to_string(),
        ]
    }

    /// Spawn the backend process
    async fn spawn_backend(&self, binary_path: &str, port: u16) -> Result<Child, String> {
        let mut cmd = Command::new(binary_path);
        cmd.args(Self::spawn_args(port))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(true);

        // Inject the Settings-UI provider configuration as the child's
        // environment. This is the packaged-desktop equivalent of the user
        // exporting provider env vars before running `apeireth gateway serve`.
        // Priority: in-memory env > OS keychain (env wins when a key was pushed
        // this session; the keychain restores it on later launches).
        let provider_env = self.provider_env.read().await.clone();
        let provider_env =
            provider_env.with_keychain_fallback(|provider| keychain::get_provider_key(provider));
        for (key, value) in provider_env.env_pairs() {
            cmd.env(key, value);
        }
        // Same for the advanced-capability toggles (fail-closed: only "1"
        // values are emitted).
        let capability_env = self.capability_env.read().await.clone();
        for (key, value) in capability_env.env_pairs() {
            cmd.env(key, value);
        }

        // Anchor the sidecar's persistent stores to absolute paths and pin its
        // working directory to app-data. The canonical CLI defaults its
        // session/cognitive DBs to RELATIVE `.apeireth/...` paths, which break
        // when the desktop is launched from a non-writable CWD (Start-menu
        // shortcuts run with CWD = System32 → "failed to create parent
        // directory: 拒绝访问 (os error 5)", gateway exits 1 — real-world
        // boot failure, 2026-09-28). With a logger attached (production),
        // app data = the directory that owns `logs/`.
        if let Some(data_dir) = self.app_data_dir() {
            let workspace_dir = self.workspace_dir.read().await.clone();
            let store_dir = workspace::resolve_store_dir(workspace_dir.as_deref(), Some(&data_dir))
                .unwrap_or_else(|| data_dir.join("data"));

            // Explicit ABSOLUTE env vars are the highest priority; anchor the
            // defaults otherwise. Relative env values (e.g. a stale
            // `.apeireth/...` left in the inherited environment) are treated
            // as absent: honoring them would resurrect the 2026-09-28 CWD
            // boot failure (System32 + relative path = Access Denied).
            let session_env = std::env::var("APEIRETH_SESSION_DB").ok();
            let cognitive_env = std::env::var("APEIRETH_COGNITIVE_DB").ok();
            let anchor_session = session_env.as_deref().is_none_or(|v| !Path::new(v).is_absolute());
            let anchor_cognitive =
                cognitive_env.as_deref().is_none_or(|v| !Path::new(v).is_absolute());

            // Only create the store dir when we are actually going to anchor
            // paths into it: an explicit absolute env store dir is the user's
            // own location and must not cause app-data side effects.
            if anchor_session || anchor_cognitive {
                let _ = std::fs::create_dir_all(&store_dir);
            }
            if anchor_session {
                cmd.env("APEIRETH_SESSION_DB", store_dir.join("sessions.sqlite3"));
            }
            if anchor_cognitive {
                cmd.env("APEIRETH_COGNITIVE_DB", store_dir.join("cognitive.sqlite3"));
            }
            cmd.current_dir(&data_dir);
        }

        // Keep the child in the app's lifetime, not the user's screen: without
        // CREATE_NO_WINDOW a console window flashes on every launch.
        // `tokio::process::Command` exposes `creation_flags` inherently on
        // Windows, so no std extension trait import is needed.
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        cmd.spawn()
            .map_err(|e| format!("Failed to spawn backend: {}", e))
    }

    /// Watch an owned child and record its exit.
    ///
    /// Without this, a backend that dies after reaching Ready leaves the UI
    /// showing a healthy state forever. Exit code and restart bookkeeping are
    /// recorded so the diagnostics surface can tell the truth.
    fn watch_for_exit(&self, pid: u32) {
        let info = Arc::clone(&self.info);
        let process = Arc::clone(&self.process);
        let logger = self.logger.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(750)).await;

                let mut guard = process.write().await;
                let Some(backend) = guard.as_mut() else {
                    return; // Deliberate stop() took the handle; nothing to report.
                };
                if backend.pid != pid {
                    return; // A restart replaced this child; its own watcher owns it.
                }

                match backend.child.try_wait() {
                    Ok(Some(status)) => {
                        guard.take();
                        drop(guard);

                        let mut info = info.write().await;
                        // Only an unexpected death is a failure; a requested
                        // stop has already moved the state to Stopping/Stopped.
                        if matches!(info.state, BackendState::Starting | BackendState::Ready) {
                            info.state = BackendState::Failed;
                            info.last_exit_code = status.code();
                            info.last_error = Some(format!(
                                "backend exited unexpectedly (code {:?})",
                                status.code()
                            ));
                            info.pid = None;
                            info.endpoint = None;
                            info.port = None;
                            if let Some(logger) = &logger {
                                logger.log_desktop(
                                    LogLevel::Error,
                                    &format!(
                                        "backend.exited_unexpectedly pid={pid} exit_code={:?}",
                                        status.code()
                                    ),
                                );
                            }
                        }
                        return;
                    }
                    Ok(None) => continue, // Still running.
                    Err(error) => {
                        drop(guard);
                        if let Some(logger) = &logger {
                            logger.log_desktop(
                                LogLevel::Warn,
                                &format!("backend.watch_failed pid={pid} error={error}"),
                            );
                        }
                        return;
                    }
                }
            }
        });
    }

    /// Wait for backend to become ready (health probe)
    async fn wait_for_ready(&self, port: u16) -> Result<(), String> {
        let endpoint = format!("http://127.0.0.1:{}/health", port);
        let timeout = Duration::from_secs(15);
        let start = Instant::now();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|error| format!("failed to build health client: {error}"))?;

        loop {
            if start.elapsed() > timeout {
                return Err(format!("Backend startup timeout after {:?}", timeout));
            }

            // A child that dies during bootstrap (broken stdio, bind failure,
            // sqlite lock) must fail immediately. Waiting out the 15s budget
            // hides the real exit and poisons the next start.
            {
                let info = self.info.read().await;
                if info.state == BackendState::Failed {
                    return Err(info.last_error.clone().unwrap_or_else(|| {
                        "backend failed during startup".to_string()
                    }));
                }
            }
            {
                let mut guard = self.process.write().await;
                if let Some(backend) = guard.as_mut() {
                    if let Ok(Some(status)) = backend.child.try_wait() {
                        return Err(format!(
                            "backend exited during startup (code {:?})",
                            status.code()
                        ));
                    }
                }
            }

            match client.get(&endpoint).send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }
}

// No `Drop` impl: stopping the child needs async, which `drop` cannot await.
// Shutdown of the owned backend is driven by the `RunEvent::ExitRequested`
// handler in `lib.rs`, which can block on `stop()`.

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::process::Command;

    /// Verified against the CLI's own help output:
    /// `apeireth gateway serve [--port PORT]`.
    #[test]
    fn spawn_args_match_canonical_cli_contract() {
        assert_eq!(
            BackendSupervisor::spawn_args(52719),
            vec!["gateway", "serve", "--port", "52719"]
        );
    }

    #[test]
    fn spawn_args_never_reference_legacy_backend() {
        let args = BackendSupervisor::spawn_args(8080).join(" ");
        for legacy in ["companion_serve", "8090", "target/debug", "examples"] {
            assert!(
                !args.contains(legacy),
                "legacy token {legacy:?} leaked into spawn args: {args}"
            );
        }
    }

    /// The injected variable names must match the canonical CLI contract
    /// (see `crates/adapters/cli` + `crates/engine/provider/src/credentials.rs`).
    #[test]
    fn provider_env_pairs_match_canonical_cli_contract() {
        let env = BackendProviderEnv {
            openai_api_key: Some("sk-openai".into()),
            openai_url: Some("https://api.deepseek.com/v1".into()),
            openai_models: Some("deepseek-v4-flash".into()),
            minimax_api_key: Some("sk-mm".into()),
            minimax_url: Some("https://api.minimax.chat/v1".into()),
            minimax_models: Some("minimax-m3".into()),
            anthropic_api_key: Some("sk-ant".into()),
            anthropic_url: Some("https://api.anthropic.com".into()),
            anthropic_models: Some("claude-sonnet-4-5".into()),
        };
        let pairs: Vec<(&str, &str)> = env.env_pairs();
        let mut map = std::collections::HashMap::new();
        for (key, value) in pairs {
            map.insert(key, value);
        }
        assert_eq!(map["OPENAI_API_KEY"], "sk-openai");
        assert_eq!(map["APEIRETH_OPENAI_URL"], "https://api.deepseek.com/v1");
        assert_eq!(map["APEIRETH_OPENAI_MODELS"], "deepseek-v4-flash");
        assert_eq!(map["APEIRETH_API_KEY"], "sk-mm");
        assert_eq!(map["APEIRETH_API_URL"], "https://api.minimax.chat/v1");
        assert_eq!(map["APEIRETH_API_MODELS"], "minimax-m3");
        assert_eq!(map["APEIRETH_ANTHROPIC_KEY"], "sk-ant");
        assert_eq!(map["APEIRETH_ANTHROPIC_URL"], "https://api.anthropic.com");
        assert_eq!(map["APEIRETH_ANTHROPIC_MODELS"], "claude-sonnet-4-5");
        assert_eq!(map.len(), 9);
    }

    #[test]
    fn provider_env_sanitized_trims_and_drops_empties() {
        let env = BackendProviderEnv {
            openai_api_key: Some("  sk-key  ".into()),
            openai_url: Some("   ".into()),
            openai_models: Some("deepseek-v4-flash".into()),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(env.openai_api_key.as_deref(), Some("sk-key"));
        assert_eq!(env.openai_url, None);
        assert_eq!(env.openai_models.as_deref(), Some("deepseek-v4-flash"));
        assert!(BackendProviderEnv::default().is_empty());
    }

    #[test]
    fn provider_env_without_secrets_keeps_no_keys() {
        let env = BackendProviderEnv {
            openai_api_key: Some("sk-openai".into()),
            openai_url: Some("https://api.deepseek.com/v1".into()),
            minimax_api_key: Some("sk-mm".into()),
            anthropic_api_key: Some("sk-ant".into()),
            anthropic_url: Some("https://api.anthropic.com".into()),
            ..Default::default()
        };
        let stripped = env.without_secrets();
        assert_eq!(stripped.openai_api_key, None);
        assert_eq!(stripped.minimax_api_key, None);
        assert_eq!(stripped.anthropic_api_key, None);
        assert_eq!(stripped.openai_url.as_deref(), Some("https://api.deepseek.com/v1"));
        assert_eq!(stripped.anthropic_url.as_deref(), Some("https://api.anthropic.com"));
    }

    /// The serialized shape is what the frontend sends over IPC.
    #[test]
    fn provider_env_serde_shape_is_snake_case() {
        let env = BackendProviderEnv {
            openai_models: Some("deepseek-v4-flash".into()),
            ..Default::default()
        };
        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("\"openai_models\":\"deepseek-v4-flash\""), "{json}");
        let round: BackendProviderEnv = serde_json::from_str(&json).unwrap();
        assert_eq!(round, env);
    }

    #[test]
    fn provider_env_keychain_fallback_env_wins_over_keychain() {
        let env = BackendProviderEnv {
            openai_api_key: Some("sk-env".into()),
            ..Default::default()
        };
        let merged = env.with_keychain_fallback(|provider| Some(format!("sk-keychain-{provider}")));
        assert_eq!(merged.openai_api_key.as_deref(), Some("sk-env"));
        assert_eq!(
            merged.minimax_api_key.as_deref(),
            Some("sk-keychain-minimax")
        );
        assert_eq!(
            merged.anthropic_api_key.as_deref(),
            Some("sk-keychain-anthropic")
        );
    }

    #[test]
    fn provider_env_keychain_fallback_fills_missing_keys_only() {
        let env = BackendProviderEnv {
            minimax_url: Some("https://api.minimax.chat/v1".into()),
            ..Default::default()
        };
        let merged = env.with_keychain_fallback(|provider| match provider {
            keychain::PROVIDER_OPENAI => Some("sk-oai".into()),
            keychain::PROVIDER_MINIMAX => Some("sk-mm".into()),
            keychain::PROVIDER_ANTHROPIC => Some("sk-ant".into()),
            _ => None,
        });
        assert_eq!(merged.openai_api_key.as_deref(), Some("sk-oai"));
        assert_eq!(merged.minimax_api_key.as_deref(), Some("sk-mm"));
        assert_eq!(merged.anthropic_api_key.as_deref(), Some("sk-ant"));
        assert_eq!(
            merged.minimax_url.as_deref(),
            Some("https://api.minimax.chat/v1"),
            "non-key fields must pass through unchanged"
        );
    }

    #[test]
    fn admin_config_request_emits_active_provider_fields() {
        let provider = BackendProviderEnv {
            openai_api_key: Some("sk-oai".into()),
            openai_url: Some("https://api.deepseek.com/v1".into()),
            openai_models: Some("deepseek-v4-flash".into()),
            ..Default::default()
        };
        let caps = BackendCapabilityEnv {
            enable_shell: true,
            ..Default::default()
        };
        let request = AdminConfigRequest::from_env(&provider, &caps);
        assert_eq!(request.provider.as_deref(), Some("openai"));
        assert_eq!(request.base_url.as_deref(), Some("https://api.deepseek.com/v1"));
        assert_eq!(request.api_key.as_deref(), Some("sk-oai"));
        assert_eq!(request.model.as_deref(), Some("deepseek-v4-flash"));
        assert!(request.capabilities.enable_shell);

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"provider\":\"openai\""), "{json}");
        assert!(json.contains("\"base_url\""), "{json}");
        assert!(json.contains("\"api_key\":\"sk-oai\""), "{json}");
        assert!(json.contains("\"capabilities\""), "{json}");
    }

    /// Capability env names must match the canonical CLI knobs, and the
    /// fail-closed contract holds: only true emits "1", false emits nothing.
    #[test]
    fn capability_env_pairs_match_canonical_knobs_and_fail_closed() {
        let caps = BackendCapabilityEnv {
            enable_shell: true,
            enable_fetch: false,
            enable_organs: true,
            enable_preference_learning: false,
            cognitive_judge: true,
            cognitive_council: false,
        };
        let pairs = caps.env_pairs();
        let mut map = std::collections::HashMap::new();
        for (key, value) in pairs {
            map.insert(key, value);
        }
        assert_eq!(map["APEIRETH_ENABLE_SHELL"], "1");
        assert_eq!(map["APEIRETH_ENABLE_ORGANS"], "1");
        assert_eq!(map["APEIRETH_COGNITIVE_JUDGE"], "1");
        assert_eq!(map.len(), 3, "false toggles must emit nothing: {map:?}");
        assert!(BackendCapabilityEnv::default().is_empty());
    }

    #[test]
    fn executable_name_is_platform_correct() {
        let name = BackendSupervisor::backend_executable_name();
        if cfg!(windows) {
            assert_eq!(name, "apeireth.exe");
        } else {
            assert_eq!(name, "apeireth");
        }
    }

    #[test]
    fn fresh_supervisor_is_stopped_and_unowned() {
        let supervisor = BackendSupervisor::build(None);
        let info = tokio_block(supervisor.info());
        assert_eq!(info.state, BackendState::Stopped);
        assert_eq!(info.ownership, BackendOwnership::External);
        assert!(info.pid.is_none());
        assert!(info.endpoint.is_none());
        assert_eq!(info.restart_count, 0);
    }

    /// An external backend must never be killed by the desktop: a fresh
    /// supervisor owns nothing, so stop() has to refuse.
    #[test]
    fn stop_refuses_to_touch_external_backend() {
        let supervisor = BackendSupervisor::build(None);
        let result = tokio_block(supervisor.stop());
        assert!(result.is_err(), "expected refusal, got {result:?}");
        assert!(
            result.unwrap_err().contains("external"),
            "refusal should name external ownership"
        );
    }

    #[test]
    fn ephemeral_port_selection_yields_a_bindable_port() {
        let supervisor = BackendSupervisor::build(None);
        let port = tokio_block(supervisor.select_free_port()).expect("port");
        assert_ne!(port, 0);
        assert_ne!(port, 8090, "must never select the legacy companion port");
        // Released back to the OS, so it is bindable again.
        std::net::TcpListener::bind(("127.0.0.1", port)).expect("port should be free");
    }

    #[test]
    fn readiness_probe_times_out_on_a_dead_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let dead_port = listener.local_addr().expect("addr").port();
        drop(listener); // Nothing is listening now.

        let supervisor = BackendSupervisor::build(None);
        let result = tokio_block(async {
            tokio::time::timeout(
                Duration::from_secs(20),
                supervisor.wait_for_ready(dead_port),
            )
            .await
            .expect("probe should return, not hang")
        });
        assert!(result.is_err(), "probe must fail when nothing is listening");
    }

    /// A readiness timeout has to land in Failed with the reason recorded —
    /// leaving the machine in Starting would show a permanent false "starting".
    #[test]
    fn failed_readiness_records_failed_state() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let dead_port = listener.local_addr().expect("addr").port();
        drop(listener);

        let supervisor = BackendSupervisor::build(None);
        tokio_block(async {
            {
                let mut info = supervisor.info.write().await;
                info.state = BackendState::Starting;
            }
            if let Err(error) = supervisor.wait_for_ready(dead_port).await {
                let mut info = supervisor.info.write().await;
                info.state = BackendState::Failed;
                info.last_error = Some(error);
            }
        });

        let info = tokio_block(supervisor.info());
        assert_eq!(info.state, BackendState::Failed);
        assert!(info.last_error.is_some(), "failure reason must be recorded");
    }

    #[test]
    fn readiness_fails_fast_when_owned_child_already_exited() {
        let supervisor = BackendSupervisor::build(None);
        let elapsed = tokio_block(async {
            let mut child = {
                #[cfg(windows)]
                {
                    Command::new("cmd")
                        .args(["/C", "exit", "42"])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .stdin(std::process::Stdio::null())
                        .spawn()
                        .expect("spawn dummy child")
                }
                #[cfg(not(windows))]
                {
                    Command::new("sh")
                        .args(["-c", "exit 42"])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .stdin(std::process::Stdio::null())
                        .spawn()
                        .expect("spawn dummy child")
                }
            };
            let pid = child.id().expect("pid");
            let _ = child.wait().await;
            {
                let mut info = supervisor.info.write().await;
                info.state = BackendState::Starting;
                info.ownership = BackendOwnership::OwnedByDesktop;
            }
            {
                let mut process = supervisor.process.write().await;
                *process = Some(BackendProcess { child, pid });
            }
            let started = Instant::now();
            let result = supervisor.wait_for_ready(1).await;
            assert!(result.is_err(), "expected failure, got {result:?}");
            let message = result.unwrap_err();
            assert!(
                message.contains("exited during startup"),
                "must report child death, not a 15s timeout: {message}"
            );
            started.elapsed()
        });
        assert!(
            elapsed < Duration::from_secs(3),
            "child death must fail fast, took {elapsed:?}"
        );
    }

    #[test]
    fn backend_info_serializes_without_secrets() {
        let supervisor = BackendSupervisor::build(None);
        let info = tokio_block(supervisor.info());
        let json = serde_json::to_string(&info).expect("serialize");

        for forbidden in ["apiKey", "api_key", "Authorization", "Bearer", "sk-", "master_token"] {
            assert!(
                !json.contains(forbidden),
                "diagnostic payload must not carry {forbidden:?}: {json}"
            );
        }
        // The fields the diagnostics surface needs are present.
        for field in ["state", "ownership", "pid", "endpoint", "restart_count"] {
            assert!(json.contains(field), "missing diagnostic field {field:?}");
        }
    }

    #[test]
    fn dev_resolution_never_returns_a_legacy_path() {
        // Whether or not a dev build exists here, it must never resolve to the
        // historical companion backend.
        if let Some(path) = BackendSupervisor::resolve_dev_backend() {
            let text = path.to_string_lossy().to_ascii_lowercase();
            assert!(!text.contains("companion_serve"));
            assert!(!text.contains("examples"));
            assert!(
                text.ends_with(BackendSupervisor::backend_executable_name()),
                "resolved path should be the canonical CLI: {text}"
            );
        }
    }

    fn tokio_block<F: std::future::Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(future)
    }
}
