//! R109: Skill descriptor 文件热加载 (polling-based, 0 新 dep)
//!
//! **目标**: 监听 skill descriptor JSON 文件, 文件变更时给 caller 发事件.
//! caller 决定 reload / 重新构造 registry / 推送给 MCP client.
//!
//! **Apeireth 真接 (本 module)**:
//! - `SkillWatcher` — 持有 root dir + 已知文件 (path → mtime) + 回调
//! - `WatchEvent` — `Added(path)` / `Modified(path)` / `Removed(path)` / `ScanError(msg)`
//! - `check_for_changes() -> Vec<WatchEvent>` — 轮询, 返变更事件
//! - `scan_once() -> Vec<WatchEvent>` — 全量扫描 + 发出所有事件
//! - `skill_files_in(dir) -> Vec<PathBuf>` — 扫 root dir 找所有 .json 描述符 (借 R63 `discover_*`)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `apeireth-skills/src/file_loader.rs` 已有 `discover_descriptor_paths` / `load_registry_from_dir` (R63 LOCKED)
//! - 0 改 `apeireth-skills/src/lib.rs` 已有 8 pub fn (R23 LOCKED)
//! - 0 引入新 dep (复用 walkdir)
//! - 0 改 workspace 1.0.0
//!
//! **借鉴锚 (S-7)**:
//! - LSP `workspace/didChangeWatchedFiles` (文件级事件)
//! - VCP vcptoolbox hot-reload 思路 (mtime compare)
//! - cargo `Metadata` (类似 mtime 比对, 0 引入 notify crate)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use walkdir::WalkDir;

use crate::file_loader::discover_descriptor_paths;

// ============================================================
// 事件
// ============================================================

/// **Skill 文件变更事件**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    /// 新文件 (Added)
    Added(PathBuf),
    /// 文件修改 (mtime 变了)
    Modified(PathBuf),
    /// 文件被删
    Removed(PathBuf),
    /// 扫描过程出错 (e.g. 权限, IO)
    ScanError(String),
}

impl WatchEvent {
    pub fn path(&self) -> Option<&Path> {
        match self {
            WatchEvent::Added(p) | WatchEvent::Modified(p) | WatchEvent::Removed(p) => Some(p),
            WatchEvent::ScanError(_) => None,
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            WatchEvent::Added(_) => "added",
            WatchEvent::Modified(_) => "modified",
            WatchEvent::Removed(_) => "removed",
            WatchEvent::ScanError(_) => "scan_error",
        }
    }
}

// ============================================================
// SkillWatcher
// ============================================================

/// **Skill descriptor 文件 watcher** (polling-based, 0 引入新 dep)
///
/// 用法:
/// ```ignore
/// let mut watcher = SkillWatcher::new(PathBuf::from("./skills"));
/// let _ = watcher.scan_initial();  // 首次扫, 填充 baseline
/// loop {
///     std::thread::sleep(Duration::from_secs(2));
///     for event in watcher.check_for_changes() {
///         // reload registry / 推送给 MCP client
///     }
/// }
/// ```
pub struct SkillWatcher {
    /// 监视的根目录
    root: PathBuf,
    /// 已知文件 path → 上次 mtime (Unix seconds)
    known: HashMap<PathBuf, u64>,
}

impl std::fmt::Debug for SkillWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillWatcher")
            .field("root", &self.root)
            .field("known_count", &self.known.len())
            .finish()
    }
}

impl SkillWatcher {
    /// **构造** — 复用 R63 `discover_descriptor_paths` (自动检嵌套 vs 平铺)
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            known: HashMap::new(),
        }
    }

    /// 监视的根目录
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 已知文件数
    pub fn known_count(&self) -> usize {
        self.known.len()
    }

    /// **首次扫描** — 填充 baseline mtime, 不发事件
    pub fn scan_initial(&mut self) -> Result<usize, String> {
        let files = self.discover_files().map_err(|e| e.to_string())?;
        let mut count = 0;
        for path in files {
            if let Some(mtime) = file_mtime_unix(&path) {
                self.known.insert(path, mtime);
                count += 1;
            }
        }
        Ok(count)
    }

    /// **检查变更** — 扫当前文件列表, 跟 baseline 比对, 发事件
    ///
    /// 不会清空 `known` (保留 baseline 给下次比较)
    pub fn check_for_changes(&mut self) -> Vec<WatchEvent> {
        let mut events = Vec::new();
        let current_files = match self.discover_files() {
            Ok(v) => v,
            Err(e) => {
                return vec![WatchEvent::ScanError(e)];
            }
        };

        // 1. 找 Added / Modified
        for path in &current_files {
            let Some(current_mtime) = file_mtime_unix(path) else { continue };
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

        // 2. 找 Removed (在 known 但不在 current)
        let current_set: std::collections::HashSet<&PathBuf> = current_files.iter().collect();
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

    /// **全量扫描 + 发所有事件** (clear known first)
    ///
    /// 适用于: 启动时一次性 init / 强制重扫
    pub fn scan_once(&mut self) -> Vec<WatchEvent> {
        self.known.clear();
        self.check_for_changes()
    }

    /// **扫 root dir, 返回所有 skill descriptor 路径** (借 R63 `discover_descriptor_paths`)
    fn discover_files(&self) -> Result<Vec<PathBuf>, String> {
        discover_descriptor_paths(&self.root).map_err(|e| e.to_string())
    }
}

// ============================================================
// 工具: 拿文件 mtime (Unix seconds, 失败 -> None)
// ============================================================

/// **拿文件 mtime 转 Unix 秒** (失败 -> None)
pub fn file_mtime_unix(path: &Path) -> Option<u64> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let duration = mtime.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    Some(duration.as_secs())
}

