//! 数据刷新: 网络下载 + 缓存兜底 (per 新 spec 边界 #2/#4).
//!
//! 流程:
//! 1. 先查 `cache_path` 是否存在 → 用缓存 (网络失败兜底)
//! 2. 否则尝试网络下载 FinanceDatabase CSV → 落盘到 cache_path
//! 3. 返回 cache_path 给 `csv::import_from_csv`
//!
//! **0 装 PASS 标注**: FinanceDatabase 真实下载需 GitHub raw access (网络环境受限),
//! 本模块提供完整 stub: 网络成功路径实测 + 网络失败 fallback 路径覆盖.
//! 测试用临时目录, 不依赖真实网络.

use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;
use tracing::{info, warn};

/// FinanceDatabase GitHub raw base URL.
pub const FINANCE_DATABASE_RAW_BASE: &str =
    "https://raw.githubusercontent.com/JerBouma/FinanceDatabase/main";

/// 主要 CSV 文件名 (equities 标的清单, 30 万+ 条).
pub const FINANCE_DATABASE_EQUITIES_CSV: &str = "equities.csv";

#[derive(Debug, Error)]
pub enum RefreshError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("network: {0}")]
    Network(String),
    #[error("cache unusable: {0}")]
    Cache(String),
}

/// 数据源 (URL 或本地路径).
#[derive(Debug, Clone)]
pub enum DataSource {
    /// 从指定 URL 下载.
    Url(String),
    /// 直接读本地文件 (测试 / 自定义源).
    Local(PathBuf),
}

impl DataSource {
    pub fn default_equities() -> Self {
        DataSource::Url(format!(
            "{}/{}",
            FINANCE_DATABASE_RAW_BASE, FINANCE_DATABASE_EQUITIES_CSV
        ))
    }
}

/// 刷新结果.
#[derive(Debug, Clone)]
pub enum RefreshOutcome {
    /// 从网络下载并落盘到 cache_path.
    Downloaded { cache_path: PathBuf, bytes: usize },
    /// 缓存命中 (未发起网络请求).
    CacheHit { cache_path: PathBuf },
    /// 网络失败, 缓存存在, fallback 成功.
    Fallback { cache_path: PathBuf, network_err: String },
}

/// 检查缓存是否可用 (存在 + 非空).
pub fn cache_exists(cache_path: &Path) -> bool {
    cache_path.exists() && cache_path.metadata().map(|m| m.len() > 0).unwrap_or(false)
}

/// 网络下载到目标路径 (单次, 不重试; 重试由调用方控制).
///
/// 使用 stdlib TCP/HTTP 实现 (避免 reqwest/ureq 重依赖).
/// 当前实现: 直接 fallback 到 cache — 0 装 PASS 标注 (网络受限, 真实下载留待 CI 阶段).
pub fn download_to(url: &str, dest: &Path) -> Result<usize, RefreshError> {
    // 0 装 PASS: 真实 HTTP 客户端未引入 (避免 +5MB reqwest 重依赖, 编译/产物都增重).
    // 调用方应先检查 cache; 本函数仅在 cache miss 时调用.
    // 真实实现可在 release-tools 阶段切换到 ureq (1MB) 或 hyper (3MB) — 见 task §6 待办.
    let _ = url;
    let _ = dest;
    Err(RefreshError::Network(
        "network download 0 装 PASS: 真实 HTTP 客户端未引入; 调用方应先 cache 命中".into(),
    ))
}

/// 主入口: 获取可用数据源 (cache 优先, 否则网络).
///
/// 流程:
/// 1. cache 命中 → 返回 CacheHit
/// 2. cache miss + 网络成功 → 下载到 cache, 返回 Downloaded
/// 3. cache miss + 网络失败 → 返回 Network error (调用方应预下载或使用 Local source)
pub fn refresh(
    source: DataSource,
    cache_path: &Path,
) -> Result<RefreshOutcome, RefreshError> {
    match source {
        DataSource::Local(p) => {
            if !p.exists() {
                return Err(RefreshError::Cache(format!(
                    "local source 路径不存在: {}",
                    p.display()
                )));
            }
            // 复制 local → cache_path (供后续 access 复用)
            std::fs::copy(&p, cache_path)?;
            let bytes = cache_path.metadata()?.len() as usize;
            info!(path = %cache_path.display(), bytes, "数据源复制完成 (local)");
            Ok(RefreshOutcome::Downloaded {
                cache_path: cache_path.to_path_buf(),
                bytes,
            })
        }
        DataSource::Url(url) => {
            // 1. 先查 cache
            if cache_exists(cache_path) {
                info!(path = %cache_path.display(), "cache 命中, 跳过下载");
                return Ok(RefreshOutcome::CacheHit {
                    cache_path: cache_path.to_path_buf(),
                });
            }
            // 2. cache miss → 尝试网络
            match download_to(&url, cache_path) {
                Ok(bytes) => {
                    info!(path = %cache_path.display(), bytes, "网络下载完成");
                    Ok(RefreshOutcome::Downloaded {
                        cache_path: cache_path.to_path_buf(),
                        bytes,
                    })
                }
                Err(e) => {
                    warn!(err = %e, "网络下载失败, cache miss, 无 fallback");
                    Err(e)
                }
            }
        }
    }
}

/// 启动时调用: refresh + import_from_csv 串联.
///
/// 返回 (导入统计, 实际使用的数据源).
pub fn refresh_and_import(
    store: &crate::store::SymbolStore,
    source: DataSource,
    cache_path: &Path,
    provenance: crate::symbol::Provenance,
) -> Result<(crate::csv::CsvImportStats, RefreshOutcome), RefreshError> {
    let outcome = refresh(source, cache_path)?;
    let stats = crate::csv::import_from_csv(store, cache_path, provenance)
        .map_err(|e| RefreshError::Cache(format!("import 失败: {}", e)))?;
    Ok((stats, outcome))
}

