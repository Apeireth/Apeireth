//! Workspace directory selection and persistence.
//!
//! The workspace directory is where the sidecar's session/cognitive SQLite
//! stores live once the user has explicitly chosen one. The directory path is
//! not secret, so it is persisted as plain JSON under the app-data directory
//! (`companion-config.json`).

use std::path::{Path, PathBuf};

/// App-data config file that holds the workspace directory (non-secret).
pub const WORKSPACE_CONFIG_FILE: &str = "companion-config.json";

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CompanionConfig {
    pub workspace_dir: Option<String>,
}

/// Read the persisted workspace dir, returning it only if it still exists.
pub fn load_workspace_dir(app_data_dir: &Path) -> Option<PathBuf> {
    let raw = std::fs::read_to_string(app_data_dir.join(WORKSPACE_CONFIG_FILE)).ok()?;
    let config: CompanionConfig = serde_json::from_str(&raw).ok()?;
    let path = PathBuf::from(config.workspace_dir?);
    path.is_dir().then_some(path)
}

/// Persist the workspace dir (non-secret) to the app-data config.
pub fn persist_workspace_dir(app_data_dir: &Path, dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| format!("failed to create app-data dir: {e}"))?;
    let config = CompanionConfig {
        workspace_dir: Some(dir.to_string_lossy().to_string()),
    };
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("failed to serialize workspace config: {e}"))?;
    std::fs::write(app_data_dir.join(WORKSPACE_CONFIG_FILE), json)
        .map_err(|e| format!("failed to write workspace config: {e}"))
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
    std::fs::write(&probe, b"").map_err(|e| {
        format!("directory is not writable ({}): {e}", dir.display())
    })?;
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
        let dir = std::env::temp_dir().join(format!(
            "apeireth-ws-missing-{}",
            std::process::id()
        ));
        assert!(ensure_writable_dir(&dir).is_err(), "missing dir must fail");

        let file = std::env::temp_dir().join(format!(
            "apeireth-ws-file-{}.txt",
            std::process::id()
        ));
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
}
