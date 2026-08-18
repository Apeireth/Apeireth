//! fsnotify watcher (notify 6.x).

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum WatchError {
    #[error("notify error: `{0}`")]
    Notify(#[from] notify::Error),
    #[error("watcher closed unexpectedly")]
    WatcherClosed,
}

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Other,
}

impl WatchEvent {
    pub fn from_notify(event: &Event) -> Self {
        let p = event.paths.first().cloned().unwrap_or_default();
        match event.kind {
            EventKind::Create(_) => WatchEvent::Created(p),
            EventKind::Modify(_) => WatchEvent::Modified(p),
            EventKind::Remove(_) => WatchEvent::Removed(p),
            _ => WatchEvent::Other,
        }
    }
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<WatchEvent>,
}

impl FileWatcher {
    pub fn new(path: &Path, recursive: bool) -> Result<Self, WatchError> {
        let (tx, rx) = mpsc::channel(64);
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(WatchEvent::from_notify(&event));
            }
        })?;
        watcher.watch(path, mode)?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    pub async fn next_event(&mut self) -> Option<WatchEvent> {
        self.rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn watcher_can_be_created() {
        let tmp = tempfile::tempdir().unwrap();
        let watcher = FileWatcher::new(tmp.path(), false);
        assert!(watcher.is_ok());
    }
}
