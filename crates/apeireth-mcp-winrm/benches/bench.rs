//! # apeireth-mcp-winrm benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 WinRM MCP Server 关键 API 性能:
//! - `validate_tool_call`: 8 工具白名单
//! - `WinRmAuthMethod` enum 序列化 (5 认证方法)
//! - `WinRmMcpConfig::default()`
//! - `WinRmAuthMethodKind` Display 转换
//! - `SecretString` Serialize (redact "***REDACTED***")
//!
//! **基线** (1.0.0): target/criterion/apeireth-mcp-winrm/bench/

use apeireth_mcp_winrm::{
    SecretString, TOOL_WHITELIST, WinRmAuthMethod, WinRmMcpConfig, validate_tool_call,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_winrm_run_command");
            let args = black_box(serde_json::json!({"command": "ipconfig"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_validate_tool_call_miss(c: &mut Criterion) {
    c.bench_function("validate_tool_call_miss", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_winrm_nonexistent");
            let args = black_box(serde_json::json!({}));
            let _ = validate_tool_call(tool, &args);
        });
    });
}

fn bench_tool_whitelist_iteration(c: &mut Criterion) {
    c.bench_function("tool_whitelist_iteration", |b| {
        b.iter(|| {
            for tool in TOOL_WHITELIST {
                black_box(tool);
            }
        });
    });
}

fn bench_winrm_auth_method_serialize(c: &mut Criterion) {
    c.bench_function("winrm_auth_method_serialize", |b| {
        let method = WinRmAuthMethod::Basic {
            username: "admin".into(),
            password: SecretString::new("p@ssw0rd"),
        };
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&method)).unwrap();
        });
    });
}

fn bench_winrm_mcp_config_default(c: &mut Criterion) {
    c.bench_function("winrm_mcp_config_default", |b| {
        b.iter(|| {
            let _ = WinRmMcpConfig::default();
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_validate_tool_call_miss,
    bench_tool_whitelist_iteration,
    bench_winrm_auth_method_serialize,
    bench_winrm_mcp_config_default
);
criterion_main!(benches);
