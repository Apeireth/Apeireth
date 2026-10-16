//! The MCP server configuration surface.
//!
//! Two entry points, one fail-closed contract:
//!
//! * `APEIRETH_MCP_SERVERS` (a JSON array of [`McpServerSpec`]) is the primary
//!   entry; when it is set it must parse and validate, or loading fails;
//! * the data directory's `mcp-servers.json` is the secondary entry and is
//!   opened through the stored-document single-doc semantics (identity +
//!   envelope self-consistency + version contract): a broken file refuses to
//!   open and is never silently replaced by defaults.
//!
//! Startup logging goes through the redaction helpers here: a server's URL may
//! be recorded, but token / secret parameters never are.

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::{self, DocCompat, StoredDocError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::exec_pipeline::DEFAULT_MAX_TIMEOUT_MS;

/// Environment variable carrying the server list (JSON array).
pub const MCP_SERVERS_ENV: &str = "APEIRETH_MCP_SERVERS";

/// Environment variable releasing server-declared read-only tools (`1` = on).
pub const MCP_READONLY_PRESET_ENV: &str = "APEIRETH_MCP_READONLY_PRESET";

/// File name of the stored-document server list inside the data directory.
pub const MCP_SERVERS_FILE: &str = "mcp-servers.json";

/// Stored-document identity of the server list file.
pub const MCP_SERVERS_DOC_NAME: &str = "mcp-servers";

/// Stored-document format version of the server list file.
pub const MCP_SERVERS_DOC_VERSION: u32 = 1;

/// Default per-call deadline for one MCP tool call, in milliseconds.
pub const DEFAULT_CALL_TIMEOUT_MS: u64 = 30_000;

/// How one server is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransportKind {
    /// A local child process speaking newline-delimited JSON-RPC on its
    /// standard streams.
    Stdio,
    /// An HTTP endpoint carrying server-sent events plus message posts.
    Sse,
    /// An HTTP endpoint answering JSON-RPC requests directly.
    Http,
}

impl McpTransportKind {
    /// Stable label used in configuration logs and errors.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stdio => "stdio",
            Self::Sse => "sse",
            Self::Http => "http",
        }
    }
}

/// One configured external server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerSpec {
    /// Stable server name; it namespaces every tool the server exposes.
    pub name: String,
    /// How the server is reached.
    pub transport: McpTransportKind,
    /// Executable for `stdio` servers (required there, rejected elsewhere).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Arguments for `stdio` servers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Endpoint URL for `sse` / `http` servers (required there, rejected
    /// for `stdio`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Whether this server is wired at all. Explicit, never defaulted.
    pub enabled: bool,
}

