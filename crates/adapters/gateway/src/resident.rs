//! Resident-slot (daemon) bookkeeping for `apeireth gateway serve`.
//!
//! A resident gateway (the always-on service form, see
//! `docs/02-guides/server-deployment.md`) claims a small **slot file**
//! ("PID 文件") before it starts serving and releases it on shutdown. The slot
//! file is the machine-level "one resident gateway" claim:
//!
//! - it records `pid` / `port` / `bind` / `started_at_ms` as `key=value` lines
//!   so every consumer (the service scripts, the desktop shell probe) can read
//!   it with the same trivial parser;
//! - claiming an already-claimed slot whose recorded port still answers a TCP
//!   connect is refused with an explicit "already running" error — a second
//!   resident gateway is never started (不双开);
//! - a stale slot (recorded port no longer answers) or a corrupt slot file is
//!   taken over silently but honestly: the new record replaces it, and the old
//!   content is gone because the instance it described is gone.
//!
//! Liveness is judged by **port reachability**, not by process-table lookup:
//! the recorded `pid` is diagnostic metadata, while "the gateway port still
//! answers" is the behavior every client actually observes. This keeps the
//! check dependency-free and identical on every platform.
//!
//! The slot file path is always explicit (`--pid-file` on the serve command or
//! a caller-supplied path) — the gateway never invents one relative to the
//! process CWD (工作目录无关).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One record in a resident slot file.
///
/// Rendered/parsed as `key=value` lines (one per line). Unknown keys are
/// ignored on parse (forward compatibility); missing required keys are an
/// error rather than a guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidentRecord {
    /// Process id of the gateway holding the slot (diagnostic metadata).
    pub pid: u32,
    /// TCP port the gateway serves on (the liveness probe target).
    pub port: u16,
    /// Bind address the gateway serves on (diagnostic metadata).
    pub bind: String,
    /// Epoch milliseconds when the gateway claimed the slot.
    pub started_at_ms: i64,
}

impl ResidentRecord {
    /// Record for the current process.
    pub fn for_this_process(bind: &str, port: u16, started_at_ms: i64) -> Self {
        Self {
            pid: std::process::id(),
            port,
            bind: bind.to_string(),
            started_at_ms,
        }
    }

    /// Render as `key=value` lines (stable key order).
    pub fn render(&self) -> String {
        format!(
            "pid={}\nport={}\nbind={}\nstarted_at_ms={}\n",
            self.pid, self.port, self.bind, self.started_at_ms
        )
    }

    /// Parse `key=value` lines. Blank lines and `#` comments are skipped.
    ///
    /// Fails loudly on a missing/invalid required key: a slot file that cannot
    /// be understood must not be treated as a healthy claim.
    pub fn parse(text: &str) -> Result<Self, ResidentParseError> {
        let mut pid: Option<u32> = None;
        let mut port: Option<u16> = None;
        let mut bind: Option<String> = None;
        let mut started_at_ms: Option<i64> = None;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(ResidentParseError::MalformedLine(line.to_string()));
            };
            match key.trim() {
                "pid" => {
                    pid =
                        Some(value.trim().parse().map_err(|_| {
                            ResidentParseError::InvalidValue("pid", value.to_string())
                        })?);
                }
                "port" => {
                    port = Some(value.trim().parse().map_err(|_| {
                        ResidentParseError::InvalidValue("port", value.to_string())
                    })?);
                }
                "bind" => {
                    bind = Some(value.trim().to_string());
                }
                "started_at_ms" => {
                    started_at_ms = Some(value.trim().parse().map_err(|_| {
                        ResidentParseError::InvalidValue("started_at_ms", value.to_string())
                    })?);
                }
                // Unknown keys are forward-compatible metadata.
                _ => {}
            }
        }
        Ok(Self {
            pid: pid.ok_or(ResidentParseError::MissingField("pid"))?,
            port: port.ok_or(ResidentParseError::MissingField("port"))?,
            bind: bind.ok_or(ResidentParseError::MissingField("bind"))?,
            started_at_ms: started_at_ms
                .ok_or(ResidentParseError::MissingField("started_at_ms"))?,
        })
    }

    /// Whether the recorded gateway still answers: a TCP connect to
    /// `127.0.0.1:{port}` succeeds within a short budget. This is the same
    /// "port/health" observation every client makes of a resident gateway.
    pub async fn is_live(&self) -> bool {
        let address = format!("127.0.0.1:{}", self.port);
        matches!(
            tokio::time::timeout(
                std::time::Duration::from_millis(500),
                tokio::net::TcpStream::connect(&address)
            )
            .await,
            Ok(Ok(_))
        )
    }
}

