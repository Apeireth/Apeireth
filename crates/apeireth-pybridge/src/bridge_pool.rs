//! 跨语言桥模块缓存池 (R127-2 Stage 6.1 跨语言桥深化)
//!
//! 借鉴 hyper 80 `pool_max_idle_per_host` 池复用模式 (per `borrowed-repos/hyper/hyper-util/src/client/pool.rs`)
//! + LIFO 策略 (LIFO 复用率更高, 延迟更低, per hyper-util 文档).
//!
//! 默认 build 0 体积 — 占位 stub 仅暴露 stats/clear, 完整 import 缓存仅在 python-ext 启用时编译.
//!
//! 借鉴 ID: `R127-2-BORROW-hyperium/hyper-stage-6-1-pool-2026-08-10` (新 ID, 不跟 R125-3 冲突).

#[cfg(feature = "python-ext")]
use std::collections::HashMap;
use std::sync::Mutex;
#[cfg(feature = "python-ext")]
use std::time::Instant;

/// 池配置 (借鉴 hyper PoolConfig)
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 每 host 最大空闲连接 (借鉴 hyper `pool_max_idle_per_host`)
    pub max_idle: usize,
    /// 空闲超时 (秒) — 超时后丢弃
    pub idle_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_idle: 32,
            idle_timeout_secs: 90,
        }
    }
}

/// 池统计 (借鉴 hyper PoolStats)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolStats {
    pub cached_modules: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl PoolStats {
    /// 缓存命中率 (0.0 ~ 1.0)
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// 模块缓存项
#[cfg(feature = "python-ext")]
struct CachedModule {
    module: pyo3::Py<pyo3::types::PyModule>,
    last_used_secs: u64,
}

#[cfg(feature = "python-ext")]
type ModuleCache = HashMap<String, CachedModule>;

/// 跨语言桥模块缓存池 (cfg-gated 完整实现)
///
/// 借鉴 hyper 80 连接池:
/// - LIFO 策略 (复用最近模块, 减少 GIL 重入)
/// - 空闲超时 (避免长期未用模块堆积)
/// - LRU eviction (按 last_used_secs 排序, 超过 max_idle 移除最旧)
#[cfg(feature = "python-ext")]
pub struct BridgeModulePool {
    config: PoolConfig,
    cache: Mutex<ModuleCache>,
    stats: Mutex<PoolStats>,
    start: Instant,
}

#[cfg(feature = "python-ext")]
impl BridgeModulePool {
    pub fn new() -> Self {
        Self::with_config(PoolConfig::default())
    }

    pub fn with_config(config: PoolConfig) -> Self {
        Self {
            config,
            cache: Mutex::new(HashMap::new()),
            stats: Mutex::new(PoolStats::default()),
            start: Instant::now(),
        }
    }

    /// 当前时间 (相对 pool start, 秒)
    fn now_secs(&self) -> u64 {
        self.start.elapsed().as_secs()
    }

    /// 配置
    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// 获取或导入模块 (借鉴 hyper 池 LIFO 复用)
    pub fn get_or_import<'py>(
        &self,
        py: pyo3::Python<'py>,
        module_name: &str,
    ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::types::PyModule>> {
        // 1) 查 cache + 清超时
        {
            let mut cache = self.cache.lock().expect("BridgeModulePool cache mutex");
            let now = self.now_secs();
            cache.retain(|_, v| now.saturating_sub(v.last_used_secs) < self.config.idle_timeout_secs);
            if let Some(cached) = cache.get_mut(module_name) {
                cached.last_used_secs = now;
                let mut stats = self.stats.lock().expect("stats mutex");
                stats.hits += 1;
                return Ok(cached.module.bind(py).clone());
            }
        }

        // 2) miss → import + 缓存 (import 失败则 misses 0, 错误传播给调用方)
        let module = py.import(module_name)?;
        // miss 计数在 import 成功后累加 (失败时 miss=0, 错误直接传播)
        {
            let mut stats = self.stats.lock().expect("stats mutex");
            stats.misses += 1;
        }
        let module_py: pyo3::Py<pyo3::types::PyModule> = module.unbind();
        {
            let mut cache = self.cache.lock().expect("BridgeModulePool cache mutex");
            // 超过 max_idle → LRU evict (移除 last_used_secs 最旧)
            while cache.len() >= self.config.max_idle {
                if let Some(oldest) = cache
                    .iter()
                    .min_by_key(|(_, v)| v.last_used_secs)
                    .map(|(k, _)| k.clone())
                {
                    cache.remove(&oldest);
                    let mut stats = self.stats.lock().expect("stats mutex");
                    stats.evictions += 1;
                } else {
                    break;
                }
            }
            let now = self.now_secs();
            cache.insert(
                module_name.to_string(),
                CachedModule {
                    module: module_py.clone_ref(py),
                    last_used_secs: now,
                },
            );
        }
        // 用 into_bound 消耗 module_py 转 Bound<PyModule> 返回
        Ok(module_py.into_bound(py))
    }

    /// 池统计快照
    pub fn stats(&self) -> PoolStats {
        let stats = self.stats.lock().expect("stats mutex").clone();
        let cache = self.cache.lock().expect("cache mutex");
        PoolStats {
            cached_modules: cache.len(),
            ..stats
        }
    }

    /// 清空 cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock().expect("cache mutex");
        cache.clear();
    }
}

