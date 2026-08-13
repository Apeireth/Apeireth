//! # apeireth-formal benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测形式化验证 skeleton 关键 API 性能:
//! - `l0_requires_ha_invariant(cfg)`: L0 永远 requires HA (1:1 Kani 不变量)
//! - `PermissionLayerConfig::new(0..6, true/false)`: 6 层权限洋葱配置
//! - `run_all()`: 全部不变量 sanity 验证
//! - `PERMISSION_ONION_DEPTH` const 访问
//! - `verify()`: panic-first 不变量验证
//!
//! **基线** (1.0.0): target/criterion/apeireth-formal/bench/

use apeireth_formal::{
    PERMISSION_ONION_DEPTH, PermissionLayerConfig, l0_requires_ha_invariant, run_all, verify,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_l0_requires_ha_invariant_true(c: &mut Criterion) {
    c.bench_function("l0_requires_ha_invariant_true", |b| {
        let cfg = PermissionLayerConfig::new(0, true);
        b.iter(|| {
            let _ = l0_requires_ha_invariant(black_box(cfg));
        });
    });
}

fn bench_l0_requires_ha_invariant_false(c: &mut Criterion) {
    c.bench_function("l0_requires_ha_invariant_false", |b| {
        let cfg = PermissionLayerConfig::new(0, false);
        b.iter(|| {
            let _ = l0_requires_ha_invariant(black_box(cfg));
        });
    });
}

fn bench_layer_config_construct(c: &mut Criterion) {
    c.bench_function("layer_config_construct", |b| {
        b.iter(|| {
            // 6 层权限洋葱 (L0..L5) 构造
            for kind in 0..PERMISSION_ONION_DEPTH {
                let _ = PermissionLayerConfig::new(kind as u8, true);
            }
        });
    });
}

fn bench_run_all(c: &mut Criterion) {
    c.bench_function("run_all", |b| {
        b.iter(|| {
            let _ = run_all();
        });
    });
}

fn bench_verify(c: &mut Criterion) {
    c.bench_function("verify", |b| {
        b.iter(|| {
            // 注: verify() 是 panic-first, 只在测试时 panic; 正常情况都通过
            verify();
        });
    });
}

criterion_group!(
    benches,
    bench_l0_requires_ha_invariant_true,
    bench_l0_requires_ha_invariant_false,
    bench_layer_config_construct,
    bench_run_all,
    bench_verify
);
criterion_main!(benches);
