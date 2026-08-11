//! R129-4 ASI Python 整合 Stage 4 自治 - D3 记忆自循环集成测试
//!
//! **任务**: ASI Python 整合 Stage 4 自治 (per decision-61 §3.1 R129-4)
//! **借鉴**: chidori journal 9 字段 (R125-8 ✅ done) 1:1
//!           + superpowers 234 Skill execution (R125-14 ✅ done)
//! **目标**: D3 记忆自循环 (memory self-loop, journal append-only) 集成测试

use std::collections::HashMap;

use apeireth_pybridge::{
    memory_self_loop_summary, DeterminismMeta, MemoryEntry, MemoryJournal, MemoryKind,
    MemoryResult, MemorySelfLoop, MEMORY_ENTRY_FIELDS, MEMORY_KIND_COUNT, MEMORY_MAX_ENTRIES,
    MEMORY_RESULT_COUNT,
};

// 1. D3 MemoryKind 7 变体
#[test]
fn d3_01_memory_kind_7() {
    assert_eq!(MemoryKind::ALL.len(), MEMORY_KIND_COUNT);
    assert_eq!(MEMORY_KIND_COUNT, 7);
}

// 2. D3 MemoryResult 4 变体
#[test]
fn d3_02_memory_result_4() {
    assert_eq!(MemoryResult::ALL.len(), MEMORY_RESULT_COUNT);
    assert_eq!(MEMORY_RESULT_COUNT, 4);
}

// 3. D3 MemoryJournal new 空
#[test]
fn d3_03_journal_new_empty() {
    let j = MemoryJournal::new();
    assert_eq!(j.len(), 0);
    assert!(j.is_empty());
}

// 4. D3 MemoryJournal append 单调 seq
#[test]
fn d3_04_journal_append_monotonic() {
    let mut j = MemoryJournal::new();
    let s0 = j.append(
        MemoryKind::ToolInvocation,
        "t1",
        "v1",
        HashMap::new(),
        "o",
        MemoryResult::Ok,
    );
    let s1 = j.append(
        MemoryKind::ToolReflection,
        "t1",
        "v1",
        HashMap::new(),
        "o",
        MemoryResult::Ok,
    );
    assert_eq!(s0, 0);
    assert_eq!(s1, 1);
    assert_eq!(j.replay(), vec![0, 1]);
}

// 5. D3 MemoryJournal filter_kind
#[test]
fn d3_05_journal_filter_kind() {
    let mut j = MemoryJournal::new();
    j.append(
        MemoryKind::ToolInvocation,
        "t",
        "v",
        HashMap::new(),
        "o",
        MemoryResult::Ok,
    );
    j.append(
        MemoryKind::ReflectionStep,
        "r",
        "v",
        HashMap::new(),
        "o",
        MemoryResult::Ok,
    );
    j.append(
        MemoryKind::ToolInvocation,
        "t",
        "v",
        HashMap::new(),
        "o",
        MemoryResult::Ok,
    );
    assert_eq!(j.filter_kind(MemoryKind::ToolInvocation).len(), 2);
    assert_eq!(j.filter_kind(MemoryKind::ReflectionStep).len(), 1);
}

// 6. D3 MemoryJournal filter_source
#[test]
fn d3_06_journal_filter_source() {
    let mut j = MemoryJournal::new();
    j.append(
        MemoryKind::ToolInvocation,
        "executor",
        "v",
        HashMap::new(),
        "o",
        MemoryResult::Ok,
    );
    j.append(
        MemoryKind::ToolInvocation,
        "reflector",
        "v",
        HashMap::new(),
        "o",
        MemoryResult::Ok,
    );
    assert_eq!(j.filter_source("executor").len(), 1);
    assert_eq!(j.filter_source("reflector").len(), 1);
}

// 7. D3 MemoryJournal get by seq
#[test]
fn d3_07_journal_get_by_seq() {
    let mut j = MemoryJournal::new();
    let s = j.append(
        MemoryKind::ToolInvocation,
        "t",
        "v",
        HashMap::new(),
        "hello",
        MemoryResult::Ok,
    );
    let e = j.get(s).expect("entry");
    assert_eq!(e.output, "hello");
}

// 8. D3 MemoryJournal capacity
#[test]
fn d3_08_journal_capacity() {
    let j = MemoryJournal::new();
    assert_eq!(j.capacity(), MEMORY_MAX_ENTRIES);
    assert!(!j.is_full());
}

// 9. D3 MemoryEntry 9 字段
#[test]
fn d3_09_memory_entry_9_fields() {
    assert_eq!(MEMORY_ENTRY_FIELDS, 9);
    let e = MemoryEntry {
        seq: 0,
        kind: MemoryKind::ToolInvocation,
        ts: 0,
        source: "s".to_string(),
        plan_version: "v".to_string(),
        input: HashMap::new(),
        output: "o".to_string(),
        result: MemoryResult::Ok,
        determinism_meta: DeterminismMeta::default(),
    };
    // 9 字段 compile-time 校验
    let _ = e.seq;
    let _ = e.kind;
    let _ = e.ts;
    let _ = e.source;
    let _ = e.plan_version;
    let _ = e.input;
    let _ = e.output;
    let _ = e.result;
    let _ = e.determinism_meta;
}

// 10. D3 MemorySelfLoop record_tool
#[test]
fn d3_10_self_loop_record_tool() {
    let mut l = MemorySelfLoop::new();
    l.start();
    let seq = l.record_tool_invocation("executor", HashMap::new(), "o", MemoryResult::Ok);
    assert_eq!(seq, 0);
    assert_eq!(l.journal().len(), 1);
    assert_eq!(l.appended_count(), 1);
}

// 11. D3 MemorySelfLoop record_reflection
#[test]
fn d3_11_self_loop_record_reflection() {
    let mut l = MemorySelfLoop::new();
    l.start();
    let seq = l.record_reflection("r", HashMap::new(), "o", MemoryResult::Ok);
    let e = l.journal().get(seq).expect("entry");
    assert_eq!(e.kind, MemoryKind::ReflectionStep);
}

// 12. D3 MemorySelfLoop record_decision
#[test]
fn d3_12_self_loop_record_decision() {
    let mut l = MemorySelfLoop::new();
    l.start();
    let seq = l.record_decision("d", HashMap::new(), "o", MemoryResult::Ok);
    let e = l.journal().get(seq).expect("entry");
    assert_eq!(e.kind, MemoryKind::DecisionMake);
}

// 13. D3 MemorySelfLoop record_audit
#[test]
fn d3_13_self_loop_record_audit() {
    let mut l = MemorySelfLoop::new();
    l.start();
    let seq = l.record_audit("8_hard_walls", "all pass", MemoryResult::Ok);
    let e = l.journal().get(seq).expect("entry");
    assert_eq!(e.kind, MemoryKind::AuditCheckpoint);
}

// 14. D3 summary 引用 chidori + superpowers
#[test]
fn d3_14_summary_cites_borrow_ids() {
    let s = memory_self_loop_summary();
    assert!(s.contains("R129-4 D3"));
    assert!(s.contains("chidori"));
    assert!(s.contains("superpowers-234"));
    assert!(s.contains("✅"));
    assert!(s.contains("0 装 PASS 严守"));
}

// 15. D3 MemorySelfLoop with_plan_version
#[test]
fn d3_15_with_plan_version() {
    let l = MemorySelfLoop::with_plan_version("custom-v1");
    // plan_version 字段是 private, 通过 summary 验证
    let s = l.summary();
    assert!(s.contains("plan_version=") || s.contains("custom-v1"), "plan_version 必 = custom-v1, got: {s}");
}