impl McpServerSpec {
    /// A `stdio` server spec.
    pub fn stdio(name: impl Into<String>, command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: McpTransportKind::Stdio,
            command: Some(command.into()),
            args: Vec::new(),
            url: None,
            enabled: true,
        }
    }

    /// An `http` / `sse` server spec.
    pub fn remote(
        name: impl Into<String>,
        transport: McpTransportKind,
        url: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            transport,
            command: None,
            args: Vec::new(),
            url: Some(url.into()),
            enabled: true,
        }
    }

    /// Append one command argument (builder style).
    #[must_use]
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Disable this server (builder style).
    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Structural validation: identity grammar, transport/target agreement.
    pub fn validate(&self) -> Result<(), McpConfigError> {
        let name = if self.name.is_empty() {
            "<unnamed>".to_string()
        } else {
            self.name.clone()
        };
        if !is_valid_server_name(&self.name) {
            return Err(McpConfigError::InvalidSpec {
                name,
                reason: "server name must match [a-z0-9][a-z0-9_-]*".to_string(),
            });
        }
        match self.transport {
            McpTransportKind::Stdio => {
                let empty = self.command.as_deref().unwrap_or("").is_empty();
                if empty {
                    return Err(McpConfigError::InvalidSpec {
                        name,
                        reason: "stdio transport requires command".to_string(),
                    });
                }
                if self.url.is_some() {
                    return Err(McpConfigError::InvalidSpec {
                        name,
                        reason: "stdio transport must not carry url".to_string(),
                    });
                }
            }
            McpTransportKind::Sse | McpTransportKind::Http => {
                if self.command.is_some() {
                    return Err(McpConfigError::InvalidSpec {
                        name,
                        reason: "remote transport must not carry command".to_string(),
                    });
                }
                let url = self.url.as_deref().unwrap_or("");
                if url.is_empty() {
                    return Err(McpConfigError::InvalidSpec {
                        name,
                        reason: format!("{} transport requires url", self.transport.as_str()),
                    });
                }
                if !is_http_url(url) {
                    return Err(McpConfigError::InvalidSpec {
                        name,
                        reason: "url must be an http(s) URL".to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// The redacted target of this server: URL with secret parameters
    /// removed, or the command with secret argument values removed.
    pub fn redacted_target(&self) -> String {
        match self.transport {
            McpTransportKind::Stdio => {
                let command = self.command.as_deref().unwrap_or("");
                let args = redact_args(&self.args);
                if args.is_empty() {
                    command.to_string()
                } else {
                    format!("{command} {}", args.join(" "))
                }
            }
            McpTransportKind::Sse | McpTransportKind::Http => {
                redact_url(self.url.as_deref().unwrap_or(""))
            }
        }
    }

    /// One redacted startup log line for this server.
    pub fn redacted_summary(&self) -> String {
        format!(
            "mcp server \"{}\": transport={} target={} enabled={}",
            self.name,
            self.transport.as_str(),
            self.redacted_target(),
            self.enabled
        )
    }
}

/// The configured server list.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Every configured server, in configuration order.
    pub servers: Vec<McpServerSpec>,
}

impl McpServerConfig {
    /// A configuration over an explicit list.
    pub fn new(servers: Vec<McpServerSpec>) -> Self {
        Self { servers }
    }

    /// Parse either the JSON-array env shape or the `{"servers": [...]}` file
    /// body shape, then validate. Fail-closed: any defect is an error.
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, McpConfigError> {
        let config: Self = match value {
            serde_json::Value::Array(items) => Self {
                servers: serde_json::from_value(serde_json::Value::Array(items)).map_err(|e| {
                    McpConfigError::InvalidJson {
                        reason: e.to_string(),
                    }
                })?,
            },
            serde_json::Value::Object(_) => {
                serde_json::from_value(value).map_err(|e| McpConfigError::InvalidJson {
                    reason: e.to_string(),
                })?
            }
            other => {
                return Err(McpConfigError::InvalidJson {
                    reason: format!("expected a JSON array or object, found {other}"),
                })
            }
        };
        config.validate()?;
        Ok(config)
    }

    /// Parse the `APEIRETH_MCP_SERVERS` raw value. Fail-closed.
    pub fn from_env_value(raw: &str) -> Result<Self, McpConfigError> {
        let value: serde_json::Value =
            serde_json::from_str(raw).map_err(|e| McpConfigError::InvalidJson {
                reason: format!("{MCP_SERVERS_ENV}: {e}"),
            })?;
        Self::from_json_value(value)
    }

    /// Validate every spec and reject duplicate server names.
    pub fn validate(&self) -> Result<(), McpConfigError> {
        let mut seen = std::collections::BTreeSet::new();
        for spec in &self.servers {
            spec.validate()?;
            if !seen.insert(spec.name.clone()) {
                return Err(McpConfigError::DuplicateServer {
                    name: spec.name.clone(),
                });
            }
        }
        Ok(())
    }

    /// Read the primary entry. `Ok(None)` when the variable is unset or
    /// empty; a value that does not parse is an error, never a fallback.
    pub fn load_from_env() -> Result<Option<Self>, McpConfigError> {
        match std::env::var(MCP_SERVERS_ENV) {
            Ok(raw) if raw.trim().is_empty() => Ok(None),
            Ok(raw) => Self::from_env_value(&raw).map(Some),
            Err(_) => Ok(None),
        }
    }

    /// Read the secondary entry from the data directory. `Ok(None)` when the
    /// file is absent; a file that exists but refuses to open is an error.
    pub fn load_from_data_dir(data_dir: &Path) -> Result<Option<Self>, McpConfigError> {
        let path = server_list_path(data_dir);
        if !path.exists() {
            return Ok(None);
        }
        let doc = stored_doc::open_single::<Self>(&path, &doc_compat())
            .map_err(McpConfigError::Stored)?;
        doc.body.validate()?;
        Ok(Some(doc.body))
    }

    /// The primary entry wins; the data directory file is the fallback; no
    /// entry at all is an empty (inert) configuration.
    pub fn load(data_dir: Option<&Path>) -> Result<Self, McpConfigError> {
        if let Some(config) = Self::load_from_env()? {
            return Ok(config);
        }
        if let Some(dir) = data_dir {
            if let Some(config) = Self::load_from_data_dir(dir)? {
                return Ok(config);
            }
        }
        Ok(Self::default())
    }

    /// The enabled servers, in configuration order.
    pub fn enabled_servers(&self) -> impl Iterator<Item = &McpServerSpec> {
        self.servers.iter().filter(|spec| spec.enabled)
    }

    /// Redacted startup log: one line per configured server. URLs may be
    /// recorded; token / secret parameters are redacted before they ever
    /// reach a log sink.
    pub fn redacted_startup_log(&self) -> Vec<String> {
        self.servers
            .iter()
            .map(McpServerSpec::redacted_summary)
            .collect()
    }
}

/// The data directory path of the stored server list.
pub fn server_list_path(data_dir: &Path) -> PathBuf {
    data_dir.join(MCP_SERVERS_FILE)
}

/// The stored-document read contract of the server list file.
pub fn doc_compat() -> DocCompat {
    DocCompat::exact(MCP_SERVERS_DOC_NAME, MCP_SERVERS_DOC_VERSION)
}

/// Options steering the bridge (risk preset, deadlines, reconnect budget).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpBridgeOptions {
    /// Release server-declared read-only tools without explicit grants.
    pub readonly_preset: bool,
    /// Per-call deadline for one MCP tool call, in milliseconds.
    pub call_timeout_ms: u64,
    /// Upper bound the timeout gate accepts.
    pub max_timeout_ms: u64,
    /// Disconnect/reconnect budget shared by every connection.
    pub reconnect: apeireth_plugin::mcp::ReconnectPolicy,
}

impl Default for McpBridgeOptions {
    fn default() -> Self {
        Self {
            readonly_preset: false,
            call_timeout_ms: DEFAULT_CALL_TIMEOUT_MS,
            max_timeout_ms: DEFAULT_MAX_TIMEOUT_MS,
            reconnect: apeireth_plugin::mcp::ReconnectPolicy::default(),
        }
    }
}

impl McpBridgeOptions {
    /// Options with the read-only preset taken from the environment knob.
    pub fn from_env() -> Self {
        let preset = std::env::var(MCP_READONLY_PRESET_ENV)
            .map(|raw| readonly_preset_value(&raw))
            .unwrap_or(false);
        Self {
            readonly_preset: preset,
            ..Self::default()
        }
    }

    /// Options with an explicit read-only preset switch.
    pub fn with_readonly_preset(mut self, on: bool) -> Self {
        self.readonly_preset = on;
        self
    }

    /// Options with an explicit per-call deadline.
    pub fn with_call_timeout(mut self, call_timeout_ms: u64, max_timeout_ms: u64) -> Self {
        self.call_timeout_ms = call_timeout_ms;
        self.max_timeout_ms = max_timeout_ms;
        self
    }

    /// Options with an explicit reconnect budget.
    pub fn with_reconnect(mut self, reconnect: apeireth_plugin::mcp::ReconnectPolicy) -> Self {
        self.reconnect = reconnect;
        self
    }
}

/// Parse the read-only preset knob: exactly `1` turns it on.
pub fn readonly_preset_value(raw: &str) -> bool {
    raw.trim() == "1"
}

/// A defect in the MCP server configuration. Every variant is fatal for
/// loading: a broken configuration refuses to open.
#[derive(Debug, Error)]
pub enum McpConfigError {
    /// The raw JSON could not be parsed or did not match the shape.
    #[error("mcp server configuration is not valid JSON: {reason}")]
    InvalidJson {
        /// What the parser refused.
        reason: String,
    },
    /// One server entry is structurally invalid.
    #[error("mcp server {name:?} is invalid: {reason}")]
    InvalidSpec {
        /// The entry's name (or `<unnamed>`).
        name: String,
        /// What validation refused.
        reason: String,
    },
    /// Two entries share one server name.
    #[error("mcp server {name:?} is configured twice")]
    DuplicateServer {
        /// The duplicated name.
        name: String,
    },
    /// The stored-document open refused (bad JSON, wrong identity, or a
    /// version outside the readable set).
    #[error("mcp server configuration file refused to open: {0}")]
    Stored(#[from] StoredDocError),
}

/// Grammar of a server name: `[a-z0-9][a-z0-9_-]*`.
///
/// The name becomes a segment of every dynamic capability id
/// (`tool.mcp.<server>.<tool>`), so it must satisfy the stable-id grammar.
pub fn is_valid_server_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_lowercase() || first.is_ascii_digit()) {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Whether `raw` is an absolute http(s) URL.
pub fn is_http_url(raw: &str) -> bool {
    match url::Url::parse(raw) {
        Ok(url) => matches!(url.scheme(), "http" | "https") && url.host_str().is_some(),
        Err(_) => false,
    }
}

/// Redact a URL for logging: keep scheme/host/path, drop userinfo secrets
/// and the values of secret-looking query parameters.
pub fn redact_url(raw: &str) -> String {
    let Ok(mut url) = url::Url::parse(raw) else {
        return "<unparseable-url>".to_string();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    if let Some(query) = url.query().map(str::to_string) {
        let redacted: Vec<String> = query
            .split('&')
            .map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next().unwrap_or("");
                let value = parts.next();
                match value {
                    Some(_) if is_secret_key(key) => format!("{key}=[redacted]"),
                    Some(value) => format!("{key}={value}"),
                    None => pair.to_string(),
                }
            })
            .collect();
        url.set_query(Some(&redacted.join("&")));
    }
    url.to_string()
}

/// Redact command arguments for logging: values of secret-looking flags and
/// the argument following a secret-looking flag are replaced.
pub fn redact_args(args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            out.push("[redacted]".to_string());
            redact_next = false;
            continue;
        }
        if let Some((key, _value)) = arg.split_once('=') {
            if is_secret_key(key) {
                out.push(format!("{key}=[redacted]"));
                continue;
            }
        }
        if is_secret_key(arg) {
            out.push(arg.clone());
            redact_next = true;
            continue;
        }
        out.push(arg.clone());
    }
    out
}

/// Whether a key / flag name looks like it carries a token or secret.
pub fn is_secret_key(key: &str) -> bool {
    let lowered = key.trim_start_matches('-').to_ascii_lowercase();
    const SECRET_FRAGMENTS: &[&str] = &[
        "token",
        "secret",
        "key",
        "password",
        "passwd",
        "pwd",
        "auth",
        "credential",
        "signature",
        "bearer",
    ];
    SECRET_FRAGMENTS
        .iter()
        .any(|fragment| lowered.contains(fragment))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("apeireth-mcp-config-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    #[test]
    fn env_shape_parses_and_validates() {
        let raw = r#"[{"name":"demo","transport":"stdio","command":"demo-server","args":["--port","7"],"enabled":true}]"#;
        let config = McpServerConfig::from_env_value(raw).unwrap();
        assert_eq!(config.servers.len(), 1);
        assert_eq!(config.servers[0].name, "demo");
        assert_eq!(config.servers[0].transport, McpTransportKind::Stdio);
        assert!(config.servers[0].enabled);
    }

    #[test]
    fn bad_json_and_bad_specs_fail_closed() {
        assert!(McpServerConfig::from_env_value("not json").is_err());
        // stdio without command
        let missing = r#"[{"name":"demo","transport":"stdio","enabled":true}]"#;
        assert!(McpServerConfig::from_env_value(missing).is_err());
        // remote without url
        let no_url = r#"[{"name":"demo","transport":"http","enabled":true}]"#;
        assert!(McpServerConfig::from_env_value(no_url).is_err());
        // unknown transport
        let bad_kind = r#"[{"name":"demo","transport":"ftp","url":"http://x","enabled":true}]"#;
        assert!(McpServerConfig::from_env_value(bad_kind).is_err());
        // duplicate server names
        let dup = r#"[{"name":"demo","transport":"stdio","command":"a","enabled":true},
                       {"name":"demo","transport":"stdio","command":"b","enabled":true}]"#;
        assert!(McpServerConfig::from_env_value(dup).is_err());
    }

    #[test]
    fn stored_doc_file_is_loaded_and_broken_file_refuses_to_open() {
        let dir = temp_dir("stored");
        let path = server_list_path(&dir);
        let body = McpServerConfig::new(vec![McpServerSpec::remote(
            "demo",
            McpTransportKind::Http,
            "http://127.0.0.1:9/mcp",
        )]);
        stored_doc::save_single(&path, &doc_compat(), body, stored_doc::DEFAULT_DOC_MODE).unwrap();
        let loaded = McpServerConfig::load_from_data_dir(&dir)
            .unwrap()
            .expect("file present");
        assert_eq!(loaded.servers[0].name, "demo");

        // A broken envelope body refuses to open; nothing is defaulted.
        std::fs::write(&path, "{ not json").unwrap();
        assert!(McpServerConfig::load_from_data_dir(&dir).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_file_reads_as_absent_not_as_default_servers() {
        let dir = temp_dir("absent");
        assert!(McpServerConfig::load_from_data_dir(&dir).unwrap().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn startup_log_redacts_token_parameters_but_keeps_urls() {
        let spec = McpServerSpec::remote(
            "demo",
            McpTransportKind::Http,
            "https://user:pass@example.test/mcp?token=abc123&region=eu",
        );
        let line = spec.redacted_summary();
        assert!(line.contains("example.test"), "url stays: {line}");
        assert!(
            !line.contains("abc123"),
            "token value must not leak: {line}"
        );
        assert!(!line.contains("pass"), "url password must not leak: {line}");
        assert!(line.contains("region=eu"), "harmless params stay: {line}");

        let stdio = McpServerSpec::stdio("local", "server-bin")
            .with_arg("--api-key")
            .with_arg("super-secret")
            .with_arg("--verbose");
        let line = stdio.redacted_summary();
        assert!(
            !line.contains("super-secret"),
            "flag value must not leak: {line}"
        );
        assert!(line.contains("--verbose"), "harmless args stay: {line}");
    }

    #[test]
    fn readonly_preset_knob_only_accepts_one() {
        assert!(readonly_preset_value("1"));
        assert!(!readonly_preset_value("0"));
        assert!(!readonly_preset_value("true"));
        assert!(!readonly_preset_value(""));
    }
}
