//! manifest — extension.toml 严格 schema 解析
//!
//! ## 严格 schema (必填 + 类型 + 范围)
//! ```toml
//! [extension]
//! name = "my-plugin"            # 必填, 1..=64, [a-z0-9-_]
//! version = "0.1.0"              # 必填, semver
//! kind = "sync"                  # 必填, 必须 ∈ PluginKind
//! description = "..."            # 必填, 1..=512
//! entry = "lib.rs"               # 必填, 1..=256
//!
//! permissions = ["invoke"]       # 必填, 0..=32 项, 每项 1..=64
//!
//! max_input_bytes = 65536        # 必填, 1..=16 MiB
//! max_output_bytes = 65536       # 必填, 1..=16 MiB
//! timeout_ms = 1000              # 必填, 1..=600_000
//! ```
//!
//! 任何字段缺失/类型错/范围越界 → `ExtensionError::ManifestSchema`.

use crate::error::{ExtensionError, Result};
use crate::types::PluginKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAX_NAME_LEN: usize = 64;
const MAX_DESC_LEN: usize = 512;
const MAX_ENTRY_LEN: usize = 256;
const MAX_PERMISSION_LEN: usize = 64;
const MAX_PERMISSIONS: usize = 32;
const MAX_VERSION_LEN: usize = 32;
const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024; // 16 MiB
const MAX_TIMEOUT_MS: u64 = 600_000; // 10 min

/// Manifest (解析后, 强类型, 已校验)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// 扩展名
    pub name: String,
    /// 版本
    pub version: String,
    /// 类型
    pub kind: PluginKind,
    /// 描述
    pub description: String,
    /// 入口
    pub entry: String,
    /// 权限列表
    pub permissions: Vec<String>,
    /// 输入大小上限 (字节)
    pub max_input_bytes: usize,
    /// 输出大小上限 (字节)
    pub max_output_bytes: usize,
    /// 执行超时 (毫秒)
    pub timeout_ms: u64,
}

/// TOML 原始结构 (用于反序列化)
#[derive(Debug, Deserialize)]
struct RawManifest {
    #[serde(default)]
    extension: Option<RawExtension>,
}

#[derive(Debug, Deserialize)]
struct RawExtension {
    name: Option<String>,
    version: Option<String>,
    kind: Option<String>,
    description: Option<String>,
    entry: Option<String>,
    permissions: Option<Vec<String>>,
    max_input_bytes: Option<usize>,
    max_output_bytes: Option<usize>,
    timeout_ms: Option<u64>,
}

impl Manifest {
    /// 从 TOML 文本解析
    pub fn from_toml(text: &str) -> Result<Self> {
        let raw: RawManifest =
            toml::from_str(text).map_err(|e| ExtensionError::ManifestParse(e.to_string()))?;
        let ext = raw
            .extension
            .ok_or_else(|| ExtensionError::ManifestSchema("missing [extension] section".into()))?;
        Self::build(ext)
    }