/// Slot-file parse failure (honest error, never a silent default).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResidentParseError {
    #[error("resident slot line is not `key=value`: {0}")]
    MalformedLine(String),
    #[error("resident slot value for `{0}` is invalid: {1}")]
    InvalidValue(&'static str, String),
    #[error("resident slot is missing required field `{0}`")]
    MissingField(&'static str),
    #[error("resident slot file could not be read: {0}")]
    Unreadable(String),
}

/// Why a slot claim was refused.
#[derive(Debug, thiserror::Error)]
pub enum ResidentError {
    /// A live gateway already holds the slot (or the port is already served):
    /// the explicit "already running" refusal (二次启动不双开).
    #[error(
        "gateway already running (pid {pid}, port {port}); refusing to start a second instance"
    )]
    AlreadyRunning { pid: u32, port: u16 },
    /// The slot file could not be read or written.
    #[error("resident slot file {path} failed: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Held while this process owns a resident slot file.
///
/// Dropping the lease removes the slot file, but only when its content still
/// describes this process — a takeover by a newer claimant is never undone.
pub struct ResidentLease {
    path: PathBuf,
    pid: u32,
}

impl ResidentLease {
    /// Path of the slot file this lease owns.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ResidentLease {
    fn drop(&mut self) {
        // Release is best-effort but ownership-checked: only delete the slot
        // when it still records our own pid (someone else may have taken over
        // a stale slot meanwhile).
        let owned = fs_err::read_to_string(&self.path)
            .ok()
            .and_then(|text| ResidentRecord::parse(&text).ok())
            .map(|record| record.pid == self.pid)
            .unwrap_or(false);
        if owned {
            let _ = fs_err::remove_file(&self.path);
        }
    }
}

/// Read a slot file. A missing file is `Ok(None)`; unreadable content is an
/// error (the caller decides whether corrupt content means "stale").
pub fn read_slot(path: &Path) -> Result<Option<ResidentRecord>, ResidentParseError> {
    let text = match fs_err::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ResidentParseError::Unreadable(format!(
                "{}: {error}",
                path.display()
            )));
        }
    };
    ResidentRecord::parse(&text).map(Some)
}

/// Claim a resident slot file for this process.
///
/// Refuses with [`ResidentError::AlreadyRunning`] when the slot describes a
/// gateway whose port still answers (a live second start). A stale or corrupt
/// slot (its port does not answer / its content is unreadable) is taken over:
/// the new record replaces it.
///
/// Ordering note: the claim happens *before* the listener binds so a refused
/// second start costs nothing, and the TCP bind remains the authoritative
/// mutual exclusion between two claims racing each other.
pub async fn claim_slot(
    path: &Path,
    record: ResidentRecord,
) -> Result<ResidentLease, ResidentError> {
    let existing = read_slot(path);
    match existing {
        Ok(Some(previous)) => {
            if previous.is_live().await {
                return Err(ResidentError::AlreadyRunning {
                    pid: previous.pid,
                    port: previous.port,
                });
            }
        }
        Ok(None) => {}
        // Corrupt slot content: the instance it described cannot be trusted to
        // exist; take over with a fresh record.
        Err(_) => {}
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs_err::create_dir_all(parent).map_err(|source| ResidentError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }
    fs_err::write(path, record.render()).map_err(|source| ResidentError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(ResidentLease {
        path: path.to_path_buf(),
        pid: record.pid,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_and_parse_round_trip() {
        let record = ResidentRecord {
            pid: 4242,
            port: 8080,
            bind: "127.0.0.1".to_string(),
            started_at_ms: 1_760_000_000_000,
        };
        let rendered = record.render();
        assert_eq!(ResidentRecord::parse(&rendered).unwrap(), record);
    }

    #[test]
    fn parse_ignores_comments_blanks_and_unknown_keys() {
        let text =
            "# resident slot\n\npid=7\nport=8080\nfuture_key=x\nbind=127.0.0.1\nstarted_at_ms=1\n";
        let record = ResidentRecord::parse(text).unwrap();
        assert_eq!(record.pid, 7);
        assert_eq!(record.port, 8080);
    }

    #[test]
    fn parse_fails_loudly_on_missing_or_malformed_fields() {
        assert!(matches!(
            ResidentRecord::parse("pid=1\nport=2\nbind=x\n"),
            Err(ResidentParseError::MissingField("started_at_ms"))
        ));
        assert!(matches!(
            ResidentRecord::parse("pid=1\nport=2\nbind=x\nstarted_at_ms=3\nnonsense\n"),
            Err(ResidentParseError::MalformedLine(_))
        ));
        assert!(matches!(
            ResidentRecord::parse("pid=nope\nport=2\nbind=x\nstarted_at_ms=3\n"),
            Err(ResidentParseError::InvalidValue("pid", _))
        ));
    }
}
