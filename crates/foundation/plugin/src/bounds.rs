//! Manifest bound and permission checks recovered from
//! `legacy/donor/apeireth-extension` (`manifest.rs`, `sandbox.rs`, `audit.rs`).
//!
//! The donor parsed `extension.toml` and ran a second plugin execution manager.
//! v2 plugins are static and in-process; this module keeps the **numeric and
//! permission algorithms** so a caller can validate metadata before treating a
//! declaration as callable. It does not register plugins and does not invoke
//! them.

use std::collections::BTreeSet;

use crate::manifest::PluginManifest;
use crate::semver;

/// Donor name length cap (`extension.toml` `name`, 1..=64).
pub const MAX_NAME_LEN: usize = 64;
/// Donor description length cap.
pub const MAX_DESC_LEN: usize = 512;
/// Donor permission-string length cap.
pub const MAX_PERMISSION_LEN: usize = 64;
/// Donor permission-list length cap.
pub const MAX_PERMISSIONS: usize = 32;
/// Donor version-string length cap (the strict parser is tighter).
pub const MAX_VERSION_LEN: usize = 32;
/// Absolute byte ceiling (16 MiB) for declared I/O.
pub const MAX_IO_BYTES: usize = 16 * 1024 * 1024;
/// Audit-layer minimum declared I/O (1 KiB). Schema parse allowed 64.
pub const MIN_AUDITED_IO_BYTES: usize = 1024;
/// Schema-layer minimum declared I/O.
pub const MIN_IO_BYTES: usize = 64;
/// Maximum declared timeout (10 minutes).
pub const MAX_TIMEOUT_MS: u64 = 600_000;

/// Resource bounds a plugin may declare in metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBounds {
    /// Maximum accepted input size.
    pub max_input_bytes: usize,
    /// Maximum produced output size.
    pub max_output_bytes: usize,
    /// Maximum execution time, milliseconds.
    pub timeout_ms: u64,
}

impl ResourceBounds {
    /// Donor-style defaults: 64 KiB in/out, 1 s timeout.
    pub const fn default_limits() -> Self {
        Self {
            max_input_bytes: 65_536,
            max_output_bytes: 65_536,
            timeout_ms: 1_000,
        }
    }
}

impl Default for ResourceBounds {
    fn default() -> Self {
        Self::default_limits()
    }
}

/// Why a bound or permission check failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundError {
    /// A required field is missing or empty.
    Schema(String),
    /// A declared permission is not in the caller's grant.
    PermissionDenied {
        /// Plugin id.
        plugin: String,
        /// Missing permission.
        required: String,
    },
    /// Payload larger than the declared input cap.
    InputTooLarge {
        /// Actual size.
        actual: usize,
        /// Declared cap.
        max: usize,
        /// Plugin id.
        plugin: String,
    },
    /// Payload larger than the declared output cap.
    OutputTooLarge {
        /// Actual size.
        actual: usize,
        /// Declared cap.
        max: usize,
        /// Plugin id.
        plugin: String,
    },
    /// Post-schema audit rejected the declaration.
    AuditRejected(String),
}

impl std::fmt::Display for BoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Schema(msg) => write!(f, "manifest schema: {msg}"),
            Self::PermissionDenied { plugin, required } => {
                write!(f, "permission denied: plugin {plugin} needs {required}")
            }
            Self::InputTooLarge {
                actual,
                max,
                plugin,
            } => write!(f, "input too large: {actual} > {max} (plugin={plugin})"),
            Self::OutputTooLarge {
                actual,
                max,
                plugin,
            } => write!(f, "output too large: {actual} > {max} (plugin={plugin})"),
            Self::AuditRejected(msg) => write!(f, "audit rejected: {msg}"),
        }
    }
}

impl std::error::Error for BoundError {}