    fn build(ext: RawExtension) -> Result<Self> {
        // 严格校验每字段
        let name = ext
            .name
            .ok_or_else(|| ExtensionError::ManifestSchema("missing extension.name".into()))?;
        validate_name(&name)?;

        let version = ext
            .version
            .ok_or_else(|| ExtensionError::ManifestSchema("missing extension.version".into()))?;
        validate_version(&version)?;

        let kind_str = ext
            .kind
            .ok_or_else(|| ExtensionError::ManifestSchema("missing extension.kind".into()))?;
        let kind = PluginKind::parse(&kind_str)
            .ok_or_else(|| ExtensionError::ManifestSchema(format!("invalid kind: {kind_str}")))?;

        let description = ext.description.ok_or_else(|| {
            ExtensionError::ManifestSchema("missing extension.description".into())
        })?;
        if description.is_empty() {
            return Err(ExtensionError::ManifestSchema(
                "description must not be empty".into(),
            ));
        }
        if description.len() > MAX_DESC_LEN {
            return Err(ExtensionError::ManifestSchema(format!(
                "description too long: {} > {MAX_DESC_LEN}",
                description.len()
            )));
        }

        let entry = ext
            .entry
            .ok_or_else(|| ExtensionError::ManifestSchema("missing extension.entry".into()))?;
        if entry.is_empty() || entry.len() > MAX_ENTRY_LEN {
            return Err(ExtensionError::ManifestSchema(format!(
                "entry must be 1..={MAX_ENTRY_LEN} chars"
            )));
        }

        let permissions = ext.permissions.unwrap_or_default();
        if permissions.len() > MAX_PERMISSIONS {
            return Err(ExtensionError::ManifestSchema(format!(
                "permissions list too long: {} > {MAX_PERMISSIONS}",
                permissions.len()
            )));
        }
        let mut seen = BTreeSet::new();
        for p in &permissions {
            if p.is_empty() || p.len() > MAX_PERMISSION_LEN {
                return Err(ExtensionError::ManifestSchema(format!(
                    "permission must be 1..={MAX_PERMISSION_LEN} chars: {p:?}"
                )));
            }
            if !seen.insert(p.clone()) {
                return Err(ExtensionError::ManifestSchema(format!(
                    "duplicate permission: {p}"
                )));
            }
        }

        let max_input_bytes = ext.max_input_bytes.ok_or_else(|| {
            ExtensionError::ManifestSchema("missing extension.max_input_bytes".into())
        })?;
        if max_input_bytes < 64 || max_input_bytes > MAX_INPUT_BYTES {
            return Err(ExtensionError::ManifestSchema(format!(
                "max_input_bytes must be 64..={MAX_INPUT_BYTES}"
            )));
        }

        let max_output_bytes = ext.max_output_bytes.ok_or_else(|| {
            ExtensionError::ManifestSchema("missing extension.max_output_bytes".into())
        })?;
        if max_output_bytes < 64 || max_output_bytes > MAX_INPUT_BYTES {
            return Err(ExtensionError::ManifestSchema(format!(
                "max_output_bytes must be 64..={MAX_INPUT_BYTES}"
            )));
        }

        let timeout_ms = ext
            .timeout_ms
            .ok_or_else(|| ExtensionError::ManifestSchema("missing extension.timeout_ms".into()))?;
        if timeout_ms == 0 || timeout_ms > MAX_TIMEOUT_MS {
            return Err(ExtensionError::ManifestSchema(format!(
                "timeout_ms must be 1..={MAX_TIMEOUT_MS}"
            )));
        }

        Ok(Manifest {
            name,
            version,
            kind,
            description,
            entry,
            permissions,
            max_input_bytes,
            max_output_bytes,
            timeout_ms,
        })
    }
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ExtensionError::ManifestSchema(
            "name must not be empty".into(),
        ));
    }
    if name.len() > MAX_NAME_LEN {
        return Err(ExtensionError::ManifestSchema(format!(
            "name too long: {} > {MAX_NAME_LEN}",
            name.len()
        )));
    }
    let valid = name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !valid {
        return Err(ExtensionError::ManifestSchema(format!(
            "name must be [a-z0-9-_] only: {name:?}"
        )));
    }
    Ok(())
}

fn validate_version(v: &str) -> Result<()> {
    if v.is_empty() || v.len() > MAX_VERSION_LEN {
        return Err(ExtensionError::ManifestSchema(format!(
            "version must be 1..={MAX_VERSION_LEN} chars: {v:?}"
        )));
    }
    // semver 简版: x.y.z, 允许 pre-release (e.g. "1.0.0-rc1")
    let mut dots = 0;
    for c in v.chars() {
        if c == '.' {
            dots += 1;
        } else if !c.is_ascii_digit() && c != '-' && !c.is_ascii_alphabetic() {
            return Err(ExtensionError::ManifestSchema(format!(
                "invalid version char: {c:?} in {v:?}"
            )));
        }
    }
    if dots < 2 {
        return Err(ExtensionError::ManifestSchema(format!(
            "version must be semver-like (x.y.z): {v:?}"
        )));
    }
    Ok(())
}