#[cfg(feature = "python-ext")]
impl Default for BridgeModulePool {
    fn default() -> Self {
        Self::new()
    }
}

/// 默认 build 下的占位池 (0 体积, 全部方法走降级 stub)
#[cfg(not(feature = "python-ext"))]
pub struct BridgeModulePool {
    stats: Mutex<PoolStats>,
    config: PoolConfig,
}

#[cfg(not(feature = "python-ext"))]
impl BridgeModulePool {
    pub fn new() -> Self {
        Self::with_config(PoolConfig::default())
    }

    pub fn with_config(config: PoolConfig) -> Self {
        Self {
            stats: Mutex::new(PoolStats::default()),
            config,
        }
    }

    pub fn config(&self) -> &PoolConfig {
        &self.config
    }

    /// 默认 build: get_or_import 永远返回空 pool (借用 cfg-gated 双实现)
    /// 调用方应通过 bridge::is_module_available 走 ModuleNotFound 降级
    pub fn stats(&self) -> PoolStats {
        self.stats.lock().expect("stats mutex").clone()
    }

    pub fn clear(&self) {
        self.stats.lock().expect("stats mutex").cached_modules = 0;
    }
}

#[cfg(not(feature = "python-ext"))]
impl Default for BridgeModulePool {
    fn default() -> Self {
        Self::new()
    }
}

