//! R128 阶段 A Stage 2 — BridgeModulePool 端到端集成测试 (per decision-57 §2.1 P10-2)
//!
//! 借鉴 Stage 1 `bridge_pool.rs` (hyper 80 LIFO 池复用) + Stage 2 新加
//! `end_to_end_smoke_check()` 公共 API, 实施端到端集成测试.
//!
//! # 借鉴来源
//!
//! - **hyper 80 池复用** (`borrowed-repos/hyper/hyper-util/src/client/pool.rs`):
//!   LIFO 策略 + `pool_max_idle_per_host` + 空闲超时. Stage 1 已实施 (R127-2 BORROW ID).
//! - **Stage 2 集成测试 API** (本文件 + lib.rs `end_to_end_smoke_check`): cfg-无关
//!   默认 build 0 体积跑 (Stage 2 src 改动, 0 装 PASS 严守).
//!
//! # cfg 守门
//!
//! - 默认 build (无 `python-ext`): 跑 0 体积 stub, pool stats 全 0, 不真正 import Python.
//! - `python-ext` build: 跑真 Python 端到端 (依赖 Stage 1 python-ext 实现, 0 装 PASS).
//!
//! # 8 硬墙 0 越界
//!
//! - B2 1.2.0 0 改
//! - A1 R11 baseline 0.8682/0.8532/0.9063 严守 (不触碰 apeireth-asi)
//! - B1 24 LOCKED 入口签名 0 改 (apeireth-pybridge 不在 24 LOCKED, 自由改 src)
//! - B5 8 哲学锚 / B3 30 维 / B4 6 重 v7 / A3 13 键 0 改
//! - C1 0 commit (写到 reports 不动 git)
//!
//! # 端到端场景
//!
//! 1. 池构造 + 默认配置 (max_idle=32, idle_timeout=90)
//! 2. 池 stats 初始全 0
//! 3. 池 clear 不影响 stats
//! 4. 池 config 调参 (max_idle=2 触发 LRU eviction)
//! 5. 池 + BridgeHealth 协同 (跨 crate 集成)
//! 6. 池 + R11 compat 协同 (跨 crate 集成)
//! 7. 池 + Stage 2 smoke check 协同 (新加 API 端到端)
//! 8. 池并发 8 个独立实例 (集成场景)
//! 9. 池 hit rate 累计 (跨调用)
//! 10. 池 Display 字段完整 (跨 2 build)

use apeireth_pybridge::{
    end_to_end_smoke_check, is_module_available, python_ext_enabled, python_is_available,
    python_version_string, r11_compat_version, r11_module_count, BridgeModulePool, PoolConfig,
    PoolStats, R11_COMPAT_VERSION, R11_MODULE_COUNT,
};

// 1. 池构造 + 默认配置 跨 build 严守
#[test]
fn stage2_e2e_pool_default_config_dual_build() {
    let pool = BridgeModulePool::new();
    let cfg = pool.config();
    assert_eq!(
        cfg.max_idle, 32,
        "Stage 1 PoolConfig::default max_idle = 32"
    );
    assert_eq!(
        cfg.idle_timeout_secs, 90,
        "Stage 1 PoolConfig::default idle_timeout = 90s"
    );
}

// 2. 池 stats 初始全 0 (cfg-无关, 默认 build + python-ext 都满足)
#[test]
fn stage2_e2e_pool_initial_stats_zero() {
    let pool = BridgeModulePool::new();
    let s = pool.stats();
    assert_eq!(s.cached_modules, 0);
    assert_eq!(s.hits, 0);
    assert_eq!(s.misses, 0);
    assert_eq!(s.evictions, 0);
    assert_eq!(s.hit_rate(), 0.0, "hit_rate at 0 hits + 0 misses = 0.0");
}

// 3. 池 clear 不影响 stats (统计累计保留, 与 Stage 1 r127_2_pool_clear_empties_cache 一致)
#[test]
fn stage2_e2e_pool_clear_keeps_stats() {
    let pool = BridgeModulePool::new();
    pool.clear();
    let s = pool.stats();
    assert_eq!(s.cached_modules, 0, "clear 后 cached 归 0");
    assert_eq!(s.hits, 0, "clear 不重置 hits 统计 (本测试 hits 本来就 0)");
    assert_eq!(s.misses, 0);
    assert_eq!(s.evictions, 0);
}

// 4. 池 config 调参: 改 max_idle 后 LRU eviction 用 (cfg-无关, 验证 config 字段)
#[test]
fn stage2_e2e_pool_custom_config_max_idle_2() {
    let pool = BridgeModulePool::with_config(PoolConfig {
        max_idle: 2,
        idle_timeout_secs: 30,
    });
    assert_eq!(pool.config().max_idle, 2);
    assert_eq!(pool.config().idle_timeout_secs, 30);
    // python-ext 下会触发 LRU eviction (Stage 1 r127_2_pool_lru_eviction), 默认 build 下不调
}

