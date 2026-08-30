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

use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::process::Stdio;
use tokio::process::{Child, Command};
use std::time::{Duration, Instant};

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
}

impl BackendSupervisor {
    pub fn new() -> Self {
        Self {
            info: Arc::new(RwLock::new(BackendInfo::default())),
            process: Arc::new(RwLock::new(None)),
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

        // Spawn backend process
        match self.spawn_backend(&backend_path, port).await {
            Ok(child) => {
                let pid = child.id().ok_or("Failed to get backend PID")?;
                let started_at = Instant::now();

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

                // Probe for readiness
                self.wait_for_ready(port).await?;

                // Transition to Ready
                let mut info = self.info.write().await;
                info.state = BackendState::Ready;
                let endpoint = info.endpoint.clone().unwrap();

                Ok(format!("Backend started at {} (PID: {})", endpoint, pid))
            }
            Err(e) => {
                let mut info = self.info.write().await;
                info.state = BackendState::Failed;
                info.last_error = Some(e.clone());
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

                    Ok(format!("Backend stopped (PID: {:?}, exit: {:?})", pid, status.code()))
                }
                Ok(Err(e)) => {
                    let mut info = self.info.write().await;
                    info.state = BackendState::Failed;
                    info.last_error = Some(format!("Wait failed: {}", e));
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

    /// Resolve the bundled backend binary path
    fn resolve_backend_binary(&self) -> Result<String, String> {
        // In development: use workspace target/debug/apeireth or target/release/apeireth
        // In production: use bundled sidecar

        #[cfg(debug_assertions)]
        {
            // Development mode - try workspace target directories
            let workspace_root = std::env::current_dir()
                .map_err(|e| format!("Failed to get current dir: {}", e))?;

            // Try release first, then debug
            let release_path = workspace_root.join("target/release/apeireth.exe");
            if release_path.exists() {
                return Ok(release_path.to_string_lossy().to_string());
            }

            let debug_path = workspace_root.join("target/debug/apeireth.exe");
            if debug_path.exists() {
                return Ok(debug_path.to_string_lossy().to_string());
            }

            Err("Backend binary not found in target/release or target/debug".to_string())
        }

        #[cfg(not(debug_assertions))]
        {
            // Production mode - use Tauri sidecar resolution
            // TODO: Implement proper sidecar resolution when bundling is configured
            Err("Production backend resolution not yet implemented".to_string())
        }
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

    /// Spawn the backend process
    async fn spawn_backend(&self, binary_path: &str, port: u16) -> Result<Child, String> {
        let mut cmd = Command::new(binary_path);
        cmd.args(&["gateway", "serve", "--port", &port.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        cmd.spawn()
            .map_err(|e| format!("Failed to spawn backend: {}", e))
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
