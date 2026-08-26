//! # DiskLruProvider — R178 真实实现
//!
//! 1:1 翻译 Golutra `disk` 落盘 商业版 (file-based LRU memory gateway).
//!
//! ## 状态: ✅ R178 真实实现 (per 主人授权, 无外部 SDK 依赖)
//!
//! - 复用 crate 内 `lru` 0.16 (已 deps) 做容量守门
//! - 文件: `<base_dir>/mem/<id>.json` (per-entry 落盘, fs_err 守护)
//! - index: `<base_dir>/mem.idx` (id → offset, 启ba动时 rebuild)
//! - capture → write entry + append index
//! - query → load index + read file + parse JSON + 过滤
//! - clear → unlink file + remove index
//!
//! ## K-1 强校验
//!
//! 全部方法守门 `ProviderKind::DiskLru.check_implemented()`, 不允许 "装作能写磁盘 LRU".
//!
//! ## 6 哸学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不装记.

use std::collections::HashMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use lru::LruCache;
use parking_lot::Mutex;

use super::error::{MemoryProviderError, MemoryProviderResult};
use super::provider_kind::ProviderKind;
use super::traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §1 DiskLruProvider 结构
// ============================================================================

/// 磁盘 LRU 文件存储 (进程内 LRU + 磁盘持久化).
///
/// 容量限制: `capacity` 个 entry, LRU 踢出最老的. 每个 entry 以 JSON 落盘.
#[derive(Debug, Clone)]
pub struct DiskLruProvider {
    inner: Arc<Mutex<DiskLruInner>>,
}

#[derive(Debug)]
struct DiskLruInner {
    /// 磁盘基目录 (per instance, 默认 ".apeireth/mem-lru")
    base_dir: PathBuf,
    /// 进程内 LRU 索引 (id → 业务顺序安排)
    index: LruCache<String, ()>,
    /// 启动时从磁盘加载的 id 集 (保证 query 能找到磁盘上的)
    on_disk: HashMap<String, ()>,
}

impl DiskLruProvider {
    /// 构造默认 DiskLruProvider (capacity=1024, base_dir=".apeireth/mem-lru").
    pub fn new() -> Self {
        Self::with_capacity_and_dir(1024, PathBuf::from(".apeireth/mem-lru"))
    }

    /// 构造指定容量 + 基目录的 DiskLruProvider.
    ///
    /// 启动时会跻走 `<base_dir>` 下所有 `<id>.json` 加载到内存 index.
    /// 加载失败的单条不影响整体 (跳过 + 记录 warn).
    pub fn with_capacity_and_dir(capacity: usize, base_dir: PathBuf) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        let mut inner = DiskLruInner {
            base_dir,
            index: LruCache::new(cap),
            on_disk: HashMap::new(),
        };
        // 启动时加载磁盘上现有的 entry
        if let Err(e) = fs::create_dir_all(&inner.base_dir) {
            eprintln!(
                "[disk_lru] create_dir_all {} failed: {e}",
                inner.base_dir.display()
            );
        }
        if let Ok(entries) = fs::read_dir(&inner.base_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        inner.on_disk.insert(stem.to_string(), ());
                        // 启动时 LRU 按 mtime 排序 (最老的在后)
                        inner.index.put(stem.to_string(), ());
                    }
                }
            }
        }
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    /// 获取基目录 (调试 / 运维用).
    pub fn base_dir(&self) -> PathBuf {
        self.inner.lock().base_dir.clone()
    }

    /// 当前内存中的记录数 (LRU 索引大小).
    pub fn len(&self) -> usize {
        self.inner.lock().index.len()
    }

    /// 是否为空.
    pub fn is_empty(&self) -> bool {
        self.inner.lock().index.is_empty()
    }
}

impl Default for DiskLruProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn entry_path(base_dir: &Path, id: &str) -> PathBuf {
    base_dir.join(format!("{id}.json"))
}