/// Skill-style kebab-case id: ASCII lowercase + digit + `-`, no consecutive
/// dashes, no leading or trailing dash. Recovered from
/// `legacy/donor/apeireth-skills` `is_valid_id`. Distinct from core
/// [`apeireth_core::kernel::PluginId`] grammar, which also allows `.` and `_`.
pub fn is_valid_kebab(id: &str) -> bool {
    if id.is_empty() || id.starts_with('-') || id.ends_with('-') {
        return false;
    }
    let mut prev_dash = false;
    for c in id.chars() {
        if c == '-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
            if !(c.is_ascii_lowercase() || c.is_ascii_digit()) {
                return false;
            }
        }
    }
    true
}

/// Validate a plugin name: non-empty, ≤ 64, `[a-z0-9-_]`.
pub fn validate_extension_name(name: &str) -> Result<(), BoundError> {
    if name.is_empty() {
        return Err(BoundError::Schema("name must not be empty".into()));
    }
    if name.len() > MAX_NAME_LEN {
        return Err(BoundError::Schema(format!(
            "name too long: {} > {MAX_NAME_LEN}",
            name.len()
        )));
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid {
        return Err(BoundError::Schema(format!(
            "name must be [a-z0-9-_] only: {name:?}"
        )));
    }
    Ok(())
}

/// Validate a version string: length cap, then strict SemVer 2.0.0.
pub fn validate_version(v: &str) -> Result<(), BoundError> {
    if v.is_empty() || v.len() > MAX_VERSION_LEN {
        return Err(BoundError::Schema(format!(
            "version must be 1..={MAX_VERSION_LEN} chars: {v:?}"
        )));
    }
    semver::parse(v).map(|_| ()).map_err(|e| BoundError::Schema(e.to_string()))
}

/// Validate a permission list: length, per-item length, no duplicates.
pub fn validate_permissions(permissions: &[String]) -> Result<(), BoundError> {
    if permissions.len() > MAX_PERMISSIONS {
        return Err(BoundError::Schema(format!(
            "permissions list too long: {} > {MAX_PERMISSIONS}",
            permissions.len()
        )));
    }
    let mut seen = BTreeSet::new();
    for p in permissions {
        if p.is_empty() || p.len() > MAX_PERMISSION_LEN {
            return Err(BoundError::Schema(format!(
                "permission must be 1..={MAX_PERMISSION_LEN} chars: {p:?}"
            )));
        }
        if !seen.insert(p.as_str()) {
            return Err(BoundError::Schema(format!("duplicate permission: {p}")));
        }
    }
    Ok(())
}

/// Schema-layer numeric bounds (min 64 bytes, max 16 MiB, timeout 1..=10 min).
pub fn validate_resource_bounds(bounds: &ResourceBounds) -> Result<(), BoundError> {
    if bounds.max_input_bytes < MIN_IO_BYTES || bounds.max_input_bytes > MAX_IO_BYTES {
        return Err(BoundError::Schema(format!(
            "max_input_bytes must be {MIN_IO_BYTES}..={MAX_IO_BYTES}"
        )));
    }
    if bounds.max_output_bytes < MIN_IO_BYTES || bounds.max_output_bytes > MAX_IO_BYTES {
        return Err(BoundError::Schema(format!(
            "max_output_bytes must be {MIN_IO_BYTES}..={MAX_IO_BYTES}"
        )));
    }
    if bounds.timeout_ms == 0 || bounds.timeout_ms > MAX_TIMEOUT_MS {
        return Err(BoundError::Schema(format!(
            "timeout_ms must be 1..={MAX_TIMEOUT_MS}"
        )));
    }
    Ok(())
}

/// Donor audit layer: at least one permission, I/O ≥ 1 KiB, timeout ≤ 10 min.
pub fn audit_bounds(permissions: &[String], bounds: &ResourceBounds) -> Result<(), BoundError> {
    if permissions.is_empty() {
        return Err(BoundError::AuditRejected(
            "no permissions declared".into(),
        ));
    }
    if bounds.max_input_bytes < MIN_AUDITED_IO_BYTES {
        return Err(BoundError::AuditRejected(format!(
            "max_input_bytes too small: {} (min {MIN_AUDITED_IO_BYTES})",
            bounds.max_input_bytes
        )));
    }
    if bounds.max_output_bytes < MIN_AUDITED_IO_BYTES {
        return Err(BoundError::AuditRejected(format!(
            "max_output_bytes too small: {} (min {MIN_AUDITED_IO_BYTES})",
            bounds.max_output_bytes
        )));
    }
    if bounds.timeout_ms > MAX_TIMEOUT_MS {
        return Err(BoundError::AuditRejected(format!(
            "timeout_ms too large: {} (max {MAX_TIMEOUT_MS})",
            bounds.timeout_ms
        )));
    }
    Ok(())
}

/// Caller grant: `caller_permissions` must be a superset of `required`.
pub fn check_permissions(
    plugin: &str,
    required: &[String],
    caller_permissions: &BTreeSet<String>,
) -> Result<(), BoundError> {
    for need in required {
        if !caller_permissions.contains(need) {
            return Err(BoundError::PermissionDenied {
                plugin: plugin.to_string(),
                required: need.clone(),
            });
        }
    }
    Ok(())
}

/// Reject input larger than the declared cap.
pub fn check_input_size(
    plugin: &str,
    input_bytes: usize,
    bounds: &ResourceBounds,
) -> Result<(), BoundError> {
    if input_bytes > bounds.max_input_bytes {
        return Err(BoundError::InputTooLarge {
            actual: input_bytes,
            max: bounds.max_input_bytes,
            plugin: plugin.to_string(),
        });
    }
    Ok(())
}

/// Reject output larger than the declared cap.
pub fn check_output_size(
    plugin: &str,
    output_bytes: usize,
    bounds: &ResourceBounds,
) -> Result<(), BoundError> {
    if output_bytes > bounds.max_output_bytes {
        return Err(BoundError::OutputTooLarge {
            actual: output_bytes,
            max: bounds.max_output_bytes,
            plugin: plugin.to_string(),
        });
    }
    Ok(())
}

/// Combined pre-call check: permissions then input size.
pub fn check_call(
    plugin: &str,
    required: &[String],
    caller_permissions: &BTreeSet<String>,
    input_bytes: usize,
    bounds: &ResourceBounds,
) -> Result<(), BoundError> {
    check_permissions(plugin, required, caller_permissions)?;
    check_input_size(plugin, input_bytes, bounds)
}

/// Default caller grant from the donor sandbox (`invoke` + `read`).
pub fn default_caller_permissions() -> BTreeSet<String> {
    ["invoke", "read"].into_iter().map(str::to_string).collect()
}

/// Privileged grant from the donor sandbox.
pub fn privileged_caller_permissions() -> BTreeSet<String> {
    [
        "invoke",
        "read",
        "write",
        "system",
        "llm_call",
        "ask_user",
        "render",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

/// Validate a live [`PluginManifest`] description length and (if present)
/// strict version. Plugin ids already pass core's stable-id grammar.
///
/// Opt-in: [`crate::PluginManager::register`] does not call this. Donor
/// `extension.toml` required a description and a semver-like version; v2
/// manifests still allow a free-form version unless a caller asks.
pub fn validate_plugin_manifest_text(manifest: &PluginManifest) -> Result<(), BoundError> {
    if manifest.description.is_empty() {
        return Err(BoundError::Schema("description must not be empty".into()));
    }
    if manifest.description.len() > MAX_DESC_LEN {
        return Err(BoundError::Schema(format!(
            "description too long: {} > {MAX_DESC_LEN}",
            manifest.description.len()
        )));
    }
    if !manifest.version.is_empty() {
        validate_version(&manifest.version)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::PluginId;
    use crate::manifest::PluginManifest;

    #[test]
    fn kebab_id_rules() {
        assert!(is_valid_kebab("summarize-text"));
        assert!(is_valid_kebab("a"));
        assert!(!is_valid_kebab(""));
        assert!(!is_valid_kebab("-a"));
        assert!(!is_valid_kebab("a-"));
        assert!(!is_valid_kebab("a--b"));
        assert!(!is_valid_kebab("Summarize"));
        assert!(!is_valid_kebab("a_b"));
        assert!(!is_valid_kebab("tool.shell"));
    }

    #[test]
    fn name_rules() {
        assert!(validate_extension_name("my-plugin").is_ok());
        assert!(validate_extension_name("MyPlugin").is_err());
        assert!(validate_extension_name("").is_err());
        assert!(validate_extension_name(&"a".repeat(65)).is_err());
    }

    #[test]
    fn version_is_strict_semver() {
        assert!(validate_version("0.1.0").is_ok());
        assert!(validate_version("1.0.0-rc.1").is_ok());
        assert!(validate_version("1.0").is_err());
        assert!(validate_version("").is_err());
    }

    #[test]
    fn duplicate_permission_rejected() {
        let err = validate_permissions(&["read".into(), "read".into()]).unwrap_err();
        assert!(matches!(err, BoundError::Schema(_)));
    }

    #[test]
    fn bounds_schema_and_audit() {
        let ok = ResourceBounds::default_limits();
        assert!(validate_resource_bounds(&ok).is_ok());
        assert!(audit_bounds(&["invoke".into()], &ok).is_ok());

        let tiny = ResourceBounds {
            max_input_bytes: 100,
            max_output_bytes: 100,
            timeout_ms: 1,
        };
        assert!(validate_resource_bounds(&tiny).is_ok());
        assert!(matches!(
            audit_bounds(&["invoke".into()], &tiny),
            Err(BoundError::AuditRejected(_))
        ));
        assert!(matches!(
            audit_bounds(&[], &ok),
            Err(BoundError::AuditRejected(_))
        ));

        let huge = ResourceBounds {
            max_input_bytes: 100,
            max_output_bytes: 100,
            timeout_ms: 9_999_999,
        };
        assert!(validate_resource_bounds(&huge).is_err());
    }

    #[test]
    fn sandbox_permission_and_size() {
        let bounds = ResourceBounds {
            max_input_bytes: 100,
            max_output_bytes: 50,
            timeout_ms: 1_000,
        };
        let caller = privileged_caller_permissions();
        assert!(check_call("p", &["invoke".into(), "write".into()], &caller, 100, &bounds).is_ok());
        assert!(matches!(
            check_call("p", &["invoke".into()], &caller, 101, &bounds),
            Err(BoundError::InputTooLarge { actual: 101, .. })
        ));
        assert!(matches!(
            check_output_size("p", 51, &bounds),
            Err(BoundError::OutputTooLarge { .. })
        ));
        assert!(matches!(
            check_call(
                "p",
                &["write".into()],
                &default_caller_permissions(),
                10,
                &bounds
            ),
            Err(BoundError::PermissionDenied { .. })
        ));
        assert!(check_call("p", &["invoke".into()], &default_caller_permissions(), 10, &bounds).is_ok());
    }

    #[test]
    fn plugin_manifest_text_rejects_empty_description_and_loose_version() {
        let m = PluginManifest::new(PluginId::new("builtin.x").unwrap(), "1.0.0", "");
        assert!(validate_plugin_manifest_text(&m).is_err());
        let m = PluginManifest::new(PluginId::new("builtin.x").unwrap(), "1.0", "ok");
        assert!(validate_plugin_manifest_text(&m).is_err());
        let m = PluginManifest::new(PluginId::new("builtin.x").unwrap(), "1.0.0", "ok");
        assert!(validate_plugin_manifest_text(&m).is_ok());
    }
}
