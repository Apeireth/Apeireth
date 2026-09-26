//! Workspace directory selection and persistence.
//!
//! The workspace directory is where the sidecar's session/cognitive SQLite
//! stores live once the user has explicitly chosen one. The directory path is
//! not secret, so it is persisted as plain JSON under the app-data directory
//! (`companion-config.json`) — as a stored-document envelope (identity +
//! version stamp + body) opened strictly through the shared storage
//! primitives: malformed / foreign / version-unsupported files are **rejected**
//! with a typed error instead of silently falling back to defaults. Pre-envelope
//! files stay readable read-only; upgrading one is an explicit
//! [`migrate_legacy_workspace_config`] call that leaves an audit line.

use std::path::{Path, PathBuf};

use apeireth_core::stored_doc::DocCompat;

use crate::stored_config;

/// App-data config file that holds the workspace directory (non-secret).
pub const WORKSPACE_CONFIG_FILE: &str = "companion-config.json";

/// 存储文档身份: 工作区配置。
pub const WORKSPACE_DOC_NAME: &str = "companion-workspace-config";
/// 工作区配置存储格式版本。
pub const WORKSPACE_DOC_VERSION: u32 = 1;

fn doc_compat() -> DocCompat {
    DocCompat::exact(WORKSPACE_DOC_NAME, WORKSPACE_DOC_VERSION)
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CompanionConfig {
    pub workspace_dir: Option<String>,
}

/// Read the persisted workspace dir, returning it only if it still exists.
///
/// - 未配置/文件缺失 → `Ok(None)` (调用方决定默认落位);
/// - 配置存在但畸形/串档/版本不符 → `Err` **拒开**, 不回退默认值;
/// - 前信封旧裸体文件只读兼容 (不静默迁移)。
pub fn load_workspace_dir(app_data_dir: &Path) -> Result<Option<PathBuf>, String> {
    let path = app_data_dir.join(WORKSPACE_CONFIG_FILE);
    let config: Option<CompanionConfig> = stored_config::load_config(&path, &doc_compat())?;
    Ok(config
        .and_then(|config| config.workspace_dir)
        .map(PathBuf::from)
        .filter(|path| path.is_dir()))
}

/// Persist the workspace dir (non-secret) to the app-data config.
pub fn persist_workspace_dir(app_data_dir: &Path, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| format!("failed to create app-data dir: {e}"))?;
    let config = CompanionConfig {
        workspace_dir: Some(dir.to_string_lossy().to_string()),
    };
    stored_config::save_config(
        &app_data_dir.join(WORKSPACE_CONFIG_FILE),
        &doc_compat(),
        &config,
    )
}

/// Explicit migration of a pre-envelope workspace config into the current
/// stored-document format (rewrites the file + appends an audit line). The
/// default open path ([`load_workspace_dir`]) never migrates on its own.
pub fn migrate_legacy_workspace_config(app_data_dir: &Path) -> Result<(), String> {
    let path = app_data_dir.join(WORKSPACE_CONFIG_FILE);
    if !path.exists() {
        return Ok(());
    }
    stored_config::migrate_legacy_config::<CompanionConfig>(&path, &doc_compat()).map(|_| ())
}

/// Resolve the directory that owns the sidecar's SQLite stores.
///
/// A configured workspace wins (`<workspace>/.apeireth`, keeping the CLI's
/// default `.apeireth` convention); otherwise the app-data anchor
/// (`<app_data>/data`) preserves the pre-workspace behavior. Returns `None`
/// only for logger-less test supervisors.
pub fn resolve_store_dir(
    workspace_dir: Option<&Path>,
    app_data_dir: Option<&Path>,
) -> Option<PathBuf> {
    match workspace_dir {
        Some(ws) => Some(ws.join(".apeireth")),
        None => app_data_dir.map(|d| d.join("data")),
    }
}