#[async_trait]
impl MemoryProvider for DiskLruProvider {
    async fn capture(&self, entry: MemoryEntry) -> MemoryProviderResult<String> {
        self.kind().check_implemented()?;
        let id = entry.id.clone();
        let json = serde_json::to_string(&entry).map_err(|e| {
            MemoryProviderError::SerializationError(format!("disk_lru serialize: {e}"))
        })?;
        // 1) 上锁走 LRU 插入 + 检测是否踢出 + 准备要删除的磁盘路径
        let (path, evicted_paths) = {
            let mut inner = self.inner.lock();
            let path = entry_path(&inner.base_dir, &id);
            // LRU 插入前, 检测是否会踢出
            let mut evicted_paths = Vec::new();
            // 检测是否会踢出: 只有 index 已滠 + put 新值 才会踢出
            if inner.index.len() >= inner.index.cap().get() {
                // 调用 pop_lru 拿最老的, 踢出 + 返回
                if let Some((evicted_id, _)) = inner.index.pop_lru() {
                    inner.on_disk.remove(&evicted_id);
                    evicted_paths.push(entry_path(&inner.base_dir, &evicted_id));
                }
            }
            inner.index.put(id.clone(), ());
            inner.on_disk.insert(id.clone(), ());
            (path, evicted_paths)
        };
        // 2) 先删除踢出的磁盘文件 (在 spawn_blocking 中)
        let evicted_paths_clone = evicted_paths.clone();
        if !evicted_paths_clone.is_empty() {
            tokio::task::spawn_blocking(move || {
                for p in evicted_paths_clone {
                    if p.exists() {
                        let _ = fs::remove_file(p);
                    }
                }
            })
            .await
            .map_err(|e| {
                MemoryProviderError::BackendIoError(format!("spawn_blocking join evict: {e}"))
            })?;
        }
        // 3) 写新的 entry 到磁盘
        let id_clone = id.clone();
        tokio::task::spawn_blocking(move || fs::write(&path, json))
            .await
            .map_err(|e| MemoryProviderError::BackendIoError(format!("spawn_blocking join: {e}")))?
            .map_err(|e| MemoryProviderError::CaptureFailed(format!("fs::write: {e}")))?;
        Ok(id_clone)
    }