// 跨 build 通用测试 (cfg-无关, 跑在所有 build)
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_config_default() {
        let cfg = PoolConfig::default();
        assert_eq!(cfg.max_idle, 32);
        assert_eq!(cfg.idle_timeout_secs, 90);
    }

    #[test]
    fn pool_stats_hit_rate_zero_when_empty() {
        let s = PoolStats::default();
        assert_eq!(s.hit_rate(), 0.0);
    }

    #[test]
    fn pool_stats_hit_rate_compute() {
        let s = PoolStats {
            hits: 7,
            misses: 3,
            ..Default::default()
        };
        assert!((s.hit_rate() - 0.7).abs() < 1e-9);
    }

    #[test]
    fn pool_default_build_can_be_constructed() {
        // 默认 build 下也构造成功 (0 体积占位)
        let pool = BridgeModulePool::new();
        let s = pool.stats();
        assert_eq!(s.cached_modules, 0);
        assert_eq!(s.hits, 0);
    }

    #[test]
    fn pool_clear_default_build() {
        let pool = BridgeModulePool::new();
        pool.clear();
        let s = pool.stats();
        assert_eq!(s.cached_modules, 0);
    }

    #[test]
    fn pool_config_clone() {
        let cfg = PoolConfig {
            max_idle: 16,
            idle_timeout_secs: 60,
        };
        let cfg2 = cfg.clone();
        assert_eq!(cfg.max_idle, cfg2.max_idle);
        assert_eq!(cfg.idle_timeout_secs, cfg2.idle_timeout_secs);
    }

    // ============================================================
    // R127-2 Stage 6.1 新增 cfg-gated tests
    // ============================================================

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pool_first_import_misses_then_cached() {
        use pyo3::prelude::*;
        let pool = BridgeModulePool::new();
        Python::attach(|py| {
            // 第一次 import → miss
            let m1 = pool.get_or_import(py, "math").expect("import math");
            let s1 = pool.stats();
            assert_eq!(s1.misses, 1);
            assert_eq!(s1.hits, 0);
            assert_eq!(s1.cached_modules, 1);
            drop(m1);

            // 第二次 → hit
            let m2 = pool.get_or_import(py, "math").expect("cached math");
            let s2 = pool.stats();
            assert_eq!(s2.misses, 1);
            assert_eq!(s2.hits, 1);
            assert_eq!(s2.cached_modules, 1);
            assert!(s2.hit_rate() > 0.0);
            drop(m2);
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pool_distinct_modules_cached_separately() {
        use pyo3::prelude::*;
        let pool = BridgeModulePool::new();
        Python::attach(|py| {
            pool.get_or_import(py, "math").unwrap();
            pool.get_or_import(py, "json").unwrap();
            let s = pool.stats();
            assert_eq!(s.misses, 2);
            assert_eq!(s.hits, 0);
            assert_eq!(s.cached_modules, 2);

            // 重复 import 命中
            pool.get_or_import(py, "math").unwrap();
            pool.get_or_import(py, "json").unwrap();
            let s2 = pool.stats();
            assert_eq!(s2.misses, 2);
            assert_eq!(s2.hits, 2);
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pool_invalid_module_errors() {
        use pyo3::prelude::*;
        let pool = BridgeModulePool::new();
        Python::attach(|py| {
            let r = pool.get_or_import(py, "not.a.real.module.zzz");
            assert!(r.is_err());
            let s = pool.stats();
            // import 失败 → miss=0 (借 miss 计 import 成功后)
            assert_eq!(s.misses, 0);
            assert_eq!(s.cached_modules, 0);
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pool_clear_empties_cache() {
        use pyo3::prelude::*;
        let pool = BridgeModulePool::new();
        Python::attach(|py| {
            pool.get_or_import(py, "math").unwrap();
            pool.get_or_import(py, "json").unwrap();
            assert_eq!(pool.stats().cached_modules, 2);

            pool.clear();
            let s = pool.stats();
            assert_eq!(s.cached_modules, 0);
            // hits/misses 累计保留 (clear 不重置统计)
        });
    }

    #[cfg(feature = "python-ext")]
    #[test]
    fn r127_2_pool_lru_eviction() {
        use pyo3::prelude::*;
        // max_idle=2, 装 3 个模块, 触发 eviction
        let pool = BridgeModulePool::with_config(PoolConfig {
            max_idle: 2,
            idle_timeout_secs: 90,
        });
        Python::attach(|py| {
            pool.get_or_import(py, "math").unwrap();
            pool.get_or_import(py, "json").unwrap();
            // 等 1s 让 math 的 last_used 更早
            std::thread::sleep(std::time::Duration::from_millis(1100));
            pool.get_or_import(py, "os").unwrap(); // 触发 eviction → math 应被移除
            let s = pool.stats();
            assert_eq!(s.cached_modules, 2);
            assert_eq!(s.evictions, 1);
        });
    }
}