/// **扫 dir, 返回所有 .json 文件** (不依赖 R63, 通用 helper)
pub fn find_json_files(dir: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let walker = if recursive {
        WalkDir::new(dir)
    } else {
        WalkDir::new(dir).max_depth(1)
    };
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "json") {
            out.push(path.to_path_buf());
        }
    }
    out
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    fn make_temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-skills-watcher-{}",
            std::time::SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_skill_json(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        write!(f, r#"{{"id":"{name}","version":"1.0.0"}}"#).unwrap();
        path
    }

    fn touch_to_newer_mtime(path: &Path) {
        // 显式 set mtime 到未来 (避 Windows 100ns 分辨率 + sleep 慢)
        let new_mtime = std::time::SystemTime::now() + std::time::Duration::from_secs(10);
        let f = fs::OpenOptions::new().write(true).open(path).unwrap();
        f.set_modified(new_mtime).unwrap();
    }

    #[test]
    fn watch_event_path_returns_path_for_file_events() {
        let p = PathBuf::from("/tmp/x.json");
        assert_eq!(WatchEvent::Added(p.clone()).path(), Some(p.as_path()));
        assert_eq!(WatchEvent::Modified(p.clone()).path(), Some(p.as_path()));
        assert_eq!(WatchEvent::Removed(p.clone()).path(), Some(p.as_path()));
        assert_eq!(WatchEvent::ScanError("e".into()).path(), None);
    }

    #[test]
    fn watch_event_kind_str() {
        assert_eq!(WatchEvent::Added("x".into()).kind_str(), "added");
        assert_eq!(WatchEvent::Modified("x".into()).kind_str(), "modified");
        assert_eq!(WatchEvent::Removed("x".into()).kind_str(), "removed");
        assert_eq!(WatchEvent::ScanError("e".into()).kind_str(), "scan_error");
    }

    #[test]
    fn file_mtime_unix_returns_some_for_existing() {
        let dir = make_temp_dir();
        let p = write_skill_json(&dir, "test.json");
        let m = file_mtime_unix(&p);
        assert!(m.is_some());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_mtime_unix_returns_none_for_missing() {
        let m = file_mtime_unix(Path::new("/tmp/does-not-exist-12345.json"));
        assert!(m.is_none());
    }

    #[test]
    fn find_json_files_flat() {
        let dir = make_temp_dir();
        write_skill_json(&dir, "a.json");
        write_skill_json(&dir, "b.json");
        fs::write(dir.join("c.txt"), "not json").unwrap();
        let files = find_json_files(&dir, false);
        assert_eq!(files.len(), 2);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_json_files_recursive() {
        let dir = make_temp_dir();
        write_skill_json(&dir, "a.json");
        let sub = dir.join("sub");
        fs::create_dir(&sub).unwrap();
        write_skill_json(&sub, "b.json");
        let files = find_json_files(&dir, true);
        assert_eq!(files.len(), 2);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_initial_populates_known() {
        let dir = make_temp_dir();
        write_skill_json(&dir, "a.json");
        write_skill_json(&dir, "b.json");
        let mut w = SkillWatcher::new(&dir);
        let count = w.scan_initial().unwrap();
        assert_eq!(count, 2);
        assert_eq!(w.known_count(), 2);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn check_for_changes_returns_added_on_new_file() {
        let dir = make_temp_dir();
        write_skill_json(&dir, "a.json");
        let mut w = SkillWatcher::new(&dir);
        w.scan_initial().unwrap();
        write_skill_json(&dir, "b.json");
        let events = w.check_for_changes();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], WatchEvent::Added(_)));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn check_for_changes_returns_removed_on_delete() {
        let dir = make_temp_dir();
        let p1 = write_skill_json(&dir, "a.json");
        let p2 = write_skill_json(&dir, "b.json");
        let mut w = SkillWatcher::new(&dir);
        w.scan_initial().unwrap();
        fs::remove_file(&p1).unwrap();
        fs::remove_file(&p2).unwrap();
        let events = w.check_for_changes();
        assert_eq!(events.len(), 2);
        for e in &events {
            assert!(matches!(e, WatchEvent::Removed(_)));
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn check_for_changes_returns_modified_on_mtime_change() {
        let dir = make_temp_dir();
        let p = write_skill_json(&dir, "a.json");
        let mut w = SkillWatcher::new(&dir);
        w.scan_initial().unwrap();
        touch_to_newer_mtime(&p);
        let events = w.check_for_changes();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], WatchEvent::Modified(_)));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn check_for_changes_returns_empty_when_no_changes() {
        let dir = make_temp_dir();
        write_skill_json(&dir, "a.json");
        let mut w = SkillWatcher::new(&dir);
        w.scan_initial().unwrap();
        let events = w.check_for_changes();
        assert!(events.is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scan_once_emits_added_for_existing_files() {
        let dir = make_temp_dir();
        write_skill_json(&dir, "a.json");
        let mut w = SkillWatcher::new(&dir);
        let events = w.scan_once();
        // scan_once = clear + check, all existing files show as Added
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], WatchEvent::Added(_)));
        assert_eq!(w.known_count(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn check_for_changes_handles_missing_root_dir() {
        let dir = std::env::temp_dir().join(format!(
            "apeireth-skills-watcher-missing-{}",
            std::time::SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut w = SkillWatcher::new(&dir);
        let events = w.check_for_changes();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], WatchEvent::ScanError(_)));
    }

    #[test]
    fn debug_impl_works() {
        let w = SkillWatcher::new("/tmp");
        let s = format!("{:?}", w);
        assert!(s.contains("SkillWatcher"));
        assert!(s.contains("known_count"));
    }

}
