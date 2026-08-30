//! Polling mtime watcher used as a cache-invalidation signal.
//!
//! Recovered from `legacy/donor/apeireth-skills/src/watcher.rs` (mtime compare,
//! Added / Modified / Removed). The donor `apeireth-agent` `watch_dir` used
//! `notify` 5.x; v2 plugin explicitly out-of-scopes inotify/hot-reload of
//! plugin code. This helper only reports **file identity changes** so a
//! caller can `clear_cache` on a [`crate::alias::AliasResolver`]. It does
//! not load plugins, spawn a thread, or own a registry.
//!
//! Zero extra dependencies: `std::fs` only.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// File-identity change relative to the last scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// Path was not in the previous snapshot.
    Added(PathBuf),
    /// Path existed and its mtime increased.
    Modified(PathBuf),
    /// Path was in the previous snapshot and is gone.
    Removed(PathBuf),
    /// The scan itself failed (permissions, missing dir, …).
    ScanError(String),
}

impl WatchEvent {
    /// Path for Added / Modified / Removed.
    pub fn path(&self) -> Option<&Path> {
        match self {
            WatchEvent::Added(p) | WatchEvent::Modified(p) | WatchEvent::Removed(p) => Some(p),
            WatchEvent::ScanError(_) => None,
        }
    }

    /// Stable kind name.
    pub fn kind_str(&self) -> &'static str {
        match self {
            WatchEvent::Added(_) => "added",
            WatchEvent::Modified(_) => "modified",
            WatchEvent::Removed(_) => "removed",
            WatchEvent::ScanError(_) => "scan_error",
        }
    }

    /// Whether this event should drop a lookup cache.
    pub fn invalidates_cache(&self) -> bool {
        matches!(
            self,
            WatchEvent::Added(_) | WatchEvent::Modified(_) | WatchEvent::Removed(_)
        )
    }
}

/// Polling snapshot of a directory of metadata files.
///
/// Default filter: files whose name is `descriptor.json` (nested layout) or
/// whose extension is `.json` directly under `root` (flat layout). Callers
/// that watch a different set can pass their own listing through
/// [`MetadataWatcher::diff_against`].
pub struct MetadataWatcher {
    root: PathBuf,
    known: HashMap<PathBuf, u64>,
    max_depth: usize,
}

impl std::fmt::Debug for MetadataWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetadataWatcher")
            .field("root", &self.root)
            .field("known_count", &self.known.len())
            .field("max_depth", &self.max_depth)
            .finish()
    }
}

