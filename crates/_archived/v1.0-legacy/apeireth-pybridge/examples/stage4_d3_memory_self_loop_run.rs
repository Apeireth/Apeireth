//! R129-4 ASI Python 整合 Stage 4 自治 - D3 记忆自循环 example
//!
//! 跑: `cargo run -p apeireth-pybridge --example stage4_d3_memory_self_loop_run`
//!
//! 演示: 记忆自循环 (D3 自治维度)
//! 借鉴: chidori journal 9 字段 + superpowers 234 Skill execution
//!
//! # 0 装 PASS 严守
//!
//! - ✅ chidori (R125-8) cloned = 借鉴真实施
//! - ✅ superpowers 234 (R125-14) cloned = 借鉴真实施

use std::collections::HashMap;

use apeireth_pybridge::{
    memory_self_loop_summary, MemoryKind, MemoryResult, MemorySelfLoop, MEMORY_ENTRY_FIELDS,
    MEMORY_KIND_COUNT, MEMORY_MAX_ENTRIES, MEMORY_RESULT_COUNT,
};

fn main() {
    println!("=== R129-4 D3: Memory Self-Loop Demo ===\n");
    println!("{}", memory_self_loop_summary());
    println!();

    // 1. MemorySelfLoop + 5 维 (4 journal + 1 loop)
    println!(
        "1. MemorySelfLoop: max_entries={}, kinds={}, results={}, fields={}",
        MEMORY_MAX_ENTRIES, MEMORY_KIND_COUNT, MEMORY_RESULT_COUNT, MEMORY_ENTRY_FIELDS
    );
    println!();

    // 2. 记录 5 类记忆 (D1 工具 / D2 反思 / D4 决策 / 观察 / audit)
    let mut l = MemorySelfLoop::new();
    l.start();

    println!("2. 记录 5 类记忆条目:");
    let s0 = l.record_tool_invocation(
        "executor",
        HashMap::new(),
        "executed v1077",
        MemoryResult::Ok,
    );
    println!("   [seq={}] ToolInvocation: executor", s0);

    let s1 = l.record_reflection(
        "reflect_node",
        HashMap::new(),
        "reflected v1400",
        MemoryResult::Ok,
    );
    println!("   [seq={}] ReflectionStep: reflect_node", s1);

    let s2 = l.record_decision(
        "decide_policy",
        HashMap::new(),
        "decided Balanced",
        MemoryResult::Ok,
    );
    println!("   [seq={}] DecisionMake: decide_policy", s2);

    let s3 = l.record_observation("observe", "observed v1458 anchor", MemoryResult::Ok);
    println!("   [seq={}] ObservationRecord: observe", s3);

    let s4 = l.record_audit(
        "8_hard_walls",
        "B1+B2+A1+B3+B4+B5+A3+C1+C2+C3 all pass",
        MemoryResult::Ok,
    );
    println!("   [seq={}] AuditCheckpoint: 8_hard_walls", s4);
    println!();

    // 3. journal 状态
    println!(
        "3. journal 状态: len={}, replay={:?}",
        l.journal().len(),
        l.journal().replay()
    );
    println!();

    // 4. filter_kind + filter_source
    println!("4. filter_kind + filter_source:");
    let tool_only = l.journal().filter_kind(MemoryKind::ToolInvocation);
    println!("   ToolInvocation: {} entries", tool_only.len());
    let executor_only = l.journal().filter_source("executor");
    println!("   source=executor: {} entries", executor_only.len());
    println!();

    // 5. 1 条 entry 详情
    println!("5. 1 条 entry 详情 (seq=0):");
    if let Some(e) = l.journal().get(0) {
        println!("{}", e);
    }
    println!();

    println!("=== D3 演示 done, 0 装 PASS 严守 ===");
}
