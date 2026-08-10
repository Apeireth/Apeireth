//! # apeireth-machine-id benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测跨平台机器指纹 crate 关键 API 性能:
//! - `validate_tool_call`: 4 工具白名单
//! - `hash_machine_id(raw)`: SHA-256 哈希 (MACHINE_ID_HASH_ALGO = "sha256" hardcode)
//! - `default_cache_path()`: 跨平台 cache 路径
//! - `Platform` enum 序列化
//! - `MachineIdResult` struct 序列化
//!
//! **基线** (1.0.0): target/criterion/apeireth-machine-id/bench/

use apeireth_machine_id::{
    MachineIdResult, Platform, TOOL_WHITELIST, default_cache_path, hash_machine_id,
    validate_tool_call,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_machine_id_get");
            let args = black_box(serde_json::json!({}));
            let _ = validate_tool_call(tool, &args);
        });
    });
}

fn bench_hash_machine_id(c: &mut Criterion) {
    c.bench_function("hash_machine_id", |b| {
        // 模拟典型 raw machine id (UUID 格式 36 字符)
        let raw = "01234567-89ab-cdef-0123-456789abcdef";
        b.iter(|| {
            let _ = hash_machine_id(black_box(raw)).unwrap();
        });
    });
}

fn bench_default_cache_path(c: &mut Criterion) {
    c.bench_function("default_cache_path", |b| {
        b.iter(|| {
            let _ = black_box(default_cache_path().unwrap());
        });
    });
}

fn bench_platform_serialize(c: &mut Criterion) {
    c.bench_function("platform_serialize", |b| {
        // 4 平台全循环 (Windows/Darwin/Linux/Bsd, per SUPPORTED_PLATFORMS 编译期 hardcode)
        for p in [Platform::Windows, Platform::Darwin, Platform::Linux, Platform::Bsd] {
            b.iter(|| {
                let _ = serde_json::to_string(black_box(&p)).unwrap();
            });
        }
    });
}

fn bench_machine_id_result_serialize(c: &mut Criterion) {
    c.bench_function("machine_id_result_serialize", |b| {
        let result = MachineIdResult::new(
            "raw-uuid",
            "hashed-hex-string",
            Platform::Linux,
            "/etc/machine-id",
        );
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&result)).unwrap();
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_hash_machine_id,
    bench_default_cache_path,
    bench_platform_serialize,
    bench_machine_id_result_serialize
);
criterion_main!(benches);
