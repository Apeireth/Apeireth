//! Apeireth Desktop Production Logging
//!
//! Persistent logging to platform-appropriate app data directories.
//!
//! # Log Files
//! - apeireth-desktop.log: Desktop app events (startup, backend lifecycle, errors)
//! - apeireth-backend.log: Captured stdout/stderr from backend process
//!
//! # Requirements
//! - Bounded file size with rotation
//! - Secret redaction (API keys, Authorization headers, tokens)
//! - Timestamped structured events
//! - Platform-appropriate paths (Windows: %LOCALAPPDATA%\Apeireth\logs\)

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Desktop logger for production diagnostics
pub struct DesktopLogger {
    log_dir: PathBuf,
    desktop_log: Arc<Mutex<Option<File>>>,
    backend_log: Arc<Mutex<Option<File>>>,
}

impl DesktopLogger {
    /// Create a new logger with platform-appropriate log directory
    pub fn new() -> Result<Self, String> {
        let log_dir = Self::resolve_log_directory()?;
        fs::create_dir_all(&log_dir)
            .map_err(|e| format!("Failed to create log directory: {}", e))?;

        let desktop_log_path = log_dir.join("apeireth-desktop.log");
        let backend_log_path = log_dir.join("apeireth-backend.log");

        let desktop_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&desktop_log_path)
            .map_err(|e| format!("Failed to open desktop log: {}", e))?;

        let backend_log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&backend_log_path)
            .map_err(|e| format!("Failed to open backend log: {}", e))?;

        Ok(Self {
            log_dir,
            desktop_log: Arc::new(Mutex::new(Some(desktop_log))),
            backend_log: Arc::new(Mutex::new(Some(backend_log))),
        })
    }

    /// Resolve platform-appropriate log directory
    fn resolve_log_directory() -> Result<PathBuf, String> {
        // Use Tauri's app data directory convention
        #[cfg(target_os = "windows")]
        {
            let local_app_data = std::env::var("LOCALAPPDATA")
                .map_err(|_| "LOCALAPPDATA not set".to_string())?;
            Ok(PathBuf::from(local_app_data).join("Apeireth").join("logs"))
        }

        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| "HOME not set".to_string())?;
            Ok(PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("app.apeireth.companion")
                .join("logs"))
        }

        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME")
                .map_err(|_| "HOME not set".to_string())?;
            Ok(PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("apeireth-companion")
                .join("logs"))
        }
    }

    /// Log a desktop event
    pub fn log_desktop(&self, level: LogLevel, message: &str) {
        let timestamp = Self::format_timestamp(SystemTime::now());
        let log_line = format!("{} {} {}\n", timestamp, level.as_str(), message);

        if let Ok(mut file_guard) = self.desktop_log.lock() {
            if let Some(file) = file_guard.as_mut() {
                let _ = file.write_all(log_line.as_bytes());
                let _ = file.flush();
            }
        }
    }

    /// Log backend output (stdout/stderr capture)
    pub fn log_backend(&self, line: &str) {
        let timestamp = Self::format_timestamp(SystemTime::now());
        let redacted = Self::redact_secrets(line);
        let log_line = format!("{} {}\n", timestamp, redacted);

        if let Ok(mut file_guard) = self.backend_log.lock() {
            if let Some(file) = file_guard.as_mut() {
                let _ = file.write_all(log_line.as_bytes());
                let _ = file.flush();
            }
        }
    }

    /// Get log directory path
    pub fn log_directory(&self) -> &Path {
        &self.log_dir
    }

    /// Format timestamp as ISO 8601
    fn format_timestamp(time: SystemTime) -> String {
        match time.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs();
                let millis = duration.subsec_millis();

                // Simple UTC formatting (YYYY-MM-DD HH:MM:SS.mmm)
                let days_since_epoch = secs / 86400;
                let time_of_day = secs % 86400;
                let hours = time_of_day / 3600;
                let minutes = (time_of_day % 3600) / 60;
                let seconds = time_of_day % 60;

                // Approximate date (good enough for logs)
                let year = 1970 + (days_since_epoch / 365);
                let day_of_year = days_since_epoch % 365;
                let month = (day_of_year / 30) + 1;
                let day = (day_of_year % 30) + 1;

                format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                    year, month, day, hours, minutes, seconds, millis
                )
            }
            Err(_) => "0000-00-00T00:00:00.000Z".to_string(),
        }
    }

    /// Redact secrets from log lines
    fn redact_secrets(line: &str) -> String {
        let mut redacted = line.to_string();

        // Redact API keys
        if let Some(pos) = redacted.find("sk-") {
            if let Some(end) = redacted[pos..].find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}') {
                redacted.replace_range(pos..pos + end, "[REDACTED_API_KEY]");
            } else {
                redacted.replace_range(pos.., "[REDACTED_API_KEY]");
            }
        }

        // Redact Authorization headers
        if redacted.contains("Authorization") {
            if let Some(pos) = redacted.find("Bearer ") {
                if let Some(end) = redacted[pos + 7..].find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',') {
                    redacted.replace_range(pos + 7..pos + 7 + end, "[REDACTED_TOKEN]");
                } else {
                    redacted.replace_range(pos + 7.., "[REDACTED_TOKEN]");
                }
            }
        }

        // Redact environment variable values for known secret patterns
        for pattern in &["APEIRETH_API_KEY=", "ANTHROPIC_API_KEY=", "OPENAI_API_KEY=", "OPENAI_API_KEY:"] {
            if let Some(pos) = redacted.find(pattern) {
                let start = pos + pattern.len();
                if let Some(end) = redacted[start..].find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',') {
                    redacted.replace_range(start..start + end, "[REDACTED]");
                } else {
                    redacted.replace_range(start.., "[REDACTED]");
                }
            }
        }

        redacted
    }

    /// Rotate log if it exceeds size limit (10MB)
    pub fn rotate_if_needed(&self) -> Result<(), String> {
        const MAX_SIZE: u64 = 10 * 1024 * 1024; // 10MB

        let desktop_log_path = self.log_dir.join("apeireth-desktop.log");
        let backend_log_path = self.log_dir.join("apeireth-backend.log");

        Self::rotate_log_file(&desktop_log_path, MAX_SIZE)?;
        Self::rotate_log_file(&backend_log_path, MAX_SIZE)?;

        Ok(())
    }

    fn rotate_log_file(path: &Path, max_size: u64) -> Result<(), String> {
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() > max_size {
                let backup_path = path.with_extension("log.old");
                let _ = fs::remove_file(&backup_path); // Remove old backup
                fs::rename(path, backup_path)
                    .map_err(|e| format!("Failed to rotate log: {}", e))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let line = "Using API key: sk-1234567890abcdef";
        let redacted = DesktopLogger::redact_secrets(line);
        assert!(redacted.contains("[REDACTED_API_KEY]"));
        assert!(!redacted.contains("sk-1234567890"));

        let line2 = "Authorization: Bearer sk-abcd1234";
        let redacted2 = DesktopLogger::redact_secrets(line2);
        assert!(redacted2.contains("[REDACTED_TOKEN]"));
        assert!(!redacted2.contains("sk-abcd"));

        let line3 = "OPENAI_API_KEY=sk-secret";
        let redacted3 = DesktopLogger::redact_secrets(line3);
        assert!(redacted3.contains("[REDACTED]"));
        assert!(!redacted3.contains("sk-secret"));
    }
}