// ============== tests ==============
#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
[extension]
name = "my-plugin"
version = "0.1.0"
kind = "sync"
description = "A test plugin"
entry = "lib.rs"
permissions = ["invoke", "read"]
max_input_bytes = 65536
max_output_bytes = 65536
timeout_ms = 1000
"#;

    #[test]
    fn parse_valid_manifest() {
        let m = Manifest::from_toml(VALID_TOML).unwrap();
        assert_eq!(m.name, "my-plugin");
        assert_eq!(m.kind, PluginKind::Sync);
        assert_eq!(m.permissions.len(), 2);
        assert_eq!(m.max_input_bytes, 65536);
    }

    #[test]
    fn missing_section() {
        let toml = r#"name = "x""#;
        let res = Manifest::from_toml(toml);
        assert!(matches!(res, Err(ExtensionError::ManifestSchema(_))));
    }

    #[test]
    fn missing_name() {
        let toml = r#"
[extension]
version = "0.1.0"
kind = "sync"
description = "x"
entry = "x.rs"
permissions = []
max_input_bytes = 100
max_output_bytes = 100
timeout_ms = 1
"#;
        let res = Manifest::from_toml(toml);
        assert!(matches!(res, Err(ExtensionError::ManifestSchema(_))));
    }

    #[test]
    fn invalid_kind() {
        let toml = r#"
[extension]
name = "x"
version = "0.1.0"
kind = "alien"
description = "x"
entry = "x.rs"
permissions = []
max_input_bytes = 100
max_output_bytes = 100
timeout_ms = 1
"#;
        let res = Manifest::from_toml(toml);
        assert!(matches!(res, Err(ExtensionError::ManifestSchema(_))));
    }

    #[test]
    fn name_uppercase_rejected() {
        let toml = r#"
[extension]
name = "MyPlugin"
version = "0.1.0"
kind = "sync"
description = "x"
entry = "x.rs"
permissions = []
max_input_bytes = 100
max_output_bytes = 100
timeout_ms = 1
"#;
        let res = Manifest::from_toml(toml);
        assert!(matches!(res, Err(ExtensionError::ManifestSchema(_))));
    }

    #[test]
    fn timeout_too_large() {
        let toml = r#"
[extension]
name = "x"
version = "0.1.0"
kind = "sync"
description = "x"
entry = "x.rs"
permissions = []
max_input_bytes = 100
max_output_bytes = 100
timeout_ms = 9999999
"#;
        let res = Manifest::from_toml(toml);
        assert!(matches!(res, Err(ExtensionError::ManifestSchema(_))));
    }

    #[test]
    fn duplicate_permission_rejected() {
        let toml = r#"
[extension]
name = "x"
version = "0.1.0"
kind = "sync"
description = "x"
entry = "x.rs"
permissions = ["read", "read"]
max_input_bytes = 100
max_output_bytes = 100
timeout_ms = 1
"#;
        let res = Manifest::from_toml(toml);
        assert!(matches!(res, Err(ExtensionError::ManifestSchema(_))));
    }

    #[test]
    fn all_6_kinds_parseable() {
        for k in PluginKind::ALL {
            let toml = format!(
                r#"
[extension]
name = "x"
version = "0.1.0"
kind = "{}"
description = "x"
entry = "x.rs"
permissions = []
max_input_bytes = 100
max_output_bytes = 100
timeout_ms = 1
"#,
                k.as_str()
            );
            let m = Manifest::from_toml(&toml).unwrap();
            assert_eq!(m.kind, *k);
        }
    }
}
