//! # apeireth-team-lead benches (R20 阶段 6 — 1.0 release #7 perf baseline)
//!
//! 5 个 bench 测 Team Lead / Orchestrator 关键 API 性能:
//! - `validate_tool_call`: 14 工具白名单 (8 协调 + 3 worktree + 3 感知)
//! - `build_supervisor_prompt`: 818 行 supervisor prompt 构建
//! - `AgentStatus` enum 序列化 (7 variant)
//! - `Message` struct 序列化
//! - `TeamConfig::default()` 构造
//!
//! **基线** (1.0.0): target/criterion/apeireth-team-lead/bench/

use apeireth_team_lead::{
    AgentStatus, Message, MessageType, TeamConfig, TOOL_WHITELIST,
    build_supervisor_prompt, validate_tool_call,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_validate_tool_call_hit(c: &mut Criterion) {
    c.bench_function("validate_tool_call_hit", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_team_lead_spawn_agent");
            let args = black_box(serde_json::json!({"task": "implement feature"}));
            validate_tool_call(tool, &args).unwrap();
        });
    });
}

fn bench_validate_tool_call_miss(c: &mut Criterion) {
    c.bench_function("validate_tool_call_miss", |b| {
        b.iter(|| {
            let tool = black_box("apeireth_team_lead_nonexistent");
            let args = black_box(serde_json::json!({}));
            let _ = validate_tool_call(tool, &args);
        });
    });
}

fn bench_tool_whitelist_14_iteration(c: &mut Criterion) {
    c.bench_function("tool_whitelist_14_iteration", |b| {
        b.iter(|| {
            // 14 工具全表查找 (8 协调 + 3 worktree + 3 感知)
            for tool in TOOL_WHITELIST {
                black_box(tool);
            }
        });
    });
}

fn bench_build_supervisor_prompt(c: &mut Criterion) {
    c.bench_function("build_supervisor_prompt", |b| {
        let providers = vec!["openai", "anthropic", "minimaxi"];
        b.iter(|| {
            let _ = build_supervisor_prompt(black_box(&providers));
        });
    });
}

fn bench_message_serialize(c: &mut Criterion) {
    c.bench_function("message_serialize", |b| {
        let msg = Message::new(
            Some("orchestrator".to_string()),
            "sub-agent",
            MessageType::Send,
            serde_json::json!({"task": "implement feature"}),
        );
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&msg)).unwrap();
        });
    });
}

fn bench_agent_status_serialize(c: &mut Criterion) {
    c.bench_function("agent_status_serialize", |b| {
        let status = AgentStatus::Running;
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&status)).unwrap();
        });
    });
}

fn bench_team_config_default(c: &mut Criterion) {
    c.bench_function("team_config_default", |b| {
        b.iter(|| {
            let _ = TeamConfig::default();
        });
    });
}

criterion_group!(
    benches,
    bench_validate_tool_call_hit,
    bench_validate_tool_call_miss,
    bench_tool_whitelist_14_iteration,
    bench_build_supervisor_prompt,
    bench_message_serialize,
    bench_agent_status_serialize,
    bench_team_config_default
);
criterion_main!(benches);
