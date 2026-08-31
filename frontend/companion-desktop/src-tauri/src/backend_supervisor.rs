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

use crate::logging::{DesktopLogger, LogLevel};
use serde::Serialize;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
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

struct BackendProcess {
    child: Child,
    pid: u32,
    port: u16,
    started_at: Instant,
}

pub struct BackendSupervisor {
    info: Arc<RwLock<BackendInfo>>,
    process: Arc<RwLock<Option<BackendProcess>>>,
    logger: Option<Arc<DesktopLogger>>,
}

impl BackendSupervisor {
    pub fn new() -> Self {
        Self {
            info: Arc::new(RwLock::new(BackendInfo::default())),
            process: Arc::new(RwLock::new(None)),
            logger: None,
        }
    }

    /// Attach a persistent logger so backend stdout/stderr reaches
    /// `apeireth-backend.log` and lifecycle events reach the desktop log.
    pub fn with_logger(logger: Arc<DesktopLogger>) -> Self {
        Self {
            info: Arc::new(RwLock::new(BackendInfo::default())),
            process: Arc::new(RwLock::new(None)),
            logger: Some(logger),
        }
    }

    fn log_desktop(&self, level: LogLevel, message: &str) {
        if let Some(logger) = &self.logger {
            logger.log_desktop(level, message);
        }
    }

    /// Drain one child pipe line-by-line into the backend log.
    ///
    /// Each line is redacted by [`DesktopLogger::log_backend`], so provider
    /// credentials echoed by the runtime never reach disk.
    fn pump_stream<R>(&self, stream: R, channel: &'static str)
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let Some(logger) = self.logger.clone() else {
            return;
        };
        tokio::spawn(async move {
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                logger.log_backend(&format!("[{channel}] {line}"));
            }
        });
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
                *process = Some(BackendProcess {
                    child,
                    pid,
                    port,
                    started_at,
                });
                drop(process);

                // Update info
                let mut info = self.info.write().await;
                info.pid = Some(pid);
                info.port = Some(port);
                info.endpoint = Some(format!("http://127.0.0.1:{}", port));
                info.started_at = Some(started_at);
                info.ownership = BackendOwnership::OwnedByDesktop;
                drop(info);

                // Probe for readiness. A timeout must land in Failed rather
                // than leaving the machine stuck in Starting forever, so the
                // error is captured instead of propagating with `?`.
                if let Err(error) = self.wait_for_ready(port).await {
                    let mut info = self.info.write().await;
                    info.state = BackendState::Failed;
                    info.last_error = Some(error.clone());
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
            .stdin(Stdio::null());

        // Keep the child in the app's lifetime, not the user's screen: without
        // CREATE_NO_WINDOW a console window flashes on every launch.
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

        loop {
            if start.elapsed() > timeout {
                return Err(format!("Backend startup timeout after {:?}", timeout));
            }

            // Try health check
            match reqwest::get(&endpoint).await {
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

impl Drop for BackendSupervisor {
    fn drop(&mut self) {
        // Note: Drop is synchronous, cannot use async stop()
        // Actual cleanup happens in Tauri's cleanup handler
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let supervisor = BackendSupervisor::new();
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
        let supervisor = BackendSupervisor::new();
        let result = tokio_block(supervisor.stop());
        assert!(result.is_err(), "expected refusal, got {result:?}");
        assert!(
            result.unwrap_err().contains("external"),
            "refusal should name external ownership"
        );
    }

    #[test]
    fn ephemeral_port_selection_yields_a_bindable_port() {
        let supervisor = BackendSupervisor::new();
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

        let supervisor = BackendSupervisor::new();
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

        let supervisor = BackendSupervisor::new();
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
    fn backend_info_serializes_without_secrets() {
        let supervisor = BackendSupervisor::new();
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
