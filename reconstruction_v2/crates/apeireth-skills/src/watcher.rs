//! File watcher for hot reload.
use std::path::{Path, PathBuf};
pub struct SkillWatcher { pub paths: Vec<PathBuf> }

impl SkillWatcher {
    pub fn new() -> Self { Self { paths: Vec::new() } }
    pub fn watch(&mut self, path: &Path) { self.paths.push(path.to_path_buf()); }
    pub fn poll(&self) -> Vec<PathBuf> { self.paths.clone() }
}

impl Default for SkillWatcher {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watch_and_poll() {
        let mut w = SkillWatcher::new();
        w.watch(Path::new("."));
        assert_eq!(w.poll().len(), 1);
    }
}
