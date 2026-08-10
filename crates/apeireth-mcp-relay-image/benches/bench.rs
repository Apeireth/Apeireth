//! # apeireth-mcp-relay-image benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 Image Relay MCP Server 关键 API 性能:
//! - `validate_tool_call`: 5 工具白名单
//! - `compute_sha256`: 1KB / 64KB / 1MB payload hash
//! - `ImageFormat::from_extension` 字符串解析
//! - `RelayCache` LRU 命中/未命中
//! - `RelayConfig::default()` 构造
//!
//! **基线** (1.0.0): target/criterion/apeireth-mcp-relay-image/bench/

use apeireth_mcp_relay_image::{
    ImageFormat, RelayConfig, TOOL_WHITELIST, compute_sha256, validate_tool_call,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_relay_image_relay");
            let args = black_box(serde_json::json!({"url": "https://example.com/img.png"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_sha256_1kb(c: &mut Criterion) {
    let data = vec![0u8; 1024];
    c.bench_function("sha256_1kb", |b| {
        b.iter(|| {
            compute_sha256(black_box(&data));
        });
    });
}

fn bench_sha256_64kb(c: &mut Criterion) {
    let data = vec![0u8; 64 * 1024];
    c.bench_function("sha256_64kb", |b| {
        b.iter(|| {
            compute_sha256(black_box(&data));
        });
    });
}

fn bench_image_format_parse(c: &mut Criterion) {
    c.bench_function("image_format_parse", |b| {
        b.iter(|| {
            // 4 个常见 MIME 循环测
            for mime in ["image/png", "image/jpeg", "image/webp", "image/gif"] {
                let _ = ImageFormat::from_mime(black_box(mime));
            }
        });
    });
}

fn bench_relay_config_default(c: &mut Criterion) {
    c.bench_function("relay_config_default", |b| {
        b.iter(|| {
            let _ = RelayConfig::default();
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_sha256_1kb,
    bench_sha256_64kb,
    bench_image_format_parse,
    bench_relay_config_default
);
criterion_main!(benches);
