//! # apeireth-lark benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 Lark/Feishu SDK Stub 关键 API 性能:
//! - `validate_tool_call`: 9 工具白名单
//! - `LarkConfig::default()`: 默认配置构造
//! - `MessageType` enum 序列化 (5 message types)
//! - `TenantAccessToken` 序列化
//! - `is_stub_mode()`: 编译期常量
//!
//! **基线** (1.0.0): target/criterion/apeireth-lark/bench/

use apeireth_lark::{is_stub_mode, validate_tool_call, LarkConfig, MessageType, TOOL_WHITELIST};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_lark_send_message");
            let args = black_box(serde_json::json!({"chat_id": "oc_xxx", "text": "hi"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_lark_config_default(c: &mut Criterion) {
    c.bench_function("lark_config_default", |b| {
        b.iter(|| {
            let _ = LarkConfig::default();
        });
    });
}

fn bench_message_type_serialize(c: &mut Criterion) {
    c.bench_function("message_type_serialize", |b| {
        let m = MessageType::Text;
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&m)).unwrap();
        });
    });
}

fn bench_tool_whitelist_9_iteration(c: &mut Criterion) {
    c.bench_function("tool_whitelist_9_iteration", |b| {
        b.iter(|| {
            for tool in TOOL_WHITELIST {
                black_box(tool);
            }
        });
    });
}

fn bench_is_stub_mode(c: &mut Criterion) {
    c.bench_function("is_stub_mode", |b| {
        b.iter(|| {
            let _ = black_box(is_stub_mode());
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_lark_config_default,
    bench_message_type_serialize,
    bench_tool_whitelist_9_iteration,
    bench_is_stub_mode
);
criterion_main!(benches);
