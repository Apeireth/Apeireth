//! # apeireth-mcp-ssh benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 SSH MCP Server 关键 API 性能:
//! - `validate_tool_call`: 8 工具白名单字符串查找
//! - `TOOL_WHITELIST` 长度断言
//! - `SshAuthMethod` enum 序列化 (5 认证方法)
//! - `SshMcpConfig::default()` 默认配置构造
//! - `SecretString` 序列化 (redact "***REDACTED***")
//!
//! **基线** (1.0.0): target/criterion/apeireth-mcp-ssh/bench/

use apeireth_mcp_ssh::{
    SshAuthMethod, SshMcpConfig, TOOL_WHITELIST, validate_tool_call, SecretString,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_ssh_exec");
            let args = black_box(serde_json::json!({"cmd": "ls -la"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_validate_tool_call_miss(c: &mut Criterion) {
    c.bench_function("validate_tool_call_miss", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_ssh_nonexistent");
            let args = black_box(serde_json::json!({}));
            let _ = validate_tool_call(tool, &args);
        });
    });
}

fn bench_tool_whitelist_iteration(c: &mut Criterion) {
    c.bench_function("tool_whitelist_iteration", |b| {
        b.iter(|| {
            // 测 8 工具全表查找
            for tool in TOOL_WHITELIST {
                black_box(tool);
            }
        });
    });
}

fn bench_ssh_auth_method_serialize(c: &mut Criterion) {
    c.bench_function("ssh_auth_method_serialize", |b| {
        let method = SshAuthMethod::Password {
            username: "root".into(),
            password: SecretString::new("hunter2"),
        };
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&method)).unwrap();
        });
    });
}

fn bench_ssh_mcp_config_default(c: &mut Criterion) {
    c.bench_function("ssh_mcp_config_default", |b| {
        b.iter(|| {
            let _ = SshMcpConfig::default();
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_validate_tool_call_miss,
    bench_tool_whitelist_iteration,
    bench_ssh_auth_method_serialize,
    bench_ssh_mcp_config_default
);
criterion_main!(benches);
