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

// Workspace policy (clippy.toml): fs_err over std::fs, so filesystem errors
// carry the offending path instead of a bare ErrorKind.
use fs_err::{self as fs, File, OpenOptions};
use std::io::Write;
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

    /// Redact secrets from log lines.
    ///
    /// Three ordered passes:
    /// 1. named assignments (`NAME=value`, `"NAME":"value"`) whose name looks
    ///    credential-bearing — the env/config case, labelled `[REDACTED]`
    /// 2. `Bearer <token>` values — labelled `[REDACTED_TOKEN]`
    /// 3. bare provider keys by prefix (`sk-…`) — labelled `[REDACTED_API_KEY]`
    ///
    /// Named assignments run first so `OPENAI_API_KEY=sk-…` is redacted as a
    /// configured credential rather than as a loose key fragment.
    fn redact_secrets(line: &str) -> String {
        let stage = Self::redact_named_assignments(line);
        let stage = Self::redact_bearer_tokens(&stage);
        Self::redact_key_prefixes(&stage)
    }

    /// Characters that terminate a secret value in log text.
    fn is_value_terminator(c: char) -> bool {
        c.is_whitespace() || matches!(c, '"' | '\'' | ',' | '}' | ']' | ')' | ';' | '&')
    }

    /// Whether a name carries a credential.
    ///
    /// Substring markers rather than an exact list: this is what the earlier
    /// hardcoded `ANTHROPIC_API_KEY=` list got wrong — the canonical variable
    /// is `APEIRETH_ANTHROPIC_KEY`, so it leaked. Any future
    /// `APEIRETH_<VENDOR>_KEY` is now covered without editing this file.
    ///
    /// The name must also look like a field identifier (underscore, or a short
    /// all-caps word) so prose such as "API key: …" is not treated as an
    /// assignment and keeps its more specific label from pass 3.
    fn is_sensitive_key_name(name: &str) -> bool {
        const MARKERS: [&str; 5] = ["KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL"];
        let upper = name.to_ascii_uppercase();
        let field_shaped = name.contains('_')
            || (name.len() >= 4
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
        field_shaped && MARKERS.iter().any(|marker| upper.contains(marker))
    }

    /// Names matching [`Self::is_sensitive_key_name`] that hold no secret.
    ///
    /// `credential_source=environment` and the keyring selector are the
    /// diagnostics that make a provider misconfiguration explainable, so they
    /// are explicitly allowed to survive redaction.
    fn is_safe_key_name(name: &str) -> bool {
        const SAFE: [&str; 4] = [
            "CREDENTIAL_SOURCE",
            "CREDENTIAL_SOURCES",
            "APEIRETH_KEYRING_BACKEND",
            "APEIRETH_KEYRING_DIR",
        ];
        SAFE.contains(&name.to_ascii_uppercase().as_str())
    }

    /// Scan back from a separator to read the assignment's name.
    ///
    /// Handles both `NAME=value` and `"NAME": "value"` by skipping the quote
    /// and whitespace that JSON puts between the name and the separator.
    fn assignment_name(haystack: &str, separator_index: usize) -> &str {
        let head = &haystack[..separator_index];
        let head = head.trim_end();
        let head = head.trim_end_matches(['"', '\'']);
        let start = head
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.'))
            .map_or(0, |index| index + 1);
        &head[start..]
    }

    /// Pass 1 — redact `NAME=value` / `"NAME":"value"` for credential names.
    fn redact_named_assignments(line: &str) -> String {
        let mut out = String::with_capacity(line.len());
        let mut rest = line;

        while let Some(separator) = rest.find(['=', ':']) {
            let name = Self::assignment_name(rest, separator);
            let value_start = separator + 1;
            let value_offset = rest[value_start..]
                .find(|c: char| !c.is_whitespace() && c != '"' && c != '\'')
                .map(|offset| value_start + offset);

            let redact_here = !name.is_empty()
                && Self::is_sensitive_key_name(name)
                && !Self::is_safe_key_name(name)
                && value_offset.is_some();

            match (redact_here, value_offset) {
                (true, Some(value_start)) => {
                    let value_end = rest[value_start..]
                        .find(Self::is_value_terminator)
                        .map_or(rest.len(), |offset| value_start + offset);
                    out.push_str(&rest[..value_start]);
                    out.push_str("[REDACTED]");
                    rest = &rest[value_end..];
                }
                _ => {
                    // Not a credential assignment: keep the separator and move on.
                    out.push_str(&rest[..value_start]);
                    rest = &rest[value_start..];
                }
            }
        }

        out.push_str(rest);
        out
    }

    /// Pass 2 — redact the token following `Bearer `.
    fn redact_bearer_tokens(line: &str) -> String {
        const BEARER: &str = "Bearer ";
        let mut out = String::with_capacity(line.len());
        let mut rest = line;

        while let Some(index) = rest.find(BEARER) {
            let value_start = index + BEARER.len();
            out.push_str(&rest[..value_start]);
            let value_end = rest[value_start..]
                .find(Self::is_value_terminator)
                .map_or(rest.len(), |offset| value_start + offset);
            if value_end > value_start {
                out.push_str("[REDACTED_TOKEN]");
            }
            rest = &rest[value_end..];
        }

        out.push_str(rest);
        out
    }

    /// Pass 3 — redact bare provider keys identified by prefix.
    fn redact_key_prefixes(line: &str) -> String {
        const PREFIXES: [&str; 3] = ["sk-", "sk_live_", "ant-"];
        let mut out = line.to_string();

        for prefix in PREFIXES {
            let mut search_from = 0;
            while let Some(offset) = out[search_from..].find(prefix) {
                let start = search_from + offset;
                // Already-redacted regions contain no prefix, so any hit is live.
                let end = out[start..]
                    .find(Self::is_value_terminator)
                    .map_or(out.len(), |offset| start + offset);
                out.replace_range(start..end, "[REDACTED_API_KEY]");
                search_from = start + "[REDACTED_API_KEY]".len();
                if search_from >= out.len() {
                    break;
                }
            }
        }

        out
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

    /// The canonical credential variables proved by the runtime audit:
    /// `APEIRETH_API_KEY` (minimax), `APEIRETH_ANTHROPIC_KEY` (anthropic),
    /// `OPENAI_API_KEY` (openai-compatible).
    #[test]
    fn redacts_canonical_provider_env_values() {
        for (line, secret) in [
            ("APEIRETH_API_KEY=test-key-mm-live-abcdef123456", "test-key-mm-live-abcdef123456"),
            ("APEIRETH_ANTHROPIC_KEY=test-key-ant-live-9f8e7d", "test-key-ant-live-9f8e7d"),
            ("OPENAI_API_KEY=test-key-oai-live-55443322", "test-key-oai-live-55443322"),
        ] {
            let redacted = DesktopLogger::redact_secrets(line);
            assert!(
                !redacted.contains(secret),
                "secret leaked for {line:?}: {redacted}"
            );
            assert!(
                redacted.contains("[REDACTED]"),
                "no redaction marker for {line:?}: {redacted}"
            );
        }
    }

    #[test]
    fn redacts_master_token_bearer_header() {
        let line = r#"{"headers":{"Authorization":"Bearer master-tok-7788"},"path":"/v1/chat"}"#;
        let redacted = DesktopLogger::redact_secrets(line);
        assert!(!redacted.contains("master-tok-7788"), "master token leaked: {redacted}");
        assert!(redacted.contains("[REDACTED_TOKEN]"));
        // Surrounding structure must survive so the log stays useful.
        assert!(redacted.contains("/v1/chat"));
    }

    #[test]
    fn keeps_non_secret_diagnostics_intact() {
        let line = "backend.ready pid=4321 port=52719 credential_source=environment";
        let redacted = DesktopLogger::redact_secrets(line);
        assert_eq!(redacted, line, "non-secret diagnostics must not be altered");
    }

    #[test]
    fn rotation_moves_oversized_log_aside() {
        let dir = std::env::temp_dir().join(format!("apeireth-rot-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp log dir");
        let log_path = dir.join("apeireth-desktop.log");
        fs::write(&log_path, vec![b'x'; 2048]).expect("seed log");

        DesktopLogger::rotate_log_file(&log_path, 1024).expect("rotate");

        assert!(
            !log_path.exists(),
            "oversized log should have been moved aside"
        );
        assert!(
            log_path.with_extension("log.old").exists(),
            "rotated backup should exist"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotation_leaves_small_log_in_place() {
        let dir = std::env::temp_dir().join(format!("apeireth-norot-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp log dir");
        let log_path = dir.join("apeireth-desktop.log");
        fs::write(&log_path, b"small").expect("seed log");

        DesktopLogger::rotate_log_file(&log_path, 1024).expect("rotate");

        assert!(log_path.exists(), "small log must stay in place");
        assert!(!log_path.with_extension("log.old").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