impl MetadataWatcher {
    /// Watch `root`. Depth 4 matches the donor file-loader bound (loop guard).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            known: HashMap::new(),
            max_depth: 4,
        }
    }

    /// Override walk depth (0 = only `root` itself as a file; 1 = direct children).
    #[must_use]
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Directory being watched.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Number of paths in the baseline.
    pub fn known_count(&self) -> usize {
        self.known.len()
    }

    /// Fill the baseline without emitting events.
    pub fn scan_initial(&mut self) -> Result<usize, String> {
        let files = discover_metadata_files(&self.root, self.max_depth)?;
        self.known.clear();
        let mut count = 0;
        for path in files {
            if let Some(mtime) = file_mtime_unix(&path) {
                self.known.insert(path, mtime);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Compare the current listing to the baseline and update it.
    pub fn check_for_changes(&mut self) -> Vec<WatchEvent> {
        let current = match discover_metadata_files(&self.root, self.max_depth) {
            Ok(v) => v,
            Err(e) => return vec![WatchEvent::ScanError(e)],
        };
        self.diff_against(&current)
    }

    /// Drop the baseline and treat every current file as Added.
    pub fn scan_once(&mut self) -> Vec<WatchEvent> {
        self.known.clear();
        self.check_for_changes()
    }

    /// Diff `current_files` against `known`, updating the baseline.
    pub fn diff_against(&mut self, current_files: &[PathBuf]) -> Vec<WatchEvent> {
        let mut events = Vec::new();
        let current_set: HashSet<&PathBuf> = current_files.iter().collect();

        for path in current_files {
            let Some(current_mtime) = file_mtime_unix(path) else {
                continue;
            };
            match self.known.get(path) {
                None => {
                    events.push(WatchEvent::Added(path.clone()));
                    self.known.insert(path.clone(), current_mtime);
                }
                Some(&known_mtime) => {
                    if current_mtime > known_mtime {
                        events.push(WatchEvent::Modified(path.clone()));
                        self.known.insert(path.clone(), current_mtime);
                    }
                }
            }
        }

        let removed: Vec<PathBuf> = self
            .known
            .keys()
            .filter(|p| !current_set.contains(p))
            .cloned()
            .collect();
        for path in removed {
            events.push(WatchEvent::Removed(path.clone()));
            self.known.remove(&path);
        }
        events
    }
}

/// Unix-seconds mtime, or `None` if the file cannot be stat'd.
pub fn file_mtime_unix(path: &Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let duration = mtime.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    Some(duration.as_secs())
}

/// Nested `descriptor.json` plus flat `*.json` directly under `root`.
pub fn discover_metadata_files(root: &Path, max_depth: usize) -> Result<Vec<PathBuf>, String> {
    if !root.exists() {
        return Err(format!("root does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("root is not a directory: {}", root.display()));
    }
    let mut paths = Vec::new();
    walk(root, root, 0, max_depth, &mut paths);
    paths.sort();
    Ok(paths)
}

fn walk(root: &Path, dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        if file_type.is_dir() {
            walk(root, &path, depth + 1, max_depth, out);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name == "descriptor.json" {
            out.push(path);
            continue;
        }
        if name.ends_with(".json") && path.parent() == Some(root) {
            out.push(path);
        }
    }
}

/// True when any event is a content change (not a scan error).
pub fn should_invalidate(events: &[WatchEvent]) -> bool {
    events.iter().any(WatchEvent::invalidates_cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    fn temp_dir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "apeireth-plugin-watch-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&base).unwrap();
        base
    }

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    fn sleep_mtime() {
        // Windows FAT/some FS have 1s mtime resolution.
        std::thread::sleep(Duration::from_millis(1100));
    }

    #[test]
    fn event_kind_and_invalidate() {
        let p = PathBuf::from("x.json");
        assert_eq!(WatchEvent::Added(p.clone()).kind_str(), "added");
        assert!(WatchEvent::Modified(p.clone()).invalidates_cache());
        assert!(!WatchEvent::ScanError("e".into()).invalidates_cache());
        assert_eq!(WatchEvent::ScanError("e".into()).path(), None);
    }

    #[test]
    fn scan_initial_is_silent() {
        let dir = temp_dir();
        write(&dir.join("a.json"), "{}");
        write(&dir.join("nested").join("descriptor.json"), "{}");
        let mut w = MetadataWatcher::new(&dir);
        let n = w.scan_initial().unwrap();
        assert_eq!(n, 2);
        assert_eq!(w.known_count(), 2);
        let events = w.check_for_changes();
        assert!(events.is_empty(), "{events:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn added_modified_removed() {
        let dir = temp_dir();
        write(&dir.join("a.json"), "1");
        let mut w = MetadataWatcher::new(&dir);
        w.scan_initial().unwrap();

        write(&dir.join("b.json"), "2");
        let events = w.check_for_changes();
        assert!(
            events.iter().any(|e| matches!(e, WatchEvent::Added(p) if p.ends_with("b.json"))),
            "{events:?}"
        );
        assert!(should_invalidate(&events));

        sleep_mtime();
        write(&dir.join("a.json"), "changed");
        let events = w.check_for_changes();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, WatchEvent::Modified(p) if p.ends_with("a.json"))),
            "{events:?}"
        );

        fs::remove_file(dir.join("b.json")).unwrap();
        let events = w.check_for_changes();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, WatchEvent::Removed(p) if p.ends_with("b.json"))),
            "{events:?}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_once_emits_added_for_existing() {
        let dir = temp_dir();
        write(&dir.join("a.json"), "{}");
        let mut w = MetadataWatcher::new(&dir);
        let events = w.scan_once();
        assert!(events.iter().any(|e| matches!(e, WatchEvent::Added(_))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_root_is_scan_error() {
        let mut w = MetadataWatcher::new("/definitely/not/a/real/apeireth/watch/dir");
        let events = w.check_for_changes();
        assert!(matches!(events.as_slice(), [WatchEvent::ScanError(_)]));
        assert!(!should_invalidate(&events));
    }

    #[test]
    fn nested_json_other_than_descriptor_is_ignored() {
        let dir = temp_dir();
        write(&dir.join("nested").join("other.json"), "{}");
        write(&dir.join("nested").join("descriptor.json"), "{}");
        let files = discover_metadata_files(&dir, 4).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("descriptor.json"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn file_mtime_unix_none_for_missing() {
        assert!(file_mtime_unix(Path::new("/no/such/file-apeireth")).is_none());
    }
}