/// Normalize a user-supplied directory to an absolute, canonical path without
/// the Windows `\\?\` verbatim prefix `canonicalize` otherwise returns.
pub fn normalize_dir(path: &Path) -> PathBuf {
    match path.canonicalize() {
        Ok(canon) => match canon.to_string_lossy().strip_prefix(r"\\?\") {
            Some(stripped) => PathBuf::from(stripped),
            None => canon,
        },
        Err(_) => path.to_path_buf(),
    }
}

/// Verify `dir` is an existing directory the desktop can write to.
///
/// A probe file is created and removed inside the candidate directory; this is
/// the only reliable cross-platform writability check (read-only ACLs are not
/// fully exposed by `std::fs::Metadata` on Windows).
pub fn ensure_writable_dir(dir: &Path) -> Result<(), String> {
    if !dir.is_dir() {
        return Err(format!("not a directory: {}", dir.display()));
    }
    let probe = dir.join(format!(".apeireth-write-probe-{}", std::process::id()));
    std::fs::write(&probe, b"")
        .map_err(|e| format!("directory is not writable ({}): {e}", dir.display()))?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

/// Common workspace candidates: home, documents, desktop, and the last-used
/// directory. Only existing directories are returned (deduplicated).
pub fn workspace_suggestions(last_used: Option<&Path>) -> Vec<String> {
    fn push_unique(out: &mut Vec<String>, path: PathBuf) {
        if path.is_dir() {
            let value = path.to_string_lossy().to_string();
            if !out.contains(&value) {
                out.push(value);
            }
        }
    }

    let mut out = Vec::new();
    if let Some(home) = home_dir() {
        push_unique(&mut out, home.clone());
        push_unique(&mut out, home.join("Documents"));
        push_unique(&mut out, home.join("Desktop"));
    }
    if let Some(last) = last_used {
        push_unique(&mut out, last.to_path_buf());
    }
    out
}

/// Resolve the current user's home directory.
pub fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_store_dir_prefers_workspace() {
        let ws = PathBuf::from("C:\\Users\\me\\workspace");
        let app = PathBuf::from("C:\\Users\\me\\AppData\\Local\\Apeireth");
        assert_eq!(
            resolve_store_dir(Some(&ws), Some(&app)),
            Some(PathBuf::from("C:\\Users\\me\\workspace\\.apeireth"))
        );
    }

    #[test]
    fn resolve_store_dir_falls_back_to_app_data() {
        let app = PathBuf::from("/tmp/apeireth");
        assert_eq!(
            resolve_store_dir(None, Some(&app)),
            Some(PathBuf::from("/tmp/apeireth/data"))
        );
        assert_eq!(resolve_store_dir(None, None), None);
    }

    #[test]
    fn ensure_writable_dir_rejects_missing_and_files() {
        let dir = std::env::temp_dir().join(format!("apeireth-ws-missing-{}", std::process::id()));
        assert!(ensure_writable_dir(&dir).is_err(), "missing dir must fail");

        let file =
            std::env::temp_dir().join(format!("apeireth-ws-file-{}.txt", std::process::id()));
        std::fs::write(&file, b"x").unwrap();
        assert!(ensure_writable_dir(&file).is_err(), "a file must fail");
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn ensure_writable_dir_accepts_writable_directory_and_cleans_probe() {
        let dir = std::env::temp_dir().join(format!("apeireth-ws-ok-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(ensure_writable_dir(&dir).is_ok());
        let entries = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(entries, 0, "probe file must be removed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn suggestions_are_existing_dirs_and_deduplicated() {
        let home = std::env::temp_dir();
        let suggestions = workspace_suggestions(Some(&home));
        assert!(suggestions.contains(&home.to_string_lossy().to_string()));
        assert!(!suggestions.iter().any(|s| s.is_empty()));
        // No duplicates.
        let mut sorted = suggestions.clone();
        sorted.sort();
        let deduped: Vec<String> = {
            let mut v = sorted.clone();
            v.dedup();
            v
        };
        assert_eq!(sorted, deduped);
    }

    // -----------------------------------------------------------------------
    // stored_doc 消费方接线: 配置拒开 + 好数据零回归 + 旧文件兼容 + 显式迁移
    // -----------------------------------------------------------------------

    fn scratch_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("apeireth-ws-stored-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 拒开生效: 坏配置不再「读坏用默认」, 显式报错。
    #[test]
    fn workspace_config_rejects_corrupt_file_without_default_fallback() {
        let app_data = scratch_dir("reject");
        let target = app_data.join("real-workspace");
        std::fs::create_dir_all(&target).unwrap();
        let path = app_data.join(WORKSPACE_CONFIG_FILE);

        std::fs::write(&path, b"{ this is not json").unwrap();
        assert!(
            load_workspace_dir(&app_data).is_err(),
            "坏 JSON 必须拒开而不是回退默认落位"
        );

        // 信封形状但串档: 同样拒开。
        std::fs::write(
            &path,
            format!(
                r#"{{"name":"other-doc","version":1,"compatible_versions":[1],"body":{{"workspace_dir":"{}"}}}}"#,
                target.display()
            ),
        )
        .unwrap();
        assert!(load_workspace_dir(&app_data).is_err(), "串档信封必须拒开");
        let _ = std::fs::remove_dir_all(&app_data);
    }

    /// 好数据路径零回归: 恢复出的配置与旧裸体写端输出逐字节等价。
    #[test]
    fn workspace_config_good_data_payload_stays_byte_equivalent() {
        let app_data = scratch_dir("equiv");
        let target = app_data.join("real-workspace");
        std::fs::create_dir_all(&target).unwrap();
        persist_workspace_dir(&app_data, &target).unwrap();

        let raw = std::fs::read(app_data.join(WORKSPACE_CONFIG_FILE)).unwrap();
        let doc: apeireth_core::stored_doc::StoredDoc<CompanionConfig> =
            serde_json::from_slice(&raw).expect("落盘是存储文档信封");
        let legacy_bare = serde_json::to_string_pretty(&doc.body).unwrap();
        assert_eq!(
            legacy_bare,
            serde_json::to_string_pretty(&CompanionConfig {
                workspace_dir: Some(target.to_string_lossy().to_string()),
            })
            .unwrap(),
            "好数据 body 必须与旧裸体写端输出逐字节等价"
        );
        assert_eq!(
            load_workspace_dir(&app_data).unwrap(),
            Some(target.clone()),
            "好数据恢复行为零回归"
        );
        let _ = std::fs::remove_dir_all(&app_data);
    }

    /// 旧文件兼容: 前信封裸体配置迁移前后都可读; 默认打开不静默迁移; 迁移留审计。
    #[test]
    fn workspace_config_legacy_file_is_readable_before_and_after_explicit_migration() {
        let app_data = scratch_dir("legacy");
        let target = app_data.join("real-workspace");
        std::fs::create_dir_all(&target).unwrap();
        let legacy_bytes = serde_json::to_vec_pretty(&CompanionConfig {
            workspace_dir: Some(target.to_string_lossy().to_string()),
        })
        .unwrap();
        std::fs::write(app_data.join(WORKSPACE_CONFIG_FILE), &legacy_bytes).unwrap();

        // 迁移前可读 (旧裸体只读兼容), 文件字节不被改写。
        assert_eq!(load_workspace_dir(&app_data).unwrap(), Some(target.clone()));
        assert_eq!(
            std::fs::read(app_data.join(WORKSPACE_CONFIG_FILE)).unwrap(),
            legacy_bytes,
            "默认打开路径不得静默迁移"
        );

        // 显式迁移留审计。
        migrate_legacy_workspace_config(&app_data).expect("显式迁移必须成功");
        let audit_path = app_data.join(format!("{WORKSPACE_CONFIG_FILE}.migrate-audit.jsonl"));
        let audit = std::fs::read_to_string(&audit_path).expect("显式迁移必须留审计");
        assert_eq!(audit.lines().count(), 1, "审计文件应恰一行: {audit}");

        // 迁移后仍可读, 内容不变。
        assert_eq!(load_workspace_dir(&app_data).unwrap(), Some(target));
        let _ = std::fs::remove_dir_all(&app_data);
    }
}
