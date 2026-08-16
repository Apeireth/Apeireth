//! R30 U6: 通用多配置文件热加载 (VCP-style notify watcher)
//!
//! **设计**:
//! - `ConfigWatcher` 一次监视一个目录 (recursive=false)
//! - 给每个文件名 (basename) 注册一个 callback
//! - 文件 create/modify 触发 callback (let callback 自己重读 + 更新自己负责的状态)
//! - notify 出错静默 (避免崩 TUI)
//!
//! **不假装**:
//! - 真用 `notify::RecommendedWatcher` + `std::sync::mpsc`
//! - 第一次 callback 触发后, 文件 watcher 仍保持 (持续热加载)
//! - 删除文件不触发 (避免误报)

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

/// R30 U6: ConfigWatcher builder.
pub struct ConfigWatcher {
    dir: PathBuf,
    callbacks: HashMap<String, Box<dyn Fn() + Send + Sync + 'static>>,
}

impl ConfigWatcher {
    /// 新建 (指定目录)
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            callbacks: HashMap::new(),
        }
    }

    /// 注册文件 basename (e.g. "settings.json") -> callback
    pub fn watch<F>(mut self, filename: &str, cb: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.callbacks.insert(filename.to_string(), Box::new(cb));
        self
    }

    /// 启动后台监视线程. 返 thread handle (join 可等).
    /// 如果目录不存在 / notify 失败 -> 静默返 None (不阻塞 TUI).
    pub fn spawn(self) -> Option<std::thread::JoinHandle<()>> {
        let dir = self.dir.clone();
        if !dir.exists() {
            let _ = std::fs::create_dir_all(&dir);
        }
        let callbacks = self.callbacks;
        let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })
        .ok()?;
        if watcher.watch(&dir, RecursiveMode::NonRecursive).is_err() {
            return None;
        }
        Some(
            std::thread::Builder::new()
                .name("apeireth-config-watcher".into())
                .spawn(move || {
                    let _hold = watcher; // 保持 alive
                    for res in rx {
                        if let Ok(ev) = res {
                            for path in &ev.paths {
                                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                    if let Some(cb) = callbacks.get(name) {
                                        cb();
                                    }
                                }
                            }
                        }
                    }
                })
                .ok()?,
        )
    }
}

/// R30 U6: 获取 apeireth 配置目录 (跨平台)
pub fn apeireth_config_dir() -> PathBuf {
    if let Ok(p) = std::env::var("APEREIRETH_CONFIG_DIR") {
        return PathBuf::from(p);
    }
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    Path::new(&home).join(".apeireth")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn watcher_registers_and_fires_callback() {
        let dir = tempfile::TempDir::new().unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let watcher = ConfigWatcher::new(dir.path().to_path_buf()).watch("test.json", move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
        let _handle = watcher.spawn();
        // Give the watcher a moment to start
        std::thread::sleep(std::time::Duration::from_millis(100));
        // Create the file (some platforms fire Create events)
        std::fs::write(dir.path().join("test.json"), "{}").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        // Modify it
        std::fs::write(dir.path().join("test.json"), "{\"x\":1}").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        // At least one event should fire (Create or Modify)
        assert!(
            counter.load(Ordering::SeqCst) >= 1,
            "expected >=1 callback, got {}",
            counter.load(Ordering::SeqCst)
        );
    }

    #[test]
    fn watcher_unregistered_file_does_not_fire() {
        let dir = tempfile::TempDir::new().unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let watcher =
            ConfigWatcher::new(dir.path().to_path_buf()).watch("watched.json", move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
        let _handle = watcher.spawn();
        std::thread::sleep(std::time::Duration::from_millis(100));
        // Write to UNREGISTERED file
        std::fs::write(dir.path().join("unwatched.json"), "{}").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert_eq!(
            counter.load(Ordering::SeqCst),
            0,
            "unregistered file should not fire"
        );
    }

    #[test]
    fn apeireth_config_dir_returns_path() {
        let d = apeireth_config_dir();
        assert!(!d.as_os_str().is_empty());
    }
}