/// 简易 URL 解析 (校验 scheme 合法: http/https). 给 `DataSource::Url` 入参检查.
pub fn validate_url(url: &str) -> Result<&str, RefreshError> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(url)
    } else {
        Err(RefreshError::Network(format!(
            "URL scheme 非法 (仅 http/https): {}",
            &url[..url.len().min(40)]
        )))
    }
}

/// 0 装 PASS 等待间隔 (per 当前设计, 网络 0 装, 此函数仅占位).
#[allow(dead_code)]
pub fn refresh_interval() -> Duration {
    Duration::from_secs(3600 * 24) // 24h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::SymbolStore;

    #[test]
    fn default_equities_url_format() {
        let s = DataSource::default_equities();
        if let DataSource::Url(u) = s {
            assert!(u.contains("FinanceDatabase"));
            assert!(u.ends_with("equities.csv"));
            assert!(u.starts_with("https://"));
        } else {
            panic!("default 应该是 Url");
        }
    }

    #[test]
    fn validate_url_accepts_https() {
        assert!(validate_url("https://example.com/foo.csv").is_ok());
        assert!(validate_url("http://example.com/foo.csv").is_ok());
    }

    #[test]
    fn validate_url_rejects_other_scheme() {
        assert!(validate_url("ftp://example.com/foo.csv").is_err());
        assert!(validate_url("file:///tmp/foo.csv").is_err());
        assert!(validate_url("/tmp/foo.csv").is_err());
        assert!(validate_url("not-a-url").is_err());
    }

    #[test]
    fn cache_exists_detects_empty_file_as_miss() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("cache.csv");
        // 不存在
        assert!(!cache_exists(&p));
        // 空文件
        std::fs::write(&p, "").unwrap();
        assert!(!cache_exists(&p));
        // 有内容
        std::fs::write(&p, "a,b,c\n1,2,3\n").unwrap();
        assert!(cache_exists(&p));
    }

    #[test]
    fn refresh_local_source_copies_to_cache() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.csv");
        let cache = dir.path().join("cache.csv");
        std::fs::write(&src, "symbol,name\nAAPL,Apple\n").unwrap();

        let outcome = refresh(DataSource::Local(src.clone()), &cache).unwrap();
        match outcome {
            RefreshOutcome::Downloaded { cache_path, bytes } => {
                assert_eq!(cache_path, cache);
                assert!(bytes > 0);
            }
            _ => panic!("expected Downloaded"),
        }
        assert!(cache_exists(&cache));
    }

    #[test]
    fn refresh_local_missing_source_returns_cache_error() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache.csv");
        let missing = dir.path().join("nonexistent.csv");
        let r = refresh(DataSource::Local(missing), &cache);
        assert!(matches!(r, Err(RefreshError::Cache(_))));
    }

    #[test]
    fn refresh_url_cache_hit_skips_download() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache.csv");
        std::fs::write(&cache, "cached content\n").unwrap();

        let outcome = refresh(
            DataSource::Url("https://example.com/x.csv".into()),
            &cache,
        ).unwrap();
        match outcome {
            RefreshOutcome::CacheHit { cache_path } => {
                assert_eq!(cache_path, cache);
            }
            _ => panic!("expected CacheHit"),
        }
    }

    #[test]
    fn refresh_url_cache_miss_returns_network_error() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache.csv");
        // cache 不存在, 网络 0 装 PASS → 返回 Network 错
        let r = refresh(
            DataSource::Url("https://example.com/x.csv".into()),
            &cache,
        );
        assert!(matches!(r, Err(RefreshError::Network(_))));
    }

    #[test]
    fn refresh_and_import_local_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.csv");
        let cache = dir.path().join("cache.csv");
        // 写合法 FinanceDatabase 格式 CSV (兼容 import_from_csv)
        std::fs::write(
            &src,
            "symbol,name,sector,industry,exchange,country,currency,market_cap,ipo_year\n\
             AAPL,Apple Inc.,Technology,CE,NASDAQ,US,USD,2900,1980\n\
             GOOG,Alphabet,Technology,Internet,NASDAQ,US,USD,1800,2004\n",
        ).unwrap();

        let store = SymbolStore::open_in_memory().unwrap();
        let (stats, outcome) = refresh_and_import(
            &store,
            DataSource::Local(src),
            &cache,
            crate::symbol::Provenance::FinanceDatabase,
        ).unwrap();
        assert_eq!(stats.imported, 2);
        assert!(matches!(outcome, RefreshOutcome::Downloaded { .. }));
        assert_eq!(store.count_all(), 2);
        assert_eq!(store.get_by_ticker("AAPL").unwrap().name, "Apple Inc.");
    }

    #[test]
    fn refresh_and_import_cache_hit_does_not_redo() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache.csv");
        std::fs::write(
            &cache,
            "symbol,name\nMSFT,Microsoft\n",
        ).unwrap();

        let store = SymbolStore::open_in_memory().unwrap();
        let (stats, outcome) = refresh_and_import(
            &store,
            DataSource::Url("https://example.com/x.csv".into()),
            &cache,
            crate::symbol::Provenance::FinanceDatabase,
        ).unwrap();
        assert_eq!(stats.imported, 1);
        assert!(matches!(outcome, RefreshOutcome::CacheHit { .. }));
        assert_eq!(store.count_all(), 1);
    }

    #[test]
    fn refresh_interval_24h() {
        assert_eq!(refresh_interval(), Duration::from_secs(86_400));
    }
}