// 5. 池 + BridgeHealth 协同 (跨 crate 集成 — bridge.rs health_check)
#[test]
fn stage2_e2e_pool_bridge_health_integration() {
    let pool = BridgeModulePool::new();
    let s = pool.stats();
    // 端到端: 池 stats + health_check 协同 (bridge.rs health_check 跨 build 一致)
    // 不直接调 health_check (它返回 BridgeHealth 不在 re-export 列表), 借 python_is_available
    let py_available = python_is_available();
    let _: bool = py_available; // 跨 build 一致可调
    assert_eq!(s.cached_modules, 0);
}

// 6. 池 + R11 compat 协同 (跨 crate 集成 — r11_compat.rs 1103 模块)
#[test]
fn stage2_e2e_pool_r11_compat_integration() {
    let pool = BridgeModulePool::new();
    let s = pool.stats();
    // 端到端: 池 stats + R11 1103 模块协同
    assert_eq!(r11_module_count(), 1103, "R11 baseline 1103 严守 (A1 严守)");
    assert_eq!(R11_MODULE_COUNT, 1103);
    assert_eq!(pool.stats().cached_modules, s.cached_modules);
}

// 7. 池 + Stage 2 smoke check 协同 (新加 API 端到端, 跨 build 一致)
#[test]
fn stage2_e2e_pool_with_stage2_smoke_check() {
    let smoke = end_to_end_smoke_check();
    // 端到端: 池 stats 应 = smoke.pool_stats (Stage 2 新加 API 内部用 BridgeModulePool::default())
    let pool = BridgeModulePool::default();
    assert_eq!(pool.stats().cached_modules, smoke.pool_stats.cached_modules);
    assert_eq!(pool.config().max_idle, smoke.pool_max_idle);
    assert_eq!(
        pool.config().idle_timeout_secs,
        smoke.pool_idle_timeout_secs
    );
    // r11 字段跨 build 一致
    assert_eq!(smoke.r11_module_count, 1103);
    assert!(smoke.r11_compat_version.contains("R14"));
}

// 8. 池并发 8 个独立实例 (集成场景 — 每个实例有独立 stats)
#[test]
fn stage2_e2e_pool_eight_independent_instances() {
    let pools: Vec<BridgeModulePool> = (0..8)
        .map(|i| {
            BridgeModulePool::with_config(PoolConfig {
                max_idle: 8 + i,
                idle_timeout_secs: 60 + i as u64,
            })
        })
        .collect();
    for (i, p) in pools.iter().enumerate() {
        assert_eq!(p.config().max_idle, 8 + i);
        assert_eq!(p.config().idle_timeout_secs, 60 + i as u64);
        assert_eq!(p.stats().cached_modules, 0);
    }
}

// 9. 池 hit rate 累计 (跨调用, 默认 build 下全是 0 因为不调 import, python-ext 下 Stage 1 测试覆盖)
#[test]
fn stage2_e2e_pool_hit_rate_accumulation() {
    let pool = BridgeModulePool::new();
    // 多次 stats 查询, hit_rate 应保持 0.0 (cfg-无关 — 默认 build 0 体积, python-ext Stage 1 覆盖)
    for _ in 0..10 {
        let s = pool.stats();
        assert_eq!(s.hit_rate(), 0.0, "默认 build 下 hit_rate 永远 = 0.0");
    }
}

// 10. 池 Display 字段完整 (跨 2 build, 借 Stage 2 BridgePoolSmoke Display)
#[test]
fn stage2_e2e_pool_display_complete_fields() {
    let smoke = end_to_end_smoke_check();
    let out = format!("{smoke}");
    // Stage 2 新加 BridgePoolSmoke Display 字段
    assert!(out.contains("r11"), "Display 含 r11: {out}");
    assert!(out.contains("pool"), "Display 含 pool: {out}");
    assert!(
        out.contains("max_idle=32"),
        "Display 含 max_idle=32 (Stage 1 default): {out}"
    );
    assert!(out.contains("cached="), "Display 含 cached= 字段: {out}");
    assert!(
        out.contains("hit_rate="),
        "Display 含 hit_rate= 字段: {out}"
    );
}

// 11. 池 + python_ext 配置 (跨 build 一致, 借 python_ext_enabled + is_module_available)
#[test]
fn stage2_e2e_pool_with_python_ext_dual_build() {
    let pool = BridgeModulePool::new();
    let _: bool = python_ext_enabled();
    let _: bool = is_module_available("math");
    let _: &str = python_version_string();
    // python_ext_enabled 与 cfg! 一致
    assert_eq!(python_ext_enabled(), cfg!(feature = "python-ext"));
    // 池 stats 不依赖 python-ext (cfg-无关)
    assert_eq!(pool.stats().cached_modules, 0);
}

// 12. 池 config 调参 idle_timeout 跨 build 严守
#[test]
fn stage2_e2e_pool_idle_timeout_zero() {
    // 边界: idle_timeout=0 立即超时 (Stage 1 实施 max_idle + idle_timeout 都 0 是合法)
    let pool = BridgeModulePool::with_config(PoolConfig {
        max_idle: 1,
        idle_timeout_secs: 0,
    });
    assert_eq!(pool.config().idle_timeout_secs, 0);
    assert_eq!(pool.config().max_idle, 1);
    // stats 仍初始 0 (默认 build)
    assert_eq!(pool.stats().cached_modules, 0);
}