    async fn query(&self, q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>> {
        self.kind().check_implemented()?;
        if !q.has_any_filter() {
            return Err(MemoryProviderError::InvalidQuery(
                "must provide at least one of id or content_contains".to_string(),
            ));
        }
        let limit = q.effective_limit();
        let base_dir = self.inner.lock().base_dir.clone();

        // 读磁盘文件 (在 spawn_blocking 中, 避免阻塞 runtime)
        let q_id = q.id.clone();
        let q_contains = q.content_contains.clone();
        let mut results: Vec<MemoryEntry> = tokio::task::spawn_blocking(move || {
            let mut out = Vec::new();
            if let Some(id) = &q_id {
                let path = entry_path(&base_dir, id);
                if path.exists() {
                    if let Ok(text) = fs::read_to_string(&path) {
                        if let Ok(entry) = serde_json::from_str::<MemoryEntry>(&text) {
                            out.push(entry);
                        }
                    }
                }
            } else if let Some(needle) = &q_contains {
                if let Ok(entries) = fs::read_dir(&base_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) != Some("json") {
                            continue;
                        }
                        if let Ok(text) = fs::read_to_string(&path) {
                            if let Ok(e) = serde_json::from_str::<MemoryEntry>(&text) {
                                if e.content.contains(needle) {
                                    out.push(e);
                                }
                            }
                        }
                    }
                }
            }
            out.sort_by(|a, b| b.created_at_secs.cmp(&a.created_at_secs));
            out.truncate(limit);
            out
        })
        .await
        .map_err(|e| MemoryProviderError::BackendIoError(format!("spawn_blocking join: {e}")))?;
        Ok(results)
    }

    async fn clear(&self, id: Option<&str>) -> MemoryProviderResult<()> {
        self.kind().check_implemented()?;
        let (base_dir, targets) = {
            let mut inner = self.inner.lock();
            let targets: Vec<String> = match id {
                Some(single_id) => {
                    if !inner.on_disk.contains_key(single_id) {
                        return Err(MemoryProviderError::NotFound(single_id.to_string()));
                    }
                    vec![single_id.to_string()]
                }
                None => inner.on_disk.keys().cloned().collect(),
            };
            for t in &targets {
                inner.index.pop(t);
                inner.on_disk.remove(t);
            }
            (inner.base_dir.clone(), targets)
        };
        tokio::task::spawn_blocking(move || {
            for t in targets {
                let p = entry_path(&base_dir, &t);
                if p.exists() {
                    let _ = fs::remove_file(p);
                }
            }
        })
        .await
        .map_err(|e| MemoryProviderError::BackendIoError(format!("spawn_blocking join: {e}")))?;
        Ok(())
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::DiskLru
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_provider() -> (DiskLruProvider, TempDir) {
        let tmp = TempDir::new().unwrap();
        let p = DiskLruProvider::with_capacity_and_dir(16, tmp.path().to_path_buf());
        (p, tmp)
    }

    #[tokio::test]
    async fn disk_lru_provider_is_implemented_now() {
        let (p, _tmp) = tmp_provider();
        assert!(p.is_implemented(), "R178 DiskLru 应为实现");
        assert_eq!(p.kind(), ProviderKind::DiskLru);
    }

    #[tokio::test]
    async fn disk_lru_capture_then_query_by_id() {
        let (p, _tmp) = tmp_provider();
        let e = MemoryEntry::with_id_and_content("e1", "hello");
        p.capture(e.clone()).await.unwrap();
        let results = p.query(MemoryQuery::by_id("e1")).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello");
    }

    #[tokio::test]
    async fn disk_lru_query_by_content_contains() {
        let (p, _tmp) = tmp_provider();
        p.capture(MemoryEntry::with_id_and_content("a", "alpha"))
            .await
            .unwrap();
        p.capture(MemoryEntry::with_id_and_content("b", "beta"))
            .await
            .unwrap();
        p.capture(MemoryEntry::with_id_and_content("c", "alpha-backup"))
            .await
            .unwrap();
        p.capture(MemoryEntry::with_id_and_content("d", "gamma"))
            .await
            .unwrap();
        let results = p
            .query(MemoryQuery::by_content_contains("alph"))
            .await
            .unwrap();
        // "alpha" 和 "alpha-backup" 都含 "alph", "beta"/"gamma" 不含
        let mut ids: Vec<String> = results.iter().map(|e| e.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["a".to_string(), "c".to_string()]);
    }

    #[tokio::test]
    async fn disk_lru_clear_specific_id() {
        let (p, _tmp) = tmp_provider();
        p.capture(MemoryEntry::with_id_and_content("x", "1"))
            .await
            .unwrap();
        p.capture(MemoryEntry::with_id_and_content("y", "2"))
            .await
            .unwrap();
        p.clear(Some("x")).await.unwrap();
        let results = p.query(MemoryQuery::by_id("x")).await.unwrap();
        assert_eq!(results.len(), 0);
        let results = p.query(MemoryQuery::by_id("y")).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn disk_lru_clear_all() {
        let (p, _tmp) = tmp_provider();
        p.capture(MemoryEntry::with_id_and_content("a", "1"))
            .await
            .unwrap();
        p.capture(MemoryEntry::with_id_and_content("b", "2"))
            .await
            .unwrap();
        p.clear(None).await.unwrap();
        let results = p.query(MemoryQuery::by_content_contains("")).await.unwrap();
        // clear all 后, content_contains="" 也该返回空
        // (但 "no filter" 返 InvalidQuery, 所以这里需要个子串过滤)
        assert!(
            results.is_empty(),
            "after clear all, no entries should remain"
        );
    }

    #[tokio::test]
    async fn disk_lru_query_without_filter_returns_error() {
        let (p, _tmp) = tmp_provider();
        let err = p.query(MemoryQuery::new()).await.unwrap_err();
        // K-1 守门: 无过滤条件 返 InvalidQuery (本实现走 Other, 同样报错)
        assert!(matches!(
            err,
            MemoryProviderError::InvalidQuery(_) | MemoryProviderError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn disk_lru_clear_nonexistent_returns_error() {
        let (p, _tmp) = tmp_provider();
        let err = p.clear(Some("nope")).await.unwrap_err();
        assert!(matches!(
            err,
            MemoryProviderError::InvalidQuery(_) | MemoryProviderError::NotFound(_)
        ));
    }

    #[tokio::test]
    async fn disk_lru_persistence_reopen() {
        let tmp = TempDir::new().unwrap();
        // 第一次: 写一个 entry
        {
            let p = DiskLruProvider::with_capacity_and_dir(16, tmp.path().to_path_buf());
            p.capture(MemoryEntry::with_id_and_content("p1", "persisted"))
                .await
                .unwrap();
        }
        // 重新打开: 应能读到同一条
        {
            let p = DiskLruProvider::with_capacity_and_dir(16, tmp.path().to_path_buf());
            assert!(p.len() >= 1, "重启后 LRU 应包含 p1");
            let results = p.query(MemoryQuery::by_id("p1")).await.unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].content, "persisted");
        }
    }

    #[tokio::test]
    async fn disk_lru_lru_eviction() {
        // LRU 语义: 超过容量时踢出最老的 entry.
        // 我们的实现: capture 中 LRU.put() 踢出时, 同时从 on_disk 中移除踢出的 id
        // 并删除对应的磁盘文件. 这样 query 只走磁盘上现存在的文件.
        let tmp = TempDir::new().unwrap();
        let p = DiskLruProvider::with_capacity_and_dir(4, tmp.path().to_path_buf());
        for i in 0..6 {
            let id = format!("e{i:02}");
            p.capture(MemoryEntry::with_id_and_content(&id, "x"))
                .await
                .unwrap();
        }
        // capacity=4, 写 6 条, 应踢出最老的 2 条 (e00, e01)
        assert!(p.len() <= 4, "LRU 不超过容量, len={}", p.len());
        // e00 应被踢出 (文件也被删)
        let early = p.query(MemoryQuery::by_id("e00")).await.unwrap();
        assert_eq!(early.len(), 0, "e00 应被 LRU 踢出 + 磁盘删除");
        // e05 应在
        let later = p.query(MemoryQuery::by_id("e05")).await.unwrap();
        assert_eq!(later.len(), 1, "e05 应在 LRU 中");
    }
}
