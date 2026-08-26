//! # apeireth-keyring benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 P0 凭证安全 keyring 关键 API 性能:
//! - `validate_tool_call`: 6 工具白名单
//! - `detect_platform()`: 跨平台 OS 检测
//! - `KeyringConfig::default()`: 默认配置构造
//! - `TokenEntry` struct 序列化
//! - `SecretBytes` Debug 脱敏
//!
//! **基线** (1.0.0): target/criterion/apeireth-keyring/bench/

use apeireth_keyring::{
    KeyringConfig, SecretBytes, TokenEntry, TokenType, TOOL_WHITELIST, detect_platform,
    validate_tool_call,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_keyring_store_token");
            let args = black_box(serde_json::json!({"service": "openai", "account": "user"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_detect_platform(c: &mut Criterion) {
    c.bench_function("detect_platform", |b| {
        b.iter(|| {
            let _ = detect_platform();
        });
    });
}

fn bench_keyring_config_default(c: &mut Criterion) {
    c.bench_function("keyring_config_default", |b| {
        b.iter(|| {
            let _ = KeyringConfig::default();
        });
    });
}

fn bench_token_entry_serialize(c: &mut Criterion) {
    c.bench_function("token_entry_serialize", |b| {
        let entry = TokenEntry::new("openai", "user@host", TokenType::Anthropic);
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&entry)).unwrap();
        });
    });
}

fn bench_secret_bytes_debug(c: &mut Criterion) {
    c.bench_function("secret_bytes_debug", |b| {
        let secret = SecretBytes::new(b"hunter2-secret-token-value");
        b.iter(|| {
            let _ = format!("{:?}", black_box(&secret));
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_detect_platform,
    bench_keyring_config_default,
    bench_token_entry_serialize,
    bench_secret_bytes_debug
);
criterion_main!(benches);
