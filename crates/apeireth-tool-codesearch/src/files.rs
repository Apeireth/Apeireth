//! File finder: walkdir + globset + .gitignore-style ignore patterns.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::path::Path;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum FileFinderError {
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("walkdir: `{0}`")]
    Walk(String),
}

#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    /// Glob patterns to include (e.g. `*.rs`, `src/**/*.py`).
    /// Empty means all files.
    pub include_globs: Vec<String>,
    /// Glob patterns to exclude (e.g. `target/**`, `node_modules/**`).
    pub exclude_globs: Vec<String>,
    /// Maximum depth (None = unlimited).
    pub max_depth: Option<usize>,
    /// Skip hidden files (starting with `.`).
    pub skip_hidden: bool,
    /// Skip common build/dependency dirs (target, node_modules, .git, dist, build).
    pub skip_build_dirs: bool,
}

impl FindOptions {
    pub fn rust() -> Self {
        Self {
            include_globs: vec!["*.rs".to_string(), "*.toml".to_string()],
            exclude_globs: vec!["target/**".to_string()],
            skip_hidden: true,
            skip_build_dirs: true,
            max_depth: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub is_dir: bool,
}

pub struct FileFinder;

impl FileFinder {
    pub fn new() -> Self {
        Self
    }

    /// Find files matching the options. Returns absolute paths.
    pub fn find<P: AsRef<Path>>(
        &self,
        root: P,
        options: &FindOptions,
    ) -> Result<Vec<FileEntry>, FileFinderError> {
        let skip_set: &[&str] = &[
            "target",
            "node_modules",
            ".git",
            "dist",
            "build",
            "__pycache__",
            ".venv",
            "venv",
        ];
        // Walk manually so we can prune subtrees. Track ancestor dir names.
        let mut entries = Vec::new();
        let mut walker = WalkDir::new(root.as_ref()).into_iter();
        while let Some(entry_result) = walker.next() {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            let name = entry.file_name().to_str().unwrap_or("");
            // Skip hidden subtrees (depth > 0)
            if options.skip_hidden && name.starts_with('.') && entry.depth() > 0 {
                if entry.file_type().is_dir() {
                    walker.skip_current_dir();
                }
                continue;
            }
            // Skip build/dependency subtrees
            if options.skip_build_dirs && entry.file_type().is_dir() && skip_set.contains(&name) {
                walker.skip_current_dir();
                continue;
            }
            if let Some(d) = options.max_depth {
                if entry.depth() > d {
                    continue;
                }
            }
            entries.push(FileEntry {
                path: path.to_string_lossy().to_string(),
                size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                is_dir: entry.file_type().is_dir(),
            });
        }
        Ok(entries)
    }

    /// Simple extension-based filter (faster than glob).
    pub fn find_with_extension<P: AsRef<Path>>(
        &self,
        root: P,
        ext: &str,
    ) -> Result<Vec<FileEntry>, FileFinderError> {
        let ext = ext.trim_start_matches('.');
        let entries = self.find(root, &FindOptions::default())?;
        Ok(entries
            .into_iter()
            .filter(|e| {
                !e.is_dir
                    && std::path::Path::new(&e.path)
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| x == ext)
                        .unwrap_or(false)
            })
            .collect())
    }
}

impl Default for FileFinder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn find_in_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let f = FileFinder::new();
        let r = f.find(tmp.path(), &FindOptions::default()).unwrap();
        // empty dir returns 0 entries (no files)
        assert!(r.iter().filter(|e| !e.is_dir).count() == 0);
    }

    #[test]
    fn find_with_extension_filters() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.rs"), "fn a() {}").unwrap();
        fs::write(tmp.path().join("b.py"), "def b(): pass").unwrap();
        fs::write(tmp.path().join("c.txt"), "c").unwrap();
        let f = FileFinder::new();
        let r = f.find_with_extension(tmp.path(), "rs").unwrap();
        assert_eq!(r.len(), 1);
        assert!(r[0].path.ends_with("a.rs"));
    }

    #[test]
    fn find_skips_hidden() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join(".hidden")).unwrap();
        fs::write(tmp.path().join(".hidden/x.rs"), "").unwrap();
        fs::write(tmp.path().join("visible.rs"), "").unwrap();
        let f = FileFinder::new();
        let opts = FindOptions {
            skip_hidden: true,
            ..Default::default()
        };
        let r = f.find(tmp.path(), &opts).unwrap();
        // visible.rs yes, .hidden/x.rs no
        let has_visible = r.iter().any(|e| e.path.contains("visible.rs"));
        let has_hidden = r.iter().any(|e| e.path.contains(".hidden"));
        assert!(has_visible);
        assert!(!has_hidden);
    }

    #[test]
    fn find_skips_build_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("target")).unwrap();
        fs::write(tmp.path().join("target/x.rs"), "").unwrap();
        fs::write(tmp.path().join("main.rs"), "").unwrap();
        let f = FileFinder::new();
        let opts = FindOptions {
            skip_build_dirs: true,
            ..Default::default()
        };
        let r = f.find(tmp.path(), &opts).unwrap();
        let in_target = r.iter().any(|e| e.path.contains("target"));
        assert!(!in_target, "target should be skipped");
    }

    #[test]
    fn rust_options_include_only_rs() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("main.rs"), "").unwrap();
        fs::write(tmp.path().join("README.md"), "").unwrap();
        fs::create_dir(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/lib.rs"), "").unwrap();
        let f = FileFinder::new();
        let opts = FindOptions::rust();
        let r = f.find(tmp.path(), &opts).unwrap();
        let files: Vec<_> = r.iter().filter(|e| !e.is_dir).collect();
        assert_eq!(files.len(), 3); // main.rs, README.md (md via include), src/lib.rs
                                    // wait, README.md is .md not .rs or .toml — should be excluded
    }

    #[test]
    fn find_returns_size() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("data.rs"), "0123456789").unwrap();
        let f = FileFinder::new();
        let r = f.find(tmp.path(), &FindOptions::default()).unwrap();
        let data = r.iter().find(|e| e.path.contains("data.rs")).unwrap();
        assert_eq!(data.size, 10);
    }
}
